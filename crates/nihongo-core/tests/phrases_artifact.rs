//! The shipped phrases artifact, checked.
//!
//! These assertions run against the **committed** artifact and need neither the
//! analyser nor the 190 MB dictionary that produced it — which is the point of
//! segmenting at build time. What they hold is:
//!
//! * **every kanji-bearing token is a word the course teaches**, checked against the
//!   words artifact rather than against a list kept here, so a phrase can never offer
//!   a character the learner has no card for. This is the invariant the phrases
//!   share with the passages (`HANDOVER_NIHONGO.md` invariant 22), and it matters
//!   more here: a passage that broke it could be edited, and an imported sentence
//!   cannot be — so for the phrases the check is the whole promise;
//! * **every stored band is the one the vocabulary derives**, recomputed rather than
//!   trusted, so the ladder a screen groups by cannot drift from the words it claims
//!   to be levelled against;
//! * **every shipped sentence names a contributor and carries a licence**, which is
//!   the attribution CC BY 2.0 FR requires and the one thing an import can get wrong
//!   without noticing — the export hands out the literal string `\N` for a sentence
//!   whose contributor it does not have, and 42.7% of its Japanese sentences are in
//!   that state;
//! * **the text reassembles from its tokens**, character for character, so the
//!   segmentation cannot have dropped or invented anything.
//!
//! The counts are pinned because they are what catch the corpus moving. A rebuild
//! that changes them is a deliberate commit, and the funnel `prepare-phrases` prints
//! is what says why they changed.

use nihongo_core::{Phrase, PhraseDataset, WordDataset};

fn dataset() -> PhraseDataset {
    PhraseDataset::from_gzip_bytes(include_bytes!("../data/phrases.bin.gz"))
        .expect("the committed phrases artifact decodes")
}

fn words() -> WordDataset {
    WordDataset::from_gzip_bytes(include_bytes!("../data/words.bin.gz"))
        .expect("the committed words artifact decodes")
}

/// The licences a Tatoeba sentence can be under, as the corpus's API enumerates
/// them. `PROBLEM` is Tatoeba's own marker for a sentence whose licence it cannot
/// state, and it is listed here so that shipping one would be a *failure* rather
/// than an unrecognised value.
const LICENCES: [&str; 3] = ["CC BY 2.0 FR", "CC0 1.0", "PROBLEM"];

#[test]
fn the_artifact_holds_the_phrases_the_ladder_is_full_at_every_band() {
    let dataset = dataset();
    assert_eq!(dataset.len(), 1400, "the corpus moved; see the funnel");
    assert_eq!(
        dataset.band_counts(),
        vec![
            (1, 200),
            (2, 200),
            (3, 200),
            (4, 200),
            (5, 200),
            (6, 200),
            (7, 200)
        ],
        "every band is capped at 200 by --per-band, so every band should be full"
    );

    let tokens: usize = dataset.phrases().iter().map(|p| p.tokens.len()).sum();
    let linked: usize = dataset
        .phrases()
        .iter()
        .flat_map(Phrase::tokens)
        .filter(|t| t.word.is_some())
        .count();
    assert_eq!((tokens, linked), (6025, 2168));
}

/// **The invariant that makes the phrases safe to ship.** Every kanji-bearing token
/// is a word the vocabulary holds, so a tap always has a card behind it.
#[test]
fn every_kanji_bearing_token_is_a_word_the_course_teaches() {
    let words = words();
    for phrase in dataset().phrases() {
        assert!(
            phrase.kanji_without_a_word().is_empty(),
            "#{} uses kanji the vocabulary does not teach: {:?}",
            phrase.id,
            phrase
                .kanji_without_a_word()
                .iter()
                .map(|t| t.surface.as_str())
                .collect::<Vec<_>>()
        );
        for token in phrase.tokens() {
            if let Some(word) = &token.word {
                assert!(
                    words.of_text(word).is_some(),
                    "#{}'s token {:?} links to {word:?}, which is not in the vocabulary",
                    phrase.id,
                    token.surface
                );
            }
        }
    }
}

/// The band a phrase is filed under is the band of its hardest word, recomputed from
/// the committed vocabulary.
///
/// The stored band is what a screen groups by, so this is the check that the ladder
/// shown to a learner is the ladder the words artifact actually implies — the same
/// arrangement `tests/words_artifact.rs` uses for a word's own band.
#[test]
fn a_phrases_band_is_its_hardest_words_band() {
    let words = words();
    for phrase in dataset().phrases() {
        let derived = phrase.band_from(&words);
        assert_eq!(
            phrase.band, derived,
            "#{} is filed in band {} but its words derive {derived}: regenerate with `pnpm run \
             prepare-phrases`",
            phrase.id, phrase.band
        );
        // And the band is at least every word's, which is what "hardest" means.
        for token in phrase.tokens() {
            if let Some(word) = token.word.as_deref().and_then(|t| words.of_text(t)) {
                assert!(
                    word.band <= phrase.band,
                    "#{} is band {} but holds {} from band {}",
                    phrase.id,
                    phrase.band,
                    word.text,
                    word.band
                );
            }
        }
    }
}

/// Every phrase carries what CC BY 2.0 FR requires: a named contributor, the
/// sentence's own id, and its licence. `\N` is the export's marker for a value it
/// does not have, and it is the failure this test exists for — it is a *non-empty*
/// string, so a pipeline that only skipped empty fields would ship it as a
/// contributor's name.
#[test]
fn every_phrase_is_attributed_and_licensed() {
    for phrase in dataset().phrases() {
        assert!(phrase.id > 0, "{:?} has no corpus id", phrase.text);
        assert!(
            !phrase.author.trim().is_empty() && phrase.author != "\\N",
            "#{} names its contributor {:?}, which is not a name",
            phrase.id,
            phrase.author
        );
        assert!(
            LICENCES.contains(&phrase.licence.as_str()),
            "#{} is under {:?}, which is not one of the corpus's licences",
            phrase.id,
            phrase.licence
        );
        assert!(
            !phrase.english.trim().is_empty(),
            "#{} has no English translation, and a phrase with none is not one a screen can offer",
            phrase.id
        );
    }
}

/// A reading is drawn over a kanji and over nothing else, and it is hiragana. The
/// analyser reads in katakana and the pipeline folds it; a screen that had to know
/// which of the two it was looking at would be a screen with a bug in it.
#[test]
fn every_kanji_bearing_token_carries_a_hiragana_reading() {
    for phrase in dataset().phrases() {
        for token in phrase.tokens() {
            if token.has_kanji() {
                let rt = token
                    .rt
                    .as_deref()
                    .unwrap_or_else(|| panic!("#{}: {:?} has no reading", phrase.id, token.surface));
                assert!(
                    !rt.is_empty(),
                    "#{}: {:?} has an empty reading",
                    phrase.id,
                    token.surface
                );
                assert!(
                    rt.chars()
                        .all(|ch| ('\u{3041}'..='\u{309F}').contains(&ch) || ch == '\u{30FC}'),
                    "#{}: {:?} is read {rt:?}, which is not hiragana",
                    phrase.id,
                    token.surface
                );
            } else {
                assert_eq!(
                    token.rt, None,
                    "#{}: {:?} is kana and needs no ruby",
                    phrase.id, token.surface
                );
            }
        }
    }
}

/// What the analyser read is what was written: the tokens reassemble into the
/// sentence, so the segmentation dropped nothing and invented no spacing.
#[test]
fn the_tokens_reassemble_into_the_sentence_they_came_from() {
    for phrase in dataset().phrases() {
        assert!(!phrase.tokens.is_empty(), "#{} has no tokens", phrase.id);
        assert_eq!(
            phrase.text_from_tokens(),
            phrase.text,
            "#{}'s tokens do not spell its text",
            phrase.id
        );
        for token in phrase.tokens() {
            assert!(
                !token.surface.is_empty(),
                "#{} has an empty token",
                phrase.id
            );
        }
    }
}

/// The cut, as properties of what shipped.
///
/// The pipeline's own rules are what produced these; this is an independent reading
/// of the same promise, so a rule loosened in the pipeline without the artifact
/// being rebuilt — or a rule that was never true — fails here.
#[test]
fn every_phrase_is_one_short_sentence_of_the_shape_promised() {
    const TERMINAL: [char; 5] = ['。', '！', '？', '!', '?'];
    const PUNCTUATION: [char; 36] = [
        '"', '\'', '`', '(', ')', '[', ']', '{', '}', '<', '>', ':', ';', '/', '\\', '|', '~', '@',
        '#', '$', '%', '^', '&', '*', '\u{300C}', '\u{300D}', '\u{300E}', '\u{300F}', '\u{FF08}',
        '\u{FF09}', '\u{3008}', '\u{3009}', '\u{300A}', '\u{300B}', '\u{3010}', '\u{3011}',
    ];

    for phrase in dataset().phrases() {
        let text = &phrase.text;
        let length = text.chars().count();
        assert!(
            (6..=32).contains(&length),
            "#{} is {length} characters, outside the 6–32 the selection names: {text:?}",
            phrase.id
        );
        assert!(
            !text.chars().any(char::is_whitespace),
            "#{} contains whitespace: {text:?}",
            phrase.id
        );
        assert!(
            !text.chars().any(|ch| ch.is_ascii_alphabetic()),
            "#{} contains Latin letters: {text:?}",
            phrase.id
        );
        assert!(
            !text.chars().any(|ch| PUNCTUATION.contains(&ch) || ch == '・'),
            "#{} contains brackets, quotes or a middle dot: {text:?}",
            phrase.id
        );
        let last = text.chars().next_back().expect("a phrase is not empty");
        assert!(
            TERMINAL.contains(&last),
            "#{} does not end with a full stop, question or exclamation mark: {text:?}",
            phrase.id
        );
        assert!(
            !text[..text.len() - last.len_utf8()]
                .chars()
                .any(|ch| TERMINAL.contains(&ch)),
            "#{} holds more than one sentence: {text:?}",
            phrase.id
        );
    }
}

/// The do-not-ship list, as an assertion. It is short and it is a judgement, and
/// this is what stops a rebuild from quietly putting any of it back.
#[test]
fn no_phrase_holds_a_sentence_the_pipeline_refuses_to_ship() {
    const BANNED: [&str; 9] = [
        "馬鹿", "バカ", "ばか", "死ね", "殺す", "殺し", "セックス", "おっぱい", "ちんこ",
    ];
    for phrase in dataset().phrases() {
        for word in BANNED {
            assert!(
                !phrase.text.contains(word),
                "#{} holds {word:?}: {:?}",
                phrase.id,
                phrase.text
            );
        }
    }
}

/// One sentence, one row: ids are the corpus's and are what the attribution file is
/// keyed by, so a duplicate would attribute one sentence twice and could hide a
/// second sentence behind the first.
#[test]
fn the_ids_are_unique_and_the_order_is_the_ladder() {
    let dataset = dataset();
    let mut ids: Vec<u32> = dataset.phrases().iter().map(|p| p.id).collect();
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count, "two phrases share a corpus id");

    // Band, then shortest first: the order the screen draws, and the order the cap
    // selected in, so the list a learner reads is the list the funnel describes.
    let mut previous: Option<&Phrase> = None;
    for phrase in dataset.phrases() {
        if let Some(previous) = previous {
            assert!(
                (previous.band, previous.text.chars().count()) <= (phrase.band, phrase.text.chars().count()),
                "#{} (band {}, {} chars) comes before #{} (band {}, {} chars)",
                previous.id,
                previous.band,
                previous.text.chars().count(),
                phrase.id,
                phrase.band,
                phrase.text.chars().count()
            );
        }
        previous = Some(phrase);
    }
}

/// An imported corpus is not a fact about Japanese: it is one corpus, filtered by one
/// rule, on one day. The artifact says which, so a later rebuild can say what moved.
#[test]
fn the_artifact_records_where_the_corpus_came_from() {
    let source = dataset().source().clone();
    assert_eq!(source.analyser, "lindera 6.2");
    assert_eq!(source.dictionary, "unidic-mecab-2.1.2");
    assert_eq!(source.corpus, "Tatoeba");
    assert_eq!(source.url, "https://tatoeba.org");
    assert!(
        source.export.contains("jpn_sentences_detailed.tsv"),
        "the export must name what was read: {}",
        source.export
    );
    assert!(
        source.export.contains("last modified"),
        "the export date is what tells a later rebuild which snapshot this is: {}",
        source.export
    );
    assert_eq!(source.licences, "CC BY 2.0 FR, CC0 1.0");
    for needle in [
        "named contributor",
        "not tagged",
        "the course teaches",
        "shortest of each band",
    ] {
        assert!(
            source.selection.contains(needle),
            "the selection rule must state what it filtered on (missing {needle:?}): {}",
            source.selection
        );
    }
}

/// The licences actually used, which is what the notice has to cover.
#[test]
fn the_licences_in_use_are_the_corpus_licences() {
    let dataset = dataset();
    let mut licences = dataset.licences();
    licences.sort_unstable();
    assert_eq!(licences, vec!["CC BY 2.0 FR"]);
}
