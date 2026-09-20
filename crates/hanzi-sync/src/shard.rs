//! The shard format, the merge, and the fold.
//!
//! ## The format
//!
//! One file per chunk of a device's attempts:
//!
//! ```text
//! devices/<device-id>/attempts/<first-seq:012>.jsonl
//! ```
//!
//! One JSON object per line, in sequence order. The device's identity is in the
//! path rather than in every line, because the path is what a store can list
//! without fetching — and because every writer owning its own directory is what
//! makes a dumb transport safe.
//!
//! A shard is named by the sequence number it *starts* at, so a closed shard can
//! never be confused with a rewritten one, and writing the same attempts twice
//! produces the same names and the same bytes. There is no content hash in the
//! name: a hash would make the name depend on the contents, which would mean a
//! device that retried a partial write created a second shard instead of
//! finishing the first.
//!
//! ## The merge
//!
//! Attempts are identified by `(device_id, seq)` — the pair, never the number
//! alone, since every device numbers its own attempts from 1. Merging is a union
//! of two lists, then a sort into [`canonical`] order. There is nothing to resolve:
//! an attempt that both sides have is the *same* attempt, and if the two copies
//! disagree then a shard was rewritten, which is the one thing this format
//! promises cannot happen — so that is reported rather than papered over.
//!
//! ## The fold
//!
//! Finally the merged log is handed to the engine, one character at a time, in
//! canonical order, and `hanzi_core::fold_attempts` rebuilds each card. Sync never
//! merges a *schedule*; it merges logs and recomputes schedules, which is why two
//! devices that practised apart converge on one answer instead of one of them
//! winning.

use std::collections::BTreeMap;

use hanzi_core::{fold_attempts, Attempt, CardState, Rating, Sm2};
use serde::{Deserialize, Serialize};

use crate::store::{RemoteStore, SyncError};

/// How many attempts go into one shard file.
///
/// A shard is closed when it reaches this, and never reopened. The number trades
/// a directory listing's worth of files against re-uploading a large one: an
/// attempt log of a few thousand entries becomes a handful of small files, and a
/// device that has been offline for a month uploads one new shard rather than the
/// whole log.
pub const CHUNK: usize = 500;

/// The directory every device's shards live under.
const DEVICES: &str = "devices";
/// The subdirectory attempts live in, under a device.
const ATTEMPTS: &str = "attempts";

/// One attempt as it travels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ShardLine {
    seq: i64,
    ch: String,
    at: String,
    score: f32,
    rating: Rating,
}

/// An attempt with the device that made it attached.
#[derive(Clone, Debug, PartialEq)]
pub struct MergedAttempt {
    /// The device that made the attempt.
    pub device_id: String,
    /// Its number in that device's own log, from 1.
    pub seq: i64,
    pub ch: String,
    pub at: String,
    pub score: f32,
    pub rating: Rating,
}

/// The name of the shard holding a device's attempts from `first_seq` on.
pub fn attempts_shard_name(device_id: &str, first_seq: i64) -> String {
    format!("{DEVICES}/{device_id}/{ATTEMPTS}/{first_seq:012}.jsonl")
}

/// The device a shard belongs to, for any shard under `devices/`.
///
/// Deliberately general: the vocabulary and cursor shards will sit beside the
/// attempt shards under the same device directory.
pub fn device_of_shard(name: &str) -> Option<&str> {
    let rest = name.strip_prefix(DEVICES)?.strip_prefix('/')?;
    let (device, _) = rest.split_once('/')?;
    (!device.is_empty()).then_some(device)
}

/// The device and first sequence of an attempt shard, if that is what `name` is.
///
/// Anything that is not exactly this shape is not an attempt shard, and callers
/// skip it: a store may hold other files, and a vocabulary shard beside these must
/// not be parsed as one.
pub fn parse_attempts_shard(name: &str) -> Option<(&str, i64)> {
    let rest = name.strip_prefix(DEVICES)?.strip_prefix('/')?;
    let (device, rest) = rest.split_once('/')?;
    let rest = rest.strip_prefix(ATTEMPTS)?.strip_prefix('/')?;
    let first: i64 = rest.strip_suffix(".jsonl")?.parse().ok()?;
    (!device.is_empty()).then_some((device, first))
}

/// The order attempts are folded in, and the order they are written in.
///
/// When it happened, then which device, then which of that device's attempts.
/// `at` is only accurate to the second, so ties are real and need a tiebreak that
/// every device computes identically: SM-2's ease factor accumulates in `f32`, so
/// two devices folding the same log in different orders would drift apart with no
/// way to notice. Neither `at` alone nor insertion order can do this job.
fn canonical(a: &MergedAttempt, b: &MergedAttempt) -> std::cmp::Ordering {
    (&a.at, &a.device_id, a.seq).cmp(&(&b.at, &b.device_id, b.seq))
}

/// Read every attempt every device has published.
///
/// Files that are not attempt shards are ignored, so a store can hold other
/// things. A shard that cannot be parsed is an error rather than a skip: a
/// truncated shard parses as a *shorter log*, which would silently lose attempts,
/// so it must not be mistaken for a valid one.
pub fn read_attempts(store: &dyn RemoteStore) -> Result<Vec<MergedAttempt>, SyncError> {
    let mut out = Vec::new();
    for entry in store.list()? {
        let Some((device_id, _)) = parse_attempts_shard(&entry.name) else {
            continue;
        };
        let bytes = store.get(&entry.name)?;
        let text = String::from_utf8(bytes)
            .map_err(|e| SyncError::Malformed(format!("{}: not UTF-8: {e}", entry.name)))?;
        for (number, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let record: ShardLine = serde_json::from_str(line).map_err(|e| {
                SyncError::Malformed(format!("{} line {}: {e}", entry.name, number + 1))
            })?;
            out.push(MergedAttempt {
                device_id: device_id.to_string(),
                seq: record.seq,
                ch: record.ch,
                at: record.at,
                score: record.score,
                rating: record.rating,
            });
        }
    }
    out.sort_by(canonical);
    Ok(out)
}

/// Write one device's attempts as shards, chunked, and return the names written.
///
/// Every attempt must belong to `device_id`: a device publishing another's
/// attempts under its own name would break the identity the whole merge rests on,
/// so it is refused rather than silently relabelled.
pub fn write_attempts(
    store: &dyn RemoteStore,
    device_id: &str,
    attempts: &[MergedAttempt],
) -> Result<Vec<String>, SyncError> {
    let mut ordered: Vec<&MergedAttempt> = attempts.iter().collect();
    ordered.sort_by(|a, b| canonical(a, b));

    let mut names = Vec::new();
    for chunk in ordered.chunks(CHUNK) {
        let first = chunk
            .first()
            .expect("a chunk of a non-empty slice is non-empty")
            .seq;
        let mut body = String::new();
        for attempt in chunk {
            if attempt.device_id != device_id {
                return Err(SyncError::Malformed(format!(
                    "an attempt of {} cannot be published as {device_id}'s",
                    attempt.device_id
                )));
            }
            let line = ShardLine {
                seq: attempt.seq,
                ch: attempt.ch.clone(),
                at: attempt.at.clone(),
                score: attempt.score,
                rating: attempt.rating,
            };
            let encoded = serde_json::to_string(&line)
                .map_err(|e| SyncError::Malformed(format!("an attempt could not be encoded: {e}")))?;
            body.push_str(&encoded);
            body.push('\n');
        }
        let name = attempts_shard_name(device_id, first);
        store.put(&name, body.as_bytes())?;
        names.push(name);
    }
    Ok(names)
}

/// Union two logs into one, in canonical order.
///
/// The two sides are the local database's log and everything read back from the
/// store. An attempt on both sides is one attempt; the same `(device_id, seq)`
/// with different contents is [`SyncError::Rewritten`], because a shard is written
/// once and never rewritten and a merge must not choose between two stories.
pub fn merge_attempts(
    local: Vec<MergedAttempt>,
    remote: Vec<MergedAttempt>,
) -> Result<Vec<MergedAttempt>, SyncError> {
    let mut by_origin: BTreeMap<(String, i64), MergedAttempt> = BTreeMap::new();
    for attempt in local.into_iter().chain(remote) {
        let key = (attempt.device_id.clone(), attempt.seq);
        match by_origin.get(&key) {
            Some(existing) if *existing != attempt => {
                return Err(SyncError::Rewritten {
                    device_id: attempt.device_id,
                    seq: attempt.seq,
                })
            }
            Some(_) => {}
            None => {
                by_origin.insert(key, attempt);
            }
        }
    }
    let mut out: Vec<MergedAttempt> = by_origin.into_values().collect();
    out.sort_by(canonical);
    Ok(out)
}

/// Rebuild every character's schedule from a merged log.
///
/// The log is sorted here rather than trusted, so that a caller which has just
/// concatenated two sources still gets one answer. Each character's attempts are
/// gathered in that same order and handed to the engine's fold, which is the only
/// thing that decides what a schedule is.
pub fn fold_cards(attempts: &[MergedAttempt]) -> BTreeMap<String, CardState> {
    let mut ordered: Vec<&MergedAttempt> = attempts.iter().collect();
    ordered.sort_by(|a, b| canonical(a, b));

    let mut by_character: BTreeMap<&str, Vec<Attempt>> = BTreeMap::new();
    for attempt in ordered {
        by_character
            .entry(attempt.ch.as_str())
            .or_default()
            .push(Attempt {
                at: attempt.at.clone(),
                score: attempt.score,
                rating: attempt.rating,
            });
    }

    by_character
        .into_iter()
        .filter_map(|(ch, log)| {
            // `fold_attempts` returns `None` for an empty log, which cannot
            // happen here: a character is only in the map because it has one.
            fold_attempts(&log, &Sm2).map(|card| (ch.to_string(), card))
        })
        .collect()
}
