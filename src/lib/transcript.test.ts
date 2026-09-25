/**
 * What the recognition block shows, pinned from both sides.
 *
 * The rule is asymmetric on purpose — substitute where the sound matched, never
 * where it did not — and the second half is the one that would be tempting to get
 * wrong, because a blanket substitution reads better and erases real feedback.
 * These tests are the two directions of that rule plus the cases where the halves
 * cannot be paired at all.
 */

import { describe, expect, it } from "vitest";
import { transcriptDisplay } from "./transcript";
import type { Heard, HeardSyllable } from "./types";

/**
 * One transcription syllable, as Rust sends it.
 *
 * `reading` is the model's syllable as the dictionary writes it and
 * `wantedReading` the target's; both default to the plain forms, so a test that is
 * not about tone gets no tone information to reason about.
 */
function syllable(
  base: string,
  wanted: string,
  reading = base,
  wantedReading = reading,
): HeardSyllable {
  return { base, reading, wanted, wantedReading, matches: base === wanted };
}

/** A `Heard` with everything filled in except what a test is about. */
function heard(text: string, syllables: HeardSyllable[], overrides: Partial<Heard> = {}): Heard {
  return {
    text,
    base: syllables.map((s) => s.base).join(" "),
    syllables,
    matched: syllables.filter((s) => s.matches).length,
    sameCount: true,
    detail: "",
    ...overrides,
  };
}

describe("a homophone the model preferred", () => {
  it("shows the character that was asked for, not the model's", () => {
    // 是 was asked for; the model wrote 事 — the same sound `shi`. The sound is
    // what was measured, so the target's character is the honest thing to show.
    const display = transcriptDisplay(heard("事", [syllable("shi", "shi")]), ["是"]);
    expect(display.text).toBe("是");
    expect(display.substituted).toBe(true);
    expect(display.characters).toEqual(["是"]);
  });

  it("says nothing when the model happened to agree", () => {
    const display = transcriptDisplay(heard("是", [syllable("shi", "shi")]), ["是"]);
    expect(display.text).toBe("是");
    expect(display.substituted).toBe(false);
  });

  it("substitutes only the syllable whose homophone the model preferred", () => {
    // 学 matched and the model wrote 学 anyway — nothing to substitute. 生 was
    // heard correctly (`sheng`) and the model wrote the homophone 声, so that one
    // shows the character the exercise asked for.
    const display = transcriptDisplay(
      heard("学声", [syllable("xue", "xue"), syllable("sheng", "sheng")]),
      ["学", "生"],
    );
    expect(display.text).toBe("学生");
    expect(display.characters).toEqual(["学", "生"]);
    expect(display.substituted).toBe(true);
  });
});

describe("a genuinely different syllable", () => {
  it("keeps the model's character, because that is the evidence", () => {
    // 四 was heard where 是 was asked for: `si` against `shi`. This is the report
    // the block exists to make, and substituting here would erase it.
    const display = transcriptDisplay(heard("四", [syllable("si", "shi")]), ["是"]);
    expect(display.text).toBe("四");
    expect(display.characters).toEqual([null]);
    expect(display.substituted).toBe(false);
  });

  it("keeps the mismatched character in a partly matching word", () => {
    // 学生 asked for; the model heard `xue` (right) and `shi` (wrong for `sheng`),
    // so it wrote 学士. The first syllable matched, so it shows the target; the
    // second did not, so the model's character stands and the chart marks it.
    const display = transcriptDisplay(
      heard("学士", [syllable("xue", "xue"), syllable("shi", "sheng")]),
      ["学", "生"],
    );
    // 生 is *not* shown: `shi` and `sheng` are different sounds, so this is not a
    // homophone and there is nothing to substitute.
    expect(display.text).toBe("学士");
    expect(display.characters).toEqual(["学", null]);
    // `substituted` is about homophones acted on, not about "something differed" —
    // the mismatch is reported through `characters`, the badge and the chart.
    expect(display.substituted).toBe(false);
  });
});

describe("a transcription that cannot be paired with the target", () => {
  it("returns the model's text when it cannot be divided", () => {
    // `sameCount` false: the reading would not split one syllable per character,
    // so no per-syllable comparison was made and none may be invented.
    const display = transcriptDisplay(
      heard("学生", [syllable("xue", "xue")], { sameCount: false }),
      ["学", "生"],
    );
    expect(display.text).toBe("学生");
    expect(display.characters).toEqual([]);
    expect(display.substituted).toBe(false);
  });

  it("returns the model's text when the syllable counts disagree", () => {
    const display = transcriptDisplay(heard("是是", [syllable("shi", "shi"), syllable("shi", "shi")]), [
      "是",
    ]);
    expect(display.text).toBe("是是");
    expect(display.substituted).toBe(false);
  });

  it("never drops characters to force the two to line up", () => {
    // Three characters against one syllable: the lengths cannot agree, so nothing
    // is substituted and the transcription is shown as the model wrote it.
    const display = transcriptDisplay(heard("是不是", [syllable("shi", "shi")]), ["是"]);
    expect(display.text).toBe("是不是");
    expect(display.characters).toEqual([]);
  });

  it("handles nothing recognised at all", () => {
    expect(transcriptDisplay(heard("", []), ["是"])).toEqual({
      text: "",
      reading: "",
      differs: false,
      characters: [],
      substituted: false,
    });
    expect(transcriptDisplay(null, ["是"])).toEqual({
      text: "",
      reading: "",
      differs: false,
      characters: [],
      substituted: false,
    });
  });
});

describe("the reading shown beside the characters", () => {
  it("gives the transcription its dictionary tone marks when it differs from the target", () => {
    // `cong` alone is 从, 葱 or 匆. The mark identifies the character the model
    // wrote, which is the point of showing the reading at all.
    const display = transcriptDisplay(heard("从", [syllable("cong", "zhong", "cóng")]), ["中"]);
    expect(display.differs).toBe(true);
    expect(display.reading).toBe("cóng");
  });

  it("marks a syllable whose sound matched when the word around it differs", () => {
    // 学士 for 学生: the first syllable is right and the second is not, so the
    // whole transcription differs and both syllables show what was recognised.
    const display = transcriptDisplay(
      heard("学士", [syllable("xue", "xue", "xué"), syllable("shi", "sheng", "shì")]),
      ["学", "生"],
    );
    expect(display.differs).toBe(true);
    expect(display.reading).toBe("xué shì");
  });

  it("shows a tone the sound comparison cannot see, with the model's own character", () => {
    // 中果 for 中国: both sounds matched once the tone was set aside, so the
    // syllable comparison calls this right. The tone on 果 is the only place the
    // difference shows, so the model's character and reading are shown together —
    // 国 above `guǒ` would contradict itself.
    const display = transcriptDisplay(
      heard("中果", [
        syllable("zhong", "zhong", "zhōng", "zhōng"),
        syllable("guo", "guo", "guǒ", "guó"),
      ]),
      ["中", "国"],
    );
    expect(display.differs).toBe(true);
    expect(display.reading).toBe("zhōng guǒ");
    expect(display.text).toBe("中果");
    expect(display.characters).toEqual(["中", null]);
  });

  it("keeps the target's character where the model agreed on sound and tone", () => {
    // 学声 for 学生: homophones, so the spelling was never in question and 生 is
    // shown — with the model's reading, which agrees down to the tone.
    const display = transcriptDisplay(
      heard("学声", [
        syllable("xue", "xue", "xué", "xué"),
        syllable("sheng", "sheng", "shēng", "shēng"),
      ]),
      ["学", "生"],
    );
    expect(display.differs).toBe(true);
    expect(display.reading).toBe("xué shēng");
    expect(display.text).toBe("学生");
    expect(display.substituted).toBe(true);
  });

  it("keeps the letters plain when the transcription is what was asked for", () => {
    const display = transcriptDisplay(heard("马", [syllable("ma", "ma", "mǎ", "mǎ")]), ["马"]);
    expect(display.differs).toBe(false);
    expect(display.reading).toBe("ma");
  });

  it("falls back to the model's plain reading when the halves cannot be paired", () => {
    const display = transcriptDisplay(
      heard("学生", [syllable("xue", "xue", "xué")], { sameCount: false, base: "xue" }),
      ["学", "生"],
    );
    expect(display.differs).toBe(false);
    expect(display.reading).toBe("xue");
  });
});
