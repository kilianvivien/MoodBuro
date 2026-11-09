mod commands;
mod menus;
mod windows;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

pub struct AppState {
    store: Mutex<md_store::Store>,
    active: Mutex<Option<String>>,
    pending: Mutex<Option<String>>,
    switching: tauri::async_runtime::Mutex<()>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(AppState {
                store: Mutex::new(md_store::Store::open(dir.join("index.db"))?),
                active: Mutex::new(None),
                pending: Mutex::new(None),
                switching: tauri::async_runtime::Mutex::new(()),
            });
            windows::create_shell(app.handle())?;
            menus::install(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Resized(_) | tauri::WindowEvent::ScaleFactorChanged { .. } =
                event
            {
                if window.label() == "main" {
                    let _ = windows::resize(window.app_handle());
                }
            }
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(id) = window.label().strip_prefix("auth-") {
                    let state = window.state::<AppState>();
                    let mut pending = state.pending.lock().expect("pending state");
                    if pending.as_deref() == Some(id) {
                        *pending = None;
                        let _ =
                            window.emit_to("shell", "auth://finished", Some("Sign-in cancelled."));
                    }
                }
                if window.label() == "main" {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::sites_list,
            commands::site_add,
            commands::site_switch,
            commands::show_onboarding,
            commands::nav,
            commands::site_reauthenticate,
            commands::site_remove,
            commands::shell_view,
            commands::settings_get,
            commands::settings_set
        ])
        .run(tauri::generate_context!())
        .expect("Could not run MoodBuro");
}
