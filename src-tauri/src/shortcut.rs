//! Opt-in system-wide Quick Open shortcut (⌘⇧M). Registered from Rust only: no
//! webview holds a global-shortcut permission, and the key combination is fixed.
use crate::AppState;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

fn shortcut() -> Shortcut {
    #[cfg(target_os = "macos")]
    let command = Modifiers::SUPER;
    #[cfg(not(target_os = "macos"))]
    let command = Modifiers::CONTROL;
    Shortcut::new(Some(command | Modifiers::SHIFT), Code::KeyM)
}
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, pressed, event| {
            if event.state == ShortcutState::Pressed && *pressed == shortcut() {
                // Same path as ⌘L: bring the window forward, hide Moodle, focus the field.
                crate::tray::show(app);
                crate::menus::show_local_view(app, "quick-open");
            }
        })
        .build()
}
/// Registers or releases the shortcut. The caller saves the preference only on success.
pub fn set(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.global_shortcut();
    let key = shortcut();
    if manager.is_registered(key) == enabled {
        return Ok(());
    }
    if enabled {
        manager.register(key).map_err(|_| {
            "The ⌘⇧M shortcut could not be registered. Another app or a system shortcut may already use it."
        })?;
    } else {
        manager
            .unregister(key)
            .map_err(|_| "The ⌘⇧M shortcut could not be turned off.")?;
    }
    Ok(())
}
/// Launch: the preference stays as saved; Settings reports a failed registration.
pub fn restore(app: &AppHandle) {
    let state = app.state::<AppState>();
    let enabled = state
        .store
        .lock()
        .ok()
        .and_then(|store| store.settings().ok())
        .unwrap_or_default()
        .global_shortcut;
    state
        .shortcut_unavailable
        .store(enabled && set(app, true).is_err(), Ordering::Relaxed);
}
pub fn release(app: &AppHandle) {
    let _ = app.global_shortcut().unregister_all();
}
