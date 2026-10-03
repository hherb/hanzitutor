//! The shipped passages artifact, checked.
//!
//! These assertions run against the **committed** artifact and need neither the
//! analyser nor the 190 MB dictionary that produced it — which is the point of
//! segmenting at build time. What they hold is:
//!
//! * **every kanji-bearing token is a word the course teaches**, checked against
//!   the words artifact rather than against a list kept here, so a passage can
//!   never offer a character the learner has no card for;
//! * **every reading is the surface form's**, which is the mistake this pipeline
//!   makes if it reads the wrong UniDic field — `reading` is the *base* form's, so
//!   行き would come out いく and 食べ たべる;
//! * **the artifact still matches the text in `data/passages/`**, so editing a
//!   passage without regenerating fails rather than shipping the old text.

use nihongo_core::passages::parse_passage;
use nihongo_core::{Passage, PassageDataset, PassageToken, WordDataset};

fn dataset() -> PassageDataset {
    PassageDataset::from_gzip_bytes(include_bytes!("../data/passages.bin.gz"))
        .expect("the committed passages artifact decodes")
}

fn words() -> WordDataset {
    WordDataset::from_gzip_bytes(include_bytes!("../data/words.bin.gz"))
        .expect("the committed words artifact decodes")
}

/// Every passage source, by key, as it is committed.
const SOURCES: [(&str, &str); 3] = [
    ("asa", include_str!("../data/passages/asa.txt")),
    ("gakko", include_str!("../data/passages/gakko.txt")),
    ("tabemono", include_str!("../data/passages/tabemono.txt")),
];

fn passage(key: &str) -> Passage {
    dataset()
        .get(key)
        .cloned()
        .unwrap_or_else(|| panic!("{key} is in the artifact"))
}

fn tokens(key: &str) -> Vec<PassageToken> {
    passage(key).tokens().cloned().collect()
}

#[test]
fn the_artifact_holds_the_passages() {
    let dataset = dataset();
    assert_eq!(dataset.len(), 3);
    let keys: Vec<&str> = dataset.passages().iter().map(|p| p.key.as_str()).collect();
    assert_eq!(keys, vec!["asa", "gakko", "tabemono"]);

    let lines: usize = dataset.passages().iter().map(|p| p.lines.len()).sum();
    let tokens: usize = dataset.passages().iter().map(|p| p.tokens().count()).sum();
    let linked: usize = dataset
        .passages()
        .iter()
        .flat_map(Passage::tokens)
        .filter(|token| token.word.is_some())
        .count();
    assert_eq!((lines, tokens, linked), (7, 37, 14));
}

/// **The invariant that makes the passages safe to ship.** A kanji the vocabulary
/// does not hold is one a learner can neither read nor tap, so `prepare-passages`
/// refuses to write the artifact at all — and this is the check that would catch a
/// passage edited *after* it was generated.
#[test]
fn every_kanji_bearing_token_is_a_word_the_course_teaches() {
    let words = words();
    for p in dataset().passages() {
        assert!(
            p.kanji_without_a_word().is_empty(),
            "{} uses kanji the vocabulary does not teach: {:?}",
            p.key,
            p.kanji_without_a_word()
                .iter()
                .map(|t| t.surface.as_str())
                .collect::<Vec<_>>()
        );
        for token in p.tokens() {
            if let Some(word) = &token.word {
                assert!(
                    words.of_text(word).is_some(),
                    "{}'s token {:?} links to {word:?}, which is not in the vocabulary",
                    p.key,
                    token.surface
                );
            }
        }
    }
}

/// The reading to draw has to be the **surface** form's. UniDic's `reading` field
/// is the *base* form's for an inflected word, so reading it blindly renders
/// 行き(いく) and 食べ(たべる)ます — which is wrong, and was wrong here until the
/// pipeline compared the surface form with the base form.
#[test]
fn a_reading_is_the_surface_forms_and_not_the_base_forms() {
    let gakko = tokens("gakko");
    let iki = gakko
        .iter()
        .find(|t| t.surface == "行き")
        .expect("行きます is in the passage");
    assert_eq!(iki.rt.as_deref(), Some("いき"), "行き is read いき, not いく");
    assert_eq!(iki.word.as_deref(), Some("行く"));

    let tabemono = tokens("tabemono");
    let tabe = tabemono
        .iter()
        .find(|t| t.surface == "食べ")
        .expect("食べます is in the passage");
    assert_eq!(tabe.rt.as_deref(), Some("たべ"), "食べ is read たべ, not たべる");

    // An uninflected word takes the conventional reading rather than the
    // pronunciation: 今日 is きょう, where UniDic's pronunciation field says きょー;
    // and the particle は is read は although it is pronounced わ.
    let asa = tokens("asa");
    let kyou = asa.iter().find(|t| t.surface == "今日").expect("今日 is there");
    assert_eq!(kyou.rt.as_deref(), Some("きょう"));
    let wa = asa.iter().find(|t| t.surface == "は").expect("は is there");
    assert_eq!(wa.rt, None, "a kana token needs no ruby");
}

#[test]
fn every_kanji_bearing_token_carries_a_hiragana_reading() {
    for p in dataset().passages() {
        for token in p.tokens() {
            if token.has_kanji() {
                let rt = token
                    .rt
                    .as_deref()
                    .unwrap_or_else(|| panic!("{}: {:?} has no reading", p.key, token.surface));
                assert!(!rt.is_empty(), "{}: {:?} has an empty reading", p.key, token.surface);
                for ch in rt.chars() {
                    assert!(
                        ('\u{3041}'..='\u{309F}').contains(&ch) || ch == '\u{30FC}',
                        "{}: {:?} is read {rt:?}, which is not hiragana — the analyser reads in \
                         katakana and the pipeline folds it",
                        p.key,
                        token.surface
                    );
                }
            } else {
                assert_eq!(
                    token.rt, None,
                    "{}: kana needs no ruby, and {:?} is kana",
                    p.key, token.surface
                );
            }
        }
    }
}

/// Tapping a word has to open the entry the course teaches, which for a conjugated
/// word is its dictionary form — 行きます is 行く, 読みます is 読む.
#[test]
fn a_conjugated_word_links_to_its_dictionary_form() {
    let words = words();
    for &(key, surface, dictionary_form) in &[
        ("gakko", "行き", "行く"),
        ("gakko", "読み", "読む"),
        ("tabemono", "飲み", "飲む"),
        ("tabemono", "食べ", "食べる"),
    ] {
        let found = tokens(key)
            .into_iter()
            .find(|t| t.surface == surface)
            .unwrap_or_else(|| panic!("{surface} is in {key}"));
        assert_eq!(found.word.as_deref(), Some(dictionary_form));
        let word = words
            .of_text(dictionary_form)
            .unwrap_or_else(|| panic!("{dictionary_form} is in the vocabulary"));
        assert_eq!(word.text, dictionary_form);
    }
}

/// The artifact carries the text that is in `data/passages/`, line for line. The
/// point is not the string comparison: it is that a passage can be edited in a
/// plain text file and the artifact *cannot* silently keep the old segmentation,
/// because this fails and says which passage is stale.
#[test]
fn the_artifact_still_matches_the_passage_sources() {
    for (key, source) in SOURCES {
        let expected = parse_passage(key, source).expect("the source parses");
        let actual = passage(key);
        assert_eq!(
            actual.text(),
            expected.text(),
            "{key}: the artifact and data/passages/{key}.txt disagree — regenerate with \
             `pnpm run prepare-passages`"
        );
        assert_eq!(actual.title, expected.title, "{key}: the title moved");
        assert_eq!(actual.gloss, expected.gloss, "{key}: the gloss moved");
        assert_eq!(
            actual.lines.len(),
            expected.lines.len(),
            "{key}: the line count moved"
        );
    }
}

/// What the analyser read, in order, is what was written — the concatenation of
/// the tokens is the line, and there are no invented spaces.
#[test]
fn the_tokens_reassemble_into_the_line_they_came_from() {
    for p in dataset().passages() {
        for (number, line) in p.lines.iter().enumerate() {
            let joined: String = line.iter().map(|t| t.surface.as_str()).collect();
            assert!(!joined.is_empty(), "{}: line {number} is empty", p.key);
            assert!(
                !joined.contains(' '),
                "{}: line {number} has a space in it — the source should not need one",
                p.key
            );
            for token in line {
                assert!(!token.surface.is_empty(), "{}: an empty token", p.key);
            }
        }
    }
}

/// A segmentation is one dictionary's answer, not a fact about Japanese, so the
/// artifact says which analyser and which dictionary produced it. A screen showing
/// these passages is showing UniDic's reading of them.
#[test]
fn the_artifact_records_the_analyser_and_its_dictionary() {
    let source = dataset().source().clone();
    assert_eq!(source.analyser, "lindera 6.2");
    assert_eq!(source.dictionary, "unidic-mecab-2.1.2");
}

#[test]
fn every_passage_has_a_title_a_gloss_and_kanji_to_read() {
    for p in dataset().passages() {
        assert!(!p.title.is_empty(), "{} has no title", p.key);
        assert!(p.gloss.is_some(), "{} has no gloss", p.key);
        assert!(p.lines.len() >= 2, "{} is a single line", p.key);
        assert!(!p.text().is_empty(), "{} has no text", p.key);
        assert!(
            p.tokens().any(PassageToken::has_kanji),
            "{} has no kanji at all, so it teaches nothing",
            p.key
        );
    }
}
