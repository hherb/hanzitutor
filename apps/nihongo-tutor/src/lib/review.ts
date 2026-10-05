/**
 * The review queue's arithmetic, as pure functions.
 *
 * Everything the Review screen says about *when* is arithmetic over ISO-8601
 * strings: how overdue a character is, when the next one comes back, and what a
 * graded attempt did to the schedule. So it is here, where `vitest` runs it
 * without a window, and the panel only renders what these return — the same
 * arrangement as `board.ts`, `words.ts` and `kanji.ts`.
 *
 * The clock is always passed in rather than read here, which is what makes
 * "2 days overdue" testable at all.
 */

import type { DueItem } from "./types";

/** The milliseconds in a day, for the difference arithmetic below. */
const DAY_MS = 86_400_000;

/** How overdue a due date is, in words a learner reads: "2 days overdue". */
export function overdueLabel(due: string, now: string): string {
  const overdue = Date.parse(now) - Date.parse(due);
  // A due date that has just passed, or one that cannot be parsed, is "due now":
  // the queue only ever offers what is due, so this is the honest floor.
  if (!Number.isFinite(overdue) || overdue <= 0) return "due now";

  const days = Math.floor(overdue / DAY_MS);
  if (days >= 1) return `${days} ${days === 1 ? "day" : "days"} overdue`;

  const hours = Math.floor(overdue / 3_600_000);
  if (hours >= 1) return `${hours} ${hours === 1 ? "hour" : "hours"} overdue`;

  const minutes = Math.max(1, Math.floor(overdue / 60_000));
  return `${minutes} ${minutes === 1 ? "minute" : "minutes"} overdue`;
}

/**
 * When a date falls, for the line that says what happens next: "in 3 days".
 *
 * The counterpart of [`overdueLabel`], and the two are one decision: a character
 * is either behind the learner or ahead of them, and neither sentence should ever
 * be produced by a date on the other side of now.
 *
 * Two details, both learned from looking at the running screen rather than from
 * the arithmetic:
 *
 * * the day count **rounds**, where `overdueLabel`'s floors. A due date exactly
 *   two days out, read a moment late, is 1.9997 days; flooring would say "in 1
 *   day" about the two-day interval the schedule had just set, and the label
 *   would disagree with the interval on the same screen;
 * * days are only used from about a day out. SM-2's "hard" interval is **half a
 *   day**, and that has to read "in 12 hours" — rounding it to the nearest day
 *   would call a twelve-hour interval "in 1 day".
 */
export function upcomingLabel(due: string, now: string): string {
  const ahead = Date.parse(due) - Date.parse(now);
  if (!Number.isFinite(ahead) || ahead <= 0) return "now";

  const days = ahead / DAY_MS;
  if (days >= 0.95) return `in ${Math.round(days)} ${Math.round(days) === 1 ? "day" : "days"}`;

  const hours = ahead / 3_600_000;
  if (hours >= 1) return `in ${Math.round(hours)} ${Math.round(hours) === 1 ? "hour" : "hours"}`;

  const minutes = Math.max(1, Math.round(ahead / 60_000));
  return `in ${minutes} ${minutes === 1 ? "minute" : "minutes"}`;
}

/**
 * What to say about a graded attempt's schedule, or null when there is nothing.
 *
 * The three cases — counted, not counted because it was not due, and an attempt
 * that could not be saved — are a rule about the schedule rather than about the
 * drawing, so they are decided here and the panel only gives them a colour.
 */
export function scheduleNote(
  graded: { scheduled: boolean; nextDue: string | null; warning: string | null },
  now: string,
): { tone: "ok" | "plain" | "bad"; text: string } | null {
  if (graded.warning) return { tone: "bad", text: graded.warning };
  if (!graded.scheduled) {
    return {
      tone: "plain",
      text: "Already scheduled — writing it again before it is due is practice, and the schedule is unchanged.",
    };
  }
  if (!graded.nextDue) return { tone: "ok", text: "Saved for review." };
  return {
    tone: "ok",
    text: `Saved for review — next ${upcomingLabel(graded.nextDue, now)}.`,
  };
}

/**
 * Whether two queue entries are the same card.
 *
 * A card is keyed by the character, and the kind is compared with it because the
 * same character can be offered as a kana, a jōyō kanji or a radical head form —
 * 人 is both a character and radical 9.
 */
function isSameCard(item: DueItem, other: DueItem | null): boolean {
  return item.ch === other?.ch && item.kind === other?.kind;
}

/**
 * The next due character after the one on the board, among those the page holds.
 *
 * **By identity rather than by position**: the queue is ordered by how overdue each
 * character is and that order moves as characters are graded, so "the next one" is
 * the first item that is not the one on the board. A character the page does not
 * hold — the learner tapped an item and then opened another screen — leaves the
 * first item as the answer.
 */
export function nextDue(items: DueItem[], current: DueItem | null): DueItem | null {
  return items.find((item) => !isSameCard(item, current)) ?? null;
}

/**
 * Whether the queue can offer anything at all after the character on the board.
 *
 * Two ways of having something next, and the second is why this is not simply
 * `nextDue(...) !== null`. `due` is the section's uncapped count and `items` is what
 * was fetched, so their difference is the part of the queue the screen has not asked
 * for. The command returns `min(limit, due)` items today, so the second term is a
 * guard rather than the usual path — but the sentence it guards is **"nothing due"**,
 * whose subject is the queue and not the page, and this is the one place that can
 * tell an emptied queue from a page nobody has fetched yet.
 */
export function hasNextDue(
  queue: { items: DueItem[]; due: number },
  current: DueItem | null,
): boolean {
  return nextDue(queue.items, current) !== null || queue.due > queue.items.length;
}
