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
}
#[derive(Serialize)]
pub struct Catalog {
    items: Vec<Resource>,
    skipped: usize,
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
    error: Option<String>,
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
                    app,
                    &client,
                    &token,
                    &saved_site,
                    &course,
                    resource,
                    &folder,
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
async fn save_one(
    app: &AppHandle,
    client: &md_moodle::Client,
    token: &str,
    site: &md_store::Site,
    course_info: &md_store::Course,
    resource: &Resource,
    folder: &Path,
) -> Result<bool, String> {
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
    let previous = store
        .offline_copies(&site.id, course_info.id)
        .map_err(|_| "Could not read offline files.")?;
    let managed = root(app)?;
    // Reuse only the latest indexed snapshot, still present and unchanged locally.
    if let Some(old) = previous.iter().find(|old| old.resource_key == resource.key) {
        let path = Path::new(&old.local_path);
        if disk::unchanged(&managed, path, &old.sha256, &hash) {
            return Ok(true);
        }
    }
    disk::managed_path(&managed, &destination)
        .map_err(|_| "Could not safely publish this offline file.")?;
    quarantine(&destination)?;
    let published = policy::publish(&destination, &section, &resource.filename)
        .map_err(|_| "Could not safely publish this offline file.")?;
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
        saved_at: now(),
    };
    if store.save_offline_copy(&copy, site.user_id).is_err() {
        // Leave the published file intact; do not erase a file the user may have opened.
        return Err("The file was saved, but its offline index could not be updated. Find it in the course folder.".into());
    }
    Ok(false)
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
