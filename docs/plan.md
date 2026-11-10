# MoodBuro — implementation tracker

Last updated: 10 November 2026.

This is the working checklist for project advancement. The [PRD](prd.md) defines the product, and the [Alpha implementation plan](alpha-plan.md) supplies the baseline scope and sequence. The [expanded Alpha feature scope](alpha-feature-expansion.md) promotes bounded user-facing slices from later milestones. Work has landed across parts of W1–W3; that does not mean those weeks or their validation gates are complete.

**How to maintain this file:** check an implementation item when it is built and its relevant checks pass. Keep live/platform validation as separate unchecked items until evidence exists. After each implementation slice, update the completed work, next priorities, verification evidence and any decisions or blockers.

## Current position

| Milestone | Status | Remaining gate |
| --- | --- | --- |
| M0 — architecture and sign-in spike | In progress | Repeatable three-site login/SSO tests, platform/security validation and D1–D5 decisions |
| M1 — Alpha core shell | Partially implemented | Notification platform qualification, remaining multi-site controls, course download routing and installed-app validation |
| Expanded Alpha features — A1–A5 | A1, A2, the read-only A4 agenda and A5 export implemented against fixtures; A3 and reminders not started | Live/native validation of the built parts; selected offline files, reminders and native Calendar import qualification |
| A6 — AI assistant | Planned, not started; final Alpha feature | Both BYOK providers, explicit context sharing, bounded tools and isolation checks |
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
- [x] Enforce origin-based navigation policy; capture managed downloads and isolate controlled same-origin popups behind separate local toolbar permissions.
- [x] Add unit/contract tests for callback and URL policies, HTTP behavior, metadata/settings/lifecycle and localization.
- [x] Pass the latest local frontend build, lint/format checks, **82 UI tests**, **59 Rust tests**, workspace Clippy and shell-capability regression checks.
- [x] Build a local ad-hoc-signed macOS app bundle; this is not a notarized distribution release.
- [x] Review available dev logs and metadata schema without exposing credentials; record findings in the [auth review](auth-review.md).

## Next steps — in priority order

**Implementation priority:** finish the current PDF/popup escape paths, then implement A1–A5 below, followed by the A6 AI panel. Run the M0/native validation tasks alongside this work; unavailable account/platform intervention is a deferred check, not a reason to stop unrelated feature implementation. The original six-week baseline no longer estimates the expanded scope; re-estimate each feature slice before promising a release date.

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

- [x] Add persistent organisation ordering from Settings with keyboard-accessible Up/Down buttons; sidebar and tray follow the saved order, and new sites append.
- [x] Add typed popup notification and unread-conversation count wrappers plus a mark-notification-read API (not yet exposed in the shell).
- [x] Implement cancellable per-site polling, silent first-poll seeding, restart deduplication and capped backoff.
- [ ] Verify sleep/wake and add explicit OS lifecycle hooks if needed (relative sleeps currently avoid catch-up bursts).
- [x] Persist notification checkpoints/counts and a bounded subject/read-state cache; emit unread/status updates to the shell.
- [x] Wire native notifications (five plus a summary per poll), aggregate dock badge and menu-bar unread state; installed-app delivery remains a QA gate.
- [x] Implement tray open/settings/quit/check-now controls and per-site counts/status; wire notification clicks to the saved site’s notifications page.
- [ ] Qualify notification click delivery (D4), tray behavior (including the new Inbox entry) and the latest-unread menu on installed macOS builds.
- [x] Hide the main window on close, retain background polling, and reopen through the tray or Dock.
- [ ] Add menu-bar-only activation preference and window-state persistence.
- [ ] Verify resource usage, accessibility and three-site behavior with the background services running.

### 3. Add deep links and safe download capture

- [x] Register `moodledesk://` on macOS and single-instance argument forwarding; queue validated links until the shell is ready and route to the accepted installation root.
- [x] Prefill HTTPS onboarding for unknown deep-link origins without starting sign-in automatically.
- [ ] Validate WKWebView download destination/finish behavior (D5).
- [x] Capture same-origin authenticated downloads into a per-site Downloads folder, sanitize names and publish atomically without overwriting collisions.
- [x] Add metadata-backed module-context mapping and course-folder routing, refreshed explicitly from Downloads; unmatched files remain in Downloads.
- [x] Migrate download history, mark interrupted transfers failed on restart, and expose recent downloads with Open/Show in Finder from the toolbar, native menu and tray.
- [x] Require inline confirmation for risky/unknown file types, revalidate paths in Rust and never auto-open downloads. One opt-in exception since 10 November 2026: a PDF or raster image saved by the "Save and Open" file-handling choice opens after its path is revalidated; every other download still never opens by itself.
- [ ] Verify installed-app deep links and session-authenticated downloads across the test rigs.

### 4. Build the expanded user-facing Alpha (planned A1–A5)

The [feature amendment](alpha-feature-expansion.md) defines scope, dependencies and acceptance criteria. The original bullets were split on 9 November 2026 so that built parts can be checked without implying the rest. A checked item here means implemented with passing automated checks against synthetic data; live-site and installed-app validation is tracked separately at the end of this section. Partial PRD coverage must not be reported as the full v1.0 feature.

- [x] Add a public-API save fallback: File › Save to Downloads (⌘S) requests the shown Moodle file again and saves that response through managed downloads. It covers `pluginfile.php` addresses without credentials; blob documents still rely on the PDFKit adapter.
- [ ] Stabilize PDF saving, native history and popup parent return in real sessions (native validation; see the follow-up section).
- [x] **A1:** notification inbox across sites (day groups, organisation and unread filters) with safe Open, Mark as Read and Mark All as Read; the server is asked first and the poller then reconciles badge, Dock and menu-bar counts.
- [x] **A1:** state unread conversations separately from notifications, with a fixed link to Moodle's Messages page.
- [x] **A1:** latest unread notifications per organisation in bounded tray submenus; selecting an item opens the fixed notifications page.
- [x] **A1:** paginated/searchable download history with site/state filters, Copy Path (pasteboard written from Rust) and confirmed Clear History that keeps files and active transfers.
- [x] **A1:** course filter with an Unknown Course bucket and course labels in download history.
- [x] **A1:** shared Rust-owned native reader for downloaded PDF, DOCX, XLSX and PPTX files on macOS, with a separate Preview action and the existing default-app Open action.
- [x] **A1:** failed-download Return to Moodle fallback: opens a still-cached course or the organisation, with explicit copy that the transfer is not restarted.
- [x] **A1:** PDF/image handling preference per type (View in Moodle by default, Save, Save and Open), decided on top-level responses and routed through managed downloads. With both choices at their default no response is handled differently from before. Only a transfer the preference started may open afterwards, limited to PDFs and raster images.
- [x] **A2:** account-bound course cache and browser with search and local pins, refresh/stale/error states per organisation, and rejection of late responses for a replaced account.
- [x] **A2:** metadata-backed course/context mapping and course-folder routing with an explicit Refresh Course Folders action. Live-site qualification remains open.
- [x] **A2:** ⌘L Quick Open over saved course names, organisations and shell views, from the Go menu, the toolbar and the keyboard.
- [x] **A2:** explicitly refresh/cache course activities and search them in Quick Open, with oldest metadata refresh and connection limits stated.
- [x] **A2:** opt-in global ⌘⇧M for Quick Open, registered from Rust only, saved only after a successful registration, reported in Settings when registration fails at launch, released when disabled or on quit. Cross-app conflicts that macOS does not report are a native QA item.
- [ ] **A3:** choose course files to save offline, with known sizes, bounded transfers and safe course/section folders.
- [ ] **A3:** browse/open saved files offline and manually refresh selected files without overwriting local edits or propagating server deletions.
- [x] **A4:** read-only upcoming agenda across sites, grouped by day, with fixed Moodle calendar links, the Mac's time zone stated and last-refresh/stale state.
- [ ] **A4:** opt-in local due-date reminders, deduplication and rescheduling after event changes/wake; no delivery promise while quit.
- [x] **A5:** manually export the selected site/date range to a token-free ICS snapshot in Downloads, with UTC times, escaped/folded text, stable account-scoped UIDs and fixed calendar links. Rust fixtures and an independent ICS parser pass.
- [ ] **A5 native qualification:** import the snapshot in Apple Calendar, check DST/text and repeat-import behaviour; qualify Finder reveal and saved site ordering in the installed app. Live subscriptions remain later.
- [ ] Validate the built A1/A2/A4 parts against a real account and the 4.5/5.3 rigs: `core_enrol_get_users_courses`, `core_calendar_get_calendar_upcoming_view`, `core_message_mark_notification_read` / `mark_all_notifications_as_read`, access-denied responses, and count reconciliation after Mark as Read.
- [ ] Check natively with `pnpm tauri dev`: the ⌘1/⌘2/⌘3/⌘L/⌥⌘L menu commands, the tray Inbox entry, hiding and restoring the Moodle view around Quick Open, keyboard focus landing in its field when ⌘L is pressed from a Moodle page, warm and cold opening of a course/calendar/notifications page, Copy Path, and the new motion under Reduce Motion and Increase Contrast.
- [ ] Check the 10 November additions natively: a PDF and an image opened directly with each handling choice (main view and popup, including the blank popup a saved file leaves behind), a forced-download link (unchanged behaviour expected), Save to Downloads on an inline PDF/image and on an ordinary page, open-after-save, embedded PDFs staying in the page, ⌘⇧M from another app with the window hidden/minimised/on another Space, a combination already used by another app, and release on quit.
- [ ] Verify the combined A1–A5 acceptance cases, access-denied behavior, account replacement/cleanup, FR/EN and keyboard paths.

### 5. AI chat side panel — final Alpha feature (A6)

Added on 10 November 2026. Plan an optional AI assistant beside the Moodle view, able to answer general questions and interact with Moodle elements exposed to the app. At the product owner’s request, implement this as **A6, after A1–A5 and before combined Alpha qualification and release**. Estimate it after the provider/API spike; it is part of the Alpha feature gate. See the [PRD scope amendment](prd.md#ai-assistant-scope-amendment--10-november-2026).

- [ ] **Provider setup and general chat:** support the Albert API or Mistral AI in bring-your-own-key (BYOK) mode, with provider/model settings, connection validation, streaming responses, cancel/retry and clear authentication, quota and network errors. Validate each provider's endpoint, model availability and tool-calling support during the spike.
- [ ] **Side panel:** add a collapsible, resizable chat panel alongside the active Moodle view, with keyboard access, FR/EN copy and accessible focus handling. Recalculate native Moodle webview bounds when the panel opens, closes or resizes.
- [ ] **Moodle context:** let the user explicitly attach an organisation/course or supported item. Start with courses, exposed activities/resources, upcoming deadlines and notifications available through the app's Moodle API/cache. Show the selected context and link answers back to Moodle sources; state missing permissions or stale data. General questions must work without Moodle context.
- [ ] **Bounded interactions:** expose an allowlist of typed tools through the Rust core, starting with course/activity lookup, deadline lookup and opening a referenced Moodle item. Add selected write actions, such as marking a notification read, only where the site exposes the capability; show the exact target and require user confirmation before each write. Keep submissions, grades and arbitrary page/script execution outside the initial slice.
- [ ] **Keys and data controls:** keep provider keys in the OS credential store and provider requests in Rust. Explain which prompt/context is sent to the selected provider before enabling Moodle context sharing; never send Moodle tokens, cookies or provider keys as model context. Make chat-history retention explicit and provide deletion controls.
- [ ] **Isolation and tool safety:** scope context, conversations and tool calls to the selected Moodle account; clear attached context on account switches and invalidate pending tools on account replacement/removal. Treat Moodle content and model output as untrusted, validate tool arguments and permissions in Rust, and keep remote Moodle webviews without privileged IPC.
- [ ] **Acceptance gate:** verify both providers, general chat without context, source-linked Moodle answers, permitted lookups/navigation, denied capabilities, confirmed/cancelled writes, prompt-injection resistance, account isolation, credential/log redaction, panel geometry and keyboard access. Use provider/Moodle fixtures plus live qualification before claiming support.

### 6. Prepare the tester Alpha

- [ ] Configure a stable self-signed Alpha identity, updater signing keys and private HTTPS hosting with the product owner.
- [ ] Implement the universal Apple Silicon/Intel DMG release workflow and verify architectures/signatures/checksums.
- [ ] Implement the signed Alpha updater channel and test an actual update between builds.
- [ ] Add rotating/redacted diagnostic logging and configure Rust advisory auditing.
- [ ] Review and resolve outstanding dependency alerts, then enforce the agreed audit checks in CI.
- [ ] Write and verify the Gatekeeper installation/tester guide.
- [ ] Run the full QA matrix, security checks, accessibility pass and bug bash.
- [ ] Review every exit criterion in the original Alpha plan; resolve P0s before releasing `0.1.0-alpha.1` to internal testers.

## Deferred beyond the Alpha

The bounded A1–A6 versions above are Alpha targets; the AI panel remains unimplemented. The following larger versions remain deferred:

- [ ] Full scheduled course-folder sync, resumable mirroring, automatic server-deletion handling and full sync/download de-duplication.
- [ ] Live authenticated calendar subscriptions and automatic external-calendar refresh; the Alpha gets agenda/reminders and manual ICS export only.
- [ ] Uploads, drag-and-drop submissions and Finder Services/share integration.
- [ ] File-content search and OS search extensions beyond ordinary indexing of saved files.
- [ ] Download pause/resume, automatic retention/deletion and expanded Finder tagging/metadata.
- [ ] Developer ID signing/notarization, managed deployment and Windows/Linux releases.


New downloads with an indexed Moodle module context go into a course folder; unmatched files remain in the per-site Downloads folder. Existing files are not moved. Selected offline files and native transfer qualification remain open. Closing the window hides it and keeps polling active; Quit ends the process.

## Evidence and related documents

- [Implementation status](implementation-status.md): verification detail and current limitations.
- [Authentication review](auth-review.md): inspected evidence and remaining security checks.
- [UI design](ui-design.md): shell, titlebar, localization and label decisions.
- [Architecture decisions](adr/0001-alpha-foundation.md): technical choices and corrections.
- [Alpha plan](alpha-plan.md): baseline and amended scope, spike definitions, QA matrix and release gate.
- [Expanded Alpha features](alpha-feature-expansion.md): A1–A5 boundaries, delivery order and acceptance criteria.

## Recent delivery checkpoints

| Commit | Delivered |
| --- | --- |
| `88adb27` | Workspace, shell and secure-sign-in foundation |
| `76a73f7` | Bilingual macOS shell, session/account lifecycle and READMEs |
| `4413c62` | Titlebar repair, organisation abbreviations and auth diagnostics/safeguards |

## Background notification delivery slice

Implemented on 9 November 2026. One cancellable worker per saved account polls every 120 seconds, backs off to 15 minutes, and stops for missing/invalid tokens. Check Now coalesces requests. New items are checked even when the unread total is unchanged. SQLite schema v3 migrates existing sites, stores metadata without credentials/context URLs, caps history at 100 rows per site, and deletes notification state with the account. Reconnect preserves checkpoints for the same account and resets them for a different Moodle user. The old worker is invalidated before credential changes.

Verification: 43 UI tests, 24 Rust tests, frontend production build, lint/format, workspace Clippy and shell-only IPC regression checks pass. HTTP contracts match the published Moodle 4.5 API but remain synthetic. [Background QA](background-qa.md) records the required live/installed-app tests. M0 and release gates remain open; no credentials, certificates or hosting setup were requested.

## Deep links and download capture slice

Implemented on 9 November 2026. Native macOS URL events and single-instance CLI forwarding share a bounded Rust inbox. Deep links preserve query/fragment, select the longest saved installation root, reject credentials/traversal/credential-bearing parameters and keep the target inside the autologin exchange. Unknown hosts prefill their HTTPS origin for user review. Failed restoration retains the destination for reconnect. No raw link enters logs or SQLite.

Downloads use WebKit's authenticated session and a private staging directory within `~/Moodle/<site-name>-<uuid>/Downloads/`. Completion URLs map to staged files because macOS omits the completion path. Atomic hard-link publication avoids overwrites and preserves the downloaded inode. Same-URL overlaps are conservatively reserved until restart to avoid mistaking a rejected request's late callback for a later transfer. History excludes source/page URLs and frontend payloads exclude local paths. Schema v4 handles interruption and account cleanup; downloaded files remain on disk after removal.

Verification: 46 UI tests and 34 Rust tests, frontend production build/lint/format, workspace Clippy and IPC checks pass. [Deep-link/download QA](links-downloads-qa.md) lists native checks, limitations and remaining D5 qualification. No real-account SSO or authenticated-transfer gate is closed by these checks.

## Follow-up: PDF escape paths and usable popups

- [x] Add a persistent Moodle Home button and matching Go-menu shortcut.
- [x] Replace macOS JavaScript Back/Forward with native WKWebView history, with a dashboard escape path.
- [x] Preserve the parent Moodle page for same-origin popups and provide a local toolbar with history, parent return, Home and Downloads.
- [x] Route PDFKit Save bytes through managed download history using a bounded macOS delegate compatibility adapter.
- [x] Shorten menu-bar site rows and group site actions/recent files in native submenus.
- [ ] Validate native PDF Save, popup/opener/close/history, blob PDFs and menu sizing in real sessions on the supported macOS matrix. The PDF save-data callback is a WebKit-private API and remains an explicit compatibility gate.

## Inbox, courses, agenda and motion slice

Implemented on 9 November 2026, at the product owner's choice of "A1 + A2 + agenda (A4)", sidebar icons for the new destinations and "native and fluid" motion.

- **Backend.** Moodle wrappers for enrolled courses, the upcoming calendar view and mark-all-read; a `Page` type that builds the only Moodle pages the shell may open; plain-text reduction of server names. SQLite schema v5 adds `courses`, `course_pins`, `events` and `sync_state`, all removed with the account and purged when a different Moodle user reconnects; course and event writes are refused for a replaced user. Download history gains paging, literal filename search, per-site clearing and lookup by id (items beyond the newest 20 can now be opened). Eleven shell-only commands replace `downloads_recent`; the capability/caller regression script passes unchanged.
- **Shell.** Inbox, Courses and Agenda views, a ⌘L Quick Open panel, a rebuilt Downloads view, shared list components, a sliding segmented control and the motion tokens. Menu: Inbox ⌘1, Courses ⌘2, Agenda ⌘3, Downloads ⌥⌘L, Quick Open… ⌘L. The design guidelines' motion, typography, component and pattern sections were revised to match.
- **Scope decisions.** Not built in this pass and left unchecked above: tray list of unread items, failed-download retry, PDF/image preference, download-to-course mapping, cached activities in Quick Open, the global shortcut, reminders, A3 and A5. Agenda items open their calendar day rather than the activity page, because the activity URL comes from the server.
- **Verification.** 70 UI tests and 42 Rust tests, frontend build/lint/format, workspace Clippy and the IPC check pass. The UI was exercised in the browser preview (`?mac&demo`) in light and dark with sample data: view switching, Mark as Read, pinning, filters, Quick Open keyboard use. **Not verified:** anything native (menus, tray, Moodle-view hiding, pasteboard) and every live Moodle call; contract tests use synthetic stock-shaped responses. No acceptance criterion in the amendment is claimed.

## Course-aware downloads and native readers — 10 November 2026

Built: explicit Refresh Course Folders in Downloads, metadata-only `core_course_get_contents` indexing, account-scoped context/course mappings, course-folder routing for new downloads, course labels/filter and Unknown Course fallback. Schema v6 preserves old rows/files and adds nullable course metadata. Existing downloads are not moved. Permissions failures remain visible; unknown contexts never use guessed page/course names.

Downloaded PDF, DOCX, XLSX and PPTX files have a Preview action separate from default-app Open. Rust resolves a completed managed download by ID and creates a Tauri-owned native document window. PDFKit renders PDFs; Quick Look renders Office files. The AppKit toolbar has zoom/fit/readout, default-app Open, Finder and full screen; PDFs also have page navigation and page count. Office navigation stays in the native preview's scrolling surface. Native views have no webview/IPC grants, no document bytes cross IPC, and account session removal closes its previews. At most three previews are open at once.

Verification and remaining live checks are recorded in [native reader/download QA](native-reader-downloads-qa.md). AI chat is now A6, at the end of Alpha feature implementation.

### Tray unread items and activity search — 10 November 2026

Implemented the next A1/A2 batch: latest unread subjects in per-organisation tray submenus; explicit activity metadata refresh in Courses; schema v7 account/course-scoped activity cache; Quick Open activity search with cache-age/connection copy; and a failed-download Return to Moodle fallback. No stale transfer URL is replayed. The tray opens the fixed notifications page without marking read. Removed courses and replaced accounts lose their cached activities; late writes from the previous account fail.

Evidence: 73 frontend tests, 52 Rust tests, frontend lint/format/build, Rust formatting, workspace Clippy, IPC guards and local debug app packaging pass. Native/live-site acceptance remains open; no such gate was marked passed. Next independent implementation items are the PDF/image preference and optional global shortcut, followed by selected offline files, reminders and ICS in the amended delivery order. A6 remains last.

## 10 November 2026 — ordering, calendar snapshots and app icon

- Organisation ordering is transactional and persists across restart/reconnect/removal; Settings disables boundary moves. No new schema migration is needed because `sites.position` already existed.
- A5 exports the filtered cached agenda on explicit request to a unique `MoodBuro-<UUID>.ics` file in Downloads, published atomically without replacement. Finder reveal failure reports that the file was saved. No automatic import or subscription is configured.
- `assets/Newicon.png` supplies the bundle PNG/ICNS/ICO files and the onboarding/README mark through `assets/app-icon.png`. Tauri’s `--fit cover` centres the landscape source into a square, keeping the central icon intact; the original asset is retained.
- Verification: 82 UI tests (also run in Europe/Paris), 59 Rust tests, independent `vobject` 0.9.9 fixture parsing, frontend build/lint/format, workspace Clippy/format and IPC boundary checks. Native Calendar import and installed-app interactions remain unchecked.
- A3 offline file selection/refresh, A4 reminders and A6 AI remain unimplemented. A5 was independently deliverable on the existing agenda model; this does not close the earlier slices or Alpha release gates.

Local native output: `target/debug/bundle/macos/MoodBuro.app` built with the new icon, ad-hoc signed after bundling and verified with `codesign --verify --deep --strict`. No native account interaction or Calendar import was performed.
