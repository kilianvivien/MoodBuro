use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    AppHandle, Emitter, Manager,
};
pub fn is_french(language: md_store::Language) -> bool {
    match language {
        md_store::Language::Fr => true,
        md_store::Language::En => false,
        md_store::Language::System => {
            #[cfg(target_os = "macos")]
            {
                preferred_french(
                    objc2_foundation::NSLocale::preferredLanguages()
                        .iter()
                        .map(|s| s.to_string()),
                )
            }
            #[cfg(not(target_os = "macos"))]
            {
                preferred_french(std::iter::once(std::env::var("LANG").unwrap_or_default()))
            }
        }
    }
}
pub fn french(app: &AppHandle) -> bool {
    let language = app
        .state::<crate::AppState>()
        .store
        .lock()
        .ok()
        .and_then(|store| store.settings().ok())
        .unwrap_or_default()
        .language;
    is_french(language)
}
/// Local shell views a menu, the tray or the shell itself may show instead of a Moodle view.
/// Quick Open overlays the current view and preserves the active account.
pub const LOCAL_VIEWS: [&str; 8] = [
    "onboarding",
    "settings",
    "downloads",
    "inbox",
    "courses",
    "agenda",
    "quick-open",
    "bookmarks",
];
/// Hides the Moodle views and asks the shell to show one of its local views.
pub fn show_local_view(app: &AppHandle, view: &str) {
    if view == "quick-open" {
        let _ = app.emit_to("shell", "shell://navigate", view);
        return;
    }
    let _ = crate::windows::hide_sites(app);
    if let Ok(mut active) = app.state::<crate::AppState>().active.lock() {
        *active = None;
    }
    // A Moodle view may hold keyboard focus; the local view (and Quick Open's field) needs it.
    if let Some(shell) = app.get_webview("shell") {
        let _ = shell.set_focus();
    }
    let _ = app.emit_to("shell", "shell://navigate", view);
}
pub fn rebuild(app: &AppHandle, language: md_store::Language) -> tauri::Result<()> {
    let fr = is_french(language);
    let text = |en, fr_text| if fr { fr_text } else { en };
    let settings = MenuItemBuilder::with_id("settings", text("Settings…", "Réglages…"))
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let add = MenuItemBuilder::with_id(
        "add-site",
        text("Add Organisation…", "Ajouter une organisation…"),
    )
    .accelerator("CmdOrCtrl+N")
    .build(app)?;
    let save = MenuItemBuilder::with_id(
        "save",
        text("Save to Downloads", "Enregistrer dans Téléchargements"),
    )
    .accelerator("CmdOrCtrl+S")
    .build(app)?;
    let home = MenuItemBuilder::with_id("home", text("Moodle Home", "Accueil Moodle"))
        .accelerator("CmdOrCtrl+Shift+H")
        .build(app)?;
    let downloads = MenuItemBuilder::with_id("downloads", text("Downloads", "Téléchargements"))
        .accelerator("CmdOrCtrl+Alt+L")
        .build(app)?;
    let inbox = MenuItemBuilder::with_id("inbox", text("Notifications", "Notifications"))
        .accelerator("CmdOrCtrl+1")
        .build(app)?;
    let courses = MenuItemBuilder::with_id("courses", text("Courses", "Cours"))
        .accelerator("CmdOrCtrl+2")
        .build(app)?;
    let agenda = MenuItemBuilder::with_id("agenda", text("Agenda", "Agenda"))
        .accelerator("CmdOrCtrl+3")
        .build(app)?;
    let quick_open =
        MenuItemBuilder::with_id("quick-open", text("Quick Open…", "Ouverture rapide…"))
            .accelerator("CmdOrCtrl+K")
            .build(app)?;
    let assistant = MenuItemBuilder::with_id("assistant", text("AI Assistant", "Assistant IA"))
        .accelerator("CmdOrCtrl+Shift+A")
        .build(app)?;
    let reload = MenuItemBuilder::with_id("reload", text("Reload Page", "Recharger la page"))
        .accelerator("CmdOrCtrl+R")
        .build(app)?;
    let back = MenuItemBuilder::with_id("back", text("Back", "Page précédente"))
        .accelerator("CmdOrCtrl+[")
        .build(app)?;
    let forward = MenuItemBuilder::with_id("forward", text("Forward", "Page suivante"))
        .accelerator("CmdOrCtrl+]")
        .build(app)?;
    let browser = MenuItemBuilder::with_id(
        "open_in_browser",
        text("Open in Default Browser", "Ouvrir dans le navigateur"),
    )
    .accelerator("CmdOrCtrl+Shift+B")
    .build(app)?;
    let app_menu = SubmenuBuilder::new(app, "MoodBuro")
        .about_with_text(text("About MoodBuro", "À propos de MoodBuro"), None)
        .separator()
        .item(&settings)
        .separator()
        .hide_with_text(text("Hide MoodBuro", "Masquer MoodBuro"))
        .hide_others_with_text(text("Hide Others", "Masquer les autres"))
        .show_all_with_text(text("Show All", "Tout afficher"))
        .separator()
        .quit_with_text(text("Quit MoodBuro", "Quitter MoodBuro"))
        .build()?;
    let bookmark = MenuItemBuilder::with_id(
        "bookmark-add",
        text("Bookmark This Page…", "Ajouter cette page aux signets…"),
    )
    .accelerator("CmdOrCtrl+D")
    .build(app)?;
    let manage_bookmarks = MenuItemBuilder::with_id(
        "bookmarks",
        text("Edit Bookmarks…", "Modifier les signets…"),
    )
    .build(app)?;
    let file = SubmenuBuilder::new(app, text("File", "Fichier"))
        .item(&add)
        .separator()
        .item(&save)
        .separator()
        .item(&bookmark)
        .item(&manage_bookmarks)
        .separator()
        .close_window_with_text(text("Close Window", "Fermer la fenêtre"))
        .build()?;
    let edit = SubmenuBuilder::new(app, text("Edit", "Édition"))
        .undo_with_text(text("Undo", "Annuler"))
        .redo_with_text(text("Redo", "Rétablir"))
        .separator()
        .cut_with_text(text("Cut", "Couper"))
        .copy_with_text(text("Copy", "Copier"))
        .paste_with_text(text("Paste", "Coller"))
        .select_all_with_text(text("Select All", "Tout sélectionner"))
        .build()?;
    let view = SubmenuBuilder::new(app, text("View", "Présentation"))
        .item(&inbox)
        .item(&courses)
        .item(&agenda)
        .item(&downloads)
        .separator()
        .item(&assistant)
        .separator()
        .item(&reload)
        .fullscreen_with_text(text("Enter Full Screen", "Activer le plein écran"))
        .build()?;
    let go = SubmenuBuilder::new(app, text("Go", "Aller"))
        .item(&quick_open)
        .separator()
        .item(&home)
        .item(&back)
        .item(&forward)
        .separator()
        .item(&browser)
        .build()?;
    let window = SubmenuBuilder::new(app, text("Window", "Fenêtre"))
        .minimize_with_text(text("Minimize", "Réduire"))
        .maximize_with_text(text("Zoom", "Agrandir"))
        .build()?;
    app.set_menu(
        MenuBuilder::new(app)
            .items(&[&app_menu, &file, &edit, &view, &go, &window])
            .build()?,
    )?;
    Ok(())
}
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let language = app
        .state::<crate::AppState>()
        .store
        .lock()
        .expect("store")
        .settings()
        .unwrap_or_default()
        .language;
    rebuild(app, language)?;
    app.on_menu_event(|app, event| {
        let id = event.id().as_ref();
        let view = if id == "add-site" { "onboarding" } else { id };
        if let Some(id) = id.strip_prefix("bookmark:") {
            let app = app.clone();
            let id = id.to_owned();
            tauri::async_runtime::spawn(async move {
                match crate::bookmarks::open(&app, &id).await {
                    Ok(()) => {
                        if let Ok(active) = app.state::<crate::AppState>().active.lock() {
                            if let Some(id) = active.as_ref() {
                                let _ = app.emit_to("shell", "navigation://site-opened", id);
                            }
                        }
                    }
                    Err(error) => {
                        let _ = app.emit_to("shell", "shell://error", error);
                    }
                }
            });
        } else if LOCAL_VIEWS.contains(&view) {
            show_local_view(app, view);
        } else if id == "bookmark-add" {
            let _ = app.emit_to("shell", "bookmarks://add", ());
        } else if id == "assistant" {
            // Not a local view: the panel sits beside the Moodle view, which stays visible.
            let _ = app.emit_to("shell", "assistant://toggle", ());
        } else if matches!(
            id,
            "home" | "back" | "forward" | "reload" | "open_in_browser" | "save"
        ) {
            if let Err(error) = crate::commands::navigate(app, id) {
                let _ = app.emit_to("shell", "shell://error", error);
            }
        }
    });
    Ok(())
}

fn preferred_french(languages: impl IntoIterator<Item = String>) -> bool {
    languages
        .into_iter()
        .find_map(|tag| {
            let tag = tag.to_ascii_lowercase();
            match tag.split(['-', '_']).next().unwrap_or_default() {
                "fr" => Some(true),
                "en" => Some(false),
                _ => None,
            }
        })
        .unwrap_or(false)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_language_matches_first_supported_system_preference() {
        assert!(preferred_french([
            "de-DE".into(),
            "fr-FR".into(),
            "en-US".into()
        ]));
        assert!(!preferred_french(["en-GB".into(), "fr-FR".into()]));
        assert!(!preferred_french(["de-DE".into()]));
        assert!(is_french(md_store::Language::Fr));
        assert!(!is_french(md_store::Language::En));
    }
}
