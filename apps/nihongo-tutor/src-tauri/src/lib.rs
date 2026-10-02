//! The kana tutor's webview-facing surface.
//!
//! The shape is the same as the other two apps': a thin `#[tauri::command]` layer
//! over methods on [`AppState`], so the whole interface can be driven and
//! asserted on from a test without opening a window. `tests/ipc_contract.rs`
//! does exactly that, and it is what stops the JSON the interface reads from
//! drifting away from the JSON this returns.
//!
//! Everything the app teaches is **embedded**: the kana artifact is compiled into
//! the binary with `include_bytes!`. There is no network path in this crate at
//! all — no download, no model, no sync — which is why it has no plugin
//! permissions in `capabilities/default.json`.

// The two that share a name with a command are aliased, so that `fn lessons`
// below is the command and `build_lessons` is the course it serves.
pub mod licences;

use licences::{AppInfo, LicenceNotice};
use nihongo_core::{
    confusions_for, lessons as build_lessons, reading, to_kana, to_kana_in, yoon as build_yoon,
    Confusable, GradeOptions, GradeReport, KanaDataset, Point, Script,
};
use serde::{Deserialize, Serialize};
use tauri::State;

/// The kana dataset, loaded once and shared by every command.
pub struct AppState {
    kana: KanaDataset,
}

impl AppState {
    /// Load the committed artifact. Panics only if the artifact is corrupt,
    /// which is a build-time fault rather than a runtime one: `build.rs` has
    /// already checked the file is there.
    pub fn load() -> Self {
        const ARTIFACT: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../crates/nihongo-core/data/kana.bin.gz"
        ));
        Self {
            kana: KanaDataset::from_gzip_bytes(ARTIFACT)
                .expect("the committed kana artifact decodes"),
        }
    }

    pub fn dataset(&self) -> &KanaDataset {
        &self.kana
    }

    /// What is in the course, in numbers.
    pub fn stats(&self) -> DatasetStats {
        let hiragana = self.kana.of_script(Script::Hiragana).count();
        let katakana = self.kana.of_script(Script::Katakana).count();
        let lesson_count = build_lessons(&self.kana, Script::Hiragana).len()
            + build_lessons(&self.kana, Script::Katakana).len();
        DatasetStats {
            kana: self.kana.len(),
            hiragana,
            katakana,
            lessons: lesson_count,
            strokes: self
                .kana
                .kana()
                .iter()
                .map(|k| k.stroke_count as usize)
                .sum(),
        }
    }

    /// The course for one script, in teaching order.
    pub fn lessons(&self, script: Script) -> Vec<LessonView> {
        build_lessons(&self.kana, script)
            .into_iter()
            .map(|lesson| LessonView {
                count: lesson.kana.len(),
                key: lesson.key,
                title: lesson.title,
                kana: lesson.kana,
                voiced: lesson.voiced,
            })
            .collect()
    }

    /// One kana, with everything a practice screen needs: the geometry it draws
    /// and grades against, how it is read, and what it is confused with.
    pub fn kana(&self, ch: char) -> Result<KanaView, String> {
        let kana = self
            .kana
            .get(ch)
            .ok_or_else(|| format!("{ch} (U+{:04X}) is not in the kana set", ch as u32))?;
        let reading = reading(ch).ok_or_else(|| format!("{ch} has no reading"))?;
        Ok(KanaView {
            ch: kana.ch,
            script: kana.script.name().to_string(),
            stroke_count: kana.stroke_count,
            practisable: kana.is_practisable(),
            romaji: reading.spellings().map(str::to_string).collect(),
            hepburn: reading.hepburn.first().copied().unwrap_or_default().to_string(),
            silent: reading.is_silent(),
            outlines: kana.outlines.clone(),
            medians: kana.medians.clone(),
            confusions: self.confusions(ch),
        })
    }

    /// Grade a handwritten attempt. The scoring is `hanzi-core`'s, unchanged:
    /// shape, placement, ink and order, against the corrected stroke geometry.
    pub fn grade(
        &self,
        ch: char,
        strokes: &[Vec<Point>],
        options: &GradeOptions,
    ) -> Result<GradeReport, String> {
        let kana = self
            .kana
            .get(ch)
            .ok_or_else(|| format!("{ch} (U+{:04X}) is not in the kana set", ch as u32))?;
        if !kana.is_practisable() {
            return Err(format!("{ch} has no stroke geometry to grade against"));
        }
        Ok(nihongo_core::grade_with_outlines(
            kana.reference_medians(),
            &kana.outlines,
            strokes,
            options,
        ))
    }

    /// The kana a given kana is confused with, from the classic set.
    pub fn confusions(&self, ch: char) -> Vec<ConfusionView> {
        confusions_for(&self.kana, ch)
            .into_iter()
            .map(|pair: Confusable| {
                let other = if pair.a == ch { pair.b } else { pair.a };
                ConfusionView {
                    ch: other,
                    tell: pair.tell.to_string(),
                }
            })
            .collect()
    }

    /// Every kana that appears in a confusion pair, with the kana it is
    /// confused with and how to tell them apart.
    ///
    /// This is the pool a discrimination drill draws from: a kana, its reading,
    /// and its partners. A kana that confuses nobody is not in it.
    pub fn drill_pool(&self) -> Vec<DrillKana> {
        let mut pool: Vec<DrillKana> = Vec::new();
        for pair in nihongo_core::CONFUSABLE {
            for (ch, other) in [(pair.a, pair.b), (pair.b, pair.a)] {
                if self.kana.get(ch).is_none() || self.kana.get(other).is_none() {
                    continue;
                }
                let Some(reading) = reading(ch) else { continue };
                let hepburn = reading.hepburn.first().copied().unwrap_or_default().to_string();
                if hepburn.is_empty() {
                    continue;
                }
                match pool.iter_mut().find(|d| d.ch == ch) {
                    Some(existing) => {
                        if !existing.partners.contains(&other) {
                            existing.partners.push(other);
                        }
                    }
                    None => pool.push(DrillKana {
                        ch,
                        script: self
                            .kana
                            .get(ch)
                            .map(|k| k.script.name())
                            .unwrap_or("hiragana")
                            .to_string(),
                        hepburn,
                        partners: vec![other],
                    }),
                }
            }
        }
        pool.sort_by_key(|d| d.ch as u32);
        pool
    }

    /// The yōon digraphs for one script — the pairs that make one mora.
    pub fn yoon(&self, script: Script) -> Vec<YoonView> {
        build_yoon(script)
            .into_iter()
            .map(|y| YoonView {
                key: y.key,
                display: y.display,
                hepburn: y.hepburn,
                kunrei: y.kunrei,
                kana: y.kana,
            })
            .collect()
    }
}

/// What the course contains.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetStats {
    pub kana: usize,
    pub hiragana: usize,
    pub katakana: usize,
    pub lessons: usize,
    pub strokes: usize,
}

/// One lesson, as the sidebar lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonView {
    pub key: String,
    pub title: String,
    pub kana: Vec<char>,
    pub voiced: bool,
    pub count: usize,
}

/// One kana, as a practice screen uses it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KanaView {
    pub ch: char,
    pub script: String,
    pub stroke_count: u8,
    pub practisable: bool,
    /// Every spelling a learner might type, Hepburn first.
    pub romaji: Vec<String>,
    /// The one to show as *the* reading.
    pub hepburn: String,
    /// True for っ and ー, which have no sound of their own.
    pub silent: bool,
    /// SVG path data in font space, one per taught stroke, in stroke order.
    pub outlines: Vec<String>,
    /// Centre-lines in display space, for the faint guide and for grading.
    pub medians: Vec<Vec<Point>>,
    pub confusions: Vec<ConfusionView>,
}

/// A kana this one is mistaken for.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfusionView {
    pub ch: char,
    pub tell: String,
}

/// A kana the discrimination drill can ask about.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DrillKana {
    pub ch: char,
    pub script: String,
    /// The reading to prompt with.
    pub hepburn: String,
    /// The kana it is mistaken for — the wrong answers the drill offers.
    pub partners: Vec<char>,
}

/// A yōon digraph.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YoonView {
    pub key: String,
    pub display: String,
    pub hepburn: String,
    pub kunrei: String,
    pub kana: Vec<char>,
}

/// The result of checking a typed romaji answer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingCheck {
    pub correct: bool,
    /// What the typing actually produced, so a wrong answer can show the learner
    /// what they wrote rather than only that it was wrong.
    pub produced: String,
}

fn script_of(name: &str) -> Result<Script, String> {
    Script::from_name(name).ok_or_else(|| format!("unknown script {name:?}; expected hiragana or katakana"))
}

#[tauri::command]
fn app_info() -> AppInfo {
    licences::APP
}

/// Every notice this app owes, so a Licences screen can show them without
/// shipping a second copy of the text in the frontend.
#[tauri::command]
fn licences() -> Vec<LicenceNotice> {
    licence_notices()
}

/// The notice catalogue, for a caller that should not have to open a window.
pub fn licence_notices() -> Vec<LicenceNotice> {
    licences::notices().to_vec()
}

#[tauri::command]
fn dataset_stats(state: State<'_, AppState>) -> DatasetStats {
    state.stats()
}

#[tauri::command]
fn lessons(state: State<'_, AppState>, script: String) -> Result<Vec<LessonView>, String> {
    Ok(state.lessons(script_of(&script)?))
}

#[tauri::command]
fn kana(state: State<'_, AppState>, ch: char) -> Result<KanaView, String> {
    state.kana(ch)
}

#[tauri::command]
fn grade_attempt(
    state: State<'_, AppState>,
    ch: char,
    strokes: Vec<Vec<Point>>,
    options: Option<GradeOptions>,
) -> Result<GradeReport, String> {
    state.grade(ch, &strokes, &options.unwrap_or_default())
}

/// The pool the confusion drill draws from.
#[tauri::command]
fn drill_pool(state: State<'_, AppState>) -> Vec<DrillKana> {
    state.drill_pool()
}

/// Check a typed reading against a kana, accepting either romanisation.
#[tauri::command]
fn check_reading(ch: char, typed: String) -> ReadingCheck {
    let produced = to_kana(&typed).unwrap_or_default();
    ReadingCheck {
        correct: nihongo_core::matches_reading(ch, &typed),
        produced,
    }
}

/// Turn typed romaji into kana — the input half of the course.
#[tauri::command]
fn romaji_to_kana(input: String, script: Option<String>) -> Result<String, String> {
    let script = match script.as_deref() {
        None => Script::Hiragana,
        Some(name) => script_of(name)?,
    };
    to_kana_in(script, &input).map_err(|e| e.to_string())
}

#[tauri::command]
fn yoon(script: String) -> Result<Vec<YoonSummary>, String> {
    Ok(build_yoon(script_of(&script)?)
        .into_iter()
        .map(|y| YoonSummary {
            hepburn: y.hepburn,
            display: y.display,
        })
        .collect())
}

/// A yōon digraph in the cut-down form the input helper needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YoonSummary {
    hepburn: String,
    display: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::load())
        .invoke_handler(tauri::generate_handler![
            app_info,
            licences,
            dataset_stats,
            lessons,
            kana,
            grade_attempt,
            drill_pool,
            check_reading,
            romaji_to_kana,
            yoon,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the kana tutor");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState::load()
    }

    #[test]
    fn the_embedded_dataset_is_the_whole_kana_set() {
        let state = state();
        let stats = state.stats();
        assert_eq!(stats.kana, 177);
        assert_eq!(stats.hiragana, 86);
        assert_eq!(stats.katakana, 91);
        assert_eq!(stats.strokes, 516);
        // Eleven plain rows, five voiced rows, then small, rare, and — in
        // katakana only — the v-series and the prolonged sound mark.
        assert_eq!(state.lessons(Script::Hiragana).len(), 18);
        assert_eq!(state.lessons(Script::Katakana).len(), 20);
        assert_eq!(stats.lessons, 38);
    }

    #[test]
    fn a_kana_view_carries_the_geometry_the_screen_draws_and_grades() {
        let view = state().kana('あ').expect("あ is in the set");
        assert_eq!(view.script, "hiragana");
        assert_eq!(view.stroke_count, 3);
        assert!(view.practisable);
        assert!(!view.silent);
        assert_eq!(view.hepburn, "a");
        assert_eq!(view.outlines.len(), 3);
        assert_eq!(view.medians.len(), 3);
        assert!(view.romaji.contains(&"a".to_string()));
    }

    #[test]
    fn a_kana_view_explains_what_it_is_confused_with() {
        let view = state().kana('シ').expect("シ is in the set");
        assert!(
            view.confusions.iter().any(|c| c.ch == 'ツ'),
            "シ should be flagged against ツ"
        );
        assert!(!view.confusions[0].tell.is_empty());
    }

    #[test]
    fn a_kana_outside_the_set_is_an_error_not_a_panic() {
        let err = state().kana('一').unwrap_err();
        assert!(err.contains("not in the kana set"), "{err}");
    }

    #[test]
    fn a_silent_mark_reports_itself_as_silent() {
        assert!(state().kana('ー').expect("ー is in the set").silent);
        assert!(state().kana('っ').expect("っ is in the set").silent);
        assert!(!state().kana('ん').expect("ん is in the set").silent);
    }

    #[test]
    fn grading_traces_the_stored_centre_line_as_legible() {
        let state = state();
        let view = state.kana('ー').expect("ー is in the set");
        let attempt = vec![view.medians[0].clone()];
        let report = state
            .grade('ー', &attempt, &GradeOptions::default())
            .expect("grades");
        assert!(report.legible, "tracing the guide must be legible: {:.0}", report.overall);
        assert_eq!(report.expected_strokes, 1);
    }

    #[test]
    fn grading_a_mark_that_is_not_taught_is_an_error() {
        let err = state()
            .grade('一', &[], &GradeOptions::default())
            .unwrap_err();
        assert!(err.contains("not in the kana set"), "{err}");
    }

    #[test]
    fn grading_uses_the_corrected_stroke_count_not_the_raw_one() {
        // あ arrives from upstream as four drawing segments and is taught with
        // three. The grader must ask for three.
        let state = state();
        let view = state.kana('あ').expect("あ is in the set");
        assert_eq!(view.stroke_count, 3);
        let report = state
            .grade('あ', &[], &GradeOptions::default())
            .expect("grades");
        assert_eq!(report.expected_strokes, 3);
    }

    #[test]
    fn the_drill_pool_holds_the_confusable_kana_and_their_partners() {
        let pool = state().drill_pool();
        assert!(!pool.is_empty());

        // シ and ツ are the pair everyone starts with, and each must offer the
        // other as a wrong answer.
        let shi = pool.iter().find(|d| d.ch == 'シ').expect("シ is drillable");
        assert_eq!(shi.hepburn, "shi");
        assert_eq!(shi.script, "katakana");
        assert!(shi.partners.contains(&'ツ'));
        let tsu = pool.iter().find(|d| d.ch == 'ツ').expect("ツ is drillable");
        assert!(tsu.partners.contains(&'シ'));

        // A kana with a partner that is not in the set is not offered, and every
        // prompt is answerable: a reading exists and at least one partner does.
        for entry in &pool {
            assert!(!entry.hepburn.is_empty(), "{} has no prompt", entry.ch);
            assert!(!entry.partners.is_empty(), "{} has no wrong answers", entry.ch);
            assert!(!entry.partners.contains(&entry.ch), "{} is its own partner", entry.ch);
            for partner in &entry.partners {
                assert!(
                    state().kana(*partner).is_ok(),
                    "{} offers {partner}, which cannot be drawn",
                    entry.ch
                );
            }
        }

        // No duplicates, and stable order.
        let mut chars: Vec<char> = pool.iter().map(|d| d.ch).collect();
        let total = chars.len();
        chars.sort_unstable();
        chars.dedup();
        assert_eq!(total, chars.len(), "a kana appears twice in the pool");
    }

    #[test]
    fn a_kana_that_confuses_nobody_is_not_in_the_drill() {
        assert!(!state().drill_pool().iter().any(|d| d.ch == 'あ'));
    }

    #[test]
    fn reading_checks_accept_both_romanisations() {
        assert!(check_reading('し', "shi".into()).correct);
        assert!(check_reading('し', "si".into()).correct);
        assert!(!check_reading('し', "chi".into()).correct);
        // A wrong answer still reports what it produced.
        let wrong = check_reading('し', "chi".into());
        assert_eq!(wrong.produced, "ち");
    }

    #[test]
    fn romaji_converts_in_both_scripts() {
        assert_eq!(romaji_to_kana("kana".into(), None).expect("converts"), "かな");
        assert_eq!(
            romaji_to_kana("kana".into(), Some("katakana".into())).expect("converts"),
            "カナ"
        );
        assert!(romaji_to_kana("kana".into(), Some("kanji".into())).is_err());
        assert!(romaji_to_kana("!!!".into(), None).is_err());
    }

    #[test]
    fn every_lesson_names_kana_the_screen_can_open() {
        let state = state();
        for script in [Script::Hiragana, Script::Katakana] {
            for lesson in state.lessons(script) {
                assert_eq!(lesson.count, lesson.kana.len());
                for ch in lesson.kana {
                    assert!(
                        state.kana(ch).is_ok(),
                        "lesson {} lists {ch}, which cannot be opened",
                        lesson.key
                    );
                }
            }
        }
    }

    #[test]
    fn the_yoon_helper_covers_every_digraph() {
        assert_eq!(yoon("hiragana".into()).expect("known script").len(), 33);
        let all = yoon("katakana".into()).expect("known script");
        assert!(all.iter().any(|y| y.display == "キュ" && y.hepburn == "kyu"));
        assert!(yoon("kanji".into()).is_err());
    }
}
