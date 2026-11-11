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
    fn error_codes_are_stable() {
        assert_eq!(EngineError::Busy.code(), "ENGINE_BUSY");
        assert_eq!(EngineError::Timeout.code(), "ENGINE_TIMEOUT");
        assert_eq!(EngineError::Unavailable.code(), "ENGINE_UNAVAILABLE");
    }
}
