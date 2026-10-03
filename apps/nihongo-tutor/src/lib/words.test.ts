import { describe, expect, it } from "vitest";
import {
  foldKana,
  furiganaCoversWord,
  furiganaSpellsReading,
  MAX_WORD_PAGE,
  needsRuby,
  pageWindow,
  readingOf,
  tokenIsTappable,
  tokenReading,
} from "./words";
import type { Word } from "./types";

function word(overrides: Partial<Word> = {}): Word {
  return {
    text: "食べる",
    reading: "たべる",
    meaning: "to eat",
    band: 2,
    bandName: "kyōiku 2",
    nf: 25,
    furigana: [
      { ruby: "食", rt: "た" },
      { ruby: "べる", rt: null },
    ],
    ...overrides,
  };
}

describe("furigana", () => {
  it("knows which segments need ruby and which are already kana", () => {
    expect(needsRuby({ ruby: "食", rt: "た" })).toBe(true);
    expect(needsRuby({ ruby: "べる", rt: null })).toBe(false);
    // An empty reading is not a reading: it would draw an empty <rt>.
    expect(needsRuby({ ruby: "食", rt: "" })).toBe(false);
  });

  it("spells the word's reading from its segments", () => {
    expect(readingOf(word())).toBe("たべる");
    // 大人 is one segment covering two characters, because おとな cannot be cut.
    expect(
      readingOf(
        word({ text: "大人", reading: "おとな", furigana: [{ ruby: "大人", rt: "おとな" }] }),
      ),
    ).toBe("おとな");
  });

  it("falls back to the stored reading when there is no furigana at all", () => {
    const bare = word({ text: "等閑", reading: "なおざり", furigana: [] });
    expect(readingOf(bare)).toBe("なおざり");
    // And a word with no furigana has nothing to spell the reading with.
    expect(furiganaSpellsReading(bare)).toBe(false);
    // But it is not *misaligned*: there is simply no ruby to draw.
    expect(furiganaCoversWord(bare)).toBe(true);
  });

  it("accepts a reading spelled in the other kana type", () => {
    // 生ゴミ: the written form decides the kana type, the dictionary the reading.
    const namagomi = word({
      text: "生ゴミ",
      reading: "なまごみ",
      furigana: [
        { ruby: "生", rt: "なま" },
        { ruby: "ゴミ", rt: null },
      ],
    });
    expect(readingOf(namagomi)).toBe("なまゴミ");
    expect(readingOf(namagomi)).not.toBe(namagomi.reading);
    expect(furiganaSpellsReading(namagomi)).toBe(true);
  });

  it("notices furigana that covers the wrong characters", () => {
    const misaligned = word({ furigana: [{ ruby: "飲", rt: "の" }] });
    expect(furiganaCoversWord(misaligned)).toBe(false);
    expect(furiganaCoversWord(word())).toBe(true);
  });
});

describe("foldKana", () => {
  it("folds katakana to hiragana and leaves everything else alone", () => {
    expect(foldKana("タベル")).toBe("たべる");
    expect(foldKana("がくせい")).toBe("がくせい");
    expect(foldKana("コーヒー")).toBe("こーひー");
    expect(foldKana("食べる")).toBe("食べる");
    // ー is a length mark belonging to either script, and ゛ is not a kana.
    expect(foldKana("ー")).toBe("ー");
  });

  it("is what makes a katakana answer right", () => {
    // A learner typing ガクセイ has answered correctly for がくせい.
    expect(foldKana("ガクセイ")).toBe(foldKana("がくせい"));
  });
});

describe("pageWindow", () => {
  it("walks the pages of a band and clamps to what exists", () => {
    expect(pageWindow(100, 10, 1)).toEqual({ offset: 0, limit: 10, page: 1, pages: 10 });
    expect(pageWindow(100, 10, 3)).toEqual({ offset: 20, limit: 10, page: 3, pages: 10 });
    // Past the end clamps to the last page rather than asking for nothing.
    expect(pageWindow(100, 10, 99)).toEqual({ offset: 90, limit: 10, page: 10, pages: 10 });
    // Before the start clamps to the first.
    expect(pageWindow(100, 10, 0)).toEqual({ offset: 0, limit: 10, page: 1, pages: 10 });
  });

  it("never asks for more than the command will serve", () => {
    const window = pageWindow(10_000, 5_000, 1);
    expect(window.limit).toBe(MAX_WORD_PAGE);
    expect(window.offset).toBe(0);
    // 10,000 words at 200 a page.
    expect(window.pages).toBe(50);
  });

  it("treats an empty list as one empty page rather than none", () => {
    expect(pageWindow(0, 10, 1)).toEqual({ offset: 0, limit: 10, page: 1, pages: 1 });
  });
});

describe("passage tokens", () => {
  it("reads as its ruby when it has one and as itself when it does not", () => {
    expect(tokenReading({ surface: "学生", rt: "がくせい", word: "学生" })).toBe("がくせい");
    expect(tokenReading({ surface: "は", rt: null, word: null })).toBe("は");
  });

  it("is tappable only when it links to a word the course teaches", () => {
    expect(tokenIsTappable({ surface: "食べ", rt: "たべ", word: "食べる" })).toBe(true);
    expect(tokenIsTappable({ surface: "ます", rt: null, word: null })).toBe(false);
    // A link to an empty string is not a link.
    expect(tokenIsTappable({ surface: "は", rt: null, word: "" })).toBe(false);
  });
});
