import { describe, expect, it } from "vitest";

import {
  COURSES,
  STAGE_VIEWS,
  covers,
  courseOf,
  defaultView,
  hasStage,
  isSection,
  tabsOf,
  type Course,
  type Section,
  type View,
} from "./nav";

/**
 * The app's own division, asserted.
 *
 * The failure this file exists for is structural rather than arithmetic: a screen
 * added to one course's template and to neither tab list, or a screen claimed by
 * both courses, or a default that opens on a screen its own course does not have.
 * None of those is a type error and all of them are visible here.
 */

/** Every screen the app can show, written out rather than derived so a new one must be added here too. */
const ALL_VIEWS: View[] = [
  "practice",
  "chart",
  "drill",
  "review",
  "kanji",
  "radicals",
  "words",
  "read",
  "licences",
  "start",
];

/**
 * The screens that hang off the footer instead of belonging to a course.
 *
 * Both are honest exceptions rather than gaps: Licences is an obligation the app
 * owes, and Start here is about the language rather than about a set of
 * characters. Written out so a third one has to be argued for here.
 */
const FOOTER_VIEWS: View[] = ["licences", "start"];

/** The courses claiming a screen, read from the structure itself. */
const claimants = (view: View): Section[] =>
  COURSES.filter((course) => course.tabs.some((tab) => tab.id === view)).map(
    (course) => course.id,
  );

describe("the courses", () => {
  it("are the two the app is divided into, kana first", () => {
    expect(COURSES.map((course) => course.id)).toEqual(["kana", "kanji"]);
    expect(COURSES.map((course) => course.label)).toEqual(["Kana", "Kanji"]);
  });

  it("each say what they are, in a sentence", () => {
    for (const course of COURSES) {
      expect(course.blurb.length).toBeGreaterThan(20);
      expect(course.blurb).toMatch(/[.:]$/);
    }
  });

  it("carry a tagline short enough for the switch's second line", () => {
    // The switch has exactly two lines per course (N10): the name, then the
    // tagline and the counts. A tagline that grew into a sentence would push the
    // counts onto a third line, so the budget is asserted rather than trusted.
    for (const course of COURSES) {
      expect(course.tagline.length).toBeLessThan(20);
      expect(course.tagline).not.toMatch(/[.:]$/);
    }
    expect(COURSES.map((course) => course.tagline)).toEqual([
      "The on-ramp",
      "The main course",
    ]);
  });

  it("open on their own first screen, and on a screen they have", () => {
    for (const course of COURSES) {
      expect(defaultView(course.id)).toBe(course.tabs[0]?.id);
      expect(covers(course.id, defaultView(course.id))).toBe(true);
    }
    // Spelled out, because "the first tab" would still pass if both courses were
    // reordered by mistake.
    expect(defaultView("kana")).toBe("practice");
    expect(defaultView("kanji")).toBe("kanji");
  });

  it("name their tabs without repeating a label inside one course", () => {
    for (const course of COURSES) {
      const labels = tabsOf(course.id).map((tab) => tab.label);
      expect(new Set(labels).size, `${course.id}: ${labels.join(", ")}`).toBe(
        labels.length,
      );
    }
  });

  it("share Review, and only Review", () => {
    const shared = ALL_VIEWS.filter((view) => claimants(view).length > 1);
    expect(shared).toEqual(["review"]);
  });

  it("give every taught screen a home, and leave the footer screens without one", () => {
    const taught = ALL_VIEWS.filter((view) => !FOOTER_VIEWS.includes(view));
    for (const view of taught) {
      const owners = claimants(view);
      // Review is the one screen two courses may claim; everything else is one.
      const expected = view === "review" ? 2 : 1;
      expect(owners.length, `${view} is in ${owners.join(" and ")}`).toBe(expected);
    }
    // Licences and Start here are in neither course, and are reachable from both.
    for (const view of FOOTER_VIEWS) {
      expect(claimants(view), `${view} is in a course`).toEqual([]);
    }
    expect(FOOTER_VIEWS).toEqual(["licences", "start"]);
  });
});

describe("the screens that are two screens", () => {
  it("are the five whose panel opens a stage of its own", () => {
    // Written out rather than derived: a stage is a screen the app takes its own
    // chrome off for, and adding or removing one is a decision rather than a
    // consequence of a template edit.
    expect(STAGE_VIEWS).toEqual(["practice", "kanji", "radicals", "words", "read"]);
  });

  it("are every screen with a stage, and no other screen has one", () => {
    expect(ALL_VIEWS.filter(hasStage)).toEqual([...STAGE_VIEWS]);
    for (const view of ALL_VIEWS) {
      if (!STAGE_VIEWS.includes(view)) expect(hasStage(view), view).toBe(false);
    }
    // The two footer screens are the ones it would be easiest to get wrong: Start
    // here hands the learner to a course but is not itself one of these.
    expect(hasStage("licences")).toBe(false);
    expect(hasStage("start")).toBe(false);
  });

  it("each belong to exactly one course, so a stage never hides another course's switch", () => {
    for (const view of STAGE_VIEWS) {
      expect(claimants(view), `${view}: ${claimants(view).join(", ")}`).toHaveLength(1);
    }
  });

  it("leave the screens that are one thing alone", () => {
    // The chart, the drill and the review queue each draw one screen with nothing
    // to open inside them, and Review is the screen two courses share.
    for (const view of ["chart", "drill", "review"] as View[]) {
      expect(hasStage(view), view).toBe(false);
    }
  });
});

describe("courseOf", () => {
  it("resolves each course's own structure", () => {
    expect(courseOf("kana").label).toBe("Kana");
    expect(courseOf("kanji").label).toBe("Kanji");
    expect(tabsOf("kanji").map((tab) => tab.id)).toEqual([
      "kanji",
      "radicals",
      "words",
      "read",
      "review",
    ]);
  });

  it("hands back the same object the table holds, not a copy that could drift", () => {
    const expected: Course | undefined = COURSES.find((course) => course.id === "kana");
    expect(courseOf("kana")).toBe(expected);
  });
});

describe("covers", () => {
  it("is the membership test the deep-link guard uses", () => {
    expect(covers("kana", "practice")).toBe(true);
    expect(covers("kana", "kanji")).toBe(false);
    expect(covers("kanji", "radicals")).toBe(true);
    expect(covers("kanji", "chart")).toBe(false);
    // Both courses have Review, which is why the section is passed around beside
    // the view rather than derived from it.
    expect(covers("kana", "review")).toBe(true);
    expect(covers("kanji", "review")).toBe(true);
    // And neither has Licences, nor Start here.
    expect(covers("kana", "licences")).toBe(false);
    expect(covers("kanji", "licences")).toBe(false);
    expect(covers("kana", "start")).toBe(false);
    expect(covers("kanji", "start")).toBe(false);
  });
});

describe("isSection", () => {
  it("accepts the two names and nothing else a file could hold", () => {
    expect(isSection("kana")).toBe(true);
    expect(isSection("kanji")).toBe(true);
    expect(isSection("Kana")).toBe(false);
    expect(isSection("licences")).toBe(false);
    expect(isSection("start")).toBe(false);
    expect(isSection(null)).toBe(false);
    expect(isSection(undefined)).toBe(false);
    expect(isSection(7)).toBe(false);
  });
});
