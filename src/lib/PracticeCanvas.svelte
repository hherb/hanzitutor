<script lang="ts">
  /**
   * The practice surface.
   *
   * Owns only the stroke currently under the pointer; completed strokes are
   * lifted to the parent so that undo, clearing and grading all have one source
   * of truth. Coordinates are converted to display space (0..=1024, y down)
   * before leaving this component, which is the space the Rust grader expects.
   *
   * Two ways to draw share this one path. *Drag* is press-and-hold, which suits a
   * stylus. *Click to draw* starts a stroke with one click, extends it as the
   * pointer moves with no button held, and ends it with a second click — because
   * a long stroke on a trackpad means holding the button down for a long time.
   * Both produce their points through the same conversion and the same filtering
   * below, so a stroke's geometry does not depend on which one drew it.
   */
  import { BOX, drawScene } from "./render";
  import type { Sweep } from "./render";
  import type { Character, GradeReport, Point } from "./types";

  interface Props {
    character: Character | null;
    strokes: Point[][];
    report: GradeReport | null;
    ghostCount: number;
    ghostStyle: "faint" | "highlight";
    showCorrections: boolean;
    /** Where the stroke-order pen is, or null when nothing is animating. */
    sweep: Sweep | null;
    /** Click to start a stroke and click again to finish, rather than dragging. */
    clickToDraw: boolean;
    /**
     * The fraction of the room available the board should take, `0..=1`.
     *
     * The board is a square fitted to its container, and this is how the
     * settings screen's board size scales that fit — the container is what the
     * layout decides, so the preference is a share of it rather than a pixel
     * size that would only suit one window.
     */
    boardFraction?: number;
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
    sweep,
    clickToDraw,
    boardFraction = 1,
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
  /**
   * True while a click-to-draw stroke is open, between its two clicks. It is a
   * plain variable for the same reason as `current`.
   */
  let open = false;

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
      sweep,
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
    void sweep;
    void side;
    void dpr;
    repaint();
  });

  // Fit the largest square that the board has room for, at the share of it the
  // settings ask for. `boardFraction` is read inside the effect so that changing
  // it re-runs this and the board resizes at once, without waiting for the
  // window to be resized.
  $effect(() => {
    const element = frame;
    const fraction = boardFraction;
    if (!element) return;
    const fit = (width: number, height: number) =>
      Math.floor(Math.min(width, height) * fraction);
    const observer = new ResizeObserver((entries) => {
      const box = entries[0]?.contentRect;
      if (!box) return;
      const next = fit(box.width, box.height);
      if (next > 0 && next !== side) side = next;
    });
    observer.observe(element);
    // The observer only fires on the *container* changing, so the new fraction
    // has to be applied to what it already measured.
    const box = element.getBoundingClientRect();
    const immediate = fit(box.width, box.height);
    if (immediate > 0 && immediate !== side) side = immediate;
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

  /** Lift whatever has been drawn to the parent as a finished stroke. */
  function commit() {
    const finished = current;
    current = null;
    open = false;
    if (finished && finished.length > 0) onStroke(finished);
    repaint();
  }

  /** Throw away an open click-to-draw stroke that was never finished. */
  function cancelDraft() {
    if (!open) return;
    current = null;
    open = false;
    repaint();
  }

  function handleDown(event: PointerEvent) {
    if (disabled || event.button !== 0) return;
    event.preventDefault();

    if (clickToDraw) {
      // The second click ends the stroke where it lands. Nothing is captured:
      // the stroke is extended by hover moves, which need no button.
      if (open) {
        push(toDisplay(event));
        commit();
      } else {
        current = [toDisplay(event)];
        open = true;
        repaint();
      }
      return;
    }

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
    // In click-to-draw mode the release is not the end of the stroke — the
    // stroke stays open until the next press, so the pointer can be repositioned
    // as many times as it takes without the button held down.
    if (clickToDraw) {
      if (current) event.preventDefault();
      return;
    }
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
    commit();
  }

  // A pointer that leaves the window mid-stroke should still commit the stroke.
  function handleCancel(event: PointerEvent) {
    if (!current) return;
    if (clickToDraw) {
      // The pointer is gone, so there is no second click to wait for; keeping
      // the stroke open would leave ink on the board that nothing can finish.
      event.preventDefault();
      push(toDisplay(event));
      commit();
      return;
    }
    handleUp(event);
  }

  /**
   * Escape or Backspace abandons an open stroke.
   *
   * This listens in the *capture* phase so that Backspace can mean two things
   * without the two handlers fighting: with a stroke open it cancels that stroke,
   * and `stopPropagation` keeps the app's Backspace-to-undo from also removing
   * the last committed one.
   */
  $effect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!open) return;
      if (event.key !== "Escape" && event.key !== "Backspace") return;
      const target = event.target as HTMLElement | null;
      if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
      event.preventDefault();
      event.stopPropagation();
      cancelDraft();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  });

  /**
   * Switching how strokes are drawn keeps the learner's ink: a stroke that is
   * open when the mode changes is committed rather than discarded, since it is
   * already on the board and throwing it away would look like the app losing it.
   */
  $effect(() => {
    void clickToDraw;
    if (open) commit();
  });
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
