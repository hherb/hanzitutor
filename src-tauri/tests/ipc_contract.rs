//! Integration tests for the boundary the webview sees.
//!
//! These lock two things that unit tests inside `hanzi-core` cannot:
//!
//! 1. the embedded dataset actually decodes, and
//! 2. the JSON field names match what the TypeScript client reads.
//!
//! A mismatch there is silent — the UI would simply show blanks — so the field
//! names are asserted explicitly rather than by round-tripping through a struct.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use hanzi_core::{GradeOptions, Point, ProgressStore, ReviewSource, VocabStore};
use hanzi_tutor_lib::{AppState, REVIEW_LIMIT};

fn state() -> AppState {
    // `None` keeps the study documents in memory, so tests never touch the
    // user's real data files.
    AppState::load(None).expect("the embedded dataset should decode")
}

/// A private directory for one test, so tests never collide.
fn data_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hanzi-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Top-level keys of a serialised value.
fn keys(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("expected a JSON object")
        .keys()
        .cloned()
        .collect()
}

fn expect_keys(value: &serde_json::Value, expected: &[&str]) {
    let actual = keys(value);
    let expected: BTreeSet<String> = expected.iter().map(|k| k.to_string()).collect();
    assert_eq!(
        actual, expected,
        "\nmissing: {:?}\nunexpected: {:?}",
        expected.difference(&actual).collect::<Vec<_>>(),
        actual.difference(&expected).collect::<Vec<_>>(),
    );
}

#[test]
fn embedded_dataset_loads_with_the_expected_coverage() {
    let stats = state().stats();
    assert_eq!(stats.lesson_size, 10);
    assert!(
        stats.characters > 9_000,
        "expected the full dataset, got {}",
        stats.characters
    );
    assert!(
        stats.teachable > 7_000,
        "expected most characters to be teachable, got {}",
        stats.teachable
    );
    assert_eq!(
        stats.lessons,
        stats.teachable.div_ceil(stats.lesson_size),
        "lessons should partition the teachable characters"
    );
    assert!(
        stats.words > 9_000,
        "expected the HSK word list, got {} words",
        stats.words
    );
}

#[test]
fn stats_serialise_with_camel_case_fields() {
    let stats = state().stats();
    let json = serde_json::to_value(&stats).unwrap();
    expect_keys(
        &json,
        &[
            "characters",
            "teachable",
            "lessons",
            "lessonSize",
            "words",
            "wordLevels",
        ],
    );

    // The sidebar lists the levels in order, with the count behind each.
    expect_keys(&json["wordLevels"][0], &["level", "words"]);
    let levels: Vec<u64> = json["wordLevels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["level"].as_u64().unwrap())
        .collect();
    assert_eq!(levels, vec![1, 2, 3, 4, 5, 6, 7], "lowest level first");
    assert_eq!(
        json["wordLevels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["words"].as_u64().unwrap())
            .sum::<u64>(),
        stats.words as u64,
        "every word belongs to exactly one level"
    );
}

#[test]
fn lessons_serialise_with_camel_case_fields() {
    let state = state();
    let lessons = state.lessons();
    assert!(!lessons.is_empty());

    let json = serde_json::to_value(&lessons[0]).unwrap();
    expect_keys(
        &json,
        &["index", "title", "characters", "firstRank", "lastRank"],
    );

    // The first lesson is the most common characters, starting at rank 1.
    assert_eq!(lessons[0].first_rank, 1);
    assert_eq!(lessons[0].characters.len(), 10);
    assert_eq!(
        lessons[0].characters.len(),
        lessons[0].characters.iter().collect::<BTreeSet<_>>().len(),
        "a lesson should not repeat a character"
    );
}

#[test]
fn character_serialises_with_camel_case_fields() {
    let state = state();
    let character = state.character('一').expect("一 should be in the dataset");
    assert_eq!(character.pinyin, vec!["yī".to_string()]);
    assert_eq!(character.outlines.len(), 1);

    let json = serde_json::to_value(&character).unwrap();
    expect_keys(
        &json,
        &[
            "ch",
            "rank",
            "hsk",
            "strokeCount",
            "radical",
            "pinyin",
            "definition",
            "etymology",
            "outlines",
            "medians",
        ],
    );

    // A character is addressed by its literal self, which crosses the IPC
    // boundary as a one-character JSON string.
    assert_eq!(json["ch"], serde_json::json!("一"));
}

#[test]
fn unknown_characters_are_reported_not_guessed() {
    let state = state();
    // A Latin letter is definitely not in the dataset.
    let error = state.character('A').unwrap_err();
    assert!(error.contains('A'), "unhelpful error: {error}");
    assert!(state.grade('A', &[], &GradeOptions::default()).is_err());
}

#[test]
fn grading_a_reference_stroke_against_itself_is_perfect() {
    let state = state();
    let character = state.character('一').unwrap();
    let report = state
        .grade('一', &character.medians, &GradeOptions::default())
        .unwrap();

    assert_eq!(report.expected_strokes, 1);
    assert_eq!(report.given_strokes, 1);
    assert!(report.count_ok);
    assert!(report.legible);
    assert!(report.order_correct);
    assert!(report.is_perfect());
    assert_eq!(report.overall, 100.0);
    assert_eq!(report.grade, hanzi_core::Grade::Excellent);
}

#[test]
fn report_serialises_with_camel_case_fields_and_snake_case_verdicts() {
    let state = state();
    // 十 is a horizontal stroke followed by a vertical one.
    let character = state.character('十').unwrap();
    let report = state
        .grade('十', &character.medians, &GradeOptions::default())
        .unwrap();

    let json = serde_json::to_value(&report).unwrap();
    expect_keys(
        &json,
        &[
            "expectedStrokes",
            "givenStrokes",
            "strayStrokes",
            "countOk",
            "strokes",
            "assignment",
            "shapeScore",
            "positionScore",
            "inkScore",
            "inkCoverage",
            "orderScore",
            "overall",
            "legible",
            "orderCorrect",
            "grade",
            "firstError",
            "fit",
        ],
    );

    expect_keys(
        &json["strokes"][0],
        &[
            "refIndex",
            "userIndex",
            "verdict",
            "shape",
            "position",
            "ink",
            "score",
        ],
    );

    // Verdict and grade are snake_case so they can be used as literal unions in
    // TypeScript and switched on exhaustively.
    assert_eq!(json["grade"], serde_json::json!("excellent"));
    assert_eq!(json["strokes"][0]["verdict"], serde_json::json!("correct"));
    assert_eq!(json["firstError"], serde_json::Value::Null);
}

#[test]
fn a_swapped_pair_of_strokes_is_legible_but_out_of_order() {
    let state = state();
    let character = state.character('十').unwrap();
    assert_eq!(character.medians.len(), 2, "十 is two strokes");

    // Write the vertical stroke first, then the horizontal one.
    let attempt = vec![character.medians[1].clone(), character.medians[0].clone()];
    let report = state
        .grade('十', &attempt, &GradeOptions::default())
        .unwrap();

    assert!(report.count_ok);
    assert!(report.legible, "the strokes are correct, just reordered");
    assert!(!report.order_correct);
    assert_eq!(report.strokes[0].verdict, hanzi_core::Verdict::OutOfOrder);
    assert_eq!(report.strokes[1].verdict, hanzi_core::Verdict::OutOfOrder);

    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["strokes"][0]["verdict"], serde_json::json!("out_of_order"));
    assert_eq!(json["strokes"][0]["userIndex"], serde_json::json!(1));
}

#[test]
fn strokes_are_accepted_in_display_space_from_a_canvas() {
    // The frontend sends `{ x, y }` objects; a plain array of arrays would fail
    // to deserialise, so exercise the real JSON shape.
    let state = state();
    let strokes: Vec<Vec<Point>> = serde_json::from_value(serde_json::json!([
        [{ "x": 120.0, "y": 400.0 }, { "x": 900.0, "y": 400.0 }]
    ]))
    .expect("the frontend's point shape should deserialise");

    let report = state
        .grade('一', &strokes, &GradeOptions::default())
        .unwrap();
    assert!(report.legible, "{report:#?}");
    assert!(report.overall > 90.0, "overall {}", report.overall);
}

#[test]
fn grade_options_accept_camel_case_from_the_frontend() {
    let parsed: GradeOptions = serde_json::from_value(serde_json::json!({
        "resampleK": 16,
        "minStrokeLen": 12,
        "globalFit": false,
    }))
    .expect("camelCase options should deserialise");
    assert_eq!(parsed.resample_k, 16);
    assert_eq!(parsed.min_stroke_len, 12.0);
    assert!(!parsed.global_fit);
    // Options sent without a pen width predate the raster measure; they get the
    // canvas default rather than a deserialisation failure.
    assert_eq!(parsed.ink_width, hanzi_core::INK_WIDTH);

    let explicit: GradeOptions = serde_json::from_value(serde_json::json!({
        "resampleK": 16,
        "minStrokeLen": 12,
        "globalFit": true,
        "inkWidth": 20.0,
    }))
    .expect("the pen width is part of the contract");
    assert_eq!(explicit.ink_width, 20.0);
}

#[test]
fn a_json_null_option_is_the_same_as_omitting_it() {
    // The frontend sends `options: null` when it has no preference.
    let parsed: Option<GradeOptions> = serde_json::from_value(serde_json::Value::Null).unwrap();
    assert!(parsed.is_none());
}

// ---- vocabulary list ------------------------------------------------------

/// A list with one entry, as the interface would receive it.
fn vocab_fixture() -> hanzi_core::VocabView {
    let mut store = VocabStore::in_memory();
    let id = store
        .add_entry("学习", "xuéxí", "to study", Some("Lesson 3"))
        .expect("adding should succeed")
        .id;
    store.record_attempt(id, 87.0).expect("recording should work");
    store.view()
}

#[test]
fn text_lookup_serialises_with_camel_case_fields() {
    let state = state();
    let lookup = state.dataset.lookup_text("学习");
    let json = serde_json::to_value(&lookup).unwrap();
    expect_keys(&json, &["pinyin", "meaning", "characters", "complete"]);
    expect_keys(&json["characters"][0], &["ch", "pinyin", "meaning"]);
}

#[test]
fn a_word_lookup_uses_the_dictionary_rather_than_inventing_a_meaning() {
    // Against the shipped dataset, not a fixture: this is the behaviour the add
    // form depends on.
    let state = state();

    let word = state.dataset.lookup_text("学习");
    assert_eq!(word.pinyin, "xuéxí", "the word's own reading");
    assert!(
        !word.meaning.is_empty(),
        "M3: a word's meaning now comes from the dictionary"
    );
    assert!(word.complete);
    assert_eq!(word.characters.len(), 2);
    assert_eq!(word.characters[1].ch, '习');

    // A single character is its own word, so both fields are filled.
    let character = state.dataset.lookup_text("好");
    assert_eq!(character.pinyin, "hǎo");
    assert!(!character.meaning.is_empty());
    assert!(character.complete);
}

#[test]
fn a_polyphonic_word_is_read_from_its_own_entry() {
    // The correctness problem M3 exists to fix: the character 着 is offered by
    // the synthesiser as `zhe`, but 着急 is `zháojí`.
    let state = state();
    let word = state.dataset.lookup_text("着急");
    assert_eq!(word.pinyin, "zháojí");
    assert!(word.meaning.contains("worry"), "got {:?}", word.meaning);
    assert_ne!(
        state.dataset.lookup_text("着").pinyin,
        "zháo",
        "the isolated character has no context, which is the point"
    );
}

#[test]
fn a_word_that_is_not_in_the_dictionary_still_invents_no_meaning() {
    let state = state();
    // 学 is a character and 龙 is a character, but 学龙 is not an HSK word.
    let unknown = state.dataset.lookup_text("学龙");
    assert_eq!(unknown.pinyin, "xuélóng", "the readings still compose");
    assert_eq!(unknown.meaning, "", "no meaning may be invented");
}

#[test]
fn a_lookup_of_something_unknown_is_flagged_incomplete() {
    let state = state();
    let mixed = state.dataset.lookup_text("学Q");
    assert!(!mixed.complete);
    assert_eq!(mixed.pinyin, "xué", "the known character still contributes");
    assert!(mixed.characters[1].pinyin.is_empty());

    assert!(!state.dataset.lookup_text("").complete);
}

// ---- the word dictionary --------------------------------------------------

#[test]
fn words_serialise_with_camel_case_fields() {
    let state = state();
    let word = state.dataset.word("学习").expect("学习 is an HSK word");
    let json = serde_json::to_value(word).unwrap();
    expect_keys(&json, &["text", "pinyin", "meaning", "hsk", "rank"]);
    assert_eq!(json["text"], serde_json::json!("学习"));
    assert_eq!(json["hsk"], serde_json::json!(1));
    // The reading is the word's own, spaces removed, which is how the app
    // writes pinyin everywhere else.
    assert_eq!(json["pinyin"], serde_json::json!("xuéxí"));
}

#[test]
fn the_search_view_reports_a_page_and_an_honest_total() {
    let state = state();
    let view = state.search_words("学", None, 5);
    let json = serde_json::to_value(&view).unwrap();
    expect_keys(&json, &["words", "total"]);
    expect_keys(&json["words"][0], &["text", "pinyin", "meaning", "hsk", "rank"]);

    assert_eq!(view.words.len(), 5, "the page is capped");
    assert!(
        view.total > view.words.len(),
        "but the total says how many matched: {}",
        view.total
    );
    assert!(
        view.words.iter().all(|word| word.text.contains('学')),
        "every hit really contains the character"
    );
}

#[test]
fn browsing_from_the_top_starts_at_the_most_useful_words() {
    let state = state();
    let page = state.search_words("", None, 100);
    assert_eq!(page.words.len(), 100);
    assert_eq!(page.total, state.stats().words, "browsing matches everything");
    assert!(
        page.words.iter().all(|word| word.hsk == 1),
        "the most useful words are the first HSK level"
    );
    // The top of the list is the everyday words a beginner meets first, which
    // is what the derived frequency — the rarest character's rank — is for.
    let top: Vec<&str> = page.words.iter().take(10).map(|w| w.text.as_str()).collect();
    assert!(
        top.contains(&"他们") && top.contains(&"我们"),
        "expected the most common pronouns at the top, got {top:?}"
    );
    // 学习 is an HSK 1 word, so browsing that level in full must reach it.
    let hsk1 = state.search_words("", Some(1), 300);
    assert!(
        hsk1.words.iter().any(|word| word.text == "学习"),
        "学习 should be in the HSK 1 list"
    );
}

#[test]
fn a_level_filter_narrows_the_search_to_that_level() {
    let state = state();
    let hsk2 = state.search_words("", Some(2), 100);
    assert!(hsk2.words.iter().all(|word| word.hsk == 2));
    let expected = state
        .stats()
        .word_levels
        .iter()
        .find(|entry| entry.level == 2)
        .map(|entry| entry.words)
        .expect("HSK 2 should have words");
    assert_eq!(hsk2.total, expected);
}

#[test]
fn words_can_be_found_by_reading_and_by_meaning() {
    let state = state();

    let by_reading = state.search_words("xuexi", None, 10);
    assert!(
        by_reading.words.iter().any(|word| word.text == "学习"),
        "a reading without tone marks finds the word"
    );

    let by_meaning = state.search_words("teacher", None, 10);
    assert!(
        by_meaning.words.iter().any(|word| word.text == "老师"),
        "an English word finds it via the definition, got {:?}",
        by_meaning
            .words
            .iter()
            .map(|w| &w.text)
            .collect::<Vec<_>>()
    );
}

#[test]
fn every_word_can_be_drawn_character_by_character() {
    // The acceptance criterion, against the real data: a word is practised by
    // writing each of its characters, so every one of them must be on the board.
    let state = state();
    let teachable: BTreeSet<char> = state.dataset.practisable_characters().into_iter().collect();
    assert!(teachable.contains(&'学'));
    assert!(!teachable.contains(&'，'), "punctuation is not drawable");

    let mut checked = 0usize;
    for word in state.dataset.words() {
        for ch in word.characters() {
            assert!(
                teachable.contains(&ch),
                "{} cannot be practised: {ch} has no strokes",
                word.text
            );
        }
        checked += 1;
    }
    assert!(checked > 9_000, "checked only {checked} words");
}

#[test]
fn a_sentence_is_practised_one_character_at_a_time_without_the_punctuation() {
    // Words are the milestone; the same path makes any text — a sentence — a
    // sequence of characters, with the marks the board cannot draw skipped.
    let state = state();
    let teachable: BTreeSet<char> = state.dataset.practisable_characters().into_iter().collect();
    let sentence = "我爱学习。";
    let drawable: Vec<char> = sentence.chars().filter(|ch| teachable.contains(ch)).collect();
    assert_eq!(drawable, vec!['我', '爱', '学', '习']);
    // And each of them really is offered by the dictionary.
    for ch in &drawable {
        assert!(state.dataset.get(*ch).is_some(), "{ch} should be loaded");
    }
}

#[test]
fn vocabulary_serialises_with_camel_case_fields() {
    let json = serde_json::to_value(vocab_fixture()).unwrap();
    expect_keys(&json, &["entries", "groups", "warning"]);
    assert_eq!(json["groups"], serde_json::json!(["Lesson 3"]));
    assert_eq!(json["warning"], serde_json::Value::Null);
}

#[test]
fn vocabulary_entries_serialise_with_camel_case_fields() {
    let json = serde_json::to_value(vocab_fixture()).unwrap();
    expect_keys(
        &json["entries"][0],
        &[
            "id",
            "text",
            "pinyin",
            "meaning",
            "group",
            "addedAt",
            "attempts",
            "bestScore",
            "lastPractised",
        ],
    );
    assert_eq!(json["entries"][0]["text"], serde_json::json!("学习"));
    assert_eq!(json["entries"][0]["attempts"], serde_json::json!(1));
    assert_eq!(json["entries"][0]["bestScore"], serde_json::json!(87.0));
    // A timestamp is an ISO-8601 UTC string, which sorts chronologically as
    // plain text — the review scheduling will rely on that.
    let added = json["entries"][0]["addedAt"].as_str().unwrap();
    assert!(added.ends_with('Z') && added.len() == 20, "got {added}");
}

#[test]
fn vocab_outcome_serialises_for_the_status_line() {
    let outcome = hanzi_tutor_lib::VocabOutcome {
        view: vocab_fixture(),
        message: "Added 1 entries".to_string(),
    };
    let json = serde_json::to_value(outcome).unwrap();
    expect_keys(&json, &["view", "message"]);
    expect_keys(&json["view"], &["entries", "groups", "warning"]);
}

#[test]
fn the_default_state_keeps_the_vocabulary_list_in_memory() {
    let state = state();
    let mut vocab = state.lock_vocab();
    assert!(vocab.store.entries().is_empty());
    // Nothing to persist, and no reason to complain about that.
    assert!(vocab.load_error.is_none());
    assert!(vocab.view().warning.is_none());
    assert!(vocab.save().is_none());
}

#[test]
fn the_vocabulary_list_persists_through_the_state_layer() {
    // Exercises the whole chain that the unit tests do not: state wiring, the
    // data directory, the real file name, and reading it back in a fresh
    // session.
    let dir = std::env::temp_dir().join(format!("hanzi-vocab-persist-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();

    let id = {
        let state = AppState::load(Some(dir.clone())).unwrap();
        let mut vocab = state.lock_vocab();
        let entry = vocab
            .store
            .add_entry("学习", "xuéxí", "to study", Some("Lesson 3"))
            .expect("adding should succeed");
        assert!(vocab.save().is_none(), "saving a fresh list should succeed");
        entry.id
    };

    // One database, and no JSON beside it: a fresh install writes `hanzi.db` and
    // nothing else.
    let path = dir.join("hanzi.db");
    assert!(path.exists(), "expected a file at {}", path.display());
    assert!(
        !dir.join("vocabulary.json").exists(),
        "the list should no longer be a JSON document of its own"
    );

    // A second session, as if the app had been restarted.
    let state = AppState::load(Some(dir.clone())).unwrap();
    let vocab = state.lock_vocab();
    assert!(vocab.load_error.is_none(), "{:?}", vocab.load_error);
    let entries = vocab.store.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, id);
    assert_eq!(entries[0].text, "学习");
    assert_eq!(entries[0].group.as_deref(), Some("Lesson 3"));
    assert!(vocab.view().warning.is_none());

    drop(vocab);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_corrupt_saved_list_is_reported_rather_than_silently_replaced() {
    // A corrupt file must not be overwritten by an empty list: that would
    // destroy study notes because of a parse error.
    let dir = std::env::temp_dir().join(format!(
        "hanzi-vocab-state-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("vocabulary.json");
    std::fs::write(&path, "{ not json").unwrap();

    let state = AppState::load(Some(dir.clone())).unwrap();
    let mut vocab = state.lock_vocab();

    let warning = vocab
        .view()
        .warning
        .clone()
        .expect("a corrupt file must produce a warning");
    assert!(warning.contains("could not be read"), "{warning}");
    // Saving is refused, so the file on disk is left exactly as it was.
    assert!(vocab.save().is_some(), "saving must be refused");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");

    drop(vocab);
    std::fs::remove_dir_all(&dir).ok();
}

// ---- practice progress and review -----------------------------------------

/// Write a `progress.json` holding one due card per character.
fn write_schedule(dir: &Path, chars: &[char]) {
    let mut cards = serde_json::Map::new();
    for ch in chars {
        cards.insert(
            ch.to_string(),
            serde_json::json!({
                "attempts": 1,
                "lapses": 1,
                "bestScore": 20.0,
                "lastScore": 20.0,
                "lastPractised": "2020-01-01T00:00:00Z",
                "due": "2020-01-01T00:01:00Z",
                "intervalDays": 0.0,
                "ease": 1.96,
                "repetitions": 0,
                "history": [
                    { "at": "2020-01-01T00:00:00Z", "score": 20.0, "rating": "again" }
                ],
            }),
        );
    }
    let document = serde_json::json!({ "version": 1, "cards": serde_json::Value::Object(cards) });
    std::fs::write(dir.join("progress.json"), document.to_string()).unwrap();
}

#[test]
fn progress_serialises_with_camel_case_fields() {
    let mut store = ProgressStore::in_memory();
    store
        .record_at('好', 88.0, "2026-09-19T09:00:00Z")
        .expect("recording should work");
    let json = serde_json::to_value(store.view_at("2026-09-19T09:30:00Z")).unwrap();

    expect_keys(&json, &["cards", "warning"]);
    assert_eq!(json["warning"], serde_json::Value::Null);
    expect_keys(
        &json["cards"][0],
        &[
            "ch",
            "attempts",
            "lapses",
            "bestScore",
            "lastScore",
            "lastPractised",
            "due",
            "intervalDays",
            "ease",
            "repetitions",
            "history",
            "dueNow",
        ],
    );
    // A card is addressed by its character, which crosses the IPC boundary as a
    // one-character JSON string.
    assert_eq!(json["cards"][0]["ch"], serde_json::json!("好"));
    assert_eq!(json["cards"][0]["dueNow"], serde_json::json!(false));

    expect_keys(&json["cards"][0]["history"][0], &["at", "score", "rating"]);
    // Ratings are snake_case so they can be literal unions in TypeScript.
    assert_eq!(json["cards"][0]["history"][0]["rating"], serde_json::json!("good"));
}

#[test]
fn review_queue_serialises_with_camel_case_fields() {
    let dir = data_dir("ipc-queue-shape");
    let state = state();
    let ch = state.lessons()[0].characters[0];
    write_schedule(&dir, &[ch]);

    let loaded = AppState::load(Some(dir.clone())).unwrap();
    let json = serde_json::to_value(loaded.review_queue()).unwrap();
    expect_keys(&json, &["items", "dueCount", "warning"]);
    expect_keys(
        &json["items"][0],
        &["ch", "due", "source", "entryId", "text"],
    );
    // Sources are snake_case, like verdicts and grades.
    assert_eq!(json["items"][0]["source"], serde_json::json!("course"));
    assert_eq!(json["items"][0]["entryId"], serde_json::Value::Null);
    assert_eq!(json["items"][0]["ch"], serde_json::json!(ch.to_string()));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cursor_serialises_with_camel_case_fields() {
    let mut store = hanzi_core::CursorStore::in_memory();
    store.set_index(7);
    let json = serde_json::to_value(store.view()).unwrap();
    expect_keys(&json, &["index", "updatedAt", "warning"]);
    assert_eq!(json["index"], serde_json::json!(7));
    assert!(json["updatedAt"].is_string(), "a move is timestamped");
}

#[test]
fn the_asr_status_serialises_with_camel_case_fields() {
    // The settings screen renders a download from this and nothing else, so its
    // keys are the contract: `downloadBytes` and `unpackedBytes` are what it shows
    // *before* anybody agrees to the download, and `downloaded` is what it turns
    // into a progress bar once somebody has.
    let state = state();
    let json = serde_json::to_value(state.asr.status()).unwrap();
    expect_keys(
        &json,
        &[
            "state",
            "installed",
            "model",
            "url",
            "licence",
            "licenceUrl",
            "downloadBytes",
            "unpackedBytes",
            "downloaded",
            "path",
            "error",
            "detail",
        ],
    );

    // A test has no data directory, so nothing can be installed — which is also
    // the state the app ships in, and the one the screen has to render first.
    assert_eq!(json["state"], serde_json::json!("absent"));
    assert_eq!(json["installed"], serde_json::json!(false));
    // The description of a download nobody has agreed to is present anyway: the
    // screen has to be able to state the address, the size and the licence before
    // the button is pressed, not after.
    assert!(
        json["url"].as_str().unwrap().starts_with("https://"),
        "the download address must be stated up front: {}",
        json["url"]
    );
    assert!(json["downloadBytes"].as_u64().unwrap() > 0);
    assert!(
        json["unpackedBytes"].as_u64().unwrap() > json["downloadBytes"].as_u64().unwrap(),
        "the model is larger unpacked than compressed, and the screen says both"
    );
    assert!(!json["licence"].as_str().unwrap().is_empty());
}

#[test]
fn settings_serialise_with_camel_case_fields() {
    let mut store = hanzi_core::SettingsStore::in_memory();
    assert!(store.set_click_to_draw(Some(true)));
    assert!(store.set_voice(Some("Meijia")));
    assert!(store.set_animation_pace(hanzi_core::Pace::Slow));
    assert!(store.set_board_size(hanzi_core::BoardSize::Compact));
    let json = serde_json::to_value(store.view()).unwrap();
    expect_keys(
        &json,
        &["clickToDraw", "voice", "animationPace", "boardSize", "warning"],
    );
    assert_eq!(json["clickToDraw"], serde_json::json!(true));
    assert_eq!(json["voice"], serde_json::json!("Meijia"));
    assert_eq!(json["warning"], serde_json::Value::Null);

    // The enum values are the names the settings screen sends back and the names
    // the database stores, so they are asserted rather than left to the derive:
    // a rename that reached only one of the three would silently reset the
    // learner's choice to the default.
    assert_eq!(json["animationPace"], serde_json::json!("slow"));
    assert_eq!(json["boardSize"], serde_json::json!("compact"));
}

#[test]
fn settings_the_screen_can_round_trip_through_a_patch() {
    // The screen sends only what changed, deserialised into the command's own
    // arguments. Parsing here is what proves the names on the wire are the names
    // the screen uses — `animationPace`, not `animation_pace`.
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Patch {
        #[serde(default)]
        click_to_draw: Option<bool>,
        #[serde(default)]
        voice: Option<String>,
        #[serde(default)]
        animation_pace: Option<hanzi_core::Pace>,
        #[serde(default)]
        board_size: Option<hanzi_core::BoardSize>,
    }

    let patch: Patch = serde_json::from_value(serde_json::json!({
        "animationPace": "fast",
        "boardSize": "large",
        "voice": "Tingting",
        "clickToDraw": false,
    }))
    .unwrap();
    assert_eq!(patch.click_to_draw, Some(false));
    assert_eq!(patch.voice.as_deref(), Some("Tingting"));
    assert_eq!(patch.animation_pace, Some(hanzi_core::Pace::Fast));
    assert_eq!(patch.board_size, Some(hanzi_core::BoardSize::Large));

    // A patch that names one preference leaves the rest absent, which is what
    // "leave this one alone" is on the wire.
    let partial: Patch = serde_json::from_value(serde_json::json!({ "voice": "Meijia" })).unwrap();
    assert_eq!(partial.voice.as_deref(), Some("Meijia"));
    assert!(partial.click_to_draw.is_none());
    assert!(partial.animation_pace.is_none());
    assert!(partial.board_size.is_none());
}

#[test]
fn an_unchosen_setting_is_null_rather_than_false() {
    // The interface reads this to decide whether to follow the device, so the
    // two states have to be distinguishable over the wire: `null` means "nobody
    // has chosen", `false` means "chosen: drag".
    let store = hanzi_core::SettingsStore::in_memory();
    let json = serde_json::to_value(store.view()).unwrap();
    assert_eq!(json["clickToDraw"], serde_json::Value::Null);
    assert_eq!(json["voice"], serde_json::Value::Null);

    // A pace and a board size are not nullable: there is no device signal to
    // resolve one from, so the stored absence means the default and the wire
    // carries the value the app will actually use.
    assert_eq!(json["animationPace"], serde_json::json!("normal"));
    assert_eq!(json["boardSize"], serde_json::json!("normal"));

    let mut store = hanzi_core::SettingsStore::in_memory();
    store.set_click_to_draw(Some(false));
    let json = serde_json::to_value(store.view()).unwrap();
    assert_eq!(json["clickToDraw"], serde_json::json!(false));
}

#[test]
fn changing_one_preference_leaves_the_others_alone() {
    // This is the whole reason every argument of `update_settings` is optional:
    // the screen sends one control's new value, and a filled-in voice must not
    // be cleared by a pace change on the way past.
    let state = state();
    state.update_settings(Some(true), Some("Meijia"), None, None);
    let view = state.update_settings(None, None, Some(hanzi_core::Pace::Fast), None);

    assert_eq!(view.click_to_draw(), Some(true), "click-to-draw was untouched");
    assert_eq!(view.voice(), Some("Meijia"), "the voice was untouched");
    assert_eq!(view.pace(), hanzi_core::Pace::Fast);
    assert_eq!(view.board_size(), hanzi_core::BoardSize::Normal);

    // And clearing the device-dependent one is its own call, because `None`
    // above already means "leave it alone".
    let view = state.clear_click_to_draw();
    assert_eq!(view.click_to_draw(), None);
    assert_eq!(view.voice(), Some("Meijia"), "only click-to-draw was cleared");
}

#[test]
fn every_preference_survives_a_restart_through_the_state_layer() {
    let dir = data_dir("ipc-settings-all");
    {
        let state = AppState::load(Some(dir.clone())).unwrap();
        let view = state.update_settings(
            Some(false),
            Some("Meijia"),
            Some(hanzi_core::Pace::Slow),
            Some(hanzi_core::BoardSize::Large),
        );
        assert!(view.warning.is_none(), "{:?}", view.warning);
    }

    let state = AppState::load(Some(dir.clone())).unwrap();
    let view = state.lock_settings().view();
    assert_eq!(view.click_to_draw(), Some(false));
    assert_eq!(view.voice(), Some("Meijia"));
    assert_eq!(view.pace(), hanzi_core::Pace::Slow);
    assert_eq!(view.board_size(), hanzi_core::BoardSize::Large);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_voice_list_says_which_voice_a_choice_resolved_to() {
    // A preference naming a voice this machine does not have falls back rather
    // than failing, and the screen can only say so honestly if `active` reports
    // what is really in use rather than what was stored.
    let state = state();
    state.update_settings(None, Some("Definitely Not An Installed Voice"), None, None);

    let voices = state.voices();
    let json = serde_json::to_value(&voices).unwrap();
    expect_keys(&json, &["available", "active"]);
    for option in json["available"].as_array().unwrap() {
        // `network` travels with each voice so the settings screen can say which
        // ones would need a connection — the app is meant to work offline, and
        // Android offers both kinds for the same locale.
        expect_keys(option, &["name", "locale", "network"]);
    }

    // Whatever this machine has, the stored preference is not a voice it can
    // speak with, so the two must not be reported as the same thing.
    assert_ne!(voices.active.as_deref(), Some("Definitely Not An Installed Voice"));
    // And the interface reads `active === null` to disable pronunciation, so it
    // must be `null` exactly when nothing Chinese is installed.
    assert_eq!(voices.active.is_none(), voices.available.is_empty());
}

#[test]
fn the_default_state_keeps_settings_in_memory() {
    let state = state();
    let mut settings = state.lock_settings();
    assert_eq!(settings.view().click_to_draw(), None);
    assert!(settings.load_error.is_none());
    assert!(settings.save().is_none());
}

#[test]
fn a_setting_choice_survives_a_restart_through_the_state_layer() {
    let dir = data_dir("ipc-settings-persist");

    {
        let state = AppState::load(Some(dir.clone())).unwrap();
        assert_eq!(
            state.lock_settings().view().click_to_draw(),
            None,
            "a fresh install has chosen nothing"
        );
        let view = state.set_click_to_draw(Some(true));
        assert_eq!(view.click_to_draw(), Some(true));
        assert!(view.warning.is_none(), "{:?}", view.warning);
    }

    // A second session, as if the app had been started again.
    let state = AppState::load(Some(dir.clone())).unwrap();
    let settings = state.lock_settings();
    assert!(settings.load_error.is_none(), "{:?}", settings.load_error);
    assert_eq!(settings.view().click_to_draw(), Some(true));

    // And it lives in the same database as everything else, not a file of its own.
    assert!(dir.join("hanzi.db").exists());
    assert!(!dir.join("settings.json").exists());

    drop(settings);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_default_state_keeps_progress_and_the_cursor_in_memory() {
    let state = state();
    let mut progress = state.lock_progress();
    assert!(progress.store.cards().is_empty());
    assert!(progress.load_error.is_none());
    assert!(progress.view().warning.is_none());
    assert!(progress.save().is_none());

    let mut cursor = state.lock_cursor();
    assert_eq!(cursor.view().index, 0);
    assert!(cursor.load_error.is_none());
    assert!(cursor.save().is_none());
}

#[test]
fn a_due_course_character_reaches_the_review_queue() {
    // The whole chain the unit tests do not cover: the real course, a saved
    // schedule on disk, the queue builder, and the state layer.
    let dir = data_dir("ipc-queue-course");
    let state = state();
    let ch = state.lessons()[1].characters[3];
    write_schedule(&dir, &[ch]);

    let loaded = AppState::load(Some(dir.clone())).unwrap();
    let queue = loaded.review_queue();
    assert_eq!(queue.due_count, 1);
    assert_eq!(queue.items.len(), 1);
    let item = &queue.items[0];
    assert_eq!(item.ch, ch);
    assert_eq!(item.text, ch.to_string());
    assert_eq!(item.source, ReviewSource::Course);
    assert_eq!(item.entry_id, None);
    assert!(queue.warning.is_none());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_due_character_of_a_saved_word_is_reviewed_as_the_word() {
    let dir = data_dir("ipc-queue-word");
    // A real two-character word, and both of its characters due.
    let characters: Vec<char> = "学习".chars().collect();
    let vocabulary = serde_json::json!({
        "version": 1,
        "nextId": 2,
        "groups": ["Lesson 3"],
        "entries": [{
            "id": 1,
            "text": "学习",
            "pinyin": "xuéxí",
            "meaning": "to study",
            "group": "Lesson 3",
            "addedAt": "2026-01-01T00:00:00Z",
            "attempts": 0,
            "bestScore": null,
            "lastPractised": null,
        }],
    });
    std::fs::write(dir.join("vocabulary.json"), vocabulary.to_string()).unwrap();
    write_schedule(&dir, &characters);

    let loaded = AppState::load(Some(dir.clone())).unwrap();
    let queue = loaded.review_queue();
    assert_eq!(queue.due_count, 1, "one word, not two characters");
    let item = &queue.items[0];
    assert_eq!(item.text, "学习");
    assert_eq!(item.source, ReviewSource::Vocabulary);
    assert_eq!(item.entry_id, Some(1));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_review_session_is_capped_but_the_total_is_honest() {
    let dir = data_dir("ipc-queue-cap");
    let state = state();
    let characters: Vec<char> = state
        .lessons()
        .iter()
        .flat_map(|lesson| lesson.characters.clone())
        .take(REVIEW_LIMIT + 5)
        .collect();
    assert_eq!(characters.len(), REVIEW_LIMIT + 5);
    write_schedule(&dir, &characters);

    let loaded = AppState::load(Some(dir.clone())).unwrap();
    let queue = loaded.review_queue();
    assert_eq!(queue.items.len(), REVIEW_LIMIT, "a session is capped");
    assert_eq!(
        queue.due_count,
        REVIEW_LIMIT + 5,
        "but the total says how many are left"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn practice_survives_a_restart_through_the_state_layer() {
    // The acceptance criterion, end to end: practise, relaunch, and the attempt
    // is still there — the character is no longer new.
    let dir = data_dir("ipc-progress-persist");

    let ch = {
        let state = AppState::load(Some(dir.clone())).unwrap();
        let ch = state.lessons()[0].characters[0];

        let mut progress = state.lock_progress();
        let card = progress
            .store
            .record(ch, 83.0)
            .expect("recording should succeed");
        assert_eq!(card.attempts, 1);
        assert!(progress.save().is_none(), "saving fresh progress should work");
        ch
    };

    // `hanzi.db`, not `progress.json`: the schedule is rows now, and the attempt
    // it just recorded is a row in the log.
    let path = dir.join("hanzi.db");
    assert!(path.exists(), "expected a file at {}", path.display());
    assert!(
        !dir.join("progress.json").exists(),
        "the schedule should no longer be a JSON document of its own"
    );

    // A second session, as if the app had been restarted.
    let state = AppState::load(Some(dir.clone())).unwrap();
    let progress = state.lock_progress();
    assert!(progress.load_error.is_none(), "{:?}", progress.load_error);
    assert!(!progress.store.is_new(ch), "a practised character is not new");
    let card = progress.store.card(ch).expect("the card should be there");
    assert_eq!(card.attempts, 1);
    assert_eq!(card.best_score, Some(83.0));
    assert_eq!(card.history.len(), 1, "the attempt is in the history");
    assert!(card.due.as_str() > "2026-01-01T00:00:00Z", "and it was scheduled");

    drop(progress);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_corrupt_schedule_is_reported_rather_than_silently_replaced() {
    let dir = data_dir("ipc-progress-corrupt");
    let path = dir.join("progress.json");
    std::fs::write(&path, "{ not json").unwrap();

    let state = AppState::load(Some(dir.clone())).unwrap();
    let mut progress = state.lock_progress();

    let warning = progress
        .view()
        .warning
        .clone()
        .expect("a corrupt schedule must produce a warning");
    assert!(warning.contains("could not be read"), "{warning}");
    // Refusing to save is the point: the file is left exactly as it was.
    assert!(progress.save().is_some(), "saving must be refused");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");

    drop(progress);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_corrupt_cursor_is_reported_without_losing_the_schedule() {
    // The reason the two are separate files: one bad file must not take the
    // other down.
    let dir = data_dir("ipc-cursor-corrupt");
    let ch = state().lessons()[0].characters[0];
    write_schedule(&dir, &[ch]);
    std::fs::write(dir.join("course-cursor.json"), "{ not json").unwrap();

    let state = AppState::load(Some(dir.clone())).unwrap();

    // The schedule loaded, history and all.
    let progress = state.lock_progress();
    assert!(progress.load_error.is_none(), "{:?}", progress.load_error);
    assert_eq!(progress.store.card(ch).unwrap().attempts, 1);
    assert!(!progress.store.is_new(ch));
    drop(progress);

    // The cursor reports its own problem and refuses to overwrite the file.
    let mut cursor = state.lock_cursor();
    let warning = cursor.view().warning.clone().expect("a warning");
    assert!(warning.contains("could not be read"), "{warning}");
    assert!(cursor.save().is_some(), "saving must be refused");
    assert_eq!(
        std::fs::read_to_string(dir.join("course-cursor.json")).unwrap(),
        "{ not json"
    );

    drop(cursor);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_course_cursor_persists_and_is_clamped_to_the_course() {
    let dir = data_dir("ipc-cursor-persist");
    let last = {
        let state = AppState::load(Some(dir.clone())).unwrap();
        let last = state.course_len() - 1;
        assert_eq!(
            state.lock_cursor().view().index,
            0,
            "a fresh install starts at the beginning"
        );

        // A position far past the end is clamped rather than trusted.
        let view = state.set_cursor(usize::MAX);
        assert_eq!(view.index, last);
        assert!(view.warning.is_none());
        last
    };

    let state = AppState::load(Some(dir.clone())).unwrap();
    assert_eq!(
        state.lock_cursor().view().index,
        last,
        "the place survived the restart"
    );
    // Moving backwards works too, and the file follows.
    assert_eq!(state.set_cursor(3).index, 3);
    assert_eq!(
        AppState::load(Some(dir.clone()))
            .unwrap()
            .lock_cursor()
            .view()
            .index,
        3
    );

    std::fs::remove_dir_all(&dir).ok();
}

// ---- what the app is, and what it ships under ------------------------------

#[test]
fn the_about_screen_gets_camel_case_fields() {
    let json = serde_json::to_value(hanzi_tutor_lib::licences::APP).unwrap();
    expect_keys(
        &json,
        &[
            "name",
            "version",
            "identifier",
            "licence",
            "copyright",
            "repository",
        ],
    );
    // The identifier is the one macOS files study data under, so a mismatch
    // would quietly point the About screen at the wrong bundle.
    assert_eq!(json["identifier"], serde_json::json!("com.hanzitutor.app"));
    assert_eq!(
        json["version"],
        serde_json::json!(env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn the_licence_notices_reach_the_interface_with_their_full_text() {
    let notices = hanzi_tutor_lib::licences::notices();
    let json = serde_json::to_value(notices).unwrap();
    expect_keys(
        &json[0],
        &[
            "id",
            "title",
            "licence",
            "source",
            "covers",
            "file",
            "bundlePath",
            "text",
        ],
    );

    // The acceptance criterion for M5: the notices are reachable from the UI,
    // and reachable means the whole text, not a stub or an empty string.
    for (notice, value) in notices.iter().zip(json.as_array().expect("an array")) {
        let text = value["text"].as_str().expect("the notice text is a string");
        assert!(
            text.len() > 500,
            "the {} notice is only {} bytes — the Licences screen would show a stub",
            notice.id,
            text.len()
        );
        assert!(
            !notice.bundle_path.is_empty() && !notice.source.is_empty(),
            "the {} notice must say where it came from and where the bundle copy is",
            notice.id
        );
    }

    let ids: BTreeSet<&str> = notices.iter().map(|notice| notice.id).collect();
    assert_eq!(
        ids.len(),
        notices.len(),
        "the screen keys the list by id, so a duplicate would silently drop a notice"
    );
    assert!(
        notices.len() >= 8,
        "expected the app licence, the provenance record and every data notice, got {}",
        notices.len()
    );
}

// ---- tone practice ---------------------------------------------------------

/// Every character of a target has a reading and a tone, single or word.
///
/// This is what decides whether the interface offers the microphone at all, so a
/// wrong answer here is either a dead control or a missing one.
#[test]
fn a_single_character_targets_the_tone_of_its_reading() {
    let state = state();

    // 妈 mā, 麻 má, 马 mǎ, 骂 mà — one of each tone, all in the dataset.
    for (ch, tone) in [("妈", 1u8), ("麻", 2), ("马", 3), ("骂", 4)] {
        let target = state
            .tone_target(ch)
            .unwrap_or_else(|| panic!("{ch} should be scorable"));
        assert_eq!(target.syllables.len(), 1, "{ch} is one syllable");
        assert_eq!(target.spoken(), vec![tone], "{ch}");
        // One syllable cannot undergo sandhi, so the two must agree.
        assert_eq!(target.syllables[0].citation, tone, "{ch}");
        assert!(!target.sandhi_applied, "{ch}");
        assert_eq!(target.syllables[0].ch.to_string(), ch);
    }
}

/// A word is scored as a word, with the tones it is actually spoken with.
///
/// This is the whole reason the target is built from the text rather than from
/// one character: 你好 is `3 + 3` in a dictionary and `2 + 3` out loud.
#[test]
fn a_word_target_applies_tone_sandhi() {
    let state = state();

    let nihao = state.tone_target("你好").expect("你好 is a word");
    assert_eq!(nihao.syllables.len(), 2);
    assert_eq!(
        nihao.syllables.iter().map(|s| s.citation).collect::<Vec<_>>(),
        vec![3, 3],
        "the dictionary tones"
    );
    assert_eq!(nihao.spoken(), vec![2, 3], "the tones actually spoken");
    assert!(nihao.sandhi_applied);
    assert_eq!(nihao.syllables[0].ch, '你');
    assert_eq!(nihao.syllables[1].ch, '好');
    assert!(nihao.detail.contains("2 + 3"), "{}", nihao.detail);

    // A word with nothing to change says so.
    let xuexi = state.tone_target("学习").expect("学习 is a word");
    assert_eq!(xuexi.spoken(), vec![2, 2]);
    assert!(!xuexi.sandhi_applied);

    // 一 changes with what follows it, which needs the characters, not just the
    // tones — so this is also a check that the characters reach the rule.
    assert_eq!(
        state.tone_target("一个").map(|t| t.spoken()),
        Some(vec![2, 4]),
        "一 before a fourth tone"
    );
    assert_eq!(
        state.tone_target("一起").map(|t| t.spoken()),
        Some(vec![4, 3]),
        "一 before a third tone"
    );
    // 不 likewise.
    assert_eq!(state.tone_target("不是").map(|t| t.spoken()), Some(vec![2, 4]));
}

/// The things tone practice must decline rather than guess at.
#[test]
fn a_target_is_refused_when_it_could_not_be_scored() {
    let state = state();

    // 的 is the neutral tone, and it is no longer refused. This assertion is the
    // inversion of the one that used to be here: a neutral tone is judged on
    // being level, which is the part of it a single syllable can show, and 的 is
    // the most common character in the language — the one character whose tone
    // the app would not look at.
    let neutral = state.tone_target("的").expect("a neutral tone is a target");
    assert_eq!(neutral.spoken(), vec![5]);
    assert_eq!(neutral.scorable(), 1);

    // Not in the dataset at all.
    assert!(state.tone_target("€").is_none());
    assert!(state.tone_target("").is_none());

    // Longer than a word. A sentence's syllable boundaries cannot be found from
    // energy alone, so it is refused rather than divided and half-scored.
    assert!(state.tone_target("我很好你好吗").is_none());

    // A word the dataset does not have as a word still works from its
    // characters, which is what makes a user's own vocabulary usable.
    let composed = state.tone_target("妈麻");
    assert!(composed.is_some(), "character readings should compose");
    assert_eq!(composed.unwrap().spoken(), vec![1, 2]);
}

/// Every offered target must be one the analyser can judge, or the button leads
/// to "I could not judge that" every time.
#[test]
fn every_offered_target_has_something_to_score() {
    let state = state();
    // 的 and 妈妈 are here for the neutral tone, which is the case this test
    // exists to protect: it used to be offered nothing at all.
    for text in [
        "妈", "你好", "学习", "一个", "一起", "不是", "妈妈", "朋友们", "的", "好了",
    ] {
        let target = state
            .tone_target(text)
            .unwrap_or_else(|| panic!("{text} should be offered"));
        assert!(
            target.scorable() > 0,
            "{text} was offered with nothing scoreable"
        );
        for syllable in &target.syllables {
            assert!(
                (1..=5).contains(&syllable.spoken),
                "{text}: tone {} is not a tone",
                syllable.spoken
            );
            assert!(!syllable.reading.is_empty(), "{text}: no reading");
        }
    }
}

/// The shape of a target, as the TypeScript client reads it.
#[test]
fn a_tone_target_serialises_with_camel_case_fields() {
    let state = state();
    let target = state.tone_target("你好").expect("你好 is a word");
    let json = serde_json::to_value(&target).unwrap();
    expect_keys(&json, &["syllables", "sandhiApplied", "detail"]);
    expect_keys(
        &json["syllables"][0],
        &["ch", "reading", "citation", "spoken"],
    );
    assert_eq!(json["sandhiApplied"], true);
    assert_eq!(json["syllables"][0]["ch"], "你");
}

/// The shape of a scored word, as the TypeScript client reads it.
///
/// The analyser is handed silence, so this asserts the *message shape*: the
/// concordant cases are covered by `hanzi-core`'s own tests, which can synthesise
/// a spoken word. What is locked here is that a failed judgement still carries
/// every field, and still carries one entry per syllable — the interface renders
/// `uncertain` from the same object it renders a score from.
#[test]
fn a_tone_result_carries_one_entry_per_syllable() {
    let state = state();
    let target = state.tone_target("你好").expect("你好 is a word");
    let quiet = vec![0.0f32; 16_000];
    let result = state.score_tones(
        &hanzi_tutor_lib::Recording {
            samples: quiet,
            sample_rate: 16_000,
            device: "test".into(),
            truncated: false,
        },
        &target,
    );

    let json = serde_json::to_value(&result).unwrap();
    expect_keys(
        &json,
        &[
            "syllables",
            "verdict",
            "score",
            "grade",
            "detail",
            "sandhiApplied",
            "boundariesMs",
            "voicedMs",
            "spanMs",
            "medianHz",
            "heard",
            "heardError",
        ],
    );

    // The two recognition keys are part of the contract even when there is
    // nothing to put in them: the interface reads `heard` on every attempt to
    // decide whether to draw the second half of the panel, and a *missing* key
    // would read as `undefined` and a `null` as "no model installed" — two
    // different things that would look identical on screen. Both are asserted
    // here, with no model installed, which is the state every fresh install is in.
    assert!(
        json["heard"].is_null(),
        "no model is installed in a test, so `heard` must be null, not missing: {}",
        json["heard"]
    );
    assert!(
        json["heardError"].is_null(),
        "nothing failed, so `heardError` must be null: {}",
        json["heardError"]
    );
    // One entry per syllable of the word, each with its character and reading, so
    // the interface can label a chart without holding pinyin rules of its own.
    assert_eq!(json["syllables"].as_array().unwrap().len(), 2);
    expect_keys(
        &json["syllables"][0],
        &[
            "position",
            "ch",
            "reading",
            "citation",
            "spoken",
            "attempt",
        ],
    );
    assert_eq!(json["syllables"][0]["ch"], "你");
    assert_eq!(json["syllables"][0]["spoken"], 2, "after sandhi");
    assert_eq!(json["syllables"][0]["citation"], 3);

    // Nothing was heard, so it says so rather than scoring.
    assert_eq!(json["verdict"], "uncertain");
    assert_eq!(json["grade"], "poor");
    assert_eq!(json["score"], 0.0);
    assert!(json["detail"].as_str().unwrap().contains("could not hear"));
    // The sandhi sentence is appended, because a learner seeing "tone 2" for 你
    // needs to know why it is not the tone their dictionary prints.
    assert!(
        json["detail"].as_str().unwrap().contains("2 + 3"),
        "{}",
        json["detail"]
    );

    // The reference contour is always present, one per syllable, so the interface
    // can draw the expected shape whether or not there is a learner's line.
    for syllable in json["syllables"].as_array().unwrap() {
        assert_eq!(syllable["attempt"]["reference"].as_array().unwrap().len(), 12);
        assert!(syllable["attempt"]["contour"].as_array().unwrap().is_empty());
    }
}

/// A single character's result is the same object with one syllable.
///
/// The single-character path and the word path share the analyser, so this is a
/// check that the shape did not diverge rather than a check of the scoring.
#[test]
fn a_single_character_result_has_one_syllable_and_no_boundaries() {
    let state = state();
    let target = state.tone_target("妈").expect("妈 is scorable");
    let quiet = vec![0.0f32; 16_000];
    let result = state.score_tones(
        &hanzi_tutor_lib::Recording {
            samples: quiet,
            sample_rate: 16_000,
            device: "test".into(),
            truncated: false,
        },
        &target,
    );
    assert_eq!(result.syllables.len(), 1);
    assert!(result.boundaries_ms.is_empty(), "one syllable has no boundary");
    assert!(!result.sandhi_applied);
    assert!(result.detail.contains("could not hear"), "{}", result.detail);
}

/// A microphone report is always a complete object, device or no device.
#[test]
fn a_microphone_status_is_always_answerable() {
    let status = hanzi_tutor_lib::Recorder::default().status();
    let json = serde_json::to_value(&status).unwrap();
    expect_keys(&json, &["available", "device", "sampleRate", "detail"]);
    assert!(!json["detail"].as_str().unwrap().is_empty());
    if !json["available"].as_bool().unwrap() {
        assert!(json["device"].is_null());
        assert_eq!(json["sampleRate"], 0);
    }
}

/// Scoring a recording keeps the analyser's verdict and adds the cap warning.
#[test]
fn a_truncated_recording_says_so_in_the_detail() {
    let state = state();
    let target = state.tone_target("妈").expect("妈 is scorable");
    let recording = hanzi_tutor_lib::Recording {
        samples: vec![0.0; 16_000],
        sample_rate: 16_000,
        device: "test".into(),
        truncated: true,
    };
    let result = state.score_tones(&recording, &target);
    assert!(
        result.detail.contains("10-second limit"),
        "a capped recording must say it was capped: {}",
        result.detail
    );

    let whole = hanzi_tutor_lib::Recording {
        truncated: false,
        ..recording
    };
    assert!(
        !state.score_tones(&whole, &target).detail.contains("limit"),
        "an uncapped recording must not claim it was capped"
    );
}
