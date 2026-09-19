/**
 * Canvas rendering for the practice surface.
 *
 * Two coordinate systems meet here, and keeping them straight is most of the
 * work:
 *
 * - **font space** — the Make Me a Hanzi stroke outlines. The character box's
 *   upper-left is `(0, 900)` and its lower-right is `(1024, -124)`, so y
 *   increases *upwards*. Outlines are drawn with a flip transform.
 * - **display space** — a 1024x1024 box with y increasing *downwards*, which is
 *   what the user's strokes are recorded in and what the Rust grader expects.
 */

import type { Character, GradeReport, Point, Verdict } from "./types";

/** Side of the character box, in design units. */
export const BOX = 1024;
/** Font-space y of the top edge of the character box. */
const FONT_TOP_Y = 900;
/**
 * Stroke width of the user's ink, in design units.
 *
 * This is also the width the grader is told to rasterise the attempt at, so the
 * ink measure compares what was drawn with what a correct trace would put down.
 * `INK_WIDTH` in `crates/hanzi-core/src/raster.rs` is the same number; the
 * attempt carries it explicitly so the two cannot drift apart silently.
 */
export const INK_WIDTH = 36;

const INK = "#1f2937";
const GHOST = "#e6ebf2";
const GUIDE = "#d7e0ea";
const GUIDE_DIAGONAL = "#eaeff5";
const HIGHLIGHT = "rgba(37, 99, 235, 0.32)";
const CORRECTION = "rgba(220, 38, 38, 0.13)";
const CORRECTION_MISSING = "rgba(220, 38, 38, 0.26)";
const EXTRA_STROKE = "#94a3b8";

/** Colour used for a user stroke, by the verdict on the stroke it matched. */
export const VERDICT_COLOUR: Record<Verdict, string> = {
  correct: "#15803d",
  out_of_order: "#b45309",
  wrong_direction: "#7c3aed",
  position_off: "#c2410c",
  shape_off: "#be123c",
  faint: "#0e7490",
  missing: "#64748b",
};

export const VERDICT_LABEL: Record<Verdict, string> = {
  correct: "Correct",
  shape_off: "Wrong shape",
  position_off: "Misplaced",
  wrong_direction: "Drawn backwards",
  out_of_order: "Out of order",
  faint: "Too little ink",
  missing: "Not written",
};

/** Everything needed to paint one frame. */
export interface Scene {
  /** Side of the square canvas in device pixels. */
  size: number;
  character: Character | null;
  /** How many reference strokes to reveal, in stroke order. */
  ghostCount: number;
  ghostStyle: "faint" | "highlight";
  strokes: Point[][];
  /** The stroke currently under the pointer, if any. */
  current: Point[] | null;
  report: GradeReport | null;
  /** Show where strokes that are wrong or missing should have gone. */
  showCorrections: boolean;
}

/**
 * Parsed outlines are expensive to build and reused every frame, so cache them
 * per character. `Character` objects are replaced rather than mutated when the
 * user moves on, so the weak keys collect naturally.
 */
const pathCache = new WeakMap<Character, Path2D[]>();

function pathsFor(character: Character): Path2D[] {
  let paths = pathCache.get(character);
  if (!paths) {
    paths = character.outlines.map((d) => new Path2D(d));
    pathCache.set(character, paths);
  }
  return paths;
}

/** Transform so that font-space coordinates land correctly on the canvas. */
function applyFontSpace(ctx: CanvasRenderingContext2D, size: number) {
  const k = size / BOX;
  // (x, y) -> (k*x, k*(900 - y))
  ctx.setTransform(k, 0, 0, -k, 0, FONT_TOP_Y * k);
}

/** Transform so that display-space coordinates map directly to the canvas. */
function applyDisplaySpace(ctx: CanvasRenderingContext2D, size: number) {
  const k = size / BOX;
  ctx.setTransform(k, 0, 0, k, 0, 0);
}

export function drawScene(ctx: CanvasRenderingContext2D, scene: Scene) {
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, scene.size, scene.size);

  drawPaper(ctx, scene.size);
  if (scene.character) {
    drawGhost(ctx, scene);
    if (scene.report && scene.showCorrections) drawCorrections(ctx, scene);
  }
  drawInk(ctx, scene);
}

/** The practice square: a 米字格 guide, as used on Chinese practice paper. */
function drawPaper(ctx: CanvasRenderingContext2D, size: number) {
  const k = size / BOX;
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, size, size);

  ctx.lineWidth = Math.max(1, k * 2);
  ctx.strokeStyle = GUIDE_DIAGONAL;
  ctx.beginPath();
  ctx.moveTo(0, 0);
  ctx.lineTo(size, size);
  ctx.moveTo(size, 0);
  ctx.lineTo(0, size);
  ctx.stroke();

  ctx.strokeStyle = GUIDE;
  ctx.beginPath();
  ctx.moveTo(size / 2, 0);
  ctx.lineTo(size / 2, size);
  ctx.moveTo(0, size / 2);
  ctx.lineTo(size, size / 2);
  ctx.stroke();

  ctx.strokeStyle = "#c8d4e2";
  ctx.lineWidth = Math.max(1, k * 3);
  ctx.strokeRect(ctx.lineWidth / 2, ctx.lineWidth / 2, size - ctx.lineWidth, size - ctx.lineWidth);
}

/** The reference character, either as a faint trace guide or mid-animation. */
function drawGhost(ctx: CanvasRenderingContext2D, scene: Scene) {
  const character = scene.character;
  if (!character) return;
  const count = Math.max(0, Math.min(scene.ghostCount, character.outlines.length));
  if (count === 0) return;

  const paths = pathsFor(character);
  applyFontSpace(ctx, scene.size);
  ctx.fillStyle = scene.ghostStyle === "highlight" ? HIGHLIGHT : GHOST;
  for (let i = 0; i < count; i++) {
    const path = paths[i];
    if (path) ctx.fill(path);
  }
}

/** Highlight where a stroke should have gone, for anything not written right. */
function drawCorrections(ctx: CanvasRenderingContext2D, scene: Scene) {
  const character = scene.character;
  const report = scene.report;
  if (!character || !report) return;

  const paths = pathsFor(character);
  applyFontSpace(ctx, scene.size);
  for (const stroke of report.strokes) {
    if (stroke.verdict === "correct") continue;
    const path = paths[stroke.refIndex];
    if (!path) continue;
    ctx.fillStyle = stroke.verdict === "missing" ? CORRECTION_MISSING : CORRECTION;
    ctx.fill(path);
  }
}

/** The user's own strokes, coloured by how they were graded. */
function drawInk(ctx: CanvasRenderingContext2D, scene: Scene) {
  applyDisplaySpace(ctx, scene.size);
  // lineWidth is in the current transform's units, which are design units here.
  ctx.lineWidth = INK_WIDTH;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";

  scene.strokes.forEach((stroke, index) => {
    ctx.strokeStyle = colourForStroke(scene.report, index);
    strokePath(ctx, stroke);
  });

  if (scene.current && scene.current.length > 0) {
    ctx.strokeStyle = INK;
    strokePath(ctx, scene.current);
  }
}

/** Colour for one of the user's strokes, given what it was matched with. */
function colourForStroke(report: GradeReport | null, userIndex: number): string {
  if (!report) return INK;
  const refIndex = report.assignment[userIndex];
  if (refIndex === null || refIndex === undefined) return EXTRA_STROKE;
  const verdict = report.strokes[refIndex]?.verdict;
  return verdict ? VERDICT_COLOUR[verdict] : INK;
}

/** Everything needed to paint one small copy of an attempt. */
export interface ThumbScene {
  /** Side of the square canvas in device pixels. */
  size: number;
  strokes: Point[][];
  report: GradeReport | null;
}

/**
 * Paint a small copy of one attempt: the ink only, on a clean square.
 *
 * This is what the boxes under the board show for the characters of a
 * multi-character entry that have already been written. The strokes go through
 * the same display-space transform and the same proportional pen width as the
 * board, so a thumbnail is a true miniature of the attempt — and it is coloured
 * by the same verdicts, so a stroke that was marked wrong is still visible as
 * wrong at thumbnail size.
 */
export function drawThumb(ctx: CanvasRenderingContext2D, scene: ThumbScene) {
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, scene.size, scene.size);
  if (scene.strokes.length === 0) return;

  applyDisplaySpace(ctx, scene.size);
  ctx.lineWidth = INK_WIDTH;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  scene.strokes.forEach((stroke, index) => {
    ctx.strokeStyle = colourForStroke(scene.report, index);
    strokePath(ctx, stroke);
  });
}

function strokePath(ctx: CanvasRenderingContext2D, points: Point[]) {
  if (points.length === 0) return;

  if (points.length === 1) {
    // A single tap produces no line; draw a dot so the mark is visible.
    ctx.fillStyle = ctx.strokeStyle;
    ctx.beginPath();
    ctx.arc(points[0].x, points[0].y, INK_WIDTH / 2, 0, Math.PI * 2);
    ctx.fill();
    return;
  }

  ctx.beginPath();
  ctx.moveTo(points[0].x, points[0].y);
  for (let i = 1; i < points.length; i++) {
    ctx.lineTo(points[i].x, points[i].y);
  }
  ctx.stroke();
}
