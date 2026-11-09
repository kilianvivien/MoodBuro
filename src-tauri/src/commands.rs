use crate::{
    auth_log::{record, Event as AuthEvent},
    windows, AppState,
};
use md_moodle::{normalize_site, parse_launch_token, Client, Tokens};
use md_secrets::Kind;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};
use tauri_plugin_opener::OpenerExt;
use url::Url;

fn shell(view: &tauri::Webview) -> Result<(), String> {
    if view.label() == "shell" {
        Ok(())
    } else {
        Err("This command is restricted to the local shell.".into())
    }
}
fn saved_site(app: &AppHandle, id: &str) -> Result<md_store::Site, String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .sites()
        .map_err(|_| "Could not read sites.")?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or("Unknown site.".into())
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteSummary {
    #[serde(flatten)]
    site: md_store::Site,
    short_label: Option<String>,
}
#[tauri::command]
pub fn sites_list(
    webview: tauri::Webview,
    state: tauri::State<AppState>,
) -> Result<Vec<SiteSummary>, String> {
    shell(&webview)?;
    let store = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?;
    store
        .sites()
        .map_err(|_| "Could not read saved sites.")?
        .into_iter()
        .map(|site| {
            let short_label = store
                .short_label(&site.id)
                .map_err(|_| "Could not read settings.")?;
            Ok(SiteSummary { site, short_label })
        })
        .collect()
}
#[tauri::command]
pub fn site_label_set(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    label: String,
) -> Result<Option<String>, String> {
    shell(&webview)?;
    saved_site(&app, &id)?;
    let label = label.trim().to_uppercase();
    if label.chars().count() > 4 || !label.chars().all(char::is_alphanumeric) {
        return Err("Use up to 4 letters or numbers for the short label.".into());
    }
    let label = if label.is_empty() { None } else { Some(label) };
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .set_short_label(&id, label.as_deref())
        .map_err(|_| "Could not save settings.")?;
    Ok(label)
}
#[tauri::command]
pub async fn site_add(webview: tauri::Webview, app: AppHandle, url: String) -> Result<(), String> {
    shell(&webview)?;
    begin_sign_in(&app, &url, None).await
}
#[tauri::command]
pub async fn site_reauthenticate(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    shell(&webview)?;
    let site = saved_site(&app, &id)?;
    begin_sign_in(&app, &site.wwwroot, Some(id)).await
}
async fn begin_sign_in(app: &AppHandle, url: &str, existing: Option<String>) -> Result<(), String> {
    let base = normalize_site(url).map_err(|e| e.to_string())?;
    let (_, config) = Client::discover(base.clone())
        .await
        .map_err(|e| e.to_string())?;
    let canonical = normalize_site(&config.wwwroot).map_err(|e| e.to_string())?;
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
    let reauth = existing.is_some();
    let id = existing.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let attempt = uuid::Uuid::new_v4().simple().to_string();
    {
        let state = app.state::<AppState>();
        let sites = state
            .store
            .lock()
            .map_err(|_| "Site storage unavailable.")?
            .sites()
            .map_err(|_| "Could not read sites.")?;
        if !reauth && sites.len() >= 10 {
            return Err("The alpha supports up to 10 sites.".into());
        }
        if sites
            .iter()
            .any(|s| s.wwwroot == canonical.as_str() && s.id != id)
        {
            return Err("This Moodle site has already been added.".into());
        }
        let mut pending = state
            .pending
            .lock()
            .map_err(|_| "Sign-in state unavailable.")?;
        if pending.is_some() {
            return Err("Finish or close the current sign-in window first.".into());
        }
        *pending = Some(attempt.clone());
    }
    record(AuthEvent::Started);
    let passport = attempt.clone();
    let mut launch = launch;
    launch
        .query_pairs_mut()
        .append_pair("service", "moodle_mobile_app")
        .append_pair("passport", &passport)
        .append_pair("urlscheme", "moodlemobile");
    let handle = app.clone();
    let callback_attempt = attempt.clone();
    let base = canonical.clone();
    let consumed = std::sync::atomic::AtomicBool::new(false);
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        format!("auth-{attempt}"),
        WebviewUrl::External(launch),
    )
    .title(format!(
        "{} · {}",
        if crate::menus::french(app) {
            "Connexion"
        } else {
            "Sign in"
        },
        config.sitename
    ))
    .inner_size(720., 780.);
    #[cfg(target_os = "macos")]
    if windows::isolated_profiles() {
        builder = builder.data_store_identifier(
            *uuid::Uuid::parse_str(&id)
                .map_err(|_| "Invalid site identifier.")?
                .as_bytes(),
        );
    }
    let auth = builder
        .on_navigation(move |target| {
            if matches!(target.scheme(), "moodlemobile" | "moodledesk") {
                let state = handle.state::<AppState>();
                if state.pending.lock().expect("pending state").as_deref()
                    != Some(&callback_attempt)
                    || consumed.swap(true, std::sync::atomic::Ordering::SeqCst)
                {
                    return false;
                }
                let parsed = parse_launch_token(target, base.as_str(), &passport);
                record(if parsed.is_ok() {
                    AuthEvent::CallbackVerified
                } else {
                    AuthEvent::CallbackRejected
                });
                let app = handle.clone();
                let id = id.clone();
                let base = base.clone();
                let attempt = callback_attempt.clone();
                tauri::async_runtime::spawn(async move {
                    let result = match parsed {
                        Ok(tokens) => {
                            complete_sign_in(&app, &id, &attempt, base, tokens, reauth).await
                        }
                        Err(e) => Err(e.to_string()),
                    };
                    let owns_attempt = {
                        let state = app.state::<AppState>();
                        let mut pending = state.pending.lock().expect("pending state");
                        if pending.as_deref() == Some(&attempt) {
                            *pending = None;
                            true
                        } else {
                            false
                        }
                    };
                    if let Some(window) = app.get_webview_window(&format!("auth-{attempt}")) {
                        let _ = window.close();
                    }
                    // Cancellation already emitted an outcome. A late callback
                    // must not finish a newer sign-in attempt in the shell.
                    if owns_attempt {
                        let _ = app.emit_to("shell", "auth://finished", result.err());
                    }
                });
                return false;
            }
            target.scheme() == "https"
                && target.username().is_empty()
                && target.password().is_none()
        })
        .on_download(|_, _| false)
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
    attempt: &str,
    base: Url,
    tokens: Tokens,
    reauth: bool,
) -> Result<(), String> {
    let info = Client::new(base.clone())
        .map_err(|e| e.to_string())?
        .site_info(&tokens.token)
        .await
        .map_err(|e| e.to_string())?;
    record(AuthEvent::TokenVerified);
    let state = app.state::<AppState>();
    let _switch = state.switching.lock().await;
    // Hold this reservation through persistence so cancellation/removal cannot race a late callback.
    let pending = state
        .pending
        .lock()
        .map_err(|_| "Sign-in state unavailable.")?;
    if pending.as_deref() != Some(attempt) {
        return Err("Sign-in cancelled.".into());
    }
    let previous_token = md_secrets::get_optional(id, Kind::Token).map_err(|_| {
        "Could not access the existing credential. Unlock the OS credential store and try again."
    })?;
    let previous_private = md_secrets::get_optional(id, Kind::Private)
        .map_err(|_| "Could not access the existing private credential.")?;
    let save = || -> Result<(), String> {
        md_secrets::put(id, Kind::Token, &tokens.token)
            .map_err(|_| "Could not save the token in the OS credential store.")?;
        if let Some(private) = &tokens.private {
            md_secrets::put(id, Kind::Private, private)
                .map_err(|_| "Could not save the private token in the OS credential store.")?;
        } else {
            md_secrets::delete(id, Kind::Private)
                .map_err(|_| "Could not update the private credential.")?;
        }
        let site = md_store::Site {
            id: id.into(),
            wwwroot: base.to_string(),
            name: info.sitename.clone(),
            user_id: info.userid,
            full_name: info.fullname.clone(),
        };
        let store = state
            .store
            .lock()
            .map_err(|_| "Site storage unavailable.")?;
        if reauth {
            store.update(&site)
        } else {
            store.insert(&site)
        }
        .map_err(|_| "Could not save the site metadata.")?;
        Ok(())
    };
    if let Err(error) = save() {
        // Restore existing credentials rather than destroying a working account on re-auth failure.
        for (kind, previous) in [
            (Kind::Token, previous_token),
            (Kind::Private, previous_private),
        ] {
            if let Some(value) = previous {
                let _ = md_secrets::put(id, kind, &value);
            } else {
                let _ = md_secrets::delete(id, kind);
            }
        }
        return Err(error);
    }
    record(AuthEvent::CredentialsSaved);
    drop(pending);
    if let Some(view) = app.get_webview(&format!("site-{id}")) {
        view.close()
            .map_err(|_| "Could not renew the Moodle view.")?;
    }
    windows::switch(app, id)?;
    let _ = app.emit_to("shell", "sites://changed", id);
    Ok(())
}
#[tauri::command]
pub async fn site_switch(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    shell(&webview)?;
    let state = app.state::<AppState>();
    let _switch = state.switching.lock().await;
    let site = saved_site(&app, &id)?;
    // Only request a new autologin key when creating a view; warm switches are local.
    let cold = app.get_webview(&format!("site-{id}")).is_none();
    if cold {
        let base = normalize_site(&site.wwwroot).map_err(|e| e.to_string())?;
        let token = md_secrets::get(&id, Kind::Token)
            .map_err(|_| "Reconnect this organisation in Settings to restore its credentials.")?;
        let client = Client::new(base.clone()).map_err(|e| e.to_string())?;
        // Validate first; an invalid token must prompt re-auth rather than silently showing a login page.
        record(AuthEvent::SessionRestoring);
        let info = client.site_info(&token).await.map_err(|e| e.to_string())?;
        if info.userid != site.user_id {
            return Err(md_moodle::Error::InvalidToken.to_string());
        }
        if let Ok(private) = md_secrets::get(&id, Kind::Private) {
            match client.autologin_key(&token, &private).await {
                Ok(key) => {
                    let target = md_moodle::autologin_target(&base, &key, site.user_id)
                        .map_err(|e| e.to_string())?;
                    windows::switch_at(&app, &id, Some(target))?;
                    record(AuthEvent::SessionRestored);
                    return Ok(());
                }
                Err(md_moodle::Error::InvalidToken) => {
                    return Err(md_moodle::Error::InvalidToken.to_string())
                }
                // A retained browser session remains valid when autologin is unavailable/rate-limited.
                Err(_) => {}
            }
        }
    }
    if cold {
        record(AuthEvent::BrowserSessionFallback);
    }
    windows::switch(&app, &id)
}
#[tauri::command]
pub fn show_onboarding(webview: tauri::Webview, app: AppHandle) -> Result<(), String> {
    shell(&webview)?;
    show_local(&app)
}
fn show_local(app: &AppHandle) -> Result<(), String> {
    windows::hide_sites(app).map_err(|_| "Could not show the local shell.")?;
    *app.state::<AppState>()
        .active
        .lock()
        .map_err(|_| "Site state unavailable.")? = None;
    Ok(())
}
#[tauri::command]
pub fn shell_view(webview: tauri::Webview, app: AppHandle, view: String) -> Result<(), String> {
    shell(&webview)?;
    if !matches!(view.as_str(), "onboarding" | "settings") {
        return Err("Unknown shell view.".into());
    }
    show_local(&app)
}
#[tauri::command]
pub fn settings_get(
    webview: tauri::Webview,
    state: tauri::State<AppState>,
) -> Result<md_store::Settings, String> {
    shell(&webview)?;
    state
        .store
        .lock()
        .map_err(|_| "Settings unavailable.")?
        .settings()
        .map_err(|_| "Could not read settings.".into())
}
#[tauri::command]
pub fn settings_set(
    webview: tauri::Webview,
    app: AppHandle,
    appearance: Option<md_store::Appearance>,
    language: Option<md_store::Language>,
) -> Result<md_store::Settings, String> {
    shell(&webview)?;
    let previous = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Settings unavailable.")?
        .settings()
        .map_err(|_| "Could not read settings.")?;
    let settings = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Settings unavailable.")?
        .set_preferences(appearance, language)
        .map_err(|_| "Could not save settings.")?;
    if windows::apply_appearance(&app, settings.appearance)
        .and_then(|()| crate::menus::rebuild(&app, settings.language))
        .is_err()
    {
        let _ = app
            .state::<AppState>()
            .store
            .lock()
            .map_err(|_| "Settings unavailable.")?
            .set_preferences(Some(previous.appearance), Some(previous.language));
        let _ = windows::apply_appearance(&app, previous.appearance);
        let _ = crate::menus::rebuild(&app, previous.language);
        return Err("Could not apply the application preferences.".into());
    }
    Ok(settings)
}
#[tauri::command]
pub async fn site_remove(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    shell(&webview)?;
    let state = app.state::<AppState>();
    let _switch = state.switching.lock().await;
    let site = saved_site(&app, &id)?;
    let pending = state
        .pending
        .lock()
        .map_err(|_| "Sign-in state unavailable.")?;
    if pending.is_some() {
        return Err(
            "Finish or close the current sign-in window before removing an organisation.".into(),
        );
    }
    windows::remove_session(&app, &site)?;
    md_secrets::delete_site(&id)
        .map_err(|_| "Could not remove this organisation’s credentials. Please try again.")?;
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .remove(&id)
        .map_err(|_| "Could not remove the saved organisation.")?;
    *app.state::<AppState>()
        .active
        .lock()
        .map_err(|_| "Site state unavailable.")? = None;
    windows::hide_sites(&app).map_err(|_| "Could not show the local shell.")?;
    drop(pending);
    // Removal refreshes the list without selecting a removed site.
    let _ = app.emit_to("shell", "sites://removed", &id);
    Ok(())
}
#[tauri::command]
pub fn nav(webview: tauri::Webview, app: AppHandle, action: String) -> Result<(), String> {
    shell(&webview)?;
    navigate(&app, &action)
}
pub fn navigate(app: &AppHandle, action: &str) -> Result<(), String> {
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
    match action {
        "back" => view.eval("history.back()"), "forward" => view.eval("history.forward()"), "reload" => view.reload(),
        "open_in_browser" => {
            let url = view.url().map_err(|_| "Could not read the current page.")?;
            if url.scheme() != "https" || url.query_pairs().any(|(k,_)| matches!(k.as_ref(), "key" | "token" | "wstoken" | "privatetoken")) { return Err("This sign-in page cannot be shared with the browser. Open the organisation’s base URL instead.".into()); }
            return app.opener().open_url(url.as_str(), None::<&str>).map_err(|_| "Could not open the browser.".into());
        }
        _ => return Err("Unknown navigation action.".into()),
    }.map_err(|_| "Navigation failed.".into())
}
