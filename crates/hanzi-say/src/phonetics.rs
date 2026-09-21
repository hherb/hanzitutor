//! Telling a mispronunciation from a spelling difference.
//!
//! ## The problem this solves
//!
//! `verify-audio` compares the source text with what a recogniser heard, and a
//! plain character-error threshold is not enough. The recogniser writes plausible
//! neighbours, so corrupt audio can score *under* any threshold that still
//! tolerates its legitimate substitutions:
//!
//! - `请进星坐` for `请进请坐` — 进 → 星 is a different syllable, and passes a CER check.
//! - `眼镜请坐` for `请进请坐` — 请 → 眼 and 进 → 镜, likewise.
//!
//! What separates the two cases is **phonetic, not orthographic**. A recogniser
//! writing 可 for 渴 has heard the right syllable and spelled it its own way. A
//! model *saying* 笔 where the text says 请 has produced a different syllable. So
//! each syllable is split into initial, final and tone, and the comparison is
//! made per position:
//!
//! | Difference | Reading | Verdict |
//! | --- | --- | --- |
//! | tone only (渴 → 可) | the right syllable, spelled differently | tolerate |
//! | final same, initial a known-confusable neighbour (刻 → 饿) | the recogniser's own confusion | tolerate |
//! | final same, initial unrelated (请 qing → 星 xing) | a different sound, clearly heard | **corrupt** |
//! | final differs (进 → 星) | not the syllable that was asked for | **corrupt** |
//! | both differ (请 → 笔) | not the syllable that was asked for | **corrupt** |
//!
//! The tolerance is deliberately narrow, and narrower than a first attempt. That
//! version accepted *any* same-final pair, which passed `请进星坐` — 请 `qing3`
//! heard as 星 `xing1` shares the final `ing`, so the clip was cleared even
//! though `q` and `x` are different sounds and a learner would say so. What is
//! genuinely indistinguishable to a listener is a small set of pairs: the
//! retroflex/dental series (`zh`/`z`, `ch`/`c`, `sh`/`s`), `n`/`l`, `f`/`h`,
//! and a syllable that drops its initial entirely (`刻` `ke4` → `饿` `e4`, where
//! finals `e` and `e` agree and the onset was simply not heard).

use std::collections::{HashMap, HashSet};

/// A syllable split into the parts a learner actually gets wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Syllable {
    pub initial: &'static str,
    pub final_: &'static str,
    pub tone: u8,
}

/// Initials, longest first, so `zh` is matched before `z`.
///
/// `y` and `w` are included because pinyin writes them where the underlying
/// sound is a medial; treating them as initials keeps 一 (`yi`) and 五 (`wu`)
/// comparable to how a reader would say them.
const INITIALS: [&str; 23] = [
    "zh", "ch", "sh", "b", "p", "m", "f", "d", "t", "n", "l", "g", "k", "h", "j", "q", "x", "r",
    "z", "c", "s", "y", "w",
];

/// Map a tone-marked vowel to its plain letter and the tone it carries.
const TONE_MARKS: [(char, char, u8); 24] = [
    ('ā', 'a', 1), ('á', 'a', 2), ('ǎ', 'a', 3), ('à', 'a', 4),
    ('ē', 'e', 1), ('é', 'e', 2), ('ě', 'e', 3), ('è', 'e', 4),
    ('ī', 'i', 1), ('í', 'i', 2), ('ǐ', 'i', 3), ('ì', 'i', 4),
    ('ō', 'o', 1), ('ó', 'o', 2), ('ǒ', 'o', 3), ('ò', 'o', 4),
    ('ū', 'u', 1), ('ú', 'u', 2), ('ǔ', 'u', 3), ('ù', 'u', 4),
    // `ü` in all four tones. Folding these into `u` would make lǜ and lù the
    // same syllable, which is exactly the kind of error this module exists to
    // catch — so they are kept distinct, and the `ü` finals are in `FINALS`.
    ('ǖ', 'ü', 1), ('ǘ', 'ü', 2), ('ǚ', 'ü', 3), ('ǜ', 'ü', 4),
];

/// Neutral tone when nothing marks it — the tone that carries no mark in pinyin.
const NEUTRAL: u8 = 5;

/// Parse one reading, in either the tone-marked or the numbered spelling.
///
/// Both are in play because the two pinyin sources disagree: `hanziDB.csv` marks
/// the tone on the vowel and `hsk-words.json` writes a digit. Normalising here
/// means the rest of the module never has to know which it was given.
pub fn parse(reading: &str) -> Option<Syllable> {
    let raw = reading.trim().to_lowercase();
    if raw.is_empty() {
        return None;
    }

    // Tone: a trailing digit if there is one, otherwise read off the vowel.
    let mut tone = None;
    let mut body = String::new();
    for ch in raw.chars() {
        if let Some(digit) = ch.to_digit(10) {
            if (1..=5).contains(&digit) {
                tone = Some(digit as u8);
            }
            continue;
        }
        match TONE_MARKS.iter().find(|(marked, _, _)| *marked == ch) {
            Some((_, plain, t)) => {
                body.push(*plain);
                tone.get_or_insert(*t);
            }
            None => body.push(ch),
        }
    }
    if body.is_empty() {
        return None;
    }
    let tone = tone.unwrap_or(NEUTRAL);

    // `v` is how the numbered sources spell `ü`; make them one character. It is
    // emphatically *not* folded into `u`: lǜ and lù are different syllables, and
    // treating them as the same would hide a real mispronunciation.
    let body = body.replace('v', "ü");

    let initial = INITIALS
        .iter()
        .find(|i| body.starts_with(**i))
        .copied()
        .unwrap_or("");
    let final_ = &body[initial.len()..];
    if final_.is_empty() {
        return None;
    }

    // The final has to be one of the fixed set for the lifetime of the struct.
    // A reading this module does not understand is dropped rather than given a
    // placeholder: a placeholder that never matches would report a corruption
    // that may not exist.
    let final_ = FINALS.iter().find(|f| **f == final_).copied()?;
    Some(Syllable { initial, final_, tone })
}

/// Every final that appears in the two bundled pinyin sources.
///
/// A closed set because [`Syllable`] borrows `'static` strings. A final outside
/// it becomes `None` from [`parse`], so a reading that cannot be understood is
/// dropped rather than compared — the alternative, a placeholder that never
/// matches, would report a corruption that may not exist.
const FINALS: [&str; 37] = [
    "a", "o", "e", "ai", "ei", "ao", "ou", "an", "en", "ang", "eng", "ong", "er", "i", "ia",
    "ie", "iao", "iu", "ian", "in", "iang", "ing", "iong", "io", "u", "ua", "uo", "uai", "ui",
    "uan", "un", "uang", "ueng", "ü", "üe", "üan", "ün",
];

/// How one heard syllable differs from the wanted one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difference {
    /// The same syllable, spelled the recogniser's way. Not a fault.
    ToneOnly,
    /// A syllable that exists, starting differently. Not treated as a fault —
    /// see the module comment for why the initial is not held to the same bar.
    InitialOnly,
    /// A different syllable. The model did not say what the text says.
    Final,
    /// One of the two characters has no known reading, so nothing can be said.
    Unknown,
}

impl Difference {
    /// True when this difference means the clip is wrong.
    pub fn is_corruption(self) -> bool {
        matches!(self, Self::Final | Self::Unknown)
    }
}

/// Compare a heard syllable against the wanted one.
///
/// Either side is a *set* of readings, because most characters have more than
/// one and the text does not say which is meant. The pair that agrees most is
/// the one used, which is the generous reading: a character is only counted
/// wrong when no reading of it matches.
pub fn compare(wanted: &HashSet<Syllable>, heard: &HashSet<Syllable>) -> Difference {
    if wanted.is_empty() || heard.is_empty() {
        return Difference::Unknown;
    }
    let mut best: Option<Difference> = None;
    for w in wanted {
        for h in heard {
            let candidate = classify(w, h);
            // Keep the most lenient verdict across the reading pairs, because a
            // character with several readings is only wrong when *none* of them
            // matches what was heard.
            let better = match best {
                None => true,
                Some(current) => rank(candidate) < rank(current),
            };
            if better {
                best = Some(candidate);
            }
        }
    }
    best.unwrap_or(Difference::Unknown)
}

/// Initials a listener genuinely does not distinguish, as unordered pairs.
///
/// Kept deliberately short. Each entry is a confusion that occurs between
/// speakers and recognisers alike; anything outside this table is a difference
/// worth reporting, which is the conservative direction for a pronunciation
/// tutor — a wrongly-flagged clip is reviewed, a wrongly-cleared one is taught.
const CONFUSABLE_INITIALS: [(&str, &str); 6] = [
    ("zh", "z"),
    ("ch", "c"),
    ("sh", "s"),
    ("n", "l"),
    ("f", "h"),
    ("r", "l"),
];

/// True when two initials are close enough that hearing one for the other is the
/// listener's confusion rather than the model's error.
///
/// An empty initial is confusable with anything: a syllable whose onset was not
/// heard at all (`刻` `ke4` → `饿` `e4`) still has the right final, and the
/// recogniser dropping an onset is not evidence the model omitted it.
fn confusable_initials(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if a.is_empty() || b.is_empty() {
        return true;
    }
    CONFUSABLE_INITIALS
        .iter()
        .any(|(x, y)| (a == *x && b == *y) || (a == *y && b == *x))
}

/// How one reading of the wanted character relates to one reading of the heard.
///
/// Exactly one verdict per pair. An earlier version returned the lenient
/// `InitialOnly` whenever *any* pair shared a final, which let a corruption
/// through: 请 `qing3` heard as 星 `xing1` shares the final `ing`, so the clip
/// was cleared even though `q` and `x` are different sounds. The leniency now
/// also requires the initials to be a genuinely confusable pair.
fn classify(w: &Syllable, h: &Syllable) -> Difference {
    match (w.initial == h.initial, w.final_ == h.final_) {
        // Identical syllable; only the tone could differ.
        (true, true) => Difference::ToneOnly,
        // The same final, with an initial a listener does not distinguish.
        (false, true) if confusable_initials(w.initial, h.initial) => Difference::InitialOnly,
        // Everything else is not the syllable the text asked for.
        _ => Difference::Final,
    }
}

fn rank(d: Difference) -> u8 {
    match d {
        Difference::ToneOnly => 0,
        Difference::InitialOnly => 1,
        Difference::Final => 2,
        Difference::Unknown => 3,
    }
}

/// A character's readings, keyed by character.
#[derive(Debug, Default)]
pub struct Readings {
    map: HashMap<char, HashSet<Syllable>>,
}

impl Readings {
    /// Build from `(character, reading)` pairs, ignoring anything unparseable.
    pub fn from_pairs<I: IntoIterator<Item = (char, String)>>(pairs: I) -> Self {
        let mut map: HashMap<char, HashSet<Syllable>> = HashMap::new();
        for (ch, reading) in pairs {
            // A single cell may hold several readings, comma-separated.
            for part in reading.split([',', '/']) {
                if let Some(syllable) = parse(part) {
                    map.entry(ch).or_default().insert(syllable);
                }
            }
        }
        Self { map }
    }

    /// The readings of one character, or an empty set when it is unknown.
    pub fn of(&self, ch: char) -> &HashSet<Syllable> {
        static EMPTY: std::sync::OnceLock<HashSet<Syllable>> = std::sync::OnceLock::new();
        self.map
            .get(&ch)
            .unwrap_or_else(|| EMPTY.get_or_init(HashSet::new))
    }

    /// How many characters have at least one reading.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Every position where `heard` is not the syllable `wanted` asks for.
///
/// Returns an empty vector when the two agree, and `None` when they cannot be
/// aligned at all — a different length means a clause was dropped or added,
/// which is a failure on its own and does not need a phonetic reading.
pub fn disagreements(wanted: &str, heard: &str, readings: &Readings) -> Option<Vec<Difference>> {
    let w: Vec<char> = wanted.chars().collect();
    let h: Vec<char> = heard.chars().collect();
    if w.len() != h.len() {
        return None;
    }
    let mut out = Vec::new();
    for (a, b) in w.iter().zip(h.iter()) {
        if a == b {
            out.push(Difference::ToneOnly);
            continue;
        }
        out.push(compare(readings.of(*a), readings.of(*b)));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(reading: &str) -> Syllable {
        parse(reading).unwrap_or_else(|| panic!("could not parse {reading:?}"))
    }

    fn set(readings: &[&str]) -> HashSet<Syllable> {
        readings.iter().map(|r| s(r)).collect()
    }

    #[test]
    fn parses_tone_marked_and_numbered_spellings_alike() {
        assert_eq!(parse("qǐng"), parse("qing3"));
        assert_eq!(parse("hǎo"), parse("hao3"));
        assert_eq!(parse("nǐ"), parse("ni3"));
        assert_eq!(parse("lǜ"), parse("lv4"));
    }

    #[test]
    fn splits_initial_and_final() {
        assert_eq!(s("zhang1").initial, "zh");
        assert_eq!(s("zhang1").final_, "ang");
        assert_eq!(s("qing3").initial, "q");
        assert_eq!(s("qing3").final_, "ing");
        assert_eq!(s("an1").initial, "");
        assert_eq!(s("an1").final_, "an");
    }

    #[test]
    fn a_neutral_tone_is_read_when_no_mark_is_present() {
        assert_eq!(s("de").tone, NEUTRAL);
        assert_eq!(s("de5").tone, NEUTRAL);
    }

    #[test]
    fn unparseable_input_is_rejected() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("   "), None);
        assert_eq!(parse("3"), None);
    }

    #[test]
    fn a_tone_difference_is_not_corruption() {
        // The measured case: the recogniser writes 可 for 渴.
        assert_eq!(compare(&set(&["ke3"]), &set(&["ke3"])), Difference::ToneOnly);
        // And a real tone slip on the same syllable.
        assert_eq!(compare(&set(&["ke3"]), &set(&["ke4"])), Difference::ToneOnly);
    }

    #[test]
    fn the_measured_corruptions_are_caught() {
        // 请 qing3 -> 笔 bi3: initial and final both differ.
        assert!(compare(&set(&["qing3"]), &set(&["bi3"])).is_corruption());
        // 进 jin4 -> 星 xing1: same initial j? No — x differs, and the final does too.
        assert!(compare(&set(&["jin4"]), &set(&["xing1"])).is_corruption());
        // 进 jin4 -> 迹 ji4: same initial, different final.
        assert!(compare(&set(&["jin4"]), &set(&["ji4"])).is_corruption());
        // 先 xian1 -> 姐 jie3.
        assert!(compare(&set(&["xian1"]), &set(&["jie3"])).is_corruption());
        // 请 qing3 -> 星 xing1: the same final, and still corruption, because the
        // initials are not a pair a listener merges.
        assert!(compare(&set(&["qing3"]), &set(&["xing1"])).is_corruption());
    }

    #[test]
    fn a_confusable_initial_on_the_same_final_is_tolerated() {
        // 刻 ke4 heard as 饿 e4: the final agrees and the onset was not heard,
        // which is the recogniser's doing rather than the model's.
        assert_eq!(compare(&set(&["ke4"]), &set(&["e4"])), Difference::InitialOnly);
        assert!(!compare(&set(&["ke4"]), &set(&["e4"])).is_corruption());
        // The retroflex/dental series, which speakers genuinely merge.
        for (a, b) in [("zhi1", "zi1"), ("chi1", "ci1"), ("shi1", "si1")] {
            assert_eq!(
                compare(&set(&[a]), &set(&[b])),
                Difference::InitialOnly,
                "{a} vs {b} should be a listener's confusion"
            );
        }
    }

    #[test]
    fn an_unrelated_initial_on_the_same_final_is_not_tolerated() {
        // The case a first attempt passed: 请 qing3 heard as 星 xing1. The final
        // matches, so the narrow rule would clear it — but `q` and `x` are
        // different sounds and a learner would say so.
        assert!(
            compare(&set(&["qing3"]), &set(&["xing1"])).is_corruption(),
            "q -> x must not be waved through"
        );
        assert!(compare(&set(&["ji1"]), &set(&["xi1"])).is_corruption());
        assert!(compare(&set(&["ke1"]), &set(&["te1"])).is_corruption());
    }

    #[test]
    fn the_tolerance_never_overrides_a_different_final() {
        // Whatever the initials, a final that differs is corruption: this is the
        // property that makes the whole screen trustworthy.
        for (a, b) in [("jin4", "xing1"), ("jin4", "ji4"), ("qing3", "qie4")] {
            assert!(
                compare(&set(&[a]), &set(&[b])).is_corruption(),
                "{a} vs {b}: a different final must always be reported"
            );
        }
    }

    #[test]
    fn a_character_with_several_readings_matches_on_any_of_them() {
        // 行 is xing2 or hang2; hearing either must not be a fault.
        let wanted = set(&["xing2", "hang2"]);
        assert_eq!(compare(&wanted, &set(&["hang2"])), Difference::ToneOnly);
        assert_eq!(compare(&wanted, &set(&["xing2"])), Difference::ToneOnly);
    }

    #[test]
    fn an_unknown_character_is_reported_rather_than_ignored() {
        assert_eq!(compare(&HashSet::new(), &set(&["qing3"])), Difference::Unknown);
        assert!(compare(&HashSet::new(), &set(&["qing3"])).is_corruption());
    }

    #[test]
    fn a_dropped_clause_cannot_be_aligned() {
        let readings = Readings::from_pairs([('到', "dao4".into())]);
        assert_eq!(disagreements("到了请下车", "到了", &readings), None);
    }

    #[test]
    fn the_same_text_agrees_everywhere() {
        let readings = Readings::from_pairs([
            ('请', "qing3".into()),
            ('进', "jin4".into()),
        ]);
        let d = disagreements("请进", "请进", &readings).unwrap();
        assert_eq!(d, vec![Difference::ToneOnly, Difference::ToneOnly]);
    }

    #[test]
    fn readings_accept_a_comma_separated_cell() {
        // `hanziDB.csv` writes alternatives this way.
        let readings = Readings::from_pairs([('的', "de, dí, dì".into())]);
        assert_eq!(readings.of('的').len(), 3);
    }

    #[test]
    fn several_readings_are_collected_per_character() {
        let readings = Readings::from_pairs([
            ('呵', "a1".into()),
            ('呵', "he1".into()),
        ]);
        assert_eq!(readings.of('呵').len(), 2);
        assert!(readings.of('未').is_empty());
    }
}
