//! Handoff history only: no feed URL, token or calendar event contents.
use super::*;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarHandoff {
    pub site_id: String,
    pub user_id: i64,
    pub opened_at: i64,
    pub provider: String,
}
impl Store {
    pub fn calendar_handoffs(&self) -> rusqlite::Result<Vec<CalendarHandoff>> {
        let mut stmt=self.0.prepare("SELECT c.site_id,c.user_id,c.opened_at,c.provider FROM calendar_handoffs c JOIN sites s ON s.id=c.site_id AND s.user_id=c.user_id")?;
        let rows = stmt.query_map([], |r| {
            Ok(CalendarHandoff {
                site_id: r.get(0)?,
                user_id: r.get(1)?,
                opened_at: r.get(2)?,
                provider: r.get(3)?,
            })
        })?;
        rows.collect()
    }
    pub fn calendar_handed_off(
        &self,
        site: &str,
        user: i64,
        at: i64,
        provider: &str,
    ) -> rusqlite::Result<()> {
        let count=self.0.execute("INSERT INTO calendar_handoffs SELECT ?1,?2,?3,?4 FROM sites WHERE id=?1 AND user_id=?2 ON CONFLICT(site_id) DO UPDATE SET user_id=excluded.user_id,opened_at=excluded.opened_at,provider=excluded.provider",params![site,user,at,provider])?;
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
    fn handoff_is_not_subscription_confirmation_and_clears_on_account_changes() {
        let store = Store::open(":memory:").unwrap();
        let mut site = Site {
            id: "s".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 7,
            full_name: "Learner".into(),
        };
        store.insert(&site).unwrap();
        assert!(store.calendar_handoffs().unwrap().is_empty());
        assert!(store.calendar_handed_off("s", 8, 1, "moodle").is_err());
        store.calendar_handed_off("s", 7, 1, "moodle").unwrap();
        store.calendar_handed_off("s", 7, 2, "moodle").unwrap();
        assert_eq!(store.calendar_handoffs().unwrap().len(), 1);
        site.user_id = 8;
        store.update(&site).unwrap();
        assert!(store.calendar_handoffs().unwrap().is_empty());
        assert!(store.calendar_handed_off("s", 7, 3, "moodle").is_err());
        store.calendar_handed_off("s", 8, 3, "moodle").unwrap();
        store.remove("s").unwrap();
        assert!(store.calendar_handoffs().unwrap().is_empty());
    }
}
