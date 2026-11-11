//! Durable resource-level checkpoints; interrupted files restart from a fresh descriptor.
use super::*;
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSync {
    pub site_id: String,
    pub user_id: i64,
    pub course_id: i64,
    pub generation: i64,
    pub interval: i64,
    pub state: String,
    pub next_run: i64,
    pub refreshed_at: i64,
    pub failures: i64,
    pub error: Option<String>,
    pub keys: Vec<String>,
    pub pending: usize,
    pub missing: usize,
}
impl Store {
    pub fn course_syncs(&self) -> rusqlite::Result<Vec<CourseSync>> {
        let mut stmt = self.0.prepare("SELECT c.site_id,s.user_id,c.course_id,c.generation,c.interval,c.state,c.next_run,c.refreshed_at,c.failures,c.error FROM course_sync c JOIN sites s ON s.id=c.site_id ORDER BY c.next_run,c.site_id,c.course_id")?;
        let rows = stmt.query_map([], |r| {
            Ok(CourseSync {
                site_id: r.get(0)?,
                user_id: r.get(1)?,
                course_id: r.get(2)?,
                generation: r.get(3)?,
                interval: r.get(4)?,
                state: r.get(5)?,
                next_run: r.get(6)?,
                refreshed_at: r.get(7)?,
                failures: r.get(8)?,
                error: r.get(9)?,
                keys: vec![],
                pending: 0,
                missing: 0,
            })
        })?;
        let mut jobs: Vec<CourseSync> = rows.collect::<rusqlite::Result<_>>()?;
        for job in &mut jobs {
            let mut files = self.0.prepare("SELECT resource_key,pending,missing FROM sync_selection WHERE site_id=?1 AND course_id=?2 ORDER BY resource_key")?;
            for row in files.query_map(params![job.site_id, job.course_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, bool>(1)?,
                    r.get::<_, bool>(2)?,
                ))
            })? {
                let (key, pending, missing) = row?;
                job.keys.push(key);
                job.pending += usize::from(pending);
                job.missing += usize::from(missing);
            }
        }
        Ok(jobs)
    }
    pub fn configure_course_sync(
        &self,
        site: &str,
        user: i64,
        course: i64,
        keys: &[String],
        interval: i64,
        now: i64,
    ) -> rusqlite::Result<()> {
        if keys.is_empty()
            || keys.len() > 50
            || !matches!(interval, 0 | 3600 | 86400)
            || keys
                .iter()
                .any(|key| key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let tx = self.0.unchecked_transaction()?;
        let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sites s JOIN courses c ON c.site_id=s.id WHERE s.id=?1 AND s.user_id=?2 AND c.course_id=?3)",params![site,user,course],|r|r.get(0))?;
        if !exists {
            return Err(rusqlite::Error::InvalidQuery);
        }
        tx.execute("INSERT INTO course_sync(site_id,course_id,generation,interval,state,next_run,refreshed_at,failures,error) VALUES(?1,?2,1,?3,?4,?5,0,0,NULL) ON CONFLICT(site_id,course_id) DO UPDATE SET generation=generation+1,interval=excluded.interval,state=excluded.state,next_run=excluded.next_run,failures=0,error=NULL,refreshed_at=0", params![site,course,interval,if interval==0 {"paused"} else {"waiting"},now])?;
        tx.execute(
            "DELETE FROM sync_selection WHERE site_id=?1 AND course_id=?2",
            params![site, course],
        )?;
        for key in keys {
            tx.execute(
                "INSERT OR IGNORE INTO sync_selection VALUES(?1,?2,?3,1,0)",
                params![site, course, key],
            )?;
        }
        tx.commit()
    }
    pub fn sync_current(&self, job: &CourseSync) -> rusqlite::Result<bool> {
        self.0.query_row("SELECT EXISTS(SELECT 1 FROM course_sync c JOIN sites s ON s.id=c.site_id WHERE c.site_id=?1 AND c.course_id=?2 AND s.user_id=?3 AND c.generation=?4 AND c.state='running' AND c.interval>0)",params![job.site_id,job.course_id,job.user_id,job.generation],|r|r.get(0))
    }
    pub fn recover_course_syncs(&self, now: i64) -> rusqlite::Result<()> {
        self.0.execute("UPDATE course_sync SET state='interrupted',next_run=?1,generation=generation+1 WHERE state='running'",[now])?;
        Ok(())
    }
    pub fn claim_course_sync(&self, now: i64) -> rusqlite::Result<Option<CourseSync>> {
        // A backwards clock jump must not postpone an opted-in schedule for days.
        self.0.execute("UPDATE course_sync SET next_run=?1+interval WHERE interval>0 AND state<>'running' AND next_run>?1+interval",[now])?;
        let Some(mut job) = self
            .course_syncs()?
            .into_iter()
            .find(|j| j.interval > 0 && j.state != "running" && j.next_run <= now)
        else {
            return Ok(None);
        };
        let tx = self.0.unchecked_transaction()?;
        if job.state == "idle" || (job.state == "cancelled" && job.pending == 0) {
            tx.execute(
                "UPDATE sync_selection SET pending=1,missing=0 WHERE site_id=?1 AND course_id=?2",
                params![job.site_id, job.course_id],
            )?;
        }
        tx.execute("UPDATE course_sync SET state='running',generation=generation+1,next_run=?3 WHERE site_id=?1 AND course_id=?2",params![job.site_id,job.course_id,now.saturating_add(job.interval)])?;
        tx.commit()?;
        job.generation += 1;
        job.state = "running".into();
        Ok(Some(job))
    }
    pub fn sync_pending(&self, job: &CourseSync) -> rusqlite::Result<Vec<String>> {
        if !self.sync_current(job)? {
            return Ok(vec![]);
        }
        let mut stmt=self.0.prepare("SELECT resource_key FROM sync_selection WHERE site_id=?1 AND course_id=?2 AND pending=1 ORDER BY resource_key")?;
        let rows = stmt
            .query_map(params![job.site_id, job.course_id], |r| r.get(0))?
            .collect();
        rows
    }
    pub fn sync_checkpoint(
        &self,
        job: &CourseSync,
        key: &str,
        error: Option<&str>,
        missing: bool,
    ) -> rusqlite::Result<()> {
        if !self.sync_current(job)? {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.0.execute("UPDATE sync_selection SET pending=?4,missing=?5 WHERE site_id=?1 AND course_id=?2 AND resource_key=?3",params![job.site_id,job.course_id,key,error.is_some(),missing])?;
        if let Some(error) = error {
            self.0.execute(
                "UPDATE course_sync SET error=?3 WHERE site_id=?1 AND course_id=?2",
                params![job.site_id, job.course_id, error],
            )?;
        }
        Ok(())
    }
    pub fn finish_course_sync(
        &self,
        job: &CourseSync,
        now: i64,
        error: Option<&str>,
    ) -> rusqlite::Result<()> {
        if !self.sync_current(job)? {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let pending = self.sync_pending(job)?.len();
        let failed = error.is_some() || pending > 0;
        let failures = if failed {
            (job.failures + 1).min(12)
        } else {
            0
        };
        let delay = if failed {
            (60_i64.saturating_mul(1_i64 << failures.min(10))).min(job.interval)
        } else {
            job.interval
        };
        self.0.execute("UPDATE course_sync SET state=?3,next_run=?4,refreshed_at=CASE WHEN ?5 THEN refreshed_at ELSE ?6 END,failures=?7,error=CASE WHEN ?5 THEN COALESCE(?8,error) ELSE NULL END WHERE site_id=?1 AND course_id=?2",params![job.site_id,job.course_id,if failed {"error"} else {"idle"},now.saturating_add(delay),failed,now,failures,error])?;
        Ok(())
    }
    /// Cancellation stops this attempt; the next scheduled cycle is still opt-in.
    pub fn control_course_sync(
        &self,
        site: &str,
        user: i64,
        course: i64,
        cancel: bool,
        now: i64,
    ) -> rusqlite::Result<()> {
        if !cancel {
            self.0.execute("UPDATE sync_selection SET pending=1,missing=0 WHERE site_id=?1 AND course_id=?2 AND EXISTS(SELECT 1 FROM course_sync c JOIN sites s ON s.id=c.site_id WHERE c.site_id=?1 AND c.course_id=?2 AND s.user_id=?3 AND (c.state='idle' OR (c.state='cancelled' AND NOT EXISTS(SELECT 1 FROM sync_selection p WHERE p.site_id=c.site_id AND p.course_id=c.course_id AND p.pending=1))))",params![site,course,user])?;
        }
        let count=self.0.execute("UPDATE course_sync SET generation=generation+1,state=CASE WHEN ?4 THEN 'cancelled' ELSE 'waiting' END,next_run=?5+CASE WHEN ?4 THEN interval ELSE 0 END WHERE site_id=?1 AND course_id=?2 AND interval>0 AND EXISTS(SELECT 1 FROM sites WHERE id=?1 AND user_id=?3)",params![site,course,user,cancel,now])?;
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
    fn setup(path: &str) -> Store {
        let store = Store::open(path).unwrap();
        if store.sites().unwrap().is_empty() {
            store
                .insert(&Site {
                    id: "s".into(),
                    wwwroot: "https://example.org/".into(),
                    name: "Example Campus".into(),
                    user_id: 7,
                    full_name: "Learner".into(),
                })
                .unwrap();
            store
                .save_courses(
                    "s",
                    7,
                    &[Course {
                        site_id: "s".into(),
                        id: 2,
                        name: "Course".into(),
                        short_name: "C".into(),
                        pinned: false,
                    }],
                    10,
                )
                .unwrap();
        }
        store
    }
    fn keys() -> Vec<String> {
        vec!["a".repeat(64), "b".repeat(64)]
    }
    #[test]
    fn migration_preserves_existing_metadata_and_schedules_do_not_catch_up_repeatedly() {
        let path =
            std::env::temp_dir().join(format!("moodburo-sync-migration-{}.db", std::process::id()));
        let store = setup(path.to_str().unwrap());
        store.0.execute_batch("DROP TABLE sync_selection; DROP TABLE course_sync; DROP TABLE download_hashes; PRAGMA user_version=13").unwrap();
        drop(store);
        let store = setup(path.to_str().unwrap());
        assert_eq!(store.courses().unwrap()[0].name, "Course");
        assert!(store.course_syncs().unwrap().is_empty());
        store
            .configure_course_sync("s", 7, 2, &keys(), 3600, 100000)
            .unwrap();
        assert!(store.claim_course_sync(10).unwrap().is_none());
        assert_eq!(store.course_syncs().unwrap()[0].next_run, 3610);
        let job = store.claim_course_sync(999999).unwrap().unwrap();
        for key in keys() {
            store.sync_checkpoint(&job, &key, None, false).unwrap();
        }
        store.finish_course_sync(&job, 999999, None).unwrap();
        assert!(store.claim_course_sync(999999).unwrap().is_none());
        assert_eq!(store.course_syncs().unwrap()[0].next_run, 1003599);
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn consent_validation_pause_and_account_isolation() {
        let store = setup(":memory:");
        assert!(store.claim_course_sync(10).unwrap().is_none());
        for (user, course, interval, keys) in [
            (8, 2, 3600, keys()),
            (7, 3, 3600, keys()),
            (7, 2, 5, keys()),
            (7, 2, 3600, vec![]),
            (7, 2, 3600, vec!["https://example.org/file?token=x".into()]),
        ] {
            assert!(store
                .configure_course_sync("s", user, course, &keys, interval, 10)
                .is_err());
        }
        store
            .configure_course_sync("s", 7, 2, &keys(), 0, 10)
            .unwrap();
        assert!(store.claim_course_sync(100).unwrap().is_none());
        store
            .configure_course_sync("s", 7, 2, &keys(), 3600, 100)
            .unwrap();
        let job = store.claim_course_sync(100).unwrap().unwrap();
        assert!(store.sync_current(&job).unwrap());
        store
            .configure_course_sync("s", 7, 2, &["c".repeat(64)], 0, 101)
            .unwrap();
        assert!(!store.sync_current(&job).unwrap());
        assert!(store
            .sync_checkpoint(&job, &keys()[0], None, false)
            .is_err());
        store.save_courses("s", 7, &[], 102).unwrap();
        assert_eq!(store.course_syncs().unwrap()[0].interval, 0);
        let mut site = store.sites().unwrap().remove(0);
        site.user_id = 8;
        store.update(&site).unwrap();
        assert!(store.course_syncs().unwrap().is_empty());
        assert!(store
            .configure_course_sync("s", 7, 2, &keys(), 3600, 103)
            .is_err());
    }
    #[test]
    fn checkpoints_resume_after_restart_and_failures_without_replaying_completed_files() {
        let path = std::env::temp_dir().join(format!("moodburo-sync-{}.db", std::process::id()));
        let store = setup(path.to_str().unwrap());
        store
            .configure_course_sync("s", 7, 2, &keys(), 3600, 100)
            .unwrap();
        let original = store.claim_course_sync(100).unwrap().unwrap();
        store
            .sync_checkpoint(&original, &keys()[0], None, false)
            .unwrap();
        drop(store);
        let store = setup(path.to_str().unwrap());
        store.recover_course_syncs(200).unwrap();
        assert!(!store.sync_current(&original).unwrap());
        let resumed = store.claim_course_sync(200).unwrap().unwrap();
        assert_eq!(
            store.sync_pending(&resumed).unwrap(),
            vec![keys()[1].clone()]
        );
        store
            .sync_checkpoint(
                &resumed,
                &keys()[1],
                Some("Could not download this resource."),
                false,
            )
            .unwrap();
        store.finish_course_sync(&resumed, 201, None).unwrap();
        let failed = store.course_syncs().unwrap().remove(0);
        assert_eq!(failed.state, "error");
        assert_eq!(failed.refreshed_at, 0);
        assert!(failed.next_run > 201 && failed.next_run <= 3801);
        assert!(store
            .claim_course_sync(failed.next_run - 1)
            .unwrap()
            .is_none());
        let retry = store.claim_course_sync(failed.next_run).unwrap().unwrap();
        assert_eq!(store.sync_pending(&retry).unwrap().len(), 1);
        store
            .sync_checkpoint(&retry, &keys()[1], None, true)
            .unwrap();
        store
            .finish_course_sync(&retry, failed.next_run, None)
            .unwrap();
        let done = store.course_syncs().unwrap().remove(0);
        assert_eq!(done.state, "idle");
        assert_eq!(done.missing, 1);
        assert_eq!(done.failures, 0);
        assert!(done.error.is_none());
        store.control_course_sync("s", 7, 2, false, 500).unwrap();
        let next = store.claim_course_sync(500).unwrap().unwrap();
        assert_eq!(store.sync_pending(&next).unwrap().len(), 2);
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn cancellation_invalidates_late_publication_and_keeps_checkpoints_for_resume() {
        let store = setup(":memory:");
        store
            .configure_course_sync("s", 7, 2, &keys(), 3600, 10)
            .unwrap();
        let job = store.claim_course_sync(10).unwrap().unwrap();
        store
            .sync_checkpoint(&job, &keys()[0], None, false)
            .unwrap();
        assert!(store.control_course_sync("s", 8, 2, true, 11).is_err());
        assert!(store.sync_current(&job).unwrap());
        store.control_course_sync("s", 7, 2, true, 11).unwrap();
        assert!(!store.sync_current(&job).unwrap());
        assert!(store.finish_course_sync(&job, 12, None).is_err());
        assert!(store.claim_course_sync(12).unwrap().is_none());
        store.control_course_sync("s", 7, 2, false, 12).unwrap();
        let next = store.claim_course_sync(12).unwrap().unwrap();
        assert_eq!(store.sync_pending(&next).unwrap(), vec![keys()[1].clone()]);
        store.save_courses("s", 7, &[], 13).unwrap();
        assert!(!store.sync_current(&next).unwrap());
        store.remove("s").unwrap();
        assert!(store.course_syncs().unwrap().is_empty());
    }
}
