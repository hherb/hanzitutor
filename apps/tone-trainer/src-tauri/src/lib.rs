//! Tone Trainer — practise the tones of Mandarin on their own.
//!
//! Hanzi Tutor teaches characters and scores the tone you said them with. This is
//! that one idea on its own: the characters that differ *only* in tone, heard and
//! then said, with the pitch you produced drawn against the shape the tone asks
//! for. Nothing is written, nothing is graded on shape, and there is no course —
//! so a learner working on tone can open this, drill for two minutes and close it.
//!
//! ## What is shared and what is not
//!
//! Almost all of the hard part is shared, and deliberately:
//!
//! - the **tone scorer** — pitch tracking, the five-level-scale templates, the
//!   decision rules — is `hanzi_core::tone`, unchanged and untouched;
//! - the **minimal pairs** are `hanzi_core::Dataset::tone_sets`, the same
//!   derivation the full app's Tones screen reads, so the two cannot disagree
//!   about which characters form a set;
//! - the **microphone** and the **system voice** are `hanzi-voice`, which exists
//!   so that this app did not have to copy the platform backends.
//!
//! What is *not* shared is the interface, and that is a real limit rather than a
//! preference: the full app's tone panel draws a score for a character on a
//! practice board, with the board's characters and the recognition hint beside
//! it. This app has no board, so it has its own screen over the same data. See
//! `src/App.svelte` for the shape of it and `HANDOVER.md` for what was lifted.
//!
//! ## The dataset is embedded
//!
//! Same artifact as the full app (`crates/hanzi-core/data/hanzi.bin.gz`), read
//! from the workspace rather than copied, so the two apps cannot ship different
//! characters. It is far more than tone pairs need — a dedicated build could
//! carry a few hundred sets instead of 7,744 characters — but it is already
//! built, already licensed and already tested, and a second data pipeline is a
//! worse cost than 13 MB in a bundle. `HANDOVER.md` records that trade.

use std::sync::Arc;

use hanzi_core::pinyin::{heard_against_readings, tone_target as build_tone_target};
use hanzi_core::tone::analyze;
use hanzi_core::{Dataset, Heard, ToneSet};
use hanzi_hearing::{Asr, AsrStatus};
use hanzi_voice::{MicrophoneStatus, Recorder, Recording, Speaker};
use serde::Serialize;
use tauri::State;

mod platform;

/// The compact artifact produced by `hanzi-core`'s `prepare-data` binary.
///
/// Read through the workspace rather than copied into this crate: both apps must
/// see the same characters, and a copy is a thing that can drift.
const ARTIFACT: &[u8] = include_bytes!("../../../../crates/hanzi-core/data/hanzi.bin.gz");

/// How many sets one call returns.
///
/// The full app's screen shows every set — a few hundred — because it filters
/// what it holds. This app drills rather than browses, so it asks for a working
/// list and the ranking (`tone_sets` orders by the rarest member, so the most
/// useful contrasts come first) decides which. A cap rather than a page: the
/// caller can raise it if it wants more.
const DEFAULT_SETS: usize = 400;

/// How many word families one call returns.
///
/// Smaller than [`DEFAULT_SETS`] because each row is a *family* of words rather
/// than one syllable, and a learner working through tone pairs wants the common
/// words first. The ranking does that: words arrive most useful first, so a cap
/// keeps the common ones and drops the rest.
const DEFAULT_WORD_SETS: usize = 150;

/// Longest word this app will score.
///
/// 你好 and 中国人 are the interesting cases; a four-character idiom is not, because
/// its tone pattern stops being something a learner drills. Four is also what the
/// full app stops at, and for the same reason: past it the syllable boundaries of
/// a recording cannot be found reliably from energy alone.
const MAX_WORD_SYLLABLES: usize = 4;

/// Longest recording, re-exported to the interface so the warning it prints and
/// the buffer the recorder keeps cannot disagree.
///
/// `hanzi_voice::MAX_RECORD_SECS` is the number; this only gives the frontend a
/// way to read it without hard-coding a second copy of it.
#[tauri::command]
fn max_record_secs() -> u32 {
    hanzi_voice::MAX_RECORD_SECS
}

/// One syllable's worth of a scored attempt, in the shape the interface reads.
///
/// Mirrors the full app's `ToneSyllableResult` field for field, so a word reads
/// the same in both apps and the chart needs no adaptation. `citation` and
/// `spoken` differ only when **tone sandhi** moved the tone — 你好 is written
/// tone 3 + tone 3 and spoken 2 + 3 — and the interface says so when they do.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyllableResult {
    /// Which syllable this is, counting from 1, for the wording.
    pub position: usize,
    /// The character that was asked for.
    pub ch: char,
    /// The reading as the dictionary writes it, tone mark included, e.g. `"nǐ"`.
    pub reading: String,
    /// The tone the dictionary gives this syllable, before sandhi.
    pub citation: u8,
    /// The tone actually spoken in this word, which is what was scored.
    pub spoken: u8,
    pub attempt: hanzi_core::ToneAttempt,
}

/// One word of a word family: a whole word, with the reading that is scored.
///
/// The reading is the **whole word's**, from the dataset's own dictionary entry,
/// which is what resolves a polyphone — 着急 is `zháojí`, where the isolated 着 has
/// no context to pick a reading from. Composing it from the characters would get
/// that wrong, and a wrong reading is a wrong tone to score against.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordMember {
    pub text: String,
    /// The reading with tone marks, e.g. `"nǐhǎo"`.
    pub reading: String,
    /// The tones **as spoken**, after sandhi. This is what a recording is scored
    /// against, and it is not always what a dictionary prints.
    pub spoken: Vec<u8>,
    /// The tones the dictionary gives, before sandhi.
    pub citation: Vec<u8>,
    pub meaning: String,
    /// Lowest HSK 3.0 level the word appears in, `1..=7`.
    pub hsk: u8,
    /// Estimated frequency: the rank of the word's rarest character. For ordering.
    pub rank: u32,
}

/// A word family: the words sharing one character from a tone set.
///
/// **Why families rather than a flat word list.** The drill this app exists for is
/// tone contrast, so the words worth offering are the ones built on the characters
/// that already differ only in tone: 妈 gives 妈妈, 麻烦, 干妈. Grouping them under
/// that character makes the connection visible, and keeps one screen able to show
/// both the contrast and the words that use it.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordSet {
    /// The character the family hangs on, which is a member of a tone set.
    pub key: char,
    /// The syllable the family is **about**: the tone set's own base, e.g.
    /// `"xin"`. Carried so the interface can search a family on what it *is* the
    /// way it searches a tone set on `ToneSet::base`, instead of only on the
    /// words inside it — which matched almost anything.
    pub base: String,
    /// That character's own readings, one per tone it has a minimal pair for, so
    /// the header can say which contrast this family is about.
    pub contrast: String,
    /// The words, most useful first.
    pub words: Vec<WordMember>,
}

/// A scored attempt, as the interface reads it.
///
/// The same field names as the full app's `ToneResult` on purpose, so the tone
/// chart lifted from it (`src/lib/ToneChart.svelte`) needs no adaptation and the
/// two cannot drift into two shapes for one judgement.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreResult {
    /// One entry per syllable of what was asked for, in order — one for a
    /// character, several for a word.
    pub syllables: Vec<SyllableResult>,
    /// True when the pitch was measured, so `syllables` holds a judgement per
    /// syllable. False when the text was longer than this app scores, in which
    /// case no tone was judged and the interface must not read a score of zero
    /// from it.
    pub tone_scored: bool,
    pub verdict: hanzi_core::ToneVerdict,
    pub score: f32,
    pub grade: hanzi_core::Grade,
    /// One plain sentence for the learner, worded by the scorer.
    pub detail: String,
    pub sandhi_applied: bool,
    pub boundaries_ms: Vec<u32>,
    pub voiced_ms: u32,
    pub span_ms: u32,
    pub median_hz: f32,
    /// What a recognition model heard, when one is installed.
    ///
    /// `None` whenever no model is installed, which is how the app ships — so a
    /// learner who never installs one sees exactly the screen they always had,
    /// with no empty box and no prompt. Same shape as the full app's, so the
    /// recognition block is the same judgement in both.
    pub heard: Option<Heard>,
    /// Why recognition failed, when a model *is* installed and could not be used.
    /// Carried beside the tone score rather than instead of it.
    pub heard_error: Option<String>,
}

/// What this app holds while it runs.
///
/// The dataset is decoded once at startup, which is the slow part (13 MB of
/// gzip), so it is built before Tauri exists — the same race the full app
/// documents in its `AppState::prepare`: on mobile the webview starts loading
/// while `setup` runs, and a command that arrives before `manage` is rejected
/// rather than made to wait.
pub struct Trainer {
    dataset: Dataset,
    recorder: Recorder,
    speaker: Arc<Speaker>,
    /// The optional recogniser. Holds nothing and opens nothing until the learner
    /// installs the model — see `hanzi-hearing`.
    asr: Asr,
}

/// Everything except the dataset, which cannot be read before Tauri exists
/// without being handed across — see [`Trainer::prepare`] and [`Trainer::assemble`].
pub struct Prepared {
    dataset: Dataset,
}

impl Trainer {
    /// Decode the embedded dataset. Touches nothing but memory.
    pub fn prepare() -> Result<Prepared, String> {
        let dataset = Dataset::from_gzip_bytes(ARTIFACT)
            .map_err(|e| format!("could not read the embedded character dataset: {e}"))?;
        Ok(Prepared { dataset })
    }

    /// Finish construction, which is cheap.
    ///
    /// The one thing that is *not* cheap is warming pronunciation, and it is
    /// deliberately not done here: it starts a thread and returns. The full app
    /// does the same thing for the same reason (`warm_voice` in its `state.rs`) —
    /// resolving the voice list costs about a second, and the first utterance of a
    /// session used to pay it, which on macOS also meant a cold audio device and a
    /// rough-sounding first word or two.
    ///
    /// `data_dir` is where the optional recognition model goes. `None` when the
    /// platform would not give one, in which case the settings say there is
    /// nowhere to keep a 163 MB download rather than failing at the moment the
    /// learner asks for it.
    pub fn assemble(prepared: Prepared, data_dir: Option<&std::path::Path>) -> Self {
        let Prepared { dataset } = prepared;
        let speaker = Arc::new(Speaker::default());
        warm(Arc::clone(&speaker));
        Self {
            dataset,
            recorder: Recorder::default(),
            speaker,
            asr: Asr::new(data_dir),
        }
    }

    /// The characters that differ only in tone, most useful contrast first.
    ///
    /// Public for the same reason [`Self::score`] is: it is the app's whole
    /// dataset answer, and `tests/tone_sets.rs` checks the derivation through it
    /// rather than trusting that a moved crate still lines up.
    pub fn tone_sets(&self, limit: usize) -> Vec<ToneSet> {
        self.dataset.tone_sets(limit)
    }

    /// Word families built on the characters that differ only in tone.
    ///
    /// ## Why this derivation, and not "all HSK words"
    ///
    /// The exercise is tone contrast. A learner who has just drilled 妈/麻/马/骂 wants
    /// the *words* that use those characters, because that is where the contrast
    /// pays off — and the tones inside a word are not the tones of the characters
    /// said one at a time: sandhi moves them. So the words are hung on the same
    /// derived tone sets the rest of the app shows, which means the two screens
    /// cannot disagree about which characters are a contrast.
    ///
    /// A family is keyed by one character and holds every HSK word containing it
    /// that this app can score: two to [`MAX_WORD_SYLLABLES`] syllables. Words
    /// outside that range are dropped rather than padded, because a tone cannot be
    /// scored past the syllable count whose boundaries the analyser can find.
    ///
    /// Ordered so the most useful comes first: by the rarest character's rank,
    /// which is the same estimate the tone sets are ranked by.
    pub fn word_sets(&self, limit: usize) -> Vec<WordSet> {
        // The characters worth building families on, most useful contrast first.
        let contrasts = self.dataset.tone_sets(limit);
        let mut seen_word = std::collections::HashSet::new();
        let mut sets = Vec::new();

        for set in &contrasts {
            // The key is the *first* member's character — the commonest reading of
            // the syllable, since `tone_sets` orders members by tone and fills each
            // slot with the most frequent character for it. Any member would do;
            // this one is stable.
            let Some(key) = set.members.first().map(|member| member.ch) else {
                continue;
            };

            let contrast = set
                .members
                .iter()
                .map(|member| format!("{} tone {}", member.ch, member.tone))
                .collect::<Vec<_>>()
                .join(" · ");

            let mut words: Vec<WordMember> = Vec::new();
            for word in self.dataset.words_with(key) {
                let characters: Vec<char> = word.text.chars().collect();
                if characters.len() < 2 || characters.len() > MAX_WORD_SYLLABLES {
                    continue;
                }
                // A word appears under every family character it contains; the
                // first family to claim it keeps it, so a word is not drilled twice
                // under two headers.
                if !seen_word.insert(word.text.clone()) {
                    continue;
                }
                // The whole-word reading, which is what resolves a polyphone. A
                // reading that will not divide one syllable per character is
                // dropped rather than scored out of step.
                let Some(target) = build_tone_target(&word.text, &word.pinyin) else {
                    continue;
                };
                if target.syllables.len() != characters.len() {
                    continue;
                }
                words.push(WordMember {
                    text: word.text.clone(),
                    reading: word.pinyin.clone(),
                    spoken: target.spoken(),
                    citation: target.syllables.iter().map(|s| s.citation).collect(),
                    meaning: word.meaning.clone(),
                    hsk: word.hsk,
                    rank: word.rank,
                });
            }

            if words.is_empty() {
                continue;
            }
            words.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.text.cmp(&b.text)));
            sets.push(WordSet {
                key,
                base: set.base.clone(),
                contrast,
                words,
            });
            if sets.len() >= limit {
                break;
            }
        }

        sets
    }

    /// Judge one recording against the tones the learner was aiming for.
    ///
    /// **One path for a character and for a word**, because they are the same
    /// measurement. The only difference is how the tones were arrived at: a
    /// character's drill picks one tone the learner chose, while a word's tones
    /// come from the dictionary and from **sandhi** — 你好 is written tone 3 + tone 3
    /// and spoken 2 + 3, so scoring it against the dictionary would mark correct
    /// speech wrong. `build_tone_target` is the module that knows those rules, and
    /// this app uses it rather than re-deriving them.
    ///
    /// `reading` is the whole word's reading (or a single character's). The
    /// syllables scored are `ch` and `reading` split together, so the two cannot
    /// disagree about how many syllables there are.
    ///
    /// **Recognition is folded in when the model is installed**, exactly as the full
    /// app folds it in. It answers the one question the pitch cannot: a tone 4 said
    /// as 骂 and a tone 4 said as 四 score identically, because the contour is the
    /// same — the learner's *syllable* was never checked. `heard_against_readings`
    /// compares sounds with the tone set aside, so a homophone counts as the sound
    /// asked for, which is the honest reading: within one syllable a character
    /// carries no information beyond its reading.
    ///
    /// With no model installed `heard` is `None` and nothing changes — the tone
    /// half is complete on its own, and recognition is the only part that can be
    /// absent.
    ///
    /// Public because it is the whole of this app's judgement and the thing an
    /// integration test can drive with a synthetic contour — see
    /// `tests/tone_sets.rs`. The commands above it only add the microphone.
    pub fn score(&self, recording: &Recording, text: &str, reading: &str) -> ScoreResult {
        let Some(target) = build_tone_target(text, reading) else {
            // The reading will not divide into one syllable per character, so
            // there is no per-syllable goal to score against. Reported as
            // *unmeasured* rather than scored zero, which would read as a
            // perfectly flat attempt.
            return ScoreResult::unmeasured(text);
        };

        let spoken = target.spoken();
        let report = analyze(&recording.samples, recording.sample_rate, &spoken);

        let mut detail = report.detail.clone();
        if target.sandhi_applied {
            detail.push(' ');
            detail.push_str(&target.detail);
        }
        if recording.truncated {
            detail.push_str(&format!(
                " (The recording hit the {}-second limit, so only the start was judged.)",
                hanzi_voice::MAX_RECORD_SECS
            ));
        }

        // One entry per syllable asked for, paired with the judgement of it. Zip
        // degrades to the shorter side, which cannot happen — `analyze` answers one
        // entry per tone — but a length mismatch must not panic in a command.
        let syllables: Vec<SyllableResult> = target
            .syllables
            .iter()
            .zip(report.syllables.iter())
            .map(|(goal, scored)| SyllableResult {
                position: scored.position,
                ch: goal.ch,
                reading: goal.reading.clone(),
                citation: goal.citation,
                spoken: goal.spoken,
                attempt: hanzi_core::ToneAttempt {
                    // The one field the scorer could not have known: the truncation
                    // is a fact about the capture, not about the pitch.
                    detail: detail.clone(),
                    ..scored.attempt.clone()
                },
            })
            .collect();

        // What was said, as opposed to how. `None` with no model installed — not a
        // failure, and not offered as one — and an error only when a model *is*
        // installed and could not be used, in which case the tone score still stands.
        // The comparison is against the target's own readings, one per syllable.
        let wanted: Vec<String> = target
            .syllables
            .iter()
            .map(|syllable| syllable.reading.clone())
            .collect();
        let (heard, heard_error) = self.hear(recording, &wanted);

        ScoreResult {
            syllables,
            tone_scored: true,
            verdict: report.verdict,
            score: report.score,
            grade: report.grade,
            detail,
            sandhi_applied: target.sandhi_applied,
            boundaries_ms: report.boundaries_ms,
            voiced_ms: report.voiced_ms,
            span_ms: report.span_ms,
            median_hz: report.median_hz,
            heard,
            heard_error,
        }
    }

    /// Read a recording as syllables, when a recognition model is installed.
    ///
    /// The transcript's own reading comes from the dataset, the way the full app
    /// resolves it: the **whole-word** entry when there is one, because that is
    /// what picks the right reading for a polyphone, and the character's own
    /// otherwise. A transcript whose reading will not divide one syllable per
    /// character is compared against nothing rather than out of step —
    /// `heard_against_readings` refuses it and says so.
    ///
    /// `wanted` is one reading per syllable the drill was showing. **The base
    /// sound is what is compared, with the tone set aside**, which is what makes
    /// sandhi harmless here: 你好's wanted readings carry the spoken tones and the
    /// comparison ignores them either way.
    fn hear(&self, recording: &Recording, wanted: &[String]) -> (Option<Heard>, Option<String>) {
        let text = match self.asr.recognize(&recording.samples, recording.sample_rate) {
            Ok(Some(text)) => text,
            // No model installed, or the model heard nothing it could write down.
            // `None` both times: the first is not a failure and the second is an
            // answer, and neither is this app's business to report as a fault.
            Ok(None) => return (None, None),
            Err(problem) => return (None, Some(problem)),
        };

        let reading = self.dataset.lookup_text(&text).pinyin;
        (
            Some(heard_against_readings(&text, &reading, wanted, true)),
            None,
        )
    }
}

impl ScoreResult {
    /// An attempt that could not be measured, with the reason in `detail`.
    ///
    /// `toneScored: false` is the whole point: a score of zero would read as a
    /// perfectly flat attempt rather than an unmeasured one, and the interface
    /// reads that flag to decide whether to draw a chart at all.
    fn unmeasured(text: &str) -> Self {
        let characters: Vec<char> = text.chars().collect();
        Self {
            syllables: Vec::new(),
            tone_scored: false,
            verdict: hanzi_core::ToneVerdict::Uncertain,
            score: 0.0,
            grade: hanzi_core::Grade::Poor,
            detail: format!(
                "The reading for {text} could not be lined up one syllable per character, \
                 so no tone was judged. ({} characters.)",
                characters.len()
            ),
            sandhi_applied: false,
            boundaries_ms: Vec::new(),
            voiced_ms: 0,
            span_ms: 0,
            median_hz: 0.0,
            heard: None,
            heard_error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// The commands the interface calls
// ---------------------------------------------------------------------------

/// Resolve the voices and wake the audio device, off the startup path.
///
/// A thread of its own, because resolving the voice list takes about a second on
/// macOS and the first screen must not wait for it. Nothing reports a failure: the
/// worst case is that the first utterance pays the cost this exists to move, which
/// is what happened before. `Speaker::prime` is a no-op on the platforms that have
/// nothing to warm, so this is not gated by platform here.
///
/// Behind an `Arc` because the speaker is shared with the commands, which need it
/// for the life of the process — a reference into a value Tauri owns would be a
/// borrowed pointer with no guarantee attached to it.
fn warm(speaker: Arc<Speaker>) {
    std::thread::spawn(move || speaker.prime());
}

/// Characters that differ only in tone, most useful first.
///
/// The whole derived list in one call, the way the full app's Tones screen reads
/// it — a few hundred sets is small, and the drill re-ranks and filters what it
/// holds rather than asking again for every change.
#[tauri::command]
fn tone_sets(trainer: State<'_, Trainer>, limit: Option<usize>) -> Vec<ToneSet> {
    trainer.tone_sets(limit.unwrap_or(DEFAULT_SETS))
}

/// Word families built on the characters that differ only in tone.
///
/// The word half of the drill. The whole derived list in one call, the way the tone
/// sets come — a few hundred families is small, and the screen filters what it
/// holds rather than asking again for every change.
#[tauri::command]
fn word_sets(trainer: State<'_, Trainer>, limit: Option<usize>) -> Vec<WordSet> {
    trainer.word_sets(limit.unwrap_or(DEFAULT_WORD_SETS))
}

/// Whether the microphone can be used, and at what rate.
///
/// A denied permission cannot be seen from here — the system still hands out a
/// device and the stream then delivers silence — so this reports what it found
/// and the interface says what silence would mean.
#[tauri::command]
fn microphone_status(trainer: State<'_, Trainer>) -> MicrophoneStatus {
    trainer.recorder.status()
}

/// Begin listening. Resolves once the device is actually open, so a failure to
/// open it arrives at the caller rather than as silence.
#[tauri::command]
fn listen_start(trainer: State<'_, Trainer>) -> Result<(), String> {
    trainer.recorder.start()
}

/// Stop listening and judge what was heard against what the drill was showing.
///
/// `text` and `reading` are what the learner was looking at while they spoke — a
/// character with the one tone they chose, or a whole word with the tones the
/// dictionary and sandhi give it. Both are passed in rather than looked up here so
/// that the judgement cannot be made against something the interface invented, and
/// this app has no board whose text could be read instead.
///
/// The **reading is resolved by the caller** because for a character drill the tone
/// was chosen by the learner and appears in no dictionary. For a word the caller
/// passes the dataset's own whole-word reading, which is what resolves a polyphone.
#[tauri::command]
fn listen_stop(
    trainer: State<'_, Trainer>,
    text: String,
    reading: String,
) -> Result<ScoreResult, String> {
    let recording = trainer.recorder.stop()?;
    Ok(trainer.score(&recording, &text, &reading))
}

/// Stop listening and throw the recording away.
///
/// The microphone is open only while the button is held, so a learner who leaves
/// the drill mid-press would otherwise leave it open with no pointer-up left to
/// close it — and the system's recording indicator lit. Stopping is all this does:
/// there is no longer a drill to judge the audio against, and a result posted onto
/// a screen the learner has left is worse than a recording discarded.
#[tauri::command]
fn listen_cancel(trainer: State<'_, Trainer>) {
    // A failure means nothing was open, which is what this wanted anyway.
    let _ = trainer.recorder.stop();
}

/// Start pronouncing a character, cutting off anything already being said.
///
/// Resolves as soon as the synthesiser has started, not when the audio finishes.
#[tauri::command]
fn speak(trainer: State<'_, Trainer>, text: String) -> Result<(), String> {
    trainer.speaker.speak(&text)
}

/// Stop the current utterance.
#[tauri::command]
fn stop_speaking(trainer: State<'_, Trainer>) {
    trainer.speaker.stop();
}

/// The voice pronunciation will use, or `null` when the system has no Chinese
/// voice installed.
///
/// A description rather than the voice itself, which is all the header shows:
/// the full app's settings screen lists and chooses voices, and this app
/// deliberately does not — one voice, the one the system picks.
#[tauri::command]
fn voice(trainer: State<'_, Trainer>) -> Option<String> {
    trainer.speaker.status()
}

/// Echo a line to the Rust process's stderr, for diagnosing a blank window.
#[tauri::command]
fn webview_log(message: String) {
    eprintln!("[webview] {message}");
}

// ---- Speech recognition: the optional model ---------------------------------
//
// The same three calls the full app exposes, over the same shared crate, so the
// two screens describe one download identically. What differs is only where the
// model is kept: this app has no study database, so its data directory holds the
// model and nothing else.

/// Whether a recognition model is installed, and how to describe one that is not.
///
/// Always answers, whatever the state: the screen has to be able to tell a learner
/// what *would* be downloaded — from where, how large, under which licence —
/// before they have agreed to any of it. This is also how the progress of a
/// download is read, which is why it takes no arguments and does no work.
#[tauri::command]
fn asr_status(trainer: State<'_, Trainer>) -> AsrStatus {
    trainer.asr.status()
}

/// Fetch, verify and unpack the recognition model.
///
/// **The one thing in this app that touches the network, and only because a
/// learner pressed it.** Resolves as soon as the download has started rather than
/// when it finishes, so the interface is not held for the minutes a 163 MB
/// transfer takes; the outcome and the progress come back through [`asr_status`],
/// which the screen polls.
#[tauri::command]
fn asr_install(trainer: State<'_, Trainer>) -> Result<(), String> {
    trainer.asr.install()
}

/// Delete the recognition model, and report the resulting state.
///
/// Returns the new status so the screen has one thing to render rather than two
/// calls that could disagree.
#[tauri::command]
fn asr_remove(trainer: State<'_, Trainer>) -> Result<AsrStatus, String> {
    trainer.asr.remove()?;
    Ok(trainer.asr.status())
}

/// The app's name and version, for the footer.
#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "Tone Trainer".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        licence: "AGPL-3.0-only".into(),
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub licence: String,
}

/// The dataset, decoded once, before Tauri exists.
///
/// A mutex rather than a `OnceLock` because the setup closure has to *take* the
/// decoded dataset out and hand it to `manage`, and `OnceLock::take` wants a
/// mutable reference a static cannot give. Written once, read once, and the lock
/// is held only for the move.
static PREPARED: std::sync::Mutex<Option<Prepared>> = std::sync::Mutex::new(None);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let prepared = match Trainer::prepare() {
        Ok(prepared) => prepared,
        Err(message) => {
            // The dataset is embedded, so this means the build itself is broken:
            // there is nothing to fall back to and no window worth opening.
            eprintln!("error: {message}");
            std::process::exit(2);
        }
    };
    *PREPARED.lock().unwrap_or_else(|e| e.into_inner()) = Some(prepared);

    tauri::Builder::default()
        .plugin(platform::init())
        .setup(move |app| {
            use tauri::{Manager, path::BaseDirectory};
            let prepared = PREPARED
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take()
                .expect("the dataset is prepared before the builder runs");

            // Where the optional recognition model goes. This app has no study
            // database, so this directory holds the model and nothing else — and
            // it is the platform's own per-app location, so the trainer's model
            // and the full app's are two downloads that cannot be mistaken for
            // one another.
            //
            // A platform that will not give one is not fatal: the settings then
            // say there is nowhere to keep a 163 MB download, rather than failing
            // at the moment the learner asks for it.
            let data_dir = match app.path().resolve("", BaseDirectory::AppData) {
                Ok(dir) => {
                    eprintln!("[data] model directory {}", dir.display());
                    Some(dir)
                }
                Err(problem) => {
                    eprintln!(
                        "[data] no application data directory ({problem}); the recognition \
                         model cannot be installed this session"
                    );
                    None
                }
            };

            app.manage(Trainer::assemble(prepared, data_dir.as_deref()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tone_sets,
            word_sets,
            microphone_status,
            listen_start,
            listen_stop,
            listen_cancel,
            speak,
            stop_speaking,
            voice,
            max_record_secs,
            asr_status,
            asr_install,
            asr_remove,
            app_info,
            webview_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tone Trainer");
}
