import { describe, expect, it } from "vitest";

import {
  hasNextDue,
  kanaScript,
  nextDue,
  overdueLabel,
  reviewPrompt,
  scheduleNote,
  upcomingLabel,
} from "./review";
import type { DueItem } from "./types";

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

  it("says a traced attempt was not counted, whatever the schedule did not do", () => {
    const note = scheduleNote(
      { scheduled: false, traced: true, nextDue: "2026-09-20T09:00:00Z", warning: null },
      now,
    );
    expect(note?.tone).toBe("plain");
    expect(note?.text).toMatch(/^Traced — not counted as a review/);
  });

  it("still puts a failed write ahead of a traced attempt", () => {
    const note = scheduleNote(
      { scheduled: false, traced: true, nextDue: null, warning: "the schedule could not be opened" },
      now,
    );
    expect(note?.tone).toBe("bad");
  });

  it("has nothing to say about nothing", () => {
    expect(scheduleNote({ scheduled: true, nextDue: null, warning: null }, now)).toEqual({
      tone: "ok",
      text: "Saved for review.",
    });
  });
});

/** One due item, with the fields the queue arithmetic does not look at. */
const due = (ch: string, kind: "kana" | "kanji" | "radical" = "kana"): DueItem => ({
  ch,
  kind,
  hint: ch,
  radical: kind === "radical" ? 9 : null,
  due: "2026-09-19T09:00:00Z",
  intervalDays: 1,
  attempts: 1,
  lapses: 0,
});

describe("nextDue", () => {
  it("is the first due character that is not the one on the board", () => {
    const items = [due("あ"), due("い"), due("う")];
    expect(nextDue(items, items[0])?.ch).toBe("い");
    expect(nextDue(items, items[1])?.ch).toBe("あ");
    expect(nextDue(items, null)?.ch).toBe("あ");
  });

  it("answers nothing when the queue holds only the character on the board", () => {
    expect(nextDue([due("あ")], due("あ"))).toBeNull();
    expect(nextDue([], null)).toBeNull();
  });

  it("tells a character from the same character's radical", () => {
    // 人 is both a jōyō character and radical 9, and a card is keyed by the
    // character — so the kind is part of the identity rather than decoration.
    const asKanji = due("人", "kanji");
    const asRadical = due("人", "radical");
    expect(nextDue([asKanji], asRadical)?.kind).toBe("kanji");
  });
});

describe("hasNextDue", () => {
  it("is true while the page holds another due character", () => {
    const items = [due("あ"), due("い")];
    expect(hasNextDue({ items, due: 2 }, items[0])).toBe(true);
    // The last item of the page is not the end of the queue: the first item is
    // the next one, which is what ordering by how overdue each character is means.
    expect(hasNextDue({ items, due: 2 }, items[1])).toBe(true);
  });

  it("is true when the page holds only the character on the board and the queue has more", () => {
    // The guard the second term exists for: "nothing due" is a claim about the
    // *queue*, and a screen that answered from the page alone would make it over
    // characters it had not asked for. The command returns
    // `min(limit, due)` items today, so this is a guard rather than the usual path
    // — but it is the difference between an emptied queue and an unasked-for page.
    expect(hasNextDue({ items: [due("あ")], due: 2 }, due("あ"))).toBe(true);
  });

  it("is false only for a queue with nothing left in it", () => {
    expect(hasNextDue({ items: [due("あ")], due: 1 }, due("あ"))).toBe(false);
    expect(hasNextDue({ items: [], due: 0 }, null)).toBe(false);
  });
});

describe("kanaScript", () => {
  it("tells the two scripts apart by block", () => {
    expect(kanaScript("か")).toBe("hiragana");
    expect(kanaScript("カ")).toBe("katakana");
    expect(kanaScript("ー")).toBe("katakana");
  });

  it("answers nothing for a character that is not a kana", () => {
    expect(kanaScript("学")).toBeNull();
    expect(kanaScript("")).toBeNull();
  });
});

describe("reviewPrompt", () => {
  /** A due item with the prompt Rust sends for it. */
  const item = (ch: string, kind: "kana" | "kanji" | "radical", hint: string): DueItem => ({
    ...due(ch, kind),
    hint,
  });

  it("never shows the character before the attempt is graded", () => {
    for (const entry of [
      item("か", "kana", "ka"),
      item("学", "kanji", "study"),
      item("亅", "radical", "radical 6 of 214"),
    ]) {
      const prompt = reviewPrompt(entry, false);
      expect(prompt.glyph).toBeNull();
      expect(prompt.hint).not.toContain(entry.ch);
    }
  });

  it("reveals the character once the attempt is graded", () => {
    expect(reviewPrompt(item("学", "kanji", "study"), true).glyph).toBe("学");
  });

  it("names the script of a kana, so ka is answerable as か or カ", () => {
    expect(reviewPrompt(item("か", "kana", "ka"), false).hint).toBe("ka · hiragana");
    expect(reviewPrompt(item("カ", "kana", "ka"), false).hint).toBe("ka · katakana");
  });

  it("says what kind of character a kanji prompt is asking for", () => {
    expect(reviewPrompt(item("学", "kanji", "study"), false).hint).toBe("study · kanji");
    expect(reviewPrompt(item("亅", "radical", "radical 6 of 214"), false).hint).toBe(
      "radical 6 of 214",
    );
  });
});
