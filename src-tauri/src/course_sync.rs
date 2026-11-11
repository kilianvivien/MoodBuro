//! One serial background worker. Consent, checkpoints and retry timing live in SQLite.
use crate::AppState;
use tauri::{AppHandle, Emitter, Manager};
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_secs() as i64)
}
pub fn changed(app: &AppHandle) {
    let _ = app.emit_to("shell", "course-sync://changed", ());
}
pub fn list(app: &AppHandle) -> Result<Vec<md_store::CourseSync>, String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .course_syncs()
        .map_err(|_| "Could not read course sync settings.".into())
}
pub fn configure(
    app: &AppHandle,
    site: &str,
    user: i64,
    course: i64,
    keys: Vec<String>,
    interval: i64,
) -> Result<(), String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .configure_course_sync(site, user, course, &keys, interval, now())
        .map_err(|_| "Could not save course sync settings. Reopen the course files.")?;
    changed(app);
    Ok(())
}
pub fn control(
    app: &AppHandle,
    site: &str,
    user: i64,
    course: i64,
    cancel: bool,
) -> Result<(), String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .control_course_sync(site, user, course, cancel, now())
        .map_err(|_| "Could not change course sync. Reopen the course files.")?;
    changed(app);
    Ok(())
}
async fn until_stopped<T>(
    work: impl std::future::Future<Output = T>,
    valid: impl Fn() -> bool,
) -> Option<T> {
    tokio::pin!(work);
    loop {
        if !valid() {
            return None;
        }
        tokio::select! {
            result = &mut work => return Some(result),
            _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => {},
        }
    }
}
pub fn install(app: &AppHandle) {
    let _ = app
        .state::<AppState>()
        .store
        .lock()
        .expect("store")
        .recover_course_syncs(now());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let job = app
                .state::<AppState>()
                .store
                .lock()
                .ok()
                .and_then(|store| store.claim_course_sync(now()).ok())
                .flatten();
            if let Some(job) = job {
                changed(&app);
                let result = until_stopped(crate::offline::sync_course(&app, &job), || {
                    app.state::<AppState>()
                        .store
                        .lock()
                        .ok()
                        .is_some_and(|store| store.sync_current(&job).unwrap_or(false))
                })
                .await;
                if let Some(result) = result {
                    if let Ok(store) = app.state::<AppState>().store.lock() {
                        let _ = store.finish_course_sync(&job, now(), result.err().as_deref());
                    }
                }
                changed(&app);
            } else {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelling_an_in_flight_job_drops_its_transfer_and_releases_resources() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        struct Guard(Arc<AtomicBool>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        tauri::async_runtime::block_on(async {
            let dropped = Arc::new(AtomicBool::new(false));
            let valid = Arc::new(AtomicBool::new(true));
            let cancel = valid.clone();
            let work = async {
                let _guard = Guard(dropped.clone());
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            };
            let trigger = async {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                cancel.store(false, Ordering::SeqCst);
            };
            let (result, ()) = tokio::join!(
                until_stopped(work, || valid.load(Ordering::SeqCst)),
                trigger
            );
            assert!(result.is_none());
            assert!(dropped.load(Ordering::SeqCst));
            assert_eq!(until_stopped(async { 42 }, || true).await, Some(42));
        });
    }
}
