import { describe, expect, it } from "vitest";

import { chartColumns, focusFor, isSingleKana, lessonKeyOf } from "./kana";
import type { ChartView, LessonView } from "./types";

/**
 * The kana screens' arithmetic.
 *
 * These three rules are what make "a kana offered anywhere can be opened on the
 * board" true rather than nearly true: which lesson a kana opened from elsewhere
 * belongs to, how wide the chart's grid is, and whether a drill answer is a kana
 * the board can draw at all.
 */

const lesson = (key: string, kana: string[]): LessonView => ({
  key,
  title: `${kana.join(" ")} — ${key}`,
  kana,
  voiced: false,
  count: kana.length,
});

const course: LessonView[] = [
  lesson("hiragana-a", ["あ", "い", "う", "え", "お"]),
  lesson("hiragana-ya", ["や", "ゆ", "よ"]),
  lesson("hiragana-small", ["ぁ", "ぃ", "ぅ", "ぇ", "ぉ", "ゃ", "ゅ", "ょ", "っ", "ゎ"]),
];

const chart = (rows: (string | null)[][]): ChartView => ({
  script: "hiragana",
  rows: rows.map((cells) => ({ sound: "ka", voiced: false, cells })),
  offGrid: [],
});

describe("lessonKeyOf", () => {
  it("finds the lesson a kana is taught in, not the first lesson", () => {
    expect(lessonKeyOf(course, "ゆ")).toBe("hiragana-ya");
    expect(lessonKeyOf(course, "ゃ")).toBe("hiragana-small");
  });

  it("answers with nothing when the course does not teach it", () => {
    expect(lessonKeyOf(course, "カ")).toBeNull();
    expect(lessonKeyOf([], "あ")).toBeNull();
  });

  it("does not mistake a kana for one it merely contains", () => {
    // き is not in きゃ: a lesson's members are its kana, not its substrings.
    const withDigraph = [lesson("hiragana-small", ["きゃ"])];
    expect(lessonKeyOf(withDigraph, "き")).toBeNull();
    expect(lessonKeyOf(withDigraph, "きゃ")).toBe("hiragana-small");
  });
});

describe("focusFor", () => {
  const hiragana: LessonView[] = [lesson("hiragana-a", ["あ", "い", "う"])];
  const katakana: LessonView[] = [lesson("katakana-a", ["ア", "イ", "ウ"])];

  it("honours a request that belongs to the course being opened", () => {
    expect(focusFor({ ch: "い", script: "hiragana" }, "hiragana", hiragana)).toBe("い");
    expect(focusFor({ ch: "ウ", script: "katakana" }, "katakana", katakana)).toBe("ウ");
  });

  it("refuses a request from the other script, and opens the first kana instead", () => {
    // The case a single un-tagged slot gets wrong: a request left over from a
    // script whose load failed must not be applied to whichever course loads next,
    // or the board shows one script's kana under the other's lessons.
    expect(focusFor({ ch: "い", script: "hiragana" }, "katakana", katakana)).toBe("ア");
    expect(focusFor({ ch: "ウ", script: "katakana" }, "hiragana", hiragana)).toBe("あ");
  });

  it("falls back to the first kana when nothing was asked for", () => {
    expect(focusFor(null, "hiragana", hiragana)).toBe("あ");
  });

  it("answers with nothing only when there is neither a request nor a course", () => {
    // The order matters and is the point: a request that belongs to this script is
    // a kana the caller was promised, so it is honoured even for an empty course —
    // what lesson to highlight is the caller's fallback, not this function's.
    expect(focusFor({ ch: "い", script: "hiragana" }, "hiragana", [])).toBe("い");
    // With no matching request and no course there is no kana to open.
    expect(focusFor({ ch: "い", script: "hiragana" }, "katakana", [])).toBeNull();
    expect(focusFor(null, "hiragana", [])).toBeNull();
  });
});

describe("chartColumns", () => {
  it("is the widest row, which is the grid's five vowels", () => {
    expect(
      chartColumns(
        chart([
          ["あ", "い", "う", "え", "お"],
          ["や", null, "ゆ", null, "よ"],
        ]),
      ),
    ).toBe(5);
  });

  it("is zero for a chart with no rows rather than a hardcoded five", () => {
    // A screen that draws `chartColumns` columns of nothing is honest about
    // having no data; five empty columns reads as a grid that failed to load.
    expect(chartColumns(chart([]))).toBe(0);
  });

  it("takes the widest row when a caller sends a short one", () => {
    expect(chartColumns(chart([["ん"], ["あ", "い", "う", "え", "お"]]))).toBe(5);
    expect(chartColumns(chart([["ん"]]))).toBe(1);
  });
});

describe("isSingleKana", () => {
  it("is true for the classic pairs' answers", () => {
    expect(isSingleKana("シ")).toBe(true);
    expect(isSingleKana("あ")).toBe(true);
    expect(isSingleKana("ー")).toBe(true);
  });

  it("is false for a yōon answer, which the board cannot write as one character", () => {
    expect(isSingleKana("きゃ")).toBe(false);
    expect(isSingleKana("キャ")).toBe(false);
  });

  it("counts characters, not bytes", () => {
    // き is three bytes in UTF-8 and one character; a `length === 1` test would
    // call it a digraph and take the board away from a kana the course teaches.
    expect(isSingleKana("き")).toBe(true);
    expect(isSingleKana("")).toBe(false);
  });
});
