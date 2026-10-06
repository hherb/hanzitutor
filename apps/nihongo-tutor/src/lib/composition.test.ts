import { describe, expect, it } from "vitest";

import {
  COMPOSITION_MIN,
  canSubmit,
  checks,
  instructionLine,
  keptLine,
  limitationNote,
  ownNote,
  poolNote,
} from "./composition";
import type { CompositionVerdict } from "./types";

/**
 * A verdict of the shape `record_grammar_composition` sends for a sentence that used
 * the pattern, written in kana — the ordinary good case.
 */
const verdict = (over: Partial<CompositionVerdict> = {}): CompositionVerdict => ({
  point: "no-hou-ga-yori",
  pattern: "〜の方が〜より",
  reading: "〜のほうが〜より",
  meaning: "X is more ... than Y",
  formation: "Noun + の方が + Noun + より",
  notes: null,
  text: "いぬのほうがねこよりすきです。",
  found: ["のほうが", "より"],
  characters: 12,
  readable: true,
  outside: [],
  copied: null,
  kept: true,
  ...over,
});

describe("canSubmit", () => {
  it("sends nothing for an empty box, and everything else", () => {
    expect(canSubmit("")).toBe(false);
    expect(canSubmit("   ")).toBe(false);
    expect(canSubmit("いぬのほうがねこよりすきです。")).toBe(true);
  });
});

describe("instructionLine", () => {
  it("names the pattern", () => {
    expect(instructionLine("〜の方が〜より")).toContain("〜の方が〜より");
  });
});

describe("checks", () => {
  it("reports a good sentence as facts, not as a score", () => {
    const lines = checks(verdict(), "いぬのほうがねこよりすきです。");
    expect(lines.map((line) => line.state)).toEqual(["ok", "ok"]);
    expect(lines[0].text).toBe("Readable as Japanese: 12 characters.");
    expect(lines[1].text).toContain("のほうが · より");
    expect(lines.map((line) => line.text).join(" ")).not.toMatch(/\b(correct|wrong)\b/i);
  });

  it("says what the typing was read back as, when that is not what was typed", () => {
    const lines = checks(
      verdict({ text: "いぬのほうがねこよりすきです。", characters: 12 }),
      "inu no houga neko yori suki desu.",
    );
    expect(lines[0]).toEqual({
      state: "note",
      text: "Read back as: いぬのほうがねこよりすきです。",
    });
  });

  it("does not call an inflected pattern a mistake", () => {
    // 〜ている is taught with しています: the pattern's own letters are absent from a
    // sentence the corpus itself wrote, so the line is a note and says why.
    const lines = checks(
      verdict({ pattern: "〜ている", found: [], text: "弟は部屋でゲームをしています。" }),
      "弟は部屋でゲームをしています。",
    );
    const patternLine = lines.find((line) => line.text.includes("〜ている"));
    expect(patternLine?.state).toBe("note");
    expect(patternLine?.text).toContain("not necessarily wrong");
    expect(patternLine?.text).toContain("しています");
  });

  it("calls an input that is not Japanese script the thing to fix", () => {
    const lines = checks(
      verdict({ readable: false, kept: false, text: "私はstudentです。" }),
      "私はstudentです。",
    );
    expect(lines[0].state).toBe("warn");
    expect(lines[0].text).toContain("not Japanese script");
  });

  it("names the bar when the input is not yet a sentence", () => {
    const lines = checks(verdict({ characters: 2, kept: false, text: "はい。" }), "はい。");
    expect(lines[0].state).toBe("warn");
    expect(lines[0].text).toContain("2 characters");
    expect(lines[0].text).toContain(`${COMPOSITION_MIN}`);
    expect(lines[0].text).toContain("a word rather than a sentence");
  });

  it("names the characters the course does not teach, and a corpus sentence written back", () => {
    const lines = checks(
      verdict({ outside: ["几", "嘘"], copied: 1 }),
      "いぬのほうがねこよりすきです。",
    );
    expect(lines.find((line) => line.text.includes("几"))?.text).toContain("no card");
    expect(lines.find((line) => line.text.includes("written back"))?.state).toBe("note");
  });
});

describe("keptLine", () => {
  it("states the one decision there is, and counts the learner's own writing", () => {
    expect(keptLine(verdict(), 3)).toBe("Kept — that is 3 sentences of your own for this pattern.");
    expect(keptLine(verdict(), 1)).toBe("Kept — that is 1 sentence of your own for this pattern.");
  });

  it("says why nothing was kept, and never says it was wrong", () => {
    const short = keptLine(verdict({ kept: false }), 0);
    expect(short).toBe("Not kept: it is shorter than a sentence.");
    const unreadable = keptLine(verdict({ kept: false, readable: false }), 0);
    expect(unreadable).toBe("Not kept: it could not be read as Japanese.");
    expect(`${short} ${unreadable}`).not.toMatch(/\bwrong\b/i);
  });
});

describe("limitationNote", () => {
  it("says what the check cannot see, which is the whole difference from a grade", () => {
    const line = limitationNote();
    expect(line).toContain("cannot tell whether a sentence is grammatical or natural");
    expect(line).toContain("needs a reader");
  });
});

describe("poolNote", () => {
  it("states the reach, which is the whole course, and why", () => {
    expect(poolNote({ points: 526, total: 526 })).toBe(
      "Every one of the 526 patterns can be written about — the whole course, because the prompt is the pattern itself and needs no sentence of it.",
    );
  });

  it("does not promise a course that is not there", () => {
    expect(poolNote({ points: 0, total: 0 })).toBe("No pattern in this course can be written about yet.");
    expect(poolNote({ points: 363, total: 526 })).toBe("363 of the 526 patterns can be written about.");
  });
});

describe("ownNote", () => {
  it("states the learner's own writing, and says nothing over an empty list", () => {
    expect(ownNote([])).toBe("");
    expect(ownNote(["私は学生です。"])).toBe(
      "The one sentence you have written with this pattern:",
    );
    expect(ownNote(["私は学生です。", "今日は天気がいいです。"])).toBe(
      "The 2 sentences you have written with this pattern, oldest first:",
    );
  });
});
