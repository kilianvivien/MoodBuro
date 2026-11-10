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

#[cfg(test)]
mod tests {
    use super::*;
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
