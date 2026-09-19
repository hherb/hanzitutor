//! Loading the character dataset that the app grades against, and the pieces of
//! application state that outlive a single command.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use hanzi_core::{
    build_lessons, build_queue, now_iso8601, CursorStore, CursorView, Dataset, ProgressStore,
    ProgressView, ReviewView, SettingsStore, SettingsView, VocabStore, VocabView,
};
use hanzi_core::pinyin::{tone_target as build_tone_target, ToneTarget};
use hanzi_core::tone::analyze;
use hanzi_store::Db;
use tauri::{AppHandle, Manager};

use crate::capture::{Recorder, Recording};
use crate::commands::{ToneResult, ToneSyllableResult, LESSON_SIZE};
use crate::speech::Speaker;

/// The compact artifact produced by `hanzi-core`'s `prepare-data` binary.
///
/// It is embedded rather than read from disk so that development builds and
/// bundled apps behave identically, with no resource-path resolution to get
/// wrong. It is about 13 MB compressed.
const ARTIFACT: &[u8] = include_bytes!("../../crates/hanzi-core/data/hanzi.bin.gz");

/// Longest text tone practice will score, in syllables.
///
/// A word, not a sentence. Four covers every HSK word, and the limit is here
/// because the syllable boundaries of a whole sentence cannot be found reliably
/// from energy alone — several syllables run together with no consonant between
/// them. Refusing past this is more honest than dividing a sentence into four
/// pieces and scoring the pieces as though they were words.
const MAX_TONE_SYLLABLES: usize = 4;

/// Overrides where study data is stored, as an environment variable.
///
/// Useful for portable installs, for keeping study data outside the standard
/// application support folder, and for testing persistence in a specific place.
pub const DATA_DIR_ENV: &str = "HANZI_TUTOR_DATA_DIR";
/// The command-line argument that does the same job, taking precedence over the
/// variable above.
///
/// `--user-dir` follows the convention the rest of the command-line world uses;
/// `--user_dir` is accepted as an alias because that is what this project's own
/// notes called it first, and a flag that has to be looked up is a flag that gets
/// mistyped. Either may be written `--user-dir <path>` or `--user-dir=<path>`.
pub const USER_DIR_FLAG: &str = "--user-dir";
pub const USER_DIR_FLAG_ALIAS: &str = "--user_dir";

/// What to say when the flag is there but the directory is not.
const USER_DIR_MISSING: &str = "--user-dir needs a directory, written as --user-dir <path>";

/// How many items one review session takes.
///
/// The queue itself is not capped — everything due is due — but a session that
/// asked for two hundred characters would never be finished, so the interface
/// takes the most overdue few and reports how many are left.
pub const REVIEW_LIMIT: usize = 20;

/// A store, the file behind it, and what went wrong reading that file.
///
/// All three documents this app persists follow the same rule, so it lives in one
/// place rather than three: a missing file is a fresh start, but a file that
/// **cannot be parsed is never overwritten**. The store falls back to memory,
/// keeps the reason in [`Self::load_error`], and refuses to save until it is
/// resolved — losing someone's study history to a parse error would be far worse
/// than refusing to write.
pub struct Persisted<S> {
    pub store: S,
    pub path: PathBuf,
    /// Why the saved file could not be opened. While this is set, saving is
    /// refused.
    pub load_error: Option<String>,
    /// What to call this document when explaining a problem to the reader.
    noun: &'static str,
}

impl<S> Persisted<S> {
    /// Wrap a store that loaded cleanly.
    pub fn loaded(store: S, path: PathBuf, noun: &'static str) -> Self {
        Self {
            store,
            path,
            load_error: None,
            noun,
        }
    }

    /// Wrap a fallback store, recording why the real one could not be read.
    pub fn failed(
        store: S,
        path: PathBuf,
        noun: &'static str,
        error: impl std::fmt::Display,
    ) -> Self {
        Self {
            store,
            load_error: Some(format!(
                "Your saved {noun} at {} could not be read: {error}. \
                 Changes are kept in memory only and will not be saved until this \
                 is resolved — move or fix that file, then restart.",
                path.display()
            )),
            path,
            noun,
        }
    }

    /// Persist through `save`, refusing while a load error is set.
    ///
    /// A failure is reported rather than returned: the change *did* take effect
    /// in memory, so the interface should show it while saying it was not written.
    pub fn save_with<E: std::fmt::Display>(
        &mut self,
        save: impl FnOnce(&mut S) -> Result<(), E>,
    ) -> Option<String> {
        if let Some(reason) = &self.load_error {
            return Some(reason.clone());
        }
        save(&mut self.store)
            .err()
            .map(|e| format!("could not save your {}: {e}", self.noun))
    }
}

/// The personal vocabulary list.
pub type VocabState = Persisted<VocabStore>;

impl Persisted<VocabStore> {
    /// Open the list from the study database, degrading to an in-memory one with
    /// an explanation if it cannot be read.
    ///
    /// The failure this reports is the import's: a `vocabulary.json` that cannot
    /// be parsed leaves the list unwritten rather than replaced by an empty one,
    /// exactly as it did when the list *was* that file.
    fn open_database(db: &Db, path: PathBuf) -> Self {
        const NOUN: &str = "vocabulary list";
        match VocabStore::open_with(Box::new(db.clone())) {
            Ok(store) => Self::loaded(store, path, NOUN),
            Err(error) => Self::failed(VocabStore::in_memory(), path, NOUN, error),
        }
    }

    /// A list kept in memory because there is no data directory at all.
    fn in_memory() -> Self {
        const NOUN: &str = "vocabulary list";
        Self::loaded(VocabStore::in_memory(), PathBuf::new(), NOUN)
    }

    /// A list that cannot be opened at all, with the reason to show.
    fn unavailable(path: PathBuf, reason: &str) -> Self {
        const NOUN: &str = "vocabulary list";
        Self::failed(VocabStore::in_memory(), path, NOUN, reason)
    }

    /// The list as the interface should see it.
    pub fn view(&self) -> VocabView {
        let mut view = self.store.view();
        view.warning = self.load_error.clone();
        view
    }

    /// Persist the list, returning a warning if that was skipped or failed.
    pub fn save(&mut self) -> Option<String> {
        self.save_with(VocabStore::save)
    }
}

/// Per-character practice history and the review schedule.
pub type ProgressState = Persisted<ProgressStore>;

impl Persisted<ProgressStore> {
    /// Open the schedule from the study database. See
    /// [`Persisted::<VocabStore>::open_database`] for the failure rule.
    fn open_database(db: &Db, path: PathBuf) -> Self {
        const NOUN: &str = "practice progress";
        match ProgressStore::open_with(Box::new(db.clone())) {
            Ok(store) => Self::loaded(store, path, NOUN),
            Err(error) => Self::failed(ProgressStore::in_memory(), path, NOUN, error),
        }
    }

    /// A schedule kept in memory because there is no data directory at all.
    fn in_memory() -> Self {
        const NOUN: &str = "practice progress";
        Self::loaded(ProgressStore::in_memory(), PathBuf::new(), NOUN)
    }

    /// A schedule that cannot be opened at all, with the reason to show.
    fn unavailable(path: PathBuf, reason: &str) -> Self {
        const NOUN: &str = "practice progress";
        Self::failed(ProgressStore::in_memory(), path, NOUN, reason)
    }

    /// The schedule as the interface should see it.
    pub fn view(&self) -> ProgressView {
        let mut view = self.store.view();
        view.warning = self.load_error.clone();
        view
    }

    /// Persist the schedule, returning a warning if that was skipped or failed.
    pub fn save(&mut self) -> Option<String> {
        self.save_with(ProgressStore::save)
    }
}

/// Where the reader was in the course.
pub type CursorState = Persisted<CursorStore>;

impl Persisted<CursorStore> {
    /// Open the cursor from the study database. The cursor is a table like the
    /// others, so one bad legacy document cannot take the schedule with it —
    /// the import is per document.
    fn open_database(db: &Db, path: PathBuf) -> Self {
        const NOUN: &str = "place in the course";
        match CursorStore::open_with(Box::new(db.clone())) {
            Ok(store) => Self::loaded(store, path, NOUN),
            Err(error) => Self::failed(CursorStore::in_memory(), path, NOUN, error),
        }
    }

    /// A cursor kept in memory because there is no data directory at all.
    fn in_memory() -> Self {
        const NOUN: &str = "place in the course";
        Self::loaded(CursorStore::in_memory(), PathBuf::new(), NOUN)
    }

    /// A cursor that cannot be opened at all, with the reason to show.
    fn unavailable(path: PathBuf, reason: &str) -> Self {
        const NOUN: &str = "place in the course";
        Self::failed(CursorStore::in_memory(), path, NOUN, reason)
    }

    /// The cursor as the interface should see it.
    pub fn view(&self) -> CursorView {
        let mut view = self.store.view();
        view.warning = self.load_error.clone();
        view
    }

    /// Persist the cursor, returning a warning if that was skipped or failed.
    pub fn save(&mut self) -> Option<String> {
        self.save_with(CursorStore::save)
    }
}

/// The learner's own settings. Preferences rather than study data, but persisted
/// the same way and behind the same refusal-to-overwrite-a-bad-document rule.
pub type SettingsState = Persisted<SettingsStore>;

impl Persisted<SettingsStore> {
    /// Open the settings from the study database. They live in the same file as
    /// everything else because they have the same lifetime: a preference that
    /// could come adrift from the data it applies to would be a second thing to
    /// keep in step.
    fn open_database(db: &Db, path: PathBuf) -> Self {
        const NOUN: &str = "settings";
        match SettingsStore::open_with(Box::new(db.clone())) {
            Ok(store) => Self::loaded(store, path, NOUN),
            Err(error) => Self::failed(SettingsStore::in_memory(), path, NOUN, error),
        }
    }

    /// Settings kept in memory because there is no data directory at all.
    fn in_memory() -> Self {
        const NOUN: &str = "settings";
        Self::loaded(SettingsStore::in_memory(), PathBuf::new(), NOUN)
    }

    /// Settings that cannot be opened at all, with the reason to show.
    fn unavailable(path: PathBuf, reason: &str) -> Self {
        const NOUN: &str = "settings";
        Self::failed(SettingsStore::in_memory(), path, NOUN, reason)
    }

    /// The settings as the interface should see them.
    pub fn view(&self) -> SettingsView {
        let mut view = self.store.view();
        view.warning = self.load_error.clone();
        view
    }

    /// Persist the settings, returning a warning if that was skipped or failed.
    pub fn save(&mut self) -> Option<String> {
        self.save_with(SettingsStore::save)
    }
}

/// State held for the lifetime of the app and shared by all commands.
pub struct AppState {
    pub dataset: Dataset,
    /// Shared with a warm-up thread, so it is behind an `Arc`.
    pub speech: Arc<Speaker>,
    /// The microphone. Opened only while the learner is holding the button, so
    /// this holds nothing but the slot a recording lives in — see `capture.rs`.
    pub capture: Recorder,
    /// Behind a mutex because every mutation is read-modify-write and must be
    /// persisted as a whole document.
    pub vocab: Mutex<VocabState>,
    pub progress: Mutex<ProgressState>,
    pub cursor: Mutex<CursorState>,
    pub settings: Mutex<SettingsState>,
    /// The teachable characters, as a set.
    ///
    /// The review queue needs to know which characters the course can offer, and
    /// rebuilding the lessons on every call is not free.
    course: HashSet<char>,
}

impl AppState {
    /// Decode the embedded dataset, open the study documents and start the
    /// pronunciation warm-up.
    ///
    /// `data_dir` is where the documents live; `None` keeps them in memory, which
    /// is what the tests use.
    pub fn load(data_dir: Option<PathBuf>) -> Result<Self, String> {
        let dataset = Dataset::from_gzip_bytes(ARTIFACT)
            .map_err(|e| format!("could not read the embedded character dataset: {e}"))?;

        let speech = Arc::new(Speaker::default());
        warm_voice(Arc::clone(&speech));

        let course: HashSet<char> = build_lessons(&dataset, LESSON_SIZE)
            .into_iter()
            .flat_map(|lesson| lesson.characters)
            .collect();

        // One database holds all three stores: progress and the vocabulary list
        // are read together on every review-queue build, and a review item
        // pointing at an entry that does not exist is not a failure mode worth
        // having. What the three *separate files* used to buy — one bad document
        // not taking the others down — is kept where it still applies, in the
        // once-only import of those files, which is per document.
        let (vocab, progress, cursor, settings) = match &data_dir {
            Some(dir) => {
                let where_it_lives = dir.join(Db::FILE_NAME);
                match Db::open(dir) {
                    Ok(db) => (
                        VocabState::open_database(&db, where_it_lives.clone()),
                        ProgressState::open_database(&db, where_it_lives.clone()),
                        CursorState::open_database(&db, where_it_lives.clone()),
                        SettingsState::open_database(&db, where_it_lives),
                    ),
                    // Without the database none of the stores can be read, so all
                    // of them say so and refuse to write. Nothing was destroyed:
                    // the reason names the file, and the JSON documents the
                    // import would have read are untouched.
                    Err(reason) => (
                        VocabState::unavailable(where_it_lives.clone(), &reason),
                        ProgressState::unavailable(where_it_lives.clone(), &reason),
                        CursorState::unavailable(where_it_lives.clone(), &reason),
                        SettingsState::unavailable(where_it_lives, &reason),
                    ),
                }
            }
            // No data directory at all: the stores stay in memory, which is what
            // the tests use and what a build without a resolvable directory gets.
            None => (
                VocabState::in_memory(),
                ProgressState::in_memory(),
                CursorState::in_memory(),
                SettingsState::in_memory(),
            ),
        };

        Ok(Self {
            dataset,
            speech,
            capture: Recorder::default(),
            course,
            vocab: Mutex::new(vocab),
            progress: Mutex::new(progress),
            cursor: Mutex::new(cursor),
            settings: Mutex::new(settings),
        })
    }

    /// Lock the vocabulary list, tolerating a poisoned mutex: a panic while
    /// holding it cannot leave the document in a state that matters.
    pub fn lock_vocab(&self) -> MutexGuard<'_, VocabState> {
        self.vocab.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Lock the practice schedule, tolerating a poisoned mutex.
    pub fn lock_progress(&self) -> MutexGuard<'_, ProgressState> {
        self.progress.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Lock the course cursor, tolerating a poisoned mutex.
    pub fn lock_cursor(&self) -> MutexGuard<'_, CursorState> {
        self.cursor.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Lock the settings, tolerating a poisoned mutex.
    pub fn lock_settings(&self) -> MutexGuard<'_, SettingsState> {
        self.settings.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The tones to practise for one character or word, or `None` when nothing
    /// here can score it.
    ///
    /// Works from the text, not from a single character, so that a word is scored
    /// as the word it is — which is the point, because tone sandhi happens
    /// *between* the syllables of a word and cannot be seen one character at a
    /// time. 你好 is the example that matters: the dictionary says tone 3 + tone
    /// 3, and it is spoken 2 + 3.
    ///
    /// The reading comes from the dictionary's **whole-word** entry when there is
    /// one, because that is what resolves a polyphone — 着急 is `zháojí`, where
    /// the isolated 着 has no context to pick a reading from. Failing that, and
    /// for single characters, each character's own first reading is used, which
    /// is the same choice `lookup_text` makes and matches what the course
    /// teaches.
    ///
    /// `None` for anything longer than [`MAX_TONE_SYLLABLES`], for text the
    /// dataset does not fully know, and for a reading that cannot be divided into
    /// one syllable per character. The interface disables the control on `None`
    /// rather than offering a recording it would then have to refuse.
    pub fn tone_target(&self, text: &str) -> Option<ToneTarget> {
        let characters: Vec<char> = text.chars().collect();
        if characters.is_empty() || characters.len() > MAX_TONE_SYLLABLES {
            return None;
        }

        if characters.len() > 1 {
            if let Some(word) = self.dataset.word(text) {
                if let Some(target) = build_tone_target(text, &word.pinyin) {
                    return Some(target);
                }
            }
        }

        let mut readings = Vec::with_capacity(characters.len());
        for ch in &characters {
            let character = self.dataset.get(*ch)?;
            readings.push(character.pinyin.first()?.clone());
        }
        // Joined the way pinyin separates syllables by hand, so that `xi` + `an`
        // cannot be read back as the single syllable `xian`.
        build_tone_target(text, &readings.join("'"))
    }

    /// Score one recording against the tones that were asked for.
    ///
    /// The two things added here rather than inside the analyser are facts about
    /// the *recording* and the *word*, not about the pitch: the analyser is
    /// handed samples and tones and cannot know that they are the first ten
    /// seconds of a longer utterance, nor that the tones it was given were
    /// themselves changed by sandhi.
    pub fn score_tones(&self, recording: &Recording, target: &ToneTarget) -> ToneResult {
        let report = analyze(&recording.samples, recording.sample_rate, &target.spoken());

        // Both sides are one entry per syllable in the same order, so the zip
        // pairs the goal for a syllable with the judgement of it. A mismatch is
        // impossible by construction — `analyze` returns one entry per tone asked
        // for — but zip degrades to the shorter side rather than panicking if a
        // future change breaks that.
        let syllables: Vec<ToneSyllableResult> = target
            .syllables
            .iter()
            .zip(report.syllables.iter())
            .map(|(goal, scored)| ToneSyllableResult {
                position: scored.position,
                ch: goal.ch,
                reading: goal.reading.clone(),
                citation: goal.citation,
                spoken: goal.spoken,
                attempt: scored.attempt.clone(),
            })
            .collect();

        let mut detail = report.detail.clone();
        if target.sandhi_applied {
            detail.push(' ');
            detail.push_str(&target.detail);
        }
        if recording.truncated {
            detail.push_str(&format!(
                " (The recording hit the {}-second limit, so only the start was scored.)",
                crate::capture::MAX_RECORD_SECS
            ));
        }

        ToneResult {
            syllables,
            verdict: report.verdict,
            score: report.score,
            grade: report.grade,
            detail,
            sandhi_applied: target.sandhi_applied,
            boundaries_ms: report.boundaries_ms,
            voiced_ms: report.voiced_ms,
            span_ms: report.span_ms,
            median_hz: report.median_hz,
        }
    }

    /// Choose how a stroke is drawn, or `None` for the device's own default.
    ///
    /// Written immediately: it is one boolean, the learner has just made the
    /// choice, and a preference that only survives a clean exit is one that
    /// looks broken. A failure to save is reported the same way the other stores
    /// report one — the change stands in memory and the view says it was not
    /// written.
    pub fn set_click_to_draw(&self, value: Option<bool>) -> SettingsView {
        let mut settings = self.lock_settings();
        settings.store.set_click_to_draw(value);
        let mut view = settings.view();
        if let Some(warning) = settings.save() {
            view.warning = Some(warning);
        }
        view
    }

    /// How many characters the course holds, so a cursor can be clamped to it.
    pub fn course_len(&self) -> usize {
        self.course.len()
    }

    /// Move the course cursor, clamped to the course.
    ///
    /// The index arrives from the interface, so it is clamped here rather than
    /// trusted: a stale or hand-edited value must not point past the end.
    pub fn set_cursor(&self, index: usize) -> CursorView {
        let mut cursor = self.lock_cursor();
        let last = self.course_len().saturating_sub(1);
        cursor.store.set_index(index.min(last));

        let mut view = cursor.view();
        if let Some(warning) = cursor.save() {
            view.warning = Some(warning);
        }
        view
    }

    /// What is due for review right now.
    ///
    /// Drawn from both sources: a character in the vocabulary list is offered as
    /// its entry, because a word is practised whole and its meaning is the
    /// context worth keeping; everything else that is due comes from the course.
    /// Only the first [`REVIEW_LIMIT`] items are returned, with `due_count`
    /// carrying the true total so the interface can say how many are left.
    pub fn review_queue(&self) -> ReviewView {
        let progress = self.lock_progress();
        let now = now_iso8601();

        // Take the vocabulary lock once, and in a fixed order relative to the
        // schedule, so two commands can never deadlock against each other.
        let (items, vocab_warning) = {
            let vocab = self.lock_vocab();
            (
                build_queue(&progress.store, &self.course, vocab.store.entries(), &now),
                vocab.load_error.clone(),
            )
        };

        let due_count = items.len();
        ReviewView {
            items: items.into_iter().take(REVIEW_LIMIT).collect(),
            due_count,
            warning: progress.load_error.clone().or(vocab_warning),
        }
    }
}

/// Read a data-directory override out of a command line.
///
/// Arguments this does not recognise are **ignored rather than rejected**. macOS
/// launches a bundled application with arguments of its own (`-psn_0_12345` and
/// the like), so a parser that refused anything unfamiliar would be unusable from
/// Finder. Only a `--user-dir` that is present but has no usable value is an
/// error, because that is a typo worth stopping for: quietly falling back to the
/// default would scatter a testing session's data into the real application
/// support directory, which is exactly what the flag exists to avoid.
pub fn user_dir_from_args<I, S>(args: I) -> Result<Option<PathBuf>, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let spaced = format!("{USER_DIR_FLAG}=");
    let spaced_alias = format!("{USER_DIR_FLAG_ALIAS}=");
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        let argument = argument.as_ref().to_string_lossy().into_owned();

        let value = if let Some(rest) = argument.strip_prefix(&spaced) {
            Some(rest.to_string())
        } else if let Some(rest) = argument.strip_prefix(&spaced_alias) {
            Some(rest.to_string())
        } else if argument == USER_DIR_FLAG || argument == USER_DIR_FLAG_ALIAS {
            match args.next() {
                // A following flag is a missing value, not a directory named
                // "--verbose". A bare "-" is allowed through, so that the error
                // comes from the filesystem rather than from here.
                Some(next) => {
                    let next = next.as_ref().to_string_lossy().into_owned();
                    if next.starts_with('-') && next != "-" {
                        return Err(USER_DIR_MISSING.to_string());
                    }
                    Some(next)
                }
                None => return Err(USER_DIR_MISSING.to_string()),
            }
        } else {
            None
        };

        if let Some(value) = value {
            let value = value.trim();
            if value.is_empty() {
                return Err(USER_DIR_MISSING.to_string());
            }
            return Ok(Some(PathBuf::from(value)));
        }
    }
    Ok(None)
}

/// The override from either source, with the flag winning over the variable.
///
/// A blank variable is not an override: `HANZI_TUTOR_DATA_DIR=` in a shell is
/// more likely to be an unset variable than an intent to write study data to the
/// current directory.
fn override_dir(cli: Option<PathBuf>, env: Option<&str>) -> Option<PathBuf> {
    if cli.is_some() {
        return cli;
    }
    env.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Where study data should live.
///
/// The `--user-dir` argument wins, then [`DATA_DIR_ENV`], then the platform's
/// application data directory — which on macOS is
/// `~/Library/Application Support/com.hanzitutor.app`, and inside a sandboxed
/// build is that path *in the app's container* rather than in the real home.
/// Deliberately resolved through the platform API rather than assembled from
/// `$HOME`: under the App Sandbox the real home is not writable, and some
/// home-directory APIs still return it, which makes a hand-built `~/.hanzi-tutor`
/// path fail only at save time.
pub fn resolve_data_dir(app: &AppHandle, cli: Option<PathBuf>) -> Result<PathBuf, String> {
    let configured = std::env::var(DATA_DIR_ENV).ok();
    if let Some(dir) = override_dir(cli, configured.as_deref()) {
        return Ok(dir);
    }
    app.path()
        .app_data_dir()
        .map_err(|e| format!("could not locate the application data directory: {e}"))
}

/// Resolve the system voice on a background thread.
///
/// Enumerating voices means running `say -v '?'`, which takes about a second.
/// Doing it here keeps that cost off both the startup path and the first
/// "hear it" click.
fn warm_voice(speaker: Arc<Speaker>) {
    std::thread::spawn(move || match speaker.status() {
        Some(voice) => eprintln!("[speech] using voice {voice}"),
        None => eprintln!(
            "[speech] no Chinese voice installed; pronunciation will be unavailable"
        ),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_command_line_without_the_flag_has_no_override() {
        assert_eq!(user_dir_from_args(args(&[])).unwrap(), None);
        // macOS passes a bundled application arguments of its own. Refusing
        // them would make the app unlaunchable from Finder.
        assert_eq!(user_dir_from_args(args(&["-psn_0_12345"])).unwrap(), None);
        assert_eq!(user_dir_from_args(args(&["--verbose"])).unwrap(), None);
        assert_eq!(user_dir_from_args(args(&["--user-directory"])).unwrap(), None);
    }

    #[test]
    fn the_flag_takes_the_next_argument_whichever_way_it_is_spelled() {
        assert_eq!(
            user_dir_from_args(args(&["--user-dir", "/tmp/one"])).unwrap(),
            Some(PathBuf::from("/tmp/one"))
        );
        assert_eq!(
            user_dir_from_args(args(&["--user_dir", "/tmp/two"])).unwrap(),
            Some(PathBuf::from("/tmp/two"))
        );
        // Position does not matter, and other arguments are left alone.
        assert_eq!(
            user_dir_from_args(args(&["--verbose", "--user-dir", "/tmp/three"])).unwrap(),
            Some(PathBuf::from("/tmp/three"))
        );
    }

    #[test]
    fn the_flag_also_takes_an_equals_form() {
        assert_eq!(
            user_dir_from_args(args(&["--user-dir=/tmp/one"])).unwrap(),
            Some(PathBuf::from("/tmp/one"))
        );
        assert_eq!(
            user_dir_from_args(args(&["--user_dir=/tmp/two"])).unwrap(),
            Some(PathBuf::from("/tmp/two"))
        );
    }

    #[test]
    fn a_path_with_a_space_survives() {
        assert_eq!(
            user_dir_from_args(args(&["--user-dir", "/tmp/a b c"])).unwrap(),
            Some(PathBuf::from("/tmp/a b c"))
        );
    }

    #[test]
    fn a_flag_with_no_usable_value_is_an_error() {
        // Better to stop than to write a session's data somewhere unexpected.
        assert!(user_dir_from_args(args(&["--user-dir"])).is_err());
        assert!(user_dir_from_args(args(&["--user-dir="])).is_err());
        assert!(user_dir_from_args(args(&["--user-dir", "   "])).is_err());
        // A following flag is a missing value, not a directory named "--verbose".
        assert!(user_dir_from_args(args(&["--user-dir", "--verbose"])).is_err());
    }

    #[test]
    fn the_flag_wins_over_the_environment_variable() {
        let flag = Some(PathBuf::from("/tmp/flag"));
        assert_eq!(override_dir(flag.clone(), Some("/tmp/env")), flag);
        assert_eq!(
            override_dir(None, Some("/tmp/env")),
            Some(PathBuf::from("/tmp/env"))
        );
        // A blank variable is an unset variable, not the current directory.
        assert_eq!(override_dir(None, Some("")), None);
        assert_eq!(override_dir(None, Some("   ")), None);
        assert_eq!(override_dir(None, None), None);
    }
}
