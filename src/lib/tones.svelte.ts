/**
 * Colouring a character, or a syllable of pinyin, by the tone it is read with.
 *
 * ## Why this is here rather than in a component
 *
 * The rules a colour needs are arithmetic over strings — which tone a written
 * syllable carries, which tone a character of a word has, which characters have
 * no tone at all — and arithmetic is the kind of thing this project pins with a
 * test rather than with a look at the running app. `tones.test.ts` exercises
 * everything in this file; the components above it only paint what it returns.
 *
 * ## The one rule that is deliberately *not* here
 *
 * **Splitting a reading into syllables lives in Rust** (`hanzi_core::pinyin`),
 * next to the code that scores those syllables, and it is not reimplemented
 * here. A word's reading arrives already divided — `Word.syllables`,
 * `VocabEntry.syllables` — and this file only *reads* that division. What it
 * does do is read the tone mark off a **single** syllable (`toneFromSyllable`),
 * which needs no splitting at all: `hǎo` is one syllable and the `ǎ` says which
 * tone. A string holding several syllables is refused rather than guessed at,
 * which is why that function returns `null` for `xuéxí` — the caller that has a
 * word has the split from Rust and does not need to guess.
 *
 * ## The two sources of a tone
 *
 * - **A whole table**, character → its own tone, sent once from Rust
 *   (`character_tones`) and folded in by [`setCharacterTones`]. This answers for
 *   a character shown on its own — a lesson list, a radical's family, a search
 *   result — where there is no reading to read it against.
 * - **The reading of the text it is part of**, when the caller has one: a word's
 *   `tones`, aligned one entry per character by Rust. That is the accurate answer
 *   for a polyphone, whose tone inside a word is not always its tone alone.
 *
 * [`characterTones`] uses the second when it fits and the first otherwise, so a
 * caller that has neither still gets whatever the table knows.
 *
 * ## Why the file ends in `.svelte.ts`
 *
 * Not for a component — there is none here — but because the table is `$state`,
 * which is what lets the colours appear when it arrives after the first paint.
 * See [`setCharacterTones`]. A plain `.ts` module would compile, run and be
 * *silently* unable to repaint anything.
 */

/**
 * A tone: the four full tones the course teaches, plus the **neutral** tone as
 * `5`.
 *
 * The neutral tone is spelled `5` rather than `0` or `"neutral"` because that is
 * what [`hanzi_core::pinyin::Syllable`] calls it and what the tone scorer
 * compares against, and two names for one thing is how they drift apart. It is
 * not a fifth *pitch*: a neutral syllable's height is set by the syllable before
 * it, and the colour says "this syllable has no tone of its own".
 */
export type Tone = 1 | 2 | 3 | 4 | 5;

/** Every tone, in the order the settings screen lists them. */
export const TONES: readonly Tone[] = [1, 2, 3, 4, 5];

/* What each tone is *called* is deliberately not here. The app already has one
   name for a tone — `TONE_NAME` in `transcript.ts`, in the scorer's own words:
   high level, rising, dipping, falling, neutral — and a second table here would
   be a second vocabulary for the same five things. A screen that needs to say
   which tone a colour means imports that one. */

/**
 * The tone each accented letter marks.
 *
 * The same table `hanzi_core::pinyin::marked_tone` holds, and it has to stay the
 * same table: a syllable marked one way in Rust and read another here would
 * colour a character with a tone it is not read with. `tones.test.ts` pins the
 * accented forms this understands, and the Rust side has a test of its own over
 * the same characters, so a mark added to one and not the other is a failure
 * rather than a wrong colour.
 *
 * The syllabic nasals (`ń`, `ň`, `ǹ`, `ḿ`) are here because 嗯 is `ń` — a
 * syllable with no vowel letter in it at all — and a table that skipped them
 * would leave one of the commonest words uncoloured.
 */
const MARKED_TONE: Record<string, Tone> = {
  ā: 1, ē: 1, ī: 1, ō: 1, ū: 1, ǖ: 1, ń: 1, ḿ: 1,
  á: 2, é: 2, í: 2, ó: 2, ú: 2, ǘ: 2,
  ǎ: 3, ě: 3, ǐ: 3, ǒ: 3, ǔ: 3, ǚ: 3, ň: 3,
  à: 4, è: 4, ì: 4, ò: 4, ù: 4, ǜ: 4, ǹ: 4,
};

/**
 * The letters that make a run a syllable rather than a stray consonant.
 *
 * Used for the one question a tone mark cannot answer on its own: a run with no
 * mark at all is the **neutral** tone if it has a vowel, and nothing at all if it
 * does not. `de` is neutral; `xy` is not a syllable.
 */
const VOWELS = "aeiouüvāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňǹḿê";

/**
 * The tone a **single written syllable** carries.
 *
 * `null` when the text is not one syllable: two marks in a run is a pair run
 * together (`xuéxí`), and reading one of them as *the* tone would put a colour on
 * a word from half its evidence. A reading of several syllables is a different
 * question, and one Rust has already answered wherever it mattered — see the
 * module note.
 *
 * A run with no mark at all is the neutral tone (5) when it has a vowel, so `de`
 * and `ma` are coloured rather than left out. A run with no vowel either — a
 * separator, punctuation, an empty string — has no tone and is `null`.
 */
export function toneFromSyllable(reading: string): Tone | null {
  const marks: Tone[] = [];
  let hasVowel = false;
  for (const ch of reading) {
    const marked = MARKED_TONE[ch];
    if (marked !== undefined) marks.push(marked);
    if (VOWELS.includes(ch)) hasVowel = true;
  }
  if (marks.length === 1) return marks[0];
  if (marks.length === 0) return hasVowel ? 5 : null;
  return null;
}

/**
 * The character → tone table, filled in once from the backend.
 *
 * A module-level map rather than a prop threaded through every panel: the same
 * glyph is drawn on a dozen screens, so passing it down would be a dozen copies
 * of one lookup. It is empty until [`setCharacterTones`] is called, and an empty
 * table simply means every character falls back to `null` — no colour — rather
 * than a wrong one.
 *
 * ## Why this file is `.svelte.ts`
 *
 * The table arrives **after** the first paint: it is one call among the startup
 * calls, and a character can be on screen before its answer is. A plain module
 * variable would leave those characters uncoloured for good, because assigning
 * to one invalidates nothing and the component that read it would never re-run —
 * the colours would simply never appear, and only on a slow launch, which is the
 * worst kind of bug to be told about. `$state` is what makes the read a
 * dependency: `TonedText` reads the table inside its own `$derived`, so filling
 * the table repaints exactly the glyphs that were waiting for it. Nothing else
 * about the module changes, and `tones.test.ts` still drives it directly.
 */
let characterTones = $state<Map<string, Tone>>(new Map());

/**
 * Fold the backend's whole table in: `[["的", 5], ["一", 1], …]`.
 *
 * Replaces whatever was there rather than merging, so a second call cannot leave
 * a stale tone behind for a character the new table does not name. A tone outside
 * `1..=5` is dropped rather than coerced: the table is Rust's answer, and a
 * number this file does not understand is a version mismatch to be silent about
 * rather than a colour to invent.
 */
export function setCharacterTones(pairs: readonly (readonly [string, number])[]): void {
  const next = new Map<string, Tone>();
  for (const [ch, tone] of pairs) {
    if (ch.length > 0 && tone >= 1 && tone <= 5) next.set(ch, tone as Tone);
  }
  characterTones = next;
}

/** How many characters the table knows, for the legend and for tests. */
export function characterToneCount(): number {
  return characterTones.size;
}

/**
 * The tone a character carries on its own, or `null` when the table does not
 * know it.
 *
 * A **lone** character: for one inside a word use [`characterTones`] with the
 * word's own tones, which is what a polyphone needs.
 */
export function toneOf(ch: string): Tone | null {
  return characterTones.get(ch) ?? null;
}

/** Is this number a tone this file understands? */
function asTone(value: number | null | undefined): Tone | null {
  return typeof value === "number" && value >= 1 && value <= 5 ? (value as Tone) : null;
}

/**
 * The tone of **each character** of `text`, in order.
 *
 * `tones` is the reading's own tones, one per character, as Rust pairs them — a
 * word's `tones`, a vocabulary entry's `tones`. When it covers a position that is
 * the answer, because the reading is in context: 重要 is `zhòngyào` where 重 on its
 * own is also `zhòng`, but 银行 is `yínháng` where 行 on its own is `xíng`, and only
 * the word knows which. Where it does not — a character with no reading, text
 * with no tones at all, a short array — the table answers, and where that does
 * not either the entry is `null` and nothing is painted.
 *
 * A caller must **not** pass a reading of a different length hoping it lines up:
 * Rust refuses to send one, and this trusts the pairing it is given per position
 * rather than trying to re-align it. An unaligned array is the caller's bug, and
 * the per-position fallback keeps it a wrong colour on one glyph rather than a
 * crash.
 */
export function characterTonesOf(
  text: string,
  tones?: readonly (number | null)[] | null,
): (Tone | null)[] {
  return [...text].map((ch, index) => asTone(tones?.[index]) ?? toneOf(ch));
}

/**
 * The tone of each syllable of a reading, for colouring the pinyin itself.
 *
 * `syllables` is the reading split one syllable per character by Rust; `tones`
 * is the tone of each of those characters. When the two line up the pinyin is
 * coloured syllable by syllable, so the reading and the glyphs beside it agree.
 *
 * When they do not — a phrase's reading, a reading Rust could not align — the
 * whole string is offered to [`toneFromSyllable`], which answers for a single
 * syllable and refuses a run of several rather than colouring half a word from
 * one mark it happened to find.
 */
export function syllableTonesOf(
  text: string,
  syllables?: readonly string[] | null,
  tones?: readonly (number | null)[] | null,
): (Tone | null)[] {
  const split = syllables ?? [];
  if (split.length > 0) {
    return split.map((_, index) => {
      const provided = asTone(tones?.[index]);
      if (provided !== null) return provided;
      // A syllable Rust split but did not tone — it cannot happen today, and a
      // mark on the syllable itself is the honest fallback if it ever does.
      return toneFromSyllable(split[index] ?? "");
    });
  }
  return [toneFromSyllable(text)];
}
