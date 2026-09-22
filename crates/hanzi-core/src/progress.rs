//! Per-character practice history and spaced-repetition scheduling.
//!
//! The course is a fixed list and the vocabulary list is the learner's own, but
//! neither remembered anything: which characters had been seen, how they scored
//! and what was due for review were all forgotten when the app closed. This
//! module is that memory.
//!
//! It follows [`crate::vocab`] deliberately — a versioned, human-readable JSON
//! document, written atomically, that **refuses to overwrite a file it could not
//! parse** — because the reason for that rule does not change: losing a month of
//! study history to a parse error would be far worse than refusing to write.
//!
//! Where it differs is the key. A record here is per *character*, keyed by the
//! character itself, because both sources are written one character at a time: a
//! character met in a lesson and the same character met inside a word are the same
//! thing to learn, so they share one schedule.
//!
//! The schedule and the reader's place in the course live in **separate files**
//! ([`ProgressStore`] and [`CursorStore`]) so that a corrupt schedule cannot lose
//! where someone was, and vice versa.
//!
//! Grading is not part of this module and must not become part of it: it turns a
//! 0..=100 score into a due date, and nothing else.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::grade::Grade;
use crate::time::{add_seconds, now_iso8601, parse_iso8601};
use crate::vocab::Entry;

/// Format version written into the documents. Bump when the shape changes and
/// add a migration; a document from the future is refused rather than guessed at.
pub const FORMAT_VERSION: u32 = 1;

/// How many attempts are kept per character in memory, and shown to the learner.
///
/// The whole history is not needed to schedule anything — the intervals carry
/// that — but a recent run of scores is what a learner wants to see, and a
/// bounded vector keeps the document from growing without limit. A backing store
/// that keeps an unbounded log still fills this from the newest rows, which is
/// why the bound is public: it is the store's contract, not a private detail.
pub const MAX_HISTORY: usize = 20;

/// How soon a failed character comes back.
///
/// Not tomorrow: a character written wrong should return in the same sitting,
/// while the attempt is still fresh in mind.
const AGAIN_SECONDS: i64 = 60;

/// SM-2's ease factor bounds. 1.3 is the textbook floor; the ceiling is not in
/// SM-2 but stops a long run of easy reviews from producing absurd intervals.
const MIN_EASE: f32 = 1.3;
const MAX_EASE: f32 = 3.0;
const START_EASE: f32 = 2.5;

/// The longest interval the schedule will hand out, in days.
///
/// SM-2 multiplies without limit, so a character answered well for long enough
/// would be scheduled years — eventually millennia — out. A year is already far
/// beyond what a learner needs, and bounding it keeps every due date inside the
/// timestamp format this module writes.
const MAX_INTERVAL_DAYS: f32 = 365.0;

/// Why a progress or cursor operation failed.
#[derive(Debug)]
pub enum ProgressError {
    /// The document could not be read or written.
    Io(String),
    /// The document is not valid JSON, or does not match the schema.
    Malformed(String),
    /// The document was written by a newer version of the app.
    UnsupportedVersion(u32),
    /// A timestamp was not the ISO-8601 UTC form this module writes.
    InvalidTimestamp(String),
}

impl std::fmt::Display for ProgressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(why) => write!(f, "{why}"),
            Self::Malformed(why) => write!(f, "it is not valid JSON: {why}"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "it was written by a newer version of the app \
                 (format {v}, this build understands {FORMAT_VERSION})"
            ),
            Self::InvalidTimestamp(at) => {
                write!(f, "{at:?} is not an ISO-8601 UTC timestamp")
            }
        }
    }
}

impl std::error::Error for ProgressError {}

/// How well an attempt went, in the four grades a review button would offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rating {
    /// Not legible, or barely: show it again shortly.
    Again,
    /// Recognisable but shaky.
    Hard,
    /// Correct.
    Good,
    /// Correct without hesitation.
    Easy,
}

impl Rating {
    /// Derive a rating from the 0..=100 headline score.
    ///
    /// The bands are the ones the feedback panel already shows
    /// ([`Grade::from_score`]), so "poor" on screen and "again" in the schedule
    /// always describe the same attempt.
    pub fn from_score(score: f32) -> Self {
        match Grade::from_score(score) {
            Grade::Excellent => Rating::Easy,
            Grade::Good => Rating::Good,
            Grade::Fair => Rating::Hard,
            Grade::Poor => Rating::Again,
        }
    }

    /// The SM-2 quality (0..=5) this rating stands for.
    pub fn quality(self) -> u8 {
        match self {
            Rating::Again => 1,
            Rating::Hard => 3,
            Rating::Good => 4,
            Rating::Easy => 5,
        }
    }

    /// The stored name of this rating.
    ///
    /// It is the same string serde writes for the JSON document and the same one
    /// the study database holds, so the three cannot drift apart — a test asserts
    /// this against the serialised form, because a renamed variant would otherwise
    /// leave a database and an interface quietly disagreeing about what a row
    /// means.
    pub fn name(self) -> &'static str {
        match self {
            Rating::Again => "again",
            Rating::Hard => "hard",
            Rating::Good => "good",
            Rating::Easy => "easy",
        }
    }

    /// The rating a stored name means, or `None` when it is not one this build
    /// knows. The caller decides whether that is worth reporting or defaulting.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "again" => Some(Rating::Again),
            "hard" => Some(Rating::Hard),
            "good" => Some(Rating::Good),
            "easy" => Some(Rating::Easy),
            _ => None,
        }
    }
}

/// One recorded attempt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    /// When it happened, ISO-8601 UTC.
    pub at: String,
    /// The 0..=100 headline score from the grading engine.
    pub score: f32,
    pub rating: Rating,
}

/// Everything remembered about one character.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardState {
    pub attempts: u32,
    /// How many times the character was failed after having been learned.
    #[serde(default)]
    pub lapses: u32,
    #[serde(default)]
    pub best_score: Option<f32>,
    #[serde(default)]
    pub last_score: Option<f32>,
    /// When it was last practised, ISO-8601 UTC. Sorts chronologically as text.
    #[serde(default)]
    pub last_practised: Option<String>,
    /// When it should next be reviewed, ISO-8601 UTC.
    pub due: String,
    /// The current interval in days. Fractional, because 12 hours is a real
    /// answer for a character that only just went in.
    pub interval_days: f32,
    /// SM-2's ease factor.
    pub ease: f32,
    /// Consecutive successful reviews; reset by [`Rating::Again`].
    pub repetitions: u32,
    /// Recent attempts, oldest first, capped at [`MAX_HISTORY`].
    #[serde(default)]
    pub history: Vec<Attempt>,
}

impl CardState {
    /// A card that has never been reviewed, as of `at`.
    fn new(at: &str) -> Self {
        Self {
            attempts: 0,
            lapses: 0,
            best_score: None,
            last_score: None,
            last_practised: None,
            due: at.to_string(),
            interval_days: 0.0,
            ease: START_EASE,
            repetitions: 0,
            history: Vec::new(),
        }
    }

    /// True when the character is due at `now`.
    ///
    /// ISO-8601 UTC strings in this fixed-width format compare correctly as
    /// text, so this needs no clock arithmetic at all.
    pub fn is_due(&self, now: &str) -> bool {
        self.due.as_str() <= now
    }

    /// This card as the interface sees it, with due-ness resolved at `now`.
    fn view(&self, ch: char, now: &str) -> CardView {
        CardView {
            ch,
            attempts: self.attempts,
            lapses: self.lapses,
            best_score: self.best_score,
            last_score: self.last_score,
            last_practised: self.last_practised.clone(),
            due: self.due.clone(),
            interval_days: self.interval_days,
            ease: self.ease,
            repetitions: self.repetitions,
            history: self.history.clone(),
            due_now: self.is_due(now),
        }
    }
}

/// The scheduling policy.
///
/// Kept behind a trait so SM-2 can be replaced — FSRS is better but wants far
/// more data than one learner produces quickly — without touching the store or
/// the interface.
pub trait Scheduler {
    /// Apply one review to `card`, whose attempt has already been recorded.
    fn review(&self, card: &mut CardState, rating: Rating, at: &str);
}

/// SM-2, the classic SuperMemo algorithm, in the shape a handwriting score
/// naturally takes.
///
/// Standard SM-2 grades an answer 0..=5 and updates an ease factor and a
/// repetition count from it; here the four [`Rating`] buttons come from the
/// attempt score and map to qualities 1/3/4/5. A failed review resets the
/// repetition count and makes the character due again within the minute; a pass
/// advances 1 day, then 6, then multiplies by the ease factor. Every rating also
/// gets a distinct first interval — 12 hours, 1 day, 2 days — so a good attempt
/// is always scheduled further out than a poor one, which is the property the
/// habit depends on.
///
/// One review produces one new due date. Nothing here is a live-updated model,
/// which is what makes it testable.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sm2;

impl Scheduler for Sm2 {
    fn review(&self, card: &mut CardState, rating: Rating, at: &str) {
        // The textbook SM-2 ease update, on the 0..=5 quality scale: an easy
        // answer raises ease slightly, a failure lowers it a lot.
        let q = rating.quality() as f32;
        card.ease =
            (card.ease + (0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02))).clamp(MIN_EASE, MAX_EASE);

        match rating {
            Rating::Again => {
                card.repetitions = 0;
                card.lapses += 1;
                card.interval_days = 0.0;
            }
            // A hard pass still advances, but more slowly than a clean one.
            Rating::Hard => {
                card.repetitions += 1;
                card.interval_days = if card.repetitions <= 1 {
                    0.5
                } else {
                    (card.interval_days * 1.2).max(0.5)
                };
            }
            Rating::Good => {
                card.repetitions += 1;
                card.interval_days = match card.repetitions {
                    0 | 1 => 1.0,
                    2 => 6.0,
                    _ => (card.interval_days * card.ease).max(1.0),
                };
            }
            Rating::Easy => {
                card.repetitions += 1;
                card.interval_days = match card.repetitions {
                    0 | 1 => 2.0,
                    2 => 8.0,
                    _ => (card.interval_days * card.ease * 1.3).max(1.0),
                };
            }
        }

        let seconds = if matches!(rating, Rating::Again) {
            AGAIN_SECONDS
        } else {
            card.interval_days = card.interval_days.min(MAX_INTERVAL_DAYS);
            (card.interval_days * 86_400.0).round() as i64
        };
        // An unparseable `at` cannot happen through `record`, which validates it;
        // falling back to `at` keeps a manually edited file usable.
        card.due = add_seconds(at, seconds).unwrap_or_else(|| at.to_string());
    }
}

/// One character's progress, as the interface needs it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardView {
    pub ch: char,
    pub attempts: u32,
    pub lapses: u32,
    pub best_score: Option<f32>,
    pub last_score: Option<f32>,
    pub last_practised: Option<String>,
    pub due: String,
    pub interval_days: f32,
    pub ease: f32,
    pub repetitions: u32,
    pub history: Vec<Attempt>,
    /// Whether the due date has already passed, resolved here so the interface
    /// never has to compare clocks.
    pub due_now: bool,
}

/// What the interface needs to render progress.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressView {
    /// Every practised character, most overdue first. A character with no card
    /// here is one that has never been attempted.
    pub cards: Vec<CardView>,
    /// Set when the change was applied in memory but could not be saved, so the
    /// interface can say so instead of silently losing data.
    #[serde(default)]
    pub warning: Option<String>,
}

/// The persisted progress document.
///
/// Public because a backing store other than the JSON file — a database, in
/// practice — has to build one. The shape is the store's, not the database's:
/// a sink translates.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub version: u32,
    /// Keyed by the character itself. A `BTreeMap` keeps the file in a stable
    /// order, so it diffs cleanly and two runs write identical bytes.
    #[serde(default)]
    pub cards: BTreeMap<String, CardState>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            cards: BTreeMap::new(),
        }
    }
}

/// One attempt on its way to wherever attempts are kept.
///
/// The schedule needs only the newest few to show a learner; a database that
/// keeps an unbounded log wants every one, which is why the store hands them
/// over explicitly rather than leaving a sink to guess from the bounded
/// [`CardState::history`].
#[derive(Clone, Debug, PartialEq)]
pub struct AttemptRecord {
    /// The character this attempt was on.
    pub ch: String,
    pub attempt: Attempt,
    /// The grading measures behind the attempt's headline score, when the caller
    /// has them.
    ///
    /// The schedule needs none of these, and nothing recomputes them: an
    /// attempt's strokes are gone by the time it is saved. They are the only
    /// record of *how* an attempt went wrong, which is what the grading
    /// tolerances have to be checked against, so they travel beside the attempt
    /// rather than being derived later. `None` is a real state — an attempt
    /// merged from a peer carries only the score, and so does one recorded
    /// before these existed.
    pub measures: Option<AttemptMeasures>,
}

/// The per-measure grading result behind one attempt's headline score.
///
/// Every field is `0..=1` except the two verdicts. The names are
/// [`GradeReport`](crate::grade::GradeReport)'s with `_score` dropped, because
/// this *is* that report reduced to the part worth keeping: enough to re-check
/// the shape tolerance and the four headline weights against real handwriting,
/// and nothing that can be recomputed.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptMeasures {
    /// Mean stroke-shape agreement.
    pub shape: f32,
    /// Mean stroke-placement agreement.
    pub position: f32,
    /// How much of the character's ink was put down.
    pub ink: f32,
    /// How much of the ink a correct trace would touch the attempt touched.
    ///
    /// Reported rather than scored, but kept: it is the "you never drew that
    /// part" signal, and ink is a quarter of the headline score, so a weight
    /// tuned without it would be tuned half-blind.
    pub ink_coverage: f32,
    /// How much of the character was written in the correct order.
    pub order: f32,
    /// Strokes recognisable and correctly placed. Independent of order.
    pub legible: bool,
    pub order_correct: bool,
}

impl AttemptMeasures {
    /// The measures behind a graded attempt.
    pub fn from_report(report: &crate::grade::GradeReport) -> Self {
        Self {
            shape: report.shape_score,
            position: report.position_score,
            ink: report.ink_score,
            ink_coverage: report.ink_coverage,
            order: report.order_score,
            legible: report.legible,
            order_correct: report.order_correct,
        }
    }
}

/// Where a schedule is kept.
///
/// The JSON file at [`ProgressStore::path`] is the built-in backing and needs no
/// sink. This trait is the seam for a different one — the SQLite store lives in
/// its own crate precisely so that this one keeps no native dependency — and it
/// is *incremental* on purpose: a whole-document file can be rewritten on every
/// attempt, but the attempt log cannot, which is the ceiling the database
/// exists to lift.
pub trait ProgressSink: std::fmt::Debug + Send {
    /// Read the whole schedule, once, when the store is opened.
    ///
    /// A schedule that has never been written is an empty one, not an error.
    fn load(&self) -> Result<Document, ProgressError>;

    /// Persist `changed` cards and every attempt recorded since the last save.
    ///
    /// `changed` names the characters whose card differs from what is already
    /// stored. Returning an error leaves the store holding the change, so the
    /// next save tries again rather than dropping it.
    fn save(
        &mut self,
        document: &Document,
        changed: &BTreeSet<String>,
        attempts: &[AttemptRecord],
    ) -> Result<(), ProgressError>;
}

/// Where a due character should be practised from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSource {
    /// A character in the built-in course.
    Course,
    /// A character of an entry in the personal vocabulary list.
    Vocabulary,
}

/// One thing to review.
///
/// For a vocabulary entry this describes the *entry*, not just the character
/// that happened to come due: a word is practised whole, so it appears once
/// however many of its characters are due.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewItem {
    /// The character whose card came due.
    pub ch: char,
    /// When it came due, ISO-8601 UTC. The queue is ordered by this.
    pub due: String,
    pub source: ReviewSource,
    /// The vocabulary entry, when the character belongs to one.
    pub entry_id: Option<u64>,
    /// The text to write: an entry's word, or the character itself.
    pub text: String,
}

/// The review queue, plus how much of it is not shown.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewView {
    pub items: Vec<ReviewItem>,
    /// How many items are due in total, before any limit was applied.
    pub due_count: usize,
    #[serde(default)]
    pub warning: Option<String>,
}

/// The practice schedule, backed by a JSON file or by a [`ProgressSink`].
#[derive(Debug)]
pub struct ProgressStore {
    path: PathBuf,
    document: Document,
    /// Set when the schedule is kept somewhere other than the JSON file at
    /// `path`. `None` is the JSON file, which is what every test uses.
    sink: Option<Box<dyn ProgressSink>>,
    /// Characters whose card has changed since the last save.
    dirty: BTreeSet<String>,
    /// Attempts recorded since the last save, oldest first.
    pending: Vec<AttemptRecord>,
}

impl ProgressStore {
    /// Open the schedule at `path`, creating an empty one if the file is absent.
    ///
    /// A missing file is normal (first run). A *corrupt* file is a hard error:
    /// starting empty would look like a term of study had vanished.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ProgressError> {
        let path = path.into();
        let document = read_document(&path)?;
        Ok(Self::from_parts(path, document, None))
    }

    /// Open a schedule kept by `sink` rather than by a JSON file.
    ///
    /// Everything above this line is the same either way: the store holds the
    /// schedule in memory and the sink only decides where it lives.
    pub fn open_with(sink: Box<dyn ProgressSink>) -> Result<Self, ProgressError> {
        let document = sink.load()?;
        Ok(Self::from_parts(PathBuf::new(), document, Some(sink)))
    }

    fn from_parts(path: PathBuf, document: Document, sink: Option<Box<dyn ProgressSink>>) -> Self {
        Self {
            path,
            document,
            sink,
            dirty: BTreeSet::new(),
            pending: Vec::new(),
        }
    }

    /// An in-memory schedule with no file behind it. Used by tests.
    pub fn in_memory() -> Self {
        Self::from_parts(PathBuf::new(), Document::default(), None)
    }

    /// The file this schedule is kept in, or an empty path when a sink holds it.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The whole schedule, as a sink sees it.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Every card, keyed by character.
    pub fn cards(&self) -> &BTreeMap<String, CardState> {
        &self.document.cards
    }

    /// One character's card, if it has ever been practised.
    pub fn card(&self, ch: char) -> Option<&CardState> {
        self.document.cards.get(&ch.to_string())
    }

    /// True when the character has never been attempted.
    pub fn is_new(&self, ch: char) -> bool {
        self.card(ch).is_none()
    }

    /// Every character due at `now`, most overdue first.
    ///
    /// Ordered by the due string, which is the same as ordering by urgency
    /// because these timestamps sort chronologically as text. The character
    /// breaks ties so the order is total and reproducible.
    pub fn due(&self, now: &str) -> Vec<(char, &CardState)> {
        let mut due: Vec<(char, &CardState)> = self
            .document
            .cards
            .iter()
            .filter_map(|(key, card)| {
                let ch = key.chars().next()?;
                card.is_due(now).then_some((ch, card))
            })
            .collect();
        due.sort_by(|(a_ch, a), (b_ch, b)| a.due.cmp(&b.due).then_with(|| a_ch.cmp(b_ch)));
        due
    }

    /// The whole schedule as the interface sees it.
    pub fn view(&self) -> ProgressView {
        self.view_at(&now_iso8601())
    }

    /// [`Self::view`] against a caller-supplied clock, which is what makes the
    /// `dueNow` flag testable.
    pub fn view_at(&self, now: &str) -> ProgressView {
        let mut cards: Vec<CardView> = self
            .document
            .cards
            .iter()
            .filter_map(|(key, card)| Some(card.view(key.chars().next()?, now)))
            .collect();
        cards.sort_by(|a, b| a.due.cmp(&b.due).then_with(|| a.ch.cmp(&b.ch)));
        ProgressView {
            cards,
            warning: None,
        }
    }

    /// Persist the schedule.
    ///
    /// Through a sink only what changed is written; without one the JSON file is
    /// rewritten whole, atomically — temporary file first, then rename, so an
    /// interrupted write cannot leave a half-written schedule behind.
    pub fn save(&mut self) -> Result<(), ProgressError> {
        match self.sink.as_mut() {
            Some(sink) => {
                sink.save(&self.document, &self.dirty, &self.pending)?;
                // Only now is the change safely stored; an error above leaves it
                // pending so the next save retries rather than dropping it.
                self.dirty.clear();
                self.pending.clear();
                Ok(())
            }
            None => save_document(&self.path, &self.document),
        }
    }

    /// Record an attempt made now.
    pub fn record(&mut self, ch: char, score: f32) -> Result<CardView, ProgressError> {
        self.record_measured(ch, score, None)
    }

    /// Record an attempt made now, keeping the measures it was graded from.
    ///
    /// This is what the app calls: the headline score is one number, and the
    /// four weights behind it can only be checked against the measures that
    /// produced it.
    pub fn record_measured(
        &mut self,
        ch: char,
        score: f32,
        measures: Option<AttemptMeasures>,
    ) -> Result<CardView, ProgressError> {
        self.record_with(ch, score, &now_iso8601(), &Sm2, measures)
    }

    /// Record an attempt at a caller-supplied time, scheduled by [`Sm2`].
    pub fn record_at(
        &mut self,
        ch: char,
        score: f32,
        at: &str,
    ) -> Result<CardView, ProgressError> {
        self.record_with(ch, score, at, &Sm2, None)
    }

    /// Record an attempt at a caller-supplied time under a chosen policy.
    ///
    /// This is what the [`Scheduler`] trait is for: the store does not care how
    /// the next due date is chosen, only that it is. `measures` reaches the sink
    /// untouched — the schedule has no opinion about it.
    pub fn record_with(
        &mut self,
        ch: char,
        score: f32,
        at: &str,
        scheduler: &dyn Scheduler,
        measures: Option<AttemptMeasures>,
    ) -> Result<CardView, ProgressError> {
        if parse_iso8601(at).is_none() {
            return Err(ProgressError::InvalidTimestamp(at.to_string()));
        }
        let score = score.clamp(0.0, 100.0);

        let card = self
            .document
            .cards
            .entry(ch.to_string())
            .or_insert_with(|| CardState::new(at));

        let recorded = apply_attempt(card, score, at, scheduler);

        // What a sink has to write: the card, and the attempt itself, which is
        // the one thing the bounded in-memory history cannot be trusted to keep.
        let key = ch.to_string();
        self.dirty.insert(key.clone());
        self.pending.push(AttemptRecord {
            ch: key.clone(),
            attempt: recorded,
            measures,
        });

        // Re-borrow immutably: the card is certainly there, it was just inserted.
        let card = self
            .document
            .cards
            .get(&key)
            .expect("the card was just inserted");
        Ok(card.view(ch, at))
    }
}

/// Apply one recorded attempt to a card, in the one order that is correct.
///
/// Pulled out of [`ProgressStore::record_with`] so that the log fold below, which
/// rebuilds a card from its attempts, cannot drift from the way the card was built
/// the first time. If the two ever disagreed, two devices would hold different
/// schedules for the same history and neither could tell which was right.
fn apply_attempt(
    card: &mut CardState,
    score: f32,
    at: &str,
    scheduler: &dyn Scheduler,
) -> Attempt {
    let rating = Rating::from_score(score);
    card.attempts += 1;
    card.last_score = Some(score);
    card.best_score = Some(match card.best_score {
        Some(best) => best.max(score),
        None => score,
    });
    card.last_practised = Some(at.to_string());
    let recorded = Attempt {
        at: at.to_string(),
        score,
        rating,
    };
    card.history.push(recorded.clone());
    if card.history.len() > MAX_HISTORY {
        let excess = card.history.len() - MAX_HISTORY;
        card.history.drain(..excess);
    }
    scheduler.review(card, rating, at);
    recorded
}

/// Rebuild a card from the attempts that produced it.
///
/// This is the property cross-device sync rests on (ROADMAP M13): a card is never
/// *merged* between devices, it is recomputed. Because [`apply_attempt`] is the
/// only thing that ever advanced a card, folding the same attempts in the same
/// order reproduces the same card — so two devices that practised the same
/// character while apart agree on one schedule as soon as each has the other's
/// log, with nothing to resolve and no winner to pick.
///
/// `attempts` must be in the order they happened, and the caller owns that order.
/// The engine deliberately knows nothing about devices, shards or clocks: it is
/// handed a sequence and returns the schedule it implies. An empty log has no
/// card in it, so this returns `None` rather than inventing one.
///
/// A log that does not go back to the character's first attempt reconstructs only
/// the part of the history it holds; [`fold_from`] is the same fold with the rest
/// of it supplied.
pub fn fold_attempts(attempts: &[Attempt], scheduler: &dyn Scheduler) -> Option<CardState> {
    let first = attempts.first()?;
    // The same starting state `record_with` gives a character it has never seen:
    // unseen, and due at the moment of its first attempt.
    Some(fold_from(CardState::new(&first.at), attempts, scheduler))
}

/// Rebuild a card from a state a log cannot reach, and the attempts that followed.
///
/// For a card whose log is incomplete — a schedule migrated from the pre-M10 JSON
/// files, which kept only the newest twenty attempts — the surviving rows are the
/// *tail* of the history, and folding them from an unseen card would rebuild a
/// schedule out of a fraction of what happened. `baseline` is what the history
/// before that tail left behind, so the fold is the whole story again: the state,
/// then every attempt the log holds that the state does not already account for.
///
/// It is deliberately the same [`apply_attempt`] the other path calls. A separate
/// implementation could drift from it, and a device folding from a baseline would
/// then disagree with one folding from a complete log — which is the silent
/// divergence this whole design exists to prevent.
pub fn fold_from(
    mut baseline: CardState,
    attempts: &[Attempt],
    scheduler: &dyn Scheduler,
) -> CardState {
    for attempt in attempts {
        apply_attempt(&mut baseline, attempt.score, &attempt.at, scheduler);
    }
    baseline
}

/// Build the review queue.
///
/// Everything due at `now`, most overdue first, drawn from both the course and
/// the vocabulary list. A character that is in a vocabulary entry is reported as
/// that entry — a word is practised whole, and its meaning is the context worth
/// keeping — and appears once however many of its characters are due. A
/// character in neither the course nor the list is skipped: there would be
/// nothing to practise it from.
///
/// The whole queue is returned; taking only the first few is a decision for
/// whoever shows it, and needs the full count to say how much is left behind.
pub fn build_queue(
    store: &ProgressStore,
    course: &HashSet<char>,
    entries: &[Entry],
    now: &str,
) -> Vec<ReviewItem> {
    // The vocabulary entry a character belongs to. The lowest id wins so the
    // choice is stable when the same character is in several entries.
    let mut in_entry: BTreeMap<char, u64> = BTreeMap::new();
    for entry in entries {
        for ch in entry.text.chars() {
            in_entry
                .entry(ch)
                .and_modify(|id| *id = (*id).min(entry.id))
                .or_insert(entry.id);
        }
    }

    let mut items: Vec<ReviewItem> = Vec::new();
    let mut seen_entries: HashSet<u64> = HashSet::new();

    for (ch, card) in store.due(now) {
        if let Some(&entry_id) = in_entry.get(&ch) {
            if !seen_entries.insert(entry_id) {
                continue;
            }
            let text = entries
                .iter()
                .find(|e| e.id == entry_id)
                .map(|e| e.text.clone())
                .unwrap_or_else(|| ch.to_string());
            items.push(ReviewItem {
                ch,
                due: card.due.clone(),
                source: ReviewSource::Vocabulary,
                entry_id: Some(entry_id),
                text,
            });
        } else if course.contains(&ch) {
            items.push(ReviewItem {
                ch,
                due: card.due.clone(),
                source: ReviewSource::Course,
                entry_id: None,
                text: ch.to_string(),
            });
        }
    }

    items
}

/// The course cursor: where the reader was last looking.
///
/// Deliberately a different file from the schedule. Exploring the course moves
/// the cursor constantly, and a corrupt file — one of these two is far more
/// likely to be hand-edited than the other — must not take the other down.
#[derive(Debug)]
pub struct CursorStore {
    path: PathBuf,
    document: CursorDocument,
    /// Set when the cursor is kept somewhere other than the JSON file at `path`.
    sink: Option<Box<dyn CursorSink>>,
}

/// The persisted cursor document. Public for the same reason as
/// [`Document`]: a sink has to build one.
///
/// Every field but the version is optional, so a document written by a newer
/// build fails the version check rather than the schema check — the message the
/// reader actually needs.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorDocument {
    pub version: u32,
    /// Position in the flattened course, as a count of characters.
    #[serde(default)]
    pub index: usize,
    /// When it was last moved, for a "continue where I left off" line.
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl Default for CursorDocument {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            index: 0,
            updated_at: None,
        }
    }
}

/// Where the course cursor is kept. See [`ProgressSink`]; one row is all there
/// is to write, so this needs no notion of a change set.
pub trait CursorSink: std::fmt::Debug + Send {
    fn load(&self) -> Result<CursorDocument, ProgressError>;
    fn save(&mut self, document: &CursorDocument) -> Result<(), ProgressError>;
}

impl CursorStore {
    /// Open the cursor at `path`, defaulting to the start when it is absent.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ProgressError> {
        let path = path.into();
        let document = match fs::read_to_string(&path) {
            Ok(text) => {
                let parsed: CursorDocument = serde_json::from_str(&text)
                    .map_err(|e| ProgressError::Malformed(e.to_string()))?;
                if parsed.version > FORMAT_VERSION {
                    return Err(ProgressError::UnsupportedVersion(parsed.version));
                }
                parsed
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => CursorDocument::default(),
            Err(e) => return Err(ProgressError::Io(format!("{}: {e}", path.display()))),
        };
        Ok(Self {
            path,
            document,
            sink: None,
        })
    }

    /// Open a cursor kept by `sink` rather than by a JSON file.
    pub fn open_with(sink: Box<dyn CursorSink>) -> Result<Self, ProgressError> {
        let document = sink.load()?;
        Ok(Self {
            path: PathBuf::new(),
            document,
            sink: Some(sink),
        })
    }

    /// An in-memory cursor with no file behind it. Used by tests.
    pub fn in_memory() -> Self {
        Self {
            path: PathBuf::new(),
            document: CursorDocument::default(),
            sink: None,
        }
    }

    /// The file this cursor is kept in, or an empty path when a sink holds it.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The whole cursor, as a sink sees it.
    pub fn document(&self) -> &CursorDocument {
        &self.document
    }

    pub fn index(&self) -> usize {
        self.document.index
    }

    /// When the cursor last moved, if it ever has.
    pub fn updated_at(&self) -> Option<&str> {
        self.document.updated_at.as_deref()
    }

    /// Move the cursor. Setting the same position does not count as a move, so
    /// merely looking at the course does not keep rewriting the file.
    pub fn set_index(&mut self, index: usize) -> bool {
        if self.document.index == index {
            return false;
        }
        self.document.index = index;
        self.document.updated_at = Some(now_iso8601());
        true
    }

    pub fn view(&self) -> CursorView {
        CursorView {
            index: self.document.index,
            updated_at: self.document.updated_at.clone(),
            warning: None,
        }
    }

    /// Persist the cursor; through a sink, or atomically to the JSON file.
    pub fn save(&mut self) -> Result<(), ProgressError> {
        match self.sink.as_mut() {
            Some(sink) => sink.save(&self.document),
            None => save_document(&self.path, &self.document),
        }
    }
}

/// Where the reader was, as the interface needs it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorView {
    /// Position in the flattened course.
    pub index: usize,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub warning: Option<String>,
}

/// Read and validate a versioned document, treating absence as a fresh start.
fn read_document(path: &Path) -> Result<Document, ProgressError> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let parsed: Document = serde_json::from_str(&text)
                .map_err(|e| ProgressError::Malformed(e.to_string()))?;
            if parsed.version > FORMAT_VERSION {
                return Err(ProgressError::UnsupportedVersion(parsed.version));
            }
            Ok(parsed)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Document::default()),
        Err(e) => Err(ProgressError::Io(format!("{}: {e}", path.display()))),
    }
}

/// Write a document atomically: temporary file first, then rename.
fn save_document<T: Serialize>(path: &Path, document: &T) -> Result<(), ProgressError> {
    if path.as_os_str().is_empty() {
        return Ok(()); // in-memory store
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| ProgressError::Io(format!("{}: {e}", parent.display())))?;
        }
    }
    let json = serde_json::to_string_pretty(document)
        .map_err(|e| ProgressError::Io(e.to_string()))?;

    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, json)
        .map_err(|e| ProgressError::Io(format!("{}: {e}", temporary.display())))?;
    fs::rename(&temporary, path).map_err(|e| ProgressError::Io(format!("{}: {e}", path.display())))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("hanzi-progress-{name}-{unique}.json"))
    }

    fn entry(id: u64, text: &str) -> Entry {
        Entry {
            id,
            text: text.to_string(),
            pinyin: String::new(),
            meaning: String::new(),
            group: None,
            added_at: "2026-01-01T00:00:00Z".to_string(),
            attempts: 0,
            best_score: None,
            last_practised: None,
        }
    }

    fn course(chars: &str) -> HashSet<char> {
        chars.chars().collect()
    }

    // ---- ratings ----------------------------------------------------------

    #[test]
    fn ratings_follow_the_grades_the_panel_shows() {
        assert_eq!(Rating::from_score(0.0), Rating::Again);
        assert_eq!(Rating::from_score(54.9), Rating::Again);
        assert_eq!(Rating::from_score(55.0), Rating::Hard);
        assert_eq!(Rating::from_score(74.9), Rating::Hard);
        assert_eq!(Rating::from_score(75.0), Rating::Good);
        assert_eq!(Rating::from_score(91.9), Rating::Good);
        assert_eq!(Rating::from_score(92.0), Rating::Easy);
        assert_eq!(Rating::from_score(100.0), Rating::Easy);

        // And the same boundaries the engine itself uses.
        for score in [0.0, 12.5, 54.9, 55.0, 74.9, 75.0, 91.9, 92.0, 100.0] {
            let expected = match Grade::from_score(score) {
                Grade::Excellent => Rating::Easy,
                Grade::Good => Rating::Good,
                Grade::Fair => Rating::Hard,
                Grade::Poor => Rating::Again,
            };
            assert_eq!(Rating::from_score(score), expected, "at {score}");
        }
    }

    // ---- recording --------------------------------------------------------

    #[test]
    fn a_first_attempt_creates_the_card_and_keeps_the_attempt() {
        let mut store = ProgressStore::in_memory();
        assert!(store.is_new('好'));

        let card = store.record_at('好', 88.0, "2026-09-19T09:00:00Z").unwrap();
        assert_eq!(card.attempts, 1);
        assert_eq!(card.best_score, Some(88.0));
        assert_eq!(card.last_score, Some(88.0));
        assert_eq!(card.last_practised.as_deref(), Some("2026-09-19T09:00:00Z"));
        assert_eq!(card.history.len(), 1);
        assert_eq!(card.history[0].score, 88.0);
        assert_eq!(card.history[0].rating, Rating::Good);
        assert!(!store.is_new('好'));
    }

    #[test]
    fn a_worse_attempt_does_not_lower_the_best() {
        let mut store = ProgressStore::in_memory();
        store.record_at('好', 90.0, "2026-09-19T09:00:00Z").unwrap();
        let card = store.record_at('好', 40.0, "2026-09-19T10:00:00Z").unwrap();

        assert_eq!(card.attempts, 2);
        assert_eq!(card.best_score, Some(90.0));
        assert_eq!(card.last_score, Some(40.0));
        assert_eq!(card.lapses, 1, "a poor attempt is a lapse");
        assert_eq!(card.history.len(), 2);
    }

    #[test]
    fn scores_are_clamped_and_timestamps_validated() {
        let mut store = ProgressStore::in_memory();
        assert_eq!(
            store.record_at('好', 140.0, "2026-09-19T09:00:00Z").unwrap().best_score,
            Some(100.0)
        );
        assert_eq!(
            store.record_at('坏', -20.0, "2026-09-19T09:00:00Z").unwrap().last_score,
            Some(0.0)
        );
        assert!(matches!(
            store.record_at('中', 80.0, "yesterday"),
            Err(ProgressError::InvalidTimestamp(_))
        ));
        assert!(store.is_new('中'), "a rejected attempt must not create a card");
    }

    #[test]
    fn history_is_capped_but_the_totals_are_not() {
        let start = "2026-09-01T00:00:00Z";
        let mut store = ProgressStore::in_memory();
        for i in 0..MAX_HISTORY + 10 {
            let at = add_seconds(start, i as i64 * 3600).unwrap();
            store.record_at('好', 80.0, &at).unwrap();
        }
        let card = store.card('好').unwrap();
        assert_eq!(card.attempts as usize, MAX_HISTORY + 10);
        assert_eq!(card.history.len(), MAX_HISTORY);
        // The oldest are dropped, the newest kept.
        assert_eq!(
            card.history.last().unwrap().at,
            add_seconds(start, (MAX_HISTORY + 9) as i64 * 3600).unwrap()
        );
        assert_eq!(
            card.history.first().unwrap().at,
            add_seconds(start, 10 * 3600).unwrap(),
            "the first ten attempts have rolled off"
        );
    }

    // ---- scheduling -------------------------------------------------------

    #[test]
    fn a_good_attempt_is_scheduled_further_out_than_a_bad_one() {
        let at = "2026-09-19T09:00:00Z";
        let mut dues = Vec::new();
        for (ch, score) in [('差', 20.0), ('中', 60.0), ('好', 80.0), ('优', 96.0)] {
            let mut store = ProgressStore::in_memory();
            let card = store.record_at(ch, score, at).unwrap();
            dues.push((score, card.due.clone(), card.interval_days));
        }

        assert!(
            dues[0].1 < dues[1].1 && dues[1].1 < dues[2].1 && dues[2].1 < dues[3].1,
            "due dates must increase with the score: {dues:?}"
        );
        assert_eq!(dues[0].2, 0.0, "a failed attempt returns in the minute");
        assert_eq!(dues[1].2, 0.5, "hard: twelve hours");
        assert_eq!(dues[2].2, 1.0, "good: a day");
        assert_eq!(dues[3].2, 2.0, "easy: two days");
        assert_eq!(dues[0].1, "2026-09-19T09:01:00Z");
        assert_eq!(dues[1].1, "2026-09-19T21:00:00Z");
        assert_eq!(dues[2].1, "2026-09-20T09:00:00Z");
        assert_eq!(dues[3].1, "2026-09-21T09:00:00Z");
    }

    #[test]
    fn repeated_success_stretches_the_interval() {
        let mut store = ProgressStore::in_memory();
        let mut at = "2026-09-01T08:00:00Z".to_string();

        let first = store.record_at('好', 80.0, &at).unwrap();
        assert_eq!(first.interval_days, 1.0);
        at = first.due.clone();

        let second = store.record_at('好', 80.0, &at).unwrap();
        assert_eq!(second.interval_days, 6.0);
        at = second.due.clone();

        let third = store.record_at('好', 80.0, &at).unwrap();
        assert!(
            third.interval_days >= 6.0 * MIN_EASE,
            "the third interval multiplies by ease: {}",
            third.interval_days
        );
        assert!(third.due > second.due, "and the due date moves later");
    }

    #[test]
    fn a_failure_resets_the_progress_and_lowers_ease() {
        let mut store = ProgressStore::in_memory();
        let mut at = "2026-09-01T08:00:00Z".to_string();
        for _ in 0..3 {
            at = store.record_at('好', 85.0, &at).unwrap().due;
        }
        let before = store.card('好').unwrap().clone();
        assert!(before.repetitions >= 3);
        assert!(before.interval_days > 1.0);

        let after = store.record_at('好', 20.0, &at).unwrap();
        assert_eq!(after.repetitions, 0);
        assert_eq!(after.lapses, 1);
        assert_eq!(after.interval_days, 0.0);
        assert!(after.ease < before.ease, "a failure costs ease");
        // Due almost at once, and the attempt is not lost.
        assert_eq!(after.due, add_seconds(&at, AGAIN_SECONDS).unwrap());
        assert_eq!(after.attempts, 4);
    }

    #[test]
    fn ease_stays_within_its_bounds() {
        let mut store = ProgressStore::in_memory();
        let mut at = "2026-09-01T08:00:00Z".to_string();
        for _ in 0..30 {
            at = store.record_at('笨', 10.0, &at).unwrap().due;
        }
        assert_eq!(store.card('笨').unwrap().ease, MIN_EASE);
        assert_eq!(
            store.card('笨').unwrap().interval_days,
            0.0,
            "a character that keeps failing never gets an interval"
        );

        let mut at = "2026-09-01T08:00:00Z".to_string();
        let mut last_review = at.clone();
        for _ in 0..30 {
            last_review = at.clone();
            at = store.record_at('优', 100.0, &at).unwrap().due;
        }
        assert_eq!(store.card('优').unwrap().ease, MAX_EASE);
        // In particular the interval is bounded, so the due date stays a date the
        // fixed-width timestamp format can hold.
        assert_eq!(store.card('优').unwrap().interval_days, MAX_INTERVAL_DAYS);
        assert_eq!(
            at,
            add_seconds(&last_review, (MAX_INTERVAL_DAYS as i64) * 86_400).unwrap(),
            "an easy review is never scheduled more than a year out"
        );
    }

    #[test]
    fn the_scheduler_is_swappable() {
        /// Ignores the rating and always pushes a review a year out.
        struct Annually;
        impl Scheduler for Annually {
            fn review(&self, card: &mut CardState, _rating: Rating, at: &str) {
                card.repetitions += 1;
                card.interval_days = 365.0;
                card.due = add_seconds(at, 365 * 86_400).unwrap();
            }
        }

        let mut store = ProgressStore::in_memory();
        let card = store
            .record_with('好', 30.0, "2026-09-19T09:00:00Z", &Annually, None)
            .unwrap();
        assert_eq!(card.interval_days, 365.0);
        assert_eq!(card.due, "2027-09-19T09:00:00Z");
        // The attempt itself is still recorded by the store, whatever the policy.
        assert_eq!(card.attempts, 1);
        assert_eq!(card.history[0].rating, Rating::Again);
    }

    // ---- due-ness and the queue -------------------------------------------

    #[test]
    fn nothing_is_due_before_its_date_and_everything_is_after() {
        let mut store = ProgressStore::in_memory();
        store.record_at('好', 80.0, "2026-09-19T09:00:00Z").unwrap();
        store.record_at('坏', 20.0, "2026-09-19T09:00:00Z").unwrap();

        // A good attempt is due tomorrow, a failed one within the minute.
        assert!(store.due("2026-09-19T09:00:30Z").is_empty());
        assert_eq!(store.due("2026-09-19T09:01:00Z").len(), 1);
        assert_eq!(store.due("2026-09-20T09:00:00Z").len(), 2);
    }

    #[test]
    fn the_queue_is_empty_after_a_good_session_and_fills_when_due() {
        let at = "2026-09-19T09:00:00Z";
        let mut store = ProgressStore::in_memory();
        let lesson = course("好学习");

        for ch in "好学习".chars() {
            store.record_at(ch, 95.0, at).unwrap();
        }

        // Immediately after a good session there is nothing to do…
        assert!(build_queue(&store, &lesson, &[], at).is_empty());
        // …and two days later everything is back.
        let later = add_seconds(at, 3 * 86_400).unwrap();
        let queue = build_queue(&store, &lesson, &[], &later);
        assert_eq!(queue.len(), 3);
        assert!(queue.iter().all(|i| i.source == ReviewSource::Course));
    }

    #[test]
    fn the_queue_is_ordered_by_how_overdue_it_is() {
        let at = "2026-09-19T09:00:00Z";
        let mut store = ProgressStore::in_memory();
        let lesson = course("甲乙丙");

        // 甲 failed an hour before the others, so it is the most overdue.
        store.record_at('甲', 20.0, "2026-09-19T08:00:00Z").unwrap();
        store.record_at('乙', 20.0, at).unwrap();
        store.record_at('丙', 20.0, at).unwrap();

        let now = "2026-09-19T12:00:00Z";
        let queue = build_queue(&store, &lesson, &[], now);
        assert_eq!(queue[0].ch, '甲', "the most overdue comes first");
        assert!(
            queue.windows(2).all(|w| w[0].due <= w[1].due),
            "and the rest follow in due order: {queue:?}"
        );
    }

    #[test]
    fn a_due_character_in_a_word_is_reviewed_as_that_word() {
        let at = "2026-09-19T09:00:00Z";
        let mut store = ProgressStore::in_memory();
        let entries = vec![entry(7, "学习"), entry(9, "好人")];
        store.record_at('学', 20.0, at).unwrap();
        store.record_at('习', 20.0, at).unwrap();
        store.record_at('好', 20.0, at).unwrap();

        let now = "2026-09-19T12:00:00Z";
        let queue = build_queue(&store, &course("好坏学习"), &entries, now);

        // 学习 is one item, not two, and there is no bare 好: the word is the unit.
        assert_eq!(queue.len(), 2);
        let word = queue.iter().find(|i| i.text == "学习").unwrap();
        assert_eq!(word.entry_id, Some(7));
        assert_eq!(word.source, ReviewSource::Vocabulary);
        assert!(queue.iter().any(|i| i.text == "好人" && i.entry_id == Some(9)));
    }

    #[test]
    fn a_character_in_both_sources_prefers_the_vocabulary_entry() {
        let at = "2026-09-19T09:00:00Z";
        let mut store = ProgressStore::in_memory();
        store.record_at('好', 20.0, at).unwrap();

        let entries = vec![entry(1, "好")];
        let queue = build_queue(&store, &course("好"), &entries, "2026-09-19T12:00:00Z");
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].source, ReviewSource::Vocabulary);
        assert_eq!(queue[0].entry_id, Some(1));
    }

    #[test]
    fn characters_in_neither_the_course_nor_the_list_are_skipped() {
        let mut store = ProgressStore::in_memory();
        store.record_at('龙', 20.0, "2026-09-19T09:00:00Z").unwrap();
        let queue = build_queue(&store, &course("好"), &[], "2026-09-19T12:00:00Z");
        assert!(queue.is_empty(), "nothing to practise it from");
    }

    #[test]
    fn the_queue_holds_everything_that_is_due() {
        let at = "2026-09-19T09:00:00Z";
        let mut store = ProgressStore::in_memory();
        let mut lesson = HashSet::new();
        for ch in "一二三四五六七八九十".chars() {
            lesson.insert(ch);
            store.record_at(ch, 20.0, at).unwrap();
        }

        let now = "2026-09-19T12:00:00Z";
        assert_eq!(build_queue(&store, &lesson, &[], now).len(), 10, "all due");
        // Truncating is the caller's job, so the count it reports stays honest.
        assert_eq!(build_queue(&store, &lesson, &[], at).len(), 0, "none due yet");
    }

    #[test]
    fn the_view_marks_what_is_due_now() {
        let mut store = ProgressStore::in_memory();
        store.record_at('好', 80.0, "2026-09-19T09:00:00Z").unwrap();
        store.record_at('坏', 20.0, "2026-09-19T09:00:00Z").unwrap();

        let view = store.view_at("2026-09-19T09:30:00Z");
        assert_eq!(view.cards.len(), 2);
        let due: Vec<char> = view.cards.iter().filter(|c| c.due_now).map(|c| c.ch).collect();
        assert_eq!(due, vec!['坏'], "only the failed one is due");

        // Ordered most overdue first, and every card carries its history.
        assert_eq!(view.cards[0].ch, '坏');
        assert_eq!(view.cards[0].history.len(), 1);
        assert_eq!(view.cards[0].history[0].rating, Rating::Again);
    }

    // ---- persistence ------------------------------------------------------

    #[test]
    fn practice_survives_a_restart() {
        let path = temp_path("restart");
        {
            let mut store = ProgressStore::open(&path).unwrap();
            store.record_at('好', 87.0, "2026-09-19T09:00:00Z").unwrap();
            store.record_at('好', 93.0, "2026-09-20T09:00:00Z").unwrap();
            store.record_at('学', 40.0, "2026-09-20T09:05:00Z").unwrap();
            store.save().unwrap();
        }

        let reopened = ProgressStore::open(&path).unwrap();
        assert!(!reopened.is_new('好'), "a practised character is not new");
        let card = reopened.card('好').unwrap();
        assert_eq!(card.attempts, 2);
        assert_eq!(card.best_score, Some(93.0));
        assert_eq!(card.history.len(), 2);
        assert_eq!(card.history[0].score, 87.0);
        assert!(card.due.as_str() > "2026-09-20T09:00:00Z");
        assert_eq!(reopened.card('学').unwrap().attempts, 1);
        assert!(reopened.is_new('坏'));

        fs::remove_file(&path).ok();
    }

    #[test]
    fn a_missing_file_is_an_empty_schedule_not_an_error() {
        let path = temp_path("absent");
        let store = ProgressStore::open(&path).unwrap();
        assert!(store.cards().is_empty());
        assert!(store.view().cards.is_empty());
        assert!(store.is_new('好'));
    }

    #[test]
    fn a_corrupt_file_is_an_error_rather_than_silent_data_loss() {
        let path = temp_path("corrupt");
        fs::write(&path, "{ this is not json").unwrap();
        let error = ProgressStore::open(&path).unwrap_err();
        assert!(matches!(error, ProgressError::Malformed(_)), "{error}");

        // Truncated mid-document must also be caught.
        fs::write(&path, r#"{"version":1,"cards":{"好":{"attempts":1,"#).unwrap();
        assert!(matches!(
            ProgressStore::open(&path).unwrap_err(),
            ProgressError::Malformed(_)
        ));

        fs::remove_file(&path).ok();
    }

    #[test]
    fn a_future_format_version_is_refused() {
        let path = temp_path("future");
        fs::write(
            &path,
            format!(r#"{{"version":{},"cards":{{}}}}"#, FORMAT_VERSION + 1),
        )
        .unwrap();
        assert!(matches!(
            ProgressStore::open(&path).unwrap_err(),
            ProgressError::UnsupportedVersion(_)
        ));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn saving_creates_missing_directories() {
        let dir = temp_path("nested").with_extension("dir");
        let path = dir.join("deeper/progress.json");
        let mut store = ProgressStore::open(&path).unwrap();
        store.record_at('好', 80.0, "2026-09-19T09:00:00Z").unwrap();
        store.save().unwrap();
        assert!(path.exists());
        assert_eq!(ProgressStore::open(&path).unwrap().cards().len(), 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_document_is_stable_and_readable() {
        let path = temp_path("stable");
        let mut store = ProgressStore::open(&path).unwrap();
        // Inserted in an arbitrary order; written in one fixed order.
        store.record_at('丙', 80.0, "2026-09-19T09:00:00Z").unwrap();
        store.record_at('甲', 80.0, "2026-09-19T09:00:00Z").unwrap();
        store.record_at('乙', 80.0, "2026-09-19T09:00:00Z").unwrap();
        store.save().unwrap();
        let first = fs::read_to_string(&path).unwrap();
        store.save().unwrap();
        assert_eq!(
            first,
            fs::read_to_string(&path).unwrap(),
            "writing the same document twice must produce the same bytes"
        );

        // A BTreeMap orders by code point, so this is 丙, 乙, 甲.
        let mut expected: Vec<char> = "丙甲乙".chars().collect();
        expected.sort();
        let positions: Vec<usize> = expected
            .iter()
            .map(|k| first.find(&format!("\"{k}\"")).expect("key present"))
            .collect();
        assert!(
            positions.windows(2).all(|w| w[0] < w[1]),
            "characters are written in code-point order: {positions:?}"
        );
        assert!(first.contains("\"due\""), "readable field names: {first}");
        assert!(first.contains("\"rating\": \"good\""), "readable ratings");
        fs::remove_file(&path).ok();
    }

    // ---- the cursor -------------------------------------------------------

    #[test]
    fn the_cursor_survives_a_restart() {
        let path = temp_path("cursor");
        {
            let mut cursor = CursorStore::open(&path).unwrap();
            assert_eq!(cursor.index(), 0, "a fresh install starts at the beginning");
            assert!(cursor.set_index(412));
            assert!(!cursor.set_index(412), "setting the same place is not a move");
            cursor.save().unwrap();
        }

        let reopened = CursorStore::open(&path).unwrap();
        assert_eq!(reopened.index(), 412);
        assert!(reopened.updated_at().is_some());

        fs::remove_file(&path).ok();
    }

    #[test]
    fn a_corrupt_cursor_is_refused_rather_than_reset() {
        let path = temp_path("cursor-corrupt");
        fs::write(&path, "{ not json").unwrap();
        assert!(matches!(
            CursorStore::open(&path).unwrap_err(),
            ProgressError::Malformed(_)
        ));
        // The file is left exactly as it was.
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn a_corrupt_cursor_cannot_take_the_schedule_with_it() {
        // The two live in separate files precisely so that one bad file — the
        // cursor is the more likely to be hand-edited — is contained.
        let dir = temp_path("separate").with_extension("dir");
        fs::create_dir_all(&dir).unwrap();
        let progress_path = dir.join("progress.json");
        let cursor_path = dir.join("cursor.json");

        let mut store = ProgressStore::open(&progress_path).unwrap();
        store.record_at('好', 85.0, "2026-09-19T09:00:00Z").unwrap();
        store.save().unwrap();
        fs::write(&cursor_path, "{ not json").unwrap();

        // The schedule still opens, with its history intact…
        let reopened = ProgressStore::open(&progress_path).unwrap();
        assert_eq!(reopened.card('好').unwrap().attempts, 1);
        // …while the cursor reports its own problem.
        assert!(CursorStore::open(&cursor_path).is_err());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_future_cursor_version_is_refused() {
        let path = temp_path("cursor-future");
        fs::write(&path, format!(r#"{{"version":{}}}"#, FORMAT_VERSION + 1)).unwrap();
        assert!(matches!(
            CursorStore::open(&path).unwrap_err(),
            ProgressError::UnsupportedVersion(_)
        ));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn an_in_memory_cursor_saves_silently() {
        let mut cursor = CursorStore::in_memory();
        assert!(cursor.set_index(3));
        cursor.save().unwrap();
        assert_eq!(cursor.view().index, 3);
        assert!(cursor.view().warning.is_none());
    }

    // ---- the fold, which cross-device sync is built on ---------------------

    #[test]
    fn the_stored_name_of_a_rating_is_the_serialised_one() {
        // The database stores ratings by name, and the JSON documents store them
        // through serde. If those two ever came apart, a row written by one build
        // would be read as something else by the next.
        for rating in [Rating::Again, Rating::Hard, Rating::Good, Rating::Easy] {
            let serialised = serde_json::to_string(&rating).unwrap();
            assert_eq!(serialised, format!("\"{}\"", rating.name()));
            assert_eq!(Rating::from_name(rating.name()), Some(rating));
        }
        assert_eq!(Rating::from_name("excellent"), None, "and not just anything");
    }

    #[test]
    fn folding_a_log_reproduces_the_card_that_recorded_it() {
        // The whole of cross-device sync rests on this: a card is not merged, it
        // is recomputed. Twenty attempts, a mix of passes and failures, so the
        // ease factor and the repetition count both move.
        let path = temp_path("fold-exact");
        let mut store = ProgressStore::open(&path).unwrap();
        let scores = [
            91.0, 42.0, 88.0, 95.0, 30.0, 77.0, 100.0, 61.0, 84.0, 55.0, 92.0, 39.0, 70.0, 99.0,
            48.0, 86.0, 93.0, 64.0, 58.0, 81.0,
        ];
        for (i, score) in scores.iter().enumerate() {
            let at = format!("2026-09-19T09:{i:02}:00Z");
            store.record_at('好', *score, &at).unwrap();
        }

        let recorded = store.document().cards.get("好").unwrap().clone();
        let log = recorded.history.clone();
        assert_eq!(log.len(), scores.len(), "the whole log is in the window");

        let folded = fold_attempts(&log, &Sm2).unwrap();
        assert_eq!(folded, recorded, "the fold rebuilds the card exactly");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn folding_is_the_same_whichever_order_the_attempts_are_gathered_in() {
        // Two devices, each with its own attempts for one character, then one
        // merged log. Which device's attempts are laid down first must not matter,
        // because the merge sorts them before folding — this is that claim, with
        // the sort done by hand.
        let first: Vec<Attempt> = (0..6)
            .map(|i| Attempt {
                at: format!("2026-09-19T09:{:02}:00Z", i * 2),
                score: 80.0 + i as f32,
                rating: Rating::from_score(80.0 + i as f32),
            })
            .collect();
        let second: Vec<Attempt> = (0..6)
            .map(|i| Attempt {
                at: format!("2026-09-19T09:{:02}:00Z", i * 2 + 1),
                score: 55.0 + i as f32,
                rating: Rating::from_score(55.0 + i as f32),
            })
            .collect();

        let mut interleaved = first.clone();
        interleaved.extend(second.clone());
        interleaved.sort_by(|a, b| a.at.cmp(&b.at));
        let folded = fold_attempts(&interleaved, &Sm2).unwrap();

        // The same attempts, offered in the other device's order first.
        let mut other = second;
        other.extend(first);
        other.sort_by(|a, b| a.at.cmp(&b.at));
        assert_eq!(fold_attempts(&other, &Sm2).unwrap(), folded);

        // And the log is now genuinely both devices': six attempts each.
        assert_eq!(folded.attempts, 12);
        assert_eq!(folded.history.len(), 12);
    }

    #[test]
    fn a_log_that_did_not_start_at_the_beginning_folds_only_what_it_holds() {
        // The migrated-card case: six attempts happened, the log kept the newest
        // three. The fold cannot invent the missing ones, so the count is what the
        // window holds — which is exactly why the shard format carries a baseline
        // for a card whose log is short of its count.
        let all: Vec<Attempt> = (0..6)
            .map(|i| Attempt {
                at: format!("2026-09-19T09:0{i}:00Z"),
                score: 70.0,
                rating: Rating::from_score(70.0),
            })
            .collect();
        let window = &all[3..];
        let folded = fold_attempts(window, &Sm2).unwrap();
        assert_eq!(folded.attempts, 3, "three attempts is all the log knows");
        assert_eq!(folded.history.len(), 3);

        assert!(fold_attempts(&[], &Sm2).is_none(), "an empty log has no card");
    }
}
