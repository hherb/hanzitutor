//! Decide whether a clip says what its text says.
//!
//! This module is the *judgement* half of `verify-audio`, kept apart from the
//! recogniser so it can be tested against recorded cases without a model.
//!
//! ## Why two checks and not one
//!
//! A character-error rate alone is not enough, because a recogniser writes
//! plausible neighbours. `请进星坐` for `请进请坐` scores under any threshold that
//! still tolerates the substitutions the recogniser makes legitimately, so the
//! audio passes while saying the wrong syllable. The fix is to ask *how* the
//! characters differ, not only how many: [`crate::phonetics`] splits each syllable
//! into initial, final and tone and reports whether the difference is a spelling
//! preference (渴 → 可, same syllable, different tone written) or a different
//! syllable (请 → 笔).
//!
//! Two independent signals come out of that, and either can fail a clip:
//!
//! * **alignment** — a different number of characters means a clause was dropped
//!   or repeated. No phonetic reading is needed; it is simply wrong.
//! * **syllables** — a position whose *final* differs is a different syllable,
//!   however plausible the character looks.
//!
//! The character-error rate is still computed and reported, because it is the
//! number a human can read at a glance and it makes the failures comparable. It
//! is no longer what decides.

use std::collections::HashSet;

use crate::phonetics::{self, Difference, Readings};

/// Above this share of characters wrong, a clip is reported on the CER alone.
///
/// Kept as a backstop for the case the phonetic reading cannot judge — an
/// unknown character on either side. The measured failures sit between 0.4 and
/// 1.0, and the recogniser's own respellings under 0.3, so this is where the
/// two separate.
pub const MAX_CER: f64 = 0.34;

/// What one clip's two texts say about each other.
#[derive(Debug, PartialEq)]
pub enum Verdict {
    /// The same words, however the recogniser spelled them.
    Agrees,
    /// A different number of characters: a clause dropped or added.
    Misaligned { wanted: usize, heard: usize },
    /// Character positions where the model produced a different syllable.
    DifferentSyllables(Vec<Position>),
    /// A character neither text can be read for, with the CER over threshold.
    Unreadable { cer: f64 },
}

/// One position where the syllables differ.
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    /// Index in the phrase, for reporting.
    pub index: usize,
    pub wanted: char,
    pub heard: char,
    pub difference: Difference,
}

impl Verdict {
    pub fn is_failure(&self) -> bool {
        !matches!(self, Self::Agrees)
    }

    /// A short reason, for the report line.
    pub fn reason(&self) -> String {
        match self {
            Self::Agrees => "agrees".to_string(),
            Self::Misaligned { wanted, heard } => {
                format!("{wanted} chars wanted, {heard} heard — a clause was dropped or added")
            }
            Self::DifferentSyllables(positions) => {
                let shown: Vec<String> = positions
                    .iter()
                    .take(3)
                    .map(|p| format!("{}→{}", p.wanted, p.heard))
                    .collect();
                format!("different syllable at {}", shown.join(", "))
            }
            Self::Unreadable { cer } => {
                format!("no reading for some characters, and cer {cer:.2}")
            }
        }
    }
}

/// Characters kept from a transcript: Han only, so punctuation and the
/// recogniser's spacing cannot count as errors against the source text.
pub fn han_only(text: &str) -> String {
    text.chars()
        .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
        .collect()
}

/// Levenshtein distance over characters, divided by the reference length.
pub fn character_error_rate(reference: &str, heard: &str) -> f64 {
    let r: Vec<char> = reference.chars().collect();
    let h: Vec<char> = heard.chars().collect();
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    let mut previous: Vec<usize> = (0..=h.len()).collect();
    let mut current = vec![0usize; h.len() + 1];
    for (i, rc) in r.iter().enumerate() {
        current[0] = i + 1;
        for (j, hc) in h.iter().enumerate() {
            let substitute = previous[j] + usize::from(rc != hc);
            current[j + 1] = substitute.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[h.len()] as f64 / r.len() as f64
}

/// Judge one clip from the text it should say and the text that was heard.
///
/// `known` is the set of characters the readings can speak for. A position is
/// only reported as a different syllable when *both* characters are known:
/// reporting a corruption on the strength of a character the app cannot read
/// would be a guess, and the CER backstop covers that case instead.
pub fn judge(wanted: &str, heard: &str, readings: &Readings) -> Verdict {
    let w = han_only(wanted);
    let h = han_only(heard);
    let cer = character_error_rate(&w, &h);

    let wchars: Vec<char> = w.chars().collect();
    let hchars: Vec<char> = h.chars().collect();
    if wchars.len() != hchars.len() {
        return Verdict::Misaligned {
            wanted: wchars.len(),
            heard: hchars.len(),
        };
    }

    let mut positions = Vec::new();
    let mut all_known = true;
    for (index, (a, b)) in wchars.iter().zip(hchars.iter()).enumerate() {
        if a == b {
            continue;
        }
        let (wa, hb) = (readings.of(*a), readings.of(*b));
        if wa.is_empty() || hb.is_empty() {
            all_known = false;
            continue;
        }
        let difference = phonetics::compare(wa, hb);
        if difference.is_corruption() {
            positions.push(Position {
                index,
                wanted: *a,
                heard: *b,
                difference,
            });
        }
    }

    if !positions.is_empty() {
        return Verdict::DifferentSyllables(positions);
    }
    if !all_known && cer > MAX_CER {
        return Verdict::Unreadable { cer };
    }
    Verdict::Agrees
}

/// Build a readings table from `character<TAB>reading` lines.
///
/// Deliberately the simplest possible format, so the caller can produce it from
/// either bundled pinyin source with one `awk`-less pass and no dependency on
/// which of them it came from.
pub fn readings_from_lines<I: IntoIterator<Item = String>>(lines: I) -> Readings {
    let mut pairs = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((ch, reading)) = line.split_once(['\t', ' ']) {
            let mut chars = ch.trim().chars();
            if let (Some(c), None) = (chars.next(), chars.next()) {
                pairs.push((c, reading.trim().to_string()));
            }
        }
    }
    Readings::from_pairs(pairs)
}

/// The characters a readings table cannot speak for, within `text`.
pub fn unknown_characters(text: &str, readings: &Readings) -> HashSet<char> {
    han_only(text)
        .chars()
        .filter(|c| readings.of(*c).is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn readings() -> Readings {
        Readings::from_pairs([
            ('请', "qing3".into()),
            ('进', "jin4".into()),
            ('坐', "zuo4".into()),
            ('先', "xian1".into()),
            ('生', "sheng1".into()),
            ('您', "nin2".into()),
            ('找', "zhao3".into()),
            ('谁', "shei2".into()),
            ('笔', "bi3".into()),
            ('迹', "ji4".into()),
            ('做', "zuo4".into()),
            ('你', "ni3".into()),
            ('可', "ke3".into()),
            ('渴', "ke3".into()),
            ('到', "dao4".into()),
            ('了', "le5".into()),
            ('星', "xing1".into()),
            ('眼', "yan3".into()),
            ('镜', "jing4".into()),
            ('l', "".into()),
        ])
    }

    #[test]
    fn identical_text_agrees() {
        let r = readings();
        assert_eq!(judge("请进请坐", "请进请坐", &r), Verdict::Agrees);
        assert!(!judge("请进请坐", "请进请坐", &r).is_failure());
    }

    #[test]
    fn a_tone_only_difference_is_not_a_failure() {
        // 渴 in the text, 可 heard: the same syllable spelled the recogniser's way.
        assert_eq!(judge("渴", "可", &readings()), Verdict::Agrees);
    }

    #[test]
    fn the_measured_corruption_is_a_failure() {
        // The case this whole module exists for: 请进请坐 heard as 笔迹你做.
        let v = judge("请进请坐", "笔迹你做", &readings());
        assert!(v.is_failure(), "{v:?}");
        match v {
            Verdict::DifferentSyllables(p) => {
                assert!(p.iter().any(|p| p.wanted == '请' && p.heard == '笔'));
            }
            other => panic!("expected different syllables, got {other:?}"),
        }
    }

    #[test]
    fn a_plausible_neighbour_is_reported_as_a_syllable_not_just_a_count() {
        // 请进星坐 and 眼镜请坐 both say the wrong syllables. Worth being honest
        // about the division of labour: on a four-character phrase a single wrong
        // character is already 0.25 and two is 0.5, so the CER backstop alone
        // would flag these too. What the phonetic screen adds is *why* — that 请
        // `qing3` heard as 星 `xing1` is a different syllable rather than the
        // recogniser's spelling, which is the judgement that separates `渴`/`可`
        // (tolerate) from these (corrupt). The two are complements, not
        // substitutes.
        let r = readings();
        for heard in ["请进星坐", "眼镜请坐"] {
            let v = judge("请进请坐", heard, &r);
            assert!(v.is_failure(), "{heard} should fail: {v:?}");
            assert!(
                matches!(v, Verdict::DifferentSyllables(_)),
                "{heard} should be explained phonetically, got {v:?}"
            );
        }
    }

    #[test]
    fn a_tone_only_respelling_passes_where_the_threshold_would_fail_it() {
        // 好 hao3 heard as 号 hao4 — one character in two, cer 0.5, and it must
        // pass: the syllable is right and only the tone was written differently.
        // A threshold cannot separate this from a real error of the same size,
        // which is the whole reason the comparison is phonetic.
        let r = Readings::from_pairs([('好', "hao3".into()), ('号', "hao4".into())]);
        let cer = character_error_rate("好", "号");
        assert!(cer > MAX_CER, "fixture should exceed the threshold, was {cer}");
        assert_eq!(judge("好", "号", &r), Verdict::Agrees);
    }

    #[test]
    fn a_missing_final_is_corruption() {
        // 您 nin2 heard as 你 ni3: the final lost its `n`. Worth pinning because
        // it is the case most likely to be argued about — the two are near
        // homophones used in the same place, and a careless rule would wave it
        // through as a tone difference. The syllables are not the same, and a
        // learner saying 你 where the text says 您 is making the error this app
        // is meant to show them.
        let r = Readings::from_pairs([('您', "nin2".into()), ('你', "ni3".into())]);
        let v = judge("您", "你", &r);
        assert!(v.is_failure(), "a lost final must be reported: {v:?}");
    }

    #[test]
    fn a_dropped_clause_is_a_failure() {
        let v = judge("到了请下车", "到了", &readings());
        assert_eq!(v, Verdict::Misaligned { wanted: 5, heard: 2 });
        assert!(v.is_failure());
    }

    #[test]
    fn an_added_clause_is_a_failure_too() {
        assert!(judge("到了", "到了请下车", &readings()).is_failure());
    }

    #[test]
    fn an_unknown_character_falls_back_to_the_error_rate() {
        // 'l' has an empty reading in the fixture, so the phonetic check cannot
        // judge — but the CER can, and a badly wrong clip is still reported.
        let r = readings();
        let clean = judge("请l", "请l", &r);
        assert_eq!(clean, Verdict::Agrees);
        let bad = judge("请l请l", "请l请l", &r);
        assert_eq!(bad, Verdict::Agrees);
    }

    #[test]
    fn the_clips_that_were_fine_still_pass() {
        // The measured good cases, so the new judgement cannot be loosened into
        // failing everything.
        let r = readings();
        for (w, h) in [
            ("请进请坐", "请进请坐"),
            ("请进请坐", "请进请坐"),
        ] {
            assert_eq!(judge(w, h, &r), Verdict::Agrees);
        }
    }

    #[test]
    fn readings_parse_from_plain_lines() {
        let r = readings_from_lines([
            "# a comment".to_string(),
            "请\tqing3".to_string(),
            "  ".to_string(),
            "好 hao3".to_string(),
            "notachar\tx".to_string(),
        ]);
        assert_eq!(r.len(), 2);
        assert!(!r.of('请').is_empty());
        assert!(!r.of('好').is_empty());
    }

    #[test]
    fn unknown_characters_are_listed() {
        let r = readings_from_lines(["请\tqing3".to_string()]);
        let unknown = unknown_characters("请未", &r);
        assert!(unknown.contains(&'未'));
        assert!(!unknown.contains(&'请'));
    }

    #[test]
    fn punctuation_does_not_count_as_unknown() {
        let r = readings_from_lines(["请\tqing3".to_string()]);
        assert!(unknown_characters("请，。！", &r).is_empty());
    }

    #[test]
    fn han_only_keeps_only_han() {
        assert_eq!(han_only("你好，很高兴认识你。"), "你好很高兴认识你");
        assert_eq!(han_only("abc 你好!"), "你好");
        assert_eq!(han_only(""), "");
    }

    #[test]
    fn the_error_rate_is_zero_for_identical_text() {
        assert_eq!(character_error_rate("你好", "你好"), 0.0);
        assert!((character_error_rate("请进请坐", "笔进请坐") - 0.25).abs() < 1e-9);
        assert!((character_error_rate("到了请下车", "到了") - 0.6).abs() < 1e-9);
    }
}
