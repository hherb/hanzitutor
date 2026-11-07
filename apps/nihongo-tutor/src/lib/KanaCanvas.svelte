<script lang="ts">
  /**
   * The practice board: pointer capture, the stroke-order animation, and the
   * painting of the verdict.
   *
   * All the drawing itself is `render.ts`, lifted from Hanzi Tutor, so this file
   * is only the interaction: turning pointer events into strokes in display
   * space, and driving the animation frame that reveals the guide stroke by
   * stroke.
   *
   * It draws **any** `Drawable`, not only a kana: the kana course was first, the
   * kanji course and the radicals panel came later, and all three hand it the
   * same four fields. The file keeps its name because that is what the handover,
   * the traps and the article about the blank window call it.
   */
  import { untrack } from "svelte";
  import { drawScene, type Scene } from "./render";
  import { ghostAfterAnimation, isStroke, pointerToDisplay, sweepAt } from "./board";
  import { StrokeInput, type PointerSample } from "./strokeInput";
  import type { Drawable, GradeReport, Point } from "./types";

  interface Props {
    character: Drawable | null;
    report: GradeReport | null;
    /** How long the whole stroke-order animation should take, in ms. */
    sweepMs?: number;
    /**
     * Whether the faint reference copy is on the board while the learner draws.
     *
     * `true` is tracing and `false` is recall, which is Hanzi Tutor's Hint
     * toggle: with the guide off, the character is on the board only in the
     * learner's memory until the animation is asked for. **Changing it clears the
     * attempt**, which is deliberate and is what Hanzi Tutor does — half an
     * attempt traced and half remembered is not an answer to either question.
     */
    guide?: boolean;
    /** Called with the finished attempt when the learner asks for a grade. */
    onchange?: (strokes: Point[][]) => void;
    /** Called when the learner asks for the guide animation. */
    onanimate?: () => void;
  }

  let {
    character,
    report,
    sweepMs = 1400,
    guide = true,
    onchange,
    onanimate,
  }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);

  /** Finished strokes, in display space. */
  let strokes = $state<Point[][]>([]);
  /**
   * Which pointer is drawing, and what it has drawn so far (`strokeInput.ts`,
   * shared with Hanzi Tutor's board). `current` is a copy of its stroke for the
   * repaint to read.
   */
  const input = new StrokeInput();
  /** The stroke under the pointer, if any. */
  let current = $state<Point[] | null>(null);
  let drawing = $state(false);
  /** How many reference strokes the animation has fully revealed. */
  let ghostCount = $state(0);
  /** Mid-animation pen position, or null when nothing is animating. */
  let sweep = $state<{ index: number; progress: number } | null>(null);

  let raf = 0;

  /** When the learner starts fresh on a new character, so does the board. */
  $effect(() => {
    // Reading these is what makes the effect re-run on a change. `guide` is one
    // of them: toggling the hint is switching between tracing and recall, and the
    // attempt belongs to whichever of the two it was drawn under.
    void character?.ch;
    void guide;
    strokes = [];
    input.discard();
    current = null;
    drawing = false;
    ghostCount = character && guide ? character.strokeCount : 0;
    sweep = null;
    cancelAnimationFrame(raf);
    // The screen above is told, because the attempt it is holding **is this one**:
    // without this, toggling the guide cleared the board while the verdict above it
    // stayed up and `Grade` still held the strokes it had just thrown away. Every
    // other path that empties the board tells the parent — `undo` and `clear` —
    // and this is the one that did not.
    onchange?.([]);
    // Deliberately no `paint()` here. `paint` reads every one of those pieces of
    // state, so calling it from this effect made the effect depend on what it
    // writes — and `strokes = []` is a fresh array, never `===` the last, so the
    // write always notified and the effect re-ran without end. Svelte threw
    // `effect_update_depth_exceeded` on mount and the window came up blank. The
    // repaint effect below covers this: `strokes = []` notifies it every time.
  });

  /**
   * Match the canvas's backing store to the size the stylesheet gave it.
   *
   * The **stylesheet** owns the geometry — the board is square by construction
   * (`.board { width: min(100%, 44vh); aspect-ratio: 1 }`) and the canvas fills
   * it — and this only sets the resolution, so the two cannot disagree. An
   * earlier version measured the board and wrote both the width and the height
   * from that measurement, which is how the board ended up a wide rectangle with
   * the kana stretched across it: any clamp on the box's height (a `max-height`,
   * a flex parent) made the measurement disagree with the layout.
   *
   * The first measurement is `untrack`ed because `paint` reads the whole picture;
   * left tracked, this effect would depend on `strokes`, `ghostCount` and `sweep`
   * and would rebuild its `ResizeObserver` on every animation frame.
   */
  $effect(() => {
    const cv = canvas;
    if (!cv) return;
    const measure = () => {
      const rect = cv.getBoundingClientRect();
      const px = Math.max(1, Math.round(rect.width * (window.devicePixelRatio || 1)));
      if (cv.width === px && cv.height === px) return;
      cv.width = px;
      cv.height = px;
      paint();
    };
    untrack(measure);
    const observer = new ResizeObserver(measure);
    observer.observe(cv);
    return () => observer.disconnect();
  });

  /** Repaint on anything that changes the picture. */
  $effect(() => {
    void strokes;
    void current;
    void report;
    void ghostCount;
    void sweep;
    void character?.ch;
    paint();
  });

  function paint() {
    const cv = canvas;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    const scene: Scene = {
      size: cv.width,
      character,
      ghostCount,
      ghostStyle: "faint",
      strokes,
      current,
      report,
      showCorrections: report !== null,
      sweep,
    };
    drawScene(ctx, scene);
  }

  /** Where a pointer landed, in the display space the grader works in. */
  function toDisplay(event: PointerEvent): Point {
    return pointerToDisplay(event, canvas!.getBoundingClientRect());
  }

  /**
   * Stop an animation that is still running, and put the guide back to the
   * board's resting state rather than leaving the strokes it had reached: with
   * the guide off, a half-drawn copy left on the board would be traced over.
   */
  function stopSweep() {
    cancelAnimationFrame(raf);
    if (sweep && character) {
      ghostCount = ghostAfterAnimation(guide, character.medians.length);
    }
    sweep = null;
  }

  /** The event as `strokeInput.ts` reads it: who it is, and where. */
  function sample(event: PointerEvent): PointerSample {
    return {
      pointerId: event.pointerId,
      pointerType: event.pointerType,
      isPrimary: event.isPrimary,
      t: event.timeStamp,
      ...toDisplay(event),
    };
  }

  /** Copy the stroke in progress into the state the board paints from. */
  function show() {
    const points = input.points;
    current = points ? [...points] : null;
    drawing = input.pointerId !== null;
    paint();
  }

  /**
   * Capture the pointer that is now drawing. `strokeInput` has already given it
   * the stroke, so a capture that throws — a pointer id the platform no longer
   * knows, which is what a synthetic event carries — must take the stroke back
   * rather than leave one open that no release will reach.
   */
  function capture(pointerId: number): boolean {
    try {
      canvas?.setPointerCapture(pointerId);
      return true;
    } catch {
      return false;
    }
  }

  function release(pointerId: number) {
    if (canvas?.hasPointerCapture(pointerId)) canvas.releasePointerCapture(pointerId);
  }

  /** A finished stroke joins the attempt. */
  function keep(finished: Point[] | null) {
    // `strokeInput` never hands back fewer than two points — a tap comes back as
    // a two-point dot — so this is a guard, not the tap filter it once was: what
    // is too small to be a stroke is the grader's stray-tap filter's call.
    if (finished && isStroke(finished)) {
      strokes = [...strokes, finished];
      onchange?.(strokes);
    }
    show();
  }

  function down(event: PointerEvent) {
    if (!character) return;
    // Only the pointer `strokeInput` lets draw: not a second finger, and not a
    // palm while the pencil writes (issue #30).
    const result = input.down(sample(event));
    if (!result.accepted) return;
    if (result.release !== null) release(result.release);
    if (!capture(event.pointerId)) {
      input.discard();
      show();
      return;
    }
    stopSweep();
    show();
  }

  function move(event: PointerEvent) {
    if (event.pointerId !== input.pointerId) return;
    if (input.move(sample(event))) show();
  }

  function up(event: PointerEvent) {
    // The lift's own position is part of the stroke: a quick dot can have no move
    // between the press and the lift, and used to be a single point that the
    // board dropped.
    const finished = input.up(sample(event));
    if (!finished) return;
    release(event.pointerId);
    keep(finished);
  }

  /**
   * The platform took the pointer back mid-stroke — an edge swipe, palm
   * rejection, the notification shade. The stroke it interrupted is thrown away
   * rather than joining the attempt half-drawn (issue #30).
   */
  function cancel(event: PointerEvent) {
    if (!input.cancel(event.pointerId, event.timeStamp)) return;
    release(event.pointerId);
    show();
  }

  /**
   * The capture went before the release did. In the ordinary order this arrives
   * after `pointerup` and finds no stroke; arriving first, the release will never
   * come, so the stroke ends with what it has.
   */
  function lost(event: PointerEvent) {
    const finished = input.lost(event.pointerId, event.timeStamp);
    if (finished) keep(finished);
  }

  /** Undo the last stroke. */
  export function undo() {
    stopSweep();
    strokes = strokes.slice(0, -1);
    onchange?.(strokes);
    paint();
  }

  /** Clear the board. */
  export function clear() {
    stopSweep();
    strokes = [];
    input.discard();
    current = null;
    onchange?.(strokes);
    paint();
  }

  /**
   * Animate the stroke order: each stroke is drawn along its own centre-line,
   * one after another, over `sweepMs`.
   *
   * The pen position is worked out from elapsed time rather than step counting,
   * so a dropped frame shortens the animation instead of slowing it down, and the
   * whole thing takes the same time on a slow machine.
   */
  export function animate() {
    if (!character || character.medians.length === 0) return;
    onanimate?.();
    const total = character.medians.length;
    const started = performance.now();

    const step = (now: number) => {
      const elapsed = now - started;
      const at = sweepAt(elapsed, total, sweepMs);
      ghostCount = at.index;
      sweep = at;
      paint();
      if (elapsed < sweepMs) {
        raf = requestAnimationFrame(step);
      } else {
        // Back to the board's resting state: the whole guide when tracing, and a
        // blank board in recall, where the animation was a demonstration and not
        // a copy to write over.
        ghostCount = ghostAfterAnimation(guide, total);
        sweep = null;
        paint();
      }
    };
    raf = requestAnimationFrame(step);
  }

  $effect(() => () => cancelAnimationFrame(raf));
</script>

<div class="board">
  <canvas
    bind:this={canvas}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={cancel}
    onlostpointercapture={lost}
    class:drawing
  ></canvas>
</div>

<style>
  .board {
    /* `min(100%, 44vh)` is what keeps it square: an `aspect-ratio` with a
       `max-height` clamps the height without narrowing the box, which is exactly
       the rectangle that shipped. Sizing by width alone, against the shorter of
       the column and the viewport, has no such failure mode. 44vh rather than a
       larger share so that the buttons and the typing box stay above the fold
       with the board at full size. */
    width: min(100%, 44vh);
    aspect-ratio: 1 / 1;
    margin-inline: auto;
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
    touch-action: none;
    border-radius: 14px;
    background: var(--board);
    box-shadow: inset 0 0 0 1px var(--line);
    cursor: crosshair;
  }
  canvas.drawing {
    cursor: default;
  }
</style>
