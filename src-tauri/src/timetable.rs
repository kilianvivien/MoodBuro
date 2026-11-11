//! Timetable PDFs read by the native InspCal engine, for the INSP Moodle only.
//! See docs/inspcal-integration-plan.md and docs/adr/0003-native-inspcal.md.
//!
//! Every entry point (link menu, course file, saved copy, download) becomes a
//! tagged [`Source`], resolved here in Rust: the shell never names a path or
//! URL. PDF bytes stay in this process and are never written anew.
use crate::AppState;
use md_inspcal::MAX_PDF_BYTES;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Stable error codes; the shell translates them (`t.timetable.errors`).
pub mod code {
    pub const UNSUPPORTED_SITE: &str = "UNSUPPORTED_SITE";
    pub const ACCOUNT_CHANGED: &str = "ACCOUNT_CHANGED";
    pub const SOURCE_UNAVAILABLE: &str = "SOURCE_UNAVAILABLE";
    pub const LINK_UNRESOLVED: &str = "LINK_UNRESOLVED";
    pub const INTENT_EXPIRED: &str = "INTENT_EXPIRED";
    pub const TOO_LARGE: &str = "TOO_LARGE";
    pub const NOT_PDF: &str = "NOT_PDF";
    pub const NETWORK: &str = "NETWORK";
    pub const ACCESS_DENIED: &str = "ACCESS_DENIED";
    pub const RECONNECT: &str = "RECONNECT";
    pub const STORAGE: &str = "STORAGE";
}

const INTENT_TTL: Duration = Duration::from_secs(60);
const MAX_INTENTS: usize = 8;
/// Bound on courses searched when a clicked link's course is not yet known.
const MAX_COURSE_SCAN: usize = 40;

/// Where a timetable PDF comes from. Every variant is resolved and
/// re-validated in Rust against the current account.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Source {
    CourseResource {
        course_id: i64,
        resource_key: String,
    },
    OfflineCopy {
        course_id: i64,
        copy_id: String,
    },
    Download {
        download_id: String,
    },
    LinkIntent {
        intent_id: String,
    },
}

/// A link the user chose “Open as timetable” on, captured natively.
#[derive(Clone, Debug)]
pub struct Intent {
    pub site_id: String,
    pub user_id: i64,
    pub view_label: String,
    /// The page the link was on; navigating away invalidates the intent.
    pub page: Option<String>,
    pub href: url::Url,
    pub label: String,
    pub created: Instant,
}

#[derive(Default)]
pub struct State {
    intents: Mutex<HashMap<String, Intent>>,
}

/// The bytes of one timetable PDF and what identifies it, never a path or URL.
pub struct Fetched {
    pub bytes: Vec<u8>,
    /// Name passed to the engine, which reads week dates from it.
    pub filename: String,
    /// Opaque reference persisted with a saved week (no URL, no path).
    pub reference: String,
}

/// The saved site, if it is the institution's Moodle and still the same account.
pub fn supported_site(
    app: &AppHandle,
    site_id: &str,
    user_id: i64,
) -> Result<md_store::Site, String> {
    let site = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| code::STORAGE)?
        .sites()
        .map_err(|_| code::STORAGE)?
        .into_iter()
        .find(|site| site.id == site_id)
        .ok_or(code::ACCOUNT_CHANGED)?;
    if site.user_id != user_id {
        return Err(code::ACCOUNT_CHANGED.into());
    }
    if !md_inspcal::supports_site(&site.wwwroot) {
        return Err(code::UNSUPPORTED_SITE.into());
    }
    Ok(site)
}

/// Saved sites for which the timetable feature is offered.
pub fn enabled_sites(app: &AppHandle) -> Vec<String> {
    let state = app.state::<AppState>();
    let Ok(store) = state.store.lock() else {
        return Vec::new();
    };
    store
        .sites()
        .unwrap_or_default()
        .into_iter()
        .filter(|site| md_inspcal::supports_site(&site.wwwroot))
        .map(|site| site.id)
        .collect()
}

/// What a clicked link points at, parsed from an untrusted DOM `href`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkTarget {
    /// `mod/resource/view.php?id=…`
    Resource { module_id: i64 },
    /// `[webservice/]pluginfile.php/<context>/<component>/<area>/<item>/<path…>/<name>`
    File {
        context: i64,
        filepath: String,
        filename: String,
    },
}

/// Parse a link on this installation that may be a timetable PDF. Only the
/// parser decides whether it really is one.
pub fn link_target(base: &url::Url, href: &url::Url) -> Option<LinkTarget> {
    if href.scheme() != "https"
        || href.origin() != base.origin()
        || !href.username().is_empty()
        || href.password().is_some()
    {
        return None;
    }
    let path = href.path().strip_prefix(base.path())?;
    if path == "mod/resource/view.php" {
        let mut ids = href.query_pairs().filter(|(key, _)| key == "id");
        let id = ids.next()?.1.parse::<i64>().ok().filter(|id| *id > 0)?;
        return ids
            .next()
            .is_none()
            .then_some(LinkTarget::Resource { module_id: id });
    }
    let tail = path
        .strip_prefix("pluginfile.php/")
        .or_else(|| path.strip_prefix("webservice/pluginfile.php/"))?;
    let segments: Vec<String> = tail
        .split('/')
        .map(|segment| {
            percent_encoding::percent_decode_str(segment)
                .decode_utf8()
                .map(|s| s.into_owned())
        })
        .collect::<Result<_, _>>()
        .ok()?;
    // context / component / filearea / itemid / [path…] / filename
    if segments.len() < 5
        || segments
            .iter()
            .any(|s| s.is_empty() || s == "." || s == ".." || s.len() > 1024)
    {
        return None;
    }
    let context = segments[0].parse::<i64>().ok().filter(|id| *id > 0)?;
    if !matches!(segments[1].as_str(), "mod_resource" | "mod_folder") || segments[2] != "content" {
        return None;
    }
    let filename = segments.last()?.clone();
    if !filename.to_ascii_lowercase().ends_with(".pdf") {
        return None;
    }
    let inner = &segments[4..segments.len() - 1];
    let filepath = if inner.is_empty() {
        "/".to_owned()
    } else {
        format!("/{}/", inner.join("/"))
    };
    Some(LinkTarget::File {
        context,
        filepath,
        filename,
    })
}

/// A shape of the link with identifiers and names removed, for development
/// logs only: never the URL, a file name or a query value.
pub fn link_shape(base: &url::Url, href: &url::Url) -> String {
    let path = href.path().strip_prefix(base.path()).unwrap_or("<other>");
    let shape: Vec<String> = path
        .split('/')
        .map(|segment| {
            if segment.chars().all(|c| c.is_ascii_digit()) && !segment.is_empty() {
                "{n}".into()
            } else if segment.contains('.') && !segment.ends_with(".php") {
                format!("{{name}}.{}", segment.rsplit('.').next().unwrap_or(""))
            } else if segment.len() > 40 {
                "{long}".into()
            } else {
                segment.into()
            }
        })
        .collect();
    let keys: Vec<String> = href
        .query_pairs()
        .map(|(key, _)| key.into_owned())
        .collect();
    format!(
        "{} keys={keys:?} target={:?}",
        shape.join("/"),
        link_target(base, href).map(|t| match t {
            LinkTarget::Resource { .. } => "resource",
            LinkTarget::File { .. } => "file",
        })
    )
}

impl State {
    /// Record a chosen link; returns its one-use identifier.
    pub fn capture(&self, intent: Intent) -> Option<String> {
        let mut intents = self.intents.lock().ok()?;
        intents.retain(|_, known| known.created.elapsed() < INTENT_TTL);
        if intents.len() >= MAX_INTENTS {
            return None;
        }
        let id = uuid::Uuid::new_v4().to_string();
        intents.insert(id.clone(), intent);
        Some(id)
    }
    fn take(&self, id: &str) -> Option<Intent> {
        let intent = self.intents.lock().ok()?.remove(id)?;
        (intent.created.elapsed() < INTENT_TTL).then_some(intent)
    }
    /// Account removal or replacement: no captured link survives it.
    pub fn forget_site(&self, site_id: &str) {
        if let Ok(mut intents) = self.intents.lock() {
            intents.retain(|_, intent| intent.site_id != site_id);
        }
    }
}

/// Called by the native menu when the user picks “Open as timetable”.
pub fn link_chosen(app: &AppHandle, intent: Intent) {
    let site_id = intent.site_id.clone();
    let Some(id) = app.state::<State>().capture(intent) else {
        return;
    };
    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Chosen {
        site_id: String,
        intent_id: String,
    }
    let _ = app.emit_to(
        "shell",
        "timetable://link",
        Chosen {
            site_id,
            intent_id: id,
        },
    );
}

fn read_local(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|_| code::SOURCE_UNAVAILABLE)?;
    let mut bytes = Vec::new();
    file.take(MAX_PDF_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| code::SOURCE_UNAVAILABLE)?;
    check_pdf(bytes)
}

fn check_pdf(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    if bytes.len() > MAX_PDF_BYTES {
        return Err(code::TOO_LARGE.into());
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err(code::NOT_PDF.into());
    }
    Ok(bytes)
}

fn fetch_error(error: md_moodle::offline::FetchError) -> String {
    use md_moodle::offline::FetchError;
    match error {
        FetchError::TooLarge => code::TOO_LARGE,
        FetchError::NotPdf => code::NOT_PDF,
        FetchError::Denied => code::ACCESS_DENIED,
        FetchError::Source => code::SOURCE_UNAVAILABLE,
        FetchError::Network => code::NETWORK,
    }
    .into()
}

/// Bytes of a catalog resource: the saved copy when one is on disk (no
/// network), otherwise an in-memory authenticated fetch.
async fn resource_bytes(
    app: &AppHandle,
    site: &md_store::Site,
    course_id: i64,
    resource: &crate::offline::Resource,
) -> Result<Fetched, String> {
    let filename = resource.original_name().to_owned();
    let reference = format!("course:{course_id}:{}", resource.key());
    if let Some(path) =
        crate::offline::saved_copy_path(app, &site.id, course_id, site.user_id, resource.key())?
    {
        if let Ok(bytes) = read_local(&path) {
            return Ok(Fetched {
                bytes,
                filename,
                reference,
            });
        }
    }
    if resource
        .bytes()
        .is_some_and(|bytes| bytes > MAX_PDF_BYTES as u64)
    {
        return Err(code::TOO_LARGE.into());
    }
    let (saved, client, token) =
        crate::library::session(app, &site.id).map_err(|_| code::RECONNECT)?;
    if saved.user_id != site.user_id {
        return Err(code::ACCOUNT_CHANGED.into());
    }
    let bytes = client
        .fetch_course_pdf(&token, resource.source(), MAX_PDF_BYTES)
        .await
        .map_err(fetch_error)?;
    Ok(Fetched {
        bytes,
        filename,
        reference,
    })
}

async fn catalog(
    app: &AppHandle,
    site: &md_store::Site,
    course_id: i64,
) -> Result<crate::offline::Catalog, String> {
    crate::offline::resources(app, &site.id, course_id, site.user_id)
        .await
        .map_err(|error| {
            if error.contains("Reconnect") {
                code::RECONNECT.into()
            } else if error.contains("another account") {
                code::ACCOUNT_CHANGED.into()
            } else {
                code::NETWORK.into()
            }
        })
}

fn matches(target: &LinkTarget, resource: &crate::offline::Resource) -> bool {
    match target {
        LinkTarget::Resource { module_id } => {
            resource.module_id() == *module_id
                && resource
                    .original_name()
                    .to_ascii_lowercase()
                    .ends_with(".pdf")
        }
        LinkTarget::File {
            context,
            filepath,
            filename,
        } => resource.matches_file(*context, filepath, filename),
    }
}

/// Courses to search for a link: the one the store already associates with
/// it first, then the account's other courses, bounded.
fn candidate_courses(
    app: &AppHandle,
    site: &md_store::Site,
    target: &LinkTarget,
) -> Result<Vec<i64>, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().map_err(|_| code::STORAGE)?;
    let known = match target {
        LinkTarget::File { context, .. } => store
            .course_for_context(&site.id, *context)
            .ok()
            .flatten()
            .map(|course| course.id),
        LinkTarget::Resource { module_id } => store
            .activities()
            .ok()
            .and_then(|items| {
                items
                    .into_iter()
                    .find(|a| a.site_id == site.id && a.id == *module_id)
            })
            .map(|activity| activity.course_id),
    };
    let mut courses: Vec<i64> = known.into_iter().collect();
    for course in store.courses().map_err(|_| code::STORAGE)? {
        if course.site_id == site.id && !courses.contains(&course.id) {
            courses.push(course.id);
        }
    }
    courses.truncate(MAX_COURSE_SCAN);
    Ok(courses)
}

async fn link_bytes(
    app: &AppHandle,
    site: &md_store::Site,
    intent: Intent,
) -> Result<Fetched, String> {
    if intent.site_id != site.id || intent.user_id != site.user_id {
        return Err(code::ACCOUNT_CHANGED.into());
    }
    // The page the link was on must still be showing in the same view.
    let current = app
        .get_webview(&intent.view_label)
        .and_then(|view| view.url().ok());
    let captured = intent
        .page
        .as_deref()
        .and_then(|page| url::Url::parse(page).ok());
    if !same_page(current.as_ref(), captured.as_ref()) {
        return Err(code::INTENT_EXPIRED.into());
    }
    let base = md_moodle::normalize_site(&site.wwwroot).map_err(|_| code::UNSUPPORTED_SITE)?;
    let target = link_target(&base, &intent.href).ok_or(code::LINK_UNRESOLVED)?;
    for course_id in candidate_courses(app, site, &target)? {
        let catalog = match catalog(app, site, course_id).await {
            Ok(catalog) => catalog,
            Err(error) if error == code::RECONNECT || error == code::ACCOUNT_CHANGED => {
                return Err(error)
            }
            Err(_) => continue,
        };
        let found: Vec<_> = catalog
            .items()
            .iter()
            .filter(|resource| matches(&target, resource))
            .collect();
        match found.as_slice() {
            [] => continue,
            [resource] => return resource_bytes(app, site, course_id, resource).await,
            _ => return Err(code::LINK_UNRESOLVED.into()),
        }
    }
    Err(code::LINK_UNRESOLVED.into())
}

/// Resolve a source to PDF bytes for the current account of a supported site.
pub async fn resolve(
    app: &AppHandle,
    site_id: &str,
    user_id: i64,
    source: Source,
) -> Result<Fetched, String> {
    let site = supported_site(app, site_id, user_id)?;
    let fetched = match source {
        Source::CourseResource {
            course_id,
            resource_key,
        } => {
            if resource_key.len() != 64 || !resource_key.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(code::SOURCE_UNAVAILABLE.into());
            }
            // A saved copy answers without any network call.
            if let Some(path) =
                crate::offline::saved_copy_path(app, &site.id, course_id, user_id, &resource_key)?
            {
                if let Ok(bytes) = read_local(&path) {
                    let filename = crate::offline::saved_copy_name(
                        app,
                        &site.id,
                        course_id,
                        user_id,
                        &resource_key,
                    )?
                    .unwrap_or_else(|| file_name(&path));
                    return finish(
                        app,
                        site_id,
                        user_id,
                        Fetched {
                            bytes,
                            filename,
                            reference: format!("course:{course_id}:{resource_key}"),
                        },
                    );
                }
            }
            let catalog = catalog(app, &site, course_id).await?;
            let resource = catalog
                .items()
                .iter()
                .find(|r| r.key() == resource_key)
                .ok_or(code::SOURCE_UNAVAILABLE)?;
            resource_bytes(app, &site, course_id, resource).await?
        }
        Source::OfflineCopy { course_id, copy_id } => {
            let path = crate::offline::copy_path(app, &site.id, course_id, user_id, &copy_id)
                .map_err(|_| code::SOURCE_UNAVAILABLE)?;
            let filename = file_name(&path);
            Fetched {
                bytes: read_local(&path)?,
                reference: format!("copy:{course_id}:{copy_id}"),
                filename,
            }
        }
        Source::Download { download_id } => {
            let item = app
                .state::<AppState>()
                .store
                .lock()
                .map_err(|_| code::STORAGE)?
                .download(&download_id)
                .map_err(|_| code::STORAGE)?
                .filter(|item| item.site_id == site.id && item.state == "complete")
                .ok_or(code::SOURCE_UNAVAILABLE)?;
            let path = crate::downloads::path(app, &item.id, true, false)
                .map_err(|_| code::SOURCE_UNAVAILABLE)?;
            Fetched {
                bytes: read_local(&path)?,
                filename: item.filename,
                reference: format!("download:{download_id}"),
            }
        }
        Source::LinkIntent { intent_id } => {
            let intent = app
                .state::<State>()
                .take(&intent_id)
                .ok_or(code::INTENT_EXPIRED)?;
            let label = intent.label.clone();
            let mut fetched = link_bytes(app, &site, intent).await?;
            fetched.filename = md_inspcal::filename_with_week(&fetched.filename, &label);
            fetched
        }
    };
    finish(app, site_id, user_id, fetched)
}

/// The account may have changed while fetching.
fn finish(
    app: &AppHandle,
    site_id: &str,
    user_id: i64,
    fetched: Fetched,
) -> Result<Fetched, String> {
    supported_site(app, site_id, user_id)?;
    Ok(fetched)
}

/// Whether the view still shows the page a link was chosen on (fragment
/// changes do not count as leaving it).
fn same_page(current: Option<&url::Url>, captured: Option<&url::Url>) -> bool {
    match (current, captured) {
        (Some(current), Some(captured)) => {
            let strip = |url: &url::Url| {
                let mut url = url.clone();
                url.set_fragment(None);
                url
            };
            strip(current) == strip(captured)
        }
        _ => false,
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> url::Url {
        url::Url::parse("https://moodle.example.org/campus/").unwrap()
    }
    fn parse(href: &str) -> Option<LinkTarget> {
        link_target(&base(), &url::Url::parse(href).unwrap())
    }

    #[test]
    fn timetable_links_are_parsed_from_this_installation_only() {
        assert_eq!(
            parse("https://moodle.example.org/campus/mod/resource/view.php?id=42"),
            Some(LinkTarget::Resource { module_id: 42 })
        );
        assert_eq!(
            parse("https://moodle.example.org/campus/pluginfile.php/77/mod_folder/content/3/Semaine%2045/Edt%20du%2009.11.2026%20au%2013.11.2026.pdf?forcedownload=1"),
            Some(LinkTarget::File {
                context: 77,
                filepath: "/Semaine 45/".into(),
                filename: "Edt du 09.11.2026 au 13.11.2026.pdf".into()
            })
        );
        assert_eq!(
            parse("https://moodle.example.org/campus/webservice/pluginfile.php/77/mod_resource/content/1/Edt.pdf"),
            Some(LinkTarget::File { context: 77, filepath: "/".into(), filename: "Edt.pdf".into() })
        );
        for refused in [
            "http://moodle.example.org/campus/mod/resource/view.php?id=42",
            "https://evil.example/campus/mod/resource/view.php?id=42",
            "https://user@moodle.example.org/campus/mod/resource/view.php?id=42",
            "https://moodle.example.org/other/mod/resource/view.php?id=42",
            "https://moodle.example.org/campus/mod/resource/view.php?id=42&id=43",
            "https://moodle.example.org/campus/mod/resource/view.php?id=-1",
            "https://moodle.example.org/campus/pluginfile.php/77/mod_folder/content/3/notes.docx",
            "https://moodle.example.org/campus/pluginfile.php/77/user/private/0/a.pdf",
            "https://moodle.example.org/campus/pluginfile.php/77/mod_folder/content/3/%2E%2E/a.pdf",
            "https://moodle.example.org/campus/pluginfile.php/x/mod_folder/content/3/a.pdf",
            "https://moodle.example.org/campus/course/view.php?id=4",
        ] {
            assert_eq!(parse(refused), None, "{refused}");
        }
    }

    #[test]
    fn development_link_shapes_hide_identifiers_and_names() {
        let shape = link_shape(
            &base(),
            &url::Url::parse("https://moodle.example.org/campus/pluginfile.php/77/mod_folder/content/3/Edt%20secret.pdf?token=abc").unwrap(),
        );
        assert!(
            shape.starts_with("pluginfile.php/{n}/mod_folder/content/{n}/{name}.pdf"),
            "{shape}"
        );
        assert!(!shape.contains("77") && !shape.contains("secret") && !shape.contains("abc"));
    }

    #[test]
    fn intents_are_one_use_bounded_and_expire() {
        let state = State::default();
        let intent = Intent {
            site_id: "site".into(),
            user_id: 1,
            view_label: "site-site".into(),
            page: None,
            href: base(),
            label: String::new(),
            created: Instant::now(),
        };
        let id = state.capture(intent.clone()).unwrap();
        assert!(state.take(&id).is_some());
        assert!(state.take(&id).is_none(), "one use");
        let old = state
            .capture(Intent {
                created: Instant::now() - INTENT_TTL,
                ..intent.clone()
            })
            .unwrap();
        assert!(state.take(&old).is_none(), "expired");
        for _ in 0..MAX_INTENTS {
            state.capture(intent.clone()).unwrap();
        }
        assert!(state.capture(intent.clone()).is_none(), "bounded");
        state.forget_site("site");
        assert!(state.capture(intent).is_some());
    }

    #[test]
    fn an_intent_needs_the_same_page_still_open() {
        let page =
            url::Url::parse("https://moodle.example.org/course/view.php?id=4#section-2").unwrap();
        let same = url::Url::parse("https://moodle.example.org/course/view.php?id=4").unwrap();
        let other = url::Url::parse("https://moodle.example.org/course/view.php?id=5").unwrap();
        assert!(same_page(Some(&same), Some(&page)));
        assert!(!same_page(Some(&other), Some(&page)));
        assert!(!same_page(None, Some(&page)));
        assert!(!same_page(Some(&same), None));
    }

    #[test]
    fn local_bytes_must_be_a_bounded_pdf() {
        assert_eq!(check_pdf(b"<html>".to_vec()).unwrap_err(), code::NOT_PDF);
        let mut big = b"%PDF-".to_vec();
        big.resize(MAX_PDF_BYTES + 1, b'x');
        assert_eq!(check_pdf(big).unwrap_err(), code::TOO_LARGE);
        let mut exact = b"%PDF-".to_vec();
        exact.resize(MAX_PDF_BYTES, b'x');
        assert!(check_pdf(exact).is_ok());
    }
}
