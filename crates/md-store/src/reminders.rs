use super::*;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderSettings {
    pub site_id: String,
    pub user_id: i64,
    pub hour: bool,
    pub day: bool,
    pub enabled_at: i64,
    pub refreshed_at: Option<i64>,
    pub refresh_error: Option<String>,
    pub delivery_failed: bool,
}
pub struct DueReminder {
    pub site_id: String,
    pub user_id: i64,
    pub event_id: i64,
    pub due: i64,
    pub offset: i64,
    pub name: String,
    pub site_name: String,
}
/// A one-off reminder the user asked for on a single deadline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderPick {
    pub site_id: String,
    pub event_id: i64,
    /// Seconds before the deadline: 3600 or 86400.
    pub offset: i64,
    pub due: i64,
    pub name: String,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PickError {
    /// The site is saved for another account.
    Account,
    /// Not a cached deadline that reminders can cover.
    Event,
    /// The moment to remind at has already passed.
    Passed,
    Storage,
}
impl From<rusqlite::Error> for PickError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Storage
    }
}
impl Store {
    /// Upcoming one-off reminders of the accounts currently saved.
    pub fn reminder_picks(&self) -> rusqlite::Result<Vec<ReminderPick>> {
        let mut query = self.0.prepare("SELECT p.site_id,p.event_id,p.offset,COALESCE(e.time_start,p.due),COALESCE(e.name,p.name) FROM reminder_picks p JOIN sites s ON s.id=p.site_id AND s.user_id=p.user_id LEFT JOIN events e ON e.site_id=p.site_id AND e.event_id=p.event_id ORDER BY 4,p.event_id,p.offset LIMIT 500")?;
        let rows = query.query_map([], |r| {
            Ok(ReminderPick {
                site_id: r.get(0)?,
                event_id: r.get(1)?,
                offset: r.get(2)?,
                due: r.get(3)?,
                name: r.get(4)?,
            })
        })?;
        rows.collect()
    }
    /// Adds a one-off reminder for a cached, eligible deadline whose reminder time is
    /// still ahead. A reminder that could never fire is refused, not stored.
    pub fn add_reminder_pick(
        &self,
        site: &str,
        user: i64,
        event: i64,
        offset: i64,
        now: i64,
    ) -> Result<ReminderPick, PickError> {
        use rusqlite::OptionalExtension;
        if !matches!(offset, 3600 | 86400) {
            return Err(PickError::Event);
        }
        let tx = self.0.unchecked_transaction()?;
        Self::owned_by(&tx, site, user).map_err(|_| PickError::Account)?;
        let (due, name): (i64, String) = tx
            .query_row("SELECT e.time_start,e.name FROM events e JOIN reminder_events r ON r.site_id=e.site_id AND r.event_id=e.event_id AND r.due=e.time_start WHERE e.site_id=?1 AND e.event_id=?2 AND e.kind='due' AND r.eligible=1", params![site, event], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?
            .ok_or(PickError::Event)?;
        if due - offset <= now {
            return Err(PickError::Passed);
        }
        tx.execute("INSERT INTO reminder_picks VALUES (?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(site_id,event_id,offset) DO UPDATE SET user_id=excluded.user_id,due=excluded.due,name=excluded.name,created_at=excluded.created_at", params![site, user, event, offset, due, name, now])?;
        tx.commit()?;
        Ok(ReminderPick {
            site_id: site.to_owned(),
            event_id: event,
            offset,
            due,
            name,
        })
    }
    /// The cached event is a deadline reminders can cover (not completed or closed).
    pub fn reminder_eligible(&self, site: &str, event: i64) -> rusqlite::Result<bool> {
        self.0.query_row("SELECT EXISTS (SELECT 1 FROM events e JOIN reminder_events r ON r.site_id=e.site_id AND r.event_id=e.event_id AND r.due=e.time_start WHERE e.site_id=?1 AND e.event_id=?2 AND e.kind='due' AND r.eligible=1)", params![site, event], |r| r.get(0))
    }
    pub fn remove_reminder_pick(
        &self,
        site: &str,
        user: i64,
        event: i64,
        offset: i64,
    ) -> rusqlite::Result<()> {
        self.0.execute("DELETE FROM reminder_picks WHERE site_id=?1 AND user_id=?2 AND event_id=?3 AND offset=?4", params![site, user, event, offset])?;
        Ok(())
    }
    /// One-off reminders now due. Claimed rows are deleted before delivery, so each is
    /// attempted at most once. The same freshness rules as `claim_reminders` apply, and a
    /// reminder the organisation-wide setting already claimed is dropped silently. Call
    /// after `claim_reminders` in the same tick.
    pub fn claim_reminder_picks(
        &self,
        last: i64,
        now: i64,
        fresh_after: i64,
    ) -> rusqlite::Result<Vec<DueReminder>> {
        if now < last || now.saturating_sub(last) > 60 {
            return Ok(Vec::new());
        }
        let tx = self.0.unchecked_transaction()?;
        let due = {
            let mut query = tx.prepare("SELECT s.id,s.user_id,e.event_id,e.time_start,p.offset,e.name,s.name,
            EXISTS (SELECT 1 FROM reminder_ledger l WHERE l.site_id=s.id AND l.user_id=s.user_id AND l.event_id=e.event_id AND l.due=e.time_start AND l.offset=p.offset)
            FROM reminder_picks p JOIN sites s ON s.id=p.site_id AND s.user_id=p.user_id JOIN events e ON e.site_id=p.site_id AND e.event_id=p.event_id JOIN reminder_events r ON r.site_id=e.site_id AND r.event_id=e.event_id AND r.due=e.time_start JOIN sync_state a ON a.site_id=s.id AND a.kind='agenda'
            WHERE e.kind='due' AND r.eligible=1 AND e.time_start>?2 AND e.time_start-p.offset>?1 AND e.time_start-p.offset<=?2 AND e.time_start-p.offset>p.created_at AND e.time_start-p.offset>r.observed_at AND a.refreshed_at>=?2-900 AND a.refreshed_at>=?3 AND a.refreshed_at<=?2 ORDER BY e.time_start,s.position,e.event_id")?;
            let rows = query.query_map(params![last, now, fresh_after], |r| {
                Ok((
                    DueReminder {
                        site_id: r.get(0)?,
                        user_id: r.get(1)?,
                        event_id: r.get(2)?,
                        due: r.get(3)?,
                        offset: r.get(4)?,
                        name: r.get(5)?,
                        site_name: r.get(6)?,
                    },
                    r.get::<_, bool>(7)?,
                ))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut items = Vec::new();
        for (item, already) in due {
            tx.execute(
                "DELETE FROM reminder_picks WHERE site_id=?1 AND event_id=?2 AND offset=?3",
                params![item.site_id, item.event_id, item.offset],
            )?;
            if !already {
                items.push(item);
            }
        }
        // Reminders whose moment passed unclaimed (asleep, offline, date moved) never fire late.
        tx.execute("DELETE FROM reminder_picks WHERE COALESCE((SELECT e.time_start FROM events e WHERE e.site_id=reminder_picks.site_id AND e.event_id=reminder_picks.event_id),due)-offset<=?1", [last])?;
        tx.commit()?;
        Ok(items)
    }
    /// Accounts with a one-off reminder waiting: their agenda must stay fresh too.
    pub fn reminder_pick_accounts(&self) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut query = self.0.prepare("SELECT DISTINCT p.site_id,p.user_id FROM reminder_picks p JOIN sites s ON s.id=p.site_id AND s.user_id=p.user_id")?;
        let rows = query.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect()
    }
    pub fn reminders(&self) -> rusqlite::Result<Vec<ReminderSettings>> {
        let mut query = self.0.prepare("SELECT s.id,s.user_id,COALESCE(r.hour,0),COALESCE(r.day,0),COALESCE(r.enabled_at,0),a.refreshed_at,r.refresh_error,COALESCE(r.delivery_failed,0) FROM sites s LEFT JOIN reminder_settings r ON r.site_id=s.id AND r.user_id=s.user_id LEFT JOIN sync_state a ON a.site_id=s.id AND a.kind='agenda' ORDER BY s.position,s.rowid")?;
        let rows = query.query_map([], |r| {
            Ok(ReminderSettings {
                site_id: r.get(0)?,
                user_id: r.get(1)?,
                hour: r.get(2)?,
                day: r.get(3)?,
                enabled_at: r.get(4)?,
                refreshed_at: r.get(5)?,
                refresh_error: r.get(6)?,
                delivery_failed: r.get(7)?,
            })
        })?;
        rows.collect()
    }
    pub fn set_reminders(
        &self,
        site: &str,
        user: i64,
        hour: bool,
        day: bool,
        now: i64,
    ) -> rusqlite::Result<()> {
        let tx = self.0.unchecked_transaction()?;
        Self::owned_by(&tx, site, user)?;
        tx.execute("INSERT INTO reminder_settings VALUES (?1,?2,?3,?4,?5,NULL,0,0) ON CONFLICT(site_id) DO UPDATE SET user_id=excluded.user_id,hour=excluded.hour,day=excluded.day,enabled_at=excluded.enabled_at,refresh_error=NULL,delivery_failed=0,requested_at=0",params![site,user,hour,day,now])?;
        tx.commit()
    }
    pub fn reminder_refresh_result(
        &self,
        site: &str,
        user: i64,
        error: Option<&str>,
    ) -> rusqlite::Result<()> {
        self.0.execute("UPDATE reminder_settings SET refresh_error=?3 WHERE site_id=?1 AND user_id=?2 AND EXISTS (SELECT 1 FROM sites WHERE id=?1 AND user_id=?2)",params![site,user,error])?;
        Ok(())
    }
    pub fn reminder_delivery_failed(&self, site: &str, user: i64) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE reminder_settings SET delivery_failed=1 WHERE site_id=?1 AND user_id=?2",
            params![site, user],
        )?;
        Ok(())
    }
    /// Claim before native delivery: at-most-once attempts survive a process crash.
    /// Deliberately skip past windows, first imports, newly changed dates, stale caches and wake gaps.
    pub fn claim_reminders(
        &self,
        last: i64,
        now: i64,
        fresh_after: i64,
    ) -> rusqlite::Result<Vec<DueReminder>> {
        if now < last || now.saturating_sub(last) > 60 {
            return Ok(Vec::new());
        }
        let tx = self.0.unchecked_transaction()?;
        let items = {
            let mut query = tx.prepare("WITH offsets AS (SELECT site_id,user_id,enabled_at,requested_at,3600 AS offset FROM reminder_settings WHERE hour=1 UNION ALL SELECT site_id,user_id,enabled_at,requested_at,86400 AS offset FROM reminder_settings WHERE day=1)
            SELECT s.id,s.user_id,e.event_id,e.time_start,o.offset,e.name,s.name FROM offsets o JOIN sites s ON s.id=o.site_id AND s.user_id=o.user_id JOIN events e ON e.site_id=s.id JOIN reminder_events r ON r.site_id=e.site_id AND r.event_id=e.event_id AND r.due=e.time_start JOIN sync_state a ON a.site_id=s.id AND a.kind='agenda'
            WHERE o.requested_at>=o.enabled_at AND o.requested_at>=?3 AND o.requested_at>=?2-900 AND e.kind='due' AND r.eligible=1 AND e.time_start>?2 AND e.time_start-o.offset>?1 AND e.time_start-o.offset<=?2 AND e.time_start-o.offset>o.enabled_at AND e.time_start-o.offset>r.observed_at AND a.refreshed_at>=?2-900 AND a.refreshed_at>=?3 AND a.refreshed_at<=?2
            AND NOT EXISTS (SELECT 1 FROM reminder_ledger l WHERE l.site_id=s.id AND l.user_id=s.user_id AND l.event_id=e.event_id AND l.due=e.time_start AND l.offset=o.offset) ORDER BY e.time_start,s.position,e.event_id")?;
            let rows = query.query_map(params![last, now, fresh_after], |r| {
                Ok(DueReminder {
                    site_id: r.get(0)?,
                    user_id: r.get(1)?,
                    event_id: r.get(2)?,
                    due: r.get(3)?,
                    offset: r.get(4)?,
                    name: r.get(5)?,
                    site_name: r.get(6)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        for item in &items {
            tx.execute(
                "INSERT INTO reminder_ledger VALUES (?1,?2,?3,?4,?5)",
                params![
                    item.site_id,
                    item.user_id,
                    item.event_id,
                    item.due,
                    item.offset
                ],
            )?;
        }
        tx.execute(
            "DELETE FROM reminder_ledger WHERE due<?1",
            [now.saturating_sub(7 * 86400)],
        )?;
        tx.commit()?;
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const START: i64 = 1_000_000;
    fn site() -> Site {
        Site {
            id: "a".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 7,
            full_name: "Learner".into(),
        }
    }
    fn event(id: i64, trigger: i64) -> Event {
        Event {
            site_id: "a".into(),
            id,
            name: "Deadline".into(),
            time_start: trigger + 3600,
            duration: 0,
            kind: "due".into(),
            course_id: Some(2),
            course_name: Some("Course".into()),
        }
    }
    fn store() -> Store {
        let store = Store::open(":memory:").unwrap();
        store.insert(&site()).unwrap();
        store
    }
    #[test]
    fn off_by_default_offsets_and_initial_import_never_catch_up() {
        let store = store();
        assert!(!store.reminders().unwrap()[0].hour);
        assert!(!store.reminders().unwrap()[0].day);
        let events = [event(1, START + 20), event(2, START - 10)];
        store.save_events("a", 7, &events, START).unwrap();
        assert!(store
            .claim_reminders(START, START + 21, 0)
            .unwrap()
            .is_empty());
        store.set_reminders("a", 7, true, false, START).unwrap();
        // Enabling requests a fresh snapshot before any reminder can fire.
        store.save_events("a", 7, &events, START + 1).unwrap();
        let claimed = store.claim_reminders(START, START + 21, 0).unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].event_id, 1);
        assert_eq!(claimed[0].offset, 3600);
        assert!(store
            .claim_reminders(START, START + 21, 0)
            .unwrap()
            .is_empty());
        // Initial import at/after the trigger never creates a delayed reminder.
        store
            .save_events("a", 7, &[event(3, START + 25)], START + 30)
            .unwrap();
        assert!(store
            .claim_reminders(START + 21, START + 31, 0)
            .unwrap()
            .is_empty());
    }
    #[test]
    fn changes_cancellations_completion_and_stale_data_reconcile() {
        let store = store();
        store.set_reminders("a", 7, true, false, START).unwrap();
        let mut e = event(1, START + 20);
        store
            .save_events("a", 7, std::slice::from_ref(&e), START)
            .unwrap();
        // A normal refresh retains the observed-at baseline for an unchanged event.
        store
            .save_events("a", 7, std::slice::from_ref(&e), START + 21)
            .unwrap();
        assert_eq!(
            store.claim_reminders(START, START + 22, 0).unwrap().len(),
            1
        );
        e.time_start += 100;
        store
            .save_events("a", 7, std::slice::from_ref(&e), START + 30)
            .unwrap();
        // A slower, older request cannot put the previous due date back.
        store
            .save_events_with_reminders(
                "a",
                7,
                &[event(1, START + 20)],
                &[(1, true)],
                START + 40,
                START + 5,
            )
            .unwrap();

        assert_eq!(
            store
                .claim_reminders(START + 110, START + 121, 0)
                .unwrap()
                .len(),
            1
        );
        e.time_start += 100;
        store
            .save_events_with_reminders(
                "a",
                7,
                std::slice::from_ref(&e),
                &[(1, false)],
                START + 130,
                START + 130,
            )
            .unwrap();
        assert!(store
            .claim_reminders(START + 210, START + 221, 0)
            .unwrap()
            .is_empty());
        store.save_events("a", 7, &[], START + 230).unwrap();
        assert!(store
            .claim_reminders(START + 230, START + 240, 0)
            .unwrap()
            .is_empty());
        let e = event(2, START + 2000);
        store.save_events("a", 7, &[e], START + 100).unwrap();
        assert!(store
            .claim_reminders(START + 1990, START + 2001, 0)
            .unwrap()
            .is_empty());
    }
    #[test]
    fn sleep_clock_changes_new_dates_and_disable_skip_missed_windows() {
        let store = store();
        store.set_reminders("a", 7, true, false, START).unwrap();
        store
            .save_events("a", 7, &[event(1, START + 20)], START)
            .unwrap();
        assert!(store
            .claim_reminders(START, START + 120, 0)
            .unwrap()
            .is_empty());
        assert!(store
            .claim_reminders(START + 120, START, 0)
            .unwrap()
            .is_empty());
        // A wake requires data refreshed after waking, even when the old cache is recent.
        assert!(store
            .claim_reminders(START + 10, START + 21, START + 15)
            .unwrap()
            .is_empty());
        // A response arriving after wake is still stale if its request began before wake.
        store
            .save_events_with_reminders(
                "a",
                7,
                &[event(1, START + 20)],
                &[(1, true)],
                START + 16,
                START,
            )
            .unwrap();
        assert!(store
            .claim_reminders(START + 16, START + 21, START + 15)
            .unwrap()
            .is_empty());
        store
            .save_events("a", 7, &[event(1, START + 20)], START + 16)
            .unwrap();
        assert_eq!(
            store
                .claim_reminders(START + 16, START + 21, START + 15)
                .unwrap()
                .len(),
            1
        );
        store
            .save_events("a", 7, &[event(2, START + 25)], START + 30)
            .unwrap();
        assert!(store
            .claim_reminders(START + 21, START + 31, 0)
            .unwrap()
            .is_empty());
        store
            .set_reminders("a", 7, false, false, START + 31)
            .unwrap();
        store
            .save_events("a", 7, &[event(3, START + 50)], START + 32)
            .unwrap();
        assert!(store
            .claim_reminders(START + 40, START + 51, 0)
            .unwrap()
            .is_empty());
        store
            .set_reminders("a", 7, true, false, START + 52)
            .unwrap();
        assert!(store
            .claim_reminders(START + 50, START + 53, 0)
            .unwrap()
            .is_empty());
    }
    #[test]
    fn ledger_preferences_and_account_boundaries_survive_restart() {
        let path =
            std::env::temp_dir().join(format!("moodburo-reminders-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let store = Store::open(&path).unwrap();
            store.insert(&site()).unwrap();
            store.set_reminders("a", 7, true, true, START).unwrap();
            let mut day = event(2, START + 20);
            day.time_start = START + 20 + 86400;
            store
                .save_events("a", 7, &[event(1, START + 20), day], START)
                .unwrap();
            assert_eq!(
                store.claim_reminders(START, START + 21, 0).unwrap().len(),
                2
            );
            store.reminder_delivery_failed("a", 7).unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert!(store.reminders().unwrap()[0].delivery_failed);
        assert!(store
            .claim_reminders(START, START + 21, 0)
            .unwrap()
            .is_empty());
        let mut replaced = site();
        replaced.user_id = 8;
        store.update(&replaced).unwrap();
        assert!(!store.reminders().unwrap()[0].hour);
        assert!(store.set_reminders("a", 7, true, true, START + 25).is_err());
        assert!(store
            .save_events("a", 7, &[event(1, START + 30)], START + 25)
            .is_err());
        store
            .reminder_refresh_result("a", 7, Some("old failure"))
            .unwrap();
        assert!(store.reminders().unwrap()[0].refresh_error.is_none());
        store.remove("a").unwrap();
        assert!(store.reminders().unwrap().is_empty());
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn one_off_reminders_fire_once_never_late_and_never_twice() {
        let store = store();
        let events = [event(1, START + 100), event(2, START - 10)];
        store.save_events("a", 7, &events, START).unwrap();
        // Refused rather than stored when it could never fire, or is not this account's.
        assert_eq!(
            store.add_reminder_pick("a", 7, 2, 3600, START),
            Err(PickError::Passed)
        );
        assert_eq!(
            store.add_reminder_pick("a", 7, 9, 3600, START),
            Err(PickError::Event)
        );
        assert_eq!(
            store.add_reminder_pick("a", 8, 1, 3600, START),
            Err(PickError::Account)
        );
        assert_eq!(
            store.add_reminder_pick("a", 7, 1, 60, START),
            Err(PickError::Event)
        );
        assert!(store.reminder_eligible("a", 1).unwrap());
        assert!(!store.reminder_eligible("a", 9).unwrap());
        let pick = store.add_reminder_pick("a", 7, 1, 3600, START).unwrap();
        assert_eq!(pick.due, START + 100 + 3600);
        assert_eq!(store.reminder_picks().unwrap(), vec![pick]);
        assert_eq!(
            store.reminder_pick_accounts().unwrap(),
            vec![("a".to_owned(), 7)]
        );
        // Not before its moment, and not from a stale agenda.
        assert!(store
            .claim_reminder_picks(START + 50, START + 60, 0)
            .unwrap()
            .is_empty());
        assert!(store
            .claim_reminder_picks(START + 95, START + 101, START + 96)
            .unwrap()
            .is_empty());
        store.save_events("a", 7, &events[..1], START + 97).unwrap();
        let claimed = store
            .claim_reminder_picks(START + 97, START + 101, START + 96)
            .unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!((claimed[0].event_id, claimed[0].offset), (1, 3600));
        assert!(store.reminder_picks().unwrap().is_empty());
        assert!(store
            .claim_reminder_picks(START + 97, START + 101, 0)
            .unwrap()
            .is_empty());
        // The organisation-wide setting claimed it first: the one-off is dropped silently.
        store
            .set_reminders("a", 7, true, false, START + 200)
            .unwrap();
        let later = [event(3, START + 300)];
        store.save_events("a", 7, &later, START + 201).unwrap();
        store
            .add_reminder_pick("a", 7, 3, 3600, START + 202)
            .unwrap();
        store.save_events("a", 7, &later, START + 299).unwrap();
        assert_eq!(
            store
                .claim_reminders(START + 295, START + 301, 0)
                .unwrap()
                .len(),
            1
        );
        assert!(store
            .claim_reminder_picks(START + 295, START + 301, 0)
            .unwrap()
            .is_empty());
        assert!(store.reminder_picks().unwrap().is_empty());
        // Missed while asleep: purged at the next tick instead of firing late.
        let missed = [event(4, START + 500)];
        store.save_events("a", 7, &missed, START + 400).unwrap();
        store
            .add_reminder_pick("a", 7, 4, 3600, START + 401)
            .unwrap();
        assert!(store
            .claim_reminder_picks(START + 600, START + 610, 0)
            .unwrap()
            .is_empty());
        assert!(store.reminder_picks().unwrap().is_empty());
        // Removal, and account replacement, forget them.
        let again = [event(5, START + 900)];
        store.save_events("a", 7, &again, START + 700).unwrap();
        store
            .add_reminder_pick("a", 7, 5, 3600, START + 701)
            .unwrap();
        store.remove_reminder_pick("a", 7, 5, 3600).unwrap();
        assert!(store.reminder_picks().unwrap().is_empty());
        store
            .add_reminder_pick("a", 7, 5, 3600, START + 701)
            .unwrap();
        let mut replaced = site();
        replaced.user_id = 8;
        store.update(&replaced).unwrap();
        assert!(store.reminder_picks().unwrap().is_empty());
    }
    #[test]
    fn elapsed_offsets_do_not_shift_at_dst_boundary_and_future_cache_is_refused() {
        let store = store();
        let dst = 1792888200;
        store.set_reminders("a", 7, true, false, dst - 60).unwrap();
        let e = event(1, dst);
        store
            .save_events("a", 7, std::slice::from_ref(&e), dst - 30)
            .unwrap();
        assert_eq!(
            store.claim_reminders(dst - 5, dst + 5, 0).unwrap()[0].due,
            dst + 3600
        );
        let e = event(2, dst + 30);
        store.save_events("a", 7, &[e], dst + 50).unwrap();
        assert!(store
            .claim_reminders(dst + 20, dst + 40, 0)
            .unwrap()
            .is_empty());
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn schema_eight_upgrades_with_reminders_off_and_retains_existing_metadata() {
        let path = std::env::temp_dir().join(format!(
            "moodburo-reminder-migration-{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let store = Store::open(&path).unwrap();
            store
                .insert(&Site {
                    id: "a".into(),
                    wwwroot: "https://example.org/".into(),
                    name: "Example".into(),
                    user_id: 7,
                    full_name: "Learner".into(),
                })
                .unwrap();
            store.0.execute_batch("DROP TABLE reminder_settings; DROP TABLE reminder_events; DROP TABLE reminder_ledger; DROP TABLE reminder_picks; PRAGMA user_version=8;").unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(store.sites().unwrap()[0].user_id, 7);
        let prefs = store.reminders().unwrap();
        assert!(!prefs[0].hour);
        assert!(!prefs[0].day);
        assert!(store.claim_reminders(100, 120, 0).unwrap().is_empty());
        assert!(store.reminder_picks().unwrap().is_empty());
        assert!(store.claim_reminder_picks(100, 120, 0).unwrap().is_empty());
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
}
