//! Two real databases, one folder, and the schedules that come out.
//!
//! `convergence.rs` proves the merge is right on lists. This proves the *plumbing*
//! around it is right — that a device publishes only its own work, that a peer's
//! attempts arrive attributed to the peer, that a schedule is rebuilt rather than
//! merged, and that a device which has never seen a character can end up with the
//! schedule for it.

use std::fs;
use std::path::{Path, PathBuf};

use hanzi_core::progress::CardState;
use hanzi_core::ProgressStore;
use hanzi_store::Db;
use hanzi_sync::{full_log, own_log, read_attempts, recompute, sync, FolderStore};

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("hanzi-sync-e2e-{name}-{}", std::process::id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).unwrap();
    path
}

fn finish(path: &Path) {
    fs::remove_dir_all(path).ok();
}

/// A device: a data directory, its database, and a schedule store over it.
fn device(name: &str) -> (PathBuf, Db, ProgressStore) {
    let path = scratch(name);
    let db = Db::open(&path).unwrap();
    let store = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    (path, db, store)
}

/// Record one attempt and commit it, the way the app does after a review.
fn record(store: &mut ProgressStore, ch: char, score: f32, at: &str) {
    store.record_at(ch, score, at).unwrap();
    store.save().unwrap();
}

/// Re-open the schedule store over the same database.
///
/// A `ProgressStore` holds the document in memory and writes through it, so after
/// a sync has rewritten the `progress_card` rows underneath it, the open store is
/// showing a stale schedule — and worse, its next `save` would write that stale
/// card back over the synced one. **A caller must reload after a sync.** There is
/// a test at the end of this file for exactly that hazard.
fn reload(db: &Db) -> ProgressStore {
    ProgressStore::open_with(Box::new(db.clone())).unwrap()
}

/// The schedule this device holds for a character.
fn card(store: &ProgressStore, ch: &str) -> CardState {
    store
        .document()
        .cards
        .get(ch)
        .unwrap_or_else(|| panic!("no card for {ch}"))
        .clone()
}

/// The same three attempts on the laptop, and a different three on the phone,
/// interleaved in time so the merged order is unambiguous.
fn laptop_practises(store: &mut ProgressStore) {
    record(store, '好', 88.0, "2026-09-19T09:00:00Z");
    record(store, '好', 91.0, "2026-09-19T21:00:00Z");
    record(store, '好', 34.0, "2026-09-21T09:00:00Z");
}

fn phone_practises(store: &mut ProgressStore) {
    record(store, '好', 95.0, "2026-09-19T12:00:00Z");
    record(store, '好', 62.0, "2026-09-20T08:00:00Z");
    record(store, '好', 80.0, "2026-09-22T07:00:00Z");
}

#[test]
fn two_databases_that_practised_apart_end_up_with_one_schedule() {
    // The whole point of the milestone, through the real store: two SQLite
    // databases, one folder, and one answer at the end.
    let (dir_a, db_a, mut progress_a) = device("converge-a");
    let (dir_b, db_b, mut progress_b) = device("converge-b");
    let shared = scratch("converge-store");
    let remote = FolderStore::open(&shared).unwrap();

    assert_ne!(db_a.device_id(), db_b.device_id(), "two devices, two names");
    laptop_practises(&mut progress_a);
    phone_practises(&mut progress_b);
    assert_eq!(card(&progress_a, "好").attempts, 3, "neither has seen the other");

    // The laptop syncs first, so the phone is the one that has to catch up.
    let first = sync(&db_a, &remote).unwrap();
    assert_eq!(first.published, 3);
    assert_eq!(first.pulled, 0, "there was nothing to pull yet");

    let second = sync(&db_b, &remote).unwrap();
    assert_eq!(second.published, 3);
    assert_eq!(second.pulled, 3, "and it learned the laptop's three");

    // The laptop now has to learn the phone's, and end up where the phone is.
    let third = sync(&db_a, &remote).unwrap();
    assert_eq!(third.pulled, 3);

    // Reloaded, because a sync rewrites the cards an open store is holding.
    let progress_a = reload(&db_a);
    let progress_b = reload(&db_b);
    let on_laptop = card(&progress_a, "好");
    let on_phone = card(&progress_b, "好");
    assert_eq!(on_laptop.attempts, 6, "three each");
    assert_eq!(on_laptop, on_phone, "one history, one schedule");
    assert_eq!(on_laptop.history.len(), 6, "and the whole log is in the window");

    // Syncing again does nothing at all: no publish, no pull, no rewrite.
    let idle = sync(&db_a, &remote).unwrap();
    assert!(idle.is_empty(), "a second sync should be a no-op: {idle:?}");
    assert_eq!(card(&reload(&db_a), "好"), on_laptop, "and change nothing");

    finish(&dir_a);
    finish(&dir_b);
    finish(&shared);
}

#[test]
fn a_device_publishes_its_own_attempts_and_attributes_a_peers_to_the_peer() {
    // The trap this module exists to avoid. After one sync the local log holds
    // both devices' attempts, so "everything in the log" and "my attempts" are
    // different questions — and publishing the first under this device's name
    // would relabel the peer's work, which is the identity the merge rests on.
    let (dir_a, db_a, mut progress_a) = device("attrib-a");
    let (dir_b, db_b, mut progress_b) = device("attrib-b");
    let shared = scratch("attrib-store");
    let remote = FolderStore::open(&shared).unwrap();

    laptop_practises(&mut progress_a);
    phone_practises(&mut progress_b);
    sync(&db_a, &remote).unwrap();
    sync(&db_b, &remote).unwrap();
    sync(&db_a, &remote).unwrap();

    // The log now holds six attempts, three of them the phone's.
    assert_eq!(full_log(&db_a).unwrap().len(), 6);
    assert_eq!(own_log(&db_a).unwrap().len(), 3, "only three are the laptop's");
    assert!(own_log(&db_a).unwrap().iter().all(|a| a.device_id == db_a.device_id()));

    // And each is attributed to whoever made it.
    let mut by_device = std::collections::BTreeMap::new();
    for attempt in full_log(&db_a).unwrap() {
        *by_device.entry(attempt.device_id.clone()).or_insert(0) += 1;
    }
    assert_eq!(by_device[db_a.device_id()], 3);
    assert_eq!(by_device[db_b.device_id()], 3, "the phone's are the phone's");

    // What was published is each device's own, under its own name, in its own
    // directory — which is what makes a dumb transport safe.
    let published = read_attempts(&FolderStore::open(&shared).unwrap()).unwrap();
    let mine = published
        .iter()
        .filter(|attempt| attempt.device_id == db_a.device_id())
        .count();
    let theirs = published
        .iter()
        .filter(|attempt| attempt.device_id == db_b.device_id())
        .count();
    assert_eq!((mine, theirs), (3, 3), "three each, and not six of one");

    let shards_in = |device: &str| {
        fs::read_dir(shared.join("devices").join(device).join("attempts"))
            .unwrap()
            .count()
    };
    assert_eq!(shards_in(db_a.device_id()), 1, "and no attempt was published twice");
    assert_eq!(shards_in(db_b.device_id()), 1);

    finish(&dir_a);
    finish(&dir_b);
    finish(&shared);
}

#[test]
fn a_device_that_has_never_seen_a_character_learns_its_whole_schedule() {
    // A new phone. It has no attempts of its own and no card; syncing has to leave
    // it holding a schedule it could not have computed from anything it had.
    let (dir_a, db_a, mut progress_a) = device("fresh-a");
    let (dir_c, db_c, _) = device("fresh-c");
    let shared = scratch("fresh-store");
    let remote = FolderStore::open(&shared).unwrap();

    laptop_practises(&mut progress_a);
    sync(&db_a, &remote).unwrap();
    let progress_c = reload(&db_c);
    assert!(
        progress_c.document().cards.is_empty(),
        "the new device starts with nothing"
    );

    let summary = sync(&db_c, &remote).unwrap();
    assert_eq!(summary.published, 0, "it has nothing of its own to publish");
    assert_eq!(summary.pulled, 3);
    assert_eq!(summary.recomputed, 1, "and it built the card from the log");

    assert_eq!(card(&reload(&db_c), "好"), card(&progress_a, "好"));

    finish(&dir_a);
    finish(&dir_c);
    finish(&shared);
}

#[test]
fn a_card_whose_log_is_short_of_its_count_is_left_alone() {
    // The migrated card, and the one case where folding would *lose* something.
    // `progress.json` kept only the newest twenty attempts, so a card can honestly
    // remember more attempts than the log holds. Rebuilding it from the log alone
    // would discard the part the log never had, so it is left as it is until the
    // baseline exists — reported, not guessed at.
    let path = scratch("short-log");
    {
        // Write a legacy document with a card that has five attempts behind it but
        // only two in its history, which is exactly what a migrated card looks
        // like. The real writer produces the format; only the history is trimmed.
        let mut progress = ProgressStore::open(path.join("progress.json")).unwrap();
        for i in 0..5 {
            let at = format!("2026-09-19T09:0{i}:00Z");
            progress.record_at('好', 80.0, &at).unwrap();
        }
        progress.save().unwrap();
        drop(progress);

        let file = path.join("progress.json");
        let text = fs::read_to_string(&file).unwrap();
        let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
        let history = document["cards"]["好"]["history"].as_array().unwrap();
        let newest_two = history[history.len() - 2..].to_vec();
        document["cards"]["好"]["history"] = serde_json::Value::Array(newest_two);
        fs::write(&file, serde_json::to_string(&document).unwrap()).unwrap();
    }

    // Opening the database imports that document: five attempts remembered, two
    // rows in the log.
    let db = Db::open(&path).unwrap();
    let progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(card(&progress, "好").attempts, 5, "the count survived the import");
    assert_eq!(full_log(&db).unwrap().len(), 2, "the log only ever had two");

    let (changed, left_alone) = recompute(&db).unwrap();
    assert_eq!(changed, 0, "nothing may be rebuilt from a log this short");
    assert_eq!(left_alone, 1, "and it is reported rather than silently skipped");

    let after = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(card(&after, "好").attempts, 5, "the schedule is untouched");

    finish(&path);
}

#[test]
fn an_open_schedule_store_that_is_not_reloaded_can_undo_a_sync() {
    // Why `reload` exists and why the app's sync must reload its store after one.
    // The store holds the document in memory and writes through it; a sync rewrites
    // the cards in the database underneath it; the store's next save writes its
    // stale copy back. This is the shape of a bug that would look like "sync
    // silently does nothing" — on one device, days later.
    let (dir_a, db_a, mut progress_a) = device("stale-a");
    let (dir_b, db_b, mut progress_b) = device("stale-b");
    let shared = scratch("stale-store");
    let remote = FolderStore::open(&shared).unwrap();

    laptop_practises(&mut progress_a);
    phone_practises(&mut progress_b);
    sync(&db_a, &remote).unwrap();
    sync(&db_b, &remote).unwrap();
    sync(&db_a, &remote).unwrap();
    assert_eq!(card(&reload(&db_a), "好").attempts, 6, "the sync worked");

    // `progress_a` still holds the three attempts it had before that, and saving it
    // again — which is what every review does — puts its stale card back.
    record(&mut progress_a, '好', 70.0, "2026-09-23T09:00:00Z");
    assert_eq!(
        card(&reload(&db_a), "好").attempts,
        4,
        "the unreloaded store wrote its snapshot back over the synced schedule"
    );

    // The damage is recoverable, and the reason is the design: the log kept every
    // attempt, so the next sync rebuilds the card from seven of them rather than
    // trusting the four the card claimed.
    let healed = sync(&db_a, &remote).unwrap();
    assert_eq!(healed.recomputed, 1);
    assert_eq!(
        card(&reload(&db_a), "好").attempts,
        7,
        "a schedule is derived, so a wrong one is recoverable"
    );

    finish(&dir_a);
    finish(&dir_b);
    finish(&shared);
}
