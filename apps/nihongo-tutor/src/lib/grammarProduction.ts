/**
 * The production exercise's arithmetic, as pure functions.
 *
 * The same split the other screens use: what can be decided without a window lives
 * here and is unit-tested by the root project's vitest, and the component only draws.
 * Three of them are rules rather than arithmetic.
 *
 * `outcomeLine` says what the answer came to, and it is the one line the learner reads
 * first once Rust has graded what they wrote — so it names **both** forms: the reading
 * the corpus gives the hole (what had to be typed) and the text the sentence writes
 * there (what the reveal puts back). They are the same string for a hole that holds no
 * kanji, and saying "written の方が" beside "のほうが" would be noise, so that case says
 * it once. `wholeSentence` is what `Hear it` speaks and what the reveal draws: the two
 * halves the question came in with the corpus's own text put back between them.
 * `poolNote` states how much of the course can be written — the same measured reach the
 * recognition drill has, because a question with no options needs no distractors.
 */

import type { GrammarDrillSize, GrammarProductionQuestion, GrammarProductionVerdict } from "./types";

/** Whether there is anything to send back. */
export function canSubmit(typed: string): boolean {
  return typed.trim() !== "";
}

/**
 * The sentence whole again, with the text that was taken out put back.
 *
 * The question arrives in two halves — the corpus's own furigana on each — so the
 * reveal is that string, and `Hear it` speaks it. Nothing is composed: the half the
 * learner could not see is the corpus's own text.
 */
export function wholeSentence(question: GrammarProductionQuestion, written: string): string {
  return (
    question.before.map((part) => part.ruby).join("") +
    written +
    question.after.map((part) => part.ruby).join("")
  );
}

/**
 * What the answer came to, in one line, whatever it was.
 *
 * Right: the answer, so a correct recall is still a lesson. Wrong: the answer *and*
 * what was written instead, because "no" without "it was のほうが" teaches nothing.
 * An answer the input engine could read none of says so rather than echoing an empty
 * string — a learner who typed `zzz` should be told what happened, not shown `you
 * wrote `.
 */
export function outcomeLine(verdict: GrammarProductionVerdict): string {
  const hole =
    verdict.written === verdict.answer
      ? `${verdict.answer}`
      : `${verdict.answer}, written ${verdict.written}`;
  if (verdict.correct) {
    return `Yes — the hole is ${hole}.`;
  }
  if (verdict.produced === "") {
    return `Not quite — the hole is ${hole}. Nothing you typed could be read as kana.`;
  }
  return `Not quite — the hole is ${hole}. You wrote ${verdict.produced}.`;
}

/**
 * How much of the course can be written, in one sentence.
 *
 * The numbers are the artifact's, and the same reach the recognition drill reports:
 * this exercise needs no distractors, so every sentence the blank rule finds is asked
 * here too — and every one of those answers is typeable, which is the measurement that
 * makes the pool the same pool. The reason the other patterns are unreachable is worth
 * stating as well: the rule finds a question only where the pattern's own text is in
 * the sentence, once, and not inside a furigana pair.
 */
export function poolNote(size: GrammarDrillSize): string {
  if (size.total <= 0 || size.points <= 0) {
    return "No pattern in this course can be written this way yet.";
  }
  if (size.points >= size.total) {
    return `Every one of the ${size.total} patterns can be written, from ${size.examples} sentences.`;
  }
  return `${size.points} of the ${size.total} patterns can be written, from ${size.examples} sentences — the rest have no sentence their own text can be found in, once, whole.`;
}
