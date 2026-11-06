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
 *   depends on neither, for a keyboard that sends the commit as an ordinary 13: it
 *   watches the field's own composition events, and an Enter that arrives within a
 *   few milliseconds of the composition ending is taken as the one that ended it.
 * * **full-width letters and typographic apostrophes.** An IME in full-width
 *   alphanumeric mode types `ｓｈｉ`, which Rust's ASCII-only lower-casing does not
 *   read as `shi`, and iOS's Smart Punctuation turns the `'` of `kin'you` into `’`,
 *   which the input table does not know. [`normaliseTyped`] folds both before a
 *   reading is sent, and [`foldForSentence`] folds the same letters — and nothing
 *   else — in a sentence of the learner's own.
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
  /** Where the press landed, looked up in what [`trackComposition`] keeps. */
  target?: EventTarget | object | null;
  /** When the press happened, compared with when the field's composition ended. */
  timeStamp?: number;
}

/** The `keyCode` every engine reports for a keypress the IME has consumed. */
export const IME_KEY_CODE = 229;

/**
 * How close to the end of a composition, in milliseconds, an Enter is still the one
 * that ended it.
 *
 * Measured either way: WebKit may stamp the keydown with the time of the native key
 * press, which comes *before* the `compositionend` it caused is created. A learner's
 * own second Enter is a deliberate press, and comes far later than this.
 */
export const COMPOSITION_END_WINDOW_MS = 50;

/**
 * The fields a composition is open in.
 *
 * A `WeakSet` rather than a flag per screen, so that one action serves every field
 * and a field that leaves the page takes its entry with it.
 */
const composing = new WeakSet<object>();

/** When each field's last composition ended, by the event's own `timeStamp`. */
const endedAt = new WeakMap<object, number>();

/**
 * Whether a keypress belongs to the input method rather than to the page.
 *
 * True while a composition is open (`isComposing`, the standard sign), for the key
 * WebKit hands over after the composition has closed (`keyCode` 229), for any key
 * that lands on a field [`trackComposition`] marks as composing, and for a key whose
 * `timeStamp` is within [`COMPOSITION_END_WINDOW_MS`] of that field's last
 * `compositionend`.
 */
export function isComposingKey(event: ImeKeyLike): boolean {
  if (event.isComposing) return true;
  if (event.keyCode === IME_KEY_CODE) return true;
  const target = event.target;
  if (typeof target !== "object" || target === null) return false;
  if (composing.has(target)) return true;
  const ended = endedAt.get(target);
  return (
    ended !== undefined &&
    typeof event.timeStamp === "number" &&
    Math.abs(event.timeStamp - ended) <= COMPOSITION_END_WINDOW_MS
  );
}

/**
 * Whether a keypress is the learner pressing Enter to submit: Enter, and not the
 * Enter that commits an IME candidate.
 */
export function isSubmitEnter(event: ImeKeyLike): boolean {
  return event.key === "Enter" && !isComposingKey(event);
}

/**
 * Watch a field's composition for [`isComposingKey`]: marked as composing from
 * `compositionstart` to `compositionend`, and the time of the end kept.
 *
 * The time is what catches the Enter that ended the composition when that Enter
 * carries neither `isComposing` nor 229. WebKit fires `compositionend` first and the
 * keydown after it, and does not promise that the two arrive in one task, so the
 * rule is a comparison of the two events' own timestamps rather than an assumption
 * about task order. It is a fallback: the two DOM signs are checked first, and they
 * are what the macOS webview is known to send.
 *
 * Leaving the field forgets both, so a composition whose end never arrived cannot
 * leave Enter dead in it.
 *
 * A Svelte action (`use:trackComposition`), typed against `EventTarget` so that a
 * test can hand it the bare one Node provides.
 */
export function trackComposition(node: EventTarget): { destroy(): void } {
  const start = () => {
    composing.add(node);
    endedAt.delete(node);
  };
  const end = (event: Event) => {
    composing.delete(node);
    endedAt.set(node, event.timeStamp);
  };
  const forget = () => {
    composing.delete(node);
    endedAt.delete(node);
  };
  node.addEventListener("compositionstart", start);
  node.addEventListener("compositionend", end);
  node.addEventListener("focusout", forget);
  return {
    destroy() {
      node.removeEventListener("compositionstart", start);
      node.removeEventListener("compositionend", end);
      node.removeEventListener("focusout", forget);
      forget();
    },
  };
}

/**
 * The typographic apostrophes and the prime, which iOS's Smart Punctuation (and a
 * pasted word) put where a learner typed `'`. NFKC leaves all three alone, and the
 * input table's `n'` is spelled with the ASCII one.
 */
const APOSTROPHES = /[\u2018\u2019\u2032]/g;

/** Full-width ASCII letters and digits, which sit exactly 0xFEE0 above their own. */
const FULL_WIDTH_ALPHANUMERIC = /[\uFF10-\uFF19\uFF21-\uFF3A\uFF41-\uFF5A]/g;

/** A run of half-width katakana, with its half-width marks and punctuation. */
const HALF_WIDTH_KANA_RUN = /[\uFF61-\uFF9F]+/g;

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
 * The typographic apostrophes `’ ‘ ′` become `'` first: NFKC leaves them alone, and
 * `kin’you` is how iOS writes `kin'you`.
 *
 * Only for answers that are graded as a reading. A free sentence of the learner's own
 * goes through [`foldForSentence`] instead, because NFKC would also turn its
 * full-width `？` and `！` into ASCII, and the sentence is kept as it was written.
 */
export function normaliseTyped(typed: string): string {
  return typed
    .replace(APOSTROPHES, "'")
    .replace(/([\u3041-\u3096\u30A1-\u30FA])\u309B/g, "$1\u3099")
    .replace(/([\u3041-\u3096\u30A1-\u30FA])\u309C/g, "$1\u309A")
    .normalize("NFKC");
}

/**
 * A sentence of the learner's own as the reader in Rust should see it: full-width
 * letters and digits folded to ASCII, typographic apostrophes to `'`, half-width
 * katakana to full-width — and **nothing else** changed.
 *
 * Narrower than [`normaliseTyped`] on purpose. The sentence is kept as it was
 * written, so its `？`, `！`, `～` and ideographic spaces stay as typed; what is
 * folded is only what would otherwise stop the sentence being read at all, which is
 * romaji typed in full width (`ｉｎｕ ｎｏ ほうが`) and katakana typed in half width.
 * The half-width runs are put through NFKC on their own, which is what joins ｶ and
 * its separate ﾞ into ガ.
 */
export function foldForSentence(typed: string): string {
  return typed
    .replace(APOSTROPHES, "'")
    .replace(FULL_WIDTH_ALPHANUMERIC, (ch) =>
      String.fromCharCode(ch.charCodeAt(0) - 0xfee0),
    )
    .replace(HALF_WIDTH_KANA_RUN, (run) => run.normalize("NFKC"));
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
