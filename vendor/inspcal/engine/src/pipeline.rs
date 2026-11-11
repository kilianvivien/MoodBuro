//! A read document to a validated timetable: `src/edt_parser/pipeline.py`.

use crate::dates;
use crate::errors::{warning, EdtError};
use crate::model::Document;
use crate::py::civil;
use crate::timetable::{Source, Timetable};
use crate::{events, grid, validate};

/// One parser version for native maintenance and the browser. Reviewed
/// golden UIDs and calendar bytes pin the identity contract across releases.
pub const PARSER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `Path(value).name`: the last component, `.` and empty ones aside.
pub fn file_name(value: &str) -> &str {
    value.split('/').rfind(|part| !part.is_empty() && *part != ".").unwrap_or("")
}

/// Build and validate the timetable of an extracted document.
///
/// `cohorts` keeps the sessions of those cohorts and the ones naming none;
/// `now` is the moment written as `parsed_at`.
pub fn parse_document(document: &Document, cohorts: Option<&[String]>, now: i64) -> Result<Timetable, EdtError> {
    let source_name = document.file.as_str();
    let bounds = dates::week_bounds_from_filename(source_name)?;
    let built = grid::build(document)?;
    let parsed = events::build(document, &built)?;
    let provenance = Source {
        file: source_name.to_owned(),
        sha256: document.sha256.clone(),
        week_start: bounds.map(|(start, _)| start),
        week_end: bounds.map(|(_, end)| end),
        promotion: events::promotion_of(&parsed),
        revision: dates::revision_of(&document.metadata),
        parsed_at: Some(civil::iso_utc(now)),
        parser_version: Some(PARSER_VERSION.to_owned()),
    };
    let mut timetable = Timetable::new(provenance, parsed, built.warnings);
    if bounds.is_none() {
        // The file name carried no week, so the year came from the PDF's
        // creation date. That is a guess, and it is also the one case where
        // the dates cannot be checked against the week — say both out loud.
        timetable.warn(
            warning::YEAR_FROM_METADATA,
            format!(
                "« {source_name} » ne nomme pas sa semaine : l'année vient de la date de création du PDF, et les dates \
                 ne sont pas vérifiées contre la semaine. Renommez le fichier « Edt du JJ.MM.AAAA au JJ.MM.AAAA.pdf » \
                 pour lever le doute."
            ),
            None,
        );
    }
    if let Some(cohorts) = cohorts.filter(|cohorts| !cohorts.is_empty()) {
        let wanted: Vec<String> = cohorts.iter().map(|token| token.to_uppercase()).collect();
        timetable
            .events
            .retain(|event| event.cohorts.is_empty() || event.cohorts.iter().any(|token| wanted.contains(token)));
        let retained: Vec<String> = timetable.events.iter().map(|event| event.uid()).collect();
        timetable.warnings.retain(|warning| warning.event_uid.as_ref().is_none_or(|uid| retained.contains(uid)));
    }
    validate::validate(&mut timetable);
    Ok(timetable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::tests::{document_of, page_of, HEADERS};

    #[test]
    fn file_names_lose_their_directories() {
        assert_eq!(file_name("a/b/week.pdf"), "week.pdf");
        assert_eq!(file_name("a/b/"), "b");
        assert_eq!(file_name("./"), "");
        assert_eq!(file_name(""), "");
    }

    #[test]
    fn a_week_is_parsed_sorted_filtered_and_checked() {
        let rows: [&[&str]; 3] = [
            &["08/09", "09h00", "10h00", "Droit", "", "FI 2026-2028", "C101", "", ""],
            &["07/09", "06h00", "10h00", "Gestion", "", "CIL 2026-2028", "C102", "", ""],
            &["07/09", "09h00", "11h00", "Accueil", "", "", "", "", ""],
        ];
        let document = document_of("Edt du 07.09.2026 au 11.09.2026.pdf", vec![page_of(&HEADERS, &rows)]);
        let all = parse_document(&document, None, 1_788_000_000).unwrap();
        let titles: Vec<&str> = all.events.iter().map(|event| event.title.as_str()).collect();
        assert_eq!(titles, ["Gestion", "Accueil", "Droit"]);
        assert_eq!(all.source.promotion.as_deref(), Some("2026-2028"));
        assert_eq!(all.source.parsed_at.as_deref(), Some("2026-08-29T10:40:00+00:00"));
        let codes: Vec<&str> = all.warnings.iter().map(|warning| warning.code).collect();
        assert_eq!(codes, ["IMPLAUSIBLE_TIME", "NO_COHORT", "EVENT_OVERLAP"]);

        let only = parse_document(&document, Some(&["fi".to_owned()]), 0).unwrap();
        let titles: Vec<&str> = only.events.iter().map(|event| event.title.as_str()).collect();
        assert_eq!(titles, ["Accueil", "Droit"]);
    }

    #[test]
    fn a_public_holiday_is_a_quiet_day_off_for_everyone() {
        let mut page = page_of(&HEADERS, &[&["10/11", "09h00", "10h00", "Droit", "", "FI 2026-2028", "C101", "", ""]]);
        page.cells.push(crate::grid::tests::cell(20.0, 100.0, 110.0, 140.0, "11/11"));
        page.cells.push(crate::grid::tests::cell(110.0, 100.0, 830.0, 140.0, "Férié"));
        let document = document_of("Edt du 09.11.2026 au 13.11.2026.pdf", vec![page]);
        let week = parse_document(&document, Some(&["fi".to_owned()]), 0).unwrap();
        let holiday = week.events.iter().find(|event| event.title == "Férié").unwrap();
        assert!(crate::events::is_holiday(holiday));
        assert!(week.warnings.is_empty(), "{:?}", week.warnings);
        let description = crate::ics::render_description(holiday, "week.pdf");
        assert!(description.starts_with("Jour férié : journée sans cours."));
        assert!(!description.contains("Horaire"));
    }

    #[test]
    fn a_file_that_names_no_week_takes_its_year_from_the_pdf() {
        let rows: [&[&str]; 1] = [&["07/09", "09h00", "10h00", "Droit", "", "FI", "C101", "", ""]];
        let mut document = document_of("planning.pdf", vec![page_of(&HEADERS, &rows)]);
        assert_eq!(parse_document(&document, None, 0).unwrap_err().code, "DATE_PARSE");
        document.metadata.insert("CreationDate".to_owned(), "D:20270901120000+02'00'".to_owned());
        let timetable = parse_document(&document, None, 0).unwrap();
        assert_eq!(timetable.events[0].date.iso(), "2027-09-07");
        assert_eq!(timetable.warnings[0].code, "YEAR_FROM_METADATA");
        assert_eq!(timetable.source.revision.as_deref(), Some("2027-09-01T12:00:00+02:00"));
    }
}
