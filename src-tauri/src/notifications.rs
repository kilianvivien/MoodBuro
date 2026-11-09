//! One cancellable worker per saved account. No credentials enter IPC/events.
use crate::AppState;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub site_id: String,
    pub unread: u64,
    pub connection: &'static str,
}
pub struct Worker {
    generation: uuid::Uuid,
    task: tauri::async_runtime::JoinHandle<()>,
    wake: Arc<tokio::sync::Notify>,
    status: Status,
}
pub type Workers = HashMap<String, Worker>;

pub fn install_click_handler(app: &AppHandle) -> tauri_plugin_notification::Result<()> {
    let handle = app.clone();
    app.notification().on_action(move |action| {
        if action.action_id() != "tap" {
            return;
        }
        let id = action
            .notification()
            .and_then(|n| n.extra().get("siteId"))
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        let app = handle.clone();
        tauri::async_runtime::spawn(async move {
            crate::tray::show(&app);
            if let Some(id) = id {
                if crate::commands::restore_site(&app, &id).await.is_ok() {
                    // Fallback routing never follows a server-supplied notification URL.
                    let base = app
                        .state::<AppState>()
                        .store
                        .lock()
                        .expect("store")
                        .sites()
                        .unwrap_or_default()
                        .into_iter()
                        .find(|s| s.id == id)
                        .and_then(|s| md_moodle::normalize_site(&s.wwwroot).ok());
                    if let (Some(base), Some(view)) = (base, app.get_webview(&format!("site-{id}")))
                    {
                        if let Ok(url) = base.join("message/output/popup/notifications.php") {
                            let _ = view.navigate(url);
                        }
                        let _ = app.emit_to("shell", "sites://changed", id);
                    }
                } else {
                    let _ = crate::windows::hide_sites(&app);
                    let _ = app.emit_to("shell", "shell://navigate", "settings");
                }
            }
        });
    })
}

pub fn statuses(app: &AppHandle) -> Vec<Status> {
    app.state::<AppState>()
        .workers
        .lock()
        .expect("workers")
        .values()
        .map(|w| w.status.clone())
        .collect()
}
pub fn publish(app: &AppHandle) {
    let statuses = statuses(app);
    let total = statuses
        .iter()
        .fold(0_u64, |n, s| n.saturating_add(s.unread));
    let _ = app.emit_to("shell", "unread://changed", &statuses);
    if let Some(window) = app.get_window("main") {
        let _ = window.set_badge_count(if total == 0 {
            None
        } else {
            Some(total.min(i64::MAX as u64) as i64)
        });
    }
    let _ = crate::tray::refresh(app, &statuses);
}
pub fn check_now(app: &AppHandle) {
    for worker in app
        .state::<AppState>()
        .workers
        .lock()
        .expect("workers")
        .values()
    {
        worker.wake.notify_one();
    }
}
pub fn stop(app: &AppHandle, id: &str) {
    if let Some(worker) = app
        .state::<AppState>()
        .workers
        .lock()
        .expect("workers")
        .remove(id)
    {
        worker.task.abort();
    }
    publish(app);
}
pub fn start(app: &AppHandle, site: md_store::Site) {
    stop(app, &site.id);
    let state = app.state::<AppState>();
    let previous = state
        .store
        .lock()
        .expect("store")
        .notification_state(&site.id)
        .ok()
        .flatten();
    let wake = Arc::new(tokio::sync::Notify::new());
    let generation = uuid::Uuid::new_v4();
    // Hold the map until insertion so even an immediate response sees its worker.
    let mut workers = state.workers.lock().expect("workers");
    let handle = app.clone();
    let id = site.id.clone();
    let signal = wake.clone();
    let task = tauri::async_runtime::spawn(async move {
        run(handle, site, generation, signal, previous.map(|p| p.0)).await;
    });
    workers.insert(
        id.clone(),
        Worker {
            generation,
            task,
            wake,
            status: Status {
                site_id: id,
                unread: previous.map_or(0, |p| p.1),
                connection: "checking",
            },
        },
    );
    drop(workers);
    publish(app);
}

async fn run(
    app: AppHandle,
    site: md_store::Site,
    generation: uuid::Uuid,
    wake: Arc<tokio::sync::Notify>,
    mut checkpoint: Option<i64>,
) {
    let client = match md_moodle::normalize_site(&site.wwwroot).and_then(md_moodle::Client::new) {
        Ok(client) => client,
        Err(_) => return,
    };
    let mut token = None;
    let mut failures: u32 = 0;
    loop {
        let credentials = if token.is_some() {
            Ok(())
        } else {
            match md_secrets::get_optional(&site.id, md_secrets::Kind::Token) {
                Ok(Some(value)) => {
                    token = Some(value);
                    Ok(())
                }
                Ok(None) => Err(md_moodle::Error::InvalidToken),
                Err(_) => Err(md_moodle::Error::WebServices),
            }
        };
        let result = match credentials {
            Ok(()) => {
                poll(
                    &client,
                    token.as_ref().expect("loaded credentials"),
                    site.user_id,
                )
                .await
            }
            Err(error) => Err(error),
        };
        let mut expired = false;
        {
            let state = app.state::<AppState>();
            let mut workers = state.workers.lock().expect("workers");
            let Some(worker) = workers
                .get_mut(&site.id)
                .filter(|w| w.generation == generation)
            else {
                return;
            };
            match result {
                Ok((unread, response)) => {
                    let batch = md_notifier::batch(checkpoint, &response.notifications);
                    let items: Vec<_> = response
                        .notifications
                        .iter()
                        .map(|n| md_store::Notification {
                            id: n.id,
                            subject: n.subject.chars().take(256).collect(),
                            time_created: n.timecreated,
                            read: n.read,
                        })
                        .collect();
                    if state
                        .store
                        .lock()
                        .expect("store")
                        .save_notifications(&site.id, batch.checkpoint, unread, &items)
                        .is_ok()
                    {
                        checkpoint = Some(batch.checkpoint);
                        worker.status.unread = unread;
                        worker.status.connection = "online";
                        failures = 0;
                        // Bounded, plain-text OS delivery; never include HTML, URLs or tokens.
                        for item in batch.alerts.iter().take(5) {
                            let body: String = item
                                .subject
                                .chars()
                                .filter(|c| !c.is_control())
                                .take(200)
                                .collect();
                            let _ = app
                                .notification()
                                .builder()
                                .title(&site.name)
                                .extra("siteId", &site.id)
                                .body(body)
                                .show();
                        }
                        if batch.alerts.len() > 5 {
                            let count = batch.alerts.len() - 5;
                            let body = if crate::menus::is_french(
                                state
                                    .store
                                    .lock()
                                    .expect("store")
                                    .settings()
                                    .unwrap_or_default()
                                    .language,
                            ) {
                                format!("+{count} autres notifications")
                            } else {
                                format!("+{count} more notifications")
                            };
                            let _ = app
                                .notification()
                                .builder()
                                .title(&site.name)
                                .extra("siteId", &site.id)
                                .body(body)
                                .show();
                        }
                    } else {
                        failures = failures.saturating_add(1);
                        worker.status.connection = "error";
                    }
                }
                Err(error) => {
                    failures = failures.saturating_add(1);
                    expired = matches!(error, md_moodle::Error::InvalidToken);
                    worker.status.connection = if expired {
                        "reauth"
                    } else if matches!(error, md_moodle::Error::Network) {
                        "offline"
                    } else {
                        "error"
                    };
                }
            }
        }
        publish(&app);
        if expired {
            let _ = app.emit_to("shell", "auth://needed", &site.id);
            return; // Reconnect replaces this worker; check-now never retries invalid credentials.
        }
        // A suspended Mac resumes with one poll, not a burst of missed interval ticks.
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(md_notifier::retry_seconds(failures))) => {},
            _ = wake.notified() => {},
        }
    }
}
async fn poll(
    client: &md_moodle::Client,
    token: &str,
    user: i64,
) -> Result<(u64, md_moodle::PopupNotifications), md_moodle::Error> {
    let notifications = client.unread_notifications(token, user).await?;
    let conversations = client.unread_conversations(token, user).await?;
    let items = client.popup_notifications(token, user).await?;
    Ok((notifications.saturating_add(conversations), items))
}
