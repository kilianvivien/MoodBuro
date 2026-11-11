//! Saved timetable weeks (schema 19). Derived content only: no PDF bytes, no
//! URL, no path. A week is replaced as a whole in one transaction; sessions
//! that leave it stay as cancellation records until the week is removed.
use super::*;

/// What a saved week holds besides its sessions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimetableWeek {
    pub site_id: String,
    pub week_key: String,
    pub user_id: i64,
    /// Opaque source reference (`course:…`, `copy:…`, `download:…`).
    pub source_ref: String,
    pub filename: String,
    pub source_sha256: String,
    pub week_start: String,
    pub week_end: String,
    pub promotion: Option<String>,
    pub cohort: String,
    /// The engine's original reusable `parsed` payload.
    pub parsed_json: String,
    pub choices_json: String,
    pub options_json: String,
    /// The engine's selected week.
    pub selected_json: String,
    pub parser_version: String,
    pub rules_version: String,
    pub adapter_contract: i64,
    pub revision: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// One session as written by a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionWrite {
    pub uid: String,
    pub date: String,
    pub last_date: String,
    pub start_at: Option<i64>,
    pub end_at: Option<i64>,
    pub all_day: bool,
    pub tentative: bool,
    /// Canonical timing (dates and wall times), to detect moves.
    pub timing: String,
    /// Display payload (JSON), to detect detail changes.
    pub payload: String,
}

/// A stored session, including cancellation records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSession {
    pub uid: String,
    pub date: String,
    pub last_date: String,
    pub start_at: Option<i64>,
    pub end_at: Option<i64>,
    pub all_day: bool,
    pub tentative: bool,
    pub cancelled: bool,
    #[serde(skip)]
    pub timing: String,
    pub payload: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekDiff {
    pub added: Vec<String>,
    pub moved: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
    pub unchanged: usize,
}

impl WeekDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.moved.is_empty()
            && self.changed.is_empty()
            && self.removed.is_empty()
    }
}

/// Compare the active sessions of a saved week with a new selection, by UID.
pub fn week_diff(old: &[StoredSession], new: &[SessionWrite]) -> WeekDiff {
    let mut diff = WeekDiff::default();
    for session in new {
        match old
            .iter()
            .find(|known| !known.cancelled && known.uid == session.uid)
        {
            None => diff.added.push(session.uid.clone()),
            Some(known) if known.timing != session.timing => diff.moved.push(session.uid.clone()),
            Some(known)
                if known.payload != session.payload || known.tentative != session.tentative =>
            {
                diff.changed.push(session.uid.clone())
            }
            Some(_) => diff.unchanged += 1,
        }
    }
    for known in old.iter().filter(|known| !known.cancelled) {
        if !new.iter().any(|session| session.uid == known.uid) {
            diff.removed.push(known.uid.clone());
        }
    }
    diff
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekSummary {
    pub week_key: String,
    pub week_start: String,
    pub week_end: String,
    pub promotion: Option<String>,
    pub cohort: String,
    pub filename: String,
    pub revision: i64,
    pub updated_at: i64,
    pub sessions: i64,
    pub tentative: i64,
}

#[derive(Debug)]
pub enum CommitError {
    /// The saved week changed since the preview (or was created meanwhile).
    Conflict,
    /// The site now belongs to another account, or is gone.
    AccountChanged,
    /// A UID already belongs to another saved week of this site.
    UidCollision,
    Storage(rusqlite::Error),
}

impl From<rusqlite::Error> for CommitError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error)
    }
}

/// Result of a commit: the new revision and what changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Committed {
    pub revision: i64,
    pub diff: WeekDiff,
}

const WEEK_COLUMNS: &str = "site_id,week_key,user_id,source_ref,filename,source_sha256,week_start,week_end,promotion,cohort,parsed_json,choices_json,options_json,selected_json,parser_version,rules_version,adapter_contract,revision,created_at,updated_at";

fn week_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<TimetableWeek> {
    Ok(TimetableWeek {
        site_id: r.get(0)?,
        week_key: r.get(1)?,
        user_id: r.get(2)?,
        source_ref: r.get(3)?,
        filename: r.get(4)?,
        source_sha256: r.get(5)?,
        week_start: r.get(6)?,
        week_end: r.get(7)?,
        promotion: r.get(8)?,
        cohort: r.get(9)?,
        parsed_json: r.get(10)?,
        choices_json: r.get(11)?,
        options_json: r.get(12)?,
        selected_json: r.get(13)?,
        parser_version: r.get(14)?,
        rules_version: r.get(15)?,
        adapter_contract: r.get(16)?,
        revision: r.get(17)?,
        created_at: r.get(18)?,
        updated_at: r.get(19)?,
    })
}

fn session_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<StoredSession> {
    Ok(StoredSession {
        uid: r.get(0)?,
        date: r.get(1)?,
        last_date: r.get(2)?,
        start_at: r.get(3)?,
        end_at: r.get(4)?,
        all_day: r.get(5)?,
        tentative: r.get(6)?,
        cancelled: r.get(7)?,
        timing: r.get(8)?,
        payload: r.get(9)?,
    })
}

const SESSION_COLUMNS: &str =
    "uid,date,last_date,start_at,end_at,all_day,tentative,cancelled,timing,payload";

pub(crate) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS timetable_weeks (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, week_key TEXT NOT NULL, user_id INTEGER NOT NULL, source_ref TEXT NOT NULL, filename TEXT NOT NULL, source_sha256 TEXT NOT NULL, week_start TEXT NOT NULL, week_end TEXT NOT NULL, promotion TEXT, cohort TEXT NOT NULL, parsed_json TEXT NOT NULL, choices_json TEXT NOT NULL, options_json TEXT NOT NULL, selected_json TEXT NOT NULL, parser_version TEXT NOT NULL, rules_version TEXT NOT NULL, adapter_contract INTEGER NOT NULL, revision INTEGER NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, last_request TEXT, PRIMARY KEY(site_id,week_key));
    CREATE TABLE IF NOT EXISTS timetable_events (site_id TEXT NOT NULL, week_key TEXT NOT NULL, uid TEXT NOT NULL, date TEXT NOT NULL, last_date TEXT NOT NULL, start_at INTEGER, end_at INTEGER, all_day INTEGER NOT NULL, tentative INTEGER NOT NULL, cancelled INTEGER NOT NULL, timing TEXT NOT NULL, payload TEXT NOT NULL, revision INTEGER NOT NULL, PRIMARY KEY(site_id,week_key,uid), FOREIGN KEY(site_id,week_key) REFERENCES timetable_weeks(site_id,week_key) ON DELETE CASCADE);
    CREATE INDEX IF NOT EXISTS timetable_events_by_date ON timetable_events(site_id,date);
    CREATE INDEX IF NOT EXISTS timetable_events_by_uid ON timetable_events(site_id,uid);
    CREATE TABLE IF NOT EXISTS timetable_outputs (site_id TEXT NOT NULL, week_key TEXT NOT NULL, channel TEXT NOT NULL, cohort TEXT NOT NULL, baseline_json TEXT NOT NULL, revision INTEGER NOT NULL, options_json TEXT NOT NULL, sha256 TEXT NOT NULL, published_at INTEGER NOT NULL, PRIMARY KEY(site_id,week_key,channel), FOREIGN KEY(site_id,week_key) REFERENCES timetable_weeks(site_id,week_key) ON DELETE CASCADE);
    CREATE TABLE IF NOT EXISTS timetable_reminders (site_id TEXT NOT NULL, week_key TEXT NOT NULL, uid TEXT NOT NULL, user_id INTEGER NOT NULL, offset INTEGER NOT NULL, created_at INTEGER NOT NULL, PRIMARY KEY(site_id,week_key,uid,offset), FOREIGN KEY(site_id,week_key) REFERENCES timetable_weeks(site_id,week_key) ON DELETE CASCADE);
    CREATE TABLE IF NOT EXISTS timetable_reminder_ledger (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, user_id INTEGER NOT NULL, uid TEXT NOT NULL, start_at INTEGER NOT NULL, offset INTEGER NOT NULL, PRIMARY KEY(site_id,user_id,uid,start_at,offset));
    CREATE TABLE IF NOT EXISTS timetable_handoffs (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, week_key TEXT NOT NULL, user_id INTEGER NOT NULL, revision INTEGER NOT NULL, sha256 TEXT NOT NULL, opened_at INTEGER NOT NULL, PRIMARY KEY(site_id,week_key));
    CREATE TABLE IF NOT EXISTS timetable_preferences (site_id TEXT PRIMARY KEY REFERENCES sites(id) ON DELETE CASCADE, user_id INTEGER NOT NULL, json TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS timetable_calendars (site_id TEXT PRIMARY KEY REFERENCES sites(id) ON DELETE CASCADE, user_id INTEGER NOT NULL, calendar_id TEXT, created_at INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS timetable_calendar_events (site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE, uid TEXT NOT NULL, event_id TEXT NOT NULL, PRIMARY KEY(site_id,uid));
";

/// Tables wiped when a site's account is replaced (see `Store::update`).
pub(crate) const ACCOUNT_TABLES: [&str; 8] = [
    "timetable_calendar_events",
    "timetable_calendars",
    "timetable_reminder_ledger",
    "timetable_handoffs",
    "timetable_preferences",
    "timetable_reminders",
    "timetable_outputs",
    "timetable_weeks",
];

fn owned(tx: &rusqlite::Transaction<'_>, site: &str, user: i64) -> rusqlite::Result<bool> {
    tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM sites WHERE id=?1 AND user_id=?2)",
        params![site, user],
        |r| r.get(0),
    )
}

impl Store {
    pub fn timetable_weeks(&self, site: &str, user: i64) -> rusqlite::Result<Vec<WeekSummary>> {
        let mut stmt = self.0.prepare(
            "SELECT w.week_key,w.week_start,w.week_end,w.promotion,w.cohort,w.filename,w.revision,w.updated_at,
                (SELECT COUNT(*) FROM timetable_events e WHERE e.site_id=w.site_id AND e.week_key=w.week_key AND e.cancelled=0),
                (SELECT COUNT(*) FROM timetable_events e WHERE e.site_id=w.site_id AND e.week_key=w.week_key AND e.cancelled=0 AND e.tentative=1)
             FROM timetable_weeks w JOIN sites s ON s.id=w.site_id AND s.user_id=w.user_id
             WHERE w.site_id=?1 AND w.user_id=?2 ORDER BY w.week_start, w.week_key",
        )?;
        let rows = stmt.query_map(params![site, user], |r| {
            Ok(WeekSummary {
                week_key: r.get(0)?,
                week_start: r.get(1)?,
                week_end: r.get(2)?,
                promotion: r.get(3)?,
                cohort: r.get(4)?,
                filename: r.get(5)?,
                revision: r.get(6)?,
                updated_at: r.get(7)?,
                sessions: r.get(8)?,
                tentative: r.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn timetable_week(
        &self,
        site: &str,
        user: i64,
        key: &str,
    ) -> rusqlite::Result<Option<(TimetableWeek, Vec<StoredSession>)>> {
        use rusqlite::OptionalExtension;
        let week = self
            .0
            .query_row(
                &format!("SELECT {WEEK_COLUMNS} FROM timetable_weeks w WHERE w.site_id=?1 AND w.user_id=?2 AND w.week_key=?3 AND EXISTS (SELECT 1 FROM sites s WHERE s.id=w.site_id AND s.user_id=w.user_id)"),
                params![site, user, key],
                week_row,
            )
            .optional()?;
        let Some(week) = week else {
            return Ok(None);
        };
        let mut stmt = self.0.prepare(&format!(
            "SELECT {SESSION_COLUMNS} FROM timetable_events WHERE site_id=?1 AND week_key=?2 ORDER BY date, start_at, uid"
        ))?;
        let sessions = stmt
            .query_map(params![site, key], session_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(Some((week, sessions)))
    }

    /// Active sessions of every saved week of the account, overlapping
    /// `[from, to]` (ISO dates, inclusive).
    pub fn timetable_sessions_between(
        &self,
        site: &str,
        user: i64,
        from: &str,
        to: &str,
    ) -> rusqlite::Result<Vec<(String, StoredSession)>> {
        let mut stmt = self.0.prepare(&format!(
            "SELECT e.week_key,{} FROM timetable_events e JOIN timetable_weeks w ON w.site_id=e.site_id AND w.week_key=e.week_key JOIN sites s ON s.id=w.site_id AND s.user_id=w.user_id
             WHERE e.site_id=?1 AND w.user_id=?2 AND e.cancelled=0 AND e.date<=?4 AND e.last_date>=?3 ORDER BY e.date, e.start_at, e.uid",
            SESSION_COLUMNS.split(',').map(|c| format!("e.{c}")).collect::<Vec<_>>().join(",")
        ))?;
        let rows = stmt.query_map(params![site, user, from, to], |r| {
            let key: String = r.get(0)?;
            let session = StoredSession {
                uid: r.get(1)?,
                date: r.get(2)?,
                last_date: r.get(3)?,
                start_at: r.get(4)?,
                end_at: r.get(5)?,
                all_day: r.get(6)?,
                tentative: r.get(7)?,
                cancelled: r.get(8)?,
                timing: r.get(9)?,
                payload: r.get(10)?,
            };
            Ok((key, session))
        })?;
        rows.collect()
    }

    /// Add or replace a whole week in one transaction. `expected` is the
    /// revision the preview was based on (`None` for a new week). Retrying
    /// the same `request` after success is a no-op returning the same revision.
    pub fn commit_timetable_week(
        &self,
        week: &TimetableWeek,
        sessions: &[SessionWrite],
        expected: Option<i64>,
        request: &str,
        now: i64,
    ) -> Result<Committed, CommitError> {
        use rusqlite::OptionalExtension;
        let tx = self.0.unchecked_transaction()?;
        if !owned(&tx, &week.site_id, week.user_id)? {
            return Err(CommitError::AccountChanged);
        }
        let current: Option<(i64, i64, Option<String>)> = tx
            .query_row(
                "SELECT revision,user_id,last_request FROM timetable_weeks WHERE site_id=?1 AND week_key=?2",
                params![week.site_id, week.week_key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if let Some((revision, owner, last)) = &current {
            if *owner != week.user_id {
                return Err(CommitError::AccountChanged);
            }
            if last.as_deref() == Some(request) && expected.is_none_or(|e| e + 1 == *revision) {
                return Ok(Committed {
                    revision: *revision,
                    diff: WeekDiff::default(),
                });
            }
        }
        if current.as_ref().map(|(revision, ..)| *revision) != expected {
            return Err(CommitError::Conflict);
        }
        for session in sessions {
            let taken: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM timetable_events WHERE site_id=?1 AND uid=?2 AND week_key<>?3 AND cancelled=0)",
                params![week.site_id, session.uid, week.week_key],
                |r| r.get(0),
            )?;
            if taken {
                return Err(CommitError::UidCollision);
            }
        }
        let old: Vec<StoredSession> = {
            let mut stmt = tx.prepare(&format!(
                "SELECT {SESSION_COLUMNS} FROM timetable_events WHERE site_id=?1 AND week_key=?2"
            ))?;
            let rows = stmt.query_map(params![week.site_id, week.week_key], session_row)?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let diff = week_diff(&old, sessions);
        let revision = expected.map_or(1, |e| e + 1);
        let created = old.is_empty().then_some(now);
        tx.execute(
            &format!("INSERT INTO timetable_weeks ({WEEK_COLUMNS},last_request) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)
             ON CONFLICT(site_id,week_key) DO UPDATE SET source_ref=excluded.source_ref,filename=excluded.filename,source_sha256=excluded.source_sha256,week_start=excluded.week_start,week_end=excluded.week_end,promotion=excluded.promotion,cohort=excluded.cohort,parsed_json=excluded.parsed_json,choices_json=excluded.choices_json,options_json=excluded.options_json,selected_json=excluded.selected_json,parser_version=excluded.parser_version,rules_version=excluded.rules_version,adapter_contract=excluded.adapter_contract,revision=excluded.revision,updated_at=excluded.updated_at,last_request=excluded.last_request"),
            params![
                week.site_id, week.week_key, week.user_id, week.source_ref, week.filename, week.source_sha256,
                week.week_start, week.week_end, week.promotion, week.cohort, week.parsed_json, week.choices_json,
                week.options_json, week.selected_json, week.parser_version, week.rules_version, week.adapter_contract,
                revision, created.unwrap_or(week.created_at.max(now)), now, request
            ],
        )?;
        // Every session not in the new selection becomes (or stays) a cancellation record.
        tx.execute(
            "UPDATE timetable_events SET cancelled=1, revision=?3 WHERE site_id=?1 AND week_key=?2 AND cancelled=0",
            params![week.site_id, week.week_key, revision],
        )?;
        for session in sessions {
            tx.execute(
                "INSERT INTO timetable_events VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0,?10,?11,?12)
                 ON CONFLICT(site_id,week_key,uid) DO UPDATE SET date=excluded.date,last_date=excluded.last_date,start_at=excluded.start_at,end_at=excluded.end_at,all_day=excluded.all_day,tentative=excluded.tentative,cancelled=0,timing=excluded.timing,payload=excluded.payload,revision=excluded.revision",
                params![
                    week.site_id, week.week_key, session.uid, session.date, session.last_date, session.start_at,
                    session.end_at, session.all_day, session.tentative, session.timing, session.payload, revision
                ],
            )?;
        }
        // A cancelled session cannot remind; a moved one keeps its toggle and is
        // rescheduled from its new start (the ledger is keyed by start).
        tx.execute(
            "DELETE FROM timetable_reminders WHERE site_id=?1 AND week_key=?2 AND uid IN (SELECT uid FROM timetable_events WHERE site_id=?1 AND week_key=?2 AND cancelled=1)",
            params![week.site_id, week.week_key],
        )?;
        tx.commit()?;
        Ok(Committed { revision, diff })
    }

    pub fn remove_timetable_week(
        &self,
        site: &str,
        user: i64,
        key: &str,
        expected: i64,
    ) -> Result<(), CommitError> {
        let tx = self.0.unchecked_transaction()?;
        if !owned(&tx, site, user)? {
            return Err(CommitError::AccountChanged);
        }
        let count = tx.execute(
            "DELETE FROM timetable_weeks WHERE site_id=?1 AND user_id=?2 AND week_key=?3 AND revision=?4",
            params![site, user, key, expected],
        )?;
        if count != 1 {
            return Err(CommitError::Conflict);
        }
        tx.commit()?;
        Ok(())
    }

    /// Earlier exported selections of a week on one channel (a JSON array).
    pub fn timetable_output_baseline(
        &self,
        site: &str,
        user: i64,
        key: &str,
        channel: &str,
    ) -> rusqlite::Result<Option<String>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row(
                "SELECT o.baseline_json FROM timetable_outputs o JOIN timetable_weeks w ON w.site_id=o.site_id AND w.week_key=o.week_key JOIN sites s ON s.id=w.site_id AND s.user_id=w.user_id WHERE o.site_id=?1 AND w.user_id=?2 AND o.week_key=?3 AND o.channel=?4",
                params![site, user, key, channel],
                |r| r.get(0),
            )
            .optional()
    }

    /// Record a published export, only once its file exists. Refused when
    /// the week changed or the account is not the owner any more.
    #[allow(clippy::too_many_arguments)]
    pub fn record_timetable_output(
        &self,
        site: &str,
        user: i64,
        key: &str,
        channel: &str,
        revision: i64,
        baseline_json: &str,
        options_json: &str,
        sha256: &str,
        at: i64,
    ) -> rusqlite::Result<()> {
        let count = self.0.execute(
            "INSERT INTO timetable_outputs SELECT w.site_id,w.week_key,?4,w.cohort,?6,?5,?7,?8,?9 FROM timetable_weeks w JOIN sites s ON s.id=w.site_id AND s.user_id=w.user_id WHERE w.site_id=?1 AND w.user_id=?2 AND w.week_key=?3 AND w.revision=?5
             ON CONFLICT(site_id,week_key,channel) DO UPDATE SET cohort=excluded.cohort,baseline_json=excluded.baseline_json,revision=excluded.revision,options_json=excluded.options_json,sha256=excluded.sha256,published_at=excluded.published_at",
            params![site, user, key, channel, revision, baseline_json, options_json, sha256, at],
        )?;
        if count == 1 {
            Ok(())
        } else {
            Err(rusqlite::Error::InvalidQuery)
        }
    }

    /// Sites whose saved weeks are kept in a Calendar calendar:
    /// `(site, user, calendar identifier once created)`.
    pub fn timetable_calendars(&self) -> rusqlite::Result<Vec<(String, i64, Option<String>)>> {
        let mut stmt = self.0.prepare(
            "SELECT c.site_id,c.user_id,c.calendar_id FROM timetable_calendars c JOIN sites s ON s.id=c.site_id AND s.user_id=c.user_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        rows.collect()
    }

    /// Turn Calendar sync on (keeping a known calendar) or record its calendar.
    pub fn set_timetable_calendar(
        &self,
        site: &str,
        user: i64,
        calendar: Option<&str>,
        at: i64,
    ) -> rusqlite::Result<()> {
        let count = self.0.execute(
            "INSERT INTO timetable_calendars SELECT ?1,?2,?3,?4 FROM sites WHERE id=?1 AND user_id=?2
             ON CONFLICT(site_id) DO UPDATE SET calendar_id=COALESCE(excluded.calendar_id,timetable_calendars.calendar_id) WHERE timetable_calendars.user_id=excluded.user_id",
            params![site, user, calendar, at],
        )?;
        if count == 1 {
            Ok(())
        } else {
            Err(rusqlite::Error::InvalidQuery)
        }
    }

    /// Stop Calendar sync; the calendar itself stays in Calendar.
    pub fn disable_timetable_calendar(&self, site: &str, user: i64) -> rusqlite::Result<()> {
        let tx = self.0.unchecked_transaction()?;
        tx.execute("DELETE FROM timetable_calendar_events WHERE site_id=?1 AND EXISTS (SELECT 1 FROM sites WHERE id=?1 AND user_id=?2)", params![site, user])?;
        tx.execute(
            "DELETE FROM timetable_calendars WHERE site_id=?1 AND user_id=?2",
            params![site, user],
        )?;
        tx.commit()
    }

    /// Which Calendar event holds each session UID.
    pub fn timetable_calendar_events(&self, site: &str) -> rusqlite::Result<Vec<(String, String)>> {
        let mut stmt = self
            .0
            .prepare("SELECT uid,event_id FROM timetable_calendar_events WHERE site_id=?1")?;
        let rows = stmt.query_map([site], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect()
    }

    pub fn set_timetable_calendar_event(
        &self,
        site: &str,
        user: i64,
        uid: &str,
        event: Option<&str>,
    ) -> rusqlite::Result<()> {
        match event {
            Some(event) => self.0.execute(
                "INSERT INTO timetable_calendar_events SELECT ?1,?3,?4 FROM sites WHERE id=?1 AND user_id=?2 ON CONFLICT(site_id,uid) DO UPDATE SET event_id=excluded.event_id",
                params![site, user, uid, event],
            )?,
            None => self.0.execute("DELETE FROM timetable_calendar_events WHERE site_id=?1 AND uid=?2", params![site, uid])?,
        };
        Ok(())
    }

    /// Forget every event mapping of a site (its calendar was recreated).
    pub fn clear_timetable_calendar_events(&self, site: &str) -> rusqlite::Result<()> {
        self.0.execute(
            "DELETE FROM timetable_calendar_events WHERE site_id=?1",
            [site],
        )?;
        Ok(())
    }

    pub fn timetable_preferences(&self, site: &str, user: i64) -> rusqlite::Result<Option<String>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row(
                "SELECT p.json FROM timetable_preferences p JOIN sites s ON s.id=p.site_id AND s.user_id=p.user_id WHERE p.site_id=?1 AND p.user_id=?2",
                params![site, user],
                |r| r.get(0),
            )
            .optional()
    }

    pub fn set_timetable_preferences(
        &self,
        site: &str,
        user: i64,
        json: &str,
    ) -> rusqlite::Result<()> {
        let count = self.0.execute(
            "INSERT INTO timetable_preferences SELECT ?1,?2,?3 FROM sites WHERE id=?1 AND user_id=?2 ON CONFLICT(site_id) DO UPDATE SET user_id=excluded.user_id,json=excluded.json",
            params![site, user, json],
        )?;
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

    fn site(store: &Store, user: i64) -> Site {
        let site = Site {
            id: "s".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: user,
            full_name: "Learner".into(),
        };
        store.insert(&site).unwrap();
        site
    }
    fn week(key: &str) -> TimetableWeek {
        TimetableWeek {
            site_id: "s".into(),
            week_key: key.into(),
            user_id: 7,
            source_ref: "course:1:abc".into(),
            filename: "Edt.pdf".into(),
            source_sha256: "00".into(),
            week_start: "2026-11-09".into(),
            week_end: "2026-11-13".into(),
            promotion: Some("2026-2028".into()),
            cohort: "ALL".into(),
            parsed_json: "[]".into(),
            choices_json: "{}".into(),
            options_json: "{}".into(),
            selected_json: "{}".into(),
            parser_version: "0.2.0".into(),
            rules_version: "r".into(),
            adapter_contract: 1,
            revision: 0,
            created_at: 0,
            updated_at: 0,
        }
    }
    fn session(uid: &str, start: i64, title: &str) -> SessionWrite {
        SessionWrite {
            uid: uid.into(),
            date: "2026-11-09".into(),
            last_date: "2026-11-09".into(),
            start_at: Some(start),
            end_at: Some(start + 3600),
            all_day: false,
            tentative: false,
            timing: format!("2026-11-09|{start}"),
            payload: format!("{{\"title\":\"{title}\"}}"),
        }
    }
    fn active(store: &Store, key: &str) -> Vec<String> {
        let (_, sessions) = store.timetable_week("s", 7, key).unwrap().unwrap();
        sessions
            .into_iter()
            .filter(|s| !s.cancelled)
            .map(|s| s.uid)
            .collect()
    }

    #[test]
    fn a_week_is_saved_reopened_and_replaced_with_cumulative_cancellations() {
        let store = Store::open(":memory:").unwrap();
        site(&store, 7);
        let a = store
            .commit_timetable_week(
                &week("k"),
                &[session("u1", 100, "A"), session("u2", 200, "B")],
                None,
                "r1",
                10,
            )
            .unwrap();
        assert_eq!(a.revision, 1);
        assert_eq!(a.diff.added.len(), 2);
        // Same request again: idempotent.
        assert_eq!(
            store
                .commit_timetable_week(&week("k"), &[], None, "r1", 11)
                .unwrap()
                .revision,
            1
        );
        assert_eq!(active(&store, "k"), ["u1", "u2"]);
        // B: u1 moved, u2 removed, u3 added.
        let b = store
            .commit_timetable_week(
                &week("k"),
                &[session("u1", 150, "A"), session("u3", 300, "C")],
                Some(1),
                "r2",
                12,
            )
            .unwrap();
        assert_eq!(
            (
                b.diff.moved.clone(),
                b.diff.removed.clone(),
                b.diff.added.clone()
            ),
            (vec!["u1".into()], vec!["u2".into()], vec!["u3".into()])
        );
        // C: u3 detail change; u2 stays cancelled across the next edit.
        let c = store
            .commit_timetable_week(
                &week("k"),
                &[session("u1", 150, "A"), session("u3", 300, "C2")],
                Some(2),
                "r3",
                13,
            )
            .unwrap();
        assert_eq!(c.diff.changed, vec!["u3".to_owned()]);
        let (_, sessions) = store.timetable_week("s", 7, "k").unwrap().unwrap();
        assert!(sessions.iter().any(|s| s.uid == "u2" && s.cancelled));
        // Restoring u2 clears its cancellation.
        store
            .commit_timetable_week(
                &week("k"),
                &[
                    session("u1", 150, "A"),
                    session("u2", 200, "B"),
                    session("u3", 300, "C2"),
                ],
                Some(3),
                "r4",
                14,
            )
            .unwrap();
        assert_eq!(active(&store, "k"), ["u1", "u2", "u3"]);
        assert_eq!(store.timetable_weeks("s", 7).unwrap()[0].sessions, 3);
    }

    #[test]
    fn competing_previews_other_accounts_and_uid_collisions_are_refused() {
        let store = Store::open(":memory:").unwrap();
        let mut owner = site(&store, 7);
        store
            .commit_timetable_week(&week("k"), &[session("u1", 100, "A")], None, "r1", 10)
            .unwrap();
        assert!(matches!(
            store.commit_timetable_week(&week("k"), &[], None, "other", 11),
            Err(CommitError::Conflict)
        ));
        assert!(matches!(
            store.commit_timetable_week(&week("k"), &[], Some(5), "other", 11),
            Err(CommitError::Conflict)
        ));
        assert!(matches!(
            store.commit_timetable_week(&week("k2"), &[session("u1", 100, "A")], None, "r9", 11),
            Err(CommitError::UidCollision)
        ));
        assert!(
            store.timetable_week("s", 7, "k2").unwrap().is_none(),
            "no partial commit"
        );
        let stranger = TimetableWeek {
            user_id: 8,
            ..week("k3")
        };
        assert!(matches!(
            store.commit_timetable_week(&stranger, &[], None, "r", 1),
            Err(CommitError::AccountChanged)
        ));
        store
            .set_timetable_preferences("s", 7, "{\"cohort\":\"FI\"}")
            .unwrap();
        // Account replacement wipes timetable data.
        owner.user_id = 8;
        store.update(&owner).unwrap();
        assert!(store.timetable_weeks("s", 8).unwrap().is_empty());
        assert!(store.timetable_week("s", 7, "k").unwrap().is_none());
        assert!(store.timetable_preferences("s", 8).unwrap().is_none());
        // Site removal cascades.
        owner.user_id = 8;
        store
            .commit_timetable_week(
                &TimetableWeek {
                    user_id: 8,
                    ..week("k")
                },
                &[session("u1", 1, "A")],
                None,
                "r",
                1,
            )
            .unwrap();
        store.remove("s").unwrap();
        let left: i64 = store
            .0
            .query_row("SELECT COUNT(*) FROM timetable_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn removal_needs_the_current_revision_and_sessions_are_listed_by_date() {
        let store = Store::open(":memory:").unwrap();
        site(&store, 7);
        store
            .commit_timetable_week(&week("k"), &[session("u1", 100, "A")], None, "r1", 10)
            .unwrap();
        let found = store
            .timetable_sessions_between("s", 7, "2026-11-09", "2026-11-09")
            .unwrap();
        assert_eq!(found.len(), 1);
        assert!(store
            .timetable_sessions_between("s", 7, "2026-11-10", "2026-11-20")
            .unwrap()
            .is_empty());
        assert!(matches!(
            store.remove_timetable_week("s", 7, "k", 2),
            Err(CommitError::Conflict)
        ));
        store.remove_timetable_week("s", 7, "k", 1).unwrap();
        assert!(store.timetable_weeks("s", 7).unwrap().is_empty());
    }

    #[test]
    fn export_baselines_are_recorded_for_the_current_revision_only() {
        let store = Store::open(":memory:").unwrap();
        site(&store, 7);
        store
            .commit_timetable_week(&week("k"), &[session("u1", 100, "A")], None, "r1", 10)
            .unwrap();
        assert!(store
            .timetable_output_baseline("s", 7, "k", "week")
            .unwrap()
            .is_none());
        assert!(
            store
                .record_timetable_output("s", 7, "k", "week", 2, "[]", "{}", "h", 1)
                .is_err(),
            "stale revision"
        );
        store
            .record_timetable_output("s", 7, "k", "week", 1, "[{\"a\":1}]", "{}", "h", 1)
            .unwrap();
        assert_eq!(
            store
                .timetable_output_baseline("s", 7, "k", "week")
                .unwrap()
                .as_deref(),
            Some("[{\"a\":1}]")
        );
        assert!(store
            .timetable_output_baseline("s", 8, "k", "week")
            .unwrap()
            .is_none());
        store
            .record_timetable_output("s", 7, "k", "week", 1, "[]", "{}", "h2", 2)
            .unwrap();
        assert_eq!(
            store
                .timetable_output_baseline("s", 7, "k", "week")
                .unwrap()
                .as_deref(),
            Some("[]")
        );
    }

    #[test]
    fn calendar_sync_is_account_bound_and_keeps_its_calendar() {
        let store = Store::open(":memory:").unwrap();
        let mut owner = site(&store, 7);
        assert!(
            store.set_timetable_calendar("s", 8, None, 1).is_err(),
            "other account"
        );
        store.set_timetable_calendar("s", 7, None, 1).unwrap();
        assert_eq!(
            store.timetable_calendars().unwrap(),
            vec![("s".into(), 7, None)]
        );
        store
            .set_timetable_calendar("s", 7, Some("cal-1"), 2)
            .unwrap();
        store.set_timetable_calendar("s", 7, None, 3).unwrap();
        assert_eq!(
            store.timetable_calendars().unwrap()[0].2.as_deref(),
            Some("cal-1"),
            "kept"
        );
        store
            .set_timetable_calendar_event("s", 7, "u1", Some("e1"))
            .unwrap();
        assert!(store
            .set_timetable_calendar_event("s", 8, "u2", Some("e2"))
            .is_ok());
        assert_eq!(
            store.timetable_calendar_events("s").unwrap(),
            vec![("u1".into(), "e1".into())]
        );
        store
            .set_timetable_calendar_event("s", 7, "u1", None)
            .unwrap();
        assert!(store.timetable_calendar_events("s").unwrap().is_empty());
        store
            .set_timetable_calendar_event("s", 7, "u1", Some("e1"))
            .unwrap();
        owner.user_id = 8;
        store.update(&owner).unwrap();
        assert!(store.timetable_calendars().unwrap().is_empty());
        assert!(store.timetable_calendar_events("s").unwrap().is_empty());
        store
            .set_timetable_calendar("s", 8, Some("cal-2"), 4)
            .unwrap();
        store
            .set_timetable_calendar_event("s", 8, "u1", Some("e1"))
            .unwrap();
        store.disable_timetable_calendar("s", 8).unwrap();
        assert!(store.timetable_calendars().unwrap().is_empty());
        assert!(store.timetable_calendar_events("s").unwrap().is_empty());
    }

    #[test]
    fn the_schema_migrates_from_version_18() {
        let path = std::env::temp_dir().join(format!("moodburo-timetable-{}.db", uuid_like()));
        {
            let store = Store::open(&path).unwrap();
            store.0.execute_batch("DROP TABLE timetable_events; DROP TABLE timetable_outputs; DROP TABLE timetable_reminders; DROP TABLE timetable_weeks; PRAGMA user_version=18;").unwrap();
        }
        let store = Store::open(&path).unwrap();
        let version: u32 = store
            .0
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 19);
        site(&store, 7);
        store
            .commit_timetable_week(&week("k"), &[], None, "r", 1)
            .unwrap();
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    fn uuid_like() -> String {
        format!("{}-{:?}", std::process::id(), std::thread::current().id()).replace(['(', ')'], "")
    }
}
