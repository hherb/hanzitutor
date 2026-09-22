/** Thin typed wrappers over the Tauri commands. */

import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  AttemptMeasures,
  AutoSync,
  AsrStatus,
  Character,
  CharacterSearchView,
  CursorView,
  DatasetStats,
  GradeOptions,
  GradeReport,
  Lesson,
  LicenceNotice,
  MarkedTone,
  Point,
  MicrophoneStatus,
  ProgressView,
  ReviewView,
  SettingsPatch,
  SayStatus,
  SettingsView,
  SpokenAudio,
  StartupView,
  SpeechTarget,
  SyncView,
  ToneResult,
  ToneSet,
  TextLookup,
  VocabOutcome,
  VocabView,
  VoicesView,
  WordSearchView,
} from "./types";

export const datasetStats = () => invoke<DatasetStats>("dataset_stats");

/**
 * The window's system bar insets, in CSS pixels.
 *
 * Only Android answers with anything but zero: there the webview's
 * `env(safe-area-inset-*)` reports the display cutout rather than the status
 * bar, so the shell asks the platform for the real numbers and publishes them
 * as the `--inset-*` custom properties that `--safe-top` / `--safe-bottom` in
 * `app.css` take the larger of.
 */
export const androidInsets = () =>
  invoke<{ top: number; bottom: number; left: number; right: number }>("android_insets");

/**
 * What the platform's speech system is doing, for when it is doing nothing.
 *
 * Android answers; every other platform returns empty fields, because there the
 * synthesiser is reachable from Rust directly and there is nothing hidden to
 * ask about.
 */
export const speechReport = () =>
  invoke<{
    engine: string;
    engines: string;
    defaultEngine: string;
    locale: string;
    networkRequired: boolean;
    chineseVoices: number;
    /** How many of those can actually be spoken with, offline. */
    chineseInstalled: number;
    chineseAvailable: string;
    lastProblem: string;
  }>("speech_report");

export const listLessons = () => invoke<Lesson[]>("lessons");

export const getCharacter = (ch: string) => invoke<Character>("character", { ch });

/**
 * Every character the board can ask for.
 *
 * Used to tell a word from the punctuation around it: a sentence written out
 * one character at a time should skip the marks it cannot draw, not dead-end on
 * them. Sent once and held.
 */
export const teachableCharacters = () => invoke<string[]>("teachable_characters");

// ---- the word dictionary --------------------------------------------------

/**
 * Search the HSK word list.
 *
 * An empty `query` browses from the most useful word down; a single character
 * lists every word containing it; readings match with or without tone marks
 * (`xuexi`, `xuéxí`) and meanings match in English. `level` narrows to one HSK
 * level. The result is one capped page plus the true total.
 */
export const searchWords = (query: string, level: number | null, limit?: number) =>
  invoke<WordSearchView>("search_words", { query, level, limit: limit ?? null });

// ---- the character dictionary ---------------------------------------------

/**
 * Search the character set.
 *
 * An empty `query` browses the course from the most common character down;
 * readings match with or without tone marks (`xue`, `xué`) and definitions match
 * in English. What was typed as several characters, or as several syllables, is
 * looked up as its parts — `医院` and `yisheng` both reach 医 and 生. `level`
 * narrows to one HSK level. The result is one capped page plus the true total.
 */
export const searchCharacters = (query: string, level: number | null, limit?: number) =>
  invoke<CharacterSearchView>("search_characters", { query, level, limit: limit ?? null });

// ---- tone pairs -----------------------------------------------------------

/**
 * Characters that differ only in tone, most useful first.
 *
 * The whole derived list in one call — a few hundred sets — because the screen
 * filters what it holds rather than asking again for each filter. `limit` is a
 * cap, not a request for more.
 */
export const toneSets = (limit?: number) =>
  invoke<ToneSet[]>("tone_sets", { limit: limit ?? null });

/**
 * Grade an attempt. `strokes` are in display space (0..=1024, y downwards), in
 * the order they were drawn.
 */
export const gradeAttempt = (ch: string, strokes: Point[][], options: GradeOptions) =>
  invoke<GradeReport>("grade_attempt", { ch, strokes, options });

/**
 * Start pronouncing a character. Resolves as soon as the synthesiser has
 * started, not when the audio finishes.
 */
export const speak = (text: string) => invoke<void>("speak", { text });

export const stopSpeaking = () => invoke<void>("stop_speaking");

/**
 * The voice pronunciation will use, or `null` when the system has no Chinese
 * voice installed.
 */
export const speechStatus = () => invoke<string | null>("speech_status");

/**
 * Echo a line to the Rust process's stderr.
 *
 * The webview console is not visible from a terminal, so milestones are logged
 * through here to make a blank or half-initialised window diagnosable.
 */
export const log = (message: string) => invoke<void>("webview_log", { message });

// ---- personal vocabulary list ---------------------------------------------

/**
 * Resolve a character or word for the vocabulary add form.
 *
 * A single character returns a reading and meaning; a word returns a composed
 * reading and per-character hints, with the meaning left for the user.
 */
export const lookupText = (text: string) => invoke<TextLookup>("lookup_text", { text });

/**
 * Write a tone mark into the syllable the cursor is in.
 *
 * The pinyin tone row's one call. `tone` is `1`..`4`, or `5` for the neutral
 * tone, which takes a mark off. Which syllable the cursor selects and which
 * letter in it takes the mark are the Rust side's rules, next to the code that
 * reads those marks back, so writing one and reading it cannot disagree.
 *
 * `caret` is a **character** offset, not the UTF-16 index `selectionStart`
 * gives: convert before calling, and convert the returned `caret` back before
 * setting the selection. The two agree for everything in the basic plane, which
 * is all pinyin is, but a field that accepted anything else would otherwise put
 * the cursor one place off.
 */
export const markTone = (text: string, caret: number, tone: number) =>
  invoke<MarkedTone>("mark_tone", { text, caret, tone });

export const vocabulary = () => invoke<VocabView>("vocabulary");

/**
 * Add an entry. `group` may be null for an unfiled entry; a new group name is
 * created on demand.
 */
export const vocabAdd = (text: string, pinyin: string, meaning: string, group: string | null) =>
  invoke<VocabView>("vocab_add", { text, pinyin, meaning, group });

export const vocabUpdate = (
  id: number,
  pinyin: string,
  meaning: string,
  group: string | null,
) => invoke<VocabView>("vocab_update", { id, pinyin, meaning, group });

export const vocabRemove = (id: number) => invoke<VocabView>("vocab_remove", { id });

export const vocabAddGroup = (name: string) => invoke<VocabView>("vocab_add_group", { name });

export const vocabRenameGroup = (from: string, to: string) =>
  invoke<VocabView>("vocab_rename_group", { from, to });

/** `purge` false keeps the group's entries and leaves them unfiled. */
export const vocabRemoveGroup = (name: string, purge: boolean) =>
  invoke<VocabView>("vocab_remove_group", { name, purge });

/** Record a practice attempt; `score` is the 0..=100 headline score. */
export const vocabRecordAttempt = (id: number, score: number) =>
  invoke<VocabView>("vocab_record_attempt", { id, score });

export const vocabExport = (path: string, format: "json" | "csv") =>
  invoke<string>("vocab_export", { path, format });

export const vocabImport = (path: string, merge: boolean) =>
  invoke<VocabOutcome>("vocab_import", { path, merge });

/**
 * Where the learner got to in one of their own groups, as an entry id.
 *
 * `null` when the group has no position, or when the position names an entry
 * this list does not have — a caller starts where it otherwise would.
 */
export const vocabCursor = (group: string) =>
  invoke<number | null>("vocab_cursor", { group });

/** Move a group's position, or clear it with `null`. */
export const setVocabCursor = (group: string, entryId: number | null) =>
  invoke<void>("set_vocab_cursor", { group, entryId });

/**
 * Write the practice log to `path`, as JSON Lines or CSV.
 *
 * The whole log, every attempt, whether or not it carries the measures it was
 * graded from. Returns the line to show the learner.
 */
export const exportPracticeLog = (path: string, format: "jsonl" | "csv") =>
  invoke<string>("export_practice_log", { path, format });

// ---- practice progress and review -----------------------------------------

/** Every practised character: attempts, history, best score and due date. */
export const progress = () => invoke<ProgressView>("progress");

/**
 * Record one graded character; `score` is the 0..=100 headline score. The
 * backend derives the review rating and schedules the next appearance.
 *
 * `measures` is the report the score came from. Passing it is what makes the
 * grader tunable later: nothing can recover the per-measure result once the
 * strokes are gone. It is optional so a caller with only a score still records.
 */
export const recordProgress = (
  ch: string,
  score: number,
  measures?: AttemptMeasures,
) => invoke<ProgressView>("record_progress", { ch, score, measures });

/** What is due for review now, most overdue first, from both sources. */
export const reviewQueue = () => invoke<ReviewView>("review_queue");

export const courseCursor = () => invoke<CursorView>("course_cursor");

/** Move the course cursor. The backend clamps the index to the course. */
export const setCourseCursor = (index: number) =>
  invoke<CursorView>("set_course_cursor", { index });

// ---- settings --------------------------------------------------------------

/** The learner's settings, with an unchosen preference reported as `null`. */
export const settings = () => invoke<SettingsView>("settings");

/**
 * Change one or more preferences.
 *
 * Only what changed has to be sent: an absent field means "leave this one
 * alone", so a voice change does not reset the board size. The value returned is
 * what to render, warning included — the backend has already written it.
 *
 * A voice of `""` goes back to the automatic voice. Going back to the *device's*
 * answer for click-to-draw is `clearClickToDraw` instead, because a missing
 * argument and a `null` one are indistinguishable once they are on the wire.
 */
export const updateSettings = (patch: SettingsPatch) =>
  invoke<SettingsView>("update_settings", patch);

/**
 * Forget the click-to-draw choice and follow the device again.
 *
 * The one preference with a third state, and the only reason the interface can
 * say "a mouse wants click-to-draw, a stylus wants to drag" rather than picking
 * one for everybody.
 */
export const clearClickToDraw = () => invoke<SettingsView>("clear_click_to_draw");

/**
 * The voices a Chinese character can be spoken with, and the one in use.
 *
 * Served from a list the backend cached at startup — enumerating the system's
 * voices takes about a second — so opening the settings screen is cheap.
 */
export const voices = () => invoke<VoicesView>("voices");

// ---- what the app is, and what it ships under ------------------------------

/** The app's name, version and licence, for the About screen. */
export const appInfo = () => invoke<AppInfo>("app_info");

/**
 * Whether this app has run here before, and which version is running.
 *
 * Read once at startup, and the two together are what decide the first screen: a
 * first run is shown the introduction, an installation that has run before is
 * shown what changed, keyed to the version reported here. One call so the flag
 * and the version cannot come from two different places.
 */
export const startup = () => invoke<StartupView>("startup");

/**
 * Every licence and attribution notice the app ships with, full text included.
 *
 * The texts are compiled into the binary rather than read from the bundle's
 * resource directory, so this cannot come back empty on a packaged build.
 */
export const licenceNotices = () => invoke<LicenceNotice[]>("licence_notices");

// ---- tone practice ---------------------------------------------------------

/**
 * What the microphone can do with the text on the board.
 *
 * Two independent answers. `tone` is null for text longer than a word, for a
 * reading that will not divide one syllable per character, and for characters the
 * dataset does not know; `recognize` says whether a recognition model is
 * installed. The control is offered when either is true, because text with no
 * tone target can still be recognised and tell the learner whether they were
 * understood.
 */
export const speechTarget = (text: string) =>
  invoke<SpeechTarget>("speech_target", { text });

/** Whether the microphone can be used, and at what rate. */
export const microphoneStatus = () => invoke<MicrophoneStatus>("microphone_status");

/**
 * Begin listening. Resolves once the device is actually open, so a failure to
 * open it arrives here rather than as silence.
 */
export const listenStart = () => invoke<void>("listen_start");

/**
 * Stop listening and judge what was heard against `text`.
 *
 * `text` is what was on screen while the learner spoke; it is resolved to tones
 * or to readings by the Rust side rather than sent from here, so the recording
 * cannot be judged against a sequence the interface invented. A short word comes
 * back with `toneScored` true and a judgement per syllable; a longer phrase comes
 * back with it false and the transcription as the answer. The detail sentence is
 * worded by the Rust side too; show it as it comes back.
 */
export const listenStop = (text: string) => invoke<ToneResult>("listen_stop", { text });

// ---- speech recognition ----------------------------------------------------

/**
 * Whether a recognition model is installed, and how to describe one that is not.
 *
 * Always answers, so the settings screen can show what *would* be downloaded —
 * from where, how large, under which licence — before a learner has agreed to any
 * of it. Poll this while `state` is `downloading`: the download runs on its own
 * thread and reports progress here rather than through events.
 */
export const asrStatus = () => invoke<AsrStatus>("asr_status");

/**
 * Fetch, verify and unpack the recognition model.
 *
 * **The one call in this app that touches the network**, and it only ever happens
 * because a learner pressed a button. Resolves once the download has *started*,
 * not when it finishes — follow `asrStatus` for the outcome and the progress.
 * Nothing else in the app behaves differently while it runs, and nothing calls
 * this on the learner's behalf.
 */
export const asrInstall = () => invoke<void>("asr_install");

/** Delete the recognition model, returning the state that leaves behind. */
export const asrRemove = () => invoke<AsrStatus>("asr_remove");

// ---- On-device speech synthesis -------------------------------------------
//
// The same shape as the recognition model above, and for the same reasons: an
// optional download the learner starts by hand, polled for progress while it
// runs. See `say.rs` for why the app offers this when the system synthesiser
// usually answers.

/** Whether the synthesis model is installed, and what installing it would cost. */
export const sayStatus = () => invoke<SayStatus>("say_status");

/**
 * Fetch and verify the synthesis model.
 *
 * Blocks until the files are on disk, so callers run it from a background task
 * and follow `sayStatus` for progress — as `asrInstall` does.
 */
export const sayInstall = () => invoke<void>("say_install");

/** Delete the synthesis model, and report the resulting state. */
export const sayRemove = () => invoke<SayStatus>("say_remove");

/**
 * Speak `text` with the installed model.
 *
 * Resolves `null` when no model is installed, which is how the app ships — not
 * an error, just a fallback to the platform synthesiser.
 */
export const saySpeak = (text: string) =>
  invoke<SpokenAudio | null>("say_speak", { text });


// ---- cross-device sync ------------------------------------------------------

/**
 * Whether this device is connected, and what happened last time.
 *
 * Reading this never touches the network: the connection is a refresh token in the
 * platform's secret store, and this only asks whether there is one.
 */
export const syncStatus = () => invoke<SyncView>("sync_status");

/**
 * Start connecting a Dropbox account, and open the authorization page.
 *
 * The page opens in the **system browser**, not in this window: Dropbox asks for
 * that, and Google's policy forbids their sign-in flow inside a webview, which
 * matters for accounts that sign in to Dropbox through Google. The URL comes back
 * as well, so a learner whose browser did not come forward can open it themselves.
 *
 * Dropbox will not redirect back to an app, so the flow ends with a code shown on
 * that page which has to be pasted into `syncConnectFinish`.
 */
export const syncConnect = () => invoke<string>("sync_connect");

/** Finish connecting, with the code the Dropbox page showed. */
export const syncConnectFinish = (code: string) =>
  invoke<SyncView>("sync_connect_finish", { code });

/**
 * Sync now: send this device's attempts, take the other's, and rebuild the
 * schedule from the whole log.
 *
 * One of the two calls in this app that send anything about the learner's study
 * data anywhere — `syncAuto` is the other, and it is the one that happens without
 * anybody pressing anything. Both are off until an account is connected, and both
 * go only to the learner's own Dropbox. Nothing else in the app behaves
 * differently while either runs.
 */
export const syncNow = () => invoke<SyncView>("sync_now");

/** Forget the account here and on Dropbox's side, leaving study data untouched. */
export const syncDisconnect = () => invoke<SyncView>("sync_disconnect");

/**
 * Ask for a fingerprint before the sign-in is used, or stop asking for one.
 *
 * Nothing is unlocked to answer this. The preference is written down, and the stored
 * item is rewritten only where there is one — which is the one moment turning the
 * prompt *off* has to read it, because an access control is fixed when a keychain
 * item is created and cannot be changed afterwards.
 *
 * Asking for nothing is the default, which is what makes a sync that starts by itself
 * possible: see `src-tauri/src/sync.rs`.
 */
export const syncSetLock = (locked: boolean) =>
  invoke<SyncView>("sync_set_lock", { locked });

/**
 * Sync because the app started or came back, rather than because you pressed a
 * button.
 *
 * Nothing here is a failure the learner has to act on: most launches are not
 * connected to anything, and a phone on a train has no network. The four outcomes
 * keep those apart so the screen can be silent about the first two — see [`AutoSync`].
 *
 * A device with no account is not even asked: the backend answers from its own
 * record, with no keychain read and no socket. A device with no *network* is refused
 * by a bounded probe rather than by `ureq`'s ten-second connect timeout, because this
 * is the one sync nobody is waiting for and it must not be the one that stalls.
 */
export const syncAuto = () => invoke<AutoSync>("sync_auto");
