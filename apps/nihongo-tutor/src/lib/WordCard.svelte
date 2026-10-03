<script lang="ts">
  /**
   * One word, drawn: the word with its furigana over the right characters, what it
   * means, where it sits in the ladder, and a box to type its reading.
   *
   * Two things here are load-bearing rather than decorative.
   *
   * **The reading is the word's own.** It comes from the dictionary over the IPC —
   * never composed from the characters — which is why 大人 shows おとな and not
   * だいじん. The learner types it, and the check accepts romaji or kana.
   *
   * **The band is ours, and the card says so.** There has been no official JLPT
   * kanji or vocabulary list since 2010, so the bands are this project's
   * derivation from the kyōiku grades and EDRDG's frequency ranking, and the name
   * under the word is `nihongo_core::band_name`'s rather than a level number that
   * would imply an authority the data does not have.
   */
  import * as api from "./api";
  import type { ReadingCheck, Word } from "./types";
  import { furiganaCoversWord, furiganaSpellsReading, needsRuby, readingOf } from "./words";

  let {
    word,
    /** Show the band and frequency line — off in a passage, where the word is in context. */
    showBand = true,
    /** Start with the reading box open. */
    askReading = true,
  }: { word: Word; showBand?: boolean; askReading?: boolean } = $props();

  let typed = $state("");
  let check = $state<ReadingCheck | null>(null);
  let error = $state<string | null>(null);

  // A new word is a new question: a verdict, a typed answer and an error all
  // belong to the word that was up when they were made.
  $effect(() => {
    word;
    typed = "";
    check = null;
    error = null;
  });

  async function checkTyped() {
    if (!typed.trim()) return;
    try {
      error = null;
      check = await api.checkWord(word.text, word.reading, typed);
    } catch (e) {
      error = String(e);
    }
  }

  /**
   * Whether the ruby can be trusted for this word.
   *
   * A word with no alignment has no ruby to draw, which is fine; a word whose
   * segments do not add up to it would put readings over the wrong characters,
   * which is not — so in that case the reading is shown plainly instead.
   */
  const drawable = $derived(furiganaCoversWord(word) && furiganaSpellsReading(word));
  const showRuby = $derived(word.furigana.length > 0 && drawable);
</script>

<article class="card">
  <div class="head">
    {#if showRuby}
      <p class="word" aria-label="{word.text} — {word.reading}">
        {#each word.furigana as segment, i (i)}
          {#if needsRuby(segment)}
            <ruby>{segment.ruby}<rt>{segment.rt}</rt></ruby>
          {:else}
            <span>{segment.ruby}</span>
          {/if}
        {/each}
      </p>
    {:else}
      <p class="word" aria-label="{word.text} — {word.reading}">
        <span class="plain">{word.text}</span>
      </p>
    {/if}
    <p class="reading">{word.reading}</p>
  </div>

  <p class="meaning">{word.meaning}</p>

  {#if showBand}
    <p class="band">
      <span class="name">{word.bandName}</span>
      {#if word.nf !== null}
        <span class="nf">EDRDG frequency nf{String(word.nf).padStart(2, "0")}</span>
      {:else}
        <span class="nf">EDRDG marks it common, without a frequency rank</span>
      {/if}
    </p>
    <p class="hint">
      Band names are ours: derived from the school grades of a word's kanji and
      EDRDG's frequency ranking, not from the JLPT, which publishes no list.
    </p>
  {/if}

  {#if askReading}
    <div class="typing">
      <label for="word-reading">Type the reading</label>
      <div class="row">
        <input
          id="word-reading"
          bind:value={typed}
          onkeydown={(e) => {
            if (e.key === "Enter") void checkTyped();
          }}
          placeholder="in romaji or kana"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
        />
        <button onclick={() => void checkTyped()} disabled={!typed.trim()}>Check</button>
      </div>
      {#if check}
        <p class="result" class:ok={check.correct}>
          {#if check.correct}
            Correct — {word.text} is {readingOf(word)}.
          {:else}
            That is {check.produced || "not a reading"} — {word.text} is read
            {readingOf(word)}.
          {/if}
        </p>
      {/if}
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
      <p class="hint small">Romaji or kana, either is accepted.</p>
    </div>
  {/if}
</article>

<style>
  .card {
    border: 1px solid var(--line, #dcdcd6);
    border-radius: 10px;
    padding: 1rem 1.1rem;
    background: var(--panel, #fff);
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 0.9rem;
    flex-wrap: wrap;
  }

  .word {
    margin: 0;
    font-size: 2.6rem;
    line-height: 1.5;
  }

  .word ruby {
    ruby-align: center;
  }

  .word rt {
    font-size: 0.85rem;
    color: var(--muted, #6b6b6b);
  }

  .reading {
    margin: 0;
    color: var(--muted, #6b6b6b);
    font-size: 1.1rem;
  }

  .meaning {
    margin: 0.5rem 0 0;
  }

  .band {
    margin: 0.6rem 0 0;
    display: flex;
    gap: 0.7rem;
    flex-wrap: wrap;
    font-size: 0.85rem;
    color: var(--muted, #6b6b6b);
  }

  .name {
    font-weight: 600;
    color: var(--ink, #26262a);
  }

  .typing {
    margin-top: 0.9rem;
  }

  .typing label {
    display: block;
    font-size: 0.85rem;
    color: var(--muted, #6b6b6b);
  }

  .row {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.3rem;
  }

  input {
    flex: 1;
    padding: 0.4rem 0.55rem;
    border: 1px solid var(--line, #dcdcd6);
    border-radius: 6px;
    font: inherit;
  }

  button {
    font: inherit;
    padding: 0.4rem 0.8rem;
    border: 1px solid var(--line, #dcdcd6);
    border-radius: 6px;
    background: var(--panel, #fff);
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .result {
    margin: 0.5rem 0 0;
  }

  .result.ok {
    color: var(--ok, #1f7a4d);
  }

  .error {
    color: var(--bad, #a3341f);
    margin: 0.4rem 0 0;
  }

  .hint {
    margin: 0.4rem 0 0;
    font-size: 0.8rem;
    color: var(--muted, #6b6b6b);
  }

  .hint.small {
    font-size: 0.75rem;
  }
</style>
