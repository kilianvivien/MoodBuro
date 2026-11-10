//! Finish a rate-limited cold restore once, without extending browser cookies.
//! Keys, tokens and one-use URLs never leave Rust or enter this state.
use crate::{
    auth_log::{record, Event},
    AppState,
};
use md_moodle::{normalize_site, Client};
use md_secrets::Kind;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

const LOCKOUT: i64 = 6 * 60;
const MARGIN: u64 = 5;
#[derive(Default)]
pub struct Restores(HashMap<String, Restore>);
struct Restore {
    generation: uuid::Uuid,
    ready_at: Instant,
    destination: Option<Url>,
    scheduled: bool,
}

impl Restores {
    fn arm(&mut self, id: &str, destination: Option<Url>, delay: Duration) {
        self.0.entry(id.into()).or_insert_with(|| Restore {
            generation: uuid::Uuid::new_v4(),
            ready_at: Instant::now() + delay,
            destination,
            scheduled: false,
        });
    }
    fn cancel(&mut self, id: &str) {
        self.0.remove(id);
    }
    fn browsing_away(&mut self, id: &str) {
        if self.0.get(id).is_some_and(|r| r.scheduled) {
            self.cancel(id);
        }
    }
    fn schedule(&mut self, id: &str) -> Option<(uuid::Uuid, Duration)> {
        let restore = self.0.get_mut(id)?;
        if restore.scheduled {
            return None;
        }
        restore.scheduled = true;
        Some((
            restore.generation,
            restore.ready_at.saturating_duration_since(Instant::now()),
        ))
    }
    fn destination(&self, id: &str, generation: uuid::Uuid) -> Option<Option<Url>> {
        self.0
            .get(id)
            .filter(|r| r.generation == generation)
            .map(|r| r.destination.clone())
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0)
}

fn retry_delay(last: Option<i64>, now: i64) -> Duration {
    // Unknown, future or stale clocks must not cause a tight retry loop.
    let elapsed = last
        .map(|last| now.saturating_sub(last))
        .filter(|elapsed| (0..LOCKOUT).contains(elapsed))
        .unwrap_or(0);
    Duration::from_secs((LOCKOUT - elapsed) as u64 + MARGIN)
}

pub fn minted(app: &AppHandle, id: &str) {
    if let Ok(store) = app.state::<AppState>().store.lock() {
        // Missing metadata merely makes the next delay conservative.
        let _ = store.autologin_minted(id, now());
    }
}

pub fn arm(app: &AppHandle, id: &str, destination: Option<Url>) {
    let state = app.state::<AppState>();
    let last = state
        .store
        .lock()
        .ok()
        .and_then(|s| s.last_autologin_mint(id).ok().flatten());
    if let Ok(mut restores) = state.restores.lock() {
        restores.arm(id, destination, retry_delay(last, now()));
    };
}

pub fn cancel(app: &AppHandle, id: &str) {
    if let Ok(mut restores) = app.state::<AppState>().restores.lock() {
        restores.cancel(id);
    }
}

fn login_page(base: &Url, url: &Url) -> bool {
    url.origin() == base.origin()
        && ["login/", "login/index.php"]
            .iter()
            .any(|path| base.join(path).is_ok_and(|u| u.path() == url.path()))
}

pub fn navigation(app: &AppHandle, id: &str, base: &Url, url: &Url) {
    if url.origin() == base.origin()
        && (base
            .join("login/logout.php")
            .is_ok_and(|u| u.path() == url.path())
            || url.query_pairs().any(|(key, _)| key == "logout"))
    {
        cancel(app, id);
    } else if !login_page(base, url) {
        // Cancel at navigation start as well, including while a retry HTTP call runs.
        if let Ok(mut restores) = app.state::<AppState>().restores.lock() {
            restores.browsing_away(id);
        }
    }
}

/// Called only for finished top-level loads of the site's main webview.
pub fn loaded(app: &AppHandle, id: &str, base: &Url, url: &Url) {
    if !login_page(base, url) {
        // SSO/manual login or browsing elsewhere has already superseded the fallback.
        cancel(app, id);
        return;
    }
    let state = app.state::<AppState>();
    let task = state
        .restores
        .lock()
        .ok()
        .and_then(|mut restores| restores.schedule(id));
    let Some((generation, delay)) = task else {
        return;
    };
    record(Event::RetryScheduled);
    let app = app.clone();
    let id = id.to_owned();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        let state = app.state::<AppState>();
        // Serializes credential changes/removal and navigation with the restore.
        let _switch = state.switching.lock().await;
        let destination = state
            .restores
            .lock()
            .ok()
            .and_then(|r| r.destination(&id, generation));
        let Some(destination) = destination else {
            return;
        };
        let site = state
            .store
            .lock()
            .ok()
            .and_then(|s| s.sites().ok())
            .and_then(|sites| sites.into_iter().find(|s| s.id == id));
        let Some(site) = site else {
            return;
        };
        let Ok(base) = normalize_site(&site.wwwroot) else {
            return;
        };
        let Some(view) = app.get_webview(&format!("site-{id}")) else {
            return;
        };
        if !view.url().is_ok_and(|url| login_page(&base, &url)) {
            return;
        }
        // Do not compete with a user-initiated reconnect.
        if state.pending.lock().map_or(true, |p| p.is_some()) {
            return;
        }
        let result = async {
            let token =
                md_secrets::get(&id, Kind::Token).map_err(|_| md_moodle::Error::InvalidToken)?;
            let private =
                md_secrets::get(&id, Kind::Private).map_err(|_| md_moodle::Error::WebServices)?;
            let client = Client::new(base.clone())?;
            if client.site_info(&token).await?.userid != site.user_id {
                return Err(md_moodle::Error::InvalidToken);
            }
            let key = client.autologin_key(&token, &private).await?;
            minted(&app, &id);
            let dashboard = base.join("my/").map_err(|_| md_moodle::Error::InvalidUrl)?;
            let target = md_moodle::autologin_target_to(
                &base,
                &key,
                site.user_id,
                destination.as_ref().unwrap_or(&dashboard),
            )?;
            // Logout/manual navigation can happen while the HTTP request is in flight.
            let current = state
                .restores
                .lock()
                .is_ok_and(|r| r.destination(&id, generation).is_some());
            if current && view.url().is_ok_and(|url| login_page(&base, &url)) {
                // Navigate this view directly; a background restore must not select it.
                let _ = view.navigate(target);
            }
            Ok::<_, md_moodle::Error>(())
        }
        .await;
        if matches!(result, Err(md_moodle::Error::InvalidToken)) {
            let _ = app.emit_to("shell", "auth://needed", &id);
        }
        // Keep the scheduled flag after failure: no further retries in this view.
        record(Event::RetryFinished);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_is_one_shot_and_old_tasks_cannot_claim_reconnected_sites() {
        let mut restores = Restores::default();
        assert!(restores.schedule("site").is_none());
        let target = Url::parse("https://example.org/course/view.php?id=2").unwrap();
        restores.arm("site", Some(target.clone()), Duration::ZERO);
        // The initial /my/ -> /login/ navigation must not cancel the armed fallback.
        restores.browsing_away("site");
        let (old, _) = restores.schedule("site").unwrap();
        // Repeated login loads, warm switches and an already failed retry cannot rearm it.
        restores.arm("site", None, Duration::ZERO);
        assert!(restores.schedule("site").is_none());
        assert_eq!(restores.destination("site", old), Some(Some(target)));
        restores.cancel("site");
        assert!(restores.destination("site", old).is_none());
        restores.arm("site", None, Duration::ZERO);
        let (new, _) = restores.schedule("site").unwrap();
        assert_ne!(old, new);
        assert!(restores.destination("site", old).is_none());
        assert_eq!(restores.destination("site", new), Some(None));
        restores.browsing_away("site");
        assert!(restores.destination("site", new).is_none());
    }
    #[test]
    fn lockout_delay_is_bounded_and_conservative() {
        assert_eq!(retry_delay(None, 1000).as_secs(), 365);
        assert_eq!(retry_delay(Some(900), 1000).as_secs(), 265);
        assert_eq!(retry_delay(Some(0), 1000).as_secs(), 365);
        assert_eq!(retry_delay(Some(2000), 1000).as_secs(), 365);
        assert_eq!(retry_delay(Some(i64::MIN), i64::MAX).as_secs(), 365);
    }
    #[test]
    fn login_detection_respects_origin_and_installation_path() {
        let base = Url::parse("https://example.org/campus/").unwrap();
        for path in ["login/", "login/index.php?test=1"] {
            assert!(login_page(&base, &base.join(path).unwrap()));
        }
        for url in [
            "https://evil.org/campus/login/",
            "https://example.org/login/",
            "https://example.org/campus/login/logout.php",
            "https://example.org/campus/login/forgot_password.php",
            "https://example.org/campus/my/",
        ] {
            assert!(!login_page(&base, &Url::parse(url).unwrap()));
        }
    }
}
