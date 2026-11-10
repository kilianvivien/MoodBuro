# Alpha implementation status

Use [the working plan](plan.md) to track completed tasks and next priorities.

## Expanded Alpha features — first pass built, not validated

At the product owner's request, the [Alpha scope amendment](alpha-feature-expansion.md) brings forward: A1 notification inbox and richer download controls; A2 course browser/pins and quick-open; A3 selected files saved offline with manual refresh; A4 upcoming agenda and opt-in local reminders; A5 manual ICS snapshot export. The original full background-sync/live-calendar/upload features are not implied by these smaller slices. The original six-week estimate requires re-estimation; existing release/security gates remain open.

A first pass over A1, A2 and the read-only A4 agenda is implemented (see the last section). A3, A5 and reminders are not started. [plan.md](plan.md) lists what is checked and what is still open.

## Delivered

The first two implementation slices cover the workspace/CI foundation, organisation-URL onboarding, the macOS shell and settings, account lifecycle and session-restoration infrastructure. This does **not** close M0 or claim the entire W1/W2 plan. `md-notifier` now implements polling decisions; `md-os` remains an extension point.

- Generic organisation URL onboarding, confirmed by the product owner; no institution is hard-coded.
- Public Moodle discovery, same-origin HTTPS redirects and `/public/` fallback. First integration target: `https://moodle.insp.gouv.fr/`; its public mobile config was checked without credentials.
- Dedicated login window, one-use signed callbacks, server-side token verification, credential rollback on failed persistence, reconnect, and cancellation guards against late callbacks.
- Per-site persistent WebKit profiles on macOS 14+, shared by auth and Moodle views. Older macOS retains the shared-store fallback.
- Cold-view restoration through the token/private-token autologin API; warm switches reuse the existing view. Missing/expired credentials ask for reconnect. A rate-limited/unavailable autologin key falls back to the existing browser session.
- HTTP client with sanitized typed errors, 15-second timeout, four in-flight calls per client, bounded read retries and jitter. Key minting is never retried automatically. Typed site-info, autologin and unread-count wrappers.
- Native macOS traffic lights/menus, opaque unified 52 pt toolbar with centred traffic lights, inactive-window dimming, system typography, focused icons, keyboard navigation and light/dark/automatic appearance. French/English UI, automatic language selection and explicit language preferences are supported from onboarding, including native menus. Settings persist in SQLite and apply to the native windows.
- Account removal asks for confirmation, requests dedicated-profile data deletion, and removes Keychain credentials and metadata. WebKit deletion is asynchronous. Shared legacy cookies remain if another saved site uses them; legacy local storage/shared IdP sessions may remain on the fallback. Full SEC-8 deletion on macOS 12–13 is still open.

## Verification

- 70 UI/store/reducer/helper/localization tests; production frontend build, ESLint and Prettier pass.
- 42 Rust tests (including mock HTTP contracts, URL/callback policy, SQLite migration/settings/reconnect, cookie domain boundaries); workspace Clippy with warnings denied passes.
- Native debug build/ad-hoc app packaging and shell-only IPC configuration/caller regression checks pass. Generated ACLs grant application commands individually to the local shell only.
- HTTP tests use synthetic stock-Moodle-shaped data, not authenticated recordings from INSP or the planned 4.5/5.3 rigs.
- JavaScript dependencies were not changed in the second slice. The previous audit had no high/critical findings and one moderate development-only minimatch advisory upstream. Rust advisory auditing remains to configure.

- Native UI checks on macOS 27.2 confirmed French onboarding, French Settings, English UI/native menu switching and light appearance. A user-supplied screenshot subsequently confirms the repaired opaque centered titlebar and authenticated INSP dashboard. This is evidence of a working real-site session, not a full protocol/security sign-off.
- Reviewed the available development log output: rebuilds and two earlier native window-move warnings; no credentials/callback URLs observed. Added fixed, development-only auth milestones that cannot accept credential/URL payloads. The live metadata database has only `sites`/`settings` tables and no credential-named columns; credential values were not read.
- Added bounded callback input, cold-restore account identity matching, and editable four-character per-site labels.

## Remaining M0 checks and next work

1. Extend the observed INSP sign-in success into repeatable callback, cookie and session-restoration tests. No credentials have been requested or used by the agent.
2. Provision Moodle 4.5/5.3 + Keycloak fixtures and test SAML/OIDC. Verify version-specific public roots, rate-limit behaviour and missing-private-token fallback.
3. D1/D2: three simultaneous sites, warm-switch timing, resize, focus, cookies/profile separation on macOS 12, 15 and current macOS.
4. Inject a remote-webview IPC probe; configuration checks do not prove runtime isolation. Verify Keychain prompts, deletion and app identity across installed updates.
5. Qualify the current PDF/popup/notification/download lifecycle, then implement A1–A5 in the amended order (inbox/download controls → courses/quick-open → selected offline files → agenda/reminders → ICS export), alongside independent M0 checks. Finish reorder/menu-bar-only controls, course routing and universal packaging/distribution before release. Same-origin HTTPS downloads are now captured. Closing the window hides it while polling continues; Quit exits.
6. Record D1–D5 results before claiming M0. Full legacy browser-data deletion remains an explicit privacy/platform gate.

See [authentication check](auth-review.md), [UI decisions](ui-design.md) and [architecture decisions](adr/0001-alpha-foundation.md).

## Background notification implementation

- Typed popup/count/read API wrappers, first-poll seeding, ID deduplication, persisted checkpoints, capped retries, invalid-token stop, and worker replacement/removal guards.
- Schema v3 stores counts and up to 100 notification subjects/read states per account. Context URLs and message bodies are not persisted. Same-account reconnect retains checkpoints; changing Moodle users clears them atomically.
- Rust-only native notification delivery (five subjects plus summary), dock badge, bilingual sidebar unread/offline/reconnect/error indicators and menu-bar counts. Tray supports Open, Check Now, Settings and Quit. The main window hides on close and reopens through the tray or Dock.
- Notification clicks are wired through the plugin’s Rust action callback to the saved account’s own notifications page; no server-supplied context URL is followed. Actual installed-app click delivery remains D4 validation.
- Defaults remain fixed at 120 seconds, backing off to 15 minutes. Relative sleeps avoid missed-tick bursts; explicit sleep/wake hooks and immediate-on-wake timing remain unverified. The newest 20 items are fetched each poll, even if the count stays equal; older arrivals outside that window are not delivered individually.
- Still pending: latest-unread tray items/read controls, notification preference controls, menu-bar-only mode, window-state persistence, resource measurements and live multi-site/native notification testing. Native submission success does not prove Notification Center delivery or Focus behavior.

See [background QA](background-qa.md) for the repeatable installed-app test script.

Local packaging for this slice: `CARGO_NET_OFFLINE=true pnpm tauri build --debug --bundles app` completed. The reused debug bundle initially failed resource-signature verification; re-signing it with `codesign --force --deep --sign -` restored a valid ad-hoc signature, verified with `codesign --verify --deep --strict`. This is a local development bundle, not a universal or distribution-qualified installer. No live account or desktop session was used for native QA.

## Deep links and downloads

- macOS `moodledesk://` registration through Info.plist and native Tauri URL events; single-instance plugin registered first, forwarding only matching scheme arguments through the same validator. The managed inbox exists before plugin setup and retains the latest request until the shell is ready.
- Origin/root matching includes ports and subdirectory installations; ambiguous roots, traversal, user info and credential-bearing query parameters are rejected. Cold restoration carries the requested page as the autologin destination. Reconnect can retry a retained link. Unknown hosts only prefill the origin (subdirectory base URLs require editing).
- Same-origin HTTPS downloads run through WebKit's session. Per-site folder routing, sanitized names, private staging, atomic collision handling, bounded concurrency and progress/finish events are implemented. History stores filename/path/state/time, excludes source/page URLs, and never exposes filesystem paths to the frontend. Account changes/removal clear history, retaining files.
- Recent Downloads view supports Open, Show in Finder and inline risky-file confirmation in both languages. It is reachable through the toolbar, native menu and tray. Unknown types also require confirmation; no file opens automatically. Rust rechecks completion, path containment, symlinks and risky type before invoking the OS opener.
- macOS omits download completion paths, so Rust maps original URLs. Overlapping identical URL requests retain a reservation until restart; this prevents rejected-request callbacks from completing a later download. Failed duplicate callbacks may temporarily mark the original failed before its eventual success. Restart is required to retry an ambiguous URL. Interrupted staging files remain private on disk; rows become failed at next launch. Course routing, cleanup policy, byte progress/cancel/resume and authenticated native QA remain open.

See [links/download QA](links-downloads-qa.md). The core/frontend checks cover policies and filesystem behavior; they do not establish D5, quarantine behavior or the installed-app cold-launch URL matrix.

Native observation for this slice: a separately identified QA bundle initially rendered French/dark onboarding with `https://new.example.edu/` prefilled from a startup argument, without automatic sign-in. The UI tool subsequently displayed a signed-in session after its window binding changed; approval review blocked further native inspection. No course links or account actions were executed. Native Downloads layout/controls, single-instance UI behavior, and the full OS URL-launch matrix remain unverified. Automated safety tests remain valid independently of this UI limitation.

## PDF navigation, Moodle popups and menu refinement

- Persistent Home control in the main toolbar and native Go menu (⌘⇧H), navigating directly to the selected site's dashboard. macOS Back/Forward use WKWebView's native history rather than `history.back()` JavaScript, skipping blank bootstrap and one-use authentication entries; missing main-view history falls back to the dashboard.
- Same-origin (including same-origin blob) Moodle popup windows share the originating WebKit configuration/session, retain their opener/close behavior and leave the parent page intact. Popups have their own local Back/Forward/Home/Reload/Downloads toolbar. Back with no earlier item returns to the originating site; Home returns the parent to the dashboard. At most three popups are accepted per site; removal/reauth closes them. External HTTPS links go to the browser only when they contain no authentication material.
- The local popup toolbar has a separate minimal capability and an exact caller/window binding. Remote `popup-*`, `site-*` and `auth-*` labels match neither local capability. The configuration/caller regression script covers this separation; runtime remote IPC probes remain open.
- The native PDF viewer's own Save action uses WebKit's `_webView:saveDataToFile:suggestedFilename:mimeType:originatingURL:` callback, which Wry does not expose. A macOS delegate proxy handles this callback, forwarding every other UI delegate selector to the original delegate. Validated PDF bytes (max 128 MiB) go through the same account-bound staging/history/collision path and receive quarantine metadata before publication. This is a [WebKit-private compatibility callback](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/UIProcess/API/Cocoa/WKUIDelegatePrivate.h), not a stable public API; regression qualification is required on supported macOS versions. No replacement PDF renderer, credential exposure or network refetch is introduced.
- Download insertion checks the source Moodle user atomically. A late save from an old account cannot populate a newly reconnected account's history.
- Tray site rows use saved abbreviations or bounded labels, with Open/Home actions in a submenu. Recent Downloads contains the five newest filenames and a full-list action; selecting a completed filename only reveals it in Finder. Download events refresh the tray immediately.

Automated verification: 46 UI tests, 34 Rust tests, frontend build/lint/format, workspace Clippy and local-only IPC/caller checks. Native PDF-save, popup history/opener behavior and the refined menu layout remain unverified in a live signed-in session following the UI inspection block noted above.

## Inbox, courses, agenda and Quick Open

- **Inbox.** Lists the cached notification subjects of every organisation (up to 100 each, 300 shown), grouped by day, filterable by organisation and unread state. Mark as Read and Mark All as Read call Moodle first, then update the local row for the same Moodle user, then wake that organisation's poller so the badge, Dock and menu-bar counts come from the server. Only an identifier already in the local cache can be marked. Unread conversations are reported separately per organisation, since the list does not contain them. Open shows the organisation's notifications page; no context URL is stored or followed.
- **Courses.** `core_enrol_get_users_courses` per organisation, at most 500 courses, names reduced to bounded plain text. The cache is shown immediately and refreshed when older than ten minutes or on request; a failing organisation keeps its saved list and shows the reason. Pins are local rows, dropped when a course leaves the list. A course opens only if its identifier is in that account's cache.
- **Agenda.** `core_calendar_get_calendar_upcoming_view`, at most 200 events, replaced as a whole on refresh so changed and cancelled events reconcile. Only id, name, start, duration, a reduced kind and the course are kept; descriptions and URLs are dropped. Entries stay listed for an hour after they end. Times use the Mac's time zone, which the view states. No submission or completion state is shown, and there are no reminders.
- **Quick Open.** ⌘L (Go menu), the toolbar magnifier, or ⌘K in the browser preview. Searches course names, organisations and shell views locally. Rust hides the Moodle view while the panel is open; dismissing it switches back to the same organisation without a new autologin.
- **Downloads.** 25 rows per page with Show More, filename search (wildcards are literal), organisation and state filters, Copy Path through `/usr/bin/pbcopy` from Rust, and Clear History (all or one organisation) that never deletes files or rows still downloading. Opening and revealing now look a download up by id instead of in the 20 newest rows.
- **Account isolation.** Schema v5 tables cascade on removal and are purged with the existing tables when a different Moodle user reconnects. `save_courses`/`save_events` check the requesting user inside the transaction, so a response that arrives after an account was replaced is discarded.
- **Limits.** Courses and Agenda refresh only while their view (or Quick Open) is open; nothing is fetched in the background. The poller still fetches the newest 20 notifications. The agenda horizon is whatever Moodle's upcoming view returns for the user's calendar preferences.

Automated verification: 70 UI tests and 42 Rust tests (including contract tests for the three new web-service calls with stock-shaped synthetic responses, store tests for account binding, paging and clearing, and view tests in both languages), frontend build/lint/format, workspace Clippy and the IPC/caller check. The interface was exercised in the browser preview with sample data in light and dark. No native window, tray, menu, pasteboard or live Moodle behaviour was verified in this slice.

## Course folders and native document reader — 10 November 2026

Built: metadata-backed course-folder routing, explicit Refresh Course Folders, download course labels/filter with Unknown Course, and schema v6 migration. New supported downloads can be previewed in Rust-owned native Tauri windows: PDFKit for PDF and Quick Look for DOCX/XLSX/PPTX. The native toolbar includes zoom, fit, scale readout, default-app Open, Finder and full screen; PDF additionally has previous/next page and page count. No document bytes or paths cross IPC. The native smoke harness exercises the actual toolbar actions independently of the user's account data.

71 UI tests and 49 Rust tests pass, with frontend build/lint/format, workspace Clippy, capability checks and a local debug app bundle. Native PDF/Office toolbar smoke checks pass and PowerPoint rendering/toolbar were observed. Real Moodle course routing, installed-app lifecycle, complex Office fidelity and the supported-platform matrix remain open in [native reader/download QA](native-reader-downloads-qa.md). Existing PDF Save/download qualification is not closed by this work.

The AI assistant is now **A6**, the final Alpha feature after A1–A5 and before combined qualification/release; it remains unimplemented.
