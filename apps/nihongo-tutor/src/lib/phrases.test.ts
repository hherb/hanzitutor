import { describe, expect, it } from "vitest";

import { firstNonEmptyBand, phraseSource, spokenPhrase, totalPhrases } from "./phrases";
import type { PhraseBandView, PhraseView } from "./types";

/**
 * The Phrases screen's own rules.
 *
 * `firstNonEmptyBand` is the one that matters. The corpus is filtered rather than
 * authored, so a band can be empty, and a screen that opened on band 1 regardless
 * would show nothing while looking like it worked.
 */

function band(band: number, phrases: number): PhraseBandView {
  return { band, name: `band ${band}`, phrases };
}

function phrase(overrides: Partial<PhraseView> = {}): PhraseView {
  return {
    id: 1297,
    text: "きみにちょっとしたものをもってきたよ。",
    english: "I brought you a little something.",
    author: "xtofu80",
    licence: "CC BY 2.0 FR",
    tokens: [],
    ...overrides,
  };
}

describe("firstNonEmptyBand", () => {
  it("opens on the first band the corpus filled", () => {
    expect(firstNonEmptyBand([band(1, 0), band(2, 3), band(3, 9)])).toBe(2);
  });

  it("opens on band 1 when band 1 has phrases", () => {
    expect(firstNonEmptyBand([band(1, 4), band(2, 3)])).toBe(1);
  });

  it("says so rather than guessing when no band holds anything", () => {
    expect(firstNonEmptyBand([band(1, 0), band(7, 0)])).toBeNull();
    expect(firstNonEmptyBand([])).toBeNull();
  });
});

describe("totalPhrases", () => {
  it("adds every band up, empty ones included", () => {
    expect(totalPhrases([band(1, 0), band(2, 3), band(7, 5)])).toBe(8);
    expect(totalPhrases([])).toBe(0);
  });
});

describe("phraseSource", () => {
  it("names the corpus id, the contributor and the licence the licence requires", () => {
    expect(phraseSource(phrase())).toBe("Tatoeba #1297 · xtofu80 · CC BY 2.0 FR");
  });

  it("does not drop a phrase's own id when the author name is odd", () => {
    // Tatoeba usernames contain spaces, brackets and non-Latin script, and none of
    // that may cost the sentence its handle.
    const line = phraseSource(
      phrase({ id: 10359138, author: "カラス (Karasu)", licence: "CC0 1.0" }),
    );
    expect(line).toBe("Tatoeba #10359138 · カラス (Karasu) · CC0 1.0");
    expect(line).toContain("10359138");
  });
});

describe("spokenPhrase", () => {
  /**
   * Tatoeba #2771616 as the artifact holds it and as the report that found this bug
   * drew it: お腹痛いの？ with the ruby おなか over お腹 and いたい over 痛い.
   */
  const reported = phrase({
    id: 2771616,
    text: "お腹痛いの？",
    english: "Does your stomach hurt?",
    author: "tommy_san",
    tokens: [
      { surface: "お腹", rt: "おなか", word: "お腹" },
      { surface: "痛い", rt: "いたい", word: "痛い" },
      { surface: "の", rt: null, word: null },
      { surface: "？", rt: null, word: null },
    ],
  });

  it("joins the artifact's readings, and the parts spell the sentence", () => {
    expect(spokenPhrase(reported)).toBe("おなかいたいの？");
    expect(spokenPhrase(reported)).not.toBe(reported.text);
  });

  it("does not hand the voice a kanji it would read its own way", () => {
    // The bug this exists to stop: `Hear it` used to pass `text`, and Apple's
    // Japanese front end says お + 腹痛(ふくつう) + い for it. Nothing here chooses a
    // reading — each token's `rt` is the artifact's — but the voice is not asked to.
    const spoken = spokenPhrase(reported);
    expect(spoken).not.toContain("腹");
    expect(spoken).not.toContain("痛");
    expect(spoken).not.toContain("ふくつう");
  });

  it("reads a kana-only token as itself", () => {
    expect(
      spokenPhrase(
        phrase({ tokens: [{ surface: "きみ", rt: null, word: null }, { surface: "に", rt: null, word: null }] }),
      ),
    ).toBe("きみに");
  });

  it("says nothing for a phrase with no tokens rather than 'undefined'", () => {
    expect(spokenPhrase(phrase({ tokens: [] }))).toBe("");
  });
});
