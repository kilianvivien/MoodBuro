//! Bounds on the one payload this engine did not produce itself:
//! `web_support/review.py`.
//!
//! Applying a group, a room, a sport slot or a missing hour needs no PDF: the
//! reviewer sends back the week the previous answer handed them. The shape is
//! checked before anything is built — sizes first, then a whitelist of the
//! `raw` keys later stages read. `Timetable::from_json` then holds every field
//! to its type.

use serde_json::{Map, Value};

use crate::py::text;

/// Ten times the largest real week.
const MAX_RESTORED_EVENTS: usize = 200;
const MAX_LINKS_PER_EVENT: usize = 20;
/// Ten times what the busiest real week raises. Python stops at 30, which
/// that week — 14 September, 44 warnings, most of them parallel group
/// sessions overlapping — exceeds: none of its choices could be applied.
/// The count grows with the pairs of sessions, not with the sessions, so the
/// bound is set well clear of it; the string cap still bounds each one.
const MAX_WARNINGS_PER_TIMETABLE: usize = 500;
const MAX_STRING_LENGTH: usize = 2_000;
/// The deepest real payload is six levels; eight leaves margin and still stops
/// a structure built only to be walked.
const MAX_JSON_DEPTH: usize = 8;

/// The `raw` keys anything downstream reads.
const ALLOWED_RAW_KEYS: [&str; 18] = [
    "bbox",
    "choice_uid",
    "default_times",
    "group_details",
    "group_labels",
    "group_speakers",
    "half_promotion",
    "lv1_room",
    "original_schedule_label",
    "page",
    "participation_confirmed",
    "pdf_start",
    "red_notes",
    "rooms",
    "schedule_label",
    "time_ranges",
    "until",
    "user_times",
];
const ALLOWED_GROUP_KEYS: [&str; 6] = ["label", "title", "start", "end", "location", "speakers"];

/// No string anywhere in the payload may exceed the cap, nor the nesting.
fn strings_are_bounded(value: &Value, depth: usize) -> bool {
    if depth > MAX_JSON_DEPTH {
        return false;
    }
    match value {
        Value::String(string) => text::len(string) <= MAX_STRING_LENGTH,
        Value::Object(fields) => fields.iter().all(|(key, item)| {
            depth < MAX_JSON_DEPTH && text::len(key) <= MAX_STRING_LENGTH && strings_are_bounded(item, depth + 1)
        }),
        Value::Array(items) => items.iter().all(|item| strings_are_bounded(item, depth + 1)),
        _ => true,
    }
}

/// Keep the fields the selection reads, as the strings it expects.
fn clean_group_details(value: &Value) -> Option<Value> {
    let mut details = Vec::new();
    for group in value.as_array()? {
        let fields: Map<String, Value> = group
            .as_object()?
            .iter()
            .filter(|(key, item)| ALLOWED_GROUP_KEYS.contains(&key.as_str()) && item.is_string())
            .map(|(key, item)| (key.clone(), item.clone()))
            .collect();
        // A group without a label is not a choice the reviewer can be offered.
        if !fields.contains_key("label") {
            return None;
        }
        details.push(Value::Object(fields));
    }
    Some(Value::Array(details))
}

/// Whitelist `raw` rather than trusting what came back: an unknown key would
/// otherwise travel into the answer and, from there, into a calendar.
fn clean_raw(value: Option<&Value>) -> Option<Value> {
    let fields = match value {
        None | Some(Value::Null) => return Some(Value::Object(Map::new())),
        Some(value) => value.as_object()?,
    };
    let mut cleaned = Map::new();
    for (key, item) in fields {
        if !ALLOWED_RAW_KEYS.contains(&key.as_str()) {
            continue;
        }
        let item = if key == "group_details" { clean_group_details(item)? } else { item.clone() };
        cleaned.insert(key.clone(), item);
    }
    Some(Value::Object(cleaned))
}

fn clean_event(value: &Value) -> Option<Value> {
    let mut event = value.as_object()?.clone();
    match event.get("links") {
        None | Some(Value::Null) => {}
        Some(links) => {
            if links.as_array()?.len() > MAX_LINKS_PER_EVENT {
                return None;
            }
        }
    }
    let raw = clean_raw(event.get("raw"))?;
    event.insert("raw".to_owned(), raw);
    Some(Value::Object(event))
}

/// A timetable payload safe to hand to `Timetable::from_json`, or `None` for
/// anything oversized or misshapen.
pub fn clean_timetable(value: Option<&Value>) -> Option<Value> {
    let fields = value?.as_object()?;
    let events = fields.get("events")?.as_array()?;
    let warnings = match fields.get("warnings") {
        None | Some(Value::Null) => Vec::new(),
        Some(warnings) => warnings.as_array()?.clone(),
    };
    if events.len() > MAX_RESTORED_EVENTS || warnings.len() > MAX_WARNINGS_PER_TIMETABLE {
        return None;
    }
    // Cheap and early: walking the strings costs far less than building the
    // events, and it is the bound that stops a single 4 MB field.
    if !strings_are_bounded(value?, 0) {
        return None;
    }
    let mut cleaned = Map::new();
    cleaned.insert("source".to_owned(), fields.get("source").cloned().unwrap_or(Value::Null));
    cleaned.insert("events".to_owned(), Value::Array(events.iter().map(clean_event).collect::<Option<_>>()?));
    cleaned.insert("warnings".to_owned(), Value::Array(warnings));
    Some(Value::Object(cleaned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn week() -> Value {
        json!({
            "source": {"file": "w.pdf", "sha256": ""},
            "events": [{"date": "2026-09-07", "start": null, "end": null, "title": "x", "links": [],
                        "raw": {"page": 1, "surprise": "y", "group_details": [{"label": "Groupe 1", "start": "09h00", "x": 1}]}}],
            "warnings": [],
        })
    }

    #[test]
    fn unknown_raw_keys_are_dropped() {
        let cleaned = clean_timetable(Some(&week())).unwrap();
        assert_eq!(
            cleaned["events"][0]["raw"],
            json!({"page": 1, "group_details": [{"label": "Groupe 1", "start": "09h00"}]})
        );
    }

    #[test]
    fn a_busy_week_keeps_all_its_warnings() {
        let mut busy = week();
        busy["warnings"] = Value::Array(vec![json!({"code": "EVENT_OVERLAP", "message": "x"}); 44]);
        let cleaned = clean_timetable(Some(&busy)).unwrap();
        assert_eq!(cleaned["warnings"].as_array().unwrap().len(), 44);
    }

    #[test]
    fn oversized_or_misshapen_payloads_are_refused() {
        assert!(clean_timetable(None).is_none());
        assert!(clean_timetable(Some(&json!([]))).is_none());
        let mut long = week();
        long["events"][0]["title"] = json!("x".repeat(2001));
        assert!(clean_timetable(Some(&long)).is_none());
        let mut many = week();
        many["events"] = Value::Array(vec![week()["events"][0].clone(); 201]);
        assert!(clean_timetable(Some(&many)).is_none());
        let mut flood = week();
        flood["warnings"] = Value::Array(vec![json!({"code": "EVENT_OVERLAP", "message": "x"}); 501]);
        assert!(clean_timetable(Some(&flood)).is_none());
        let mut links = week();
        links["events"][0]["links"] = Value::Array(vec![json!({"url": "u"}); 21]);
        assert!(clean_timetable(Some(&links)).is_none());
        let mut unlabelled = week();
        unlabelled["events"][0]["raw"]["group_details"] = json!([{"start": "09h00"}]);
        assert!(clean_timetable(Some(&unlabelled)).is_none());
        let mut deep = week();
        deep["events"][0]["raw"]["bbox"] = json!([[[[[[1]]]]]]);
        assert!(clean_timetable(Some(&deep)).is_none());
    }
}
