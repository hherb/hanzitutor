import { describe, expect, it } from "vitest";

import { canSubmit, outcomeLine, poolNote, wholeSentence } from "./grammarProduction";
import type { GrammarProductionQuestion, GrammarProductionVerdict } from "./types";

/**
 * The first question of the course, as `next_grammar_production` sends it — which is
 * to say with the answer absent: 犬の方が猫より好きです。 arrives as its two halves and
 * its English, and the pattern, the meaning and the text taken out are not here
 * because Rust does not send them.
 */
const question: GrammarProductionQuestion = {
  point: "no-hou-ga-yori",
  example: 0,
  before: [{ ruby: "犬", rt: "いぬ" }],
  after: [
    { ruby: "猫", rt: "ねこ" },
    { ruby: "より", rt: null },
    { ruby: "好", rt: "す" },
    { ruby: "きです。", rt: null },
  ],
  en: "I like dogs more than cats.",
};

const verdict = (over: Partial<GrammarProductionVerdict> = {}): GrammarProductionVerdict => ({
  point: "no-hou-ga-yori",
  pattern: "〜の方が〜より",
  reading: "〜のほうが〜より",
  meaning: "X is more ... than Y",
  formation: "Noun + の方が + Noun + より",
  notes: null,
  written: "の方が",
  answer: "のほうが",
  produced: "のほうが",
  correct: true,
  ...over,
});

describe("canSubmit", () => {
  it("sends nothing for an empty box, and everything else", () => {
    expect(canSubmit("")).toBe(false);
    expect(canSubmit("   ")).toBe(false);
    expect(canSubmit("nohouga")).toBe(true);
    expect(canSubmit("のほうが")).toBe(true);
  });
});

describe("wholeSentence", () => {
  it("puts the corpus's own text back between the two halves", () => {
    // The halves carry the furigana for the kanji around the hole, so the string
    // `Hear it` speaks is the sentence as the corpus wrote it.
    expect(wholeSentence(question, "の方が")).toBe("犬の方が猫より好きです。");
  });

  it("is the two halves alone when the hole is at an end", () => {
    const atTheEnd: GrammarProductionQuestion = {
      point: "te-kudasai",
      example: 1,
      before: [{ ruby: "ちょっと待っ", rt: "ちょっとまっ" }],
      after: [],
      en: "Please wait a moment.",
    };
    expect(wholeSentence(atTheEnd, "てください")).toBe("ちょっと待ってください");
  });
});

describe("outcomeLine", () => {
  it("names the reading the corpus gives the hole when the answer is right", () => {
    expect(outcomeLine(verdict())).toBe("Yes — the hole is のほうが, written の方が.");
  });

  it("says it once when the hole holds no kanji", () => {
    // あまり is written as it is read; "the hole is あまり, written あまり" would be
    // noise about a distinction the learner cannot see.
    expect(outcomeLine(verdict({ written: "あまり", answer: "あまり", produced: "あまり" }))).toBe(
      "Yes — the hole is あまり.",
    );
  });

  it("says what was written instead, and what the hole was", () => {
    const line = outcomeLine(verdict({ correct: false, produced: "より" }));
    expect(line).toContain("the hole is のほうが, written の方が");
    expect(line).toContain("You wrote より");
  });

  it("says when the typing could not be read at all", () => {
    // `produced` is empty when the input engine could not read any of it, and "you
    // wrote " would say nothing about what happened.
    const line = outcomeLine(verdict({ correct: false, produced: "" }));
    expect(line).toContain("Nothing you typed could be read as kana");
    expect(line).not.toContain("You wrote");
  });
});

describe("poolNote", () => {
  it("states the measured share, which is the drill's own pool", () => {
    const line = poolNote({ points: 363, examples: 903, total: 526 });
    expect(line).toContain("363 of the 526 patterns can be written");
    expect(line).toContain("903 sentences");
    expect(line).toContain("the rest");
  });

  it("says so plainly when every pattern can be written", () => {
    expect(poolNote({ points: 526, examples: 1526, total: 526 })).toBe(
      "Every one of the 526 patterns can be written, from 1526 sentences.",
    );
  });

  it("does not promise a course that is not there", () => {
    expect(poolNote({ points: 0, examples: 0, total: 0 })).toBe(
      "No pattern in this course can be written this way yet.",
    );
    expect(poolNote({ points: 0, examples: 0, total: 526 })).toContain("No pattern");
  });
});
