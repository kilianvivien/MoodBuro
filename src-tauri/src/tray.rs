use crate::notifications::Status;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    // Small monochrome template glyph, generated locally without another image asset.
    let mut rgba = vec![0_u8; 18 * 18 * 4];
    for y in 3..15 {
        for x in 3..15 {
            if !(5..13).contains(&x) || y == 3 || y == 14 || (y < 9 && (x == y || x + y == 17)) {
                rgba[(y * 18 + x) * 4 + 3] = 255;
            }
        }
    }
    TrayIconBuilder::with_id("unread")
        .icon(tauri::image::Image::new_owned(rgba, 18, 18))
        .icon_as_template(true)
        .tooltip("MoodBuro")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray-open" => show(app),
            "tray-check" => crate::notifications::check_now(app),
            "tray-settings" => {
                show(app);
                let _ = crate::windows::hide_sites(app);
                let _ = app.emit_to("shell", "shell://navigate", "settings");
            }
            "tray-quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    refresh(app, &[])
}
pub fn refresh(app: &AppHandle, statuses: &[Status]) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id("unread") else {
        return Ok(());
    };
    let fr = crate::menus::french(app);
    let text = |en, french| if fr { french } else { en };
    let sites = app
        .state::<crate::AppState>()
        .store
        .lock()
        .expect("store")
        .sites()
        .unwrap_or_default();
    let mut menu = MenuBuilder::new(app);
    for site in sites {
        let status = statuses.iter().find(|s| s.site_id == site.id);
        let count = status.map_or(0, |s| s.unread);
        let suffix = match status.map(|s| s.connection) {
            Some("reauth") => text(" — Reconnect in Settings", " — Reconnecter dans Réglages"),
            Some("offline") => text(" — Offline", " — Hors ligne"),
            Some("error") => text(" — Check failed", " — Échec de la vérification"),
            _ => "",
        };
        menu = menu.item(
            &MenuItemBuilder::new(format!("{}: {count}{suffix}", site.name))
                .enabled(false)
                .build(app)?,
        );
    }
    let open = MenuItemBuilder::with_id("tray-open", text("Open MoodBuro", "Ouvrir MoodBuro"))
        .build(app)?;
    let check = MenuItemBuilder::with_id("tray-check", text("Check Now", "Vérifier maintenant"))
        .build(app)?;
    let settings =
        MenuItemBuilder::with_id("tray-settings", text("Settings…", "Réglages…")).build(app)?;
    let quit = MenuItemBuilder::with_id("tray-quit", text("Quit MoodBuro", "Quitter MoodBuro"))
        .build(app)?;
    tray.set_menu(Some(
        menu.separator()
            .items(&[&open, &check, &settings])
            .separator()
            .item(&quit)
            .build()?,
    ))?;
    let count = statuses
        .iter()
        .fold(0_u64, |n, s| n.saturating_add(s.unread));
    tray.set_title(if count == 0 {
        None
    } else {
        Some(count.to_string())
    })?;
    Ok(())
}
