mod assistant;
mod auth_log;
mod bookmarks;
mod calendar;
mod commands;
mod documents;
mod downloads;
#[cfg(target_os = "macos")]
mod frame_guard;
mod library;
mod links;
mod menus;
#[cfg(target_os = "macos")]
mod native_overlay;
#[cfg(target_os = "macos")]
mod native_preview;
mod notifications;
mod offline;
#[cfg(target_os = "macos")]
mod pdf_save;
mod reminders;
mod session_restore;
mod shortcut;
mod tray;
mod window_state;
mod windows;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// How long a Save request or an open-after-save mark waits for its response.
pub const OPEN_PENDING: std::time::Duration = std::time::Duration::from_secs(15);

pub struct AppState {
    store: Mutex<md_store::Store>,
    offline_job: tauri::async_runtime::Mutex<()>,
    workers: Mutex<notifications::Workers>,
    downloads: Mutex<downloads::Transfers>,
    active: Mutex<Option<String>>,
    pending: Mutex<Option<String>>,
    switching: tauri::async_runtime::Mutex<()>,
    restores: Mutex<session_restore::Restores>,
    /// Organisations whose next download was started by the "save and open" preference.
    open_next: Mutex<std::collections::HashMap<String, std::time::Instant>>,
    /// The saved global shortcut could not be registered at launch.
    shortcut_unavailable: std::sync::atomic::AtomicBool,
}

pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(links::Inbox::default()))
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            tray::show(app);
            for arg in args
                .iter()
                .skip(1)
                .filter(|arg| arg.to_ascii_lowercase().starts_with("moodledesk:"))
            {
                links::receive(app, arg);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(shortcut::plugin())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(AppState {
                store: Mutex::new(md_store::Store::open(dir.join("index.db"))?),
                offline_job: tauri::async_runtime::Mutex::new(()),
                workers: Mutex::new(Default::default()),
                downloads: Mutex::new(Default::default()),
                active: Mutex::new(None),
                pending: Mutex::new(None),
                switching: tauri::async_runtime::Mutex::new(()),
                restores: Mutex::new(Default::default()),
                open_next: Mutex::new(Default::default()),
                shortcut_unavailable: Default::default(),
            });
            app.state::<AppState>()
                .store
                .lock()
                .expect("store")
                .downloads_interrupt()?;
            md_secrets::configure_legacy_sites(
                app.state::<AppState>()
                    .store
                    .lock()
                    .expect("store")
                    .sites()?
                    .into_iter()
                    .map(|site| site.id),
            );
            assistant::install(app.handle());
            window_state::install(app.handle());
            windows::create_shell(app.handle())?;
            menus::install(app.handle())?;
            tray::install(app.handle())?;
            notifications::install_click_handler(app.handle())?;
            // A shortcut that cannot be registered must never stop the app from starting.
            shortcut::restore(app.handle());
            reminders::install(app.handle());
            let sites = app
                .state::<AppState>()
                .store
                .lock()
                .expect("store")
                .sites()?;
            for site in sites {
                notifications::start(app.handle(), site);
            }
            for arg in std::env::args()
                .skip(1)
                .filter(|arg| arg.to_ascii_lowercase().starts_with("moodledesk:"))
            {
                links::receive(app.handle(), &arg);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Resized(_) | tauri::WindowEvent::ScaleFactorChanged { .. } =
                event
            {
                if window.label().starts_with("popup-") {
                    let _ = windows::resize_popup(window);
                }
                if window.label() == "main" {
                    let _ = windows::resize(window.app_handle());
                    // Full screen hides the traffic lights; the toolbar reclaims their inset.
                    let full = window.is_fullscreen().unwrap_or(false);
                    let _ = window.emit_to("shell", "window://fullscreen", full);
                }
            }
            if window.label() == "main" {
                window_state::update(window);
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        window_state::save(window.app_handle());
                        let _ = window.hide();
                    }
                    // Inactive windows dim selection and toolbar text, as AppKit does.
                    tauri::WindowEvent::Focused(focused) => {
                        let _ = window.emit_to("shell", "window://focused", *focused);
                    }
                    tauri::WindowEvent::ThemeChanged(theme) => {
                        let _ = window.set_background_color(Some(windows::backing(*theme)));
                    }
                    _ => {}
                }
            }
            if let tauri::WindowEvent::Destroyed = event {
                #[cfg(target_os = "macos")]
                if window.label().starts_with("document-") {
                    native_preview::remove(window.label());
                }
                #[cfg(target_os = "macos")]
                if window.label().starts_with("popup-") {
                    pdf_save::remove(window.app_handle(), window.label());
                    frame_guard::remove(window.app_handle(), window.label());
                }
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
            commands::assistant_status,
            commands::assistant_configure,
            commands::assistant_key_set,
            commands::assistant_key_remove,
            commands::assistant_models,
            commands::assistant_panel,
            commands::assistant_history,
            commands::assistant_send,
            commands::assistant_cancel,
            commands::assistant_confirm,
            commands::assistant_clear,
            commands::assistant_open_file,
            commands::assistant_current_course,
            commands::reminders_picks,
            commands::reminders_pick,
            commands::offline_resources,
            commands::offline_saved,
            commands::offline_save,
            commands::offline_open,
            commands::sites_list,
            commands::site_move,
            commands::agenda_export,
            commands::popup_context,
            commands::popup_nav,
            commands::deep_link_open,
            commands::downloads_page,
            commands::downloads_clear,
            commands::download_copy_path,
            commands::inbox_list,
            commands::inbox_mark_read,
            commands::site_open_page,
            commands::courses_list,
            commands::activities_list,
            commands::courses_refresh,
            commands::course_pin,
            commands::agenda_list,
            commands::reminders_list,
            commands::reminders_set,
            commands::agenda_refresh,
            commands::download_preview,
            commands::course_folders_refresh,
            commands::download_open,
            commands::download_reveal,
            commands::unread_list,
            commands::site_label_set,
            commands::site_add,
            commands::site_switch,
            commands::show_onboarding,
            commands::nav,
            commands::site_reauthenticate,
            commands::site_remove,
            commands::shell_view,
            commands::session_status,
            commands::bookmarks_list,
            commands::bookmark_menu,
            commands::bookmark_capture,
            commands::bookmark_save,
            commands::bookmark_delete,
            commands::bookmark_open,
            commands::settings_get,
            commands::settings_set
        ])
        .build(tauri::generate_context!())
        .expect("Could not build MoodBuro")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                window_state::save(app);
                shortcut::release(app);
            }
            #[cfg(target_os = "macos")]
            match event {
                tauri::RunEvent::Reopen { .. } => tray::show(app),
                tauri::RunEvent::Opened { urls } => {
                    for url in urls {
                        links::receive(app, url.as_str());
                    }
                }
                _ => {}
            }
            #[cfg(not(target_os = "macos"))]
            let _ = event;
        });
}
