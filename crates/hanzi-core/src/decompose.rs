//! Reading a character's Make Me a Hanzi **decomposition**.
//!
//! Make Me a Hanzi describes how a character is built from its parts with an IDS
//! string — an ideographic description sequence: a layout operator followed by
//! its operands, each of which is a character or another sequence. 说 is
//! `⿰讠兑` (left and right), 言 is `⿱亠⿱二口` (above and below, nested), and 草
//! is `⿱艹早`. `？` stands where the source could not name a part.
//!
//! This module turns that string into something a screen can draw: the parts in
//! reading order, each marked with whether the board can write it, and the
//! outermost arrangement in words. **Nested sequences are flattened**, so the
//! parts returned are the character's own components rather than the whole tree
//! — `言` comes back as 亠, 二, 口. A part that is itself a character stays one
//! part: `草` is 艹 and 早, never 艹, 日, 十, because the source decomposes one
//! level and the glyph 早 is what the learner is being shown.
//!
//! The parsing is here rather than in the interface for the same reason the
//! radical grouping is: it is logic, it is testable without a window, and there
//! is one answer for every screen.

use serde::{Deserialize, Serialize};

/// The twelve IDS operators Make Me a Hanzi uses, and how each arranges its
/// operands. The wording is the screen's, so it is decided once, here.
const OPERATORS: [(char, &str); 12] = [
    ('⿰', "left and right"),
    ('⿱', "above and below"),
    ('⿲', "left, middle and right"),
    ('⿳', "above, middle and below"),
    ('⿴', "enclosed"),
    ('⿵', "enclosed from above"),
    ('⿶', "enclosed from below"),
    ('⿷', "enclosed from the left"),
    ('⿸', "enclosed from the upper left"),
    ('⿹', "enclosed from the upper right"),
    ('⿺', "enclosed from the lower left"),
    ('⿻', "overlapping"),
];

/// The placeholder Make Me a Hanzi writes where it cannot name a part.
const UNKNOWN: char = '？';

/// How one operand of a decomposition is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    /// The part, or `None` where the source could not name it. A screen shows
    /// the unknown ones too, so the arrangement it describes stays true.
    pub ch: Option<char>,
    /// True when the board can draw this part, so it can be written on its own.
    /// A part the board cannot draw is still worth showing; it is just not
    /// something to practise.
    pub drawable: bool,
}

/// What a character is built from.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decomposition {
    /// The IDS string as stored, e.g. `⿰讠兑`. Empty when the source has none.
    pub raw: String,
    /// The outermost arrangement in words, e.g. `"left and right"`. Empty when
    /// the character is a single glyph rather than a composition.
    pub layout: String,
    /// The parts, in reading order. Nested sequences are flattened, so these are
    /// the character's own components.
    pub parts: Vec<Component>,
}

/// Read a decomposition string.
///
/// `drawable` answers whether the board can write a part, which is the dataset's
/// question rather than this module's — so it is passed in and the result is one
/// struct on both sides of the boundary.
pub fn parse(raw: &str, drawable: impl Fn(char) -> bool) -> Decomposition {
    let raw = raw.trim();
    let symbols: Vec<char> = raw.chars().collect();
    let layout = symbols
        .first()
        .and_then(|symbol| layout_of(*symbol))
        .unwrap_or("")
        .to_string();

    let mut leaves = Vec::new();
    let mut at = 0;
    collect(&symbols, &mut at, &mut leaves);

    let parts = leaves
        .into_iter()
        .map(|ch| {
            if ch == UNKNOWN || ch == '?' {
                Component {
                    ch: None,
                    drawable: false,
                }
            } else {
                Component {
                    ch: Some(ch),
                    drawable: drawable(ch),
                }
            }
        })
        .collect();

    Decomposition {
        raw: raw.to_string(),
        layout,
        parts,
    }
}

/// How one operator arranges its operands, or `None` for anything else.
fn layout_of(symbol: char) -> Option<&'static str> {
    OPERATORS
        .iter()
        .find(|(operator, _)| *operator == symbol)
        .map(|(_, layout)| *layout)
}

/// Flatten one operand (or a whole sequence) into `out`.
///
/// An operator takes **every** symbol after it as operands, because where one
/// operand ends is decided by the operand itself: a nested sequence consumes its
/// own. That is what makes one recursive pass enough.
fn collect(symbols: &[char], at: &mut usize, out: &mut Vec<char>) {
    let Some(&symbol) = symbols.get(*at) else {
        return;
    };
    *at += 1;

    if layout_of(symbol).is_some() {
        while *at < symbols.len() {
            collect(symbols, at, out);
        }
    } else {
        out.push(symbol);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every glyph in these fixtures is drawable unless a test says otherwise.
    fn all(_: char) -> bool {
        true
    }

    fn glyphs(decomposition: &Decomposition) -> Vec<Option<char>> {
        decomposition.parts.iter().map(|part| part.ch).collect()
    }

    #[test]
    fn a_binary_decomposition_gives_two_parts_and_its_arrangement() {
        let found = parse("⿰讠兑", all);
        assert_eq!(glyphs(&found), vec![Some('讠'), Some('兑')]);
        assert_eq!(found.layout, "left and right");
        assert_eq!(found.raw, "⿰讠兑");
    }

    #[test]
    fn a_nested_decomposition_is_flattened_in_reading_order() {
        // 言 is 亠 over (二 over 口): three parts, and the outermost arrangement
        // is the one the screen describes.
        let found = parse("⿱亠⿱二口", all);
        assert_eq!(glyphs(&found), vec![Some('亠'), Some('二'), Some('口')]);
        assert_eq!(found.layout, "above and below");
    }

    #[test]
    fn a_component_that_is_itself_a_character_stays_one_part() {
        // 草 is 艹 over 早. The source decomposes one level, and 早 is the glyph
        // the learner is shown — it is not taken apart again into 日 and 十.
        let found = parse("⿱艹早", all);
        assert_eq!(glyphs(&found), vec![Some('艹'), Some('早')]);
    }

    #[test]
    fn every_operator_is_named() {
        for (symbol, layout) in OPERATORS {
            let found = parse(&format!("{symbol}甲乙"), all);
            assert_eq!(found.layout, layout, "{symbol} is unnamed");
            assert_eq!(glyphs(&found), vec![Some('甲'), Some('乙')]);
        }
    }

    #[test]
    fn an_unnamed_part_is_kept_as_a_gap_rather_than_dropped() {
        // 不 is 一 over an unnamed part. Dropping the gap would say the character
        // is 一 alone, and "above and below" would then be describing nothing.
        let found = parse("⿱一？", all);
        assert_eq!(glyphs(&found), vec![Some('一'), None]);
        assert_eq!(found.layout, "above and below");
        assert!(!found.parts[1].drawable);
    }

    #[test]
    fn a_character_with_no_decomposition_has_no_parts() {
        let none = parse("", all);
        assert!(none.parts.is_empty());
        assert_eq!(none.layout, "");

        let unknown = parse("？", all);
        assert_eq!(glyphs(&unknown), vec![None]);
        assert_eq!(
            unknown.layout, "",
            "a single unnamed glyph is not an arrangement"
        );
    }

    #[test]
    fn a_bare_glyph_is_one_part_with_no_arrangement() {
        let found = parse("一", all);
        assert_eq!(glyphs(&found), vec![Some('一')]);
        assert_eq!(found.layout, "");
    }

    #[test]
    fn drawability_comes_from_the_caller() {
        // The dataset answers this, so the parser asks rather than guesses.
        let found = parse("⿰讠兑", |ch| ch == '兑');
        assert!(!found.parts[0].drawable, "讠 was refused by the caller");
        assert!(found.parts[1].drawable);
    }

    #[test]
    fn stray_text_after_the_operands_is_still_read_as_a_part() {
        // The upstream strings are trusted but not sacred: a malformed one must
        // not panic or silently truncate what it does say.
        let found = parse("⿰白勺x", all);
        assert_eq!(glyphs(&found), vec![Some('白'), Some('勺'), Some('x')]);
    }
}
