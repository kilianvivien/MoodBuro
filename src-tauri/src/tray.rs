use crate::notifications::Status;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
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
            "tray-downloads" => {
                show(app);
                let _ = crate::windows::hide_sites(app);
                let _ = app.emit_to("shell", "shell://navigate", "downloads");
            }
            "tray-inbox" => {
                show(app);
                crate::menus::show_local_view(app, "inbox");
            }
            "tray-settings" => {
                show(app);
                let _ = crate::windows::hide_sites(app);
                let _ = app.emit_to("shell", "shell://navigate", "settings");
            }
            "tray-quit" => app.exit(0),
            id if id.starts_with("tray-site:") || id.starts_with("tray-home:") => {
                let home = id.starts_with("tray-home:");
                let id = id.split_once(':').expect("site menu id").1.to_owned();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let result = open_site(&app, &id, home).await;
                    show(&app);
                    if let Err(error) = result {
                        let _ = crate::windows::hide_sites(&app);
                        let _ = app.emit_to("shell", "shell://navigate", "settings");
                        let _ = app.emit_to("shell", "shell://error", error);
                    }
                });
            }
            id if id.starts_with("tray-notification:") => {
                if let Some((site, notification)) = id
                    .strip_prefix("tray-notification:")
                    .and_then(|s| s.rsplit_once(':'))
                {
                    if let Ok(notification) = notification.parse::<i64>() {
                        let cached = crate::library::inbox(app)
                            .unwrap_or_default()
                            .iter()
                            .any(|n| n.site_id == site && n.id == notification && !n.read);
                        if cached {
                            let app = app.clone();
                            let site = site.to_owned();
                            tauri::async_runtime::spawn(async move {
                                let result =
                                    crate::library::open_page(&app, &site, "notifications", None)
                                        .await;
                                show(&app);
                                if let Err(error) = result {
                                    crate::menus::show_local_view(&app, "inbox");
                                    let _ = app.emit_to("shell", "shell://error", error);
                                } else {
                                    let _ = app.emit_to("shell", "navigation://site-opened", site);
                                }
                            });
                        }
                    }
                }
            }
            id if id.starts_with("tray-reveal:") => {
                let id = id.strip_prefix("tray-reveal:").expect("download menu id");
                let result = crate::downloads::path(app, id, false, false).and_then(|path| {
                    use tauri_plugin_opener::OpenerExt;
                    app.opener()
                        .reveal_item_in_dir(path)
                        .map_err(|_| "Could not reveal the downloaded file.".to_owned())
                });
                if let Err(error) = result {
                    show(app);
                    let _ = app.emit_to("shell", "shell://error", error);
                }
            }
            _ => {}
        })
        .build(app)?;
    refresh(app, &[])
}
fn compact(value: &str, max: usize) -> String {
    let value: String = value.chars().filter(|c| !c.is_control()).collect();
    if value.chars().count() <= max {
        value
    } else {
        format!("{}…", value.chars().take(max - 1).collect::<String>())
    }
}
async fn open_site(app: &AppHandle, id: &str, home: bool) -> Result<(), String> {
    let destination = if home {
        let site = app
            .state::<crate::AppState>()
            .store
            .lock()
            .map_err(|_| "Site storage unavailable.")?
            .sites()
            .map_err(|_| "Could not read sites.")?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or("Unknown site.")?;
        Some(
            md_moodle::normalize_site(&site.wwwroot)
                .map_err(|e| e.to_string())?
                .join("my/")
                .map_err(|_| "Invalid site URL.")?,
        )
    } else {
        None
    };
    crate::commands::restore_site_at(app, id, destination).await?;
    let _ = app.emit_to("shell", "navigation://site-opened", id);
    Ok(())
}
fn latest_unread<'a>(items: &'a [md_store::InboxItem], site: &str) -> Vec<&'a md_store::InboxItem> {
    let mut unread: Vec<_> = items
        .iter()
        .filter(|n| n.site_id == site && !n.read)
        .collect();
    unread.sort_by_key(|n| std::cmp::Reverse((n.time_created, n.id)));
    unread.truncate(5);
    unread
}
fn count_all(statuses: &[Status]) -> u64 {
    statuses
        .iter()
        .fold(0_u64, |n, s| n.saturating_add(s.unread))
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
    let notifications = crate::library::inbox(app).unwrap_or_default();
    for site in sites {
        let status = statuses.iter().find(|s| s.site_id == site.id);
        let count = status.map_or(0, |s| s.unread);
        let status_text = match status.map(|s| s.connection) {
            Some("reauth") => Some(text("Reconnect in Settings", "Reconnecter dans Réglages")),
            Some("offline") => Some(text("Offline", "Hors ligne")),
            Some("error") => Some(text("Check failed", "Échec de la vérification")),
            _ => None,
        };
        let short = app
            .state::<crate::AppState>()
            .store
            .lock()
            .expect("store")
            .short_label(&site.id)
            .ok()
            .flatten();
        let name = compact(short.as_deref().unwrap_or(&site.name), 32);
        let label = if count == 0 {
            name
        } else {
            format!("{name} · {count}")
        };
        let open = MenuItemBuilder::with_id(
            format!("tray-site:{}", site.id),
            text("Open Moodle", "Ouvrir Moodle"),
        )
        .build(app)?;
        let home = MenuItemBuilder::with_id(
            format!("tray-home:{}", site.id),
            text("Moodle Home", "Accueil Moodle"),
        )
        .build(app)?;
        let mut site_menu = SubmenuBuilder::new(app, label).items(&[&open, &home]);
        let unread = latest_unread(&notifications, &site.id);
        if !unread.is_empty() {
            let mut latest = SubmenuBuilder::new(app, text("Latest Unread", "Dernières non lues"));
            for item in unread {
                latest = latest.item(
                    &MenuItemBuilder::with_id(
                        format!("tray-notification:{}:{}", site.id, item.id),
                        compact(&item.subject, 48),
                    )
                    .build(app)?,
                );
            }
            site_menu = site_menu.separator().item(&latest.build()?);
        }
        if let Some(status) = status_text {
            site_menu = site_menu
                .separator()
                .item(&MenuItemBuilder::new(status).enabled(false).build(app)?);
        }
        menu = menu.item(&site_menu.build()?);
    }
    let open = MenuItemBuilder::with_id("tray-open", text("Open MoodBuro", "Ouvrir MoodBuro"))
        .build(app)?;
    let inbox_label = if count_all(statuses) == 0 {
        text("Inbox…", "Boîte de réception…").to_owned()
    } else {
        format!(
            "{} · {}",
            text("Inbox…", "Boîte de réception…"),
            count_all(statuses)
        )
    };
    let inbox = MenuItemBuilder::with_id("tray-inbox", inbox_label).build(app)?;
    let check = MenuItemBuilder::with_id("tray-check", text("Check Now", "Vérifier maintenant"))
        .build(app)?;
    let settings =
        MenuItemBuilder::with_id("tray-settings", text("Settings…", "Réglages…")).build(app)?;
    let downloads_open = MenuItemBuilder::with_id(
        "tray-downloads",
        text("Open Downloads…", "Ouvrir les téléchargements…"),
    )
    .build(app)?;
    let mut downloads =
        SubmenuBuilder::new(app, text("Recent Downloads", "Téléchargements récents"))
            .item(&downloads_open)
            .separator();
    let recent = app
        .state::<crate::AppState>()
        .store
        .lock()
        .expect("store")
        .downloads_recent()
        .unwrap_or_default();
    if recent.is_empty() {
        downloads = downloads.item(
            &MenuItemBuilder::new(text("No downloads yet", "Aucun téléchargement"))
                .enabled(false)
                .build(app)?,
        );
    }
    for item in recent.into_iter().take(5) {
        let name = compact(&item.filename, 36);
        let suffix = match item.state.as_str() {
            "downloading" => text(" — Downloading", " — En cours"),
            "failed" => text(" — Failed", " — Échec"),
            _ => "",
        };
        // Selecting a recent item only reveals it; risky files never launch from the tray.
        downloads = downloads.item(
            &MenuItemBuilder::with_id(
                format!("tray-reveal:{}", item.id),
                format!("{name}{suffix}"),
            )
            .enabled(item.state == "complete")
            .build(app)?,
        );
    }
    let downloads = downloads.build()?;
    let quit = MenuItemBuilder::with_id("tray-quit", text("Quit MoodBuro", "Quitter MoodBuro"))
        .build(app)?;
    tray.set_menu(Some(
        menu.separator()
            .items(&[&open, &inbox, &check, &downloads, &settings])
            .separator()
            .item(&quit)
            .build()?,
    ))?;
    let count = count_all(statuses);
    tray.set_title(if count == 0 {
        None
    } else {
        Some(count.to_string())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn latest_items_are_unread_site_scoped_newest_and_bounded() {
        let items: Vec<_> = (0..12)
            .map(|i| md_store::InboxItem {
                site_id: if i == 11 { "other" } else { "site" }.into(),
                id: i,
                subject: "Subject".into(),
                time_created: i,
                read: i == 10,
            })
            .collect();
        assert_eq!(
            latest_unread(&items, "site")
                .iter()
                .map(|n| n.id)
                .collect::<Vec<_>>(),
            vec![9, 8, 7, 6, 5]
        );
    }
    #[test]
    fn menu_labels_are_bounded_and_unicode_safe() {
        assert_eq!(compact("CX", 32), "CX");
        assert_eq!(compact("é".repeat(40).as_str(), 32).chars().count(), 32);
        assert!(compact(
            "A very long organisation name that would stretch the menu",
            32
        )
        .ends_with('…'));
        assert_eq!(compact("Name\n", 32), "Name");
    }
}
