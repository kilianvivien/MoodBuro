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
impl Store {
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
            store.0.execute_batch("DROP TABLE reminder_settings; DROP TABLE reminder_events; DROP TABLE reminder_ledger; PRAGMA user_version=8;").unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(store.sites().unwrap()[0].user_id, 7);
        let prefs = store.reminders().unwrap();
        assert!(!prefs[0].hour);
        assert!(!prefs[0].day);
        assert!(store.claim_reminders(100, 120, 0).unwrap().is_empty());
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
}
