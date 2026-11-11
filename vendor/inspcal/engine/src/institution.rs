//! The school's own facts, from `engine/institution.json`.
//!
//! The file is compiled into the browser and native Rust tools: a new LV1 room
//! is still a one-line data commit, followed by a rebuild.

use std::sync::LazyLock;

use serde_json::Value;

use crate::py::civil::Time;

const DATA: &str = include_str!("../institution.json");

pub struct Institution {
    /// Bumped whenever anything in the file changes; written into every calendar.
    pub version: String,
    /// The Moodle folder holding the LV1 rotation table.
    pub rotation_url: String,
    /// Rooms seen in the rotation tables: a convenience, never a closed set.
    pub lv1_rooms: Vec<String>,
    pub cohort_tokens: Vec<String>,
    pub sport_keyword: String,
    /// What « Matin » and « Après-midi » mean in hours, keyed in lower case.
    pub half_day_slots: Vec<(String, Time, Time)>,
    /// Header labels by column role, in the order the file lists the roles.
    pub header_aliases: Vec<(String, Vec<String>)>,
}

fn strings(value: &Value) -> Vec<String> {
    value.as_array().expect("a list").iter().map(|item| item.as_str().expect("a string").to_owned()).collect()
}

fn hour(value: &Value) -> Time {
    let (hour, minute) = value.as_str().and_then(|text| text.split_once(':')).expect("HH:MM");
    Time::new(hour.parse().expect("an hour"), minute.parse().expect("a minute")).expect("a time of day")
}

pub static INSTITUTION: LazyLock<Institution> = LazyLock::new(|| {
    let data: Value = serde_json::from_str(DATA).expect("institution.json is valid JSON");
    let text = |key: &str| data[key].as_str().unwrap_or_else(|| panic!("institution.json: {key}")).to_owned();
    Institution {
        version: text("version"),
        rotation_url: text("rotation_url"),
        lv1_rooms: strings(&data["lv1_rooms"]),
        cohort_tokens: strings(&data["cohort_tokens"]).iter().map(|token| token.to_uppercase()).collect(),
        sport_keyword: text("sport_keyword"),
        half_day_slots: data["half_day_slots"]
            .as_object()
            .expect("half_day_slots")
            .iter()
            .map(|(label, slot)| (label.to_lowercase(), hour(&slot[0]), hour(&slot[1])))
            .collect(),
        header_aliases: data["header_aliases"]
            .as_object()
            .expect("header_aliases")
            .iter()
            .map(|(role, aliases)| (role.clone(), strings(aliases)))
            .collect(),
    }
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_loads_with_its_roles_in_order() {
        let roles: Vec<&str> = INSTITUTION.header_aliases.iter().map(|(role, _)| role.as_str()).collect();
        assert_eq!(roles, crate::timetable::COLUMN_ROLES);
        assert_eq!(INSTITUTION.cohort_tokens, ["FI", "CIL", "OFF"]);
        assert_eq!(INSTITUTION.half_day_slots[0].0, "matin");
    }
}
