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
//! The settings *screen* is built (`src/lib/SettingsPanel.svelte`), and the
//! expensive part of one was never the controls: it is having somewhere to put
//! the values, a way to read them before the first frame, and a defined answer
//! for what happens when a stored value is missing, unknown or unreadable. That
//! is this module. Adding a preference is then a field here plus a control there
//! — and a field the store does not know about needs no database change at all,
//! because the rows are keyed by name. [`Settings::intro_seen`] is the one field
//! with no control of its own: the app writes it when the introduction is
//! dismissed, and the settings screen only replays that introduction.
//!
//! ## Which fields are optional, and why not all of them
//!
//! `None` means *the learner has not chosen*, which is not the same as `false`.
//! That only earns its keep for a preference the interface can resolve from the
//! **device** — a trackpad or a mouse wants click-to-draw, a stylus or a finger
//! wants to drag — because the absence then has a third, observable meaning, and
//! the app can follow the device until the learner actually flips the switch.
//!
//! A preference with no device signal ([`Pace`], [`BoardSize`]) is a plain value
//! with a `Default` instead. Making those optional too would add a state nobody
//! can tell apart from the default and would push a `?? "normal"` into every
//! caller. What is *stored* is the same either way: no row at all, which is what
//! a fresh install looks like.

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

/// How fast the stroke-order animation runs.
///
/// A closed choice rather than a number of milliseconds: what a learner wants to
/// say is "slower, please", and three bands are what the settings screen can
/// offer honestly.
///
/// **The value is a name, not a scale.** What each name does to the animation is
/// the *interface's* business — `paceScale` in `src/App.svelte` is the one place
/// that turns a name into a factor, because that is where the animation's bounds
/// live and a speed is meaningless without them. Do not add a `scale()` here: a
/// second copy of the number in Rust would drift from the one the app uses, and
/// nothing would fail.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Pace {
    /// Slower: for a learner meeting stroke order for the first time.
    Slow,
    /// The pace the animation has always had.
    Normal,
    /// Faster: for revising a character already known.
    Fast,
}

impl Pace {
    /// Every pace, in the order the settings screen offers them.
    ///
    /// The list lives here rather than in the interface so that adding a pace
    /// cannot leave the control out of step with the enum — the same reason
    /// [`BoardSize`] has one.
    pub const ALL: [Pace; 3] = [Pace::Slow, Pace::Normal, Pace::Fast];
}

impl Default for Pace {
    /// Normal, which is the pace the animation shipped with. Unlike the
    /// device-dependent preferences this one is a *value* rather than an
    /// `Option`: there is no device signal to read a pace from, so "nobody has
    /// chosen" would be indistinguishable in behaviour from "chose normal", and
    /// the stored absence is already carried by the row being missing.
    fn default() -> Self {
        Self::Normal
    }
}

/// How large the practice board is drawn.
///
/// A scale rather than a pixel size: the board sizes itself from its container
/// so that it fits the window on any screen, and what a learner actually wants
/// to change is how much of that space it takes. As with [`Pace`], the *name*
/// travels and the fraction that implements it lives in the interface
/// (`boardFraction` in `src/App.svelte`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BoardSize {
    /// Smallest, for a window with little room to spare.
    Compact,
    /// The size the board has always been.
    Normal,
    /// Largest, for a big screen or a stylus.
    Large,
}

impl BoardSize {
    /// Every size, in the order the settings screen offers them.
    pub const ALL: [BoardSize; 3] = [BoardSize::Compact, BoardSize::Normal, BoardSize::Large];
}

impl Default for BoardSize {
    /// Normal, for the reason [`Pace::default`] gives: there is no device signal
    /// for this either.
    fn default() -> Self {
        Self::Normal
    }
}

/// The preferences whose absence has a device-dependent answer.
///
/// Only these are `Option`: the interface can resolve a missing one *from the
/// machine it is running on*, which is what makes storing the absence worth
/// doing. A preference with no device signal lives as a plain value with a
/// `Default` — the row is missing either way, and pretending otherwise would add
/// a third state nobody can observe.
///
/// Absent fields are *unset*, not defaulted — see the module note. `Default` is
/// therefore every field `None`, which is exactly a fresh install.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// How a stroke is drawn: `Some(true)` click to start and click to finish,
    /// `Some(false)` press and drag, `None` let the device decide.
    pub click_to_draw: Option<bool>,
    /// The pronunciation voice to use, by name. Unknown names are matched
    /// against the installed voices ignoring the locale qualifier macOS appends
    /// — `Tingting (Chinese (China mainland))` is the same voice as `Tingting` —
    /// and a name this machine does not have falls back to the automatic choice
    /// rather than failing, because a preference set on one machine must not
    /// break pronunciation on another. `None` is the automatic choice.
    pub voice: Option<String>,
    /// How fast the stroke-order animation runs when nobody has chosen
    /// differently. Stored as a value, not an absence — see [`Pace::default`].
    #[serde(default)]
    pub animation_pace: Pace,
    /// How large the board is drawn — see [`BoardSize::default`].
    #[serde(default)]
    pub board_size: BoardSize,
    /// Whether characters are shown coloured by the tone they are read with.
    ///
    /// A memory aid rather than a study setting: nothing is graded differently,
    /// and a learner either finds a colour on every glyph helpful or finds it
    /// noise. **Off by default**, because it changes how the whole interface
    /// looks and nobody asked for it on the first run — unlike the pace or the
    /// board size, there is no sensible answer to guess.
    ///
    /// A plain `bool` rather than an `Option`, for the reason [`Pace::default`]
    /// gives: there is no device signal to resolve an unchosen value from, so a
    /// missing row already means "off" and storing `false` would add nothing.
    #[serde(default)]
    pub tone_colours: bool,
    /// Whether the introduction has been read and dismissed.
    ///
    /// The one field here the *app* writes rather than a control on the settings
    /// screen: the interface shows the introduction once, on the first run that
    /// has not seen it, and this is how it knows. It earns its place in this
    /// document because it is persisted, per-device and loss-tolerant — losing
    /// it costs a learner one dismissal, not any work — which is exactly the
    /// category the module note draws.
    ///
    /// A plain `bool` rather than an `Option`, like [`Pace`] and [`BoardSize`]:
    /// there is no device signal to resolve an unchosen value from, so the two
    /// states are "not seen" and "seen" and the stored absence means the former.
    /// **Settings do not sync**, so this is per-device on purpose: the
    /// introduction is about the screen in front of you, and a phone should not
    /// have the wizard a laptop already dismissed.
    #[serde(default)]
    pub intro_seen: bool,
    /// The app version whose "what's new" pages have been read, if any.
    ///
    /// The upgrade half of the same idea: an installation that has run before is
    /// shown what changed rather than the introduction, once per version. The
    /// stored value is the **version string** rather than a flag, because "have
    /// you read the notes for *this* release" is the question, and a boolean
    /// would either show every release's notes again or none of them. `None`
    /// means no notes have ever been read here — which is what every
    /// installation upgrading from a build that had none has, and is exactly why
    /// it is `Option` and not a `bool` defaulting to some version.
    ///
    /// Deliberately a version and not a timestamp: the app ships a fixed set of
    /// pages, so the only thing worth recording is which release they describe.
    #[serde(default)]
    pub whats_new_seen: Option<String>,
}

/// Where settings are kept.
///
/// The JSON file at [`SettingsStore::path`] is the built-in backing; this is the
/// seam for the database, which is what the app actually uses.
pub trait SettingsSink: std::fmt::Debug + Send {
    fn load(&self) -> Result<Settings, SettingsError>;
    fn save(&mut self, settings: &Settings) -> Result<(), SettingsError>;
}

/// One change to the preferences, as a screen sends it.
///
/// ## Why a struct rather than one argument per preference
///
/// This used to be six arguments on the command and six on the state method, one
/// per preference, and adding [`Settings::tone_colours`] made eight — past the
/// point where a signature is readable, and at a ceiling that every future
/// preference would hit again. A document with a field per preference is what the
/// thing actually is, and this is that document's `Deserialize`.
///
/// ## What the shapes mean on the wire
///
/// Every field is `Option` and **absent means leave that preference alone**: the
/// screen sends only the control the learner touched, so changing the voice must
/// not reset the board size on the way past. Clearing is spelled per preference,
/// because `None` is already taken:
///
/// * `click_to_draw` cannot be cleared here at all — going back to the device's
///   own answer is `AppState::clear_click_to_draw`, since "leave it alone" and
///   "forget my choice" are different instructions.
/// * `voice` is the **empty string** to go back to the automatic voice. A voice
///   is never legitimately nameless, so the empty string is free to mean this.
/// * `animation_pace` and `board_size` are closed sets, so every value a caller
///   can send is a choice.
/// * `tone_colours`, `intro_seen` and `whats_new_seen` are plain values: there is
///   no third state for any of them to be returned to.
///
/// ## `deny_unknown_fields`
///
/// Deliberately set, and it is what keeps the guarantee the flat arguments used
/// to have: a preference this build does not know is **rejected** rather than
/// silently ignored, so a screen and a binary that disagree about the names are
/// told so instead of appearing to save something that goes nowhere. It is
/// checked against the interface's own `SettingsPatch` by a test.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct SettingsPatch {
    /// Choose how a stroke is drawn. `None` leaves the choice as it is.
    pub click_to_draw: Option<bool>,
    /// Choose the pronunciation voice by name; `""` goes back to the automatic
    /// choice. `None` leaves the choice as it is.
    pub voice: Option<String>,
    /// Choose how fast the stroke-order animation runs.
    pub animation_pace: Option<Pace>,
    /// Choose how large the board is drawn.
    pub board_size: Option<BoardSize>,
    /// Turn colouring by tone on or off.
    pub tone_colours: Option<bool>,
    /// Record that the introduction has been dismissed.
    pub intro_seen: Option<bool>,
    /// Record whose "what's new" pages have been read.
    pub whats_new_seen: Option<String>,
}

impl SettingsPatch {
    /// True when the patch asks for nothing at all.
    ///
    /// A screen that sends an empty patch is answered without a write, which is
    /// what keeps the "no Save button" rule from turning into a save per frame: an
    /// untouched control is not a change.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
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

    /// The voice to pronounce with, or `None` for the automatic choice.
    pub fn voice(&self) -> Option<&str> {
        self.settings.voice.as_deref()
    }

    /// How fast the stroke-order animation should run.
    pub fn pace(&self) -> Pace {
        self.settings.animation_pace
    }

    /// How large the board should be drawn.
    pub fn board_size(&self) -> BoardSize {
        self.settings.board_size
    }

    /// Whether characters should be coloured by the tone they are read with.
    pub fn tone_colours(&self) -> bool {
        self.settings.tone_colours
    }

    /// Whether the introduction has been read and dismissed.
    pub fn intro_seen(&self) -> bool {
        self.settings.intro_seen
    }

    /// The version whose "what's new" pages have been read, if any.
    pub fn whats_new_seen(&self) -> Option<&str> {
        self.settings.whats_new_seen.as_deref()
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

    /// Choose the voice to pronounce with, or `None` for the automatic choice.
    ///
    /// An empty (or all-whitespace) name is stored as `None`, so a field the
    /// learner has cleared goes back to the automatic choice rather than being
    /// persisted as a voice called "".
    pub fn set_voice(&mut self, value: Option<&str>) -> bool {
        let value = value.map(str::trim).filter(|name| !name.is_empty());
        let value = value.map(str::to_string);
        if self.settings.voice == value {
            return false;
        }
        self.settings.voice = value;
        self.dirty = true;
        true
    }

    /// Choose how fast the stroke-order animation runs.
    pub fn set_animation_pace(&mut self, value: Pace) -> bool {
        if self.settings.animation_pace == value {
            return false;
        }
        self.settings.animation_pace = value;
        self.dirty = true;
        true
    }

    /// Choose how large the board is drawn.
    pub fn set_board_size(&mut self, value: BoardSize) -> bool {
        if self.settings.board_size == value {
            return false;
        }
        self.settings.board_size = value;
        self.dirty = true;
        true
    }

    /// Turn colouring by tone on or off.
    pub fn set_tone_colours(&mut self, value: bool) -> bool {
        if self.settings.tone_colours == value {
            return false;
        }
        self.settings.tone_colours = value;
        self.dirty = true;
        true
    }

    /// Record that the introduction has (or has not) been read.
    ///
    /// Written by the app when the introduction is dismissed, not by a control:
    /// the settings screen's "show it again" button replays it in place and
    /// leaves this alone, so reading the introduction twice does not depend on
    /// clearing the record first. Passing `false` is the route back to a first
    /// run, which is what a test wants and no screen offers.
    pub fn set_intro_seen(&mut self, value: bool) -> bool {
        if self.settings.intro_seen == value {
            return false;
        }
        self.settings.intro_seen = value;
        self.dirty = true;
        true
    }

    /// Record which release's "what's new" pages have been read.
    ///
    /// An empty (or all-whitespace) version is stored as `None`, the same way a
    /// cleared voice goes back to the automatic choice: a version nobody can name
    /// is not a release whose notes have been read. Unlike [`Self::set_intro_seen`]
    /// this cannot be set to a default — there is no meaningful "unread version",
    /// only the absence of one — so the way back to "never shown" is `None`, which
    /// is what a test uses and no screen offers.
    pub fn set_whats_new_seen(&mut self, value: Option<&str>) -> bool {
        let value = value.map(str::trim).filter(|v| !v.is_empty());
        let value = value.map(str::to_string);
        if self.settings.whats_new_seen == value {
            return false;
        }
        self.settings.whats_new_seen = value;
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
    fn a_patch_carries_only_what_was_touched() {
        // The wire shape the settings screen sends, and the whole reason every
        // field is optional: absent means "leave this preference alone", so a
        // voice change cannot reset the board size on the way past.
        let patch: SettingsPatch = serde_json::from_str(r#"{"voice":"Meijia"}"#).unwrap();
        assert_eq!(patch.voice.as_deref(), Some("Meijia"));
        assert_eq!(patch.click_to_draw, None);
        assert_eq!(patch.animation_pace, None);
        assert_eq!(patch.board_size, None);
        assert_eq!(patch.tone_colours, None);
        assert_eq!(patch.intro_seen, None);
        assert_eq!(patch.whats_new_seen, None);
        assert!(!patch.is_empty());

        let nothing: SettingsPatch = serde_json::from_str("{}").unwrap();
        assert!(nothing.is_empty(), "no control touched is not a change");

        // An empty voice is how the automatic choice is asked for, and it is a
        // real instruction rather than an absence.
        let automatic: SettingsPatch = serde_json::from_str(r#"{"voice":""}"#).unwrap();
        assert_eq!(automatic.voice.as_deref(), Some(""));
        assert!(!automatic.is_empty());
    }

    #[test]
    fn a_patch_names_a_preference_this_build_does_not_know() {
        // `deny_unknown_fields` is deliberate: a screen and a binary that
        // disagree about a preference's name are told so, rather than appearing
        // to save something that goes nowhere.
        let error = serde_json::from_str::<SettingsPatch>(r#"{"toneColors":true}"#).unwrap_err();
        assert!(
            error.to_string().contains("toneColors"),
            "the unknown name has to be in the message: {error}"
        );

        // Every field the interface has is one this build accepts, spelled the
        // way the interface spells it. A rename that reached only one side would
        // otherwise be silent.
        let all: SettingsPatch = serde_json::from_str(
            r#"{"clickToDraw":true,"voice":"Tingting","animationPace":"fast",
                "boardSize":"large","toneColours":true,"introSeen":true,
                "whatsNewSeen":"0.5.6"}"#,
        )
        .unwrap();
        assert_eq!(all.click_to_draw, Some(true));
        assert_eq!(all.voice.as_deref(), Some("Tingting"));
        assert_eq!(all.animation_pace, Some(Pace::Fast));
        assert_eq!(all.board_size, Some(BoardSize::Large));
        assert_eq!(all.tone_colours, Some(true));
        assert_eq!(all.intro_seen, Some(true));
        assert_eq!(all.whats_new_seen.as_deref(), Some("0.5.6"));

        // And it serialises back under the same names, which is what lets the
        // interface's own type be checked against this one.
        let json = serde_json::to_value(&all).unwrap();
        assert_eq!(json["toneColours"], serde_json::json!(true));
        assert_eq!(json["animationPace"], serde_json::json!("fast"));
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
    fn a_preference_with_no_device_signal_defaults_rather_than_going_unset() {
        // The distinction the module note draws: a *device-dependent* preference
        // is an `Option`, so "nobody has chosen" survives storage. A pace and a
        // board size have no device to read, so their absence has one observable
        // meaning and the value is a plain enum.
        let store = SettingsStore::in_memory();
        assert_eq!(store.settings().animation_pace, Pace::Normal);
        assert_eq!(store.settings().board_size, BoardSize::Normal);
        assert_eq!(store.settings().voice, None);
        assert!(!store.settings().intro_seen);
        assert_eq!(store.settings().whats_new_seen, None);
        assert!(!store.settings().tone_colours, "colour is off until asked for");

        // And the same on a fresh document that carries none of the new keys.
        let path = temp_path("missing-new-keys");
        fs::write(&path, format!(r#"{{"version":{FORMAT_VERSION}}}"#)).unwrap();
        let store = SettingsStore::open(&path).unwrap();
        assert_eq!(store.view().pace(), Pace::Normal);
        assert_eq!(store.view().board_size(), BoardSize::Normal);
        assert_eq!(store.view().voice(), None);
        assert!(!store.view().intro_seen());
        assert_eq!(store.view().whats_new_seen(), None);
        assert!(!store.view().tone_colours());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn colouring_by_tone_is_off_until_it_is_asked_for() {
        // The one preference that changes how the whole interface looks. Nothing
        // chosen means nothing stored, so a learner who never touches it leaves no
        // row behind — the same rule the pace and the board size follow.
        let path = temp_path("tone-colours");
        let mut store = SettingsStore::open(&path).unwrap();
        assert!(!store.view().tone_colours());
        assert!(!store.set_tone_colours(false), "already the default");
        store.save().unwrap();
        assert!(!path.exists(), "nothing chosen is not a document");

        assert!(store.set_tone_colours(true));
        assert!(!store.set_tone_colours(true), "already on");
        store.save().unwrap();

        let reopened = SettingsStore::open(&path).unwrap();
        assert!(reopened.view().tone_colours());
        let text = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["toneColours"], serde_json::json!(true));

        fs::remove_file(&path).ok();
    }

    #[test]
    fn the_introduction_is_not_read_until_it_is_dismissed() {
        // The one thing the interface decides its first screen on. A fresh
        // install has not seen it, and dismissing it is a change worth writing;
        // dismissing it twice is not a change at all.
        let path = temp_path("intro");
        let mut store = SettingsStore::open(&path).unwrap();
        assert!(!store.view().intro_seen(), "a fresh install has seen nothing");
        assert!(store.set_intro_seen(true));
        assert!(!store.set_intro_seen(true), "already dismissed");
        store.save().unwrap();

        let reopened = SettingsStore::open(&path).unwrap();
        assert!(reopened.view().intro_seen());
        assert_eq!(
            serde_json::to_value(reopened.view()).unwrap()["introSeen"],
            serde_json::json!(true)
        );

        fs::remove_file(&path).ok();
    }

    #[test]
    fn an_unread_introduction_leaves_no_row_behind() {
        // The same rule the pace and the board size follow: the default is what
        // a missing row already means, so a document that records nothing must
        // not be written at all.
        let path = temp_path("intro-unread");
        let mut store = SettingsStore::open(&path).unwrap();
        assert!(!store.set_intro_seen(false), "already the default");
        store.save().unwrap();
        assert!(!path.exists(), "nothing chosen is not a document");
    }

    #[test]
    fn the_release_notes_are_remembered_by_version() {
        // "Have you read the notes for *this* release" is the question, so the
        // value is the version and not a flag: a boolean would either show every
        // release's notes again or none of them ever again.
        let path = temp_path("whats-new");
        let mut store = SettingsStore::open(&path).unwrap();
        assert_eq!(store.view().whats_new_seen(), None, "nothing read yet");

        assert!(store.set_whats_new_seen(Some("0.5.6")));
        assert!(!store.set_whats_new_seen(Some("0.5.6")), "already read");
        assert!(store.set_whats_new_seen(Some("0.5.7")), "a later release is news again");
        store.save().unwrap();

        let reopened = SettingsStore::open(&path).unwrap();
        assert_eq!(reopened.view().whats_new_seen(), Some("0.5.7"));
        assert_eq!(
            serde_json::to_value(reopened.view()).unwrap()["whatsNewSeen"],
            serde_json::json!("0.5.7")
        );

        fs::remove_file(&path).ok();
    }

    #[test]
    fn clearing_the_seen_version_goes_back_to_never_shown() {
        // The absence is the only "unread" there is — a blank version is not a
        // release whose notes were read, so it clears rather than storing "".
        let mut store = SettingsStore::in_memory();
        assert!(store.set_whats_new_seen(Some("0.5.6")));
        assert!(store.set_whats_new_seen(Some("   ")));
        assert_eq!(store.view().whats_new_seen(), None);
        assert!(!store.set_whats_new_seen(None), "already unread");
    }

    #[test]
    fn an_unread_release_is_not_a_document_either() {
        let path = temp_path("whats-new-unread");
        let mut store = SettingsStore::open(&path).unwrap();
        assert!(!store.set_whats_new_seen(None), "already the default");
        store.save().unwrap();
        assert!(!path.exists(), "nothing read is not a document");
    }

    #[test]
    fn the_new_preferences_round_trip_through_the_file() {
        let path = temp_path("round-trip-new");
        {
            let mut store = SettingsStore::open(&path).unwrap();
            assert!(store.set_voice(Some("Meijia")));
            assert!(store.set_animation_pace(Pace::Slow));
            assert!(store.set_board_size(BoardSize::Large));
            assert!(store.set_tone_colours(true));
            store.save().unwrap();
        }
        let reopened = SettingsStore::open(&path).unwrap();
        assert_eq!(reopened.view().voice(), Some("Meijia"));
        assert_eq!(reopened.view().pace(), Pace::Slow);
        assert_eq!(reopened.view().board_size(), BoardSize::Large);
        assert!(reopened.view().tone_colours());

        // The enum names are the ones the interface and the database key rows
        // use, so they are asserted rather than left to the derive.
        let text = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["animationPace"], serde_json::json!("slow"));
        assert_eq!(parsed["boardSize"], serde_json::json!("large"));
        assert_eq!(parsed["voice"], serde_json::json!("Meijia"));
        assert_eq!(parsed["toneColours"], serde_json::json!(true));

        fs::remove_file(&path).ok();
    }

    #[test]
    fn clearing_the_voice_goes_back_to_automatic_rather_than_an_empty_name() {
        let mut store = SettingsStore::in_memory();
        assert!(store.set_voice(Some("Meijia")));
        assert!(store.set_voice(Some("   ")));
        assert_eq!(store.view().voice(), None);
        assert!(!store.set_voice(None), "already unset");
    }

    #[test]
    fn every_choice_is_offered_and_the_names_are_stable() {
        // `ALL` is what the settings screen renders, in order, so a choice added
        // to the enum but not to the list would be unreachable. The *names* are
        // asserted because three things have to agree on them: this enum, the
        // interface's own union type, and the rows in the database (see
        // `hanzi-store`). A rename that reached only one of the three would show
        // the learner their choice had been reset.
        assert_eq!(Pace::ALL, [Pace::Slow, Pace::Normal, Pace::Fast]);
        assert_eq!(Pace::default(), Pace::Normal);
        assert_eq!(BoardSize::ALL, [BoardSize::Compact, BoardSize::Normal, BoardSize::Large]);
        assert_eq!(BoardSize::default(), BoardSize::Normal);

        let names: Vec<String> = Pace::ALL
            .iter()
            .map(|pace| serde_json::to_string(pace).unwrap().trim_matches('"').to_string())
            .collect();
        assert_eq!(names, ["slow", "normal", "fast"]);
        let names: Vec<String> = BoardSize::ALL
            .iter()
            .map(|size| serde_json::to_string(size).unwrap().trim_matches('"').to_string())
            .collect();
        assert_eq!(names, ["compact", "normal", "large"]);
    }

    #[test]
    fn setting_a_preference_to_its_current_value_is_not_a_change() {
        let mut store = SettingsStore::in_memory();
        assert!(store.set_animation_pace(Pace::Fast));
        assert!(!store.set_animation_pace(Pace::Fast));
        assert!(store.set_board_size(BoardSize::Compact));
        assert!(!store.set_board_size(BoardSize::Compact));
        assert!(store.set_voice(Some("Tingting")));
        assert!(!store.set_voice(Some("Tingting")));
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
