use super::*;
#[derive(Clone)]
pub struct DownloadDetails {
    pub bytes: u64,
    pub source_page: String,
    pub metadata_applied: Option<bool>,
}
impl Store {
    pub fn download_details(&self, id: &str) -> rusqlite::Result<Option<DownloadDetails>> {
        use rusqlite::OptionalExtension;
        self.0.query_row("SELECT bytes,source_page,metadata_applied FROM download_details WHERE download_id=?1",[id],|r|Ok(DownloadDetails{bytes:r.get(0)?,source_page:r.get(1)?,metadata_applied:r.get(2)?})).optional()
    }
    pub fn save_download_details(
        &self,
        id: &str,
        user: i64,
        details: &DownloadDetails,
    ) -> rusqlite::Result<()> {
        let count=self.0.execute("INSERT INTO download_details SELECT d.id,?3,?4,?5 FROM downloads d JOIN sites s ON s.id=d.site_id WHERE d.id=?1 AND s.user_id=?2 AND d.state='complete' ON CONFLICT(download_id) DO UPDATE SET bytes=excluded.bytes,source_page=excluded.source_page,metadata_applied=excluded.metadata_applied",params![id,user,details.bytes,details.source_page,details.metadata_applied])?;
        if count == 1 {
            Ok(())
        } else {
            Err(rusqlite::Error::InvalidQuery)
        }
    }
    pub fn download_hash(&self, id: &str) -> rusqlite::Result<Option<String>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row(
                "SELECT sha256 FROM download_hashes WHERE download_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
    }
    pub fn download_file_referenced(&self, path: &str, except: &str) -> rusqlite::Result<bool> {
        self.0.query_row("SELECT EXISTS(SELECT 1 FROM offline_copies WHERE local_path=?1 UNION ALL SELECT 1 FROM downloads WHERE local_path=?1 AND id<>?2 AND state='complete')",params![path,except],|r|r.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_checks_shared_references_and_history_clear_only_forgets_metadata() {
        let store = Store::open(":memory:").unwrap();
        store
            .insert(&Site {
                id: "s".into(),
                wwwroot: "https://example.org/".into(),
                name: "Example".into(),
                user_id: 7,
                full_name: "Learner".into(),
            })
            .unwrap();
        let mut row = Download {
            id: "one".into(),
            site_id: "s".into(),
            filename: "notes.pdf".into(),
            local_path: "/Moodle/notes.pdf".into(),
            state: "complete".into(),
            course_id: None,
            course_name: None,
            started_at: 1,
        };
        store.download_insert(&row).unwrap();
        let details = DownloadDetails {
            bytes: 5,
            source_page: "https://example.org/my/".into(),
            metadata_applied: Some(true),
        };
        assert!(store.save_download_details("one", 8, &details).is_err());
        store.save_download_details("one", 7, &details).unwrap();
        store
            .fingerprint_download("one", 7, &"a".repeat(64))
            .unwrap();
        assert!(store.download_hash("one").unwrap().is_some());
        assert!(!store
            .download_file_referenced(&row.local_path, "one")
            .unwrap());
        row.id = "two".into();
        store.download_insert(&row).unwrap();
        assert!(store
            .download_file_referenced(&row.local_path, "one")
            .unwrap());
        store
            .download_finish("two", "/Moodle/.trash/c/notes.pdf", "removed")
            .unwrap();
        assert!(!store
            .download_file_referenced(&row.local_path, "one")
            .unwrap());
        store.downloads_clear(None).unwrap();
        assert!(store.download_details("one").unwrap().is_none());
        assert!(store.download_hash("one").unwrap().is_none());
    }
}
