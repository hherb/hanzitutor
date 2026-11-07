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
//! A shard is named by the sequence number it *starts* at, and writing the same
//! attempts twice produces the same names and the same bytes. There is no content
//! hash in the name: a hash would make the name depend on the contents, which would
//! mean a device that retried a partial write created a second shard instead of
//! finishing the first.
//!
//! ## A shard only grows
//!
//! A device's **last** shard is open: each publish adds its new attempts to the end
//! of it until it holds [`CHUNK`], and only then starts a new file. (Starting a new
//! file every publish, as earlier builds did, left a phone that syncs on every
//! launch with hundreds of one-line shards a year, each one more download per
//! sync.) The rule every writer keeps, and every reader relies on, is:
//!
//! - **A new version of a shard is the old one's bytes, unchanged, followed by
//!   more lines** — this device's own attempts, numbered above every attempt the
//!   shard already holds, never more than [`CHUNK`] lines in all. Nothing is reordered, edited or taken
//!   out. Only the device whose directory it is appends, and only to its last
//!   shard, and only when every attempt already in it is one its own log holds
//!   identically ([`open_tail`]). A last shard holding an attempt that log does
//!   not is another install's under the same name, and the publish refuses
//!   ([`SyncError::SharedIdentity`]) rather than append or start a conflicting
//!   shard.
//! - **A reader treats a shard as a set of attempts, whatever version it reads.**
//!   A longer version of a shard it has already folded adds the attempts it had
//!   not seen; a version whose attempt `(device, seq)` differs from the one
//!   already held is [`SyncError::Rewritten`], exactly as two shards disagreeing
//!   always was. So "rewritten" is about an attempt, never about a file having
//!   changed. A changed revision is what tells a reader to look again (see
//!   [`crate::seen`]).
//! - **The one other write to an existing attempt shard is a repair** of this
//!   device's own *unreadable* one (`repair_own_shards`), which writes every
//!   attempt the file can have held from this device's log. Nobody can have folded
//!   the unreadable version, so it contradicts no reader; the attempts it writes
//!   are the ones every reader already holds, so it contradicts no earlier
//!   readable version either.
//!
//! Earlier builds read a shard the same way — every shard, every sync, merged
//! attempt by attempt — so a grown shard is, to them, a shard with more attempts
//! in it.
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

use hanzi_core::{fold_attempts, fold_from, Attempt, CardState, Rating, Sm2};

use crate::document::Baselines;
use serde::{Deserialize, Serialize};

use crate::store::{RemoteStore, SyncError};

/// How many attempts go into one shard file.
///
/// A shard is closed when it reaches this, and never reopened; until then it is
/// this device's open shard, and each publish adds to it. The number trades a
/// directory listing's worth of files against re-uploading a large one: an attempt
/// log of a few thousand entries becomes a handful of files, and the most a publish
/// re-uploads is one shard of this size.
pub const CHUNK: usize = 500;

/// The directory every device's shards live under.
const DEVICES: &str = "devices";
/// The subdirectory attempts live in, under a device.
const ATTEMPTS: &str = "attempts";

/// One attempt as it travels.
///
/// **Changing this changes what a cached shard rebuilds to.** [`crate::seen`]
/// does not keep a shard's bytes: it rebuilds a folded shard's text from the local
/// log through this type, so a field added here that the log does not carry would
/// be silently missing from every rebuilt shard. The record of folded shards is
/// keyed on [`line_format`], which follows the field names automatically; any
/// other change to the line — a field's meaning or encoding, a new rating — must
/// bump [`LINE_VERSION`] so that every cached shard is downloaded again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ShardLine {
    seq: i64,
    ch: String,
    at: String,
    score: f32,
    rating: Rating,
}

/// The attempt line's format version: bump it with any change to [`ShardLine`]
/// that its field names do not show.
const LINE_VERSION: u32 = 1;

/// The attempt line's format, as [`crate::seen`] keys its record on: the version
/// and the field names, read off an encoded line so that adding or renaming a
/// field changes it without anybody having to remember to.
pub(crate) fn line_format() -> String {
    let sample = ShardLine {
        seq: 1,
        ch: String::new(),
        at: String::new(),
        score: 0.0,
        rating: Rating::Good,
    };
    // Sorted here rather than trusted to be: whether a JSON object keeps its
    // insertion order depends on a `serde_json` feature another crate may turn on.
    let mut fields: Vec<String> = match serde_json::to_value(&sample) {
        Ok(serde_json::Value::Object(map)) => map.keys().cloned().collect(),
        _ => Vec::new(),
    };
    fields.sort();
    let fields = fields.join(",");
    format!("{LINE_VERSION}:{fields}")
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
        out.extend(
            parse_shard(&entry.name, device_id, &bytes)
                .map_err(|fault| SyncError::Malformed(fault.reason))?,
        );
    }
    out.sort_by(canonical);
    Ok(out)
}

/// One attempt shard's lines, or what is wrong with it.
///
/// All or nothing, for the reason [`read_attempts`] gives: a shard that is
/// partly readable must not pass for a shorter one.
pub(crate) fn parse_shard(
    name: &str,
    device_id: &str,
    bytes: &[u8],
) -> Result<Vec<MergedAttempt>, ShardFault> {
    let damaged = |reason: String| ShardFault {
        newer_format: false,
        reason,
    };
    let text =
        std::str::from_utf8(bytes).map_err(|e| damaged(format!("{name}: not UTF-8: {e}")))?;
    let mut out = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record: ShardLine = serde_json::from_str(line).map_err(|e| ShardFault {
            // A line that is sound JSON but not an attempt this build can read — a
            // rating it has never heard of, a field changed shape — was written by
            // a newer build, not broken by a transport. The difference decides
            // whether this device may write the file again (it must not: it would
            // drop what the newer build said) and what the learner is told.
            newer_format: serde_json::from_str::<serde_json::Value>(line).is_ok(),
            reason: format!("{name} line {}: {e}", number + 1),
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
    Ok(out)
}

/// Why an attempt shard could not be read.
pub(crate) struct ShardFault {
    /// Sound JSON this build cannot decode, rather than a damaged file.
    pub(crate) newer_format: bool,
    pub(crate) reason: String,
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

    write_new_shards(store, device_id, &ordered)
}

/// Attempts, already in canonical order, as new shards of [`CHUNK`] each.
fn write_new_shards(
    store: &dyn RemoteStore,
    device_id: &str,
    ordered: &[&MergedAttempt],
) -> Result<Vec<String>, SyncError> {
    let mut names = Vec::new();
    for chunk in ordered.chunks(CHUNK) {
        let first = chunk
            .first()
            .expect("a chunk of a non-empty slice is non-empty")
            .seq;
        let body = shard_body(device_id, chunk)?;
        let name = attempts_shard_name(device_id, first);
        store.put(&name, body.as_bytes())?;
        names.push(name);
    }
    Ok(names)
}

/// A device's last attempt shard, as found open for appending.
pub(crate) struct OpenTail {
    name: String,
    /// The revision it was listed with, for [`RemoteStore::replace`]; empty when
    /// it is not known, and then the append is a plain write.
    revision: String,
    bytes: Vec<u8>,
    seqs: std::collections::BTreeSet<i64>,
}

impl OpenTail {
    /// The highest sequence number it holds: an append adds only attempts above it.
    fn highest(&self) -> i64 {
        self.seqs.last().copied().unwrap_or(i64::MIN)
    }
}

/// The shard a publish may append to: this device's last one, if it can take more.
///
/// `last_start` is the number this device's last attempt shard is named after,
/// counting ones that could not be read — so an unreadable last shard means no
/// append, rather than an append to the one before it, which would put new
/// attempts in a file whose successor's name says they are not there. `own` is this
/// device's own log.
///
/// It is open when it is in the listing, reads, holds fewer than [`CHUNK`]
/// attempts, and ends with a whole line; otherwise this is `None`, and the publish
/// starts a new shard, as earlier builds always did.
///
/// Whether open or full, a readable last shard must hold nothing but attempts
/// that `own` holds identically, and if it does not this is
/// [`SyncError::SharedIdentity`]. A shard holding an attempt of this name that
/// this log lacks, or holds differently, was written by another copy of this
/// database (#12) that has published past where this log ends: appending would
/// mix two histories in one file, and starting a new shard would publish
/// different attempts under numbers the other already used — `Rewritten` on every
/// device for good. Refusing is the most this can do; #12's `settle_identity`,
/// which gives the copy a name of its own, is the fix.
pub(crate) fn open_tail(
    store: &dyn RemoteStore,
    device_id: &str,
    last_start: Option<i64>,
    own: &[MergedAttempt],
) -> Result<Option<OpenTail>, SyncError> {
    let Some(last_start) = last_start else {
        return Ok(None);
    };
    let Some(entry) = store
        .list()?
        .into_iter()
        .find(|entry| parse_attempts_shard(&entry.name) == Some((device_id, last_start)))
    else {
        return Ok(None);
    };
    let bytes = store.get(&entry.name)?;
    let Ok(held) = parse_shard(&entry.name, device_id, &bytes) else {
        return Ok(None);
    };
    let mine: std::collections::HashMap<i64, &MergedAttempt> =
        own.iter().map(|attempt| (attempt.seq, attempt)).collect();
    if let Some(stranger) = held
        .iter()
        .find(|attempt| mine.get(&attempt.seq) != Some(attempt))
    {
        return Err(SyncError::SharedIdentity {
            device_id: device_id.to_string(),
            seq: stranger.seq,
        });
    }
    let open = !held.is_empty() && held.len() < CHUNK && bytes.last() == Some(&b'\n');
    Ok(open.then(|| OpenTail {
        name: entry.name,
        revision: entry.revision,
        bytes,
        seqs: held.iter().map(|attempt| attempt.seq).collect(),
    }))
}

/// Write a device's newest attempts: onto its open shard first, until that holds
/// [`CHUNK`], and the rest as new shards. Returns the names written, and how many
/// attempts were written — which leaves out any the open shard already held.
///
/// Only attempts numbered **above** everything the open shard holds are appended:
/// the rule is that a shard grows by newer attempts. Anything lower — which the
/// caller should not normally pass, since it drops whatever a readable shard
/// already holds, but which a store that lost an old shard can produce — goes into
/// new shards of its own.
///
/// The open shard is written with [`RemoteStore::replace`] against the revision
/// it was listed with, so a second writer in this directory makes this fail
/// rather than lose what it wrote. Its existing bytes are kept exactly; see the
/// module note for the rule.
pub(crate) fn append_attempts(
    store: &dyn RemoteStore,
    device_id: &str,
    attempts: &[MergedAttempt],
    tail: Option<&OpenTail>,
) -> Result<(Vec<String>, usize), SyncError> {
    let mut ordered: Vec<&MergedAttempt> = attempts
        .iter()
        .filter(|attempt| !tail.is_some_and(|tail| tail.seqs.contains(&attempt.seq)))
        .collect();
    ordered.sort_by(|a, b| canonical(a, b));
    let written = ordered.len();

    let mut names = Vec::new();
    let mut rest = ordered;
    if let Some(tail) = tail {
        let (newer, older): (Vec<&MergedAttempt>, Vec<&MergedAttempt>) = rest
            .into_iter()
            .partition(|attempt| attempt.seq > tail.highest());
        let room = CHUNK.saturating_sub(tail.seqs.len()).min(newer.len());
        if room > 0 {
            let mut bytes = tail.bytes.clone();
            bytes.extend_from_slice(shard_body(device_id, &newer[..room])?.as_bytes());
            if tail.revision.is_empty() {
                store.put(&tail.name, &bytes)?;
            } else {
                store.replace(&tail.name, &bytes, &tail.revision)?;
            }
            names.push(tail.name.clone());
        }
        // What did not fit, and anything older than the open shard, in canonical
        // order as always.
        rest = older.into_iter().chain(newer[room..].iter().copied()).collect();
        rest.sort_by(|a, b| canonical(a, b));
    }
    names.extend(write_new_shards(store, device_id, &rest)?);
    Ok((names, written))
}

/// The text of one attempt shard: one line per attempt, in the order given.
///
/// Every attempt must belong to `device_id`, for the reason [`write_attempts`]
/// gives.
///
/// `device_id` is whatever `Db::device_id` hands back, borrowed or owned.
pub(crate) fn shard_body(
    device_id: impl AsRef<str>,
    attempts: &[&MergedAttempt],
) -> Result<String, SyncError> {
    let device_id = device_id.as_ref();
    let mut body = String::new();
    for attempt in attempts {
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
    Ok(body)
}

/// Sort attempts into the order shards are written and folded in.
pub(crate) fn sort_canonical(attempts: &mut [MergedAttempt]) {
    attempts.sort_by(canonical);
}

/// Union two logs into one, in canonical order.
///
/// The two sides are the local database's log and everything read back from the
/// store. An attempt on both sides is one attempt; the same `(device_id, seq)`
/// with different contents is [`SyncError::Rewritten`], because an attempt is
/// written once and never changed — a shard only grows — and a merge must not
/// choose between two stories.
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
///
/// ## What the baselines change
///
/// For a character some device has published a baseline for, the fold starts from
/// that state instead of from an unseen card — and the rows that state already
/// accounts for are **left out**, because folding them again would count them twice.
/// Everything else is the same fold, which is the point: a device folding from a
/// baseline and a device folding from a complete log run the same code over the same
/// attempts and must land on the same card.
///
/// A character can also be *only* in a baseline, with no row in the log at all: a
/// migrated card whose stored history was empty has nothing in the log to key it by,
/// and its state is exactly what its own device holds. So the character set is the
/// union of the two, not the log's.
///
/// ## The one approximation, stated rather than hidden
///
/// A baseline carries no timestamp — it is a state, not an event — so every attempt
/// that is applied on top of one is applied *after* it, whatever the clock says. An
/// attempt a peer recorded before this device migrated is therefore folded as though it
/// came afterwards. That is the best available reading: the baseline is the only record
/// of what happened before, and it has no date to order against. It is the same
/// approximation the loser of two rival baselines gets, and it is bounded by the fact
/// that a baseline exists at all only for characters whose history predates the log.
pub fn fold_cards(attempts: &[MergedAttempt], baselines: &Baselines) -> BTreeMap<String, CardState> {
    let mut ordered: Vec<&MergedAttempt> = attempts.iter().collect();
    ordered.sort_by(|a, b| canonical(a, b));

    let mut by_character: BTreeMap<&str, Vec<&MergedAttempt>> = BTreeMap::new();
    for attempt in ordered {
        by_character.entry(attempt.ch.as_str()).or_default().push(attempt);
    }

    let mut folded = BTreeMap::new();
    for (ch, rows) in &by_character {
        let log: Vec<Attempt> = rows
            .iter()
            .filter(|row| !baselines.covers(&row.ch, &row.device_id, row.seq))
            .map(|row| Attempt {
                at: row.at.clone(),
                score: row.score,
                rating: row.rating,
            })
            .collect();
        let card = match baselines.winner(ch) {
            Some(state) => Some(fold_from(state.clone(), &log, &Sm2)),
            // No baseline, so the log is the whole story — and `fold_attempts`
            // answers `None` for an empty one, which cannot happen for a character
            // that is in this map. Everything below is a character that is in a
            // baseline and not in the log, which the loop after this one covers.
            None => fold_attempts(&log, &Sm2),
        };
        if let Some(card) = card {
            folded.insert(ch.to_string(), card);
        }
    }

    // A character with a baseline and no rows at all: nothing to fold onto it, and
    // its state is the baseline itself.
    for (ch, (_, state)) in baselines.winners() {
        folded
            .entry(ch.clone())
            .or_insert_with(|| state.clone());
    }

    folded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(seq: i64) -> MergedAttempt {
        MergedAttempt {
            device_id: "me".into(),
            seq,
            ch: "好".into(),
            at: format!("2026-09-20T09:00:{seq:02}Z"),
            score: 80.0,
            rating: Rating::Good,
        }
    }

    #[test]
    fn only_attempts_above_the_open_shard_are_appended_to_it() {
        // A store that lost an old shard can leave this device with an attempt to
        // write that is numbered below its open shard; it goes into a shard of its
        // own, so the open one only ever grows by newer attempts.
        let root = std::env::temp_dir().join(format!("hanzi-shard-tail-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        let store = crate::FolderStore::open(&root).unwrap();
        let own: Vec<MergedAttempt> = (1..=7).map(attempt).collect();
        write_attempts(&store, "me", &own[4..6]).unwrap();
        let tail = open_tail(&store, "me", Some(5), &own).unwrap().expect("open");

        let (names, written) =
            append_attempts(&store, "me", &[attempt(3), attempt(7)], Some(&tail)).unwrap();
        assert_eq!(written, 2);
        assert_eq!(
            names,
            [attempts_shard_name("me", 5), attempts_shard_name("me", 3)]
        );
        let seqs = |name: &str| -> Vec<i64> {
            parse_shard(name, "me", &store.get(name).unwrap())
                .ok()
                .unwrap()
                .into_iter()
                .map(|a| a.seq)
                .collect()
        };
        assert_eq!(seqs(&attempts_shard_name("me", 5)), [5, 6, 7]);
        assert_eq!(seqs(&attempts_shard_name("me", 3)), [3]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_line_format_follows_the_fields_of_a_line() {
        assert_eq!(line_format(), "1:at,ch,rating,score,seq");
    }
}
