import { describe, expect, it } from "vitest";
import {
  boardInUse,
  courseCursorAfterSync,
  drillGroup,
  placeKeptBy,
  sharedGroup,
  syncNoticeLine,
  takeDeferred,
  type CourseBoard,
} from "./practiceCursor";

/** An entry of the list, as far as its group goes. */
const entry = (id: number, group: string | null) => ({ id, group });
const hsk2 = (id: number) => entry(id, "HSK2");

describe("the group every entry of a drill shares", () => {
  it("is that group when they all agree", () => {
    expect(sharedGroup([hsk2(1), hsk2(2)])).toBe("HSK2");
  });

  it("is none when they disagree, or are unfiled", () => {
    expect(sharedGroup([hsk2(1), entry(2, "HSK3")])).toBeNull();
    expect(sharedGroup([entry(1, null), hsk2(2)])).toBeNull();
    expect(sharedGroup([])).toBeNull();
  });
});

describe("the group whose place a vocabulary drill keeps (issue #32 §2)", () => {
  it("is the group the caller named", () => {
    expect(drillGroup([hsk2(1)], "HSK2")).toBe("HSK2");
  });

  it("follows the entries when the caller named none but they agree", () => {
    expect(drillGroup([hsk2(1), hsk2(2)], null)).toBe("HSK2");
  });

  it("is none for a one-off drill of a single row, whatever its group", () => {
    // The row's own Practise button: finishing it must not move, or erase, the
    // place the learner had reached in HSK2.
    expect(drillGroup([hsk2(7)], null, { keepPlace: false })).toBeNull();
    expect(drillGroup([hsk2(7)], "HSK2", { keepPlace: false })).toBeNull();
  });
});

describe("the place a finished entry writes (issue #32 §1)", () => {
  it("is the drill's group while a vocabulary drill is on the board", () => {
    expect(placeKeptBy("vocabulary", "HSK2")).toBe("HSK2");
  });

  it("is none for a drill from any other screen, even after a group drill", () => {
    // A group left over from the drill that was interrupted must not be written
    // with null when a word, character, radical, tone or review drill finishes.
    for (const source of ["words", "characters", "radicals", "tones", "review", "course"]) {
      expect(placeKeptBy(source, "HSK2"), source).toBeNull();
    }
  });

  it("is none for a vocabulary drill that is not one group", () => {
    expect(placeKeptBy("vocabulary", null)).toBeNull();
  });
});

describe("the course position after a sync (issue #32 §3)", () => {
  /** On the course at 37 (0-based 36), saved, nothing waiting to be saved, board
   * empty. */
  const idle: CourseBoard = {
    current: 36,
    lastSaved: 36,
    saving: false,
    total: 100,
    onCourse: true,
    busy: false,
  };

  /** The decision for a board that did not change while the cursor was read. */
  const decide = (stored: number, board: CourseBoard) =>
    courseCursorAfterSync(stored, board, board);

  it("moves an idle course board to where the other device left off", () => {
    expect(decide(51, idle)).toEqual({ action: "move", index: 51 });
  });

  it("does nothing when the position did not move", () => {
    expect(decide(36, idle)).toEqual({ action: "stay" });
  });

  it("does nothing when the stored position clamps onto the board's", () => {
    expect(decide(500, { ...idle, current: 99, lastSaved: 99 })).toEqual({ action: "stay" });
  });

  it("keeps a local move that has not been saved yet, which is newer", () => {
    // The learner pressed → and the debounced save has not run: what the sync
    // read is older than what is on the board.
    expect(decide(51, { ...idle, current: 37, saving: true })).toEqual({ action: "stay" });
  });

  it("keeps a local move whose save went out while the position was being read", () => {
    // 37 → 38 with the save pending when the read began; the save fired during
    // the round trip, so by the answer the board looks saved and idle — but the
    // read may be from before that write, and moving would lose the newer 38.
    const before = { ...idle, current: 37, lastSaved: 36, saving: true };
    const now = { ...idle, current: 37, lastSaved: 37 };
    expect(courseCursorAfterSync(51, now, before)).toEqual({ action: "stay" });
  });

  it("keeps a local position whose save is still on its way to the backend", () => {
    expect(decide(51, { ...idle, saving: true })).toEqual({ action: "stay" });
  });

  it("keeps a board the learner moved while the position was being read", () => {
    expect(courseCursorAfterSync(51, { ...idle, current: 40, lastSaved: 40 }, idle)).toEqual({
      action: "stay",
    });
  });

  it("waits while the board is in use", () => {
    expect(decide(51, { ...idle, busy: true })).toEqual({ action: "defer", index: 51 });
  });

  it("waits while another drill is on the board", () => {
    expect(decide(51, { ...idle, onCourse: false })).toEqual({ action: "defer", index: 51 });
  });

  it("clamps a stored position past the end of the course", () => {
    expect(decide(500, idle)).toEqual({ action: "move", index: 99 });
    expect(decide(-3, { ...idle, current: 5, lastSaved: 5 })).toEqual({
      action: "move",
      index: 0,
    });
  });

  it("stays put before the course is loaded", () => {
    expect(decide(51, { ...idle, total: 0 })).toEqual({ action: "stay" });
  });
});

describe("whether the board is in use", () => {
  const empty = { strokes: 0, playing: false, revealed: 0, sweeping: false };

  it("is not when nothing is on it", () => {
    expect(boardInUse(empty)).toBe(false);
  });

  it("is while there are strokes, the stroke order plays, or a hint shows", () => {
    expect(boardInUse({ ...empty, strokes: 1 })).toBe(true);
    expect(boardInUse({ ...empty, playing: true })).toBe(true);
    expect(boardInUse({ ...empty, revealed: 2 })).toBe(true);
    expect(boardInUse({ ...empty, sweeping: true })).toBe(true);
  });
});

describe("taking a deferred course position", () => {
  it("is taken at the learner's next step on from where it was deferred", () => {
    expect(takeDeferred({ index: 51, at: 36 }, 36)).toBe(51);
    expect(takeDeferred({ index: 51, at: 36 }, 36, 1)).toBe(51);
  });

  it("is not taken by a step back", () => {
    // ← is "show me the one before", not "I am done here".
    expect(takeDeferred({ index: 51, at: 36 }, 36, -1)).toBeNull();
  });

  it("is dropped once the learner has picked somewhere else", () => {
    // A lesson or character chosen since is a newer request than the sync.
    expect(takeDeferred({ index: 51, at: 36 }, 12)).toBeNull();
  });

  it("is nothing when nothing was deferred", () => {
    expect(takeDeferred(null, 36)).toBeNull();
  });
});

describe("the sync notice, when the board did not follow", () => {
  const said = "Synced: received 3 attempts, moved to where you left off.";

  it("is the backend's line when the board moved, stayed, or there was no decision", () => {
    expect(syncNoticeLine(said, { action: "move", index: 51 })).toBe(said);
    expect(syncNoticeLine(said, { action: "stay" })).toBe(said);
    expect(syncNoticeLine(said, null)).toBe(said);
  });

  it("says the move is still to come when it was deferred", () => {
    expect(syncNoticeLine(said, { action: "defer", index: 51 })).toBe(
      "Synced: received 3 attempts, will move to where you left off at your next step.",
    );
  });

  it("leaves a line that says nothing about the position alone", () => {
    const other = "Synced: received 3 attempts.";
    expect(syncNoticeLine(other, { action: "defer", index: 51 })).toBe(other);
  });
});
