//! Typed view of the engine's selected week, for storage and display.
//!
//! Identity (`calendar_uid`), multi-day spans and tentative status come from
//! the engine itself: tentative is read back from the engine's own calendar
//! output, so the saved week always shows what an export would write.
use inspcal_engine::py::civil::{Date, Time};
use inspcal_engine::timetable::Timetable;
use serde::Serialize;
use serde_json::Value;

/// One session of a saved or previewed week.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// Canonical calendar identity (`Event::calendar_uid`).
    pub uid: String,
    pub title: String,
    pub module: Option<String>,
    pub location: Option<String>,
    pub location_note: Option<String>,
    pub speakers: Vec<String>,
    pub cohort_label: Option<String>,
    pub preparation: Option<String>,
    pub notes: Vec<String>,
    pub remote: bool,
    /// `YYYY-MM-DD`, school wall date.
    pub date: String,
    /// Last day covered, inclusive (multi-day sessions).
    pub last_date: String,
    /// `HH:MM` Europe/Paris wall time; `None` for all-day/undated hours.
    pub start: Option<String>,
    pub end: Option<String>,
    /// Instants, when the wall times exist unambiguously in Europe/Paris.
    pub start_at: Option<i64>,
    pub end_at: Option<i64>,
    pub all_day: bool,
    pub tentative: bool,
}

/// Why a week cannot be identified or projected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeekError {
    /// No week bounds: refusing rather than merging unrelated weeks.
    MissingIdentity,
    Unreadable,
}

/// `v1|<start>|<end>|<len>:<promotion>`: unambiguous, versioned, and the
/// same for any revision of the same week of the same promotion (as
/// `ics::same_week` compares them). Not a file name, hash or course.
pub fn week_key(timetable: &Value) -> Result<String, WeekError> {
    let parsed = Timetable::from_json(timetable).ok_or(WeekError::Unreadable)?;
    let (Some(start), Some(end)) = (parsed.source.week_start, parsed.source.week_end) else {
        return Err(WeekError::MissingIdentity);
    };
    let promotion = parsed.source.promotion.unwrap_or_default();
    Ok(format!(
        "v1|{}|{}|{}:{promotion}",
        start.iso(),
        end.iso(),
        promotion.chars().count()
    ))
}

/// Week bounds and promotion, for display.
pub fn week_bounds(timetable: &Value) -> Option<(String, String, Option<String>)> {
    let parsed = Timetable::from_json(timetable)?;
    Some((
        parsed.source.week_start?.iso(),
        parsed.source.week_end?.iso(),
        parsed.source.promotion,
    ))
}

/// The engine's STATUS for each UID in a calendar it rendered.
fn statuses(ics: &[u8]) -> Vec<(String, String)> {
    let text = String::from_utf8_lossy(ics)
        .replace("\r\n ", "")
        .replace("\n ", "");
    let mut found = Vec::new();
    let mut uid = None;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line == "BEGIN:VEVENT" {
            uid = None;
        } else if let Some(value) = line.strip_prefix("UID:") {
            uid = Some(
                value
                    .replace("\\,", ",")
                    .replace("\\;", ";")
                    .replace("\\\\", "\\"),
            );
        } else if let Some(status) = line.strip_prefix("STATUS:") {
            if let Some(uid) = uid.take() {
                found.push((uid, status.to_owned()));
            }
        }
    }
    found
}

/// Sessions of the engine's selected week (`body.timetable`), with tentative
/// status taken from the calendar the engine rendered for it
/// (`body.download`). Without a calendar (an empty week) nothing is tentative.
pub fn sessions(selected: &Value, calendar: Option<&[u8]>) -> Result<Vec<Session>, WeekError> {
    let timetable = Timetable::from_json(selected).ok_or(WeekError::Unreadable)?;
    let statuses = calendar.map(statuses).unwrap_or_default();
    Ok(timetable
        .events
        .iter()
        .map(|event| {
            let uid = event.calendar_uid();
            let last = event.last_date();
            let tentative = statuses
                .iter()
                .any(|(known, status)| *known == uid && status == "TENTATIVE");
            let timed = event.start.is_some();
            Session {
                title: event.title.clone(),
                module: event.module.clone(),
                location: event.location.clone(),
                location_note: event.location_note.clone(),
                speakers: event.speakers.clone(),
                cohort_label: event.cohort_label.clone(),
                preparation: event.preparation.clone(),
                notes: event.notes.clone(),
                remote: event.remote,
                date: event.date.iso(),
                last_date: last.iso(),
                start: event.start.map(Time::colon),
                end: event.end.map(Time::colon),
                start_at: event.start.and_then(|time| paris_instant(event.date, time)),
                end_at: event.end.and_then(|time| paris_instant(event.date, time)),
                all_day: !timed,
                tentative,
                uid,
            }
        })
        .collect())
}

/// Noon in Paris on an ISO date: an unambiguous anchor for all-day sessions.
pub fn paris_noon(date: &str) -> Option<i64> {
    let day = Date::parse_iso(date)?;
    paris_instant(day, Time::new(12, 0)?)
}

/// Last Sunday of a month, as days since 1970-01-01.
fn last_sunday(year: i32, month: u32) -> i64 {
    let last =
        Date::new(i64::from(year), i64::from(month), 31).expect("March and October have 31 days");
    let days = last.days();
    // 1970-01-01 was a Thursday; Sunday is weekday 0 here.
    let weekday = (days + 4).rem_euclid(7);
    days - weekday
}

/// Europe/Paris under the EU rule in force since 1996: UTC+2 from 01:00 UTC
/// on the last Sunday of March to 01:00 UTC on the last Sunday of October,
/// UTC+1 otherwise. Wall times that do not exist (spring gap) or exist twice
/// (autumn overlap) give `None`: they stay undated rather than guessed.
pub fn paris_instant(date: Date, time: Time) -> Option<i64> {
    if date.year < 1996 {
        return None;
    }
    let wall = date.days() * 86_400 + i64::from(time.hour) * 3_600 + i64::from(time.minute) * 60;
    let summer_from = last_sunday(date.year, 3) * 86_400 + 3_600;
    let summer_until = last_sunday(date.year, 10) * 86_400 + 3_600;
    let candidates: Vec<i64> = [(wall - 7_200, true), (wall - 3_600, false)]
        .into_iter()
        .filter(|(instant, summer)| (*instant >= summer_from && *instant < summer_until) == *summer)
        .map(|(instant, _)| instant)
        .collect();
    match candidates.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// One calendar file for a saved week, written by the engine's own ICS
/// renderer: stable UIDs, SEQUENCE, Europe/Paris VTIMEZONE, all-day and
/// tentative status. `previous` holds the week's earlier exported selections
/// (same week, same cohort): sessions they had that `selected` no longer has
/// are written as `STATUS:CANCELLED`, so a re-import removes them.
pub fn export_calendar(
    selected: &Value,
    previous: &[Value],
    prefix: &str,
    alarm: Option<i64>,
) -> Result<Vec<u8>, WeekError> {
    use inspcal_engine::ics::{to_calendars_after, IcsOptions};
    let current = Timetable::from_json(selected).ok_or(WeekError::Unreadable)?;
    let earlier: Vec<Timetable> = previous.iter().filter_map(Timetable::from_json).collect();
    let options = IcsOptions {
        prefix: prefix.to_owned(),
        alarm_minutes: alarm,
        split_by_module: false,
        ..IcsOptions::default()
    };
    let rendered = std::panic::catch_unwind(|| {
        to_calendars_after(std::slice::from_ref(&current), &earlier, &options)
    })
    .map_err(|_| WeekError::Unreadable)?
    .map_err(|_| WeekError::Unreadable)?;
    rendered
        .into_iter()
        .next()
        .map(|(_, bytes)| bytes)
        .ok_or(WeekError::Unreadable)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> Option<i64> {
        paris_instant(
            Date::new(year, month, day).unwrap(),
            Time::new(hour, minute).unwrap(),
        )
    }

    #[test]
    fn paris_wall_times_follow_the_eu_rule_and_refuse_gaps_and_overlaps() {
        // 2026-11-09 09:00 CET = 08:00 UTC.
        assert_eq!(at(2026, 11, 9, 9, 0), Some(1_794_211_200));
        // 2026-07-01 09:00 CEST = 07:00 UTC.
        assert_eq!(at(2026, 7, 1, 9, 0), Some(1_782_889_200));
        // 2026-03-29: 02:30 does not exist; 01:59 and 03:00 do.
        assert_eq!(at(2026, 3, 29, 2, 30), None);
        assert_eq!(
            at(2026, 3, 29, 1, 59).unwrap() + 60,
            at(2026, 3, 29, 3, 0).unwrap()
        );
        // 2026-10-25: 02:30 happens twice.
        assert_eq!(at(2026, 10, 25, 2, 30), None);
        assert_eq!(
            at(2026, 10, 25, 3, 0).unwrap() - at(2026, 10, 25, 1, 59).unwrap(),
            2 * 3_600 + 60
        );
    }

    #[test]
    fn all_day_anchors_are_noon_in_paris() {
        assert_eq!(paris_noon("2026-11-09"), Some(1_794_222_000));
        assert_eq!(paris_noon("not a date"), None);
    }

    #[test]
    fn statuses_are_read_from_folded_calendar_lines() {
        let ics = b"BEGIN:VEVENT\r\nUID:edt-insp-1\r\nSTATUS:TENTATIVE\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:edt-in\r\n sp-2\r\nSTATUS:CONFIRMED\r\nEND:VEVENT\r\n";
        assert_eq!(
            statuses(ics),
            vec![
                ("edt-insp-1".into(), "TENTATIVE".into()),
                ("edt-insp-2".into(), "CONFIRMED".into())
            ]
        );
    }
}
