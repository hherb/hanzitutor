/**
 * The grammar drill's arithmetic, as pure functions.
 *
 * The same split the other screens use: what can be decided without a window lives
 * here and is unit-tested by the root project's vitest, and the component only draws.
 *
 * Three of these are rules rather than arithmetic. `optionState` is what makes a
 * question *judged* rather than merely answered — which option is coloured right and
 * which wrong, decided from the two ids and nothing else, because the screen is not
 * told whether the answer was right and must not be. `poolNote` states how much of the
 * course the drill can reach: 163 of the 526 patterns have no sentence their own text
 * can be found in, and a learner who goes looking for one of them should meet that
 * fact rather than a silence. `recordLine` is the same sentence the kana drill puts
 * under an answer, over this drill's own tally.
 */

import type { GrammarDrillSize, GrammarOption, GrammarQuestion, GrammarTally } from "./types";

/** How one option should be drawn once an answer has been picked. */
export type OptionState = "idle" | "right" | "wrong";

/**
 * Whether this option is the answer, the one that was picked, or neither.
 *
 * `picked` is null before an answer, and every option is then idle — which is the
 * whole point: the screen must not colour the answer before it is chosen, even though
 * the payload names it.
 */
export function optionState(
  option: GrammarOption,
  question: GrammarQuestion,
  picked: string | null,
): OptionState {
  if (picked === null) return "idle";
  if (option.id === question.point) return "right";
  return option.id === picked ? "wrong" : "idle";
}

/** Whether the answer that was picked is the pattern the sentence takes. */
export function isRight(question: GrammarQuestion, picked: string): boolean {
  return picked === question.point;
}

/** The pattern the learner picked, as the corpus writes it, or null if it is not an option. */
export function pickedPattern(question: GrammarQuestion, picked: string | null): string | null {
  if (picked === null) return null;
  return question.options.find((option) => option.id === picked)?.pattern ?? null;
}

/**
 * What the answer says, in one line, whatever it was.
 *
 * Right: the pattern and what it means, so a correct guess is still a lesson. Wrong:
 * what was picked, then the pattern the sentence takes and what it means — never the
 * pattern alone, because the point of the drill is the meaning it carries.
 */
export function outcomeLine(question: GrammarQuestion, picked: string): string {
  if (isRight(question, picked)) {
    return `Yes — ${question.pattern} means ${question.meaning}.`;
  }
  const wrong = pickedPattern(question, picked) ?? "That";
  return `${wrong} is not it — the sentence takes ${question.pattern}, ${question.meaning}.`;
}

/**
 * The session's score, which the drill keeps for the sitting only.
 *
 * No percentage before the first answer, because "0 / 0 · 0%" is a number about
 * nothing.
 */
export function sessionLine(right: number, asked: number): string {
  if (asked <= 0) return "Nothing answered yet";
  const percent = Math.round((right / asked) * 100);
  return `${right} / ${asked} · ${percent}%`;
}

/**
 * What the record now says about this pattern, and what it means for what comes next.
 *
 * The rule is `nihongo_core::drill`'s — a miss makes a pattern three times as likely
 * at first, a right answer takes it down — and saying it is what makes the weighting a
 * promise rather than a mystery.
 */
export function recordLine(tally: GrammarTally, pattern: string): string {
  if (tally.wrong > 0) {
    return `${pattern}: missed ${tally.wrong} of ${tally.asked} — this pattern comes up more often until you stop missing it.`;
  }
  return `${pattern}: ${tally.correct} of ${tally.asked} right — this pattern comes up less often now.`;
}

/**
 * How much of the course the drill can ask, in one sentence.
 *
 * The numbers are the artifact's, measured rather than promised (363 of the 526
 * patterns, over 903 of the 1,526 sentences at the time of writing), and the reason
 * the other patterns are unreachable is worth stating too: the rule finds a question
 * only where the pattern's own text is in the sentence, once, and not inside a
 * furigana pair.
 */
export function poolNote(size: GrammarDrillSize): string {
  if (size.total <= 0 || size.points <= 0) {
    return "No pattern in this course can be asked this way yet.";
  }
  if (size.points >= size.total) {
    return `Every one of the ${size.total} patterns can be asked, over ${size.examples} sentences.`;
  }
  return `${size.points} of the ${size.total} patterns can be asked, over ${size.examples} sentences — the rest have no sentence their own text can be found in, once, whole.`;
}

/**
 * What there is to say when a grammar stage's draw comes back empty, instead of the
 * "Loading…" a `null` used to leave on screen for good (issue #34).
 *
 * Both draws are `null` only when no pattern of the course has a sentence they can
 * ask about — `next_grammar_question` needs the pattern's own text in the sentence
 * once, whole, with two other patterns safe to offer beside it, and
 * `next_grammar_production` needs that hole to be a typeable answer — so a second
 * draw would find nothing either, and the way forward is another exercise. The two
 * stages share the rule here as they share `recordLine` and `sessionLine`; `write`
 * is the production stage. The count is the course's, when the size has arrived.
 */
export function emptyNote(size: GrammarDrillSize | null, exercise: "choose" | "write"): string {
  const nothing = exercise === "choose" ? "Nothing to ask here" : "Nothing to write here";
  if (size && size.total <= 0) {
    return `${nothing}: the grammar course holds no patterns.`;
  }
  const which = size ? `none of the course's ${size.total} patterns` : "no pattern of the course";
  const can =
    exercise === "choose"
      ? "this drill can make a question of"
      : "this exercise can take a typeable answer out of";
  return `${nothing}: ${which} has a sentence ${can}. Try another exercise from the grammar course.`;
}
