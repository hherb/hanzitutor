/**
 * What the Words search box returns, pinned where it broke.
 *
 * The bug these guard against was a pure rule with a visible symptom: typing `z`
 * returned the 心 family, whose only `z` was the second syllable of 心脏病. Every
 * match rule contributed something to that, so each is tested from the side that
 * used to be wrong.
 */

import { describe, expect, it } from "vitest";
import { searchFamilies } from "./familySearch";
import type { WordMember, WordSet } from "./types";

/** One word, with only the fields the search reads spelled out. */
function word(text: string, reading: string, meaning = ""): WordMember {
  const syllables = [...text].length;
  return {
    text,
    reading,
    spoken: Array(syllables).fill(1),
    citation: Array(syllables).fill(1),
    meaning,
    hsk: 1,
    rank: 1,
  };
}

/** One family: a key character, the syllable it is about, and its words. */
function family(key: string, base: string, words: WordMember[]): WordSet {
  return { key, base, contrast: `${key} tone 1`, words };
}

const HEART = family("心", "xin", [
  word("心脏病", "xīnzàngbìng", "heart disease"),
]);
const MIDDLE = family("中", "zhong", [
  word("中国", "Zhōngguó", "China"),
  word("中年", "zhōngnián", "middle age"),
  // 中心 `zhōngxīn` is why the 中 family answers a search for `xin` at all.
  word("中心", "zhōngxīn", "center; heart; core"),
]);
const EXCELLENT = family("优", "you", [word("优化", "yōuhuà", "to optimize")]);

describe("searching word families", () => {
  it("matches a single letter against the family's own syllable, not the words in it", () => {
    // The regression: `z` is in 心脏病, so the 心 family used to come back for it —
    // under a header that reads 心 and a word list that looks nothing like `z`.
    expect(searchFamilies([HEART], "z")).toEqual([]);
    // 中's own syllable is `zhong`, so `z` is a seed of *it*.
    expect(searchFamilies([MIDDLE], "z")).toEqual([MIDDLE]);
  });

  it("does not search the English definition", () => {
    // "to optimize" has a `z` in it, and a definition is a coincidence about the
    // translation rather than anything about the sound.
    expect(searchFamilies([EXCELLENT], "optimize")).toEqual([]);
    expect(searchFamilies([EXCELLENT], "z")).toEqual([]);
  });

  it("puts a family whose syllable is the query above one that merely contains a word", () => {
    const hits = searchFamilies([MIDDLE, HEART], "xin");
    // 心 is `xin` itself; 中 holds 中心 `zhōngxīn`, so it matches only through a
    // word — and a learner who typed the syllable means the first one.
    expect(hits).toEqual([HEART, MIDDLE]);
  });

  it("finds a family by a word typed as characters", () => {
    expect(searchFamilies([HEART, MIDDLE], "中国")).toEqual([MIDDLE]);
  });

  it("finds a family by a whole word's reading", () => {
    expect(searchFamilies([HEART, MIDDLE], "zhongguo")).toEqual([MIDDLE]);
  });

  it("still reaches a family through a syllable inside one of its words", () => {
    // `ni` is the middle syllable of 中年. A query of two or more letters is a
    // syllable fragment rather than a seed, so this stays a match.
    expect(searchFamilies([HEART, MIDDLE], "ni")).toEqual([MIDDLE]);
  });

  it("folds the tone marks a learner types", () => {
    // 心 comes first because `xin` is its own syllable; 中 follows through 中心.
    expect(searchFamilies([HEART, MIDDLE], "xīn")).toEqual([HEART, MIDDLE]);
    expect(searchFamilies([HEART, MIDDLE], "Zhōngguó")).toEqual([MIDDLE]);
  });

  it("returns the whole list for an empty query", () => {
    const families = [HEART, MIDDLE, EXCELLENT];
    expect(searchFamilies(families, "")).toBe(families);
    expect(searchFamilies(families, "   ")).toBe(families);
  });
});
