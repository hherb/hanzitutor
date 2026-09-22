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
use hanzi_core::{AttemptMeasures, BoardSize, CursorStore, Pace, ProgressStore, Rating, SettingsStore, VocabStore};
use hanzi_store::{Db, SCHEMA_VERSION};

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
fn a_data_directory_that_does_not_exist_yet_is_created() {
    // A first run has no directory, and `--user-dir` names one the reader chose:
    // SQLite will not create either, and a study store that needs `mkdir` first
    // is a study store that does not work on a fresh install.
    let dir = dir("create-dir");
    finish(&dir);
    assert!(!dir.exists(), "the test must start with no directory");

    let db = Db::open(&dir).unwrap();
    let mut store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    store.record_at('好', 83.0, "2026-09-19T09:00:00Z").unwrap();
    store.save().unwrap();

    assert!(dir.join("hanzi.db").exists());
    drop(store);
    let reopened = ProgressStore::open_with(Box::new(Db::open(&dir).unwrap())).unwrap();
    assert_eq!(reopened.card('好').unwrap().attempts, 1);

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

#[test]
fn a_setting_that_was_never_chosen_is_not_a_setting_that_is_off() {
    // The distinction the whole tri-state exists for: the interface resolves an
    // unchosen preference from the device, so storage has to be able to say
    // "nobody chose" rather than "chose off".
    let dir = dir("settings");
    let db = Db::open(&dir).unwrap();

    let store = SettingsStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(store.click_to_draw(), None, "a fresh database has no choice");

    // Off is a choice, and it round-trips as one.
    let mut store = SettingsStore::open_with(Box::new(db.clone())).unwrap();
    assert!(store.set_click_to_draw(Some(false)));
    store.save().unwrap();
    drop(store);

    let store = SettingsStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(store.click_to_draw(), Some(false));

    // Choosing nothing again removes the row rather than storing a third state.
    let mut store = SettingsStore::open_with(Box::new(db.clone())).unwrap();
    assert!(store.set_click_to_draw(None));
    store.save().unwrap();
    drop(store);

    let rows: i64 = rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 0, "an unset preference leaves no row behind");
    let store = SettingsStore::open_with(Box::new(Db::open(&dir).unwrap())).unwrap();
    assert_eq!(store.click_to_draw(), None);

    finish(&dir);
}

#[test]
fn a_chosen_setting_survives_a_restart() {
    let dir = dir("settings-restart");
    {
        let db = Db::open(&dir).unwrap();
        let mut store = SettingsStore::open_with(Box::new(db)).unwrap();
        store.set_click_to_draw(Some(true));
        store.save().unwrap();
    }

    // A second session, as if the app had been started again.
    let db = Db::open(&dir).unwrap();
    let store = SettingsStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(store.click_to_draw(), Some(true));

    // And the settings table is not the study data: it is a row of its own, and
    // nothing about it touches the cards or the log.
    assert_eq!(db.attempt_count().unwrap(), 0);
    let rows: i64 = rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 1);

    finish(&dir);
}

#[test]
fn every_preference_round_trips_through_the_database() {
    // The settings screen writes all four, so all four have to survive a
    // restart — and the pace and the board size are stored as *names*, which is
    // what the screen reads back. A mismatch there would show the learner their
    // choice had been reset every time the app started.
    let dir = dir("settings-all");
    {
        let db = Db::open(&dir).unwrap();
        let mut store = SettingsStore::open_with(Box::new(db)).unwrap();
        assert!(store.set_click_to_draw(Some(false)));
        assert!(store.set_voice(Some("Meijia")));
        assert!(store.set_animation_pace(Pace::Fast));
        assert!(store.set_board_size(BoardSize::Compact));
        store.save().unwrap();
    }

    let db = Db::open(&dir).unwrap();
    let store = SettingsStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(store.click_to_draw(), Some(false));
    assert_eq!(store.view().voice(), Some("Meijia"));
    assert_eq!(store.view().pace(), Pace::Fast);
    assert_eq!(store.view().board_size(), BoardSize::Compact);

    // One row each, and clearing one clears only its own row.
    let count = |db: &Db| -> i64 {
        rusqlite::Connection::open(db.path())
            .unwrap()
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .unwrap()
    };
    assert_eq!(count(&db), 4);

    let mut store = SettingsStore::open_with(Box::new(db.clone())).unwrap();
    assert!(store.set_voice(None));
    store.save().unwrap();
    assert_eq!(count(&db), 3);
    let store = SettingsStore::open_with(Box::new(Db::open(&dir).unwrap())).unwrap();
    assert_eq!(store.view().voice(), None);
    assert_eq!(store.view().pace(), Pace::Fast, "only the voice was cleared");

    // Picking the default is not a row: "normal" is what a missing row already
    // means, so storing it would leave a database full of decisions nobody took.
    let mut store = SettingsStore::open_with(Box::new(db.clone())).unwrap();
    assert!(store.set_animation_pace(Pace::Normal));
    store.save().unwrap();
    assert_eq!(count(&db), 2);
    let store = SettingsStore::open_with(Box::new(Db::open(&dir).unwrap())).unwrap();
    assert_eq!(store.view().pace(), Pace::Normal);

    finish(&dir);
}

#[test]
fn a_stored_name_the_build_does_not_know_is_reported_rather_than_guessed() {
    // A pace is a closed set of names. A row holding anything else means either
    // a hand-edited database or a build whose names changed, and silently
    // calling it "normal" would hide a real mismatch behind a plausible default.
    let dir = dir("settings-bad-pace");
    let db = Db::open(&dir).unwrap();
    rusqlite::Connection::open(db.path())
        .unwrap()
        .execute(
            "INSERT INTO settings (key, value) VALUES ('animation_pace', 'leisurely')",
            [],
        )
        .unwrap();

    let error = SettingsStore::open_with(Box::new(Db::open(&dir).unwrap())).unwrap_err();
    assert!(
        matches!(&error, hanzi_core::SettingsError::Malformed(_)),
        "{error}"
    );
    assert!(error.to_string().contains("animation_pace"), "{error}");

    finish(&dir);
}

#[test]
fn a_setting_a_newer_build_wrote_is_not_mistaken_for_a_choice() {    // A row this build does not know is left alone (a newer build may have
    // written it), but a row for a key it *does* know has to be readable.
    let dir = dir("settings-unknown");
    let db = Db::open(&dir).unwrap();
    {
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('from_a_newer_build', 'whatever')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('click_to_draw', 'true')",
            [],
        )
        .unwrap();
    }

    let store = SettingsStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(store.click_to_draw(), Some(true));

    // A value that is neither true nor false is reported rather than guessed at.
    {
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        conn.execute(
            "UPDATE settings SET value = 'perhaps' WHERE key = 'click_to_draw'",
            [],
        )
        .unwrap();
    }
    let error = SettingsStore::open_with(Box::new(Db::open(&dir).unwrap())).unwrap_err();
    assert!(
        matches!(&error, hanzi_core::SettingsError::Malformed(_)),
        "{error}"
    );
    assert!(error.to_string().contains("click_to_draw"), "{error}");

    finish(&dir);
}

#[test]
fn a_database_from_a_newer_build_is_refused_untouched() {
    // The guard has to read the version *before* stamping its own, or it would
    // downgrade the file and then accept it — the one failure mode a version
    // number exists to prevent.
    let dir = dir("newer-schema");
    let path = Db::open(&dir).unwrap().path();
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("UPDATE meta SET value = '99' WHERE key = 'schema'", [])
            .unwrap();
    }
    let before = fs::metadata(&path).unwrap().len();

    let error = Db::open(&dir).expect_err("a newer schema must not be opened");
    assert!(error.contains("newer version"), "{error}");
    assert!(error.contains("99"), "the message should say what it found: {error}");

    let recorded: String = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(recorded, "99", "the file was rewritten, not refused");
    assert_eq!(fs::metadata(&path).unwrap().len(), before);

    finish(&dir);
}

#[test]
fn an_older_database_gains_the_new_tables_without_losing_anything() {
    // The upgrade path for schema 1 → 2: a database with study data in it, and
    // no `settings` table yet, opens and works.
    let dir = dir("older-schema");
    let db = Db::open(&dir).unwrap();
    let mut progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    progress.record_at('好', 83.0, "2026-09-19T09:00:00Z").unwrap();
    progress.save().unwrap();
    drop(progress);

    // Roll the file back to what schema 1 looked like.
    {
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        conn.execute("DROP TABLE settings", []).unwrap();
        conn.execute("UPDATE meta SET value = '1' WHERE key = 'schema'", [])
            .unwrap();
    }

    let db = Db::open(&dir).unwrap();
    assert_eq!(db.attempt_count().unwrap(), 1, "the study data is still there");
    let mut store = SettingsStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(store.click_to_draw(), None, "and the new table starts empty");
    store.set_click_to_draw(Some(true));
    store.save().unwrap();

    let recorded: String = rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(recorded, SCHEMA_VERSION.to_string());

    finish(&dir);
}

// ---- schema 3: what an attempt has to be able to say about itself ----------
//
// These are M13's groundwork. Nothing here syncs anything yet; what it establishes
// is that an attempt can be *named* — this device, this one — which is what a
// merge needs before there can be one.

#[test]
fn a_database_has_one_device_identity_that_does_not_change() {
    // The identity has to survive a restart, or the same device's attempts would
    // be re-imported as a stranger's on every run.
    let other_dir = dir("device-id-other");
    let dir = dir("device-id");
    let db = Db::open(&dir).unwrap();
    let first = db.device_id().to_string();
    assert!(!first.trim().is_empty(), "a device is named, not blank");

    assert_eq!(db.device_id(), first, "and it is the same name twice");
    let reopened = Db::open(&dir).unwrap();
    assert_eq!(reopened.device_id(), first, "and across a restart");

    // Two databases are two devices, which is the property a merge depends on.
    let other = Db::open(&other_dir).unwrap();
    assert_ne!(other.device_id(), first);

    finish(&dir);
    finish(&other_dir);
}

#[test]
fn an_attempt_is_stamped_with_its_device_and_its_number_on_it() {
    let second_dir = dir("provenance-second");
    let dir = dir("provenance");
    let db = Db::open(&dir).unwrap();
    let mut store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    store.record_at('好', 83.0, "2026-09-19T09:00:00Z").unwrap();
    store.record_at('好', 91.0, "2026-09-19T10:00:00Z").unwrap();
    store.save().unwrap();

    let logged = db.attempts(Some('好')).unwrap();
    assert_eq!(logged.len(), 2);
    assert_eq!(logged[0].device_id, db.device_id(), "this device made it");
    assert_eq!(logged[0].origin(), (db.device_id(), 1), "numbered from 1");
    assert_eq!(logged[1].origin().1, 2, "and in order");

    // The numbers are per device, so a second device also starts at 1. That is
    // why the *pair* has to be unique and the number on its own is not.
    let second = Db::open(&second_dir).unwrap();
    let mut store = ProgressStore::open_with(Box::new(second.clone())).unwrap();
    store.record_at('好', 70.0, "2026-09-19T11:00:00Z").unwrap();
    store.save().unwrap();
    let theirs = second.attempts(Some('好')).unwrap();
    assert_eq!(theirs[0].origin().1, 1, "the other device also starts at 1");
    assert_ne!(theirs[0].device_id, logged[0].device_id);

    finish(&dir);
    finish(&second_dir);
}

#[test]
fn a_peer_sending_an_attempt_this_device_already_has_is_not_a_second_attempt() {
    // The merge's safety property, and the reason `(device_id, seq)` is a unique
    // index rather than a convention: a sync that runs twice, or is retried after
    // a failure it cannot tell happened, must not double a learner's history.
    let dir = dir("no-duplicates");
    let db = Db::open(&dir).unwrap();
    let insert = || {
        rusqlite::Connection::open(db.path())
            .unwrap()
            .execute(
                "INSERT INTO attempt (device_id, seq, ch, at, score, rating)
                 VALUES ('peer', 7, '好', '2026-09-19T08:00:00Z', 60.0, 'good')",
                [],
            )
    };
    insert().expect("the first copy is the peer's attempt arriving");
    assert!(insert().is_err(), "the same attempt again is refused");
    assert_eq!(db.attempt_count().unwrap(), 1);

    finish(&dir);
}

#[test]
fn the_log_reads_in_the_order_attempts_happened_not_the_order_they_arrived() {
    // What a merge does to a log: a peer's attempts are inserted now but happened
    // earlier. Reading by rowid would put them last, and since the schedule is
    // folded in the order the log is read, that would silently reorder a
    // learner's reviews.
    let dir = dir("merge-order");
    let db = Db::open(&dir).unwrap();
    let mut store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    store.record_at('好', 80.0, "2026-09-19T12:00:00Z").unwrap();
    store.save().unwrap();
    drop(store);

    // Older than the attempt above, and inserted after it.
    rusqlite::Connection::open(db.path())
        .unwrap()
        .execute(
            "INSERT INTO attempt (device_id, seq, ch, at, score, rating)
             VALUES ('peer', 1, '好', '2026-09-19T08:00:00Z', 60.0, 'good')",
            [],
        )
        .unwrap();

    let logged = db.attempts(Some('好')).unwrap();
    assert_eq!(logged.len(), 2);
    assert_eq!(logged[0].score, 60.0, "the earlier attempt reads first");
    assert_eq!(logged[0].device_id, "peer");
    assert_eq!(logged[1].score, 80.0);

    // And the card's recent history, which is the same read, agrees.
    let store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    let history = &store.card('好').unwrap().history;
    assert_eq!(history[0].score, 60.0);
    assert_eq!(history[1].score, 80.0);

    finish(&dir);
}

#[test]
fn a_schema_two_database_gains_attempt_provenance_without_renumbering() {
    // The upgrade path for schema 2 → 3. A log written before these columns
    // existed merges as correctly as one written after: every row belonged to
    // this device, and its rowid is already the order it happened in, so the
    // backfill invents nothing.
    let dir = dir("older-attempts");
    let db = Db::open(&dir).unwrap();
    let device = db.device_id().to_string();
    let mut store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    for i in 0..3 {
        let at = format!("2026-09-19T09:0{i}:00Z");
        store.record_at('好', 70.0 + i as f32, &at).unwrap();
    }
    store.save().unwrap();
    drop(store);

    // Roll the file back to what schema 2 looked like. The device identity stays:
    // a device does not become a different device by being upgraded.
    {
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        conn.execute("DROP INDEX attempt_origin", []).unwrap();
        conn.execute("ALTER TABLE attempt DROP COLUMN device_id", [])
            .unwrap();
        conn.execute("ALTER TABLE attempt DROP COLUMN seq", []).unwrap();
        let old = "UPDATE meta SET value = '2' WHERE key = 'schema'";
        conn.execute(old, []).unwrap();
    }

    let db = Db::open(&dir).unwrap();
    assert_eq!(db.device_id(), device, "it is the same device afterwards");
    let logged = db.attempts(Some('好')).unwrap();
    assert_eq!(logged.len(), 3, "no attempt was lost");
    for (i, attempt) in logged.iter().enumerate() {
        assert_eq!(attempt.device_id, device);
        assert_eq!(attempt.seq, attempt.id, "the rowid was already the order");
        assert_eq!(attempt.score, 70.0 + i as f32);
    }

    // And the upgrade is not a one-off: reopening again changes nothing.
    let again = Db::open(&dir).unwrap();
    assert_eq!(again.attempts(Some('好')).unwrap().len(), 3);

    finish(&dir);
}

// ---- schema 5: what an attempt was graded from -----------------------------
//
// The headline score is one number, and it is not enough to check the grader
// against. Four weights and a shape tolerance produced it, and the measures are
// the only record of how — which is why they are written down rather than
// recomputed: an attempt's strokes are gone by the time its row exists.

/// The measures a test records, so the round trip compares something with
/// fractional values that a rounding bug could not pass by accident.
fn some_measures() -> AttemptMeasures {
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

#[test]
fn the_measures_behind_an_attempt_are_kept_with_it() {
    let dir = dir("measures");
    let db = Db::open(&dir).unwrap();
    let mut store = ProgressStore::open_with(Box::new(db.clone())).unwrap();

    let measures = some_measures();
    store.record_measured('好', 63.0, Some(measures)).unwrap();
    // A caller with only a score still records; it just records less. The two
    // states have to stay distinguishable — `ink = 0.0` is a real verdict,
    // "never measured" is not a verdict at all.
    store.record_at('学', 88.0, "2026-09-19T10:00:00Z").unwrap();
    store.save().unwrap();

    let logged = db.attempts(None).unwrap();
    assert_eq!(logged.len(), 2);
    let measured = logged.iter().find(|a| a.ch == "好").unwrap();
    assert_eq!(measured.measures, Some(measures), "every measure survived");
    let bare = logged.iter().find(|a| a.ch == "学").unwrap();
    assert_eq!(bare.measures, None);

    // It is in the file rather than in memory, so a restart does not lose it.
    drop(store);
    let reopened = Db::open(&dir).unwrap();
    let logged = reopened.attempts(Some('好')).unwrap();
    assert_eq!(logged[0].measures, Some(measures));
    assert_eq!(logged[0].score, 63.0, "the headline score is unchanged");

    finish(&dir);
}

#[test]
fn a_schema_four_database_gains_the_measure_columns_and_keeps_its_attempts() {
    // Rows written before schema 5 have no measures, and cannot be given any:
    // the strokes they came from are long gone. What the upgrade must do is add
    // the columns without touching a row, and read the old rows back as "not
    // measured" — a zero would be a verdict the grader never gave.
    let dir = dir("older-measures");
    let db = Db::open(&dir).unwrap();
    let mut store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    store
        .record_measured('好', 71.0, Some(some_measures()))
        .unwrap();
    store.save().unwrap();
    drop(store);

    // Roll the file back to the shape schema 4 left it in.
    {
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        for column in [
            "shape",
            "position",
            "ink",
            "ink_coverage",
            "order_score",
            "legible",
            "order_correct",
        ] {
            conn.execute(&format!("ALTER TABLE attempt DROP COLUMN {column}"), [])
                .unwrap();
        }
        conn.execute("UPDATE meta SET value = '4' WHERE key = 'schema'", [])
            .unwrap();
    }

    let db = Db::open(&dir).unwrap();
    let logged = db.attempts(Some('好')).unwrap();
    assert_eq!(logged.len(), 1, "the attempt survived the upgrade");
    assert_eq!(logged[0].score, 71.0);
    assert_eq!(
        logged[0].measures, None,
        "an attempt from before the columns existed reads as unmeasured"
    );

    // The columns are usable straight away, on the same file.
    let measures = some_measures();
    let mut store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    store.record_measured('学', 55.0, Some(measures)).unwrap();
    store.save().unwrap();
    let logged = db.attempts(Some('学')).unwrap();
    assert_eq!(logged[0].measures, Some(measures));

    // And the upgrade is not a one-off: reopening again changes nothing.
    let again = Db::open(&dir).unwrap();
    assert_eq!(again.attempts(None).unwrap().len(), 2);

    finish(&dir);
}

// ---- schema 4: how a vocabulary entry is named, stamped and removed --------
//
// These are M13's groundwork for the list rather than the schedule. An attempt is
// appended and never changes, so a merged log is a union. An entry is *edited and
// deleted*, so two devices can disagree about one row — which is what the uuid, the
// stamp and the tombstone are for.

/// One vocabulary row as the database holds it, tombstones included.
struct VocabRow {
    uuid: String,
    updated_at: String,
    device_id: String,
    deleted: i64,
}

/// Backdate an entry's stamp, so a test can tell an edit from a no-op.
///
/// Two operations inside one second would otherwise share a stamp: `updated_at` is
/// whole seconds, which is fine for the merge — a device publishes its *current* row,
/// so its own two writes never compete — but useless for a test that has to see a
/// stamp move.
fn age_the_stamp(db: &Db, id: u64, stamp: &str) {
    rusqlite::Connection::open(db.path())
        .unwrap()
        .execute(
            "UPDATE vocab_entry SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![stamp, id as i64],
        )
        .unwrap();
}

fn vocab_row(db: &Db, id: u64) -> VocabRow {
    rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row(
            "SELECT uuid, updated_at, device_id, deleted FROM vocab_entry WHERE id = ?1",
            [id as i64],
            |row| {
                Ok(VocabRow {
                    uuid: row.get(0)?,
                    updated_at: row.get(1)?,
                    device_id: row.get(2)?,
                    deleted: row.get(3)?,
                })
            },
        )
        .unwrap()
}

#[test]
fn a_new_entry_is_given_a_name_and_a_stamp_of_its_own() {
    let dir = dir("vocab-identity");
    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let id = vocab
        .add_entry("学习", "xuéxí", "to study", None)
        .unwrap()
        .id;
    vocab.save().unwrap();

    let row = vocab_row(&db, id);
    assert_eq!(row.uuid.len(), 36, "a uuid, not an empty string: {}", row.uuid);
    assert!(!row.updated_at.is_empty(), "and a stamp to settle arguments with");
    assert_eq!(row.device_id, db.device_id(), "stamped with this device");
    assert_eq!(row.deleted, 0);

    finish(&dir);
}

#[test]
fn practising_an_entry_does_not_move_its_stamp_but_editing_it_does() {
    // The invariant that makes per-entry last-writer-wins safe to use at all. There
    // is one stamp for the whole entry, so if practising moved it, a learner who
    // practised on the phone could silently undo an edit they had made on the
    // laptop — and the edit would be gone with nothing to recover it from.
    let dir = dir("vocab-stamp");
    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let id = vocab
        .add_entry("学习", "xuéxí", "to study", None)
        .unwrap()
        .id;
    vocab.save().unwrap();
    age_the_stamp(&db, id, "2000-01-01T00:00:00Z");
    let after_adding = vocab_row(&db, id).updated_at;

    vocab.record_attempt(id, 91.0).unwrap();
    vocab.save().unwrap();
    assert_eq!(
        vocab_row(&db, id).updated_at,
        after_adding,
        "practising is not an edit"
    );
    let scored = vocab.entries().iter().find(|e| e.id == id).unwrap();
    assert_eq!(scored.best_score, Some(91.0), "though the score is kept");

    vocab
        .update_entry(id, "xuéxí", "to study hard", None)
        .unwrap();
    vocab.save().unwrap();
    assert_ne!(
        vocab_row(&db, id).updated_at,
        after_adding,
        "what the learner typed is an edit, and it moves the stamp"
    );

    finish(&dir);
}

#[test]
fn a_removed_entry_leaves_a_tombstone_rather_than_nothing() {
    // Erasing the row would be worse than losing the record: a device that still
    // holds its own copy would put the entry straight back, because "I have no row
    // for this" and "I have not heard about this yet" are the same thing to a peer.
    let dir = dir("vocab-tombstone");
    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let id = vocab
        .add_entry("学习", "xuéxí", "to study", None)
        .unwrap()
        .id;
    vocab.save().unwrap();
    age_the_stamp(&db, id, "2000-01-01T00:00:00Z");
    let before = vocab_row(&db, id).updated_at;

    vocab.remove_entry(id).unwrap();
    vocab.save().unwrap();

    assert!(vocab.entries().is_empty(), "the interface sees it gone");
    assert_eq!(
        rusqlite::Connection::open(db.path())
            .unwrap()
            .query_row("SELECT COUNT(*) FROM vocab_entry WHERE id = ?1", [id as i64], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1,
        "and the row is still there to say so"
    );
    let row = vocab_row(&db, id);
    assert_eq!(row.deleted, 1, "marked removed");
    assert_ne!(row.updated_at, before, "with a stamp that beats the old copy");
    assert_eq!(row.device_id, db.device_id());

    finish(&dir);
}

#[test]
fn a_schema_three_database_gains_vocabulary_identity_without_reusing_a_name() {
    // The upgrade path for schema 3 → 4. Every entry that predates the columns
    // needs a name no other device will ever pick, and there is nothing in the row
    // to derive one from.
    let dir = dir("vocab-older");
    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let first_entry = vocab.add_entry("学习", "xuéxí", "to study", None).unwrap();
    let (first, added_at) = (first_entry.id, first_entry.added_at.clone());
    let second = vocab.add_entry("你好", "nǐhǎo", "hello", None).unwrap().id;
    vocab.save().unwrap();
    drop(vocab);

    {
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        conn.execute("DROP INDEX vocab_entry_uuid", []).unwrap();
        for column in ["uuid", "updated_at", "deleted", "device_id"] {
            conn.execute(&format!("ALTER TABLE vocab_entry DROP COLUMN {column}"), [])
                .unwrap();
        }
        conn.execute("UPDATE meta SET value = '3' WHERE key = 'schema'", [])
            .unwrap();
    }

    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(vocab.entries().len(), 2, "nothing was lost");
    let first_row = vocab_row(&db, first);
    let second_row = vocab_row(&db, second);
    assert_eq!(first_row.uuid.len(), 36);
    assert_eq!(second_row.uuid.len(), 36);
    assert_ne!(first_row.uuid, second_row.uuid, "no two entries share a name");
    assert_eq!(
        first_row.updated_at, added_at,
        "and the stamp falls back to when it was added"
    );
    // Still usable afterwards.
    vocab.record_attempt(first, 80.0).unwrap();
    vocab.save().unwrap();

    finish(&dir);
}

// ---- schema 6: where the learner got to in one of their own groups ---------

#[test]
fn a_group_position_round_trips_and_is_stored_as_the_entry_uuid() {
    let dir = dir("vocab-cursor");
    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let first = vocab
        .add_entry("学生", "", "", Some("SiLu"))
        .unwrap()
        .id;
    let second = vocab
        .add_entry("姐姐", "", "", Some("SiLu"))
        .unwrap()
        .id;
    vocab.save().unwrap();

    assert_eq!(db.vocab_cursor("SiLu").unwrap(), None, "no position yet");
    db.set_vocab_cursor("SiLu", Some(second)).unwrap();
    assert_eq!(db.vocab_cursor("SiLu").unwrap(), Some(second));
    assert_eq!(db.vocab_cursor("Nope").unwrap(), None, "an unknown group");

    // What is stored is the entry's **uuid**, not its id. An id is handed out per
    // device, so an id names a different word on a phone than on a laptop and the
    // position would move the moment the list is synced.
    let rows = db.vocab_cursors().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].group_name, "SiLu");
    let uuid = rows[0].entry_uuid.clone().expect("a position");
    assert_ne!(uuid, second.to_string(), "the row must not hold the local id");
    let by_uuid: i64 = rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row("SELECT id FROM vocab_entry WHERE uuid = ?1", [&uuid], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(by_uuid as u64, second);
    assert_ne!(first, second, "the two entries are told apart");

    // The stamp moves only when the position does, so a drill that re-records
    // where it already was cannot manufacture a newer stamp for a peer to lose
    // against.
    let before = rows[0].revision;
    db.set_vocab_cursor("SiLu", Some(second)).unwrap();
    assert_eq!(db.vocab_cursors().unwrap()[0].revision, before, "no-op write");
    db.set_vocab_cursor("SiLu", Some(first)).unwrap();
    assert_eq!(db.vocab_cursors().unwrap()[0].revision, before + 1);

    // It is in the file, so a restart resumes where the learner got to.
    drop(vocab);
    let reopened = Db::open(&dir).unwrap();
    assert_eq!(reopened.vocab_cursor("SiLu").unwrap(), Some(first));

    finish(&dir);
}

#[test]
fn a_renamed_group_keeps_its_position() {
    // The position is keyed by the group's name, so a rename is the row moving
    // rather than a new row: a group renamed after a lesson would otherwise
    // silently start again from the top.
    let dir = dir("vocab-cursor-rename");
    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let entry = vocab
        .add_entry("学生", "", "", Some("Lesson 1"))
        .unwrap()
        .id;
    vocab.save().unwrap();
    db.set_vocab_cursor("Lesson 1", Some(entry)).unwrap();

    vocab.rename_group("Lesson 1", "Chapter 1").unwrap();
    vocab.save().unwrap();
    db.rename_vocab_cursor("Lesson 1", "Chapter 1").unwrap();

    assert_eq!(db.vocab_cursor("Lesson 1").unwrap(), None, "the old name is gone");
    assert_eq!(db.vocab_cursor("Chapter 1").unwrap(), Some(entry));
    assert_eq!(db.vocab_cursors().unwrap().len(), 1, "one row, moved");

    finish(&dir);
}

#[test]
fn a_deleted_group_forgets_its_position() {
    let dir = dir("vocab-cursor-delete");
    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let entry = vocab
        .add_entry("学生", "", "", Some("Lesson 1"))
        .unwrap()
        .id;
    vocab.save().unwrap();
    db.set_vocab_cursor("Lesson 1", Some(entry)).unwrap();

    vocab.remove_group("Lesson 1", false).unwrap();
    vocab.save().unwrap();
    db.delete_vocab_cursor("Lesson 1").unwrap();

    assert_eq!(db.vocab_cursor("Lesson 1").unwrap(), None);
    assert!(db.vocab_cursors().unwrap().is_empty(), "no orphan row left");

    finish(&dir);
}

#[test]
fn a_position_whose_entry_is_gone_reads_as_no_position() {
    // Deleting the entry a position names leaves the row naming a word that is
    // not there. That is answered with "no position" rather than an error, so a
    // caller falls back to starting where it can rather than failing to start.
    let dir = dir("vocab-cursor-gone");
    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let entry = vocab
        .add_entry("学生", "", "", Some("Lesson 1"))
        .unwrap()
        .id;
    vocab.save().unwrap();
    db.set_vocab_cursor("Lesson 1", Some(entry)).unwrap();
    assert_eq!(db.vocab_cursor("Lesson 1").unwrap(), Some(entry));

    vocab.remove_entry(entry).unwrap();
    vocab.save().unwrap();

    assert_eq!(db.vocab_cursor("Lesson 1").unwrap(), None);
    // The row survives, so a peer that still has the entry can still be told
    // where this device got to.
    assert_eq!(db.vocab_cursors().unwrap().len(), 1);

    finish(&dir);
}

#[test]
fn a_position_can_be_cleared() {
    let dir = dir("vocab-cursor-clear");
    let db = Db::open(&dir).unwrap();
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let entry = vocab
        .add_entry("学生", "", "", Some("Lesson 1"))
        .unwrap()
        .id;
    vocab.save().unwrap();

    db.set_vocab_cursor("Lesson 1", Some(entry)).unwrap();
    db.set_vocab_cursor("Lesson 1", None).unwrap();
    assert_eq!(db.vocab_cursor("Lesson 1").unwrap(), None);
    let rows = db.vocab_cursors().unwrap();
    assert_eq!(rows.len(), 1, "cleared, not removed");
    assert_eq!(rows[0].entry_uuid, None);

    finish(&dir);
}
