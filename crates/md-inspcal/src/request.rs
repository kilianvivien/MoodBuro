//! Typed engine requests and answers. MoodBuro never forwards raw engine
//! fields from the shell: choices and options are validated here and encoded
//! as the engine's string fields.
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub const MAX_CHOICES: usize = 500;
pub const MAX_PREFIX: usize = 60;
pub const MAX_ALARM: i64 = 1_440;
const MAX_KEY: usize = 128;
const MAX_VALUE: usize = 200;

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Cohort {
    #[default]
    All,
    Fi,
    Cil,
    Off,
}

impl Cohort {
    fn field(self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::Fi => "FI",
            Self::Cil => "CIL",
            Self::Off => "OFF",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeChoice {
    pub start: String,
    pub end: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SportChoice {
    Keep,
    Exclude,
}

/// The reviewer's answers, keyed by the engine's session/week identities.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct Choices {
    /// Group label, `""` to join an optional session, `"__exclude__"` to decline.
    pub groups: BTreeMap<String, String>,
    /// Week start → `"A"` or `"B"`.
    pub half_promotion: BTreeMap<String, String>,
    pub rooms: BTreeMap<String, String>,
    pub times: BTreeMap<String, TimeChoice>,
    pub sport: BTreeMap<String, SportChoice>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct Options {
    pub cohort: Cohort,
    pub prefix: String,
    /// Minutes before each session for exported calendar alarms.
    pub alarm: Option<i64>,
    pub split_by_module: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            cohort: Cohort::All,
            prefix: String::new(),
            alarm: None,
            split_by_module: false,
        }
    }
}

/// Validation failures, as stable codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invalid {
    TooManyChoices,
    Choice,
    Prefix,
    Alarm,
}

impl Invalid {
    pub fn code(self) -> &'static str {
        match self {
            Self::TooManyChoices => "TOO_MANY_CHOICES",
            Self::Choice => "INVALID_CHOICE",
            Self::Prefix => "INVALID_PREFIX",
            Self::Alarm => "INVALID_ALARM",
        }
    }
}

fn bounded(key: &str, value: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_KEY
        && value.len() <= MAX_VALUE
        && !key.chars().chain(value.chars()).any(char::is_control)
}

impl Choices {
    pub fn validate(&self) -> Result<(), Invalid> {
        let count = self.groups.len()
            + self.half_promotion.len()
            + self.rooms.len()
            + self.times.len()
            + self.sport.len();
        if count > MAX_CHOICES {
            return Err(Invalid::TooManyChoices);
        }
        let plain = self
            .groups
            .iter()
            .chain(&self.half_promotion)
            .chain(&self.rooms);
        if !plain.into_iter().all(|(key, value)| bounded(key, value)) {
            return Err(Invalid::Choice);
        }
        if !self
            .half_promotion
            .values()
            .all(|half| half == "A" || half == "B")
        {
            return Err(Invalid::Choice);
        }
        let hhmm = |value: &str| {
            value.len() == 5
                && value.as_bytes()[2] == b':'
                && value[..2].parse::<u8>().is_ok_and(|h| h < 24)
                && value[3..].parse::<u8>().is_ok_and(|m| m < 60)
        };
        if !self.times.iter().all(|(key, time)| {
            bounded(key, "") && hhmm(&time.start) && hhmm(&time.end) && time.start < time.end
        }) {
            return Err(Invalid::Choice);
        }
        if !self.sport.keys().all(|key| bounded(key, "")) {
            return Err(Invalid::Choice);
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
            && self.half_promotion.is_empty()
            && self.rooms.is_empty()
            && self.times.is_empty()
            && self.sport.is_empty()
    }
}

impl Options {
    pub fn validate(&self) -> Result<(), Invalid> {
        if self.prefix.chars().count() > MAX_PREFIX || self.prefix.chars().any(char::is_control) {
            return Err(Invalid::Prefix);
        }
        if self
            .alarm
            .is_some_and(|minutes| !(0..=MAX_ALARM).contains(&minutes))
        {
            return Err(Invalid::Alarm);
        }
        Ok(())
    }
}

/// Engine fields for one request. `parsed` is the original reusable
/// `[{outcome,timetable}]` (never an already filtered week); `None` means the
/// PDF in the same call is read. `exported` holds bare selected timetables of
/// the same week and cohort, never `{timetable,cancelled}` envelopes.
pub fn fields(
    options: &Options,
    choices: &Choices,
    parsed: Option<&Value>,
    exported: &[Value],
) -> Result<Value, Invalid> {
    options.validate()?;
    choices.validate()?;
    let mut fields = Map::new();
    fields.insert("cohort".into(), json!(options.cohort.field()));
    fields.insert("prefix".into(), json!(options.prefix));
    if let Some(alarm) = options.alarm {
        fields.insert("alarm".into(), json!(alarm.to_string()));
    }
    if options.split_by_module {
        fields.insert("split_by_module".into(), json!("true"));
    }
    if let Some(parsed) = parsed {
        fields.insert("timetables".into(), json!(parsed.to_string()));
    }
    if !exported.is_empty() {
        fields.insert(
            "exported".into(),
            json!(Value::Array(exported.to_vec()).to_string()),
        );
    }
    let encode = |value: Value| json!(value.to_string());
    fields.insert("group_choices".into(), encode(json!(choices.groups)));
    fields.insert(
        "half_promotion_choices".into(),
        encode(json!(choices.half_promotion)),
    );
    fields.insert("room_choices".into(), encode(json!(choices.rooms)));
    fields.insert("time_choices".into(), encode(json!(choices.times)));
    fields.insert("sport_choices".into(), encode(json!(choices.sport)));
    Ok(Value::Object(fields))
}

/// What the review screen may see of the engine's questions: identities,
/// labels and times, never raw URLs.
pub const QUESTION_KEYS: [&str; 6] = [
    "group_sessions",
    "participation_sessions",
    "half_promotion_weeks",
    "time_sessions",
    "sport_weeks",
    "lv1_sessions",
];

/// A successful engine answer, split by purpose (plan §5).
#[derive(Clone, Debug)]
pub struct Answer {
    /// Original reusable pivot payload, to re-run choices without the PDF.
    pub parsed: Value,
    /// Selected combined week (warnings inside).
    pub selected: Value,
    /// Bare selected timetables for a later `exported` baseline.
    pub exportable: Vec<Value>,
    /// Cancellation records the engine reported alongside.
    pub cancelled: Vec<Value>,
    pub questions: Map<String, Value>,
    pub lv1_rooms: Vec<String>,
    /// The engine's calendar file for the selection, when it produced one.
    pub calendar: Option<Vec<u8>>,
}

/// An engine refusal, as a stable code plus the engine's own (French) detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub code: &'static str,
    pub detail: Option<String>,
}

fn file_error(body: &Value) -> Option<&'static str> {
    let code = body["error"]["code"]
        .as_str()
        .or_else(|| body["files"][0]["error"]["code"].as_str())?;
    Some(match code {
        "DATE_PARSE" => "WEEK_DATE_MISSING",
        "NOT_PDF" | "INVALID_PDF" | "PDF_READ" => "UNSUPPORTED_PDF",
        "TOO_LARGE" => "TOO_LARGE",
        _ => "UNSUPPORTED_PDF",
    })
}

/// Read an engine answer. Status 200 alone does not mean every choice is
/// answered; callers inspect `questions`.
pub fn answer(raw: &Value) -> Result<Answer, Refused> {
    let status = raw["status"].as_u64().unwrap_or(0);
    let body = &raw["body"];
    let detail = body["detail"].as_str().map(str::to_owned);
    if status != 200 {
        let code = match status {
            0 => "ENGINE_INTERNAL",
            400 => "INVALID_CHOICE",
            413 => "TOO_LARGE",
            _ => file_error(body).unwrap_or("UNSUPPORTED_PDF"),
        };
        return Err(Refused { code, detail });
    }
    let parsed = body["parsed"].clone();
    if !parsed.is_array() || !body["timetable"].is_object() {
        return Err(Refused {
            code: "ENGINE_INTERNAL",
            detail,
        });
    }
    let (exportable, cancelled) = body["exportable"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|envelope| (envelope["timetable"].clone(), envelope["cancelled"].clone()))
        .filter(|(timetable, _)| timetable.is_object())
        .fold(
            (Vec::new(), Vec::new()),
            |(mut kept, mut gone), (timetable, cancelled)| {
                kept.push(timetable);
                gone.extend(cancelled.as_array().cloned().unwrap_or_default());
                (kept, gone)
            },
        );
    let questions = QUESTION_KEYS
        .iter()
        .map(|key| ((*key).to_owned(), body[*key].clone()))
        .collect();
    let lv1_rooms = body["lv1_rooms"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|room| room.as_str().map(str::to_owned))
        .collect();
    let calendar = body["download"]["base64"]
        .as_str()
        .and_then(|data| base64::engine::general_purpose::STANDARD.decode(data).ok());
    Ok(Answer {
        parsed,
        selected: body["timetable"].clone(),
        exportable,
        cancelled,
        questions,
        lv1_rooms,
        calendar,
    })
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned).filter(|s| !s.is_empty())
}

/// A session as the review screen sees it: no geometry, no URLs.
fn brief(event: &Value) -> Value {
    json!({
        "uid": event["raw"]["choice_uid"].as_str().filter(|s| !s.is_empty()).or(event["uid"].as_str()),
        "sessionUid": event["uid"],
        "title": text(&event["title"]),
        "date": text(&event["date"]),
        "start": text(&event["start"]),
        "end": text(&event["end"]),
        "location": text(&event["location"]),
        "cohortLabel": text(&event["cohort_label"]),
        "module": text(&event["module"]),
    })
}

fn briefs(list: &Value) -> Vec<Value> {
    list.as_array().into_iter().flatten().map(brief).collect()
}

/// Review questions for the shell. Group options carry label, room and
/// times; links are reduced to their labels.
pub fn review_questions(answer: &Answer) -> Value {
    let q = &answer.questions;
    let groups: Vec<Value> = q["group_sessions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|event| {
            let mut item = brief(event);
            item["uid"] = event["uid"].clone();
            item["options"] = event["raw"]["group_details"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|group| json!({"label": text(&group["label"]), "location": text(&group["location"]), "start": text(&group["start"]), "end": text(&group["end"])}))
                .collect();
            item
        })
        .collect();
    let weeks = |key: &str| -> Vec<Value> {
        q[key]
            .as_array()
            .into_iter()
            .flatten()
            .map(|week| json!({"weekStart": text(&week["week_start"]), "sessions": briefs(&week["sessions"])}))
            .collect()
    };
    let with_session_uid = |key: &str| -> Vec<Value> {
        q[key]
            .as_array()
            .into_iter()
            .flatten()
            .map(|event| {
                let mut item = brief(event);
                item["uid"] = event["uid"].clone();
                item
            })
            .collect()
    };
    json!({
        "groups": groups,
        "participation": with_session_uid("participation_sessions"),
        "halfPromotion": weeks("half_promotion_weeks"),
        "times": with_session_uid("time_sessions"),
        "sport": weeks("sport_weeks"),
        "lv1": with_session_uid("lv1_sessions"),
        "lv1Rooms": answer.lv1_rooms,
    })
}

/// Warnings of the selected week, as `{code, message, uid}`.
pub fn warnings(answer: &Answer) -> Vec<Value> {
    answer.selected["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|w| json!({"code": text(&w["code"]), "message": text(&w["message"]), "uid": text(&w["event_uid"])}))
        .collect()
}

/// Labels of a session's links, without URLs; `index` resolves one in Rust.
pub fn link_labels(event: &Value) -> Vec<String> {
    event["links"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|link| text(&link["label"]))
        .collect()
}

/// The URL of a session's link, only if it is a plain HTTPS address with no
/// credentials. The shell receives labels only and asks Rust to open one.
pub fn safe_link(event: &Value, index: usize) -> Option<url::Url> {
    let raw = event["links"].as_array()?.get(index)?["url"].as_str()?;
    let url = url::Url::parse(raw).ok()?;
    (url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some())
    .then_some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_are_engine_strings_and_choices_are_bounded() {
        let mut choices = Choices::default();
        choices
            .groups
            .insert("edt-insp-1".into(), "__exclude__".into());
        choices.times.insert(
            "edt-insp-2".into(),
            TimeChoice {
                start: "10:00".into(),
                end: "11:30".into(),
            },
        );
        let options = Options {
            cohort: Cohort::Fi,
            prefix: "INSP ".into(),
            alarm: Some(15),
            split_by_module: false,
        };
        let built = fields(&options, &choices, Some(&json!([])), &[]).unwrap();
        assert_eq!(built["cohort"], "FI");
        assert_eq!(built["alarm"], "15");
        assert_eq!(built["timetables"], "[]");
        assert_eq!(built["group_choices"], "{\"edt-insp-1\":\"__exclude__\"}");
        assert!(built["group_choices"].is_string() && built["time_choices"].is_string());
        assert!(built.get("exported").is_none());

        let mut many = Choices::default();
        for index in 0..=MAX_CHOICES {
            many.rooms.insert(format!("uid-{index}"), "C102".into());
        }
        assert_eq!(many.validate(), Err(Invalid::TooManyChoices));
        let bad_time = Choices {
            times: [(
                "u".into(),
                TimeChoice {
                    start: "11:00".into(),
                    end: "10:00".into(),
                },
            )]
            .into(),
            ..Default::default()
        };
        assert_eq!(bad_time.validate(), Err(Invalid::Choice));
        let bad_half = Choices {
            half_promotion: [("2026-11-09".into(), "C".into())].into(),
            ..Default::default()
        };
        assert_eq!(bad_half.validate(), Err(Invalid::Choice));
        assert_eq!(
            Options {
                prefix: "x".repeat(61),
                ..Default::default()
            }
            .validate(),
            Err(Invalid::Prefix)
        );
        assert_eq!(
            Options {
                alarm: Some(1441),
                ..Default::default()
            }
            .validate(),
            Err(Invalid::Alarm)
        );
        assert_eq!(
            Options {
                alarm: Some(-1),
                ..Default::default()
            }
            .validate(),
            Err(Invalid::Alarm)
        );
        assert!(
            serde_json::from_value::<Choices>(json!({"participation": {}})).is_err(),
            "unknown fields refused"
        );
    }

    #[test]
    fn exportable_envelopes_are_unwrapped_and_refusals_coded() {
        let raw = json!({"status": 200, "body": {
            "parsed": [{"outcome": {}, "timetable": {}}],
            "timetable": {"events": []},
            "exportable": [{"timetable": {"events": [1]}, "cancelled": [{"uid": "x"}]}],
            "download": null
        }});
        let answer = answer(&raw).unwrap();
        assert_eq!(answer.exportable, vec![json!({"events": [1]})]);
        assert_eq!(answer.cancelled, vec![json!({"uid": "x"})]);
        assert!(answer.calendar.is_none());
        let refused = super::answer(
            &json!({"status": 422, "body": {"detail": "x", "error": {"code": "DATE_PARSE"}}}),
        )
        .unwrap_err();
        assert_eq!(refused.code, "WEEK_DATE_MISSING");
        assert_eq!(
            super::answer(&json!({"status": 0, "body": {}}))
                .unwrap_err()
                .code,
            "ENGINE_INTERNAL"
        );
    }
}
