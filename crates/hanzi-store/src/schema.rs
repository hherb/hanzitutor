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
/// 4 — the same idea for the vocabulary list, where an entry is *edited* rather
///     than only appended to: a `uuid` to name it across devices, an `updated_at`
///     to settle who wrote last, and a `deleted` tombstone so that a removal
///     travels instead of the entry being resurrected by a peer's older copy.
/// 5 — the grading measures behind an attempt's headline score (`shape`,
///     `position`, `ink`, `ink_coverage`, `order_score`, `legible`,
///     `order_correct`), so the tolerances those scores were graded against can be
///     checked against real handwriting instead of synthetic jitter. All
///     nullable on purpose: an attempt recorded before this, and one merged in
///     from a peer, carry only the headline score, and a measure that was never
///     taken must not read as a zero.
/// 6 — `vocab_cursor`, where the learner got to inside one of their own groups.
///     A new table rather than a column, so an older database gains it empty on
///     the next open and nothing has to be rewritten.
pub const SCHEMA_VERSION: i64 = 6;

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

-- Where the learner got to in one of their own groups, one row per group. The
-- course has `course_cursor`; this is its vocabulary equivalent, except that a
-- vocabulary list has many groups rather than one fixed order.
--
-- The position is the entry's **uuid**, not its local `id`. Ids are handed out
-- per device, so an id means a different word on a phone than on a laptop; the
-- uuid is the same word everywhere, which is what makes this row worth syncing
-- later without a migration. The store translates between the two on the way in
-- and out — see `Db::vocab_cursor` — so nothing above it has to know.
--
-- `entry_uuid` is nullable because a position can be *absent*: the row exists to
-- carry the stamp, and a null position means start at the top — which is also
-- what a group whose position was cleared says.
CREATE TABLE IF NOT EXISTS vocab_cursor (
    group_name TEXT PRIMARY KEY,
    entry_uuid TEXT,
    updated_at TEXT NOT NULL,
    device_id  TEXT NOT NULL,
    revision   INTEGER NOT NULL DEFAULT 0
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

    // ---- schema 4: the vocabulary list's turn ------------------------------
    //
    // The vocabulary list is the harder half of sync, and the difference is worth
    // stating because it explains every choice below. An attempt is *appended*: it
    // never changes, so a merged log is a union and there is nothing to settle. An
    // entry is *edited and deleted*, so two devices can disagree about one entry
    // and something has to say which of them is right. That something is
    // `updated_at`, the device id as a tiebreak, and a tombstone for a removal.
    if !has_column(conn, "vocab_entry", "uuid")? {
        conn.execute_batch("ALTER TABLE vocab_entry ADD COLUMN uuid TEXT")?;
        // Every row that existed before this column needs a name no other device
        // will ever pick, and there is nothing in the row to derive one from.
        backfill_entry_uuids(conn)?;
    }
    if !has_column(conn, "vocab_entry", "updated_at")? {
        conn.execute_batch("ALTER TABLE vocab_entry ADD COLUMN updated_at TEXT")?;
        // `added_at` is the best timestamp there is. An entry that was edited
        // afterwards has a stamp earlier than it deserves, which errs towards
        // losing to a peer's edit rather than winning over it — the harmless
        // direction, since the alternative is inventing a time nothing recorded.
        conn.execute(
            "UPDATE vocab_entry SET updated_at = added_at WHERE updated_at IS NULL",
            [],
        )?;
    }
    if !has_column(conn, "vocab_entry", "deleted")? {
        conn.execute_batch(
            "ALTER TABLE vocab_entry ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0",
        )?;
    }
    if !has_column(conn, "vocab_entry", "device_id")? {
        conn.execute_batch("ALTER TABLE vocab_entry ADD COLUMN device_id TEXT")?;
        // Every row that existed before this column was written by this device:
        // there was no other writer. Same rule as schema 3's attempt backfill.
        conn.execute(
            "UPDATE vocab_entry SET device_id = ?1 WHERE device_id IS NULL",
            [&device],
        )?;
    }
    if !has_column(conn, "vocab_group", "updated_at")? {
        conn.execute_batch("ALTER TABLE vocab_group ADD COLUMN updated_at TEXT")?;
        // The epoch, deliberately. A group has no creation time recorded anywhere,
        // and a constant is what keeps two devices' backfills from disagreeing; the
        // epoch guarantees that any real edit, on any device, wins over it.
        conn.execute(
            "UPDATE vocab_group SET updated_at = '1970-01-01T00:00:00Z' WHERE updated_at IS NULL",
            [],
        )?;
    }
    if !has_column(conn, "vocab_group", "deleted")? {
        conn.execute_batch(
            "ALTER TABLE vocab_group ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0",
        )?;
    }
    if !has_column(conn, "vocab_group", "device_id")? {
        conn.execute_batch("ALTER TABLE vocab_group ADD COLUMN device_id TEXT")?;
        conn.execute(
            "UPDATE vocab_group SET device_id = ?1 WHERE device_id IS NULL",
            [&device],
        )?;
    }
    // Unique so that one entry can never be two rows. NULLs are distinct to SQLite,
    // so a row still waiting for its uuid does not collide with another.
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS vocab_entry_uuid ON vocab_entry (uuid)",
    )?;

    // The course cursor is one row and moves for one reason, so it needs only a
    // tiebreak for two devices moving it inside the same second — the same problem
    // the entry stamp solves with the same answer.
    if !has_column(conn, "course_cursor", "device_id")? {
        conn.execute_batch("ALTER TABLE course_cursor ADD COLUMN device_id TEXT")?;
        conn.execute(
            "UPDATE course_cursor SET device_id = ?1 WHERE device_id IS NULL",
            [&device],
        )?;
    }

    // A per-row counter, bumped on every write by whichever device makes it, and the
    // third part of the stamp after the time and the device.
    //
    // It is here because time and device are not enough, and the failure is worse
    // than it sounds. `updated_at` is whole seconds, so a learner who adds an entry
    // and deletes it again inside one second produces two writes with the *same*
    // stamp from the *same* device — and a peer holding the first of them has
    // nothing to compare against, so it keeps the entry and the two devices stay
    // different for ever with no later write to heal them. The counter makes a
    // device's own writes ordered, so the second always beats the first, and every
    // device computes the same answer from the same records.
    for table in ["vocab_entry", "vocab_group", "course_cursor"] {
        if !has_column(conn, table, "revision")? {
            conn.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN revision INTEGER NOT NULL DEFAULT 0"
            ))?;
        }
    }

    // ---- schema 5: what an attempt was graded from -------------------------
    //
    // The headline score is one number, and it is not enough to check the
    // grader against: four weights and a shape tolerance produced it, and
    // without the measures there is no way to ask whether any of them is set
    // where it should be. They are added to the log rather than to the card
    // because they describe one attempt, not a schedule.
    //
    // Deliberately no backfill and no default. A row written before these
    // columns existed has no measures, and neither has an attempt merged in from
    // a peer — the shard format carries the score and the time, and nothing
    // here invents the rest. `NULL` says "not measured", which is the truth;
    // `0.0` would say "measured, and wrong".
    for column in [
        "shape",
        "position",
        "ink",
        "ink_coverage",
        "order_score",
    ] {
        if !has_column(conn, "attempt", column)? {
            conn.execute_batch(&format!("ALTER TABLE attempt ADD COLUMN {column} REAL"))?;
        }
    }
    for column in ["legible", "order_correct"] {
        if !has_column(conn, "attempt", column)? {
            conn.execute_batch(&format!("ALTER TABLE attempt ADD COLUMN {column} INTEGER"))?;
        }
    }

    Ok(())
}

/// Give every entry that predates schema 4 a name of its own.
///
/// One statement per row rather than one clever statement, because the value has to
/// come from the operating system's random generator and SQL has none worth using
/// for this. A vocabulary list is hundreds of entries, so this runs once and
/// quickly.
fn backfill_entry_uuids(conn: &Connection) -> rusqlite::Result<()> {
    let ids: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM vocab_entry WHERE uuid IS NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        ids
    };
    for id in ids {
        conn.execute(
            "UPDATE vocab_entry SET uuid = ?1 WHERE id = ?2",
            rusqlite::params![uuid::Uuid::new_v4().to_string(), id],
        )?;
    }
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
