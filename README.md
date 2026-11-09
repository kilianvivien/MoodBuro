# MoodBuro

A macOS-first desktop home for your organisation’s Moodle, built with Tauri 2, React and Rust. Users enter their own organisation’s HTTPS Moodle base URL during onboarding and sign in on the organisation’s own page. MoodBuro never draws a password form.

This repository implements the **Alpha foundation**, the first slice of W1 in the [Alpha plan](docs/alpha-plan.md). It is not yet a tester-ready Alpha.

## Run locally

Install Node 22+, pnpm 9.15.9, Xcode command line tools and Rust (the toolchain file pins 1.91.1).

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
```

For frontend development alone, use `pnpm dev`. Authentication requires the desktop runtime; browser mode shows the onboarding but does not fake sign-in.

```sh
pnpm build
pnpm lint
pnpm test
pnpm format:check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/check-security.py
```

## Implemented

- Organisation URL onboarding, public mobile configuration discovery, dedicated HTTPS sign-in window and one-use signed token callback verification.
- Verified tokens stored in the OS credential store; secret buffers zeroized on drop. SQLite stores site metadata only.
- React shell, persisted site list, lazy Moodle child webviews, site switching, resize handling and navigation controls.
- Local shell-only IPC capabilities, explicit application command permissions, caller checks and origin-based navigation guard.
- Rust workspace, pnpm lockfile, app icons, core/UI tests and Linux/macOS CI.

Downloads are deliberately blocked until the W5 download manager exists. Window close exits the app until W4 provides a tray and background lifecycle. Site removal, re-authentication, notifications, autologin bridging, deep links and release distribution are subsequent work.

## Layout and decisions

`apps/shell` contains React/Vite/TypeScript/Zustand. `src-tauri` wires the desktop app. `crates/md-moodle`, `md-secrets` and `md-store` implement protocol/security, credentials and metadata. `md-notifier` and `md-os` are explicitly deferred extension points.

Read the [PRD](docs/prd.md), [Alpha plan](docs/alpha-plan.md), [foundation status and next steps](docs/implementation-status.md), and [initial architecture decisions](docs/adr/0001-alpha-foundation.md). The original planning documents retain their “Moodle Desk” working name; MoodBuro is the provisional implementation name matching this repository.
