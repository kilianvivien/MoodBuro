//! User-selected one-shot file transfers. Source URLs live only in this stack frame.
use crate::{library, AppState};
use md_os::{downloads as policy, offline as disk};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    key: String,
    module_id: i64,
    section_id: i64,
    section_name: String,
    activity_name: String,
    filename: String,
    bytes: Option<u64>,
    #[serde(skip_serializing)]
    source: String,
    /// Moodle's own identity of the file, for matching a clicked link.
    #[serde(skip_serializing)]
    context_id: Option<i64>,
    #[serde(skip_serializing)]
    raw_filename: String,
    #[serde(skip_serializing)]
    raw_filepath: String,
}
impl Resource {
    pub(crate) fn key(&self) -> &str {
        &self.key
    }
    pub(crate) fn module_id(&self) -> i64 {
        self.module_id
    }
    /// The file name as Moodle stores it (the engine reads week dates from it).
    pub(crate) fn original_name(&self) -> &str {
        &self.raw_filename
    }
    /// Validated web-service URL, without credentials; never leaves Rust.
    pub(crate) fn source(&self) -> &str {
        &self.source
    }
    pub(crate) fn bytes(&self) -> Option<u64> {
        self.bytes
    }
    pub(crate) fn matches_file(&self, context: i64, filepath: &str, filename: &str) -> bool {
        self.context_id == Some(context)
            && self.raw_filepath == filepath
            && self.raw_filename == filename
    }
}
#[derive(Serialize)]
pub struct Catalog {
    items: Vec<Resource>,
    skipped: usize,
}
impl Catalog {
    pub(crate) fn items(&self) -> &[Resource] {
        &self.items
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    #[serde(flatten)]
    copy: md_store::OfflineCopy,
    available: bool,
    needs_confirmation: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    key: String,
    reused: bool,
    pub(crate) error: Option<String>,
}
/// What the assistant may know about a course file: names and size, and the stable
/// key that identifies it. The source URL never leaves this module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub key: String,
    pub module_id: i64,
    pub section: String,
    pub activity: String,
    pub filename: String,
    pub bytes: Option<u64>,
    /// A copy saved earlier is still in the course folder.
    pub saved: bool,
}
fn listed(items: &[Resource], saved: &std::collections::HashSet<String>) -> Vec<Listed> {
    items
        .iter()
        .map(|item| Listed {
            key: item.key.clone(),
            module_id: item.module_id,
            section: item.section_name.clone(),
            activity: item.activity_name.clone(),
            filename: item.filename.clone(),
            bytes: item.bytes,
            saved: saved.contains(&item.key),
        })
        .collect()
}
/// The course's files with their saved state, and how many entries were left out.
pub async fn listing(
    app: &AppHandle,
    site: &str,
    id: i64,
    user: i64,
) -> Result<(Vec<Listed>, usize), String> {
    let catalog = resources(app, site, id, user).await?;
    let saved = saved(app, site, id, user)?
        .into_iter()
        .filter(|copy| copy.available)
        .map(|copy| copy.copy.resource_key)
        .collect();
    Ok((listed(&catalog.items, &saved), catalog.skipped))
}
/// Latest saved copy of a resource that is still inside the managed folder.
fn latest_copy(
    app: &AppHandle,
    site: &str,
    id: i64,
    user: i64,
    select: impl Fn(&md_store::OfflineCopy) -> bool,
) -> Result<Option<(md_store::OfflineCopy, PathBuf)>, String> {
    course(app, site, id, user)?;
    let copies = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .offline_copies(site, id)
        .map_err(|_| "Could not read offline files.")?;
    let root = root(app)?;
    // Newest first: the first one still on disk wins.
    Ok(copies
        .into_iter()
        .filter(|copy| select(copy))
        .find_map(|copy| {
            let path = PathBuf::from(&copy.local_path);
            disk::managed_path(&root, &path).ok()?;
            Some((copy, path))
        }))
}
/// The newest saved copy of a resource still on disk, if any.
pub(crate) fn saved_copy_path(
    app: &AppHandle,
    site: &str,
    id: i64,
    user: i64,
    key: &str,
) -> Result<Option<PathBuf>, String> {
    Ok(latest_copy(app, site, id, user, |copy| copy.resource_key == key)?.map(|(_, path)| path))
}
/// Original Moodle file name of the newest saved copy of a resource. The
/// catalog's name is not stored; the published name keeps it (sanitized).
pub(crate) fn saved_copy_name(
    app: &AppHandle,
    site: &str,
    id: i64,
    user: i64,
    key: &str,
) -> Result<Option<String>, String> {
    Ok(
        latest_copy(app, site, id, user, |copy| copy.resource_key == key)?
            .map(|(copy, _)| copy.filename),
    )
}
/// Revalidated path of one saved copy, for the native document reader.
pub fn copy_path(
    app: &AppHandle,
    site: &str,
    id: i64,
    user: i64,
    copy_id: &str,
) -> Result<PathBuf, String> {
    latest_copy(app, site, id, user, |copy| copy.id == copy_id)?
        .map(|(_, path)| path)
        .ok_or_else(|| {
            "This offline file is unavailable. Save a fresh copy from the course.".into()
        })
}
/// Shows the saved copy of a resource: documents the native reader supports are
/// previewed, anything else is only revealed in Finder (never launched). `false`
/// when no copy is saved, so the caller can fall back to Moodle.
pub async fn show(
    app: &AppHandle,
    site: &str,
    id: i64,
    user: i64,
    key: &str,
) -> Result<bool, String> {
    if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("This resource is no longer available. Reload the course file list.".into());
    }
    let Some((copy, path)) = latest_copy(app, site, id, user, |copy| copy.resource_key == key)?
    else {
        return Ok(false);
    };
    if crate::documents::supported(&copy.filename) && !policy::needs_confirmation(&path) {
        crate::documents::preview_offline(app, site, id, user, &copy).await?;
    } else {
        app.opener()
            .reveal_item_in_dir(&path)
            .map_err(|_| "Could not open this offline file.")?;
    }
    Ok(true)
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    site_id: &'a str,
    course_id: i64,
    key: &'a str,
    received: u64,
    total: Option<u64>,
}
fn course(app: &AppHandle, site: &str, id: i64, user: i64) -> Result<md_store::Course, String> {
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
        return Err(
            "This offline selection belongs to another account. Reopen the course files.".into(),
        );
    }
    store
        .courses()
        .map_err(|_| "Could not read the course list.")?
        .into_iter()
        .find(|c| c.site_id == site && c.id == id)
        .ok_or("This course is no longer in your list. Refresh your courses.".into())
}
fn catalog(base: &url::Url, sections: Vec<md_moodle::CourseSection>) -> Result<Catalog, String> {
    let mut items = Vec::new();
    let mut skipped = 0;
    let mut keys = std::collections::HashSet::new();
    for section in sections {
        for module in section.modules {
            for file in module.contents {
                if !matches!(module.modname.as_str(), "resource" | "folder")
                    || file.kind != "file"
                    || module.id <= 0
                    || file.filename.is_empty()
                    || file.filename.len() > 1024
                    || file.filepath.as_ref().is_some_and(|v| v.len() > 2048)
                {
                    skipped += 1;
                    continue;
                }
                let Some(source) = file
                    .fileurl
                    .as_deref()
                    .and_then(|raw| md_moodle::offline::file_url(base, raw))
                else {
                    skipped += 1;
                    continue;
                };
                let key = disk::key(
                    module.id,
                    file.filepath.as_deref().unwrap_or("/"),
                    &file.filename,
                );
                if !keys.insert(key.clone()) {
                    return Err("The course returned ambiguous file metadata.".into());
                }
                items.push(Resource {
                    key,
                    module_id: module.id,
                    section_id: section.id,
                    section_name: md_moodle::plain_text(&section.name, 200),
                    activity_name: md_moodle::plain_text(&module.name, 200),
                    filename: policy::safe_name(&file.filename),
                    bytes: file.filesize,
                    source: source.to_string(),
                    context_id: module.contextid,
                    raw_filename: file.filename.clone(),
                    raw_filepath: file.filepath.clone().unwrap_or_else(|| "/".into()),
                });
                if items.len() > 1000 {
                    return Err(
                        "This course has too many files. Save files from Moodle instead.".into(),
                    );
                }
            }
        }
    }
    Ok(Catalog { items, skipped })
}
pub async fn resources(app: &AppHandle, site: &str, id: i64, user: i64) -> Result<Catalog, String> {
    course(app, site, id, user)?;
    let (saved, client, token) = library::session(app, site)?;
    if saved.user_id != user {
        return Err(
            "This offline selection belongs to another account. Reopen the course files.".into(),
        );
    }
    let sections = client
        .course_files(&token, id)
        .await
        .map_err(|e| e.to_string())?;
    course(app, site, id, user)?;
    catalog(
        &md_moodle::normalize_site(&saved.wwwroot).map_err(|e| e.to_string())?,
        sections,
    )
}
fn root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .home_dir()
        .map_err(|_| "Download folder unavailable.")?
        .join("Moodle"))
}
pub fn saved(app: &AppHandle, site: &str, id: i64, user: i64) -> Result<Vec<Saved>, String> {
    course(app, site, id, user)?;
    let items = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .offline_copies(site, id)
        .map_err(|_| "Could not read offline files.")?;
    let root = root(app)?;
    Ok(items
        .into_iter()
        .map(|copy| Saved {
            available: disk::managed_path(&root, Path::new(&copy.local_path)).is_ok(),
            needs_confirmation: policy::needs_confirmation(Path::new(&copy.filename)),
            copy,
        })
        .collect())
}
fn quarantine(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let value = format!("0083;{:x};MoodBuro;", now());
        let result = std::process::Command::new("/usr/bin/xattr")
            .args(["-w", "com.apple.quarantine", &value])
            .arg(path)
            .status();
        if !result.is_ok_and(|status| status.success()) {
            return Err("Could not safely publish this offline file.".into());
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = path;
    Ok(())
}
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_secs() as i64)
}
struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub async fn save(
    app: &AppHandle,
    site: &str,
    id: i64,
    user: i64,
    keys: Vec<String>,
) -> Result<Vec<Outcome>, String> {
    if keys.is_empty()
        || keys.len() > 50
        || keys
            .iter()
            .any(|k| k.len() != 64 || !k.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err("Select between 1 and 50 files to save offline.".into());
    }
    let state = app.state::<AppState>();
    let _job = state
        .offline_job
        .try_lock()
        .map_err(|_| "Another offline transfer is running. Wait for it to finish.")?;
    let course = course(app, site, id, user)?;
    // Re-fetch every source; never replay a persisted URL or stale remote file descriptor.
    let (saved_site, client, token) = library::session(app, site)?;
    if saved_site.user_id != user {
        return Err(
            "This offline selection belongs to another account. Reopen the course files.".into(),
        );
    }
    let sections = client
        .course_files(&token, id)
        .await
        .map_err(|e| e.to_string())?;
    let files = catalog(
        &md_moodle::normalize_site(&saved_site.wwwroot).map_err(|e| e.to_string())?,
        sections,
    )?;
    let folder = crate::downloads::folder(app, &saved_site, Some(&course))?;
    let mut outcomes = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for key in keys.into_iter().filter(|key| seen.insert(key.clone())) {
        let result = match files.items.iter().find(|f| f.key == key) {
            Some(resource) => {
                save_one(
                    &SaveContext {
                        app,
                        client: &client,
                        token: &token,
                        site: &saved_site,
                        course_info: &course,
                        folder: &folder,
                        job: None,
                    },
                    resource,
                )
                .await
            }
            None => {
                Err("This resource is no longer available. Reload the course file list.".into())
            }
        };
        outcomes.push(match result {
            Ok(reused) => Outcome {
                key,
                reused,
                error: None,
            },
            Err(error) => Outcome {
                key,
                reused: false,
                error: Some(error),
            },
        });
    }
    Ok(outcomes)
}
/// Scheduled work uses the same guarded downloader and durable per-resource checkpoints.
pub async fn sync_course(app: &AppHandle, job: &md_store::CourseSync) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _lock = state
        .offline_job
        .try_lock()
        .map_err(|_| "Another offline transfer is running. Wait for it to finish.")?;
    let course_info = course(app, &job.site_id, job.course_id, job.user_id)?;
    let (site, client, token) = library::session(app, &job.site_id)?;
    if site.user_id != job.user_id {
        return Err("Course sync was stopped.".into());
    }
    let files = resources(app, &job.site_id, job.course_id, job.user_id).await?;
    let keys = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .sync_pending(job)
        .map_err(|_| "Could not read offline files.")?;
    let folder = crate::downloads::folder(app, &site, Some(&course_info))?;
    for key in keys {
        let resource = files.items.iter().find(|f| f.key == key);
        let error = if let Some(resource) = resource {
            save_one(
                &SaveContext {
                    app,
                    client: &client,
                    token: &token,
                    site: &site,
                    course_info: &course_info,
                    folder: &folder,
                    job: Some(job),
                },
                resource,
            )
            .await
            .err()
        } else {
            None
        };
        state
            .store
            .lock()
            .map_err(|_| "Site storage unavailable.")?
            .sync_checkpoint(job, &key, error.as_deref(), resource.is_none())
            .map_err(|_| "Course sync was stopped.")?;
        crate::course_sync::changed(app);
    }
    Ok(())
}

struct SaveContext<'a> {
    app: &'a AppHandle,
    client: &'a md_moodle::Client,
    token: &'a str,
    site: &'a md_store::Site,
    course_info: &'a md_store::Course,
    folder: &'a Path,
    job: Option<&'a md_store::CourseSync>,
}
async fn save_one(context: &SaveContext<'_>, resource: &Resource) -> Result<bool, String> {
    let SaveContext {
        app,
        client,
        token,
        site,
        course_info,
        folder,
        job,
    } = *context;
    course(app, &site.id, course_info.id, site.user_id)?;
    if resource
        .bytes
        .is_some_and(|n| n > md_moodle::offline::MAX_FILE_BYTES)
    {
        return Err("This resource exceeds the 128 MB offline limit.".into());
    }
    let section = folder.join(format!(
        "{}-{}",
        policy::safe_name(&resource.section_name),
        resource.section_id
    ));
    policy::directory(&section).map_err(|_| "Could not save this resource.")?;
    // Conservative reservation permits an unknown or changed file up to the transfer cap.
    let required = md_moodle::offline::MAX_FILE_BYTES + 16 * 1024 * 1024;
    if disk::available(&section).map_err(|_| "Could not check available disk space.")? < required {
        return Err("Not enough disk space. Free at least 144 MB and try again.".into());
    }
    let staging_root = section.join(".moodburo-staging");
    policy::directory(&staging_root).map_err(|_| "Could not save this resource.")?;
    let private = staging_root.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir(&private).map_err(|_| "Could not save this resource.")?;
    let staging = Staging(private);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staging.0, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "Could not save this resource.")?;
    }
    let destination = staging.0.join("payload");
    let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
    let bytes = client
        .save_course_file(token, &resource.source, &destination, |received| {
            if last.elapsed() >= std::time::Duration::from_millis(200) {
                let _ = app.emit_to(
                    "shell",
                    "offline://progress",
                    Progress {
                        site_id: &site.id,
                        course_id: course_info.id,
                        key: &resource.key,
                        received,
                        total: resource.bytes,
                    },
                );
                last = std::time::Instant::now();
            }
        })
        .await?;
    disk::managed_path(&root(app)?, &destination)
        .map_err(|_| "Could not safely publish this offline file.")?;
    let hash = disk::hash(&destination).map_err(|_| "Could not save this resource.")?;
    let state = app.state::<AppState>();
    // No await after account revalidation: account replacement/removal cannot interleave publication.
    let store = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?;
    if !store
        .sites()
        .map_err(|_| "Could not read sites.")?
        .iter()
        .any(|s| s.id == site.id && s.user_id == site.user_id)
    {
        return Err(
            "This offline selection belongs to another account. Reopen the course files.".into(),
        );
    }
    if let Some(job) = job {
        if !store
            .sync_current(job)
            .map_err(|_| "Could not read offline files.")?
        {
            return Err("Course sync was stopped.".into());
        }
    }
    let previous = store
        .offline_copies(&site.id, course_info.id)
        .map_err(|_| "Could not read offline files.")?;
    let managed = root(app)?;
    // Reuse only the latest indexed snapshot, still present and unchanged locally.
    if let Some(old) = previous.iter().find(|old| old.resource_key == resource.key) {
        let path = Path::new(&old.local_path);
        if disk::unchanged(&managed, path, &old.sha256, &hash) {
            store
                .offline_refreshed(&old.id, &site.id, site.user_id, now())
                .map_err(|_| "Could not update offline freshness.")?;
            return Ok(true);
        }
    }
    disk::managed_path(&managed, &destination)
        .map_err(|_| "Could not safely publish this offline file.")?;
    quarantine(&destination)?;
    let candidates = store
        .shared_files(&md_store::FileIdentity {
            site: &site.id,
            user: site.user_id,
            course: course_info.id,
            filename: &resource.filename,
            sha256: &hash,
        })
        .map_err(|_| "Could not read offline files.")?;
    let reused = disk::reusable(&managed, &candidates, &hash);
    let shared = reused.is_some();
    let published = match reused {
        Some(path) => path,
        None => policy::publish(&destination, &section, &resource.filename)
            .map_err(|_| "Could not safely publish this offline file.")?,
    };
    let refreshed_at = now();
    let copy = md_store::OfflineCopy {
        id: uuid::Uuid::new_v4().to_string(),
        site_id: site.id.clone(),
        course_id: course_info.id,
        resource_key: resource.key.clone(),
        module_id: resource.module_id,
        filename: published
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        section_name: resource.section_name.clone(),
        local_path: published.to_string_lossy().into_owned(),
        sha256: hash,
        bytes,
        saved_at: refreshed_at,
        refreshed_at,
    };
    if store.save_offline_copy(&copy, site.user_id).is_err() {
        // Leave the published file intact; do not erase a file the user may have opened.
        return Err("The file was saved, but its offline index could not be updated. Find it in the course folder.".into());
    }
    Ok(shared)
}
pub fn open(
    app: &AppHandle,
    site: &str,
    id: i64,
    user: i64,
    copy_id: &str,
    reveal: bool,
    confirmed: bool,
) -> Result<(), String> {
    course(app, site, id, user)?;
    let state = app.state::<AppState>();
    let store = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?;
    let item = store
        .offline_copies(site, id)
        .map_err(|_| "Could not read offline files.")?
        .into_iter()
        .find(|c| c.id == copy_id)
        .ok_or("This offline file is unavailable. Save a fresh copy from the course.")?;
    let path = Path::new(&item.local_path);
    disk::managed_path(&root(app)?, path)
        .map_err(|_| "This offline file is unavailable. Save a fresh copy from the course.")?;
    if !reveal && policy::needs_confirmation(path) && !confirmed {
        return Err("Confirm opening this file first.".into());
    }
    if reveal {
        app.opener().reveal_item_in_dir(path)
    } else {
        app.opener().open_path(path.to_string_lossy(), None::<&str>)
    }
    .map_err(|_| "Could not open this offline file.".into())
}

/// Remove one version, never every version of a resource. The store lock binds
/// filesystem handling and metadata removal to the currently connected account.
pub fn remove(
    app: &AppHandle,
    site: &str,
    id: i64,
    user: i64,
    copy_id: &str,
) -> Result<&'static str, String> {
    course(app, site, id, user)?;
    let state = app.state::<AppState>();
    let _job = state
        .offline_job
        .try_lock()
        .map_err(|_| "Another offline transfer is running. Wait for it to finish.")?;
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
        return Err(
            "This offline selection belongs to another account. Reopen the course files.".into(),
        );
    }
    let copy = store
        .offline_copies(site, id)
        .map_err(|_| "Could not read offline files.")?
        .into_iter()
        .find(|c| c.id == copy_id)
        .ok_or("This offline file is unavailable. Save a fresh copy from the course.")?;
    let shared = store
        .file_referenced(&copy.local_path, copy_id)
        .map_err(|_| "Could not read offline files.")?;
    let result = if shared {
        disk::Removal::Kept
    } else {
        disk::remove_copy(
            &root(app)?,
            Path::new(&copy.local_path),
            &copy.sha256,
            &uuid::Uuid::new_v4().to_string(),
        )
        .map_err(|_| "Could not remove this offline copy. The file was kept; try again.")?
    };
    store.remove_offline_copy(copy_id, site, id, user)
        .map_err(|_| "Could not remove the offline entry. Any archived file remains in ~/Moodle/.trash; reload and try again.")?;
    Ok(match result {
        disk::Removal::Archived => "archived",
        disk::Removal::Kept => "kept",
        disk::Removal::Missing => "missing",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn section(id: i64, source: &str) -> md_moodle::CourseSection {
        md_moodle::CourseSection {
            id,
            name: "Week / 1".into(),
            modules: vec![md_moodle::CourseModule {
                id: 42,
                name: "<b>Lecture notes</b>".into(),
                modname: "resource".into(),
                contextid: Some(12),
                contents: vec![md_moodle::offline::Content {
                    kind: "file".into(),
                    filename: "../notes.pdf".into(),
                    filepath: Some("/".into()),
                    filesize: Some(45),
                    fileurl: Some(source.into()),
                }],
            }],
        }
    }
    #[test]
    fn descriptors_are_plain_bounded_and_stable_without_source_urls_in_keys() {
        let base = md_moodle::normalize_site("https://example.org/campus/").unwrap();
        let a = catalog(&base, vec![section(1, "https://example.org/campus/webservice/pluginfile.php/12/mod_resource/content/1/notes.pdf")]).unwrap();
        let b = catalog(&base, vec![section(2, "https://example.org/campus/webservice/pluginfile.php/12/mod_resource/content/2/notes.pdf")]).unwrap();
        assert_eq!(a.items[0].key, b.items[0].key); // revision and section changes retain the resource identity
        assert_eq!(a.items[0].key.len(), 64);
        assert_eq!(a.items[0].filename, "_notes.pdf");
        assert_eq!(a.items[0].activity_name, "Lecture notes");
        let unsafe_source = catalog(
            &base,
            vec![section(1, "https://evil.org/file.pdf?token=secret")],
        )
        .unwrap();
        assert!(unsafe_source.items.is_empty());
        // What the assistant receives carries names and the key, never the source URL.
        let shared = listed(&a.items, &std::iter::once(a.items[0].key.clone()).collect());
        assert!(shared[0].saved);
        assert_eq!(shared[0].filename, "_notes.pdf");
        let debug = format!("{shared:?}");
        assert!(!debug.contains("http") && !debug.contains("pluginfile"));
        assert!(!listed(&a.items, &Default::default())[0].saved);
        assert_eq!(unsafe_source.skipped, 1);
        let mut unsupported = section(
            1,
            "https://example.org/campus/webservice/pluginfile.php/12/a",
        );
        unsupported.modules[0].modname = "page".into();
        assert!(catalog(&base, vec![unsupported]).unwrap().items.is_empty());
        assert!(catalog(
            &base,
            vec![
                section(
                    1,
                    "https://example.org/campus/webservice/pluginfile.php/12/a"
                ),
                section(
                    1,
                    "https://example.org/campus/webservice/pluginfile.php/12/a"
                )
            ]
        )
        .is_err());
    }
}
