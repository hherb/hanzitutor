//! The shipped vocabulary artifact, checked.
//!
//! These assertions are the contract the vocabulary course relies on, and they run
//! against the **committed** artifact, so they hold in CI without any upstream
//! download — the dictionary inputs are 118 MB of JSON, 63 MB of XML and 33 MB of
//! furigana, and nobody should need any of it to check what shipped.
//!
//! Three things are frozen, and the first two are checked against the *kanji*
//! artifact rather than against a remembered rule:
//!
//! * **which words are in** — EDRDG's ranked vocabulary, narrowed to words whose
//!   every kanji the committed kanji artifact can draw and teach;
//! * **this project's ladder** — the band is the highest kyōiku grade among the
//!   word's kanji, recomputed here from `kanji.bin.gz` so the two artifacts cannot
//!   drift apart;
//! * **the furigana** — that the segments cover the word and spell its reading,
//!   and that no reading was invented to fill a gap.

use nihongo_core::words::Ruby;
use nihongo_core::{band_for, band_name, is_kanji, BANDS, KanjiDataset, WordDataset};

fn dataset() -> WordDataset {
    WordDataset::from_gzip_bytes(include_bytes!("../data/words.bin.gz"))
        .expect("the committed words artifact decodes")
}

fn kanji() -> KanjiDataset {
    KanjiDataset::from_gzip_bytes(include_bytes!("../data/kanji.bin.gz"))
        .expect("the committed kanji artifact decodes")
}

fn word(text: &str, reading: &str) -> nihongo_core::Word {
    dataset()
        .find(text, reading)
        .cloned()
        .unwrap_or_else(|| panic!("{text} ({reading}) is in the vocabulary"))
}

#[test]
fn the_artifact_holds_the_ranked_vocabulary() {
    let dataset = dataset();
    assert_eq!(dataset.len(), 21_902);
    assert_eq!(
        dataset.band_counts(),
        vec![(1, 655), (2, 2_284), (3, 3_284), (4, 3_072), (5, 3_408), (6, 2_705), (7, 6_494)],
        "the ladder's shape: a word is readable when its kanji are known, so the bands \
         are not the same size and are not meant to be"
    );
}

#[test]
fn every_word_is_one_a_course_can_teach() {
    for w in dataset().words() {
        assert!(!w.text.is_empty(), "a word with no text");
        assert!(!w.reading.is_empty(), "{} has no reading", w.text);
        assert!(!w.meaning.is_empty(), "{} has no gloss", w.text);
        assert!(
            BANDS.contains(&w.band),
            "{} is in band {}, which the ladder does not have",
            w.text,
            w.band
        );
        assert!(
            (1..=48).contains(&w.nf),
            "{} has nf {}, and nf is a block of 500 between 1 and 48",
            w.text,
            w.nf
        );
        assert!(
            !w.kanji().is_empty(),
            "{} contains no kanji, so it teaches no character and is not this course's",
            w.text
        );
    }
}

/// The membership rule, checked the only way that cannot drift: against the
/// artifact the board draws from. A word whose kanji is not there could be offered
/// and then not be drawable.
#[test]
fn every_kanji_in_every_word_is_one_the_kanji_artifact_holds() {
    let kanji = kanji();
    for w in dataset().words() {
        for ch in w.kanji() {
            assert!(
                kanji.get(ch).is_some(),
                "{} contains {ch}, which the committed kanji artifact does not hold",
                w.text
            );
        }
    }
}

/// And the ladder itself is recomputed rather than trusted: the band stored in the
/// words artifact is the one the kanji artifact's grades derive. If KANJIDIC2 ever
/// regrades a character, this fails rather than leaving the course quietly wrong.
#[test]
fn every_words_band_is_what_its_kanji_grades_derive() {
    let kanji = kanji();
    for w in dataset().words() {
        let grades: Vec<u8> = w
            .kanji()
            .into_iter()
            .filter_map(|ch| kanji.get(ch).map(|k| k.grade))
            .collect();
        assert_eq!(
            w.band,
            band_for(grades),
            "{} is stored in band {} but its kanji put it in {}",
            w.text,
            w.band,
            band_for(
                w.kanji()
                    .into_iter()
                    .filter_map(|ch| kanji.get(ch).map(|k| k.grade))
            )
        );
    }
}

/// The property that makes furigana worth carrying: the pieces cover the word, and
/// they spell its reading. JmdictFurigana satisfies it for every entry it aligns —
/// 0 exceptions over 712,069 segments — and this is what would notice an upstream
/// release that changed that.
#[test]
fn furigana_covers_the_word_and_spells_its_reading() {
    let dataset = dataset();
    let mut with_furigana = 0usize;
    let mut without = Vec::new();
    let mut kana_type_only = Vec::new();

    for w in dataset.words() {
        if !w.has_furigana() {
            without.push(w.text.clone());
            continue;
        }
        with_furigana += 1;
        assert_eq!(
            w.text_from_furigana(),
            w.text,
            "{}'s furigana covers different characters",
            w.text
        );
        if w.reading_from_furigana() != w.reading {
            kana_type_only.push(w.text.clone());
        }
        assert!(
            w.furigana_spells_reading(),
            "{}'s furigana spells {:?} but its reading is {:?}",
            w.text,
            w.reading_from_furigana(),
            w.reading
        );
    }

    assert_eq!(
        with_furigana, 21_836,
        "the alignment JmdictFurigana solves; the rest keep their reading and no ruby"
    );
    assert_eq!(
        without.len(),
        66,
        "the words carried without furigana: the upstream release does not align them — \
         gathered, never invented"
    );
    // Two words spell their reading in the other kana type, because the written form
    // decided the type and the dictionary decided the reading. Both are right; the
    // exception is named so that a third one is noticed rather than tolerated.
    kana_type_only.sort();
    assert_eq!(
        kana_type_only,
        vec!["タンパク質".to_string(), "生ゴミ".to_string()],
        "the words whose ruby is the same reading in the other kana type"
    );
}

/// The direction that matters for a screen: a segment with a kanji in it must carry
/// a reading, or the learner sees a kanji with no way to read it.
///
/// The converse is deliberately **not** asserted. 6,372 segments have a reading
/// over plain kana, and they are correct: こんにちは is written こんにち**は** and
/// read こんにち**わ**, which is exactly the case a "kana needs no reading" rule
/// would get wrong.
#[test]
fn a_furigana_segment_over_kanji_always_carries_its_reading() {
    for w in dataset().words() {
        for part in &w.furigana {
            if part.ruby.chars().any(is_kanji) {
                assert!(
                    part.rt.is_some(),
                    "{}: the segment {:?} contains a kanji and has no reading",
                    w.text,
                    part.ruby
                );
            }
            assert!(!part.ruby.is_empty(), "{} has an empty furigana segment", w.text);
        }
    }
}

/// **The acceptance criterion about readings, as a test.**
///
/// A word's reading comes from the dictionary and is never assembled from its
/// characters. Every one of these is a word where assembling it would be wrong, and
/// they are the reason the field exists: 大人 is おとな and not だいじん, 今日 is
/// きょう and not いまひ, 一人 is ひとり and not いちにん, 明日 is あした and not
/// みょうにち.
#[test]
fn a_words_reading_is_its_own_and_never_composed_from_its_characters() {
    for &(text, reading, wrong) in &[
        ("大人", "おとな", "だいじん"),
        ("今日", "きょう", "いまひ"),
        ("一人", "ひとり", "いちにん"),
        ("明日", "あした", "みょうにち"),
    ] {
        let w = word(text, reading);
        assert_eq!(w.reading, reading);
        assert_ne!(w.reading, wrong, "{text} must not be read as if composed");
    }
}

/// And the furigana puts each reading over the right characters rather than over the
/// word as a whole — which is the difference between a reading aid and a caption.
#[test]
fn furigana_sits_over_the_right_characters() {
    let taberu = word("食べる", "たべる");
    assert_eq!(
        taberu.furigana,
        vec![
            Ruby::new("食", Some("た".to_string())),
            Ruby::new("べる", None)
        ],
        "the reading of 食 goes over 食, and the okurigana べる stands on its own"
    );

    // The other shape, and the reason a segment can cover two characters: おとな
    // cannot be cut between 大 and 人, so the whole compound takes one reading.
    // (JmdictFurigana's own worked example, 大人買い, is not in this vocabulary —
    // EDRDG gives it no nf rank — so the example here is one that is.)
    let otona = word("大人", "おとな");
    assert_eq!(
        otona.furigana,
        vec![Ruby::new("大人", Some("おとな".to_string()))],
        "おとな covers 大人 together"
    );
}

#[test]
fn words_are_ordered_by_band_then_frequency_without_duplicates() {
    let dataset = dataset();
    let mut pairs: Vec<(u8, u8)> = Vec::new();
    for w in dataset.words() {
        pairs.push((w.band, w.nf));
    }
    let mut sorted = pairs.clone();
    sorted.sort_unstable();
    assert_eq!(pairs, sorted, "the course order is the stored order");

    let unique: std::collections::BTreeSet<(String, String)> = dataset
        .words()
        .iter()
        .map(|w| (w.text.clone(), w.reading.clone()))
        .collect();
    assert_eq!(
        unique.len(),
        dataset.len(),
        "a word is written and read one way, once"
    );
}

#[test]
fn a_word_is_found_by_its_text_by_its_reading_and_by_its_kanji() {
    let dataset = dataset();
    assert_eq!(dataset.find("食べる", "たべる").map(|w| w.nf), Some(25));
    assert!(dataset.find("食べる", "くう").is_none(), "that is a different word");

    let eater = dataset.of_text("食べる").expect("食べる is present");
    assert_eq!(eater.meaning, "to eat");

    let with_food: Vec<&str> = dataset.of_kanji('食').map(|w| w.text.as_str()).collect();
    assert!(
        with_food.contains(&"食べる") && with_food.contains(&"食事"),
        "the words a character appears in is what a character screen lists: {with_food:?}"
    );
    assert!(!with_food.is_empty());
}

/// The bands are this project's, and the names a screen uses have to say so. There
/// has been no official JLPT kanji or vocabulary list since 2010, so claiming one
/// would be a lie about the data rather than a simplification.
#[test]
fn the_ladder_is_ours_and_its_names_say_what_it_is() {
    assert_eq!(band_name(1), "kyōiku 1");
    assert_eq!(band_name(6), "kyōiku 6");
    assert_eq!(band_name(7), "jōyō beyond the school grades");
    for band in BANDS {
        let name = band_name(band);
        assert!(
            !name.to_ascii_uppercase().contains("JLPT"),
            "the ladder is derived, not the JLPT's, and {band} says {name:?}"
        );
    }
}

/// The artifact records which snapshots it was built from, because the EDRDG
/// obligation to keep the data current is unenforceable against an artifact that
/// cannot say what it holds. Pinning them here makes a bump a deliberate commit —
/// `LICENSES.md` has the procedure.
#[test]
fn the_artifact_records_the_snapshots_it_was_built_from() {
    let source = dataset().source().clone();
    assert_eq!(source.jmdict_version, "3.6.2");
    assert_eq!(source.jmdict_date, "2026-09-28");
    assert_eq!(
        source.nf_source_date, "2026-10-02",
        "the nf ranking comes from EDRDG's own XML, whose file is rebuilt daily and \
         carries its creation date instead of a version"
    );
    assert_eq!(source.furigana_release, "2.3.1+2026-09-25");
}

/// Every band is populated, and the first is the small one: few words are written
/// entirely with grade-1 kanji, which is a fact about Japanese rather than a hole
/// in the data.
#[test]
fn every_band_has_words_and_the_first_is_the_smallest() {
    let dataset = dataset();
    for (band, count) in dataset.band_counts() {
        assert!(count > 0, "band {band} is empty");
    }
    let first = dataset.of_band(1).count();
    assert_eq!(first, 655);
    assert!(
        dataset.of_band(1).all(|w| w.band == 1),
        "a band holds only its own words"
    );
}
