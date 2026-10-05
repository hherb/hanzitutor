/**
 * The Phrases screen's arithmetic, as pure functions.
 *
 * The same split the rest of the app uses: what can be reasoned about without a
 * window lives here and is unit-tested by vitest, and the component only draws.
 *
 * The first of these is a real rule rather than a formatting convenience. The
 * corpus is filtered, not authored, so **nothing guarantees that band 1 holds
 * anything**: a band is the band of a phrase's hardest word, and whether any
 * imported sentence has nothing harder than a kyōiku-1 word is a fact about the
 * corpus that a given export can change. A screen that opened on band 1 regardless
 * would therefore open on an empty list the day the filter got stricter, and say
 * nothing about why.
 */

import type { PhraseBandView, PhraseView } from "./types";

/**
 * The band to open on: the first the corpus actually filled.
 *
 * `null` when no band holds a phrase at all, which is a state the screen has to say
 * in words rather than draw an empty list for — an empty phrase corpus means the
 * artifact was built with a filter that kept nothing.
 */
export function firstNonEmptyBand(bands: PhraseBandView[]): number | null {
  const found = bands.find((entry) => entry.phrases > 0);
  return found ? found.band : null;
}

/** How many phrases the whole corpus holds, for the header. */
export function totalPhrases(bands: PhraseBandView[]): number {
  return bands.reduce((sum, entry) => sum + entry.phrases, 0);
}

/**
 * Where one phrase came from, in the words the licence asks for.
 *
 * Tatoeba's sentence id, the contributor and the licence: CC BY 2.0 FR requires the
 * author *and* the licence to be named, and the id is what makes the sentence
 * findable, so all three travel with the phrase on screen and not only in the
 * licence file. The corpus is named once in the panel's own header, where it
 * applies to every row.
 */
export function phraseSource(phrase: PhraseView): string {
  return `Tatoeba #${phrase.id} · ${phrase.author} · ${phrase.licence}`;
}
