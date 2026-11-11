//! Maintainer-only proof-reading and grid diagnostics. Never shipped in WASM.
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;

use crate::model::Document;
use crate::timetable::Timetable;

/// Calendar references for a newly proof-read week. Existing reference files
/// are changed only by the maintainer's explicit add-week/--replace action.
pub fn contract(document: &Document, now: i64) -> Result<Value, crate::errors::EdtError> {
    let mut contract = Map::new();
    for cohort in ["ALL", "FI", "CIL", "OFF"] {
        let cohorts = (cohort != "ALL").then(|| vec![cohort.to_owned()]);
        let table = crate::pipeline::parse_document(document, cohorts.as_deref(), now)?;
        let pivot = table.to_json();
        let mut reference = json!({"status": 200, "event_uids": pivot["events"].as_array().unwrap().iter()
            .map(|event| event["uid"].clone()).collect::<Vec<_>>()});
        if !table.events.is_empty() {
            let options =
                crate::ics::IcsOptions { prefix: "INSP ".to_owned(), alarm_minutes: Some(15), ..Default::default() };
            let calendars = crate::ics::to_calendars_multi(&[table], &options)?;
            reference["calendar_sha256"] =
                json!(Sha256::digest(&calendars[0].1).iter().map(|byte| format!("{byte:02x}")).collect::<String>());
        }
        contract.insert(cohort.to_owned(), reference);
    }
    Ok(Value::Object(contract))
}

pub fn report(timetable: &Timetable) -> String {
    let value = timetable.to_json();
    let mut out = format!(
        "# {}\n\n{} séances, {} avertissements\n\n",
        timetable.source.file,
        timetable.events.len(),
        timetable.warnings.len()
    );
    for event in value["events"].as_array().into_iter().flatten() {
        let text = |key: &str| event[key].as_str().unwrap_or("");
        writeln!(out, "- {} {}–{} — {}", text("date"), text("start"), text("end"), text("title")).unwrap();
        writeln!(
            out,
            "  Salle : {} ; parcours : {} ; cohorte : {}",
            text("location"),
            text("module"),
            text("cohort_label")
        )
        .unwrap();
        if let Some(groups) = event["raw"]["group_details"].as_array() {
            for group in groups {
                writeln!(out, "  Groupe : {group}").unwrap();
            }
        }
        if event["remote"].as_bool() == Some(true) {
            out.push_str("  ⚑ À DISTANCE : vérifier le soulignement dans le PDF.\n");
        }
        let words = format!("{} {} {}", text("title"), text("description"), event["notes"]).to_lowercase();
        if ["annul", "reporté", "reportee", "reportée"].iter().any(|word| words.contains(word)) {
            out.push_str("  ⚑ ANNULATION POSSIBLE : vérifier le PDF et le statut du calendrier.\n");
        }
    }
    out.push_str("\n## Avertissements — à justifier contre le PDF\n\n");
    for warning in &timetable.warnings {
        writeln!(out, "- [{}] {}", warning.code, warning.message).unwrap();
    }
    out
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// Vector grid diagnostic: the extracted visible lines, numbered cells and
/// table bounds, in original PDF coordinates. No renderer or upload is needed.
pub fn overlays(document: &Document) -> Vec<(String, String)> {
    document.pages.iter().map(|page| {
        let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\"><title>{}</title><rect width=\"100%\" height=\"100%\" fill=\"white\"/>", page.width, page.height, xml(&document.file));
        for (index, cell) in page.cells.iter().enumerate() {
            let [left, top, right, bottom] = cell.bbox;
            write!(svg, "<rect x=\"{left}\" y=\"{top}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"#0ea5e9\" stroke-width=\"0.5\"/><text x=\"{left}\" y=\"{}\" fill=\"#e11d48\" font-size=\"5\">{}</text>", right-left, bottom-top, top+5.0, index+1).unwrap();
            for line in &cell.lines {
                let [x, _, _, baseline] = line.bbox;
                write!(svg, "<text x=\"{x}\" y=\"{baseline}\" font-family=\"sans-serif\" font-size=\"{}\" font-weight=\"{}\">{}</text>", line.size, if line.bold { "bold" } else { "normal" }, xml(&line.text)).unwrap();
            }
        }
        if let Some([left, top, right, bottom]) = page.table_bbox {
            write!(svg, "<rect x=\"{left}\" y=\"{top}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"#e11d48\" stroke-width=\"1\"/>", right-left, bottom-top).unwrap();
        }
        svg.push_str("</svg>\n");
        (format!("page-{}.svg", page.number), svg)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_text_cannot_inject_svg() {
        let mut document = crate::grid::tests::document_of(
            "<&\".pdf",
            vec![crate::grid::tests::page_of(
                &crate::grid::tests::HEADERS,
                &[&["07/09", "09h00", "10h00", "<script>&", "", "FI", "C101", "", ""]],
            )],
        );
        document.pages[0].cells[0].lines[0].text = "<script>".to_owned();
        let (_, svg) = &overlays(&document)[0];
        assert!(!svg.contains("<script>"));
        assert!(svg.contains("&lt;script&gt;"));
    }
}
