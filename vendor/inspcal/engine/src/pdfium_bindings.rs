//! Read-only PDFium calls needed by extraction. Native calls still delegate to
//! pdfium-render; the browser replaces numeric calls with a cached, synchronous
//! bridge that reuses scratch memory. The extraction algorithms are shared.
#![allow(non_snake_case, clippy::too_many_arguments)]
use pdfium_render::prelude::*;
use std::os::raw::{c_char, c_double, c_float, c_int, c_uint, c_ulong, c_void};

pub(crate) trait ReadBindings {
    fn library(&self) -> &dyn PdfiumLibraryBindings;
    unsafe fn FPDFAction_GetType(&self, action: FPDF_ACTION) -> c_ulong {
        self.library().FPDFAction_GetType(action)
    }
    unsafe fn FPDFAction_GetURIPath(
        &self,
        document: FPDF_DOCUMENT,
        action: FPDF_ACTION,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        self.library().FPDFAction_GetURIPath(document, action, buffer, buflen)
    }
    unsafe fn FPDFFont_GetAscent(&self, font: FPDF_FONT, font_size: c_float, ascent: *mut c_float) -> FPDF_BOOL {
        self.library().FPDFFont_GetAscent(font, font_size, ascent)
    }
    unsafe fn FPDFFont_GetBaseFontName(&self, font: FPDF_FONT, buffer: *mut c_char, length: usize) -> usize {
        self.library().FPDFFont_GetBaseFontName(font, buffer, length)
    }
    unsafe fn FPDFFont_GetDescent(&self, font: FPDF_FONT, font_size: c_float, descent: *mut c_float) -> FPDF_BOOL {
        self.library().FPDFFont_GetDescent(font, font_size, descent)
    }
    unsafe fn FPDFFormObj_CountObjects(&self, form_object: FPDF_PAGEOBJECT) -> c_int {
        self.library().FPDFFormObj_CountObjects(form_object)
    }
    unsafe fn FPDFFormObj_GetObject(&self, form_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECT {
        self.library().FPDFFormObj_GetObject(form_object, index)
    }
    unsafe fn FPDFLink_Enumerate(
        &self,
        page: FPDF_PAGE,
        start_pos: *mut c_int,
        link_annot: *mut FPDF_LINK,
    ) -> FPDF_BOOL {
        self.library().FPDFLink_Enumerate(page, start_pos, link_annot)
    }
    unsafe fn FPDFLink_GetAction(&self, link: FPDF_LINK) -> FPDF_ACTION {
        self.library().FPDFLink_GetAction(link)
    }
    unsafe fn FPDFLink_GetAnnotRect(&self, link_annot: FPDF_LINK, rect: *mut FS_RECTF) -> FPDF_BOOL {
        self.library().FPDFLink_GetAnnotRect(link_annot, rect)
    }
    unsafe fn FPDFPageObj_GetFillColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        self.library().FPDFPageObj_GetFillColor(page_object, R, G, B, A)
    }
    unsafe fn FPDFPageObj_GetMatrix(&self, page_object: FPDF_PAGEOBJECT, matrix: *mut FS_MATRIX) -> FPDF_BOOL {
        self.library().FPDFPageObj_GetMatrix(page_object, matrix)
    }
    unsafe fn FPDFPageObj_GetType(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        self.library().FPDFPageObj_GetType(page_object)
    }
    unsafe fn FPDFPage_CountObjects(&self, page: FPDF_PAGE) -> c_int {
        self.library().FPDFPage_CountObjects(page)
    }
    unsafe fn FPDFPage_GetCropBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        self.library().FPDFPage_GetCropBox(page, left, bottom, right, top)
    }
    unsafe fn FPDFPage_GetMediaBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        self.library().FPDFPage_GetMediaBox(page, left, bottom, right, top)
    }
    unsafe fn FPDFPage_GetObject(&self, page: FPDF_PAGE, index: c_int) -> FPDF_PAGEOBJECT {
        self.library().FPDFPage_GetObject(page, index)
    }
    unsafe fn FPDFPage_GetRotation(&self, page: FPDF_PAGE) -> c_int {
        self.library().FPDFPage_GetRotation(page)
    }
    unsafe fn FPDFPathSegment_GetClose(&self, segment: FPDF_PATHSEGMENT) -> FPDF_BOOL {
        self.library().FPDFPathSegment_GetClose(segment)
    }
    unsafe fn FPDFPathSegment_GetPoint(
        &self,
        segment: FPDF_PATHSEGMENT,
        x: *mut c_float,
        y: *mut c_float,
    ) -> FPDF_BOOL {
        self.library().FPDFPathSegment_GetPoint(segment, x, y)
    }
    unsafe fn FPDFPathSegment_GetType(&self, segment: FPDF_PATHSEGMENT) -> c_int {
        self.library().FPDFPathSegment_GetType(segment)
    }
    unsafe fn FPDFPath_CountSegments(&self, path: FPDF_PAGEOBJECT) -> c_int {
        self.library().FPDFPath_CountSegments(path)
    }
    unsafe fn FPDFPath_GetDrawMode(
        &self,
        path: FPDF_PAGEOBJECT,
        fillmode: *mut c_int,
        stroke: *mut FPDF_BOOL,
    ) -> FPDF_BOOL {
        self.library().FPDFPath_GetDrawMode(path, fillmode, stroke)
    }
    unsafe fn FPDFPath_GetPathSegment(&self, path: FPDF_PAGEOBJECT, index: c_int) -> FPDF_PATHSEGMENT {
        self.library().FPDFPath_GetPathSegment(path, index)
    }
    unsafe fn FPDFTextObj_GetFont(&self, text: FPDF_PAGEOBJECT) -> FPDF_FONT {
        self.library().FPDFTextObj_GetFont(text)
    }
    unsafe fn FPDFText_ClosePage(&self, text_page: FPDF_TEXTPAGE) {
        self.library().FPDFText_ClosePage(text_page)
    }
    unsafe fn FPDFText_CountChars(&self, text_page: FPDF_TEXTPAGE) -> c_int {
        self.library().FPDFText_CountChars(text_page)
    }
    unsafe fn FPDFText_GetCharOrigin(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        x: *mut c_double,
        y: *mut c_double,
    ) -> FPDF_BOOL {
        self.library().FPDFText_GetCharOrigin(text_page, index, x, y)
    }
    unsafe fn FPDFText_GetFillColor(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        self.library().FPDFText_GetFillColor(text_page, index, R, G, B, A)
    }
    unsafe fn FPDFText_GetFontSize(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_double {
        self.library().FPDFText_GetFontSize(text_page, index)
    }
    unsafe fn FPDFText_GetLooseCharBox(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        rect: *mut FS_RECTF,
    ) -> FPDF_BOOL {
        self.library().FPDFText_GetLooseCharBox(text_page, index, rect)
    }
    unsafe fn FPDFText_GetMatrix(&self, text_page: FPDF_TEXTPAGE, index: c_int, matrix: *mut FS_MATRIX) -> FPDF_BOOL {
        self.library().FPDFText_GetMatrix(text_page, index, matrix)
    }
    unsafe fn FPDFText_GetTextObject(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> FPDF_PAGEOBJECT {
        self.library().FPDFText_GetTextObject(text_page, index)
    }
    unsafe fn FPDFText_GetUnicode(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_uint {
        self.library().FPDFText_GetUnicode(text_page, index)
    }
    unsafe fn FPDFText_HasUnicodeMapError(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        self.library().FPDFText_HasUnicodeMapError(text_page, index)
    }
    unsafe fn FPDFText_IsGenerated(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        self.library().FPDFText_IsGenerated(text_page, index)
    }
    unsafe fn FPDFText_LoadPage(&self, page: FPDF_PAGE) -> FPDF_TEXTPAGE {
        self.library().FPDFText_LoadPage(page)
    }
    unsafe fn FPDF_CloseDocument(&self, document: FPDF_DOCUMENT) {
        self.library().FPDF_CloseDocument(document)
    }
    unsafe fn FPDF_ClosePage(&self, page: FPDF_PAGE) {
        self.library().FPDF_ClosePage(page)
    }
    unsafe fn FPDF_GetMetaText(
        &self,
        document: FPDF_DOCUMENT,
        tag: &str,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        self.library().FPDF_GetMetaText(document, tag, buffer, buflen)
    }
    unsafe fn FPDF_GetPageCount(&self, document: FPDF_DOCUMENT) -> c_int {
        self.library().FPDF_GetPageCount(document)
    }
    unsafe fn FPDF_LoadMemDocument64(&self, data_buf: &[u8], password: Option<&str>) -> FPDF_DOCUMENT {
        self.library().FPDF_LoadMemDocument64(data_buf, password)
    }
    unsafe fn FPDF_LoadPage(&self, document: FPDF_DOCUMENT, page_index: c_int) -> FPDF_PAGE {
        self.library().FPDF_LoadPage(document, page_index)
    }
}

pub(crate) struct NativeBindings<'a>(pub &'a dyn PdfiumLibraryBindings);
impl ReadBindings for NativeBindings<'_> {
    fn library(&self) -> &dyn PdfiumLibraryBindings {
        self.0
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/src/pdfium_bridge.js")]
extern "C" {
    #[wasm_bindgen(js_name = initializeFastPdfium)]
    fn initialize_bridge(pdfium: JsValue, memory: JsValue);
    #[wasm_bindgen(js_name = callPdfium)]
    fn call_pdfium(op: u32, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> f64;
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn initialize_fast_pdfium(pdfium: JsValue) {
    initialize_bridge(pdfium, wasm_bindgen::memory());
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct FastBindings(pub Box<dyn PdfiumLibraryBindings>);
#[cfg(target_arch = "wasm32")]
impl ReadBindings for FastBindings {
    fn library(&self) -> &dyn PdfiumLibraryBindings {
        self.0.as_ref()
    }
    unsafe fn FPDFAction_GetType(&self, action: FPDF_ACTION) -> c_ulong {
        call_pdfium(0, action as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as c_ulong
    }
    unsafe fn FPDFFont_GetAscent(&self, font: FPDF_FONT, font_size: c_float, ascent: *mut c_float) -> FPDF_BOOL {
        call_pdfium(1, font as usize as f64, font_size as f64, ascent as usize as f64, 0.0, 0.0, 0.0) as FPDF_BOOL
    }
    unsafe fn FPDFFont_GetDescent(&self, font: FPDF_FONT, font_size: c_float, descent: *mut c_float) -> FPDF_BOOL {
        call_pdfium(2, font as usize as f64, font_size as f64, descent as usize as f64, 0.0, 0.0, 0.0) as FPDF_BOOL
    }
    unsafe fn FPDFFormObj_CountObjects(&self, form_object: FPDF_PAGEOBJECT) -> c_int {
        call_pdfium(3, form_object as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as c_int
    }
    unsafe fn FPDFFormObj_GetObject(&self, form_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECT {
        call_pdfium(4, form_object as usize as f64, index as f64, 0.0, 0.0, 0.0, 0.0) as usize as FPDF_PAGEOBJECT
    }
    unsafe fn FPDFLink_Enumerate(
        &self,
        page: FPDF_PAGE,
        start_pos: *mut c_int,
        link_annot: *mut FPDF_LINK,
    ) -> FPDF_BOOL {
        call_pdfium(5, page as usize as f64, start_pos as usize as f64, link_annot as usize as f64, 0.0, 0.0, 0.0)
            as FPDF_BOOL
    }
    unsafe fn FPDFLink_GetAction(&self, link: FPDF_LINK) -> FPDF_ACTION {
        call_pdfium(6, link as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as usize as FPDF_ACTION
    }
    unsafe fn FPDFLink_GetAnnotRect(&self, link_annot: FPDF_LINK, rect: *mut FS_RECTF) -> FPDF_BOOL {
        call_pdfium(7, link_annot as usize as f64, rect as usize as f64, 0.0, 0.0, 0.0, 0.0) as FPDF_BOOL
    }
    unsafe fn FPDFPageObj_GetFillColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        call_pdfium(
            8,
            page_object as usize as f64,
            R as usize as f64,
            G as usize as f64,
            B as usize as f64,
            A as usize as f64,
            0.0,
        ) as FPDF_BOOL
    }
    unsafe fn FPDFPageObj_GetMatrix(&self, page_object: FPDF_PAGEOBJECT, matrix: *mut FS_MATRIX) -> FPDF_BOOL {
        call_pdfium(9, page_object as usize as f64, matrix as usize as f64, 0.0, 0.0, 0.0, 0.0) as FPDF_BOOL
    }
    unsafe fn FPDFPageObj_GetType(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        call_pdfium(10, page_object as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as c_int
    }
    unsafe fn FPDFPage_CountObjects(&self, page: FPDF_PAGE) -> c_int {
        call_pdfium(11, page as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as c_int
    }
    unsafe fn FPDFPage_GetCropBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        call_pdfium(
            12,
            page as usize as f64,
            left as usize as f64,
            bottom as usize as f64,
            right as usize as f64,
            top as usize as f64,
            0.0,
        ) as FPDF_BOOL
    }
    unsafe fn FPDFPage_GetMediaBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        call_pdfium(
            13,
            page as usize as f64,
            left as usize as f64,
            bottom as usize as f64,
            right as usize as f64,
            top as usize as f64,
            0.0,
        ) as FPDF_BOOL
    }
    unsafe fn FPDFPage_GetObject(&self, page: FPDF_PAGE, index: c_int) -> FPDF_PAGEOBJECT {
        call_pdfium(14, page as usize as f64, index as f64, 0.0, 0.0, 0.0, 0.0) as usize as FPDF_PAGEOBJECT
    }
    unsafe fn FPDFPage_GetRotation(&self, page: FPDF_PAGE) -> c_int {
        call_pdfium(15, page as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as c_int
    }
    unsafe fn FPDFPathSegment_GetClose(&self, segment: FPDF_PATHSEGMENT) -> FPDF_BOOL {
        call_pdfium(16, segment as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as FPDF_BOOL
    }
    unsafe fn FPDFPathSegment_GetPoint(
        &self,
        segment: FPDF_PATHSEGMENT,
        x: *mut c_float,
        y: *mut c_float,
    ) -> FPDF_BOOL {
        call_pdfium(17, segment as usize as f64, x as usize as f64, y as usize as f64, 0.0, 0.0, 0.0) as FPDF_BOOL
    }
    unsafe fn FPDFPathSegment_GetType(&self, segment: FPDF_PATHSEGMENT) -> c_int {
        call_pdfium(18, segment as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as c_int
    }
    unsafe fn FPDFPath_CountSegments(&self, path: FPDF_PAGEOBJECT) -> c_int {
        call_pdfium(19, path as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as c_int
    }
    unsafe fn FPDFPath_GetDrawMode(
        &self,
        path: FPDF_PAGEOBJECT,
        fillmode: *mut c_int,
        stroke: *mut FPDF_BOOL,
    ) -> FPDF_BOOL {
        call_pdfium(20, path as usize as f64, fillmode as usize as f64, stroke as usize as f64, 0.0, 0.0, 0.0)
            as FPDF_BOOL
    }
    unsafe fn FPDFPath_GetPathSegment(&self, path: FPDF_PAGEOBJECT, index: c_int) -> FPDF_PATHSEGMENT {
        call_pdfium(21, path as usize as f64, index as f64, 0.0, 0.0, 0.0, 0.0) as usize as FPDF_PATHSEGMENT
    }
    unsafe fn FPDFTextObj_GetFont(&self, text: FPDF_PAGEOBJECT) -> FPDF_FONT {
        call_pdfium(22, text as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as usize as FPDF_FONT
    }
    unsafe fn FPDFText_ClosePage(&self, text_page: FPDF_TEXTPAGE) {
        call_pdfium(23, text_page as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0);
    }
    unsafe fn FPDFText_CountChars(&self, text_page: FPDF_TEXTPAGE) -> c_int {
        call_pdfium(24, text_page as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as c_int
    }
    unsafe fn FPDFText_GetCharOrigin(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        x: *mut c_double,
        y: *mut c_double,
    ) -> FPDF_BOOL {
        call_pdfium(25, text_page as usize as f64, index as f64, x as usize as f64, y as usize as f64, 0.0, 0.0)
            as FPDF_BOOL
    }
    unsafe fn FPDFText_GetFillColor(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        call_pdfium(
            26,
            text_page as usize as f64,
            index as f64,
            R as usize as f64,
            G as usize as f64,
            B as usize as f64,
            A as usize as f64,
        ) as FPDF_BOOL
    }
    unsafe fn FPDFText_GetFontSize(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_double {
        call_pdfium(27, text_page as usize as f64, index as f64, 0.0, 0.0, 0.0, 0.0) as c_double
    }
    unsafe fn FPDFText_GetLooseCharBox(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        rect: *mut FS_RECTF,
    ) -> FPDF_BOOL {
        call_pdfium(28, text_page as usize as f64, index as f64, rect as usize as f64, 0.0, 0.0, 0.0) as FPDF_BOOL
    }
    unsafe fn FPDFText_GetMatrix(&self, text_page: FPDF_TEXTPAGE, index: c_int, matrix: *mut FS_MATRIX) -> FPDF_BOOL {
        call_pdfium(29, text_page as usize as f64, index as f64, matrix as usize as f64, 0.0, 0.0, 0.0) as FPDF_BOOL
    }
    unsafe fn FPDFText_GetTextObject(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> FPDF_PAGEOBJECT {
        call_pdfium(30, text_page as usize as f64, index as f64, 0.0, 0.0, 0.0, 0.0) as usize as FPDF_PAGEOBJECT
    }
    unsafe fn FPDFText_GetUnicode(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_uint {
        call_pdfium(31, text_page as usize as f64, index as f64, 0.0, 0.0, 0.0, 0.0) as c_uint
    }
    unsafe fn FPDFText_HasUnicodeMapError(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        call_pdfium(32, text_page as usize as f64, index as f64, 0.0, 0.0, 0.0, 0.0) as c_int
    }
    unsafe fn FPDFText_IsGenerated(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        call_pdfium(33, text_page as usize as f64, index as f64, 0.0, 0.0, 0.0, 0.0) as c_int
    }
    unsafe fn FPDFText_LoadPage(&self, page: FPDF_PAGE) -> FPDF_TEXTPAGE {
        call_pdfium(34, page as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as usize as FPDF_TEXTPAGE
    }
    unsafe fn FPDF_CloseDocument(&self, document: FPDF_DOCUMENT) {
        call_pdfium(35, document as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0);
    }
    unsafe fn FPDF_ClosePage(&self, page: FPDF_PAGE) {
        call_pdfium(36, page as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0);
    }
    unsafe fn FPDF_GetPageCount(&self, document: FPDF_DOCUMENT) -> c_int {
        call_pdfium(37, document as usize as f64, 0.0, 0.0, 0.0, 0.0, 0.0) as c_int
    }
    unsafe fn FPDF_LoadPage(&self, document: FPDF_DOCUMENT, page_index: c_int) -> FPDF_PAGE {
        call_pdfium(38, document as usize as f64, page_index as f64, 0.0, 0.0, 0.0, 0.0) as usize as FPDF_PAGE
    }
}
