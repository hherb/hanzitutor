/**
 * How a recognition's transcription is shown beside the tone judgement.
 *
 * **Copied from the full app's `src/lib/transcript.ts`.** It is the second piece
 * of interface this app duplicates on purpose — `ToneChart.svelte` is the first.
 * Both apps recognise with the same model and compare with the same rule, so they
 * must show the same thing; `HANDOVER.md` §10 records why this is a copy rather
 * than a shared package, and what would change that.
 *
 * ## The one rule
 *
 * **Within one syllable a character carries no information beyond its reading.**
 * 是 and 事 are the same sound, so a model that writes 事 where 是 was asked for has
 * misread *nothing* — its language model simply prefers the commoner character,
 * which is what a language model is for. Where the model heard the sound the
 * exercise asked for, the panel therefore shows the character the exercise asked
 * for: that asserts only what was actually measured (the sound matched) instead of
 * repeating a spelling guess that was never tested.
 *
 * **Where the sounds differ, the model's character is shown unchanged.** 四 heard
 * for 是 is evidence, and it is the whole reason this block exists; substituting
 * there would erase it.
 *
 * ## The reading, and where a tone mark is allowed
 *
 * The transcription's letters are shown beside its characters, and **the tone
 * marks appear whenever the model's transcription is not the text that was asked
 * for**. Then the learner is being shown what was actually recognised, tonality
 * included: `cóng` identifies 从 where a bare `cong` is equally 葱 and 匆, and a
 * syllable whose *sound* matched can still have been heard at the wrong tone,
 * which is precisely the difference a tone drill is about. Every mark is the
 * dictionary's reading of the character the model wrote — a fact about the
 * transcription, not a measurement of the learner's pitch. Where that tone differs
 * from the one asked for, the model's own character is shown with it: the target's
 * character stands in only where the model agreed on the sound *and* the tone, so
 * a character and the reading under it can never contradict each other.
 *
 * Where the transcription **is** the text that was asked for, nothing was
 * misheard and the letters stay plain.
 *
 * This is presentation only. The tone comes from the pitch contour in
 * `hanzi_core::tone`, and the recogniser never sees the target — nothing here can
 * change a score.
 */

import type { Heard, HeardSyllable } from "./types";

/** The tone names, in the words the scorer uses. */
export const TONE_NAME: Record<number, string> = {
  1: "high level",
  2: "rising",
  3: "dipping",
  4: "falling",
  5: "neutral",
};

export interface TranscriptDisplay {
  /** What to render as the transcription's characters. */
  text: string;
  /**
   * What to render as the transcription's reading.
   *
   * Where the model's transcription **differs from the text that was asked for**,
   * every syllable carries the model's own dictionary tone mark — the learner is
   * being shown what was actually recognised, tonality included, and that is the
   * whole reason the mark is allowed at all. Where the transcription is the text
   * that was asked for, nothing was misheard and the letters stay plain.
   *
   * Empty with the text when the model heard nothing. Falls back to the plain
   * reading — `Heard.base` — when the two halves cannot be lined up, because with
   * no comparison there is no per-syllable reading to show.
   */
  reading: string;
  /**
   * True when the transcription is not the text that was asked for, character for
   * character. This is what decides whether the reading carries tone marks.
   */
  differs: boolean;
  /**
   * One entry per transcription syllable: the character the exercise asked for
   * where the sound matched **and the model's reading agreed down to the tone**,
   * `null` where it did not.
   *
   * Empty when the two halves cannot be lined up.
   */
  characters: (string | null)[];
  /** True when at least one character shown is the target's rather than the model's. */
  substituted: boolean;
}

/**
 * Decide what to show for the transcription's characters.
 *
 * `targetCharacters` is one per syllable of the target, in order — the same list
 * the charts are drawn from, which is what makes the pairing meaningful.
 *
 * The substitution is gated on four things, and every one of them is load-bearing:
 *
 * - `sameCount`, because the model's reading could not be divided one syllable per
 *   character otherwise, and the comparison itself is not made in that case;
 * - equal lengths on both sides, so index `i` means the same syllable everywhere;
 * - `syllable.matches`, which is the sound comparison;
 * - `reading` and `wantedReading` agreeing, tone marks and all. Where the model
 *   put a **different tone** on the syllable, its own character stands: printing
 *   the target's character above the model's tone would contradict itself, and it
 *   would hide the one difference a tone drill exists to find.
 *
 * When the model's text cannot be split to match, the model's text is returned
 * whole. Characters are never dropped to force the lengths to agree: a shortened
 * transcription would be a different claim from the one the model made.
 */
export function transcriptDisplay(
  heard: Heard | null,
  targetCharacters: string[],
): TranscriptDisplay {
  if (heard === null) {
    return { text: "", reading: "", differs: false, characters: [], substituted: false };
  }

  const model = [...heard.text];
  const aligned =
    heard.sameCount &&
    heard.syllables.length === targetCharacters.length &&
    model.length === heard.syllables.length;
  // No pairing, so no comparison and no syllable to mark: the model's own text and
  // its plain reading, whole.
  if (!aligned) {
    return {
      text: heard.text,
      reading: heard.base,
      differs: false,
      characters: [],
      substituted: false,
    };
  }

  // Whether the model wrote something other than what was asked for. When it did,
  // the reading carries every syllable's dictionary tone, because the point is to
  // show what was actually recognised, tones included — a syllable whose *sound*
  // matched can still have been heard at the wrong tone, and that difference is
  // exactly what a tone drill is looking for.
  const differs = model.some((ch, index) => ch !== targetCharacters[index]);
  // The target's character may stand in for the model's only when the two agree on
  // the sound *and* the tone. Where the tone differs, the model's own character and
  // reading go together — see the note on `characters` above.
  const agrees = (syllable: HeardSyllable) =>
    syllable.reading.toLowerCase() === syllable.wantedReading.toLowerCase();
  const characters = heard.syllables.map((syllable, index) =>
    syllable.matches && agrees(syllable) ? (targetCharacters[index] ?? null) : null,
  );
  const shown = model.map((ch, index) => characters[index] ?? ch);
  const reading = heard.syllables
    .map((syllable) => (differs ? syllable.reading || syllable.base : syllable.base))
    .join(" ");

  return {
    text: shown.join(""),
    reading,
    differs,
    characters,
    // Asked of the *syllables*, not of the rendered characters. Comparing the two
    // strings would miss a substitution whenever the model's character for a
    // matched sound happened to equal the target's anyway — and, worse, would
    // report a substitution for a syllable whose sound did **not** match but whose
    // character was written the same by coincidence. Neither is a homophone this
    // rule acted on, and only the syllable comparison knows the difference.
    substituted: heard.syllables.some(
      (syllable, index) =>
        syllable.matches && agrees(syllable) && model[index] !== characters[index],
    ),
  };
}
