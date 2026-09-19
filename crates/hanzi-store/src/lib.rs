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
        schema::apply(&conn).map_err(|e| at(&path, e))?;
        if let Some(found) = schema::version(&conn).map_err(|e| at(&path, e))? {
            if found > SCHEMA_VERSION {
                return Err(format!(
                    "{} is a study database from a newer version of the app \
                     (schema {found}, this build understands {SCHEMA_VERSION})",
                    path.display()
                ));
            }
        }
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            dir,
        })
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
    pub fn attempts(&self, ch: Option<char>) -> Result<Vec<LoggedAttempt>, String> {
        let conn = self.lock();
        let path = self.path();
        let sql = "SELECT id, ch, at, score, rating FROM attempt";
        let mut stmt = match ch {
            Some(_) => conn
                .prepare(&format!("{sql} WHERE ch = ?1 ORDER BY id"))
                .map_err(|e| at(&path, e))?,
            None => conn
                .prepare(&format!("{sql} ORDER BY id"))
                .map_err(|e| at(&path, e))?,
        };
        let read = |row: &rusqlite::Row<'_>| {
            Ok(LoggedAttempt {
                id: row.get(0)?,
                ch: row.get(1)?,
                at: row.get(2)?,
                score: row.get(3)?,
                rating: row.get(4)?,
            })
        };
        let rows = match ch {
            Some(ch) => stmt.query_map([ch.to_string()], read),
            None => stmt.query_map([], read),
        }
        .map_err(|e| at(&path, e))?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| at(&path, e))?);
        }
        Ok(out)
    }
}

/// One row of the attempt log.
#[derive(Clone, Debug, PartialEq)]
pub struct LoggedAttempt {
    /// Insertion order, which is chronological: it is the log's own sequence.
    pub id: i64,
    pub ch: String,
    /// When it happened, ISO-8601 UTC.
    pub at: String,
    pub score: f32,
    /// `again`, `hard`, `good` or `easy`.
    pub rating: String,
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

// ---- the schedule ---------------------------------------------------------

impl ProgressSink for Db {
    fn load(&self) -> Result<ProgressDocument, ProgressError> {
        let mut conn = self.lock();
        migrate::progress(&mut conn, self.dir())?;
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
        for record in attempts {
            insert_attempt(&tx, record).map_err(|e| progress_error(&path, e))?;
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

fn insert_attempt(conn: &Connection, record: &AttemptRecord) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO attempt (ch, at, score, rating) VALUES (?1, ?2, ?3, ?4)",
        params![
            record.ch,
            record.attempt.at,
            record.attempt.score,
            rating_text(record.attempt.rating),
        ],
    )?;
    Ok(())
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
fn read_histories(
    conn: &Connection,
    path: &Path,
    cards: &mut BTreeMap<String, CardState>,
) -> Result<(), ProgressError> {
    let mut stmt = conn
        .prepare(
            "SELECT ch, at, score, rating FROM (
                 SELECT id, ch, at, score, rating,
                        ROW_NUMBER() OVER (PARTITION BY ch ORDER BY id DESC) AS recent
                 FROM attempt
             ) WHERE recent <= ?1 ORDER BY id",
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
        card.history.push(Attempt {
            at,
            score,
            rating: rating_from(&rating).map_err(|e| match e {
                ProgressError::Malformed(why) => ProgressError::Malformed(format!("{why} in {}", path.display())),
                other => other,
            })?,
        });
    }
    Ok(())
}

fn rating_text(rating: Rating) -> &'static str {
    match rating {
        Rating::Again => "again",
        Rating::Hard => "hard",
        Rating::Good => "good",
        Rating::Easy => "easy",
    }
}

fn rating_from(text: &str) -> Result<Rating, ProgressError> {
    match text {
        "again" => Ok(Rating::Again),
        "hard" => Ok(Rating::Hard),
        "good" => Ok(Rating::Good),
        "easy" => Ok(Rating::Easy),
        other => Err(ProgressError::Malformed(format!(
            "an attempt in the study database has an unknown rating {other:?}"
        ))),
    }
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
        migrate::vocabulary(&mut conn, self.dir())?;
        let path = self.path();

        let next_id = meta_i64(&conn, "vocab_next_id")
            .map_err(|e| vocab_error(&path, e))?
            .unwrap_or(1)
            .max(1) as u64;

        let mut groups = Vec::new();
        {
            let mut stmt = conn
                .prepare("SELECT name FROM vocab_group ORDER BY position, name")
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
                     FROM vocab_entry ORDER BY id",
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

        // Groups are a short ordered list, so they are replaced outright; the
        // entries are upserted and the leftovers deleted, which keeps their ids
        // (and therefore any review item pointing at them) stable.
        tx.execute("DELETE FROM vocab_group", [])
            .map_err(|e| vocab_error(&path, e))?;
        for (position, name) in document.groups.iter().enumerate() {
            tx.execute(
                "INSERT INTO vocab_group (name, position) VALUES (?1, ?2)",
                params![name, position as i64],
            )
            .map_err(|e| vocab_error(&path, e))?;
        }

        let kept: HashSet<i64> = document.entries.iter().map(|e| e.id as i64).collect();
        let existing: Vec<i64> = {
            let mut stmt = tx
                .prepare("SELECT id FROM vocab_entry")
                .map_err(|e| vocab_error(&path, e))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, i64>(0))
                .map_err(|e| vocab_error(&path, e))?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row.map_err(|e| vocab_error(&path, e))?);
            }
            ids
        };
        for id in existing {
            if !kept.contains(&id) {
                tx.execute("DELETE FROM vocab_entry WHERE id = ?1", [id])
                    .map_err(|e| vocab_error(&path, e))?;
            }
        }
        for entry in &document.entries {
            write_entry(&tx, entry).map_err(|e| vocab_error(&path, e))?;
        }

        set_meta(&tx, "vocab_next_id", &document.next_id.to_string())
            .map_err(|e| vocab_error(&path, e))?;
        tx.commit().map_err(|e| vocab_error(&path, e))
    }
}

fn write_entry(conn: &Connection, entry: &Entry) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO vocab_entry
             (id, text, pinyin, meaning, group_name, added_at, attempts, best_score, last_practised)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET
             text = excluded.text,
             pinyin = excluded.pinyin,
             meaning = excluded.meaning,
             group_name = excluded.group_name,
             added_at = excluded.added_at,
             attempts = excluded.attempts,
             best_score = excluded.best_score,
             last_practised = excluded.last_practised",
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
        ],
    )?;
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
