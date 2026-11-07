/**
 * Which pointer is drawing, and what the stroke it draws is made of.
 *
 * **Copied verbatim to `apps/nihongo-tutor/src/lib/strokeInput.ts`**, the way
 * `render.ts` is (HANDOVER_NIHONGO §4.6), and `strokeInput.test.ts` runs the same
 * cases against both copies so the two boards cannot drift apart in behaviour.
 *
 * Both boards used to start a fresh stroke for *any* pointer that went down and
 * extend it from any pointer that moved (issue #30). On a tablet that is exactly
 * wrong: a palm resting on the glass or a second finger landing while the pencil
 * writes threw the stroke in progress away, the two pointers' samples then
 * interleaved into one zig-zag, and the first one lifted committed it for
 * grading. So a stroke here belongs to **one pointer**, chosen on the way down:
 *
 * - **A non-primary pointer never draws.** A second finger is never the primary
 *   touch, so it cannot interrupt the first.
 * - **While a pointer draws, a pointer of another kind is ignored** — a palm
 *   (a touch, and the *first* touch, so primary) while the pencil writes — **except
 *   that a pen takes over from a touch.** The palm usually lands first; the pencil
 *   is what the learner is writing with. The touch's stroke is thrown away, and
 *   the caller is told which pointer to release.
 * - **A touch that lands just after the pen lifts is ignored** — for
 *   [`PEN_GRACE_MS`]. Between two pencil strokes the hand stays on the glass, and
 *   with no pen down at that moment there is nothing for the pen to win against,
 *   so the palm would draw a stroke of its own. A finger after the grace period
 *   draws as it always did, and a learner who never uses a pen never meets it.
 * - **A primary pointer of the same kind going down again starts again.** There is
 *   only ever one primary pointer of a kind, so the first one's release must have
 *   been lost; waiting for it would leave the board deaf. This is what the boards
 *   did for every pointer before, kept for the one case where it is right.
 *
 * How a stroke ends matters as much:
 *
 * - **A release adds the position it happened at**, because a quick flick can
 *   deliver no move at all between the press and the lift.
 * - **A lone point is doubled**, so a tap is a two-point dot rather than a
 *   one-point mark that a board or a grader drops on sight. Whether a dot that
 *   small is a stroke is the grader's call (its stray-tap filter), not the input's.
 * - **A cancel throws the stroke away.** `pointercancel` is the platform taking
 *   the pointer back — an edge swipe, palm rejection, the notification shade —
 *   and the stroke it interrupted is not what the learner meant to draw.
 * - **A lost capture finishes the stroke with what it has.** The browser sends
 *   `lostpointercapture` *after* the release or the cancel, by which time the
 *   stroke is already over; arriving first, it means the release will never
 *   come, and keeping the ink is the failure that loses nothing.
 *
 * Pointer events arrive as plain objects already mapped into display space, so
 * this module never sees the DOM and the root project's vitest can run it. Time
 * arrives the same way — as the event's own `timeStamp` — so the grace period is
 * arithmetic on what the caller passed, not a clock this module reads.
 *
 * [`isDrawnStroke`] is the other half of a tap: the boards keep one on the board
 * (it is ink the learner put down, and Undo takes it back), but a screen should
 * not offer to grade an attempt that holds nothing else.
 */

/** A position on the board, in display space. */
export interface StrokePoint {
  x: number;
  y: number;
}

/** The parts of a `PointerEvent` that decide who draws, with its position. */
export interface PointerSample extends StrokePoint {
  pointerId: number;
  /** `"mouse"`, `"pen"` or `"touch"`, as `PointerEvent.pointerType` reports it. */
  pointerType: string;
  isPrimary: boolean;
  /**
   * When it happened, in milliseconds — `PointerEvent.timeStamp`. Optional: the
   * pen's grace period needs it, and without it simply does not apply.
   */
  t?: number;
}

/** How long after a pen lifts a touch is taken for the hand that held it, in ms. */
export const PEN_GRACE_MS = 1000;

export interface StrokeInputOptions {
  /** Drop a sample closer than this to the one before it. `0` keeps every one. */
  minSpacing?: number;
  /** Stop growing a stroke once it has this many points. */
  maxPoints?: number;
  /** Override [`PEN_GRACE_MS`]; `0` turns the grace period off. */
  penGraceMs?: number;
}

/**
 * What a press did.
 *
 * `release` is a pointer whose stroke this press threw away — a touch a pen took
 * over from, or a pointer whose release was lost — and whose capture the caller
 * should let go of. It is `null` when there is nothing to release.
 */
export type Down = { accepted: false } | { accepted: true; release: number | null };

const IGNORED: Down = { accepted: false };

export class StrokeInput {
  private stroke: StrokePoint[] | null = null;
  private activeId: number | null = null;
  private activeType: string | null = null;
  /** When the last pen stroke ended, if a timestamp said. */
  private penUpAt: number | null = null;
  private readonly minSpacing: number;
  private readonly maxPoints: number;
  private readonly penGraceMs: number;

  constructor(options: StrokeInputOptions = {}) {
    this.minSpacing = options.minSpacing ?? 0;
    this.maxPoints = options.maxPoints ?? Infinity;
    this.penGraceMs = options.penGraceMs ?? PEN_GRACE_MS;
  }

  /** The stroke in progress, for painting, or `null` when there is none. */
  get points(): readonly StrokePoint[] | null {
    return this.stroke;
  }

  /** The pointer drawing the stroke in progress, or `null`. */
  get pointerId(): number | null {
    return this.activeId;
  }

  /** A pointer went down: start its stroke, or ignore it. */
  down(sample: PointerSample): Down {
    if (!sample.isPrimary) return IGNORED;
    const active = this.activeId;
    if (active !== null) {
      const sameKind = sample.pointerType === this.activeType;
      const penOverTouch = sample.pointerType === "pen" && this.activeType === "touch";
      if (!sameKind && !penOverTouch) return IGNORED;
    } else if (sample.pointerType === "touch" && this.inPenGrace(sample.t)) {
      return IGNORED;
    }
    this.activeId = sample.pointerId;
    this.activeType = sample.pointerType;
    this.stroke = [];
    this.push(sample);
    return { accepted: true, release: active !== null && active !== sample.pointerId ? active : null };
  }

  /**
   * A pointer moved. Returns whether the stroke took it.
   *
   * `coalesced` is what `getCoalescedEvents()` returned, already in display
   * space: the full-rate samples the browser batched into this one move. When it
   * is empty or absent the move itself is the sample.
   */
  move(sample: PointerSample, coalesced: readonly StrokePoint[] = []): boolean {
    if (!this.owns(sample.pointerId)) return false;
    for (const point of coalesced.length > 0 ? coalesced : [sample]) this.push(point);
    return true;
  }

  /** A pointer was lifted. Returns the finished stroke if it was drawing one. */
  up(sample: PointerSample): StrokePoint[] | null {
    if (!this.owns(sample.pointerId)) return null;
    this.push(sample);
    this.ended(sample.t);
    return this.take();
  }

  /**
   * A pointer was cancelled, at `t` if the caller knows. Returns whether a stroke
   * was thrown away.
   */
  cancel(pointerId: number, t?: number): boolean {
    if (!this.owns(pointerId)) return false;
    this.ended(t);
    this.discard();
    return true;
  }

  /**
   * A pointer lost its capture, at `t` if the caller knows. Returns the stroke it
   * was still drawing, if any.
   */
  lost(pointerId: number, t?: number): StrokePoint[] | null {
    if (!this.owns(pointerId)) return null;
    this.ended(t);
    return this.take();
  }

  /**
   * Open a stroke that no pointer holds — Hanzi Tutor's click to draw, which
   * extends a stroke by hover between two clicks that need not share a pointer.
   */
  begin(point: StrokePoint) {
    this.activeId = null;
    this.activeType = null;
    this.stroke = [];
    this.push(point);
  }

  /** Extend whatever stroke is open. */
  extend(point: StrokePoint) {
    this.push(point);
  }

  /** Close whatever stroke is open, at `point` if given. */
  finish(point?: StrokePoint): StrokePoint[] | null {
    if (point) this.push(point);
    return this.take();
  }

  /** Throw away whatever stroke is open. */
  discard() {
    this.stroke = null;
    this.activeId = null;
    this.activeType = null;
  }

  /** The drawing pointer's stroke is ending; remember when, if it was a pen. */
  private ended(t: number | undefined) {
    if (this.activeType === "pen" && t !== undefined) this.penUpAt = t;
  }

  private inPenGrace(t: number | undefined): boolean {
    if (t === undefined || this.penUpAt === null) return false;
    const since = t - this.penUpAt;
    return since >= 0 && since < this.penGraceMs;
  }

  private owns(pointerId: number): boolean {
    return this.stroke !== null && this.activeId === pointerId;
  }

  private push(point: StrokePoint) {
    const stroke = this.stroke;
    if (!stroke || stroke.length >= this.maxPoints) return;
    const last = stroke[stroke.length - 1];
    if (last && Math.hypot(point.x - last.x, point.y - last.y) < this.minSpacing) return;
    // Only the coordinates: this array is what goes to the grader.
    stroke.push({ x: point.x, y: point.y });
  }

  private take(): StrokePoint[] | null {
    const stroke = this.stroke;
    this.discard();
    if (!stroke || stroke.length === 0) return null;
    if (stroke.length === 1) stroke.push({ ...stroke[0] });
    return stroke;
  }
}

/**
 * How far a stroke must travel, in design units, to be more than a tap.
 *
 * Deliberately far below the grader's stray-tap threshold — `min(12, half the
 * shortest reference stroke)`, which the shortest stroke in the Chinese data (4
 * units, in 黧) puts at 2 — so this can only disagree with the grader in the safe
 * direction: it never refuses a mark the grader would keep as a stroke. It exists
 * to tell a tap (coincident points, or sub-pixel jitter: one unit is under half a
 * pixel on a 400-pixel board) from ink, not to second-guess the grader, which
 * still has the last word on a mark between the two.
 */
export const DRAWN_MIN_LENGTH = 1;

/** Whether a stroke travelled far enough to be more than a tap. */
export function isDrawnStroke(points: readonly StrokePoint[]): boolean {
  let length = 0;
  for (let i = 1; i < points.length; i++) {
    length += Math.hypot(points[i].x - points[i - 1].x, points[i].y - points[i - 1].y);
    if (length >= DRAWN_MIN_LENGTH) return true;
  }
  return false;
}

/**
 * Whether an attempt holds anything to grade: at least one stroke that is more
 * than a tap. A screen offers Grade on this rather than on a stroke count, so a
 * lone accidental tap cannot be graded — and scored zero — on the learner's
 * behalf.
 */
export function hasDrawnStroke(strokes: readonly (readonly StrokePoint[])[]): boolean {
  return strokes.some(isDrawnStroke);
}
