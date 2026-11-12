//! Integrity rules and plausibility checks: `src/edt_parser/validate.py`.
//!
//! When in doubt the parser emits a visible warning, never a silently wrong
//! value. Blocking conditions have already been raised by the time a timetable
//! gets here; this module only adds warnings to it.

use serde_json::Value;

use crate::errors::warning;
use crate::events::{is_holiday, is_sport};
use crate::institution::INSTITUTION;
use crate::py::civil::Time;
use crate::timetable::{truthy, Event, Timetable};

/// Outside this range an event is suspicious enough to warn about.
const PLAUSIBLE_DAY_START: Time = Time { hour: 7, minute: 0 };
const PLAUSIBLE_DAY_END: Time = Time { hour: 21, minute: 0 };

/// A day with more than this many events is worth a second look.
const MAX_EVENTS_PER_DAY: usize = 12;

/// How far outside the week named by the file a date may fall unremarked.
const WEEK_SLACK_DAYS: i64 = 2;

fn overlap(left: &Event, right: &Event) -> bool {
    let (Some(left_start), Some(left_end), Some(right_start), Some(right_end)) =
        (left.start, left.end, right.start, right.end)
    else {
        return false;
    };
    left.date == right.date
        && left_start < right_end
        && right_start < left_end
        && (left.cohorts.is_empty()
            || right.cohorts.is_empty()
            || left.cohorts.iter().any(|token| right.cohorts.contains(token)))
}

fn label<'a>(event: &'a Event, default: &'a str) -> &'a str {
    event.raw.get("schedule_label").and_then(Value::as_str).unwrap_or(default)
}

/// Append every applicable warning to `timetable`.
pub fn validate(timetable: &mut Timetable) {
    let mut found: Vec<(&'static str, String, Option<String>)> = Vec::new();
    let (week_start, week_end) = (timetable.source.week_start, timetable.source.week_end);
    let events = &timetable.events;

    for event in events {
        let uid = Some(event.uid());
        let title = &event.title;
        let mut warn = |code, message: String| found.push((code, message, uid.clone()));
        // A day off concerns everyone and has no hours: nothing to check.
        let holiday = is_holiday(event);
        if event.cohorts.is_empty() && !holiday {
            let detail = if event.groups.multi {
                format!("séance en {} groupes", event.groups.count)
            } else {
                "cellule Cohorte vide".to_owned()
            };
            warn(
                warning::NO_COHORT,
                format!("« {title} » n'indique aucune cohorte FI/CIL/OFF ({detail}) — conservée pour tout le monde"),
            );
        }
        if truthy(event.raw.get("lv1_room")) {
            warn(
                warning::ROOM_ROTATION,
                format!(
                    "« {title} » : la salle dépend de la rotation LV1 — à relever sur Moodle ({})",
                    INSTITUTION.rotation_url
                ),
            );
        } else if let (None, Some(note)) =
            (&event.location, event.location_note.as_ref().filter(|note| !note.is_empty()))
        {
            warn(warning::AMBIGUOUS_LOCATION, format!("« {title} » : salle {}", note.to_lowercase()));
        }
        for note in event.raw.get("red_notes").and_then(Value::as_array).into_iter().flatten() {
            warn(warning::RED_TEXT, format!("« {title} » : {}", note.as_str().unwrap_or_default()));
        }
        for link in &event.links {
            if link.label.as_ref().is_none_or(|label| label.is_empty()) {
                warn(warning::LINK_WITHOUT_LABEL, format!("« {title} » : {}", link.url));
            }
        }
        let day = event.date.day_month();
        match (event.start, event.end) {
            (_, None) if !holiday => warn(
                warning::UNSCHEDULED_SESSION,
                format!(
                    "{day} — « {title} » : {}. Conservée comme rappel sans bloquer votre disponibilité ; horaires à vérifier.",
                    label(event, "horaires non précisés")
                ),
            ),
            (Some(start), Some(end)) if truthy(event.raw.get("default_times")) && !truthy(event.raw.get("user_times")) => warn(
                warning::DEFAULT_SCHEDULE,
                format!(
                    "{day} — « {title} » : le PDF indique « {} ». Horaire fixé par défaut de {} à {} \
                     — corrigez-le si vous connaissez le vrai créneau.",
                    label(event, "horaire imprécis"),
                    start.colon(),
                    end.colon()
                ),
            ),
            _ => {}
        }
        // A default we set ourselves (DEFAULT_SCHEDULE above) is already
        // flagged; only printed times are judged for plausibility.
        let guessed = truthy(event.raw.get("default_times")) && !truthy(event.raw.get("user_times"));
        if let (Some(start), Some(end), false) = (event.start, event.end, guessed) {
            if start < PLAUSIBLE_DAY_START || end > PLAUSIBLE_DAY_END {
                warn(
                    warning::IMPLAUSIBLE_TIME,
                    format!("« {title} » se déroule de {} à {}", start.colon(), end.colon()),
                );
            }
        }
        if let (Some(start), Some(end)) = (week_start, week_end) {
            if event.date < start.plus_days(-WEEK_SLACK_DAYS) || event.date > end.plus_days(WEEK_SLACK_DAYS) {
                warn(
                    warning::DATE_OUT_OF_RANGE,
                    format!("« {title} » tombe le {}, hors de la semaine {start} – {end}", event.date),
                );
            }
        }
    }

    for pair in events.windows(2) {
        if pair[1].date < pair[0].date {
            let message = format!("le {} suit le {} dans le tableau", pair[1].date, pair[0].date);
            found.push((warning::DATE_NOT_INCREASING, message, Some(pair[1].uid())));
        }
    }
    found.extend(overlaps(events));

    let mut days: Vec<_> = events.iter().map(|event| event.date).collect();
    days.sort();
    days.dedup();
    for day in days {
        let count = events.iter().filter(|event| event.date == day).count();
        if count > MAX_EVENTS_PER_DAY {
            found.push((warning::IMPLAUSIBLE_EVENT_COUNT, format!("{count} séances le {day}"), None));
        }
    }
    if events.is_empty() {
        found.push((warning::IMPLAUSIBLE_EVENT_COUNT, "aucune séance trouvée dans ce PDF".to_owned(), None));
    }

    // Two sessions hashing to one UID means the calendar silently keeps one.
    // The occurrence index rules it out for a parsed week; what this still
    // catches is an event added without the identities being assigned again.
    let uids: Vec<String> = events.iter().map(Event::calendar_uid).collect();
    for (index, uid) in uids.iter().enumerate() {
        let count = uids.iter().filter(|other| *other == uid).count();
        if count < 2 || uids[..index].contains(uid) {
            continue;
        }
        found.push((
            warning::DUPLICATE_UID,
            format!(
                "{count} séances « {} » partagent une identité de calendrier (même semaine, intitulé, cohorte et rang) : \
                 votre agenda n'en gardera qu'une.",
                events[index].title
            ),
            Some(uid.clone()),
        ));
    }

    // One APSA session a week is compulsory; the PDF prints every slot. They
    // are all kept — dropping two of three is the reviewer's call — and it is
    // said once, with the slots listed.
    let sport: Vec<&Event> = events.iter().filter(|event| is_sport(event)).collect();
    if sport.len() >= 2 {
        let slots: Vec<String> = sport
            .iter()
            .map(|event| match event.start {
                Some(start) => format!("{} à {}", event.date.day_month(), start.colon()),
                None => event.date.day_month(),
            })
            .collect();
        found.push((
            warning::SPORT_CHOICE,
            format!(
                "{} créneaux {} cette semaine ({}) — une seule séance est obligatoire : choisissez celle que vous suivez.",
                sport.len(),
                if sport.iter().all(|event| event.title.contains("APSA")) { "APSA" } else { "de sport" },
                slots.join(", ")
            ),
            None,
        ));
    }

    for (code, message, uid) in found {
        timetable.warn(code, message, uid);
    }
}

/// One warning per pair of sessions of the same day that share a cohort and
/// some of their time.
pub fn overlaps(events: &[Event]) -> Vec<(&'static str, String, Option<String>)> {
    let mut found = Vec::new();
    for (index, left) in events.iter().enumerate() {
        for right in &events[index + 1..] {
            if overlap(left, right) {
                found.push((
                    warning::EVENT_OVERLAP,
                    format!("« {} » et « {} » se chevauchent le {}", left.title, right.title, left.date),
                    Some(left.uid()),
                ));
            }
        }
    }
    found
}
