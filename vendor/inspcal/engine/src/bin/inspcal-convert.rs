//! Run the whole engine from a terminal, for comparison with Python's.
//!
//!     inspcal-convert parse --pdfium <libpdfium> [--cohort FI] week.pdf
//!     inspcal-convert parse --document dump.json [--cohort FI] week.pdf
//!     inspcal-convert convert --pdfium <libpdfium> [--now 1788000000] < request.json
//!     inspcal-convert calendars < weeks.json
//!
//! `parse` prints `{"timetable": …}` or `{"error": …}` for one PDF, the line
//! `edt_parser.worker` writes. With `--document` the PDF is not read: the
//! logic runs on a document written by `tools/dump_document.py`, so that a
//! difference from Python can only come from the logic.
//!
//! `convert` answers one request as the page would send it, and prints
//! `{"status", "body"}`. The request names its files by path:
//!
//!     {"fields": {"cohort": "FI"}, "files": [{"name": "…", "type": "application/pdf", "path": "…"}]}
//!
//! `calendars` writes the calendar files of weeks already parsed, given as
//! `{"timetables": [pivot JSON, …], "prefix", "alarm", "split"}`. It reads no
//! PDF and needs no PDFium.
//!
//! The PDFium library can also be named by the `INSPCAL_PDFIUM` variable.
//! Exit status: 0 when an answer was printed, 1 on misuse.

use std::io::Read;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use inspcal_engine::convert::{self, Upload};
use inspcal_engine::{Document, ExtractError};
use pdfium_render::prelude::{Pdfium, PdfiumLibraryBindings};
use serde_json::Value;

const USAGE: &str = "usage: inspcal-convert parse (--pdfium <lib> | --document <dump.json>) [--cohort <X>] <file.pdf>
       inspcal-convert convert --pdfium <lib> [--now <seconds>] < request.json
       inspcal-convert calendars < weeks.json";

struct Arguments {
    command: String,
    library: Option<String>,
    document: Option<String>,
    cohort: Option<String>,
    now: i64,
    file: Option<String>,
}

fn arguments() -> Option<Arguments> {
    let mut given = std::env::args().skip(1);
    let mut parsed = Arguments {
        command: given.next()?,
        library: std::env::var("INSPCAL_PDFIUM").ok(),
        document: None,
        cohort: None,
        now: SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs() as i64),
        file: None,
    };
    while let Some(argument) = given.next() {
        match argument.as_str() {
            "--pdfium" => parsed.library = Some(given.next()?),
            "--document" => parsed.document = Some(given.next()?),
            "--cohort" => parsed.cohort = Some(given.next()?),
            "--now" => parsed.now = given.next()?.parse().ok()?,
            _ => parsed.file = Some(argument),
        }
    }
    Some(parsed)
}

fn request() -> Result<Value, String> {
    let mut request = String::new();
    std::io::stdin().read_to_string(&mut request).map_err(|error| format!("cannot read the request: {error}"))?;
    serde_json::from_str(&request).map_err(|error| format!("the request is not JSON: {error}"))
}

fn run(arguments: &Arguments, reader: &mut convert::Reader) -> Result<String, String> {
    match arguments.command.as_str() {
        "parse" => {
            let file = arguments.file.as_ref().ok_or(USAGE)?;
            let bytes = std::fs::read(file).map_err(|error| format!("cannot read {file}: {error}"))?;
            let cohorts = arguments.cohort.clone().map(|cohort| vec![cohort]);
            Ok(convert::parse(&bytes, file, cohorts.as_deref(), reader, arguments.now).to_string())
        }
        "convert" => {
            let request = request()?;
            let mut files = Vec::new();
            for file in request["files"].as_array().into_iter().flatten() {
                let path = file["path"].as_str().ok_or("a file of the request names no path")?;
                files.push(Upload {
                    name: file["name"].as_str().map(str::to_owned),
                    content_type: file["type"].as_str().map(str::to_owned),
                    bytes: std::fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))?,
                });
            }
            Ok(convert::convert(&request["fields"].to_string(), &files, reader, arguments.now))
        }
        _ => Err(USAGE.to_owned()),
    }
}

fn main() -> ExitCode {
    let Some(arguments) = arguments() else {
        eprintln!("{USAGE}");
        return ExitCode::from(1);
    };
    let result = if arguments.command == "calendars" {
        request().map(|request| convert::calendars(&request).to_string())
    } else if let Some(dump) = &arguments.document {
        let document =
            std::fs::read_to_string(dump).map_err(|error| format!("cannot read {dump}: {error}")).and_then(|text| {
                serde_json::from_str::<Document>(&text).map_err(|error| format!("{dump} is not a document: {error}"))
            });
        match document {
            Err(error) => Err(error),
            Ok(document) => run(&arguments, &mut |_: &[u8], file: &str| -> Result<Document, ExtractError> {
                Ok(Document { file: file.to_owned(), ..document.clone() })
            }),
        }
    } else if let Some(library) = &arguments.library {
        match Pdfium::bind_to_library(library) {
            Err(error) => Err(format!("cannot load PDFium from {library}: {error:?}")),
            Ok(bindings) => {
                let bindings: &dyn PdfiumLibraryBindings = bindings.as_ref();
                // SAFETY: the library is initialised once, before any other
                // call, and destroyed after the last one.
                unsafe { bindings.FPDF_InitLibrary() };
                let result =
                    run(&arguments, &mut |bytes: &[u8], file: &str| inspcal_engine::extract(bindings, bytes, file));
                unsafe { bindings.FPDF_DestroyLibrary() };
                result
            }
        }
    } else {
        Err(USAGE.to_owned())
    };
    match result {
        Ok(answer) => {
            println!("{answer}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
