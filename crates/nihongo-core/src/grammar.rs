//! The grammar course: patterns imported from OpenJLPT, with the examples their
//! own authors wrote for them.
//!
//! # What this is, and what it deliberately is not
//!
//! `ROADMAP_NIHONGO.md`'s known weak spots have said since N0 that **grammar is
//! the largest gap in the product**: particles, the copula and every inflectional
//! ending are kana, so a learner who finishes the kana course has unlocked the
//! grammar and been given nothing to use it on. This module is the data half of
//! closing that gap — 526 patterns, each with the meaning it carries, the forms it
//! is built from, and two or three whole sentences showing it.
//!
//! The text is **not this project's**. It is [OpenJLPT](https://github.com/evanclan/OpenJLPT)'s
//! grammar dataset, whose explanations and example sentences were written by that
//! project's contributors and released under **CC BY-SA 4.0**. `prepare-grammar`
//! compacts it into `grammar.bin.gz` and records the attribution and the
//! modifications in the artifact itself, because a share-alike licence requires
//! both to travel with a redistribution.
//!
//! # The levels are dropped, and that is the decision worth reading
//!
//! The source files are `n5.json` … `n1.json`, and the level of a point is part of
//! how the corpus is organised. **None of it is in this artifact.** There has been
//! no official JLPT grammar list since 2010, the source's own NOTICE says its
//! levels "follow the consensus of common JLPT preparation materials" rather than
//! any authority, and `HANDOVER_NIHONGO.md` invariant 20 forbids the interface
//! from implying that a level here is the JLPT's. So the level field is not read,
//! the five files become **one course in the order the corpus introduces them**,
//! and [`strip_level_references`] takes the level cross-references out of the notes
//! (38 of them — "use までに (N4)") as well.
//!
//! # Why there is no band on a pattern
//!
//! The vocabulary and the phrases are levelled by [`crate::words::band_for`] — the
//! highest kyōiku grade among a word's kanji — and the obvious move is to level a
//! grammar point the same way, by the hardest character in its examples. It was
//! measured before it was rejected: **344 of the 526 points land in band 7**,
//! because an example sentence is three times the length of a word and a single
//! jōyō-remainder character puts the whole point there, and only **2** points reach
//! band 1. A ladder that puts 65% of the course on its top rung is not a ladder, so
//! a pattern has no band and the screen offers one ordered course instead. The
//! order is the corpus's teaching order, and the screen says whose ladder it is
//! not.
//!
//! # The invariant a grammar point is held to, and the one it is excused
//!
//! [`crate::phrases`] and [`crate::passages`] guarantee that every kanji-bearing
//! token is a word the course teaches. **A grammar example cannot be, and is not
//! held to it**: these sentences are the corpus's own teaching text, chosen by
//! their authors to show a pattern rather than to stay inside a 16,073-word
//! vocabulary, and 557 of the 1,526 examples contain a jōyō-remainder character.
//! What replaces the guarantee is the corpus's own furigana: **every example that
//! contains a kanji carries a reading over every one of them**, parsed by
//! [`parse_furigana`] and checked at build time, so a learner can always read what
//! is shown even where there is no word card to open. The measurements are in
//! `tests/grammar_artifact.rs`.

use serde::{Deserialize, Serialize};

use crate::words::{is_kanji, Ruby};

/// Magic bytes at the head of a grammar artifact, checked before decoding.
pub const GRAMMAR_ARTIFACT_MAGIC: &[u8; 8] = b"GRAMM001";

/// One example sentence: the Japanese, its English, and the furigana the corpus
/// wrote over it.
///
/// The ruby is the **corpus's**, parsed rather than composed: `{私|わたし}は…` is
/// the source's own markup, so this app never invents an alignment — the rule
/// [`crate::words`] states for a word's reading, applied to a sentence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarExample {
    /// The sentence as written, full-width punctuation and all.
    pub ja: String,
    /// The English translation the corpus pairs with it.
    pub en: String,
    /// The furigana, in order. Concatenating the parts always spells
    /// [`GrammarExample::ja`].
    ///
    /// A sentence with no kanji needs no reading and carries **one plain segment**
    /// covering the whole of it (16 of the 1,526 are in that state), so the list is
    /// never empty and every example is drawn the same way. The two facts
    /// `prepare-grammar` refuses to write are an example with a kanji the markup
    /// does not cover, and a markup that does not spell the sentence.
    pub ruby: Vec<Ruby>,
}

impl GrammarExample {
    /// Whether the sentence contains a kanji at all, and so needs furigana.
    pub fn has_kanji(&self) -> bool {
        self.ja.chars().any(is_kanji)
    }

    /// The sentence as the ruby spells it.
    ///
    /// Equal to [`GrammarExample::ja`] by construction — the parser refuses a
    /// mismatch — and the artifact test asserts the two agree rather than assuming
    /// it.
    pub fn text_from_ruby(&self) -> String {
        self.ruby.iter().map(|part| part.ruby.as_str()).collect()
    }

    /// The sentence in kana, as the readings spell it.
    pub fn kana(&self) -> String {
        self.ruby
            .iter()
            .map(|part| part.rt.as_deref().unwrap_or(&part.ruby))
            .collect()
    }

    /// The kanji the furigana does not cover.
    ///
    /// Empty is the property a shipped example must have: a character with no
    /// reading over it is one the learner cannot read and has no card for, since
    /// grammar examples are not held to the vocabulary.
    pub fn kanji_without_ruby(&self) -> Vec<char> {
        // Every character of `ja` is in the ruby parts in order, so this walks the
        // ruby rather than the sentence: a kanji inside a part with an `rt` is
        // covered, and one inside a part without is not.
        self.ruby
            .iter()
            .filter(|part| part.rt.is_none())
            .flat_map(|part| part.ruby.chars())
            .filter(|ch| is_kanji(*ch))
            .collect()
    }
}

/// One grammar point: the pattern, what it means, how it is built, and the
/// sentences that show it.
///
/// There is **no level field and no band**, deliberately — see this module's
/// documentation. What there is instead is the corpus's own identifier, which is a
/// slug of the romaji (`ni taishite` → `ni-taishite`), so a pattern can be cited,
/// linked to and asserted on without a second numbering scheme.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarPoint {
    /// The corpus's stable id for the point: a slug of its romaji.
    pub id: String,
    /// The pattern as written, e.g. `〜に対して`. The tilde marks where the
    /// preceding word or clause goes.
    pub pattern: String,
    /// The pattern in kana, present only when it contains a kanji (107 of the 526).
    pub reading: Option<String>,
    /// Modified Hepburn, the corpus's own, with no macrons.
    pub romaji: String,
    /// What it means, in the corpus's words.
    pub meaning: String,
    /// What it attaches to — `Noun + に対して / Noun + に対する + Noun`.
    pub formation: String,
    /// Nuance, register, or how it differs from a similar pattern. The corpus's
    /// prose with its JLPT level cross-references removed; 2 of the 526 have none.
    pub notes: Option<String>,
    /// The corpus's own tags, 1–3 of them, from the controlled list in its schema.
    pub tags: Vec<String>,
    /// Two or three examples, in the corpus's order.
    pub examples: Vec<GrammarExample>,
}

impl GrammarPoint {
    /// Whether this point answers a case-insensitive query.
    ///
    /// The fields a learner might type: the pattern itself, its romaji, its
    /// meaning, its id, its tags, and the formation tokens (`Verb-te`, `Noun`).
    /// Notes are left out on purpose — they are prose about a pattern, and matching
    /// them makes a search for まで return every point that mentions it in passing.
    pub fn matches(&self, query: &str) -> bool {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return false;
        }
        let fields = [
            self.pattern.as_str(),
            self.romaji.as_str(),
            self.meaning.as_str(),
            self.id.as_str(),
            self.formation.as_str(),
        ];
        if fields
            .iter()
            .any(|field| field.to_lowercase().contains(&needle))
        {
            return true;
        }
        self.tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(&needle))
    }

    /// Whether the pattern itself is written with a kanji, and so has a reading.
    pub fn pattern_has_kanji(&self) -> bool {
        self.pattern.chars().any(is_kanji)
    }

    /// Every example, in the corpus's order.
    pub fn examples(&self) -> impl Iterator<Item = &GrammarExample> {
        self.examples.iter()
    }
}

/// Where the grammar came from, and what was done to it.
///
/// Recorded because an imported course is not a fact about Japanese: it is one
/// project's corpus, read on one day, with a licence that asks for both the
/// attribution and a statement of the modifications. A later rebuild has to be able
/// to say what changed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarSource {
    /// The project, named: `OpenJLPT`.
    pub corpus: String,
    /// Where it lives: `https://github.com/evanclan/OpenJLPT`.
    pub url: String,
    /// The dataset version its own `meta.json` states, e.g. `0.3.0`.
    pub version: String,
    /// The licence the grammar content is under: `CC BY-SA 4.0`.
    pub licence: String,
    /// The date `scripts/fetch-openjlpt.sh` recorded, or `unknown`.
    pub retrieved: String,
    /// The selection rule in one sentence, so the artifact describes its own cut.
    pub selection: String,
    /// What was changed on the way in, as the share-alike licence requires.
    pub modifications: String,
}

/// The decoded payload of the shipped grammar artifact.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GrammarArtifact {
    pub points: Vec<GrammarPoint>,
    pub source: GrammarSource,
}

impl GrammarArtifact {
    pub fn new(points: Vec<GrammarPoint>, source: GrammarSource) -> Self {
        Self { points, source }
    }
}

/// The grammar course, in the order it is taught.
#[derive(Clone, Debug, Default)]
pub struct GrammarDataset {
    points: Vec<GrammarPoint>,
    source: GrammarSource,
}

impl GrammarDataset {
    /// The dataset, in the order it was given.
    ///
    /// **Deliberately not sorted.** The vocabulary and the phrases are ordered by
    /// their ladder because a band is what a screen draws; a grammar point has no
    /// band (this module's documentation has the measurement), and the corpus's own
    /// order — the order a course introduces the patterns in — is the one thing
    /// that says what to read after what. So the order the pipeline wrote is the
    /// order the screen shows, and `tests/grammar_artifact.rs` pins its ends.
    pub fn from_points(points: Vec<GrammarPoint>, source: GrammarSource) -> Self {
        Self { points, source }
    }

    /// Decode an artifact produced by `prepare-grammar`.
    pub fn from_gzip_bytes(bytes: &[u8]) -> std::io::Result<Self> {
        use std::io::Read;

        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut raw = Vec::new();
        decoder.read_to_end(&mut raw)?;

        if raw.len() < GRAMMAR_ARTIFACT_MAGIC.len()
            || &raw[..GRAMMAR_ARTIFACT_MAGIC.len()] != GRAMMAR_ARTIFACT_MAGIC
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "not a grammar artifact (bad or outdated magic); re-run `prepare-grammar`",
            ));
        }
        let artifact: GrammarArtifact =
            postcard::from_bytes(&raw[GRAMMAR_ARTIFACT_MAGIC.len()..]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("corrupt grammar artifact: {e}"),
                )
            })?;
        Ok(Self::from_points(artifact.points, artifact.source))
    }

    pub fn points(&self) -> &[GrammarPoint] {
        &self.points
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn source(&self) -> &GrammarSource {
        &self.source
    }

    /// The point with this corpus id.
    pub fn get(&self, id: &str) -> Option<&GrammarPoint> {
        self.points.iter().find(|point| point.id == id)
    }

    /// Where this point sits in the course, counted from 1 — what a stage shows as
    /// "12 of 526" and what the arrows step through.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.points.iter().position(|point| point.id == id).map(|i| i + 1)
    }

    /// One page of the course, clamped the way the command clamps it.
    pub fn page(&self, offset: usize, limit: usize) -> Vec<&GrammarPoint> {
        self.points.iter().skip(offset).take(limit).collect()
    }

    /// The points a query matches, at most `limit` of them.
    ///
    /// An empty query matches nothing rather than everything: the screen draws the
    /// course's own pages while the search box is empty, and a search that answered
    /// with all 526 would be a second way to ask for page one.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&GrammarPoint> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        self.points
            .iter()
            .filter(|point| point.matches(query))
            .take(limit)
            .collect()
    }

    /// How many points a query matches, before any page is applied.
    pub fn search_total(&self, query: &str) -> usize {
        if query.trim().is_empty() {
            return 0;
        }
        self.points.iter().filter(|point| point.matches(query)).count()
    }

    /// Every tag used by the course, once each, in the corpus's order of first use.
    ///
    /// Derived rather than stored, so it cannot disagree with the points.
    pub fn tags(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        for point in &self.points {
            for tag in &point.tags {
                if !seen.contains(&tag.as_str()) {
                    seen.push(tag);
                }
            }
        }
        seen
    }

    /// How many example sentences the course carries.
    pub fn example_count(&self) -> usize {
        self.points.iter().map(|point| point.examples.len()).sum()
    }
}

/// What was wrong with a furigana markup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FuriganaError {
    /// A `{` with no `}` after it.
    UnclosedBrace,
    /// A `}` that no `{` opened.
    StrayBrace,
    /// A span with no `|` in it, so there is nothing to read.
    NoSeparator(String),
    /// A span whose reading is empty.
    EmptyReading(String),
    /// The spans do not spell the sentence they are meant to annotate.
    TextMismatch(String),
}

impl std::fmt::Display for FuriganaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FuriganaError::UnclosedBrace => write!(f, "a `{{` with no `}}` after it"),
            FuriganaError::StrayBrace => write!(f, "a `}}` that no `{{` opened"),
            FuriganaError::NoSeparator(span) => {
                write!(f, "the span {span:?} has no `|` in it")
            }
            FuriganaError::EmptyReading(span) => {
                write!(f, "the span {span:?} has an empty reading")
            }
            FuriganaError::TextMismatch(text) => write!(
                f,
                "the furigana spells {text:?}, which is not the sentence it annotates"
            ),
        }
    }
}

impl std::error::Error for FuriganaError {}

/// Parse the corpus's furigana markup into ruby segments.
///
/// The markup is `{私|わたし}は{学生|がくせい}です。`: a braced span is a run of text
/// with its reading after a `|`, and everything outside the braces needs none. The
/// parser is the *only* place this project reads an alignment for a grammar
/// example — the alternative, composing one from the characters, is the mistake
/// [`crate::words`] refuses one step out — so it is a function with tests rather
/// than six lines inside the pipeline.
///
/// It refuses a markup that does not spell `text` exactly, because ruby drawn over
/// the wrong characters is worse than no ruby: the segments must concatenate back
/// to the sentence.
pub fn parse_furigana(text: &str, markup: &str) -> Result<Vec<Ruby>, FuriganaError> {
    let mut parts: Vec<Ruby> = Vec::new();
    let mut plain = String::new();
    let mut chars = markup.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                let mut span = String::new();
                let mut closed = false;
                for inner in chars.by_ref() {
                    if inner == '}' {
                        closed = true;
                        break;
                    }
                    span.push(inner);
                }
                if !closed {
                    return Err(FuriganaError::UnclosedBrace);
                }
                if !plain.is_empty() {
                    parts.push(Ruby::new(std::mem::take(&mut plain), None));
                }
                let Some((ruby, rt)) = span.split_once('|') else {
                    return Err(FuriganaError::NoSeparator(span));
                };
                if rt.trim().is_empty() {
                    return Err(FuriganaError::EmptyReading(span));
                }
                parts.push(Ruby::new(ruby, Some(rt.to_string())));
            }
            '}' => return Err(FuriganaError::StrayBrace),
            other => plain.push(other),
        }
    }

    if !plain.is_empty() {
        parts.push(Ruby::new(plain, None));
    }

    let spelled: String = parts.iter().map(|part| part.ruby.as_str()).collect();
    if spelled != text {
        return Err(FuriganaError::TextMismatch(spelled));
    }
    Ok(parts)
}

/// Take the JLPT level references out of a note.
///
/// The corpus labels a pattern by level, and its notes cross-reference other
/// patterns by the same labels: *"Softer requests use 〜てくれませんか (N4)"*, *"Compare
/// N2 〜かのようだ"*. The level is dropped here (this module's documentation says
/// why), so the references have to go too, or the interface would be naming levels
/// it does not carry. **38 of the 524 notes change**, and only ever by losing the
/// label: a bracketed tag goes with the space it sat behind, `at N4` becomes
/// `elsewhere`, and a bare label is simply removed. `tests/grammar_artifact.rs`
/// asserts that no shipped text carries one afterwards, which is the test
/// `words::band_name`'s "no name says JLPT" obligation takes here.
///
/// It is a **modification of the corpus's text**, and the artifact's
/// [`GrammarSource::modifications`] says so, as CC BY-SA 4.0 §3(a)(1)(B) requires.
pub fn strip_level_references(note: &str) -> String {
    let mut out = String::with_capacity(note.len());
    let chars: Vec<char> = note.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // `(N4)` / `（N4）`, with any space in front of it.
        if matches!(chars[i], '(' | '（') {
            if let Some(level) = level_at(&chars, i + 1) {
                let mut j = level;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < chars.len() && matches!(chars[j], ')' | '）') {
                    while out.ends_with(' ') {
                        out.pop();
                    }
                    i = j + 1;
                    continue;
                }
            }
        }
        // `at N4` -> `elsewhere`: "covered at N3" is the one construction where
        // removing the label alone would leave a sentence with a hole in it.
        if chars[i..].starts_with(&['a', 't', ' ']) {
            if let Some(level) = level_at(&chars, i + 3) {
                if level == i + 5 {
                    out.push_str("elsewhere");
                    i = level;
                    continue;
                }
            }
        }
        // A bare label.
        if let Some(level) = level_at(&chars, i) {
            i = level;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    // Two spaces can be left where a label stood between them. Nothing else is
    // tidied: a note's own `...` is prose, and a rule that removed the space in
    // front of every full stop would rewrite notes it was never meant to touch.
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out.trim().to_string()
}

/// `N1`–`N5` starting at `start`, answering the index just past it.
fn level_at(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start) != Some(&'N') {
        return None;
    }
    match chars.get(start + 1) {
        Some('1'..='5') => Some(start + 2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(ja: &str, markup: &str) -> GrammarExample {
        GrammarExample {
            ja: ja.to_string(),
            en: "t".to_string(),
            ruby: parse_furigana(ja, markup).expect("parses"),
        }
    }

    fn point(id: &str, pattern: &str, tags: &[&str]) -> GrammarPoint {
        GrammarPoint {
            id: id.to_string(),
            pattern: pattern.to_string(),
            reading: None,
            romaji: id.replace('-', " "),
            meaning: "meaning".to_string(),
            formation: "Noun + は".to_string(),
            notes: None,
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            examples: vec![example("私は学生です。", "{私|わたし}は{学生|がくせい}です。")],
        }
    }

    #[test]
    fn the_markup_becomes_ruby_that_spells_the_sentence() {
        let parts = parse_furigana("私は学生です。", "{私|わたし}は{学生|がくせい}です。").unwrap();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], Ruby::new("私", Some("わたし".into())));
        assert_eq!(parts[1], Ruby::new("は", None));
        assert_eq!(parts[3], Ruby::new("です。", None));
        let spelled: String = parts.iter().map(|p| p.ruby.as_str()).collect();
        assert_eq!(spelled, "私は学生です。");
    }

    #[test]
    fn a_markup_that_does_not_spell_the_sentence_is_refused() {
        let err = parse_furigana("私は学生です。", "{私|わたし}は{学生|がくせい}です").unwrap_err();
        assert!(matches!(err, FuriganaError::TextMismatch(_)), "{err:?}");
        assert!(err.to_string().contains("which is not the sentence"));
    }

    #[test]
    fn the_three_malformed_markups_are_each_named() {
        assert_eq!(
            parse_furigana("あ", "{私|わたし").unwrap_err(),
            FuriganaError::UnclosedBrace
        );
        assert_eq!(parse_furigana("あ}", "あ}").unwrap_err(), FuriganaError::StrayBrace);
        assert!(matches!(
            parse_furigana("私", "{私}").unwrap_err(),
            FuriganaError::NoSeparator(_)
        ));
        assert!(matches!(
            parse_furigana("私", "{私|}").unwrap_err(),
            FuriganaError::EmptyReading(_)
        ));
    }

    #[test]
    fn an_example_knows_what_it_reads_as_and_what_is_not_covered() {
        let read = example("私は学生です。", "{私|わたし}は{学生|がくせい}です。");
        assert!(read.has_kanji());
        assert_eq!(read.text_from_ruby(), "私は学生です。");
        assert_eq!(read.kana(), "わたしはがくせいです。");
        assert!(read.kanji_without_ruby().is_empty());

        // Kana only: nothing to annotate, and no kanji to be missing a reading.
        let kana = GrammarExample {
            ja: "これはペンです。".to_string(),
            en: "t".to_string(),
            ruby: Vec::new(),
        };
        assert!(!kana.has_kanji());
        assert!(kana.kanji_without_ruby().is_empty());

        // A kanji left outside the markup is exactly what the pipeline refuses.
        let uncovered = GrammarExample {
            ja: "学生です。".to_string(),
            en: "t".to_string(),
            ruby: vec![Ruby::new("学生です。", None)],
        };
        assert_eq!(uncovered.kanji_without_ruby(), vec!['学', '生']);
    }

    #[test]
    fn the_level_references_come_out_of_a_note_and_the_sentence_survives() {
        assert_eq!(
            strip_level_references("For a deadline ('by'), use までに (N4)."),
            "For a deadline ('by'), use までに."
        );
        assert_eq!(
            strip_level_references("A little more formal than けど (N4)."),
            "A little more formal than けど."
        );
        assert_eq!(
            strip_level_references("Compare しか〜ない (N4), which stresses that."),
            "Compare しか〜ない, which stresses that."
        );
        assert_eq!(
            strip_level_references("Compare N2 〜かのようだ."),
            "Compare 〜かのようだ."
        );
        assert_eq!(
            strip_level_references("The 'like, as if' use is covered at N3."),
            "The 'like, as if' use is covered elsewhere."
        );
        assert_eq!(
            strip_level_references("The 'typical of' らしい (男らしい) is a separate N3 point."),
            "The 'typical of' らしい (男らしい) is a separate point."
        );
        // A note that mentions no level is left exactly as it was, and a genuine
        // `...` is not mistaken for one.
        assert_eq!(
            strip_level_references("Used with いくら: 'no matter ...'"),
            "Used with いくら: 'no matter ...'"
        );
        // What a screen would never show again.
        assert!(!strip_level_references("covered at N3, see N1 〜すら").contains("N3"));
    }

    #[test]
    fn a_point_answers_the_queries_a_learner_would_type() {
        let mut p = point("ni-taishite", "〜に対して", &["relation", "formal"]);
        p.romaji = "ni taishite".to_string();
        p.meaning = "toward; against".to_string();
        p.formation = "Noun + に対して".to_string();
        for query in ["に対", "taishite", "TOWARD", "relation", "Noun", "ni-taishite"] {
            assert!(p.matches(query), "{query} should match");
        }
        for query in ["", "   ", "kyōiku", "たべて"] {
            assert!(!p.matches(query), "{query} should not match");
        }
    }

    #[test]
    fn the_course_is_offered_in_the_order_it_was_given() {
        let dataset = GrammarDataset::from_points(
            vec![
                point("wa", "〜は", &["particle"]),
                point("ga", "〜が", &["particle"]),
                point("te-kudasai", "〜てください", &["request"]),
            ],
            GrammarSource::default(),
        );
        let ids: Vec<&str> = dataset.points().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["wa", "ga", "te-kudasai"]);
        assert_eq!(dataset.position("ga"), Some(2));
        assert_eq!(dataset.get("ga").map(|p| p.pattern.as_str()), Some("〜が"));
        assert_eq!(dataset.position("nope"), None);
    }

    #[test]
    fn a_page_is_a_window_and_a_search_is_bounded() {
        let points: Vec<GrammarPoint> = (0..526)
            .map(|i| point(&format!("p{i}"), "〜は", &["particle"]))
            .collect();
        let dataset = GrammarDataset::from_points(points, GrammarSource::default());
        assert_eq!(dataset.len(), 526);
        assert_eq!(dataset.example_count(), 526);
        assert_eq!(dataset.page(0, 20).len(), 20);
        assert_eq!(dataset.page(520, 20).len(), 6, "the last page is short");
        assert!(dataset.page(526, 20).is_empty(), "past the end is empty");
        assert_eq!(dataset.page(0, 20)[0].id, "p0");

        assert_eq!(dataset.search("p1", 10).len(), 10, "a search is capped");
        assert_eq!(dataset.search_total("p1"), 111, "p1, p10…p19, p100…p199");
        assert_eq!(dataset.search("", 10).len(), 0, "an empty query asks for nothing");
        assert_eq!(dataset.search_total("nothing here"), 0);
    }

    #[test]
    fn the_tags_are_reported_once_each_in_the_order_met() {
        let dataset = GrammarDataset::from_points(
            vec![
                point("a", "〜は", &["particle", "topic"]),
                point("b", "〜が", &["particle", "subject"]),
            ],
            GrammarSource::default(),
        );
        assert_eq!(dataset.tags(), vec!["particle", "topic", "subject"]);
    }

    #[test]
    fn an_artifact_that_is_not_one_is_refused_by_its_magic() {
        use std::io::Write;

        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(b"PHRAS001\x00\x00").unwrap();
        let bytes = encoder.finish().unwrap();
        let err = GrammarDataset::from_gzip_bytes(&bytes).unwrap_err();
        assert!(err.to_string().contains("not a grammar artifact"), "{err}");
    }

    #[test]
    fn an_artifact_round_trips_through_postcard_and_gzip() {
        use std::io::Write;

        let dataset = GrammarDataset::from_points(
            vec![point("wa", "〜は", &["particle", "topic"])],
            GrammarSource {
                corpus: "OpenJLPT".to_string(),
                url: "https://github.com/evanclan/OpenJLPT".to_string(),
                version: "0.3.0".to_string(),
                licence: "CC BY-SA 4.0".to_string(),
                retrieved: "2026-10-05".to_string(),
                selection: "one rule".to_string(),
                modifications: "another".to_string(),
            },
        );
        let payload = postcard::to_allocvec(&GrammarArtifact::new(
            dataset.points().to_vec(),
            dataset.source().clone(),
        ))
        .unwrap();
        let mut raw = GRAMMAR_ARTIFACT_MAGIC.to_vec();
        raw.extend_from_slice(&payload);
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(&raw).unwrap();
        let bytes = encoder.finish().unwrap();

        let decoded = GrammarDataset::from_gzip_bytes(&bytes).expect("decodes");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded.source().licence, "CC BY-SA 4.0");
        assert_eq!(
            decoded.get("wa").map(|p| p.examples[0].kana()),
            Some("わたしはがくせいです。".into())
        );
        assert!(decoded.get("nope").is_none());
    }
}
