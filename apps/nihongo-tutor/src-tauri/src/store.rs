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
//!
//! ## When a file is not what it should be
//!
//! One rule for every file here, because every one of them is someone's history:
//! **this app is never the thing that destroys it.** A file that will not parse — a
//! hand edit gone wrong, or a crash that cut it off in the middle of a Japanese
//! character — is moved aside under a name of its own and a fresh one started. A
//! file that cannot be read at all, a file from a newer build, or one that could not
//! be moved aside is left exactly where it is, and that store runs in memory for the
//! session. Either way the learner is told, on the next response that store gives,
//! rather than finding out from an empty record a week later.

use std::fs;
use std::io::{self, Write};
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

/// What a store did with one answer, and what the learner should be told about it.
///
/// The answer is recorded in memory whatever happens to the file, so `value` is
/// always the record as it now stands. `warning` is set when the file could not be
/// written, or when the store has something to say about how it was opened: a
/// verdict travels with its warning rather than being replaced by an error, because
/// an error is what made a screen offer the same answer again, and every retry was
/// counted a second time.
#[derive(Clone, Debug, PartialEq)]
pub struct Saved<T> {
    pub value: T,
    pub warning: Option<String>,
}

/// Where a store's file is, and what the learner has not yet been told about it.
///
/// Shared by the five stores below that keep their own JSON, so the rule in the
/// module's header is written once and cannot drift between a preference and a
/// tally.
struct StoreFile {
    /// `None` means nowhere to write: the record still works for this session. That
    /// is the state the unit tests run in, what the app falls back to when the
    /// platform will not name a data directory, and what a store is put in when its
    /// file must not be written over.
    path: Option<PathBuf>,
    /// Something the learner should hear about this file. Said **once** when the
    /// file was moved aside and a fresh one is being written — from then on the
    /// store is working normally — and said on **every** response while the store
    /// runs in memory, because the learner is losing work for as long as that lasts.
    warning: Option<String>,
}

impl StoreFile {
    fn in_memory() -> Self {
        Self {
            path: None,
            warning: None,
        }
    }

    /// Read `path`, by the rule in the module's header.
    ///
    /// `what` names the file for the learner ("the kana drill's record").
    /// `understood` is the newest `version` this build reads, for a file that has
    /// one: a file claiming a later one is refused and left alone, and that is
    /// decided **before** the file is parsed as this build's shape — a newer build
    /// that restructured the file has not written a damaged one, and moving it aside
    /// would put it out of reach of the build that can read it.
    fn open<T: DeserializeOwned + Default>(
        path: PathBuf,
        what: &str,
        understood: Option<u32>,
    ) -> (T, Self) {
        // Bytes rather than a string, so that a file cut off in the middle of a
        // multi-byte character is a *parse* error — moved aside like any other —
        // rather than an I/O error. Only a file that truly cannot be read is left
        // for the I/O branch below.
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return (T::default(), Self::at(path))
            }
            // Permission denied, a disk error, a directory in the file's place: the
            // file may be perfectly good, so it is not moved and not written over.
            Err(err) => return Self::unwritable(format!("{what} could not be opened ({err})")),
        };
        if let Some(understood) = understood {
            // A probe that does not parse says nothing either way; the full parse
            // below decides whether the file is damaged.
            if let Ok(VersionProbe {
                version: Some(version),
            }) = serde_json::from_slice::<VersionProbe>(&bytes)
            {
                if version > understood {
                    return Self::unwritable(format!(
                        "{what} was saved by a newer version of the app (format {version}, this \
                         one reads up to {understood}), so the file was left as it is"
                    ));
                }
            }
        }
        match serde_json::from_slice::<T>(&bytes) {
            Ok(value) => (value, Self::at(path)),
            Err(why) => match set_aside(&path) {
                Ok(aside) => {
                    let warning = format!(
                        "{what} could not be read ({why}); it was kept as {} and a fresh one \
                         started",
                        aside.display()
                    );
                    eprintln!("{}: {warning}", path.display());
                    (
                        T::default(),
                        Self {
                            path: Some(path),
                            warning: Some(warning),
                        },
                    )
                }
                // If the bad file cannot be moved out of the way, writing a fresh
                // one would replace it — which is the one thing this must not do.
                Err(err) => Self::unwritable(format!(
                    "{what} could not be read ({why}) and could not be kept aside ({err})"
                )),
            },
        }
    }

    fn at(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            warning: None,
        }
    }

    /// A store that will not write, with the reason.
    fn unwritable<T: Default>(why: String) -> (T, Self) {
        let warning = format!("{why}; this session will not be remembered");
        eprintln!("{warning}");
        (
            T::default(),
            Self {
                path: None,
                warning: Some(warning),
            },
        )
    }

    fn is_persistent(&self) -> bool {
        self.path.is_some()
    }

    fn write<T: Serialize>(&self, value: &T) -> io::Result<()> {
        match &self.path {
            Some(path) => write_json(path, value),
            None => Ok(()),
        }
    }

    fn write_secret<T: Serialize>(&self, value: &T) -> io::Result<()> {
        match &self.path {
            Some(path) => write_secret_json(path, value),
            None => Ok(()),
        }
    }

    /// What this response should tell the learner about the file, if anything.
    ///
    /// Taken when the store is writing normally — the learner has now been told the
    /// old file was kept aside — and repeated while it runs in memory.
    fn notice(&mut self) -> Option<String> {
        if self.path.is_some() {
            self.warning.take()
        } else {
            self.warning.clone()
        }
    }

    /// The warning for a response that has just tried to write: the notice, and the
    /// write's own failure after it. `done` says what still happened
    /// ("the answer was counted").
    fn report(&mut self, saved: io::Result<()>, done: &str) -> Option<String> {
        let failed = saved
            .err()
            .map(|err| format!("{done} but could not be saved: {err}"));
        join(self.notice(), failed)
    }
}

/// Just the `version` of a file, whatever shape the rest of it has.
#[derive(Deserialize)]
struct VersionProbe {
    #[serde(default)]
    version: Option<u32>,
}

/// Two things to tell the learner, as one line, in the order they happened.
fn join(first: Option<String>, then: Option<String>) -> Option<String> {
    match (first, then) {
        (Some(first), Some(then)) => Some(format!("{first}; {then}")),
        (first, then) => first.or(then),
    }
}

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
/// The same file ritual as the other stores (see the module's header), and a write
/// failure is returned rather than swallowed. Losing a preference is cheap, which
/// is exactly why it must not be allowed to take the schedule with it — it lives
/// in its own file for that reason and not for tidiness.
pub struct PrefsStore {
    file: StoreFile,
    prefs: Prefs,
}

impl PrefsStore {
    /// A store that keeps its answer in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            file: StoreFile::in_memory(),
            prefs: Prefs::default(),
        }
    }

    /// Load `prefs.json` from `dir`, or start on the default.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let (prefs, file) = StoreFile::open(
            dir.as_ref().join(PREFS_FILE_NAME),
            "the remembered section",
            None,
        );
        Self { file, prefs }
    }

    /// What is remembered.
    pub fn prefs(&self) -> Prefs {
        self.prefs
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.file.is_persistent()
    }

    /// Remember the section, writing the file, and say what the learner should be
    /// told.
    ///
    /// The answer is kept **even if the write fails** — the learner did move — and
    /// the failure comes back rather than being swallowed, for the same reason the
    /// drill's and the schedule's do: a preference that is not being saved should
    /// not look like one that is.
    pub fn set_section(&mut self, section: Section) -> Option<String> {
        self.prefs.section = Some(section);
        let saved = self.file.write(&self.prefs);
        self.file.report(saved, "the section was changed")
    }
}

/// What the app knows about this learner's confusion pairs, and where it is kept.
pub struct ConfusionStore {
    /// In memory when there is nowhere to write — the tallies still work, for this
    /// session only. An app that cannot remember is still better than an app that
    /// will not start.
    file: StoreFile,
    log: ConfusionLog,
}

impl ConfusionStore {
    /// A store that keeps its record in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            file: StoreFile::in_memory(),
            log: ConfusionLog::new(),
        }
    }

    /// Load `confusions.json` from `dir`, or start empty.
    ///
    /// A missing file is not an error: that is the first run, and the first
    /// answer creates it.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let (log, file) = StoreFile::open(
            dir.as_ref().join(FILE_NAME),
            "the kana drill's record",
            None,
        );
        Self { file, log }
    }

    /// The record itself.
    pub fn log(&self) -> &ConfusionLog {
        &self.log
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.file.is_persistent()
    }

    /// Record one answer against a pair, write the file, and say what the record
    /// now is.
    ///
    /// The record is kept **even if the write fails** — the answer happened — and
    /// the failure travels as the warning beside it rather than instead of it,
    /// because a learner whose progress is not being saved should be told rather
    /// than discover it later, and should not be asked the same question twice.
    pub fn record(&mut self, key: &str, correct: bool) -> Saved<PairTally> {
        let value = self.log.record(key, correct);
        let saved = self.save();
        Saved {
            value,
            warning: self.file.report(saved, "the answer was counted"),
        }
    }

    /// Write the file, if there is one to write.
    pub fn save(&self) -> io::Result<()> {
        self.file.write(&self.log)
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
    /// In memory when there is nowhere to write, as in the other stores: the
    /// tallies still work for this session.
    file: StoreFile,
    log: GrammarLog,
}

impl GrammarStore {
    /// A store that keeps its record in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            file: StoreFile::in_memory(),
            log: GrammarLog::new(),
        }
    }

    /// Load `grammar.json` from `dir`, or start empty.
    ///
    /// A missing file is not an error: that is the first run, and the first answer
    /// creates it.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let (log, file) = StoreFile::open(
            dir.as_ref().join(GRAMMAR_FILE_NAME),
            "the grammar drill's record",
            None,
        );
        Self { file, log }
    }

    /// The record itself.
    pub fn log(&self) -> &GrammarLog {
        &self.log
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.file.is_persistent()
    }

    /// Record one answer against a pattern, write the file, and say what the record
    /// now is.
    ///
    /// The same rule the kana drill's store follows: the record is kept **even if
    /// the write fails** — the answer happened — and the failure travels beside the
    /// record rather than instead of it.
    pub fn record(&mut self, id: &str, correct: bool) -> Saved<PairTally> {
        let value = self.log.record(id, correct);
        let saved = self.save();
        Saved {
            value,
            warning: self.file.report(saved, "the answer was counted"),
        }
    }

    /// Write the file, if there is one to write.
    pub fn save(&self) -> io::Result<()> {
        self.file.write(&self.log)
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
    /// In memory when there is nowhere to write, as in the other stores: this
    /// session keeps the sentences and the next one will not.
    file: StoreFile,
    log: CompositionLog,
}

impl CompositionStore {
    /// A store that keeps the learner's sentences in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            file: StoreFile::in_memory(),
            log: CompositionLog::new(),
        }
    }

    /// Load `compositions.json` from `dir`, or start empty.
    ///
    /// A missing file is not an error: that is the first run, and the first sentence
    /// written creates it. A file written by a **newer** build is refused and left
    /// untouched, as the review schedule's is: this build would read it with the
    /// fields it does not know dropped, and writing it back would lose them for good.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let (log, file) = StoreFile::open(
            dir.as_ref().join(COMPOSITIONS_FILE_NAME),
            "the file of your written sentences",
            Some(CompositionLog::new().version),
        );
        Self { file, log }
    }

    /// What has been written.
    pub fn log(&self) -> &CompositionLog {
        &self.log
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.file.is_persistent()
    }

    /// Keep one sentence, write the file, and answer how many this pattern holds.
    ///
    /// The same rule the other stores follow: the sentence is kept **even if the
    /// write fails** — the learner wrote it — and the failure travels beside the
    /// count rather than being swallowed, because a learner whose own writing is not
    /// being saved should be told rather than discover it later.
    pub fn add(&mut self, id: &str, text: &str, at: &str) -> Saved<usize> {
        let value = self.log.add(id, text, at);
        let saved = self.save();
        Saved {
            value,
            warning: self.file.report(saved, "the sentence was kept"),
        }
    }

    /// What the learner should be told about this file, when nothing was written.
    pub fn notice(&mut self) -> Option<String> {
        self.file.notice()
    }

    /// Write the file, if there is one to write.
    pub fn save(&self) -> io::Result<()> {
        self.file.write(&self.log)
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
    /// In memory when there is nowhere to write: the settings still work for this
    /// session.
    file: StoreFile,
    settings: ModelSettings,
}

impl ModelStore {
    /// A store that keeps the settings in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            file: StoreFile::in_memory(),
            settings: ModelSettings::default(),
        }
    }

    /// Load `model.json` from `dir`, or start unconfigured.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let (settings, file) = StoreFile::open(
            dir.as_ref().join(MODEL_FILE_NAME),
            "the model settings",
            None,
        );
        Self { file, settings }
    }

    /// What the learner configured.
    pub fn settings(&self) -> &ModelSettings {
        &self.settings
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.file.is_persistent()
    }

    /// What the learner should be told about this file, if anything — see
    /// [`StoreFile::notice`] for when it is said once and when on every response.
    pub fn notice(&mut self) -> Option<String> {
        self.file.notice()
    }

    /// Replace the settings, writing the file, and say what the learner should be
    /// told.
    ///
    /// The settings are kept **even if the write fails** — the learner typed them and
    /// this session uses them from now on — so the failure is a warning beside them,
    /// for the reason the other stores give, and not an error that would leave a
    /// screen drawing the settings this session has already replaced.
    pub fn save(&mut self, settings: ModelSettings) -> Option<String> {
        self.settings = settings;
        let saved = self.file.write_secret(&self.settings);
        self.file.report(saved, "the settings were kept")
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
                eprintln!(
                    "{}: {why}; moving it aside and starting a fresh schedule",
                    path.display()
                );
                match set_aside(&path) {
                    // The unreadable file is out of the way, so a fresh schedule
                    // can take its place and be written to from here on.
                    Ok(aside) => match ProgressStore::open(&path) {
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

    /// Why this session may not be remembered, when there is a reason, without
    /// counting as having told the learner.
    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    /// What this response should tell the learner about the schedule's file, by
    /// the rule the other stores follow: "kept aside" once, since the fresh
    /// schedule is then working; "not remembered" on every response, for as long
    /// as it is true.
    pub fn notice(&mut self) -> Option<String> {
        if self.is_persistent() {
            self.warning.take()
        } else {
            self.warning.clone()
        }
    }

    /// Offer a graded attempt to the schedule and say what it did.
    ///
    /// The card is kept **even if the write fails** — the attempt happened — and
    /// the failure comes back on the outcome rather than being swallowed, because
    /// a learner whose progress is not being saved should be told rather than
    /// discover it later.
    ///
    /// Every outcome — practice, a counted review, or an attempt that could not be
    /// counted — carries [`Self::notice`] as well, joined to any failure of its own,
    /// so that neither hides the other.
    pub fn record_attempt(&mut self, ch: char, score: f32) -> Result<AttemptOutcome, String> {
        let now = now_iso8601();
        if let Some(card) = self.store.card(ch) {
            if !card.is_due(&now) {
                let due = card.due.clone();
                return Ok(AttemptOutcome {
                    counted: false,
                    due,
                    warning: self.notice(),
                });
            }
        }
        let card = match self.store.record(ch, score) {
            Ok(card) => card,
            Err(err) => {
                let why = join(self.notice(), Some(err.to_string()));
                return Err(why.unwrap_or_default());
            }
        };
        let failed = self
            .store
            .save()
            .err()
            .map(|err| format!("the attempt was counted but could not be saved: {err}"));
        let warning = join(self.notice(), failed);
        Ok(AttemptOutcome {
            counted: true,
            due: card.due,
            warning,
        })
    }
}

/// Move a file that will not parse out of the way, and say where it went.
///
/// The new name is the old one with `.corrupt-` and the time appended —
/// `confusions.json.corrupt-20261106T150625Z` — and a counter after that if the name
/// is taken, so a second bad file can never land on the copy kept from the first.
/// The time is written without its colons, which some platforms do not allow in a
/// file name. The name is built on the path's own bytes rather than on its
/// `display()`, which would replace whatever in a data directory's name is not
/// UTF-8 and so name a directory that does not exist.
fn set_aside(path: &Path) -> io::Result<PathBuf> {
    let stamp: String = now_iso8601()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    for n in 0u32.. {
        let mut name = path.as_os_str().to_owned();
        name.push(format!(".corrupt-{stamp}"));
        if n > 0 {
            name.push(format!("-{n}"));
        }
        let aside = PathBuf::from(name);
        // Only a name that is certainly absent is free. Anything else — a file, a
        // dangling link, or a name that cannot be looked at — is passed over or is
        // a reason to stop, never a reason to rename onto it.
        match fs::symlink_metadata(&aside) {
            Ok(_) => continue,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
        if move_without_replacing(path, &aside)? {
            return Ok(aside);
        }
    }
    Err(io::Error::other(
        "no free name to keep the file aside under",
    ))
}

/// Move `from` to `to` unless `to` exists, answering whether it moved.
///
/// A rename replaces whatever is at its target, so on Unix the move is a hard link —
/// which refuses an existing target, atomically — followed by removing the old
/// name. A file system that cannot link (FAT, some network mounts) falls back to the
/// rename, which is safe there because the name was just seen to be free and this
/// is the only process writing this directory.
fn move_without_replacing(from: &Path, to: &Path) -> io::Result<bool> {
    #[cfg(unix)]
    match fs::hard_link(from, to) {
        Ok(()) => return fs::remove_file(from).map(|()| true),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
        Err(_) => {}
    }
    fs::rename(from, to).map(|()| true)
}

/// Write a JSON file whole, via a neighbouring temporary and a rename.
///
/// A rename within a directory is atomic, so a reader either sees the previous
/// file or the new one and never a half-written file — which matters because these
/// files are rewritten after *every* answer, and the machine can lose power between
/// two of them.
fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    write_atomically(path, value, false)
}

/// Write a JSON file whole, readable and writable only by its owner.
///
/// The same temporary-plus-rename as [`write_json`], with the mode set on the
/// temporary before anything is written into it, so the secret is never briefly
/// world-readable. On Unix the mode is `0600`; on the platforms where that means
/// nothing the file is simply written, which is what the platform's own directory
/// permissions are for.
fn write_secret_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    write_atomically(path, value, true)
}

/// The one writer behind [`write_json`] and [`write_secret_json`].
///
/// Atomic is not the same as durable. A rename is ordered against nothing the file
/// system has not yet flushed, so after a power cut on APFS, or on ext4 and XFS
/// without `auto_da_alloc`, the new name can point at a file whose bytes never
/// reached the disk — a zero-length history. So the temporary is synced before it is
/// renamed, and the directory after, which is what makes the rename itself survive.
fn write_atomically<T: Serialize>(path: &Path, value: &T, owner_only: bool) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        // The secret's temporary is created `0600`, so there is no moment at which it
        // exists with the default mode; the explicit `set_permissions` after it is for
        // a stale temporary left by an earlier crash, which `mode` does not touch.
        #[cfg(unix)]
        if owner_only {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        #[cfg(unix)]
        if owner_only {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        #[cfg(not(unix))]
        let _ = owner_only;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    sync_parent(path);
    Ok(())
}

/// Flush the directory entry a rename just changed, on the platforms that can.
///
/// Best effort, deliberately: by now the file's own bytes are on the disk and the
/// rename has happened, so a file system that refuses to sync a directory (some
/// network and FUSE mounts do) has not lost the answer, and reporting "could not be
/// saved" would be untrue. Windows has no directory handle to sync; its rename is
/// already durable once the file is.
fn sync_parent(path: &Path) {
    #[cfg(unix)]
    {
        let dir = match path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => dir,
            _ => Path::new("."),
        };
        if let Err(err) = fs::File::open(dir).and_then(|dir| dir.sync_all()) {
            eprintln!(
                "{}: the directory could not be synced ({err})",
                dir.display()
            );
        }
    }
    #[cfg(not(unix))]
    let _ = path;
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

    /// Unwrap what a store answered when nothing went wrong: the value, and the
    /// assertion that there was nothing to tell the learner.
    trait Clean<T> {
        fn expect_clean(self, why: &str) -> T;
    }

    impl<T> Clean<T> for Saved<T> {
        fn expect_clean(self, why: &str) -> T {
            assert_eq!(self.warning, None, "{why}");
            self.value
        }
    }

    impl Clean<()> for Option<String> {
        fn expect_clean(self, why: &str) {
            assert_eq!(self, None, "{why}");
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

        let tally = store.record(&shi_tsu(), false).expect_clean("saves");
        assert_eq!(tally.asked, 1);
        assert_eq!(tally.wrong, 1);
        assert!(dir.path().join(FILE_NAME).exists(), "the answer was written");
    }

    #[test]
    fn the_record_survives_a_restart() {
        let dir = TempDir::new("restart");
        {
            let mut store = ConfusionStore::at_dir(dir.path());
            store.record(&shi_tsu(), false).expect_clean("saves");
            store.record(&shi_tsu(), true).expect_clean("saves");
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
        store.record(&shi_tsu(), false).expect_clean("saves");

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
        assert!(asides(dir.path(), FILE_NAME).is_empty());
    }

    // ---- the learner's own sentences ---------------------------------------

    #[test]
    fn a_first_run_has_written_nothing_and_the_first_sentence_makes_the_file() {
        let dir = TempDir::new("composition-first-run");
        let mut store = CompositionStore::at_dir(dir.path());
        assert!(store.is_persistent());
        assert!(store.log().is_empty());
        assert!(!dir.path().join(COMPOSITIONS_FILE_NAME).exists(), "nothing written yet");

        let count = store
            .add(
                "ni-hanshite",
                "彼の予想に反して、試合は延期された。",
                "2026-10-07T00:00:00Z",
            )
            .expect_clean("saves");
        assert_eq!(count, 1);
        assert!(dir.path().join(COMPOSITIONS_FILE_NAME).exists(), "the sentence was written");
    }

    #[test]
    fn the_sentences_survive_a_restart_in_the_order_they_were_written() {
        let dir = TempDir::new("composition-restart");
        {
            let mut store = CompositionStore::at_dir(dir.path());
            store
                .add("wa", "私は学生です。", "2026-10-07T00:00:00Z")
                .expect_clean("saves");
            store
                .add("wa", "今日は天気がいいです。", "2026-10-07T00:01:00Z")
                .expect_clean("saves");
            store
                .add("ga", "雨が降っています。", "2026-10-07T00:02:00Z")
                .expect_clean("saves");
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
        store
            .add("wa", "私は学生です。", "2026-10-07T00:00:00Z")
            .expect_clean("saves");

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
            fs::read_to_string(aside_of(dir.path(), "compositions.json")).expect("still there"),
            "{ this is not json",
            "kept as it was, not as an empty file"
        );

        // And the app carries on: the next sentence writes a valid file, and says
        // where the old one went.
        let saved = store.add("wa", "私は学生です。", "2026-10-07T00:00:00Z");
        assert!(saved.warning.is_some_and(|why| why.contains("kept as")));
        let reread = CompositionStore::at_dir(dir.path());
        assert_eq!(reread.log().sentences("wa"), vec!["私は学生です。"]);
    }

    #[test]
    fn a_write_failure_is_reported_and_the_sentence_is_still_kept_in_memory() {
        let dir = TempDir::new("composition-unwritable");
        fs::create_dir_all(dir.path().join("compositions.json.tmp"))
            .expect("a directory in the way");
        let mut store = CompositionStore::at_dir(dir.path());
        let saved = store.add("wa", "私は学生です。", "2026-10-07T00:00:00Z");
        assert_eq!(saved.value, 1, "the sentence is counted");
        let warning = saved.warning.expect("the failure surfaces");
        assert!(
            warning.contains("the sentence was kept but could not be saved"),
            "{warning}"
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
        let aside = aside_of(dir.path(), "confusions.json");
        assert!(aside.exists(), "the unreadable file was kept");
        assert_eq!(
            fs::read_to_string(&aside).expect("still there"),
            "{ this is not json",
            "and kept as it was, not as an empty file"
        );

        // And the app carries on: the next answer writes a valid file, and says
        // where the old one went.
        let saved = store.record(&shi_tsu(), false);
        assert!(saved.warning.is_some_and(|why| why.contains("kept as")));
        let reread = ConfusionStore::at_dir(dir.path());
        assert_eq!(reread.log().tally(&shi_tsu()).wrong, 1);
    }

    /// The files `name` has been moved aside as, in `dir`.
    fn asides(dir: &Path, name: &str) -> Vec<PathBuf> {
        let prefix = format!("{name}.corrupt-");
        fs::read_dir(dir)
            .expect("the directory")
            .map(|entry| entry.expect("an entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|file| file.to_str())
                    .is_some_and(|file| file.starts_with(&prefix))
            })
            .collect()
    }

    /// The one file `name` has been moved aside as, in `dir`.
    fn aside_of(dir: &Path, name: &str) -> PathBuf {
        let mut found = asides(dir, name);
        assert_eq!(found.len(), 1, "one copy kept aside: {found:?}");
        found.remove(0)
    }

    /// How many files in `dir` hold exactly `bytes`, whatever they are called.
    ///
    /// The tests below assert that a learner's file *survives*, not what it was
    /// renamed to: the aside name carries a timestamp, and pinning it would test the
    /// clock rather than the promise.
    fn files_holding(dir: &Path, bytes: &[u8]) -> usize {
        fs::read_dir(dir)
            .expect("the directory")
            .filter_map(|entry| fs::read(entry.expect("an entry").path()).ok())
            .filter(|held| held == bytes)
            .count()
    }

    /// A tally file a crash cut off in the middle of a Japanese character: valid JSON
    /// up to the cut, and then two of the three bytes of シ. `read_to_string` refuses
    /// it as invalid UTF-8 before serde ever sees it, which is the path that used to
    /// fall through to "nothing recorded" with the file left where the next answer
    /// would overwrite it.
    fn cut_mid_character() -> Vec<u8> {
        let mut bytes = br#"{"pairs":{"a|b":{"asked":40,"correct":10,"wrong":30}},"x":""#.to_vec();
        bytes.extend_from_slice(&"シ".as_bytes()[..2]);
        bytes
    }

    #[test]
    fn a_file_cut_off_mid_character_is_kept_and_not_overwritten_by_the_next_answer() {
        let dir = TempDir::new("cut-mid-character");
        let path = dir.path().join(FILE_NAME);
        let cut = cut_mid_character();
        assert!(
            String::from_utf8(cut.clone()).is_err(),
            "the test file is invalid UTF-8"
        );
        fs::write(&path, &cut).expect("writes");

        let mut store = ConfusionStore::at_dir(dir.path());
        assert!(store.log().is_empty(), "an unreadable file is not a crash");
        assert!(
            store.is_persistent(),
            "the bad file is out of the way, so a fresh one is written"
        );
        let first = store.record(&shi_tsu(), false);
        let warning = first.warning.expect("the learner is told");
        assert!(warning.contains("kept as"), "{warning}");
        assert_eq!(
            files_holding(dir.path(), &cut),
            1,
            "the learner's history survives the next answer, byte for byte"
        );
        assert_eq!(
            fs::read(aside_of(dir.path(), FILE_NAME)).expect("the copy"),
            cut,
            "under the name the warning gives"
        );
        assert!(warning.contains(&aside_of(dir.path(), FILE_NAME).display().to_string()));

        // Said once: from here on the store is writing normally.
        store
            .record(&shi_tsu(), true)
            .expect_clean("an ordinary answer");
        assert_eq!(
            ConfusionStore::at_dir(dir.path())
                .log()
                .tally(&shi_tsu())
                .asked,
            2
        );
    }

    #[test]
    fn a_composition_file_cut_off_mid_character_is_kept_too() {
        let dir = TempDir::new("composition-cut-mid-character");
        let path = dir.path().join(COMPOSITIONS_FILE_NAME);
        let mut cut = br#"{"version":1,"written":{"wa":[{"text":""#.to_vec();
        cut.extend_from_slice(&"私".as_bytes()[..1]);
        fs::write(&path, &cut).expect("writes");

        let mut store = CompositionStore::at_dir(dir.path());
        let saved = store.add("wa", "私は学生です。", "2026-10-07T00:00:00Z");
        assert!(
            saved.warning.is_some_and(|why| why.contains("kept as")),
            "the learner is told"
        );
        assert_eq!(
            files_holding(dir.path(), &cut),
            1,
            "the learner's sentences survive"
        );
    }

    #[test]
    fn a_second_corruption_does_not_destroy_the_copy_kept_after_the_first() {
        let dir = TempDir::new("corrupt-twice");
        let path = dir.path().join(GRAMMAR_FILE_NAME);

        fs::write(&path, "{ the first bad file").expect("writes");
        let mut store = GrammarStore::at_dir(dir.path());
        assert!(store.record("te-kudasai", true).warning.is_some());

        fs::write(&path, "{ the second bad file").expect("writes");
        let mut store = GrammarStore::at_dir(dir.path());
        assert!(store.record("te-kudasai", true).warning.is_some());

        assert_eq!(
            asides(dir.path(), GRAMMAR_FILE_NAME).len(),
            2,
            "two copies, two names"
        );

        assert_eq!(
            files_holding(dir.path(), b"{ the first bad file"),
            1,
            "the first copy"
        );
        assert_eq!(
            files_holding(dir.path(), b"{ the second bad file"),
            1,
            "the second copy"
        );
    }

    #[test]
    fn a_composition_file_from_a_newer_build_is_not_rewritten() {
        // A later build may add fields this one does not know. Reading the file would
        // drop them, and writing it back would lose them for good — the review
        // schedule already refuses such a file, and the learner's own writing deserves
        // the same.
        let dir = TempDir::new("composition-future");
        let path = dir.path().join(COMPOSITIONS_FILE_NAME);
        let future = r#"{"version":99,"written":{"wa":[{"text":"未来","at":"2030-01-01T00:00:00Z"}]},"drafts":{}}"#;
        fs::write(&path, future).expect("writes");

        let mut store = CompositionStore::at_dir(dir.path());
        assert!(!store.is_persistent(), "nothing may be written over it");
        assert!(
            store.log().is_empty(),
            "and nothing of it is half-read either"
        );
        for _ in 0..2 {
            let saved = store.add("wa", "私は学生です。", "2026-10-07T00:00:00Z");
            let warning = saved.warning.expect("the learner is told, every time");
            assert!(warning.contains("newer version"), "{warning}");
            assert!(warning.contains("will not be remembered"), "{warning}");
        }
        assert_eq!(
            store.log().sentences("wa").len(),
            2,
            "this session still keeps them"
        );
        assert_eq!(
            fs::read_to_string(&path).expect("still there"),
            future,
            "the file this build does not understand was not touched"
        );
        assert!(
            asides(dir.path(), COMPOSITIONS_FILE_NAME).is_empty(),
            "nor moved"
        );
    }

    #[test]
    fn an_in_memory_store_records_without_writing_anything() {
        let mut store = ConfusionStore::in_memory();
        assert!(!store.is_persistent());
        store
            .record(&shi_tsu(), false)
            .expect_clean("nothing to write");
        assert_eq!(store.log().tally(&shi_tsu()).wrong, 1);
    }

    #[test]
    fn a_write_failure_is_reported_rather_than_hidden() {
        // A directory where the atomic write's temporary goes is the cheapest way to
        // make the write fail on every platform, and it must come back as a warning
        // beside the tally rather than a panic — or worse, as nothing at all.
        let dir = TempDir::new("unwritable");
        fs::create_dir_all(dir.path().join("confusions.json.tmp")).expect("a directory in the way");
        let mut store = ConfusionStore::at_dir(dir.path());
        let saved = store.record(&shi_tsu(), false);
        assert_eq!(saved.value.wrong, 1, "the tally comes back");
        let warning = saved.warning.expect("the failure surfaces");
        assert!(
            warning.contains("the answer was counted but could not be saved"),
            "{warning}"
        );
        assert_eq!(
            store.log().tally(&shi_tsu()).wrong,
            1,
            "and the answer is still counted in memory"
        );
        // Each failed write is its own news, so the next answer says so again.
        assert!(store.record(&shi_tsu(), true).warning.is_some());
    }

    #[test]
    fn a_file_that_cannot_be_read_at_all_is_left_alone_and_the_store_runs_in_memory() {
        // A directory in the file's place cannot be read, and neither can a file the
        // learner no longer has permission for or one on a failing disk. None of those
        // is evidence the file is bad, so it is not moved and not written over: this
        // session runs in memory and says so on every answer.
        let dir = TempDir::new("unreadable");
        fs::create_dir_all(dir.path().join(FILE_NAME)).expect("a directory in the file's place");
        let mut store = ConfusionStore::at_dir(dir.path());
        assert!(!store.is_persistent(), "nothing may be written over it");
        for _ in 0..2 {
            let saved = store.record(&shi_tsu(), false);
            let warning = saved.warning.expect("the learner is told, every time");
            assert!(warning.contains("could not be opened"), "{warning}");
            assert!(warning.contains("will not be remembered"), "{warning}");
        }
        assert_eq!(
            store.log().tally(&shi_tsu()).wrong,
            2,
            "and the answers still count"
        );
        assert!(
            dir.path().join(FILE_NAME).is_dir(),
            "what was there is still there"
        );
        assert!(
            asides(dir.path(), FILE_NAME).is_empty(),
            "and it was not moved"
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
            .expect_clean("saves");
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
                .expect_clean("saves");
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
            .expect_clean("saves");

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
            fs::read_to_string(aside_of(dir.path(), "model.json")).expect("still there"),
            "{ this is not json"
        );
        assert!(
            store.notice().is_some_and(|why| why.contains("kept as")),
            "the learner is told"
        );
        assert_eq!(store.notice(), None, "once");

        // And the app carries on: saving writes a valid file.
        store
            .save(ModelSettings {
                endpoint: "http://127.0.0.1:1234/v1".to_string(),
                model: "local-model".to_string(),
                key: None,
            })
            .expect_clean("saves");
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
        let aside = aside_of(dir.path(), "review.json");
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
    fn a_schedule_cut_off_mid_character_is_kept_aside_and_a_fresh_one_started() {
        // The schedule used to be read as a string, so a file cut off inside あ was an
        // I/O error rather than a damaged file: never moved aside, and the app ran in
        // memory on every launch from then on.
        let dir = TempDir::new("review-cut");
        let mut cut = br#"{"version":1,"cards":{""#.to_vec();
        cut.extend_from_slice(&"あ".as_bytes()[..2]);
        fs::write(dir.path().join(REVIEW_FILE_NAME), &cut).expect("writes");

        let mut store = ReviewStore::at_dir(dir.path());
        assert!(store.is_persistent(), "the bad file is out of the way");
        assert!(
            store.warning().is_some_and(|why| why.contains("kept as")),
            "{:?}",
            store.warning()
        );
        assert_eq!(
            files_holding(dir.path(), &cut),
            1,
            "the history survives, byte for byte"
        );
        store.record_attempt('あ', 20.0).expect("records");
        assert!(
            dir.path().join(REVIEW_FILE_NAME).exists(),
            "and a fresh schedule is written"
        );
        assert_eq!(files_holding(dir.path(), &cut), 1);
    }

    #[test]
    fn the_schedules_notice_is_said_once_and_is_not_hidden_by_a_failed_write() {
        // The rule the other stores follow: "kept aside" is news once, and a failed
        // write is news every time — and when both happen on the same attempt, the
        // learner hears both rather than whichever was looked at first.
        let dir = TempDir::new("review-notice");
        fs::write(dir.path().join(REVIEW_FILE_NAME), "{ this is not json").expect("writes");
        let mut store = ReviewStore::at_dir(dir.path());
        fs::create_dir_all(dir.path().join("review.json.tmp")).expect("a directory in the way");

        let first = store.record_attempt('あ', 20.0).expect("records");
        let warning = first.warning.expect("the learner is told");
        assert!(warning.contains("kept as"), "{warning}");
        assert!(warning.contains("could not be saved"), "{warning}");

        let second = store.record_attempt('い', 20.0).expect("records");
        let warning = second.warning.expect("the failed write is still news");
        assert!(!warning.contains("kept as"), "said once: {warning}");
        assert!(warning.contains("could not be saved"), "{warning}");
    }

    #[test]
    fn a_composition_file_from_a_newer_build_with_a_new_shape_is_refused_not_moved() {
        // The version is read before the shape: a newer build that restructured the
        // file has written a file this build must not touch, not a damaged one.
        let dir = TempDir::new("composition-future-shape");
        let path = dir.path().join(COMPOSITIONS_FILE_NAME);
        let future = r#"{"version":2,"written":["a shape this build has never seen"]}"#;
        fs::write(&path, future).expect("writes");

        let mut store = CompositionStore::at_dir(dir.path());
        assert!(!store.is_persistent(), "nothing may be written over it");
        let saved = store.add("wa", "私は学生です。", "2026-10-07T00:00:00Z");
        let warning = saved.warning.expect("the learner is told");
        assert!(warning.contains("newer version"), "{warning}");
        assert_eq!(fs::read_to_string(&path).expect("still there"), future);
        assert!(
            asides(dir.path(), COMPOSITIONS_FILE_NAME).is_empty(),
            "and not moved aside"
        );
    }

    // Linux only, because only there can the directory exist at all: APFS (macOS,
    // iOS) refuses a name that is not valid UTF-8 with EILSEQ, so on Apple systems
    // the path this protects cannot arise. Android's ext4/f2fs accept it, as Linux
    // does, and are covered by the same code.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_bad_file_in_a_directory_whose_name_is_not_utf8_is_still_moved_aside() {
        // A data directory's name is the platform's, not necessarily UTF-8. Building
        // the aside name through `display()` would replace the odd byte, name a
        // directory that does not exist, fail the rename, and leave the store in
        // memory on every launch.
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let root = TempDir::new("non-utf8");
        let dir = root.path().join(OsStr::from_bytes(b"data-\xff"));
        fs::create_dir_all(&dir).expect("a directory with a non-UTF-8 name");
        fs::write(dir.join(FILE_NAME), "{ this is not json").expect("writes");

        let mut store = ConfusionStore::at_dir(&dir);
        assert!(
            store.is_persistent(),
            "the bad file was moved, so a fresh one is written"
        );
        assert_eq!(files_holding(&dir, b"{ this is not json"), 1);
        assert_eq!(
            asides(&dir, FILE_NAME).len(),
            1,
            "beside the original, in the same directory"
        );
        let saved = store.record(&shi_tsu(), false);
        assert!(saved.warning.is_some_and(|why| why.contains("kept as")));
        assert!(dir.join(FILE_NAME).exists());
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
            store.record("te-kudasai", false).expect_clean("writes");
            store.record("te-kudasai", true).expect_clean("writes");
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
            fs::read_to_string(aside_of(dir.path(), "grammar.json")).expect("still there"),
            "{ this is not json",
            "kept as it was, not as an empty file"
        );

        // And the app carries on: the next answer writes a valid file, and says
        // where the old one went.
        let saved = store.record("te-kudasai", true);
        assert!(saved.warning.is_some_and(|why| why.contains("kept as")));
        assert_eq!(GrammarStore::at_dir(dir.path()).log().patterns_seen(), 1);
    }

    #[test]
    fn a_grammar_write_that_fails_is_reported_and_the_answer_is_still_counted() {
        let dir = TempDir::new("grammar-unwritable");
        fs::create_dir_all(dir.path().join("grammar.json.tmp")).expect("a directory in the way");
        let mut store = GrammarStore::at_dir(dir.path());

        let saved = store.record("te-kudasai", false);
        assert_eq!(
            saved.value,
            PairTally {
                asked: 1,
                correct: 0,
                wrong: 1
            }
        );
        assert!(
            saved
                .warning
                .is_some_and(|why| why.contains("could not be saved")),
            "the failure surfaces"
        );
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
        store
            .record("te-kudasai", true)
            .expect_clean("nothing to write");
        assert_eq!(store.log().tally("te-kudasai").correct, 1);
    }

    #[test]
    fn the_section_survives_a_restart_and_is_readable_json() {
        let dir = TempDir::new("prefs-restart");
        {
            let mut store = PrefsStore::at_dir(dir.path());
            store.set_section(Section::Kanji).expect_clean("writes");
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
            fs::read_to_string(aside_of(dir.path(), "prefs.json")).expect("still there"),
            "{ this is not json",
            "kept as it was, not as an empty file"
        );

        // And the app carries on: the next move writes a valid file, and says where
        // the old one went.
        let warning = store
            .set_section(Section::Kanji)
            .expect("the learner is told");
        assert!(warning.contains("kept as"), "{warning}");
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

        let warning = store
            .set_section(Section::Kanji)
            .expect("the failure surfaces");
        assert!(warning.contains("could not be saved"), "{warning}");
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
        store
            .set_section(Section::Kanji)
            .expect_clean("nothing to write");
        assert_eq!(store.prefs().section, Some(Section::Kanji));
    }
}
