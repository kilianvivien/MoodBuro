# B2 — stock Moodle calendar subscription

Implementation based on `1d7b7c7`, 11 November 2026. Automated implementation checks and one live feed-access check are recorded below; the full installed Calendar acceptance gate remains open.

## Workflow and supported source

Agenda → Calendar Subscription → Set Up… asks before passing a private feed to Apple Calendar. File → Subscribe to Calendar… (⌥⌘S) opens the same section. Calendar owns final subscription creation, destination account, refresh cadence, alerts and removal. MoodBuro reports a setup request, never claims the external subscription was installed or is refreshing successfully. Existing manual, token-free ICS snapshot export stays available with its existing filters.

Only stock Moodle is implemented. `md_moodle::calendar` owns the provider identity, issuer and credential-bearing `Feed`; the Tauri layer owns consent, identity revalidation, handoff and history. A future Inspcal-derived Moodle plugin can add its own provider adapter and feed validation/export policy here. No plugin endpoint, arbitrary pasted URL, automatic plugin discovery or guessed credential format is enabled now. That provider will need its own capability/version contract, feed scope, revocation and HTTPS tests; the UI/native handoff should be reused. Stock feed creation and the existing local ICS renderer remain independent, leaving room for a future provider-specific ICS export.

## Stock protocol and credentials

The app verifies the current account with `core_webservice_get_site_info`, then POSTs `core_calendar_get_calendar_export_token` with the web-service credential in the request body. The returned calendar token is distinct from the API token. Rust builds a fixed same-installation HTTPS `calendar/export_execute.php` address with `userid`, `authtoken`, `preset_what=all`, and `preset_time=recentupcoming`.

Stock `recentupcoming` covers the past five days and the next 60 days, evaluated by Moodle on each refresh. It is independent of the shell's date filters and cached upcoming view. The feed is read-only and server-owned; the app neither rewrites its events nor appends alarms. Moodle preserves server event UIDs across refreshes. A rolling window is not a complete calendar archive, and completed activities are not guaranteed to disappear from this export.

Feed verification uses a separate cookie-free HTTPS client, no redirects, a 20-second timeout and a 5 MiB response cap. It requires `text/calendar` and a valid bounded VCALENDAR envelope/version; HTTP 200 alone is insufficient. It does not replace Calendar's ICS parser. Transport errors are mapped to fixed messages without request URLs or server bodies.

`ExportToken` and `Feed` have zeroizing owned credentials and no Debug/Serialize implementation. Calendar receives the private URL through NSWorkspace targeting `com.apple.iCal` explicitly, preserving HTTPS; no conversion to potentially insecure `webcal://`, default-handler dispatch, pasteboard, shell subprocess or URL-bearing IPC is used. Recent-item registration is disabled. Calendar may retain/sync the feed according to its own account settings, selected by the user.

## History, duplicate protection and account lifecycle

SQLite schema 16 stores only site/user identifiers, provider ID and the setup-attempt timestamp. It stores no feed address, calendar token or event contents. A per-account in-flight guard rejects concurrent setup. A durable attempt record is written before native dispatch: timeout, process interruption, a launch error or cancellation in Calendar still causes the next attempt to require explicit repeat confirmation. This avoids silently reopening a possibly successful handoff. The record is not subscription confirmation.

Account identity is rechecked after network operations, immediately before dispatch under the store lock, and after callback completion. Account replacement/removal clears local attempt history and prevents late old-account writes. A handoff already dispatched to Calendar cannot be recalled by account removal.

Stock Moodle's export token is derived from the user ID, stored password value and server export salt. Signing out, deleting the app's API credential or removing the organisation does **not** revoke a token already shared with Calendar. There is no app-specific stock revoke function in this implementation. Remove the subscription in Calendar and ask the Moodle administrator about revocation; do not promise that an SSO password change alone revokes it. The UI states this briefly before consent and during account removal. MoodBuro does not request EventKit/calendar-store permissions, enumerate users' calendars or delete external subscriptions.

The guard covers repeated setup through this app. It cannot detect a manually configured subscription, a previous ICS import, a setup performed on another Mac or an account history cleared by removal. Users must check Calendar before reopening setup; existing manual imports can produce duplicates because snapshot and stock-feed UID scopes differ.

## Agenda and reminder boundary

The shell still caches Moodle's bounded upcoming-view API (maximum 200 normalized events). This is not the full export feed. Missing/expired data remains visible with freshness/error information; unavailable action/completion fields cannot establish that a task is complete. Existing A4 eligibility treats explicit non-actionable/zero-item actions as ineligible; absent action metadata remains unknown and does not by itself suppress the reminder. Reminder schedules therefore cannot guarantee complete coverage or completion-aware suppression across all Moodle contracts.

MoodBuro reminders remain default-off, per-account, at 24 hours/1 hour or one-off, while the process runs. Closing the window keeps them running; quitting stops them. Apple Calendar refresh/alerts are independent and may operate while MoodBuro is quit. Users can disable either alert path to avoid duplicate alerts. B2 does not add a second reminder engine or silently enable either app's alerts.

## Evidence recorded on 11 November 2026

- The user-provided authenticated browser account exposed the standard export form and rolling-calendar option. After explicit approval, its generated private URL was fetched in-page with `credentials: 'omit'` and `redirect: 'error'`: HTTP 200, `text/calendar`, valid VCALENDAR, zero events. No token or full URL was returned to chat, written to the repository or added to Calendar.
- A synthetic invalid token also returned HTTP 200 but did not return VCALENDAR. This motivates response-content verification. It is not a real token-revocation test.
- No events were available in that live export, so real UID stability, changes/deletions, recurrence or DST behavior was not established. A live app REST-token invocation and the full supported-version/SSO matrix also remain open; authenticated browser export is not proof of those contracts.
- 119 frontend and 127 Rust tests pass, plus frontend lint/format/build, Rust formatting, workspace Clippy with warnings denied and shell-only IPC security checks. Fixtures cover token separation, identity mismatch, fixed URLs, login/invalid/revoked/redirect responses, empty feeds, repeat confirmation, account lifecycle and concise bilingual consent/IPC DTOs. Earlier snapshot escaping/UID/DST tests remain intact.
- Integrated-browser demo checks covered English/light, French/dark at 700 CSS px, no horizontal overflow, Cancel-first focus, Escape/focus restoration, demo-only handoff history and repeat warning. A further French/light check at 780px with the assistant open left 368px of main content and a 320px consent panel, with no horizontal overflow. The explanatory management paragraph is behind a secondary disclosure. No user calendar was added or changed.
- The macOS adapter and native menu compile and pass Clippy. NSWorkspace callback success only proves dispatch, not completed Calendar setup. Real Calendar creation/refresh and installed-app privacy/permissions behavior remain unqualified.

## Remaining installed/live checks

- [ ] On the primary account and supported Moodle rigs, retrieve the token through the app's REST service; test disabled export, missing service capability, expired credentials and SSO-only accounts.
- [ ] Finish setup in Apple Calendar from an installed app; verify HTTPS, refresh cadence, native permission/launch failures, final cancellation and persistence after MoodBuro quits. Check the macOS 12+ support matrix.
- [ ] Create/change/delete test events; verify stable UIDs, no duplicates, recurring events, all-day events and Europe/Paris DST on actual Calendar refresh. Use a test account/fixture calendar, not production event edits.
- [ ] Test actual server-side revocation and recovery with administrator-approved credentials. Confirm expired/revoked feeds stop refreshing without disclosing private URLs in app logs/errors.
- [ ] Test account replacement/removal during retrieval and dispatch, restart after native handoff before callback, repeated setup after a timeout, and manual-import/subscription overlap.
- [ ] Qualify native keyboard/VoiceOver, assistant-open minimum width, contrast and motion preferences, plus the File-menu accelerator.

## Primary references

- [Moodle 4.5 service registration](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/lib/db/services.php) — stock mobile service includes the calendar export-token API.
- [Calendar export-token external method](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/calendar/classes/external/export/token.php) — no parameters, export-enabled check, permanent export token.
- [Calendar export implementation](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/calendar/export_execute.php) — credential validation, rolling window and server UIDs.
- [Calendar token derivation](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/calendar/lib.php) — password/export-salt dependency, independent of the web-service token.
- Installed Calendar 27.0 `iCal.sdef` describes GetURL as subscribing to a remote calendar via HTTP/webcal; the adapter uses explicit HTTPS dispatch. This local declaration is not cross-version runtime qualification.
