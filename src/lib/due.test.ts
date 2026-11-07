import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { dueLabel } from "./due";

/**
 * A wall-clock moment in the zone the tests run in.
 *
 * Built from local fields rather than from an ISO string, because the rule under
 * test is about the learner's *calendar*: "tomorrow" is the next date on their
 * wall, wherever that wall is. The due date crosses as the backend sends it, an
 * ISO timestamp, so the label is exercised on the same input it gets in the app.
 */
function local(year: number, month: number, day: number, hour = 0, minute = 0): Date {
  return new Date(year, month - 1, day, hour, minute);
}
const iso = (date: Date) => date.toISOString();

describe("dueLabel", () => {
  it("calls a card due at 08:00 tomorrow 'tomorrow', even at 22:00 the night before", () => {
    // Ten hours away is under a day, which is what used to make it "today".
    const now = local(2026, 9, 14, 22, 0);
    expect(dueLabel(iso(local(2026, 9, 15, 8, 0)), now)).toBe("tomorrow");
  });

  it("says 'today' only for a time later on the same date", () => {
    const now = local(2026, 9, 14, 8, 0);
    expect(dueLabel(iso(local(2026, 9, 14, 21, 0)), now)).toBe("today");
    expect(dueLabel(iso(local(2026, 9, 14, 23, 59)), now)).toBe("today");
  });

  it("counts a day and a half by the dates it lands on, not by rounding", () => {
    // From the morning, a day and a half is tomorrow evening.
    expect(dueLabel(iso(local(2026, 9, 15, 20, 0)), local(2026, 9, 14, 8, 0))).toBe(
      "tomorrow",
    );
    // From late at night, it is the morning after next.
    expect(dueLabel(iso(local(2026, 9, 16, 10, 0)), local(2026, 9, 14, 22, 0))).toBe(
      "in 2 days",
    );
  });

  it("counts whole calendar days further out", () => {
    const now = local(2026, 9, 14, 23, 0);
    expect(dueLabel(iso(local(2026, 9, 19, 0, 30)), now)).toBe("in 5 days");
    expect(dueLabel(iso(local(2026, 10, 13, 12, 0)), now)).toBe("in 29 days");
  });

  it("says months once it is a month or more away", () => {
    const now = local(2026, 9, 14, 12, 0);
    expect(dueLabel(iso(local(2026, 10, 14, 12, 0)), now)).toBe("next month");
    expect(dueLabel(iso(local(2026, 12, 14, 12, 0)), now)).toBe("in 3 months");
  });

  it("says 'now' for anything already due", () => {
    const now = local(2026, 9, 14, 12, 0);
    expect(dueLabel(iso(now), now)).toBe("now");
    expect(dueLabel(iso(local(2026, 9, 14, 11, 59)), now)).toBe("now");
    expect(dueLabel(iso(local(2026, 9, 1, 9, 0)), now)).toBe("now");
  });

  it("hands back text it cannot read as a date", () => {
    expect(dueLabel("not a date", local(2026, 9, 14))).toBe("not a date");
  });

  it("reads the clock when no moment is given", () => {
    const soon = new Date(Date.now() + 10 * 86_400_000);
    expect(dueLabel(iso(soon))).toMatch(/^in (9|10|11) days$/);
  });
});

/**
 * Across a daylight-saving change the day is 23 or 25 hours long, which is exactly
 * where a 24-hour window and a calendar disagree. Run in a zone that has one, and
 * put the zone back afterwards: Node re-reads `TZ` when it changes.
 */
describe("dueLabel across a daylight-saving change", () => {
  beforeAll(() => {
    vi.stubEnv("TZ", "Europe/Berlin");
  });
  afterAll(() => {
    vi.unstubAllEnvs();
  });

  it("is in a zone that changes its clocks", () => {
    // 29 March 2026 is 23 hours long in Berlin. If this fails, the zone was not
    // applied and the cases below prove nothing.
    expect(local(2026, 3, 30).getTime() - local(2026, 3, 29).getTime()).toBe(23 * 3_600_000);
  });

  it("calls tomorrow evening 'tomorrow' over the short spring day", () => {
    // 22½ hours away: under a day by the clock, but the next date.
    expect(dueLabel(iso(local(2026, 3, 29, 23, 0)), local(2026, 3, 28, 23, 30))).toBe(
      "tomorrow",
    );
  });

  it("calls the day after tomorrow 'in 2 days' even when it is only 24 hours away", () => {
    expect(dueLabel(iso(local(2026, 3, 30, 0, 30)), local(2026, 3, 28, 23, 30))).toBe(
      "in 2 days",
    );
  });

  it("calls the day after tomorrow 'in 2 days' over the long autumn day", () => {
    // 25 October 2026 is 25 hours long: just after midnight on the 26th is 27½
    // hours after 22:00 on the 24th, which a rolling window rounds to one day.
    expect(dueLabel(iso(local(2026, 10, 26, 0, 30)), local(2026, 10, 24, 22, 0))).toBe(
      "in 2 days",
    );
  });
});
