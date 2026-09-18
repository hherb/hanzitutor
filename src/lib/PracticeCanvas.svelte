<script lang="ts">
  /**
   * The practice surface.
   *
   * Owns only the stroke currently under the pointer; completed strokes are
   * lifted to the parent so that undo, clearing and grading all have one source
   * of truth. Coordinates are converted to display space (0..=1024, y down)
   * before leaving this component, which is the space the Rust grader expects.
   */
  import { BOX, drawScene } from "./render";
  import type { Character, GradeReport, Point } from "./types";

  interface Props {
    character: Character | null;
    strokes: Point[][];
    report: GradeReport | null;
    ghostCount: number;
    ghostStyle: "faint" | "highlight";
    showCorrections: boolean;
    disabled?: boolean;
    onStroke: (stroke: Point[]) => void;
  }

  let {
    character,
    strokes,
    report,
    ghostCount,
    ghostStyle,
    showCorrections,
    disabled = false,
    onStroke,
  }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  /**
   * The canvas is sized in CSS from `side`, so it cannot measure itself: at
   * startup it is 0x0 and an observer on the canvas would never see a size to
   * grow into. The surrounding board has its size from the layout, so that is
   * what gets observed.
   */
  let frame = $state<HTMLDivElement | null>(null);
  /** Side of the square board in CSS pixels. */
  let side = $state(0);
  let dpr = $state(1);

  /**
   * The in-progress stroke is deliberately *not* `$state`: it changes on every
   * pointer move and only needs to schedule a repaint, so keeping it reactive
   * would mean paying for deep proxying at pointer frequency.
   */
  let current: Point[] | null = null;

  /** Ignore samples closer together than this (design units), to cut jitter. */
  const MIN_SPACING = 2;
  /** Bound the payload per stroke. */
  const MAX_POINTS = 1024;

  let scheduled = 0;
  function repaint() {
    if (scheduled) return;
    scheduled = requestAnimationFrame(() => {
      scheduled = 0;
      paint();
    });
  }

  function paint() {
    const element = canvas;
    if (!element || side === 0) return;
    const pixels = Math.round(side * dpr);
    if (element.width !== pixels || element.height !== pixels) {
      element.width = pixels;
      element.height = pixels;
    }
    const ctx = element.getContext("2d");
    if (!ctx) return;
    drawScene(ctx, {
      size: pixels,
      character,
      ghostCount,
      ghostStyle,
      strokes,
      current,
      report,
      showCorrections,
    });
  }

  // Repaint whenever anything visible changes.
  $effect(() => {
    void character;
    void strokes;
    void report;
    void ghostCount;
    void ghostStyle;
    void showCorrections;
    void side;
    void dpr;
    repaint();
  });

  // Fit the largest square that the board has room for.
  $effect(() => {
    const element = frame;
    if (!element) return;
    const observer = new ResizeObserver((entries) => {
      const box = entries[0]?.contentRect;
      if (!box) return;
      const next = Math.floor(Math.min(box.width, box.height));
      if (next > 0 && next !== side) side = next;
    });
    observer.observe(element);
    return () => observer.disconnect();
  });

  // A window can move between displays with different pixel ratios.
  $effect(() => {
    const update = () => {
      dpr = window.devicePixelRatio || 1;
    };
    update();
    const query = window.matchMedia(`(resolution: ${dpr}dppx)`);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  });

  /** Pointer position in display space. */
  function toDisplay(event: PointerEvent): Point {
    const rect = canvas!.getBoundingClientRect();
    return {
      x: ((event.clientX - rect.left) / rect.width) * BOX,
      y: ((event.clientY - rect.top) / rect.height) * BOX,
    };
  }

  function push(point: Point) {
    if (!current || current.length >= MAX_POINTS) return;
    const last = current[current.length - 1];
    if (last && Math.hypot(point.x - last.x, point.y - last.y) < MIN_SPACING) return;
    current.push(point);
  }

  function handleDown(event: PointerEvent) {
    if (disabled || event.button !== 0) return;
    event.preventDefault();
    canvas?.setPointerCapture(event.pointerId);
    current = [toDisplay(event)];
    repaint();
  }

  function handleMove(event: PointerEvent) {
    if (!current) return;
    event.preventDefault();
    // Coalesced events recover the full-rate samples that the browser batched
    // into a single pointermove, which matters for smooth handwriting.
    const samples = event.getCoalescedEvents?.() ?? [];
    for (const sample of samples.length > 0 ? samples : [event]) {
      push(toDisplay(sample));
    }
    repaint();
  }

  function handleUp(event: PointerEvent) {
    if (!current) return;
    event.preventDefault();
    if (canvas?.hasPointerCapture(event.pointerId)) {
      canvas.releasePointerCapture(event.pointerId);
    }
    // The release position is real input, and for a quick flick it can be the
    // only sample after the press: a stroke that is never extended collapses to
    // a single point and is then discarded as an accidental tap, which looks
    // exactly like the app throwing the stroke away. `push` drops it when it
    // merely repeats the last sample, so this costs nothing normally.
    push(toDisplay(event));
    const finished = current;
    current = null;
    if (finished.length > 0) onStroke(finished);
    repaint();
  }

  // A pointer that leaves the window mid-stroke should still commit the stroke.
  function handleCancel(event: PointerEvent) {
    handleUp(event);
  }
</script>

<div class="board" bind:this={frame}>
  <canvas
    bind:this={canvas}
    class:disabled
    style="width: {side}px; height: {side}px;"
    onpointerdown={handleDown}
    onpointermove={handleMove}
    onpointerup={handleUp}
    onpointercancel={handleCancel}
    oncontextmenu={(event) => event.preventDefault()}
  ></canvas>
</div>

<style>
  .board {
    /* Take the leftover height of the stage column and centre the square in
       it. This element has a size from the layout, independent of the canvas,
       which is what lets the observer above break the 0x0 deadlock. */
    flex: 1;
    width: 100%;
    min-height: 320px;
    display: grid;
    place-items: center;
  }
  canvas {
    display: block;
    border-radius: 10px;
    box-shadow: 0 1px 3px rgb(15 23 42 / 0.14), 0 8px 24px rgb(15 23 42 / 0.08);
    /* The pointer is the only scroller here: stop the webview panning or
       selecting text while a stroke is being drawn. */
    touch-action: none;
    user-select: none;
    cursor: crosshair;
  }
  canvas.disabled {
    cursor: default;
    opacity: 0.65;
  }
</style>
