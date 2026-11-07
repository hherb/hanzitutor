//! The shards a sync could not use, and how a sync carries on without them.
//!
//! A sync reads every device's files, so it used to take one bad file to stop it
//! for everybody: a device that upgraded and wrote its vocabulary in a newer format,
//! or one shard a sync client truncated, and every other device stopped syncing
//! *everything* — attempts included — until somebody found the file.
//!
//! Now a file that cannot be used is set aside, **per device and per kind of
//! document**, and the sync goes on with the rest. Leaving one out is safe for the
//! same reason throughout: nothing here ever acts on *absence*.
//!
//! - **Attempts** are a grow-only set. A device whose shards are left out has
//!   simply not been heard from yet; the local log keeps everything it already
//!   holds, and the schedule is folded from that, exactly as it would be before the
//!   device's first sync. All of that device's attempt shards are left out
//!   together, never one of them, so a fold never sees a log with a hole in it.
//! - **The vocabulary list, the course position and the group positions** settle by
//!   last-writer-wins over records that are *present*. A removal is a record (a
//!   tombstone), never a missing entry, so leaving a device's document out cannot
//!   remove anything of that device's; it only delays its changes. And no device
//!   ever writes another device's document.
//! - **A baseline** is the one whose absence would change a result: without it a
//!   migrated card's rows would be folded from nothing. So when any baseline is left
//!   out the schedules are **not rebuilt** on that sync at all — every card keeps the
//!   schedule it has, which is stale at worst and never invented.
//!
//! Every file left out is reported in [`crate::Summary::skipped`], and
//! [`crate::Summary::notice`] says it in words a learner can act on.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

use crate::document::{is_document, screen_document};
use crate::seen::Seen;
use crate::shard::{device_of_shard, parse_attempts_shard, parse_shard};
use crate::store::{RemoteEntry, RemoteStore, SyncError};

/// Which document a skipped file was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShardKind {
    Attempts,
    Vocab,
    Cursor,
    VocabCursors,
    Baseline,
}

/// One file a sync left out, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedShard {
    /// The file's name in the store.
    pub name: String,
    /// The device whose directory it is in.
    pub device_id: String,
    pub kind: ShardKind,
    /// In this device's own directory: a file this device wrote, damaged since.
    pub own: bool,
    /// Written by a newer version of the app, rather than unreadable.
    pub newer_format: bool,
    /// What was wrong, for a log or a bug report.
    pub reason: String,
}

/// A store as one sync sees it: every file read once, and the ones this build
/// cannot use left out of the listing.
///
/// The readers in [`crate::document`] and [`crate::shard`] stay strict — a file
/// they are handed that they cannot read is still an error, because a reader that
/// skipped silently is how data goes missing unnoticed. What changes is that a sync
/// reads every file up front, through the same parsers, and hands the readers a
/// listing without the files they would refuse. So every reader on the sync's path
/// tolerates a bad file the same way, without each having to remember to.
///
/// Files are kept as they are read, so this costs no extra download: the store is
/// **listed once per sync** and each file fetched at most once, and every reader
/// after that is served from here. An attempt shard this database has already
/// folded at the revision it is listed with is not fetched at all: its text is
/// rebuilt from the local log ([`crate::seen`]). The one exception is this
/// device's own *last* shard, which is always fetched, because it is the file a
/// publish appends to and the append keeps its bytes exactly as they are. A write
/// goes through to the real store and into the view too, so a reader later in the
/// sync sees what this device has just published.
///
/// One guard, because a view that leaves files out must never be the reason one is
/// overwritten: a write to a file that was left out is **not made**, unless it is
/// one of this device's own documents (or a repair, through [`Screened::mend`]). Nothing in a sync writes into another device's
/// directory today, so this costs nothing; it is here so that a later change that
/// does — publishing attempts on a peer's behalf, say — cannot replace a file it
/// could not read with the part of it this device happens to know. A write it
/// refuses is recorded ([`Screened::withheld`]) rather than passed off as done.
///
/// A file in this device's **own** directory that was left out is the opposite
/// case: this device is the only one that can mend it, and nobody else will. See
/// `repair_own_shards` in [`crate::local`].
pub(crate) struct Screened<'a> {
    inner: &'a dyn RemoteStore,
    own_device: String,
    listing: RefCell<Vec<RemoteEntry>>,
    held: RefCell<BTreeMap<String, Vec<u8>>>,
    left_out: RefCell<BTreeSet<String>>,
    skipped: Vec<SkippedShard>,
    /// The sequence numbers of this device's attempts that a readable shard holds.
    own_readable: BTreeSet<i64>,
    /// The sequence number each of this device's attempt shards is named after,
    /// readable or not.
    own_starts: BTreeSet<i64>,
    /// Writes refused by the guard, and how many attempts they carried.
    withheld: RefCell<Vec<String>>,
    withheld_attempts: Cell<usize>,
}

impl<'a> Screened<'a> {
    /// Read every file a sync uses, and set aside the ones that cannot be used.
    ///
    /// A store that cannot be listed or a file that cannot be fetched is still an
    /// error: that is the network or the account, and the sync should say so rather
    /// than carry on as though every device had gone quiet.
    ///
    /// `own_device` is whatever `Db::device_id` hands back, borrowed or owned.
    #[cfg(test)]
    pub(crate) fn read(
        inner: &'a dyn RemoteStore,
        own_device: impl Into<String>,
    ) -> Result<Self, SyncError> {
        Self::read_with(inner, own_device, &Seen::nothing())
    }

    /// The same, rebuilding from the local log every attempt shard `seen` says
    /// this database has already folded at the revision it is listed with, rather
    /// than downloading it.
    pub(crate) fn read_with(
        inner: &'a dyn RemoteStore,
        own_device: impl Into<String>,
        seen: &Seen,
    ) -> Result<Self, SyncError> {
        let own_device: String = own_device.into();
        let listing = inner.list()?;
        let mut own_readable = BTreeSet::new();
        let own_starts: BTreeSet<i64> = listing
            .iter()
            .filter_map(|entry| parse_attempts_shard(&entry.name))
            .filter(|(device_id, _)| *device_id == own_device)
            .map(|(_, first)| first)
            .collect();
        let own_last = own_starts.last().copied();
        let mut held = BTreeMap::new();
        let mut left_out = BTreeSet::new();
        let mut skipped = Vec::new();
        let mut unreadable_logs = BTreeSet::new();
        for entry in &listing {
            if let Some((device_id, first)) = parse_attempts_shard(&entry.name) {
                // This device's last shard is always fetched: a publish appends to
                // its bytes, which must be the store's and not a rebuilt copy.
                let open_tail = device_id == own_device && Some(first) == own_last;
                let rebuilt = if open_tail {
                    None
                } else {
                    seen.rebuild(&entry.name, &entry.revision)
                };
                let bytes = match rebuilt {
                    Some(bytes) => bytes,
                    None => inner.get(&entry.name)?,
                };
                match parse_shard(&entry.name, device_id, &bytes) {
                    Ok(attempts) => {
                        if device_id == own_device {
                            own_readable.extend(attempts.iter().map(|a| a.seq));
                        }
                        held.insert(entry.name.clone(), bytes);
                    }
                    Err(fault) => {
                        unreadable_logs.insert(device_id.to_string());
                        left_out.insert(entry.name.clone());
                        skipped.push(SkippedShard {
                            name: entry.name.clone(),
                            device_id: device_id.to_string(),
                            kind: ShardKind::Attempts,
                            own: device_id == own_device,
                            newer_format: fault.newer_format,
                            reason: fault.reason,
                        });
                    }
                }
            } else if is_document(&entry.name) {
                let bytes = inner.get(&entry.name)?;
                match screen_document(&entry.name, &bytes) {
                    Some(Err(mut skip)) => {
                        left_out.insert(entry.name.clone());
                        skip.own = skip.device_id == own_device;
                        skipped.push(skip);
                    }
                    _ => {
                        held.insert(entry.name.clone(), bytes);
                    }
                }
            }
        }
        // Another device with one unreadable attempt shard is left out whole: a log
        // with a hole in it would fold into a schedule no device holding the whole
        // log computes. That is no worse than the device not having synced yet —
        // the local log keeps everything it already pulled.
        //
        // This device's own readable shards stay in view, and only its damaged ones
        // are hidden. Its own attempts are all in its local log, so hiding them
        // protects no fold; and what the store holds under this device's name is
        // exactly what a check of this device's identity has to see — a copy of
        // the database (#12) is recognised by the store knowing attempts under its
        // name that its log does not, and a view with none of them would let it
        // publish over the original's.
        for entry in &listing {
            if let Some((device_id, _)) = parse_attempts_shard(&entry.name) {
                if device_id != own_device && unreadable_logs.contains(device_id) {
                    left_out.insert(entry.name.clone());
                    held.remove(&entry.name);
                }
            }
        }
        let listing = listing
            .into_iter()
            .filter(|entry| !left_out.contains(&entry.name))
            .collect();
        Ok(Self {
            inner,
            own_device,
            listing: RefCell::new(listing),
            held: RefCell::new(held),
            left_out: RefCell::new(left_out),
            skipped,
            own_readable,
            own_starts,
            withheld: RefCell::new(Vec::new()),
            withheld_attempts: Cell::new(0),
        })
    }

    /// This device's own attempt shards that are damaged (not merely in a newer
    /// format), by name, with the sequence number each is named after.
    pub(crate) fn own_damaged_attempts(&self) -> Vec<(String, i64)> {
        self.skipped()
            .into_iter()
            .filter(|s| s.own && s.kind == ShardKind::Attempts && !s.newer_format)
            .filter_map(|s| parse_attempts_shard(&s.name).map(|(_, first)| (s.name.clone(), first)))
            .collect()
    }

    /// The first sequence number of the next of this device's attempt shards
    /// after the one named after `first`, if there is one.
    pub(crate) fn next_own_start(&self, first: i64) -> Option<i64> {
        self.own_starts.range(first + 1..).next().copied()
    }

    /// The sequence number this device's last attempt shard is named after,
    /// readable or not: the one a publish may append to.
    pub(crate) fn last_own_start(&self) -> Option<i64> {
        self.own_starts.last().copied()
    }

    /// Every attempt shard in view at a known revision, with the sequence numbers
    /// it holds: what [`crate::seen`] records once the pull has folded them.
    ///
    /// A file this sync wrote is not among them — its new revision is not known
    /// until the next listing — and neither is one that was left out.
    pub(crate) fn folded_attempt_shards(&self) -> Vec<(String, String, Vec<i64>)> {
        let held = self.held.borrow();
        self.listing
            .borrow()
            .iter()
            .filter(|entry| !entry.revision.is_empty())
            .filter_map(|entry| {
                let (device_id, _) = parse_attempts_shard(&entry.name)?;
                let bytes = held.get(&entry.name)?;
                let attempts = parse_shard(&entry.name, device_id, bytes).ok()?;
                Some((
                    entry.name.clone(),
                    entry.revision.clone(),
                    attempts.into_iter().map(|a| a.seq).collect(),
                ))
            })
            .collect()
    }

    /// The first sequence number of the previous one, if there is one.
    pub(crate) fn previous_own_start(&self, first: i64) -> Option<i64> {
        self.own_starts.range(..first).next_back().copied()
    }

    /// Whether a readable shard holds this device's attempt `seq`.
    pub(crate) fn holds_own(&self, seq: i64) -> bool {
        self.own_readable.contains(&seq)
    }

    /// The files that were left out and that this device has since written afresh:
    /// its own, mended.
    pub(crate) fn repaired(&self) -> Vec<SkippedShard> {
        let left_out = self.left_out.borrow();
        self.skipped
            .iter()
            .filter(|s| !left_out.contains(&s.name))
            .cloned()
            .collect()
    }

    /// The writes the guard refused, by name.
    pub(crate) fn withheld(&self) -> Vec<String> {
        self.withheld.borrow().clone()
    }

    /// How many attempts those writes carried, so they are not counted as sent.
    pub(crate) fn withheld_attempts(&self) -> usize {
        self.withheld_attempts.get()
    }

    /// The files left out, in the order they were found — less any this device has
    /// since written afresh, which were its own and are readable again.
    pub(crate) fn skipped(&self) -> Vec<SkippedShard> {
        let left_out = self.left_out.borrow();
        self.skipped
            .iter()
            .filter(|s| left_out.contains(&s.name))
            .cloned()
            .collect()
    }

    /// Whether another device's baseline was left out — see the module note for why
    /// that stops the schedules being rebuilt. This device's own does not count: a
    /// sync folds from the copy in its own database, never from the store's.
    pub(crate) fn baseline_skipped(&self) -> bool {
        self.skipped()
            .iter()
            .any(|s| s.kind == ShardKind::Baseline && !s.own)
    }
}

impl RemoteStore for Screened<'_> {
    fn list(&self) -> Result<Vec<RemoteEntry>, SyncError> {
        Ok(self.listing.borrow().clone())
    }

    fn get(&self, name: &str) -> Result<Vec<u8>, SyncError> {
        if let Some(bytes) = self.held.borrow().get(name) {
            return Ok(bytes.clone());
        }
        self.inner.get(name)
    }

    fn put(&self, name: &str, bytes: &[u8]) -> Result<(), SyncError> {
        if self.withhold(name, bytes) {
            return Ok(());
        }
        self.inner.put(name, bytes)?;
        self.written(name, bytes);
        Ok(())
    }

    /// The same guard as [`Screened::put`], then the store's compare-and-swap.
    fn replace(&self, name: &str, bytes: &[u8], revision: &str) -> Result<(), SyncError> {
        if self.withhold(name, bytes) {
            return Ok(());
        }
        self.inner.replace(name, bytes, revision)?;
        self.written(name, bytes);
        Ok(())
    }
}

impl Screened<'_> {
    /// Whether the guard refuses this write, recording it if so.
    fn withhold(&self, name: &str, bytes: &[u8]) -> bool {
        let own = device_of_shard(name) == Some(self.own_device.as_str());
        let attempts = parse_attempts_shard(name).is_some();
        if (!own || attempts) && self.left_out.borrow().contains(name) {
            // See the type's note: never overwrite a file this sync could not read.
            // Not an error, because the sync can go on without it; recorded, so the
            // write is neither lost silently nor counted as made.
            //
            // This device's own *documents* are the exception — it writes them
            // whole every sync, from its own database — but not its own attempt
            // shards. One of those that was left out is either in a newer format
            // or damaged, and a publish that happens to choose its name holds only
            // the attempts it decided to write, not what the file held: only
            // [`Screened::mend`], after its checks, may replace it.
            self.withheld.borrow_mut().push(name.to_string());
            if parse_attempts_shard(name).is_some() {
                let lines = String::from_utf8_lossy(bytes)
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .count();
                self.withheld_attempts
                    .set(self.withheld_attempts.get() + lines);
            }
            return true;
        }
        false
    }

    /// Replace one of this device's own damaged attempt shards — the one write the
    /// guard in `put` lets through to such a file. Only `repair_own_shards` calls
    /// it, once it has checked that this log can have written the file.
    pub(crate) fn mend(&self, name: &str, bytes: &[u8]) -> Result<(), SyncError> {
        debug_assert!(device_of_shard(name) == Some(self.own_device.as_str()));
        self.write_through(name, bytes)
    }

    /// Write to the store, and into this view.
    fn write_through(&self, name: &str, bytes: &[u8]) -> Result<(), SyncError> {
        self.inner.put(name, bytes)?;
        self.written(name, bytes);
        Ok(())
    }

    /// Bring this view up to date with a write the store has accepted.
    ///
    /// The file's revision is not known until the next listing, so it is
    /// recorded as unknown: a later write in this sync is then a plain one, and
    /// [`crate::seen`] does not record the file as folded at the revision it had
    /// before.
    fn written(&self, name: &str, bytes: &[u8]) {
        self.left_out.borrow_mut().remove(name);
        self.held
            .borrow_mut()
            .insert(name.to_string(), bytes.to_vec());
        let mut listing = self.listing.borrow_mut();
        match listing.iter_mut().find(|entry| entry.name == name) {
            Some(entry) => entry.revision.clear(),
            None => listing.push(RemoteEntry {
                name: name.to_string(),
                revision: String::new(),
            }),
        }
    }
}

/// What to tell the learner about the files a sync left out or mended, if any.
pub(crate) fn notice(skipped: &[SkippedShard], repaired: &[SkippedShard]) -> Option<String> {
    let mut sentences = Vec::new();
    if skipped.iter().any(|s| s.newer_format && !s.own) {
        sentences.push(
            "Another device uses a newer version of the app; update this one to sync its \
             changes."
                .to_string(),
        );
    }
    let unreadable = skipped.iter().filter(|s| !s.newer_format && !s.own).count();
    if unreadable > 0 {
        sentences.push(format!(
            "{} could not be read, so {} left out of this sync; everything else synced.",
            match unreadable {
                1 => "One file another device wrote".to_string(),
                n => format!("{n} files other devices wrote"),
            },
            if unreadable == 1 {
                "its changes were"
            } else {
                "their changes were"
            },
        ));
    }
    if !repaired.is_empty() {
        sentences.push(
            "A file this device wrote to Dropbox was damaged; it has been written again."
                .to_string(),
        );
    }
    if skipped.iter().any(|s| s.own) {
        sentences.push(
            "A file this device wrote to Dropbox earlier cannot be read, and this device \
             cannot write it again, so it was left as it is."
                .to_string(),
        );
    }
    if skipped
        .iter()
        .any(|s| s.kind == ShardKind::Baseline && !s.own)
    {
        sentences.push(
            "Schedules were left as they are until that device's files can be read.".to_string(),
        );
    }
    (!sentences.is_empty()).then(|| sentences.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skipped(kind: ShardKind, newer_format: bool) -> SkippedShard {
        SkippedShard {
            name: "devices/x/vocab.json".into(),
            device_id: "x".into(),
            kind,
            own: false,
            newer_format,
            reason: "because".into(),
        }
    }

    #[test]
    fn a_file_left_out_is_never_overwritten_except_in_this_devices_own_directory() {
        let root = std::env::temp_dir().join(format!("hanzi-screened-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        let store = crate::FolderStore::open(&root).unwrap();
        store
            .put("devices/peer/vocab.json", b"{\"version\": 9}")
            .unwrap();
        store.put("devices/me/vocab.json", b"damaged").unwrap();

        let screened = Screened::read(&store, "me").unwrap();
        assert!(
            screened.list().unwrap().is_empty(),
            "both are left out of the view"
        );
        assert_eq!(screened.skipped().len(), 2);

        // A peer's file this sync could not read is not replaced with what this
        // device happens to know...
        screened.put("devices/peer/vocab.json", b"{}").unwrap();
        assert_eq!(
            store.get("devices/peer/vocab.json").unwrap(),
            b"{\"version\": 9}"
        );
        // ...but this device's own is its to rewrite, and is readable again after.
        screened.put("devices/me/vocab.json", b"{}").unwrap();
        assert_eq!(store.get("devices/me/vocab.json").unwrap(), b"{}");
        assert_eq!(
            screened.skipped().len(),
            1,
            "only the peer's is still left out"
        );
        assert!(!screened.baseline_skipped());
        assert_eq!(screened.repaired().len(), 1, "and its own counts as mended");
        assert_eq!(
            screened.withheld(),
            ["devices/peer/vocab.json"],
            "the refused write is recorded, not passed off as made"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn this_devices_readable_shards_stay_in_view_when_one_of_them_is_damaged() {
        // Another device with a damaged shard is left out whole; this device's own
        // readable shards are not, because what the store holds under this
        // device's name is what a check of its identity has to see (#12).
        let root = std::env::temp_dir().join(format!("hanzi-screened-own-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        let store = crate::FolderStore::open(&root).unwrap();
        let line = |seq: i64| {
            format!(
                "{{\"seq\":{seq},\"ch\":\"好\",\"at\":\"2026-09-20T09:00:00Z\",\"score\":80.0,\"rating\":\"good\"}}\n"
            )
        };
        let own_good = crate::attempts_shard_name("me", 1);
        let own_bad = crate::attempts_shard_name("me", 2);
        let peer_good = crate::attempts_shard_name("peer", 1);
        let peer_bad = crate::attempts_shard_name("peer", 2);
        store.put(&own_good, line(1).as_bytes()).unwrap();
        store.put(&own_bad, b"{truncated").unwrap();
        store.put(&peer_good, line(1).as_bytes()).unwrap();
        store.put(&peer_bad, b"{truncated").unwrap();

        let screened = Screened::read(&store, "me").unwrap();
        let names: Vec<String> = screened
            .list()
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(
            names,
            [own_good],
            "only this device's readable shard is in view"
        );
        assert!(screened.holds_own(1));
        assert_eq!(screened.own_damaged_attempts(), [(own_bad.clone(), 2)]);

        // Not even this device's own publish may write over its damaged shard —
        // only a repair, through `mend`.
        screened.put(&own_bad, line(2).as_bytes()).unwrap();
        assert_eq!(store.get(&own_bad).unwrap(), b"{truncated");
        assert_eq!(screened.withheld(), vec![own_bad.clone()]);
        assert_eq!(screened.withheld_attempts(), 1);
        screened.mend(&own_bad, line(2).as_bytes()).unwrap();
        assert_eq!(store.get(&own_bad).unwrap(), line(2).as_bytes());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_newer_device_is_named_as_the_reason_and_the_remedy_given() {
        let message = notice(&[skipped(ShardKind::Vocab, true)], &[]).unwrap();
        assert_eq!(
            message,
            "Another device uses a newer version of the app; update this one to sync its changes."
        );
    }

    #[test]
    fn an_unreadable_file_is_counted_and_a_skipped_baseline_says_schedules_waited() {
        let message = notice(
            &[
                skipped(ShardKind::Attempts, false),
                skipped(ShardKind::Baseline, false),
            ],
            &[],
        )
        .unwrap();
        assert!(
            message.starts_with("2 files other devices wrote could not be read"),
            "{message}"
        );
        assert!(
            message.contains("Schedules were left as they are"),
            "{message}"
        );
        assert_eq!(notice(&[], &[]), None);
    }

    #[test]
    fn this_devices_own_damaged_file_is_told_apart_from_another_devices() {
        let own = SkippedShard {
            own: true,
            ..skipped(ShardKind::Attempts, false)
        };
        assert_eq!(
            notice(&[], std::slice::from_ref(&own)).unwrap(),
            "A file this device wrote to Dropbox was damaged; it has been written again."
        );
        let message = notice(&[own], &[]).unwrap();
        assert!(!message.contains("another device"), "{message}");
        assert!(message.contains("this device wrote"), "{message}");
    }
}
