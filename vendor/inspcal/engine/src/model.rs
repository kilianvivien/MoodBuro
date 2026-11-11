//! What extraction produces: the same shapes as the dataclasses of
//! `src/edt_parser/pdf_extract.py`, serialised to the JSON that
//! `tools/dump_document.py` writes for the Python side.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// `(x0, top, x1, bottom)` in PDF points, `top` measured from the top of the page.
pub type BBox = [f64; 4];

/// Red, green, blue, each in `0.0..=1.0`.
pub type Rgb = [f64; 3];

/// One line of text inside a cell, with the attributes later stages read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextLine {
    pub text: String,
    pub bbox: BBox,
    pub bold: bool,
    pub size: f64,
    pub color: Option<Rgb>,
}

/// A PDF URI annotation. Degenerate rectangles are dropped before this exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub url: String,
    pub bbox: BBox,
    pub page: u32,
}

/// A table cell as the table finder reports it, plus its content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub bbox: BBox,
    pub page: u32,
    pub text: String,
    pub lines: Vec<TextLine>,
    pub fill: Option<Rgb>,
    pub underlined: bool,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub number: u32,
    pub width: f64,
    pub height: f64,
    pub cells: Vec<Cell>,
    pub links: Vec<Link>,
    pub table_bbox: Option<BBox>,
    pub table_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub file: String,
    pub sha256: String,
    pub pages: Vec<Page>,
    /// The Info entries later stages read: `CreationDate` and `ModDate`.
    pub metadata: BTreeMap<String, String>,
}

/// A blocking error, with the code `src/edt_parser/errors.py` gives the same
/// failure — or `UNSUPPORTED_PDF`, which Python never raises: this reader
/// refuses a document it was not written for instead of guessing at it.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

impl ExtractError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), page: None }
    }

    pub fn on_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    pub fn unsupported(what: &str) -> Self {
        Self::new("UNSUPPORTED_PDF", format!("PDF hors du format pris en charge : {what}"))
    }
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ExtractError {}
