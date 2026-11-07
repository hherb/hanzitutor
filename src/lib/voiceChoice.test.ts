import { describe, expect, it } from "vitest";
import type { VoicesView } from "./types";
import { selectedVoice, voiceFellBack } from "./voiceChoice";

/** Apple's novelty voice in two Chinese locales: one name, two voices (issue #39). */
const MAINLAND = "com.apple.eloquence.zh-CN.Eddy";
const TAIWAN = "com.apple.eloquence.zh-TW.Eddy";

function view(chosen: string | null, active: string | null = "Eddy"): VoicesView {
  return {
    available: [
      { id: MAINLAND, name: "Eddy", locale: "zh-CN", network: false },
      { id: TAIWAN, name: "Eddy", locale: "zh-TW", network: false },
    ],
    active,
    chosen,
  };
}

describe("the voice the settings list shows", () => {
  it("is the automatic entry when nothing is chosen", () => {
    expect(selectedVoice(null, view(null))).toBe("");
  });

  it("is the stored id when it is listed", () => {
    expect(selectedVoice(TAIWAN, view(TAIWAN))).toBe(TAIWAN);
  });

  it("selects the entry a bare name from an older build resolves to", () => {
    // `Eddy` is two entries now; the backend says which one it means.
    expect(selectedVoice("Eddy", view(MAINLAND))).toBe(MAINLAND);
    expect(voiceFellBack("Eddy", view(MAINLAND))).toBe(false);
  });

  it("does not snap back to the previous choice before the list is refreshed", () => {
    // The learner has just picked the Taiwanese Eddy; `chosen` still names
    // the mainland one until the save comes back.
    expect(selectedVoice(TAIWAN, view(MAINLAND))).toBe(TAIWAN);
    expect(voiceFellBack(TAIWAN, view(null))).toBe(false);
  });

  it("keeps a voice this machine does not have selected, and says it fell back", () => {
    const gone = "com.apple.voice.compact.zh-CN.Tingting";
    expect(selectedVoice(gone, view(null))).toBe(gone);
    expect(voiceFellBack(gone, view(null))).toBe(true);
    // With no voice at all there is nothing it fell back *to*.
    expect(voiceFellBack(gone, view(null, null))).toBe(false);
    expect(voiceFellBack(null, view(null))).toBe(false);
  });
});
