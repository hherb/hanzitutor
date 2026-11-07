//! The personal vocabulary list.
//!
//! The built-in course is a fixed frequency-ordered list. Someone actually
//! studying Mandarin has their *own* lessons — a textbook unit, a class, a
//! tutor's handout — and needs to record the characters and words met there and
//! drill exactly those. This module is that list.
//!
//! It is deliberately a plain JSON document on disk rather than a database:
//! small, human-readable, easy to back up, diff or hand-edit. The store takes a
//! path rather than deciding one, so where it lives is a caller's decision and
//! tests can use a temporary directory.
//!
//! An entry's `text` may be one character or several. Single characters get
//! their pinyin and meaning filled in from the bundled dataset when added;
//! multi-character words carry whatever the user typed, because the dataset has
//! no word data (see `ROADMAP.md` M3).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// Timestamps are shared with the practice schedule (see `crate::time`), which
// needs to add intervals to them as well as format them. Re-exported here so the
// vocabulary module's public surface is unchanged.
pub use crate::time::{iso8601_from_unix, now_iso8601};

/// Format version written into the document. Bump when the shape changes and
/// add a migration; a document from the future is refused rather than guessed at.
pub const FORMAT_VERSION: u32 = 1;

/// Longest entry accepted, in characters.
const MAX_TEXT_CHARS: usize = 20;
/// Longest group name accepted, in characters.
const MAX_GROUP_CHARS: usize = 40;

/// Why a vocabulary operation failed.
#[derive(Debug)]
pub enum VocabError {
    /// The entry id does not exist.
    NoSuchEntry(u64),
    /// The group name does not exist.
    NoSuchGroup(String),
    /// A group with that name already exists.
    DuplicateGroup(String),
    /// The same text is already in that group.
    DuplicateEntry(String),
    /// The text was empty, too long, or contained control characters.
    InvalidText(String),
    /// The group name was empty or too long.
    InvalidGroup(String),
    /// The document could not be read or written.
    Io(String),
    /// The document is not valid JSON, or does not match the schema.
    Malformed(String),
    /// The document was written by a newer version of the app.
    UnsupportedVersion(u32),
    /// A replace was refused because not one of the file's `count` entries
    /// could be used under its `rules`, so it would have emptied the list. The
    /// file itself was well formed.
    NothingUsable { count: usize, rules: ImportRules },
}

impl std::fmt::Display for VocabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchEntry(id) => write!(f, "no vocabulary entry with id {id}"),
            Self::NoSuchGroup(name) => write!(f, "no group named {name:?}"),
            Self::DuplicateGroup(name) => write!(f, "a group named {name:?} already exists"),
            Self::DuplicateEntry(text) => {
                write!(f, "{text:?} is already in that group")
            }
            Self::InvalidText(why) => write!(f, "invalid entry: {why}"),
            Self::InvalidGroup(why) => write!(f, "invalid group name: {why}"),
            Self::Io(why) => write!(f, "{why}"),
            Self::Malformed(why) => write!(f, "the vocabulary file is malformed: {why}"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "the vocabulary file was written by a newer version of the app \
                 (format {v}, this build understands {FORMAT_VERSION})"
            ),
            Self::NothingUsable { count, rules } => {
                let faults = match rules {
                    ImportRules::Typed => "empty, too long or holds a control character",
                    ImportRules::Restore => "empty or holds a control character",
                };
                if *count == 1 {
                    write!(
                        f,
                        "the one entry in that file cannot be used — it is {faults} — so \
                         your list was left as it was"
                    )
                } else {
                    write!(
                        f,
                        "none of the {count} entries in that file can be used — each is \
                         {faults} — so your list was left as it was"
                    )
                }
            }
        }
    }
}

impl std::error::Error for VocabError {}

/// One item being studied.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Stable identifier, unique within the document.
    pub id: u64,
    /// The character or word itself.
    pub text: String,
    /// Pronunciation, filled from the dataset for single characters but always
    /// editable.
    #[serde(default)]
    pub pinyin: String,
    #[serde(default)]
    pub meaning: String,
    /// Group name, or `None` for entries not yet filed under a lesson.
    #[serde(default)]
    pub group: Option<String>,
    /// When the entry was added, as an ISO-8601 UTC string. Stored as text so
    /// the file stays readable and so ordering is a plain string comparison —
    /// UTC timestamps in this format sort chronologically.
    pub added_at: String,
    /// How many times the entry has been practised.
    #[serde(default)]
    pub attempts: u32,
    /// Best score achieved, 0..=100.
    #[serde(default)]
    pub best_score: Option<f32>,
    /// When it was last practised, if ever.
    #[serde(default)]
    pub last_practised: Option<String>,
}

impl Entry {
    /// True when this entry is a single character, which is what the bundled
    /// dataset can supply pinyin and a meaning for.
    pub fn is_single_character(&self) -> bool {
        self.text.chars().count() == 1
    }

    /// The characters this entry is written as, in order.
    pub fn characters(&self) -> Vec<char> {
        self.text.chars().collect()
    }
}

/// How well the learner knows one entry, read from the **schedule** rather than
/// from this device's own practice record.
///
/// The distinction is the whole point of the type. [`Entry::attempts`],
/// `best_score` and `last_practised` belong to one device — they are deliberately
/// not part of the stamp that settles a merge — so a list that had just synced
/// would read "not practised" for an entry the other device had known for months.
/// A tag has to mean the same thing everywhere, and what is folded from the synced
/// attempt log does: see [`crate::progress::entry_progress`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryProgress {
    /// No character of the entry is in the schedule at all — nobody has ever
    /// written it, on any device.
    New,
    /// Begun, and not finished: at least one character has never been practised,
    /// or one of them is still on the early rungs of its ladder.
    Learning,
    /// At least one character has come up for review now, whatever else is true of
    /// the others. Which *entry* the queue then names is up to
    /// [`crate::progress::build_queue`] — a character shared by two entries is
    /// offered once, as the lower-numbered of the two — so this says the work is
    /// here, not that the queue will spell this row out.
    Due,
    /// Every character has a card, none of them is due, and every one is
    /// scheduled at least [`crate::progress::KNOWN_INTERVAL_DAYS`] out.
    Known,
}

/// What the **schedule** says about one entry — as opposed to what this device
/// remembers about it.
///
/// Both answers come from the cards a character carries, which are folded from the
/// synced attempt log, so they mean the same thing on every device. [`Entry`]'s own
/// `attempts`, `best_score` and `last_practised` are the other kind of fact: this
/// device's, deliberately outside the stamp that settles a merge, and the wrong
/// source for either of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryStanding {
    /// How well it is known, for the tag on the card.
    pub progress: EntryProgress,
    /// Whether **every** character the board can draw for the entry has been
    /// practised at least once — the question the drill's queue asks, because an
    /// entry is recorded when all of its characters are written.
    ///
    /// The strict reading is deliberate: one character still untouched keeps the
    /// entry out of the drill queue, so an entry abandoned half-written is offered
    /// again rather than quietly counted as done. With **nothing** to draw it is
    /// vacuously true, which is also the right answer for the queue — an entry the
    /// board cannot ask for must not be offered.
    pub all_characters_practised: bool,
}

/// One entry as the interface sees it: the entry itself, and what the schedule
/// knows about it.
///
/// A view of its own rather than fields on [`Entry`], for the reason
/// [`VocabView::warning`] is not a field of [`Document`]: these are **derived from
/// the schedule**, and storing them would be a second source of truth for
/// something the very next attempt changes — and they would travel in the exported
/// JSON as though the learner had written them.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabEntryView {
    #[serde(flatten)]
    pub entry: Entry,
    /// `None` means **the schedule was not consulted** — a view the engine built
    /// on its own, as a test fixture or a round trip through a document does. It is
    /// deliberately not a `New`/`false` standing, which is the schedule answering
    /// "nothing here has ever been practised": the same distinction schema 5 draws
    /// between a null measure and a zero one, and for the same reason — a reader
    /// that conflates them reports something it invented.
    #[serde(default)]
    pub standing: Option<EntryStanding>,
    /// One tone per character of the entry's text, tone 1..=5, in order.
    ///
    /// The entry's **own reading** read against its own characters — the same
    /// pairing the word list sends, and for the same reason: the rule that says
    /// which syllable belongs to which character lives in [`crate::pinyin`], and
    /// a second copy of it in the interface would be a second source of truth for
    /// something a learner's own typed pinyin can already settle. 医院 typed as
    /// `yīyuàn` colours 院 with tone 4, and 爱好 typed as `àihào` colours 好 with
    /// tone 4 where the character on its own is `hǎo`, tone 3.
    ///
    /// **Empty** when the reading will not divide one syllable per character —
    /// the pinyin box accepts anything — or when it is blank. The interface then
    /// falls back to each character's own tone, which is right for everything but
    /// a polyphone, rather than showing a pairing nobody can justify.
    #[serde(default)]
    pub tones: Vec<u8>,
    /// The entry's reading split one syllable per character, for colouring the
    /// pinyin beside the characters. Empty exactly when [`Self::tones`] is.
    #[serde(default)]
    pub syllables: Vec<String>,
}

impl VocabEntryView {
    /// The entry on its own, with no standing: what [`VocabStore::view`] can
    /// answer on its own.
    ///
    /// A reading saved with tone numbers by an older build, or imported that
    /// way, is shown with the marks its syllables are aligned as
    /// ([`crate::pinyin::marked_reading`]), so the interface can find each one
    /// in the text. The document on disk is left as it is.
    fn untagged(mut entry: Entry) -> Self {
        entry.pinyin = crate::pinyin::marked_reading(&entry.pinyin);
        let (tones, syllables) = crate::pinyin::aligned_reading(&entry.text, &entry.pinyin)
            .map(|list| {
                (
                    list.iter().map(|syllable| syllable.tone).collect(),
                    list.into_iter().map(|syllable| syllable.text).collect(),
                )
            })
            .unwrap_or_default();
        Self {
            entry,
            standing: None,
            tones,
            syllables,
        }
    }
}

/// What the interface needs to render the list.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabView {
    pub entries: Vec<VocabEntryView>,
    pub groups: Vec<String>,
    /// Set when the change was applied in memory but could not be saved, so the
    /// interface can say so instead of silently losing data.
    #[serde(default)]
    pub warning: Option<String>,
}

/// The persisted document. Public because a backing store other than the JSON
/// file has to build one; the shape is the list's, not the database's.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub version: u32,
    /// Ids are allocated from here, so they are never reused after a deletion.
    pub next_id: u64,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub entries: Vec<Entry>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            next_id: 1,
            groups: Vec::new(),
            entries: Vec::new(),
        }
    }
}

/// Summary of an import, so the interface can report what happened.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub added: usize,
    pub skipped_duplicates: usize,
    /// Entries left out because their text breaks the import's
    /// [`ImportRules`]: empty, or holding a control character, and for a file
    /// the learner chose also longer than [`VocabStore::add_entry`] accepts. The
    /// rest of the file is still imported, the way a duplicate is skipped rather
    /// than failing the whole import.
    #[serde(default)]
    pub skipped_invalid: usize,
    /// Entries kept, but filed under no group, because the group name they
    /// carried breaks the rules. Counted only for entries actually added.
    #[serde(default)]
    pub unfiled_invalid_group: usize,
    /// Names in the file's list of groups left out because they are blank or
    /// break the rules.
    #[serde(default)]
    pub skipped_invalid_groups: usize,
    /// Entries that, once filed under no group, met another copy of the same
    /// word there and were merged into it — practice counters added up, the way
    /// [`VocabStore::remove_group`] merges — rather than skipped as duplicates.
    #[serde(default)]
    pub merged_unfiled: usize,
    pub groups_added: usize,
    /// True when the import replaced the existing list rather than merging.
    pub replaced: bool,
}

/// How strictly [`VocabStore::import_json_with`] holds a file's rows to the
/// list's rules (issue #40).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportRules {
    /// A file the learner chose: every row is held to the rules typing it is, as
    /// [`VocabStore::add_entry`] and [`VocabStore::add_group`] check them.
    Typed,
    /// The app's own data coming back, as a backup restore. Earlier versions,
    /// the sync and the legacy migration never checked length, so a backup may
    /// hold text or group names longer than typing allows, and they are kept;
    /// only what cannot be an entry at all — empty text, or text with a control
    /// character — is left out.
    Restore,
}

impl ImportRules {
    /// The entry's text, normalised, or `None` when it breaks the rules.
    fn text(self, text: &str) -> Option<String> {
        match self {
            Self::Typed => validate_text(text).ok(),
            Self::Restore => {
                let text = text.trim();
                (!text.is_empty() && !text.chars().any(char::is_control)).then(|| text.to_string())
            }
        }
    }

    /// The group, normalised (`Some(None)` is unfiled), or `None` when the
    /// name breaks the rules.
    fn group(self, group: Option<&str>) -> Option<Option<String>> {
        match self {
            Self::Typed => validate_group(group).ok(),
            Self::Restore => match group.map(str::trim).filter(|g| !g.is_empty()) {
                None => Some(None),
                Some(g) if g.chars().any(char::is_control) => None,
                Some(g) => Some(Some(g.to_string())),
            },
        }
    }
}

/// An import checked against its rules and ready to join the list.
///
/// Everything that can refuse a file happens in [`Self::prepare`], and it reads
/// nothing but the file, so a caller with other work riding on the import — a
/// backup restore, which merges an attempt log as well — can find out first
/// and change nothing at all when the list would be refused. Applying it with
/// [`VocabStore::apply_import`] cannot fail.
#[derive(Clone, Debug)]
pub struct PreparedImport {
    merge: bool,
    groups: Vec<String>,
    /// Each entry, normalised, with whether it was filed under no group because
    /// its group name broke the rules.
    entries: Vec<(Entry, bool)>,
    summary: ImportSummary,
}

impl PreparedImport {
    /// Read and check an exported document.
    ///
    /// Every row is held to `rules` (issue #40), text and group names trimmed
    /// first and a blank group read as unfiled:
    ///
    /// * an entry whose **text** breaks them is skipped and counted in
    ///   [`ImportSummary::skipped_invalid`];
    /// * an entry whose only fault is its **group** is kept, filed under no
    ///   group, and counted in [`ImportSummary::unfiled_invalid_group`];
    /// * a bad name in the list of groups is dropped and counted in
    ///   [`ImportSummary::skipped_invalid_groups`].
    ///
    /// The practice fields are made sane on the way in: a best score is
    /// clamped to `0..=100` (none when it is not a number), and an `addedAt`
    /// that is not an ISO-8601 UTC time becomes now, as a `lastPractised` that
    /// is not one becomes none.
    ///
    /// The file is refused when it is not a list at all, comes from a newer
    /// version, or would *replace* the list with nothing because every entry in
    /// it broke the rules ([`VocabError::NothingUsable`]) — a replace that
    /// empties the list should be what the file says, not what the rules left of
    /// it.
    pub fn prepare(json: &str, merge: bool, rules: ImportRules) -> Result<Self, VocabError> {
        let incoming: Document =
            serde_json::from_str(json).map_err(|e| VocabError::Malformed(e.to_string()))?;
        if incoming.version > FORMAT_VERSION {
            return Err(VocabError::UnsupportedVersion(incoming.version));
        }

        let mut summary = ImportSummary {
            replaced: !merge,
            ..Default::default()
        };
        let mut groups = Vec::new();
        for name in incoming.groups {
            match rules.group(Some(&name)) {
                Some(Some(name)) => groups.push(name),
                // Blank, or breaking the rules: no group, as `add_group` says.
                _ => summary.skipped_invalid_groups += 1,
            }
        }
        let mut entries = Vec::new();
        for mut entry in incoming.entries {
            let Some(text) = rules.text(&entry.text) else {
                summary.skipped_invalid += 1;
                continue;
            };
            let group = rules.group(entry.group.as_deref());
            let refiled = group.is_none();
            entry.text = text;
            entry.group = group.flatten();
            entry.pinyin = entry.pinyin.trim().to_string();
            entry.meaning = entry.meaning.trim().to_string();
            entry.best_score = entry
                .best_score
                .filter(|score| score.is_finite())
                .map(|score| score.clamp(0.0, 100.0));
            if crate::time::parse_iso8601(&entry.added_at).is_none() {
                entry.added_at = now_iso8601();
            }
            entry.last_practised = entry
                .last_practised
                .filter(|when| crate::time::parse_iso8601(when).is_some());
            entries.push((entry, refiled));
        }
        if !merge && entries.is_empty() && summary.skipped_invalid > 0 {
            return Err(VocabError::NothingUsable {
                count: summary.skipped_invalid,
                rules,
            });
        }
        Ok(Self {
            merge,
            groups,
            entries,
            summary,
        })
    }
}

/// Where a vocabulary list is kept.
///
/// The JSON file at [`VocabStore::path`] is the built-in backing and needs no
/// sink; this is the seam for a database, which lives in its own crate so that
/// this one keeps no native dependency. A list is small and edited by hand, so
/// unlike the attempt log it is handed over whole.
pub trait VocabSink: std::fmt::Debug + Send {
    fn load(&self) -> Result<Document, VocabError>;
    /// Persist `document`.
    ///
    /// Returns `Some(fresh)` when the backing store had moved on since this sink
    /// last loaded or saved (a sync wrote to it in between), in which case the
    /// sink has written only what the learner changed and `fresh` is the list as
    /// it now stands. The store adopts it, so its next save starts from the truth
    /// rather than from a document that no longer describes the database.
    fn save(&mut self, document: &Document) -> Result<Option<Document>, VocabError>;
}

/// The vocabulary list, backed by a JSON file or by a [`VocabSink`].
#[derive(Debug)]
pub struct VocabStore {
    path: PathBuf,
    document: Document,
    /// Set when the list is kept somewhere other than the JSON file at `path`.
    sink: Option<Box<dyn VocabSink>>,
}

impl VocabStore {
    /// Open the list at `path`, creating an empty one if the file is absent.
    ///
    /// A missing file is normal (first run). A *corrupt* file is a hard error:
    /// silently starting empty would look like the user's work had vanished.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, VocabError> {
        let path = path.into();
        let document = match fs::read_to_string(&path) {
            Ok(text) => {
                let parsed: Document = serde_json::from_str(&text)
                    .map_err(|e| VocabError::Malformed(e.to_string()))?;
                if parsed.version > FORMAT_VERSION {
                    return Err(VocabError::UnsupportedVersion(parsed.version));
                }
                parsed
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Document::default(),
            Err(e) => return Err(VocabError::Io(format!("{}: {e}", path.display()))),
        };
        let mut store = Self {
            path,
            document,
            sink: None,
        };
        store.sort();
        Ok(store)
    }

    /// Open a list kept by `sink` rather than by a JSON file.
    pub fn open_with(sink: Box<dyn VocabSink>) -> Result<Self, VocabError> {
        let document = sink.load()?;
        let mut store = Self {
            path: PathBuf::new(),
            document,
            sink: Some(sink),
        };
        store.sort();
        Ok(store)
    }

    /// An in-memory list with no file behind it. Used by tests.
    pub fn in_memory() -> Self {
        Self {
            path: PathBuf::new(),
            document: Document::default(),
            sink: None,
        }
    }

    /// The file this list is kept in, or an empty path when a sink holds it.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The whole list, as a sink sees it.
    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn view(&self) -> VocabView {
        VocabView {
            entries: self
                .document
                .entries
                .iter()
                .cloned()
                .map(VocabEntryView::untagged)
                .collect(),
            groups: self.document.groups.clone(),
            warning: None,
        }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.document.entries
    }

    pub fn groups(&self) -> &[String] {
        &self.document.groups
    }

    /// The entries in one group. Pass `None` for the unfiled ones.
    pub fn entries_in(&self, group: Option<&str>) -> Vec<&Entry> {
        self.document
            .entries
            .iter()
            .filter(|e| e.group.as_deref() == group)
            .collect()
    }

    /// Entries due to be practised for a given selection: everything, one group,
    /// or the unfiled remainder.
    pub fn practice_set(&self, group: Option<&str>) -> Vec<Entry> {
        match group {
            // `None` means "the whole list", which is what the interface asks
            // for when no particular group is selected.
            None => self.document.entries.clone(),
            Some(name) => self
                .entries_in(Some(name))
                .into_iter()
                .cloned()
                .collect(),
        }
    }

    /// Persist the list; through a sink, or atomically to the JSON file.
    ///
    /// Writes to a temporary file and renames, so an interrupted write cannot
    /// leave a half-written list behind.
    pub fn save(&mut self) -> Result<(), VocabError> {
        if let Some(sink) = self.sink.as_mut() {
            if let Some(fresh) = sink.save(&self.document)? {
                self.document = fresh;
                self.sort();
            }
            return Ok(());
        }
        if self.path.as_os_str().is_empty() {
            return Ok(()); // in-memory store
        }
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| VocabError::Io(format!("{}: {e}", parent.display())))?;
            }
        }
        let json = serde_json::to_string_pretty(&self.document)
            .map_err(|e| VocabError::Io(e.to_string()))?;

        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, json)
            .map_err(|e| VocabError::Io(format!("{}: {e}", temporary.display())))?;
        fs::rename(&temporary, &self.path)
            .map_err(|e| VocabError::Io(format!("{}: {e}", self.path.display())))?;
        Ok(())
    }

    /// Add an entry, filling in group creation and pinyin as needed.
    ///
    /// A reading typed with tone numbers (`ni3hao3`) is saved with the marks
    /// they stand for (`nǐhǎo`) — see [`crate::pinyin::marked_reading`]; anything
    /// else is saved as typed.
    pub fn add_entry(
        &mut self,
        text: &str,
        pinyin: &str,
        meaning: &str,
        group: Option<&str>,
    ) -> Result<Entry, VocabError> {
        let text = validate_text(text)?;
        let group = validate_group(group)?;

        if self
            .document
            .entries
            .iter()
            .any(|e| e.text == text && e.group == group)
        {
            return Err(VocabError::DuplicateEntry(text));
        }
        if let Some(name) = &group {
            self.ensure_group(name);
        }

        let entry = Entry {
            id: self.document.next_id,
            text,
            pinyin: crate::pinyin::marked_reading(pinyin.trim()),
            meaning: meaning.trim().to_string(),
            group,
            added_at: now_iso8601(),
            attempts: 0,
            best_score: None,
            last_practised: None,
        };
        self.document.next_id += 1;
        self.document.entries.push(entry.clone());
        self.sort();
        Ok(entry)
    }

    /// Change an entry's pinyin, meaning or group.
    ///
    /// Moving an entry into a group that already holds the same text is refused
    /// with [`VocabError::DuplicateEntry`], the rule [`Self::add_entry`] keeps
    /// (issue #40), and nothing changes — not even the group being created. An
    /// entry that stays where it is can always be edited.
    pub fn update_entry(
        &mut self,
        id: u64,
        pinyin: &str,
        meaning: &str,
        group: Option<&str>,
    ) -> Result<Entry, VocabError> {
        let group = validate_group(group)?;
        let index = self
            .document
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or(VocabError::NoSuchEntry(id))?;
        let moving = &self.document.entries[index];
        if moving.group != group
            && self
                .document
                .entries
                .iter()
                .any(|e| e.id != id && e.text == moving.text && e.group == group)
        {
            return Err(VocabError::DuplicateEntry(moving.text.clone()));
        }
        // A rename to a non-empty group should not require creating it first.
        if let Some(name) = &group {
            self.ensure_group(name);
        }
        let entry = &mut self.document.entries[index];
        entry.pinyin = crate::pinyin::marked_reading(pinyin.trim());
        entry.meaning = meaning.trim().to_string();
        entry.group = group;
        let updated = entry.clone();
        self.sort();
        Ok(updated)
    }

    pub fn remove_entry(&mut self, id: u64) -> Result<Entry, VocabError> {
        let index = self
            .document
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or(VocabError::NoSuchEntry(id))?;
        Ok(self.document.entries.remove(index))
    }

    pub fn add_group(&mut self, name: &str) -> Result<String, VocabError> {
        let name = validate_group(Some(name))?
            .ok_or_else(|| VocabError::InvalidGroup("the name is empty".into()))?;
        if self.document.groups.contains(&name) {
            return Err(VocabError::DuplicateGroup(name));
        }
        self.document.groups.push(name.clone());
        self.sort();
        Ok(name)
    }

    /// Rename a group, moving its entries with it.
    pub fn rename_group(&mut self, from: &str, to: &str) -> Result<String, VocabError> {
        if !self.document.groups.iter().any(|g| g == from) {
            return Err(VocabError::NoSuchGroup(from.to_string()));
        }
        let to = validate_group(Some(to))?
            .ok_or_else(|| VocabError::InvalidGroup("the name is empty".into()))?;
        if to != from && self.document.groups.contains(&to) {
            return Err(VocabError::DuplicateGroup(to));
        }
        for group in self.document.groups.iter_mut() {
            if *group == from {
                *group = to.clone();
            }
        }
        for entry in self.document.entries.iter_mut() {
            if entry.group.as_deref() == Some(from) {
                entry.group = Some(to.clone());
            }
        }
        self.sort();
        Ok(to)
    }

    /// Remove a group.
    ///
    /// With `delete_entries` false the entries survive and become unfiled, which
    /// is the safe default — deleting a label should not destroy work. Returns
    /// how many entries left the group, either way.
    ///
    /// An entry whose text is **already unfiled** is merged into that entry
    /// rather than becoming a second copy of it (issue #40), since one word may
    /// be in a group only once — the rule [`Self::add_entry`] keeps — and
    /// refusing would leave a group that cannot be deleted without first deleting
    /// work. The entry already unfiled survives, and nothing it holds is
    /// overwritten: it takes the moved entry's pinyin and meaning only where its
    /// own are blank, and the practice counters add up — attempts summed, the
    /// better best score and the later practice kept. The counters add up
    /// exactly as the sync's collapse of two copies of one word adds them
    /// (`collapse_counters` in `hanzi-store`, issue #35). The content does not
    /// follow that collapse, on purpose: the collapse carries the newest copy's
    /// pinyin and meaning by last-writer-wins, while removing a group is the
    /// learner tidying their own list, and keeps what the survivor holds.
    ///
    /// That collapse keeps the copy with the lowest uuid, which this crate cannot
    /// see. Choosing by group instead is what keeps devices agreeing: which entry
    /// is unfiled is synced state, so two devices that remove the same group pick
    /// the same survivor and delete the same copy, and a sync then never meets two
    /// live copies to collapse. Two copies of one word inside the removed group —
    /// only an older list can hold them — fold together the same way, the first
    /// unfiled becoming the survivor for the rest. One case is left to chance:
    /// when a list already holds two unfiled copies of the word (again, only an
    /// older one can), the survivor is the first in list order, and two such
    /// copies with the same `added_at` and text tie there, so which one survives
    /// may differ between devices — the collapse above then settles it.
    pub fn remove_group(&mut self, name: &str, delete_entries: bool) -> Result<usize, VocabError> {
        let index = self
            .document
            .groups
            .iter()
            .position(|g| g == name)
            .ok_or_else(|| VocabError::NoSuchGroup(name.to_string()))?;
        self.document.groups.remove(index);

        if delete_entries {
            let before = self.document.entries.len();
            self.document
                .entries
                .retain(|e| e.group.as_deref() != Some(name));
            Ok(before - self.document.entries.len())
        } else {
            let mut unfiled = 0;
            let mut merged = Vec::new();
            for index in 0..self.document.entries.len() {
                if self.document.entries[index].group.as_deref() != Some(name) {
                    continue;
                }
                unfiled += 1;
                let text = &self.document.entries[index].text;
                let twin = self
                    .document
                    .entries
                    .iter()
                    .position(|e| e.group.is_none() && e.text == *text);
                match twin {
                    Some(survivor) => {
                        let moved = self.document.entries[index].clone();
                        merge_into(&mut self.document.entries[survivor], &moved);
                        merged.push(moved.id);
                    }
                    None => self.document.entries[index].group = None,
                }
            }
            self.document.entries.retain(|e| !merged.contains(&e.id));
            self.sort();
            Ok(unfiled)
        }
    }

    /// Record a practice attempt against an entry.
    ///
    /// `score` is the 0..=100 headline score from the grading engine.
    pub fn record_attempt(&mut self, id: u64, score: f32) -> Result<Entry, VocabError> {
        let score = score.clamp(0.0, 100.0);
        let entry = self
            .document
            .entries
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or(VocabError::NoSuchEntry(id))?;
        // Saturating: an imported count can already sit at the top.
        entry.attempts = entry.attempts.saturating_add(1);
        entry.best_score = Some(match entry.best_score {
            Some(best) => best.max(score),
            None => score,
        });
        entry.last_practised = Some(now_iso8601());
        Ok(entry.clone())
    }

    /// The whole list as pretty JSON, for export or display.
    pub fn export_json(&self) -> Result<String, VocabError> {
        serde_json::to_string_pretty(&self.document)
            .map_err(|e| VocabError::Io(e.to_string()))
    }

    /// The list as CSV, for spreadsheets. Lossy by design — use JSON to
    /// round-trip — so there is no matching import.
    pub fn export_csv(&self) -> String {
        let mut out = String::from("text,pinyin,meaning,group,attempts,best_score,last_practised,added_at\n");
        for entry in &self.document.entries {
            let fields = [
                entry.text.clone(),
                entry.pinyin.clone(),
                entry.meaning.clone(),
                entry.group.clone().unwrap_or_default(),
                entry.attempts.to_string(),
                entry
                    .best_score
                    .map(|s| format!("{s:.0}"))
                    .unwrap_or_default(),
                entry.last_practised.clone().unwrap_or_default(),
                entry.added_at.clone(),
            ];
            let row: Vec<String> = fields.iter().map(|f| csv_field(f)).collect();
            out.push_str(&row.join(","));
            out.push('\n');
        }
        out
    }

    /// Merge or replace from an exported document the learner chose, held to
    /// [`ImportRules::Typed`]. See [`Self::import_json_with`].
    pub fn import_json(&mut self, json: &str, merge: bool) -> Result<ImportSummary, VocabError> {
        self.import_json_with(json, merge, ImportRules::Typed)
    }

    /// Merge or replace from an exported document.
    ///
    /// [`PreparedImport::prepare`] then [`Self::apply_import`]; see the first
    /// for what is checked and when a file is refused, and the second for how
    /// it joins the list.
    pub fn import_json_with(
        &mut self,
        json: &str,
        merge: bool,
        rules: ImportRules,
    ) -> Result<ImportSummary, VocabError> {
        let prepared = PreparedImport::prepare(json, merge, rules)?;
        Ok(self.apply_import(prepared))
    }

    /// Join a checked import to the list. Nothing here can fail.
    ///
    /// Merging keeps existing entries, adds the new ones and unions the groups;
    /// an imported entry whose text already exists in the same group is skipped
    /// rather than duplicated — whether it was there before or arrived earlier in
    /// the same file, in either mode. The one exception is a collision between
    /// two entries of the same file that the import itself caused, by filing one
    /// of them under no group because its group name broke the rules: that is
    /// not a repeat in the file, so the two are merged instead, as
    /// [`Self::remove_group`] merges, and counted in
    /// [`ImportSummary::merged_unfiled`]. A collision with an entry the list
    /// already held is a duplicate like any other, so importing a file twice
    /// never counts its practice twice.
    pub fn apply_import(&mut self, prepared: PreparedImport) -> ImportSummary {
        let PreparedImport {
            merge,
            groups,
            entries,
            mut summary,
        } = prepared;

        if !merge {
            self.document.entries.clear();
            self.document.groups.clear();
        }

        for name in groups {
            if !self.document.groups.contains(&name) {
                self.document.groups.push(name);
                summary.groups_added += 1;
            }
        }

        // Ids of the entries this import filed under no group, and the first id
        // it hands out, which tells its own entries from the list's.
        let mut refiled_ids = Vec::new();
        let first_new_id = self.document.next_id;
        for (mut entry, refiled) in entries {
            let twin = self
                .document
                .entries
                .iter()
                .position(|e| e.text == entry.text && e.group == entry.group);
            if let Some(twin) = twin {
                let survivor = &mut self.document.entries[twin];
                let caused_here = refiled || refiled_ids.contains(&survivor.id);
                if survivor.id >= first_new_id && caused_here {
                    merge_into(survivor, &entry);
                    summary.merged_unfiled += 1;
                } else {
                    summary.skipped_duplicates += 1;
                }
                continue;
            }
            if let Some(name) = entry.group.clone() {
                self.ensure_group(&name);
            }
            // Ids are reallocated so an import can never collide with, or
            // resurrect, an id this document has already used.
            entry.id = self.document.next_id;
            self.document.next_id += 1;
            if refiled {
                refiled_ids.push(entry.id);
                summary.unfiled_invalid_group += 1;
            }
            self.document.entries.push(entry);
            summary.added += 1;
        }

        self.sort();
        summary
    }

    fn ensure_group(&mut self, name: &str) {
        if !self.document.groups.iter().any(|g| g == name) {
            self.document.groups.push(name.to_string());
        }
    }

    /// Keep groups alphabetical and entries grouped together, in the order they
    /// were added. Deterministic ordering keeps the file diffable.
    /// Put the list in the order every device agrees on.
    ///
    /// By `added_at`, then the text — **not** by `id`. An `id` is handed out per
    /// device, and a peer's entries arrive in the order the merge produced them
    /// (uuid order), so ordering by id showed the same list in a different order
    /// on every device: one group's word sat at 51 here and 11 there, and a drill
    /// resumed at the right word only to work through a differently-ordered queue.
    /// `added_at` is an ISO-8601 UTC string that syncs, so it sorts chronologically
    /// and identically everywhere; the text settles two entries added in the same
    /// second, which is otherwise the one case left to chance.
    fn sort(&mut self) {
        self.document.groups.sort();
        self.document.groups.dedup();
        self.document.entries.sort_by(entry_order);
    }
}

/// Fold `moved` into `survivor`, two entries for the same word in the same place.
///
/// The survivor keeps everything it has; the moved entry's pinyin and meaning
/// fill only what is blank, and the practice counters add up the way the sync
/// adds them when it collapses duplicates — attempts summed, the better best
/// score and the later practice kept.
fn merge_into(survivor: &mut Entry, moved: &Entry) {
    if survivor.pinyin.is_empty() {
        survivor.pinyin = moved.pinyin.clone();
    }
    if survivor.meaning.is_empty() {
        survivor.meaning = moved.meaning.clone();
    }
    survivor.attempts = survivor.attempts.saturating_add(moved.attempts);
    survivor.best_score = match (survivor.best_score, moved.best_score) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    survivor.last_practised = match (&survivor.last_practised, &moved.last_practised) {
        (Some(a), Some(b)) => Some(a.max(b).clone()),
        (a, b) => a.clone().or_else(|| b.clone()),
    };
}

/// Validate and normalise entry text.
fn validate_text(text: &str) -> Result<String, VocabError> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(VocabError::InvalidText("it is empty".into()));
    }
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err(VocabError::InvalidText(format!(
            "it is longer than {MAX_TEXT_CHARS} characters"
        )));
    }
    if text.chars().any(|c| c.is_control()) {
        return Err(VocabError::InvalidText(
            "it contains a control character".into(),
        ));
    }
    Ok(text)
}

/// Validate and normalise a group name, where `None` and empty both mean
/// "unfiled".
fn validate_group(group: Option<&str>) -> Result<Option<String>, VocabError> {
    let Some(group) = group else {
        return Ok(None);
    };
    let group = group.trim();
    if group.is_empty() {
        return Ok(None);
    }
    if group.chars().count() > MAX_GROUP_CHARS {
        return Err(VocabError::InvalidGroup(format!(
            "it is longer than {MAX_GROUP_CHARS} characters"
        )));
    }
    if group.chars().any(|c| c.is_control()) {
        return Err(VocabError::InvalidGroup(
            "it contains a control character".into(),
        ));
    }
    Ok(Some(group.to_string()))
}

/// Quote a CSV field when it contains a comma, quote or newline, and disarm a
/// field a spreadsheet would read as a formula.
///
/// The one CSV quoting rule in the project, shared with the attempt log's
/// export: two spellings of "escape a field" is one more than a spreadsheet
/// will forgive.
///
/// **This is a safety boundary, not only a formatting one** — the quoting half
/// is RFC 4180 and the other half is not, so do not simplify it back. Excel,
/// LibreOffice Calc, Google Sheets and Numbers treat a cell that *starts* with
/// `=`, `+`, `-`, `@`, a tab or a carriage return as a formula, and CSV quoting
/// is stripped on load: `"=1+1"` is still the formula `=1+1`. Since the
/// vocabulary list holds learner-supplied text — including text that arrived
/// from somebody else's shared list through [`VocabStore::import_json`] — an
/// exported cell would otherwise be live code in whoever opens it. A leading
/// apostrophe is the spreadsheet convention for "this is text"; it is stripped
/// on display and costs an ordinary field nothing.
pub fn csv_field(value: &str) -> String {
    let value = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.to_string()
    };
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
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
        std::env::temp_dir().join(format!("hanzi-vocab-{name}-{unique}.json"))
    }

    // ---- adding and updating ---------------------------------------------

    #[test]
    fn adds_entries_and_creates_groups_implicitly() {
        let mut store = VocabStore::in_memory();
        let entry = store
            .add_entry("学习", "xuéxí", "to study", Some("Lesson 3"))
            .unwrap();

        assert_eq!(entry.text, "学习");
        assert_eq!(entry.attempts, 0);
        assert_eq!(entry.best_score, None);
        assert!(entry.last_practised.is_none());
        assert_eq!(store.groups(), ["Lesson 3"]);
        // A word is not a single character, so the dataset cannot supply it.
        assert!(!entry.is_single_character());
    }

    #[test]
    fn a_single_character_is_recognised_as_such() {
        let mut store = VocabStore::in_memory();
        let entry = store.add_entry("好", "hǎo", "good", None).unwrap();
        assert!(entry.is_single_character());
        assert_eq!(entry.characters(), vec!['好']);
        assert_eq!(store.groups(), [] as [String; 0]);
        assert_eq!(entry.group, None);
    }

    #[test]
    fn an_entry_carries_the_tone_of_each_of_its_characters() {
        // The view pairs the entry's own reading with its own characters, so a
        // screen can colour it without a second rule for splitting pinyin.
        let mut store = VocabStore::in_memory();
        store.add_entry("学习", "xuéxí", "to study", None).unwrap();
        store.add_entry("的话", "dehuà", "words", None).unwrap();
        let view = store.view();

        let learning = &view.entries[0];
        assert_eq!(learning.tones, vec![2, 2]);
        assert_eq!(learning.syllables, vec!["xué".to_string(), "xí".to_string()]);
        // A neutral syllable keeps its place: 的 is tone 5, which is the case a
        // split driven by tone marks alone would get wrong.
        let words = &view.entries[1];
        assert_eq!(words.tones, vec![5, 4]);
        assert_eq!(words.syllables, vec!["de".to_string(), "huà".to_string()]);
    }

    #[test]
    fn a_reading_typed_with_tone_numbers_is_kept_with_marks() {
        // `ni3hao3` is what a learner without a pinyin keyboard types; it is
        // saved the way the dataset writes it, so the list shows `nǐhǎo` and the
        // reading colours like any other (issue #20).
        let mut store = VocabStore::in_memory();
        let added = store.add_entry("你好", "ni3hao3", "hello", None).unwrap();
        assert_eq!(added.pinyin, "nǐhǎo");
        let view = store.view();
        assert_eq!(view.entries[0].tones, vec![3, 3]);
        assert_eq!(
            view.entries[0].syllables,
            vec!["nǐ".to_string(), "hǎo".to_string()]
        );

        let updated = store
            .update_entry(added.id, "xi1an1", "Xi'an", None)
            .unwrap();
        assert_eq!(updated.pinyin, "xī'ān");
        // Anything that is not pinyin is kept exactly as typed.
        let kept = store
            .update_entry(added.id, "  ni hao? ", "hello", None)
            .unwrap();
        assert_eq!(kept.pinyin, "ni hao?");
    }

    #[test]
    fn a_numbered_reading_saved_by_an_older_build_is_shown_with_marks() {
        // An entry already on disk as `xi1'an1` (or imported so) is shown with
        // the marks its syllables are aligned as, so the interface finds each
        // syllable in the text and keeps the separator between them.
        let json = r#"{"version":1,"nextId":3,"groups":[],"entries":[
            {"id":1,"text":"西安","pinyin":"xi1'an1","meaning":"Xi'an","group":null,
             "addedAt":"2026-01-01T00:00:00Z","attempts":0,"bestScore":null,"lastPractised":null},
            {"id":2,"text":"你好","pinyin":"ni3 hao3","meaning":"hello","group":null,
             "addedAt":"2026-01-01T00:00:00Z","attempts":0,"bestScore":null,"lastPractised":null}
        ]}"#;
        let mut store = VocabStore::in_memory();
        store.import_json(json, false).unwrap();
        let view = store.view();
        let shown = |text: &str| {
            view.entries
                .iter()
                .find(|e| e.entry.text == text)
                .unwrap_or_else(|| panic!("{text} was imported"))
        };
        let xian = shown("西安");
        assert_eq!(xian.entry.pinyin, "xī'ān");
        assert_eq!(xian.syllables, vec!["xī".to_string(), "ān".to_string()]);
        let hello = shown("你好");
        assert_eq!(hello.entry.pinyin, "nǐ hǎo");
        assert_eq!(hello.tones, vec![3, 3]);
        // Each syllable is found in the text as shown.
        for entry in [xian, hello] {
            for syllable in &entry.syllables {
                assert!(entry.entry.pinyin.contains(syllable.as_str()), "{syllable}");
            }
        }
    }

    #[test]
    fn an_entry_whose_reading_does_not_fit_its_text_carries_no_tones() {
        // The pinyin box accepts anything, so the honest answer for a reading
        // that will not divide one syllable per character is to send nothing and
        // let the interface fall back to each character's own tone. Pairing it up
        // anyway would put a tone on the wrong glyph.
        let mut store = VocabStore::in_memory();
        store.add_entry("学习", "xué", "to study", None).unwrap();
        store.add_entry("你好", "", "hello", None).unwrap();
        store.add_entry("朋友", "péng yǒu yǒu", "friend", None).unwrap();
        let view = store.view();

        for entry in &view.entries {
            assert!(entry.tones.is_empty(), "{:?} has no aligned tones", entry.entry.text);
            assert!(entry.syllables.is_empty());
        }
    }

    #[test]
    fn rejects_empty_oversized_and_control_character_text() {
        let mut store = VocabStore::in_memory();
        assert!(matches!(
            store.add_entry("   ", "", "", None),
            Err(VocabError::InvalidText(_))
        ));
        assert!(matches!(
            store.add_entry(&"一".repeat(MAX_TEXT_CHARS + 1), "", "", None),
            Err(VocabError::InvalidText(_))
        ));
        assert!(matches!(
            store.add_entry("好\n坏", "", "", None),
            Err(VocabError::InvalidText(_))
        ));
        // Text is trimmed rather than rejected.
        let entry = store.add_entry("  好  ", "", "", None).unwrap();
        assert_eq!(entry.text, "好");
    }

    #[test]
    fn rejects_a_duplicate_within_the_same_group() {
        let mut store = VocabStore::in_memory();
        store.add_entry("好", "", "", Some("A")).unwrap();
        assert!(matches!(
            store.add_entry("好", "", "", Some("A")),
            Err(VocabError::DuplicateEntry(_))
        ));
        // The same text in a different group is a different thing to learn.
        store.add_entry("好", "", "", Some("B")).unwrap();
        assert_eq!(store.entries().len(), 2);
    }

    #[test]
    fn blank_group_names_mean_unfiled() {
        let mut store = VocabStore::in_memory();
        let entry = store.add_entry("好", "", "", Some("   ")).unwrap();
        assert_eq!(entry.group, None);
        assert_eq!(store.groups(), [] as [String; 0]);
    }

    #[test]
    fn records_attempts_and_keeps_the_best_score() {
        let mut store = VocabStore::in_memory();
        let id = store.add_entry("好", "", "", None).unwrap().id;

        store.record_attempt(id, 70.0).unwrap();
        let after_first = store.entries()[0].clone();
        assert_eq!(after_first.attempts, 1);
        assert_eq!(after_first.best_score, Some(70.0));
        assert!(after_first.last_practised.is_some());

        store.record_attempt(id, 55.0).unwrap();
        let after_worse = store.entries()[0].clone();
        assert_eq!(after_worse.attempts, 2);
        assert_eq!(
            after_worse.best_score,
            Some(70.0),
            "a worse attempt must not lower the best"
        );

        store.record_attempt(id, 95.0).unwrap();
        assert_eq!(store.entries()[0].best_score, Some(95.0));
    }

    #[test]
    fn attempt_scores_are_clamped() {
        let mut store = VocabStore::in_memory();
        let id = store.add_entry("好", "", "", None).unwrap().id;
        store.record_attempt(id, 120.0).unwrap();
        assert_eq!(store.entries()[0].best_score, Some(100.0));
    }

    #[test]
    fn missing_ids_are_reported() {
        let mut store = VocabStore::in_memory();
        assert!(matches!(
            store.record_attempt(99, 50.0),
            Err(VocabError::NoSuchEntry(99))
        ));
        assert!(matches!(
            store.remove_entry(99),
            Err(VocabError::NoSuchEntry(99))
        ));
        assert!(matches!(
            store.update_entry(99, "", "", None),
            Err(VocabError::NoSuchEntry(99))
        ));
    }

    #[test]
    fn updating_moves_an_entry_between_groups() {
        let mut store = VocabStore::in_memory();
        let id = store.add_entry("好", "hǎo", "good", Some("A")).unwrap().id;
        store.update_entry(id, "hào", "to like", Some("B")).unwrap();

        let entry = &store.entries()[0];
        assert_eq!(entry.pinyin, "hào");
        assert_eq!(entry.meaning, "to like");
        assert_eq!(entry.group.as_deref(), Some("B"));
        // The new group is created on demand, the old one is left in place.
        assert!(store.groups().contains(&"B".to_string()));
    }

    /// Moving an entry onto a word already in the target group is refused, the
    /// rule `add_entry` keeps, and changes nothing (issue #40).
    #[test]
    fn updating_refuses_to_move_an_entry_onto_its_own_duplicate() {
        let mut store = VocabStore::in_memory();
        let filed = store.add_entry("好", "hǎo", "good", Some("A")).unwrap().id;
        store.add_entry("好", "hǎo", "good", Some("B")).unwrap();
        let unfiled = store.add_entry("好", "", "", None).unwrap().id;
        let before = store.document().clone();

        for (id, target) in [(filed, Some("B")), (filed, None), (unfiled, Some("A"))] {
            let refused = store
                .update_entry(id, "hào", "to like", target)
                .unwrap_err();
            assert!(
                matches!(refused, VocabError::DuplicateEntry(ref t) if t == "好"),
                "{refused}"
            );
        }
        assert_eq!(
            store.document().entries,
            before.entries,
            "nothing was edited"
        );
        assert_eq!(store.groups(), before.groups);

        // An entry that stays where it is can still be edited.
        let kept = store
            .update_entry(filed, "hào", "to like", Some("A"))
            .unwrap();
        assert_eq!(kept.meaning, "to like");
    }

    /// A list from before the group check can name a group the list of groups
    /// does not hold. A move refused there must not add the name to that list.
    #[test]
    fn a_refused_move_into_an_unlisted_group_leaves_the_groups_alone() {
        let path = temp_path("unlisted-group");
        fs::write(
            &path,
            r#"{"version":1,"nextId":3,"groups":[],"entries":[
                {"id":1,"text":"好","group":"Ghost","addedAt":"2026-01-01T00:00:00Z"},
                {"id":2,"text":"好","addedAt":"2026-01-01T00:00:01Z"}]}"#,
        )
        .unwrap();
        let mut store = VocabStore::open(&path).unwrap();
        assert!(store.groups().is_empty());
        let refused = store.update_entry(2, "", "", Some("Ghost"));
        assert!(
            matches!(refused, Err(VocabError::DuplicateEntry(_))),
            "{refused:?}"
        );
        assert!(store.groups().is_empty(), "{:?}", store.groups());
        assert_eq!(store.entries_in(None).len(), 1);
        fs::remove_file(&path).ok();
    }

    // ---- groups -----------------------------------------------------------

    #[test]
    fn manages_groups_and_protects_entries() {
        let mut store = VocabStore::in_memory();
        store.add_group("Lesson 3").unwrap();
        assert!(matches!(
            store.add_group("Lesson 3"),
            Err(VocabError::DuplicateGroup(_))
        ));

        let id = store.add_entry("好", "", "", Some("Lesson 3")).unwrap().id;

        // Renaming carries the entries along.
        store.rename_group("Lesson 3", "Lesson 4").unwrap();
        assert!(store.groups().contains(&"Lesson 4".to_string()));
        assert!(!store.groups().contains(&"Lesson 3".to_string()));
        assert_eq!(store.entries()[0].group.as_deref(), Some("Lesson 4"));

        // Renaming onto an existing name is refused rather than merging silently.
        store.add_group("Other").unwrap();
        assert!(matches!(
            store.rename_group("Lesson 4", "Other"),
            Err(VocabError::DuplicateGroup(_))
        ));

        // Removing a group keeps its entries by default — deleting a label must
        // not destroy work.
        let unfiled = store.remove_group("Lesson 4", false).unwrap();
        assert_eq!(unfiled, 1);
        assert_eq!(store.entries().len(), 1);
        assert_eq!(store.entries()[0].group, None);
        assert_eq!(store.entries()[0].id, id);

        // Removing with delete is explicit, and reports the count.
        store.add_group("Doomed").unwrap();
        store.add_entry("坏", "", "", Some("Doomed")).unwrap();
        assert_eq!(store.remove_group("Doomed", true).unwrap(), 1);
        assert_eq!(store.entries().len(), 1);
    }

    /// Unfiling a group's entries never leaves two unfiled copies of one word
    /// (issue #40): an entry whose text is already unfiled is merged into it.
    ///
    /// The one already unfiled survives with its own fields, takes the moved
    /// entry's pinyin and meaning only where its own are blank, and the practice
    /// counters add up, as the sync's collapse of duplicates adds them.
    #[test]
    fn removing_a_group_merges_an_entry_into_its_unfiled_twin() {
        let mut store = VocabStore::in_memory();
        let survivor = store.add_entry("好", "", "good", None).unwrap().id;
        let moved = store.add_entry("好", "hǎo", "fine", Some("L1")).unwrap().id;
        let other = store.add_entry("坏", "huài", "bad", Some("L1")).unwrap().id;
        store.record_attempt(survivor, 40.0).unwrap();
        store.record_attempt(moved, 90.0).unwrap();
        store.record_attempt(moved, 70.0).unwrap();

        assert_eq!(store.remove_group("L1", false).unwrap(), 2);
        assert!(store.groups().is_empty());
        let unfiled = store.entries_in(None);
        assert_eq!(unfiled.len(), 2, "{unfiled:?}");
        let texts: Vec<&str> = unfiled.iter().map(|e| e.text.as_str()).collect();
        assert_eq!(texts.iter().filter(|t| **t == "好").count(), 1);

        let merged = store.entries().iter().find(|e| e.text == "好").unwrap();
        assert_eq!(merged.id, survivor, "the entry already unfiled survives");
        assert_eq!(merged.meaning, "good", "its own meaning is kept");
        assert_eq!(
            merged.pinyin, "hǎo",
            "a blank pinyin is filled from the twin"
        );
        assert_eq!(merged.attempts, 3);
        assert_eq!(merged.best_score, Some(90.0));
        assert!(merged.last_practised.is_some());
        assert!(store.entries().iter().all(|e| e.id != moved));

        let kept = store.entries().iter().find(|e| e.id == other).unwrap();
        assert_eq!(
            kept.group, None,
            "an entry with no twin is unfiled as before"
        );

        // And the result keeps the rule `add_entry` keeps.
        assert!(matches!(
            store.add_entry("好", "", "", None),
            Err(VocabError::DuplicateEntry(_))
        ));
    }

    /// Two copies of one word inside the removed group — only a list from
    /// before the check can hold them — become one unfiled entry: the first in
    /// list order is unfiled, and the next folds into it.
    #[test]
    fn removing_a_group_folds_its_own_duplicates_together() {
        let path = temp_path("duplicates-in-group");
        fs::write(
            &path,
            r#"{"version":1,"nextId":4,"groups":["L1"],"entries":[
                {"id":1,"text":"好","meaning":"good","group":"L1",
                 "addedAt":"2026-01-01T00:00:00Z","attempts":2,"bestScore":60},
                {"id":2,"text":"好","pinyin":"hǎo","group":"L1",
                 "addedAt":"2026-01-01T00:00:01Z","attempts":1,"bestScore":80},
                {"id":3,"text":"坏","group":"L1","addedAt":"2026-01-01T00:00:02Z"}]}"#,
        )
        .unwrap();
        let mut store = VocabStore::open(&path).unwrap();
        assert_eq!(store.remove_group("L1", false).unwrap(), 3);
        assert_eq!(store.entries().len(), 2);
        let good = store.entries().iter().find(|e| e.text == "好").unwrap();
        assert_eq!(good.id, 1, "the earlier copy survives");
        assert_eq!(good.group, None);
        assert_eq!(
            (good.meaning.as_str(), good.pinyin.as_str()),
            ("good", "hǎo")
        );
        assert_eq!(good.attempts, 3);
        assert_eq!(good.best_score, Some(80.0));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn unknown_groups_are_reported() {
        let mut store = VocabStore::in_memory();
        assert!(matches!(
            store.rename_group("nope", "other"),
            Err(VocabError::NoSuchGroup(_))
        ));
        assert!(matches!(
            store.remove_group("nope", false),
            Err(VocabError::NoSuchGroup(_))
        ));
    }

    #[test]
    fn selecting_a_practice_set() {
        let mut store = VocabStore::in_memory();
        store.add_entry("好", "", "", Some("A")).unwrap();
        store.add_entry("坏", "", "", Some("B")).unwrap();
        store.add_entry("中", "", "", None).unwrap();

        assert_eq!(store.practice_set(None).len(), 3, "no group means everything");
        assert_eq!(store.practice_set(Some("A")).len(), 1);
        assert_eq!(store.practice_set(Some("B"))[0].text, "坏");
        assert!(store.practice_set(Some("missing")).is_empty());
    }

    // ---- persistence ------------------------------------------------------

    #[test]
    fn round_trips_through_a_file() {
        let path = temp_path("round-trip");
        let id = {
            let mut store = VocabStore::open(&path).unwrap();
            let id = store
                .add_entry("学习", "xuéxí", "to study", Some("Lesson 3"))
                .unwrap()
                .id;
            store.record_attempt(id, 88.0).unwrap();
            store.save().unwrap();
            id
        };

        let reopened = VocabStore::open(&path).unwrap();
        assert_eq!(reopened.entries().len(), 1);
        assert_eq!(reopened.groups(), ["Lesson 3"]);
        let entry = &reopened.entries()[0];
        assert_eq!(entry.id, id);
        assert_eq!(entry.text, "学习");
        assert_eq!(entry.meaning, "to study");
        assert_eq!(entry.attempts, 1);
        assert_eq!(entry.best_score, Some(88.0));

        // Ids keep climbing across sessions, so a new entry cannot collide with
        // one that was deleted before the restart.
        let mut reopened = reopened;
        let next = reopened.add_entry("好", "", "", None).unwrap();
        assert!(next.id > id);

        fs::remove_file(&path).ok();
    }

    #[test]
    fn a_missing_file_is_an_empty_list_not_an_error() {
        let path = temp_path("absent");
        let store = VocabStore::open(&path).unwrap();
        assert!(store.entries().is_empty());
        assert!(store.groups().is_empty());
    }

    #[test]
    fn a_corrupt_file_is_an_error_rather_than_silent_data_loss() {
        let path = temp_path("corrupt");
        fs::write(&path, "{ this is not json").unwrap();
        let error = VocabStore::open(&path).unwrap_err();
        assert!(matches!(error, VocabError::Malformed(_)), "{error}");

        // Truncated mid-document must also be caught.
        fs::write(&path, r#"{"version":1,"nextId":3,"entries":[{"id":1,"#).unwrap();
        assert!(matches!(
            VocabStore::open(&path).unwrap_err(),
            VocabError::Malformed(_)
        ));

        fs::remove_file(&path).ok();
    }

    #[test]
    fn a_future_format_version_is_refused() {
        let path = temp_path("future");
        fs::write(
            &path,
            format!(r#"{{"version":{},"nextId":1,"groups":[],"entries":[]}}"#, FORMAT_VERSION + 1),
        )
        .unwrap();
        assert!(matches!(
            VocabStore::open(&path).unwrap_err(),
            VocabError::UnsupportedVersion(_)
        ));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn saving_creates_missing_directories() {
        let dir = temp_path("nested").with_extension("dir");
        let path = dir.join("deeper/vocabulary.json");
        let mut store = VocabStore::open(&path).unwrap();
        store.add_entry("好", "", "", None).unwrap();
        store.save().unwrap();
        assert!(path.exists());
        assert_eq!(VocabStore::open(&path).unwrap().entries().len(), 1);
        fs::remove_dir_all(&dir).ok();
    }

    // ---- export and import ------------------------------------------------

    #[test]
    fn exports_and_imports_losslessly() {
        let mut source = VocabStore::in_memory();
        let id = source.add_entry("学习", "xuéxí", "to study", Some("L3")).unwrap().id;
        source.record_attempt(id, 91.0).unwrap();
        let json = source.export_json().unwrap();

        let mut target = VocabStore::in_memory();
        let summary = target.import_json(&json, false).unwrap();
        assert_eq!(summary.added, 1);
        assert!(summary.replaced);
        assert_eq!(target.entries().len(), 1);
        assert_eq!(target.entries()[0].text, "学习");
        assert_eq!(target.entries()[0].attempts, 1);
        assert_eq!(target.entries()[0].best_score, Some(91.0));
        assert_eq!(target.groups(), ["L3"]);
    }

    #[test]
    fn merging_skips_duplicates_and_unions_groups() {
        let mut target = VocabStore::in_memory();
        target.add_entry("好", "hǎo", "good", Some("A")).unwrap();

        let mut source = VocabStore::in_memory();
        source.add_entry("好", "hǎo", "good", Some("A")).unwrap(); // duplicate
        source.add_entry("坏", "huài", "bad", Some("B")).unwrap(); // new
        let json = source.export_json().unwrap();

        let summary = target.import_json(&json, true).unwrap();
        assert!(!summary.replaced);
        assert_eq!(summary.added, 1);
        assert_eq!(summary.skipped_duplicates, 1);
        assert_eq!(target.entries().len(), 2);
        assert_eq!(target.groups(), ["A", "B"]);
    }

    #[test]
    fn importing_over_existing_entries_replaces_them() {
        let mut target = VocabStore::in_memory();
        target.add_entry("旧", "", "", Some("Old")).unwrap();

        let mut source = VocabStore::in_memory();
        source.add_entry("新", "", "", Some("New")).unwrap();
        let json = source.export_json().unwrap();

        target.import_json(&json, false).unwrap();
        assert_eq!(target.entries().len(), 1);
        assert_eq!(target.entries()[0].text, "新");
        assert_eq!(target.groups(), ["New"], "old groups are cleared too");
    }

    #[test]
    fn imported_ids_never_collide_with_existing_ones() {
        let mut store = VocabStore::in_memory();
        let original = store.add_entry("好", "", "", None).unwrap().id;

        // Hand-written import that reuses id 1.
        let json = r#"{"version":1,"nextId":2,"groups":[],"entries":[
            {"id":1,"text":"坏","addedAt":"2026-01-01T00:00:00Z"}]}"#;
        store.import_json(json, true).unwrap();

        let ids: Vec<u64> = store.entries().iter().map(|e| e.id).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&original));
        assert_eq!(ids.iter().collect::<std::collections::HashSet<_>>().len(), 2);
    }

    #[test]
    fn malformed_imports_are_rejected() {
        let mut store = VocabStore::in_memory();
        store.add_entry("好", "", "", None).unwrap();
        assert!(matches!(
            store.import_json("nonsense", true),
            Err(VocabError::Malformed(_))
        ));
        assert!(matches!(
            store.import_json(
                &format!(r#"{{"version":{},"nextId":1}}"#, FORMAT_VERSION + 1),
                true
            ),
            Err(VocabError::UnsupportedVersion(_))
        ));
        // A rejected import must leave the list untouched.
        assert_eq!(store.entries().len(), 1);
    }

    /// An import is held to the rules typing an entry is (issue #40): a row that
    /// breaks one is skipped and counted, and the rest of the file still arrives.
    #[test]
    fn importing_skips_and_counts_rows_that_break_the_rules() {
        let long = "字".repeat(MAX_TEXT_CHARS + 1);
        let long_group = "g".repeat(MAX_GROUP_CHARS + 1);
        let json = serde_json::json!({
            "version": 1,
            "nextId": 99,
            "groups": ["L1", "", "  ", long_group, "bad\u{7}group", "  L2  "],
            "entries": [
                {"id": 1, "text": "好", "group": "L1", "addedAt": "2026-01-01T00:00:00Z"},
                {"id": 2, "text": "", "addedAt": "2026-01-01T00:00:00Z"},
                {"id": 3, "text": "   ", "addedAt": "2026-01-01T00:00:00Z"},
                {"id": 4, "text": long, "addedAt": "2026-01-01T00:00:00Z"},
                {"id": 5, "text": "a\u{0}b", "addedAt": "2026-01-01T00:00:00Z"},
                {"id": 6, "text": "坏", "group": long_group, "addedAt": "2026-01-01T00:00:00Z"},
                {"id": 7, "text": "中", "group": "bad\ngroup", "addedAt": "2026-01-01T00:00:00Z"},
                // Blank is unfiled, not a group named "".
                {"id": 8, "text": "  人 ", "group": "", "pinyin": " rén ",
                 "addedAt": "2026-01-01T00:00:00Z"},
                {"id": 9, "text": "大", "group": "  L2 ", "addedAt": "2026-01-01T00:00:00Z"},
            ]
        })
        .to_string();

        for merge in [false, true] {
            let mut store = VocabStore::in_memory();
            let summary = store.import_json(&json, merge).unwrap();
            // 好, 人 and 大 as they were; 坏 and 中 kept, but under no group,
            // because only their group names were at fault.
            assert_eq!(summary.added, 5, "{summary:?}");
            assert_eq!(summary.skipped_invalid, 4, "{summary:?}");
            assert_eq!(summary.unfiled_invalid_group, 2, "{summary:?}");
            assert_eq!(summary.skipped_invalid_groups, 4, "{summary:?}");
            assert_eq!(summary.skipped_duplicates, 0);
            assert_eq!(store.groups(), ["L1", "L2"]);
            let person = store.entries().iter().find(|e| e.text == "人").unwrap();
            assert_eq!(person.group, None);
            assert_eq!(person.pinyin, "rén");
            assert_eq!(store.entries_in(Some("L2"))[0].text, "大");
            for refiled in ["坏", "中"] {
                let entry = store.entries().iter().find(|e| e.text == refiled).unwrap();
                assert_eq!(entry.group, None, "{entry:?}");
            }
            for entry in store.entries() {
                assert!(validate_text(&entry.text).is_ok(), "{entry:?}");
                assert_eq!(
                    validate_group(entry.group.as_deref()).unwrap(),
                    entry.group,
                    "{entry:?}"
                );
            }
        }

        // Restoring the app's own data keeps what is only too long: earlier
        // versions, the sync and the migration never checked length.
        let mut store = VocabStore::in_memory();
        let summary = store
            .import_json_with(&json, false, ImportRules::Restore)
            .unwrap();
        assert_eq!(summary.added, 6, "{summary:?}");
        assert_eq!(summary.skipped_invalid, 3, "{summary:?}");
        assert_eq!(summary.unfiled_invalid_group, 1, "{summary:?}");
        assert_eq!(summary.skipped_invalid_groups, 3, "{summary:?}");
        assert!(store.entries().iter().any(|e| e.text == long));
        assert_eq!(store.entries_in(Some(&long_group))[0].text, "坏");
        assert_eq!(store.groups(), ["L1", "L2", long_group.as_str()]);
        assert_eq!(
            store
                .entries()
                .iter()
                .find(|e| e.text == "中")
                .unwrap()
                .group,
            None
        );
    }

    /// A replace is checked before anything is cleared: one that would leave
    /// nothing because every entry broke the rules is refused, untouched.
    #[test]
    fn a_replace_that_the_rules_would_empty_is_refused() {
        let json = r#"{"version":1,"nextId":3,"groups":["New"],"entries":[
            {"id":1,"text":"","addedAt":"2026-01-01T00:00:00Z"},
            {"id":2,"text":"a\u0007","group":"New","addedAt":"2026-01-01T00:00:00Z"}]}"#;
        for rules in [ImportRules::Typed, ImportRules::Restore] {
            let mut store = VocabStore::in_memory();
            store.add_entry("好", "", "", Some("Old")).unwrap();
            let before = store.document().clone();
            let refused = store.import_json_with(json, false, rules).unwrap_err();
            assert!(
                matches!(refused, VocabError::NothingUsable { count: 2, .. }),
                "{refused}"
            );
            assert_eq!(store.document().entries, before.entries);
            assert_eq!(store.groups(), before.groups);

            // Adding nothing is no loss, so a merge goes ahead and says so.
            let summary = store.import_json_with(json, true, rules).unwrap();
            assert_eq!((summary.added, summary.skipped_invalid), (0, 2));
            assert_eq!(store.entries().len(), 1);
        }

        // The refusal says what is wrong, in the words of the rules, and in
        // the singular for one.
        let typed = VocabError::NothingUsable {
            count: 2,
            rules: ImportRules::Typed,
        };
        assert_eq!(
            typed.to_string(),
            "none of the 2 entries in that file can be used — each is empty, too long or \
             holds a control character — so your list was left as it was"
        );
        let one = VocabError::NothingUsable {
            count: 1,
            rules: ImportRules::Restore,
        };
        assert_eq!(
            one.to_string(),
            "the one entry in that file cannot be used — it is empty or holds a control \
             character — so your list was left as it was"
        );

        // A file that is empty on purpose still replaces the list.
        let mut store = VocabStore::in_memory();
        store.add_entry("好", "", "", None).unwrap();
        store
            .import_json(r#"{"version":1,"nextId":1}"#, false)
            .unwrap();
        assert!(store.entries().is_empty());
    }

    /// Two entries of one file filed under no group by the import itself,
    /// because their group names broke the rules, are one word in one place —
    /// not a repeat in the file — so they are merged, counters added up, as
    /// `remove_group` merges. Landing on a word the list already held is an
    /// ordinary duplicate, so a second import of the file counts nothing twice.
    #[test]
    fn entries_refiled_onto_one_another_are_merged_not_skipped() {
        let first = "a".repeat(MAX_GROUP_CHARS + 1);
        let second = "b".repeat(MAX_GROUP_CHARS + 1);
        let json = serde_json::json!({
            "version": 1,
            "nextId": 9,
            "entries": [
                {"id": 1, "text": "好", "group": first, "meaning": "good",
                 "addedAt": "2026-01-01T00:00:00Z", "attempts": 2, "bestScore": 60},
                {"id": 2, "text": "好", "group": second, "pinyin": "hǎo",
                 "addedAt": "2026-01-01T00:00:01Z", "attempts": 3, "bestScore": 90},
                {"id": 3, "text": "坏", "group": first, "attempts": 1,
                 "addedAt": "2026-01-01T00:00:02Z"},
            ]
        })
        .to_string();

        let mut store = VocabStore::in_memory();
        store.add_entry("坏", "", "", None).unwrap();
        let summary = store.import_json(&json, true).unwrap();
        assert_eq!(summary.added, 1, "{summary:?}");
        assert_eq!(summary.unfiled_invalid_group, 1, "{summary:?}");
        assert_eq!(summary.merged_unfiled, 1, "{summary:?}");
        assert_eq!(summary.skipped_duplicates, 1, "{summary:?}");
        assert_eq!(store.entries().len(), 2);
        let good = store.entries().iter().find(|e| e.text == "好").unwrap();
        assert_eq!(good.group, None);
        assert_eq!(
            (good.meaning.as_str(), good.pinyin.as_str()),
            ("good", "hǎo")
        );
        assert_eq!((good.attempts, good.best_score), (5, Some(90.0)));
        let bad = store.entries().iter().find(|e| e.text == "坏").unwrap();
        assert_eq!(bad.attempts, 0, "the list's own entry is left as it was");

        let summary = store.import_json(&json, true).unwrap();
        assert_eq!((summary.merged_unfiled, summary.skipped_duplicates), (0, 3));
        let good = store.entries().iter().find(|e| e.text == "好").unwrap();
        assert_eq!(good.attempts, 5, "a second import counts nothing twice");
    }

    /// The practice fields arrive sane: a best score inside `0..=100`, or none
    /// when it is not a number; a time that is not one is not kept as one.
    #[test]
    fn imported_practice_fields_are_made_sane() {
        let json = r#"{"version":1,"nextId":9,"entries":[
            {"id":1,"text":"一","addedAt":"2026-01-01T00:00:00Z","bestScore":150,
             "lastPractised":"2026-01-02T00:00:00Z"},
            {"id":2,"text":"二","addedAt":"yesterday","bestScore":-5,"lastPractised":"never"},
            {"id":3,"text":"三","addedAt":"2026-01-01T00:00:00Z","bestScore":1e300},
            {"id":4,"text":"四","addedAt":"2026-01-01T00:00:00Z","attempts":4294967295}]}"#;
        let mut store = VocabStore::in_memory();
        store.import_json(json, false).unwrap();
        let find = |text: &str| {
            store
                .entries()
                .iter()
                .find(|e| e.text == text)
                .unwrap()
                .clone()
        };
        assert_eq!(find("一").best_score, Some(100.0));
        assert_eq!(
            find("一").last_practised.as_deref(),
            Some("2026-01-02T00:00:00Z")
        );
        assert_eq!(find("二").best_score, Some(0.0));
        assert!(crate::time::parse_iso8601(&find("二").added_at).is_some());
        assert_eq!(find("二").last_practised, None);
        assert_eq!(
            find("三").best_score,
            None,
            "too large for an f32 is no score"
        );

        // A count already at the top saturates rather than overflowing.
        let four = find("四").id;
        assert_eq!(store.record_attempt(four, 50.0).unwrap().attempts, u32::MAX);
    }

    /// One word in one group once, however it arrives (issue #40): twice in
    /// the same file, or already in the list, and spelt with stray spaces or not.
    #[test]
    fn importing_never_makes_a_duplicate() {
        let json = r#"{"version":1,"nextId":9,"groups":["A"],"entries":[
            {"id":1,"text":"好","group":"A","addedAt":"2026-01-01T00:00:00Z"},
            {"id":2,"text":" 好 ","group":"A ","addedAt":"2026-01-01T00:00:01Z"},
            {"id":3,"text":"好","addedAt":"2026-01-01T00:00:02Z"},
            {"id":4,"text":"好 ","group":"","addedAt":"2026-01-01T00:00:03Z"},
            {"id":5,"text":"坏","addedAt":"2026-01-01T00:00:04Z"}]}"#;

        // Replacing: only the file's own repeats are skipped.
        let mut store = VocabStore::in_memory();
        store.add_entry("坏", "", "", None).unwrap();
        let summary = store.import_json(json, false).unwrap();
        assert_eq!(
            (summary.added, summary.skipped_duplicates),
            (3, 2),
            "{summary:?}"
        );

        // Adding: what is already there is skipped too.
        let mut store = VocabStore::in_memory();
        store.add_entry("坏", "", "", None).unwrap();
        store.add_entry("好", "", "", Some("A")).unwrap();
        let summary = store.import_json(json, true).unwrap();
        assert_eq!(
            (summary.added, summary.skipped_duplicates),
            (1, 4),
            "{summary:?}"
        );

        let mut seen = std::collections::HashSet::new();
        for entry in store.entries() {
            assert!(
                seen.insert((entry.text.clone(), entry.group.clone())),
                "{entry:?}"
            );
        }
    }

    #[test]
    fn csv_export_quotes_awkward_fields() {
        let mut store = VocabStore::in_memory();
        store
            .add_entry("好", "hǎo", "good, \"fine\"", Some("Lesson, 3"))
            .unwrap();
        let csv = store.export_csv();

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2, "header plus one row: {csv}");
        assert!(lines[0].starts_with("text,pinyin,meaning"));
        assert!(lines[1].contains("\"good, \"\"fine\"\"\""));
        assert!(lines[1].contains("\"Lesson, 3\""));
    }

    #[test]
    fn a_field_a_spreadsheet_would_run_is_defused() {
        // The reported bug, through the real route in: a shared list is
        // imported, exported as CSV for a spreadsheet, and the formula in it
        // runs in *that* reader. Quoting is not neutralising — CSV quoting is
        // stripped on load, so `"=1+1"` is still a formula — and the pinyin and
        // meaning columns below have no comma of their own, so they would not
        // even be quoted.
        //
        // Imported, the way a shared list arrives. Since issue #40 the import
        // holds text to the rules `add_entry` does, and a formula short enough to
        // be an entry passes them — nothing about `=` is invalid in a word list —
        // so the export is still the boundary that has to defuse it.
        let hostile = serde_json::json!({
            "version": FORMAT_VERSION,
            "nextId": 2,
            "groups": ["-2+3"],
            "entries": [{
                "id": 1,
                "text": "=HYPERLINK(\"e.x\",A1)",
                "pinyin": "+1+1",
                "meaning": "@SUM(1+1)*cmd|' /c calc'!A0",
                "group": "-2+3",
                "addedAt": "2026-01-01T00:00:00Z",
            }],
        });
        let mut store = VocabStore::in_memory();
        store.import_json(&hostile.to_string(), false).unwrap();
        let csv = store.export_csv();
        let row = csv.lines().nth(1).unwrap();

        // Every dangerous field is preceded by the text marker, and the tab and
        // carriage return spellings are markers too. The first is quoted as well
        // — it contains commas of its own — which is why the marker is asserted
        // on the field rather than on the start of the line.
        assert!(row.contains("'=HYPERLINK"), "the character is defused: {row}");
        assert!(row.contains("'+1+1"), "the pinyin is defused: {row}");
        assert!(row.contains("'@SUM"), "the meaning is defused: {row}");
        assert!(row.contains("'-2+3"), "the group is defused: {row}");
        assert_eq!(csv_field("=1+1"), "'=1+1");
        assert_eq!(csv_field("\tcmd"), "'\tcmd");
        assert_eq!(csv_field("\rcmd"), "\"'\rcmd\"");

        // An ordinary field is untouched: the marker is not a tax on everything.
        assert_eq!(csv_field("好"), "好");
        assert_eq!(csv_field(""), "");
        assert_eq!(csv_field("good, \"fine\""), "\"good, \"\"fine\"\"\"");
        // And a field that only *contains* a dangerous character is safe: the
        // formula rule is about the first character, which is what a spreadsheet
        // looks at.
        assert_eq!(csv_field("a=1"), "a=1");
    }

    // ---- ordering ---------------------------------------------------------

    #[test]
    fn entries_are_grouped_and_ordered_deterministically() {
        let mut store = VocabStore::in_memory();
        store.add_entry("三", "", "", Some("B")).unwrap();
        store.add_entry("一", "", "", Some("A")).unwrap();
        store.add_entry("二", "", "", Some("B")).unwrap();
        store.add_entry("零", "", "", None).unwrap();

        let order: Vec<&str> = store.entries().iter().map(|e| e.text.as_str()).collect();
        // Unfiled first (empty group key), then A, then B, each in insertion order.
        assert_eq!(order, ["零", "一", "三", "二"]);
        assert_eq!(store.groups(), ["A", "B"]);
    }
}

/// The one order the list is kept in, on every device.
///
/// A free function so the rule can be tested on its own, and so the reason it is
/// not `id` has somewhere to live: an `id` is per device, and a peer's entries
/// arrive in the order the merge produced them, so ordering by id showed one
/// list in a different order on every device. `added_at` syncs and sorts
/// chronologically; the text settles two entries added inside the same second.
fn entry_order(a: &Entry, b: &Entry) -> std::cmp::Ordering {
    a.group
        .as_deref()
        .unwrap_or("")
        .cmp(b.group.as_deref().unwrap_or(""))
        .then_with(|| a.added_at.cmp(&b.added_at))
        .then_with(|| a.text.cmp(&b.text))
}

#[cfg(test)]
mod order_tests {
    use super::*;

    fn entry(id: u64, text: &str, group: Option<&str>, added_at: &str) -> Entry {
        Entry {
            id,
            text: text.to_string(),
            pinyin: String::new(),
            meaning: String::new(),
            group: group.map(str::to_string),
            added_at: added_at.to_string(),
            attempts: 0,
            best_score: None,
            last_practised: None,
        }
    }

    #[test]
    fn the_list_is_ordered_by_when_entries_were_added_not_by_id() {
        // The bug this fixes: two devices holding the same two entries in
        // opposite id order showed the list in opposite order. `added_at` is the
        // same on both, so the order is too.
        let older = entry(9, "学生", Some("SiLu"), "2026-09-20T09:00:00Z");
        let newer = entry(2, "姐姐", Some("SiLu"), "2026-09-21T09:00:00Z");
        assert_eq!(entry_order(&older, &newer), std::cmp::Ordering::Less);
        assert_eq!(entry_order(&newer, &older), std::cmp::Ordering::Greater);

        let mut rows = [
            entry(1, "z", Some("B"), "2026-09-20T00:00:00Z"),
            entry(2, "a", Some("B"), "2026-09-21T00:00:00Z"),
            entry(3, "m", Some("A"), "2026-09-22T00:00:00Z"),
            entry(4, "n", None, "2026-09-19T00:00:00Z"),
        ];
        // An array rather than a `Vec`: the size is part of the test.
        rows.sort_by(entry_order);
        let order: Vec<&str> = rows.iter().map(|e| e.text.as_str()).collect();
        // Unfiled first (the empty group name), then by group name, then by time
        // within the group — never by id.
        assert_eq!(order, ["n", "m", "z", "a"]);
    }

    #[test]
    fn two_entries_added_in_the_same_second_have_a_stable_order() {
        // `added_at` is whole seconds, so a tie is real; the text settles it, and
        // both devices settle it the same way.
        let a = entry(5, "中国", Some("SiLu"), "2026-09-20T09:00:00Z");
        let b = entry(6, "学生", Some("SiLu"), "2026-09-20T09:00:00Z");
        // 中 (U+4E2D) sorts before 学 (U+5B66), so the tie is settled on the
        // text and not on either id.
        assert_eq!(entry_order(&a, &b), std::cmp::Ordering::Less);
        assert_eq!(entry_order(&b, &a), std::cmp::Ordering::Greater);
    }
}
