//! Writing a sentence of one's own: free composition, and what a machine may say
//! about it.
//!
//! N19 gave the grammar course a screen, N20 made it ask a learner to *choose* the
//! pattern a sentence takes, and N22 made it ask them to *write* that pattern. All
//! three put the corpus's own sentence in front of the learner. What none of them
//! does — and what `ROADMAP_NIHONGO.md`'s known weak spots has carried as the honest
//! remaining gap since N19 — is ask for **a sentence of the learner's own**, where
//! the sentence is not the corpus's and there is no single right answer to grade
//! against.
//!
//! This module is that exercise's data half. The learner is shown one pattern and
//! writes a sentence with it; what comes back is a **checklist**, not a mark.
//!
//! # The measurement that decides what can be marked, and it decides most of this
//!
//! The obvious rule — the sentence uses the pattern when the pattern's own text is
//! in it — was measured against the corpus before it was adopted, and the corpus
//! itself fails it: **1,271 of the 1,526 examples** contain a lump of their own
//! pattern, and the other **255** do not, because the corpus writes the pattern
//! inflected or derived. `〜ている` is taught with しています, `〜に行く` with
//! 行きます, `〜がほしい` with ほしくない. A checker that demanded the pattern's
//! letters would refuse 255 of the corpus's own teaching sentences, so **it cannot
//! be a gate** and is not one: [`CompositionVerdict::found`] is evidence the screen
//! states, and the screen says plainly what it cannot see.
//!
//! The same is true of the other two checks, and both were measured rather than
//! assumed:
//!
//! * **every kanji taught** is a note, not a gate, because **two of the corpus's own
//!   sentences** write a character outside the course (几 in one, 嘘 in another);
//! * **the sentence is long enough** is the one hard bar, and its floor is the
//!   corpus's own shortest example — **six** kana/kanji characters (私は学生です。),
//!   below which the input is a word rather than a sentence.
//!
//! # What is kept, and why it is the sentence rather than a mark
//!
//! There is no right answer, so nothing is scored and the pattern's weight
//! ([`crate::grammar_drill::GrammarLog`]) is **read** here, never written: this
//! exercise cannot tell a good sentence from a poor one, and a record of a judgement
//! it cannot make would be worse than no record. What is kept is what the learner
//! actually wrote — in a **fifth file of this app's own** (`compositions.json`,
//! invariant 15), keyed by pattern, shown back on the screen that asked for it. It
//! is a notebook rather than a scoreboard.
//!
//! # The reach, which is the widest of the three grammar exercises
//!
//! The drill can ask **363 of the 526** patterns and the written-cloze exercise the
//! same 363, because both need a sentence the pattern's own text can be found in.
//! This exercise needs no sentence at all — the prompt *is* the pattern — so it
//! reaches **every one of the 526**, which is what [`size`] reports and
//! `tests/grammar_artifact.rs` pins.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::grammar::{GrammarDataset, GrammarExample, GrammarPoint};
use crate::grammar_drill::{lumps, GrammarLog};
use crate::kanji::KanjiDataset;
use crate::kana::Script;
use crate::words::is_kanji;

/// The shortest thing this exercise will keep, in kana/kanji characters.
///
/// Six, and the number is the corpus's rather than a preference: the shortest
/// sentence OpenJLPT writes for any of its 526 patterns is 私は学生です。 and
/// 危ない！止まれ！, six Japanese characters each, so six is the floor below which
/// what was typed is a word rather than a sentence. `tests/grammar_artifact.rs`
/// re-derives it from the committed artifact.
pub const MIN_COMPOSITION: usize = 6;

/// One pattern, put in front of the learner to write a sentence with.
///
/// Everything here is the artifact's: the pattern, its reading where the corpus
/// stores one, its meaning, its formation and its own examples — which are the
/// **model**, not the answer, because this exercise has no answer. `written` is the
/// learner's own: the sentences they have already written with this pattern, in the
/// order they wrote them, so the screen can show them their own writing rather than
/// a score.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionPrompt {
    /// The point's own id, which the answer is kept against.
    pub point: String,
    /// The pattern as the course writes it, `〜` slots and all.
    pub pattern: String,
    /// Its kana reading, where the corpus stores one because the pattern itself is
    /// written with a kanji. Never composed here.
    pub reading: Option<String>,
    /// What the pattern means, in the corpus's words.
    pub meaning: String,
    /// What it attaches to — `Noun + に対して`.
    pub formation: String,
    /// The corpus's note on nuance, where it has one.
    pub notes: Option<String>,
    /// The pattern's own example sentences, with the corpus's furigana: the model a
    /// learner who has just met the pattern can copy the *shape* of.
    pub examples: Vec<GrammarExample>,
    /// The sentences this learner has already written with this pattern.
    pub written: Vec<String>,
}

/// What the app could see in the sentence, and what it could not.
///
/// Five fields of observation and one decision. The decision is
/// [`CompositionVerdict::kept`] — whether the sentence is a sentence — and
/// everything else is stated rather than judged: [`CompositionVerdict::found`] is
/// the pattern's own text the sentence contains (possibly nothing, and that is not
/// necessarily wrong), [`CompositionVerdict::outside`] names characters the course
/// does not teach (a fact, not a fault), and [`CompositionVerdict::copied`]
/// notices a corpus sentence written back verbatim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionVerdict {
    /// The point the prompt named — echoed, so a screen cannot file a sentence under
    /// a pattern the learner was not shown.
    pub point: String,
    /// The pattern, as the course writes it.
    pub pattern: String,
    /// Its kana reading, where the corpus stores one.
    pub reading: Option<String>,
    /// What the pattern means.
    pub meaning: String,
    /// What it attaches to.
    pub formation: String,
    /// The corpus's note on nuance, where it has one.
    pub notes: Option<String>,
    /// The sentence as it was read back — the learner's typing with pure romaji
    /// converted to kana, and Japanese script taken as written.
    pub text: String,
    /// Which of the pattern's own lumps the sentence contains, longest first.
    ///
    /// A lump is read from the pattern **and** from its kana reading where the
    /// corpus stores one, so a learner who writes はんして for a pattern the corpus
    /// spells 〜に反して is seen. Empty is a legitimate and common answer: 255 of the
    /// corpus's own 1,526 sentences are in that state, because they inflect the
    /// pattern. This is the field the screen must not present as a verdict.
    pub found: Vec<String>,
    /// How many kana/kanji characters the sentence holds.
    pub characters: usize,
    /// Whether every character is Japanese script or the punctuation a sentence
    /// needs. False means some of what was typed could not be read — a Latin letter
    /// left inside a Japanese sentence, most often — and nothing is kept.
    pub readable: bool,
    /// Kanji the course does not teach, in the order they appear and each once.
    ///
    /// A fact about the sentence rather than a fault in it: a learner may write a
    /// character this course has no card for, and the corpus itself does it twice.
    pub outside: Vec<char>,
    /// Which of the point's own example sentences this repeats verbatim, if any.
    ///
    /// The examples travel with the prompt, so a learner can copy one — which is
    /// what reading a model first invites. Saying so is a courtesy; the sentence is
    /// kept either way.
    pub copied: Option<usize>,
    /// Whether the sentence was a sentence: readable, and at least
    /// [`MIN_COMPOSITION`] characters long. This is the whole of the judgement.
    pub kept: bool,
}

/// One sentence a learner wrote, and when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Composition {
    /// The sentence, exactly as it was read back — never rewritten here.
    pub text: String,
    /// When it was written, ISO-8601 UTC, from the caller's clock.
    pub at: String,
}

/// The learner's own sentences, keyed by the pattern they were written for.
///
/// **A fifth file of this app's own** (invariant 15), separate from `grammar.json`
/// for the reason the four files are separate from each other: what is here is not
/// a tally but writing, and a mistake in writing one must not be able to reach the
/// other. The kana drill's key is a pair of spellings, the grammar drill's is a
/// pattern id, and this one is a pattern id too — which is exactly why it is a
/// second file rather than a second map in `grammar.json`: one map holding both
/// would have no way to say which of the two a key meant.
///
/// Nothing is dropped: a learner's own writing is not a cache, and the file is small
/// (a sentence is a few dozen bytes) even after a year.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CompositionLog {
    /// The file's format, so a later build can tell what it is reading.
    pub version: u32,
    /// Keyed by [`GrammarPoint::id`]. Unknown keys — patterns a later corpus added,
    /// or a hand-edited file — are kept as they are rather than dropped.
    pub written: BTreeMap<String, Vec<Composition>>,
}

impl Default for CompositionLog {
    fn default() -> Self {
        Self {
            version: 1,
            written: BTreeMap::new(),
        }
    }
}

impl CompositionLog {
    /// An empty log: nothing written yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether anything has been written at all.
    pub fn is_empty(&self) -> bool {
        self.written.values().all(Vec::is_empty)
    }

    /// The sentences written for one pattern, in the order they were written.
    pub fn sentences(&self, id: &str) -> Vec<&str> {
        self.written
            .get(id)
            .map(|list| list.iter().map(|entry| entry.text.as_str()).collect())
            .unwrap_or_default()
    }

    /// How many sentences have been written for one pattern.
    pub fn count(&self, id: &str) -> usize {
        self.written.get(id).map(Vec::len).unwrap_or(0)
    }

    /// Keep one sentence, and answer how many this pattern now holds.
    ///
    /// The caller supplies the clock: this type decides nothing about time, which is
    /// what lets a test assert the record rather than the date.
    pub fn add(&mut self, id: &str, text: &str, at: &str) -> usize {
        let list = self.written.entry(id.to_string()).or_default();
        list.push(Composition {
            text: text.to_string(),
            at: at.to_string(),
        });
        list.len()
    }

    /// How many sentences have been written in all.
    pub fn total(&self) -> usize {
        self.written.values().map(Vec::len).sum()
    }

    /// How many patterns have at least one sentence.
    pub fn patterns(&self) -> usize {
        self.written.values().filter(|list| !list.is_empty()).count()
    }
}

/// What the composition exercise can ask, for the screen's own line about it.
///
/// The same three-number shape the other two grammar exercises report is not used
/// here: this one asks about a point, not about a sentence of it, so there is no
/// count of sentences to state. What it has is the reach — and the reach is the
/// whole course, which is the fact the screen should carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CompositionSize {
    /// How many of the course's points can be written about: all of them.
    pub points: usize,
    /// How many points the course holds at all.
    pub total: usize,
}

/// The fixed lumps of a pattern, read from its written form **and** from its kana
/// reading where the corpus stores one.
///
/// [`lumps`] is the drill's own rule — the `/` alternatives, each bracketed group
/// expanded both ways, each `〜`-separated fragment — and this adds one thing to it
/// that only free composition needs: **the reading is read the same way**. The drill
/// and the written-cloze exercise know the sentence and ask for the pattern, so they
/// never need to recognise はんして as 反して; a learner writing their own sentence
/// may write either, and both are the pattern. `〜の方が〜より` gives の方が and
/// より as well as のほうが and より, so a sentence written entirely in kana is read.
pub fn own_lumps(point: &GrammarPoint) -> Vec<String> {
    let mut out = lumps(&point.pattern);
    if let Some(reading) = point.reading.as_deref() {
        out.extend(lumps(reading));
    }
    out.sort_by_key(|lump| std::cmp::Reverse(lump.chars().count()));
    out.dedup();
    out
}

/// What one pattern puts in front of the learner.
pub fn prompt_for(point: &GrammarPoint, log: &CompositionLog) -> CompositionPrompt {
    CompositionPrompt {
        point: point.id.clone(),
        pattern: point.pattern.clone(),
        reading: point.reading.clone(),
        meaning: point.meaning.clone(),
        formation: point.formation.clone(),
        notes: point.notes.clone(),
        examples: point.examples.clone(),
        written: log
            .sentences(&point.id)
            .into_iter()
            .map(str::to_string)
            .collect(),
    }
}

/// Draw the next pattern to write about: the whole course, weighted by the
/// patterns this learner has missed in the other two exercises.
///
/// **The pool is every point of the course** — 526 of 526 — because the prompt needs
/// no sentence and so nothing can make a pattern unaskable here, which is the one
/// reach the other two grammar exercises do not have. The weighting is
/// [`GrammarLog`]'s and is the same weight the drill draws by, so the patterns a
/// learner keeps missing come up here too; nothing is written back to it, because
/// this exercise has no verdict to record (this module's documentation says why).
///
/// `roll` is the caller's, for the reason every draw in this crate takes one: a
/// test can name the answer it wants instead of waiting for it.
pub fn draw(
    dataset: &GrammarDataset,
    weights: &GrammarLog,
    log: &CompositionLog,
    roll: f64,
) -> Option<CompositionPrompt> {
    let pool: Vec<&GrammarPoint> = dataset.points().iter().collect();
    let point = weights.pick(&pool, roll)?;
    Some(prompt_for(point, log))
}

/// What the exercise can ask of a dataset, in numbers.
pub fn size(dataset: &GrammarDataset) -> CompositionSize {
    CompositionSize {
        points: dataset.len(),
        total: dataset.len(),
    }
}

/// Read a typed sentence back: pure romaji becomes kana, Japanese script is taken as
/// it stands.
///
/// [`crate::input::produce_kana`] is the rule the other exercises use and this is
/// deliberately not it: that function treats *any* string with no kana as romaji,
/// which would turn a sentence written only in kanji into nothing at all. A whole
/// sentence is not a reading — it may be written with kanji, and the kana in it are
/// the learner's own — so kanji counts as typed language here.
///
/// What is **not** handled, deliberately and with the failure stated on screen: a
/// Latin word left *inside* a Japanese sentence (`私はstudentです。`). Splitting a
/// sentence into runs and converting each would be guessing at where a word starts,
/// which is the mistake this project has already recorded once for the vocabulary
/// (invariant 40); a learner is asked to write the whole thing in one script and
/// told when they have not.
fn read_back(typed: &str) -> String {
    let typed = typed.trim();
    if typed.chars().any(|ch| Script::of(ch).is_some() || is_kanji(ch)) {
        return typed.to_string();
    }
    // Pure romaji: a sentence is typed as separate words and Japanese is written
    // without the spaces between them, so each word is converted and the results are
    // joined. A word the table cannot read — `hanshite.` with a full stop, or a
    // Latin name — makes the whole attempt unreadable rather than half-converted, and
    // `readable` is what says so on screen.
    let mut out = String::new();
    for word in typed.split_whitespace() {
        match crate::input::to_kana(word) {
            Ok(kana) => out.push_str(&kana),
            Err(_) => return typed.to_string(),
        }
    }
    out
}

/// Whether a character belongs in a Japanese sentence.
///
/// Kana, kanji and `ー` are the language; the rest is the punctuation and the space a
/// sentence may carry. Latin letters are **not** on the list, which is what makes
/// [`CompositionVerdict::readable`] false for them.
fn is_japanese(ch: char) -> bool {
    Script::of(ch).is_some()
        || is_kanji(ch)
        || matches!(
            ch,
            '。' | '、'
                | '！'
                | '？'
                | '「'
                | '」'
                | '『'
                | '』'
                | '（'
                | '）'
                | '・'
                | '〜'
                | '～'
                | '…'
                | '—'
                | ' '
                | '　'
                | '.'
                | ','
                | '!'
                | '?'
                | ':'
                | ';'
                | '-'
                | '('
                | ')'
                | '\''
                | '"'
                | '0'..='9'
                | '０'..='９'
        )
}

/// How many kana/kanji characters a sentence holds: the measure the length bar uses.
fn japanese_len(text: &str) -> usize {
    text.chars()
        .filter(|ch| Script::of(*ch).is_some() || is_kanji(*ch))
        .count()
}

/// What may be handed to the voice for a sentence the learner wrote: the sentence as
/// [`read_back`] reads it, when that is Japanese at all.
///
/// N24 widened what the app may speak — a learner's own composed sentence is now one
/// of the things `HANDOVER_NIHONGO.md` invariant 27 allows — and this is the whole of
/// the widening, in one place rather than in the screen that draws the button:
///
/// * **the read-back form is spoken, not the typing.** A learner who writes
///   `inu no houga neko yori suki desu` has written いぬのほうがねこよりすきです, and
///   handing the synthesiser the romaji would have a Japanese voice read Latin
///   letters;
/// * **and nothing is spoken that is not Japanese.** A mixed input the app did not
///   read — a Latin word left inside a Japanese sentence — is `None`, so the button
///   reports that rather than saying something the learner did not write.
///
/// Empty input, and input that reads back to nothing, are `None` too.
pub fn spoken_form(typed: &str) -> Option<String> {
    let text = read_back(typed);
    (!text.is_empty() && text.chars().all(is_japanese)).then_some(text)
}

/// Read a sentence the learner wrote, and say what the app can see in it.
///
/// **This is the whole of the marking, and it is not a grade.** The only judgement
/// is [`CompositionVerdict::kept`]: is this a sentence at all — readable, and at
/// least [`MIN_COMPOSITION`] characters of kana or kanji. Everything else is
/// reported: the pattern's own text where it appears, the characters the course does
/// not teach, and the corpus sentence if one was written back. Whether the sentence
/// is *good* — grammatical, natural, using the pattern in the right sense — is not
/// something this function can see, and no field here pretends otherwise.
pub fn verdict_for(
    point: &GrammarPoint,
    kanji: &KanjiDataset,
    typed: &str,
) -> CompositionVerdict {
    let text = read_back(typed);
    let found: Vec<String> = own_lumps(point)
        .into_iter()
        .filter(|lump| !lump.is_empty() && text.contains(lump.as_str()))
        .collect();
    let mut outside: Vec<char> = Vec::new();
    for ch in text.chars().filter(|ch| is_kanji(*ch)) {
        if kanji.get(ch).is_none() && !outside.contains(&ch) {
            outside.push(ch);
        }
    }
    let copied = point.examples.iter().position(|example| example.ja == text);
    let characters = japanese_len(&text);
    let readable = !text.is_empty() && text.chars().all(is_japanese);
    CompositionVerdict {
        point: point.id.clone(),
        pattern: point.pattern.clone(),
        reading: point.reading.clone(),
        meaning: point.meaning.clone(),
        formation: point.formation.clone(),
        notes: point.notes.clone(),
        text,
        found,
        characters,
        readable,
        outside,
        copied,
        kept: readable && characters >= MIN_COMPOSITION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::{parse_furigana, GrammarSource};
    use crate::kanji::{Kanji, KanjiSource};
    use hanzi_core::Point;

    fn example(ja: &str, markup: &str) -> GrammarExample {
        GrammarExample {
            ja: ja.to_string(),
            en: "t".to_string(),
            ruby: parse_furigana(ja, markup).expect("parses"),
            words: Vec::new(),
        }
    }

    fn point(id: &str, pattern: &str) -> GrammarPoint {
        GrammarPoint {
            id: id.to_string(),
            pattern: pattern.to_string(),
            reading: None,
            romaji: id.replace('-', " "),
            meaning: "meaning".to_string(),
            formation: "Noun + は".to_string(),
            notes: None,
            tags: vec!["particle".to_string()],
            examples: vec![example("私は学生です。", "{私|わたし}は{学生|がくせい}です。")],
        }
    }

    /// A course of real shape: the characters these tests write with are taught,
    /// and nothing else is. 顎 is deliberately left out — it is Japanese the course
    /// has no card for, which is the fact the outside check reports.
    fn kanji() -> KanjiDataset {
        let taught = |ch: char| Kanji {
            ch,
            grade: 1,
            stroke_count: 1,
            frequency: None,
            radical: '一',
            radical_note: None,
            radical_number: 1,
            on: Vec::new(),
            kun: Vec::new(),
            meanings: Vec::new(),
            nanori: Vec::new(),
            decomposition: String::new(),
            outlines: vec!["M0,0".to_string()],
            medians: vec![vec![Point::new(0.0, 0.0)]],
        };
        KanjiDataset::from_kanji(
            [
                '犬', '猫', '私', '学', '生', '日', '彼', '予', '想', '反', '試', '合', '延', '期', '痛', '言',
            ]
                .into_iter()
                .map(taught)
                .collect(),
            KanjiSource::default(),
        )
    }

    #[test]
    fn a_pattern_is_recognised_in_the_form_the_course_writes_and_in_kana() {
        let mut p = point("ni-hanshite", "〜に反して");
        assert_eq!(own_lumps(&p), vec!["に反して"]);
        p.reading = Some("〜にはんして".to_string());
        assert_eq!(own_lumps(&p), vec!["にはんして", "に反して"]);
    }

    #[test]
    fn the_reading_of_a_two_lump_pattern_is_read_as_its_own_lumps() {
        let mut p = point("no-hou-ga-yori", "〜の方が〜より");
        p.reading = Some("〜のほうが〜より".to_string());
        let lumps = own_lumps(&p);
        for lump in ["の方が", "より", "のほうが"] {
            assert!(lumps.contains(&lump.to_string()), "{lump:?} missing from {lumps:?}");
        }
    }

    #[test]
    fn a_sentence_that_uses_the_pattern_is_seen_to() {
        let p = point("ni-hanshite", "〜に反して");
        let verdict = verdict_for(&p, &kanji(), "彼の予想に反して、試合は延期された。");
        assert_eq!(verdict.found, vec!["に反して"]);
        assert!(verdict.kept, "a real sentence: {verdict:?}");
        assert!(verdict.readable);
        assert!(verdict.outside.is_empty(), "every character is one the course has");
        assert_eq!(verdict.copied, None);
    }

    /// The measurement the whole exercise turns on: an inflected pattern is not
    /// seen, and the sentence is kept anyway, because the check is evidence rather
    /// than a gate. 〜ている is the corpus's own example of it — it teaches the
    /// pattern with しています.
    #[test]
    fn an_inflected_pattern_is_not_seen_and_the_sentence_is_kept_anyway() {
        let p = point("te-iru", "〜ている");
        let verdict = verdict_for(&p, &kanji(), "弟は部屋でゲームをしています。");
        assert!(verdict.found.is_empty(), "しています is not the text 〜ている writes");
        assert!(verdict.kept, "and that is not held against the learner: {verdict:?}");
    }

    #[test]
    fn a_short_input_is_not_a_sentence_and_nothing_is_kept() {
        let p = point("wa", "〜は");
        let short = verdict_for(&p, &kanji(), "はい。");
        assert_eq!(short.characters, 2);
        assert!(short.readable, "はい。 is Japanese; it is just not a sentence");
        assert!(!short.kept);
        assert_eq!(short.found, vec!["は"], "the pattern is in it, and it is still not kept");

        // The floor is the corpus's own shortest sentence, and it is inclusive.
        let floor = verdict_for(&p, &kanji(), "私は学生です。");
        assert_eq!(floor.characters, MIN_COMPOSITION);
        assert!(floor.kept);
    }

    #[test]
    fn romaji_is_read_back_as_kana_and_japanese_script_is_left_alone() {
        let mut p = point("ni-hanshite", "〜に反して");
        p.reading = Some("〜にはんして".to_string());
        let typed = verdict_for(&p, &kanji(), "kare no yosou ni hanshite");
        assert_eq!(typed.text, "かれのよそうにはんして", "words joined, as Japanese is written");
        assert_eq!(typed.found, vec!["にはんして"], "the reading's own lump is seen");
        assert!(typed.kept, "{typed:?}");

        // A sentence written only in kanji is not romaji and must survive: this is
        // the case `produce_kana` alone would turn into an empty string.
        let written = verdict_for(&p, &kanji(), "予想反。");
        assert_eq!(written.text, "予想反。");

        // A word the table cannot read is not half-converted: the attempt is left as
        // typed, and `readable` is what says it could not be read.
        let stuck = verdict_for(&p, &kanji(), "hanshite.");
        assert_eq!(stuck.text, "hanshite.");
        assert!(!stuck.readable);
        assert!(!stuck.kept);
    }

    #[test]
    fn a_latin_word_inside_a_japanese_sentence_is_not_read_and_says_so() {
        let p = point("wa", "〜は");
        let mixed = verdict_for(&p, &kanji(), "私はstudentです。");
        assert!(!mixed.readable, "the app does not guess where a Latin word ends");
        assert!(!mixed.kept);
        assert!(!mixed.text.is_empty(), "and it says what it read rather than nothing");
    }

    #[test]
    fn a_character_the_course_does_not_teach_is_named_rather_than_refused() {
        let p = point("wa", "〜は");
        // 顎 is Japanese the course has no card for; every other character is taught.
        let verdict = verdict_for(&p, &kanji(), "彼は顎が痛いと言った。");
        assert_eq!(verdict.outside, vec!['顎'], "named, and the taught character is not");
        assert!(verdict.kept, "a fact about the sentence, not a fault in it: {verdict:?}");
    }

    #[test]
    fn a_corpus_sentence_written_back_is_noticed() {
        let p = point("wa", "〜は");
        let verdict = verdict_for(&p, &kanji(), "私は学生です。");
        assert_eq!(verdict.copied, Some(0));
        assert!(verdict.kept, "copying is noticed, not punished");
    }

    #[test]
    fn the_reading_survives_into_the_verdict_for_the_reveal() {
        let mut p = point("yotei", "〜予定");
        p.reading = Some("〜よてい".to_string());
        p.notes = Some("a plan".to_string());
        let verdict = verdict_for(&p, &kanji(), "来週旅行する予定です。");
        assert_eq!(verdict.pattern, "〜予定");
        assert_eq!(verdict.reading.as_deref(), Some("〜よてい"));
        assert_eq!(verdict.notes.as_deref(), Some("a plan"));
        assert_eq!(verdict.formation, "Noun + は");
        assert_eq!(verdict.point, "yotei");
    }

    #[test]
    fn the_prompt_carries_the_pattern_its_model_and_what_this_learner_has_written() {
        let p = point("wa", "〜は");
        let mut log = CompositionLog::new();
        assert!(log.is_empty());
        assert_eq!(log.add("wa", "私は学生です。", "2026-10-07T00:00:00Z"), 1);
        assert_eq!(log.add("wa", "今日は天気がいいです。", "2026-10-07T00:01:00Z"), 2);
        assert_eq!(log.add("ga", "雨が降っています。", "2026-10-07T00:02:00Z"), 1);
        assert!(!log.is_empty());
        assert_eq!(log.total(), 3);
        assert_eq!(log.patterns(), 2);
        assert_eq!(log.count("wa"), 2);
        assert_eq!(log.count("nope"), 0);
        assert_eq!(
            log.sentences("wa"),
            vec!["私は学生です。", "今日は天気がいいです。"],
            "in the order they were written"
        );

        let prompt = prompt_for(&p, &log);
        assert_eq!(prompt.point, "wa");
        assert_eq!(prompt.pattern, "〜は");
        assert_eq!(prompt.examples.len(), 1, "the model travels with the prompt");
        assert_eq!(prompt.written.len(), 2);
        assert_eq!(prompt.written[0], "私は学生です。");
        // A pattern nobody has written about shows an empty list rather than
        // somebody else's sentences.
        assert!(prompt_for(&point("ka", "〜か"), &log).written.is_empty());
    }

    #[test]
    fn the_log_round_trips_through_the_encoding_its_own_file_uses() {
        let mut log = CompositionLog::new();
        log.add("wa", "私は学生です。", "2026-10-07T00:00:00Z");
        log.add("に-ほ", "にほんです。", "2026-10-07T00:01:00Z");
        let bytes = postcard::to_allocvec(&log).expect("serialises");
        let read: CompositionLog = postcard::from_bytes(&bytes).expect("reads back");
        assert_eq!(read, log);
        assert_eq!(read.version, 1);
        assert_eq!(read.sentences("に-ほ"), vec!["にほんです。"]);
        // The default is what a file missing its version reads as: the shape a
        // hand-edited or older `compositions.json` arrives in.
        assert_eq!(CompositionLog::default().version, 1);
    }

    #[test]
    fn every_pattern_of_the_course_is_in_the_pool_and_the_draw_is_weighted() {
        let dataset = GrammarDataset::from_points(
            vec![point("wa", "〜は"), point("ga", "〜が"), point("o", "〜を")],
            GrammarSource::default(),
        );
        assert_eq!(size(&dataset), CompositionSize { points: 3, total: 3 });

        let log = CompositionLog::new();
        let mut weights = GrammarLog::new();
        // Weights only, no tallies: a pattern this learner has missed is likelier.
        weights.record("ga", false);
        weights.record("ga", false);
        let drawn = draw(&dataset, &weights, &log, 0.0).expect("a pattern");
        assert_eq!(drawn.point, "wa", "the first weight span is the first pattern");
        let drawn = draw(&dataset, &weights, &log, 1.0).expect("a pattern");
        assert_eq!(drawn.point, "o", "the last span is the last pattern");
        // An empty course and a course with no drawable point are answers, not panics.
        let empty = GrammarDataset::default();
        assert!(draw(&empty, &weights, &log, 0.5).is_none());
    }

    /// N24's widening of invariant 27: a composed sentence may be spoken, and what is
    /// spoken is what the app read the typing as — never the typing itself.
    #[test]
    fn what_may_be_spoken_is_the_sentence_read_back_and_only_when_it_is_japanese() {
        assert_eq!(
            spoken_form("inu no houga neko yori suki desu").as_deref(),
            Some("いぬのほうがねこよりすきです"),
            "romaji is spoken as the kana it reads as"
        );
        assert_eq!(
            spoken_form("いぬのほうがねこよりすきです。").as_deref(),
            Some("いぬのほうがねこよりすきです。"),
            "kana is spoken as it stands"
        );
        assert_eq!(spoken_form("   ").as_deref(), None, "nothing to say");
        assert_eq!(spoken_form("").as_deref(), None);
        assert_eq!(
            spoken_form("私はstudentです。").as_deref(),
            None,
            "a sentence the app could not read is not handed to a Japanese voice"
        );
    }

    #[test]
    fn a_verdict_is_built_from_an_empty_dataset_without_panicking() {
        let p = point("wa", "〜は");
        let none = KanjiDataset::default();
        let verdict = verdict_for(&p, &none, "私は学生です。");
        assert_eq!(verdict.outside, vec!['私', '学', '生']);
        assert!(verdict.kept);
        // Nothing typed at all is readable by nobody's standard.
        let blank = verdict_for(&p, &none, "   ");
        assert!(!blank.readable);
        assert!(!blank.kept);
        assert_eq!(blank.characters, 0);
    }
}
