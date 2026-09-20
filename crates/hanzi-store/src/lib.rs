//! Durable study storage for Hanzi Tutor: one SQLite database behind the
//! engine's three stores.
//!
//! The engine keeps a schedule, a vocabulary list and a course cursor in memory
//! and hands them to a sink when they change. This crate is that sink, and it
//! exists for one reason: the schedule's attempts want to be an **unbounded log**,
//! and a whole-document format cannot hold one — after N attempts the document is
//! O(N) and every attempt rewrites all of it. Rows have no such ceiling, and the
//! card keeps only what the scheduler needs.
//!
//! ## One database, not three
//!
//! Progress and the vocabulary list are read together on every review-queue
//! build, so two files would mean two connections and a cross-file consistency
//! problem — a review item pointing at a deleted entry — for nothing. The
//! "separate files so one bad file cannot take the others down" rule that the
//! JSON layout was built on exists because a hand-written document can fail to
//! *parse*. That specific failure does not apply to tables, and where it does
//! apply — importing the old documents — the import is per document and per
//! document recorded, so a corrupt `course-cursor.json` blocks the cursor only
//! (see [`migrate`]).
//!
//! ## What it does not do
//!
//! It does not schedule, search or build queues: all of that is
//! `hanzi-core`'s, unchanged, and knows nothing about SQL. The only thing this
//! crate knows is how to read and write the documents the engine hands it.

mod migrate;
mod schema;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use hanzi_core::progress::{
    CursorDocument, CursorSink, Document as ProgressDocument, ProgressError, ProgressSink,
    MAX_HISTORY,
};
use hanzi_core::settings::{BoardSize, Pace, Settings, SettingsError, SettingsSink};
use hanzi_core::time::now_iso8601;
use hanzi_core::vocab::{Document as VocabDocument, VocabError, VocabSink};
use hanzi_core::{Attempt, AttemptRecord, CardState, Entry, Rating};
use rusqlite::{params, Connection};

pub use schema::SCHEMA_VERSION;

/// The study database: one file, one connection, three stores.
///
/// Cheap to clone — the clones share the connection — which is how the three
/// stores are handed the same database.
#[derive(Debug, Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
    dir: PathBuf,
    /// This device's identity, read once at open. It is what makes an attempt's
    /// `(device_id, seq)` pair mean the same thing on every device, so two logs
    /// can be merged without a row colliding — see [`schema`]'s `upgrade`.
    device_id: String,
}

impl Db {
    /// The database's file name inside the data directory.
    pub const FILE_NAME: &'static str = "hanzi.db";

    /// Open the database in `dir`, creating and migrating it if needed.
    ///
    /// An unreadable file is an error, never a silent replacement: the caller
    /// turns it into a warning and refuses to save, exactly as it did when the
    /// study files were JSON. The old documents are *not* imported here — that
    /// happens per store, in [`ProgressSink::load`] and its siblings, so that one
    /// bad document cannot block the others.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, String> {
        let dir = dir.into();
        let path = dir.join(Self::FILE_NAME);
        // The directory has to exist before SQLite can create a database in it,
        // and on a first run it may not: `app_data_dir()` is a path, not a
        // promise, and `--user-dir` names one the reader chose. The study files
        // used to make their own directories as they wrote; this is that
        // behaviour, moved to where the file is now opened.
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let conn = Connection::open(&path).map_err(|e| at(&path, e))?;
        // Write-ahead logging is what makes an interrupted write survivable:
        // a half-finished transaction is rolled back out of the log rather than
        // left in the main database.
        let mode: String = conn
            .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
            .map_err(|e| at(&path, e))?;
        if mode.to_lowercase() != "wal" {
            return Err(format!(
                "{}: the database refused write-ahead logging (it is in {mode:?} mode), \
                 so an interrupted write could not be recovered",
                path.display()
            ));
        }
        conn.execute_batch("PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;")
            .map_err(|e| at(&path, e))?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|e| at(&path, e))?;
        // The version is read *before* the schema is applied, which is the whole
        // point of the guard: `apply` stamps this build's version, so checking
        // afterwards would silently downgrade a database written by a newer app
        // and then accept it. A database whose tables this build does not
        // understand must be refused untouched, not rewritten.
        if let Some(found) = schema::version(&conn).map_err(|e| at(&path, e))? {
            if found > SCHEMA_VERSION {
                return Err(format!(
                    "{} is a study database from a newer version of the app \
                     (schema {found}, this build understands {SCHEMA_VERSION})",
                    path.display()
                ));
            }
        }
        schema::apply(&conn).map_err(|e| at(&path, e))?;
        // `apply` has already generated this if the database was new or predated
        // schema 3; reading it back here is what makes every write in this
        // session able to name its own device without touching the table again.
        let device_id = schema::device_id(&conn).map_err(|e| at(&path, e))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            dir,
            device_id,
        })
    }

    /// This device's identity, for the sync layer to name its shards with.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// The database file.
    pub fn path(&self) -> PathBuf {
        self.dir.join(Self::FILE_NAME)
    }

    /// The directory the old JSON documents would be in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        // A poisoned lock means a thread panicked while holding it. The
        // connection itself is still a connection, and refusing to touch study
        // data for the rest of the session would be worse than carrying on.
        self.conn.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// How many attempts the log holds.
    pub fn attempt_count(&self) -> Result<i64, String> {
        let conn = self.lock();
        conn.query_row("SELECT COUNT(*) FROM attempt", [], |row| row.get(0))
            .map_err(|e| at(&self.path(), e))
    }

    /// The attempt log, oldest first — everything, or one character's.
    ///
    /// This is the log the whole design exists for, and reading it is what makes
    /// it useful: tuning the grading tolerances against real attempts starts
    /// here, and so does any future "what have I been getting wrong" screen.
    ///
    /// "Oldest first" is [`ATTEMPT_ORDER`], which is when the attempt *happened*
    /// rather than when this file heard about it. Those differ as soon as a
    /// peer's attempts have been merged in.
    pub fn attempts(&self, ch: Option<char>) -> Result<Vec<LoggedAttempt>, String> {
        let conn = self.lock();
        let path = self.path();
        let sql = "SELECT id, device_id, seq, ch, at, score, rating FROM attempt";
        let mut stmt = match ch {
            Some(_) => conn
                .prepare(&format!("{sql} WHERE ch = ?1 {ATTEMPT_ORDER}"))
                .map_err(|e| at(&path, e))?,
            None => conn
                .prepare(&format!("{sql} {ATTEMPT_ORDER}"))
                .map_err(|e| at(&path, e))?,
        };
        let rows = match ch {
            Some(ch) => stmt.query_map([ch.to_string()], read_attempt),
            None => stmt.query_map([], read_attempt),
        }
        .map_err(|e| at(&path, e))?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| at(&path, e))?);
        }
        Ok(out)
    }

    /// This device's own attempts, in the order it made them.
    ///
    /// Not the same question as [`Db::attempts`], which answers with everything
    /// the log holds — including attempts merged in from other devices. Sync
    /// publishes *this* device's work under its own name, and a device that
    /// published a peer's attempts as its own would corrupt the identity the whole
    /// merge rests on, so the two questions must not be confused.
    ///
    /// Ordered by `seq` rather than by time, because that is what a shard's
    /// numbering is: a device's own log in the order it wrote it.
    pub fn own_attempts(&self) -> Result<Vec<LoggedAttempt>, String> {
        let conn = self.lock();
        let path = self.path();
        let mut stmt = conn
            .prepare(
                "SELECT id, device_id, seq, ch, at, score, rating FROM attempt
                 WHERE device_id = ?1 ORDER BY seq",
            )
            .map_err(|e| at(&path, e))?;
        let rows = stmt
            .query_map([&self.device_id], read_attempt)
            .map_err(|e| at(&path, e))?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| at(&path, e))?);
        }
        Ok(out)
    }

    /// Merge attempts made elsewhere into the log, returning how many were new.
    ///
    /// An attempt already held — same `(device_id, seq)` — is left exactly as it
    /// is. That is the whole of the merge on the storage side: the pair names one
    /// attempt, so a sync that runs twice must neither duplicate it nor rewrite it.
    /// Deciding whether two copies *agree* is the caller's job, because it is the
    /// caller that has both texts to compare; this can only say "already have it".
    pub fn merge_attempts(&self, incoming: &[IncomingAttempt]) -> Result<usize, String> {
        if incoming.is_empty() {
            return Ok(0);
        }
        let path = self.path();
        let mut conn = self.lock();
        let tx = conn.transaction().map_err(|e| at(&path, e))?;
        let mut added = 0usize;
        for attempt in incoming {
            added += tx
                .execute(
                    "INSERT INTO attempt (device_id, seq, ch, at, score, rating)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(device_id, seq) DO NOTHING",
                    params![
                        attempt.device_id,
                        attempt.seq,
                        attempt.ch,
                        attempt.at,
                        attempt.score,
                        attempt.rating.name(),
                    ],
                )
                .map_err(|e| at(&path, e))?;
        }
        tx.commit().map_err(|e| at(&path, e))?;
        Ok(added)
    }

    /// Read a value out of the database's `meta` table.
    ///
    /// The table is the app's own bookkeeping — the schema version and the
    /// once-only import markers live there — and sync keeps its watermarks beside
    /// them rather than in a file of its own, so that one backup captures
    /// everything.
    pub fn meta_value(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.lock();
        meta_get(&conn, key).map_err(|e| at(&self.path(), e))
    }

    /// Write a value into the database's `meta` table.
    pub fn set_meta_value(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.lock();
        set_meta(&conn, key, value).map_err(|e| at(&self.path(), e))
    }
}

/// Read one attempt row, in the column order every query above uses.
fn read_attempt(row: &rusqlite::Row<'_>) -> rusqlite::Result<LoggedAttempt> {
    Ok(LoggedAttempt {
        id: row.get(0)?,
        device_id: row.get(1)?,
        seq: row.get(2)?,
        ch: row.get(3)?,
        at: row.get(4)?,
        score: row.get(5)?,
        rating: row.get(6)?,
    })
}

/// The order the log is read in, and the order the schedule is folded in.
///
/// Deliberately not `id`. `id` is this file's insertion order, and the moment a
/// peer's attempts have been merged in, a row inserted later can have happened
/// earlier. `(at, device_id, seq)` is a total order that every device computes
/// identically: `at` is only accurate to the second, so ties are real, and the
/// tiebreak has to be something all devices agree on — SM-2's ease factor
/// accumulates in `f32`, so folding the same log in two different orders would
/// leave two devices with slightly different schedules and no way to notice.
const ATTEMPT_ORDER: &str = "ORDER BY at, device_id, seq";

/// One row of the attempt log.
#[derive(Clone, Debug, PartialEq)]
pub struct LoggedAttempt {
    /// This file's rowid. It orders nothing across devices and is not part of the
    /// attempt's identity — `(device_id, seq)` is.
    pub id: i64,
    /// The device that made the attempt.
    pub device_id: String,
    /// The attempt's number in that device's own log, from 1.
    pub seq: i64,
    pub ch: String,
    /// When it happened, ISO-8601 UTC.
    pub at: String,
    pub score: f32,
    /// `again`, `hard`, `good` or `easy`.
    pub rating: String,
}

impl LoggedAttempt {
    /// The pair that names this attempt on every device.
    ///
    /// This is the merge key: a peer sending an attempt this file already has must
    /// be recognised as the same attempt and not appended again.
    pub fn origin(&self) -> (&str, i64) {
        (&self.device_id, self.seq)
    }
}

/// An attempt made on another device, on its way into this log.
///
/// The same facts as a [`LoggedAttempt`] without this file's rowid, which means
/// nothing anywhere else. A separate type rather than a `LoggedAttempt` with a
/// placeholder `id`, because an `id` that is a lie is worse than no `id` at all.
#[derive(Clone, Debug, PartialEq)]
pub struct IncomingAttempt {
    pub device_id: String,
    pub seq: i64,
    pub ch: String,
    pub at: String,
    pub score: f32,
    pub rating: Rating,
}

/// A database error, with the file it came from: "it could not be read" is not
/// much use without knowing which file to look at.
fn at(path: &Path, e: rusqlite::Error) -> String {
    format!("{}: {e}", path.display())
}

fn progress_error(path: &Path, e: rusqlite::Error) -> ProgressError {
    ProgressError::Io(at(path, e))
}

fn vocab_error(path: &Path, e: rusqlite::Error) -> VocabError {
    VocabError::Io(at(path, e))
}

fn settings_error(path: &Path, e: rusqlite::Error) -> SettingsError {
    SettingsError::Io(at(path, e))
}

// ---- the schedule ---------------------------------------------------------

impl ProgressSink for Db {
    fn load(&self) -> Result<ProgressDocument, ProgressError> {
        let mut conn = self.lock();
        migrate::progress(&mut conn, self.dir(), &self.device_id)?;
        let path = self.path();
        let mut cards = read_cards(&conn, &path)?;
        read_histories(&conn, &path, &mut cards)?;
        Ok(ProgressDocument {
            version: hanzi_core::progress::FORMAT_VERSION,
            cards,
        })
    }

    fn save(
        &mut self,
        document: &ProgressDocument,
        changed: &BTreeSet<String>,
        attempts: &[AttemptRecord],
    ) -> Result<(), ProgressError> {
        if changed.is_empty() && attempts.is_empty() {
            return Ok(());
        }
        let path = self.path();
        let mut conn = self.lock();
        // One transaction for the card and its attempt: a schedule that
        // advanced without the attempt behind it would be a lie, and the reverse
        // would show an attempt the schedule never scheduled.
        let tx = conn.transaction().map_err(|e| progress_error(&path, e))?;
        for ch in changed {
            if let Some(card) = document.cards.get(ch) {
                write_card(&tx, ch, card).map_err(|e| progress_error(&path, e))?;
            }
        }
        let first = next_seq(&tx, &self.device_id).map_err(|e| progress_error(&path, e))?;
        for (seq, record) in (first..).zip(attempts.iter()) {
            insert_attempt(&tx, &self.device_id, seq, record)
                .map_err(|e| progress_error(&path, e))?;
        }
        tx.commit().map_err(|e| progress_error(&path, e))
    }
}

/// The card columns, in the order both the insert and the select use them.
const CARD_COLUMNS: &str = "ch, attempts, lapses, best_score, last_score, \
                            last_practised, due, interval_days, ease, repetitions";

fn write_card(conn: &Connection, ch: &str, card: &CardState) -> rusqlite::Result<()> {
    conn.execute(
        &format!(
            "INSERT INTO progress_card ({CARD_COLUMNS})
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(ch) DO UPDATE SET
                 attempts = excluded.attempts,
                 lapses = excluded.lapses,
                 best_score = excluded.best_score,
                 last_score = excluded.last_score,
                 last_practised = excluded.last_practised,
                 due = excluded.due,
                 interval_days = excluded.interval_days,
                 ease = excluded.ease,
                 repetitions = excluded.repetitions"
        ),
        params![
            ch,
            card.attempts,
            card.lapses,
            card.best_score,
            card.last_score,
            card.last_practised,
            card.due,
            card.interval_days,
            card.ease,
            card.repetitions,
        ],
    )?;
    Ok(())
}

fn insert_attempt(
    conn: &Connection,
    device_id: &str,
    seq: i64,
    record: &AttemptRecord,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO attempt (device_id, seq, ch, at, score, rating)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            device_id,
            seq,
            record.ch,
            record.attempt.at,
            record.attempt.score,
            record.attempt.rating.name(),
        ],
    )?;
    Ok(())
}

/// The next number this device should give an attempt.
///
/// Numbered per device — note the `WHERE` — because `(device_id, seq)` is what
/// has to be unique across a merged log, not `seq` on its own. Two devices both
/// starting at 1 is the intended behaviour, not a collision.
pub(crate) fn next_seq(conn: &Connection, device_id: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM attempt WHERE device_id = ?1",
        [device_id],
        |row| row.get(0),
    )
}

fn read_cards(
    conn: &Connection,
    path: &Path,
) -> Result<BTreeMap<String, CardState>, ProgressError> {
    let mut stmt = conn
        .prepare(&format!("SELECT {CARD_COLUMNS} FROM progress_card"))
        .map_err(|e| progress_error(path, e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                CardState {
                    attempts: row.get(1)?,
                    lapses: row.get(2)?,
                    best_score: row.get(3)?,
                    last_score: row.get(4)?,
                    last_practised: row.get(5)?,
                    due: row.get(6)?,
                    interval_days: row.get(7)?,
                    ease: row.get(8)?,
                    repetitions: row.get(9)?,
                    // Filled from the log below: the card does not keep them.
                    history: Vec::new(),
                },
            ))
        })
        .map_err(|e| progress_error(path, e))?;

    let mut cards = BTreeMap::new();
    for row in rows {
        let (ch, card) = row.map_err(|e| progress_error(path, e))?;
        cards.insert(ch, card);
    }
    Ok(cards)
}

/// Fill each card's recent history from the attempt log.
///
/// The newest [`MAX_HISTORY`] rows per character, oldest first — the same view
/// the JSON document used to carry, except that the log behind it keeps going.
/// The window function is what keeps this one query rather than one per card.
///
/// "Newest" is [`ATTEMPT_ORDER`] again, and for the same reason: after a merge,
/// the rows that arrived last are not the attempts that happened last.
fn read_histories(
    conn: &Connection,
    path: &Path,
    cards: &mut BTreeMap<String, CardState>,
) -> Result<(), ProgressError> {
    let mut stmt = conn
        .prepare(
            "SELECT ch, at, score, rating FROM (
                 SELECT ch, at, score, rating, device_id, seq,
                        ROW_NUMBER() OVER (PARTITION BY ch
                                           ORDER BY at DESC, device_id DESC, seq DESC) AS recent
                 FROM attempt
             ) WHERE recent <= ?1 ORDER BY at, device_id, seq",
        )
        .map_err(|e| progress_error(path, e))?;
    let rows = stmt
        .query_map([MAX_HISTORY as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f32>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| progress_error(path, e))?;

    for row in rows {
        let (ch, at, score, rating) = row.map_err(|e| progress_error(path, e))?;
        let Some(card) = cards.get_mut(&ch) else {
            // An attempt with no card cannot happen through the store, which
            // writes both in one transaction; ignoring it is safer than
            // inventing a card with no schedule.
            continue;
        };
        // The stored name is the engine's, not this crate's: `Rating::name` is
        // asserted against serde's own form, so a database and the JSON documents
        // cannot come to disagree about what a row means.
        let rating = Rating::from_name(&rating).ok_or_else(|| {
            ProgressError::Malformed(format!(
                "{}: an attempt in the study database has an unknown rating {rating:?}",
                path.display()
            ))
        })?;
        card.history.push(Attempt { at, score, rating });
    }
    Ok(())
}

// ---- the course cursor ----------------------------------------------------

impl CursorSink for Db {
    fn load(&self) -> Result<CursorDocument, ProgressError> {
        let mut conn = self.lock();
        migrate::cursor(&mut conn, self.dir())?;
        let path = self.path();
        let document = conn
            .query_row(
                "SELECT position, updated_at FROM course_cursor WHERE only_row = 1",
                [],
                |row| {
                    Ok(CursorDocument {
                        version: hanzi_core::progress::FORMAT_VERSION,
                        index: row.get::<_, i64>(0)?.max(0) as usize,
                        updated_at: row.get(1)?,
                    })
                },
            )
            .map_err(|e| progress_error(&path, e))?;
        Ok(document)
    }

    fn save(&mut self, document: &CursorDocument) -> Result<(), ProgressError> {
        let path = self.path();
        let conn = self.lock();
        conn.execute(
            "INSERT INTO course_cursor (only_row, position, updated_at) VALUES (1, ?1, ?2)
             ON CONFLICT(only_row) DO UPDATE SET
                 position = excluded.position,
                 updated_at = excluded.updated_at",
            params![document.index as i64, document.updated_at],
        )
        .map_err(|e| progress_error(&path, e))?;
        Ok(())
    }
}

// ---- the vocabulary list --------------------------------------------------

impl VocabSink for Db {
    fn load(&self) -> Result<VocabDocument, VocabError> {
        let mut conn = self.lock();
        migrate::vocabulary(&mut conn, self.dir(), &self.device_id)?;
        let path = self.path();

        let next_id = meta_i64(&conn, "vocab_next_id")
            .map_err(|e| vocab_error(&path, e))?
            .unwrap_or(1)
            .max(1) as u64;

        let mut groups = Vec::new();
        {
            let mut stmt = conn
                .prepare("SELECT name FROM vocab_group WHERE deleted = 0 ORDER BY position, name")
                .map_err(|e| vocab_error(&path, e))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| vocab_error(&path, e))?;
            for row in rows {
                groups.push(row.map_err(|e| vocab_error(&path, e))?);
            }
        }

        let mut entries = Vec::new();
        {
            let mut stmt = conn
                .prepare(
                    "SELECT id, text, pinyin, meaning, group_name, added_at, \
                            attempts, best_score, last_practised
                     FROM vocab_entry WHERE deleted = 0 ORDER BY id",
                )
                .map_err(|e| vocab_error(&path, e))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(Entry {
                        id: row.get::<_, i64>(0)?.max(0) as u64,
                        text: row.get(1)?,
                        pinyin: row.get(2)?,
                        meaning: row.get(3)?,
                        group: row.get(4)?,
                        added_at: row.get(5)?,
                        attempts: row.get(6)?,
                        best_score: row.get(7)?,
                        last_practised: row.get(8)?,
                    })
                })
                .map_err(|e| vocab_error(&path, e))?;
            for row in rows {
                entries.push(row.map_err(|e| vocab_error(&path, e))?);
            }
        }

        Ok(VocabDocument {
            version: hanzi_core::vocab::FORMAT_VERSION,
            next_id,
            groups,
            entries,
        })
    }

    fn save(&mut self, document: &VocabDocument) -> Result<(), VocabError> {
        let path = self.path();
        let mut conn = self.lock();
        let tx = conn.transaction().map_err(|e| vocab_error(&path, e))?;
        let now = now_iso8601();
        let device = self.device_id.clone();

        // Nothing is deleted any more, and that is the change sync forced. An
        // entry that disappears from the document is **tombstoned**: a row saying
        // "this was removed, at this time, by this device". Erasing it instead
        // would be worse than losing the record — a peer that still holds an older
        // copy would put it straight back on the next sync, because absence cannot
        // be distinguished from "I have not heard about that one yet".
        //
        // Groups go the same way, and a group is named by its name because there is
        // nothing else to name it by: a rename arrives as one name leaving and
        // another arriving.
        let wanted: HashSet<&str> = document.groups.iter().map(String::as_str).collect();
        for name in group_names(&tx, false).map_err(|e| vocab_error(&path, e))? {
            if !wanted.contains(name.as_str()) {
                tx.execute(
                    "UPDATE vocab_group SET deleted = 1, updated_at = ?1, device_id = ?2
                     WHERE name = ?3",
                    params![now, device, name],
                )
                .map_err(|e| vocab_error(&path, e))?;
            }
        }
        for (position, name) in document.groups.iter().enumerate() {
            // The stamp moves only when something the learner chose moves. A sync
            // saves the whole document many times over, and a stamp that advanced
            // on every save would make each device's newest write the winner for no
            // reason — turning "nobody has touched this" into a race.
            tx.execute(
                "INSERT INTO vocab_group (name, position, updated_at, deleted, device_id)
                 VALUES (?1, ?2, ?3, 0, ?4)
                 ON CONFLICT(name) DO UPDATE SET
                     updated_at = CASE
                         WHEN vocab_group.position <> excluded.position
                           OR vocab_group.deleted <> 0
                         THEN excluded.updated_at ELSE vocab_group.updated_at END,
                     device_id = CASE
                         WHEN vocab_group.position <> excluded.position
                           OR vocab_group.deleted <> 0
                         THEN excluded.device_id ELSE vocab_group.device_id END,
                     position = excluded.position,
                     deleted = 0",
                params![name, position as i64, now, device],
            )
            .map_err(|e| vocab_error(&path, e))?;
        }

        let kept: HashSet<i64> = document.entries.iter().map(|e| e.id as i64).collect();
        for id in entry_ids(&tx, false).map_err(|e| vocab_error(&path, e))? {
            if !kept.contains(&id) {
                tx.execute(
                    "UPDATE vocab_entry SET deleted = 1, updated_at = ?1, device_id = ?2
                     WHERE id = ?3",
                    params![now, device, id],
                )
                .map_err(|e| vocab_error(&path, e))?;
            }
        }
        for entry in &document.entries {
            write_entry(&tx, entry, &now, &device).map_err(|e| vocab_error(&path, e))?;
        }

        set_meta(&tx, "vocab_next_id", &document.next_id.to_string())
            .map_err(|e| vocab_error(&path, e))?;
        tx.commit().map_err(|e| vocab_error(&path, e))
    }
}

/// The ids of the entries this build can see, live or tombstoned.
fn entry_ids(conn: &Connection, live_only: bool) -> rusqlite::Result<Vec<i64>> {
    let sql = match live_only {
        true => "SELECT id FROM vocab_entry WHERE deleted = 0",
        false => "SELECT id FROM vocab_entry",
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }
    Ok(ids)
}

/// The names of the groups this build can see, live or tombstoned.
fn group_names(conn: &Connection, live_only: bool) -> rusqlite::Result<Vec<String>> {
    let sql = match live_only {
        true => "SELECT name FROM vocab_group WHERE deleted = 0",
        false => "SELECT name FROM vocab_group",
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut names = Vec::new();
    for row in rows {
        names.push(row?);
    }
    Ok(names)
}

/// Write one entry from the document the engine handed over.
///
/// Two things here are deliberate. The `uuid` is only ever *inserted*, never
/// updated — it is the entry's name on other devices, so it must survive every
/// edit, and a fresh one is generated on the way past even when the row already
/// exists. And the last-writer-wins stamp moves only when **what the learner typed**
/// moves: `text`, `pinyin`, `meaning` or the group. The practice counters do not
/// move it, and that is the important one — a learner practising an entry on the
/// phone must not be able to clobber an edit made on the laptop just because the
/// practice happened later, and a single stamp per entry cannot express "these
/// fields changed but those did not".
fn write_entry(
    conn: &Connection,
    entry: &Entry,
    now: &str,
    device: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO vocab_entry
             (id, text, pinyin, meaning, group_name, added_at, attempts, best_score,
              last_practised, uuid, updated_at, deleted, device_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12)
         ON CONFLICT(id) DO UPDATE SET
             updated_at = CASE
                 WHEN vocab_entry.text <> excluded.text
                   OR vocab_entry.pinyin <> excluded.pinyin
                   OR vocab_entry.meaning <> excluded.meaning
                   OR IFNULL(vocab_entry.group_name, '') <> IFNULL(excluded.group_name, '')
                   OR vocab_entry.deleted <> 0
                 THEN excluded.updated_at ELSE vocab_entry.updated_at END,
             device_id = CASE
                 WHEN vocab_entry.text <> excluded.text
                   OR vocab_entry.pinyin <> excluded.pinyin
                   OR vocab_entry.meaning <> excluded.meaning
                   OR IFNULL(vocab_entry.group_name, '') <> IFNULL(excluded.group_name, '')
                   OR vocab_entry.deleted <> 0
                 THEN excluded.device_id ELSE vocab_entry.device_id END,
             text = excluded.text,
             pinyin = excluded.pinyin,
             meaning = excluded.meaning,
             group_name = excluded.group_name,
             added_at = excluded.added_at,
             attempts = excluded.attempts,
             best_score = excluded.best_score,
             last_practised = excluded.last_practised,
             deleted = 0",
        params![
            entry.id as i64,
            entry.text,
            entry.pinyin,
            entry.meaning,
            entry.group,
            entry.added_at,
            entry.attempts,
            entry.best_score,
            entry.last_practised,
            uuid::Uuid::new_v4().to_string(),
            now,
            device,
        ],
    )?;
    Ok(())
}

// ---- settings -------------------------------------------------------------

/// The settings keys this build knows. An unknown row in the table is left
/// alone rather than pruned: a newer build may have written it.
const CLICK_TO_DRAW: &str = "click_to_draw";
const VOICE: &str = "voice";
const ANIMATION_PACE: &str = "animation_pace";
const BOARD_SIZE: &str = "board_size";

impl SettingsSink for Db {
    /// Read the settings a learner has actually chosen.
    ///
    /// A missing row is an unset preference, not a default: the interface is
    /// what decides that a trackpad should start out click-to-draw, and it can
    /// only do that if "nobody has chosen" survives storage as an absence.
    /// A missing row is also how the two *value* preferences ([`Pace`],
    /// [`BoardSize`]) fall back to their `Default`, since a preference with no
    /// device signal has only the one meaning for its absence.
    fn load(&self) -> Result<Settings, SettingsError> {
        let conn = self.lock();
        let path = self.path();
        let mut settings = Settings::default();
        if let Some(text) =
            setting_get(&conn, CLICK_TO_DRAW).map_err(|e| settings_error(&path, e))?
        {
            settings.click_to_draw =
                Some(match text.as_str() {
                    "true" => true,
                    "false" => false,
                    other => {
                        return Err(SettingsError::Malformed(format!(
                            "{}: the stored setting {CLICK_TO_DRAW:?} is {other:?}, which is                              neither true nor false",
                            path.display()
                        )))
                    }
                });
        }
        if let Some(text) = setting_get(&conn, VOICE).map_err(|e| settings_error(&path, e))? {
            // An empty row is treated as no choice rather than a voice called "".
            // `save` clears the row instead of writing one, so this is only
            // reachable in a hand-edited database — and falling back to the
            // automatic voice is the harmless reading of it.
            let name = text.trim();
            settings.voice = (!name.is_empty()).then(|| name.to_string());
        }
        if let Some(text) =
            setting_get(&conn, ANIMATION_PACE).map_err(|e| settings_error(&path, e))?
        {
            settings.animation_pace = match text.as_str() {
                "slow" => Pace::Slow,
                "normal" => Pace::Normal,
                "fast" => Pace::Fast,
                other => {
                    return Err(SettingsError::Malformed(format!(
                        "{}: the stored setting {ANIMATION_PACE:?} is {other:?}, which is                              not one of slow, normal or fast",
                        path.display()
                    )))
                }
            };
        }
        if let Some(text) =
            setting_get(&conn, BOARD_SIZE).map_err(|e| settings_error(&path, e))?
        {
            settings.board_size = match text.as_str() {
                "compact" => BoardSize::Compact,
                "normal" => BoardSize::Normal,
                "large" => BoardSize::Large,
                other => {
                    return Err(SettingsError::Malformed(format!(
                        "{}: the stored setting {BOARD_SIZE:?} is {other:?}, which is                              not one of compact, normal or large",
                        path.display()
                    )))
                }
            };
        }
        Ok(settings)
    }

    /// Write the settings. Every field this build knows is written or cleared,
    /// so a value removed by the learner is removed from the table rather than
    /// left behind to be mistaken for a choice.
    ///
    /// A pace or a board size *at its default* is written as no row, for the
    /// same reason an unset `Option` is: the default is what a missing row
    /// already means, so a row saying "normal" adds nothing an absent row does
    /// not, while making a fresh install look like a database full of decisions
    /// nobody took. A learner who deliberately picks normal gets the behaviour
    /// they asked for and the row is simply gone.
    fn save(&mut self, settings: &Settings) -> Result<(), SettingsError> {
        let path = self.path();
        let mut conn = self.lock();
        let tx = conn.transaction().map_err(|e| settings_error(&path, e))?;
        let rows = [
            (
                CLICK_TO_DRAW,
                settings
                    .click_to_draw
                    .map(|on| if on { "true" } else { "false" }.to_string()),
            ),
            (VOICE, settings.voice.clone()),
            (
                ANIMATION_PACE,
                (settings.animation_pace != Pace::default())
                    .then(|| pace_key(settings.animation_pace).to_string()),
            ),
            (
                BOARD_SIZE,
                (settings.board_size != BoardSize::default())
                    .then(|| board_size_key(settings.board_size).to_string()),
            ),
        ];
        for (key, value) in rows {
            setting_put(&tx, key, value.as_deref()).map_err(|e| settings_error(&path, e))?;
        }
        tx.commit().map_err(|e| settings_error(&path, e))
    }
}

/// The stored name of a pace. Asserted against the serialised form by a test, so
/// a renamed variant cannot leave the database and the interface disagreeing
/// about what a row means.
fn pace_key(pace: Pace) -> &'static str {
    match pace {
        Pace::Slow => "slow",
        Pace::Normal => "normal",
        Pace::Fast => "fast",
    }
}

/// The stored name of a board size.
fn board_size_key(size: BoardSize) -> &'static str {
    match size {
        BoardSize::Compact => "compact",
        BoardSize::Normal => "normal",
        BoardSize::Large => "large",
    }
}

fn setting_get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
}

/// Set a value, or remove the row when there is none.
fn setting_put(
    conn: &Connection,
    key: &str,
    value: Option<&str>,
) -> rusqlite::Result<()> {
    match value {
        Some(value) => {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
        }
        None => {
            conn.execute("DELETE FROM settings WHERE key = ?1", [key])?;
        }
    }
    Ok(())
}

// ---- meta -----------------------------------------------------------------

pub(crate) fn meta_get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
}

fn meta_i64(conn: &Connection, key: &str) -> rusqlite::Result<Option<i64>> {
    Ok(meta_get(conn, key)?.and_then(|text| text.parse().ok()))
}

pub(crate) fn set_meta(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
