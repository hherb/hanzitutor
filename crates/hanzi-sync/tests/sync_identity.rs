//! A device's identity, and what it decides it has already published.
//!
//! `two_devices.rs` assumes every database is a device of its own and every store
//! is the one it has always synced to. These are the three ways that assumption
//! breaks in ordinary use — issue #12 — and what sync has to do about each:
//!
//! - **A database restored or cloned** onto another phone keeps the device id it
//!   was backed up with, and numbers its next attempt from where the *backup*
//!   ended. Publishing that would rewrite a shard every peer already holds.
//! - **A store that is empty** — a different Dropbox account, an app folder deleted
//!   to start over — has to be given the whole history, not just what is new since
//!   the last sync to some other store.
//! - **A backup file restored on a new phone** brings attempts made under the old
//!   phone's id, which no rule about "this device's own attempts" would publish.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use hanzi_core::{ProgressStore, Rating};
use hanzi_store::backup::Backup;
use hanzi_store::{Db, IncomingAttempt};
use hanzi_sync::{
    attempts_shard_name, baseline_shard_name, full_log, local_baselines, own_log, read_attempts,
    recompute, sync, write_attempts, FolderStore, MergedAttempt, RemoteStore,
};

fn scratch(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("hanzi-sync-identity-{name}-{}", std::process::id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).unwrap();
    path
}

fn finish(paths: &[&Path]) {
    for path in paths {
        fs::remove_dir_all(path).ok();
    }
}

/// A device: its database and a schedule store over it.
fn open(path: &Path) -> (Db, ProgressStore) {
    let db = Db::open(path).unwrap();
    let store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    (db, store)
}

/// Record one attempt and commit it, the way the app does after a review.
fn record(store: &mut ProgressStore, ch: char, score: f32, at: &str) {
    store.record_at(ch, score, at).unwrap();
    store.save().unwrap();
}

/// Copy a data directory whose database is closed, the way an OS backup or a
/// copied data folder carries it to another phone.
///
/// Every file is copied, the write-ahead log included, so the copy is the database
/// exactly as it stood — not merely its last checkpoint.
fn copy_dir(from: &Path, to: &Path) {
    fs::remove_dir_all(to).ok();
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.metadata().unwrap().is_file() {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

/// Every attempt the store holds, as `(device, seq, at)`, for comparing exactly.
fn on_remote(store: &dyn RemoteStore) -> BTreeSet<(String, i64, String)> {
    read_attempts(store)
        .unwrap()
        .into_iter()
        .map(|a| (a.device_id, a.seq, a.at))
        .collect()
}

/// The same, for a local log.
fn in_log(db: &Db) -> BTreeSet<(String, i64, String)> {
    full_log(db)
        .unwrap()
        .into_iter()
        .map(|a: MergedAttempt| (a.device_id, a.seq, a.at))
        .collect()
}

/// The schedule this database holds for a character, read fresh.
fn card(db: &Db, ch: &str) -> hanzi_core::progress::CardState {
    ProgressStore::open_with(Box::new(db.clone()))
        .unwrap()
        .document()
        .cards
        .get(ch)
        .unwrap_or_else(|| panic!("no card for {ch}"))
        .clone()
}

#[test]
fn a_restored_database_never_rewrites_a_published_shard_or_breaks_a_peer() {
    // The issue's reproduction, step by step.
    let dir_a = scratch("restore-a");
    let dir_backup = scratch("restore-backup");
    let dir_restored = scratch("restore-restored");
    let dir_c = scratch("restore-c");
    let shared = scratch("restore-store");
    let remote = FolderStore::open(&shared).unwrap();

    // 1. A publishes three attempts, and its database is backed up.
    let original_id = {
        let (db_a, mut progress_a) = open(&dir_a);
        record(&mut progress_a, '好', 88.0, "2026-09-19T09:00:00Z");
        record(&mut progress_a, '好', 91.0, "2026-09-19T21:00:00Z");
        record(&mut progress_a, '好', 34.0, "2026-09-21T09:00:00Z");
        assert_eq!(sync(&db_a, &remote).unwrap().published, 3);
        db_a.device_id().to_string()
    };
    copy_dir(&dir_a, &dir_backup);

    // 2. A publishes two more, in a shard of their own, and peer C pulls all five.
    let (db_a, mut progress_a) = open(&dir_a);
    record(&mut progress_a, '好', 70.0, "2026-09-22T09:00:00Z");
    record(&mut progress_a, '好', 75.0, "2026-09-23T09:00:00Z");
    assert_eq!(sync(&db_a, &remote).unwrap().published, 2);
    let (db_c, _) = open(&dir_c);
    assert_eq!(sync(&db_c, &remote).unwrap().pulled, 5);
    let before = on_remote(&remote);
    assert_eq!(before.len(), 5);

    // 3. The backup is restored on a new phone, which practises once and syncs.
    copy_dir(&dir_backup, &dir_restored);
    let (db_r, mut progress_r) = open(&dir_restored);
    assert_eq!(
        db_r.device_id(),
        original_id,
        "a restored database keeps its id"
    );
    record(&mut progress_r, '好', 99.0, "2026-10-01T09:00:00Z");
    let restored = sync(&db_r, &remote).expect("the restored phone syncs");
    assert_eq!(restored.published, 1, "its one new attempt");
    assert_eq!(
        restored.pulled, 2,
        "and it learns what the original did after the backup"
    );

    // Nothing that was on the store has changed: the new attempt went somewhere new.
    let after = on_remote(&remote);
    assert!(
        after.is_superset(&before),
        "a published attempt was rewritten: {after:?}"
    );
    assert_eq!(after.len(), 6);
    assert_ne!(
        db_r.device_id(),
        original_id,
        "the restored phone took a name of its own"
    );
    let new_id = db_r.device_id().to_string();
    assert!(
        after.contains(&(new_id.clone(), 1, "2026-10-01T09:00:00Z".to_string())),
        "its new attempt is numbered from 1 under its new name: {after:?}"
    );
    // And the change of name stays changed: the next database open reads it back.
    drop(progress_r);
    drop(db_r);
    let (db_r, mut progress_r) = open(&dir_restored);
    assert_eq!(db_r.device_id(), new_id);

    // 4. C is not broken: it syncs, and learns the restored phone's attempt.
    let c = sync(&db_c, &remote).expect("a peer is not broken by the restore");
    assert_eq!(c.pulled, 1);
    // Nor is the original, which goes on under its own name.
    let a = sync(&db_a, &remote).expect("the original is not broken either");
    assert_eq!(a.pulled, 1);
    assert_eq!(db_a.device_id(), original_id, "and keeps it");

    // Everyone holds the same six attempts and the same schedule.
    assert_eq!(in_log(&db_a), after);
    assert_eq!(in_log(&db_c), after);
    assert_eq!(in_log(&db_r), after);
    assert_eq!(card(&db_a, "好"), card(&db_c, "好"));
    assert_eq!(card(&db_r, "好"), card(&db_c, "好"));
    assert_eq!(card(&db_c, "好").attempts, 6);

    // The restored phone practises again and is numbered on from its new name, so
    // its next sync is an ordinary one.
    record(&mut progress_r, '好', 50.0, "2026-10-02T09:00:00Z");
    let next = sync(&db_r, &remote).unwrap();
    assert_eq!(next.published, 1);
    assert_eq!(db_r.device_id(), new_id, "no second change of name");
    assert!(on_remote(&remote).contains(&(new_id, 2, "2026-10-02T09:00:00Z".to_string())));
    assert_eq!(sync(&db_c, &remote).unwrap().pulled, 1);

    finish(&[&dir_a, &dir_backup, &dir_restored, &dir_c, &shared]);
}

#[test]
fn a_cloned_database_used_alongside_its_original_becomes_a_second_device() {
    // Two live copies of one database — a copied data directory — practise apart.
    // Whichever syncs second finds its own name holding an attempt it never made.
    let dir_a = scratch("clone-a");
    let dir_b = scratch("clone-b");
    let shared = scratch("clone-store");
    let remote = FolderStore::open(&shared).unwrap();

    {
        let (db_a, mut progress_a) = open(&dir_a);
        record(&mut progress_a, '人', 80.0, "2026-09-19T09:00:00Z");
        sync(&db_a, &remote).unwrap();
    }
    copy_dir(&dir_a, &dir_b);
    let (db_a, mut progress_a) = open(&dir_a);
    let (db_b, mut progress_b) = open(&dir_b);
    assert_eq!(db_a.device_id(), db_b.device_id());

    record(&mut progress_a, '人', 60.0, "2026-09-20T09:00:00Z");
    record(&mut progress_b, '人', 40.0, "2026-09-20T10:00:00Z");
    record(&mut progress_b, '人', 45.0, "2026-09-20T11:00:00Z");
    assert_eq!(sync(&db_a, &remote).unwrap().published, 1);
    let before = on_remote(&remote);
    let b = sync(&db_b, &remote).expect("the clone syncs");
    assert_eq!(b.published, 2);
    assert_eq!(b.pulled, 1, "and learns its original's attempt");
    assert_ne!(
        db_a.device_id(),
        db_b.device_id(),
        "and is a device of its own now"
    );
    assert!(on_remote(&remote).is_superset(&before));

    sync(&db_a, &remote).expect("the original is not broken");
    assert_eq!(in_log(&db_a), in_log(&db_b));
    assert_eq!(in_log(&db_a).len(), 4);
    assert_eq!(card(&db_a, "人"), card(&db_b, "人"));

    // The schedule store the clone opened *before* it was renamed goes on under the
    // new name, because the name is shared by every handle on the database rather
    // than copied into each. Recording under the old one would collide all over
    // again. (Its schedule is stale after the sync — see `two_devices.rs` — which
    // the next sync repairs; what is checked here is only whose name it writes.)
    let renamed = db_b.device_id().to_string();
    record(&mut progress_b, '人', 90.0, "2026-09-21T10:00:00Z");
    let own: Vec<(String, i64)> = own_log(&db_b)
        .unwrap()
        .into_iter()
        .map(|a| (a.device_id, a.seq))
        .collect();
    assert_eq!(
        own,
        vec![(renamed.clone(), 1), (renamed.clone(), 2), (renamed, 3)]
    );
    assert_eq!(sync(&db_b, &remote).unwrap().published, 1);
    assert_eq!(sync(&db_a, &remote).unwrap().pulled, 1);
    assert_eq!(in_log(&db_a), in_log(&db_b));
    assert_eq!(card(&db_a, "人"), card(&db_b, "人"));

    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn a_store_that_is_empty_is_given_the_whole_history() {
    // A different account, or the app folder deleted to start over: the device has
    // synced before, to somewhere else, and a watermark that remembered that would
    // publish nothing here.
    let dir_a = scratch("empty-a");
    let dir_b = scratch("empty-b");
    let dir_new = scratch("empty-new");
    let old_store = scratch("empty-old-store");
    let new_store = scratch("empty-new-store");
    let old = FolderStore::open(&old_store).unwrap();
    let new = FolderStore::open(&new_store).unwrap();

    let (db_a, mut progress_a) = open(&dir_a);
    let (db_b, mut progress_b) = open(&dir_b);
    let (id_a, id_b) = (db_a.device_id(), db_b.device_id());
    record(&mut progress_a, '好', 88.0, "2026-09-19T09:00:00Z");
    record(&mut progress_a, '好', 91.0, "2026-09-19T21:00:00Z");
    record(&mut progress_a, '好', 34.0, "2026-09-21T09:00:00Z");
    record(&mut progress_b, '好', 62.0, "2026-09-20T08:00:00Z");
    assert_eq!(sync(&db_a, &old).unwrap().published, 3);
    assert_eq!(sync(&db_b, &old).unwrap().published, 1);
    assert_eq!(sync(&db_a, &old).unwrap().pulled, 1);

    // A moves to the new store. Everything it holds goes there — its own three and
    // B's one, under B's name, because nobody else is going to put them there.
    let moved = sync(&db_a, &new).unwrap();
    assert_eq!(
        moved.published, 4,
        "the whole history, not just what is new: {moved:?}"
    );
    assert_eq!(on_remote(&new), in_log(&db_a));
    assert_eq!(sync(&db_a, &new).unwrap().published, 0, "and only once");

    // A new device on the new store receives all of it.
    let (db_new, _) = open(&dir_new);
    assert_eq!(sync(&db_new, &new).unwrap().pulled, 4);
    assert_eq!(card(&db_new, "好"), card(&db_a, "好"));

    // B follows A over. It has nothing the new store lacks, and writes nothing.
    let b = sync(&db_b, &new).unwrap();
    assert_eq!(b.published, 0, "{b:?}");
    assert_eq!(b.pulled, 0);
    // Nobody took a new name: the move is not mistaken for a copy.
    assert_eq!(db_a.device_id(), id_a);
    assert_eq!(db_b.device_id(), id_b);
    assert!(on_remote(&new)
        .iter()
        .all(|(device, ..)| *device == id_a || *device == id_b));

    finish(&[&dir_a, &dir_b, &dir_new, &old_store, &new_store]);
}

/// A backup file of `db`, read back the way the restore screen reads it, merged into
/// `into` and the schedule rebuilt — what `import_backup` does in the app.
fn restore_backup(from: &Db, into: &Db) -> usize {
    import_backup(into, &backup_of(from))
}

/// The backup file the app would write for `db`, as it stands now.
fn backup_of(db: &Db) -> String {
    let attempts = db.attempts(None).unwrap();
    Backup::build(
        "2026-10-01T00:00:00Z",
        r#"{"nextId":1,"entries":[]}"#,
        &attempts,
    )
    .unwrap()
    .to_json()
    .unwrap()
}

/// Restore a backup file into `into`, returning how many attempts were new.
fn import_backup(into: &Db, text: &str) -> usize {
    let backup = Backup::parse(text).unwrap();
    let added = into.merge_attempts(&backup.incoming_attempts()).unwrap();
    recompute(into, &local_baselines(into).unwrap()).unwrap();
    added
}

#[test]
fn attempts_restored_from_a_backup_file_reach_a_peer() {
    // The usual move to a new phone: the old one never synced, its backup file is
    // restored on the new one, and the new one syncs.
    let dir_old = scratch("backup-old");
    let dir_new = scratch("backup-new");
    let dir_peer = scratch("backup-peer");
    let shared = scratch("backup-store");
    let remote = FolderStore::open(&shared).unwrap();

    let (db_old, mut progress_old) = open(&dir_old);
    record(&mut progress_old, '好', 88.0, "2026-09-19T09:00:00Z");
    record(&mut progress_old, '好', 91.0, "2026-09-19T21:00:00Z");
    record(&mut progress_old, '人', 70.0, "2026-09-20T09:00:00Z");

    let (db_new, mut progress_new) = open(&dir_new);
    let (old_id, new_id) = (db_old.device_id(), db_new.device_id());
    assert_eq!(restore_backup(&db_old, &db_new), 3);
    record(&mut progress_new, '好', 50.0, "2026-10-02T09:00:00Z");
    let new = sync(&db_new, &remote).unwrap();
    assert_eq!(
        new.published, 4,
        "the restored three and its own one: {new:?}"
    );

    // They travel under the old phone's name, which is who made them — so a copy
    // published by anybody else is the same attempt, never a second one.
    let remote_now = on_remote(&remote);
    assert_eq!(
        remote_now
            .iter()
            .filter(|(device, ..)| *device == old_id)
            .count(),
        3
    );
    assert_eq!(
        db_new.device_id(),
        new_id,
        "the new phone kept its own name"
    );
    assert!(remote_now.contains(&(new_id.clone(), 1, "2026-10-02T09:00:00Z".to_string())));

    let (db_peer, _) = open(&dir_peer);
    assert_eq!(sync(&db_peer, &remote).unwrap().pulled, 4);
    assert_eq!(in_log(&db_peer), in_log(&db_new));
    assert_eq!(card(&db_peer, "好"), card(&db_new, "好"));
    assert_eq!(card(&db_peer, "人").attempts, 1);

    // The old phone turns out to be alive after all and syncs. Its three are already
    // there under its own name, so it publishes nothing and nobody counts them twice.
    let old = sync(&db_old, &remote).expect("the old phone is not broken");
    assert_eq!(old.published, 0, "{old:?}");
    assert_eq!(old.pulled, 1);
    assert_eq!(
        db_old.device_id(),
        old_id,
        "and keeps its name: the store agrees with it"
    );
    assert_eq!(db_new.device_id(), new_id);
    assert_eq!(sync(&db_peer, &remote).unwrap().pulled, 0);
    assert_eq!(card(&db_peer, "好").attempts, 3);

    finish(&[&dir_old, &dir_new, &dir_peer, &shared]);
}

#[test]
fn two_phones_restoring_the_same_backup_publish_it_once() {
    // Both write the old phone's attempts. Because they travel under the old
    // phone's name, the two copies are one attempt each, not two.
    let dir_old = scratch("twice-old");
    let dir_one = scratch("twice-one");
    let dir_two = scratch("twice-two");
    let dir_peer = scratch("twice-peer");
    let shared = scratch("twice-store");
    let remote = FolderStore::open(&shared).unwrap();

    let (db_old, mut progress_old) = open(&dir_old);
    record(&mut progress_old, '好', 88.0, "2026-09-19T09:00:00Z");
    record(&mut progress_old, '好', 91.0, "2026-09-19T21:00:00Z");
    let (db_one, _) = open(&dir_one);
    let (db_two, _) = open(&dir_two);
    restore_backup(&db_old, &db_one);
    restore_backup(&db_old, &db_two);

    assert_eq!(sync(&db_one, &remote).unwrap().published, 2);
    assert_eq!(
        sync(&db_two, &remote).unwrap().published,
        0,
        "already there"
    );
    let (db_peer, _) = open(&dir_peer);
    assert_eq!(sync(&db_peer, &remote).unwrap().pulled, 2);
    assert_eq!(card(&db_peer, "好").attempts, 2);

    finish(&[&dir_old, &dir_one, &dir_two, &dir_peer, &shared]);
}

#[test]
fn an_attempt_left_under_a_former_name_is_moved_rather_than_left_to_collide() {
    // The rename asks of every row inside its own transaction, so a review that
    // records while a sync runs cannot leave an attempt behind under the old name.
    // One can still get there another way — a build without that care, or a backup
    // from before the rename restored on top — and once the original publishes the
    // same number, the two would disagree on every pull from then on. This puts one
    // there directly and checks that the next sync moves it instead.
    let dir_a = scratch("straggler-a");
    let dir_backup = scratch("straggler-backup");
    let dir_r = scratch("straggler-r");
    let dir_c = scratch("straggler-c");
    let shared = scratch("straggler-store");
    let remote = FolderStore::open(&shared).unwrap();

    let old_id = {
        let (db_a, mut progress_a) = open(&dir_a);
        record(&mut progress_a, '好', 88.0, "2026-09-19T09:00:00Z");
        record(&mut progress_a, '好', 91.0, "2026-09-19T21:00:00Z");
        sync(&db_a, &remote).unwrap();
        db_a.device_id()
    };
    copy_dir(&dir_a, &dir_backup);
    let (db_a, mut progress_a) = open(&dir_a);
    record(&mut progress_a, '好', 70.0, "2026-09-22T09:00:00Z");
    sync(&db_a, &remote).unwrap();

    copy_dir(&dir_backup, &dir_r);
    let (db_r, mut progress_r) = open(&dir_r);
    record(&mut progress_r, '好', 99.0, "2026-10-01T09:00:00Z");
    sync(&db_r, &remote).unwrap();
    let new_id = db_r.device_id();
    assert_ne!(new_id, old_id);

    // A straggler under the old name, numbered where the original goes next.
    let straggler = IncomingAttempt {
        device_id: old_id.clone(),
        seq: 4,
        ch: "好".into(),
        at: "2026-10-01T10:00:00Z".into(),
        score: 55.0,
        rating: Rating::from_score(55.0),
        measures: None,
    };
    assert_eq!(db_r.merge_attempts(&[straggler]).unwrap(), 1);
    // And the original publishes its own attempt 4.
    record(&mut progress_a, '好', 60.0, "2026-10-02T09:00:00Z");
    sync(&db_a, &remote).unwrap();

    let repaired = sync(&db_r, &remote).expect("the straggler does not break the copy");
    assert_eq!(repaired.published, 1, "the straggler, under the new name");
    assert_eq!(repaired.pulled, 1, "and the original's attempt 4");
    assert!(on_remote(&remote).contains(&(new_id.clone(), 2, "2026-10-01T10:00:00Z".into())));
    assert!(on_remote(&remote).contains(&(old_id, 4, "2026-10-02T09:00:00Z".into())));
    assert_eq!(db_r.device_id(), new_id, "no further rename was needed");

    let (db_c, _) = open(&dir_c);
    sync(&db_c, &remote).expect("a peer is not broken");
    sync(&db_a, &remote).expect("nor is the original");
    assert_eq!(in_log(&db_a), on_remote(&remote));
    assert_eq!(in_log(&db_r), on_remote(&remote));
    assert_eq!(in_log(&db_c), on_remote(&remote));
    assert_eq!(on_remote(&remote).len(), 6);
    assert_eq!(card(&db_a, "好"), card(&db_r, "好"));
    assert_eq!(card(&db_c, "好"), card(&db_r, "好"));

    finish(&[&dir_a, &dir_backup, &dir_r, &dir_c, &shared]);
}

/// Write a `progress.json` whose card remembers `attempts` attempts but kept only
/// the newest `kept` of them — what the pre-M10 format produces once a history is
/// longer than it stored — and open a database over it, which imports it.
///
/// The same document as `two_devices.rs`'s helper of the same name.
fn migrated_short_log(path: &Path, ch: char, attempts: usize, kept: usize) {
    let mut progress = ProgressStore::open(path.join("progress.json")).unwrap();
    for i in 0..attempts {
        let at = format!("2026-09-19T09:{i:02}:00Z");
        progress.record_at(ch, 80.0, &at).unwrap();
    }
    progress.save().unwrap();
    drop(progress);

    let file = path.join("progress.json");
    let text = fs::read_to_string(&file).unwrap();
    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    let key = ch.to_string();
    let history = document["cards"][&key]["history"].as_array().unwrap();
    let newest = history[history.len() - kept..].to_vec();
    document["cards"][&key]["history"] = serde_json::Value::Array(newest);
    fs::write(&file, serde_json::to_string(&document).unwrap()).unwrap();

    // Imported here, so a copy of the directory is a copy of the database.
    let (db, _) = open(path);
    assert_eq!(full_log(&db).unwrap().len(), kept);
}

#[test]
fn a_copy_made_before_the_first_baseline_folds_from_the_originals() {
    // A database with a migrated card is copied before it has ever synced, so the
    // copy has no baseline of its own; the original then syncs and publishes one.
    // The copy must take the original's — it is the same history — rather than
    // capture one that claims the original's attempts and overwrites its file.
    let dir_a = scratch("baseline-copy-a");
    let dir_r = scratch("baseline-copy-r");
    let dir_c = scratch("baseline-copy-c");
    let shared = scratch("baseline-copy-store");
    let remote = FolderStore::open(&shared).unwrap();

    migrated_short_log(&dir_a, '好', 5, 2);
    copy_dir(&dir_a, &dir_r);

    let (db_a, mut progress_a) = open(&dir_a);
    sync(&db_a, &remote).unwrap();
    let id_a = db_a.device_id();
    let baseline_file = shared.join(baseline_shard_name(&id_a));
    let original_baseline = fs::read(&baseline_file).expect("the original published one");
    record(&mut progress_a, '好', 40.0, "2026-10-01T09:00:00Z");
    record(&mut progress_a, '好', 45.0, "2026-10-02T09:00:00Z");
    sync(&db_a, &remote).unwrap();
    let (db_c, _) = open(&dir_c);
    sync(&db_c, &remote).unwrap();

    let (db_r, mut progress_r) = open(&dir_r);
    record(&mut progress_r, '好', 95.0, "2026-10-01T12:00:00Z");
    sync(&db_r, &remote).expect("the copy syncs");
    assert_ne!(db_r.device_id(), id_a, "and is a device of its own");

    for _ in 0..2 {
        sync(&db_a, &remote).unwrap();
        sync(&db_c, &remote).unwrap();
        sync(&db_r, &remote).unwrap();
    }
    let on_a = card(&db_a, "好");
    assert_eq!(card(&db_r, "好"), on_a, "one history, one schedule");
    assert_eq!(card(&db_c, "好"), on_a);
    assert_eq!(
        on_a.attempts, 8,
        "five from the baseline, two since, one from the copy"
    );
    assert_eq!(
        fs::read(&baseline_file).unwrap(),
        original_baseline,
        "the original's baseline was not overwritten"
    );
    assert!(
        !shared.join(baseline_shard_name(&db_r.device_id())).exists(),
        "and the copy did not publish one of its own: it has nothing the original's lacks"
    );

    finish(&[&dir_a, &dir_r, &dir_c, &shared]);
}

#[test]
fn a_copy_that_has_to_rename_with_no_baseline_to_adopt_does_not_invent_one() {
    // The original synced only with a build that predates baselines, so the store
    // has none to adopt. A baseline the copy captured now would count the attempt
    // its rename is about to move, so it captures none and leaves the migrated card
    // alone — what a missing baseline has always meant.
    let dir_a = scratch("no-baseline-a");
    let dir_r = scratch("no-baseline-r");
    let shared = scratch("no-baseline-store");
    let remote = FolderStore::open(&shared).unwrap();

    migrated_short_log(&dir_a, '好', 5, 2);
    copy_dir(&dir_a, &dir_r);
    {
        // The older build: its own attempts in shards, a watermark, no baseline.
        let (db_a, mut progress_a) = open(&dir_a);
        record(&mut progress_a, '好', 40.0, "2026-10-01T09:00:00Z");
        write_attempts(&remote, &db_a.device_id(), &own_log(&db_a).unwrap()).unwrap();
        db_a.set_meta_value("sync:published_seq", "3").unwrap();
    }

    let (db_r, mut progress_r) = open(&dir_r);
    let id = db_r.device_id();
    record(&mut progress_r, '好', 95.0, "2026-10-01T12:00:00Z");
    // The schedule, without the recent-history window, which is read from the log
    // and so grows by the original's attempt whatever happens to the card.
    let schedule = |db: &Db| {
        let mut card = card(db, "好");
        card.history.clear();
        card
    };
    let before = schedule(&db_r);
    let summary = sync(&db_r, &remote).unwrap();
    assert_ne!(db_r.device_id(), id, "it renamed");
    assert_eq!(summary.left_alone, 1, "{summary:?}");
    assert_eq!(
        schedule(&db_r),
        before,
        "the migrated card is left as it was"
    );
    assert!(!shared.join(baseline_shard_name(&id)).exists());
    assert!(!shared.join(baseline_shard_name(&db_r.device_id())).exists());

    finish(&[&dir_a, &dir_r, &shared]);
}

#[test]
fn an_existing_database_with_a_watermark_publishes_only_what_is_new() {
    // The upgrade path: a database that has synced under the watermark, with its
    // shards where that build put them. Reading what is published from the store
    // must agree with the watermark exactly — no rename, nothing republished, no
    // shard touched.
    let dir_a = scratch("upgrade-a");
    let dir_c = scratch("upgrade-c");
    let shared = scratch("upgrade-store");
    let remote = FolderStore::open(&shared).unwrap();

    let (db_a, mut progress_a) = open(&dir_a);
    let id = db_a.device_id();
    record(&mut progress_a, '好', 88.0, "2026-09-19T09:00:00Z");
    record(&mut progress_a, '好', 91.0, "2026-09-19T21:00:00Z");
    record(&mut progress_a, '好', 34.0, "2026-09-21T09:00:00Z");
    write_attempts(&remote, &id, &own_log(&db_a).unwrap()).unwrap();
    db_a.set_meta_value("sync:published_seq", "3").unwrap();
    record(&mut progress_a, '好', 70.0, "2026-09-22T09:00:00Z");
    record(&mut progress_a, '好', 75.0, "2026-09-23T09:00:00Z");
    let newer: Vec<_> = own_log(&db_a)
        .unwrap()
        .into_iter()
        .filter(|attempt| attempt.seq > 3)
        .collect();
    write_attempts(&remote, &id, &newer).unwrap();
    db_a.set_meta_value("sync:published_seq", "5").unwrap();
    let (db_c, _) = open(&dir_c);
    assert_eq!(sync(&db_c, &remote).unwrap().pulled, 5);

    let shard_bytes = |first: i64| fs::read(shared.join(attempts_shard_name(&id, first))).unwrap();
    let (first, fourth) = (shard_bytes(1), shard_bytes(4));
    record(&mut progress_a, '好', 50.0, "2026-09-24T09:00:00Z");
    let upgraded = sync(&db_a, &remote).unwrap();
    assert_eq!(upgraded.published, 1, "only the new one: {upgraded:?}");
    assert_eq!(upgraded.pulled, 0);
    assert_eq!(db_a.device_id(), id, "no rename");
    assert_eq!(shard_bytes(1), first);
    // The new attempt tops up the open shard (#36) rather than starting one: the
    // old bytes unchanged, followed by the new line, and no shard named after it.
    let grown = shard_bytes(4);
    assert!(grown.starts_with(&fourth) && grown.len() > fourth.len());
    assert!(!shared.join(attempts_shard_name(&id, 6)).exists());
    assert_eq!(
        db_a.meta_value("sync:published_seq").unwrap().as_deref(),
        Some("6"),
        "and the watermark an older build would read moved with it"
    );
    assert_eq!(sync(&db_c, &remote).unwrap().pulled, 1);
    assert!(sync(&db_a, &remote).unwrap().is_empty());

    finish(&[&dir_a, &dir_c, &shared]);
}

#[test]
fn an_attempt_under_a_name_no_device_could_have_neither_breaks_nor_repeats_a_sync() {
    // A name that is not a plain one — a hand-edited backup, say — would be a path on
    // the store: `..` and a backslash are refused there, failing every sync, and `.`
    // writes somewhere sync never reads back, so it would be written again every time.
    // Such an attempt stays in the log and is simply not published.
    let dir_a = scratch("odd-names-a");
    let shared = scratch("odd-names-store");
    let remote = FolderStore::open(&shared).unwrap();
    let (db_a, mut progress_a) = open(&dir_a);
    let id = db_a.device_id();
    record(&mut progress_a, '好', 88.0, "2026-09-19T09:00:00Z");
    let odd: Vec<IncomingAttempt> = ["..", ".", "a\\b", "a/b"]
        .iter()
        .enumerate()
        .map(|(i, device)| IncomingAttempt {
            device_id: device.to_string(),
            seq: 1,
            ch: "人".into(),
            at: format!("2026-09-20T09:00:0{i}Z"),
            score: 70.0,
            rating: Rating::from_score(70.0),
            measures: None,
        })
        .collect();
    assert_eq!(db_a.merge_attempts(&odd).unwrap(), 4);

    let first = sync(&db_a, &remote).expect("an odd name does not fail the sync");
    assert_eq!(
        first.published, 1,
        "only the attempt with a real name: {first:?}"
    );
    let second = sync(&db_a, &remote).unwrap();
    assert_eq!(
        second.published, 0,
        "and nothing is written again: {second:?}"
    );
    assert!(on_remote(&remote).iter().all(|(device, ..)| *device == id));
    let written: Vec<_> = fs::read_dir(shared.join("devices"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(written, vec![id], "no directory for any of them");

    finish(&[&dir_a, &shared]);
}

#[test]
fn a_backup_from_before_the_rename_does_not_bring_its_moved_attempts_back() {
    // A copy records two attempts under the name it inherited and exports a backup.
    // Its next sync renames it and moves both. Restoring that backup afterwards must
    // not bring them back under the old name — one would come back as a second copy
    // of an attempt already published under the new one, counted twice everywhere.
    let dir_a = scratch("old-backup-a");
    let dir_b = scratch("old-backup-b");
    let dir_c = scratch("old-backup-c");
    let shared = scratch("old-backup-store");
    let remote = FolderStore::open(&shared).unwrap();

    let old_id = {
        let (db_a, mut progress_a) = open(&dir_a);
        record(&mut progress_a, '好', 88.0, "2026-09-19T09:00:00Z");
        record(&mut progress_a, '好', 91.0, "2026-09-19T21:00:00Z");
        sync(&db_a, &remote).unwrap();
        db_a.device_id()
    };
    copy_dir(&dir_a, &dir_b);
    let (db_a, mut progress_a) = open(&dir_a);
    record(&mut progress_a, '好', 70.0, "2026-09-22T09:00:00Z");
    sync(&db_a, &remote).unwrap();

    let (db_b, mut progress_b) = open(&dir_b);
    record(&mut progress_b, '好', 99.0, "2026-10-01T09:00:00Z");
    record(&mut progress_b, '好', 98.0, "2026-10-01T10:00:00Z");
    let before_the_rename = backup_of(&db_b);
    sync(&db_b, &remote).unwrap();
    assert_ne!(db_b.device_id(), old_id, "the copy renamed");

    assert_eq!(
        import_backup(&db_b, &before_the_rename),
        0,
        "every attempt in it is already here, one of them under the new name"
    );
    for db in [&db_b, &db_a, &db_b] {
        sync(db, &remote).expect("nobody is broken");
    }
    let (db_c, _) = open(&dir_c);
    sync(&db_c, &remote).unwrap();

    assert_eq!(
        db_a.device_id(),
        old_id,
        "the original had no reason to rename"
    );
    let stored = on_remote(&remote);
    let distinct: BTreeSet<&String> = stored.iter().map(|(_, _, at)| at).collect();
    assert_eq!(stored.len(), 5, "{stored:?}");
    assert_eq!(distinct.len(), 5, "each attempt once: {stored:?}");
    for db in [&db_a, &db_b, &db_c] {
        assert_eq!(in_log(db), stored);
        assert_eq!(card(db, "好").attempts, 5);
    }
    assert_eq!(card(&db_a, "好"), card(&db_b, "好"));
    assert_eq!(card(&db_c, "好"), card(&db_b, "好"));

    finish(&[&dir_a, &dir_b, &dir_c, &shared]);
}
