/**
 * The mnemonic set, looked up by character.
 *
 * The records are authored in `data/mnemonics/zh-<level>.source.tsv` and built
 * by `scripts/build-mnemonics.py` into `data/mnemonics/mnemonics.json`, which
 * vite bundles. Keeping the lookup here rather than in the dataset artifact
 * means the prose can be corrected and rebuilt without regenerating a 13 MB
 * artifact or bumping its format version — mnemonics are the one part of a
 * character that is editorial rather than measured.
 *
 * A mnemonic is a memory hook, not an etymology and not a pronunciation guide.
 * `sound` is an English approximation that only has to be close enough to call
 * the character back; the readings on the card are the authoritative sound.
 */
import raw from "../../data/mnemonics/mnemonics.json";

export interface Mnemonic {
  /** Which HSK level the record was authored for, e.g. `"hsk1"`. */
  level: string;
  /** The English sound hook; `""` when there is no honest one. */
  sound: string;
  /** The story, one or two sentences. */
  mnemonic: string;
}

const byChar = new Map<string, Mnemonic>(Object.entries(raw as Record<string, Mnemonic>));

/** How many characters carry a mnemonic; shown on the Licences screen footer. */
export const MNEMONIC_COUNT = byChar.size;

/** The mnemonic for a character, or `undefined` when the set does not cover it. */
export function mnemonicFor(ch: string): Mnemonic | undefined {
  return byChar.get(ch);
}

/**
 * The caveat that has to appear wherever a sound hook is shown.
 *
 * English spellings and Chinese readings disagree, and a hook like "way" for
 * 为 is only a memory jog. Presenting it as the pronunciation would teach the
 * wrong thing, so the label travels with the hook rather than living in a
 * help page nobody opens.
 */
export const SOUND_NOTE =
  "The sound hint is an English memory hook, not a pronunciation guide — read the pinyin for the actual sound.";
