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

/// Which half of the app a screen belongs to.
///
/// The app is two courses, not one list of screens: **kana are the on-ramp and
/// kanji are the product**, and a learner finishes the first in days and spends
/// years in the second. The interface therefore shows one of these at a time, and
/// the review queue is asked for one at a time — which is what this type is for.
/// Nothing about the *schedule* is split: the cards stay in one file, keyed by
/// character (invariant 15), and this only decides which of them a screen offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    /// The kana course, its chart, its drill, and the kana that are due.
    Kana,
    /// The character course, the radicals, the vocabulary, the passages, and the
    /// kanji and radical head forms that are due.
    Kanji,
}

impl Section {
    /// The stored name, which is the string serde writes and the one the
    /// interface sends. Pinned by a test for the same reason
    /// [`ReviewKind::name`] is.
    pub fn name(self) -> &'static str {
        match self {
            Section::Kana => "kana",
            Section::Kanji => "kanji",
        }
    }

    /// Whether a scheduled character of this kind is taught in this section.
    ///
    /// The radical head forms go with the kanji rather than with the kana: they
    /// are the character course's own table, reached from its screen, and the only
    /// thing that teaches one is the Radicals panel.
    pub fn covers(self, kind: ReviewKind) -> bool {
        match self {
            Section::Kana => kind == ReviewKind::Kana,
            Section::Kanji => matches!(kind, ReviewKind::Kanji | ReviewKind::Radical),
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
///
/// `section` narrows it to one half of the app; `None` is the whole schedule,
/// which is what a caller that is not a course screen (a test, or a summary)
/// wants.
pub fn due_items(
    store: &ProgressStore,
    kana: &KanaDataset,
    kanji: &KanjiDataset,
    now: &str,
    section: Option<Section>,
) -> Vec<DueItem> {
    store
        .due(now)
        .into_iter()
        .filter_map(|(ch, card)| due_item(ch, card, kana, kanji))
        .filter(|item| section.is_none_or(|section| section.covers(item.kind)))
        .collect()
}

/// The section's own cards, due or not: what its screen counts and, when nothing
/// is due, what it names a date from.
///
/// A card the board cannot draw is left out here exactly as it is left out of the
/// queue, so "12 scheduled" and the list behind it can never disagree.
fn section_cards<'a>(
    store: &'a ProgressStore,
    kana: &KanaDataset,
    kanji: &KanjiDataset,
    section: Option<Section>,
) -> Vec<(char, &'a CardState)> {
    store
        .cards()
        .iter()
        .filter_map(|(key, card)| {
            // Keyed by character, and `chars().next()` is how `ProgressStore`
            // itself reads the key — mirroring it keeps a hand-edited file's
            // multi-character key from inventing a card.
            let ch = key.chars().next()?;
            let kind = kind_of(ch, kana, kanji)?;
            match section {
                Some(section) if !section.covers(kind) => None,
                _ => Some((ch, card)),
            }
        })
        .collect()
}

/// One section's queue, whole: the due characters, how much of the schedule
/// stands behind them, and when the next one comes back.
#[derive(Clone, Debug)]
pub struct Queue {
    /// Due characters in the section, most overdue first, **uncapped** — a screen
    /// takes the page it wants and still says how big the backlog is.
    pub items: Vec<DueItem>,
    /// How many are due, which is the length of the uncapped queue.
    pub due: usize,
    /// How many characters the section's schedule holds at all, due or not.
    pub cards: usize,
    /// When the next character comes back, and only when nothing is due now.
    pub next_due: Option<String>,
}

/// Build one section's queue, whole, against a caller-supplied clock.
///
/// The date arithmetic is not repeated here: `ProgressStore` decides what is due
/// and this decides what the section can show, which is the same division of
/// labour the rest of this module keeps.
pub fn queue(
    store: &ProgressStore,
    kana: &KanaDataset,
    kanji: &KanjiDataset,
    now: &str,
    section: Option<Section>,
) -> Queue {
    let items = due_items(store, kana, kanji, now, section);
    let cards = section_cards(store, kana, kanji, section);
    // Nothing due: the soonest of the section's own cards is worth naming, so the
    // screen can answer "what now?" rather than only "nothing".
    let next_due = if items.is_empty() {
        cards.iter().map(|(_, card)| card.due.clone()).min()
    } else {
        None
    };
    Queue {
        due: items.len(),
        items,
        cards: cards.len(),
        next_due,
    }
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
    fn the_two_sections_partition_every_kind_of_character() {
        // The app is two courses, and this is the rule that keeps them one: every
        // kind a card can be is taught in exactly one of them, so no scheduled
        // character can be invisible on both screens and none can be claimed by
        // both.
        let kinds = [ReviewKind::Kana, ReviewKind::Kanji, ReviewKind::Radical];
        for kind in kinds {
            let claimed: Vec<Section> = [Section::Kana, Section::Kanji]
                .into_iter()
                .filter(|section| section.covers(kind))
                .collect();
            assert_eq!(claimed.len(), 1, "{kind:?} is taught in {claimed:?}");
        }
        // And the pairing is the documented one: the head forms are the character
        // course's, not the kana course's.
        assert!(Section::Kana.covers(ReviewKind::Kana));
        assert!(Section::Kanji.covers(ReviewKind::Kanji));
        assert!(Section::Kanji.covers(ReviewKind::Radical));
        assert_eq!(Section::Kana.name(), "kana");
        assert_eq!(Section::Kanji.name(), "kanji");
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
        assert!(due_items(&store, &kana, &kanji, "2026-09-19T09:00:00Z", None).is_empty());
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

        let due = due_items(&store, &kana, &kanji, "2026-09-19T09:30:00Z", None);
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
        let due = due_items(&store, &kana, &kanji, "2026-09-19T09:30:00Z", None);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].ch, 'あ');
    }

    #[test]
    fn a_kanji_and_a_radical_come_back_with_their_own_prompts() {
        let (kana, kanji) = datasets();
        let mut store = ProgressStore::in_memory();
        score(&mut store, '学', 20.0, "2026-09-19T09:00:00Z");
        score(&mut store, '亅', 20.0, "2026-09-19T09:00:00Z");

        let due = due_items(&store, &kana, &kanji, "2026-09-19T09:30:00Z", None);
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

    // ---- one section's queue ----------------------------------------------

    /// A schedule with one due character of each kind, and one that is not due.
    ///
    /// Written through `record_at` rather than by hand so the fixture is the same
    /// scheduler a learner's file came from.
    fn three_kinds(at: &str) -> ProgressStore {
        let mut store = ProgressStore::in_memory();
        score(&mut store, 'あ', 20.0, at);
        score(&mut store, '学', 20.0, at);
        score(&mut store, '亅', 20.0, at);
        store
    }

    #[test]
    fn a_section_offers_only_what_it_teaches() {
        let (kana, kanji) = datasets();
        let store = three_kinds("2026-09-19T09:00:00Z");
        let now = "2026-09-19T09:30:00Z";

        let kana_half = queue(&store, &kana, &kanji, now, Some(Section::Kana));
        assert_eq!(
            kana_half.items.iter().map(|item| item.ch).collect::<Vec<_>>(),
            vec!['あ'],
            "the kana course is not shown a kanji or a head form"
        );
        assert_eq!(kana_half.cards, 1, "and its own count is its own");
        assert_eq!(kana_half.due, 1);

        let kanji_half = queue(&store, &kana, &kanji, now, Some(Section::Kanji));
        let mut kanji_chars: Vec<char> = kanji_half.items.iter().map(|item| item.ch).collect();
        // Both were recorded at the same moment, so they tie on the due string and
        // the character breaks the tie; the set is what this asserts, not the code
        // point order `ProgressStore` chose.
        kanji_chars.sort();
        assert_eq!(
            kanji_chars,
            vec!['亅', '学'],
            "the character course carries the head forms with it"
        );
        assert_eq!(kanji_half.cards, 2);
        assert_eq!(kanji_half.due, 2);

        // And the two halves add up to the whole schedule, which is the property
        // that makes the split a division rather than a filter that loses cards.
        let whole = queue(&store, &kana, &kanji, now, None);
        assert_eq!(whole.cards, kana_half.cards + kanji_half.cards);
        assert_eq!(whole.due, kana_half.due + kanji_half.due);
    }

    #[test]
    fn a_section_with_nothing_due_names_its_own_next_date() {
        let (kana, kanji) = datasets();
        // Both are taught well (due tomorrow), so neither is due now.
        let mut store = ProgressStore::in_memory();
        score(&mut store, 'あ', 90.0, "2026-09-19T09:00:00Z");
        score(&mut store, '学', 90.0, "2026-09-19T09:00:00Z");
        let now = "2026-09-19T09:30:00Z";

        let kana_half = queue(&store, &kana, &kanji, now, Some(Section::Kana));
        assert!(kana_half.items.is_empty());
        assert_eq!(kana_half.due, 0);
        assert_eq!(kana_half.cards, 1, "the kana card is still the kana's");
        assert!(
            kana_half.next_due.is_some(),
            "the kana screen can still answer when あ comes back"
        );

        // A section with no cards at all has no date to name, rather than the
        // other section's.
        let mut only_kana = ProgressStore::in_memory();
        score(&mut only_kana, 'あ', 90.0, "2026-09-19T09:00:00Z");
        let empty = queue(&only_kana, &kana, &kanji, now, Some(Section::Kanji));
        assert_eq!(empty.cards, 0);
        assert!(empty.next_due.is_none(), "{:?}", empty.next_due);
    }

    #[test]
    fn a_section_does_not_count_a_card_it_cannot_show() {
        let (kana, kanji) = datasets();
        let mut store = ProgressStore::in_memory();
        // 鳩 resolves to nothing, so neither section can offer it and neither
        // counts it — which is what keeps "N scheduled" honest.
        score(&mut store, '鳩', 90.0, "2026-09-19T09:00:00Z");
        score(&mut store, 'あ', 90.0, "2026-09-19T09:00:00Z");
        let now = "2026-09-19T09:30:00Z";

        assert_eq!(queue(&store, &kana, &kanji, now, None).cards, 1);
        assert_eq!(
            queue(&store, &kana, &kanji, now, Some(Section::Kana)).cards,
            1
        );
        assert_eq!(
            queue(&store, &kana, &kanji, now, Some(Section::Kanji)).cards,
            0
        );
    }
}
