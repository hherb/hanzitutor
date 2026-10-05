import { describe, expect, it } from "vitest";

import {
  GRAMMAR_PAGE,
  GRAMMAR_SEARCH,
  pageOf,
  positionLabel,
  positionsOnPage,
  rowLabel,
  searchNote,
  searchNoteFor,
  stepTo,
} from "./grammar";

describe("positionLabel", () => {
  it("states where a pattern is in a course that has no ladder", () => {
    expect(positionLabel(1, 526)).toBe("1 of 526");
    expect(positionLabel(526, 526)).toBe("526 of 526");
  });
});

describe("searchNote", () => {
  it("says nothing while the box is empty, because the course is what is on screen", () => {
    expect(searchNote(0, 0, "")).toBe("");
    expect(searchNote(0, 0, "   ")).toBe("");
  });

  it("names the query when every match is drawn", () => {
    expect(searchNote(3, 3, "particle")).toBe("3 patterns match “particle”.");
    expect(searchNote(1, 1, "wa")).toBe("1 pattern match “wa”.");
  });

  it("says when the list is not all of the matches", () => {
    // The half that matters: sixty of a hundred and eleven must not read as a
    // course that small.
    expect(searchNote(111, 60, "は")).toBe("Showing the first 60 of 111 matches for “は”.");
    expect(searchNote(2, 1, "wa")).toBe("Showing the first 1 of 2 matches for “wa”.");
  });

  it("is a sentence rather than an empty list when nothing matches", () => {
    expect(searchNote(0, 0, "zzzz")).toBe("No patterns match “zzzz”.");
  });

  it("trims the query it quotes", () => {
    expect(searchNote(1, 1, "  wa  ")).toBe("1 pattern match “wa”.");
  });
});

describe("searchNoteFor", () => {
  it("says nothing until an answer for this query has arrived", () => {
    // The first keystroke of a search has no answer yet, and a note built from the
    // previous one would say "No patterns match “p”." over a search still running.
    expect(searchNoteFor(null, "p")).toBe("");
    expect(searchNoteFor(null, "")).toBe("");
  });

  it("is the answer's note once there is one", () => {
    expect(searchNoteFor({ total: 3, points: [1, 2, 3] }, "particle")).toBe(
      "3 patterns match “particle”.",
    );
    expect(searchNoteFor({ total: 111, points: new Array(60).fill(1) }, "は")).toBe(
      "Showing the first 60 of 111 matches for “は”.",
    );
    expect(searchNoteFor({ total: 0, points: [] }, "zzzz")).toBe(
      "No patterns match “zzzz”.",
    );
  });
});

describe("stepTo", () => {
  it("moves one position along the course", () => {
    expect(stepTo(1, 1, 526)).toEqual({ position: 2, page: 1 });
    expect(stepTo(20, -1, 526)).toEqual({ position: 19, page: 1 });
  });

  it("says which page the neighbour is on, so a boundary is a page change", () => {
    // The last row of page one: the next position is the first row of page two.
    expect(stepTo(20, 1, 526)).toEqual({ position: 21, page: 2 });
    expect(stepTo(21, -1, 526)).toEqual({ position: 20, page: 1 });
    expect(stepTo(520, 1, 526)).toEqual({ position: 521, page: 27 });
  });

  it("stops at the ends of the course rather than wrapping", () => {
    expect(stepTo(1, -1, 526)).toBeNull();
    expect(stepTo(526, 1, 526)).toBeNull();
    expect(stepTo(527, 1, 526)).toBeNull();
    expect(stepTo(0, 1, 526)).toBeNull();
  });

  it("steps from anywhere in the course, including a search result", () => {
    // A card opened from a search knows its position, so the arrows still walk the
    // course rather than the list that happened to be on screen.
    expect(stepTo(300, 1, 526)).toEqual({ position: 301, page: 16 });
  });
});

describe("pageOf", () => {
  it("is one-based and follows the page size", () => {
    expect(pageOf(1)).toBe(1);
    expect(pageOf(20)).toBe(1);
    expect(pageOf(21)).toBe(2);
    expect(pageOf(526)).toBe(27);
    expect(pageOf(0)).toBe(1);
  });
});

describe("positionsOnPage", () => {
  it("numbers a page from its offset, one-based", () => {
    expect(positionsOnPage(0, 3)).toEqual([1, 2, 3]);
    expect(positionsOnPage(520, 2)).toEqual([521, 522]);
    expect(positionsOnPage(0, 0)).toEqual([]);
  });
});

describe("the two constants", () => {
  it("are the page sizes the screen and the command agree on", () => {
    expect(GRAMMAR_PAGE).toBe(20);
    expect(GRAMMAR_SEARCH).toBe(60);
    expect(GRAMMAR_SEARCH).toBeGreaterThan(GRAMMAR_PAGE);
  });
});

describe("rowLabel", () => {
  it("is the pattern and its meaning on one line", () => {
    expect(
      rowLabel({
        position: 1,
        id: "wa",
        pattern: "〜は",
        reading: null,
        romaji: "wa",
        meaning: "as for; marks the topic",
        formation: "Noun + は",
        notes: null,
        tags: ["particle"],
        examples: [],
      }),
    ).toBe("〜は — as for; marks the topic");
  });
});
