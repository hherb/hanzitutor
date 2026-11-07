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

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use crate::document::{is_document, screen_document};
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
/// Files are kept as they are read, so this costs no extra download: each is
/// fetched once per sync, as before. A write goes through to the real store and
/// into the view too, so a reader later in the sync sees what this device has just
/// published.
///
/// One guard, because a view that leaves files out must never be the reason one is
/// overwritten: a write to a file that was left out is **not made**, unless it is in
/// this device's own directory. Nothing in a sync writes into another device's
/// directory today, so this costs nothing; it is here so that a later change that
/// does — publishing attempts on a peer's behalf, say — cannot replace a file it
/// could not read with the part of it this device happens to know.
pub(crate) struct Screened<'a> {
    inner: &'a dyn RemoteStore,
    own_device: String,
    listing: RefCell<Vec<RemoteEntry>>,
    held: RefCell<BTreeMap<String, Vec<u8>>>,
    left_out: RefCell<BTreeSet<String>>,
    skipped: Vec<SkippedShard>,
}

impl<'a> Screened<'a> {
    /// Read every file a sync uses, and set aside the ones that cannot be used.
    ///
    /// A store that cannot be listed or a file that cannot be fetched is still an
    /// error: that is the network or the account, and the sync should say so rather
    /// than carry on as though every device had gone quiet.
    ///
    /// `own_device` is whatever `Db::device_id` hands back, borrowed or owned.
    pub(crate) fn read(
        inner: &'a dyn RemoteStore,
        own_device: impl Into<String>,
    ) -> Result<Self, SyncError> {
        let listing = inner.list()?;
        let mut held = BTreeMap::new();
        let mut left_out = BTreeSet::new();
        let mut skipped = Vec::new();
        let mut unreadable_logs = BTreeSet::new();
        for entry in &listing {
            if let Some((device_id, _)) = parse_attempts_shard(&entry.name) {
                let bytes = inner.get(&entry.name)?;
                match parse_shard(&entry.name, device_id, &bytes) {
                    Ok(_) => {
                        held.insert(entry.name.clone(), bytes);
                    }
                    Err(reason) => {
                        unreadable_logs.insert(device_id.to_string());
                        skipped.push(SkippedShard {
                            name: entry.name.clone(),
                            device_id: device_id.to_string(),
                            kind: ShardKind::Attempts,
                            newer_format: false,
                            reason,
                        });
                    }
                }
            } else if is_document(&entry.name) {
                let bytes = inner.get(&entry.name)?;
                match screen_document(&entry.name, &bytes) {
                    Some(Err(skip)) => {
                        left_out.insert(entry.name.clone());
                        skipped.push(skip);
                    }
                    _ => {
                        held.insert(entry.name.clone(), bytes);
                    }
                }
            }
        }
        // A device with one unreadable attempt shard is left out whole: a log with
        // a hole in it would fold into a schedule no device holding the whole log
        // computes. That is no worse than the device not having synced yet — the
        // local log keeps everything it already pulled.
        for entry in &listing {
            if let Some((device_id, _)) = parse_attempts_shard(&entry.name) {
                if unreadable_logs.contains(device_id) {
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
            own_device: own_device.into(),
            listing: RefCell::new(listing),
            held: RefCell::new(held),
            left_out: RefCell::new(left_out),
            skipped,
        })
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
            .any(|s| s.kind == ShardKind::Baseline && s.device_id != self.own_device)
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
        let own = device_of_shard(name) == Some(self.own_device.as_str());
        if !own && self.left_out.borrow().contains(name) {
            // See the type's note: never overwrite a file this sync could not read.
            return Ok(());
        }
        self.inner.put(name, bytes)?;
        self.left_out.borrow_mut().remove(name);
        self.held
            .borrow_mut()
            .insert(name.to_string(), bytes.to_vec());
        let mut listing = self.listing.borrow_mut();
        if !listing.iter().any(|entry| entry.name == name) {
            listing.push(RemoteEntry {
                name: name.to_string(),
                revision: String::new(),
            });
        }
        Ok(())
    }
}

/// What to tell the learner about the files a sync left out, if any were.
pub(crate) fn notice(skipped: &[SkippedShard]) -> Option<String> {
    if skipped.is_empty() {
        return None;
    }
    let mut sentences = Vec::new();
    if skipped.iter().any(|s| s.newer_format) {
        sentences.push(
            "Another device uses a newer version of the app; update this one to sync its \
             changes."
                .to_string(),
        );
    }
    let unreadable = skipped.iter().filter(|s| !s.newer_format).count();
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
    if skipped.iter().any(|s| s.kind == ShardKind::Baseline) {
        sentences.push(
            "Schedules were left as they are until that device's files can be read.".to_string(),
        );
    }
    Some(sentences.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skipped(kind: ShardKind, newer_format: bool) -> SkippedShard {
        SkippedShard {
            name: "devices/x/vocab.json".into(),
            device_id: "x".into(),
            kind,
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
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_newer_device_is_named_as_the_reason_and_the_remedy_given() {
        let message = notice(&[skipped(ShardKind::Vocab, true)]).unwrap();
        assert_eq!(
            message,
            "Another device uses a newer version of the app; update this one to sync its changes."
        );
    }

    #[test]
    fn an_unreadable_file_is_counted_and_a_skipped_baseline_says_schedules_waited() {
        let message = notice(&[
            skipped(ShardKind::Attempts, false),
            skipped(ShardKind::Baseline, false),
        ])
        .unwrap();
        assert!(
            message.starts_with("2 files other devices wrote could not be read"),
            "{message}"
        );
        assert!(
            message.contains("Schedules were left as they are"),
            "{message}"
        );
        assert_eq!(notice(&[]), None);
    }
}
