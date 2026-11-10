//! Rust-owned native document windows. Document paths and contents never cross IPC.
//! Quick Look handles parsing and drawing; no document HTML enters a webview.
use tauri::AppHandle;

pub fn supported(filename: &str) -> bool {
    std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "pdf" | "docx" | "xlsx" | "pptx"
            )
        })
}

pub async fn preview(app: &AppHandle, id: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let (send, receive) = tokio::sync::oneshot::channel();
        let handle = app.clone();
        let id = id.to_owned();
        app.run_on_main_thread(move || {
            let _ = send.send(native::open(&handle, &id));
        })
        .map_err(|_| "Could not preview this document.")?;
        receive
            .await
            .map_err(|_| "Could not preview this document.")?
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, id);
        Err("Document preview is currently available on macOS. Use Open in Default App.".into())
    }
}

/// Closing an account's native windows releases Quick Look's access to those files.
pub fn close_site(app: &AppHandle, site: &str) {
    use tauri::Manager;
    let prefix = format!("document-{site}-");
    for (label, window) in app.windows() {
        if label.starts_with(&prefix) {
            let _ = window.close();
        }
    }
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use crate::AppState;
    use tauri::Manager;

    pub fn open(app: &AppHandle, id: &str) -> Result<(), String> {
        // Resolve a completed managed download by ID, rejecting missing files,
        // symlinks and paths outside ~/Moodle. Never accept a frontend path/URL.
        let path = crate::downloads::path(app, id, false, false)?;
        if !supported(&path.to_string_lossy()) {
            return Err("This document type cannot be previewed.".into());
        }
        let state = app.state::<AppState>();
        let store = state
            .store
            .lock()
            .map_err(|_| "Site storage unavailable.")?;
        let item = store
            .download(id)
            .map_err(|_| "Could not read download history.")?
            .ok_or("This download is unavailable.")?;
        let appearance = store.settings().unwrap_or_default().appearance;
        drop(store);
        let label = format!("document-{}-{}", item.site_id, item.id);
        if let Some(window) = app.get_window(&label) {
            let _ = window.show();
            return window
                .set_focus()
                .map_err(|_| "Could not preview this document.".into());
        }
        if app
            .windows()
            .keys()
            .filter(|s| s.starts_with("document-"))
            .count()
            >= 3
        {
            return Err("Close a document preview before opening another.".into());
        }
        let theme = match appearance {
            md_store::Appearance::Light => Some(tauri::Theme::Light),
            md_store::Appearance::Dark => Some(tauri::Theme::Dark),
            md_store::Appearance::System => None,
        };
        let window = tauri::WindowBuilder::new(app, &label)
            .title(&item.filename)
            .inner_size(900., 720.)
            .min_inner_size(480., 360.)
            .theme(theme)
            .visible(false)
            .build()
            .map_err(|_| "Could not preview this document.")?;
        let handle = app.clone();
        let download = id.to_owned();
        let result = crate::native_preview::mount(
            &window,
            &path,
            crate::menus::french(app),
            Box::new(move || crate::downloads::path(&handle, &download, false, true)),
        )
        .and_then(|()| {
            window
                .show()
                .map_err(|_| "Could not preview this document.")?;
            window
                .set_focus()
                .map_err(|_| "Could not preview this document.".into())
        });
        if result.is_err() {
            let _ = window.close();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_supported_documents_enter_the_native_reader() {
        for name in ["notes.PDF", "lesson.docx", "budget.xlsx", "slides.pptx"] {
            assert!(supported(name));
        }
        for name in [
            "lesson.docm",
            "budget.xlsm",
            "notes.pdf.app",
            "page.html",
            "script",
            "file.docx.exe",
        ] {
            assert!(!supported(name));
        }
    }
}
