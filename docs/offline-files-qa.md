# Selected offline files — A3

Implementation is covered by synthetic HTTP, filesystem, SQLite and shell tests. Live Moodle and installed macOS validation remain required. This is a manual plain-file workflow, not course mirroring.

## Implementation boundary

Courses → Files opens saved copies without a network call. Load File List explicitly calls `core_course_get_contents` with contents included. Only `resource` and `folder` plain-file entries with same-installation HTTPS `webservice/pluginfile.php` addresses are accepted; other entries are counted as unsupported. [Moodle’s file-handling API](https://moodledev.io/docs/4.5/apis/subsystems/external/files) documents token-authenticated file downloads; the service must permit file downloads.

The shell receives names, sizes and opaque SHA-256 resource keys derived from module ID, filepath and filename. Source URLs stay in Rust and are fetched again before every batch. A resource key survives source revision/section changes; renamed or deleted resources must be selected again from the new list.

Transfers are serial across the app, with at most 50 selected resources per batch, a 128 MiB per-file byte cap and a conservative 144 MiB free-space gate before each transfer. Requests time out after 180 seconds, refuse redirects, and reject HTML/JSON login/error responses. Progress events are throttled to five per second; individual outcomes appear after the batch finishes. Quitting stops transfers; there is no pause/resume. An interrupted process can leave private staging directories; automatic retention/cleanup remains later.

Files use sanitized organisation/course/section folders under `~/Moodle`, private staging and atomic non-replacing publication. macOS quarantine is applied before publication. SHA-256 compares the downloaded bytes with the newest indexed copy and its current local bytes; only an unchanged, still-present regular file is reused. Remote changes, local edits or missing/moved files cause a new copy to be published. Old files are never deleted. Repeated saved versions remain listed (latest 2,000 per course); moving a file outside its indexed path requires saving a fresh copy.

Offline records live separately from download history: clearing history does not remove them. Removing/replacing an account clears its index but keeps files on disk. The final account check, publication and index insertion share the store lock, so late old-account transfers cannot attach to the replacement account. Local paths and hashes never cross IPC. Open and Finder resolve stored copy IDs and validate all managed path components; risky/unknown file types use the existing inline confirmation.

## Manual qualification (not yet performed)

- [ ] On Moodle 4.5 and 5.3, load a course with a resource PDF, a folder with repeated filenames/subfolders, a zero-byte file, a missing size, a web page, a package and an external URL. Check supported/omitted items and access-denied copy.
- [ ] Select several files, check known/unknown sizes, transfer progress, errors, section folders, quarantine and collision names. Test a file over 128 MiB and insufficient disk space.
- [ ] Quit, disconnect the network and reopen. From Courses → Files, open/reveal saved files without loading the remote list. Verify unsupported risky types ask before opening.
- [ ] Refresh an unchanged file: keep the same indexed copy. Change the server bytes: retain the old file and save beside it. Edit, move or delete a local copy: never overwrite or remove it; save a fresh copy instead.
- [ ] Delete a server resource: Refresh Copy reports it unavailable while the saved local file still opens. Nothing propagates deletions.
- [ ] Remove/replace an account during a slow transfer: no old-account metadata or completed publication appears in the new account; existing files remain in Finder.
- [ ] Clear Downloads history and reopen saved course files; offline records remain. Test restart with a failed/partial transfer and confirm it is never offered as a completed copy.
- [ ] Check French/English, keyboard selection, inline confirmation focus/Escape, VoiceOver labels and light/dark/contrast/motion states. The original icon remains embedded in the current development bundle.
