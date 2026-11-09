use crate::AppState;
use md_moodle::{navigation, Navigation};
use tauri::{
    webview::WebviewBuilder, AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl,
};
use tauri_plugin_opener::OpenerExt;

pub const SIDEBAR: f64 = 72.;
pub const TOOLBAR: f64 = 44.;
#[cfg(target_os = "macos")]
pub const TITLEBAR: f64 = 32.;
#[cfg(not(target_os = "macos"))]
pub const TITLEBAR: f64 = 0.;
const CONTENT_TOP: f64 = TITLEBAR + TOOLBAR;
pub fn create_shell(app: &AppHandle) -> tauri::Result<()> {
    let builder = tauri::WindowBuilder::new(app, "main")
        .title("MoodBuro")
        .inner_size(1120., 760.)
        .min_inner_size(760., 540.);
    // The window stays opaque. AppKit overlays its real traffic lights while
    // the local shell paints the titlebar backing and centered title.
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    let window = builder.build()?;
    let appearance = app
        .state::<AppState>()
        .store
        .lock()
        .expect("store")
        .settings()
        .unwrap_or_default()
        .appearance;
    apply_appearance(app, appearance)?;
    window.add_child(
        WebviewBuilder::new("shell", WebviewUrl::App("index.html".into())).auto_resize(),
        LogicalPosition::new(0., 0.),
        LogicalSize::new(1120., 760.),
    )?;
    Ok(())
}
pub fn resize(app: &AppHandle) -> tauri::Result<()> {
    let window = app.get_window("main").expect("main window");
    let size = window
        .inner_size()?
        .to_logical::<f64>(window.scale_factor()?);
    for (label, view) in app.webviews() {
        if label.starts_with("site-") {
            view.set_position(LogicalPosition::new(SIDEBAR, CONTENT_TOP))?;
            view.set_size(LogicalSize::new(
                (size.width - SIDEBAR).max(1.),
                (size.height - CONTENT_TOP).max(1.),
            ))?;
        }
    }
    Ok(())
}
pub fn hide_sites(app: &AppHandle) -> tauri::Result<()> {
    for (label, view) in app.webviews() {
        if label.starts_with("site-") {
            view.hide()?;
        }
    }
    Ok(())
}
pub fn switch(app: &AppHandle, id: &str) -> Result<(), String> {
    switch_at(app, id, None)
}
pub fn switch_at(app: &AppHandle, id: &str, target: Option<url::Url>) -> Result<(), String> {
    let site = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .sites()
        .map_err(|_| "Could not read sites.")?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or("Unknown site.")?;
    let label = format!("site-{id}");
    if app.get_webview(&label).is_none() {
        let base = md_moodle::normalize_site(&site.wwwroot).map_err(|e| e.to_string())?;
        let policy_base = base.clone();
        let handle = app.clone();
        let builder = WebviewBuilder::new(
            &label,
            WebviewUrl::External(
                target.unwrap_or(base.join("my/").map_err(|_| "Invalid site URL.")?),
            ),
        )
        .on_navigation(move |target| match navigation(&policy_base, target) {
            Navigation::Allow => true,
            Navigation::External => {
                let _ = handle.opener().open_url(target.as_str(), None::<&str>);
                false
            }
            Navigation::Block => false,
        })
        // No unmanaged file downloads before the W5 capture implementation.
        .on_download(|_, _| false)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny);
        let builder = profile(builder, id)?;
        app.get_window("main")
            .ok_or("Main window missing.")?
            .add_child(
                builder,
                LogicalPosition::new(SIDEBAR, CONTENT_TOP),
                LogicalSize::new(1., 1.),
            )
            .map_err(|_| "Could not create the Moodle view.")?;
    }
    resize(app).map_err(|_| "Could not resize the Moodle view.")?;
    hide_sites(app).map_err(|_| "Could not switch sites.")?;
    app.get_webview(&label)
        .ok_or("Moodle view missing.")?
        .show()
        .map_err(|_| "Could not show the Moodle view.")?;
    *app.state::<AppState>()
        .active
        .lock()
        .map_err(|_| "Site state unavailable.")? = Some(id.to_owned());
    Ok(())
}

pub fn apply_appearance(app: &AppHandle, appearance: md_store::Appearance) -> tauri::Result<()> {
    let theme = match appearance {
        md_store::Appearance::System => None,
        md_store::Appearance::Light => Some(tauri::Theme::Light),
        md_store::Appearance::Dark => Some(tauri::Theme::Dark),
    };
    for window in app.windows().values() {
        window.set_theme(theme)?;
    }
    Ok(())
}
pub fn isolated_profiles() -> bool {
    #[cfg(target_os = "macos")]
    {
        objc2::available!(macos = 14.0)
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}
pub fn profile(
    builder: WebviewBuilder<tauri::Wry>,
    id: &str,
) -> Result<WebviewBuilder<tauri::Wry>, String> {
    #[cfg(target_os = "macos")]
    if isolated_profiles() {
        let uuid = uuid::Uuid::parse_str(id).map_err(|_| "Invalid site identifier.")?;
        return Ok(builder.data_store_identifier(*uuid.as_bytes()));
    }
    let _ = id;
    Ok(builder)
}

pub fn remove_session(app: &AppHandle, site: &md_store::Site) -> Result<(), String> {
    let label = format!("site-{}", site.id);
    if isolated_profiles() {
        let view = match app.get_webview(&label) {
            Some(view) => view,
            None => {
                // Clear a lazy site's own data store without navigating to Moodle.
                let builder = profile(
                    WebviewBuilder::new(
                        &label,
                        WebviewUrl::External(url::Url::parse("about:blank").expect("blank URL")),
                    ),
                    &site.id,
                )?;
                let view = app
                    .get_window("main")
                    .ok_or("Main window missing.")?
                    .add_child(
                        builder,
                        LogicalPosition::new(SIDEBAR, CONTENT_TOP),
                        LogicalSize::new(1., 1.),
                    )
                    .map_err(|_| "Could not access the site’s browser data.")?;
                view.hide()
                    .map_err(|_| "Could not hide the browser data view.")?;
                view
            }
        };
        // WebKit schedules deletion asynchronously; never clear the shared shell data store.
        view.clear_all_browsing_data()
            .map_err(|_| "Could not clear the site’s browser data.")?;
        view.close()
            .map_err(|_| "Could not close the Moodle view.")?;
    } else if let Some(view) = app.get_webview(&label) {
        view.close()
            .map_err(|_| "Could not close the Moodle view.")?;
    }
    // Also clear matching legacy cookies from foundation builds using the default store.
    // Keep cookies that another saved site shares (old macOS profiles can share an IdP).
    let base = md_moodle::normalize_site(&site.wwwroot).map_err(|e| e.to_string())?;
    let host = base.host_str().ok_or("Invalid site host.")?;
    let other_hosts: Vec<String> = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .sites()
        .map_err(|_| "Could not read sites.")?
        .iter()
        .filter(|s| s.id != site.id)
        .filter_map(|s| {
            url::Url::parse(&s.wwwroot)
                .ok()?
                .host_str()
                .map(str::to_owned)
        })
        .collect();
    let shell = app.get_webview("shell").ok_or("Local shell missing.")?;
    for cookie in shell
        .cookies()
        .map_err(|_| "Could not read browser cookies.")?
    {
        if let Some(domain) = cookie.domain() {
            if cookie_matches(host, domain)
                && !other_hosts.iter().any(|h| cookie_matches(h, domain))
            {
                shell
                    .delete_cookie(cookie)
                    .map_err(|_| "Could not remove a site cookie.")?;
            }
        }
    }
    Ok(())
}
fn cookie_matches(host: &str, domain: &str) -> bool {
    let domain = domain.trim_start_matches('.');
    host.eq_ignore_ascii_case(domain)
        || host
            .to_ascii_lowercase()
            .ends_with(&format!(".{}", domain.to_ascii_lowercase()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cookie_domain_boundaries() {
        assert!(cookie_matches("moodle.example.org", ".example.org"));
        assert!(cookie_matches("EXAMPLE.org", "example.org"));
        assert!(!cookie_matches("notexample.org", "example.org"));
        assert!(!cookie_matches("example.org.evil.test", "example.org"));
    }
}
