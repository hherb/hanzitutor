import { describe, expect, it } from "vitest";

import {
  chartColumns,
  isSingleKana,
  joinedLabel,
  lessonLabel,
  lessonOf,
  neighbourIn,
  positionIn,
} from "./kana";
import type { ChartView, LessonView } from "./types";

/**
 * The kana screens' arithmetic.
 *
 * Four rules make "a kana offered anywhere can be opened on the board" true
 * rather than nearly true: which lesson a kana belongs to, how the arrows step
 * through it, how wide the chart's grid is, and whether a drill answer is a kana
 * the board can draw at all. The fifth is how a verdict says which taught strokes
 * a hand drew joined — the line that makes the per-stroke list's numbers honest.
 */

const lesson = (key: string, kana: string[], title?: string): LessonView => ({
  key,
  title: title ?? `${kana.join(" ")} — ${key}`,
  kana,
  voiced: false,
  count: kana.length,
});

const course: LessonView[] = [
  lesson("hiragana-a", ["あ", "い", "う", "え", "お"], "あ い う え お — a i u e o"),
  lesson("hiragana-ya", ["や", "ゆ", "よ"], "や ゆ よ — ya yu yo"),
  lesson("hiragana-small", ["ぁ", "ぃ", "ぅ", "ぇ", "ぉ"], "Small kana — ぁ ぃ ぅ ぇ ぉ"),
];

describe("lessonOf", () => {
  it("finds the lesson a kana is taught in, not the first lesson", () => {
    expect(lessonOf(course, "ゆ")?.key).toBe("hiragana-ya");
    expect(lessonOf(course, "ぉ")?.key).toBe("hiragana-small");
  });

  it("answers with nothing when the course does not teach it", () => {
    expect(lessonOf(course, "カ")).toBeNull();
    expect(lessonOf([], "あ")).toBeNull();
  });

  it("does not mistake a kana for one it merely contains", () => {
    // き is not in きゃ: a lesson's members are its kana, not its substrings.
    const withDigraph = [lesson("hiragana-small", ["きゃ"])];
    expect(lessonOf(withDigraph, "き")).toBeNull();
    expect(lessonOf(withDigraph, "きゃ")?.key).toBe("hiragana-small");
  });
});

describe("lessonLabel", () => {
  it("is the row's sound for a lesson of the gojūon grid", () => {
    expect(lessonLabel(course[0]!)).toBe("a i u e o");
    expect(lessonLabel(course[1]!)).toBe("ya yu yo");
  });

  it("is the group's own name for a lesson off the grid", () => {
    // The title's halves swap places here — "Small kana — ぁ ぃ ぅ ぇ ぉ" names the
    // group first — and reading the first half blindly would call the kana the
    // lesson's sound and the group its name.
    expect(lessonLabel(course[2]!)).toBe("Small kana");
  });

  it("does not read the label off a lesson whose kana it cannot match", () => {
    // A title shape this does not know still answers with the half a learner used
    // to see, rather than with an empty label or with the whole title.
    const odd = lesson("hiragana-x", ["あ"], "Something else entirely");
    expect(lessonLabel(odd)).toBe("Something else entirely");
  });
});

describe("neighbourIn", () => {
  it("steps to the kana either side inside the lesson", () => {
    expect(neighbourIn(course[0]!, "い", 1)).toBe("う");
    expect(neighbourIn(course[0]!, "い", -1)).toBe("あ");
  });

  it("stops at both ends rather than wrapping", () => {
    // A lesson is a sequence a learner works through; an arrow that jumped from
    // the last kana back to the first would hide that the lesson is finished.
    expect(neighbourIn(course[0]!, "あ", -1)).toBeNull();
    expect(neighbourIn(course[0]!, "お", 1)).toBeNull();
  });

  it("answers nothing for a kana the lesson does not hold, and for no lesson", () => {
    // The case the stage meets when it opens a kana from the confusions list:
    // シ belongs to another lesson, so this one has no arrow to offer.
    expect(neighbourIn(course[0]!, "ん", 1)).toBeNull();
    expect(neighbourIn(null, "あ", 1)).toBeNull();
  });

  it("does not move when asked for no step", () => {
    expect(neighbourIn(course[0]!, "あ", 0)).toBeNull();
  });
});

describe("positionIn", () => {
  it("counts from one, which is what the stage prints", () => {
    expect(positionIn(course[0]!, "あ")).toBe(1);
    expect(positionIn(course[0]!, "お")).toBe(5);
  });

  it("is zero for a kana the lesson does not hold, and for no lesson", () => {
    expect(positionIn(course[0]!, "ん")).toBe(0);
    expect(positionIn(null, "あ")).toBe(0);
  });
});

const chart = (rows: (string | null)[][]): ChartView => ({
  script: "hiragana",
  rows: rows.map((cells) => ({ sound: "ka", voiced: false, cells })),
  offGrid: [],
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

describe("joinedLabel", () => {
  it("is null when the attempt was written the way it is taught", () => {
    // Null rather than an empty string: a caller leaves the line out rather than
    // drawing an empty one under the verdict.
    expect(joinedLabel([])).toBeNull();
  });

  it("names the taught strokes a hand drew as one", () => {
    // さ in two strokes: taught strokes 1 and 2 written together.
    expect(joinedLabel([[1, 2]])).toBe("1+2");
    // き in three: the last two together.
    expect(joinedLabel([[3, 4]])).toBe("3+4");
  });

  it("names every join when a hand joined in more than one place", () => {
    expect(joinedLabel([[1, 2], [4, 5]])).toBe("1+2, 4+5");
  });
});
