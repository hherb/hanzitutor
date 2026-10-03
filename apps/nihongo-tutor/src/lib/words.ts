/**
 * The vocabulary's arithmetic, as pure functions.
 *
 * The same split the board uses: what can be reasoned about without a window
 * lives here and is unit-tested by the root project's vitest, and the components
 * in `*.svelte` only draw.
 *
 * Two of these mirror rules the Rust side owns. `foldKana` is the same katakana →
 * hiragana fold `nihongo_core::normalise_to_hiragana` applies, and it is repeated
 * rather than shipped over the IPC because a reading is compared on every
 * keystroke. `pageWindow` mirrors the clamp `words_in_band` applies to `limit`, so
 * the screen asks for a page the command will actually serve.
 */

import type { PassageToken, Ruby, Word } from "./types";

/** The largest page the command will serve. Mirrors `MAX_WORD_PAGE` in Rust. */
export const MAX_WORD_PAGE = 200;

/** Whether a furigana segment has a reading to draw over it. */
export function needsRuby(segment: Ruby): boolean {
  return typeof segment.rt === "string" && segment.rt.length > 0;
}

/**
 * The word's reading as its furigana spells it.
 *
 * Falls back to the word's stored reading when there is no furigana: the small
 * number of words JmdictFurigana does not align still have their own reading, and
 * losing it because the ruby is missing would be worse than showing no ruby.
 */
export function readingOf(word: Word): string {
  if (word.furigana.length === 0) return word.reading;
  return word.furigana.map((segment) => segment.rt ?? segment.ruby).join("");
}

/**
 * Fold katakana to hiragana, leaving everything else alone.
 *
 * The katakana block sits exactly `0x60` above the hiragana one, which is the
 * same offset `nihongo_core` uses for script conversion. `ー` is outside both
 * ranges and is deliberately left as it is: it is a length mark that belongs to
 * either script.
 */
export function foldKana(input: string): string {
  let out = "";
  for (const ch of input) {
    const code = ch.codePointAt(0) ?? 0;
    out += code >= 0x30a1 && code <= 0x30f6 ? String.fromCodePoint(code - 0x60) : ch;
  }
  return out;
}

/**
 * Whether the furigana spells the word's reading.
 *
 * Compared after folding the kana type, and that is not laxness: two words in the
 * shipped vocabulary spell their reading in the other kana type — 生ゴミ is read
 * なまごみ but written with a katakana ゴミ — and comparing the raw strings would
 * call the display and the reading both wrong.
 */
export function furiganaSpellsReading(word: Word): boolean {
  if (word.furigana.length === 0) return false;
  return foldKana(readingOf(word)) === foldKana(word.reading);
}

/**
 * Whether a word's furigana covers exactly the characters of the word.
 *
 * A segment list that does not add up to the word is a misalignment, and drawing
 * it puts readings over the wrong characters — which is worse than drawing none.
 */
export function furiganaCoversWord(word: Word): boolean {
  if (word.furigana.length === 0) return true;
  return word.furigana.map((segment) => segment.ruby).join("") === word.text;
}

/** One page of a list, clamped the way the Rust command clamps it. */
export interface Window {
  offset: number;
  limit: number;
  page: number;
  pages: number;
}

export function pageWindow(total: number, pageSize: number, page: number): Window {
  const size = Math.min(Math.max(Math.trunc(pageSize) || 1, 1), MAX_WORD_PAGE);
  const pages = Math.max(Math.ceil(total / size), 1);
  const clamped = Math.min(Math.max(Math.trunc(page) || 1, 1), pages);
  return { offset: (clamped - 1) * size, limit: size, page: clamped, pages };
}

/** The reading the learner would have to type for a token, for a hint. */
export function tokenReading(token: PassageToken): string {
  return token.rt ?? token.surface;
}

/** Whether a passage token can be tapped to open a card. */
export function tokenIsTappable(token: PassageToken): boolean {
  return typeof token.word === "string" && token.word.length > 0;
}
