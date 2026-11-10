//! Move a sibling webview above the others without hiding or reloading them.
#[cfg(target_os = "macos")]
pub fn raise(view: &tauri::Webview) -> tauri::Result<()> {
    view.with_webview(|native| unsafe {
        // WKWebView is an NSView subclass; this closure runs on AppKit's thread.
        let view = &*(native.inner() as *const objc2_app_kit::NSView);
        if let Some(parent) = view.superview() {
            parent.addSubview_positioned_relativeTo(
                view,
                objc2_app_kit::NSWindowOrderingMode::Above,
                None,
            );
        }
    })
}
