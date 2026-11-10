//! Local reminders only while the process runs; no scheduled OS delivery while quit.
use crate::AppState;
use std::{sync::Arc, time::Duration};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
#[derive(Default)]
pub struct Signals {
    refresh: Arc<tokio::sync::Notify>,
}
pub fn wake(app: &AppHandle) {
    app.state::<Signals>().refresh.notify_one();
}
pub fn changed(app: &AppHandle) {
    let _ = app.emit_to("shell", "reminders://changed", ());
}
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_secs() as i64)
}
pub fn list(app: &AppHandle) -> Result<Vec<md_store::ReminderSettings>, String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .reminders()
        .map_err(|_| "Could not read reminder settings.".into())
}
pub fn set(app: &AppHandle, site: &str, user: i64, hour: bool, day: bool) -> Result<(), String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .set_reminders(site, user, hour, day, now())
        .map_err(|_| {
            "Could not save reminder settings. Reopen the agenda for the current account."
        })?;
    wake(app);
    changed(app);
    Ok(())
}
pub fn install(app: &AppHandle) {
    app.manage(Signals::default());
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut last = now();
        let mut fresh_after = last;
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;
            let current = now();
            if current < last || current.saturating_sub(last) > 60 {
                // Wake or clock jump: discard missed reminders and require a new snapshot.
                fresh_after = current;
                last = current;
                wake(&handle);
                changed(&handle);
                continue;
            }
            deliver(&handle, last, current, fresh_after);
            last = current;
        }
    });
    let handle = app.clone();
    let signal = app.state::<Signals>().refresh.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            for settings in list(&handle)
                .unwrap_or_default()
                .into_iter()
                .filter(|s| s.hour || s.day)
            {
                let error = crate::library::refresh_agenda_for_user(
                    &handle,
                    &settings.site_id,
                    Some(settings.user_id),
                )
                .await
                .err();
                let state = handle.state::<AppState>();
                if let Ok(store) = state.store.lock() {
                    let _ = store.reminder_refresh_result(
                        &settings.site_id,
                        settings.user_id,
                        error.as_deref(),
                    );
                }
                changed(&handle);
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(300)) => {},
                _ = signal.notified() => {},
            }
        }
    });
}
fn deliver(app: &AppHandle, last: i64, now: i64, fresh_after: i64) {
    let state = app.state::<AppState>();
    // Keep account/prefs stable through claim and native notification attempts.
    let Ok(store) = state.store.lock() else {
        return;
    };
    let Ok(items) = store.claim_reminders(last, now, fresh_after) else {
        return;
    };
    if items.is_empty() {
        return;
    }
    let french = crate::menus::is_french(store.settings().unwrap_or_default().language);
    for item in items.iter().take(5) {
        let when = match (item.offset, french) {
            (86400, false) => "Due in 24 hours",
            (86400, true) => "Échéance dans 24 heures",
            (_, false) => "Due in 1 hour",
            (_, true) => "Échéance dans 1 heure",
        };
        let body = format!("{when} · {}", md_moodle::plain_text(&item.name, 200));
        if app
            .notification()
            .builder()
            .title(md_moodle::plain_text(&item.site_name, 100))
            .body(body)
            .extra("kind", "reminder")
            .show()
            .is_err()
        {
            let _ = store.reminder_delivery_failed(&item.site_id, item.user_id);
        }
    }
    if items.len() > 5 {
        let count = items.len() - 5;
        let body = if french {
            format!("{count} autres échéances. Ouvrez l’agenda pour les consulter.")
        } else {
            format!("{count} more deadlines. Open the agenda to review them.")
        };
        if app
            .notification()
            .builder()
            .title("MoodBuro")
            .body(body)
            .extra("kind", "reminder")
            .show()
            .is_err()
        {
            for item in items.iter().skip(5) {
                let _ = store.reminder_delivery_failed(&item.site_id, item.user_id);
            }
        }
    }
    drop(store);
    changed(app);
}
