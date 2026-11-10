//! Manual native QA without touching MoodBuro's accounts, Keychain or database:
//! cargo run -p moodburo --example document-preview -- /absolute/path/to/fixture.pdf [...]
#[cfg(target_os = "macos")]
#[path = "../src/native_preview.rs"]
mod native_preview;

#[cfg(target_os = "macos")]
fn main() {
    let paths: Vec<_> = std::env::args_os()
        .skip(1)
        .map(std::path::PathBuf::from)
        .collect();
    assert!(
        !paths.is_empty(),
        "Pass local PDF, DOCX, XLSX or PPTX fixtures."
    );
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                native_preview::remove(window.label());
            }
        })
        .setup(move |app| {
            for (index, path) in paths.iter().enumerate() {
                let window = tauri::WindowBuilder::new(app, format!("reader-smoke-{index}"))
                    .title(path.file_name().unwrap_or_default().to_string_lossy())
                    .inner_size(900., 720.)
                    .min_inner_size(480., 360.)
                    .visible(false)
                    .build()?;
                native_preview::mount(
                    &window,
                    &path.canonicalize()?,
                    false,
                    Box::new({
                        let path = path.clone();
                        move || path.canonicalize().map_err(|e| e.to_string())
                    }),
                )
                .map_err(std::io::Error::other)?;
                window.show()?;
            }
            #[cfg(debug_assertions)]
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    let app = handle.clone();
                    let _ = handle.run_on_main_thread(move || {
                        use tauri::Manager;
                        for label in app.windows().keys() {
                            native_preview::verify_controls(label);
                        }
                    });
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Native reader smoke failed");
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The native reader smoke harness requires macOS.");
}
