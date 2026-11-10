//! Local, account-bound shortcuts. Never persist authentication/action URLs.
use crate::AppState;
use md_store::Bookmark;
use tauri::{AppHandle, Manager};
use url::Url;
const ERROR: &str = "This page cannot be bookmarked. Open a Moodle course, activity or other standard content page.";
pub fn path(base: &Url, url: &Url) -> Result<String, String> {
    if url.origin() != base.origin() || !url.username().is_empty() || url.password().is_some() {
        return Err(ERROR.into());
    }
    let relative = url.path().strip_prefix(base.path()).ok_or(ERROR)?;
    let fixed = [
        "my/",
        "my/index.php",
        "my/courses.php",
        "course/view.php",
        "course/index.php",
        "calendar/view.php",
        "message/index.php",
        "message/output/popup/notifications.php",
        "user/profile.php",
        "grade/report/user/index.php",
    ];
    let module = relative
        .strip_prefix("mod/")
        .and_then(|s| s.split_once('/'))
        .is_some_and(|(module, page)| {
            !module.is_empty()
                && module.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                && ["view.php", "discuss.php"].contains(&page)
        });
    if !fixed.contains(&relative) && !module {
        return Err(ERROR.into());
    }
    let mut clean = base.join(relative).map_err(|_| ERROR)?;
    let pairs: Vec<_> = url
        .query_pairs()
        .filter(|(key, value)| {
            [
                "id",
                "course",
                "userid",
                "view",
                "time",
                "page",
                "section",
                "sectionid",
                "mode",
                "group",
                "d",
                "chapterid",
            ]
            .contains(&key.as_ref())
                && value.len() <= 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
        .collect();
    if !pairs.is_empty() {
        clean.query_pairs_mut().extend_pairs(pairs);
    }
    Ok(clean
        .as_str()
        .strip_prefix(base.as_str())
        .ok_or(ERROR)?
        .to_owned())
}
fn site(app: &AppHandle, id: &str) -> Result<md_store::Site, String> {
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
pub fn list(app: &AppHandle) -> Result<Vec<Bookmark>, String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .bookmarks()
        .map_err(|_| "Could not read bookmarks.".into())
}
pub async fn capture(app: &AppHandle) -> Result<Bookmark, String> {
    let id = app
        .state::<AppState>()
        .active
        .lock()
        .map_err(|_| "Site state unavailable.")?
        .clone()
        .ok_or(ERROR)?;
    let s = site(app, &id)?;
    let base = md_moodle::normalize_site(&s.wwwroot).map_err(|_| ERROR)?;
    let view = app.get_webview(&format!("site-{id}")).ok_or(ERROR)?;
    let url = view.url().map_err(|_| ERROR)?;
    let saved_path = path(&base, &url)?;
    #[cfg(target_os = "macos")]
    let title = {
        let (send, receive) = tokio::sync::oneshot::channel();
        view.with_webview(move |native| unsafe {
            let webview = &*(native.inner() as *const objc2_web_kit::WKWebView);
            let _ = send.send(webview.title().map(|title| title.to_string()));
        })
        .map_err(|_| ERROR)?;
        receive.await.ok().flatten()
    };
    #[cfg(not(target_os = "macos"))]
    let title: Option<String> = None;
    let name = title
        .map(|title| md_moodle::plain_text(&title, 100))
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(s.name);
    Ok(Bookmark {
        id: uuid::Uuid::new_v4().to_string(),
        site_id: id,
        user_id: s.user_id,
        name,
        path: saved_path,
        icon: "bookmark".into(),
    })
}
pub fn save(app: &AppHandle, mut b: Bookmark) -> Result<(), String> {
    if uuid::Uuid::parse_str(&b.id).is_err()
        || ![
            "bookmark",
            "star",
            "book",
            "calendar",
            "file",
            "folder",
            "message",
            "graduation",
        ]
        .contains(&b.icon.as_str())
    {
        return Err("Invalid bookmark.".into());
    }
    let s = site(app, &b.site_id)?;
    if s.user_id != b.user_id {
        return Err("This account has changed. Open the page again.".into());
    }
    let base = md_moodle::normalize_site(&s.wwwroot).map_err(|_| ERROR)?;
    let target = base.join(&b.path).map_err(|_| ERROR)?;
    if path(&base, &target)? != b.path {
        return Err(ERROR.into());
    }
    b.name = md_moodle::plain_text(&b.name, 100);
    if b.name.trim().is_empty() {
        return Err("Give the bookmark a name.".into());
    }
    let state = app.state::<AppState>();
    let store = state
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?;
    if store
        .bookmarks()
        .map_err(|_| "Could not read bookmarks.")?
        .len()
        >= 200
        && !store
            .bookmarks()
            .map_err(|_| "Could not read bookmarks.")?
            .iter()
            .any(|x| x.id == b.id)
    {
        return Err("Bookmark limit reached.".into());
    }
    store
        .bookmark_save(&b)
        .map_err(|_| "Could not save bookmark.".into())
}
pub fn delete(app: &AppHandle, id: &str) -> Result<(), String> {
    app.state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .bookmark_delete(id)
        .map_err(|_| "Could not remove bookmark.".into())
}
pub async fn open(app: &AppHandle, id: &str) -> Result<(), String> {
    let b = list(app)?
        .into_iter()
        .find(|b| b.id == id)
        .ok_or("Bookmark unavailable.")?;
    let s = site(app, &b.site_id)?;
    let base = md_moodle::normalize_site(&s.wwwroot).map_err(|_| ERROR)?;
    let target = base.join(&b.path).map_err(|_| ERROR)?;
    if path(&base, &target)? != b.path {
        return Err(ERROR.into());
    }
    crate::commands::restore_site_for_user(app, &b.site_id, Some(target), Some(b.user_id)).await
}

/// A real AppKit context menu: it stays above the remote Moodle view.
pub fn menu(app: &AppHandle) -> Result<(), String> {
    use tauri::menu::{ContextMenu, MenuBuilder, MenuItemBuilder};
    let fr = crate::menus::french(app);
    let mut menu = MenuBuilder::new(app);
    let items = list(app)?;
    let sites = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|_| "Site storage unavailable.")?
        .sites()
        .map_err(|_| "Could not read sites.")?;
    for b in items.iter().skip(5) {
        let label = if sites.len() > 1 {
            format!(
                "{} — {}",
                b.name,
                sites
                    .iter()
                    .find(|s| s.id == b.site_id)
                    .map(|s| md_moodle::plain_text(&s.name, 40))
                    .unwrap_or_default()
            )
        } else {
            b.name.clone()
        };
        menu = menu.item(
            &MenuItemBuilder::with_id(format!("bookmark:{}", b.id), label)
                .build(app)
                .map_err(|_| "Could not show bookmarks.")?,
        );
    }
    let manage = MenuItemBuilder::with_id(
        "bookmarks",
        if fr {
            "Modifier les signets…"
        } else {
            "Edit Bookmarks…"
        },
    )
    .build(app)
    .map_err(|_| "Could not show bookmarks.")?;
    let menu = if items.len() > 5 {
        menu.separator()
    } else {
        menu
    };
    let menu = menu
        .item(&manage)
        .build()
        .map_err(|_| "Could not show bookmarks.")?;
    let window = app.get_window("main").ok_or("Main window missing.")?;
    menu.popup(window)
        .map_err(|_| "Could not show bookmarks.".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bookmarks_strip_secrets_and_reject_actions_other_origins_and_sibling_sites() {
        let b = Url::parse("https://example.org/campus/").unwrap();
        assert_eq!(
            path(
                &b,
                &b.join("course/view.php?id=42&sesskey=secret&token=secret#anchor")
                    .unwrap()
            )
            .unwrap(),
            "course/view.php?id=42"
        );
        assert_eq!(
            path(&b, &b.join("mod/forum/discuss.php?d=7").unwrap()).unwrap(),
            "mod/forum/discuss.php?d=7"
        );
        for u in [
            "https://evil.org/campus/course/view.php?id=2",
            "https://example.org/course/view.php?id=2",
            "https://user:secret@example.org/campus/my/",
            "https://example.org/campus/login/index.php",
            "https://example.org/campus/login/logout.php",
            "https://example.org/campus/admin/tool/mobile/autologin.php?key=secret",
            "https://example.org/campus/mod/forum/post.php?id=2",
        ] {
            assert!(path(&b, &Url::parse(u).unwrap()).is_err());
        }
    }
}
