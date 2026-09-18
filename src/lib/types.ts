/**
 * Types mirroring the Rust structures exposed over Tauri.
 *
 * The Rust side serialises with camelCase field names, so these line up
 * one-for-one with `hanzi_core::{Character, Lesson, GradeReport}`.
 */

/** A point in the 1024x1024 character box, y increasing downwards. */
export interface Point {
  x: number;
  y: number;
}

export type Verdict =
  | "correct"
  | "shape_off"
  | "position_off"
  | "wrong_direction"
  | "out_of_order"
  | "missing";

export type Grade = "excellent" | "good" | "fair" | "poor";

export interface StrokeVerdict {
  refIndex: number;
  /** Index into the attempt's strokes, or null when the stroke was never written. */
  userIndex: number | null;
  verdict: Verdict;
  shape: number;
  position: number;
  score: number;
}

/** How the attempt sat in the box before any fitting was applied. */
export interface FitInfo {
  scale: number;
  offset: number;
}

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
  orderScore: number;
  overall: number;
  legible: boolean;
  orderCorrect: boolean;
  grade: Grade;
  firstError: number | null;
  fit: FitInfo | null;
}

export interface Character {
  ch: string;
  /** Frequency rank, 1 = most common; 0 when not in the frequency list. */
  rank: number;
  /** HSK level, or 0 when not in the HSK lists. */
  hsk: number;
  strokeCount: number;
  radical: string;
  pinyin: string[];
  definition: string;
  etymology: string;
  /** SVG path data in font space (y up), one per stroke, in stroke order. */
  outlines: string[];
  /** Stroke centre-lines in display space (y down), in stroke order. */
  medians: Point[][];
}

export interface Lesson {
  index: number;
  title: string;
  characters: string[];
  firstRank: number;
  lastRank: number;
}

export interface DatasetStats {
  characters: number;
  teachable: number;
  lessons: number;
  lessonSize: number;
}

export interface GradeOptions {
  resampleK: number;
  minStrokeLen: number;
  globalFit: boolean;
}

// ---- personal vocabulary list ---------------------------------------------

/** What one character of a piece of study text contributes. */
export interface CharacterHint {
  ch: string;
  /** Every reading the dataset knows, most common first. */
  pinyin: string[];
  meaning: string;
}

/**
 * A draft reading and meaning for some study text.
 *
 * A single character gets both. A word gets its readings composed — word pinyin
 * is the characters' readings run together — but its meaning is left blank,
 * because a word's meaning cannot be derived from its characters.
 */
export interface TextLookup {
  pinyin: string;
  meaning: string;
  characters: CharacterHint[];
  /** True when every character was found and has a reading. */
  complete: boolean;
}

/**
 * One item in the personal vocabulary list.
 *
 * `text` may be a single character or a word. Single characters get their pinyin
 * and meaning filled from the dataset; words carry whatever the user typed,
 * because the dataset has no word data.
 */
export interface VocabEntry {
  id: number;
  text: string;
  pinyin: string;
  meaning: string;
  /** Group name, or null when the entry is not filed under a lesson. */
  group: string | null;
  /** ISO-8601 UTC timestamp, which also sorts chronologically as text. */
  addedAt: string;
  attempts: number;
  bestScore: number | null;
  lastPractised: string | null;
}

export interface VocabView {
  entries: VocabEntry[];
  groups: string[];
  /**
   * Set when a change was applied in memory but not saved, so the interface can
   * say so instead of appearing to have lost data.
   */
  warning: string | null;
}

/** A vocabulary change plus a note about what happened. */
export interface VocabOutcome {
  view: VocabView;
  message: string;
}

// ---- practice progress and review -----------------------------------------

/**
 * How well an attempt went, in the four grades a review offers. Derived by the
 * backend from the attempt score, using the same bands as `Grade`.
 */
export type Rating = "again" | "hard" | "good" | "easy";

/** One recorded attempt. */
export interface Attempt {
  /** ISO-8601 UTC timestamp. */
  at: string;
  score: number;
  rating: Rating;
}

/** Everything remembered about one character. */
export interface ProgressCard {
  ch: string;
  attempts: number;
  lapses: number;
  bestScore: number | null;
  lastScore: number | null;
  /** ISO-8601 UTC timestamp. */
  lastPractised: string | null;
  /** When it should next be reviewed, ISO-8601 UTC. Sorts as text. */
  due: string;
  intervalDays: number;
  ease: number;
  repetitions: number;
  /** Recent attempts, oldest first. */
  history: Attempt[];
  /** Whether the due date has already passed. */
  dueNow: boolean;
}

export interface ProgressView {
  /** Every practised character, most overdue first. A character absent from
   * this list has never been attempted. */
  cards: ProgressCard[];
  /** Set when a change was kept in memory but not saved. */
  warning: string | null;
}

/** Which of the two sources a due character came from. */
export type ReviewSource = "course" | "vocabulary";

/**
 * One thing to review.
 *
 * For a word this describes the entry, not just the character that came due: a
 * word is practised whole, so it appears once however many of its characters
 * are due.
 */
export interface ReviewItem {
  /** The character whose card came due. */
  ch: string;
  due: string;
  source: ReviewSource;
  /** The vocabulary entry, when the character belongs to one. */
  entryId: number | null;
  /** The text to write: an entry's word, or the character itself. */
  text: string;
}

export interface ReviewView {
  items: ReviewItem[];
  /** How many items are due in total; `items` is capped to one session. */
  dueCount: number;
  warning: string | null;
}

/** Where the reader was in the course. */
export interface CursorView {
  index: number;
  updatedAt: string | null;
  warning: string | null;
}

/**
 * One thing the board is asking for: a single character, or a word written one
 * character at a time. Both the vocabulary list and a review session hand the
 * board a queue of these, so practice has one path.
 */
export interface PracticeItem {
  text: string;
  /** The vocabulary entry this came from, or null for a course character. */
  entryId: number | null;
  pinyin: string;
  meaning: string;
}
