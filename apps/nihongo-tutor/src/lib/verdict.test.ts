import { describe, expect, it } from "vitest";

import { everyStrokeFine, strokeCountNote } from "./verdict";
import type { StrokeVerdict } from "./types";

/**
 * The verdict card's sentences about the stroke count.
 *
 * The case that made them necessary is a perfect kana with one extra mark: every
 * per-stroke row says "correct", the score is held at 74, and the card must say
 * why rather than claim every stroke was right.
 */

const stroke = (refIndex: number, verdict: StrokeVerdict["verdict"]): StrokeVerdict => ({
  refIndex,
  userIndex: verdict === "missing" ? null : refIndex,
  verdict,
  shape: 1,
  position: 1,
  ink: 1,
  score: 1,
});

describe("strokeCountNote", () => {
  it("says nothing when the count is right", () => {
    expect(strokeCountNote({ countOk: true, expectedStrokes: 3, givenStrokes: 3 })).toBeNull();
  });

  it("names both counts when a stroke was added", () => {
    expect(strokeCountNote({ countOk: false, expectedStrokes: 3, givenStrokes: 4 })).toBe(
      "This character has 3 strokes, but you wrote 4.",
    );
  });

  it("names both counts when a stroke is missing, singular for one", () => {
    expect(strokeCountNote({ countOk: false, expectedStrokes: 1, givenStrokes: 2 })).toBe(
      "This character has 1 stroke, but you wrote 2.",
    );
    expect(strokeCountNote({ countOk: false, expectedStrokes: 4, givenStrokes: 3 })).toBe(
      "This character has 4 strokes, but you wrote 3.",
    );
  });
});

describe("everyStrokeFine", () => {
  const correct = [stroke(0, "correct"), stroke(1, "correct"), stroke(2, "correct")];

  it("is true for a correct attempt of the right count", () => {
    expect(everyStrokeFine({ countOk: true, strokes: correct })).toBe(true);
  });

  it("is false for correct strokes plus an extra one", () => {
    // あ with one extra mark: every row correct, the count wrong.
    expect(everyStrokeFine({ countOk: false, strokes: correct })).toBe(false);
  });

  it("is false when a stroke is faulty or missing", () => {
    const faulty = [stroke(0, "correct"), stroke(1, "out_of_order"), stroke(2, "correct")];
    expect(everyStrokeFine({ countOk: true, strokes: faulty })).toBe(false);
    const missing = [stroke(0, "correct"), stroke(1, "correct"), stroke(2, "missing")];
    expect(everyStrokeFine({ countOk: false, strokes: missing })).toBe(false);
  });
});
