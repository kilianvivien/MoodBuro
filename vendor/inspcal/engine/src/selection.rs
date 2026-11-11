//! The reviewer's choices, written into a copy of the parsed weeks:
//! `src/edt_parser/selection.py`.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{json, Map, Value};

use crate::dates;
use crate::errors::warning;
use crate::events::{is_registration_only, is_sport, SPEAKER_SEPARATOR};
use crate::institution::INSTITUTION;
use crate::py::civil::Time;
use crate::py::text;
use crate::rooms;
use crate::timetable::{truthy, Event, Groups, Timetable};
use crate::validate;

/// An APSA slot prints a start and no end. Once the reviewer says this is the
/// session they attend, the calendar needs a block: two hours, the length of a
/// sport slot, and editable like any other completed schedule.
const SPORT_DEFAULT_MINUTES: u32 = 120;

/// Group schedules supplied outside the PDF, scoped to a dated session and
/// its printed window. They are review choices, not extracted PDF facts.
#[derive(serde::Deserialize)]
struct GroupSchedule {
    date: String,
    title: String,
    start: String,
    end: String,
    rotation: Option<String>,
    groups: Vec<Map<String, Value>>,
}

static GROUP_SCHEDULES: LazyLock<Vec<GroupSchedule>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("group-schedules.json")).expect("valid supplied group schedules")
});

fn supplied_schedule(event: &Event) -> Option<&'static GroupSchedule> {
    GROUP_SCHEDULES.iter().find(|schedule| {
        event.date.iso() == schedule.date
            && crate::grid::fold(&event.title) == crate::grid::fold(&schedule.title)
            && event.start.map(Time::colon).as_deref() == Some(schedule.start.as_str())
            && event.end.map(Time::colon).as_deref() == Some(schedule.end.as_str())
    })
}

pub fn rotation_label(event: &Event) -> Option<&str> {
    // Explicit printed group details outrank the supplemental schedule.
    if event.raw.get("group_details").and_then(Value::as_array).is_some_and(|groups| !groups.is_empty()) {
        return None;
    }
    supplied_schedule(event)?.rotation.as_deref()
}

pub fn group_details(event: &Event) -> Vec<Map<String, Value>> {
    let printed: Vec<_> = event
        .raw
        .get("group_details")
        .and_then(Value::as_array)
        .map(|groups| groups.iter().filter_map(Value::as_object).cloned().collect())
        .unwrap_or_default();
    if !printed.is_empty() {
        return printed;
    }
    let Some(schedule) = supplied_schedule(event) else { return Vec::new() };
    schedule
        .groups
        .iter()
        .cloned()
        .map(|mut group| {
            if let Some(location) = &event.location {
                group.insert("location".to_owned(), json!(location));
            }
            if !event.speakers.is_empty() {
                group.insert("speakers".to_owned(), json!(event.speakers.join(SPEAKER_SEPARATOR)));
            }
            group
        })
        .collect()
}

/// What the reviewer chose, each map keyed on the event UID as parsed.
#[derive(Debug, Default)]
pub struct Choices {
    /// A or B, keyed on the Monday of the week; independent of workshop groups.
    pub half_promotions: Map<String, Value>,
    /// A group label, or `__exclude__`.
    pub groups: Map<String, Value>,
    /// The LV1 room read off the Moodle rotation.
    pub rooms: Map<String, Value>,
    /// `{"start": "HH:MM", "end": "HH:MM"}`, either possibly empty.
    pub times: Map<String, Value>,
    /// `keep` or `exclude`, naming the one sport session attended.
    pub sport: Map<String, Value>,
}

pub fn keeps_half_promotion(event: &Event, choices: &Choices) -> bool {
    let Some(half) = event.raw.get("half_promotion").and_then(Value::as_str) else { return true };
    let week = event.week_start.unwrap_or(event.date).iso();
    choices.half_promotions.get(&week).and_then(Value::as_str).is_none_or(|chosen| chosen == half)
}

fn text_of<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key).and_then(Value::as_str)
}

fn label(event: &Event) -> Option<&str> {
    text_of(&event.raw, "schedule_label")
}

/// A copy of `timetables` with the reviewer's choices written in.
///
/// Choosing a group changes an event's title and cohort, and therefore its
/// UID, so every key is read before anything is applied. `now` stamps the
/// weeks that were touched: one instant for the whole run, so two tables
/// revised together stay comparable.
pub fn select_groups(timetables: &[Timetable], choices: &Choices, now: i64) -> Result<Vec<Timetable>, String> {
    let mut result = timetables.to_vec();
    for (week, half) in &choices.half_promotions {
        if !matches!(half.as_str(), Some("A" | "B"))
            || !timetables.iter().any(|table| {
                table.source.week_start.map(|date| date.iso()).as_ref() == Some(week)
                    && table.events.iter().any(|event| event.raw.get("half_promotion") == Some(half))
            })
        {
            return Err("Le choix de demi-promo est invalide pour cette semaine.".to_owned());
        }
    }
    if choices.sport.values().any(|value| !matches!(value.as_str(), Some("keep" | "exclude"))) {
        return Err("Choix de séance de sport invalide.".to_owned());
    }
    let known: Vec<String> = result.iter().flat_map(|table| &table.events).map(Event::uid).collect();
    let chosen =
        choices.groups.keys().chain(choices.rooms.keys()).chain(choices.times.keys()).chain(choices.sport.keys());
    if chosen.into_iter().any(|uid| !known.contains(uid)) {
        return Err("Une séance sélectionnée n'est plus présente. Relancez l'analyse.".to_owned());
    }
    for table in &mut result {
        let mut kept = Vec::new();
        let mut touched = false;
        let picked_sport = table.events.iter().any(|event| choices.sport.contains_key(&event.uid()));
        let kept_sport: Vec<String> = table
            .events
            .iter()
            .filter(|event| is_sport(event) && text_of(&choices.sport, &event.uid()) == Some("keep"))
            .map(Event::uid)
            .collect();
        if kept_sport.len() > 1 {
            return Err("Choisissez une seule séance de sport par semaine.".to_owned());
        }
        for mut event in std::mem::take(&mut table.events) {
            if !keeps_half_promotion(&event, choices) {
                touched = true;
                continue;
            }
            let uid = event.uid();
            let choice = text_of(&choices.groups, &uid);
            touched = touched
                || choices.groups.contains_key(&uid)
                || choices.rooms.contains_key(&uid)
                || choices.times.contains_key(&uid)
                || choices.sport.contains_key(&uid);
            if choice == Some("__exclude__") {
                continue;
            }
            if is_sport(&event) && !kept_sport.is_empty() && !kept_sport.contains(&uid) {
                continue;
            }
            let drop = |table: &mut Timetable, codes: &[&str]| {
                table
                    .warnings
                    .retain(|warning| !(warning.event_uid.as_deref() == Some(&uid) && codes.contains(&warning.code)));
            };
            if let Some(answer) = text_of(&choices.sport, &uid) {
                if !is_sport(&event) {
                    return Err("Cette séance n'est pas un créneau de sport.".to_owned());
                }
                if answer == "exclude" {
                    continue;
                }
                keep_sport(&mut event);
                if event.end.is_some() {
                    drop(table, &[warning::UNSCHEDULED_SESSION]);
                }
            }
            // Opting in turns an optional session's tentative block into a
            // confirmed one; with no answer it stays tentative.
            if choice == Some("") && is_registration_only(&event) {
                event.raw.insert("participation_confirmed".to_owned(), json!(true));
            }
            if let Some(choice) = choice.filter(|choice| !choice.is_empty()) {
                apply_group(&mut event, choice)?;
                if event.location.as_ref().is_some_and(|room| !room.is_empty()) {
                    drop(table, &[warning::AMBIGUOUS_LOCATION]);
                }
            }
            // The room comes last: an LV1 room read off Moodle is a fact about
            // this week, and outranks whatever the group cell happened to say.
            if let Some(room) = text_of(&choices.rooms, &uid) {
                if !truthy(event.raw.get("lv1_room")) {
                    return Err("Cette séance n'est pas un créneau de LV1.".to_owned());
                }
                let room = rooms::clean_room(room)?;
                event.location_note = None;
                drop(table, &[warning::ROOM_ROTATION]);
                if !INSTITUTION.lv1_rooms.contains(&room) {
                    // Keep the room — the reviewer read it off Moodle and
                    // knows better than a list in a file. But say it is off
                    // the roster: that is how a stale roster becomes visible.
                    table.warn(
                        warning::UNKNOWN_ROOM,
                        format!(
                            "« {} » : la salle « {room} » ne figure pas dans la rotation LV1 connue. Elle est conservée \
                             telle quelle ; signalez-la pour mettre la liste à jour.",
                            event.title
                        ),
                        Some(uid.clone()),
                    );
                }
                event.location = Some(room);
            }
            if let Some(times) = choices.times.get(&uid) {
                set_times(&mut event, times)?;
                drop(table, &[warning::UNSCHEDULED_SESSION, warning::DEFAULT_SCHEDULE]);
                if event.end.is_none() {
                    table.warn(
                        warning::UNSCHEDULED_SESSION,
                        format!(
                            "{} — « {} » : {}. Rappel sans durée définie.",
                            event.date.day_month(),
                            event.title,
                            label(&event).unwrap_or_default()
                        ),
                        Some(uid.clone()),
                    );
                }
            }
            let renamed = event.uid();
            for warning in &mut table.warnings {
                if warning.event_uid.as_deref() == Some(&uid) {
                    warning.event_uid = Some(renamed.clone());
                }
            }
            // A choice changes title or cohort, and therefore the UID. The
            // review panels keep asking on the UID they were given, so the
            // event carries it: without it, a second round of choices would
            // address no one.
            if renamed != uid {
                event.raw.insert("choice_uid".to_owned(), json!(uid));
            }
            kept.push(event);
        }
        table.events = kept;
        if picked_sport {
            table.warnings.retain(|warning| warning.code != warning::SPORT_CHOICE);
        }
        let valid: Vec<String> = table.events.iter().map(Event::uid).collect();
        table
            .warnings
            .retain(|warning| warning.event_uid.as_ref().is_none_or(|uid| uid.is_empty() || valid.contains(uid)));
        if touched {
            table.revised_at = Some(now);
        }
        if !choices.times.is_empty() || !choices.groups.is_empty() || !choices.half_promotions.is_empty() {
            table.warnings.retain(|warning| warning.code != warning::EVENT_OVERLAP);
            for (code, message, uid) in validate::overlaps(&table.events) {
                table.warn(code, message, uid);
            }
        }
    }
    Ok(result)
}

/// Narrow a session in groups to the one group the reviewer is in.
fn apply_group(event: &mut Event, choice: &str) -> Result<(), String> {
    let details = group_details(event);
    let rotation = rotation_label(event).is_some();
    let matches: Vec<&Map<String, Value>> =
        details.iter().filter(|group| text_of(group, "label") == Some(choice)).collect();
    let [group] = matches[..] else {
        return Err("Le groupe choisi est inconnu pour cette séance.".to_owned());
    };
    let (Some(start), Some(end)) = (text_of(group, "start"), text_of(group, "end")) else {
        return Err("Les horaires de ce groupe sont ambigus.".to_owned());
    };
    // Python lets an unreadable hour escape here as a parser error, which no
    // caller expects. It is the same refusal as hours that run backwards.
    let invalid = || "Les horaires de ce groupe sont invalides.".to_owned();
    let start = dates::parse_time(start).map_err(|_| invalid())?;
    let end = dates::parse_time(end).map_err(|_| invalid())?;
    if start >= end {
        return Err(invalid());
    }
    (event.start, event.end) = (Some(start), Some(end));
    event.cohort_label = Some(choice.to_owned());
    if let Some(title) = text_of(group, "title").filter(|title| !title.is_empty()) {
        // A block of workshops: the group's own is the session.
        event.title = format!("{} : {title}", event.title);
    }
    event.groups = Groups::default();
    event.location = text_of(group, "location").map(str::to_owned);
    let has_room = event.location.as_ref().is_some_and(|room| !room.is_empty());
    event.location_note = (!has_room).then(|| "À vérifier dans le PDF".to_owned());
    if let Some(speakers) = text_of(group, "speakers").filter(|speakers| !speakers.is_empty()) {
        event.speakers = speakers.split(SPEAKER_SEPARATOR).map(str::to_owned).collect();
    }
    let labels: Vec<String> =
        details.iter().map(|group| format!("{} : ", text_of(group, "label").unwrap_or_default())).collect();
    let own = format!("{choice} : ");
    event.notes.retain(|note| !labels.iter().any(|label| note.starts_with(label)) || note.starts_with(&own));
    if rotation {
        event.notes.push(format!("Horaires précisés par le roulement des ateliers pour {choice}."));
    }
    Ok(())
}

/// The chosen APSA becomes a real block; the PDF only prints its start.
fn keep_sport(event: &mut Event) {
    let (Some(start), None) = (event.start, event.end) else { return };
    let end = start.plus_minutes(SPORT_DEFAULT_MINUTES);
    if end <= start {
        // A slot running past midnight keeps no end.
        return;
    }
    event.end = Some(end);
    let printed = event.raw.get("schedule_label").cloned().unwrap_or(Value::Null);
    event.raw.insert("original_schedule_label".to_owned(), printed);
    event.raw.insert("schedule_label".to_owned(), json!(format!("à partir de {}", start.french())));
    event.raw.insert("default_times".to_owned(), json!(true));
}

/// Fill incomplete schedules only; keep the PDF's known start authoritative.
///
/// A default schedule — the block `Matin`/`Après-midi` stands for, or the two
/// hours given to the chosen APSA — is ours, not the PDF's, so it stays
/// editable, start included.
fn set_times(event: &mut Event, choice: &Value) -> Result<(), String> {
    static HOUR: LazyLock<Regex> = LazyLock::new(|| text::full(r"(?:[01]\d|2[0-3]):[0-5]\d"));
    let guessed = truthy(event.raw.get("default_times"));
    if event.end.is_some() && !guessed {
        return Err("Cette séance possède déjà des horaires complets dans le PDF.".to_owned());
    }
    if event.last_date() != event.date {
        // « Du 28/09 au 02/10 »: hours on one day would shrink the whole stretch.
        return Err("Cette séance s'étend sur plusieurs jours : ses horaires ne se complètent pas.".to_owned());
    }
    let fields = choice.as_object().filter(|fields| fields.len() == 2);
    let (Some(start), Some(end)) = (fields.and_then(|f| text_of(f, "start")), fields.and_then(|f| text_of(f, "end")))
    else {
        return Err("Les horaires renseignés sont invalides.".to_owned());
    };
    if [start, end].iter().any(|value| !value.is_empty() && !HOUR.is_match(value)) {
        return Err("Utilisez des horaires au format HH:MM.".to_owned());
    }
    if start.is_empty() {
        if !end.is_empty() || !guessed {
            return Err("Renseignez une heure de début avant l’heure de fin.".to_owned());
        }
        // Both fields emptied on a schedule we guessed: the reviewer is
        // telling us they do not know either, so the session goes back to the
        // whole day.
        let original = event.raw.shift_remove("original_schedule_label").filter(|value| truthy(Some(value)));
        let restored = original.or_else(|| event.raw.get("schedule_label").cloned()).unwrap_or(Value::Null);
        event.raw.insert("schedule_label".to_owned(), restored);
        event.raw.shift_remove("default_times");
        event.raw.shift_remove("pdf_start");
        event.raw.insert("user_times".to_owned(), json!(true));
        (event.start, event.end) = (None, None);
        return Ok(());
    }
    let invalid = || "Utilisez des horaires au format HH:MM.".to_owned();
    let start = Time::parse_colon(start).ok_or_else(invalid)?;
    let end = if end.is_empty() { None } else { Some(Time::parse_colon(end).ok_or_else(invalid)?) };
    // The start on the event is the one the PDF printed.
    if truthy(event.raw.get("pdf_start")) && Some(start) != event.start {
        return Err("L’heure de début indiquée dans le PDF doit être conservée.".to_owned());
    }
    if end.is_some_and(|end| end <= start) {
        return Err("L’heure de fin doit être après l’heure de début.".to_owned());
    }
    if !event.raw.contains_key("original_schedule_label") {
        let printed = event.raw.get("schedule_label").cloned().unwrap_or(Value::Null);
        event.raw.insert("original_schedule_label".to_owned(), printed);
    }
    event.raw.insert("user_times".to_owned(), json!(true));
    event.raw.shift_remove("default_times");
    (event.start, event.end) = (Some(start), end);
    if end.is_none() {
        let label = format!("À partir de {} (fin non précisée)", start.french());
        event.raw.insert("schedule_label".to_owned(), json!(label));
    } else {
        event.raw.shift_remove("schedule_label");
    }
    Ok(())
}
