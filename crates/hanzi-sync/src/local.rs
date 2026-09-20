//! The local database, as sync sees it.
//!
//! Everything in this module is plumbing between two things that are each already
//! tested: the `hanzi-store` log, and the pure merge in [`crate::shard`]. It is
//! kept in its own file precisely so that it is the *only* place the two meet.
//!
//! ## Why a device must publish only its own attempts
//!
//! [`Db::attempts`] answers with everything the log holds, which after one sync
//! includes attempts made on other devices. Publishing that would relabel a peer's
//! work as this device's, and `(device_id, seq)` is the identity the entire merge
//! rests on — so publishing reads [`Db::own_attempts`] and nothing else. The two
//! questions look interchangeable and are not, which is why they have different
//! names.
//!
//! ## What a sync does, in order
//!
//! 1. **Publish** this device's attempts that no shard holds yet, and move the
//!    watermark.
//! 2. **Pull** every shard back into the local log, adding only what is missing.
//! 3. **Recompute** the schedule from the whole log, which is the step that makes
//!    two devices agree: neither schedule is merged, both are rebuilt from the
//!    same attempts.
//!
//! Recomputing is deliberately conservative about one case. A card whose count
//! exceeds its rows in the log had history before the log existed — that is a card
//! migrated from the pre-M10 JSON files, which kept only the newest twenty
//! attempts. Folding those from the log would discard the part the log never held,
//! so they are left exactly as they are until the baseline ROADMAP M13 describes
//! exists. Guessing would be worse than waiting.
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

use std::collections::{BTreeMap, BTreeSet, HashSet};

use hanzi_core::progress::{ProgressError, ProgressSink};
use hanzi_store::{Db, IncomingAttempt, LoggedAttempt};

use crate::shard::{fold_cards, merge_attempts, read_attempts, write_attempts, MergedAttempt};
use crate::store::{RemoteStore, SyncError};

/// Where this device records how far it has published its own log.
///
/// In the database's `meta` table beside the schema version and the import
/// markers, so that one backup captures it and a restored database does not
/// re-publish everything it ever recorded.
const PUBLISHED_THROUGH: &str = "sync:published_seq";

/// What one sync did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Attempts of this device's that were written to shards for the first time.
    pub published: usize,
    /// A peer's attempts that this log did not already hold.
    pub pulled: usize,
    /// Characters whose schedule was rebuilt from the merged log.
    pub recomputed: usize,
    /// Characters deliberately left alone because their log is incomplete.
    pub left_alone: usize,
}

impl Summary {
    /// True when the sync found nothing to do, which is what a second run of an
    /// already-synced device should report.
    pub fn is_empty(&self) -> bool {
        self.published == 0 && self.pulled == 0 && self.recomputed == 0
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

/// Publish this device's attempts that no shard holds yet.
///
/// The watermark is the highest sequence number already published, so a sync
/// publishes only what is new and a retried sync publishes nothing. It is advanced
/// only after the shards are written, so a failure re-publishes rather than
/// skipping — and re-publishing is safe, because the same attempts produce the same
/// shard names and the same bytes.
pub fn publish(db: &Db, store: &dyn RemoteStore) -> Result<usize, SyncError> {
    let published_through = watermark(db)?;
    let fresh: Vec<MergedAttempt> = own_log(db)?
        .into_iter()
        .filter(|attempt| attempt.seq > published_through)
        .collect();
    if fresh.is_empty() {
        return Ok(0);
    }

    write_attempts(store, db.device_id(), &fresh)?;

    let highest = fresh
        .iter()
        .map(|attempt| attempt.seq)
        .max()
        .unwrap_or(published_through);
    db.set_meta_value(PUBLISHED_THROUGH, &highest.to_string())
        .map_err(SyncError::Io)?;
    Ok(fresh.len())
}

/// Bring every peer's attempts into the local log, returning how many were new.
///
/// The merge runs in memory before anything is written, so that a shard which
/// disagrees with one this device already has is *reported* rather than quietly
/// dropped by the database's `DO NOTHING`. That is the one check worth an extra
/// pass: a rewritten shard means something has gone wrong that the learner would
/// otherwise never hear about.
pub fn pull(db: &Db, store: &dyn RemoteStore) -> Result<usize, SyncError> {
    let remote = read_attempts(store)?;
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

/// Rebuild the local schedule from the whole log.
///
/// Returns how many characters were rebuilt and how many were left alone. See the
/// module note for why any are left alone at all.
pub fn recompute(db: &Db) -> Result<(usize, usize), SyncError> {
    let log = full_log(db)?;
    let folded = fold_cards(&log);

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
        if known > rows {
            // Its own card remembers more attempts than the log holds, so the log
            // is not the whole story and folding it would lose the rest.
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

/// Publish, pull, and rebuild the schedule: one whole sync.
pub fn sync(db: &Db, store: &dyn RemoteStore) -> Result<Summary, SyncError> {
    let published = publish(db, store)?;
    let pulled = pull(db, store)?;
    let (recomputed, left_alone) = recompute(db)?;
    Ok(Summary {
        published,
        pulled,
        recomputed,
        left_alone,
    })
}

/// How far this device has published its own log.
fn watermark(db: &Db) -> Result<i64, SyncError> {
    Ok(db
        .meta_value(PUBLISHED_THROUGH)
        .map_err(SyncError::Io)?
        .and_then(|text| text.parse().ok())
        .unwrap_or(0))
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
        }
    }
}
