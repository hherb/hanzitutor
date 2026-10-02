//! Offline data for learning Japanese kana, and the grading that goes with it.
//!
//! This crate is the Japanese counterpart to `hanzi-core`, and it is deliberately
//! thin: the geometry engine — resampling, the Hungarian stroke pairing, the
//! order analysis, the ink measures, the scheduling — is language-neutral and
//! lives in `hanzi-core`, so a kana is graded by exactly the code that grades a
//! Chinese character.
//!
//! What is here is the kana data layer: how to load the shipped artifact, what a
//! kana is, and the one piece of upstream repair the source data needs (see
//! [`kana`]).
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
pub mod input;
pub mod kana;
pub mod readings;

pub use curriculum::{
    confusions_for, lessons, to_hiragana, to_katakana, yoon, Confusable, Lesson, Row, Yoon,
    CHOONPU, CONFUSABLE, KATAKANA_ONLY, RARE_KANA, ROWS, SMALL_KANA,
};
pub use hanzi_core::{
    grade, grade_with_outlines, Grade, GradeOptions, GradeReport, Point, StrokeVerdict, Verdict,
};
pub use kana::{
    merge_strokes, segment_to_stroke, Artifact, Kana, KanaDataset, MergeError, Script,
    ARTIFACT_MAGIC,
};
pub use input::{
    continuations, matches_reading, matches_word, normalise_to_hiragana, to_kana, to_kana_in,
    RomajiError,
};
pub use readings::{reading, Reading};
