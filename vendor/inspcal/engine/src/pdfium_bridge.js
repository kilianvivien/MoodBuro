// Read-only wasm32 PDFium ABI. Entries match ReadBindings in pdfium_bindings.rs.
// These functions are synchronous and never re-enter Rust while scratch is in
// use. Heap views are refreshed on growth in either module; the fixed scratch
// buffer lives for the worker's lifetime. No glyph/table logic lives here.
const definitions = [
  ["FPDFAction_GetType", 1, []],
  ["FPDFFont_GetAscent", 3, [[2, 4]]],
  ["FPDFFont_GetDescent", 3, [[2, 4]]],
  ["FPDFFormObj_CountObjects", 1, []],
  ["FPDFFormObj_GetObject", 2, []],
  ["FPDFLink_Enumerate", 3, [[1, 4], [2, 4]]],
  ["FPDFLink_GetAction", 1, []],
  ["FPDFLink_GetAnnotRect", 2, [[1, 16]]],
  ["FPDFPageObj_GetFillColor", 5, [[1, 4], [2, 4], [3, 4], [4, 4]]],
  ["FPDFPageObj_GetMatrix", 2, [[1, 24]]],
  ["FPDFPageObj_GetType", 1, []],
  ["FPDFPage_CountObjects", 1, []],
  ["FPDFPage_GetCropBox", 5, [[1, 4], [2, 4], [3, 4], [4, 4]]],
  ["FPDFPage_GetMediaBox", 5, [[1, 4], [2, 4], [3, 4], [4, 4]]],
  ["FPDFPage_GetObject", 2, []],
  ["FPDFPage_GetRotation", 1, []],
  ["FPDFPathSegment_GetClose", 1, []],
  ["FPDFPathSegment_GetPoint", 3, [[1, 4], [2, 4]]],
  ["FPDFPathSegment_GetType", 1, []],
  ["FPDFPath_CountSegments", 1, []],
  ["FPDFPath_GetDrawMode", 3, [[1, 4], [2, 4]]],
  ["FPDFPath_GetPathSegment", 2, []],
  ["FPDFTextObj_GetFont", 1, []],
  ["FPDFText_ClosePage", 1, []],
  ["FPDFText_CountChars", 1, []],
  ["FPDFText_GetCharOrigin", 4, [[2, 8], [3, 8]]],
  ["FPDFText_GetFillColor", 6, [[2, 4], [3, 4], [4, 4], [5, 4]]],
  ["FPDFText_GetFontSize", 2, []],
  ["FPDFText_GetLooseCharBox", 3, [[2, 16]]],
  ["FPDFText_GetMatrix", 3, [[2, 24]]],
  ["FPDFText_GetTextObject", 2, []],
  ["FPDFText_GetUnicode", 2, []],
  ["FPDFText_HasUnicodeMapError", 2, []],
  ["FPDFText_IsGenerated", 2, []],
  ["FPDFText_LoadPage", 1, []],
  ["FPDF_CloseDocument", 1, []],
  ["FPDF_ClosePage", 1, []],
  ["FPDF_GetPageCount", 1, []],
  ["FPDF_LoadPage", 2, []],
];
let pdfium, rustMemory, rustHeap, scratch, calls;
export function initializeFastPdfium(module, memory) {
  if (calls) throw new Error("PDFium bridge already initialized");
  pdfium = module;
  rustMemory = memory;
  scratch = pdfium._malloc(128);
  if (!scratch) throw new Error("PDFium scratch allocation failed");
  calls = definitions.map(([name, count, outputs]) => {
    const fn = pdfium[`_${name}`];
    if (typeof fn !== "function") throw new Error(`PDFium function missing: ${name}`);
    return { fn, args: new Array(count).fill(0), outputs };
  });
}
export function callPdfium(op, a, b, c, d, e, f) {
  if (!rustHeap || rustHeap.buffer !== rustMemory.buffer) rustHeap = new Uint8Array(rustMemory.buffer);
  const entry = calls[op], args = entry.args;
  if (args.length > 0) args[0] = a;
  if (args.length > 1) args[1] = b;
  if (args.length > 2) args[2] = c;
  if (args.length > 3) args[3] = d;
  if (args.length > 4) args[4] = e;
  if (args.length > 5) args[5] = f;
  let offset = 0;
  for (const [index, size] of entry.outputs) {
    const pointer = args[index];
    if (!pointer) throw new Error("Null PDFium output pointer");
    // Save the original Rust address separately from the argument substituted
    // for PDFium. Maximum 4 outputs, each with an 8-byte-aligned scratch slot.
    pdfium.HEAPU8.set(rustHeap.subarray(pointer, pointer + size), scratch + offset);
    args[index] = scratch + offset;
    offset += (size + 7) & ~7;
  }
  const result = entry.fn(...args);
  if (result && entry.outputs.length) {
    // PDFium can grow its heap during a call, invalidating its old typed view.
    let offset = 0;
    for (const [index, size] of entry.outputs) {
      const pointer = index === 0 ? a : index === 1 ? b : index === 2 ? c : index === 3 ? d : index === 4 ? e : f;
      rustHeap.set(pdfium.HEAPU8.subarray(scratch + offset, scratch + offset + size), pointer);
      offset += (size + 7) & ~7;
    }
  }
  return result || 0;
}
