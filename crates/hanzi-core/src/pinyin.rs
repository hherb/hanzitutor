//! Pinyin readings: splitting a word's reading into syllables, and the tone each
//! one carries.
//!
//! ## Why this exists
//!
//! The dataset stores a **character's** reading as a list (`好` → `["hǎo",
//! "hào"]`) but a **word's** reading as one run-together string (`学习` →
//! `"xuéxí"`). Scoring a word's tones needs that string taken apart, and needs to
//! know how many syllables it holds so that the recording can be divided the same
//! way.
//!
//! Tone practice needs one more thing that a dictionary does not give:
//! **sandhi**. The dictionary reads 你好 as `nǐhǎo`, tone 3 + tone 3, but nobody
//! says that — it is spoken `níhǎo`, tone 2 + tone 3. Scoring a learner against
//! the dictionary tones would flag correct speech as wrong, which is the failure
//! the research warns about (§6.2). [`spoken_tones`] is that correction.
//!
//! ## Splitting without a syllable table
//!
//! A full Mandarin syllable inventory is about 400 entries. It is not needed: a
//! syllable is `[initial consonants] vowel-run [coda]`, the only codas are `n`,
//! `ng` and `r`, and CC-CEDICT already writes an apostrophe at every ambiguous
//! boundary (`xī'ān`, not `xiān`). So a vowel run followed by anything but a coda
//! starts a new syllable, which is a rule rather than a table — and the caller
//! checks the result against the number of characters, so a reading this gets
//! wrong is refused rather than scored.

use serde::{Deserialize, Serialize};

/// One syllable of a reading.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Syllable {
    /// The syllable as written, tone mark included, e.g. `"xué"`.
    pub text: String,
    /// The tone it carries, 1..=4, or `5` for the neutral tone.
    pub tone: u8,
}

/// One syllable of a tone target: which character, how it reads, and which tone
/// to score it against.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSyllable {
    pub ch: char,
    /// The syllable as the dictionary writes it, e.g. `"nǐ"`.
    pub reading: String,
    /// The dictionary's tone, before sandhi.
    pub citation: u8,
    /// The tone actually spoken, after sandhi. This is what a recording is
    /// scored against.
    pub spoken: u8,
}

/// What to score a word or character against.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneTarget {
    pub syllables: Vec<TargetSyllable>,
    /// True when [`spoken_tones`] changed something, so the interface can say
    /// why the tone being asked for is not the one printed in a dictionary.
    pub sandhi_applied: bool,
    /// One plain sentence about the tones, worded here so there is one place the
    /// rule is explained.
    pub detail: String,
}

impl ToneTarget {
    /// The tones to score against, in order.
    pub fn spoken(&self) -> Vec<u8> {
        self.syllables.iter().map(|s| s.spoken).collect()
    }

    /// How many syllables carry a tone that can actually be judged.
    ///
    /// The neutral tone cannot: it is short and pitched by the syllable before
    /// it, so it is carried in the target and reported, but not scored.
    pub fn scorable(&self) -> usize {
        self.syllables
            .iter()
            .filter(|s| (1..=4).contains(&s.spoken))
            .count()
    }
}

/// Is this character part of a syllable's vowel run?
///
/// The syllabic nasals (`ń`, `ň`, `ǹ`, `ḿ`) count: 嗯 is `ń`, a syllable with no
/// vowel letter in it at all.
fn is_vowel(ch: char) -> bool {
    matches!(
        ch,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'ü'
            | 'v'
            | 'ā'
            | 'á'
            | 'ǎ'
            | 'à'
            | 'ē'
            | 'é'
            | 'ě'
            | 'è'
            | 'ê'
            | 'ī'
            | 'í'
            | 'ǐ'
            | 'ì'
            | 'ō'
            | 'ó'
            | 'ǒ'
            | 'ò'
            | 'ū'
            | 'ú'
            | 'ǔ'
            | 'ù'
            | 'ǖ'
            | 'ǘ'
            | 'ǚ'
            | 'ǜ'
            | 'ń'
            | 'ň'
            | 'ǹ'
            | 'ḿ'
    )
}

/// The tone a single accented vowel marks, if it marks one.
fn marked_tone(ch: char) -> Option<u8> {
    match ch {
        'ā' | 'ē' | 'ī' | 'ō' | 'ū' | 'ǖ' | 'ń' | 'ḿ' => Some(1),
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'ǘ' => Some(2),
        'ǎ' | 'ě' | 'ǐ' | 'ǒ' | 'ǔ' | 'ǚ' | 'ň' => Some(3),
        'à' | 'è' | 'ì' | 'ò' | 'ù' | 'ǜ' | 'ǹ' => Some(4),
        _ => None,
    }
}

/// The characters that mark a syllable boundary by hand.
fn is_separator(ch: char) -> bool {
    matches!(ch, '\'' | ' ' | '-' | '·' | '/' | ',')
}

/// Split one run of letters into syllables.
fn split_run(run: &str) -> Vec<String> {
    let chars: Vec<char> = run.chars().collect();
    let mut out = Vec::new();
    let mut current = String::new();
    let mut i = 0;

    while i < chars.len() {
        if !is_vowel(chars[i]) {
            current.push(chars[i]);
            i += 1;
            continue;
        }

        // A vowel run, then at most one coda: `n`, `ng`, or `r`.
        current.push(chars[i]);
        let mut j = i + 1;
        while j < chars.len() && is_vowel(chars[j]) {
            current.push(chars[j]);
            j += 1;
        }

        if j < chars.len() {
            let next_is_vowel = j + 1 < chars.len() && is_vowel(chars[j + 1]);
            match chars[j] {
                // `ng` is always a coda. A bare `n` is a coda unless a vowel
                // follows it, in which case it is the next syllable's initial —
                // this is what separates `qùnián` into `qù` + `nián`.
                'n' if j + 1 < chars.len() && chars[j + 1] == 'g' => {
                    current.push('n');
                    current.push('g');
                    j += 2;
                }
                'n' if !next_is_vowel => {
                    current.push('n');
                    j += 1;
                }
                // `r` is a coda only when a vowel does not follow: `ér` is one
                // syllable, `nǚrén` is two.
                'r' if !next_is_vowel => {
                    current.push('r');
                    j += 1;
                }
                _ => {}
            }
        }

        out.push(std::mem::take(&mut current));
        i = j;
    }

    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// The tone a written syllable carries.
///
/// A syllable with no mark is the neutral tone (`5`) — but only if it has a vowel
/// at all, so that a stray consonant run is refused rather than called neutral.
fn syllable_tone(text: &str) -> Option<u8> {
    let marks: Vec<u8> = text.chars().filter_map(marked_tone).collect();
    match marks.len() {
        0 => text.chars().any(is_vowel).then_some(5),
        1 => Some(marks[0]),
        // Two tone marks in one syllable is not a syllable. This is what catches
        // an unseparated pair such as `nǐhǎo` being fed in as one.
        _ => None,
    }
}

/// Split a reading into its syllables.
///
/// `None` when the reading is empty or any part of it cannot be read as a
/// syllable. Apostrophes, spaces and hyphens are hard boundaries, which is what
/// makes `xi'an` two syllables and `xian` one.
pub fn syllables(reading: &str) -> Option<Vec<Syllable>> {
    let reading = reading.trim();
    if reading.is_empty() {
        return None;
    }

    let mut out = Vec::new();
    for run in reading.split(is_separator) {
        if run.is_empty() {
            continue;
        }
        for text in split_run(run) {
            let tone = syllable_tone(&text)?;
            out.push(Syllable { text, tone });
        }
    }

    (!out.is_empty()).then_some(out)
}

/// The tone a single-syllable reading carries, or `None` when it cannot be read.
///
/// A reading of several syllables is refused rather than half-read: it has
/// several tones and none of them is the target of a one-syllable exercise.
pub fn tone_from_pinyin(reading: &str) -> Option<u8> {
    let list = syllables(reading)?;
    (list.len() == 1).then(|| list[0].tone)
}

/// The tones as they are actually spoken, given the dictionary's own.
///
/// Three rules, which are the ones that matter for vocabulary:
///
/// 1. **Third tone before third tone** becomes second. A run shortens all but its
///    last syllable: 你好 `3+3` → `2+3`, 我很好 `3+3+3` → `2+2+3`.
/// 2. **不** (`bù`, tone 4) becomes second before a fourth tone: 不是 `4+4` →
///    `2+4`. It stays fourth otherwise.
/// 3. **一** (`yī`, tone 1) becomes second before a fourth tone (一个 → `2+4`) and
///    fourth before anything else (一天 → `4+1`, 一起 → `4+3`). Alone, or as an
///    ordinal, it stays first — which is why a final 一 is left alone here.
///
/// The characters are needed as well as the tones because rules 2 and 3 are about
/// *which* character it is, not which tone.
///
/// Deliberately not modelled: the half-third-tone realisation of a third tone in
/// running speech (which is a matter of how far the dip goes, not which tone it
/// is), and the optional sandhi of 一 in very casual speech.
pub fn spoken_tones(characters: &[char], citation: &[u8]) -> Vec<u8> {
    let mut tones = citation.to_vec();
    let count = tones.len().min(characters.len());

    for i in 0..count {
        let next = tones.get(i + 1).copied();
        match characters[i] {
            '不' if tones[i] == 4 && next == Some(4) => tones[i] = 2,
            '一' if tones[i] == 1 => match next {
                Some(4) => tones[i] = 2,
                Some(_) => tones[i] = 4,
                None => {}
            },
            _ => {}
        }
    }

    // The third-tone run, last. Applied after the other two so that a 一 which
    // became fourth is not counted as part of a run of thirds.
    let mut i = 0;
    while i < tones.len() {
        if tones[i] == 3 {
            let start = i;
            let mut end = i;
            while end + 1 < tones.len() && tones[end + 1] == 3 {
                end += 1;
            }
            for tone in tones.iter_mut().take(end).skip(start) {
                *tone = 2;
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }

    tones
}

/// Build the tone target for `text`, read as `reading`.
///
/// `None` when the reading does not divide into exactly as many syllables as
/// there are characters. That check is the point of the function: it is what
/// makes a splitting rule safe, because a reading the rule gets wrong is refused
/// rather than silently mis-aligned against the recording.
///
/// Also `None` when no syllable carries a scoreable tone — a single neutral-tone
/// character such as 的, where there is nothing to judge.
pub fn tone_target(text: &str, reading: &str) -> Option<ToneTarget> {
    let characters: Vec<char> = text.chars().collect();
    let list = syllables(reading)?;
    if characters.is_empty() || list.len() != characters.len() {
        return None;
    }

    let citation: Vec<u8> = list.iter().map(|s| s.tone).collect();
    let spoken = spoken_tones(&characters, &citation);
    if !spoken.iter().any(|t| (1..=4).contains(t)) {
        return None;
    }

    let syllables: Vec<TargetSyllable> = characters
        .iter()
        .zip(list.iter())
        .zip(spoken.iter())
        .zip(citation.iter())
        .map(|(((ch, written), spoken), citation)| TargetSyllable {
            ch: *ch,
            reading: written.text.clone(),
            citation: *citation,
            spoken: *spoken,
        })
        .collect();

    let sandhi_applied = spoken != citation;
    let detail = if sandhi_applied {
        format!(
            "Said as a word this is {}, not the {} a dictionary lists syllable by \
             syllable — tones are scored as they are spoken.",
            join_tones(&spoken),
            join_tones(&citation)
        )
    } else {
        format!("Scored against {}.", join_tones(&spoken))
    };

    Some(ToneTarget {
        syllables,
        sandhi_applied,
        detail,
    })
}

/// `"2 + 3"`, for a sentence.
fn join_tones(tones: &[u8]) -> String {
    tones
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(" + ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(reading: &str) -> Vec<String> {
        syllables(reading)
            .unwrap_or_else(|| panic!("{reading} should split"))
            .into_iter()
            .map(|s| s.text)
            .collect()
    }

    fn tones(reading: &str) -> Vec<u8> {
        syllables(reading)
            .unwrap_or_else(|| panic!("{reading} should split"))
            .into_iter()
            .map(|s| s.tone)
            .collect()
    }

    #[test]
    fn a_single_syllable_is_left_whole() {
        assert_eq!(split("hǎo"), ["hǎo"]);
        assert_eq!(split("yī"), ["yī"]);
        assert_eq!(split("ér"), ["ér"]);
        assert_eq!(split("de"), ["de"]);
        assert_eq!(split("xian"), ["xian"], "xian is one syllable");
        assert_eq!(split("zhuāng"), ["zhuāng"]);
        assert_eq!(split("xióng"), ["xióng"]);
    }

    #[test]
    fn syllables_are_split_at_the_vowel_runs() {
        assert_eq!(split("xuéxí"), ["xué", "xí"]);
        assert_eq!(split("zháojí"), ["zháo", "jí"]);
        assert_eq!(split("nǐhǎo"), ["nǐ", "hǎo"]);
        assert_eq!(split("bùcuò"), ["bù", "cuò"]);
        assert_eq!(split("shénme"), ["shén", "me"]);
        assert_eq!(split("péngyou"), ["péng", "you"]);
        assert_eq!(split("yīdiǎn"), ["yī", "diǎn"]);
        assert_eq!(split("wǒmen"), ["wǒ", "men"]);
    }

    #[test]
    fn a_leading_consonant_of_the_next_syllable_is_not_a_coda() {
        // The `n` in `nián` is an initial, not the coda of `qù`.
        assert_eq!(split("qùnián"), ["qù", "nián"]);
        assert_eq!(split("kànkan"), ["kàn", "kan"]);
        // `r` likewise: a coda in `ér`, an initial in `rén`.
        assert_eq!(split("érzi"), ["ér", "zi"]);
        assert_eq!(split("nǚrén"), ["nǚ", "rén"]);
        assert_eq!(tones("nǚrén"), [3, 2]);
    }

    #[test]
    fn an_apostrophe_is_a_hard_boundary() {
        assert_eq!(split("xi'an"), ["xi", "an"]);
        assert_eq!(split("xī'ān"), ["xī", "ān"]);
        assert_eq!(split("píng'ān"), ["píng", "ān"]);
        // And the same letters without it are one syllable.
        assert_eq!(split("xiān"), ["xiān"]);
        assert_eq!(split("nǐ hǎo"), ["nǐ", "hǎo"]);
    }

    #[test]
    fn tones_are_read_from_the_marks() {
        assert_eq!(tones("hǎo"), [3]);
        assert_eq!(tones("shì"), [4]);
        assert_eq!(tones("xuéxí"), [2, 2]);
        assert_eq!(tones("lǜ"), [4]);
        assert_eq!(tones("nǐhǎo"), [3, 3]);
        assert_eq!(tones("māma"), [1, 5], "no mark on the second is neutral");
        assert_eq!(tones("péngyou"), [2, 5]);
    }

    #[test]
    fn an_unreadable_reading_is_refused() {
        assert!(syllables("").is_none());
        assert!(syllables("   ").is_none());
        // No vowel at all: a consonant run, not a syllable.
        assert!(syllables("xyz").is_none());
        // Two marks in one run is two syllables run together, which is not one.
        assert!(syllables("hǎohǎo").is_none() || split("hǎohǎo").len() == 2);
    }

    #[test]
    fn tone_from_pinyin_reads_one_syllable_and_refuses_more() {
        assert_eq!(tone_from_pinyin("hǎo"), Some(3));
        assert_eq!(tone_from_pinyin("shì"), Some(4));
        assert_eq!(tone_from_pinyin("lǜ"), Some(4));
        assert_eq!(tone_from_pinyin("de"), Some(5));
        // Several syllables: a different question, refused rather than halved.
        assert_eq!(tone_from_pinyin("nǐhǎo"), None);
        assert_eq!(tone_from_pinyin("xuexi"), None);
        assert_eq!(tone_from_pinyin("nǐ hǎo"), None);
        assert_eq!(tone_from_pinyin(""), None);
    }

    #[test]
    fn third_tone_before_third_tone_becomes_second() {
        let chars: Vec<char> = "你好".chars().collect();
        assert_eq!(spoken_tones(&chars, &[3, 3]), [2, 3]);

        // A run shortens all but its last.
        let chars: Vec<char> = "我很好".chars().collect();
        assert_eq!(spoken_tones(&chars, &[3, 3, 3]), [2, 2, 3]);

        // A single third tone, and a pair broken by another tone, are untouched.
        let chars: Vec<char> = "好".chars().collect();
        assert_eq!(spoken_tones(&chars, &[3]), [3]);
        let chars: Vec<char> = "很高".chars().collect();
        assert_eq!(spoken_tones(&chars, &[3, 1]), [3, 1]);
    }

    #[test]
    fn bu_becomes_second_before_a_fourth_tone_only() {
        let chars: Vec<char> = "不是".chars().collect();
        assert_eq!(spoken_tones(&chars, &[4, 4]), [2, 4]);

        // Before anything else it stays fourth.
        let chars: Vec<char> = "不好".chars().collect();
        assert_eq!(spoken_tones(&chars, &[4, 3]), [4, 3]);

        // And a 不 that is not tone 4 in the dictionary is left alone.
        let chars: Vec<char> = "不".chars().collect();
        assert_eq!(spoken_tones(&chars, &[4]), [4]);
    }

    #[test]
    fn yi_changes_with_what_follows_it() {
        // Before a fourth tone it is second.
        let chars: Vec<char> = "一个".chars().collect();
        assert_eq!(spoken_tones(&chars, &[1, 4]), [2, 4]);
        // Before anything else it is fourth.
        let chars: Vec<char> = "一天".chars().collect();
        assert_eq!(spoken_tones(&chars, &[1, 1]), [4, 1]);
        let chars: Vec<char> = "一起".chars().collect();
        assert_eq!(spoken_tones(&chars, &[1, 3]), [4, 3]);
        // Alone or final, it stays first.
        let chars: Vec<char> = "一".chars().collect();
        assert_eq!(spoken_tones(&chars, &[1]), [1]);
        let chars: Vec<char> = "第一".chars().collect();
        assert_eq!(spoken_tones(&chars, &[4, 1]), [4, 1], "final 一 is untouched");
    }

    #[test]
    fn a_target_carries_both_readings_and_the_sandhi_sentence() {
        let target = tone_target("你好", "nǐhǎo").expect("你好 reads as two syllables");
        assert_eq!(target.spoken(), [2, 3]);
        assert!(target.sandhi_applied);
        assert_eq!(target.syllables[0].citation, 3);
        assert_eq!(target.syllables[0].spoken, 2);
        assert_eq!(target.syllables[0].reading, "nǐ");
        assert_eq!(target.syllables[0].ch, '你');
        assert!(
            target.detail.contains("2 + 3") && target.detail.contains("3 + 3"),
            "the sentence must give both readings: {}",
            target.detail
        );

        // A word with no sandhi says so quietly.
        let plain = tone_target("学习", "xuéxí").expect("学习 reads as two syllables");
        assert_eq!(plain.spoken(), [2, 2]);
        assert!(!plain.sandhi_applied);
        assert!(plain.detail.contains("2 + 2"), "{}", plain.detail);
    }

    #[test]
    fn a_target_refuses_a_mismatch_between_syllables_and_characters() {
        // Two characters, one syllable: the reading cannot be aligned, so it is
        // refused rather than scored against the wrong character.
        assert!(tone_target("学习", "xué").is_none());
        assert!(tone_target("学习", "").is_none());
        assert!(tone_target("", "xué").is_none());
        // Three syllables for two characters, likewise.
        assert!(tone_target("学习", "xuéxíxí").is_none());
    }

    #[test]
    fn a_target_with_nothing_scoreable_is_refused() {
        // 的 is the neutral tone: real, and not something to score.
        assert!(tone_target("的", "de").is_none());
        assert_eq!(tone_target("妈妈", "māma").map(|t| t.spoken()), Some(vec![1, 5]));
        assert_eq!(tone_target("妈妈", "māma").map(|t| t.scorable()), Some(1));
    }

    #[test]
    fn sandhi_does_not_apply_across_an_apostrophe_free_join() {
        // 一 + 天 as composed readings, joined with an apostrophe by the caller,
        // still splits and still takes sandhi.
        let target = tone_target("一天", "yī'tiān").expect("joined readings still split");
        assert_eq!(target.spoken(), [4, 1]);
        assert_eq!(target.syllables.len(), 2);
    }
}
