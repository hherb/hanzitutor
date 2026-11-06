//! The input engine, checked against every answer the shipped course asks for.
//!
//! The unit tests in `input.rs` hold the rules one example at a time; these hold
//! them over the **committed** grammar and vocabulary artifacts, so that a rule
//! which is right in isolation and wrong on the course's own data is caught here.
//! Two sources of romaji are used, and each says something different:
//!
//! * **The corpus's own.** A grammar point carries the romaji the corpus wrote for
//!   it (`wake de wa nai`, `o kudasai`), which is what the card shows the learner.
//!   Where a production blank is the whole pattern — 650 of the 903 — that romaji
//!   is the blank's, and a learner who copies it, with its spaces or without them,
//!   must be marked right. Before issue #21, 681 of those 1,292 typings were
//!   marked wrong: the spaces made every multi-word answer unreadable, and the
//!   particles は and を were accepted only as `ha` and `wo`.
//! * **Standard Hepburn, composed here.** The vocabulary carries no romaji, and
//!   most blanks are a lump of their pattern rather than the whole of it, so every
//!   answer is also typed the way a textbook spells it — `onna` for おんな, `matcha`
//!   for まっちゃ, `ti` for ティ — by [`hepburn`] below, which reads the readings
//!   table rather than the input engine so the two cannot agree by construction.
//!   An answer containing ー is left out and counted: a long-vowel mark is a
//!   spelling the engine deliberately does not type (see `input.rs`).

use nihongo_core::{reading, yoon, GrammarDataset, Script, WordDataset};

fn grammar() -> GrammarDataset {
    GrammarDataset::from_gzip_bytes(include_bytes!("../data/grammar.bin.gz"))
        .expect("the committed grammar artifact decodes")
}

fn words() -> WordDataset {
    WordDataset::from_gzip_bytes(include_bytes!("../data/words.bin.gz"))
        .expect("the committed words artifact decodes")
}

/// The katakana digraphs a loanword is written with, in the Hepburn spelling a
/// textbook gives them — not the IME's (`thi`, `twu`), which is the point.
const EXTENDED: &[(&str, &str)] = &[
    ("ゔぁ", "va"),
    ("ゔぃ", "vi"),
    ("ゔぇ", "ve"),
    ("ゔぉ", "vo"),
    ("ふぁ", "fa"),
    ("ふぃ", "fi"),
    ("ふぇ", "fe"),
    ("ふぉ", "fo"),
    ("ふゅ", "fyu"),
    ("しぇ", "she"),
    ("じぇ", "je"),
    ("ちぇ", "che"),
    ("てぃ", "ti"),
    ("でぃ", "di"),
    ("てゅ", "tyu"),
    ("でゅ", "dyu"),
    ("とぅ", "tu"),
    ("どぅ", "du"),
    ("うぃ", "wi"),
    ("うぇ", "we"),
    ("うぉ", "wo"),
    ("いぇ", "ye"),
    ("くぁ", "kwa"),
    ("ぐぁ", "gwa"),
    ("つぁ", "tsa"),
    ("つぃ", "tsi"),
    ("つぇ", "tse"),
    ("つぉ", "tso"),
];

/// A kana string in standard Hepburn, as a learner reading a textbook would type
/// it, or `None` when it holds something Hepburn has no keystrokes for (ー).
///
/// Composed from the readings table and the yōon list, never from the input
/// engine: ん is `n` (and `n'` before a vowel or `y`), a sokuon doubles the next
/// consonant (`tch` before `ch`), and は, を and へ are spelled `ha`, `o` and `he` —
/// the word's own reading, since the composer cannot tell a particle.
fn hepburn(kana: &str) -> Option<String> {
    let kana = nihongo_core::normalise_to_hiragana(kana);
    let digraphs: Vec<(String, String)> = yoon(Script::Hiragana)
        .into_iter()
        .map(|y| (y.display, y.hepburn))
        .chain(EXTENDED.iter().map(|(k, r)| (k.to_string(), r.to_string())))
        .collect();
    let mut pieces: Vec<String> = Vec::new();
    let mut rest = kana.as_str();
    while let Some(first) = rest.chars().next() {
        if let Some((k, r)) = digraphs.iter().find(|(k, _)| rest.starts_with(k.as_str())) {
            pieces.push(r.clone());
            rest = &rest[k.len()..];
            continue;
        }
        let spelled = match first {
            'っ' => "っ".to_string(),
            'ん' => "ん".to_string(),
            _ => reading(first)?.hepburn.first()?.to_string(),
        };
        pieces.push(spelled);
        rest = &rest[first.len_utf8()..];
    }
    let mut out = String::new();
    for (i, piece) in pieces.iter().enumerate() {
        let next = pieces.get(i + 1).map(String::as_str).unwrap_or("");
        match piece.as_str() {
            "っ" if next.starts_with("ch") => out.push('t'),
            "っ" => out.push(next.chars().next()?),
            "ん" => {
                out.push('n');
                if next.starts_with(|c: char| "aiueoy".contains(c)) {
                    out.push('\'');
                }
            }
            other => out.push_str(other),
        }
    }
    Some(out)
}

#[test]
fn the_composer_spells_what_a_textbook_spells() {
    for (kana, romaji) in [
        ("おんな", "onna"),
        ("まっちゃ", "matcha"),
        ("がっこう", "gakkou"),
        ("きんようび", "kin'youbi"),
        ("ヴァイオリン", "vaiorin"),
        ("こんにちは", "konnichiha"),
        ("パーティ", "pa-ti"),
    ] {
        let expected = (!romaji.contains('-')).then(|| romaji.to_string());
        assert_eq!(hepburn(kana), expected, "{kana}");
    }
}

/// The grammar point's own romaji, for a blank that is the whole pattern.
fn corpus_romaji_answers() -> Vec<(String, String, String)> {
    let dataset = grammar();
    nihongo_core::askable(&dataset)
        .into_iter()
        .filter_map(|answer| {
            let kana = answer.blank_kana();
            let pattern: String = answer
                .point
                .reading
                .as_deref()
                .unwrap_or(&answer.point.pattern)
                .chars()
                .filter(|ch| !matches!(ch, '〜' | '～' | ' ' | '　'))
                .collect();
            (pattern == kana).then(|| (answer.point.id.clone(), kana, answer.point.romaji.clone()))
        })
        .collect()
}

#[test]
fn every_whole_pattern_blank_is_answered_by_the_corpus_romaji_with_or_without_spaces() {
    let answers = corpus_romaji_answers();
    assert_eq!(answers.len(), 650, "blanks that are their whole pattern");
    // Two points annotate their romaji rather than spell it — `shika ... nai` and
    // `shika nai (no choice)` — and nobody types the annotation. Counted, not
    // silently dropped, so a third is noticed.
    let (typeable, annotated): (Vec<_>, Vec<_>) = answers.iter().partition(|(_, _, romaji)| {
        romaji
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || matches!(ch, ' ' | '-' | '\''))
    });
    assert_eq!(annotated.len(), 4, "{annotated:?}");
    assert_eq!(typeable.len(), 646);
    let mut failures = Vec::new();
    for (id, kana, romaji) in &typeable {
        let joined: String = romaji.split_whitespace().collect();
        for typed in [romaji.as_str(), joined.as_str()] {
            if !nihongo_core::matches_kana_text(kana, typed) {
                failures.push(format!("{id}: {typed:?} for {kana}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} typings rejected, e.g. {:?}",
        failures.len(),
        typeable.len() * 2,
        &failures[..failures.len().min(12)]
    );
}

#[test]
fn every_production_answer_is_answered_by_its_hepburn() {
    let dataset = grammar();
    let mut failures = Vec::new();
    let mut asked = 0usize;
    for answer in nihongo_core::askable(&dataset) {
        asked += 1;
        let kana = answer.blank_kana();
        let typed = hepburn(&kana).unwrap_or_else(|| panic!("{kana} has a Hepburn spelling"));
        if !nihongo_core::matches_kana_text(&kana, &typed) {
            failures.push(format!("{}: {typed:?} for {kana}", answer.point.id));
        }
        // And whatever spelling the gate hands out is one the check accepts.
        let gate = nihongo_core::romaji_for(&kana).expect("every askable blank is typeable");
        assert!(
            nihongo_core::matches_kana_text(&kana, &gate),
            "{kana} typed as {gate}"
        );
    }
    assert_eq!(asked, 903);
    assert!(
        failures.is_empty(),
        "{} of {asked} rejected, e.g. {:?}",
        failures.len(),
        &failures[..failures.len().min(12)]
    );
}

#[test]
fn every_word_reading_is_answered_by_its_hepburn() {
    let dataset = words();
    let (mut checked, mut long_vowel) = (0usize, 0usize);
    let mut failures = Vec::new();
    for word in dataset.words() {
        let Some(typed) = hepburn(&word.reading) else {
            assert!(
                word.reading.contains('ー'),
                "{} has no Hepburn spelling",
                word.reading
            );
            long_vowel += 1;
            continue;
        };
        checked += 1;
        if !nihongo_core::matches_kana_text(&word.reading, &typed) {
            failures.push(format!("{} {}: {typed:?}", word.text, word.reading));
        }
    }
    assert_eq!(checked + long_vowel, dataset.len());
    assert!(
        failures.is_empty(),
        "{} of {checked} word readings rejected ({long_vowel} with ー not typed), e.g. {:?}",
        failures.len(),
        &failures[..failures.len().min(12)]
    );
}
