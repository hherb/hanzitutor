<script lang="ts">
  /**
   * Reading: a short passage with a reading over every kanji, and a tap on any
   * word opens the card for it.
   *
   * The segmentation is not done here. Japanese has no spaces, so words have to be
   * found by a morphological analyser — and the analyser and its 134 MB dictionary
   * run at **build time**, in `prepare-passages`, so the artifact carries the
   * tokens and this screen only draws them. That is what keeps the app's promise
   * that nothing is downloaded: there is no tokeniser here to download one for.
   *
   * A token is tappable when the pipeline linked it to a word the course teaches,
   * and every kanji-bearing token is one of those — `prepare-passages` refuses to
   * build a passage that uses a kanji the vocabulary does not hold, because such a
   * passage would ask a learner to read a character they have no card for. So a
   * tap always works, and the underline under a tappable word says so.
   */
  import WordCard from "./WordCard.svelte";
  import * as api from "./api";
  import type { PassageSummary, PassageView, Word } from "./types";
  import type { VoiceStatus } from "./speech";
  import { needsRuby, tokenIsTappable } from "./words";

  /** What the `voice` command answered, asked once by `App.svelte`. */
  let { voice }: { voice: VoiceStatus } = $props();

  let summaries = $state<PassageSummary[]>([]);
  let key = $state<string | null>(null);
  let passage = $state<PassageView | null>(null);
  let opened = $state<Word | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function loadList() {
    try {
      error = null;
      summaries = await api.passages();
      key = summaries[0]?.key ?? null;
    } catch (e) {
      error = String(e);
    }
  }

  async function loadPassage() {
    if (!key) return;
    try {
      error = null;
      busy = true;
      opened = null;
      passage = await api.passage(key);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function open(word: string) {
    try {
      error = null;
      opened = await api.wordOfText(word);
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    void key;
    void loadPassage();
  });

  void loadList();
</script>

<section class="reading">
  <header>
    <h2>Read</h2>
    <p class="sub">
      Short passages, written for this course and held to the words it teaches — every kanji
      here is one you have a card for. Tap a word to open it.
    </p>
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <div class="passages" role="tablist" aria-label="Passages">
    {#each summaries as entry (entry.key)}
      <button
        role="tab"
        aria-selected={key === entry.key}
        class:active={key === entry.key}
        onclick={() => (key = entry.key)}
      >
        <span class="title">{entry.title}</span>
        <span class="count">{entry.tokens} words</span>
      </button>
    {/each}
  </div>

  <div class="layout">
    <section class="text" aria-label="Passage">
      {#if busy}
        <p class="hint">Loading…</p>
      {/if}
      {#if passage}
        <h3>{passage.title}</h3>
        {#each passage.lines as line, i (i)}
          <p class="line">
            {#each line as token, j (j)}
              {#if tokenIsTappable(token)}
                <button
                  class="token"
                  title={token.surface}
                  onclick={() => void open(token.word as string)}
                >
                  {#if needsRuby({ ruby: token.surface, rt: token.rt })}
                    <ruby>{token.surface}<rt>{token.rt}</rt></ruby>
                  {:else}
                    {token.surface}
                  {/if}
                </button>
              {:else}
                <span class="token plain">
                  {#if needsRuby({ ruby: token.surface, rt: token.rt })}
                    <ruby>{token.surface}<rt>{token.rt}</rt></ruby>
                  {:else}
                    {token.surface}
                  {/if}
                </span>
              {/if}
            {/each}
          </p>
        {/each}
        {#if passage.gloss}
          <p class="gloss">{passage.gloss}</p>
        {/if}
      {/if}
    </section>

    <section class="card-slot">
      {#if opened}
        <WordCard word={opened} {voice} />
      {:else}
        <p class="placeholder">Tap any underlined word in the passage to open it here.</p>
      {/if}
    </section>
  </div>
</section>

<style>
  .reading header h2 {
    margin: 0;
  }

  .sub {
    margin: 0.2rem 0 0;
    color: var(--muted, #6b6b6b);
    max-width: 65ch;
  }

  .passages {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin: 1rem 0;
  }

  .passages button {
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

  .passages button.active {
    border-color: var(--accent, #2f6f4f);
    background: var(--accent-soft, #eaf3ed);
  }

  .passages .count {
    font-size: 0.8rem;
    color: var(--muted, #6b6b6b);
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
    gap: 1.2rem;
    align-items: start;
  }

  @media (max-width: 900px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }

  .text h3 {
    margin: 0 0 0.6rem;
    font-size: 1.1rem;
  }

  .line {
    margin: 0 0 0.7rem;
    font-size: 1.5rem;
    line-height: 2.4;
  }

  .token {
    font: inherit;
    font-size: inherit;
    padding: 0 0.05rem;
    border: 0;
    background: none;
    color: inherit;
    vertical-align: baseline;
  }

  .token.plain {
    cursor: default;
  }

  button.token {
    border-bottom: 2px solid var(--accent-soft, #cfe3d7);
    cursor: pointer;
  }

  button.token:hover {
    background: var(--accent-soft, #eaf3ed);
  }

  .token rt {
    font-size: 0.7rem;
    color: var(--muted, #6b6b6b);
  }

  .gloss {
    margin-top: 1rem;
    color: var(--muted, #6b6b6b);
    font-size: 0.9rem;
  }

  .placeholder {
    color: var(--muted, #6b6b6b);
    border: 1px dashed var(--line, #dcdcd6);
    border-radius: 10px;
    padding: 2rem 1rem;
    text-align: center;
  }

  .hint {
    color: var(--muted, #6b6b6b);
  }

  .error {
    color: var(--bad, #a3341f);
  }
</style>
