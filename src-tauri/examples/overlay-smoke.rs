//! Native sibling-view ordering check with synthetic views and no saved accounts.
#[cfg(target_os = "macos")]
#[path = "../src/native_overlay.rs"]
mod native_overlay;
#[cfg(target_os = "macos")]
fn main() {
    use tauri::{webview::WebviewBuilder, LogicalPosition, LogicalSize, Manager, WebviewUrl};
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "app.moodburo.overlay-smoke".into();
    tauri::Builder::default()
        .setup(|app| {
            let window = tauri::WindowBuilder::new(app, "main")
                .visible(false)
                .build()?;
            for label in ["shell", "site-fixture"] {
                window.add_child(
                    WebviewBuilder::new(
                        label,
                        WebviewUrl::External("about:blank".parse().unwrap()),
                    )
                    .background_color(tauri::window::Color(0, 0, 0, 0)),
                    LogicalPosition::new(0., 0.),
                    LogicalSize::new(600., 400.),
                )?;
            }
            let app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                let handle = app.clone();
                app.run_on_main_thread(move || {
                    let shell = handle.get_webview("shell").unwrap();
                    let site = handle.get_webview("site-fixture").unwrap();
                    for view in [&shell, &site, &shell, &site] {
                        native_overlay::raise(view).unwrap();
                        view.with_webview(|native| unsafe {
                            let v = &*(native.inner() as *const objc2_app_kit::NSView);
                            assert!(!v.isHidden());
                            let parent = v.superview().unwrap();
                            assert!(std::ptr::eq(&*parent.subviews().lastObject().unwrap(), v));
                        })
                        .unwrap();
                    }
                    println!("Native overlay: shell/site sibling ordering and visibility passed.");
                    handle.exit(0);
                })
                .unwrap();
            });
            Ok(())
        })
        .run(context)
        .expect("Overlay smoke failed");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
