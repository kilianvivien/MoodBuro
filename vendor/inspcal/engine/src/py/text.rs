//! `str`, `re` and `unicodedata`, as far as the parser uses them.
//!
//! Where Rust and Python disagree the Python answer is the one kept, because
//! titles folded here end up in calendar UIDs. Two differences remain, both
//! out of reach of a timetable: digits are ASCII only (Python's `\d` and
//! `int()` take any Unicode decimal digit), and `\b` is the regex crate's
//! word boundary, which counts a combining mark as part of a word.

use std::sync::LazyLock;

use regex::Regex;
use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;

pub use crate::raw::py_isspace as is_space;

/// What `[^\W\d_]` matches in Python: a letter, or a numeral that is no digit.
pub const LETTER: &str = r"[\p{L}\p{No}\p{Nl}]";

/// Compile a pattern written the way the Python source writes it.
///
/// `\s` becomes Python's white space, which has four separators more than
/// Unicode's; `\d` becomes an ASCII digit.
pub fn regex(pattern: &str) -> Regex {
    let translated = pattern.replace(r"\s", r"[\s\x1c-\x1f]").replace(r"\d", "[0-9]");
    Regex::new(&translated).unwrap_or_else(|error| panic!("pattern {pattern:?}: {error}"))
}

/// `re.fullmatch`: the same pattern, held to the whole string.
pub fn full(pattern: &str) -> Regex {
    regex(&format!("^(?:{pattern})$"))
}

/// `re.match`: the same pattern, held to the start of the string.
pub fn prefix(pattern: &str) -> Regex {
    regex(&format!("^(?:{pattern})"))
}

/// `text.split()`.
pub fn split(text: &str) -> Vec<&str> {
    text.split(is_space).filter(|word| !word.is_empty()).collect()
}

/// `" ".join(text.split())`.
pub fn squeeze(text: &str) -> String {
    split(text).join(" ")
}

/// `text.strip()`.
pub fn strip(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// `text.splitlines()`.
pub fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        let breaks =
            matches!(c, '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}'..='\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}');
        if !breaks {
            continue;
        }
        lines.push(&text[start..index]);
        start = index + c.len_utf8();
        if c == '\r' && matches!(chars.peek(), Some((_, '\n'))) {
            chars.next();
            start += 1;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// `text[:count]`, counted in characters as Python does.
pub fn head(text: &str, count: usize) -> &str {
    text.char_indices().nth(count).map_or(text, |(index, _)| &text[..index])
}

/// `len(text)`.
pub fn len(text: &str) -> usize {
    text.chars().count()
}

/// `unicodedata.normalize("NFKD", text)` with the combining marks removed.
pub fn unaccent(text: &str) -> String {
    text.nfkd().filter(|&c| canonical_combining_class(c) == 0).collect()
}

/// `text.casefold()`.
pub fn casefold(text: &str) -> String {
    caseless::default_case_fold_str(text)
}

/// Whether Python's `[^\W\d_]` matches this character.
pub fn is_letter(c: char) -> bool {
    static ONE: LazyLock<Regex> = LazyLock::new(|| full(LETTER));
    c.is_ascii_alphabetic() || (!c.is_ascii() && ONE.is_match(c.encode_utf8(&mut [0; 4])))
}

/// `any(char.isalpha() for char in text)`.
pub fn has_alpha(text: &str) -> bool {
    static ALPHA: LazyLock<Regex> = LazyLock::new(|| regex(r"\p{L}"));
    ALPHA.is_match(text)
}

/// `re.sub(r"\s+", " ", text).strip()`.
pub fn single_spaced(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending = false;
    for c in text.chars() {
        if is_space(c) {
            pending = true;
        } else {
            if pending && !out.is_empty() {
                out.push(' ');
            }
            pending = false;
            out.push(c);
        }
    }
    out
}

/// `int(digits)` for a run of ASCII digits, however long.
pub fn int(digits: &str) -> i64 {
    digits.parse().unwrap_or(i64::MAX)
}

/// Python's `round(value, digits)`: correctly rounded, ties to even.
pub fn round(value: f64, digits: usize) -> f64 {
    format!("{value:.digits$}").parse().unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_strip_use_pythons_white_space() {
        assert_eq!(split(" a\u{1f}b\u{a0}c \n"), vec!["a", "b", "c"]);
        assert_eq!(strip("\u{1c} x \u{85}"), "x");
        assert_eq!(single_spaced("  a \n\t b  "), "a b");
    }

    #[test]
    fn splitlines_knows_every_line_boundary() {
        assert_eq!(splitlines("a\r\nb\rc\n"), vec!["a", "b", "c"]);
        assert_eq!(splitlines("a\n\nb"), vec!["a", "", "b"]);
        assert_eq!(splitlines("a\u{2028}b\u{85}"), vec!["a", "b"]);
        assert!(splitlines("").is_empty());
    }

    #[test]
    fn folding_matches_python() {
        assert_eq!(unaccent("Thème de la séance ﬁn ①"), "Theme de la seance fin 1");
        assert_eq!(casefold("Straße ÉCOLE"), "strasse école");
        assert_eq!(head("déjà vu", 4), "déjà");
        assert_eq!(len("déjà"), 4);
    }

    #[test]
    fn letters_are_pythons() {
        assert!(is_letter('é') && is_letter('Z') && is_letter('²'));
        assert!(!is_letter('1') && !is_letter('_') && !is_letter('-'));
        assert!(has_alpha("12 a") && !has_alpha("12 ²"));
    }

    #[test]
    fn round_is_correctly_rounded() {
        assert_eq!(round(2.675, 2), 2.67);
        assert_eq!(round(36.25, 1), 36.2);
        assert_eq!(round(-0.04, 1), 0.0);
    }
}
