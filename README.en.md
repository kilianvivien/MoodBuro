<p align="center"><img src="assets/app-icon.png" width="112" alt="MoodBuro icon" /></p>
<h1 align="center">MoodBuro</h1>
<p align="center"><strong>Your Moodle, at home on your Mac.</strong><br/>A lightweight desktop home for your organisation’s Moodle.</p>
<p align="center"><a href="README.md">Français</a> · English</p>

MoodBuro hosts the **real Moodle site** in a macOS application, keeping your organisations, courses and tools close at hand. Enter your organisation’s Moodle address and sign in on its own page.

## Download and install

**[Download MoodBuro 0.3.0 for macOS](https://github.com/kilianvivien/MoodBuro/releases/latest)**

The universal DMG includes **Apple Silicon and Intel** binaries, with macOS 12 as the configured minimum. Download the DMG and `SHA256SUMS.txt`, verify the checksum, then drag MoodBuro into Applications. Quit the app before replacing it during an upgrade.

Version 0.3.0 is **ad-hoc signed and not notarized**. macOS may require first-launch approval in System Settings → Privacy & Security. From version 0.2.0, check for the update in Settings → General. For a first installation or an earlier version, install the DMG manually.

## What’s new in 0.3.0

- **InspCal timetables for INSP’s Moodle**: open a supported PDF from the Moodle context menu or Downloads, then find saved weeks in Agenda, even offline.
- **Week and list views**, with rooms, speakers, links and group or attendance choices. Review changes before saving a corrected week.
- **Calendar sync** to a dedicated “Emploi du temps” calendar, plus ICS export. InspCal settings let you choose the cohort, event prefix and exported reminders.

## Your courses, at home on the Mac

- **Multiple Moodle organisations**, each with its own account, editable abbreviation and, on macOS 14+, a separate persistent WebKit profile.
- **Quiet native chrome**, real window controls, native menus, a compact toolbar and remembered window size and position.
- **French and English**, automatic or explicit language selection, automatic/light/dark appearance, and contrast and motion preferences.
- **Quick Open with ⌘K**, to find courses, activities, organisations and local views over the current page.

| Feature | What you can do |
| --- | --- |
| Bookmarks | Save a Moodle page with ⌘D, rename it and choose from eight icons. Five bookmarks appear in the sidebar; additional entries use a native overflow menu. |
| Notifications | Read the inbox, manage read state and see counts in the sidebar, Dock and menu bar. |
| Courses and agenda | Find courses, activities and deadlines, pin courses and export the agenda as ICS. |
| Reminders | Enable local reminders per organisation or choose a reminder for a deadline. Reminders default off. |
| Downloads | Track transfers, search history, open files or reveal them in Finder, with organisation/course routing when the course is identified. |
| Offline files | Select course files to keep on the Mac and refresh them manually, preserving local edits and previous versions. |
| Document readers | View supported PDF and Office documents in native reader windows. |
| Optional AI assistant | Use Albert API or Mistral AI with your own key, attach Moodle context explicitly, follow sources and confirm proposed actions. |

The assistant can find courses, activities, notifications, files and deadlines. Actions that change data or save a file require confirmation. Conversations stay in memory until quit.

## Useful shortcuts

| Action | Shortcut |
| --- | --- |
| Quick Open | ⌘K |
| Bookmark this page | ⌘D |
| AI assistant | ⇧⌘A |
| Notifications · Courses · Agenda | ⌘1 · ⌘2 · ⌘3 |
| Settings | ⌘, |
| Quit | ⌘Q |

Closing the window hides it while notification polling and reminders keep running. **Quit** stops the app. If Moodle temporarily limits automatic sign-in after a restart, the toolbar explains the wait and shows the next retry time.

## Credentials and local data

MoodBuro never draws a Moodle password form. Tokens and API keys stay in Rust and the OS credential store, never in SQLite or the local UI. On macOS they share one Keychain item and one startup read. The first migration can still prompt for each old item.

Remote Moodle and login pages have no privileged IPC access. Downloads are restricted to the allowed HTTPS origin. Bookmarks are local and account-bound; removing or replacing the account clears them.

Removing an organisation deletes its credentials and metadata and requests isolated-profile removal. Downloaded files remain. WebKit deletion is asynchronous; some shared browser data and SSO sessions can remain on older macOS versions.

## Develop and verify

Install Node 22+, pnpm 9.15.9, Rust and Xcode tools. The Rust compiler is pinned in `rust-toolchain.toml`.

```sh
git clone https://github.com/kilianvivien/MoodBuro.git
cd MoodBuro
pnpm install --frozen-lockfile
pnpm tauri dev
```

`pnpm dev` runs the frontend alone. Real authentication and secure storage require Tauri. The demo at `http://127.0.0.1:1420/?mac&demo` uses fictional in-memory data without a real Moodle session.

```sh
pnpm lint
pnpm format:check
pnpm test
pnpm build
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/check-security.py
```

Version 0.1.0 passed **109 frontend and 112 Rust tests**, plus formatting, builds, Clippy and IPC permission checks. Automated tests and synthetic HTTP contracts do not replace real-account and platform qualification.

## Architecture

`apps/shell` contains React, TypeScript, Vite and Zustand. `src-tauri` owns native windows, menus and IPC. The Rust crates cover Moodle protocol/HTTP (`md-moodle`), OS credentials (`md-secrets`), SQLite metadata (`md-store`), AI providers and controlled tools (`md-assistant`), native InspCal/PDFium timetable reading (`md-inspcal`), polling decisions (`md-notifier`) and system/file operations (`md-os`).

## Compatibility and next steps

The current distribution targets macOS. Full macOS-version, Intel runtime, SSO, native notification and accessibility qualification remains open, alongside Developer ID signing, notarization and installed update qualification. Windows and Linux remain later targets.

The native PDF Save adapter uses a private WebKit callback and requires macOS compatibility qualification. MoodBuro is generic: no organisation is preconfigured. This project is independent of Moodle HQ.


## License

[MIT](LICENSE) — © 2026 Kilian Vivien.

## Update releases (maintainers)

The macOS packaging workflow builds signed universal updater archives and publishes normal releases from `vMAJOR.MINOR.PATCH` tags. Matching `-beta.N` tags remain prereleases on a separate beta feed. Keep the workspace Cargo manifest, Tauri config and both package manifests at the same version and refresh lockfiles before tagging.

Actions uses the `TAURI_SIGNING_PRIVATE_KEY` secret and `MOODBURO_UPDATE_PUBLIC_KEY` variable. Normal builds fetch `latest.json` from the latest normal GitHub release and accept only newer normal versions. Beta builds stay on `beta-feed/beta.json`. Install the first updater-enabled version (0.2.0) manually. Settings → General supports launch/six-hour checks, opt-out, release notes, progress, explicit install and relaunch.

Developer ID signing/notarization and installed Intel/Apple Silicon update qualification remain open. Keep the signing-key backup safe: changing the verification key requires an update signed by the old key first. Release assets are uploaded to a draft before publication; a failed publish step can be recovered after inspecting the draft and its artifacts.
