# The Rust engine and maintainer tools

Rust is the sole parser, natively for repository maintenance and as WebAssembly in the browser. The two builds share extraction, timetable logic, review choices, identity and calendar serialization. Python is only development orchestration and the packaging source of a native PDFium library.

## Build

```bash
uv sync --frozen --dev
cd engine
cargo build --release --locked
cargo install wasm-bindgen-cli --version 0.2.129 --locked
cd ..
uv run python tools/build_rust_bundle.py
```

`tools/build_rust_bundle.py --check` rebuilds and compares the committed runtime byte for byte. That holds on the machine that built it only: another host produces different WebAssembly from the same sources, so CI relies on `tests/test_rust_bundle.py` (source fingerprint and manifest hashes) and on the native/WASM parity run instead. `--wasm-bindgen PATH` selects an explicit packager and `--pdfium-archive PATH` supplies the pinned 8046d download. The source fingerprint includes the engine and `institution.json`. Cargo's package version is the parser version; do not change it casually because calendar identity is a regression contract.

## Proof-read a week

An explicit `Demi-promo A/B` heading in Thème (or `½ promo A/B` in Cohorte)
is retained as `raw.half_promotion`. The heading may name its cohorts first
(`CIL Demi-promo A`, `FI-OFF 2026-2028 Demi-promo B`), or share its paragraph
with the cohorts and title (`½ promo B - FI-CIL-OFF : Note d'analyse
collective`); both forms appear in the 16–20 November 2026 export. Review offers one A/B choice per week,
filtering only labelled sessions and keeping common sessions. Numbered and
lettered workshop groups remain separate choices within the chosen half;
their membership is never inferred across modules. One exception within a module:
a session printing no half, under the same merged Module cell as labelled
sessions before and after it that all name one half, takes that half and its
heading (9–13 November 2026: Friday's role plays for groups 7–12 sit between
two `1/2 promo B` sessions and mirror Thursday's demi-promo A). The full session title
appears before its demi-promo heading in the calendar.

`src/group-schedules.json` holds group times supplied outside the PDF. Each
entry must match the session's date, title and printed time window exactly;
printed group details take precedence. These are offered during review only,
so the parsed PDF and an export without a choice retain the original window.
The 20 October 2026 ANR entry was supplied by the user: A 13:00–14:00,
B 14:00–15:00, C 15:00–16:00, D 16:00–17:00, E 17:00–18:00.
The 30 October management entries come from the user-supplied
`Roulement par groupes ateliers 30 oct.pdf`, visually checked against its four
rows. The general timetable remains the only imported PDF. A separate Friday
A–D group choice applies to all four workshops, independently of Thursday's
demi-promotion and workshop groups. Slots are 09:00–10:25, 10:40–12:05,
13:30–14:55 and 15:10–16:35. The general timetable's titles, speakers, rooms and
membership link are retained; the inclusive-practices workshop has a slightly
different title in the supplemental PDF, so matching uses the general title.

These are internal repository tools, not a replacement user-facing CLI or skill. Load a native PDFium library explicitly:

```bash
export INSPCAL_PDFIUM=$(uv run python -c "import pypdfium2_raw,pathlib; print(next(p for p in pathlib.Path(pypdfium2_raw.__file__).parent.iterdir() if p.suffix in ('.dylib','.so','.dll')))")
engine/target/release/inspcal-maintain check "tests/fixtures/public/Edt du 07.09.2026 au 11.09.2026.pdf"
engine/target/release/inspcal-maintain parse "tests/fixtures/public/Edt du 07.09.2026 au 11.09.2026.pdf" --output /tmp/week.json
engine/target/release/inspcal-maintain debug "tests/fixtures/public/Edt du 07.09.2026 au 11.09.2026.pdf" --output debug
```

`check` prints sessions, group details, warnings and remote/cancellation prompts. `parse` writes pivot JSON. `debug` writes one standalone SVG per page: extracted text and numbered cell/table outlines in PDF coordinates. It works before date/event validation, so a broken date can still be diagnosed. The overlay substitutes fonts; compare it with the original PDF, not as a pixel-identical page rendering.

A new private week joins the corpus with:

```bash
engine/target/release/inspcal-maintain add-week "/path/Edt du 19.10.2026 au 23.10.2026.pdf" --dry-run
engine/target/release/inspcal-maintain add-week "/path/Edt du 19.10.2026 au 23.10.2026.pdf"
```

The tool parses before writing, refuses existing fixtures or goldens unless `--replace` is explicit, and keeps all cohorts. `--corpus PATH` selects another corpus root. Both private outputs stay gitignored. Proof-read the JSON and warnings against the original before trusting the golden; it is not proof simply because the tool generated it.

`add-week` also writes the four cohort calendar hashes to `golden/contracts.json`; proof-read the new output before treating that contract as a reference. `tests/golden/public/contracts.json` and the optional private `tests/golden/contracts.json` pin four cohorts' statuses, UIDs and calendar hashes. Existing contracts were frozen from the retired Python engine and must never be regenerated just to pass a failing test.

## Lower-level diagnostics

`inspcal-extract --pdfium LIBRARY week.pdf` prints the extracted document.
`inspcal-convert parse --document document.json week.pdf` checks logic without extraction.
`inspcal-convert convert --pdfium LIBRARY` reads a JSON request on stdin and answers `{status, body}`. Files have `name`, `type`, `path`; the request contains `fields` and `files`.
`inspcal-convert calendars` reads timetable JSON and retains correction tombstones for maintainer checks. The page still exports selected revisions only.

## Release gates

- `cargo test --release --locked` and both native/WASM Clippy builds.
- `uv run pytest`: frozen adversarial cases, maintenance commands and runtime freshness.
- `npm run test:rust`: every available public/private PDF, four cohorts, immutable goldens/calendar hashes, native/WASM equality and every offered review choice, including weeks with more than 30 warnings.
- `npm run test:e2e`: the real Rust engine through the interface, local failures, offline install, review and download. WebKit offline reload has a Playwright limitation; Chromium covers that case.
- `npm run benchmark:rust -- "path/to/week.pdf"`: stock and cached PDFium bindings, five samples after warm-up, timed inside the worker without Playwright JSON transport.

UIDs and calendar bytes must remain compatible. The only geometric golden allowance is 0.1 pt on `raw.bbox`; parse time and legacy private fixture hashes are volatile. Notices are served in `public/rs/THIRD_PARTY_NOTICES.txt`.
