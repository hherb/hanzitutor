<script lang="ts">
  /**
   * A run of Chinese characters, one span to a character.
   *
   * ## Why one span each
   *
   * A tone is a property of a *character*, so the colour has to be able to reach
   * a character, and there is no way to colour part of a text node. Splitting the
   * run is therefore the whole job of this component: it writes the same
   * characters, adjacent and unseparated, and each carries a `data-tone`
   * attribute that `app.css` turns into a colour — and only while the learner has
   * asked for it, since every rule is scoped under `.tone-colours` on the app
   * root. With the switch off the spans are invisible and the text is exactly
   * what it was.
   *
   * ## Where the tone comes from
   *
   * `tones`, when the caller has it: one entry per character, aligned by Rust
   * from the reading the text is actually shown with — a word's own reading, a
   * vocabulary entry's. That is the accurate answer for a polyphone, whose tone
   * inside a word is not always its tone alone.
   *
   * Where it does not, `tones.ts` falls back to the table of each character's own
   * tone, sent once from Rust. A character neither knows is written with no
   * `data-tone` at all, so it is left in the ordinary ink rather than given a
   * colour that would be a guess.
   *
   * See `src/lib/tones.ts` for both, and for why splitting a reading is *not*
   * done anywhere in the interface.
   */
  import { characterTonesOf } from "./tones.svelte";

  interface Props {
    /** The characters to write, in order. */
    text: string;
    /**
     * One tone per character, aligned to `text` — a word's `tones`, an entry's.
     * Absent, empty or short, the per-character table answers for whatever it
     * covers. A `null` entry is a position with no tone of its own, which falls
     * back the same way. Never a reading of a different length: Rust refuses to
     * pair one up.
     */
    tones?: readonly (number | null)[] | null;
  }

  let { text, tones = null }: Props = $props();

  const parts = $derived.by(() => {
    const list = characterTonesOf(text, tones);
    return [...text].map((ch, index) => ({ ch, tone: list[index] ?? null }));
  });
</script>

<!--
  One line on purpose: the spans have to be adjacent with nothing between them,
  or the component would put whitespace into the middle of a word.
-->
{#each parts as part, index (index)}<span class="tone-char" data-tone={part.tone ?? undefined}>{part.ch}</span>{/each}
