//! What the discrimination drill remembers about *this* learner.
//!
//! The drill asks one question — here is a reading, which of these two shapes is
//! it — over the pairs in [`crate::CONFUSABLE`]. The pairs are practitioner
//! consensus, so they are the same for everybody; which of them *you* get wrong is
//! not, and until this module existed the drill threw that away after every
//! answer. It is the cheapest personalised thing in the app and it needs no data
//! from anywhere.
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

use crate::curriculum::{Confusable, CONFUSABLE};

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

/// The key a pair is stored under: both kana, lower code point first, so that
/// (ツ, シ) and (シ, ツ) are one entry rather than two halves of one learner's
/// mistake. `ゐ|わ`, `シ|ツ`.
///
/// A string rather than a struct because it is also the key in the JSON file, and
/// that file is meant to be readable by whoever is wondering what the app knows
/// about them.
pub fn pair_key(pair: &Confusable) -> String {
    let (first, second) = if (pair.a as u32) <= (pair.b as u32) {
        (pair.a, pair.b)
    } else {
        (pair.b, pair.a)
    };
    format!("{first}|{second}")
}

/// The pair a key names, in **either** spelling, or `None` if it is not one of
/// the classic pairs.
///
/// This is the validation the app does before it records anything: a key that is
/// not a pair is a bug in the caller, and writing it into the learner's file
/// would leave junk there for good. `ツ|シ` finds `シ|ツ`, because the two name
/// one pair — the canonical order is what the file is *written* in, not a rule
/// the caller has to know.
pub fn find_pair(key: &str) -> Option<&'static Confusable> {
    let mut chars = key.split('|').map(|part| {
        let mut chars = part.chars();
        match (chars.next(), chars.next()) {
            (Some(ch), None) => Some(ch),
            _ => None,
        }
    });
    let (Some(Some(a)), Some(Some(b)), None) = (chars.next(), chars.next(), chars.next()) else {
        return None;
    };
    CONFUSABLE
        .iter()
        .find(|pair| (pair.a == a && pair.b == b) || (pair.a == b && pair.b == a))
}

/// The learner's record over the confusion pairs.
///
/// A pair that has never been asked is simply absent from the map, so a fresh
/// log is empty and the file on disk is `{"pairs": {}}` rather than thirteen
/// entries that say nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfusionLog {
    /// Keyed by [`pair_key`]. Unknown keys — pairs a later version added, or a
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
        if pairs.is_empty() {
            return None;
        }
        let total: f64 = pairs
            .iter()
            .map(|pair| self.weight(&pair_key(pair)))
            .sum();
        // The floor makes this unreachable, but a total that is zero or NaN —
        // a caller handing over a whole log of its own — must not become a
        // division by nothing.
        if !total.is_finite() || total <= 0.0 {
            return None;
        }
        let roll = if roll.is_nan() { 0.0 } else { roll.clamp(0.0, 1.0) };
        // `roll == 1.0` would land past the last pair and be caught by the
        // fallback below; the clamp keeps a caller's `1.0` on the last pair
        // rather than making it an accident of floating-point.
        let mut remaining = roll.min(1.0 - f64::EPSILON) * total;
        for pair in pairs {
            let weight = self.weight(&pair_key(pair));
            if remaining < weight {
                return Some(pair);
            }
            remaining -= weight;
        }
        pairs.last()
    }

    /// How the weights stand, biggest first, for a caller that wants to show the
    /// learner what they are being asked about most. Ties keep the order the
    /// pairs are declared in, so this is stable.
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
}
