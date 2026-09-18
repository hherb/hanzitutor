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
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

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

/// What the interface needs to render the list.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabView {
    pub entries: Vec<Entry>,
    pub groups: Vec<String>,
    /// Set when the change was applied in memory but could not be saved, so the
    /// interface can say so instead of silently losing data.
    #[serde(default)]
    pub warning: Option<String>,
}

/// The persisted document.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Document {
    version: u32,
    /// Ids are allocated from here, so they are never reused after a deletion.
    next_id: u64,
    #[serde(default)]
    groups: Vec<String>,
    #[serde(default)]
    entries: Vec<Entry>,
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
    pub groups_added: usize,
    /// True when the import replaced the existing list rather than merging.
    pub replaced: bool,
}

/// The vocabulary list, backed by a JSON file.
#[derive(Debug)]
pub struct VocabStore {
    path: PathBuf,
    document: Document,
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
        let mut store = Self { path, document };
        store.sort();
        Ok(store)
    }

    /// An in-memory list with no file behind it. Used by tests.
    pub fn in_memory() -> Self {
        Self {
            path: PathBuf::new(),
            document: Document::default(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn view(&self) -> VocabView {
        VocabView {
            entries: self.document.entries.clone(),
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

    /// Write the document to disk, atomically.
    ///
    /// Writes to a temporary file and renames, so an interrupted write cannot
    /// leave a half-written list behind.
    pub fn save(&self) -> Result<(), VocabError> {
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
            pinyin: pinyin.trim().to_string(),
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
    pub fn update_entry(
        &mut self,
        id: u64,
        pinyin: &str,
        meaning: &str,
        group: Option<&str>,
    ) -> Result<Entry, VocabError> {
        let group = validate_group(group)?;
        // A rename to a non-empty group should not require creating it first.
        if let Some(name) = &group {
            self.ensure_group(name);
        }
        let entry = self
            .document
            .entries
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or(VocabError::NoSuchEntry(id))?;
        entry.pinyin = pinyin.trim().to_string();
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
    /// is the safe default — deleting a label should not destroy work.
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
            for entry in self.document.entries.iter_mut() {
                if entry.group.as_deref() == Some(name) {
                    entry.group = None;
                    unfiled += 1;
                }
            }
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
        entry.attempts += 1;
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

    /// Merge or replace from an exported document.
    ///
    /// Merging keeps existing entries, adds the new ones and unions the groups;
    /// an imported entry whose text already exists in the same group is skipped
    /// rather than duplicated.
    pub fn import_json(&mut self, json: &str, merge: bool) -> Result<ImportSummary, VocabError> {
        let incoming: Document =
            serde_json::from_str(json).map_err(|e| VocabError::Malformed(e.to_string()))?;
        if incoming.version > FORMAT_VERSION {
            return Err(VocabError::UnsupportedVersion(incoming.version));
        }

        let mut summary = ImportSummary {
            replaced: !merge,
            ..Default::default()
        };

        if !merge {
            self.document.entries.clear();
            self.document.groups.clear();
        }

        for name in incoming.groups {
            if !self.document.groups.contains(&name) {
                self.document.groups.push(name);
                summary.groups_added += 1;
            }
        }

        for mut entry in incoming.entries {
            let duplicate = self
                .document
                .entries
                .iter()
                .any(|e| e.text == entry.text && e.group == entry.group);
            if duplicate {
                summary.skipped_duplicates += 1;
                continue;
            }
            if let Some(name) = entry.group.clone() {
                self.ensure_group(&name);
            }
            // Ids are reallocated so an import can never collide with, or
            // resurrect, an id this document has already used.
            entry.id = self.document.next_id;
            self.document.next_id += 1;
            self.document.entries.push(entry);
            summary.added += 1;
        }

        self.sort();
        Ok(summary)
    }

    fn ensure_group(&mut self, name: &str) {
        if !self.document.groups.iter().any(|g| g == name) {
            self.document.groups.push(name.to_string());
        }
    }

    /// Keep groups alphabetical and entries grouped together, in the order they
    /// were added. Deterministic ordering keeps the file diffable.
    fn sort(&mut self) {
        self.document.groups.sort();
        self.document.groups.dedup();
        self.document.entries.sort_by(|a, b| {
            let key = |e: &Entry| (e.group.clone().unwrap_or_default(), e.id);
            key(a).cmp(&key(b))
        });
    }
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

/// Quote a CSV field when it contains a comma, quote or newline.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// The current time as an ISO-8601 UTC string, e.g. `2026-09-19T00:12:34Z`.
///
/// Implemented directly rather than pulling in a date library: only formatting
/// is ever needed, and UTC timestamps in this format sort chronologically as
/// plain strings, which is what the review scheduling will rely on.
pub fn now_iso8601() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    iso8601_from_unix(seconds)
}

/// Format a Unix timestamp as ISO-8601 UTC.
pub fn iso8601_from_unix(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let secs_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second) = (
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60,
    );
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Convert a count of days since the Unix epoch to a civil date.
///
/// Howard Hinnant's `civil_from_days`, which is exact for the whole range of
/// `i64` days.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("hanzi-vocab-{name}-{unique}.json"))
    }

    // ---- timestamps -------------------------------------------------------

    #[test]
    fn formats_known_timestamps() {
        assert_eq!(iso8601_from_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601_from_unix(946_684_800), "2000-01-01T00:00:00Z");
        assert_eq!(iso8601_from_unix(1_234_567_890), "2009-02-13T23:31:30Z");
        // 2000 was a leap year, so this date exists.
        assert_eq!(iso8601_from_unix(951_782_400), "2000-02-29T00:00:00Z");
        // And a time of day, to check the division.
        assert_eq!(iso8601_from_unix(86_399), "1970-01-01T23:59:59Z");
    }

    #[test]
    fn timestamps_sort_chronologically_as_strings() {
        let earlier = iso8601_from_unix(1_000_000_000);
        let later = iso8601_from_unix(1_700_000_000);
        assert!(earlier < later, "{earlier} should sort before {later}");

        // Across a year boundary, where a naive format would break.
        let new_year = iso8601_from_unix(1_704_067_200); // 2024-01-01
        let old_year = iso8601_from_unix(1_701_000_000); // 2023-11-24
        assert!(old_year < new_year);
    }

    #[test]
    fn now_is_plausible() {
        let now = now_iso8601();
        assert_eq!(now.len(), 20, "unexpected format: {now}");
        assert!(now.starts_with("20"), "unexpected year: {now}");
        assert!(now.ends_with('Z'));
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
