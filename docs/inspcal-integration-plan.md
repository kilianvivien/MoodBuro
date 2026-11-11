# Bundling the InspCal timetable engine into MoodBuro — integration plan

Date: 11 November 2026. Status: **plan only — no implementation**. Investigated against MoodBuro HEAD (`28a04b1`) and the standalone InspCal project at `~/Documents/Parseur PDF vers calendrier` (InspCal 2.2, engine `inspcal-engine` 0.2.0). Reviewed 11 November 2026 by two independent GLM 5.3 review threads: all file:line claims re-verified against both repos; corrections applied (schema version, UID form, byte limits, WASM boot sequence, response-body fields, CI guard mechanics, timezone/wipe-list details, opener variant, INSP-site gating, PDFium thread-safety/`thread_safe` feature, week-key identity, site-scoped PKs, lipo'd dylib provenance) and the Option A recommendation confirmed — no fatal flaw found in the current ad-hoc/non-hardened packaging; post-beta hardened-runtime dylib loading remains the key spike item (P0).

Idea under investigation: when the user finds the INSP timetable PDF inside Moodle, hand it straight to the InspCal conversion engine (Rust compiled to WebAssembly, PDFium-based) and either **host the resulting calendar inside MoodBuro's Agenda** or **export a standard `.ics`** to the user's own calendar app (Apple Calendar, Google, Outlook).

This document describes what exists on both sides, the integration options, a recommended detailed design, phasing, testing, and risks. It follows the style of [plan.md](plan.md), [alpha-feature-expansion.md](alpha-feature-expansion.md) and the [ADRs](adr/); QA evidence, when the feature lands, belongs in a new `calendar-pdf-qa.md`.

---

## 1. User story

1. In MoodBuro's **Course files** view (or **Downloads**, or the **Agenda**), the user sees a timetable PDF published on Moodle (e.g. `Edt du 17.11.2026 au 21.11.2026.pdf`).
2. One action — **"Convert to calendar"** — sends the file to the bundled InspCal engine. The PDF never leaves the device and is never written to disk.
3. A **review step** shows the parsed week: open questions the PDF leaves undecided (workshop group, LV1 room, APSA slot, half-promotion A/B), warnings (ambiguous rooms, overlaps), and options (cohort FI/CIL/OFF/all, title prefix, reminder delay, one calendar per module).
4. On confirm, the user gets the result two ways:
   - **Hosted in MoodBuro**: sessions appear in the Agenda alongside Moodle deadlines, can trigger the existing local reminders, and are included in the existing A5 ICS export.
   - **Exported**: a standard `.ics` (RFC 5545, `Europe/Paris`, stable UIDs) is saved to Downloads and revealed in Finder, or opened directly in Apple Calendar via the existing opener flow — mirroring how [calendar subscription](calendar-qa.md) hands off to Calendar.
5. When the INSP republishes a corrected week, re-running the conversion on the new PDF shows **what changed** (added/moved/cancelled sessions) and updates both the hosted agenda and any re-exported `.ics` without duplicates.

---

## 2. What exists today

### 2.1 The InspCal engine (external project)

The project `~/Documents/Parseur PDF vers calendrier` is a static PWA plus a Rust engine in `engine/`. The engine is the reusable part:

- **Crate**: `engine/Cargo.toml` — `inspcal-engine` 0.2.0, `crate-type = ["cdylib", "rlib"]`, so the *same source* builds as WASM for the browser and as a native rlib for tools/embedding. Dependencies are plain Rust (`pdfium-render = { version = "=0.9.4", default-features = false, features = ["pdfium_latest"] }` at `engine/Cargo.toml:29`; `pdfium_latest` selects pdfium-render's **7881-revision binding constants** (`pdfium_latest = ["pdfium_7881"]`, pdfium-render `Cargo.toml:100`), while the PDFium **binary** is pinned as release 8046d by `public/rs/bundle.json` (`pdfium_release`), not by the crate — the PWA already runs 7881 constants against the 8046d binary via dynamic symbol resolution — plus serde, regex, …); PDFium itself is bound **at run time** (native dylib, or a separate WASM module in the browser). The wasm-only deps (`wasm-bindgen`, `js-sys`) are `cfg(target_arch = "wasm32")`, so a native path-dependency compiles clean.
- **Pipeline** (`engine/src/`): `pdfium` → `objects`/`tables`/`text`/`extract` (PDF geometry → grid → sessions, no OCR, no LLM) → `grid`/`events`/`dates`/`rooms`/`validate`/`pipeline` → `timetable` (pivot model + JSON) → `selection` (reviewer choices) → `review`/`convert` → `ics` (RFC 5545 writer with deterministic UIDs, `TZID=Europe/Paris` + `VTIMEZONE`, `X-INSPCAL-RULES`, `STATUS:CANCELLED` for withdrawn sessions when previous exports are supplied).
- **School rules are data, not code**: `engine/institution.json` (compiled in via `include_str!`, `src/institution.rs`) holds cohorts (`FI`, `CIL`, `OFF`), LV1 rooms, the APSA keyword, header aliases, and the Moodle folder URLs. Bumping its `version` is a data-only change.
- **WASM entry point** (`engine/src/browser.rs`, exposed by wasm-bindgen):
  - `new BrowserEngine(fast?)` — binds PDFium and calls `FPDF_InitLibrary()`;
  - `convert(fields: string, files: Array<{name, type, bytes: Uint8Array}>, now: number) -> string` returning `{"status": number, "body": {...}}` (see Appendix A);
  - requires a prior `initialize_pdfium_render(pdfiumModule, module, false)` and then `initialize_fast_pdfium(pdfiumModule)` with the external PDFium WASM module (`public/rs-worker.js:38-39`; `engine/src/pdfium_bindings.rs:229`) — the first call is what makes `BrowserEngine::new()`'s `Pdfium::bind_to_system_library()` (`engine/src/browser.rs:19`) succeed under WASM.
- **Distributed bundle** (`public/rs/`, committed): `engine_js` + `engine_wasm` (2,708,813 B) + `pdfium_js` + `pdfium_wasm` (5,298,237 B) — 8,290,388 B raw, ~3.34 MB gzip — all pinned by SHA-256 in `public/rs/bundle.json` and verified before execution by `public/rs-worker.js` inside a dedicated Web Worker (`public/local-engine.js` owns boot timeouts and error codes such as `LOCAL_BUNDLE_MISMATCH`, `LOCAL_BOOT_FAILED`, `LOCAL_CRASH`).
- **Native entry points already exist**: `engine/src/bin/inspcal-extract.rs` / `inspcal-convert.rs` / `inspcal-maintain.rs` run the same rlib natively, loading a PDFium library from `INSPCAL_PDFIUM` or `Pdfium::bind_to_library(path)`; `src/lib.rs` exposes `pub fn extract(bindings, bytes, file)` and `pub mod convert` with `convert::convert(fields_json, uploads, reader, now)` — i.e. **the engine can be called directly from native Rust without any WASM**, which is the key fact for a Tauri app.
- **Hard limits** (`engine/src/convert.rs:23-25`): max 8 PDFs per request, **4,000,000 bytes total** (4 MB decimal, not 4 MiB), prefix ≤ 60 chars, alarm 0–1440 min, choices maps ≤ 500 entries; files must start with `%PDF-` **and** carry `content_type` `application/pdf` | `application/octet-stream` | empty; per-file failures are reported per file (`NOT_PDF`, `WEEK_DATE_MISSING`, `UNSUPPORTED_PDF`, `WORKER_FAILED`, `REVISION_CONFLICT`) without failing the batch; `UNSUPPORTED_PDF` short-circuits the whole answer (`status: 0, body.engine_error`).
- **Tests/fixtures we can reuse**: `tests/fixtures/public/*.pdf` (14 fictitious weeks), `tests/golden/public/contracts.json` (per-cohort status/UID/calendar-hash contracts), native/WASM parity suite, `tools/build_rust_bundle.py` for bundle provenance. Note: `docs/moodle-engine-sync.md` governs delivering the compiled bundle to the *Moodle plugin* repo (`brunobernardinsp/InspCal`); proposing engine changes back to InspCal itself is a PR to `kilianvivien/InspCal`, and a submodule bump still needs MoodBuro-side contract tests against `contracts.json`.

### 2.2 MoodBuro surfaces this feature touches

- **App shape**: Tauri 2, macOS first. Privileged logic lives in `src-tauri/src/` (Rust); the local UI is the React/TS shell in `apps/shell/src/`. Only the `shell` webview may invoke commands (`src-tauri/capabilities/shell.json`, `local: true`); every command re-checks `shell(&webview)?` (e.g. `commands.rs:47 agenda_export`). CSP in `src-tauri/tauri.conf.json` is `script-src 'self'` with **no `wasm-unsafe-eval`** — relevant for Option B (§3).
- **Moodle access** (`crates/md-moodle`): token-authenticated web services (`course_files` → `core_course_get_contents`, `lib.rs:665`; `upcoming_events` → `core_calendar_get_calendar_upcoming_view`, `lib.rs:678`) and token-authenticated file downloads: `offline.rs:23 file_url()` strictly validates `webservice/pluginfile.php` URLs (same origin, no credentials/fragments, only `forcedownload`/`file` query keys) and `offline.rs:41 save_course_file()` downloads with the token appended inside `md-moodle` as an HTTPS query parameter (`crates/md-moodle/src/offline.rs:49`), never visible to the shell — HTTPS-only, no redirects, 128 MiB cap. `library::session(app, site)` (`src-tauri/src/library.rs:36`) yields `(site, client, token)`.
- **Course file catalog** (`src-tauri/src/offline.rs:183 catalog()`, `resources()` at `:236`): builds the resource list shown in `apps/shell/src/views/CourseFiles.tsx` — each `OfflineResource` (`ipc.ts:76`) carries `key`, `moduleId`, `filename`, `bytes` but **no URL**; Rust re-derives the source URL from `course_files` when acting on a key.
- **Agenda today** (A4): `events` table (`crates/md-store/src/lib.rs:212`, schema 18) caches Moodle upcoming events as `Event { site_id, id, name, time_start, duration, kind, course_id, course_name }` (`lib.rs:131`), rendered by `views/Agenda.tsx`; local reminders run from `reminder_*` tables.
- **Calendar today** (A5 + B2): `src-tauri/src/calendar.rs` renders a token-free ICS snapshot from `events` (stable account-scoped UIDs, atomic staged + hard-link publish into Downloads, Finder reveal); `calendar_subscription.rs` hands the private Moodle feed URL to Apple Calendar on macOS with consent/dedup/history (`calendar_handoffs` table). [calendar-qa.md](calendar-qa.md) explicitly leaves room for "a future Inspcal-derived … provider adapter and … provider-specific ICS export" — this plan is that hook.
- **Shell plumbing**: commands registered in `src-tauri/src/lib.rs` (`generate_handler!`), DTOs in `apps/shell/src/ipc.ts`, FR/EN strings in `apps/shell/src/i18n/messages.ts`, static assets would go in `apps/shell/public/` (copied by Vite; none exists yet).

---

## 3. Integration options

The engine supports two embeddings. Both keep all processing on-device.

| Criterion | **A — native Rust crate in `src-tauri`** (recommended) | **B — WASM in the shell webview** (InspCal's own model) |
| --- | --- | --- |
| How | Path-dependency on the engine rlib (vendored submodule, §4.1); bind native PDFium with `Pdfium::bind_to_library(...)`; call `inspcal_engine::convert::convert(...)` | Ship the 4 assets from `public/rs/` (~8 MB) in `apps/shell/public/rs/`, boot them in a dedicated Worker like `rs-worker.js`, SHA-256-verify against a bundled `bundle.json` |
| CSP | **No change** — no WASM in the webview | Requires adding `wasm-unsafe-eval` to `script-src` in `tauri.conf.json` (shell is local-only, but it widens the shell's attack surface) |
| PDF bytes path | Rust fetches with the token and converts in-process on a `spawn_blocking` thread; bytes stay in memory | Shell must obtain bytes (IPC payload or disk read), post them to the worker |
| PDFium | Must bundle a macOS universal `libpdfium.dylib` (~10–25 MB, lipo'd from the pinned per-arch 8046d wheels, §4.2) via `bundle.resources`; signed with the app | Already solved: `pdfium-*.wasm` in the bundle |
| Performance | Faster (no WASM boundary, no worker), runs while the shell window is hidden, background-refreshable | Proven in InspCal's PWA; ~seconds per PDF in a worker |
| Updates | Engine bump = app release (submodule bump + contract tests) | Same in a packaged app (assets are inside the bundle); "swap the bundle" only helps dev builds |
| Failure modes | Dylib load/signing issues (spike item); app size grows. Current distribution is ad-hoc signed **without** hardened runtime (`package-macos.yml`, no `signingIdentity`/`hardenedRuntime` in `tauri.conf.json`), so a bundled dylib loads today; the post-beta Developer ID + hardened runtime (MAC-9) requires the dylib signed with the same identity, which bundled resources get automatically — re-verify in that spike | WKWebView Lockdown Mode disables WASM (documented for iOS in InspCal's own README; macOS behavior unverified — conversion impossible there if it applies); worker lifecycle in a Tauri webview |
| Code reuse | 100 % of parser logic; wasm-only `browser.rs` is `#[cfg(wasm32)]` and simply not compiled | 100 %, including the proven worker boot/verification code |
| Verified costs | Dylib packaging gated by the P0 spike; app grows by the universal dylib (~10–25 MB, measured in P0); engine/rules updates ride app releases (`institution.json` is `include_str!`-compiled) | CSP needs `wasm-unsafe-eval` added to `script-src 'self'` (`src-tauri/tauri.conf.json:20`); the 4 assets = 8,290,388 B raw / ~3.34 MB gzip (`public/rs/bundle.json`) inside the app bundle **and** again in webview memory; PDF bytes must reach the webview (IPC payload or disk read), against the bytes-stay-in-Rust policy |

**Recommendation: Option A.** MoodBuro is a native app with a privileged Rust core; the PDF is already fetched with the token in Rust, the results belong in SQLite, and Option A avoids touching the CSP, avoids an 8 MB WASM payload in the webview, and lets the conversion run on a background thread (needed later for refresh/corrections). The engine was deliberately written as `rlib` + native bins, so this is a supported configuration, not a hack. **Keep Option B as fallback** if bundling/signing the native PDFium dylib proves problematic in the packaged app; the shell-side review UI and the engine contract (§4.4) are identical either way, so the fallback is cheap.

---

## 4. Detailed design (Option A)

### 4.1 Vendoring the engine and build integration

- Add the InspCal repo as a **git submodule** at `vendor/inspcal` (its engine is at `vendor/inspcal/engine`) and add a thin workspace crate `crates/md-inspcal` that path-depends on `vendor/inspcal/engine` (`inspcal-engine`), re-exporting what MoodBuro needs. A wrapper crate keeps `src-tauri` decoupled from upstream layout changes and gives us one place for our own sanitization layer. Alternative: subtree-copy `engine/` into `crates/inspcal-engine`; submodule is preferred because upstream sync stays a plain `git submodule update` (see the provenance note in §2.1 on `docs/moodle-engine-sync.md`).
- Workspace: `Cargo.toml` already globs `crates/*`, so `crates/md-inspcal` joins automatically. Toolchain: InspCal pins Rust 1.91.1 (`engine/rust-toolchain.toml`); MoodBuro workspace requires 1.91 — compatible, confirm in the spike.
- `institution.json` is compiled in via `include_str!` — rules updates are data commits in the submodule, still requiring an app rebuild under Option A (acceptable; recorded as a version contract like InspCal's `parser_version` / rules version, both of which we persist per export, §4.5).
- CI: the workspace `cargo test`/`clippy` will now compile the engine; add a MoodBuro-side contract test (below) and keep InspCal's own `contracts.json` as the upstream regression net when bumping the submodule.

### 4.2 Native PDFium library

- Bundle the **same pinned PDFium build (release 8046d)** the WASM bundle uses, as a macOS universal `libpdfium.dylib`. Provenance is by **archive/wheel hash**, not by any Cargo feature (§2.1): the pinned archive in `public/rs/bundle.json` (`pdfium_archive_sha256`) and the `pypdfium2_raw` wheels (already used for the native tools) are single-arch and dev-only, so the universal dylib must be assembled with `lipo` from the two pinned per-arch wheels, with all input hashes recorded next to `pdfium_archive_sha256`. Ship it as a bundled resource (`bundle.resources`, loaded from `app.path().resource_dir()`) — `bundle.externalBin` is Tauri's sidecar-**executable** mechanism for spawned `Command`s; both are signed with the app, resources is the documented dylib path.
- At first use (lazy, once per process): `Pdfium::bind_to_library(path)` → `FPDF_InitLibrary()` → keep the bindings in a `OnceLock` inside `md-inspcal`, mirroring `browser.rs`. Load failure maps to a fixed, sanitized error (`ENGINE_UNAVAILABLE`), never a path/URL.
- Thread-safety: PDFium is **not** thread-safe ("Pdfium itself is not thread-safe", pdfium-render `src/bindings/thread_safe.rs:4-6`), and pdfium-render 0.9.4 only declares `Send + Sync` behind its optional `thread_safe` feature (`src/pdfium.rs:625-629`), which the engine does not enable (`engine/Cargo.toml:29`). `crates/md-inspcal` must declare `pdfium-render = { version = "=0.9.4", default-features = false, features = ["thread_safe", "pdfium_latest"] }` so Cargo feature unification gives the engine mutex-marshalled bindings too (`ThreadSafePdfiumBindings`). Even then, hold one `OnceLock` binding and serialize conversions behind our own mutex (like the offline job lock in `AppState`) — it bounds CPU and the ~2×PDF + base64 peak memory. Concurrent `timetable_convert` calls stay disallowed (async waiters on the mutex), revisited only if the P0 spike shows a need (prove with two concurrent converts in one Rust test).
- License: engine MIT, PDFium Apache-2.0 — extend the app's third-party notices (InspCal ships `public/rs/THIRD_PARTY_NOTICES.txt` to reuse) and the in-app About/legal section.
- Spike gate: prove the dylib loads and converts one fixture inside the **packaged, ad-hoc-signed app** (library validation / hardened runtime), and record the final app size delta.

### 4.3 Fetch path: Moodle PDF → bytes, without exposing URLs or tokens

1. Shell sends only opaque identifiers: `timetable_convert { siteId, userId, courseId?, resourceKey? }` (course catalog entry), or `{ downloadId }` for an already-saved download (read its `local_path` from the `downloads` table after managed-path validation, like `offline.rs` does).
2. Rust (`src-tauri/src/timetable.rs`, new module): `library::session(app, site)` → re-validate `saved.user_id == user` (same pattern as `offline::resources`), call `client.course_files(&token, course_id)`, rebuild the catalog, find the resource by `key`, and re-derive its URL with `md_moodle::offline::file_url()` (strict same-origin `webservice/pluginfile.php` validation).
3. Download **into memory** with a new `md_moodle::offline::fetch_course_file(token, raw, limit) -> Result<Vec<u8>, String>`: same hardening as `save_course_file` (HTTPS-only, no redirects, 180 s timeout, `permits` semaphore — 4 permits, `crates/md-moodle/src/lib.rs:481`) but a **4,000,000-byte cap** (the engine's `MAX_TOTAL_UPLOAD_BYTES`, `engine/src/convert.rs:24` — 4 MB decimal, not 4 MiB) and no disk write. `receive_limited`'s error strings hardcode "128 MB" (`crates/md-moodle/src/offline.rs:95,129`), so the limit and message must be parametrized before reuse. The PDF bytes never touch SQLite, logs, or IPC. The shell-side pre-check must compare against 4,000,000 as well.
4. Filename passed to the engine comes from the catalog (`safe_name`d), so the engine's week-date-from-filename logic (`Edt du JJ.MM.AAAA au …`) keeps working; if absent, the shell asks for the week's Monday (`week_start` field).

### 4.4 Engine request/response contract (unchanged from InspCal)

Call `inspcal_engine::convert::convert(fields_json, &uploads, &mut reader, now)` on a `spawn_blocking` thread, where `reader` wraps `inspcal_engine::extract(&BINDINGS, bytes, name)` and `now` is epoch seconds.

- `uploads`: `vec![Upload { name, content_type: Some("application/pdf"), bytes }]`.
- `fields` (JSON string; see Appendix A for the full table): `cohort`, `prefix`, `alarm`, `split_by_module`, `week_start(s)`, the five `*_choices` maps, `timetables` (previously parsed weeks — the **review round-trip**), `exported` (previous exports for correction diffs).
- Response `{"status", "body"}`: `body.files[]` per-file outcomes (`pending|included|superseded|failed`, incl. `REVISION_CONFLICT` as a failure code), `body.timetable` (pivot JSON, with `warnings` **inside** it — there is no top-level `body.warnings`), the question lists (`group_sessions`, `half_promotion_weeks`, `time_sessions`, `sport_weeks`, `lv1_sessions`, `lv1_rooms`, `lv1_rotation_url`, `participation_sessions`), `body.parsed` (`[{"outcome", "timetable"}]` — the round-trip payload to send back as `timetables`), `body.exportable` (`[{"timetable", "cancelled"}]` — the payload to send back as `exported` on the next conversion), `body.download` (`{filename, mime, base64}` ICS or ZIP), `body.parser_version`, `body.partial`. `status 200` = timetable present; `status 0` + `engine_error: "UNSUPPORTED_PDF"` = not a known timetable; 4xx refusals carry a French `detail` message that we map to our own i18n keys.
- **Two-step review flow** (avoids sending the PDF twice): call 1 with the PDF returns `timetable` + questions; the shell renders choices; call 2 sends `timetables = [{"timetable": <stored pivot JSON>}]` plus the `*_choices` fields and receives the final `download`. This is exactly InspCal's own flow and is already supported by `restore_timetables`.

### 4.5 IPC surface and DTO sanitization

- New commands in `src-tauri/src/commands.rs`, all guarded by `shell(&webview)?`, registered in `lib.rs`, and added as `allow-timetable-*` permissions in `capabilities/shell.json`:
  - `timetable_convert { siteId, userId, courseId?, resourceKey?, downloadId?, fields }` → sanitized `TimetableResult` DTO (below). Async; runs fetch + engine off the main thread; per-request timeout (~30 s + 15 s/file, mirroring InspCal's worker timeouts).
  - `timetable_weeks { siteId, userId }` → list of hosted weeks (from §4.6) for the Agenda/import UI.
  - `timetable_export_ics { siteId, userId, weekKey }` → re-render ICS from the stored pivot (no re-parse) and publish via the existing atomic staged + hard-link pattern from `calendar.rs::export`.
  - `timetable_remove { siteId, userId, weekKey }` → delete hosted week (cascade).
- **DTO rules** (matching the existing "no URLs/credentials in IPC" policy): the shell DTO carries only ids, filenames, counts, the question/warning structures, the pivot timetable JSON (it contains session data incl. Moodle *links* — validate each link is same-origin HTTPS with the site base before forwarding; drop otherwise), `parser_version`, and the base64 download only on the final confirm call. No source URL, no token, no local path.
- Errors are codes + i18n keys (`NOT_PDF`, `UNSUPPORTED_PDF`, `WEEK_DATE_MISSING`, `ENGINE_UNAVAILABLE`, `TOO_LARGE`, `NETWORK`, …), rendered through `i18n/ErrorText.tsx` like other shell errors.
- CI mechanics: `scripts/check-security.py` (run in `.github/workflows/ci.yml`) statically asserts exactly two capability files and that the count of `    shell(&webview)?;` guard lines equals the count of `#[tauri::command]` in `commands.rs` — every new command must contain that exact guard line or CI fails; permissions go into `capabilities/shell.json` only. Timeouts are not a Tauri feature: implement the ~30 s + 15 s/file budget (`public/local-engine.js` `BASE_TIMEOUT_MS`/`PER_FILE_TIMEOUT_MS`; boot is 90 s) as `tokio::time::timeout` around the `spawn_blocking` await. Cap the base64 `download` in the DTO (an ICS week is small, but assert it).

### 4.6 Storage (schema 19) and hosting in the Agenda

New tables in `crates/md-store` (schema 19, cascading on site removal like every other table):

- `timetable_weeks (site_id TEXT, week_key TEXT, course_id, resource_key, filename, sha256, week_start, timetable_json, parser_version, rules_version, parsed_at, updated_at, PRIMARY KEY(site_id, week_key))` — `week_key` = the engine's own week identity (week start + end + promotion, as `week_of`/`same_week` match it, `engine/src/ics.rs:511,632`), **not** the source hash: a corrected republished PDF changes the sha256 and must re-use the same row, otherwise the corrections flow duplicates weeks and breaks the `exported` correlation. `sha256` stays a plain column for refresh/change detection. `timetable_json` is the engine's pivot `to_json()` so any later re-export/re-render needs no PDF and no re-parse.
- `timetable_events (site_id, uid TEXT, week_key, summary, location, description, links, dtstart, dtend, cancelled, PRIMARY KEY(site_id, uid), FOREIGN KEY(site_id, week_key) REFERENCES timetable_weeks(site_id, week_key))` — one row per session, **keyed by the engine's stable ICS UID**, which is what makes re-import update-in-place instead of duplicating. Derived from `timetable_json` in Rust after each conversion (never hand-edited).
- `timetable_exports (site_id, week_key, exported_json, exported_at)` — feeds the engine's `exported` field on the next conversion so corrections produce `STATUS:CANCELLED` and a change summary.
- The three tables must also be added to the account-replacement wipe list in `md_store::Store::update` (`crates/md-store/src/lib.rs:849`, the `for table in [...]` loop), not only to `ON DELETE CASCADE` — `remove()` cascades, but `update()` with a new `user_id` does not.
- Timezone: the engine's pivot carries `Europe/Paris` **wall times** (and its ICS carries `TZID=Europe/Paris` + `VTIMEZONE`, `engine/src/ics.rs:25,429+`), while MoodBuro stores epoch seconds (`calendar.rs` emits UTC `DTSTART`). Deriving `dtstart/dtend` for `timetable_events` must interpret date+time in `Europe/Paris` via a tz database (e.g. chrono-tz), **never the device-local timezone**, and DST-boundary sessions must be checked against the engine's `VTIMEZONE`.

Hosting in the Agenda:

- `agenda_list` merges `events` (Moodle) with `timetable_events` (joined via `timetable_weeks` for the site), surfaced as `AgendaEvent` with `kind: "timetable"`, `courseName` from the source course, and the engine UID kept as the event identity.
- Reminders: the reminder pipeline keys on integer `event_id`; for timetable events derive a stable `i64` from the first 63 bits of SHA-256(uid) with an in-site uniqueness check (or, cleaner, extend `reminder_*` with a nullable uid key — decide at implementation; either way, one reminder per session, same 24 h/1 h defaults).
- A5 export (`calendar.rs::render`) includes hosted timetable events, keeping the **engine UIDs verbatim** — `edt-insp-<sha1[:16]>`, no `@` domain (`engine/src/timetable.rs:16,44-48,150-155`) — so an external calendar app updates instead of duplicating on re-export, and so imports made earlier through InspCal's own PWA keep matching (the engine also emits `X-INSPCAL-LEGACY-UID` for the pre-release content-hash scheme, `engine/src/ics.rs:363-368`). These strings differ from the A5 Moodle-scoped UIDs (`…@moodburo.local`, `src-tauri/src/calendar.rs:62`), which [calendar-qa.md](calendar-qa.md) already notes are a different scope. Open: some calendar apps expect an `@domain`-form UID — keep verbatim for PWA compatibility unless QA shows a problem (§7). The existing A5 future-only filter (`store.events(now - 3600)`, `src-tauri/src/calendar.rs:124`) applies to timetable events too — past hosted weeks are intentionally not in a forward-looking snapshot; state this in the UI so QA does not read it as a bug.
- Corrections: when a new PDF for the same week is converted, pass `exported` from `timetable_exports`; the engine returns `cancelled` sessions and revision info; we upsert `timetable_events` (mark `cancelled=1`) and show a "what changed" summary in the review UI. Pass only that week's stored export — `restore_exported` refuses more than 8 weeks outright (bound `MAX_UPLOAD_FILES`, `engine/src/convert.rs:23`), so never dump every hosted week into `exported`.

### 4.7 Outputs

- **Save .ics**: decode `body.download.base64`, write with the existing atomic staged-file + `hard_link` publish into Downloads (`calendar.rs::export` pattern), then `opener().reveal_item_in_dir`. ZIP (split-by-module) handled the same way. No quarantine xattr needed — the file is generated locally from local data, same as the A5 export.
- **Open in Calendar (macOS)**: reuse the consent + dedup + `calendar_handoffs` machinery from `calendar_subscription.rs`, but open the saved `.ics` **file** with the `NSWorkspace` file-open variant targeting `com.apple.iCal` — that module opens an HTTPS feed URL (`open_native`, `calendar_subscription.rs`), a local file needs the file-URL open variant; never a shell, `webcal://` or default-handler path. Record the handoff with provider `inspcal` so repeat-open needs confirmation, exactly like the stock feed.
- **Host in MoodBuro**: the default — upsert §4.6 and navigate to the Agenda filtered to the week.

### 4.8 Shell UI

- **Entry points**:
  - `CourseFiles.tsx`: on catalog rows whose `filename` ends in `.pdf`, add a **"Convert to calendar"** row action (next to the offline checkbox) → `timetable_convert` with `{siteId, courseId, resourceKey}`. Do not gate on the listed `bytes` — it is `number | null` (`ipc.ts:83`) and Moodle often omits it; the 4,000,000-byte cap is enforced by the §4.3 fetch.
  - `Downloads.tsx`: same action on downloaded PDFs (`downloadId`).
  - `Agenda.tsx`: an **"Import timetable PDF"** button offering already-saved PDFs (offline copies + downloads) — a disk pick can come later via the already-initialized dialog plugin (`src-tauri/src/lib.rs:72`); it still needs permission entries in `capabilities/shell.json` plus managed-path validation.
  - Optional later: drag-and-drop a PDF onto the shell window, and `moodledesk://` deep-link routing to a PDF.
- **Review screen** (new `views/TimetableReview.tsx`, in MoodBuro's design language per [design-guidelines.md](design-guidelines.md)): week summary (dates, session count), the engine's question lists rendered as choice controls (half-promotion A/B, workshop group per session, LV1 room from `lv1_rooms` + rotation link, time override, sport keep/exclude), warnings list with the codes from InspCal's `docs/warnings.md`, options (cohort select, prefix, alarm, split-by-module), and three outcomes: **Add to MoodBuro**, **Save .ics**, **Open in Calendar**. All strings in `i18n/messages.ts`, FR + EN.
- **Agenda integration**: hosted weeks show as a distinct `kind` with a small tag; the existing export/subscribe header actions keep working and now include timetable sessions.

### 4.9 Security and privacy checklist

- All conversion local: no network, no upload path, PDF bytes in memory only, never logged, never persisted; staging cleanup at startup already removes `.moodburo-staging` payloads — our flow adds no files at all.
- No token, signed URL, or source URL crosses IPC (DTO rules §4.5); `file_url()` validation reused; token appended only inside `md-moodle`, HTTPS-only, redirects disabled.
- CSP unchanged under Option A; new commands are shell-only via capability + `shell(&webview)?` runtime guard; IPC-denial probe in QA must cover the new commands.
- Quarantine policy untouched (we never write the PDF); exported `.ics` is generated content like the A5 export.
- Engine errors sanitized to codes; `UNSUPPORTED_PDF` gets a friendly message ("this PDF isn't a recognized INSP timetable").
- GDPR: the PDF may contain other people's data (intervenants); it stays on-device, but exported `.ics` files can be shared by the user — state this once in the review screen. Hosted data is covered by the existing account-removal cascade (SEC-8 wipe includes the new tables).
- The engine is INSP-specific (French headers, FI/CIL/OFF, APSA, `moodle.insp.gouv.fr` links in `engine/institution.json`): on non-INSP sites the action ends in `UNSUPPORTED_PDF`. Gate the entry points on the site's `wwwroot` host (`md_store::Site.wwwroot`) matching the `institution.json` host (`moodle.insp.gouv.fr`) instead of showing an action that usually fails; the engine still refuses unknown PDFs defensively even on the INSP site.

---

## 5. Phasing and acceptance criteria

| Phase | Content | Acceptance |
| --- | --- | --- |
| **P0 — spike (1–2 days)** | Submodule + `crates/md-inspcal` with `thread_safe` enabled on pdfium-render (§4.2); native PDFium dylib bundled and loading in dev **and** packaged app; convert one fixture from `tests/fixtures/public/` end-to-end in a Rust test, incl. two concurrent converts behind the mutex; measure size/time (dylib + chrono-tz tzdata ≈ 1 MB) | Fixture converts natively with expected event count; `thread_safe` feature unification compiles clean and CI stays green; packaged app loads the lipo'd dylib (ad-hoc signed); decision A-vs-B recorded as an ADR |
| **P1 — backend pipeline** | §4.3 fetch (4 MB cap, in-memory), §4.4 convert wrapper, §4.5 commands + capabilities + DTO sanitization, timeouts, error mapping | Unit tests: fixture convert via `timetable_convert`-equivalent; URL/token absence in DTOs (assert on serialized DTO); non-shell webview denied; >4 MB refused |
| **P2 — review UI** | §4.8 review screen with choices round-trip (`timetables` field), warnings, options; FR/EN | Vitest component tests; manual pass on a real INSP PDF (private, never committed) matching InspCal's review |
| **P3 — outputs** | §4.6 schema 19 (current schema is 18, `crates/md-store/src/lib.rs:238`) + hosting in `agenda_list`; §4.7 save/open/reveal ICS; A5 export inclusion; reminders for hosted sessions | Hosted sessions appear beside Moodle events; re-import updates in place (same UID, no dupes); ICS re-import into Calendar updates, not duplicates; reminder fires on a hosted session |
| **P4 — corrections & refresh** | `exported` flow, `timetable_exports`, change summary UI; optional scheduled re-check of the source PDF (sha256 compare via existing `course_sync` machinery) prompting re-conversion | Republished week shows changed/cancelled sessions; hosted rows marked cancelled; re-export emits `STATUS:CANCELLED` |
| **P5 — polish** | Downloads/Agenda entry points, drag-and-drop, deep link, About/legal notices, QA doc `calendar-pdf-qa.md`, release notes | Full QA matrix incl. IPC-denial probe, Lockdown note n/a (Option A), live INSP account pass recorded |

---

## 6. Testing and CI

- **Rust**: contract test in `crates/md-inspcal` — copy one fictitious fixture PDF from `vendor/inspcal/tests/fixtures/public/` (14 available) into MoodBuro fixtures; assert status 200, expected session count, and that a second run with `exported` yields the golden UID set (reuse InspCal's `contracts.json` hashes for one cohort if feasible). DTO sanitization tests (no `http`, no token in serialized output). `cargo clippy -D warnings` already workspace-wide. Note: the engine's `[profile.release]` settings (`opt-level = "s"`, `lto`, `panic = "abort"`, `engine/Cargo.toml`) are **ignored** when the crate is built as a path-dependency of the MoodBuro workspace — profiles come from the root `Cargo.toml` — so no `panic = "abort"` reaches the app; confirm size/perf in P0.
- **Frontend**: vitest for the review component (choices round-trip, warning rendering, disabled states for >4,000,000 bytes), following `calendar.test.tsx` patterns; i18n key parity test already exists (`i18n.test.ts`).
- **IPC guard**: `scripts/check-security.py` is a static guard (capability files, guard-line count); the runtime IPC-denial probe stays a QA gate and must cover the four new commands.
- **Manual QA**: live INSP Moodle account; packaged app; re-import idempotency in Apple Calendar; correction flow; account removal cascade clears timetable tables; size/memory measurements vs the <120 MB idle target.
- Private real PDFs stay gitignored, exactly like InspCal's policy; CI uses only fictitious fixtures.

---

## 7. Risks, limits, open questions

- **Native PDFium dylib in a packaged app** (signing/library validation, size ~10–25 MB for the universal build — only the archive SHA-256 is pinned today, so this is an estimate until P0 measures it; no prebuilt universal 8046d dylib exists in either repo, it must be `lipo`'d from the pinned per-arch wheels) — biggest technical unknown; P0 spike decides A vs B. Today's ad-hoc, non-hardened build loads a bundled dylib; the post-beta Developer ID + hardened runtime (MAC-9) needs same-identity signing, which bundled resources get automatically — unproven until that spike. Fallback B costs a CSP `wasm-unsafe-eval` addition (`tauri.conf.json:20`) plus WASM-off under Lockdown Mode (iOS-documented; macOS WKWebView unverified).
- **Engine is INSP-specific**: gated on the site `wwwroot` host per §4.9; even on the INSP site, an unknown PDF yields `UNSUPPORTED_PDF`.
- **Timezone**: engine emits `Europe/Paris` wall times; MoodBuro stores epoch seconds — derive `dtstart/dtend` via a tz database (chrono-tz), never device-local (§4.6), and check DST-boundary sessions against the engine's `VTIMEZONE` output.
- **UID contract**: engine UIDs are bare `edt-insp-<sha1[:16]>` (no `@domain`, `engine/src/timetable.rs:16`) and their inputs include parser code (`identity_title` folding), so a parser-version bump can change UIDs and duplicate events in external calendars; pin parser/rules versions per week in `timetable_weeks`, treat submodule bumps as migration events, and use `contracts.json` as the regression net — on a deliberate bump decide re-render vs. matching via `X-INSPCAL-LEGACY-UID`. Whether Apple/Google/Outlook dedupe on bare UIDs is untested; P3/P4 import tests must cover it (keeping them verbatim preserves compatibility with PWA-imported calendars).
- **External-calendar tombstones unverified**: what Apple Calendar does with `STATUS:CANCELLED` on re-import is explicitly unverified upstream (InspCal README) and in MoodBuro's QA backlog ([calendar-qa.md](calendar-qa.md)) — the whole "no duplicates on correction" story for external apps rests on it; P4 acceptance must include a real re-import test.
- **Rules updates ride app releases** under Option A (`institution.json` is `include_str!`-compiled, `engine/src/institution.rs:12`): a new LV1 room ships only with an app update — accepted cost, unlike the PWA's swappable bundle.
- **Reminder identity**: integer-keyed reminder tables (`reminder_events`/`reminder_picks`, 24 h/1 h offsets at `src-tauri/src/reminders.rs:174-175`) need the uid→i64 SHA-256 derivation with an in-site uniqueness check, or a nullable-uid schema tweak (decide in P3).
- **Not in scope for v1 of this feature**: writing events back to Moodle, editing sessions, multi-week batch import beyond the engine's 8-PDF / 4,000,000-byte limits (the `timetables` round-trip is likewise capped at 8 items, choices maps at 500), OCR/other PDF formats.
- **Open question**: should hosted timetable sessions also be pushed to Apple Calendar automatically (EventKit) — PRD's OS-8 ambition — or stay behind the existing manual export/subscription flows? Recommend staying manual in this slice.

---

## 8. Rough effort

P0 spike 1–2 days; P1 ~1 week; P2 ~1 week; P3 ~1–1.5 weeks; P4 ~0.5–1 week; P5 ~0.5 week. Total roughly **4–6 weeks** including QA, dominated by the PDFium packaging spike and the review UI. Option B fallback adds ~1 week (worker port + CSP change + asset pipeline) but removes the dylib risk.

---

## Appendix A — engine API reference (as found in `vendor/inspcal/engine/src`)

**Call (native)**: `inspcal_engine::convert::convert(fields_json: &str, files: &[convert::Upload], reader: &mut dyn FnMut(&[u8], &str) -> Result<Document, ExtractError>, now: i64) -> String`, with `reader = |bytes, name| inspcal_engine::extract(BINDINGS.as_ref(), bytes, name)` after `Pdfium::bind_to_library(path)` + `FPDF_InitLibrary()` — the proven pattern of `engine/src/bin/inspcal-extract.rs:47`. **WASM equivalent**: `new BrowserEngine().convert(fieldsJson, [{name, type, bytes: Uint8Array}], nowSeconds)` → same JSON string.

**`fields`** (all strings; unknown/invalid → 4xx refusal with `detail`):

| Field | Meaning |
| --- | --- |
| `cohort` | `ALL` (default) or one of `institution.json` `cohort_tokens` (`FI`, `CIL`, `OFF`) |
| `prefix` | ≤ 60 chars prepended to event titles |
| `alarm` | reminder minutes, 0–1440, optional |
| `split_by_module` | `true` → ZIP of one calendar per module instead of a single `.ics` |
| `week_start` / `week_starts` | ISO Monday per file (needed when the filename carries no date and >1 file) |
| `group_choices` | JSON map uid → group label |
| `half_promotion_choices` | JSON map week → `A`/`B` |
| `room_choices` | JSON map uid → LV1 room (empty = undecided) |
| `time_choices` | JSON map uid → `{start, end}` |
| `sport_choices` | JSON map uid → `keep`/`exclude` |
| `timetables` | JSON array of previously parsed weeks (`[{"timetable": …, "outcome": …}]`) — the review round-trip; mutually exclusive with PDF upload |
| `exported` | JSON array of the reader's previous export timetables, for correction diffs |

**Response** `{"status": u16, "body": …}`: `body.files[]` = `{index, filename, status: pending|included|superseded|failed, error?: {code, message}, event_count?}` (failure codes incl. `NOT_PDF`, `WEEK_DATE_MISSING`, `UNSUPPORTED_PDF`, `WORKER_FAILED`, `REVISION_CONFLICT`); `body.timetable` (pivot JSON or null; `warnings` live inside it); `body.download` = `{filename, mime, base64}` or null; `body.group_sessions`, `body.participation_sessions`, `body.half_promotion_weeks`, `body.time_sessions`, `body.sport_weeks`, `body.lv1_sessions`, `body.lv1_rooms`, `body.lv1_rotation_url` (review questions); `body.parsed`, `body.exportable`, `body.partial`, `body.parser_version`. `status` 200 = timetable present; 0 = `body.engine_error` (e.g. `UNSUPPORTED_PDF`); 4xx = refusal (`body.detail`, French). Limits: ≤ 8 files, ≤ 4,000,000 bytes total, files must start with `%PDF-` and have `content_type` `application/pdf` | `application/octet-stream` | empty.

**ICS guarantees** (`engine/src/ics.rs`): RFC 5545, `PRODID:-//edt-parser//INSP//FR` (`:23`), `TZID=Europe/Paris` with embedded `VTIMEZONE` (`:25,429+`), deterministic bare UIDs `edt-insp-<sha1[:16]>` per unchanged session (`engine/src/timetable.rs:16,44-48`), `X-INSPCAL-LEGACY-UID` for the pre-release scheme (`ics.rs:363-368`), `X-INSPCAL-RULES` = rules version, `STATUS:CANCELLED` for sessions withdrawn since a supplied `exported` week, VALARM when `alarm` set.

## Appendix B — file-by-file change list (Option A)

- `vendor/inspcal` — new git submodule (InspCal repo).
- `crates/md-inspcal/` — new wrapper crate: path-dep on the engine rlib, lazy PDFium binding (`OnceLock`), `convert` wrapper with limits/timeouts, pivot→`timetable_events` derivation, DTO sanitization.
- `crates/md-moodle/src/offline.rs` — add in-memory `fetch_course_file(token, raw, limit)` (same hardening as `save_course_file`).
- `crates/md-store/src/lib.rs` — schema 19: `timetable_weeks`, `timetable_events`, `timetable_exports` (+ cascade on site removal, migrations, tests).
- `src-tauri/src/timetable.rs` — new: fetch→convert pipeline, command implementations.
- `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` — register `timetable_convert`, `timetable_weeks`, `timetable_export_ics`, `timetable_remove` with `shell(&webview)?` guards.
- `src-tauri/capabilities/shell.json` — `allow-timetable-*` permissions.
- `src-tauri/tauri.conf.json` — `bundle.resources` for the universal `libpdfium.dylib` (lipo'd from the two pinned per-arch 8046d wheels, hashes recorded next to `pdfium_archive_sha256`); no CSP change.
- `src-tauri/src/calendar.rs` — include hosted timetable events in the A5 ICS render (engine UIDs).
- `src-tauri/src/calendar_subscription.rs` — provider `inspcal` handoff for "Open in Calendar" (reuse consent/dedup machinery).
- `apps/shell/src/ipc.ts` — DTO types + `ipc.timetable*` wrappers; `apps/shell/src/views/TimetableReview.tsx` (new), `CourseFiles.tsx`, `Downloads.tsx`, `Agenda.tsx` — entry points and hosting; `apps/shell/src/i18n/messages.ts` — FR/EN strings.
- `docs/` — this plan, later `calendar-pdf-qa.md`, an ADR for the A/B decision, release notes.
- CI — engine compile + contract tests, IPC guard coverage, size budget check.
