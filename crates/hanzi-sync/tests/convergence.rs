//! What cross-device sync has to be true of.
//!
//! These are M13's acceptance criteria, as tests. The one that matters most is
//! convergence: two devices that practised the same character while apart must end
//! up with *the same* schedule, whichever of them does the merging — because if
//! they can disagree, the learner has two schedules and no way to know which is
//! real.
//!
//! Everything here runs against a temporary directory. That is the point of the
//! split: the merge is the part that has to be right, and it is testable with no
//! account, no network and no credentials.

use std::fs;
use std::path::{Path, PathBuf};

use hanzi_core::Rating;
use hanzi_sync::{
    attempts_shard_name, device_of_shard, fold_cards, merge_attempts, parse_attempts_shard, Baselines,
    read_attempts, write_attempts, FolderStore, MergedAttempt, RemoteStore,
};

/// A fold with nothing to fold from, which is the ordinary case: a log that goes
/// back to every character's first attempt needs no baseline.
fn no_baseline() -> Baselines {
    Baselines::default()
}

/// A private directory per test, cleaned up by the caller's `finish`.
fn dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("hanzi-sync-{name}-{}", std::process::id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).unwrap();
    path
}

fn finish(dir: &Path) {
    fs::remove_dir_all(dir).ok();
}

/// One attempt, as a device would have logged it.
fn attempt(device: &str, seq: i64, ch: &str, at: &str, score: f32) -> MergedAttempt {
    MergedAttempt {
        device_id: device.to_string(),
        seq,
        ch: ch.to_string(),
        at: at.to_string(),
        score,
        rating: Rating::from_score(score),
    }
}

/// The laptop's log: three attempts at 好, one of them a failure, and one at 学.
fn laptop() -> Vec<MergedAttempt> {
    vec![
        attempt("laptop", 1, "好", "2026-09-19T09:00:00Z", 88.0),
        attempt("laptop", 2, "好", "2026-09-19T21:00:00Z", 91.0),
        attempt("laptop", 3, "好", "2026-09-21T09:00:00Z", 34.0),
        attempt("laptop", 4, "学", "2026-09-21T09:05:00Z", 77.0),
    ]
}

/// The phone's log for the same character, practised while the laptop was off.
fn phone() -> Vec<MergedAttempt> {
    vec![
        attempt("phone", 1, "好", "2026-09-20T08:00:00Z", 95.0),
        attempt("phone", 2, "好", "2026-09-20T20:00:00Z", 62.0),
        attempt("phone", 3, "好", "2026-09-22T07:00:00Z", 80.0),
        attempt("phone", 4, "学", "2026-09-22T07:30:00Z", 83.0),
    ]
}

#[test]
fn two_devices_that_practised_apart_converge_on_one_schedule() {
    // The headline claim. Each device appends its own attempts; both fold the
    // union; both must land on the same card. Nothing is merged but the log.
    let dir = dir("converge");
    let store = FolderStore::open(&dir).unwrap();

    write_attempts(&store, "laptop", &laptop()).unwrap();
    write_attempts(&store, "phone", &phone()).unwrap();

    // What each device can see: its own log in hand, the peer's shards fetched.
    let published = read_attempts(&store).unwrap();
    let peers_of_laptop: Vec<MergedAttempt> = published
        .iter()
        .filter(|a| a.device_id == "phone")
        .cloned()
        .collect();
    let peers_of_phone: Vec<MergedAttempt> = published
        .iter()
        .filter(|a| a.device_id == "laptop")
        .cloned()
        .collect();

    let on_laptop = fold_cards(&merge_attempts(laptop(), peers_of_laptop).unwrap(), &no_baseline());
    let on_phone = fold_cards(&merge_attempts(phone(), peers_of_phone).unwrap(), &no_baseline());

    assert_eq!(
        on_laptop, on_phone,
        "the same history has to mean the same schedule on both devices"
    );

    // And the merged log is genuinely both logs: eight attempts, none lost.
    let merged = merge_attempts(laptop(), phone()).unwrap();
    let card = &on_laptop["好"];
    assert_eq!(card.attempts, 6, "three on the laptop, three on the phone");
    assert_eq!(card.history.len(), 6, "all of them inside the window");
    assert_eq!(merged.len(), 8, "six of 好 and two of 学");

    finish(&dir);
}

#[test]
fn the_order_the_merge_is_given_the_two_logs_in_does_not_matter() {
    // The obvious way to get convergence wrong is to let "my log" and "their log"
    // be folded in that order. This is that claim directly.
    let forwards = merge_attempts(laptop(), phone()).unwrap();
    let backwards = merge_attempts(phone(), laptop()).unwrap();
    assert_eq!(forwards, backwards, "a union does not have a loser");
    assert_eq!(fold_cards(&forwards, &no_baseline()), fold_cards(&backwards, &no_baseline()));

    finish(&dir("order"));
}

#[test]
fn an_attempt_reads_back_exactly_as_it_was_written() {
    // Scores are `f32` and they travel as JSON text, so a value that comes back
    // one bit different would make a card folded from the log disagree with the
    // card the device that made it holds — and the disagreement would be
    // invisible.
    let dir = dir("round-trip");
    let store = FolderStore::open(&dir).unwrap();
    let original = vec![
        attempt("phone", 1, "好", "2026-09-19T09:00:00Z", 83.7),
        attempt("phone", 2, "好", "2026-09-19T09:01:00Z", 0.0),
        attempt("phone", 3, "好", "2026-09-19T09:02:00Z", 100.0),
        attempt("phone", 4, "好", "2026-09-19T09:03:00Z", 42.42),
    ];
    write_attempts(&store, "phone", &original).unwrap();

    let read = read_attempts(&store).unwrap();
    assert_eq!(read, original, "byte for byte, or the fold will differ");

    finish(&dir);
}

#[test]
fn syncing_twice_changes_nothing() {
    // A sync is retried whenever it fails, and it cannot always tell whether the
    // failure happened before or after the write. So the second run must be a
    // no-op: the same names, the same bytes, one copy of each attempt.
    let dir = dir("idempotent");
    let store = FolderStore::open(&dir).unwrap();

    let first = write_attempts(&store, "laptop", &laptop()).unwrap();
    let before = read_attempts(&store).unwrap();

    let second = write_attempts(&store, "laptop", &laptop()).unwrap();
    let after = read_attempts(&store).unwrap();

    assert_eq!(first, second, "the same attempts produce the same shard names");
    assert_eq!(before, after, "and reading them back produces the same log");
    assert_eq!(after.len(), laptop().len(), "nothing was duplicated");

    // And merging what is already merged is still the same log.
    let merged = merge_attempts(laptop(), after.clone()).unwrap();
    assert_eq!(merged, after);

    finish(&dir);
}

#[test]
fn the_same_sequence_number_on_two_devices_is_two_attempts() {
    // Why the pair is the identity and the number is not. Both devices' logs start
    // at 1, and both attempts have to survive.
    let dir = dir("per-device-seq");
    let store = FolderStore::open(&dir).unwrap();

    let one = vec![attempt("laptop", 1, "好", "2026-09-19T09:00:00Z", 88.0)];
    let two = vec![attempt("phone", 1, "好", "2026-09-19T10:00:00Z", 91.0)];
    write_attempts(&store, "laptop", &one).unwrap();
    write_attempts(&store, "phone", &two).unwrap();

    let merged = merge_attempts(one, two).unwrap();
    assert_eq!(merged.len(), 2, "sequence 1 twice is still two attempts");
    assert_eq!(fold_cards(&merged, &no_baseline())["好"].attempts, 2);

    finish(&dir);
}

#[test]
fn two_shards_disagreeing_about_one_attempt_is_reported_not_resolved() {
    // `(device_id, seq)` is written once and never rewritten. If two copies carry
    // different scores then a shard was replaced, and a merge that quietly picked
    // one would be inventing history for a learner who cannot check it.
    let honest = vec![attempt("phone", 1, "好", "2026-09-19T09:00:00Z", 91.0)];
    let rewritten = vec![attempt("phone", 1, "好", "2026-09-19T09:00:00Z", 12.0)];

    let error = merge_attempts(honest, rewritten).unwrap_err();
    assert!(
        matches!(
            error,
            hanzi_sync::SyncError::Rewritten { ref device_id, seq } if device_id == "phone" && seq == 1
        ),
        "got {error:?}"
    );

    finish(&dir("rewritten"));
}

#[test]
fn a_store_holding_other_files_is_not_mistaken_for_a_log() {
    // A synced folder is somebody's folder. It will contain other things.
    let dir = dir("other-files");
    let store = FolderStore::open(&dir).unwrap();
    write_attempts(&store, "laptop", &laptop()).unwrap();
    store.put("README.txt", b"not a shard").unwrap();
    store
        .put("devices/laptop/notes.txt", b"also not a shard")
        .unwrap();
    // A half-written shard left behind by an interrupted write must be ignored
    // rather than parsed as a short log.
    store
        .put("devices/laptop/attempts/000000000001.jsonl.part", b"{\"seq\":")
        .unwrap();

    let read = read_attempts(&store).unwrap();
    assert_eq!(read, laptop(), "only the real shard was read");

    finish(&dir);
}

#[test]
fn a_truncated_shard_is_an_error_rather_than_a_shorter_log() {
    // The failure that would be silent: a shard missing its last line parses
    // perfectly well as a log with one fewer attempt in it.
    let dir = dir("truncated");
    let store = FolderStore::open(&dir).unwrap();
    let name = attempts_shard_name("laptop", 1);
    store
        .put(
            &name,
            b"{\"seq\":1,\"ch\":\"\xe5\xa5\xbd\",\"at\":\"2026-09-19T09:00:00Z\",\"score\":88.0,\"rating\":\"good\"}\n{\"seq\":2,\"ch\":\"\xe5\xa5\xbd\",\"at\"",
        )
        .unwrap();

    let error = read_attempts(&store).unwrap_err();
    assert!(
        matches!(error, hanzi_sync::SyncError::Malformed(ref why) if why.contains("line 2")),
        "got {error:?}"
    );

    finish(&dir);
}

#[test]
fn a_shard_name_cannot_reach_outside_the_store() {
    // A shard name comes from a remote store, so it is not this program's text.
    // `../../something` is a fine filename to a filesystem and a catastrophic one
    // to obey.
    let dir = dir("escape");
    let store = FolderStore::open(dir.join("store")).unwrap();

    for name in [
        "../escaped.jsonl",
        "devices/../../escaped.jsonl",
        "/etc/passwd",
        "",
        "devices\\laptop\\attempts\\1.jsonl",
    ] {
        let written = store.put(name, b"nope");
        assert!(written.is_err(), "{name:?} should not be writable");
        assert!(store.get(name).is_err(), "{name:?} should not be readable");
    }
    assert!(
        !dir.join("escaped.jsonl").exists(),
        "nothing was written outside the store"
    );

    finish(&dir);
}

#[test]
fn a_shard_name_says_which_device_it_belongs_to() {
    let name = attempts_shard_name("a1b2c3", 501);
    assert_eq!(name, "devices/a1b2c3/attempts/000000000501.jsonl");
    assert_eq!(device_of_shard(&name), Some("a1b2c3"));
    assert_eq!(parse_attempts_shard(&name), Some(("a1b2c3", 501)));

    // Everything that is not one of ours.
    assert_eq!(parse_attempts_shard("devices/a1b2c3/vocab.json"), None);
    assert_eq!(parse_attempts_shard("devices/a1b2c3/attempts/x.jsonl"), None);
    assert_eq!(parse_attempts_shard("notes.txt"), None);
    assert_eq!(device_of_shard("notes.txt"), None);
}

#[test]
fn a_chunk_of_the_log_is_one_shard_and_a_longer_log_is_more_than_one() {
    // Sharding is what keeps a month offline from meaning one huge upload, and it
    // is also why a shard's name is the sequence it starts at.
    let dir = dir("chunks");
    let store = FolderStore::open(&dir).unwrap();

    let many: Vec<MergedAttempt> = (1..=1200)
        .map(|i| {
            let at = format!("2026-09-19T09:{:02}:{:02}Z", (i / 60) % 60, i % 60);
            attempt("laptop", i, "好", &at, 70.0)
        })
        .collect();
    let names = write_attempts(&store, "laptop", &many).unwrap();

    assert_eq!(names.len(), 3, "1200 attempts at 500 to a shard");
    assert_eq!(names[0], attempts_shard_name("laptop", 1));
    assert_eq!(names[1], attempts_shard_name("laptop", 501));
    assert_eq!(names[2], attempts_shard_name("laptop", 1001));

    let read = read_attempts(&store).unwrap();
    assert_eq!(read.len(), 1200, "and every attempt came back");

    finish(&dir);
}

#[test]
fn folding_an_empty_log_produces_no_cards() {
    // A store with nothing in it is a device that has never synced, not an error.
    let cards = fold_cards(&[], &no_baseline());
    assert!(cards.is_empty());
    assert!(merge_attempts(vec![], vec![]).unwrap().is_empty());

    finish(&dir("empty"));
}
