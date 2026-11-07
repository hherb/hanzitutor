import { describe, expect, it } from "vitest";

import { summariseWord } from "./wordAttempt";

describe("summariseWord", () => {
  it("averages the characters and calls a recalled word recalled", () => {
    const word = summariseWord([
      { report: { overall: 80 }, mode: "recall" },
      { report: { overall: 60 }, mode: "recall" },
    ]);
    expect(word.scores).toEqual([80, 60]);
    expect(word.mean).toBe(70);
    expect(word.traced).toBe(false);
  });

  it("calls a word traced when any one character was traced", () => {
    const word = summariseWord([
      { report: { overall: 95 }, mode: "recall" },
      { report: { overall: 99 }, mode: "trace" },
    ]);
    expect(word.traced).toBe(true);
  });

  it("reads a slot with no recorded mode as recall, as the backend does", () => {
    expect(summariseWord([{ report: { overall: 90 } }]).traced).toBe(false);
  });
});
