# Alpha foundation status

## Delivered scope

The first implementation slice covers the workspace, CI, local onboarding, child webview wiring, navigation policy, callback verification, credential wrapper and initial metadata store. It does **not** close M0 or claim the complete W1 checklist. `md-notifier` and `md-os` are scaffold crates only.

Product owner confirmed generic organisation URL onboarding. First real integration target: `https://moodle.insp.gouv.fr/`. Public discovery returned `wwwroot=https://moodle.insp.gouv.fr`, `typeoflogin=3`, and the stock mobile `launch.php` URL. No account credentials or tokens were used for this check.

## Remaining spike checks

- D1: run three sites simultaneously; verify resize, focus and keyboard controls on macOS 12 and 15. The foundation compiles the child API but has not passed that matrix.
- D2: validate cookies across auth and Moodle views; test separate persistent stores on macOS 14+. Default shared WebKit store is the current fallback.
- D3: complete a real INSP login interactively; test callback interception on Moodle 4.5 and 5.3, SAML and OIDC. Public configuration discovery alone does not validate SSO.
- Run an injected remote-webview IPC probe. The automated capability check is a configuration regression guard, not runtime proof.
- Verify Keychain prompts in an installed app and deletion/identity across updates. Credential store get/delete primitives exist; site sign-out UX is not implemented.
- D4/D5: notification click routing and WKWebView download interception are future spikes, not implemented features.

## Next implementation slice (W1 → W2)

1. Provision Moodle 4.5/5.3 + Keycloak test rigs and capture sanitised contract fixtures.
2. Run INSP sign-in, then implement autologin bridge, `/public`/canonical redirects, rate limits, retries and typed WS wrappers.
3. Add session renewal/re-auth UX and safe site removal with cookies + Keychain + metadata cleanup.
4. Record D1–D5 results before claiming M0.
5. Continue W3 multi-site polish, W4 notifier/tray, W5 deep links/downloads, W6 universal packaging and distribution.

## Local verification

- TypeScript/Vite production build, ESLint, Prettier and 2 Vitest store tests passed.
- Workspace `cargo check`, 5 Rust unit tests and `cargo clippy --workspace --all-targets -- -D warnings` passed on the development Mac.
- Shell-only capability/caller regression script passed; generated Tauri ACL manifests confirm application commands are individually gated.
- JavaScript audit: no high/critical findings after updating the initial test tooling. One moderate development-only minimatch advisory remains through ESLint/typescript-eslint (patched release requires a new major); no major override is forced into those tools. Rust advisory audit is still to be configured.
- Public INSP mobile configuration discovery succeeded. Interactive authentication, Keychain prompts, visual QA and the multi-version/macOS matrix remain unverified.
