//! The allowlist of actions a model may request. Model output is untrusted: a call is
//! only a request, parsed here into a typed value and authorised again by the caller.
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Course(i64),
    Activity(i64),
    Deadline(i64),
    Notifications,
    Calendar,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    ListCourses,
    FindActivities {
        query: String,
        course_id: Option<i64>,
    },
    ListDeadlines {
        days: u32,
    },
    ListNotifications {
        unread_only: bool,
    },
    Open(Target),
    /// The only write: always confirmed by the user before it runs.
    MarkNotificationRead {
        id: i64,
    },
}
impl Call {
    pub fn is_write(&self) -> bool {
        matches!(self, Self::MarkNotificationRead { .. })
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("unknown tool")]
    Unknown,
    #[error("invalid arguments")]
    Arguments,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Nothing {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Find {
    #[serde(default)]
    query: String,
    #[serde(default)]
    course_id: Option<i64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Deadlines {
    #[serde(default)]
    days: Option<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Notifications {
    #[serde(default)]
    unread_only: Option<bool>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Open {
    kind: String,
    #[serde(default)]
    id: Option<i64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identifier {
    id: i64,
}

fn arguments<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, Error> {
    // Some models send an empty string for a tool without arguments.
    let raw = if raw.trim().is_empty() { "{}" } else { raw };
    let value: Value = serde_json::from_str(raw).map_err(|_| Error::Arguments)?;
    // Named fields only: serde would otherwise accept a positional array.
    if !value.is_object() {
        return Err(Error::Arguments);
    }
    serde_json::from_value(value).map_err(|_| Error::Arguments)
}
fn positive(id: Option<i64>) -> Result<i64, Error> {
    id.filter(|id| *id > 0).ok_or(Error::Arguments)
}

pub fn parse(name: &str, raw: &str) -> Result<Call, Error> {
    Ok(match name {
        "list_courses" => {
            arguments::<Nothing>(raw)?;
            Call::ListCourses
        }
        "find_activities" => {
            let find: Find = arguments(raw)?;
            let query = find.query.trim().to_owned();
            if query.chars().count() > 100 {
                return Err(Error::Arguments);
            }
            Call::FindActivities {
                query,
                course_id: find.course_id.map(|id| positive(Some(id))).transpose()?,
            }
        }
        "list_deadlines" => {
            let days = arguments::<Deadlines>(raw)?.days.unwrap_or(14);
            if !(1..=60).contains(&days) {
                return Err(Error::Arguments);
            }
            Call::ListDeadlines { days }
        }
        "list_notifications" => Call::ListNotifications {
            unread_only: arguments::<Notifications>(raw)?
                .unread_only
                .unwrap_or(false),
        },
        "open_in_moodle" => {
            let open: Open = arguments(raw)?;
            Call::Open(match (open.kind.as_str(), open.id) {
                ("course", id) => Target::Course(positive(id)?),
                ("activity", id) => Target::Activity(positive(id)?),
                ("deadline", id) => Target::Deadline(positive(id)?),
                ("notifications", None) => Target::Notifications,
                ("calendar", None) => Target::Calendar,
                _ => return Err(Error::Arguments),
            })
        }
        "mark_notification_read" => Call::MarkNotificationRead {
            id: positive(Some(arguments::<Identifier>(raw)?.id))?,
        },
        _ => return Err(Error::Unknown),
    })
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": {"type": "object", "properties": properties, "required": required},
        },
    })
}
/// OpenAI-style tool definitions accepted by both providers.
pub fn definitions() -> Value {
    json!([
        tool(
            "list_courses",
            "List the user's Moodle courses in the attached organisation.",
            json!({}),
            &[]
        ),
        tool(
            "find_activities",
            "Search activities and resources by name in the user's saved course contents. Leave query empty to list a course.",
            json!({
                "query": {"type": "string", "description": "Words from the activity name."},
                "course_id": {"type": "integer", "description": "Limit to one course id from list_courses."},
            }),
            &[]
        ),
        tool(
            "list_deadlines",
            "List upcoming Moodle calendar events and deadlines.",
            json!({"days": {"type": "integer", "description": "Days ahead, 1 to 60. Default 14."}}),
            &[]
        ),
        tool(
            "list_notifications",
            "List recent Moodle notification subjects.",
            json!({"unread_only": {"type": "boolean"}}),
            &[]
        ),
        tool(
            "open_in_moodle",
            "Show an item in the app's Moodle view. Use only when the user asks to open or go to something.",
            json!({
                "kind": {"type": "string", "enum": ["course", "activity", "deadline", "notifications", "calendar"]},
                "id": {"type": "integer", "description": "Required for course, activity and deadline."},
            }),
            &["kind"]
        ),
        tool(
            "mark_notification_read",
            "Ask the user to confirm marking one notification as read. Use only when the user asks for it.",
            json!({"id": {"type": "integer", "description": "Notification id from list_notifications."}}),
            &["id"]
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_defined_tool_parses_and_nothing_else_does() {
        let definitions = definitions();
        let names: Vec<&str> = definitions
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap())
            .collect();
        assert_eq!(names.len(), 6);
        for name in &names {
            assert_ne!(parse(name, "not json"), Err(Error::Unknown));
        }
        for name in [
            "submit_assignment",
            "set_grade",
            "run_script",
            "mark_all_notifications_read",
            "",
        ] {
            assert_eq!(parse(name, "{}"), Err(Error::Unknown));
        }
    }
    #[test]
    fn typed_arguments_with_defaults() {
        assert_eq!(parse("list_courses", ""), Ok(Call::ListCourses));
        assert_eq!(
            parse("find_activities", r#"{"query":" quiz ","course_id":4}"#),
            Ok(Call::FindActivities {
                query: "quiz".into(),
                course_id: Some(4)
            })
        );
        assert_eq!(
            parse("list_deadlines", "{}"),
            Ok(Call::ListDeadlines { days: 14 })
        );
        assert_eq!(
            parse("list_notifications", r#"{"unread_only":true}"#),
            Ok(Call::ListNotifications { unread_only: true })
        );
        assert_eq!(
            parse("open_in_moodle", r#"{"kind":"deadline","id":9}"#),
            Ok(Call::Open(Target::Deadline(9)))
        );
        assert_eq!(
            parse("open_in_moodle", r#"{"kind":"calendar"}"#),
            Ok(Call::Open(Target::Calendar))
        );
        let write = parse("mark_notification_read", r#"{"id":3}"#).unwrap();
        assert!(write.is_write());
        assert!(!Call::ListCourses.is_write());
    }
    #[test]
    fn unknown_fields_urls_and_out_of_range_values_are_rejected() {
        for (name, raw) in [
            ("list_courses", r#"{"site":"other"}"#),
            (
                "find_activities",
                r#"{"query":"a","url":"https://evil.example"}"#,
            ),
            ("find_activities", r#"{"course_id":0}"#),
            ("list_deadlines", r#"{"days":0}"#),
            ("list_deadlines", r#"{"days":61}"#),
            ("list_deadlines", r#"{"days":"all"}"#),
            ("open_in_moodle", r#"{"kind":"course"}"#),
            ("open_in_moodle", r#"{"kind":"course","id":-1}"#),
            ("open_in_moodle", r#"{"kind":"calendar","id":1}"#),
            ("open_in_moodle", r#"{"kind":"https://evil.example"}"#),
            ("open_in_moodle", r#"{"kind":"messages"}"#),
            ("open_in_moodle", r#"{"kind":"course","id":1,"url":"x"}"#),
            ("mark_notification_read", "{}"),
            ("mark_notification_read", r#"{"id":null}"#),
            ("mark_notification_read", r#"{"id":1,"all":true}"#),
            ("mark_notification_read", r#"[1]"#),
        ] {
            assert_eq!(parse(name, raw), Err(Error::Arguments), "{name} {raw}");
        }
        let long = format!(r#"{{"query":"{}"}}"#, "a".repeat(101));
        assert_eq!(parse("find_activities", &long), Err(Error::Arguments));
    }
}
