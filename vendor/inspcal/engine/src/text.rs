//! pdfplumber's `extract_text` for a handful of characters, with its default
//! settings (`utils/text.py`): characters are put on lines, read left to
//! right, and cut into words at every blank and at every gap wider than 3 pt.
//! Words on a line are then joined by one space, lines by a newline.

use crate::cluster::cluster_objects;
use crate::raw::Char;

const X_TOLERANCE: f64 = 3.0;
const Y_TOLERANCE: f64 = 3.0;

struct Word {
    text: String,
    top: f64,
}

/// What a ligature glyph stands for; pdfplumber spells it out in words.
fn expand(c: char, into: &mut String) {
    match c {
        'ﬀ' => into.push_str("ff"),
        'ﬃ' => into.push_str("ffi"),
        'ﬄ' => into.push_str("ffl"),
        'ﬁ' => into.push_str("fi"),
        'ﬂ' => into.push_str("fl"),
        'ﬆ' | 'ﬅ' => into.push_str("st"),
        other => into.push(other),
    }
}

fn word_of(chars: &[&Char]) -> Word {
    let mut text = String::new();
    for c in chars {
        expand(c.text, &mut text);
    }
    Word { text, top: chars.iter().map(|c| c.top).fold(f64::INFINITY, f64::min) }
}

pub fn extract_text(chars: &[&Char]) -> String {
    if chars.is_empty() {
        return String::new();
    }
    let mut words: Vec<Word> = Vec::new();
    for mut line in cluster_objects(chars, |c| c.top, Y_TOLERANCE) {
        line.sort_by(|a, b| a.x0.total_cmp(&b.x0));
        let mut current: Vec<&Char> = Vec::new();
        for c in line {
            if c.is_space() {
                if !current.is_empty() {
                    words.push(word_of(&current));
                    current.clear();
                }
                continue;
            }
            if let Some(previous) = current.last() {
                let apart = c.x0 < previous.x0
                    || c.x0 > previous.x1 + X_TOLERANCE
                    || (c.top - previous.top).abs() > Y_TOLERANCE;
                if apart {
                    words.push(word_of(&current));
                    current.clear();
                }
            }
            current.push(c);
        }
        if !current.is_empty() {
            words.push(word_of(&current));
        }
    }
    let indices: Vec<usize> = (0..words.len()).collect();
    cluster_objects(&indices, |&index| words[index].top, Y_TOLERANCE)
        .iter()
        .map(|line| line.iter().map(|&index| words[index].text.as_str()).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(text: char, x0: f64, x1: f64, top: f64) -> Char {
        Char { text, x0, x1, top, bottom: top + 9.0, size: 9.0, fontname: String::new(), color: None }
    }

    fn read(chars: &[Char]) -> String {
        extract_text(&chars.iter().collect::<Vec<_>>())
    }

    #[test]
    fn a_blank_or_a_wide_gap_starts_a_word() {
        let chars = [
            glyph('a', 0.0, 5.0, 10.0),
            glyph('b', 5.0, 10.0, 10.0),
            glyph(' ', 10.0, 12.0, 10.0),
            glyph('c', 12.0, 17.0, 10.0),
            glyph('d', 20.1, 25.0, 10.0), // 3.1 pt after c
            glyph('e', 28.0, 33.0, 10.0), // exactly 3 pt after d: same word
        ];
        assert_eq!(read(&chars), "ab c de");
    }

    #[test]
    fn characters_are_read_left_to_right_whatever_their_order() {
        let chars = [glyph('b', 5.0, 10.0, 10.0), glyph('a', 0.0, 5.0, 10.0)];
        assert_eq!(read(&chars), "ab");
    }

    #[test]
    fn lines_further_apart_than_the_tolerance_are_separate() {
        // b sits 2.5 pt lower than a: still the same line. c does not.
        let chars = [glyph('a', 0.0, 5.0, 10.0), glyph('b', 5.0, 10.0, 12.5), glyph('c', 0.0, 5.0, 21.0)];
        assert_eq!(read(&chars), "ab\nc");
    }

    #[test]
    fn ligatures_are_spelled_out() {
        assert_eq!(read(&[glyph('ﬁ', 0.0, 5.0, 10.0), glyph('n', 5.0, 10.0, 10.0)]), "fin");
    }

    #[test]
    fn a_zero_width_space_still_separates() {
        let chars = [glyph('a', 0.0, 5.0, 10.0), glyph(' ', 5.0, 5.0, 10.0), glyph('b', 5.5, 10.0, 10.0)];
        assert_eq!(read(&chars), "a b");
    }
}
