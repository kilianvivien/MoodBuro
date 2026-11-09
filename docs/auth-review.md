# Authentication check during the titlebar repair

## Evidence inspected

- User-provided screenshot shows an authenticated INSP dashboard in the app. The screenshot is not copied into the repository.
- Available dev-server output initially contained rebuilds and two native window-move warnings. No token, private token or callback URL was observed in that output.
- New fixed development-only milestones subsequently showed saved-session restoration, a one-use autologin navigation, and retained-browser-session fallbacks after rebuilds. Starting autologin navigation is not itself proof that every step completed.
- Read-only inspection of the live metadata database found only `sites` and `settings`, with no credential-named columns. No credential values or Keychain passwords were read.

## Code checks and changes

- Login uses the organisation’s own page. Auth views have no privileged IPC grant.
- Callbacks use a random one-use challenge, signature validation, bounded input before Base64 decoding, and server-side token validation before persistence.
- Saved tokens go through the OS credential wrapper. Existing credentials are restored on a failed save where the OS store allows it; rollback failures remain a platform/integration test concern.
- Cold restoration now verifies that the validated token’s user matches the saved account before constructing autologin navigation.
- Autologin destinations are restricted to the expected same-origin Moodle endpoint. Keys/private tokens stay out of shell IPC, SQLite and diagnostic logs.
- Diagnostic events use an enum with fixed text, accept no arbitrary payload, and only emit in development builds.
- The auth view permits HTTPS IdP redirects and blocks non-HTTPS navigation, downloads and new windows. Remote IPC isolation still needs an injected runtime probe.

## Still to verify

Installed-app Keychain access/identity across updates, profile deletion completion, legacy shared-store cleanup, runtime IPC denial, and the Moodle 4.5/5.3 SAML/OIDC matrix remain Alpha gates. These observations support the current implementation; they do not establish a complete security sign-off.
