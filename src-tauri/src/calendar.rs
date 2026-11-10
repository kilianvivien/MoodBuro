//! Explicit, local RFC 5545 snapshots. No network access or credential-bearing URLs.
use crate::AppState;
use chrono::{DateTime, Datelike, Utc};
use md_store::{Event, Site};
use std::io::Write;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

fn timestamp(seconds: i64) -> Result<String, String> {
    let date = DateTime::<Utc>::from_timestamp(seconds, 0)
        .filter(|date| (1..=9999).contains(&date.year()))
        .ok_or("Invalid calendar date range.")?;
    Ok(date.format("%Y%m%dT%H%M%SZ").to_string())
}
fn text(value: &str) -> String {
    let mut result = String::new();
    for c in value.replace("\r\n", "\n").replace('\r', "\n").chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            ';' => result.push_str("\\;"),
            ',' => result.push_str("\\,"),
            '\n' => result.push_str("\\n"),
            c if c.is_control() => {}
            c => result.push(c),
        }
    }
    result
}
/// Fold at UTF-8 boundaries, counting continuation whitespace in the 75-octet limit.
fn line(out: &mut String, value: &str) {
    let mut width = 0;
    for c in value.chars() {
        if width + c.len_utf8() > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(c);
        width += c.len_utf8();
    }
    out.push_str("\r\n");
}
fn render(events: &[Event], sites: &[Site], created: i64) -> Result<String, String> {
    let mut out = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//MoodBuro//Agenda snapshot//EN\r\nCALSCALE:GREGORIAN\r\n");
    let stamp = timestamp(created)?;
    for event in events {
        let site = sites
            .iter()
            .find(|s| s.id == event.site_id)
            .ok_or("Unknown site.")?;
        if event.id <= 0 || event.time_start < 0 || event.duration < 0 {
            return Err("Invalid calendar event.".into());
        }
        let base =
            md_moodle::normalize_site(&site.wwwroot).map_err(|_| "Invalid calendar event.")?;
        let url = md_moodle::page_url(&base, md_moodle::Page::CalendarDay(event.time_start))
            .map_err(|_| "Invalid calendar event.")?;
        // Site IDs encoded as hex cannot inject a property; account changes yield new UIDs.
        let scope: String = site.id.bytes().map(|b| format!("{b:02x}")).collect();
        line(&mut out, "BEGIN:VEVENT");
        line(
            &mut out,
            &format!("UID:{scope}-{}-{}@moodburo.local", site.user_id, event.id),
        );
        line(&mut out, &format!("DTSTAMP:{stamp}"));
        line(
            &mut out,
            &format!("DTSTART:{}", timestamp(event.time_start)?),
        );
        if event.duration > 0 {
            let end = event
                .time_start
                .checked_add(event.duration)
                .ok_or("Invalid calendar event.")?;
            line(&mut out, &format!("DTEND:{}", timestamp(end)?));
        }
        line(&mut out, &format!("SUMMARY:{}", text(&event.name)));
        let description = [Some(site.name.as_str()), event.course_name.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");
        line(&mut out, &format!("DESCRIPTION:{}", text(&description)));
        line(&mut out, &format!("URL:{url}"));
        line(&mut out, "END:VEVENT");
    }
    line(&mut out, "END:VCALENDAR");
    Ok(out)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exported {
    filename: String,
    count: usize,
    revealed: bool,
}

pub fn export(
    app: &AppHandle,
    site_id: Option<&str>,
    start: Option<i64>,
    end: Option<i64>,
) -> Result<Exported, String> {
    if start.zip(end).is_some_and(|(a, b)| a >= b) {
        return Err("Invalid calendar date range.".into());
    }
    for date in [start, end].into_iter().flatten() {
        timestamp(date)?;
    }
    let state = app.state::<AppState>();
    // Keep the account snapshot valid through publication; replacement/removal waits.
    let store = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?;
    let sites = store.sites().map_err(|_| "Could not read sites.")?;
    if site_id.is_some_and(|id| !sites.iter().any(|s| s.id == id)) {
        return Err("Unknown site.".into());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_secs() as i64);
    let events: Vec<_> = store
        .events(now - 3600)
        .map_err(|_| "Could not read the agenda.")?
        .into_iter()
        .filter(|e| {
            site_id.is_none_or(|id| id == e.site_id)
                && start.is_none_or(|s| e.time_start >= s)
                && end.is_none_or(|end| e.time_start < end)
        })
        .collect();
    if events.is_empty() {
        return Err("No calendar events match these filters.".into());
    }
    let contents = render(&events, &sites, now)?;
    let folder = app
        .path()
        .download_dir()
        .map_err(|_| "Download folder unavailable.")?;
    let filename = format!("MoodBuro-{}.ics", uuid::Uuid::new_v4());
    let path = folder.join(&filename);
    let staged = folder.join(format!(".{filename}.partial"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged)
        .map_err(|_| "Could not export the calendar.")?;
    let saved = (|| -> std::io::Result<()> {
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        // Hard link publishes atomically without replacing any existing file.
        std::fs::hard_link(&staged, &path)
    })();
    let _ = std::fs::remove_file(&staged);
    saved.map_err(|_| "Could not export the calendar.")?;
    drop(store);
    let revealed = app.opener().reveal_item_in_dir(&path).is_ok();
    Ok(Exported {
        filename,
        count: events.len(),
        revealed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixtures() -> (Vec<Event>, Vec<Site>) {
        (
            vec![Event {
                site_id: "site".into(),
                id: 42,
                name: "Échéance, cours; test\\notes\r\nBEGIN:VALARM".into(),
                time_start: 1792888200,
                duration: 7200,
                kind: "due".into(),
                course_id: Some(2),
                course_name: Some("Français".into()),
            }],
            vec![Site {
                id: "site".into(),
                user_id: 7,
                wwwroot: "https://example.org/campus/".into(),
                name: "Université".into(),
                full_name: "Private name".into(),
            }],
        )
    }
    #[test]
    fn escapes_metadata_and_exports_only_fixed_calendar_links() {
        let (events, sites) = fixtures();
        let ics = render(&events, &sites, 1792888200).unwrap();
        assert_eq!(ics, include_str!("../tests/fixtures/agenda.ics"));
        assert!(ics.contains("SUMMARY:Échéance\\, cours\\; test\\\\notes\\nBEGIN:VALARM\r\n"));
        assert!(ics.contains("URL:https://example.org/campus/calendar/view.php?view=day&time="));
        assert!(!ics.contains("\r\nBEGIN:VALARM"));
        assert!(!ics.contains("Private name"));
        assert!(ics.contains("DTSTART:20261025T003000Z\r\nDTEND:20261025T023000Z"));
    }
    #[test]
    fn stable_uids_change_only_with_account_or_event_identity() {
        let (mut events, mut sites) = fixtures();
        let uid = |e: &[Event], s: &[Site]| {
            render(e, s, 1)
                .unwrap()
                .lines()
                .find(|l| l.starts_with("UID:"))
                .unwrap()
                .to_owned()
        };
        let first = uid(&events, &sites);
        events[0].time_start += 3600;
        events[0].name = "Changed".into();
        assert_eq!(first, uid(&events, &sites));
        sites[0].user_id += 1;
        assert_ne!(first, uid(&events, &sites));
    }
    #[test]
    fn folds_utf8_and_rejects_invalid_dates_and_overflow() {
        let value = format!("SUMMARY:{}", "é🙂".repeat(80));
        let mut out = String::new();
        line(&mut out, &value);
        assert!(out.split("\r\n").all(|l| l.len() <= 75));
        assert_eq!(out.replace("\r\n ", "").trim_end(), value);
        let (mut events, sites) = fixtures();
        events[0].duration = i64::MAX;
        assert!(render(&events, &sites, 1).is_err());
        assert!(timestamp(i64::MAX).is_err());
        let (events, mut sites) = fixtures();
        sites[0].wwwroot = "https://example.org/?token=secret".into();
        assert!(render(&events, &sites, 1).is_err());
    }
}
