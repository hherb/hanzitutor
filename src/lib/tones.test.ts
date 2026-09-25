import { afterEach, describe, expect, it } from "vitest";
import {
  TONES,
  characterToneCount,
  characterTonesOf,
  setCharacterTones,
  syllableTonesOf,
  toneFromSyllable,
  toneOf,
  type Tone,
} from "./tones.svelte";

/** Leave the module-level table as it was found. */
afterEach(() => setCharacterTones([]));

describe("the tone a single syllable is written with", () => {
  it("reads the mark", () => {
    expect(toneFromSyllable("mā")).toBe(1);
    expect(toneFromSyllable("má")).toBe(2);
    expect(toneFromSyllable("mǎ")).toBe(3);
    expect(toneFromSyllable("mà")).toBe(4);
    expect(toneFromSyllable("xué")).toBe(2);
    expect(toneFromSyllable("lǜ"), "the umlaut carries the mark too").toBe(4);
    expect(toneFromSyllable("ér"), "an r-coda is still one syllable").toBe(2);
    expect(toneFromSyllable("ń"), "嗯 is a syllable with no vowel letter").toBe(1);
  });

  it("calls an unmarked syllable neutral", () => {
    // A reading with no mark at all *is* the neutral tone — that is what the
    // spelling means, in this app and in a dictionary alike.
    expect(toneFromSyllable("de")).toBe(5);
    expect(toneFromSyllable("ma")).toBe(5);
    expect(toneFromSyllable("me")).toBe(5);
  });

  it("refuses a run that is not one syllable", () => {
    // Two marks is a pair run together: reading either of them as *the* tone
    // would colour a word from half its evidence. `xuéxí` has a split, and the
    // split comes from Rust.
    expect(toneFromSyllable("xuéxí")).toBeNull();
    expect(toneFromSyllable("nǐhǎo")).toBeNull();
    // No vowel and no mark: a separator, punctuation, a stray consonant, or
    // nothing at all. There is no tone to show.
    expect(toneFromSyllable("")).toBeNull();
    expect(toneFromSyllable("n")).toBeNull();
    expect(toneFromSyllable("·")).toBeNull();
    expect(toneFromSyllable(" ")).toBeNull();
  });

  it("understands every accented letter the Rust side does", () => {
    // The two tables have to agree: a mark written into a reading by Rust and
    // not read here would colour a character with a tone it is not read with.
    // Pinned as data so a mark added to one side only is a failure here.
    const table: Record<number, string[]> = {
      1: ["ā", "ē", "ī", "ō", "ū", "ǖ", "ń", "ḿ"],
      2: ["á", "é", "í", "ó", "ú", "ǘ"],
      3: ["ǎ", "ě", "ǐ", "ǒ", "ǔ", "ǚ", "ň"],
      4: ["à", "è", "ì", "ò", "ù", "ǜ", "ǹ"],
    };
    for (const tone of [1, 2, 3, 4]) {
      for (const letter of table[tone]) {
        expect(toneFromSyllable(letter), `${letter} is ${tone}`).toBe(tone);
      }
    }
    // Twenty-eight marks over the four full tones is what the Rust table holds
    // (8 + 6 + 7 + 7); a change to either table should make somebody read this.
    expect(table[1].length + table[2].length + table[3].length + table[4].length).toBe(28);
  });

  it("numbers the tones in the order the interface lists them", () => {
    // The numbers themselves are the protocol with Rust — 5 is the neutral tone
    // there and here — so they are asserted rather than left to the type.
    expect(TONES).toEqual([1, 2, 3, 4, 5]);
  });
});

describe("the character table sent from Rust", () => {
  it("answers for a character shown on its own", () => {
    setCharacterTones([
      ["一", 1],
      ["的", 5],
      ["好", 3],
    ]);
    expect(characterToneCount()).toBe(3);
    expect(toneOf("的")).toBe(5);
    expect(toneOf("好")).toBe(3);
    expect(toneOf("未")).toBeNull();
  });

  it("replaces rather than merges, and drops what it cannot read", () => {
    setCharacterTones([["一", 1]]);
    setCharacterTones([["好", 3]]);
    expect(characterToneCount()).toBe(1);
    expect(toneOf("一"), "a stale tone must not survive a reload").toBeNull();

    // A tone outside 1..=5 is not a colour to invent, and an empty character is
    // not a character.
    setCharacterTones([
      ["一", 0],
      ["二", 9],
      ["", 1],
      ["三", 3],
    ]);
    expect(characterToneCount()).toBe(1);
    expect(toneOf("三")).toBe(3);
  });
});

describe("the tone of every character of a piece of text", () => {
  it("prefers the reading the text is shown with", () => {
    setCharacterTones([
      ["重", 4],
      ["行", 2],
    ]);
    // The word's own reading is the answer where it was sent, because a
    // polyphone's tone in a word is not always its tone alone. 银行 happens to
    // agree with the table here; the point is which one is consulted.
    expect(characterTonesOf("银行", [2, 2])).toEqual([2, 2]);
    // 重要 is `zhòngyào` — tone 4 on 重 either way, and the word's 1 on 要.
    expect(characterTonesOf("重要", [4, 4])).toEqual([4, 4]);
  });

  it("falls back to the table where the reading has nothing", () => {
    setCharacterTones([
      ["银", 2],
      ["行", 2],
      ["重", 4],
    ]);
    // No tones at all: a lesson list, a radical's family, a search result.
    expect(characterTonesOf("银行")).toEqual([2, 2]);
    // A character the table does not know has no tone, and is left unpainted
    // rather than given one.
    expect(characterTonesOf("银未")).toEqual([2, null]);
    // A short array is the caller's bug; the per-position fallback keeps it a
    // missing colour on one glyph rather than a crash or a wrong one.
    expect(characterTonesOf("银行", [4])).toEqual([4, 2]);
    // A value the caller could not have got from Rust is not a tone.
    expect(characterTonesOf("银", [0])).toEqual([2]);
  });

  it("reads one entry per character, not one per run", () => {
    // The split is Rust's job, and this trusts the pairing it is handed: three
    // characters and three tones is three colours.
    setCharacterTones([["学", 2], ["习", 2], ["好", 3]]);
    expect(characterTonesOf("学习好", [2, 2, 3])).toHaveLength(3);
    expect(characterTonesOf("学习好")).toEqual([2, 2, 3]);
  });
});

describe("the tone of each syllable of pinyin", () => {
  it("colours syllable by syllable where Rust sent the split", () => {
    expect(syllableTonesOf("xuéxí", ["xué", "xí"], [2, 2])).toEqual([2, 2]);
    expect(syllableTonesOf("nǐhǎo", ["nǐ", "hǎo"], [3, 3])).toEqual([3, 3]);
    // 的话 is `dehuà`: a neutral syllable followed by a full one, which is the
    // case a split driven by tone marks alone gets wrong.
    expect(syllableTonesOf("dehuà", ["de", "huà"], [5, 4])).toEqual([5, 4]);
  });

  it("falls back to the syllable's own mark, and refuses a run of several", () => {
    // One syllable with no split sent: the mark on it is the answer.
    expect(syllableTonesOf("hǎo")).toEqual([3]);
    expect(syllableTonesOf("de")).toEqual([5]);
    // Several syllables and no split: refused rather than read from one mark.
    expect(syllableTonesOf("xuéxí")).toEqual([null]);
    // A split Rust sent but did not tone falls back to the mark on the syllable.
    expect(syllableTonesOf("xuéxí", ["xué", "xí"], [])).toEqual([2, 2]);
  });
});
