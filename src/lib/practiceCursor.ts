/**
 * Which saved position a drill reads and writes, and when a synced one is used.
 *
 * Two kinds of place are kept: the course cursor, one index into the course, and
 * one entry per vocabulary group. Both are synced, and both were lost in ways
 * that looked the same to a learner — "it forgot where I was" (issue #32). The
 * decisions are here, out of `App.svelte`, so that each of those ways is a test
 * rather than something to reproduce by hand on two devices.
 */

/** Anything that may belong to a vocabulary group. */
export interface Grouped {
  group: string | null;
}

/**
 * The one group every entry belongs to, or `null` when they do not agree.
 *
 * A view that is one group's list *is* that group, so its place is kept whether
 * or not the caller knew to name the group.
 */
export function sharedGroup(entries: readonly Grouped[]): string | null {
  const first = entries[0]?.group ?? null;
  if (first === null) return null;
  return entries.every((entry) => entry.group === first) ? first : null;
}

/** How a vocabulary drill was asked for. */
export interface DrillOptions {
  /**
   * `false` for a drill that must keep no place at all: a row's own Practise
   * button drills one entry as a one-off, and finishing it must neither move nor
   * erase the place the learner had reached in that entry's group. `null` for the
   * group cannot say this, because `null` already means "work the group out from
   * the entries".
   */
  keepPlace?: boolean;
}

/**
 * The group whose place a vocabulary drill resumes from and writes, or `null`.
 *
 * The named group when there is one, otherwise the group all the entries share —
 * unless the drill is a one-off, which keeps none.
 */
export function drillGroup(
  entries: readonly Grouped[],
  group: string | null,
  options: DrillOptions = {},
): string | null {
  if (options.keepPlace === false) return null;
  return group ?? sharedGroup(entries);
}

/**
 * The group whose place finishing an entry writes, or `null` for none.
 *
 * Only a vocabulary drill keeps a group's place. The group is remembered when the
 * drill starts, and a drill from another screen — words, characters, radicals,
 * tones or a review — can replace it on the board without the group being
 * forgotten; finishing *that* drill must not write the group's place, or it erases
 * it, on every device it syncs to. Checking the source as well as clearing the
 * group when a drill starts means neither alone has to be right.
 */
export function placeKeptBy(source: string, group: string | null): string | null {
  return source === "vocabulary" ? group : null;
}

/** What the board is doing when a sync hands back a course position. */
export interface CourseBoard {
  /** The course position the board is on. */
  current: number;
  /** The position last handed to the backend. Differs from `current` while a
   * move is waiting for its debounced save. */
  lastSaved: number;
  /** Characters in the course; `0` before it has loaded. */
  total: number;
  /** Whether the board is showing the course, rather than another drill. */
  onCourse: boolean;
  /** Whether there are strokes on the board — an attempt under way or graded. */
  writing: boolean;
}

/** What to do with a course position read back after a sync. */
export type CursorDecision =
  | { action: "stay" }
  | { action: "move"; index: number }
  | { action: "defer"; index: number };

/**
 * Whether the board should follow a course position that a sync may have moved.
 *
 * - Nothing to do when it did not move, or before the course has loaded (the
 *   start-up restore reads it then).
 * - A move on this device that has not been saved yet is newer than anything the
 *   sync could have read, so it stays and its save goes ahead.
 * - The board follows at once when it is on the course with nothing written.
 * - Otherwise the position waits: moving the board under a learner who is writing
 *   would throw the attempt away, and the course is not on the board during
 *   another drill. See [`takeDeferred`] for when it is used.
 */
export function courseCursorAfterSync(stored: number, board: CourseBoard): CursorDecision {
  if (board.total <= 0) return { action: "stay" };
  const index = Math.min(Math.max(0, stored), board.total - 1);
  if (index === board.current) return { action: "stay" };
  if (board.current !== board.lastSaved) return { action: "stay" };
  if (!board.onCourse || board.writing) return { action: "defer", index };
  return { action: "move", index };
}

/** A synced course position that is waiting for the board, and where the board
 * was when it arrived. */
export interface DeferredCursor {
  index: number;
  at: number;
}

/**
 * The deferred position to move to now, or `null` to carry on as asked.
 *
 * Called at the learner's next step in the course — an arrow, Next, or coming
 * back to the course from another drill. It is used only while the board is
 * still where it was when the position arrived: a lesson or character picked
 * since is a newer request than the sync, and wins.
 */
export function takeDeferred(deferred: DeferredCursor | null, current: number): number | null {
  if (deferred === null || deferred.at !== current) return null;
  return deferred.index;
}
