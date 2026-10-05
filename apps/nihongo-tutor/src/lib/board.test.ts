import { describe, expect, it } from "vitest";
import { MIN_POINTS, afterGrade, isStroke, pointerToDisplay, sweepAt } from "./board";
import { BOX } from "./render";

describe("pointerToDisplay", () => {
  const rect = { left: 100, top: 50, width: 512, height: 512 };

  it("maps the element's corners to the corners of the box", () => {
    expect(pointerToDisplay({ clientX: 100, clientY: 50 }, rect)).toEqual({ x: 0, y: 0 });
    expect(pointerToDisplay({ clientX: 612, clientY: 562 }, rect)).toEqual({
      x: BOX,
      y: BOX,
    });
  });

  it("maps the middle to the middle", () => {
    const centre = pointerToDisplay({ clientX: 356, clientY: 306 }, rect);
    expect(centre.x).toBeCloseTo(BOX / 2, 6);
    expect(centre.y).toBeCloseTo(BOX / 2, 6);
  });

  it("scales with the element, not with the backing store", () => {
    // The canvas is drawn at the device pixel ratio, but a pointer arrives in CSS
    // pixels, so a smaller element must still cover the whole box.
    const small = { left: 0, top: 0, width: 128, height: 128 };
    const at = pointerToDisplay({ clientX: 64, clientY: 64 }, small);
    expect(at.x).toBeCloseTo(BOX / 2, 6);
    expect(at.y).toBeCloseTo(BOX / 2, 6);
  });

  it("reports the centre rather than NaN when the element has no size", () => {
    // An unlaid-out or hidden canvas has a zero-width rect; dividing by it would
    // produce NaN points, which would poison every stroke in the attempt.
    for (const broken of [
      { left: 0, top: 0, width: 0, height: 0 },
      { left: 0, top: 0, width: 0, height: 512 },
      { left: 0, top: 0, width: 512, height: 0 },
    ]) {
      const at = pointerToDisplay({ clientX: 10, clientY: 10 }, broken);
      expect(Number.isFinite(at.x)).toBe(true);
      expect(Number.isFinite(at.y)).toBe(true);
      expect(at).toEqual({ x: BOX / 2, y: BOX / 2 });
    }
  });

  it("handles a pointer outside the element without clamping", () => {
    // Overshooting the board is a real thing a hand does, and the grader has a
    // placement measure for it; clamping here would hide it.
    const at = pointerToDisplay({ clientX: 0, clientY: 0 }, rect);
    expect(at.x).toBeLessThan(0);
    expect(at.y).toBeLessThan(0);
  });
});

describe("sweepAt", () => {
  it("starts at the first stroke and finishes on the last", () => {
    expect(sweepAt(0, 3, 900)).toEqual({ index: 0, progress: 0 });
    expect(sweepAt(900, 3, 900)).toEqual({ index: 2, progress: 1 });
  });

  it("walks one stroke at a time, evenly", () => {
    // Three strokes over 900 ms is 300 ms each.
    expect(sweepAt(150, 3, 900)).toEqual({ index: 0, progress: 0.5 });
    expect(sweepAt(300, 3, 900)).toEqual({ index: 1, progress: 0 });
    expect(sweepAt(450, 3, 900)).toEqual({ index: 1, progress: 0.5 });
    expect(sweepAt(600, 3, 900)).toEqual({ index: 2, progress: 0 });
  });

  it("never runs off the end, however late it is called", () => {
    // The frame loop can overshoot by a frame; indexing past the last stroke
    // would be a crash mid-animation.
    for (const elapsed of [900.1, 1200, 100000]) {
      const at = sweepAt(elapsed, 3, 900);
      expect(at.index).toBe(2);
      expect(at.progress).toBe(1);
    }
  });

  it("copes with a single-stroke kana", () => {
    // ん, る, し and ー are one stroke, so the whole animation is one segment.
    expect(sweepAt(0, 1, 500)).toEqual({ index: 0, progress: 0 });
    expect(sweepAt(250, 1, 500)).toEqual({ index: 0, progress: 0.5 });
    expect(sweepAt(500, 1, 500)).toEqual({ index: 0, progress: 1 });
  });

  it("treats nonsense input as finished rather than throwing", () => {
    expect(sweepAt(100, 0, 900)).toEqual({ index: 0, progress: 1 });
    expect(sweepAt(-5, 3, 900)).toEqual({ index: 0, progress: 0 });
  });
});

describe("isStroke", () => {
  it("keeps a drawn stroke and drops a tap", () => {
    expect(isStroke([{ x: 1, y: 1 }, { x: 2, y: 2 }])).toBe(true);
    expect(isStroke([{ x: 1, y: 1 }])).toBe(false);
    expect(isStroke([])).toBe(false);
    expect(MIN_POINTS).toBe(2);
  });
});

describe("afterGrade", () => {
  it("asks for a verdict while there is none", () => {
    // Skipping with the corner arrow is not grading, so the control stays Grade.
    expect(afterGrade(false, true, true)).toBe("grade");
    expect(afterGrade(false, true, false)).toBe("grade");
    // Nothing drawn and no verdict yet: the board's own answer is the same.
    expect(afterGrade(false, false, false)).toBe("grade");
  });

  it("offers the next character once the attempt has been judged", () => {
    expect(afterGrade(true, true, true)).toBe("next");
  });

  it("offers the way back when the lesson is finished", () => {
    // The last character of a lesson has no next one, and the way on is the
    // course — Hanzi Tutor's "Finish" at the end of an entry.
    expect(afterGrade(true, true, false)).toBe("finish");
  });

  it("keeps asking for a verdict on a radical, which is in no lesson", () => {
    // 92 of the 214 head forms are not jōyō characters and belong to no lesson:
    // there is nothing to advance to and nothing to finish.
    expect(afterGrade(true, false, false)).toBe("grade");
    expect(afterGrade(true, false, true)).toBe("grade");
  });
});
