/**
 * The kana screens' arithmetic, as pure functions.
 *
 * The chart, the practice course and the drill all offer kana, and offering one is
 * a promise: tapping it opens it on the board. Three small rules make that
 * promise, and none of them is a component concern — which is why they live here,
 * where they can be tested without a browser (the root project's vitest runs
 * these, see `HANDOVER_NIHONGO.md` §5).
 */

import type { ChartView, LessonView, ScriptName } from "./types";

/**
 * The lesson a kana belongs to, so a kana opened from another screen lands with
 * its own row highlighted rather than the first lesson's.
 *
 * `null` when the course does not teach it, which a caller treats as "leave the
 * sidebar where it is" rather than as an error: the chart's off-grid groups hold
 * kana the lessons do teach, but a caller with a stale course is a caller mid
 * script switch, not a caller with a bug.
 */
export function lessonKeyOf(course: LessonView[], ch: string): string | null {
  return course.find((lesson) => lesson.kana.includes(ch))?.key ?? null;
}

/**
 * Which kana a freshly loaded course should open on.
 *
 * `asked` is what another screen requested and the script it belongs to, and
 * `loaded` is the script the course being opened is for. The pending request is
 * honoured **only when the two scripts agree**, and the first kana of the course
 * is the fallback — including for a stale request, which is the case this exists
 * for: a request left over from a script whose load failed, or from a tap that a
 * later one overtook, must not be applied to a course it does not belong to. That
 * would open one script's kana under the other's lessons, which is worse than
 * opening the first kana and saying nothing.
 *
 * Pure, and here rather than inline, because it is the rule the race is about and
 * a component cannot be tested in this app (the frontend suite has no DOM).
 */
export function focusFor(
  asked: { ch: string; script: ScriptName } | null,
  loaded: ScriptName,
  course: LessonView[],
): string | null {
  if (asked && asked.script === loaded) return asked.ch;
  return course[0]?.kana[0] ?? null;
}

/**
 * How many columns the chart's grid has.
 *
 * Read from the rows rather than written down, because it is also what the CSS
 * grid is told. The backend always sends five — a, i, u, e, o — and a row of a
 * different length would otherwise be drawn as if it had five.
 */
export function chartColumns(view: ChartView): number {
  return view.rows.reduce((widest, row) => Math.max(widest, row.cells.length), 0);
}

/**
 * Whether a drill answer is a single kana, which is what the board can open.
 *
 * A yōon answer is a digraph: きゃ is two characters and two strokes' worth of
 * shape, and the board grades one character at a time. So the drill offers "write
 * it" for the classic pairs and not for the contrasts, rather than offering a
 * button that would open half an answer.
 */
export function isSingleKana(spelling: string): boolean {
  return [...spelling].length === 1;
}
