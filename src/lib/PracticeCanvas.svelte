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
   * (`strokeInput.ts`), so a stroke's geometry does not depend on which one drew
   * it. Only a drag belongs to one pointer: a palm or a second finger cannot take
   * it over, a pen can take it from a touch, and a cancelled drag is thrown away
   * (issue #30).
   */
  import { BOX, drawScene } from "./render";
  import type { Sweep } from "./render";
  import { StrokeInput } from "./strokeInput";
  import type { PointerSample } from "./strokeInput";
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
   * Which pointer is drawing, and the stroke it is drawing (`strokeInput.ts`,
   * shared with Nihongo Tutor's board).
   *
   * The in-progress stroke is deliberately *not* `$state`: it changes on every
   * pointer move and only needs to schedule a repaint, so keeping it reactive
   * would mean paying for deep proxying at pointer frequency. `paint` reads it
   * from here.
   *
   * Samples closer together than `minSpacing` (design units) are dropped, to cut
   * jitter, and `maxPoints` bounds the payload per stroke.
   */
  const input = new StrokeInput({ minSpacing: 2, maxPoints: 1024 });
  /**
   * True while a click-to-draw stroke is open, between its two clicks. It is a
   * plain variable for the same reason as the stroke itself.
   */
  let open = false;

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
      current: input.points as Point[] | null,
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

  /** The event as `strokeInput.ts` reads it: who it is, and where. */
  function sample(event: PointerEvent): PointerSample {
    return {
      pointerId: event.pointerId,
      pointerType: event.pointerType,
      isPrimary: event.isPrimary,
      ...toDisplay(event),
    };
  }

  function release(pointerId: number) {
    if (canvas?.hasPointerCapture(pointerId)) canvas.releasePointerCapture(pointerId);
  }

  /** Lift a finished stroke to the parent. */
  function lift(finished: Point[] | null) {
    open = false;
    if (finished) onStroke(finished);
    repaint();
  }

  /** Lift whatever click-to-draw stroke is open to the parent, ending at `point`. */
  function commit(point?: Point) {
    lift(input.finish(point));
  }

  /** Throw away an open click-to-draw stroke that was never finished. */
  function cancelDraft() {
    if (!open) return;
    input.discard();
    open = false;
    repaint();
  }

  function handleDown(event: PointerEvent) {
    if (disabled || event.button !== 0) return;
    event.preventDefault();

    if (clickToDraw) {
      // A second finger or a palm is not a click.
      if (!event.isPrimary) return;
      // The second click ends the stroke where it lands. Nothing is captured:
      // the stroke is extended by hover moves, which need no button — and the
      // two clicks need not share a pointer id, since a touch gets a new one per
      // contact.
      if (open) {
        commit(toDisplay(event));
      } else {
        input.begin(toDisplay(event));
        open = true;
        repaint();
      }
      return;
    }

    const down = input.down(sample(event));
    if (!down.accepted) return;
    if (down.release !== null) release(down.release);
    canvas?.setPointerCapture(event.pointerId);
    repaint();
  }

  function handleMove(event: PointerEvent) {
    if (!input.points) return;
    // Coalesced events recover the full-rate samples that the browser batched
    // into a single pointermove, which matters for smooth handwriting.
    const samples = event.getCoalescedEvents?.() ?? [];
    if (clickToDraw) {
      if (!open || !event.isPrimary) return;
      event.preventDefault();
      for (const one of samples.length > 0 ? samples : [event]) input.extend(toDisplay(one));
      repaint();
      return;
    }
    // Another pointer's moves — a resting palm's — cost nothing to ignore.
    if (event.pointerId !== input.pointerId) return;
    if (!input.move(sample(event), samples.map((one) => toDisplay(one)))) return;
    event.preventDefault();
    repaint();
  }

  function handleUp(event: PointerEvent) {
    // In click-to-draw mode the release is not the end of the stroke — the
    // stroke stays open until the next press, so the pointer can be repositioned
    // as many times as it takes without the button held down.
    if (clickToDraw) {
      if (open) event.preventDefault();
      return;
    }
    // The release position is real input, and for a quick flick it can be the
    // only sample after the press: `up` adds it (and drops it when it merely
    // repeats the last sample), and a stroke that is still a single point comes
    // back as a two-point dot rather than vanishing.
    const finished = input.up(sample(event));
    if (!finished) return;
    event.preventDefault();
    release(event.pointerId);
    lift(finished);
  }

  /**
   * The platform took the pointer back mid-stroke — an edge swipe, palm
   * rejection, the notification shade. A dragged stroke it interrupted is thrown
   * away rather than graded half-drawn (issue #30).
   */
  function handleCancel(event: PointerEvent) {
    if (clickToDraw) {
      if (!open || !event.isPrimary) return;
      // The pointer is gone, so there is no second click to wait for; keeping
      // the stroke open would leave ink on the board that nothing can finish.
      // A click-to-draw stroke was built deliberately, click by click, so it is
      // kept rather than thrown away.
      event.preventDefault();
      commit(toDisplay(event));
      return;
    }
    if (!input.cancel(event.pointerId)) return;
    event.preventDefault();
    release(event.pointerId);
    repaint();
  }

  /**
   * The capture went before the release did. In the ordinary order this arrives
   * after `pointerup` and finds no stroke; arriving first, the release will never
   * come, so the stroke ends with what it has.
   */
  function handleLostCapture(event: PointerEvent) {
    if (clickToDraw) return;
    const finished = input.lost(event.pointerId);
    if (finished) lift(finished);
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
    onlostpointercapture={handleLostCapture}
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
