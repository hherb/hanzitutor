//! The kana tutor's webview-facing surface.
//!
//! The shape is the same as the other two apps': a thin `#[tauri::command]` layer
//! over methods on [`AppState`], so the whole interface can be driven and
//! asserted on from a test without opening a window. `tests/ipc_contract.rs`
//! does exactly that, and it is what stops the JSON the interface reads from
//! drifting away from the JSON this returns.
//!
//! Everything the app teaches is **embedded**: the kana artifact is compiled into
//! the binary with `include_bytes!`. There is no network path in this crate at
//! all — no download, no model, no sync — which is why it has no plugin
//! permissions in `capabilities/default.json`.

// The two that share a name with a command are aliased, so that `fn lessons`
// below is the command and `build_lessons` is the course it serves.
pub mod licences;
pub mod store;

use licences::{AppInfo, LicenceNotice};
use nihongo_core::{
    band_name, confusions_for, find_pair, lessons as build_lessons, normalise_to_hiragana,
    pair_key, reading, to_kana, to_kana_in, yoon as build_yoon, Confusable, ConfusionLog,
    GradeOptions, GradeReport, KanaDataset, Passage, PassageDataset, PassageToken, Point, Ruby,
    Script, Word, WordDataset, CONFUSABLE,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use store::ConfusionStore;
use tauri::{Manager, State};

/// The datasets, loaded once and shared by every command, and the drill's memory
/// of what this learner gets wrong.
///
/// All four artifacts are **embedded** — `include_bytes!`, not read from a path —
/// so there is no data directory to find, no file to go missing, and no filesystem
/// permission for the webview to hold. The kanji artifact is the largest at 3 MB
/// and is not served by any command yet; it is here because the vocabulary needs
/// the characters it holds to be gradeable, which is N8's first half.
pub struct AppState {
    kana: KanaDataset,
    words: WordDataset,
    passages: PassageDataset,
    /// Behind a `Mutex` because Tauri hands every command a shared `&AppState`
    /// and recording an answer is a write. Contention is nil: one learner, one
    /// window, and a lock held for the microseconds a JSON write takes.
    drill: Mutex<ConfusionStore>,
}

impl AppState {
    /// Load the committed artifact, with the drill's record kept in memory only.
    ///
    /// This is what a test wants and what the app falls back to when the platform
    /// will not name a data directory.
    pub fn load() -> Self {
        Self::with_store(ConfusionStore::in_memory())
    }

    /// Load the committed artifact and the learner's record from `dir`.
    pub fn load_at(dir: impl Into<PathBuf>) -> Self {
        Self::with_store(ConfusionStore::at_dir(dir.into()))
    }

    fn with_store(drill: ConfusionStore) -> Self {
        macro_rules! artifact {
            ($name:literal) => {
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../crates/nihongo-core/data/",
                    $name
                ))
            };
        }
        Self {
            kana: KanaDataset::from_gzip_bytes(artifact!("kana.bin.gz"))
                .expect("the committed kana artifact decodes"),
            words: WordDataset::from_gzip_bytes(artifact!("words.bin.gz"))
                .expect("the committed words artifact decodes"),
            passages: PassageDataset::from_gzip_bytes(artifact!("passages.bin.gz"))
                .expect("the committed passages artifact decodes"),
            drill: Mutex::new(drill),
        }
    }

    pub fn dataset(&self) -> &KanaDataset {
        &self.kana
    }

    /// The learner's record, for a caller that wants to show or assert on it.
    pub fn log(&self) -> ConfusionLog {
        self.drill
            .lock()
            .expect("the drill's store is not poisoned")
            .log()
            .clone()
    }

    /// What is in the course, in numbers.
    pub fn stats(&self) -> DatasetStats {
        let hiragana = self.kana.of_script(Script::Hiragana).count();
        let katakana = self.kana.of_script(Script::Katakana).count();
        let lesson_count = build_lessons(&self.kana, Script::Hiragana).len()
            + build_lessons(&self.kana, Script::Katakana).len();
        DatasetStats {
            kana: self.kana.len(),
            hiragana,
            katakana,
            lessons: lesson_count,
            words: self.words.len(),
            passages: self.passages.len(),
            strokes: self
                .kana
                .kana()
                .iter()
                .map(|k| k.stroke_count as usize)
                .sum(),
        }
    }

    /// The course for one script, in teaching order.
    pub fn lessons(&self, script: Script) -> Vec<LessonView> {
        build_lessons(&self.kana, script)
            .into_iter()
            .map(|lesson| LessonView {
                count: lesson.kana.len(),
                key: lesson.key,
                title: lesson.title,
                kana: lesson.kana,
                voiced: lesson.voiced,
            })
            .collect()
    }

    /// One kana, with everything a practice screen needs: the geometry it draws
    /// and grades against, how it is read, and what it is confused with.
    pub fn kana(&self, ch: char) -> Result<KanaView, String> {
        let kana = self
            .kana
            .get(ch)
            .ok_or_else(|| format!("{ch} (U+{:04X}) is not in the kana set", ch as u32))?;
        let reading = reading(ch).ok_or_else(|| format!("{ch} has no reading"))?;
        Ok(KanaView {
            ch: kana.ch,
            script: kana.script.name().to_string(),
            stroke_count: kana.stroke_count,
            practisable: kana.is_practisable(),
            romaji: reading.spellings().map(str::to_string).collect(),
            hepburn: reading.hepburn.first().copied().unwrap_or_default().to_string(),
            silent: reading.is_silent(),
            outlines: kana.outlines.clone(),
            medians: kana.medians.clone(),
            confusions: self.confusions(ch),
        })
    }

    /// Grade a handwritten attempt. The scoring is `hanzi-core`'s, unchanged:
    /// shape, placement, ink and order, against the corrected stroke geometry.
    pub fn grade(
        &self,
        ch: char,
        strokes: &[Vec<Point>],
        options: &GradeOptions,
    ) -> Result<GradeReport, String> {
        let kana = self
            .kana
            .get(ch)
            .ok_or_else(|| format!("{ch} (U+{:04X}) is not in the kana set", ch as u32))?;
        if !kana.is_practisable() {
            return Err(format!("{ch} has no stroke geometry to grade against"));
        }
        Ok(nihongo_core::grade_with_outlines(
            kana.reference_medians(),
            &kana.outlines,
            strokes,
            options,
        ))
    }

    /// The kana a given kana is confused with, from the classic set.
    pub fn confusions(&self, ch: char) -> Vec<ConfusionView> {
        confusions_for(&self.kana, ch)
            .into_iter()
            .map(|pair: Confusable| {
                let other = if pair.a == ch { pair.b } else { pair.a };
                ConfusionView {
                    ch: other,
                    tell: pair.tell.to_string(),
                }
            })
            .collect()
    }

    /// The pairs the drill can actually ask: **both** kana in the dataset and both
    /// with a reading to prompt with. A pair that fails this is skipped rather than
    /// asked with an empty prompt, which is why the field is filtered here instead
    /// of being assumed complete.
    fn askable_pairs(&self) -> Vec<Confusable> {
        CONFUSABLE
            .iter()
            .copied()
            .filter(|pair| self.pair_is_askable(pair))
            .collect()
    }

    /// The pair the drill should ask about next, weighted towards the ones this
    /// learner gets wrong.
    ///
    /// Two rolls, because two independent things are being decided: which pair,
    /// and which of the pair is the one being asked for. Both are supplied rather
    /// than drawn here so that the whole decision is deterministic in a test; the
    /// command above passes two rolls from the clock.
    pub fn next_drill_question_with_rolls(&self, pair_roll: f64, side_roll: f64) -> Option<DrillQuestion> {
        let askable = self.askable_pairs();
        let log = self
            .drill
            .lock()
            .expect("the drill's store is not poisoned");
        let pair = *log.log().pick(&askable, pair_roll)?;
        drop(log);

        // Which way round to ask. Both members are askable, so this cannot fail;
        // it returns `None` rather than panicking all the same, because a kana
        // dataset is data and data is allowed to be wrong.
        let (target, other) = if side_roll < 0.5 {
            (pair.a, pair.b)
        } else {
            (pair.b, pair.a)
        };
        let hepburn = reading(target)?.hepburn.first()?.to_string();

        Some(DrillQuestion {
            pair: pair_key(&pair),
            ch: target,
            hepburn,
            // Exactly the two kana of the pair, so that every answer says
            // something unambiguous about *this* pair: which of these two shapes
            // is the reading. The wider four-option question the drill used to ask
            // tested more at once and taught less — a miss could not be attributed
            // to a pair, which is precisely what has to be remembered.
            options: vec![target, other],
            tell: pair.tell.to_string(),
        })
    }

    /// The next question, with the randomness supplied by the clock.
    pub fn next_drill_question(&self) -> Option<DrillQuestion> {
        self.next_drill_question_with_rolls(roll(), roll())
    }

    /// Record what the learner answered, and say what the pair's record now is.
    ///
    /// The caller says which pair it was asked about, which kana was wanted and
    /// which was picked; **correctness is decided here**, not sent by the
    /// interface. A client that could post `correct: true` would be a client that
    /// could lie to itself, and the file is meant to be worth reading.
    pub fn record_drill_answer(
        &self,
        pair: &str,
        target: char,
        picked: char,
    ) -> Result<DrillTally, String> {
        let pair = find_pair(pair).ok_or_else(|| {
            format!("{pair:?} is not one of the confusion pairs, so there is nothing to record")
        })?;
        if target != pair.a && target != pair.b {
            return Err(format!("{target} is not in {}", pair_key(pair)));
        }
        if picked != pair.a && picked != pair.b {
            return Err(format!(
                "{picked} is neither of the two answers {} offers",
                pair_key(pair)
            ));
        }
        let key = pair_key(pair);
        let tally = self
            .drill
            .lock()
            .expect("the drill's store is not poisoned")
            .record(&key, picked == target)
            .map_err(|err| format!("the answer was counted but could not be saved: {err}"))?;
        Ok(DrillTally {
            pair: key,
            asked: tally.asked,
            correct: tally.correct,
            wrong: tally.wrong,
            weight: tally.weight(),
        })
    }

    /// Whether a pair can be asked at all: both kana are in the dataset and both
    /// have a reading to prompt with.
    fn pair_is_askable(&self, pair: &Confusable) -> bool {
        self.kana_has_a_prompt(pair.a) && self.kana_has_a_prompt(pair.b)
    }

    /// Whether a kana can be *asked about*: it is in the dataset and has a
    /// non-empty Hepburn reading for the prompt.
    fn kana_has_a_prompt(&self, ch: char) -> bool {
        self.kana.get(ch).is_some() && reading(ch).is_some_and(|r| !r.hepburn.is_empty())
    }

    /// The yōon digraphs for one script — the pairs that make one mora.
    pub fn yoon(&self, script: Script) -> Vec<YoonView> {
        build_yoon(script)
            .into_iter()
            .map(|y| YoonView {
                key: y.key,
                display: y.display,
                hepburn: y.hepburn,
                kunrei: y.kunrei,
                kana: y.kana,
            })
            .collect()
    }

    /// The ladder, one entry per band, with the counts a screen shows.
    ///
    /// The names come from `nihongo_core::band_name` rather than from this
    /// interface, because the ladder is *this project's* derivation and the
    /// interface has to say so: there has been no official JLPT list since 2010,
    /// and a band labelled as one would be a lie about the data.
    pub fn word_bands(&self) -> Vec<BandView> {
        self.words
            .band_counts()
            .into_iter()
            .map(|(band, words)| BandView {
                band,
                name: band_name(band).to_string(),
                words,
            })
            .collect()
    }

    /// One page of a band's words, in course order.
    ///
    /// Paged rather than whole because band 7 holds nearly five thousand words and
    /// the interface shows a list: a single payload would be a megabyte of JSON to
    /// draw sixty rows.
    pub fn words_in_band(&self, band: u8, offset: usize, limit: usize) -> WordPage {
        let total = self.words.of_band(band).count();
        let words = self
            .words
            .of_band(band)
            .skip(offset)
            .take(limit.clamp(1, MAX_WORD_PAGE))
            .map(word_view)
            .collect();
        WordPage {
            band,
            total,
            offset,
            words,
        }
    }

    /// One word, by its text and its reading.
    pub fn word(&self, text: &str, reading: &str) -> Result<WordView, String> {
        self.words
            .find(text, reading)
            .map(word_view)
            .ok_or_else(|| format!("{text} ({reading}) is not in the vocabulary"))
    }

    /// One word, by its text alone.
    ///
    /// What a tapped passage token has: the token carries the word's text, because
    /// that is what the sentence shows, and the token's own reading is the
    /// *surface's* — 行き is read いき while the word is 行く, read いく — so it
    /// cannot identify the entry. Six texts in the vocabulary are read two ways;
    /// this returns the one the course reaches first, which is the earlier band and
    /// the more frequent.
    pub fn word_of_text(&self, text: &str) -> Result<WordView, String> {
        self.words
            .of_text(text)
            .map(word_view)
            .ok_or_else(|| format!("{text} is not in the vocabulary"))
    }

    /// Check a typed reading against a word.
    ///
    /// The word is graded **as a word**: its own reading, taken from the
    /// dictionary, and never assembled from its characters — which is the whole
    /// reason 大人 is おとな and not だいじん. Romaji is accepted, and so is kana
    /// typed directly, because a learner who can already read kana should not have
    /// to transliterate to answer.
    pub fn check_word(&self, text: &str, reading: &str, typed: &str) -> Result<ReadingCheck, String> {
        let word = self
            .words
            .find(text, reading)
            .ok_or_else(|| format!("{text} ({reading}) is not in the vocabulary"))?;

        let typed = typed.trim();
        let produced = if typed.chars().any(is_kana) {
            typed.to_string()
        } else {
            to_kana(typed).unwrap_or_default()
        };
        let correct = !produced.is_empty()
            && normalise_to_hiragana(&produced) == normalise_to_hiragana(&word.reading);
        Ok(ReadingCheck { correct, produced })
    }

    /// The passages, as a list a screen can offer.
    pub fn passages(&self) -> Vec<PassageSummary> {
        self.passages
            .passages()
            .iter()
            .map(|passage| PassageSummary {
                key: passage.key.clone(),
                title: passage.title.clone(),
                gloss: passage.gloss.clone(),
                lines: passage.lines.len(),
                tokens: passage.tokens().count(),
            })
            .collect()
    }

    /// One passage, segmented, with the reading over every kanji.
    pub fn passage(&self, key: &str) -> Result<PassageView, String> {
        self.passages
            .get(key)
            .map(passage_view)
            .ok_or_else(|| format!("{key} is not one of the passages"))
    }
}

/// The largest page a caller may ask for.
///
/// Bounded here rather than trusted from the interface: `limit` crosses the IPC
/// boundary, and an unbounded one would let a stray number build a payload the
/// window cannot draw.
const MAX_WORD_PAGE: usize = 200;

/// Whether a string is kana, for telling a typed reading from typed romaji.
fn is_kana(ch: char) -> bool {
    matches!(ch as u32, 0x3041..=0x309F | 0x30A1..=0x30FA | 0x30FC)
}

fn word_view(word: &Word) -> WordView {
    WordView {
        text: word.text.clone(),
        reading: word.reading.clone(),
        meaning: word.meaning.clone(),
        band: word.band,
        band_name: band_name(word.band).to_string(),
        nf: word.nf,
        furigana: word.furigana.clone(),
    }
}

fn passage_view(passage: &Passage) -> PassageView {
    PassageView {
        key: passage.key.clone(),
        title: passage.title.clone(),
        gloss: passage.gloss.clone(),
        lines: passage
            .lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|token: &PassageToken| PassageTokenView {
                        surface: token.surface.clone(),
                        rt: token.rt.clone(),
                        word: token.word.clone(),
                    })
                    .collect()
            })
            .collect(),
    }
}

/// What the course contains.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetStats {
    pub kana: usize,
    pub hiragana: usize,
    pub katakana: usize,
    pub lessons: usize,
    /// How many words the vocabulary holds, so the header can say so without a
    /// second call.
    pub words: usize,
    /// How many reading passages are shipped.
    pub passages: usize,
    pub strokes: usize,
}

/// One lesson, as the sidebar lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonView {
    pub key: String,
    pub title: String,
    pub kana: Vec<char>,
    pub voiced: bool,
    pub count: usize,
}

/// One kana, as a practice screen uses it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KanaView {
    pub ch: char,
    pub script: String,
    pub stroke_count: u8,
    pub practisable: bool,
    /// Every spelling a learner might type, Hepburn first.
    pub romaji: Vec<String>,
    /// The one to show as *the* reading.
    pub hepburn: String,
    /// True for っ and ー, which have no sound of their own.
    pub silent: bool,
    /// SVG path data in font space, one per taught stroke, in stroke order.
    pub outlines: Vec<String>,
    /// Centre-lines in display space, for the faint guide and for grading.
    pub medians: Vec<Vec<Point>>,
    pub confusions: Vec<ConfusionView>,
}

/// A kana this one is mistaken for.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfusionView {
    pub ch: char,
    pub tell: String,
}

/// One question for the discrimination drill.
///
/// The pair travels with the question because the answer is recorded against the
/// *pair*, and the asker is the only thing that knows which pair it chose. The
/// component is free to shuffle `options` for display; it must send `pair`,
/// `ch` and the kana that was picked back unchanged.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DrillQuestion {
    /// The canonical key of the pair under test, e.g. `シ|ツ`.
    pub pair: String,
    /// The kana the learner is being asked to recognise.
    pub ch: char,
    /// The reading to prompt with.
    pub hepburn: String,
    /// The kana to offer as answers, one of which is `ch`. Two, for now: the pair
    /// itself — see `next_drill_question_with_rolls` for why not more.
    pub options: Vec<char>,
    /// What tells the two apart, so a miss teaches as well as records.
    pub tell: String,
}

/// What the learner's record for one pair now is, after an answer.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DrillTally {
    pub pair: String,
    pub asked: u32,
    pub correct: u32,
    pub wrong: u32,
    /// The pair's share of the drill, per the rule in `nihongo_core::drill`.
    pub weight: f64,
}

/// A yōon digraph.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YoonView {
    pub key: String,
    pub display: String,
    pub hepburn: String,
    pub kunrei: String,
    pub kana: Vec<char>,
}

/// One band of the vocabulary ladder.
///
/// `name` is the label a screen shows, and it is deliberately not "JLPT n": the
/// bands are derived from the kyōiku grades and EDRDG's frequency ranking, and
/// `nihongo_core::band_name` is where the words a learner sees come from.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BandView {
    pub band: u8,
    pub name: String,
    pub words: usize,
}

/// A page of one band's words.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WordPage {
    pub band: u8,
    /// How many words the band holds in all, so a screen can page without
    /// guessing.
    pub total: usize,
    pub offset: usize,
    pub words: Vec<WordView>,
}

/// One word, as the vocabulary screen draws it: the word, its own reading, the
/// furigana that puts that reading over the right characters, and its band.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WordView {
    pub text: String,
    /// The whole word's reading, from the dictionary — never composed from the
    /// characters.
    pub reading: String,
    pub meaning: String,
    pub band: u8,
    /// The band's name, so the screen never has to map a number to a claim.
    pub band_name: String,
    /// EDRDG's frequency block, 1–48, or `null` for a word it marks common
    /// without ranking.
    pub nf: Option<u8>,
    /// The furigana, in order. Empty for the words JmdictFurigana does not align.
    pub furigana: Vec<Ruby>,
}

/// One passage, as the reading screen offers it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PassageSummary {
    pub key: String,
    pub title: String,
    pub gloss: Option<String>,
    pub lines: usize,
    pub tokens: usize,
}

/// One passage, segmented, with a reading over every kanji.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PassageView {
    pub key: String,
    pub title: String,
    pub gloss: Option<String>,
    pub lines: Vec<Vec<PassageTokenView>>,
}

/// One word of a passage.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PassageTokenView {
    pub surface: String,
    /// The reading to draw over it, in hiragana; `null` for kana, which needs no
    /// ruby.
    pub rt: Option<String>,
    /// The vocabulary word this token is, when it is one, so a tap can open its
    /// card. The dictionary form: 行き links to 行く.
    pub word: Option<String>,
}

/// The result of checking a typed romaji answer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingCheck {
    pub correct: bool,
    /// What the typing actually produced, so a wrong answer can show the learner
    /// what they wrote rather than only that it was wrong.
    pub produced: String,
}

fn script_of(name: &str) -> Result<Script, String> {
    Script::from_name(name).ok_or_else(|| format!("unknown script {name:?}; expected hiragana or katakana"))
}

#[tauri::command]
fn app_info() -> AppInfo {
    licences::APP
}

/// Every notice this app owes, so a Licences screen can show them without
/// shipping a second copy of the text in the frontend.
#[tauri::command]
fn licences() -> Vec<LicenceNotice> {
    licence_notices()
}

/// The notice catalogue, for a caller that should not have to open a window.
pub fn licence_notices() -> Vec<LicenceNotice> {
    licences::notices().to_vec()
}

#[tauri::command]
fn dataset_stats(state: State<'_, AppState>) -> DatasetStats {
    state.stats()
}

#[tauri::command]
fn lessons(state: State<'_, AppState>, script: String) -> Result<Vec<LessonView>, String> {
    Ok(state.lessons(script_of(&script)?))
}

#[tauri::command]
fn kana(state: State<'_, AppState>, ch: char) -> Result<KanaView, String> {
    state.kana(ch)
}

#[tauri::command]
fn grade_attempt(
    state: State<'_, AppState>,
    ch: char,
    strokes: Vec<Vec<Point>>,
    options: Option<GradeOptions>,
) -> Result<GradeReport, String> {
    state.grade(ch, &strokes, &options.unwrap_or_default())
}

/// The next question for the discrimination drill, weighted towards the pairs
/// this learner gets wrong. `None` only if no pair can be asked at all.
#[tauri::command]
fn next_drill_question(state: State<'_, AppState>) -> Option<DrillQuestion> {
    state.next_drill_question()
}

/// Record what the learner answered, and return the pair's record now.
#[tauri::command]
fn record_drill_answer(
    state: State<'_, AppState>,
    pair: String,
    target: char,
    picked: char,
) -> Result<DrillTally, String> {
    state.record_drill_answer(&pair, target, picked)
}

/// Check a typed reading against a kana, accepting either romanisation.
#[tauri::command]
fn check_reading(ch: char, typed: String) -> ReadingCheck {
    let produced = to_kana(&typed).unwrap_or_default();
    ReadingCheck {
        correct: nihongo_core::matches_reading(ch, &typed),
        produced,
    }
}

/// Turn typed romaji into kana — the input half of the course.
#[tauri::command]
fn romaji_to_kana(input: String, script: Option<String>) -> Result<String, String> {
    let script = match script.as_deref() {
        None => Script::Hiragana,
        Some(name) => script_of(name)?,
    };
    to_kana_in(script, &input).map_err(|e| e.to_string())
}

#[tauri::command]
fn yoon(script: String) -> Result<Vec<YoonSummary>, String> {
    Ok(build_yoon(script_of(&script)?)
        .into_iter()
        .map(|y| YoonSummary {
            hepburn: y.hepburn,
            display: y.display,
        })
        .collect())
}

/// The vocabulary ladder, one entry per band, with each band's size.
#[tauri::command]
fn word_bands(state: State<'_, AppState>) -> Vec<BandView> {
    state.word_bands()
}

/// One page of a band's words, in course order.
#[tauri::command]
fn words_in_band(state: State<'_, AppState>, band: u8, offset: usize, limit: usize) -> WordPage {
    state.words_in_band(band, offset, limit)
}

/// One word, by its text and its reading.
#[tauri::command]
fn word(state: State<'_, AppState>, text: String, reading: String) -> Result<WordView, String> {
    state.word(&text, &reading)
}

/// One word, by its text alone — what a tapped passage token has.
#[tauri::command]
fn word_of_text(state: State<'_, AppState>, text: String) -> Result<WordView, String> {
    state.word_of_text(&text)
}

/// Check a typed reading against a word. The word is graded as a word — its own
/// reading, which is never composed from its characters.
#[tauri::command]
fn check_word(
    state: State<'_, AppState>,
    text: String,
    reading: String,
    typed: String,
) -> Result<ReadingCheck, String> {
    state.check_word(&text, &reading, &typed)
}

/// The passages a learner can read.
#[tauri::command]
fn passages(state: State<'_, AppState>) -> Vec<PassageSummary> {
    state.passages()
}

/// One passage, segmented, with the reading over every kanji.
#[tauri::command]
fn passage(state: State<'_, AppState>, key: String) -> Result<PassageView, String> {
    state.passage(&key)
}

/// A yōon digraph in the cut-down form the input helper needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YoonSummary {
    hepburn: String,
    display: String,
}

/// A roll in `0..1`, from the clock and a counter.
///
/// Not cryptography, and not a game's RNG: it decides which pair to ask, and the
/// only requirements are that two calls in quick succession differ and that the
/// result is spread out rather than stuck at one end. `splitmix64`'s finaliser
/// over `nanoseconds ^ counter` is enough for that, and it is written out here
/// because a dependency for four lines of arithmetic would be the tail wagging
/// the dog — `HANDOVER.md`'s "the app has no network path" is the same instinct.
fn roll() -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.subsec_nanos() as u64)
        .unwrap_or(0);
    let mut z = nanos
        ^ COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    // 53 bits is the whole of a f64's mantissa, so this is uniform where it can
    // be and never rounds up to exactly 1.0.
    (z >> 11) as f64 / (1u64 << 53) as f64
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // The learner's own file, in the app's own directory. Rust writes it
            // rather than the webview, so the app still needs no filesystem
            // permission in `capabilities/default.json` — the webview has no way
            // to name, read or write a path.
            let state = match app.path().app_data_dir() {
                Ok(dir) => AppState::load_at(dir),
                Err(err) => {
                    eprintln!("no app data directory ({err}); this session will not be remembered");
                    AppState::load()
                }
            };
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            licences,
            dataset_stats,
            lessons,
            kana,
            grade_attempt,
            next_drill_question,
            record_drill_answer,
            check_reading,
            romaji_to_kana,
            yoon,
            word_bands,
            words_in_band,
            word,
            word_of_text,
            check_word,
            passages,
            passage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the kana tutor");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState::load()
    }

    #[test]
    fn the_embedded_dataset_is_the_whole_kana_set() {
        let state = state();
        let stats = state.stats();
        assert_eq!(stats.kana, 177);
        assert_eq!(stats.hiragana, 86);
        assert_eq!(stats.katakana, 91);
        assert_eq!(stats.strokes, 516);
        // Eleven plain rows, five voiced rows, then small, rare, and — in
        // katakana only — the v-series and the prolonged sound mark.
        assert_eq!(state.lessons(Script::Hiragana).len(), 18);
        assert_eq!(state.lessons(Script::Katakana).len(), 20);
        assert_eq!(stats.lessons, 38);
    }

    #[test]
    fn a_kana_view_carries_the_geometry_the_screen_draws_and_grades() {
        let view = state().kana('あ').expect("あ is in the set");
        assert_eq!(view.script, "hiragana");
        assert_eq!(view.stroke_count, 3);
        assert!(view.practisable);
        assert!(!view.silent);
        assert_eq!(view.hepburn, "a");
        assert_eq!(view.outlines.len(), 3);
        assert_eq!(view.medians.len(), 3);
        assert!(view.romaji.contains(&"a".to_string()));
    }

    #[test]
    fn a_kana_view_explains_what_it_is_confused_with() {
        let view = state().kana('シ').expect("シ is in the set");
        assert!(
            view.confusions.iter().any(|c| c.ch == 'ツ'),
            "シ should be flagged against ツ"
        );
        assert!(!view.confusions[0].tell.is_empty());
    }

    #[test]
    fn a_kana_outside_the_set_is_an_error_not_a_panic() {
        let err = state().kana('一').unwrap_err();
        assert!(err.contains("not in the kana set"), "{err}");
    }

    #[test]
    fn a_silent_mark_reports_itself_as_silent() {
        assert!(state().kana('ー').expect("ー is in the set").silent);
        assert!(state().kana('っ').expect("っ is in the set").silent);
        assert!(!state().kana('ん').expect("ん is in the set").silent);
    }

    #[test]
    fn grading_traces_the_stored_centre_line_as_legible() {
        let state = state();
        let view = state.kana('ー').expect("ー is in the set");
        let attempt = vec![view.medians[0].clone()];
        let report = state
            .grade('ー', &attempt, &GradeOptions::default())
            .expect("grades");
        assert!(report.legible, "tracing the guide must be legible: {:.0}", report.overall);
        assert_eq!(report.expected_strokes, 1);
    }

    #[test]
    fn grading_a_mark_that_is_not_taught_is_an_error() {
        let err = state()
            .grade('一', &[], &GradeOptions::default())
            .unwrap_err();
        assert!(err.contains("not in the kana set"), "{err}");
    }

    #[test]
    fn grading_uses_the_corrected_stroke_count_not_the_raw_one() {
        // あ arrives from upstream as four drawing segments and is taught with
        // three. The grader must ask for three.
        let state = state();
        let view = state.kana('あ').expect("あ is in the set");
        assert_eq!(view.stroke_count, 3);
        let report = state
            .grade('あ', &[], &GradeOptions::default())
            .expect("grades");
        assert_eq!(report.expected_strokes, 3);
    }

    #[test]
    fn every_classic_pair_can_actually_be_asked() {
        let state = state();
        let askable = state.askable_pairs();
        assert_eq!(
            askable.len(),
            CONFUSABLE.len(),
            "a pair the dataset cannot prompt would silently shrink the drill"
        );

        // And the question each of them produces is answerable: the target is one
        // of the options, both options are the pair's own kana, and the prompt is
        // not empty.
        for (index, pair) in askable.iter().enumerate() {
            let roll = (index as f64 + 0.5) / askable.len() as f64;
            for side in [0.0, 0.75] {
                let question = state
                    .next_drill_question_with_rolls(roll, side)
                    .expect("an askable pair yields a question");
                assert_eq!(question.pair, pair_key(pair));
                assert!(
                    question.options.contains(&question.ch),
                    "{} must be one of its own options",
                    question.ch
                );
                assert_eq!(question.options.len(), 2, "the pair, and nothing else");
                for option in &question.options {
                    assert!(
                        *option == pair.a || *option == pair.b,
                        "{option} is not in {}",
                        question.pair
                    );
                }
                assert!(!question.hepburn.is_empty(), "{} has no prompt", question.ch);
                assert!(!question.tell.is_empty(), "{} has no tell", question.pair);
            }
        }

        // The two sides are two questions, not the same one twice.
        let first = state.next_drill_question_with_rolls(0.0, 0.0).expect("a question");
        let second = state.next_drill_question_with_rolls(0.0, 1.0).expect("a question");
        assert_eq!(first.pair, second.pair);
        assert_ne!(first.ch, second.ch, "the pair asked from the other side");
        let mut offered = first.options.clone();
        offered.sort_unstable();
        let mut both = vec![first.ch, second.ch];
        both.sort_unstable();
        assert_eq!(offered, both, "the pair's two kana, whichever side is asked");
    }

    #[test]
    fn an_answer_is_recorded_against_the_pair_and_the_arithmetic_says_so() {
        let state = state();
        let pair = pair_key(CONFUSABLE.iter().find(|p| p.a == 'シ').expect("シ/ツ"));

        // A miss: 1 + 2.
        let after_miss = state
            .record_drill_answer(&pair, 'シ', 'ツ')
            .expect("records");
        assert_eq!(after_miss.pair, pair);
        assert_eq!(after_miss.asked, 1);
        assert_eq!(after_miss.wrong, 1);
        assert_eq!(after_miss.correct, 0);
        assert_eq!(after_miss.weight, 3.0);

        // The same pair named the other way round, answered correctly: one pair,
        // and the weight comes back down.
        let after_hit = state
            .record_drill_answer("ツ|シ", 'ツ', 'ツ')
            .expect("records");
        assert_eq!(after_hit.pair, pair, "one pair, whichever way it is named");
        assert_eq!(after_hit.asked, 2);
        assert_eq!(after_hit.weight, 2.0);

        // Correctness is decided here, from the two kana — not sent by the caller.
        let lying = state
            .record_drill_answer(&pair, 'シ', 'ツ')
            .expect("records");
        assert_eq!(lying.wrong, 2, "シ asked for and ツ picked is a miss");
        assert_eq!(lying.correct, 1);
    }

    #[test]
    fn an_answer_about_something_that_is_not_a_pair_is_refused() {
        let state = state();
        let err = state
            .record_drill_answer("あ|い", 'あ', 'い')
            .unwrap_err();
        assert!(err.contains("not one of the confusion pairs"), "{err}");

        // A pair, but a kana that is not in it.
        let err = state.record_drill_answer("シ|ツ", 'あ', 'ツ').unwrap_err();
        assert!(err.contains("is not in シ|ツ"), "{err}");
        let err = state.record_drill_answer("シ|ツ", 'シ', 'あ').unwrap_err();
        assert!(err.contains("neither of the two answers"), "{err}");

        // And nothing was written by any of it.
        assert!(state.log().is_empty());
    }

    #[test]
    fn the_drill_prefers_the_pair_the_learner_keeps_getting_wrong() {
        let state = state();
        // ン/ソ (ソ|ン) sits early in the list, so a mid-field roll lands past it
        // while everything weighs the same.
        let roll = 0.3;
        let before = state.next_drill_question_with_rolls(roll, 0.0).expect("a question");
        assert_ne!(before.pair, "ソ|ン");

        state.record_drill_answer("ソ|ン", 'ン', 'ソ').expect("records");
        state.record_drill_answer("ソ|ン", 'ン', 'ソ').expect("records");
        let after = state.next_drill_question_with_rolls(roll, 0.0).expect("a question");
        assert_eq!(after.pair, "ソ|ン", "the same roll now lands on the missed pair");
    }

    #[test]
    fn the_question_is_never_one_the_board_cannot_draw() {
        // Every kana the drill can ask about is in the dataset, so the pair is
        // never outside the set the rest of the app works in.
        let state = state();
        for index in 0..CONFUSABLE.len() {
            let roll = (index as f64 + 0.01) / CONFUSABLE.len() as f64;
            let question = state.next_drill_question_with_rolls(roll, 0.5).expect("a question");
            for option in &question.options {
                assert!(state.kana(*option).is_ok(), "{option} cannot be drawn");
            }
        }
    }

    #[test]
    fn reading_checks_accept_both_romanisations() {
        assert!(check_reading('し', "shi".into()).correct);
        assert!(check_reading('し', "si".into()).correct);
        assert!(!check_reading('し', "chi".into()).correct);
        // A wrong answer still reports what it produced.
        let wrong = check_reading('し', "chi".into());
        assert_eq!(wrong.produced, "ち");
    }

    #[test]
    fn romaji_converts_in_both_scripts() {
        assert_eq!(romaji_to_kana("kana".into(), None).expect("converts"), "かな");
        assert_eq!(
            romaji_to_kana("kana".into(), Some("katakana".into())).expect("converts"),
            "カナ"
        );
        assert!(romaji_to_kana("kana".into(), Some("kanji".into())).is_err());
        assert!(romaji_to_kana("!!!".into(), None).is_err());
    }

    #[test]
    fn every_lesson_names_kana_the_screen_can_open() {
        let state = state();
        for script in [Script::Hiragana, Script::Katakana] {
            for lesson in state.lessons(script) {
                assert_eq!(lesson.count, lesson.kana.len());
                for ch in lesson.kana {
                    assert!(
                        state.kana(ch).is_ok(),
                        "lesson {} lists {ch}, which cannot be opened",
                        lesson.key
                    );
                }
            }
        }
    }

    #[test]
    fn the_ladder_comes_back_with_our_names_and_its_sizes() {
        let state = state();
        let bands = state.word_bands();
        assert_eq!(bands.len(), 7, "six kyōiku grades and the remainder");
        assert_eq!(
            bands.iter().map(|b| b.words).sum::<usize>(),
            state.words.len()
        );
        assert_eq!(bands[0].band, 1);
        assert_eq!(bands[0].name, "kyōiku 1");
        assert_eq!(bands[6].name, "jōyō beyond the school grades");
        for band in &bands {
            assert!(
                !band.name.to_ascii_uppercase().contains("JLPT"),
                "the ladder is derived, not the JLPT's: {:?}",
                band.name
            );
        }
        // No band is empty, and band 1 is the smallest — a fact about Japanese
        // rather than a hole in the data.
        assert!(bands.iter().all(|b| b.words > 0));
        assert!(bands[0].words < bands[6].words);
    }

    #[test]
    fn a_page_of_words_carries_what_a_word_card_draws() {
        let state = state();
        let page = state.words_in_band(1, 0, 10);
        assert_eq!(page.band, 1);
        assert_eq!(page.offset, 0);
        assert_eq!(page.words.len(), 10);
        assert!(
            page.total > page.words.len(),
            "band 1 holds more than one page: {} words",
            page.total
        );

        let first = &page.words[0];
        assert!(!first.text.is_empty());
        assert!(!first.reading.is_empty());
        assert!(!first.meaning.is_empty());
        assert_eq!(first.band, 1);
        assert_eq!(first.band_name, "kyōiku 1");
        assert!(first.nf.is_none_or(|rank| (1..=48).contains(&rank)));

        // A page past the end is empty rather than an error, and a limit the
        // interface should not have asked for is clamped.
        assert!(state.words_in_band(1, page.total, 10).words.is_empty());
        assert!(state.words_in_band(1, 0, 10_000).words.len() <= 200);
    }

    #[test]
    fn a_word_is_graded_as_a_word_and_never_from_its_characters() {
        let state = state();
        // 大人 is おとな. A reading composed from the characters would be
        // だいじん, and the point of carrying the word's own reading is that it is
        // not.
        assert!(state.check_word("大人", "おとな", "otona").expect("a word").correct);
        assert!(!state.check_word("大人", "おとな", "daijin").expect("a word").correct);
        // The wrong answer says what it produced, so a screen can show it.
        assert_eq!(
            state.check_word("大人", "おとな", "daijin").expect("a word").produced,
            "だいじん"
        );
        // A word that is not one is an error rather than a silent `false`.
        assert!(state.check_word("大人", "だいじん", "otona").is_err());
    }

    #[test]
    fn a_kana_answer_is_accepted_as_well_as_romaji() {
        let state = state();
        assert!(state.check_word("学生", "がくせい", "gakusei").expect("a word").correct);
        assert!(
            state.check_word("学生", "がくせい", "がくせい").expect("a word").correct,
            "a learner who can read kana should not have to transliterate"
        );
        assert!(
            state.check_word("学生", "がくせい", "ガクセイ").expect("a word").correct,
            "the kana type should not decide the answer"
        );
    }

    #[test]
    fn a_word_can_be_fetched_by_its_text_and_reading() {
        let state = state();
        let word = state.word("食べる", "たべる").expect("食べる is in the vocabulary");
        assert_eq!(word.text, "食べる");
        assert_eq!(word.reading, "たべる");
        assert_eq!(word.meaning, "to eat");
        assert_eq!(word.furigana.len(), 2);
        assert_eq!(word.furigana[0].ruby, "食");
        assert_eq!(word.furigana[0].rt.as_deref(), Some("た"));
        assert!(state.word("食べる", "くう").is_err(), "that is a different word");
    }

    #[test]
    fn a_word_a_passage_links_to_is_found_by_its_text_alone() {
        // The token carries the word's text, not its reading, so a tap has to be
        // able to open an entry from that alone — 行き's own reading is いき and the
        // word it links to is 行く, read いく.
        let state = state();
        let word = state.word_of_text("行く").expect("行く is in the vocabulary");
        assert_eq!(word.text, "行く");
        assert_eq!(word.reading, "いく");

        // 行き is an entry in its own right — a noun — so this finds that one, and
        // the two are different words rather than one answer to two questions.
        let noun = state.word_of_text("行き").expect("行き is a noun entry");
        assert_eq!(noun.text, "行き");
        assert_ne!(noun.reading, word.reading);

        // Anything the course does not teach is a message rather than a panic.
        assert!(state.word_of_text("あいうえお").is_err());
    }

    #[test]
    fn the_passages_come_back_segmented_with_their_readings() {
        let state = state();
        let summaries = state.passages();
        assert_eq!(summaries.len(), 3);
        assert!(summaries.iter().all(|p| p.lines > 0 && p.tokens > 0));
        assert!(summaries.iter().all(|p| p.gloss.is_some()));

        let asa = state.passage("asa").expect("asa is a passage");
        assert_eq!(asa.title, "あさ");
        // 学生 is one token, carries its reading, and links to the vocabulary word.
        let student = asa
            .lines
            .iter()
            .flatten()
            .find(|token| token.surface == "学生")
            .expect("学生 is in the passage");
        assert_eq!(student.rt.as_deref(), Some("がくせい"));
        assert_eq!(student.word.as_deref(), Some("学生"));
        // A particle is a token too, and needs no ruby.
        let particle = asa
            .lines
            .iter()
            .flatten()
            .find(|token| token.surface == "は")
            .expect("は is in the passage");
        assert_eq!(particle.rt, None);

        assert!(state.passage("nope").is_err());
    }

    #[test]
    fn every_token_of_every_passage_that_has_a_word_can_be_opened() {
        // What a tap does, as a test: the link a passage carries has to resolve to
        // a word the vocabulary screen can actually show.
        let state = state();
        let mut opened = 0;
        for summary in state.passages() {
            for line in state.passage(&summary.key).expect("a passage").lines {
                for token in line {
                    let Some(text) = &token.word else { continue };
                    // The link is the word's text; the reading is whatever the
                    // vocabulary holds for it, which is what the card will show.
                    let held = state
                        .words
                        .of_text(text)
                        .unwrap_or_else(|| panic!("{text} is linked but not held"));
                    let word = state
                        .word(&held.text, &held.reading)
                        .unwrap_or_else(|e| panic!("{text} does not open: {e}"));
                    assert_eq!(word.text, *text);
                    if token.surface.chars().any(nihongo_core::is_kanji) {
                        assert!(
                            token.rt.is_some(),
                            "{text}: a kanji token with no reading cannot be drawn"
                        );
                    }
                    opened += 1;
                }
            }
        }
        assert!(opened >= 10, "the passages link to {opened} words in all");
    }

    #[test]
    fn the_yoon_helper_covers_every_digraph() {
        assert_eq!(yoon("hiragana".into()).expect("known script").len(), 33);
        let all = yoon("katakana".into()).expect("known script");
        assert!(all.iter().any(|y| y.display == "キュ" && y.hepburn == "kyu"));
        assert!(yoon("kanji".into()).is_err());
    }
}
