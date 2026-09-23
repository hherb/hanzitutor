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

/**
 * Radius of the band a pen sweeps while revealing a stroke, in design units.
 *
 * The band has to uncover the whole *width* of the stroke, not only its middle:
 * too narrow and the outline's edges fill in late, in disconnected fragments,
 * which looks like a rendering fault rather than a stroke being written. So the
 * radius is not a guess — it is measured from the stroke's own outline, once per
 * character and only when the animation first runs (see [`strokeRadii`]).
 *
 * Measured on the shipped data (87,609 strokes), an outline point sits 34.8
 * units from its centre-line at the median, 55.0 at the 90th percentile, 85.7 at
 * the 99th, 125.4 at the 99.99th and 274 at the worst. [`MAX_HALF_WIDTH`] is
 * above all but a handful; [`DEFAULT_HALF_WIDTH`] is what a stroke falls back to
 * if its outline cannot be measured at all.
 */
const DEFAULT_HALF_WIDTH = 72;
const MAX_HALF_WIDTH = 160;
/** Clamp on the measured radius, so one outlier stroke cannot fill the box. */
const MIN_SWEEP_RADIUS = 18;
const MAX_SWEEP_RADIUS = 170;
/** Points sampled along a centre-line when measuring its stroke's width. */
const WIDTH_SAMPLES = 7;
/** Steps of the binary search for the outline's edge, each halving the range. */
const WIDTH_STEPS = 8;

/** Radius of the dot marking the pen's position, in design units. */
const PEN_RADIUS = 16;

const INK = "#1f2937";
const PEN = "#1d4ed8";
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
  /** Mid-animation pen position, or null when nothing is animating. */
  sweep: Sweep | null;
}

/**
 * A pen part-way along one reference stroke, during the stroke-order animation.
 *
 * `index` names the stroke and `progress` is how far along its centre-line the
 * pen has travelled, `0..=1` by length. Everything up to `ghostCount` is already
 * revealed, so the two together say exactly how much of the character is on the
 * board.
 */
export interface Sweep {
  index: number;
  progress: number;
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

/**
 * Measured half-width of each stroke, per character, filled in by
 * [`strokeRadii`] the first time that character is animated.
 */
const widthCache = new WeakMap<Character, number[]>();

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
  const sweep = scene.sweep;
  if (count === 0 && !sweep) return;

  const paths = pathsFor(character);
  applyFontSpace(ctx, scene.size);
  const colour = scene.ghostStyle === "highlight" ? HIGHLIGHT : GHOST;
  ctx.fillStyle = colour;
  for (let i = 0; i < count; i++) {
    const path = paths[i];
    if (path) ctx.fill(path);
  }

  // The stroke under the pen: revealed only as far as the pen has travelled, so
  // the character appears to be written rather than switched on a stroke at a
  // time. The transform is still font space here, which is the space the outline
  // is stored in; the centre-line is flipped into it below.
  if (!sweep) return;
  const index = sweep.index;
  if (index < 0 || index >= character.outlines.length || index < count) return;
  const outline = paths[index];
  const median = character.medians[index];
  if (outline && median && median.length > 0) {
    const radius = strokeRadii(ctx, character)[index] ?? DEFAULT_HALF_WIDTH;
    drawSweptStroke(ctx, outline, median, sweep.progress, radius, colour);
  }
}

/**
 * How far each of a character's strokes extends either side of its centre-line,
 * in design units, measured from the outline itself.
 *
 * The canvas cannot be asked for a path's geometry, but it can be asked whether
 * a point is inside one, so each stroke's width is found by walking outward from
 * points along its centre-line until the outline is left. That is exactly the
 * radius the sweeping band needs, and it is measured per stroke rather than
 * assumed: strokes in the same character differ by a factor of three, and a
 * single constant is either too narrow to uncover the wide ones — which is what
 * leaves the reveal in fragments — or wide enough to flash a short 点 in whole.
 *
 * The cost is a few hundred hit tests per character, paid once, on the first
 * frame of the first animation for that character and cached thereafter; nothing
 * here runs while the animation is idle.
 *
 * Exported for `render.test.ts`: the measurement is arithmetic over the outline
 * and the centre-line, so it is pinned there against a fake canvas rather than
 * only exercised by watching the animation.
 */
export function strokeRadii(ctx: CanvasRenderingContext2D, character: Character): number[] {
  const cached = widthCache.get(character);
  if (cached) return cached;

  const paths = pathsFor(character);
  // The hit test measures in the path's own coordinates, which are font space at
  // 1:1, so the transform is cleared for the duration.
  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  const radii = character.medians.map((median, index) => {
    const outline = paths[index];
    if (!outline || median.length === 0) return DEFAULT_HALF_WIDTH;
    return measureStrokeRadius(ctx, outline, median);
  });
  ctx.restore();

  widthCache.set(character, radii);
  return radii;
}

function measureStrokeRadius(
  ctx: CanvasRenderingContext2D,
  outline: Path2D,
  median: Point[],
): number {
  let widest = 0;
  for (const { point, tangent } of sampleAlong(median, WIDTH_SAMPLES)) {
    // Font space, which is where the outline path lives: the centre-lines are
    // stored in display space with y down.
    const x = point.x;
    const y = FONT_TOP_Y - point.y;
    if (!ctx.isPointInPath(outline, x, y)) continue;
    for (const side of [1, -1]) {
      const nx = -tangent.y * side;
      const ny = tangent.x * side;
      // The outward direction follows the same y flip, so the search walks the
      // normal in font space rather than across the stroke.
      let inside = 0;
      let outside = MAX_HALF_WIDTH;
      if (!ctx.isPointInPath(outline, x + nx * outside, y - ny * outside)) {
        for (let step = 0; step < WIDTH_STEPS; step++) {
          const middle = (inside + outside) / 2;
          if (ctx.isPointInPath(outline, x + nx * middle, y - ny * middle)) inside = middle;
          else outside = middle;
        }
      } else {
        inside = outside;
      }
      widest = Math.max(widest, inside);
    }
  }
  if (widest <= 0) return DEFAULT_HALF_WIDTH;
  // A little margin, so the whole cross-section under the pen is uncovered
  // rather than its edge being left for the end of the stroke.
  return Math.min(MAX_SWEEP_RADIUS, Math.max(MIN_SWEEP_RADIUS, widest * 1.15 + 6));
}

/**
 * Points spread evenly by length along a polyline, with their local direction.
 *
 * Exported for `render.test.ts`, which pins the sample positions and tangents;
 * the animation itself only ever reaches it through [`measureStrokeRadius`].
 */
export function sampleAlong(
  points: Point[],
  count: number,
): { point: Point; tangent: Point }[] {
  const out: { point: Point; tangent: Point }[] = [];
  if (points.length === 0) return out;
  const total = polylineLength(points);
  if (total <= 0) return [{ point: points[0], tangent: { x: 1, y: 0 } }];
  for (let k = 0; k < count; k++) {
    const target = ((k + 0.5) / count) * total;
    let walked = 0;
    for (let i = 1; i < points.length; i++) {
      const a = points[i - 1];
      const b = points[i];
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const segment = Math.hypot(dx, dy);
      if (walked + segment >= target && segment > 0) {
        const t = (target - walked) / segment;
        out.push({
          point: { x: a.x + dx * t, y: a.y + dy * t },
          tangent: { x: dx / segment, y: dy / segment },
        });
        break;
      }
      walked += segment;
    }
  }
  return out;
}

/**
 * Paint one stroke of the animation: the part of its outline the pen has
 * already swept, and the pen itself.
 *
 * The outline is *clipped* to the swept band rather than drawn at partial
 * opacity, because a half-drawn stroke should look half-written — the ink is
 * either there or it is not. `ctx.clip()` fills the current path, so the band is
 * built as a union of subpaths (a disc at every centre-line point the pen has
 * passed, and a quad joining consecutive ones); overlapping subpaths union
 * under the non-zero winding rule, which is what makes one path out of it.
 */
function drawSweptStroke(
  ctx: CanvasRenderingContext2D,
  outline: Path2D,
  median: Point[],
  progress: number,
  radius: number,
  colour: string,
) {
  const fontMedian = median.map((p) => ({ x: p.x, y: FONT_TOP_Y - p.y }));
  const reached = Math.max(0, Math.min(1, progress));
  if (reached >= 1) {
    // Done: the band never quite covers a very thick short stroke, so the
    // finished stroke is the whole outline rather than the sweep's leftovers.
    ctx.fillStyle = colour;
    ctx.fill(outline);
    return;
  }

  const prefix = prefixAt(fontMedian, reached);
  if (prefix.length === 0) return;
  ctx.save();
  ctx.beginPath();
  sweptBand(ctx, prefix, radius);
  ctx.clip();
  ctx.fillStyle = colour;
  ctx.fill(outline);
  ctx.restore();

  const head = prefix[prefix.length - 1];
  ctx.fillStyle = PEN;
  ctx.beginPath();
  ctx.arc(head.x, head.y, PEN_RADIUS, 0, Math.PI * 2);
  ctx.fill();
}

/** Total length of a polyline, in design units. */
export function polylineLength(points: Point[]): number {
  let total = 0;
  for (let i = 1; i < points.length; i++) {
    total += Math.hypot(points[i].x - points[i - 1].x, points[i].y - points[i - 1].y);
  }
  return total;
}

/**
 * The start of `points`, cut off `progress` of the way along it by length —
 * never by index, because the samples are not evenly spaced.
 *
 * Exported for `render.test.ts`. The caller clamps `progress` to `0..=1`
 * ([`drawSweptStroke`]); this function trusts the value it is given.
 */
export function prefixAt(points: Point[], progress: number): Point[] {
  if (points.length === 0) return [];
  const total = polylineLength(points);
  if (total <= 0) return [points[0]];
  const target = progress * total;
  const out: Point[] = [points[0]];
  let walked = 0;
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1];
    const b = points[i];
    const segment = Math.hypot(b.x - a.x, b.y - a.y);
    if (walked + segment >= target) {
      const t = segment > 0 ? (target - walked) / segment : 0;
      out.push({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t });
      return out;
    }
    walked += segment;
    out.push(b);
  }
  return out;
}

/**
 * A thick band following `points`, as one path: a disc at each point and a
 * filled quad between each pair. Round joins come free from the discs, which is
 * what keeps the band's inner edge smooth where it turns a corner.
 */
function sweptBand(ctx: CanvasRenderingContext2D, points: Point[], radius: number) {
  for (let i = 0; i < points.length; i++) {
    const p = points[i];
    ctx.moveTo(p.x + radius, p.y);
    ctx.arc(p.x, p.y, radius, 0, Math.PI * 2);
    const next = points[i + 1];
    if (!next) break;
    const dx = next.x - p.x;
    const dy = next.y - p.y;
    const length = Math.hypot(dx, dy);
    if (length === 0) continue;
    const nx = (-dy / length) * radius;
    const ny = (dx / length) * radius;
    ctx.moveTo(p.x + nx, p.y + ny);
    ctx.lineTo(next.x + nx, next.y + ny);
    ctx.lineTo(next.x - nx, next.y - ny);
    ctx.lineTo(p.x - nx, p.y - ny);
    ctx.closePath();
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
