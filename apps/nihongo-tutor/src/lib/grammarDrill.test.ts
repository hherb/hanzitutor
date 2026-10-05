import { describe, expect, it } from "vitest";

import {
  isRight,
  optionState,
  outcomeLine,
  pickedPattern,
  poolNote,
  recordLine,
  sessionLine,
} from "./grammarDrill";
import type { GrammarQuestion, GrammarTally } from "./types";

/** The first question of the course, as `next_grammar_question` sends it. */
const question: GrammarQuestion = {
  point: "wa",
  before: [{ ruby: "私", rt: "わたし" }, { ruby: "は", rt: null }],
  after: [{ ruby: "学生です。", rt: null }],
  ja: "私は学生です。",
  en: "I am a student.",
  pattern: "〜は",
  reading: null,
  meaning: "as for; marks the topic of the sentence",
  formation: "Noun + は",
  notes: "Written は but pronounced 'wa'.",
  options: [
    { id: "wa", pattern: "〜は", reading: null },
    { id: "ga", pattern: "〜が", reading: null },
    { id: "mo", pattern: "〜も", reading: null },
  ],
};

describe("optionState", () => {
  it("keeps every option idle until an answer is picked", () => {
    // The payload names the answer, so the screen could colour it before the learner
    // has answered; that is exactly what must not happen.
    for (const option of question.options) {
      expect(optionState(option, question, null)).toBe("idle");
    }
  });

  it("marks the answer right and the wrong pick alone", () => {
    const [right, wrong, other] = question.options;
    expect(optionState(right, question, "ga")).toBe("right");
    expect(optionState(wrong, question, "ga")).toBe("wrong");
    expect(optionState(other, question, "ga")).toBe("idle");
  });

  it("marks the answer right when it was the one picked", () => {
    expect(optionState(question.options[0], question, "wa")).toBe("right");
    expect(optionState(question.options[1], question, "wa")).toBe("idle");
  });
});

describe("isRight and pickedPattern", () => {
  it("decides rightness from the two ids alone", () => {
    expect(isRight(question, "wa")).toBe(true);
    expect(isRight(question, "ga")).toBe(false);
    // An id that is not among the options is wrong rather than an error: the screen
    // only ever sends one it drew, and Rust decides again on its own side.
    expect(isRight(question, "no-such-pattern")).toBe(false);
  });

  it("gives the picked pattern as the corpus writes it", () => {
    expect(pickedPattern(question, "ga")).toBe("〜が");
    expect(pickedPattern(question, null)).toBeNull();
    expect(pickedPattern(question, "no-such-pattern")).toBeNull();
  });
});

describe("outcomeLine", () => {
  it("names the meaning even when the answer was right", () => {
    expect(outcomeLine(question, "wa")).toBe(
      "Yes — 〜は means as for; marks the topic of the sentence.",
    );
  });

  it("says what was picked, then what the sentence takes", () => {
    const line = outcomeLine(question, "ga");
    expect(line).toContain("〜が");
    expect(line).toContain("〜は");
    expect(line).toContain(question.meaning);
    expect(line.startsWith("〜が is not it")).toBe(true);
  });

  it("does not blame a pattern that is not on screen", () => {
    expect(outcomeLine(question, "gone")).toContain("That is not it");
  });
});

describe("sessionLine", () => {
  it("says nothing about a percentage before the first answer", () => {
    expect(sessionLine(0, 0)).toBe("Nothing answered yet");
    expect(sessionLine(0, 0)).not.toContain("%");
  });

  it("counts the sitting, rounded", () => {
    expect(sessionLine(1, 1)).toBe("1 / 1 · 100%");
    expect(sessionLine(2, 3)).toBe("2 / 3 · 67%");
    expect(sessionLine(0, 4)).toBe("0 / 4 · 0%");
  });
});

describe("recordLine", () => {
  const tally = (asked: number, correct: number, wrong: number): GrammarTally => ({
    point: "wa",
    asked,
    correct,
    wrong,
    weight: 1 + 2 * wrong - correct,
  });

  it("says a missed pattern comes up more often", () => {
    const line = recordLine(tally(3, 1, 2), "〜は");
    expect(line).toContain("missed 2 of 3");
    expect(line).toContain("more often");
  });

  it("says a pattern answered right comes up less often", () => {
    const line = recordLine(tally(2, 2, 0), "〜は");
    expect(line).toContain("2 of 2 right");
    expect(line).toContain("less often");
  });

  it("prefers the miss when a record holds both", () => {
    // A record with any miss in it is a pattern still being learned, and the learner
    // is told the half that changes what happens next.
    expect(recordLine(tally(4, 3, 1), "〜は")).toContain("more often");
  });
});

describe("poolNote", () => {
  it("states the measured share, and why the rest are unreachable", () => {
    const line = poolNote({ points: 363, examples: 903, total: 526 });
    expect(line).toContain("363 of the 526 patterns");
    expect(line).toContain("903 sentences");
    expect(line).toContain("the rest");
  });

  it("says so plainly when every pattern can be asked", () => {
    const line = poolNote({ points: 526, examples: 1526, total: 526 });
    expect(line).toBe("Every one of the 526 patterns can be asked, over 1526 sentences.");
  });

  it("does not promise a course that is not there", () => {
    expect(poolNote({ points: 0, examples: 0, total: 0 })).toBe(
      "No pattern in this course can be asked this way yet.",
    );
    expect(poolNote({ points: 0, examples: 0, total: 526 })).toContain("No pattern");
  });
});
