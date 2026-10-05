<script lang="ts">
  /**
   * The grammar stage: one pattern and everything the corpus says about it.
   *
   * ## What the card draws, and in what order
   *
   * The thing being studied comes first — the pattern, with its kana reading above
   * it when the pattern itself is written with a kanji, and its romaji under it —
   * then what it means, then the form it attaches to, then the sentences that show
   * it, then the note about nuance and the corpus's tags. That is invariant 33's
   * arrangement with no board in the middle: the shape of the thing above, the
   * explanation under it.
   *
   * ## Why it is a screen of its own, and why it has arrows
   *
   * The course screen is a list of 526 patterns and a search box; a card with three
   * sentences and a paragraph of nuance is not something to draw under a row of it.
   * The arrows step through the **course order**, which is the only sequence there
   * is here — a pattern has no band to be sorted into (`nihongo_core::grammar` has
   * the measurement that settled that), so the order the corpus teaches them in is
   * what "next" means. At the end of a page the panel fetches the next one; the
   * arrows are disabled only at the ends of the course.
   *
   * ## What is spoken
   *
   * Each **example sentence as written**, and never the pattern: a pattern like
   * `〜に対して` is not a sentence, and the placeholder `〜` is not something a
   * synthesiser should be asked to read. This is invariant 37's precedent applied
   * here — a whole sentence has no ambiguous reading to choose between, which is
   * why the sentence itself may be handed to the voice where a bare kanji may not.
   */
  import Icon from "./Icon.svelte";
  import SpeakButton from "./SpeakButton.svelte";
  import type { GrammarPoint } from "./types";
  import type { VoiceStatus } from "./speech";
  import { positionLabel } from "./grammar";
  import { needsRuby } from "./words";

  interface Props {
    /** The point this stage is about, as the course screen's row already holds it. */
    point: GrammarPoint;
    /** Where it sits in the course, counted from 1. */
    position: number;
    /** How many points the course holds, for the "12 of 526" line. */
    total: number;
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice: VoiceStatus;
    /** Back to the list. */
    onleave: () => void;
    /** Step to the previous or the next pattern in course order. */
    onstep: (direction: -1 | 1) => void;
    /** Whether each arrow can be followed — the course's ends, not the page's. */
    canPrev: boolean;
    canNext: boolean;
  }

  let { point, position, total, voice, onleave, onstep, canPrev, canNext }: Props =
    $props();
</script>

<section class="stage">
  <header class="head">
    <!-- Out of the card and back to the list, named for the list it returns to. -->
    <button class="back" onclick={onleave} title="Back to the grammar course">
      <Icon name="back" />
      <span>Grammar</span>
    </button>

    <span class="where">{positionLabel(position, total)}</span>

    <!-- The course order is the sequence, so these are the way on — there is no
         board here and nothing to grade. -->
    <span class="arrows">
      <button
        class="arrow"
        disabled={!canPrev}
        onclick={() => onstep(-1)}
        title="The previous pattern in this course"
        aria-label="Previous pattern"
      >
        <Icon name="back" />
      </button>
      <button
        class="arrow"
        disabled={!canNext}
        onclick={() => onstep(1)}
        title="The next pattern in this course"
        aria-label="Next pattern"
      >
        <Icon name="next" />
      </button>
    </span>
  </header>

  <div class="pattern" lang="ja">
    {#if point.reading}
      <span class="reading">{point.reading}</span>
    {/if}
    <span class="written">{point.pattern}</span>
    <span class="romaji">{point.romaji}</span>
  </div>

  <p class="meaning">{point.meaning}</p>

  <p class="formation"><span class="label">Written with</span> {point.formation}</p>

  <section class="examples" aria-label="Examples">
    {#each point.examples as example, i (i)}
      <article class="example">
        <p class="sentence" lang="ja">
          {#each example.ruby as part, j (j)}
            {#if needsRuby(part)}
              <ruby>{part.ruby}<rt>{part.rt}</rt></ruby>
            {:else}
              {part.ruby}
            {/if}
          {/each}
        </p>
        <p class="english" lang="en">{example.en}</p>
        <div class="say">
          <SpeakButton text={example.ja} {voice} label="Hear it" note={i === 0} />
        </div>
      </article>
    {/each}
  </section>

  {#if point.notes}
    <p class="note">{point.notes}</p>
  {/if}

  {#if point.tags.length > 0}
    <!-- The corpus's own tags, drawn as the words they are: they are what its
         search and its own grouping use, and they are honest about how loose a
         taxonomy this is. -->
    <p class="tags">
      {#each point.tags as tag, i (tag)}
        {#if i > 0}<span class="sep">·</span>{/if}<span class="tag">{tag}</span>
      {/each}
    </p>
  {/if}
</section>

<style>
  .stage {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    max-width: 70ch;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
  }

  .back {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 44px;
    padding: 5px 12px 5px 9px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .back:hover {
    border-color: var(--accent);
  }

  .where {
    margin-left: auto;
    font-size: 0.78rem;
    color: var(--muted);
  }

  .arrows {
    display: inline-flex;
    gap: 4px;
  }

  .arrow {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 44px;
    min-height: 44px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
  }

  .arrow:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .arrow:not(:disabled):hover {
    border-color: var(--accent);
  }

  .pattern {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.2rem 0;
  }

  .pattern .reading {
    font-size: 0.95rem;
    color: var(--muted);
  }

  .pattern .written {
    font-size: 2rem;
    line-height: 1.25;
  }

  .pattern .romaji {
    font-size: 0.9rem;
    color: var(--muted);
  }

  .meaning {
    margin: 0;
    font-size: 1.1rem;
  }

  .formation {
    margin: 0;
    font-size: 0.9rem;
    color: var(--muted);
  }

  .formation .label {
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.72rem;
    margin-right: 0.35rem;
  }

  .examples {
    display: grid;
    gap: 0.6rem;
    margin-top: 0.3rem;
  }

  .example {
    display: grid;
    gap: 0.35rem;
    padding: 0.6rem 0.7rem;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel);
  }

  /* The sentence is the thing being read, so it gets the size; the reading over a
     kanji is drawn by <rt> and stays small, as it is on the Phrases screen. */
  .sentence {
    margin: 0;
    font-size: 1.25rem;
    line-height: 2.2;
  }

  .sentence rt {
    font-size: 0.7rem;
    color: var(--muted);
  }

  .english {
    margin: 0;
    color: var(--muted);
    font-size: 0.9rem;
  }

  .say {
    min-height: 44px;
    display: flex;
    align-items: center;
  }

  .note {
    margin: 0;
    font-size: 0.9rem;
    color: var(--muted);
  }

  .tags {
    margin: 0;
    font-size: 0.78rem;
    color: var(--muted);
  }

  .tags .sep {
    margin: 0 0.35rem;
  }
</style>
