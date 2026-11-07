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
//! * **Near misses, which must be refused.** Accepting more is only half of a
//!   check. Every は said `wa` and every へ said `e` where it is *not* a particle
//!   (花 as `wana`, はず as `wazu`), and every Hepburn spelling with its apostrophe
//!   dropped (禁煙 as `kinen`), is typed against every word and every production
//!   answer, and none may be accepted. The particle positions are the ones the
//!   word card and the drill grade with: a word's own final は (今日は), and the
//!   は the grammar point's romaji spells `wa`.

use nihongo_core::{reading, yoon, Askable, GrammarDataset, Script, Word, WordDataset};

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

/// A production answer graded the way the drill grades it: against the blank's
/// reading, with the particles the point's own romaji names.
fn blank_accepts(answer: &Askable<'_>, typed: &str) -> bool {
    answer.check(typed).correct
}

/// A word reading graded the way the word card grades it.
fn word_accepts(word: &Word, typed: &str) -> bool {
    nihongo_core::check_kana_text_with(&word.reading, typed, &word.particles()).correct
}

/// The grammar point's own romaji, for a blank that is the whole pattern.
fn whole_pattern_blanks(dataset: &GrammarDataset) -> Vec<Askable<'_>> {
    nihongo_core::askable(dataset)
        .into_iter()
        .filter(|answer| {
            let pattern: String = answer
                .point
                .reading
                .as_deref()
                .unwrap_or(&answer.point.pattern)
                .chars()
                .filter(|ch| !matches!(ch, '〜' | '～' | ' ' | '　'))
                .collect();
            pattern == answer.blank_kana()
        })
        .collect()
}

#[test]
fn every_whole_pattern_blank_is_answered_by_the_corpus_romaji_with_or_without_spaces() {
    let dataset = grammar();
    let answers = whole_pattern_blanks(&dataset);
    assert_eq!(answers.len(), 650, "blanks that are their whole pattern");
    // Two points annotate their romaji rather than spell it — `shika ... nai` and
    // `shika nai (no choice)` — and nobody types the annotation. Counted, not
    // silently dropped, so a third is noticed.
    let (typeable, annotated): (Vec<_>, Vec<_>) = answers.iter().partition(|answer| {
        answer
            .point
            .romaji
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || matches!(ch, ' ' | '-' | '\''))
    });
    assert_eq!(annotated.len(), 4);
    assert_eq!(typeable.len(), 646);
    let mut failures = Vec::new();
    for answer in &typeable {
        let romaji = answer.point.romaji.as_str();
        let joined: String = romaji.split_whitespace().collect();
        for typed in [romaji, joined.as_str()] {
            if !blank_accepts(answer, typed) {
                failures.push(format!(
                    "{}: {typed:?} for {}",
                    answer.point.id,
                    answer.blank_kana()
                ));
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
        if !blank_accepts(&answer, &typed) {
            failures.push(format!("{}: {typed:?} for {kana}", answer.point.id));
        }
        // And whatever spelling the gate hands out is one the check accepts.
        let gate = nihongo_core::romaji_for(&kana).expect("every askable blank is typeable");
        assert!(blank_accepts(&answer, &gate), "{kana} typed as {gate}");
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
        if !word_accepts(word, &typed) {
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

/// Every typing of `kana` that is a near miss rather than a spelling: each は said
/// `wa` and each へ said `e` at a position that is **not** a particle, one at a
/// time, and the Hepburn spelling with its apostrophes dropped (`kin'en` as
/// `kinen`). A near miss that [`nihongo_core::to_kana`] reads as the kana anyway is
/// not one, and is left out.
fn near_misses(kana: &str, particles: &[usize]) -> Vec<String> {
    let kana = nihongo_core::normalise_to_hiragana(kana);
    let chars: Vec<char> = kana.chars().collect();
    let mut misses = Vec::new();
    for (at, ch) in chars.iter().enumerate() {
        let said = match ch {
            'は' => 'わ',
            'へ' => 'え',
            _ => continue,
        };
        if particles.contains(&at) {
            continue;
        }
        let mut spoken = chars.clone();
        spoken[at] = said;
        misses.extend(hepburn(&spoken.iter().collect::<String>()));
    }
    if let Some(spelled) = hepburn(&kana).filter(|spelled| spelled.contains('\'')) {
        misses.push(spelled.replace('\'', ""));
    }
    misses.retain(|typed| {
        nihongo_core::to_kana(typed)
            .map(|read| nihongo_core::normalise_to_hiragana(&read) != kana)
            .unwrap_or(true)
    });
    misses
}

#[test]
fn no_word_reading_accepts_a_near_miss() {
    // Before the particle positions and the ん fix, all 1,014 were accepted: 花 as
    // `wana`, 母 as `wawa`, 下手 as `eta`, 禁煙 as `kinen`.
    let dataset = words();
    let (mut tried, mut accepted) = (0usize, Vec::new());
    for word in dataset.words() {
        for typed in near_misses(&word.reading, &word.particles()) {
            tried += 1;
            if word_accepts(word, &typed) {
                accepted.push(format!("{} {}: {typed:?}", word.text, word.reading));
            }
        }
    }
    assert_eq!(tried, 1_014, "near misses tried");
    assert!(
        accepted.is_empty(),
        "{} of {tried} near misses accepted, e.g. {:?}",
        accepted.len(),
        &accepted[..accepted.len().min(12)]
    );
    // And the particles a word does have are still said: 今日は is `konnichiwa`.
    let spoken: Vec<&Word> = dataset
        .words()
        .iter()
        .filter(|w| !w.particles().is_empty())
        .collect();
    assert_eq!(spoken.len(), 8, "words ending in a particle は or へ");
    let hello = dataset
        .find("今日は", "こんにちは")
        .expect("今日は is in the vocabulary");
    assert!(word_accepts(hello, "konnichiwa"));
    assert!(word_accepts(hello, "konnichiha"));
    for (text, reading, typed) in [
        ("花", "はな", "wana"),
        ("八", "はち", "wachi"),
        ("母", "はは", "wawa"),
        ("林", "はやし", "wayashi"),
        ("早い", "はやい", "wayai"),
        ("下手", "へた", "eta"),
        ("部屋", "へや", "eya"),
        ("禁煙", "きんえん", "kinen"),
    ] {
        let word = dataset
            .find(text, reading)
            .unwrap_or_else(|| panic!("{text} is a word"));
        assert!(!word_accepts(word, typed), "{text} as {typed}");
    }
}

#[test]
fn no_production_answer_accepts_a_near_miss() {
    // Before the particle positions, all 18 were accepted: はずだ as `wazuda`.
    let dataset = grammar();
    let (mut tried, mut accepted) = (0usize, Vec::new());
    for answer in nihongo_core::askable(&dataset) {
        for typed in near_misses(&answer.blank_kana(), &answer.particles()) {
            tried += 1;
            if blank_accepts(&answer, &typed) {
                accepted.push(format!(
                    "{}: {typed:?} for {}",
                    answer.point.id,
                    answer.blank_kana()
                ));
            }
        }
    }
    assert_eq!(tried, 18, "near misses tried");
    assert!(
        accepted.is_empty(),
        "{} of {tried} near misses accepted, e.g. {:?}",
        accepted.len(),
        &accepted[..accepted.len().min(12)]
    );
    // Which blanks have a particle は or へ, read off the corpus's romaji: 92 of the
    // 110 that hold either kana, and the other 18 are exactly the ones whose romaji
    // says `ha` or `he` — はず, はじめて, 反して, 早い, 下手, 憚らない.
    let (mut spoken, mut strict) = (0usize, std::collections::BTreeSet::new());
    for answer in nihongo_core::askable(&dataset) {
        if !answer.blank_kana().contains(['は', 'へ']) {
            continue;
        }
        if answer.particles().is_empty() {
            strict.insert(answer.point.id.as_str());
        } else {
            spoken += 1;
        }
    }
    assert_eq!(spoken, 92);
    assert_eq!(
        strict.into_iter().collect::<Vec<_>>(),
        [
            "ga-hayai-ka",
            "ga-jouzu-ga-heta",
            "hazu-da",
            "hazu-ga-nai",
            "ni-hanshite",
            "o-hajime",
            "te-habakaranai",
            "te-hajimete",
        ]
    );
    // A blank that is a lump of its pattern, or one variant of several, still has
    // its particle: ところへ out of 〜ところに / 〜ところへ / 〜ところを, はない out of
    // 〜ほど〜はない, ばあいは out of 〜場合（は）.
    for (id, typed) in [
        ("tokoro-ni-tokoro-e-tokoro-o", "tokoro e"),
        ("hodo-wa-nai", "wa nai"),
        ("baai-wa", "baai wa"),
        ("de-wa-arimasen-ja-nai", "de wa arimasen"),
    ] {
        let point = dataset
            .get(id)
            .unwrap_or_else(|| panic!("{id} is in the corpus"));
        let accepted = nihongo_core::askable_of(point)
            .iter()
            .filter(|answer| answer.blank_kana().contains(['は', 'へ']))
            .any(|answer| blank_accepts(answer, typed));
        assert!(accepted, "{id}: {typed}");
    }

    // The two the review named, end to end through the drill's own verdict.
    for (id, typed) in [("hazu-da", "wazuda"), ("te-hajimete", "tewajimete")] {
        let point = dataset
            .get(id)
            .unwrap_or_else(|| panic!("{id} is in the corpus"));
        let asked = nihongo_core::producible_of(point);
        let (example, _) = asked.first().expect("it can be asked");
        let verdict = nihongo_core::verdict_for(point, *example, typed).expect("a verdict");
        assert!(!verdict.correct, "{id}: {typed}");
    }
}
