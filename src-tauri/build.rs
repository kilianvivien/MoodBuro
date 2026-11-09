fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "sites_list",
            "unread_list",
            "site_label_set",
            "site_add",
            "site_switch",
            "show_onboarding",
            "nav",
            "site_reauthenticate",
            "site_remove",
            "shell_view",
            "settings_get",
            "settings_set",
        ]),
    ))
    .expect("Tauri build failed");
}
