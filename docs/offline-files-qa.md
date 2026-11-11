# Selected offline files — A3

Implementation is covered by synthetic HTTP, filesystem, SQLite and shell tests. Live Moodle and installed macOS validation remain required. This is selected plain-file sync with manual or opt-in scheduled refresh, not an automatically expanding course mirror. [ADR 0002](adr/0002-selected-file-sync.md) records the implementation policy and remaining boundary.

## Implementation boundary

Courses → Files opens saved copies without a network call. Load File List explicitly calls `core_course_get_contents` with contents included. Only `resource` and `folder` plain-file entries with same-installation HTTPS `webservice/pluginfile.php` addresses are accepted; other entries are counted as unsupported. [Moodle’s file-handling API](https://moodledev.io/docs/4.5/apis/subsystems/external/files) documents token-authenticated file downloads; the service must permit file downloads.

The shell receives names, sizes and opaque SHA-256 resource keys derived from module ID, filepath and filename. Source URLs stay in Rust and are fetched again before every batch. A resource key survives source revision/section changes; renamed or deleted resources must be selected again from the new list.

Transfers are serial across the app, with at most 50 selected resources per batch, a 128 MiB per-file byte cap and a conservative 144 MiB free-space gate before each transfer. Requests time out after 180 seconds, refuse redirects, and reject HTML/JSON login/error responses. Progress events are throttled to five per second; individual outcomes appear after the batch finishes. Quitting stops transfers. Scheduled jobs retain per-resource checkpoints; unfinished files restart from fresh descriptors. Stale offline payloads are cleaned at startup.

New publications use sanitized organisation/course/section folders under `~/Moodle`, private staging and atomic non-replacing publication. macOS quarantine is applied before publication. SHA-256 compares the downloaded bytes with the newest indexed copy and its current local bytes; only an unchanged, still-present regular file is reused. Remote changes, local edits or missing/moved files request a separate copy; an intact matching shared copy may be reused instead of publishing another file. Refresh never deletes old files. Explicit removal is described below. Repeated saved versions remain listed (latest 2,000 per course); moving a file outside its indexed path requires saving a fresh copy.

Offline records live separately from download history: clearing history does not remove them. Removing/replacing an account clears its index but keeps files on disk. The final account check, publication and index insertion share the store lock, so late old-account transfers cannot attach to the replacement account. Local paths and hashes never cross IPC. Open and Finder resolve stored copy IDs and validate all managed path components; risky/unknown file types use the existing inline confirmation.

## B1 availability and freshness follow-up

The resource chooser distinguishes available offline, saved-but-unavailable and not saved. Saved rows show last successful refresh; schema 13 preserves it across restart, advancing it only after successful publication or hash-verified reuse of the intact local copy. Historical copies initially use their save time. Availability means the indexed managed file exists, not that its bytes still match Moodle; local edits remain protected by the existing refresh policy. Failed refresh feedback remains until the next save attempt or leaving the view.

- [ ] Save a file and reload the resource list: it is available offline. Refresh unchanged bytes later: original save time and copy count stay fixed; last successful refresh advances and survives restart.
- [ ] Disconnect before refreshing, and separately force a per-file access failure: prior copies remain openable; the successful refresh date stays unchanged. A moved/deleted copy stays unavailable and cannot be opened.
- [ ] Reconnect and successfully refresh: clear the failed-refresh feedback. Check all three chooser states and long refresh labels in FR/EN at minimum width, light/dark/contrast and with VoiceOver.

## Explicit removal and retention policy

Remove Copy… acts on one saved version, after inline confirmation naming the file and organisation. Rust resolves its indexed copy ID, checks account/course identity, excludes simultaneous offline transfers and holds the store lock across filesystem handling and metadata removal. Intact hash-matching managed files move into a fresh private directory beneath `~/Moodle/.trash`; collisions and unsafe archive directories fail without removing the index. Edited or unsafe source paths stay in place and only their entry is removed. Any file referenced by another offline entry or completed download stays in place too. Missing files only lose their entry. Related freshness metadata cascades; other versions stay listed. Hashing and file operations run off the IPC thread.

Archives are retained indefinitely, including across account removal. There is no automatic purge, app restore button or immediate disk-space reclamation in this slice. Use Finder → Go → Go to Folder → `~/Moodle/.trash` to recover a file or remove archives manually. If metadata removal fails after archiving, the error identifies the archive folder; the unavailable entry can be removed again. Filesystem operations and SQLite are not a crash-atomic transaction: an interruption can leave a recoverable archive plus an unavailable entry. External file edits after the final hash check can be archived, but bytes are never permanently deleted by this action.

- [ ] Remove one of several saved versions: Cancel/Escape changes nothing; confirmation removes only that row. The unchanged file remains recoverable under `.trash` after restart.
- [ ] Edit a copy before confirming removal: its original path/content remains; the result explains it was kept. Remove a missing copy: no unrelated file changes.
- [ ] Test a locked/permission-denied source and unsafe `.trash` directory: show an error and retain the row. Remove while another offline transfer runs: refuse and allow retry afterward.
- [ ] Confirm removal in FR/EN, at minimum width beside the assistant, using keyboard and VoiceOver. Cancel receives initial focus; completion/cancellation moves focus to the saved-list heading. Confirm account replacement/removal cannot affect another account’s files.

## Shared lookup and scheduled sync qualification

Configure from Courses → Files: load the Moodle list, select up to 50 supported files, then Refresh Selected Files Daily. The schedule selector supports hourly/daily/pause. Refresh Now resumes unfinished work; Cancel ends this attempt without disabling the next scheduled one. Configured resources are labelled in the chooser. Freshness and pending/missing counts persist across restart. Shared copies require an account/course/name/hash match plus a fresh local hash; both transfer paths continue to verify incoming bytes, so this is publication deduplication rather than bandwidth deduplication.

- [ ] Enable a selection, close/hide the window, then reopen: status progresses and saved files are usable. Pause during a slow transfer; confirm no late publication after pause is accepted. Resume/change selection and verify unrelated completed versions remain untouched.
- [ ] Finish one of several files, then quit during the next. Relaunch offline: completed copies open, failed refresh leaves them in place, and only unfinished work is retried with backoff. Reconnect: the remaining resources finish; no partial file appears as available.
- [ ] Download a known-course file, then save matching bytes offline; reverse the order. Confirm one published path is used, download history/Offline Files both open it, and removal of one reference preserves the other. Edit/move the shared file and repeat: keep edits and publish a separate copy. Unknown-course/large files should retain normal behavior.
- [ ] Change server bytes, rename/delete/move files or withdraw permissions. Confirm separate versions/missing feedback, preserved prior copies and no destructive remote-deletion propagation. Review the current list to update the configured selection. New remote files must not be selected implicitly.
- [ ] Revoke credentials, replace/remove the account or remove the course while syncing: no late old-account publication/index entry; removing a course pauses its schedule. Recoverable archives and user files stay in Finder.
- [ ] Simulate disk pressure, unwritable folders, app termination between publication/checkpoint, forward/backward clock changes and sleep/wake. Verify bounded retries, usable old copies and no immediate catch-up loop. Confirm plain saved documents are discoverable by Spotlight on an installed build.
- [ ] Check FR/EN, keyboard/VoiceOver, narrow layout beside the assistant, Increase Contrast and Reduce Motion. Confirm the browser demo clearly states that it records settings without running background transfers.

## Manual qualification (not yet performed)

- [ ] On Moodle 4.5 and 5.3, load a course with a resource PDF, a folder with repeated filenames/subfolders, a zero-byte file, a missing size, a web page, a package and an external URL. Check supported/omitted items and access-denied copy.
- [ ] Select several files, check known/unknown sizes, transfer progress, errors, section folders, quarantine and collision names. Test a file over 128 MiB and insufficient disk space.
- [ ] Quit, disconnect the network and reopen. From Courses → Files, open/reveal saved files without loading the remote list. Verify unsupported risky types ask before opening.
- [ ] Refresh an unchanged file: keep the same indexed copy. Change the server bytes: retain the old file and save beside it. Edit, move or delete a local copy: never overwrite or remove it; save a fresh copy instead.
- [ ] Delete a server resource: Refresh Copy reports it unavailable while the saved local file still opens. Nothing propagates deletions.
- [ ] Remove/replace an account during a slow transfer: no old-account metadata or completed publication appears in the new account; existing files remain in Finder.
- [ ] Clear Downloads history and reopen saved course files; offline records remain. Test restart with a failed/partial transfer and confirm it is never offered as a completed copy.
- [ ] Check French/English, keyboard selection, inline confirmation focus/Escape, VoiceOver labels and light/dark/contrast/motion states. The original icon remains embedded in the current development bundle.
