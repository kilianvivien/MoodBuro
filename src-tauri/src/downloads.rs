//! WebKit owns the authenticated transfer. Rust selects its staging destination,
//! remembers macOS's missing completion path, and publishes without overwriting.
use crate::AppState;
use md_os::downloads as policy;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tauri::{webview::DownloadEvent, AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub type Transfers = HashMap<(String, String), Transfer>;
#[derive(Clone)]
pub struct Transfer {
    ambiguous: bool,
    finished: bool,
    user_id: i64,
    id: String,
    staged: PathBuf,
    folder: PathBuf,
    filename: String,
}
fn finish_transfer(
    transfers: &mut Transfers,
    key: &(String, String),
    success: bool,
) -> Option<Transfer> {
    let transfer = transfers.get_mut(key)?;
    if transfer.ambiguous {
        if !success || transfer.finished {
            return None;
        }
        transfer.finished = true;
        // Retain a tombstone: a late failure from the rejected request cannot
        // be mistaken for a subsequent transfer of the same URL.
        Some(transfer.clone())
    } else {
        transfers.remove(key)
    }
}
fn folder(app: &AppHandle, site: &md_store::Site) -> Result<PathBuf, String> {
    let root = app
        .path()
        .home_dir()
        .map_err(|_| "Download folder unavailable.")?
        .join("Moodle");
    policy::directory(&root).map_err(|_| "Download folder unavailable.")?;
    let id = uuid::Uuid::parse_str(&site.id).map_err(|_| "Invalid site identifier.")?;
    let site_dir = root.join(format!("{}-{}", policy::safe_name(&site.name), id));
    policy::directory(&site_dir).map_err(|_| "Download folder unavailable.")?;
    let folder = site_dir.join("Downloads");
    policy::directory(&folder).map_err(|_| "Download folder unavailable.")?;
    Ok(folder)
}
fn changed(app: &AppHandle) {
    let _ = app.emit_to("shell", "downloads://changed", ());
    let _ = crate::tray::refresh(app, &crate::notifications::statuses(app));
}
pub fn handle(app: &AppHandle, site_id: &str, event: DownloadEvent<'_>) -> bool {
    handle_for_user(app, site_id, event, None)
}
pub fn handle_for_user(
    app: &AppHandle,
    site_id: &str,
    event: DownloadEvent<'_>,
    expected_user: Option<i64>,
) -> bool {
    match event {
        DownloadEvent::Requested { url, destination } => {
            let result = (|| -> Result<(), String> {
                let state = app.state::<AppState>();
                let mut transfers = state
                    .downloads
                    .lock()
                    .map_err(|_| "Download unavailable.")?;
                let key = (site_id.to_owned(), url.to_string());
                let site = state
                    .store
                    .lock()
                    .map_err(|_| "Site storage unavailable.")?
                    .sites()
                    .map_err(|_| "Could not read sites.")?
                    .into_iter()
                    .find(|s| s.id == site_id)
                    .ok_or("Unknown site.")?;
                if expected_user.is_some_and(|user| user != site.user_id) {
                    return Err("This download is unavailable.".into());
                }
                let base = md_moodle::normalize_site(&site.wwwroot).map_err(|e| e.to_string())?;
                if md_moodle::navigation(&base, &url) != md_moodle::Navigation::Allow {
                    return Err("This download is outside the organisation’s secure site.".into());
                }
                if let Some(transfer) = transfers.get_mut(&key) {
                    transfer.ambiguous = true;
                    return Err("This file was requested twice. Restart MoodBuro before downloading it again.".into());
                }
                if transfers.len() >= 10 {
                    return Err(
                        "This file is already downloading or the download queue is full.".into(),
                    );
                }
                let folder = folder(app, &site)?;
                let staging = folder.join(".moodburo-staging");
                policy::directory(&staging).map_err(|_| "Download folder unavailable.")?;
                let id = uuid::Uuid::new_v4().to_string();
                let private = staging.join(&id);
                std::fs::create_dir(&private).map_err(|_| "Download folder unavailable.")?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700))
                        .map_err(|_| "Download folder unavailable.")?;
                }
                let filename = policy::safe_name(
                    destination
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("download"),
                );
                let staged = private.join(&filename);
                let item = md_store::Download {
                    id: id.clone(),
                    site_id: site_id.into(),
                    filename: filename.clone(),
                    local_path: staged.to_string_lossy().into_owned(),
                    state: "downloading".into(),
                    started_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                };
                if state
                    .store
                    .lock()
                    .map_err(|_| "Site storage unavailable.")?
                    .download_insert_for_user(&item, site.user_id)
                    .is_err()
                {
                    let _ = std::fs::remove_dir(&private);
                    return Err("Could not save download history.".into());
                }
                *destination = staged.clone();
                transfers.insert(
                    key,
                    Transfer {
                        id,
                        staged,
                        folder,
                        filename,
                        ambiguous: false,
                        finished: false,
                        user_id: site.user_id,
                    },
                );
                Ok(())
            })();
            if let Err(error) = result {
                let _ = app.emit_to("shell", "shell://error", error);
                return false;
            }
            changed(app);
            true
        }
        DownloadEvent::Finished { url, success, .. } => {
            let state = app.state::<AppState>();
            let key = (site_id.to_owned(), url.to_string());
            let transfer = {
                let mut transfers = state.downloads.lock().expect("downloads");
                if let Some(item) = transfers
                    .get(&key)
                    .filter(|item| item.ambiguous && !item.finished && !success)
                {
                    // A rejected duplicate may emit a failure while the original
                    // is still writing. Keep its reservation and never publish on failure.
                    let _ = state.store.lock().expect("store").download_finish(
                        &item.id,
                        &item.staged.to_string_lossy(),
                        "failed",
                    );
                }
                finish_transfer(&mut transfers, &key, success)
            };
            changed(app);
            if let Some(transfer) = transfer {
                // Account removal deletes its rows; never recreate them from a late callback.
                let saved = state
                    .store
                    .lock()
                    .expect("store")
                    .sites()
                    .unwrap_or_default()
                    .iter()
                    .any(|s| s.id == site_id && s.user_id == transfer.user_id);
                let completed = if success && saved {
                    policy::publish(&transfer.staged, &transfer.folder, &transfer.filename).ok()
                } else {
                    None
                };
                let path = completed
                    .as_deref()
                    .unwrap_or(&transfer.staged)
                    .to_string_lossy();
                let _ = state.store.lock().expect("store").download_finish(
                    &transfer.id,
                    &path,
                    if completed.is_some() {
                        "complete"
                    } else {
                        "failed"
                    },
                );
                let _ = std::fs::remove_file(&transfer.staged);
                if let Some(parent) = transfer.staged.parent() {
                    let _ = std::fs::remove_dir(parent);
                }
                if saved {
                    let fr = crate::menus::french(app);
                    let body = match (completed.is_some(), fr) {
                        (true, false) => "Download complete. Open Downloads in MoodBuro to view it.",
                        (true, true) => "Téléchargement terminé. Ouvrez Téléchargements dans MoodBuro pour le consulter.",
                        (false, false) => "Download failed. Please try again from Moodle.",
                        (false, true) => "Échec du téléchargement. Réessayez depuis Moodle.",
                    };
                    let _ = app
                        .notification()
                        .builder()
                        .title("MoodBuro")
                        .body(body)
                        .show();
                }
                changed(app);
            }
            true
        }
        _ => false,
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    #[serde(flatten)]
    item: md_store::Download,
    needs_confirmation: bool,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    items: Vec<Summary>,
    total: u64,
}
pub const PAGE_SIZE: u32 = 25;
pub fn page(
    app: &AppHandle,
    query: &str,
    site_id: Option<&str>,
    state: Option<&str>,
    offset: u32,
) -> Result<Page, String> {
    if !matches!(state, None | Some("downloading" | "complete" | "failed")) || query.len() > 256 {
        return Err("Unknown download filter.".into());
    }
    let filter = md_store::DownloadFilter {
        query,
        site_id,
        state,
    };
    let (items, total) = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .downloads_page(&filter, offset, PAGE_SIZE)
        .map_err(|_| "Could not read download history.")?;
    Ok(Page {
        items: items
            .into_iter()
            .map(|item| Summary {
                needs_confirmation: policy::needs_confirmation(Path::new(&item.filename)),
                item,
            })
            .collect(),
        total,
    })
}
/// Forgets finished history rows; the files themselves are never touched.
pub fn clear(app: &AppHandle, site_id: Option<&str>) -> Result<(), String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .downloads_clear(site_id)
        .map_err(|_| "Could not clear download history.")?;
    changed(app);
    Ok(())
}
/// Puts the validated path on the pasteboard from Rust; it never crosses IPC.
pub fn copy_path(app: &AppHandle, id: &str) -> Result<(), String> {
    let path = path(app, id, false, false)?;
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        let failed = || "Could not copy the file path.".to_owned();
        let mut child = std::process::Command::new("/usr/bin/pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|_| failed())?;
        child
            .stdin
            .take()
            .ok_or_else(failed)?
            .write_all(path.to_string_lossy().as_bytes())
            .map_err(|_| failed())?;
        if child.wait().map_err(|_| failed())?.success() {
            Ok(())
        } else {
            Err(failed())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err("Could not copy the file path.".into())
    }
}
pub fn path(app: &AppHandle, id: &str, confirmed: bool, opening: bool) -> Result<PathBuf, String> {
    let item = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .download(id)
        .map_err(|_| "Could not read download history.")?
        .filter(|d| d.state == "complete")
        .ok_or("This download is unavailable.")?;
    let path = PathBuf::from(&item.local_path);
    let root = app
        .path()
        .home_dir()
        .map_err(|_| "Download folder unavailable.")?
        .join("Moodle");
    // Reject moved/replaced files and symlinks escaping the managed folder.
    if std::fs::symlink_metadata(&root)
        .map_err(|_| "This download is unavailable.")?
        .file_type()
        .is_symlink()
    {
        return Err("This download is unavailable.".into());
    }
    let root = root
        .canonicalize()
        .map_err(|_| "This download is unavailable.")?;
    let actual = path
        .canonicalize()
        .map_err(|_| "This download is unavailable.")?;
    if !actual.starts_with(root)
        || !actual.is_file()
        || std::fs::symlink_metadata(&path)
            .map_err(|_| "This download is unavailable.")?
            .file_type()
            .is_symlink()
    {
        return Err("This download is unavailable.".into());
    }
    if opening && policy::needs_confirmation(&actual) && !confirmed {
        return Err("Confirm opening this file first.".into());
    }
    Ok(actual)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_failure_cannot_finish_original_or_a_later_transfer() {
        let key = ("site".into(), "https://example.org/file".into());
        let mut transfers = Transfers::new();
        transfers.insert(
            key.clone(),
            Transfer {
                id: "one".into(),
                staged: "/staged".into(),
                folder: "/folder".into(),
                filename: "file.pdf".into(),
                ambiguous: true,
                finished: false,
                user_id: 1,
            },
        );
        assert!(finish_transfer(&mut transfers, &key, false).is_none());
        assert_eq!(transfers.len(), 1);
        assert!(finish_transfer(&mut transfers, &key, true).is_some());
        assert!(finish_transfer(&mut transfers, &key, false).is_none());
        assert!(finish_transfer(&mut transfers, &key, true).is_none());
        assert_eq!(transfers.len(), 1); // Reserved until restart: never match a later transfer.
    }
}
