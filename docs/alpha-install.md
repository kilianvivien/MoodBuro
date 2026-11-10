# Install MoodBuro Alpha

## Download and install

1. Download `MoodBuro_0.1.0-alpha.3_universal.dmg` and `SHA256SUMS.txt` from the private [GitHub prerelease](https://github.com/kilianvivien/MoodBuro/releases/tag/v0.1.0-alpha.3). You need repository access. The universal installer contains both Apple Silicon and Intel binaries. The configured minimum is macOS 12; compatibility across supported OS versions still needs qualification.
2. In Terminal, change to the directory containing both downloads and run `shasum -a 256 -c SHA256SUMS.txt`. The DMG must report `OK` before you open it.
3. Open the DMG and drag MoodBuro into Applications. Eject the DMG and launch MoodBuro from Applications.
4. This alpha is ad-hoc signed, **not Developer ID signed or notarized**. If macOS blocks it, follow [Apple's instructions](https://support.apple.com/en-us/102445): after attempting to open the app, go to System Settings → Privacy & Security → Open Anyway, then confirm. Older macOS versions use System Preferences → Security & Privacy. Managed Macs may require administrator approval. Do not disable Gatekeeper globally.
5. Enter your organisation's Moodle URL and sign in in its own login window. Test with an account appropriate for early Alpha software.

## Upgrades and removal

Quit MoodBuro before replacing it in Applications. Closing the window only hides it and keeps notification polling active; use MoodBuro → Quit. There is no automatic updater in this release: download subsequent DMGs manually. Ad-hoc signing does not provide a stable publisher identity, so Keychain may ask for permission again after an upgrade.

In builds after alpha.3, macOS credentials share one Keychain item and one in-memory read per process. The first launch migrates the old items and can still ask for each old item; subsequent launches use the combined item. Migration writes the combined item before deleting old entries and retries interrupted cleanup. Ad-hoc signing may still require access approval after updates.

Removing an organisation in Settings clears its app-managed credentials and metadata; downloaded/offline files remain. Legacy shared WebKit/SSO data on older macOS may remain, as documented in the authentication review. Deleting the app alone does not remove all application data or Keychain items.

## Assistant

The AI Assistant is optional and uses your own Albert API or Mistral AI key. Configure it in Settings. Moodle sharing defaults off and requires consent plus an explicit context attachment. Keys stay in the OS credential store; conversations stay in memory and are deleted on quit. Provider usage may incur charges or quota limits. See [assistant scope and qualification](assistant-qa.md).

## Alpha limits and bug reports

This is an early tester prerelease, not a fully qualified production release. Automated tests do not establish live SSO compatibility, Notification Center delivery, clean-install behavior, Intel runtime behavior, native accessibility or live provider support. Read the feature QA documents linked from [implementation status](implementation-status.md).

Report the app version, macOS version, processor, Moodle version if known, exact reproduction steps and expected/actual result. Remove Moodle tokens, API keys, personal information and sensitive screenshots from reports. Do not post credentials or sign-in callback URLs.

## Rebuild

Install the toolchains listed in the README and both Rust targets:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm install --frozen-lockfile
pnpm tauri build --ci --target universal-apple-darwin --bundles app,dmg --config src-tauri/tauri.alpha.conf.json
```

Native icon packaging requires Xcode 26 or later.

The output is under `target/universal-apple-darwin/release/bundle/`. Check both architectures with `lipo -archs` and verify the app using `codesign --verify --deep --strict`. Verify the DMG with `hdiutil verify` and generate checksums with `shasum -a 256`. The manual GitHub Actions packaging workflow builds the selected ref and saves verified installers as run artifacts; publishing is a separate explicit step.

The pinned Rust 1.91 release profile leaves build-time compiler plugins unstripped to avoid the [LLVM Mach-O string-pool alignment bug](https://github.com/llvm/llvm-project/issues/203678) exposed by macOS 27. This does not change the optimization of the shipped application.
