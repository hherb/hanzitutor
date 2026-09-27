//! One file holding everything a learner has made: their list and their log.
//!
//! The vocabulary list and the attempt log are separate stores with separate
//! exports, and both of those exports are *single-purpose*: the list's JSON is
//! for moving a list, and the log's JSON Lines is for analysing attempts. Neither
//! is a backup, and offering one as though it were left the learner exporting a
//! file, restoring it, and finding half their work still on the old device.
//!
//! So this is the format that round-trips, and it is deliberately a different
//! file: both documents in one, under a `format` marker that makes a wrong file
//! an error rather than a silently empty restore.
//!
//! ## What is in it, and what is not
//!
//! In: every vocabulary entry and group, and every attempt with its measures.
//! Out: the review schedule, the course position and the preferences. The
//! schedule is **derived** — `hanzi_sync::recompute` folds it back out of the
//! imported log — so storing it as well would be a second copy that can disagree
//! with the first. The other two are cheap to set again and are not what this
//! milestone was asked for; see ROADMAP "A backup of the list and the log".
//!
//! ## Why the vocabulary document is embedded rather than re-modelled
//!
//! Its shape belongs to `hanzi-core`, and it already has a reader and a writer
//! that round-trip it — [`hanzi_core::VocabStore::import_json`]. Copying that
//! shape into a struct here would make this crate a second owner of a format it
//! does not own, and the two would drift. So the document travels as opaque JSON
//! under `vocabulary`, and the engine's own reader is what reads it.

use hanzi_core::Rating;
use serde::{Deserialize, Serialize};

use crate::export::ExportedAttempt;
use crate::{IncomingAttempt, LoggedAttempt};

/// The `format` marker every backup carries. The first thing [`Backup::parse`]
/// looks at, so that a vocabulary export or an unrelated JSON file is refused by
/// name rather than imported as an empty backup.
pub const BACKUP_FORMAT: &str = "hanzi-tutor-backup";

/// The version of the *combined document*, which is not the version of either
/// document inside it. A build that adds a field bumps this; a build that renames
/// one in the vocabulary list does not, because the list carries its own version.
pub const BACKUP_VERSION: u32 = 1;

/// Everything the learner has made, as one document.
///
/// Fields are private and read through the methods below so that a `Backup` that
/// exists has already passed the `format` and `version` checks in
/// [`Backup::parse`]: nothing downstream has to ask again whether the file is
/// really one of ours.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    /// Always [`BACKUP_FORMAT`]. Written from the constant, checked on the way in.
    format: String,
    version: u32,
    exported_at: String,
    /// The vocabulary list verbatim, exactly as [`hanzi_core::VocabStore::export_json`]
    /// writes it. Opaque here by design — see the module note.
    vocabulary: serde_json::Value,
    attempts: Vec<ExportedAttempt>,
}

/// Why a file could not be read as a backup.
///
/// Separate variants rather than one string because the *next move* differs: the
/// wrong file wants another file, a newer backup wants a newer app, and a
/// malformed one is worth looking at before trying again.
#[derive(Debug)]
pub enum BackupError {
    /// Not JSON, or JSON that is not shaped like this document.
    Malformed(String),
    /// A `format` marker this build did not write.
    NotABackup(String),
    /// The vocabulary list's own export, which is a real file and a common
    /// mistake to pick — it has an import of its own, on the vocabulary screen.
    VocabularyOnly,
    /// The practice log's own export (JSON Lines or CSV), which is a real file
    /// too, and the other half of the same mistake.
    LogOnly,
    /// Written by a newer build.
    UnsupportedVersion(u32),
    /// The embedded list could not be handed to the list's reader.
    Vocabulary(String),
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "that file is not a backup: {why}"),
            Self::NotABackup(format) => write!(
                f,
                "that file says it is {format:?}, which is not a Hanzi Tutor backup"
            ),
            Self::VocabularyOnly => write!(
                f,
                "that is a vocabulary-list export, not a backup. Use Import on the \
                 vocabulary screen for it; a backup also carries your practice log."
            ),
            Self::LogOnly => write!(
                f,
                "that is your practice log's own export, not a backup. It holds your \
                 attempts but not your vocabulary, and Restore cannot read it — use \
                 Back up… to write a file Restore can read."
            ),
            Self::UnsupportedVersion(v) => write!(
                f,
                "that backup was written by a newer version of the app \
                 (backup format {v}, this build understands {BACKUP_VERSION})"
            ),
            Self::Vocabulary(v) => write!(f, "the vocabulary list in that backup is unusable: {v}"),
        }
    }
}

impl std::error::Error for BackupError {}

impl Backup {
    /// Gather a list and a log into one document.
    ///
    /// `vocabulary_json` is [`hanzi_core::VocabStore::export_json`]'s output, and
    /// `exported_at` is ISO-8601 UTC. Both are arguments rather than read here,
    /// so that what the document looks like is testable without a database.
    pub fn build(
        exported_at: &str,
        vocabulary_json: &str,
        attempts: &[LoggedAttempt],
    ) -> Result<Self, BackupError> {
        let vocabulary: serde_json::Value = serde_json::from_str(vocabulary_json)
            .map_err(|e| BackupError::Vocabulary(e.to_string()))?;
        Ok(Self {
            format: BACKUP_FORMAT.to_string(),
            version: BACKUP_VERSION,
            exported_at: exported_at.to_string(),
            vocabulary,
            attempts: attempts.iter().map(ExportedAttempt::from).collect(),
        })
    }

    /// The document as the file's text: pretty JSON, because a person may open it
    /// to see what they are about to restore, and a one-line blob says nothing.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Read a backup back, refusing anything that is not one.
    ///
    /// The `format` marker is checked before the rest of the document is parsed,
    /// so a wrong file gets "that is a vocabulary export" rather than a complaint
    /// about a missing field.
    pub fn parse(json: &str) -> Result<Self, BackupError> {
        let value: serde_json::Value = match serde_json::from_str(json) {
            Ok(value) => value,
            Err(error) => {
                // A JSON Lines log is not one JSON document, so it fails to parse
                // as a whole — and "trailing characters at line 2" is a poor way
                // to be told you picked the file the app itself wrote.
                return Err(if looks_like_the_log(json) {
                    BackupError::LogOnly
                } else {
                    BackupError::Malformed(error.to_string())
                });
            }
        };

        match value.get("format").and_then(|field| field.as_str()) {
            Some(BACKUP_FORMAT) => {}
            Some(other) => return Err(BackupError::NotABackup(other.to_string())),
            None if looks_like_a_list(&value) => return Err(BackupError::VocabularyOnly),
            None => {
                return Err(BackupError::Malformed(
                    "it has no \"format\" field, so it is not one of ours".to_string(),
                ))
            }
        }

        let backup: Self =
            serde_json::from_value(value).map_err(|e| BackupError::Malformed(e.to_string()))?;
        if backup.version > BACKUP_VERSION {
            return Err(BackupError::UnsupportedVersion(backup.version));
        }
        Ok(backup)
    }

    /// When the backup was written, ISO-8601 UTC, for the message afterwards.
    pub fn exported_at(&self) -> &str {
        &self.exported_at
    }

    /// The vocabulary document as text, ready for
    /// [`hanzi_core::VocabStore::import_json`].
    pub fn vocabulary_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.vocabulary)
    }

    /// How many attempts the backup holds.
    pub fn attempt_count(&self) -> usize {
        self.attempts.len()
    }

    /// How many of them carry grading measures, for the message afterwards.
    pub fn measured_count(&self) -> usize {
        self.attempts
            .iter()
            .filter(|attempt| attempt.measures.is_some())
            .count()
    }

    /// The attempts in the shape the log's merge takes.
    ///
    /// A rating this build does not know is read as the rating its score implies,
    /// which is the same harmless reading a hand-edited database gets — see
    /// `hanzi_sync::local`, where a merged attempt's name is settled the same way.
    pub fn incoming_attempts(&self) -> Vec<IncomingAttempt> {
        self.attempts
            .iter()
            .map(|attempt| IncomingAttempt {
                device_id: attempt.device_id.clone(),
                seq: attempt.seq,
                ch: attempt.ch.clone(),
                at: attempt.at.clone(),
                score: attempt.score,
                rating: Rating::from_name(&attempt.rating)
                    .unwrap_or_else(|| Rating::from_score(attempt.score)),
                measures: attempt.measures,
            })
            .collect()
    }
}

/// Whether `json` is one of this crate's backup documents.
///
/// The vocabulary list has an import of its own two screens from the backup's,
/// and to a learner the two files look alike. Its importer asks this so that
/// picking the wrong one is answered with *which screen wants it* rather than a
/// complaint about a missing field — the mirror of [`BackupError::VocabularyOnly`].
pub fn is_a_backup(json: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|value| {
            value
                .get("format")
                .and_then(|field| field.as_str())
                .map(|format| format == BACKUP_FORMAT)
        })
        .unwrap_or(false)
}

/// Whether a document with no `format` field is the vocabulary list's own
/// export. It is the one wrong file a learner is likely to choose, because the
/// two exports sit two screens apart and both are called "export".
fn looks_like_a_list(value: &serde_json::Value) -> bool {
    value.get("nextId").is_some() && value.get("entries").is_some()
}

/// Whether the text is the practice log's own export: JSON Lines, whose first
/// object names an attempt, or the CSV, whose first line is its header.
fn looks_like_the_log(text: &str) -> bool {
    let Some(first) = text.lines().find(|line| !line.trim().is_empty()) else {
        return false;
    };
    if first.trim_start().starts_with("ch,at,score") {
        return true;
    }
    serde_json::from_str::<serde_json::Value>(first)
        .ok()
        .is_some_and(|line| line.get("ch").is_some() && line.get("score").is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanzi_core::AttemptMeasures;

    fn measures() -> AttemptMeasures {
        AttemptMeasures {
            shape: 0.82,
            position: 0.71,
            ink: 0.33,
            ink_coverage: 0.95,
            order: 1.0,
            legible: false,
            order_correct: true,
        }
    }

    fn attempt(ch: &str, measures: Option<AttemptMeasures>) -> LoggedAttempt {
        LoggedAttempt {
            id: 1,
            device_id: "device-a".into(),
            seq: 7,
            ch: ch.into(),
            at: "2026-09-22T09:00:00Z".into(),
            score: 63.0,
            rating: "hard".into(),
            measures,
        }
    }

    const LIST: &str = r#"{"version":1,"nextId":3,"groups":["SiLu"],
        "entries":[{"id":1,"text":"学生","pinyin":"xuésheng","meaning":"student",
        "group":"SiLu","attempts":0,"bestScore":null,"lastPractised":null,
        "addedAt":"2026-09-20T09:00:00Z"}]}"#;

    #[test]
    fn a_backup_round_trips_both_documents() {
        let backup = Backup::build(
            "2026-09-27T12:00:00Z",
            LIST,
            &[attempt("好", Some(measures())), attempt("学", None)],
        )
        .unwrap();
        let text = backup.to_json().unwrap();

        // It is one document, not a stream, and it says what it is.
        assert!(text.starts_with('{'), "pretty JSON, not JSON Lines");
        let raw: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(raw["format"], BACKUP_FORMAT);
        assert_eq!(raw["version"], BACKUP_VERSION);
        assert_eq!(raw["exportedAt"], "2026-09-27T12:00:00Z");

        let read = Backup::parse(&text).unwrap();
        assert_eq!(read.exported_at(), "2026-09-27T12:00:00Z");
        assert_eq!(read.attempt_count(), 2);
        assert_eq!(read.measured_count(), 1);

        // The list comes back byte for byte, so its own reader can take it.
        let list: serde_json::Value =
            serde_json::from_str(&read.vocabulary_json().unwrap()).unwrap();
        assert_eq!(list["entries"][0]["text"], "学生");
        assert_eq!(list["groups"][0], "SiLu");

        // And the attempts come back with their measures, and without inventing
        // any for the one that never had them.
        let incoming = read.incoming_attempts();
        assert_eq!(incoming.len(), 2);
        assert_eq!(incoming[0].measures, Some(measures()));
        assert_eq!(incoming[0].device_id, "device-a");
        assert_eq!(incoming[0].seq, 7);
        assert_eq!(incoming[1].ch, "学");
        assert_eq!(incoming[1].measures, None);
        assert_eq!(incoming[1].rating, Rating::Hard);
    }

    #[test]
    fn a_file_that_is_not_a_backup_is_refused_by_name() {
        // The vocabulary list's own export: a real file, and the mistake worth a
        // message of its own.
        match Backup::parse(LIST) {
            Err(BackupError::VocabularyOnly) => {}
            other => panic!("a list export should be named as one, got {other:?}"),
        }

        // The practice log's own export, in either of its shapes. Both are files
        // this app wrote, so neither should be reported as "not JSON".
        let jsonl = "{\"ch\":\"好\",\"at\":\"2026-09-22T09:00:00Z\",\"score\":63.0,\
                     \"rating\":\"hard\",\"deviceId\":\"device-a\",\"seq\":7}\n\
                     {\"ch\":\"学\",\"at\":\"2026-09-22T10:00:00Z\",\"score\":88.0,\
                     \"rating\":\"good\",\"deviceId\":\"device-a\",\"seq\":8}\n";
        match Backup::parse(jsonl) {
            Err(BackupError::LogOnly) => {}
            other => panic!("a JSON Lines log should be named as one, got {other:?}"),
        }
        match Backup::parse(
            "ch,at,score,rating,device_id,seq,shape\n好,2026-09-22,63,hard,a,7,0.8\n",
        ) {
            Err(BackupError::LogOnly) => {}
            other => panic!("a CSV log should be named as one, got {other:?}"),
        }

        // Somebody else's JSON.
        assert!(matches!(
            Backup::parse(r#"{"hello":"world"}"#),
            Err(BackupError::Malformed(_))
        ));
        assert!(matches!(
            Backup::parse("not json at all"),
            Err(BackupError::Malformed(_))
        ));
        // A document that names a different format is not ours either.
        assert!(matches!(
            Backup::parse(r#"{"format":"something-else","version":1}"#),
            Err(BackupError::NotABackup(_))
        ));

        // And the other direction, which the vocabulary list's own importer
        // asks: a backup is not a list, and it should be able to say so.
        let backup = Backup::build("2026-09-27T12:00:00Z", LIST, &[]).unwrap();
        assert!(is_a_backup(&backup.to_json().unwrap()));
        assert!(!is_a_backup(LIST), "a list export is not a backup");
        assert!(!is_a_backup("not json at all"));
    }

    #[test]
    fn a_backup_from_a_newer_build_is_refused_rather_than_half_read() {
        let text = format!(
            r#"{{"format":"{BACKUP_FORMAT}","version":{},"exportedAt":"2026-09-27T12:00:00Z",
                "vocabulary":{},"attempts":[]}}"#,
            BACKUP_VERSION + 1,
            LIST
        );
        match Backup::parse(&text) {
            Err(BackupError::UnsupportedVersion(v)) => assert_eq!(v, BACKUP_VERSION + 1),
            other => panic!("a newer backup should be refused, got {other:?}"),
        }
    }

    #[test]
    fn a_rating_this_build_does_not_know_is_read_from_the_score() {
        // A hand-edited backup, or one written by a build with a fifth rating.
        // The score is the attempt's own, so it is what the schedule must use.
        let text = format!(
            r#"{{"format":"{BACKUP_FORMAT}","version":1,"exportedAt":"2026-09-27T12:00:00Z",
                "vocabulary":{LIST},
                "attempts":[{{"ch":"好","at":"2026-09-22T09:00:00Z","score":95.0,
                "rating":"splendid","deviceId":"device-a","seq":1}}]}}"#
        );
        let incoming = Backup::parse(&text).unwrap().incoming_attempts();
        assert_eq!(incoming[0].rating, Rating::Easy);
        assert_eq!(incoming[0].measures, None);
    }
}
