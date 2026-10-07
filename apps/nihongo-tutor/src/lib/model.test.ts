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
} from "./model";
import type { ModelReview, ModelStatus, ModelVerdict } from "./types";

const status = (over: Partial<ModelStatus> = {}): ModelStatus => ({
  endpoint: "https://api.openai.com/v1",
  model: "gpt-4o-mini",
  keySet: true,
  configured: true,
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
