//! New-file metadata only. Inputs are fixed public Moodle pages, never transfer URLs.
use std::path::Path;
fn escape(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub fn strings(values: &[&str]) -> String {
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><array>{}</array></plist>",values.iter().map(|v|format!("<string>{}</string>",escape(v))).collect::<String>())
}
pub fn annotate(path: &Path, source: &str, course: Option<&str>) -> bool {
    #[cfg(target_os = "macos")]
    {
        let write = |key: &str, value: String| {
            std::process::Command::new("/usr/bin/xattr")
                .args(["-s", "-w", key, &value])
                .arg(path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
        };
        let source_ok = write("com.apple.metadata:kMDItemWhereFroms", strings(&[source]));
        let tag_ok = course
            .is_none_or(|name| write("com.apple.metadata:_kMDItemUserTags", strings(&[name])));
        source_ok && tag_ok
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, source, course);
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    fn finder_metadata_is_a_readable_plist_and_preserves_quarantine() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let path = std::env::temp_dir().join(format!(
            "moodburo-metadata-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, b"fixture").unwrap();
        assert!(Command::new("/usr/bin/xattr")
            .args(["-w", "com.apple.quarantine", "fixture-quarantine"])
            .arg(&path)
            .status()
            .unwrap()
            .success());
        assert!(annotate(
            &path,
            "https://example.org/course/view.php?id=2",
            Some("Public <&> Economics")
        ));
        for (key, expected) in [
            (
                "com.apple.metadata:kMDItemWhereFroms",
                "https://example.org/course/view.php?id=2",
            ),
            (
                "com.apple.metadata:_kMDItemUserTags",
                "Public <&> Economics",
            ),
        ] {
            let raw = Command::new("/usr/bin/xattr")
                .args(["-p", key])
                .arg(&path)
                .output()
                .unwrap();
            assert!(raw.status.success());
            let mut decoder = Command::new("/usr/bin/plutil")
                .args(["-extract", "0", "raw", "-o", "-", "-"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            decoder
                .stdin
                .take()
                .unwrap()
                .write_all(&raw.stdout)
                .unwrap();
            let decoded = decoder.wait_with_output().unwrap();
            assert!(decoded.status.success());
            assert_eq!(String::from_utf8(decoded.stdout).unwrap().trim(), expected);
        }
        let quarantine = Command::new("/usr/bin/xattr")
            .args(["-p", "com.apple.quarantine"])
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8(quarantine.stdout).unwrap().trim(),
            "fixture-quarantine"
        );
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn metadata_text_cannot_inject_properties() {
        let value = strings(&["Course <&>\"\u{0}é"]);
        assert!(value.contains("Course &lt;&amp;&gt;&quot;é"));
        assert!(!value.contains('\0'));
        assert_eq!(value.matches("<string>").count(), 1);
    }
}
