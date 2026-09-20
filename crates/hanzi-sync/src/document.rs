//! The shards that get *rewritten*: the vocabulary list and the course cursor.
//!
//! [`crate::shard`] holds the attempt log, and the difference between the two is
//! the whole reason they are separate files. An attempt is appended and never
//! changes, so its shard is immutable once closed and the merge is a plain union.
//! An entry is **edited and deleted**, so its shard is a document that a device
//! rewrites every time it syncs.
//!
//! ## What that costs, and what it does not
//!
//! Rewriting gives up the immutability the attempt log has, and it is worth being
//! exact about what that property was for. It was never needed to make the *merge*
//! correct — a union of append-only records is correct because records never
//! change, and a last-writer-wins merge is correct for the opposite reason: every
//! record carries the stamp that settles it. It was needed to make a **dumb
//! transport** safe, and that safety survives, because every writer still owns its
//! own directory. Two devices never write one file; a device rewriting its own
//! `vocab.json` cannot conflict with anything, and Dropbox uploads are atomic, so a
//! reader sees the old document or the new one and never half of either.
//!
//! What is genuinely lost: a shard is no longer a permanent record of what a device
//! once said. That is fine here and would not be fine for attempts, which is why
//! the two are not the same file and not the same format.
//!
//! ## Why last-writer-wins is the right answer for an entry
//!
//! Because there is no better one. Two devices editing the same sentence while
//! apart is a genuine disagreement, not a mergeable one, and a three-way merge of
//! prose would be a guess dressed up as an algorithm. So one of them wins, whole:
//! the later stamp, and the writing device id when two stamps fall in the same
//! second. What matters is that both devices agree on *which* one won, which they
//! do, because the comparison is over values the winner wrote rather than anything
//! either of them computed.

use std::collections::BTreeMap;

use hanzi_store::{SyncedCursor, SyncedEntry, SyncedGroup};

use crate::shard::device_of_shard;
use crate::store::{RemoteStore, SyncError};

/// The vocabulary document's format version.
///
/// Refused rather than half-understood when a peer writes a newer one, for the same
/// reason the study database refuses a newer schema: the fields this build does not
/// know are the ones that would be silently dropped.
const VOCAB_VERSION: u32 = 1;
/// The cursor document's format version.
const CURSOR_VERSION: u32 = 1;

/// The file a device's vocabulary view lives in.
const VOCAB_FILE: &str = "vocab.json";
/// The file a device's course position lives in.
const CURSOR_FILE: &str = "cursor.json";

/// The name of a device's vocabulary shard.
pub fn vocab_shard_name(device_id: &str) -> String {
    format!("devices/{device_id}/{VOCAB_FILE}")
}

/// The name of a device's cursor shard.
pub fn cursor_shard_name(device_id: &str) -> String {
    format!("devices/{device_id}/{CURSOR_FILE}")
}

/// The device a shard belongs to, when `name` is exactly `devices/<id>/<file>`.
///
/// Exact, rather than a prefix match: a store may hold other things, and a file
/// called `vocab.json.backup` in the right folder is not a shard this build wrote.
fn owner_of<'a>(name: &'a str, file: &str) -> Option<&'a str> {
    let device = device_of_shard(name)?;
    (name.len() == "devices/".len() + device.len() + 1 + file.len()
        && name.ends_with(file)
        && name.as_bytes()["devices/".len() + device.len()] == b'/')
        .then_some(device)
}

/// What travels for the vocabulary list.
#[derive(serde::Serialize, serde::Deserialize)]
struct VocabShard {
    version: u32,
    entries: Vec<SyncedEntry>,
    groups: Vec<SyncedGroup>,
}

/// What travels for the course cursor.
#[derive(serde::Serialize, serde::Deserialize)]
struct CursorShard {
    version: u32,
    cursor: SyncedCursor,
}

/// The stamp that settles who wrote last: the time, the device, and which of that
/// device's writes it was.
///
/// All three parts earn their place. The time is the ordinary answer. The device
/// settles two devices writing inside one second. And the revision settles two
/// writes by *one* device inside one second — which is not a hypothetical: adding an
/// entry and deleting it again takes well under a second, and without the third part
/// a peer holding the first of those writes has nothing to compare against, keeps
/// the entry, and stays different from the other device for ever.
///
/// It has to be a *total* order both devices compute the same way, or they would
/// disagree about the winner and diverge silently.
fn stamp<'a>(updated_at: &'a str, device_id: &'a str, revision: i64) -> (&'a str, &'a str, i64) {
    (updated_at, device_id, revision)
}

/// Read every device's vocabulary view.
///
/// A shard that cannot be parsed is an error rather than a skip, and a newer format
/// version is refused rather than half-read: dropping the fields this build does not
/// know would be silent data loss on the other device's behalf.
pub fn read_vocab(
    store: &dyn RemoteStore,
) -> Result<(Vec<SyncedEntry>, Vec<SyncedGroup>), SyncError> {
    let mut entries = Vec::new();
    let mut groups = Vec::new();
    for entry in store.list()? {
        if owner_of(&entry.name, VOCAB_FILE).is_none() {
            continue;
        }
        let bytes = store.get(&entry.name)?;
        let shard: VocabShard = serde_json::from_slice(&bytes).map_err(|e| {
            SyncError::Malformed(format!("{} could not be read: {e}", entry.name))
        })?;
        if shard.version > VOCAB_VERSION {
            return Err(SyncError::Malformed(format!(
                "{} was written by a newer version of the app (format {}, this build \
                 understands {VOCAB_VERSION})",
                entry.name, shard.version
            )));
        }
        entries.extend(shard.entries);
        groups.extend(shard.groups);
    }
    Ok((entries, groups))
}

/// Publish this device's whole vocabulary view.
///
/// The whole view every time, tombstones included, because that is what makes the
/// merge a function of the shards rather than of their history. It does mean a sync
/// re-uploads a document that may be identical to the last one — a few kilobytes,
/// and the alternative is a change-tracking scheme whose bugs would be far more
/// expensive than the bytes.
pub fn write_vocab(
    store: &dyn RemoteStore,
    device_id: &str,
    entries: &[SyncedEntry],
    groups: &[SyncedGroup],
) -> Result<String, SyncError> {
    let shard = VocabShard {
        version: VOCAB_VERSION,
        entries: entries.to_vec(),
        groups: groups.to_vec(),
    };
    let encoded = serde_json::to_vec(&shard).map_err(|e| {
        SyncError::Malformed(format!("the vocabulary list could not be encoded: {e}"))
    })?;
    let name = vocab_shard_name(device_id);
    store.put(&name, &encoded)?;
    Ok(name)
}

/// Read every device's course position.
pub fn read_cursor(store: &dyn RemoteStore) -> Result<Vec<SyncedCursor>, SyncError> {
    let mut found = Vec::new();
    for entry in store.list()? {
        if owner_of(&entry.name, CURSOR_FILE).is_none() {
            continue;
        }
        let bytes = store.get(&entry.name)?;
        let shard: CursorShard = serde_json::from_slice(&bytes).map_err(|e| {
            SyncError::Malformed(format!("{} could not be read: {e}", entry.name))
        })?;
        if shard.version > CURSOR_VERSION {
            return Err(SyncError::Malformed(format!(
                "{} was written by a newer version of the app (format {}, this build \
                 understands {CURSOR_VERSION})",
                entry.name, shard.version
            )));
        }
        found.push(shard.cursor);
    }
    Ok(found)
}

/// Publish this device's course position.
pub fn write_cursor(
    store: &dyn RemoteStore,
    device_id: &str,
    cursor: &SyncedCursor,
) -> Result<String, SyncError> {
    let shard = CursorShard {
        version: CURSOR_VERSION,
        cursor: cursor.clone(),
    };
    let encoded = serde_json::to_vec(&shard).map_err(|e| {
        SyncError::Malformed(format!("the course position could not be encoded: {e}"))
    })?;
    let name = cursor_shard_name(device_id);
    store.put(&name, &encoded)?;
    Ok(name)
}

/// Settle one entry against another: the later stamp wins, and the device id only
/// matters when two devices wrote inside the same second.
fn later_than(candidate: &SyncedEntry, winner: &SyncedEntry) -> bool {
    stamp(
        &candidate.updated_at,
        &candidate.device_id,
        candidate.revision,
    ) > stamp(&winner.updated_at, &winner.device_id, winner.revision)
}

/// Merge two vocabulary views, last writer winning per entry and per group.
///
/// No error case, and that is the difference from the attempt log. Two shards
/// disagreeing about `(device_id, seq)` means one was rewritten, which the format
/// forbids and the merge refuses. Two shards disagreeing about an entry is the
/// ordinary case this whole format exists for, so something has to win and the
/// stamps say which.
pub fn merge_vocab(
    local_entries: Vec<SyncedEntry>,
    local_groups: Vec<SyncedGroup>,
    remote_entries: Vec<SyncedEntry>,
    remote_groups: Vec<SyncedGroup>,
) -> (Vec<SyncedEntry>, Vec<SyncedGroup>) {
    let mut entries: BTreeMap<String, SyncedEntry> = BTreeMap::new();
    for entry in local_entries.into_iter().chain(remote_entries) {
        let uuid = entry.uuid.clone();
        match entries.get(&uuid) {
            Some(winner) if !later_than(&entry, winner) => {}
            _ => {
                entries.insert(uuid, entry);
            }
        }
    }

    let mut groups: BTreeMap<String, SyncedGroup> = BTreeMap::new();
    for group in local_groups.into_iter().chain(remote_groups) {
        let name = group.name.clone();
        match groups.get(&name) {
            Some(winner)
                if stamp(&group.updated_at, &group.device_id, group.revision)
                    <= stamp(&winner.updated_at, &winner.device_id, winner.revision) => {}
            _ => {
                groups.insert(name, group);
            }
        }
    }

    (
        entries.into_values().collect(),
        groups.into_values().collect(),
    )
}

/// The course position that won, or `None` when no device has published one.
pub fn merge_cursor(local: Option<SyncedCursor>, remote: Vec<SyncedCursor>) -> Option<SyncedCursor> {
    local
        .into_iter()
        .chain(remote)
        .reduce(|winner, candidate| {
            if stamp(
                &candidate.updated_at,
                &candidate.device_id,
                candidate.revision,
            ) > stamp(&winner.updated_at, &winner.device_id, winner.revision)
            {
                candidate
            } else {
                winner
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(uuid: &str, meaning: &str, updated_at: &str, device_id: &str) -> SyncedEntry {
        SyncedEntry {
            uuid: uuid.to_string(),
            text: "学习".to_string(),
            pinyin: "xuéxí".to_string(),
            meaning: meaning.to_string(),
            group: None,
            added_at: "2026-09-19T09:00:00Z".to_string(),
            updated_at: updated_at.to_string(),
            device_id: device_id.to_string(),
            revision: 0,
            deleted: false,
        }
    }

    #[test]
    fn a_shard_name_says_which_device_owns_it_and_what_it_is() {
        assert_eq!(vocab_shard_name("abc"), "devices/abc/vocab.json");
        assert_eq!(cursor_shard_name("abc"), "devices/abc/cursor.json");
        assert_eq!(owner_of("devices/abc/vocab.json", VOCAB_FILE), Some("abc"));
        assert_eq!(owner_of("devices/abc/cursor.json", CURSOR_FILE), Some("abc"));

        // Exactly, or a store holding other things would be read as shards.
        assert_eq!(owner_of("devices/abc/vocab.json.bak", VOCAB_FILE), None);
        assert_eq!(owner_of("devices/abc/attempts/000001.jsonl", VOCAB_FILE), None);
        assert_eq!(owner_of("devices/abc/notes/vocab.json", VOCAB_FILE), None);
        assert_eq!(owner_of("vocab.json", VOCAB_FILE), None);
        assert_eq!(owner_of("devices//vocab.json", VOCAB_FILE), None);
    }

    #[test]
    fn the_later_stamp_wins_and_the_device_settles_a_tie() {
        let older = entry("u1", "first", "2026-09-20T09:00:00Z", "phone");
        let newer = entry("u1", "second", "2026-09-20T10:00:00Z", "laptop");

        // Either order, same answer: that is what makes the merge safe to run on
        // however many devices in whatever order they happen to sync.
        let (forwards, _) = merge_vocab(vec![older.clone()], vec![], vec![newer.clone()], vec![]);
        let (backwards, _) = merge_vocab(vec![newer.clone()], vec![], vec![older.clone()], vec![]);
        assert_eq!(forwards, backwards);
        assert_eq!(forwards[0].meaning, "second");

        // Same second: the device id decides, and it decides the same way both ways.
        let from_a = entry("u1", "on a", "2026-09-20T10:00:00Z", "aaa");
        let from_b = entry("u1", "on b", "2026-09-20T10:00:00Z", "bbb");
        let (merged, _) = merge_vocab(vec![from_a.clone()], vec![], vec![from_b.clone()], vec![]);
        assert_eq!(merged[0].meaning, "on b", "the higher device id wins the tie");
        let (other_way, _) = merge_vocab(vec![from_b], vec![], vec![from_a], vec![]);
        assert_eq!(other_way[0].meaning, "on b");
    }

    #[test]
    fn a_deletion_wins_over_an_older_copy_of_the_same_entry() {
        // The case a tombstone exists for. Without it the peer's older copy would be
        // the only record left and the entry would come back.
        let alive = entry("u1", "to study", "2026-09-20T09:00:00Z", "phone");
        let mut removed = entry("u1", "to study", "2026-09-20T10:00:00Z", "laptop");
        removed.deleted = true;

        let (merged, _) = merge_vocab(vec![alive], vec![], vec![removed], vec![]);
        assert_eq!(merged.len(), 1, "the record survives so the removal can travel");
        assert!(merged[0].deleted, "and it says the entry is gone");
    }

    #[test]
    fn merging_the_same_view_twice_changes_nothing() {
        let view = vec![entry("u1", "one", "2026-09-20T09:00:00Z", "phone")];
        let (once, _) = merge_vocab(view.clone(), vec![], view.clone(), vec![]);
        let (twice, _) = merge_vocab(once.clone(), vec![], view, vec![]);
        assert_eq!(once, twice, "a retried sync must not keep changing its mind");
    }

    #[test]
    fn the_cursor_goes_to_whoever_moved_it_last() {
        let phone = SyncedCursor {
            position: 40,
            updated_at: "2026-09-20T09:00:00Z".to_string(),
            device_id: "phone".to_string(),
            revision: 0,
        };
        let laptop = SyncedCursor {
            position: 90,
            updated_at: "2026-09-20T18:00:00Z".to_string(),
            device_id: "laptop".to_string(),
            revision: 0,
        };

        let merged = merge_cursor(Some(phone.clone()), vec![laptop.clone()]).unwrap();
        assert_eq!(merged.position, 90);
        let other_way = merge_cursor(Some(laptop), vec![phone]).unwrap();
        assert_eq!(other_way.position, 90, "and in either order");

        assert_eq!(merge_cursor(None, vec![]), None, "nobody has moved it");
        let only = SyncedCursor {
            position: 7,
            updated_at: String::new(),
            device_id: String::new(),
            revision: 0,
        };
        assert_eq!(merge_cursor(None, vec![only.clone()]).unwrap().position, 7);
    }
}
