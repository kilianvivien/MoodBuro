//! LV1 rooms, the one detail the timetable never carries:
//! `src/edt_parser/rooms.py`.

use std::sync::LazyLock;

use regex::Regex;

use crate::py::text;

/// Longer than this is prose, not a room name.
pub const MAX_ROOM_LENGTH: usize = 40;

/// Shown in place of the room until someone chooses one.
pub const ROTATION_NOTE: &str = "Selon la rotation LV1 sur Moodle";

/// `LV1`, `LV 1`, `Langue vivante 1`, `Langues vivantes I`. The Python
/// pattern wraps this in two look-arounds, checked by hand in `mentions_lv1`.
static LV1_RE: LazyLock<Regex> = LazyLock::new(|| text::regex(r"(?i)LV\s?1|langues?\s+vivantes?\s+(?:1|I)"));

/// True when the text names the LV1 slot — but never `LV10`, nor a stray `1`
/// after some other word.
pub fn mentions_lv1(value: &str) -> bool {
    // `[A-Za-z0-9]` under IGNORECASE also takes the two characters that fold
    // to ASCII letters: the long s and the Kelvin sign.
    let joins = |c: char| c.is_ascii_alphanumeric() || c == 'ſ' || c == '\u{212a}';
    LV1_RE.find_iter(value).any(|found| {
        !value[..found.start()].chars().next_back().is_some_and(joins)
            && !value[found.end()..].chars().next().is_some_and(joins)
    })
}

/// Normalise a room the reviewer typed, or explain what is implausible.
pub fn clean_room(value: &str) -> Result<String, String> {
    let room = text::squeeze(value);
    if room.is_empty() {
        return Err("Indiquez une salle, ou laissez le choix vide.".to_owned());
    }
    if text::len(&room) > MAX_ROOM_LENGTH {
        return Err(format!("Le nom de salle doit contenir au maximum {MAX_ROOM_LENGTH} caractères."));
    }
    if room.chars().any(|c| (c as u32) < 32 || c as u32 == 127) {
        return Err("Ce nom de salle contient des caractères interdits.".to_owned());
    }
    Ok(room)
}

/// Fold a location note into a sentence: first letter only, so that the
/// proper nouns in « Selon la rotation LV1 sur Moodle » survive.
pub fn soften(note: &str) -> String {
    let mut chars = note.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lv1_is_recognised_and_lv10_is_not() {
        for text in ["LV1", "lv 1 Anglais", "LV1-Anglais", "Langue vivante 1", "Langues vivantes I", "(LV1)"] {
            assert!(mentions_lv1(text), "{text}");
        }
        for text in ["LV10", "LV2", "XLV1", "Langue vivante 12", "niveau 1", "Langues vivantes II"] {
            assert!(!mentions_lv1(text), "{text}");
        }
    }

    #[test]
    fn rooms_are_cleaned_or_refused() {
        assert_eq!(clean_room("  C 301\t bis ").unwrap(), "C 301 bis");
        assert!(clean_room("   ").is_err());
        assert!(clean_room(&"x".repeat(41)).is_err());
        assert!(clean_room("C\u{7}301").is_err());
        assert_eq!(soften("Selon la rotation LV1 sur Moodle"), "selon la rotation LV1 sur Moodle");
    }
}
