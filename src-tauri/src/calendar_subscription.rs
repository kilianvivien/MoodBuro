//! Explicit handoff of provider-issued HTTPS feeds. No URLs/credentials in IPC or SQLite.
use crate::AppState;
#[cfg(target_os = "macos")]
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Manager};
#[cfg(target_os = "macos")]
#[derive(Default)]
pub struct Operations(Arc<Mutex<HashSet<(String, i64)>>>);
#[cfg(not(target_os = "macos"))]
#[derive(Default)]
pub struct Operations;
#[cfg(target_os = "macos")]
struct Claim {
    operations: Arc<Mutex<HashSet<(String, i64)>>>,
    key: (String, i64),
}
#[cfg(target_os = "macos")]
impl Drop for Claim {
    fn drop(&mut self) {
        if let Ok(mut active) = self.operations.lock() {
            active.remove(&self.key);
        }
    }
}
pub fn list(app: &AppHandle) -> Result<Vec<md_store::CalendarHandoff>, String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .calendar_handoffs()
        .map_err(|_| "Could not read calendar setup history.".into())
}
#[cfg(any(target_os = "macos", test))]
fn authorize(store: &md_store::Store, site: &str, user: i64, repeat: bool) -> Result<(), String> {
    validate(store, site, user)?;
    if !repeat
        && store
            .calendar_handoffs()
            .map_err(|_| "Could not read calendar setup history.")?
            .iter()
            .any(|h| h.site_id == site && h.user_id == user)
    {
        return Err(
            "Calendar setup was already opened. Confirm reopening it to avoid duplicates.".into(),
        );
    }
    Ok(())
}
#[cfg(any(target_os = "macos", test))]
fn validate(store: &md_store::Store, site: &str, user: i64) -> Result<(), String> {
    if store
        .sites()
        .map_err(|_| "Could not read sites.")?
        .iter()
        .any(|s| s.id == site && s.user_id == user)
    {
        Ok(())
    } else {
        Err("This calendar belongs to another account. Reopen the agenda.".into())
    }
}
pub async fn subscribe(
    app: &AppHandle,
    site: &str,
    user: i64,
    confirmed: bool,
    repeat: bool,
) -> Result<(), String> {
    if !confirmed {
        return Err("Confirm sharing the private calendar link with Calendar first.".into());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, site, user, repeat);
        Err("Calendar subscription setup is available on macOS. Use ICS export instead.".into())
    }
    #[cfg(target_os = "macos")]
    {
        let operations = app.state::<Operations>().0.clone();
        let key = (site.to_string(), user);
        if !operations
            .lock()
            .map_err(|_| "Calendar setup is unavailable.")?
            .insert(key.clone())
        {
            return Err("Calendar setup is already running.".into());
        }
        let _claim = Claim { operations, key };
        {
            let state = app.state::<AppState>();
            let store = state
                .store
                .lock()
                .map_err(|_| "Site storage unavailable.")?;
            authorize(&store, site, user, repeat)?;
        }
        let (saved, client, token) = crate::library::session(app, site)?;
        if saved.user_id != user {
            return Err("This calendar belongs to another account. Reopen the agenda.".into());
        }
        let feed=client.calendar_feed(&token,user).await.map_err(|_|"Moodle calendar subscription is unavailable. Open calendar export in Moodle or use ICS export.")?;
        client
            .probe_calendar(&feed)
            .await
            .map_err(|_| "The calendar feed could not be verified. Try again or use ICS export.")?;
        let provider = feed.provider.id();
        let (send, receive) = tokio::sync::oneshot::channel();
        let handle = app.clone();
        let site = site.to_owned();
        app.run_on_main_thread(move || {
            let state = handle.state::<AppState>();
            let _result = state
                .store
                .lock()
                .map_err(|_| "Site storage unavailable.".to_string())
                .and_then(|store| {
                    validate(&store, &site, user)?;
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs() as i64);
                    store
                        .calendar_handed_off(&site, user, now, provider)
                        .map_err(|_| "Could not save calendar setup history.".to_string())?;
                    open_native(feed, send);
                    Ok(())
                });
            // The sender is owned by the native callback after dispatch. Validation failure
            // drops it; the caller receives a sanitized failure without the private URL.
        })
        .map_err(|_| "Could not open Calendar setup.")?;
        tokio::time::timeout(std::time::Duration::from_secs(30), receive)
            .await
            .map_err(|_| "Calendar did not respond. Check Calendar before trying again.")?
            .map_err(|_| "Could not open Calendar setup.")??;
        let state = app.state::<AppState>();
        let store = state
            .store
            .lock()
            .map_err(|_| "Site storage unavailable.")?;
        validate(&store, &saved.id, user)?;
        Ok(())
    }
}
#[cfg(target_os = "macos")]
fn open_native(
    feed: md_moodle::calendar::Feed,
    send: tokio::sync::oneshot::Sender<Result<(), String>>,
) {
    use objc2_app_kit::{NSWorkspace, NSWorkspaceOpenConfiguration};
    use objc2_foundation::{NSArray, NSString, NSURL};
    let workspace = NSWorkspace::sharedWorkspace();
    let Some(application) =
        workspace.URLForApplicationWithBundleIdentifier(&NSString::from_str("com.apple.iCal"))
    else {
        let _ = send.send(Err(
            "Apple Calendar is not available. Use ICS export instead.".into(),
        ));
        return;
    };
    let Some(url) = NSURL::URLWithString(&NSString::from_str(feed.as_str())) else {
        let _ = send.send(Err("Could not open Calendar setup.".into()));
        return;
    };
    let configuration = NSWorkspaceOpenConfiguration::configuration();
    configuration.setAddsToRecentItems(false);
    let send = Mutex::new(Some(send));
    let callback = block2::RcBlock::new(
        move |_: *mut objc2_app_kit::NSRunningApplication,
              error: *mut objc2_foundation::NSError| {
            if let Some(send) = send.lock().ok().and_then(|mut value| value.take()) {
                let _ = send.send(if error.is_null() {
                    Ok(())
                } else {
                    Err("Could not open Calendar setup.".into())
                });
            }
        },
    );
    // Explicit Calendar target preserves HTTPS. Do not replace it with webcal://,
    // launch a shell with the private URL, or use an arbitrary default URL handler.
    workspace.openURLs_withApplicationAtURL_configuration_completionHandler(
        &NSArray::from_retained_slice(&[url]),
        &application,
        &configuration,
        Some(&callback),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeat_setup_requires_consent_and_account_replacement_invalidates_old_requests() {
        let store = md_store::Store::open(":memory:").unwrap();
        let mut site = md_store::Site {
            id: "s".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 7,
            full_name: "Learner".into(),
        };
        store.insert(&site).unwrap();
        assert!(authorize(&store, "s", 7, false).is_ok());
        store.calendar_handed_off("s", 7, 1, "moodle").unwrap();
        assert!(authorize(&store, "s", 7, false).is_err());
        assert!(authorize(&store, "s", 7, true).is_ok());
        site.user_id = 8;
        store.update(&site).unwrap();
        assert!(authorize(&store, "s", 7, true).is_err());
        assert!(authorize(&store, "s", 8, false).is_ok());
        store.remove("s").unwrap();
        assert!(authorize(&store, "s", 8, true).is_err());
    }
}
