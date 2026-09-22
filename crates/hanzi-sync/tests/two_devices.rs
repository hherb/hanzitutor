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
use hanzi_core::{CursorStore, ProgressStore, VocabStore};
use hanzi_store::Db;
use hanzi_sync::{full_log, own_log, read_attempts, recompute, sync, Baselines, FolderStore};

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

/// Write a `progress.json` whose card remembers five attempts but kept only two of
/// them, which is what the pre-M10 format produces once a history is longer than the
/// twenty it stored.
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
}

#[test]
fn a_card_whose_log_is_short_of_its_count_keeps_its_schedule_through_the_baseline() {
    // M13's baseline, and the acceptance criterion it was written for. The migrated
    // card remembers five attempts and its log holds two, so folding those two would
    // rebuild a schedule out of the tail of a history — wrong on the device that has
    // them and differently wrong on a device that has only some of them. The baseline
    // is the state the missing part left behind, and it travels, so the device that
    // never saw the JSON ends up with the same schedule as the one that did.
    let path_a = scratch("baseline-a");
    migrated_short_log(&path_a, '好', 5, 2);
    let db_a = Db::open(&path_a).unwrap();
    assert_eq!(card(&reload(&db_a), "好").attempts, 5, "the count survived the import");
    assert_eq!(full_log(&db_a).unwrap().len(), 2, "the log only ever had two");

    let shared = scratch("baseline-store");
    let remote = FolderStore::open(&shared).unwrap();
    // A's first sync is where the baseline is captured and published.
    sync(&db_a, &remote).unwrap();
    let on_a = card(&reload(&db_a), "好");

    let (dir_b, db_b, _) = device("baseline-b");
    sync(&db_b, &remote).unwrap();
    let on_b = card(&reload(&db_b), "好");

    assert_eq!(on_b, on_a, "both devices fold the same card from the same baseline");
    assert_eq!(on_b.attempts, 5, "including the three attempts the log never held");
    assert_eq!(
        on_b.ease, on_a.ease,
        "and the ease the lost attempts had accumulated, which no fold of the two \
         surviving rows could have reached"
    );

    // The proof that it did not stop at copying the card over: B's new attempt has
    // to be folded *on top of* the baseline. Ignoring it is what "left alone" did,
    // and it is why this was worth building.
    let mut progress_b = reload(&db_b);
    record(&mut progress_b, '好', 90.0, "2026-10-01T09:00:00Z");
    sync(&db_b, &remote).unwrap();
    sync(&db_a, &remote).unwrap();

    let after_a = card(&reload(&db_a), "好");
    let after_b = card(&reload(&db_b), "好");
    assert_eq!(after_a, after_b, "one history, one schedule");
    assert_eq!(after_a.attempts, 6, "five from the baseline and one since");
    assert!(after_a.last_score == Some(90.0), "the new attempt is the latest");

    // And a third sync still has nothing to do.
    let idle = sync(&db_a, &remote).unwrap();
    assert!(idle.is_empty(), "a second sync should be a no-op: {idle:?}");

    finish(&path_a);
    finish(&dir_b);
    finish(&shared);
}

#[test]
fn the_baseline_does_not_suppress_a_character_whose_log_is_complete() {
    // The range a baseline covers is a stretch of one device's log, and it says
    // which rows the *cards it names* already hold — nothing about the other
    // characters that device wrote in the same stretch. Treating it as global would
    // quietly drop them and leave their cards at whatever they were.
    let path_a = scratch("baseline-mixed");
    // 好 migrated with a history that does not go back to its first attempt.
    migrated_short_log(&path_a, '好', 5, 2);
    let db_a = Db::open(&path_a).unwrap();

    // 猫 is practised afterwards, so its log is complete and it needs no baseline.
    let mut progress_a = reload(&db_a);
    record(&mut progress_a, '猫', 70.0, "2026-09-25T09:00:00Z");
    record(&mut progress_a, '猫', 80.0, "2026-09-26T09:00:00Z");

    let shared = scratch("baseline-mixed-store");
    let remote = FolderStore::open(&shared).unwrap();
    sync(&db_a, &remote).unwrap();
    let on_a = card(&reload(&db_a), "猫");
    assert_eq!(on_a.attempts, 2);

    let (dir_b, db_b, _) = device("baseline-mixed-b");
    sync(&db_b, &remote).unwrap();
    let on_b = card(&reload(&db_b), "猫");
    assert_eq!(on_b, on_a, "the complete character crossed intact");
    assert_eq!(on_b.attempts, 2);

    finish(&path_a);
    finish(&dir_b);
    finish(&shared);
}

#[test]
fn a_card_whose_log_is_short_of_its_count_is_left_alone() {
    // The same card as the baseline test above, on a device that has never published
    // a baseline — a peer whose own baseline has not arrived, or a database whose
    // capture has not happened yet. Leaving the schedule alone is what this did
    // before the baseline existed, and it is deliberately still the fallback: a
    // missing baseline has to degrade to no answer, never to a wrong one.
    let path = scratch("short-log");
    migrated_short_log(&path, '好', 5, 2);

    // Opening the database imports that document: five attempts remembered, two
    // rows in the log.
    let db = Db::open(&path).unwrap();
    let progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    assert_eq!(card(&progress, "好").attempts, 5, "the count survived the import");
    assert_eq!(full_log(&db).unwrap().len(), 2, "the log only ever had two");

    let (changed, left_alone) = recompute(&db, &Baselines::default()).unwrap();
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

#[test]
fn three_devices_all_end_up_with_the_same_log() {
    // The configuration this is actually used in: a Mac, an iPhone and an Android
    // phone, all syncing against one app folder. Two devices prove the merge; three
    // prove that reading *every* peer's directory works, which is the one thing a
    // pair cannot exercise — a merge that quietly only ever looked at one other
    // device would pass every two-device test there is.
    let (dir_a, db_a, mut progress_a) = device("three-a");
    let (dir_b, db_b, mut progress_b) = device("three-b");
    let (dir_c, db_c, mut progress_c) = device("three-c");
    let shared = scratch("three-store");
    let remote = FolderStore::open(&shared).unwrap();

    record(&mut progress_a, '好', 88.0, "2026-09-19T09:00:00Z");
    record(&mut progress_b, '好', 91.0, "2026-09-19T12:00:00Z");
    record(&mut progress_c, '好', 74.0, "2026-09-19T18:00:00Z");

    // Two rounds, in a fixed order. The first publishes and lets the last device
    // collect; the second is what carries the late publishers back to the first, so
    // that a fixed order still converges. Real devices do this in whatever order
    // they are opened, which is why the fix for it is "sync again", not a schedule.
    for _ in 0..2 {
        for db in [&db_a, &db_b, &db_c] {
            sync(db, &remote).unwrap();
        }
    }

    let on_a = card(&reload(&db_a), "好");
    let on_b = card(&reload(&db_b), "好");
    let on_c = card(&reload(&db_c), "好");
    assert_eq!(on_a.attempts, 3, "one attempt from each device");
    assert_eq!(on_a, on_b, "three logs, one schedule");
    assert_eq!(on_b, on_c, "and the third agrees as well");

    // Settled: nothing left to send or receive anywhere.
    for db in [&db_a, &db_b, &db_c] {
        assert!(
            sync(db, &remote).unwrap().is_empty(),
            "a settled three-way sync should be a no-op"
        );
    }

    finish(&dir_a);
    finish(&dir_b);
    finish(&dir_c);
    finish(&shared);
}

/// Three devices sharing one vocabulary list, which is the harder half of sync.
///
/// An attempt is appended and never changes, so the schedule's merge is a union. An
/// entry is edited and deleted, so two devices can hold different versions of one
/// row — and the point of these tests is that they end up agreeing about which
/// version is the real one, without either device having to be told.
#[test]
fn a_vocabulary_list_added_on_one_device_appears_on_the_others() {
    let (dir_a, db_a, _) = device("vocab-a");
    let (dir_b, db_b, _) = device("vocab-b");
    let (dir_c, db_c, _) = device("vocab-c");
    let shared = scratch("vocab-store");
    let remote = FolderStore::open(&shared).unwrap();

    // One device writes something by hand, which is the only way an entry is made.
    {
        let mut vocab = VocabStore::open_with(Box::new(db_a.clone())).unwrap();
        vocab
            .add_entry("学习", "xuéxí", "to study", Some("Lesson 3"))
            .unwrap();
        vocab.save().unwrap();
    }

    // The other two have never heard of it.
    for db in [&db_b, &db_c] {
        let vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
        assert!(vocab.entries().is_empty(), "nothing yet");
        assert!(vocab.groups().is_empty());
    }

    for _ in 0..2 {
        for db in [&db_a, &db_b, &db_c] {
            sync(db, &remote).unwrap();
        }
    }

    for (db, who) in [(&db_b, "the second device"), (&db_c, "the third")] {
        let vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
        assert_eq!(vocab.entries().len(), 1, "{who} should have the entry");
        assert_eq!(vocab.entries()[0].text, "学习");
        assert_eq!(vocab.entries()[0].meaning, "to study");
        assert_eq!(
            vocab.entries()[0].group.as_deref(),
            Some("Lesson 3"),
            "and the group it was filed under"
        );
        assert_eq!(vocab.groups(), ["Lesson 3"]);
    }

    finish(&dir_a);
    finish(&dir_b);
    finish(&dir_c);
    finish(&shared);
}

#[test]
fn an_entry_edited_on_two_devices_ends_up_the_same_way_on_both() {
    // The genuine disagreement, and the one case a merge cannot be clever about.
    // One of the edits wins whole; what matters is that both devices agree on which,
    // because a learner seeing two different lists has no way to tell them apart.
    let (dir_a, db_a, _) = device("edit-a");
    let (dir_b, db_b, _) = device("edit-b");
    let shared = scratch("edit-store");
    let remote = FolderStore::open(&shared).unwrap();

    let id_a = {
        let mut vocab = VocabStore::open_with(Box::new(db_a.clone())).unwrap();
        let id = vocab
            .add_entry("学习", "xuéxí", "to study", None)
            .unwrap()
            .id;
        vocab.save().unwrap();
        id
    };
    // Get it across first, so both devices are editing the same entry rather than
    // one of them inventing it.
    sync(&db_a, &remote).unwrap();
    sync(&db_b, &remote).unwrap();

    let id_b = {
        let vocab = VocabStore::open_with(Box::new(db_b.clone())).unwrap();
        assert_eq!(vocab.entries().len(), 1, "the entry travelled");
        vocab.entries()[0].id
    };

    // Two edits, and the second one is deliberately the later of the two. Backdating
    // the first is what makes "later" mean something: both happen inside one second.
    {
        let mut vocab = VocabStore::open_with(Box::new(db_a.clone())).unwrap();
        vocab.update_entry(id_a, "xuéxí", "the laptop's meaning", None).unwrap();
        vocab.save().unwrap();
    }
    backdate(&db_a, id_a, "2000-01-01T00:00:00Z");
    {
        let mut vocab = VocabStore::open_with(Box::new(db_b.clone())).unwrap();
        vocab.update_entry(id_b, "xuéxí", "the phone's meaning", None).unwrap();
        vocab.save().unwrap();
    }

    for _ in 0..2 {
        for db in [&db_a, &db_b] {
            sync(db, &remote).unwrap();
        }
    }

    let on_a = VocabStore::open_with(Box::new(db_a.clone())).unwrap();
    let on_b = VocabStore::open_with(Box::new(db_b.clone())).unwrap();
    assert_eq!(on_a.entries().len(), 1);
    assert_eq!(
        on_a.entries()[0].meaning, on_b.entries()[0].meaning,
        "one of the edits won, and both devices say which"
    );
    assert_eq!(
        on_a.entries()[0].meaning, "the phone's meaning",
        "and it is the later one"
    );

    finish(&dir_a);
    finish(&dir_b);
    finish(&shared);
}

#[test]
fn an_entry_removed_on_one_device_stays_removed_on_the_other() {
    // Without a tombstone the peer's own copy would be the only record left and the
    // entry would come straight back — which is exactly what "sync deleted my work"
    // usually turns out to be.
    let (dir_a, db_a, _) = device("remove-a");
    let (dir_b, db_b, _) = device("remove-b");
    let shared = scratch("remove-store");
    let remote = FolderStore::open(&shared).unwrap();

    let id = {
        let mut vocab = VocabStore::open_with(Box::new(db_a.clone())).unwrap();
        let id = vocab
            .add_entry("学习", "xuéxí", "to study", None)
            .unwrap()
            .id;
        vocab.save().unwrap();
        id
    };
    sync(&db_a, &remote).unwrap();
    sync(&db_b, &remote).unwrap();
    assert_eq!(
        VocabStore::open_with(Box::new(db_b.clone())).unwrap().entries().len(),
        1,
        "both devices have it"
    );

    // Removal on the laptop, and the laptop's stamp is aged so it cannot lose a
    // same-second tie to the copy that was just delivered.
    {
        let mut vocab = VocabStore::open_with(Box::new(db_a.clone())).unwrap();
        vocab.remove_entry(id).unwrap();
        vocab.save().unwrap();
    }

    for _ in 0..2 {
        for db in [&db_a, &db_b] {
            sync(db, &remote).unwrap();
        }
    }

    for db in [&db_a, &db_b] {
        let vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
        assert!(
            vocab.entries().is_empty(),
            "the removal travelled instead of the entry coming back"
        );
    }

    finish(&dir_a);
    finish(&dir_b);
    finish(&shared);
}

#[test]
fn the_course_position_follows_whichever_device_moved_it_last() {
    let (dir_a, db_a, _) = device("cursor-a");
    let (dir_b, db_b, _) = device("cursor-b");
    let shared = scratch("cursor-store");
    let remote = FolderStore::open(&shared).unwrap();

    {
        let mut cursor = CursorStore::open_with(Box::new(db_a.clone())).unwrap();
        cursor.set_index(120);
        cursor.save().unwrap();
    }
    // Aged, so that the other device's move is unambiguously *later* rather than a
    // same-second tie. A tie is settled by device id, which is deterministic and
    // converges — but it is not "the later move wins", so a test that wants to see
    // the later move win has to make it later.
    backdate_cursor(&db_a, "2000-01-01T00:00:00Z");
    sync(&db_a, &remote).unwrap();

    // The other device is behind, and picks up where the first one is.
    sync(&db_b, &remote).unwrap();
    let on_b = CursorStore::open_with(Box::new(db_b.clone())).unwrap();
    assert_eq!(on_b.view().index, 120, "the position travelled");

    // Then it moves further along, and the first device follows it.
    {
        let mut cursor = CursorStore::open_with(Box::new(db_b.clone())).unwrap();
        cursor.set_index(340);
        cursor.save().unwrap();
    }
    for _ in 0..2 {
        for db in [&db_a, &db_b] {
            sync(db, &remote).unwrap();
        }
    }
    for db in [&db_a, &db_b] {
        let cursor = CursorStore::open_with(Box::new(db.clone())).unwrap();
        assert_eq!(cursor.view().index, 340, "both devices agree where they are");
    }

    finish(&dir_a);
    finish(&dir_b);
    finish(&shared);
}

/// Age the course position's stamp, for the same reason as [`backdate`].
fn backdate_cursor(db: &Db, stamp: &str) {
    rusqlite::Connection::open(db.path())
        .unwrap()
        .execute(
            "UPDATE course_cursor SET updated_at = ?1 WHERE only_row = 1",
            rusqlite::params![stamp],
        )
        .unwrap();
}

/// Age an entry's stamp, so that "later" is a fact rather than a same-second tie.
fn backdate(db: &Db, id: u64, stamp: &str) {
    rusqlite::Connection::open(db.path())
        .unwrap()
        .execute(
            "UPDATE vocab_entry SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![stamp, id as i64],
        )
        .unwrap();
}

/// The uuid of a local entry id — the entry's name on every device.
fn entry_uuid(db: &Db, id: u64) -> String {
    rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row(
            "SELECT uuid FROM vocab_entry WHERE id = ?1",
            [id as i64],
            |row| row.get(0),
        )
        .unwrap()
}

/// A group's stored position, as a uuid, which is what two devices can compare.
fn position_of(db: &Db, group: &str) -> Option<String> {
    db.vocab_cursors()
        .unwrap()
        .into_iter()
        .find(|cursor| cursor.group_name == group)
        .and_then(|cursor| cursor.entry_uuid)
}

#[test]
fn a_group_position_follows_the_device_that_drilled_last() {
    // The end-to-end version of the per-group rule, through the real adapter:
    // publish, pull and apply, on two databases.
    let (dir_a, db_a, _) = device("vocab-cursor-a");
    let (dir_b, db_b, _) = device("vocab-cursor-b");
    let shared = scratch("vocab-cursor-store");
    let remote = FolderStore::open(&shared).unwrap();

    // Two groups, both devices, so that the per-group settling is exercised
    // rather than a single global position.
    let mut ids = Vec::new();
    for (db, tag) in [(&db_a, "a"), (&db_b, "b")] {
        let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
        let silu = vocab.add_entry("学生", "", "", Some("SiLu")).unwrap().id;
        let silu2 = vocab.add_entry("姐姐", "", "", Some("SiLu")).unwrap().id;
        let hsk = vocab.add_entry("中国", "", "", Some("HSK 1")).unwrap().id;
        vocab.save().unwrap();
        ids.push((tag, silu, silu2, hsk));
    }
    let (_, a_silu, a_silu2, _) = ids[0];
    let (_, _, _, b_hsk) = ids[1];

    // The laptop reaches the second entry of SiLu; the phone reaches the third
    // of HSK 1.
    db_a.set_vocab_cursor("SiLu", Some(a_silu2)).unwrap();
    db_b.set_vocab_cursor("HSK 1", Some(b_hsk)).unwrap();

    for _ in 0..2 {
        for db in [&db_a, &db_b] {
            sync(db, &remote).unwrap();
        }
    }

    // Both devices hold both groups' positions. What is compared is the **uuid**,
    // because that is the entry's name everywhere: the two devices resolve the
    // same position to *different local ids*, which is exactly why the row stores
    // a uuid in the first place. Each uuid is read on the device that created the
    // entry — an id is meaningless anywhere else.
    let silu_uuid = entry_uuid(&db_a, a_silu2);
    let hsk_uuid = entry_uuid(&db_b, b_hsk);
    for db in [&db_a, &db_b] {
        assert_eq!(position_of(db, "SiLu").as_deref(), Some(silu_uuid.as_str()));
        assert_eq!(position_of(db, "HSK 1").as_deref(), Some(hsk_uuid.as_str()));
    }
    // And the two devices really did number them differently, which is the point
    // the uuid is carrying.
    assert_ne!(
        db_a.vocab_cursor("HSK 1").unwrap(),
        Some(b_hsk),
        "the phone's local id for its own entry is not the laptop's"
    );

    // The laptop now moves further in SiLu, and the phone follows. Nothing is
    // aged: the write is this device's own, so its revision is higher than the
    // copy the phone holds and the stamp settles it either way.
    db_a.set_vocab_cursor("SiLu", Some(a_silu)).unwrap();
    for _ in 0..2 {
        for db in [&db_a, &db_b] {
            sync(db, &remote).unwrap();
        }
    }
    let moved = entry_uuid(&db_a, a_silu);
    for db in [&db_a, &db_b] {
        assert_eq!(
            position_of(db, "SiLu").as_deref(),
            Some(moved.as_str()),
            "the newer move won"
        );
        assert_eq!(
            position_of(db, "HSK 1").as_deref(),
            Some(hsk_uuid.as_str()),
            "the other group was not dragged by it"
        );
    }

    finish(&dir_a);
    finish(&dir_b);
}
