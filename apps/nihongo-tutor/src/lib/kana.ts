/**
 * The kana screens' arithmetic, as pure functions.
 *
 * The chart, the kana course, the stage a lesson opens and the drill all offer
 * kana, and offering one is a promise: tapping it opens it on the board. Four
 * small rules make that promise, and none of them is a component concern — which
 * is why they live here, where they can be tested without a browser (the root
 * project's vitest runs these, see `HANDOVER_NIHONGO.md` §5).
 *
 * `lessonOf`, `lessonLabel`, `neighbourIn` and `positionIn` are the kana course's
 * own: which lesson teaches a kana, what to call that lesson on a card, and how
 * the arrows either side of the board step through it. They are `kanji.ts`'s
 * rules for the kana payload rather than a second implementation of them — the
 * two lessons name their characters differently (`kana` against `kanji`), and
 * that field is the whole of the difference.
 */

import type { ChartView, LessonView } from "./types";

/**
 * The lesson a kana is taught in, so a kana opened from another screen — or
 * stepped to inside the stage — lands with **its own** lesson's arrows rather
 * than the first lesson's.
 *
 * `null` when the course does not teach it. Every kana the chart offers is in
 * some lesson (invariant 29), so a miss is a caller mid script switch rather than
 * a kana with no home, and the stage treats it as "no arrows" rather than as an
 * error.
 */
export function lessonOf(course: LessonView[], ch: string): LessonView | null {
  return course.find((lesson) => lesson.kana.includes(ch)) ?? null;
}

/**
 * What to call a lesson on the card that offers it: the row's sound, or the
 * off-grid group's label.
 *
 * A lesson's title is written as `"<one half> — <the other half>"`, and **which
 * half is the sound depends on the kind of lesson it is**: a row of the gojūon
 * grid is `"あ い う え お — a i u e o"` and an off-grid group is
 * `"Small kana — ゃ ゅ ょ っ"`. Reading the first half as the name, which is what
 * a `split(" — ")[0]` does, calls the small kana a lesson and ゃ ゅ ょ っ its
 * sound.
 *
 * So the half that is *not* the lesson's own kana is the name, and it is decided
 * from the data rather than from a second table of group names in the interface:
 * the title's kana is exactly `kana.join(" ")`, and the other half is the label.
 */
export function lessonLabel(lesson: LessonView): string {
  const [head = "", tail = ""] = lesson.title.split(" — ");
  const drawn = lesson.kana.join(" ");
  if (head === drawn) return tail;
  if (tail === drawn) return head;
  // Neither half is the drawn kana: a title shape this does not know. The first
  // half is what the interface used to show, so that is what it still shows.
  return head || tail;
}

/**
 * The kana `delta` places from `ch` inside its lesson — the arrow either side of
 * the board.
 *
 * `null` at either end rather than wrapping, for `kanji.ts`'s reason: a lesson is
 * a sequence a learner works through, and an arrow that jumped from the last kana
 * back to the first would hide the fact that the lesson is finished. A kana the
 * lesson does not hold, and a lesson of one, both answer `null` — there is
 * nowhere to step to.
 */
export function neighbourIn(
  lesson: LessonView | null,
  ch: string,
  delta: number,
): string | null {
  if (!lesson || delta === 0) return null;
  const at = lesson.kana.indexOf(ch);
  if (at < 0) return null;
  return lesson.kana[at + delta] ?? null;
}

/** Where `ch` sits in its lesson, 1-based, for the stage that says `3 / 5`. */
export function positionIn(lesson: LessonView | null, ch: string): number {
  if (!lesson) return 0;
  const at = lesson.kana.indexOf(ch);
  return at < 0 ? 0 : at + 1;
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

/**
 * The taught strokes the grader read as drawn joined, as a label — `"1+2"` for a
 * さ written in two strokes — or `null` when the attempt was written the way it
 * is taught.
 *
 * The verdict panel lists the **drawn** strokes by number, so without this line
 * "stroke 3" of a き written joined would name taught strokes 3 and 4. `null`
 * rather than an empty string, so a caller leaves the line out rather than
 * drawing an empty one.
 */
export function joinedLabel(joined: number[][]): string | null {
  if (joined.length === 0) return null;
  return joined.map((group) => group.join("+")).join(", ");
}
