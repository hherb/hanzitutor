//! The app's own study files.
//!
//! Kana Tutor keeps exactly two things about its learner: which confusion pairs
//! they get wrong, and when each character the board has taught comes back. Both
//! live here — in files of this app's own, under this app's data directory — and
//! deliberately **not** in a shared store. The Japanese and Chinese apps are
//! separate products and their learners' data is separate with them: what is
//! shared between the apps is *code*, never user data. See
//! `HANDOVER_NIHONGO.md` invariant 15.
//!
//! Both files are plain JSON, written whole and atomically, so that a person can
//! read one, copy it as a backup, or delete it to start over. Both are written by
//! Rust rather than by the webview, which is why the app still holds no
//! filesystem permission in `capabilities/default.json`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nihongo_core::{now_iso8601, ConfusionLog, PairTally, ProgressError, ProgressStore};

/// The file's name inside the app's data directory.
pub const FILE_NAME: &str = "confusions.json";

/// The review schedule's file name, in the same directory.
pub const REVIEW_FILE_NAME: &str = "review.json";

/// What the app knows about this learner's confusion pairs, and where it is kept.
pub struct ConfusionStore {
    /// `None` means nowhere to write — the tallies still work, for this session
    /// only. That is the state the unit tests run in, and what the app falls back
    /// to when the platform will not name a data directory, because an app that
    /// cannot remember is still better than an app that will not start.
    path: Option<PathBuf>,
    log: ConfusionLog,
}

impl ConfusionStore {
    /// A store that keeps its record in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            log: ConfusionLog::new(),
        }
    }

    /// Load `confusions.json` from `dir`, or start empty.
    ///
    /// A missing file is not an error: that is the first run, and the first
    /// answer creates it.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let path = dir.as_ref().join(FILE_NAME);
        Self {
            log: read(&path),
            path: Some(path),
        }
    }

    /// The record itself.
    pub fn log(&self) -> &ConfusionLog {
        &self.log
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.path.is_some()
    }

    /// Record one answer against a pair, write the file, and say what the record
    /// now is.
    ///
    /// The record is kept **even if the write fails** — the answer happened — but
    /// the failure is returned rather than swallowed, because a learner whose
    /// progress is not being saved should be told rather than discover it later.
    pub fn record(&mut self, key: &str, correct: bool) -> io::Result<PairTally> {
        let tally = self.log.record(key, correct);
        self.save()?;
        Ok(tally)
    }

    /// Write the file, if there is one to write.
    pub fn save(&self) -> io::Result<()> {
        match &self.path {
            Some(path) => write(path, &self.log),
            None => Ok(()),
        }
    }
}

/// What one attempt did to the review schedule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptOutcome {
    /// True when the attempt advanced the schedule, because the character was
    /// new or due. A later attempt inside the interval is practice, not a review.
    pub counted: bool,
    /// When the character comes up next, ISO-8601 UTC.
    pub due: String,
    /// Set when the attempt was counted but the schedule could not be written.
    /// The card is held in memory either way; this is how the learner is told.
    pub warning: Option<String>,
}

/// The review schedule: one card per character the board has taught, kept in a
/// file of this app's own.
///
/// The scheduling itself is **not** here. `hanzi_core::progress`'s SM-2, its
/// intervals and its due-date arithmetic are reused unchanged, exactly as the
/// geometry engine is, and the schedule it produces is what this type holds. What
/// this type adds is the app's half: *where* the schedule lives, that a file which
/// will not parse is kept rather than overwritten, and the rule that makes a
/// review a review.
///
/// ## A review is an attempt on a character that is new or due
///
/// Recording every grade would be wrong in a way that is easy to miss: a learner
/// who writes あ five times in a row would advance SM-2's interval five times and
/// not see あ again for a year, on the strength of one sitting. So an attempt
/// advances the schedule only when the card is new or its due date has passed.
/// Writing it again before then is practice, it still gets graded, and the
/// schedule is left exactly as it was.
pub struct ReviewStore {
    store: ProgressStore,
    /// Something the learner should be told: the schedule could not be read and
    /// was kept aside, or there is nowhere to write and this session will not be
    /// remembered. Never a reason not to start.
    warning: Option<String>,
}

impl ReviewStore {
    /// A schedule that keeps its cards in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            store: ProgressStore::in_memory(),
            warning: None,
        }
    }

    /// Load `review.json` from `dir`, or start a fresh schedule.
    ///
    /// A missing file is the first run. A file that will not parse is moved
    /// aside — it is the learner's history, and this app should not be the thing
    /// that deletes it — and a file written by a *newer* build is left untouched
    /// and this session runs in memory, because overwriting a schedule this build
    /// does not understand is the one way to lose one for good.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let path = dir.as_ref().join(REVIEW_FILE_NAME);
        match ProgressStore::open(&path) {
            Ok(store) => Self {
                store,
                warning: None,
            },
            Err(ProgressError::Malformed(why)) => {
                let aside = corrupt_path(&path);
                eprintln!(
                    "{}: {why}; moving it aside and starting a fresh schedule",
                    path.display()
                );
                match fs::rename(&path, &aside) {
                    // The unreadable file is out of the way, so a fresh schedule
                    // can take its place and be written to from here on.
                    Ok(()) => match ProgressStore::open(&path) {
                        Ok(store) => Self {
                            store,
                            warning: Some(format!(
                                "the review schedule could not be read ({why}); it was kept as {} \
                                 and a fresh one started",
                                aside.display()
                            )),
                        },
                        Err(err) => Self::unusable(format!(
                            "the review schedule could not be read ({why}) and, once kept as {}, a \
                             fresh one could not be started ({err})",
                            aside.display()
                        )),
                    },
                    Err(err) => Self::unusable(format!(
                        "the review schedule could not be read ({why}) and could not be kept aside \
                         ({err})"
                    )),
                }
            }
            // An unreadable file, or one from the future: do not write over it.
            Err(err) => Self::unusable(format!("the review schedule could not be opened ({err})")),
        }
    }

    /// A schedule that will not be written to, with the reason.
    fn unusable(why: String) -> Self {
        eprintln!("{why}; this session will not be remembered");
        Self {
            store: ProgressStore::in_memory(),
            warning: Some(format!("{why}; this session will not be remembered")),
        }
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        !self.store.path().as_os_str().is_empty()
    }

    /// The schedule itself, for a caller that wants to show or assert on it.
    pub fn store(&self) -> &ProgressStore {
        &self.store
    }

    /// Why this session may not be remembered, when there is a reason.
    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    /// Offer a graded attempt to the schedule and say what it did.
    ///
    /// The card is kept **even if the write fails** — the attempt happened — and
    /// the failure comes back on the outcome rather than being swallowed, because
    /// a learner whose progress is not being saved should be told rather than
    /// discover it later.
    pub fn record_attempt(&mut self, ch: char, score: f32) -> Result<AttemptOutcome, String> {
        let now = now_iso8601();
        if let Some(card) = self.store.card(ch) {
            if !card.is_due(&now) {
                return Ok(AttemptOutcome {
                    counted: false,
                    due: card.due.clone(),
                    warning: None,
                });
            }
        }
        let card = self.store.record(ch, score).map_err(|err| err.to_string())?;
        let warning = self.store.save().err().map(|err| {
            format!("the attempt was counted but could not be saved: {err}")
        });
        Ok(AttemptOutcome {
            counted: true,
            due: card.due,
            warning,
        })
    }
}

/// Read the log, treating anything unreadable as "nothing recorded yet".
///
/// A file that will not parse is **moved aside** as `confusions.json.corrupt`
/// rather than overwritten. It is the learner's history; this app should not be
/// the thing that deletes it, and leaving it where it is would mean failing to
/// write on every subsequent answer. The next answer writes a fresh file.
fn read(path: &Path) -> ConfusionLog {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<ConfusionLog>(&text) {
            Ok(log) => log,
            Err(err) => {
                eprintln!(
                    "{}: {err}; moving it aside and starting with no record",
                    path.display()
                );
                let _ = fs::rename(path, corrupt_path(path));
                ConfusionLog::new()
            }
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => ConfusionLog::new(),
        Err(err) => {
            eprintln!("{}: {err}; starting with no record", path.display());
            ConfusionLog::new()
        }
    }
}

/// Where a file that would not parse is kept.
fn corrupt_path(path: &Path) -> PathBuf {
    path.with_extension("json.corrupt")
}

/// Write the log whole, via a neighbouring temporary and a rename.
///
/// A rename within a directory is atomic, so a reader either sees the previous
/// file or the new one and never a half-written file — which matters because the
/// file is rewritten after *every* answer, and the machine can lose power between
/// two of them.
fn write(path: &Path, log: &ConfusionLog) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(log).map_err(io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text.as_bytes())?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nihongo_core::{find_pair, pair_key};

    /// A directory of our own under the system temporary directory, removed when
    /// the test ends.
    ///
    /// Named with the process id and a counter so that two tests — or two runs —
    /// never share one, which is the failure mode that makes a test that passes
    /// alone fail in a suite.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let mut path = std::env::temp_dir();
            path.push(format!(
                "nihongo-tutor-store-{}-{tag}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("a temporary directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn shi_tsu() -> String {
        pair_key(find_pair("シ|ツ").expect("the pair exists"))
    }

    #[test]
    fn a_first_run_starts_empty_and_the_first_answer_creates_the_file() {
        let dir = TempDir::new("first-run");
        let mut store = ConfusionStore::at_dir(dir.path());
        assert!(store.is_persistent());
        assert!(store.log().is_empty());
        assert!(!dir.path().join(FILE_NAME).exists(), "nothing written yet");

        let tally = store.record(&shi_tsu(), false).expect("saves");
        assert_eq!(tally.asked, 1);
        assert_eq!(tally.wrong, 1);
        assert!(dir.path().join(FILE_NAME).exists(), "the answer was written");
    }

    #[test]
    fn the_record_survives_a_restart() {
        let dir = TempDir::new("restart");
        {
            let mut store = ConfusionStore::at_dir(dir.path());
            store.record(&shi_tsu(), false).expect("saves");
            store.record(&shi_tsu(), true).expect("saves");
        }
        let reopened = ConfusionStore::at_dir(dir.path());
        let tally = reopened.log().tally(&shi_tsu());
        assert_eq!(tally.asked, 2);
        assert_eq!(tally.wrong, 1);
        assert_eq!(tally.correct, 1);
        assert_eq!(
            reopened.log().weight(&shi_tsu()),
            2.0,
            "1 + 2 for the miss − 1 for the hit"
        );
    }

    #[test]
    fn the_file_is_readable_json_of_the_shape_the_docs_promise() {
        let dir = TempDir::new("shape");
        let mut store = ConfusionStore::at_dir(dir.path());
        store.record(&shi_tsu(), false).expect("saves");

        let text = fs::read_to_string(dir.path().join(FILE_NAME)).expect("the file");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        // The whole file is the log, and a tally is three camelCase counts, so a
        // person opening it sees `{"pairs":{"シ|ツ":{"asked":1,...}}}`.
        let tally = &value["pairs"]["シ|ツ"];
        assert_eq!(tally["asked"], 1);
        assert_eq!(tally["correct"], 0);
        assert_eq!(tally["wrong"], 1);
    }

    #[test]
    fn an_empty_object_is_a_valid_file_and_not_a_corrupt_one() {
        // A file written by an older build, or emptied by hand, must read as
        // "nothing recorded" rather than be moved aside as corrupt.
        let dir = TempDir::new("empty-object");
        fs::write(dir.path().join(FILE_NAME), "{}").expect("writes");
        let store = ConfusionStore::at_dir(dir.path());
        assert!(store.log().is_empty());
        assert!(!dir.path().join("confusions.json.corrupt").exists());
    }

    #[test]
    fn a_file_that_will_not_parse_is_kept_aside_and_not_overwritten() {
        let dir = TempDir::new("corrupt");
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, "{ this is not json").expect("writes");

        let mut store = ConfusionStore::at_dir(dir.path());
        assert!(store.log().is_empty(), "a corrupt file is not a crash");
        let aside = dir.path().join("confusions.json.corrupt");
        assert!(aside.exists(), "the unreadable file was kept");
        assert_eq!(
            fs::read_to_string(&aside).expect("still there"),
            "{ this is not json",
            "and kept as it was, not as an empty file"
        );

        // And the app carries on: the next answer writes a valid file.
        store.record(&shi_tsu(), false).expect("saves");
        let reread = ConfusionStore::at_dir(dir.path());
        assert_eq!(reread.log().tally(&shi_tsu()).wrong, 1);
    }

    #[test]
    fn an_in_memory_store_records_without_writing_anything() {
        let mut store = ConfusionStore::in_memory();
        assert!(!store.is_persistent());
        store.record(&shi_tsu(), false).expect("nothing to write");
        assert_eq!(store.log().tally(&shi_tsu()).wrong, 1);
    }

    #[test]
    fn a_write_failure_is_reported_rather_than_hidden() {
        // A directory where the file should be is the cheapest way to make the
        // write fail on every platform, and it must come back as an error rather
        // than a panic — or worse, as nothing at all.
        let dir = TempDir::new("unwritable");
        fs::create_dir_all(dir.path().join(FILE_NAME)).expect("a directory in the file's place");
        let mut store = ConfusionStore::at_dir(dir.path());
        assert!(store.record(&shi_tsu(), false).is_err(), "the failure surfaces");
        assert_eq!(
            store.log().tally(&shi_tsu()).wrong,
            1,
            "and the answer is still counted in memory"
        );
    }

    // ---- the review schedule ----------------------------------------------

    #[test]
    fn a_first_run_has_an_empty_schedule_and_the_first_attempt_makes_the_file() {
        let dir = TempDir::new("review-first-run");
        let mut store = ReviewStore::at_dir(dir.path());
        assert!(store.is_persistent());
        assert!(store.store().cards().is_empty());
        assert!(store.warning().is_none(), "a first run is not a problem");
        assert!(!dir.path().join(REVIEW_FILE_NAME).exists(), "nothing written yet");

        let outcome = store.record_attempt('あ', 20.0).expect("saves");
        assert!(outcome.counted, "a new character is a review");
        assert!(
            outcome.due.as_str() > now_iso8601().as_str(),
            "a failed attempt comes back later, not never: {}",
            outcome.due
        );
        assert!(dir.path().join(REVIEW_FILE_NAME).exists(), "the attempt was written");
    }

    #[test]
    fn an_attempt_inside_the_interval_is_practice_and_not_a_review() {
        // The rule that stops five grades in one sitting becoming a year's
        // interval: once the card is scheduled, writing it again does not move it.
        let dir = TempDir::new("review-not-due");
        let mut store = ReviewStore::at_dir(dir.path());
        let first = store.record_attempt('あ', 90.0).expect("saves");
        assert!(first.counted);

        let again = store.record_attempt('あ', 90.0).expect("nothing to write");
        assert!(!again.counted, "the card was not due");
        assert_eq!(again.due, first.due, "and the schedule did not move");
        assert_eq!(
            store.store().card('あ').expect("a card").attempts,
            1,
            "the second attempt is not even an attempt as far as the schedule is concerned"
        );
    }

    #[test]
    fn the_schedule_survives_a_restart() {
        let dir = TempDir::new("review-restart");
        let due = {
            let mut store = ReviewStore::at_dir(dir.path());
            store.record_attempt('あ', 90.0).expect("saves");
            let due = store.store().card('あ').expect("a card").due.clone();
            assert_eq!(store.store().cards().len(), 1);
            due
        };
        let reopened = ReviewStore::at_dir(dir.path());
        let card = reopened.store().card('あ').expect("the card is still there");
        assert_eq!(card.due, due);
        assert_eq!(card.attempts, 1);
    }

    #[test]
    fn the_review_file_is_readable_json_of_the_shape_the_docs_promise() {
        let dir = TempDir::new("review-shape");
        let mut store = ReviewStore::at_dir(dir.path());
        store.record_attempt('あ', 90.0).expect("saves");

        let text = fs::read_to_string(dir.path().join(REVIEW_FILE_NAME)).expect("the file");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(value["version"], 1);
        assert_eq!(value["cards"]["あ"]["attempts"], 1);
        assert_eq!(value["cards"]["あ"]["repetitions"], 1);
        assert!(value["cards"]["あ"]["due"].is_string());
    }

    #[test]
    fn a_schedule_that_will_not_parse_is_kept_aside_and_not_overwritten() {
        let dir = TempDir::new("review-corrupt");
        let path = dir.path().join(REVIEW_FILE_NAME);
        fs::write(&path, "{ this is not json").expect("writes");

        let mut store = ReviewStore::at_dir(dir.path());
        assert!(store.store().cards().is_empty(), "a corrupt file is not a crash");
        assert!(
            store.warning().is_some_and(|why| why.contains("kept as")),
            "and the learner is told what happened: {:?}",
            store.warning()
        );
        let aside = dir.path().join("review.json.corrupt");
        assert_eq!(
            fs::read_to_string(&aside).expect("still there"),
            "{ this is not json",
            "kept as it was, not as an empty file"
        );

        // And the app carries on: the next attempt writes a valid schedule.
        store.record_attempt('あ', 20.0).expect("saves");
        let reread = ReviewStore::at_dir(dir.path());
        assert_eq!(reread.store().card('あ').expect("a card").attempts, 1);
    }

    #[test]
    fn a_schedule_from_a_newer_build_is_left_alone_and_the_session_runs_in_memory() {
        let dir = TempDir::new("review-future");
        let path = dir.path().join(REVIEW_FILE_NAME);
        let future = r#"{"version":99,"cards":{}}"#;
        fs::write(&path, future).expect("writes");

        let mut store = ReviewStore::at_dir(dir.path());
        assert!(!store.is_persistent(), "nothing may be written over it");
        assert!(
            store.warning().is_some_and(|why| why.contains("not be remembered")),
            "{:?}",
            store.warning()
        );
        store.record_attempt('あ', 20.0).expect("nothing to write");
        assert_eq!(
            fs::read_to_string(&path).expect("still there"),
            future,
            "the file this build does not understand was not touched"
        );
    }

    #[test]
    fn an_in_memory_schedule_records_without_writing_anything() {
        let mut store = ReviewStore::in_memory();
        assert!(!store.is_persistent());
        store.record_attempt('あ', 20.0).expect("nothing to write");
        assert_eq!(store.store().card('あ').expect("a card").attempts, 1);
    }

    #[test]
    fn a_failed_write_is_reported_and_the_attempt_is_still_counted() {
        // The temporary file the atomic write needs is a directory, so the write
        // fails on every platform. The failure must surface — and the attempt must
        // survive it in memory rather than being dropped.
        let dir = TempDir::new("review-unwritable");
        fs::create_dir_all(dir.path().join("review.json.tmp")).expect("a directory in the way");
        let mut store = ReviewStore::at_dir(dir.path());

        let outcome = store.record_attempt('あ', 20.0).expect("the attempt itself is recorded");
        assert!(outcome.counted);
        let warning = outcome.warning.expect("the failed write is reported");
        assert!(warning.contains("could not be saved"), "{warning}");
        assert_eq!(
            store.store().card('あ').expect("a card").attempts,
            1,
            "the attempt happened, and the schedule holds it until it can be written"
        );
    }
}
