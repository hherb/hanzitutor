//! The commands the frontend can call.
//!
//! The `#[tauri::command]` functions are deliberately thin: each forwards to a
//! method on [`AppState`]. That keeps the whole surface the webview depends on
//! testable without opening a window — see `tests/ipc_contract.rs`.

use hanzi_core::{
    build_lessons, grade_with_outlines, Character, CursorView, GradeOptions, GradeReport, Lesson,
    Point, ProgressView, ReviewView, SettingsView, TextLookup, VocabView, Word,
};
use serde::Serialize;
use tauri::State;

use crate::licences::{AppInfo, LicenceNotice};
use crate::state::{AppState, ProgressState, VocabState};

/// How many characters make up one lesson.
pub const LESSON_SIZE: usize = 10;

/// How many words one search returns.
///
/// The word list is thousands long, so a page is capped and the true total is
/// reported beside it — the same honesty the review queue uses.
pub const WORD_PAGE: usize = 100;

/// Summary of what the app ships with, shown in the sidebar.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetStats {
    pub characters: usize,
    pub teachable: usize,
    pub lessons: usize,
    pub lesson_size: usize,
    /// How many words the dictionary holds.
    pub words: usize,
    /// How many words sit at each HSK level, lowest first.
    pub word_levels: Vec<LevelCount>,
}

/// How many words one HSK level holds.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelCount {
    pub level: u8,
    pub words: usize,
}

/// One page of a word search, with the number of matches behind it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordSearchView {
    /// The page itself, best match first.
    pub words: Vec<Word>,
    /// How many words matched in total; `words` is capped to one page.
    pub total: usize,
}

impl AppState {
    pub fn stats(&self) -> DatasetStats {
        let dataset = &self.dataset;
        DatasetStats {
            characters: dataset.len(),
            teachable: dataset.ranked().count(),
            lessons: build_lessons(dataset, LESSON_SIZE).len(),
            lesson_size: LESSON_SIZE,
            words: dataset.word_count(),
            word_levels: dataset
                .words_per_level()
                .into_iter()
                .map(|(level, words)| LevelCount { level, words })
                .collect(),
        }
    }

    /// One page of a word search, plus how many words matched altogether.
    ///
    /// `query` is matched against characters, readings and definitions; `level`
    /// narrows it to one HSK level; an empty query browses from the most useful
    /// word down. See [`hanzi_core::Dataset::search_words`].
    pub fn search_words(&self, query: &str, level: Option<u8>, limit: usize) -> WordSearchView {
        WordSearchView {
            words: self
                .dataset
                .search_words(query, level, limit)
                .into_iter()
                .cloned()
                .collect(),
            total: self.dataset.count_words(query, level),
        }
    }

    /// The whole course in order. A few thousand entries is small enough to
    /// send in one go, which keeps navigation in the frontend trivial.
    pub fn lessons(&self) -> Vec<Lesson> {
        build_lessons(&self.dataset, LESSON_SIZE)
    }

    /// Everything needed to display and practise one character: stroke outlines
    /// in font space, stroke centre-lines in display space, and metadata.
    pub fn character(&self, ch: char) -> Result<Character, String> {
        self.dataset
            .get(ch)
            .cloned()
            .ok_or_else(|| format!("'{ch}' is not in the character dataset"))
    }

    /// Grade a handwritten attempt. `strokes` are in display space (origin
    /// top-left, y downwards, within a 1024x1024 box), in drawing order.
    ///
    /// The character's own stroke outlines go in too, so the ink measure
    /// compares against the ink the guide shows rather than only against the
    /// centre-line. See [`hanzi_core::grade_with_outlines`].
    pub fn grade(
        &self,
        ch: char,
        strokes: &[Vec<Point>],
        options: &GradeOptions,
    ) -> Result<GradeReport, String> {
        let character = self
            .dataset
            .get(ch)
            .ok_or_else(|| format!("'{ch}' is not in the character dataset"))?;
        Ok(grade_with_outlines(
            character.reference_medians(),
            &character.outlines,
            strokes,
            options,
        ))
    }
}

#[tauri::command]
pub fn dataset_stats(state: State<'_, AppState>) -> DatasetStats {
    state.stats()
}

#[tauri::command]
pub fn lessons(state: State<'_, AppState>) -> Vec<Lesson> {
    state.lessons()
}

#[tauri::command]
pub fn character(state: State<'_, AppState>, ch: char) -> Result<Character, String> {
    state.character(ch)
}

/// Every character the board can ask for.
///
/// The interface needs this to tell a word from the punctuation around it: a
/// sentence added to the vocabulary list should be written one character at a
/// time with the commas skipped, not dead-end on a mark that has no strokes.
/// Sent once and held, like the course itself.
#[tauri::command]
pub fn teachable_characters(state: State<'_, AppState>) -> Vec<char> {
    state.dataset.practisable_characters()
}

// ---- the word dictionary ----------------------------------------------------

/// Search the HSK word list by character, reading or meaning.
///
/// `query` empty browses from the most useful word down; a single character
/// lists every word containing it. `level` narrows to one HSK level. The result
/// is capped at [`WORD_PAGE`] unless `limit` says otherwise, with the true
/// total reported beside it.
#[tauri::command]
pub fn search_words(
    state: State<'_, AppState>,
    query: String,
    level: Option<u8>,
    limit: Option<usize>,
) -> WordSearchView {
    let limit = limit.unwrap_or(WORD_PAGE).clamp(1, WORD_PAGE);
    state.search_words(&query, level, limit)
}

#[tauri::command]
pub fn grade_attempt(
    state: State<'_, AppState>,
    ch: char,
    strokes: Vec<Vec<Point>>,
    options: Option<GradeOptions>,
) -> Result<GradeReport, String> {
    state.grade(ch, &strokes, &options.unwrap_or_default())
}

/// Start pronouncing a character. Returns immediately; it does not wait for the
/// audio to finish.
#[tauri::command]
pub fn speak(state: State<'_, AppState>, text: String) -> Result<(), String> {
    state.speech.speak(&text)
}

/// Cut off the current utterance, if any.
#[tauri::command]
pub fn stop_speaking(state: State<'_, AppState>) {
    state.speech.stop();
}

/// The voice pronunciation will use, or `None` when the system has no Chinese
/// voice installed. The interface disables the control rather than offering
/// something that cannot work.
#[tauri::command]
pub fn speech_status(state: State<'_, AppState>) -> Option<String> {
    state.speech.status()
}

// ---- Personal vocabulary list ---------------------------------------------

/// Resolve a character or word for the add form.
///
/// A single character gets its reading *and* meaning. A word gets its readings
/// composed into a draft pinyin, but **not** a meaning: a word's meaning cannot
/// be derived from its characters, and a plausible-looking invention would be
/// worse than a blank the learner fills in. The per-character hints come back
/// either way so there is something to work from.
#[tauri::command]
pub fn lookup_text(state: State<'_, AppState>, text: String) -> TextLookup {
    state.dataset.lookup_text(&text)
}

/// A change to the vocabulary list, with a note about what happened.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabOutcome {
    pub view: VocabView,
    /// Human-readable note for a status line.
    pub message: String,
}

/// Persist a change and hand back the list as it now stands.
///
/// A save failure is reported through the view's `warning` rather than as a hard
/// error, because the change *did* take effect in memory: the interface should
/// show it while explaining that it was not written to disk.
fn committed(vocab: &mut VocabState) -> VocabView {
    let mut view = vocab.view();
    if let Some(warning) = vocab.save() {
        view.warning = Some(warning);
    }
    view
}

/// The whole list: every entry and every group.
#[tauri::command]
pub fn vocabulary(state: State<'_, AppState>) -> VocabView {
    state.lock_vocab().view()
}

#[tauri::command]
pub fn vocab_add(
    state: State<'_, AppState>,
    text: String,
    pinyin: String,
    meaning: String,
    group: Option<String>,
) -> Result<VocabView, String> {
    let mut vocab = state.lock_vocab();
    vocab
        .store
        .add_entry(&text, &pinyin, &meaning, group.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(committed(&mut vocab))
}

#[tauri::command]
pub fn vocab_update(
    state: State<'_, AppState>,
    id: u64,
    pinyin: String,
    meaning: String,
    group: Option<String>,
) -> Result<VocabView, String> {
    let mut vocab = state.lock_vocab();
    vocab
        .store
        .update_entry(id, &pinyin, &meaning, group.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(committed(&mut vocab))
}

#[tauri::command]
pub fn vocab_remove(state: State<'_, AppState>, id: u64) -> Result<VocabView, String> {
    let mut vocab = state.lock_vocab();
    vocab.store.remove_entry(id).map_err(|e| e.to_string())?;
    Ok(committed(&mut vocab))
}

#[tauri::command]
pub fn vocab_add_group(state: State<'_, AppState>, name: String) -> Result<VocabView, String> {
    let mut vocab = state.lock_vocab();
    vocab.store.add_group(&name).map_err(|e| e.to_string())?;
    Ok(committed(&mut vocab))
}

#[tauri::command]
pub fn vocab_rename_group(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<VocabView, String> {
    let mut vocab = state.lock_vocab();
    vocab
        .store
        .rename_group(&from, &to)
        .map_err(|e| e.to_string())?;
    Ok(committed(&mut vocab))
}

/// Remove a group.
///
/// `purge` false keeps its entries and leaves them unfiled; true deletes them.
/// The default in the interface is false, because deleting a label should not
/// destroy work.
#[tauri::command]
pub fn vocab_remove_group(
    state: State<'_, AppState>,
    name: String,
    purge: bool,
) -> Result<VocabView, String> {
    let mut vocab = state.lock_vocab();
    vocab
        .store
        .remove_group(&name, purge)
        .map_err(|e| e.to_string())?;
    Ok(committed(&mut vocab))
}

/// Record a practice attempt against an entry. `score` is the 0..=100 headline
/// score from the grading engine.
#[tauri::command]
pub fn vocab_record_attempt(
    state: State<'_, AppState>,
    id: u64,
    score: f32,
) -> Result<VocabView, String> {
    let mut vocab = state.lock_vocab();
    vocab
        .store
        .record_attempt(id, score)
        .map_err(|e| e.to_string())?;
    Ok(committed(&mut vocab))
}

/// Write the list to `path` as `format`, which is `"json"` (lossless) or
/// `"csv"` (for spreadsheets). Returns a note for the interface.
#[tauri::command]
pub fn vocab_export(
    state: State<'_, AppState>,
    path: String,
    format: String,
) -> Result<String, String> {
    let vocab = state.lock_vocab();
    let count = vocab.store.entries().len();
    let (contents, label) = match format.as_str() {
        "json" => (
            vocab.store.export_json().map_err(|e| e.to_string())?,
            "JSON",
        ),
        "csv" => (vocab.store.export_csv(), "CSV"),
        other => return Err(format!("unknown export format {other:?}")),
    };
    std::fs::write(&path, contents).map_err(|e| format!("could not write {path}: {e}"))?;
    Ok(format!("Exported {count} entries as {label} to {path}"))
}

/// Read a previously exported document from `path`.
///
/// `merge` false replaces the current list; true adds to it, skipping entries
/// whose text is already present in the same group.
#[tauri::command]
pub fn vocab_import(
    state: State<'_, AppState>,
    path: String,
    merge: bool,
) -> Result<VocabOutcome, String> {
    let json =
        std::fs::read_to_string(&path).map_err(|e| format!("could not read {path}: {e}"))?;

    let mut vocab = state.lock_vocab();
    let summary = vocab
        .store
        .import_json(&json, merge)
        .map_err(|e| e.to_string())?;
    let view = committed(&mut vocab);

    let mut message = if summary.replaced {
        format!("Replaced your list with {} entries", summary.added)
    } else {
        format!("Added {} entries", summary.added)
    };
    if summary.skipped_duplicates > 0 {
        message.push_str(&format!(
            ", skipped {} already in the list",
            summary.skipped_duplicates
        ));
    }
    if summary.groups_added > 0 {
        message.push_str(&format!(", {} new groups", summary.groups_added));
    }

    Ok(VocabOutcome { view, message })
}

// ---- practice progress and review ------------------------------------------

/// Persist a change to the schedule and hand back the schedule as it now stands.
///
/// As with the vocabulary list, a save failure is reported through the view's
/// `warning` rather than as a hard error: the attempt *was* recorded in memory,
/// and appearing to lose it would be worse than saying it was not written.
fn committed_progress(progress: &mut ProgressState) -> ProgressView {
    let mut view = progress.view();
    if let Some(warning) = progress.save() {
        view.warning = Some(warning);
    }
    view
}

/// Every practised character: attempts, best score, history and due date.
#[tauri::command]
pub fn progress(state: State<'_, AppState>) -> ProgressView {
    state.lock_progress().view()
}

/// Record one graded character.
///
/// `score` is the 0..=100 headline score from the grading engine; the store
/// derives the review rating from it and schedules the next one. A character
/// with no card is one that has never been attempted.
#[tauri::command]
pub fn record_progress(
    state: State<'_, AppState>,
    ch: char,
    score: f32,
) -> Result<ProgressView, String> {
    let mut progress = state.lock_progress();
    progress
        .store
        .record(ch, score)
        .map_err(|e| e.to_string())?;
    Ok(committed_progress(&mut progress))
}

/// What is due for review now, most overdue first, from the course and the
/// vocabulary list. `dueCount` is the full total; `items` is the capped session.
#[tauri::command]
pub fn review_queue(state: State<'_, AppState>) -> ReviewView {
    state.review_queue()
}

/// Where the reader was in the course, so the app can open there next time.
#[tauri::command]
pub fn course_cursor(state: State<'_, AppState>) -> CursorView {
    state.lock_cursor().view()
}

/// Move the course cursor.
///
/// The index is clamped inside [`AppState::set_cursor`], so a stale or
/// hand-edited value cannot point past the end of the course.
#[tauri::command]
pub fn set_course_cursor(state: State<'_, AppState>, index: usize) -> CursorView {
    state.set_cursor(index)
}

/// The learner's settings, with an unset preference reported as `null`.
///
/// `null` is not `false`: it means nobody has chosen, which is what lets the
/// interface follow the device — click-to-draw on a trackpad or with a mouse,
/// dragging with a stylus or a finger — and only override it once the learner
/// has actually flipped the switch.
#[tauri::command]
pub fn settings(state: State<'_, AppState>) -> SettingsView {
    state.lock_settings().view()
}

/// Change a setting.
///
/// `clickToDraw` is `true`, `false`, or `null` to go back to the device's own
/// default. The change is written straight away; the view that comes back is
/// what the interface should render, warning included.
#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, click_to_draw: Option<bool>) -> SettingsView {
    state.set_click_to_draw(click_to_draw)
}

/// Echo a line from the webview to stderr.
///
/// A webview's `console.log` never reaches the terminal, which makes a blank
/// window hard to diagnose: a failed `invoke` and a rendering bug look the
/// same. The frontend calls this at each milestone so `pnpm dev` shows how far
/// it got. Development aid only — nothing is stored.
#[tauri::command]
pub fn webview_log(message: String) {
    eprintln!("[webview] {message}");
}

// ---- what the app is, and what it ships under ------------------------------

/// The app's name, version and licence, for the About screen.
///
/// The version is the workspace `Cargo.toml`'s, which `tests/licences.rs` holds
/// equal to the versions in `tauri.conf.json` and `package.json` — three files
/// that would otherwise drift apart silently.
#[tauri::command]
pub fn app_info() -> AppInfo {
    crate::licences::APP
}

/// Every licence and attribution notice the app ships with, full text included.
///
/// The texts are compiled into the binary, so this cannot fail on a packaged
/// build the way reading them out of a resource directory could. The same files
/// are also copied into the bundle as plain text, for anyone auditing it
/// without launching it.
#[tauri::command]
pub fn licence_notices() -> Vec<LicenceNotice> {
    crate::licences::NOTICES.to_vec()
}
