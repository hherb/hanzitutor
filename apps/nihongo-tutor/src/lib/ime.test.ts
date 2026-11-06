import { afterEach, describe, expect, it, vi } from "vitest";
import {
  IME_KEY_CODE,
  hasJapaneseScript,
  isComposingKey,
  isSubmitEnter,
  normaliseTyped,
  romajiOnlyNote,
  trackComposition,
} from "./ime";

describe("the Enter that submits", () => {
  it("is a plain Enter", () => {
    expect(isSubmitEnter({ key: "Enter" })).toBe(true);
    expect(isSubmitEnter({ key: "Enter", isComposing: false, keyCode: 13 })).toBe(true);
  });

  it("is not the Enter that commits a candidate while the composition is open", () => {
    // Chromium and Gecko: the keydown arrives before compositionend.
    expect(isSubmitEnter({ key: "Enter", isComposing: true, keyCode: 229 })).toBe(false);
    expect(isSubmitEnter({ key: "Enter", isComposing: true })).toBe(false);
  });

  it("is not WebKit's Enter, which arrives after the composition has closed", () => {
    // The case this module exists for: the macOS and iOS webviews report
    // isComposing = false, and only keyCode 229 says the IME had the key.
    expect(isSubmitEnter({ key: "Enter", isComposing: false, keyCode: IME_KEY_CODE })).toBe(
      false,
    );
  });

  it("is not any other key", () => {
    for (const key of ["a", " ", "Process", "Tab"]) {
      expect(isSubmitEnter({ key }), key).toBe(false);
    }
  });
});

describe("a key the input method owns", () => {
  it("is one with either sign, and no other", () => {
    expect(isComposingKey({ key: "Process", keyCode: 229 })).toBe(true);
    expect(isComposingKey({ key: "a", isComposing: true })).toBe(true);
    expect(isComposingKey({ key: "h", keyCode: 72 })).toBe(false);
    expect(isComposingKey({ key: "h", target: null })).toBe(false);
  });
});

describe("watching a field's composition", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("marks the field from compositionstart until a task after compositionend", () => {
    vi.useFakeTimers();
    const field = new EventTarget();
    const action = trackComposition(field);
    // The Enter carries neither sign, which is the worst case: the mark is all that
    // keeps it from submitting.
    const enter = { key: "Enter", isComposing: false, keyCode: 13, target: field };

    expect(isSubmitEnter(enter)).toBe(true);

    field.dispatchEvent(new Event("compositionstart"));
    expect(isSubmitEnter(enter)).toBe(false);

    // WebKit's order: compositionend, and only then the keydown for the Enter
    // that ended it. Still marked, because that keydown is in the same task.
    field.dispatchEvent(new Event("compositionend"));
    expect(isSubmitEnter(enter)).toBe(false);

    // A later task: the next Enter is the learner's.
    vi.runAllTimers();
    expect(isSubmitEnter(enter)).toBe(true);

    action.destroy();
  });

  it("stays marked when a new composition starts before the old mark clears", () => {
    vi.useFakeTimers();
    const field = new EventTarget();
    const action = trackComposition(field);
    const enter = { key: "Enter", target: field };

    field.dispatchEvent(new Event("compositionstart"));
    field.dispatchEvent(new Event("compositionend"));
    field.dispatchEvent(new Event("compositionstart"));
    vi.runAllTimers();
    expect(isSubmitEnter(enter)).toBe(false);

    action.destroy();
  });

  it("forgets the field once the action is destroyed", () => {
    const field = new EventTarget();
    const action = trackComposition(field);
    field.dispatchEvent(new Event("compositionstart"));
    action.destroy();
    expect(isSubmitEnter({ key: "Enter", target: field })).toBe(true);
    // And no longer listens.
    field.dispatchEvent(new Event("compositionstart"));
    expect(isSubmitEnter({ key: "Enter", target: field })).toBe(true);
  });

  it("marks only the field it watches", () => {
    const field = new EventTarget();
    const other = new EventTarget();
    const action = trackComposition(field);
    field.dispatchEvent(new Event("compositionstart"));
    expect(isSubmitEnter({ key: "Enter", target: other })).toBe(true);
    action.destroy();
  });
});

describe("what is sent to be graded", () => {
  it("folds full-width romaji to ASCII", () => {
    expect(normaliseTyped("ｓｈｉ")).toBe("shi");
    expect(normaliseTyped("ＳＨＩ")).toBe("SHI");
    expect(normaliseTyped("ｎａｉｔｏ")).toBe("naito");
  });

  it("folds the ideographic space to an ordinary one", () => {
    expect(normaliseTyped("shi　")).toBe("shi ");
    expect(normaliseTyped("ｎｏ　ｈｏｕｇａ")).toBe("no houga");
  });

  it("leaves ordinary romaji and kana as they are", () => {
    for (const typed of ["shi", "si", "kya", "n'a", "し", "ないといけない", "シャ", "ー", "っ"]) {
      expect(normaliseTyped(typed), typed).toBe(typed);
    }
  });

  it("keeps precomposed voiced kana whole", () => {
    // NFKC composes: が, ぱ and ヴ stay one character each.
    for (const typed of ["が", "ぱ", "ヴ", "ゔ", "じゃ", "ぴょ"]) {
      const normalised = normaliseTyped(typed);
      expect(normalised, typed).toBe(typed);
      expect([...normalised].length, typed).toBe([...typed].length);
    }
  });

  it("composes a kana and a combining voiced mark", () => {
    expect(normaliseTyped("が")).toBe("が");
    expect(normaliseTyped("ぱ")).toBe("ぱ");
  });

  it("turns a spacing voiced mark after a kana into the voiced kana", () => {
    // NFKC alone would make か゛ into "か", a space and a combining mark.
    expect("か゛".normalize("NFKC")).toBe("か ゙");
    expect(normaliseTyped("か゛")).toBe("が");
    expect(normaliseTyped("は゜")).toBe("ぱ");
    expect(normaliseTyped("カ゛")).toBe("ガ");
  });

  it("widens half-width katakana, voiced marks and all", () => {
    expect(normaliseTyped("ｼ")).toBe("シ");
    expect(normaliseTyped("ｶﾞ")).toBe("ガ");
    expect(normaliseTyped("ﾊﾟ")).toBe("パ");
    expect(normaliseTyped("ｰ")).toBe("ー");
  });
});

describe("kana in the romaji box", () => {
  it("is noticed, in either script or as kanji", () => {
    for (const typed of ["し", "シ", "ｼ", "shiし", "詩", "ー"]) {
      expect(hasJapaneseScript(typed), typed).toBe(true);
      expect(romajiOnlyNote(typed), typed).toContain("romaji");
    }
  });

  it("is not claimed for romaji, full-width or not", () => {
    for (const typed of ["shi", "si", "ｓｈｉ", "", "zzz", "n'"]) {
      expect(hasJapaneseScript(typed), typed).toBe(false);
      expect(romajiOnlyNote(typed), typed).toBeNull();
    }
  });
});
