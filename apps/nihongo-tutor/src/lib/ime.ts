/**
 * What a Japanese input method does to a keypress and to a typed answer, as pure
 * functions and one small action.
 *
 * Every answer box in this app is one a learner may type into with the Japanese IME
 * on, and the IME changes three things the screens would otherwise get wrong:
 *
 * * **the Enter that confirms a candidate.** Typing `naito` shows ないと underlined,
 *   and Enter commits it. That Enter is the IME's, not the learner's "check my
 *   answer" — but WebKit, which is the webview of both the macOS and the iOS shells,
 *   delivers it as a keydown *after* `compositionend`, with `isComposing` already
 *   false. What gives it away is `keyCode` 229, the code every engine uses for a key
 *   the IME has consumed. Chromium and Gecko set `isComposing` instead. So a screen
 *   that submits on `key === "Enter"` alone would grade ないと while the learner was
 *   still writing ないといけない, record the miss, and lock the field.
 *   [`isComposingKey`] checks both signs, and [`trackComposition`] adds a third that
 *   depends on neither: it watches the element's own composition events and keeps
 *   the field marked as composing until the task that ended the composition is over,
 *   which is when WebKit's late keydown arrives.
 * * **full-width letters.** An IME in full-width alphanumeric mode types `ｓｈｉ`,
 *   which Rust's ASCII-only lower-casing does not read as `shi`. [`normaliseTyped`]
 *   folds it with NFKC before it is sent.
 * * **kana in a box that wants romaji.** With the IME on, `shi` becomes し before
 *   the learner sees it, and the kana reading box asks for the *sound* of a kana —
 *   typing the kana itself back answers nothing. [`romajiOnlyNote`] says so, rather
 *   than letting the box report "not a reading".
 *
 * Kept here, apart from the screens, so that `vitest` can run every rule without a
 * window — the same split as `speech.ts` and `words.ts`.
 */

/** The shape of a keydown this module needs, so a test needs no DOM. */
export interface ImeKeyLike {
  key: string;
  isComposing?: boolean;
  /**
   * Deprecated in the DOM, and still the only sign WebKit gives for the Enter that
   * commits a candidate: 229 means "the input method has this key".
   */
  keyCode?: number;
  /** Where the press landed, looked up in the set [`trackComposition`] keeps. */
  target?: EventTarget | object | null;
}

/** The `keyCode` every engine reports for a keypress the IME has consumed. */
export const IME_KEY_CODE = 229;

/**
 * The elements a composition is open in, or has only just closed in.
 *
 * A `WeakSet` rather than a flag per screen, so that one action serves every field
 * and a field that leaves the page takes its entry with it.
 */
const composing = new WeakSet<object>();

/**
 * Whether a keypress belongs to the input method rather than to the page.
 *
 * True while a composition is open (`isComposing`, the standard sign), for the key
 * WebKit hands over after the composition has closed (`keyCode` 229), and for any key
 * that lands on a field [`trackComposition`] still marks as composing.
 */
export function isComposingKey(event: ImeKeyLike): boolean {
  if (event.isComposing) return true;
  if (event.keyCode === IME_KEY_CODE) return true;
  const target = event.target;
  return typeof target === "object" && target !== null && composing.has(target);
}

/**
 * Whether a keypress is the learner pressing Enter to submit: Enter, and not the
 * Enter that commits an IME candidate.
 */
export function isSubmitEnter(event: ImeKeyLike): boolean {
  return event.key === "Enter" && !isComposingKey(event);
}

/**
 * Mark a field as composing from `compositionstart` until just after
 * `compositionend`, for [`isComposingKey`] to read.
 *
 * The "just after" is the point. WebKit fires `compositionend` and only then the
 * keydown for the Enter that ended it, so clearing the mark on `compositionend`
 * itself would clear it one event too early. It is cleared on a zero-delay timer
 * instead: a later task than the one the composition ended in, and still long before
 * a learner can press Enter a second time on purpose. A composition that starts again
 * before the timer runs cancels it.
 *
 * A Svelte action (`use:trackComposition`), typed against `EventTarget` so that a
 * test can hand it the bare one Node provides.
 */
export function trackComposition(node: EventTarget): { destroy(): void } {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const start = () => {
    clearTimeout(timer);
    composing.add(node);
  };
  const end = () => {
    clearTimeout(timer);
    timer = setTimeout(() => composing.delete(node), 0);
  };
  node.addEventListener("compositionstart", start);
  node.addEventListener("compositionend", end);
  return {
    destroy() {
      clearTimeout(timer);
      node.removeEventListener("compositionstart", start);
      node.removeEventListener("compositionend", end);
      composing.delete(node);
    },
  };
}

/**
 * A typed answer as the grader should see it: full-width letters, digits and spaces
 * folded to their ordinary forms, and half-width katakana to full-width.
 *
 * NFKC does both, and it composes rather than decomposes, so が and ぱ come out as the
 * single characters they went in as, and か followed by the combining dakuten becomes
 * が. One case it gets wrong for this purpose, and it is folded first: the *spacing*
 * marks ゛ and ゜ (U+309B, U+309C), which some IMEs and some pasted text use after a
 * kana. NFKC maps each to a space followed by the combining mark, which would leave
 * か゛ as `か ゙` — two characters and a space, and no longer a kana with a voiced
 * mark. Written after a kana, each is turned into its combining twin before NFKC
 * runs, and NFKC then composes it into the voiced kana it was meant to be.
 *
 * Only for answers that are graded as a reading. A free sentence of the learner's own
 * is not passed through this, because NFKC would also turn its full-width `？` and
 * `！` into ASCII, and the sentence is kept as it was written.
 */
export function normaliseTyped(typed: string): string {
  return typed
    .replace(/([ぁ-ゖァ-ヺ])゛/g, "$1゙")
    .replace(/([ぁ-ゖァ-ヺ])゜/g, "$1゚")
    .normalize("NFKC");
}

/**
 * Japanese script — kana of either kind, or kanji — anywhere in a string.
 *
 * Kanji as well as kana, because an IME that has converted し may have gone on to
 * 詩, and that is just as far from the romaji the box wants.
 */
const JAPANESE_SCRIPT = /[\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Han}ー]/u;

/** Whether the string holds any Japanese script. */
export function hasJapaneseScript(text: string): boolean {
  return JAPANESE_SCRIPT.test(text);
}

/**
 * What to tell a learner who typed Japanese script into a box that wants romaji, or
 * `null` when what they typed is not that.
 *
 * Said instead of a verdict, and not recorded as one: the learner has not answered
 * the question wrongly, they have answered it with the IME switched on.
 */
export function romajiOnlyNote(typed: string): string | null {
  if (!hasJapaneseScript(normaliseTyped(typed))) return null;
  return "Type the sound in romaji, not in kana — switch the Japanese input off.";
}
