<script lang="ts">
  /**
   * The vocabulary: the course screen — the ladder down the side and the words of
   * one band — and the stage one of the words opens.
   *
   * ## Two screens, and this is the first one
   *
   * The card used to sit in a second column beside the list: the bands, the page of
   * words, and the chosen word's card. On a phone that is N12's complaint — the
   * thing a learner tapped a row for is *below* the twenty-four rows they then have
   * to scroll back through — so the list is the course and `WordStage` is what a row
   * opens. `HANDOVER_NIHONGO.md` invariants 33 and 34 are the rule.
   *
   * ## The ladder is the part worth reading
   *
   * A word's band is the **highest kyōiku grade among its kanji** — 1 to 6, with a
   * seventh for a word containing a kanji from the jōyō remainder — and EDRDG's
   * frequency rank orders the words inside a band. That is a teaching order, not a
   * popularity contest: a word becomes readable exactly when its kanji are known.
   *
   * It is **this project's** ladder and the screen says so twice — in the header
   * and on every card — because there has been no official JLPT kanji or
   * vocabulary list since 2010, and a level number would imply an authority the
   * data does not have.
   */
  import WordStage from "./WordStage.svelte";
  import * as api from "./api";
  import type { BandView, Word } from "./types";
  import type { VoiceStatus } from "./speech";
  import { pageWindow } from "./words";

  interface Props {
    /**
     * What the `voice` command answered.
     *
     * Passed straight through to the stage rather than asked for here: `App.svelte`
     * asks once, so one missing Japanese voice disables every control in the app
     * together, and a panel that asked again would be a second answer to one
     * question.
     */
    voice: VoiceStatus;
    /**
     * Whether a stage is up, told to `App.svelte` so it can take the course switch
     * and the tab row out of the way while one is (invariants 33 and 34).
     */
    stage?: boolean;
  }

  let { voice, stage = $bindable(false) }: Props = $props();

  let bands = $state<BandView[]>([]);
  let band = $state(1);
  let page = $state(1);
  let total = $state(0);
  let words = $state<Word[]>([]);
  /** The word whose stage is up, or `null` while the course itself is showing. */
  let open = $state<Word | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /** How many words a page holds; the same number the command is asked for. */
  const PAGE = 24;

  const window_ = $derived(pageWindow(total, PAGE, page));

  /**
   * Keep the app told whether a stage is up.
   *
   * One place rather than two: `open` is set by a row and by leaving, and a flag
   * written at each would be wrong the day a third caller arrives.
   */
  $effect(() => {
    stage = open !== null;
  });

  async function loadBands() {
    try {
      error = null;
      bands = await api.wordBands();
      if (bands.length > 0 && !bands.some((b) => b.band === band)) {
        band = bands[0].band;
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function loadPage() {
    try {
      error = null;
      busy = true;
      const found = await api.wordsInBand(band, window_.offset, window_.limit);
      words = found.words;
      total = found.total;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  // A band change starts at page one, and the page load follows from the window
  // the state derives — so there is one place that fetches rather than two.
  $effect(() => {
    void band;
    page = 1;
  });

  $effect(() => {
    void window_.offset;
    void window_.limit;
    void loadPage();
  });

  void loadBands();
</script>

{#if open}
  <!--
    Keyed on the word, so a second row mounts a fresh stage rather than merging the
    new word into the reading box and the verdict of the old one.
  -->
  {#key open.text + open.reading}
    <WordStage word={open} {voice} onleave={() => (open = null)} />
  {/key}
{:else}
  <section class="vocab">
    <header>
      <h2>Words</h2>
      <p class="sub">
        {bands.reduce((sum, b) => sum + b.words, 0).toLocaleString()} words, each with the
        reading the dictionary gives it and an English gloss. Choose a word to see its
        furigana and to check its reading.
      </p>
      <p class="hint">
        The bands are <strong>ours</strong>: a word's band is the highest school grade among
        its kanji, with a seventh for the jōyō remainder, and EDRDG's frequency orders them
        inside a band. They are not the JLPT's — there has been no official JLPT list since
        2010 — and a learner should read them as a rough order rather than a level.
      </p>
    </header>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <div class="bands" role="tablist" aria-label="Bands">
      {#each bands as entry (entry.band)}
        <button
          role="tab"
          aria-selected={band === entry.band}
          class:active={band === entry.band}
          onclick={() => (band = entry.band)}
        >
          <span class="label">{entry.name}</span>
          <span class="count">{entry.words}</span>
        </button>
      {/each}
    </div>

    <section class="list" aria-label="Words in this band">
      {#if busy}
        <p class="hint">Loading…</p>
      {/if}
      <ul>
        {#each words as entry (entry.text + entry.reading)}
          <li>
            <button
              class="row"
              onclick={() => (open = entry)}
              title="Open {entry.text} to see its furigana and check its reading"
            >
              <span class="text" lang="ja">{entry.text}</span>
              <span class="reading" lang="ja">{entry.reading}</span>
              <span class="gloss">{entry.meaning}</span>
            </button>
          </li>
        {/each}
      </ul>

      {#if window_.pages > 1}
        <nav class="pager" aria-label="Pages">
          <button disabled={window_.page <= 1} onclick={() => (page = window_.page - 1)}>
            Previous
          </button>
          <span>page {window_.page} of {window_.pages}</span>
          <button disabled={window_.page >= window_.pages} onclick={() => (page = window_.page + 1)}>
            Next
          </button>
        </nav>
      {/if}
    </section>
  </section>
{/if}

<style>
  .vocab header h2 {
    margin: 0;
  }

  .sub {
    margin: 0.2rem 0 0;
    color: var(--muted, #6b6b6b);
  }

  .hint {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: var(--muted, #6b6b6b);
    max-width: 60ch;
  }

  .bands {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin: 1rem 0;
  }

  .bands button {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    font: inherit;
    padding: 0.35rem 0.7rem;
    border: 1px solid var(--line, #dcdcd6);
    border-radius: 999px;
    background: var(--panel, #fff);
    cursor: pointer;
  }

  .bands button.active {
    border-color: var(--accent, #2f6f4f);
    background: var(--accent-soft, #eaf3ed);
  }

  .bands .count {
    font-size: 0.8rem;
    color: var(--muted, #6b6b6b);
  }

  /* The band's words, as a list to choose from rather than a column beside a card
     — the card is a screen of its own now. */
  .list {
    max-width: 90ch;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.25rem;
  }

  .row {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(4rem, auto) minmax(4rem, auto) 1fr;
    gap: 0.7rem;
    align-items: baseline;
    text-align: left;
    font: inherit;
    padding: 0.35rem 0.5rem;
    border: 1px solid transparent;
    border-radius: 6px;
    background: none;
    cursor: pointer;
  }

  .row:hover {
    border-color: var(--accent, #2f6f4f);
    background: var(--hover, #f4f4f1);
  }

  .text {
    font-size: 1.15rem;
  }

  .reading {
    color: var(--muted, #6b6b6b);
    font-size: 0.9rem;
  }

  .gloss {
    color: var(--muted, #6b6b6b);
    font-size: 0.85rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pager {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    margin-top: 0.8rem;
    font-size: 0.85rem;
    color: var(--muted, #6b6b6b);
  }

  .pager button {
    font: inherit;
    padding: 0.3rem 0.6rem;
    border: 1px solid var(--line, #dcdcd6);
    border-radius: 6px;
    background: var(--panel, #fff);
    cursor: pointer;
  }

  .pager button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .error {
    color: var(--bad, #a3341f);
  }
</style>
