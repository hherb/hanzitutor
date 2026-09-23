/**
 * The geometry behind the practice board, pinned.
 *
 * This is the interface's first test run, and it exists because the stroke
 * animation had none: `prefixAt`, `sampleAlong` and `strokeRadii` are
 * arithmetic over arrays, and until now the only thing checking them was the
 * compiler. A wrong prefix or a wrong sample is invisible in a screenshot and
 * obvious to a learner, so each is asserted against a hand-computed value here
 * rather than by watching the canvas.
 *
 * Two fakes make that possible without a browser:
 *
 * - a `Path2D` stub, because `pathsFor` builds one per outline and Node has no
 *   such constructor;
 * - a recording canvas context, whose `isPointInPath` is a predicate the test
 *   supplies. The width measurement is entirely hit tests, so a synthetic
 *   outline as simple as a vertical strip pins the arithmetic exactly — including
 *   which direction the search walks and where it clamps.
 */

import { beforeEach, describe, expect, it } from "vitest";
import {
  INK_WIDTH,
  drawScene,
  drawThumb,
  polylineLength,
  prefixAt,
  sampleAlong,
  strokeRadii,
} from "./render";
import type { Character, Point } from "./types";

/** A character whose only interesting parts are its outlines and centre-lines. */
function character(overrides: Partial<Character> = {}): Character {
  return {
    ch: "一",
    rank: 1,
    hsk: 1,
    strokeCount: 1,
    radical: "一",
    pinyin: ["yī"],
    definition: "one",
    etymology: "",
    decomposition: "",
    outlines: ["M 0 0"],
    medians: [[{ x: 0, y: 0 }, { x: 100, y: 0 }]],
    ...overrides,
  };
}

const p = (x: number, y: number): Point => ({ x, y });

/** The global `Path2D`, which `render.ts` and this test both build. */
class FakePath2D {
  constructor(public readonly d: string) {}
}

beforeEach(() => {
  (globalThis as unknown as { Path2D: unknown }).Path2D = FakePath2D;
});

type Op =
  | { op: "setTransform"; a: number; b: number; c: number; d: number; e: number; f: number }
  | { op: "moveTo"; x: number; y: number }
  | { op: "lineTo"; x: number; y: number }
  | { op: "arc"; x: number; y: number; r: number }
  | { op: "clip" }
  | { op: "fill" }
  | { op: "stroke" }
  | { op: "beginPath" }
  | { op: "closePath" }
  | { op: "clearRect" }
  | { op: "fillRect" }
  | { op: "strokeRect" }
  | { op: "save" }
  | { op: "restore" };

/**
 * A canvas context that records what it was asked to draw and answers hit tests
 * from `inside`. Everything `render.ts` touches is here; nothing else is.
 */
function recorder(inside: (x: number, y: number) => boolean = () => false) {
  const ops: Op[] = [];
  let hitTests = 0;
  const ctx = {
    fillStyle: "",
    strokeStyle: "",
    lineWidth: 0,
    lineCap: "butt" as CanvasLineCap,
    lineJoin: "miter" as CanvasLineJoin,
    setTransform: (a: number, b: number, c: number, d: number, e: number, f: number) =>
      ops.push({ op: "setTransform", a, b, c, d, e, f }),
    clearRect: () => ops.push({ op: "clearRect" }),
    fillRect: () => ops.push({ op: "fillRect" }),
    strokeRect: () => ops.push({ op: "strokeRect" }),
    beginPath: () => ops.push({ op: "beginPath" }),
    closePath: () => ops.push({ op: "closePath" }),
    moveTo: (x: number, y: number) => ops.push({ op: "moveTo", x, y }),
    lineTo: (x: number, y: number) => ops.push({ op: "lineTo", x, y }),
    arc: (x: number, y: number, r: number) => ops.push({ op: "arc", x, y, r }),
    clip: () => ops.push({ op: "clip" }),
    fill: () => ops.push({ op: "fill" }),
    stroke: () => ops.push({ op: "stroke" }),
    save: () => ops.push({ op: "save" }),
    restore: () => ops.push({ op: "restore" }),
    isPointInPath: (_path: unknown, x: number, y: number) => {
      hitTests += 1;
      return inside(x, y);
    },
  };
  return {
    ctx: ctx as unknown as CanvasRenderingContext2D,
    ops,
    hitTests: () => hitTests,
  };
}

const arcs = (ops: Op[]) =>
  ops.filter((o): o is Extract<Op, { op: "arc" }> => o.op === "arc");
const transforms = (ops: Op[]) =>
  ops.filter((o): o is Extract<Op, { op: "setTransform" }> => o.op === "setTransform");
const count = (ops: Op[], op: Op["op"]) => ops.filter((o) => o.op === op).length;

describe("polylineLength", () => {
  it("is zero for nothing and for a single point", () => {
    expect(polylineLength([])).toBe(0);
    expect(polylineLength([p(7, 7)])).toBe(0);
  });

  it("sums the segments, and a repeated point adds nothing", () => {
    // Two 3-4-5 triangles, with a duplicate in the middle that must not count.
    expect(polylineLength([p(0, 0), p(3, 4), p(3, 4), p(6, 8)])).toBeCloseTo(10, 12);
  });
});

describe("prefixAt", () => {
  it("returns nothing for no points, and the point itself for a degenerate run", () => {
    expect(prefixAt([], 0.5)).toEqual([]);
    // One point, or several at the same place: there is no length to cut.
    expect(prefixAt([p(5, 5)], 0.5)).toEqual([p(5, 5)]);
    expect(prefixAt([p(1, 1), p(1, 1)], 0.5)).toEqual([p(1, 1)]);
  });

  it("cuts by length, not by index — the samples are not evenly spaced", () => {
    // A 1-unit first segment then a 10-unit one. Half way is x = 5.5; cutting
    // by index would stop at the vertex at x = 1 and be visibly wrong. The
    // vertex is kept because the pen really passes through it.
    const prefix = prefixAt([p(0, 0), p(1, 0), p(11, 0)], 0.5);
    expect(prefix).toHaveLength(3);
    expect(prefix[2].x).toBeCloseTo(5.5, 12);
    expect(prefix[2].y).toBeCloseTo(0, 12);
  });

  it("interpolates inside the segment that contains the target", () => {
    const prefix = prefixAt([p(0, 0), p(100, 0)], 0.25);
    expect(prefix).toEqual([p(0, 0), p(25, 0)]);
  });

  it("includes a vertex exactly when the target reaches it", () => {
    const corner = [p(0, 0), p(10, 0), p(10, 10)];
    // Half of the 20-unit polyline is the corner itself.
    expect(prefixAt(corner, 0.5)).toEqual([p(0, 0), p(10, 0)]);
    // Three quarters is half way up the second segment.
    const later = prefixAt(corner, 0.75);
    expect(later).toHaveLength(3);
    expect(later[2].x).toBeCloseTo(10, 12);
    expect(later[2].y).toBeCloseTo(5, 12);
  });

  it("returns the whole run at or past the end", () => {
    const line = [p(0, 0), p(10, 0), p(11, 0)];
    expect(prefixAt(line, 1)).toEqual(line);
    // Out of range is the caller's to clamp; past the end is still the whole run.
    expect(prefixAt(line, 2)).toEqual(line);
  });
});

describe("sampleAlong", () => {
  it("returns nothing for no points", () => {
    expect(sampleAlong([], 7)).toEqual([]);
  });

  it("returns the single point, with a fallback tangent, when there is no length", () => {
    expect(sampleAlong([p(5, 5)], 7)).toEqual([{ point: p(5, 5), tangent: p(1, 0) }]);
    expect(sampleAlong([p(5, 5), p(5, 5)], 3)).toEqual([
      { point: p(5, 5), tangent: p(1, 0) },
    ]);
  });

  it("spreads samples at the middle of equal-length bins", () => {
    const samples = sampleAlong([p(0, 0), p(100, 0)], 4);
    expect(samples).toHaveLength(4);
    expect(samples.map((s) => s.point.x)).toEqual([12.5, 37.5, 62.5, 87.5]);
    expect(samples.every((s) => s.tangent.x === 1 && s.tangent.y === 0)).toBe(true);
  });

  it("takes each sample's tangent from the segment it lands on", () => {
    // A 20-unit polyline: the first sample is on the horizontal segment, the
    // second on the vertical one, so the tangents must differ.
    const samples = sampleAlong([p(0, 0), p(10, 0), p(10, 10)], 2);
    expect(samples).toHaveLength(2);
    expect(samples[0].point).toEqual(p(5, 0));
    expect(samples[0].tangent).toEqual(p(1, 0));
    expect(samples[1].point).toEqual(p(10, 5));
    expect(samples[1].tangent).toEqual(p(0, 1));
  });

  it("normalises the tangent to a unit vector on a diagonal", () => {
    // 3-4-5 again: the direction is (0.6, 0.8), not the raw (3, 4).
    const [sample] = sampleAlong([p(0, 0), p(3, 4)], 1);
    expect(sample.point.x).toBeCloseTo(1.5, 12);
    expect(sample.point.y).toBeCloseTo(2, 12);
    expect(sample.tangent.x).toBeCloseTo(0.6, 12);
    expect(sample.tangent.y).toBeCloseTo(0.8, 12);
  });
});

describe("strokeRadii", () => {
  /** A vertical centre-line, so the outline's half-width is measured sideways. */
  const vertical = () => character({ medians: [[p(100, 0), p(100, 100)]] });

  /** An outline that is the vertical strip |x - 100| <= halfWidth. */
  const strip = (halfWidth: number) => (x: number) => Math.abs(x - 100) <= halfWidth;

  it("measures the outline's half-width and adds the margin", () => {
    const { ctx } = recorder(strip(50));
    // 50 * 1.15 + 6, which is the constant in measureStrokeRadius.
    expect(strokeRadii(ctx, vertical())[0]).toBeCloseTo(63.5, 6);
  });

  it("clamps a wide stroke so one of them cannot fill the box", () => {
    const { ctx } = recorder(strip(200));
    expect(strokeRadii(ctx, vertical())[0]).toBe(170);
  });

  it("clamps a thin stroke so it still uncovers its own width", () => {
    const { ctx } = recorder(strip(5));
    expect(strokeRadii(ctx, vertical())[0]).toBe(18);
  });

  it("falls back to the default when the centre-line is not inside the outline", () => {
    // A strip of zero width contains x = 100 and nothing either side of it, so
    // the search measures nothing and the default is the only honest answer.
    const { ctx } = recorder(strip(0));
    expect(strokeRadii(ctx, vertical())[0]).toBe(72);
  });

  it("falls back per stroke for an empty centre-line and a missing outline", () => {
    const { ctx } = recorder(() => false);
    const two = character({
      outlines: ["M 0 0"],
      medians: [[], [p(0, 0), p(100, 0)]],
    });
    // The first has no centre-line to measure; the second has no outline.
    expect(strokeRadii(ctx, two)).toEqual([72, 72]);
  });

  it("measures each character once and caches the result", () => {
    const { ctx, hitTests } = recorder(strip(50));
    const ch = vertical();
    const first = strokeRadii(ctx, ch);
    const afterFirst = hitTests();
    expect(afterFirst).toBeGreaterThan(0);

    // The same character object, so the per-character cache is what answers.
    const second = strokeRadii(ctx, ch);
    expect(hitTests()).toBe(afterFirst);
    expect(second).toBe(first);
  });
});

describe("the stroke-order animation", () => {
  const scene = (sweep: { index: number; progress: number } | null, size = 1024) => ({
    size,
    character: character({ medians: [[p(0, 100), p(100, 100)]] }),
    ghostCount: 0,
    ghostStyle: "faint" as const,
    strokes: [],
    current: null,
    report: null,
    showCorrections: false,
    sweep,
  });

  it("fills a finished stroke whole, with no band and no pen", () => {
    const { ctx, ops } = recorder();
    drawScene(ctx, scene({ index: 0, progress: 1 }));
    // Appending the whole outline is what avoids a thick short stroke keeping
    // the sweep's leftovers at the end.
    expect(count(ops, "clip")).toBe(0);
    expect(arcs(ops)).toHaveLength(0);
    expect(count(ops, "fill")).toBeGreaterThan(0);
  });

  it("reveals part of a stroke and puts the pen at the head of the sweep", () => {
    const { ctx, ops } = recorder();
    drawScene(ctx, scene({ index: 0, progress: 0.5 }));

    // The centre-line is display space (y down) and is flipped into font space,
    // so a stroke at y = 100 half way along has its head at (50, 800).
    expect(transforms(ops)).toContainEqual({
      op: "setTransform",
      a: 1,
      b: 0,
      c: 0,
      d: -1,
      e: 0,
      f: 900,
    });
    expect(count(ops, "clip")).toBe(1);
    const pen = arcs(ops).filter((a) => a.r === 16);
    expect(pen).toEqual([{ op: "arc", x: 50, y: 800, r: 16 }]);
  });

  it("clamps progress, so the first frame is a dot at the stroke's start", () => {
    const { ctx, ops } = recorder();
    drawScene(ctx, scene({ index: 0, progress: 0 }));
    const pen = arcs(ops).filter((a) => a.r === 16);
    expect(pen).toEqual([{ op: "arc", x: 0, y: 800, r: 16 }]);
  });

  it("scales the font-space transform with the canvas", () => {
    const { ctx, ops } = recorder();
    drawScene(ctx, scene({ index: 0, progress: 1 }, 512));
    // 512 / 1024 = 0.5, and the y flip scales with it: 900 * 0.5 = 450.
    expect(transforms(ops)).toContainEqual({
      op: "setTransform",
      a: 0.5,
      b: 0,
      c: 0,
      d: -0.5,
      e: 0,
      f: 450,
    });
  });

  it("leaves a scene with no sweep alone", () => {
    const { ctx, ops } = recorder();
    drawScene(ctx, scene(null));
    expect(count(ops, "clip")).toBe(0);
    expect(arcs(ops)).toHaveLength(0);
  });
});

describe("drawThumb", () => {
  it("draws a dot for a single tap, at the display-space pen width", () => {
    const { ctx, ops } = recorder();
    drawThumb(ctx, { size: 512, strokes: [[p(100, 200)]], report: null });
    // Display space is not flipped: (x, y) maps straight through at size / BOX.
    expect(transforms(ops)).toContainEqual({
      op: "setTransform",
      a: 0.5,
      b: 0,
      c: 0,
      d: 0.5,
      e: 0,
      f: 0,
    });
    expect(arcs(ops)).toEqual([{ op: "arc", x: 100, y: 200, r: INK_WIDTH / 2 }]);
    expect(count(ops, "stroke")).toBe(0);
  });

  it("strokes a line for a real stroke", () => {
    const { ctx, ops } = recorder();
    drawThumb(ctx, { size: 512, strokes: [[p(0, 0), p(50, 0)]], report: null });
    expect(ops).toContainEqual({ op: "moveTo", x: 0, y: 0 });
    expect(ops).toContainEqual({ op: "lineTo", x: 50, y: 0 });
    expect(count(ops, "stroke")).toBe(1);
    expect(arcs(ops)).toHaveLength(0);
  });

  it("draws nothing for an empty attempt", () => {
    const { ctx, ops } = recorder();
    drawThumb(ctx, { size: 256, strokes: [], report: null });
    expect(ops).toEqual([
      { op: "setTransform", a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 },
      { op: "clearRect" },
    ]);
  });
});
