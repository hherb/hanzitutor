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
use std::path::PathBuf;
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

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("hanzi-sync-cost-{name}-{}", std::process::id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).unwrap();
    path
}

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
    fn open(root: &PathBuf) -> Self {
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
    let (dir_a, a) = device("fill-a");
    let (dir_b, b) = device("fill-b");
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
    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn the_open_shard_is_closed_at_chunk_and_the_next_one_started() {
    let (dir_a, a) = device("chunk-a");
    let (dir_b, b) = device("chunk-b");
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
    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn an_older_builds_one_line_shards_are_left_and_the_last_of_them_grows() {
    // A device that synced with an earlier build has one shard per publish. The
    // new build leaves every one of them as it is and appends to the last — and a
    // peer reading the way an earlier build does (`pull`, every shard, every
    // time) takes the longer file as more attempts, with nothing rewritten.
    let (dir_a, a) = device("legacy-a");
    let (dir_b, b) = device("legacy-b");
    let shared = scratch("legacy-store");
    let remote = FolderStore::open(&shared).unwrap();

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
                &hanzi_sync::attempts_shard_name(a.device_id(), seq),
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
    finish(&[&dir_a, &dir_b, &shared]);
}

// ---- the store is listed once and an unchanged shard not fetched again -------

#[test]
fn a_sync_lists_the_store_once() {
    let (dir_a, a) = device("list-a");
    let (dir_b, b) = device("list-b");
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
    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn an_unchanged_shard_is_not_downloaded_again() {
    let (dir_a, a) = device("cache-a");
    let (dir_b, b) = device("cache-b");
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
    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn a_peer_whose_log_lost_what_it_had_folded_downloads_the_shard_again() {
    // The record of folded shards is checked against the log, not trusted: a
    // database that no longer holds a shard's attempts (restored from an older
    // backup, say) fetches it rather than taking the record's word.
    let (dir_a, a) = device("lost-a");
    let (dir_b, b) = device("lost-b");
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
    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn a_damaged_shard_is_fetched_every_sync_and_comes_back_once_mended() {
    let (dir_a, a) = device("mended-a");
    let (dir_b, b) = device("mended-b");
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
    finish(&[&dir_a, &dir_b, &shared]);
}

// ---- growth is the only change a shard may undergo ---------------------------

#[test]
fn a_shard_whose_lines_changed_is_rewritten_even_after_its_old_version_was_folded() {
    let (dir_a, a) = device("edited-a");
    let (dir_b, b) = device("edited-b");
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
        matches!(&error, SyncError::Rewritten { device_id, seq: 1 } if device_id == a.device_id()),
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
    finish(&[&dir_a, &dir_b, &shared]);
}

#[test]
fn a_second_writer_in_this_directory_makes_the_append_fail_rather_than_be_lost() {
    // Two installs sharing one identity (#12) are the one way two writers meet in
    // one directory. The append is checked against the revision it was listed
    // with, so the second of them fails rather than drops what the first wrote.
    let (dir_a, a) = device("race-a");
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
    finish(&[&dir_a, &shared]);
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
    let (dir_a, a) = device("dropbox-a");
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
    finish(&[&dir_a]);
}
