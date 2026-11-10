# Alpha implementation status

Use [the working plan](plan.md) to track completed tasks and next priorities.

## Expanded Alpha features — first pass built, not validated

At the product owner's request, the [Alpha scope amendment](alpha-feature-expansion.md) brings forward: A1 notification inbox and richer download controls; A2 course browser/pins and quick-open; A3 selected files saved offline with manual refresh; A4 upcoming agenda and opt-in local reminders; A5 manual ICS snapshot export. The original full background-sync/live-calendar/upload features are not implied by these smaller slices. The original six-week estimate requires re-estimation; existing release/security gates remain open.

A first pass over A1, A2 and the read-only A4 agenda is implemented (see the last section). A3 selected offline files and A5 export are now implemented against fixtures (see below); local reminders and the A6 assistant are implemented against fixtures (see the later sections). [plan.md](plan.md) lists what is checked and what is still open.

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

## Tray unread items and cached activities — 10 November 2026

- Organisation tray submenus list up to five newest unread notification subjects, with bounded Unicode-safe labels. Selecting a subject checks the current cache and opens that organisation's fixed notifications page; it neither marks the item read nor follows a server-provided URL.
- Refresh Activities in Courses (or Refresh Course Folders in Downloads) explicitly indexes course-module identifiers, bounded plain-text names and module types. Schema v7 stores these per account/course without descriptions, resource URLs or credentials. Successful course refresh prunes removed courses' activities; successful activity refresh replaces that course's activities. Account replacement/removal clears the cache and late old-account writes are refused.
- Quick Open searches cached activity names as well as courses, organisations and shell views. It states the oldest metadata refresh and that opening Moodle pages requires a connection. Activity navigation uses only a cached identifier and a validated module name to construct an internal `mod/<module>/view.php?id=<id>` route. Labels without an activity page are excluded. Indexing is explicit; no automatic all-course fetch runs when opening Quick Open.
- Failed downloads offer Return to Moodle, using a still-cached course or the organisation. Copy explicitly says the transfer is not restarted. No stale source URL is replayed.

Verification: 73 frontend tests and 52 Rust tests pass, along with frontend lint/format/build, Rust formatting, workspace Clippy with warnings denied and the shell-only IPC guard. Rust HTTP-contract tests require localhost binding and passed outside the restricted sandbox. Tests cover stock-shaped activity metadata, navigation input rejection, stale-account write rejection, course reconciliation, tray filtering/order/bounds and the failed-download controls in both languages. A local debug `MoodBuro.app` bundle also builds successfully. Live Moodle activity links, installed-app tray selection and keyboard/VoiceOver qualification remain open. The global shortcut, PDF/image preference, A3, reminders, ICS and A6 remain unfinished.

### File handling, Save to Downloads and the global shortcut — 10 November 2026

Closes the A1 and A2 checklists as built-against-fixtures; A3, reminders and A5 are next, A6 last.

- **PDF/image preference.** Settings → Files has one choice per type: View in Moodle (default), Save, Save and Open. The navigation-delegate proxy that already handles embedded frames now also decides top-level responses with the public `WKNavigationResponsePolicy::Download`, so WebKit hands the transfer to the existing download delegate and `downloads::handle_for_user` remains the only writer. The decision is a pure function (`md_moodle::response_downloads`): the organisation's own HTTPS origin only, PDF and PNG/JPEG/GIF/WebP only, never frames or other origins.
- **Defaults unchanged.** With both choices on View in Moodle every response is forwarded untouched, including forced downloads of types WebKit can display, which still render as before. Saving those consistently is left as a separate decision.
- **Save to Downloads (⌘S, File menu).** The public-API fallback for the PDFKit Save adapter: when the view shows a credential-free `pluginfile.php` file of this installation, it is requested again and that one response is saved. Anything else is refused with a message and no navigation.
- **Open after save.** Only a transfer started by the "Save and Open" choice is opened, after the published path is revalidated and only for PDF/raster-image extensions. A download link the user clicked, the Save command and every other type are saved and nothing more.
- **Global shortcut.** `tauri-plugin-global-shortcut` (pinned) is used from Rust only; no webview has a plugin permission and the combination is fixed. `settings_set` registers before saving and rolls back on failure; a failure at launch never blocks start-up and is reported by `settings_get`. The shortcut shows the window and takes the ⌘L path.
- **Settings plumbing.** `md_store::Settings` gained `pdf`, `images` and `globalShortcut` (existing `settings` table, no schema change); `settings_set` now restores every field on failure.

Evidence: 79 frontend tests, 55 Rust tests, frontend lint/format/build, Rust formatting, workspace Clippy and the IPC guard pass.

**Not viewed:** unlike earlier slices, the new Settings rows were not looked at in the browser preview; their layout was only checked against the CSS (single-line hints, 640 pt column).

**Not verified natively:** none of this was run in the app. Open points: that WebKit keeps the current page when a top-level response becomes a download; that Moodle's direct file links arrive as top-level responses with the expected MIME type (4.5 and 5.3 rigs); a `target=_blank` file leaving an empty popup window after it is saved; open-after-save timing; and whether ⌘⇧M registration fails or silently loses to an app that already holds it. These are listed in [plan.md](plan.md) §4.

## Organisation ordering, calendar export and new icon — 10 November 2026

- Settings provides labelled Up/Down actions for organisations. SQLite saves relative moves atomically; the sidebar and menu bar use that order. Reconnect preserves it and new sites append. A disk-backed test covers restart, boundary moves, removal and insertion.
- Agenda adds inclusive local date filters and manual ICS export for the selected site/range. Rust generates the snapshot from account-bound cached metadata, with stable UIDs, UTC dates, escaped text, UTF-8-safe line folding and fixed Moodle calendar-page URLs. The file publishes without overwriting in Downloads; Finder is asked to reveal it. FR/EN copy explains manual import, snapshot staleness, repeat-import duplicates and reveal failures.
- The export fixture is checked byte-for-byte by Rust, then independently parsed with `vobject` 0.9.9 using `scripts/check-calendar.py` (install that package in a temporary virtual environment). It verifies accented/multiline text, injected-property escaping, identity, fixed links and the Europe/Paris autumn DST boundary. Apple Calendar import remains unverified.
- `assets/Newicon.png` is used for the macOS bundle and desktop icon formats, plus onboarding and both READMEs. The original landscape asset is retained; generated assets use Tauri’s centred square cover conversion.
- Checks: 82 UI tests, 59 Rust tests, frontend build/lint/format, Clippy and cargo format, local shell/popup IPC guard, independent ICS parsing. A3, reminders, A6 and existing live/native/platform gates remain open.

The first parser attempted (`icalendar` 6.3.2 on system Python 3.9) decoded an escaped literal backslash before `n` as a newline. `vobject` 0.9.9 preserves the literal backslash and passes the complete fixture and round-trip checks; the export follows RFC 5545 escaping. This parser difference reinforces the open Apple Calendar import gate.

A debug macOS app was bundled at `target/debug/bundle/macos/MoodBuro.app`, then locally ad-hoc signed after bundling; `codesign --verify --deep --strict` passed. The embedded `icon.icns` matches the generated new asset. This is a local development bundle, not a distribution release.

## Selected offline course files — A3, 10 November 2026

Courses now offers Files per course, with separate local saved-copy and explicitly fetched remote-resource lists. Selection shows known/unknown sizes; serial saves and manual refresh return per-file outcomes and byte progress. Saved copies use the existing guarded native Open/Finder and risky-file confirmation conventions. FR/EN copy states transfer limits, manual one-way behaviour, local edits, account cleanup and unsupported resources.

The new Rust path accepts only same-installation HTTPS web-service file URLs, refetches descriptors before saving, refuses redirects and common login/error MIME types, limits bytes/time, checks free space and publishes quarantined files without replacement into safe course/section folders. SHA-256 reuse requires an intact latest indexed local snapshot. Renames, remote revisions and local edits cannot overwrite an existing file. SQLite schema 8 isolates offline records from download history; account replacement/removal clears the index but keeps files.

Evidence: 87 UI tests; 66 Rust tests covering file endpoint policy, metadata contracts, redirect/access/size failures, no-overwrite publication, local edits/moves, symlink components, persistent indexing and account cleanup; lint/format/build, Clippy and shell-capability checks. Native macOS quarantine/Open/Finder, real-account downloading and full restart-without-network validation remain open. See [offline qualification and limits](offline-files-qa.md). A4 reminders and A6 are still unimplemented.

The local development bundle at `target/debug/bundle/macos/MoodBuro.app` was rebuilt for A3 with the new icon retained, ad-hoc signed after bundling, and verified with `codesign --verify --deep --strict`. It was not launched against a real account during this slice.

## Local deadline reminders — A4, 10 November 2026

Agenda offers default-off per-account 24-hour and 1-hour reminders. Background agenda refresh and a local 15-second scheduler reconcile normalized deadlines, conservative action eligibility and recent-cache requirements. Schema 9 persists preferences, event observation times and account/event/due/offset claims. Claims precede native attempts, preventing replay across restart; missed startup/wake windows and past/newly imported triggers are skipped. Changed dates, deletions, old-account responses and older in-flight requests cannot restore stale schedules. A pre-sleep request does not qualify as a post-wake snapshot.

The shell reports stale agenda data, sanitized refresh errors and known failed attempts. Copy explicitly states that macOS permissions/Focus and actual notification display cannot be verified by the desktop notification plugin. Closing the window keeps the scheduler alive; quitting stops it. Native attempts are capped at five individual alerts plus one summary; clicks open the local Agenda.

Checks: 92 UI tests, 73 Rust tests, frontend build/lint/format, workspace Clippy/format and IPC boundary checks. Fixtures cover persistent deduplication, schema migration, account changes, deadlines moved/deleted/unavailable, wake/clock gaps, out-of-order responses and DST timestamps. [Real-site and installed-app reminder qualification](reminders-qa.md) remains pending. A6 is the next unimplemented feature slice; existing Alpha release gates remain open.

The local development bundle `target/debug/bundle/macos/MoodBuro.app` was rebuilt with reminders, ad-hoc signed after bundling and verified with `codesign --verify --deep --strict`. Its icon matches the existing generated new icon. No real-account refresh or native reminder delivery was triggered during implementation.


## A6 AI assistant — 10 November 2026

Implemented the final Alpha feature slice: Albert/Mistral BYOK settings and model discovery, Rust-only streamed requests, cancel/retry, session-only account-scoped history and deletion, explicit provider-specific context consent, and a collapsible/resizable bilingual panel beside the native Moodle view. Keys use separate OS credential-store entries; non-secret preferences use namespaced SQLite settings.

Six typed tools cover course/activity/deadline/notification lookup, fixed-route navigation and individually confirmed notification-read writes. Model arguments cannot supply a URL or service name. Citations resolve only against metadata actually attached/looked up. Account replacement/removal cancels work; provider changes and consent withdrawal clear conversations. Detached requests omit earlier Moodle-derived answers, including orphaned answers at a bounded-history boundary. Stop prevents subsequent tool execution; both approval and account validity are rechecked before a write. Mistral model capabilities take precedence over its base/fine-tuned type label, and embedding-only lists are not offered as chat models.

Checks: 100 frontend tests and 95 Rust tests pass, along with frontend lint/format/build, workspace Clippy with warnings denied, Rust formatting and shell-only IPC guards. Rust HTTP tests ran with localhost mock-server access. The local debug `MoodBuro.app` bundle builds; its reused signature was replaced with an ad-hoc signature and passes `codesign --verify --deep --strict`. Verification and limits are recorded in [assistant qualification](assistant-qa.md). Provider endpoints/protocols were checked against official references and tested with local HTTP fixtures. Live provider credentials, real-site tool behavior, native panel/focus/VoiceOver, Keychain lifecycle and runtime IPC probes remain unqualified; the Alpha release gate remains open.


## First tester prerelease — 10 November 2026

Prepared `v0.1.0-alpha.1` with aligned package versions, a universal Apple Silicon/Intel release-mode DMG, an explicit ad-hoc signing overlay, checksums, release notes and an installation/Gatekeeper guide. Added a manual GitHub Actions packaging workflow; CI's portable-core checks now include `md-assistant`. The release build leaves host compiler plugins unstripped to avoid the pinned Rust toolchain's LLVM/Mach-O alignment issue on macOS 27. Existing live/native acceptance gates remain open. Distribution is a private GitHub prerelease, with manual upgrades and no notarization or automatic updater.

Artifact verification: both executable slices report macOS 12.0 minimum and version `0.1.0-alpha.1`. `codesign --verify --deep --strict` passes on the built app and the copy mounted read-only from the DMG; `hdiutil verify` passes, and the image contains the Applications installation link. DMG SHA-256: `46b9f30e3e798e3925487bfae7660012be0aaa98f66f66b0178b21293b1b2c6a`. This verifies packaging, not a clean-machine install or Intel execution.


## UI and macOS fixes — alpha.2, 10 November 2026

Fixed Settings grid min-content expansion and reflowed long labels/controls, including when the assistant is open at the minimum window size. Agenda now prioritizes events, offers date filters on demand and links to reminder preferences in Settings. Secondary explanations in the list views and Settings use native disclosure controls. Download status events carry only filename, state and account identifiers to the shell; progress/completion/failure is visible in the toolbar over Moodle until acknowledged. Completed download rows use a success checkmark, and the storage note explains `~/Moodle` plus Finder access and the internal folder identifiers without moving existing files. Provider defaults now use the requested DeepSeek Albert model and Mistral Large 4, preserving explicit saved models.

Native Icon Composer source and its compiled `Assets.car` are packaged with `CFBundleIconName=MoodBuro` for macOS 26+, with the legacy `.icns` kept for older systems. The compiled asset is checked in because the Tauri/actool subprocess path fails on Xcode 27; it was generated and inspected using Apple's compiler. Main-window size/position/maximized state is persisted atomically in the app config directory. Restoration uses the available monitor work area and ignores transient minimized/fullscreen bounds. The isolated `window-state-smoke` app saved a 1040×680 window and a second process restored it successfully, without accessing MoodBuro accounts.

Checks: 101 frontend tests and 97 Rust tests pass, as do lint, formatting, production frontend build, workspace Clippy and shell-only IPC guards. WebKit review covered Settings, Agenda, Downloads, Inbox, Courses, offline files, onboarding, Quick Open and the assistant; FR/EN, narrow windows, dark/Increase Contrast and Reduce Motion were checked. Main-view and Settings scroll widths matched client widths, including 348px of remaining content beside the open panel. Download feedback rendering is covered in all three states and localization; real authenticated transfer/notification and full VoiceOver qualification remain open. The macOS Actions jobs now use the supported `macos-26` image in place of retired `macos-14`.

The alpha.2 universal DMG passes `hdiutil verify`; the app mounted from it passes strict deep codesign verification, has x86_64/arm64 slices, and includes the exact native icon asset with `CFBundleIconName=MoodBuro`. macOS system icon rendering was inspected without launching the user account. Installer SHA-256: `2cec2bbcfd506b4476be4a46c459e66ae2c9bda3ca7646c850917c047b0256b1`.
