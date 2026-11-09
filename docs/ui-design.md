# Shell UI design (alpha)

The local shell (`apps/shell`) should feel like a native macOS app that hosts Moodle, not a website.

## Principles

- **Native first.** The real macOS titlebar and traffic lights stay; the shell never draws imitation window controls. Menu commands (Settings… ⌘,, Add Organisation… ⌘N, Back/Forward/Reload) live in the native menu bar and reach the shell through `shell://navigate` / `shell://error`.
- **System typography and colour.** `system-ui` (SF Pro) at 13 px, AppKit-like semantic tokens (`--label`, `--label-2`, `--separator`, `--fill`…) and the system blue accent (`#007aff` / `#0a84ff`).
- **Materials sparingly.** On desktop the window is transparent over the native sidebar material, so the 72 px sidebar shows it directly. The 44 px toolbar is a translucent blurred bar over scrolling content. Everything else is opaque.
- **Honest controls.** Every control works. Navigation buttons appear only when a site is shown. The browser preview says what it cannot do instead of pretending.

## Geometry

The 72 px sidebar and 44 px toolbar are fixed in `src-tauri/src/windows.rs` (`SIDEBAR`, `TOOLBAR`). Remote site views are positioned by Rust and cover the content area, so shell messages for a visible site show in the toolbar, not in the content area.

## Views

| View             | Shown when                  | Remote view                                        |
| ---------------- | --------------------------- | -------------------------------------------------- |
| Site             | A site is selected          | Visible (positioned by Rust)                       |
| Add Organisation | No sites, sidebar **+**, ⌘N | Hidden (`show_onboarding`)                         |
| Settings         | Sidebar gear, ⌘,            | Hidden (`shell_view('settings')` before rendering) |

Escape, **Done** or **Cancel** returns to the last site, or the first site if that one was removed.

## Connection progress

One state machine (`src/lib/connection.ts`) covers adding a site and signing in again, because Rust allows one pending sign-in. Its phases match real work: _checking_ (public site discovery), _signing-in_ (organisation window open, until `auth://finished`), then _idle_ with the outcome. Errors are attributed to the add form or to the site row that caused them.

## Settings

- **General → Theme:** Automatic / Light / Dark segmented control (native radios, arrow-key selection). CSS reads `data-theme` on `<html>`; Automatic follows `prefers-color-scheme`.
- **General → Language:** Automatic / Français / English. Language names are shown in their own language (`lang` set on each), as macOS does.
- Both settings apply optimistically and persist through the one `settings_set` command, which receives only the changed field (`{ appearance }` or `{ language }`) and returns both saved values. A failed save rolls back only that field and explains why. Each picker is disabled only while its own value saves. In the browser preview, settings live in memory and the banner says they last until reload.
- **Organisations:** each row offers Open, Sign In Again (`site_reauthenticate`) and Remove…. Removal always asks in an inline alert dialog before calling `site_remove`. Focus moves to Cancel, and Escape closes only the confirmation.

## Language

The shell is fully translated into English and French (`src/i18n/`): every visible string, accessible label, tooltip, progress step, confirmation and status message.

- **Locale.** `resolveLocale` uses the saved language when it is French or English. Automatic picks the first supported language in `navigator.languages` (so `de-DE, fr-FR` gives French), else English, and follows `languagechange`. `<html lang>` tracks the locale. Rust renders the native menus in the same saved/system language.
- **Catalogues.** `messages.ts` holds `en` and a `fr` catalogue that TypeScript forces into the same shape; interpolated messages are functions. French follows macOS terms (Réglages, Terminé, Précédent/Suivant, Retirer, trousseau) and uses non-breaking spaces before `?` and `:`. Counts use `Intl.PluralRules` (French says « 0 site »), and dates use `formatDate`/`useI18n().date` with `Intl.DateTimeFormat` (en-GB / fr-FR).
- **Backend errors.** Rust returns English strings for now. The shell keeps them raw in state and translates them when it renders, so switching language also translates a message already on screen. `errors.ts` maps every current Rust error (commands, windows, sign-in cancellation, `md-moodle::Error`) to French by exact text, ignoring apostrophe style and surrounding whitespace. An unknown error in French shows a French sentence saying what failed (adding, signing in again, saving a setting, removing, navigating) followed by « Détail : » and the original wording marked `lang="en"`. In the toolbar, where there is one line, that detail goes in the tooltip. When Rust adds or rewords an error, update `errors.ts`; the tests check that every listed error has a translation.
- **No organisation names.** UI copy and examples use neutral placeholders (`moodle.example.edu` / `moodle.exemple.fr`). Tests reject a hardcoded organisation name in the catalogues.

## Accessibility

- Visible focus rings on every control; Up/Down/Home/End move between sidebar items; `aria-current` marks the current site/view.
- Progress and outcomes use `role="status"`/`role="alert"`; the field links its help or error with `aria-describedby`.
- `prefers-contrast: more` strengthens text, separators and control edges; `prefers-reduced-motion` removes transitions and slows spinners; `prefers-reduced-transparency` makes the browser-preview materials opaque (desktop materials follow macOS).

## Integrated titlebar

The main window uses Tauri's transparent native titlebar plus AppKit's explicit `Expanded` toolbar style, keeping the title centered. The titlebar separator is removed to blend it into the material-backed shell. AppKit continues to own the title and its drag surface, native double-click behaviour, traffic lights and window-management controls. No borderless window or JavaScript drag surface is introduced. The centered transparent style is compiled but still needs visual confirmation after relaunch; native UI access was denied at the end of the preceding UI checks.
