//! What the durable store has to be true of.
//!
//! These are the M10 acceptance criteria, as tests: an existing install upgrades
//! by importing its JSON files and those files are untouched afterwards; the
//! import is per document, so one corrupt file does not take the others down; a
//! fresh install creates no JSON at all; the attempt log grows without a ceiling;
//! and a write that never commits cannot corrupt what is already stored.
//!
//! The JSON the import reads is produced by *the app's own stores* rather than
//! written out by hand, so the test starts from real saved files — the shape a
//! learner's data is actually in, including its pretty-printing and its version
//! field.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use hanzi_core::progress::{build_queue, ReviewSource, MAX_HISTORY};
use hanzi_core::{CursorStore, ProgressStore, Rating, VocabStore};
use hanzi_store::Db;

/// A private directory per test, cleaned up by the caller's `finish`.
fn dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("hanzi-store-{name}-{}", std::process::id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).unwrap();
    path
}

fn finish(dir: &Path) {
    fs::remove_dir_all(dir).ok();
}

/// Write the three JSON documents the way a pre-M10 install has them.
///
/// Returns the bytes of each, so a test can prove the import did not touch them.
fn write_legacy_documents(dir: &Path) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut progress = ProgressStore::open(dir.join("progress.json")).unwrap();
    progress.record_at('好', 83.0, "2026-09-19T09:00:00Z").unwrap();
    progress.record_at('好', 91.0, "2026-09-19T10:00:00Z").unwrap();
    progress.record_at('学', 40.0, "2026-09-19T11:00:00Z").unwrap();
    progress.save().unwrap();

    let mut vocab = VocabStore::open(dir.join("vocabulary.json")).unwrap();
    let id = vocab
        .add_entry("学习", "xuéxí", "to study", Some("Lesson 3"))
        .unwrap()
        .id;
    vocab.record_attempt(id, 88.0).unwrap();
    vocab.save().unwrap();

    let mut cursor = CursorStore::open(dir.join("course-cursor.json")).unwrap();
    cursor.set_index(413);
    cursor.save().unwrap();

    (
        fs::read(dir.join("progress.json")).unwrap(),
        fs::read(dir.join("vocabulary.json")).unwrap(),
        fs::read(dir.join("course-cursor.json")).unwrap(),
    )
}

#[test]
fn an_existing_install_is_imported_and_its_files_are_left_alone() {
    let dir = dir("import");
    let before = write_legacy_documents(&dir);

    // The first open of each store imports: the callers are the app, exactly as
    // it opens them at startup.
    let db = Db::open(&dir).unwrap();
    let progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    let vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let cursor = CursorStore::open_with(Box::new(db.clone())).unwrap();

    // The schedule, with its counts, best score and recent history.
    let card = progress.card('好').expect("好 was practised twice");
    assert_eq!(card.attempts, 2);
    assert_eq!(card.best_score, Some(91.0));
    assert_eq!(card.last_score, Some(91.0));
    assert_eq!(card.last_practised.as_deref(), Some("2026-09-19T10:00:00Z"));
    assert_eq!(card.history.len(), 2, "both attempts are in the log's view");
    assert_eq!(card.history[0].score, 83.0);
    // The rating is the grade band, so the import cannot invent one: 83 is a
    // *good* and 91 is one point short of an *easy*.
    assert_eq!(card.history[0].rating, Rating::Good);
    assert_eq!(card.history[1].rating, Rating::from_score(91.0));
    assert_eq!(progress.card('学').unwrap().attempts, 1);

    // The vocabulary list, ids and all.
    assert_eq!(vocab.groups(), ["Lesson 3"]);
    assert_eq!(vocab.entries().len(), 1);
    assert_eq!(vocab.entries()[0].text, "学习");
    assert_eq!(vocab.entries()[0].group.as_deref(), Some("Lesson 3"));
    assert_eq!(vocab.entries()[0].attempts, 1);
    assert_eq!(vocab.entries()[0].best_score, Some(88.0));
    assert_eq!(
        vocab.document().next_id,
        2,
        "ids keep counting from where the document was"
    );

    // The cursor.
    assert_eq!(cursor.index(), 413);
    assert!(cursor.updated_at().is_some());

    // And the log holds what the import brought.
    assert_eq!(db.attempt_count().unwrap(), 3);
    let logged = db.attempts(Some('好')).unwrap();
    assert_eq!(logged.len(), 2);
    assert_eq!(logged[0].at, "2026-09-19T09:00:00Z");
    assert_eq!(logged[1].rating, "good", "91 is one point short of easy");
    assert!(logged[0].id < logged[1].id, "the log is in the order it happened");

    // The old documents are still there, byte for byte: the database is a copy,
    // not a move.
    assert_eq!(fs::read(dir.join("progress.json")).unwrap(), before.0);
    assert_eq!(fs::read(dir.join("vocabulary.json")).unwrap(), before.1);
    assert_eq!(fs::read(dir.join("course-cursor.json")).unwrap(), before.2);

    // Now that the import is recorded, the JSON is no longer read at all — which
    // is what makes the database the authority from here on.
    fs::write(dir.join("progress.json"), "{ not json any more").unwrap();
    let mut progress = ProgressStore::open_with(Box::new(Db::open(&dir).unwrap())).unwrap();
    assert_eq!(progress.card('好').unwrap().attempts, 2);
    assert!(progress.record_at('好', 70.0, "2026-09-20T09:00:00Z").is_ok());
    assert!(progress.save().is_ok());

    finish(&dir);
}

#[test]
fn a_fresh_install_creates_only_the_database() {
    let dir = dir("fresh");
    let db = Db::open(&dir).unwrap();

    let mut progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let mut cursor = CursorStore::open_with(Box::new(db.clone())).unwrap();

    progress.record_at('好', 83.0, "2026-09-19T09:00:00Z").unwrap();
    progress.save().unwrap();
    vocab.add_entry("好", "hǎo", "good", None).unwrap();
    vocab.save().unwrap();
    cursor.set_index(7);
    cursor.save().unwrap();

    assert!(db.path().exists(), "the database is where it should be");
    let mut json = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().to_string();
        if name.ends_with(".json") {
            json.push(name);
        }
    }
    assert!(json.is_empty(), "a fresh install wrote JSON: {json:?}");

    finish(&dir);
}

#[test]
fn the_import_is_per_document_so_one_bad_file_blocks_only_itself() {
    let dir = dir("per-document");
    let before = write_legacy_documents(&dir);
    // The cursor is the document most likely to be hand-edited, and the one the
    // old layout kept separate for exactly this reason.
    fs::write(dir.join("course-cursor.json"), "{ not json").unwrap();

    let db = Db::open(&dir).unwrap();

    // The schedule loads, history and all.
    let progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(progress.card('好').unwrap().attempts, 2);

    // The cursor reports its own problem…
    let error = CursorStore::open_with(Box::new(db.clone())).unwrap_err();
    assert!(
        matches!(&error, hanzi_core::ProgressError::Malformed(_)),
        "{error}"
    );
    // It names the document to fix, not only the database it belongs to: the
    // message is a warning on screen, and `hanzi.db` is not the file with the
    // problem.
    assert!(
        error.to_string().contains("course-cursor.json"),
        "the warning should point at the file: {error}"
    );
    // …and has not been recorded as imported, so fixing it imports it.
    assert_eq!(fs::read_to_string(dir.join("course-cursor.json")).unwrap(), "{ not json");

    fs::write(
        dir.join("course-cursor.json"),
        String::from_utf8(before.2.clone()).unwrap(),
    )
    .unwrap();
    let cursor = CursorStore::open_with(Box::new(Db::open(&dir).unwrap())).unwrap();
    assert_eq!(cursor.index(), 413);

    finish(&dir);
}

#[test]
fn the_attempt_log_keeps_more_than_a_card_can_show() {
    // The ceiling a whole-document format could not lift: 25 attempts on one
    // character, every one of them recorded, while the card shows the newest 20.
    let dir = dir("log");
    let db = Db::open(&dir).unwrap();
    let mut store = ProgressStore::open_with(Box::new(db.clone())).unwrap();

    for i in 0..25 {
        let at = format!("2026-09-19T09:{i:02}:00Z");
        store.record_at('好', 50.0 + i as f32, &at).unwrap();
        store.save().unwrap();
    }

    assert_eq!(db.attempt_count().unwrap(), 25, "every attempt is in the log");
    let logged = db.attempts(Some('好')).unwrap();
    assert_eq!(logged.len(), 25);
    assert_eq!(logged[0].score, 50.0);
    assert_eq!(logged[24].score, 74.0);
    assert_eq!(logged[0].at, "2026-09-19T09:00:00Z");

    drop(store);
    let store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    let card = store.card('好').unwrap();
    assert_eq!(card.attempts, 25, "the count is what was recorded");
    assert_eq!(card.history.len(), MAX_HISTORY, "the view is the newest 20");
    assert_eq!(card.history[0].score, 55.0, "oldest of the newest 20");
    assert_eq!(card.history[MAX_HISTORY - 1].score, 74.0);

    // A second character's log is its own.
    assert!(db.attempts(Some('学')).unwrap().is_empty());
    assert_eq!(db.attempts(None).unwrap().len(), 25);

    finish(&dir);
}

#[test]
fn the_journal_is_write_ahead_and_an_uncommitted_write_leaves_nothing() {
    let dir = dir("atomic");
    let db = Db::open(&dir).unwrap();
    let mut store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    store.record_at('好', 83.0, "2026-09-19T09:00:00Z").unwrap();
    store.save().unwrap();
    drop(store);

    let mode: String = rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(mode.to_lowercase(), "wal", "an interrupted write must be recoverable");

    // A second connection that writes and is dropped without committing is what
    // a process killed during a save leaves behind.
    {
        let mut conn = rusqlite::Connection::open(db.path()).unwrap();
        let tx = conn.transaction().unwrap();
        tx.execute(
            "INSERT INTO progress_card
                 (ch, attempts, lapses, best_score, last_score, last_practised,
                  due, interval_days, ease, repetitions)
             VALUES ('坏', 9, 9, 1.0, 1.0, '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z', 1.0, 2.5, 1)",
            [],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO attempt (ch, at, score, rating)
             VALUES ('坏', '2026-01-01T00:00:00Z', 1.0, 'again')",
            [],
        )
        .unwrap();
        // No commit.
    }

    let reopened = ProgressStore::open_with(Box::new(Db::open(&dir).unwrap())).unwrap();
    assert!(reopened.card('好').is_some(), "the committed attempt survived");
    assert_eq!(reopened.card('好').unwrap().attempts, 1);
    assert!(reopened.is_new('坏'), "the uncommitted write left nothing");
    assert_eq!(Db::open(&dir).unwrap().attempt_count().unwrap(), 1);
    assert_eq!(Db::open(&dir).unwrap().attempts(None).unwrap().len(), 1);

    finish(&dir);
}

#[test]
fn a_database_that_cannot_be_read_is_reported_rather_than_replaced() {
    let dir = dir("corrupt");
    let path = dir.join(Db::FILE_NAME);
    fs::write(&path, "this is not a database").unwrap();
    let before = fs::read(&path).unwrap();

    let error = Db::open(&dir).expect_err("a corrupt database must not open");
    assert!(!error.is_empty());
    assert_eq!(
        fs::read(&path).unwrap(),
        before,
        "the file on disk is left exactly as it was"
    );

    finish(&dir);
}

#[test]
fn the_vocabulary_list_round_trips_through_the_database() {
    let dir = dir("vocab");
    let id = {
        let db = Db::open(&dir).unwrap();
        let mut vocab = VocabStore::open_with(Box::new(db)).unwrap();
        let id = vocab
            .add_entry("学习", "xuéxí", "to study", Some("Lesson 3"))
            .unwrap()
            .id;
        vocab.add_group("Lesson 4").unwrap();
        vocab.record_attempt(id, 91.0).unwrap();
        vocab.save().unwrap();
        id
    };

    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(vocab.groups(), ["Lesson 3", "Lesson 4"]);
    assert_eq!(vocab.entries().len(), 1);
    assert_eq!(vocab.entries()[0].id, id);
    assert_eq!(vocab.entries()[0].best_score, Some(91.0));

    // Editing: rename the group, remove the entry, and the rows follow.
    vocab.rename_group("Lesson 3", "Chapter 3").unwrap();
    vocab.save().unwrap();
    let mut reopened = VocabStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(reopened.groups(), ["Chapter 3", "Lesson 4"]);
    assert_eq!(
        reopened.entries()[0].group.as_deref(),
        Some("Chapter 3"),
        "the entry moved with its group"
    );

    reopened.remove_entry(id).unwrap();
    reopened.save().unwrap();
    let after = VocabStore::open_with(Box::new(db)).unwrap();
    assert!(after.entries().is_empty(), "a removed entry leaves no row");
    // The id is not reused, so a new entry cannot inherit the old one's history.
    assert_eq!(after.document().next_id, id + 1);

    finish(&dir);
}

#[test]
fn progress_and_the_vocabulary_list_are_read_from_one_database() {
    // The reason there is one file rather than three: the review queue reads
    // both, and a character that belongs to a saved word must come back as that
    // word.
    let dir = dir("shared");
    let db = Db::open(&dir).unwrap();

    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let id = vocab
        .add_entry("学习", "xuéxí", "to study", None)
        .unwrap()
        .id;
    vocab.save().unwrap();

    let mut progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    // Both far enough in the past to be due now, whatever today is, and 学 the
    // more overdue of the two — which is what decides whose due date the entry
    // is reported against.
    progress.record_at('学', 20.0, "2020-01-01T00:00:00Z").unwrap();
    progress.record_at('习', 20.0, "2021-01-01T00:00:00Z").unwrap();
    progress.save().unwrap();
    drop(progress);

    let progress = ProgressStore::open_with(Box::new(db)).unwrap();
    let course: HashSet<char> = HashSet::new();
    let queue = build_queue(&progress, &course, vocab.entries(), "2030-01-01T00:00:00Z");

    assert_eq!(queue.len(), 1, "one word, not two characters");
    assert_eq!(queue[0].source, ReviewSource::Vocabulary);
    assert_eq!(queue[0].entry_id, Some(id));
    assert_eq!(queue[0].text, "学习");
    assert_eq!(queue[0].ch, '学', "the most overdue character leads the entry");

    finish(&dir);
}

#[test]
fn the_cursor_round_trips_and_only_ever_has_one_row() {
    let dir = dir("cursor");
    {
        let db = Db::open(&dir).unwrap();
        let mut cursor = CursorStore::open_with(Box::new(db.clone())).unwrap();
        assert_eq!(cursor.index(), 0, "a fresh install starts at the beginning");
        assert!(cursor.set_index(413));
        cursor.save().unwrap();
        assert!(!cursor.set_index(413), "the same position is not a move");
        cursor.save().unwrap();
    }

    let db = Db::open(&dir).unwrap();
    let cursor = CursorStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(cursor.index(), 413);

    let rows: i64 = rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM course_cursor", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 1);

    finish(&dir);
}
