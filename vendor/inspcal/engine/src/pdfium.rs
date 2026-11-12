//! The only module that talks to PDFium.
//!
//! It reads three things from a page and hands them on as plain data
//! (`raw.rs`): characters, painted paths and URI links.
//!
//! PDFium's text page is not a transcript of the content stream. It drops
//! most space glyphs and inserts spaces of its own where words are apart, and
//! it reports a hyphen that ends a line as U+0002. Both are undone here so
//! that later stages see what pdfplumber would have shown them:
//!
//! * a generated space becomes a zero-width space character placed at the end
//!   of the glyph before it;
//! * U+0002 becomes `-` again.
//!
//! A character's box is not PDFium's either. pdfminer gives every glyph a box
//! one font size high starting at the font's descent, and the parser's line
//! clustering was tuned on those. PDFium supplies the ingredients — baseline
//! origin, text matrix, font size, descent — and the box is rebuilt from them.

use std::collections::HashMap;
use std::os::raw::{c_double, c_float, c_int, c_uint, c_ulong, c_void};

use pdfium_render::prelude::*;

use crate::model::{ExtractError, Rgb};
use crate::pdfium_bindings::ReadBindings;
use crate::raw::{Char, Op, Path, RawDocument, RawLink, RawPage};

pub const MAX_PAGES: usize = 20;

const PAGEOBJ_PATH: c_int = 2;
const PAGEOBJ_FORM: c_int = 5;
const SEGMENT_LINETO: c_int = 0;
const SEGMENT_BEZIERTO: c_int = 1;
const SEGMENT_MOVETO: c_int = 2;
const FILLMODE_NONE: c_int = 0;
const ACTION_URI: c_ulong = 3;
/// How deep form XObjects may nest before the page is refused.
const MAX_FORM_DEPTH: usize = 8;

/// PDFium stores coordinates as 32-bit floats. The shortest decimal that
/// reads back as the same float is, for a number written in the PDF with
/// seven significant digits or fewer, the number that was written — which is
/// the value pdfminer parsed. Widening the float directly would be off by up
/// to 2e-5 pt instead.
fn widen(value: f32) -> f64 {
    value.to_string().parse().unwrap_or(value as f64)
}

#[derive(Clone, Copy)]
struct Matrix {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Matrix {
    const IDENTITY: Matrix = Matrix { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    fn from(m: &FS_MATRIX) -> Self {
        Matrix { a: widen(m.a), b: widen(m.b), c: widen(m.c), d: widen(m.d), e: widen(m.e), f: widen(m.f) }
    }

    /// pdfminer's `apply_matrix_pt`, operation for operation.
    fn apply(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    /// The matrix that applies `self` first, then `outer`.
    fn then(&self, outer: &Matrix) -> Matrix {
        Matrix {
            a: self.a * outer.a + self.b * outer.c,
            b: self.a * outer.b + self.b * outer.d,
            c: self.c * outer.a + self.d * outer.c,
            d: self.c * outer.b + self.d * outer.d,
            e: self.e * outer.a + self.f * outer.c + outer.e,
            f: self.e * outer.b + self.f * outer.d + outer.f,
        }
    }

    fn is_identity(&self) -> bool {
        self.a == 1.0 && self.b == 0.0 && self.c == 0.0 && self.d == 1.0 && self.e == 0.0 && self.f == 0.0
    }
}

const EMPTY_MATRIX: FS_MATRIX = FS_MATRIX { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };
const EMPTY_RECT: FS_RECTF = FS_RECTF { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 };

fn rgb(r: c_uint, g: c_uint, b: c_uint) -> Rgb {
    [r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0]
}

/// Read a whole document. PDFium must already be initialised.
pub(crate) fn read(bindings: &dyn ReadBindings, bytes: &[u8]) -> Result<RawDocument, ExtractError> {
    // SAFETY: every handle below comes from PDFium, is checked for null before
    // use and is closed before this function returns; `bytes` outlives them.
    unsafe {
        let document = bindings.FPDF_LoadMemDocument64(bytes, None);
        if document.is_null() {
            return Err(ExtractError::new("PDF_UNREADABLE", "fichier illisible comme PDF"));
        }
        let result = read_document(bindings, document);
        bindings.FPDF_CloseDocument(document);
        result
    }
}

unsafe fn read_document(bindings: &dyn ReadBindings, document: FPDF_DOCUMENT) -> Result<RawDocument, ExtractError> {
    let count = bindings.FPDF_GetPageCount(document);
    if count < 0 {
        return Err(ExtractError::new("PDF_UNREADABLE", "PDF endommagé ou incomplet"));
    }
    if count as usize > MAX_PAGES {
        return Err(ExtractError::new("PDF_TOO_COMPLEX", format!("Le PDF dépasse la limite de {MAX_PAGES} pages.")));
    }
    let mut pages = Vec::with_capacity(count as usize);
    for index in 0..count {
        let number = index as u32 + 1;
        let page = bindings.FPDF_LoadPage(document, index);
        if page.is_null() {
            return Err(ExtractError::new("PDF_UNREADABLE", "PDF endommagé ou incomplet").on_page(number));
        }
        let result = read_page(bindings, document, page, number);
        bindings.FPDF_ClosePage(page);
        pages.push(result.map_err(|error| error.on_page(number))?);
    }
    Ok(RawDocument {
        pages,
        creation_date: meta_text(bindings, document, "CreationDate"),
        mod_date: meta_text(bindings, document, "ModDate"),
    })
}

unsafe fn meta_text(bindings: &dyn ReadBindings, document: FPDF_DOCUMENT, tag: &str) -> Option<String> {
    let needed = bindings.FPDF_GetMetaText(document, tag, std::ptr::null_mut(), 0);
    if needed <= 2 {
        return None; // absent, or the terminator alone
    }
    let mut buffer = vec![0u8; needed as usize];
    let written = bindings.FPDF_GetMetaText(document, tag, buffer.as_mut_ptr() as *mut c_void, needed);
    let units: Vec<u16> = buffer[..(written.min(needed) as usize)]
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|&unit| unit != 0)
        .collect();
    let text = String::from_utf16_lossy(&units);
    (!text.is_empty()).then_some(text)
}

unsafe fn read_page(
    bindings: &dyn ReadBindings,
    document: FPDF_DOCUMENT,
    page: FPDF_PAGE,
    number: u32,
) -> Result<RawPage, ExtractError> {
    if bindings.FPDFPage_GetRotation(page) != 0 {
        return Err(ExtractError::unsupported("page pivotée"));
    }
    let (mut left, mut bottom, mut right, mut top) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    if bindings.FPDFPage_GetMediaBox(page, &mut left, &mut bottom, &mut right, &mut top) == 0 {
        return Err(ExtractError::unsupported("page sans MediaBox"));
    }
    if left != 0.0 || bottom != 0.0 {
        return Err(ExtractError::unsupported("MediaBox décalée"));
    }
    let (mut c_left, mut c_bottom, mut c_right, mut c_top) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    if bindings.FPDFPage_GetCropBox(page, &mut c_left, &mut c_bottom, &mut c_right, &mut c_top) != 0
        && (c_left, c_bottom, c_right, c_top) != (left, bottom, right, top)
    {
        return Err(ExtractError::unsupported("CropBox différente de la MediaBox"));
    }
    let mut out = RawPage { number, width: widen(right), height: widen(top), ..RawPage::default() };

    read_objects(bindings, ObjectList::Page(page), &Matrix::IDENTITY, 0, &mut out)?;
    read_links(bindings, document, page, &mut out);

    let text_page = bindings.FPDFText_LoadPage(page);
    if text_page.is_null() {
        return Err(ExtractError::new("PDF_UNREADABLE", "PDF endommagé ou incomplet"));
    }
    let result = read_chars(bindings, text_page, out.height);
    bindings.FPDFText_ClosePage(text_page);
    out.chars = result?;
    Ok(out)
}

struct FontFacts {
    /// Descent in thousandths of the font size, negative below the baseline.
    descent: f64,
    /// Ascent, in the same unit.
    ascent: f64,
    name: String,
}

unsafe fn font_facts(bindings: &dyn ReadBindings, font: FPDF_FONT) -> Option<FontFacts> {
    let (mut descent, mut ascent): (c_float, c_float) = (0.0, 0.0);
    if bindings.FPDFFont_GetDescent(font, 1000.0, &mut descent) == 0
        || bindings.FPDFFont_GetAscent(font, 1000.0, &mut ascent) == 0
    {
        return None;
    }
    let mut buffer = [0u8; 256];
    let length = bindings.FPDFFont_GetBaseFontName(font, buffer.as_mut_ptr() as *mut _, buffer.len());
    let end = buffer.iter().position(|&byte| byte == 0).unwrap_or(length.min(buffer.len()));
    Some(FontFacts {
        descent: widen(descent),
        ascent: widen(ascent),
        name: String::from_utf8_lossy(&buffer[..end]).into_owned(),
    })
}

unsafe fn read_chars(
    bindings: &dyn ReadBindings,
    text_page: FPDF_TEXTPAGE,
    page_height: f64,
) -> Result<Vec<Char>, ExtractError> {
    let count = bindings.FPDFText_CountChars(text_page).max(0);
    let mut chars: Vec<Char> = Vec::with_capacity(count as usize);
    let mut fonts: HashMap<usize, Option<FontFacts>> = HashMap::new();
    // Generated spaces waiting for the glyph after them.
    let mut generated: Vec<usize> = Vec::new();
    for index in 0..count {
        let unicode = bindings.FPDFText_GetUnicode(text_page, index);
        if bindings.FPDFText_IsGenerated(text_page, index) == 1 {
            // Line breaks carry no position; a space stands where a word ends.
            if unicode == 0x20 {
                if let Some(previous) = chars.last().cloned() {
                    generated.push(chars.len());
                    chars.push(Char { text: ' ', x0: previous.x1, ..previous });
                }
            }
            continue;
        }
        if unicode == 0 || bindings.FPDFText_HasUnicodeMapError(text_page, index) == 1 {
            return Err(ExtractError::unsupported("caractère sans correspondance Unicode"));
        }
        // The marker PDFium leaves in place of a hyphen that ends a line.
        let text = if unicode == 0x02 { '-' } else { char::from_u32(unicode).unwrap_or('\u{fffd}') };

        let mut matrix = EMPTY_MATRIX;
        let (mut origin_x, mut origin_y): (c_double, c_double) = (0.0, 0.0);
        let mut loose = EMPTY_RECT;
        if bindings.FPDFText_GetMatrix(text_page, index, &mut matrix) == 0
            || bindings.FPDFText_GetCharOrigin(text_page, index, &mut origin_x, &mut origin_y) == 0
            || bindings.FPDFText_GetLooseCharBox(text_page, index, &mut loose) == 0
        {
            return Err(ExtractError::unsupported("caractère sans position"));
        }
        // Slanted text is fine — Word fakes italics by shearing — but a rotated
        // or mirrored glyph has no line to sit on.
        if matrix.b != 0.0 || matrix.a <= 0.0 || matrix.d <= 0.0 {
            return Err(ExtractError::unsupported("texte pivoté ou en miroir"));
        }
        let object = bindings.FPDFText_GetTextObject(text_page, index);
        let font = if object.is_null() { std::ptr::null_mut() } else { bindings.FPDFTextObj_GetFont(object) };
        if font.is_null() {
            return Err(ExtractError::unsupported("caractère sans police"));
        }
        let facts = fonts.entry(font as usize).or_insert_with(|| font_facts(bindings, font));
        let Some(facts) = facts.as_ref() else {
            return Err(ExtractError::unsupported("police sans métriques"));
        };

        // pdfminer's box, in text space, runs from (0, descent) to (advance,
        // descent + font size); its two corners are then carried to the page.
        // PDFium's loose box is the bounds of its own slanted box, which runs
        // from the descent to the ascent: its right edge is the advance plus
        // however far the slant carries that box's rightmost corner.
        let font_size = bindings.FPDFText_GetFontSize(text_page, index);
        let (shear, scale_y) = (widen(matrix.c), widen(matrix.d));
        let low = facts.descent / 1000.0 * font_size;
        let high = low + font_size;
        let slant = (shear * low).max(shear * facts.ascent / 1000.0 * font_size);
        let advance = (widen(loose.right) - origin_x - slant).max(0.0);
        let (xa, xb) = (origin_x + shear * low, origin_x + advance + shear * high);
        let (ya, yb) = (origin_y + scale_y * low, origin_y + scale_y * high);
        let (x0, x1) = (xa.min(xb), xa.max(xb));
        let (y0, y1) = (ya.min(yb), ya.max(yb));

        let (mut r, mut g, mut b, mut a): (c_uint, c_uint, c_uint, c_uint) = (0, 0, 0, 0);
        let color = (bindings.FPDFText_GetFillColor(text_page, index, &mut r, &mut g, &mut b, &mut a) != 0)
            .then(|| rgb(r, g, b));

        let top = page_height - y1;
        // Slanted glyphs overlap: the previous one's right edge can lie past
        // this one's left edge (16 November 2026, italic links). A space
        // generated between them must not sort after this glyph, or the word
        // break moves one letter on (« ordinateure t »).
        for space in generated.drain(..) {
            let glyph_start = chars[space - 1].x0;
            let gap = &mut chars[space];
            // Only when this glyph starts inside the previous one's box.
            if (gap.top - top).abs() <= gap.size / 2.0 && glyph_start < x0 && x0 < gap.x0 {
                (gap.x0, gap.x1) = (x0, x0);
            }
        }
        chars.push(Char {
            text,
            x0,
            x1,
            top,
            bottom: page_height - y0,
            size: y1 - y0,
            fontname: facts.name.clone(),
            color,
        });
    }
    Ok(chars)
}

#[derive(Clone, Copy)]
enum ObjectList {
    Page(FPDF_PAGE),
    Form(FPDF_PAGEOBJECT),
}

unsafe fn read_objects(
    bindings: &dyn ReadBindings,
    list: ObjectList,
    outer: &Matrix,
    depth: usize,
    out: &mut RawPage,
) -> Result<(), ExtractError> {
    if depth > MAX_FORM_DEPTH {
        return Err(ExtractError::unsupported("formulaires imbriqués trop profondément"));
    }
    let count = match list {
        ObjectList::Page(page) => bindings.FPDFPage_CountObjects(page),
        ObjectList::Form(form) => bindings.FPDFFormObj_CountObjects(form),
    };
    for index in 0..count.max(0) {
        let object = match list {
            ObjectList::Page(page) => bindings.FPDFPage_GetObject(page, index),
            ObjectList::Form(form) => bindings.FPDFFormObj_GetObject(form, index as c_ulong),
        };
        if object.is_null() {
            continue;
        }
        let mut own = EMPTY_MATRIX;
        let has_matrix = bindings.FPDFPageObj_GetMatrix(object, &mut own) != 0;
        match bindings.FPDFPageObj_GetType(object) {
            PAGEOBJ_PATH => {
                if !has_matrix {
                    return Err(ExtractError::unsupported("tracé sans matrice"));
                }
                let matrix = Matrix::from(&own).then(outer);
                if let Some(path) = read_path(bindings, object, &matrix) {
                    out.paths.push(path);
                }
            }
            PAGEOBJ_FORM => {
                if !has_matrix {
                    return Err(ExtractError::unsupported("formulaire sans matrice"));
                }
                let matrix = Matrix::from(&own).then(outer);
                read_objects(bindings, ObjectList::Form(object), &matrix, depth + 1, out)?;
            }
            1 => {} // text: read through the text page, which keeps every glyph's position
            _ => out.other_objects += 1,
        }
    }
    Ok(())
}

unsafe fn read_path(bindings: &dyn ReadBindings, object: FPDF_PAGEOBJECT, matrix: &Matrix) -> Option<Path> {
    let mut fill_mode: c_int = FILLMODE_NONE;
    let mut stroke: FPDF_BOOL = 0;
    if bindings.FPDFPath_GetDrawMode(object, &mut fill_mode, &mut stroke) == 0 {
        return None;
    }
    // A path that is neither filled nor stroked paints nothing; pdfminer never
    // reports one.
    if fill_mode == FILLMODE_NONE && stroke == 0 {
        return None;
    }
    let count = bindings.FPDFPath_CountSegments(object);
    let mut ops = Vec::with_capacity(count.max(0) as usize + 1);
    let mut pending_curve = 0;
    for index in 0..count.max(0) {
        let segment = bindings.FPDFPath_GetPathSegment(object, index);
        if segment.is_null() {
            return None;
        }
        let (mut x, mut y): (c_float, c_float) = (0.0, 0.0);
        if bindings.FPDFPathSegment_GetPoint(segment, &mut x, &mut y) == 0 {
            return None;
        }
        let raw = (widen(x), widen(y));
        let point = if matrix.is_identity() { raw } else { matrix.apply(raw) };
        match bindings.FPDFPathSegment_GetType(segment) {
            SEGMENT_MOVETO => ops.push((Op::Move, point)),
            SEGMENT_LINETO => ops.push((Op::Line, point)),
            SEGMENT_BEZIERTO => {
                // Three points make one curve; pdfminer keeps only where it ends.
                pending_curve += 1;
                if pending_curve == 3 {
                    pending_curve = 0;
                    ops.push((Op::Curve, point));
                }
            }
            _ => return None,
        }
        if bindings.FPDFPathSegment_GetClose(segment) != 0 {
            ops.push((Op::Close, point));
        }
    }
    let (mut r, mut g, mut b, mut a): (c_uint, c_uint, c_uint, c_uint) = (0, 0, 0, 0);
    let fill_color =
        (bindings.FPDFPageObj_GetFillColor(object, &mut r, &mut g, &mut b, &mut a) != 0).then(|| rgb(r, g, b));
    Some(Path { ops, fill_color })
}

unsafe fn read_links(bindings: &dyn ReadBindings, document: FPDF_DOCUMENT, page: FPDF_PAGE, out: &mut RawPage) {
    let mut position: c_int = 0;
    let mut link: FPDF_LINK = std::ptr::null_mut();
    while bindings.FPDFLink_Enumerate(page, &mut position, &mut link) != 0 {
        if link.is_null() {
            continue;
        }
        let action = bindings.FPDFLink_GetAction(link);
        if action.is_null() || bindings.FPDFAction_GetType(action) != ACTION_URI {
            continue;
        }
        let needed = bindings.FPDFAction_GetURIPath(document, action, std::ptr::null_mut(), 0);
        if needed <= 1 {
            continue;
        }
        let mut buffer = vec![0u8; needed as usize];
        bindings.FPDFAction_GetURIPath(document, action, buffer.as_mut_ptr() as *mut c_void, needed);
        let end = buffer.iter().position(|&byte| byte == 0).unwrap_or(buffer.len());
        let url = String::from_utf8_lossy(&buffer[..end]).into_owned();
        let mut rect = EMPTY_RECT;
        if url.is_empty() || bindings.FPDFLink_GetAnnotRect(link, &mut rect) == 0 {
            continue;
        }
        out.links
            .push(RawLink { url, rect: [widen(rect.left), widen(rect.bottom), widen(rect.right), widen(rect.top)] });
    }
}
