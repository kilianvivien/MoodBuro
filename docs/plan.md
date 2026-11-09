# MoodBuro — implementation tracker

Last updated: 9 November 2026.

This is the working checklist for project advancement. The [PRD](prd.md) defines the product, and the [Alpha implementation plan](alpha-plan.md) supplies the original scope and sequence. Work has landed across parts of W1–W3; that does not mean those weeks or their validation gates are complete.

**How to maintain this file:** check an implementation item when it is built and its relevant checks pass. Keep live/platform validation as separate unchecked items until evidence exists. After each implementation slice, update the completed work, next priorities, verification evidence and any decisions or blockers.

## Current position

| Milestone | Status | Remaining gate |
| --- | --- | --- |
| M0 — architecture and sign-in spike | In progress | Repeatable three-site login/SSO tests, platform/security validation and D1–D5 decisions |
| M1 — Alpha core shell | Partially implemented | Notifications, badge/tray/background lifecycle, deep links, downloads and release qualification |
| First tester Alpha | Not ready for distribution | All Alpha exit criteria, universal packaging and private distribution setup |

The INSP dashboard has been observed authenticated in the app. This is a successful real-site observation; it does not close the complete SSO, session-restoration or security test matrix.

## Completed implementation

### Foundation and tooling

- [x] Connect the local project to `kilianvivien/MoodBuro` and publish implementation to `main`.
- [x] Move the PRD and original Alpha plan into `docs/`.
- [x] Create the pnpm workspace, React/Vite/TypeScript/Zustand shell and Rust/Cargo workspace.
- [x] Add the Tauri app and separate Moodle, credential-store and SQLite crates.
- [x] Scaffold notifier and OS-adapter crates as explicit future extension points.
- [x] Pin toolchains, commit lockfiles, and configure frontend checks and Linux/macOS CI jobs.
- [x] Add French and English GitHub READMEs and app icon assets.

### Shell and macOS interface

- [x] Build organisation URL onboarding, sidebar, toolbar and Settings.
- [x] Render the real Moodle site in lazy child webviews, with Rust-owned sizing and site switching.
- [x] Add back/forward/reload and open-in-browser controls.
- [x] Add native menus and shortcuts for settings, adding sites and navigation.
- [x] Provide French/English copy, accessible labels and known-error translations from onboarding.
- [x] Follow system language/appearance by default; persist explicit language and theme choices.
- [x] Add keyboard navigation, focus treatment and contrast/motion/transparency preferences.
- [x] Repair the transparent-titlebar regression: use an opaque, centered title strip with native traffic lights and a local drag region.
- [x] Adjust remote webview bounds below the integrated window chrome.
- [x] Add editable per-organisation abbreviations while retaining the full accessible site name.

### Authentication, sessions and local data

- [x] Normalize HTTPS organisation URLs and discover public Moodle mobile configuration.
- [x] Support same-origin HTTPS redirects and the explicit `/public/` discovery fallback.
- [x] Open the organisation’s own login page in an unprivileged auth window.
- [x] Verify one-use token callbacks, bound callback input, and validate tokens with Moodle before persistence.
- [x] Store tokens in the OS credential store; keep metadata and preferences in SQLite.
- [x] Add reconnect, cancellation/race guards and credential rollback attempts on failed persistence.
- [x] Use dedicated persistent WebKit profiles for auth/site views on macOS 14+.
- [x] Implement cold-view autologin restoration and warm-view reuse.
- [x] Check restored token/account identity and restrict autologin destinations to the expected Moodle endpoint.
- [x] Add sanitized service errors, read retries/backoff, timeouts and bounded concurrent requests per HTTP client.
- [x] Implement typed site-info, autologin and unread-count API wrappers.
- [x] Add confirmed local account removal, credential/metadata cleanup and isolated-profile deletion requests.
- [x] Add fixed, development-only authentication milestones that accept no URL or credential payload.

### Security boundaries and automated verification

- [x] Grant privileged IPC only to the local shell, with explicit command permissions and caller checks.
- [x] Enforce origin-based navigation policy; block unmanaged downloads and popups for now.
- [x] Add unit/contract tests for callback and URL policies, HTTP behavior, metadata/settings/lifecycle and localization.
- [x] Pass the latest local frontend build, lint/format checks, **42 UI tests**, **19 Rust tests**, workspace Clippy and shell-capability regression checks.
- [x] Build a local ad-hoc-signed macOS app bundle; this is not a notarized distribution release.
- [x] Review available dev logs and metadata schema without exposing credentials; record findings in the [auth review](auth-review.md).

## Next steps — in priority order

### 1. Close the M0 validation gaps

- [ ] Turn the observed INSP login into a repeatable manual script covering sign-in, cancellation, reconnect, restart and session restoration.
- [ ] Test autologin rate limiting, unavailable/missing private tokens, expired tokens and network failures.
- [ ] Provision HTTPS Moodle 4.5 and 5.3 rigs with Keycloak and seeded test users/courses.
- [ ] Capture sanitized version-specific contract fixtures and test manual login, SAML and OIDC across three sites.
- [ ] Validate three simultaneous sites, resize/focus/shortcuts and warm switching against the plan’s timing target.
- [ ] Validate cookie/profile separation on macOS 12–13 fallback, macOS 15 and current macOS.
- [ ] Test the integrated titlebar’s drag/double-click/fullscreen behavior and light/dark layouts across supported macOS versions.
- [ ] Inject a runtime IPC-denial probe into remote views; configuration checks alone are insufficient.
- [ ] Verify installed-app Keychain access, failure/rollback behavior and identity across updates.
- [ ] Verify asynchronous profile deletion; resolve or explicitly gate legacy shared-browser-data cleanup (SEC-8).
- [ ] Record measured D1–D5 spike results and accepted fallbacks in `docs/adr/` before claiming M0 complete.

### 2. Finish multi-site behavior and build background notifications

- [ ] Add site reorder and explicit offline/re-auth/unread indicators.
- [ ] Complete typed notification/message wrappers and read-state operations.
- [ ] Implement per-site polling, first-poll seeding, deduplication, backoff and sleep/wake handling.
- [ ] Persist notification state and emit unread updates to the shell.
- [ ] Deliver native notifications, aggregate dock badge and menu-bar unread state.
- [ ] Implement tray/menu-bar controls, check-now action and notification click routing (D4).
- [ ] Implement hide-on-close/background operation and regular/menu-bar-only activation behavior.
- [ ] Verify resource usage, accessibility and three-site behavior with the background services running.

### 3. Add deep links and safe download capture

- [ ] Register `moodledesk://` and single-instance forwarding; route links only to an accepted site/path.
- [ ] Prefill onboarding for an unknown site opened through a deep link.
- [ ] Validate WKWebView download destination/finish behavior (D5).
- [ ] Route downloads into the correct site/course folder with safe names and collision handling.
- [ ] Persist download history and expose recent downloads with open/reveal actions.
- [ ] Require confirmation for risky file types and prevent automatic opening.
- [ ] Verify installed-app deep links and session-authenticated downloads across the test rigs.

### 4. Prepare the tester Alpha

- [ ] Configure a stable self-signed Alpha identity, updater signing keys and private HTTPS hosting with the product owner.
- [ ] Implement the universal Apple Silicon/Intel DMG release workflow and verify architectures/signatures/checksums.
- [ ] Implement the signed Alpha updater channel and test an actual update between builds.
- [ ] Add rotating/redacted diagnostic logging and configure Rust advisory auditing.
- [ ] Review and resolve outstanding dependency alerts, then enforce the agreed audit checks in CI.
- [ ] Write and verify the Gatekeeper installation/tester guide.
- [ ] Run the full QA matrix, security checks, accessibility pass and bug bash.
- [ ] Review every exit criterion in the original Alpha plan; resolve P0s before releasing `0.1.0-alpha.1` to internal testers.

## Deferred beyond the Alpha

- [ ] Course-folder sync and offline file library.
- [ ] Calendar subscriptions and deadline reminders.
- [ ] Uploads, drag-and-drop submissions and quick-open search.
- [ ] Expanded download controls, metadata and sync deduplication.
- [ ] Developer ID signing/notarization and managed deployment work.
- [ ] Windows and Linux platform validation and releases.

These are roadmap items, not enabled features. Downloads are currently blocked in Moodle views, and closing the window exits until tray/background operation is implemented.

## Evidence and related documents

- [Implementation status](implementation-status.md): verification detail and current limitations.
- [Authentication review](auth-review.md): inspected evidence and remaining security checks.
- [UI design](ui-design.md): shell, titlebar, localization and label decisions.
- [Architecture decisions](adr/0001-alpha-foundation.md): technical choices and corrections.
- [Original Alpha plan](alpha-plan.md): scope, spike definitions, QA matrix and release gate.

## Recent delivery checkpoints

| Commit | Delivered |
| --- | --- |
| `88adb27` | Workspace, shell and secure-sign-in foundation |
| `76a73f7` | Bilingual macOS shell, session/account lifecycle and READMEs |
| `4413c62` | Titlebar repair, organisation abbreviations and auth diagnostics/safeguards |
