//! The app's own study files.
//!
//! Nihongo Tutor keeps exactly five things about its learner: which confusion pairs
//! they get wrong, which grammar patterns they miss, **what they have written in
//! their own sentences**, when each character the board has taught comes back, and
//! which half of the app they were last in. All five live
//! here — in files of this app's own, under this app's data directory — and
//! deliberately **not** in a shared store. The Japanese and Chinese apps are
//! separate products and their learners' data is separate with them: what is shared
//! between the apps is *code*, never user data. See `HANDOVER_NIHONGO.md`
//! invariant 15.
//!
//! All five files are plain JSON, written whole and atomically, so that a person can
//! read one, copy it as a backup, or delete it to start over. All five are written by
//! Rust rather than by the webview, which is why the app still holds no filesystem
//! permission in `capabilities/default.json`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nihongo_core::{
    now_iso8601, CompositionLog, ConfusionLog, GrammarLog, PairTally, ProgressError, ProgressStore,
    Section,
};

use crate::model::ModelSettings;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// The file's name inside the app's data directory.
pub const FILE_NAME: &str = "confusions.json";

/// The grammar drill's file name, in the same directory.
pub const GRAMMAR_FILE_NAME: &str = "grammar.json";

/// The composition exercise's file name, in the same directory.
pub const COMPOSITIONS_FILE_NAME: &str = "compositions.json";

/// The model-check settings' file name, in the same directory.
///
/// The one file here that is **not** study data: it holds an address and, where the
/// endpoint needs one, an API key. It is written readable only by its owner, and the
/// key never crosses the IPC boundary back to the webview (invariants 15 and 43).
pub const MODEL_FILE_NAME: &str = "model.json";

/// The review schedule's file name, in the same directory.
pub const REVIEW_FILE_NAME: &str = "review.json";

/// What the app remembers about the shape of the interface itself.
pub const PREFS_FILE_NAME: &str = "prefs.json";

/// The half of the app that is open, remembered so the next start returns there.
///
/// It is small enough to be a field rather than a file of its own, and it is
/// deliberately **not** a third thing the review schedule knows: which screen is
/// open has nothing to do with what is due, and a preference must never be able to
/// corrupt a schedule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Prefs {
    /// The section last open, or `None` on a first run — which the app reads as
    /// the kana on-ramp, the one thing a learner needs before anything else.
    pub section: Option<Section>,
}

impl Prefs {
    /// The section to open, with a first run answered rather than `None`.
    pub fn section_or_default(&self) -> Section {
        self.section.unwrap_or(Section::Kana)
    }
}

/// What the app remembers about how the learner left the interface.
///
/// The same file ritual as the other two stores: a missing file is the first run,
/// a file that will not parse is moved aside rather than overwritten, and a write
/// failure is returned rather than swallowed. Losing a preference is cheap, which
/// is exactly why it must not be allowed to take the schedule with it — it lives
/// in its own file for that reason and not for tidiness.
pub struct PrefsStore {
    /// `None` means nowhere to write: the preference still works for this
    /// session, which is the state the unit tests run in.
    path: Option<PathBuf>,
    prefs: Prefs,
}

impl PrefsStore {
    /// A store that keeps its answer in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            prefs: Prefs::default(),
        }
    }

    /// Load `prefs.json` from `dir`, or start on the default.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let path = dir.as_ref().join(PREFS_FILE_NAME);
        Self {
            prefs: read_json(&path),
            path: Some(path),
        }
    }

    /// What is remembered.
    pub fn prefs(&self) -> Prefs {
        self.prefs
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.path.is_some()
    }

    /// Remember the section, writing the file, and say when the write failed.
    ///
    /// The answer is kept **even if the write fails** — the learner did move — and
    /// the failure comes back rather than being swallowed, for the same reason the
    /// drill's and the schedule's do: a preference that is not being saved should
    /// not look like one that is.
    pub fn set_section(&mut self, section: Section) -> Result<(), io::Error> {
        self.prefs.section = Some(section);
        match &self.path {
            Some(path) => write_json(path, &self.prefs),
            None => Ok(()),
        }
    }
}


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
            log: read_json(&path),
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
            Some(path) => write_json(path, &self.log),
            None => Ok(()),
        }
    }
}

/// What the app knows about this learner's grammar patterns, and where it is kept.
///
/// The kana drill's [`ConfusionStore`] one course over, and a **separate file** for
/// the reason invariant 15 gives: a mistake in writing one must not be able to reach
/// the other, and a pattern id (`ni-taishite`) is not a pair of kana (`シ|ツ`) — the
/// two key spaces would be one map's keys if these shared a file, and nothing
/// reading it could tell which was which.
///
/// The tally itself is [`nihongo_core::GrammarLog`], which holds the same
/// [`PairTally`] and the same weight rule as the kana drill.
pub struct GrammarStore {
    /// `None` means nowhere to write, as in the other stores: the tallies still work
    /// for this session, and the app falls back to that when the platform will not
    /// name a data directory.
    path: Option<PathBuf>,
    log: GrammarLog,
}

impl GrammarStore {
    /// A store that keeps its record in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            log: GrammarLog::new(),
        }
    }

    /// Load `grammar.json` from `dir`, or start empty.
    ///
    /// A missing file is not an error: that is the first run, and the first answer
    /// creates it.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let path = dir.as_ref().join(GRAMMAR_FILE_NAME);
        Self {
            log: read_json(&path),
            path: Some(path),
        }
    }

    /// The record itself.
    pub fn log(&self) -> &GrammarLog {
        &self.log
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.path.is_some()
    }

    /// Record one answer against a pattern, write the file, and say what the record
    /// now is.
    ///
    /// The same rule the kana drill's store follows: the record is kept **even if
    /// the write fails** — the answer happened — but the failure is returned rather
    /// than swallowed, because a learner whose progress is not being saved should be
    /// told rather than discover it later.
    pub fn record(&mut self, id: &str, correct: bool) -> io::Result<PairTally> {
        let tally = self.log.record(id, correct);
        self.save()?;
        Ok(tally)
    }

    /// Write the file, if there is one to write.
    pub fn save(&self) -> io::Result<()> {
        match &self.path {
            Some(path) => write_json(path, &self.log),
            None => Ok(()),
        }
    }
}

/// The sentences this learner has written of their own, and where they are kept.
///
/// The fifth file (invariant 15), beside the kana drill's, the grammar drill's, the
/// schedule and the preference. It is **not** a tally and not a score: free
/// composition has no right answer, so `nihongo_core::composition` keeps what the
/// learner wrote rather than a mark for it, and this is where that lives. It is a
/// separate file for the reason the others are: the shape is different (a list of
/// sentences with the date each was written, not counters), and a mistake in writing
/// one must not be able to reach the other.
pub struct CompositionStore {
    /// `None` means nowhere to write, as in the other stores: this session keeps the
    /// sentences and the next one will not.
    path: Option<PathBuf>,
    log: CompositionLog,
}

impl CompositionStore {
    /// A store that keeps the learner's sentences in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            log: CompositionLog::new(),
        }
    }

    /// Load `compositions.json` from `dir`, or start empty.
    ///
    /// A missing file is not an error: that is the first run, and the first sentence
    /// written creates it.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let path = dir.as_ref().join(COMPOSITIONS_FILE_NAME);
        Self {
            log: read_json(&path),
            path: Some(path),
        }
    }

    /// What has been written.
    pub fn log(&self) -> &CompositionLog {
        &self.log
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.path.is_some()
    }

    /// Keep one sentence, write the file, and answer how many this pattern holds.
    ///
    /// The same rule the other stores follow: the sentence is kept **even if the
    /// write fails** — the learner wrote it — but the failure is returned rather than
    /// swallowed, because a learner whose own writing is not being saved should be
    /// told rather than discover it later.
    pub fn add(&mut self, id: &str, text: &str, at: &str) -> io::Result<usize> {
        let count = self.log.add(id, text, at);
        self.save()?;
        Ok(count)
    }

    /// Write the file, if there is one to write.
    pub fn save(&self) -> io::Result<()> {
        match &self.path {
            Some(path) => write_json(path, &self.log),
            None => Ok(()),
        }
    }
}

/// The model-check settings, and where they are kept.
///
/// The sixth file (invariant 15), and the only one that is not about the learner's
/// study: an endpoint, a model name and possibly a key. It is a file rather than a
/// session setting because a learner who has typed their address once should not have
/// to type it again — and it is its own file because a secret must not sit in the same
/// JSON as a schedule.
///
/// **The key is the one secret this app writes.** It is kept in plain text, because
/// this app has no keychain integration and a key that is not persisted is a key
/// retyped every session; the file is created readable and writable only by its owner
/// (`0600` on Unix) and never crosses back over the IPC boundary — the interface is
/// told whether a key is set, never what it is. A learner who does not want a key on
/// disk can point the endpoint at a local server, which is the other half of what this
/// feature is for.
///
/// There is no version field, deliberately, for the reason `GrammarLog` has none: the
/// shape is three strings with serde defaults, so a file from an older build — or one
/// a person edited by hand — reads as *unconfigured in part* rather than as an error,
/// and a file that will not parse at all is moved aside by the same ritual as the
/// other five.
pub struct ModelStore {
    /// `None` means nowhere to write: the settings still work for this session.
    path: Option<PathBuf>,
    settings: ModelSettings,
}

impl ModelStore {
    /// A store that keeps the settings in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            settings: ModelSettings::default(),
        }
    }

    /// Load `model.json` from `dir`, or start unconfigured.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let path = dir.as_ref().join(MODEL_FILE_NAME);
        Self {
            settings: read_json(&path),
            path: Some(path),
        }
    }

    /// What the learner configured.
    pub fn settings(&self) -> &ModelSettings {
        &self.settings
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.path.is_some()
    }

    /// Replace the settings, writing the file, and say when the write failed.
    ///
    /// The settings are kept **even if the write fails** — the learner typed them and
    /// this session can use them — and the failure comes back rather than being
    /// swallowed, for the reason the other stores give.
    pub fn save(&mut self, settings: ModelSettings) -> io::Result<()> {
        self.settings = settings;
        match &self.path {
            Some(path) => write_secret_json(path, &self.settings),
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

/// Read a JSON file, treating anything unreadable as "nothing recorded yet".
///
/// A file that will not parse is **moved aside** as `<name>.json.corrupt` rather
/// than overwritten. It is the learner's history; this app should not be the thing
/// that deletes it, and leaving it where it is would mean failing to write on every
/// subsequent answer. The next write creates a fresh file.
///
/// Generic because all three of the app's files are read this way, and the ritual
/// — keep the unreadable one, carry on with the default, never crash on a file a
/// person edited by hand — is the same ritual for a preference as for a schedule.
fn read_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<T>(&text) {
            Ok(value) => value,
            Err(err) => {
                eprintln!(
                    "{}: {err}; moving it aside and starting with no record",
                    path.display()
                );
                let _ = fs::rename(path, corrupt_path(path));
                T::default()
            }
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => T::default(),
        Err(err) => {
            eprintln!("{}: {err}; starting with no record", path.display());
            T::default()
        }
    }
}

/// Where a file that would not parse is kept.
fn corrupt_path(path: &Path) -> PathBuf {
    path.with_extension("json.corrupt")
}

/// Write a JSON file whole, via a neighbouring temporary and a rename.
///
/// A rename within a directory is atomic, so a reader either sees the previous
/// file or the new one and never a half-written file — which matters because these
/// files are rewritten after *every* answer, and the machine can lose power between
/// two of them.
fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text.as_bytes())?;
    fs::rename(&tmp, path)
}

/// Write a JSON file whole, readable and writable only by its owner.
///
/// The same temporary-plus-rename as [`write_json`], with the mode set on the
/// temporary before it is renamed, so the secret is never briefly world-readable. On
/// Unix the mode is `0600`; on the platforms where that means nothing the file is
/// simply written, which is what the platform's own directory permissions are for.
fn write_secret_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
    }
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

    // ---- the learner's own sentences ---------------------------------------

    #[test]
    fn a_first_run_has_written_nothing_and_the_first_sentence_makes_the_file() {
        let dir = TempDir::new("composition-first-run");
        let mut store = CompositionStore::at_dir(dir.path());
        assert!(store.is_persistent());
        assert!(store.log().is_empty());
        assert!(!dir.path().join(COMPOSITIONS_FILE_NAME).exists(), "nothing written yet");

        let count = store.add("ni-hanshite", "彼の予想に反して、試合は延期された。", "2026-10-07T00:00:00Z")
            .expect("saves");
        assert_eq!(count, 1);
        assert!(dir.path().join(COMPOSITIONS_FILE_NAME).exists(), "the sentence was written");
    }

    #[test]
    fn the_sentences_survive_a_restart_in_the_order_they_were_written() {
        let dir = TempDir::new("composition-restart");
        {
            let mut store = CompositionStore::at_dir(dir.path());
            store.add("wa", "私は学生です。", "2026-10-07T00:00:00Z").expect("saves");
            store.add("wa", "今日は天気がいいです。", "2026-10-07T00:01:00Z").expect("saves");
            store.add("ga", "雨が降っています。", "2026-10-07T00:02:00Z").expect("saves");
        }
        let reopened = CompositionStore::at_dir(dir.path());
        assert_eq!(reopened.log().total(), 3);
        assert_eq!(reopened.log().patterns(), 2);
        assert_eq!(
            reopened.log().sentences("wa"),
            vec!["私は学生です。", "今日は天気がいいです。"]
        );
        assert_eq!(reopened.log().count("ga"), 1);
        assert_eq!(reopened.log().count("nope"), 0);
    }

    #[test]
    fn the_composition_file_is_readable_json_of_the_shape_the_docs_promise() {
        let dir = TempDir::new("composition-shape");
        let mut store = CompositionStore::at_dir(dir.path());
        store.add("wa", "私は学生です。", "2026-10-07T00:00:00Z").expect("saves");

        let text = fs::read_to_string(dir.path().join(COMPOSITIONS_FILE_NAME)).expect("the file");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        // A person opening it sees `{"version":1,"written":{"wa":[{"text":…,"at":…}]}}`:
        // their own sentences under the pattern each was written for.
        assert_eq!(value["version"], 1);
        assert_eq!(value["written"]["wa"][0]["text"], "私は学生です。");
        assert_eq!(value["written"]["wa"][0]["at"], "2026-10-07T00:00:00Z");
    }

    #[test]
    fn a_file_of_written_sentences_that_will_not_parse_is_kept_aside() {
        let dir = TempDir::new("composition-corrupt");
        let path = dir.path().join(COMPOSITIONS_FILE_NAME);
        fs::write(&path, "{ this is not json").expect("writes");

        let mut store = CompositionStore::at_dir(dir.path());
        assert!(store.log().is_empty(), "a corrupt file is not a crash");
        assert_eq!(
            fs::read_to_string(dir.path().join("compositions.json.corrupt")).expect("still there"),
            "{ this is not json",
            "kept as it was, not as an empty file"
        );

        // And the app carries on: the next sentence writes a valid file.
        store.add("wa", "私は学生です。", "2026-10-07T00:00:00Z").expect("saves");
        let reread = CompositionStore::at_dir(dir.path());
        assert_eq!(reread.log().sentences("wa"), vec!["私は学生です。"]);
    }

    #[test]
    fn a_write_failure_is_reported_and_the_sentence_is_still_kept_in_memory() {
        let dir = TempDir::new("composition-unwritable");
        fs::create_dir_all(dir.path().join(COMPOSITIONS_FILE_NAME))
            .expect("a directory in the file's place");
        let mut store = CompositionStore::at_dir(dir.path());
        assert!(
            store.add("wa", "私は学生です。", "2026-10-07T00:00:00Z").is_err(),
            "the failure surfaces"
        );
        assert_eq!(
            store.log().sentences("wa"),
            vec!["私は学生です。"],
            "and the learner's sentence is not lost with it"
        );
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

    // ---- the model check's settings ----------------------------------------

    #[test]
    fn a_first_run_has_no_model_configured_and_saving_one_makes_the_file() {
        let dir = TempDir::new("model-first-run");
        let mut store = ModelStore::at_dir(dir.path());
        assert!(store.is_persistent());
        assert!(!store.settings().configured(), "nothing is configured before anything is typed");
        assert!(!dir.path().join(MODEL_FILE_NAME).exists(), "nothing written yet");

        store
            .save(ModelSettings {
                endpoint: "http://127.0.0.1:11434/v1".to_string(),
                model: "llama3.1".to_string(),
                key: None,
            })
            .expect("saves");
        assert!(dir.path().join(MODEL_FILE_NAME).exists());
        assert!(store.settings().configured());
    }

    #[test]
    fn the_model_settings_survive_a_restart_and_the_key_is_one_of_them() {
        let dir = TempDir::new("model-restart");
        {
            let mut store = ModelStore::at_dir(dir.path());
            store
                .save(ModelSettings {
                    endpoint: "https://api.openai.com/v1".to_string(),
                    model: "gpt-4o-mini".to_string(),
                    key: Some("sk-secret".to_string()),
                })
                .expect("saves");
        }
        let reopened = ModelStore::at_dir(dir.path());
        assert_eq!(reopened.settings().endpoint, "https://api.openai.com/v1");
        assert_eq!(reopened.settings().model, "gpt-4o-mini");
        assert_eq!(reopened.settings().key.as_deref(), Some("sk-secret"));
        assert!(reopened.settings().validate().is_ok());
    }

    #[test]
    fn the_key_file_is_readable_by_its_owner_only_and_is_the_promised_shape() {
        let dir = TempDir::new("model-shape");
        let mut store = ModelStore::at_dir(dir.path());
        store
            .save(ModelSettings {
                endpoint: "http://127.0.0.1:1234/v1".to_string(),
                model: "local-model".to_string(),
                key: Some("sk-secret".to_string()),
            })
            .expect("saves");

        let path = dir.path().join(MODEL_FILE_NAME);
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("the file")).expect("valid JSON");
        assert_eq!(value["endpoint"], "http://127.0.0.1:1234/v1");
        assert_eq!(value["model"], "local-model");
        assert_eq!(value["key"], "sk-secret");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).expect("the file").permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "a secret is not world-readable: {mode:o}");
        }
    }

    #[test]
    fn a_model_file_that_will_not_parse_is_kept_aside_and_leaves_the_app_unconfigured() {
        let dir = TempDir::new("model-corrupt");
        let path = dir.path().join(MODEL_FILE_NAME);
        fs::write(&path, "{ this is not json").expect("writes");

        let mut store = ModelStore::at_dir(dir.path());
        assert!(!store.settings().configured(), "a corrupt file is not a crash");
        assert_eq!(
            fs::read_to_string(dir.path().join("model.json.corrupt")).expect("still there"),
            "{ this is not json"
        );

        // And the app carries on: saving writes a valid file.
        store
            .save(ModelSettings {
                endpoint: "http://127.0.0.1:1234/v1".to_string(),
                model: "local-model".to_string(),
                key: None,
            })
            .expect("saves");
        assert!(ModelStore::at_dir(dir.path()).settings().configured());
    }

    #[test]
    fn a_model_file_from_an_older_build_reads_as_far_as_it_can() {
        // The shape is three strings with defaults, so a file missing the key — or
        // missing everything but the endpoint — is partly configured rather than
        // corrupt.
        let dir = TempDir::new("model-partial");
        fs::write(
            dir.path().join(MODEL_FILE_NAME),
            r#"{"endpoint": "http://127.0.0.1:1234/v1"}"#,
        )
        .expect("writes");
        let store = ModelStore::at_dir(dir.path());
        assert_eq!(store.settings().endpoint, "http://127.0.0.1:1234/v1");
        assert_eq!(store.settings().model, "");
        assert_eq!(store.settings().key, None);
        assert!(!store.settings().configured(), "a model name is still missing");
    }

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

    // ---- the remembered section -------------------------------------------

    #[test]
    fn a_first_run_opens_on_the_kana_on_ramp_and_writes_nothing() {
        let dir = TempDir::new("prefs-first-run");
        let store = PrefsStore::at_dir(dir.path());
        assert!(store.is_persistent());
        assert_eq!(store.prefs().section, None, "nothing remembered yet");
        assert_eq!(
            store.prefs().section_or_default(),
            Section::Kana,
            "a learner who has never been here needs the kana first"
        );
        assert!(
            !dir.path().join(PREFS_FILE_NAME).exists(),
            "opening the app is not a preference to write"
        );
    }

    // ---- the grammar drill's record ---------------------------------------

    #[test]
    fn a_grammar_answer_survives_a_restart_in_its_own_file() {
        let dir = TempDir::new("grammar-restart");
        {
            let mut store = GrammarStore::at_dir(dir.path());
            assert!(store.is_persistent());
            store.record("te-kudasai", false).expect("writes");
            store.record("te-kudasai", true).expect("writes");
        }
        let reopened = GrammarStore::at_dir(dir.path());
        assert_eq!(
            reopened.log().tally("te-kudasai"),
            PairTally { asked: 2, correct: 1, wrong: 1 }
        );
        assert_eq!(reopened.log().patterns_seen(), 1);

        // Its own file, not the kana drill's: the two key spaces must not meet.
        let text = fs::read_to_string(dir.path().join(GRAMMAR_FILE_NAME)).expect("the file");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(value["points"]["te-kudasai"]["wrong"], 1);
        assert!(value.get("pairs").is_none(), "the kana drill's key is not this file's");
        assert!(
            !dir.path().join(FILE_NAME).exists(),
            "recording a grammar answer wrote nothing into confusions.json"
        );
    }

    #[test]
    fn a_grammar_file_that_will_not_parse_is_kept_aside_and_does_not_block_the_app() {
        let dir = TempDir::new("grammar-corrupt");
        let path = dir.path().join(GRAMMAR_FILE_NAME);
        fs::write(&path, "{ this is not json").expect("writes");

        let mut store = GrammarStore::at_dir(dir.path());
        assert!(store.log().is_empty(), "a corrupt file is not a crash");
        assert_eq!(
            fs::read_to_string(dir.path().join("grammar.json.corrupt")).expect("still there"),
            "{ this is not json",
            "kept as it was, not as an empty file"
        );

        // And the app carries on: the next answer writes a valid file.
        store.record("te-kudasai", true).expect("writes");
        assert_eq!(GrammarStore::at_dir(dir.path()).log().patterns_seen(), 1);
    }

    #[test]
    fn a_grammar_write_that_fails_is_reported_and_the_answer_is_still_counted() {
        let dir = TempDir::new("grammar-unwritable");
        fs::create_dir_all(dir.path().join("grammar.json.tmp")).expect("a directory in the way");
        let mut store = GrammarStore::at_dir(dir.path());

        assert!(store.record("te-kudasai", false).is_err(), "the failure surfaces");
        assert_eq!(
            store.log().tally("te-kudasai"),
            PairTally { asked: 1, correct: 0, wrong: 1 },
            "the answer happened, and the record holds it until it can be written"
        );
    }

    #[test]
    fn an_in_memory_grammar_store_records_without_writing_anything() {
        let mut store = GrammarStore::in_memory();
        assert!(!store.is_persistent());
        store.record("te-kudasai", true).expect("nothing to write");
        assert_eq!(store.log().tally("te-kudasai").correct, 1);
    }

    #[test]
    fn the_section_survives_a_restart_and_is_readable_json() {
        let dir = TempDir::new("prefs-restart");
        {
            let mut store = PrefsStore::at_dir(dir.path());
            store.set_section(Section::Kanji).expect("writes");
        }
        let reopened = PrefsStore::at_dir(dir.path());
        assert_eq!(reopened.prefs().section, Some(Section::Kanji));
        assert_eq!(reopened.prefs().section_or_default(), Section::Kanji);

        // The file is the shape the handover documents, so a person can see and
        // edit which half the app opens on.
        let text = fs::read_to_string(dir.path().join(PREFS_FILE_NAME)).expect("the file");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(value["section"], "kanji");
    }

    #[test]
    fn a_preference_file_that_will_not_parse_is_kept_aside_and_does_not_block_the_app() {
        let dir = TempDir::new("prefs-corrupt");
        let path = dir.path().join(PREFS_FILE_NAME);
        fs::write(&path, "{ this is not json").expect("writes");

        let mut store = PrefsStore::at_dir(dir.path());
        assert_eq!(store.prefs().section, None, "a corrupt file is not a crash");
        assert_eq!(store.prefs().section_or_default(), Section::Kana);
        assert_eq!(
            fs::read_to_string(dir.path().join("prefs.json.corrupt")).expect("still there"),
            "{ this is not json",
            "kept as it was, not as an empty file"
        );

        // And the app carries on: the next move writes a valid file.
        store.set_section(Section::Kanji).expect("writes");
        assert_eq!(
            PrefsStore::at_dir(dir.path()).prefs().section,
            Some(Section::Kanji)
        );
    }

    #[test]
    fn a_preference_that_cannot_be_written_says_so_and_is_kept_in_memory() {
        let dir = TempDir::new("prefs-unwritable");
        fs::create_dir_all(dir.path().join("prefs.json.tmp")).expect("a directory in the way");
        let mut store = PrefsStore::at_dir(dir.path());

        assert!(store.set_section(Section::Kanji).is_err(), "the failure surfaces");
        assert_eq!(
            store.prefs().section,
            Some(Section::Kanji),
            "the learner did move, so this session still knows where they are"
        );
    }

    #[test]
    fn an_in_memory_preference_store_remembers_without_writing_anything() {
        let mut store = PrefsStore::in_memory();
        assert!(!store.is_persistent());
        store.set_section(Section::Kanji).expect("nothing to write");
        assert_eq!(store.prefs().section, Some(Section::Kanji));
    }
}
