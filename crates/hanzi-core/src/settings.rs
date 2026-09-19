//! The learner's settings.
//!
//! Not study data: nothing here is graded, scheduled or reviewed, and losing it
//! costs a preference rather than work. It lives in this crate anyway because it
//! is *persisted*, and every persisted thing in this app follows one shape — a
//! small typed document, a sink that decides where it lives (see
//! [`SettingsSink`], and `crates/hanzi-store` for the database behind it), and a
//! `Persisted` wrapper in the app that refuses to write over a document it could
//! not read.
//!
//! ## Why a struct rather than a flag on the app state
//!
//! A settings *dialog* is coming, and the expensive part of one is not the
//! controls: it is having somewhere to put the values, a way to read them before
//! the first frame, and a defined answer for what happens when a stored value is
//! missing, unknown or unreadable. That is this module. Adding a preference is
//! then a field here plus a control there — and a field the store does not know
//! about needs no database change at all, because the rows are keyed by name.
//!
//! ## Every field is optional, and that is the point
//!
//! `None` means *the learner has not chosen*, which is not the same as `false`.
//! The interface resolves a missing choice from the device — a trackpad or a
//! mouse wants click-to-draw, a stylus or a finger wants to drag — and only
//! writes a value once the learner has actually flipped the switch. Collapsing
//! that into a plain `bool` default would make "not chosen yet" indistinguishable
//! from "chosen off", and the device default would then be impossible to offer.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Format version written into the settings document.
pub const FORMAT_VERSION: u32 = 1;

/// Why a settings operation failed.
#[derive(Debug)]
pub enum SettingsError {
    /// The document could not be read or written.
    Io(String),
    /// The document is not valid JSON, or does not match the schema.
    Malformed(String),
    /// The document was written by a newer version of the app.
    UnsupportedVersion(u32),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(why) => write!(f, "{why}"),
            Self::Malformed(why) => write!(f, "it is not valid JSON: {why}"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "it was written by a newer version of the app \
                 (format {v}, this build understands {FORMAT_VERSION})"
            ),
        }
    }
}

impl std::error::Error for SettingsError {}

/// What the learner has chosen, if anything.
///
/// Absent fields are *unset*, not defaulted — see the module note. `Default` is
/// therefore every field `None`, which is exactly a fresh install.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// How a stroke is drawn: `Some(true)` click to start and click to finish,
    /// `Some(false)` press and drag, `None` let the device decide.
    pub click_to_draw: Option<bool>,
}

/// Where settings are kept.
///
/// The JSON file at [`SettingsStore::path`] is the built-in backing; this is the
/// seam for the database, which is what the app actually uses.
pub trait SettingsSink: std::fmt::Debug + Send {
    fn load(&self) -> Result<Settings, SettingsError>;
    fn save(&mut self, settings: &Settings) -> Result<(), SettingsError>;
}

/// The persisted document: a version, and the settings.
///
/// The version exists so a document written by a newer build is refused rather
/// than read for fields this build does not know — the same rule the study
/// documents follow, for the same reason: silently ignoring a setting someone
/// changed is worse than saying the file is from a newer app.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Document {
    version: u32,
    #[serde(flatten)]
    settings: Settings,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            settings: Settings::default(),
        }
    }
}

/// The interface's view of the settings, with the save-failure warning the other
/// views carry.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    #[serde(flatten)]
    pub settings: Settings,
    #[serde(default)]
    pub warning: Option<String>,
}

impl SettingsView {
    /// How a stroke should be drawn, or `None` if nobody has chosen.
    ///
    /// The settings are flattened into this view so that adding a preference
    /// touches one struct rather than two; this is the accessor that keeps
    /// `view.click_to_draw()` reading better than `view.settings.click_to_draw`.
    pub fn click_to_draw(&self) -> Option<bool> {
        self.settings.click_to_draw
    }
}

/// The learner's settings, backed by a JSON file or by a [`SettingsSink`].
#[derive(Debug)]
pub struct SettingsStore {
    path: PathBuf,
    settings: Settings,
    sink: Option<Box<dyn SettingsSink>>,
    /// Set when a change has been made but not yet written, so a save that has
    /// nothing to do can say so rather than rewriting the file.
    dirty: bool,
}

impl SettingsStore {
    /// Open the settings at `path`, starting from nothing chosen if it is absent.
    ///
    /// A missing file is a fresh install. A *corrupt* file is a hard error: the
    /// app reports it and refuses to save rather than replacing whatever is in it.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, SettingsError> {
        let path = path.into();
        let document = read_document(&path)?;
        Ok(Self {
            path,
            settings: document.settings,
            sink: None,
            dirty: false,
        })
    }

    /// Open settings kept by `sink` rather than by a JSON file.
    pub fn open_with(sink: Box<dyn SettingsSink>) -> Result<Self, SettingsError> {
        let settings = sink.load()?;
        Ok(Self {
            path: PathBuf::new(),
            settings,
            sink: Some(sink),
            dirty: false,
        })
    }

    /// Settings in memory, with no file behind them. Used by tests.
    pub fn in_memory() -> Self {
        Self {
            path: PathBuf::new(),
            settings: Settings::default(),
            sink: None,
            dirty: false,
        }
    }

    /// The file these settings are kept in, or an empty path when a sink holds
    /// them.
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// How a stroke should be drawn, or `None` if the learner has not chosen.
    pub fn click_to_draw(&self) -> Option<bool> {
        self.settings.click_to_draw
    }

    /// Choose how a stroke is drawn, or pass `None` to go back to the device's
    /// own default. Returns true when that was a change.
    pub fn set_click_to_draw(&mut self, value: Option<bool>) -> bool {
        if self.settings.click_to_draw == value {
            return false;
        }
        self.settings.click_to_draw = value;
        self.dirty = true;
        true
    }

    /// Settings as the interface sees them.
    pub fn view(&self) -> SettingsView {
        SettingsView {
            settings: self.settings.clone(),
            warning: None,
        }
    }

    /// Persist the settings, if anything changed.
    ///
    /// Through a sink the whole (tiny) document is handed over; without one the
    /// JSON file is rewritten atomically by [`save_document`]. A save with
    /// nothing to write is not a write: the app saves after every toggle, and
    /// this is also what keeps a fresh install — where nothing has been chosen —
    /// from leaving a document behind that records a decision nobody made.
    pub fn save(&mut self) -> Result<(), SettingsError> {
        if !self.dirty {
            return Ok(());
        }
        match self.sink.as_mut() {
            Some(sink) => sink.save(&self.settings)?,
            None => save_document(
                &self.path,
                &Document {
                    version: FORMAT_VERSION,
                    settings: self.settings.clone(),
                },
            )?,
        }
        self.dirty = false;
        Ok(())
    }
}

/// Read and validate a document, treating absence as an unset first run.
fn read_document(path: &Path) -> Result<Document, SettingsError> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let parsed: Document = serde_json::from_str(&text)
                .map_err(|e| SettingsError::Malformed(e.to_string()))?;
            if parsed.version > FORMAT_VERSION {
                return Err(SettingsError::UnsupportedVersion(parsed.version));
            }
            Ok(parsed)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Document::default()),
        Err(e) => Err(SettingsError::Io(format!("{}: {e}", path.display()))),
    }
}

/// Write the document atomically: temporary file first, then rename.
fn save_document(path: &Path, document: &Document) -> Result<(), SettingsError> {
    if path.as_os_str().is_empty() {
        return Ok(()); // in-memory store
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| SettingsError::Io(format!("{}: {e}", parent.display())))?;
        }
    }
    let json = serde_json::to_string_pretty(document)
        .map_err(|e| SettingsError::Io(e.to_string()))?;

    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, json)
        .map_err(|e| SettingsError::Io(format!("{}: {e}", temporary.display())))?;
    fs::rename(&temporary, path).map_err(|e| SettingsError::Io(format!("{}: {e}", path.display())))?;
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
        std::env::temp_dir().join(format!("hanzi-settings-{name}-{unique}.json"))
    }

    #[test]
    fn a_fresh_install_has_chosen_nothing() {
        let path = temp_path("fresh");
        let store = SettingsStore::open(&path).unwrap();
        assert_eq!(store.click_to_draw(), None);
        assert!(store.view().warning.is_none());
        // Nothing chosen means nothing to write, and no file to leave behind.
        let mut store = store;
        store.save().unwrap();
        assert!(!path.exists(), "an unset preference is not a document");
    }

    #[test]
    fn an_unset_preference_is_not_the_same_as_off() {
        // The whole reason the field is optional: "off" is a choice the learner
        // made, and it has to survive a restart as such.
        let path = temp_path("false");
        let mut store = SettingsStore::open(&path).unwrap();
        assert!(store.set_click_to_draw(Some(false)));
        store.save().unwrap();

        let reopened = SettingsStore::open(&path).unwrap();
        assert_eq!(reopened.click_to_draw(), Some(false));

        // Writing `null` back records the absence of a choice, not a false one.
        let mut store = SettingsStore::open(&path).unwrap();
        assert!(store.set_click_to_draw(None));
        store.save().unwrap();
        let reopened = SettingsStore::open(&path).unwrap();
        assert_eq!(reopened.click_to_draw(), None);
    }

    #[test]
    fn setting_the_same_value_is_not_a_change() {
        let mut store = SettingsStore::in_memory();
        assert!(store.set_click_to_draw(Some(true)));
        assert!(!store.set_click_to_draw(Some(true)));
        assert!(store.set_click_to_draw(Some(false)));
        assert!(store.set_click_to_draw(None));
        assert!(!store.set_click_to_draw(None));
    }

    #[test]
    fn settings_round_trip_through_the_file() {
        let path = temp_path("round-trip");
        {
            let mut store = SettingsStore::open(&path).unwrap();
            store.set_click_to_draw(Some(true));
            store.save().unwrap();
        }
        let reopened = SettingsStore::open(&path).unwrap();
        assert_eq!(reopened.click_to_draw(), Some(true));

        // The document carries a version, and stays readable as JSON.
        let text = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["version"], serde_json::json!(FORMAT_VERSION));
        assert_eq!(parsed["clickToDraw"], serde_json::json!(true));

        fs::remove_file(&path).ok();
    }

    #[test]
    fn a_corrupt_file_is_an_error_rather_than_a_silent_reset() {
        let path = temp_path("corrupt");
        fs::write(&path, "{ not json").unwrap();
        let error = SettingsStore::open(&path).unwrap_err();
        assert!(matches!(error, SettingsError::Malformed(_)), "{error}");
        // And it is left exactly as it was.
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn a_future_version_is_refused() {
        let path = temp_path("future");
        fs::write(&path, format!(r#"{{"version":{}}}"#, FORMAT_VERSION + 1)).unwrap();
        assert!(matches!(
            SettingsStore::open(&path).unwrap_err(),
            SettingsError::UnsupportedVersion(_)
        ));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn unknown_fields_do_not_break_a_document() {
        // A settings file written by a build that knew more fields than this one
        // still opens: the fields it does not know are ignored, and the ones it
        // does are read. (A *newer version* is refused; an extra key is not.)
        let path = temp_path("unknown-field");
        fs::write(
            &path,
            format!(
                r#"{{"version":{FORMAT_VERSION},"clickToDraw":true,"futureThing":42}}"#
            ),
        )
        .unwrap();
        let store = SettingsStore::open(&path).unwrap();
        assert_eq!(store.click_to_draw(), Some(true));
        fs::remove_file(&path).ok();
    }
}
