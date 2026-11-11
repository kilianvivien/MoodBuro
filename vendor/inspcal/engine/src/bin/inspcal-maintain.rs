//! Repository maintenance, not a distributed user CLI.
use inspcal_engine::{maintain, pipeline};
use pdfium_render::prelude::Pdfium;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const HELP: &str = "inspcal-maintain <check|parse|add-week|debug> <week.pdf> --pdfium <library>
  --output PATH     JSON / Markdown file, or directory for debug SVGs
  --cohort FI       optional cohort filter (check/parse)
  --corpus PATH     add-week corpus (default: repository tests/)
  --dry-run         proof-read without writing
  --replace         explicitly replace an existing fixture and golden
The PDFium library can also be set with INSPCAL_PDFIUM.";

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or(HELP)?;
    if command == "--help" || command == "-h" {
        println!("{HELP}");
        return Ok(());
    }
    if !["check", "parse", "add-week", "debug"].contains(&command.as_str()) {
        return Err(HELP.to_owned());
    }
    let mut file = None;
    let mut library = std::env::var("INSPCAL_PDFIUM").ok();
    let mut output = None;
    let mut cohort = None;
    let mut corpus = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("tests");
    let (mut replace, mut dry) = (false, false);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pdfium" => library = Some(args.next().ok_or(HELP)?),
            "--output" | "-o" => output = Some(PathBuf::from(args.next().ok_or(HELP)?)),
            "--cohort" => cohort = Some(args.next().ok_or(HELP)?),
            "--corpus" => corpus = PathBuf::from(args.next().ok_or(HELP)?),
            "--replace" => replace = true,
            "--dry-run" => dry = true,
            value if !value.starts_with('-') && file.is_none() => file = Some(PathBuf::from(value)),
            _ => return Err(HELP.to_owned()),
        }
    }
    if command == "add-week" && cohort.is_some() {
        return Err("add-week stores all cohorts".to_owned());
    }
    let file = file.ok_or(HELP)?;
    let name = file.file_name().and_then(|name| name.to_str()).ok_or("Unreadable file name")?;
    let library = library.ok_or("Set INSPCAL_PDFIUM or --pdfium")?;
    let bytes = std::fs::read(&file).map_err(|error| error.to_string())?;
    let bindings = Pdfium::bind_to_library(library).map_err(|error| format!("PDFium: {error:?}"))?;
    // SAFETY: one initialisation, before all reads, with destruction after extraction.
    unsafe { bindings.FPDF_InitLibrary() };
    let extracted = inspcal_engine::extract(bindings.as_ref(), &bytes, name);
    unsafe { bindings.FPDF_DestroyLibrary() };
    let document = extracted.map_err(|error| error.to_string())?;
    if command == "debug" {
        let output = output.unwrap_or_else(|| PathBuf::from("debug"));
        std::fs::create_dir_all(&output).map_err(|error| error.to_string())?;
        for (name, svg) in maintain::overlays(&document) {
            let target = output.join(name);
            std::fs::write(&target, svg).map_err(|error| error.to_string())?;
            println!("{}", target.display());
        }
        return Ok(());
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_secs() as i64;
    let cohorts = match cohort.map(|cohort| cohort.to_uppercase()) {
        None => None,
        Some(cohort) if cohort == "ALL" => None,
        Some(cohort) if inspcal_engine::institution::INSTITUTION.cohort_tokens.contains(&cohort) => Some(vec![cohort]),
        Some(_) => return Err("Unknown cohort".to_owned()),
    };
    let timetable = pipeline::parse_document(&document, cohorts.as_deref(), now).map_err(|error| error.to_string())?;
    match command.as_str() {
        "parse" => {
            let json = serde_json::to_string_pretty(&timetable.to_json()).map_err(|error| error.to_string())? + "\n";
            if let Some(output) = output {
                std::fs::write(output, json).map_err(|error| error.to_string())?;
            } else {
                print!("{json}");
            }
        }
        "check" => {
            let report = maintain::report(&timetable);
            if let Some(output) = output {
                std::fs::write(output, report).map_err(|error| error.to_string())?;
            } else {
                print!("{report}");
            }
        }
        "add-week" => {
            print!("{}", maintain::report(&timetable));
            if inspcal_engine::dates::week_bounds_from_filename(name).map_err(|error| error.to_string())?.is_none() {
                return Err("Name the file Edt du JJ.MM.AAAA au JJ.MM.AAAA.pdf".to_owned());
            }
            if dry {
                println!("Dry run: nothing written.");
                return Ok(());
            }
            let fixture = corpus.join("fixtures").join(name);
            let golden = corpus.join("golden").join(format!("{}.json", file.file_stem().unwrap().to_string_lossy()));
            if !replace && (fixture.exists() || golden.exists()) {
                return Err("Fixture or golden already exists; use --replace after proof-reading".to_owned());
            }
            let json = serde_json::to_string_pretty(&timetable.to_json()).map_err(|error| error.to_string())? + "\n";
            let contracts_file = corpus.join("golden/contracts.json");
            let mut contracts: serde_json::Value = if contracts_file.exists() {
                serde_json::from_str(&std::fs::read_to_string(&contracts_file).map_err(|error| error.to_string())?)
                    .map_err(|error| format!("Unreadable contracts: {error}"))?
            } else {
                serde_json::json!({"parser_version": pipeline::PARSER_VERSION,
                    "reference": "Rust maintainer output; proof-read every added week", "fixtures": {}})
            };
            let references = contracts["fixtures"].as_object_mut().ok_or("Invalid contract fixture map")?;
            if !replace && references.contains_key(name) {
                return Err("Contract already exists; use --replace".to_owned());
            }
            references.insert(name.to_owned(), maintain::contract(&document, now).map_err(|error| error.to_string())?);
            let contracts_json = serde_json::to_string_pretty(&contracts).map_err(|error| error.to_string())? + "\n";
            std::fs::create_dir_all(fixture.parent().unwrap()).map_err(|error| error.to_string())?;
            std::fs::create_dir_all(golden.parent().unwrap()).map_err(|error| error.to_string())?;
            // Parsing and validation above finish before either destination is touched.
            std::fs::write(&fixture, bytes).map_err(|error| error.to_string())?;
            std::fs::write(&golden, json).map_err(|error| error.to_string())?;
            std::fs::write(&contracts_file, contracts_json).map_err(|error| error.to_string())?;
            println!(
                "{}\n{}\nProof-read against the PDF before trusting this golden.",
                fixture.display(),
                golden.display()
            );
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
