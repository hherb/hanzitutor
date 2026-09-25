/** Thin typed wrappers over this app's Tauri commands. */

import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  AsrStatus,
  MicrophoneStatus,
  ScoreResult,
  ToneSet,
  WordSet,
} from "./types";

/**
 * Characters that differ only in tone, most useful contrast first.
 *
 * The whole derived list in one call, so the drill filters and re-ranks what it
 * holds rather than asking again for every change. `limit` is a cap, not a
 * request for more.
 */
export const toneSets = (limit?: number) =>
  invoke<ToneSet[]>("tone_sets", { limit: limit ?? null });

/**
 * Word families built on those characters, most useful first.
 *
 * The word half of the drill. A family is keyed by one character from a tone set
 * and holds the HSK words containing it, with the tones they are actually *spoken*
 * with — sandhi included, so 你好 is offered as tone 2 + tone 3 rather than the
 * dictionary's 3 + 3.
 */
export const wordSets = (limit?: number) =>
  invoke<WordSet[]>("word_sets", { limit: limit ?? null });

/** Whether the microphone can be used, and at what rate. */
export const microphoneStatus = () => invoke<MicrophoneStatus>("microphone_status");

/**
 * Begin listening. Resolves once the device is actually open, so a failure to
 * open it arrives here rather than as silence.
 */
export const listenStart = () => invoke<void>("listen_start");

/**
 * Stop listening and judge what was heard against what the drill was showing.
 *
 * `text` and `reading` are what the learner was looking at while they spoke — a
 * character with the one tone they chose, or a whole word with the tones the
 * dictionary and sandhi give it. Both are passed in rather than looked up by the
 * backend so that the judgement cannot be made against something the interface
 * invented.
 *
 * The reading is resolved here because for a character drill the tone was chosen by
 * the learner and appears in no dictionary; for a word it is the dataset's own
 * whole-word reading, which is what resolves a polyphone.
 */
export const listenStop = (text: string, reading: string) =>
  invoke<ScoreResult>("listen_stop", { text, reading });

/**
 * Stop listening and throw the recording away.
 *
 * What leaving the Say screen while the button is held needs: the microphone has
 * to close, but there is no longer a drill to judge the audio against, and posting
 * a result onto a screen the learner has left would be worse than discarding it.
 */
export const listenCancel = () => invoke<void>("listen_cancel");

/**
 * Start pronouncing a character. Resolves as soon as the synthesiser has
 * started, not when the audio finishes.
 *
 * The **character** is spoken rather than its pinyin: the system's Chinese voice
 * has a lexicon, so 妈 produces `mā`, whereas an English-trained voice handed
 * the string `mā` would guess at the diacritic.
 */
export const speak = (text: string) => invoke<void>("speak", { text });

/** Stop whatever is being spoken. */
export const stopSpeaking = () => invoke<void>("stop_speaking");

/** The voice pronunciation will use, or `null` when there is no Chinese voice. */
export const voice = () => invoke<string | null>("voice");

/** The longest recording the backend will keep, in seconds. */
export const maxRecordSecs = () => invoke<number>("max_record_secs");

// ---- the optional recognition model ----------------------------------------

/**
 * Whether a recognition model is installed, and how to describe one that is not.
 *
 * Always answers, so the screen can show what *would* be downloaded — from where,
 * how large, under which licence — before the learner has agreed to any of it.
 * Poll this while `state` is `downloading`: the download runs on its own thread
 * and reports progress here rather than through events.
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

/** The app's name, version and licence. */
export const appInfo = () => invoke<AppInfo>("app_info");

/**
 * Echo a line to the Rust process's stderr.
 *
 * The webview console is not visible from a terminal, so a blank window is
 * otherwise undiagnosable.
 */
export const log = (message: string) => invoke<void>("webview_log", { message });
