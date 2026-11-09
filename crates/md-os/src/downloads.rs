//! Filename and filesystem policy for session-authenticated WebKit downloads.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub fn safe_name(input: &str) -> String {
    let clean: String = input
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .take(150)
        .collect();
    let mut clean = clean.trim_matches([' ', '.']).to_owned();
    while clean.len() > 180 {
        clean.pop();
    }
    clean = clean.trim_matches([' ', '.']).to_owned();
    if clean.is_empty() {
        "download".into()
    } else {
        clean
    }
}
pub fn pdf_save_name(mime: &str, bytes: &[u8], suggested: &str) -> Option<String> {
    if mime != "application/pdf" || bytes.len() > 128 * 1024 * 1024 || !bytes.starts_with(b"%PDF-")
    {
        return None;
    }
    let mut name = safe_name(suggested);
    if !name.to_ascii_lowercase().ends_with(".pdf") {
        name.push_str(".pdf");
    }
    Some(name)
}
pub fn needs_confirmation(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    !matches!(
        ext.as_str(),
        "pdf" | "txt" | "csv" | "docx" | "xlsx" | "pptx" | "png" | "jpg" | "jpeg" | "gif" | "webp"
    )
}
/// Refuse symlinks in directories owned by the app's download routing.
pub fn directory(path: &Path) -> io::Result<()> {
    match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let meta = fs::symlink_metadata(path)?;
            if meta.is_dir() && !meta.file_type().is_symlink() {
                Ok(())
            } else {
                Err(io::Error::other("Unsafe download directory"))
            }
        }
        Err(error) => Err(error),
    }
}
/// A same-filesystem hard link publishes a completed file without ever replacing
/// an existing destination, even if another process creates it concurrently.
pub fn publish(staged: &Path, folder: &Path, filename: &str) -> io::Result<PathBuf> {
    let meta = fs::symlink_metadata(staged)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(io::Error::other("Unsafe staged file"));
    }
    let filename = safe_name(filename);
    let name = Path::new(&filename);
    let stem = name
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("download");
    let ext = name
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| format!(".{v}"))
        .unwrap_or_default();
    for index in 1..=10_000 {
        let candidate = folder.join(if index == 1 {
            filename.clone()
        } else {
            format!("{stem} ({index}){ext}")
        });
        match fs::hard_link(staged, &candidate) {
            Ok(()) => {
                let _ = fs::remove_file(staged);
                return Ok(candidate);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::other("Too many filename collisions"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filenames_and_risky_types() {
        assert_eq!(
            pdf_save_name("application/pdf", b"%PDF-1.7\n", "../../lesson"),
            Some("_.._lesson.pdf".into())
        );
        assert!(pdf_save_name("text/html", b"%PDF-1.7", "lesson.pdf").is_none());
        assert!(pdf_save_name("application/pdf", b"not a PDF", "lesson.pdf").is_none());
        assert_eq!(
            safe_name("../course\\file:notes.pdf"),
            "_course_file_notes.pdf"
        );
        assert_eq!(safe_name("..."), "download");
        assert_eq!(safe_name("a\u{202e}gpj.exe"), "agpj.exe");
        assert!(needs_confirmation(Path::new("lesson.PDF.app")));
        assert!(needs_confirmation(Path::new("lesson.dmg")));
        assert!(needs_confirmation(Path::new("script")));
        assert!(!needs_confirmation(Path::new("lesson.PDF")));
    }
    #[test]
    fn publication_never_overwrites_and_rejects_symlinks() {
        let root = std::env::temp_dir().join(format!("moodburo-download-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let staged = root.join("staged");
        fs::write(root.join("notes.pdf"), "original").unwrap();
        fs::write(&staged, "new file").unwrap();
        let result = publish(&staged, &root, "notes.pdf").unwrap();
        assert_eq!(result.file_name().unwrap(), "notes (2).pdf");
        assert_eq!(
            fs::read_to_string(root.join("notes.pdf")).unwrap(),
            "original"
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&result, &staged).unwrap();
            assert!(publish(&staged, &root, "unsafe.pdf").is_err());
            assert!(directory(&staged).is_err());
        }
        fs::remove_dir_all(root).unwrap();
    }
}
