/**
 * How a due date is said out loud.
 *
 * The backend compares due timestamps as text; the interface only has to make
 * one readable. It lives here rather than in `App.svelte` because two screens
 * say it — the board's detail card, and the character lookup's — and two copies
 * of "in 5 days" would drift.
 *
 * "Today" and "tomorrow" are dates on the learner's own calendar, not 24-hour
 * windows from the clock (issue #33): at 22:00 a card due at 08:00 is due
 * *tomorrow*, though it is only ten hours away, and a day and a half from the
 * morning is tomorrow evening rather than "in 2 days". Counting dates rather than
 * hours is also what keeps the answer right across a daylight-saving change, when
 * a day is 23 or 25 hours long.
 *
 * `now` is a parameter so that the tests can say what the clock reads; the app
 * leaves it to default.
 */
export function dueLabel(due: string, now: Date = new Date()): string {
  const at = Date.parse(due);
  if (Number.isNaN(at)) return due;
  if (at <= now.getTime()) return "now";
  const days = calendarDay(new Date(at)) - calendarDay(now);
  if (days <= 0) return "today";
  if (days === 1) return "tomorrow";
  if (days < 30) return `in ${days} days`;
  const months = Math.round(days / 30);
  return months <= 1 ? "next month" : `in ${months} months`;
}

/**
 * The local calendar date a moment falls on, as a count of days.
 *
 * The local year, month and day are read and then counted as if they were a UTC
 * date, which has no daylight-saving changes, so two dates a day apart are always
 * exactly one apart here whatever the clocks did in between.
 */
function calendarDay(moment: Date): number {
  return Math.round(
    Date.UTC(moment.getFullYear(), moment.getMonth(), moment.getDate()) / 86_400_000,
  );
}

/**
 * What a character with no card is called, given how often it was traced.
 *
 * A card exists only once a character has been written from memory: a trace over
 * the faint guide is kept but schedules nothing. So "not practised yet" was
 * wrong for a character traced ten times, and this is the one wording every
 * screen uses for it. `traced` is the count from `ProgressView.traced`.
 */
export function unwrittenLabel(traced: number | undefined): string {
  if (traced && traced > 0) return `traced ${traced}× · not yet written from memory`;
  return "not yet written from memory";
}
