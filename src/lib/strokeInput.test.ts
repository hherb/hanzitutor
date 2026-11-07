/**
 * The board's pointer rules, pinned (issue #30).
 *
 * `strokeInput.ts` is the one place both drawing boards decide which pointer is
 * drawing and what a stroke is made of, so each rule the issue named is a case
 * here: a palm or a second finger cannot hijack the stroke in progress, a pen
 * takes over from a touch, a cancelled stroke is thrown away rather than graded,
 * and a stroke always ends where the pointer was lifted — so a quick dot is two
 * points and survives the trip to the grader.
 *
 * Pointer events are plain objects, already in display space: the module never
 * sees the DOM, which is what lets the root project's vitest run it in Node.
 */
import { describe, expect, it } from "vitest";
import * as here from "./strokeInput";
import * as nihongo from "../../apps/nihongo-tutor/src/lib/strokeInput";
import type { PointerSample } from "./strokeInput";

const PEN = 2;
const FINGER = 10;
const PALM = 11;
const MOUSE = 1;

function pen(x: number, y: number, pointerId = PEN): PointerSample {
  return { pointerId, pointerType: "pen", isPrimary: true, x, y };
}
function touch(x: number, y: number, pointerId = FINGER, isPrimary = true): PointerSample {
  return { pointerId, pointerType: "touch", isPrimary, x, y };
}
function mouse(x: number, y: number): PointerSample {
  return { pointerId: MOUSE, pointerType: "mouse", isPrimary: true, x, y };
}

const xy = (x: number, y: number) => ({ x, y });

// Both copies, the same cases: Nihongo Tutor's is a copy on purpose, and this is
// what keeps it one.
describe.each([
  ["Hanzi Tutor", here.StrokeInput],
  ["Nihongo Tutor", nihongo.StrokeInput],
] as const)("stroke input in %s", (_, StrokeInput) => {
  describe("which pointer draws", () => {
    it("ignores a second finger that lands mid-stroke", () => {
      const input = new StrokeInput();
      expect(input.down(touch(100, 100))).toEqual({ accepted: true, release: null });
      input.move(touch(110, 100));

      // A second finger is never the primary touch.
      const second = touch(500, 500, PALM, false);
      expect(input.down(second)).toEqual({ accepted: false });
      expect(input.move(touch(510, 510, PALM, false))).toBe(false);
      // Its release neither ends nor commits anything.
      expect(input.up(touch(520, 520, PALM, false))).toBeNull();

      input.move(touch(120, 100));
      expect(input.up(touch(130, 100))).toStrictEqual([
        xy(100, 100),
        xy(110, 100),
        xy(120, 100),
        xy(130, 100),
      ]);
    });

    it("ignores a resting palm while the pen is drawing", () => {
      const input = new StrokeInput();
      input.down(pen(100, 100));
      input.move(pen(150, 100));

      // The palm is the first touch, so it *is* primary — the pen still wins.
      expect(input.down(touch(800, 800, PALM))).toEqual({ accepted: false });
      expect(input.move(touch(805, 805, PALM))).toBe(false);
      expect(input.up(touch(810, 810, PALM))).toBeNull();
      expect(input.pointerId).toBe(PEN);

      expect(input.up(pen(200, 100))).toStrictEqual([xy(100, 100), xy(150, 100), xy(200, 100)]);
    });

    it("hands the stroke to a pen that lands while a touch is drawing", () => {
      // The palm got there first; the pencil is what the learner is writing with.
      const input = new StrokeInput();
      input.down(touch(800, 800, PALM));
      input.move(touch(805, 805, PALM));

      expect(input.down(pen(100, 100))).toEqual({ accepted: true, release: PALM });
      expect(input.pointerId).toBe(PEN);
      expect(input.points).toStrictEqual([xy(100, 100)]);

      // The palm's later moves and its lift are nobody's business now.
      expect(input.move(touch(810, 810, PALM))).toBe(false);
      expect(input.up(touch(815, 815, PALM))).toBeNull();

      input.move(pen(150, 100));
      expect(input.up(pen(200, 100))).toStrictEqual([xy(100, 100), xy(150, 100), xy(200, 100)]);
    });

    it("ignores a non-primary pointer even when nothing is drawing", () => {
      const input = new StrokeInput();
      expect(input.down(touch(10, 10, PALM, false))).toEqual({ accepted: false });
      expect(input.points).toBeNull();
      expect(input.pointerId).toBeNull();
    });

    it("ignores a touch or a mouse while another kind of pointer is drawing", () => {
      const input = new StrokeInput();
      input.down(mouse(100, 100));
      expect(input.down(touch(500, 500))).toEqual({ accepted: false });
      expect(input.down(pen(500, 500))).toEqual({ accepted: false });
      expect(input.pointerId).toBe(MOUSE);

      const other = new StrokeInput();
      other.down(pen(100, 100));
      expect(other.down(mouse(500, 500))).toEqual({ accepted: false });
      expect(other.pointerId).toBe(PEN);
    });

    it("starts again when the same kind of primary pointer goes down twice", () => {
      // There is only ever one primary pointer of a kind, so a second press means
      // the first one's release never arrived. Waiting for it would lock the board.
      const input = new StrokeInput();
      input.down(touch(100, 100, 20));
      input.move(touch(150, 100, 20));
      expect(input.down(touch(300, 300, 21))).toEqual({ accepted: true, release: 20 });
      expect(input.points).toStrictEqual([xy(300, 300)]);

      const clicks = new StrokeInput();
      clicks.down(mouse(100, 100));
      expect(clicks.down(mouse(300, 300))).toEqual({ accepted: true, release: null });
      expect(clicks.points).toStrictEqual([xy(300, 300)]);
    });

    it("accepts a move only from the pointer that is drawing", () => {
      const input = new StrokeInput();
      expect(input.move(pen(1, 1))).toBe(false);
      input.down(pen(100, 100));
      expect(input.move(pen(150, 100, 99))).toBe(false);
      expect(input.points).toStrictEqual([xy(100, 100)]);
    });
  });

  describe("how a stroke ends", () => {
    it("adds the position the pointer was lifted at", () => {
      // A quick flick can deliver no move at all between the press and the lift.
      const input = new StrokeInput();
      input.down(touch(100, 100));
      expect(input.up(touch(140, 130))).toStrictEqual([xy(100, 100), xy(140, 130)]);
      expect(input.points).toBeNull();
      expect(input.pointerId).toBeNull();
    });

    it("keeps a tap as a two-point dot rather than dropping it", () => {
      const input = new StrokeInput();
      input.down(pen(300, 400));
      expect(input.up(pen(300, 400))).toStrictEqual([xy(300, 400), xy(300, 400)]);
    });

    it("keeps a tap as a dot when the lift is too close to count as a sample", () => {
      const input = new StrokeInput({ minSpacing: 2 });
      input.down(pen(300, 400));
      expect(input.up(pen(301, 400))).toStrictEqual([xy(300, 400), xy(300, 400)]);
    });

    it("does not repeat the last sample when the lift lands on it", () => {
      const input = new StrokeInput({ minSpacing: 2 });
      input.down(pen(0, 0));
      input.move(pen(50, 0));
      expect(input.up(pen(50.5, 0))).toStrictEqual([xy(0, 0), xy(50, 0)]);
    });

    it("throws a cancelled stroke away", () => {
      const input = new StrokeInput();
      input.down(touch(100, 100));
      input.move(touch(200, 100));
      expect(input.cancel(FINGER)).toBe(true);
      expect(input.points).toBeNull();
      expect(input.pointerId).toBeNull();
      // Whatever arrives after the cancel for that pointer is not a stroke.
      expect(input.move(touch(250, 100))).toBe(false);
      expect(input.up(touch(300, 100))).toBeNull();
    });

    it("leaves the stroke alone when some other pointer is cancelled", () => {
      const input = new StrokeInput();
      input.down(pen(100, 100));
      expect(input.cancel(PALM)).toBe(false);
      expect(input.up(pen(200, 100))).toStrictEqual([xy(100, 100), xy(200, 100)]);
    });

    it("finishes the stroke with what it has when the capture is lost first", () => {
      const input = new StrokeInput();
      input.down(pen(100, 100));
      input.move(pen(150, 100));
      expect(input.lost(PALM)).toBeNull();
      expect(input.lost(PEN)).toStrictEqual([xy(100, 100), xy(150, 100)]);
      expect(input.points).toBeNull();
      // The ordinary order — the lift, then the lost capture — is one stroke.
      input.down(pen(10, 10));
      expect(input.up(pen(20, 10))).not.toBeNull();
      expect(input.lost(PEN)).toBeNull();
    });

    it("returns a lone point lost with the capture as a dot", () => {
      const input = new StrokeInput();
      input.down(pen(5, 6));
      expect(input.lost(PEN)).toStrictEqual([xy(5, 6), xy(5, 6)]);
    });
  });

  describe("the samples a stroke keeps", () => {
    it("keeps every sample by default", () => {
      const input = new StrokeInput();
      input.down(pen(0, 0));
      input.move(pen(0.5, 0));
      input.move(pen(0.5, 0));
      expect(input.up(pen(1, 0))).toStrictEqual([xy(0, 0), xy(0.5, 0), xy(0.5, 0), xy(1, 0)]);
    });

    it("drops samples closer than the spacing it was given", () => {
      const input = new StrokeInput({ minSpacing: 2 });
      input.down(pen(0, 0));
      input.move(pen(1, 0));
      input.move(pen(3, 0));
      expect(input.points).toStrictEqual([xy(0, 0), xy(3, 0)]);
    });

    it("takes the coalesced samples of a move in place of the move itself", () => {
      const input = new StrokeInput();
      input.down(pen(0, 0));
      expect(input.move(pen(30, 0), [xy(10, 0), xy(20, 0), xy(30, 0)])).toBe(true);
      expect(input.points).toStrictEqual([xy(0, 0), xy(10, 0), xy(20, 0), xy(30, 0)]);
      // An empty list is a browser with nothing batched: the move is the sample.
      input.move(pen(40, 0), []);
      expect(input.points).toStrictEqual([xy(0, 0), xy(10, 0), xy(20, 0), xy(30, 0), xy(40, 0)]);
    });

    it("stops growing at the point budget", () => {
      const input = new StrokeInput({ maxPoints: 3 });
      input.down(pen(0, 0));
      for (let x = 10; x <= 100; x += 10) input.move(pen(x, 0));
      expect(input.up(pen(200, 0))).toStrictEqual([xy(0, 0), xy(10, 0), xy(20, 0)]);
    });

    it("keeps only the coordinates, not the pointer's identity", () => {
      // The stroke is what goes to the grader; it must be plain points.
      const input = new StrokeInput();
      input.down(pen(1, 2));
      const stroke = input.up(pen(3, 4))!;
      expect(stroke.every((p) => Object.keys(p).sort().join() === "x,y")).toBe(true);
    });

    it("hands back its own copy, which later input does not touch", () => {
      const input = new StrokeInput();
      input.down(pen(1, 2));
      const stroke = input.up(pen(1, 2))!;
      expect(stroke[0]).not.toBe(stroke[1]);
      input.down(pen(9, 9));
      input.move(pen(10, 9));
      expect(stroke).toStrictEqual([xy(1, 2), xy(1, 2)]);
    });
  });

  describe("a stroke that no pointer holds", () => {
    // Hanzi Tutor's click-to-draw: one click opens the stroke, hover moves extend
    // it and a second click — from whatever pointer id — closes it.
    it("opens, extends and finishes with the closing position", () => {
      const input = new StrokeInput({ minSpacing: 2 });
      input.begin(xy(0, 0));
      expect(input.pointerId).toBeNull();
      input.extend(xy(50, 0));
      expect(input.points).toStrictEqual([xy(0, 0), xy(50, 0)]);
      expect(input.finish(xy(100, 0))).toStrictEqual([xy(0, 0), xy(50, 0), xy(100, 0)]);
      expect(input.points).toBeNull();
    });

    it("finishes a single click as a dot", () => {
      const input = new StrokeInput();
      input.begin(xy(7, 8));
      expect(input.finish()).toStrictEqual([xy(7, 8), xy(7, 8)]);
    });

    it("can be thrown away, and finishing nothing is nothing", () => {
      const input = new StrokeInput();
      input.begin(xy(7, 8));
      input.discard();
      expect(input.points).toBeNull();
      expect(input.finish(xy(1, 1))).toBeNull();
      input.extend(xy(2, 2));
      expect(input.points).toBeNull();
    });

    it("is replaced by a pointer that goes down to drag", () => {
      const input = new StrokeInput();
      input.begin(xy(7, 8));
      expect(input.down(mouse(100, 100))).toEqual({ accepted: true, release: null });
      expect(input.points).toStrictEqual([xy(100, 100)]);
    });
  });
});

/** A sample stamped with `PointerEvent.timeStamp`, in milliseconds. */
const at = <T extends PointerSample>(sample: T, t: number): T => ({ ...sample, t });

describe.each([
  ["Hanzi Tutor", here],
  ["Nihongo Tutor", nihongo],
] as const)("the pen's grace period in %s", (_, module) => {
  const { PEN_GRACE_MS, StrokeInput } = module;

  it("ignores a palm that lands just after the pen lifts", () => {
    // Between two pencil strokes the hand stays on the glass. Without a pen down
    // at the same moment there is nothing for the pen to win against, so the
    // palm would draw — and lift — a stroke of its own.
    const input = new StrokeInput();
    input.down(at(pen(100, 100), 1000));
    expect(input.up(at(pen(200, 100), 1100))).not.toBeNull();

    expect(input.down(at(touch(800, 800, PALM), 1100 + PEN_GRACE_MS - 1))).toEqual({
      accepted: false,
    });
    expect(input.points).toBeNull();
    expect(input.up(at(touch(810, 810, PALM), 1100 + PEN_GRACE_MS))).toBeNull();

    // The pencil coming back is unaffected.
    expect(input.down(at(pen(300, 300), 1100 + PEN_GRACE_MS))).toEqual({
      accepted: true,
      release: null,
    });
  });

  it("lets a finger draw once the grace period is over", () => {
    const input = new StrokeInput();
    input.down(at(pen(100, 100), 0));
    input.up(at(pen(200, 100), 50));
    expect(input.down(at(touch(400, 400), 50 + PEN_GRACE_MS))).toEqual({
      accepted: true,
      release: null,
    });
    expect(input.up(at(touch(450, 400), 60 + PEN_GRACE_MS))).toStrictEqual([
      xy(400, 400),
      xy(450, 400),
    ]);
  });

  it("starts the grace period however the pen's stroke ended", () => {
    const cancelled = new StrokeInput();
    cancelled.down(at(pen(100, 100), 0));
    cancelled.cancel(PEN, 10);
    expect(cancelled.down(at(touch(1, 1), 20))).toEqual({ accepted: false });

    const lost = new StrokeInput();
    lost.down(at(pen(100, 100), 0));
    lost.lost(PEN, 10);
    expect(lost.down(at(touch(1, 1), 20))).toEqual({ accepted: false });
  });

  it("leaves a finger user alone, and a mouse after a pen", () => {
    // No pen, no grace period: a finger stroke straight after a finger stroke.
    const fingers = new StrokeInput();
    fingers.down(at(touch(100, 100, 30), 0));
    fingers.up(at(touch(200, 100, 30), 10));
    expect(fingers.down(at(touch(300, 100, 31), 20)).accepted).toBe(true);

    const mixed = new StrokeInput();
    mixed.down(at(pen(100, 100), 0));
    mixed.up(at(pen(200, 100), 10));
    expect(mixed.down(at(mouse(5, 5), 20)).accepted).toBe(true);
  });

  it("does not apply without a timestamp to measure it by", () => {
    const input = new StrokeInput();
    input.down(pen(100, 100));
    input.up(pen(200, 100));
    expect(input.down(touch(300, 300)).accepted).toBe(true);
  });

  it("can be configured, or turned off", () => {
    const short = new StrokeInput({ penGraceMs: 100 });
    short.down(at(pen(0, 0), 0));
    short.up(at(pen(10, 0), 0));
    expect(short.down(at(touch(1, 1), 99)).accepted).toBe(false);
    expect(short.down(at(touch(1, 1), 100)).accepted).toBe(true);

    const off = new StrokeInput({ penGraceMs: 0 });
    off.down(at(pen(0, 0), 0));
    off.up(at(pen(10, 0), 0));
    expect(off.down(at(touch(1, 1), 0)).accepted).toBe(true);
  });
});

describe.each([
  ["Hanzi Tutor", here],
  ["Nihongo Tutor", nihongo],
] as const)("what counts as something drawn in %s", (_, module) => {
  const { DRAWN_MIN_LENGTH, hasDrawnStroke, isDrawnStroke } = module;

  it("refuses a tap: one point, or the two coincident points a board hands over", () => {
    expect(isDrawnStroke([])).toBe(false);
    expect(isDrawnStroke([xy(5, 5)])).toBe(false);
    expect(isDrawnStroke([xy(5, 5), xy(5, 5)])).toBe(false);
    expect(isDrawnStroke([xy(5, 5), xy(5.3, 5.3), xy(5, 5)])).toBe(false);
  });

  it("accepts anything that travelled, measured along the path", () => {
    expect(isDrawnStroke([xy(5, 5), xy(5 + DRAWN_MIN_LENGTH, 5)])).toBe(true);
    // Back and forth over the same spot still travelled.
    expect(isDrawnStroke([xy(0, 0), xy(0.6, 0), xy(0, 0)])).toBe(true);
    expect(isDrawnStroke([xy(0, 0), xy(300, 0)])).toBe(true);
  });

  it("stays far below the grader's stray threshold, so it never refuses a stroke the grader keeps", () => {
    // The grader drops marks under min(12, half the shortest reference stroke);
    // the shortest stroke in the Chinese data is 4 units, which puts its
    // threshold at 2. Anything between this and that is the grader's to call.
    expect(DRAWN_MIN_LENGTH).toBeLessThan(2);
  });

  it("asks whether any stroke of an attempt was drawn", () => {
    expect(hasDrawnStroke([])).toBe(false);
    expect(hasDrawnStroke([[xy(1, 1), xy(1, 1)]])).toBe(false);
    expect(hasDrawnStroke([[xy(1, 1), xy(1, 1)], [xy(0, 0), xy(50, 0)]])).toBe(true);
  });
});

describe("the two copies", () => {
  it("are byte-identical", async () => {
    // Node's own module, typed by hand: the root project has no `@types/node`.
    const fs = (await import("node:fs" as string)) as {
      readFileSync(path: URL, encoding: "utf8"): string;
    };
    const read = (relative: string) => fs.readFileSync(new URL(relative, import.meta.url), "utf8");
    expect(read("../../apps/nihongo-tutor/src/lib/strokeInput.ts")).toBe(read("./strokeInput.ts"));
  });
});
