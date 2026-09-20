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
    /// All five do, neutral included — it is judged on being level, and what
    /// that can and cannot see is `tone::NEUTRAL_LIMIT`. The range is written
    /// out here rather than imported from `tone`, which is the one place this
    /// module and that one overlap: they are kept apart on purpose (see the
    /// module notes in `tone.rs`), and a two-number range is a smaller price
    /// than the dependency. **Keep it equal to `tone::is_scorable`.**
    pub fn scorable(&self) -> usize {
        self.syllables
            .iter()
            .filter(|s| (1..=5).contains(&s.spoken))
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
/// There is no longer a second refusal for a reading whose syllables are all
/// neutral — 的, 了, 吗 and the rest of the particles. It used to be here, and it
/// was the wrong place for it: whether a tone can be *judged* is a question about
/// the analysis, which this module knows nothing about, not about pinyin. Neutral
/// tones are scored now (see `tone::NEUTRAL_LIMIT` for what that can and cannot
/// see), so the decision no longer exists to make.
pub fn tone_target(text: &str, reading: &str) -> Option<ToneTarget> {
    let characters: Vec<char> = text.chars().collect();
    let list = syllables(reading)?;
    if characters.is_empty() || list.len() != characters.len() {
        return None;
    }

    let citation: Vec<u8> = list.iter().map(|s| s.tone).collect();
    let spoken = spoken_tones(&characters, &citation);

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

/// `"2 + 3"`, or `"1 + neutral"`, for a sentence.
///
/// A neutral tone is spelled out rather than numbered. "1 + 5" reads as a fifth
/// full tone beside the four the course teaches, and a learner has no reason to
/// know that 5 means "no tone mark on the reading". The full tones stay digits,
/// because that is what the panel's own labels use.
fn join_tones(tones: &[u8]) -> String {
    tones
        .iter()
        .map(|t| {
            if *t == 5 {
                "neutral".to_string()
            } else {
                t.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

// ---------------------------------------------------------------------------
// What was heard
// ---------------------------------------------------------------------------

/// A reading with its tone taken off: `nǐ` → `ni`, `nǚ` → `nv`.
///
/// This is what a transcription is compared against a target *as*. Two reasons,
/// and they are different reasons:
///
/// - **Comparing characters does not work for single syllables.** 是, 事 and 士
///   are all `shì`, so a recogniser that transcribed the right sound as the
///   wrong character would be reported as a wrong syllable. They are the same
///   sound; that is the whole point of comparing readings.
/// - **The tone is dropped on purpose**, and not because it does not matter. It
///   matters more than anything else here — it is `tone.rs`'s entire job. It is
///   dropped because a recogniser's *reading of a character implies a tone that
///   the learner may never have produced*: the language model repairs a wrong
///   tone toward the likely word (research §6.1), so a dictionary tone attached
///   to a transcribed character is evidence about the language model, not about
///   the learner's voice. A tone score has to come from F0, and it does
///   ([`crate::tone`]).
///
/// `ü` is kept as its own letter rather than folded into `u`: 女 (`nǚ`) and 努
/// (`nǔ`) are different syllables, and treating them as the same sound would
/// call a wrong syllable right — the one outcome worse than saying nothing.
pub fn base(reading: &str) -> String {
    reading
        .chars()
        .filter_map(|ch| match ch {
            'a' | 'ā' | 'á' | 'ǎ' | 'à' => Some('a'),
            'e' | 'ē' | 'é' | 'ě' | 'è' | 'ê' => Some('e'),
            'i' | 'ī' | 'í' | 'ǐ' | 'ì' => Some('i'),
            'o' | 'ō' | 'ó' | 'ǒ' | 'ò' => Some('o'),
            'u' | 'ū' | 'ú' | 'ǔ' | 'ù' => Some('u'),
            'ü' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' | 'v' => Some('v'),
            // The syllabic nasals: 嗯 is `ń`, a syllable with no vowel letter.
            'n' | 'ń' | 'ň' | 'ǹ' => Some('n'),
            'm' | 'ḿ' => Some('m'),
            other => other.is_ascii_alphabetic().then(|| other.to_ascii_lowercase()),
        })
        .collect()
}

/// One syllable of a transcription, beside the one the exercise asked for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeardSyllable {
    /// The syllable as heard, plain: `shi`. No tone mark — see [`base`].
    pub base: String,
    /// The syllable the exercise asked for, the same way: `si`.
    pub wanted: String,
    /// True when they are the same sound once the tone is set aside.
    pub matches: bool,
}

/// What a speech recogniser made of one recording, read against the target.
///
/// ## What this is evidence of, and what it is not
///
/// It answers *which syllables were said*, not *how well*. A recogniser carries a
/// strong language-model prior and is built to be robust to the errors a learner
/// makes, so it under-reports them — most for the learners who need feedback most
/// (research §6.1). Read it as: "the app did not hear the syllable you were asked
/// for", which is a real and useful thing to be told, and never as "your
/// pronunciation was correct", which it cannot know. The tone is scored from the
/// pitch, separately, and [`Heard::detail`] says which half is which so no one
/// has to guess.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Heard {
    /// What was transcribed, in characters. Empty when nothing was recognised.
    pub text: String,
    /// The same, as plain letters with the syllables spaced: `shi shi`.
    pub base: String,
    /// One entry per syllable the transcription divided into.
    pub syllables: Vec<HeardSyllable>,
    /// How many syllables were the sound asked for.
    pub matched: usize,
    /// True when the transcription divided into exactly as many syllables as the
    /// target has, which is what makes a per-syllable comparison meaningful.
    pub same_count: bool,
    /// One plain sentence, worded here so there is one place it is worded.
    pub detail: String,
}

impl Heard {
    /// Everything asked for was heard, syllable for syllable.
    pub fn all_matched(&self) -> bool {
        self.same_count && !self.syllables.is_empty() && self.matched == self.syllables.len()
    }
}

/// Read a transcription against the target it was meant to be.
///
/// `reading` is the transcription's own reading — the word's, from the dataset,
/// so that a polyphone comes out right, or a reading the caller composed from the
/// characters. It is taken as a parameter rather than looked up because this
/// module knows nothing about the dataset, exactly as [`tone_target`] does.
///
/// The same alignment rule as [`tone_target`] applies: a reading that does not
/// divide into one syllable per character is **refused rather than compared out
/// of step**, because a split this module got wrong would be reported to the
/// learner as a syllable they mispronounced.
pub fn heard_against(heard: &str, reading: &str, target: &ToneTarget) -> Heard {
    let characters: Vec<char> = heard.chars().collect();
    let list = syllables(reading).unwrap_or_default();
    let aligned = !characters.is_empty() && list.len() == characters.len();

    let heard_syllables: Vec<String> = if aligned {
        list.iter().map(|s| base(&s.text)).collect()
    } else {
        Vec::new()
    };
    let wanted: Vec<String> = target.syllables.iter().map(|s| base(&s.reading)).collect();

    let syllables: Vec<HeardSyllable> = heard_syllables
        .iter()
        .zip(wanted.iter())
        .map(|(base, wanted)| HeardSyllable {
            base: base.clone(),
            wanted: wanted.clone(),
            matches: base == wanted,
        })
        .collect();
    let matched = syllables.iter().filter(|s| s.matches).count();
    let same_count = aligned && heard_syllables.len() == wanted.len();
    let base_text = heard_syllables.join(" ");
    let wanted_text = wanted.join(" ");

    let detail = if heard.trim().is_empty() {
        "Nothing was recognised in that recording. The tone below is still judged \
         from the pitch, which does not need a transcription."
            .to_string()
    } else if !aligned {
        format!(
            "The transcription ({}) could not be divided into one syllable per character, \
             so it was not read against what was asked for. The tone below is unaffected.",
            heard.trim()
        )
    } else if !same_count {
        format!(
            "Heard {base_text} — {} syllable{} where {wanted_text} ({}) was asked for. \
             The tone below is still judged from the pitch.",
            wanted.len(),
            if wanted.len() == 1 { "" } else { "s" },
            wanted.len(),
        )
    } else if matched == syllables.len() {
        format!(
            "Heard {base_text}: the syllable{} asked for. What was heard is a transcription, \
             not a judgement of the tone — the tone below is measured from the pitch.",
            if syllables.len() == 1 { "" } else { "s" },
        )
    } else {
        let wrong: Vec<String> = syllables
            .iter()
            .filter(|s| !s.matches)
            .map(|s| format!("{} where {} was asked for", s.base, s.wanted))
            .collect();
        format!(
            "Heard {base_text} ({}) — {}. The tone below is measured from the pitch, not \
             from this transcription.",
            wrong.join(", "),
            if matched == 0 {
                "none of that is the syllable wanted".to_string()
            } else {
                format!(
                    "{} of {} syllables match",
                    matched,
                    syllables.len()
                )
            }
        )
    };

    Heard {
        text: heard.trim().to_string(),
        base: base_text,
        syllables,
        matched,
        same_count,
        detail,
    }
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
    fn a_reading_of_particles_is_still_a_target() {
        // 的 is the neutral tone. It used to be refused here — "real, and not
        // something to score" — which left the most common character in the
        // language as the one the tone panel would not look at. Neutral tones
        // are scored now, on being level.
        let target = tone_target("的", "de").expect("a neutral reading is a target");
        assert_eq!(target.spoken(), vec![5]);
        assert_eq!(target.scorable(), 1);
        assert!(!target.sandhi_applied);
        assert!(target.detail.contains("neutral"), "{}", target.detail);

        // A word that mixes them keeps both, and both count.
        assert_eq!(tone_target("妈妈", "māma").map(|t| t.spoken()), Some(vec![1, 5]));
        assert_eq!(tone_target("妈妈", "māma").map(|t| t.scorable()), Some(2));
    }

    #[test]
    fn sandhi_does_not_apply_across_an_apostrophe_free_join() {
        // 一 + 天 as composed readings, joined with an apostrophe by the caller,
        // still splits and still takes sandhi.
        let target = tone_target("一天", "yī'tiān").expect("joined readings still split");
        assert_eq!(target.spoken(), [4, 1]);
        assert_eq!(target.syllables.len(), 2);
    }

    #[test]
    fn a_base_reading_drops_the_tone_but_not_the_umlaut() {
        assert_eq!(base("nǐ"), "ni");
        assert_eq!(base("hǎo"), "hao");
        assert_eq!(base("shì"), "shi");
        assert_eq!(base("de"), "de");
        // 嗯 is `ń`: a syllable with no vowel letter at all.
        assert_eq!(base("ń"), "n");
        // The reason `ü` is not folded into `u`: 女 nǚ and 努 nǔ are different
        // syllables, and calling them the same sound would call a wrong syllable
        // right.
        assert_eq!(base("nǚ"), "nv");
        assert_eq!(base("nǔ"), "nu");
        assert_ne!(base("nǚ"), base("nǔ"));
    }

    #[test]
    fn hearing_what_was_asked_for_says_so() {
        let target = tone_target("你好", "nǐhǎo").expect("你好 is a target");
        let heard = heard_against("你好", "nǐhǎo", &target);
        assert!(heard.all_matched());
        assert_eq!(heard.matched, 2);
        assert_eq!(heard.base, "ni hao");
        assert_eq!(heard.text, "你好");
        assert!(heard.same_count);
        // The wording has to keep the transcription and the tone apart: the
        // recogniser did not judge the tone and must not be presented as if it
        // had.
        assert!(heard.detail.contains("ni hao"), "{}", heard.detail);
        assert!(heard.detail.contains("not a judgement of the tone"), "{}", heard.detail);
    }

    #[test]
    fn a_homophone_is_the_same_syllable_and_a_different_one_is_not() {
        // 是 and 事 are both `shì`. A recogniser picking the other character has
        // still heard the right syllable, which is why the comparison is by
        // reading and not by character — this is the whole reason `heard_against`
        // exists.
        let target = tone_target("是", "shì").expect("是 is a target");
        let homophone = heard_against("事", "shì", &target);
        assert!(homophone.all_matched(), "{}", homophone.detail);
        assert_ne!(homophone.text, "是");

        // sì and shì are different syllables, and that is what a learner needs
        // to be told.
        let target = tone_target("四", "sì").expect("四 is a target");
        let wrong = heard_against("是", "shì", &target);
        assert!(!wrong.all_matched());
        assert_eq!(wrong.matched, 0);
        assert_eq!(wrong.syllables[0].base, "shi");
        assert_eq!(wrong.syllables[0].wanted, "si");
        assert!(wrong.detail.contains("shi where si was asked for"), "{}", wrong.detail);
    }

    #[test]
    fn the_wrong_number_of_syllables_is_reported_rather_than_aligned() {
        let target = tone_target("你好", "nǐhǎo").expect("你好 is a target");
        let heard = heard_against("你", "nǐ", &target);
        assert!(!heard.same_count);
        assert!(!heard.all_matched());
        assert!(heard.detail.contains("2 syllables"), "{}", heard.detail);
        assert!(heard.detail.contains("ni"), "{}", heard.detail);
    }

    #[test]
    fn a_transcription_that_cannot_be_split_is_refused() {
        // Two characters, one syllable: comparing them in order would report a
        // syllable the learner never said. The same refusal `tone_target` makes.
        let target = tone_target("你好", "nǐhǎo").expect("你好 is a target");
        let heard = heard_against("你好", "nǐ", &target);
        assert!(!heard.same_count);
        assert!(heard.syllables.is_empty());
        assert!(heard.detail.contains("could not be divided"), "{}", heard.detail);
    }

    #[test]
    fn hearing_nothing_is_said_plainly_and_leaves_the_tone_alone() {
        let target = tone_target("是", "shì").expect("是 is a target");
        let heard = heard_against("", "", &target);
        assert!(!heard.all_matched());
        assert!(heard.text.is_empty());
        assert!(heard.detail.contains("Nothing was recognised"), "{}", heard.detail);
        assert!(heard.detail.contains("tone"), "{}", heard.detail);
    }
}
