/**
 * The board's arithmetic, as pure functions.
 *
 * This is the part of the canvas that can be wrong without looking wrong: a
 * pointer mapped into the wrong space draws a stroke in the wrong place, and the
 * animation's progress arithmetic decides which stroke the pen is on. Both are
 * simple enough to be obviously right and easy enough to be subtly wrong, so
 * they live here — plain TypeScript, no canvas and no DOM — where the root
 * project's vitest can check them.
 */

import { BOX } from "./render";
import type { Point } from "./types";

/** A pointer position, as the browser reports it. */
export interface ClientPoint {
  clientX: number;
  clientY: number;
}

/** The rectangle a canvas occupies on screen. */
export interface ClientRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * Where a pointer landed, in **display space** — the 1024×1024 box with y
 * increasing downwards that the grader works in.
 *
 * The canvas is square and its backing store is scaled for the device pixel
 * ratio, but the pointer arrives in CSS pixels, so the mapping is by the
 * element's own client rectangle. A degenerate rectangle (zero width, which is
 * what an unlaid-out or hidden canvas reports) would divide by zero and produce
 * `NaN` points that poison the whole attempt, so it yields the centre instead.
 */
export function pointerToDisplay(
  point: ClientPoint,
  rect: ClientRect,
  box: number = BOX,
): Point {
  if (!(rect.width > 0) || !(rect.height > 0)) {
    return { x: box / 2, y: box / 2 };
  }
  return {
    x: ((point.clientX - rect.left) / rect.width) * box,
    y: ((point.clientY - rect.top) / rect.height) * box,
  };
}

/** Where the pen is part-way through the stroke-order animation. */
export interface SweepPosition {
  /** The stroke being drawn, as a reference index. */
  index: number;
  /** How far along that stroke's centre-line, 0..=1. */
  progress: number;
}

/**
 * The pen position after `elapsed` milliseconds of a sweep over `strokes`
 * strokes that takes `totalMs` in all.
 *
 * Worked out from elapsed time rather than by counting frames, so a dropped frame
 * shortens the animation instead of slowing it down and the whole thing takes the
 * same time on a slow machine. When the time is up it reports the last stroke
 * complete rather than running off the end, so a caller that keeps calling it
 * cannot index out of bounds.
 */
export function sweepAt(
  elapsed: number,
  strokes: number,
  totalMs: number,
): SweepPosition {
  if (strokes <= 0) return { index: 0, progress: 1 };
  const each = totalMs / strokes;
  if (!(elapsed > 0)) return { index: 0, progress: 0 };
  if (elapsed >= totalMs) return { index: strokes - 1, progress: 1 };
  const index = Math.min(strokes - 1, Math.floor(elapsed / each));
  const progress = Math.min(1, (elapsed - index * each) / each);
  return { index, progress };
}

/**
 * Whether a stroke is long enough to be a stroke.
 *
 * A tap or a stray click records one or two points and would otherwise be
 * submitted as an attempt, so the board drops it. The threshold is in design
 * units out of 1024, and it is deliberately tiny: this is a filter for
 * accidents, not a judgement about handwriting, which is the grader's business.
 */
export const MIN_POINTS = 2;

export function isStroke(points: Point[]): boolean {
  return points.length >= MIN_POINTS;
}

/**
 * What the board's own primary control offers **after** an attempt has been
 * graded.
 *
 * `grade` is the control that asks for a verdict; once there is one, the same
 * place holds the way on. That is Hanzi Tutor's arrangement — its commit button
 * becomes `Next` — and the reason it is wanted here is the hand: the learner has
 * just pressed the button at the bottom of the screen, and reaching back to the
 * lesson's arrows in the top corner to move on is a journey the verdict itself
 * can save.
 *
 * The corner arrows keep their own job, which is **skipping**: they step through
 * the lesson without grading, and so leave this control reading `grade` until
 * something has been judged.
 *
 * `grade` is also the answer for a radical head form, which belongs to no lesson:
 * there is nothing to advance to and nothing to finish. `finish` is the end of a
 * lesson — there is no next character, and the way on is back to the course — and
 * it is Hanzi Tutor's rule for the end of an entry rather than a new one.
 */
export type AfterGrade = "grade" | "next" | "finish";

export function afterGrade(graded: boolean, inLesson: boolean, hasNext: boolean): AfterGrade {
  if (!graded || !inLesson) return "grade";
  return hasNext ? "next" : "finish";
}

/**
 * The same sentence for the **review queue**, whose end is a state rather than an
 * action.
 *
 * The queue is a board with no lesson behind it: every item in it is something to
 * write, and when the last of them has been graded there is nothing to advance to
 * *and* nothing to go back to, so the control says so — `nothing`, which the
 * maintainer's word for it is "nothing due" — instead of the stage's `finish`.
 *
 * `hasNext` is asked of the **refreshed** queue: `grade` reloads it, and an
 * attempt that counted as a review takes its character out of the due list, so
 * what is left is what comes next. It is also asked of the queue rather than of
 * the page: a queue with more due than one page holds has a next item even when
 * the page has run out, which is why the caller counts both.
 */
export type AfterQueue = "grade" | "next" | "nothing";

export function afterGradeInQueue(graded: boolean, hasNext: boolean): AfterQueue {
  if (!graded) return "grade";
  return hasNext ? "next" : "nothing";
}
