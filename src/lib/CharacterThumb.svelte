<script lang="ts">
  /**
   * A small, non-interactive picture of one character's attempt.
   *
   * The boxes under the board use this to show the characters of a
   * multi-character entry that have already been written. It is deliberately
   * dumb: it paints whatever strokes it is handed, in the same display space and
   * at the same proportional pen width as the board, so a thumbnail is a
   * miniature of the real attempt rather than a redrawn sketch. Clicking is the
   * parent button's job, so nothing here takes the pointer.
   */
  import { drawThumb } from "./render";
  import type { GradeReport, Point } from "./types";

  interface Props {
    strokes: Point[][];
    report: GradeReport | null;
    /** Side of the square in CSS pixels. */
    size?: number;
  }

  let { strokes, report, size = 46 }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let dpr = $state(1);

  // A window can move between displays with different pixel ratios, the same as
  // the board.
  $effect(() => {
    const update = () => {
      dpr = window.devicePixelRatio || 1;
    };
    update();
    const query = window.matchMedia(`(resolution: ${dpr}dppx)`);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  });

  $effect(() => {
    // Read every input so the effect re-runs when any of them changes.
    void strokes;
    void report;
    void size;
    void dpr;

    const element = canvas;
    if (!element) return;
    const pixels = Math.round(size * dpr);
    if (element.width !== pixels || element.height !== pixels) {
      element.width = pixels;
      element.height = pixels;
    }
    const ctx = element.getContext("2d");
    if (!ctx) return;
    drawThumb(ctx, { size: pixels, strokes, report });
  });
</script>

<canvas bind:this={canvas} style="width: {size}px; height: {size}px;"></canvas>

<style>
  canvas {
    display: block;
    pointer-events: none;
  }
</style>
