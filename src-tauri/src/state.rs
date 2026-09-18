//! Loading the character dataset that the app grades against, and the pieces of
//! application state that outlive a single command.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use hanzi_core::{
    build_lessons, build_queue, now_iso8601, CursorStore, CursorView, Dataset, ProgressStore,
    ProgressView, ReviewView, VocabStore, VocabView,
};
use tauri::{AppHandle, Manager};

use crate::commands::LESSON_SIZE;
use crate::speech::Speaker;

/// The compact artifact produced by `hanzi-core`'s `prepare-data` binary.
///
/// It is embedded rather than read from disk so that development builds and
/// bundled apps behave identically, with no resource-path resolution to get
/// wrong. It is about 13 MB compressed.
const ARTIFACT: &[u8] = include_bytes!("../../crates/hanzi-core/data/hanzi.bin.gz");

/// File names inside the application data directory.
///
/// The schedule and the reader's place in the course are deliberately **separate
/// files**: exploring the course moves the cursor constantly, and one corrupt
/// file must not take the other down with it.
const VOCAB_FILE: &str = "vocabulary.json";
const PROGRESS_FILE: &str = "progress.json";
const CURSOR_FILE: &str = "course-cursor.json";

/// Overrides where study data is stored.
///
/// Useful for portable installs, for keeping study data outside the standard
/// application support folder, and for testing persistence in a specific place.
pub const DATA_DIR_ENV: &str = "HANZI_TUTOR_DATA_DIR";

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
        &self,
        save: impl FnOnce(&S) -> Result<(), E>,
    ) -> Option<String> {
        if let Some(reason) = &self.load_error {
            return Some(reason.clone());
        }
        save(&self.store)
            .err()
            .map(|e| format!("could not save your {}: {e}", self.noun))
    }
}

/// The personal vocabulary list.
pub type VocabState = Persisted<VocabStore>;

impl Persisted<VocabStore> {
    /// Open the list, degrading to an in-memory one with an explanation.
    fn open(path: PathBuf) -> Self {
        const NOUN: &str = "vocabulary list";
        if path.as_os_str().is_empty() {
            return Self::loaded(VocabStore::in_memory(), path, NOUN);
        }
        match VocabStore::open(&path) {
            Ok(store) => Self::loaded(store, path, NOUN),
            Err(error) => Self::failed(VocabStore::in_memory(), path, NOUN, error),
        }
    }

    /// The list as the interface should see it.
    pub fn view(&self) -> VocabView {
        let mut view = self.store.view();
        view.warning = self.load_error.clone();
        view
    }

    /// Persist the list, returning a warning if that was skipped or failed.
    pub fn save(&self) -> Option<String> {
        self.save_with(VocabStore::save)
    }
}

/// Per-character practice history and the review schedule.
pub type ProgressState = Persisted<ProgressStore>;

impl Persisted<ProgressStore> {
    fn open(path: PathBuf) -> Self {
        const NOUN: &str = "practice progress";
        if path.as_os_str().is_empty() {
            return Self::loaded(ProgressStore::in_memory(), path, NOUN);
        }
        match ProgressStore::open(&path) {
            Ok(store) => Self::loaded(store, path, NOUN),
            Err(error) => Self::failed(ProgressStore::in_memory(), path, NOUN, error),
        }
    }

    /// The schedule as the interface should see it.
    pub fn view(&self) -> ProgressView {
        let mut view = self.store.view();
        view.warning = self.load_error.clone();
        view
    }

    /// Persist the schedule, returning a warning if that was skipped or failed.
    pub fn save(&self) -> Option<String> {
        self.save_with(ProgressStore::save)
    }
}

/// Where the reader was in the course.
pub type CursorState = Persisted<CursorStore>;

impl Persisted<CursorStore> {
    fn open(path: PathBuf) -> Self {
        const NOUN: &str = "place in the course";
        if path.as_os_str().is_empty() {
            return Self::loaded(CursorStore::in_memory(), path, NOUN);
        }
        match CursorStore::open(&path) {
            Ok(store) => Self::loaded(store, path, NOUN),
            Err(error) => Self::failed(CursorStore::in_memory(), path, NOUN, error),
        }
    }

    /// The cursor as the interface should see it.
    pub fn view(&self) -> CursorView {
        let mut view = self.store.view();
        view.warning = self.load_error.clone();
        view
    }

    /// Persist the cursor, returning a warning if that was skipped or failed.
    pub fn save(&self) -> Option<String> {
        self.save_with(CursorStore::save)
    }
}

/// State held for the lifetime of the app and shared by all commands.
pub struct AppState {
    pub dataset: Dataset,
    /// Shared with a warm-up thread, so it is behind an `Arc`.
    pub speech: Arc<Speaker>,
    /// Behind a mutex because every mutation is read-modify-write and must be
    /// persisted as a whole document.
    pub vocab: Mutex<VocabState>,
    pub progress: Mutex<ProgressState>,
    pub cursor: Mutex<CursorState>,
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

        let path = |name: &str| match &data_dir {
            Some(dir) => dir.join(name),
            None => PathBuf::new(),
        };

        Ok(Self {
            dataset,
            speech,
            course,
            vocab: Mutex::new(VocabState::open(path(VOCAB_FILE))),
            progress: Mutex::new(ProgressState::open(path(PROGRESS_FILE))),
            cursor: Mutex::new(CursorState::open(path(CURSOR_FILE))),
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

/// Where study data should live.
///
/// Honours [`DATA_DIR_ENV`] first, so the location can be overridden without
/// touching the app, then falls back to the platform's application data
/// directory.
pub fn resolve_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var(DATA_DIR_ENV) {
        let dir = dir.trim();
        if !dir.is_empty() {
            return Ok(PathBuf::from(dir));
        }
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
