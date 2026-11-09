# Shell UI design (alpha)

Decision log. For the rules new UI must follow, see [design-guidelines.md](design-guidelines.md).

The local shell (`apps/shell`) should feel like a native macOS app that hosts Moodle, not a website.

## Principles

- **Native first.** The real macOS traffic lights stay, centred in the unified toolbar, which is also the drag region; the shell never draws imitation window controls. Menu commands (Settings… ⌘,, Add Organisation… ⌘N, Back/Forward/Reload) live in the native menu bar and reach the shell through `shell://navigate` / `shell://error`.
- **System typography and colour.** `system-ui` (SF Pro) at 13 px, AppKit-like semantic tokens (`--label`, `--label-2`, `--separator`, `--fill`…) and the system blue accent (`#007aff` / `#0a84ff`).
- **Opaque surfaces.** The window, 72 px sidebar and toolbar are opaque semantic surfaces; controls use subtle fills and semantic colours. The accent follows the macOS accent colour when WebKit exposes it.
- **Honest controls.** Every control works. Navigation buttons appear only when a site is shown. The browser preview says what it cannot do instead of pretending.

## Geometry

The 72 px sidebar and the toolbar (52 pt unified on macOS, 44 px elsewhere) are fixed in `src-tauri/src/windows.rs` (`SIDEBAR`, `TOOLBAR`). Remote site views are positioned by Rust and cover the content area, so shell messages for a visible site show in the toolbar, not in the content area.

## Views

| View             | Shown when                  | Remote view                                        |
| ---------------- | --------------------------- | -------------------------------------------------- |
| Site             | A site is selected          | Visible (positioned by Rust)                       |
| Add Organisation | No sites, sidebar **+**, ⌘N | Hidden (`show_onboarding`)                         |
| Settings         | Sidebar gear, ⌘,            | Hidden (`shell_view('settings')` before rendering) |
| Inbox            | Sidebar, ⌘1, tray           | Hidden (`shell_view('inbox')`)                     |
| Courses          | Sidebar, ⌘2                 | Hidden (`shell_view('courses')`)                   |
| Agenda           | Sidebar, ⌘3                 | Hidden (`shell_view('agenda')`)                    |
| Downloads        | Toolbar, ⌥⌘L, tray          | Hidden (`shell_view('downloads')`)                 |
| Quick Open       | Toolbar, ⌘L (panel)         | Hidden while open; restored if dismissed           |

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
- `prefers-contrast: more` strengthens text, separators and control edges; `prefers-reduced-motion` removes transitions and slows spinners; there are no translucent materials, so Reduce Transparency needs no override.

## Unified toolbar (November 2026)

The separate 32-point title strip stacked on a 44-point toolbar was replaced by one 52-point unified toolbar, as in Safari, Mail and Finder. `TitleBarStyle::Overlay` keeps the native traffic lights, now placed at (20, 26) through `WindowConfig.traffic_light_position` so they are vertically centred; the toolbar spans the full width with a 78-point leading inset and is a `data-tauri-drag-region="deep"` (buttons still click; double-click zooms). The window title is hidden; the toolbar shows the site name over its host, like an NSWindow title/subtitle. Moodle views start at 52 points.

Rust emits `window://focused` and `window://fullscreen` so the shell greys selection and dims toolbar text when the window is not key, and drops the traffic-light inset in full screen. The window and shell webview get an opaque backing matching `--window` to avoid a white flash at launch. Sidebar selection is a tinted rounded rectangle (no edge bar), which turns grey when the window is inactive. Release builds suppress the web context menu outside text fields.

Whole-window transparency and Liquid Glass stay out: they previously left the titlebar without a reliable backing. A user screenshot of the native window (authenticated dashboard) confirms the centred traffic lights, toolbar layout and content alignment. Still to check by hand: dragging and double-click zoom on the toolbar, the full-screen inset, inactive-window dimming, release-build context-menu suppression, and that a non-blue system accent colour is picked up.

## Organisation short labels

Réglages → Organisations provides an editable abbreviation (up to four letters/numbers). It replaces automatic initials in the sidebar and account avatar, while the full site name remains in labels/tooltips. Clearing it restores automatic initials. Labels persist independently of credentials, survive reconnect, and are deleted with the local account. No institution is hard-coded.

## Inbox, Courses, Agenda and Quick Open (November 2026)

Product-owner choices for this slice: build A1, A2 and the read-only part of A4 together; motion "native and fluid"; new destinations as icons in the existing 72-point sidebar rather than toolbar buttons, so no geometry changed in Rust.

- **Sidebar.** Inbox (with the total unread count), Courses and Agenda sit above the organisation tiles, separated by a short rule, and appear only once an organisation is connected. Downloads stays a toolbar command. Organisation badges moved to the top-right corner so they no longer cover the initials.
- **One list pattern.** The four list views share `.page`: a header (title, count, commands), an optional filter bar, grouped rows and a footnote stating the limits. Inbox and Agenda group by calendar day in the Mac's time zone ("Today", "Tomorrow", then full dates).
- **Inbox counts.** The badge counts notifications plus unread conversations, but the list holds notifications only. A row per organisation states its unread conversations and opens Moodle's Messages page, so marking everything read does not leave an unexplained badge.
- **Opening things.** Notifications open the organisation's notifications page, courses open `course/view.php`, events open the calendar day. All are fixed pages built in Rust; a course identifier is accepted only if it is in that account's cached list. Server-supplied context URLs are still never followed or stored.
- **Quick Open.** A Spotlight-like panel rather than a view: it searches saved course names, organisations and shell views without a network call (accent-insensitive, every word must match, name starts rank first). With nothing typed it lists pinned courses, organisations and views. Cached activities are not indexed yet, and the global ⌘⇧M shortcut is not built; both stay open in the tracker.
- **Downloads.** History is paged (25 rows, Show More) with a filename search, organisation pop-up and state filter. Copy Path writes to the pasteboard from Rust, so paths still never reach the shell. Clear History asks first, keeps files and keeps transfers in progress listed.
- **Motion.** The earlier "colour fades only" rule was replaced at the product owner's request; the rules are in the guidelines. The window geometry and the Moodle view never animate.
- **Not verified natively.** Everything above was checked in the browser preview with sample data and by automated tests. The native menu shortcuts, the tray Inbox entry, hiding/restoring the Moodle view around Quick Open, and all live Moodle calls remain to be checked in the app with a real account.
