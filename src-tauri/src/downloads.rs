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

/// Shell-only transfer status: no source URL, destination path or credentials.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Feedback {
    site_id: String,
    user_id: i64,
    filename: String,
    state: &'static str,
}

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
    /// Started by the "save and open" preference for a PDF or image.
    open_after: bool,
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
pub(crate) fn folder(
    app: &AppHandle,
    site: &md_store::Site,
    course: Option<&md_store::Course>,
) -> Result<PathBuf, String> {
    let root = app
        .path()
        .home_dir()
        .map_err(|_| "Download folder unavailable.")?
        .join("Moodle");
    policy::directory(&root).map_err(|_| "Download folder unavailable.")?;
    let id = uuid::Uuid::parse_str(&site.id).map_err(|_| "Invalid site identifier.")?;
    let site_dir = root.join(format!("{}-{}", policy::safe_name(&site.name), id));
    policy::directory(&site_dir).map_err(|_| "Download folder unavailable.")?;
    let folder = site_dir.join(match course {
        Some(course) => format!(
            "{}-{}",
            policy::safe_name(if course.short_name.is_empty() {
                &course.name
            } else {
                &course.short_name
            }),
            course.id
        ),
        None => "Downloads".into(),
    });
    policy::directory(&folder).map_err(|_| "Download folder unavailable.")?;
    Ok(folder)
}
/// Completion and lookup share the account lock with offline publication/removal.
fn complete(
    store: &md_store::Store,
    root: &Path,
    site: &str,
    transfer: &Transfer,
) -> Option<PathBuf> {
    let item = store.download(&transfer.id).ok()??;
    if item.site_id != site
        || !store
            .sites()
            .ok()?
            .iter()
            .any(|s| s.id == site && s.user_id == transfer.user_id)
    {
        return None;
    }
    md_os::offline::managed_path(root, &transfer.staged).ok()?;
    // Hashing is bounded to the offline limit. Larger and unidentified downloads retain their normal path.
    let hash = item
        .course_id
        .and_then(|_| md_os::offline::hash(&transfer.staged).ok());
    let reused = hash.as_ref().and_then(|hash| {
        let files = store
            .shared_files(&md_store::FileIdentity {
                site,
                user: transfer.user_id,
                course: item.course_id?,
                filename: &transfer.filename,
                sha256: hash,
            })
            .ok()?;
        md_os::offline::reusable(root, &files, hash)
    });
    let path = reused
        .or_else(|| policy::publish(&transfer.staged, &transfer.folder, &transfer.filename).ok())?;
    store
        .download_finish(&transfer.id, &path.to_string_lossy(), "complete")
        .ok()?;
    if let Some(hash) = hash {
        let _ = store.fingerprint_download(&transfer.id, transfer.user_id, &hash);
    }
    Some(path)
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
            let result = (|| -> Result<Feedback, String> {
                let state = app.state::<AppState>();
                let mut transfers = state
                    .downloads
                    .lock()
                    .map_err(|_| "Download unavailable.")?;
                // One mark per organisation, set just before WebKit starts this transfer.
                // Taken first, so a refused request cannot leave it for a later download.
                let open_after = state
                    .open_next
                    .lock()
                    .ok()
                    .and_then(|mut pending| pending.remove(site_id))
                    .is_some_and(|at| at.elapsed() < crate::OPEN_PENDING);
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
                let course = md_moodle::file_context(&base, &url).and_then(|context| {
                    state
                        .store
                        .lock()
                        .ok()?
                        .course_for_context(site_id, context)
                        .ok()
                        .flatten()
                });
                let folder = folder(app, &site, course.as_ref())?;
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
                    course_id: course.as_ref().map(|c| c.id),
                    course_name: course.map(|c| c.name),
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
                        filename: filename.clone(),
                        ambiguous: false,
                        finished: false,
                        user_id: site.user_id,
                        open_after,
                    },
                );
                Ok(Feedback {
                    site_id: site_id.into(),
                    user_id: site.user_id,
                    filename,
                    state: "downloading",
                })
            })();
            match result {
                Ok(feedback) => {
                    let _ = app.emit_to("shell", "downloads://feedback", feedback);
                }
                Err(error) => {
                    let _ = app.emit_to("shell", "shell://error", error);
                    return false;
                }
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
                let handle = app.clone();
                let site = site_id.to_owned();
                tauri::async_runtime::spawn_blocking(move || {
                    finished(&handle, &site, transfer, success)
                });
            }
            true
        }
        _ => false,
    }
}

// Hashing completed payloads can be substantial. Keep it off WebKit's callback thread.
fn finished(app: &AppHandle, site_id: &str, transfer: Transfer, success: bool) {
    let state = app.state::<AppState>();
    // Account removal deletes its rows; never recreate them from a late callback.
    let (saved, completed) = {
        let store = state.store.lock().expect("store");
        let saved = store
            .sites()
            .unwrap_or_default()
            .iter()
            .any(|s| s.id == site_id && s.user_id == transfer.user_id);
        let completed = if success && saved {
            app.path()
                .home_dir()
                .ok()
                .and_then(|home| complete(&store, &home.join("Moodle"), site_id, &transfer))
        } else {
            None
        };
        if completed.is_none() {
            let _ =
                store.download_finish(&transfer.id, &transfer.staged.to_string_lossy(), "failed");
        }
        (saved, completed)
    };
    let _ = std::fs::remove_file(&transfer.staged);
    if let Some(parent) = transfer.staged.parent() {
        let _ = std::fs::remove_dir(parent);
    }
    // The preference covers PDFs and images only. The path is revalidated and
    // the risky-type gate still applies, so nothing else ever opens by itself.
    if completed.is_some() && transfer.open_after && saved {
        if let Ok(path) = self::path(app, &transfer.id, false, true) {
            if policy::opens_after_save(&path) {
                use tauri_plugin_opener::OpenerExt;
                let _ = app.opener().open_path(path.to_string_lossy(), None::<&str>);
            }
        }
    }
    if saved {
        let filename = completed
            .as_deref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or(&transfer.filename)
            .to_owned();
        let _ = app.emit_to(
            "shell",
            "downloads://feedback",
            Feedback {
                site_id: site_id.into(),
                user_id: transfer.user_id,
                filename,
                state: if completed.is_some() {
                    "complete"
                } else {
                    "failed"
                },
            },
        );
        let fr = crate::menus::french(app);
        let body = match (completed.is_some(), fr) {
            (true, false) => "Download complete. Open Downloads in MoodBuro to view it.",
            (true, true) => {
                "Téléchargement terminé. Ouvrez Téléchargements dans MoodBuro pour le consulter."
            }
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    #[serde(flatten)]
    item: md_store::Download,
    needs_confirmation: bool,
    can_preview: bool,
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
    course_id: Option<i64>,
) -> Result<Page, String> {
    if !matches!(state, None | Some("downloading" | "complete" | "failed")) || query.len() > 256 {
        return Err("Unknown download filter.".into());
    }
    let filter = md_store::DownloadFilter {
        query,
        site_id,
        state,
        course_id,
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
                can_preview: cfg!(target_os = "macos")
                    && crate::documents::supported(&item.filename),
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
    fn completed_download_reuses_verified_offline_bytes_but_preserves_local_edits() {
        let root =
            std::env::temp_dir().join(format!("moodburo-shared-download-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let existing = root.join("notes.pdf");
        let staged = root.join("payload");
        std::fs::write(&existing, b"same bytes").unwrap();
        std::fs::write(&staged, b"same bytes").unwrap();
        let store = md_store::Store::open(":memory:").unwrap();
        let mut site = md_store::Site {
            id: "s".into(),
            wwwroot: "https://example.org/".into(),
            name: "Example".into(),
            user_id: 7,
            full_name: "Learner".into(),
        };
        store.insert(&site).unwrap();
        let hash = md_os::offline::hash(&existing).unwrap();
        store
            .save_offline_copy(
                &md_store::OfflineCopy {
                    id: "c".into(),
                    site_id: "s".into(),
                    course_id: 2,
                    resource_key: "a".repeat(64),
                    module_id: 42,
                    filename: "notes.pdf".into(),
                    section_name: "Week 1".into(),
                    local_path: existing.to_string_lossy().into_owned(),
                    sha256: hash.clone(),
                    bytes: 10,
                    saved_at: 1,
                    refreshed_at: 1,
                },
                7,
            )
            .unwrap();
        let mut transfer = Transfer {
            id: "one".into(),
            staged: staged.clone(),
            folder: root.clone(),
            filename: "notes.pdf".into(),
            ambiguous: false,
            finished: false,
            user_id: 7,
            open_after: false,
        };
        let mut row = md_store::Download {
            id: "one".into(),
            site_id: "s".into(),
            filename: "notes.pdf".into(),
            local_path: staged.to_string_lossy().into_owned(),
            state: "downloading".into(),
            started_at: 2,
            course_id: Some(2),
            course_name: None,
        };
        store.download_insert(&row).unwrap();
        assert_eq!(complete(&store, &root, "s", &transfer).unwrap(), existing);
        assert!(store
            .file_referenced(&existing.to_string_lossy(), "c")
            .unwrap());
        assert!(!root.join("notes (2).pdf").exists());
        std::fs::write(&existing, b"my edits").unwrap();
        row.id = "two".into();
        transfer.id = "two".into();
        store.download_insert(&row).unwrap();
        let published = complete(&store, &root, "s", &transfer).unwrap();
        assert_ne!(published, existing);
        assert_eq!(std::fs::read(&existing).unwrap(), b"my edits");
        assert_eq!(std::fs::read(published).unwrap(), b"same bytes");
        site.user_id = 8;
        store.update(&site).unwrap();
        assert!(complete(&store, &root, "s", &transfer).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
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
                open_after: false,
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
