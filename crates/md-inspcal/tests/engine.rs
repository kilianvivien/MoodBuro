//! Native engine contract: InspCal's reviewed public goldens, malformed input,
//! admission and the soft wait budget. The admission permit is process-wide,
//! so every test here takes `SERIAL` first.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use md_inspcal::{run_job, try_admit, EngineError};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn admission_refuses_a_second_conversion_until_the_first_ends() {
    let _serial = serial();
    let first = try_admit().expect("first admission");
    assert_eq!(try_admit().unwrap_err(), EngineError::Busy);
    drop(first);
    drop(try_admit().expect("admitted again once released"));
}

#[test]
fn a_timed_out_job_keeps_the_permit_and_its_result_is_discarded() {
    let _serial = serial();
    let finished = Arc::new(AtomicBool::new(false));
    let flag = finished.clone();
    let permit = try_admit().unwrap();
    let started = Instant::now();
    let outcome = run_job(permit, Duration::from_millis(50), move |_permit| {
        std::thread::sleep(Duration::from_millis(400));
        flag.store(true, Ordering::SeqCst);
        "late result"
    });
    assert_eq!(outcome.unwrap_err(), EngineError::Timeout);
    assert!(
        started.elapsed() < Duration::from_millis(300),
        "waiting stopped at the budget"
    );
    assert_eq!(
        try_admit().unwrap_err(),
        EngineError::Busy,
        "the running job still holds the permit"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(permit) = try_admit() {
            assert!(
                finished.load(Ordering::SeqCst),
                "released only after the job ended"
            );
            drop(permit);
            break;
        }
        assert!(Instant::now() < deadline, "permit never released");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_panicking_job_releases_the_permit() {
    let _serial = serial();
    let outcome: Result<(), _> = run_job(try_admit().unwrap(), Duration::from_secs(5), |_| {
        panic!("engine bug")
    });
    assert_eq!(outcome.unwrap_err(), EngineError::Internal);
    drop(try_admit().expect("released before the caller is answered"));
}

#[test]
fn a_finished_job_has_released_the_permit_when_its_result_arrives() {
    let _serial = serial();
    for _ in 0..200 {
        let value = run_job(try_admit().unwrap(), Duration::from_secs(5), |_| 7).unwrap();
        assert_eq!(value, 7);
        drop(try_admit().expect("no window where the answer precedes the release"));
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn the_engine_is_unavailable_without_pdfium() {
    assert_eq!(md_inspcal::engine().err(), Some(EngineError::Unavailable));
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use base64::Engine as _;
    use md_inspcal::Upload;
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use std::path::{Path, PathBuf};

    fn vendored(relative: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/inspcal")
            .join(relative)
    }

    fn upload(name: &str, bytes: Vec<u8>) -> Upload {
        Upload {
            name: Some(name.into()),
            content_type: Some("application/pdf".into()),
            bytes,
        }
    }

    fn convert(fields: Value, uploads: Vec<Upload>) -> Value {
        let engine = md_inspcal::engine().expect("PDFium");
        let permit = try_admit().expect("admitted");
        engine
            .convert(&permit, &fields, &uploads, 1_788_000_000)
            .expect("engine answer")
    }

    fn calendar_hash(answer: &Value) -> String {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(
                answer["body"]["download"]["base64"]
                    .as_str()
                    .expect("download"),
            )
            .unwrap();
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn public_fixtures_match_inspcals_reviewed_goldens() {
        let _serial = serial();
        let contracts: Value = serde_json::from_slice(
            &std::fs::read(vendored("tests/golden/public/contracts.json")).unwrap(),
        )
        .unwrap();
        let mut names: Vec<String> = std::fs::read_dir(vendored("tests/fixtures/public"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".pdf"))
            .collect();
        names.sort();
        assert!(names.len() >= 7, "public fixtures missing");
        let mut cases = 0;
        for name in &names {
            let bytes = std::fs::read(vendored(&format!("tests/fixtures/public/{name}"))).unwrap();
            for cohort in ["ALL", "FI", "CIL", "OFF"] {
                let contract = &contracts["fixtures"][name][cohort];
                let answer = convert(
                    json!({"cohort": cohort, "alarm": "15", "prefix": "INSP "}),
                    vec![upload(name, bytes.clone())],
                );
                assert_eq!(
                    answer["status"], contract["status"],
                    "{name}/{cohort} status"
                );
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
                assert_eq!(uids, expected, "{name}/{cohort} UIDs");
                if let Some(hash) = contract["calendar_sha256"].as_str() {
                    assert_eq!(calendar_hash(&answer), hash, "{name}/{cohort} calendar");
                }
                if cohort == "ALL" {
                    let golden = vendored(&format!(
                        "tests/golden/public/{}",
                        name.replace(".pdf", ".json")
                    ));
                    let mut expected: Value =
                        serde_json::from_slice(&std::fs::read(golden).unwrap()).unwrap();
                    let actual = &answer["body"]["parsed"][0]["timetable"];
                    expected["source"]
                        .as_object_mut()
                        .unwrap()
                        .remove("parsed_at");
                    for (key, value) in expected.as_object().unwrap() {
                        let mut actual = actual[key].clone();
                        if key == "source" {
                            actual.as_object_mut().unwrap().remove("parsed_at");
                        }
                        assert_eq!(&actual, value, "{name} pivot {key}");
                    }
                }
                cases += 1;
            }
        }
        assert_eq!(cases, names.len() * 4);
    }

    #[test]
    fn a_choice_edit_reuses_the_parsed_pivot_without_reading_the_pdf() {
        let _serial = serial();
        let name = "Edt du 26.10.2026 au 30.10.2026.pdf";
        let bytes = std::fs::read(vendored(&format!("tests/fixtures/public/{name}"))).unwrap();
        let first = convert(json!({"cohort": "ALL"}), vec![upload(name, bytes)]);
        assert_eq!(first["status"], 200);
        let parsed = first["body"]["parsed"].to_string();
        let again = convert(json!({"cohort": "ALL", "timetables": parsed}), vec![]);
        assert_eq!(again["status"], 200);
        assert_eq!(
            again["body"]["timetable"]["events"],
            first["body"]["timetable"]["events"]
        );
    }

    #[test]
    fn malformed_input_is_refused_without_crashing() {
        let _serial = serial();
        let real = std::fs::read(vendored(
            "tests/fixtures/public/Edt du 09.11.2026 au 13.11.2026.pdf",
        ))
        .unwrap();
        let mut noise = Vec::with_capacity(64 * 1024);
        let mut state = 0x9e37_79b9_u32;
        for _ in 0..noise.capacity() {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            noise.push(state as u8);
        }
        let mut header_then_noise = b"%PDF-1.7\n".to_vec();
        header_then_noise.extend_from_slice(&noise);
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("empty", Vec::new()),
            ("noise", noise),
            ("header then noise", header_then_noise),
            ("truncated", real[..real.len() / 2].to_vec()),
            (
                "html login page",
                b"<!doctype html><title>Login</title>".to_vec(),
            ),
            ("over the limit", vec![b'%'; md_inspcal::MAX_PDF_BYTES + 1]),
        ];
        for (label, bytes) in cases {
            let answer = convert(
                json!({"cohort": "FI"}),
                vec![upload("Edt du 09.11.2026 au 13.11.2026.pdf", bytes)],
            );
            assert_ne!(answer["status"], 200, "{label} must not convert");
            assert!(
                answer["body"]["download"].is_null(),
                "{label} must produce no calendar"
            );
        }
    }

    #[test]
    fn the_real_engine_runs_inside_the_job_runner() {
        let _serial = serial();
        let bytes = std::fs::read(vendored(
            "tests/fixtures/public/Edt du 02.11.2026 au 06.11.2026.pdf",
        ))
        .unwrap();
        let engine = md_inspcal::engine().unwrap();
        let answer = run_job(
            try_admit().unwrap(),
            md_inspcal::DEFAULT_WAIT,
            move |permit| {
                engine.convert(
                    permit,
                    &json!({"cohort": "FI"}),
                    &[upload("Edt du 02.11.2026 au 06.11.2026.pdf", bytes)],
                    0,
                )
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(answer["status"], 200);
        drop(try_admit().expect("released with the answer"));
    }

    #[test]
    fn the_packaged_self_check_passes() {
        let _serial = serial();
        let report = md_inspcal::smoke().expect("smoke");
        assert!(report.events > 0);
    }
}
