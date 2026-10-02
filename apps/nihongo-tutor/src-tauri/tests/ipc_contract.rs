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
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn state() -> AppState {
    AppState::load()
}

/// A directory of our own under the system temporary directory, removed when the
/// test ends. The store's own unit tests have the same helper; an integration
/// test cannot reach it, and ten duplicated lines are cheaper than a public API
/// that exists only for tests.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "nihongo-tutor-contract-{}-{tag}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("a temporary directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
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
    // The interface posts `{ ch, strokes, options }` — a stroke is a list of
    // {x, y}, and `options` is `{ inkWidth }` and nothing else, deliberately:
    // the other three tunables stay at the values the tolerance study was fitted
    // against (see `src/lib/api.ts`). This is that round trip, options included,
    // because reading the options back through serde is what catches a field the
    // interface sends that Rust cannot fill in. Without it this app shipped a
    // Grade button whose every attempt came back "missing field `resampleK`" —
    // the options were never deserialised in a test, so nothing failed here.
    let payload = json!({
        "ch": "ー",
        "strokes": [[{"x": 120.0, "y": 500.0}, {"x": 900.0, "y": 505.0}]],
        "options": {"inkWidth": 36.0}
    });
    let ch: char = payload["ch"].as_str().expect("a string").chars().next().expect("one char");
    let strokes: Vec<Vec<Point>> = serde_json::from_value(payload["strokes"].clone())
        .expect("the strokes deserialise");
    let options: GradeOptions = serde_json::from_value(payload["options"].clone())
        .expect("the options the interface posts deserialise");
    assert_eq!(options.ink_width, 36.0);
    assert_eq!(options.resample_k, GradeOptions::default().resample_k);

    let report = state().grade(ch, &strokes, &options).expect("grades");
    assert!(report.legible, "a level stroke across the middle is ー: {:.0}", report.overall);
}

#[test]
fn the_options_the_interface_posts_survive_a_round_trip() {
    // Every field the interface *could* send, sent. A field renamed on one side
    // and not the other is then a failure here rather than a rejected command.
    let all = GradeOptions {
        resample_k: 16,
        min_stroke_len: 12.0,
        global_fit: true,
        ink_width: 36.0,
    };
    let posted = serde_json::to_value(&all).expect("serialises");
    assert_eq!(
        keys(&posted),
        vec!["globalFit", "inkWidth", "minStrokeLen", "resampleK"]
    );
    let read_back: GradeOptions = serde_json::from_value(posted).expect("deserialises");
    assert_eq!(read_back.resample_k, all.resample_k);
    assert_eq!(read_back.min_stroke_len, all.min_stroke_len);
    assert_eq!(read_back.global_fit, all.global_fit);
    assert_eq!(read_back.ink_width, all.ink_width);
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

/// The arguments `record_drill_answer` takes, as the webview posts them.
///
/// Spelled out rather than inferred, because this is the half of the contract
/// that fails loudly (a wrong key is a rejected command) and the half that has
/// already gone wrong once in this app — see invariant 14 in
/// `HANDOVER_NIHONGO.md`. There is deliberately no `correct` field: the
/// interface says what was asked and what was picked, and Rust decides.
#[derive(Deserialize)]
struct RecordDrillAnswer {
    pair: String,
    target: char,
    picked: char,
}

#[test]
fn a_drill_question_crosses_with_its_pair_its_prompt_and_its_options() {
    let question = state().next_drill_question().expect("a pair can be asked");
    let value = serde_json::to_value(&question).expect("serialises");
    assert_eq!(keys(&value), vec!["ch", "hepburn", "options", "pair", "tell"]);
    assert!(question.options.contains(&question.ch), "the answer is among the options");
    assert_eq!(question.options.len(), 2, "the pair, and nothing else");
    assert!(!question.hepburn.is_empty());
    assert!(!question.tell.is_empty(), "a miss has to be able to teach something");
}

#[test]
fn the_drill_answer_the_interface_posts_is_read_the_way_the_command_reads_it() {
    // `src/lib/api.ts` authors exactly this payload for exactly this command:
    //   invoke("record_drill_answer", { pair, target, picked })
    // One state for both answers: `AppState::load()` keeps its record in memory,
    // so two of them are two learners as far as this is concerned.
    let state = state();
    let payload = json!({ "pair": "シ|ツ", "target": "シ", "picked": "ツ" });
    let args: RecordDrillAnswer =
        serde_json::from_value(payload).expect("the payload the interface posts deserialises");

    let tally = state
        .record_drill_answer(&args.pair, args.target, args.picked)
        .expect("records");
    assert_eq!(tally.pair, "シ|ツ");
    assert_eq!(tally.asked, 1);
    assert_eq!(tally.wrong, 1, "シ asked for and ツ picked is a miss");
    assert_eq!(tally.weight, 3.0);

    let value = serde_json::to_value(&tally).expect("serialises");
    assert_eq!(
        keys(&value),
        vec!["asked", "correct", "pair", "weight", "wrong"]
    );

    // The same payload, answered correctly, is a hit on the same pair.
    let answered = json!({ "pair": "ツ|シ", "target": "ツ", "picked": "ツ" });
    let args: RecordDrillAnswer = serde_json::from_value(answered).expect("deserialises");
    let tally = state
        .record_drill_answer(&args.pair, args.target, args.picked)
        .expect("records");
    assert_eq!(tally.pair, "シ|ツ", "one pair, whichever way it is named");
    assert_eq!(tally.asked, 2);
    assert_eq!(tally.correct, 1);
    assert_eq!(tally.weight, 2.0, "1 + 2 for the miss − 1 for the hit");
}

#[test]
fn an_answer_about_a_key_that_is_not_a_pair_is_refused_with_a_message() {
    let state = state();
    let payload = json!({ "pair": "あ|い", "target": "あ", "picked": "い" });
    let args: RecordDrillAnswer = serde_json::from_value(payload).expect("deserialises");
    let err = state
        .record_drill_answer(&args.pair, args.target, args.picked)
        .unwrap_err();
    assert!(err.contains("not one of the confusion pairs"), "{err}");
    assert!(state.log().is_empty(), "nothing was written");
}

#[test]
fn what_the_drill_remembers_survives_a_restart() {
    let dir = TempDir::new("restart");
    let missed = {
        let state = AppState::load_at(dir.path());
        let question = state
            .next_drill_question_with_rolls(0.0, 0.0)
            .expect("a question");
        let wrong = question
            .options
            .iter()
            .copied()
            .find(|ch| *ch != question.ch)
            .expect("the other half of the pair");
        state
            .record_drill_answer(&question.pair, question.ch, wrong)
            .expect("records");
        question.pair
    };

    // The file is where the handover says it is, inside the app's own directory
    // and named for what it holds.
    assert!(
        dir.path().join("confusions.json").exists(),
        "the answer was written to the app's own file"
    );

    // A second AppState over the same directory is what a restart looks like.
    let reopened = AppState::load_at(dir.path());
    let log = reopened.log();
    assert_eq!(log.pairs_seen(), 1);
    assert_eq!(log.weight(&missed), 3.0, "the miss is still there");
    let (heaviest, weight) = log.weights().remove(0);
    assert_eq!(heaviest, missed);
    assert_eq!(weight, 3.0);
}

#[test]
fn a_first_run_writes_nothing_until_something_is_answered() {
    let dir = TempDir::new("first-run");
    let state = AppState::load_at(dir.path());
    assert!(state.log().is_empty());
    assert!(
        !dir.path().join("confusions.json").exists(),
        "an app that has been opened is not an app with a record"
    );
}

#[test]
fn app_info_names_the_app_the_bundle_config_names() {
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json"))
        .expect("tauri.conf.json parses");
    assert_eq!(config["identifier"], "com.hanzitutor.kana");
    assert_eq!(config["productName"], "Kana Tutor");
    assert_eq!(config["version"], env!("CARGO_PKG_VERSION"));
}
