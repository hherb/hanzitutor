/** Thin typed wrappers over the Tauri commands. */

import { invoke } from "@tauri-apps/api/core";
import type { Character, DatasetStats, GradeOptions, GradeReport, Lesson, Point } from "./types";

export const datasetStats = () => invoke<DatasetStats>("dataset_stats");

export const listLessons = () => invoke<Lesson[]>("lessons");

export const getCharacter = (ch: string) => invoke<Character>("character", { ch });

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
