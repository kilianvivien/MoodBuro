//! InspCal's engine: a timetable PDF in, calendar events out.
//!
//! `docs/web-engine-plan.md` describes the port this crate is. It follows the
//! Python parser module for module, and is held to it by the test suite.
//!
//! Reading the PDF (stage 1). PDFium reads the file; everything after that is
//! plain Rust on plain data:
//!
//! * `pdfium`  — the only code that calls PDFium;
//! * `objects` — painted paths sorted into rectangles, lines and curves;
//! * `tables`  — pdfplumber's table finder;
//! * `text`    — pdfplumber's `extract_text`;
//! * `extract` — `pdf_extract.py` itself.
//!
//! Understanding it (stage 2), one module per Python module:
//!
//! * `grid`, `events`, `dates`, `rooms`, `validate`, `pipeline` — the parser;
//! * `timetable` — the pivot model and its JSON;
//! * `selection` — the reviewer's choices;
//! * `ics`       — the calendar file, written as `icalendar` writes it;
//! * `review`, `convert` — a request from the page, answered as the API does.
//!
//! `py` holds what had to be reproduced of Python itself.

#[cfg(target_arch = "wasm32")]
mod browser;
mod cluster;
pub mod convert;
pub mod dates;
pub mod errors;
pub mod events;
pub mod extract;
pub mod grid;
pub mod ics;
pub mod institution;
#[cfg(not(target_arch = "wasm32"))]
pub mod maintain;
pub mod model;
pub mod objects;
pub mod pdfium;
mod pdfium_bindings;
pub mod pipeline;
pub mod py;
pub mod raw;
pub mod review;
pub mod rooms;
pub mod selection;
pub mod tables;
pub mod text;
pub mod timetable;
pub mod validate;

use pdfium_render::prelude::PdfiumLibraryBindings;
use sha2::{Digest, Sha256};

pub use model::{Document, ExtractError};

/// Read one PDF. PDFium must have been initialised on `bindings`.
pub fn extract(bindings: &dyn PdfiumLibraryBindings, bytes: &[u8], file: &str) -> Result<Document, ExtractError> {
    extract_with_reader(&pdfium_bindings::NativeBindings(bindings), bytes, file)
}

fn extract_with_reader(
    bindings: &dyn pdfium_bindings::ReadBindings,
    bytes: &[u8],
    file: &str,
) -> Result<Document, ExtractError> {
    let raw = pdfium::read(bindings, bytes)?;
    let sha256 = Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect();
    extract::build(&raw, file, sha256)
}
