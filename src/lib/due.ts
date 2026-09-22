/**
 * How a due date is said out loud.
 *
 * The backend compares due timestamps as text; the interface only has to make
 * one readable, so this measures from the clock in whole days. It lives here
 * rather than in `App.svelte` because two screens say it — the board's detail
 * card, and the character lookup's — and two copies of "in 5 days" would drift.
 */
export function dueLabel(due: string): string {
  const remaining = Date.parse(due) - Date.now();
  if (Number.isNaN(remaining)) return due;
  if (remaining <= 0) return "now";
  if (remaining < 86_400_000) return "today";
  const days = Math.round(remaining / 86_400_000);
  if (days <= 1) return "tomorrow";
  if (days < 30) return `in ${days} days`;
  const months = Math.round(days / 30);
  return months <= 1 ? "next month" : `in ${months} months`;
}
