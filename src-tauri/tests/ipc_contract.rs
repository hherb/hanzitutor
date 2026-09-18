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

use hanzi_core::{GradeOptions, Point};
use hanzi_tutor_lib::AppState;

fn state() -> AppState {
    AppState::load().expect("the embedded dataset should decode")
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
}

#[test]
fn stats_serialise_with_camel_case_fields() {
    let json = serde_json::to_value(state().stats()).unwrap();
    expect_keys(&json, &["characters", "teachable", "lessons", "lessonSize"]);
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
}

#[test]
fn a_json_null_option_is_the_same_as_omitting_it() {
    // The frontend sends `options: null` when it has no preference.
    let parsed: Option<GradeOptions> = serde_json::from_value(serde_json::Value::Null).unwrap();
    assert!(parsed.is_none());
}
