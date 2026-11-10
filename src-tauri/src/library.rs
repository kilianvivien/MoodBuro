//! Account-bound metadata behind the local Inbox, Courses and Agenda views.
//! Tokens stay in Rust; only bounded plain-text names reach SQLite and the shell.
use crate::AppState;
use md_moodle::{normalize_site, plain_text, Client, Page};
use tauri::{AppHandle, Manager};

const MAX_COURSES: usize = 500;
const MAX_EVENTS: usize = 200;
const NAME: usize = 200;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing<T> {
    items: Vec<T>,
    /// Last successful refresh per organisation; absent means never refreshed.
    refreshed: Vec<md_store::Refreshed>,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}
fn site(app: &AppHandle, id: &str) -> Result<md_store::Site, String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .sites()
        .map_err(|_| "Could not read sites.")?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or("Unknown site.".into())
}
/// The saved account, a client for its base URL and its Keychain token.
pub(crate) fn session(
    app: &AppHandle,
    id: &str,
) -> Result<
    (
        md_store::Site,
        Client,
        impl std::ops::Deref<Target = String>,
    ),
    String,
> {
    let site = site(app, id)?;
    let base = normalize_site(&site.wwwroot).map_err(|e| e.to_string())?;
    let client = Client::new(base).map_err(|e| e.to_string())?;
    let token = md_secrets::get(id, md_secrets::Kind::Token)
        .map_err(|_| "Reconnect this organisation in Settings to restore its credentials.")?;
    Ok((site, client, token))
}

pub fn page(name: &str, value: Option<i64>) -> Result<Page, String> {
    Ok(match (name, value) {
        ("notifications", None) => Page::Notifications,
        ("messages", None) => Page::Messages,
        ("calendar", None) => Page::Calendar,
        ("calendar-day", Some(time)) => Page::CalendarDay(time),
        ("course", Some(id)) => Page::Course(id),
        _ => return Err("Unknown Moodle page.".into()),
    })
}
/// Opens one of the fixed Moodle pages in the organisation's own view.
pub async fn open_page(
    app: &AppHandle,
    id: &str,
    name: &str,
    value: Option<i64>,
) -> Result<(), String> {
    let saved = site(app, id)?;
    if let ("course", Some(course)) = (name, value) {
        // Only a course from this account's cached list can be opened by identifier.
        let known = app
            .state::<AppState>()
            .store
            .lock()
            .map_err(|_| "Site storage unavailable.")?
            .courses()
            .map_err(|_| "Could not read the course list.")?
            .iter()
            .any(|c| c.site_id == id && c.id == course);
        if !known {
            return Err("This course is no longer in your list. Refresh your courses.".into());
        }
    }
    let base = normalize_site(&saved.wwwroot).map_err(|e| e.to_string())?;
    if name == "activity" {
        let activity = activities(app)?
            .into_iter()
            .find(|a| a.site_id == id && Some(a.id) == value)
            .ok_or("This activity is no longer available. Refresh your course activities.")?;
        let target = activity_url(&base, &activity.module, activity.id)?;
        return crate::commands::restore_site_at(app, id, Some(target)).await;
    }
    let target = md_moodle::page_url(&base, page(name, value)?).map_err(|e| e.to_string())?;
    crate::commands::restore_site_at(app, id, Some(target)).await
}

fn activity_url(base: &url::Url, module: &str, id: i64) -> Result<url::Url, String> {
    if id <= 0
        || module.is_empty()
        || module == "label"
        || module.len() > 64
        || !module
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err("Invalid activity metadata.".into());
    }
    base.join(&format!("mod/{module}/view.php?id={id}"))
        .map_err(|_| "Invalid activity metadata.".into())
}
pub fn activities(app: &AppHandle) -> Result<Vec<md_store::Activity>, String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .activities()
        .map_err(|_| "Could not read course activities.".into())
}
pub fn inbox(app: &AppHandle) -> Result<Vec<md_store::InboxItem>, String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .notifications_list()
        .map_err(|_| "Could not read notifications.".into())
}
/// Server first; local state changes only after Moodle accepted the request.
pub async fn mark_read(app: &AppHandle, id: &str, notification: Option<i64>) -> Result<(), String> {
    let (site, client, token) = session(app, id)?;
    if let Some(notification) = notification {
        let cached = app
            .state::<AppState>()
            .store
            .lock()
            .map_err(|_| "Site storage unavailable.")?
            .notification_cached(id, notification)
            .map_err(|_| "Could not read notifications.")?;
        if !cached {
            return Err("This notification is no longer available.".into());
        }
        client.mark_notification_read(&token, notification).await
    } else {
        client
            .mark_all_notifications_read(&token, site.user_id)
            .await
    }
    .map_err(|e| e.to_string())?;
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .notifications_mark_read(id, site.user_id, notification)
        .map_err(|_| "Could not update notifications.")?;
    // The next poll reconciles the authoritative counts for the badge, Dock and menu bar.
    crate::notifications::wake(app, id);
    Ok(())
}

pub fn courses(app: &AppHandle) -> Result<Listing<md_store::Course>, String> {
    let state = app.state::<AppState>();
    let store = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?;
    Ok(Listing {
        items: store
            .courses()
            .map_err(|_| "Could not read the course list.")?,
        refreshed: store
            .refreshed("courses")
            .map_err(|_| "Could not read the course list.")?,
    })
}
pub async fn refresh_courses(app: &AppHandle, id: &str) -> Result<(), String> {
    let (site, client, token) = session(app, id)?;
    let fetched = client
        .courses(&token, site.user_id)
        .await
        .map_err(|e| e.to_string())?;
    let items: Vec<_> = fetched
        .into_iter()
        .filter(|c| c.id > 0)
        .take(MAX_COURSES)
        .map(|c| md_store::Course {
            site_id: id.to_owned(),
            id: c.id,
            name: plain_text(&c.fullname, NAME),
            short_name: plain_text(&c.shortname, 64),
            pinned: false,
        })
        .collect();
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .save_courses(id, site.user_id, &items, now())
        .map_err(|_| "Could not save the course list.".into())
}
pub fn pin(app: &AppHandle, id: &str, course: i64, pinned: bool) -> Result<(), String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .set_course_pin(id, course, pinned)
        .map_err(|_| "Could not update this pin.".into())
}

pub fn agenda(app: &AppHandle) -> Result<Listing<md_store::Event>, String> {
    let state = app.state::<AppState>();
    let store = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?;
    Ok(Listing {
        // Entries stay visible for an hour after they start or end.
        items: store
            .events(now() - 3600)
            .map_err(|_| "Could not read the agenda.")?,
        refreshed: store
            .refreshed("agenda")
            .map_err(|_| "Could not read the agenda.")?,
    })
}
pub async fn refresh_agenda(app: &AppHandle, id: &str) -> Result<(), String> {
    let (site, client, token) = session(app, id)?;
    let fetched = client
        .upcoming_events(&token)
        .await
        .map_err(|e| e.to_string())?;
    let items: Vec<_> = fetched
        .into_iter()
        .filter(|e| e.timestart > 0 && e.timeduration >= 0)
        .take(MAX_EVENTS)
        .map(|e| {
            // Moodle reports the site front page as course 1; that is not a course.
            let course = e.course.filter(|c| c.id > 1);
            md_store::Event {
                site_id: id.to_owned(),
                id: e.id,
                name: plain_text(&e.name, NAME),
                time_start: e.timestart,
                duration: e.timeduration,
                kind: kind(&e.eventtype).into(),
                course_id: course.as_ref().map(|c| c.id),
                course_name: course
                    .map(|c| plain_text(&c.fullname, NAME))
                    .filter(|name| !name.is_empty()),
            }
        })
        .collect();
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .save_events(id, site.user_id, &items, now())
        .map_err(|_| "Could not save the agenda.".into())
}
/// Moodle's open-ended event type, reduced to the few kinds the shell labels.
fn kind(event_type: &str) -> &'static str {
    match event_type {
        "due" | "gradingdue" | "close" | "expectcompletionon" => "due",
        "open" => "open",
        "user" => "user",
        "site" | "category" => "site",
        "course" | "group" => "course",
        _ => "other",
    }
}

/// User-triggered indexing; web-service permissions determine which contexts exist.
/// Refreshes metadata first and never guesses course ownership from the current page.
pub async fn refresh_folders(app: &AppHandle, id: &str) -> Result<(), String> {
    let expected = site(app, id)?.user_id;
    refresh_courses(app, id).await?;
    let courses = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .courses()
        .map_err(|_| "Could not read the course list.")?;
    for course in courses.into_iter().filter(|c| c.site_id == id) {
        refresh_contexts(app, id, course.id, expected).await?;
    }
    Ok(())
}
pub async fn refresh_contexts(
    app: &AppHandle,
    id: &str,
    course: i64,
    expected_user: i64,
) -> Result<(), String> {
    let (site, client, token) = session(app, id)?;
    if site.user_id != expected_user {
        return Err("This course is no longer in your list. Refresh your courses.".into());
    }
    let sections = client
        .course_contents(&token, course)
        .await
        .map_err(|e| e.to_string())?;
    let modules: Vec<_> = sections
        .into_iter()
        .flat_map(|s| s.modules)
        .take(10000)
        .collect();
    let contexts: Vec<_> = modules
        .iter()
        .filter_map(|m| m.contextid)
        .filter(|id| *id > 0)
        .take(10000)
        .collect();
    let base = normalize_site(&site.wwwroot).map_err(|e| e.to_string())?;
    let items: Vec<_> = modules
        .into_iter()
        .filter(|m| activity_url(&base, &m.modname, m.id).is_ok() && !m.name.trim().is_empty())
        .map(|m| md_store::Activity {
            site_id: id.to_owned(),
            course_id: course,
            id: m.id,
            name: plain_text(&m.name, NAME),
            module: m.modname,
            refreshed_at: now(),
        })
        .collect();
    let state = app.state::<AppState>();
    let store = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?;
    store
        .save_course_contexts(id, site.user_id, course, &contexts)
        .map_err(|_| "Could not save course folders.")?;
    store
        .save_activities(id, site.user_id, course, &items)
        .map_err(|_| "Could not save course activities.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activity_paths_are_built_from_identifiers_only() {
        let base = normalize_site("https://example.org/campus/").unwrap();
        assert_eq!(
            activity_url(&base, "assign", 42).unwrap().as_str(),
            "https://example.org/campus/mod/assign/view.php?id=42"
        );
        for module in [
            "../admin",
            "assign?token=x",
            "",
            "label",
            "https://evil.org",
        ] {
            assert!(activity_url(&base, module, 1).is_err());
        }
        assert!(activity_url(&base, "quiz", 0).is_err());
    }
    #[test]
    fn only_fixed_pages_with_matching_arguments_are_accepted() {
        assert_eq!(page("course", Some(4)).unwrap(), Page::Course(4));
        assert_eq!(page("notifications", None).unwrap(), Page::Notifications);
        assert_eq!(page("calendar-day", Some(9)).unwrap(), Page::CalendarDay(9));
        for (name, value) in [
            ("course", None),
            ("notifications", Some(1)),
            ("https://example.org/", None),
            ("", None),
        ] {
            assert!(page(name, value).is_err());
        }
        assert_eq!(kind("gradingdue"), "due");
        assert_eq!(kind("something-new"), "other");
    }
}
