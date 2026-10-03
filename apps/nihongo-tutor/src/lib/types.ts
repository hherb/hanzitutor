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
export interface Kana extends Drawable {
  script: ScriptName;
  practisable: boolean;
  /** Every spelling a learner might type, Hepburn first. */
  romaji: string[];
  /** The one to show as *the* reading. */
  hepburn: string;
  /** True for っ and ー, which have no sound of their own. */
  silent: boolean;
  confusions: ConfusionView[];
}

/**
 * The name the renderer knows a drawable thing by.
 *
 * `render.ts` is lifted unchanged from Hanzi Tutor, where its input is a Chinese
 * `Character`; everything it actually reads is `outlines` and `medians`, which a
 * kana, a kanji and a radical head form all carry. Keeping the lift verbatim is
 * worth more than the name, so the name stays and the shape is structural: this
 * was `= Kana` until the kanji screens were built, and widening it to the shape
 * the file reads is what let one board serve all three without touching
 * `render.ts` at all.
 */
export type Character = Drawable;

/**
 * Anything the practice board can draw and the grader can score.
 *
 * The app writes three kinds of thing — a kana, a jōyō kanji, and one of the 214
 * radical head forms — and they differ in everything except this. A component
 * asks for a `Drawable`, so there is one canvas, one animation and one grading
 * call for all three; the Rust side resolves the character the same way.
 */
export interface Drawable {
  ch: string;
  /** How many strokes it is taught with, which is what the grader sees. */
  strokeCount: number;
  /** SVG path data in font space, one per taught stroke, in stroke order. */
  outlines: string[];
  /** Centre-lines in display space, for the faint guide and for grading. */
  medians: Point[][];
}

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
  /** How many jōyō kanji the artifact holds. */
  kanji: number;
  /** How many of them are kyōiku — grades 1 to 6. */
  kyoiku: number;
  /** How many Kangxi radicals the table holds: always 214. */
  radicals: number;
  /** How many lessons the kanji course is sliced into. */
  kanjiLessons: number;
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

/** One lesson of the kanji course, as the sidebar lists it. */
export interface KanjiLessonView {
  /** Stable key, e.g. `"g1-3"`. */
  key: string;
  /** Where the lesson sits inside its grade, e.g. `1–10`. */
  title: string;
  /** KANJIDIC2's grade: 1–6 kyōiku, 8 the jōyō remainder. */
  grade: number;
  /**
   * What to call the grade, from `nihongo_core::grade_name` — the same names the
   * vocabulary's bands use, because the two are one ladder.
   */
  gradeName: string;
  count: number;
  kanji: string[];
}

/**
 * The radical a character is classified under, in both of its shapes.
 *
 * `ch` is the head form from the 214-radical table (手) and `form` is the shape
 * written inside the character (扌). They differ for most characters, and the
 * difference is the lesson rather than an inconsistency.
 */
export interface RadicalRef {
  /** The classical number, 1–214. */
  number: number;
  /** The head form. */
  ch: string;
  /** The shape written inside this character. */
  form: string;
  /** What the upstream dictionary says in parentheses, when it says anything. */
  note: string | null;
  strokeCount: number;
  /** How many characters in the course are classified under it. */
  characters: number;
}

/** One part of a character's decomposition. */
export interface DecompositionPart {
  /** The part, or null where the source could not name it. */
  ch: string | null;
  /** True when the board can write it, so it can be opened on its own. */
  drawable: boolean;
}

/** What a character is built from, parsed from AnimCJK's IDS string. */
export interface Decomposition {
  /** The IDS string as stored, e.g. `⿳𰃮子`. Empty when there is none. */
  raw: string;
  /** The outermost arrangement in words, e.g. `"above, middle and below"`. */
  layout: string;
  /** The parts, in reading order. */
  parts: DecompositionPart[];
}

/** One jōyō kanji, as the course draws it. */
export interface KanjiView extends Drawable {
  grade: number;
  /** What to call the grade, from `nihongo_core::grade_name`. */
  gradeName: string;
  /** KANJIDIC2's frequency rank, or null for the characters it does not rank. */
  frequency: number | null;
  /** On'yomi in katakana, in KANJIDIC2's order. */
  on: string[];
  /** Kun'yomi, with KANJIDIC2's okurigana markers intact (た.べる). */
  kun: string[];
  meanings: string[];
  nanori: string[];
  radical: RadicalRef;
  decomposition: Decomposition;
  practisable: boolean;
}

/** One radical and the characters that share it. */
export interface RadicalFamilyView {
  /** The classical number, 1–214. */
  number: number;
  /** The head form, never the shape written inside a character. */
  ch: string;
  strokeCount: number;
  /**
   * The characters classified under it, most frequent first. Empty for the
   * sixteen radicals no jōyō character uses — they are still listed.
   */
  characters: string[];
}

/** One radical on its own, with the geometry the board writes it with. */
export interface RadicalView extends Drawable {
  number: number;
  characters: string[];
}

/**
 * A character another screen has asked the kanji board to open.
 *
 * The radicals screen works in radicals, the course works in characters, and
 * both end up on the one board — so the request says which kind it is rather
 * than leaving the board to guess from the character.
 */
export type KanjiPick =
  | { kind: "kanji"; ch: string }
  | { kind: "radical"; number: number };

/**
 * The verdict on an attempt, and what the review schedule did with it.
 *
 * Grading and scheduling are one command because they are one action: the learner
 * wrote a character and pressed Grade. `scheduled` is false for a second attempt
 * inside the character's interval — that attempt is still graded, and the
 * schedule is deliberately left alone, because five attempts in one sitting must
 * not stretch an interval by months.
 */
export interface GradedAttempt {
  report: GradeReport;
  /** True when the character was new or due, so the attempt advanced the schedule. */
  scheduled: boolean;
  /** When the character comes back, ISO-8601 UTC. */
  nextDue: string | null;
  /**
   * Set when the attempt was counted but could not be written to the learner's
   * own file, or when the schedule could not be opened at all. The screen says so
   * rather than losing it quietly.
   */
  warning: string | null;
}

/** What kind of thing a due character is, which says how to fetch its geometry. */
export type ReviewKind = "kana" | "kanji" | "radical";

/**
 * One character the review queue is offering.
 *
 * A card is about a character, not a word: the board grades one character at a
 * time, and that is the unit `hanzi_core::progress` schedules.
 */
export interface DueItem {
  ch: string;
  kind: ReviewKind;
  /** The prompt beside it: a kana's reading, a kanji's gloss, or the radical's number. */
  hint: string;
  /**
   * The Kangxi number, for a head form the character course cannot reach — 92 of
   * the 214 are not jōyō characters, so this is the only way to ask for its
   * geometry. Null for a kana or a kanji.
   */
  radical: number | null;
  /** When it came due, ISO-8601 UTC. The queue is ordered by this. */
  due: string;
  intervalDays: number;
  attempts: number;
  lapses: number;
}

/** The review queue, plus how much of the schedule stands behind it. */
export interface ReviewQueueView {
  /** Due characters, most overdue first, capped at the caller's limit. */
  items: DueItem[];
  /** Every character the schedule holds at all, due or not. */
  cards: number;
  /** How many are due, which is the length of the *uncapped* queue. */
  due: number;
  /** When the next character comes back, when nothing is due now. */
  nextDue: string | null;
  warning: string | null;
}
