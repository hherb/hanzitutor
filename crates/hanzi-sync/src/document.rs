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

use hanzi_core::progress::CardState;
use hanzi_store::{SyncedCursor, SyncedEntry, SyncedGroup, SyncedVocabCursor};

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
/// The vocabulary-position document's format version.
const VOCAB_CURSOR_VERSION: u32 = 1;
/// The baseline document's format version.
const BASELINE_VERSION: u32 = 1;

/// The file a device's vocabulary view lives in.
const VOCAB_FILE: &str = "vocab.json";
/// The file a device's course position lives in.
const CURSOR_FILE: &str = "cursor.json";
/// The file a device's per-group vocabulary positions live in.
const VOCAB_CURSOR_FILE: &str = "vocab-cursor.json";
/// The file a device's baseline lives in.
const BASELINE_FILE: &str = "baseline.json";

/// The name of a device's vocabulary shard.
pub fn vocab_shard_name(device_id: &str) -> String {
    format!("devices/{device_id}/{VOCAB_FILE}")
}

/// The name of a device's cursor shard.
pub fn cursor_shard_name(device_id: &str) -> String {
    format!("devices/{device_id}/{CURSOR_FILE}")
}

/// The name of a device's vocabulary-position shard.
pub fn vocab_cursor_shard_name(device_id: &str) -> String {
    format!("devices/{device_id}/{VOCAB_CURSOR_FILE}")
}

/// The name of a device's baseline shard.
pub fn baseline_shard_name(device_id: &str) -> String {
    format!("devices/{device_id}/{BASELINE_FILE}")
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

/// What travels for a learner's own groups' positions.
#[derive(serde::Serialize, serde::Deserialize)]
struct VocabCursorShard {
    version: u32,
    cursors: Vec<SyncedVocabCursor>,
}

/// Read every device's vocabulary positions.
pub fn read_vocab_cursors(
    store: &dyn RemoteStore,
) -> Result<Vec<SyncedVocabCursor>, SyncError> {
    let mut found = Vec::new();
    for entry in store.list()? {
        if owner_of(&entry.name, VOCAB_CURSOR_FILE).is_none() {
            continue;
        }
        let bytes = store.get(&entry.name)?;
        let shard: VocabCursorShard = serde_json::from_slice(&bytes).map_err(|e| {
            SyncError::Malformed(format!("{} could not be read: {e}", entry.name))
        })?;
        if shard.version > VOCAB_CURSOR_VERSION {
            return Err(SyncError::Malformed(format!(
                "{} was written by a newer version of the app (format {}, this build \
                 understands {VOCAB_CURSOR_VERSION})",
                entry.name, shard.version
            )));
        }
        found.extend(shard.cursors);
    }
    Ok(found)
}

/// Publish this device's vocabulary positions, whole.
///
/// Every group the learner has a place in, the same document-every-time rule as
/// the list itself. A device with no positions publishes nothing, which is what
/// absence has to mean here: the format has no way to say "forget every group",
/// and inventing one would let a device that had merely not synced yet wipe a
/// peer's places.
pub fn write_vocab_cursors(
    store: &dyn RemoteStore,
    device_id: &str,
    cursors: &[SyncedVocabCursor],
) -> Result<String, SyncError> {
    let shard = VocabCursorShard {
        version: VOCAB_CURSOR_VERSION,
        cursors: cursors.to_vec(),
    };
    let encoded = serde_json::to_vec(&shard).map_err(|e| {
        SyncError::Malformed(format!("the vocabulary positions could not be encoded: {e}"))
    })?;
    let name = vocab_cursor_shard_name(device_id);
    store.put(&name, &encoded)?;
    Ok(name)
}

/// Merge two devices' vocabulary positions, last writer winning **per group**.
///
/// The per-group part is the whole of it. A position is only meaningful against
/// the list it was taken in, so comparing one group's stamp with another's would
/// let a device that happened to drill a different lesson later drag a group it
/// never opened forward, or backwards. Each group is settled alone.
pub fn merge_vocab_cursors(
    local: Vec<SyncedVocabCursor>,
    remote: Vec<SyncedVocabCursor>,
) -> Vec<SyncedVocabCursor> {
    let mut by_group: BTreeMap<String, SyncedVocabCursor> = BTreeMap::new();
    for cursor in local.into_iter().chain(remote) {
        let group = cursor.group_name.clone();
        let wins = match by_group.get(&group) {
            Some(winner) => {
                stamp(&cursor.updated_at, &cursor.device_id, cursor.revision)
                    > stamp(&winner.updated_at, &winner.device_id, winner.revision)
            }
            None => true,
        };
        if wins {
            by_group.insert(group, cursor);
        }
    }
    by_group.into_values().collect()
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

// ---- the baseline ----------------------------------------------------------
//
// A card whose log does not go back to its first attempt cannot be rebuilt from the
// log, and folding the part of it that survives would produce a schedule from a
// fraction of what happened. The baseline is what makes such a card foldable again:
// the state the lost history left behind, published so that every device folds the
// same card from the same starting point.
//
// It is a document rather than a log: a device captures one the first time it syncs
// after this existed — see `crate::local` — and what it captured never changes, because
// it describes history that has already happened. It is re-uploaded on every sync
// anyway, for the same reason the vocabulary document is: a shard that went missing
// comes back by itself, and tracking whether this one changed would cost more than the
// bytes do.

/// One device's baseline.
///
/// It belongs to exactly one device, and its `device_id` is not decoration: the row
/// range below is in *that* device's `seq` numbering, which no other device shares.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Baseline {
    /// The device that captured it.
    pub device_id: String,
    /// This device's log rows with `seq` in `[from, to)` are already inside every
    /// card here, so folding them again would count them twice.
    ///
    /// A range rather than a list of attempts, because it is *every* row this device
    /// had written when the baseline was captured — its imported history and whatever
    /// it practised afterwards — and those are contiguous: `seq` is handed out one
    /// device at a time, in the order things happened, starting at zero.
    #[serde(default)]
    pub from: i64,
    #[serde(default)]
    pub to: i64,
    /// The card states, by character.
    #[serde(default)]
    pub cards: BTreeMap<String, CardState>,
}

/// What travels for the baseline.
#[derive(serde::Serialize, serde::Deserialize)]
struct BaselineShard {
    version: u32,
    device_id: String,
    from: i64,
    to: i64,
    cards: BTreeMap<String, CardState>,
}

/// Read every device's baseline.
pub fn read_baselines(store: &dyn RemoteStore) -> Result<Vec<Baseline>, SyncError> {
    let mut found = Vec::new();
    for entry in store.list()? {
        if owner_of(&entry.name, BASELINE_FILE).is_none() {
            continue;
        }
        let bytes = store.get(&entry.name)?;
        let shard: BaselineShard = serde_json::from_slice(&bytes).map_err(|e| {
            SyncError::Malformed(format!("{} could not be read: {e}", entry.name))
        })?;
        if shard.version > BASELINE_VERSION {
            return Err(SyncError::Malformed(format!(
                "{} was written by a newer version of the app (format {}, this build \
                 understands {BASELINE_VERSION})",
                entry.name, shard.version
            )));
        }
        // The directory a baseline sits in and the device it claims to be are
        // checked against each other, which no other shard here needs. The reason is
        // the row range: it says "these rows of device X are already inside these
        // cards", so a file that claimed another device's name could suppress that
        // device's attempts on every peer — a whole character's history quietly
        // folded into nothing. A shard can only speak for the device whose directory
        // it is in.
        let owner = owner_of(&entry.name, BASELINE_FILE).unwrap_or_default();
        if owner != shard.device_id {
            return Err(SyncError::Malformed(format!(
                "{} claims to be {}'s baseline, and it is in {}'s directory",
                entry.name, shard.device_id, owner
            )));
        }
        found.push(Baseline {
            device_id: shard.device_id,
            from: shard.from,
            to: shard.to,
            cards: shard.cards,
        });
    }
    Ok(found)
}

/// Publish a device's baseline.
///
/// Only ever called with a baseline that has something in it: a device with no
/// incomplete cards has nothing to say, and an empty document would be a file every
/// peer downloads to learn nothing.
pub fn write_baseline(store: &dyn RemoteStore, baseline: &Baseline) -> Result<String, SyncError> {
    let shard = BaselineShard {
        version: BASELINE_VERSION,
        device_id: baseline.device_id.clone(),
        from: baseline.from,
        to: baseline.to,
        cards: baseline.cards.clone(),
    };
    let encoded = serde_json::to_vec(&shard).map_err(|e| {
        SyncError::Malformed(format!("the baseline could not be encoded: {e}"))
    })?;
    let name = baseline_shard_name(&baseline.device_id);
    store.put(&name, &encoded)?;
    Ok(name)
}

/// Every baseline a fold has to know about, resolved into the two questions it asks.
///
/// The fold asks one thing of one character: *where do I start, and which of these
/// rows am I already past?* Answering that is the whole of this type, and it exists
/// because the answer has to be the same on every device — so the winner is chosen
/// by a rule over values the baselines themselves carry, never by anything a device
/// computed or by the order they arrived in.
#[derive(Clone, Debug, Default)]
pub struct Baselines {
    /// The winning state per character, with the device it came from.
    winners: BTreeMap<String, (String, CardState)>,
    /// Per character, the device ranges whose rows that character's state already
    /// accounts for.
    ///
    /// Keyed by character, and that is load-bearing rather than tidy. A baseline's
    /// range is a stretch of one device's log, and it says "the cards named here
    /// already hold these rows" — nothing at all about the rows the same device
    /// wrote for characters it did *not* name. A device with an incomplete history
    /// for one character and a complete one for another is the ordinary case, and
    /// treating the range as global would silently drop the second character's
    /// attempts and leave its card at whatever it was.
    covered: BTreeMap<String, Vec<(String, i64, i64)>>,
}

impl Baselines {
    /// Resolve the baselines of every device into one view.
    ///
    /// Two devices with a baseline for the same character is the degenerate case: it
    /// means both migrated from a JSON file that had got out of step, and the two
    /// states are not two halves of one history but two rival accounts of it. **The
    /// one that remembers more attempts wins**, which is the rule ROADMAP M13
    /// prescribes; the device id settles a tie, so that every device picks the same
    /// one. Nothing here can tell which account is *right*, and pretending otherwise
    /// would be inventing history.
    pub fn resolve(devices: Vec<Baseline>) -> Self {
        let mut winners: BTreeMap<String, (String, CardState)> = BTreeMap::new();
        let mut covered: BTreeMap<String, Vec<(String, i64, i64)>> = BTreeMap::new();
        for baseline in devices {
            for (ch, card) in baseline.cards {
                covered.entry(ch.clone()).or_default().push((
                    baseline.device_id.clone(),
                    baseline.from,
                    baseline.to,
                ));
                let challenger = (baseline.device_id.clone(), card);
                match winners.get(&ch) {
                    Some(held) if !beats(&challenger, held) => {}
                    _ => {
                        winners.insert(ch, challenger);
                    }
                }
            }
        }
        Self { winners, covered }
    }

    /// The state to fold this character from, if any baseline has one.
    pub fn winner(&self, ch: &str) -> Option<&CardState> {
        self.winners.get(ch).map(|(_, card)| card)
    }

    /// Every character a baseline has a state for, with the device it came from.
    ///
    /// The fold needs this as well as [`Self::winner`], because a character can be in
    /// a baseline and have no row in the log at all — a migrated card whose stored
    /// history was empty — and it still has a schedule that every device should
    /// agree on.
    pub fn winners(&self) -> impl Iterator<Item = (&String, &(String, CardState))> {
        self.winners.iter()
    }

    /// Whether one row is already accounted for by the baseline for its character.
    ///
    /// Deliberately asked of *every* baseline naming that character and not only the
    /// winning one. A losing baseline's rows were part of the account that was
    /// discarded, and there is no reading of two rivals under which keeping them is
    /// right — they describe the same stretch of one character's history, from the
    /// other side, and applying them on top of the winner would count that stretch
    /// twice.
    pub fn covers(&self, ch: &str, device_id: &str, seq: i64) -> bool {
        self.covered
            .get(ch)
            .is_some_and(|ranges| {
                ranges
                    .iter()
                    .any(|(device, from, to)| device == device_id && seq >= *from && seq < *to)
            })
    }

    /// Whether anything at all is known here, which is the common case: false for a
    /// learner whose log has always been complete, and false for every device that
    /// has never imported a JSON file.
    pub fn is_empty(&self) -> bool {
        self.winners.is_empty()
    }
}

/// Whether one candidate baseline should win over the one held.
///
/// More attempts first, then the **lower** device id. Which way the tiebreak runs is
/// arbitrary; that it *is* one is not. A rule that could tie would let two devices
/// pick different winners out of the same shards and diverge with nothing to notice
/// it by, so the comparison has to be a total order over what the baselines
/// themselves carry — never the order they arrived in, and never anything a device
/// computed.
fn beats(candidate: &(String, CardState), held: &(String, CardState)) -> bool {
    let (candidate_attempts, held_attempts) = (candidate.1.attempts, held.1.attempts);
    match candidate_attempts.cmp(&held_attempts) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => candidate.0 < held.0,
    }
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

    /// A card state with just enough filled in to tell two of them apart.
    fn baseline_card(attempts: u32, ease: f32) -> CardState {
        CardState {
            attempts,
            lapses: 0,
            best_score: None,
            last_score: None,
            last_practised: None,
            due: "2026-09-19T09:00:00Z".to_string(),
            interval_days: 0.0,
            ease,
            repetitions: 0,
            history: Vec::new(),
        }
    }

    fn baseline(device: &str, from: i64, to: i64, cards: &[(&str, u32, f32)]) -> Baseline {
        Baseline {
            device_id: device.to_string(),
            from,
            to,
            cards: cards
                .iter()
                .map(|(ch, attempts, ease)| (ch.to_string(), baseline_card(*attempts, *ease)))
                .collect(),
        }
    }

    #[test]
    fn two_baselines_for_one_character_are_settled_by_the_one_that_remembers_more() {
        // The degenerate case: two devices migrated from JSON files that had got out
        // of step, so the two states are rival accounts rather than two halves of one
        // history. Nothing here can tell which is right, so the rule is the one
        // ROADMAP M13 prescribes — and the *device id* settles a tie, because a rule
        // that could tie would let two devices choose differently and diverge with
        // nothing to notice it by.
        let phone = baseline("phone", 0, 3, &[("好", 5, 2.5)]);
        let laptop = baseline("laptop", 0, 9, &[("好", 9, 2.1)]);

        let resolved = Baselines::resolve(vec![phone.clone(), laptop.clone()]);
        assert_eq!(resolved.winner("好").unwrap().attempts, 9, "more attempts wins");
        let other_way = Baselines::resolve(vec![laptop, phone]);
        assert_eq!(
            other_way.winner("好").unwrap().ease,
            resolved.winner("好").unwrap().ease,
            "and the same one wins whichever order the shards arrived in"
        );

        // A tie goes to the lower device id, and to the same one either way round.
        let a = baseline("aaa", 0, 1, &[("好", 4, 2.0)]);
        let b = baseline("bbb", 0, 1, &[("好", 4, 2.9)]);
        assert_eq!(Baselines::resolve(vec![b.clone(), a.clone()]).winner("好").unwrap().ease, 2.0);
        assert_eq!(Baselines::resolve(vec![a, b]).winner("好").unwrap().ease, 2.0);
    }

    #[test]
    fn a_baseline_speaks_only_for_the_characters_it_names() {
        // The bug this scoping exists for. A baseline's range is a stretch of one
        // device's log, and it claims the cards it names already hold those rows — it
        // says nothing about the characters the same device wrote in the same stretch.
        // Read as a global range it would drop them, and their cards would be left at
        // whatever they were, silently.
        let device = baseline("laptop", 0, 3, &[("好", 5, 2.5)]);
        let resolved = Baselines::resolve(vec![device]);

        assert!(resolved.covers("好", "laptop", 1), "the named character is covered");
        assert!(!resolved.covers("好", "laptop", 3), "but only inside the range");
        assert!(!resolved.covers("好", "phone", 1), "and only its own rows");
        assert!(
            !resolved.covers("猫", "laptop", 1),
            "a character the baseline does not name keeps its rows"
        );
        assert!(resolved.winner("猫").is_none(), "and has no state to fold from");
    }

    /// The store the shard-reading tests use: a name to bytes.
    struct MemoryStore(BTreeMap<String, Vec<u8>>);

    impl RemoteStore for MemoryStore {
        fn list(&self) -> Result<Vec<crate::RemoteEntry>, SyncError> {
            Ok(self
                .0
                .iter()
                .map(|(name, bytes)| crate::RemoteEntry {
                    name: name.clone(),
                    revision: format!("{}", bytes.len()),
                })
                .collect())
        }
        fn get(&self, name: &str) -> Result<Vec<u8>, SyncError> {
            self.0
                .get(name)
                .cloned()
                .ok_or_else(|| SyncError::Io(format!("{name} is not in the store")))
        }
        fn put(&self, _name: &str, _bytes: &[u8]) -> Result<(), SyncError> {
            unreachable!("the reading tests never write")
        }
    }

    #[test]
    fn a_baseline_in_the_wrong_directory_is_refused() {
        // The check that makes the row range safe to trust. A file claiming another
        // device's name could suppress that device's attempts everywhere it travelled,
        // which is a whole character's history folded into nothing.
        let text = serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "device_id": "laptop",
            "from": 0,
            "to": 9,
            "cards": {},
        }))
        .unwrap();
        let store = MemoryStore(BTreeMap::from([(
            baseline_shard_name("phone"),
            text,
        )]));
        let refusal = read_baselines(&store).unwrap_err();
        assert!(
            refusal.to_string().contains("directory"),
            "the refusal should say what is wrong: {refusal}"
        );
    }

    #[test]
    fn a_baseline_with_nothing_in_it_covers_nothing() {
        // A capture that found no incomplete cards still writes itself down, so the
        // scan happens once rather than on every sync. It must then be inert: an
        // empty document that suppressed a whole seq range would break every
        // character on the device that captured it.
        let empty = baseline("laptop", 0, 500, &[]);
        let resolved = Baselines::resolve(vec![empty]);
        assert!(resolved.is_empty());
        assert!(!resolved.covers("好", "laptop", 1));
        assert!(!resolved.covers("好", "laptop", 499));
    }
}
