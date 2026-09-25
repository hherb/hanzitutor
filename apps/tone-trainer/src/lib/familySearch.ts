/**
 * Finding a word family, which is the Words half of the one search box.
 *
 * This is the testable half of the tone trainer's search. It lives in a module of
 * its own rather than inside `App.svelte` so that the rule can be pinned without a
 * browser: the bug it exists to prevent was a pure rule, and a rule that can only
 * be checked by typing into the running app is a rule that comes back.
 *
 * The predicate is deliberately about what a family **is** before what it
 * contains. A family is a syllable's minimal-pair set plus the words built on one
 * of its characters, so the syllable is the thing a learner is searching for when
 * they type a reading — and matching the words instead is how a query for `z`
 * came back with the 心 family, whose only `z` was the second syllable of 心脏病.
 */

import type { WordSet } from "./types";

/** Fold a query for comparison: tone marks gone, `v` and `u` the same letter. */
export function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/v/g, "u");
}

/**
 * Word families matching the query, best match first.
 *
 * A family is searched on what it **is** before anything inside it:
 *
 * 1. its own syllable (`base`) — the same rule the Characters list applies to a
 *    tone set's base, so `zhong` reaches 中 and `z` reaches every family whose
 *    syllable begins with `z`, exactly as the other list does;
 * 2. its key character, typed exactly;
 * 3. a word in it, typed as characters (`你好`) — so a word in hand finds the
 *    family it belongs to;
 * 4. a word's reading, but only for a query of two or more letters.
 *
 * Rule 4's length gate is the fix for the wrong results this list used to give: a
 * single letter sits inside almost every long reading, so matching one pulled in
 * families with nothing to do with the query — `z` returned 心 through 心脏病,
 * where the only `z` is the second syllable of one word among ninety-eight. One
 * letter is a syllable seed, and a family's own syllable is where that belongs.
 *
 * The English definition is not searched at all. The placeholder offers a word, a
 * character and a reading, and those are the three things a learner has in hand; a
 * definition put a family in the list for a coincidence in its translation and
 * never for anything about the sound.
 *
 * Folded the same way the tone sets are, so tone marks and `ü` are not things a
 * learner has to type. Within one rank the list's own usefulness order stands —
 * `Array.prototype.sort` is stable, so this does not need a second key.
 */
export function searchFamilies(families: WordSet[], query: string): WordSet[] {
  const text = query.trim();
  if (text === "") return families;

  const folded = fold(text);
  const ranked: { family: WordSet; rank: number }[] = [];
  for (const family of families) {
    let rank: number | null = null;
    if (family.key === text || fold(family.base) === folded) rank = 0;
    else if (fold(family.base).includes(folded)) rank = 1;
    else if (family.words.some((word) => word.text.includes(text))) rank = 2;
    else if (
      folded.length >= 2 &&
      family.words.some((word) => fold(word.reading).includes(folded))
    )
      rank = 3;
    if (rank !== null) ranked.push({ family, rank });
  }

  ranked.sort((a, b) => a.rank - b.rank);
  return ranked.map((hit) => hit.family);
}
