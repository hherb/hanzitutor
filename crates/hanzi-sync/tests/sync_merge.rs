//! Issue #35: the same word added on two devices, a file another device wrote that
//! this one cannot read, and a clock that runs ahead.
//!
//! Each of these used to go wrong on the *first* sync of an ordinary learner — the
//! one who used both devices before connecting them, the one whose phone updated
//! before the laptop, the one whose tablet's clock is an hour out — so each is
//! tested end to end, through two real databases and one folder.

use std::fs;
use std::path::{Path, PathBuf};

use hanzi_core::time::{add_seconds, now_iso8601};
use hanzi_core::{CursorStore, ProgressStore, VocabStore};
use hanzi_store::{Db, SyncedEntry};
use hanzi_sync::{full_log, merge_vocab, read_vocab, sync, FolderStore, RemoteStore, ShardKind};

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("hanzi-sync-merge-{name}-{}", std::process::id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).unwrap();
    path
}

fn finish(paths: &[&Path]) {
    for path in paths {
        fs::remove_dir_all(path).ok();
    }
}

/// A device as the app starts one: the database opened, and the list loaded once,
/// which is when the old JSON list is imported. (A sync *before* that first load
/// meets the import resetting the id counter, which is #11's to fix, not this
/// file's.)
fn device(name: &str) -> (PathBuf, Db) {
    let path = scratch(name);
    let db = Db::open(&path).unwrap();
    VocabStore::open_with(Box::new(db.clone())).unwrap();
    (path, db)
}

fn sql(db: &Db) -> rusqlite::Connection {
    rusqlite::Connection::open(db.path()).unwrap()
}

/// Add a word, and return its local id.
fn add(db: &Db, text: &str, meaning: &str, group: &str) -> u64 {
    let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
    let id = vocab
        .add_entry(text, "xuéxí", meaning, Some(group))
        .unwrap()
        .id;
    vocab.save().unwrap();
    id
}

fn entries(db: &Db) -> Vec<hanzi_core::Entry> {
    VocabStore::open_with(Box::new(db.clone()))
        .unwrap()
        .entries()
        .to_vec()
}

fn uuid_of(db: &Db, id: u64) -> String {
    sql(db)
        .query_row(
            "SELECT uuid FROM vocab_entry WHERE id = ?1",
            [id as i64],
            |row| row.get(0),
        )
        .unwrap()
}

/// Give an entry a chosen uuid, so a test can say which of two duplicates is the
/// lower and therefore which one survives.
fn rename_uuid(db: &Db, id: u64, uuid: &str) {
    sql(db)
        .execute(
            "UPDATE vocab_entry SET uuid = ?1 WHERE id = ?2",
            rusqlite::params![uuid, id as i64],
        )
        .unwrap();
}

fn set_entry_stamp(db: &Db, id: u64, stamp: &str) {
    sql(db)
        .execute(
            "UPDATE vocab_entry SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![stamp, id as i64],
        )
        .unwrap();
}

fn sync_all(dbs: &[&Db], remote: &FolderStore) {
    for _ in 0..2 {
        for db in dbs {
            sync(db, remote).unwrap();
        }
    }
}

const LOW: &str = "00000000-0000-4000-8000-000000000001";
const HIGH: &str = "ffffffff-ffff-4fff-bfff-fffffffffffe";

// ---- 1. the same word added on two devices ---------------------------------

#[test]
fn the_same_word_added_on_two_devices_is_one_entry_on_both_and_keeps_its_history() {
    // The reproduction from the issue: A and B each add 学习 to HSK1 and practise it,
    // then sync. It used to leave two entries on both devices.
    let (dir_a, db_a) = device("dup-a");
    let (dir_b, db_b) = device("dup-b");
    let shared = scratch("dup-store");
    let remote = FolderStore::open(&shared).unwrap();

    let id_a = add(&db_a, "学习", "to study", "HSK1");
    let id_b = add(&db_b, "学习", "to study", "HSK1");
    // B's is the higher uuid, so B is the device whose own row gives way — the
    // harder side, because its practice counters and its group position are the
    // ones that must survive the switch.
    rename_uuid(&db_a, id_a, LOW);
    rename_uuid(&db_b, id_b, HIGH);

    // Practise on both: the per-entry counters each device keeps, and the attempt
    // log the schedule is folded from.
    for (db, id, score, at) in [
        (&db_a, id_a, 70.0, "2026-09-19T09:00:00Z"),
        (&db_b, id_b, 90.0, "2026-09-19T10:00:00Z"),
    ] {
        let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
        vocab.record_attempt(id, score).unwrap();
        vocab.record_attempt(id, score - 10.0).unwrap();
        vocab.save().unwrap();
        let mut progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
        progress.record_at('学', score, at).unwrap();
        progress.save().unwrap();
    }
    db_b.set_vocab_cursor("HSK1", Some(id_b)).unwrap();

    sync_all(&[&db_a, &db_b], &remote);

    for (db, who) in [(&db_a, "A"), (&db_b, "B")] {
        let list = entries(db);
        assert_eq!(
            list.len(),
            1,
            "{who} should hold one 学习, not two: {list:?}"
        );
        assert_eq!(
            uuid_of(db, list[0].id),
            LOW,
            "{who}: the lower uuid survives"
        );
        assert_eq!(
            full_log(db).unwrap().len(),
            2,
            "{who}: both attempts are in the log"
        );
        let progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
        assert_eq!(
            progress.document().cards["学"].attempts,
            2,
            "{who}: one schedule from both"
        );
    }

    // B practised its own row twice, best 90. Those counters are B's practice of the
    // same word, so they are on the survivor now rather than gone with the row.
    let on_b = &entries(&db_b)[0];
    assert_eq!(on_b.attempts, 2, "B's practice moved to the survivor");
    assert_eq!(on_b.best_score, Some(90.0));
    let on_a = &entries(&db_a)[0];
    assert_eq!(on_a.attempts, 2, "and A's own are untouched");
    assert_eq!(on_a.best_score, Some(70.0));

    // B's place in the group named its own row; it still means that word.
    let place = db_b.vocab_cursor("HSK1").unwrap().expect("a position");
    assert_eq!(place, on_b.id, "B's position resolves to the survivor");
    // And on A, which never held B's row, the position B published resolves too.
    let place = db_a
        .vocab_cursor("HSK1")
        .unwrap()
        .expect("the position travelled");
    assert_eq!(place, on_a.id, "A resolves B's position to the survivor");

    // Settled: another round changes nothing on either device.
    for db in [&db_a, &db_b] {
        let again = sync(db, &remote).unwrap();
        assert_eq!(
            again.vocab_changed, 0,
            "a settled list stays settled: {again:?}"
        );
    }
    // And what the store holds merges to one live 学习 for any device that reads it.
    let (stored, groups) = read_vocab(&remote).unwrap();
    let (merged, _) = merge_vocab(stored, groups, vec![], vec![]);
    assert_eq!(
        merged.iter().filter(|e| !e.deleted).count(),
        1,
        "{merged:?}"
    );

    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn the_newer_meaning_survives_whichever_duplicate_keeps_its_name() {
    // The two devices typed different meanings. The lower uuid keeps its name, but
    // which *meaning* survives is last-writer-wins across the pair — the same rule
    // as two edits of one entry — not an accident of which uuid was lower.
    let (dir_a, db_a) = device("dup-meaning-a");
    let (dir_b, db_b) = device("dup-meaning-b");
    let shared = scratch("dup-meaning-store");
    let remote = FolderStore::open(&shared).unwrap();

    let id_a = add(&db_a, "学习", "to study", "HSK1");
    let id_b = add(&db_b, "学习", "to learn (edited later)", "HSK1");
    rename_uuid(&db_a, id_a, LOW);
    rename_uuid(&db_b, id_b, HIGH);
    set_entry_stamp(&db_a, id_a, "2026-01-01T00:00:00Z");
    set_entry_stamp(&db_b, id_b, "2026-02-01T00:00:00Z");

    sync_all(&[&db_a, &db_b], &remote);

    for db in [&db_a, &db_b] {
        let list = entries(db);
        assert_eq!(list.len(), 1);
        assert_eq!(uuid_of(db, list[0].id), LOW);
        assert_eq!(
            list[0].meaning, "to learn (edited later)",
            "the newer edit won"
        );
    }

    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn a_collapsed_word_deleted_later_stays_deleted_and_a_lone_deletion_is_respected() {
    let (dir_a, db_a) = device("dup-delete-a");
    let (dir_b, db_b) = device("dup-delete-b");
    let shared = scratch("dup-delete-store");
    let remote = FolderStore::open(&shared).unwrap();

    // A deleted its own copy before ever syncing, while B's copy is alive: the
    // deletion was of A's entry, so B's survives, on both.
    let id_a = add(&db_a, "学习", "to study", "HSK1");
    add(&db_b, "学习", "to study", "HSK1");
    {
        let mut vocab = VocabStore::open_with(Box::new(db_a.clone())).unwrap();
        vocab.remove_entry(id_a).unwrap();
        vocab.save().unwrap();
    }
    sync_all(&[&db_a, &db_b], &remote);
    assert_eq!(entries(&db_a).len(), 1, "B's copy arrived on A");
    assert_eq!(entries(&db_b).len(), 1);

    // Now a collapsed pair, then the survivor deleted on one device. The loser's
    // tombstone keeps it from coming back from a peer's stale copy.
    let x_a = add(&db_a, "老师", "teacher", "HSK1");
    let x_b = add(&db_b, "老师", "teacher", "HSK1");
    rename_uuid(&db_a, x_a, LOW);
    rename_uuid(&db_b, x_b, HIGH);
    sync_all(&[&db_a, &db_b], &remote);
    let survivor = entries(&db_b)
        .into_iter()
        .find(|e| e.text == "老师")
        .expect("one 老师");
    {
        let mut vocab = VocabStore::open_with(Box::new(db_b.clone())).unwrap();
        vocab.remove_entry(survivor.id).unwrap();
        vocab.save().unwrap();
    }
    sync_all(&[&db_a, &db_b], &remote);
    for db in [&db_a, &db_b] {
        assert!(
            entries(db).iter().all(|e| e.text != "老师"),
            "the deleted word stays deleted, and its duplicate does not come back"
        );
    }

    finish(&[&dir_a, &dir_b, &shared]);
}

fn synced(uuid: &str, text: &str, meaning: &str, at: &str, device: &str) -> SyncedEntry {
    SyncedEntry {
        uuid: uuid.to_string(),
        text: text.to_string(),
        pinyin: String::new(),
        meaning: meaning.to_string(),
        group: Some("HSK1".to_string()),
        added_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: at.to_string(),
        device_id: device.to_string(),
        revision: 0,
        deleted: false,
    }
}

#[test]
fn the_collapse_is_the_same_whatever_order_the_shards_arrive_in() {
    // Three devices each added the word, and one of them also has an unrelated
    // entry. Every order of folding the three shards in, one at a time, must land on
    // the same list — and folding any shard in again must change nothing.
    let shards = [
        vec![synced(
            "b-uuid",
            "学习",
            "b",
            "2026-03-01T00:00:00Z",
            "dev-b",
        )],
        vec![
            synced("c-uuid", "学习", "c", "2026-02-01T00:00:00Z", "dev-c"),
            synced("z-uuid", "朋友", "friend", "2026-02-01T00:00:00Z", "dev-c"),
        ],
        vec![synced(
            "a-uuid",
            "学习",
            "a",
            "2026-01-01T00:00:00Z",
            "dev-a",
        )],
    ];
    let fold = |order: &[usize]| {
        let mut state: Vec<SyncedEntry> = Vec::new();
        for &i in order {
            state = merge_vocab(state, vec![], shards[i].clone(), vec![]).0;
        }
        state
    };
    let orders = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let first = fold(&orders[0]);
    for order in &orders[1..] {
        assert_eq!(
            fold(order),
            first,
            "order {order:?} reached a different list"
        );
    }
    let live: Vec<_> = first.iter().filter(|e| !e.deleted).collect();
    assert_eq!(live.len(), 2, "one 学习 and the unrelated 朋友: {first:?}");
    let word = live.iter().find(|e| e.text == "学习").unwrap();
    assert_eq!(word.uuid, "a-uuid", "the lowest uuid keeps its name");
    assert_eq!(word.meaning, "b", "and the newest meaning");
    for shard in &shards {
        let again = merge_vocab(first.clone(), vec![], shard.clone(), vec![]).0;
        assert_eq!(again, first, "merging a shard twice changes nothing");
    }
}

// ---- 3. a file another device wrote that this one cannot read --------------

/// Write a file straight into the shared folder, as another device would have.
fn plant(remote: &FolderStore, name: &str, bytes: &[u8]) {
    remote.put(name, bytes).unwrap();
}

#[test]
fn a_newer_or_damaged_shard_from_one_device_does_not_stop_the_others() {
    let (dir_a, db_a) = device("skip-a");
    let (dir_b, db_b) = device("skip-b");
    let shared = scratch("skip-store");
    let remote = FolderStore::open(&shared).unwrap();

    // A third device, on an older version, publishes a word; both others take it.
    let old_view = serde_json::json!({
        "version": 1,
        "entries": [{
            "uuid": "newer-device-word", "text": "朋友", "pinyin": "péngyou",
            "meaning": "friend", "group": null, "added_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z", "device_id": "newer-device",
            "revision": 0, "deleted": false
        }],
        "groups": []
    });
    plant(
        &remote,
        "devices/newer-device/vocab.json",
        old_view.to_string().as_bytes(),
    );
    sync(&db_a, &remote).unwrap();
    sync(&db_b, &remote).unwrap();

    // Then it upgrades and writes its list in a format this build has never seen,
    // and a fourth device's attempt shard arrives damaged.
    let newer = br#"{"version": 2, "words": {"newer-device-word": {"gone": true}}}"#;
    plant(&remote, "devices/newer-device/vocab.json", newer);
    plant(
        &remote,
        "devices/broken-device/attempts/000000000001.jsonl",
        b"{\"seq\":1,\"ch\":\"\xe5\xa5\xbd\",\"at\":\"2026-09-19T09:00:00Z\",\"score\":80,\"rating\":\"good\"}\n{not json\n",
    );

    // The two ordinary devices go on syncing with each other: a word and an attempt.
    add(&db_a, "学习", "to study", "HSK1");
    {
        let mut progress = ProgressStore::open_with(Box::new(db_a.clone())).unwrap();
        progress
            .record_at('好', 80.0, "2026-09-20T09:00:00Z")
            .unwrap();
        progress.save().unwrap();
    }
    let on_a = sync(&db_a, &remote).expect("one bad file must not stop the sync");
    let on_b = sync(&db_b, &remote).expect("nor on the other device");
    assert_eq!(on_b.pulled, 1, "B received A's attempt");
    assert!(
        entries(&db_b).iter().any(|e| e.text == "学习"),
        "and A's word"
    );

    // Both files are reported, by kind, and the learner is told what to do.
    for summary in [&on_a, &on_b] {
        let kinds: Vec<_> = summary
            .skipped
            .iter()
            .map(|s| (s.kind, s.newer_format))
            .collect();
        assert!(kinds.contains(&(ShardKind::Vocab, true)), "{kinds:?}");
        assert!(kinds.contains(&(ShardKind::Attempts, false)), "{kinds:?}");
        let notice = summary.notice().expect("a skipped file is said");
        assert!(
            notice.contains("Another device uses a newer version of the app"),
            "{notice}"
        );
        assert!(notice.contains("could not be read"), "{notice}");
    }

    // Nothing of the newer device's was removed for having been unreadable: its word
    // is still live on both, and nobody wrote a tombstone for it.
    for db in [&db_a, &db_b] {
        assert!(
            entries(db).iter().any(|e| e.text == "朋友"),
            "the word is kept"
        );
        let (view, _) = db.vocab_view().unwrap();
        let word = view.iter().find(|e| e.uuid == "newer-device-word").unwrap();
        assert!(!word.deleted);
    }
    // No attempt of the damaged device's was half-read.
    assert!(full_log(&db_b)
        .unwrap()
        .iter()
        .all(|a| a.device_id != "broken-device"));
    // And the files themselves are untouched: neither device wrote over them.
    assert_eq!(
        fs::read(shared.join("devices/newer-device/vocab.json")).unwrap(),
        newer
    );

    // A direct read still refuses, as it always did; only a sync carries on.
    assert!(read_vocab(&remote).is_err());

    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn an_unreadable_baseline_leaves_every_schedule_as_it_is() {
    // A baseline is what a migrated card is folded from, so without it a fold would
    // rebuild the card from the tail of its history. A sync that cannot read one
    // must not rebuild anything.
    let (dir_a, db_a) = device("skip-baseline-a");
    let shared = scratch("skip-baseline-store");
    let remote = FolderStore::open(&shared).unwrap();

    // A peer's attempt arrives, which would ordinarily rebuild 好 here...
    plant(
        &remote,
        "devices/peer/attempts/000000000001.jsonl",
        "{\"seq\":1,\"ch\":\"好\",\"at\":\"2026-09-19T09:00:00Z\",\"score\":80,\"rating\":\"good\"}\n"
            .as_bytes(),
    );
    // ...but the same peer's baseline is damaged.
    plant(
        &remote,
        "devices/peer/baseline.json",
        b"{\"version\": 1, \"device_id\": ",
    );

    let summary = sync(&db_a, &remote).unwrap();
    assert_eq!(summary.pulled, 1, "the attempt is in the log");
    assert_eq!(
        summary.recomputed, 0,
        "but no schedule was rebuilt without the baseline"
    );
    assert!(summary
        .skipped
        .iter()
        .any(|s| s.kind == ShardKind::Baseline));
    assert!(summary
        .notice()
        .unwrap()
        .contains("Schedules were left as they are"));

    // Once the file is readable, the next sync rebuilds as normal.
    fs::remove_file(shared.join("devices/peer/baseline.json")).unwrap();
    let summary = sync(&db_a, &remote).unwrap();
    assert!(summary.skipped.is_empty());
    assert_eq!(summary.recomputed, 1);

    finish(&[&dir_a, &shared]);
}

// ---- 4. a clock that runs ahead ---------------------------------------------

/// An hour from now, as the fast device's clock would say.
fn an_hour_ahead() -> String {
    add_seconds(&now_iso8601(), 3600).unwrap()
}

#[test]
fn a_device_whose_clock_runs_ahead_does_not_win_every_later_edit() {
    let (dir_a, db_a) = device("clock-a");
    let (dir_b, db_b) = device("clock-b");
    let shared = scratch("clock-store");
    let remote = FolderStore::open(&shared).unwrap();

    // A's clock is an hour fast: its edit is stamped an hour from now.
    let id_a = add(&db_a, "学习", "fast clock's meaning", "HSK1");
    set_entry_stamp(&db_a, id_a, &an_hour_ahead());
    {
        let mut cursor = CursorStore::open_with(Box::new(db_a.clone())).unwrap();
        cursor.set_index(120);
        cursor.save().unwrap();
    }
    sql(&db_a)
        .execute(
            "UPDATE course_cursor SET updated_at = ?1",
            [an_hour_ahead()],
        )
        .unwrap();
    sync(&db_a, &remote).unwrap();
    sync(&db_b, &remote).unwrap();

    // B has seen that, and B is restarted before the learner edits — what it saw
    // has to survive that.
    drop(db_b);
    let db_b = Db::open(&dir_b).unwrap();
    let id_b = entries(&db_b)[0].id;
    {
        let mut vocab = VocabStore::open_with(Box::new(db_b.clone())).unwrap();
        vocab
            .update_entry(id_b, "xuéxí", "the later edit", Some("HSK1"))
            .unwrap();
        vocab.save().unwrap();
        let mut cursor = CursorStore::open_with(Box::new(db_b.clone())).unwrap();
        cursor.set_index(340);
        cursor.save().unwrap();
    }
    sync_all(&[&db_a, &db_b], &remote);

    for (db, who) in [(&db_a, "A"), (&db_b, "B")] {
        assert_eq!(
            entries(db)[0].meaning,
            "the later edit",
            "{who}: the edit made last wins, not the one with the fast clock"
        );
        let cursor = CursorStore::open_with(Box::new(db.clone())).unwrap();
        assert_eq!(
            cursor.view().index,
            340,
            "{who}: and so does the move made last"
        );
    }
    // The stamps keep their format: fixed-width ISO-8601, so they still sort.
    let (view, _) = db_b.vocab_view().unwrap();
    assert_eq!(view[0].updated_at.len(), "2026-09-20T10:00:00Z".len());
    assert!(view[0].updated_at.ends_with('Z'));

    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn looking_at_the_course_again_does_not_restamp_an_old_position() {
    // The app saves the cursor on every visit, moved or not. With stamps coming
    // from the sync clock, a save that did not move it must keep the stamp it had,
    // or this device's old position would beat a peer's newer move.
    let (dir_a, db_a) = device("clock-still-a");
    {
        let mut cursor = CursorStore::open_with(Box::new(db_a.clone())).unwrap();
        cursor.set_index(120);
        cursor.save().unwrap();
    }
    sql(&db_a)
        .execute(
            "UPDATE course_cursor SET updated_at = '2000-01-01T00:00:00Z'",
            [],
        )
        .unwrap();
    {
        let mut cursor = CursorStore::open_with(Box::new(db_a.clone())).unwrap();
        cursor.set_index(120);
        cursor.save().unwrap();
    }
    assert_eq!(
        db_a.cursor_view().unwrap().unwrap().updated_at,
        "2000-01-01T00:00:00Z",
        "an unmoved cursor keeps its stamp"
    );
    finish(&[&dir_a]);
}

// ---- also: a group imported from the old JSON list --------------------------

#[test]
fn a_group_imported_from_the_old_list_is_published_with_a_stamp_and_a_device() {
    let dir = scratch("legacy-group");
    {
        let mut legacy = VocabStore::open(dir.join("vocabulary.json")).unwrap();
        legacy
            .add_entry("学习", "xuéxí", "to study", Some("HSK1"))
            .unwrap();
        legacy.save().unwrap();
    }
    let db = Db::open(&dir).unwrap();
    assert_eq!(entries(&db).len(), 1, "the old list was imported");
    let (_, groups) = db.vocab_view().unwrap();
    let group = groups.iter().find(|g| g.name == "HSK1").expect("the group");
    assert_eq!(
        group.device_id,
        db.device_id(),
        "the device that imported it"
    );
    assert_eq!(
        group.updated_at, "1970-01-01T00:00:00Z",
        "the stamp the schema backfill gives a group with no history"
    );
    finish(&[&dir]);
}
