//! SQLite holds site metadata only. Never credentials, callback URLs or page content.
use rusqlite::{params, Connection};
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    pub wwwroot: String,
    pub name: String,
    pub user_id: i64,
    pub full_name: String,
}
pub struct Store(Connection);
impl Store {
    pub fn open(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; BEGIN;
            CREATE TABLE IF NOT EXISTS sites (id TEXT PRIMARY KEY, wwwroot TEXT NOT NULL UNIQUE, name TEXT NOT NULL, user_id INTEGER NOT NULL, full_name TEXT NOT NULL, position INTEGER NOT NULL DEFAULT 0);
            PRAGMA user_version=1; COMMIT;")?;
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
