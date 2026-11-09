<p align="center"><img src="assets/icon.svg" width="112" alt="MoodBuro icon" /></p>
<h1 align="center">MoodBuro</h1>
<p align="center"><strong>Your Moodle, at home on your Mac.</strong><br/>A lightweight desktop home for your organisation’s Moodle.</p>
<p align="center"><a href="README.md">Français</a> · English</p>

MoodBuro hosts the **real Moodle site** in a macOS-first Tauri application. Enter your organisation’s base URL and sign in on its own page. MoodBuro never draws a password form.

> **Early Alpha development.** Automated tests and local builds pass, but real SSO and the platform/version matrix remain unverified. No signed, notarized installer is published yet.

## Current features

- A compact macOS shell with real window controls, native menus and shortcuts, an opaque integrated centered titlebar and drag area, and consistent icons.
- Full French/English shell UI from onboarding, system-language detection, language selection and automatic/light/dark appearance.
- Organisation URL discovery, a dedicated login window, one-use callback verification, server-side token validation and OS credential storage.
- Multiple sites, lazy child webviews, session restoration and reconnect. macOS 14+ uses a dedicated persistent WebKit store per organisation.
- Editable organisation abbreviations and account removal with confirmation, credential/metadata cleanup and asynchronous isolated-profile deletion. Some shared browser data and SSO sessions can remain on older macOS; full legacy cleanup is still a validation gate.
- Background notification polling, persisted unread counts, sidebar connection indicators, dock badge and menu-bar controls. Closing the window keeps polling; use Quit to exit. Native delivery and click routing await installed-app qualification.
- A persistent Moodle Home button, native macOS back/forward navigation and controlled Moodle popups with their own toolbar and parent return path.
- Validated `moodledesk://` routing and unknown-site onboarding prefill, plus single-instance forwarding.
- Same-origin session downloads to per-organisation folders, recent history, Open/Show in Finder and confirmation for risky file types. Course routing and authenticated native-download qualification remain pending.
- Rust and UI tests, mock HTTP contracts, shell-only IPC permission checks and Linux/macOS CI.

## Development

Install Node 22+, pnpm 9.15.9, Rust and Xcode command line tools. The Rust toolchain is pinned in the repository.

```sh
git clone https://github.com/kilianvivien/MoodBuro.git
cd MoodBuro
pnpm install --frozen-lockfile
pnpm tauri dev
```

`pnpm dev` runs the frontend alone; real authentication requires Tauri. `pnpm tauri build --debug --bundles app` creates a local development app, not a notarized release.

```sh
pnpm build
pnpm lint
pnpm test
pnpm format:check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/check-security.py
```

## Architecture

`apps/shell` contains React, TypeScript, Vite and Zustand. `src-tauri` owns native windows, menus and IPC. `md-moodle`, `md-secrets` and `md-store` implement Moodle protocol/HTTP, OS credential storage and SQLite metadata. `md-notifier` provides polling decisions; `md-os` remains an extension point.

Remote Moodle/login/popup pages have no privileged IPC grants. Local popup toolbars have a separate minimal capability bound to their own window. Tokens remain in Rust and the OS credential store, never in SQLite or the local UI. Autologin URLs remain Rust-side. Synthetic HTTP fixtures do not replace live integration tests.

## Roadmap and limits

Next: INSP and Moodle 4.5/5.3 SAML/OIDC validation, installed-app security/profile checks, native notification/deep-link/download qualification, course download routing and universal macOS releases. Signing and notarization follow the unsigned Alpha. Windows and Linux remain later targets.

Downloads from outside the organisation’s HTTPS origin remain blocked. Background notification QA is described in [the test script](docs/background-qa.md). No institution is hard-coded; INSP is the first integration target.

[Progress tracker](docs/plan.md) · [PRD](docs/prd.md) · [Alpha plan](docs/alpha-plan.md) · [Implementation status](docs/implementation-status.md) · [Architecture decisions](docs/adr/0001-alpha-foundation.md) · [UI decisions](docs/ui-design.md) · [Design guidelines](docs/design-guidelines.md)

The original documents use the historical working name “Moodle Desk”; the implementation uses MoodBuro. This project is independent of Moodle HQ.

[MIT License](LICENSE) — © 2026 Kilian Vivien.

The native PDF Save adapter uses a WebKit-specific private delegate callback and still requires macOS compatibility qualification.
