//! The commands the frontend can call.
//!
//! The `#[tauri::command]` functions are deliberately thin: each forwards to a
//! method on [`AppState`]. That keeps the whole surface the webview depends on
//! testable without opening a window — see `tests/ipc_contract.rs`.

use hanzi_core::{
    build_lessons, grade_with_outlines, tone::ToneAttempt, BoardSize, Character, CursorView,
    GradeOptions, GradeReport, Grade, Heard, Lesson, Pace, Point, ProgressView, ReviewView,
    SettingsView, TextLookup, ToneVerdict, ToneTarget, VocabView, Word,
};
use serde::Serialize;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::asr::AsrStatus;
use crate::capture::MicrophoneStatus;
use crate::licences::{AppInfo, LicenceNotice};
use crate::state::{AppState, ProgressState, VocabState};
use crate::sync::{AutoSync, SyncService, SyncView};

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

/// One voice the settings screen can offer, as it reads in a list.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceOption {
    /// The name as the system reports it, which is what a choice is stored as.
    pub name: String,
    /// The locale, e.g. `zh_CN`. Shown beside the name because it is the only
    /// thing that tells two similarly named Chinese voices apart.
    pub locale: String,
    /// Whether this voice needs a network connection to speak.
    ///
    /// Never true on macOS or iOS, where every voice the system lists is local.
    /// On Android it is the difference between a voice that works on a train and
    /// one that does not, which is worth saying before the learner picks it
    /// rather than after.
    pub network: bool,
}

/// The Chinese voices this machine offers, and the one actually in use.
///
/// Both halves matter to the settings screen: the list is what can be chosen,
/// and `active` is what a choice *resolved to* — which is not always the same
/// thing, since a preference naming a voice that is not installed falls back to
/// the automatic choice rather than failing. Without `active` the screen could
/// not say that honestly.
///
/// `available` is empty where the platform has no voice enumeration (see
/// `speech.rs`), and the screen says so rather than showing an empty list.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoicesView {
    pub available: Vec<VoiceOption>,
    /// The name of the voice in use, or `None` when nothing Chinese is
    /// installed and pronunciation is unavailable.
    pub active: Option<String>,
}

/// The voices a Chinese character can be spoken with, and the one in use.
///
/// Served from a cached list: enumerating the system's voices takes about a
/// second, and the settings screen asks every time it is opened.
#[tauri::command]
pub fn voices(state: State<'_, AppState>) -> VoicesView {
    state.voices()
}

// ---- tone practice ---------------------------------------------------------

/// One syllable's worth of a scored utterance.
///
/// The character and its reading travel with the judgement so that the interface
/// can label each syllable of a word without holding any pinyin rules of its own.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneSyllableResult {
    /// Which syllable this is, counting from 1.
    pub position: usize,
    pub ch: char,
    /// The reading as a dictionary writes it, tone mark included, e.g. `"nǐ"`.
    pub reading: String,
    /// The tone the dictionary gives this syllable, before sandhi.
    pub citation: u8,
    /// The tone actually spoken in this word, which is what was scored.
    pub spoken: u8,
    pub attempt: ToneAttempt,
}

/// A scored character or word, as the interface reads it.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneResult {
    /// One entry per syllable, in the order they were spoken, always the same
    /// length as the target's syllables.
    pub syllables: Vec<ToneSyllableResult>,
    pub verdict: ToneVerdict,
    /// Mean of the scores of the syllables that could be scored; `0` when none
    /// could. On the same 0..=100 scale and the same bands as a handwriting
    /// score.
    pub score: f32,
    pub grade: Grade,
    /// One plain sentence, worded by the Rust side. The interface styles this; it
    /// does not reword it, so a judgement is expressed in exactly one place.
    pub detail: String,
    /// True when tone sandhi changed the tones, so that the interface can explain
    /// why it is not asking for the tone a dictionary prints.
    pub sandhi_applied: bool,
    /// Where the syllables were divided, in milliseconds **from the start of
    /// speech**. Same baseline as `voiced_ms`; empty for a single syllable.
    pub boundaries_ms: Vec<u32>,
    pub voiced_ms: u32,
    pub span_ms: u32,
    pub median_hz: f32,
    /// What a recognition model heard, when one is installed.
    ///
    /// `None` whenever no model is installed, which is how the app ships — so a
    /// learner who never installs one sees exactly the panel they always have,
    /// with no empty box and no prompt. See `asr.rs` for why the model is
    /// optional at all.
    pub heard: Option<Heard>,
    /// Why recognition failed, when a model *is* installed and could not be used.
    ///
    /// Carried beside the tone score rather than instead of it: the pitch was
    /// measured perfectly well, and losing that because a 228 MB file went
    /// missing would be the wrong way round.
    pub heard_error: Option<String>,
}

/// The tones a character or word should be practised with, or `None` when there
/// is nothing this can score.
///
/// Takes the **text** rather than one character, so that a word is scored as a
/// word — which is what makes tone sandhi work, since it happens between the
/// syllables of a word (你好 is spoken 2 + 3, not the dictionary's 3 + 3).
///
/// `None` covers text the dataset does not fully know, a reading that will not
/// divide into one syllable per character, more than a few syllables, and
/// anything with no tone that can be judged at all. The interface disables the
/// control on `None` rather than offering a recording it would then refuse.
///
/// The reading is resolved here rather than in the frontend so that the rules —
/// which diacritic means which tone, what counts as one syllable, when sandhi
/// applies — live in one place, next to the code that scores against them.
#[tauri::command]
pub fn tone_target(state: State<'_, AppState>, text: String) -> Option<ToneTarget> {
    state.tone_target(&text)
}

/// Whether the microphone can be used, and at what rate.
///
/// A denied permission cannot be seen from here — the operating system still
/// hands out a device, and the stream then delivers silence — so this reports
/// what it found and says what silence would mean.
#[tauri::command]
pub fn microphone_status(state: State<'_, AppState>) -> MicrophoneStatus {
    state.capture.status()
}

/// The window's system bar insets, in CSS pixels.
///
/// The page cannot measure these for itself. `env(safe-area-inset-*)` in an
/// Android WebView reports the **display cutout**, not the status bar, so on a
/// device with a notch or punch-hole it happens to be right and on one with a
/// plain bezel — an emulator, say — it is zero and the header is drawn
/// underneath the clock. Android knows the real answer, so this asks it.
/// Everywhere else the answer is zero, which leaves the CSS `env()` values in
/// charge on iOS exactly as before.
#[derive(Debug, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Insets {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

#[tauri::command]
pub fn android_insets() -> Result<Insets, String> {
    #[cfg(target_os = "android")]
    {
        crate::platform::call("insets", ())
    }
    #[cfg(not(target_os = "android"))]
    {
        Ok(Insets {
            top: 0.0,
            bottom: 0.0,
            left: 0.0,
            right: 0.0,
        })
    }
}

/// What the platform's speech system is actually doing.
///
/// Only Android fills this in, and every field is something that has caused
/// silence on a real phone: no engine, an engine with no Chinese voice, a voice
/// whose data was never downloaded, a network voice with no network. Asking the
/// synthesiser directly is the only way to tell those apart — from the outside
/// they all look like "the button did nothing".
#[derive(Debug, Default, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechReport {
    /// The voice now in use, by the platform's own name.
    pub engine: String,
    /// Every synthesiser installed, and its label.
    pub engines: String,
    /// What the system considers the default synthesiser.
    pub default_engine: String,
    /// The locale of the voice now in use, as a BCP-47 tag.
    pub locale: String,
    /// Whether that voice needs a network connection to speak.
    pub network_required: bool,
    /// How many Chinese voices the engine offers.
    pub chinese_voices: usize,
    /// How many of those can actually be spoken with.
    ///
    /// Android's engine advertises voices whose data has never been downloaded
    /// and marks them as not installed; they can be selected and then produce
    /// either an error or nothing at all. This is the count that matters, and it
    /// is not the same as `chinese_voices`.
    pub chinese_installed: usize,
    /// Each Chinese voice as the engine describes it — name, locale, whether it
    /// needs a network, whether its data is installed — in one line, because its
    /// whole purpose is to be read in a log.
    pub chinese_list: String,
    /// Whether the engine can speak Mandarin at all, in words.
    pub chinese_available: String,
    /// Why the last utterance failed, or empty when it did not.
    pub last_problem: String,
}

#[tauri::command]
pub fn speech_report() -> Result<SpeechReport, String> {
    #[cfg(target_os = "android")]
    {
        crate::platform::call("speechReport", ())
    }
    #[cfg(not(target_os = "android"))]
    {
        // Every other platform's speech is reachable from Rust directly, so
        // there is nothing hidden to report.
        Ok(SpeechReport::default())
    }
}

/// Begin listening. Resolves once the device is actually open, so a failure is
/// reported to the caller rather than swallowed in the audio thread.
#[tauri::command]
pub fn listen_start(state: State<'_, AppState>) -> Result<(), String> {
    state.capture.start()
}

/// Stop listening and score what was heard against the tones of `text`.
///
/// The text is what was on screen while the learner spoke, and the tones are
/// derived from it here rather than being sent by the frontend: that keeps one
/// source of truth, and it means the recording cannot be scored against a
/// sequence the interface made up.
///
/// The recorder is stopped **before** the text is resolved. If the text turned
/// out not to be scorable, returning early with the microphone still open would
/// leave it recording until the app was quit.
#[tauri::command]
pub fn listen_stop(state: State<'_, AppState>, text: String) -> Result<ToneResult, String> {
    let recording = state.capture.stop()?;
    match state.tone_target(&text) {
        Some(target) => Ok(state.score_tones(&recording, &target)),
        None => Err(format!(
            "There is no tone sequence to score {text} against. Practise a single character \
             or a short word."
        )),
    }
}

// ---- Speech recognition (ROADMAP.md M12) ----------------------------------

/// Whether a recognition model is installed, and how to describe one that is not.
///
/// Always answers, whatever the state: the settings screen has to be able to tell
/// a learner what *would* be downloaded — from where, how large, under which
/// licence — before they have agreed to any of it. This is also how the progress
/// of a download is read, which is why it takes no arguments and does no work.
#[tauri::command]
pub fn asr_status(state: State<'_, AppState>) -> AsrStatus {
    state.asr.status()
}

/// Fetch, verify and unpack the recognition model.
///
/// **The one thing in this app that touches the network, and only because a
/// learner pressed it.** Resolves as soon as the download has started rather than
/// when it finishes, so the interface is not held for the minutes a 163 MB
/// transfer takes; the outcome and the progress come back through
/// [`asr_status`], which the settings screen polls.
#[tauri::command]
pub fn asr_install(state: State<'_, AppState>) -> Result<(), String> {
    state.asr.install()
}

/// Delete the recognition model, and report the resulting state.
///
/// Returns the new status so the screen has one thing to render rather than two
/// calls that could disagree.
#[tauri::command]
pub fn asr_remove(state: State<'_, AppState>) -> Result<AsrStatus, String> {
    state.asr.remove()?;
    Ok(state.asr.status())
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

/// Change one or more settings.
///
/// Every argument is **optional, and absent means "leave this preference
/// alone"** — that is what lets the settings screen send only the control the
/// learner touched instead of resetting the other three on the way past.
///
/// Each preference that *can be un-chosen* spells its own clear, because a
/// missing argument already means "leave alone" and so cannot double as
/// "clear":
///
/// - `clickToDraw`: `true`/`false` to choose. Going back to the device's own
///   answer is its own command, [`clear_click_to_draw`], since `null` over the
///   wire is exactly what an omitted argument looks like.
/// - `voice`: a name to choose, `""` to go back to the automatic voice. A voice
///   is never legitimately nameless, so the empty string is free to mean this.
/// - `animationPace` / `boardSize`: closed sets, so every value is a choice.
///
/// The change is written straight away; the view that comes back is what the
/// interface should render, warning included.
#[tauri::command]
pub fn update_settings(
    state: State<'_, AppState>,
    click_to_draw: Option<bool>,
    voice: Option<String>,
    animation_pace: Option<Pace>,
    board_size: Option<BoardSize>,
) -> SettingsView {
    state.update_settings(
        click_to_draw,
        voice.as_deref(),
        animation_pace,
        board_size,
    )
}

/// Go back to the device's own answer for how a stroke is drawn.
///
/// A command of its own rather than a `null` argument: over the wire a missing
/// argument and a null one are indistinguishable, and for every *other*
/// preference absent has to keep meaning "leave it alone". The tri-state is what
/// makes an unchosen preference worth representing, so it gets an honest route.
#[tauri::command]
pub fn clear_click_to_draw(state: State<'_, AppState>) -> SettingsView {
    state.clear_click_to_draw()
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

// ---- cross-device sync ------------------------------------------------------
//
// Six commands, and the shape of the flow is a paste rather than a redirect: the
// app opens Dropbox's authorization page in the system browser, the learner copies
// the code it shows them, and pastes it back. That is forced by Dropbox refusing
// custom URL schemes, and it is why nothing here waits on a callback. See
// `crate::sync` for why the token lives in the Keychain and not in `hanzi.db`.
//
// **The ones that touch the network are `async`, and that is not decoration.** A
// plain `#[tauri::command]` runs on the main thread — the thread the webview draws
// on — so a sync declared that way freezes the window for as long as it takes, and
// the "Syncing…" line that is supposed to say so cannot be painted until the sync
// it describes is already over. The `async` attribute on a *synchronous* function
// is what moves it to Tauri's runtime with its signature unchanged, which is what
// `State` needs. Two of them can now overlap, which is why `SyncService` keeps a
// gate; see its note.

/// What the sync screen should be showing.
#[tauri::command]
pub fn sync_status(sync: State<'_, SyncService>) -> SyncView {
    sync.view()
}

/// Begin connecting an account, and open the authorization page.
///
/// Returns the URL as well as opening it, so that a learner whose browser did not
/// come forward can open it themselves rather than being stuck.
#[tauri::command]
pub fn sync_connect(
    app: tauri::AppHandle,
    sync: State<'_, SyncService>,
) -> Result<String, String> {
    let url = sync.begin()?;
    // In the system browser, never a webview: Dropbox asks for that, and Google's
    // policy forbids their sign-in flow inside one.
    //
    // Through the **plugin handle**, not `tauri_plugin_opener::open_url`. That free
    // function is a process spawn and desktop-only; on a phone it fails with
    // `operation not permitted (os error 1)`. `Opener::open_url` is the one with a
    // `#[cfg(mobile)]` twin that calls into the platform's own URL opener, and it
    // needs an `AppHandle` to reach — which is why the handle is a parameter here
    // even though nothing else in this command appears to use it.
    app.opener()
        .open_url(url.clone(), None::<&str>)
        .map_err(|e| format!("could not open a browser: {e}"))?;
    Ok(url)
}

/// Finish connecting, with the code the learner pasted.
#[tauri::command(async)]
pub fn sync_connect_finish(sync: State<'_, SyncService>, code: String) -> Result<SyncView, String> {
    sync.finish(&code)
}

/// Sync now.
///
/// Takes the app state as well as the sync service, and that is the whole reason
/// this command is more than a forward: a sync rebuilds schedules in the database,
/// while the open practice store still holds the document from before it. Its next
/// `save` — which every review performs — would write those stale cards back over
/// the synced ones. So the store is reloaded, on failure as well as success,
/// because a sync that died partway through `recompute` may still have written
/// some of them.
#[tauri::command(async)]
pub fn sync_now(
    state: State<'_, AppState>,
    sync: State<'_, SyncService>,
) -> Result<SyncView, String> {
    let outcome = sync.now();
    // Every store the sync can have rewritten, not just the schedule. See
    // `AppState::reload_after_sync` for why forgetting one of these is not merely a
    // stale screen.
    state.reload_after_sync();
    outcome
}

/// Forget the account, here and on Dropbox's side.
#[tauri::command(async)]
pub fn sync_disconnect(sync: State<'_, SyncService>) -> SyncView {
    sync.disconnect()
}

/// Sync because the app started or came back, rather than because somebody pressed
/// a button.
///
/// Nothing here is a failure the learner has to act on: most launches are not
/// connected to anything, and a phone on a train has no network. `AutoSync` keeps
/// those apart so the screen can be silent about the first two and say something
/// about the others — see `crate::sync`.
#[tauri::command(async)]
pub fn sync_auto(state: State<'_, AppState>, sync: State<'_, SyncService>) -> AutoSync {
    let outcome = sync.auto();
    // Skipping the reload when nothing was attempted is the only case that is safe
    // to skip, and it is worth skipping: `reload_after_sync` re-reads three stores,
    // and a launch with no network would otherwise pay for all three to learn that
    // nothing had changed. A *failed* pass still reloads, for the reason `sync_now`
    // gives — it may have written some schedules before it died.
    if !matches!(outcome, AutoSync::Skipped { .. } | AutoSync::Offline { .. }) {
        state.reload_after_sync();
    }
    outcome
}

/// Ask for a fingerprint before the sign-in is used, or stop asking for one.
///
/// Nothing is read to answer this: the preference is recorded, and the stored item
/// is rewritten only when there is one — which is the one moment turning the prompt
/// *off* has to read it. See `crate::sync` for why asking for nothing is the default
/// and why drawing the screen never unlocks anything.
#[tauri::command]
pub fn sync_set_lock(sync: State<'_, SyncService>, locked: bool) -> Result<SyncView, String> {
    sync.set_lock(locked)
}
