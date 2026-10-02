/**
 * The shapes that cross the IPC boundary, and the ones the canvas works in.
 *
 * These mirror the Rust types in `src-tauri/src/lib.rs` one for one, and
 * `src-tauri/tests/ipc_contract.rs` asserts the keys and value shapes on the Rust
 * side. A rename there fails that test rather than quietly showing nothing here.
 */

/** A point in whichever space the caller is working in. */
export interface Point {
  x: number;
  y: number;
}

/** How a single stroke was judged. */
export type Verdict =
  | "correct"
  | "shape_off"
  | "position_off"
  | "wrong_direction"
  | "out_of_order"
  | "faint"
  | "missing";

export interface StrokeVerdict {
  refIndex: number;
  /** Index into the attempt's strokes, or null when the stroke was never written. */
  userIndex: number | null;
  verdict: Verdict;
  shape: number;
  position: number;
  /** How much of the ink a correct stroke needs that this one put down, 0..1. */
  ink: number;
  score: number;
}

/** How the attempt sat in the box before any fitting was applied. */
export interface FitInfo {
  scale: number;
  offset: number;
}

/**
 * The grader's verdict on one attempt. Produced by the same code that grades a
 * Chinese character — nothing about it is language-specific.
 */
export interface GradeReport {
  expectedStrokes: number;
  givenStrokes: number;
  strayStrokes: number;
  countOk: boolean;
  strokes: StrokeVerdict[];
  /** `assignment[userStrokeIndex]` = reference stroke index, or null if extra. */
  assignment: (number | null)[];
  shapeScore: number;
  positionScore: number;
  /** How much ink was put down, 0..1. */
  inkScore: number;
  /** How much of the kana's own ink was reached, 0..1. */
  inkCoverage: number;
  orderScore: number;
  /** Headline score, 0..100. */
  overall: number;
  /** Recognisable and correctly placed, independent of order. */
  legible: boolean;
  fit?: FitInfo;
}

/** A kana this one is mistaken for, and what tells them apart. */
export interface ConfusionView {
  ch: string;
  tell: string;
}

/**
 * One kana, as the practice screen uses it.
 *
 * `outlines` are SVG path data in **font space** (y increases upwards, top edge
 * at y = 900) and `medians` are centre-lines in **display space** (y increases
 * downwards) — the same two spaces as the Chinese app, because AnimCJK publishes
 * its kana in Make Me a Hanzi's own frame.
 */
export interface Kana {
  ch: string;
  script: ScriptName;
  /** How many strokes this kana is *taught* with, which is what the grader sees. */
  strokeCount: number;
  practisable: boolean;
  /** Every spelling a learner might type, Hepburn first. */
  romaji: string[];
  /** The one to show as *the* reading. */
  hepburn: string;
  /** True for っ and ー, which have no sound of their own. */
  silent: boolean;
  outlines: string[];
  medians: Point[][];
  confusions: ConfusionView[];
}

/**
 * The name the renderer knows a drawable thing by.
 *
 * `render.ts` is lifted unchanged from Hanzi Tutor, where its input is a Chinese
 * `Character`; a kana carries the same two fields it actually reads — `outlines`
 * and `medians` — so the alias is all the adaptation it needs. Keeping the lift
 * verbatim is worth more than the name.
 */
export type Character = Kana;

export type ScriptName = "hiragana" | "katakana";

/** What the course contains. */
export interface DatasetStats {
  kana: number;
  hiragana: number;
  katakana: number;
  lessons: number;
  strokes: number;
}

/** One lesson, as the sidebar lists it. */
export interface LessonView {
  key: string;
  title: string;
  kana: string[];
  voiced: boolean;
  count: number;
}

/** A yōon digraph: two kana that make one mora. */
export interface YoonView {
  key: string;
  display: string;
  hepburn: string;
  kunrei: string;
  kana: string[];
}

/** The result of checking a typed romaji answer. */
export interface ReadingCheck {
  correct: boolean;
  /** What the typing actually produced, so a wrong answer can show it. */
  produced: string;
}

export interface AppInfo {
  name: string;
  version: string;
  identifier: string;
  licence: string;
  copyright: string;
  repository: string;
}

/**
 * A kana the discrimination drill can ask about.
 *
 * `partners` are the kana it is mistaken for — the wrong answers. A kana that
 * confuses nobody is not in the pool at all.
 */
export interface DrillKana {
  ch: string;
  script: ScriptName;
  /** The reading to prompt with. */
  hepburn: string;
  partners: string[];
}

/** One licence notice, with its text, for a Licences screen. */
export interface LicenceNotice {
  id: string;
  title: string;
  licence: string;
  source: string;
  covers: string;
  file: string;
  bundlePath: string;
  /** The notice text itself, compiled into the binary and sent over IPC. */
  text: string;
}
