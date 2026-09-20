//! Offline grading of handwritten simplified Chinese characters.
//!
//! This crate is deliberately free of any UI or platform dependency. It knows
//! how to load the character dataset, how to compare a hand-drawn attempt
//! against the reference stroke data, and how to organise a course from a
//! frequency list. A Tauri app, a CLI or a test harness can all drive it.
//!
//! ```no_run
//! use hanzi_core::{grade, GradeOptions, Dataset};
//!
//! # fn main() -> std::io::Result<()> {
//! let bytes = std::fs::read("data/hanzi.bin.gz")?;
//! let dataset = Dataset::from_gzip_bytes(&bytes)?;
//! let character = dataset.get('一').expect("一 is in the dataset");
//!
//! // One horizontal stroke, drawn left to right across the middle of the box.
//! let attempt = vec![vec![
//!     hanzi_core::Point::new(120.0, 400.0),
//!     hanzi_core::Point::new(900.0, 400.0),
//! ]];
//!
//! let report = grade(
//!     character.reference_medians(),
//!     &attempt,
//!     &GradeOptions::default(),
//! );
//! println!("{:.0}/100, legible: {}", report.overall, report.legible);
//! # Ok(())
//! # }
//! ```

pub mod curriculum;
pub mod dataset;
pub mod geom;
pub mod grade;
pub mod pinyin;
pub mod progress;
mod raster;
pub mod settings;
pub mod time;
pub mod tone;
pub mod vocab;

pub use curriculum::{build_lessons, lesson_at, Lesson};
pub use dataset::{Artifact, Character, CharacterHint, Dataset, TextLookup, Word};
pub use geom::Point;
pub use pinyin::{
    base, heard_against, spoken_tones, syllables, tone_from_pinyin, tone_target, Heard,
    HeardSyllable, Syllable, ToneTarget,
};
pub use grade::{
    grade, grade_with_outlines, FitInfo, Grade, GradeOptions, GradeReport, StrokeVerdict,
    Verdict, INK_OK,
};
pub use raster::INK_WIDTH;
pub use progress::{
    build_queue, fold_attempts, Attempt, AttemptRecord, CardState, CardView, CursorDocument,
    CursorSink, CursorStore, CursorView, ProgressError, ProgressSink, ProgressStore, ProgressView,
    Rating, ReviewItem, ReviewSource, ReviewView, Scheduler, Sm2,
};
pub use settings::{
    BoardSize, Pace, Settings, SettingsError, SettingsSink, SettingsStore, SettingsView,
};
pub use time::{iso8601_from_unix, now_iso8601, parse_iso8601};
pub use tone::{
    analyze, analyze_tone, tone_name, tone_template, SyllableReport, ToneAttempt, ToneReport,
    ToneVerdict,
};
pub use vocab::{Entry, ImportSummary, VocabError, VocabSink, VocabStore, VocabView};
