mod assistant;
mod auth_log;
mod bookmarks;
mod calendar;
mod calendar_subscription;
mod commands;
mod course_sync;
mod desktop;
mod documents;
#[cfg(target_os = "macos")]
mod download_control;
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
mod timetable;
#[cfg(target_os = "macos")]
mod timetable_calendar;
#[cfg(target_os = "macos")]
mod timetable_menu;
mod timetable_review;
mod tray;
mod updates;
mod uploads;
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

/// The metadata database. Debug builds use their own copy, seeded once from a
/// consistent snapshot of the release database, so a development schema never
/// migrates the database the installed app depends on.
fn store_path(dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let release = dir.join("index.db");
    if !cfg!(debug_assertions) {
        return Ok(release);
    }
    let dev = dir.join("index.dev.db");
    if !dev.exists() && release.exists() {
        md_store::Store::snapshot(&release, &dev).map_err(std::io::Error::other)?;
    }
    Ok(dev)
}

/// Run the native timetable engine's self-check; returns the exit status.
pub fn inspcal_self_check() -> i32 {
    match md_inspcal::smoke() {
        Ok(report) => {
            println!(
                "inspcal self-check ok: engine {} contract {} events {} calendar {} in {} ms",
                md_inspcal::ENGINE_VERSION,
                md_inspcal::ADAPTER_CONTRACT,
                report.events,
                report.calendar_sha256,
                report.elapsed_ms
            );
            0
        }
        Err(error) => {
            eprintln!("inspcal self-check failed: {error}");
            1
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(links::Inbox::default()))
        .manage(calendar_subscription::Operations::default())
        .manage(uploads::State::default())
        .manage(updates::State::default())
        .manage(timetable::State::default())
        .manage(timetable_review::Drafts::default())
        .manage(timetable_calendar::Worker::default())
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(shortcut::plugin())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(AppState {
                store: Mutex::new(md_store::Store::open(store_path(&dir)?)?),
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
            // Before windows/IPC and workers start: no active transfer can be removed.
            if let Ok(home) = app.path().home_dir() {
                md_os::offline::clean_staging(&home.join("Moodle"));
            }
            app.state::<AppState>()
                .store
                .lock()
                .expect("store")
                .recover_uploads()?;
            assistant::install(app.handle());
            window_state::install(app.handle());
            windows::create_shell(app.handle())?;
            menus::install(app.handle())?;
            tray::install(app.handle())?;
            desktop::restore(app.handle());
            #[cfg(target_os = "macos")]
            download_control::install(app.handle());
            notifications::install_click_handler(app.handle())?;
            // A shortcut that cannot be registered must never stop the app from starting.
            shortcut::restore(app.handle());
            reminders::install(app.handle());
            course_sync::install(app.handle());
            updates::start(app.handle());
            let sites = app
                .state::<AppState>()
                .store
                .lock()
                .expect("store")
                .sites()?;
            for site in sites {
                notifications::start(app.handle(), site);
            }
            timetable_calendar::start(app.handle());
            for arg in std::env::args()
                .skip(1)
                .filter(|arg| arg.to_ascii_lowercase().starts_with("moodledesk:"))
            {
                links::receive(app.handle(), &arg);
            }
            Ok(())
        })
        .on_webview_event(|view, event| {
            if view.window().label() == "main"
                && (view.label() == "shell" || view.label().starts_with("site-"))
            {
                if let tauri::WebviewEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) =
                    event
                {
                    if let Err(error) = uploads::drop_file(view.app_handle(), paths) {
                        let _ = view.emit_to("shell", "shell://error", error);
                    }
                }
            }
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
                if window.label() == "main" {
                    use tauri::Manager;
                    window
                        .app_handle()
                        .state::<timetable_review::Drafts>()
                        .clear();
                }
                #[cfg(target_os = "macos")]
                if window.label().starts_with("document-") {
                    native_preview::remove(window.label());
                }
                #[cfg(target_os = "macos")]
                if window.label().starts_with("popup-") {
                    pdf_save::remove(window.app_handle(), window.label());
                    #[cfg(target_os = "macos")]
                    timetable_menu::remove(window.app_handle(), window.label());
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
            commands::timetable_status,
            commands::timetable_prepare,
            commands::timetable_review,
            commands::timetable_commit,
            commands::timetable_discard,
            commands::timetable_weeks,
            commands::timetable_week,
            commands::timetable_edit,
            commands::timetable_remove,
            commands::timetable_preferences,
            commands::timetable_preferences_set,
            commands::timetable_open_link,
            commands::timetable_export,
            commands::timetable_calendar,
            commands::timetable_calendar_enable,
            commands::timetable_calendar_disable,
            commands::assistant_status,
            commands::assistant_configure,
            commands::assistant_key_set,
            commands::assistant_key_remove,
            commands::assistant_key_help,
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
            commands::offline_remove,
            commands::course_sync_list,
            commands::course_sync_configure,
            commands::course_sync_control,
            commands::sites_list,
            commands::site_move,
            commands::agenda_export,
            commands::download_cancel,
            commands::download_remove_file,
            commands::desktop_preferences,
            commands::desktop_preferences_set,
            commands::finder_moodle_folder,
            commands::upload_pick,
            commands::upload_files,
            commands::upload_assignments,
            commands::upload_prepare,
            commands::upload_confirm,
            commands::upload_stop,
            commands::upload_history,
            commands::upload_open,
            commands::upload_reviewed,
            commands::calendar_handoffs,
            commands::calendar_subscribe,
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
            commands::sidebar_menu,
            commands::bookmark_capture,
            commands::bookmark_save,
            commands::bookmark_delete,
            commands::bookmark_open,
            commands::updates_status,
            commands::updates_check,
            commands::updates_install,
            commands::updates_preference,
            commands::updates_restart,
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
                    let files: Vec<_> = urls
                        .iter()
                        .filter(|u| u.scheme() == "file")
                        .filter_map(|u| u.to_file_path().ok())
                        .collect();
                    if !files.is_empty() {
                        if let Err(error) = uploads::drop_file(app, &files) {
                            let _ = app.emit_to("shell", "shell://error", error);
                        }
                    }
                    for url in urls.into_iter().filter(|u| u.scheme() != "file") {
                        links::receive(app, url.as_str());
                    }
                }
                _ => {}
            }
            #[cfg(not(target_os = "macos"))]
            let _ = event;
        });
}
