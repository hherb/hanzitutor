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
  /** How many words the vocabulary holds. */
  words: number;
  /** How many reading passages are shipped. */
  passages: number;
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
 * One question from the discrimination drill.
 *
 * The pair travels with the question because the answer is recorded against the
 * *pair*, and the asker is the only thing that knows which one it chose. `options`
 * are the kana to offer, one of which is `ch`. Send `pair`, `ch` and the kana that
 * was picked back unchanged: whether it was right is decided in Rust, because a
 * client that could report its own correctness could lie to itself.
 */
export interface DrillQuestion {
  /** The canonical key of the pair under test, e.g. `シ|ツ`. */
  pair: string;
  /** The kana the learner is being asked to recognise. */
  ch: string;
  /** The reading to prompt with. */
  hepburn: string;
  /** The kana to offer as answers, one of which is `ch`. */
  options: string[];
  /** What tells the two apart, so a miss teaches as well as records. */
  tell: string;
}

/** What the learner's record for one pair is, after an answer. */
export interface DrillTally {
  pair: string;
  asked: number;
  correct: number;
  wrong: number;
  /** The pair's share of the drill — see `nihongo_core::drill` for the rule. */
  weight: number;
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

/**
 * One segment of a word's furigana: the text it covers, and the reading over it.
 *
 * `rt` is null for a segment that is already kana, which needs no ruby — and it is
 * *present* rather than absent, so the renderer can tell "no reading" from "field
 * renamed".
 */
export interface Ruby {
  ruby: string;
  rt: string | null;
}

/** One band of the vocabulary ladder. */
export interface BandView {
  band: number;
  /**
   * What the band is called, from `nihongo_core::band_name` — "kyōiku 3", "jōyō
   * beyond the school grades". The ladder is this project's derivation from the
   * kyōiku grades and EDRDG's frequency ranking, not the JLPT's, and these names
   * are how the interface says so.
   */
  name: string;
  words: number;
}

/** One word, as the vocabulary screen draws it. */
export interface Word {
  text: string;
  /** The whole word's reading, from the dictionary — never composed from its characters. */
  reading: string;
  meaning: string;
  band: number;
  bandName: string;
  /** EDRDG's frequency block, 1–48, or null for a word it marks common without ranking. */
  nf: number | null;
  furigana: Ruby[];
}

/** A page of one band's words. */
export interface WordPage {
  band: number;
  total: number;
  offset: number;
  words: Word[];
}

/** One passage, as the list offers it. */
export interface PassageSummary {
  key: string;
  title: string;
  gloss: string | null;
  lines: number;
  tokens: number;
}

/** One word of a passage. */
export interface PassageToken {
  surface: string;
  /** The reading to draw over it; null for kana. */
  rt: string | null;
  /** The vocabulary word this token is, so a tap can open its card. */
  word: string | null;
}

/** One passage, segmented. */
export interface PassageView {
  key: string;
  title: string;
  gloss: string | null;
  lines: PassageToken[][];
}
