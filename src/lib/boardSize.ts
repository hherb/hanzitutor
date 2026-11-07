/**
 * How much of the room available the board takes, from the board-size
 * preference.
 *
 * A share of the container rather than a pixel side: the board fits whatever
 * window it is in, and the preference says how much of that fit to use, so a
 * large board stays large on a big screen and a compact one stays usable on a
 * small one.
 *
 * **This is the only place a board size becomes a number.** `BoardSize` in
 * `hanzi-core` carries the name, not the fraction, precisely so that the value
 * cannot exist in two places and drift.
 *
 * Three different numbers, because three names that draw two sizes are a setting
 * that lies (issue #33: Large used to be Normal). Large is the whole fit — the
 * most the window has room for, so nothing can be larger — and Normal leaves a
 * margin round it, so choosing Large is a visible step up.
 */
import type { BoardSize } from "./types";

export const BOARD_FRACTIONS: Readonly<Record<BoardSize, number>> = {
  compact: 0.72,
  normal: 0.86,
  large: 1,
};

/**
 * The share of the fit for a board size.
 *
 * The value comes from a settings file, so a name this build does not know — one
 * written by a newer version, or by hand — is drawn at the default size rather
 * than at `NaN`.
 */
export function boardFractionOf(size: BoardSize | null | undefined): number {
  if (size != null && Object.hasOwn(BOARD_FRACTIONS, size)) return BOARD_FRACTIONS[size];
  return BOARD_FRACTIONS.normal;
}
