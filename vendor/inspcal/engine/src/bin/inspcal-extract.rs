//! Print the extracted document of a PDF as JSON, for comparison with Python's.
//!
//!     inspcal-extract --pdfium /path/to/libpdfium.dylib week.pdf
//!
//! The PDFium library can also be named by the `INSPCAL_PDFIUM` variable.
//! Exit status: 0 with the document, 2 with `{"error": …}`, 1 on misuse.

use std::path::Path;
use std::process::ExitCode;

use pdfium_render::prelude::Pdfium;

fn main() -> ExitCode {
    let mut library = std::env::var("INSPCAL_PDFIUM").ok();
    let mut file = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--pdfium" {
            library = arguments.next();
        } else {
            file = Some(argument);
        }
    }
    let (Some(library), Some(file)) = (library, file) else {
        eprintln!("usage: inspcal-extract --pdfium <libpdfium> <file.pdf>");
        return ExitCode::from(1);
    };
    let bindings = match Pdfium::bind_to_library(&library) {
        Ok(bindings) => bindings,
        Err(error) => {
            eprintln!("cannot load PDFium from {library}: {error:?}");
            return ExitCode::from(1);
        }
    };
    let bytes = match std::fs::read(&file) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {file}: {error}");
            return ExitCode::from(1);
        }
    };
    let name = Path::new(&file).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();

    // SAFETY: the library is initialised once, before any other call, and
    // destroyed after the last one.
    unsafe { bindings.FPDF_InitLibrary() };
    let result = inspcal_engine::extract(bindings.as_ref(), &bytes, &name);
    unsafe { bindings.FPDF_DestroyLibrary() };

    match result {
        Ok(document) => {
            println!("{}", serde_json::to_string(&document).expect("a document is always serialisable"));
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("{}", serde_json::json!({ "error": error }));
            ExitCode::from(2)
        }
    }
}
