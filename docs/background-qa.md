# Background notifications: installed-app QA

Implementation is ready for these tests; none of the live outcomes below is claimed. Run on each supported macOS generation and record app revision, macOS version, Moodle version and pass/fail. Use test accounts and sanitized IDs/counts only; never attach tokens, Keychain values, callback URLs or private message bodies.

## Repeatable account/session checks (M0)

1. Add an HTTPS test organisation, complete its own login page, and confirm the dashboard. Close a second login attempt to check cancellation leaves the original account usable.
2. Quit and reopen the installed app. Confirm the same account and browser session restore. Repeat with a missing private token, autologin rate limiting, expired token and network unavailable, using a disposable test account and administrator-controlled rig.
3. Reconnect the same account. Confirm no duplicate site and no burst of old notifications. Reconnect as a different user: the notification seed must reset silently.
4. Remove the account. Confirm metadata/notification rows and credentials disappear, the tray/badge no longer include it, and an in-flight poll cannot recreate it. Verify asynchronous profile removal separately.
5. Repeat sign-in/cancellation/restart across manual, SAML and OIDC sites. Test three simultaneous sites and record warm-switch timings, resize/focus and profile isolation results.

## Notification and lifecycle checks (W4)

1. Install the local `.app` in Applications and launch it. Enable its notifications in macOS Settings if necessary. Connect a test site with old unread items: first poll must update sidebar/tray/dock counts without posting historical alerts.
2. Create a new Moodle notification. Within one 120-second interval, confirm one native alert and updated aggregate counts. Click Check Now repeatedly: requests should coalesce and alerts must not duplicate.
3. Read one old item while creating a new one, leaving the total unchanged. The new item must still alert. Create seven notifications together: at most five individual alerts and one summary should appear.
4. Read items in Moodle. Check Now must lower or clear counts across all three surfaces. The aggregate includes unread conversations plus popup notifications; only popup subjects produce individual native alerts in this slice.
5. Quit/reopen with the same account and unchanged notifications: no duplicate alerts. Create a new notification while the app is quit; reopening should discover it using the persisted checkpoint (within the newest-20 window).
6. Click a delivered notification with the app hidden. Verify it reveals the correct site’s notifications page. Repeat across three sites and after removing the originating account. Record whether the OS/plugin actually reports the click; delivery while quit remains unqualified.
7. Close the window. Confirm the process remains, polls still arrive and tray Open, tray Settings and Dock reopen work. Quit from the tray and Cmd+Q must terminate it. Switch shell language and verify tray labels update.
8. Disable networking: after a failed poll, the sidebar/tray should indicate offline and retain last-known counts. Confirm intervals back off to at most 15 minutes, then recover. Invalidate the test token server-side: show reconnect state and stop requests until reconnect/restart.
9. Sleep longer than two intervals, wake and observe request timestamps: no catch-up burst or duplicate alerts. Record time to next poll; immediate wake polling is not implemented. Test Focus on/off and notification permissions denied.
10. Measure one-site idle memory and three-site switching while polling. Record accessibility, light/dark, inactive-window and Increase Contrast checks for the new status badges.

## Limits to keep visible

No automatic read mutation is exposed to the shell. Poll interval/preferences, menu-bar-only mode, window-state persistence and the tray’s latest-unread submenu remain future work. Native delivery is subject to OS policy. Alert history is bounded: the worker examines the newest 20 popup records and stores at most 100 subjects/read flags per site. A crash after checkpoint commit but before native delivery can lose that alert; the unread count remains available. Signing, distribution and the complete SSO/platform matrix remain separate release gates.

Protocol reference: [Moodle 4.5 popup API](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/message/output/popup/externallib.php) and [message API](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/message/externallib.php). Current contracts are synthetic, not authenticated recordings.
