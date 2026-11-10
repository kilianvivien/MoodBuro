# A4 — local deadline reminders

Built against synthetic fixtures. Native notification delivery, click routing and real Moodle completion metadata remain unqualified.

## Behaviour and limits

Agenda contains independent **24 hours before** and **1 hour before** checkboxes per saved account, both off by default. They apply to that organisation’s cached upcoming deadlines independently of the agenda’s visible date/site filters. Preference writes validate the saved Moodle user. Account replacement/removal clears preferences, event scheduling facts and the delivery ledger.

A background reader refreshes enabled agendas serially, then waits five minutes or responds to a preference/wake signal. Slow or failed requests can lengthen the cycle. Requests run in Rust using Keychain credentials. No provider, URL, token, message body or new remote write is involved. Only the existing normalized `due` event kinds qualify. Moodle action metadata explicitly reported as non-actionable or with zero remaining items is suppressed; missing action metadata is unknown, not a claim of completion. [Moodle’s action exporter](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/calendar/classes/external/event_action_exporter.php) defines those fields; plugin-specific semantics still need live qualification.

The local scheduler checks every 15 seconds. It uses Unix timestamps and elapsed-hour offsets, so changing the UI time zone or crossing DST does not shift a deadline. Both request-start and successful-refresh timestamps must be recent (within 15 minutes). Startup requires a new request. A wall-clock gap over 60 seconds or a backward clock jump skips the missed window and requires a request started after the gap; this is periodic resume detection, not an OS wake-while-quit service. Shorter pauses use the ordinary bounded check window. A response from a request started before sleep cannot qualify as a post-wake refresh; an older response cannot replace a newer schedule.

New imports and changed due times/eligibility establish a new observation baseline. Triggers already passed when observed or enabled are skipped. Authoritative refresh removes deleted events from scheduling. Only the current (at most 200 events per organisation) agenda response participates; there is no promise of a complete institutional calendar.

SQLite schema 9 persists a ledger keyed by organisation, Moodle user, event, due time and offset. Entries are claimed before native delivery under the account/store lock. This provides **at-most-once attempts**, including across crashes/restarts: a crash or notification error after claim can lose an alert, which is deliberately not replayed. Up to five individual alerts plus one summary are attempted per check. Clicking a reminder requests the local Agenda, never a server-supplied URL. Existing unread counts and Inbox contents are unaffected. Notifications already delivered can remain in macOS Notification Center after account removal.

Closing the main window keeps the process running; quitting stops this scheduler. macOS notification permissions, Focus and delivery remain OS-controlled. The notification plugin’s desktop permission methods return Granted without inspecting macOS settings; the UI therefore never presents that as verified permission. It shows sanitized agenda-refresh errors, stale-cache warnings and known failed delivery attempts, and explicitly explains the inability to confirm display. In-app read-only browsing remains available when reminders cannot run.

## Automated evidence

Store tests cover defaults, offsets, first-import suppression, unchanged refresh, moved due dates, eligibility changes, deletion, stale/future caches, sleep/backward-clock gaps, pre-sleep responses, out-of-order requests, disable/re-enable, persistent deduplication, schema-8 migration, account replacement and DST-boundary timestamps. Moodle fixture tests check unavailable/zero-item actions. Shell tests cover per-account labels, checked/disabled states, refresh/delivery warnings, French copy and account-scoped IPC arguments. These do not establish native delivery or the actual OS sleep matrix.

## Native/live qualification (pending)

- [ ] Enable each offset separately and together for three sites; restart and verify settings persist. Confirm no initial burst. Disable one site and leave the others active.
- [ ] Generate short test deadlines with a test-only timing fixture, observe a single alert and its click opening Agenda, then restart and verify no duplicate. Check the five-plus-summary bound.
- [ ] Move a due date earlier/later, delete an event and complete an action; compare Moodle 4.5/5.3 metadata and ensure completed/unavailable actions are suppressed without inventing completion states.
- [ ] Suspend around a trigger (including a request in flight), wake after over a minute, and verify no missed-alert burst. Check future reminders resume only after a fresh post-wake request. Also test short sleep, clock changes and DST.
- [ ] Hide/close the window, quit/restart, and test expired credentials, network denial, stale snapshots, notification permission denial and Focus. Confirm honest in-app errors and the absence of a quit-time delivery promise.
- [ ] Remove/replace an account while a refresh is in flight; no old-account preference, schedule, failure or new delivery should attach to the replacement account.
- [ ] Exercise English/French, keyboard and VoiceOver checkbox labels, light/dark, Increase Contrast and Reduce Motion in the installed build.
