# MoodBuro — implementation tracker

Last reconciled: 10 November 2026, against repository HEAD `c2361d1`.

This is the current planning checklist. The [PRD](prd.md) defines requirements; the [original Alpha plan](alpha-plan.md) and [scope expansion](alpha-feature-expansion.md) retain the baseline and acceptance criteria. [Implementation status](implementation-status.md) retains the delivery history and verification evidence. Historical week labels and six-/thirteen-week estimates are not current commitments.

**Status rule:** checked means implemented with recorded verification, not fully qualified on live Moodle or every supported Mac. Acceptance gates stay unchecked until their evidence is recorded. Record the revision, environment, result and remaining limitations when closing a gate. Keep new delivery history in implementation-status.md rather than appending it to this checklist.

## Current position

| Milestone | Implementation stocktake | Acceptance / next gate |
| --- | --- | --- |
| M0 — architecture and sign-in spike | Secure auth, session bridge and isolated webviews built; an authenticated INSP dashboard was observed | Open: repeatable three-site manual/SAML/OIDC matrix, D1–D5 results, runtime security and installed Keychain checks |
| M1 — core shell | Multi-site shell, background polling, tray/badge, links and managed downloads built | Open: native/platform qualification, menu-bar-only mode and resource measurements |
| Expanded Alpha — A1–A6 | All six bounded feature slices built with automated fixture coverage | Open: combined real-account, native, accessibility and provider qualification |
| Tester distribution | alpha.1/alpha.2 universal DMG packaging recorded; current package version is `0.1.0` | Ad-hoc signing and manual upgrades; clean-machine/Intel execution and full Alpha exit gate remain open |
| Post-alpha.2 follow-up | One-shot rate-limited cold-session recovery landed in `c2361d1` | Installed-app recovery checks open; inclusion in a subsequent installer not yet recorded |
| M2 — beta preparation | Proposed work sequence below; expanded Alpha already supplies parts of later requirements | Implement remaining sync/calendar, download/macOS features and GitHub auto-update first; qualify for beta release |
| M3 — macOS v1.0 | Later release gate in the PRD | Full requirements review, release metrics and outstanding v1.0 requirements; not implied by beta completion |

**Assessment:** Alpha feature implementation has reached the tester stage. Alpha acceptance is still open. Beta work should build on the delivered slices instead of implementing inbox, course browsing, offline selection, reminders, ICS export or the assistant again.

## Delivered Alpha scope

| Area | Implemented | Evidence / boundary |
| --- | --- | --- |
| Foundation | pnpm/React/Vite/TypeScript shell, Cargo workspace, Tauri app, Moodle/store/secrets/notifier/assistant crates and CI | `md-os` remains an extension point. CI runs frontend checks and npm high-severity audit, portable Rust tests, macOS workspace tests/build and shell IPC guard; Rust advisory auditing is still absent. |
| Shell and accounts | Generic HTTPS onboarding; native menus, FR/EN, theme/accessibility preferences; lazy site webviews, native history, Home, controlled popups, reconnect/removal | macOS 14+ uses dedicated persistent stores; macOS 12–13 shared-store cleanup remains a privacy gate. |
| Auth and security | One-use signed callbacks, token/account validation, OS credential storage, sanitized errors, bounded requests, shell-only privileged IPC | Runtime IPC denial, real SSO, installed Keychain lifecycle and full deletion remain unqualified. |
| Background and links | Per-account polling/backoff, silent seeding, persisted deduplication, tray/unread counts, dock badge, hide-on-close; validated `moodledesk://` routing | Only popup notifications generate individual alerts; conversation counts are separate. Native clicks, sleep/wake and cold-launch deep links need qualification. |
| A1 — inbox and downloads | Inbox/read actions, bounded tray subjects; searchable/paginated history, site/course/state filters, Copy Path, confirmed history clear, Return to Moodle fallback; managed course routing, PDF/image preferences, Save to Downloads and native PDF/Office readers | Return to Moodle does not retry a transfer. Existing files stay in place. Private PDFKit Save adapter/blob support remains a compatibility gate. |
| A2 — courses and Quick Open | Account-bound course cache, pins, explicit activity indexing, cache age, ⌘L and opt-in global ⌘⇧M; persistent organisation ordering | Metadata search only; no server full-text or file-content search. Global shortcut conflicts need native checks. |
| A3 — offline files | Selected resource/folder files, bounded serial downloads, offline saved-copy list, manual refresh, hashes and non-overwriting publication | No automatic course mirroring, resume, server-deletion propagation or shared sync/download deduplication. |
| A4 — agenda/reminders | Cached upcoming agenda and default-off per-account 24-hour/1-hour reminders; persisted at-most-once attempts, stale-data and wake-gap handling | No delivery while quit; permissions/Focus and actual display unqualified. Upcoming-view horizon and incomplete Moodle completion metadata remain limits. |
| A5 — calendar export | Token-free manual ICS snapshot with stable account-scoped UIDs, UTC dates, escaping/folding and Finder reveal | Independently parsed fixture; Apple Calendar import/re-import still open. No live subscription. |
| A6 — assistant | Albert/Mistral BYOK settings, streamed chat, session-only history/deletion, explicit context consent, source references, six bounded tools and individually confirmed notification-read writes | Provider/Moodle fixtures only; live providers, native geometry/focus, adversarial account/tool and Keychain checks open. No submissions or grade changes. |
| alpha.2 polish | Responsive Settings and disclosures, agenda simplification, in-app download feedback/storage guidance, native icon assets, saved main-window geometry | Narrow/light/dark/contrast/motion shell review and isolated native save/relaunch smoke recorded; full VoiceOver/live transfer validation remains open. |
| Packaging | Universal Intel/Apple Silicon app/DMG, checksums, ad-hoc signing overlay, manual Actions workflow and tester guide | Packaging evidence does not prove clean-machine install, Intel execution, notarization or updates. |
| Latest session fix | Conservative one-shot cold-restore retry after autologin rate limiting; cancels on logout/navigation/reconnect/removal and preserves destination/selection | Source and regression tests landed after alpha.2 packaging; native checks in [background QA](background-qa.md#rate-limited-startup-restoration). |

The latest recorded full suite is **101 frontend / 97 Rust tests at alpha.2**, plus lint, formatting, production frontend build, workspace Clippy and IPC checks. Those are historical verification results, not a fresh run at `c2361d1`. See [release notes](releases/v0.1.0-alpha.2.md) and [implementation evidence](implementation-status.md). The follow-up adds restoration/store regression coverage; its native behavior remains pending.

## Alpha carry-over — qualification and missing implementation

These remain Alpha obligations even when executed during beta preparation. Early tester packaging did not waive them.

### Q1 — auth, security and architecture (M0)

- [ ] Provision HTTPS Moodle 4.5/5.3 and Keycloak rigs with seeded users/courses; capture sanitized version-specific fixtures. [Rigs are currently instructions only](../test-rigs/README.md).
- [ ] Repeat manual, SAML and OIDC sign-in across three sites: cancellation, reconnect, restart, missing/expired/private tokens, network failures and rate limits, including the new one-shot recovery.
- [ ] Measure three-site warm switching against the <300 ms target; verify resize, focus, shortcuts, profile/cookie separation and titlebar/fullscreen behavior on the supported macOS matrix.
- [ ] Inject a remote-webview IPC-denial probe; verify shell/popup/native-reader privilege boundaries at runtime.
- [ ] Qualify installed Keychain access, failure/rollback, removal and identity transitions across upgrades; inspect redacted logs/metadata without exposing secrets.
- [ ] Verify asynchronous profile deletion; resolve SEC-8 shared-store/IdP cleanup on macOS 12–13 or record an explicit support/scope decision.
- [ ] Record measured D1–D5 decisions and accepted fallbacks in [ADRs](adr/0001-alpha-foundation.md). Static configuration and one real dashboard observation do not close M0.

### Q2 — native core shell and file handling (M1)

- [ ] Qualify notification delivery/click routing (D4), first-poll silence, deduplication, read-count reconciliation, tray Inbox/latest-unread items and hide/reopen/quit across three sites. [Background QA](background-qa.md).
- [ ] Measure polling sleep/wake and resource behavior; add OS lifecycle hooks if measurements require them. Verify idle memory against the original <120 MB one-site target.
- [ ] Qualify cold/warm installed deep links, unknown-site onboarding and authenticated downloads/destinations (D5). [Links/download QA](links-downloads-qa.md).
- [ ] Qualify PDF Save, blob PDFs, native Back/Forward, popup parent/close behavior, menu sizing and private-adapter compatibility. Test all PDF/image handling choices in main/popup views, embedded files, forced downloads and open-after-save. [Native reader/download QA](native-reader-downloads-qa.md).
- [ ] Validate live course-folder mapping, missing/risky-file actions, Open/Preview/Finder/Copy Path, account cleanup and history persistence.
- [ ] Implement menu-bar-only activation preference; main-window geometry persistence is already delivered in alpha.2.

### Q3 — combined expanded-feature acceptance (A1–A6)

- [ ] A1/A2: qualify real course/activity and notification APIs, denied capabilities, read-count reconciliation, cache/account replacement, ordering and warm/cold navigation on both Moodle rigs.
- [ ] A2: verify ⌘1/⌘2/⌘3/⌘L/⌥⌘L, focus from a Moodle page, native view hide/restore and global ⌘⇧M from other apps/Spaces; conflicts and release on quit.
- [ ] A3: real resource/folder contracts, permissions, quarantine, offline restart/Open, edited/moved files and account replacement. [Offline QA](offline-files-qa.md).
- [ ] A4: real action/completion metadata, deadline changes, delivery/clicks, permissions/Focus, sleep/clock/DST and stale schedules. [Reminder QA](reminders-qa.md).
- [ ] A5: import/re-import in Apple Calendar; verify DST, escaped text, snapshot copy and Finder reveal in an installed build.
- [ ] A6: both live BYOK providers, model/tool support, context-free and source-linked chat, confirmed/cancelled writes, prompt injection, stop/retry, key lifecycle and account isolation. [Assistant QA](assistant-qa.md).
- [ ] Run combined FR/EN, keyboard/VoiceOver, Reduce Motion/Increase Contrast, panel geometry and narrow-window checks in the native app.

### Q4 — distribution and Alpha gate review

- [x] Universal DMG packaging, architecture/signature/checksum verification, manual packaging workflow and [tester installation guide](alpha-install.md).
- [ ] Qualify clean-machine installation and actual Intel execution across the supported macOS matrix; distinguish the macOS 12 binary minimum from tested support.
- [ ] Add rotating/redacted diagnostic logging; configure Rust advisory auditing and resolve outstanding dependency findings. The npm audit CI step is already present.
- [x] Supersede the original stable self-signed Alpha identity/updater sequence: GitHub automatic updates are beta scope under B4; Apple Developer ID signing/notarization is post-beta. This records the scope decision only; update implementation remains unchecked. Current Alpha testers use ad-hoc builds/manual replacement.
- [ ] Run bug bash and review every [original exit criterion](alpha-plan.md#1-scope-team-and-exit-criteria): zero P0s, ≥99% crash-free sessions over the last test week, live/native/security and installation evidence. Record any accepted scope changes; no Alpha completion claim yet.

## Proposed beta implementation sequence (M2)

The PRD retains M3 as the macOS v1.0 gate, but its embedded roadmap does not supply a text M2 task breakdown. **B1–B5 below organize beta preparation**, derived from remaining PRD requirements. Feature-first sequencing, GitHub automatic updates and post-beta Apple signed distribution reflect the product owner’s instructions; individual feature details remain planning targets, not estimates or completed gates. Re-estimate each after its spike. Independent engineering can proceed alongside account/platform qualification; qualification must finish before claiming beta readiness.

### B1 — course sync and shared file identity

- [ ] Spike the existing A3 index/downloader against OS-6/OS-7 and DL-2/DL-3; define stable file identity, shared sync/download lookup and schema migration before adding another transfer path.
- [ ] Add explicit per-course sync selection, scheduled refresh, bounded concurrency/backoff and visible status/cancellation.
- [ ] Define and implement restart/resume, server rename/deletion, local-edit conflicts and `.trash/` retention semantics. Preview destructive effects and protect local edits; A3 currently saves alongside existing files.
- [ ] Reuse unchanged local files for browsing downloads and refresh changed versions under the agreed policy; retain safe endpoint, quarantine and account-isolation boundaries.
- [ ] Verify offline restart, interrupted transfers, permission loss, disk pressure, account replacement and Spotlight discovery on installed builds.

**Dependencies:** existing course/offline caches; qualify file contracts while building. **Exit:** selected courses refresh predictably without duplicate transfers or local data loss, including recovery across restart. Full sync must not be marked done based on manual A3 refresh.

### B2 — live calendar integration

- [ ] Resolve the PRD Q6 export-token/capability question and safe credential handling before constructing any subscription URL.
- [ ] Implement supported Apple Calendar subscription/refresh (OS-8/MAC-5), with explicit consent, permission errors and account removal/revocation behavior; retain manual ICS fallback.
- [ ] Reconcile upcoming-view coverage with the full reminder requirement; define handling of missing completion/action data and scheduling limitations.
- [ ] Qualify subscription refresh, time zones/DST, changed/deleted events, token revocation and duplicate prevention in Calendar.

**Dependencies:** A4/A5 event/export model and subscription token spike; native qualification runs alongside implementation. **Exit:** a qualified live integration with clear refresh/revocation behavior, or an explicit scope decision retaining snapshot-only support.

### B3 — download and macOS workflows

- [ ] Select and implement the beta subset of DL-4/DL-5/DL-6/DL-8/DL-11: cancel/pause/resume/fresh-source retry, native completion actions/progress, metadata and separate cleanup controls. Reuse B1 transfer identity rather than replaying stale authenticated URLs.
- [ ] Reconcile expanded history/Where From requirements with the current deliberate omission of source URLs; document privacy/security policy before persisting new metadata.
- [ ] Complete menu-bar-only mode if still open, optional start-at-login (OS-14) and offered Finder sidebar integration (MAC-4); qualify permissions, relaunch and disabled defaults.
- [ ] Decide whether upload/submission drafts (OS-10) belong in beta or the M3 backlog; if included, spike upload/save-submission contracts and define exact-target confirmation and duplicate/failure handling first.

**Exit:** the selected beta subset is explicit, implemented and qualified; all remaining v1.0 requirements stay visible for M3. Download retention (DL-9), Finder Services (MAC-7), MDM (MAC-10), dock recent-course menus and other outstanding PRD surfaces need an explicit release assignment before M3 scope is closed.

### B4 — automatic updates through GitHub Releases

Requested beta scope: discover and install updates distributed through GitHub Releases. This is distinct from Apple Developer ID signing/notarization, which is deferred until after beta.

- [ ] Extend the existing universal macOS packaging workflow to produce updater-compatible artifacts, cryptographic update signatures, checksums and a versioned beta manifest alongside the DMG; publish them to GitHub Releases.
- [ ] Configure the beta update feed, release/prerelease selection and version rules. Keep beta and future stable channels explicit and avoid downgrades or incompatible-platform offers.
- [ ] Resolve authenticated access for the existing private repository: select a secure delivery/access model without bundling a personal GitHub token or making the repository public implicitly. Record the decision before wiring the feed.
- [ ] Implement automatic update discovery with available-version/release-note feedback, download progress, install/relaunch controls, failure/retry states and an update-disable preference/policy. State the check cadence and installation consent behavior explicitly.
- [ ] Store update-signing secrets in CI and verification material in the app; reject altered or unsigned update artifacts. Update signatures remain required even while the app itself uses ad-hoc signing.
- [ ] Test an actual installed beta→beta update on both architectures, including data/settings/files preservation, credential access, failed/interrupted downloads and invalid-signature rejection. Document manual installation of the first updater-enabled build for Alpha testers; do not claim existing alpha.2 can update itself.

**Dependencies:** existing universal packaging, GitHub release access decision and update keys. **Exit:** an installed beta discovers, verifies and installs a newer beta from GitHub Releases, with recoverable failures and no embedded repository credentials. Apple signing/notarization is not a prerequisite for implementing this feature; qualify Gatekeeper/Keychain behavior for the chosen beta identity.

### B5 — baseline qualification and beta release gate

Run independent checks alongside B1–B4; qualification is a release gate, not a blanket prerequisite to starting beta features.

- [ ] Execute Q1–Q4, record results by revision/platform/site, and fix failures in the affected subsystem. Apply the recorded scope decision: B4 replaces the old Alpha updater sequence, and Apple signed distribution stays post-beta.
- [ ] Decide the supported macOS/Moodle matrix using actual results, including legacy data cleanup and the private PDF Save adapter fallback.
- [ ] Finish dependency-audit enforcement and rotating/redacted diagnostics; retain useful bug-report evidence without secrets.
- [ ] Confirm full requirement gaps: partial A1–A6 coverage does not satisfy full OS/MAC/DL requirements. Keep exceptions explicit in the PRD/ADRs.
- [ ] Run combined beta feature, update, clean-machine installation, account/security and accessibility qualification; update tester instructions for beta's ad-hoc distribution and automatic-update limitations.

**Exit:** implemented beta features meet their acceptance checks; repeatable baseline scripts/fixtures and measured D1–D5 decisions exist; no unresolved P0; all remaining Alpha obligations have an explicit disposition. Developer ID signing/notarization remains post-beta.

## Post-beta — Apple signed distribution (toward M3)

Explicit sequencing decision: **defer Developer ID signing, hardened-runtime distribution qualification, notarization and stapling until after beta.** Beta uses the existing ad-hoc distribution approach and Gatekeeper guidance, with cryptographically signed update artifacts under B4.

- [ ] Configure Developer ID, hardened runtime, notarization/stapling and secure Apple CI credentials (MAC-9).
- [ ] Qualify the ad-hoc beta→Developer ID transition, including installed upgrades, Keychain identity/prompts, data migrations and recovery.
- [ ] Verify clean-machine signed installation and updates on both architectures; revise installer/release guidance.

**Exit:** verified Apple signed/notarized distribution and update continuity before claiming the full MAC-9/v1.0 release requirement. This work does not block beta feature implementation or the beta release gate.

## Next implementation starting point

1. Start B1 course sync/shared file identity; retain the delivered A3 file-safety guarantees.
2. Spike B2 calendar-token feasibility and implement B3 download/macOS workflows in bounded slices.
3. Implement B4 GitHub automatic updates, resolving private-release access and update-signing setup before end-to-end installation tests.
4. Run independent Q1–Q4 checks alongside feature work, then close B5 for beta release. Estimate feature slices and map remaining v1.0 requirements to M2 or M3 before committing to a beta date.
5. After beta, implement and qualify Apple signed distribution for M3.

Windows v1.1 and Linux v1.2 remain later platform work. Broad autonomous AI actions, assignment submissions through the assistant, grade changes and file-content search are not additions to this beta proposal.

## Evidence index

- [Implementation history and verification](implementation-status.md)
- [Alpha baseline, exit criteria and D1–D5](alpha-plan.md)
- [Expanded Alpha boundaries and acceptance criteria](alpha-feature-expansion.md)
- [Authentication review](auth-review.md) and [architecture decisions](adr/0001-alpha-foundation.md)
- [Background QA](background-qa.md), [links/downloads QA](links-downloads-qa.md), [native reader QA](native-reader-downloads-qa.md)
- [Offline QA](offline-files-qa.md), [reminder QA](reminders-qa.md), [assistant QA](assistant-qa.md)
- [alpha.1 release notes](releases/v0.1.0-alpha.1.md), [alpha.2 release notes](releases/v0.1.0-alpha.2.md) and [installation guide](alpha-install.md)
