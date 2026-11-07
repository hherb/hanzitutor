//! The shipped word list's readings, checked against the committed artifact.
//!
//! A word's reading is stored as one string (`xuéxí`), and every screen that
//! colours a word, scores its tones or reads a transcription against it divides
//! that string back into one syllable per character with
//! [`hanzi_core::pinyin::syllables`]. So the artifact and the splitter are one
//! contract: a reading the splitter cannot divide, or divides wrongly, is a word
//! the learner is shown the wrong syllables for or cannot practise at all.
//!
//! Issue #19 is what this pins. The pipeline used to remove the source's spaces,
//! after which about 130 words split at the wrong place (全国 → quáng + uó) and
//! 131 could not be divided (女儿 `nǚér`, 可爱 `kěài`, and most erhua words).

use hanzi_core::pinyin::{aligned_reading, base, syllables, tone_target};
use hanzi_core::Dataset;

fn dataset() -> Dataset {
    Dataset::from_gzip_bytes(include_bytes!("../data/hanzi.bin.gz"))
        .expect("the committed artifact decodes")
}

/// An erhua syllable: one that ends in the `r` written for a following 儿,
/// which is every syllable ending in `r` except `er` itself (儿, 二, 而 …).
fn is_erhua(syllable: &str) -> bool {
    let plain = base(syllable);
    plain.ends_with('r') && plain != "er"
}

/// Pair a reading's syllables with a word's characters, letting an erhua
/// syllable cover the 儿 that follows it. `None` when they do not pair up.
fn align(text: &str, reading: &str) -> Option<Vec<String>> {
    let characters: Vec<char> = text.chars().collect();
    let list = syllables(reading)?;
    let mut at = 0;
    let mut out = Vec::new();
    for syllable in list {
        characters.get(at)?;
        at += 1;
        if is_erhua(&syllable.text) {
            if characters.get(at) != Some(&'儿') {
                return None;
            }
            at += 1;
        }
        out.push(syllable.text);
    }
    (at == characters.len()).then_some(out)
}

#[test]
fn every_word_reading_divides_one_syllable_per_character() {
    let dataset = dataset();
    let mut wrong = Vec::new();
    let mut erhua = 0;
    for word in dataset.words() {
        match align(&word.text, &word.pinyin) {
            None => wrong.push(format!("{} {}", word.text, word.pinyin)),
            Some(list) if list.len() != word.text.chars().count() => erhua += 1,
            Some(_) => {
                // Without erhua the reading must also make a tone target, which
                // is the same division seen from the tone screen.
                if tone_target(&word.text, &word.pinyin).is_none() {
                    wrong.push(format!("{} {} (no tone target)", word.text, word.pinyin));
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} word readings do not divide one syllable per character:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    // The erhua words are the only ones allowed to have fewer syllables than
    // characters: their 儿 is the `r` of the syllable before it, not a syllable
    // of its own, so they divide correctly but still make no tone target — that
    // is one syllable per character, and pairing them would need a target that
    // can say one syllable covers two characters. Pinned, so a refresh of the
    // word list that changes how many there are is noticed.
    assert_eq!(erhua, 57, "erhua words in the HSK lists");
}

#[test]
fn word_readings_hold_no_stray_markup() {
    // `r5` was CC-CEDICT's numbered neutral tone, `u:` its spelling of `ü` and
    // `˙` a neutral-tone dot; none of them is pinyin a learner should be shown.
    let dataset = dataset();
    let stray: Vec<String> = dataset
        .words()
        .iter()
        .filter(|w| {
            w.pinyin
                .chars()
                .any(|c| c.is_ascii_digit() || matches!(c, ':' | '˙' | ' '))
        })
        .map(|w| format!("{} {}", w.text, w.pinyin))
        .collect();
    assert!(stray.is_empty(), "{}", stray.join("\n"));
}

#[test]
fn an_apostrophe_is_only_written_before_a_vowel_initial_syllable() {
    // The orthography's rule, and the only place the artifact uses one: the
    // reading a learner sees is the reading a dictionary prints.
    let dataset = dataset();
    let mut misplaced = Vec::new();
    for word in dataset.words() {
        let chars: Vec<char> = word.pinyin.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if *c != '\'' {
                continue;
            }
            let next = chars.get(i + 1).map(|c| base(&c.to_string()));
            if i == 0 || !matches!(next.as_deref(), Some("a" | "o" | "e")) {
                misplaced.push(format!("{} {}", word.text, word.pinyin));
            }
        }
    }
    assert!(misplaced.is_empty(), "{}", misplaced.join("\n"));
}

#[test]
fn the_words_issue_19_reported_read_correctly() {
    let dataset = dataset();
    let split = |text: &str| -> Vec<String> {
        let word = dataset
            .word(text)
            .unwrap_or_else(|| panic!("{text} is in the word list"));
        aligned_reading(text, &word.pinyin)
            .unwrap_or_else(|| panic!("{text} ({}) aligns", word.pinyin))
            .into_iter()
            .map(|s| s.text)
            .collect()
    };
    assert_eq!(split("全国"), ["quán", "guó"]);
    assert_eq!(split("见过"), ["jiàn", "guò"]);
    assert_eq!(split("难过"), ["nán", "guò"]);
    assert_eq!(split("参观"), ["cān", "guān"]);
    assert_eq!(split("晚安"), ["wǎn", "ān"]);
    assert_eq!(split("办公室"), ["bàn", "gōng", "shì"]);
    assert_eq!(split("人工"), ["rén", "gōng"]);
    assert_eq!(split("然而"), ["rán", "ér"]);
    assert_eq!(split("女儿"), ["nǚ", "ér"]);
    assert_eq!(split("可爱"), ["kě", "ài"]);
    assert_eq!(split("第二"), ["dì", "èr"]);

    let erhua = |text: &str| -> Vec<String> {
        let word = dataset.word(text).expect("in the word list");
        align(text, &word.pinyin).unwrap_or_else(|| panic!("{text} ({}) aligns", word.pinyin))
    };
    assert_eq!(erhua("这儿"), ["zhèr"]);
    assert_eq!(erhua("一点儿"), ["yī", "diǎnr"]);
    assert_eq!(erhua("玩儿"), ["wánr"]);
    assert_eq!(erhua("不一会儿"), ["bù", "yī", "huǐr"]);
}

#[test]
fn no_boundary_falls_between_the_wrong_letters() {
    // The count can come out right with the boundary in the wrong place —
    // 全国 as quáng + uó is two syllables for two characters — so the syllables
    // are checked against what the character dataset says each character can be
    // read as, tone aside. That alone is too strict: the dataset lists only the
    // commonest readings, so a polyphone's rarer reading in a word (地 as `dì`)
    // is not in it. But a boundary in the wrong place always spoils **both**
    // syllables beside it, and two neighbouring polyphones each on a reading the
    // dataset lacks is rare enough to name, so that is what is flagged.
    let dataset = dataset();
    let mut shifted = Vec::new();
    for word in dataset.words() {
        let Some(list) = align(&word.text, &word.pinyin) else {
            continue;
        };
        let chars: Vec<char> = word.text.chars().collect();
        let mut at = 0;
        let mut known = Vec::new();
        for syllable in &list {
            let mut plain = base(syllable);
            let ch = chars[at];
            at += 1;
            if is_erhua(syllable) {
                plain.pop();
                at += 1;
            }
            known.push(
                dataset
                    .get(ch)
                    .is_some_and(|c| c.pinyin.iter().any(|r| base(r) == plain)),
            );
        }
        // Read correctly, and each on a reading the character list lacks:
        // 的 as `dì` beside 地 as `dì`, and 长 as `cháng` beside it.
        if matches!(word.text.as_str(), "目的地" | "天长地久") {
            continue;
        }
        if known.windows(2).any(|pair| !pair[0] && !pair[1]) {
            shifted.push(format!("{} {} {:?}", word.text, word.pinyin, list));
        }
    }
    assert!(
        shifted.is_empty(),
        "{} words with a neighbouring pair of unknown syllables:\n{}",
        shifted.len(),
        shifted.join("\n")
    );
}

#[test]
fn the_ordinals_issue_20_reported_keep_yi_in_its_first_tone() {
    // 第一线 and 第一手 are in the word list, and their 一 is an ordinal: the
    // learner is scored against yī, not the yí or yì of a counted 一.
    let dataset = dataset();
    let spoken = |text: &str| -> Vec<u8> {
        let word = dataset
            .word(text)
            .unwrap_or_else(|| panic!("{text} is in the word list"));
        tone_target(text, &word.pinyin)
            .unwrap_or_else(|| panic!("{text} ({}) makes a target", word.pinyin))
            .spoken()
    };
    assert_eq!(spoken("第一线"), [4, 1, 4]);
    assert_eq!(spoken("第一手"), [4, 1, 3]);
    // The counted 一 still changes.
    assert_eq!(spoken("一起"), [4, 3]);
}
