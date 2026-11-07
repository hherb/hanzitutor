//! The local database, as sync sees it.
//!
//! Everything in this module is plumbing between two things that are each already
//! tested: the `hanzi-store` log, and the pure merge in [`crate::shard`]. It is
//! kept in its own file precisely so that it is the *only* place the two meet.
//!
//! ## Why a device never publishes an attempt under its own name that it did not make
//!
//! [`Db::attempts`] answers with everything the log holds, which after one sync
//! includes attempts made on other devices. Publishing those under this device's
//! name would relabel a peer's work as this device's, and `(device_id, seq)` is the
//! identity the entire merge rests on. So an attempt is only ever published under
//! the device that **made** it: this device's own under its own name, and anything
//! else the log holds that no shard does — a restored backup's, or everything,
//! after a move to an empty store — under its maker's name. See [`publish`].
//!
//! ## What "already published" means
//!
//! Whatever the store holds, read from the store. Not a watermark in the database:
//! a watermark describes the store this database last synced *to*, and is wrong
//! about any other — a second Dropbox account, an app folder deleted to start
//! again — and it is carried, unchanged, by a database restored onto another phone.
//! The store is read anyway, because the pull needs every shard, so asking it costs
//! nothing extra; [`sync`] reads it once and hands the same answer to both halves.
//!
//! ## What a sync does, in order
//!
//! 1. **Settle this device's identity** against the store: if the store holds
//!    attempts under this device's name that this log does not, this database is a
//!    copy of another one, and it takes a new name before writing anything — see
//!    [`settle_identity`].
//! 2. **Publish** every attempt the log holds that no shard does.
//! 3. **Pull** every shard back into the local log, adding only what is missing.
//! 4. **Recompute** the schedule from the whole log, which is the step that makes
//!    two devices agree: neither schedule is merged, both are rebuilt from the
//!    same attempts.
//!
//! Recomputing handles one case specially, and the baseline is what it is for. A card
//! whose count exceeds its rows in the log had history before the log existed — that
//! is a card migrated from the pre-M10 JSON files, which kept only the newest twenty
//! attempts. Folding those rows alone would rebuild a schedule out of the *tail* of a
//! history, so the device that owns the card writes down the state the missing part
//! left behind, publishes it, and every device folds from it.
//!
//! A card with no baseline to fold from is still left exactly as it is and counted in
//! [`Summary::left_alone`]. That is what this did before the baseline existed, and it
//! is kept deliberately: a missing baseline has to degrade to a stale schedule, never
//! to an invented one.
//!
//! ## The caller must reload its schedule store afterwards
//!
//! A `ProgressStore` holds the document in memory and writes through it. A sync
//! rewrites the `progress_card` rows underneath it, so an open store is left
//! showing a stale schedule — and its next `save`, which every review performs,
//! would write that stale card back over the synced one. Doing so is *recoverable*
//! rather than fatal, because the log kept every attempt and the next sync rebuilds
//! the card from it; but it is a silent wrong schedule until then, which is exactly
//! the kind of failure this milestone exists to avoid. Whoever wires this to the
//! app must reload the store after a sync, and
//! `an_open_schedule_store_that_is_not_reloaded_can_undo_a_sync` in
//! `tests/two_devices.rs` is the test that says so.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use hanzi_core::progress::{CardState, ProgressError, ProgressSink};
use hanzi_store::{Db, IncomingAttempt, LoggedAttempt};

use crate::document::{
    baseline_shard_name, merge_cursor, merge_vocab, merge_vocab_cursors, read_baselines,
    read_cursor, read_vocab, read_vocab_cursors, write_baseline, write_cursor, write_vocab,
    write_vocab_cursors, Baseline, Baselines,
};
use crate::shard::{
    attempts_shard_name, fold_cards, merge_attempts, parse_attempts_shard, read_attempts,
    write_attempts, MergedAttempt,
};
use crate::store::{RemoteStore, SyncError};

/// Where this device records how far it has published its own log.
///
/// **Written, and no longer read.** What has been published is read from the store
/// itself — see the module note for why a value in the database cannot answer that.
/// It is still kept up to date for an older build that opens this database after a
/// downgrade: that build trusts it, and a value that had stopped moving would have it
/// re-publish everything from the point this one took over.
const PUBLISHED_THROUGH: &str = "sync:published_seq";

/// Where this device keeps its own baseline, as a JSON document.
///
/// In `meta` beside the watermark rather than in a file of its own, for the reason
/// given on `Db::meta_value`: one backup captures everything. It is written once and
/// never changes, because it describes history that has already happened — which is
/// also why it is a document here rather than a table.
const BASELINE_KEY: &str = "sync:baseline";

/// What one sync did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Attempts written to shards for the first time.
    ///
    /// Usually this device's newest, but not only: anything the log holds that the
    /// store did not — a restored backup's attempts, or the whole history after a
    /// move to an empty store — is published too, under the device that made it.
    pub published: usize,
    /// A peer's attempts that this log did not already hold.
    pub pulled: usize,
    /// Characters whose schedule was rebuilt from the merged log.
    pub recomputed: usize,
    /// Characters deliberately left alone because their log is incomplete.
    pub left_alone: usize,
    /// Attempts the store lacks that were *not* published, because the device
    /// they name has a name no device of this app could have — see
    /// `publishable_name`. They stay in the log; this says they are there.
    pub unpublishable: usize,
    /// Vocabulary entries and groups whose stored form changed.
    pub vocab_changed: usize,
    /// Whether the course position moved.
    pub cursor_moved: bool,
}

impl Summary {
    /// True when the sync found nothing to do, which is what a second run of an
    /// already-synced device should report.
    pub fn is_empty(&self) -> bool {
        self.published == 0
            && self.pulled == 0
            && self.recomputed == 0
            && self.vocab_changed == 0
            && !self.cursor_moved
    }
}

impl From<ProgressError> for SyncError {
    fn from(error: ProgressError) -> Self {
        SyncError::Io(error.to_string())
    }
}

/// Every attempt in the local log, this device's and everyone else's.
pub fn full_log(db: &Db) -> Result<Vec<MergedAttempt>, SyncError> {
    let rows = db.attempts(None).map_err(SyncError::Io)?;
    Ok(rows.into_iter().map(MergedAttempt::from).collect())
}

/// This device's own attempts only.
pub fn own_log(db: &Db) -> Result<Vec<MergedAttempt>, SyncError> {
    let rows = db.own_attempts().map_err(SyncError::Io)?;
    Ok(rows.into_iter().map(MergedAttempt::from).collect())
}

/// Publish every attempt the local log holds that no shard holds yet.
///
/// Reads the store to find out what it holds; [`sync`] reads it once for this and
/// the pull together. See `publish_against` for what is published, and why.
pub fn publish(db: &Db, store: &dyn RemoteStore) -> Result<usize, SyncError> {
    let remote = read_attempts(store)?;
    Ok(publish_against(db, store, &remote)?.0)
}

/// Publish against a reading of the store already in hand.
///
/// ## What is already published
///
/// Exactly what `remote` holds, matched by `(device_id, seq)`. So a retried sync
/// publishes nothing, a sync to an empty store publishes everything, and a failure
/// part-way re-publishes rather than skipping: nothing is recorded as published
/// until the store says it is.
///
/// ## What is published, and under whose name
///
/// Every attempt the log holds that `remote` does not, each **under the device
/// that made it**. This device's own are the ordinary case. The rest are a peer's
/// attempts this log holds and the store has lost — a move to a new account — or
/// attempts restored from a backup file under the old phone's name, which nobody
/// else is going to publish because the old phone is gone.
///
/// Writing into another device's directory looks like it breaks the rule that
/// every writer owns its own, and it is safe for the reason that rule exists: an
/// attempt is the same attempt whoever holds a copy of it, so two devices writing
/// one device's attempts write the same lines. What could go wrong is a *name*
/// colliding with a different shard, and that cannot happen either — a shard is
/// named after the first attempt in it, and every attempt written here is one the
/// store did not hold, so no shard of that name can be there. Two phones that
/// restored the same backup and both publish it write the same attempts under the
/// same name, which every peer merges as one.
///
/// An attempt the store holds a *different* copy of is not published: that
/// disagreement is for the pull to report, never for a publish to settle by
/// overwriting. This device's own name is the exception that needs more than
/// that, and gets it first — see [`settle_identity`].
fn publish_against(
    db: &Db,
    store: &dyn RemoteStore,
    remote: &[MergedAttempt],
) -> Result<(usize, usize), SyncError> {
    let held: HashMap<(&str, i64), &MergedAttempt> = remote
        .iter()
        .map(|attempt| ((attempt.device_id.as_str(), attempt.seq), attempt))
        .collect();

    settle_identity(db, store, &held)?;
    let device = db.device_id();

    let mut fresh: BTreeMap<String, Vec<MergedAttempt>> = BTreeMap::new();
    let mut unpublishable = 0usize;
    for attempt in full_log(db)? {
        if held.contains_key(&(attempt.device_id.as_str(), attempt.seq)) {
            continue;
        }
        if attempt.device_id != device && !publishable_name(&attempt.device_id) {
            // A name arrived in a backup file, which is text anybody can edit.
            // Skipped and counted rather than written — see `publishable_name`.
            // This device's own name is generated here and is always fine.
            unpublishable += 1;
            continue;
        }
        fresh
            .entry(attempt.device_id.clone())
            .or_default()
            .push(attempt);
    }

    let mut published = 0usize;
    for (owner, attempts) in &fresh {
        write_attempts(store, owner, attempts)?;
        published += attempts.len();
    }

    // Kept for an older build only — see `PUBLISHED_THROUGH`.
    let highest = held
        .keys()
        .filter(|(owner, _)| *owner == device)
        .map(|(_, seq)| *seq)
        .chain(
            fresh
                .get(&device)
                .into_iter()
                .flatten()
                .map(|attempt| attempt.seq),
        )
        .max()
        .unwrap_or(0);
    db.set_meta_value(PUBLISHED_THROUGH, &highest.to_string())
        .map_err(SyncError::Io)?;
    Ok((published, unpublishable))
}

/// Make sure this device's name is its own before anything is written under it.
///
/// ## The failure this prevents
///
/// The device id lives in the database, so a database restored from a backup —
/// a new phone, a reinstall, a copied data directory — comes back with the id it
/// was backed up with, and numbers its next attempt from where the *backup*
/// ended. If the original went on syncing after that backup, the store already
/// holds attempts under those numbers. Publishing would overwrite them, which
/// loses the original's attempts from the store and leaves every peer that pulled
/// them with two stories about one attempt — [`SyncError::Rewritten`] on every
/// sync from then on, with no way back. Two live copies of one database meet the
/// same collision on whichever syncs second.
///
/// ## How it is detected, and what it does
///
/// The store holds an attempt under this device's name that this log does not
/// hold, or holds differently. Nothing else produces that: this device's own log
/// is append-only and every attempt it publishes it keeps, so a store that knows
/// more about this name than this log does has heard from another copy of it.
///
/// The answer is a **new name**, never an overwrite. Everything this log shares
/// with the store stays under the old name, because it is the same history the
/// store already has; the attempts the store does not hold move to the new name,
/// numbered from 1, and are published there; and the pull that follows brings in
/// whatever the other copy did. Every device ends up with every attempt, each
/// under exactly one name, and nothing on the store was rewritten.
///
/// ## Attempts left under a former name
///
/// The rename asks of every row, inside its own transaction, whether the store
/// holds it — so an attempt a review records *while* this runs is either moved or
/// written under the new name, never left behind under the old one. An attempt can
/// still turn up under a former name some other way: a database renamed by a build
/// without that care, or a backup restored from before the rename. If the store
/// holds a *different* attempt under that name and number, only this device can
/// have made this one, so it is moved to the current name as well
/// ([`Db::reclaim_attempts`]) rather than left to fail every pull with
/// [`SyncError::Rewritten`]. A former name's attempts that the store does not hold
/// at all are left alone: they are as likely to be the original's, pulled before a
/// move to an empty store, and are published under that name like anyone else's.
///
/// ## What it cannot do
///
/// - **Repair a store the old behaviour already damaged.** A build without this
///   check overwrote the original's shard; a peer that had pulled the original's
///   attempt still disagrees with the store about it, and still fails with
///   [`SyncError::Rewritten`] — there is no telling, from here, which copy is the
///   real one. Worse, the *original* now finds the copy's attempt under its own
///   name, takes a new one itself, and republishes its own overwritten attempt
///   under it, so a peer that kept the first copy holds it twice.
/// - **Judge "unpublished" against any store but this one.** A copy used with two
///   accounts in turn renames what the *current* store lacks, which can include
///   attempts it already published to the other; were the two stores' contents
///   ever brought together, those would be there twice.
/// - **Help a third device with a copy's old backup.** A backup exported on a copy
///   *before* it renamed holds its attempts under the inherited name. Restored on
///   this device, they are dropped, because the rename recorded where they went
///   ([`Db::merge_attempts`]). Restored on any *other* device, nothing says so:
///   that device holds them as the inherited name's, publishes those the store
///   lacks, and one that collides with the original's attempt of the same number
///   leaves it failing with [`SyncError::Rewritten`] for good — much as restoring
///   any backup that disagrees with the store already did.
/// - **Cover the group positions published between syncs.** The app publishes a
///   group position on its own, outside a sync (`publish_positions_to` in
///   `src-tauri/src/sync.rs`), under whatever name the database has at that
///   moment. Before a copy's first sync, or in the moment around a rename, that is
///   the inherited name, so the copy can overwrite the original's
///   `vocab-cursor.json` once. Positions are settled per group by their stamps and
///   the file is rewritten whole on every publish, so the original's next publish
///   restores it; no attempt is involved and nothing fails.
/// - **Rebuild a migrated card when there is no baseline to adopt.** A rename with
///   none leaves a pre-M10 card stale on the copy — see `settle_baseline` — until
///   the original publishes a baseline, and for good if it never does.
///
/// Returns the new name, if there is one.
fn settle_identity(
    db: &Db,
    store: &dyn RemoteStore,
    held: &HashMap<(&str, i64), &MergedAttempt>,
) -> Result<Option<String>, SyncError> {
    let device = db.device_id();
    let own: HashMap<i64, MergedAttempt> = own_log(db)?
        .into_iter()
        .map(|attempt| (attempt.seq, attempt))
        .collect();
    let someone_else = held
        .iter()
        .filter(|((owner, _), _)| *owner == device)
        .any(|((_, seq), remote)| own.get(seq) != Some(*remote));

    // Before the name can change, because a baseline accounts for rows by name.
    settle_baseline(db, store, someone_else)?;

    let renamed = if someone_else {
        let published = |row: &LoggedAttempt| held_as_is(held, row);
        Some(db.rotate_device_id(&published).map_err(SyncError::Io)?)
    } else {
        None
    };

    // A copy under a former name of an attempt the rename already moved — a backup
    // from before the rename, restored by a build that let it in — goes first, so
    // that it is neither reclaimed below as a second attempt nor published again.
    db.forget_moved_copies().map_err(SyncError::Io)?;
    for former in db.former_device_ids().map_err(SyncError::Io)? {
        let keep = |row: &LoggedAttempt| {
            held.get(&(former.as_str(), row.seq))
                .is_none_or(|remote| **remote == MergedAttempt::from(row.clone()))
        };
        db.reclaim_attempts(&former, &keep).map_err(SyncError::Io)?;
    }
    Ok(renamed)
}

/// Whether the store holds exactly this attempt, under its name and number.
fn held_as_is(held: &HashMap<(&str, i64), &MergedAttempt>, row: &LoggedAttempt) -> bool {
    held.get(&(row.device_id.as_str(), row.seq))
        .is_some_and(|remote| **remote == MergedAttempt::from(row.clone()))
}

/// Settle this device's baseline before its name is checked, once.
///
/// A baseline says "this device's rows up to here are already inside these cards",
/// by name and number, so it has to agree with the store about both. Capturing one
/// here, the way a device that has never synced does, is wrong for a copy:
///
/// - **The store already holds a baseline under this name.** Then this database is
///   a copy of the device that captured it, taken before it did. That baseline is
///   *this* history too — the copy shares it up to the moment it was taken — and
///   every peer already folds from it, so it is adopted as this device's own. A
///   baseline of the copy's own would cover the original's attempts it never saw,
///   claim its own new ones, and overwrite the original's file besides.
/// - **The name is about to change.** The attempts the rename moves are inside the
///   cards as they stand, and would sit outside a baseline taken under the old name
///   — folded on top of a state that already counts them. There is no range under
///   one name that describes the cards, so none is captured: an empty one is
///   written down, and a migrated card is left alone rather than rebuilt wrong,
///   which is what a missing baseline has always meant. If the original publishes
///   its own later, every device folds from that.
///
/// Anything else is the ordinary first capture.
fn settle_baseline(db: &Db, store: &dyn RemoteStore, renaming: bool) -> Result<(), SyncError> {
    if own_baseline(db)?.is_some() {
        return Ok(());
    }
    let device = db.device_id();
    let adopted = read_baselines(store)?
        .into_iter()
        .find(|baseline| baseline.device_id == device);
    let baseline = match adopted {
        Some(baseline) => baseline,
        None if renaming => Baseline {
            device_id: device,
            from: 0,
            to: 0,
            cards: BTreeMap::new(),
        },
        None => {
            capture_baseline(db)?;
            return Ok(());
        }
    };
    let text = serde_json::to_string(&baseline)
        .map_err(|e| SyncError::Malformed(format!("the baseline could not be encoded: {e}")))?;
    db.set_meta_value(BASELINE_KEY, &text)
        .map_err(SyncError::Io)
}

/// Whether a device name is one this device may write a shard under.
///
/// Strictly a name a device of this app could have ([`hanzi_store::is_valid_device_id`]),
/// not merely one a path can hold. A name is a directory on the store, and the
/// names that are not plain are the dangerous ones: `..` or a backslash is refused
/// by a folder store and would fail *every* sync, and `.` or a `/` writes
/// somewhere [`parse_attempts_shard`] never reads back, so the same attempts would
/// be written again on every sync and never found. A restored backup is refused
/// such a name before it reaches the log; this is the guard for one that got in
/// some other way. The round trip is checked as well, so the two rules can never
/// drift apart.
fn publishable_name(device_id: &str) -> bool {
    let name = attempts_shard_name(device_id, 1);
    hanzi_store::is_valid_device_id(device_id)
        && parse_attempts_shard(&name) == Some((device_id, 1))
}

/// Publish this device's vocabulary list and course position.
///
/// Both are written **whole, every time**, and that is the difference from the
/// attempt log rather than an oversight. An attempt is appended and never changes,
/// so its shard is written once; an entry is edited, so there is nothing to append —
/// what a device has to say about an entry is whatever it currently holds. See
/// [`crate::document`] for why that does not put the merge at risk.
pub fn publish_documents(db: &Db, store: &dyn RemoteStore) -> Result<(), SyncError> {
    let device = db.device_id();
    let (entries, groups) = db.vocab_view().map_err(SyncError::Io)?;
    write_vocab(store, &device, &entries, &groups)?;
    // A device that has never moved the cursor has nothing to say about where it
    // is, and publishing a guess would be worse than publishing nothing.
    if let Some(cursor) = db.cursor_view().map_err(SyncError::Io)? {
        write_cursor(store, &device, &cursor)?;
    }
    // And where the learner got to in each of their own groups. Nothing to say
    // when there is nothing to say: absence must not mean "forget every group",
    // or a device that had merely not synced yet would wipe a peer's places.
    let cursors = db.vocab_cursors().map_err(SyncError::Io)?;
    if !cursors.is_empty() {
        write_vocab_cursors(store, &device, &cursors)?;
    }
    // Before the pull, and that ordering is the point: a peer that reads this
    // device's attempts must be able to read the baseline that says how to fold
    // them. A peer that misses it folds what it can and is corrected by its next
    // sync, which is the right direction to fail in.
    //
    // Only under this device's current name, or where the store has none for the
    // name it carries. A device that has been renamed still holds the baseline of
    // its former name, and the store's copy of that is the former name's to keep:
    // two devices writing one baseline file is how a peer's would be overwritten.
    // Republishing it to a store that lacks one — a move to a new account — is what
    // lets the new account fold that name's attempts correctly.
    if let Some(baseline) = capture_baseline(db)? {
        if baseline.device_id == device || !holds_baseline_of(store, &baseline.device_id)? {
            write_baseline(store, &baseline)?;
        }
    }
    Ok(())
}

/// Whether the store holds a baseline for `device_id`.
fn holds_baseline_of(store: &dyn RemoteStore, device_id: &str) -> Result<bool, SyncError> {
    let name = baseline_shard_name(device_id);
    Ok(store.list()?.iter().any(|entry| entry.name == name))
}

/// This device's baseline, captured on first use and read back afterwards.
///
/// ## What it is, and why it has to be captured rather than derived
///
/// A card imported from the pre-M10 JSON files remembers more attempts than the log
/// holds, because that format kept only the newest twenty. The rows that survive are
/// the *tail* of the character's history, and rebuilding the card from them alone
/// would produce a schedule out of the last twenty attempts of a hundred — wrong
/// everywhere, and wrong differently on a device that has only some of them. The
/// state the missing history left behind cannot be recovered from the log, so it is
/// written down once, here.
///
/// ## Why the covered range is what it is
///
/// The baseline accounts for **this device's own rows written so far**, and nothing
/// else. Every one of them is genuinely inside the captured state: an attempt this
/// device records goes through `apply_attempt` and is written back with the card in
/// the same `save`, so the card has always moved with the log. A *peer's* rows are
/// deliberately outside it — those arrived through a pull and, for exactly the cards
/// that need a baseline, the recompute that follows a pull has always skipped them.
/// Leaving them unfolded here would be the bug this exists to fix.
///
/// ## Why the capture is lazy rather than an upgrade step
///
/// It happens on the first sync after this existed, which is the first moment
/// anybody needs it. A database that has already migrated gets one the same way a new
/// one does, and one that never syncs never needs one — so there is no schema change
/// and nothing to get wrong on the upgrade path.
///
/// An empty baseline is still written down, so that the capture is a one-off rather
/// than a scan of every card on every sync. It is not *published* when empty: a file
/// every peer downloads to learn nothing is worse than no file.
fn capture_baseline(db: &Db) -> Result<Option<Baseline>, SyncError> {
    if let Some(text) = db.meta_value(BASELINE_KEY).map_err(SyncError::Io)? {
        if let Ok(baseline) = serde_json::from_str::<Baseline>(&text) {
            return Ok((!baseline.cards.is_empty()).then_some(baseline));
        }
    }

    let log = full_log(db)?;
    let mut rows: BTreeMap<String, u32> = BTreeMap::new();
    for attempt in &log {
        *rows.entry(attempt.ch.clone()).or_insert(0) += 1;
    }
    // `own_attempts` is ordered by `seq`, so the last one names the range.
    let next = db
        .own_attempts()
        .map_err(SyncError::Io)?
        .last()
        .map(|attempt| attempt.seq + 1)
        .unwrap_or(0);
    let document = db.load().map_err(|e| SyncError::Io(e.to_string()))?;
    let cards: BTreeMap<String, CardState> = document
        .cards
        .iter()
        .filter(|(ch, card)| card.attempts > rows.get(ch.as_str()).copied().unwrap_or(0))
        .map(|(ch, card)| (ch.clone(), card.clone()))
        .collect();

    let baseline = Baseline {
        device_id: db.device_id(),
        from: 0,
        to: next,
        cards,
    };
    let text = serde_json::to_string(&baseline)
        .map_err(|e| SyncError::Malformed(format!("the baseline could not be encoded: {e}")))?;
    db.set_meta_value(BASELINE_KEY, &text).map_err(SyncError::Io)?;
    Ok((!baseline.cards.is_empty()).then_some(baseline))
}

/// Bring every peer's attempts into the local log, returning how many were new.
///
/// The merge runs in memory before anything is written, so that a shard which
/// disagrees with one this device already has is *reported* rather than quietly
/// dropped by the database's `DO NOTHING`. That is the one check worth an extra
/// pass: a rewritten shard means something has gone wrong that the learner would
/// otherwise never hear about.
pub fn pull(db: &Db, store: &dyn RemoteStore) -> Result<usize, SyncError> {
    pull_from(db, read_attempts(store)?)
}

/// The pull, from a reading of the store already in hand.
///
/// [`sync`] hands it the reading the publish was decided from, rather than reading
/// every shard a second time. What that reading lacks is only what was written in
/// the moments since — this device's own just-published shards, which the log
/// already holds, and anything a peer wrote meanwhile, which the next sync brings.
fn pull_from(db: &Db, remote: Vec<MergedAttempt>) -> Result<usize, SyncError> {
    if remote.is_empty() {
        return Ok(0);
    }
    let local = full_log(db)?;
    let held: HashSet<(String, i64)> = local
        .iter()
        .map(|attempt| (attempt.device_id.clone(), attempt.seq))
        .collect();

    let merged = merge_attempts(local, remote)?;
    let incoming: Vec<IncomingAttempt> = merged
        .into_iter()
        .filter(|attempt| !held.contains(&(attempt.device_id.clone(), attempt.seq)))
        .map(IncomingAttempt::from)
        .collect();

    db.merge_attempts(&incoming).map_err(SyncError::Io)
}

/// Rebuild the local schedule from the whole log, folding from a baseline where one
/// exists.
///
/// Returns how many characters were rebuilt and how many were left alone. See the
/// module note for why any are left alone at all.
pub fn recompute(db: &Db, baselines: &Baselines) -> Result<(usize, usize), SyncError> {
    let log = full_log(db)?;
    let folded = fold_cards(&log, baselines);

    let mut rows_in_log: BTreeMap<&str, u32> = BTreeMap::new();
    for attempt in &log {
        *rows_in_log.entry(attempt.ch.as_str()).or_insert(0) += 1;
    }

    let mut db = db.clone();
    let mut document = db.load()?;
    let mut changed = BTreeSet::new();
    let mut left_alone = 0usize;

    for (ch, card) in folded {
        let existing = document.cards.get(&ch);
        let known = existing.map(|card| card.attempts).unwrap_or(0);
        let rows = rows_in_log.get(ch.as_str()).copied().unwrap_or(0);
        if known > rows && baselines.winner(&ch).is_none() {
            // Its own card remembers more attempts than the log holds, so the log is
            // not the whole story and folding it would lose the rest. A baseline is
            // what makes that story whole again, so what is left here is a card whose
            // device has not published one yet — a device that has not synced since
            // it migrated, or a peer's baseline that has not arrived. It is the *old*
            // behaviour, kept deliberately: a missing baseline has to degrade to
            // leaving the schedule alone, never to inventing one.
            left_alone += 1;
            continue;
        }
        if existing == Some(&card) {
            // The log already implies this schedule, so there is nothing to write.
            // Not an optimisation but the difference between "synced" and "wrote
            // the same thing again": a sync that changes nothing should report
            // that it changed nothing.
            continue;
        }
        document.cards.insert(ch.clone(), card);
        changed.insert(ch);
    }

    if !changed.is_empty() {
        db.save(&document, &changed, &[])?;
    }
    Ok((changed.len(), left_alone))
}

/// Settle the vocabulary list with every peer, returning how many rows changed.
///
/// The merge runs over this device's view and every shard, and what it produces is
/// applied back — so a device that already holds the winning version writes nothing,
/// and a device holding an older one does not keep it.
pub fn pull_vocab(db: &Db, store: &dyn RemoteStore) -> Result<usize, SyncError> {
    let (local_entries, local_groups) = db.vocab_view().map_err(SyncError::Io)?;
    let (remote_entries, remote_groups) = read_vocab(store)?;
    let (entries, groups) =
        merge_vocab(local_entries, local_groups, remote_entries, remote_groups);
    db.apply_vocab_view(&entries, &groups).map_err(SyncError::Io)
}

/// Move to whoever last moved the course position, returning whether it moved here.
pub fn pull_cursor(db: &Db, store: &dyn RemoteStore) -> Result<bool, SyncError> {
    let local = db.cursor_view().map_err(SyncError::Io)?;
    let remote = read_cursor(store)?;
    match merge_cursor(local, remote) {
        Some(winner) => db.set_cursor_from_sync(&winner).map_err(SyncError::Io),
        None => Ok(false),
    }
}

/// Move to whoever last moved each group's position, returning whether any moved.
///
/// One merge for every group at once, because the settling is *per group*: this
/// is the one place in the sync where two records are compared and must not be,
/// since a position only means anything against the list it was taken in.
pub fn pull_vocab_cursors(db: &Db, store: &dyn RemoteStore) -> Result<bool, SyncError> {
    let local = db.vocab_cursors().map_err(SyncError::Io)?;
    let remote = read_vocab_cursors(store)?;
    let merged = merge_vocab_cursors(local, remote);
    let changed = db.apply_vocab_cursors(&merged).map_err(SyncError::Io)?;
    Ok(changed > 0)
}

/// Publish, pull, and rebuild: one whole sync.
///
/// The order matters in two places. Attempts are published before anything is
/// pulled, because the log is what the schedule is folded from and a device should
/// never be in the position of having applied a peer's work without having offered
/// its own. And the documents are published *after* the attempts, because
/// publishing the attempts is where this device may take a new name — see
/// `settle_identity` — and the documents must go out under the name it ends up
/// with, not over the directory of the copy it turned out to be.
pub fn sync(db: &Db, store: &dyn RemoteStore) -> Result<Summary, SyncError> {
    // Read once, for both halves: the publish decides from it what the store
    // lacks, and the pull merges it.
    let remote = read_attempts(store)?;
    let (published, unpublishable) = publish_against(db, store, &remote)?;
    publish_documents(db, store)?;
    let pulled = pull_from(db, remote)?;
    let vocab_changed = pull_vocab(db, store)?;
    let cursor_moved = pull_cursor(db, store)?;
    // Whether a group position moved is not reported: unlike the course
    // position, the learner sees it the moment they open the group, and the
    // summary line is about what a sync did to the schedule.
    let _ = pull_vocab_cursors(db, store)?;
    // Every device's baseline, this one's included, resolved into the two questions
    // the fold asks of it. Read after the pull so that a peer's baseline arriving in
    // this pass is used in this pass rather than the next one.
    let mut baselines = read_baselines(store)?;
    if let Some(own) = own_baseline(db)? {
        baselines.push(own);
    }
    let (recomputed, left_alone) = recompute(db, &Baselines::resolve(baselines))?;
    Ok(Summary {
        published,
        pulled,
        recomputed,
        left_alone,
        unpublishable,
        vocab_changed,
        cursor_moved,
    })
}

/// This device's own baseline, if it has one.
///
/// Read from `meta` rather than from a store, because it is this device's own
/// record of history that predates the log — see the module note. A baseline that
/// cannot be parsed is treated as absent, which is the safe direction: a missing
/// baseline leaves a card alone rather than inventing one.
fn own_baseline(db: &Db) -> Result<Option<Baseline>, SyncError> {
    match db.meta_value(BASELINE_KEY).map_err(SyncError::Io)? {
        Some(text) => Ok(serde_json::from_str::<Baseline>(&text).ok()),
        None => Ok(None),
    }
}

/// This device's own baseline, resolved, with no remote to ask.
///
/// A **restore** needs this and nothing else. Restoring a backup file has no
/// store to read peers' baselines from, but the fold still has to know how to
/// treat a card this device migrated from the old JSON files, and that is exactly
/// what its own baseline records. With no baseline — the ordinary case — the log
/// is the whole story and every card folds from the beginning.
pub fn local_baselines(db: &Db) -> Result<Baselines, SyncError> {
    Ok(Baselines::resolve(own_baseline(db)?.into_iter().collect()))
}

/// A local log row, as it travels.
impl From<LoggedAttempt> for MergedAttempt {
    fn from(row: LoggedAttempt) -> Self {
        MergedAttempt {
            device_id: row.device_id,
            seq: row.seq,
            ch: row.ch,
            at: row.at,
            score: row.score,
            rating: hanzi_core::Rating::from_name(&row.rating).unwrap_or_else(|| {
                // The store refuses an unknown name on the way in, so this cannot
                // be reached through this program. Deriving from the score is the
                // harmless reading of a hand-edited database.
                hanzi_core::Rating::from_score(row.score)
            }),
        }
    }
}

/// An attempt on its way into the local log.
impl From<MergedAttempt> for IncomingAttempt {
    fn from(attempt: MergedAttempt) -> Self {
        IncomingAttempt {
            device_id: attempt.device_id,
            seq: attempt.seq,
            ch: attempt.ch,
            at: attempt.at,
            score: attempt.score,
            rating: attempt.rating,
            // A shard carries the score and the time and nothing else, so there
            // are no measures to hand over. A backup is the other source of
            // incoming attempts and does carry them — see `hanzi_store::backup`.
            measures: None,
        }
    }
}
