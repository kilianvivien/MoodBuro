use crate::AppState;
use md_moodle::{navigation, Navigation};
use tauri::{
    webview::WebviewBuilder, AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl,
};
use tauri_plugin_opener::OpenerExt;

// Shell geometry. Keep in sync with `--sidebar-width` / `--toolbar-height`
// in apps/shell/src/styles.css and docs/design-guidelines.md.
pub const SIDEBAR: f64 = 72.;
/// macOS: one unified 52 pt titlebar + toolbar, like Safari, Mail or Finder.
#[cfg(target_os = "macos")]
pub const TOOLBAR: f64 = 52.;
#[cfg(not(target_os = "macos"))]
pub const TOOLBAR: f64 = 44.;
const CONTENT_TOP: f64 = TOOLBAR;
/// Native traffic lights, vertically centred in the unified toolbar.
#[cfg(target_os = "macos")]
const TRAFFIC_LIGHTS: tauri::utils::config::LogicalPosition =
    tauri::utils::config::LogicalPosition { x: 20., y: 26. };
/// Opaque window backing (`--window` in styles.css): no white flash on launch.
const LIGHT_BACKING: tauri::window::Color = tauri::window::Color(245, 245, 247, 255);
const DARK_BACKING: tauri::window::Color = tauri::window::Color(30, 30, 31, 255);

pub fn create_shell(app: &AppHandle) -> tauri::Result<()> {
    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut config = tauri::utils::config::WindowConfig {
        label: "main".into(),
        title: "MoodBuro".into(),
        width: 1120.,
        height: 760.,
        min_width: Some(760.),
        min_height: Some(540.),
        ..Default::default()
    };
    // The window stays opaque. AppKit draws the real traffic lights over the
    // local shell, which paints the unified toolbar and its drag region.
    #[cfg(target_os = "macos")]
    {
        config.title_bar_style = tauri::TitleBarStyle::Overlay;
        config.hidden_title = true;
        config.traffic_light_position = Some(TRAFFIC_LIGHTS);
    }
    let window = tauri::WindowBuilder::from_config(app, &config)?.build()?;
    let appearance = app
        .state::<AppState>()
        .store
        .lock()
        .expect("store")
        .settings()
        .unwrap_or_default()
        .appearance;
    apply_appearance(app, appearance)?;
    let backing = backing(window.theme().unwrap_or(tauri::Theme::Light));
    window.set_background_color(Some(backing))?;
    window.add_child(
        WebviewBuilder::new("shell", WebviewUrl::App("index.html".into()))
            .background_color(backing)
            .auto_resize(),
        LogicalPosition::new(0., 0.),
        LogicalSize::new(1120., 760.),
    )?;
    Ok(())
}
pub fn backing(theme: tauri::Theme) -> tauri::window::Color {
    match theme {
        tauri::Theme::Dark => DARK_BACKING,
        _ => LIGHT_BACKING,
    }
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
    if let Some(view) = app.get_webview(&label) {
        if let Some(target) = target {
            view.navigate(target).map_err(|_| "Navigation failed.")?;
        }
    } else {
        let base = md_moodle::normalize_site(&site.wwwroot).map_err(|e| e.to_string())?;
        let policy_base = base.clone();
        let handle = app.clone();
        let download_app = app.clone();
        let download_site = id.to_owned();
        let download_user = site.user_id;
        let popup_app = app.clone();
        let popup_site = site.clone();
        let builder = WebviewBuilder::new(
            &label,
            WebviewUrl::External(
                target.unwrap_or(base.join("my/").map_err(|_| "Invalid site URL.")?),
            ),
        )
        .on_navigation(move |target| match navigation(&policy_base, target) {
            Navigation::Allow => true,
            Navigation::External => {
                if md_moodle::shareable_url(target) {
                    let _ = handle.opener().open_url(target.as_str(), None::<&str>);
                }
                false
            }
            Navigation::Block => false,
        })
        .on_download(move |_, event| {
            crate::downloads::handle_for_user(
                &download_app,
                &download_site,
                event,
                Some(download_user),
            )
        })
        .on_new_window(move |url, features| popup(&popup_app, &popup_site, url, features));
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
    #[cfg(target_os = "macos")]
    if let Some(view) = app.get_webview(&label) {
        crate::pdf_save::install(app, &view, id, site.user_id)
            .map_err(|_| "Could not prepare PDF saving.")?;
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
        // Automatic: read back the theme AppKit now resolves to.
        let resolved = theme.unwrap_or(window.theme().unwrap_or(tauri::Theme::Light));
        window.set_background_color(Some(backing(resolved)))?;
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
    for (label, view) in app.webviews() {
        if label.starts_with(&format!("popup-{}-", site.id)) {
            view.window()
                .close()
                .map_err(|_| "Could not close the Moodle view.")?;
        }
    }
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
        #[cfg(target_os = "macos")]
        crate::pdf_save::remove(app, &label);
    } else if let Some(view) = app.get_webview(&label) {
        view.close()
            .map_err(|_| "Could not close the Moodle view.")?;
        #[cfg(target_os = "macos")]
        crate::pdf_save::remove(app, &label);
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
/// Native menu commands follow the focused Moodle popup; shell controls target
/// the selected site. Popups never become privileged shell webviews.
pub fn navigation_target(app: &AppHandle) -> Result<(String, tauri::Webview), String> {
    let state = app.state::<AppState>();
    let sites = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .sites()
        .map_err(|_| "Could not read sites.")?;
    for (label, view) in app.webviews() {
        if label.starts_with("popup-") && view.window().is_focused().unwrap_or(false) {
            if let Some(site) = sites
                .iter()
                .find(|site| label.starts_with(&format!("popup-{}-", site.id)))
            {
                return Ok((site.id.clone(), view));
            }
        }
    }
    let id = state
        .active
        .lock()
        .map_err(|_| "Site state unavailable.")?
        .clone()
        .ok_or("Choose a site first.")?;
    let view = app
        .get_webview(&format!("site-{id}"))
        .ok_or("Moodle view missing.")?;
    Ok((id, view))
}

pub fn history(
    app: &AppHandle,
    view: &tauri::Webview,
    forward: bool,
    dashboard: url::Url,
    site_id: &str,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let fallback = view.clone();
        let site_id = site_id.to_owned();
        let app = app.clone();
        view.with_webview(move |native| unsafe {
            // Tauri runs this closure on the main thread and keeps WKWebView alive.
            let webview: &objc2_web_kit::WKWebView = &*native.inner().cast();
            if let Some(item) = native_history_item(webview, forward, &dashboard) {
                let _ = webview.goToBackForwardListItem(&item);
            } else if forward {
                // No useful forward entry.
            } else if fallback.label().starts_with("popup-") {
                let _ = fallback.window().close();
                let _ = switch(&app, &site_id);
                let _ = app.emit_to("shell", "navigation://site-opened", &site_id);
                crate::tray::show(&app);
            } else {
                let _ = fallback.navigate(dashboard);
            }
        })
        .map_err(|_| "Navigation failed.".to_owned())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, dashboard, site_id);
        view.eval(if forward {
            "history.forward()"
        } else {
            "history.back()"
        })
        .map_err(|_| "Navigation failed.".into())
    }
}

fn popup(
    app: &AppHandle,
    site: &md_store::Site,
    url: url::Url,
    features: tauri::webview::NewWindowFeatures,
) -> tauri::webview::NewWindowResponse<tauri::Wry> {
    use tauri::{webview::NewWindowResponse, WebviewWindowBuilder};
    let Ok(base) = md_moodle::normalize_site(&site.wwwroot) else {
        return NewWindowResponse::Deny;
    };
    let blank = url.as_str() == "about:blank";
    if !blank && navigation(&base, &url) != Navigation::Allow {
        if navigation(&base, &url) == Navigation::External && md_moodle::shareable_url(&url) {
            let _ = app.opener().open_url(url.as_str(), None::<&str>);
        }
        return NewWindowResponse::Deny;
    }
    let prefix = format!("popup-{}-", site.id);
    if app
        .webviews()
        .keys()
        .filter(|label| label.starts_with(&prefix))
        .count()
        >= 3
    {
        let _ = app.emit_to(
            "shell",
            "shell://error",
            "Close a Moodle popup window before opening another.",
        );
        return NewWindowResponse::Deny;
    }
    let label = format!("{prefix}{}", uuid::Uuid::new_v4());
    let policy_base = base;
    let navigation_app = app.clone();
    let download_app = app.clone();
    let download_site = site.id.clone();
    let download_user = site.user_id;
    let child_app = app.clone();
    let child_site = site.clone();
    let builder = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External(url::Url::parse("about:blank").expect("blank")),
    )
    // Preserves opener/window.close semantics and the originating WebKit session.
    .window_features(features)
    .title(&site.name)
    .inner_size(960., 720.)
    .min_inner_size(640., 480.)
    .on_navigation(move |url| {
        if url.as_str() == "about:blank" {
            return true;
        }
        match navigation(&policy_base, url) {
            Navigation::Allow => true,
            Navigation::External => {
                if md_moodle::shareable_url(url) {
                    let _ = navigation_app.opener().open_url(url.as_str(), None::<&str>);
                }
                false
            }
            Navigation::Block => false,
        }
    })
    .on_page_load(|view, _| {
        let _ = view.emit_to(
            format!("chrome-{}", view.label()),
            "navigation://changed",
            (),
        );
    })
    .on_download(move |_, event| {
        crate::downloads::handle_for_user(&download_app, &download_site, event, Some(download_user))
    })
    .on_new_window(move |url, features| popup(&child_app, &child_site, url, features));
    match builder.build() {
        Ok(window) => {
            let remote: &tauri::Webview = window.as_ref();
            if remote.set_auto_resize(false).is_err() {
                let _ = window.close();
                return NewWindowResponse::Deny;
            }
            let chrome = WebviewBuilder::new(
                format!("chrome-{label}"),
                WebviewUrl::App("index.html?popup".into()),
            );
            if remote
                .window()
                .add_child(
                    chrome,
                    LogicalPosition::new(0., 0.),
                    LogicalSize::new(960., 44.),
                )
                .is_err()
            {
                let _ = window.close();
                return NewWindowResponse::Deny;
            }
            let _ = resize_popup(&remote.window());
            #[cfg(target_os = "macos")]
            let _ = crate::pdf_save::install(app, window.as_ref(), &site.id, site.user_id);
            NewWindowResponse::Create { window }
        }
        Err(_) => NewWindowResponse::Deny,
    }
}

pub fn resize_popup(window: &tauri::Window) -> tauri::Result<()> {
    let size = window
        .inner_size()?
        .to_logical::<f64>(window.scale_factor()?);
    if let Some(remote) = window.app_handle().get_webview(window.label()) {
        remote.set_position(LogicalPosition::new(0., 44.))?;
        remote.set_size(LogicalSize::new(size.width, (size.height - 44.).max(1.)))?;
    }
    if let Some(chrome) = window
        .app_handle()
        .get_webview(&format!("chrome-{}", window.label()))
    {
        chrome.set_size(LogicalSize::new(size.width, 44.))?;
    }
    Ok(())
}
#[cfg(target_os = "macos")]
unsafe fn native_history_item(
    view: &objc2_web_kit::WKWebView,
    forward: bool,
    base: &url::Url,
) -> Option<objc2::rc::Retained<objc2_web_kit::WKBackForwardListItem>> {
    // Called only inside with_webview's main-thread closure. Retained list items
    // stay valid through goToBackForwardListItem without replaying one-use login URLs.
    unsafe {
        let list = view.backForwardList();
        let entries = if forward {
            list.forwardList()
        } else {
            list.backList()
        };
        let mut entries: Vec<_> = entries.iter().collect();
        if !forward {
            entries.reverse();
        }
        entries.into_iter().find(|item| {
            item.URL()
                .absoluteString()
                .and_then(|value| url::Url::parse(&value.to_string()).ok())
                .is_some_and(|target| md_moodle::history_url_allowed(base, &target))
        })
    }
}
pub async fn history_state(view: &tauri::Webview, base: url::Url) -> Result<(bool, bool), String> {
    #[cfg(target_os = "macos")]
    {
        let (send, receive) = tokio::sync::oneshot::channel();
        view.with_webview(move |native| unsafe {
            let webview: &objc2_web_kit::WKWebView = &*native.inner().cast();
            let _ = send.send((
                native_history_item(webview, false, &base).is_some(),
                native_history_item(webview, true, &base).is_some(),
            ));
        })
        .map_err(|_| "Navigation failed.")?;
        receive.await.map_err(|_| "Navigation failed.".into())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (view, base);
        Ok((true, true))
    }
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
