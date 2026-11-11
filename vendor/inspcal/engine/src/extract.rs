//! `src/edt_parser/pdf_extract.py`, function for function.
//!
//! These PDFs are Word exports: every cell border is a *filled* white rectangle
//! 0.48 pt high, hyperlink underlines are blue rectangles 0.72 pt high, and cell
//! backgrounds are plain filled rectangles. Nothing is stroked.
//!
//! The constants and the order of operations are the Python module's. Change
//! one here and the two readers stop agreeing on the same PDF.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::model::{BBox, Cell, Document, ExtractError, Link, Page, Rgb, TextLine};
use crate::objects::{classify, Rect};
use crate::raw::{py_isspace, Char, RawDocument, RawPage};
use crate::tables::{find_tables, Table};
use crate::text::extract_text;

pub const MAX_PAGE_OBJECTS: usize = 30_000;

/// A rectangle thinner than this in one direction is a rule, not a fill.
const THIN_MAX: f64 = 2.0;
/// …and shorter than this in the other direction is noise.
const THIN_MIN_LENGTH: f64 = 5.0;
/// A fill must be at least this big in both directions to count as a background.
const FILL_MIN: f64 = 2.0;
/// Vertical gap between a text line and the rule underlining it.
const UNDERLINE_GAP: f64 = 4.0;
/// Chars whose tops differ by less than this belong to the same text line.
const LINE_TOLERANCE: f64 = 3.0;
/// Share of a blank glyph that must sit on a real one for it to be an artefact.
const STRAY_SPACE_OVERLAP: f64 = 0.8;
/// How far off the line a blank glyph spread over two letters must sit.
const STRAY_SPACE_SHIFT: f64 = 1.0;
/// Tables smaller than this fraction of the biggest one are decorations.
const MIN_TABLE_AREA_RATIO: f64 = 0.2;
/// Column edges land 3-4 pt apart when an inner rectangle doubles them.
const X_EDGE_TOLERANCE: f64 = 4.0;
/// Row edges are never closer than ~9 pt, so this only removes exact duplicates.
const Y_EDGE_TOLERANCE: f64 = 2.0;
/// A text line must cross a row boundary by this much to prove it is not one.
const STRADDLE_MIN: f64 = 1.5;

const WHITE: Rgb = [1.0, 1.0, 1.0];
/// The blue Word gives hyperlink underlines — those are links, not "remote".
const LINK_BLUE: Rgb = [0.02, 0.388, 0.757];
const COLOR_TOLERANCE: f64 = 0.06;

fn same_color(left: Option<Rgb>, right: Rgb) -> bool {
    left.is_some_and(|left| left.iter().zip(right).all(|(a, b)| (a - b).abs() <= COLOR_TOLERANCE))
}

/// A thin filled rectangle: a cell border (white) or an underline.
struct Rule {
    bbox: BBox,
    color: Option<Rgb>,
}

impl Rule {
    fn is_border(&self) -> bool {
        same_color(self.color, WHITE)
    }

    fn is_link_underline(&self) -> bool {
        same_color(self.color, LINK_BLUE)
    }
}

type Fill = (BBox, Rgb);

fn center(bbox: &BBox) -> (f64, f64) {
    ((bbox[0] + bbox[2]) / 2.0, (bbox[1] + bbox[3]) / 2.0)
}

fn area(bbox: &BBox) -> f64 {
    (bbox[2] - bbox[0]).max(0.0) * (bbox[3] - bbox[1]).max(0.0)
}

fn contains(outer: &BBox, inner: &BBox, tolerance: f64) -> bool {
    outer[0] - tolerance <= inner[0]
        && outer[1] - tolerance <= inner[1]
        && inner[2] <= outer[2] + tolerance
        && inner[3] <= outer[3] + tolerance
}

fn overlaps(left: &BBox, right: &BBox) -> bool {
    const TOLERANCE: f64 = 0.5;
    left[2].min(right[2]) - left[0].max(right[0]) > TOLERANCE
        && left[3].min(right[3]) - left[1].max(right[1]) > TOLERANCE
}

/// Python's `round(value, 2)`: correctly rounded, ties to even.
fn round2(value: f64) -> f64 {
    format!("{value:.2}").parse().unwrap_or(value)
}

/// Cluster nearby edge coordinates into single representative values.
fn snap(mut values: Vec<f64>, tolerance: f64) -> Vec<f64> {
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

fn snapped(value: f64, edges: &[f64], tolerance: f64) -> f64 {
    let mut nearest = edges[0];
    for &edge in edges {
        if (edge - value).abs() < (nearest - value).abs() {
            nearest = edge;
        }
    }
    if (nearest - value).abs() <= tolerance {
        nearest
    } else {
        value
    }
}

/// True when a visible character sits astride `edge`.
fn line_crosses(edge: f64, column_chars: &[&Char]) -> bool {
    column_chars.iter().any(|c| c.top + STRADDLE_MIN <= edge && edge <= c.bottom - STRADDLE_MIN)
}

/// True when one cell background runs across `edge` with no border on it.
///
/// Word sometimes shades a cell line by line: a rectangle per text line, nested
/// inside the cell's own background. The table finder reads their edges as row
/// boundaries, and since they fall *between* lines no text straddles them. The
/// cell's background is the proof: a real row boundary separates two fills and
/// carries a white border rule, a line shading sits on one unbroken fill.
fn spanned_by_fill(x0: f64, x1: f64, edge: f64, fills: &[Fill], rules: &[Rule]) -> bool {
    let spanned = fills.iter().any(|(bbox, color)| {
        !same_color(Some(*color), WHITE)
            && (bbox[0] - x0).abs() <= X_EDGE_TOLERANCE
            && (bbox[2] - x1).abs() <= X_EDGE_TOLERANCE
            && bbox[1] + STRADDLE_MIN <= edge
            && edge <= bbox[3] - STRADDLE_MIN
    });
    spanned
        && !rules.iter().any(|rule| {
            rule.is_border()
                && rule.bbox[1] - STRADDLE_MIN <= edge
                && edge <= rule.bbox[3] + STRADDLE_MIN
                && rule.bbox[2].min(x1) - rule.bbox[0].max(x0) > (x1 - x0) / 2.0
        })
}

/// Re-join cells that an inner rectangle split in two.
///
/// A nested rectangle inside a cell makes the table finder see a row boundary
/// where there is none. A text line running across that boundary proves it is
/// an artefact — a real row never cuts a line in half — and so does a single
/// cell background running across it.
fn merge_fragments(boxes: Vec<BBox>, chars: &[Char], fills: &[Fill], rules: &[Rule]) -> Vec<BBox> {
    // Columns in the order they are first met, keyed on their rounded span.
    let mut order: Vec<(u64, u64)> = Vec::new();
    let mut columns: HashMap<(u64, u64), (f64, f64, Vec<BBox>)> = HashMap::new();
    for bbox in boxes {
        let (x0, x1) = (round2(bbox[0]) + 0.0, round2(bbox[2]) + 0.0);
        let key = (x0.to_bits(), x1.to_bits());
        columns
            .entry(key)
            .or_insert_with(|| {
                order.push(key);
                (x0, x1, Vec::new())
            })
            .2
            .push(bbox);
    }

    let mut merged: Vec<BBox> = Vec::new();
    for key in order {
        let (x0, x1, mut group) = columns.remove(&key).expect("every key in `order` was inserted");
        group.sort_by(|a, b| a[1].total_cmp(&b[1]));
        let column_chars: Vec<&Char> = chars
            .iter()
            .filter(|c| {
                let middle = (c.x0 + c.x1) / 2.0;
                x0 <= middle && middle <= x1 && !c.is_space()
            })
            .collect();
        let mut current = group[0];
        for bbox in &group[1..] {
            let edge = (current[3] + bbox[1]) / 2.0;
            if (bbox[1] - current[3]).abs() < 0.6
                && (line_crosses(edge, &column_chars) || spanned_by_fill(x0, x1, edge, fills, rules))
            {
                current = [x0, current[1], x1, bbox[3]];
            } else {
                merged.push(current);
                current = *bbox;
            }
        }
        merged.push(current);
    }
    merged.sort_by(|a, b| a[1].total_cmp(&b[1]).then(a[0].total_cmp(&b[0])));
    merged
}

/// Snap cell edges, drop the slivers that snapping collapses, re-join splits.
fn normalise_boxes(raw: &[BBox], chars: &[Char], fills: &[Fill], rules: &[Rule]) -> Vec<BBox> {
    if raw.is_empty() {
        return Vec::new();
    }
    let xs = snap(raw.iter().flat_map(|bbox| [bbox[0], bbox[2]]).collect(), X_EDGE_TOLERANCE);
    let ys = snap(raw.iter().flat_map(|bbox| [bbox[1], bbox[3]]).collect(), Y_EDGE_TOLERANCE);
    let mut boxes: Vec<BBox> = Vec::new();
    let mut seen: HashSet<[u64; 4]> = HashSet::new();
    for bbox in raw {
        let moved: BBox = [
            snapped(bbox[0], &xs, X_EDGE_TOLERANCE),
            snapped(bbox[1], &ys, Y_EDGE_TOLERANCE),
            snapped(bbox[2], &xs, X_EDGE_TOLERANCE),
            snapped(bbox[3], &ys, Y_EDGE_TOLERANCE),
        ];
        if moved[2] - moved[0] < FILL_MIN || moved[3] - moved[1] < FILL_MIN {
            continue;
        }
        if seen.insert(moved.map(|value| (round2(value) + 0.0).to_bits())) {
            boxes.push(moved);
        }
    }
    merge_fragments(boxes, chars, fills, rules)
}

/// The timetable table(s), decorations and nested rectangles removed.
///
/// A page normally has exactly one; any table large enough and not nested
/// inside another one is kept, top first.
fn main_tables(tables: Vec<Table>, page: u32) -> Result<Vec<Table>, ExtractError> {
    if tables.is_empty() {
        return Err(ExtractError::new("NO_TABLE", "aucun tableau trouvé sur cette page").on_page(page));
    }
    let largest = tables.iter().map(|table| area(&table.bbox)).fold(0.0, f64::max);
    let big: Vec<Table> =
        tables.into_iter().filter(|table| area(&table.bbox) >= MIN_TABLE_AREA_RATIO * largest).collect();
    let mut kept: Vec<Table> = (0..big.len())
        .filter(|&index| {
            !(0..big.len()).any(|other| other != index && contains(&big[other].bbox, &big[index].bbox, 1.0))
        })
        .map(|index| big[index].clone())
        .collect();
    kept.sort_by(|a, b| a.bbox[1].total_cmp(&b.bbox[1]));
    Ok(kept)
}

/// Split the page rectangles into thin rules and background fills.
fn classify_rects(rects: &[Rect]) -> (Vec<Rule>, Vec<Fill>) {
    let mut rules = Vec::new();
    let mut fills = Vec::new();
    for rect in rects {
        let bbox: BBox = [rect.x0, rect.top, rect.x1, rect.bottom];
        let (width, height) = (bbox[2] - bbox[0], bbox[3] - bbox[1]);
        if width.min(height) < THIN_MAX {
            if width.max(height) >= THIN_MIN_LENGTH {
                rules.push(Rule { bbox, color: rect.color });
            }
        } else if width >= FILL_MIN && height >= FILL_MIN {
            if let Some(color) = rect.color {
                fills.push((bbox, color));
            }
        }
    }
    (rules, fills)
}

fn cluster_by_top<'a>(chars: &[&'a Char]) -> Vec<Vec<&'a Char>> {
    let mut sorted: Vec<&Char> = chars.to_vec();
    sorted.sort_by(|a, b| a.top.total_cmp(&b.top).then(a.x0.total_cmp(&b.x0)));
    let mut clusters: Vec<Vec<&Char>> = Vec::new();
    for c in sorted {
        match clusters.last_mut() {
            Some(cluster) if c.top - cluster[0].top <= LINE_TOLERANCE => cluster.push(c),
            _ => clusters.push(vec![c]),
        }
    }
    clusters
}

/// True when `c` sits higher or lower than every glyph it touches.
fn off_baseline(c: &Char, glyphs: &[&Char]) -> bool {
    let mut touching = glyphs.iter().filter(|glyph| glyph.x0 < c.x1 && c.x0 < glyph.x1).peekable();
    touching.peek().is_some() && touching.all(|glyph| (glyph.top - c.top).abs() > STRAY_SPACE_SHIFT)
}

/// Remove blank glyphs printed on top of a real one.
///
/// Word sometimes emits a stray space over a letter; the text extractor then
/// breaks the word in two ("la j ournée"). A genuine space never overlaps the
/// glyph next to it by more than a rounding error.
fn drop_stray_spaces(chars: Vec<&Char>) -> Vec<&Char> {
    let glyphs: Vec<&Char> = chars.iter().copied().filter(|c| !c.is_space()).collect();
    chars
        .into_iter()
        .filter(|c| {
            let width = c.x1 - c.x0;
            if !c.is_space() || width <= 0.0 {
                return true;
            }
            let overlaps: Vec<f64> =
                glyphs.iter().map(|glyph| (c.x1.min(glyph.x1) - c.x0.max(glyph.x0)).max(0.0)).collect();
            if overlaps.iter().copied().fold(0.0, f64::max) / width > STRAY_SPACE_OVERLAP {
                return false;
            }
            // Also astride two letters ("confér ence"): half on each, so neither
            // covers it alone. Off the line's baseline too, which a condensed but
            // genuine space never is.
            !(overlaps.iter().sum::<f64>() / width > STRAY_SPACE_OVERLAP && off_baseline(c, &glyphs))
        })
        .collect()
}

fn strip(text: &str) -> &str {
    text.trim_matches(py_isspace)
}

/// The colour most of the characters have; the first one met wins a tie.
fn commonest(colors: &[Rgb]) -> Option<Rgb> {
    let mut counts: Vec<([u64; 3], Rgb, usize)> = Vec::new();
    for color in colors {
        let key = color.map(f64::to_bits);
        match counts.iter_mut().find(|entry| entry.0 == key) {
            Some(entry) => entry.2 += 1,
            None => counts.push((key, *color, 1)),
        }
    }
    let mut best: Option<(Rgb, usize)> = None;
    for (_, color, count) in counts {
        if best.is_none_or(|(_, most)| count > most) {
            best = Some((color, count));
        }
    }
    best.map(|(color, _)| color)
}

/// Cluster a cell's characters into lines, keeping font, size and colour.
fn text_lines(chars: &[&Char]) -> Vec<TextLine> {
    let mut lines = Vec::new();
    for cluster in cluster_by_top(chars) {
        let cluster = drop_stray_spaces(cluster);
        let text = extract_text(&cluster).replace('\n', " ");
        let text = strip(&text);
        if text.is_empty() {
            continue;
        }
        // What the line looks like is read off its visible glyphs alone. A blank
        // has no ink, and PDFium does not report most of them.
        let glyphs: Vec<&Char> = cluster.iter().copied().filter(|c| !c.is_space()).collect();
        let colors: Vec<Rgb> = glyphs.iter().filter_map(|c| c.color).collect();
        let bold = glyphs.iter().filter(|c| c.fontname.to_lowercase().contains("bold")).count() * 2 > glyphs.len();
        let fold = |pick: fn(&Char) -> f64, start: f64, combine: fn(f64, f64) -> f64| {
            glyphs.iter().map(|c| pick(c)).fold(start, combine)
        };
        lines.push(TextLine {
            text: text.to_string(),
            bbox: [
                fold(|c| c.x0, f64::INFINITY, f64::min),
                fold(|c| c.top, f64::INFINITY, f64::min),
                fold(|c| c.x1, f64::NEG_INFINITY, f64::max),
                fold(|c| c.bottom, f64::NEG_INFINITY, f64::max),
            ],
            bold,
            size: fold(|c| c.size, f64::NEG_INFINITY, f64::max),
            color: commonest(&colors),
        });
    }
    lines
}

/// The smallest background rectangle covering the cell centre.
fn fill_of(bbox: &BBox, fills: &[Fill]) -> Option<Rgb> {
    let (x, y) = center(bbox);
    let mut best: Option<&Fill> = None;
    for fill in fills {
        let rect = &fill.0;
        if rect[0] <= x
            && x <= rect[2]
            && rect[1] <= y
            && y <= rect[3]
            && best.is_none_or(|current| area(rect) < area(&current.0))
        {
            best = Some(fill);
        }
    }
    best.map(|fill| fill.1)
}

/// A non-link rule sitting just under a text line marks a remote course.
fn is_underlined(lines: &[TextLine], bbox: &BBox, candidates: &[&Rule]) -> bool {
    candidates.iter().filter(|rule| contains(bbox, &rule.bbox, 2.0)).any(|rule| {
        lines.iter().any(|line| {
            let gap = rule.bbox[1] - line.bbox[3];
            (0.0..=UNDERLINE_GAP).contains(&gap)
                && rule.bbox[2].min(line.bbox[2]) - rule.bbox[0].max(line.bbox[0]) > 0.0
        })
    })
}

fn extract_page(raw: &RawPage) -> Result<Page, ExtractError> {
    let shapes = classify(&raw.paths, raw.height);
    if raw.chars.len() + shapes.count + raw.other_objects > MAX_PAGE_OBJECTS {
        return Err(ExtractError::new("PDF_TOO_COMPLEX", "La page contient trop d'éléments graphiques ou de texte.")
            .on_page(raw.number));
    }
    if raw.chars.is_empty() {
        return Err(
            ExtractError::new("NO_TEXT_LAYER", "cette page ne contient aucun texte (PDF scanné ?)").on_page(raw.number)
        );
    }

    let tables = main_tables(find_tables(&shapes.edges), raw.number)?;
    let (rules, fills) = classify_rects(&shapes.rects);
    let underline_rules: Vec<&Rule> =
        rules.iter().filter(|rule| !rule.is_border() && !rule.is_link_underline()).collect();
    let links: Vec<Link> = raw
        .links
        .iter()
        .map(|link| {
            let [left, bottom, right, top] = link.rect;
            Link {
                url: link.url.clone(),
                bbox: [left.min(right), raw.height - top.max(bottom), left.max(right), raw.height - top.min(bottom)],
                page: raw.number,
            }
        })
        .filter(|link| area(&link.bbox) > 0.0)
        .collect();

    let centers: Vec<(f64, f64)> = raw.chars.iter().map(|c| ((c.x0 + c.x1) / 2.0, (c.top + c.bottom) / 2.0)).collect();
    let mut columns: BTreeMap<(u64, u64), Vec<usize>> = BTreeMap::new();
    let cell_boxes: Vec<BBox> = tables.iter().flat_map(|table| table.cells.iter().copied()).collect();

    let mut cells = Vec::new();
    for bbox in normalise_boxes(&cell_boxes, &raw.chars, &fills, &rules) {
        // Cells share column spans. Keep original character order and inclusive
        // centre boundaries, including merged cells and glyphs on an edge.
        let column = columns.entry((bbox[0].to_bits(), bbox[2].to_bits())).or_insert_with(|| {
            (0..raw.chars.len()).filter(|&index| bbox[0] <= centers[index].0 && centers[index].0 <= bbox[2]).collect()
        });
        let inside: Vec<&Char> = column
            .iter()
            .filter(|&&index| bbox[1] <= centers[index].1 && centers[index].1 <= bbox[3])
            .map(|&index| &raw.chars[index])
            .collect();
        let lines = text_lines(&inside);
        cells.push(Cell {
            bbox,
            page: raw.number,
            text: lines.iter().map(|line| line.text.as_str()).collect::<Vec<_>>().join("\n"),
            fill: fill_of(&bbox, &fills),
            underlined: is_underlined(&lines, &bbox, &underline_rules),
            links: links.iter().filter(|link| overlaps(&bbox, &link.bbox)).cloned().collect(),
            lines,
        });
    }

    Ok(Page {
        number: raw.number,
        width: raw.width,
        height: raw.height,
        cells,
        links,
        table_bbox: tables.first().map(|table| table.bbox),
        table_count: tables.len(),
    })
}

/// Turn what was read from the PDF into the document later stages work on.
pub fn build(raw: &RawDocument, file: &str, sha256: String) -> Result<Document, ExtractError> {
    let mut metadata = BTreeMap::new();
    if let Some(value) = &raw.creation_date {
        metadata.insert("CreationDate".to_string(), value.clone());
    }
    if let Some(value) = &raw.mod_date {
        metadata.insert("ModDate".to_string(), value.clone());
    }
    Ok(Document {
        file: file.to_string(),
        sha256,
        pages: raw.pages.iter().map(extract_page).collect::<Result<_, _>>()?,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(text: char, x0: f64, x1: f64, top: f64) -> Char {
        Char {
            text,
            x0,
            x1,
            top,
            bottom: top + 9.0,
            size: 9.0,
            fontname: "Marianne-Regular".into(),
            color: Some([0.0; 3]),
        }
    }

    #[test]
    fn round2_rounds_like_python() {
        // round(2.675, 2) is 2.67 in Python: the float sits just under the tie.
        assert_eq!(round2(2.675), 2.67);
        assert_eq!(round2(113.395_000_1), 113.4);
        assert_eq!(round2(-0.004), 0.0);
    }

    #[test]
    fn snap_merges_phantom_edges_only() {
        let edges = snap(vec![100.0, 103.5, 140.0, 140.0, 190.0], X_EDGE_TOLERANCE);
        assert_eq!(edges, vec![101.75, 140.0, 190.0]);
        assert_eq!(snapped(102.0, &edges, X_EDGE_TOLERANCE), 101.75);
        assert_eq!(snapped(120.0, &edges, X_EDGE_TOLERANCE), 120.0);
    }

    #[test]
    fn a_blank_printed_over_a_letter_is_dropped() {
        let chars = [glyph('j', 10.0, 13.0, 50.0), glyph(' ', 10.2, 12.8, 50.0), glyph('o', 13.0, 18.0, 50.0)];
        let kept = drop_stray_spaces(chars.iter().collect());
        assert_eq!(kept.iter().map(|c| c.text).collect::<String>(), "jo");
    }

    #[test]
    fn a_genuine_space_is_kept_and_splits_the_line_into_words() {
        let chars = [
            glyph('l', 10.0, 12.0, 50.0),
            glyph('a', 12.0, 17.0, 50.0),
            glyph(' ', 17.0, 19.4, 50.0),
            glyph('j', 19.4, 22.0, 50.0),
        ];
        let lines = text_lines(&chars.iter().collect::<Vec<_>>());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "la j");
        assert_eq!(lines[0].bbox, [10.0, 50.0, 22.0, 59.0]);
    }

    #[test]
    fn a_line_is_measured_on_its_glyphs_not_its_blanks() {
        let mut blank = glyph(' ', 22.0, 30.0, 47.0);
        blank.size = 12.0;
        blank.bottom = 61.0;
        let chars = [glyph('a', 10.0, 15.0, 50.0), glyph('b', 15.0, 22.0, 50.0), blank];
        let lines = text_lines(&chars.iter().collect::<Vec<_>>());
        assert_eq!(lines[0].bbox, [10.0, 50.0, 22.0, 59.0]);
        assert_eq!(lines[0].size, 9.0);
    }

    #[test]
    fn rectangles_are_rules_fills_or_noise() {
        let rect = |x0: f64, top: f64, x1: f64, bottom: f64| Rect {
            x0,
            x1,
            top,
            bottom,
            width: x1 - x0,
            height: bottom - top,
            color: Some(WHITE),
        };
        let (rules, fills) = classify_rects(&[
            rect(10.0, 10.0, 200.0, 10.48), // a border
            rect(10.0, 10.0, 12.0, 10.48),  // too short to be one
            rect(10.0, 20.0, 200.0, 60.0),  // a background
        ]);
        assert_eq!((rules.len(), fills.len()), (1, 1));
        assert!(rules[0].is_border());
    }

    #[test]
    fn a_row_boundary_crossed_by_text_is_not_one() {
        let boxes = vec![[10.0, 10.0, 100.0, 30.0], [10.0, 30.0, 100.0, 50.0], [10.0, 50.0, 100.0, 70.0]];
        // One glyph sits astride y = 30; nothing crosses y = 50.
        let chars = [glyph('a', 20.0, 25.0, 25.0)];
        let merged = merge_fragments(boxes, &chars, &[], &[]);
        assert_eq!(merged, vec![[10.0, 10.0, 100.0, 50.0], [10.0, 50.0, 100.0, 70.0]]);
    }
}
