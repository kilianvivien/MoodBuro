use crate::{windows, AppState};
use md_moodle::{normalize_site, parse_launch_token, Client, Tokens};
use md_secrets::Kind;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};
use tauri_plugin_opener::OpenerExt;
use url::Url;

// AppManifest permissions enforce IPC access. This caller check is defence in depth.
fn shell(view: &tauri::Webview) -> Result<(), String> {
    if view.label() == "shell" {
        Ok(())
    } else {
        Err("This command is restricted to the local shell.".into())
    }
}
#[tauri::command]
pub fn sites_list(
    webview: tauri::Webview,
    state: tauri::State<AppState>,
) -> Result<Vec<md_store::Site>, String> {
    shell(&webview)?;
    state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .sites()
        .map_err(|_| "Could not read saved sites.".into())
}
#[tauri::command]
pub async fn site_add(webview: tauri::Webview, app: AppHandle, url: String) -> Result<(), String> {
    shell(&webview)?;
    let base = normalize_site(&url).map_err(|e| e.to_string())?;
    let config = Client::new(base.clone())
        .map_err(|e| e.to_string())?
        .public_config()
        .await
        .map_err(|e| e.to_string())?;
    let canonical = normalize_site(&config.wwwroot).map_err(|e| e.to_string())?;
    // A server cannot redirect authentication to an unrelated origin.
    if canonical.origin() != base.origin() {
        return Err(
            "The site reports a different base origin. Enter its canonical Moodle URL.".into(),
        );
    }
    let launch = config
        .launchurl
        .as_deref()
        .map(Url::parse)
        .transpose()
        .map_err(|_| "Invalid sign-in URL.")?
        .unwrap_or(
            canonical
                .join("admin/tool/mobile/launch.php")
                .map_err(|_| "Invalid sign-in URL.")?,
        );
    if launch.origin() != canonical.origin()
        || launch.scheme() != "https"
        || !launch.username().is_empty()
        || launch.password().is_some()
    {
        return Err("The site supplied an unsafe sign-in URL.".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    {
        let state = app.state::<AppState>();
        let sites = state
            .store
            .lock()
            .map_err(|_| "Site storage unavailable.")?
            .sites()
            .map_err(|_| "Could not read sites.")?;
        if sites.len() >= 10 {
            return Err("The alpha supports up to 10 sites.".into());
        }
        if sites.iter().any(|s| s.wwwroot == canonical.as_str()) {
            return Err("This Moodle site has already been added.".into());
        }
        let mut pending = state
            .pending
            .lock()
            .map_err(|_| "Sign-in state unavailable.")?;
        if pending.is_some() {
            return Err("Finish or close the current sign-in window first.".into());
        }
        *pending = Some(id.clone());
    }
    let passport = uuid::Uuid::new_v4().simple().to_string();
    let mut launch = launch;
    launch
        .query_pairs_mut()
        .append_pair("service", "moodle_mobile_app")
        .append_pair("passport", &passport)
        .append_pair("urlscheme", "moodlemobile");
    let handle = app.clone();
    let callback_id = id.clone();
    let base = canonical.clone();
    let consumed = std::sync::atomic::AtomicBool::new(false);
    let auth =
        tauri::WebviewWindowBuilder::new(&app, format!("auth-{id}"), WebviewUrl::External(launch))
            .title(format!("Sign in · {}", config.sitename))
            .inner_size(720., 780.)
            .on_navigation(move |target| {
                if matches!(target.scheme(), "moodlemobile" | "moodledesk") {
                    let state = handle.state::<AppState>();
                    let pending = state.pending.lock().expect("pending state");
                    if pending.as_deref() != Some(&callback_id)
                        || consumed.swap(true, std::sync::atomic::Ordering::SeqCst)
                    {
                        return false;
                    }
                    drop(pending); // The one-use flag rejects replay while the attempt remains reserved.
                    let parsed = parse_launch_token(target, base.as_str(), &passport);
                    let app = handle.clone();
                    let id = callback_id.clone();
                    let base = base.clone();
                    tauri::async_runtime::spawn(async move {
                        let result = match parsed {
                            Ok(tokens) => complete_sign_in(&app, &id, base, tokens).await,
                            Err(e) => Err(e.to_string()),
                        };
                        *app.state::<AppState>()
                            .pending
                            .lock()
                            .expect("pending state") = None;
                        if let Some(window) = app.get_webview_window(&format!("auth-{id}")) {
                            let _ = window.close();
                        }
                        let _ = app.emit_to("shell", "auth://finished", result.err());
                    });
                    return false;
                }
                // HTTPS IdP redirects are allowed only inside this unprivileged auth view.
                target.scheme() == "https"
                    && target.username().is_empty()
                    && target.password().is_none()
            })
            .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
            .build();
    if auth.is_err() {
        *app.state::<AppState>()
            .pending
            .lock()
            .map_err(|_| "Sign-in state unavailable.")? = None;
        return Err("Could not open the organisation sign-in window.".into());
    }
    Ok(())
}
async fn complete_sign_in(
    app: &AppHandle,
    id: &str,
    base: Url,
    tokens: Tokens,
) -> Result<(), String> {
    let info = Client::new(base.clone())
        .map_err(|e| e.to_string())?
        .site_info(&tokens.token)
        .await
        .map_err(|e| e.to_string())?;
    if app
        .state::<AppState>()
        .pending
        .lock()
        .map_err(|_| "Sign-in state unavailable.")?
        .as_deref()
        != Some(id)
    {
        return Err("Sign-in cancelled.".into());
    }
    // Store only after server verification; remove partial credentials on any failure.
    let save = || -> Result<(), String> {
        md_secrets::put(id, Kind::Token, &tokens.token)
            .map_err(|_| "Could not save the token in the OS credential store.")?;
        if let Some(private) = &tokens.private {
            md_secrets::put(id, Kind::Private, private)
                .map_err(|_| "Could not save the private token in the OS credential store.")?;
        }
        app.state::<AppState>()
            .store
            .lock()
            .map_err(|_| "Site storage unavailable.")?
            .insert(&md_store::Site {
                id: id.into(),
                wwwroot: base.to_string(),
                name: info.sitename,
                user_id: info.userid,
                full_name: info.fullname,
            })
            .map_err(|_| "Could not save the site metadata.")?;
        Ok(())
    };
    if let Err(error) = save() {
        let _ = md_secrets::delete_site(id);
        return Err(error);
    }
    let _ = app.emit_to("shell", "sites://changed", id);
    // W1 uses the browser session established during sign-in. Autologin bridging is W2.
    windows::switch(app, id)
}
#[tauri::command]
pub async fn site_switch(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    shell(&webview)?;
    windows::switch(&app, &id)
}
#[tauri::command]
pub fn show_onboarding(webview: tauri::Webview, app: AppHandle) -> Result<(), String> {
    shell(&webview)?;
    windows::hide_sites(&app).map_err(|_| "Could not show onboarding.".into())
}
#[tauri::command]
pub fn nav(webview: tauri::Webview, app: AppHandle, action: String) -> Result<(), String> {
    shell(&webview)?;
    let id = app
        .state::<AppState>()
        .active
        .lock()
        .map_err(|_| "Site state unavailable.")?
        .clone()
        .ok_or("Choose a site first.")?;
    let view = app
        .get_webview(&format!("site-{id}"))
        .ok_or("Moodle view missing.")?;
    match action.as_str() {
        "back" => view.eval("history.back()"),
        "forward" => view.eval("history.forward()"),
        "reload" => view.reload(),
        "open_in_browser" => {
            let url = view.url().map_err(|_| "Could not read the current page.")?;
            if url.scheme() != "https" {
                return Err("Only HTTPS pages can be opened.".into());
            }
            return app
                .opener()
                .open_url(url.as_str(), None::<&str>)
                .map_err(|_| "Could not open the browser.".into());
        }
        _ => return Err("Unknown navigation action.".into()),
    }
    .map_err(|_| "Navigation failed.".into())
}
