//! What the discrimination drill remembers about *this* learner.
//!
//! The drill asks one question — here is a reading, which of these two spellings
//! is it — over the pairs the exercises are built from, and there are three
//! families of them: the thirteen classic **confusions** in [`crate::CONFUSABLE`],
//! the **yōon contrasts** where a digraph is set against the same sound written
//! long (きゃ against きや — one mora against two), and the **voicing contrasts**
//! where a kana is set against the same kana with a mark (か against が, は
//! against ば and ぱ). See [`DrillKind`].
//!
//! The classic pairs are practitioner consensus, so they are the same for
//! everybody; which of them *you* get wrong is not, and until this module existed
//! the drill threw that away after every answer. It is the cheapest personalised
//! thing in the app and it needs no data from anywhere.
//!
//! # The rule, written down rather than left emergent
//!
//! Every pair carries a weight, and the next pair is drawn **weighted at
//! random**:
//!
//! ```text
//! weight = max(0.25, 1 + 2 × wrong − correct)
//! ```
//!
//! * a pair nobody has been asked yet weighs **1**;
//! * every wrong answer **adds 2**, so one miss makes a pair three times as
//!   likely as an untouched one and two misses five times;
//! * every right answer **takes off 1**, so a pair the learner has learned
//!   recedes;
//! * and nothing ever falls below **0.25**, so a pair that is never missed
//!   becomes *rare rather than impossible*. That floor is the whole reason for
//!   keeping a tally instead of deleting a pair: a learner who has stopped being
//!   asked about る/ろ would never find out that they have started confusing
//!   them again.
//!
//! The arithmetic is deliberately trivial and integer-valued, which is what makes
//! it explainable to the learner it is judging ("you have missed this pair twice,
//! so it comes up more often until you stop") and testable without a scheduler.
//! It is not a spaced-repetition scheduler and should not become one: `ROADMAP.md`
//! M13 and `ROADMAP_NIHONGO.md` N2 own that question, and this is the small thing
//! that pays for itself before any of it.
//!
//! # This type is data, and it is the app's, not the engine's
//!
//! Nothing here reads or writes a file; the type is `serde`-serialisable so the
//! app can keep it where it likes. It is specifically **not** in the shared
//! `hanzi-store`: the two apps' learner data is separate by design (see
//! `HANDOVER_NIHONGO.md` invariant 15), and a shared store for two separate apps
//! is the thing that turns "separate" into "tangled".

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::curriculum::{Confusable, CONFUSABLE, ROWS};
use crate::readings::reading;
use crate::Script;

/// The weight of a pair that has never been asked.
pub const START_WEIGHT: f64 = 1.0;
/// What one wrong answer adds to a pair's weight.
pub const WRONG_WEIGHT: f64 = 2.0;
/// What one right answer takes off it.
pub const CORRECT_WEIGHT: f64 = 1.0;
/// The floor: no pair is ever worth nothing, because a pair the learner has
/// stopped missing has to come back eventually — see the module docs.
pub const MIN_WEIGHT: f64 = 0.25;

/// What one learner has done with one pair.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PairTally {
    /// How many times the pair has been asked.
    pub asked: u32,
    /// How many of those were answered correctly.
    pub correct: u32,
    /// How many were not. `correct + wrong == asked` is an invariant the
    /// recording path keeps, not something a reader of the file should assume.
    pub wrong: u32,
}

impl PairTally {
    /// This pair's share of the drill, per the rule in the module docs.
    pub fn weight(&self) -> f64 {
        (START_WEIGHT + WRONG_WEIGHT * f64::from(self.wrong)
            - CORRECT_WEIGHT * f64::from(self.correct))
        .max(MIN_WEIGHT)
    }

    /// Record one answer against this pair.
    pub fn record(&mut self, correct: bool) {
        self.asked += 1;
        if correct {
            self.correct += 1;
        } else {
            self.wrong += 1;
        }
    }
}

/// The key a pair of spellings is stored under: both spellings, lower first, so
/// that (ツ, シ) and (シ, ツ) are one entry rather than two halves of one
/// learner's mistake. `わ|ゐ`, `シ|ツ`, `きゃ|きや`, `か|が`.
///
/// A string rather than a struct because it is also the key in the JSON file, and
/// that file is meant to be readable by whoever is wondering what the app knows
/// about them.
///
/// Spelled for `&str` rather than for a kana because a drill pair's spelling is
/// not always one character: a yōon contrast is the digraph きゃ against the two
/// full-size kana きや. Ordering two strings compares them code point by code
/// point, which is what the single-kana ordering did, so a file written before
/// this existed reads back under the same keys —
/// `a_kana_pair_keeps_the_key_it_was_stored_under` is the check.
pub fn key_of(a: &str, b: &str) -> String {
    if a <= b {
        format!("{a}|{b}")
    } else {
        format!("{b}|{a}")
    }
}

/// The key a classic pair is stored under.
pub fn pair_key(pair: &Confusable) -> String {
    key_of(&pair.a.to_string(), &pair.b.to_string())
}

/// The two spellings a key names, in the order the key stores them.
///
/// `None` for anything that is not one bar between two non-empty spellings, which
/// is the shape a caller can get wrong: `"シ"`, `"シ|"`, `"|ツ"` and `"シ|ツ|"` are
/// all rejected rather than read as something.
pub fn split_key(key: &str) -> Option<(String, String)> {
    let mut parts = key.split('|');
    let (Some(a), Some(b), None) = (parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    if a.is_empty() || b.is_empty() {
        return None;
    }
    Some((a.to_string(), b.to_string()))
}

/// Which exercise a drill question belongs to.
///
/// The classic pairs are the thirteen that every kana teacher reaches for. The
/// other two are what a pair of unrelated kana cannot express: a **yōon contrast**
/// (きゃ is one mora and きや is two, and they differ only in the size of the second
/// character) and a **voicing contrast** (か and が are the same kana with a mark
/// added).
///
/// The script is part of the kind rather than a setting beside it, because a
/// prompt names one script's answer: `kya` is きゃ in hiragana and キャ in katakana,
/// and `ga` is が and not ガ. A question that mixed them would have two right
/// answers. The classic pairs are the exception — り/リ is a pair *across* the two
/// scripts, which is the point of it — so `Confusion` carries no script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DrillKind {
    /// The thirteen pairs in [`crate::CONFUSABLE`].
    Confusion,
    /// Hiragana yōon against the same sound written long: きゃ / きや.
    YoonHiragana,
    /// The same in katakana.
    YoonKatakana,
    /// Hiragana voiced against plain: か / が, は / ば, は / ぱ.
    VoicingHiragana,
    /// The same in katakana.
    VoicingKatakana,
}

impl DrillKind {
    /// Every exercise, in the order a screen should offer them.
    pub const ALL: [DrillKind; 5] = [
        DrillKind::Confusion,
        DrillKind::YoonHiragana,
        DrillKind::YoonKatakana,
        DrillKind::VoicingHiragana,
        DrillKind::VoicingKatakana,
    ];

    /// The script this exercise asks about, where it asks about one.
    pub fn script(self) -> Option<Script> {
        match self {
            DrillKind::Confusion => None,
            DrillKind::YoonHiragana | DrillKind::VoicingHiragana => Some(Script::Hiragana),
            DrillKind::YoonKatakana | DrillKind::VoicingKatakana => Some(Script::Katakana),
        }
    }

    /// The name a screen shows, and the string it crosses the IPC under.
    pub fn name(self) -> &'static str {
        match self {
            DrillKind::Confusion => "confusion",
            DrillKind::YoonHiragana => "yoon-hiragana",
            DrillKind::YoonKatakana => "yoon-katakana",
            DrillKind::VoicingHiragana => "voicing-hiragana",
            DrillKind::VoicingKatakana => "voicing-katakana",
        }
    }

    /// Every pair this exercise can ask, before any dataset is consulted.
    pub fn pairs(self) -> Vec<DrillPair> {
        match self {
            DrillKind::Confusion => confusion_pairs(),
            DrillKind::YoonHiragana => yoon_pairs(Script::Hiragana),
            DrillKind::YoonKatakana => yoon_pairs(Script::Katakana),
            DrillKind::VoicingHiragana => voicing_pairs(Script::Hiragana),
            DrillKind::VoicingKatakana => voicing_pairs(Script::Katakana),
        }
    }
}

/// One side of a drill pair: a spelling, and the reading it is asked about by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrillSide {
    /// What the learner sees and picks, e.g. `"きゃ"`.
    pub spelling: String,
    /// The reading the question prompts with, e.g. `"kya"`.
    pub prompt: String,
}

/// Two spellings a learner has to tell apart, and what tells them apart.
///
/// This is the drill's unit, and it is deliberately wider than a pair of kana:
/// a yōon contrast is two *spellings* of two characters each. Everything the
/// question needs travels with the pair — including a prompt per side, because
/// which of the two is asked for is a roll, and each has its own reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrillPair {
    pub kind: DrillKind,
    /// The two sides, in the order they are declared.
    pub sides: [DrillSide; 2],
    /// What tells them apart, in one line, for when the answer is wrong.
    pub tell: String,
}

impl DrillPair {
    /// The key this pair is recorded under — see [`key_of`].
    pub fn key(&self) -> String {
        key_of(&self.sides[0].spelling, &self.sides[1].spelling)
    }

    /// Whether `spelling` is one of the two sides.
    pub fn holds(&self, spelling: &str) -> bool {
        self.sides.iter().any(|side| side.spelling == spelling)
    }

    /// The two spellings, in the order they are declared.
    pub fn spellings(&self) -> [&str; 2] {
        [self.sides[0].spelling.as_str(), self.sides[1].spelling.as_str()]
    }

    /// Every character either spelling is written with.
    ///
    /// A caller that can only draw some characters — the app, against its own
    /// committed kana — filters on this rather than on the spelling being one
    /// character, which is what a digraph is not.
    pub fn characters(&self) -> impl Iterator<Item = char> + '_ {
        self.sides.iter().flat_map(|side| side.spelling.chars())
    }
}

/// The classic pairs as drill pairs: one kana against the one it is confused
/// with, each prompted with its own reading.
pub fn confusion_pairs() -> Vec<DrillPair> {
    CONFUSABLE
        .iter()
        .map(|pair| DrillPair {
            kind: DrillKind::Confusion,
            sides: [side_of(pair.a), side_of(pair.b)],
            tell: pair.tell.to_string(),
        })
        .collect()
}

/// The yōon contrasts of one script: each digraph against the same consonant and
/// vowel written with full-size kana, which is one mora against two.
pub fn yoon_pairs(script: Script) -> Vec<DrillPair> {
    let kind = match script {
        Script::Hiragana => DrillKind::YoonHiragana,
        Script::Katakana => DrillKind::YoonKatakana,
    };
    crate::curriculum::yoon(script)
        .into_iter()
        .map(|y| {
            let plain: String = y.plain.iter().collect();
            DrillPair {
                kind,
                sides: [
                    DrillSide { spelling: y.display.clone(), prompt: y.hepburn.clone() },
                    DrillSide { spelling: plain.clone(), prompt: y.plain_hepburn.clone() },
                ],
                tell: format!(
                    "{} is one mora — the small {}; {} is two, {} + {}",
                    y.display, y.kana[1], plain, y.plain[0], y.plain[1]
                ),
            }
        })
        .collect()
}

/// One kana as a side of a classic pair, with the reading it is prompted by.
fn side_of(ch: char) -> DrillSide {
    DrillSide {
        spelling: ch.to_string(),
        prompt: reading(ch)
            .and_then(|r| r.hepburn.first().copied())
            .unwrap_or_default()
            .to_string(),
    }
}

/// Which plain row each voiced row is the voiced form of, and the mark that makes
/// it voiced.
///
/// A relation between **rows** rather than between kana, because は has two: ば
/// carries the dakuten and ぱ the handakuten, and they are the same plain row
/// voiced twice. Nothing here is derived by arithmetic on code points — the kana
/// in a pair happen to be one or two code points apart, and
/// `the_voicing_pairs_differ_only_by_the_mark` is what checks that, rather than
/// the pairs being *built* from it.
const VOICED_FROM: &[(&str, &str, &str)] = &[
    ("ga", "ka", "dakuten ゛"),
    ("za", "sa", "dakuten ゛"),
    ("da", "ta", "dakuten ゛"),
    ("ba", "ha", "dakuten ゛"),
    ("pa", "ha", "handakuten ゜"),
];

/// The voicing contrasts of one script: か against が, は against ば and against
/// ぱ, and every other kana of a voiced row against the plain kana it is written
/// from.
///
/// This is not a confusion in the classic sense — the two shapes are near
/// identical *on purpose* — it is the mark that a beginner leaves off, and it is
/// the other half of what the course teaches and nothing drilled.
pub fn voicing_pairs(script: Script) -> Vec<DrillPair> {
    let kind = match script {
        Script::Hiragana => DrillKind::VoicingHiragana,
        Script::Katakana => DrillKind::VoicingKatakana,
    };
    let mut out = Vec::new();
    for &(voiced_sound, plain_sound, mark) in VOICED_FROM {
        let find = |sound: &str| ROWS.iter().find(|row| row.sound == sound);
        let (Some(voiced), Some(plain)) = (find(voiced_sound), find(plain_sound)) else {
            continue;
        };
        let voiced_cells = voiced.scripted_cells(script);
        let plain_cells = plain.scripted_cells(script);
        for (voiced, plain) in voiced_cells.iter().zip(&plain_cells) {
            // The rows are the same shape, so a hole in one is a hole in the
            // other; a mismatch means the grid and this table disagree.
            let (Some(voiced), Some(plain)) = (voiced, plain) else {
                continue;
            };
            out.push(DrillPair {
                kind,
                // Plain first, so the key reads か|が rather than が|か.
                sides: [side_of(*plain), side_of(*voiced)],
                // No full stop: a tell is a clause the screen puts a stop of its
                // own after, which is why the classic ones have none either. (The
                // first version of this line ended in one, and the live check
                // showed the learner `dakuten ゛..`.)
                tell: format!(
                    "{plain} is the plain kana; {voiced} is the same kana with the {mark}"
                ),
            });
        }
    }
    out
}

/// The classic pair a key names, in **either** spelling, or `None` if it is not
/// one of them.
///
/// This is the classic set only, and that is what it is for: a caller that wants
/// to know whether a key is one of the thirteen. What the app records against is
/// the wider pool it can actually ask — see [`DrillKind`] — because a yōon
/// contrast is a pair too and its key must not be refused.
///
/// `ツ|シ` finds `シ|ツ`, because the two name one pair — the canonical order is
/// what the file is *written* in, not a rule the caller has to know.
pub fn find_pair(key: &str) -> Option<&'static Confusable> {
    let (a, b) = split_key(key)?;
    CONFUSABLE
        .iter()
        .find(|pair| (pair.a.to_string() == a && pair.b.to_string() == b)
            || (pair.a.to_string() == b && pair.b.to_string() == a))
}

/// The learner's record over the pairs the drill asks about.
///
/// It holds a tally per pair, keyed by [`key_of`], and it does not care which
/// exercise a key came from: a learner who keeps writing きや for きゃ is confused
/// in the same way as one who writes ツ for シ, and both are weighted by the rule
/// in this module's docs. What is *not* shared is the file — it belongs to this
/// app alone (invariant 15).
///
/// A pair that has never been asked is simply absent from the map, so a fresh
/// log is empty and the file on disk is `{"pairs": {}}` rather than thirteen
/// entries that say nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfusionLog {
    /// Keyed by [`key_of`]. Unknown keys — pairs a later version added, or a
    /// hand-edited file — are kept as they are rather than dropped, so an older
    /// build does not silently delete a newer one's history.
    pub pairs: BTreeMap<String, PairTally>,
}

impl ConfusionLog {
    /// An empty log: every pair at its starting weight.
    pub fn new() -> Self {
        Self::default()
    }

    /// What this pair's record is. An unknown key has no record, which is the
    /// same as never having been asked.
    pub fn tally(&self, key: &str) -> PairTally {
        self.pairs.get(key).copied().unwrap_or_default()
    }

    /// This pair's weight, per the rule in the module docs.
    pub fn weight(&self, key: &str) -> f64 {
        self.tally(key).weight()
    }

    /// Record one answer against a pair, and say what the record now is.
    pub fn record(&mut self, key: &str, correct: bool) -> PairTally {
        let tally = self.pairs.entry(key.to_string()).or_default();
        tally.record(correct);
        *tally
    }

    /// Whether anything has been recorded at all.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// How many pairs have a record — that is, how many have been asked.
    pub fn pairs_seen(&self) -> usize {
        self.pairs.len()
    }

    /// Draw a pair, weighted by the rule, from `pairs`.
    ///
    /// `roll` is a number in `0..1` supplied by the caller, and that is the whole
    /// point: the arithmetic here is pure, so a test can ask for the pair at
    /// `0.0`, at `0.999` and at a boundary, and a caller that wants randomness
    /// supplies its own. Out-of-range rolls are clamped rather than trusted.
    ///
    /// Returns `None` only when there is nothing to draw from — with the floor in
    /// place, a non-empty list always has a non-zero total weight.
    pub fn pick<'a>(&self, pairs: &'a [Confusable], roll: f64) -> Option<&'a Confusable> {
        self.pick_by(pairs, pair_key, roll)
    }

    /// The same draw over anything that has a key — which is what lets one rule
    /// and one tally serve the classic pairs, the yōon contrasts and the voicing
    /// contrasts alike. The key function is the caller's because the exercises
    /// spell their pairs differently and store them in the same file.
    pub fn pick_by<'a, T>(
        &self,
        items: &'a [T],
        key: impl Fn(&T) -> String,
        roll: f64,
    ) -> Option<&'a T> {
        if items.is_empty() {
            return None;
        }
        let total: f64 = items.iter().map(|item| self.weight(&key(item))).sum();
        // The floor makes this unreachable, but a total that is zero or NaN —
        // a caller handing over a whole log of its own — must not become a
        // division by nothing.
        if !total.is_finite() || total <= 0.0 {
            return None;
        }
        let roll = if roll.is_nan() { 0.0 } else { roll.clamp(0.0, 1.0) };
        // `roll == 1.0` would land past the last item and be caught by the
        // fallback below; the clamp keeps a caller's `1.0` on the last item
        // rather than making it an accident of floating-point.
        let mut remaining = roll.min(1.0 - f64::EPSILON) * total;
        for item in items {
            let weight = self.weight(&key(item));
            if remaining < weight {
                return Some(item);
            }
            remaining -= weight;
        }
        items.last()
    }

    /// How the classic pairs' weights stand, biggest first, for a caller that
    /// wants to show the learner what they are being asked about most. Ties keep
    /// the order the pairs are declared in, so this is stable.
    ///
    /// The classic thirteen only: a yōon contrast is a pair this records and
    /// weights like any other, but it is not one of the
    /// [`crate::CONFUSABLE`] list this is a view of.
    pub fn weights(&self) -> Vec<(String, f64)> {
        let mut out: Vec<(String, f64)> = CONFUSABLE
            .iter()
            .map(|pair| (pair_key(pair), self.weight(&pair_key(pair))))
            .collect();
        out.sort_by(|a, b| b.1.total_cmp(&a.1));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair_of(a: char, b: char) -> &'static Confusable {
        CONFUSABLE
            .iter()
            .find(|p| (p.a == a && p.b == b) || (p.a == b && p.b == a))
            .expect("a known pair")
    }

    #[test]
    fn a_fresh_log_weights_every_pair_the_same() {
        let log = ConfusionLog::new();
        assert!(log.is_empty());
        assert_eq!(log.pairs_seen(), 0);
        for pair in CONFUSABLE {
            assert_eq!(log.weight(&pair_key(pair)), START_WEIGHT, "{pair:?}");
            assert_eq!(log.tally(&pair_key(pair)), PairTally::default());
        }
    }

    #[test]
    fn the_rule_is_arithmetic_not_a_mood() {
        let mut log = ConfusionLog::new();
        let key = pair_key(pair_of('る', 'ろ'));

        log.record(&key, false);
        assert_eq!(log.tally(&key), PairTally { asked: 1, correct: 0, wrong: 1 });
        assert_eq!(log.weight(&key), 3.0, "1 + 2 for one miss");

        log.record(&key, false);
        assert_eq!(log.weight(&key), 5.0, "1 + 2 + 2 for two");

        log.record(&key, true);
        assert_eq!(log.weight(&key), 4.0, "one right takes off 1");
        assert_eq!(
            log.tally(&key),
            PairTally { asked: 3, correct: 1, wrong: 2 }
        );
    }

    #[test]
    fn a_pair_that_is_never_missed_gets_rare_but_not_impossible() {
        let mut log = ConfusionLog::new();
        let key = pair_key(pair_of('シ', 'ツ'));
        for _ in 0..10 {
            log.record(&key, true);
        }
        assert_eq!(log.weight(&key), MIN_WEIGHT);
        assert!(log.weight(&key) > 0.0);
        // And it is still drawable, which is the point of the floor.
        assert!(log.pick(CONFUSABLE, 0.0).is_some());
    }

    #[test]
    fn a_pair_is_one_entry_whichever_way_round_it_is_named() {
        let forward = pair_key(pair_of('シ', 'ツ'));
        let backward = pair_key(pair_of('ツ', 'シ'));
        assert_eq!(forward, backward);
        assert_eq!(forward, "シ|ツ", "lower code point first");
        assert_eq!(find_pair(&forward).map(|p| p.tell), find_pair(&backward).map(|p| p.tell));
        assert!(find_pair("あ|い").is_none(), "and a non-pair is not one");
    }

    #[test]
    fn a_key_that_is_not_two_kana_and_a_bar_is_not_a_pair() {
        // The three shapes a caller can get wrong, and none of them is a pair:
        // one kana, three, and nothing at all.
        for key in ["シ", "シ|ツ|ク", "", "|", "シ|", "|ツ", "シ|ツ|"] {
            assert!(find_pair(key).is_none(), "{key:?} is not a pair");
        }
        // And the canonical spelling of a real pair still is one.
        assert!(find_pair("シ|ツ").is_some());
        assert!(find_pair("ツ|シ").is_some());
    }

    #[test]
    fn recording_under_one_spelling_is_found_under_the_other() {
        let mut log = ConfusionLog::new();
        log.record(&pair_key(pair_of('ツ', 'シ')), false);
        assert_eq!(log.tally(&pair_key(pair_of('シ', 'ツ'))).wrong, 1);
        assert_eq!(log.pairs_seen(), 1, "one pair, not two halves of one mistake");
    }

    #[test]
    fn the_draw_is_deterministic_in_the_roll() {
        let log = ConfusionLog::new();
        assert_eq!(log.pick(CONFUSABLE, 0.0), CONFUSABLE.first());
        let last = log.pick(CONFUSABLE, 0.999).expect("a pair");
        assert_eq!(Some(last), CONFUSABLE.last());
        // The same roll always names the same pair.
        assert_eq!(log.pick(CONFUSABLE, 0.42), log.pick(CONFUSABLE, 0.42));
        // Out-of-range rolls are clamped, not trusted.
        assert_eq!(log.pick(CONFUSABLE, -5.0), CONFUSABLE.first());
        assert_eq!(log.pick(CONFUSABLE, 12.0), CONFUSABLE.last());
        assert_eq!(log.pick(CONFUSABLE, f64::NAN), CONFUSABLE.first());
        assert_eq!(log.pick(&[], 0.5), None, "nothing to draw from");
    }

    #[test]
    fn a_pair_the_learner_gets_wrong_is_drawn_more_often() {
        let wrong = pair_of('ン', 'ソ');
        let key = pair_key(wrong);

        // A roll that lands in the middle of the field. Fresh, it names whichever
        // pair sits there — ク/フ, the fourth of the thirteen, at 0.3 × 13 = 3.9;
        // after two misses on ン/ソ that pair's share has grown from 1/13 of the
        // field to 5/17, so the same roll now lands inside it.
        let fresh = ConfusionLog::new();
        let roll = 0.3;
        assert_ne!(fresh.pick(CONFUSABLE, roll), Some(wrong));

        let mut log = ConfusionLog::new();
        log.record(&key, false);
        log.record(&key, false);
        assert_eq!(log.pick(CONFUSABLE, roll), Some(wrong));

        // And it is not merely first: its weight really is the largest.
        let (heaviest, weight) = log.weights().remove(0);
        assert_eq!(heaviest, key);
        assert_eq!(weight, 5.0);
    }

    #[test]
    fn the_log_survives_a_round_trip_and_keeps_what_it_does_not_understand() {
        let mut log = ConfusionLog::new();
        log.record(&pair_key(pair_of('わ', 'ゐ')), false);
        log.record(&pair_key(pair_of('わ', 'ゐ')), false);
        log.record(&pair_key(pair_of('わ', 'ゐ')), true);
        // A pair a later version might know about, or a hand edit.
        log.pairs.insert("あ|い".to_string(), PairTally { asked: 4, correct: 3, wrong: 1 });

        let bytes = postcard::to_allocvec(&log).expect("serialises");
        let read_back: ConfusionLog = postcard::from_bytes(&bytes).expect("deserialises");
        assert_eq!(read_back, log);
        assert_eq!(read_back.pairs_seen(), 2);
        assert_eq!(read_back.tally("あ|い").asked, 4, "unknown keys are kept");
        assert_eq!(
            read_back.tally(&pair_key(pair_of('わ', 'ゐ'))),
            PairTally { asked: 3, correct: 1, wrong: 2 },
            "the counts survive, not just the shape"
        );
        assert_eq!(read_back.weight(&pair_key(pair_of('わ', 'ゐ'))), 4.0);
    }

    #[test]
    fn a_kana_pair_keeps_the_key_it_was_stored_under() {
        // The app has already written files under these keys, so widening the key
        // from a pair of kana to a pair of spellings must not move one of them.
        for pair in CONFUSABLE {
            assert_eq!(
                key_of(&pair.a.to_string(), &pair.b.to_string()),
                pair_key(pair),
                "{}/{} changed its key",
                pair.a,
                pair.b
            );
        }
        assert_eq!(pair_key(pair_of('シ', 'ツ')), "シ|ツ");
        assert_eq!(pair_key(pair_of('わ', 'ゐ')), "わ|ゐ", "the lower code point leads");
        assert_eq!(key_of("きや", "きゃ"), "きゃ|きや", "and a yōon sorts by its second kana");
    }

    #[test]
    fn split_key_reads_two_spellings_and_refuses_everything_else() {
        assert_eq!(split_key("シ|ツ"), Some(("シ".into(), "ツ".into())));
        assert_eq!(split_key("きゃ|きや"), Some(("きゃ".into(), "きや".into())));
        // The shapes a caller can get wrong, and none of them is a pair.
        for key in ["シ", "シ|ツ|ク", "", "|", "シ|", "|ツ", "シ|ツ|"] {
            assert_eq!(split_key(key), None, "{key:?} is not two spellings");
        }
    }

    #[test]
    fn the_yoon_contrasts_are_the_digraph_against_the_long_spelling() {
        for (kind, script) in [
            (DrillKind::YoonHiragana, Script::Hiragana),
            (DrillKind::YoonKatakana, Script::Katakana),
        ] {
            assert_eq!(kind.script(), Some(script));
            let pairs = yoon_pairs(script);
            assert_eq!(pairs.len(), 33, "eleven bases of three");

            let kya = pairs.iter().find(|p| p.key() == "きゃ|きや" || p.key() == "キャ|キヤ");
            let kya = kya.unwrap_or_else(|| panic!("{kind:?} holds the kya contrast"));
            assert_eq!(kya.kind, kind);
            assert_eq!(kya.spellings()[0].chars().count(), 2, "a digraph is two characters");
            assert_eq!(kya.spellings()[1].chars().count(), 2, "and so is its counterpart");
            assert_eq!(kya.sides[0].prompt, "kya");
            assert_eq!(kya.sides[1].prompt, "kiya", "the long spelling is prompted long");
            // The tell names both spellings, so it reads correctly whichever way
            // round the question was asked.
            for spelling in kya.spellings() {
                assert!(kya.tell.contains(spelling), "{} is not in the tell", spelling);
            }

            // Every pair is two spellings that differ, and every character of
            // them is a kana the app can draw (its own test asserts that).
            for pair in &pairs {
                assert!(pair.holds(pair.spellings()[0]));
                assert!(!pair.holds("きゃきゃ"), "a longer spelling is not a side");
                assert_ne!(pair.spellings()[0], pair.spellings()[1]);
                assert_eq!(pair.characters().count(), 4, "two spellings of two kana");
                assert!(!pair.sides[0].prompt.is_empty());
                assert!(!pair.sides[1].prompt.is_empty());
            }
        }
    }

    #[test]
    fn every_kind_is_a_pool_of_its_own_with_its_own_keys() {
        assert_eq!(DrillKind::ALL.len(), 5);
        assert_eq!(confusion_pairs().len(), CONFUSABLE.len());
        assert_eq!(DrillKind::YoonHiragana.pairs().len(), 33);
        assert_eq!(DrillKind::YoonKatakana.pairs().len(), 33);
        assert_eq!(DrillKind::VoicingHiragana.pairs().len(), 25);
        assert_eq!(DrillKind::VoicingKatakana.pairs().len(), 25);

        // A key names one pair under one exercise: a tally recorded against a
        // yōon contrast cannot be mistaken for one against a classic pair, and
        // the same kana-pair cannot be claimed by two exercises.
        let mut keys: Vec<String> =
            DrillKind::ALL.iter().flat_map(|k| k.pairs()).map(|p| p.key()).collect();
        let total = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(total, keys.len(), "two pairs share a key");

        // Every tell is a **clause** the screen puts a stop after, never a sentence
        // with one of its own. The first version of the voicing tell ended in a
        // full stop and the live check showed the learner `dakuten ゛..`, which no
        // test had looked for because the thirteen classic tells happen not to.
        for pair in DrillKind::ALL.iter().flat_map(|kind| kind.pairs()) {
            assert!(!pair.tell.trim().is_empty(), "{} has no tell", pair.key());
            assert!(!pair.tell.ends_with('.'), "{} ends its tell with a stop", pair.key());
        }

        // And the kind is what a caller filters by: the yōon spellings are two
        // characters, everything else's are one.
        assert_eq!(DrillKind::Confusion.script(), None, "a classic pair spans the scripts");
        assert_eq!(DrillKind::VoicingKatakana.script(), Some(Script::Katakana));
        for pair in DrillKind::Confusion.pairs()
            .into_iter()
            .chain(DrillKind::VoicingHiragana.pairs())
        {
            for spelling in pair.spellings() {
                assert_eq!(spelling.chars().count(), 1, "{spelling}");
            }
            assert!(!pair.tell.is_empty());
        }
        for pair in DrillKind::YoonHiragana.pairs() {
            for spelling in pair.spellings() {
                assert_eq!(spelling.chars().count(), 2, "{spelling}");
            }
        }
    }

    #[test]
    fn the_voicing_pairs_differ_only_by_the_mark() {
        // The two kana of a voicing pair are the same kana with a dakuten or a
        // handakuten, which in this block is one code point for the dakuten and
        // two for the handakuten. The pairs are *built* from the rows (see
        // `VOICED_FROM`), so this is the independent half: it says the relation the
        // table claims is really the one the characters have.
        for script in [Script::Hiragana, Script::Katakana] {
            let pairs = voicing_pairs(script);
            assert_eq!(pairs.len(), 25, "{script:?}: five voiced rows of five");
            for pair in &pairs {
                assert_eq!(pair.kind.script(), Some(script));
                let [Some(plain), Some(voiced)] = [pair.sides[0].spelling.chars().next(), pair.sides[1].spelling.chars().next()]
                else {
                    panic!("{} is not two kana", pair.key())
                };
                let step = (voiced as u32) - (plain as u32);
                match step {
                    1 => assert!(pair.tell.contains("dakuten"), "{}", pair.key()),
                    2 => assert!(pair.tell.contains("handakuten"), "{}", pair.key()),
                    _ => panic!("{} differ by {step} code points, not a mark", pair.key()),
                }
                assert_ne!(pair.sides[0].prompt, pair.sides[1].prompt, "{}", pair.key());
                assert!(!pair.sides[0].prompt.is_empty() && !pair.sides[1].prompt.is_empty());
            }

            // は is the one kana with two voiced forms, and both are asked about.
            let ha: Vec<String> = pairs
                .iter()
                .filter(|p| p.sides[0].spelling == "は" || p.sides[0].spelling == "ハ")
                .map(DrillPair::key)
                .collect();
            assert_eq!(ha.len(), 2, "{script:?}: は is voiced two ways");
            assert!(ha.contains(&"は|ば".to_string()) || ha.contains(&"ハ|バ".to_string()));
            assert!(ha.contains(&"は|ぱ".to_string()) || ha.contains(&"ハ|パ".to_string()));
        }

        // And the pairs are (plain, voiced) in that order, so the key reads the way
        // the tell does.
        let kaga = voicing_pairs(Script::Hiragana);
        let ka = kaga.iter().find(|p| p.key() == "か|が").expect("か / が");
        assert_eq!(ka.sides[0].prompt, "ka");
        assert_eq!(ka.sides[1].prompt, "ga");
        assert!(ka.tell.contains("か") && ka.tell.contains("が"));
    }

    #[test]
    fn the_voicing_pairs_come_from_the_rows_the_course_teaches() {
        // The plain row a voiced row is voiced from has to exist, be plain, and be
        // the same shape — otherwise the zip in `voicing_pairs` would silently
        // produce fewer pairs than the language has.
        for &(voiced_sound, plain_sound, _) in VOICED_FROM {
            let row = |sound: &str| {
                ROWS.iter().find(|r| r.sound == sound).unwrap_or_else(|| panic!("no {sound} row"))
            };
            let voiced = row(voiced_sound);
            let plain = row(plain_sound);
            assert!(voiced.voiced, "{voiced_sound} is not a voiced row");
            assert!(!plain.voiced, "{plain_sound} is not a plain row");
            assert_eq!(voiced.len(), plain.len(), "{voiced_sound} / {plain_sound} differ in shape");
            assert_eq!(voiced.len(), 5);
        }
    }

    /// **The written exception to the prompt rule**, measured rather than assumed.
    ///
    /// A side's prompt is its kana's own primary reading, and a caller that types
    /// that reading expects the kana back. For four of the hundred voicing sides
    /// that is not what happens, and it cannot be: ぢ's Hepburn reading is `ji`,
    /// the same as じ's, so `to_kana_in` answers じ; づ's is `zu`, the same as
    /// ず's. Hepburn does not distinguish the yotsugana and Kunrei does (`di`,
    /// `du`), and swapping the prompts to the Kunrei spellings would teach a
    /// romanisation the Practice screen never shows — so the prompt stays the
    /// kana's own reading and the exception is written here instead.
    ///
    /// It does not make the question ambiguous: the drill only ever offers the
    /// pair's own two spellings, and within the pair ち is prompted `chi` and ぢ
    /// `ji`. What it means is that the prompt identifies the *contrast* rather
    /// than the character, which is also why `matches_reading('ぢ', "ji")` is
    /// true and a learner who then types `ji` on the board is not marked wrong.
    #[test]
    fn four_voicing_prompts_are_the_yotsugana_and_type_back_to_their_pair() {
        let expected = [('ち', 'ぢ', "chi", "ji"), ('つ', 'づ', "tsu", "zu")];
        let mut checked = 0;
        for script in [Script::Hiragana, Script::Katakana] {
            for pair in voicing_pairs(script) {
                let (Some(plain), Some(voiced)) =
                    (pair.sides[0].spelling.chars().next(), pair.sides[1].spelling.chars().next())
                else {
                    continue;
                };
                let mirror = |hira: char| match script {
                    Script::Hiragana => Some(hira),
                    Script::Katakana => crate::to_katakana(hira),
                };
                let Some((_, want_voiced, plain_prompt, voiced_prompt)) =
                    expected.iter().find(|(p, _, _, _)| mirror(*p) == Some(plain)).copied()
                else {
                    // Every other prompt types back to its own kana.
                    for side in &pair.sides {
                        let typed = crate::to_kana_in(script, &side.prompt).ok();
                        assert_eq!(
                            typed.as_deref(),
                            Some(side.spelling.as_str()),
                            "{} is prompted {} and types as something else",
                            side.spelling,
                            side.prompt
                        );
                    }
                    continue;
                };
                assert_eq!(mirror(want_voiced), Some(voiced));
                assert_eq!(pair.sides[0].prompt, plain_prompt);
                assert_eq!(pair.sides[1].prompt, voiced_prompt);
                // The pair is still answerable because the two prompts differ.
                assert_ne!(pair.sides[0].prompt, pair.sides[1].prompt);
                // And the voiced side's reading is accepted for it, which is why
                // this is an exception to the *round trip* rather than to the
                // reading: `ji` really is how ぢ is written in Hepburn.
                assert!(crate::matches_reading(voiced, voiced_prompt));
                assert_ne!(
                    crate::to_kana_in(script, voiced_prompt).ok().as_deref(),
                    Some(pair.sides[1].spelling.as_str()),
                    "{} unexpectedly types back, so the exception is stale",
                    pair.sides[1].spelling
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 4, "two kana in two scripts are the yotsugana exception");
    }


    #[test]
    fn the_draw_and_the_rule_work_over_a_yoon_pair_too() {
        let pairs = yoon_pairs(Script::Hiragana);
        let kya = pairs.iter().find(|p| p.key() == "きゃ|きや").expect("the contrast");
        assert_eq!(kya.key(), key_of("きゃ", "きや"));
        let key = kya.key();

        // The same arithmetic, over a different pool: a roll that lands one pair
        // along on a fresh log lands on the missed pair once it weighs three.
        let fresh = ConfusionLog::new();
        let roll = 0.05;
        assert_ne!(
            fresh.pick_by(&pairs, DrillPair::key, roll).map(DrillPair::key),
            Some(key.clone())
        );
        assert_eq!(fresh.pick_by(&pairs, DrillPair::key, 0.0).map(DrillPair::key), pairs.first().map(DrillPair::key));
        assert_eq!(fresh.pick_by(&pairs, DrillPair::key, 1.0).map(DrillPair::key), pairs.last().map(DrillPair::key));
        assert_eq!(fresh.pick_by(&[], DrillPair::key, 0.5), None, "nothing to draw from");

        let mut log = ConfusionLog::new();
        log.record(&key, false);
        assert_eq!(log.weight(&key), 3.0, "the rule is the same one");
        assert_eq!(
            log.pick_by(&pairs, DrillPair::key, roll).map(DrillPair::key),
            Some(key.clone()),
            "the same roll now lands on the missed pair"
        );
    }
}
