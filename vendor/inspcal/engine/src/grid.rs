//! The logical table grid: `src/edt_parser/grid.py`.
//!
//! Column positions differ from one PDF to the next, so columns always come
//! from the header text. Continuation pages carry no header of their own and
//! reuse the first page's columns, after checking that their edges line up.

use std::collections::BTreeSet;

use crate::errors::{warning, EdtError};
use crate::institution::INSTITUTION;
use crate::model::{Cell, Document, Page};
use crate::py::text;
use crate::timetable::Warning;

/// How far a continuation page's edges may drift from the reference page's.
const PAGE_EDGE_TOLERANCE: f64 = 6.0;
/// Share of a column that a cell must cover to be considered part of it.
const COVERAGE_RATIO: f64 = 0.5;
/// Share of a strip that a cell must cover to be listed in it.
const STRIP_RATIO: f64 = 0.5;
/// Roles that must be recognised for a header to be read with confidence.
const MIN_HEADER_MATCHES: usize = 5;
/// Share of the reference edges a continuation page must still show.
const MIN_EDGE_OVERLAP: f64 = 2.0 / 3.0;

/// Without these four, no event can be built: their absence is blocking.
const REQUIRED_ROLES: [&str; 4] = ["date", "start", "end", "theme"];

/// Accent-free, lower-case, single-spaced form used for header matching.
pub fn fold(value: &str) -> String {
    let plain: String = text::unaccent(value)
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { ' ' })
        .collect();
    text::single_spaced(&plain)
}

/// Cluster nearby edge coordinates into single representative values.
pub fn snap(mut values: Vec<f64>, tolerance: f64) -> Vec<f64> {
    values.sort_by(f64::total_cmp);
    let mut clusters: Vec<Vec<f64>> = Vec::new();
    for value in values {
        match clusters.last_mut() {
            Some(cluster) if value - cluster[cluster.len() - 1] <= tolerance => cluster.push(value),
            _ => clusters.push(vec![value]),
        }
    }
    clusters.iter().map(|cluster| cluster.iter().sum::<f64>() / cluster.len() as f64).collect()
}

/// A named column and its horizontal extent on the page.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub role: String,
    pub label: String,
    pub x0: f64,
    pub x1: f64,
}

impl Column {
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    /// True when more than half of this column is covered by `cell`.
    pub fn covers(&self, cell: &Cell) -> bool {
        let overlap = self.x1.min(cell.bbox[2]) - self.x0.max(cell.bbox[0]);
        overlap > COVERAGE_RATIO * self.width()
    }
}

/// The whole document as columns plus body cells, pages already concatenated.
#[derive(Debug, Default)]
pub struct Grid {
    pub columns: Vec<Column>,
    pub extra_columns: Vec<Column>,
    /// Body cells, header rows excluded, in document order. A cell's position
    /// here is its identity: Python tells cells apart by `is`.
    pub cells: Vec<Cell>,
    pub warnings: Vec<Warning>,
    /// Named and unknown columns, left to right.
    all_columns: Vec<Column>,
    /// `roles_of` for every body cell, worked out once.
    roles: Vec<Vec<String>>,
}

impl Grid {
    pub fn column(&self, role: &str) -> Option<&Column> {
        self.columns.iter().find(|column| column.role == role)
    }

    /// Every column this cell covers; more than one means a merged cell.
    pub fn roles_of(&self, cell: &Cell) -> Vec<String> {
        self.all_columns.iter().filter(|column| column.covers(cell)).map(|column| column.role.clone()).collect()
    }

    /// `roles_of` for the body cell at `index`.
    pub fn roles(&self, index: usize) -> &[String] {
        &self.roles[index]
    }

    /// Body cells belonging to `role`, in document order.
    pub fn cells_for(&self, role: &str) -> Vec<usize> {
        let Some(column) = self.column(role) else { return Vec::new() };
        (0..self.cells.len()).filter(|&index| column.covers(&self.cells[index])).collect()
    }

    /// Cells belonging to `role` and to no other column.
    pub fn own_cells_for(&self, role: &str) -> Vec<usize> {
        self.cells_for(role).into_iter().filter(|&index| self.roles[index] == [role]).collect()
    }

    fn settle_columns(&mut self, columns: Vec<Column>, extra: Vec<Column>) {
        self.all_columns = columns.iter().chain(&extra).cloned().collect();
        self.all_columns.sort_by(|left, right| left.x0.total_cmp(&right.x0));
        self.columns = columns;
        self.extra_columns = extra;
    }
}

/// A horizontal slice of the table between two snapped edges: the cells in it.
fn strips_of(cells: &[Cell]) -> Vec<Vec<usize>> {
    let edges = snap(cells.iter().flat_map(|cell| [cell.bbox[1], cell.bbox[3]]).collect(), 1.0);
    let mut strips = Vec::new();
    for pair in edges.windows(2) {
        let (top, bottom) = (pair[0], pair[1]);
        let members: Vec<usize> = (0..cells.len())
            .filter(|&index| {
                cells[index].bbox[3].min(bottom) - cells[index].bbox[1].max(top) > STRIP_RATIO * (bottom - top)
            })
            .collect();
        if !members.is_empty() {
            strips.push(members);
        }
    }
    strips
}

/// Best role for a header cell, `None` when nothing matches.
fn match_role(label: &str, taken: &[String]) -> Option<String> {
    let folded = fold(label);
    let words: BTreeSet<&str> = folded.split(' ').filter(|word| !word.is_empty()).collect();
    if words.is_empty() {
        return None;
    }
    let mut best: Option<(&str, usize)> = None;
    for (role, aliases) in &INSTITUTION.header_aliases {
        if taken.contains(role) {
            continue;
        }
        for alias in aliases {
            let alias_words: BTreeSet<&str> = text::split(alias).into_iter().collect();
            if alias_words.is_subset(&words) && best.is_none_or(|(_, score)| alias_words.len() > score) {
                best = Some((role, alias_words.len()));
            }
        }
    }
    best.map(|(role, _)| role.to_owned())
}

/// Turn a candidate header strip into columns, or `None` if it is not one.
///
/// A strip is a header when it names every required role. A header that has
/// had another column renamed is read anyway and flagged by the caller.
fn header_columns(cells: &[Cell], strip: &[usize]) -> Option<(Vec<Column>, Vec<Column>)> {
    let mut ordered = strip.to_vec();
    ordered.sort_by(|&left, &right| cells[left].bbox[0].total_cmp(&cells[right].bbox[0]));
    let (mut columns, mut extra, mut taken) = (Vec::new(), Vec::new(), Vec::new());
    for index in ordered {
        let cell = &cells[index];
        let label = text::squeeze(&cell.text);
        match match_role(&cell.text, &taken) {
            None => {
                let name = fold(&label);
                let role = format!("extra:{}", if name.is_empty() { "unnamed" } else { &name });
                extra.push(Column { role, label, x0: cell.bbox[0], x1: cell.bbox[2] });
            }
            Some(role) => {
                taken.push(role.clone());
                columns.push(Column { role, label, x0: cell.bbox[0], x1: cell.bbox[2] });
            }
        }
    }
    if !REQUIRED_ROLES.iter().all(|role| taken.iter().any(|known| known == role)) {
        return None;
    }
    columns.sort_by(|left, right| left.x0.total_cmp(&right.x0));
    Some((columns, extra))
}

/// A continuation page must still show the reference page's column edges.
///
/// One exception: the line between two columns may move on a later page (the
/// 9–13 November 2026 week widens « Salle » by 21 pt on pages 2 and 3). It is
/// accepted when the reference boundary is absent from the page and the shift
/// stays under `COVERAGE_RATIO` of both neighbouring columns, so every cell is
/// still assigned to exactly the columns it would have been on the first page.
/// A boundary that is present *and* joined by a new one is an extra column,
/// and is still refused.
fn check_edges<'a>(cells: impl Iterator<Item = &'a Cell>, columns: &[Column]) -> Result<(), EdtError> {
    let mismatch =
        || EdtError::column_mismatch("les colonnes de cette page ne correspondent pas à celles de la première");
    let reference = snap(columns.iter().flat_map(|column| [column.x0, column.x1]).collect(), 1.0);
    let observed = snap(cells.flat_map(|cell| [cell.bbox[0], cell.bbox[2]]).collect(), 1.0);
    let near = |edge: f64, known: f64| (edge - known).abs() <= PAGE_EDGE_TOLERANCE;
    let mut matched: Vec<bool> =
        reference.iter().map(|&known| observed.iter().any(|&edge| near(edge, known))).collect();
    for &edge in &observed {
        if reference.iter().any(|&known| near(edge, known)) {
            continue;
        }
        let moved = (1..reference.len().saturating_sub(1)).find(|&index| {
            let narrower = (reference[index] - reference[index - 1]).min(reference[index + 1] - reference[index]);
            !matched[index] && (edge - reference[index]).abs() < COVERAGE_RATIO * narrower
        });
        match moved {
            Some(index) => matched[index] = true,
            None => return Err(mismatch()),
        }
    }
    let count = matched.iter().filter(|&&found| found).count();
    if (count as f64) < MIN_EDGE_OVERLAP * reference.len() as f64 {
        return Err(mismatch());
    }
    Ok(())
}

fn read_page(grid: &mut Grid, page: &Page) -> Result<(), EdtError> {
    let strips = strips_of(&page.cells);
    let header = strips.iter().find_map(|strip| header_columns(&page.cells, strip).map(|found| (strip, found)));
    let Some((strip, (columns, extra))) = header else {
        if grid.columns.is_empty() {
            return Err(EdtError::header_not_found(
                "aucune ligne d'en-tête reconnue — attendu Date / Début / Fin / Thème…",
            ));
        }
        check_edges(page.cells.iter(), &grid.all_columns)?;
        grid.cells.extend(page.cells.iter().cloned());
        return Ok(());
    };
    if grid.columns.is_empty() {
        for column in &extra {
            grid.warnings.push(Warning {
                code: warning::UNKNOWN_COLUMN,
                message: format!("colonne inconnue « {} » page {}", column.label, page.number),
                event_uid: None,
            });
        }
        if columns.len() < MIN_HEADER_MATCHES {
            // Enough to build events, not enough to be sure the grid is the
            // one we know. Say so loudly and keep going.
            let mut named: Vec<&str> = columns.iter().map(|column| column.role.as_str()).collect();
            named.sort_unstable();
            grid.warnings.push(Warning {
                code: warning::LOW_CONFIDENCE,
                message: format!(
                    "seules {} colonnes sur {} ont été reconnues ({}) : le modèle de l'école a probablement changé. \
                     Les séances ci-dessous sont à relire entièrement.",
                    columns.len(),
                    INSTITUTION.header_aliases.len(),
                    named.join(", ")
                ),
                event_uid: None,
            });
        }
        grid.settle_columns(columns, extra);
    } else {
        // A continuation page that repeats the header still has to agree with
        // the first page's geometry.
        check_edges(strip.iter().map(|&index| &page.cells[index]), &grid.all_columns)?;
    }
    grid.cells
        .extend((0..page.cells.len()).filter(|index| !strip.contains(index)).map(|index| page.cells[index].clone()));
    Ok(())
}

/// Detect the header, name the columns and gather the body cells.
///
/// Unknown columns are kept as `extra:<name>` and reported as a warning,
/// never as an error.
pub fn build(document: &Document) -> Result<Grid, EdtError> {
    let mut grid = Grid::default();
    for page in &document.pages {
        read_page(&mut grid, page)?;
    }
    if REQUIRED_ROLES.iter().any(|role| grid.column(role).is_none()) {
        return Err(EdtError::missing_column("colonnes obligatoires manquantes"));
    }
    grid.roles = grid.cells.iter().map(|cell| grid.roles_of(cell)).collect();
    Ok(grid)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::TextLine;
    use std::collections::BTreeMap;

    pub(crate) fn cell(x0: f64, top: f64, x1: f64, bottom: f64, content: &str) -> Cell {
        let lines = content
            .split('\n')
            .filter(|line| !line.is_empty())
            .enumerate()
            .map(|(index, line)| TextLine {
                text: line.to_owned(),
                bbox: [x0 + 2.0, top + 2.0 + 12.0 * index as f64, x1 - 2.0, top + 12.0 + 12.0 * index as f64],
                bold: false,
                size: 10.0,
                color: None,
            })
            .collect();
        Cell {
            bbox: [x0, top, x1, bottom],
            page: 1,
            text: content.to_owned(),
            lines,
            fill: None,
            underlined: false,
            links: Vec::new(),
        }
    }

    pub(crate) const HEADERS: [&str; 9] = [
        "Date",
        "Début",
        "Fin",
        "Thème de la séance",
        "Intervenant(s)",
        "Cohorte concernée",
        "Salle",
        "Préparation de la session",
        "Module",
    ];

    /// A page whose rows are given as nine texts each, under the usual header.
    pub(crate) fn page_of(headers: &[&str], rows: &[&[&str]]) -> Page {
        let mut cells = Vec::new();
        let width = 90.0;
        for (column, header) in headers.iter().enumerate() {
            cells.push(cell(20.0 + width * column as f64, 40.0, 20.0 + width * (column + 1) as f64, 60.0, header));
        }
        for (row, texts) in rows.iter().enumerate() {
            let top = 60.0 + 40.0 * row as f64;
            for (column, content) in texts.iter().enumerate() {
                cells.push(cell(
                    20.0 + width * column as f64,
                    top,
                    20.0 + width * (column + 1) as f64,
                    top + 40.0,
                    content,
                ));
            }
        }
        Page { number: 1, width: 842.0, height: 595.0, cells, links: Vec::new(), table_bbox: None, table_count: 1 }
    }

    pub(crate) fn document_of(file: &str, pages: Vec<Page>) -> Document {
        Document { file: file.to_owned(), sha256: "0".repeat(64), pages, metadata: BTreeMap::new() }
    }

    #[test]
    fn fold_keeps_letters_and_digits_only() {
        assert_eq!(fold("  Thème de la  séance\n(s) "), "theme de la seance s");
        assert_eq!(fold("Après-midi"), "apres midi");
        assert_eq!(fold("—"), "");
    }

    #[test]
    fn the_header_names_the_columns() {
        let page = page_of(&HEADERS, &[&["07/09", "09h00", "10h00", "Cours", "", "FI", "C101", "", "Droit"]]);
        let grid = build(&document_of("w.pdf", vec![page])).unwrap();
        let roles: Vec<&str> = grid.columns.iter().map(|column| column.role.as_str()).collect();
        assert_eq!(roles, crate::timetable::COLUMN_ROLES);
        assert_eq!(grid.cells.len(), 9);
        assert_eq!(grid.roles(3), ["theme"]);
        assert!(grid.warnings.is_empty());
    }

    #[test]
    fn an_unknown_column_is_kept_and_reported() {
        let mut headers = HEADERS.to_vec();
        headers[8] = "Observations";
        let grid = build(&document_of("w.pdf", vec![page_of(&headers, &[])])).unwrap();
        assert_eq!(grid.extra_columns[0].role, "extra:observations");
        assert_eq!(grid.warnings[0].message, "colonne inconnue « Observations » page 1");
    }

    #[test]
    fn a_page_without_a_header_is_refused_first_and_checked_after() {
        let no_header = page_of(&["a", "b"], &[]);
        assert_eq!(build(&document_of("w.pdf", vec![no_header.clone()])).unwrap_err().code, "HEADER_NOT_FOUND");
        let first = page_of(&HEADERS, &[]);
        assert_eq!(build(&document_of("w.pdf", vec![first.clone(), no_header])).unwrap_err().code, "COLUMN_MISMATCH");
        let mut second = page_of(&HEADERS, &[&["07/09", "09h00", "10h00", "Cours", "", "FI", "C101", "", "Droit"]]);
        second.cells.drain(..9);
        assert_eq!(build(&document_of("w.pdf", vec![first, second])).unwrap().cells.len(), 9);
    }

    /// Moves the boundary between columns `left` and `left + 1` by `shift`.
    fn shift_boundary(page: &mut Page, left: usize, shift: f64) {
        let boundary = 20.0 + 90.0 * (left + 1) as f64;
        for cell in &mut page.cells {
            for edge in [0, 2] {
                if (cell.bbox[edge] - boundary).abs() < 0.5 {
                    cell.bbox[edge] += shift;
                }
            }
        }
    }

    #[test]
    fn a_continuation_page_may_move_one_column_boundary() {
        let row: &[&str] = &["07/09", "09h00", "10h00", "Cours", "", "FI", "C101", "Lire", "Droit"];
        let first = page_of(&HEADERS, &[]);
        let mut second = page_of(&HEADERS, &[row]);
        second.cells.drain(..9);
        shift_boundary(&mut second, 6, 21.0);
        let grid = build(&document_of("w.pdf", vec![first.clone(), second.clone()])).unwrap();
        assert_eq!(grid.roles(6), ["location"]);
        assert_eq!(grid.roles(7), ["preparation"]);

        // Past half of the narrower neighbour, cells would change column.
        let mut far = page_of(&HEADERS, &[row]);
        far.cells.drain(..9);
        shift_boundary(&mut far, 6, 50.0);
        assert_eq!(build(&document_of("w.pdf", vec![first.clone(), far])).unwrap_err().code, "COLUMN_MISMATCH");

        // The original boundary still present beside a new one is an extra column.
        let mut split = page_of(&HEADERS, &[row]);
        split.cells.drain(..9);
        split.cells.push(cell(560.0, 60.0, 581.0, 100.0, "x"));
        assert_eq!(build(&document_of("w.pdf", vec![first, split])).unwrap_err().code, "COLUMN_MISMATCH");
    }

    #[test]
    fn four_recognised_columns_are_read_with_a_warning() {
        let headers = ["Date", "Début", "Fin", "Thème", "Qui", "Pour qui", "Où", "Avant", "Quoi"];
        let grid = build(&document_of("w.pdf", vec![page_of(&headers, &[])])).unwrap();
        assert_eq!(grid.columns.len(), 4);
        assert_eq!(grid.warnings.last().unwrap().code, "LOW_CONFIDENCE");
        assert!(grid
            .warnings
            .last()
            .unwrap()
            .message
            .starts_with("seules 4 colonnes sur 9 ont été reconnues (date, end, start, theme)"));
    }
}
