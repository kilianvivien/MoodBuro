//! Assistant preferences and the account-scoped reads behind its tools.
//! Conversations are never written to SQLite: they live in memory until quit.
use super::{InboxItem, Store};
use rusqlite::{params, OptionalExtension};

const PREFIX: &str = "assistant:";

impl Store {
    pub fn assistant_value(&self, name: &str) -> rusqlite::Result<Option<String>> {
        self.0
            .query_row(
                "SELECT value FROM settings WHERE key=?1",
                [format!("{PREFIX}{name}")],
                |r| r.get(0),
            )
            .optional()
    }
    /// `None` forgets the value.
    pub fn set_assistant_value(&self, name: &str, value: Option<&str>) -> rusqlite::Result<()> {
        let key = format!("{PREFIX}{name}");
        match value {
            Some(value) => self.0.execute(
                "INSERT INTO settings(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![key, value],
            )?,
            None => self.0.execute("DELETE FROM settings WHERE key=?1", [key])?,
        };
        Ok(())
    }
    /// One account's cached notification subjects, newest first.
    pub fn notifications_for(&self, site_id: &str) -> rusqlite::Result<Vec<InboxItem>> {
        let mut stmt = self.0.prepare("SELECT site_id,moodle_id,subject,time_created,read FROM notifications WHERE site_id=?1 ORDER BY time_created DESC,moodle_id DESC LIMIT 100")?;
        let items = stmt.query_map([site_id], |r| {
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
}

#[cfg(test)]
mod tests {
    use crate::{Notification, Site, Store};
    fn store() -> Store {
        let store = Store::open(":memory:").unwrap();
        for id in ["a", "b"] {
            store
                .insert(&Site {
                    id: id.into(),
                    wwwroot: format!("https://{id}.example.org/"),
                    name: id.into(),
                    user_id: 1,
                    full_name: "Learner".into(),
                })
                .unwrap();
        }
        store
    }
    #[test]
    fn preferences_are_namespaced_and_removable() {
        let store = store();
        assert_eq!(store.assistant_value("provider").unwrap(), None);
        store
            .set_assistant_value("provider", Some("albert"))
            .unwrap();
        store
            .set_assistant_value("provider", Some("mistral"))
            .unwrap();
        assert_eq!(
            store.assistant_value("provider").unwrap().as_deref(),
            Some("mistral")
        );
        // Unknown keys leave the typed application settings untouched.
        assert_eq!(store.settings().unwrap(), Default::default());
        store.set_assistant_value("provider", None).unwrap();
        assert_eq!(store.assistant_value("provider").unwrap(), None);
    }
    #[test]
    fn notification_lookups_never_cross_accounts() {
        let store = store();
        let item = |id, subject: &str| Notification {
            id,
            subject: subject.into(),
            time_created: id,
            read: false,
        };
        store
            .save_notifications("a", 2, 2, &[item(1, "Old"), item(2, "New")])
            .unwrap();
        store
            .save_notifications("b", 9, 1, &[item(9, "Other account")])
            .unwrap();
        let subjects: Vec<_> = store
            .notifications_for("a")
            .unwrap()
            .into_iter()
            .map(|n| n.subject)
            .collect();
        assert_eq!(subjects, ["New", "Old"]);
        assert!(store.notifications_for("missing").unwrap().is_empty());
        // Replacing the account behind a site drops what the previous user could see.
        store
            .update(&Site {
                id: "a".into(),
                wwwroot: "https://a.example.org/".into(),
                name: "a".into(),
                user_id: 2,
                full_name: "Someone else".into(),
            })
            .unwrap();
        assert!(store.notifications_for("a").unwrap().is_empty());
        assert_eq!(store.notifications_for("b").unwrap().len(), 1);
    }
}
