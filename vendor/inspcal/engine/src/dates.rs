//! Dates and times as the PDF prints them: `src/edt_parser/dates.py`.
//!
//! The PDFs print dates as `DD/MM` without a year: the year comes from the
//! file name (`Edt du 31.08.2026 au 04.09.2026.pdf`) or, failing that, from
//! the PDF metadata.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::errors::EdtError;
use crate::py::civil::{Date, Reading, Time};
use crate::py::text;

/// `Edt du 31.08.2026 au 04.09.2026.pdf` → both bounds of the week.
static FILENAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    text::regex(
        r"(?i)du[\s_]+(?P<d1>\d{1,2})[.\-/](?P<m1>\d{1,2})[.\-/](?P<y1>\d{4})[\s_]+au[\s_]+(?P<d2>\d{1,2})[.\-/](?P<m2>\d{1,2})[.\-/](?P<y2>\d{4})",
    )
});

/// `07/09`, `lun. 07/09`, `07/09/2026`.
static DAY_MONTH_RE: LazyLock<Regex> =
    LazyLock::new(|| text::regex(r"(?P<day>\d{1,2})\s*[/.]\s*(?P<month>\d{1,2})(?:\s*[/.]\s*(?P<year>\d{4}|\d{2}))?"));

/// `09h00`, `9h`, `09H00`, `9:00`, `09.00`.
static TIME_RE: LazyLock<Regex> = LazyLock::new(|| text::regex(r"(?P<hour>\d{1,2})\s*[hH:.]\s*(?P<minute>\d{2})?"));

/// `D:20260903184430+02'00'` in the PDF metadata.
static PDF_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    text::prefix(
        r"D:(?P<year>\d{4})(?P<month>\d{2})(?P<day>\d{2})(?:(?P<hour>\d{2})(?P<minute>\d{2})(?P<second>\d{2})?)?(?:(?P<sign>[Z+\-])(?P<offset_h>\d{2})'?(?P<offset_m>\d{2})?)?",
    )
});

/// `5 octobre`, `lundi 1er oct.`, `12 déc. 2026` — folded, so accent-free.
static MONTH_NAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    text::regex(
        r"\b(?P<day>\d{1,2})(?:er)?\s+(?P<month>janv|fevr?|mars|avr|mai|juin|juil|aout|sept?|oct|nov|dec)[a-z]*\.?(?:\s+(?P<year>\d{4}))?\b",
    )
});

const MONTHS: [(&str, i64); 14] = [
    ("janv", 1),
    ("fev", 2),
    ("fevr", 2),
    ("mars", 3),
    ("avr", 4),
    ("mai", 5),
    ("juin", 6),
    ("juil", 7),
    ("aout", 8),
    ("sep", 9),
    ("sept", 9),
    ("oct", 10),
    ("nov", 11),
    ("dec", 12),
];
const WEEKDAYS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];

fn number(found: &regex::Captures<'_>, name: &str) -> Option<i64> {
    found.name(name).filter(|digits| !digits.as_str().is_empty()).map(|digits| text::int(digits.as_str()))
}

/// `(week_start, week_end)` read from the file name, or `None`.
pub fn week_bounds_from_filename(filename: &str) -> Result<Option<(Date, Date)>, EdtError> {
    let Some(found) = FILENAME_RE.captures(filename) else { return Ok(None) };
    let bound = |year: &str, month: &str, day: &str| {
        Date::new(text::int(&found[year]), text::int(&found[month]), text::int(&found[day]))
    };
    let (Some(start), Some(end)) = (bound("y1", "m1", "d1"), bound("y2", "m2", "d2")) else {
        return Err(EdtError::date("Les dates du nom de fichier sont invalides."));
    };
    if end < start || end.days() - start.days() > 6 {
        return Err(EdtError::date("Le nom doit décrire une semaine de sept jours au maximum."));
    }
    Ok(Some((start, end)))
}

/// A PDF metadata timestamp as `(reading, minutes east of UTC)`, or `None` if
/// absent or malformed.
///
/// An offset of a day or more counts as malformed. Python's `timezone()`
/// raises on one, outside the function's own `try`, and the parse dies of it.
pub fn parse_pdf_datetime(raw: Option<&str>) -> Option<(Reading, i64)> {
    let found = PDF_DATE_RE.captures(raw.unwrap_or(""))?;
    let mut offset = 0;
    if let Some(sign) = found.name("sign").filter(|sign| sign.as_str() != "Z") {
        offset = number(&found, "offset_h")? * 60 + number(&found, "offset_m").unwrap_or(0);
        if offset >= 1440 {
            return None;
        }
        if sign.as_str() == "-" {
            offset = -offset;
        }
    }
    let date = Date::new(number(&found, "year")?, number(&found, "month")?, number(&found, "day")?)?;
    let part = |name: &str| number(&found, name).unwrap_or(0);
    Some((Reading::new(date, part("hour"), part("minute"), part("second"))?, offset))
}

/// When this PDF was last written, as `isoformat()` prints it.
pub fn revision_of(metadata: &BTreeMap<String, String>) -> Option<String> {
    let read = |key: &str| parse_pdf_datetime(metadata.get(key).map(String::as_str));
    read("ModDate").or_else(|| read("CreationDate")).map(|(reading, offset)| reading.iso_with_offset(offset))
}

/// The date that gives `DD/MM` cells their year, and whether it had to come
/// from the PDF metadata. The file name is authoritative.
pub fn reference_date(filename: &str, metadata: &BTreeMap<String, String>) -> Result<(Date, bool), EdtError> {
    if let Some((start, _)) = week_bounds_from_filename(filename)? {
        return Ok((start, false));
    }
    match parse_pdf_datetime(metadata.get("CreationDate").map(String::as_str)) {
        Some((created, _)) => Ok((created.date, true)),
        None => Err(EdtError::date("ni semaine dans le nom du fichier, ni date exploitable dans le PDF")),
    }
}

/// Parse a single `Début`/`Fin` cell.
pub fn parse_time(value: &str) -> Result<Time, EdtError> {
    let found = TIME_RE.captures(value).ok_or_else(|| EdtError::time("horaire illisible"))?;
    Time::new(text::int(&found["hour"]), number(&found, "minute").unwrap_or(0))
        .ok_or_else(|| EdtError::time("horaire hors limites"))
}

fn fold(value: &str) -> String {
    text::unaccent(value).to_lowercase()
}

/// `Lundi` with no number: only the week the file names can place it.
fn weekday_only(value: &str, week_start: Option<Date>) -> Option<Date> {
    static WORD: LazyLock<Regex> = LazyLock::new(|| text::regex("[a-z]+"));
    let week_start = week_start?;
    let folded = fold(value);
    let words: Vec<&str> = WORD.find_iter(&folded).map(|word| word.as_str()).collect();
    if words.len() != 1 || value.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let index = WEEKDAYS.iter().position(|name| words[0].len() >= 3 && name.starts_with(words[0]))?;
    Some(week_start.plus_days((index as i64 - week_start.weekday()).rem_euclid(7)))
}

/// Complete a `DD/MM` cell with the year inferred from `reference`.
///
/// A week straddling New Year is handled in both directions. The month may
/// also be written out (`lundi 5 octobre`). A bare weekday (`Lundi`) is placed
/// in the week the file name announces, and only then: `week_start` is `None`
/// when the year came from the PDF metadata.
pub fn parse_day_month(value: &str, reference: Date, week_start: Option<Date>) -> Result<Date, EdtError> {
    let folded = fold(value);
    let (day, month, year) = if let Some(found) = DAY_MONTH_RE.captures(value) {
        (text::int(&found["day"]), text::int(&found["month"]), number(&found, "year"))
    } else if let Some(found) = MONTH_NAME_RE.captures(&folded) {
        let month = MONTHS.iter().find(|(name, _)| *name == &found["month"]).map_or(0, |(_, month)| *month);
        (text::int(&found["day"]), month, number(&found, "year"))
    } else if let Some(day) = weekday_only(value, week_start) {
        return Ok(day);
    } else {
        return Err(EdtError::date("date illisible"));
    };
    let year = match year {
        Some(year) if year < 100 => year + 2000,
        Some(year) => year,
        None if month == 1 && reference.month == 12 => i64::from(reference.year) + 1,
        None if month == 12 && reference.month == 1 => i64::from(reference.year) - 1,
        None => i64::from(reference.year),
    };
    Date::new(year, month, day).ok_or_else(|| EdtError::date("date inexistante au calendrier"))
}

/// `13:30-17:00`, the form stored in `Groups.time_ranges`.
pub fn format_range(start: Time, end: Time) -> String {
    format!("{}-{}", start.colon(), end.colon())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i64, month: i64, day: i64) -> Date {
        Date::new(year, month, day).unwrap()
    }

    #[test]
    fn the_file_name_gives_the_week() {
        let bounds = week_bounds_from_filename("Edt du 31.08.2026 au 04.09.2026.pdf").unwrap();
        assert_eq!(bounds, Some((date(2026, 8, 31), date(2026, 9, 4))));
        assert_eq!(week_bounds_from_filename("planning.pdf").unwrap(), None);
        assert!(week_bounds_from_filename("edt DU 31-02-2026 au 04-03-2026.pdf").is_err());
        assert!(week_bounds_from_filename("Edt du 01.09.2026 au 30.09.2026.pdf").is_err());
    }

    #[test]
    fn times_are_read_tolerantly() {
        for (text, hour, minute) in
            [("09h00", 9, 0), ("9h", 9, 0), ("09H30", 9, 30), ("9:05", 9, 5), ("14 h 15", 14, 15)]
        {
            assert_eq!(parse_time(text).unwrap(), Time::new(hour, minute).unwrap(), "{text}");
        }
        assert_eq!(parse_time("midi").unwrap_err().message, "horaire illisible");
        assert_eq!(parse_time("25h00").unwrap_err().message, "horaire hors limites");
    }

    #[test]
    fn a_day_takes_its_year_from_the_reference() {
        let monday = date(2026, 9, 7);
        assert_eq!(parse_day_month("lun. 07/09", monday, None).unwrap(), monday);
        assert_eq!(parse_day_month("07/09/27", monday, None).unwrap(), date(2027, 9, 7));
        assert_eq!(parse_day_month("02/01", date(2026, 12, 28), None).unwrap(), date(2027, 1, 2));
        assert_eq!(parse_day_month("31/12", date(2027, 1, 1), None).unwrap(), date(2026, 12, 31));
        assert_eq!(parse_day_month("Mardi 1er déc. 2026", monday, None).unwrap(), date(2026, 12, 1));
        assert_eq!(parse_day_month("jeudi 10 septembre", monday, None).unwrap(), date(2026, 9, 10));
        assert_eq!(parse_day_month("Mercredi", monday, Some(monday)).unwrap(), date(2026, 9, 9));
        assert_eq!(parse_day_month("Mercredi", monday, None).unwrap_err().message, "date illisible");
        assert_eq!(parse_day_month("31/02", monday, None).unwrap_err().message, "date inexistante au calendrier");
    }

    #[test]
    fn pdf_metadata_dates_keep_their_offset() {
        let metadata = BTreeMap::from([
            ("CreationDate".to_owned(), "D:20260901120000Z".to_owned()),
            ("ModDate".to_owned(), "D:20260903184430+02'00'".to_owned()),
        ]);
        assert_eq!(revision_of(&metadata).unwrap(), "2026-09-03T18:44:30+02:00");
        assert_eq!(reference_date("x.pdf", &metadata).unwrap(), (date(2026, 9, 1), true));
        let only_date = BTreeMap::from([("CreationDate".to_owned(), "D:20260901".to_owned())]);
        assert_eq!(revision_of(&only_date).unwrap(), "2026-09-01T00:00:00+00:00");
        let broken = BTreeMap::from([("ModDate".to_owned(), "D:20261301".to_owned())]);
        assert_eq!(revision_of(&broken), None);
        assert_eq!(reference_date("x.pdf", &broken).unwrap_err().code, "DATE_PARSE");
    }
}
