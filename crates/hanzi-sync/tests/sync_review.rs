//! The follow-up to issue #35: what an independent review of the first fix found.
//!
//! Each test here is a case the review reproduced. They are kept apart from
//! `sync_merge.rs` so each file reads as one story: that one is the issue as
//! reported, this one is the edges of the fix. The last test is the review's
//! property harness: random histories on three or four devices, with skewed clocks,
//! through the real merge, checked for convergence.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use hanzi_core::time::{add_seconds, iso8601_from_unix, now_iso8601, parse_iso8601};
use hanzi_core::{CursorStore, ProgressStore, VocabStore};
use hanzi_store::{Db, SyncedEntry};
use hanzi_sync::{full_log, merge_vocab, read_attempts, sync, FolderStore, RemoteStore};

fn scratch(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("hanzi-sync-review-{name}-{}", std::process::id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).unwrap();
    path
}

/// A device as the app starts one: the database open and the list loaded once.
fn device(name: &str) -> (PathBuf, Db) {
    let path = scratch(name);
    let db = Db::open(&path).unwrap();
    VocabStore::open_with(Box::new(db.clone())).unwrap();
    (path, db)
}

fn finish(paths: &[&PathBuf]) {
    for path in paths {
        fs::remove_dir_all(path).ok();
    }
}

fn practise(db: &Db, ch: char, at: &str) {
    let mut progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    progress.record_at(ch, 80.0, at).unwrap();
    progress.save().unwrap();
}

fn own_shards(remote: &FolderStore, db: &Db) -> Vec<String> {
    let prefix = format!("devices/{}/attempts/", db.device_id());
    remote
        .list()
        .unwrap()
        .into_iter()
        .map(|entry| entry.name)
        .filter(|name| name.starts_with(&prefix) && name.ends_with(".jsonl"))
        .collect()
}

fn attempts_of(db: &Db, of: &Db) -> BTreeSet<String> {
    full_log(db)
        .unwrap()
        .into_iter()
        .filter(|attempt| attempt.device_id == of.device_id())
        .map(|attempt| format!("{}@{}", attempt.ch, attempt.at))
        .collect()
}

// ---- a device's own damaged shard ---------------------------------------------

#[test]
fn a_devices_own_damaged_shard_is_written_again_and_its_peers_recover() {
    // A shard is never renamed and the publish only writes what is past the
    // watermark, so a damaged shard of A's used to stay damaged — and every peer
    // left A out whole, for good, new attempts included.
    let (dir_a, a) = device("own-a");
    let (dir_b, b) = device("own-b");
    let shared = scratch("own-store");
    let remote = FolderStore::open(&shared).unwrap();

    practise(&a, '好', "2026-09-20T09:00:00Z");
    sync(&a, &remote).unwrap();
    practise(&a, '你', "2026-09-21T09:00:00Z");
    sync(&a, &remote).unwrap();
    // Both publishes went into one open shard (#36). Split it as an earlier build
    // would have written it, one shard per publish, so that a damaged *earlier*
    // shard — one with a successor — is covered as well as the last.
    let whole = own_shards(&remote, &a).remove(0);
    let text = String::from_utf8(remote.get(&whole).unwrap()).unwrap();
    let mut split = text.lines();
    remote
        .put(&whole, format!("{}\n", split.next().unwrap()).as_bytes())
        .unwrap();
    remote
        .put(
            &hanzi_sync::attempts_shard_name(&a.device_id(), 2),
            format!("{}\n", split.next().unwrap()).as_bytes(),
        )
        .unwrap();
    sync(&b, &remote).unwrap();
    let shards = own_shards(&remote, &a);
    assert_eq!(shards.len(), 2, "one shard per publish: {shards:?}");

    // Each of A's two shards in turn, so a damaged *later* shard is covered too.
    for (round, damaged) in shards.iter().enumerate() {
        remote.put(damaged, b"{truncated").unwrap();
        let ch = ['学', '习'][round];
        practise(&a, ch, &format!("2026-09-2{}T09:00:00Z", 2 + round));

        let on_a = sync(&a, &remote).unwrap();
        assert_eq!(on_a.repaired.len(), 1, "A mended its own file: {on_a:?}");
        assert!(
            on_a.skipped.is_empty(),
            "and has nothing left out: {on_a:?}"
        );
        assert_eq!(
            on_a.notice().unwrap(),
            "A file this device wrote to Dropbox was damaged; it has been written again."
        );
        assert!(read_attempts(&remote).is_ok(), "every shard reads again");

        let on_b = sync(&b, &remote).unwrap();
        assert!(
            on_b.skipped.is_empty(),
            "B no longer leaves A out: {on_b:?}"
        );
        assert_eq!(
            attempts_of(&b, &a),
            attempts_of(&a, &a),
            "B has all of A's attempts"
        );
    }

    // Settled: a further sync mends nothing.
    assert!(sync(&a, &remote).unwrap().repaired.is_empty());
    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn a_restored_copy_with_fewer_attempts_does_not_overwrite_the_damaged_file() {
    // The damaged file is the only copy of what it held. A database that holds
    // fewer of this name's attempts — restored from a backup taken before the end
    // of its log, here made by losing the last attempt — would destroy the rest by
    // writing what it has. It leaves the file, and says so; the database that
    // wrote the file mends it.
    let (dir_a, a) = device("restored-a");
    let shared = scratch("restored-store");
    let remote = FolderStore::open(&shared).unwrap();
    practise(&a, '好', "2026-09-20T09:00:00Z");
    sync(&a, &remote).unwrap();
    practise(&a, '你', "2026-09-21T09:00:00Z");
    practise(&a, '学', "2026-09-22T09:00:00Z");
    sync(&a, &remote).unwrap();
    let later = own_shards(&remote, &a).last().unwrap().clone();

    // The copy, which has lost the last attempt of the later shard.
    let dir_copy = scratch("restored-copy");
    rusqlite::Connection::open(a.path())
        .unwrap()
        .execute(
            "VACUUM INTO ?1",
            [dir_copy.join(Db::FILE_NAME).to_string_lossy().to_string()],
        )
        .unwrap();
    rusqlite::Connection::open(dir_copy.join(Db::FILE_NAME))
        .unwrap()
        .execute("DELETE FROM attempt WHERE ch = '学'", [])
        .unwrap();
    remote.put(&later, b"{truncated").unwrap();

    let copy = Db::open(&dir_copy).unwrap();
    assert_eq!(copy.device_id(), a.device_id(), "the copy carries the name");
    let on_copy = sync(&copy, &remote).unwrap();
    assert!(on_copy.repaired.is_empty(), "{on_copy:?}");
    assert!(
        on_copy.skipped.iter().any(|s| s.name == later && s.own),
        "{on_copy:?}"
    );
    assert!(
        on_copy.notice().unwrap().contains("cannot write it again"),
        "{:?}",
        on_copy.notice()
    );
    assert_eq!(
        remote.get(&later).unwrap(),
        b"{truncated",
        "the file is left as it was, not replaced with the part the copy knows"
    );

    // The database that wrote it does mend it, whole.
    let on_a = sync(&a, &remote).unwrap();
    assert_eq!(on_a.repaired.len(), 1, "{on_a:?}");
    assert_eq!(read_attempts(&remote).unwrap().len(), 3);
    finish(&[&dir_a, &dir_copy, &shared]);
}

#[test]
fn a_shard_line_in_a_newer_format_is_never_written_over_and_is_reported_as_newer() {
    // Sound JSON this build cannot decode — here a rating it has never heard of —
    // was written by a newer build, not broken in transit. Rewriting it would drop
    // what the newer build said, so not even the device that owns the name does.
    let (dir_a, a) = device("newer-line-a");
    let (dir_b, b) = device("newer-line-b");
    let shared = scratch("newer-line-store");
    let remote = FolderStore::open(&shared).unwrap();
    practise(&a, '好', "2026-09-20T09:00:00Z");
    sync(&a, &remote).unwrap();
    let shard = own_shards(&remote, &a).remove(0);
    let text = String::from_utf8(remote.get(&shard).unwrap()).unwrap();
    let mut line: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    line["rating"] = serde_json::json!("sublime");
    let newer = format!("{line}\n");
    remote.put(&shard, newer.as_bytes()).unwrap();

    let on_a = sync(&a, &remote).unwrap();
    assert!(on_a.repaired.is_empty(), "{on_a:?}");
    let skip = on_a
        .skipped
        .iter()
        .find(|s| s.name == shard)
        .expect("reported");
    assert!(skip.own && skip.newer_format, "{skip:?}");
    assert_eq!(
        remote.get(&shard).unwrap(),
        newer.as_bytes(),
        "never written over"
    );

    let on_b = sync(&b, &remote).unwrap();
    assert!(
        on_b.notice()
            .unwrap()
            .contains("Another device uses a newer version"),
        "{:?}",
        on_b.notice()
    );
    finish(&[&dir_a, &dir_b, &shared]);
}

// ---- the clock ticks past this device's own last stamp -------------------------

#[test]
fn a_duplicates_owner_editing_it_again_before_hearing_of_the_collapse_keeps_the_edit() {
    // A collapse tombstones the losing duplicate at its own stamp and revision + 1.
    // If its owner then edited it again inside the same second, that edit had the
    // same stamp and revision, and the removal won the tie: the edit was lost
    // instead of being carried onto the survivor. A clock that strictly increases
    // puts the owner's next write after anything it already wrote.
    let (dir_a, a) = device("tie-a");
    let (dir_b, b) = device("tie-b");
    let shared = scratch("tie-store");
    let remote = FolderStore::open(&shared).unwrap();

    let add = |db: &Db, meaning: &str| {
        let mut vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
        let id = vocab
            .add_entry("学习", "xuéxí", meaning, Some("HSK1"))
            .unwrap()
            .id;
        vocab.save().unwrap();
        id
    };
    let rename = |db: &Db, id: u64, uuid: &str| {
        rusqlite::Connection::open(db.path())
            .unwrap()
            .execute(
                "UPDATE vocab_entry SET uuid = ?1 WHERE id = ?2",
                rusqlite::params![uuid, id as i64],
            )
            .unwrap();
    };
    let id_a = add(&a, "A's meaning");
    let id_b = add(&b, "B's first meaning");
    rename(&a, id_a, "00000000-0000-4000-8000-000000000001");
    rename(&b, id_b, "ffffffff-ffff-4fff-bfff-fffffffffffe");
    // "Inside the same second" made certain rather than likely: B's copy, and B's
    // clock, both stand at one stamp a little ahead of the wall clock, so B's next
    // write would land on exactly that stamp if the clock did not tick past it.
    let pinned = add_seconds(&now_iso8601(), 30).unwrap();
    let conn = rusqlite::Connection::open(b.path()).unwrap();
    conn.execute(
        "UPDATE vocab_entry SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![pinned, id_b as i64],
    )
    .unwrap();
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'sync:clock'",
        rusqlite::params![pinned],
    )
    .unwrap();
    drop(conn);

    // B publishes its copy; A collapses the pair, and its next sync publishes the
    // tombstone (a sync publishes before it pulls).
    sync(&b, &remote).unwrap();
    sync(&a, &remote).unwrap();
    sync(&a, &remote).unwrap();
    // B, not yet having heard, corrects its copy straight away.
    {
        let mut vocab = VocabStore::open_with(Box::new(b.clone())).unwrap();
        vocab
            .update_entry(id_b, "xuéxí", "B's corrected meaning", Some("HSK1"))
            .unwrap();
        vocab.save().unwrap();
    }
    for _ in 0..2 {
        sync(&b, &remote).unwrap();
        sync(&a, &remote).unwrap();
    }
    for db in [&a, &b] {
        let vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
        let meanings: Vec<_> = vocab.entries().iter().map(|e| e.meaning.clone()).collect();
        assert_eq!(
            meanings,
            ["B's corrected meaning"],
            "the later edit survives"
        );
    }
    finish(&[&dir_a, &dir_b, &shared]);
}

// ---- a tie on the stamp is settled by the content ------------------------------

fn entry(uuid: &str, meaning: &str, at: &str, dev: &str, rev: i64) -> SyncedEntry {
    SyncedEntry {
        uuid: uuid.into(),
        text: "学习".into(),
        pinyin: "xuéxí".into(),
        meaning: meaning.into(),
        group: Some("HSK1".into()),
        added_at: "2026-01-01T00:00:00Z".into(),
        updated_at: at.into(),
        device_id: dev.into(),
        revision: rev,
        deleted: false,
    }
}

#[test]
fn a_survivor_carrying_another_rows_stamp_settles_the_same_way_in_either_order() {
    // The survivor of a collapse takes the newest duplicate's stamp. A device that
    // wrote the duplicate and, in the same second, an edit of the survivor's own
    // uuid at the same revision leaves two copies of the survivor with one stamp
    // and different content. Without a tiebreak on the content, which one won
    // depended on the order they met.
    let edit_of_s = entry("a", "B's edit of S", "2026-01-01T00:00:05Z", "B", 3);
    let d = entry("b", "D content", "2026-01-01T00:00:05Z", "B", 3);
    let s_old = entry("a", "orig", "2026-01-01T00:00:01Z", "A", 0);

    let (x, _) = merge_vocab(vec![s_old], vec![], vec![d.clone()], vec![]);
    let (y, _) = merge_vocab(vec![edit_of_s], vec![], vec![d], vec![]);
    let (xy, _) = merge_vocab(x.clone(), vec![], y.clone(), vec![]);
    let (yx, _) = merge_vocab(y, vec![], x, vec![]);
    assert_eq!(xy, yx, "the same list whichever side merges the other");
}

// ---- a cursor save that does not move writes nothing ---------------------------

#[test]
fn a_cursor_save_that_does_not_move_leaves_the_peers_position_as_it_was() {
    let (dir_a, a) = device("cur-a");
    let (dir_b, b) = device("cur-b");
    let shared = scratch("cur-store");
    let remote = FolderStore::open(&shared).unwrap();
    {
        let mut cursor = CursorStore::open_with(Box::new(b.clone())).unwrap();
        cursor.set_index(7);
        cursor.save().unwrap();
    }
    sync(&b, &remote).unwrap();
    sync(&a, &remote).unwrap();
    let pulled = a.cursor_view().unwrap().unwrap();
    assert_eq!(pulled.device_id, b.device_id());

    // A visits the same place: the app saves, nothing moved.
    {
        let mut cursor = CursorStore::open_with(Box::new(a.clone())).unwrap();
        cursor.set_index(7);
        cursor.save().unwrap();
    }
    assert_eq!(
        a.cursor_view().unwrap().unwrap(),
        pulled,
        "not a new version of B's position under A's name"
    );
    finish(&[&dir_a, &dir_b, &shared]);
}

// ---- a peer whose clock is years ahead -----------------------------------------

#[test]
fn a_peer_stamping_far_in_the_future_neither_pins_the_clock_nor_wins_an_edit_made_here() {
    let (dir_a, a) = device("ff-a");
    let (dir_b, b) = device("ff-b");
    let shared = scratch("ff-store");
    let remote = FolderStore::open(&shared).unwrap();
    let view = serde_json::json!({
        "version": 1,
        "entries": [{
            "uuid": "z", "text": "猫", "pinyin": "māo", "meaning": "cat", "group": null,
            "added_at": "2099-01-01T00:00:00Z", "updated_at": "2099-01-01T00:00:00Z",
            "device_id": "broken-rtc", "revision": 0, "deleted": false
        }],
        "groups": []
    });
    remote
        .put("devices/broken-rtc/vocab.json", view.to_string().as_bytes())
        .unwrap();
    sync(&a, &remote).unwrap();
    sync(&b, &remote).unwrap();

    // A's own writes stay on A's clock...
    let cat = {
        let mut vocab = VocabStore::open_with(Box::new(a.clone())).unwrap();
        vocab.add_entry("狗", "gǒu", "dog", None).unwrap();
        vocab.save().unwrap();
        vocab.entries().iter().find(|e| e.text == "猫").unwrap().id
    };
    let (view, _) = a.vocab_view().unwrap();
    let dog = view.iter().find(|e| e.text == "狗").unwrap();
    assert!(
        dog.updated_at.as_str() < "2099",
        "not pinned to 2099: {}",
        dog.updated_at
    );

    // ...but an edit of the far-future record itself is stamped past it, so it is
    // not undone by the record it replaced.
    {
        let mut vocab = VocabStore::open_with(Box::new(a.clone())).unwrap();
        vocab
            .update_entry(cat, "māo", "cat, edited here", None)
            .unwrap();
        vocab.save().unwrap();
    }
    for _ in 0..2 {
        sync(&a, &remote).unwrap();
        sync(&b, &remote).unwrap();
    }
    for db in [&a, &b] {
        let vocab = VocabStore::open_with(Box::new(db.clone())).unwrap();
        let cat = vocab.entries().iter().find(|e| e.text == "猫").unwrap();
        assert_eq!(cat.meaning, "cat, edited here");
    }
    finish(&[&dir_a, &dir_b, &shared]);
}

// ---- the property harness -------------------------------------------------------

/// A small deterministic generator, so a failure names a seed that replays it.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// One simulated device: its view of the list, its clock floor, and how far its
/// wall clock is off.
struct Sim {
    name: String,
    view: BTreeMap<String, SyncedEntry>,
    floor: i64,
    skew: i64,
}

const BASE: i64 = 1_800_000_000;

/// The store's clock rule (see `hanzi-store`'s `clock`): later than the wall clock,
/// later than everything seen or issued — or, with `clock` off, the bare wall clock.
fn stamp_for(d: &mut Sim, now: i64, clock: bool) -> String {
    let wall = now + d.skew;
    let stamp = if clock { wall.max(d.floor + 1) } else { wall };
    d.floor = d.floor.max(stamp);
    iso8601_from_unix(stamp)
}

fn observe(d: &mut Sim) {
    for e in d.view.values() {
        if e.device_id != d.name {
            if let Some(at) = parse_iso8601(&e.updated_at) {
                d.floor = d.floor.max(at);
            }
        }
    }
}

/// What `apply_vocab_view` keeps: everything held, everything live, and the
/// tombstone of a collapsed duplicate.
fn kept(merged: &[SyncedEntry], e: &SyncedEntry, held: bool) -> bool {
    held || !e.deleted
        || merged
            .iter()
            .any(|o| !o.deleted && o.text == e.text && o.group == e.group && o.uuid != e.uuid)
}

fn sim_sync(d: &mut Sim, published: &mut BTreeMap<String, Vec<SyncedEntry>>, clock: bool) {
    published.insert(d.name.clone(), d.view.values().cloned().collect());
    let remote: Vec<SyncedEntry> = published.values().flatten().cloned().collect();
    let (merged, _) = merge_vocab(d.view.values().cloned().collect(), vec![], remote, vec![]);
    d.view = merged
        .iter()
        .filter(|e| kept(&merged, e, d.view.contains_key(&e.uuid)))
        .map(|e| (e.uuid.clone(), e.clone()))
        .collect();
    if clock {
        observe(d);
    }
}

fn live(d: &Sim) -> BTreeMap<String, SyncedEntry> {
    d.view
        .iter()
        .filter(|(_, e)| !e.deleted)
        .map(|(k, e)| (k.clone(), e.clone()))
        .collect()
}

fn run(seed: u64, devices: usize, ops: usize, clock: bool) -> Result<(), String> {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut ds: Vec<Sim> = (0..devices)
        .map(|i| Sim {
            name: format!("dev{i}"),
            view: BTreeMap::new(),
            floor: 0,
            skew: if i == 0 {
                0
            } else {
                (rng.below(3) as i64 - 1) * 5
            },
        })
        .collect();
    let mut published = BTreeMap::new();
    let mut made = 0u64;
    let mut now = BASE;
    for _ in 0..ops {
        now += rng.below(3) as i64; // often inside one second
        let di = rng.below(devices as u64) as usize;
        let d = &mut ds[di];
        let lives: Vec<String> = d
            .view
            .values()
            .filter(|e| !e.deleted)
            .map(|e| e.uuid.clone())
            .collect();
        match rng.below(10) {
            0..=2 => {
                let text = ["学习", "你好"][rng.below(2) as usize];
                let group = Some(["G", "H"][rng.below(2) as usize].to_string());
                // `add_entry` refuses a duplicate on one device.
                if d.view
                    .values()
                    .any(|e| !e.deleted && e.text == text && e.group == group)
                {
                    continue;
                }
                made += 1;
                let uuid = format!("{:04x}-{made}", rng.below(65536));
                let stamp = stamp_for(d, now, clock);
                d.view.insert(
                    uuid.clone(),
                    SyncedEntry {
                        uuid,
                        text: text.into(),
                        pinyin: "p".into(),
                        meaning: format!("m{made}"),
                        group,
                        added_at: stamp.clone(),
                        updated_at: stamp,
                        device_id: d.name.clone(),
                        revision: 0,
                        deleted: false,
                    },
                );
            }
            3..=5 if !lives.is_empty() => {
                let uuid = lives[rng.below(lives.len() as u64) as usize].clone();
                let delete = rng.below(3) == 0;
                let move_group = rng.below(3) == 0;
                let stamp = stamp_for(d, now, clock);
                let name = d.name.clone();
                let e = d.view.get_mut(&uuid).unwrap();
                if delete {
                    e.deleted = true;
                } else {
                    e.meaning = format!("edit-{name}-{now}");
                    if move_group {
                        let other = if e.group.as_deref() == Some("G") {
                            "H"
                        } else {
                            "G"
                        };
                        e.group = Some(other.to_string());
                    }
                }
                e.updated_at = stamp;
                e.device_id = name;
                e.revision += 1;
            }
            _ => sim_sync(d, &mut published, clock),
        }
    }
    for _ in 0..4 {
        for d in ds.iter_mut() {
            sim_sync(d, &mut published, clock);
        }
    }
    let first = live(&ds[0]);
    for d in &ds[1..] {
        if live(d) != first {
            return Err(format!("seed {seed}: {} differs from dev0", d.name));
        }
    }
    let mut words = BTreeSet::new();
    for e in first.values() {
        if !words.insert((e.text.clone(), e.group.clone())) {
            return Err(format!("seed {seed}: {} survives twice", e.text));
        }
    }
    let view: Vec<SyncedEntry> = ds[0].view.values().cloned().collect();
    let (again, _) = merge_vocab(view.clone(), vec![], vec![], vec![]);
    if again != view {
        return Err(format!("seed {seed}: one more merge changed the list"));
    }
    Ok(())
}

#[test]
fn random_histories_on_several_devices_converge_with_one_entry_per_word() {
    let mut failures = Vec::new();
    for seed in 1..800u64 {
        for (devices, clock) in [(3, true), (3, false), (4, true)] {
            if let Err(why) = run(seed, devices, 60, clock) {
                failures.push(why);
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures, e.g. {}",
        failures.len(),
        failures[0]
    );
}
