//! A conversion request, without the transport around it:
//! `web_support/convert.py` and `web_support/browser.py`.
//!
//! The answer is the status and JSON body `/api/parse` sends, so the page
//! renders this engine's results with the code it already has.

use serde_json::{json, Map, Value};

use crate::dates;
use crate::errors::EdtError;
use crate::events::is_sport;
use crate::ics::{self, IcsOptions};
use crate::institution::INSTITUTION;
use crate::model::{Document, ExtractError};
use crate::pipeline::{self, PARSER_VERSION};
use crate::py::civil::Date;
use crate::py::text;
use crate::review::clean_timetable;
use crate::rooms;
use crate::selection::{group_details, keeps_half_promotion, rotation_label, select_groups, Choices};
use crate::timetable::{truthy, Event, Timetable};

const MAX_UPLOAD_FILES: usize = 8;
const MAX_TOTAL_UPLOAD_BYTES: usize = 4_000_000;
const MAX_CHOICES: usize = 500;
const DEFAULT_FILENAME: &str = "emploi-du-temps.pdf";

/// One uploaded file, as the page hands it over.
#[derive(Debug, Clone)]
pub struct Upload {
    pub name: Option<String>,
    pub content_type: Option<String>,
    pub bytes: Vec<u8>,
}

/// What turns PDF bytes into a `Document`: PDFium in the browser and in the
/// command-line tool, a canned document in a test.
pub type Reader<'a> = dyn FnMut(&[u8], &str) -> Result<Document, ExtractError> + 'a;

/// A request refused as a whole, with the status and message to answer.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Refusal {
    status: u16,
    detail: String,
}

fn refuse<T>(status: u16, detail: &str) -> Result<T, Refusal> {
    Err(Refusal { status, detail: detail.to_owned() })
}

/// The validated form fields of one conversion request.
#[derive(Debug)]
struct Options {
    /// `ALL`, or one cohort.
    cohort: String,
    prefix: String,
    alarm: Option<i64>,
    split: bool,
    starts: Vec<Option<String>>,
    timetables: Option<String>,
    /// Weeks this reader exported before, as their device kept them.
    exported: Vec<Timetable>,
    choices: Choices,
}

fn safe_filename(value: &str) -> String {
    let value = if value.is_empty() { DEFAULT_FILENAME } else { value };
    text::head(pipeline::file_name(&value.replace('\\', "/")), 180).to_owned()
}

fn dated_filename(week_start: &str) -> Result<String, Refusal> {
    match Date::parse_iso(week_start) {
        Some(start) if start.weekday() == 0 && (1901..=9990).contains(&start.year) => {
            Ok(format!("Edt du {} au {}.pdf", start.dotted(), start.plus_days(4).dotted()))
        }
        _ => refuse(400, "La date de début de semaine est invalide."),
    }
}

fn object(value: &str, message: &str) -> Result<Map<String, Value>, Refusal> {
    match serde_json::from_str(value) {
        Ok(Value::Object(fields)) if fields.len() <= MAX_CHOICES => Ok(fields),
        _ => refuse(400, message),
    }
}

fn string_map(value: &str, message: &str) -> Result<Map<String, Value>, Refusal> {
    let fields = object(value, message)?;
    if fields.values().all(Value::is_string) {
        Ok(fields)
    } else {
        refuse(400, message)
    }
}

/// The form as `app.js` builds it: strings, some of them optional.
struct Form<'a>(&'a Map<String, Value>);

impl Form<'_> {
    /// A field that must be a string when it is there at all.
    fn text(&self, key: &str) -> Result<Option<&str>, Refusal> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value)) => Ok(Some(value)),
            Some(_) => refuse(422, "Le formulaire est invalide."),
        }
    }

    fn alarm(&self) -> Result<Option<i64>, Refusal> {
        let parsed = match self.0.get("alarm") {
            None | Some(Value::Null) => return Ok(None),
            Some(Value::String(value)) if value.is_empty() => return Ok(None),
            Some(Value::String(value)) => text::strip(value).parse::<i64>().ok(),
            Some(Value::Number(value)) => value.as_i64().or_else(|| value.as_f64().map(|float| float.trunc() as i64)),
            Some(Value::Bool(value)) => Some(i64::from(*value)),
            Some(_) => None,
        };
        parsed.map(Some).ok_or(Refusal { status: 422, detail: "Le délai du rappel est invalide.".to_owned() })
    }

    fn split(&self) -> bool {
        let spelled = match self.0.get("split_by_module") {
            Some(Value::String(value)) => value.to_lowercase(),
            Some(Value::Bool(value)) => value.to_string(),
            Some(Value::Number(value)) => value.to_string(),
            _ => String::new(),
        };
        matches!(spelled.as_str(), "true" | "1" | "on")
    }
}

/// Check every field before any work, in the order the reader is told.
fn read_options(form: &Form, pdf_count: usize) -> Result<Options, Refusal> {
    let alarm = form.alarm()?;
    if pdf_count > MAX_UPLOAD_FILES {
        return refuse(413, &format!("Vous pouvez convertir jusqu'à {MAX_UPLOAD_FILES} PDF."));
    }
    let groups = string_map(form.text("group_choices")?.unwrap_or("{}"), "Les choix de groupes sont invalides.")?;
    let half_promotions =
        string_map(form.text("half_promotion_choices")?.unwrap_or("{}"), "Les choix de demi-promo sont invalides.")?;
    let typed_rooms = string_map(form.text("room_choices")?.unwrap_or("{}"), "Les salles choisies sont invalides.")?;
    let mut picked_rooms = Map::new();
    for (uid, value) in &typed_rooms {
        // An empty value means "no choice yet", not an error: the session
        // keeps the note that sends the reviewer back to Moodle.
        let value = value.as_str().unwrap_or_default();
        if !text::strip(value).is_empty() {
            let room = rooms::clean_room(value).map_err(|detail| Refusal { status: 400, detail })?;
            picked_rooms.insert(uid.clone(), json!(room));
        }
    }
    let invalid_times = "Les horaires renseignés sont invalides.";
    let times = object(form.text("time_choices")?.unwrap_or("{}"), invalid_times)?;
    let both_hours = |value: &Value| {
        value.as_object().is_some_and(|hours| {
            hours.len() == 2 && ["start", "end"].iter().all(|key| hours.get(*key).is_some_and(Value::is_string))
        })
    };
    if !times.values().all(both_hours) {
        return refuse(400, invalid_times);
    }
    let invalid_sport = "Le choix de séance de sport est invalide.";
    let sport = object(form.text("sport_choices")?.unwrap_or("{}"), invalid_sport)?;
    if !sport.values().all(|value| matches!(value.as_str(), Some("keep" | "exclude"))) {
        return refuse(400, invalid_sport);
    }
    let cohort = form.text("cohort")?.unwrap_or("ALL").to_uppercase();
    if cohort != "ALL" && !INSTITUTION.cohort_tokens.contains(&cohort) {
        return refuse(400, "La cohorte choisie est inconnue.");
    }
    if alarm.is_some_and(|minutes| !(0..=1440).contains(&minutes)) {
        return refuse(400, "Le délai du rappel est invalide.");
    }
    let prefix = form.text("prefix")?.unwrap_or("");
    if text::len(prefix) > 60 {
        return refuse(400, "Le préfixe doit contenir au maximum 60 caractères.");
    }
    let timetables = form.text("timetables")?.filter(|payload| !payload.is_empty());
    if pdf_count == 0 && timetables.is_none() {
        return refuse(400, "Aucun PDF reçu.");
    }
    let week_start = form.text("week_start")?;
    let starts = match form.text("week_starts")?.filter(|value| !value.is_empty()) {
        None => Some(vec![week_start.map(str::to_owned); pdf_count]),
        Some(listed) => serde_json::from_str::<Value>(listed).ok().and_then(|value| {
            value
                .as_array()?
                .iter()
                .map(|item| match item {
                    Value::Null => Some(None),
                    Value::String(start) => Some(Some(start.clone())),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
        }),
    };
    let Some(starts) = starts.filter(|starts| starts.len() == pdf_count) else {
        return refuse(400, "Indiquez une date par fichier, dans le même ordre.");
    };
    if timetables.is_some() && pdf_count > 0 {
        return refuse(400, "Envoyez soit des PDF, soit une semaine déjà analysée.");
    }
    let exported = match form.text("exported")?.filter(|payload| !payload.is_empty()) {
        None => Vec::new(),
        Some(payload) => restore_exported(payload)?,
    };
    Ok(Options {
        cohort,
        prefix: prefix.to_owned(),
        alarm,
        split: form.split(),
        starts,
        timetables: timetables.map(str::to_owned),
        exported,
        choices: Choices { half_promotions, groups, rooms: picked_rooms, times, sport },
    })
}

/// One line of the file list: `index`, `filename`, `status`, then whatever
/// the conversion learns about the file.
type Outcome = Map<String, Value>;

fn pending(index: usize, filename: &str) -> Outcome {
    let Value::Object(outcome) = json!({"index": index, "filename": filename, "status": "pending"}) else {
        unreachable!()
    };
    outcome
}

fn fail(outcome: &mut Outcome, code: &str, message: &str) {
    outcome.insert("status".to_owned(), json!("failed"));
    outcome.insert("error".to_owned(), json!({"code": code, "message": message}));
}

/// Rebuild the timetables a reviewer already holds, instead of re-uploading.
///
/// One message for every shape of malformed input: the reviewer's way out is
/// the same, and naming the bound that tripped would only help someone
/// probing for it.
fn restore_timetables(payload: &str, outcomes: &mut Vec<Outcome>) -> Result<Vec<(usize, Timetable)>, Refusal> {
    let restore = |outcomes: &mut Vec<Outcome>| -> Option<Vec<(usize, Timetable)>> {
        let items = serde_json::from_str::<Value>(payload).ok()?;
        let items = items.as_array().filter(|items| (1..=MAX_UPLOAD_FILES).contains(&items.len()))?;
        let mut restored = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let item = item.as_object()?;
            let timetable = Timetable::from_json(&clean_timetable(item.get("timetable"))?)?;
            let filename = match item.get("outcome") {
                None | Some(Value::Null) => None,
                Some(outcome) => match outcome.as_object()?.get("filename") {
                    Some(Value::String(name)) if !name.is_empty() => Some(name.clone()),
                    Some(name) if truthy(Some(name)) => return None,
                    _ => None,
                },
            };
            outcomes.push(pending(index, &safe_filename(&filename.unwrap_or_else(|| timetable.source.file.clone()))));
            restored.push((index, timetable));
        }
        Some(restored)
    };
    restore(outcomes).ok_or(Refusal {
        status: 400,
        detail: "La semaine transmise est illisible. Reprenez depuis le PDF.".to_owned(),
    })
}

/// The weeks a reader's device kept of earlier exports, held to the same
/// bounds as a week sent back for review: the page stored them, but nothing
/// says the page is what sends them.
fn restore_exported(payload: &str) -> Result<Vec<Timetable>, Refusal> {
    let restore = || -> Option<Vec<Timetable>> {
        let items = serde_json::from_str::<Value>(payload).ok()?;
        let items = items.as_array().filter(|items| items.len() <= MAX_UPLOAD_FILES)?;
        items.iter().map(|item| Timetable::from_json(&clean_timetable(Some(item))?)).collect()
    };
    restore()
        .ok_or(Refusal { status: 400, detail: "Les semaines gardées sur cet appareil sont illisibles.".to_owned() })
}

/// The files worth parsing, as `(index, file name to parse under, original)`.
///
/// A file that is not a PDF, or whose week cannot be dated, fails on its own
/// line; the others go on.
fn accept_uploads(
    files: &[Upload],
    options: &Options,
    outcomes: &mut Vec<Outcome>,
) -> Result<Vec<(usize, String, String)>, Refusal> {
    let mut uploads = Vec::new();
    let mut total_bytes = 0;
    for (index, file) in files.iter().enumerate() {
        let original = safe_filename(file.name.as_deref().unwrap_or(""));
        outcomes.push(pending(index, &original));
        total_bytes += file.bytes.len();
        if total_bytes > MAX_TOTAL_UPLOAD_BYTES {
            return refuse(413, "L'ensemble des PDF dépasse la limite de 4 Mo.");
        }
        let kind = file.content_type.as_deref().unwrap_or("");
        if !file.bytes.starts_with(b"%PDF-") || !matches!(kind, "application/pdf" | "application/octet-stream" | "") {
            fail(&mut outcomes[index], "NOT_PDF", "Ce fichier n'est pas un PDF.");
            continue;
        }
        let dated = match dates::week_bounds_from_filename(&original) {
            Err(error) => {
                fail(&mut outcomes[index], error.code, &error.message);
                continue;
            }
            Ok(bounds) => bounds.is_some(),
        };
        let filename = match options.starts[index].as_deref().filter(|start| !start.is_empty()) {
            _ if dated => Ok(original.clone()),
            Some(start) => dated_filename(start),
            None if files.len() > 1 => refuse(400, "Indiquez le lundi de cette semaine."),
            None => Ok(original.clone()),
        };
        match filename {
            Ok(filename) => uploads.push((index, filename, original)),
            Err(refusal) => fail(&mut outcomes[index], "WEEK_DATE_MISSING", &refusal.detail),
        }
    }
    Ok(uploads)
}

/// One PDF to one timetable: `worker.parse_bytes`.
fn parse_bytes(
    bytes: &[u8],
    filename: &str,
    cohorts: Option<&[String]>,
    reader: &mut Reader,
    now: i64,
) -> Result<Timetable, EdtError> {
    let source_name = pipeline::file_name(filename);
    if source_name.is_empty() {
        return Err(EdtError::unexpected());
    }
    let document = reader(bytes, source_name)?;
    pipeline::parse_document(&document, cohorts, now)
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word =
            chunk.iter().enumerate().fold(0u32, |word, (index, &byte)| word | u32::from(byte) << (16 - 8 * index));
        for position in 0..4 {
            if position <= chunk.len() {
                out.push(ALPHABET[(word >> (18 - 6 * position) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { crc >> 1 ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// A ZIP archive with its members stored as they are. Python deflates them;
/// calendars are small, and no compressor is worth shipping to the browser.
fn zip(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    // 1 January 1980, midnight: the same fixed date Python gives each member.
    const DATE: u16 = 0x0021;
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for (name, data) in members {
        let offset = out.len() as u32;
        let mut fields = Vec::new();
        fields.extend(20u16.to_le_bytes()); // version needed
        fields.extend(0x0800u16.to_le_bytes()); // names are UTF-8
        fields.extend(0u16.to_le_bytes()); // stored
        fields.extend(0u16.to_le_bytes()); // time
        fields.extend(DATE.to_le_bytes());
        fields.extend(crc32(data).to_le_bytes());
        fields.extend((data.len() as u32).to_le_bytes());
        fields.extend((data.len() as u32).to_le_bytes());
        fields.extend((name.len() as u16).to_le_bytes());
        fields.extend(0u16.to_le_bytes()); // extra
        out.extend(b"PK\x03\x04");
        out.extend(&fields);
        out.extend(name.as_bytes());
        out.extend(data);
        directory.extend(b"PK\x01\x02");
        directory.extend(20u16.to_le_bytes()); // version made by
        directory.extend(&fields);
        directory.extend([0; 10]); // comment, disk, attributes
        directory.extend(offset.to_le_bytes());
        directory.extend(name.as_bytes());
    }
    let start = out.len() as u32;
    out.extend(&directory);
    out.extend(b"PK\x05\x06\0\0\0\0");
    out.extend((members.len() as u16).to_le_bytes());
    out.extend((members.len() as u16).to_le_bytes());
    out.extend((directory.len() as u32).to_le_bytes());
    out.extend(start.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out
}

fn build_download(timetables: &[Timetable], options: &Options) -> Result<Value, EdtError> {
    let rendering = IcsOptions {
        prefix: text::head(&options.prefix, 60).to_owned(),
        alarm_minutes: options.alarm,
        split_by_module: options.split,
        ..IcsOptions::default()
    };
    let mut calendars = ics::to_calendars_after(timetables, &options.exported, &rendering)?;
    if !options.split {
        let (_, payload) = calendars.remove(0);
        return Ok(json!({
            "filename": if timetables.len() == 1 { "semaine-insp.ics" } else { "semaines-insp.ics" },
            "mime": "text/calendar;charset=utf-8",
            "base64": base64(&payload),
        }));
    }
    let members: Vec<(String, Vec<u8>)> =
        calendars.into_iter().map(|(slug, payload)| (format!("semaine-insp-{slug}.ics"), payload)).collect();
    Ok(json!({"filename": "calendriers-insp.zip", "mime": "application/zip", "base64": base64(&zip(&members))}))
}

fn combined_timetable(timetables: &[Timetable]) -> Result<Value, EdtError> {
    if let [only] = timetables {
        return Ok(only.to_json());
    }
    let mut entries = ics::merge(timetables)?;
    entries.sort_by_cached_key(|(event, _)| (event.date, event.sort_time(), event.uid()));
    let starts = timetables.iter().filter_map(|timetable| timetable.source.week_start);
    let ends = timetables.iter().filter_map(|timetable| timetable.source.week_end);
    let mut promotions: Vec<&str> = timetables
        .iter()
        .filter_map(|timetable| timetable.source.promotion.as_deref())
        .filter(|name| !name.is_empty())
        .collect();
    promotions.sort_unstable();
    promotions.dedup();
    let warnings: Vec<Value> = timetables
        .iter()
        .flat_map(|timetable| &timetable.warnings)
        .map(|warning| json!({"code": warning.code, "event_uid": warning.event_uid, "message": warning.message}))
        .collect();
    Ok(json!({
        "source": {
            "file": format!("{} fichiers PDF", timetables.len()),
            "files": timetables.iter().map(|timetable| timetable.source.file.as_str()).collect::<Vec<_>>(),
            "sha256": "",
            "week_start": starts.min().map(Date::iso),
            "week_end": ends.max().map(Date::iso),
            "promotion": if promotions.is_empty() { Value::Null } else { json!(promotions.join(", ")) },
            "revision": null,
            "parsed_at": null,
            "parser_version": PARSER_VERSION,
        },
        "events": entries.iter().map(|(event, _)| event.to_json()).collect::<Vec<_>>(),
        "warnings": warnings,
    }))
}

/// Everything after the parse: revisions, the review panels and the file.
fn respond(
    parsed: &[(usize, Timetable)],
    options: &Options,
    outcomes: &mut [Outcome],
    now: i64,
) -> Result<Value, Refusal> {
    let timetables: Vec<Timetable> = parsed.iter().map(|(_, timetable)| timetable.clone()).collect();
    let (selected, conflicts) = ics::resolve_revisions(&timetables);
    for (position, (index, timetable)) in parsed.iter().enumerate() {
        let outcome = &mut outcomes[*index];
        if conflicts.contains(&position) {
            fail(
                outcome,
                "REVISION_CONFLICT",
                "Versions incompatibles de la même semaine. Gardez uniquement celle à utiliser.",
            );
        } else if selected.contains(&position) {
            outcome.insert("status".to_owned(), json!("included"));
            outcome.insert("event_count".to_owned(), json!(timetable.events.len()));
        } else {
            outcome.insert("status".to_owned(), json!("superseded"));
        }
    }
    let selected: Vec<Timetable> = selected.into_iter().map(|position| timetables[position].clone()).collect();
    let events = || selected.iter().flat_map(|table| &table.events);
    let sessions = |wanted: &str| -> Vec<Value> {
        events().filter(|event| truthy(event.raw.get(wanted))).map(Event::to_json).collect()
    };
    let group_sessions: Vec<Value> = events()
        .filter(|event| keeps_half_promotion(event, &options.choices))
        .filter_map(|event| {
            let details = group_details(event);
            if details.is_empty() {
                return None;
            }
            let mut session = event.to_json();
            session["raw"]["group_details"] = json!(details);
            if let Some(rotation) = rotation_label(event) {
                session["raw"]["rotation"] = json!(rotation);
            }
            Some(session)
        })
        .collect();
    let lv1_sessions = sessions("lv1_room");
    // The Orient exchange and sessions for `Elèves inscrits` concern only some
    // students. Reuse the review's explicit include (empty string) /
    // __exclude__ decisions, keyed by UID. Keep the parsed timetable unchanged
    // until the student answers.
    let participation_sessions: Vec<Value> = events()
        .filter(|event| {
            crate::grid::fold(&event.title) == "moment d echange dgam eleves voie orient"
                || crate::events::is_registration_only(event)
        })
        .map(Event::to_json)
        .collect();
    let half_promotion_weeks: Vec<Value> = selected
        .iter()
        .filter_map(|table| {
            let sessions: Vec<Value> = table
                .events
                .iter()
                .filter(|event| event.raw.contains_key("half_promotion"))
                .map(Event::to_json)
                .collect();
            if sessions.is_empty() {
                return None;
            }
            Some(json!({"week_start": table.source.week_start.map(Date::iso), "sessions": sessions}))
        })
        .collect();
    // A schedule we filled in ourselves is offered for correction just like a
    // missing one: the reviewer knows the real slot, the default only guesses.
    let editable: Vec<String> = events()
        .filter(|event| {
            (event.end.is_none() || truthy(event.raw.get("default_times")))
                && event.last_date() == event.date
                && !crate::events::is_holiday(event)
        })
        .map(Event::uid)
        .collect();
    // One sport session a week is compulsory, so the weeks are kept apart.
    let sport_weeks: Vec<Value> = selected
        .iter()
        .map(|table| (table, table.events.iter().filter(|event| is_sport(event)).map(Event::to_json).collect::<Vec<_>>()))
        .filter(|(_, sessions)| sessions.len() > 1)
        .map(|(table, sessions)| {
            json!({"week_start": table.source.week_start.map(Date::iso), "file": table.source.file, "sessions": sessions})
        })
        .collect();
    let mut selected =
        select_groups(&selected, &options.choices, now).map_err(|detail| Refusal { status: 400, detail })?;
    // A week exported before goes out again as a revision of that export,
    // whatever the PDF's own date says: the clock is what puts it ahead of a
    // choice made last time, so a calendar takes its updates and tombstones.
    for table in &mut selected {
        if table.revised_at.is_none() && options.exported.iter().any(|earlier| ics::same_week(earlier, table)) {
            table.revised_at = Some(now);
        }
    }
    // The hours are offered on the reviewed sessions, so the panel shows what
    // the download holds — but keyed on the UID the choices travel under. A
    // sport slot is asked about only once it has been kept.
    let mut time_sessions = Vec::new();
    for event in selected.iter().flat_map(|table| &table.events) {
        let key = event.calendar_uid();
        let kept_sport = options.choices.sport.get(&key).and_then(Value::as_str) == Some("keep");
        if !editable.contains(&key) || (is_sport(event) && !kept_sport) {
            continue;
        }
        let mut session = event.to_json();
        session["uid"] = json!(key);
        time_sessions.push(session);
    }
    let downloadable = selected.iter().any(|table| !table.events.is_empty());
    // What the reviewer needs to come back with instead of the PDFs: only the
    // weeks actually included, each with its line of the file list.
    let reusable: Vec<Value> = parsed
        .iter()
        .filter(|(index, _)| outcomes[*index].get("status") == Some(&json!("included")))
        .map(|(index, timetable)| json!({"outcome": outcomes[*index], "timetable": timetable.to_json()}))
        .collect();
    // What the device keeps once the reader takes the file: each week as it
    // goes into the calendar, with the sessions this export cancels in it.
    let gone = ics::dropped(&selected, &options.exported);
    let exportable: Vec<Value> = selected
        .iter()
        .enumerate()
        .map(|(position, table)| {
            let cancelled: Vec<Value> =
                gone.iter().filter(|(winner, _)| *winner == position).map(|(_, event)| event.to_json()).collect();
            json!({"timetable": table.to_json(), "cancelled": cancelled})
        })
        .collect();
    let internal = |_: EdtError| Refusal { status: 500, detail: "Erreur interne du parseur.".to_owned() };
    Ok(json!({
        "parsed": reusable,
        "exportable": exportable,
        "group_sessions": group_sessions,
        "participation_sessions": participation_sessions,
        "half_promotion_weeks": half_promotion_weeks,
        "time_sessions": time_sessions,
        "sport_weeks": sport_weeks,
        "lv1_sessions": lv1_sessions,
        "lv1_rooms": INSTITUTION.lv1_rooms,
        "lv1_rotation_url": INSTITUTION.rotation_url,
        "timetable": if selected.is_empty() { Value::Null } else { combined_timetable(&selected).map_err(internal)? },
        "download": if downloadable { build_download(&selected, options).map_err(internal)? } else { Value::Null },
        "files": outcomes,
        "partial": outcomes.iter().any(|outcome| outcome.get("status") == Some(&json!("failed"))),
        "parser_version": PARSER_VERSION,
    }))
}

/// The HTTP status of a finished conversion; explains an empty one in place.
fn status_of(result: &mut Value, outcomes: &[Outcome], pdf_count: usize) -> u16 {
    if !result["timetable"].is_null() {
        return 200;
    }
    let error = outcomes
        .first()
        .and_then(|outcome| outcome.get("error").cloned())
        .unwrap_or_else(|| json!({"code": "NO_RESULT", "message": "Aucun résultat."}));
    let not_pdf = error["code"] == "NOT_PDF" && pdf_count == 1;
    result["detail"] = json!("Aucun PDF n'a pu être inclus.");
    result["error"] = error;
    if not_pdf {
        415
    } else {
        422
    }
}

fn answer(fields: &Value, files: &[Upload], reader: &mut Reader, now: i64) -> Result<(u16, Value), Refusal> {
    let Some(fields) = fields.as_object() else { return refuse(422, "Le formulaire est invalide.") };
    let options = read_options(&Form(fields), files.len())?;
    let mut outcomes = Vec::new();
    let parsed = if let Some(payload) = &options.timetables {
        restore_timetables(payload, &mut outcomes)?
    } else {
        let cohorts = (options.cohort != "ALL").then(|| vec![options.cohort.clone()]);
        let mut parsed = Vec::new();
        let mut unsupported = false;
        for (index, filename, original) in accept_uploads(files, &options, &mut outcomes)? {
            match parse_bytes(&files[index].bytes, &filename, cohorts.as_deref(), reader, now) {
                Err(error) => {
                    unsupported |= error.code == "UNSUPPORTED_PDF";
                    fail(&mut outcomes[index], error.code, &error.message);
                }
                // Through the pivot JSON, as the server has always done: the
                // week is sorted and identified again from what it kept.
                Ok(timetable) => match Timetable::from_json(&timetable.to_json()) {
                    Some(mut timetable) => {
                        timetable.source.file = original;
                        parsed.push((index, timetable));
                    }
                    None => fail(
                        &mut outcomes[index],
                        "WORKER_FAILED",
                        "L'analyseur n'a pas pu démarrer. Réessayez dans un instant.",
                    ),
                },
            }
        }
        // A document this reader was not written for is not answered with a
        // week short of a file: the page offers another engine instead.
        if unsupported {
            return Ok((0, json!({"engine_error": "UNSUPPORTED_PDF"})));
        }
        parsed
    };
    let mut result = respond(&parsed, &options, &mut outcomes, now)?;
    Ok((status_of(&mut result, &outcomes, files.len()), result))
}

/// Answer one request: the form fields as JSON and the uploaded files in,
/// `{"status", "body"}` as JSON out — what `web_support.browser.convert_js`
/// answers. `now` is the current time in seconds since 1970, which a
/// WebAssembly module has to be told.
pub fn convert(fields_json: &str, files: &[Upload], reader: &mut Reader, now: i64) -> String {
    let fields = serde_json::from_str(fields_json).unwrap_or(Value::Null);
    let (status, body) = match answer(&fields, files, reader, now) {
        Ok(answer) => answer,
        Err(refusal) => (refusal.status, json!({"detail": refusal.detail})),
    };
    json!({"status": status, "body": body}).to_string()
}

/// One PDF read on its own: `{"timetable": …}` or `{"error": …}`, the line
/// `edt_parser.worker` writes for it.
pub fn parse(bytes: &[u8], filename: &str, cohorts: Option<&[String]>, reader: &mut Reader, now: i64) -> Value {
    match parse_bytes(bytes, filename, cohorts, reader, now) {
        Ok(timetable) => json!({"timetable": timetable.to_json()}),
        Err(error) => json!({"error": {"code": error.code, "message": error.message}}),
    }
}

/// Several weeks, given as their pivot JSON, to their calendar files:
/// `ics.to_calendars_multi`. The request is `{"timetables": […], "prefix",
/// "alarm", "split"}`; the answer maps each file-name slug to its calendar,
/// or holds an `error`.
///
/// Unlike `convert`, this is handed every revision and not only the kept
/// ones, so it is where a session dropped by a correction gets its
/// `STATUS:CANCELLED`.
pub fn calendars(request: &Value) -> Value {
    let timetables: Option<Vec<Timetable>> =
        request["timetables"].as_array().and_then(|items| items.iter().map(Timetable::from_json).collect());
    let Some(timetables) = timetables else {
        return json!({"error": {"code": "UNEXPECTED", "message": "Ces semaines sont illisibles."}});
    };
    let options = IcsOptions {
        prefix: request["prefix"].as_str().unwrap_or_default().to_owned(),
        alarm_minutes: request["alarm"].as_i64(),
        split_by_module: request["split"].as_bool().unwrap_or(false),
        ..IcsOptions::default()
    };
    match ics::to_calendars_multi(&timetables, &options) {
        Err(error) => json!({"error": {"code": error.code, "message": error.message}}),
        Ok(calendars) => {
            let files: Map<String, Value> =
                calendars.into_iter().map(|(slug, payload)| (slug, json!(String::from_utf8_lossy(&payload)))).collect();
            json!({"calendars": files})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::tests::{document_of, page_of, HEADERS};

    const WEEK: &str = "Edt du 07.09.2026 au 11.09.2026.pdf";

    fn reader(_bytes: &[u8], file: &str) -> Result<Document, ExtractError> {
        let rows: [&[&str]; 3] = [
            &["07/09", "09h00", "10h00", "LV1 Anglais", "", "FI 2026-2028", "", "", "Langues"],
            &["08/09", "09h00", "10h00", "Droit", "M. Paul LEROY", "FI 2026-2028", "C101", "", "Droit"],
            &["09/09", "09h00", "10h00", "Gestion", "", "CIL 2026-2028", "C102", "", ""],
        ];
        Ok(document_of(file, vec![page_of(&HEADERS, &rows)]))
    }

    fn pdf(name: &str) -> Upload {
        Upload {
            name: Some(name.to_owned()),
            content_type: Some("application/pdf".to_owned()),
            bytes: b"%PDF-1.6".to_vec(),
        }
    }

    fn run(fields: Value, files: &[Upload]) -> (u64, Value) {
        let answer: Value =
            serde_json::from_str(&convert(&fields.to_string(), files, &mut reader, 1_788_000_000)).unwrap();
        (answer["status"].as_u64().unwrap(), answer["body"].clone())
    }

    #[test]
    fn a_week_is_converted_to_one_calendar() {
        let (status, body) = run(json!({"cohort": "fi", "alarm": "15", "prefix": "INSP "}), &[pdf(WEEK)]);
        assert_eq!(status, 200);
        assert_eq!(body["files"], json!([{"index": 0, "filename": WEEK, "status": "included", "event_count": 2}]));
        assert_eq!(body["download"]["filename"], "semaine-insp.ics");
        assert_eq!(body["lv1_sessions"].as_array().unwrap().len(), 1);
        assert_eq!(body["parsed"][0]["timetable"], body["timetable"]);
        assert_eq!(body["partial"], false);
    }

    #[test]
    fn supplied_anr_groups_narrow_only_the_matching_dated_session() {
        let mut reader = |_: &[u8], file: &str| {
            let rows: [&[&str]; 4] = [
                &["20/10", "13h00", "18h00", "Expérimentation ANR", "Mme Alice EXEMPLE", "FI", "C301", "", "Science"],
                &["21/10", "13h00", "18h00", "Expérimentation ANR", "", "FI", "C302", "", "Science"],
                &["20/10", "14h00", "18h00", "Expérimentation ANR", "", "FI", "C303", "", "Science"],
                &["20/10", "13h00", "18h00", "Travail en autonomie", "", "FI", "", "", "Science"],
            ];
            Ok(document_of(file, vec![page_of(&HEADERS, &rows)]))
        };
        let first: Value = serde_json::from_str(&convert(
            "{}",
            &[pdf("Edt du 19.10.2026 au 23.10.2026.pdf")],
            &mut reader,
            1_788_000_000,
        ))
        .unwrap();
        let body = &first["body"];
        assert_eq!(body["group_sessions"].as_array().unwrap().len(), 1);
        let session = &body["group_sessions"][0];
        assert_eq!(session["raw"]["group_details"].as_array().unwrap().len(), 5);
        let uid = session["uid"].as_str().unwrap();
        let original = body["parsed"][0]["timetable"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["uid"] == uid)
            .unwrap();
        assert!(original["raw"].get("group_details").is_none());
        assert_eq!(original["start"], "13:00");
        assert_eq!(original["end"], "18:00");
        for (index, letter) in ["A", "B", "C", "D", "E"].iter().enumerate() {
            let label = format!("Groupe {letter}");
            let (status, reviewed) = run(
                json!({"timetables": body["parsed"].to_string(), "group_choices": json!({uid: label}).to_string()}),
                &[],
            );
            assert_eq!(status, 200);
            let events = reviewed["timetable"]["events"].as_array().unwrap();
            assert_eq!(events.len(), 4);
            let event = events.iter().find(|event| event["cohort_label"] == label).unwrap();
            assert_eq!(event["start"], format!("{}:00", 13 + index));
            assert_eq!(event["end"], format!("{}:00", 14 + index));
            assert_eq!(event["location"], "C301");
            assert_eq!(event["speakers"], json!(["Mme Alice EXEMPLE"]));
            assert_eq!(event["raw"]["choice_uid"], uid);
            assert_eq!(event["cohorts"], json!(["FI"]));
        }
        assert_eq!(run(json!({"timetables": body["parsed"].to_string(), "group_choices": json!({uid: "Groupe F"}).to_string()}), &[]).0, 400);
    }

    #[test]
    fn fitness_groups_are_one_sport_choice_with_the_printed_hours() {
        let mut reader = |_: &[u8], file: &str| {
            let rows: [&[&str]; 2] = [
                &[
                    "21/10",
                    "17h45",
                    "20h15",
                    "Diagnostic condition physique Groupe 1",
                    "",
                    "FI",
                    "Gymnase",
                    "",
                    "Sport",
                ],
                &[
                    "22/10",
                    "17h45",
                    "20h15",
                    "Diagnostic de condition physique Groupe 2",
                    "",
                    "FI",
                    "Gymnase",
                    "",
                    "Sport",
                ],
            ];
            Ok(document_of(file, vec![page_of(&HEADERS, &rows)]))
        };
        let first: Value = serde_json::from_str(&convert(
            "{}",
            &[pdf("Edt du 19.10.2026 au 23.10.2026.pdf")],
            &mut reader,
            1_788_000_000,
        ))
        .unwrap();
        let body = &first["body"];
        let sessions = body["sport_weeks"][0]["sessions"].as_array().unwrap();
        assert_eq!(sessions.len(), 2);
        for session in sessions {
            let uid = session["uid"].as_str().unwrap();
            // Keeping one alone must also discard the other in the engine.
            let (status, reviewed) = run(
                json!({"timetables": body["parsed"].to_string(),
                "sport_choices": json!({uid: "keep"}).to_string()}),
                &[],
            );
            assert_eq!(status, 200);
            let events = reviewed["timetable"]["events"].as_array().unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0]["uid"], uid);
            assert_eq!(events[0]["start"], "17:45");
            assert_eq!(events[0]["end"], "20:15");
            assert!(reviewed["time_sessions"].as_array().unwrap().is_empty());
            assert!(reviewed["timetable"]["warnings"].as_array().unwrap().iter().all(|w| w["code"] != "SPORT_CHOICE"));
        }
        let choices: Map<String, Value> =
            sessions.iter().map(|session| (session["uid"].as_str().unwrap().to_owned(), json!("keep"))).collect();
        assert_eq!(run(json!({"timetables": body["parsed"].to_string(), "sport_choices": serde_json::to_string(&choices).unwrap()}), &[]).0, 400);
    }

    #[test]
    fn a_review_needs_no_pdf_and_keeps_the_calendar_uid() {
        let (_, first) = run(json!({}), &[pdf(WEEK)]);
        let lv1 = first["lv1_sessions"][0]["uid"].as_str().unwrap();
        let fields =
            json!({"timetables": first["parsed"].to_string(), "room_choices": json!({lv1: "Z999"}).to_string()});
        let (status, second) = run(fields, &[]);
        assert_eq!(status, 200);
        let event = &second["timetable"]["events"][0];
        assert_eq!(event["location"], "Z999");
        assert_eq!(event["uid"], lv1);
        let codes: Vec<&str> =
            second["timetable"]["warnings"].as_array().unwrap().iter().map(|w| w["code"].as_str().unwrap()).collect();
        assert!(codes.contains(&"UNKNOWN_ROOM") && !codes.contains(&"ROOM_ROTATION"));
        assert!(second["lv1_sessions"].as_array().unwrap().len() == 1);
    }

    #[test]
    fn refusals_carry_their_status() {
        assert_eq!(run(json!({}), &[]), (400, json!({"detail": "Aucun PDF reçu."})));
        assert_eq!(run(json!({"cohort": "XYZ"}), &[pdf(WEEK)]).0, 400);
        assert_eq!(run(json!({"alarm": "5000"}), &[pdf(WEEK)]).0, 400);
        assert_eq!(run(json!({"alarm": "bientôt"}), &[pdf(WEEK)]).0, 422);
        assert_eq!(
            run(json!({"timetables": "[]"}), &[]).1["detail"],
            "La semaine transmise est illisible. Reprenez depuis le PDF."
        );
        assert_eq!(
            run(json!({"group_choices": "[]"}), &[pdf(WEEK)]).1["detail"],
            "Les choix de groupes sont invalides."
        );
        assert_eq!(run(json!({"group_choices": "{\"edt-insp-0\": \"Groupe 1\"}"}), &[pdf(WEEK)]).0, 400);
        let nine: Vec<Upload> = (0..9).map(|_| pdf(WEEK)).collect();
        assert_eq!(run(json!({}), &nine).0, 413);
    }

    #[test]
    fn a_file_that_is_no_pdf_fails_on_its_own_line() {
        let text = Upload { name: Some("notes.pdf".to_owned()), content_type: None, bytes: b"bonjour".to_vec() };
        let (status, body) = run(json!({}), std::slice::from_ref(&text));
        assert_eq!(status, 415);
        assert_eq!(body["error"]["code"], "NOT_PDF");
        let (status, body) = run(json!({}), &[text, pdf(WEEK), pdf("planning.pdf")]);
        assert_eq!(status, 200);
        let statuses: Vec<&str> =
            body["files"].as_array().unwrap().iter().map(|file| file["status"].as_str().unwrap()).collect();
        assert_eq!(statuses, ["failed", "included", "failed"]);
        assert_eq!(body["files"][2]["error"]["code"], "WEEK_DATE_MISSING");
        assert_eq!(body["partial"], true);
    }

    #[test]
    fn an_undated_file_takes_the_monday_it_is_given() {
        let (status, body) = run(json!({"week_start": "2026-09-07"}), &[pdf("planning.pdf")]);
        assert_eq!(status, 200);
        assert_eq!(body["timetable"]["source"]["file"], "planning.pdf");
        assert_eq!(body["timetable"]["source"]["week_end"], "2026-09-11");
        assert_eq!(
            run(json!({"week_start": "2026-09-08"}), &[pdf("planning.pdf")]).1["error"]["code"],
            "WEEK_DATE_MISSING"
        );
    }

    #[test]
    fn an_unsupported_pdf_hands_the_request_back() {
        let mut refusing = |_: &[u8], _: &str| Err(ExtractError::unsupported("texte tourné"));
        let answer: Value = serde_json::from_str(&convert("{}", &[pdf(WEEK)], &mut refusing, 0)).unwrap();
        assert_eq!(answer, json!({"status": 0, "body": {"engine_error": "UNSUPPORTED_PDF"}}));
    }

    #[test]
    fn modules_are_split_into_an_archive() {
        let (_, body) = run(json!({"split_by_module": "true"}), &[pdf(WEEK)]);
        assert_eq!(body["download"]["filename"], "calendriers-insp.zip");
        assert!(body["download"]["base64"].as_str().unwrap().starts_with("UEsDB"));
    }

    #[test]
    fn a_session_the_correction_dropped_is_cancelled() {
        let week = |revision: &str, rows: &[&[&str]]| {
            let mut document = document_of(WEEK, vec![page_of(&HEADERS, rows)]);
            document.metadata.insert("ModDate".to_owned(), revision.to_owned());
            pipeline::parse_document(&document, None, 0).unwrap().to_json()
        };
        let droit: &[&str] = &["08/09", "09h00", "10h00", "Droit", "", "FI 2026-2028", "C101", "", ""];
        let gestion: &[&str] = &["09/09", "09h00", "10h00", "Gestion", "", "FI 2026-2028", "C102", "", ""];
        let before = week("D:20260901100000Z", &[droit, gestion]);
        let after = week("D:20260902100000Z", &[droit]);
        let answer = calendars(&json!({"timetables": [before, after]}));
        let calendar = answer["calendars"][""].as_str().unwrap();
        let events: Vec<&str> = calendar.split("BEGIN:VEVENT").skip(1).collect();
        assert_eq!(events.len(), 2);
        assert!(events[0].contains("SUMMARY:Droit\r\n") && events[0].contains("STATUS:CONFIRMED\r\n"));
        assert!(events[1].contains("SUMMARY:Gestion\r\n") && events[1].contains("STATUS:CANCELLED\r\n"));
        // The tombstone carries the kept week's revision, so a calendar takes it.
        assert!(events.iter().all(|event| event.contains("DTSTAMP:20260902T100000Z\r\n")));

        let conflict = calendars(&json!({"timetables": [week("", &[droit, gestion]), week("", &[droit])]}));
        assert_eq!(conflict["error"]["code"], "REVISION_CONFLICT");
    }

    #[test]
    fn a_session_missing_since_the_last_export_is_cancelled() {
        // The device kept the week as it was exported: three sessions.
        let (_, before) = run(json!({}), &[pdf(WEEK)]);
        let kept = json!([before["exportable"][0]["timetable"]]).to_string();
        assert_eq!(before["exportable"][0]["cancelled"], json!([]));
        // The same week comes back without « Gestion ».
        let mut corrected = before["parsed"].clone();
        let events = corrected[0]["timetable"]["events"].as_array_mut().unwrap();
        events.retain(|event| event["title"] != "Gestion");
        let fields = json!({"timetables": corrected.to_string(), "exported": kept});
        let (status, after) = run(fields, &[]);
        assert_eq!(status, 200);
        let cancelled = after["exportable"][0]["cancelled"].as_array().unwrap();
        assert_eq!(cancelled.len(), 1);
        assert_eq!(cancelled[0]["title"], "Gestion");
        assert_eq!(after["timetable"]["events"].as_array().unwrap().len(), 2);
        let calendar = decode(after["download"]["base64"].as_str().unwrap());
        let events: Vec<&str> = calendar.split("BEGIN:VEVENT").skip(1).collect();
        assert_eq!(events.len(), 3);
        let gestion = events.iter().find(|event| event.contains("SUMMARY:Gestion\r\n")).unwrap();
        assert!(gestion.contains("STATUS:CANCELLED\r\n"));
        assert_eq!(events.iter().filter(|event| event.contains("STATUS:CONFIRMED\r\n")).count(), 2);
        // Exported again, the week outranks its first export.
        let sequence = |calendar: &str| -> i64 {
            let line = calendar.lines().find(|line| line.starts_with("SEQUENCE:")).unwrap();
            line["SEQUENCE:".len()..].trim().parse().unwrap()
        };
        assert!(sequence(&calendar) > sequence(&decode(before["download"]["base64"].as_str().unwrap())));
        // A week of another name cancels nothing, and a malformed one is refused.
        let mut other = before["exportable"][0]["timetable"].clone();
        other["source"]["week_start"] = json!("2026-09-14");
        other["source"]["week_end"] = json!("2026-09-18");
        let fields = json!({"timetables": corrected.to_string(), "exported": json!([other]).to_string()});
        assert_eq!(run(fields, &[]).1["exportable"][0]["cancelled"], json!([]));
        let fields = json!({"timetables": corrected.to_string(), "exported": "[{}]"});
        assert_eq!(run(fields, &[]).0, 400);
    }

    fn decode(encoded: &str) -> String {
        const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut bytes = Vec::new();
        let (mut word, mut bits) = (0u32, 0);
        for symbol in encoded.bytes().filter(|&symbol| symbol != b'=') {
            word = word << 6 | ALPHABET.iter().position(|&known| known == symbol).unwrap() as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                bytes.push((word >> bits) as u8);
            }
        }
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn base64_and_crc_match_the_standards() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
