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
  DatasetStats,
  DrillQuestion,
  DrillTally,
  GradeReport,
  Kana,
  LessonView,
  LicenceNotice,
  Point,
  ReadingCheck,
  ScriptName,
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
