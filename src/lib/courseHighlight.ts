/**
 * Which course character the sidebar's lesson grid may pick out.
 *
 * The grid marks the course position (`.char.current`) and scrolls to it. In
 * recall that is the answer: the header hides the glyph behind a `?`, and the
 * sidebar beside it used to name the character anyway (issue #31). So the mark
 * follows the same rule as the header — `answerVisible` in `App.svelte` — and
 * is withheld until the answer is shown.
 *
 * A pure function so the rule can be tested; `App.svelte` passes its state in.
 */

export interface HighlightState {
  /** What the board is working through — `"course"`, or a queue such as a review. */
  source: string;
  mode: "trace" | "recall";
  /** The character at the course position, or `null` before the course loads. */
  courseCharacter: string | null;
  /** The character actually loaded on the board, which lags a move by a load. */
  boardCharacter: string | null;
  /** Whether the board is showing its answer (`App.svelte`'s `answerVisible`). */
  answerVisible: boolean;
}

/**
 * The character to mark, or `null` to mark none.
 *
 * - **Trace** shows the character on the board, so there is nothing to hide.
 * - **Course recall** marks it once the answer is visible *for that character*.
 *   `answerVisible` belongs to whatever is loaded, and after a graded attempt
 *   the cursor moves before the next character has loaded: for that moment the
 *   old grade would mark the new character, and the grid would scroll to it.
 * - **Recall with a queue on the board** marks nothing. The course position is
 *   then only where the course resumes, but it is often in the queue as well —
 *   the character last practised, answered badly, is due a minute later — so the
 *   mark could name the review's answer or a character of a word still to write.
 *   With the mark gone the grid still scrolls to the open lesson.
 */
export function highlightedCourseCharacter(state: HighlightState): string | null {
  const { courseCharacter } = state;
  if (courseCharacter === null) return null;
  if (state.mode === "trace") return courseCharacter;
  if (state.source !== "course") return null;
  return state.boardCharacter === courseCharacter && state.answerVisible ? courseCharacter : null;
}
