import { describe, expect, it } from "vitest";
import { BOARD_FRACTIONS, boardFractionOf } from "./boardSize";
import type { BoardSize } from "./types";

describe("boardFractionOf", () => {
  it("draws each of the three sizes at a different size, smallest first", () => {
    const compact = boardFractionOf("compact");
    const normal = boardFractionOf("normal");
    const large = boardFractionOf("large");
    expect(compact).toBeLessThan(normal);
    expect(normal).toBeLessThan(large);
  });

  it("never asks for more than the room there is", () => {
    for (const fraction of Object.values(BOARD_FRACTIONS)) {
      expect(fraction).toBeGreaterThan(0);
      expect(fraction).toBeLessThanOrEqual(1);
    }
    expect(boardFractionOf("large")).toBe(1);
  });

  it("draws a size it does not know at the default", () => {
    expect(boardFractionOf(undefined)).toBe(boardFractionOf("normal"));
    expect(boardFractionOf(null)).toBe(boardFractionOf("normal"));
    expect(boardFractionOf("huge" as BoardSize)).toBe(boardFractionOf("normal"));
    // An inherited name is not a size either.
    expect(boardFractionOf("toString" as BoardSize)).toBe(boardFractionOf("normal"));
    expect(boardFractionOf("constructor" as BoardSize)).toBe(boardFractionOf("normal"));
    expect(boardFractionOf("__proto__" as BoardSize)).toBe(boardFractionOf("normal"));
  });

  it("works where Object.hasOwn does not exist", () => {
    // Safari before 15.4 — macOS 12.0–12.2 and iOS 15.0–15.3, which the app
    // supports — has no `Object.hasOwn`, and the build does not polyfill it: a
    // call to it there throws while the app is first drawn.
    const hasOwn = Object.getOwnPropertyDescriptor(Object, "hasOwn");
    Reflect.deleteProperty(Object, "hasOwn");
    try {
      expect(boardFractionOf("large")).toBe(1);
      expect(boardFractionOf("compact")).toBe(BOARD_FRACTIONS.compact);
      expect(boardFractionOf("toString" as BoardSize)).toBe(BOARD_FRACTIONS.normal);
    } finally {
      if (hasOwn) Object.defineProperty(Object, "hasOwn", hasOwn);
    }
  });
});
