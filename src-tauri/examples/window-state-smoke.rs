//! Verify native size persistence in a separate app identity; no accounts are opened.
//! Run twice: cargo run -p moodburo --example window-state-smoke -- write|restore
#[path = "../src/window_state.rs"]
mod window_state;
use tauri::Manager;

fn main() {
    let writing = std::env::args().nth(1).as_deref() == Some("write");
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "app.moodburo.window-state-smoke".into();
    tauri::Builder::default()
        .on_window_event(|window, _| window_state::update(window))
        .setup(move |app| {
            window_state::install(app.handle());
            let window = tauri::WindowBuilder::new(app, "main")
                .title("MoodBuro window-state smoke")
                .inner_size(1120., 760.)
                .visible(false)
                .build()?;
            window_state::restore(&window);
            if writing {
                window.set_size(tauri::LogicalSize::new(1040., 680.))?;
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let app = handle.clone();
                handle
                    .run_on_main_thread(move || {
                        let window = app.get_window("main").unwrap();
                        let size = window
                            .inner_size()
                            .unwrap()
                            .to_logical::<f64>(window.scale_factor().unwrap());
                        assert_eq!((size.width, size.height), (1040., 680.));
                        window_state::save(&app);
                        println!(
                            "Native window state {}: {} x {}",
                            if writing { "saved" } else { "restored" },
                            size.width,
                            size.height
                        );
                        app.exit(0);
                    })
                    .unwrap();
            });
            Ok(())
        })
        .run(context)
        .expect("Window-state smoke failed");
}
