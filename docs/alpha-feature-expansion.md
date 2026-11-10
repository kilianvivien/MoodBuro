# Expanded Alpha feature scope

Decision: 9 November 2026, at the product owner's request to bring more user-facing features into the Alpha.

These are **planned implementation targets, not delivered features**. This amendment replaces the original Alpha's exclusion of all offline files, calendar features and quick-open. It promotes bounded slices of the PRD's later work; it does not bring every part of those requirements forward. The [working tracker](plan.md) owns completion, the [implementation status](implementation-status.md) owns evidence, and the [PRD](prd.md) retains the full v1.0 requirements.

The original six-week schedule was for the smaller core-shell scope. It is now a historical estimate, not a commitment for this expanded Alpha. Estimate each slice after its API/UX spike; no new release date is promised here. The original sign-in, security, platform and packaging gates remain required.

## What moves into Alpha

| Slice | User-visible outcome | PRD coverage promoted | Boundary of this first version |
| --- | --- | --- | --- |
| A1 — Inbox and useful downloads | Read notifications across sites, mark individual items read, find and manage saved downloads | OS-3–OS-5, MAC-1/MAC-3 actions in the app; DL-4/DL-8 partial | Local inbox and paginated history; native notification actions only where qualified. No pause/resume or unsafe replay of old download URLs. |
| A2 — Courses and quick-open | Browse enrolled courses, pin frequently used ones, and jump to a course or cached activity with the keyboard | OS-12 partial, MAC-8; supports the existing DL-2 course-routing requirement | Course/cached-activity metadata search, in-app Cmd+L and an optional global Cmd+Shift+M shortcut. No server full-text search or file-content indexing. |
| A3 — Save selected files offline | Choose files from a course and keep readable copies in the course's Finder folder | OS-6/OS-7 partial; plain-file portion of MAC-4 | User-selected, one-shot transfers and manual refresh. No automatic course mirroring, scheduled sync, server-deletion propagation or upload of local edits. |
| A4 — Agenda and reminders | See upcoming Moodle events/deadlines across sites and opt into local reminders | OS-9 partial | Read-only upcoming agenda and configurable local reminders while MoodBuro is running. No offline submission or wake-while-quit guarantee. |
| A5 — Calendar export | Export the visible agenda to a local `.ics` file for import into Calendar | OS-8/MAC-5 partial | Manual snapshot export generated from agenda data. No live `webcal://` subscription, background refresh or token-bearing feed URL. |
| A6 — AI assistant (added 10 November 2026) | Chat in a side panel using Albert or Mistral AI with BYOK; answer general questions and interact with explicitly selected Moodle context | [AI scope amendment](prd.md#ai-assistant-scope-amendment--10-november-2026) | Final Alpha feature after A1–A5; account-scoped read/navigation tools and individually confirmed supported writes. No broad autonomy, submissions or grade changes. |

Pins are a small addition to the course browser, stored locally per account. They do not change Moodle favourites or other server state.

## Delivery order and dependencies

1. Finish the current PDF/popup/download escape paths, including a usable public-API save fallback, before building more file workflows on them. Keep live platform checks tracked separately when user intervention is unavailable.
2. **A1:** expose the existing notification/read operations and improve the current Downloads view. This provides immediate value using the current foundation.
3. **A2:** build the account-bound course metadata index, course browser, pins and quick-open. Use it to finish reliable course/context routing for downloads.
4. **A3:** add explicit offline file selection and manual refresh on top of the course index and safe file-publication rules.
5. **A4:** add the upcoming agenda and opt-in reminders, independently of file transfers.
6. **A5:** export the agenda locally as ICS; this depends on A4's normalized event model, not on a server calendar-export token.
7. **A6:** implement the AI chat side panel last, with the provider, context-sharing and bounded-tool acceptance gate in the [working plan](plan.md#5-ai-chat-side-panel--final-alpha-feature-a6).
8. Run the combined feature/security/platform matrix and package the expanded Alpha. Account-dependent checks, signing identity, updater keys and hosting are separate tasks; unavailable intervention must not block unrelated implementation or be recorded as a passed gate.

A completed slice should produce a reviewable local build and its relevant evidence. These slices are the target feature set for the expanded Alpha. If a slice cannot meet its acceptance criteria, record an explicit scope decision in the tracker rather than silently dropping it or marking the full PRD requirement complete.

## Progress — 9 November 2026

First implementation pass over A1, A2 and the read-only half of A4, against synthetic fixtures only. Checked boxes below mean "built and covered by automated tests"; none of the **Acceptance** paragraphs is met yet, because each needs a real account, an installed build or the 4.5/5.3 rigs. Items with an italic note are partly built. A3, A5 and A4's reminders are untouched.

## A1 — Notification inbox and download controls

- [x] Add a local notification inbox grouped/filterable by site, with subject, date, unread state, Open and Mark Read.
- [ ] Use the existing native/menu-bar entry points to reach the inbox and expose the latest unread items without wide menu rows. _(Partly built: View → Inbox ⌘1 and a tray Inbox entry with the count; individual unread items are not listed in the tray.)_
- [x] Open a safe site notification page when a precise activity destination is unavailable; never follow an unvalidated context URL.
- [x] Refresh authoritative counts/read state after Mark Read. Show failures and preserve the previous state when the server rejects the operation.
- [x] Replace the 20-row-only Downloads view with paginated history and filename/site/state filters; add course filtering when A2 supplies a reliable association. Keep an explicit Unknown Course bucket. _(Built: paging and filename/site/state/course filters with Unknown Course. Course mapping comes from explicit Moodle module-metadata refresh; live qualification remains open.)_
- [x] Add Copy Path and per-site Clear History, keeping files on disk. Confirm history clearing and describe its effect.
- [ ] Add a safe retry route: resolve a fresh source from stable Moodle file metadata when available; otherwise offer return to the relevant Moodle page/site. Never persist or replay a stale credential-bearing URL or claim that revealing a page restarted a transfer.
- [ ] Add a PDF/image handling preference: retain inline viewing by default; offer an explicit Save/Open in Default App route through managed downloads. Risky/unknown types always retain confirmation and never auto-open.

**Acceptance:** a notification can be opened/read with badge/tray counts reconciled; reconnect and permission failures are honest; an old download remains discoverable after restart; clearing history leaves files untouched; missing files, risky types and expired source links follow the existing guards. Native actions have an in-app fallback.

## A2 — Course browser, pins and quick-open

- [x] Fetch enrolled-course metadata with bounded requests, cache it per account, and expose refresh, empty, offline/stale and permission-error states.
- [x] Add a compact course browser with local pins and course/site labels. Opening a result routes through the existing session bridge and navigation guard.
- [ ] Add Cmd+L quick-open across sites' course names and already cached activities. Search cached metadata locally; show what is available offline and when the cache was refreshed. _(Built: ⌘L over course names, organisations and shell views. Open: no activity metadata is cached yet, and the panel does not show the cache age — the Courses view does.)_
- [ ] Offer an opt-in global Cmd+Shift+M shortcut; handle registration conflicts visibly and unregister it when disabled or quitting.
- [x] Reuse course/context identifiers to finish course-folder download routing. Keep the Downloads fallback when association is unknown; never infer a course from a similar name.
- [x] Clear metadata/pins on removal or account replacement and reject late responses from the previous user.

**Acceptance:** keyboard-only search can open the correct course across three sites; pins survive restart; a cached list remains usable offline without implying that the Moodle page itself is offline; shortcut conflicts do not override another binding or prevent app use. API access failure does not break ordinary Moodle browsing.

## A3 — Selected files offline

- [ ] List downloadable resources for a selected course and let the user explicitly choose files before starting a transfer. Show known sizes and label unknown sizes.
- [ ] Provide bounded transfer concurrency, available-space checks, progress/status and actionable per-file failures.
- [ ] Write to course/section folders using the established sanitization, staging, collision, quarantine and risky-open rules. Keep tokens in Rust/Keychain and out of URLs exposed to the shell, logs and SQLite.
- [ ] Add an offline files list with Open and Show in Finder that works without a Moodle connection; opening the online activity remains a separate action.
- [ ] Implement Refresh Selected Files. Reuse only an indexed unchanged copy that still matches the local snapshot; keep changed server versions alongside existing files. Never overwrite or remove user-edited/moved files.
- [ ] Explain that refresh is manual and one-way. Preserve files when an account is removed; clear private account/index metadata and revoke access to late old-account writes.

**Acceptance:** selected files can be saved from a supported test course and reopened after quitting and disconnecting the network. Repeat refresh does not duplicate unchanged indexed files; a changed remote version or local edit does not destroy an existing copy. Unsupported resources and denied permissions are reported without claiming a complete offline copy of the course. Plain files can be found by normal OS indexing where enabled; forced Spotlight indexing/Finder sidebar modification is not part of this slice.

## A4 — Upcoming agenda and local reminders

- [x] Fetch available upcoming event/deadline metadata per site into a normalized read-only agenda. Link each item back to its Moodle page through validated routing. _(Each item opens its day in the Moodle calendar, a fixed page; the activity's own URL is not followed.)_
- [x] Display the site/course, event date, time zone and last-refresh/stale state. Do not invent submission/completion status the service does not supply.
- [ ] Offer reminders, off by default, with 24-hour/1-hour offsets and per-site control. Suppress past and known-completed items and deduplicate by account/event/due-time/offset.
- [ ] Reconcile changed/deleted events after refresh; avoid an old-reminder burst on initial import or after sleep. Reschedule from the latest known state on wake.
- [ ] Make permission/network limitations visible. No reminder guarantee is made while the app is quit; closing the main window still uses the background process.

**Acceptance:** fixtures cover changed due dates, cancellations, account replacement, time-zone/DST boundaries, restart and duplicate prevention. Native reminder delivery is qualified on the supported macOS matrix. Server access denial leaves the other app features working.

## A5 — Manual ICS export

- [ ] Export the selected site/date range from the agenda as a local ICS snapshot, with stable account-scoped event UIDs, proper escaping and explicit time zones/UTC values.
- [ ] Include only intended event metadata and safe Moodle page links, never web-service tokens, private tokens, calendar secrets or signed login URLs.
- [ ] Use explicit user export/import actions; explain that the snapshot does not update automatically. Do not call this a calendar subscription.

**Acceptance:** validate the generated file with an ICS parser and import a test calendar into Apple Calendar; verify times, accented/multiline text and DST cases. Stable UIDs are provided, but repeat-import merge behavior must be tested rather than assumed. No external calendar account or private hosting is needed for file generation.

## Stays after Alpha

- Full background course-folder mirroring, automatic refresh/deletions, resumable sync and full DL-3 sync/download de-duplication.
- Live authenticated calendar subscriptions and automatic external-calendar refresh.
- Upload staging, assignment submission assistance, Finder Services/share integration and any automatic server-side write beyond marking a notification read.
- File-content search/Spotlight extensions, offline editing/quiz attempts and automatic upload of local changes.
- Download pause/resume, automatic retention/deletion and expanded Finder metadata/tagging.
- Developer ID signing/notarization, managed deployment and Windows/Linux releases.

## Shared release criteria

Course browsing, the inbox and agenda are compact metadata/navigation surfaces; course activities, editing and submissions still use Moodle’s own pages. All promoted features need French/English copy, keyboard access, empty/offline/error states, account isolation and relevant automated checks. Native and real-site checks stay explicit when they cannot be completed autonomously. The original Alpha's security/sign-in/platform/distribution gates still apply. There are no new external credentials, signing keys or hosting prerequisites for building these feature slices against synthetic fixtures.
