/** Thin typed wrappers over the Tauri commands. */

import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  Character,
  CursorView,
  DatasetStats,
  GradeOptions,
  GradeReport,
  Lesson,
  LicenceNotice,
  Point,
  ProgressView,
  ReviewView,
  SettingsView,
  TextLookup,
  VocabOutcome,
  VocabView,
  WordSearchView,
} from "./types";

export const datasetStats = () => invoke<DatasetStats>("dataset_stats");

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

// ---- practice progress and review -----------------------------------------

/** Every practised character: attempts, history, best score and due date. */
export const progress = () => invoke<ProgressView>("progress");

/**
 * Record one graded character; `score` is the 0..=100 headline score. The
 * backend derives the review rating and schedules the next appearance.
 */
export const recordProgress = (ch: string, score: number) =>
  invoke<ProgressView>("record_progress", { ch, score });

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
 * Change how a stroke is drawn.
 *
 * `null` goes back to the device's own default, which is what a "follow my
 * device" option in a future settings dialog will send; the switch in the
 * controls writes `true` or `false`.
 */
export const updateSettings = (clickToDraw: boolean | null) =>
  invoke<SettingsView>("update_settings", { clickToDraw });

// ---- what the app is, and what it ships under ------------------------------

/** The app's name, version and licence, for the About screen. */
export const appInfo = () => invoke<AppInfo>("app_info");

/**
 * Every licence and attribution notice the app ships with, full text included.
 *
 * The texts are compiled into the binary rather than read from the bundle's
 * resource directory, so this cannot come back empty on a packaged build.
 */
export const licenceNotices = () => invoke<LicenceNotice[]>("licence_notices");
