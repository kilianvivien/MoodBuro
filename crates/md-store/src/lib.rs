//! SQLite holds site metadata only. Never credentials, callback URLs or page content.
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
pub struct Store(Connection);
impl Store {
    pub fn open(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 2 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; BEGIN;
            CREATE TABLE IF NOT EXISTS sites (id TEXT PRIMARY KEY, wwwroot TEXT NOT NULL UNIQUE, name TEXT NOT NULL, user_id INTEGER NOT NULL, full_name TEXT NOT NULL, position INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            PRAGMA user_version=2; COMMIT;")?;
        Ok(Self(connection))
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
        self.0.execute(
            "UPDATE sites SET name=?2,user_id=?3,full_name=?4,wwwroot=?5 WHERE id=?1",
            params![
                site.id,
                site.name,
                site.user_id,
                site.full_name,
                site.wwwroot
            ],
        )?;
        Ok(())
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
        site.user_id = 2;
        site.full_name = "New".into();
        store.update(&site).unwrap();
        assert_eq!(store.sites().unwrap().len(), 1);
        assert_eq!(store.sites().unwrap()[0].user_id, 2);
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
