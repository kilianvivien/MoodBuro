# Deep links and download QA

Keep implementation checks separate from observed native/platform results. Record revision, macOS/Moodle versions and outcomes. Never paste real token URLs, callback URLs, private subjects or credentials into test reports.

## Deep links

Use an installed app bundle. `tauri dev` is insufficient to validate LaunchServices registration.

1. Open `moodledesk://new.example.edu/course/view.php?id=4` with no corresponding saved site. Expect onboarding with `https://new.example.edu/` prefilled and no automatic sign-in/network discovery. For a subdirectory installation, edit the base URL before continuing.
2. Repeat while the app is hidden and quit. Confirm one running instance and that the initial URL is not lost before shell initialization. Forward another URL from a second process; the newest queued request should appear.
3. With disposable saved sites, open course links for each. Verify correct site/path/query/fragment, including a port and a subdirectory root. Confirm a cold session finishes autologin before displaying the requested page.
4. Expire a test token. Opening a link should show a sanitized error in Settings; reconnect should retry the retained destination. A newer link must take precedence.
5. Try user-info, `token`/`wstoken`/`key`/`sesskey` parameters (including percent-encoded keys), sibling installation paths, encoded separators, traversal, wrong schemes and oversized input. Expect rejection and no URL or credential in logs/storage. Unknown hosts must never be added automatically.
6. On the supported OS matrix, test native single-instance launch/exit/relaunch and Dock reopen. Validate the app-menu shortcut/focus/resize matrix after navigation.

## Downloads

Use session-authenticated Moodle test files, not a real institutional account for synthetic negative cases.

1. Download PDF, DOCX, MBZ and DMG files through Moodle. Verify they land in `~/Moodle/<site-name>-<uuid>/Downloads/`, not WebKit's default folder. Check private staging files are removed after normal completion.
2. Download different URLs with identical suggested filenames concurrently. Verify both complete under distinct names and existing files remain byte-for-byte intact. Include Unicode, path separators, dotfiles, control/bidi characters and long names.
3. Request the exact same URL while its first transfer is active. The duplicate must be denied. This URL stays reserved until restart because macOS exposes no per-transfer identity; a late rejected-request callback must never publish a later transfer. After restart, retry normally.
4. Verify recent list state at start/finish/failure and the 20-row display limit. Check toolbar, View menu and tray entry. Interrupt a transfer by quitting; on restart its row must show failed. Partial staging files from interruption are retained privately and never offered for opening.
5. Open a PDF explicitly. Open DMG, app/executable, script, unknown extension and extensionless files: require inline confirmation first, focus Cancel by default, and keep Escape within the confirmation. Never auto-open on completion. Show in Finder must not launch the file.
6. Delete/move a completed file or replace it with a symlink outside `~/Moodle`; actions must refuse. Replace the root folder with a symlink in a disposable test home; download routing/open must refuse. Verify macOS quarantine metadata survives hard-link publication before accepting the risky-open gate.
7. Remove/reconnect an organisation while downloading. Its old rows must not return; changing Moodle users clears history while retaining previously downloaded files. A late old-user completion must not publish or notify as the new user.
8. Check external-origin and non-HTTPS download requests are blocked. Validate forced downloads, Content-Disposition, interrupted connections, redirects and WebKit completion callbacks (D5). Verify the original-request URL matches start/finish on macOS.
9. Check French/English, dark/light, Increase Contrast, keyboard navigation and VoiceOver labels. Repeated Open/Reveal operations must remain disabled while an OS action is pending.

## Current limits

All files use the per-site Downloads fallback; course discovery/context mapping is not implemented. No byte progress, cancellation, resume, automatic deduplication or full file manager is claimed. Same-URL overlaps require restart before retry. Filesystem publication relies on same-volume hard links; unsupported filesystems report failure. Notifications and native OS open/reveal are subject to platform behavior. Full installed-app testing on macOS 12/15/current and real Moodle SSO/download sessions remains a release gate.

## Observed in this implementation slice

A uniquely identified local QA bundle showed French/dark onboarding with the HTTPS origin prefilled from its startup URL argument. Automatic sign-in was not triggered. Subsequent UI inspection was stopped by approval review when the computer-use tool displayed a signed-in session after changing its binding. This observation does not validate authenticated routing, the Downloads screen, single-instance forwarding or native transfer completion. Those checks stay open.

## PDF and popup regression cases

1. In a Moodle course, open a PDF inline. Main toolbar Home must always return to the site's dashboard, even if there is no earlier history. Back must use native history and return from the PDF without depending on JavaScript execution in the document. Blank bootstrap and one-use login entries must be skipped, not replayed.
2. Open a course resource/quiz/document through a popup. The parent remains intact; a local popup toolbar appears above the content. Back/Forward follow that window's history; at its first entry, Back to Moodle closes it and restores the originating site, including when another site was selected in the main window. Home closes it and opens the parent dashboard.
3. Verify popup opener messaging, `window.close()`, POST targets, blank-then-written windows and three-window limit. External HTTPS targets open in the default browser; unsafe schemes and credential-bearing URLs are denied. Same-origin blob PDFs are accepted; cross-origin blobs are rejected. Reconnect/remove closes site popups.
4. Click the PDF viewer's own Save/download control. Expect a managed history row and a saved PDF with quarantine metadata, including annotations if WebKit supplies them. The delegate proxy uses WebKit's private save-data callback, so verify its availability on macOS 12/15/current. Exercise normal JavaScript dialogs, popup creation, file chooser and permissions after proxy installation to detect forwarding regressions.
5. The direct PDF path accepts PDF MIME/header data up to 128 MiB; larger/unsupported exports should show a translated error and advise Moodle's ordinary file link. Test duplicate names, overlapping requests, reconnect during save and file quarantine/collision behavior.
6. Inspect the menu bar with a very long organisation name and a saved abbreviation. Main menu width stays bounded; the organisation submenu opens the saved site or Home. Recent Downloads lists five short filenames, shows in-progress/failed status, refreshes on completion and reveals completed items without opening them.
7. Inject the remote IPC-denial probe into popup content as well as ordinary site/auth views. The only added grants belong to the local `chrome-popup-*` toolbar and are limited to its context/navigation commands.
