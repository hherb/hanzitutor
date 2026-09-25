<script lang="ts">
  /**
   * The readings a character is listed with — `hǎo · hào` — each coloured by its
   * own tone.
   *
   * A character's readings arrive as several strings, and each one *is* a
   * syllable, so each one's tone is read off its own mark rather than paired by
   * Rust: there is nothing to split and nothing to align. That is the one case
   * `toneFromSyllable` answers on its own, and it is why this needs no `tones`
   * prop — see `src/lib/tones.ts`.
   *
   * It exists as a component because seven screens show a character's readings this
   * way, and the separator, the colouring and the "no reading recorded" dash are
   * one decision each.
   */
  import TonedPinyin from "./TonedPinyin.svelte";

  interface Props {
    /** Every reading the dataset knows, most common first. */
    readings: readonly string[];
    /** What to put between them. Two spaces and a middot, as it always was. */
    separator?: string;
    /** What to show when there are none. */
    empty?: string;
  }

  let { readings, separator = "  ·  ", empty = "—" }: Props = $props();
</script>

{#each readings as reading, index (index)}{index > 0 ? separator : ""}<TonedPinyin text={reading} />{/each}{#if readings.length === 0}{empty}{/if}
