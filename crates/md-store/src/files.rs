//! Account-scoped content lookup. Never persist a source URL or infer identity from size.
use super::*;
pub struct FileIdentity<'a> {
    pub site: &'a str,
    pub user: i64,
    pub course: i64,
    pub filename: &'a str,
    pub sha256: &'a str,
}
impl Store {
    pub fn shared_files(&self, file: &FileIdentity<'_>) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.0.prepare("SELECT local_path FROM (SELECT local_path,saved_at AS at FROM offline_copies WHERE site_id=?1 AND course_id=?2 AND filename=?3 AND sha256=?4 UNION ALL SELECT d.local_path,d.started_at AS at FROM downloads d JOIN download_hashes h ON h.download_id=d.id WHERE d.site_id=?1 AND d.course_id=?2 AND d.filename=?3 AND h.sha256=?4 AND d.state='complete') WHERE EXISTS(SELECT 1 FROM sites WHERE id=?1 AND user_id=?5) ORDER BY at DESC LIMIT 32")?;
        let rows = stmt
            .query_map(
                params![
                    file.site,
                    file.course,
                    file.filename,
                    file.sha256,
                    file.user
                ],
                |r| r.get(0),
            )?
            .collect();
        rows
    }
    pub fn fingerprint_download(&self, id: &str, user: i64, hash: &str) -> rusqlite::Result<()> {
        let count = self.0.execute("INSERT INTO download_hashes SELECT d.id,?3 FROM downloads d JOIN sites s ON s.id=d.site_id WHERE d.id=?1 AND s.user_id=?2 AND d.state='complete' AND d.course_id IS NOT NULL ON CONFLICT(download_id) DO UPDATE SET sha256=excluded.sha256", params![id,user,hash])?;
        if count == 1 {
            Ok(())
        } else {
            Err(rusqlite::Error::InvalidQuery)
        }
    }
    /// Any remaining reference keeps the file in place when an offline entry is removed.
    pub fn file_referenced(&self, path: &str, except_copy: &str) -> rusqlite::Result<bool> {
        self.0.query_row("SELECT EXISTS(SELECT 1 FROM offline_copies WHERE local_path=?1 AND id<>?2 UNION ALL SELECT 1 FROM downloads WHERE local_path=?1 AND state='complete')", params![path,except_copy], |r| r.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lookup_is_scoped_fingerprints_cascade_and_shared_offline_entries_survive_history_clear() {
        let store = Store::open(":memory:").unwrap();
        let mut site = Site {
            id: "s".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 7,
            full_name: "Learner".into(),
        };
        store.insert(&site).unwrap();
        let mut download = Download {
            id: "d".into(),
            site_id: "s".into(),
            filename: "notes.pdf".into(),
            local_path: "/Moodle/notes.pdf".into(),
            state: "downloading".into(),
            started_at: 10,
            course_id: Some(2),
            course_name: Some("Course".into()),
        };
        store.download_insert(&download).unwrap();
        assert!(store.fingerprint_download("d", 7, "hash").is_err());
        store
            .download_finish("d", &download.local_path, "complete")
            .unwrap();
        assert!(store.fingerprint_download("d", 8, "hash").is_err());
        store.fingerprint_download("d", 7, "hash").unwrap();
        let mut file = FileIdentity {
            site: "s",
            user: 7,
            course: 2,
            filename: "notes.pdf",
            sha256: "hash",
        };
        assert_eq!(
            store.shared_files(&file).unwrap(),
            vec![download.local_path.clone()]
        );
        file.user = 8;
        assert!(store.shared_files(&file).unwrap().is_empty());
        file.user = 7;
        file.course = 3;
        assert!(store.shared_files(&file).unwrap().is_empty());
        file.course = 2;
        file.filename = "notes.sh";
        assert!(store.shared_files(&file).unwrap().is_empty());
        file.filename = "notes.pdf";
        let copy = OfflineCopy {
            id: "c".into(),
            site_id: "s".into(),
            course_id: 2,
            resource_key: "a".repeat(64),
            module_id: 4,
            filename: "notes.pdf".into(),
            section_name: "Week 1".into(),
            local_path: download.local_path.clone(),
            sha256: "hash".into(),
            bytes: 10,
            saved_at: 11,
            refreshed_at: 11,
        };
        store.save_offline_copy(&copy, 7).unwrap();
        assert!(store.file_referenced(&copy.local_path, "c").unwrap());
        store.downloads_clear(Some("s")).unwrap();
        assert!(!store.file_referenced(&copy.local_path, "c").unwrap());
        assert_eq!(store.shared_files(&file).unwrap(), vec![copy.local_path]);
        assert_eq!(
            store
                .0
                .query_row("SELECT COUNT(*) FROM download_hashes", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        download.id = "new".into();
        download.state = "complete".into();
        store.download_insert(&download).unwrap();
        store.fingerprint_download("new", 7, "hash").unwrap();
        site.user_id = 8;
        store.update(&site).unwrap();
        assert!(store.shared_files(&file).unwrap().is_empty());
        assert!(store.fingerprint_download("new", 7, "hash").is_err());
    }
}
