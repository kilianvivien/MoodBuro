//! Backend-owned timetable drafts: preview, review, commit (plan §5).
//!
//! A draft holds the engine's original `parsed` payload, the reviewer's
//! choices and the current selection. Nothing is saved, exported or notified
//! during preview. A commit is checked against the draft revision, the saved
//! week's revision and the account, then written in one transaction.
use crate::timetable::{self, code, Source};
use crate::AppState;
use md_inspcal::request::{self, Answer, Choices, Options};
use md_inspcal::week::{self, Session};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const DRAFT_TTL: Duration = Duration::from_secs(15 * 60);
const DRAFTS_PER_ACCOUNT: usize = 2;
const MAX_DRAFT_BYTES: usize = 8 * 1024 * 1024;

struct Draft {
    site_id: String,
    user_id: i64,
    generation: u64,
    revision: u64,
    source_ref: String,
    filename: String,
    source_sha256: String,
    week_key: String,
    /// Saved week revision this draft would replace.
    base: Option<i64>,
    parsed: Value,
    choices: Choices,
    options: Options,
    choices_reset: bool,
    answer: Answer,
    sessions: Vec<Session>,
    touched: Instant,
    /// Request that committed this draft; a retry with it is answered again.
    committed: Option<String>,
}

#[derive(Default)]
pub struct Drafts {
    drafts: Mutex<HashMap<String, Draft>>,
    generations: Mutex<HashMap<String, u64>>,
}

impl Drafts {
    fn generation(&self, site: &str) -> u64 {
        self.generations
            .lock()
            .map(|g| g.get(site).copied().unwrap_or(0))
            .unwrap_or(u64::MAX)
    }
    /// Account removed or replaced: drop its drafts; late results are refused.
    pub fn forget_site(&self, site: &str) {
        if let Ok(mut generations) = self.generations.lock() {
            *generations.entry(site.to_owned()).or_default() += 1;
        }
        if let Ok(mut drafts) = self.drafts.lock() {
            drafts.retain(|_, draft| draft.site_id != site);
        }
    }
    /// Keep a new draft: refused if its account changed since it started,
    /// over the size bound, and evicting the account's oldest beyond two.
    fn insert(&self, draft: Draft) -> Result<String, &'static str> {
        let size = draft.parsed.to_string().len() + draft.answer.selected.to_string().len();
        if size > MAX_DRAFT_BYTES {
            return Err("TOO_LARGE");
        }
        if self.generation(&draft.site_id) != draft.generation {
            return Err(code::ACCOUNT_CHANGED);
        }
        let mut map = self.drafts.lock().map_err(|_| code::STORAGE)?;
        map.retain(|_, known| known.touched.elapsed() < DRAFT_TTL);
        let mut mine: Vec<(String, Instant)> = map
            .iter()
            .filter(|(_, known)| known.site_id == draft.site_id && known.user_id == draft.user_id)
            .map(|(id, known)| (id.clone(), known.touched))
            .collect();
        mine.sort_by_key(|(_, touched)| *touched);
        while mine.len() >= DRAFTS_PER_ACCOUNT {
            let (oldest, _) = mine.remove(0);
            map.remove(&oldest);
        }
        let id = uuid::Uuid::new_v4().to_string();
        map.insert(id.clone(), draft);
        Ok(id)
    }
    /// Use a live draft; an expired or unknown one is `STALE_DRAFT`.
    fn with<T>(
        &self,
        id: &str,
        f: impl FnOnce(&mut Draft) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut map = self.drafts.lock().map_err(|_| code::STORAGE)?;
        let draft = map
            .get_mut(id)
            .filter(|d| d.touched.elapsed() < DRAFT_TTL)
            .ok_or("STALE_DRAFT")?;
        draft.touched = Instant::now();
        f(draft)
    }
    /// Shell teardown: drafts never outlive the window that reviewed them.
    pub fn clear(&self) {
        if let Ok(mut drafts) = self.drafts.lock() {
            drafts.clear();
        }
    }
}

/// Forget everything in flight for a site whose account changed.
pub fn forget_site(app: &AppHandle, site: &str) {
    app.state::<Drafts>().forget_site(site);
    app.state::<timetable::State>().forget_site(site);
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// Not saved yet: primary action “Add week”.
    New,
    /// A saved week would change: “Update week”.
    Update,
    /// Same as the saved week: “Show saved week”.
    Unchanged,
    /// Editing choices of a saved week: “Save changes”.
    Edit,
    /// The saved week, read-only.
    Saved,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    #[serde(flatten)]
    session: Value,
    cancelled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekView {
    site_id: String,
    week_key: String,
    week_start: String,
    week_end: String,
    promotion: Option<String>,
    filename: String,
    mode: Mode,
    draft_id: Option<String>,
    draft_revision: Option<u64>,
    saved_revision: Option<i64>,
    sessions: Vec<SessionView>,
    questions: Value,
    pending: usize,
    warnings: Vec<Value>,
    diff: Option<md_store::WeekDiff>,
    options: Options,
    choices: Choices,
    choices_reset: bool,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_secs() as i64)
}

fn payload(session: &Session, selected: &Value) -> String {
    let mut value = serde_json::to_value(session).unwrap_or(Value::Null);
    let links = event_json(selected, &session.uid)
        .map(|event| request::link_labels(&event))
        .unwrap_or_default();
    value["links"] = json!(links);
    value.to_string()
}

fn event_json(selected: &Value, uid: &str) -> Option<Value> {
    selected["events"]
        .as_array()?
        .iter()
        .find(|event| {
            event["raw"]["choice_uid"]
                .as_str()
                .filter(|s| !s.is_empty())
                .or(event["uid"].as_str())
                == Some(uid)
        })
        .cloned()
}

fn writes(sessions: &[Session], selected: &Value) -> Vec<md_store::SessionWrite> {
    sessions
        .iter()
        .map(|session| md_store::SessionWrite {
            uid: session.uid.clone(),
            date: session.date.clone(),
            last_date: session.last_date.clone(),
            start_at: session.start_at,
            end_at: session.end_at,
            all_day: session.all_day,
            tentative: session.tentative,
            timing: format!(
                "{}|{}|{}|{}",
                session.date,
                session.last_date,
                session.start.as_deref().unwrap_or(""),
                session.end.as_deref().unwrap_or("")
            ),
            payload: payload(session, selected),
        })
        .collect()
}

/// Unanswered review questions; they stay visible and tentative, never a
/// prerequisite to saving.
fn pending(questions: &Value, choices: &Choices) -> usize {
    let unanswered = |key: &str, answered: &dyn Fn(&str) -> bool| {
        questions[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| item["uid"].as_str().is_some_and(|uid| !answered(uid)))
            .count()
    };
    let weeks = |key: &str, answered: &dyn Fn(&str) -> bool| {
        questions[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| {
                item["weekStart"]
                    .as_str()
                    .is_some_and(|week| !answered(week))
            })
            .count()
    };
    unanswered("groups", &|uid| choices.groups.contains_key(uid))
        + unanswered("participation", &|uid| choices.groups.contains_key(uid))
        + unanswered("times", &|uid| choices.times.contains_key(uid))
        + unanswered("lv1", &|uid| choices.rooms.contains_key(uid))
        + weeks("halfPromotion", &|week| {
            choices.half_promotion.contains_key(week)
        })
        + questions["sport"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|week| {
                week["sessions"].as_array().into_iter().flatten().any(|s| {
                    s["uid"]
                        .as_str()
                        .is_some_and(|uid| !choices.sport.contains_key(uid))
                })
            })
            .count()
}

fn bounds(answer: &Answer) -> Result<(String, String, Option<String>), String> {
    week::week_bounds(&answer.selected).ok_or_else(|| "WEEK_DATE_MISSING".into())
}

fn draft_view(
    id: &str,
    draft: &Draft,
    mode: Mode,
    diff: Option<md_store::WeekDiff>,
) -> Result<WeekView, String> {
    let (week_start, week_end, promotion) = bounds(&draft.answer)?;
    let questions = request::review_questions(&draft.answer);
    let selected = &draft.answer.selected;
    Ok(WeekView {
        site_id: draft.site_id.clone(),
        week_key: draft.week_key.clone(),
        week_start,
        week_end,
        promotion,
        filename: draft.filename.clone(),
        mode,
        draft_id: Some(id.to_owned()),
        draft_revision: Some(draft.revision),
        saved_revision: draft.base,
        sessions: draft
            .sessions
            .iter()
            .map(|session| SessionView {
                session: serde_json::from_str(&payload(session, selected)).unwrap_or(Value::Null),
                cancelled: false,
            })
            .collect(),
        pending: pending(&questions, &draft.choices),
        questions,
        warnings: request::warnings(&draft.answer),
        diff,
        options: draft.options.clone(),
        choices: draft.choices.clone(),
        choices_reset: draft.choices_reset,
    })
}

/// One engine call under the admission permit, off the async runtime.
async fn run(
    permit: md_inspcal::Permit,
    fields: Vec<(Value, Vec<md_inspcal::Upload>)>,
) -> Result<Vec<Value>, String> {
    let engine = md_inspcal::engine().map_err(|e| e.code().to_owned())?;
    let stamp = now();
    tauri::async_runtime::spawn_blocking(move || {
        md_inspcal::run_job(permit, md_inspcal::DEFAULT_WAIT, move |permit| {
            fields
                .into_iter()
                .map(|(fields, uploads)| engine.convert(permit, &fields, &uploads, stamp))
                .collect::<Result<Vec<_>, _>>()
        })
    })
    .await
    .map_err(|_| md_inspcal::EngineError::Internal.code().to_owned())?
    .map_err(|e| e.code().to_owned())?
    .map_err(|e| e.code().to_owned())
}

fn refused(error: request::Refused) -> String {
    error.code.to_owned()
}

fn invalid(error: request::Invalid) -> String {
    error.code().to_owned()
}

fn preferences(app: &AppHandle, site: &str, user: i64) -> Options {
    app.state::<AppState>()
        .store
        .lock()
        .ok()
        .and_then(|store| store.timetable_preferences(site, user).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn saved(
    app: &AppHandle,
    site: &str,
    user: i64,
    key: &str,
) -> Result<Option<(md_store::TimetableWeek, Vec<md_store::StoredSession>)>, String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| code::STORAGE)?
        .timetable_week(site, user, key)
        .map_err(|_| code::STORAGE.into())
}

fn insert(app: &AppHandle, draft: Draft) -> Result<String, String> {
    app.state::<Drafts>().insert(draft).map_err(str::to_owned)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareRequest {
    site_id: String,
    user_id: i64,
    source: Source,
    /// The Monday, when the PDF's dates could not be read (`YYYY-MM-DD`).
    week_start: Option<String>,
}

/// Read a timetable and open it as a draft week.
pub async fn prepare(app: &AppHandle, input: PrepareRequest) -> Result<WeekView, String> {
    let PrepareRequest {
        site_id,
        user_id,
        source,
        week_start,
    } = input;
    timetable::supported_site(app, &site_id, user_id)?;
    let iso = |day: &str| {
        day.len() == 10
            && day.bytes().enumerate().all(|(i, b)| {
                if i == 4 || i == 7 {
                    b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            })
    };
    if week_start.as_deref().is_some_and(|day| !iso(day)) {
        return Err("INVALID_CHOICE".into());
    }
    let generation = app.state::<Drafts>().generation(&site_id);
    // Admission before any fetch: a second activation is refused, not queued.
    let permit = md_inspcal::try_admit().map_err(|e| e.code().to_owned())?;
    let fetched = timetable::resolve(app, &site_id, user_id, source).await?;
    let options = Options {
        split_by_module: false,
        ..preferences(app, &site_id, user_id)
    };
    let mut fields = request::fields(&options, &Choices::default(), None, &[]).map_err(invalid)?;
    if let Some(day) = &week_start {
        fields["week_start"] = json!(day);
    }
    let sha256 = {
        use sha2::Digest;
        sha2::Sha256::digest(&fetched.bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let upload = md_inspcal::Upload {
        name: Some(fetched.filename.clone()),
        content_type: Some("application/pdf".into()),
        bytes: fetched.bytes,
    };
    let first = run(permit, vec![(fields, vec![upload])]).await?;
    let answer = request::answer(&first[0]).map_err(refused)?;
    let week_key = week::week_key(&answer.selected).map_err(|_| "WEEK_DATE_MISSING".to_owned())?;
    let stored = saved(app, &site_id, user_id, &week_key)?;
    // A corrected PDF keeps the saved week's choices where they still apply.
    let (answer, choices, options, choices_reset) = match &stored {
        Some((week, _)) => {
            let saved_choices: Choices =
                serde_json::from_str(&week.choices_json).unwrap_or_default();
            let saved_options: Options =
                serde_json::from_str(&week.options_json).unwrap_or_default();
            let fields = request::fields(&saved_options, &saved_choices, Some(&answer.parsed), &[])
                .map_err(invalid)?;
            let permit = md_inspcal::try_admit().map_err(|e| e.code().to_owned())?;
            match run(permit, vec![(fields, vec![])])
                .await
                .map(|mut v| request::answer(&v.remove(0)))
            {
                Ok(Ok(reviewed)) => (reviewed, saved_choices, saved_options, false),
                Ok(Err(_)) => (
                    answer,
                    Choices::default(),
                    saved_options,
                    !saved_choices.is_empty(),
                ),
                Err(error) => return Err(error),
            }
        }
        None => (answer, Choices::default(), options, false),
    };
    let sessions = week::sessions(&answer.selected, answer.calendar.as_deref())
        .map_err(|_| code::STORAGE.to_owned())?;
    let (mode, diff) = match &stored {
        None => (Mode::New, None),
        Some((week, old)) => {
            let diff = md_store::week_diff(old, &writes(&sessions, &answer.selected));
            let same = week.source_sha256 == sha256 && diff.is_empty() && !choices_reset;
            (
                if same { Mode::Unchanged } else { Mode::Update },
                Some(diff),
            )
        }
    };
    timetable::supported_site(app, &site_id, user_id)?;
    let draft = Draft {
        site_id,
        user_id,
        generation,
        revision: 1,
        source_ref: fetched.reference,
        filename: fetched.filename,
        source_sha256: sha256,
        week_key,
        base: stored.as_ref().map(|(week, _)| week.revision),
        parsed: answer.parsed.clone(),
        choices,
        options,
        choices_reset,
        sessions,
        answer,
        touched: Instant::now(),
        committed: None,
    };
    let view_draft = |id: &str, draft: &Draft| draft_view(id, draft, mode, diff.clone());
    let id = insert(app, draft)?;
    with_draft(app, &id, |draft| view_draft(&id, draft))
}

fn with_draft<T>(
    app: &AppHandle,
    id: &str,
    f: impl FnOnce(&mut Draft) -> Result<T, String>,
) -> Result<T, String> {
    app.state::<Drafts>().with(id, f)
}

/// Re-apply choices to the original parsed week; the PDF is not read again.
pub async fn review(
    app: &AppHandle,
    id: &str,
    expected: u64,
    choices: Choices,
    options: Options,
) -> Result<WeekView, String> {
    let (parsed, site_id, user_id, generation) = with_draft(app, id, |draft| {
        if draft.revision != expected || draft.committed.is_some() {
            return Err("STALE_DRAFT".into());
        }
        Ok((
            draft.parsed.clone(),
            draft.site_id.clone(),
            draft.user_id,
            draft.generation,
        ))
    })?;
    timetable::supported_site(app, &site_id, user_id)?;
    let options = Options {
        split_by_module: false,
        ..options
    };
    let fields = request::fields(&options, &choices, Some(&parsed), &[]).map_err(invalid)?;
    let permit = md_inspcal::try_admit().map_err(|e| e.code().to_owned())?;
    let mut answers = run(permit, vec![(fields, vec![])]).await?;
    let answer = request::answer(&answers.remove(0)).map_err(refused)?;
    let sessions = week::sessions(&answer.selected, answer.calendar.as_deref())
        .map_err(|_| code::STORAGE.to_owned())?;
    if app.state::<Drafts>().generation(&site_id) != generation {
        return Err(code::ACCOUNT_CHANGED.into());
    }
    let stored = with_draft(app, id, |draft| Ok(draft.week_key.clone()))
        .and_then(|key| saved(app, &site_id, user_id, &key))?;
    with_draft(app, id, |draft| {
        // Ignore a late answer for an older revision.
        if draft.revision != expected {
            return Err("STALE_DRAFT".into());
        }
        draft.revision += 1;
        draft.choices = choices;
        draft.options = options;
        draft.choices_reset = false;
        draft.sessions = sessions;
        draft.answer = answer;
        let (mode, diff) = match &stored {
            None => (Mode::New, None),
            Some((week, old)) => {
                let diff =
                    md_store::week_diff(old, &writes(&draft.sessions, &draft.answer.selected));
                let same = week.source_sha256 == draft.source_sha256
                    && diff.is_empty()
                    && week.choices_json
                        == serde_json::to_string(&draft.choices).unwrap_or_default()
                    && week.options_json
                        == serde_json::to_string(&draft.options).unwrap_or_default();
                (
                    if same {
                        Mode::Unchanged
                    } else if draft.source_ref == "saved" {
                        Mode::Edit
                    } else {
                        Mode::Update
                    },
                    Some(diff),
                )
            }
        };
        draft_view(id, draft, mode, diff)
    })
}

/// Add or update the week. Retrying the same `request_id` is a no-op.
pub fn commit(
    app: &AppHandle,
    id: &str,
    expected: u64,
    request_id: &str,
) -> Result<WeekView, String> {
    if request_id.is_empty() || request_id.len() > 64 {
        return Err("INVALID_CHOICE".into());
    }
    let (site_id, user_id, key) = with_draft(app, id, |draft| {
        if let Some(done) = &draft.committed {
            // A retry of the same commit is answered again; anything else is stale.
            return if done == request_id {
                Ok((draft.site_id.clone(), draft.user_id, draft.week_key.clone()))
            } else {
                Err("STALE_DRAFT".into())
            };
        }
        if draft.revision != expected {
            return Err("STALE_DRAFT".into());
        }
        if app.state::<Drafts>().generation(&draft.site_id) != draft.generation {
            return Err(code::ACCOUNT_CHANGED.into());
        }
        let (week_start, week_end, promotion) = bounds(&draft.answer)?;
        let source_ref = if draft.source_ref == "saved" {
            saved(app, &draft.site_id, draft.user_id, &draft.week_key)?
                .map(|(w, _)| w.source_ref)
                .unwrap_or_default()
        } else {
            draft.source_ref.clone()
        };
        let record = md_store::TimetableWeek {
            site_id: draft.site_id.clone(),
            week_key: draft.week_key.clone(),
            user_id: draft.user_id,
            source_ref,
            filename: draft.filename.clone(),
            source_sha256: draft.source_sha256.clone(),
            week_start,
            week_end,
            promotion,
            cohort: serde_json::to_value(draft.options.cohort)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            parsed_json: draft.parsed.to_string(),
            choices_json: serde_json::to_string(&draft.choices).map_err(|_| code::STORAGE)?,
            options_json: serde_json::to_string(&draft.options).map_err(|_| code::STORAGE)?,
            selected_json: draft.answer.selected.to_string(),
            parser_version: md_inspcal::ENGINE_VERSION.into(),
            rules_version: md_inspcal::rules_version(),
            adapter_contract: i64::from(md_inspcal::ADAPTER_CONTRACT),
            revision: 0,
            created_at: now(),
            updated_at: now(),
        };
        let rows = writes(&draft.sessions, &draft.answer.selected);
        let state = app.state::<AppState>();
        let store = state.store.lock().map_err(|_| code::STORAGE)?;
        match store.commit_timetable_week(&record, &rows, draft.base, request_id, now()) {
            Ok(_) => {}
            Err(md_store::CommitError::Conflict) => return Err("WEEK_CHANGED".into()),
            Err(md_store::CommitError::AccountChanged) => return Err(code::ACCOUNT_CHANGED.into()),
            Err(md_store::CommitError::UidCollision) => return Err("UID_COLLISION".into()),
            Err(md_store::CommitError::Storage(_)) => return Err(code::STORAGE.into()),
        }
        // Remember the cohort as the default for the next week.
        let defaults = Options {
            split_by_module: false,
            ..draft.options.clone()
        };
        let _ = store.set_timetable_preferences(
            &draft.site_id,
            draft.user_id,
            &serde_json::to_string(&defaults).unwrap_or_default(),
        );
        draft.committed = Some(request_id.to_owned());
        Ok((draft.site_id.clone(), draft.user_id, draft.week_key.clone()))
    })?;
    #[cfg(target_os = "macos")]
    crate::timetable_calendar::changed(app, &site_id, user_id);
    week_view(app, &site_id, user_id, &key)
}

pub fn discard(app: &AppHandle, id: &str) {
    if let Ok(mut map) = app.state::<Drafts>().drafts.lock() {
        map.remove(id);
    }
}

/// A saved week, from SQLite only: no PDF, no network.
pub fn week_view(app: &AppHandle, site: &str, user: i64, key: &str) -> Result<WeekView, String> {
    timetable::supported_site(app, site, user)?;
    let (week, sessions) = saved(app, site, user, key)?.ok_or("WEEK_MISSING")?;
    let options: Options = serde_json::from_str(&week.options_json).unwrap_or_default();
    let choices: Choices = serde_json::from_str(&week.choices_json).unwrap_or_default();
    Ok(WeekView {
        site_id: week.site_id,
        week_key: week.week_key,
        week_start: week.week_start,
        week_end: week.week_end,
        promotion: week.promotion,
        filename: week.filename,
        mode: Mode::Saved,
        draft_id: None,
        draft_revision: None,
        saved_revision: Some(week.revision),
        pending: sessions
            .iter()
            .filter(|s| !s.cancelled && s.tentative)
            .count(),
        sessions: sessions
            .into_iter()
            .filter(|s| !s.cancelled)
            .map(|s| SessionView {
                session: serde_json::from_str(&s.payload).unwrap_or(Value::Null),
                cancelled: false,
            })
            .collect(),
        questions: Value::Null,
        warnings: Vec::new(),
        diff: None,
        options,
        choices,
        choices_reset: false,
    })
}

/// Reopen a saved week's choices as a draft, without the PDF.
pub async fn edit(
    app: &AppHandle,
    site: &str,
    user: i64,
    key: &str,
    expected: i64,
) -> Result<WeekView, String> {
    timetable::supported_site(app, site, user)?;
    let generation = app.state::<Drafts>().generation(site);
    let (week, _) = saved(app, site, user, key)?.ok_or("WEEK_MISSING")?;
    if week.revision != expected {
        return Err("WEEK_CHANGED".into());
    }
    let parsed: Value = serde_json::from_str(&week.parsed_json).map_err(|_| code::STORAGE)?;
    let choices: Choices = serde_json::from_str(&week.choices_json).unwrap_or_default();
    let options: Options = serde_json::from_str(&week.options_json).unwrap_or_default();
    let fields = request::fields(&options, &choices, Some(&parsed), &[]).map_err(invalid)?;
    let permit = md_inspcal::try_admit().map_err(|e| e.code().to_owned())?;
    let mut answers = run(permit, vec![(fields, vec![])]).await?;
    let answer = request::answer(&answers.remove(0)).map_err(refused)?;
    let sessions = week::sessions(&answer.selected, answer.calendar.as_deref())
        .map_err(|_| code::STORAGE.to_owned())?;
    let draft = Draft {
        site_id: site.to_owned(),
        user_id: user,
        generation,
        revision: 1,
        source_ref: "saved".into(),
        filename: week.filename.clone(),
        source_sha256: week.source_sha256.clone(),
        week_key: key.to_owned(),
        base: Some(week.revision),
        parsed,
        choices,
        options,
        choices_reset: false,
        answer,
        sessions,
        touched: Instant::now(),
        committed: None,
    };
    let id = insert(app, draft)?;
    with_draft(app, &id, |draft| {
        draft_view(&id, draft, Mode::Edit, Some(md_store::WeekDiff::default()))
    })
}

pub fn remove(
    app: &AppHandle,
    site: &str,
    user: i64,
    key: &str,
    expected: i64,
) -> Result<(), String> {
    timetable::supported_site(app, site, user)?;
    let state = app.state::<AppState>();
    let store = state.store.lock().map_err(|_| code::STORAGE)?;
    match store.remove_timetable_week(site, user, key, expected) {
        Ok(()) => {
            drop(store);
            #[cfg(target_os = "macos")]
            crate::timetable_calendar::changed(app, site, user);
            Ok(())
        }
        Err(md_store::CommitError::Conflict) => Err("WEEK_CHANGED".into()),
        Err(md_store::CommitError::AccountChanged) => Err(code::ACCOUNT_CHANGED.into()),
        Err(_) => Err(code::STORAGE.into()),
    }
}

pub fn weeks(app: &AppHandle, site: &str, user: i64) -> Result<Vec<md_store::WeekSummary>, String> {
    timetable::supported_site(app, site, user)?;
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| code::STORAGE)?
        .timetable_weeks(site, user)
        .map_err(|_| code::STORAGE.into())
}

/// Open one of a session's links in the default browser, resolved here so
/// the shell never handles URLs. HTTPS only, no credentials.
/// Where a session link was opened.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Opened {
    /// In the organisation's own Moodle view, inside MoodBuro.
    Moodle,
    /// In the default browser (another website).
    Browser,
}

/// Open one of a session's links, resolved here so the shell never handles
/// URLs. A page of the organisation's Moodle opens in MoodBuro's Moodle view
/// under the usual navigation policy; another HTTPS site opens in the
/// default browser. Works for a saved week or the current preview.
pub fn open_link(
    app: &AppHandle,
    site: &str,
    user: i64,
    key: &str,
    draft: Option<&str>,
    uid: &str,
    index: usize,
) -> Result<Opened, String> {
    use tauri::Emitter;
    use tauri_plugin_opener::OpenerExt;
    let saved_site = timetable::supported_site(app, site, user)?;
    let selected: Value = match draft {
        Some(id) => with_draft(app, id, |draft| {
            if draft.site_id != site || draft.user_id != user {
                return Err(code::ACCOUNT_CHANGED.into());
            }
            Ok(draft.answer.selected.clone())
        })?,
        None => {
            let (week, _) = saved(app, site, user, key)?.ok_or("WEEK_MISSING")?;
            serde_json::from_str(&week.selected_json).map_err(|_| code::STORAGE)?
        }
    };
    let event = event_json(&selected, uid).ok_or("WEEK_MISSING")?;
    let url = request::safe_link(&event, index).ok_or(code::SOURCE_UNAVAILABLE)?;
    let base =
        md_moodle::normalize_site(&saved_site.wwwroot).map_err(|_| code::UNSUPPORTED_SITE)?;
    match md_moodle::navigation(&base, &url) {
        md_moodle::Navigation::Allow => {
            crate::windows::switch_at(app, site, Some(url))
                .map_err(|_| code::SOURCE_UNAVAILABLE)?;
            let _ = app.emit_to("shell", "navigation://site-opened", site);
            Ok(Opened::Moodle)
        }
        md_moodle::Navigation::External if md_moodle::shareable_url(&url) => {
            app.opener()
                .open_url(url.as_str(), None::<&str>)
                .map_err(|_| code::SOURCE_UNAVAILABLE)?;
            Ok(Opened::Browser)
        }
        _ => Err(code::SOURCE_UNAVAILABLE.into()),
    }
}

/// Channel of the export baseline: what Calendar was last given for a week.
const EXPORT_CHANNEL: &str = "week-file";
/// Earlier exports kept per week, so a cancellation outlives later edits.
const MAX_BASELINES: usize = 12;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedWeek {
    filename: String,
    sessions: usize,
    cancelled: usize,
    revealed: bool,
    opened: bool,
    /// False when the file was saved but its export history was not; the
    /// next export then omits cancellations it cannot know about.
    history_saved: bool,
}

/// Export a saved week as one .ics in Downloads, written by the engine:
/// stable UIDs, so importing a corrected week updates its sessions, and
/// sessions removed since an earlier export are cancelled. With `calendar`,
/// the file is then handed to Calendar (which asks where to add it).
pub fn export(
    app: &AppHandle,
    site: &str,
    user: i64,
    key: &str,
    expected: i64,
    calendar: bool,
) -> Result<ExportedWeek, String> {
    use tauri_plugin_opener::OpenerExt;
    timetable::supported_site(app, site, user)?;
    let (week, sessions) = saved(app, site, user, key)?.ok_or("WEEK_MISSING")?;
    if week.revision != expected {
        return Err("WEEK_CHANGED".into());
    }
    let options: Options = serde_json::from_str(&week.options_json).unwrap_or_default();
    let selected: Value = serde_json::from_str(&week.selected_json).map_err(|_| code::STORAGE)?;
    let mut baselines: Vec<Value> = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| code::STORAGE)?
        .timetable_output_baseline(site, user, key, EXPORT_CHANNEL)
        .map_err(|_| code::STORAGE)?
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();
    let bytes = week::export_calendar(&selected, &baselines, &options.prefix, options.alarm)
        .map_err(|_| md_inspcal::EngineError::Internal.code().to_owned())?;
    let text = String::from_utf8_lossy(&bytes);
    let cancelled = text.matches("STATUS:CANCELLED").count();
    let base = format!("Emploi du temps {}", week.week_start);
    let names: Vec<String> = std::iter::once(format!("{base}.ics"))
        .chain((2..=20).map(|n| format!("{base} ({n}).ics")))
        .collect();
    // ~/Moodle/Calendriers, beside the files MoodBuro already manages: no
    // extra macOS folder permission (Downloads is privacy-protected).
    let folder = app
        .path()
        .home_dir()
        .map_err(|_| "EXPORT_FAILED")?
        .join("Moodle")
        .join("Calendriers");
    let published = md_os::downloads::directory(&folder)
        .and_then(|()| crate::calendar::publish_in(&folder, &names, &bytes));
    #[cfg(debug_assertions)]
    eprintln!(
        "timetable export: {:?}",
        published.as_ref().map(|_| "saved").map_err(|e| e.kind())
    );
    let path = published.map_err(|_| "EXPORT_FAILED".to_owned())?;
    // History only after the file exists; a failure here keeps the file.
    baselines.push(selected);
    let keep = baselines.len().saturating_sub(MAX_BASELINES);
    baselines.drain(..keep);
    let sha256 = {
        use sha2::Digest;
        sha2::Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let history_saved = app
        .state::<AppState>()
        .store
        .lock()
        .ok()
        .map(|store| {
            store
                .record_timetable_output(
                    site,
                    user,
                    key,
                    EXPORT_CHANNEL,
                    week.revision,
                    &Value::Array(baselines).to_string(),
                    &week.options_json,
                    &sha256,
                    now(),
                )
                .is_ok()
        })
        .unwrap_or(false);
    let opened = calendar
        && app
            .opener()
            .open_path(path.to_string_lossy(), Some("Calendar"))
            .is_ok();
    let revealed = !opened && app.opener().reveal_item_in_dir(&path).is_ok();
    Ok(ExportedWeek {
        filename: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        sessions: sessions.iter().filter(|s| !s.cancelled).count(),
        cancelled,
        revealed,
        opened,
        history_saved,
    })
}

pub fn preferences_get(app: &AppHandle, site: &str, user: i64) -> Result<Options, String> {
    timetable::supported_site(app, site, user)?;
    Ok(preferences(app, site, user))
}

pub fn preferences_set(
    app: &AppHandle,
    site: &str,
    user: i64,
    options: Options,
) -> Result<(), String> {
    timetable::supported_site(app, site, user)?;
    options.validate().map_err(invalid)?;
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| code::STORAGE)?
        .set_timetable_preferences(
            site,
            user,
            &serde_json::to_string(&options).map_err(|_| code::STORAGE)?,
        )
        .map_err(|_| code::STORAGE.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(site: &str, generation: u64) -> Draft {
        Draft {
            site_id: site.into(),
            user_id: 1,
            generation,
            revision: 1,
            source_ref: "course:1:k".into(),
            filename: "Edt.pdf".into(),
            source_sha256: String::new(),
            week_key: "v1|a|b|0:".into(),
            base: None,
            parsed: json!([]),
            choices: Choices::default(),
            options: Options::default(),
            choices_reset: false,
            answer: Answer {
                parsed: json!([]),
                selected: json!({}),
                exportable: vec![],
                cancelled: vec![],
                questions: Default::default(),
                lv1_rooms: vec![],
                calendar: None,
            },
            sessions: vec![],
            touched: Instant::now(),
            committed: None,
        }
    }

    #[test]
    fn drafts_are_bounded_per_account_expire_and_die_with_the_account() {
        let drafts = Drafts::default();
        let first = drafts.insert(draft("s", 0)).unwrap();
        let second = drafts.insert(draft("s", 0)).unwrap();
        let third = drafts.insert(draft("s", 0)).unwrap();
        assert_eq!(
            drafts.with(&first, |_| Ok(())).unwrap_err(),
            "STALE_DRAFT",
            "oldest evicted"
        );
        assert!(
            drafts.with(&second, |_| Ok(())).is_ok() && drafts.with(&third, |_| Ok(())).is_ok()
        );
        let other = drafts.insert(draft("t", 0)).unwrap();
        // Expired drafts are unusable.
        drafts
            .with(&other, |d| {
                d.touched = Instant::now() - DRAFT_TTL;
                Ok(())
            })
            .unwrap();
        assert_eq!(drafts.with(&other, |_| Ok(())).unwrap_err(), "STALE_DRAFT");
        // An account change drops drafts and refuses a late one started before it.
        drafts.forget_site("s");
        assert!(drafts.with(&third, |_| Ok(())).is_err());
        assert_eq!(
            drafts.insert(draft("s", 0)).unwrap_err(),
            code::ACCOUNT_CHANGED
        );
        assert!(drafts.insert(draft("s", 1)).is_ok());
        let mut big = draft("t", 0);
        big.parsed = json!("x".repeat(MAX_DRAFT_BYTES));
        assert_eq!(drafts.insert(big).unwrap_err(), "TOO_LARGE");
        drafts.clear();
        assert!(drafts.drafts.lock().unwrap().is_empty());
    }

    #[test]
    fn pending_counts_unanswered_questions_only() {
        let questions = json!({
            "groups": [{"uid": "g1"}, {"uid": "g2"}],
            "participation": [{"uid": "p1"}],
            "halfPromotion": [{"weekStart": "2026-11-09"}],
            "times": [], "lv1": [],
            "sport": [{"weekStart": "2026-11-09", "sessions": [{"uid": "s1"}, {"uid": "s2"}]}]
        });
        let mut choices = Choices::default();
        assert_eq!(pending(&questions, &choices), 5);
        choices.groups.insert("g1".into(), "Groupe A".into());
        choices.groups.insert("p1".into(), String::new());
        choices
            .half_promotion
            .insert("2026-11-09".into(), "A".into());
        choices
            .sport
            .insert("s1".into(), request::SportChoice::Keep);
        assert_eq!(pending(&questions, &choices), 2);
        choices
            .sport
            .insert("s2".into(), request::SportChoice::Exclude);
        assert_eq!(pending(&questions, &choices), 1);
    }
}
