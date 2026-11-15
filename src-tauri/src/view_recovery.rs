//! Rebuild a failed foreground Moodle view once connectivity has returned.
//! The browser resumes at the dashboard. An interrupted request is never replayed.
use crate::{auth_log, AppState};
use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

const STALL_TIMEOUT: Duration = Duration::from_secs(30);
const FAILURE_GRACE: Duration = Duration::from_secs(2);
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);
const PROBE_DELAYS: [Duration; 4] = [
    Duration::ZERO,
    Duration::from_secs(5),
    Duration::from_secs(15),
    Duration::from_secs(30),
];
const REBUILD_INTERVAL: Duration = Duration::from_secs(90);

#[derive(Default)]
pub struct Rebuilds(HashMap<String, Instant>);

impl Rebuilds {
    fn reserve(&mut self, site: &str, now: Instant) -> bool {
        self.0
            .retain(|_, started| now.duration_since(*started) < REBUILD_INTERVAL);
        if self.0.contains_key(site) {
            return false;
        }
        self.0.insert(site.to_owned(), now);
        true
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Provisional,
    Committed,
    Failed,
}

#[derive(Default)]
struct Navigation {
    eligible: bool,
    revision: u64,
    id: Option<usize>,
    stage: Option<Stage>,
    claimed: bool,
}

impl Navigation {
    fn policy(&mut self, eligible: bool) {
        self.eligible = eligible;
        self.invalidate();
        self.id = None;
        self.stage = None;
    }

    fn started(&mut self, id: usize) -> Option<u64> {
        self.invalidate();
        self.id = self.eligible.then_some(id);
        self.stage = self.eligible.then_some(Stage::Provisional);
        self.id.map(|_| self.revision)
    }

    fn update(&mut self, id: usize, stage: Option<Stage>) -> Option<u64> {
        if self.id != Some(id) {
            return None;
        }
        // Commit/completion ends the provisional stall check. A genuine
        // post-commit failure can claim its own recovery if one occurs.
        if stage != Some(Stage::Failed) {
            self.invalidate();
        }
        self.stage = stage;
        if matches!(stage, None | Some(Stage::Failed)) {
            self.id = None;
        }
        Some(self.revision)
    }

    fn terminated(&mut self) -> Option<u64> {
        self.id = None;
        self.stage = self.eligible.then_some(Stage::Failed);
        self.stage.map(|_| self.revision)
    }

    fn invalidate(&mut self) {
        self.revision += 1;
        self.claimed = false;
    }

    fn claim(&mut self, revision: u64, trigger: Stage) -> bool {
        if self.claimed || self.revision != revision || self.stage != Some(trigger) {
            return false;
        }
        self.claimed = true;
        true
    }

    fn is_recovering(&self, revision: u64) -> bool {
        self.claimed
            && self.revision == revision
            && matches!(self.stage, Some(Stage::Provisional | Stage::Failed))
    }
}

async fn check_account<F>(
    delays: &[Duration],
    is_current: impl Fn() -> bool,
    mut probe: impl FnMut() -> F,
) -> Option<bool>
where
    F: Future<Output = Result<bool, md_moodle::Error>>,
{
    for &delay in delays {
        tokio::time::sleep(delay).await;
        if !is_current() {
            return None;
        }
        let result = tokio::time::timeout(PROBE_TIMEOUT, probe()).await;
        if !is_current() {
            return None;
        }
        match result {
            Ok(Ok(matches_account)) => return Some(matches_account),
            Ok(Err(md_moodle::Error::InvalidToken)) => return Some(false),
            Ok(Err(md_moodle::Error::Network)) | Err(_) => {}
            Ok(Err(_)) => return None,
        }
    }
    None
}

/// Only track ordinary GETs from the dashboard and course browsing.
/// A view on a login, editor, quiz, submission or download page is left alone.
pub fn browse_page(base: &Url, url: &Url) -> bool {
    md_moodle::shareable_url(url)
        && base.origin() == url.origin()
        && url.path().strip_prefix(base.path()).is_some_and(|path| {
            matches!(
                path,
                "" | "index.php"
                    | "my/"
                    | "my/index.php"
                    | "my/courses.php"
                    | "course/index.php"
                    | "course/view.php"
                    | "course/section.php"
            )
        })
        && url.query_pairs().all(|(key, value)| {
            matches!(key.as_ref(), "id" | "section" | "categoryid")
                && !value.is_empty()
                && value.bytes().all(|byte| byte.is_ascii_digit())
        })
}

#[derive(Clone)]
pub struct Recovery {
    app: AppHandle,
    site: String,
    navigation: Arc<Mutex<Navigation>>,
}

impl Recovery {
    pub fn new(app: AppHandle, site: String) -> Self {
        Self {
            app,
            site,
            navigation: Default::default(),
        }
    }

    pub fn same_view(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.navigation, &other.navigation)
    }

    pub fn policy(&self, eligible: bool) {
        if let Ok(mut navigation) = self.navigation.lock() {
            navigation.policy(eligible);
        }
    }

    pub fn started(&self, id: usize) {
        if let Some(revision) = self.navigation.lock().ok().and_then(|mut n| n.started(id)) {
            self.schedule(revision, Stage::Provisional, STALL_TIMEOUT);
        }
    }

    pub fn committed(&self, id: usize) {
        if let Ok(mut navigation) = self.navigation.lock() {
            navigation.update(id, Some(Stage::Committed));
        }
    }

    pub fn ended(&self, id: usize, connection_failed: bool) {
        let stage = connection_failed.then_some(Stage::Failed);
        let revision = self
            .navigation
            .lock()
            .ok()
            .and_then(|mut n| n.update(id, stage));
        if let Some(revision) = revision.filter(|_| connection_failed) {
            self.schedule(revision, Stage::Failed, FAILURE_GRACE);
        }
    }

    pub fn terminated(&self) {
        if let Some(revision) = self.navigation.lock().ok().and_then(|mut n| n.terminated()) {
            self.schedule(revision, Stage::Failed, FAILURE_GRACE);
        }
    }

    fn can_recover_foreground_site(&self) -> bool {
        let state = self.app.state::<AppState>();
        state
            .active
            .lock()
            .is_ok_and(|id| id.as_deref() == Some(self.site.as_str()))
            && state.pending.lock().is_ok_and(|pending| pending.is_none())
            && !state
                .quick_open_visible
                .load(std::sync::atomic::Ordering::Relaxed)
    }

    fn is_pending(&self, revision: u64) -> bool {
        self.navigation
            .lock()
            .is_ok_and(|n| n.is_recovering(revision))
            && self.can_recover_foreground_site()
    }

    fn schedule(&self, revision: u64, trigger: Stage, delay: Duration) {
        let recovery = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(delay).await;
            if !recovery.can_recover_foreground_site()
                || !recovery
                    .navigation
                    .lock()
                    .is_ok_and(|mut n| n.claim(revision, trigger))
            {
                return;
            }
            let Ok((site, client, token)) = crate::library::session(&recovery.app, &recovery.site)
            else {
                return;
            };
            auth_log::record(auth_log::Event::ViewRecoveryStarted);
            let Some(credential_valid) = check_account(
                &PROBE_DELAYS,
                || recovery.is_pending(revision),
                || async {
                    client
                        .site_info(&token)
                        .await
                        .map(|info| info.userid == site.user_id)
                },
            )
            .await
            else {
                return;
            };
            let app = recovery.app.clone();
            let _ = app.run_on_main_thread(move || recovery.finish(revision, credential_valid));
        });
    }

    fn finish(&self, revision: u64, credential_valid: bool) {
        let state = self.app.state::<AppState>();
        let Ok(_switch) = state.switching.try_lock() else {
            return;
        };
        if !self.is_pending(revision)
            || !crate::frame_guard::is_current_recovery(&self.site, self)
            || !self
                .app
                .get_window("main")
                .is_some_and(|window| window.is_focused().unwrap_or(false))
        {
            return;
        }
        if !credential_valid {
            let _ = self.app.emit_to(
                "shell",
                "shell://error",
                md_moodle::Error::InvalidToken.to_string(),
            );
            return;
        }
        if !state
            .view_rebuilds
            .lock()
            .is_ok_and(|mut rebuilds| rebuilds.reserve(&self.site, Instant::now()))
        {
            return;
        }
        let rebuild = crate::windows::close_site_view(&self.app, &self.site)
            .map_err(|_| "Could not renew the Moodle view.".to_owned())
            .and_then(|()| {
                // The new view first uses the retained profile. If Moodle shows its
                // login page, the existing restore flow can supply a fresh key.
                crate::session_restore::arm_after_view_rebuild(&self.app, &self.site);
                crate::windows::switch(&self.app, &self.site)
            });
        match rebuild {
            Ok(()) => {
                auth_log::record(auth_log::Event::ViewRebuilt);
                crate::notifications::wake(&self.app, &self.site);
            }
            Err(error) => {
                crate::session_restore::cancel(&self.app, &self.site);
                let _ = self.app.emit_to("shell", "shell://error", error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_only_the_current_navigation_and_stops_the_timer_at_commit() {
        let mut n = Navigation::default();
        n.policy(true);
        let first = n.started(1).unwrap();
        assert!(n.claim(first, Stage::Provisional));
        assert!(n.is_recovering(first));
        n.update(1, Some(Stage::Committed));
        assert!(!n.is_recovering(first));
        assert!(!n.claim(first, Stage::Provisional));

        let failure = n.update(1, Some(Stage::Failed)).unwrap();
        assert!(n.claim(failure, Stage::Failed));
        n.policy(true);
        let next = n.started(2).unwrap();
        assert_eq!(n.update(1, Some(Stage::Failed)), None);
        assert!(!n.is_recovering(failure));
        n.update(2, None);
        assert!(!n.claim(next, Stage::Provisional));
        assert!(n.terminated().is_some());
        n.policy(false);
        assert!(n.terminated().is_none());
        assert!(n.started(3).is_none());
    }

    #[test]
    fn a_stall_and_failure_share_the_same_recovery_task() {
        let mut navigation = Navigation::default();
        navigation.policy(true);
        let revision = navigation.started(1).unwrap();
        assert!(navigation.claim(revision, Stage::Provisional));
        assert_eq!(navigation.update(1, Some(Stage::Failed)), Some(revision));
        assert!(navigation.is_recovering(revision));
        assert!(!navigation.claim(revision, Stage::Failed));
    }

    #[test]
    fn rebuilds_are_throttled_per_site_across_views() {
        let mut rebuilds = Rebuilds::default();
        let now = Instant::now();
        assert!(rebuilds.reserve("a", now));
        assert!(!rebuilds.reserve("a", now));
        assert!(rebuilds.reserve("b", now));
        assert!(rebuilds.reserve("a", now + REBUILD_INTERVAL));
    }

    #[tokio::test]
    async fn retries_a_temporary_outage_until_moodle_responds() {
        let mut responses = [Err(md_moodle::Error::Network), Ok(true)].into_iter();
        let result = check_account(
            &[Duration::ZERO; PROBE_DELAYS.len()],
            || true,
            || std::future::ready(responses.next().unwrap()),
        )
        .await;
        assert_eq!(result, Some(true));
        assert_eq!(responses.len(), 0);
    }

    #[tokio::test]
    async fn stops_probing_when_the_navigation_changes() {
        let mut navigation = Navigation::default();
        navigation.policy(true);
        let revision = navigation.started(1).unwrap();
        assert!(navigation.claim(revision, Stage::Provisional));
        let navigation = std::cell::RefCell::new(navigation);
        let mut probes = 0;
        let result = check_account(
            &[Duration::ZERO; PROBE_DELAYS.len()],
            || navigation.borrow().is_recovering(revision),
            || {
                probes += 1;
                navigation.borrow_mut().policy(false);
                std::future::ready(Err(md_moodle::Error::Network))
            },
        )
        .await;
        assert_eq!(result, None);
        assert_eq!(probes, 1);
    }

    #[tokio::test]
    async fn limits_probes_on_a_persistent_outage_and_stops_for_invalid_credentials() {
        for (transient, expected, count) in
            [(true, None, PROBE_DELAYS.len()), (false, Some(false), 1)]
        {
            let mut probes = 0;
            let result = check_account(
                &[Duration::ZERO; PROBE_DELAYS.len()],
                || true,
                || {
                    probes += 1;
                    std::future::ready(Err(if transient {
                        md_moodle::Error::Network
                    } else {
                        md_moodle::Error::InvalidToken
                    }))
                },
            )
            .await;
            assert_eq!(result, expected);
            assert_eq!(probes, count);
        }
    }

    #[test]
    fn recognises_read_pages_within_the_saved_installation() {
        let base = Url::parse("https://example.org/campus/").unwrap();
        for path in ["", "my/", "course/view.php?id=42&section=3"] {
            assert!(browse_page(&base, &base.join(path).unwrap()));
        }
        for path in [
            "login/index.php",
            "course/edit.php?id=42",
            "course/view.php?id=42&sesskey=secret",
            "course/view.php?id=42&edit=on",
            "mod/quiz/attempt.php?attempt=1",
            "/course/view.php?id=42",
            "https://example.net/campus/my/",
        ] {
            assert!(!browse_page(&base, &base.join(path).unwrap()));
        }
    }
}
