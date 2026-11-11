# ADR 0002 — selected-file sync and shared local copies

Date: 11 November 2026. Status: implemented against fixtures; live/platform qualification open. Applies to unpublished B1 work based on `9712e37`.

## Scope and consent

B1 builds on A3: plain downloadable files from Moodle `resource` and `folder` modules, selected explicitly per account/course (1–50 files, 128 MiB per file). Moodle pages, books, packages and external resources remain unsupported; use Moodle for those. This is selected-resource sync, not an automatically expanding mirror of every course attachment. New or renamed resources require reviewing the fetched list and updating the selection.

Schedules default absent/off. Applying the first selection enables daily refresh; users can choose hourly or pause. Changing the selection replaces that course's selection and invalidates its active job, retaining every existing saved file. The UI explains the choice before it is applied. Nothing runs while the application is quit. Closing/hiding its window leaves the process and scheduler running.

## Identity and migration

Logical resource identity stays account/course-scoped module ID + raw Moodle filepath + raw filename, encoded by the existing length-delimited SHA-256 key. Source revision and section movement do not change the key. Rename changes the key; no attempt is made to infer a rename from equal filenames/sizes.

Content lookup is separately scoped by site/current account, course, exact indexed filename and SHA-256 of bytes. Both offline copies and newly fingerprinted completed browser downloads are candidates (newest 32 matching entries). A candidate is reusable only after managed-path validation and a fresh bounded hash of its local bytes. Edited, moved or unsafe copies cannot be reused. Unknown course mappings and files over the offline limit retain normal download behavior. Existing unfingerprinted download history is not backfilled by guessing.

Schema 14 adds `download_hashes` keyed to download history with cascading cleanup. Schema 15 adds `course_sync` and `sync_selection`; schema 13 freshness metadata remains intact. Account replacement/removal clears all job metadata and fingerprints through explicit account cleanup/FKs. No source URL, token, signed URL or local path is added to shell DTOs. The store lock serializes publication, shared lookup and account replacement across both transfer paths.

Shared-file reuse occurs **after transfer**: authenticated incoming bytes establish current content before comparing snapshots. It avoids duplicate published files, not duplicate network traffic. Neither filenames/sizes nor an old source address establish unchanged remote bytes. Full pre-transfer DL-3 optimization remains open. Moodle exposes file `timemodified` and revision-bearing exported file URLs ([official folder implementation](https://github.com/moodle/moodle/blob/main/public/mod/folder/lib.php)); a future metadata-based fast path needs versioned live-contract qualification before trusting those signals. Browser transfers remain WebKit-owned.

## Scheduling, checkpoints and cancellation

One worker handles one course/file at a time and shares the offline transfer lock with manual saves/removal. A successful per-file save/reuse is checkpointed; other files can fail independently. Failed files retry after 2 minutes, doubling up to the chosen interval, with a bounded failure counter. A fully successful cycle schedules from completion, avoiding immediate catch-up loops after sleep. Backward clock jumps cap the remaining delay at one chosen interval.

Interrupted `running` jobs are invalidated and become due on restart. Completed checkpoints remain; unfinished files restart from fresh descriptors and zero bytes. This deliberately does not implement HTTP Range/byte-level resume, whose authenticated source/version semantics are not established. A crash between publication and checkpoint can repeat a transfer, but hash-verified lookup reuses an intact saved copy. No partial payload is ever indexed as available.

Cancel invalidates the current job and retains its checkpoints; the next scheduled attempt stays enabled, or Refresh Now resumes sooner. Pause/reconfiguration invalidates the generation too. The worker drops an in-flight request within its 250 ms cancellation polling cadence; publication rechecks account/generation under the store lock, so stale results cannot publish after cancellation/reconfiguration. A synchronous publication that finishes before cancellation takes the lock is already completed work and remains saved. Removing a course from a successfully refreshed course list pauses its schedule.

At startup, before windows/IPC/workers, bounded non-symlink traversal removes incomplete `payload` files only from UUID-named `.moodburo-staging` directories. It skips `.trash` and published documents. Interrupted browser-download cleanup remains outside this bounded offline-payload cleanup.

## Changes, deletion and retention

Changed remote bytes or local edits produce a separate non-overwriting snapshot. The last usable copy always remains accessible. A missing/renamed/inaccessible descriptor is reported after a successful course listing; local versions are never deleted automatically. Permission/transport failures retain checkpoints and last-successful freshness. Users review selections from the current Moodle file list, with configured entries labelled.

Explicit Remove Copy previews one version and its retention policy. Intact unshared files move to a fresh private directory under `~/Moodle/.trash`; edited/unsafe files stay in place. Files still referenced by another offline entry or completed download remain in place too. Removing the offline entry does not clear download history; clearing download history does not remove offline entries/files. An enabled sync selection can save a removed copy again: pause or edit it first to prevent that.

Archives and prior versions are retained indefinitely, including across account removal. No automatic permanent deletion or app restore button is included. Finder recovery/manual cleanup is explained in the confirmation/result. Filesystem and SQLite changes are not crash-atomic; a recoverable archive can coexist with an unavailable index entry after interruption. This preserves the earlier A3 no-overwrite policy instead of implementing automatic replacement/deletion.

## Qualification and remaining boundary

B1 feature mechanisms have fixture coverage; this ADR does not close the B1 exit gate. Live Moodle 4.5/5.3 contracts, offline installed-app restart, slow/intermittent networking, actual sleep/wake, native quarantine, disk/permission faults, VoiceOver/layout and Spotlight discovery still require evidence. The full PRD OS-6 mirror and DL-3 pre-transfer bandwidth optimization remain broader targets, alongside byte-range resume if later required. See [offline QA](../offline-files-qa.md).
