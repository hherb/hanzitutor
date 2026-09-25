/**
 * The shapes the Rust side sends.
 *
 * Three of them — `ToneAttempt`, `ToneVerdict` and the syllable/result pair —
 * are the field names of the full app's tone panel, deliberately and not by
 * accident: the pitch chart in `lib/ToneChart.svelte` is lifted from that panel
 * and reads exactly these, so keeping the names identical is what makes the
 * lift possible and what would make the two drift if anyone renamed one side.
 * They are re-declared here rather than imported because the two apps are
 * separate npm projects; a shared package is the obvious next step and is noted
 * in `HANDOVER.md`.
 */

/** The bands a 0..=100 score falls into. Same scale and bands as handwriting. */
export type Grade = "excellent" | "good" | "fair" | "poor";

/** What the scorer decided the learner's pitch was. */
export type ToneVerdict = "match" | "off_target" | "uncertain";

/** One character of a minimal pair: a syllable read at one tone. */
export interface ToneSetMember {
  ch: string;
  /** The reading with its tone mark, e.g. `"mǎ"`. */
  reading: string;
  /** `1`..`4`. */
  tone: number;
  definition: string;
  /** The character's frequency rank; lower is more common. */
  rank: number;
}

/** Characters that differ only in tone: one syllable, one character per tone. */
export interface ToneSet {
  /** The syllable with its tone marks stripped, e.g. `"ma"`. `ü` stays distinct from `u`. */
  base: string;
  /** Two or more, tone 1 first. */
  members: ToneSetMember[];
}

/** One syllable's worth of a scored attempt. */
export interface ToneAttempt {
  /** The tone that was asked for. */
  expectedTone: number;
  /** The tone the contour looked most like, when there was a contour at all. */
  heardTone: number | null;
  /** 0..=100. */
  score: number;
  grade: Grade;
  verdict: ToneVerdict;
  /** One plain sentence for the learner, worded in Rust. Show it as it comes. */
  detail: string;
  /** The learner's pitch, 0..=1 for drawing, low pitch at 0. */
  contour: number[];
  /** The expected tone's canonical shape, on the same scale. */
  reference: number[];
  medianHz: number;
  rangeSemitones: number;
  voicedMs: number;
  spanMs: number;
}

/** A scored syllable, with what was asked for beside the judgement. */
export interface SyllableResult {
  /** Which syllable this is, counting from 1. */
  position: number;
  ch: string;
  /** The reading as the dictionary writes it, tone mark included, e.g. `"nǐ"`. */
  reading: string;
  /** The dictionary's tone, before sandhi. */
  citation: number;
  /** The tone actually spoken in this word, which is what was scored. */
  spoken: number;
  attempt: ToneAttempt;
}

/** One word of a word family: a whole word, with the reading that is scored. */
export interface WordMember {
  text: string;
  /** The **whole word's** reading, e.g. `"nǐhǎo"` — what resolves a polyphone. */
  reading: string;
  /** The tones as spoken, after sandhi. This is what a recording is scored against. */
  spoken: number[];
  /** The dictionary's tones, before sandhi. */
  citation: number[];
  meaning: string;
  /** Lowest HSK 3.0 level the word appears in, 1..=7. */
  hsk: number;
  /** Estimated frequency. Used for ordering; lower is more useful. */
  rank: number;
}

/**
 * A word family: the words sharing one character from a tone set.
 *
 * Keyed by that character, which is what makes the connection to the tone contrast
 * visible: a learner who has drilled 中/种/重 gets the words built on 中.
 */
export interface WordSet {
  /** The character the family hangs on, a member of a tone set. */
  key: string;
  /**
   * The syllable the family is about: the tone set's base, e.g. `"xin"`.
   *
   * This is what lets the Words list be searched the way the Characters list is —
   * on what a family *is* rather than only on the words inside it.
   */
  base: string;
  /** The contrast in words, e.g. `中 tone 1 · 种 tone 3 · 重 tone 4`. */
  contrast: string;
  /** The words, most useful first. */
  words: WordMember[];
}

/**
 * A scored attempt.
 *
 * `toneScored` is always true here — this app scores one syllable against a tone
 * the learner chose — but the field is kept because the chart reads it and because
 * the honest `false` case is what a word drill would need, the way the full app's
 * longer-text path already uses it.
 */
export interface ScoreResult {
  syllables: SyllableResult[];
  toneScored: boolean;
  verdict: ToneVerdict;
  score: number;
  grade: Grade;
  detail: string;
  sandhiApplied: boolean;
  boundariesMs: number[];
  voicedMs: number;
  spanMs: number;
  medianHz: number;
  /**
   * What a recognition model heard, when one is installed.
   *
   * `null` when no model is installed, which is how the app ships — so this is
   * `null` for most learners and the screen then draws no recognition block at
   * all, rather than an empty one.
   */
  heard: Heard | null;
  /** Why recognition failed, when a model *is* installed and could not be used. */
  heardError: string | null;
}

/** One syllable of a transcription, beside the one the exercise asked for. */
export interface HeardSyllable {
  /** The syllable as heard, plain: `shi`. No tone mark. */
  base: string;
  /**
   * The syllable as the dictionary reads the character the model wrote, tone mark
   * included: `shì`; `""` when the dataset cannot read it.
   *
   * A fact about the model's spelling, not about the learner's voice — shown only
   * on a syllable heard *differently*, where the mark is what identifies the
   * character (`cóng` for 从). See `transcript.ts`.
   */
  reading: string;
  /** The syllable the exercise asked for, the same way: `si`. */
  wanted: string;
  /**
   * The syllable the exercise asked for as the dictionary writes it, tone mark
   * included: `sì`.
   *
   * Paired with `reading`, this is what tells "the right sound at a different
   * tone" (`cóng` for `zhōng`) from "the right sound and the same tone". The
   * comparison never reads it; `transcript.ts` uses it to decide whether showing
   * the model's own character and tone is evidence or noise.
   */
  wantedReading: string;
  /** True when they are the same sound once the tone is set aside. */
  matches: boolean;
}

/**
 * What a speech recogniser made of one recording, read against what was asked for.
 *
 * This answers *which syllable was said*, never *how well*. It is a transcription,
 * and `detail` is written by the Rust side to keep it from being read as a
 * pronunciation score — show that sentence rather than writing another.
 *
 * `sameCount` is what makes a per-syllable comparison meaningful; when it is false
 * the transcription could not be divided one syllable per character and was not
 * compared at all, so nothing here says the learner got it wrong.
 */
export interface Heard {
  /** What was transcribed, in characters. Empty when nothing was recognised. */
  text: string;
  /** The same as plain letters with the syllables spaced: `shi`. */
  base: string;
  syllables: HeardSyllable[];
  /** How many syllables were the sound asked for. */
  matched: number;
  /** True when the transcription had as many syllables as the target. */
  sameCount: boolean;
  /** One plain sentence, worded by the Rust side. */
  detail: string;
}

/**
 * Whether the optional recognition model is installed, and how to describe one
 * that is not.
 *
 * The same shape as the full app's, because it is the same download: both apps
 * point at one upstream model and one pinned digest, so a learner reading either
 * screen is being told the same thing.
 */
export interface AsrStatus {
  /** `absent` — never asked for; `downloading`; `installed`; `failed`. */
  state: "absent" | "downloading" | "installed" | "failed";
  installed: boolean;
  /** The model's name, always present so the screen can describe it up front. */
  model: string;
  /** Where it comes from: this app's only network access. */
  url: string;
  licence: string;
  licenceUrl: string;
  /** How large the download is, in bytes. */
  downloadBytes: number;
  /** How large it becomes once unpacked, in bytes. */
  unpackedBytes: number;
  /** Bytes fetched so far, while `state` is `downloading`. */
  downloaded: number;
  /** Where it is on disk, once it is anywhere. */
  path: string | null;
  /** Why the last attempt failed, when one did. */
  error: string | null;
  /** One plain sentence, worded by the Rust side. */
  detail: string;
}

/** Whether the microphone can be used, and at what rate. */
export interface MicrophoneStatus {
  available: boolean;
  device: string | null;
  sampleRate: number;
  /** What to say when it is not available, or why it might hear nothing. */
  detail: string;
}

/** The app's name, version and licence, for the footer. */
export interface AppInfo {
  name: string;
  version: string;
  licence: string;
}
