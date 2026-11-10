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
fn popup_shell(view: &tauri::Webview) -> Result<(), String> {
    if view.window().label().starts_with("popup-")
        && view.label() == format!("chrome-{}", view.window().label())
    {
        Ok(())
    } else {
        Err("This command is restricted to the popup toolbar.".into())
    }
}
fn popup_target(
    app: &AppHandle,
    chrome: &tauri::Webview,
) -> Result<(md_store::Site, tauri::Webview), String> {
    let label = chrome.window().label().to_owned();
    let site = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .sites()
        .map_err(|_| "Could not read sites.")?
        .into_iter()
        .find(|s| label.starts_with(&format!("popup-{}-", s.id)))
        .ok_or("Unknown site.")?;
    let target = app.get_webview(&label).ok_or("Moodle view missing.")?;
    Ok((site, target))
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PopupContext {
    site_name: String,
    #[serde(flatten)]
    settings: md_store::Settings,
    can_go_back: bool,
    can_go_forward: bool,
}
#[tauri::command]
pub async fn popup_context(
    webview: tauri::Webview,
    app: AppHandle,
) -> Result<PopupContext, String> {
    popup_shell(&webview)?;
    let (site, target) = popup_target(&app, &webview)?;
    let settings = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Settings unavailable.")?
        .settings()
        .map_err(|_| "Could not read settings.")?;
    let (can_go_back, can_go_forward) = windows::history_state(
        &target,
        normalize_site(&site.wwwroot).map_err(|e| e.to_string())?,
    )
    .await?;
    Ok(PopupContext {
        site_name: site.name,
        settings,
        can_go_back,
        can_go_forward,
    })
}
#[tauri::command]
pub fn popup_nav(webview: tauri::Webview, app: AppHandle, action: String) -> Result<(), String> {
    popup_shell(&webview)?;
    let (site, target) = popup_target(&app, &webview)?;
    if action == "downloads" {
        windows::hide_sites(&app).map_err(|_| "Could not show the local shell.")?;
        crate::tray::show(&app);
        let _ = app.emit_to("shell", "shell://navigate", "downloads");
        return Ok(());
    }
    navigate_view(&app, &site.id, &target, &action)
}
#[tauri::command]
pub fn downloads_page(
    webview: tauri::Webview,
    app: AppHandle,
    query: String,
    site_id: Option<String>,
    state: Option<String>,
    offset: u32,
    course_id: Option<i64>,
) -> Result<crate::downloads::Page, String> {
    shell(&webview)?;
    crate::downloads::page(
        &app,
        &query,
        site_id.as_deref(),
        state.as_deref(),
        offset,
        course_id,
    )
}
#[tauri::command]
pub fn downloads_clear(
    webview: tauri::Webview,
    app: AppHandle,
    site_id: Option<String>,
) -> Result<(), String> {
    shell(&webview)?;
    crate::downloads::clear(&app, site_id.as_deref())
}
#[tauri::command]
pub fn download_copy_path(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    shell(&webview)?;
    crate::downloads::copy_path(&app, &id)
}
#[tauri::command]
pub fn inbox_list(
    webview: tauri::Webview,
    app: AppHandle,
) -> Result<Vec<md_store::InboxItem>, String> {
    shell(&webview)?;
    crate::library::inbox(&app)
}
#[tauri::command]
pub async fn inbox_mark_read(
    webview: tauri::Webview,
    app: AppHandle,
    site_id: String,
    id: Option<i64>,
) -> Result<(), String> {
    shell(&webview)?;
    crate::library::mark_read(&app, &site_id, id).await
}
#[tauri::command]
pub async fn site_open_page(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    page: String,
    value: Option<i64>,
) -> Result<(), String> {
    shell(&webview)?;
    crate::library::open_page(&app, &id, &page, value).await
}
#[tauri::command]
pub fn activities_list(
    webview: tauri::Webview,
    app: AppHandle,
) -> Result<Vec<md_store::Activity>, String> {
    shell(&webview)?;
    crate::library::activities(&app)
}
#[tauri::command]
pub fn courses_list(
    webview: tauri::Webview,
    app: AppHandle,
) -> Result<crate::library::Listing<md_store::Course>, String> {
    shell(&webview)?;
    crate::library::courses(&app)
}
#[tauri::command]
pub async fn courses_refresh(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    shell(&webview)?;
    crate::library::refresh_courses(&app, &id).await
}
#[tauri::command]
pub fn course_pin(
    webview: tauri::Webview,
    app: AppHandle,
    site_id: String,
    course_id: i64,
    pinned: bool,
) -> Result<(), String> {
    shell(&webview)?;
    crate::library::pin(&app, &site_id, course_id, pinned)
}
#[tauri::command]
pub fn agenda_list(
    webview: tauri::Webview,
    app: AppHandle,
) -> Result<crate::library::Listing<md_store::Event>, String> {
    shell(&webview)?;
    crate::library::agenda(&app)
}
#[tauri::command]
pub async fn agenda_refresh(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    shell(&webview)?;
    crate::library::refresh_agenda(&app, &id).await
}
#[tauri::command]
pub async fn download_preview(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    shell(&webview)?;
    crate::documents::preview(&app, &id).await
}
#[tauri::command]
pub async fn course_folders_refresh(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    shell(&webview)?;
    crate::library::refresh_folders(&app, &id).await
}
#[tauri::command]
pub fn download_open(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    confirmed: bool,
) -> Result<(), String> {
    shell(&webview)?;
    let path = crate::downloads::path(&app, &id, confirmed, true)?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|_| "Could not open the downloaded file.".into())
}
#[tauri::command]
pub fn download_reveal(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    shell(&webview)?;
    let path = crate::downloads::path(&app, &id, false, false)?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|_| "Could not reveal the downloaded file.".into())
}
#[tauri::command]
pub async fn deep_link_open(
    webview: tauri::Webview,
    app: AppHandle,
) -> Result<Option<crate::links::Outcome>, String> {
    shell(&webview)?;
    crate::links::open(&app).await
}
#[tauri::command]
pub fn unread_list(
    webview: tauri::Webview,
    app: AppHandle,
) -> Result<Vec<crate::notifications::Status>, String> {
    shell(&webview)?;
    Ok(crate::notifications::statuses(&app))
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
    // Invalidate the old worker before changing account identity or credentials.
    crate::notifications::stop(app, id);
    for (label, view) in app.webviews() {
        if label.starts_with(&format!("popup-{id}-")) {
            view.window()
                .close()
                .map_err(|_| "Could not close the Moodle view.")?;
        }
    }
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
        let mut restored = true;
        for (kind, previous) in [
            (Kind::Token, previous_token),
            (Kind::Private, previous_private),
        ] {
            if let Some(value) = previous {
                restored &= md_secrets::put(id, kind, &value).is_ok();
            } else {
                restored &= md_secrets::delete(id, kind).is_ok();
            }
        }
        if reauth && restored {
            if let Ok(site) = saved_site(app, id) {
                crate::notifications::start(app, site);
            }
        }
        return Err(error);
    }
    crate::notifications::start(app, saved_site(app, id)?);
    record(AuthEvent::CredentialsSaved);
    drop(pending);
    if let Some(view) = app.get_webview(&format!("site-{id}")) {
        #[cfg(target_os = "macos")]
        crate::pdf_save::remove(app, view.label());
        #[cfg(target_os = "macos")]
        crate::frame_guard::remove(app, view.label());
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
    restore_site(&app, &id).await
}
pub async fn restore_site(app: &AppHandle, id: &str) -> Result<(), String> {
    restore_site_at(app, id, None).await
}
pub async fn restore_site_at(
    app: &AppHandle,
    id: &str,
    destination: Option<Url>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _switch = state.switching.lock().await;
    let site = saved_site(app, id)?;
    // Only request a new autologin key when creating a view; warm switches are local.
    let cold = app.get_webview(&format!("site-{id}")).is_none();
    if cold {
        let base = normalize_site(&site.wwwroot).map_err(|e| e.to_string())?;
        let token = md_secrets::get(id, Kind::Token)
            .map_err(|_| "Reconnect this organisation in Settings to restore its credentials.")?;
        let client = Client::new(base.clone()).map_err(|e| e.to_string())?;
        // Validate first; an invalid token must prompt re-auth rather than silently showing a login page.
        record(AuthEvent::SessionRestoring);
        let info = client.site_info(&token).await.map_err(|e| e.to_string())?;
        if info.userid != site.user_id {
            return Err(md_moodle::Error::InvalidToken.to_string());
        }
        if let Ok(private) = md_secrets::get(id, Kind::Private) {
            match client.autologin_key(&token, &private).await {
                Ok(key) => {
                    let dashboard = base.join("my/").map_err(|_| "Invalid site URL.")?;
                    let target = md_moodle::autologin_target_to(
                        &base,
                        &key,
                        site.user_id,
                        destination.as_ref().unwrap_or(&dashboard),
                    )
                    .map_err(|e| e.to_string())?;
                    windows::switch_at(app, id, Some(target))?;
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
    windows::switch_at(app, id, destination)
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
    if !crate::menus::LOCAL_VIEWS.contains(&view.as_str()) {
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
    crate::notifications::publish(&app);
    for (label, _) in app.webviews() {
        if label.starts_with("chrome-popup-") {
            let _ = app.emit_to(label, "navigation://changed", ());
        }
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
    crate::notifications::stop(&app, &id);
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
    let (id, view) = windows::navigation_target(app)?;
    navigate_view(app, &id, &view, action)
}
pub fn navigate_view(
    app: &AppHandle,
    id: &str,
    view: &tauri::Webview,
    action: &str,
) -> Result<(), String> {
    let site = saved_site(app, id)?;
    let dashboard = normalize_site(&site.wwwroot)
        .map_err(|e| e.to_string())?
        .join("my/")
        .map_err(|_| "Invalid site URL.")?;
    match action {
        "back" => return windows::history(app, view, false, dashboard, id),
        "forward" => return windows::history(app, view, true, dashboard, id),
        "home" => {
            if view.label().starts_with("popup-") { let _ = view.window().close(); }
            windows::switch_at(app, id, Some(dashboard))?;
            crate::tray::show(app);
            let _ = app.emit_to("shell", "navigation://site-opened", id);
            return Ok(());
        }
        "reload" => view.reload(),
        "open_in_browser" => {
            let url = view.url().map_err(|_| "Could not read the current page.")?;
            if !md_moodle::shareable_url(&url) { return Err("This sign-in page cannot be shared with the browser. Open the organisation’s base URL instead.".into()); }
            return app.opener().open_url(url.as_str(), None::<&str>).map_err(|_| "Could not open the browser.".into());
        }
        _ => return Err("Unknown navigation action.".into()),
    }.map_err(|_| "Navigation failed.".into())
}
