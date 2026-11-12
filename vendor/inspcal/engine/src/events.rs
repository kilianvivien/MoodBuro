//! Grid to events: `src/edt_parser/events.py`, function for function.
//!
//! Each non-empty `Thème` cell anchors a session. A shared-cohort session with
//! separate sequential time slots becomes one event per slot; alternative
//! group slots stay together. Other fields come from cells overlapping the
//! anchor's vertical interval, including cells merged over several slots.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{json, Map, Value};

use crate::dates;
use crate::errors::EdtError;
use crate::grid::{fold, Grid};
use crate::institution::INSTITUTION;
use crate::model::{Cell, Document, Link as Annotation, TextLine};
use crate::py::civil::{Date, Time};
use crate::py::text;
use crate::rooms;
use crate::timetable::{Event, Groups, Link};

/// How far apart two edges may be and still be the same edge (`pdf_extract`).
const X_EDGE_TOLERANCE: f64 = 4.0;

/// A vertical gap wider than this share of the line height starts a paragraph.
/// Wrapped lines sit ~2 pt apart, a new paragraph ~12 pt.
const PARAGRAPH_GAP_RATIO: f64 = 0.5;

/// Between the speakers of one group in `group_details`.
pub const SPEAKER_SEPARATOR: &str = ", ";

/// Between the rooms of one session held in several at once.
const ROOM_SEPARATOR: &str = " / ";

/// Shown instead of a room when each group has its own.
const GROUP_LOCATION_NOTE: &str = "Selon le groupe";

fn tokens() -> String {
    INSTITUTION.cohort_tokens.iter().map(|token| regex::escape(token)).collect::<Vec<_>>().join("|")
}

/// `FI-CIL-OFF 2026-2028` → tokens {FI, CIL, OFF} + promotion 2026-2028.
static COHORT_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| text::regex(&format!(r"\b({})\b", tokens())));
static PROMOTION_RE: LazyLock<Regex> =
    LazyLock::new(|| text::regex(r"\b(?P<start>20\d{2})\s*[-–]\s*(?P<end>20\d{2})\b"));

const GROUP: &str = r"(?i)\bGroupes?\s*(?P<first>\d+)(?:\s*[-–à]\s*(?P<last>\d+))?";
static GROUP_RE: LazyLock<Regex> = LazyLock::new(|| text::regex(GROUP));
static GROUP_FULL_RE: LazyLock<Regex> = LazyLock::new(|| text::full(GROUP));
static LETTER_GROUP_RE: LazyLock<Regex> =
    LazyLock::new(|| text::full(r"(?i)\bGroupes?\s*(?P<first>[A-Z])(?:\s*[-–à]\s*(?P<last>[A-Z]))?\b"));
static PAIRED_GROUP_RE: LazyLock<Regex> = LazyLock::new(|| text::full(r"(?i)Groupes\s+\d+\s+et\s+\d+"));
/// `Groupe B3`: a numbered group within a demi-promo (from 9 November 2026).
static HALF_GROUP_RE: LazyLock<Regex> = LazyLock::new(|| text::full(r"(?i)Groupe\s*[A-Z]\s*\d+"));
/// `1/2 Promo A - Groupe 1`: the same, spelt out (16 November 2026).
static HALF_NUMBERED_GROUP_RE: LazyLock<Regex> =
    LazyLock::new(|| text::full(r"(?i)(?:demi[- ]promo(?:tion)?|½\s*promo|1/2\s*promo)\s+[AB]\s*[-–]\s*Groupe\s*\d+"));

/// A title may carry a cohort prefix (`FI-CIL-OFF : Parcours …`) to strip.
static COHORT_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| text::prefix(&format!(r"\s*(?:{0})(?:\s*[-/]\s*(?:{0}))*\s*:\s*", tokens())));

/// How a speaker entry starts. The full stop is optional throughout: the
/// school writes both `Dr DURAND` and `Dr. DURAND`. Python ends this pattern
/// on a look-ahead; here the title is the first group and what follows it is
/// matched without being part of it.
static SPEAKER_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    text::regex(
        r"^((?:MM|Mmes|Mme|Mlle|Mgr|M|Prof|Dr|Pr|Me|Mademoiselle|Messieurs|Mesdames|Monsieur|Madame|Générale|Général|Commissaire|Commandant|Colonel|Amiral|Préfète|Préfet|Maître)\.?)(?:\s|$)",
    )
});

/// A session running over several days: `Du 28/09 au 02/10` in one cell
/// merged across the Date, Début and Fin columns.
static DATE_RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    text::full(&format!(
        r"(?i)Du\s+(?:{0}+\.?\s+)?(?P<first>\d{{1,2}}(?:er)?(?:\s*/\s*\d{{1,2}}(?:\s*/\s*\d{{2,4}})?)?)\s+au\s+(?:{0}+\.?\s+)?(?P<last>\d{{1,2}}\s*/\s*\d{{1,2}}(?:\s*/\s*\d{{2,4}})?)",
        text::LETTER
    ))
});

/// A bare room code: `C301`, `C301-C302`, `C303/C302`.
static ROOM_CODE_RE: LazyLock<Regex> = LazyLock::new(|| text::full(r"[A-Z]\d{3}(?:\s*[-/]\s*[A-Z]?\d{3})*"));

/// Sport. Several slots are printed each week and only one is compulsory.
static SPORT_RE: LazyLock<Regex> =
    LazyLock::new(|| text::regex(&format!(r"(?i)\b{}\b", regex::escape(&INSTITUTION.sport_keyword))));
// The fitness diagnostic prints its groups in the title, on separate days,
// rather than naming the slots APSA. They require the same single choice.
static FITNESS_RE: LazyLock<Regex> =
    LazyLock::new(|| text::full(r"(?i)Diagnostic\s+(?:de\s+)?condition\s+physique\s+Groupe\s+\d+"));

/// A start with no end: `A partir de 18h00`, `À partir de 18h`, `Dès 17h45`.
const OPEN_START: &str = r"(?i)(?:[AÀaà] partir de|[Dd][èe]s)\s+\d{1,2}\s*[hH:]\s*(?:\d{2})?";
static OPEN_START_FULL_RE: LazyLock<Regex> = LazyLock::new(|| text::full(OPEN_START));
static OPEN_START_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| text::prefix(OPEN_START));

/// A merged Début/Fin cell naming the whole day: the day is known, the hours
/// are not, so it stays an all-day reminder like any session with no hour.
static WHOLE_DAY_RE: LazyLock<Regex> =
    LazyLock::new(|| text::full(r"(?i)(?:toute la |la )?journ[ée]e(?: enti[èe]re| complète)?"));

/// A merged Début/Fin cell carrying both ends: `14h00 - 16h00`, `De 9h à 12h`.
static MERGED_RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    text::full(
        r"(?i)(?:de\s+)?(?P<start>\d{1,2}\s*[hH:]\s*(?:\d{2})?)\s*(?:[-–—/]|à|a)\s*(?P<end>\d{1,2}\s*[hH:]\s*(?:\d{2})?)",
    )
});

static AUTONOMOUS_RE: LazyLock<Regex> =
    LazyLock::new(|| text::full(r"((?:FI|CIL|OFF)(?:-(?:FI|CIL|OFF))*)\s+Travail en autonomie\s+(.+)"));

const HALF_PROMOTION: &str = r"(?:demi[- ]promo(?:tion)?|½\s*promo|1/2\s*promo)\s+([AB])";

/// An explicit half-promotion heading, never inferred from group numbers.
///
/// The heading may name the cohorts it concerns first (`CIL Demi-promo A`,
/// `FI-OFF 2026-2028 Demi-promo B`, 16–20 November 2026).
pub fn half_promotion_label(value: &str) -> Option<String> {
    static HALF: LazyLock<Regex> = LazyLock::new(|| {
        text::full(&format!(
            r"(?i)(?:(?:{0})(?:\s*-\s*(?:{0}))*(?:\s+20\d{{2}}\s*[-–]\s*(?:20)?\d{{2}})?\s+)?{HALF_PROMOTION}",
            tokens()
        ))
    });
    HALF.captures(&clean_text(value)).map(|found| found[1].to_uppercase())
}

/// `½ promo B - FI-CIL-OFF : Note d'analyse collective`: the half heading and
/// the title share one paragraph. `(heading, rest)`.
fn leading_half_promotion(value: &str) -> Option<(String, String)> {
    static LEAD: LazyLock<Regex> =
        LazyLock::new(|| text::full(&format!(r"(?i)(?P<heading>{HALF_PROMOTION})\s*[-–—:]\s*(?P<rest>.+)")));
    let cleaned = clean_text(value);
    let found = LEAD.captures(&cleaned)?;
    Some((found["heading"].to_owned(), found["rest"].to_owned()))
}

/// The demi-promo heading a Thème cell opens with, if any.
fn half_promotion_heading(theme: &Cell) -> Option<String> {
    let first = paragraphs(theme).into_iter().next()?;
    if half_promotion_label(&first).is_some() {
        return Some(first);
    }
    leading_half_promotion(&first).map(|(heading, _)| heading)
}

/// True for one of the week's interchangeable sport sessions.
pub fn is_sport(event: &Event) -> bool {
    SPORT_RE.is_match(&event.title) || FITNESS_RE.is_match(&event.title)
}

/// A public holiday row (`Férié`, `Jour férié`, 11 November 2026): a whole
/// day off for everyone, kept as an all-day marker with no hours to ask for.
pub fn is_holiday(event: &Event) -> bool {
    event.start.is_none() && event.end.is_none() && matches!(fold(&event.title).as_str(), "ferie" | "jour ferie")
}

/// True for a session reserved to students who registered (`Elèves inscrits`):
/// optional, so it is exported as unconfirmed until the student opts in.
pub fn is_registration_only(event: &Event) -> bool {
    event.cohort_label.as_deref().is_some_and(|label| fold(label).contains("inscrit"))
}

/// `(tokens, promotion)` for a `Cohorte` cell.
///
/// Union semantics: `FI-CIL-OFF` concerns FI, CIL and OFF students alike. An
/// unrecognised cell yields no token: the event is kept and warned about.
pub fn parse_cohorts(value: &str) -> (Vec<String>, Option<String>) {
    let mut tokens: Vec<String> = Vec::new();
    for found in COHORT_TOKEN_RE.find_iter(value) {
        if !tokens.iter().any(|token| token == found.as_str()) {
            tokens.push(found.as_str().to_owned());
        }
    }
    let promotion = PROMOTION_RE.captures(value).map(|found| format!("{}-{}", &found["start"], &found["end"]));
    (tokens, promotion)
}

/// Count numbered or lettered groups, including explicit ranges.
pub fn parse_group_count(value: &str) -> Option<i64> {
    if let Some(found) = GROUP_RE.captures(value) {
        return Some(match found.name("last") {
            None => 1,
            Some(last) => text::int(last.as_str()) - text::int(&found["first"]) + 1,
        });
    }
    let cleaned = clean_text(value);
    let found = LETTER_GROUP_RE.captures(&cleaned)?;
    let letter = |value: &str| value.to_uppercase().chars().next().map_or(0, |c| i64::from(u32::from(c)));
    Some(match found.name("last") {
        None => 1,
        Some(last) => letter(last.as_str()) - letter(&found["first"]) + 1,
    })
}

/// Re-join a multi-line cell: de-hyphenate names, normalise whitespace.
///
/// A compound word broken across two lines (`SIMON-\nMICHEL`, `outre-\nmer`)
/// loses the line break and keeps the hyphen: Word only wraps at a real one.
pub fn clean_text(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut joined = String::with_capacity(value.len());
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        let breaks = c == '-'
            && chars.get(index + 1) == Some(&'\n')
            && index > 0
            && text::is_letter(chars[index - 1])
            && chars.get(index + 2).is_some_and(|&next| text::is_letter(next));
        joined.push(match c {
            '’' | '‘' => '\'',
            other => other,
        });
        index += if breaks { 2 } else { 1 };
    }
    text::single_spaced(&joined)
}

/// Drop a leading `FI-CIL-OFF : ` from a title.
fn strip_cohort_prefix(title: &str) -> &str {
    let end = COHORT_PREFIX_RE.find(title).map_or(0, |found| found.end());
    text::strip(&title[end..])
}

/// `MICHEL`, `DURAND` — the tail of a name wrapped onto the next line.
fn is_surname_fragment(value: &str) -> bool {
    value == value.to_uppercase() && text::has_alpha(value)
}

/// A civility plus at least one upper-case surname reads as a whole name.
fn looks_complete(entry: &str) -> bool {
    // Past the civility: `M.` is upper-case too, and is no surname.
    SPEAKER_TITLE_RE
        .captures(entry)
        .is_some_and(|title| text::split(&entry[title[1].len()..]).into_iter().any(is_surname_fragment))
}

/// A trailing hyphen means the surname was split mid-word (`SIMON-MICHEL`).
fn append(left: &str, right: &str) -> String {
    if left.ends_with('-') {
        format!("{left}{right}")
    } else {
        format!("{left} {right}")
    }
}

/// Turn the `Intervenant(s)` lines into one entry per speaker.
///
/// Word writes one speaker per line, but a long name wraps — sometimes into
/// the next sub-cell. An upper-case line continues the name above it; a line
/// opening with a civility starts a new one; anything else continues the
/// entry above unless that entry already reads as a complete name.
pub fn parse_speakers<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();
    for raw in lines {
        let line = text::squeeze(raw);
        if line.is_empty() {
            continue;
        }
        let continues = match entries.last() {
            None => false,
            Some(_) if is_surname_fragment(&line) => true,
            Some(_) if SPEAKER_TITLE_RE.is_match(&line) => false,
            Some(last) => !looks_complete(last),
        };
        match entries.last_mut() {
            Some(last) if continues => *last = append(last, &line),
            _ => entries.push(line),
        }
    }
    let mut unique: Vec<String> = Vec::new();
    for entry in entries {
        let cleaned = clean_text(&entry);
        if !cleaned.is_empty() && !unique.contains(&cleaned) {
            unique.push(cleaned);
        }
    }
    unique
}

fn joined(lines: &[&TextLine]) -> String {
    clean_text(&lines.iter().map(|line| line.text.as_str()).collect::<Vec<_>>().join("\n"))
}

/// Split a cell's lines into paragraphs on the vertical gaps between them.
fn paragraphs(cell: &Cell) -> Vec<String> {
    let mut blocks: Vec<Vec<&TextLine>> = Vec::new();
    for line in &cell.lines {
        let height = line.bbox[3] - line.bbox[1];
        match blocks.last_mut() {
            Some(block) if line.bbox[1] - block[block.len() - 1].bbox[3] <= PARAGRAPH_GAP_RATIO * height => {
                block.push(line)
            }
            _ => blocks.push(vec![line]),
        }
    }
    blocks.iter().map(|block| joined(block)).collect()
}

/// True when the link rectangle sits on this text line.
fn covers(line: &TextLine, link: &Annotation) -> bool {
    line.bbox[3].min(link.bbox[3]) - line.bbox[1].max(link.bbox[1]) > 1.0
        && line.bbox[2].min(link.bbox[2]) - line.bbox[0].max(link.bbox[0]) > 1.0
}

/// Red text marks an instruction to the students.
fn is_red(line: &TextLine) -> bool {
    line.color.is_some_and(|[red, green, blue]| red > 0.6 && green < 0.3 && blue < 0.3)
}

fn is_empty(cell: &Cell) -> bool {
    text::strip(&cell.text).is_empty()
}

/// True when this cell shares vertical space with `[top, bottom]`.
fn overlaps_rows(cell: &Cell, top: f64, bottom: f64) -> bool {
    cell.bbox[3].min(bottom) - cell.bbox[1].max(top) > 1.0
}

fn fill_hex(cell: &Cell) -> Option<String> {
    cell.fill.map(|fill| {
        let channel = |value: f64| (value * 255.0).round_ties_even().clamp(0.0, 255.0) as u8;
        format!("#{:02X}{:02X}{:02X}", channel(fill[0]), channel(fill[1]), channel(fill[2]))
    })
}

/// One `Link` per URL and column.
///
/// Word emits one annotation per text line, so a link spanning three lines
/// arrives in three pieces whose rectangles overlap each other. Gathering the
/// lines they cover — rather than labelling each piece — avoids repeating
/// words in the label.
fn collect_links(sources: &[(&str, &Cell)]) -> Vec<Link> {
    let mut grouped: Vec<((&str, &str), Vec<&TextLine>)> = Vec::new();
    for &(column, cell) in sources {
        for annotation in &cell.links {
            let key = (annotation.url.as_str(), column);
            let position = grouped.iter().position(|(known, _)| *known == key).unwrap_or_else(|| {
                grouped.push((key, Vec::new()));
                grouped.len() - 1
            });
            let covered = &mut grouped[position].1;
            for line in &cell.lines {
                if covers(line, annotation) && !covered.contains(&line) {
                    covered.push(line);
                }
            }
        }
    }
    grouped
        .into_iter()
        .map(|((url, column), mut lines)| {
            lines.sort_by(|left, right| {
                left.bbox[1].total_cmp(&right.bbox[1]).then(left.bbox[0].total_cmp(&right.bbox[0]))
            });
            let mut label = String::new();
            for line in lines {
                label = if label.is_empty() { line.text.clone() } else { append(&label, &line.text) };
            }
            let label = clean_text(&label);
            Link { url: url.to_owned(), label: (!label.is_empty()).then_some(label), column: Some(column.to_owned()) }
        })
        .collect()
}

/// The cells that describe one `Thème` cell, grouped by column.
struct Anchor<'g> {
    grid: &'g Grid,
    /// The Thème cell. For a sequential slot this is a copy cut down to the
    /// slot's rows, which is why it is held by value.
    theme: Cell,
    /// Cells facing the theme, as positions in `grid.cells`.
    body: Vec<usize>,
    /// The per-group titles when `theme` heads a block of workshops.
    workshops: Vec<usize>,
}

impl<'g> Anchor<'g> {
    /// `theme_index` is where `theme` sits in the grid, or `None` for a copy:
    /// the original then counts as one more cell facing it, as in Python.
    fn new(grid: &'g Grid, theme: Cell, theme_index: Option<usize>, workshops: Vec<usize>) -> Self {
        let body = (0..grid.cells.len())
            .filter(|&index| {
                let cell = &grid.cells[index];
                cell.page == theme.page
                    && Some(index) != theme_index
                    && overlaps_rows(cell, theme.bbox[1], theme.bbox[3])
            })
            .collect();
        Self { grid, theme, body, workshops }
    }

    fn cell(&self, index: usize) -> &'g Cell {
        &self.grid.cells[index]
    }

    /// Non-empty cells belonging to `role` alone, top to bottom.
    fn own(&self, role: &str) -> Vec<&'g Cell> {
        let Some(column) = self.grid.column(role) else { return Vec::new() };
        let mut cells: Vec<&Cell> = self
            .body
            .iter()
            .filter(|&&index| {
                column.covers(self.cell(index)) && self.grid.roles(index).len() == 1 && !is_empty(self.cell(index))
            })
            .map(|&index| self.cell(index))
            .collect();
        cells.sort_by(|left, right| left.bbox[1].total_cmp(&right.bbox[1]));
        cells
    }

    fn own_texts(&self, role: &str) -> Vec<String> {
        self.own(role).into_iter().map(|cell| clean_text(&cell.text)).collect()
    }

    /// The merged `Du JJ/MM au JJ/MM` cell standing in for Date and times.
    fn date_range_cell(&self) -> Option<usize> {
        self.body.iter().copied().find(|&index| {
            let roles = self.grid.roles(index);
            roles.len() > 1
                && roles.iter().any(|role| role == "date")
                && !roles.iter().any(|role| role == "theme")
                && DATE_RANGE_RE.is_match(&clean_text(&self.cell(index).text))
        })
    }

    /// Cells spanning several columns without covering Thème.
    fn notes(&self) -> Vec<&'g Cell> {
        let range = self.date_range_cell();
        self.body
            .iter()
            .filter(|&&index| {
                let roles = self.grid.roles(index);
                !is_empty(self.cell(index))
                    && roles.len() > 1
                    && !roles.iter().any(|role| role == "theme")
                    && Some(index) != range
            })
            .map(|&index| self.cell(index))
            .collect()
    }

    /// `(header label, cell)` for columns the header did not recognise: what
    /// such a column says still reaches the reader, as a note.
    fn extras(&self) -> Vec<(&'g str, &'g Cell)> {
        let mut found = Vec::new();
        for column in &self.grid.extra_columns {
            for &index in &self.body {
                let cell = self.cell(index);
                if !is_empty(cell) && column.covers(cell) && self.grid.roles(index) == [column.role.as_str()] {
                    found.push((column.label.as_str(), cell));
                }
            }
        }
        found.sort_by(|left, right| {
            left.1.bbox[1].total_cmp(&right.1.bbox[1]).then(left.1.bbox[0].total_cmp(&right.1.bbox[0]))
        });
        found
    }

    /// Cells merged across Début and Fin, and nothing else.
    fn merged_hours(&self) -> impl Iterator<Item = &'g Cell> + '_ {
        self.body.iter().filter(|&&index| self.grid.roles(index) == ["start", "end"]).map(|&index| self.cell(index))
    }
}

/// Both hours printed in one cell merged across Début and Fin.
fn merged_range(anchor: &Anchor) -> (Vec<String>, Vec<String>) {
    for cell in anchor.merged_hours() {
        if let Some(found) = MERGED_RANGE_RE.captures(&clean_text(&cell.text)) {
            return (vec![found["start"].to_owned()], vec![found["end"].to_owned()]);
        }
    }
    (Vec::new(), Vec::new())
}

fn times(anchor: &Anchor) -> Result<(Vec<String>, Vec<String>), EdtError> {
    let mut starts = anchor.own_texts("start");
    let mut ends = anchor.own_texts("end");
    if starts.is_empty() && ends.is_empty() {
        (starts, ends) = merged_range(anchor);
    }
    // Groups starting at staggered times but ending together (or the reverse)
    // share one merged cell on that side: it faces every group.
    if starts.len() == 1 && ends.len() > 1 {
        starts = vec![starts[0].clone(); ends.len()];
    } else if ends.len() == 1 && starts.len() > 1 {
        ends = vec![ends[0].clone(); starts.len()];
    }
    if starts.is_empty() || ends.is_empty() {
        return Err(EdtError::time("aucun horaire de début ou de fin ne fait face à cette séance"));
    }
    if starts.len() != ends.len() {
        return Err(EdtError::time("les horaires de début et de fin ne s'apparient pas"));
    }
    Ok((starts, ends))
}

/// `Matin` / `Après-midi` → the block those words stand for.
///
/// Compared folded, so `Après midi`, `APRES-MIDI` and `Matinée` name the same
/// block as the school's own spelling. Only the whole cell counts.
fn half_day(label: &str) -> Option<(Time, Time)> {
    let mut key = fold(label);
    if key == "matinee" {
        key = "matin".to_owned();
    }
    INSTITUTION.half_day_slots.iter().find(|(name, _, _)| fold(name) == key).map(|&(_, start, end)| (start, end))
}

/// The end given to a session that runs until the end of its day.
const DAY_END: Time = Time { hour: 23, minute: 59 };

/// Sessions printed with a start only (`A partir de 13h00`) that last until
/// the end of the day, as the user asked for 17 November 2026. The end is a
/// default, flagged and editable in review; APSA stays an open reminder.
fn runs_to_day_end(title: &str) -> bool {
    fold(title) == "choix du nom de promotion"
}

/// Recognise deliberately merged time fields, without hiding malformed times.
fn unscheduled_label(anchor: &Anchor) -> Result<Option<String>, EdtError> {
    let roles = anchor.grid.roles_of(&anchor.theme);
    if ["start", "end"].iter().all(|wanted| roles.iter().any(|role| role == wanted)) {
        return Ok(Some("horaires non précisés".to_owned()));
    }
    for cell in anchor.merged_hours() {
        let label = clean_text(&cell.text);
        if OPEN_START_FULL_RE.is_match(&label) {
            dates::parse_time(&label)?;
            return Ok(Some(format!("{label} (fin non précisée)")));
        }
        if half_day(&label).is_some() || WHOLE_DAY_RE.is_match(&label) {
            return Ok(Some(label));
        }
    }
    Ok(None)
}

/// `(first day, last day, label)` for a session printed over several days.
///
/// Only when no ordinary Date cell faces the session: the range then replaces
/// both the day and the times, and the session becomes an all-day reminder
/// spanning the whole range.
fn date_range(anchor: &Anchor, reference: Date) -> Result<Option<(Date, Date, String)>, EdtError> {
    if !anchor.own("date").is_empty() {
        return Ok(None);
    }
    let Some(index) = anchor.date_range_cell() else { return Ok(None) };
    let label = clean_text(&anchor.cell(index).text);
    let found = DATE_RANGE_RE.captures(&label).expect("the cell was chosen for matching");
    let last = dates::parse_day_month(&found["last"], reference, None)?;
    let first = if found["first"].contains('/') {
        dates::parse_day_month(&found["first"], reference, None)?
    } else {
        // `Du 28 au 30/09`: the month is written once, at the end. A first
        // day past the last one is in the month before (`Du 29 au 02/10`).
        let digits = &found["first"];
        let day = text::int(digits.strip_suffix("er").unwrap_or(digits));
        let month_start = Date { day: 1, ..last };
        let base = if day <= i64::from(last.day) { month_start } else { month_start.plus_days(-1) };
        Date::new(i64::from(base.year), i64::from(base.month), day)
            .ok_or_else(|| EdtError::date("date inexistante au calendrier"))?
    };
    if last < first {
        return Err(EdtError::date("la période de la séance se termine avant de commencer"));
    }
    Ok(Some((first, last, label)))
}

/// The one date facing the session.
///
/// `carried` is the day still running at the foot of the previous page: a
/// Date cell merged over several rows that Word breaks across two pages prints
/// its text on the first part only, so the rows at the top of the next page
/// face an empty cell.
fn date_of(
    anchor: &Anchor,
    reference: Date,
    week_start: Option<Date>,
    carried: Option<Date>,
) -> Result<Date, EdtError> {
    let mut texts = anchor.own_texts("date");
    texts.sort();
    texts.dedup();
    if let (true, Some(day)) = (texts.is_empty(), carried) {
        return Ok(day);
    }
    if texts.len() != 1 {
        return Err(EdtError::date("une séance doit faire face à une seule date"));
    }
    dates::parse_day_month(&texts[0], reference, week_start)
}

/// `(title, description, cohort tokens read from a title prefix)`.
///
/// A cell whose first paragraph is a cohort prefix (`FI-CIL-OFF : Parcours
/// transition écologique`) carries the real title in the next paragraph; the
/// prefix becomes description and cohort information.
fn title_and_description(anchor: &Anchor) -> (String, Option<String>, Vec<String>) {
    let blocks: Vec<String> = paragraphs(&anchor.theme).into_iter().filter(|block| !block.is_empty()).collect();
    if blocks.is_empty() {
        return (String::new(), None, Vec::new());
    }
    if let Some(found) = AUTONOMOUS_RE.captures(&clean_text(&anchor.theme.text)) {
        return (found[2].to_owned(), Some("Travail en autonomie".to_owned()), parse_cohorts(&found[1]).0);
    }
    let (title, rest, prefix_tokens) = if let Some((heading, rest)) = leading_half_promotion(&blocks[0]) {
        let title = strip_cohort_prefix(&rest).to_owned();
        let tokens = if title == rest { Vec::new() } else { parse_cohorts(&rest).0 };
        (format!("{title} ({heading})"), blocks[1..].to_vec(), tokens)
    } else if blocks.len() > 1 && half_promotion_label(&blocks[0]).is_some() {
        (format!("{} ({})", blocks[1], blocks[0]), blocks[2..].to_vec(), Vec::new())
    } else if blocks.len() > 1 && COHORT_PREFIX_RE.is_match(&blocks[0]) {
        let mut rest = vec![blocks[0].clone()];
        rest.extend_from_slice(&blocks[2..]);
        (blocks[1].clone(), rest, parse_cohorts(&blocks[0]).0)
    } else {
        let title = strip_cohort_prefix(&blocks[0]).to_owned();
        let tokens = if title == blocks[0] { Vec::new() } else { parse_cohorts(&blocks[0]).0 };
        (title, blocks[1..].to_vec(), tokens)
    };
    let description = rest.join("\n");
    (title, (!description.is_empty()).then_some(description), prefix_tokens)
}

fn dedupe(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut unique: Vec<String> = Vec::new();
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

/// `Groupe 1 … Groupe 9` reads better as `Groupes 1-9`.
fn cohort_label(labels: &[String]) -> Option<String> {
    let unique = dedupe(labels.iter().cloned());
    let numbers: Option<Vec<(i64, i64)>> = unique
        .iter()
        .map(|label| {
            GROUP_FULL_RE.captures(label).map(|found| {
                let first = text::int(&found["first"]);
                (first, found.name("last").map_or(first, |last| text::int(last.as_str())))
            })
        })
        .collect();
    if let Some(numbers) = numbers.filter(|numbers| numbers.len() > 1) {
        if numbers.windows(2).all(|pair| pair[1].0 == pair[0].1 + 1) {
            return Some(format!("Groupes {}-{}", numbers[0].0, numbers[numbers.len() - 1].1));
        }
    }
    let label = unique.join(" / ");
    (!label.is_empty()).then_some(label)
}

fn cohorts(anchor: &Anchor, fallback: &[String]) -> (Vec<String>, Option<String>) {
    // Demi-promo groups sharing a cell for want of a rule are still listed
    // apart. Paired groups keep their frozen label (19–23 October 2026).
    let labels: Vec<String> = anchor
        .own("cohort")
        .into_iter()
        .flat_map(|cell| {
            let rows = group_rows(cell);
            if rows.iter().all(|row| HALF_NUMBERED_GROUP_RE.is_match(&clean_text(&row.text))) {
                rows.iter().map(|row| clean_text(&row.text)).collect()
            } else {
                vec![clean_text(&cell.text)]
            }
        })
        .collect();
    let mut tokens: Vec<String> = Vec::new();
    for label in &labels {
        for token in parse_cohorts(label).0 {
            if !tokens.contains(&token) {
                tokens.push(token);
            }
        }
    }
    if tokens.is_empty() {
        tokens = fallback.to_vec();
    }
    (tokens, cohort_label(&labels))
}

// Some exports omit a horizontal rule between two groups (`Groupes 1 et 2`
// on one line each; `1/2 Promo A -` / `Groupe 1` wrapped over two, 16
// November 2026). Only split when every printed line belongs to a complete
// label; anything else stays intact.
fn group_rows(cell: &Cell) -> Vec<Cell> {
    let complete = |text: &str| {
        let text = clean_text(text);
        PAIRED_GROUP_RE.is_match(&text) || HALF_NUMBERED_GROUP_RE.is_match(&text)
    };
    let mut entries: Vec<Vec<&TextLine>> = Vec::new();
    let mut pending: Vec<&TextLine> = Vec::new();
    for line in &cell.lines {
        pending.push(line);
        let joined = pending.iter().map(|line| line.text.as_str()).collect::<Vec<_>>().join("\n");
        if complete(&joined) {
            entries.push(std::mem::take(&mut pending));
        }
    }
    if entries.len() < 2 || !pending.is_empty() {
        return vec![cell.clone()];
    }
    let mut rows = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let mut row = cell.clone();
        row.text = entry.iter().map(|line| line.text.as_str()).collect::<Vec<_>>().join("\n");
        row.lines = entry.iter().map(|line| (*line).clone()).collect();
        if index > 0 {
            row.bbox[1] = (entries[index - 1].last().unwrap().bbox[3] + entry[0].bbox[1]) / 2.0;
        }
        if let Some(next) = entries.get(index + 1) {
            row.bbox[3] = (entry.last().unwrap().bbox[3] + next[0].bbox[1]) / 2.0;
        }
        rows.push(row);
    }
    rows
}

/// Keep explicit group/room/time associations, including shared cells.
/// Lists cannot be zipped: shared cells and missing dividers need geometry.
fn group_details(anchor: &Anchor) -> Vec<Map<String, Value>> {
    let mut groups = Vec::new();
    let rows = anchor.own("cohort").into_iter().flat_map(|cell| {
        let rows = group_rows(cell);
        let split = rows.len() > 1;
        rows.into_iter().map(move |row| (row, split))
    });
    for (group, split) in rows {
        let label = clean_text(&group.text);
        if ![&*GROUP_FULL_RE, &*LETTER_GROUP_RE, &*PAIRED_GROUP_RE, &*HALF_GROUP_RE, &*HALF_NUMBERED_GROUP_RE]
            .iter()
            .any(|pattern| pattern.is_match(&label))
        {
            continue;
        }
        let faces = |cell: &Cell| cell.bbox[1] <= group.bbox[1] + 2.0 && cell.bbox[3] >= group.bbox[3] - 2.0;
        let mut fields = Map::new();
        for role in ["start", "end", "location", "speakers"] {
            let cells: Vec<&Cell> = anchor.own(role).into_iter().filter(|cell| faces(cell)).collect();
            let [cell] = cells[..] else { continue };
            // A cell shared with the next group only because a divider is
            // missing: the printed lines facing this group are its own.
            let beyond = cell.bbox[1] < group.bbox[1] - 2.0 || cell.bbox[3] > group.bbox[3] + 2.0;
            let facing: Vec<&TextLine> = cell
                .lines
                .iter()
                .filter(|line| line.bbox[1] >= group.bbox[1] && line.bbox[3] <= group.bbox[3])
                .collect();
            if role == "speakers" && split && HALF_NUMBERED_GROUP_RE.is_match(&label) && beyond && cell.lines.len() > 1
            {
                let speakers = parse_speakers(facing.iter().map(|line| line.text.as_str()));
                if !speakers.is_empty() {
                    fields.insert(role.to_owned(), json!(speakers.join(SPEAKER_SEPARATOR)));
                }
            } else if role == "speakers" {
                // Several speakers in one group: kept apart, not glued.
                let speakers = if cell.lines.is_empty() {
                    parse_speakers(text::splitlines(&cell.text))
                } else {
                    parse_speakers(cell.lines.iter().map(|line| line.text.as_str()))
                };
                if !speakers.is_empty() {
                    fields.insert(role.to_owned(), json!(speakers.join(SPEAKER_SEPARATOR)));
                }
            } else if role == "location"
                && (PAIRED_GROUP_RE.is_match(&label) || split && HALF_NUMBERED_GROUP_RE.is_match(&label))
                && beyond
                && cell.lines.len() > 1
            {
                // Two rooms in a cell with no divider: use the printed line
                // facing this group, never their list position.
                if let [line] = facing[..] {
                    fields.insert(role.to_owned(), json!(clean_text(&line.text)));
                }
            } else {
                fields.insert(role.to_owned(), json!(clean_text(&cell.text)));
            }
        }
        let titles: Vec<&Cell> =
            anchor.workshops.iter().map(|&index| anchor.cell(index)).filter(|cell| faces(cell)).collect();
        if let [title] = titles[..] {
            fields.insert("title".to_owned(), json!(clean_text(&title.text)));
        }
        fields.insert("label".to_owned(), json!(label));
        groups.push(fields);
    }
    groups
}

fn group_notes(details: &[Map<String, Value>]) -> Result<Vec<String>, EdtError> {
    let mut notes = Vec::new();
    for fields in details {
        let field = |key: &str| fields.get(key).and_then(Value::as_str);
        let mut parts: Vec<String> = field("title").map(str::to_owned).into_iter().collect();
        if let (Some(start), Some(end)) = (field("start"), field("end")) {
            parts.push(dates::format_range(dates::parse_time(start)?, dates::parse_time(end)?));
        }
        if let Some(location) = field("location") {
            parts.push(format!("Salle : {location}"));
        }
        if let Some(speakers) = field("speakers") {
            parts.push(speakers.to_owned());
        }
        if !parts.is_empty() {
            notes.push(format!("{} : {}", field("label").unwrap_or_default(), parts.join(" ; ")));
        }
    }
    Ok(notes)
}

/// Sub-groups are flagged, not split.
fn groups(anchor: &Anchor, labels: Option<&str>, starts: &[String], ends: &[String]) -> Result<Groups, EdtError> {
    let cohort_cells = anchor.own("cohort");
    let mut ranges = Vec::new();
    for (start, end) in starts.iter().zip(ends) {
        ranges.push(dates::format_range(dates::parse_time(start)?, dates::parse_time(end)?));
    }
    let ranges = dedupe(ranges);
    let labelled = cohort_cells.iter().any(|cell| parse_group_count(&clean_text(&cell.text)).is_some());
    if !(cohort_cells.len() > 1 || ranges.len() > 1 || labelled) {
        return Ok(Groups::default());
    }
    let count = if cohort_cells.len() > 1 {
        cohort_cells.iter().map(|cell| group_rows(cell).len() as i64).sum()
    } else {
        parse_group_count(labels.unwrap_or("")).unwrap_or(0)
    };
    Ok(Groups { multi: true, count, time_ranges: ranges })
}

/// The room, without a link line printed under it (`Plan d'accès`).
///
/// The link itself is kept with the event's links. A room that *is* a link,
/// with nothing else in the cell, stays as it is.
fn room_text(cell: &Cell) -> String {
    let plain: Vec<&TextLine> =
        cell.lines.iter().filter(|line| !cell.links.iter().any(|link| covers(line, link))).collect();
    if plain.is_empty() {
        return clean_text(&cell.text);
    }
    // Several rooms in one cell (a kiosque spread over three amphitheatres):
    // Word leaves a blank line between them, or stacks bare room codes. A name
    // that merely wraps does neither, and stays whole.
    let mut rooms: Vec<Vec<&TextLine>> = Vec::new();
    for line in plain {
        if let Some(room) = rooms.last_mut() {
            let above = room[room.len() - 1];
            let gap = line.bbox[1] - above.bbox[3];
            let stacked =
                ROOM_CODE_RE.is_match(&clean_text(&line.text)) && ROOM_CODE_RE.is_match(&clean_text(&above.text));
            if gap <= PARAGRAPH_GAP_RATIO * (line.bbox[3] - line.bbox[1]) && !stacked {
                room.push(line);
                continue;
            }
        }
        rooms.push(vec![line]);
    }
    rooms.iter().map(|room| joined(room)).collect::<Vec<_>>().join(ROOM_SEPARATOR)
}

/// One room, or `None` plus a note when it depends on the group.
fn location(anchor: &Anchor) -> (Option<String>, Option<String>, Vec<String>) {
    let named: Vec<String> = anchor.own("location").into_iter().map(room_text).collect();
    match named.len() {
        0 => (None, None, named),
        1 => (Some(named[0].clone()), None, named),
        // Several Salle cells face one session: either one room per group, or
        // a single room name Word wrapped over several sub-cells. Nothing in
        // the geometry tells them apart, so the list is kept in `raw`.
        _ => (None, Some(GROUP_LOCATION_NOTE.to_owned()), named),
    }
}

/// Thème cells split in two: one heading, one workshop per group beside it.
///
/// `Ateliers sur Mayotte` sits in the left part of the Thème column, and each
/// group's own workshop in the right part, facing its `Groupe N`, speaker and
/// room. That is one session in groups whose groups happen to carry a title of
/// their own. Only when every group can be told apart — two or more `Groupe`
/// labels — otherwise the workshops stay separate sessions.
fn workshop_blocks(grid: &Grid) -> Vec<(usize, Anchor<'_>)> {
    let Some(column) = grid.column("theme") else { return Vec::new() };
    let inside: Vec<usize> = (0..grid.cells.len())
        .filter(|&index| {
            let cell = &grid.cells[index];
            !is_empty(cell)
                && cell.bbox[0] >= column.x0 - X_EDGE_TOLERANCE
                && cell.bbox[2] <= column.x1 + X_EDGE_TOLERANCE
                && cell.bbox[2] - cell.bbox[0] < column.width() - X_EDGE_TOLERANCE
        })
        .collect();
    let mut blocks = Vec::new();
    for &heading_index in &inside {
        let heading = &grid.cells[heading_index];
        let mut workshops: Vec<usize> = inside
            .iter()
            .copied()
            .filter(|&index| {
                let cell = &grid.cells[index];
                cell.page == heading.page
                    && (cell.bbox[0] - heading.bbox[2]).abs() <= X_EDGE_TOLERANCE
                    && cell.bbox[1] >= heading.bbox[1] - 2.0
                    && cell.bbox[3] <= heading.bbox[3] + 2.0
            })
            .collect();
        workshops.sort_by(|&left, &right| grid.cells[left].bbox[1].total_cmp(&grid.cells[right].bbox[1]));
        if workshops.len() < 2 {
            continue;
        }
        let mut anchor = Anchor::new(grid, heading.clone(), Some(heading_index), workshops);
        anchor.body.retain(|index| !anchor.workshops.contains(index));
        let details = group_details(&anchor);
        if details.len() >= 2 && details.iter().all(|fields| fields.contains_key("title")) {
            blocks.push((heading_index, anchor));
        }
    }
    blocks
}

/// A shared cohort attends both sides of lunch, rather than choosing one.
///
/// Only explicit, aligned time rows for one named cohort qualify. Group rows
/// and overlapping alternatives retain their original combined anchor.
/// Slicing its vertical interval also keeps each slot's own room and speaker
/// while inheriting the theme and any other cells shared across the slots.
///
/// Demi-promotions swapping a session across lunch (`Demi-promo A` facing the
/// morning row, `Demi-promo B` the afternoon one) are sliced the same way, so
/// each half keeps its own hours and the weekly A/B choice applies.
fn sequential_slots(anchor: Anchor<'_>) -> Result<Vec<Anchor<'_>>, EdtError> {
    let cohorts = anchor.own("cohort");
    let (starts, ends) = (anchor.own("start"), anchor.own("end"));
    let aligned = |first: &Cell, last: &Cell| {
        (first.bbox[1] - last.bbox[1]).abs() <= 1.0 && (first.bbox[3] - last.bbox[3]).abs() <= 1.0
    };
    if !anchor.workshops.is_empty() || starts.len() < 2 || starts.len() != ends.len() {
        return Ok(vec![anchor]);
    }
    if !starts.iter().zip(&ends).all(|(first, last)| aligned(first, last)) {
        return Ok(vec![anchor]);
    }
    let halves: Vec<Option<String>> = cohorts.iter().map(|cell| half_promotion_label(&cell.text)).collect();
    let split_halves = cohorts.len() == starts.len()
        && halves.iter().all(Option::is_some)
        && dedupe(halves.iter().flatten().cloned()).len() == halves.len()
        && cohorts.iter().zip(&starts).all(|(cohort, start)| aligned(cohort, start));
    if !split_halves {
        let [cohort] = cohorts[..] else { return Ok(vec![anchor]) };
        if parse_cohorts(&cohort.text).0.is_empty() || parse_group_count(&cohort.text).is_some() {
            return Ok(vec![anchor]);
        }
    }
    let mut hours = Vec::new();
    for (first, last) in starts.iter().zip(&ends) {
        hours.push((dates::parse_time(&first.text)?, dates::parse_time(&last.text)?));
    }
    if hours.iter().any(|(start, end)| start >= end) {
        return Err(EdtError::time("la séance se termine avant de commencer"));
    }
    if hours.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Ok(vec![anchor]);
    }
    let theme = &anchor.theme;
    Ok(starts
        .iter()
        .zip(&ends)
        .map(|(first, last)| {
            let bbox =
                [theme.bbox[0], theme.bbox[1].max(first.bbox[1]), theme.bbox[2], theme.bbox[3].min(last.bbox[3])];
            Anchor::new(anchor.grid, Cell { bbox, ..theme.clone() }, None, Vec::new())
        })
        .collect())
}

/// One anchor per session, in document order.
fn anchors(grid: &Grid) -> Result<Vec<Anchor<'_>>, EdtError> {
    let blocks = workshop_blocks(grid);
    let claimed: Vec<usize> = blocks
        .iter()
        .flat_map(|(heading, block)| std::iter::once(*heading).chain(block.workshops.iter().copied()))
        .collect();
    let mut ordered: Vec<Anchor> = grid
        .cells_for("theme")
        .into_iter()
        .filter(|index| !is_empty(&grid.cells[*index]) && !claimed.contains(index))
        .map(|index| Anchor::new(grid, grid.cells[index].clone(), Some(index), Vec::new()))
        .collect();
    ordered.extend(blocks.into_iter().map(|(_, block)| block));
    ordered.sort_by(|left, right| {
        left.theme.page.cmp(&right.theme.page).then(left.theme.bbox[1].total_cmp(&right.theme.bbox[1]))
    });
    let mut slots = Vec::new();
    for anchor in ordered {
        slots.extend(sequential_slots(anchor)?);
    }
    Ok(slots)
}

/// Red text is an instruction to the students; consecutive lines are one.
fn red_notes(cells: &[&Cell]) -> Vec<String> {
    let mut notes = Vec::new();
    for cell in cells {
        let mut run: Vec<&str> = Vec::new();
        for line in &cell.lines {
            if is_red(line) {
                run.push(&line.text);
            } else if !run.is_empty() {
                notes.push(clean_text(&run.join(" ")));
                run.clear();
            }
        }
        if !run.is_empty() {
            notes.push(clean_text(&run.join(" ")));
        }
    }
    notes
}

/// Walk the `Thème` anchors and emit sessions with their explicit slots.
pub fn build(document: &Document, grid: &Grid) -> Result<Vec<Event>, EdtError> {
    let (reference, from_metadata) = dates::reference_date(&document.file, &document.metadata)?;
    let week_start = (!from_metadata).then_some(reference);
    // Per page, the top of the first Date cell that prints something.
    let mut first_dates: Vec<(u32, f64)> = Vec::new();
    for index in grid.own_cells_for("date") {
        let cell = &grid.cells[index];
        if is_empty(cell) {
            continue;
        }
        match first_dates.iter_mut().find(|(page, _)| *page == cell.page) {
            Some((_, top)) => *top = top.min(cell.bbox[1]),
            None => first_dates.push((cell.page, cell.bbox[1])),
        }
    }
    let mut events: Vec<Event> = Vec::new();
    // Per event: the Module cell it faces, and its printed demi-promo heading.
    let mut sections: Vec<Section> = Vec::new();
    let mut first_page = 0;

    for anchor in anchors(grid)? {
        let theme = &anchor.theme;
        let span = date_range(&anchor, reference)?;
        let first_date =
            first_dates.iter().find(|(page, _)| *page == theme.page).map_or(f64::INFINITY, |(_, top)| *top);
        let carried = match events.last() {
            Some(last) if theme.page > first_page && theme.bbox[1] < first_date => Some(last.date),
            _ => None,
        };
        let (day, until, unscheduled) = match span {
            Some((first, last, label)) => (first, Some(last), Some(label)),
            None => (date_of(&anchor, reference, week_start, carried)?, None, unscheduled_label(&anchor)?),
        };
        let block = unscheduled.as_deref().and_then(half_day);
        let (mut starts, mut ends) = (Vec::new(), Vec::new());
        let (start, mut end) = if until.is_some() {
            (None, None)
        } else if let Some(label) = &unscheduled {
            match block {
                Some((start, end)) => (Some(start), Some(end)),
                None if OPEN_START_PREFIX_RE.is_match(label) => (Some(dates::parse_time(label)?), None),
                None => (None, None),
            }
        } else {
            (starts, ends) = times(&anchor)?;
            // Every start, then every end: the first unreadable one is the
            // error reported, and Python meets them in this order.
            let read =
                |values: &[String]| values.iter().map(|value| dates::parse_time(value)).collect::<Result<Vec<_>, _>>();
            (read(&starts)?.into_iter().min(), read(&ends)?.into_iter().max())
        };
        if let (Some(start), Some(end)) = (start, end) {
            if start >= end {
                return Err(EdtError::time("la séance se termine avant de commencer"));
            }
        }

        let (title, description, prefix_tokens) = title_and_description(&anchor);
        let day_end = start.is_some() && end.is_none() && block.is_none() && runs_to_day_end(&title);
        if day_end {
            end = Some(DAY_END);
        }
        let (cohort_tokens, mut label) = cohorts(&anchor, &prefix_tokens);
        if label.is_none() && !prefix_tokens.is_empty() {
            label = Some(prefix_tokens.join("-"));
        }
        let groups = groups(&anchor, label.as_deref(), &starts, &ends)?;
        let module = anchor.own("module").first().map(|cell| clean_text(&cell.text));
        let (location, mut location_note, room_cells) = location(&anchor);
        // An LV1 slot with a blank Salle column: the groups rotate and only
        // the table on Moodle knows this week's assignment, so the room is
        // left to the reviewer. A slot that does print rooms is an ordinary
        // group session.
        let lv1_room = room_cells.is_empty()
            && (rooms::mentions_lv1(&title) || module.as_deref().is_some_and(rooms::mentions_lv1));
        if lv1_room {
            location_note = Some(rooms::ROTATION_NOTE.to_owned());
        }

        let speaker_cells = anchor.own("speakers");
        let speakers = parse_speakers(speaker_cells.iter().flat_map(|cell| &cell.lines).map(|line| line.text.as_str()));

        let preparation_cells = anchor.own("preparation");
        let preparation = clean_text(
            &preparation_cells
                .iter()
                .flat_map(|cell| &cell.lines)
                .filter(|line| !is_red(line))
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        );
        let red = red_notes(&preparation_cells);
        let note_cells = anchor.notes();
        let extra_cells = anchor.extras();
        let details = group_details(&anchor);
        let mut notes = red.clone();
        notes.extend(note_cells.iter().map(|cell| clean_text(&cell.text)));
        // `Observations : Apporter une pièce d'identité` — the header names it.
        notes.extend(extra_cells.iter().map(|(header, cell)| {
            let note = clean_text(&cell.text);
            if header.is_empty() {
                note
            } else {
                format!("{header} : {note}")
            }
        }));
        notes.extend(group_notes(&details)?);

        let mut sources: Vec<(&str, &Cell)> = vec![("theme", theme)];
        for role in ["speakers", "cohort", "location", "preparation", "module"] {
            sources.extend(anchor.own(role).into_iter().map(|cell| (role, cell)));
        }
        sources.extend(note_cells.iter().map(|cell| ("note", *cell)));
        sources.extend(extra_cells.iter().map(|(_, cell)| ("note", *cell)));

        let mut raw = Map::new();
        let heading = half_promotion_heading(theme);
        let module_cell = anchor.own("module").first().map(|cell| (cell.page, cell.bbox.map(f64::to_bits)));
        sections.push((module_cell, heading.clone()));
        let half = heading
            .as_deref()
            .and_then(half_promotion_label)
            .or_else(|| label.as_deref().and_then(half_promotion_label));
        if let Some(half) = half {
            raw.insert("half_promotion".to_owned(), json!(half));
        }
        if let Some(label) = &unscheduled {
            raw.insert("schedule_label".to_owned(), json!(label));
        }
        if let Some(until) = until.filter(|until| *until > day) {
            raw.insert("until".to_owned(), json!(until.iso()));
        }
        if block.is_some() || day_end {
            raw.insert("default_times".to_owned(), json!(true));
        }
        if unscheduled.is_some() && block.is_none() && start.is_some() {
            raw.insert("pdf_start".to_owned(), json!(true));
        }
        if !details.is_empty() {
            raw.insert("group_details".to_owned(), json!(details));
        }
        if lv1_room {
            raw.insert("lv1_room".to_owned(), json!(true));
        }
        raw.insert("page".to_owned(), json!(theme.page));
        raw.insert("bbox".to_owned(), json!(theme.bbox.map(|value| text::round(value, 1))));
        let ranges: Vec<String> = starts.iter().zip(&ends).map(|(start, end)| format!("{start}-{end}")).collect();
        raw.insert("time_ranges".to_owned(), json!(ranges));
        raw.insert("group_labels".to_owned(), json!(anchor.own_texts("cohort")));
        raw.insert("rooms".to_owned(), json!(room_cells));
        raw.insert("red_notes".to_owned(), json!(red));
        raw.insert("group_speakers".to_owned(), json!(anchor.own_texts("speakers")));

        if events.is_empty() {
            first_page = theme.page;
        }
        events.push(Event {
            date: day,
            start,
            end,
            title,
            description,
            module,
            module_color: fill_hex(theme),
            cohorts: cohort_tokens,
            cohort_label: label,
            groups,
            speakers,
            location,
            location_note,
            preparation: (!preparation.is_empty()).then_some(preparation),
            notes: dedupe(notes).into_iter().filter(|note| !note.is_empty()).collect(),
            links: collect_links(&sources),
            remote: theme.underlined,
            raw,
            week_start: None,
            occurrence: 1,
        });
    }
    inherit_half_promotions(&mut events, &sections);
    Ok(events)
}

/// The Module cell an event faces (page and bbox bits), and its printed
/// demi-promo heading.
type Section = (Option<(u32, [u64; 4])>, Option<String>);

/// A session left unlabelled between sessions of one demi-promo, under the
/// same merged Module cell, belongs to that demi-promo (9–13 November 2026:
/// Friday's role plays for groups 7–12 sit between two `1/2 promo B` rows).
///
/// Only within one Module cell, only when every labelled session in it names
/// the same half, and only between a labelled session before and one after:
/// membership is still never inferred across modules or from group numbers.
fn inherit_half_promotions(events: &mut [Event], sections: &[Section]) {
    let half_of = |event: &Event| event.raw.get("half_promotion").and_then(Value::as_str).map(str::to_owned);
    for index in 0..events.len() {
        let Some(section) = sections[index].0 else { continue };
        if half_of(&events[index]).is_some() {
            continue;
        }
        let members: Vec<usize> = (0..events.len()).filter(|&other| sections[other].0 == Some(section)).collect();
        let halves: Vec<String> = members.iter().filter_map(|&other| half_of(&events[other])).collect();
        let Some(half) = halves.first().cloned() else { continue };
        let labelled = |other: &usize| half_of(&events[*other]).is_some();
        if halves.iter().any(|other| *other != half)
            || !members.iter().any(|other| *other < index && labelled(other))
            || !members.iter().any(|other| *other > index && labelled(other))
        {
            continue;
        }
        // The heading the neighbours print, so titles read alike.
        let heading = members.iter().find_map(|&other| sections[other].1.clone());
        let event = &mut events[index];
        if let Some(heading) = heading {
            event.title = format!("{} ({heading})", event.title);
        }
        event.raw.insert("half_promotion".to_owned(), json!(half));
    }
}

/// The promotion mentioned most often in the cohort cells.
pub fn promotion_of(events: &[Event]) -> Option<String> {
    let mut found: Vec<(String, usize)> = Vec::new();
    for event in events {
        if let Some(promotion) = parse_cohorts(event.cohort_label.as_deref().unwrap_or("")).1 {
            match found.iter_mut().find(|(known, _)| *known == promotion) {
                Some((_, count)) => *count += 1,
                None => found.push((promotion, 1)),
            }
        }
    }
    // The first to reach the highest count, as `Counter.most_common` has it.
    let best = found.iter().map(|(_, count)| *count).max()?;
    found.into_iter().find(|(_, count)| *count == best).map(|(promotion, _)| promotion)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::tests::{cell, document_of, page_of, HEADERS};

    fn week(rows: &[&[&str]]) -> Result<Vec<Event>, EdtError> {
        let document = document_of("Edt du 07.09.2026 au 11.09.2026.pdf", vec![page_of(&HEADERS, rows)]);
        build(&document, &crate::grid::build(&document)?)
    }

    #[test]
    fn cohorts_are_a_union_with_a_promotion() {
        assert_eq!(
            parse_cohorts("FI-CIL-OFF 2026 – 2028"),
            (vec!["FI".to_owned(), "CIL".to_owned(), "OFF".to_owned()], Some("2026-2028".to_owned()))
        );
        assert_eq!(parse_cohorts("OFFICIERS, FIL"), (Vec::new(), None));
    }

    #[test]
    fn groups_are_counted_by_number_or_letter() {
        assert_eq!(parse_group_count("Groupe 3"), Some(1));
        assert_eq!(parse_group_count("Groupes 1 à 9"), Some(9));
        assert_eq!(parse_group_count("Groupes A-D"), Some(4));
        assert_eq!(parse_group_count("groupe b"), Some(1));
        assert_eq!(parse_group_count("Groupe Alpha"), None);
        assert_eq!(parse_group_count("FI"), None);
        assert_eq!(
            cohort_label(&["Groupe 1".into(), "Groupe 2".into(), "Groupes 3-4".into()]),
            Some("Groupes 1-4".into())
        );
        assert_eq!(cohort_label(&["Groupe 1".into(), "Groupe 3".into()]), Some("Groupe 1 / Groupe 3".into()));
        assert_eq!(cohort_label(&[]), None);
    }

    #[test]
    fn text_is_rejoined_at_real_hyphens_only() {
        assert_eq!(clean_text("SIMON-\nMICHEL"), "SIMON-MICHEL");
        assert_eq!(clean_text("outre-\nmer et\n l’État"), "outre-mer et l'État");
        assert_eq!(clean_text("09h00 -\n10h00"), "09h00 - 10h00");
        assert_eq!(clean_text("2026-\n2028"), "2026- 2028");
        assert_eq!(strip_cohort_prefix("FI-CIL / OFF : Parcours public"), "Parcours public");
        assert_eq!(strip_cohort_prefix("Finances : budget"), "Finances : budget");
    }

    #[test]
    fn speakers_are_one_entry_each() {
        let lines = [
            "Mme Alice DURAND-",
            "MARTIN",
            "M. Paul",
            "LEROY",
            "Dr. Jean PETIT",
            "consultant",
            "Préfète Anne MOREAU",
            "Mme Alice DURAND-MARTIN",
        ];
        assert_eq!(
            parse_speakers(lines),
            ["Mme Alice DURAND-MARTIN", "M. Paul LEROY", "Dr. Jean PETIT", "consultant", "Préfète Anne MOREAU"]
        );
        assert_eq!(parse_speakers(["Cabinet", "d'avocats", "Me X"]), ["Cabinet d'avocats", "Me X"]);
        assert!(!SPEAKER_TITLE_RE.is_match("Monique LEROY") && !SPEAKER_TITLE_RE.is_match("Mmex"));
    }

    #[test]
    fn a_plain_row_becomes_an_event() {
        let events = week(&[&[
            "lun. 07/09",
            "09h00",
            "12h30",
            "FI-CIL : Droit public",
            "Mme Alice DURAND",
            "FI-CIL 2026-2028",
            "C101",
            "Lire le dossier",
            "Droit",
        ]])
        .unwrap();
        let event = &events[0];
        assert_eq!(event.date.iso(), "2026-09-07");
        assert_eq!(
            (event.start.unwrap().colon(), event.end.unwrap().colon()),
            ("09:00".to_owned(), "12:30".to_owned())
        );
        assert_eq!(event.title, "Droit public");
        assert_eq!(event.cohorts, ["FI", "CIL"]);
        assert_eq!(event.cohort_label.as_deref(), Some("FI-CIL 2026-2028"));
        assert_eq!(event.location.as_deref(), Some("C101"));
        assert_eq!(event.speakers, ["Mme Alice DURAND"]);
        assert_eq!(event.preparation.as_deref(), Some("Lire le dossier"));
        assert_eq!(event.raw["time_ranges"], json!(["09h00-12h30"]));
        assert_eq!(event.raw["bbox"], json!([290.0, 60.0, 380.0, 100.0]));
        assert!(!event.groups.multi);
        assert_eq!(promotion_of(&events).as_deref(), Some("2026-2028"));
    }

    #[test]
    fn a_blank_lv1_room_is_left_to_the_reviewer() {
        let events = week(&[&["07/09", "14h00", "16h00", "LV1 Anglais", "", "FI", "", "", "Langues"]]).unwrap();
        assert_eq!(events[0].location_note.as_deref(), Some(rooms::ROTATION_NOTE));
        assert_eq!(events[0].raw["lv1_room"], json!(true));
    }

    #[test]
    fn hours_that_do_not_pair_or_run_backwards_are_blocking() {
        let missing = week(&[&["07/09", "", "", "Cours", "", "FI", "", "", ""]]).unwrap_err();
        assert_eq!(missing.message, "aucun horaire de début ou de fin ne fait face à cette séance");
        let backwards = week(&[&["07/09", "12h00", "09h00", "Cours", "", "FI", "", "", ""]]).unwrap_err();
        assert_eq!(
            (backwards.code, backwards.message.as_str()),
            ("TIME_PARSE", "la séance se termine avant de commencer")
        );
        let undated = week(&[&["", "09h00", "10h00", "Cours", "", "FI", "", "", ""]]).unwrap_err();
        assert_eq!(undated.code, "DATE_PARSE");
    }

    #[test]
    fn a_merged_hours_cell_gives_a_block_a_start_or_a_reminder() {
        let mut page = page_of(&HEADERS, &[]);
        let mut row = |top: f64, date: &str, hours: &str, theme: &str| {
            page.cells.push(cell(20.0, top, 110.0, top + 40.0, date));
            page.cells.push(cell(110.0, top, 290.0, top + 40.0, hours));
            page.cells.push(cell(290.0, top, 380.0, top + 40.0, theme));
            page.cells.push(cell(470.0, top, 560.0, top + 40.0, "FI"));
        };
        row(60.0, "07/09", "Après-midi", "Séminaire");
        row(100.0, "08/09", "A partir de 18h00", "APSA");
        row(140.0, "09/09", "Journée", "Visite");
        row(180.0, "10/09", "14h00 - 16h30", "Atelier");
        let document = document_of("Edt du 07.09.2026 au 11.09.2026.pdf", vec![page]);
        let events = build(&document, &crate::grid::build(&document).unwrap()).unwrap();
        let hours: Vec<_> = events.iter().map(|e| (e.start.map(Time::colon), e.end.map(Time::colon))).collect();
        assert_eq!(hours[0], (Some("14:00".into()), Some("17:30".into())));
        assert_eq!(events[0].raw["default_times"], json!(true));
        assert_eq!(hours[1], (Some("18:00".into()), None));
        assert_eq!(events[1].raw["schedule_label"], json!("A partir de 18h00 (fin non précisée)"));
        assert_eq!(events[1].raw["pdf_start"], json!(true));
        assert!(is_sport(&events[1]));
        assert_eq!(hours[2], (None, None));
        assert_eq!(hours[3], (Some("14:00".into()), Some("16:30".into())));
        assert_eq!(events[3].raw["time_ranges"], json!(["14h00-16h30"]));
    }

    #[test]
    fn demi_promo_groups_keep_their_own_room_and_speaker() {
        // 9–13 November 2026: one session row split into `Groupe B1`, `B2`…
        let mut page = page_of(&HEADERS, &[]);
        let spanning = [(0, "jeu. 12/11"), (1, "09H30"), (2, "13H00"), (3, "1/2 promo B\nAtelier"), (8, "Com")];
        for (column, content) in spanning {
            let x0 = 20.0 + 90.0 * column as f64;
            page.cells.push(cell(x0, 60.0, x0 + 90.0, 140.0, content));
        }
        for (row, (speaker, group, room)) in
            [("M. Paul LEROY", "Groupe B1", "C203"), ("Mme Anne PETIT", "Groupe B2", "C204")].into_iter().enumerate()
        {
            let top = 60.0 + 40.0 * row as f64;
            page.cells.push(cell(380.0, top, 470.0, top + 40.0, speaker));
            page.cells.push(cell(470.0, top, 560.0, top + 40.0, group));
            page.cells.push(cell(560.0, top, 650.0, top + 40.0, room));
        }
        let document = document_of("Edt du 07.09.2026 au 11.09.2026.pdf", vec![page]);
        let events = build(&document, &crate::grid::build(&document).unwrap()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].notes,
            [
                "Groupe B1 : 09:30-13:00 ; Salle : C203 ; M. Paul LEROY",
                "Groupe B2 : 09:30-13:00 ; Salle : C204 ; Mme Anne PETIT"
            ]
        );
        assert!(!HALF_GROUP_RE.is_match("Groupe B") && !HALF_GROUP_RE.is_match("Groupe 12"));
    }

    #[test]
    fn an_unlabelled_session_between_one_demi_promo_joins_it() {
        // Three sessions under one merged Module cell; `last` heads the third.
        let section = |last: &str| {
            let mut page = page_of(&HEADERS, &[]);
            let rows = [
                ("09h00", "10h00", "1/2 promo B\n\nApports".to_owned()),
                ("10h30", "11h45", "Mises en situation".to_owned()),
                ("16h00", "17h00", format!("{last}\n\nBilan")),
            ];
            for (row, (start, end, theme)) in rows.iter().enumerate() {
                let top = 60.0 + 40.0 * row as f64;
                for (column, content) in [(0, "ven. 13/11"), (1, start), (2, end), (3, theme.as_str()), (5, "Groupes")]
                {
                    let x0 = 20.0 + 90.0 * column as f64;
                    let mut made = cell(x0, top, x0 + 90.0, top + 40.0, content);
                    // `cell` drops the blank line: open the paragraph gap by hand.
                    for line in made.lines.iter_mut().skip(1) {
                        line.bbox[1] += 12.0;
                        line.bbox[3] += 12.0;
                    }
                    page.cells.push(made);
                }
            }
            page.cells.push(cell(740.0, 60.0, 830.0, 180.0, "Parcours RH"));
            let document = document_of("Edt du 09.11.2026 au 13.11.2026.pdf", vec![page]);
            build(&document, &crate::grid::build(&document).unwrap()).unwrap()
        };
        let events = section("1/2 promo B");
        assert_eq!(events[1].title, "Mises en situation (1/2 promo B)");
        assert_eq!(events[1].raw["half_promotion"], "B");

        // Neighbours that disagree, or none after it: nothing is inferred.
        for last in ["1/2 promo A", "Sans demi-promo"] {
            let events = section(last);
            assert_eq!(events[1].title, "Mises en situation");
            assert!(events[1].raw.get("half_promotion").is_none());
        }
    }

    #[test]
    fn a_demi_promo_heading_may_name_its_cohorts_or_share_the_title() {
        // 16–20 November 2026.
        for (heading, half) in [
            ("CIL Demi-promo A", "A"),
            ("FI-OFF Demi-promo B", "B"),
            ("FI-OFF 2026-2028 Demi-promo A", "A"),
            ("FI-OFF\n2026-28\nDemi-promo B", "B"),
            ("1/2 promo B", "B"),
        ] {
            assert_eq!(half_promotion_label(heading).as_deref(), Some(half), "{heading}");
        }
        for other in ["Parcours Demi-promo A", "Demi-promo A Etude de crise", "½ promo B - Note"] {
            assert!(half_promotion_label(other).is_none(), "{other}");
        }
        assert_eq!(
            leading_half_promotion("½ promo B - FI-CIL-OFF : Note d’analyse collective"),
            Some(("½ promo B".to_owned(), "FI-CIL-OFF : Note d'analyse collective".to_owned()))
        );
        assert!(leading_half_promotion("1/2 promo B").is_none());
        assert!(leading_half_promotion("Note d'analyse collective").is_none());
    }

    #[test]
    fn wrapped_demi_promo_groups_without_a_rule_are_split() {
        // `1/2 Promo A -` / `Groupe 1` over two lines, groups 1 and 2 sharing
        // one cell because the export drew no rule between them.
        let shared = cell(0.0, 0.0, 90.0, 60.0, "1/2 Promo A -\nGroupe 1\n1/2 Promo A -\nGroupe 2");
        let rows = group_rows(&shared);
        assert_eq!(
            rows.iter().map(|row| clean_text(&row.text)).collect::<Vec<_>>(),
            ["1/2 Promo A - Groupe 1", "1/2 Promo A - Groupe 2"]
        );
        assert!(rows[0].bbox[3] <= rows[1].bbox[1] + f64::EPSILON);
        // A label left incomplete keeps the cell whole.
        assert_eq!(group_rows(&cell(0.0, 0.0, 90.0, 60.0, "1/2 Promo A -\nGroupe 1\n1/2 Promo A -")).len(), 1);
        assert_eq!(group_rows(&cell(0.0, 0.0, 90.0, 40.0, "1/2 Promo A -\nGroupe 1")).len(), 1);
    }

    #[test]
    fn only_the_promotion_vote_runs_to_the_end_of_its_day() {
        assert!(runs_to_day_end("Choix du nom de promotion"));
        assert!(runs_to_day_end("CHOIX DU NOM DE PROMOTION"));
        assert!(!runs_to_day_end("APSA"));
        assert!(!runs_to_day_end("Conférence"));
    }

    #[test]
    fn a_session_over_several_days_is_one_reminder() {
        let mut page = page_of(&HEADERS, &[]);
        page.cells.push(cell(20.0, 60.0, 290.0, 100.0, "Du 28 au 02/10"));
        page.cells.push(cell(290.0, 60.0, 380.0, 100.0, "Entretiens"));
        let document = document_of("Edt du 28.09.2026 au 02.10.2026.pdf", vec![page]);
        let events = build(&document, &crate::grid::build(&document).unwrap()).unwrap();
        assert_eq!(events[0].date.iso(), "2026-09-28");
        assert_eq!(events[0].raw["until"], json!("2026-10-02"));
        assert_eq!(events[0].raw["schedule_label"], json!("Du 28 au 02/10"));
        assert_eq!((events[0].start, events[0].end), (None, None));
    }
}
