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

use hanzi_core::ProgressStore;
use hanzi_store::backup::Backup;
use hanzi_store::Db;
use hanzi_sync::{
    full_log, local_baselines, read_attempts, recompute, sync, FolderStore, MergedAttempt,
    RemoteStore,
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
    let own: Vec<(String, i64)> = hanzi_sync::own_log(&db_b)
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
    assert_eq!(db_a.device_id(), card_owner(&db_a), "nobody changed name");

    finish(&[&dir_a, &dir_b, &dir_new, &old_store, &new_store]);
}

/// The device id this database's own attempts carry — a check that moving stores
/// did not make anybody rotate.
fn card_owner(db: &Db) -> String {
    hanzi_sync::own_log(db).unwrap()[0].device_id.clone()
}

/// A backup file of `db`, read back the way the restore screen reads it, merged into
/// `into` and the schedule rebuilt — what `import_backup` does in the app.
fn restore_backup(from: &Db, into: &Db) -> usize {
    let attempts = from.attempts(None).unwrap();
    let text = Backup::build(
        "2026-10-01T00:00:00Z",
        r#"{"nextId":1,"entries":[]}"#,
        &attempts,
    )
    .unwrap()
    .to_json()
    .unwrap();
    let backup = Backup::parse(&text).unwrap();
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
    assert_eq!(restore_backup(&db_old, &db_new), 3);
    record(&mut progress_new, '好', 50.0, "2026-10-02T09:00:00Z");
    let new = sync(&db_new, &remote).unwrap();
    assert_eq!(
        new.published, 4,
        "the restored three and its own one: {new:?}"
    );

    // They travel under the old phone's name, which is who made them — so a copy
    // published by anybody else is the same attempt, never a second one.
    let old_id = db_old.device_id().to_string();
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
        card_owner(&db_new),
        "the new phone kept its own name"
    );

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
