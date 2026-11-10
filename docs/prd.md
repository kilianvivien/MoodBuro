# PRD — Tauri Desktop Shell for Moodle

Nov 10, 2026 · @Kilian

## 1. Summary, problem and goals

We will build a Tauri 2 desktop app (working name **Moodle Desk**) whose purpose is to make Moodle part of the operating system: courses appear as a folder in Finder or Explorer, deadlines land in the OS calendar, Moodle notifications arrive as native notifications with a badge on the dock or taskbar, files are dragged from the desktop straight into an assignment, and a global shortcut opens any course in a second. The real Moodle site still renders inside the app window, so nothing of Moodle is re-implemented. Target: a signed, notarized v1.0 on macOS first, then Windows (v1.1) and Linux (v1.2), supporting Moodle 4.5 LTS through 5.3 LTS. The Rust core and shell UI are written cross-platform from day one; only the OS adapters and release work are sequenced.

**Problem.** Moodle HQ [discontinued its Electron-based Moodle Desktop app in May 2021](https://docs.moodle.org/en/Moodle_Desktop) and now points desktop users to the browser or an installed PWA. The PWA gives a window and an icon, but no offline file library, no background notifications when the window is closed, no multi-site switcher and no OS keychain. Running the mobile app on desktop only works on Apple Silicon Macs; Moodle calls it not recommended on Windows and unsupported on Linux.

**Positioning: the modern successor to Moodle Desktop.** Moodle Desktop was an Electron repackaging of the mobile app, with a mobile-shaped UI and no deep OS hooks. Moodle Desk replaces it with three changes: the full Moodle site instead of a reduced mobile UI, a small native Tauri core instead of a bundled Chromium, and first-class OS integration (Finder, Spotlight, Calendar, Notification Center, Keychain) instead of an app that merely runs on the desktop.

| Moodle Desktop (retired May 2021) | Moodle Desk |
| --- | --- |
| View course activities in the mobile app's UI | Full Moodle site in the window, every activity and theme (OS-1) |
| Download materials for offline use, inside the app | Course files synced to `~/Moodle`, searchable by Spotlight, readable by any app (OS-6, OS-7) |
| Attempt quizzes | Online quizzes in the full site; offline attempts not in v1.0 |
| Push notifications | Native notifications, badge, menu-bar extra, due-date reminders (OS-3 to OS-5, OS-9) |
| No file handling beyond the app's own viewer | Download manager that files every download by course (§3.2) |
| Electron, about the size of a browser | Tauri, OS webview, installer under 15 MB |
| Branding through Moodle's paid service | Self-configurable: preset sites, name, icon and feature switches (§4.7) |

**Why Tauri.** Tauri uses the OS webview (WebView2, WKWebView, WebKitGTK) instead of bundling Chromium, so installers stay small and memory use low. Its Rust core and capability system let us keep the Moodle token and file I/O outside the web page, which a plain Electron wrapper does not enforce by default.

**Goals (v1.0)**

1. Moodle feels native: notifications, unread badge, tray or menu-bar icon, autostart and a global quick-open shortcut behave like any first-party OS app.
2. Course files live in the file system: a synced "Moodle" folder per site, browsable and searchable by the OS (Spotlight, Windows Search, GNOME Tracker) and readable offline.
3. Deadlines live in the OS calendar: assignment, quiz and event dates subscribed into Apple Calendar, Outlook or GNOME Calendar.
4. Files go in from the OS: drag and drop onto the app, the dock icon or a submission page; "Open with / Share to Moodle Desk" from the file manager.
5. One sign-in, including SSO (SAML, CAS, OAuth 2, Shibboleth), with the web-service token kept in the OS keychain.
6. Full-fidelity Moodle in the window: every page, activity and theme works as in a desktop browser.
7. Multi-site: up to 10 Moodle accounts side by side, each with its own folder, calendar and badge count.
8. Signed installers and signed auto-updates on all three OSes.

**Non-goals (v1.0)**

- Re-implementing Moodle UI screens natively (that is the mobile app's model, and it costs a full app team).
- Offline quiz attempts, offline forum posting or any offline write with later sync.
- Replacing Safe Exam Browser or any proctoring tool.
- Moodle server-side plugin development; v1.0 must work against a stock site with the mobile web services enabled.
- Mobile builds (Tauri 2 supports them, but the official Moodle app already covers phones).

## 2. Users and key use cases

The primary user is a learner who lives on a laptop and uses Moodle daily; the teacher and the IT administrator are secondary but decide adoption.

| Persona | Context | What they need from OS integration |
| --- | --- | --- |
| Learner (student, trainee, staff in professional training) | 2–6 courses, often several Moodle sites (university + employer), patchy network on the move | Deadlines in their calendar, files in a folder they can search, a ping when grades or feedback arrive |
| Teacher | Manages several courses, uploads many files, answers forum and message threads | Drag-and-drop uploads, unread-message badge, quick switch between courses |
| IT / LMS administrator | Rolls software out to a managed fleet (Intune, Jamf, GPO) | MSI/PKG installers, preconfigured site URL, no extra server plugin, predictable security model |

**Key use cases (v1.0)**

1. **First run:** the user enters a site URL (or it is preset by the admin), signs in through the site's normal or SSO page, and the app creates the course folder and calendar subscription automatically.
2. **Morning check:** the menu-bar or tray icon shows a badge of 3; one click lists two new grades and one forum reply; clicking an item opens that page in the app window.
3. **Offline study:** on a train, the learner opens the course folder in Finder, finds last week's PDF through Spotlight, and reads it with no network.
4. **Submission:** the learner drags a .docx from the desktop onto the dock icon, picks the course and assignment from a short list, and the file lands in the submission draft; the app then opens the assignment page to confirm.
5. **Deadline:** an assignment due Friday appears in the OS calendar on Monday with the site's own reminder, and a native notification fires 24 h before.
6. **Quick open:** a global shortcut (default Ctrl/Cmd+Shift+M) opens a search box across all sites' courses and activities; Enter opens the match.
7. **Deep link:** a `moodledesk://` link in an email or Teams message opens the right site and page in the app instead of the browser.

## 3. Functional requirements by OS surface

Every requirement maps one OS surface to one Moodle capability that already exists in stock Moodle's mobile web services, so v1.0 needs no server plugin. P0 = v1.0 blocker, P1 = v1.0 target, P2 = later.

| ID | OS surface | Requirement | Pri. | Moodle side | Tauri side |
| --- | --- | --- | --- | --- | --- |
| OS-1 | App window | Render the live Moodle site with session cookies; back/forward, zoom, print, find-in-page | P0 | Normal web pages | Main webview on the site URL |
| OS-2 | Keychain | Store one token per site in macOS Keychain, Windows Credential Manager or Secret Service; never on disk in clear | P0 | `login/token.php` or `admin/tool/mobile/launch.php` | `keyring` crate in Rust |
| OS-3 | Notifications | Native notification for each new Moodle notification and message; click opens the target page | P0 | `message_popup_get_popup_notifications`, `core_message_get_unread_conversations_count` | Rust poller + notification plugin |
| OS-4 | Badge | Unread count on dock (macOS), taskbar overlay (Windows), launcher (Linux where supported) | P0 | Same as OS-3 | Window badge / overlay icon APIs |
| OS-5 | Tray / menu bar | Icon with unread list, per-site status, "Sync now", "Quit"; app keeps polling when window is closed | P0 | Same as OS-3 | Tray icon + hide-on-close |
| OS-6 | File system | Synced folder `~/Moodle/<site>/<course>/<section>/` mirroring course files; user chooses which courses sync | P0 | `core_enrol_get_users_courses`, `core_course_get_contents`, `webservice/pluginfile.php` | Rust downloader, fs plugin scoped to that folder |
| OS-7 | OS search | Synced files are plain files in an indexed user folder, so Spotlight, Windows Search and Tracker find them with no extra work | P0 | — | Folder placement only |
| OS-8 | Calendar | One-click subscription of the user's Moodle calendar in the default calendar app | P1 | Calendar export URL (`calendar/export_execute.php` with the user's auth token) | Open `webcal://` URL with opener plugin; `.ics` file fallback |
| OS-9 | Calendar alerts | Native reminder 24 h and 1 h before due dates (configurable) | P1 | `core_calendar_get_action_events_by_timesort` | Scheduled local notifications |
| OS-10 | Drag and drop | Drop files on the window or dock icon → choose course and assignment → file added to submission draft | P1 | `webservice/upload.php` (draft area), `mod_assign_save_submission` | Webview drag-drop event, Rust upload |
| OS-11 | File manager | "Open with Moodle Desk" / Windows "Send to" entry that starts the same upload flow | P2 | Same as OS-10 | File associations, SendTo shortcut |
| OS-12 | Global shortcut | Ctrl/Cmd+Shift+M opens a quick-open palette across all sites' courses and activities | P1 | Local index built from OS-6 sync | Global-shortcut plugin + small palette window |
| OS-13 | Deep links | `moodledesk://<site>/<path>` opens the page in the app; also receives SSO token callbacks | P0 | Custom URL scheme accepted by the site (see §4) | Deep-link + single-instance plugins |
| OS-14 | Autostart | Optional start at login, minimized to tray | P1 | — | Autostart plugin |
| OS-15 | Default apps | Every file downloaded from the Moodle view is captured by the shell's download manager (§3.2), filed by course and opened in the OS default app | P0 | `pluginfile.php` | Opener plugin |
| OS-16 | Multi-site | Up to 10 sites; a sidebar or menu switches sites; each has its own token, folder, badge share | P1 | Per-site token | One webview per site, lazily created |
| OS-17 | Jump list / dock menu | Recent courses in the Windows jump list and macOS dock menu | P2 | Local index | Native menu APIs via Rust |
| OS-18 | Session bridge | When a token exists, open the web view already signed in, without a second login | P1 | `tool_mobile_get_autologin_key` + `admin/tool/mobile/autologin.php` | Rust requests key, loads URL in webview |

The in-window UI added by the app is deliberately thin: a site switcher, a sync panel and settings. Everything else is Moodle's own pages.

### 3.1 macOS-first requirements (v1.0)

macOS is the priority surface, so v1.0 is judged by how native it feels on a Mac. These extend the table above for macOS only.

| ID | Requirement | Pri. | How |
| --- | --- | --- | --- |
| MAC-1 | Menu-bar extra (status item) with unread count and list; app can run as menu-bar-only (no dock icon) if the user chooses | P0 | Tray icon API; `ActivationPolicy::Accessory` toggle |
| MAC-2 | Dock badge with unread count; dock menu with recent courses | P0 | Badge count API; native dock menu via Rust |
| MAC-3 | Notification Center: grouped per site, respects Focus modes, actionable ("Open", "Mark read") | P0 | Notification plugin; actions where the plugin supports them |
| MAC-4 | `~/Moodle` appears in the Finder sidebar on first run (offered, not forced); Quick Look and Spotlight work on synced files with no extra code | P0 | Plain files in the home folder; Finder favourite via user prompt |
| MAC-5 | Apple Calendar subscription in one click | P1 | Open `webcal://` URL; Calendar handles the rest |
| MAC-6 | Keychain storage with the app's own access group; no password prompts in normal use | P0 | `keyring` crate (Apple native store) |
| MAC-7 | Services menu entry "Upload to Moodle Desk" for files selected in Finder | P2 | `NSServices` in Info.plist; a Share extension is out of scope (needs a separate app-extension target) |
| MAC-8 | Native menu bar (App, File, Edit, View, Go, Window, Help) with standard shortcuts, Cmd+L for the quick-open palette inside the app | P0 | Tauri menu API |
| MAC-9 | Universal binary (Apple Silicon + Intel), hardened runtime, Developer ID signing, notarization; distributed as DMG outside the Mac App Store | P0 | Tauri bundler; App Store sandbox would block the `~/Moodle` folder model |
| MAC-10 | Managed deployment through Jamf / MDM: signed PKG and a managed-preferences domain for preset sites and feature switches | P1 | PKG bundle target; read `CFPreferences` in Rust |

Windows (v1.1) reuses the same core and adds MSI/NSIS packaging, taskbar overlay badge, jump list, toast actions, Credential Manager and a `webcal:` path through Outlook. Linux (v1.2) adds AppImage/.deb/.rpm, Secret Service, AppIndicator tray and a WebKitGTK QA pass.

### 3.2 Download manager for files downloaded through Moodle

The shell, not the webview, owns every download the user starts while browsing Moodle: resources, assignment feedback files, submission copies, forum attachments, grade exports, backups. Each one is saved into the right course folder, recorded in the index and handed to the OS, so "where did that file go?" never happens.

| ID | Requirement | Pri. |
| --- | --- | --- |
| DL-1 | Intercept every download started in a Moodle webview; the webview's own download UI never appears | P0 |
| DL-2 | Route by origin: a `pluginfile.php` URL whose context maps to a known course goes to `~/Moodle/<site>/<course>/<section>/`; anything else (exports, reports, backups) goes to `~/Moodle/<site>/Downloads/` | P0 |
| DL-3 | De-duplicate with folder sync: if the same file is already synced and unchanged, open the local copy instead of downloading again; if newer on the server, replace it and keep the old one in `.trash/` | P0 |
| DL-4 | Downloads panel in the shell (and in the menu-bar list): progress, pause/resume, cancel, retry, "Open", "Show in Finder", "Copy path", filter by site and course | P0 |
| DL-5 | On completion: native notification with "Open" and "Show in Finder"; the dock icon's download progress bar while files are in flight (macOS) | P1 |
| DL-6 | macOS metadata: keep the `com.apple.quarantine` attribute so Gatekeeper still checks executables and macros; write "Where from" (`kMDItemWhereFroms`) with the Moodle page URL; add a Finder tag with the course short name, so Spotlight and Finder can filter by course | P1 |
| DL-7 | Inline types (PDF, images, video): user setting per type — view inside Moodle, or download and open in the default app (default: PDF opens in Preview) | P1 |
| DL-8 | History: every download kept in the index with source page, course, activity, size, date and local path; history survives app restarts and can be cleared per site | P1 |
| DL-9 | Retention: optional auto-cleanup of `Downloads/` items older than N days (off by default); never touches files in course folders the user has moved or edited | P2 |
| DL-10 | Risky types (`.exe`, `.app`, `.dmg`, `.pkg`, scripts, macro-enabled Office files) show a warning before opening and are never opened automatically | P0 |
| DL-11 | Sign-out (SEC-8) offers to keep or delete that site's downloads and history separately from synced course files | P1 |

**Mechanism.** Tauri 2's webview builder exposes an `on_download` hook that fires when a download is requested and when it finishes, and lets the app set the destination path or cancel. The Rust core uses it to compute the target folder (DL-2), check the index for a synced copy (DL-3, cancelling the webview download and opening the local file instead), and record progress. Downloads that need the user's session cookies are left to the webview to fetch; Rust only chooses where the file lands, then moves, tags and indexes it on completion. The index gains a `downloads` table (id, site, course, module, source URL, page URL, local path, size, SHA-256, state, started, finished).

## 4. Technical architecture

The app is a Rust core that owns everything touching the OS and the Moodle token, plus two kinds of webview: a local React UI with IPC access, and remote Moodle webviews with none. The Moodle page can never reach the token, the file system or a Tauri command.

&#91;embedded content: architecture · two trust zones, Rust core, OS adapters\]

The Moodle view talks to Moodle directly for pages but reaches the Rust core only through the navigation and download hooks; the token, files and OS surfaces stay on the Rust side.

### 4.1 Components

| Component | Runs in | Responsibility |
| --- | --- | --- |
| Shell UI | Local webview (React, Vite, TypeScript, Zustand) | Site switcher, sync panel, settings, quick-open palette, upload picker |
| Moodle view | Remote webview, one per site | Renders the real site; zero capabilities, no `window.__TAURI__` |
| Auth module | Rust | Sign-in flow, token parsing and validation, keychain read/write, autologin key |
| WS client | Rust (`reqwest`, `tokio`) | POST to `webservice/rest/server.php` with `moodlewsrestformat=json`; retries, backoff, per-site rate limit |
| Sync engine | Rust | Diff course contents against the local index, download changed files, write to `~/Moodle/…` |
| Notifier | Rust background task | Polls notifications, messages and action events; drives notifications, badge and tray |
| Local index | SQLite (`rusqlite`) in app data dir | Sites, courses, modules, files (hash, `timemodified`, local path), sync state, search index |
| OS adapters | Rust + official plugins | Tray, badge, notification, deep-link, single-instance, global-shortcut, autostart, opener, updater, window-state |

The index lives in Rust SQLite rather than in Dexie because the notifier and sync engine must run while no UI webview is open (window closed to tray).

### 4.2 Sign-in and token

All sign-ins, password and SSO alike, go through the site's own `admin/tool/mobile/launch.php` page inside a dedicated auth webview, so the app never sees a password.

1. Rust calls `tool_mobile_get_public_config` (no token needed) to read the site name, login type and identity providers.
2. The auth webview loads `launch.php?service=moodle_mobile_app&passport=<random>&urlscheme=<scheme>`; Moodle redirects to its login page or the SSO IdP as usual.
3. On success Moodle redirects to `<scheme>://token=<base64>`. The webview's `on_navigation` hook catches that URL before the OS does, decodes `signature:::token:::privatetoken`, and checks the signature (MD5 of site URL + passport).
4. Rust stores token and private token in the keychain, calls `core_webservice_get_site_info` to confirm the user, and closes the auth webview.
5. To open the Moodle view already signed in, Rust calls `tool_mobile_get_autologin_key` and loads `admin/tool/mobile/autologin.php` with that key (OS-18).

**Scheme decision.** Moodle's `forcedurlscheme` setting defaults to `moodlemobile`, and launch.php refuses other schemes unless the admin changes it; changing it would break SSO in the official mobile app. Because step 3 intercepts the redirect inside our own webview, the app can request `moodlemobile` and catch it without registering that scheme with the OS, so stock sites work unchanged. Sites that prefer a distinct identity can set a dedicated service and scheme (open question Q2). `login/token.php` is kept only as a fallback for sites where launch.php is disabled.

### 4.3 Sync engine

- Trigger: on sign-in, every 30 min by default, on "Sync now", and when a course notification arrives.
- Diff: `core_course_get_contents` per synced course; a file is fetched when its URL, size or `timemodified` differs from the index.
- Download: `webservice/pluginfile.php` with the token, 4 parallel downloads per site, resumable, written to a temp file then renamed atomically.
- Layout: `~/Moodle/<site>/<course short name>/<NN section name>/<file>`; names sanitized per OS; collisions suffixed.
- Deletions on the server move local files to `~/Moodle/<site>/.trash/` for 30 days, never silently deleted.
- Local edits are never uploaded back; the folder is read-only by convention and a README says so.

### 4.4 Notifier

- Polls every 2 min while online (configurable 1–15), backs off to 15 min on battery saver or after errors.
- New items since the last seen ID become native notifications, grouped per site; the badge shows the sum of unread notifications and conversations.
- Uses `core_calendar_get_action_events_by_timesort` once an hour to schedule local due-date reminders (OS-9).
- Desktop push is not used: Moodle's push path (Airnotifier) targets the mobile apps, so polling is the v1.0 design.

### 4.5 Upload flow

A dropped file is posted to `webservice/upload.php` (user draft area, returns an `itemid`), then `mod_assign_save_submission` attaches that draft to the chosen assignment. The app then opens the assignment page so the user reviews and submits through Moodle's own UI; the app never presses "Submit".

### 4.6 Stack

| Layer | Choice |
| --- | --- |
| Framework | Tauri 2 (stable), Rust stable |
| UI | React + Vite + TypeScript, Zustand for state |
| Rust crates | `reqwest`, `tokio`, `serde`, `rusqlite`, `keyring`, `md5`, `base64`, `tracing` |
| Official plugins | deep-link, single-instance, notification, global-shortcut, autostart, opener, fs (scoped), store, updater, window-state, log, dialog |
| Multi-webview | Child webviews in one window (Tauri `unstable` feature); fallback: one window per site if that API proves unreliable on Linux |

### 4.7 Adaptability

The app adapts to institutions and to future OS features without forks.

- **Configuration, not code:** a signed `config.json` or MDM profile sets preset sites, locked site list, app name and icon for branded builds, polling and sync defaults, and turns each OS feature on or off.
- **OS adapters behind one interface:** each OS surface (notifications, badge, tray, calendar, file tagging, credential store) is a Rust trait with one implementation per OS, so a new surface (for example macOS widgets or Windows Widgets) is added as one adapter without touching sync, auth or UI.
- **Feature modules:** sync, download manager, calendar, upload and quick-open are independent modules the config can disable; the shell UI renders only enabled ones.
- **Server-side growth path:** v1.0 needs nothing on the server; a later optional Moodle plugin (`local_moodledesk`) could add a dedicated service, push-style change feeds instead of polling, and admin-side policy, while sites without it keep working.

## 5. Security, privacy and compliance

The security model rests on one rule: remote Moodle content is untrusted, and only the Rust core holds secrets or touches the OS.

| ID | Requirement | Pri. |
| --- | --- | --- |
| SEC-1 | Moodle webviews get no Tauri capability; capabilities list only the local shell windows by label, and `remote.urls` is never used | P0 |
| SEC-2 | Navigation allowlist per site: the site origin plus the SSO IdP origins found at sign-in; any other link opens in the default browser | P0 |
| SEC-3 | Tokens and private tokens only in the OS keychain; never in SQLite, logs, crash reports or the store plugin | P0 |
| SEC-4 | The app never asks for, sees or stores a Moodle or SSO password (all sign-ins via launch.php, §4.2) | P0 |
| SEC-5 | fs plugin scoped to `~/Moodle/` and the app data dir; no shell plugin; no arbitrary command execution | P0 |
| SEC-6 | Strict CSP on the local UI (`default-src 'self'`); no remote scripts in the shell | P0 |
| SEC-7 | TLS only; refuse `http://` sites unless a developer flag is set; no certificate-error bypass in the UI | P0 |
| SEC-8 | Sign-out deletes the site's token (`server-side revocation: see Q5`), its webview cookies, its index rows and, after confirmation, its synced folder | P0 |
| SEC-9 | Updater artifacts signed with the Tauri updater key; installers code-signed (Windows Authenticode, Apple Developer ID + notarization) | P0 |
| SEC-10 | Dependency audit (`cargo audit`, `npm audit`) in CI; Tauri and plugin updates applied within 14 days of a security release | P1 |
| SEC-11 | User-Agent identifies the app (`MoodleDesk/<version>`) so site admins can see and, if they wish, block it | P1 |

**Privacy.** By default, data stays on the device and goes only to the user's own Moodle site; no analytics by default. The planned optional AI assistant (§7) adds an explicit exception: prompts and user-approved Moodle context are sent to the selected Albert or Mistral AI provider using the user's own key. Before enabling context sharing, explain the data sent and the provider's applicable processing/retention terms; no provider retention guarantee is assumed. Moodle tokens and cookies must never enter model context. Opt-in crash reports exclude URLs, file names and course names. Synced files are personal data under GDPR when they include other users' content (forum attachments, feedback), so the sign-out wipe (SEC-8) and a documented local-storage notice are required.

**Exams.** Quiz pages that require Safe Exam Browser must keep failing in the app as they do in a normal browser; the app sends no SEB headers and makes no attempt to emulate them.

## 6. Non-functional requirements, packaging and distribution

The app must stay lighter than a browser tab when idle in the tray and must install cleanly on managed fleets.

| Area | Requirement (v1.0 target) |
| --- | --- |
| Installer size | Under 15 MB per platform |
| Idle memory | Under 120 MB with window closed to tray and one site signed in |
| Cold start | Window with site visible in under 2 s on a 2022 mid-range laptop (excluding network) |
| Notification latency | Under 1 polling interval (default 2 min) from Moodle event to native notification |
| Sync throughput | Limited by network; never more than 4 concurrent requests per site |
| Battery | No polling faster than 15 min on battery saver; no wake locks |
| Resilience | Offline start works: tray, folder, palette and index usable with no network |
| Accessibility | Shell UI meets WCAG 2.1 AA; keyboard-only operation of palette, switcher and upload picker |
| Localisation | Shell UI in English and French at launch; Moodle content uses the site's own language packs |
| Logs | Rotating local logs, 10 MB cap, no tokens or personal content |

**Platform matrix**

| Order | OS | Minimum | Webview | Packaging and notes |
| --- | --- | --- | --- | --- |
| 1 (v1.0) | macOS | 12 Monterey | WKWebView | Universal DMG + PKG; Developer ID + notarization; dock badge, menu-bar extra |
| 2 (v1.1) | Windows | 10 22H2, 11 | WebView2 (Evergreen, bootstrapped by installer) | MSI for admins, NSIS for users; taskbar overlay badge, jump list |
| 3 (v1.2) | Linux | Ubuntu 22.04, Fedora 39 | WebKitGTK 4.1 | AppImage, .deb, .rpm; tray depends on desktop environment; WebKitGTK is not in Moodle's tested browser list, so it gets its own QA pass |

**Moodle compatibility.** Supported: Moodle 4.5 LTS, 5.0, 5.1, 5.2 and 5.3 LTS (released 5 October 2026). Site prerequisites: "Enable web services for mobile devices" on and the `moodle_mobile_app` service enabled; nothing else.

**Updates.** Tauri updater plugin pulling a signed JSON manifest from a static host; stable and beta channels; admins can disable updates via a policy key (registry on Windows, managed preferences on macOS).

**Managed deployment.** A `config.json` beside the binary or an MDM policy can preset site URL(s), lock the site list, set polling interval and disable features (calendar, sync, autostart).

## 7. Milestones, metrics, risks and open questions

The original baseline estimated macOS v1.0 at about 13 weeks for two engineers (one Rust-leaning, one front-end), with Windows v1.1 and Linux v1.2 each about 4 weeks later. These are historical estimates; the expanded Alpha scope below requires re-estimation by slice after the M0/API spikes.

&#91;embedded content: roadmap · 6 phases, macOS first\]

Phases are drawn equal width, not to scale; the M3 gate is the macOS v1.0 release, and Windows and Linux reuse the same core with platform adapters only.

### Alpha scope amendment — 9 November 2026

The product owner requested more user-facing features in the Alpha. The [expanded Alpha feature plan](alpha-feature-expansion.md) and [working tracker](plan.md) now promote the following bounded slices from later roadmap work. This changes delivery sequencing, not the full v1.0 requirement definitions in §3.

| Delivery | Promoted scope | Larger scope retained for later |
| --- | --- | --- |
| Expanded Alpha A1 | Notification inbox/read actions; searchable download history, Copy Path, safe retry/fallback and inline-file preferences (OS-3–OS-5, MAC-3, DL-4/DL-7/DL-8 partial) | Unqualified native notification actions, download pause/resume and full metadata/retention controls |
| Expanded Alpha A2 | Course browser/pins; course/cached-activity quick-open, Cmd+L and an optional global shortcut (OS-12/MAC-8 partial) | Server full-text and file-content search |
| Expanded Alpha A3 | Explicitly selected course files saved offline and manually refreshed (OS-6/OS-7 partial; plain-file MAC-4 behavior) | Full scheduled mirroring, automatic deletion handling, resumable sync and complete DL-3 de-duplication |
| Expanded Alpha A4–A5 | Read-only upcoming agenda, opt-in local reminders while running, and manual token-free ICS snapshot export (OS-8/OS-9/MAC-5 partial) | Live authenticated calendar subscriptions, automatic external-calendar refresh and reminder delivery guarantees while quit |
| Expanded Alpha A6 | Optional Albert/Mistral AI BYOK chat side panel with explicit Moodle context and bounded tools, implemented after A1–A5 | Broad autonomous actions, assignment submissions and grade changes |
| Later releases | Uploads/submission assistance (OS-10), Finder Services, signing/notarization, managed deployment and additional platforms | Unchanged v1.0/later-platform requirements remain applicable |

The original Alpha six-week estimate and overall sequencing are baselines for the smaller scope, not commitments for this expansion. Re-estimate by slice. None of A1–A5 is marked delivered by this planning decision; full security, sign-in, account-isolation and platform gates remain mandatory. The live-calendar export-token question Q6 still applies to the deferred subscription path; manual ICS generation from agenda data does not depend on that token.

### AI assistant scope amendment — 10 November 2026

Add an optional, collapsible AI chat side panel alongside the Moodle view. Users bring their own Albert API or Mistral AI key and choose their provider/model. The assistant answers general questions without Moodle context and can use explicitly selected Moodle data exposed to the app to answer course questions, find deadlines/resources and open relevant items. Answers grounded in Moodle data link back to their sources and identify unavailable or stale context.

Interaction uses a bounded set of account-scoped tools mediated by the Rust core, respecting the site's actual web-service capabilities and the user's permissions. Start with read/search/navigation; selected writes such as marking a notification read require an exact-target preview and explicit confirmation. The initial scope excludes assignment submissions, grade changes and arbitrary page/script execution. Moodle content and model output are untrusted inputs, never authorization for a tool call.

This feature is **A6, the final implementation slice of the expanded Alpha**, after A1–A5 and before combined qualification/release. It is part of the Alpha feature gate. The [working roadmap](plan.md#q3--combined-expanded-feature-acceptance-a1a6) tracks provider validation, panel layout/accessibility, credential storage, explicit context sharing, history deletion, account isolation and acceptance checks. This amendment records scope rather than verification. As of 10 November, the bounded implementation has landed against fixtures; live provider/Moodle and native qualification remain open in the working tracker.

### 7.1 Success metrics (90 days after v1.0)

| Metric | Target |
| --- | --- |
| Crash-free sessions | ≥ 99.5 % |
| Users with folder sync on | ≥ 60 % of active users |
| Users with calendar subscribed | ≥ 40 % |
| Notification click-through | ≥ 25 % |
| Sign-in to first completed sync | Median under 3 min |
| Support tickets | Under 2 per 100 active users per month |

### 7.2 Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| A site admin objects to tokens issued to the `moodle_mobile_app` service for a non-official client | Users of that site blocked | Identifying User-Agent (SEC-11), admin docs, optional dedicated service and scheme (Q2) |
| Multi-webview API is still behind Tauri's `unstable` flag | Layout bugs, API changes | Fallback to one window per site (§4.6) |
| WebKitGTK renders some Moodle pages or editors (TinyMCE) differently | Broken pages on Linux | Dedicated Linux QA pass; "Open in browser" menu item on every page |
| SSO IdPs that block embedded webviews (some OAuth 2 providers) | Sign-in fails | Fallback: system browser + deep-link callback with a registered `moodledesk` scheme where the site allows it |
| Large courses (thousands of files) slow the first sync | Poor first impression | Opt-in per course, size shown before sync, resumable downloads |
| Moodle web-service changes across 4.5–5.3 | Sync or notifier breaks | Contract tests against one Docker instance per supported version in CI |
| Trademark: "Moodle" in the product name | Forced rename | Check Moodle trademark policy before public release (Q3) |

### 7.3 Open questions

- [ ] **Q1** Audience: a generic app for any Moodle site, or a build branded for one institution with a preset URL?
- [ ] **Q2** Default sign-in identity: reuse `moodle_mobile_app` + `moodlemobile` interception, or require sites to create a dedicated service and scheme?
- [ ] **Q3** Product name and Moodle trademark clearance.
- [ ] **Q4** Linux tray support baseline (GNOME needs an AppIndicator extension): is tray-less Linux acceptable for v1.0?
- [ ] **Q5** Server-side token revocation on sign-out: which web-service function, if any, is available across 4.5–5.3?
- [ ] **Q6** Calendar export: confirm the export auth-token path works with SSO-only accounts on all supported versions.

### 7.4 Sources

- [Moodle Desktop (discontinued) — MoodleDocs](https://docs.moodle.org/en/Moodle_Desktop)
- [Moodle 5.1 release notes — moodledev.io](https://moodledev.io/general/releases/5.1)
- [Moodle release support dates — endoflife.date](https://endoflife.date/moodle)
- [MDL-67767: forcedurlscheme and launch.php validation — Moodle tracker](https://tracker-old.moodle.org/browse/MDL-67767)
- [MDL-56905: private token via launch.php — Moodle tracker](https://tracker-old.moodle.org/browse/MDL-56905)
- [Example desktop client using launch.php interception (bk-study-desk wiki)](https://github.com/xeroz369/bk-study-desk/wiki/How-the-app-works)
- [Tauri 2 plugin releases](https://v2.tauri.app/release/deep-link/v2.0.0/)
