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
  });
});
