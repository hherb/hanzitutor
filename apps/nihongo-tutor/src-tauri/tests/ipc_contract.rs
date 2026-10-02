//! The JSON contract the interface reads, locked.
//!
//! The webview is not type-checked against Rust, so a renamed field is a silent
//! break: the interface reads `undefined` and shows nothing, and nothing fails
//! until someone opens a window. This file asserts the exact keys and value
//! shapes of every type that crosses the boundary, so a rename fails here first.
//!
//! It drives `AppState` directly rather than through Tauri, which is the point of
//! keeping the commands thin.

use nihongo_tutor_lib::AppState;
use nihongo_core::{GradeOptions, Point};
use serde_json::{json, Value};

fn state() -> AppState {
    AppState::load()
}

fn keys(value: &Value) -> Vec<String> {
    let mut out: Vec<String> = value
        .as_object()
        .expect("an object")
        .keys()
        .cloned()
        .collect();
    out.sort();
    out
}

#[test]
fn dataset_stats_crosses_as_camel_case() {
    let stats = serde_json::to_value(state().stats()).expect("serialises");
    assert_eq!(
        keys(&stats),
        vec!["hiragana", "kana", "katakana", "lessons", "strokes"]
    );
    assert_eq!(stats["kana"], 177);
    assert_eq!(stats["hiragana"], 86);
    assert_eq!(stats["katakana"], 91);
    assert_eq!(stats["strokes"], 516);
}

#[test]
fn a_kana_crosses_with_the_geometry_and_the_readings() {
    let view = serde_json::to_value(state().kana('あ').expect("あ")).expect("serialises");
    assert_eq!(
        keys(&view),
        vec![
            "ch",
            "confusions",
            "hepburn",
            "medians",
            "outlines",
            "practisable",
            "romaji",
            "script",
            "silent",
            "strokeCount",
        ]
    );
    assert_eq!(view["ch"], "あ");
    assert_eq!(view["strokeCount"], 3);
    assert_eq!(view["script"], "hiragana");
    assert_eq!(view["practisable"], true);
    assert_eq!(view["silent"], false);
    assert_eq!(view["hepburn"], "a");
    assert_eq!(view["romaji"], json!(["a"]));
    assert_eq!(view["outlines"].as_array().expect("array").len(), 3);

    // The centre-lines are objects with x and y, not pairs, because `Point`
    // serialises as a struct.
    let first = &view["medians"][0][0];
    assert!(first.get("x").is_some(), "a median point has an x");
    assert!(first.get("y").is_some(), "a median point has a y");
}

#[test]
fn a_silent_mark_carries_no_readings() {
    let view = serde_json::to_value(state().kana('ー').expect("ー")).expect("serialises");
    assert_eq!(view["silent"], true);
    assert_eq!(view["romaji"], json!([]));
    assert_eq!(view["hepburn"], "");
}

#[test]
fn confusions_cross_as_a_list_of_kana_and_their_tell() {
    let view = serde_json::to_value(state().kana('シ').expect("シ")).expect("serialises");
    let confusions = view["confusions"].as_array().expect("array");
    assert!(!confusions.is_empty());
    assert_eq!(keys(&confusions[0]), vec!["ch", "tell"]);
    assert!(confusions
        .iter()
        .any(|c| c["ch"] == "ツ" && c["tell"].as_str().is_some_and(|t| !t.is_empty())));
}

#[test]
fn a_lesson_crosses_with_its_kana_and_a_stable_key() {
    let lessons = state().lessons(nihongo_core::Script::Hiragana);
    let first = serde_json::to_value(&lessons[0]).expect("serialises");
    assert_eq!(keys(&first), vec!["count", "kana", "key", "title", "voiced"]);
    assert_eq!(first["key"], "hiragana-a");
    assert_eq!(first["kana"], json!(["あ", "い", "う", "え", "お"]));
    assert_eq!(first["count"], 5);
    assert_eq!(first["voiced"], false);

    let voiced = lessons
        .iter()
        .find(|l| l.voiced)
        .expect("there are voiced lessons");
    assert_eq!(voiced.key, "hiragana-ga");
}

#[test]
fn a_yoon_crosses_with_both_romanisations() {
    let all = state().yoon(nihongo_core::Script::Hiragana);
    let kyu = all.iter().find(|y| y.hepburn == "kyu").expect("きゅ");
    let value = serde_json::to_value(kyu).expect("serialises");
    assert_eq!(keys(&value), vec!["display", "hepburn", "kana", "key", "kunrei"]);
    assert_eq!(value["display"], "きゅ");
    assert_eq!(value["kana"], json!(["き", "ゅ"]));
    assert_eq!(value["key"], "hiragana-kyu");
}

#[test]
fn a_grade_report_crosses_with_the_four_headline_scores() {
    let state = state();
    let view = state.kana('ー').expect("ー");
    let attempt = vec![view.medians[0].clone()];
    let report = state
        .grade('ー', &attempt, &GradeOptions::default())
        .expect("grades");
    let value = serde_json::to_value(report).expect("serialises");

    // The scores the interface turns into the four verdict colours.
    assert!(value.get("shapeScore").is_some());
    assert!(value.get("positionScore").is_some());
    assert!(value.get("inkScore").is_some());
    assert!(value.get("orderScore").is_some());
    assert!(value.get("overall").is_some());
    assert!(value.get("legible").is_some());
    assert!(value.get("strokes").is_some());
    assert!(value.get("assignment").is_some());
    assert_eq!(value["expectedStrokes"], 1);
}

#[test]
fn grading_reads_the_attempt_the_interface_sends() {
    // The interface posts `{ ch, strokes, options? }`, where a stroke is a list
    // of {x, y}. This is that round trip: build the JSON, read it back, grade.
    let payload = json!({
        "ch": "ー",
        "strokes": [[{"x": 120.0, "y": 500.0}, {"x": 900.0, "y": 505.0}]]
    });
    let ch: char = payload["ch"].as_str().expect("a string").chars().next().expect("one char");
    let strokes: Vec<Vec<Point>> = serde_json::from_value(payload["strokes"].clone())
        .expect("the strokes deserialise");

    let report = state().grade(ch, &strokes, &GradeOptions::default()).expect("grades");
    assert!(report.legible, "a level stroke across the middle is ー: {:.0}", report.overall);
}

#[test]
fn an_error_crosses_as_a_string_not_as_a_panic() {
    // Every fallible command returns Result<_, String>, so the interface can show
    // the message. This is the shape that makes that true.
    let err = state().grade('一', &[], &GradeOptions::default()).unwrap_err();
    let as_json = serde_json::to_value(&err).expect("a String serialises");
    assert!(as_json.is_string());
    assert!(as_json.as_str().expect("a string").contains("not in the kana set"));
}

#[test]
fn app_info_names_the_app_the_bundle_config_names() {
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json"))
        .expect("tauri.conf.json parses");
    assert_eq!(config["identifier"], "com.hanzitutor.kana");
    assert_eq!(config["productName"], "Kana Tutor");
    assert_eq!(config["version"], env!("CARGO_PKG_VERSION"));
}
