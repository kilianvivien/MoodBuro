//! Blocking errors and warning codes: `src/edt_parser/errors.py`.

use std::fmt;

use crate::model::ExtractError;

/// A blocking parser error: the file yields no timetable.
///
/// Python attaches context to these for its logs. Only the code and the
/// message reach a reader, so only those are kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdtError {
    pub code: &'static str,
    pub message: String,
}

impl EdtError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }

    pub fn header_not_found(message: &str) -> Self {
        Self::new("HEADER_NOT_FOUND", message)
    }

    pub fn missing_column(message: &str) -> Self {
        Self::new("MISSING_COLUMN", message)
    }

    pub fn column_mismatch(message: &str) -> Self {
        Self::new("COLUMN_MISMATCH", message)
    }

    pub fn time(message: &str) -> Self {
        Self::new("TIME_PARSE", message)
    }

    pub fn date(message: &str) -> Self {
        Self::new("DATE_PARSE", message)
    }

    pub fn revision_conflict(message: &str) -> Self {
        Self::new("REVISION_CONFLICT", message)
    }

    /// What `worker.parse_bytes` answers for an exception it did not expect.
    pub fn unexpected() -> Self {
        Self::new("UNEXPECTED", "Erreur interne du parseur.")
    }
}

impl From<ExtractError> for EdtError {
    fn from(error: ExtractError) -> Self {
        Self { code: error.code, message: error.message }
    }
}

impl fmt::Display for EdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for EdtError {}

/// Non-blocking issues, surfaced in the JSON and on the review page.
pub mod warning {
    pub const NO_COHORT: &str = "NO_COHORT";
    pub const UNSCHEDULED_SESSION: &str = "UNSCHEDULED_SESSION";
    pub const DEFAULT_SCHEDULE: &str = "DEFAULT_SCHEDULE";
    pub const SPORT_CHOICE: &str = "SPORT_CHOICE";
    pub const AMBIGUOUS_LOCATION: &str = "AMBIGUOUS_LOCATION";
    pub const ROOM_ROTATION: &str = "ROOM_ROTATION";
    pub const DATE_OUT_OF_RANGE: &str = "DATE_OUT_OF_RANGE";
    pub const DATE_NOT_INCREASING: &str = "DATE_NOT_INCREASING";
    pub const UNKNOWN_COLUMN: &str = "UNKNOWN_COLUMN";
    pub const UNKNOWN_ROOM: &str = "UNKNOWN_ROOM";
    pub const RED_TEXT: &str = "RED_TEXT";
    pub const YEAR_FROM_METADATA: &str = "YEAR_FROM_METADATA";
    pub const LINK_WITHOUT_LABEL: &str = "LINK_WITHOUT_LABEL";
    pub const EVENT_OVERLAP: &str = "EVENT_OVERLAP";
    pub const IMPLAUSIBLE_EVENT_COUNT: &str = "IMPLAUSIBLE_EVENT_COUNT";
    pub const IMPLAUSIBLE_TIME: &str = "IMPLAUSIBLE_TIME";
    pub const DUPLICATE_UID: &str = "DUPLICATE_UID";
    pub const LOW_CONFIDENCE: &str = "LOW_CONFIDENCE";

    const ALL: [&str; 18] = [
        NO_COHORT,
        UNSCHEDULED_SESSION,
        DEFAULT_SCHEDULE,
        SPORT_CHOICE,
        AMBIGUOUS_LOCATION,
        ROOM_ROTATION,
        DATE_OUT_OF_RANGE,
        DATE_NOT_INCREASING,
        UNKNOWN_COLUMN,
        UNKNOWN_ROOM,
        RED_TEXT,
        YEAR_FROM_METADATA,
        LINK_WITHOUT_LABEL,
        EVENT_OVERLAP,
        IMPLAUSIBLE_EVENT_COUNT,
        IMPLAUSIBLE_TIME,
        DUPLICATE_UID,
        LOW_CONFIDENCE,
    ];

    /// `WarningCode(value)`: the known code, or nothing.
    pub fn known(value: &str) -> Option<&'static str> {
        ALL.into_iter().find(|code| *code == value)
    }
}
