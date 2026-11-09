use crate::AppState;
use md_moodle::{navigation, Navigation};
use tauri::{
    webview::WebviewBuilder, AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl,
};
use tauri_plugin_opener::OpenerExt;

pub const SIDEBAR: f64 = 72.;
pub const TOOLBAR: f64 = 44.;
pub fn create_shell(app: &AppHandle) -> tauri::Result<()> {
    let window = tauri::WindowBuilder::new(app, "main")
        .title("MoodBuro")
        .inner_size(1120., 760.)
        .min_inner_size(760., 540.)
        .build()?;
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
            view.set_position(LogicalPosition::new(SIDEBAR, TOOLBAR))?;
            view.set_size(LogicalSize::new(
                (size.width - SIDEBAR).max(1.),
                (size.height - TOOLBAR).max(1.),
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
            WebviewUrl::External(base.join("my/").map_err(|_| "Invalid site URL.")?),
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
        app.get_window("main")
            .ok_or("Main window missing.")?
            .add_child(
                builder,
                LogicalPosition::new(SIDEBAR, TOOLBAR),
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
