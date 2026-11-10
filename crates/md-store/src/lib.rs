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
    pub course_id: Option<i64>,
    pub course_name: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
    pub site_id: String,
    pub id: i64,
    pub subject: String,
    pub time_created: i64,
    pub read: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    pub site_id: String,
    pub id: i64,
    pub name: String,
    pub short_name: String,
    pub pinned: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub site_id: String,
    pub id: i64,
    pub name: String,
    pub time_start: i64,
    pub duration: i64,
    pub kind: String,
    pub course_id: Option<i64>,
    pub course_name: Option<String>,
}
/// When an account's cached metadata (`courses` or `agenda`) was last refreshed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Refreshed {
    pub site_id: String,
    pub refreshed_at: i64,
}
#[derive(Default)]
pub struct DownloadFilter<'a> {
    pub query: &'a str,
    pub site_id: Option<&'a str>,
    pub state: Option<&'a str>,
    /// None = all, Some(0) = unknown course.
    pub course_id: Option<i64>,
}
fn download_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Download> {
    Ok(Download {
        id: r.get(0)?,
        site_id: r.get(1)?,
        filename: r.get(2)?,
        local_path: r.get(3)?,
        state: r.get(4)?,
        started_at: r.get(5)?,
        course_id: r.get(6)?,
        course_name: r.get(7)?,
    })
}
pub struct Store(Connection);
impl Store {
    pub fn open(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 6 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; BEGIN;
            CREATE TABLE IF NOT EXISTS sites (id TEXT PRIMARY KEY, wwwroot TEXT NOT NULL UNIQUE, name TEXT NOT NULL, user_id INTEGER NOT NULL, full_name TEXT NOT NULL, position INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS notification_state (site_id TEXT PRIMARY KEY REFERENCES sites(id) ON DELETE CASCADE, checkpoint INTEGER NOT NULL, unread INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS notifications (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, moodle_id INTEGER NOT NULL, subject TEXT NOT NULL, time_created INTEGER NOT NULL, read INTEGER NOT NULL, PRIMARY KEY(site_id,moodle_id));
            CREATE TABLE IF NOT EXISTS downloads (id TEXT PRIMARY KEY, site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, filename TEXT NOT NULL, local_path TEXT NOT NULL, state TEXT NOT NULL, started_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS courses (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, course_id INTEGER NOT NULL, name TEXT NOT NULL, short_name TEXT NOT NULL, PRIMARY KEY(site_id,course_id));
            CREATE TABLE IF NOT EXISTS course_pins (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, course_id INTEGER NOT NULL, PRIMARY KEY(site_id,course_id));
            CREATE TABLE IF NOT EXISTS events (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, event_id INTEGER NOT NULL, name TEXT NOT NULL, time_start INTEGER NOT NULL, duration INTEGER NOT NULL, kind TEXT NOT NULL, course_id INTEGER, course_name TEXT, PRIMARY KEY(site_id,event_id));
            CREATE TABLE IF NOT EXISTS sync_state (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, kind TEXT NOT NULL, refreshed_at INTEGER NOT NULL, PRIMARY KEY(site_id,kind));
            CREATE TABLE IF NOT EXISTS course_contexts (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, course_id INTEGER NOT NULL, context_id INTEGER NOT NULL, PRIMARY KEY(site_id,course_id,context_id));")?;
        if version < 6 {
            connection.execute_batch("ALTER TABLE downloads ADD COLUMN course_id INTEGER; ALTER TABLE downloads ADD COLUMN course_name TEXT;")?;
        }
        connection.execute_batch("PRAGMA user_version=6; COMMIT;")?;
        Ok(Self(connection))
    }
    pub fn download_insert(&self, item: &Download) -> rusqlite::Result<()> {
        self.0.execute(
            "INSERT INTO downloads VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                item.id,
                item.site_id,
                item.filename,
                item.local_path,
                item.state,
                item.started_at,
                item.course_id,
                item.course_name
            ],
        )?;
        Ok(())
    }
    pub fn download_insert_for_user(&self, item: &Download, user_id: i64) -> rusqlite::Result<()> {
        let inserted = self.0.execute(
            "INSERT INTO downloads SELECT ?1,?2,?3,?4,?5,?6,?8,?9 FROM sites WHERE id=?2 AND user_id=?7",
            params![
                item.id,
                item.site_id,
                item.filename,
                item.local_path,
                item.state,
                item.started_at,
                user_id, item.course_id, item.course_name
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
        let mut stmt = self.0.prepare("SELECT id,site_id,filename,local_path,state,started_at,course_id,course_name FROM downloads ORDER BY started_at DESC,rowid DESC LIMIT 20")?;
        let items = stmt.query_map([], download_row)?;
        items.collect()
    }
    pub fn download(&self, id: &str) -> rusqlite::Result<Option<Download>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row(
                "SELECT id,site_id,filename,local_path,state,started_at,course_id,course_name FROM downloads WHERE id=?1",
                [id],
                download_row,
            )
            .optional()
    }
    /// One page of history, newest first, with the total number of matching rows.
    pub fn downloads_page(
        &self,
        filter: &DownloadFilter<'_>,
        offset: u32,
        limit: u32,
    ) -> rusqlite::Result<(Vec<Download>, u64)> {
        // The filename filter is a literal substring: LIKE wildcards are escaped.
        let pattern = format!(
            "%{}%",
            filter
                .query
                .trim()
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        const WHERE: &str = "filename LIKE ?1 ESCAPE '\\' AND (?2 IS NULL OR site_id=?2) AND (?3 IS NULL OR state=?3) AND (?6 IS NULL OR (?6=0 AND course_id IS NULL) OR course_id=?6)";
        let total: u64 = self.0.query_row(
            &format!("SELECT COUNT(*) FROM downloads WHERE {WHERE}"),
            params![
                pattern,
                filter.site_id,
                filter.state,
                rusqlite::types::Null,
                rusqlite::types::Null,
                filter.course_id
            ],
            |r| r.get(0),
        )?;
        let mut stmt = self.0.prepare(&format!("SELECT id,site_id,filename,local_path,state,started_at,course_id,course_name FROM downloads WHERE {WHERE} ORDER BY started_at DESC,rowid DESC LIMIT ?4 OFFSET ?5"))?;
        let items = stmt
            .query_map(
                params![
                    pattern,
                    filter.site_id,
                    filter.state,
                    limit,
                    offset,
                    filter.course_id
                ],
                download_row,
            )?
            .collect::<rusqlite::Result<_>>()?;
        Ok((items, total))
    }
    /// Forgets finished history rows (never the files). Active transfers stay listed.
    pub fn downloads_clear(&self, site_id: Option<&str>) -> rusqlite::Result<usize> {
        self.0.execute(
            "DELETE FROM downloads WHERE state<>'downloading' AND (?1 IS NULL OR site_id=?1)",
            params![site_id],
        )
    }
    pub fn notifications_list(&self) -> rusqlite::Result<Vec<InboxItem>> {
        let mut stmt = self.0.prepare("SELECT site_id,moodle_id,subject,time_created,read FROM notifications ORDER BY time_created DESC,moodle_id DESC LIMIT 300")?;
        let items = stmt.query_map([], |r| {
            Ok(InboxItem {
                site_id: r.get(0)?,
                id: r.get(1)?,
                subject: r.get(2)?,
                time_created: r.get(3)?,
                read: r.get(4)?,
            })
        })?;
        items.collect()
    }
    pub fn notification_cached(&self, site_id: &str, id: i64) -> rusqlite::Result<bool> {
        self.0.query_row(
            "SELECT EXISTS (SELECT 1 FROM notifications WHERE site_id=?1 AND moodle_id=?2)",
            params![site_id, id],
            |r| r.get(0),
        )
    }
    /// Local read state after the server accepted it; `None` marks the whole account.
    /// Bound to the Moodle user that made the request, so a replaced account is untouched.
    pub fn notifications_mark_read(
        &self,
        site_id: &str,
        user_id: i64,
        id: Option<i64>,
    ) -> rusqlite::Result<usize> {
        self.0.execute(
            "UPDATE notifications SET read=1 WHERE site_id=?1 AND (?3 IS NULL OR moodle_id=?3) AND EXISTS (SELECT 1 FROM sites WHERE id=?1 AND user_id=?2)",
            params![site_id, user_id, id],
        )
    }
    fn owned_by(
        tx: &rusqlite::Transaction<'_>,
        site_id: &str,
        user_id: i64,
    ) -> rusqlite::Result<()> {
        let owned: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM sites WHERE id=?1 AND user_id=?2)",
            params![site_id, user_id],
            |r| r.get(0),
        )?;
        // A late response for a removed or replaced account must not be stored.
        if owned {
            Ok(())
        } else {
            Err(rusqlite::Error::InvalidQuery)
        }
    }
    /// Replaces the account's course list. Pins for courses no longer listed are dropped.
    pub fn save_courses(
        &self,
        site_id: &str,
        user_id: i64,
        items: &[Course],
        now: i64,
    ) -> rusqlite::Result<()> {
        let tx = self.0.unchecked_transaction()?;
        Self::owned_by(&tx, site_id, user_id)?;
        tx.execute("DELETE FROM courses WHERE site_id=?1", [site_id])?;
        for item in items {
            tx.execute(
                "INSERT OR REPLACE INTO courses VALUES (?1,?2,?3,?4)",
                params![site_id, item.id, item.name, item.short_name],
            )?;
        }
        tx.execute("DELETE FROM course_contexts WHERE site_id=?1 AND course_id NOT IN (SELECT course_id FROM courses WHERE site_id=?1)", [site_id])?;
        tx.execute("DELETE FROM course_pins WHERE site_id=?1 AND course_id NOT IN (SELECT course_id FROM courses WHERE site_id=?1)", [site_id])?;
        tx.execute("INSERT INTO sync_state VALUES (?1,'courses',?2) ON CONFLICT(site_id,kind) DO UPDATE SET refreshed_at=excluded.refreshed_at", params![site_id, now])?;
        tx.commit()
    }
    pub fn courses(&self) -> rusqlite::Result<Vec<Course>> {
        let mut stmt = self.0.prepare("SELECT c.site_id,c.course_id,c.name,c.short_name,p.course_id IS NOT NULL FROM courses c LEFT JOIN course_pins p ON p.site_id=c.site_id AND p.course_id=c.course_id ORDER BY c.name COLLATE NOCASE,c.course_id")?;
        let items = stmt.query_map([], |r| {
            Ok(Course {
                site_id: r.get(0)?,
                id: r.get(1)?,
                name: r.get(2)?,
                short_name: r.get(3)?,
                pinned: r.get(4)?,
            })
        })?;
        items.collect()
    }
    /// Replace only this account/course's authoritative module contexts. Never persist URLs.
    pub fn save_course_contexts(
        &self,
        site_id: &str,
        user_id: i64,
        course_id: i64,
        contexts: &[i64],
    ) -> rusqlite::Result<()> {
        let tx = self.0.unchecked_transaction()?;
        Self::owned_by(&tx, site_id, user_id)?;
        let known: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM courses WHERE site_id=?1 AND course_id=?2)",
            params![site_id, course_id],
            |r| r.get(0),
        )?;
        if !known {
            return Err(rusqlite::Error::InvalidQuery);
        }
        tx.execute(
            "DELETE FROM course_contexts WHERE site_id=?1 AND course_id=?2",
            params![site_id, course_id],
        )?;
        for context in contexts.iter().filter(|id| **id > 0).take(10000) {
            tx.execute(
                "INSERT OR IGNORE INTO course_contexts VALUES (?1,?2,?3)",
                params![site_id, course_id, context],
            )?;
        }
        tx.commit()
    }
    /// An ambiguous context deliberately falls back to Unknown Course.
    pub fn course_for_context(
        &self,
        site_id: &str,
        context: i64,
    ) -> rusqlite::Result<Option<Course>> {
        let ids = {
            let mut stmt = self.0.prepare(
                "SELECT course_id FROM course_contexts WHERE site_id=?1 AND context_id=?2 LIMIT 2",
            )?;
            let rows = stmt.query_map(params![site_id, context], |r| r.get::<_, i64>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        if ids.len() != 1 {
            return Ok(None);
        }
        Ok(self
            .courses()?
            .into_iter()
            .find(|c| c.site_id == site_id && c.id == ids[0]))
    }
    /// Pins are local only; they never change Moodle favourites. Unknown courses are refused.
    pub fn set_course_pin(
        &self,
        site_id: &str,
        course_id: i64,
        pinned: bool,
    ) -> rusqlite::Result<()> {
        let changed = if pinned {
            self.0.execute("INSERT OR IGNORE INTO course_pins SELECT site_id,course_id FROM courses WHERE site_id=?1 AND course_id=?2", params![site_id, course_id])?;
            self.0.query_row(
                "SELECT COUNT(*) FROM course_pins WHERE site_id=?1 AND course_id=?2",
                params![site_id, course_id],
                |r| r.get::<_, usize>(0),
            )?
        } else {
            self.0.execute(
                "DELETE FROM course_pins WHERE site_id=?1 AND course_id=?2",
                params![site_id, course_id],
            )?;
            1
        };
        if changed == 1 {
            Ok(())
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }
    }
    /// Replaces the account's upcoming events, so changed and cancelled entries reconcile.
    pub fn save_events(
        &self,
        site_id: &str,
        user_id: i64,
        items: &[Event],
        now: i64,
    ) -> rusqlite::Result<()> {
        let tx = self.0.unchecked_transaction()?;
        Self::owned_by(&tx, site_id, user_id)?;
        tx.execute("DELETE FROM events WHERE site_id=?1", [site_id])?;
        for item in items {
            tx.execute(
                "INSERT OR REPLACE INTO events VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    site_id,
                    item.id,
                    item.name,
                    item.time_start,
                    item.duration,
                    item.kind,
                    item.course_id,
                    item.course_name
                ],
            )?;
        }
        tx.execute("INSERT INTO sync_state VALUES (?1,'agenda',?2) ON CONFLICT(site_id,kind) DO UPDATE SET refreshed_at=excluded.refreshed_at", params![site_id, now])?;
        tx.commit()
    }
    /// Cached events that have not ended before `from`, soonest first.
    pub fn events(&self, from: i64) -> rusqlite::Result<Vec<Event>> {
        let mut stmt = self.0.prepare("SELECT site_id,event_id,name,time_start,duration,kind,course_id,course_name FROM events WHERE time_start+duration>=?1 ORDER BY time_start,event_id LIMIT 500")?;
        let items = stmt.query_map([from], |r| {
            Ok(Event {
                site_id: r.get(0)?,
                id: r.get(1)?,
                name: r.get(2)?,
                time_start: r.get(3)?,
                duration: r.get(4)?,
                kind: r.get(5)?,
                course_id: r.get(6)?,
                course_name: r.get(7)?,
            })
        })?;
        items.collect()
    }
    pub fn refreshed(&self, kind: &str) -> rusqlite::Result<Vec<Refreshed>> {
        let mut stmt = self
            .0
            .prepare("SELECT site_id,refreshed_at FROM sync_state WHERE kind=?1")?;
        let items = stmt.query_map([kind], |r| {
            Ok(Refreshed {
                site_id: r.get(0)?,
                refreshed_at: r.get(1)?,
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
        for table in [
            "notifications",
            "notification_state",
            "downloads",
            "courses",
            "course_contexts",
            "course_pins",
            "events",
            "sync_state",
        ] {
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
                    course_id: None,
                    course_name: None,
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
                course_id: None,
                course_name: None,
            })
            .unwrap();
        store.remove("site").unwrap();
        assert!(store.downloads_recent().unwrap().is_empty());
    }
}

#[cfg(test)]
mod library_tests {
    use super::*;
    fn store() -> (Store, Site) {
        let store = Store::open(":memory:").unwrap();
        let site = Site {
            id: "site".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 1,
            full_name: "Learner".into(),
        };
        store.insert(&site).unwrap();
        (store, site)
    }
    fn course(id: i64, name: &str) -> Course {
        Course {
            site_id: "site".into(),
            id,
            name: name.into(),
            short_name: String::new(),
            pinned: false,
        }
    }
    #[test]
    fn course_contexts_are_account_bound_and_ambiguous_contexts_are_unknown() {
        let (store, mut site) = store();
        store
            .save_courses("site", 1, &[course(2, "Law"), course(3, "Economics")], 1)
            .unwrap();
        store.save_course_contexts("site", 1, 2, &[50, 51]).unwrap();
        assert_eq!(store.course_for_context("site", 50).unwrap().unwrap().id, 2);
        assert!(store.course_for_context("other", 50).unwrap().is_none());
        assert!(store.save_course_contexts("site", 2, 2, &[60]).is_err());
        assert!(store.save_course_contexts("site", 1, 99, &[60]).is_err());
        store.save_course_contexts("site", 1, 3, &[50]).unwrap();
        assert!(store.course_for_context("site", 50).unwrap().is_none());
        store.save_course_contexts("site", 1, 2, &[52]).unwrap();
        assert!(store.course_for_context("site", 51).unwrap().is_none());
        site.user_id = 2;
        store.update(&site).unwrap();
        assert!(store.course_for_context("site", 52).unwrap().is_none());
        assert!(store.save_course_contexts("site", 1, 2, &[60]).is_err());
    }
    #[test]
    fn history_preserves_course_labels_and_filters_unknowns() {
        let (store, _) = store();
        for (id, course_id, course_name) in [
            ("known", Some(2), Some("Law".into())),
            ("unknown", None, None),
        ] {
            store
                .download_insert_for_user(
                    &Download {
                        id: id.into(),
                        site_id: "site".into(),
                        filename: "notes.pdf".into(),
                        local_path: "/Moodle/notes.pdf".into(),
                        state: "complete".into(),
                        started_at: 1,
                        course_id,
                        course_name,
                    },
                    1,
                )
                .unwrap();
        }
        let (known, total) = store
            .downloads_page(
                &DownloadFilter {
                    course_id: Some(2),
                    ..Default::default()
                },
                0,
                25,
            )
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(known[0].course_name.as_deref(), Some("Law"));
        let (unknown, total) = store
            .downloads_page(
                &DownloadFilter {
                    course_id: Some(0),
                    ..Default::default()
                },
                0,
                25,
            )
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(unknown[0].id, "unknown");
    }
    #[test]
    fn version_five_downloads_migrate_without_losing_files_or_rows() {
        // Use a real file to exercise Store::open's migration and its second-open path.
        let file = std::env::temp_dir().join(format!("moodburo-v5-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&file);
        let legacy = Connection::open(&file).unwrap();
        legacy.execute_batch("CREATE TABLE downloads(id TEXT PRIMARY KEY,site_id TEXT,filename TEXT,local_path TEXT,state TEXT,started_at INTEGER); INSERT INTO downloads VALUES ('legacy','site','notes.pdf','/Moodle/notes.pdf','complete',1); PRAGMA user_version=5;").unwrap();
        drop(legacy);
        for _ in 0..2 {
            let store = Store::open(&file).unwrap();
            let row = store.download("legacy").unwrap().unwrap();
            assert_eq!(row.local_path, "/Moodle/notes.pdf");
            assert!(row.course_id.is_none() && row.course_name.is_none());
        }
        std::fs::remove_file(&file).unwrap();
    }
    fn event(id: i64, time_start: i64) -> Event {
        Event {
            site_id: "site".into(),
            id,
            name: "Essay due".into(),
            time_start,
            duration: 0,
            kind: "due".into(),
            course_id: Some(7),
            course_name: Some("Law".into()),
        }
    }
    #[test]
    fn courses_pins_and_late_responses_are_account_bound() {
        let (store, mut site) = store();
        store
            .save_courses("site", 1, &[course(2, "zoology"), course(1, "Algebra")], 10)
            .unwrap();
        store.set_course_pin("site", 2, true).unwrap();
        store.set_course_pin("site", 2, true).unwrap();
        assert!(store.set_course_pin("site", 99, true).is_err());
        let courses = store.courses().unwrap();
        assert_eq!(courses[0].name, "Algebra");
        assert!(courses[1].pinned && !courses[0].pinned);
        assert_eq!(store.refreshed("courses").unwrap()[0].refreshed_at, 10);
        assert!(store.refreshed("agenda").unwrap().is_empty());
        // A refresh keeps pins for courses still listed and drops the others.
        store
            .save_courses("site", 1, &[course(1, "Algebra")], 11)
            .unwrap();
        assert!(store.courses().unwrap().iter().all(|c| !c.pinned));
        store.set_course_pin("site", 1, true).unwrap();
        // A response for the previous Moodle user is refused and changes nothing.
        assert!(store.save_courses("site", 2, &[], 12).is_err());
        assert_eq!(store.courses().unwrap().len(), 1);
        store.set_course_pin("site", 1, false).unwrap();
        store.set_course_pin("site", 1, true).unwrap();
        site.user_id = 2;
        store.update(&site).unwrap();
        assert!(store.courses().unwrap().is_empty());
        assert!(store.refreshed("courses").unwrap().is_empty());
        let pins: i64 = store
            .0
            .query_row("SELECT COUNT(*) FROM course_pins", [], |r| r.get(0))
            .unwrap();
        assert_eq!(pins, 0);
    }
    #[test]
    fn agenda_reconciles_changes_and_is_removed_with_the_account() {
        let (store, _) = store();
        store
            .save_events("site", 1, &[event(1, 100), event(2, 50), event(3, 10)], 5)
            .unwrap();
        let upcoming = store.events(40).unwrap();
        assert_eq!(
            upcoming.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![2, 1]
        );
        // Changed due date and a cancelled event after refresh.
        store.save_events("site", 1, &[event(1, 300)], 6).unwrap();
        let upcoming = store.events(0).unwrap();
        assert_eq!(upcoming.len(), 1);
        assert_eq!(upcoming[0].time_start, 300);
        assert!(store.save_events("site", 9, &[], 7).is_err());
        assert_eq!(store.refreshed("agenda").unwrap()[0].refreshed_at, 6);
        store.remove("site").unwrap();
        assert!(store.events(0).unwrap().is_empty());
        assert!(store.refreshed("agenda").unwrap().is_empty());
    }
    #[test]
    fn inbox_read_state_is_scoped_to_the_requesting_account() {
        let (store, _) = store();
        let items: Vec<_> = (1..=3)
            .map(|id| Notification {
                id,
                subject: format!("Notice {id}"),
                time_created: id,
                read: false,
            })
            .collect();
        store.save_notifications("site", 3, 3, &items).unwrap();
        assert!(store.notification_cached("site", 2).unwrap());
        assert!(!store.notification_cached("site", 9).unwrap());
        assert_eq!(
            store.notifications_mark_read("site", 2, Some(2)).unwrap(),
            0
        );
        assert_eq!(
            store.notifications_mark_read("site", 1, Some(2)).unwrap(),
            1
        );
        let list = store.notifications_list().unwrap();
        assert_eq!(list[0].id, 3);
        assert_eq!(
            list.iter().map(|n| n.read).collect::<Vec<_>>(),
            vec![false, true, false]
        );
        assert_eq!(store.notifications_mark_read("site", 1, None).unwrap(), 3);
        assert!(store.notifications_list().unwrap().iter().all(|n| n.read));
    }
    #[test]
    fn download_history_pages_filters_and_clears_without_touching_active_rows() {
        let (store, _) = store();
        for n in 0..30 {
            store
                .download_insert(&Download {
                    id: n.to_string(),
                    site_id: "site".into(),
                    filename: if n % 2 == 0 {
                        format!("notes_{n}.pdf")
                    } else {
                        format!("Slides 100% {n}.key")
                    },
                    local_path: "/home/Moodle/x".into(),
                    state: if n == 29 { "downloading" } else { "complete" }.into(),
                    started_at: n,
                    course_id: None,
                    course_name: None,
                })
                .unwrap();
        }
        let all = DownloadFilter::default();
        let (page, total) = store.downloads_page(&all, 0, 25).unwrap();
        assert_eq!((page.len(), total), (25, 30));
        assert_eq!(page[0].id, "29");
        assert_eq!(store.downloads_page(&all, 25, 25).unwrap().0.len(), 5);
        // Wildcards in the search text are literal.
        let filter = |query| DownloadFilter {
            query,
            ..Default::default()
        };
        assert_eq!(
            store.downloads_page(&filter("NOTES_"), 0, 50).unwrap().1,
            15
        );
        assert_eq!(store.downloads_page(&filter("100%"), 0, 50).unwrap().1, 15);
        assert_eq!(store.downloads_page(&filter("%"), 0, 50).unwrap().1, 15);
        let active = DownloadFilter {
            state: Some("downloading"),
            site_id: Some("site"),
            ..Default::default()
        };
        assert_eq!(store.downloads_page(&active, 0, 50).unwrap().1, 1);
        let other = DownloadFilter {
            site_id: Some("other"),
            ..Default::default()
        };
        assert_eq!(store.downloads_page(&other, 0, 50).unwrap().1, 0);
        assert_eq!(store.download("3").unwrap().unwrap().started_at, 3);
        assert!(store.download("missing").unwrap().is_none());
        assert_eq!(store.downloads_clear(Some("other")).unwrap(), 0);
        assert_eq!(store.downloads_clear(Some("site")).unwrap(), 29);
        assert_eq!(store.downloads_page(&all, 0, 50).unwrap().1, 1);
    }
}
