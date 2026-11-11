//! The pivot data model: `src/edt_parser/model.py`.
//!
//! `Timetable::to_json` produces the JSON the goldens hold, and
//! `Timetable::from_json` reads it back for a review that sends no PDF.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};

use crate::errors::warning;
use crate::py::civil::{Date, Time};
use crate::py::text;

pub const UID_PREFIX: &str = "edt-insp-";

/// The longest `Du … au …` stretch a single session may cover.
pub const MAX_SPAN_DAYS: i64 = 31;

/// Columns of the INSP timetable, in reading order.
pub const COLUMN_ROLES: [&str; 9] =
    ["date", "start", "end", "theme", "speakers", "cohort", "location", "preparation", "module"];

/// Fold a title to a stable form, used for UID hashing and comparisons.
pub fn normalise_title(title: &str) -> String {
    let folded = text::unaccent(title).replace(['’', '‘'], "'");
    text::casefold(&text::single_spaced(&folded))
}

/// `normalise_title`, minus a trailing parenthetical: the school clarifying a
/// session between two revisions must not turn it into a new one.
pub fn identity_title(title: &str) -> String {
    static SUFFIX: LazyLock<Regex> = LazyLock::new(|| text::regex(r"\s*\([^()]*\)$"));
    let normalised = normalise_title(title);
    let shortened = text::strip(&SUFFIX.replace(&normalised, "")).to_owned();
    if shortened.is_empty() {
        normalised
    } else {
        shortened
    }
}

fn uid_of(payload: &str) -> String {
    let digest = Sha1::digest(payload.as_bytes());
    let hex: String = digest.iter().take(8).map(|byte| format!("{byte:02x}")).collect();
    format!("{UID_PREFIX}{hex}")
}

/// Python's truth value of a JSON value held in `raw`.
pub fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => number.as_f64() != Some(0.0),
        Some(Value::String(string)) => !string.is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Object(fields)) => !fields.is_empty(),
    }
}

/// A PDF link annotation attached to an event.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub url: String,
    pub label: Option<String>,
    pub column: Option<String>,
}

/// Sub-group information: flagged, not split.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Groups {
    pub multi: bool,
    pub count: i64,
    pub time_ranges: Vec<String>,
}

/// A non-blocking issue.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub code: &'static str,
    pub message: String,
    pub event_uid: Option<String>,
}

/// One session or sequential slot anchored on a non-empty `Thème` cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub date: Date,
    pub start: Option<Time>,
    pub end: Option<Time>,
    pub title: String,
    pub description: Option<String>,
    pub module: Option<String>,
    pub module_color: Option<String>,
    pub cohorts: Vec<String>,
    pub cohort_label: Option<String>,
    pub groups: Groups,
    pub speakers: Vec<String>,
    pub location: Option<String>,
    pub location_note: Option<String>,
    pub preparation: Option<String>,
    pub notes: Vec<String>,
    pub links: Vec<Link>,
    pub remote: bool,
    pub raw: Map<String, Value>,
    /// The week this event belongs to, stamped by `Timetable::assign_identities`.
    pub week_start: Option<Date>,
    /// Which session of its (title, cohort) group this is within the week,
    /// from 1. Stamped once: dropping another session must not renumber it.
    pub occurrence: u32,
}

impl Event {
    /// The last day a session printed over several days (`Du … au …`) covers.
    pub fn last_date(&self) -> Date {
        let until = self.raw.get("until").and_then(Value::as_str).and_then(Date::parse_iso);
        match until {
            Some(last) if self.date < last && last.days() <= self.date.days() + MAX_SPAN_DAYS => last,
            _ => self.date,
        }
    }

    pub fn sort_time(&self) -> Time {
        self.start.unwrap_or(Time::MIDNIGHT)
    }

    /// Stable across a revision that moves this session: the week, the title,
    /// the cohort and the occurrence, never the day or the hours.
    pub fn uid(&self) -> String {
        let anchor = self.week_start.unwrap_or(self.date);
        let cohort = self.cohort_label.as_deref().unwrap_or("");
        uid_of(&format!("{anchor}|{}|{cohort}|{}", identity_title(&self.title), self.occurrence))
    }

    /// The content hash UIDs used before the week-anchored identity.
    pub fn legacy_uid(&self) -> String {
        let hour = |time: Option<Time>| time.map(Time::colon).unwrap_or_default();
        let cohort = self.cohort_label.as_deref().unwrap_or("");
        uid_of(&format!(
            "{}|{}|{}|{}|{cohort}",
            self.date,
            hour(self.start),
            hour(self.end),
            normalise_title(&self.title)
        ))
    }

    /// The UID written into the ICS: the one the session had before any choice.
    pub fn calendar_uid(&self) -> String {
        match self.raw.get("choice_uid").and_then(Value::as_str) {
            Some(uid) if !uid.is_empty() => uid.to_owned(),
            _ => self.uid(),
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "uid": self.uid(),
            "date": self.date.iso(),
            "start": self.start.map(Time::colon),
            "end": self.end.map(Time::colon),
            "title": self.title,
            "description": self.description,
            "module": self.module,
            "module_color": self.module_color,
            "cohorts": self.cohorts,
            "cohort_label": self.cohort_label,
            "groups": {
                "multi": self.groups.multi,
                "count": self.groups.count,
                "time_ranges": self.groups.time_ranges,
            },
            "speakers": self.speakers,
            "location": self.location,
            "location_note": self.location_note,
            "preparation": self.preparation,
            "notes": self.notes,
            "links": self.links.iter().map(|link| json!({
                "label": link.label, "url": link.url, "column": link.column,
            })).collect::<Vec<_>>(),
            "remote": self.remote,
            "raw": self.raw,
        })
    }
}

/// Provenance of a parse run.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Source {
    pub file: String,
    pub sha256: String,
    pub week_start: Option<Date>,
    pub week_end: Option<Date>,
    pub promotion: Option<String>,
    /// When the PDF itself was last written (ISO 8601).
    pub revision: Option<String>,
    pub parsed_at: Option<String>,
    pub parser_version: Option<String>,
}

impl Source {
    pub fn to_json(&self) -> Value {
        json!({
            "file": self.file,
            "sha256": self.sha256,
            "week_start": self.week_start.map(Date::iso),
            "week_end": self.week_end.map(Date::iso),
            "promotion": self.promotion,
            "revision": self.revision,
            "parsed_at": self.parsed_at,
            "parser_version": self.parser_version,
        })
    }
}

/// A parsed PDF: its provenance, its events and the warnings raised.
#[derive(Debug, Clone, PartialEq)]
pub struct Timetable {
    pub source: Source,
    pub events: Vec<Event>,
    pub warnings: Vec<Warning>,
    /// When the reviewer's choices were written in, if they were. Outside the
    /// pivot JSON: that describes the PDF, not one download of it.
    pub revised_at: Option<i64>,
}

impl Timetable {
    /// Events are put in the day's order, not the grid's, and identified.
    pub fn new(source: Source, mut events: Vec<Event>, warnings: Vec<Warning>) -> Self {
        events.sort_by_key(|event| (event.date, event.sort_time()));
        let mut timetable = Self { source, events, warnings, revised_at: None };
        timetable.assign_identities();
        timetable
    }

    /// Stamp every event with the week it belongs to and its occurrence within
    /// its (title, cohort) group, in an order that does not depend on the grid.
    pub fn assign_identities(&mut self) {
        let mut order: Vec<usize> = (0..self.events.len()).collect();
        let key = |event: &Event| {
            (
                event.date,
                event.sort_time(),
                normalise_title(&event.title),
                event.cohort_label.clone().unwrap_or_default(),
                event.location.clone().unwrap_or_default(),
                event.speakers.clone(),
            )
        };
        order.sort_by_cached_key(|&index| key(&self.events[index]));
        let mut counts: Vec<((String, String), u32)> = Vec::new();
        for index in order {
            let event = &mut self.events[index];
            let group = (identity_title(&event.title), event.cohort_label.clone().unwrap_or_default());
            let position = counts.iter().position(|(known, _)| *known == group).unwrap_or_else(|| {
                counts.push((group, 0));
                counts.len() - 1
            });
            counts[position].1 += 1;
            event.week_start = self.source.week_start;
            event.occurrence = counts[position].1;
        }
    }

    pub fn warn(&mut self, code: &'static str, message: String, event_uid: Option<String>) {
        self.warnings.push(Warning { code, message, event_uid });
    }

    pub fn to_json(&self) -> Value {
        json!({
            "source": self.source.to_json(),
            "events": self.events.iter().map(Event::to_json).collect::<Vec<_>>(),
            "warnings": self.warnings.iter().map(|warning| json!({
                "code": warning.code, "event_uid": warning.event_uid, "message": warning.message,
            })).collect::<Vec<_>>(),
        })
    }

    /// Restore the JSON `to_json` writes. `None` for anything else: unlike
    /// Python's `from_dict`, every field is held to its type here, so a
    /// payload this program could not have written is refused before any of
    /// it reaches a calendar.
    pub fn from_json(value: &Value) -> Option<Self> {
        let fields = value.as_object()?;
        let source = source_from(fields.get("source")?)?;
        let events = fields.get("events")?.as_array()?.iter().map(event_from).collect::<Option<Vec<_>>>()?;
        let warnings = match fields.get("warnings") {
            None => Vec::new(),
            Some(items) => items.as_array()?.iter().map(warning_from).collect::<Option<Vec<_>>>()?,
        };
        Some(Self::new(source, events, warnings))
    }
}

fn optional_string(value: Option<&Value>) -> Option<Option<String>> {
    match value {
        None | Some(Value::Null) => Some(None),
        Some(Value::String(string)) => Some(Some(string.clone())),
        Some(_) => None,
    }
}

fn string_list(value: Option<&Value>) -> Option<Vec<String>> {
    match value {
        None => Some(Vec::new()),
        Some(items) => items.as_array()?.iter().map(|item| item.as_str().map(str::to_owned)).collect(),
    }
}

fn only_keys(fields: &Map<String, Value>, allowed: &[&str]) -> Option<()> {
    fields.keys().all(|key| allowed.contains(&key.as_str())).then_some(())
}

fn optional_date(value: Option<&Value>) -> Option<Option<Date>> {
    match optional_string(value)? {
        Some(text) if !text.is_empty() => Date::parse_iso(&text).map(Some),
        _ => Some(None),
    }
}

fn source_from(value: &Value) -> Option<Source> {
    let fields = value.as_object()?;
    only_keys(
        fields,
        &["file", "sha256", "week_start", "week_end", "promotion", "revision", "parsed_at", "parser_version"],
    )?;
    Some(Source {
        file: fields.get("file")?.as_str()?.to_owned(),
        sha256: fields.get("sha256")?.as_str()?.to_owned(),
        week_start: optional_date(fields.get("week_start"))?,
        week_end: optional_date(fields.get("week_end"))?,
        promotion: optional_string(fields.get("promotion"))?,
        revision: optional_string(fields.get("revision"))?,
        parsed_at: optional_string(fields.get("parsed_at"))?,
        parser_version: optional_string(fields.get("parser_version"))?,
    })
}

fn optional_time(value: &Value) -> Option<Option<Time>> {
    match value {
        Value::Null => Some(None),
        Value::String(text) if text.is_empty() => Some(None),
        Value::String(text) => Time::parse_colon(text).map(Some),
        _ => None,
    }
}

/// The types of the `raw` entries later stages read. Which keys may be present
/// is `review::clean_timetable`'s business; what they may hold is settled here.
fn raw_from(value: Option<&Value>) -> Option<Map<String, Value>> {
    let Some(value) = value else { return Some(Map::new()) };
    let raw = value.as_object()?;
    let strings = |value: &Value| value.as_array().is_some_and(|items| items.iter().all(Value::is_string));
    for (key, item) in raw {
        let sound = match key.as_str() {
            "choice_uid" | "schedule_label" | "until" => item.is_string(),
            "original_schedule_label" => item.is_string() || item.is_null(),
            "default_times" | "lv1_room" | "pdf_start" | "user_times" => item.is_boolean(),
            "page" => item.is_u64(),
            "bbox" => item.as_array().is_some_and(|items| items.iter().all(Value::is_number)),
            "group_labels" | "group_speakers" | "red_notes" | "rooms" | "time_ranges" => strings(item),
            "group_details" => item.as_array().is_some_and(|groups| {
                groups.iter().all(|group| group.as_object().is_some_and(|fields| fields.values().all(Value::is_string)))
            }),
            _ => true,
        };
        if !sound {
            return None;
        }
    }
    Some(raw.clone())
}

fn event_from(value: &Value) -> Option<Event> {
    let fields = value.as_object()?;
    only_keys(
        fields,
        &[
            // Derived, and recomputed from the source the timetable is given.
            "uid",
            "owner",
            "week_start",
            "occurrence",
            "date",
            "start",
            "end",
            "title",
            "description",
            "module",
            "module_color",
            "cohorts",
            "cohort_label",
            "groups",
            "speakers",
            "location",
            "location_note",
            "preparation",
            "notes",
            "links",
            "remote",
            "raw",
        ],
    )?;
    let groups = match fields.get("groups") {
        None => Groups::default(),
        Some(value) => {
            let groups = value.as_object()?;
            only_keys(groups, &["multi", "count", "time_ranges"])?;
            Groups {
                multi: groups.get("multi").map_or(Some(false), Value::as_bool)?,
                count: groups.get("count").map_or(Some(0), Value::as_i64)?,
                time_ranges: string_list(groups.get("time_ranges"))?,
            }
        }
    };
    let links = match fields.get("links") {
        None => Vec::new(),
        Some(items) => items
            .as_array()?
            .iter()
            .map(|item| {
                let link = item.as_object()?;
                only_keys(link, &["url", "label", "column"])?;
                Some(Link {
                    url: link.get("url")?.as_str()?.to_owned(),
                    label: optional_string(link.get("label"))?,
                    column: optional_string(link.get("column"))?,
                })
            })
            .collect::<Option<Vec<_>>>()?,
    };
    Some(Event {
        date: Date::parse_iso(fields.get("date")?.as_str()?)?,
        start: optional_time(fields.get("start")?)?,
        end: optional_time(fields.get("end")?)?,
        title: fields.get("title")?.as_str()?.to_owned(),
        description: optional_string(fields.get("description"))?,
        module: optional_string(fields.get("module"))?,
        module_color: optional_string(fields.get("module_color"))?,
        cohorts: string_list(fields.get("cohorts"))?,
        cohort_label: optional_string(fields.get("cohort_label"))?,
        groups,
        speakers: string_list(fields.get("speakers"))?,
        location: optional_string(fields.get("location"))?,
        location_note: optional_string(fields.get("location_note"))?,
        preparation: optional_string(fields.get("preparation"))?,
        notes: string_list(fields.get("notes"))?,
        links,
        remote: fields.get("remote").map_or(Some(false), Value::as_bool)?,
        raw: raw_from(fields.get("raw"))?,
        week_start: None,
        occurrence: 1,
    })
}

fn warning_from(value: &Value) -> Option<Warning> {
    let fields = value.as_object()?;
    Some(Warning {
        code: warning::known(fields.get("code")?.as_str()?)?,
        message: fields.get("message")?.as_str()?.to_owned(),
        event_uid: optional_string(fields.get("event_uid"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn event(day: u32, title: &str) -> Event {
        Event {
            date: Date::new(2026, 9, i64::from(day)).unwrap(),
            start: Time::new(9, 0),
            end: Time::new(10, 0),
            title: title.to_owned(),
            description: None,
            module: None,
            module_color: None,
            cohorts: vec!["FI".to_owned()],
            cohort_label: Some("FI 2026-2028".to_owned()),
            groups: Groups::default(),
            speakers: Vec::new(),
            location: None,
            location_note: None,
            preparation: None,
            notes: Vec::new(),
            links: Vec::new(),
            remote: false,
            raw: Map::new(),
            week_start: None,
            occurrence: 1,
        }
    }

    fn week() -> Source {
        Source { file: "w.pdf".to_owned(), week_start: Date::new(2026, 9, 7), ..Source::default() }
    }

    #[test]
    fn titles_fold_like_python() {
        assert_eq!(normalise_title("  L’État  et\nla  DÉONTOLOGIE "), "l'etat et la deontologie");
        assert_eq!(identity_title("Tronc commun (Séquences 0.1 et 0.2)"), "tronc commun");
        assert_eq!(identity_title("(Séquence 1)"), "(sequence 1)");
    }

    #[test]
    fn uids_are_the_ones_python_computes() {
        // hashlib.sha1("2026-09-07|ateliers de deontologie|FI 2026-2028|1".encode()).hexdigest()[:16]
        let timetable = Timetable::new(week(), vec![event(8, "Ateliers de déontologie")], Vec::new());
        assert_eq!(timetable.events[0].uid(), "edt-insp-07b1f41ad0ef10fd");
    }

    #[test]
    fn a_moved_session_keeps_its_uid_and_twins_are_numbered() {
        let before = Timetable::new(week(), vec![event(8, "Atelier"), event(9, "Atelier")], Vec::new());
        let mut moved = event(8, "Atelier");
        moved.date = Date::new(2026, 9, 11).unwrap();
        let after = Timetable::new(week(), vec![moved, event(9, "Atelier")], Vec::new());
        assert_eq!(before.events.iter().map(|e| e.occurrence).collect::<Vec<_>>(), [1, 2]);
        let uids = |timetable: &Timetable| {
            let mut uids: Vec<String> = timetable.events.iter().map(Event::uid).collect();
            uids.sort();
            uids
        };
        assert_eq!(uids(&before), uids(&after));
    }

    #[test]
    fn the_pivot_json_round_trips() {
        let mut first = event(8, "Atelier");
        first.raw.insert("page".to_owned(), json!(1));
        first.raw.insert("until".to_owned(), json!("2026-09-10"));
        first.links.push(Link { url: "https://example.org".to_owned(), label: None, column: Some("theme".to_owned()) });
        let timetable = Timetable::new(week(), vec![first], Vec::new());
        assert_eq!(timetable.events[0].last_date(), Date::new(2026, 9, 10).unwrap());
        let restored = Timetable::from_json(&timetable.to_json()).unwrap();
        assert_eq!(restored, timetable);
    }

    #[test]
    fn a_payload_of_the_wrong_shape_is_refused() {
        let good = Timetable::new(week(), vec![event(8, "Atelier")], Vec::new()).to_json();
        let mut wrong = good.clone();
        wrong["events"][0]["title"] = json!(12);
        assert!(Timetable::from_json(&wrong).is_none());
        let mut wrong = good.clone();
        wrong["events"][0]["surprise"] = json!(true);
        assert!(Timetable::from_json(&wrong).is_none());
        let mut wrong = good.clone();
        wrong["events"][0]["raw"]["schedule_label"] = json!(["x"]);
        assert!(Timetable::from_json(&wrong).is_none());
        let mut wrong = good;
        wrong["warnings"] = json!([{"code": "MADE_UP", "message": "x"}]);
        assert!(Timetable::from_json(&wrong).is_none());
    }
}
