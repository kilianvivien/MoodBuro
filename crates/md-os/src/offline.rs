//! Local snapshots are immutable from MoodBuro's perspective; refresh publishes alongside.
use sha2::{Digest, Sha256};
use std::{
    io::{self, Read},
    path::Path,
};
pub fn key(module: i64, filepath: &str, filename: &str) -> String {
    let mut digest = Sha256::new();
    for value in [
        module.to_string().as_bytes(),
        filepath.as_bytes(),
        filename.as_bytes(),
    ] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value);
    }
    format!("{:x}", digest.finalize())
}
pub fn hash(path: &Path) -> io::Result<String> {
    let meta = std::fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > 128 * 1024 * 1024 {
        return Err(io::Error::other("Unsafe snapshot"));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path)?.take(128 * 1024 * 1024 + 1);
    let mut total = 0_u64;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > 128 * 1024 * 1024 {
            return Err(io::Error::other("Snapshot too large"));
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
/// Check all components beneath the managed root, not only the final filename.
pub fn managed_path(root: &Path, path: &Path) -> io::Result<()> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| io::Error::other("Outside managed root"))?;
    let mut checked = root.to_path_buf();
    if std::fs::symlink_metadata(&checked)?
        .file_type()
        .is_symlink()
    {
        return Err(io::Error::other("Unsafe root"));
    }
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(io::Error::other("Unsafe path"));
        }
        checked.push(component);
        if std::fs::symlink_metadata(&checked)?
            .file_type()
            .is_symlink()
        {
            return Err(io::Error::other("Unsafe path"));
        }
    }
    if !checked.is_file() {
        return Err(io::Error::other("Missing file"));
    }
    Ok(())
}
#[cfg(unix)]
pub fn available(path: &Path) -> io::Result<u64> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let path = CString::new(path.as_os_str().as_bytes())?;
    let mut info = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // statvfs writes the supplied structure on success; path is NUL-terminated.
    if unsafe { libc::statvfs(path.as_ptr(), info.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let info = unsafe { info.assume_init() };
    #[allow(clippy::unnecessary_cast)] // statvfs field widths differ across Unix targets.
    let bytes = (info.f_bavail as u64).saturating_mul(info.f_frsize as u64);
    Ok(bytes)
}
#[cfg(not(unix))]
pub fn available(_path: &Path) -> io::Result<u64> {
    Err(io::Error::other(
        "Offline storage is unavailable on this platform",
    ))
}

/// A matching remote hash is insufficient if the local file was edited or moved.
pub fn unchanged(root: &Path, path: &Path, snapshot: &str, incoming: &str) -> bool {
    snapshot == incoming
        && managed_path(root, path).is_ok()
        && hash(path).is_ok_and(|local| local == snapshot)
}

/// At startup only, discard incomplete payloads in private staging folders.
/// Never traverse symlinks or archives and never delete a published document.
pub fn clean_staging(root: &Path) -> usize {
    fn walk(root: &Path, folder: &Path, depth: usize, budget: &mut usize) -> usize {
        if depth > 3 || *budget == 0 {
            return 0;
        }
        let Ok(meta) = std::fs::symlink_metadata(folder) else {
            return 0;
        };
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return 0;
        }
        let Ok(entries) = std::fs::read_dir(folder) else {
            return 0;
        };
        let mut removed = 0;
        for entry in entries.flatten() {
            if *budget == 0 {
                break;
            }
            *budget -= 1;
            let path = entry.path();
            let name = entry.file_name();
            if name == ".trash" {
                continue;
            }
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if !meta.is_dir() || meta.file_type().is_symlink() {
                continue;
            }
            if name == ".moodburo-staging" {
                let Ok(staged) = std::fs::read_dir(&path) else {
                    continue;
                };
                for item in staged.flatten() {
                    if *budget == 0 {
                        break;
                    }
                    *budget -= 1;
                    let id = item.file_name().to_string_lossy().into_owned();
                    if id.len() != 36 || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
                        continue;
                    }
                    let payload = item.path().join("payload");
                    if managed_path(root, &payload).is_ok()
                        && std::fs::remove_file(&payload).is_ok()
                    {
                        removed += 1;
                        let _ = std::fs::remove_dir(item.path());
                    }
                }
            } else {
                removed += walk(root, &path, depth + 1, budget);
            }
        }
        removed
    }
    walk(root, root, 0, &mut 10000)
}

/// Candidates must come from the account-scoped index; bytes are checked again.
pub fn reusable(root: &Path, paths: &[String], incoming: &str) -> Option<std::path::PathBuf> {
    paths
        .iter()
        .map(std::path::PathBuf::from)
        .find(|path| unchanged(root, path, incoming, incoming))
}

/// Removal is reversible: intact copies are archived, edited/unsafe paths stay put.
#[derive(Debug, PartialEq, Eq)]
pub enum Removal {
    Archived,
    Kept,
    Missing,
}
pub fn remove_copy(
    root: &Path,
    path: &Path,
    snapshot: &str,
    archive_id: &str,
) -> io::Result<Removal> {
    // Never use a caller-supplied path component without validating it.
    if archive_id.is_empty()
        || !archive_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(io::Error::other("Invalid archive identity"));
    }
    if let Err(error) = std::fs::symlink_metadata(path) {
        return if error.kind() == io::ErrorKind::NotFound {
            Ok(Removal::Missing)
        } else {
            Err(error)
        };
    }
    if managed_path(root, path).is_err() {
        return Ok(Removal::Kept);
    }
    if hash(path)? != snapshot {
        return Ok(Removal::Kept);
    }
    let trash = root.join(".trash");
    crate::downloads::directory(&trash)?;
    // A fresh directory prevents replacing an earlier archived version.
    let destination = trash.join(archive_id);
    std::fs::create_dir(&destination)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o700))?;
    }
    // Revalidate after creating the archive directory. No file contents are deleted.
    if !unchanged(root, path, snapshot, snapshot) {
        let _ = std::fs::remove_dir(&destination);
        return Ok(Removal::Kept);
    }
    let target = destination.join(
        path.file_name()
            .ok_or_else(|| io::Error::other("Invalid file"))?,
    );
    std::fs::rename(path, target)?;
    Ok(Removal::Archived)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_cleanup_removes_only_partial_payloads_not_archives_documents_or_symlinks() {
        let root =
            std::env::temp_dir().join(format!("moodburo-staging-clean-{}", std::process::id()));
        let id = "12345678-1234-1234-1234-123456789abc";
        let stage = root.join("site/course/section/.moodburo-staging").join(id);
        let archive = root.join(".trash").join(".moodburo-staging").join(id);
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::create_dir_all(&archive).unwrap();
        std::fs::write(stage.join("payload"), b"partial").unwrap();
        std::fs::write(archive.join("payload"), b"archived").unwrap();
        let document = root.join("site/course/notes.pdf");
        std::fs::write(&document, b"published").unwrap();
        assert_eq!(clean_staging(&root), 1);
        assert!(document.exists());
        assert!(archive.join("payload").exists());
        assert_eq!(clean_staging(&root), 0);
        std::fs::create_dir_all(&stage).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&document, stage.join("payload")).unwrap();
            assert_eq!(clean_staging(&root), 0);
        }
        assert_eq!(std::fs::read(document).unwrap(), b"published");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn removal_archives_only_intact_copies_and_never_replaces_an_archive() {
        let root = std::env::temp_dir().join(format!("moodburo-removal-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("notes.txt");
        std::fs::write(&file, "original").unwrap();
        let snapshot = hash(&file).unwrap();
        assert_eq!(
            remove_copy(&root, &file, &snapshot, "first").unwrap(),
            Removal::Archived
        );
        assert!(!file.exists());
        assert_eq!(
            std::fs::read_to_string(root.join(".trash/first/notes.txt")).unwrap(),
            "original"
        );
        std::fs::write(&file, "original").unwrap();
        assert!(remove_copy(&root, &file, &snapshot, "first").is_err());
        assert!(file.exists());
        std::fs::write(&file, "local edits").unwrap();
        assert_eq!(
            remove_copy(&root, &file, &snapshot, "second").unwrap(),
            Removal::Kept
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "local edits");
        assert_eq!(
            remove_copy(&root, &root.join("missing"), &snapshot, "third").unwrap(),
            Removal::Missing
        );
        assert!(remove_copy(&root, &file, &snapshot, "../escape").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn removal_refuses_symlinks_outside_paths_and_unsafe_archive_folders() {
        let root =
            std::env::temp_dir().join(format!("moodburo-removal-links-{}", std::process::id()));
        let managed = root.join("managed");
        std::fs::create_dir_all(&managed).unwrap();
        let outside = root.join("outside.txt");
        std::fs::write(&outside, "kept").unwrap();
        let snapshot = hash(&outside).unwrap();
        assert_eq!(
            remove_copy(&managed, &outside, &snapshot, "outside").unwrap(),
            Removal::Kept
        );
        std::os::unix::fs::symlink(&outside, managed.join("linked")).unwrap();
        assert_eq!(
            remove_copy(&managed, &managed.join("linked"), &snapshot, "linked").unwrap(),
            Removal::Kept
        );
        std::os::unix::fs::symlink(&root, managed.join(".trash")).unwrap();
        let file = managed.join("notes.txt");
        std::fs::write(&file, "kept").unwrap();
        assert!(remove_copy(&managed, &file, &snapshot, "unsafe").is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "kept");
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "kept");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn refresh_preserves_local_edits_and_server_versions_and_missing_copies() {
        let root = std::env::temp_dir().join(format!("moodburo-snapshot-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let original = root.join("notes.txt");
        std::fs::write(&original, "original").unwrap();
        let snapshot = hash(&original).unwrap();
        assert!(unchanged(&root, &original, &snapshot, &snapshot));
        std::fs::write(&original, "local edit").unwrap();
        assert!(!unchanged(&root, &original, &snapshot, &snapshot));
        let staged = root.join("payload");
        std::fs::write(&staged, "remote version").unwrap();
        let remote = hash(&staged).unwrap();
        assert!(!unchanged(&root, &original, &snapshot, &remote));
        let new = crate::downloads::publish(&staged, &root, "notes.txt").unwrap();
        assert_ne!(new, original);
        assert_eq!(std::fs::read_to_string(&original).unwrap(), "local edit");
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "remote version");
        assert!(unchanged(&root, &new, &remote, &remote));
        std::fs::rename(&new, root.join("moved.txt")).unwrap();
        assert!(!unchanged(&root, &new, &remote, &remote));
        assert!(!unchanged(
            &root,
            &root.join("missing"),
            &snapshot,
            &snapshot
        ));
        assert!(available(&root).unwrap() > 0);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn component_symlinks_and_ambiguous_resource_keys_are_rejected() {
        assert_ne!(key(1, "/ab", "c"), key(1, "/a", "bc"));
        assert_ne!(key(1, "/", "a"), key(2, "/", "a"));
        let root =
            std::env::temp_dir().join(format!("moodburo-snapshot-links-{}", std::process::id()));
        std::fs::create_dir_all(root.join("actual")).unwrap();
        let file = root.join("actual/notes.txt");
        std::fs::write(&file, "a").unwrap();
        assert!(managed_path(&root, &file).is_ok());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("actual"), root.join("linked")).unwrap();
            assert!(managed_path(&root, &root.join("linked/notes.txt")).is_err());
            std::os::unix::fs::symlink(&file, root.join("file-link")).unwrap();
            assert!(hash(&root.join("file-link")).is_err());
        }
        assert!(managed_path(&root, &root.join("../outside")).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
