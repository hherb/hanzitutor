//! Loading the character dataset that the app grades against, and the pieces of
//! application state that outlive a single command.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use hanzi_core::{Dataset, VocabStore, VocabView};
use tauri::{AppHandle, Manager};

use crate::speech::Speaker;

/// The compact artifact produced by `hanzi-core`'s `prepare-data` binary.
///
/// It is embedded rather than read from disk so that development builds and
/// bundled apps behave identically, with no resource-path resolution to get
/// wrong. It is about 13 MB compressed.
const ARTIFACT: &[u8] = include_bytes!("../../crates/hanzi-core/data/hanzi.bin.gz");

/// Name of the vocabulary file inside the application data directory.
const VOCAB_FILE: &str = "vocabulary.json";

/// Overrides where the vocabulary list is stored.
///
/// Useful for portable installs, for keeping study data outside the standard
/// application support folder, and for testing persistence in a specific place.
pub const DATA_DIR_ENV: &str = "HANZI_TUTOR_DATA_DIR";

/// The personal vocabulary list, plus what went wrong loading it.
pub struct VocabState {
    pub store: VocabStore,
    pub path: PathBuf,
    /// Why the saved list could not be opened. While this is set, saving is
    /// **refused**, so that a corrupt file is never silently replaced by an
    /// empty list — losing someone's study notes to a parse error would be far
    /// worse than refusing to write.
    pub load_error: Option<String>,
}

impl VocabState {
    /// The list as the interface should see it.
    pub fn view(&self) -> VocabView {
        let mut view = self.store.view();
        view.warning = self.load_error.clone();
        view
    }

    /// Persist the list, returning a warning if that was skipped or failed.
    pub fn save(&self) -> Option<String> {
        if let Some(reason) = &self.load_error {
            return Some(reason.clone());
        }
        self.store
            .save()
            .err()
            .map(|e| format!("could not save your vocabulary list: {e}"))
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
}

impl AppState {
    /// Decode the embedded dataset, open the vocabulary list and start the
    /// pronunciation warm-up.
    ///
    /// `data_dir` is where the vocabulary file lives; `None` keeps it in memory,
    /// which is what the tests use.
    pub fn load(data_dir: Option<PathBuf>) -> Result<Self, String> {
        let dataset = Dataset::from_gzip_bytes(ARTIFACT)
            .map_err(|e| format!("could not read the embedded character dataset: {e}"))?;

        let speech = Arc::new(Speaker::default());
        warm_voice(Arc::clone(&speech));

        let path = match data_dir {
            Some(dir) => dir.join(VOCAB_FILE),
            None => PathBuf::new(),
        };
        let vocab = Mutex::new(open_vocabulary(path));

        Ok(Self {
            dataset,
            speech,
            vocab,
        })
    }

    /// Lock the vocabulary list, tolerating a poisoned mutex: a panic while
    /// holding it cannot leave the document in a state that matters.
    pub fn lock_vocab(&self) -> MutexGuard<'_, VocabState> {
        self.vocab.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Open the list, degrading to an in-memory one with an explanation.
fn open_vocabulary(path: PathBuf) -> VocabState {
    if path.as_os_str().is_empty() {
        return VocabState {
            store: VocabStore::in_memory(),
            path,
            load_error: None,
        };
    }
    match VocabStore::open(&path) {
        Ok(store) => VocabState {
            store,
            path,
            load_error: None,
        },
        Err(error) => VocabState {
            store: VocabStore::in_memory(),
            path: path.clone(),
            load_error: Some(format!(
                "Your saved vocabulary list at {} could not be read: {error}. \
                 New entries are kept in memory only and will not be saved until \
                 this is resolved — move or fix that file, then restart.",
                path.display()
            )),
        },
    }
}

/// Where the vocabulary list should live.
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
