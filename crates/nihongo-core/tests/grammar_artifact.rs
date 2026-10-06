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
//!   `grammar.rs`);
//! * **the vocabulary link is real**: every word a sentence is recorded as using is
//!   a word the committed words artifact teaches, each is named once, and the reach
//!   of the link (526 patterns, 1,523 sentences, 8,151 links) is pinned. This is the
//!   one cross-artifact assertion here: the two are committed separately and either
//!   can be rebuilt alone.
//! * **the course can be asked both ways**: the drill's own reach over the committed
//!   sentences (363 of the 526 patterns, over 903), and the *production* exercise's
//!   answer for every one of them — a kana reading the corpus wrote, which the input
//!   engine can type, which is what makes its pool the same pool asked differently.
//!
//! The counts are pinned because they are what catch the corpus moving. A rebuild
//! that changes them is a deliberate commit, and the numbers `prepare-grammar`
//! prints are what say why they changed.

use nihongo_core::{
    is_kanji, GrammarDataset, GrammarLog, GrammarPoint, KanjiDataset, WordDataset, MIN_COMPOSITION,
};

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

/// The other half of the level stripping: a label taken out of the middle of a
/// sentence must not leave the sentence with a hole in it. The corpus's own notes
/// contain no ` :`, ` ;` or ` ,` at all — measured, not assumed — so one in the
/// artifact is a label that was removed without the space it sat behind, which is
/// how `Formal N2: 〜がたい.` first shipped as `Formal : 〜がたい.`
#[test]
fn no_note_has_a_hole_where_a_level_label_was_taken_out() {
    let dataset = dataset();
    for point in dataset.points() {
        let Some(notes) = point.notes.as_deref() else {
            continue;
        };
        for mark in [" :", " ;", " ,"] {
            assert!(
                !notes.contains(mark),
                "{}: a space before {mark:?} is where a label used to be: {notes:?}",
                point.id
            );
        }
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
    assert!(
        source.modifications.contains("UniDic"),
        "the word link is not the corpus's, so the artifact has to say where it came \
         from: {:?}",
        source.modifications
    );
    assert!(
        source.analyser.starts_with("lindera"),
        "the analyser that read the examples' vocabulary is part of what this \
         artifact is: {:?}",
        source.analyser
    );
    assert!(
        source.dictionary.starts_with("unidic"),
        "and so is the dictionary it read them with: {:?}",
        source.dictionary
    );
}

/// The vocabulary link: every word a sentence is recorded as using is a word the
/// course teaches, and each is recorded once.
///
/// This is the assertion behind the screen's promise. `prepare-grammar` links a
/// sentence to the course's vocabulary through UniDic, and the app draws that link
/// as a list of words whose cards it can open — so a text here that is not in the
/// words artifact is a row the app would have to drop, and a text named twice is a
/// row drawn twice. Both are cross-artifact facts, which is why they are checked
/// here rather than in `prepare-grammar` alone: the two artifacts are committed
/// separately and either can be rebuilt on its own.
#[test]
fn every_linked_word_is_one_the_course_teaches_and_is_named_once() {
    let words = WordDataset::from_gzip_bytes(include_bytes!("../data/words.bin.gz"))
        .expect("the committed words artifact decodes");
    let dataset = dataset();
    let mut links = 0usize;
    for point in dataset.points() {
        for example in point.examples() {
            let mut seen: Vec<&str> = Vec::new();
            for word in &example.words {
                assert!(
                    words.of_text(word).is_some(),
                    "{}: {:?} is linked to {word:?}, which the vocabulary does not teach",
                    point.id,
                    example.ja
                );
                assert!(
                    !seen.contains(&word.as_str()),
                    "{}: {word:?} is named twice for {:?}",
                    point.id,
                    example.ja
                );
                seen.push(word);
                links += 1;
            }
        }
    }
    assert_eq!(links, 8151, "the links the shipped artifact holds");
}

/// How far the link reaches, pinned — because "the grammar course and the words
/// course are one course read two ways" is a claim about numbers.
///
/// **526 of the 526** patterns have at least one linked word, **1,523 of the 1,526**
/// sentences use one (three are made of words this course does not teach), the
/// per-sentence lists add up to **8,151** links, the union a pattern shows is
/// **7,345** words, and the most one pattern lists is **28**. A rebuild that moves
/// any of them has either changed the analyser, the vocabulary or the corpus, and
/// this is where that is a decision rather than a diff.
#[test]
fn the_vocabulary_link_reaches_the_whole_course() {
    let dataset = dataset();
    let with_words = dataset
        .points()
        .iter()
        .filter(|point| !point.words().is_empty())
        .count();
    assert_eq!(with_words, 526, "every pattern's sentences use a word of the course");

    let examples_with_words = dataset
        .points()
        .iter()
        .flat_map(|point| point.examples())
        .filter(|example| !example.words.is_empty())
        .count();
    assert_eq!(examples_with_words, 1523, "of the 1,526 sentences");

    let stored: usize = dataset
        .points()
        .iter()
        .flat_map(|point| point.examples())
        .map(|example| example.words.len())
        .sum();
    assert_eq!(stored, 8151);
    let union: usize = dataset.points().iter().map(|point| point.words().len()).sum();
    assert_eq!(union, 7345, "the same links, each word named once per pattern");
    assert_eq!(
        dataset.points().iter().map(|point| point.words().len()).max(),
        Some(28),
        "the longest list one pattern draws"
    );

    // The union is the sentences' lists, in the sentences' order, each name once —
    // the rule `GrammarPoint::words` states, re-derived here so a change to it is a
    // failing test rather than a quietly different screen.
    for point in dataset.points() {
        let mut expected: Vec<&str> = Vec::new();
        for example in point.examples() {
            for word in &example.words {
                if !expected.contains(&word.as_str()) {
                    expected.push(word);
                }
            }
        }
        assert_eq!(point.words(), expected, "{}", point.id);
    }

    // A beginner pattern, by name, so the shape of the link is legible in the test:
    // 〜は's three sentences are built from words the course teaches.
    let wa = dataset.get("wa").expect("〜は is in the corpus");
    assert_eq!(
        wa.examples[0].words,
        vec!["私".to_string(), "学生".to_string()],
        "私 and 学生, in the order the sentence says them"
    );
    assert_eq!(
        wa.words(),
        vec!["私", "学生", "今日", "天気", "良い", "本", "面白い"],
        "and the pattern's own list is those three sentences' words, each once"
    );
}

/// The other half of the link: the kanji a pattern is written with are characters
/// the app can actually open, which is a fact about the corpus rather than a hope —
/// all 143 of them are in the jōyō set.
#[test]
fn every_character_a_pattern_is_written_with_can_be_opened() {
    let kanji = KanjiDataset::from_gzip_bytes(include_bytes!("../data/kanji.bin.gz"))
        .expect("the committed kanji artifact decodes");
    let dataset = dataset();
    let mut with_characters = 0usize;
    let mut total = 0usize;
    let mut outside: Vec<String> = Vec::new();
    for point in dataset.points() {
        let expected: Vec<char> = {
            let mut seen: Vec<char> = Vec::new();
            for ch in point.pattern.chars().filter(|ch| is_kanji(*ch)) {
                if !seen.contains(&ch) {
                    seen.push(ch);
                }
            }
            seen
        };
        assert_eq!(point.characters(), expected, "{}", point.id);
        if expected.is_empty() {
            continue;
        }
        with_characters += 1;
        for ch in &expected {
            total += 1;
            if kanji.get(*ch).is_none() {
                outside.push(format!("{} writes {ch}", point.pattern));
            }
        }
    }
    assert_eq!(with_characters, 107, "the patterns written with a kanji");
    assert_eq!(total, 143, "characters across those patterns, each counted once");
    assert!(
        outside.is_empty(),
        "a pattern writing a character outside the jōyō set would offer a card the \
         board cannot open: {outside:?}"
    );
}

/// The drill's own account of the course, pinned.
///
/// The rule that finds a blank is a *reading* of the corpus rather than a field of
/// it (see `nihongo_core::grammar_drill`), so the numbers it comes to are the
/// artifact's numbers and belong here with the rest of them: if the corpus moves, or
/// the rule changes what it can locate, this is where the change is deliberate.
///
/// **363 of the 526 points and 903 of the 1,526 sentences** at a three-character
/// minimum, against 343 and 852 before the bracketed groups were read as what they
/// are — 17 points were reachable only because `〜あまり（に）` holds an optional に
/// rather than the lump `あまり（に）`, which occurs in no Japanese sentence.
#[test]
fn the_drill_can_ask_most_of_the_course_from_the_corpus_own_sentences() {
    use nihongo_core::grammar_drill::{askable, blank_for, can_distract, size};

    let dataset = dataset();
    let measured = size(&dataset);
    assert_eq!(measured.total, 526);
    assert_eq!(measured.points, 363, "363 of the 526 patterns have a sentence the rule can blank");
    assert_eq!(measured.examples, 903, "over 903 of the 1,526 sentences");

    // And over every one of those sentences, the three properties the question
    // rests on: the blank is at least as long as the floor, it occurs exactly once
    // (a hole with two places to go is not a question), and no other point's pattern
    // says exactly what came out (or two options would be right).
    let mut clashes = 0usize;
    let mut clash_points = std::collections::BTreeSet::new();
    for answer in askable(&dataset) {
        assert!(
            answer.blank.text.chars().count() >= nihongo_core::MIN_BLANK,
            "{} asked a blank of {}",
            answer.point.id,
            answer.blank.text.chars().count()
        );
        assert_eq!(
            answer.example.ja.matches(&answer.blank.text).count(),
            1,
            "{}'s blank occurs more than once in {}",
            answer.point.id,
            answer.example.ja
        );
        // The split keeps the sentence whole, which is what the screen draws.
        assert_eq!(
            answer.before().iter().map(|part| part.ruby.as_str()).collect::<String>()
                + &answer.blank.text
                + &answer.after().iter().map(|part| part.ruby.as_str()).collect::<String>(),
            answer.example.ja,
            "{} lost or invented text",
            answer.point.id
        );
        if dataset.points().iter().any(|other| {
            other.id != answer.point.id
                && nihongo_core::lumps(&other.pattern).contains(&answer.blank.text)
        }) {
            clashes += 1;
            clash_points.insert(answer.point.id.clone());
        }
    }
    assert_eq!(clashes, 34, "34 sentences have a point elsewhere that says exactly what came out");
    assert_eq!(clash_points.len(), 16, "and 16 points are involved");

    // Every point that can be asked has a question with three distinct options and
    // the answer among them, and every option that is not the answer is safe.
    let mut asked = 0usize;
    for answer in askable(&dataset) {
        let options: Vec<&nihongo_core::GrammarPoint> = dataset
            .points()
            .iter()
            .filter(|other| can_distract(&answer, other))
            .collect();
        assert!(
            options.len() >= nihongo_core::OPTION_COUNT - 1,
            "{} cannot find two safe distractors",
            answer.point.id
        );
        asked += 1;
    }
    assert_eq!(asked, 903);

    // A pattern the corpus stores with an optional particle is asked at all only
    // because the brackets are read as what they are.
    let amari = dataset.get("amari-ni").expect("〜あまり（に） is in the corpus");
    assert_eq!(amari.pattern, "〜あまり（に）");
    assert!(
        amari.examples.iter().any(|example| blank_for(amari, example).is_some()),
        "〜あまり（に） is askable because あまりに is one of its lumps"
    );
    assert!(
        nihongo_core::lumps(&amari.pattern).iter().any(|lump| lump == "あまりに"),
        "the optional に is one of the pattern's own readings: {:?}",
        nihongo_core::lumps(&amari.pattern)
    );
    let blank = blank_for(amari, &amari.examples[0]).expect("its first sentence can be asked");
    assert!(
        blank.text == "あまり" || blank.text == "あまりに",
        "{:?} is not one of the pattern's readings",
        blank.text
    );
}

/// The other half of asking: the same sentences, with the learner **writing** the
/// pattern instead of choosing it (`invariant 41`).
///
/// The measurement this pins is the reach. Production needs no distractors, so its
/// pool is the blank rule's own — and every blank it finds has a reading the input
/// engine can produce, so it reaches exactly what the recognition drill reaches:
/// **363 of the 526 patterns over 903 sentences, with nothing refused**. The answer
/// is the corpus's own reading of the hole, and the longest one in the course is
/// `ていただけませんか`.
#[test]
fn the_production_exercise_asks_the_same_sentences_the_other_way() {
    let dataset = dataset();
    let size = nihongo_core::production_size(&dataset);
    assert_eq!(size.points, 363);
    assert_eq!(size.examples, 903);
    assert_eq!(size.total, 526);
    assert_eq!(
        size,
        nihongo_core::grammar_drill_size(&dataset),
        "the two exercises ask the same pool, differently"
    );

    let mut longest = String::new();
    for answer in nihongo_core::askable(&dataset) {
        let kana = answer.blank_kana();
        assert!(!kana.is_empty(), "{}'s blank has no reading", answer.point.id);
        assert!(!kana.chars().any(is_kanji), "{}'s answer is not kana: {kana}", answer.point.id);
        if kana.chars().count() > longest.chars().count() {
            longest = kana.clone();
        }
        let romaji = nihongo_core::romaji_for(&kana)
            .unwrap_or_else(|| panic!("{}'s answer {kana} cannot be typed", answer.point.id));
        assert!(nihongo_core::matches_kana_text(&kana, &romaji));
    }
    assert_eq!(longest, "ていただけませんか");

    // One named question, end to end: 犬の方が猫より好きです。 takes の方が out and the
    // learner writes のほうが, which the data layer grades from the corpus's reading.
    let yori = dataset.get("no-hou-ga-yori").expect("〜の方が〜より is in the corpus");
    assert_eq!(yori.pattern, "〜の方が〜より");
    let answer = nihongo_core::askable_of(yori).into_iter().next().expect("it can be asked");
    assert_eq!(answer.blank.text, "の方が");
    assert_eq!(answer.blank_kana(), "のほうが");
    let right = nihongo_core::verdict_for(yori, 0, "nohouga").expect("a verdict");
    assert!(right.correct);
    assert_eq!(right.written, "の方が");
    assert_eq!(right.answer, "のほうが");
    assert_eq!(right.produced, "のほうが");
    let wrong = nihongo_core::verdict_for(yori, 0, "yori").expect("a verdict");
    assert!(!wrong.correct);
    assert_eq!(wrong.pattern, yori.pattern, "the verdict carries the lesson");
}

/// What the holes a learner writes actually are, counted over the committed course.
///
/// Three shapes, and each decides something rather than decorating a document:
/// **172 of the 903** holes take a kanji, so those questions are answered by a reading
/// — the corpus's own, never one composed here — while the other 731 are plain kana
/// and answered by what is written. **4 of the 903** answers contain ぢ or づ, which no
/// spelling of the input table produces (`ji` types じ), so the yotsugana fold is
/// load-bearing for exactly those four and the test proves it on them by typing the
/// common kana's spelling. And **none** contains katakana: the corpus's furigana over a
/// pattern's own text is hiragana throughout, so the katakana half of the comparison is
/// defensive rather than used — the input engine's own tests cover it.
#[test]
fn the_hole_a_learner_writes_is_what_the_corpus_reads() {
    let dataset = dataset();
    let (mut kanji, mut plain, mut yotsugana) = (0usize, 0usize, 0usize);
    for answer in nihongo_core::askable(&dataset) {
        let kana = answer.blank_kana();
        assert!(!kana.chars().any(is_kanji), "{}'s answer is not kana", answer.point.id);
        assert!(
            !kana.chars().any(|ch| ('\u{30A1}'..='\u{30FA}').contains(&ch)),
            "{}'s answer is katakana: {kana}",
            answer.point.id
        );
        let takes_a_kanji = answer.blank.text.chars().any(is_kanji);
        if takes_a_kanji {
            kanji += 1;
            assert_ne!(kana, answer.blank.text, "{} composed a reading", answer.point.id);
        } else {
            plain += 1;
            assert_eq!(kana, answer.blank.text, "{}'s answer is not what it writes", answer.point.id);
        }
        // The reading the corpus gives, typed back, is right.
        let romaji = nihongo_core::romaji_for(&kana).expect("the answer is typeable");
        assert!(nihongo_core::matches_kana_text(&kana, &romaji));

        if kana.chars().any(|ch| matches!(ch, 'ぢ' | 'づ')) {
            yotsugana += 1;
            // The fold, on a real answer: the same sound spelled with the *common*
            // kana is accepted, because `ji` and `zu` are what the input table
            // produces and ぢ and づ are what the corpus wrote.
            let common: String = kana
                .chars()
                .map(|ch| match ch {
                    'ぢ' => 'じ',
                    'づ' => 'ず',
                    other => other,
                })
                .collect();
            assert_ne!(common, kana);
            let typed = nihongo_core::romaji_for(&common).expect("the common spelling is typeable");
            assert!(
                nihongo_core::matches_kana_text(&kana, &typed),
                "{}: {common} typed as {typed} must be accepted for {kana}",
                answer.point.id
            );
        }
    }
    assert_eq!(kanji, 172, "holes that take a kanji, answered by their reading");
    assert_eq!(plain, 731, "holes that are kana already, answered by what is written");
    assert_eq!(kanji + plain, 903);
    assert_eq!(yotsugana, 4, "answers whose reading the input table cannot spell exactly");
}

/// Free composition: a sentence of the learner's own, and the three measurements
/// that say what may honestly be put beside it (`invariant 42`).
///
/// The exercise is the one grammar question the corpus cannot answer for the app, so
/// every claim it makes had to be measured against the corpus first:
///
/// * **the pool is the whole course** — 526 of 526 — because the prompt is the
///   pattern itself and needs no sentence, which is the one reach the drill's 363 and
///   the written exercise's 363 do not have;
/// * **the pattern's own text is not a gate**, and this is the measurement the whole
///   exercise turns on: **1,271 of the 1,526** examples contain a lump of their own
///   pattern and **255** do not, because the corpus writes the pattern inflected
///   (〜ている as しています, 〜に行く as 行きます). A checker that demanded the
///   pattern's letters would refuse 255 of the corpus's own teaching sentences;
/// * **the same for the characters**: exactly two of the 1,526 sentences write a
///   character outside the course (几 and 嘘), so that is a fact the screen states
///   rather than a fault it refuses;
/// * **the floor is the corpus's own shortest sentence**: six kana/kanji characters,
///   which is `MIN_COMPOSITION`.
#[test]
fn a_sentence_of_the_learners_own_is_measured_against_the_corpus() {
    let dataset = dataset();
    let kanji = KanjiDataset::from_gzip_bytes(include_bytes!("../data/kanji.bin.gz"))
        .expect("the committed kanji artifact decodes");

    // The reach: every point of the course, and a draw that returns a pattern of it.
    let size = nihongo_core::composition_size(&dataset);
    assert_eq!(size.total, 526);
    assert_eq!(size.points, size.total, "the prompt is the pattern, so nothing is unaskable");
    let log = nihongo_core::CompositionLog::new();
    let weights = GrammarLog::new();
    let drawn = nihongo_core::draw_composition(&dataset, &weights, &log, 0.0)
        .expect("the course has a first pattern");
    assert_eq!(drawn.point, "wa", "an even weight draws the course's first pattern");
    assert!(!drawn.examples.is_empty(), "the model travels with the prompt");
    assert!(drawn.written.is_empty(), "and nothing has been written yet");

    // The pattern's own text, counted over every sentence the corpus wrote.
    let mut contains = 0usize;
    let mut misses: Vec<String> = Vec::new();
    let mut shortest = usize::MAX;
    let mut those_characters: Vec<char> = Vec::new();
    for point in dataset.points() {
        let lumps = nihongo_core::own_lumps(point);
        for example in point.examples() {
            if lumps.iter().any(|lump| example.ja.contains(lump.as_str())) {
                contains += 1;
            } else if misses.len() < 3 {
                misses.push(format!("{}: {}", point.id, example.ja));
            }
            let length = example
                .ja
                .chars()
                .filter(|ch| nihongo_core::Script::of(*ch).is_some() || is_kanji(*ch))
                .count();
            shortest = shortest.min(length);
            for ch in example.ja.chars().filter(|ch| is_kanji(*ch)) {
                if kanji.get(ch).is_none() && !those_characters.contains(&ch) {
                    those_characters.push(ch);
                }
            }
        }
    }
    assert_eq!(
        contains, 1271,
        "the rule that would have been a gate, and the corpus's own sentences that fail it"
    );
    assert_eq!(contains + 255, dataset.example_count());
    assert_eq!(
        misses[0].split(':').next(),
        Some("ga-hoshii"),
        "〜がほしい is taught with ほしくない: {misses:?}"
    );
    assert_eq!(
        shortest, MIN_COMPOSITION,
        "the floor the corpus itself sets: 私は学生です。 and 危ない！止まれ！"
    );
    assert_eq!(
        those_characters,
        vec!['几', '嘘'],
        "two of the corpus's own sentences write a character this course has no card for"
    );

    // End to end, over the committed artifacts. 〜ている is the inflected case: the
    // corpus's own sentence is kept even though the pattern's letters are not in it.
    let te_iru = dataset.get("te-iru").expect("〜ている is in the course");
    let inflected = nihongo_core::composition_verdict_for(
        te_iru,
        &kanji,
        &te_iru.examples[0].ja,
    );
    assert!(inflected.found.is_empty(), "しています does not contain ている");
    assert!(inflected.kept, "and the sentence is a sentence: {inflected:?}");
    assert_eq!(inflected.copied, Some(0), "it is the corpus's own sentence, written back");

    // And the case the check does see: 〜の方が〜より, whose sentence is the corpus's.
    let yori = dataset.get("no-hou-ga-yori").expect("〜の方が〜より is in the course");
    let written = nihongo_core::composition_verdict_for(yori, &kanji, "犬の方が猫より好きです。");
    assert_eq!(written.found, vec!["の方が", "より"], "both lumps, longest first");
    assert!(written.kept && written.readable);
    assert!(written.outside.is_empty(), "every character is one the course teaches");
    assert_eq!(written.copied, Some(0), "the corpus wrote this sentence too");

    // The same pattern written in kana, which is what the reading is read for.
    let kana = nihongo_core::composition_verdict_for(yori, &kanji, "いぬのほうがねこよりすきです。");
    assert_eq!(kana.found, vec!["のほうが", "より"]);
    assert!(kana.kept, "a kana sentence is a sentence: {kana:?}");
    assert_eq!(kana.copied, None, "and it is not the corpus's spelling of it");
}
