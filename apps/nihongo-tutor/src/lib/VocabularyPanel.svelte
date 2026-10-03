<script lang="ts">
  /**
   * The vocabulary: the ladder down the side, the words of one band, and the card
   * for the word that is selected.
   *
   * The ladder is the part worth reading. A word's band is the **highest kyōiku
   * grade among its kanji** — 1 to 6, with a seventh for a word containing a kanji
   * from the jōyō remainder — and EDRDG's frequency rank orders the words inside a
   * band. That is a teaching order, not a popularity contest: a word becomes
   * readable exactly when its kanji are known.
   *
   * It is **this project's** ladder and the screen says so twice — in the header
   * and on every card — because there has been no official JLPT kanji or
   * vocabulary list since 2010, and a level number would imply an authority the
   * data does not have.
   */
  import WordCard from "./WordCard.svelte";
  import * as api from "./api";
  import type { BandView, Word } from "./types";
  import type { VoiceStatus } from "./speech";
  import { pageWindow } from "./words";

  /**
   * What the `voice` command answered.
   *
   * Passed straight through to the card rather than asked for here: `App.svelte`
   * asks once, so one missing Japanese voice disables every control in the app
   * together, and a panel that asked again would be a second answer to one
   * question.
   */
  let { voice }: { voice: VoiceStatus } = $props();

  let bands = $state<BandView[]>([]);
  let band = $state(1);
  let page = $state(1);
  let total = $state(0);
  let words = $state<Word[]>([]);
  let selected = $state<Word | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /** How many words a page holds; the same number the command is asked for. */
  const PAGE = 24;

  const window_ = $derived(pageWindow(total, PAGE, page));

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
      selected = null;
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

<section class="vocab">
  <header>
    <h2>Words</h2>
    <p class="sub">
      {bands.reduce((sum, b) => sum + b.words, 0).toLocaleString()} words, each with the reading
      the dictionary gives it and an English gloss.
    </p>
    <p class="hint">
      The bands are <strong>ours</strong>: a word's band is the highest school grade among its
      kanji, with a seventh for the jōyō remainder, and EDRDG's frequency orders them inside a
      band. They are not the JLPT's — there has been no official JLPT list since 2010 — and a
      learner should read them as a rough order rather than a level.
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

  <div class="layout">
    <section class="list" aria-label="Words in this band">
      {#if busy}
        <p class="hint">Loading…</p>
      {/if}
      <ul>
        {#each words as entry (entry.text + entry.reading)}
          <li>
            <button
              class="row"
              class:active={selected?.text === entry.text && selected?.reading === entry.reading}
              onclick={() => (selected = entry)}
            >
              <span class="text">{entry.text}</span>
              <span class="reading">{entry.reading}</span>
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

    <section class="card-slot">
      {#if selected}
        <WordCard word={selected} {voice} />
      {:else}
        <p class="placeholder">Choose a word to see its furigana and to check its reading.</p>
      {/if}
    </section>
  </div>
</section>

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

  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.1fr);
    gap: 1.2rem;
    align-items: start;
  }

  @media (max-width: 900px) {
    .layout {
      grid-template-columns: 1fr;
    }
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
    background: var(--hover, #f4f4f1);
  }

  .row.active {
    border-color: var(--accent, #2f6f4f);
    background: var(--accent-soft, #eaf3ed);
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

  .placeholder {
    color: var(--muted, #6b6b6b);
    border: 1px dashed var(--line, #dcdcd6);
    border-radius: 10px;
    padding: 2rem 1rem;
    text-align: center;
  }

  .error {
    color: var(--bad, #a3341f);
  }
</style>
