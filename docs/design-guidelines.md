# MoodBuro design guidelines

How to build UI for the MoodBuro shell (`apps/shell`) so it feels like a Mac app, not a website in a window. Read this **before** adding a view, control or message. [ui-design.md](ui-design.md) records past decisions and the reasons for them; this document is the rulebook for new work.

When this guide and Apple's [Human Interface Guidelines for macOS](https://developer.apple.com/design/human-interface-guidelines/designing-for-macos) disagree, follow the HIG and update this guide.

---

## 1. Principles

1. **Native before custom.** If AppKit has the thing (menu bar commands, traffic lights, window title, context menus in text fields, Keychain prompts, notifications), use the real one. The shell never draws imitation window controls, fake menus or fake dialogs styled as macOS sheets.
2. **Moodle is the content; the shell is chrome.** The shell stays quiet: neutral surfaces, one accent colour, no brand illustrations around Moodle. Anything that competes with the Moodle page for attention needs a reason.
3. **Honest controls.** Every visible control works. A control that cannot work right now is hidden (navigation outside a site) or disabled with a reason nearby. The browser preview says what it cannot do.
4. **Calm feedback.** Progress is shown where the work happens (the row, the form), in words that match the actual step. Errors say what failed and what to do next. No toasts that vanish before they can be read.
5. **Keyboard, VoiceOver and both languages are features, not polish.** A change is not finished until it works with the keyboard alone, reads correctly in VoiceOver, and exists in French and English.

---

## 2. Window anatomy

```
┌──────────────────────────────────────────────────────────────┐
│ ● ● ●   ⌂ ← →  ↻   Site name                     ⌕  ⤓  ⤴    │  unified toolbar, 52 pt (macOS)
│                moodle.example.edu                            │  drag region, double-click zooms
├──────┬───────────────────────────────────────────────────────┤
│  ✉ 5 │                                                       │  views: Inbox, Courses, Agenda
│  ▤   │                                                       │
│  ▦   │            Moodle web view (positioned by Rust)       │
│  ──  │       or a local view: Inbox / Courses / Agenda /     │
│  EU  │       Downloads / Add Organisation / Settings         │  organisations
│  CX  │                                                       │
│  +   │                                                       │
│  ⚙   │                                                       │
└──────┴───────────────────────────────────────────────────────┘
 72 pt sidebar
```

| Region          | Size                                | Owner                     | Notes                                                                                                                                    |
| --------------- | ----------------------------------- | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Unified toolbar | 52 pt high (macOS), 44 pt elsewhere | `components/Toolbar.tsx`  | Spans the full width on macOS. Content starts after `--traffic-inset` (78 pt), which drops to 0 in full screen.                          |
| Traffic lights  | x 20, y 26 (`TRAFFIC_LIGHTS`)       | AppKit via `windows.rs`   | Real controls, vertically centred in the toolbar. Never covered by shell UI.                                                             |
| Sidebar         | 72 pt wide                          | `components/Sidebar.tsx`  | Starts below the toolbar on macOS. App-wide views (Inbox, Courses, Agenda) sit above the organisations and appear once one is connected. |
| Content         | the rest                            | `App.tsx` `<main>` / Rust | Remote Moodle views cover it completely.                                                                                                 |

### Geometry is defined twice — change both together

| Value          | CSS (`styles.css`)                               | Rust (`src-tauri/src/windows.rs`)           |
| -------------- | ------------------------------------------------ | ------------------------------------------- |
| Sidebar width  | `--sidebar-width: 72px`                          | `SIDEBAR`                                   |
| Toolbar height | `--toolbar-height` (44px; 52px under `.desktop`) | `TOOLBAR` (`cfg(target_os = "macos")` → 52) |
| Traffic lights | `--traffic-inset`                                | `TRAFFIC_LIGHTS`                            |
| Window backing | `--window`                                       | `LIGHT_BACKING` / `DARK_BACKING`            |

Because Moodle views sit **above** the shell webview, anything the user must see while a site is open (errors, status) goes in the toolbar, never in the content area.

### Window states mirrored on `<html>`

| Hook                             | Set by                                                | Effect                                                                                                 |
| -------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `.desktop`                       | `lib/window.ts` (macOS app, or `?mac` in dev preview) | Unified toolbar geometry.                                                                              |
| `.fullscreen`                    | Rust `window://fullscreen` event                      | Removes the traffic-light inset.                                                                       |
| `[data-inactive]`                | Rust `window://focused` event                         | Selection turns grey; toolbar title and icons dim — as in every AppKit app when the window is not key. |
| `[data-theme="light" \| "dark"]` | Settings → Theme                                      | Pins the appearance; absent = follow the system.                                                       |

New components must look right in all combinations: light/dark × active/inactive × Increase Contrast.

---

## 3. Colour

All colour comes from the semantic tokens in `styles.css`. **Never write a literal colour in a component rule**; if a token is missing, add it to every theme block (light, both dark blocks, both Increase Contrast blocks) in the same change.

| Token                                              | Use                                                                                                            |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `--window`                                         | Window and content background; unified toolbar background (`--toolbar`).                                       |
| `--sidebar`                                        | Sidebar surface (slightly darker than `--window`).                                                             |
| `--elevated`                                       | Grouped lists, cards, progress panels.                                                                         |
| `--control`                                        | Push button and text-field bezel fill.                                                                         |
| `--label` / `--label-2` / `--label-3`              | Primary text · secondary text (subtitles, hints) · tertiary text (placeholders, disabled, waiting steps).      |
| `--separator`                                      | Hairlines between rows/regions.                                                                                |
| `--control-border`                                 | Bezel edges of buttons, fields, segmented controls.                                                            |
| `--fill` / `--fill-2`                              | Hover · pressed/tile backgrounds for borderless controls.                                                      |
| `--accent`                                         | The user's macOS accent colour (falls back to system blue). Primary buttons, active progress, links-as-status. |
| `--accent-tint`, `--selection`, `--selection-text` | Informational banners · source-list selection (auto-greys when inactive).                                      |
| `--panel-shadow`                                   | The Quick Open panel only: the one surface that floats above the window.                                       |
| `--focus`                                          | Keyboard focus ring.                                                                                           |
| `--red`, `--red-text`, `--red-tint`                | Destructive buttons · error text · error capsule.                                                              |
| `--green-text`                                     | Success confirmation text.                                                                                     |

Rules:

- **Accent is meaning, not decoration.** It marks the primary action, the current selection, progress and unread state (the inbox dot and count badge, as in Mail). Never use it for headings, icons at rest or large backgrounds.
- **Keyboard selection in a menu-like list is filled.** The Quick Open result under the keyboard is a solid `--accent` row with `--on-accent` text, as in Spotlight and menus. Everything else stays tinted.
- **Selection is tinted, not filled.** Sidebar selection uses `--selection` (accent at ~16–26 %), not a solid accent block.
- **Red only for errors and destructive actions.** Green only for a completed action's confirmation.
- The app icon gradient (`.app-mark`) is the one place brand blue is fixed; it is an icon, not UI.

---

## 4. Typography

The system font (SF Pro) via `system-ui`. Sizes are tokens:

| Token             | Size  | Use                                                           | Weight                                                                               |
| ----------------- | ----- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `--text-title`    | 22 px | One large title per local view (onboarding)                   | 700                                                                                  |
| `--text-headline` | 15 px | The heading of a list view (`.page-header`), Quick Open field | 600 (400 in the field)                                                               |
| `--text-body`     | 13 px | Everything by default: rows, buttons, fields, toolbar title   | 400; 500 for row titles/field labels; 600 for section titles, toolbar title, avatars |
| `--text-small`    | 12 px | Segmented controls, banners, section messages, small buttons  | 400                                                                                  |
| `--text-caption`  | 11 px | Subtitles, hints, footnotes, toolbar subtitle                 | 400                                                                                  |

Rules:

- No other sizes. If something needs to stand out, use weight or `--label` vs `--label-2`, not a bigger font.
- Sentence case for everything except product names ("Add organisation…" in prose, but follow macOS title case in **menu items and buttons** in English: "Sign In Again", "Open in Default Browser"; French always uses sentence case).
- An ellipsis (`…`, the single character) ends any command that asks for more input before acting ("Remove…", "Settings…").
- Truncate single-line labels with `text-overflow: ellipsis` and expose the full text in `title` / `aria-label`.
- UI text is not selectable (`user-select: none` on `:root`); inputs re-enable it. Don't make labels selectable unless they are values the user would copy.

---

## 5. Spacing, radius and motion

- **Spacing** uses a 2/4 pt rhythm: 2, 4, 6, 8, 10, 12, 16, 20, 24, 28, 32, 48. Rows: 8 × 12 padding, 12 gap, min-height 44 (40 for footers). Between sections: 28.
- **Radius** tokens, smallest to largest: `--radius-xs` 4 (tiny icon buttons) · `--radius-s` 5 (segments, compact fields, small buttons) · `--radius-m` 6 (push buttons, toolbar tools) · `--radius-l` 8 (fields, inner panels, row avatars) · `--radius-xl` 10 (groups, sidebar items) · `--radius-icon` 15 (64 pt app mark). Nested shapes use a smaller radius than their container.
- **Radius** also has `--radius-panel` 12 for the floating Quick Open panel.
- **Motion** is native and fluid, never decorative (revised November 2026; it used to allow colour fades only). Three durations and three curves, all tokens:

  | Token                     | Value            | Use                                                                                                  |
  | ------------------------- | ---------------- | ---------------------------------------------------------------------------------------------------- |
  | `--fast` + `--ease`       | 120 ms           | Colour, shadow and press feedback (`scale(0.97)` on buttons, `0.92` on toolbar items).               |
  | `--medium` + `--ease-out` | 220 ms           | A part moving or appearing in place: segmented thumb, confirmation, toolbar alert, Quick Open panel. |
  | `--slow` + `--ease-out`   | 320 ms           | A local view entering (8 pt rise and fade) and its first eight rows following at 20 ms steps.        |
  | `--spring`                | slight overshoot | Things that pop: unread badges, the unread dot, the pin, a completed progress mark.                  |

  Rules: animate `opacity` and `transform` only; nothing loops except spinners and loading skeletons; an entrance plays once when the element mounts, never on every re-render; nothing delays input. **Never animate the sidebar width, the toolbar height or anything else the Moodle view is positioned against** — the native view does not follow. Under Reduce Motion every transition and animation is removed (spinners slow down instead), and the UI must be complete without them.

- **Shadows**: `--shadow` for grouped surfaces, `--control-shadow` for bezelled controls. No large drop shadows; this is a flat window, not a stack of cards.

---

## 6. Components

Build from these classes before inventing new ones. Each must keep all of its states: **rest, hover, active (pressed), disabled, focus-visible**, and where relevant **current** (`aria-current="page"`) and **inactive window**.

| Component           | Class / file                                                                      | When to use                                                                                                                                                                       |
| ------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Push button         | `.button` (+ `.primary`, `.destructive`, `.large`, `.small`)                      | Any command in a local view. One `.primary` per view at most, and it is the default (Return) action. `.destructive` only inside a confirmation.                                   |
| Toolbar item        | `.tool` (+ `.small`)                                                              | Icon-only commands in the toolbar. Always has `title` and `aria-label`.                                                                                                           |
| Sidebar item        | `.sidebar-item` (+ `.site-avatar`)                                                | Top-level destinations only (organisations, Add, Settings). Up/Down/Home/End move focus.                                                                                          |
| Text field          | `.text-field` wrapping an `<input>` (+ `.compact`, `.invalid`, `[data-disabled]`) | All text entry. Pair with `.field-label` and a `.field-message` linked through `aria-describedby`.                                                                                |
| Segmented control   | `Segmented` in `components/ui.tsx` (`.segmented`)                                 | 2–4 mutually exclusive options that apply immediately. Built on native radios for keyboard support; the selected bezel slides between segments.                                   |
| Pop-up button       | `SiteSelect` in `components/ui.tsx` (`.popup-button`)                             | Choosing one of many (organisation filter). A native `<select>` in a push-button bezel, so the menu is the real macOS one.                                                        |
| Search field        | `SearchField` in `components/ui.tsx` (`.search-field`)                            | Filtering a list in place. Has a clear button; Escape clears the text before it leaves the view.                                                                                  |
| List view           | `.page` › `PageHeader` › `.filter-bar` › `.group` of `.list-row`                  | Inbox, Courses, Agenda, Downloads. Header: title, count, commands. Rows: leading mark (`.row-icon`, `.row-avatar`, `.unread-dot`, `.event-time`), text column, trailing commands. |
| Icon button         | `.button.icon` (+ `.small`, `aria-pressed`)                                       | A secondary row command with a well-known glyph (Mark as Read, Pin, Show in Finder, Copy Path, Refresh). Always `title` + `aria-label`.                                           |
| Tag                 | `.tag` (`data-kind`)                                                              | One-word classification at the end of a row (agenda event kind). Neutral; `due` is accent-tinted.                                                                                 |
| Empty state         | `EmptyState` (`.empty-state`)                                                     | A list with nothing in it. Says whether it is empty, filtered to nothing, or all caught up — these are different messages.                                                        |
| Loading             | `Skeleton`                                                                        | A cached list that has not loaded yet. Not for actions the user started (use the control's own spinner).                                                                          |
| Quick Open          | `components/QuickOpen.tsx` (`.quick-open`)                                        | ⌘L. Combobox + listbox over saved names; Up/Down/Home/End, Return, Escape. The only floating panel in the shell.                                                                  |
| Grouped list        | `.group` › `.group-row` / `.site-row` (+ `.group-footer`)                         | Settings-style forms: label + description on the left, control on the right. Inset separators start at the text column.                                                           |
| Nested row          | `.site-label-editor` pattern                                                      | A secondary setting that belongs to the row above, aligned with its text column.                                                                                                  |
| Inline confirmation | `.confirm` (`role="alertdialog"`)                                                 | Destructive actions. Focus goes to Cancel; Escape cancels only the confirmation.                                                                                                  |
| Section message     | `.section-message.error` / `.success`                                             | Outcome of an action in a local view, under the group it concerns.                                                                                                                |
| Toolbar alert       | `.toolbar-alert`                                                                  | Errors while a Moodle view is visible. One line; detail in the tooltip; dismissible.                                                                                              |
| Progress            | `.progress` + `.progress-step`                                                    | Multi-step work the user waits for. Steps match real phases (`lib/connection.ts`).                                                                                                |
| Banner              | `.preview-banner` (`role="note"`)                                                 | A persistent, page-level fact (browser preview limits). Not for errors.                                                                                                           |

Icons: [lucide-react](https://lucide.dev), stroke width **1.75** (2 at ≤ 14 px), sizes **18** (sidebar), **16** (toolbar, fields), **14** (inline with text), **12** (captions). Always `aria-hidden="true"` with a text label or `aria-label` on the control. Prefer the glyph that matches the SF Symbol macOS uses for the same command (arrow.left → `ArrowLeft`, gear → `Settings`, arrow.clockwise → `RotateCw`).

---

## 7. Patterns

- **Commands live in the menu bar first.** Every shell command with a toolbar or sidebar affordance also has a native menu item with the standard shortcut (`src-tauri/src/menus.rs`): Settings… ⌘,, Add Organisation… ⌘N, Inbox ⌘1, Courses ⌘2, Agenda ⌘3, Downloads ⌥⌘L (as in Safari), Quick Open… ⌘L, Save to Downloads ⌘S, Back ⌘[, Forward ⌘], Reload ⌘R. New commands follow the [standard macOS shortcuts](https://support.apple.com/en-us/102650) and never reuse a system one. A shortcut must be a menu accelerator: while a Moodle view has focus, key events never reach the shell.
- **Local views** (Inbox, Courses, Agenda, Downloads, Add Organisation, Settings) hide the remote view through Rust before rendering, have a toolbar title, and leave with **Done** / **Cancel** or Escape. A new one is added in `menus::LOCAL_VIEWS`, the `View`/`ShellView` types, the toolbar title and the menu.
- **Cached lists say how old they are.** Courses and Agenda show the saved list at once, refresh organisations older than ten minutes in the background, and show "Updated … ago". One organisation failing keeps the others and the saved rows, with the reason above the list.
- **Server-changing actions wait for the server.** Mark as Read changes the row only after Moodle accepted it; a failure leaves the row as it was and says why. Local-only state (pins) applies at once and rolls back on failure.
- **Opening Moodle from a list** goes through `site_open_page`, which accepts a fixed page name and builds the URL in Rust from the saved site. Never pass a URL from the shell, and never follow one supplied by the server.
- **Quick Open** is the one exception to "no overlays": a floating panel over the content area, without dimming the window. Because the Moodle view sits above the shell, Rust hides it while the panel is open and shows it again if the panel is dismissed.
- **Settings apply immediately** (no Save button) unless the value is free text, which saves on Return or an explicit small button. Failed saves roll back only that field and explain why.
- **Destructive actions** always ask inline first, say exactly what will be lost, and name the organisation.
- **Errors**: keep the raw backend text in state and translate at render time (`useI18n().error` / `ErrorText`). Show what failed, then the detail.
- **Context menus**: release builds suppress the web context menu outside text fields (`lib/window.ts`). If a feature needs one, design a real menu; don't re-enable the web one.
- **No web idioms**: no hand cursors on buttons (`cursor: default`), no underlined links for in-app actions, no hover-only affordances, no modal overlays dimming the window, no scroll bounce in shell views.

---

## 8. Writing and localisation

- Every visible string, `title`, `aria-label` and status lives in `src/i18n/messages.ts` in **both** `en` and `fr`; TypeScript enforces the shape. No string literals in components.
- English follows macOS terms (Settings, Done, Remove…). French follows macOS French (Réglages, Terminé, Retirer…, trousseau), uses ` ` before `? ! : ;` and « guillemets ».
- Plurals via `Intl.PluralRules`; dates via `useI18n().date`.
- New or reworded Rust errors must be added to `src/i18n/errors.ts`.
- **Never name a real institution** in UI copy, examples or fixtures. Use `moodle.example.edu` / `moodle.exemple.fr`.

---

## 9. Accessibility

- Visible focus ring (`--focus`) on every control; never `outline: none` without a replacement.
- Landmarks: one `<nav>` (sidebar), one `<header>` (toolbar), one `<main>`. Each local view has an `h1`/`h2`.
- `aria-current="page"` on the current destination; `role="status"` for progress/success, `role="alert"` for errors.
- Must hold up under **Increase Contrast** (`prefers-contrast: more` blocks), **Reduce Motion** and **Dark Mode**. Check text contrast ≥ 4.5:1 for `--label-2` on every surface it's used on.
- Hit targets ≥ 20 × 20 pt, with 28 pt for toolbar items.

---

## 10. Working on the UI

- **Browser preview:** `pnpm dev`, then open `http://127.0.0.1:1420/?mac&demo`. `?mac` applies the macOS window chrome; `?demo` loads two neutral sample organisations with sample notifications, courses, events and downloads (in memory, so Mark as Read, pins and Clear History can be tried). Both flags are dev-only and stripped from release builds. ⌘K opens Quick Open in the preview, because the browser owns ⌘L. Every new IPC call needs a demo fixture in `ipc.ts`.
- **Native check:** `pnpm tauri dev`. Anything touching the toolbar, traffic lights, window states or geometry must be checked in the real window: drag, double-click to zoom, full screen, window inactive, light/dark.
- `pnpm lint && pnpm format:check && pnpm test && pnpm build`, and `cargo clippy --workspace --all-targets -- -D warnings`.

### PR checklist for UI changes

- [ ] Uses existing tokens and components; any new token is defined in every theme block.
- [ ] Geometry changes made in both `styles.css` and `windows.rs`.
- [ ] Checked in light and dark, active and inactive window, and Increase Contrast.
- [ ] Any animation uses the motion tokens, touches only `opacity`/`transform`, and the view is complete with Reduce Motion on.
- [ ] Keyboard-only path works; focus is visible; VoiceOver labels make sense.
- [ ] Every new string exists in English and French; no institution names.
- [ ] Commands also reachable from the menu bar with a standard shortcut.
- [ ] Native window checked with `pnpm tauri dev` when chrome or geometry changed.

## Moodle popup windows

Same-origin Moodle popups preserve their opener and WebKit configuration/session in separate native windows (at most three per site). Their local toolbar is 44 pt high (`windows::resize_popup` and `.popup-bar`), below a standard native titlebar. Remote content has no IPC grants; `chrome-popup-*` local toolbars have a separate capability granting only popup context/navigation and event listening. Caller checks bind each toolbar to its own window.

Back uses WKWebView history. At the start of a popup's history the label becomes “Back to Moodle” / « Retour à Moodle » and the action closes it, restoring its originating site view. Home explicitly returns the parent window to the site dashboard. Main-window PDF navigation also uses native history, with dashboard fallback when no earlier item exists. Every site toolbar includes a Home control; the native Go menu exposes the same command with ⌘⇧H.

Menu-bar rows use the saved abbreviation or bounded organisation text, count and a per-site submenu. Recent Downloads is a submenu with five bounded filenames plus the full list entry. Clicking a filename reveals it in Finder, never opens it; risky-file confirmation remains in the local Downloads view.

## Native document readers

Downloads exposes Preview separately from Open for PDF, DOCX, XLSX and PPTX. Preview creates a Rust-owned Tauri document window with real AppKit chrome and an NSToolbar; no imitation controls, document HTML or document bytes are sent to the shell. PDFKit supplies the PDF surface; Quick Look supplies Office content inside a native magnifiable scroll view. Common toolbar actions are zoom out/in, fit, a zoom readout, Open in Default App, Show in Finder and full screen. PDFs additionally show previous/next page and current/total page count. Office navigation uses native document scrolling; unsupported page buttons are omitted. Labels/tooltips follow the app's French/English preference when the document opens, and appearance follows app settings.

Each preview is bounded to a completed managed download ID, paths are revalidated before external Open/Finder, and three previews may be open simultaneously. AppKit owns resize/fullscreen/focus and toolbar overflow. Native preview implementations are in `src-tauri/src/documents.rs` and `native_preview.rs`; these windows do not alter the main shell's CSS/webview geometry.
