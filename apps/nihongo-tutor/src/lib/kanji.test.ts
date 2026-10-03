/**
 * The kanji course's and the radicals panel's arithmetic, pinned.
 *
 * These five functions decide which tab a lesson is under, how big a grade is,
 * which lesson holds a character another screen asked for, and how 214 radicals
 * are ordered and found. All of it is invisible in a screenshot when it is
 * wrong — a character one lesson away from where it should be looks exactly like
 * a course — so each is asserted here rather than by watching a canvas.
 */

import { describe, expect, it } from "vitest";
import {
  findFamilies,
  gradeTabs,
  lessonHolding,
  lessonsIn,
  sortFamilies,
} from "./kanji";
import type { KanjiLessonView, RadicalFamilyView } from "./types";

function lesson(
  key: string,
  grade: number,
  kanji: string,
  title = "1–10",
): KanjiLessonView {
  return {
    key,
    title,
    grade,
    gradeName: grade === 8 ? "jōyō beyond the school grades" : `kyōiku ${grade}`,
    count: [...kanji].length,
    kanji: [...kanji],
  };
}

/** Three lessons over two grades: 1 (two lessons) and the remainder (one). */
const course: KanjiLessonView[] = [
  lesson("g1-1", 1, "日一大", "1–3"),
  lesson("g1-2", 1, "中出三", "4–6"),
  lesson("g8-1", 8, "亜鬱", "1–2"),
];

function family(number: number, ch: string, characters: string): RadicalFamilyView {
  return { number, ch, strokeCount: 1, characters: [...characters] };
}

const families: RadicalFamilyView[] = [
  family(64, "手", "手持指"),
  family(1, "一", "一三"),
  family(90, "爿", ""),
];

describe("the grade tabs", () => {
  it("groups the lessons by grade, in the order the course lists them", () => {
    expect(gradeTabs(course)).toEqual([
      { grade: 1, name: "kyōiku 1", kanji: 6, lessons: 2 },
      { grade: 8, name: "jōyō beyond the school grades", kanji: 2, lessons: 1 },
    ]);
  });

  it("is empty for an empty course rather than throwing", () => {
    expect(gradeTabs([])).toEqual([]);
  });
});

describe("finding a lesson", () => {
  it("filters a grade's lessons", () => {
    expect(lessonsIn(course, 1).map((l) => l.key)).toEqual(["g1-1", "g1-2"]);
    expect(lessonsIn(course, 8).map((l) => l.key)).toEqual(["g8-1"]);
    expect(lessonsIn(course, 2)).toEqual([]);
  });

  it("finds the lesson a character belongs to", () => {
    expect(lessonHolding(course, "中")?.key).toBe("g1-2");
    expect(lessonHolding(course, "鬱")?.key).toBe("g8-1");
  });

  it("returns null for a character the course does not teach", () => {
    // 鳩 is jinmeiyō: a component can name it, and there is no lesson for it.
    expect(lessonHolding(course, "鳩")).toBeNull();
  });
});

describe("ordering the radicals", () => {
  it("numbers them in Kangxi order", () => {
    expect(sortFamilies(families, "number").map((f) => f.number)).toEqual([1, 64, 90]);
  });

  it("puts the biggest families first, and breaks ties by number", () => {
    expect(sortFamilies(families, "size").map((f) => f.number)).toEqual([64, 1, 90]);
  });

  it("does not reorder the caller's array", () => {
    const before = families.map((f) => f.number);
    sortFamilies(families, "size");
    expect(families.map((f) => f.number)).toEqual(before);
  });
});

describe("searching the radicals", () => {
  it("finds a family by its head form", () => {
    expect(findFamilies(families, "手").map((f) => f.number)).toEqual([64]);
  });

  it("finds a family by its classical number", () => {
    expect(findFamilies(families, "90").map((f) => f.ch)).toEqual(["爿"]);
  });

  it("finds a family by a character that uses it", () => {
    // 持 writes 扌 and belongs to 手's family, which is the question a learner
    // arrives with: what is this character made of?
    expect(findFamilies(families, "持").map((f) => f.number)).toEqual([64]);
  });

  it("keeps everything for an empty query and nothing for a miss", () => {
    expect(findFamilies(families, "  ")).toHaveLength(3);
    expect(findFamilies(families, "鳩")).toEqual([]);
  });
});
