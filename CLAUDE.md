# MoodBuro — notes for coding agents

A macOS-first Tauri 2 app hosting Moodle sites. Rust in `src-tauri/` and `crates/`, local shell UI (React + plain CSS) in `apps/shell/`.

## UI work

- **Read [docs/design-guidelines.md](docs/design-guidelines.md) before changing any UI.** It defines the tokens, components, window geometry, EN/FR copy rules, accessibility requirements and the PR checklist. [docs/ui-design.md](docs/ui-design.md) records past decisions.
- Use the tokens in `apps/shell/src/styles.css`; never hard-code colours, radii or font sizes in component rules.
- Toolbar/sidebar geometry is defined in both `styles.css` and `src-tauri/src/windows.rs`; change both.
- Every string goes in both catalogues in `apps/shell/src/i18n/messages.ts`; never name a real institution.
- Browser preview: `pnpm dev`, open `http://127.0.0.1:1420/?mac&demo`. Native: `pnpm tauri dev`.

## Checks

```
pnpm lint && pnpm format:check && pnpm test && pnpm build
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
python3 scripts/check-security.py
```
