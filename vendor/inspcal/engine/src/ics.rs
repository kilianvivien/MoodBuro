//! ICS generation: `src/edt_parser/ics.py`, and the part of the `icalendar`
//! library it relies on.
//!
//! The output is held to Python's byte for byte: the property order, the
//! folding at 75 octets, the escaping and the `VTIMEZONE` are `icalendar`'s.
//! UIDs are deterministic for an unchanged session; times always carry
//! `TZID=Europe/Paris`; missing revision metadata uses a fixed content-derived
//! date, never the parsing clock.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use crate::errors::EdtError;
use crate::events::{is_holiday, is_registration_only};
use crate::institution::INSTITUTION;
use crate::py::civil::{self, Date};
use crate::py::text;
use crate::rooms;
use crate::timetable::{truthy, Event, Timetable};

const PRODID: &str = "-//edt-parser//INSP//FR";
pub const CALNAME_DEFAULT: &str = "INSP — Emploi du temps";
const TZID: &str = "Europe/Paris";

/// SEQUENCE counts minutes since 2020-01-01 UTC, so a PDF rewritten later
/// always sorts after an earlier one while staying inside a 32-bit integer.
const SEQUENCE_EPOCH: Date = Date { year: 2020, month: 1, day: 1 };

/// How far around the timetable the VTIMEZONE must describe the transitions.
const TZ_YEARS_BEFORE: i32 = 1;
const TZ_YEARS_AFTER: i32 = 5;

/// RFC 7986 wants a CSS3 colour name, not a hex value, so the module colour
/// is mapped to the nearest of these.
const CSS_COLORS: [(&str, [i64; 3]); 28] = [
    ("black", [0, 0, 0]),
    ("white", [255, 255, 255]),
    ("gray", [128, 128, 128]),
    ("silver", [192, 192, 192]),
    ("red", [255, 0, 0]),
    ("maroon", [128, 0, 0]),
    ("salmon", [250, 128, 114]),
    ("coral", [255, 127, 80]),
    ("orange", [255, 165, 0]),
    ("gold", [255, 215, 0]),
    ("yellow", [255, 255, 0]),
    ("khaki", [240, 230, 140]),
    ("olive", [128, 128, 0]),
    ("green", [0, 128, 0]),
    ("lime", [0, 255, 0]),
    ("teal", [0, 128, 128]),
    ("turquoise", [64, 224, 208]),
    ("aqua", [0, 255, 255]),
    ("navy", [0, 0, 128]),
    ("blue", [0, 0, 255]),
    ("cornflowerblue", [100, 149, 237]),
    ("purple", [128, 0, 128]),
    ("violet", [238, 130, 238]),
    ("plum", [221, 160, 221]),
    ("pink", [255, 192, 203]),
    ("brown", [165, 42, 42]),
    ("tan", [210, 180, 140]),
    ("beige", [245, 245, 220]),
];

/// Rendering options, as the form of the web page sets them.
#[derive(Debug, Clone)]
pub struct IcsOptions {
    pub calendar_name: String,
    pub prefix: String,
    pub alarm_minutes: Option<i64>,
    pub split_by_module: bool,
}

impl Default for IcsOptions {
    fn default() -> Self {
        Self {
            calendar_name: CALNAME_DEFAULT.to_owned(),
            prefix: String::new(),
            alarm_minutes: None,
            split_by_module: false,
        }
    }
}

/// `INSP – Ateliers de déontologie [9 groupes]`.
pub fn render_summary(event: &Event, options: &IcsOptions) -> String {
    let mut summary = format!("{}{}", options.prefix, event.title);
    if event.groups.multi && event.groups.count > 1 {
        summary += &format!(" [{} groupes]", event.groups.count);
    }
    if event.remote {
        summary += " [à distance]";
    }
    if event.end.is_none() {
        summary += " [horaire à préciser]";
    }
    summary
}

fn filled(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.is_empty())
}

/// The structured description block, readable in the three target apps.
/// An optional session the student has not opted into.
fn unconfirmed(event: &Event) -> bool {
    is_registration_only(event) && !truthy(event.raw.get("participation_confirmed"))
}

pub fn render_description(event: &Event, source_file: &str) -> String {
    let label = |default: &str| event.raw.get("schedule_label").and_then(Value::as_str).unwrap_or(default).to_owned();
    let mut lines: Vec<String> = Vec::new();
    if unconfirmed(event) {
        lines.push("Séance facultative : participation non confirmée. Votre disponibilité reste libre.".to_owned());
        lines.push(String::new());
    }
    if is_holiday(event) {
        lines.push("Jour férié : journée sans cours.".to_owned());
        lines.push(String::new());
    } else if event.end.is_none() {
        lines.push(format!("Horaire : {}", label("non précisé")));
        lines.push("Rappel sans durée définie : vérifiez le planning. Votre disponibilité reste libre.".to_owned());
        lines.push(String::new());
    }
    if truthy(event.raw.get("user_times")) {
        lines.push("Horaires complétés manuellement.".to_owned());
    } else if truthy(event.raw.get("default_times")) {
        lines.push(format!("Le PDF indique « {} » : horaire fixé par défaut, à confirmer.", label("horaire imprécis")));
    }
    if let Some(description) = filled(&event.description) {
        lines.extend(text::splitlines(description).into_iter().map(str::to_owned));
        lines.push(String::new());
    }
    if let Some(module) = filled(&event.module) {
        lines.push(format!("Module : {module}"));
    }
    if let Some(cohort) = filled(&event.cohort_label) {
        lines.push(format!("Cohorte : {cohort}"));
    }
    if !event.speakers.is_empty() {
        let heading = if event.speakers.len() > 1 { "Intervenant·es" } else { "Intervenant·e" };
        lines.push(format!("{heading} : {}", event.speakers.join(", ")));
    }
    let note = filled(&event.location_note);
    if let (Some(note), None, false) = (note, filled(&event.location), event.groups.multi) {
        lines.push(format!("Salle : {}", rooms::soften(note)));
    }
    if let Some(preparation) = filled(&event.preparation) {
        lines.push(format!("Préparation : {preparation}"));
    }
    if event.groups.multi {
        let mut detail = match event.groups.count {
            0 => "⚠ Séance en groupes".to_owned(),
            count => format!("⚠ Séance en {count} groupes"),
        };
        if !event.groups.time_ranges.is_empty() {
            // `13:30-17:00` → `13h30–17h00`.
            let ranges: Vec<String> =
                event.groups.time_ranges.iter().map(|range| range.replace(':', "h").replace('-', "–")).collect();
            detail += &format!(", horaires {}", ranges.join(" / "));
        }
        if let Some(note) = note {
            detail += &format!(", salle {}", rooms::soften(note));
        }
        lines.push(detail);
    }
    lines.extend(event.notes.iter().map(|note| format!("⚠ {note}")));
    if !event.links.is_empty() {
        lines.push("Liens :".to_owned());
        for link in &event.links {
            lines.push(match filled(&link.label) {
                Some(label) => format!(" - {label} : {}", link.url),
                None => format!(" - {}", link.url),
            });
        }
    }
    if !source_file.is_empty() {
        lines.push(match event.raw.get("page").filter(|page| truthy(Some(page))) {
            Some(page) => format!("Source : {source_file} (page {page})"),
            None => format!("Source : {source_file}"),
        });
    }
    lines.join("\n")
}

fn nearest_css_color(hex: Option<&str>) -> Option<&'static str> {
    static HEX: LazyLock<Regex> = LazyLock::new(|| text::full("#[0-9A-Fa-f]{6}"));
    let hex = hex.filter(|hex| HEX.is_match(hex))?;
    let channel = |index: usize| i64::from_str_radix(&hex[index..index + 2], 16).unwrap_or(0);
    let target = [channel(1), channel(3), channel(5)];
    let distance = |reference: &[i64; 3]| (0..3).map(|index| (target[index] - reference[index]).pow(2)).sum::<i64>();
    CSS_COLORS.iter().min_by_key(|(_, reference)| distance(reference)).map(|(name, _)| *name)
}

/// When the PDF was last written, if its metadata said.
fn revision(timetable: &Timetable) -> Option<i64> {
    civil::parse_iso_moment(timetable.source.revision.as_deref().unwrap_or(""))
}

/// A deterministic date when PDF revision metadata is unavailable.
fn stamp(timetable: &Timetable) -> i64 {
    revision(timetable).unwrap_or_else(|| {
        let earliest = timetable.events.iter().map(|event| event.date).min();
        timetable.source.week_start.or(earliest).unwrap_or(SEQUENCE_EPOCH).midnight()
    })
}

fn minutes(moment: i64) -> i64 {
    (moment - SEQUENCE_EPOCH.midnight()).div_euclid(60).max(0)
}

/// Minutes since the epoch, so a later revision always wins.
///
/// Two revisions are added, not compared. A choice made in the browser — an
/// LV1 room, a group — changes the event while leaving the PDF untouched, so
/// the PDF's own date cannot carry it. Taking the later of the two would not
/// work either: a timetable is published before the week it describes, so the
/// PDF's date normally sits in the future, where no clock can overtake it.
fn sequence(timetable: &Timetable) -> i64 {
    let base = minutes(stamp(timetable));
    timetable.revised_at.map_or(base, |revised| base + minutes(revised))
}

/// The link a reader most likely wants: preparation first, then module.
fn primary_url(event: &Event) -> Option<&str> {
    ["preparation", "note", "theme", "module", "speakers"]
        .iter()
        .find_map(|column| event.links.iter().find(|link| link.column.as_deref() == Some(column)))
        .or(event.links.first())
        .map(|link| link.url.as_str())
}

/// TEXT escaping, in the order `icalendar` applies it.
fn escape(value: &str) -> String {
    value
        .replace("\\N", "\n")
        .replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace("\r\n", "\\n")
        .replace(['\n', '\r'], "\\n")
}

/// Fold a content line the way `icalendar` does: under 75 octets a line,
/// never inside a character, never between a backslash and what it escapes.
fn fold(line: &str, out: &mut String) {
    let mut current = String::new();
    for c in line.chars() {
        if !current.is_empty() && current.len() + c.len_utf8() >= 75 {
            let carried = match current.chars().next_back() {
                Some(last @ ('\\' | '^')) if current.len() > 1 => {
                    current.pop();
                    Some(last)
                }
                _ => None,
            };
            out.push_str(&current);
            out.push_str("\r\n ");
            current.clear();
            current.extend(carried);
        }
        current.push(c);
    }
    out.push_str(&current);
    out.push_str("\r\n");
}

/// A component being written: its properties are sorted the way `icalendar`
/// sorts them when the component is closed.
struct Component {
    name: &'static str,
    /// Names that come first, in this order; the others follow alphabetically.
    canonical: &'static [&'static str],
    properties: Vec<(String, String)>,
    children: Vec<Component>,
}

impl Component {
    fn new(name: &'static str, canonical: &'static [&'static str]) -> Self {
        Self { name, canonical, properties: Vec::new(), children: Vec::new() }
    }

    /// A property whose value is already in its iCalendar form. `name` may
    /// carry parameters (`DTSTART;TZID=Europe/Paris`).
    fn raw(&mut self, name: &str, value: impl Into<String>) {
        self.properties.push((name.to_owned(), value.into()));
    }

    fn text(&mut self, name: &str, value: &str) {
        self.raw(name, escape(value));
    }

    /// A property `icalendar` knows no type for — every `X-` one — which it
    /// writes as it stands, commas and semicolons included. A line break would
    /// make it refuse the whole calendar; here it is escaped instead.
    fn unknown(&mut self, name: &str, value: &str) {
        self.raw(name, value.replace("\r\n", "\\n").replace(['\n', '\r'], "\\n"));
    }

    fn write(mut self, out: &mut String) {
        fold(&format!("BEGIN:{}", self.name), out);
        let rank = |name: &str| {
            let bare = name.split(';').next().unwrap_or(name);
            (self.canonical.iter().position(|known| *known == bare).unwrap_or(usize::MAX), bare.to_owned())
        };
        self.properties.sort_by_cached_key(|(name, _)| rank(name));
        for (name, value) in &self.properties {
            fold(&format!("{name}:{value}"), out);
        }
        for child in self.children {
            child.write(out);
        }
        fold(&format!("END:{}", self.name), out);
    }
}

const EVENT_ORDER: [&str; 11] = [
    "SUMMARY",
    "DTSTART",
    "DTEND",
    "DURATION",
    "DTSTAMP",
    "UID",
    "RECURRENCE-ID",
    "SEQUENCE",
    "RRULE",
    "RDATE",
    "EXDATE",
];
const CALENDAR_ORDER: [&str; 8] =
    ["VERSION", "PRODID", "CALSCALE", "METHOD", "DESCRIPTION", "X-WR-CALDESC", "NAME", "X-WR-CALNAME"];

/// A `timedelta` of minus `minutes`, as `icalendar` writes a duration.
fn trigger(minutes: i64) -> String {
    let (days, hours, rest) = (minutes / 1440, minutes % 1440 / 60, minutes % 60);
    let mut time = String::new();
    if hours > 0 {
        time += &format!("{hours}H");
    }
    if rest > 0 {
        time += &format!("{rest}M");
    }
    let sign = if minutes > 0 { "-" } else { "" };
    match (days, time.is_empty()) {
        (0, false) => format!("{sign}PT{time}"),
        (_, true) => format!("{sign}P{days}D"),
        (_, false) => format!("{sign}P{days}DT{time}"),
    }
}

fn vevent(event: &Event, timetable: &Timetable, options: &IcsOptions, cancelled: bool) -> Component {
    let mut component = Component::new("VEVENT", &EVENT_ORDER);
    let uid = event.calendar_uid();
    component.text("UID", &uid);
    component.raw("DTSTAMP", civil::stamp(stamp(timetable)));
    component.raw("LAST-MODIFIED", civil::stamp(timetable.revised_at.unwrap_or_else(|| stamp(timetable))));
    component.raw("SEQUENCE", sequence(timetable).to_string());
    // The identity this session had under the content-hash scheme, kept for
    // one release so an older import can still be matched up by hand.
    let legacy = event.legacy_uid();
    if legacy != uid {
        component.unknown("X-INSPCAL-LEGACY-UID", &legacy);
    }
    match event.start {
        None => {
            component.raw("DTSTART;VALUE=DATE", event.date.compact());
            component.raw("DTEND;VALUE=DATE", event.last_date().plus_days(1).compact());
        }
        Some(start) => {
            let local = |time: civil::Time| format!("{}T{}", event.date.compact(), time.compact());
            component.raw(&format!("DTSTART;TZID={TZID}"), local(start));
            if let Some(end) = event.end {
                component.raw(&format!("DTEND;TZID={TZID}"), local(end));
            }
        }
    }
    component.text("SUMMARY", &render_summary(event, options));
    component.text("DESCRIPTION", &render_description(event, &timetable.source.file));
    let tentative = unconfirmed(event);
    let status = if cancelled {
        "CANCELLED"
    } else if tentative {
        "TENTATIVE"
    } else {
        "CONFIRMED"
    };
    component.raw("STATUS", status);
    component.raw("TRANSP", if event.end.is_none() || tentative { "TRANSPARENT" } else { "OPAQUE" });
    if let Some(location) = filled(&event.location) {
        component.text("LOCATION", location);
    }
    if let Some(note) = filled(&event.location_note) {
        component.unknown("X-EDT-LOCATION-NOTE", note);
    }
    if let Some(module) = filled(&event.module) {
        component.text("CATEGORIES", module);
    }
    if let Some(color) = nearest_css_color(event.module_color.as_deref()) {
        component.raw("COLOR", color);
    }
    if let Some(url) = primary_url(event).filter(|url| !url.is_empty()) {
        component.raw("URL", url);
    }
    if !event.cohorts.is_empty() {
        component.unknown("X-EDT-COHORTS", &event.cohorts.join(","));
    }
    if event.groups.multi {
        component.raw("X-EDT-GROUPS", event.groups.count.to_string());
    }
    if let (Some(minutes), Some(_)) = (options.alarm_minutes, event.start) {
        let mut alarm = Component::new("VALARM", &[]);
        alarm.raw("ACTION", "DISPLAY");
        alarm.text("DESCRIPTION", &event.title);
        alarm.raw("TRIGGER", trigger(minutes));
        component.children.push(alarm);
    }
    component
}

/// The last Sunday of a month of 31 days, when Europe changes its clocks.
fn last_sunday(year: i32, month: u32) -> Date {
    let end = Date { year, month, day: 31 };
    end.plus_days(-((end.weekday() + 1) % 7))
}

/// The `VTIMEZONE` `icalendar` builds from tzdata for `Europe/Paris`, covering
/// the years the calendar can reach.
///
/// No timezone database is shipped: the rule is the European Union's since
/// 1996 — clocks change at 01:00 UTC on the last Sundays of March and October.
/// Python's output is the same for any timetable from 1997 on.
fn vtimezone(first: Date, last: Date) -> Component {
    let years = first.year - TZ_YEARS_BEFORE..last.year + TZ_YEARS_AFTER;
    let period = |name: &'static str, start: String, later: Vec<String>, tzname: &str, from: &str, to: &str| {
        let mut component = Component::new(name, &[]);
        component.raw("DTSTART", start);
        if !later.is_empty() {
            component.raw("RDATE", later.join(","));
        }
        component.raw("TZNAME", tzname);
        component.raw("TZOFFSETFROM", from);
        component.raw("TZOFFSETTO", to);
        component
    };
    let changes = |month: u32| -> Vec<String> {
        years.clone().map(|year| format!("{}T030000", last_sunday(year, month).compact())).collect()
    };
    let (mut spring, mut autumn) = (changes(3), changes(10));
    let mut component = Component::new("VTIMEZONE", &["TZID"]);
    component.raw("TZID", TZID);
    let opening = format!("{:04}0101T000000", years.start);
    component.children.push(period("STANDARD", opening, Vec::new(), "CET", "+0100", "+0100"));
    if !spring.is_empty() {
        component.children.push(period("DAYLIGHT", spring.remove(0), spring, "CEST", "+0100", "+0200"));
        component.children.push(period("STANDARD", autumn.remove(0), autumn, "CET", "+0200", "+0100"));
    }
    component
}

/// One event and the parsed week it comes from, as positions in the list of
/// timetables: a calendar can merge several weeks, so an event carries its own
/// provenance rather than the calendar's.
type Entry<'a> = (&'a Event, &'a Timetable);

fn calendar(
    entries: &[Entry],
    timetables: &[Timetable],
    options: &IcsOptions,
    name: &str,
    cancelled: &[String],
) -> Vec<u8> {
    let mut calendar = Component::new("VCALENDAR", &CALENDAR_ORDER);
    calendar.raw("PRODID", PRODID);
    calendar.raw("VERSION", "2.0");
    calendar.raw("CALSCALE", "GREGORIAN");
    calendar.raw("METHOD", "PUBLISH");
    calendar.unknown("X-WR-CALNAME", name);
    calendar.raw("X-WR-TIMEZONE", TZID);
    // The institution facts this export was produced under, so that a calendar
    // that reads oddly can be traced to the rules that produced it.
    calendar.unknown("X-INSPCAL-RULES", &INSTITUTION.version);
    let mut promotions: Vec<&str> =
        timetables.iter().filter_map(|timetable| filled(&timetable.source.promotion)).collect();
    promotions.sort_unstable();
    promotions.dedup();
    if !promotions.is_empty() {
        let label = if promotions.len() == 1 { "Promotion" } else { "Promotions" };
        calendar.unknown("X-WR-CALDESC", &format!("{label} {}", promotions.join(", ")));
    }
    let anchors: Vec<Date> = entries
        .iter()
        .map(|(event, _)| event.date)
        .chain(timetables.iter().filter_map(|timetable| timetable.source.week_start))
        .collect();
    let first = anchors.iter().copied().min().unwrap_or(SEQUENCE_EPOCH);
    let last = anchors.iter().copied().max().unwrap_or(first);
    calendar.children.push(vtimezone(first, last));
    let mut ordered: Vec<&Entry> = entries.iter().collect();
    ordered.sort_by_cached_key(|entry| (entry.0.date, entry.0.sort_time(), entry.0.uid()));
    for &(event, origin) in ordered {
        calendar.children.push(vevent(event, origin, options, cancelled.contains(&event.calendar_uid())));
    }
    let mut out = String::new();
    calendar.write(&mut out);
    out.into_bytes()
}

fn week_of(timetable: &Timetable) -> (Option<Date>, Option<Date>, Option<&str>) {
    (timetable.source.week_start, timetable.source.week_end, timetable.source.promotion.as_deref())
}

fn content(timetable: &Timetable) -> Vec<Value> {
    timetable.events.iter().map(Event::to_json).collect()
}

/// Select whole weekly snapshots: `(selected, conflicts)`, as positions in
/// `timetables`.
///
/// A weekly INSP PDF is a complete snapshot for its promotion, not a patch.
/// Revision order is never inferred from upload order, file names or parsing
/// time.
pub fn resolve_revisions(timetables: &[Timetable]) -> (Vec<usize>, Vec<usize>) {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (index, timetable) in timetables.iter().enumerate() {
        let dated = timetable.source.week_start.is_some() && timetable.source.week_end.is_some();
        let known = groups.iter_mut().find(|group| {
            let other = &timetables[group[0]];
            dated
                && other.source.week_start.is_some()
                && other.source.week_end.is_some()
                && week_of(other) == week_of(timetable)
        });
        match known {
            Some(group) => group.push(index),
            None => groups.push(vec![index]),
        }
    }
    let (mut selected, mut conflicts) = (Vec::new(), Vec::new());
    for group in groups {
        if group.len() == 1 {
            selected.extend(group);
            continue;
        }
        // The first of the greatest, or of the least, as `max` and `min` do.
        let first_by = |items: &[usize], better: &dyn Fn(usize, usize) -> bool| {
            items
                .iter()
                .copied()
                .reduce(|best, item| if better(item, best) { item } else { best })
                .expect("a group is never empty")
        };
        let file = |index: usize| timetables[index].source.file.as_str();
        // Parsed content is compared, not hashes: metadata-only rewrites are harmless.
        let contents: Vec<Vec<Value>> = group.iter().map(|&index| content(&timetables[index])).collect();
        let revisions: Vec<Option<i64>> = group.iter().map(|&index| revision(&timetables[index])).collect();
        if contents.iter().all(|item| *item == contents[0]) {
            let key = |index: usize| (revision(&timetables[index]).unwrap_or(SEQUENCE_EPOCH.midnight()), file(index));
            selected.push(first_by(&group, &|item, best| key(item) > key(best)));
        } else if let Some(latest) =
            revisions.iter().copied().collect::<Option<Vec<i64>>>().and_then(|all| all.into_iter().max())
        {
            let newest: Vec<usize> = (0..group.len()).filter(|&position| revisions[position] == Some(latest)).collect();
            if newest.iter().all(|&position| contents[position] == contents[newest[0]]) {
                let files: Vec<usize> = newest.iter().map(|&position| group[position]).collect();
                selected.push(first_by(&files, &|item, best| file(item) < file(best)));
            } else {
                conflicts.extend(group);
            }
        } else {
            conflicts.extend(group);
        }
    }
    (selected, conflicts)
}

/// Every event of the selected weeks, each with the week it comes from.
pub fn merge(timetables: &[Timetable]) -> Result<Vec<Entry<'_>>, EdtError> {
    let (selected, conflicts) = resolve_revisions(timetables);
    if !conflicts.is_empty() {
        return Err(EdtError::revision_conflict(
            "Plusieurs versions de la même semaine ne peuvent pas être départagées. Conservez uniquement le PDF à utiliser.",
        ));
    }
    Ok(selected
        .into_iter()
        .flat_map(|index| timetables[index].events.iter().map(move |event| (event, &timetables[index])))
        .collect())
}

/// Sessions a newer revision of a week dropped, as tombstones to export.
///
/// A stable UID makes a corrected session update the one already imported. It
/// does nothing for a session the correction removed outright: `STATUS`
/// `CANCELLED` is how a calendar is told to drop one. Only knowable when both
/// revisions are in the same export. Each tombstone is paired with the kept
/// week, so it carries that week's `SEQUENCE`, strictly higher than the
/// revision the reader imported the session from.
fn cancellations(timetables: &[Timetable]) -> Vec<Entry<'_>> {
    let (selected, conflicts) = resolve_revisions(timetables);
    if !conflicts.is_empty() {
        return Vec::new();
    }
    let mut dead = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (index, timetable) in timetables.iter().enumerate() {
        if selected.contains(&index) {
            continue;
        }
        // Superseded by the last selected week of the same name, if any.
        let Some(winner) =
            selected.iter().rev().map(|&kept| &timetables[kept]).find(|kept| week_of(kept) == week_of(timetable))
        else {
            continue;
        };
        let live: Vec<String> = winner.events.iter().map(Event::calendar_uid).collect();
        for event in &timetable.events {
            let uid = event.calendar_uid();
            if live.contains(&uid) || seen.contains(&uid) {
                continue;
            }
            seen.push(uid);
            dead.push((event, winner));
        }
    }
    dead
}

/// Whether two timetables describe the same week of the same promotion.
pub fn same_week(one: &Timetable, other: &Timetable) -> bool {
    week_of(one) == week_of(other)
}

/// Sessions the reader exported earlier that the same week no longer holds,
/// each with the position in `timetables` of the week that replaces it.
///
/// `exported` is what the reader's device kept of its last export of a week:
/// the sessions as they went into the calendar, choices applied. Whatever is
/// missing from the week now on screen — dropped by a correction, left out by
/// a choice — has to be cancelled, or it stays in the calendar for ever. The
/// caller sends only weeks exported under the same cohort filter: a session
/// of another cohort is not one that went away.
pub fn dropped<'a>(timetables: &[Timetable], exported: &'a [Timetable]) -> Vec<(usize, &'a Event)> {
    let (selected, conflicts) = resolve_revisions(timetables);
    if !conflicts.is_empty() {
        return Vec::new();
    }
    let mut dead = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for earlier in exported {
        let Some(&winner) = selected.iter().rev().find(|&&kept| same_week(&timetables[kept], earlier)) else {
            continue;
        };
        let live: Vec<String> = timetables[winner].events.iter().map(Event::calendar_uid).collect();
        for event in &earlier.events {
            let uid = event.calendar_uid();
            if live.contains(&uid) || seen.contains(&uid) {
                continue;
            }
            seen.push(uid);
            dead.push((winner, event));
        }
    }
    dead
}

/// `Parcours transition écologique` → `parcours-transition-ecologique`.
///
/// Python folds with `unidecode`, which transliterates every script. Accents
/// and the Latin ligatures are folded the same here; any other character
/// outside ASCII separates two words, where `unidecode` might spell it out.
/// Only the names of the files inside a split archive can differ for it.
pub fn slugify(value: &str) -> String {
    let mut folded = String::new();
    for c in text::unaccent(value).chars() {
        match c {
            'œ' | 'Œ' => folded += "oe",
            'æ' | 'Æ' => folded += "ae",
            'ß' => folded += "ss",
            'ø' | 'Ø' => folded.push('o'),
            'đ' | 'Đ' | 'ð' | 'Ð' => folded.push('d'),
            'ł' | 'Ł' => folded.push('l'),
            '€' => folded += "eur",
            '°' => folded += "deg",
            c if c.is_ascii_alphanumeric() => folded.push(c.to_ascii_lowercase()),
            _ => folded.push('-'),
        }
    }
    let slug = folded.split('-').filter(|part| !part.is_empty()).collect::<Vec<_>>().join("-");
    if slug.is_empty() {
        "sans-module".to_owned()
    } else {
        slug
    }
}

/// Serialise several parsed weeks: `(file-name slug, calendar)` pairs, the
/// slug empty when a single calendar is produced, one calendar per module
/// otherwise.
pub fn to_calendars_multi(timetables: &[Timetable], options: &IcsOptions) -> Result<Vec<(String, Vec<u8>)>, EdtError> {
    to_calendars_after(timetables, &[], options)
}

/// `to_calendars_multi`, for a reader who exported some of these weeks
/// before: what `dropped` finds missing since is cancelled too.
pub fn to_calendars_after(
    timetables: &[Timetable],
    exported: &[Timetable],
    options: &IcsOptions,
) -> Result<Vec<(String, Vec<u8>)>, EdtError> {
    let mut entries = merge(timetables)?;
    // A session the newer revision dropped is exported too, as a tombstone: a
    // stable UID updates what moved, and only STATUS:CANCELLED removes what
    // went away.
    let mut dead = cancellations(timetables);
    for (winner, event) in dropped(timetables, exported) {
        if !dead.iter().any(|(known, _)| known.calendar_uid() == event.calendar_uid()) {
            dead.push((event, &timetables[winner]));
        }
    }
    let cancelled: Vec<String> = dead.iter().map(|(event, _)| event.calendar_uid()).collect();
    entries.extend(dead);
    if !options.split_by_module {
        return Ok(vec![(String::new(), calendar(&entries, timetables, options, &options.calendar_name, &cancelled))]);
    }
    let mut grouped: Vec<(&str, Vec<Entry>)> = Vec::new();
    for entry in entries {
        let module = filled(&entry.0.module).unwrap_or("Sans module");
        match grouped.iter_mut().find(|(known, _)| *known == module) {
            Some((_, group)) => group.push(entry),
            None => grouped.push((module, vec![entry])),
        }
    }
    grouped.sort_by_key(|(module, _)| *module);
    let mut calendars: Vec<(String, Vec<u8>)> = Vec::new();
    for (module, group) in grouped {
        let base = slugify(module);
        let mut slug = base.clone();
        let mut index = 2;
        while calendars.iter().any(|(known, _)| *known == slug) {
            slug = format!("{base}-{index}");
            index += 1;
        }
        let name = format!("{} — {module}", options.calendar_name);
        calendars.push((slug, calendar(&group, timetables, options, &name, &cancelled)));
    }
    Ok(calendars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_escaped_in_icalendars_order() {
        assert_eq!(escape("a, b; c\\d\r\ne\nf"), "a\\, b\\; c\\\\d\\ne\\nf");
    }

    #[test]
    fn long_lines_fold_under_75_octets_between_characters() {
        let mut out = String::new();
        fold(&format!("DESCRIPTION:{}", "é".repeat(60)), &mut out);
        let lines: Vec<&str> = out.trim_end().split("\r\n").collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].len(), 74);
        assert!(lines[1].starts_with(' ') && lines[1].len() <= 75);
        assert_eq!(lines.concat().replace(' ', ""), format!("DESCRIPTION:{}", "é".repeat(60)));
    }

    #[test]
    fn a_fold_never_parts_a_backslash_from_what_it_escapes() {
        let mut out = String::new();
        fold(&format!("{}\\,suite", "x".repeat(73)), &mut out);
        assert_eq!(out, format!("{}\r\n \\,suite\r\n", "x".repeat(73)));
    }

    #[test]
    fn durations_are_written_like_icalendar() {
        assert_eq!(trigger(15), "-PT15M");
        assert_eq!(trigger(90), "-PT1H30M");
        assert_eq!(trigger(60), "-PT1H");
        assert_eq!(trigger(1440), "-P1D");
        assert_eq!(trigger(0), "P0D");
    }

    #[test]
    fn clocks_change_on_the_last_sunday() {
        assert_eq!(last_sunday(2026, 3).iso(), "2026-03-29");
        assert_eq!(last_sunday(2026, 10).iso(), "2026-10-25");
        assert_eq!(last_sunday(2030, 3).iso(), "2030-03-31");
    }

    #[test]
    fn colours_and_slugs() {
        assert_eq!(nearest_css_color(Some("#FE0102")), Some("red"));
        assert_eq!(nearest_css_color(Some("#FFF")), None);
        assert_eq!(nearest_css_color(None), None);
        assert_eq!(slugify("Parcours transition écologique"), "parcours-transition-ecologique");
        assert_eq!(slugify("L’État & l’Œuvre — 2"), "l-etat-l-oeuvre-2");
        assert_eq!(slugify("—"), "sans-module");
    }
}
