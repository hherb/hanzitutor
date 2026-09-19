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
  | "faint"
  | "missing";

export type Grade = "excellent" | "good" | "fair" | "poor";

export interface StrokeVerdict {
  refIndex: number;
  /** Index into the attempt's strokes, or null when the stroke was never written. */
  userIndex: number | null;
  verdict: Verdict;
  shape: number;
  position: number;
  /**
   * How much of the ink a correct stroke needs that this one put down, 0..1.
   * The only measure that can see a stroke drawn too thin: the shape score is
   * scale-invariant, so it cannot see width at all.
   */
  ink: number;
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
  /**
   * How much ink was put down, 0..1, averaged over the character's strokes with
   * anything unwritten counting zero. 1 means every stroke laid down as much ink
   * as a correct trace at the canvas pen width; about 0.33 means a pen a third
   * of that width.
   */
  inkScore: number;
  /**
   * How much of the character's own ink was reached, 0..1 — the "you never drew
   * that part" signal. Reported rather than scored: a wobbly but correctly inked
   * stroke also misses part of the outline, and that is a placement fault the
   * position score already covers.
   */
  inkCoverage: number;
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
  /** How many words the HSK dictionary holds. */
  words: number;
  /** How many words sit at each HSK level, lowest first. */
  wordLevels: LevelCount[];
}

/** How many words one HSK level holds. */
export interface LevelCount {
  level: number;
  words: number;
}

// ---- the word dictionary --------------------------------------------------

/**
 * One word from the HSK 3.0 vocabulary.
 *
 * Single characters are deliberately absent: the course already teaches every
 * character with its most common reading, so a word entry is always several
 * characters written in turn.
 */
export interface Word {
  /** The word in simplified characters, e.g. `"学习"`. */
  text: string;
  /**
   * The reading of the whole word, e.g. `"xuéxí"`. Taken from a dictionary, so
   * a polyphonic word is right: 着急 is `zháojí`, not `zhejí`.
   */
  pinyin: string;
  meaning: string;
  /** Lowest HSK 3.0 level the word appears in, 1..=7. */
  hsk: number;
  /** Derived frequency: the rarest character's rank. Ordering only. */
  rank: number;
}

/** One page of a word search, with the number of matches behind it. */
export interface WordSearchView {
  words: Word[];
  /** How many words matched in total; `words` is capped to one page. */
  total: number;
}

export interface GradeOptions {
  resampleK: number;
  minStrokeLen: number;
  globalFit: boolean;
  /**
   * Width, in design units, of the ink the canvas paints the attempt with. The
   * grader rasterises the strokes at this width, so it must be the width the
   * board actually drew them with.
   */
  inkWidth: number;
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
 * The learner's settings.
 *
 * Every field is `null` when nobody has chosen it, which is *not* the same as
 * `false`: the interface resolves an unchosen preference from the device and only
 * writes a value once the learner has flipped the switch themselves.
 */
export interface SettingsView {
  /** `true` click to start and click to finish; `false` press and drag; `null` let the device decide. */
  clickToDraw: boolean | null;
  /** Set when a change was applied in memory but could not be saved. */
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

// ---- what the app is, and what it ships under ------------------------------

/** The app's own identity, shown on the About screen. */
export interface AppInfo {
  name: string;
  version: string;
  identifier: string;
  licence: string;
  copyright: string;
  repository: string;
}

/**
 * One licence or attribution notice, with its full text.
 *
 * `covers` says what the app takes from that work and `bundlePath` names where
 * the plain-text copy lives inside the application bundle, so a reader can
 * check either without leaving the screen. The text itself is compiled into the
 * binary, so it is present even if the bundle's resource copy were lost.
 */
export interface LicenceNotice {
  id: string;
  title: string;
  licence: string;
  source: string;
  covers: string;
  /** Repository-relative path the text came from. */
  file: string;
  /** Where the copy in the application bundle sits. */
  bundlePath: string;
  text: string;
}

// ---- tone practice ---------------------------------------------------------

/** How a spoken syllable was judged. */
export type ToneVerdict = "match" | "off_target" | "uncertain";

/**
 * The result of scoring one spoken syllable.
 *
 * `contour` and `reference` are both on the same 0..1 scale — low pitch at 0 —
 * so they can be drawn over each other as two lines. `reference` is the expected
 * tone's canonical shape and is always present; `contour` is empty when nothing
 * could be heard, which is the only case where there is no learner's line to
 * draw.
 *
 * `score` is 0..100 on the same scale as a handwriting score, and `grade` is the
 * same band, so a tone and a stroke can be styled alike.
 */
export interface ToneAttempt {
  expectedTone: number;
  /** The tone the contour looks most like, or null when there was no contour. */
  heardTone: number | null;
  score: number;
  grade: Grade;
  verdict: ToneVerdict;
  /** One plain sentence, worded by the Rust side. Show this, do not reword it. */
  detail: string;
  contour: number[];
  reference: number[];
  medianHz: number;
  rangeSemitones: number;
  voicedMs: number;
  spanMs: number;
}

/**
 * One syllable of a word, with the tone to score it against.
 *
 * `citation` is what a dictionary prints and `spoken` is what is actually said:
 * they differ when tone sandhi applies, which is why both are sent. 你好 is
 * `3 + 3` in a dictionary and `2 + 3` out loud.
 */
export interface TargetSyllable {
  ch: string;
  /** The reading as a dictionary writes it, tone mark included. */
  reading: string;
  citation: number;
  spoken: number;
}

/** The tones a character or word should be practised with. */
export interface ToneTarget {
  syllables: TargetSyllable[];
  /** True when sandhi changed a tone, so the interface can say why. */
  sandhiApplied: boolean;
  detail: string;
}

/** One syllable's worth of a scored utterance. */
export interface ToneSyllableResult extends TargetSyllable {
  /** Which syllable this is, counting from 1. */
  position: number;
  attempt: ToneAttempt;
}

/**
 * A scored character or word.
 *
 * `syllables` always has one entry per syllable of the target, whatever was
 * heard, so the interface can pair them with the characters without guessing.
 */
export interface ToneResult {
  syllables: ToneSyllableResult[];
  verdict: ToneVerdict;
  /** Mean of the scoreable syllables, 0..100; 0 when none could be scored. */
  score: number;
  grade: Grade;
  /** One plain sentence, worded by the Rust side. Style it; do not reword it. */
  detail: string;
  sandhiApplied: boolean;
  /**
   * Where the syllables were divided, in ms **from the start of speech**.
   * Empty for a single syllable. Same baseline as `voicedMs`, so a boundary can
   * be read against it — the recording itself starts whenever the button was
   * pressed, which is mostly the learner's own pause.
   */
  boundariesMs: number[];
  voicedMs: number;
  spanMs: number;
  medianHz: number;
}

/**
 * Whether the microphone can be used.
 *
 * `available` cannot see a *denied permission*: the system still hands out a
 * device and the stream then delivers silence. That shows up as a tone attempt
 * with no contour, which is reported as "I could not hear enough voice".
 */
export interface MicrophoneStatus {
  available: boolean;
  device: string | null;
  sampleRate: number;
  detail: string;
}
