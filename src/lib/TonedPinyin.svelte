<script lang="ts">
  /**
   * A pinyin reading, one span to a syllable, so the reading can carry the same
   * colour as the characters it belongs to.
   *
   * ## The split is Rust's
   *
   * `syllables` is the reading divided one syllable per character by
   * `hanzi_core::pinyin`, which is where the rule lives and stays. This component
   * only *reads* that division — it never re-splits a reading, because a second
   * copy of the rule in TypeScript is exactly the drift the Rust one exists to
   * prevent. 的话 is `dehuà`, a neutral syllable followed by a full one, and it is
   * the split that says so rather than the marks.
   *
   * With no split — a phrase's reading, a reading Rust could not align — the whole
   * string is offered to `toneFromSyllable`, which answers for a single syllable
   * and refuses a run of several rather than colouring half a word from one mark.
   *
   * ## Colour, not a chip
   *
   * A pinyin syllable is coloured but not given the pale background a character
   * gets: the chip is the memory aid, and repeating it under every syllable would
   * be noise around the reading rather than a second cue. See the `.tone-pinyin`
   * rules in `app.css`.
   */
  import { syllableTonesOf } from "./tones.svelte";

  interface Props {
    /** The reading to write. One syllable, or several run together. */
    text: string;
    /**
     * The reading split one syllable per character, from Rust. Empty when the
     * reading could not be aligned, which is a state this paints honestly rather
     * than hides.
     */
    syllables?: readonly string[] | null;
    /** One tone per character the reading belongs to, aligned to `syllables`. */
    tones?: readonly (number | null)[] | null;
    /** What to put between syllables. The default keeps a word run together. */
    separator?: string;
  }

  let { text, syllables = null, tones = null, separator = "" }: Props = $props();

  const parts = $derived.by(() => {
    const split = syllables ?? [];
    if (split.length > 0) {
      const list = syllableTonesOf(text, split, tones);
      return split.map((syllable, index) => ({ text: syllable, tone: list[index] ?? null }));
    }
    return [{ text, tone: syllableTonesOf(text, null, tones)[0] ?? null }];
  });
</script>

<!-- Adjacent on purpose, so a word's reading keeps the spacing it had. -->
{#each parts as part, index (index)}{#if index > 0}{separator}{/if}<span class="tone-char tone-pinyin" data-tone={part.tone ?? undefined}>{part.text}</span>{/each}
