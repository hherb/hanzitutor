//! Importing the study documents the app used to keep as JSON.
//!
//! Once each, and never destructively. The import happens the first time a store
//! is opened, records that it happened in `meta`, and **reads** the old file: the
//! document is left exactly where it is, byte for byte, because it is the only
//! copy of a learner's work until the database has it.
//!
//! ## Per document, not per directory
//!
//! The three documents are imported independently, and each records its own
//! marker, so a `course-cursor.json` that cannot be parsed blocks the cursor and
//! nothing else — the schedule still loads, which is the same promise the three
//! separate files were making. A document that fails to import is simply not
//! marked, so fixing the file and restarting imports it; until then its store
//! reports the problem and refuses to save, rather than starting empty over it.
//!
//! ## What a card's history means afterwards
//!
//! The import copies each card's bounded recent history into the attempt log, so
//! the log starts life holding exactly what was saved. A card's stored attempt
//! *count* may be larger than the history that survived in JSON; the count is
//! kept as it was, and attempts recorded from here on append to the log. Nothing
//! is invented to fill the gap.

use std::path::{Path, PathBuf};

use hanzi_core::progress::{CursorStore, ProgressError, ProgressStore};
use hanzi_core::time::now_iso8601;
use hanzi_core::vocab::{VocabError, VocabStore};
use hanzi_core::AttemptRecord;
use rusqlite::Connection;

use crate::{
    insert_attempt, meta_get, next_seq, progress_error, set_meta, vocab_error, write_card,
    write_entry, Db,
};

/// The documents, by the names the app has written since M1.
pub const PROGRESS_FILE: &str = "progress.json";
pub const VOCABULARY_FILE: &str = "vocabulary.json";
pub const CURSOR_FILE: &str = "course-cursor.json";

const PROGRESS_KEY: &str = "import:progress.json";
const VOCABULARY_KEY: &str = "import:vocabulary.json";
const CURSOR_KEY: &str = "import:course-cursor.json";

/// Import `progress.json`, if it has not been imported already.
///
/// Every attempt the document carried belonged to this device, so they enter the
/// log under `device_id` and take its next numbers in the order the document
/// listed them.
pub(crate) fn progress(
    conn: &mut Connection,
    dir: &Path,
    device_id: &str,
) -> Result<(), ProgressError> {
    let db = db_path(dir);
    if imported(conn, PROGRESS_KEY).map_err(|e| progress_error(&db, e))? {
        return Ok(());
    }
    let file = dir.join(PROGRESS_FILE);
    // The engine's own reader decides what a valid document is, so the import
    // cannot disagree with the app about what "corrupt" means — and a document
    // from a newer build is refused here rather than half-understood.
    let document = ProgressStore::open(&file).map_err(|e| named(&file, e))?;

    let tx = conn.transaction().map_err(|e| progress_error(&db, e))?;
    let mut seq = next_seq(&tx, device_id).map_err(|e| progress_error(&db, e))?;
    for (ch, card) in &document.document().cards {
        write_card(&tx, ch, card).map_err(|e| progress_error(&db, e))?;
        for attempt in &card.history {
            let record = AttemptRecord {
                ch: ch.clone(),
                attempt: attempt.clone(),
            };
            insert_attempt(&tx, device_id, seq, &record).map_err(|e| progress_error(&db, e))?;
            seq += 1;
        }
    }
    set_meta(&tx, PROGRESS_KEY, &marker(&file)).map_err(|e| progress_error(&db, e))?;
    tx.commit().map_err(|e| progress_error(&db, e))
}

/// Import `vocabulary.json`, if it has not been imported already.
/// Import `vocabulary.json`, if it has not been imported already.
///
/// Every entry belonged to this device, so it enters the list under `device_id`
/// with a stamp from now — there is nothing older to claim.
pub(crate) fn vocabulary(
    conn: &mut Connection,
    dir: &Path,
    device_id: &str,
) -> Result<(), VocabError> {
    let db = db_path(dir);
    if imported(conn, VOCABULARY_KEY).map_err(|e| vocab_error(&db, e))? {
        return Ok(());
    }
    let file = dir.join(VOCABULARY_FILE);
    let store = VocabStore::open(&file).map_err(|e| named_vocab(&file, e))?;
    let document = store.document();

    let tx = conn.transaction().map_err(|e| vocab_error(&db, e))?;
    for (position, name) in document.groups.iter().enumerate() {
        tx.execute(
            "INSERT INTO vocab_group (name, position) VALUES (?1, ?2)
             ON CONFLICT(name) DO UPDATE SET position = excluded.position",
            rusqlite::params![name, position as i64],
        )
        .map_err(|e| vocab_error(&db, e))?;
    }
    // The imported entries have no stamp of their own, so they take one from this
    // device and this moment: they are new here, and there was no other writer.
    let stamp = now_iso8601();
    for entry in &document.entries {
        write_entry(&tx, entry, &stamp, device_id).map_err(|e| vocab_error(&db, e))?;
    }
    set_meta(&tx, "vocab_next_id", &document.next_id.to_string())
        .map_err(|e| vocab_error(&db, e))?;
    set_meta(&tx, VOCABULARY_KEY, &marker(&file)).map_err(|e| vocab_error(&db, e))?;
    tx.commit().map_err(|e| vocab_error(&db, e))
}

/// Import `course-cursor.json`, if it has not been imported already.
pub(crate) fn cursor(conn: &mut Connection, dir: &Path) -> Result<(), ProgressError> {
    let db = db_path(dir);
    if imported(conn, CURSOR_KEY).map_err(|e| progress_error(&db, e))? {
        return Ok(());
    }
    let file = dir.join(CURSOR_FILE);

    // Nothing to import means nothing to write, and that guard is load-bearing
    // rather than tidy. The upsert below replaces the whole row, so a device that
    // has already been *synced* into position 340 would have it overwritten with
    // the default 0 by its own "import" of a file that does not exist. Reading a
    // missing document as an empty one is right for a store; writing it over a row
    // somebody else put there is not.
    if !file.exists() {
        let tx = conn.transaction().map_err(|e| progress_error(&db, e))?;
        set_meta(&tx, CURSOR_KEY, &marker(&file)).map_err(|e| progress_error(&db, e))?;
        return tx.commit().map_err(|e| progress_error(&db, e));
    }

    let store = CursorStore::open(&file).map_err(|e| named(&file, e))?;
    let document = store.document();

    let tx = conn.transaction().map_err(|e| progress_error(&db, e))?;
    tx.execute(
        "INSERT INTO course_cursor (only_row, position, updated_at) VALUES (1, ?1, ?2)
         ON CONFLICT(only_row) DO UPDATE SET
             position = excluded.position,
             updated_at = excluded.updated_at",
        rusqlite::params![document.index as i64, document.updated_at],
    )
    .map_err(|e| progress_error(&db, e))?;
    set_meta(&tx, CURSOR_KEY, &marker(&file)).map_err(|e| progress_error(&db, e))?;
    tx.commit().map_err(|e| progress_error(&db, e))
}

/// True when this document has already been dealt with — imported, or found
/// absent on a run that looked.
fn imported(conn: &Connection, key: &str) -> rusqlite::Result<bool> {
    Ok(meta_get(conn, key)?.is_some())
}

/// What to record once a document has been dealt with.
///
/// A timestamp is worth having: it says when the study data moved, which is the
/// first question to ask of a database that looks emptier than expected. The
/// absent case is recorded too, so the app does not look for that file again.
fn marker(file: &Path) -> String {
    if file.exists() {
        format!("imported {}", now_iso8601())
    } else {
        "nothing to import".to_string()
    }
}

fn db_path(dir: &Path) -> PathBuf {
    dir.join(Db::FILE_NAME)
}

/// Name the document an import error came from.
///
/// The store a warning belongs to lives in the database, so the path the reader
/// is shown is `hanzi.db` — but the file that has to be fixed is the JSON one,
/// and an error that says "it is not valid JSON" without saying *which* file
/// leaves them looking in the wrong place. Only the two content errors are
/// renamed: an I/O error already carries its own path.
fn named(file: &Path, error: ProgressError) -> ProgressError {
    match error {
        ProgressError::Malformed(why) => {
            ProgressError::Malformed(format!("{}: {why}", file.display()))
        }
        ProgressError::UnsupportedVersion(v) => ProgressError::Malformed(format!(
            "{}: it was written by a newer version of the app \
             (format {v}, this build understands {})",
            file.display(),
            hanzi_core::progress::FORMAT_VERSION
        )),
        other => other,
    }
}

/// The vocabulary list's version of [`named`], whose errors are its own type.
fn named_vocab(file: &Path, error: VocabError) -> VocabError {
    match error {
        VocabError::Malformed(why) => {
            VocabError::Malformed(format!("{}: {why}", file.display()))
        }
        VocabError::UnsupportedVersion(v) => VocabError::Malformed(format!(
            "{}: it was written by a newer version of the app \
             (format {v}, this build understands {})",
            file.display(),
            hanzi_core::vocab::FORMAT_VERSION
        )),
        other => other,
    }
}
