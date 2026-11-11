//! What is read from the PDF before any interpretation: characters, painted
//! paths and link rectangles, in page coordinates. `pdfium.rs` fills these;
//! nothing after it knows which library the PDF was read with.

use crate::model::Rgb;

/// A character, in the conventions pdfplumber reports one: `top` and `bottom`
/// measured from the top of the page, and a box one font size high that
/// starts at the font's descent — not the outline of the glyph.
#[derive(Debug, Clone)]
pub struct Char {
    pub text: char,
    pub x0: f64,
    pub x1: f64,
    pub top: f64,
    pub bottom: f64,
    pub size: f64,
    pub fontname: String,
    pub color: Option<Rgb>,
}

impl Char {
    pub fn is_space(&self) -> bool {
        py_isspace(self.text)
    }
}

/// `str.isspace()` for one character: Unicode white space plus the four ASCII
/// separators Python counts and Rust does not.
pub fn py_isspace(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Move,
    Line,
    Curve,
    Close,
}

/// A painted path: its construction operators with their end points, already
/// in page space (PDF orientation, `y` upwards).
#[derive(Debug, Clone)]
pub struct Path {
    pub ops: Vec<(Op, (f64, f64))>,
    pub fill_color: Option<Rgb>,
}

/// A URI annotation's rectangle, in page space (`y` upwards).
#[derive(Debug, Clone)]
pub struct RawLink {
    pub url: String,
    pub rect: [f64; 4],
}

#[derive(Debug, Clone, Default)]
pub struct RawPage {
    pub number: u32,
    pub width: f64,
    pub height: f64,
    pub chars: Vec<Char>,
    pub paths: Vec<Path>,
    pub links: Vec<RawLink>,
    /// Images and shadings: counted against the page's object budget only.
    pub other_objects: usize,
}

#[derive(Debug, Clone, Default)]
pub struct RawDocument {
    pub pages: Vec<RawPage>,
    pub creation_date: Option<String>,
    pub mod_date: Option<String>,
}
