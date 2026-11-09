# Alpha implementation status

## Delivered

The first two implementation slices cover the workspace/CI foundation, organisation-URL onboarding, the macOS shell and settings, account lifecycle and session-restoration infrastructure. This does **not** close M0 or claim the entire W1/W2 plan. `md-notifier` and `md-os` remain extension points.

- Generic organisation URL onboarding, confirmed by the product owner; no institution is hard-coded.
- Public Moodle discovery, same-origin HTTPS redirects and `/public/` fallback. First integration target: `https://moodle.insp.gouv.fr/`; its public mobile config was checked without credentials.
- Dedicated login window, one-use signed callbacks, server-side token verification, credential rollback on failed persistence, reconnect, and cancellation guards against late callbacks.
- Per-site persistent WebKit profiles on macOS 14+, shared by auth and Moodle views. Older macOS retains the shared-store fallback.
- Cold-view restoration through the token/private-token autologin API; warm switches reuse the existing view. Missing/expired credentials ask for reconnect. A rate-limited/unavailable autologin key falls back to the existing browser session.
- HTTP client with sanitized typed errors, 15-second timeout, four in-flight calls per client, bounded read retries and jitter. Key minting is never retried automatically. Typed site-info, autologin and unread-count wrappers.
- Native macOS titlebar/menus, Liquid Glass material where supported with AppKit fallback, system typography, focused icons, keyboard navigation and light/dark/automatic appearance. French/English UI, automatic language selection and explicit language preferences are supported from onboarding, including native menus. Settings persist in SQLite and apply to the native windows.
- Account removal asks for confirmation, requests dedicated-profile data deletion, and removes Keychain credentials and metadata. WebKit deletion is asynchronous. Shared legacy cookies remain if another saved site uses them; legacy local storage/shared IdP sessions may remain on the fallback. Full SEC-8 deletion on macOS 12–13 is still open.

## Verification

- 41 UI/store/reducer/helper/localization tests; production frontend build, ESLint and Prettier pass.
- 17 Rust tests (including mock HTTP contracts, URL/callback policy, SQLite migration/settings/reconnect, cookie domain boundaries); workspace Clippy with warnings denied passes.
- Native debug build/ad-hoc app packaging and shell-only IPC configuration/caller regression checks pass. Generated ACLs grant application commands individually to the local shell only.
- HTTP tests use synthetic stock-Moodle-shaped data, not authenticated recordings from INSP or the planned 4.5/5.3 rigs.
- JavaScript dependencies were not changed in the second slice. The previous audit had no high/critical findings and one moderate development-only minimatch advisory upstream. Rust advisory auditing remains to configure.

- Native UI checks on macOS 27.2 confirmed French onboarding on first launch, French Settings, English UI/native menu switching, and the light Settings layout. The final transparent/centered titlebar adjustment still needs visual confirmation after relaunch because UI access was subsequently denied. Test settings remained English/light at that point; defaults in code remain automatic.

## Remaining M0 checks and next work

1. Complete interactive INSP login; validate callback interception and cookie/session restoration end to end. No credentials have been requested or used by the agent.
2. Provision Moodle 4.5/5.3 + Keycloak fixtures and test SAML/OIDC. Verify version-specific public roots, rate-limit behaviour and missing-private-token fallback.
3. D1/D2: three simultaneous sites, warm-switch timing, resize, focus, cookies/profile separation on macOS 12, 15 and current macOS.
4. Inject a remote-webview IPC probe; configuration checks do not prove runtime isolation. Verify Keychain prompts, deletion and app identity across installed updates.
5. Finish typed notification wrappers and notification/tray work (W4), then deep links/download capture (W5) and universal packaging/distribution (W6). Downloads remain blocked; closing the window exits until a real tray/background lifecycle exists.
6. Record D1–D5 results before claiming M0. Full legacy browser-data deletion remains an explicit privacy/platform gate.

See [UI decisions](ui-design.md) and [architecture decisions](adr/0001-alpha-foundation.md).
