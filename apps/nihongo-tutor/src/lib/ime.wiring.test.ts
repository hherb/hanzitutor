import { describe, expect, it } from "vitest";

/**
 * Every screen's source, as text.
 *
 * A check on the wiring rather than on a rule: `ime.test.ts` proves that
 * `isSubmitEnter` tells the IME's Enter from the learner's, and this proves the
 * screens ask it. Read as source because the tests run without a DOM, and because
 * the regression it guards against is exactly a new `key === "Enter"` written into
 * a component.
 */
const sources = import.meta.glob<string>("./*.svelte", {
  query: "?raw",
  import: "default",
  eager: true,
});

/** Every element tag that has an `onkeydown`, with the file it is in. */
function keydownElements(): { file: string; tag: string }[] {
  const found: { file: string; tag: string }[] = [];
  for (const [file, source] of Object.entries(sources)) {
    // An opening tag runs to the first `>` that is not inside a `{…}` expression,
    // and `=>` inside a handler is one; so match tags by their braces instead.
    const tags = source.match(/<(?!svelte:)[a-z]+\b(?:[^>{}]|\{(?:[^{}]|\{[^{}]*\})*\})*>/g) ?? [];
    for (const tag of tags) {
      if (tag.includes("onkeydown=")) found.push({ file, tag });
    }
  }
  return found;
}

describe("the screens' Enter handlers", () => {
  it("are found at all", () => {
    // The three answer boxes this was written for; if the search finds fewer, it has
    // stopped working rather than the screens having stopped listening.
    const files = keydownElements().map((element) => element.file);
    for (const file of ["./GrammarProduction.svelte", "./WordCard.svelte", "./KanaPractice.svelte"]) {
      expect(files, file).toContain(file);
    }
  });

  it("all ask isSubmitEnter, on a field whose composition is watched", () => {
    for (const { file, tag } of keydownElements()) {
      expect(tag, file).toContain("isSubmitEnter(");
      expect(tag, file).toContain("use:trackComposition");
    }
  });

  it("all keep the system from rewriting romaji", () => {
    for (const { file, tag } of keydownElements()) {
      for (const attribute of ['autocorrect="off"', 'autocapitalize="off"', 'spellcheck="false"']) {
        expect(tag, `${file} ${attribute}`).toContain(attribute);
      }
    }
  });

  it("never test for Enter by name", () => {
    for (const [file, source] of Object.entries(sources)) {
      expect(source, file).not.toMatch(/key\s*===?\s*["']Enter["']/);
    }
  });
});
