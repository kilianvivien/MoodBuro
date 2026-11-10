# Moodle Desk — Alpha Implementation Plan

Nov 9, 2026 · @Kilian

**Historical baseline and acceptance specification.** The week-by-week tasks, code sketches, proposed CI/release pipeline and dates below describe the original design, not the current implementation. The reconciled [working tracker](plan.md) owns delivery status and beta preparation; chronological evidence is in [implementation status](implementation-status.md). Current packaging uses manual `package-macos.yml` on `macos-26`, ad-hoc signing and manual upgrades; the signed updater pipeline below has not shipped. The current beta plan prioritizes features and GitHub automatic updates; Apple Developer ID signing/notarization is deferred until after beta.

Implementation progress: use [the working checklist](plan.md) and see [current status](implementation-status.md) for delivered slices and remaining gates. **Scope amendment (9 November 2026):** the [expanded Alpha feature plan](alpha-feature-expansion.md) is part of this plan and promotes bounded later-milestone features; its limits override older exclusions below.

## 1. Scope, team and exit criteria

The baseline Alpha was scoped as an unsigned (ad-hoc or self-signed) macOS build in 6 weeks by two engineers. The expanded Alpha now includes bounded slices of later milestones A1–A5 and the final A6 AI assistant as well as PRD milestones M0 (spike) and M1 (core shell); the original six-week estimate must be re-estimated for this added scope. The core includes sign-in to any Moodle site including SSO, several sites side by side, native notifications with dock badge and menu-bar extra, deep links, and basic capture of downloads from the Moodle view. It goes to 10–20 internal testers on 2–3 real Moodle sites.

**Team.** Engineer R owns the Rust core (auth, WS client, index, notifier, OS adapters, release). Engineer F owns the shell UI (React, Vite, TypeScript, Zustand) and QA scripts. Both review each other's PRs; the product owner runs the weekly demo.

**In the alpha (PRD IDs)**

| Area | Included | Deferred beyond Alpha |
| --- | --- | --- |
| Window and webviews | OS-1, SEC-1, SEC-2, SEC-6, SEC-7 | — |
| Sign-in | OS-2, SEC-3, SEC-4, OS-18 (autologin) | `login/token.php` fallback |
| Multi-site | OS-16 (up to 10 sites) | Per-site badge share in settings |
| Notifications | OS-3–OS-5, MAC-1, MAC-2 (badge); A1 inbox/Open/Mark Read and A4 opt-in local reminders (OS-9 partial) | Native actions beyond qualified platform support; reminder guarantees while quit |
| Deep links | OS-13 | — |
| Downloads | DL-1/DL-2/DL-10; A1 searchable/paginated history, Copy Path, safe retry/fallback, PDF/image preferences (DL-4/DL-7/DL-8 partial) | Pause/resume, full DL-3 sync de-dup, automatic retention and expanded DL-6 metadata |
| Courses and quick-open | A2 course browser, local pins, Cmd+L palette and optional global shortcut (OS-12/MAC-8 partial) | Server full-text and file-content search |
| Offline files | A3 explicit selected-file saving and manual refresh (OS-6/OS-7 partial) | Full scheduled course mirroring, resumable sync and automatic server deletions |
| Agenda and calendar | A4 read-only upcoming agenda/reminders; A5 manual token-free ICS snapshot (OS-8/OS-9/MAC-5 partial) | Live authenticated calendar subscription and automatic calendar refresh |
| macOS polish | MAC-6, MAC-8; MAC-9 universal binary only (signing and notarization deferred to beta) | MAC-4 Finder favourite, MAC-10 MDM |
| AI assistant | A6, after A1–A5: Albert/Mistral AI BYOK side panel, general chat, explicit Moodle context and bounded tools | Broad autonomous actions, submissions and grade changes |
| Not in alpha | — | Upload (OS-10), full automatic sync/live calendar integrations beyond the slices above, Windows, Linux |

Basic download capture is pulled forward from M2 because, without it, files downloaded in the Moodle view land in WebKit's default location and the alpha would teach testers the wrong habit.

**Expanded user-feature acceptance gates (implementations landed; qualification open)**

- [ ] A1: read/open a notification and reconcile unread counts; find an older download, copy its path, clear history without deleting files, and handle missing/expired sources honestly.
- [ ] A2: browse/pin courses and quick-open the correct course/cached activity across sites; cached/offline state and global-shortcut conflicts are explicit.
- [ ] A3: explicitly save selected course files, reopen them offline after restart and manually refresh without destroying edited files or deleting server-removed files locally.
- [ ] A4: show upcoming agenda items and deliver opt-in reminders with time-zone/DST, changed-date, stale-data and deduplication checks.
- [ ] A5: export a valid token-free ICS snapshot and import it into Apple Calendar; the UI clearly says it is not a live subscription.
- [ ] A6: qualify both BYOK providers, general chat, explicit Moodle context, bounded tools/confirmed writes, credential protection and account isolation. Implement last among Alpha features.

The full acceptance criteria are in the [feature amendment](alpha-feature-expansion.md). These gates supplement, rather than replace, the original gates below. Any change to the target feature set must be recorded explicitly.

**Exit criteria (alpha gate)**

- [ ] Sign-in works on 3 sites: one manual-login Moodle 5.3, one SAML SSO, one OAuth 2 / OIDC SSO, with the app never seeing a password.
- [ ] Token and private token are only in the Keychain (verified with `security find-generic-password` and a grep of the app data dir).
- [ ] Moodle webviews cannot call any Tauri command (verified by an injected test script in a test build).
- [ ] 3 sites signed in at once; switching takes under 300 ms once a site is loaded.
- [ ] New Moodle notification shows in Notification Center within one polling interval; badge and menu-bar count match Moodle's own unread count.
- [ ] Closing the window keeps the app polling in the menu bar; idle memory under 120 MB with one site.
- [ ] `moodledesk://` links open the right site and page, also when the app was not running.
- [ ] Downloads from the Moodle view land in `~/Moodle/<site>/…`; risky types prompt before opening.
- [ ] Universal DMG installs on a clean macOS 12 and macOS 15 machine by following the tester guide's Gatekeeper steps; Keychain does not re-prompt after an update (self-signed identity).
- [ ] Zero P0 bugs open; crash-free sessions ≥ 99 % over the last test week.

## 2. Week-by-week plan

**Scheduling amendment:** the weeks below describe the original core-shell baseline. They are not a six-week promise for the expanded Alpha. After stabilizing current PDF/popups, implement A1 inbox/download controls → A2 course index/quick-open → A3 selected offline files → A4 agenda/reminders → A5 ICS export → A6 AI chat side panel, then combined qualification/release. Estimate each slice and update the working tracker; run independent M0 checks alongside feature work.

In the original baseline, weeks 1–2 covered the M0 spike and three-site SSO; weeks 3–6 covered the core shell and first Alpha build. Weekly demos and local builds remain useful checkpoints, but the expanded feature sequence above replaces that original release timing.

&#91;embedded content: alpha timeline · 6 weeks, 2 lanes, 2 gates\]

The week-by-week checklist in §7 is retained as historical scope. Track implementation and qualification separately in [plan.md](plan.md).

## 3. Repository layout, tooling and CI

One repository, one Cargo workspace, one Vite app; the Rust core is split into crates so OS adapters and Moodle logic can be tested without a window.

```
moodle-desk/
├─ apps/shell/                 # React + Vite + TS + Zustand (local UI)
│  ├─ src/stores/              # sites, unread, downloads, settings
│  ├─ src/views/               # Sidebar, Toolbar, AddSite, Settings, DownloadsMenu
│  └─ src/ipc.ts               # typed wrappers over invoke() + listen()
├─ src-tauri/                  # Tauri app crate (thin: wiring only)
│  ├─ capabilities/            # local shell + restricted popup toolbar; no remote grants
│  ├─ tauri.conf.json
│  └─ src/{main.rs,lib.rs,commands.rs,windows.rs,tray.rs}
├─ crates/
│  ├─ md-moodle/               # WS client, token parsing, DTOs (no Tauri dependency)
│  ├─ md-store/                # SQLite index + migrations
│  ├─ md-secrets/              # keyring wrapper
│  ├─ md-notifier/             # polling loop, diffing, backoff
│  └─ md-os/                   # trait OsAdapter + macos.rs (windows.rs, linux.rs stubs)
├─ test-rigs/                  # docker-compose: Moodle 4.5, 5.3, Keycloak
└─ .github/workflows/          # ci.yml, release-macos.yml
```

| Tool | Version / choice | Why |
| --- | --- | --- |
| Rust | stable, pinned in `rust-toolchain.toml` | Reproducible builds |
| Tauri | 2.x latest stable, `unstable` feature for child webviews | Multi-site in one window |
| Node | 22 LTS, pnpm | Matches existing Vite projects |
| Lint / format | `clippy -D warnings`, `rustfmt`, ESLint, Prettier | Enforced in CI |
| Tests | `cargo nextest`, Vitest, `wiremock` for HTTP | Fast, no live Moodle needed for unit tests |
| Audit | `cargo audit`, `cargo deny`, `pnpm audit` | SEC-10 |
| Logging | `tracing` + `tauri-plugin-log`, redaction layer for tokens and URLs with `token=` | SEC-3 |

**CI (GitHub Actions).** `ci.yml` on every PR: lint, unit and contract tests on `ubuntu-latest` (core crates are OS-agnostic), plus a `macos-14` job that runs `cargo check` on the macOS adapter and builds the app unsigned. `release-macos.yml` on tag `v0.*`: universal build, sign, notarize, staple, upload DMG and updater JSON (§6).

**Conventions.** Trunk-based with short PRs; every PR names its PRD IDs in the title (e.g. `OS-3: notifier diffing`); a PR that touches `capabilities/` or `windows.rs` needs both engineers' approval.

## 4. Module-by-module implementation

Each module below lists what to build, the key code shape and its done-when test. API names are Tauri 2 and stock Moodle; anything marked *verify* is confirmed during the W1 spike.

### 4.1 Window, webviews and capabilities (W1, R)

One window `main` holds the local shell webview (`shell`) and one child webview per site (`site-<id>`), created lazily on first switch and positioned by Rust to the right of the 72 px sidebar and below the 44 px toolbar.

```rust
let site = WebviewBuilder::new(format!("site-{id}"), WebviewUrl::External(start_url))
    .user_agent(&format!("{default_ua} MoodleDesk/{VERSION}"))
    .on_navigation(move |url| nav_guard(&allowlist, url))   // SEC-2 + token interception
    .on_download(move |wv, ev| downloads::handle(wv, ev))     // DL-1/DL-2
    .data_directory(site_profile_dir(id));                    // cookies isolated per site
window.add_child(site, LogicalPosition::new(72.0, 44.0), content_size)?;
```

`capabilities/shell.json` grants permissions to `"webviews": ["shell"]` only; no capability names a `site-*` label and none uses `remote.urls`, so Moodle pages get no IPC (SEC-1). `withGlobalTauri` stays false. On window resize, Rust recomputes child bounds; the shell UI never positions webviews.

*Verify:* `data_directory` per child webview on macOS (WKWebView data stores need macOS 14+ for multiple persistent stores); fallback on macOS 12–13 is one shared store with cookies namespaced by origin, which is safe because each site is a different origin.

Done when: an injected `window.__TAURI_INTERNALS__` probe in a site webview finds nothing, and resizing never shows a gap or overlap.

### 4.2 Navigation guard (W1, R)

`nav_guard` returns `true` to allow, `false` to block, per URL:

1. Scheme `moodlemobile`/`moodledesk` with `token=` → hand to auth (4.3), block.
2. Origin = site origin, or an IdP origin recorded during this site's sign-in → allow.
3. `http://` on a non-dev build → block and show an error page (SEC-7).
4. Anything else → block and open in the default browser with the opener plugin.

IdP origins are learned during sign-in (every origin the auth webview passes through between `launch.php` and the token redirect) and stored per site.

### 4.3 Sign-in (W1–W2, R + F)

Flow, all in Rust except the Add Site form:

1. F: Add Site form takes a URL; R normalizes it (scheme, trailing slash, `/public` redirect on 5.1+) and calls `tool_mobile_get_public_config` via `ajax`/REST without token to get `wwwroot`, site name, `typeoflogin`, `launchurl`.
2. R: generate `passport` (32 random hex chars), open an auth window `auth-<id>` on `{launchurl}?service=moodle_mobile_app&passport={p}&urlscheme=moodlemobile`.
3. The auth webview's `on_navigation` sees `moodlemobile://token=<b64>`; decode → `signature:::token:::privatetoken`; check `signature == md5(wwwroot + passport)`; reject otherwise.
4. Store secrets (4.4), call `core_webservice_get_site_info`, write the site row (4.6), close the auth window, open the site webview via autologin (step 5).
5. Autologin: `tool_mobile_get_autologin_key(privatetoken)` → `{key, autologinurl}`; load `autologinurl?userid=…&key=…&urlto=<dashboard>`. The site rate-limits key requests, so on refusal load the dashboard and let the session cookie (shared with the auth webview profile) carry the login.

```rust
pub fn parse_launch_token(url: &Url, wwwroot: &str, passport: &str) -> Result<Tokens> {
    let b64 = url.as_str().split("token=").nth(1).ok_or(Error::NoToken)?;
    let raw = String::from_utf8(BASE64_STANDARD.decode(b64.trim_end_matches('/'))?)?;
    let parts: Vec<&str> = raw.split(":::").collect();
    let expected = format!("{:x}", md5::compute(format!("{wwwroot}{passport}")));
    ensure!(parts.first() == Some(&expected.as_str()), Error::BadSignature);
    Ok(Tokens { token: parts[1].into(), private: parts.get(2).map(|s| s.to_string()) })
}
```

Done when: the 3 rig sites (§5) sign in; a tampered signature is rejected by a unit test; the auth window never shows a password field drawn by the app.

### 4.4 Secrets (W1, R)

`md-secrets` wraps `keyring` (v3, `apple-native` feature) with service `app.moodledesk` and account `site:<uuid>:token` / `site:<uuid>:private`. API: `put`, `get`, `delete`, `delete_site`. Tokens live in memory only inside `md-moodle::Client` and are zeroized on drop (`zeroize`). Done when: deleting a site leaves no entry in Keychain Access.

### 4.5 Moodle WS client (W2, R)

`md-moodle::Client { wwwroot, token, http: reqwest::Client }` with one generic call:

```rust
pub async fn call<T: DeserializeOwned>(&self, f: &str, args: &[(&str, String)]) -> Result<T> {
    let mut form = vec![("wstoken", self.token.clone()), ("wsfunction", f.into()),
                        ("moodlewsrestformat", "json".into())];
    form.extend(args.iter().map(|(k, v)| (*k, v.clone())));
    let body: Value = self.http.post(self.endpoint()).form(&form).send().await?.json().await?;
    if let Some(code) = body.get("errorcode") { return Err(Error::Moodle(code.to_string())); }
    Ok(serde_json::from_value(body)?)
}
```

Typed wrappers for alpha: `site_info`, `get_public_config`, `get_autologin_key`, `popup_notifications`, `unread_popup_count`, `unread_conversations_count`, `mark_notification_read`. Errors map to states: `invalidtoken` → site needs re-auth (badge on its sidebar icon), network → offline, others → logged. Timeouts 15 s, 2 retries with jitter, max 4 in-flight per site.

### 4.6 Local index (W3, R)

`md-store` on `rusqlite` + `rusqlite_migration`, file `~/Library/Application Support/app.moodledesk/index.db`, WAL mode. Alpha schema:

```sql
CREATE TABLE sites (id TEXT PRIMARY KEY, wwwroot TEXT UNIQUE, name TEXT, user_id INTEGER,
  full_name TEXT, avatar_url TEXT, idp_origins TEXT, position INTEGER, state TEXT,
  last_seen_notification INTEGER DEFAULT 0, created_at INTEGER);
CREATE TABLE notifications (site_id TEXT, moodle_id INTEGER, subject TEXT, context_url TEXT,
  time_created INTEGER, read INTEGER, PRIMARY KEY (site_id, moodle_id));
CREATE TABLE downloads (id TEXT PRIMARY KEY, site_id TEXT, course_id INTEGER, source_url TEXT,
  page_url TEXT, local_path TEXT, size INTEGER, state TEXT, started_at INTEGER, finished_at INTEGER);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
```

No tokens or file contents in this database (SEC-3).

### 4.7 Multi-site and shell UI (W3, F + R)

Shell layout: 72 px sidebar of site avatars (drag to reorder, unread dot, warning dot for re-auth), 44 px toolbar (back, forward, reload, site name, "Open in browser", downloads menu), content area occupied by the active site webview. Zustand stores: `sites`, `unread`, `downloads`, `settings`, fed by Rust events.

IPC surface (all commands in `commands.rs`, all typed in `ipc.ts`):

| Command | Purpose |
| --- | --- |
| `site_add(url)` → `SiteSummary` | Runs 4.3 |
| `site_remove(id, wipe_downloads)` | SEC-8 |
| `site_switch(id)` | Show/create child webview, hide others |
| `site_reorder(ids)` | Sidebar order |
| `nav(action)` | back, forward, reload, open\_in\_browser on active site |
| `downloads_recent(limit)` / `download_open(id)` / `download_reveal(id)` | Minimal list |
| `settings_get()` / `settings_set(patch)` | Poll interval, menu-bar-only, autostart (off in alpha) |

Events from Rust: `sites://changed`, `unread://changed`, `downloads://progress`, `auth://needed`. Done when: 3 sites switch under 300 ms after first load and hidden webviews stop timers via WebKit's background throttling (*verify*).

### 4.8 Notifier (W4, R)

One `tokio` task per site in `md-notifier`, interval from settings (default 120 s), paused when the Mac sleeps and resumed on wake.

1. `message_popup_get_unread_popup_notification_count` + `core_message_get_unread_conversations_count` → unread total.
2. If the count rose, `message_popup_get_popup_notifications(newestfirst=1, limit=20)`; new = `id > last_seen_notification`.
3. For each new item (max 5 per tick, then a summary "+N more"): native notification with site name as title, Moodle subject as body.
4. Persist rows, update `last_seen_notification`, emit `unread://changed`.
5. Backoff: on errors double the interval up to 15 min; on `invalidtoken` stop and emit `auth://needed`.

The first poll after sign-in only seeds `last_seen_notification`; it never fires a burst of old notifications.

*Verify:* notification click routing. `tauri-plugin-notification` on desktop focuses the app but may not report which notification was clicked; if so, use a thin macOS adapter on `UNUserNotificationCenter` (e.g. the `user-notify` or `mac-notification-sys` crates) so a click opens `context_url` in the right site. Alpha accepts "click focuses the app and opens the notifications page" as fallback.

### 4.9 Menu-bar extra, dock badge, window lifecycle (W4, R)

- Tray built with `TrayIconBuilder` (feature `tray-icon`), template icon so it follows light/dark menu bar; `tray.set_title(Some("3"))` shows the count beside the icon on macOS.
- Tray menu (rebuilt on `unread://changed`): per site, the 5 latest unread items; then "Recent downloads" submenu; "Open Moodle Desk", "Check now", "Settings…", "Quit".
- Dock badge: `window.set_badge_count(Some(n))`, cleared at 0.
- Close button hides the window (`CloseRequested` → `api.prevent_close()` + `hide()`); Cmd+Q quits. The setting "Menu bar only" switches `ActivationPolicy::Accessory` / `Regular` at runtime.
- Window size and position via `tauri-plugin-window-state`.

### 4.10 Deep links and single instance (W5, R)

`tauri-plugin-deep-link` with `plugins.deep-link.desktop.schemes = ["moodledesk"]` (written into Info.plist at bundle time) and `tauri-plugin-single-instance` with its `deep-link` feature so a second launch forwards the URL. Router: `moodledesk://<host>/<path>` → find site by host; if signed in, `site_switch` and navigate to `wwwroot + path`; if unknown, open Add Site prefilled. Only paths on the site's own origin are accepted. Note: deep links only work from the installed `.app` bundle, not from `tauri dev`, so QA uses signed nightly builds.

### 4.11 Download capture (W5, R + F)

`downloads::handle` on `DownloadEvent::Requested { url, destination }`:

1. Resolve site from the webview label; classify the URL: `pluginfile.php/<contextid>/…` with a known course context → `~/Moodle/<site>/<course short name>/`; otherwise `~/Moodle/<site>/Downloads/`. (In alpha, course names come from `core_enrol_get_users_courses`, cached hourly; the context→course map is filled lazily from the page URL the user was on.)
2. Sanitize file name, add `  (2) ` on collision, set `*destination`, insert a `downloads` row, emit progress.

On `DownloadEvent::Finished { path, success }`: update the row; notify "Downloaded \<file> — Open / Show in Finder"; if the extension is on the risky list (DL-10), never auto-open and show a confirmation in the shell before any open. WebKit downloads with the user's session, so no cookie handling is needed in Rust. The menu-bar "Recent downloads" and the toolbar downloads menu read the last 20 rows.

## 5. Test plan

Testing rests on a local rig of two Moodle versions and an identity provider, automated tests for everything below the window, and scripted manual QA for the macOS surfaces, because Tauri's WebDriver (`tauri-driver`) does not support macOS.

### 5.1 Test rig (`test-rigs/`, ready end of W1)

| Service | Purpose | Notes |
| --- | --- | --- |
| Moodle 4.5 LTS | Oldest supported version | Based on `moodlehq/moodle-docker`; mobile web services on; seeded with 2 courses, 3 users |
| Moodle 5.3 LTS | Newest version, `/public` layout | Same seed script |
| Keycloak | SAML IdP and OIDC provider | 5.3 uses SAML via the `auth_saml2` plugin; 4.5 uses core OAuth 2 against Keycloak OIDC |
| Mailpit | Catch site e-mail | Account confirmation flows |
| Caddy | TLS with a local CA for `*.moodle.test` | SEC-7 requires HTTPS even in tests |

A seed script creates notifications and messages on demand (`make notify SITE=53 N=5`) so notifier tests are repeatable. One real institutional site joins in W2 as the third sign-in target, with its admin informed (PRD SEC-11).

### 5.2 Automated tests

| Layer | What | Tool | Runs |
| --- | --- | --- | --- |
| Unit | Token parsing and signature, URL normalization, nav guard rules, download routing and name sanitizing, backoff | `cargo nextest` | Every PR |
| Contract | Each WS wrapper against recorded JSON from 4.5 and 5.3; error shapes (`invalidtoken`, `accessexception`) | `wiremock` + fixtures | Every PR |
| Live integration | Same wrappers against the docker rig with a real token | `cargo nextest --features live` | Nightly |
| Store | Migrations up from empty and from previous alpha builds | `cargo nextest` | Every PR |
| UI | Stores and components with mocked IPC | Vitest + Testing Library | Every PR |
| Security | Build a test variant that injects a probe into site webviews and asserts no IPC; grep logs and DB for token patterns | Custom script | Nightly |

### 5.3 Manual QA script (each nightly build, \~30 min)

1. Fresh install from DMG on macOS 12 and macOS 15 (Apple Silicon); one Intel Mac once a week.
2. Add each of the 3 sites; confirm no password field is drawn by the app; check Keychain entries.
3. Switch sites rapidly 20 times; back/forward across SSO redirects.
4. Trigger 5 notifications from the rig; check Notification Center, badge, menu-bar title and list; Focus mode on/off.
5. Close window, wait one interval, trigger again; Cmd+Q; reopen.
6. Click `moodledesk://` links from Mail and Notes, with the app running and quit.
7. Download a PDF, a .docx, an .mbz backup and a .dmg from the rig; check folders, risky-type prompt, menu list.
8. Sleep the Mac 10 minutes, wake, check the notifier resumes without duplicates.
9. Sign out of one site; confirm Keychain entry, cookies and rows are gone.

Bugs go to the tracker with label `alpha`, severity P0–P3 and the PRD ID.

## 6. Alpha build and distribution (unsigned)

Alpha builds ship as an **unsigned** universal DMG from CI: ad-hoc signed only (required for Apple Silicon), not Developer ID-signed, not notarized. Developer ID signing and notarization (PRD MAC-9, SEC-9) move to the beta. The Tauri updater, which uses its own signature independent of Apple's, still runs on an `alpha` channel.

**What "unsigned" means for testers and how we handle it**

| Effect | Handling in alpha |
| --- | --- |
| Gatekeeper blocks the first launch ("cannot be opened because Apple cannot check it") | Tester guide: on macOS 12–14, Control-click → Open; on macOS 15, try to open once, then System Settings → Privacy & Security → Open Anyway (the Control-click bypass no longer works there); terminal alternative `xattr -dr com.apple.quarantine "/Applications/Moodle Desk.app"` |
| Keychain asks "Moodle Desk wants to use your confidential information" again after each update, because an ad-hoc signature changes every build | Sign every alpha with the same self-signed code-signing certificate created once in Keychain Access and stored in CI secrets, so the app's identity stays stable and "Always Allow" survives updates (*verify* in W1); otherwise testers click Always Allow once per build |
| Notifications, deep links, tray | Work with an ad-hoc or self-signed bundle installed in `/Applications` (*verify* in W1, D4) |
| Updater | Tauri-signed `.app.tar.gz` replaces the bundle in place; no new Gatekeeper prompt expected for files the app downloads itself (*verify* with alpha.2) |
| Trust | DMG distributed only on the private page with its SHA-256 printed next to it |

**Prerequisites (W1, product owner)**

- [ ] Self-signed code-signing certificate ("Moodle Desk Alpha") exported to CI secrets.
- [ ] Updater key pair from `tauri signer generate`; private key and password in CI secrets, public key in `tauri.conf.json`.
- [ ] Static HTTPS host for the DMG and `alpha/latest.json`.
- [ ] Tester guide page with the Gatekeeper steps above, with screenshots for macOS 12 and 15.

**Build pipeline (`release-macos.yml`, runs on `macos-14`)**

1. Checkout, install Rust targets `aarch64-apple-darwin` and `x86_64-apple-darwin`, pnpm install.
2. Import the self-signed certificate into a temporary keychain.
3. `pnpm tauri build --target universal-apple-darwin` with `APPLE_SIGNING_IDENTITY="Moodle Desk Alpha"` and `TAURI_SIGNING_PRIVATE_KEY`; no Apple notarization variables set, so the bundler skips notarization.
4. `bundle.createUpdaterArtifacts: true` produces the `.app.tar.gz` and `.sig`; a script writes `alpha/latest.json`.
5. Verify: `codesign --verify --deep --strict` passes; `lipo -archs` shows both architectures.
6. Upload DMG, its SHA-256, updater artifacts and `latest.json`; post the changelog to the testers' channel.

**Moving to signed builds (beta).** Enrol in the Apple Developer Program, add a Developer ID Application certificate and an App Store Connect API key to CI, switch `APPLE_SIGNING_IDENTITY`, enable hardened runtime notarization. Testers' Keychain items will prompt once more when the signing identity changes.

**Entitlements.** No App Sandbox (PRD MAC-9: the `~/Moodle` folder model needs it off); hardened runtime can be switched on already in alpha to surface entitlement issues early.

**Distribution.** Private download page; testers sign a short alpha agreement (data stays local, logs shared only on request). Crash reports come from `tauri-plugin-log` files attached to bug reports; no telemetry service.

**Versioning.** `0.1.0-alpha.N`, one build per week (Friday), plus hotfix builds for P0s. The updater checks at launch and every 6 hours and installs on next quit.

## 7. Spike decisions, risks and task checklist

Week 1 answers five questions that could change the design; each has a default, a test and a fallback, and the answers are recorded in `docs/adr/` before W2 starts.

| # | Decision | Test in W1 | Fallback if it fails |
| --- | --- | --- | --- |
| D1 | Child webviews (`unstable`) in one window | 3 sites, resize, focus, keyboard shortcuts, find-in-page | One window per site with a native Window menu switcher |
| D2 | Per-site WebKit data stores | Two sites with separate cookies on macOS 12 and 15 | Shared store (safe: different origins) |
| D3 | Intercepting `moodlemobile://` in `on_navigation` | Sign in on rig 4.5, 5.3 and the real site | Ask site admin for a dedicated service + `moodledesk` scheme; `login/token.php` for manual-login sites |
| D4 | Notification click routing on macOS | Click a notification with the app hidden and quit | Thin `UNUserNotificationCenter` adapter; or click opens the notifications page |
| D5 | `on_download` sets destination on WKWebView | PDF, forced-download and `Content-Disposition` files | Cancel the webview download and refetch in Rust with the cookies from the site webview |

**Risks for the alpha**

| Risk | Likelihood | Mitigation |
| --- | --- | --- |
| Unsigned build: Gatekeeper friction and Keychain prompts put testers off | Medium | Tester guide with screenshots; stable self-signed identity across builds (§6); signed builds in beta |
| An SSO IdP refuses embedded WebKit (Google-style OAuth) | Medium | Not in alpha rigs; record which IdPs fail; PRD fallback with system browser in beta |
| Real site's admin restricts `moodle_mobile_app` | Low | Talk to the admin in W1; keep a second real site in reserve |
| Tauri `unstable` API changes on update | Low | Pin Tauri minor version for the alpha |
| Notifier load on real sites | Low | 120 s default, count-first polling, backoff; share numbers with the admin |

**Original core-shell task checklist by week**

A1–A6 and the core-shell implementation are reconciled in [plan.md](plan.md). The unchecked bundles below are historical: each mixes implementation with still-open qualification/setup tasks and must not be read as wholly unstarted.

- [ ] W1 R: workspace, CI, child webview + capability isolation, nav guard, auth window + token parsing, Keychain wrapper
- [ ] W1 F: shell skeleton (sidebar, toolbar, Add Site), test rig compose, self-signed certificate, updater key and download host with product owner
- [ ] W2 R: WS client + typed wrappers, public config, site info, autologin, contract fixtures for 4.5 and 5.3
- [ ] W2 F: sign-in UX (progress, errors, re-auth), rig SSO via Keycloak, ADRs D1–D5; **M0 gate: SSO on 3 sites**
- [ ] W3 R: SQLite store + migrations, site lifecycle (add, remove, re-auth), child webview manager
- [ ] W3 F: multi-site sidebar (reorder, dots), Zustand stores, settings view
- [ ] W4 R: notifier tasks, sleep/wake, notifications, badge, menu-bar extra, hide-on-close, activation policy
- [ ] W4 F: unread UI, tray menu content, first manual QA run
- [ ] W5 R: deep links + single instance, download capture and routing, risky-type guard
- [ ] W5 F: downloads menu, open / reveal, deep-link Add Site prefill, accessibility pass on the shell
- [ ] W6 R: unsigned universal release pipeline with stable self-signed identity, updater channel, log redaction audit
- [ ] W6 F: full QA matrix, bug bash, tester guide; **alpha gate review and release `0.1.0-alpha.1`**
