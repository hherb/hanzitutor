import { describe, expect, it } from "vitest";
import type { VoicesView } from "./types";
import { selectedVoice, storedVoiceLabel, voiceFellBack, voiceLabel } from "./voiceChoice";
import type { VoiceOption } from "./types";

/** Apple's novelty voice in two Chinese locales: one name, two voices (issue #39). */
const MAINLAND = "com.apple.eloquence.zh-CN.Eddy";
const TAIWAN = "com.apple.eloquence.zh-TW.Eddy";

function view(chosen: string | null, active: string | null = "Eddy"): VoicesView {
  return {
    available: [
      { id: MAINLAND, name: "Eddy", locale: "zh-CN", network: false, quality: null },
      { id: TAIWAN, name: "Eddy", locale: "zh-TW", network: false, quality: null },
    ],
    active,
    chosen,
    elsewhere: null,
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

  it("treats an empty stored value as no choice at all", () => {
    // `""` is how the automatic choice is *sent*; it is never a voice.
    expect(selectedVoice("", view(null))).toBe("");
    expect(voiceFellBack("", view(null))).toBe(false);
  });
});

describe("how a voice reads", () => {
  const compact: VoiceOption = {
    id: "com.apple.voice.compact.zh-CN.Tingting",
    name: "Tingting",
    locale: "zh-CN",
    network: false,
    quality: "standard",
  };
  const enhanced: VoiceOption = {
    ...compact,
    id: "com.apple.voice.enhanced.zh-CN.Tingting",
    quality: "enhanced",
  };

  it("names the quality only when two entries would otherwise read the same", () => {
    expect(voiceLabel(compact, [compact])).toBe("Tingting — zh-CN");
    expect(voiceLabel(compact, [compact, enhanced])).toBe("Tingting — zh-CN (standard)");
    expect(voiceLabel(enhanced, [compact, enhanced])).toBe("Tingting — zh-CN (enhanced)");
    // Eddy in two locales is told apart by the locale already.
    const view = (): VoiceOption[] => [
      { id: "a", name: "Eddy", locale: "zh-CN", network: false, quality: "standard" },
      { id: "b", name: "Eddy", locale: "zh-TW", network: false, quality: "standard" },
    ];
    expect(voiceLabel(view()[0], view())).toBe("Eddy — zh-CN");
  });

  it("says when a voice needs a network", () => {
    const network: VoiceOption = {
      id: "zh-cn-x-ccc-network",
      name: "zh-cn-x-ccc-network",
      locale: "zh-CN",
      network: true,
      quality: null,
    };
    expect(voiceLabel(network, [network])).toBe("zh-cn-x-ccc-network — zh-CN · needs internet");
  });

  it("reads an Apple identifier as its name and locale", () => {
    expect(storedVoiceLabel("com.apple.voice.compact.zh-CN.Tingting")).toBe("Tingting (zh-CN)");
    expect(storedVoiceLabel("com.apple.eloquence.ja-JP.Eddy")).toBe("Eddy (ja-JP)");
    // A name is shown as it is, qualifier and all.
    expect(storedVoiceLabel("Eddy (Chinese (China mainland))")).toBe(
      "Eddy (Chinese (China mainland))",
    );
    expect(storedVoiceLabel("Meijia")).toBe("Meijia");
    expect(storedVoiceLabel("zh-cn-x-ccc-local")).toBe("zh-cn-x-ccc-local");
  });
});
