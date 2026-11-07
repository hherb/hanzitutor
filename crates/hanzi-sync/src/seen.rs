//! What this device already knows about the store, so that a sync downloads only
//! what changed.
//!
//! ## What is kept
//!
//! For every attempt shard a sync has **folded** — read, merged into the local log
//! without a disagreement — its name, the revision it was listed with, and the
//! sequence numbers it held. Not its bytes: every attempt it held is in the local
//! log already, because that is what "folded" means, so the log is the copy.
//!
//! The next sync that lists the same name at the same revision rebuilds the
//! shard's text from the log instead of downloading it, and hands the readers that
//! text exactly as if it had been fetched. Nothing downstream of
//! [`crate::skipped::Screened`] knows the difference, so the merge, the repair and
//! the checks of this device's identity see the same attempts either way.
//!
//! ## Why it cannot make a sync fold something stale
//!
//! - **A revision names one version of a file.** Dropbox's `rev` changes with
//!   every write; a directory's size and modification time do too, and an attempt
//!   shard only ever grows (see [`crate::shard`]), so its size alone changes with
//!   every append. A changed revision is downloaded and checked in full, exactly
//!   as before this existed — which is what still catches a shard whose lines
//!   were changed ([`crate::SyncError::Rewritten`]). An empty revision is never
//!   taken to match.
//! - **It is recorded only after the merge succeeded**, so a shard that
//!   disagreed with the log is never recorded and is downloaded and refused again
//!   on every sync, as before.
//! - **The log is checked, not trusted.** A shard is rebuilt only when the log
//!   still holds every one of the recorded attempts under the shard's device;
//!   one missing — a database restored from a backup, a renamed identity — and
//!   the shard is downloaded instead.
//! - **A file that could not be read is never recorded**, so it is fetched again
//!   on every sync, and a file that has been mended comes back on the first sync
//!   after.
//!
//! ## What is not kept
//!
//! The vocabulary list, the course position, the group positions and the
//! baselines: a device writes those whole on every sync, so their revisions
//! change about as often as they are read, and there are at most four per device
//! however long it has been syncing. It is the attempt shards whose number grows.
//!
//! The record lives in `meta`, as one small JSON document, beside the watermark.
//! It is advisory: one that cannot be read is treated as empty, which costs one
//! sync's worth of downloads and nothing else.

use std::collections::{BTreeMap, HashMap};

use hanzi_store::Db;
use serde::{Deserialize, Serialize};

use crate::shard::{parse_attempts_shard, shard_body, sort_canonical, MergedAttempt};
use crate::store::SyncError;

/// Where the record is kept, in the database's `meta` table.
const SEEN_KEY: &str = "sync:seen_shards";
/// The record's format.
///
/// A rebuilt shard is the local log run back through the attempt line's encoder,
/// so a record is only good for the line format it was made under: it is stored
/// with that format ([`shard::line_format`](crate::shard::line_format), which
/// follows `ShardLine`'s field names by itself) and dropped when it differs. A
/// change to the line that its field names do not show must bump `LINE_VERSION`
/// in `shard.rs`; a change to this record's own shape must bump this.
const SEEN_VERSION: u32 = 1;

/// The record, as stored.
#[derive(Default, Serialize, Deserialize)]
struct Record {
    version: u32,
    /// The attempt line format the shards were folded under.
    #[serde(default)]
    line_format: String,
    shards: BTreeMap<String, Folded>,
}

/// One folded shard: the revision it was folded at and the attempts it held.
#[derive(Clone, Serialize, Deserialize)]
struct Folded {
    rev: String,
    /// Sequence numbers as inclusive ranges, because a shard's are nearly always
    /// one run.
    seqs: Vec<(i64, i64)>,
}

/// What a sync already knows, and the log to rebuild from.
pub(crate) struct Seen {
    shards: BTreeMap<String, Folded>,
    log: HashMap<(String, i64), MergedAttempt>,
}

impl Seen {
    /// Knowing nothing, so that every shard is downloaded.
    pub(crate) fn nothing() -> Self {
        Self {
            shards: BTreeMap::new(),
            log: HashMap::new(),
        }
    }

    /// What this database recorded, and its whole log.
    pub(crate) fn load(db: &Db) -> Result<Self, SyncError> {
        let shards = db
            .meta_value(SEEN_KEY)
            .map_err(SyncError::Io)?
            .and_then(|text| serde_json::from_str::<Record>(&text).ok())
            .filter(|record| {
                record.version == SEEN_VERSION && record.line_format == crate::shard::line_format()
            })
            .map(|record| record.shards)
            .unwrap_or_default();
        if shards.is_empty() {
            return Ok(Self::nothing());
        }
        let log = crate::local::full_log(db)?
            .into_iter()
            .map(|attempt| ((attempt.device_id.clone(), attempt.seq), attempt))
            .collect();
        Ok(Self { shards, log })
    }

    /// The text of a shard folded at exactly this revision, rebuilt from the log —
    /// or `None`, and the shard is downloaded.
    pub(crate) fn rebuild(&self, name: &str, revision: &str) -> Option<Vec<u8>> {
        if revision.is_empty() {
            return None;
        }
        let folded = self.shards.get(name).filter(|f| f.rev == revision)?;
        let (device_id, _) = parse_attempts_shard(name)?;
        let mut attempts = Vec::new();
        for &(low, high) in &folded.seqs {
            for seq in low..=high {
                attempts.push(self.log.get(&(device_id.to_string(), seq))?.clone());
            }
        }
        if attempts.is_empty() {
            return None;
        }
        sort_canonical(&mut attempts);
        let refs: Vec<&MergedAttempt> = attempts.iter().collect();
        shard_body(device_id, &refs).ok().map(String::into_bytes)
    }

    /// Record the shards a sync has just folded, as `(name, revision, seqs)`, in
    /// place of everything recorded before — so a file that has gone from the
    /// store goes from the record too.
    pub(crate) fn save(
        db: &Db,
        folded: impl IntoIterator<Item = (String, String, Vec<i64>)>,
    ) -> Result<(), SyncError> {
        let shards = folded
            .into_iter()
            .filter(|(_, rev, seqs)| !rev.is_empty() && !seqs.is_empty())
            .map(|(name, rev, seqs)| {
                (
                    name,
                    Folded {
                        rev,
                        seqs: runs(seqs),
                    },
                )
            })
            .collect();
        let record = Record {
            version: SEEN_VERSION,
            line_format: crate::shard::line_format(),
            shards,
        };
        let text = serde_json::to_string(&record).map_err(|e| {
            SyncError::Malformed(format!(
                "the record of folded shards could not be encoded: {e}"
            ))
        })?;
        // Most syncs change nothing here, and need not write the same text again.
        if db.meta_value(SEEN_KEY).map_err(SyncError::Io)?.as_deref() == Some(text.as_str()) {
            return Ok(());
        }
        db.set_meta_value(SEEN_KEY, &text).map_err(SyncError::Io)
    }
}

/// Sequence numbers as inclusive runs.
fn runs(mut seqs: Vec<i64>) -> Vec<(i64, i64)> {
    seqs.sort_unstable();
    seqs.dedup();
    let mut out: Vec<(i64, i64)> = Vec::new();
    for seq in seqs {
        match out.last_mut() {
            Some((_, high)) if *high + 1 == seq => *high = seq,
            _ => out.push((seq, seq)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_numbers_are_kept_as_runs() {
        assert_eq!(runs(vec![3, 1, 2, 7, 5, 6, 6]), [(1, 3), (5, 7)]);
        assert!(runs(Vec::new()).is_empty());
    }
}
