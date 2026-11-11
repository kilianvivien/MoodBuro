# B3 — downloads, Mac workflows and assignment drafts

Implementation and fixture evidence: 11 November 2026. This is a bounded beta implementation; the installed-app/live-Moodle qualification gate remains open.

## Selected beta scope

| Area | Implemented | Remaining qualification or scope |
| --- | --- | --- |
| Downloads | Public WKDownload progress, cancel, aggregate Dock progress; fresh-source retry through Return to Moodle | Installed WebKit/version matrix and completion/cancel races. Pause and byte-range resume remain deferred. |
| Completion | Native notification actions for Open and Show in Finder; tapping opens Downloads | Installed notification permissions, action delivery and relaunch. Risky types still require confirmation in Downloads. |
| Metadata | Size, fixed course/site source page, Finder Where From and course tag on new files | Activity-specific source attribution remains deferred. Existing files are not backfilled. |
| Cleanup | Per-file confirmation, verified intact files archived in `~/Moodle/.trash`, independent history clearing | Site-removal keep/delete choices (full DL-11) and automatic retention remain deferred. |
| Mac | Menu-bar-only mode, opt-in launch at login, offered Finder favourite workflow | Installed relaunch/login/Dock qualification; Finder favourite is a user drag, not an automatic sidebar mutation. |
| Uploads | Native picker/window drop/Dock-open intake, explicit assignment draft confirmation, durable uncertain-result handling | Live sandbox assignment, permissions/version matrix and native file-drop qualification. No final submission feature. |

Cancelled or ambiguous browser download URLs retain a reservation until restart. This prevents late URL-only WebKit callbacks from finishing a later transfer. A new Moodle-issued URL can start normally; requesting the exact reserved URL reports that MoodBuro must restart. No authenticated download URL is replayed by the shell.

## Metadata and cleanup policy

Only a fixed `course/view.php?id=…` URL, or `my/` when the course is unknown, is retained as the source page. This identifies the organisation/course and is intentionally visible in Finder metadata. “Public” here means credential-free navigation, not publicly readable course content. Actual transfer URLs, query tokens, cookies, referrers and arbitrary Moodle-supplied source URLs are excluded.

Metadata is best effort and applied only to newly published files; reused offline copies retain their metadata. Failed annotation is reported on the row. Quarantine is neither removed nor replaced. Mac fixture tests independently decode both property lists with `plutil` and verify an existing quarantine attribute survives.

History clearing only deletes index entries. File removal checks account-owned history, shared offline/download references, managed-path boundaries and the saved byte hash. Edited, unsafe, shared or uncheckable files are kept. Hash checking is bounded to 128 MiB; larger files use Finder removal. Archives are retained indefinitely and can be revealed in Finder; removal does not immediately free disk space. Clearing history afterwards also removes its archive pointer, without deleting the archive. No automatic archive purge is enabled.

## Moodle draft contract and confirmation

The stock Moodle 4.5 source was inspected before adding writes:

- [Assignment external API](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/mod/assign/externallib.php): assignment configuration, submission status, `mod_assign_save_submission` and warning results.
- [Assignment save logic](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/mod/assign/locallib.php): without `submissiondrafts`, saving may mark work submitted. Such assignments are rejected.
- [Draft upload endpoint](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/webservice/upload.php): multipart POST to `webservice/upload.php`, token in the body, new user draft item and returned file/owner metadata.

One nonempty regular file, up to 32 MiB and the assignment's lower limit, is supported. The assignment must explicitly use individual submissions, file-only submission plugins and the separate final-submit/draft setting. Existing work, submitted work, locked drafts, group work, online text and unknown metadata lead back to Moodle. This is deliberately narrower than generic Moodle submission editing.

A file path is granted by the OS picker or a native file-drop/open event. JavaScript receives an opaque file ID, filename and size. It cannot supply a filesystem path. Grants expire after 30 minutes; at most 20 are retained. The confirmation names the file, size, organisation, course and assignment. Its single-use approval expires after five minutes and binds account IDs, activity ID and a SHA-256 snapshot. The file is re-read and checked after approval, before sending its bytes.

The current API identity, assignment configuration and empty/editable status are fetched again before the first write; the draft status is checked again after upload, before attachment. Local account changes are serialized with the upload. These checks do not provide atomic compare-and-swap against another browser editing Moodle concurrently; that race remains a qualification concern and is why existing work is not offered for replacement.

Both write requests refuse redirects and have no application retries. The upload response must identify exactly one matching user-owned draft file. `mod_assign_save_submission` attaches that item; success requires an empty warning list and a subsequent status showing a **draft** with the expected filename/size. There is no final-submit endpoint in this flow. Review and final submission happen in Moodle.

Schema 17 adds an account-bound operation journal; schema 18 adds download metadata. The journal contains target identifiers, filename, size, hash, timestamps and state, without paths, tokens or contents. The hash is omitted from IPC. Interrupted pending operations become uncertain at startup and are never replayed. Stop, timeout, verification failure or a dropped response conservatively blocks a new attempt for the same assignment until the user checks Moodle and explicitly acknowledges it. This acknowledgement only changes local retry eligibility; every new write still needs a new file/target review and server checks. It cannot undo a request Moodle already received.

## Settings and provider help

Settings is grouped into General, Files, Reminders, AI Assistant and Organisations. Category buttons use existing tokens, Lucide icons, roving keyboard focus and one visible panel; hidden panels preserve in-progress local state. Mac preferences sit in General. Empty Reminders shows an Add Organisation action rather than a blank panel. Full context-sharing information remains in a disclosure.

The model selector loads the provider's model list automatically with a saved key, resets on provider/key changes, and ignores stale requests. It only offers returned identifiers; no free-text model input or extra saved-model option is injected. The help icon opens a small native HTML dialog with three provider-specific steps, fixed official links, focus containment and Escape/Done restoration.

Help references checked on 11 November 2026: [Albert access and ProConnect](https://guides.ia.numerique.gouv.fr/albert-api), [Albert portal](https://albert.playground.etalab.gouv.fr/), [Mistral key creation](https://docs.mistral.ai/getting-started/quickstarts/studio/activate-and-generate-api-key), [Mistral key portal](https://console.mistral.ai/api-keys). The app does not obtain or generate provider keys for the user.

## Evidence and remaining acceptance

- 131 frontend and 141 Rust tests pass. Frontend lint/format/build, Rust format, workspace Clippy with warnings denied, IPC capability/caller checks and diff whitespace checks pass.
- Synthetic HTTP fixtures cover multipart body credentials, returned owner/file validation, redirect refusal, no write retries, server warnings and draft-only status/configuration policy. SQLite/file fixtures cover restart uncertainty, account replacement/removal, shared references, archive safety, changed bytes and symlink rejection.
- Browser demo exercised provider switching/model refresh, Albert/Mistral help, keyboard focus/Escape, FR/dark at 700px, upload destination review, saved and stopped/uncertain feedback. At 700px with the assistant open, main content is 320px and upload confirmation is 272px, without document overflow. These are mock writes, not production Moodle writes.
- Full-width screenshots were inspected. Narrow screenshot capture timed out after resize; narrow evidence is DOM geometry/interaction, not a complete visual acceptance claim. See [Settings review](settings-qa.md).
- Still required: installed picker/window/Dock intake, actual sandbox draft save and cancellation, server-side permissions and size/type limits, concurrent external edits, account changes during writes, login enable/disable and hidden launch, Dock/tray behavior, Finder favourites/recovery, native notifications, VoiceOver and supported macOS/Moodle versions. No real assignment, login item or Finder favourite was changed during this work.

Full DL-4 pause/resume, expanded DL-8 activity attribution, full DL-11 site cleanup, DL-9 retention, Finder Services, Dock recent-course menus and MDM remain visible v1.0 work. This batch does not close M3 or the B3 qualification gate.
