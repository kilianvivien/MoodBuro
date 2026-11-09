# ADR 0001 — Alpha foundation

Status: accepted for scaffolding; platform spike results pending.

- **Audience:** generic app. Confirmed by the product owner. Onboarding asks for an organisation’s Moodle base URL; no institution is hard-coded or auto-connected.
- **Name:** MoodBuro provisionally, matching the repository. Bundle/keyring namespace `app.moodburo`; public branding remains reviewable.
- **Stack:** Tauri 2.12 minor pinned with `unstable` child webviews, React 19, Vite 7, TypeScript and Zustand. Lockfiles committed. Rust 1.91.1 and Node 22 baseline.
- **IPC:** local `shell` webview only. Application commands are declared in `AppManifest`, granted individually and additionally check the caller label. No window-wide capability, remote grant or global Tauri object.
- **Sign-in:** stock `moodle_mobile_app` service, embedded HTTPS login, intercepted `moodlemobile` callback. The Moodle launch protocol is confirmed against official 4.5 source. Callback passport is single-use, token is verified with site info before Keychain persistence. Browser fallback and autologin bridging are W2 work.
- **Data stores (D2):** use WebKit’s shared default store in this spike, retaining the auth session across the auth window and site child. Different origins retain browser cookie separation. Multiple accounts on the same canonical site are rejected. Persistent per-site stores on macOS 14+ remain a D2 experiment; do not mix custom site stores with a default auth store.
- **Navigation:** site origin in the Moodle view; external HTTPS links open in the OS browser. Auth may navigate HTTPS IdPs but receives no IPC grants. New windows and downloads are blocked pending explicit support. No dynamically learned IdP allowlist yet; revisit when implementing session refresh/SSO inside the site view.
- **Persistence:** first SQLite metadata schema is version 1. Tokens never enter IPC, SQLite or local storage. Destructive lifecycle operations are deferred until cookie and metadata removal can be implemented together.
- **Distribution:** no fake signing keys or updater endpoints. Universal DMG release workflow comes after actual signing identity, updater keys and hosting are configured in W6.

## References

- https://v2.tauri.app/security/capabilities/
- https://docs.rs/tauri/latest/tauri/webview/struct.WebviewBuilder.html
- https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/admin/tool/mobile/launch.php
