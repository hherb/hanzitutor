/**
 * The sentences a verdict card adds under its per-stroke list.
 *
 * The per-stroke list only has a row for each stroke the character is taught
 * with, so it cannot show a stroke the character does not have. The grader holds
 * such an attempt below the good band — a perfect あ with one extra mark scores
 * 74 — and without a sentence saying why, the card showed that number beside a
 * list of nothing but "correct" and "Every stroke the right shape, in the right
 * place, in order." The Chinese app's feedback panel says it in the same words.
 */

import type { GradeReport } from "./types";

/** Why the count is wrong, in a learner's terms, or `null` when it is right. */
export function strokeCountNote(
  report: Pick<GradeReport, "countOk" | "expectedStrokes" | "givenStrokes">,
): string | null {
  if (report.countOk) return null;
  const plural = report.expectedStrokes === 1 ? "" : "s";
  return `This character has ${report.expectedStrokes} stroke${plural}, but you wrote ${report.givenStrokes}.`;
}

/**
 * True when there is nothing to correct: the count is right and every stroke is
 * correct. Only then may the card say every stroke was right — a missing stroke
 * or an extra one is a fault the per-stroke rows alone do not show.
 */
export function everyStrokeFine(report: Pick<GradeReport, "countOk" | "strokes">): boolean {
  return report.countOk && report.strokes.every((s) => s.verdict === "correct");
}
