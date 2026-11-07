import { describe, expect, it } from "vitest";

import {
  attributionLine,
  correctedLine,
  disclosureLine,
  explanationLine,
  failureLine,
  hostOf,
  reviewHeadline,
  settingsNote,
  settingsStayOpen,
} from "./model";
import type { ModelReview, ModelStatus, ModelVerdict } from "./types";

const status = (over: Partial<ModelStatus> = {}): ModelStatus => ({
  endpoint: "https://api.openai.com/v1",
  model: "gpt-4o-mini",
  keySet: true,
  configured: true,
  warning: null,
  keyNotice: null,
  ...over,
});

const review = (over: Partial<ModelReview> = {}): ModelReview => ({
  verdict: "understandable",
  corrected: "いぬのほうがねこよりすきです。",
  explanation: "Close: the pattern is right, and より is optional here.",
  raw: "{}",
  ...over,
});

const verdict = (over: Partial<ModelVerdict> = {}): ModelVerdict => ({
  review: review(),
  model: "gpt-4o-mini",
  host: "api.openai.com",
  sent: "〜の方が〜より · いぬのほうがねこよりすきです",
  ...over,
});

describe("hostOf", () => {
  it("names the host, however the learner spelled the endpoint", () => {
    expect(hostOf("https://api.openai.com/v1")).toBe("api.openai.com");
    expect(hostOf("http://127.0.0.1:11434/v1")).toBe("127.0.0.1:11434");
    expect(hostOf("http://127.0.0.1:1234/v1/")).toBe("127.0.0.1:1234");
    expect(hostOf("")).toBe("");
  });

  it("names the host the request goes to, never a name or password before an @", () => {
    expect(hostOf("https://api.openai.com@evil.example/v1")).toBe("evil.example");
    expect(hostOf("https://user:hunter2@host.example/v1")).toBe("host.example");
    expect(hostOf("https://API.OpenAI.com:443/v1")).toBe("api.openai.com");
    expect(hostOf("http://[::1]:8080/v1")).toBe("[::1]:8080");
  });

  it("falls back to the text when it is not an address at all", () => {
    expect(hostOf("not an address at all")).toBe("not an address at all");
    expect(hostOf("127.0.0.1:1234/v1")).toBe("127.0.0.1:1234");
  });
});

describe("settingsNote", () => {
  it("says where the settings live and that a key is optional", () => {
    const note = settingsNote();
    expect(note).toContain("this app's own data directory");
    expect(note).toContain("A local server needs no key");
  });
});

describe("disclosureLine", () => {
  it("says what leaves the machine and where it goes, before it goes", () => {
    const line = disclosureLine(status(), "〜の方が〜より");
    expect(line).toContain("Sends the pattern 〜の方が〜より and your sentence");
    expect(line).toContain("api.openai.com");
    expect(line).toContain("gpt-4o-mini");
    expect(line).toContain("nothing is kept");
  });

  it("names a local endpoint as the host a learner typed", () => {
    const line = disclosureLine(
      status({ endpoint: "http://127.0.0.1:11434/v1", model: "llama3.1" }),
      "〜は",
    );
    expect(line).toContain("127.0.0.1:11434");
    expect(line).toContain("llama3.1");
  });
});

describe("reviewHeadline", () => {
  it("echoes the model's own word rather than translating it", () => {
    expect(reviewHeadline(review())).toBe("Understandable");
    expect(reviewHeadline(review({ verdict: "incorrect" }))).toBe("Incorrect");
    expect(reviewHeadline(review({ verdict: "correct" }))).toBe("Correct");
  });

  it("has a heading for a reply that gave no verdict word", () => {
    expect(reviewHeadline(review({ verdict: "" }))).toBe("The model's answer");
  });
});

describe("correctedLine", () => {
  it("shows the version the model offered, and nothing when it offered none", () => {
    expect(correctedLine(review())).toBe("The model would write: いぬのほうがねこよりすきです。");
    expect(correctedLine(review({ corrected: null }))).toBe("");
    expect(correctedLine(review({ corrected: "   " }))).toBe("");
  });
});

describe("explanationLine", () => {
  it("passes the model's explanation through, prose or not", () => {
    expect(explanationLine(review())).toContain("より is optional");
    expect(explanationLine(review({ explanation: "" }))).toBe("");
  });
});

describe("attributionLine", () => {
  it("names the model and the host, and keeps the model's word apart from the app's", () => {
    const line = attributionLine(verdict());
    expect(line).toContain("gpt-4o-mini at api.openai.com");
    expect(line).toContain("not this app");
    expect(line).toContain("can be wrong");
    expect(line).toContain("the checklist above is the only part of this screen that this app decided");
  });
});

describe("failureLine", () => {
  it("shows what the command said, and has something to say when it said nothing", () => {
    expect(failureLine("the endpoint answered 401: Incorrect API key provided")).toContain(
      "Incorrect API key provided",
    );
    expect(failureLine("   ")).toBe("The model check did not finish.");
  });
});

describe("settingsStayOpen", () => {
  it("closes the settings once a save has left something to send", () => {
    expect(settingsStayOpen(status())).toBe(false);
  });

  it("keeps them open while nothing is configured", () => {
    expect(settingsStayOpen(status({ configured: false }))).toBe(true);
  });

  it("keeps them open when a save left the key behind, so the learner can type the new one", () => {
    // Rust drops a stored key when the endpoint moves to another host, and says so;
    // the field that asks for the key is the one place that sentence is any use.
    const moved = status({
      endpoint: "http://192.168.1.20:11434/v1",
      keySet: false,
      keyNotice:
        "the saved key was for api.openai.com and has not been kept for 192.168.1.20:11434: enter the key for the new endpoint, if it needs one",
    });
    expect(settingsStayOpen(moved)).toBe(true);
  });
});
