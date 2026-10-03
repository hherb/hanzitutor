import { describe, expect, it } from "vitest";
import { canHear, isHearItKey, voiceNote, type KeyLike } from "./speech";

describe("what can be heard", () => {
  it("says nothing is wrong when a voice is in use", () => {
    const voice = "Kyoko (Japanese (Japan)) (ja_JP)";
    expect(canHear(voice)).toBe(true);
    expect(voiceNote(voice)).toBeNull();
  });

  it("tells the learner where to add a voice when the machine has none", () => {
    // The one state a button cannot express: it has to say what is missing and
    // where to fix it, under the app's own override variable rather than the
    // Chinese app's.
    expect(canHear(null)).toBe(false);
    const note = voiceNote(null);
    expect(note).toContain("No Japanese voice is installed");
    expect(note).toContain("Manage Voices");
    expect(note).toContain("NIHONGO_TUTOR_VOICE");
  });

  it("does not call an answer that has not arrived a missing voice", () => {
    // `undefined` is "still asking". Reporting it as absent would be a claim the
    // app is about to contradict, and it would flash on every cold start.
    expect(canHear(undefined)).toBe(false);
    expect(voiceNote(undefined)).toBe("Looking for a Japanese voice…");
    expect(voiceNote(undefined)).not.toContain("No Japanese voice");
  });
});

describe("the hear-it shortcut", () => {
  const press = (key: string, extra: Partial<KeyLike> = {}): KeyLike => ({
    key,
    target: { tagName: "BODY" },
    ...extra,
  });

  it("is h, in either case, with nothing held", () => {
    expect(isHearItKey(press("h"))).toBe(true);
    expect(isHearItKey(press("H"))).toBe(true);
  });

  it("is not a key held down", () => {
    expect(isHearItKey(press("h", { repeat: true }))).toBe(false);
  });

  it("is not a modified key, so the window's own shortcuts survive", () => {
    // ⌘H hides the app on macOS. A shortcut that ate it would be worse than no
    // shortcut at all.
    expect(isHearItKey(press("h", { metaKey: true }))).toBe(false);
    expect(isHearItKey(press("h", { ctrlKey: true }))).toBe(false);
    expect(isHearItKey(press("h", { altKey: true }))).toBe(false);
  });

  it("is not any other key", () => {
    for (const key of ["a", " ", "Enter", "は"]) {
      expect(isHearItKey(press(key)), key).toBe(false);
    }
  });

  it("is not a letter meant for a field being typed in", () => {
    // The important one: `は` is typed `ha`, and the reading box is exactly where
    // this rule would otherwise steal a keystroke from the learner.
    for (const tagName of ["INPUT", "TEXTAREA", "SELECT", "input"]) {
      expect(isHearItKey(press("h", { target: { tagName } })), tagName).toBe(false);
    }
    expect(
      isHearItKey(press("h", { target: { tagName: "DIV", isContentEditable: true } })),
    ).toBe(false);
  });

  it("is still h on a button, where it has no other meaning", () => {
    // Unlike Space, which activates the focused button: this is why the shortcut
    // is a letter. A learner who has just clicked a kana cell is focused on it.
    expect(isHearItKey(press("h", { target: { tagName: "BUTTON" } }))).toBe(true);
  });

  it("is h with no target at all", () => {
    expect(isHearItKey({ key: "h" })).toBe(true);
  });
});
