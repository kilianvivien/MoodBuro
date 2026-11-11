//! Main-thread public WKDownload handles; opaque native state never crosses IPC.
use objc2::rc::Retained;
use objc2_foundation::NSProgressReporting;
use objc2_web_kit::WKDownload;
use std::{cell::RefCell, collections::HashMap, time::Instant};
use tauri::{AppHandle, Emitter, Manager};
struct Native {
    download: Retained<WKDownload>,
    created: Instant,
}
thread_local! {static ACTIVE:RefCell<HashMap<(String,String),Native>>=RefCell::new(HashMap::new());}
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: String,
    pub received: u64,
    pub total: Option<u64>,
}
pub fn register(site: &str, download: &WKDownload) {
    let source = unsafe { download.originalRequest() }
        .and_then(|r| r.URL())
        .and_then(|u| u.absoluteString())
        .and_then(|u| url::Url::parse(&u.to_string()).ok())
        .map(|u| u.to_string());
    if let Some(source) = source {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active.len() < 20 {
                active
                    .entry((site.into(), source))
                    .or_insert_with(|| Native {
                        download: unsafe {
                            Retained::retain(download as *const _ as *mut WKDownload)
                        }
                        .expect("live WebKit download"),
                        created: Instant::now(),
                    });
            }
        });
    }
}
pub fn poll(app: &AppHandle) {
    let mut progress = Vec::new();
    ACTIVE.with(|active| {
        active.borrow_mut().retain(|(site, url), native| {
            if let Some((id, _)) = crate::downloads::live_info(app, site, url) {
                let p = native.download.progress();
                let total = p.totalUnitCount();
                progress.push(Progress {
                    id,
                    received: p.completedUnitCount().max(0) as u64,
                    total: (total > 0).then_some(total as u64),
                });
                true
            } else {
                native.created.elapsed().as_secs() < 3
            }
        })
    });
    let _ = app.emit_to("shell", "downloads://progress", &progress);
    if let Some(window) = app.get_window("main") {
        use tauri::window::{ProgressBarState, ProgressBarStatus};
        let total: u64 = progress
            .iter()
            .filter_map(|p| p.total)
            .fold(0, u64::saturating_add);
        let received: u64 = progress
            .iter()
            .map(|p| p.received)
            .fold(0, u64::saturating_add);
        let status = if progress.is_empty() {
            ProgressBarStatus::None
        } else if total == 0 || progress.iter().any(|p| p.total.is_none()) {
            ProgressBarStatus::Indeterminate
        } else {
            ProgressBarStatus::Normal
        };
        let _ = window.set_progress_bar(ProgressBarState {
            status: Some(status),
            progress: (total > 0).then(|| {
                received
                    .saturating_mul(100)
                    .checked_div(total)
                    .unwrap_or(0)
                    .min(100)
            }),
        });
    }
}
pub async fn cancel(app: &AppHandle, site: String, source: String) -> Result<(), String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let entry = ACTIVE.with(|active| active.borrow_mut().remove(&(site, source)));
        if let Some(native) = entry {
            let send = std::sync::Mutex::new(Some(send));
            let callback = block2::RcBlock::new(move |_: *mut objc2_foundation::NSData| {
                if let Some(send) = send.lock().ok().and_then(|mut s| s.take()) {
                    let _ = send.send(Ok(()));
                }
            });
            unsafe {
                native.download.cancel(Some(&callback));
            }
        } else {
            let _ = send.send(Err(
                "This download cannot be cancelled here. Return to Moodle.".to_string(),
            ));
        }
        poll(&handle);
    })
    .map_err(|_| "Could not cancel the download.")?;
    receive
        .await
        .map_err(|_| "Could not cancel the download.".to_string())?
}
pub fn install(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || poll(&handle));
        }
    });
}
