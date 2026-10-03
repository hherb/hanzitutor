//! Offline data for learning Japanese, and the grading that goes with it.
//!
//! This crate is the Japanese counterpart to `hanzi-core`, and it is deliberately
//! thin: the geometry engine — resampling, the Hungarian stroke pairing, the
//! order analysis, the ink measures, the scheduling — is language-neutral and
//! lives in `hanzi-core`, so a kana is graded by exactly the code that grades a
//! Chinese character, and so is a kanji.
//!
//! What is here is the data layer: how to load the shipped artifacts, what a
//! kana is, what a kanji is, and the one piece of upstream repair the kana
//! source needs (see [`kana`], and [`kanji`] for why the kanji source needs
//! none).
//!
//! ```no_run
//! use nihongo_core::{KanaDataset, Script};
//! use hanzi_core::{grade, GradeOptions};
//!
//! # fn main() -> std::io::Result<()> {
//! let bytes = std::fs::read("crates/nihongo-core/data/kana.bin.gz")?;
//! let dataset = KanaDataset::from_gzip_bytes(&bytes)?;
//! assert_eq!(dataset.of_script(Script::Hiragana).count(), 86);
//!
//! let kana = dataset.get('あ').expect("あ is in the dataset");
//! assert_eq!(kana.stroke_count, 3, "あ is taught with three strokes");
//!
//! // One horizontal stroke, drawn left to right across the top of the box.
//! let attempt = vec![vec![
//!     hanzi_core::Point::new(150.0, 250.0),
//!     hanzi_core::Point::new(850.0, 260.0),
//! ]];
//! let report = grade(kana.reference_medians(), &attempt, &GradeOptions::default());
//! println!("{:.0}/100", report.overall);
//! # Ok(())
//! # }
//! ```

pub mod curriculum;
pub mod drill;
pub mod input;
pub mod kana;
pub mod kanji;
pub mod passages;
pub mod readings;
pub mod words;

pub use curriculum::{
    confusions_for, lessons, to_hiragana, to_katakana, yoon, Confusable, Lesson, Row, Yoon,
    CHOONPU, CONFUSABLE, KATAKANA_ONLY, RARE_KANA, ROWS, SMALL_KANA,
};
pub use drill::{
    find_pair, pair_key, ConfusionLog, PairTally, CORRECT_WEIGHT, MIN_WEIGHT, START_WEIGHT,
    WRONG_WEIGHT,
};
pub use hanzi_core::{
    grade, grade_with_outlines, Grade, GradeOptions, GradeReport, Point, StrokeVerdict, Verdict,
};
pub use kana::{
    merge_strokes, segment_to_stroke, Artifact, Kana, KanaDataset, MergeError, Script,
    ARTIFACT_MAGIC,
};
pub use kanji::{
    parse_radical, Kanji, KanjiArtifact, KanjiDataset, KanjiSource, JOYO_COUNT, JOYO_GRADES,
    KANJI_ARTIFACT_MAGIC,
};
pub use input::{
    continuations, matches_reading, matches_word, normalise_to_hiragana, to_kana, to_kana_in,
    RomajiError,
};
pub use readings::{reading, Reading};
pub use words::{
    band_for, band_name, is_kanji, Ruby, Word, WordsArtifact, WordDataset, WordsSource, BANDS,
    REMAINDER_BAND, WORDS_ARTIFACT_MAGIC,
};
pub use passages::{
    parse_passage, Passage, PassageDataset, PassageToken, PassagesArtifact, PassagesSource,
    PASSAGES_ARTIFACT_MAGIC,
};
