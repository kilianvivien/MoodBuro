//! Rust-owned signed updater. Remote webviews have no updater permissions.
use crate::AppState;
use std::{sync::Mutex, time::Duration};
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
const BETA_FEED: &str =
    "https://github.com/kilianvivien/MoodBuro/releases/download/beta-feed/beta.json";
const STABLE_FEED: &str =
    "https://github.com/kilianvivien/MoodBuro/releases/latest/download/latest.json";
const KEY: Option<&str> = option_env!("MOODBURO_UPDATE_PUBLIC_KEY");
#[derive(Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub configured: bool,
    pub automatic: bool,
    pub phase: String,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub received: u64,
    pub total: Option<u64>,
}
#[derive(Default)]
pub struct State {
    status: Mutex<Status>,
    pending: tauri::async_runtime::Mutex<Option<Update>>,
    operation: tauri::async_runtime::Mutex<()>,
}
fn configured() -> bool {
    cfg!(target_os = "macos") && KEY.is_some_and(|k| !k.trim().is_empty())
}
fn phase(app: &AppHandle, value: &str) {
    app.state::<State>()
        .status
        .lock()
        .expect("update status")
        .phase = value.into();
}
pub fn status(app: &AppHandle) -> Result<Status, String> {
    let mut value = app
        .state::<State>()
        .status
        .lock()
        .map_err(|_| "Update unavailable.")?
        .clone();
    value.configured = configured();
    value.automatic = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Settings unavailable.")?
        .automatic_updates()
        .map_err(|_| "Could not read settings.")?;
    if !value.configured {
        value.phase = "unconfigured".into();
    } else if value.phase.is_empty() {
        value.phase = "idle".into();
    }
    Ok(value)
}
pub fn preference(app: &AppHandle, enabled: bool) -> Result<(), String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Settings unavailable.")?
        .set_automatic_updates(enabled)
        .map_err(|_| "Could not save settings.".into())
}
// A stable install never opts into prereleases. Beta installs stay on their own feed.
fn same_channel(current: &str, offered: &str) -> bool {
    if current.contains("-beta.") {
        offered
            .split_once('-')
            .is_some_and(|(_, pre)| pre.starts_with("beta."))
    } else {
        !offered.contains('-')
    }
}
fn feed(current: &str) -> &'static str {
    if current.contains("-beta.") {
        BETA_FEED
    } else {
        STABLE_FEED
    }
}
fn trusted_download(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.username().is_empty()
        && url.password().is_none()
        && url
            .path()
            .starts_with("/kilianvivien/MoodBuro/releases/download/")
        && url.query().is_none()
        && url.fragment().is_none()
}
pub async fn check(app: &AppHandle) -> Result<(), String> {
    if !configured() {
        return Err("Update unavailable.".into());
    }
    let state = app.state::<State>();
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| "Update already running.")?;
    if status(app)?.phase == "ready" {
        return Ok(());
    }
    phase(app, "checking");
    let result = async {
        let updater = app
            .updater_builder()
            .pubkey(KEY.unwrap_or_default())
            .endpoints(vec![feed(&app.package_info().version.to_string())
                .parse()
                .expect("fixed HTTPS feed")])?
            .timeout(Duration::from_secs(120))
            .version_comparator(|current, release| {
                release.version > current
                    && same_channel(&current.to_string(), &release.version.to_string())
            })
            .build()?;
        updater.check().await
    }
    .await;
    match result {
        Ok(update) => {
            if update
                .as_ref()
                .is_some_and(|u| !trusted_download(&u.download_url))
            {
                phase(app, "checkFailed");
                return Err("Could not check for updates.".into());
            }
            let mut pending = state.pending.lock().await;
            let mut value = state.status.lock().expect("update status");
            value.version = update.as_ref().map(|u| u.version.clone());
            value.notes = update
                .as_ref()
                .and_then(|u| u.body.as_ref().map(|s| s.chars().take(12000).collect()));
            value.phase = if update.is_some() {
                "available"
            } else {
                "current"
            }
            .into();
            *pending = update;
            Ok(())
        }
        Err(_) => {
            phase(app, "checkFailed");
            Err("Could not check for updates.".into())
        }
    }
}
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<State>();
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| "Update already running.")?;
    let pending = state.pending.lock().await;
    let update = pending.as_ref().ok_or("Check for updates first.")?;
    {
        let mut value = state.status.lock().expect("update status");
        value.phase = "downloading".into();
        value.received = 0;
        value.total = None;
    }
    // download() verifies the embedded-key signature before install() touches the app.
    let bytes = update
        .download(
            |chunk, total| {
                let mut value = state.status.lock().expect("update status");
                value.received += chunk as u64;
                value.total = total;
            },
            || {},
        )
        .await
        .map_err(|_| {
            phase(app, "installFailed");
            "Could not verify or download the update.".to_string()
        })?;
    phase(app, "installing");
    update.install(bytes).map_err(|_| {
        phase(app, "installFailed");
        "Could not install the update.".to_string()
    })?;
    phase(app, "ready");
    Ok(())
}
pub fn start(app: &AppHandle) {
    if !configured() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            // Fail closed if preferences cannot be read. Manual checks remain available.
            if status(&app).is_ok_and(|s| s.automatic) {
                let _ = check(&app).await;
            }
            tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_and_beta_channels_stay_separate() {
        assert!(same_channel("0.2.0", "0.2.1"));
        assert!(same_channel("0.2.0-beta.1", "0.2.0-beta.2"));
        assert_eq!(feed("0.2.0"), STABLE_FEED);
        assert_eq!(feed("0.2.1-beta.1"), BETA_FEED);
        for offered in ["0.2.1-beta.1", "0.2.1-alpha.1", "0.2.1-rc.1"] {
            assert!(!same_channel("0.2.0", offered));
        }
        assert!(!same_channel("0.2.0-beta.1", "0.2.0"));
    }
    #[test]
    fn download_stays_in_our_release_repository() {
        assert!(trusted_download(&"https://github.com/kilianvivien/MoodBuro/releases/download/v0.2.0-beta.1/MoodBuro.app.tar.gz".parse().unwrap()));
        for u in [
            "http://github.com/kilianvivien/MoodBuro/releases/download/v1/app",
            "https://github.com/other/repo/releases/download/v1/app",
            "https://github.com/kilianvivien/MoodBuro/releases/download-evil/app",
        ] {
            assert!(!trusted_download(&u.parse().unwrap()));
        }
    }
}
