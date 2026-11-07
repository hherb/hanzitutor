import { describe, expect, it } from "vitest";
import { highlightedCourseCharacter } from "./courseHighlight";
import type { HighlightState } from "./courseHighlight";

/** Course recall on 好, loaded, nothing written or graded yet. */
const RECALL: HighlightState = {
  source: "course",
  mode: "recall",
  courseCharacter: "好",
  boardCharacter: "好",
  answerVisible: false,
};
const state = (overrides: Partial<HighlightState>): HighlightState => ({ ...RECALL, ...overrides });

describe("the course character the sidebar picks out", () => {
  it("is withheld in course recall until the answer is shown", () => {
    expect(highlightedCourseCharacter(RECALL)).toBeNull();
  });

  it("is shown once the attempt is graded or the strokes are played", () => {
    expect(highlightedCourseCharacter(state({ answerVisible: true }))).toBe("好");
  });

  it("is withheld while the board still holds the previous, graded character", () => {
    // Enter after a grade moves the course before the next character has
    // loaded: the old grade still says "answer visible", but it is the old
    // character's answer, and the cursor already points at the new one.
    expect(
      highlightedCourseCharacter(
        state({ courseCharacter: "我", boardCharacter: "好", answerVisible: true }),
      ),
    ).toBeNull();
    expect(
      highlightedCourseCharacter(state({ boardCharacter: null, answerVisible: true })),
    ).toBeNull();
  });

  it("is always shown in trace, where the board shows the character anyway", () => {
    expect(highlightedCourseCharacter(state({ mode: "trace" }))).toBe("好");
    expect(highlightedCourseCharacter(state({ mode: "trace", boardCharacter: "我" }))).toBe("好");
  });

  it("is withheld in recall while a queue is on the board", () => {
    // The course position is only where the course will resume, but it is very
    // often in the queue too: the character just practised, answered badly, is
    // due a minute later — so marking it could name the review's answer, or a
    // character of the word still to be written.
    expect(highlightedCourseCharacter(state({ source: "review" }))).toBeNull();
    expect(
      highlightedCourseCharacter(
        state({ source: "review", boardCharacter: "我", answerVisible: true }),
      ),
    ).toBeNull();
    expect(highlightedCourseCharacter(state({ source: "review", mode: "trace" }))).toBe("好");
  });

  it("is nothing when the course has no character", () => {
    expect(highlightedCourseCharacter(state({ courseCharacter: null, mode: "trace" }))).toBeNull();
  });
});
