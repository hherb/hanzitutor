import { describe, expect, it, vi } from "vitest";
import {
  COMPOSITION_END_WINDOW_MS,
  IME_KEY_CODE,
  foldForSentence,
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

/** A composition or focus event with a timestamp of the test's choosing. */
function at(type: string, timeStamp: number): Event {
  const event = new Event(type);
  Object.defineProperty(event, "timeStamp", { value: timeStamp });
  return event;
}

describe("watching a field's composition", () => {
  it("marks the field while the composition is open", () => {
    const field = new EventTarget();
    const action = trackComposition(field);
    // The Enter carries neither DOM sign, which is the worst case: what the action
    // keeps is all that stands between it and a submitted answer.
    const enter = { key: "Enter", isComposing: false, keyCode: 13, target: field };

    expect(isSubmitEnter({ ...enter, timeStamp: 900 })).toBe(true);
    field.dispatchEvent(at("compositionstart", 1000));
    expect(isSubmitEnter({ ...enter, timeStamp: 1010 })).toBe(false);
    action.destroy();
  });

  it("takes an Enter close to compositionend as the one that ended it, either side", () => {
    const field = new EventTarget();
    const action = trackComposition(field);
    const enter = (timeStamp: number) => ({ key: "Enter", keyCode: 13, target: field, timeStamp });

    field.dispatchEvent(at("compositionstart", 1000));
    field.dispatchEvent(at("compositionend", 2000));
    // After the end, as WebKit dispatches it, in whatever task.
    expect(isSubmitEnter(enter(2000))).toBe(false);
    expect(isSubmitEnter(enter(2000 + COMPOSITION_END_WINDOW_MS))).toBe(false);
    // Stamped with the native key press, a little before the end it caused.
    expect(isSubmitEnter(enter(1990))).toBe(false);
    // A deliberate second press, later.
    expect(isSubmitEnter(enter(2000 + COMPOSITION_END_WINDOW_MS + 1))).toBe(true);
    expect(isSubmitEnter(enter(2400))).toBe(true);
    action.destroy();
  });

  it("does not depend on a timer or on task order", () => {
    vi.useFakeTimers();
    try {
      const field = new EventTarget();
      const action = trackComposition(field);
      field.dispatchEvent(at("compositionstart", 1000));
      field.dispatchEvent(at("compositionend", 2000));
      // Every pending task run before the keydown arrives: still the IME's.
      vi.runAllTimers();
      expect(isSubmitEnter({ key: "Enter", target: field, timeStamp: 2005 })).toBe(false);
      action.destroy();
    } finally {
      vi.useRealTimers();
    }
  });

  it("forgets a composition when the field loses focus, so Enter cannot stay dead", () => {
    const field = new EventTarget();
    const action = trackComposition(field);
    // A composition whose end never arrived.
    field.dispatchEvent(at("compositionstart", 1000));
    field.dispatchEvent(at("focusout", 1500));
    expect(isSubmitEnter({ key: "Enter", target: field, timeStamp: 1600 })).toBe(true);

    // And an end that did arrive is forgotten too.
    field.dispatchEvent(at("compositionstart", 2000));
    field.dispatchEvent(at("compositionend", 2100));
    field.dispatchEvent(at("focusout", 2101));
    expect(isSubmitEnter({ key: "Enter", target: field, timeStamp: 2110 })).toBe(true);
    action.destroy();
  });

  it("starts afresh when a new composition opens", () => {
    const field = new EventTarget();
    const action = trackComposition(field);
    field.dispatchEvent(at("compositionstart", 1000));
    field.dispatchEvent(at("compositionend", 1100));
    field.dispatchEvent(at("compositionstart", 1120));
    expect(isSubmitEnter({ key: "Enter", target: field, timeStamp: 5000 })).toBe(false);
    field.dispatchEvent(at("compositionend", 6000));
    expect(isSubmitEnter({ key: "Enter", target: field, timeStamp: 6010 })).toBe(false);
    expect(isSubmitEnter({ key: "Enter", target: field, timeStamp: 7000 })).toBe(true);
    action.destroy();
  });

  it("is not fooled by a keydown with no timestamp", () => {
    const field = new EventTarget();
    const action = trackComposition(field);
    field.dispatchEvent(at("compositionstart", 1000));
    field.dispatchEvent(at("compositionend", 1100));
    expect(isSubmitEnter({ key: "Enter", target: field })).toBe(true);
    action.destroy();
  });

  it("forgets the field once the action is destroyed", () => {
    const field = new EventTarget();
    const action = trackComposition(field);
    field.dispatchEvent(at("compositionstart", 1000));
    action.destroy();
    expect(isSubmitEnter({ key: "Enter", target: field, timeStamp: 1010 })).toBe(true);
    // And no longer listens.
    field.dispatchEvent(at("compositionstart", 2000));
    expect(isSubmitEnter({ key: "Enter", target: field, timeStamp: 2010 })).toBe(true);
  });

  it("marks only the field it watches", () => {
    const field = new EventTarget();
    const other = new EventTarget();
    const action = trackComposition(field);
    field.dispatchEvent(at("compositionstart", 1000));
    field.dispatchEvent(at("compositionend", 1100));
    expect(isSubmitEnter({ key: "Enter", target: other, timeStamp: 1105 })).toBe(true);
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
    expect(normaliseTyped("shi\u3000")).toBe("shi ");
    expect(normaliseTyped("ｎｏ\u3000ｈｏｕｇａ")).toBe("no houga");
  });

  it("turns typographic apostrophes into the one the input table spells n' with", () => {
    // iOS Smart Punctuation writes kin'you as kin’you; NFKC alone keeps the ’.
    expect("kin\u2019you".normalize("NFKC")).toBe("kin\u2019you");
    expect(normaliseTyped("kin\u2019you")).toBe("kin'you");
    expect(normaliseTyped("kin\u2018you")).toBe("kin'you");
    expect(normaliseTyped("kin\u2032you")).toBe("kin'you");
    expect(normaliseTyped("ｋｉｎ\u2019ｙｏｕ")).toBe("kin'you");
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
    expect(normaliseTyped("か\u3099")).toBe("が");
    expect(normaliseTyped("は\u309A")).toBe("ぱ");
  });

  it("turns a spacing voiced mark after a kana into the voiced kana", () => {
    // NFKC alone would make か゛ into "か", a space and a combining mark.
    expect("か゛".normalize("NFKC")).toBe("か \u3099");
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

describe("a sentence of the learner's own", () => {
  it("folds full-width romaji and digits, and nothing around them", () => {
    expect(foldForSentence("ｉｎｕ ｎｏ ｈｏｕｇａ ｎｅｋｏ ｙｏｒｉ ｓｕｋｉ ｄｅｓｕ。")).toBe(
      "inu no houga neko yori suki desu。",
    );
    expect(foldForSentence("ＩＮＵ")).toBe("INU");
    expect(foldForSentence("２０２６年")).toBe("2026年");
  });

  it("keeps the sentence's own punctuation and spaces as written", () => {
    // NFKC would make each of these ASCII; the sentence is kept as typed.
    const written = "本当？\u3000すごい！～（笑）、です。";
    expect(foldForSentence(written)).toBe(written);
    expect(written.normalize("NFKC")).not.toBe(written);
  });

  it("widens half-width katakana, joining a separate voiced mark", () => {
    expect(foldForSentence("ｺｰﾋｰが好き")).toBe("コーヒーが好き");
    expect(foldForSentence("ｶﾞｯｺｳ")).toBe("ガッコウ");
    expect(foldForSentence("ﾊﾟﾝ？")).toBe("パン？");
  });

  it("turns typographic apostrophes into the ASCII one", () => {
    expect(foldForSentence("kin\u2019you ni")).toBe("kin'you ni");
  });

  it("leaves a sentence in ordinary kana, kanji and romaji alone", () => {
    for (const written of ["犬の方が猫より好きです。", "inu no houga neko yori suki desu.", "が ぱ ヴ"]) {
      expect(foldForSentence(written), written).toBe(written);
    }
  });
});
