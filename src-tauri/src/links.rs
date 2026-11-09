//! Native/CLI input waits here until the privileged shell is ready. Only the most
//! recent request is retained; no raw URL is logged, persisted or emitted.
use crate::AppState;
use md_moodle::deep_link::{Link, Route};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Default)]
pub struct Inbox {
    pending: Option<Result<Link, ()>>,
}
#[derive(serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Outcome {
    Site { site_id: String },
    Onboarding { url: String },
}
pub fn receive(app: &AppHandle, input: &str) {
    // This inbox is managed before plugin setup, so early second launches queue safely.
    app.state::<std::sync::Mutex<Inbox>>()
        .lock()
        .expect("link inbox")
        .pending = Some(Link::parse(input).map_err(|_| ()));
    if app.try_state::<AppState>().is_some() {
        crate::tray::show(app);
    }
    let _ = app.emit_to("shell", "deep-link://pending", ());
}
pub async fn open(app: &AppHandle) -> Result<Option<Outcome>, String> {
    let state = app.state::<AppState>();
    let inbox = app.state::<std::sync::Mutex<Inbox>>();
    let pending = inbox
        .lock()
        .map_err(|_| "Site state unavailable.")?
        .pending
        .take();
    let Some(pending) = pending else {
        return Ok(None);
    };
    let link = pending.map_err(|_| md_moodle::Error::InvalidDeepLink.to_string())?;
    let sites = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .sites()
        .map_err(|_| "Could not read sites.")?;
    let route = link
        .route(sites.iter().map(|s| (s.id.as_str(), s.wwwroot.as_str())))
        .map_err(|e| e.to_string())?;
    match route {
        Route::Site { id, target } => {
            if let Err(error) = crate::commands::restore_site_at(app, &id, Some(target)).await {
                // Retain the destination for a successful reconnect, unless a newer
                // request has already arrived. Credential errors remain sanitized.
                let mut inbox = inbox.lock().map_err(|_| "Site state unavailable.")?;
                if inbox.pending.is_none() {
                    inbox.pending = Some(Ok(link));
                }
                return Err(error);
            }
            Ok(Some(Outcome::Site { site_id: id }))
        }
        Route::Onboarding { base } => {
            crate::windows::hide_sites(app).map_err(|_| "Could not show the local shell.")?;
            *state.active.lock().map_err(|_| "Site state unavailable.")? = None;
            Ok(Some(Outcome::Onboarding {
                url: base.to_string(),
            }))
        }
    }
}
