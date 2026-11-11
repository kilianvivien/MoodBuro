//! Calendar dates and wall-clock times: the part of `datetime` the parser uses.
//!
//! A moment in time is a count of seconds since 1970 in UTC, as an `i64`.

use std::fmt;
use std::sync::LazyLock;

use regex::Regex;

use super::text;

/// A day of the proleptic Gregorian calendar, year 1 to 9999 like `datetime.date`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 31,
    }
}

impl Date {
    /// `date(year, month, day)`, or `None` where Python raises `ValueError`.
    pub fn new(year: i64, month: i64, day: i64) -> Option<Self> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return None;
        }
        let (year, month) = (year as i32, month as u32);
        (1..=i64::from(days_in_month(year, month))).contains(&day).then_some(Self { year, month, day: day as u32 })
    }

    /// Days since 1970-01-01.
    pub fn days(self) -> i64 {
        let year = i64::from(self.year) - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year.rem_euclid(400);
        let month = i64::from(self.month);
        let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// The day `days` after 1970-01-01, kept inside the years a date can hold.
    pub fn from_days(days: i64) -> Self {
        let days =
            days.clamp(Self { year: 1, month: 1, day: 1 }.days(), Self { year: 9999, month: 12, day: 31 }.days());
        let shifted = days + 719_468;
        let era = shifted.div_euclid(146_097);
        let day_of_era = shifted.rem_euclid(146_097);
        let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
        let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Self { year: year as i32, month: month as u32, day: day as u32 }
    }

    pub fn plus_days(self, days: i64) -> Self {
        Self::from_days(self.days() + days)
    }

    /// `date.weekday()`: Monday is 0.
    pub fn weekday(self) -> i64 {
        (self.days() + 3).rem_euclid(7)
    }

    /// `date.fromisoformat`, for the one form this program writes: `YYYY-MM-DD`.
    pub fn parse_iso(value: &str) -> Option<Self> {
        static ISO: LazyLock<Regex> = LazyLock::new(|| text::full(r"(\d{4})-(\d{2})-(\d{2})"));
        let found = ISO.captures(value)?;
        Self::new(text::int(&found[1]), text::int(&found[2]), text::int(&found[3]))
    }

    /// `2026-09-07`.
    pub fn iso(self) -> String {
        self.to_string()
    }

    /// `07/09`.
    pub fn day_month(self) -> String {
        format!("{:02}/{:02}", self.day, self.month)
    }

    /// `07.09.2026`.
    pub fn dotted(self) -> String {
        format!("{:02}.{:02}.{:04}", self.day, self.month, self.year)
    }

    /// `20260907`, the iCalendar form.
    pub fn compact(self) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day)
    }

    /// Midnight UTC on this day.
    pub fn midnight(self) -> i64 {
        self.days() * 86_400
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// A time of day to the minute, which is all a timetable prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
}

impl Time {
    pub const MIDNIGHT: Self = Self { hour: 0, minute: 0 };

    pub fn new(hour: i64, minute: i64) -> Option<Self> {
        ((0..24).contains(&hour) && (0..60).contains(&minute))
            .then_some(Self { hour: hour as u32, minute: minute as u32 })
    }

    /// `HH:MM` exactly, the form the pivot JSON and the review page use.
    pub fn parse_colon(value: &str) -> Option<Self> {
        static COLON: LazyLock<Regex> = LazyLock::new(|| text::full(r"(\d{2}):(\d{2})"));
        let found = COLON.captures(value)?;
        Self::new(text::int(&found[1]), text::int(&found[2]))
    }

    /// `13:30`.
    pub fn colon(self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// `13h30`, the way the PDF writes times.
    pub fn french(self) -> String {
        format!("{:02}h{:02}", self.hour, self.minute)
    }

    /// `133000`, the iCalendar form.
    pub fn compact(self) -> String {
        format!("{:02}{:02}00", self.hour, self.minute)
    }

    /// This time `minutes` later, wrapping at midnight as `datetime.time()` does.
    pub fn plus_minutes(self, minutes: u32) -> Self {
        let total = (self.hour * 60 + self.minute + minutes) % 1440;
        Self { hour: total / 60, minute: total % 60 }
    }
}

/// A calendar reading with seconds, used for PDF metadata and time stamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub date: Date,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl Reading {
    pub fn new(date: Date, hour: i64, minute: i64, second: i64) -> Option<Self> {
        ((0..24).contains(&hour) && (0..60).contains(&minute) && (0..60).contains(&second)).then_some(Self {
            date,
            hour: hour as u32,
            minute: minute as u32,
            second: second as u32,
        })
    }

    /// The moment this reading names at `offset_minutes` east of UTC.
    pub fn at_offset(self, offset_minutes: i64) -> i64 {
        self.date.midnight() + i64::from(self.hour * 3600 + self.minute * 60 + self.second) - offset_minutes * 60
    }

    /// What a UTC clock reads at `moment`.
    pub fn of(moment: i64) -> Self {
        let seconds = moment.rem_euclid(86_400) as u32;
        Self {
            date: Date::from_days(moment.div_euclid(86_400)),
            hour: seconds / 3600,
            minute: seconds % 3600 / 60,
            second: seconds % 60,
        }
    }

    fn clock(self) -> String {
        format!("{}T{:02}:{:02}:{:02}", self.date, self.hour, self.minute, self.second)
    }

    /// `datetime.isoformat()` for an aware value: `2026-09-03T18:44:30+02:00`.
    pub fn iso_with_offset(self, offset_minutes: i64) -> String {
        let sign = if offset_minutes < 0 { '-' } else { '+' };
        let size = offset_minutes.abs();
        format!("{}{sign}{:02}:{:02}", self.clock(), size / 60, size % 60)
    }
}

/// `datetime.isoformat(timespec="seconds")` of a UTC moment.
pub fn iso_utc(moment: i64) -> String {
    Reading::of(moment).iso_with_offset(0)
}

/// `20260917T090000Z`, the iCalendar form of a UTC moment.
pub fn stamp(moment: i64) -> String {
    let reading = Reading::of(moment);
    format!("{}T{:02}{:02}{:02}Z", reading.date.compact(), reading.hour, reading.minute, reading.second)
}

/// `datetime.fromisoformat`, for the forms `isoformat` writes. A value with no
/// offset is taken as UTC, which is what `ics._revision` does with it.
pub fn parse_iso_moment(value: &str) -> Option<i64> {
    static ISO: LazyLock<Regex> = LazyLock::new(|| {
        text::full(
            r"(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2}))?(?:\.\d{1,6})?(?:(Z)|([+-])(\d{2}):(\d{2}))?",
        )
    });
    let found = ISO.captures(value)?;
    let number = |index: usize| found.get(index).map_or(0, |digits| text::int(digits.as_str()));
    let date = Date::new(number(1), number(2), number(3))?;
    let reading = Reading::new(date, number(4), number(5), number(6))?;
    let mut offset = number(9) * 60 + number(10);
    if number(10) >= 60 || offset >= 1440 {
        return None;
    }
    if found.get(8).is_some_and(|sign| sign.as_str() == "-") {
        offset = -offset;
    }
    Some(reading.at_offset(offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i64, month: i64, day: i64) -> Date {
        Date::new(year, month, day).unwrap()
    }

    #[test]
    fn dates_round_trip_through_day_counts() {
        assert_eq!(date(1970, 1, 1).days(), 0);
        assert_eq!(date(2026, 9, 7).days(), 20_703);
        for days in [-719_162, -1, 0, 59, 60, 11_016, 20_703, 2_932_896] {
            assert_eq!(Date::from_days(days).days(), days);
        }
        assert_eq!(date(2024, 2, 28).plus_days(2), date(2024, 3, 1));
        assert_eq!(date(2026, 12, 31).plus_days(1), date(2027, 1, 1));
    }

    #[test]
    fn impossible_dates_are_refused() {
        assert!(Date::new(2026, 2, 29).is_none());
        assert!(Date::new(2024, 2, 29).is_some());
        assert!(Date::new(0, 1, 1).is_none());
        assert!(Date::new(2026, 13, 1).is_none());
        assert!(Date::parse_iso("2026-9-7").is_none());
        assert_eq!(Date::parse_iso("2026-09-07"), Some(date(2026, 9, 7)));
    }

    #[test]
    fn monday_is_zero() {
        assert_eq!(date(2026, 9, 7).weekday(), 0);
        assert_eq!(date(2026, 9, 13).weekday(), 6);
        assert_eq!(date(1970, 1, 1).weekday(), 3);
    }

    #[test]
    fn moments_follow_their_offset() {
        let reading = Reading::new(date(2026, 9, 3), 18, 44, 30).unwrap();
        let moment = reading.at_offset(120);
        assert_eq!(stamp(moment), "20260903T164430Z");
        assert_eq!(reading.iso_with_offset(120), "2026-09-03T18:44:30+02:00");
        assert_eq!(reading.iso_with_offset(-330), "2026-09-03T18:44:30-05:30");
        assert_eq!(parse_iso_moment("2026-09-03T18:44:30+02:00"), Some(moment));
        assert_eq!(parse_iso_moment("2026-09-03T16:44:30"), Some(moment));
        assert_eq!(parse_iso_moment("jeudi"), None);
        assert_eq!(iso_utc(moment), "2026-09-03T16:44:30+00:00");
    }

    #[test]
    fn a_time_wraps_at_midnight() {
        let late = Time::new(23, 30).unwrap();
        assert_eq!(late.plus_minutes(120), Time::new(1, 30).unwrap());
        assert_eq!(late.french(), "23h30");
        assert_eq!(Time::parse_colon("09:05").unwrap().colon(), "09:05");
        assert!(Time::parse_colon("9:05").is_none() && Time::parse_colon("24:00").is_none());
    }
}
