//! Saved timetable weeks kept in a dedicated calendar of Calendar (EventKit).
//!
//! With the user's permission, MoodBuro creates an « Emploi du temps »
//! calendar in the account of their default calendar (so an iCloud account
//! reaches their other devices) and keeps it equal to the saved weeks: each
//! session is one event, matched by its engine UID; moved and changed
//! sessions are updated in place, removed or cancelled ones are deleted.
//! Only events MoodBuro created (recorded by identifier) are ever touched.
//!
//! EventKit objects stay on one worker thread that owns the event store.
use crate::AppState;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_event_kit::{
    EKAlarm, EKAuthorizationStatus, EKCalendar, EKEntityType, EKEvent, EKEventAvailability,
    EKEventStore, EKSourceType, EKSpan,
};
use objc2_foundation::{NSDate, NSError, NSString, NSTimeZone};
use serde::Serialize;
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager};

const CALENDAR_TITLE: &str = "Emploi du temps";
type Job = Box<dyn FnOnce(&EKEventStore) + Send>;

/// The worker that owns the event store.
#[derive(Default)]
pub struct Worker {
    jobs: Mutex<Option<mpsc::Sender<Job>>>,
}

fn queue(app: &AppHandle, job: Job) -> Result<(), String> {
    let worker = app.state::<Worker>();
    let mut jobs = worker.jobs.lock().map_err(|_| "CALENDAR_FAILED")?;
    if jobs.is_none() {
        let (sender, receiver) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("timetable-calendar".into())
            .spawn(move || {
                // SAFETY: EKEventStore is created and used on this thread only.
                let store = unsafe { EKEventStore::new() };
                for job in receiver {
                    job(&store);
                }
            })
            .map_err(|_| "CALENDAR_FAILED")?;
        *jobs = Some(sender);
    }
    jobs.as_ref()
        .ok_or("CALENDAR_FAILED")?
        .send(job)
        .map_err(|_| "CALENDAR_FAILED".into())
}

/// Run a job on the worker and wait for its answer (off the async runtime).
fn run<T: Send + 'static>(
    app: &AppHandle,
    job: impl FnOnce(&EKEventStore) -> T + Send + 'static,
) -> Result<T, String> {
    let (sender, receiver) = mpsc::sync_channel(1);
    queue(
        app,
        Box::new(move |store| {
            let _ = sender.send(job(store));
        }),
    )?;
    // The permission prompt waits for the user.
    receiver
        .recv_timeout(Duration::from_secs(300))
        .map_err(|_| "CALENDAR_FAILED".into())
}

fn ns(text: &str) -> Retained<NSString> {
    NSString::from_str(text)
}

fn status() -> EKAuthorizationStatus {
    unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) }
}

/// Full access to events, asking once if macOS has not been asked yet.
fn ensure_access(store: &EKEventStore) -> Result<(), &'static str> {
    match status() {
        EKAuthorizationStatus::FullAccess => return Ok(()),
        EKAuthorizationStatus::NotDetermined => {}
        _ => return Err("CALENDAR_DENIED"),
    }
    let (sender, receiver) = mpsc::channel();
    let done = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
        let _ = sender.send(granted.as_bool());
    });
    unsafe {
        if objc2::available!(macos = 14.0) {
            store.requestFullAccessToEventsWithCompletion(RcBlock::as_ptr(&done));
        } else {
            #[allow(deprecated)]
            store.requestAccessToEntityType_completion(EKEntityType::Event, RcBlock::as_ptr(&done));
        }
    }
    match receiver.recv_timeout(Duration::from_secs(300)) {
        Ok(true) => Ok(()),
        _ => Err("CALENDAR_DENIED"),
    }
}

/// The site's calendar: the saved one if it still exists, else a new one in
/// the default calendar's account (falling back to On My Mac).
fn calendar(
    store: &EKEventStore,
    saved: Option<&str>,
) -> Result<(Retained<EKCalendar>, bool), &'static str> {
    if let Some(id) = saved {
        if let Some(found) = unsafe { store.calendarWithIdentifier(&ns(id)) } {
            return Ok((found, false));
        }
    }
    let create = |source| -> Option<Retained<EKCalendar>> {
        let created =
            unsafe { EKCalendar::calendarForEntityType_eventStore(EKEntityType::Event, store) };
        unsafe {
            created.setTitle(&ns(CALENDAR_TITLE));
            created.setSource(Some(source));
            store.saveCalendar_commit_error(&created, true).ok()?;
        }
        Some(created)
    };
    let preferred =
        unsafe { store.defaultCalendarForNewEvents() }.and_then(|c| unsafe { c.source() });
    if let Some(created) = preferred.as_deref().and_then(create) {
        return Ok((created, true));
    }
    let local = unsafe { store.sources() }
        .iter()
        .find(|source| unsafe { source.sourceType() } == EKSourceType::Local);
    local
        .as_deref()
        .and_then(create)
        .map(|created| (created, true))
        .ok_or("CALENDAR_FAILED")
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Shown {
    title: String,
    location: Option<String>,
    location_note: Option<String>,
    speakers: Vec<String>,
    cohort_label: Option<String>,
    module: Option<String>,
    preparation: Option<String>,
    notes: Vec<String>,
    date: String,
    last_date: String,
}

fn fill(
    event: &EKEvent,
    session: &md_store::StoredSession,
    prefix: &str,
    alarm: Option<i64>,
) -> Option<()> {
    let shown: Shown = serde_json::from_str(&session.payload).ok()?;
    let details: Vec<String> = [
        shown.cohort_label.clone(),
        (!shown.speakers.is_empty()).then(|| shown.speakers.join(", ")),
        shown.module.clone(),
        shown.preparation.clone(),
    ]
    .into_iter()
    .flatten()
    .chain(shown.notes.iter().cloned())
    .filter(|line| !line.trim().is_empty())
    .collect();
    let timed = session.start_at.filter(|_| !session.all_day);
    unsafe {
        event.setTitle(Some(&ns(&format!("{prefix}{}", shown.title))));
        event.setLocation(
            shown
                .location
                .as_deref()
                .or(shown.location_note.as_deref())
                .map(ns)
                .as_deref(),
        );
        event.setNotes(
            (!details.is_empty())
                .then(|| ns(&details.join("\n")))
                .as_deref(),
        );
        event.setAvailability(if session.tentative {
            EKEventAvailability::Tentative
        } else {
            EKEventAvailability::Busy
        });
        event.setAlarms(None);
        match timed {
            Some(start) => {
                let end = session
                    .end_at
                    .filter(|end| *end > start)
                    .unwrap_or(start + 1800);
                event.setAllDay(false);
                event.setTimeZone(NSTimeZone::timeZoneWithName(&ns("Europe/Paris")).as_deref());
                event.setStartDate(Some(&NSDate::dateWithTimeIntervalSince1970(start as f64)));
                event.setEndDate(Some(&NSDate::dateWithTimeIntervalSince1970(end as f64)));
                if let Some(minutes) = alarm {
                    event.addAlarm(&EKAlarm::alarmWithRelativeOffset(-(minutes as f64) * 60.0));
                }
            }
            None => {
                let first = md_inspcal::week::paris_noon(&shown.date)?;
                let last = md_inspcal::week::paris_noon(&shown.last_date).unwrap_or(first);
                event.setAllDay(true);
                event.setTimeZone(None);
                event.setStartDate(Some(&NSDate::dateWithTimeIntervalSince1970(first as f64)));
                event.setEndDate(Some(&NSDate::dateWithTimeIntervalSince1970(last as f64)));
            }
        }
    }
    Some(())
}

/// What a sync changed.
#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Synced {
    pub saved: usize,
    pub removed: usize,
}

/// Make the site's calendar equal to its saved weeks. Runs on the worker.
fn sync_now(
    app: &AppHandle,
    store: &EKEventStore,
    site: &str,
    user: i64,
) -> Result<Synced, String> {
    if status() != EKAuthorizationStatus::FullAccess {
        return Err("CALENDAR_DENIED".into());
    }
    let state = app.state::<AppState>();
    let (saved_id, sessions, mapped) = {
        let db = state
            .store
            .lock()
            .map_err(|_| crate::timetable::code::STORAGE)?;
        let row = db
            .timetable_calendars()
            .map_err(|_| crate::timetable::code::STORAGE)?
            .into_iter()
            .find(|(known, owner, _)| known == site && *owner == user)
            .ok_or("CALENDAR_OFF")?;
        let sessions = db
            .timetable_sessions_between(site, user, "0001-01-01", "9999-12-31")
            .map_err(|_| crate::timetable::code::STORAGE)?;
        let mapped = db
            .timetable_calendar_events(site)
            .map_err(|_| crate::timetable::code::STORAGE)?;
        (row.2, sessions, mapped)
    };
    let options = crate::timetable_review::preferences_get(app, site, user)?;
    let (target, created) = calendar(store, saved_id.as_deref())?;
    let mut mapped = mapped;
    if created {
        // A new calendar: earlier events (if any) belong to a deleted one.
        mapped.clear();
        let id = unsafe { target.calendarIdentifier() }.to_string();
        let db = state
            .store
            .lock()
            .map_err(|_| crate::timetable::code::STORAGE)?;
        db.clear_timetable_calendar_events(site)
            .map_err(|_| crate::timetable::code::STORAGE)?;
        db.set_timetable_calendar(site, user, Some(&id), 0)
            .map_err(|_| crate::timetable::code::STORAGE)?;
    }
    let mut written: Vec<(String, Retained<EKEvent>)> = Vec::new();
    for (_, session) in &sessions {
        let known = mapped
            .iter()
            .find(|(uid, _)| *uid == session.uid)
            .and_then(|(_, id)| unsafe { store.eventWithIdentifier(&ns(id)) })
            .filter(|event| {
                unsafe { event.calendar() }.is_some_and(
                    |c| unsafe { c.calendarIdentifier() } == unsafe { target.calendarIdentifier() },
                )
            });
        let event = known.unwrap_or_else(|| unsafe { EKEvent::eventWithEventStore(store) });
        unsafe { event.setCalendar(Some(&target)) };
        if fill(&event, session, &options.prefix, options.alarm).is_none() {
            continue;
        }
        unsafe { store.saveEvent_span_commit_error(&event, EKSpan::ThisEvent, false) }
            .map_err(|_| "CALENDAR_FAILED")?;
        written.push((session.uid.clone(), event));
    }
    let mut removed = Vec::new();
    for (uid, id) in &mapped {
        if sessions.iter().any(|(_, s)| s.uid == *uid) {
            continue;
        }
        if let Some(event) = unsafe { store.eventWithIdentifier(&ns(id)) } {
            unsafe { store.removeEvent_span_commit_error(&event, EKSpan::ThisEvent, false) }
                .map_err(|_| "CALENDAR_FAILED")?;
        }
        removed.push(uid.clone());
    }
    unsafe { store.commit() }.map_err(|_| "CALENDAR_FAILED")?;
    let db = state
        .store
        .lock()
        .map_err(|_| crate::timetable::code::STORAGE)?;
    for (uid, event) in &written {
        if let Some(id) = unsafe { event.eventIdentifier() } {
            let _ = db.set_timetable_calendar_event(site, user, uid, Some(&id.to_string()));
        }
    }
    for uid in &removed {
        let _ = db.set_timetable_calendar_event(site, user, uid, None);
    }
    Ok(Synced {
        saved: written.len(),
        removed: removed.len(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarStatus {
    enabled: bool,
    /// "granted", "denied" or "notDetermined".
    access: &'static str,
}

fn access() -> &'static str {
    match status() {
        EKAuthorizationStatus::FullAccess => "granted",
        EKAuthorizationStatus::NotDetermined => "notDetermined",
        _ => "denied",
    }
}

pub fn calendar_status(app: &AppHandle, site: &str, user: i64) -> Result<CalendarStatus, String> {
    crate::timetable::supported_site(app, site, user)?;
    let enabled = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| crate::timetable::code::STORAGE)?
        .timetable_calendars()
        .map_err(|_| crate::timetable::code::STORAGE)?
        .iter()
        .any(|(known, owner, _)| known == site && *owner == user);
    Ok(CalendarStatus {
        enabled,
        access: access(),
    })
}

/// Ask for Calendar access if needed, create the calendar and fill it.
pub fn enable(app: &AppHandle, site: &str, user: i64) -> Result<Synced, String> {
    crate::timetable::supported_site(app, site, user)?;
    run(app, ensure_access).and_then(|granted| granted.map_err(str::to_owned))?;
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| crate::timetable::code::STORAGE)?
        .set_timetable_calendar(site, user, None, now())
        .map_err(|_| crate::timetable::code::STORAGE)?;
    let (app2, site2) = (app.clone(), site.to_owned());
    run(app, move |store| sync_now(&app2, store, &site2, user))?
}

/// Stop keeping the calendar up to date. The calendar stays in Calendar.
pub fn disable(app: &AppHandle, site: &str, user: i64) -> Result<(), String> {
    crate::timetable::supported_site(app, site, user)?;
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| crate::timetable::code::STORAGE)?
        .disable_timetable_calendar(site, user)
        .map_err(|_| crate::timetable::code::STORAGE.into())
}

/// Saved weeks changed: update the calendar in the background, if enabled.
pub fn changed(app: &AppHandle, site: &str, user: i64) {
    let enabled = app
        .state::<AppState>()
        .store
        .lock()
        .ok()
        .and_then(|db| db.timetable_calendars().ok())
        .is_some_and(|rows| {
            rows.iter()
                .any(|(known, owner, _)| known == site && *owner == user)
        });
    if !enabled || status() != EKAuthorizationStatus::FullAccess {
        return;
    }
    let (app2, site) = (app.clone(), site.to_owned());
    let _ = queue(
        app,
        Box::new(move |store| {
            let _result = sync_now(&app2, store, &site, user);
            #[cfg(debug_assertions)]
            eprintln!(
                "timetable calendar sync: {:?}",
                _result.map(|s| (s.saved, s.removed))
            );
        }),
    );
}

/// At launch: bring every enabled calendar up to date.
pub fn start(app: &AppHandle) {
    let rows = app
        .state::<AppState>()
        .store
        .lock()
        .ok()
        .and_then(|db| db.timetable_calendars().ok())
        .unwrap_or_default();
    for (site, user, _) in rows {
        changed(app, &site, user);
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_secs() as i64)
}
