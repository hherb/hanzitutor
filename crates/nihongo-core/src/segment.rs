//! Turning a line of Japanese into tokens — the analyser half that both pipelines
//! share.
//!
//! Japanese has no spaces, so a text has to be *segmented* before a learner can tap
//! a word, and the reading over a word comes from knowing what the word is rather
//! than from its characters. Both the passages and the phrases need exactly that,
//! and they need the *same* answer: a word in a phrase and the same word in a
//! passage must not come out with two readings because two binaries each wrote
//! their own version of the rule.
//!
//! So the three decisions live here and only here:
//!
//! * **Which vocabulary word a token is** ([`word_for`]): the analyser's *lexeme*,
//!   so 食べた links to 食べる, falling back to the surface when the analyser names
//!   no lexeme it is written as.
//! * **What reading to draw over it** ([`reading_for`]): UniDic publishes two
//!   candidates and neither is right on its own — the full reasoning is on the
//!   function, and it is the trap that made the reading 食べ(たべる)ます once.
//! * **That whitespace is not a token** ([`analyse`]): a line's spaces are
//!   layout, not words, and neither artifact carries them.
//!
//! Gated behind the `tokenize` feature with the `lindera` dependency it uses: the
//! analyser runs at build time, and the app ships no tokeniser at all.

use std::borrow::Cow;

use lindera::segmenter::Segmenter;
use lindera::token::Token;

use crate::input::normalise_to_hiragana;
use crate::words::{is_kanji, WordDataset};

/// One token of a text, as the analyser segmented it and the vocabulary read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segmented {
    /// The text as written.
    pub surface: String,
    /// The reading to draw over it, in hiragana. `None` for a token whose surface
    /// has no kanji, which needs no ruby.
    pub rt: Option<String>,
    /// The vocabulary word this token is, by its dictionary form, when the course
    /// teaches it.
    pub word: Option<String>,
}

impl Segmented {
    pub fn has_kanji(&self) -> bool {
        self.surface.chars().any(is_kanji)
    }
}

/// Segment `text` into words, each read and each linked to the vocabulary.
///
/// A token whose surface is only whitespace is dropped: it is the space between
/// two words, not a word. The caller decides what to do about a kanji-bearing token
/// that the vocabulary does not hold — `prepare-passages` refuses to write the
/// artifact, and `prepare-phrases` drops the sentence — which is why this reports
/// it rather than judging it.
pub fn analyse(
    segmenter: &Segmenter,
    text: &str,
    words: &WordDataset,
) -> Result<Vec<Segmented>, String> {
    let mut analysed = segmenter
        .segment(Cow::Borrowed(text))
        .map_err(|e| format!("could not segment {text:?}: {e}"))?;

    let mut out: Vec<Segmented> = Vec::new();
    for token in analysed.iter_mut() {
        let surface = token.surface.to_string();
        if surface.trim().is_empty() {
            continue;
        }
        let word = word_for(token, &surface, words);
        let has_kanji = surface.chars().any(is_kanji);
        let rt = if has_kanji {
            reading_for(token, &surface, word.as_deref(), words)
        } else {
            None
        };
        out.push(Segmented { surface, rt, word });
    }
    Ok(out)
}

/// The vocabulary word a token is, if it is one.
///
/// The link is to the word's **dictionary form**, which is what the analyser calls
/// the lexeme: a passage that says 食べた links to 食べる, and 行きます to 行く,
/// because those are the entries the course teaches. Matching the surface first
/// would work for nouns and nouns only.
fn word_for(token: &mut Token<'_>, surface: &str, words: &WordDataset) -> Option<String> {
    let lexeme = token.get("lexeme").map(str::to_string);
    for candidate in [lexeme.as_deref(), Some(surface)].into_iter().flatten() {
        if let Some(word) = words.of_text(candidate) {
            return Some(word.text.clone());
        }
    }
    None
}

/// The reading to draw over a token, in hiragana.
///
/// **Which field to read is not obvious, and getting it wrong renders the wrong
/// ruby.** UniDic publishes two candidates and neither is right on its own:
///
/// * `reading` is the reading of the **base form** for an inflected word — 行き
///   gives イク and 食べ gives タベル — which would draw 行き(いく) and
///   食べ(たべる)ます. For a word that is not inflected it is the surface's own
///   reading, and written the conventional way: 今日 gives キョウ and the particle
///   は gives ハ.
/// * `phonological_surface_form` is the surface's, but in *pronunciation*
///   notation: 行き gives イキ and 食べ gives タベ, which is right, while 今日 gives
///   キョー and は gives ワ, which is not — は is written は and read は however it
///   is pronounced.
///
/// So the rule follows from that: when the surface **is** the base form, take
/// `reading`; when it is inflected, take the surface's pronunciation. UniDic says
/// which is which through `orthographic_surface_form` and
/// `orthographic_base_form`, which this compares rather than inferring from the
/// part of speech.
///
/// UniDic writes `*` for a reading it does not have, and katakana for the ones it
/// does, so the result is folded to hiragana — the artifacts' readings and their
/// furigana are hiragana throughout, and a screen should not have to know which of
/// the two it is looking at. When the analyser has no reading at all and the token
/// is a vocabulary word, that word's own reading is exactly right, so the fallback
/// is a fact rather than a guess.
fn reading_for(
    token: &mut Token<'_>,
    surface: &str,
    word: Option<&str>,
    words: &WordDataset,
) -> Option<String> {
    // A word written exactly as the course writes it takes the course's own
    // reading, so that a text and the word card behind it cannot disagree —
    // UniDic reads 私 as わたくし and the vocabulary says わたし, and both are
    // correct Japanese, which is precisely why one of them has to win here.
    if let Some(word) = word.and_then(|text| words.of_text(text)) {
        if word.text == surface {
            return Some(word.reading.clone());
        }
    }

    let surface_form = token.get("orthographic_surface_form").map(str::to_string);
    let base_form = token.get("orthographic_base_form").map(str::to_string);
    let inflected = matches!((&surface_form, &base_form), (Some(surface), Some(base)) if surface != base);
    let field = if inflected {
        "phonological_surface_form"
    } else {
        "reading"
    };

    let from_analyser = token
        .get(field)
        .map(str::trim)
        .filter(|reading| !reading.is_empty() && *reading != "*")
        .map(normalise_to_hiragana);

    from_analyser.or_else(|| word.and_then(|text| words.of_text(text).map(|w| w.reading.clone())))
}
