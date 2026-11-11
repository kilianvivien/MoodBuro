//! OS-granted files, account-bound exact-target approvals and a non-replaying write journal.
use crate::AppState;
use std::{collections::HashMap, path::PathBuf, sync::Mutex, time::Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
#[derive(Clone)]
struct Grant {
    path: PathBuf,
    name: String,
    bytes: u64,
    created: Instant,
}
struct Prepared {
    file: String,
    site: String,
    user: i64,
    course: i64,
    assignment: i64,
    module: i64,
    hash: String,
    created: Instant,
}
#[derive(Default)]
pub struct State {
    grants: Mutex<HashMap<String, Grant>>,
    approvals: Mutex<HashMap<String, Prepared>>,
    cancel: Mutex<Option<(String, tokio::sync::watch::Sender<bool>)>>,
}
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: String,
    pub name: String,
    pub bytes: u64,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    id: i64,
    module_id: i64,
    name: String,
    supported: bool,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    id: String,
    filename: String,
    bytes: u64,
    organisation: String,
    course: String,
    assignment: String,
}
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}
fn account(
    app: &AppHandle,
    site: &str,
    user: i64,
    course: i64,
) -> Result<md_store::Course, String> {
    let state = app.state::<AppState>();
    let store = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?;
    if !store
        .sites()
        .map_err(|_| "Could not read sites.")?
        .iter()
        .any(|s| s.id == site && s.user_id == user)
    {
        return Err("The upload account changed. Choose the assignment again.".into());
    }
    store
        .courses()
        .map_err(|_| "Could not read the course list.")?
        .into_iter()
        .find(|c| c.site_id == site && c.id == course)
        .ok_or("This course is no longer in your list. Refresh your courses.".into())
}
fn grant(app: &AppHandle, path: PathBuf) -> Result<File, String> {
    let meta = std::fs::symlink_metadata(&path).map_err(|_| "Could not read the selected file.")?;
    if !path.is_absolute()
        || !meta.is_file()
        || meta.file_type().is_symlink()
        || meta.len() == 0
        || meta.len() > md_moodle::assignments::MAX_UPLOAD_BYTES as u64
    {
        return Err("Choose one regular file up to 32 MB.".into());
    }
    let raw = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Could not read the selected file.")?;
    let name = md_os::downloads::safe_name(raw);
    let id = uuid::Uuid::new_v4().to_string();
    let state = app.state::<State>();
    let mut grants = state
        .grants
        .lock()
        .map_err(|_| "Upload preparation is unavailable.")?;
    grants.retain(|_, g| g.created.elapsed().as_secs() < 1800);
    if grants.len() >= 20 {
        return Err("Too many selected files. Reopen MoodBuro and try again.".into());
    }
    grants.insert(
        id.clone(),
        Grant {
            path,
            name: name.clone(),
            bytes: meta.len(),
            created: Instant::now(),
        },
    );
    Ok(File {
        id,
        name,
        bytes: meta.len(),
    })
}
pub fn drop_file(app: &AppHandle, paths: &[PathBuf]) -> Result<(), String> {
    if paths.len() != 1 {
        return Err("Choose one regular file up to 32 MB.".into());
    }
    let file = grant(app, paths[0].clone())?;
    crate::tray::show(app);
    crate::menus::show_local_view(app, "uploads");
    let _ = app.emit_to("shell", "uploads://selected", file);
    Ok(())
}
pub async fn pick(app: &AppHandle) -> Result<Option<File>, String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    let handle = app.clone();
    app.dialog().file().pick_file(move |file| {
        let result = file
            .map(|f| {
                f.into_path()
                    .map_err(|_| "Could not read the selected file.".to_string())
                    .and_then(|path| grant(&handle, path))
            })
            .transpose();
        let _ = send.send(result);
    });
    receive
        .await
        .map_err(|_| "Could not open the file picker.".to_string())?
}
pub fn files(app: &AppHandle) -> Result<Vec<File>, String> {
    let state = app.state::<State>();
    let grants = state
        .grants
        .lock()
        .map_err(|_| "Upload preparation is unavailable.")?;
    let mut entries: Vec<_> = grants
        .iter()
        .filter(|(_, g)| g.created.elapsed().as_secs() < 1800)
        .collect();
    entries.sort_by_key(|(_, g)| g.created);
    Ok(entries
        .into_iter()
        .map(|(id, g)| File {
            id: id.clone(),
            name: g.name.clone(),
            bytes: g.bytes,
        })
        .collect())
}
pub async fn targets(
    app: &AppHandle,
    site: &str,
    user: i64,
    course: i64,
) -> Result<Vec<Target>, String> {
    account(app, site, user, course)?;
    let (_, client, token) = crate::library::session(app, site)?;
    let assignments = client
        .assignments(&token, course)
        .await
        .map_err(|e| e.to_string())?;
    account(app, site, user, course)?;
    Ok(assignments
        .into_iter()
        .map(|a| Target {
            id: a.id,
            module_id: a.cmid,
            supported: a.supported(),
            name: a.name,
        })
        .collect())
}
async fn bytes(grant: Grant) -> Result<(Grant, Vec<u8>, String), String> {
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::Read;
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc_no_follow());
        }
        let mut file = options
            .open(&grant.path)
            .map_err(|_| "Could not read the selected file.")?;
        let meta = file
            .metadata()
            .map_err(|_| "Could not read the selected file.")?;
        if !meta.is_file() || meta.len() != grant.bytes {
            return Err("The selected file changed. Choose it again.".into());
        }
        let mut data = Vec::new();
        file.by_ref()
            .take(md_moodle::assignments::MAX_UPLOAD_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|_| "Could not read the selected file.")?;
        if data.len() as u64 != grant.bytes {
            return Err("The selected file changed. Choose it again.".into());
        }
        let hash = md_os::upload_hash(&data);
        Ok((grant, data, hash))
    })
    .await
    .map_err(|_| "Could not read the selected file.".to_string())?
}
#[cfg(unix)]
fn libc_no_follow() -> i32 {
    md_os::no_follow_flag()
}
fn selected(app: &AppHandle, id: &str) -> Result<Grant, String> {
    app.state::<State>()
        .grants
        .lock()
        .map_err(|_| "Upload preparation is unavailable.")?
        .get(id)
        .filter(|g| g.created.elapsed().as_secs() < 1800)
        .cloned()
        .ok_or("Choose the file again.".into())
}
pub async fn prepare(
    app: &AppHandle,
    site: &str,
    user: i64,
    course: i64,
    assignment: i64,
    file: &str,
) -> Result<Review, String> {
    let course_info = account(app, site, user, course)?;
    let (site_info, client, token) = crate::library::session(app, site)?;
    let a = client
        .assignments(&token, course)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|a| a.id == assignment)
        .ok_or("Choose the assignment again.")?;
    if !a.supported() {
        return Err("This assignment needs Moodle. Choose an individual file draft.".into());
    }
    let status = client
        .assignment_status(&token, assignment)
        .await
        .map_err(|e| e.to_string())?;
    if !md_moodle::assignments::empty_editable_status(&status, user) {
        return Err("This draft is locked or already contains work. Continue in Moodle.".into());
    }
    let (grant, _, hash) = bytes(selected(app, file)?).await?;
    if !a.allows_size(grant.bytes as usize) {
        return Err("This file exceeds the assignment’s limit.".into());
    }
    account(app, site, user, course)?;
    let id = uuid::Uuid::new_v4().to_string();
    let state = app.state::<State>();
    let mut approvals = state
        .approvals
        .lock()
        .map_err(|_| "Upload preparation is unavailable.")?;
    approvals.retain(|_, a| a.created.elapsed().as_secs() < 300);
    if approvals.len() >= 20 {
        return Err("Too many upload approvals. Reopen the upload view.".into());
    }
    approvals.insert(
        id.clone(),
        Prepared {
            file: file.into(),
            site: site.into(),
            user,
            course,
            assignment,
            module: a.cmid,
            hash,
            created: Instant::now(),
        },
    );
    Ok(Review {
        id,
        filename: grant.name,
        bytes: grant.bytes,
        organisation: site_info.name,
        course: course_info.name,
        assignment: a.name,
    })
}
pub fn stop(app: &AppHandle, id: &str) -> Result<(), String> {
    if let Some((active, cancel)) = app
        .state::<State>()
        .cancel
        .lock()
        .map_err(|_| "Upload preparation is unavailable.")?
        .as_ref()
    {
        if active == id {
            let _ = cancel.send(true);
        }
    }
    Ok(())
}
pub async fn confirm(app: &AppHandle, id: &str, confirmed: bool) -> Result<(), String> {
    if !confirmed {
        return Err("Confirm the upload destination first.".into());
    }
    let state = app.state::<State>();
    let approval = state
        .approvals
        .lock()
        .map_err(|_| "Upload preparation is unavailable.")?
        .remove(id)
        .filter(|a| a.created.elapsed().as_secs() < 300)
        .ok_or("The upload approval expired. Review the destination again.")?;
    let (cancel, mut cancelled) = tokio::sync::watch::channel(false);
    {
        let mut active = state
            .cancel
            .lock()
            .map_err(|_| "Upload preparation is unavailable.")?;
        if active.is_some() {
            return Err("Another upload is running.".into());
        }
        *active = Some((id.into(), cancel));
    }
    let result = async {
        let app_state = app.state::<AppState>();
        let _identity = tokio::select! {
            biased;
            _ = cancelled.changed() => return Err("Upload stopped. Check Moodle before retrying.".into()),
            lock = app_state.switching.lock() => lock,
        };
        account(app, &approval.site, approval.user, approval.course)?;
        let (grant, data, hash) = tokio::select! {
            biased;
            _ = cancelled.changed() => return Err("Upload stopped. Check Moodle before retrying.".into()),
            result = bytes(selected(app, &approval.file)?) => result?,
        };
        if hash != approval.hash {
            return Err("The selected file changed. Choose it again.".into());
        }
        run(app, id, &approval, &grant, data, &mut cancelled).await
    }.await;
    *state
        .cancel
        .lock()
        .map_err(|_| "Upload preparation is unavailable.")? = None;
    let _ = app.emit_to("shell", "uploads://changed", ());
    result
}
fn record(
    app: &AppHandle,
    id: &str,
    user: i64,
    status: &str,
    draft: Option<i64>,
) -> Result<(), String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .upload_state(id, user, status, draft)
        .map_err(|_| "Could not record upload progress.".into())
}
async fn run(
    app: &AppHandle,
    id: &str,
    a: &Prepared,
    file: &Grant,
    data: Vec<u8>,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    let (_, client, token) = crate::library::session(app, &a.site)?;
    if client
        .site_info(&token)
        .await
        .map_err(|e| e.to_string())?
        .userid
        != a.user
    {
        return Err("The upload account changed. Choose the assignment again.".into());
    }
    let target = client
        .assignments(&token, a.course)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|t| {
            t.id == a.assignment && t.cmid == a.module && t.supported() && t.allows_size(data.len())
        })
        .ok_or("Choose the assignment again.")?;
    let before = client
        .assignment_status(&token, target.id)
        .await
        .map_err(|e| e.to_string())?;
    if !md_moodle::assignments::empty_editable_status(&before, a.user) {
        return Err("This draft is locked or already contains work. Continue in Moodle.".into());
    }
    let state = app.state::<AppState>();
    state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .upload_start(&md_store::DraftUpload {
            id: id.into(),
            site_id: a.site.clone(),
            user_id: a.user,
            course_id: a.course,
            assignment_id: a.assignment,
            module_id: a.module,
            filename: file.name.clone(),
            bytes: file.bytes,
            sha256: a.hash.clone(),
            state: "started".into(),
            created_at: now(),
            draft_id: None,
        })
        .map_err(|_| "Check the previous upload in Moodle before starting another attempt.")?;
    let perform = async {
        if *cancel.borrow() {
            return Err("Upload stopped. Check Moodle before retrying.".into());
        }
        let draft = tokio::select! {
            biased;
            _ = cancel.changed() => return Err("Upload stopped. Check Moodle before retrying.".into()),
            result = client.upload_draft_file(&token, a.user, &file.name, data) => result.map_err(|_| "Upload outcome is uncertain. Check Moodle before retrying.")?,
        };
        record(app, id, a.user, "uploaded", Some(draft))?;
        let fresh = client.assignment_status(&token, target.id).await.map_err(|e| e.to_string())?;
        if !md_moodle::assignments::empty_editable_status(&fresh, a.user) {
            return Err("The Moodle draft changed. No file was attached; continue in Moodle.".into());
        }
        if *cancel.borrow() {
            return Err("Upload stopped. Check Moodle before retrying.".into());
        }
        record(app, id, a.user, "saving", Some(draft))?;
        tokio::select! {
            biased;
            _ = cancel.changed() => return Err("Upload stopped. Check Moodle before retrying.".into()),
            result = client.save_file_draft(&token, target.id, draft) => result.map_err(|_| "Upload outcome is uncertain. Check Moodle before retrying.")?,
        };
        let saved = client.assignment_status(&token, target.id).await.map_err(|_| "Upload outcome is uncertain. Check Moodle before retrying.")?;
        if !md_moodle::assignments::saved_draft_contains(&saved, a.user, &file.name, file.bytes as usize) {
            return Err("Upload outcome is uncertain. Check Moodle before retrying.".into());
        }
        Ok(())
    }.await;
    state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .upload_state(
            id,
            a.user,
            if perform.is_ok() {
                "saved"
            } else {
                "uncertain"
            },
            None,
        )
        .map_err(|_| "Could not record upload progress.")?;
    perform
}

pub async fn open(
    app: &AppHandle,
    site: &str,
    user: i64,
    course: i64,
    assignment: i64,
) -> Result<(), String> {
    account(app, site, user, course)?;
    let (saved, client, token) = crate::library::session(app, site)?;
    let target = client
        .assignments(&token, course)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|a| a.id == assignment && a.cmid > 0)
        .ok_or("Choose the assignment again.")?;
    account(app, site, user, course)?;
    let base = md_moodle::normalize_site(&saved.wwwroot).map_err(|e| e.to_string())?;
    let mut url = base
        .join("mod/assign/view.php")
        .map_err(|_| "Invalid site URL.")?;
    url.query_pairs_mut()
        .append_pair("id", &target.cmid.to_string());
    crate::commands::restore_site_at(app, site, Some(url)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn native_file_snapshots_detect_changed_bytes_and_refuse_symlinks() {
        let root =
            std::env::temp_dir().join(format!("moodburo-upload-grant-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("essay.pdf");
        std::fs::write(&path, b"first").unwrap();
        let grant = Grant {
            path: path.clone(),
            name: "essay.pdf".into(),
            bytes: 5,
            created: Instant::now(),
        };
        let (_, data, hash) = bytes(grant.clone()).await.unwrap();
        assert_eq!(data, b"first");
        std::fs::write(&path, b"other").unwrap();
        let (_, _, changed) = bytes(grant.clone()).await.unwrap();
        assert_ne!(hash, changed);
        std::fs::write(&path, b"longer").unwrap();
        assert!(bytes(grant.clone()).await.is_err());
        #[cfg(unix)]
        {
            std::fs::remove_file(&path).unwrap();
            std::os::unix::fs::symlink(root.join("target"), &path).unwrap();
            std::fs::write(root.join("target"), b"first").unwrap();
            assert!(bytes(grant).await.is_err());
        }
        let dto = serde_json::to_string(&File {
            id: "opaque".into(),
            name: "essay.pdf".into(),
            bytes: 5,
        })
        .unwrap();
        assert!(!dto.contains(&root.to_string_lossy().to_string()));
        assert!(!dto.contains("path"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
