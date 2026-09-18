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
pub mod progress;
pub mod time;
pub mod vocab;

pub use curriculum::{build_lessons, lesson_at, Lesson};
pub use dataset::{Character, CharacterHint, Dataset, TextLookup};
pub use geom::Point;
pub use grade::{
    grade, FitInfo, Grade, GradeOptions, GradeReport, StrokeVerdict, Verdict,
};
pub use progress::{
    build_queue, Attempt, CardState, CardView, CursorStore, CursorView, ProgressError,
    ProgressStore, ProgressView, Rating, ReviewItem, ReviewSource, ReviewView, Scheduler,
    Sm2,
};
pub use time::{iso8601_from_unix, now_iso8601, parse_iso8601};
pub use vocab::{
    Entry, ImportSummary, VocabError, VocabStore, VocabView,
};
