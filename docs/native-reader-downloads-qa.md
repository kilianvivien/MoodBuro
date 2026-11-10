# Course-aware downloads and native reader QA

Implemented 10 November 2026. Native checks below used synthetic documents, not real account data.

## Behavior

- Downloads → Refresh Course Folders indexes enrolled courses and module context IDs through `core_course_get_contents` with `excludecontents=1`. Selecting an organisation limits the refresh. Request/permission failures are shown; the app does not scrape Moodle pages or persist file URLs/tokens.
- New `pluginfile.php` downloads with an unambiguous indexed context are published to `~/Moodle/<organisation>-<uuid>/<course-short-name>-<course-id>/`. Unmatched downloads remain in `Downloads`. Course names/IDs are recorded with the history row. Existing files/history remain in place and retain Unknown Course until a new transfer supplies a mapping.
- Downloads can be filtered by organisation, course (after choosing an organisation), Unknown Course, filename and transfer state. Course IDs are scoped by organisation; ambiguous mappings fall back safely.
- Preview supports completed PDF, DOCX, XLSX and PPTX downloads on macOS. Open remains a separate default-app action. File paths and contents never cross IPC; Rust validates the managed completed download before creating a native Tauri document window.
- PDFs use PDFKit with previous/next page, page count, zoom, fit and scale readout. Office uses Quick Look inside a magnifiable native scroll view, with zoom, fit and readout; page/sheet/slide navigation follows the system preview's scrolling behavior. Host PDF-only page buttons are omitted for Office.
- All readers have a real AppKit toolbar with Open in Default App, Show in Finder and full screen, standard native window controls, French/English tooltips and app appearance. Open/Finder revalidate the file; unavailable files show a native alert. At most three production previews are open; reopening the same download focuses its existing window. Removing/replacing an account session closes its previews.

## Verification evidence

- 71 frontend tests, including separate Preview/Open actions, course labels/Unknown Course and disabled incomplete-transfer actions; build, lint and format checks pass.
- 49 Rust tests cover old-schema migration/reopen, download metadata/filtering, context isolation/replacement/ambiguity, supported document types, pluginfile URL boundaries and the metadata-only Moodle contract. HTTP tests use local mock servers, not live Moodle responses.
- Workspace Clippy with warnings denied, formatting and shell/popup capability checks pass. Native document windows have no webviews or IPC grants.
- A local debug app bundle builds. This is not a notarized or release-qualified package.
- The standalone native harness (`cargo run -p moodburo --example document-preview -- <absolute fixture paths>`) exercises the same native view/toolbar without reading MoodBuro's database or Keychain. It invokes the actual Objective-C toolbar actions and asserts zoom/fit/readout behavior for all four formats and next/previous page behavior for a two-page PDF.
- Actual native controls passed for PDF and all three Office formats. The PowerPoint toolbar was inspected visually; Zoom In and Fit were exercised through the UI. A minimal hand-written PPTX initially showed only an icon; a fully formed two-slide presentation rendered correctly, also confirmed by the product owner. Invalid/unsupported Office documents may still show the system fallback rather than content.

## Remaining live and platform qualification

- [ ] Real Moodle and 4.5/5.3 rigs: refresh/index, denied service, real authenticated downloads, non-slash pluginfile URLs, subdirectory installations and correct course-folder publication.
- [ ] Installed app: Preview from Downloads, missing/moved/symlinked files, repeat focus, fourth-window cap, account replacement/removal while previews are open, and default-app/Finder failures.
- [ ] DOCX with images/tables/headers, multi-sheet XLSX, large presentations, fonts and complex Office layouts; Quick Look is a read-only preview, not an Office editor or a layout-fidelity guarantee.
- [ ] Supported macOS matrix: resize, fullscreen, small-window toolbar overflow, VoiceOver/keyboard navigation, light/dark, Increase Contrast and live appearance changes. Toolbar language is selected when the document opens; reopen it after changing the app's language.
- [ ] Pinch zoom/readout synchronization, extended multi-page scrolling and large-file resource usage on installed builds.

The existing PDFKit Save delegate/private-API compatibility gate, native WKWebView download lifecycle, safe retry and PDF/image preferences remain separate open tasks. Previewing a completed download does not close those gates.

## API references

- [Apple QLPreviewView](https://developer.apple.com/documentation/quicklookui/qlpreviewview)
- [Apple PDFView](https://developer.apple.com/documentation/pdfkit/pdfview)
- [Moodle 4.5 course external API source](https://github.com/moodle/moodle/blob/MOODLE_405_STABLE/course/externallib.php)
