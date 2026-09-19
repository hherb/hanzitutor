//! The schema, and applying it.
//!
//! Tables are the four things the app keeps: one row per practised character,
//! an **unbounded** log of attempts, the vocabulary list, and the course cursor.
//! The split between the card and the log is the point of the whole exercise: the
//! card holds what the *schedule* needs and a count, and every attempt ever made
//! goes in `attempt`, which nothing rewrites.
//!
//! `meta` carries the schema version and the once-only import markers.

use rusqlite::Connection;

/// Bumped when a table changes shape. There is one version so far; a future
/// change adds a step here rather than reading an old shape hopefully.
pub const SCHEMA_VERSION: i64 = 1;

/// Everything the database needs, in one idempotent script.
///
/// Written as `IF NOT EXISTS` throughout so that opening an existing database is
/// the same code path as creating one, and a future column is added by a
/// migration step rather than by changing what a fresh install gets.
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- One row per practised character: what the scheduler needs, plus the totals.
-- The recent attempts themselves are *not* here; they are rows in `attempt`.
CREATE TABLE IF NOT EXISTS progress_card (
    ch             TEXT PRIMARY KEY,
    attempts       INTEGER NOT NULL,
    lapses         INTEGER NOT NULL,
    best_score     REAL,
    last_score     REAL,
    last_practised TEXT,
    due            TEXT NOT NULL,
    interval_days  REAL NOT NULL,
    ease           REAL NOT NULL,
    repetitions    INTEGER NOT NULL
);

-- Every attempt ever recorded, in the order it happened. This table is the
-- reason the store exists: a document format rewrites all of it on every
-- attempt, so it could never grow past a bounded array.
CREATE TABLE IF NOT EXISTS attempt (
    id     INTEGER PRIMARY KEY AUTOINCREMENT,
    ch     TEXT NOT NULL,
    at     TEXT NOT NULL,
    score  REAL NOT NULL,
    rating TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS attempt_by_ch ON attempt (ch, id);

CREATE TABLE IF NOT EXISTS vocab_group (
    name     TEXT PRIMARY KEY,
    position INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS vocab_entry (
    id             INTEGER PRIMARY KEY,
    text           TEXT NOT NULL,
    pinyin         TEXT NOT NULL,
    meaning        TEXT NOT NULL,
    group_name     TEXT,
    added_at       TEXT NOT NULL,
    attempts       INTEGER NOT NULL,
    best_score     REAL,
    last_practised TEXT
);

-- One row, always: the course position. `CHECK` is what keeps it one row.
CREATE TABLE IF NOT EXISTS course_cursor (
    only_row   INTEGER PRIMARY KEY CHECK (only_row = 1),
    position   INTEGER NOT NULL,
    updated_at TEXT
);
";

/// Create anything missing, and record the schema version.
pub(crate) fn apply(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA)?;
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('schema', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [SCHEMA_VERSION.to_string()],
    )?;
    Ok(())
}

/// The schema version recorded in the database, if any.
///
/// A database written by a *newer* build is refused rather than opened: the
/// tables in it are not the tables this build understands, and guessing would be
/// how study data gets mangled.
pub(crate) fn version(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
            row.get(0)
        })
        .ok();
    Ok(value.and_then(|text| text.parse().ok()))
}
