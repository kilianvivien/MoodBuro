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

## Follow-on decision — macOS shell and sessions

- Native titlebar and traffic lights remain owned by macOS. Native menu items provide Settings (`Cmd+,`), Add Organisation (`Cmd+N`), reload (`Cmd+R`), history (`Cmd+[` / `Cmd+]`) and external browser (`Cmd+Shift+B`). The web shell uses system fonts, semantic light/dark colours and a restrained monochrome Lucide icon family.
- Tauri supplies actual `LiquidGlassRegular` window material on supported macOS, falling back to AppKit `Sidebar` material on earlier versions. The local shell permits transparency; content stays opaque. `macos-private-api` is enabled for transparent WKWebView rendering, consistent with the PRD’s outside-the-App-Store distribution. This is not a conversion of every HTML control to an AppKit control.
- On macOS 14+, auth and site webviews share a persistent store identified by the site's UUID. Other platforms and macOS 12–13 retain the shared default store. Site removal requests asynchronous deletion of the dedicated store and removes credentials/metadata. Shared legacy cookies are removed only when no retained site uses their domain; legacy local storage and shared IdP sessions may remain. Full SEC-8 deletion on that fallback remains an Alpha gate.
- One-use autologin URLs stay Rust-side. The Rust client identifies `MoodBuro/<version> MoodleMobile`: Moodle's stock `is_moodle_app()` requires that compatibility marker, while MoodBuro's own identity remains explicit. Key minting is never retried automatically. Unavailable/rate-limited keys fall back to an existing browser session; invalid tokens ask the user to reconnect in Settings.
- Public discovery supports same-origin HTTPS redirects and an explicit `/public/` fallback. Cross-origin canonical sites still require the user to enter the canonical URL; credentials are never forwarded across origins.
- Contract tests use synthetic stock-Moodle-shaped responses, not recordings from authenticated INSP accounts. Moodle 4.5/5.3 rigs and live SSO remain necessary before M0.

## Titlebar correction

The transparent/Expanded native titlebar failed in the running dev app. It is superseded by an opaque window with native overlay traffic lights and a locally painted 32-point centered title/drag strip. Window-wide Liquid Glass and the macOS private-API transparency feature are removed. This trades desktop translucency for reliable contrast and geometry. A user-supplied screenshot confirms the visual repair.
