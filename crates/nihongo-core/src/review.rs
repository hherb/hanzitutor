//! The review queue: what the board has taught this learner, and what is due.
//!
//! The schedule itself is not here. It is `hanzi_core::progress` — SM-2, the
//! due-date arithmetic and the attempt log — because counting strokes, pairing
//! them and deciding when to ask again are the same problem whatever the script.
//! What *is* here is the Japanese half: which of the things the board can draw a
//! card can be about, and what to prompt a learner with when it comes back.
//!
//! ## The unit is a character
//!
//! A card is keyed by the character, exactly as `hanzi_core::progress` keys it,
//! because that is what the board grades: one kana, one jōyō kanji, or one of the
//! 214 radical head forms. The vocabulary is not scheduled here. A word has its
//! own reading and is checked by typing rather than by writing, so it needs its
//! own attempt source and its own decision; this module deliberately does not
//! pretend the two are the same question.
//!
//! ## A card can only be about something the board can draw
//!
//! [`kind_of`] resolves a character in the same order [`crate::kanji`] and the
//! app's `grade` command do — kana, then jōyō kanji, then a radical head form —
//! and returns `None` for anything else. A card whose character resolves to
//! nothing is skipped rather than offered: a review screen that hands the learner
//! something the board cannot write is the dead end invariant 13 forbids.

use serde::{Deserialize, Serialize};

use hanzi_core::progress::{CardState, ProgressStore};

use crate::kana::KanaDataset;
use crate::kanji::KanjiDataset;
use crate::readings::reading;

/// What kind of thing a scheduled character is.
///
/// The order is the board's own lookup order, and it matters: 手 is a kana
/// dataset miss, a jōyō kanji and a radical head form all at once, and it is the
/// kanji that a screen should call it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewKind {
    Kana,
    Kanji,
    /// One of the 214 head forms, and not a jōyō character — 92 of them are not.
    Radical,
}

impl ReviewKind {
    /// The stored name, which is the string serde writes and the one the
    /// interface reads. A test pins the two together so a renamed variant cannot
    /// leave the two sides disagreeing about what a row means.
    pub fn name(self) -> &'static str {
        match self {
            ReviewKind::Kana => "kana",
            ReviewKind::Kanji => "kanji",
            ReviewKind::Radical => "radical",
        }
    }
}

/// One due character, as the review screen offers it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueItem {
    pub ch: char,
    pub kind: ReviewKind,
    /// The prompt beside the character: a kana's reading, a kanji's first gloss,
    /// or the radical's number. Never empty — see [`hint_for`].
    pub hint: String,
    /// The Kangxi number, when this is a head form that is *not* a jōyō character.
    /// It is the only way to ask for that geometry, because the character course
    /// cannot reach it.
    pub radical: Option<u8>,
    /// When it came due, ISO-8601 UTC. The queue is ordered by this.
    pub due: String,
    pub interval_days: f32,
    pub attempts: u32,
    pub lapses: u32,
}

/// What a character is, in the order the board resolves it.
///
/// `None` means the board cannot draw it, so there is nothing to review and no
/// geometry to grade against.
pub fn kind_of(ch: char, kana: &KanaDataset, kanji: &KanjiDataset) -> Option<ReviewKind> {
    if kana.get(ch).is_some() {
        return Some(ReviewKind::Kana);
    }
    if kanji.get(ch).is_some() {
        return Some(ReviewKind::Kanji);
    }
    if kanji.radicals().iter().any(|radical| radical.ch == ch) {
        return Some(ReviewKind::Radical);
    }
    None
}

/// The Kangxi number of the radical head form `ch`, when the table holds it.
fn radical_number(ch: char, kanji: &KanjiDataset) -> Option<u8> {
    kanji
        .radicals()
        .iter()
        .find(|radical| radical.ch == ch)
        .map(|radical| radical.number)
}

/// The prompt shown beside a due character.
///
/// It is never empty: an empty prompt reads as a rendering bug rather than as a
/// character with nothing to say, so each kind falls back through its own fields
/// and finally to the character's code point.
///
/// A kana's prompt comes from the readings rather than from the dataset, because
/// a reading is what a learner is being asked to produce and the two are already
/// one table — the same reason `input.rs` derives its spellings from it.
fn hint_for(ch: char, kind: ReviewKind, kanji: &KanjiDataset) -> String {
    match kind {
        ReviewKind::Kana => match reading(ch) {
            Some(read) if read.is_silent() => "no sound of its own".to_string(),
            Some(read) => read
                .hepburn
                .first()
                .map(|sound| (*sound).to_string())
                .unwrap_or_else(|| fallback(ch)),
            None => fallback(ch),
        },
        ReviewKind::Kanji => {
            let kanji = kanji.get(ch);
            kanji
                .and_then(|k| k.meanings.first().cloned())
                .or_else(|| kanji.and_then(|k| k.on.first().cloned()))
                .or_else(|| kanji.and_then(|k| k.kun.first().cloned()))
                .unwrap_or_else(|| fallback(ch))
        }
        ReviewKind::Radical => match radical_number(ch, kanji) {
            Some(number) => format!("radical {number} of 214"),
            None => fallback(ch),
        },
    }
}

/// The last-resort prompt, which names the character rather than showing nothing.
fn fallback(ch: char) -> String {
    format!("U+{:04X}", ch as u32)
}

/// Build the review queue.
///
/// Everything due at `now`, most overdue first — the order `ProgressStore::due`
/// already returns — with each character resolved to what the board will draw.
/// A due card whose character the board cannot resolve is skipped, and the count
/// returned beside the queue is the number of items *offered*, not the number of
/// cards on file.
pub fn due_items(
    store: &ProgressStore,
    kana: &KanaDataset,
    kanji: &KanjiDataset,
    now: &str,
) -> Vec<DueItem> {
    store
        .due(now)
        .into_iter()
        .filter_map(|(ch, card)| due_item(ch, card, kana, kanji))
        .collect()
}

/// One due card as an item, or `None` when the board cannot draw its character.
fn due_item(
    ch: char,
    card: &CardState,
    kana: &KanaDataset,
    kanji: &KanjiDataset,
) -> Option<DueItem> {
    let kind = kind_of(ch, kana, kanji)?;
    Some(DueItem {
        ch,
        kind,
        hint: hint_for(ch, kind, kanji),
        radical: match kind {
            ReviewKind::Radical => radical_number(ch, kanji),
            _ => None,
        },
        due: card.due.clone(),
        interval_days: card.interval_days,
        attempts: card.attempts,
        lapses: card.lapses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use crate::kana::KanaDataset;
    use crate::kanji::KanjiDataset;

    /// The committed artifacts, read from the crate's own `data/` directory.
    ///
    /// A test in the crate that owns the artifacts should read them the way a
    /// consumer does, and this is that: no fixture, no hand-built geometry.
    fn datasets() -> (KanaDataset, KanjiDataset) {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
        let kana = std::fs::read(dir.join("kana.bin.gz")).expect("the committed kana artifact");
        let kanji = std::fs::read(dir.join("kanji.bin.gz")).expect("the committed kanji artifact");
        (
            KanaDataset::from_gzip_bytes(&kana).expect("the kana artifact decodes"),
            KanjiDataset::from_gzip_bytes(&kanji).expect("the kanji artifact decodes"),
        )
    }

    #[test]
    fn a_character_resolves_the_way_the_board_resolves_it() {
        let (kana, kanji) = datasets();
        assert_eq!(kind_of('あ', &kana, &kanji), Some(ReviewKind::Kana));
        // 手 is all three at once; the board looks the kana up first, then the
        // kanji, then the radical, and this has to agree or two screens would
        // disagree about what the same character is.
        assert_eq!(kind_of('手', &kana, &kanji), Some(ReviewKind::Kanji));
        // 亅 is a radical head form and not a jōyō character.
        assert_eq!(kind_of('亅', &kana, &kanji), Some(ReviewKind::Radical));
        // And something the board cannot draw has no answer at all.
        assert_eq!(kind_of('鳩', &kana, &kanji), None);
    }

    #[test]
    fn the_kind_names_are_the_ones_the_interface_reads() {
        // The strings themselves are pinned where they cross the boundary —
        // `tests/ipc_contract.rs` in the app — because that is the half a rename
        // actually breaks. This is the other half: `name()` and the serde tag are
        // one decision, so `ReviewKind::Kana` can never be `"hiragana"` here and
        // `"kana"` there.
        assert_eq!(ReviewKind::Kana.name(), "kana");
        assert_eq!(ReviewKind::Kanji.name(), "kanji");
        assert_eq!(ReviewKind::Radical.name(), "radical");
    }

    #[test]
    fn a_prompt_is_what_the_learner_needs_and_never_empty() {
        let (_, kanji) = datasets();
        assert_eq!(hint_for('あ', ReviewKind::Kana, &kanji), "a");
        // っ has no sound of its own, and saying so is the prompt.
        assert_eq!(
            hint_for('っ', ReviewKind::Kana, &kanji),
            "no sound of its own"
        );
        assert!(!hint_for('学', ReviewKind::Kanji, &kanji).is_empty());
        assert_eq!(
            hint_for('亅', ReviewKind::Radical, &kanji),
            "radical 6 of 214"
        );
    }

    // ---- the queue --------------------------------------------------------

    fn score(store: &mut ProgressStore, ch: char, value: f32, at: &str) {
        store.record_at(ch, value, at).expect("records");
    }

    #[test]
    fn an_empty_schedule_has_an_empty_queue() {
        let (kana, kanji) = datasets();
        let store = ProgressStore::in_memory();
        assert!(due_items(&store, &kana, &kanji, "2026-09-19T09:00:00Z").is_empty());
    }

    #[test]
    fn only_what_is_due_is_offered_most_overdue_first() {
        let (kana, kanji) = datasets();
        let mut store = ProgressStore::in_memory();
        // Two characters that failed (due within the minute) and one that went
        // well (due tomorrow), recorded at the same moment.
        score(&mut store, 'あ', 20.0, "2026-09-19T09:00:00Z");
        score(&mut store, 'い', 20.0, "2026-09-19T09:05:00Z");
        score(&mut store, 'う', 90.0, "2026-09-19T09:00:00Z");

        let due = due_items(&store, &kana, &kanji, "2026-09-19T09:30:00Z");
        assert_eq!(
            due.iter().map(|item| item.ch).collect::<Vec<_>>(),
            vec!['あ', 'い'],
            "う is not due until tomorrow"
        );
        assert!(due.iter().all(|item| item.kind == ReviewKind::Kana));
        assert_eq!(due[0].attempts, 1);
        assert_eq!(due[0].lapses, 1);
        assert!(due[0].interval_days < 1.0);
    }

    #[test]
    fn a_card_the_board_cannot_draw_is_skipped_rather_than_offered() {
        let (kana, kanji) = datasets();
        let mut store = ProgressStore::in_memory();
        // A hand-edited file is allowed to hold anything. 鳩 is not a jōyō
        // character, so there is no geometry to put on the board and no card to
        // offer the learner.
        score(&mut store, '鳩', 20.0, "2026-09-19T09:00:00Z");
        score(&mut store, 'あ', 20.0, "2026-09-19T09:00:00Z");
        let due = due_items(&store, &kana, &kanji, "2026-09-19T09:30:00Z");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].ch, 'あ');
    }

    #[test]
    fn a_kanji_and_a_radical_come_back_with_their_own_prompts() {
        let (kana, kanji) = datasets();
        let mut store = ProgressStore::in_memory();
        score(&mut store, '学', 20.0, "2026-09-19T09:00:00Z");
        score(&mut store, '亅', 20.0, "2026-09-19T09:00:00Z");

        let due = due_items(&store, &kana, &kanji, "2026-09-19T09:30:00Z");
        let by_ch: Vec<(char, ReviewKind, Option<u8>)> = due
            .iter()
            .map(|item| (item.ch, item.kind, item.radical))
            .collect();
        assert!(by_ch.contains(&('学', ReviewKind::Kanji, None)));
        assert!(
            by_ch.contains(&('亅', ReviewKind::Radical, Some(6))),
            "a radical the character course cannot reach still says which one it is: {by_ch:?}"
        );
    }
}
