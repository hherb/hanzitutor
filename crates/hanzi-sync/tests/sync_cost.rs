//! Issue #36: what a sync costs, and that it stops growing with every sync.
//!
//! Three costs used to grow without bound. Each publish started a new shard, so a
//! phone that syncs at every launch built up hundreds of one-line files a year;
//! every sync downloaded every one of them, from every device, because the
//! revision a listing reports was never read; and a busy Dropbox (a 429) failed
//! the whole pass. Each test here pins one half of the fix, against a store that
//! counts what it is asked to do.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use hanzi_core::ProgressStore;
use hanzi_core::VocabStore;
use hanzi_store::Db;
use hanzi_sync::{
    full_log, pull, read_attempts, sync, DropboxStore, FolderStore, Http, RemoteEntry, RemoteStore,
    SyncError, CHUNK, RETRY_BUDGET, RETRY_TRIES,
};
use serde_json::json;

/// A scratch directory, removed when it goes out of scope — so a test that fails
/// part-way leaves nothing behind either.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

impl std::ops::Deref for Scratch {
    type Target = PathBuf;
    fn deref(&self) -> &PathBuf {
        &self.0
    }
}

fn scratch(name: &str) -> Scratch {
    let path = std::env::temp_dir().join(format!("hanzi-sync-cost-{name}-{}", std::process::id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).unwrap();
    Scratch(path)
}

/// A device, with its directory: the database is declared after the directory,
/// so it is closed before the directory is removed.
fn device(name: &str) -> (Scratch, Db) {
    let path = scratch(name);
    let db = Db::open(&*path).unwrap();
    VocabStore::open_with(Box::new(db.clone())).unwrap();
    (path, db)
}

/// `count` attempts, a minute apart from `start`, saved together.
fn practise(db: &Db, start: i64, count: i64) {
    let mut progress = ProgressStore::open_with(Box::new(db.clone())).unwrap();
    for n in start..start + count {
        let at = hanzi_core::time::iso8601_from_unix(1_790_000_000 + 60 * n);
        let ch = ['好', '你', '学', '习'][(n % 4) as usize];
        progress.record_at(ch, 80.0, &at).unwrap();
    }
    progress.save().unwrap();
}

fn own_shards(remote: &dyn RemoteStore, db: &Db) -> Vec<String> {
    let prefix = format!("devices/{}/attempts/", db.device_id());
    remote
        .list()
        .unwrap()
        .into_iter()
        .map(|entry| entry.name)
        .filter(|name| name.starts_with(&prefix))
        .collect()
}

fn lines(remote: &dyn RemoteStore, name: &str) -> usize {
    String::from_utf8(remote.get(name).unwrap())
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn attempts_of(db: &Db, of: &Db) -> usize {
    full_log(db)
        .unwrap()
        .into_iter()
        .filter(|attempt| attempt.device_id == of.device_id())
        .count()
}

// ---- a store that counts ------------------------------------------------------

/// Another writer's move, made just before an append goes through.
type Meddle = Box<dyn FnOnce(&FolderStore, &str)>;

/// A directory that counts what it is asked, and can act as a second writer.
struct Counting {
    inner: FolderStore,
    lists: Cell<usize>,
    gets: RefCell<Vec<String>>,
    puts: RefCell<Vec<String>>,
    replaces: RefCell<Vec<String>>,
    /// Run once, just before the next `replace` goes through: another writer
    /// getting in between this sync's listing and its append.
    before_replace: RefCell<Option<Meddle>>,
}

impl Counting {
    fn open(root: &Path) -> Self {
        Self {
            inner: FolderStore::open(root).unwrap(),
            lists: Cell::new(0),
            gets: RefCell::new(Vec::new()),
            puts: RefCell::new(Vec::new()),
            replaces: RefCell::new(Vec::new()),
            before_replace: RefCell::new(None),
        }
    }

    fn reset(&self) {
        self.lists.set(0);
        self.gets.borrow_mut().clear();
        self.puts.borrow_mut().clear();
        self.replaces.borrow_mut().clear();
    }

    /// The attempt shards downloaded since the last reset.
    fn shard_gets(&self) -> Vec<String> {
        self.gets
            .borrow()
            .iter()
            .filter(|name| name.contains("/attempts/"))
            .cloned()
            .collect()
    }
}

impl RemoteStore for Counting {
    fn list(&self) -> Result<Vec<RemoteEntry>, SyncError> {
        self.lists.set(self.lists.get() + 1);
        self.inner.list()
    }

    fn get(&self, name: &str) -> Result<Vec<u8>, SyncError> {
        self.gets.borrow_mut().push(name.to_string());
        self.inner.get(name)
    }

    fn put(&self, name: &str, bytes: &[u8]) -> Result<(), SyncError> {
        self.puts.borrow_mut().push(name.to_string());
        self.inner.put(name, bytes)
    }

    fn replace(&self, name: &str, bytes: &[u8], revision: &str) -> Result<(), SyncError> {
        self.replaces.borrow_mut().push(name.to_string());
        if let Some(meddle) = self.before_replace.borrow_mut().take() {
            meddle(&self.inner, name);
        }
        self.inner.replace(name, bytes, revision)
    }
}

// ---- one open shard, not one shard per sync ----------------------------------

#[test]
fn small_syncs_fill_one_shard_rather_than_making_one_each() {
    let (_dir_a, a) = device("fill-a");
    let (_dir_b, b) = device("fill-b");
    let shared = scratch("fill-store");
    let remote = Counting::open(&shared);

    let syncs = 12;
    for n in 0..syncs {
        practise(&a, n, 1);
        let on_a = sync(&a, &remote).unwrap();
        assert_eq!(on_a.published, 1, "sync {n}: {on_a:?}");
        // The peer folds the growing shard as it goes, so every version is read.
        let on_b = sync(&b, &remote).unwrap();
        assert_eq!(
            on_b.pulled, 1,
            "sync {n}: the peer gains exactly the new one"
        );
    }

    let shards = own_shards(&remote, &a);
    assert_eq!(
        shards.len(),
        (syncs as usize).div_ceil(CHUNK),
        "at most ceil(N / CHUNK) shards: {shards:?}"
    );
    assert_eq!(lines(&remote, &shards[0]), syncs as usize);
    assert_eq!(attempts_of(&b, &a), syncs as usize);
    assert!(
        remote.replaces.borrow().len() >= syncs as usize - 1,
        "every append after the first is a checked replace"
    );
}

#[test]
fn the_open_shard_is_closed_at_chunk_and_the_next_one_started() {
    let (_dir_a, a) = device("chunk-a");
    let (_dir_b, b) = device("chunk-b");
    let shared = scratch("chunk-store");
    let remote = Counting::open(&shared);

    let first = CHUNK as i64 - 2;
    practise(&a, 0, first);
    sync(&a, &remote).unwrap();
    for n in 0..3 {
        practise(&a, first + n, 1);
        sync(&a, &remote).unwrap();
    }

    let shards = own_shards(&remote, &a);
    assert_eq!(
        shards.len(),
        (CHUNK + 1).div_ceil(CHUNK),
        "CHUNK + 1 attempts: {shards:?}"
    );
    assert_eq!(lines(&remote, &shards[0]), CHUNK, "the first is full");
    assert_eq!(
        lines(&remote, &shards[1]),
        1,
        "the next holds the one after"
    );

    sync(&b, &remote).unwrap();
    assert_eq!(attempts_of(&b, &a), CHUNK + 1);
}

#[test]
fn an_older_builds_one_line_shards_are_left_and_the_last_of_them_grows() {
    // A device that synced with an earlier build has one shard per publish. The
    // new build leaves every one of them as it is and appends to the last — and a
    // peer reading the way an earlier build does (`pull`, every shard, every
    // time) takes the longer file as more attempts, with nothing rewritten.
    let (_dir_a, a) = device("legacy-a");
    let (_dir_b, b) = device("legacy-b");
    let shared = scratch("legacy-store");
    let remote = FolderStore::open(shared.as_path()).unwrap();

    // What an earlier build left: three syncs, three one-line shards.
    practise(&a, 0, 3);
    sync(&a, &remote).unwrap();
    let whole = own_shards(&remote, &a).remove(0);
    let text = String::from_utf8(remote.get(&whole).unwrap()).unwrap();
    fs::remove_file(shared.join(&whole)).unwrap();
    for line in text.lines() {
        let seq = serde_json::from_str::<serde_json::Value>(line).unwrap()["seq"]
            .as_i64()
            .unwrap();
        remote
            .put(
                &hanzi_sync::attempts_shard_name(&a.device_id(), seq),
                format!("{line}\n").as_bytes(),
            )
            .unwrap();
    }
    let legacy = own_shards(&remote, &a);
    assert_eq!(legacy.len(), 3);
    let before: Vec<Vec<u8>> = legacy.iter().map(|n| remote.get(n).unwrap()).collect();

    // The peer, reading as an earlier build does.
    assert_eq!(pull(&b, &remote).unwrap(), 3);

    practise(&a, 3, 2);
    assert_eq!(sync(&a, &remote).unwrap().published, 2);
    assert_eq!(own_shards(&remote, &a), legacy, "no new file");
    assert_eq!(remote.get(&legacy[0]).unwrap(), before[0], "untouched");
    assert_eq!(remote.get(&legacy[1]).unwrap(), before[1], "untouched");
    let grown = remote.get(&legacy[2]).unwrap();
    assert!(
        grown.starts_with(&before[2]),
        "the last keeps its bytes and grows"
    );
    assert_eq!(lines(&remote, &legacy[2]), 3);

    assert_eq!(
        pull(&b, &remote).unwrap(),
        2,
        "an earlier build's reader adds only the new ones"
    );
    assert_eq!(attempts_of(&b, &a), 5);
}

// ---- the store is listed once and an unchanged shard not fetched again -------

#[test]
fn a_sync_lists_the_store_once() {
    let (_dir_a, a) = device("list-a");
    let (_dir_b, b) = device("list-b");
    let shared = scratch("list-store");
    let remote = Counting::open(&shared);

    let check = |db: &Db, what: &str| {
        remote.reset();
        sync(db, &remote).unwrap();
        assert_eq!(remote.lists.get(), 1, "{what}");
    };
    check(&a, "an empty store");
    practise(&a, 0, 2);
    check(&a, "publishing");
    check(&b, "pulling a peer");
    practise(&b, 10, 1);
    check(&b, "both");
    check(&a, "nothing new");

    // And when a file has to be left out, or mended.
    let shard = own_shards(&remote, &a).remove(0);
    remote.inner.put(&shard, b"{truncated").unwrap();
    check(&b, "a peer's file damaged");
    check(&a, "this device's own file mended");
}

#[test]
fn an_unchanged_shard_is_not_downloaded_again() {
    let (_dir_a, a) = device("cache-a");
    let (_dir_b, b) = device("cache-b");
    let shared = scratch("cache-store");
    let remote = Counting::open(&shared);

    practise(&a, 0, 3);
    sync(&a, &remote).unwrap();
    let shard = own_shards(&remote, &a).remove(0);

    remote.reset();
    assert_eq!(sync(&b, &remote).unwrap().pulled, 3);
    assert_eq!(
        remote.shard_gets(),
        std::slice::from_ref(&shard),
        "read once"
    );

    remote.reset();
    let again = sync(&b, &remote).unwrap();
    assert_eq!(again.pulled, 0);
    assert!(
        remote.shard_gets().is_empty(),
        "unchanged, so not downloaded: {:?}",
        remote.shard_gets()
    );

    // It grows: the new revision is fetched, once, and only what is new is added.
    practise(&a, 3, 1);
    sync(&a, &remote).unwrap();
    remote.reset();
    assert_eq!(sync(&b, &remote).unwrap().pulled, 1);
    assert_eq!(remote.shard_gets(), std::slice::from_ref(&shard));
    remote.reset();
    sync(&b, &remote).unwrap();
    assert!(remote.shard_gets().is_empty(), "{:?}", remote.shard_gets());
    assert_eq!(attempts_of(&b, &a), 4);
}

#[test]
fn a_peer_whose_log_lost_what_it_had_folded_downloads_the_shard_again() {
    // The record of folded shards is checked against the log, not trusted: a
    // database that no longer holds a shard's attempts (restored from an older
    // backup, say) fetches it rather than taking the record's word.
    let (_dir_a, a) = device("lost-a");
    let (_dir_b, b) = device("lost-b");
    let shared = scratch("lost-store");
    let remote = Counting::open(&shared);

    practise(&a, 0, 3);
    sync(&a, &remote).unwrap();
    sync(&b, &remote).unwrap();
    rusqlite::Connection::open(b.path())
        .unwrap()
        .execute(
            "DELETE FROM attempt WHERE device_id = ?1",
            [a.device_id().to_string()],
        )
        .unwrap();

    remote.reset();
    assert_eq!(sync(&b, &remote).unwrap().pulled, 3, "pulled again");
    assert_eq!(remote.shard_gets().len(), 1);
}

#[test]
fn a_damaged_shard_is_fetched_every_sync_and_comes_back_once_mended() {
    let (_dir_a, a) = device("mended-a");
    let (_dir_b, b) = device("mended-b");
    let shared = scratch("mended-store");
    let remote = Counting::open(&shared);

    practise(&a, 0, 2);
    sync(&a, &remote).unwrap();
    let shard = own_shards(&remote, &a).remove(0);
    remote.inner.put(&shard, b"{truncated").unwrap();

    for _ in 0..2 {
        remote.reset();
        let on_b = sync(&b, &remote).unwrap();
        assert_eq!(on_b.skipped.len(), 1, "{on_b:?}");
        assert_eq!(
            remote.shard_gets(),
            std::slice::from_ref(&shard),
            "looked at again"
        );
    }
    sync(&a, &remote).unwrap();
    let on_b = sync(&b, &remote).unwrap();
    assert!(on_b.skipped.is_empty(), "{on_b:?}");
    assert_eq!(on_b.pulled, 2);
}

// ---- growth is the only change a shard may undergo ---------------------------

#[test]
fn a_shard_whose_lines_changed_is_rewritten_even_after_its_old_version_was_folded() {
    let (_dir_a, a) = device("edited-a");
    let (_dir_b, b) = device("edited-b");
    let shared = scratch("edited-store");
    let remote = Counting::open(&shared);

    practise(&a, 0, 2);
    sync(&a, &remote).unwrap();
    sync(&b, &remote).unwrap();
    sync(&b, &remote).unwrap(); // recorded as folded

    // The same length, the same attempts, one score changed.
    let shard = own_shards(&remote, &a).remove(0);
    let text = String::from_utf8(remote.get(&shard).unwrap()).unwrap();
    let edited = text.replacen("\"score\":80.0", "\"score\":90.0", 1);
    assert_ne!(edited, text);
    remote.inner.put(&shard, edited.as_bytes()).unwrap();

    let error = sync(&b, &remote).unwrap_err();
    assert!(
        matches!(&error, SyncError::Rewritten { device_id, seq: 1 } if *device_id == a.device_id()),
        "{error:?}"
    );
    // And a version that grew *and* changed a line is no better.
    let grown = format!(
        "{edited}{}",
        text.lines()
            .last()
            .unwrap()
            .replace("\"seq\":2", "\"seq\":3")
    );
    remote
        .inner
        .put(&shard, format!("{grown}\n").as_bytes())
        .unwrap();
    assert!(matches!(
        sync(&b, &remote).unwrap_err(),
        SyncError::Rewritten { .. }
    ));
}

#[test]
fn a_second_writer_in_this_directory_makes_the_append_fail_rather_than_be_lost() {
    // Two installs sharing one identity (#12) are the one way two writers meet in
    // one directory. The append is checked against the revision it was listed
    // with, so the second of them fails rather than drops what the first wrote.
    let (_dir_a, a) = device("race-a");
    let shared = scratch("race-store");
    let remote = Counting::open(&shared);

    practise(&a, 0, 1);
    sync(&a, &remote).unwrap();
    let shard = own_shards(&remote, &a).remove(0);
    let theirs = format!(
        "{}{{\"seq\":2,\"ch\":\"你\",\"at\":\"2026-09-20T09:00:00Z\",\"score\":70.0,\"rating\":\"good\"}}\n",
        String::from_utf8(remote.get(&shard).unwrap()).unwrap()
    );
    let written = theirs.clone();
    *remote.before_replace.borrow_mut() = Some(Box::new(move |store, name| {
        store.put(name, written.as_bytes()).unwrap();
    }));

    practise(&a, 1, 1);
    let error = sync(&a, &remote).unwrap_err();
    assert!(matches!(error, SyncError::Io(_)), "{error:?}");
    assert_eq!(
        remote.inner.get(&shard).unwrap(),
        theirs.as_bytes(),
        "the other writer's append is still there"
    );
}

#[test]
fn a_sync_retried_after_its_watermark_was_lost_writes_nothing_twice() {
    // A publish whose shards were written and whose watermark was not — the
    // database write failed, or the app was killed in between — is retried by the
    // next sync. Every attempt it wrote is already in a readable shard of this
    // device's, so the retry writes nothing: not into the full shard, not into the
    // one started after it, and not as a third.
    let (_dir_a, a) = device("retry-a");
    let shared = scratch("retry-store");
    let remote = Counting::open(&shared);

    let first = CHUNK as i64 - 2;
    practise(&a, 0, first);
    sync(&a, &remote).unwrap();
    let before = a.meta_value("sync:published_seq").unwrap().unwrap();
    practise(&a, first, 5);
    assert_eq!(sync(&a, &remote).unwrap().published, 5);
    let shards = own_shards(&remote, &a);
    assert_eq!(shards.len(), 2, "the open shard filled and the next began");
    let bytes: Vec<Vec<u8>> = shards.iter().map(|n| remote.get(n).unwrap()).collect();

    // As though the watermark write had never happened.
    a.set_meta_value("sync:published_seq", &before).unwrap();
    remote.reset();
    let retried = sync(&a, &remote).unwrap();
    assert_eq!(retried.published, 0, "{retried:?}");
    assert_eq!(own_shards(&remote, &a), shards, "no new shard");
    for (name, was) in shards.iter().zip(&bytes) {
        assert_eq!(&remote.get(name).unwrap(), was, "{name} unchanged");
    }
    assert_eq!(
        read_attempts(&remote).unwrap().len(),
        CHUNK + 3,
        "every attempt once"
    );
}

#[test]
fn a_copy_of_this_database_that_is_behind_the_store_takes_a_new_name_and_appends_to_nothing_of_the_original() {
    // Two installs with one identity, outside the moment they race: the copy's
    // last shard holds attempts under its name that its log does not. Appending to
    // it, or starting a new shard, would publish different attempts under the same
    // numbers, and every device would fail with `Rewritten` for good. Before #47
    // the copy refused (`SharedIdentity`); with #47's `settle_identity` it takes a
    // new name first, so the open shard it then appends to is its own, and the
    // original's files are left exactly as they were.
    let (_dir_a, a) = device("copy-a");
    let shared = scratch("copy-store");
    let remote = Counting::open(&shared);
    practise(&a, 0, 1);
    sync(&a, &remote).unwrap();

    let dir_b = scratch("copy-b");
    rusqlite::Connection::open(a.path())
        .unwrap()
        .execute(
            "VACUUM INTO ?1",
            [dir_b.join(Db::FILE_NAME).to_string_lossy().to_string()],
        )
        .unwrap();
    let b = Db::open(&*dir_b).unwrap();
    assert_eq!(b.device_id(), a.device_id());

    practise(&a, 1, 1);
    sync(&a, &remote).unwrap();
    let shards = own_shards(&remote, &a);
    let bytes = remote.get(&shards[0]).unwrap();

    practise(&b, 5, 1);
    let summary = sync(&b, &remote).expect("the copy renames rather than refuses");
    assert_ne!(b.device_id(), a.device_id(), "the copy took a new name");
    assert_eq!(summary.published, 1, "{summary:?}");
    assert_eq!(own_shards(&remote, &a), shards, "nothing new under the old name");
    assert_eq!(remote.get(&shards[0]).unwrap(), bytes, "nothing changed");
    assert_eq!(own_shards(&remote, &b).len(), 1, "its own attempt, under its own name");
    sync(&a, &remote).unwrap();
    assert_eq!(read_attempts(&remote).unwrap().len(), 3, "every attempt once");
    drop(b);
}

/// A copy of `db`, as a restored backup or a copied data directory is: the same
/// device id, and the log as it stood.
fn copy_of(db: &Db, name: &str) -> (Scratch, Db) {
    let dir = scratch(name);
    rusqlite::Connection::open(db.path())
        .unwrap()
        .execute(
            "VACUUM INTO ?1",
            [dir.join(Db::FILE_NAME).to_string_lossy().to_string()],
        )
        .unwrap();
    let copy = Db::open(&*dir).unwrap();
    assert_eq!(copy.device_id(), db.device_id());
    (dir, copy)
}

/// Cut a file short, the way a sync client does: the first `keep` bytes stay.
fn truncate(remote: &dyn RemoteStore, name: &str, keep: usize) {
    let bytes = remote.get(name).unwrap();
    remote.put(name, &bytes[..keep]).unwrap();
}

/// The byte length of the first `lines` lines of a file.
fn prefix_of_lines(remote: &dyn RemoteStore, name: &str, lines: usize) -> usize {
    let bytes = remote.get(name).unwrap();
    bytes
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\n')
        .nth(lines - 1)
        .map(|(at, _)| at + 1)
        .unwrap()
}

#[test]
fn a_copy_that_renames_does_not_republish_history_hidden_in_a_damaged_shard() {
    // A's first shard is closed and its second open; B is a copy of A; A goes on
    // and appends to the open shard, which is what tells B it is a copy. But A's
    // first shard is damaged, so B cannot see it: the rename must not take the 500
    // attempts B shares with A, which are in that file, for B's own unpublished
    // ones and publish them again under its new name.
    let (_dir_a, a) = device("hidden-a");
    let shared = scratch("hidden-store");
    let remote = FolderStore::open(&*shared).unwrap();
    practise(&a, 0, CHUNK as i64);
    sync(&a, &remote).unwrap();
    practise(&a, CHUNK as i64, 100);
    sync(&a, &remote).unwrap();
    assert_eq!(own_shards(&remote, &a).len(), 2);

    let (_dir_b, b) = copy_of(&a, "hidden-b");
    practise(&a, 600, 1);
    sync(&a, &remote).unwrap();
    let first = hanzi_sync::attempts_shard_name(&a.device_id(), 1);
    truncate(&remote, &first, 1000);

    practise(&b, 700, 1);
    let on_b = sync(&b, &remote).unwrap();
    assert_ne!(b.device_id(), a.device_id(), "B took a new name");
    assert_eq!(on_b.published, 1, "only B's own new attempt: {on_b:?}");

    // Once A mends its file, every device holds every attempt exactly once.
    sync(&a, &remote).unwrap();
    let (_dir_c, c) = device("hidden-c");
    sync(&c, &remote).unwrap();
    assert_eq!(read_attempts(&remote).unwrap().len(), 602);
    assert_eq!(full_log(&c).unwrap().len(), 602);
}

#[test]
fn a_copy_does_not_mend_the_originals_grown_tail_over_with_its_own() {
    // A's open shard holds 1..5; B is a copy taken then; A appends 6 and 7, and C
    // pulls them. The shard is then cut short in the middle of 6, which hides the
    // evidence that would tell B it is a copy. B must neither write the shard
    // again from its own 1..5 nor publish its own, different, 6 under A's name:
    // C already holds A's 6, and would refuse every sync after.
    let (_dir_a, a) = device("tail-a");
    let shared = scratch("tail-store");
    let remote = FolderStore::open(&*shared).unwrap();
    practise(&a, 0, 5);
    sync(&a, &remote).unwrap();
    let (_dir_b, b) = copy_of(&a, "tail-b");
    practise(&a, 5, 2);
    sync(&a, &remote).unwrap();
    let (_dir_c, c) = device("tail-c");
    assert_eq!(sync(&c, &remote).unwrap().pulled, 7);

    let shard = hanzi_sync::attempts_shard_name(&a.device_id(), 1);
    truncate(&remote, &shard, prefix_of_lines(&remote, &shard, 5) + 12);
    let cut = remote.get(&shard).unwrap();

    practise(&b, 50, 1);
    sync(&b, &remote).unwrap();
    assert_ne!(b.device_id(), a.device_id(), "the file says more than B wrote");
    assert_eq!(remote.get(&shard).unwrap(), cut, "and B left it alone");
    sync(&c, &remote).expect("C is not handed a second attempt 6 under A's name");

    // A, the file's writer, mends it; then B's attempt goes out under its own name
    // and everybody has everything once.
    sync(&a, &remote).unwrap();
    sync(&b, &remote).unwrap();
    sync(&c, &remote).unwrap();
    assert_eq!(read_attempts(&remote).unwrap().len(), 8);
    assert_eq!(full_log(&c).unwrap().len(), 8);
}

#[test]
fn a_shard_holding_an_attempt_numbered_below_its_name_is_mended_not_taken_for_a_copy() {
    // A clock that stepped back: attempt 1 is recorded at a later time than
    // attempt 2, so the shard is named after 2 and holds both. Cut short, it still
    // holds no more than this log wrote into it, and is this device's to mend.
    let (_dir_a, a) = device("below-a");
    let shared = scratch("below-store");
    let remote = FolderStore::open(&*shared).unwrap();
    {
        let mut progress = ProgressStore::open_with(Box::new(a.clone())).unwrap();
        progress.record_at('好', 80.0, "2026-09-20T10:00:00Z").unwrap();
        progress.record_at('你', 80.0, "2026-09-20T09:00:00Z").unwrap();
        progress.save().unwrap();
    }
    sync(&a, &remote).unwrap();
    let shard = hanzi_sync::attempts_shard_name(&a.device_id(), 2);
    assert_eq!(lines(&remote, &shard), 2, "{:?}", own_shards(&remote, &a));
    let id = a.device_id();
    truncate(&remote, &shard, prefix_of_lines(&remote, &shard, 1) + 10);

    let summary = sync(&a, &remote).unwrap();
    assert_eq!(a.device_id(), id, "not renamed: {summary:?}");
    assert_eq!(summary.repaired.len(), 1, "{summary:?}");
    assert!(summary.skipped.is_empty(), "{summary:?}");
    assert_eq!(read_attempts(&remote).unwrap().len(), 2);
}

#[test]
fn two_damaged_shards_are_mended_each_with_its_own_and_mended_again_later() {
    // Mending the first of two damaged shards makes its attempts readable again,
    // so the second must not take them in as well: a second file holding 500
    // attempts below its name would, once cut short again, look like more than
    // this log ever wrote into it.
    let (_dir_a, a) = device("twice-a");
    let shared = scratch("twice-store");
    let remote = FolderStore::open(&*shared).unwrap();
    practise(&a, 0, CHUNK as i64);
    sync(&a, &remote).unwrap();
    practise(&a, CHUNK as i64, 19);
    sync(&a, &remote).unwrap();
    let first = hanzi_sync::attempts_shard_name(&a.device_id(), 1);
    let open = hanzi_sync::attempts_shard_name(&a.device_id(), CHUNK as i64 + 1);
    truncate(&remote, &first, 1000);
    truncate(&remote, &open, 300);

    let id = a.device_id();
    let summary = sync(&a, &remote).unwrap();
    assert_eq!(summary.repaired.len(), 2, "{summary:?}");
    assert_eq!(lines(&remote, &open), 19, "only its own");

    truncate(&remote, &open, 300);
    let summary = sync(&a, &remote).unwrap();
    assert_eq!(a.device_id(), id, "not renamed: {summary:?}");
    assert_eq!(summary.repaired.len(), 1, "{summary:?}");
    assert_eq!(read_attempts(&remote).unwrap().len(), CHUNK + 19);
}

#[test]
fn a_copys_attempt_held_back_by_a_damaged_shard_goes_out_under_its_new_name() {
    // B renames because A's open shard, cut short, holds more than B wrote. B's
    // own attempts that would have gone in that file stay under the old name
    // while it is unreadable. Once A mends it, every one of them moves to B's new
    // name: those whose numbers A used, and the one A never reached — which,
    // published under A's name, would make A take a new name in turn.
    let (_dir_a, a) = device("held-a");
    let shared = scratch("held-store");
    let remote = FolderStore::open(&*shared).unwrap();
    practise(&a, 0, 5);
    sync(&a, &remote).unwrap();
    let (_dir_b, b) = copy_of(&a, "held-b");
    practise(&a, 5, 2);
    sync(&a, &remote).unwrap();
    let id = a.device_id();
    let shard = hanzi_sync::attempts_shard_name(&id, 1);
    truncate(&remote, &shard, prefix_of_lines(&remote, &shard, 5) + 12);

    practise(&b, 50, 3);
    sync(&b, &remote).unwrap();
    assert_ne!(b.device_id(), id, "B renamed");

    sync(&a, &remote).unwrap();
    sync(&b, &remote).unwrap();
    sync(&a, &remote).unwrap();
    assert_eq!(a.device_id(), id, "A keeps its name");
    assert_eq!(attempts_of(&b, &b), 3, "all three of B's are B's");
    let (_dir_c, c) = device("held-c");
    sync(&c, &remote).unwrap();
    assert_eq!(full_log(&c).unwrap().len(), 10);
    assert_eq!(attempts_of(&c, &a), 7);
}

// ---- a busy Dropbox is waited for, within a bound ----------------------------

/// A Dropbox with revisions, `update` mode, and failures on demand.
#[derive(Default)]
struct Dropbox {
    files: Mutex<BTreeMap<String, (Vec<u8>, u64)>>,
    next_rev: Mutex<u64>,
    /// Answers to give, in order, before any real one.
    failures: Mutex<Vec<SyncError>>,
    /// Fail every request with this, for ever.
    always: Option<SyncError>,
    calls: Mutex<usize>,
    uploads: Mutex<Vec<serde_json::Value>>,
}

impl Dropbox {
    fn answer(&self) -> Result<(), SyncError> {
        *self.calls.lock().unwrap() += 1;
        if let Some(error) = &self.always {
            return Err(error.clone());
        }
        let mut failures = self.failures.lock().unwrap();
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.remove(0))
        }
    }

    fn calls(&self) -> usize {
        *self.calls.lock().unwrap()
    }
}

fn busy(retry_after: Option<u64>) -> SyncError {
    SyncError::Unavailable {
        status: 429,
        retry_after,
        reason: "too_many_requests".into(),
    }
}

impl Http for Dropbox {
    fn rpc(&self, url: &str, _bearer: &str, _body: &str) -> Result<String, SyncError> {
        self.answer()?;
        assert!(url.ends_with("files/list_folder"), "{url}");
        let entries: Vec<serde_json::Value> = self
            .files
            .lock()
            .unwrap()
            .iter()
            .map(|(path, (_, rev))| {
                json!({ ".tag": "file", "path_display": path, "rev": format!("{rev:09x}") })
            })
            .collect();
        Ok(json!({ "entries": entries, "cursor": "c", "has_more": false }).to_string())
    }

    fn form(&self, _url: &str, _body: &str) -> Result<String, SyncError> {
        unreachable!("no token endpoint here")
    }

    fn upload(
        &self,
        _url: &str,
        _bearer: &str,
        arg: &str,
        body: &[u8],
    ) -> Result<String, SyncError> {
        self.answer()?;
        let arg: serde_json::Value = serde_json::from_str(arg).unwrap();
        self.uploads.lock().unwrap().push(arg.clone());
        let path = arg["path"].as_str().unwrap().to_string();
        let mut files = self.files.lock().unwrap();
        if arg["mode"][".tag"] == "update" {
            let listed = arg["mode"]["update"].as_str().unwrap();
            let current = files.get(&path).map(|(_, rev)| format!("{rev:09x}"));
            if current.as_deref() != Some(listed) {
                return Err(SyncError::Io(format!(
                    "files/upload answered 409: path/conflict/file/.. ({path})"
                )));
            }
        }
        let mut next = self.next_rev.lock().unwrap();
        *next += 1;
        files.insert(path, (body.to_vec(), *next));
        Ok("{}".into())
    }

    fn download(&self, _url: &str, _bearer: &str, arg: &str) -> Result<Vec<u8>, SyncError> {
        self.answer()?;
        let arg: serde_json::Value = serde_json::from_str(arg).unwrap();
        let path = arg["path"].as_str().unwrap();
        Ok(self.files.lock().unwrap()[path].0.clone())
    }
}

#[test]
fn a_busy_dropbox_is_waited_for_as_asked_and_the_request_then_goes_through() {
    let dropbox = Dropbox::default();
    *dropbox.failures.lock().unwrap() = vec![busy(Some(2)), busy(Some(3))];
    let slept = RefCell::new(Vec::new());
    let store = DropboxStore::new(&dropbox, "token").with_sleep(|d| slept.borrow_mut().push(d));

    assert!(store.list().unwrap().is_empty());
    assert_eq!(
        *slept.borrow(),
        [Duration::from_secs(2), Duration::from_secs(3)],
        "Retry-After, honoured"
    );
    assert_eq!(dropbox.calls(), 3);
}

#[test]
fn a_dropbox_that_stays_unavailable_is_given_up_on_within_the_bound() {
    let dropbox = Dropbox {
        always: Some(SyncError::Unavailable {
            status: 503,
            retry_after: None,
            reason: "503".into(),
        }),
        ..Dropbox::default()
    };
    let slept = RefCell::new(Vec::new());
    let store = DropboxStore::new(&dropbox, "token").with_sleep(|d| slept.borrow_mut().push(d));

    let error = store.get("devices/x/vocab.json").unwrap_err();
    assert!(
        matches!(error, SyncError::Unavailable { status: 503, .. }),
        "{error:?}"
    );
    assert_eq!(dropbox.calls(), RETRY_TRIES as usize);
    let total: Duration = slept.borrow().iter().sum();
    assert!(total <= RETRY_BUDGET, "{total:?}");
    // Backing off: each wait at least the one before it, give or take the jitter.
    let waits = slept.borrow().clone();
    assert_eq!(waits.len(), RETRY_TRIES as usize - 1);
    assert!(waits.last().unwrap() > waits.first().unwrap(), "{waits:?}");
}

#[test]
fn the_bound_holds_across_a_whole_sync_and_a_wait_past_it_is_not_started() {
    let dropbox = Dropbox::default();
    // Twenty-nine seconds asked for and given; then a wait that would go past the
    // budget is refused at once, and so is every one after it in this store.
    *dropbox.failures.lock().unwrap() = vec![busy(Some(29)), busy(Some(2)), busy(Some(5))];
    let slept = RefCell::new(Vec::new());
    let store = DropboxStore::new(&dropbox, "token").with_sleep(|d| slept.borrow_mut().push(d));

    let error = store.list().unwrap_err();
    assert!(
        matches!(
            error,
            SyncError::Unavailable {
                retry_after: Some(2),
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(*slept.borrow(), [Duration::from_secs(29)]);
    assert!(store
        .list()
        .unwrap_err()
        .to_string()
        .contains("too_many_requests"));
    assert_eq!(slept.borrow().len(), 1, "no more waiting in this store");

    // Anything that is not a busy server is answered at once, never tried again.
    let dropbox = Dropbox::default();
    *dropbox.failures.lock().unwrap() = vec![SyncError::Io("409".into())];
    let store = DropboxStore::new(&dropbox, "token").with_sleep(|_| panic!("no wait"));
    assert!(store.list().is_err());
    assert_eq!(dropbox.calls(), 1);
}

#[test]
fn an_append_through_dropbox_is_an_update_against_the_listed_rev() {
    let (_dir_a, a) = device("dropbox-a");
    let dropbox = Dropbox::default();
    let store = DropboxStore::new(&dropbox, "token").with_sleep(|_| panic!("no wait"));

    practise(&a, 0, 1);
    sync(&a, &store).unwrap();
    let shard = format!("/{}", own_shards(&store, &a).remove(0));
    let rev = format!("{:09x}", dropbox.files.lock().unwrap()[&shard].1);
    dropbox.uploads.lock().unwrap().clear();

    practise(&a, 1, 1);
    sync(&a, &store).unwrap();
    let uploads = dropbox.uploads.lock().unwrap().clone();
    let append = uploads
        .iter()
        .find(|arg| arg["path"] == shard.as_str())
        .expect("the shard was uploaded");
    assert_eq!(append["mode"], json!({ ".tag": "update", "update": rev }));
    assert_eq!(read_attempts(&store).unwrap().len(), 2);
    assert_eq!(own_shards(&store, &a).len(), 1);

    // Every other upload — the documents — still overwrites.
    assert!(uploads
        .iter()
        .filter(|arg| arg["path"] != shard.as_str())
        .all(|arg| arg["mode"] == json!({ ".tag": "overwrite" })));
}

#[test]
fn a_retry_after_of_nothing_still_backs_off() {
    let dropbox = Dropbox::default();
    *dropbox.failures.lock().unwrap() = vec![busy(Some(0)), busy(Some(0))];
    let slept = RefCell::new(Vec::new());
    let store = DropboxStore::new(&dropbox, "token").with_sleep(|d| slept.borrow_mut().push(d));
    store.list().unwrap();
    let waits = slept.borrow().clone();
    assert_eq!(waits.len(), 2);
    assert!(
        waits.iter().all(|wait| *wait >= Duration::from_millis(250)),
        "never back to back: {waits:?}"
    );
}

#[test]
fn a_conflicting_append_through_dropbox_says_so_in_words() {
    let (_dir_a, a) = device("dropbox-conflict-a");
    let dropbox = Dropbox::default();
    let store = DropboxStore::new(&dropbox, "token").with_sleep(|_| panic!("no wait"));
    practise(&a, 0, 1);
    sync(&a, &store).unwrap();
    let error = store
        .replace(&own_shards(&store, &a)[0], b"x\n", "not-the-rev")
        .unwrap_err();
    assert!(error.to_string().contains("at the same time"), "{error}");
}
