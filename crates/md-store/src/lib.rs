//! SQLite holds account/preferences and bounded notification metadata. Never credentials,
//! callback URLs, notification context URLs or message bodies.
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    pub wwwroot: String,
    pub name: String,
    pub user_id: i64,
    pub full_name: String,
}
#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    System,
    Fr,
    En,
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Settings {
    pub appearance: Appearance,
    #[serde(default)]
    pub language: Language,
}
pub struct Notification {
    pub id: i64,
    pub subject: String,
    pub time_created: i64,
    pub read: bool,
}
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    pub id: String,
    pub site_id: String,
    pub filename: String,
    #[serde(skip_serializing)]
    pub local_path: String,
    pub state: String,
    pub started_at: i64,
}
pub struct Store(Connection);
impl Store {
    pub fn open(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 4 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; BEGIN;
            CREATE TABLE IF NOT EXISTS sites (id TEXT PRIMARY KEY, wwwroot TEXT NOT NULL UNIQUE, name TEXT NOT NULL, user_id INTEGER NOT NULL, full_name TEXT NOT NULL, position INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS notification_state (site_id TEXT PRIMARY KEY REFERENCES sites(id) ON DELETE CASCADE, checkpoint INTEGER NOT NULL, unread INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS notifications (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, moodle_id INTEGER NOT NULL, subject TEXT NOT NULL, time_created INTEGER NOT NULL, read INTEGER NOT NULL, PRIMARY KEY(site_id,moodle_id));
            CREATE TABLE IF NOT EXISTS downloads (id TEXT PRIMARY KEY, site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, filename TEXT NOT NULL, local_path TEXT NOT NULL, state TEXT NOT NULL, started_at INTEGER NOT NULL);
            PRAGMA user_version=4; COMMIT;")?;
        Ok(Self(connection))
    }
    pub fn download_insert(&self, item: &Download) -> rusqlite::Result<()> {
        self.0.execute(
            "INSERT INTO downloads VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                item.id,
                item.site_id,
                item.filename,
                item.local_path,
                item.state,
                item.started_at
            ],
        )?;
        Ok(())
    }
    pub fn download_insert_for_user(&self, item: &Download, user_id: i64) -> rusqlite::Result<()> {
        let inserted = self.0.execute(
            "INSERT INTO downloads SELECT ?1,?2,?3,?4,?5,?6 FROM sites WHERE id=?2 AND user_id=?7",
            params![
                item.id,
                item.site_id,
                item.filename,
                item.local_path,
                item.state,
                item.started_at,
                user_id
            ],
        )?;
        if inserted == 1 {
            Ok(())
        } else {
            Err(rusqlite::Error::InvalidQuery)
        }
    }
    pub fn download_finish(&self, id: &str, path: &str, state: &str) -> rusqlite::Result<()> {
        let filename = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download");
        self.0.execute(
            "UPDATE downloads SET local_path=?2,state=?3,filename=?4 WHERE id=?1",
            params![id, path, state, filename],
        )?;
        Ok(())
    }
    pub fn downloads_recent(&self) -> rusqlite::Result<Vec<Download>> {
        let mut stmt = self.0.prepare("SELECT id,site_id,filename,local_path,state,started_at FROM downloads ORDER BY started_at DESC,rowid DESC LIMIT 20")?;
        let items = stmt.query_map([], |r| {
            Ok(Download {
                id: r.get(0)?,
                site_id: r.get(1)?,
                filename: r.get(2)?,
                local_path: r.get(3)?,
                state: r.get(4)?,
                started_at: r.get(5)?,
            })
        })?;
        items.collect()
    }
    pub fn downloads_interrupt(&self) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE downloads SET state='failed' WHERE state='downloading'",
            [],
        )?;
        Ok(())
    }
    pub fn notification_state(&self, id: &str) -> rusqlite::Result<Option<(i64, u64)>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row(
                "SELECT checkpoint,unread FROM notification_state WHERE site_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
    }
    /// State and history advance together; an interrupted write cannot advance the checkpoint.
    pub fn save_notifications(
        &self,
        id: &str,
        checkpoint: i64,
        unread: u64,
        items: &[Notification],
    ) -> rusqlite::Result<()> {
        let tx = self.0.unchecked_transaction()?;
        for item in items {
            tx.execute("INSERT INTO notifications VALUES (?1,?2,?3,?4,?5) ON CONFLICT(site_id,moodle_id) DO UPDATE SET read=excluded.read", params![id,item.id,item.subject,item.time_created,item.read])?;
        }
        tx.execute("INSERT INTO notification_state VALUES (?1,?2,?3) ON CONFLICT(site_id) DO UPDATE SET checkpoint=excluded.checkpoint,unread=excluded.unread", params![id,checkpoint,unread.min(i64::MAX as u64)])?;
        // Keep the metadata cache bounded per organisation.
        tx.execute("DELETE FROM notifications WHERE site_id=?1 AND moodle_id NOT IN (SELECT moodle_id FROM notifications WHERE site_id=?1 ORDER BY moodle_id DESC LIMIT 100)", [id])?;
        tx.commit()
    }
    pub fn sites(&self) -> rusqlite::Result<Vec<Site>> {
        let mut statement = self.0.prepare(
            "SELECT id,wwwroot,name,user_id,full_name FROM sites ORDER BY position,rowid",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Site {
                id: row.get(0)?,
                wwwroot: row.get(1)?,
                name: row.get(2)?,
                user_id: row.get(3)?,
                full_name: row.get(4)?,
            })
        })?;
        rows.collect()
    }
    pub fn short_label(&self, id: &str) -> rusqlite::Result<Option<String>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row(
                "SELECT value FROM settings WHERE key=?1",
                [format!("site-label:{id}")],
                |r| r.get(0),
            )
            .optional()
    }
    pub fn set_short_label(&self, id: &str, label: Option<&str>) -> rusqlite::Result<()> {
        let key = format!("site-label:{id}");
        if let Some(label) = label {
            self.0.execute("INSERT INTO settings(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,label])?;
        } else {
            self.0.execute("DELETE FROM settings WHERE key=?1", [key])?;
        }
        Ok(())
    }
    pub fn settings(&self) -> rusqlite::Result<Settings> {
        let mut settings = Settings::default();
        let mut stmt = self.0.prepare("SELECT key,value FROM settings")?;
        let values =
            stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for pair in values {
            let (key, value) = pair?;
            match key.as_str() {
                "appearance" => {
                    settings.appearance = match value.as_str() {
                        "light" => Appearance::Light,
                        "dark" => Appearance::Dark,
                        _ => Appearance::System,
                    }
                }
                "language" => {
                    settings.language = match value.as_str() {
                        "fr" => Language::Fr,
                        "en" => Language::En,
                        _ => Language::System,
                    }
                }
                _ => {}
            }
        }
        Ok(settings)
    }
    pub fn set_preferences(
        &self,
        appearance: Option<Appearance>,
        language: Option<Language>,
    ) -> rusqlite::Result<Settings> {
        let tx = self.0.unchecked_transaction()?;
        if let Some(appearance) = appearance {
            let value = match appearance {
                Appearance::Light => "light",
                Appearance::Dark => "dark",
                Appearance::System => "system",
            };
            tx.execute("INSERT INTO settings(key,value) VALUES ('appearance',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [value])?;
        }
        if let Some(language) = language {
            let value = match language {
                Language::Fr => "fr",
                Language::En => "en",
                Language::System => "system",
            };
            tx.execute("INSERT INTO settings(key,value) VALUES ('language',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [value])?;
        }
        tx.commit()?;
        self.settings()
    }
    pub fn set_appearance(&self, appearance: Appearance) -> rusqlite::Result<Settings> {
        self.set_preferences(Some(appearance), None)
    }
    pub fn update(&self, site: &Site) -> rusqlite::Result<()> {
        let tx = self.0.unchecked_transaction()?;
        // A different account must start with a fresh, silent notification seed.
        for table in ["notifications", "notification_state", "downloads"] {
            tx.execute(&format!("DELETE FROM {table} WHERE site_id=?1 AND EXISTS (SELECT 1 FROM sites WHERE id=?1 AND user_id<>?2)"), params![site.id, site.user_id])?;
        }
        tx.execute(
            "UPDATE sites SET name=?2,user_id=?3,full_name=?4,wwwroot=?5 WHERE id=?1",
            params![
                site.id,
                site.name,
                site.user_id,
                site.full_name,
                site.wwwroot
            ],
        )?;
        tx.commit()
    }
    pub fn remove(&self, id: &str) -> rusqlite::Result<()> {
        let tx = self.0.unchecked_transaction()?;
        tx.execute("DELETE FROM sites WHERE id=?1", [id])?;
        tx.execute(
            "DELETE FROM settings WHERE key=?1",
            [format!("site-label:{id}")],
        )?;
        tx.commit()
    }

    pub fn insert(&self, site: &Site) -> rusqlite::Result<()> {
        self.0.execute(
            "INSERT INTO sites(id,wwwroot,name,user_id,full_name) VALUES (?1,?2,?3,?4,?5)",
            params![
                site.id,
                site.wwwroot,
                site.name,
                site.user_id,
                site.full_name
            ],
        )?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_and_unique_site() {
        let store = Store::open(":memory:").unwrap();
        assert!(store.sites().unwrap().is_empty());
        let site = Site {
            id: "id".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 2,
            full_name: "Learner".into(),
        };
        store.insert(&site).unwrap();
        assert_eq!(store.sites().unwrap()[0].user_id, 2);
        assert!(store.insert(&site).is_err());
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn settings_survive_reopen_and_existing_site_is_migrated() {
        // Exercise the real version-1 schema before opening the new store.
        let path =
            std::env::temp_dir().join(format!("moodburo-migration-{}.db", std::process::id()));
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE sites(id TEXT PRIMARY KEY,wwwroot TEXT NOT NULL UNIQUE,name TEXT NOT NULL,user_id INTEGER NOT NULL,full_name TEXT NOT NULL,position INTEGER NOT NULL DEFAULT 0); INSERT INTO sites VALUES ('one','https://example.org/','Original',1,'Learner',0); PRAGMA user_version=1;").unwrap();
        drop(conn);
        {
            let store = Store::open(&path).unwrap();
            assert_eq!(store.settings().unwrap().appearance, Appearance::System);
            store.set_appearance(Appearance::Dark).unwrap();
            assert_eq!(store.sites().unwrap()[0].name, "Original");
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(store.settings().unwrap().appearance, Appearance::Dark);
        store.remove("one").unwrap();
        assert!(store.sites().unwrap().is_empty());
        drop(store);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn reconnect_updates_the_existing_account() {
        let store = Store::open(":memory:").unwrap();
        let mut site = Site {
            id: "one".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 1,
            full_name: "Old".into(),
        };
        store.insert(&site).unwrap();
        store.save_notifications("one", 8, 3, &[]).unwrap();
        store.update(&site).unwrap();
        assert_eq!(store.notification_state("one").unwrap(), Some((8, 3)));
        site.user_id = 2;
        site.full_name = "New".into();
        store.update(&site).unwrap();
        assert_eq!(store.sites().unwrap().len(), 1);
        assert_eq!(store.sites().unwrap()[0].user_id, 2);
        assert!(store.notification_state("one").unwrap().is_none());
    }
}

#[cfg(test)]
mod language_tests {
    use super::*;
    #[test]
    fn language_and_appearance_patches_preserve_each_other() {
        let store = Store::open(":memory:").unwrap();
        assert_eq!(store.settings().unwrap().language, Language::System);
        store.set_preferences(None, Some(Language::Fr)).unwrap();
        let settings = store.set_preferences(Some(Appearance::Dark), None).unwrap();
        assert_eq!(settings.language, Language::Fr);
        let settings = store.set_preferences(None, Some(Language::En)).unwrap();
        assert_eq!(settings.appearance, Appearance::Dark);
        assert_eq!(settings.language, Language::En);
    }
}

#[cfg(test)]
mod label_tests {
    use super::*;
    #[test]
    fn labels_are_per_site_and_removed_with_account() {
        let store = Store::open(":memory:").unwrap();
        store.set_short_label("one", Some("TEST")).unwrap();
        store.set_short_label("two", Some("ABC")).unwrap();
        assert_eq!(store.short_label("one").unwrap().as_deref(), Some("TEST"));
        store.remove("one").unwrap();
        assert!(store.short_label("one").unwrap().is_none());
        assert_eq!(store.short_label("two").unwrap().as_deref(), Some("ABC"));
        store.set_short_label("two", None).unwrap();
        assert!(store.short_label("two").unwrap().is_none());
    }
}

#[cfg(test)]
mod notification_tests {
    use super::*;
    #[test]
    fn checkpoint_history_and_account_cleanup_are_atomic() {
        let store = Store::open(":memory:").unwrap();
        store
            .insert(&Site {
                id: "one".into(),
                wwwroot: "https://example.org/".into(),
                name: "Example".into(),
                user_id: 1,
                full_name: "Learner".into(),
            })
            .unwrap();
        assert!(store.notification_state("one").unwrap().is_none());
        let items: Vec<_> = (1..=110)
            .map(|id| Notification {
                id,
                subject: "Notice".into(),
                time_created: 1,
                read: false,
            })
            .collect();
        store.save_notifications("one", 110, 4, &items).unwrap();
        assert_eq!(store.notification_state("one").unwrap(), Some((110, 4)));
        let count: i64 = store
            .0
            .query_row("SELECT COUNT(*) FROM notifications", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 100);
        assert!(store.save_notifications("missing", 111, 5, &items).is_err());
        assert_eq!(store.notification_state("one").unwrap(), Some((110, 4)));
        store.remove("one").unwrap();
        assert!(store.notification_state("one").unwrap().is_none());
        let count: i64 = store
            .0
            .query_row("SELECT COUNT(*) FROM notifications", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}

#[cfg(test)]
mod download_tests {
    use super::*;
    #[test]
    fn interrupted_history_final_names_and_cascade_cleanup() {
        let store = Store::open(":memory:").unwrap();
        let mut site = Site {
            id: "site".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 1,
            full_name: "Learner".into(),
        };
        store.insert(&site).unwrap();
        for n in 0..21 {
            store
                .download_insert(&Download {
                    id: n.to_string(),
                    site_id: site.id.clone(),
                    filename: "notes.pdf".into(),
                    local_path: "/private/staging/notes.pdf".into(),
                    state: "downloading".into(),
                    started_at: n,
                })
                .unwrap();
        }
        assert_eq!(store.downloads_recent().unwrap().len(), 20);
        store
            .download_finish("20", "/home/Moodle/notes (2).pdf", "complete")
            .unwrap();
        store.downloads_interrupt().unwrap();
        let recent = store.downloads_recent().unwrap();
        assert_eq!(recent[0].filename, "notes (2).pdf");
        assert_eq!(recent[0].state, "complete");
        assert_eq!(recent[1].state, "failed");
        store.update(&site).unwrap();
        assert_eq!(store.downloads_recent().unwrap().len(), 20);
        site.user_id = 2;
        store.update(&site).unwrap();
        assert!(store.downloads_recent().unwrap().is_empty());
        store
            .download_insert(&Download {
                id: "new".into(),
                site_id: site.id.clone(),
                filename: "file.pdf".into(),
                local_path: "/home/Moodle/file.pdf".into(),
                state: "complete".into(),
                started_at: 22,
            })
            .unwrap();
        store.remove("site").unwrap();
        assert!(store.downloads_recent().unwrap().is_empty());
    }
}
