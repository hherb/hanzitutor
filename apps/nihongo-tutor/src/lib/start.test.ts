import { describe, expect, it } from "vitest";

import { ALIKE_PAGE, ALIKE_READINGS, alikeNote } from "./start";

/**
 * The Start screen's arithmetic, asserted without a window.
 *
 * Two of these are properties of the demonstration rather than formatting. The
 * readings must be kana, because `words_of_reading` refuses anything else and a
 * romaji entry here would blank the screen's evidence; and a count larger than a
 * page must not be reported as a complete list.
 */

/** The kana the command accepts, which is what a reading is written in. */
const KANA = /^[\u3041-\u309F\u30A1-\u30FA\u30FC]+$/;

describe("the readings the screen demonstrates with", () => {
  it("are kana, because a reading is", () => {
    expect(ALIKE_READINGS.length).toBeGreaterThan(0);
    for (const reading of ALIKE_READINGS) {
      expect(KANA.test(reading), `${reading} is not written in kana`).toBe(true);
    }
  });

  it("are no larger than a page, so the screen shows what it counts", () => {
    // The screen must draw **every** word of each reading it demonstrates, or the
    // note it prints would describe a list it truncated. かみ is the larger of the
    // two at five words in the committed artifact — pinned by
    // `crates/nihongo-core/tests/words_artifact.rs` — so a page that fell to four
    // would be a screen that silently dropped one.
    expect(ALIKE_PAGE).toBeGreaterThanOrEqual(5);
  });
});

describe("alikeNote", () => {
  it("states a reading the course does not carry rather than drawing it empty", () => {
    expect(alikeNote(0, ALIKE_PAGE, "ぬれ")).toBe("Nothing this course teaches is read ぬれ.");
  });

  it("counts in words, and in the singular when there is one", () => {
    expect(alikeNote(1, ALIKE_PAGE, "はな")).toBe("One word this course teaches is read はな.");
    expect(alikeNote(3, ALIKE_PAGE, "はし")).toBe("3 words this course teaches are read はし.");
  });

  it("says so when more words carry the reading than a page draws", () => {
    expect(alikeNote(60, 12, "かみ")).toBe(
      "60 words this course teaches are read かみ; the first 12 are below.",
    );
    // The boundary: exactly a page is a complete list and says nothing extra.
    expect(alikeNote(12, 12, "かみ")).toBe("12 words this course teaches are read かみ.");
  });

  it("groups a large count so it reads as a number", () => {
    // Compared against `toLocaleString` rather than a literal: the separator is
    // the machine's, and the test is about using it rather than about which one.
    expect(alikeNote(1_234, 12, "かみ")).toContain(`${(1_234).toLocaleString()} words`);
  });
});
