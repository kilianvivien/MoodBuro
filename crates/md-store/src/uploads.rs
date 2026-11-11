//! Submission-operation journal. File paths, tokens and file contents are not stored.
use super::*;
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftUpload {
    pub id: String,
    pub site_id: String,
    pub user_id: i64,
    pub course_id: i64,
    pub assignment_id: i64,
    pub module_id: i64,
    pub filename: String,
    pub bytes: u64,
    #[serde(skip_serializing)]
    pub sha256: String,
    pub state: String,
    pub created_at: i64,
    pub draft_id: Option<i64>,
}
impl Store {
    pub fn draft_uploads(&self) -> rusqlite::Result<Vec<DraftUpload>> {
        let mut stmt=self.0.prepare("SELECT u.id,u.site_id,u.user_id,u.course_id,u.assignment_id,u.module_id,u.filename,u.bytes,u.sha256,u.state,u.created_at,u.draft_id FROM draft_uploads u JOIN sites s ON s.id=u.site_id AND s.user_id=u.user_id ORDER BY u.created_at DESC,u.rowid DESC LIMIT 100")?;
        let rows = stmt.query_map([], |r| {
            Ok(DraftUpload {
                id: r.get(0)?,
                site_id: r.get(1)?,
                user_id: r.get(2)?,
                course_id: r.get(3)?,
                assignment_id: r.get(4)?,
                module_id: r.get(5)?,
                filename: r.get(6)?,
                bytes: r.get(7)?,
                sha256: r.get(8)?,
                state: r.get(9)?,
                created_at: r.get(10)?,
                draft_id: r.get(11)?,
            })
        })?;
        rows.collect()
    }
    pub fn upload_start(&self, u: &DraftUpload) -> rusqlite::Result<()> {
        let count=self.0.execute("INSERT INTO draft_uploads SELECT ?1,?2,?3,?4,?5,?6,?7,?8,?9,'started',?10,NULL FROM sites WHERE id=?2 AND user_id=?3 AND NOT EXISTS(SELECT 1 FROM draft_uploads WHERE site_id=?2 AND user_id=?3 AND assignment_id=?5 AND state IN ('started','uploaded','saving','uncertain'))",params![u.id,u.site_id,u.user_id,u.course_id,u.assignment_id,u.module_id,u.filename,u.bytes,u.sha256,u.created_at])?;
        if count == 1 {
            Ok(())
        } else {
            Err(rusqlite::Error::InvalidQuery)
        }
    }
    pub fn upload_state(
        &self,
        id: &str,
        user: i64,
        state: &str,
        draft: Option<i64>,
    ) -> rusqlite::Result<()> {
        if !matches!(
            state,
            "uploaded" | "saving" | "saved" | "uncertain" | "failed" | "reviewed"
        ) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let count=self.0.execute("UPDATE draft_uploads SET state=?3,draft_id=COALESCE(?4,draft_id) WHERE id=?1 AND user_id=?2 AND EXISTS(SELECT 1 FROM sites WHERE id=draft_uploads.site_id AND user_id=?2)",params![id,user,state,draft])?;
        if count == 1 {
            Ok(())
        } else {
            Err(rusqlite::Error::InvalidQuery)
        }
    }
    pub fn recover_uploads(&self) -> rusqlite::Result<()> {
        self.0.execute("UPDATE draft_uploads SET state='uncertain' WHERE state IN ('started','uploaded','saving')",[])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn site() -> Site {
        Site {
            id: "s".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 7,
            full_name: "Learner".into(),
        }
    }
    fn upload(id: &str) -> DraftUpload {
        DraftUpload {
            id: id.into(),
            site_id: "s".into(),
            user_id: 7,
            course_id: 2,
            assignment_id: 9,
            module_id: 19,
            filename: "essay.pdf".into(),
            bytes: 5,
            sha256: "a".repeat(64),
            state: "started".into(),
            created_at: 1,
            draft_id: None,
        }
    }
    #[test]
    fn restart_never_replays_a_write_and_uncertainty_blocks_another_attempt() {
        let path = std::env::temp_dir().join(format!(
            "moodburo-upload-journal-{}.sqlite",
            std::process::id()
        ));
        let store = Store::open(&path).unwrap();
        store.insert(&site()).unwrap();
        store.upload_start(&upload("one")).unwrap();
        store.upload_state("one", 7, "uploaded", Some(99)).unwrap();
        drop(store);
        let store = Store::open(&path).unwrap();
        store.recover_uploads().unwrap();
        let rows = store.draft_uploads().unwrap();
        assert_eq!(rows[0].state, "uncertain");
        assert_eq!(rows[0].draft_id, Some(99));
        assert!(store.upload_start(&upload("two")).is_err());
        assert!(store.upload_state("one", 8, "reviewed", None).is_err());
        store.upload_state("one", 7, "reviewed", None).unwrap();
        store.upload_start(&upload("two")).unwrap();
        let serialized = serde_json::to_string(&store.draft_uploads().unwrap()).unwrap();
        assert!(!serialized.contains("sha256"));
        assert!(!serialized.contains("localPath"));
        let mut changed = site();
        changed.user_id = 8;
        store.update(&changed).unwrap();
        assert!(store.draft_uploads().unwrap().is_empty());
        assert!(store.upload_start(&upload("old-account")).is_err());
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn removal_cascades_and_preferences_default_off() {
        let store = Store::open(":memory:").unwrap();
        store.insert(&site()).unwrap();
        assert!(!store.menu_bar_only().unwrap());
        store.set_menu_bar_only(true).unwrap();
        assert!(store.menu_bar_only().unwrap());
        store.upload_start(&upload("one")).unwrap();
        store.remove("s").unwrap();
        assert!(store.draft_uploads().unwrap().is_empty());
    }
}
