/**
 * The IPC surface, in one place.
 *
 * Every function here is a thin wrapper over `invoke`, so a screen never writes
 * a command name as a string literal in two places. The names match the
 * `#[tauri::command]` functions in `src-tauri/src/lib.rs`, and the payload keys
 * match the field names this app's `types.ts` declares.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  BandView,
  DatasetStats,
  DrillQuestion,
  DrillTally,
  GradeReport,
  Kana,
  KanjiLessonView,
  KanjiView,
  LessonView,
  LicenceNotice,
  PassageSummary,
  PassageView,
  Point,
  RadicalFamilyView,
  RadicalView,
  ReadingCheck,
  ScriptName,
  Word,
  WordPage,
  YoonView,
} from "./types";

/**
 * The pen width the canvas paints with, in design units out of 1024.
 *
 * The grader is told this explicitly so that the ink measure compares what was
 * drawn with what a correct trace at the same width would put down. It is the
 * same number as `INK_WIDTH` in `render.ts` and in
 * `crates/hanzi-core/src/raster.rs`; passing it rather than letting the grader
 * assume keeps the three from drifting apart silently.
 */
import { INK_WIDTH } from "./render";

export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}

export function datasetStats(): Promise<DatasetStats> {
  return invoke<DatasetStats>("dataset_stats");
}

export function lessons(script: ScriptName): Promise<LessonView[]> {
  return invoke<LessonView[]>("lessons", { script });
}

export function kana(ch: string): Promise<Kana> {
  return invoke<Kana>("kana", { ch });
}

/**
 * Grade a handwritten attempt.
 *
 * Only the pen width is sent; the other three tunables are left to the Rust
 * default, because those defaults are the ones the whole tolerance study was
 * fitted against and the interface has no business overriding them piecemeal.
 * `GradeOptions` therefore deserialises a partial object — an interface that
 * sends some fields and not others must not be rejected for the ones it left
 * out.
 */
export function gradeAttempt(ch: string, strokes: Point[][]): Promise<GradeReport> {
  return invoke<GradeReport>("grade_attempt", {
    ch,
    strokes,
    options: { inkWidth: INK_WIDTH },
  });
}

/** Check a typed reading, accepting either romanisation. */
export function checkReading(ch: string, typed: string): Promise<ReadingCheck> {
  return invoke<ReadingCheck>("check_reading", { ch, typed });
}

/** Turn typed romaji into kana. */
export function romajiToKana(input: string, script?: ScriptName): Promise<string> {
  return invoke<string>("romaji_to_kana", { input, script: script ?? null });
}

/**
 * The next question for the discrimination drill, or `null` if no pair can be
 * asked at all.
 *
 * The pair is drawn in Rust, weighted by how often this learner has missed it —
 * the rule is in `nihongo_core::drill` — rather than drawn here from a pool. That
 * is not a layering preference: only the asker knows which pair it asked, and the
 * answer has to be remembered against that pair.
 */
export function nextDrillQuestion(): Promise<DrillQuestion | null> {
  return invoke<DrillQuestion | null>("next_drill_question");
}

/**
 * Record what the learner answered, and get the pair's record back.
 *
 * `target` is the kana the question asked for and `picked` is the one that was
 * chosen. There is deliberately no way to say whether it was right: correctness is
 * decided in Rust from those two, against the pair the question named.
 */
export function recordDrillAnswer(
  pair: string,
  target: string,
  picked: string,
): Promise<DrillTally> {
  return invoke<DrillTally>("record_drill_answer", { pair, target, picked });
}

/** Every notice this app owes, with its text. */
export function licences(): Promise<LicenceNotice[]> {
  return invoke<LicenceNotice[]>("licences");
}

/** The yōon digraphs for a script, for an input helper. */
export function yoon(script: ScriptName): Promise<YoonView[]> {
  return invoke<YoonView[]>("yoon", { script });
}

/**
 * The kanji course: every grade in teaching order, sliced into lessons.
 *
 * Whole rather than paged, unlike a band of words: it is 2,136 characters and
 * their lesson keys, where a band is nearly five thousand words with readings,
 * glosses and furigana each.
 */
export function kanjiLessons(): Promise<KanjiLessonView[]> {
  return invoke<KanjiLessonView[]>("kanji_lessons");
}

/** One kanji: the geometry the board writes, the readings, and the structure. */
export function kanji(ch: string): Promise<KanjiView> {
  return invoke<KanjiView>("kanji", { ch });
}

/** The 214 Kangxi radicals, each with the characters that share it. */
export function radicals(): Promise<RadicalFamilyView[]> {
  return invoke<RadicalFamilyView[]>("radicals");
}

/**
 * One radical, with the geometry the board writes it with.
 *
 * A head form is not always a jōyō character — 92 of the 214 are not — so this is
 * how the board gets something to draw for the radicals the character course
 * cannot reach.
 */
export function radical(number: number): Promise<RadicalView> {
  return invoke<RadicalView>("radical", { number });
}

/** The vocabulary ladder: each band's name and how many words it holds. */
export function wordBands(): Promise<BandView[]> {
  return invoke<BandView[]>("word_bands");
}

/**
 * One page of a band's words, in course order.
 *
 * Paged because band 7 holds nearly five thousand words: the screen draws a list,
 * and the whole band as JSON would be a megabyte to show sixty rows.
 */
export function wordsInBand(band: number, offset = 0, limit = 60): Promise<WordPage> {
  return invoke<WordPage>("words_in_band", { band, offset, limit });
}

/** One word, by its text and its reading. */
export function word(text: string, reading: string): Promise<Word> {
  return invoke<Word>("word", { text, reading });
}

/**
 * One word, by its text alone.
 *
 * What a tapped passage token has: the token carries the word's *text*, and its
 * own reading is the surface's (行き is read いき while the word is 行く, read
 * いく), so the reading cannot identify the entry. Where a text is read two ways
 * this returns the one the course reaches first.
 */
export function wordOfText(text: string): Promise<Word> {
  return invoke<Word>("word_of_text", { text });
}

/**
 * Check a typed reading against a word.
 *
 * The word is graded *as a word*: its own reading, taken from the dictionary and
 * never composed from its characters. Romaji and kana are both accepted — a
 * learner who can read kana should not have to transliterate to answer.
 */
export function checkWord(text: string, reading: string, typed: string): Promise<ReadingCheck> {
  return invoke<ReadingCheck>("check_word", { text, reading, typed });
}

/** The passages a learner can read. */
export function passages(): Promise<PassageSummary[]> {
  return invoke<PassageSummary[]>("passages");
}

/** One passage, segmented, with a reading over every kanji. */
export function passage(key: string): Promise<PassageView> {
  return invoke<PassageView>("passage", { key });
}
