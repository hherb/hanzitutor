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
   *
   * **And the word can be heard, as its own reading.** What is spoken is the
   * dictionary's kana — おとな for 大人 — and never the written form, which is the
   * same rule the reading itself follows one step further out: a synthesiser handed
   * 大人 would choose a reading, and the app does not choose readings.
   *
   * **And the reading is held back while the card asks for it.** With the box
   * open, the furigana, the reading line and the voice would each be the answer
   * printed (or spoken) above the question, so they wait until a check has been
   * made — the verdict names the reading, right or wrong — or the learner asks to
   * see it (`words.ts`'s `readingShown`).
   */
  import * as api from "./api";
  import SpeakButton from "./SpeakButton.svelte";
  import type { ReadingCheck, Word } from "./types";
  import type { VoiceStatus } from "./speech";
  import { isSubmitEnter, trackComposition } from "./ime";
  import {
    furiganaCoversWord,
    furiganaSpellsReading,
    needsRuby,
    readingOf,
    readingShown,
  } from "./words";

  let {
    word,
    /** What the `voice` command answered, asked once by the screen above. */
    voice,
    /** Show the band and frequency line — off in a passage, where the word is in context. */
    showBand = true,
    /** Start with the reading box open. */
    askReading = true,
  }: {
    word: Word;
    voice: VoiceStatus;
    showBand?: boolean;
    askReading?: boolean;
  } = $props();

  let typed = $state("");
  let check = $state<ReadingCheck | null>(null);
  let error = $state<string | null>(null);
  /** Whether the learner asked to see the reading without typing it. */
  let revealed = $state(false);

  // A new word is a new question: a verdict, a typed answer and an error all
  // belong to the word that was up when they were made.
  $effect(() => {
    word;
    typed = "";
    check = null;
    error = null;
    revealed = false;
  });

  const shown = $derived(readingShown({ asking: askReading, checked: check !== null, revealed }));

  /** Whether a check is in flight, so a second Enter does not send a second one. */
  let checking = false;

  async function checkTyped() {
    if (checking || !typed.trim()) return;
    const asked = word;
    try {
      checking = true;
      error = null;
      const answered = await api.checkWord(asked.text, asked.reading, typed);
      // A verdict for the word that was up when it was asked, and no other: the card
      // can be handed a new word while the check is on its way.
      if (word === asked) check = answered;
    } catch (e) {
      if (word === asked) error = String(e);
    } finally {
      checking = false;
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
  const showRuby = $derived(word.furigana.length > 0 && drawable && shown);
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
      <p class="word" aria-label={shown ? `${word.text} — ${word.reading}` : word.text}>
        <span class="plain">{word.text}</span>
      </p>
    {/if}
    {#if shown}
      <p class="reading">{word.reading}</p>
      <SpeakButton text={word.reading} {voice} />
    {:else}
      <button class="reveal" type="button" onclick={() => (revealed = true)}>
        Show the reading
      </button>
    {/if}
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
          use:trackComposition
          onkeydown={(e) => {
            // Not the Enter that commits an IME candidate (`ime.ts`): a learner
            // writing おとな in kana commits it before checking it.
            if (isSubmitEnter(e)) void checkTyped();
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

  /* Where the reading would be, while the box below asks for it. */
  .reveal {
    padding: 0.2rem 0.6rem;
    font-size: 0.85rem;
    color: var(--muted, #6b6b6b);
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
