//! The shipped grammar artifact, checked.
//!
//! These assertions run against the **committed** artifact and need neither the
//! corpus nor the network that produced it. What they hold:
//!
//! * **every kanji in an example carries a reading over it**, which is the
//!   invariant that replaces the vocabulary rule here: a grammar example is the
//!   corpus's own teaching sentence rather than a word the course teaches, so what
//!   the learner is promised instead is that anything they cannot read has furigana
//!   above it. This is the check `prepare-grammar` refuses to write without;
//! * **no shipped text names a JLPT level**, which is `HANDOVER_NIHONGO.md`
//!   invariant 20's obligation applied to the third imported dataset. The corpus
//!   labels every point `N5`–`N1` and cross-references its notes by the same
//!   labels, and none of that is in here — not as a field, not as a tag, not inside
//!   a note;
//! * **the examples reassemble from their own ruby**, character for character, so
//!   the furigana markup cannot have dropped or invented anything;
//! * **the order is the corpus's teaching order**, not a sort: the five level files
//!   read as one course, which is what the screen offers instead of a band the
//!   measurement rejected (344 of 526 points would land in band 7 — see
//!   `grammar.rs`).
//!
//! The counts are pinned because they are what catch the corpus moving. A rebuild
//! that changes them is a deliberate commit, and the numbers `prepare-grammar`
//! prints are what say why they changed.

use nihongo_core::{is_kanji, GrammarDataset, GrammarPoint};

fn dataset() -> GrammarDataset {
    GrammarDataset::from_gzip_bytes(include_bytes!("../data/grammar.bin.gz"))
        .expect("the committed grammar artifact decodes")
}

/// Every run of text a screen can draw, so a test can hold all of it at once.
fn shipped_text(point: &GrammarPoint) -> Vec<(&'static str, &str)> {
    let mut fields = vec![
        ("id", point.id.as_str()),
        ("pattern", point.pattern.as_str()),
        ("romaji", point.romaji.as_str()),
        ("meaning", point.meaning.as_str()),
        ("formation", point.formation.as_str()),
    ];
    if let Some(reading) = point.reading.as_deref() {
        fields.push(("reading", reading));
    }
    if let Some(notes) = point.notes.as_deref() {
        fields.push(("notes", notes));
    }
    for tag in &point.tags {
        fields.push(("tag", tag.as_str()));
    }
    for example in &point.examples {
        fields.push(("example", example.ja.as_str()));
        fields.push(("translation", example.en.as_str()));
    }
    fields
}

#[test]
fn the_artifact_holds_the_points_the_corpus_publishes() {
    let dataset = dataset();
    assert_eq!(dataset.len(), 526, "the corpus moved; see prepare-grammar's funnel");
    assert_eq!(
        dataset.example_count(),
        1526,
        "474 points carry three examples and 52 carry two"
    );
    assert_eq!(
        dataset
            .points()
            .iter()
            .filter(|point| point.reading.is_some())
            .count(),
        107,
        "the patterns written with a kanji, and only those, carry a reading"
    );
    assert_eq!(dataset.tags().len(), 70, "the corpus's controlled tag list");
    assert_eq!(
        dataset
            .points()
            .iter()
            .filter(|point| point.notes.is_none())
            .count(),
        2,
        "524 of the 526 points carry a note"
    );
}

#[test]
fn every_kanji_in_an_example_carries_a_reading_over_it() {
    let dataset = dataset();
    let mut checked = 0usize;
    let mut uncovered: Vec<String> = Vec::new();
    for point in dataset.points() {
        for example in point.examples() {
            checked += 1;
            let missing = example.kanji_without_ruby();
            if !missing.is_empty() {
                uncovered.push(format!(
                    "{}: {:?} has no reading over {}",
                    point.id,
                    example.ja,
                    missing.iter().collect::<String>()
                ));
            }
            if example.has_kanji() {
                assert!(
                    !example.ruby.is_empty(),
                    "{}: {:?} contains a kanji and no furigana at all",
                    point.id,
                    example.ja
                );
            }
        }
    }
    assert_eq!(checked, 1526);
    assert!(uncovered.is_empty(), "{uncovered:#?}");
}

#[test]
fn the_examples_reassemble_from_their_ruby() {
    let dataset = dataset();
    let mut kana_only = 0usize;
    for point in dataset.points() {
        for example in point.examples() {
            assert_eq!(
                example.text_from_ruby(),
                example.ja,
                "{}: the ruby does not spell the sentence",
                point.id
            );
            if !example.has_kanji() {
                kana_only += 1;
                assert!(
                    example.ruby.len() == 1 && example.ruby[0].rt.is_none(),
                    "{}: {:?} has no kanji, so it carries one plain segment and no reading",
                    point.id,
                    example.ja
                );
            }
            assert!(
                !example.kana().chars().any(is_kanji),
                "{}: {:?} reads back with a kanji still in it",
                point.id,
                example.ja
            );
        }
    }
    assert_eq!(
        kana_only, 16,
        "the examples with no kanji to annotate, as the pipeline prints"
    );
}

/// Invariant 20's obligation, on the third imported dataset: the interface may not
/// imply that a level here is the JLPT's, and the surest way to keep that promise
/// is to ship no level at all — not the corpus's field, not a tag, and not the
/// cross-references inside its notes.
#[test]
fn no_shipped_text_names_a_jlpt_level() {
    let dataset = dataset();
    for point in dataset.points() {
        for (field, text) in shipped_text(point) {
            for label in ["N1", "N2", "N3", "N4", "N5", "JLPT"] {
                assert!(
                    !text.contains(label),
                    "{}: {field} names a level: {text:?}",
                    point.id
                );
            }
        }
        assert!(
            !point.tags.iter().any(|tag| tag.starts_with('N')),
            "{}: a level arrived as a tag: {:?}",
            point.id,
            point.tags
        );
    }
}

#[test]
fn the_order_is_the_corpus_teaching_order_and_not_a_sort() {
    let dataset = dataset();
    let ids: Vec<&str> = dataset.points().iter().map(|p| p.id.as_str()).collect();
    assert_eq!(&ids[..3], ["wa", "ga-subject", "o"], "the course opens where a course does");
    assert_eq!(
        ids.last().copied(),
        Some("nai-to-mo-kagiranai"),
        "and closes where the corpus's last level file does"
    );
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_ne!(ids, sorted, "the order is the corpus's, not an alphabetical sort");

    // The five level files are one course in the order n5 → n1, and the boundary
    // between two of them is where a rebuild that reordered the files would show.
    let first = dataset.points().first().expect("a first point");
    let last = dataset.points().last().expect("a last point");
    assert_eq!(first.pattern, "〜は");
    assert_eq!(last.pattern, "〜ないとも限らない");
}

#[test]
fn every_point_carries_what_a_card_draws() {
    let dataset = dataset();
    let mut ids: Vec<&str> = Vec::new();
    for point in dataset.points() {
        assert!(!point.id.trim().is_empty());
        assert!(
            !ids.contains(&point.id.as_str()),
            "{} appears twice; the id is the artifact's key",
            point.id
        );
        ids.push(&point.id);
        assert!(!point.pattern.trim().is_empty(), "{}: no pattern", point.id);
        assert!(!point.romaji.trim().is_empty(), "{}: no romaji", point.id);
        assert!(!point.meaning.trim().is_empty(), "{}: no meaning", point.id);
        assert!(!point.formation.trim().is_empty(), "{}: no formation", point.id);
        if point.pattern_has_kanji() {
            assert!(
                point.reading.is_some(),
                "{}: {} is written with a kanji and carries no reading",
                point.id,
                point.pattern
            );
        }
        assert!(
            (1..=3).contains(&point.tags.len()),
            "{}: {} tags, and the corpus's schema allows one to three",
            point.id,
            point.tags.len()
        );
        for tag in &point.tags {
            assert!(
                !tag.is_empty()
                    && tag
                        .chars()
                        .all(|ch| ch.is_ascii_lowercase() || ch == '-'),
                "{}: the tag {tag:?} is not a tag from the corpus's controlled list",
                point.id
            );
        }
        assert!(
            (2..=3).contains(&point.examples.len()),
            "{}: {} examples, and the corpus writes two or three",
            point.id,
            point.examples.len()
        );
        for example in point.examples() {
            assert!(!example.ja.trim().is_empty(), "{}: an empty sentence", point.id);
            assert!(!example.en.trim().is_empty(), "{}: a sentence with no translation", point.id);
        }
    }
    assert_eq!(ids.len(), 526);
}

#[test]
fn a_pattern_is_found_by_what_a_learner_would_type() {
    let dataset = dataset();
    assert_eq!(
        dataset.search("ni taishite", 10).first().map(|p| p.id.as_str()),
        Some("ni-taishite"),
        "the romaji the card shows is a way in"
    );
    assert_eq!(
        dataset.search("ni-taishite", 10).first().map(|p| p.id.as_str()),
        Some("ni-taishite"),
        "and so is the id the artifact is keyed by"
    );
    let tagged = dataset
        .points()
        .iter()
        .filter(|point| point.tags.iter().any(|tag| tag == "particle"))
        .count();
    assert_eq!(tagged, 24, "24 points are tagged `particle`");
    assert!(
        dataset.search_total("particle") >= tagged,
        "every one of them is found by its tag"
    );
    assert_eq!(
        dataset.search("", 10).len(),
        0,
        "an empty query asks for nothing rather than everything"
    );
    assert_eq!(dataset.search_total("no such pattern"), 0);
}

#[test]
fn the_artifact_records_where_the_corpus_came_from_and_what_was_changed() {
    let dataset = dataset();
    let source = dataset.source();
    assert_eq!(source.corpus, "OpenJLPT");
    assert_eq!(source.url, "https://github.com/evanclan/OpenJLPT");
    assert_eq!(source.version, "0.3.0", "the dataset's own meta.json version");
    assert_eq!(
        source.licence, "CC BY-SA 4.0",
        "the share-alike licence the artifact is redistributed under"
    );
    assert!(!source.retrieved.trim().is_empty());
    assert!(
        source.selection.contains("526"),
        "the artifact describes its own cut: {:?}",
        source.selection
    );
    assert!(
        source.modifications.contains("level references"),
        "CC BY-SA 4.0 §3(a)(1)(B) asks for the modification to be stated: {:?}",
        source.modifications
    );
    assert!(
        source.modifications.contains("furigana"),
        "parsing the corpus's markup into ruby segments is a change too: {:?}",
        source.modifications
    );
}
