import { describe, expect, it } from "vitest";

import { overdueLabel, scheduleNote, upcomingLabel } from "./review";

/**
 * The Review screen's arithmetic.
 *
 * Every case is a sentence a learner reads, so the boundaries are the point: a
 * due date that has just passed must not say "0 days overdue", a date an hour
 * ahead must not say "0 hours", and the three outcomes of a graded attempt have
 * to be distinguishable without the schedule being re-derived here.
 */

const at = (iso: string) => iso;

describe("overdueLabel", () => {
  it("says a date that has just passed is due now, not zero days overdue", () => {
    expect(overdueLabel(at("2026-09-19T09:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "due now",
    );
    // A second in the future is a clock skew rather than a schedule.
    expect(overdueLabel(at("2026-09-19T09:00:01Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "due now",
    );
  });

  it("counts minutes, then hours, then days", () => {
    expect(overdueLabel(at("2026-09-19T08:59:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "1 minute overdue",
    );
    expect(overdueLabel(at("2026-09-19T08:30:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "30 minutes overdue",
    );
    expect(overdueLabel(at("2026-09-19T08:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "1 hour overdue",
    );
    expect(overdueLabel(at("2026-09-18T09:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "1 day overdue",
    );
    expect(overdueLabel(at("2026-09-16T09:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "3 days overdue",
    );
  });

  it("does not claim a label for a date it cannot read", () => {
    expect(overdueLabel("not a date", at("2026-09-19T09:00:00Z"))).toBe("due now");
  });
});

describe("upcomingLabel", () => {
  it("is the mirror of overdueLabel, and never says a past date is ahead", () => {
    expect(upcomingLabel(at("2026-09-19T09:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe("now");
    expect(upcomingLabel(at("2026-09-18T09:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe("now");
    expect(upcomingLabel(at("2026-09-19T10:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "in 1 hour",
    );
    expect(upcomingLabel(at("2026-09-20T09:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "in 1 day",
    );
    expect(upcomingLabel(at("2026-09-26T09:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "in 7 days",
    );
  });

  it("rounds a whole-day interval read a moment late, rather than dropping a day", () => {
    // The live case: a two-day interval, looked at forty seconds after the
    // attempt. Flooring this said "in 1 day" beside an interval of 2.
    expect(upcomingLabel(at("2026-09-21T08:59:20Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "in 2 days",
    );
    expect(upcomingLabel(at("2026-09-20T08:59:20Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "in 1 day",
    );
  });

  it("keeps SM-2's half-day interval in hours, where it belongs", () => {
    // A "hard" pass is scheduled twelve hours out; "in 1 day" would be a lie
    // about the schedule the same screen is showing.
    expect(upcomingLabel(at("2026-09-19T21:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "in 12 hours",
    );
    expect(upcomingLabel(at("2026-09-19T15:00:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "in 6 hours",
    );
  });

  it("says a failure's minute in minutes", () => {
    expect(upcomingLabel(at("2026-09-19T09:01:00Z"), at("2026-09-19T09:00:00Z"))).toBe(
      "in 1 minute",
    );
  });
});

describe("scheduleNote", () => {
  const now = at("2026-09-19T09:00:00Z");

  it("reports a counted attempt with when it comes back", () => {
    const note = scheduleNote(
      { scheduled: true, nextDue: "2026-09-20T09:00:00Z", warning: null },
      now,
    );
    expect(note?.tone).toBe("ok");
    expect(note?.text).toBe("Saved for review — next in 1 day.");
  });

  it("says so when the attempt was practice rather than a review", () => {
    const note = scheduleNote({ scheduled: false, nextDue: null, warning: null }, now);
    expect(note?.tone).toBe("plain");
    expect(note?.text).toContain("practice");
  });

  it("puts a failed write ahead of everything else", () => {
    const note = scheduleNote(
      {
        scheduled: true,
        nextDue: "2026-09-20T09:00:00Z",
        warning: "the attempt was counted but could not be saved: denied",
      },
      now,
    );
    expect(note?.tone).toBe("bad");
    expect(note?.text).toContain("could not be saved");
  });

  it("has nothing to say about nothing", () => {
    expect(scheduleNote({ scheduled: true, nextDue: null, warning: null }, now)).toEqual({
      tone: "ok",
      text: "Saved for review.",
    });
  });
});
