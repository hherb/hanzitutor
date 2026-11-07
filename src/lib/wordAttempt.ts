/**
 * What a finished vocabulary word adds up to.
 *
 * A word is written one character at a time, each graded on its own board, and
 * the entry remembers the mean. Pulled out of `finishCharacter` so the decision
 * is taken from the word's *own* slots — that function moves the board to the
 * next entry, and reading the slots afterwards read the next word's (empty)
 * ones, so a traced word was always recorded as written from memory.
 */
import type { PracticeMode } from "./types";

/** One character's graded attempt, as the word's slot strip holds it. */
export interface GradedSlot {
  report: { overall: number };
  /** How the board was set when the character was graded. */
  mode?: PracticeMode;
}

export interface WordAttempt {
  /** Each character's headline score, in the word's order. */
  scores: number[];
  /** Their mean: what the entry records. */
  mean: number;
  /**
   * True when any character was traced over the guide. Such a word was not
   * written from memory, so the entry's attempt count and best score are left
   * alone, as the schedule leaves its cards.
   */
  traced: boolean;
}

/** Summarise a word whose every character has been graded. */
export function summariseWord(slots: readonly GradedSlot[]): WordAttempt {
  const scores = slots.map((slot) => slot.report.overall);
  const total = scores.reduce((sum, value) => sum + value, 0);
  const mean = scores.length === 0 ? 0 : total / scores.length;
  return { scores, mean, traced: slots.some((slot) => slot.mode === "trace") };
}
