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
  import { isStroke, pointerToDisplay, sweepAt } from "./board";
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
    current = null;
    drawing = false;
    ghostCount = character && guide ? character.strokeCount : 0;
    sweep = null;
    cancelAnimationFrame(raf);
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

  function down(event: PointerEvent) {
    if (!character) return;
    cancelAnimationFrame(raf);
    sweep = null;
    drawing = true;
    (event.target as HTMLElement).setPointerCapture(event.pointerId);
    current = [toDisplay(event)];
    paint();
  }

  function move(event: PointerEvent) {
    if (!drawing || !current) return;
    current = [...current, toDisplay(event)];
    paint();
  }

  function up(event: PointerEvent) {
    if (!drawing) return;
    drawing = false;
    (event.target as HTMLElement).releasePointerCapture?.(event.pointerId);
    // A tap is not a stroke; submitting one would have the grader count a
    // stroke the learner never meant to draw.
    if (current && isStroke(current)) {
      strokes = [...strokes, current];
      onchange?.(strokes);
    }
    current = null;
    paint();
  }

  /** Undo the last stroke. */
  export function undo() {
    strokes = strokes.slice(0, -1);
    onchange?.(strokes);
    paint();
  }

  /** Clear the board. */
  export function clear() {
    strokes = [];
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
        ghostCount = total;
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
    onpointercancel={up}
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
