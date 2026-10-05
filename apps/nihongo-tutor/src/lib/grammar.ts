/**
 * The grammar course's arithmetic, as pure functions.
 *
 * The same split the other screens use: what can be reasoned about without a window
 * lives here and is unit-tested by the root project's vitest, and the components in
 * `*.svelte` only draw.
 *
 * Two of these are rules rather than arithmetic. `positionLabel` is how a course
 * with no ladder says where a learner is — the pattern has no band, so "12 of 526"
 * is the whole statement of place. `stepFrom` is what the arrows either side of a
 * pattern do, including the step that falls off the end of a page and has to fetch
 * the next one.
 */

import type { GrammarPoint } from "./types";

/**
 * How many patterns a page holds.
 *
 * Smaller than the vocabulary's 24 because a grammar point is the largest row the
 * app sends: a pattern, its notes and two or three whole sentences with furigana
 * over every kanji.
 */
export const GRAMMAR_PAGE = 20;

/** How many matches a search draws at once. Mirrors what `api.grammarSearch` asks for. */
export const GRAMMAR_SEARCH = 60;

/** Where a pattern sits in the course, counted from 1, as its card states it. */
export function positionLabel(position: number, total: number): string {
  return `${position} of ${total}`;
}

/**
 * What a search box's answer says about itself.
 *
 * Empty while the box is empty, because the course's own pages are what is on
 * screen then. Otherwise it names the query, and — the half that matters — says
 * when the list is **not** all of the matches: a search that drew sixty of a
 * hundred and eleven and said nothing would read as a course that small.
 */
export function searchNote(total: number, shown: number, query: string): string {
  const asked = query.trim();
  if (asked === "") return "";
  const noun = total === 1 ? "pattern" : "patterns";
  if (total === 0) return `No ${noun} match “${asked}”.`;
  if (shown >= total) return `${total} ${noun} match “${asked}”.`;
  return `Showing the first ${shown} of ${total} matches for “${asked}”.`;
}

/**
 * Where a card's arrows go: the next position in the course, and the page it is on.
 *
 * The course order is the sequence the arrows step through — a pattern has no band
 * and no id order to follow — so a step is a change of **position** rather than of
 * index, and it is asked for from a search result as readily as from a page: the
 * position says which page holds the neighbour, and the screen fetches it if it is
 * not the one already loaded. `null` is an arrow that cannot be followed, which is
 * the end of the course rather than the end of a page.
 */
export function stepTo(
  position: number,
  direction: -1 | 1,
  total: number,
  pageSize: number = GRAMMAR_PAGE,
): { position: number; page: number } | null {
  if (position < 1 || position > total) return null;
  const target = position + direction;
  if (target < 1 || target > total) return null;
  return { position: target, page: pageOf(target, pageSize) };
}

/** The page a position sits on, one-based. */
export function pageOf(position: number, pageSize: number = GRAMMAR_PAGE): number {
  const size = Math.max(Math.trunc(pageSize) || 1, 1);
  return Math.floor((Math.max(position, 1) - 1) / size) + 1;
}

/** The patterns' positions, for a page that starts at `offset`. */
export function positionsOnPage(offset: number, count: number): number[] {
  return Array.from({ length: count }, (_, i) => offset + i + 1);
}

/** A point's row text, for a title attribute or a test that wants one line. */
export function rowLabel(point: GrammarPoint): string {
  return `${point.pattern} — ${point.meaning}`;
}
