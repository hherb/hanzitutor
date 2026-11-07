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
 * this module never sees the DOM and the root project's vitest can run it.
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
}

export interface StrokeInputOptions {
  /** Drop a sample closer than this to the one before it. `0` keeps every one. */
  minSpacing?: number;
  /** Stop growing a stroke once it has this many points. */
  maxPoints?: number;
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
  private readonly minSpacing: number;
  private readonly maxPoints: number;

  constructor(options: StrokeInputOptions = {}) {
    this.minSpacing = options.minSpacing ?? 0;
    this.maxPoints = options.maxPoints ?? Infinity;
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
    return this.take();
  }

  /** A pointer was cancelled. Returns whether a stroke was thrown away. */
  cancel(pointerId: number): boolean {
    if (!this.owns(pointerId)) return false;
    this.discard();
    return true;
  }

  /** A pointer lost its capture. Returns the stroke it was still drawing, if any. */
  lost(pointerId: number): StrokePoint[] | null {
    if (!this.owns(pointerId)) return null;
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
