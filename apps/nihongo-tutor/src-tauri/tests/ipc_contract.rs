//! The JSON contract the interface reads, locked.
//!
//! The webview is not type-checked against Rust, so a renamed field is a silent
//! break: the interface reads `undefined` and shows nothing, and nothing fails
//! until someone opens a window. This file asserts the exact keys and value
//! shapes of every type that crosses the boundary, so a rename fails here first.
//!
//! It drives `AppState` directly rather than through Tauri, which is the point of
//! keeping the commands thin.

use hanzi_voice::Language;
use nihongo_core::{DrillKind, GradeOptions, Point};
use nihongo_tutor_lib::AppState;
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
        vec![
            "hiragana",
            "kana",
            "kanji",
            "kanjiLessons",
            "katakana",
            "kyoiku",
            "lessons",
            "passages",
            "radicals",
            "strokes",
            "words"
        ]
    );
    assert_eq!(stats["kana"], 177);
    assert_eq!(stats["hiragana"], 86);
    assert_eq!(stats["katakana"], 91);
    assert_eq!(stats["strokes"], 516);
    assert_eq!(stats["words"], 16_073, "the vocabulary the app embeds");
    assert_eq!(stats["passages"], 3);
    assert_eq!(stats["kanji"], 2_136, "the jōyō set the app embeds");
    assert_eq!(stats["kyoiku"], 1_026, "grades 1 to 6");
    assert_eq!(stats["radicals"], 214);
    assert_eq!(stats["kanjiLessons"], 216);
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

    // The command the interface actually calls, so the request half above is
    // checked against the path that runs rather than against a sibling of it.
    let graded = state()
        .grade_and_schedule(ch, &strokes, &options)
        .expect("grades");
    assert!(
        graded.report.legible,
        "a level stroke across the middle is ー: {:.0}",
        graded.report.overall
    );
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
    let err = state().grade('鳩', &[], &GradeOptions::default()).unwrap_err();
    let as_json = serde_json::to_value(&err).expect("a String serialises");
    assert!(as_json.is_string());
    assert!(as_json
        .as_str()
        .expect("a string")
        .contains("not a kana, a jōyō kanji, or a radical"));
}

/// The arguments `record_drill_answer` takes, as the webview posts them.
///
/// Spelled out rather than inferred, because this is the half of the contract
/// that fails loudly (a wrong key is a rejected command) and the half that has
/// already gone wrong once in this app — see invariant 14 in
/// `HANDOVER_NIHONGO.md`. There is deliberately no `correct` field: the
/// interface says what was asked and what was picked, and Rust decides.
///
/// A spelling rather than a kana, because a yōon question's two answers are two
/// characters each — きゃ against きや — and a `char` would reject the payload.
#[derive(Deserialize)]
struct RecordDrillAnswer {
    pair: String,
    target: String,
    picked: String,
}

#[test]
fn a_drill_question_crosses_with_its_pair_its_prompt_and_its_options() {
    let question = state()
        .next_drill_question(DrillKind::Confusion)
        .expect("a pair can be asked");
    let value = serde_json::to_value(&question).expect("serialises");
    assert_eq!(keys(&value), vec!["ch", "hepburn", "kind", "options", "pair", "tell"]);
    assert_eq!(question.kind, "confusion");
    assert!(question.options.contains(&question.ch), "the answer is among the options");
    assert_eq!(question.options.len(), 2, "the pair, and nothing else");
    assert!(!question.hepburn.is_empty());
    assert!(!question.tell.is_empty(), "a miss has to be able to teach something");
}

#[test]
fn a_yoon_question_crosses_with_two_character_answers() {
    // The one shape a `char`-typed payload could not carry: the answers are
    // spellings, and the two of them differ only in the size of the small kana.
    let question = state()
        .next_drill_question_with_rolls(DrillKind::YoonHiragana, 0.0, 0.0)
        .expect("the first yōon can be asked");
    let value = serde_json::to_value(&question).expect("serialises");
    assert_eq!(keys(&value), vec!["ch", "hepburn", "kind", "options", "pair", "tell"]);
    assert_eq!(question.kind, "yoon-hiragana");
    assert_eq!(question.pair, "きゃ|きや");
    assert_eq!(question.ch, "きゃ");
    assert_eq!(question.hepburn, "kya");
    assert_eq!(question.options, vec!["きゃ".to_string(), "きや".to_string()]);
    assert!(question.tell.contains("きゃ") && question.tell.contains("きや"));

    // And the payload the interface posts for it is read the way the command
    // reads it, which is the half invariant 14 exists for.
    let payload = json!({ "pair": "きゃ|きや", "target": "きゃ", "picked": "きや" });
    let args: RecordDrillAnswer =
        serde_json::from_value(payload).expect("the payload the interface posts deserialises");
    let tally = state()
        .record_drill_answer(&args.pair, &args.target, &args.picked)
        .expect("records");
    assert_eq!(tally.pair, "きゃ|きや");
    assert_eq!(tally.wrong, 1);
    assert_eq!(tally.weight, 3.0, "a yōon miss is weighted by the same rule");
}

/// The argument `kana_chart` takes, as the webview posts it.
///
/// The **request** half of the contract, spelled out for the same reason
/// `RecordDrillAnswer` is: a `#[tauri::command]`'s arguments are read out of the
/// payload by name, so a renamed key is a rejected command and the screen shows an
/// error rather than a chart. `AppState::chart` cannot catch that — it takes a
/// `Script`, not a payload — which is the whole of invariant 14.
#[derive(Deserialize)]
struct KanaChartArgs {
    script: String,
}

/// The argument `next_drill_question` takes, as the webview posts it.
///
/// `kind` is **optional on the wire**: a caller that predates the yōon drill asks
/// for nothing and must still get the classic pairs, which is what the command's
/// `unwrap_or(DrillKind::Confusion)` is for.
#[derive(Deserialize)]
struct NextDrillQuestionArgs {
    kind: Option<DrillKind>,
}

#[test]
fn the_chart_the_interface_asks_for_is_read_the_way_the_command_reads_it() {
    // `src/lib/api.ts` authors exactly this payload for exactly this command:
    //   invoke("kana_chart", { script })
    // The name is parsed by the same function the command parses it with, so this
    // pins the key *and* the two names the interface is allowed to send.
    let args: KanaChartArgs =
        serde_json::from_value(json!({ "script": "katakana" })).expect("the interface's payload");
    assert_eq!(
        nihongo_core::Script::from_name(&args.script),
        Some(nihongo_core::Script::Katakana),
        "the script the screen sent is the script the command resolves"
    );
    let args: KanaChartArgs =
        serde_json::from_value(json!({ "script": "hiragana" })).expect("the interface's payload");
    assert_eq!(
        nihongo_core::Script::from_name(&args.script),
        Some(nihongo_core::Script::Hiragana)
    );

    // And a name the app does not know is rejected by that parse rather than
    // silently answered with a different script.
    assert_eq!(nihongo_core::Script::from_name("kanji"), None);
}

#[test]
fn the_kind_the_interface_asks_for_is_read_the_way_the_command_reads_it() {
    // `src/lib/api.ts` authors exactly this payload for exactly this command:
    //   invoke("next_drill_question", { kind })
    // The strings are the frontend's `DrillKind` union, and each has to resolve to
    // the exercise whose pairs it names.
    for (name, kind) in [
        ("confusion", DrillKind::Confusion),
        ("yoon-hiragana", DrillKind::YoonHiragana),
        ("yoon-katakana", DrillKind::YoonKatakana),
        ("voicing-hiragana", DrillKind::VoicingHiragana),
        ("voicing-katakana", DrillKind::VoicingKatakana),
    ] {
        let args: NextDrillQuestionArgs = serde_json::from_value(json!({ "kind": name }))
            .unwrap_or_else(|e| panic!("the interface's payload for {name:?}: {e}"));
        assert_eq!(args.kind, Some(kind), "{name:?} is {kind:?}");
        // And the exercise it names is a pool the app can actually ask from, so a
        // name that deserialises but has no pairs is not a silent empty drill.
        assert!(!state().askable_pairs(kind).is_empty(), "{name:?} has no askable pairs");
    }

    // A caller that sends nothing, or an explicit null, gets the classic pairs
    // rather than a rejection — which is what "optional" has to mean here.
    let args: NextDrillQuestionArgs = serde_json::from_value(json!({})).expect("an omitted kind");
    assert_eq!(args.kind, None);
    let args: NextDrillQuestionArgs =
        serde_json::from_value(json!({ "kind": null })).expect("a null kind");
    assert_eq!(args.kind, None);
    assert_eq!(args.kind.unwrap_or(DrillKind::Confusion), DrillKind::Confusion);

    // And a name outside the union is a deserialisation failure rather than a
    // quietly different exercise.
    assert!(serde_json::from_value::<NextDrillQuestionArgs>(json!({ "kind": "kana" })).is_err());
}

#[test]
fn a_kana_chart_crosses_with_the_grid_and_the_groups_off_it() {
    let chart = state().chart(nihongo_core::Script::Katakana);    let value = serde_json::to_value(&chart).expect("serialises");
    assert_eq!(keys(&value), vec!["offGrid", "rows", "script", "vowels"]);
    assert_eq!(value["script"], "katakana");
    assert_eq!(value["vowels"], json!(["a", "i", "u", "e", "o"]));

    let rows = chart.rows.as_slice();
    assert_eq!(rows.len(), 16);
    assert_eq!(keys(&serde_json::to_value(&rows[0]).expect("serialises")), vec![
        "cells",
        "sound",
        "voiced"
    ]);
    // Five slots whatever the row holds, and a hole crosses as null rather than
    // being left out — a screen that receives four cells cannot know which column
    // the missing one was.
    for row in rows {
        assert_eq!(row.cells.len(), 5, "{} is not a five-column row", row.sound);
    }
    let ya = rows.iter().find(|row| row.sound == "ya").expect("the や row");
    assert_eq!(ya.cells, vec![Some("ヤ".into()), None, Some("ユ".into()), None, Some("ヨ".into())]);
    assert_eq!(
        serde_json::to_value(ya).expect("serialises")["cells"],
        json!(["ヤ", null, "ユ", null, "ヨ"])
    );

    let group = chart
        .off_grid
        .iter()
        .find(|group| group.key == "katakana-v")
        .expect("the v-series is off the grid");
    assert_eq!(keys(&serde_json::to_value(group).expect("serialises")), vec![
        "kana", "key", "title"
    ]);
    assert_eq!(group.kana, vec!["ヷ", "ヸ", "ヹ", "ヺ"]);
    assert!(group.title.contains("ヷ ヸ ヹ ヺ"), "{} names its kana", group.title);

    // The other script is a different chart, and says which one it is: a screen
    // switching the toggle can tell an answer about katakana from one about the
    // hiragana it just left.
    let hiragana = state().chart(nihongo_core::Script::Hiragana);
    assert_eq!(hiragana.script, "hiragana");
    assert!(hiragana.off_grid.iter().all(|group| !group.key.contains("katakana")));
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
        .record_drill_answer(&args.pair, &args.target, &args.picked)
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
        .record_drill_answer(&args.pair, &args.target, &args.picked)
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
        .record_drill_answer(&args.pair, &args.target, &args.picked)
        .unwrap_err();
    assert!(err.contains("not one of the pairs this drill asks about"), "{err}");
    assert!(state.log().is_empty(), "nothing was written");
}

#[test]
fn what_the_drill_remembers_survives_a_restart() {
    let dir = TempDir::new("restart");
    let missed = {
        let state = AppState::load_at(dir.path());
        let question = state
            .next_drill_question_with_rolls(DrillKind::Confusion, 0.0, 0.0)
            .expect("a question");
        let wrong = question
            .options
            .iter()
            .find(|option| **option != question.ch)
            .expect("the other half of the pair")
            .clone();
        state
            .record_drill_answer(&question.pair, &question.ch, &wrong)
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

// ---------------------------------------------------------------------------
// The vocabulary and the passages.
//
// These are the request halves as much as the response halves. Invariant 14 of
// HANDOVER_NIHONGO.md exists because this app shipped a Grade button that could
// not grade: the interface posted `{ inkWidth }`, the command deserialised a
// struct with more required fields, and every attempt came back "missing field
// `resampleK`" while 110 tests passed. So every command that takes arguments is
// driven here with the payload `api.ts` actually builds — and every response is
// read with the keys `types.ts` declares.

#[test]
fn the_ladder_crosses_with_our_names_and_each_bands_size() {
    let bands = state().word_bands();
    assert_eq!(bands.len(), 7);

    let value = serde_json::to_value(&bands).expect("serialises");
    let first = &value[0];
    // camelCase like every other payload, and the three keys the screen reads.
    for key in ["band", "name", "words"] {
        assert!(first.get(key).is_some(), "band is missing {key}: {first}");
    }
    assert_eq!(first["band"], 1);
    assert_eq!(first["name"], "kyōiku 1");
    assert!(first["words"].as_u64().expect("a count") > 0);
    assert_eq!(
        value.as_array().expect("an array").len(),
        7,
        "six grades and the remainder"
    );
}

#[test]
fn a_word_page_crosses_with_the_text_a_card_draws() {
    let page = state().words_in_band(1, 2, 5);
    let value = serde_json::to_value(&page).expect("serialises");
    for key in ["band", "total", "offset", "words"] {
        assert!(value.get(key).is_some(), "the page is missing {key}: {value}");
    }
    assert_eq!(value["band"], 1);
    assert_eq!(value["offset"], 2);

    let words = value["words"].as_array().expect("an array");
    assert_eq!(words.len(), 5);
    let word = &words[0];
    for key in ["text", "reading", "meaning", "band", "bandName", "nf", "furigana"] {
        assert!(word.get(key).is_some(), "a word is missing {key}: {word}");
    }
    // The furigana is the segment shape the card renders: ruby, and rt when the
    // segment has a kanji in it.
    let with_ruby = words
        .iter()
        .find(|w| !w["furigana"].as_array().expect("an array").is_empty())
        .expect("a band-1 word with furigana");
    let segment = &with_ruby["furigana"][0];
    assert!(segment.get("ruby").is_some(), "{segment}");
    assert!(segment.get("rt").is_some(), "rt is present even when null: {segment}");
}

#[test]
fn a_word_crosses_with_its_own_reading_and_never_a_composed_one() {
    let word = state().word("大人", "おとな").expect("大人 is in the vocabulary");
    let value = serde_json::to_value(&word).expect("serialises");
    assert_eq!(value["text"], "大人");
    assert_eq!(
        value["reading"], "おとな",
        "the word's own reading: だいじん is what composing it from the characters \
         would give"
    );
    assert_eq!(value["furigana"][0]["ruby"], "大人");
    assert_eq!(value["furigana"][0]["rt"], "おとな");

    // A word that is not in the vocabulary is a message rather than a panic, and
    // the message is what the screen shows.
    assert!(state().word("大人", "だいじん").is_err());
}

/// The request half, driven exactly as `api.ts` builds it: `{ text, reading,
/// typed }`, with the answer read back by the keys `types.ts` declares.
#[test]
fn the_word_the_interface_posts_is_checked_the_way_the_command_reads_it() {
    let payload = json!({
        "text": "学生",
        "reading": "がくせい",
        "typed": "gakusei",
    });
    let text = payload["text"].as_str().expect("a string");
    let reading = payload["reading"].as_str().expect("a string");
    let typed = payload["typed"].as_str().expect("a string");

    let check = state().check_word(text, reading, typed).expect("checks");
    assert!(check.correct);
    let value = serde_json::to_value(&check).expect("serialises");
    for key in ["correct", "produced"] {
        assert!(value.get(key).is_some(), "the check is missing {key}: {value}");
    }
    assert_eq!(value["correct"], true);
    assert_eq!(value["produced"], "がくせい");

    // And a wrong answer still says what it produced, in kana, for the message.
    let wrong = state().check_word(text, reading, "sensei").expect("checks");
    assert!(!wrong.correct);
    assert_eq!(wrong.produced, "せんせい");
}

#[test]
fn the_passage_list_crosses_with_what_the_reading_screen_offers() {
    let summaries = state().passages();
    assert_eq!(summaries.len(), 3);
    let value = serde_json::to_value(&summaries).expect("serialises");
    for key in ["key", "title", "gloss", "lines", "tokens"] {
        assert!(value[0].get(key).is_some(), "a summary is missing {key}: {}", value[0]);
    }
    assert_eq!(value[0]["key"], "asa");
}

/// The passage itself, and the token shape a tap depends on: `surface`, `rt` (null
/// for kana) and `word` (the dictionary form a card opens, null when there is
/// none).
#[test]
fn a_passage_crosses_with_its_tokens_and_their_links() {
    let passage = state().passage("gakko").expect("gakko is a passage");
    let value = serde_json::to_value(&passage).expect("serialises");
    for key in ["key", "title", "gloss", "lines"] {
        assert!(value.get(key).is_some(), "a passage is missing {key}: {value}");
    }

    let lines = value["lines"].as_array().expect("an array of lines");
    let tokens: Vec<&Value> = lines
        .iter()
        .flat_map(|line| line.as_array().expect("an array of tokens"))
        .collect();

    let verb = tokens
        .iter()
        .find(|t| t["surface"] == "行き")
        .expect("行きます is in the passage");
    assert_eq!(verb["rt"], "いき");
    assert_eq!(
        verb["word"], "行く",
        "the link is the dictionary form, so a tap opens the entry the course teaches"
    );

    let particle = tokens
        .iter()
        .find(|t| t["surface"] == "は")
        .expect("は is in the passage");
    assert_eq!(particle["rt"], Value::Null, "kana needs no ruby");

    // Every token has all three keys, so the screen never reads `undefined`.
    for token in &tokens {
        for key in ["surface", "rt", "word"] {
            assert!(token.get(key).is_some(), "a token is missing {key}: {token}");
        }
    }
}

// ---------------------------------------------------------------------------
// The kanji: the course, one character, and the 214 radicals.
//
// The same two halves as the vocabulary's — every response read with the keys
// `types.ts` declares, and every request driven with the payload `api.ts` builds.

#[test]
fn a_kanji_lesson_crosses_with_its_characters_and_its_grade() {
    let lessons = state().kanji_lessons();
    assert_eq!(lessons.len(), 216);
    let first = serde_json::to_value(&lessons[0]).expect("serialises");
    assert_eq!(
        keys(&first),
        vec!["count", "grade", "gradeName", "kanji", "key", "title"]
    );
    assert_eq!(first["key"], "g1-1");
    assert_eq!(first["title"], "1\u{2013}10");
    assert_eq!(first["grade"], 1);
    assert_eq!(
        first["gradeName"], "kyōiku 1",
        "the grade is named by the vocabulary ladder — see `nihongo_core::grade_name`"
    );
    assert_eq!(first["count"], 10);
    assert_eq!(first["kanji"][0], "日");

    // The remainder's grade is 8 in KANJIDIC2, and the screen is told what to call
    // it rather than mapping a number to a claim itself.
    let last = serde_json::to_value(lessons.last().expect("a last lesson")).expect("serialises");
    assert_eq!(last["grade"], 8);
    assert_eq!(last["gradeName"], "jōyō beyond the school grades");
}

#[test]
fn a_kanji_crosses_with_the_geometry_the_readings_and_the_components() {
    let value =
        serde_json::to_value(state().kanji('学').expect("学 is jōyō")).expect("serialises");
    assert_eq!(
        keys(&value),
        vec![
            "ch",
            "decomposition",
            "frequency",
            "grade",
            "gradeName",
            "kun",
            "meanings",
            "medians",
            "nanori",
            "on",
            "outlines",
            "practisable",
            "radical",
            "strokeCount",
        ]
    );
    assert_eq!(value["ch"], "学");
    assert_eq!(value["grade"], 1);
    assert_eq!(value["gradeName"], "kyōiku 1");
    assert_eq!(value["strokeCount"], 8);
    assert_eq!(value["practisable"], true);
    assert_eq!(value["on"], json!(["ガク"]));
    assert_eq!(value["kun"], json!(["まな.ぶ"]));
    assert!(!value["meanings"].as_array().expect("an array").is_empty());
    assert_eq!(
        value["outlines"].as_array().expect("an array").len(),
        8,
        "one outline per taught stroke, which is what the board draws"
    );
    let point = &value["medians"][0][0];
    assert!(point.get("x").is_some() && point.get("y").is_some(), "{point}");

    // The radical, in both shapes: 学 writes 子 and 子 is radical 39.
    assert_eq!(
        keys(&value["radical"]),
        vec!["ch", "characters", "form", "note", "number", "strokeCount"]
    );
    assert_eq!(value["radical"]["number"], 39);
    assert_eq!(value["radical"]["ch"], "子");
    assert_eq!(value["radical"]["form"], "子");
    assert_eq!(value["radical"]["note"], Value::Null, "present even when there is none");
    assert_eq!(value["radical"]["characters"], 9);

    // The components, from the IDS string: the arrangement in words, and the
    // parts — `ch` null where the source could not name one, `drawable` telling
    // the screen whether it can be opened on the board.
    let decomposition = &value["decomposition"];
    assert_eq!(
        keys(decomposition),
        vec!["layout", "parts", "raw"],
        "the IDS shape `hanzi-core`'s parser hands both apps"
    );
    assert_eq!(decomposition["raw"], "⿳𰃮子");
    assert_eq!(decomposition["layout"], "above, middle and below");
    for part in decomposition["parts"].as_array().expect("an array") {
        assert!(part.get("ch").is_some(), "ch is present even when null: {part}");
        assert!(part.get("drawable").is_some(), "{part}");
    }
    assert!(
        decomposition["parts"]
            .as_array()
            .expect("an array")
            .iter()
            .any(|p| p["ch"] == "子" && p["drawable"] == true),
        "子 is a course character: {decomposition}"
    );
}

/// The request half of the kanji command, driven as `api.ts` builds it:
/// `{ ch }`, one character.
#[test]
fn the_kanji_the_interface_asks_for_is_looked_up_by_its_character() {
    let payload = json!({ "ch": "海" });
    let ch: char = payload["ch"]
        .as_str()
        .expect("a string")
        .chars()
        .next()
        .expect("one character");
    let view = state().kanji(ch).expect("海 is jōyō");
    assert_eq!(view.ch, '海');
    assert_eq!(view.grade, 2);
    assert_eq!(view.radical.number, 85, "氵 is radical 85, whose head form is 水");
    assert_eq!(view.radical.form, '氵');
    assert_eq!(view.radical.ch, '水');

    // And one the course does not hold is a message, camelCase not required.
    assert!(state().kanji('鳩').is_err());
}

/// The response half of the character card's vocabulary list: the page shape
/// `types.ts` declares, and the word fields the card underneath draws.
#[test]
fn the_words_of_a_character_cross_with_what_its_card_lists() {
    let page = state().words_of_kanji('学', 0, 5).expect("学 is jōyō");
    let value = serde_json::to_value(&page).expect("serialises");
    assert_eq!(keys(&value), vec!["ch", "offset", "total", "words"]);
    assert_eq!(value["ch"], "学", "the character the answer is about, echoed back");
    assert_eq!(value["offset"], 0);
    assert!(value["total"].as_u64().expect("a number") > 5, "{value}");

    let words = value["words"].as_array().expect("an array");
    assert_eq!(words.len(), 5);
    for key in ["text", "reading", "meaning", "band", "bandName", "nf", "furigana"] {
        assert!(words[0].get(key).is_some(), "a word is missing {key}: {}", words[0]);
    }
    for word in words {
        assert!(
            word["text"].as_str().expect("a string").contains('学'),
            "{word} is not written with 学"
        );
    }
}

/// The request half, driven exactly as `api.ts` builds it: `{ ch, offset, limit }`.
#[test]
fn the_page_of_a_characters_words_is_read_the_way_the_command_reads_it() {
    let payload = json!({ "ch": "学", "offset": 2, "limit": 3 });
    let ch: char = payload["ch"]
        .as_str()
        .expect("a string")
        .chars()
        .next()
        .expect("one character");
    let offset = payload["offset"].as_u64().expect("a number") as usize;
    let limit = payload["limit"].as_u64().expect("a number") as usize;

    let page = state().words_of_kanji(ch, offset, limit).expect("学 is jōyō");
    assert_eq!(page.ch, '学');
    assert_eq!(page.offset, 2);
    assert_eq!(page.words.len(), 3);
    assert!(page.total > 5, "学 is written in more than one page: {}", page.total);

    // A jōyō character in no word is an empty page, and one outside the set is a
    // message — the card says the first and shows the second.
    let none = state().words_of_kanji('且', 0, 3).expect("且 is jōyō");
    assert_eq!(none.total, 0);
    assert!(none.words.is_empty());
    assert!(state().words_of_kanji('鳩', 0, 3).is_err());
}

#[test]
fn a_radical_family_crosses_with_its_head_form_and_its_members() {    let radicals = state().radicals();
    assert_eq!(radicals.len(), 214);

    let family = radicals.iter().find(|r| r.number == 64).expect("64 is 手");
    let value = serde_json::to_value(family).expect("serialises");
    assert_eq!(
        keys(&value),
        vec!["ch", "characters", "number", "strokeCount"]
    );
    assert_eq!(value["number"], 64);
    assert_eq!(value["ch"], "手", "the head form, never the 扌 inside 持");
    assert_eq!(value["strokeCount"], 4);
    assert_eq!(value["characters"].as_array().expect("an array").len(), 95);
    assert_eq!(value["characters"][0], "手");

    // The sixteen radicals no jōyō character uses are still listed, because
    // "nothing in this set uses it" is a fact about the set.
    let empty: Vec<char> = radicals
        .iter()
        .filter(|r| r.characters.is_empty())
        .map(|r| r.ch)
        .collect();
    assert_eq!(empty.len(), 16, "{empty:?}");
    assert!(empty.contains(&'龠'), "{empty:?}");
}

/// The request half of the radical command: `{ number }`, 1 to 214.
#[test]
fn the_radical_the_interface_asks_for_comes_back_with_its_geometry() {
    let payload = json!({ "number": 64 });
    let number = payload["number"].as_u64().expect("a number") as u8;
    let value = serde_json::to_value(state().radical(number).expect("64 is 手"))
        .expect("serialises");
    assert!(value["outlines"].as_array().expect("an array").len() == 4);
    assert!(value["medians"].as_array().expect("an array").len() == 4);
    assert_eq!(value["ch"], "手");
    assert_eq!(value["characters"].as_array().expect("an array").len(), 95);

    // A number that is not a radical is an error string, not a panic.
    for number in [0u8, 215] {
        let err = state().radical(number).unwrap_err();
        assert!(err.contains("numbered 1 to 214"), "{err}");
    }
}

// ---- the review queue ------------------------------------------------------

/// The response half of grading: the verdict the panel draws, and the two fields
/// beside it that say what the review schedule did with the attempt.
///
/// This is the shape `src/lib/api.ts` unwraps, so a field renamed on one side is
/// a failure here rather than `undefined` on the screen.
#[test]
fn a_graded_attempt_crosses_with_the_verdict_and_the_schedule() {
    let state = state();
    let kana = state.kana('あ').expect("あ");
    let attempt = vec![kana.medians[0].clone()];
    let graded = state
        .grade_and_schedule('あ', &attempt, &GradeOptions::default())
        .expect("grades");

    let value = serde_json::to_value(&graded).expect("serialises");
    assert_eq!(
        keys(&value),
        vec!["nextDue", "report", "scheduled", "warning"],
        "the wrapper the interface unwraps"
    );
    assert!(value["report"]["overall"].is_number(), "the verdict is inside it");
    assert!(value["report"]["strokes"].is_array());
    assert_eq!(value["scheduled"], true, "a character with no card is a review");
    assert!(value["nextDue"].is_string(), "and the attempt produced a due date");
    assert!(value["warning"].is_null(), "nothing to report about the write");
}

/// The rule that keeps one sitting from stretching an interval by months: an
/// attempt on a character that is not due is graded and does not reschedule it.
#[test]
fn a_second_attempt_inside_the_interval_does_not_reschedule() {
    let state = state();
    // The payload the interface posts, for the command that now schedules too.
    let payload = json!({
        "ch": "あ",
        "strokes": [[{"x": 300.0, "y": 300.0}, {"x": 700.0, "y": 300.0}]],
        "options": {"inkWidth": 36.0}
    });
    let ch: char = payload["ch"].as_str().expect("a string").chars().next().expect("one char");
    let strokes: Vec<Vec<Point>> =
        serde_json::from_value(payload["strokes"].clone()).expect("the strokes deserialise");
    let options: GradeOptions =
        serde_json::from_value(payload["options"].clone()).expect("the options deserialise");

    let first = state.grade_and_schedule(ch, &strokes, &options).expect("grades");
    assert!(first.scheduled, "the first attempt on a character is a review");

    let again = state.grade_and_schedule(ch, &strokes, &options).expect("grades");
    assert!(!again.scheduled, "the card is not due again yet");
    assert_eq!(again.next_due, first.next_due, "and the due date did not move");
    assert_eq!(again.report.overall, first.report.overall, "the verdict is still a verdict");
    assert_eq!(state.review_queue(40).cards, 1, "one character, one card");
}

/// What the Review screen reads: the due characters with their prompts, most
/// overdue first, and how much of the schedule stands behind them.
///
/// The schedule is written by hand here for two reasons that are really one: it
/// is a plain file a person can edit, and putting a card *in the past* is the
/// only way to have something due without waiting a day for the scheduler to
/// produce it.
#[test]
fn the_review_queue_crosses_with_the_due_characters() {
    let dir = TempDir::new("review-due");
    fs::write(
        dir.path().join("review.json"),
        r#"{"version":1,"cards":{
             "あ":{"attempts":2,"lapses":1,"due":"2020-01-01T00:00:00Z","intervalDays":0.0,"ease":2.5,"repetitions":0},
             "学":{"attempts":1,"due":"2020-01-02T00:00:00Z","intervalDays":0.0,"ease":2.5,"repetitions":0},
             "亅":{"attempts":1,"due":"2020-01-03T00:00:00Z","intervalDays":0.0,"ease":2.5,"repetitions":0}
           }}"#,
    )
    .expect("writes a schedule");

    let state = AppState::load_at(dir.path());
    let queue = state.review_queue(40);
    let value = serde_json::to_value(&queue).expect("serialises");
    assert_eq!(
        keys(&value),
        vec!["cards", "due", "items", "nextDue", "warning"]
    );
    assert_eq!(value["cards"], 3, "every card, due or not");
    assert_eq!(value["due"], 3);
    assert!(value["nextDue"].is_null(), "something is due, so there is no next one");
    assert!(value["warning"].is_null(), "the file parsed, so there is nothing to say");

    let items = value["items"].as_array().expect("an array");
    assert_eq!(items.len(), 3);
    assert_eq!(
        keys(&items[0]),
        vec![
            "attempts",
            "ch",
            "due",
            "hint",
            "intervalDays",
            "kind",
            "lapses",
            "radical"
        ]
    );

    // Most overdue first, each resolved to what the board will draw: a kana, a
    // jōyō kanji, and a radical head form the character course cannot reach.
    assert_eq!(items[0]["ch"], "あ");
    assert_eq!(items[0]["kind"], "kana", "the tag the interface switches on");
    assert_eq!(items[0]["hint"], "a", "the prompt comes from the readings");
    assert_eq!(items[0]["attempts"], 2);
    assert_eq!(items[0]["lapses"], 1);
    assert!(items[0]["radical"].is_null());

    assert_eq!(items[1]["ch"], "学");
    assert_eq!(items[1]["kind"], "kanji");
    assert!(
        items[1]["hint"].as_str().is_some_and(|hint| !hint.is_empty()),
        "a prompt is never empty: {}",
        items[1]["hint"]
    );

    assert_eq!(items[2]["ch"], "亅");
    assert_eq!(items[2]["kind"], "radical");
    assert_eq!(items[2]["radical"], 6, "the number is how its geometry is fetched");
    assert_eq!(items[2]["hint"], "radical 6 of 214");

    // The limit caps what is *shown*, not what is counted.
    let page = state.review_queue(2);
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.due, 3);
    assert_eq!(page.items[0].ch, 'あ');
}

/// The request half of the review queue: `{ limit }`, as the page size the screen
/// is asking for.
///
/// `src/lib/api.ts` always sends one, so the deserialisation is the part that can
/// go wrong quietly — a renamed key would make the command fall back to its own
/// page size and the screen would show twenty where it asked for two.
#[test]
fn the_page_the_interface_asks_for_is_read_the_way_the_command_reads_it() {
    #[derive(Deserialize)]
    struct ReviewQueueArgs {
        limit: Option<usize>,
    }

    let args: ReviewQueueArgs =
        serde_json::from_value(json!({ "limit": 2 })).expect("the payload the interface posts");
    assert_eq!(args.limit, Some(2), "the screen's page size arrives");

    // And a caller that sends nothing gets the command's own page rather than a
    // rejection, which is what makes `limit` optional on the Rust side.
    let args: ReviewQueueArgs = serde_json::from_value(json!({})).expect("an omitted limit");
    assert_eq!(args.limit, None);
}

/// A character the board cannot draw cannot be graded, so it can never reach the
/// schedule — the card the queue would have no geometry for is never created.
#[test]
fn a_character_the_board_cannot_draw_is_never_scheduled() {
    let state = state();
    let payload = json!({ "ch": "鳩", "strokes": [], "options": { "inkWidth": 36.0 } });
    let ch: char = payload["ch"].as_str().expect("a string").chars().next().expect("one char");
    let strokes: Vec<Vec<Point>> =
        serde_json::from_value(payload["strokes"].clone()).expect("deserialises");

    let err = state
        .grade_and_schedule(ch, &strokes, &GradeOptions::default())
        .unwrap_err();
    assert!(err.contains("not a kana, a jōyō kanji, or a radical"), "{err}");
    assert_eq!(state.review_queue(40).cards, 0, "nothing was scheduled");
}

/// The schedule is the app's own file, in the app's own directory, and it
/// survives a restart — which is the whole point of writing it down.
#[test]
fn what_the_schedule_remembers_survives_a_restart() {
    let dir = TempDir::new("review-restart");
    let due = {
        let state = AppState::load_at(dir.path());
        let graded = state
            .grade_and_schedule('あ', &[], &GradeOptions::default())
            .expect("grades");
        assert!(graded.scheduled);
        graded.next_due.expect("a new card has a due date")
    };

    assert!(
        dir.path().join("review.json").exists(),
        "the attempt was written to the app's own schedule file"
    );
    assert!(
        !dir.path().join("confusions.json").exists(),
        "and the drill's file is a different file, not a shared one"
    );

    // A second AppState over the same directory is what a restart looks like.
    let reopened = AppState::load_at(dir.path());
    let queue = reopened.review_queue(40);
    assert_eq!(queue.cards, 1, "the character was remembered");
    assert_eq!(queue.due, 0, "a failed attempt comes back in a minute, not at once");
    assert_eq!(
        queue.next_due.as_deref(),
        Some(due.as_str()),
        "and it comes back when the schedule said it would"
    );
}

// ---- pronunciation --------------------------------------------------------

/// The arguments `speak` takes, as the webview posts them.
///
/// One field, and the test is still worth having for the reason invariant 14
/// gives: a `#[tauri::command]`'s arguments are read out of the payload **by
/// name**, so a rename on either side is a rejected command rather than a
/// compile error — and this is the app whose Grade button shipped unable to grade
/// a single attempt for exactly that reason. Spelled out here the way
/// `RecordDrillAnswer` is, rather than inferred from the command.
#[derive(Deserialize)]
struct Speak {
    text: String,
}

#[test]
fn the_text_the_interface_asks_to_hear_is_read_the_way_the_command_reads_it() {
    // `src/lib/api.ts` authors exactly this payload for exactly this command:
    //   invoke("speak", { text })
    // The text is whatever the screen is showing — a kana, or a word's own stored
    // reading — and is never composed in Rust; the round trip below is about the
    // key, which is the half a test of `AppState::speak` cannot see.
    let state = state();
    let payload = json!({ "text": "あ" });
    let args: Speak =
        serde_json::from_value(payload).expect("the payload the interface posts deserialises");
    assert_eq!(args.text, "あ");

    // And the command refuses an empty utterance rather than calling silence a
    // success — decided before the synthesiser is reached, which is what makes
    // this safe to run in a suite: a test that really spoke would make a noise on
    // whoever is at the machine. The successful path is checked by tapping the
    // button on a real window, not from here.
    assert!(state.speak("").is_err());
    assert!(state.speak("   ").is_err());
}

/// The voice the interface is offered is a Japanese one.
///
/// `Speaker::default` speaks Chinese, because the two apps that existed before
/// the kana tutor do — so "which language did this app ask for" is a real
/// question with a wrong answer that no other test here would catch. The
/// interface reads the answer to decide whether a "Hear it" button can do
/// anything at all, and null is a legitimate answer on a machine with no Japanese
/// voice installed; what is never legitimate is a Chinese voice.
#[test]
fn the_voice_the_interface_is_offered_is_japanese() {
    let state = state();
    assert_eq!(state.speaker_handle().language(), Language::Japanese);

    match state.voice_status() {
        Some(voice) => {
            assert!(
                voice.contains("ja_JP") || voice.contains("ja-JP") || voice.contains("Japanese"),
                "a description of a Japanese voice, or nothing: {voice}"
            );
            // The shape the interface reads: a string, which it shows.
            assert!(serde_json::to_value(&voice).expect("serialises").is_string());
        }
        None => {
            // Nothing installed: the interface is handed `null` and disables the
            // button, so `None` has to cross as null rather than as a message.
            let absent: Option<String> = None;
            assert!(serde_json::to_value(absent).expect("serialises").is_null());
        }
    }
}
