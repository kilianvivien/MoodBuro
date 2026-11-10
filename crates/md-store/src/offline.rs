use super::*;
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineCopy {
    pub id: String,
    pub site_id: String,
    pub course_id: i64,
    pub resource_key: String,
    pub module_id: i64,
    pub filename: String,
    pub section_name: String,
    #[serde(skip_serializing)]
    pub local_path: String,
    #[serde(skip_serializing)]
    pub sha256: String,
    pub bytes: u64,
    pub saved_at: i64,
}
impl Store {
    pub fn offline_copies(&self, site: &str, course: i64) -> rusqlite::Result<Vec<OfflineCopy>> {
        let mut stmt = self.0.prepare("SELECT id,site_id,course_id,resource_key,module_id,filename,section_name,local_path,sha256,bytes,saved_at FROM offline_copies WHERE site_id=?1 AND course_id=?2 ORDER BY saved_at DESC,rowid DESC LIMIT 2000")?;
        let rows = stmt.query_map(params![site, course], |r| {
            Ok(OfflineCopy {
                id: r.get(0)?,
                site_id: r.get(1)?,
                course_id: r.get(2)?,
                resource_key: r.get(3)?,
                module_id: r.get(4)?,
                filename: r.get(5)?,
                section_name: r.get(6)?,
                local_path: r.get(7)?,
                sha256: r.get(8)?,
                bytes: r.get(9)?,
                saved_at: r.get(10)?,
            })
        })?;
        rows.collect()
    }
    pub fn save_offline_copy(
        &self,
        copy: &OfflineCopy,
        expected_user: i64,
    ) -> rusqlite::Result<()> {
        let count = self.0.execute("INSERT INTO offline_copies SELECT ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11 FROM sites WHERE id=?2 AND user_id=?12", params![copy.id,copy.site_id,copy.course_id,copy.resource_key,copy.module_id,copy.filename,copy.section_name,copy.local_path,copy.sha256,copy.bytes,copy.saved_at,expected_user])?;
        if count == 1 {
            Ok(())
        } else {
            Err(rusqlite::Error::InvalidQuery)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copies_persist_migrate_and_clear_only_metadata_on_account_replacement() {
        let root =
            std::env::temp_dir().join(format!("moodburo-offline-store-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let database = root.join("index.db");
        let file = root.join("notes.txt");
        std::fs::write(&file, "user content").unwrap();
        let mut site = Site {
            id: "s".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 7,
            full_name: "Learner".into(),
        };
        let mut copy = OfflineCopy {
            id: "copy1".into(),
            site_id: "s".into(),
            course_id: 2,
            resource_key: "key".into(),
            module_id: 42,
            filename: "notes.txt".into(),
            section_name: "Week 1".into(),
            local_path: file.to_string_lossy().into_owned(),
            sha256: "hash".into(),
            bytes: 12,
            saved_at: 10,
        };
        {
            let store = Store::open(&database).unwrap();
            store.insert(&site).unwrap();
            store.save_offline_copy(&copy, 7).unwrap();
            // Simulate a pre-offline-schema database while keeping legacy account metadata.
            store.0.execute_batch("PRAGMA user_version=7").unwrap();
        }
        let store = Store::open(&database).unwrap();
        assert_eq!(store.sites().unwrap()[0].user_id, 7);
        assert_eq!(store.offline_copies("s", 2).unwrap()[0].id, "copy1");
        assert!(store.offline_copies("s", 3).unwrap().is_empty());
        copy.id = "copy2".into();
        copy.saved_at = 11;
        store.save_offline_copy(&copy, 7).unwrap();
        assert_eq!(store.offline_copies("s", 2).unwrap()[0].id, "copy2");
        site.user_id = 8;
        store.update(&site).unwrap();
        assert!(store.offline_copies("s", 2).unwrap().is_empty());
        assert!(store.save_offline_copy(&copy, 7).is_err());
        store.save_offline_copy(&copy, 8).unwrap();
        store.remove("s").unwrap();
        assert!(store.offline_copies("s", 2).unwrap().is_empty());
        assert!(store.save_offline_copy(&copy, 8).is_err());
        assert_eq!(std::fs::read_to_string(file).unwrap(), "user content");
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
