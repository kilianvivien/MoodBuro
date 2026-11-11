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
pub const LOCAL_VIEWS: [&str; 9] = [
    "onboarding",
    "settings",
    "downloads",
    "inbox",
    "courses",
    "agenda",
    "quick-open",
    "bookmarks",
    "uploads",
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
    let calendar_subscription = MenuItemBuilder::with_id(
        "calendar-subscribe",
        text("Subscribe to Calendar…", "S’abonner au calendrier…"),
    )
    .accelerator("CmdOrCtrl+Alt+S")
    .build(app)?;
    let uploads = MenuItemBuilder::with_id(
        "uploads",
        text(
            "Upload Assignment Draft…",
            "Téléverser un brouillon de devoir…",
        ),
    )
    .accelerator("CmdOrCtrl+Shift+U")
    .build(app)?;
    let file = SubmenuBuilder::new(app, text("File", "Fichier"))
        .item(&add)
        .separator()
        .item(&save)
        .item(&uploads)
        .item(&calendar_subscription)
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
        if id == "ctx:check" {
            crate::notifications::check_now(app);
        } else if let Some(action) = id.strip_prefix("ctx:") {
            // A sidebar context-menu choice: the shell runs it through its own commands.
            let _ = app.emit_to("shell", "sidebar://action", action);
        } else if let Some(id) = id.strip_prefix("bookmark:") {
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
        } else if id == "calendar-subscribe" {
            show_local_view(app, "calendar-subscribe");
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

/// What a sidebar context menu offers: `(action, English, French, enabled)`; `None` is a
/// separator. Actions are fixed names, completed with `id` for an organisation or bookmark.
type ContextItem = Option<(String, &'static str, &'static str, bool)>;
fn context_items(kind: &str, id: &str, position: Option<(usize, usize)>) -> Vec<ContextItem> {
    let item = |action: &str, en, fr| Some((action.to_owned(), en, fr, true));
    let with = |action: &str, en, fr, enabled| Some((format!("{action}:{id}"), en, fr, enabled));
    match kind {
        "inbox" => vec![
            item(
                "view:inbox",
                "Open Notifications",
                "Ouvrir les notifications",
            ),
            None,
            item("mark-read", "Mark All as Read", "Tout marquer comme lu"),
            item(
                "check",
                "Check for Notifications",
                "Relever les notifications",
            ),
        ],
        "courses" => vec![
            item("view:courses", "Open Courses", "Ouvrir les cours"),
            None,
            item("refresh:courses", "Refresh Courses", "Actualiser les cours"),
        ],
        "agenda" => vec![
            item("view:agenda", "Open Agenda", "Ouvrir l’agenda"),
            None,
            item("refresh:agenda", "Refresh Agenda", "Actualiser l’agenda"),
            item(
                "calendar",
                "Subscribe to Calendar…",
                "S’abonner au calendrier…",
            ),
            item(
                "settings:notifications",
                "Reminder Settings…",
                "Réglages des rappels…",
            ),
        ],
        "site" => {
            let (index, count) = position.unwrap_or((0, 1));
            vec![
                with("site-open", "Open", "Ouvrir", true),
                with("site-home", "Moodle Home", "Accueil Moodle", true),
                with(
                    "site-notifications",
                    "Notifications in Moodle",
                    "Notifications dans Moodle",
                    true,
                ),
                with(
                    "site-messages",
                    "Messages in Moodle",
                    "Messages dans Moodle",
                    true,
                ),
                None,
                with(
                    "mark-read",
                    "Mark Notifications as Read",
                    "Marquer les notifications comme lues",
                    true,
                ),
                None,
                with("site-up", "Move Up", "Monter", index > 0),
                with("site-down", "Move Down", "Descendre", index + 1 < count),
                None,
                with("site-reauth", "Sign In Again…", "Se reconnecter…", true),
                item(
                    "settings:organisations",
                    "Organisation Settings…",
                    "Réglages des organisations…",
                ),
            ]
        }
        "add" => vec![item(
            "view:onboarding",
            "Add Organisation…",
            "Ajouter une organisation…",
        )],
        "bookmark" => vec![
            with("bookmark-open", "Open", "Ouvrir", true),
            with(
                "bookmark-edit",
                "Edit Bookmark…",
                "Modifier le signet…",
                true,
            ),
            None,
            item("bookmarks", "Edit Bookmarks…", "Modifier les signets…"),
        ],
        "settings" => vec![
            item("settings:general", "General", "Général"),
            item("settings:files", "Files", "Fichiers"),
            item("settings:notifications", "Reminders", "Rappels"),
            item("settings:assistant", "AI Assistant", "Assistant IA"),
            item("settings:organisations", "Organisations", "Organisations"),
        ],
        _ => Vec::new(),
    }
}
/// Shows the native context menu of a sidebar item at the pointer. The menu only names
/// fixed actions on a saved organisation or bookmark; the shell performs the chosen one.
pub fn sidebar_context(app: &AppHandle, kind: &str, id: Option<&str>) -> Result<(), String> {
    use tauri::menu::ContextMenu;
    const FAILED: &str = "Could not show the menu.";
    let state = app.state::<crate::AppState>();
    let (id, position) = match kind {
        "site" => {
            let sites = state
                .store
                .lock()
                .map_err(|_| "Site storage unavailable.")?
                .sites()
                .map_err(|_| "Could not read sites.")?;
            let id = id.ok_or(FAILED)?;
            let index = sites.iter().position(|s| s.id == id).ok_or(FAILED)?;
            (id, Some((index, sites.len())))
        }
        "bookmark" => {
            let id = id.ok_or(FAILED)?;
            if !crate::bookmarks::list(app)?.iter().any(|b| b.id == id) {
                return Err(FAILED.into());
            }
            (id, None)
        }
        _ => ("", None),
    };
    let items = context_items(kind, id, position);
    if items.is_empty() {
        return Err(FAILED.into());
    }
    let fr = french(app);
    let mut menu = MenuBuilder::new(app);
    for item in items {
        menu = match item {
            None => menu.separator(),
            Some((action, en, french, enabled)) => menu.item(
                &MenuItemBuilder::with_id(format!("ctx:{action}"), if fr { french } else { en })
                    .enabled(enabled)
                    .build(app)
                    .map_err(|_| FAILED)?,
            ),
        };
    }
    let menu = menu.build().map_err(|_| FAILED)?;
    let window = app.get_window("main").ok_or("Main window missing.")?;
    menu.popup(window).map_err(|_| FAILED.into())
}

#[cfg(test)]
mod context_tests {
    use super::context_items;
    #[test]
    fn context_menus_name_fixed_actions_and_respect_the_organisation_order() {
        let first = context_items("site", "a", Some((0, 2)));
        let find = |items: &[super::ContextItem], action: &str| {
            items
                .iter()
                .flatten()
                .find(|item| item.0 == action)
                .map(|item| item.3)
        };
        assert_eq!(find(&first, "site-up:a"), Some(false));
        assert_eq!(find(&first, "site-down:a"), Some(true));
        let only = context_items("site", "a", Some((0, 1)));
        assert_eq!(find(&only, "site-down:a"), Some(false));
        // Nothing destructive, and nothing for a kind the sidebar does not have.
        for kind in [
            "inbox", "courses", "agenda", "site", "add", "bookmark", "settings",
        ] {
            let items = context_items(kind, "a", None);
            assert!(!items.is_empty());
            assert!(items
                .iter()
                .flatten()
                .all(|item| !item.0.contains("remove")));
        }
        assert!(context_items("other", "a", None).is_empty());
    }
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
