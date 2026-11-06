//! A vocabulary list that was open while a sync rewrote the database underneath it.
//!
//! The app holds the list in memory and saves the whole document after every
//! change, while a sync writes the merged list straight into the database. Until
//! the app reloads, its document is older than the database, and a save made in
//! that window used to treat every entry the sync had just brought in as one the
//! learner had removed — tombstoning it here and, through the next sync, on every
//! other device. These tests hold a store open across a sync, the way the app
//! does, and check that its next save writes only what the learner changed.

use std::fs;
use std::path::{Path, PathBuf};

use hanzi_core::VocabStore;
use hanzi_store::Db;
use hanzi_sync::{sync, FolderStore};

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("hanzi-sync-stale-{name}-{}", std::process::id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).unwrap();
    path
}

fn finish(paths: &[&Path]) {
    for path in paths {
        fs::remove_dir_all(path).ok();
    }
}

fn open(db: &Db) -> VocabStore {
    VocabStore::open_with(Box::new(db.clone())).unwrap()
}

/// The live entries' texts on a device, sorted.
fn texts(db: &Db) -> Vec<String> {
    let mut texts: Vec<String> = open(db).entries().iter().map(|e| e.text.clone()).collect();
    texts.sort();
    texts
}

fn id_of_text(store: &VocabStore, text: &str) -> u64 {
    store
        .entries()
        .iter()
        .find(|e| e.text == text)
        .unwrap_or_else(|| panic!("no entry {text}"))
        .id
}

/// `(revision, updated_at, device_id, deleted)` of the row holding `text`.
fn row(db: &Db, text: &str) -> (i64, String, String, bool) {
    rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row(
            "SELECT revision, updated_at, device_id, deleted FROM vocab_entry WHERE text = ?1",
            [text],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, i64>(3)? != 0,
                ))
            },
        )
        .unwrap()
}

/// Age every entry's stamp on a device, so that a later edit is unambiguously
/// later rather than a same-second tie.
fn backdate_all(db: &Db, stamp: &str) {
    rusqlite::Connection::open(db.path())
        .unwrap()
        .execute("UPDATE vocab_entry SET updated_at = ?1", [stamp])
        .unwrap();
}

fn settle(dbs: &[&Db], remote: &FolderStore) {
    for _ in 0..2 {
        for db in dbs {
            sync(db, remote).unwrap();
        }
    }
}

#[test]
fn a_save_from_a_list_opened_before_a_sync_keeps_what_the_sync_pulled() {
    let (dir_a, dir_b, shared) = (scratch("keep-a"), scratch("keep-b"), scratch("keep-store"));
    let (db_a, db_b) = (Db::open(&dir_a).unwrap(), Db::open(&dir_b).unwrap());
    let remote = FolderStore::open(&shared).unwrap();

    {
        let mut vocab = open(&db_b);
        vocab
            .add_entry("汉字", "hànzì", "characters", None)
            .unwrap();
        vocab.save().unwrap();
    }
    settle(&[&db_a, &db_b], &remote);

    // The phone has its list open, as the app always does.
    let mut open_on_b = open(&db_b);

    // The laptop adds a word, and the phone's sync pulls it in underneath the open
    // list.
    {
        let mut vocab = open(&db_a);
        vocab.add_entry("学习", "xuéxí", "to study", None).unwrap();
        vocab.save().unwrap();
    }
    sync(&db_a, &remote).unwrap();
    sync(&db_b, &remote).unwrap();

    // Before the app reloads, the learner practises the word they already had.
    let id = id_of_text(&open_on_b, "汉字");
    open_on_b.record_attempt(id, 80.0).unwrap();
    open_on_b.save().unwrap();

    assert_eq!(
        texts(&db_b),
        ["学习", "汉字"],
        "the pulled word survived the save"
    );
    let mut held: Vec<String> = open_on_b.entries().iter().map(|e| e.text.clone()).collect();
    held.sort();
    assert_eq!(held, ["学习", "汉字"], "and the open list now holds it too");

    settle(&[&db_a, &db_b], &remote);
    for db in [&db_a, &db_b] {
        assert_eq!(texts(db), ["学习", "汉字"], "no device lost the word");
    }
    let practised = open(&db_b);
    let entry = practised
        .entries()
        .iter()
        .find(|e| e.text == "汉字")
        .unwrap();
    assert_eq!(entry.attempts, 1, "the practice attempt itself was kept");

    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn a_word_added_from_a_stale_list_does_not_overwrite_a_pulled_one() {
    let (dir_a, dir_b, shared) = (scratch("id-a"), scratch("id-b"), scratch("id-store"));
    let (db_a, db_b) = (Db::open(&dir_a).unwrap(), Db::open(&dir_b).unwrap());
    let remote = FolderStore::open(&shared).unwrap();

    // Opened while empty, so its next id is the first one — the same number the
    // sync is about to give the word it pulls.
    let mut open_on_b = open(&db_b);

    {
        let mut vocab = open(&db_a);
        vocab.add_entry("学习", "xuéxí", "to study", None).unwrap();
        vocab.save().unwrap();
    }
    sync(&db_a, &remote).unwrap();
    sync(&db_b, &remote).unwrap();

    open_on_b
        .add_entry("汉字", "hànzì", "characters", None)
        .unwrap();
    open_on_b.save().unwrap();

    assert_eq!(
        texts(&db_b),
        ["学习", "汉字"],
        "both words are on the phone"
    );
    let ids: std::collections::HashSet<u64> = open(&db_b).entries().iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), 2, "under two different ids");
    assert!(!row(&db_b, "学习").3, "the pulled word is still live");

    settle(&[&db_a, &db_b], &remote);
    for db in [&db_a, &db_b] {
        assert_eq!(texts(db), ["学习", "汉字"], "and both reach every device");
    }

    // The renumbered word is still the open list's to edit: the store adopted the
    // database's numbering, so an edit lands on the right row.
    let id = id_of_text(&open_on_b, "汉字");
    open_on_b
        .update_entry(id, "hànzì", "Chinese characters", None)
        .unwrap();
    open_on_b.save().unwrap();
    let reopened = open(&db_b);
    let meanings: Vec<(&str, &str)> = reopened
        .entries()
        .iter()
        .map(|e| (e.text.as_str(), e.meaning.as_str()))
        .collect();
    assert!(meanings.contains(&("汉字", "Chinese characters")));
    assert!(
        meanings.contains(&("学习", "to study")),
        "the pulled word was not touched"
    );

    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn saving_an_unchanged_list_does_not_restamp_tombstones() {
    let dir = scratch("tomb");
    let db = Db::open(&dir).unwrap();

    {
        let mut vocab = open(&db);
        vocab.add_entry("学习", "xuéxí", "to study", None).unwrap();
        vocab
            .add_entry("汉字", "hànzì", "characters", None)
            .unwrap();
        vocab.save().unwrap();
        let id = id_of_text(&vocab, "学习");
        vocab.remove_entry(id).unwrap();
        vocab.save().unwrap();
    }
    rusqlite::Connection::open(db.path())
        .unwrap()
        .execute(
            "UPDATE vocab_entry SET updated_at = '2000-01-01T00:00:00Z', device_id = 'peer'
             WHERE text = '学习'",
            [],
        )
        .unwrap();
    let before = row(&db, "学习");
    assert!(before.3, "it is a tombstone");

    // Unrelated practice, saved twice, from two different handles.
    for _ in 0..2 {
        let mut vocab = open(&db);
        let id = id_of_text(&vocab, "汉字");
        vocab.record_attempt(id, 70.0).unwrap();
        vocab.save().unwrap();
        vocab.save().unwrap();
    }

    assert_eq!(
        row(&db, "学习"),
        before,
        "the deletion keeps its own stamp and author"
    );

    finish(&[&dir]);
}

#[test]
fn a_practice_attempt_from_a_stale_list_does_not_bring_back_a_peers_deletion() {
    let (dir_a, dir_b, shared) = (
        scratch("undel-a"),
        scratch("undel-b"),
        scratch("undel-store"),
    );
    let (db_a, db_b) = (Db::open(&dir_a).unwrap(), Db::open(&dir_b).unwrap());
    let remote = FolderStore::open(&shared).unwrap();

    {
        let mut vocab = open(&db_a);
        vocab.add_entry("学习", "xuéxí", "to study", None).unwrap();
        vocab.save().unwrap();
    }
    // Aged before anything travels, so every copy of the original — the uploaded
    // ones included — is older than the change made below.
    backdate_all(&db_a, "2000-01-01T00:00:00Z");
    settle(&[&db_a, &db_b], &remote);

    let mut open_on_b = open(&db_b);

    {
        let mut vocab = open(&db_a);
        let id = id_of_text(&vocab, "学习");
        vocab.remove_entry(id).unwrap();
        vocab.save().unwrap();
    }
    sync(&db_a, &remote).unwrap();
    sync(&db_b, &remote).unwrap();

    let id = id_of_text(&open_on_b, "学习");
    open_on_b.record_attempt(id, 90.0).unwrap();
    open_on_b.save().unwrap();

    assert!(
        texts(&db_b).is_empty(),
        "practising a stale copy did not undelete it"
    );
    assert!(
        open_on_b.entries().is_empty(),
        "and the open list caught up with the deletion"
    );

    settle(&[&db_a, &db_b], &remote);
    for db in [&db_a, &db_b] {
        assert!(texts(db).is_empty(), "the deletion stands everywhere");
    }

    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn a_stale_list_does_not_overwrite_a_peers_edit() {
    let (dir_a, dir_b, shared) = (scratch("edit-a"), scratch("edit-b"), scratch("edit-store"));
    let (db_a, db_b) = (Db::open(&dir_a).unwrap(), Db::open(&dir_b).unwrap());
    let remote = FolderStore::open(&shared).unwrap();

    {
        let mut vocab = open(&db_a);
        vocab.add_entry("学习", "xuéxí", "to study", None).unwrap();
        vocab.save().unwrap();
    }
    // Aged before anything travels, so every copy of the original — the uploaded
    // ones included — is older than the change made below.
    backdate_all(&db_a, "2000-01-01T00:00:00Z");
    settle(&[&db_a, &db_b], &remote);

    let mut open_on_b = open(&db_b);

    {
        let mut vocab = open(&db_a);
        let id = id_of_text(&vocab, "学习");
        vocab.update_entry(id, "xuéxí", "to learn", None).unwrap();
        vocab.save().unwrap();
    }
    // The laptop's edit is aged so that the phone's save below is unambiguously
    // later: a stale copy written with a newer stamp is exactly what would win.
    backdate_all(&db_a, "2001-01-01T00:00:00Z");
    sync(&db_a, &remote).unwrap();
    sync(&db_b, &remote).unwrap();

    // The phone practises its stale copy, which still says "to study".
    let id = id_of_text(&open_on_b, "学习");
    open_on_b.record_attempt(id, 75.0).unwrap();
    open_on_b.save().unwrap();

    settle(&[&db_a, &db_b], &remote);
    for db in [&db_a, &db_b] {
        let vocab = open(db);
        assert_eq!(vocab.entries().len(), 1);
        assert_eq!(
            vocab.entries()[0].meaning,
            "to learn",
            "the laptop's edit stands"
        );
    }
    assert_eq!(
        open(&db_b).entries()[0].attempts,
        1,
        "and the attempt was counted"
    );

    finish(&[&dir_a, &dir_b, &shared]);
}
