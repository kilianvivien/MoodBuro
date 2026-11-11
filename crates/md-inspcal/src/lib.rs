//! MoodBuro's adapter around the vendored native InspCal engine.
//!
//! The engine (`vendor/inspcal/engine`, see `vendor/inspcal-provenance.json`)
//! turns timetable PDF bytes into a weekly calendar. This crate owns the parts
//! MoodBuro needs around it:
//!
//! * one process-wide PDFium, initialised once and never destroyed while a job
//!   could still use it ([`engine`]);
//! * one conversion at a time, admitted with a non-waiting [`try_admit`] before
//!   any fetch or read, whose [`Permit`] travels into the blocking job;
//! * a soft wait budget ([`run_job`]): waiting stops, the native parse does
//!   not, and its late result is dropped while the permit stays held;
//! * a self-check against a reviewed public fixture ([`smoke`]).
//!
//! PDF bytes never leave Rust. Nothing here touches the network, SQLite or
//! the filesystem.

pub mod request;
pub mod week;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use base64::Engine as _;
pub use inspcal_engine::convert::Upload;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Bumped when MoodBuro's expectations of the engine's payloads change.
pub const ADAPTER_CONTRACT: u32 = 1;
/// The engine's own total upload limit; one more byte is read to detect it.
pub const MAX_PDF_BYTES: usize = 4_000_000;
/// The initial UI wait budget once the bytes are in hand.
pub const DEFAULT_WAIT: Duration = Duration::from_secs(45);
/// Engine semantic version, from the vendored snapshot.
pub const ENGINE_VERSION: &str = inspcal_engine::pipeline::PARSER_VERSION;

/// Version of the vendored institution rules (`institution.json`).
pub fn rules_version() -> String {
    inspcal_engine::institution::INSTITUTION.version.clone()
}

/// The Moodle origin the vendored institution rules were written for, taken
/// from the rules themselves (`institution.json`), so the gate moves with them.
pub fn supported_origin() -> Option<url::Origin> {
    let rules = url::Url::parse(&inspcal_engine::institution::INSTITUTION.rotation_url).ok()?;
    (rules.scheme() == "https").then(|| rules.origin())
}

/// Whether the timetable feature is offered for a saved site: only the
/// institution's own Moodle, over HTTPS. Every timetable entry point checks
/// this in Rust; the shell only mirrors it for display.
pub fn supports_site(wwwroot: &str) -> bool {
    let (Some(origin), Ok(site)) = (supported_origin(), url::Url::parse(wwwroot)) else {
        return false;
    };
    site.scheme() == "https"
        && site.username().is_empty()
        && site.password().is_none()
        && site.origin() == origin
}

/// The engine reads a week's dates from its file name
/// (`Edt du 09.11.2026 au 13.11.2026.pdf`). Moodle link labels often spell
/// them out instead (`Edt du 09 novembre au 13 novembre 26`). When `filename`
/// carries no readable week but `label` states one completely, including its
/// year, return an engine-readable name; otherwise return `filename` as is.
/// A year is never inferred.
pub fn filename_with_week(filename: &str, label: &str) -> String {
    if matches!(
        inspcal_engine::dates::week_bounds_from_filename(filename),
        Ok(Some(_))
    ) {
        return filename.to_owned();
    }
    match week_from_label(label) {
        Some(((d1, m1, y1), (d2, m2, y2))) => {
            let candidate = format!("Edt du {d1:02}.{m1:02}.{y1} au {d2:02}.{m2:02}.{y2}.pdf");
            match inspcal_engine::dates::week_bounds_from_filename(&candidate) {
                Ok(Some(_)) => candidate,
                _ => filename.to_owned(),
            }
        }
        None => filename.to_owned(),
    }
}

/// Whether a link label looks like a weekly timetable, to decide if the
/// “Open as timetable” item is offered. A hint only: the parser decides.
pub fn looks_like_timetable(label: &str) -> bool {
    let folded = label.to_lowercase();
    let words: Vec<&str> = folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    words.contains(&"edt")
        || folded.contains("emploi du temps")
        || week_from_label(label).is_some()
        || matches!(
            inspcal_engine::dates::week_bounds_from_filename(label),
            Ok(Some(_))
        )
}

type Day = (u32, u32, i32);

fn week_from_label(label: &str) -> Option<(Day, Day)> {
    const MONTHS: [(&str, u32); 12] = [
        ("janv", 1),
        ("fevr", 2),
        ("mars", 3),
        ("avr", 4),
        ("mai", 5),
        ("juin", 6),
        ("juil", 7),
        ("aout", 8),
        ("sept", 9),
        ("oct", 10),
        ("nov", 11),
        ("dec", 12),
    ];
    let folded: String = label
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' => 'e',
            'û' | 'ù' => 'u',
            'ô' => 'o',
            '.' | ',' | '_' | '-' => ' ',
            other => other,
        })
        .collect();
    let words: Vec<&str> = folded.split_whitespace().collect();
    let month = |word: &str| {
        MONTHS
            .iter()
            .find(|(prefix, _)| word.starts_with(prefix))
            .map(|(_, n)| *n)
    };
    let day = |word: &str| {
        word.trim_end_matches("er")
            .parse::<u32>()
            .ok()
            .filter(|d| (1..=31).contains(d))
    };
    let start = words.iter().position(|w| *w == "du")?;
    let [d1, m1, au, d2, m2, year, ..] = words.get(start + 1..)? else {
        return None;
    };
    if *au != "au" {
        return None;
    }
    let (d1, m1, d2, m2) = (day(d1)?, month(m1)?, day(d2)?, month(m2)?);
    let year: i32 = match year.len() {
        2 => 2000 + year.parse::<i32>().ok()?,
        4 => year.parse().ok()?,
        _ => return None,
    };
    let start_year = if m1 > m2 { year - 1 } else { year };
    Some(((d1, m1, start_year), (d2, m2, year)))
}

/// Stable failure codes, translated by the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineError {
    /// No PDFium on this platform or it failed to initialise.
    Unavailable,
    /// Another conversion holds the permit.
    Busy,
    /// The soft wait budget elapsed; the job keeps running and is discarded.
    Timeout,
    /// The engine panicked; nothing it produced is used.
    Internal,
}

impl EngineError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Unavailable => "ENGINE_UNAVAILABLE",
            Self::Busy => "ENGINE_BUSY",
            Self::Timeout => "ENGINE_TIMEOUT",
            Self::Internal => "ENGINE_INTERNAL",
        }
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for EngineError {}

static ADMITTED: AtomicBool = AtomicBool::new(false);

/// The right to run the one native conversion. Released when dropped, which
/// for a timed-out job is when the job actually ends.
#[derive(Debug)]
pub struct Permit {
    _private: (),
}

impl Drop for Permit {
    fn drop(&mut self) {
        ADMITTED.store(false, Ordering::Release);
    }
}

/// Admit a conversion without waiting. Call before fetching or reading the
/// PDF so a second activation is refused instead of queued.
pub fn try_admit() -> Result<Permit, EngineError> {
    ADMITTED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map(|_| Permit { _private: () })
        .map_err(|_| EngineError::Busy)
}

/// The process-wide native engine.
pub struct Engine {
    #[cfg(target_os = "macos")]
    bindings: Box<dyn pdfium_render::prelude::PdfiumLibraryBindings>,
}

#[cfg(target_os = "macos")]
fn initialise() -> Result<Engine, EngineError> {
    use pdfium_render::prelude::Pdfium;
    let bindings =
        Pdfium::bind_to_statically_linked_library().map_err(|_| EngineError::Unavailable)?;
    // SAFETY: called exactly once, before any other PDFium call; the library
    // is never destroyed, so no job can outlive it.
    unsafe { bindings.FPDF_InitLibrary() };
    Ok(Engine { bindings })
}

#[cfg(not(target_os = "macos"))]
fn initialise() -> Result<Engine, EngineError> {
    Err(EngineError::Unavailable)
}

/// The engine, initialising PDFium on first use.
pub fn engine() -> Result<&'static Engine, EngineError> {
    static ENGINE: std::sync::OnceLock<Result<Engine, EngineError>> = std::sync::OnceLock::new();
    ENGINE
        .get_or_init(initialise)
        .as_ref()
        .map_err(|error| *error)
}

impl Engine {
    /// Answer one engine request (`{"status","body"}`), as InspCal's page
    /// would. `fields` are the engine's string fields. Holding the permit is
    /// required; panics are contained and reported as [`EngineError::Internal`].
    pub fn convert(
        &self,
        _permit: &Permit,
        fields: &Value,
        uploads: &[Upload],
        now: i64,
    ) -> Result<Value, EngineError> {
        let fields = fields.to_string();
        let answer = catch_unwind(AssertUnwindSafe(|| {
            self.convert_unchecked(&fields, uploads, now)
        }))
        .map_err(|_| EngineError::Internal)?;
        serde_json::from_str(&answer).map_err(|_| EngineError::Internal)
    }

    #[cfg(target_os = "macos")]
    fn convert_unchecked(&self, fields: &str, uploads: &[Upload], now: i64) -> String {
        let bindings = self.bindings.as_ref();
        let mut reader = |bytes: &[u8], file: &str| inspcal_engine::extract(bindings, bytes, file);
        inspcal_engine::convert::convert(fields, uploads, &mut reader, now)
    }

    #[cfg(not(target_os = "macos"))]
    fn convert_unchecked(&self, _fields: &str, _uploads: &[Upload], _now: i64) -> String {
        unreachable!("no engine is constructed without PDFium")
    }
}

/// Calendar files for weeks already parsed; needs no PDFium.
pub fn calendars(request: &Value) -> Result<Value, EngineError> {
    catch_unwind(AssertUnwindSafe(|| {
        inspcal_engine::convert::calendars(request)
    }))
    .map_err(|_| EngineError::Internal)
}

/// Run a native job on its own thread and wait at most `budget`. On timeout
/// the caller gets [`EngineError::Timeout`]; the job keeps the permit until it
/// really ends and its result is dropped, never delivered.
pub fn run_job<T, F>(permit: Permit, budget: Duration, job: F) -> Result<T, EngineError>
where
    T: Send + 'static,
    F: FnOnce(&Permit) -> T + Send + 'static,
{
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("inspcal-engine".into())
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| job(&permit)));
            // Release before answering, so a caller that has its result can
            // admit the next conversion at once.
            drop(permit);
            // A dropped receiver means the caller stopped waiting: discard.
            let _ = sender.send(result);
        })
        .map_err(|_| EngineError::Unavailable)?;
    match receiver.recv_timeout(budget) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_)) => Err(EngineError::Internal),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(EngineError::Timeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(EngineError::Internal),
    }
}

/// The public, fictional fixture compiled in for the packaged-app self-check.
const SMOKE_NAME: &str = "Edt du 09.11.2026 au 13.11.2026.pdf";
const SMOKE_PDF: &[u8] = include_bytes!(
    "../../../vendor/inspcal/tests/fixtures/public/Edt du 09.11.2026 au 13.11.2026.pdf"
);
const SMOKE_CONTRACTS: &str =
    include_str!("../../../vendor/inspcal/tests/golden/public/contracts.json");

/// What [`smoke`] observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeReport {
    pub events: usize,
    pub calendar_sha256: String,
    pub elapsed_ms: u128,
}

/// Convert the compiled-in public fixture and compare it with InspCal's
/// reviewed contract (event UIDs and calendar hash, cohort FI).
pub fn smoke() -> Result<SmokeReport, String> {
    let engine = engine().map_err(|error| error.to_string())?;
    let permit = try_admit().map_err(|error| error.to_string())?;
    let started = std::time::Instant::now();
    let fields = serde_json::json!({"cohort": "FI", "alarm": "15", "prefix": "INSP "});
    let upload = Upload {
        name: Some(SMOKE_NAME.into()),
        content_type: Some("application/pdf".into()),
        bytes: SMOKE_PDF.to_vec(),
    };
    let answer = engine
        .convert(&permit, &fields, &[upload], 1_788_000_000)
        .map_err(|error| error.to_string())?;
    let elapsed_ms = started.elapsed().as_millis();
    let contracts: Value =
        serde_json::from_str(SMOKE_CONTRACTS).map_err(|error| error.to_string())?;
    let contract = &contracts["fixtures"][SMOKE_NAME]["FI"];
    if answer["status"] != contract["status"] {
        return Err(format!(
            "status {} != {}",
            answer["status"], contract["status"]
        ));
    }
    let uids: Vec<&Value> = answer["body"]["timetable"]["events"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|event| &event["uid"])
        .collect();
    let expected: Vec<&Value> = contract["event_uids"]
        .as_array()
        .into_iter()
        .flatten()
        .collect();
    if uids != expected {
        return Err("event UIDs differ from the reviewed contract".into());
    }
    let calendar = base64::engine::general_purpose::STANDARD
        .decode(
            answer["body"]["download"]["base64"]
                .as_str()
                .unwrap_or_default(),
        )
        .map_err(|error| error.to_string())?;
    let calendar_sha256 = hex(&Sha256::digest(&calendar));
    if contract["calendar_sha256"].as_str() != Some(calendar_sha256.as_str()) {
        return Err("calendar differs from the reviewed contract".into());
    }
    Ok(SmokeReport {
        events: uids.len(),
        calendar_sha256,
        elapsed_ms,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn engine_and_permit_cross_threads() {
        assert_send_sync::<Engine>();
        fn assert_send<T: Send>() {}
        assert_send::<Permit>();
    }

    #[test]
    fn only_the_institutions_moodle_is_supported() {
        let origin = supported_origin().expect("rules name an https Moodle");
        let root = origin.ascii_serialization();
        assert!(supports_site(&format!("{root}/")));
        assert!(supports_site(&format!("{root}/some/path/")));
        let host = root.trim_start_matches("https://");
        for other in [
            format!("http://{host}/"),
            format!("https://{host}.evil.example/"),
            format!("https://evil.example/{host}/"),
            format!("https://user@{host}/"),
            format!("https://{host}:8443/"),
            "https://moodle.example.org/".into(),
            "not a url".into(),
        ] {
            assert!(!supports_site(&other), "{other}");
        }
    }

    #[test]
    fn link_labels_give_the_engine_a_week_only_when_complete() {
        let named = "Edt du 09.11.2026 au 13.11.2026.pdf";
        assert_eq!(filename_with_week(named, "anything"), named);
        assert_eq!(
            filename_with_week("edt.pdf", "Edt du 09 novembre au 13 novembre 26"),
            "Edt du 09.11.2026 au 13.11.2026.pdf"
        );
        assert_eq!(
            filename_with_week("edt.pdf", "EDT du 29 décembre au 2 janvier 2027"),
            "Edt du 29.12.2026 au 02.01.2027.pdf"
        );
        assert_eq!(
            filename_with_week("edt.pdf", "Edt du 1er sept. au 5 sept. 26"),
            "Edt du 01.09.2026 au 05.09.2026.pdf"
        );
        for vague in [
            "Edt du 09 novembre au 13 novembre",
            "Emploi du temps",
            "Edt du 9 au 13 novembre 26",
            "",
        ] {
            assert_eq!(filename_with_week("edt.pdf", vague), "edt.pdf", "{vague}");
        }
        // More than seven days is not a week: the engine's own check refuses it.
        assert_eq!(
            filename_with_week("edt.pdf", "du 1 novembre au 30 novembre 26"),
            "edt.pdf"
        );
    }

    #[test]
    fn timetable_labels_are_recognised() {
        for yes in [
            "Edt du 09 novembre au 13 novembre 26",
            "EDT semaine 45",
            "Emploi du temps S1",
            "Edt du 09.11.2026 au 13.11.2026.pdf",
            "Semaine du 9 nov. au 13 nov. 2026",
        ] {
            assert!(looks_like_timetable(yes), "{yes}");
        }
        for no in [
            "Calendrier synoptique de septembre",
            "Calendrier mensuel prévisionnel",
            "Credit",
            "Règlement des études",
        ] {
            assert!(!looks_like_timetable(no), "{no}");
        }
    }

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(EngineError::Busy.code(), "ENGINE_BUSY");
        assert_eq!(EngineError::Timeout.code(), "ENGINE_TIMEOUT");
        assert_eq!(EngineError::Unavailable.code(), "ENGINE_UNAVAILABLE");
    }
}
