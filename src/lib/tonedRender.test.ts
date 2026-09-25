import { render } from "svelte/server";
import { afterEach, describe, expect, it } from "vitest";
import TonedPinyin from "./TonedPinyin.svelte";
import TonedText from "./TonedText.svelte";
import { setCharacterTones } from "./tones.svelte";

/**
 * What the toned components actually put on the page.
 *
 * `tones.test.ts` pins the arithmetic — which tone a character or a syllable has.
 * This pins the *markup*, and it is here for one property in particular that
 * nothing else can see: **the spans have to be adjacent**. A newline or a stray
 * space between two characters would put whitespace into the middle of every word
 * in the app, and it is invisible in a diff of the component's own template. So
 * the expected HTML is written out in full rather than matched loosely.
 *
 * Rendered on the server rather than mounted: `svelte/server` needs no DOM, so
 * this costs the project no `jsdom` — which is the reason the test runner is in
 * node mode in the first place.
 */
afterEach(() => setCharacterTones([]));

/** The text nodes of an HTML string, with the tags taken out. */
function textOf(html: string): string {
  return html.replace(/<[^>]*>/g, "");
}

/**
 * The markup with Svelte's hydration markers removed.
 *
 * A server render carries `<!--[-->` and friends around every block, which are
 * comment nodes: they are not laid out, they are gone by the time the client has
 * hydrated, and they say nothing about what the learner sees. What is left is the
 * elements — and, if anybody ever reformats the template, the space between them.
 */
function markup(html: string): string {
  return html.replace(/<!--[\s\S]*?-->/g, "");
}

describe("a run of characters", () => {
  it("writes one span each, with nothing between them", () => {
    setCharacterTones([
      ["学", 2],
      ["习", 2],
      ["好", 3],
    ]);
    const { body } = render(TonedText, { props: { text: "学习好" } });

    expect(markup(body)).toBe(
      '<span class="tone-char" data-tone="2">学</span>' +
        '<span class="tone-char" data-tone="2">习</span>' +
        '<span class="tone-char" data-tone="3">好</span>',
    );
    // The characters are still the characters, and no space crept in. This is
    // the assertion that would fail if somebody reformatted the template.
    expect(textOf(body)).toBe("学习好");
  });

  it("prefers the tones of the text it is read with", () => {
    // 好 is tone 3 on its own and tone 4 in 爱好, and the word's own reading is
    // the one that was sent — a polyphone in context.
    setCharacterTones([
      ["爱", 4],
      ["好", 3],
    ]);
    const { body } = render(TonedText, { props: { text: "爱好", tones: [4, 4] } });
    expect(body).toContain('data-tone="4">爱<');
    expect(body).toContain('data-tone="4">好<');
  });

  it("leaves a character it knows no tone for unpainted", () => {
    // No `data-tone` attribute at all, so `app.css` has nothing to match and the
    // character stays in the ordinary ink rather than taking a guessed colour.
    setCharacterTones([["好", 3]]);
    const { body } = render(TonedText, { props: { text: "好未" } });
    expect(markup(body)).toBe(
      '<span class="tone-char" data-tone="3">好</span><span class="tone-char">未</span>',
    );
  });

  it("writes nothing at all for no text", () => {
    const { body } = render(TonedText, { props: { text: "" } });
    expect(markup(body)).toBe("");
  });
});

describe("a pinyin reading", () => {
  it("writes one span per syllable of the split it was handed", () => {
    const { body } = render(TonedPinyin, {
      props: { text: "dehuà", syllables: ["de", "huà"], tones: [5, 4] },
    });
    expect(markup(body)).toBe(
      '<span class="tone-char tone-pinyin" data-tone="5">de</span>' +
        '<span class="tone-char tone-pinyin" data-tone="4">huà</span>',
    );
    // Nothing between the syllables by default: a word's reading stays run
    // together, and only the colour separates them.
    expect(textOf(body)).toBe("dehuà");
  });

  it("separates syllables when it is asked to, and only between them", () => {
    const { body } = render(TonedPinyin, {
      props: { text: "nǐhǎo", syllables: ["nǐ", "hǎo"], tones: [3, 3], separator: " " },
    });
    expect(textOf(body)).toBe("nǐ hǎo");
  });

  it("reads a lone syllable off its own mark", () => {
    const { body } = render(TonedPinyin, { props: { text: "mǎ" } });
    expect(markup(body)).toBe('<span class="tone-char tone-pinyin" data-tone="3">mǎ</span>');
  });

  it("writes a run of several syllables with no split, uncoloured", () => {
    // The refusal: two marks is a pair run together, and colouring it from one of
    // them would be half the evidence. No `data-tone`, so ordinary ink.
    const { body } = render(TonedPinyin, { props: { text: "xuéxí" } });
    expect(markup(body)).toBe('<span class="tone-char tone-pinyin">xuéxí</span>');
  });
});
