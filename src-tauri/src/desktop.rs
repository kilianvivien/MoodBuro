//! Optional Mac integration. Launch-at-login is changed only by an explicit setting.
use crate::AppState;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub menu_bar_only: bool,
    pub start_at_login: bool,
}
pub fn get(app: &AppHandle) -> Result<Preferences, String> {
    let menu = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Settings unavailable.")?
        .menu_bar_only()
        .map_err(|_| "Could not read settings.")?;
    let login = app
        .autolaunch()
        .is_enabled()
        .map_err(|_| "Could not read login settings.")?;
    Ok(Preferences {
        menu_bar_only: menu,
        start_at_login: login,
    })
}
pub fn set(
    app: &AppHandle,
    menu: Option<bool>,
    login: Option<bool>,
) -> Result<Preferences, String> {
    let before = get(app)?;
    if let Some(on) = login {
        if on {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        }
        .map_err(|_| "Could not change launch at login.")?;
    }
    if let Some(on) = menu {
        #[cfg(target_os = "macos")]
        if app
            .set_activation_policy(if on {
                tauri::ActivationPolicy::Accessory
            } else {
                tauri::ActivationPolicy::Regular
            })
            .is_err()
        {
            return Err("Could not change menu-bar mode.".into());
        }
        let saved = app
            .state::<AppState>()
            .store
            .lock()
            .map_err(|_| "Settings unavailable.")?
            .set_menu_bar_only(on);
        if saved.is_err() {
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(if before.menu_bar_only {
                tauri::ActivationPolicy::Accessory
            } else {
                tauri::ActivationPolicy::Regular
            });
            return Err("Could not save settings.".into());
        }
    }
    get(app)
}
pub fn restore(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    if app
        .state::<AppState>()
        .store
        .lock()
        .ok()
        .and_then(|s| s.menu_bar_only().ok())
        .unwrap_or(false)
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    }
    if std::env::args().any(|arg| arg == "--autostart") {
        if let Some(window) = app.get_window("main") {
            let _ = window.hide();
        }
    }
}
pub fn finder(app: &AppHandle) -> Result<(), String> {
    let root = app
        .path()
        .home_dir()
        .map_err(|_| "Download folder unavailable.")?
        .join("Moodle");
    md_os::downloads::directory(&root).map_err(|_| "Download folder unavailable.")?;
    app.opener()
        .reveal_item_in_dir(root)
        .map_err(|_| "Could not show the Moodle folder in Finder.".into())
}
