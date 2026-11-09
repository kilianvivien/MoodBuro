fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "sites_list",
            "site_add",
            "site_switch",
            "show_onboarding",
            "nav",
        ]),
    ))
    .expect("Tauri build failed");
}
