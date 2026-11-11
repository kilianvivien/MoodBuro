//! Fail early, with the fix, when the prepared PDFium is missing on macOS.
fn main() {
    println!("cargo:rerun-if-env-changed=PDFIUM_STATIC_LIB_PATH");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let directory = std::env::var("PDFIUM_STATIC_LIB_PATH").unwrap_or_default();
    let library = std::path::Path::new(&directory).join("libpdfium.a");
    println!("cargo:rerun-if-changed={}", library.display());
    if !library.is_file() {
        panic!(
            "PDFium is not prepared ({} is missing). Run: python3 scripts/prepare-inspcal-pdfium.py",
            library.display()
        );
    }
}
