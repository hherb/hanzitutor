//! The schema, and applying it.
//!
//! Tables are the four things the app keeps: one row per practised character,
//! an **unbounded** log of attempts, the vocabulary list, and the course cursor.
//! The split between the card and the log is the point of the whole exercise: the
//! card holds what the *schedule* needs and a count, and every attempt ever made
//! goes in `attempt`, which nothing rewrites.
//!
//! `meta` carries the schema version and the once-only import markers, and
//! `settings` holds the learner's own preferences — a different kind of thing
//! from study data, kept in the same file because it is the same lifetime.

use rusqlite::Connection;

/// Bumped when a table changes shape.
///
/// 1 — progress cards, the attempt log, the vocabulary list, the cursor.
/// 2 — `settings`, which is additive: an older database gains the table empty on
///     the next open, and nothing has to be rewritten.
/// 3 — sync provenance: `device_id` in `meta`, and `device_id`/`seq` on
///     `attempt`, so one attempt can be named the same way on every device. See
///     [`upgrade`], which is where columns are added.
pub const SCHEMA_VERSION: i64 = 3;

/// Everything the database needs, in one idempotent script.
///
/// Written as `IF NOT EXISTS` throughout so that opening an existing database is
/// the same code path as creating one, and a future column is added by a
/// migration step rather than by changing what a fresh install gets — see
/// [`upgrade`], which runs after this on every open, fresh install included. Each
/// table below is therefore frozen at the shape it was first created with, and
/// no definition is kept in two places.
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

-- The learner's preferences, one row per value they have actually chosen. A
-- preference that has not been chosen has *no row*: absent is not false, and
-- the interface turns an absent choice into the device's own default.
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

/// Create anything missing, add any column this build needs, and record the
/// schema version.
pub(crate) fn apply(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA)?;
    upgrade(conn)?;
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('schema', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [SCHEMA_VERSION.to_string()],
    )?;
    Ok(())
}

/// The key this device's identity is stored under in `meta`.
pub(crate) const DEVICE_ID_KEY: &str = "device_id";

/// Bring an existing database's *columns* up to this build's shape.
///
/// Every step asks the table what it already has, so running this on a database
/// it has already been run on does nothing. It runs on every open, including a
/// fresh install's first one, because [`SCHEMA`] deliberately creates each table
/// at its original shape.
///
/// ## What schema 3 adds, and what it means for rows already there
///
/// A syncable attempt is named by `(device_id, seq)`. The existing `id` cannot do
/// that job: it is this file's rowid, so two devices both reach 1, 2, 3 and a
/// merged log would collide on every row. `seq` is therefore the sequence of the
/// device that *made* the attempt, and `device_id` says which device that was.
///
/// Rows that predate these columns were all written by this device — there was no
/// other writer — so they are backfilled with this device's identity and with
/// `seq = id`, which is already the order they happened in. Nothing has to be
/// renumbered, and a log written before sync existed merges as correctly as one
/// written after it.
///
/// The unique index is the merge's safety property, not a tidiness measure:
/// re-importing an attempt a peer already sent is a no-op rather than a duplicate,
/// which is what lets a sync be retried as often as it likes.
fn upgrade(conn: &Connection) -> rusqlite::Result<()> {
    let device = device_id(conn)?;
    if !has_column(conn, "attempt", "device_id")? {
        conn.execute_batch("ALTER TABLE attempt ADD COLUMN device_id TEXT")?;
        conn.execute(
            "UPDATE attempt SET device_id = ?1 WHERE device_id IS NULL",
            [&device],
        )?;
    }
    if !has_column(conn, "attempt", "seq")? {
        conn.execute_batch("ALTER TABLE attempt ADD COLUMN seq INTEGER")?;
        conn.execute("UPDATE attempt SET seq = id WHERE seq IS NULL", [])?;
    }
    // An index rather than a `UNIQUE` in the table definition, because SQLite
    // cannot add a table constraint to a table that already exists — and an index
    // is what the merge needs anyway.
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS attempt_origin ON attempt (device_id, seq)",
    )?;
    Ok(())
}

/// This device's identity, generated once and then never changed.
///
/// Stored in `meta` rather than derived from the machine: a hostname or a MAC
/// address would be a stable identifier the learner cannot reset, and would
/// collide the moment two devices were cloned from one image. A random value
/// inside the database also means "delete the database and start again" produces
/// a genuinely new device, which is what the shard layout needs it to mean.
pub(crate) fn device_id(conn: &Connection) -> rusqlite::Result<String> {
    if let Some(existing) = crate::meta_get(conn, DEVICE_ID_KEY)? {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    crate::set_meta(conn, DEVICE_ID_KEY, &id)?;
    Ok(id)
}

/// Whether `table` already has a column called `column`.
///
/// `table` is always a literal from this module and never anything a caller
/// supplies: `PRAGMA table_info` takes no bound parameter, so the name has to be
/// formatted into the statement, and it must not be under anyone else's control.
fn has_column(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        if row.get::<_, String>(1)? == column {
            return Ok(true);
        }
    }
    Ok(false)
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
