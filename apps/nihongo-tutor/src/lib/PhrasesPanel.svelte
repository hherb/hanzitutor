<script lang="ts">
  /**
   * Graded phrases: short sentences imported from Tatoeba, levelled on this course's
   * own ladder, read with a reading over every kanji, and heard in the system voice.
   *
   * ## What this screen is for, beside Read
   *
   * `Read` is the passages — the long form, written for this course. This is the
   * **graded** half: hundreds of real sentences, each one held to the vocabulary the
   * course teaches and placed in the band of its hardest word, so a learner can read
   * at their own level rather than at the level of three hand-written texts. Hanzi
   * Tutor's Phrases screen is the same idea.
   *
   * ## The band is the ladder the Words screen already uses
   *
   * Not a JLPT level: there has been no official list since 2010, and a phrase's band
   * is the band of its **hardest word** on exactly the ladder `band_name` names. That
   * is why the chips here and the chips on Words carry the same seven labels.
   *
   * ## Every kanji here is tappable, and that is a promise the pipeline keeps
   *
   * `prepare-phrases` throws away any sentence containing a kanji-bearing word the
   * course does not teach, so a tap always has a card behind it. The artifact is
   * asserted against the vocabulary by `tests/phrases_artifact.rs`, which is the same
   * rule `prepare-passages` enforces by refusing to write — see
   * `HANDOVER_NIHONGO.md` invariant 22.
   *
   * ## Attribution is on the screen, not only in the licence file
   *
   * The sentences are Tatoeba's, contributed under CC BY 2.0 FR, which asks for the
   * author and the licence to be named. Naming them once in a notice a reader never
   * opens is the weak reading of that, so every row carries its own sentence id, its
   * contributor and its licence, and the panel's header states the corpus. The long
   * per-contributor notice is shipped as well — `licences/TATOEBA-phrases.txt`.
   */
  import SpeakButton from "./SpeakButton.svelte";
  import WordCard from "./WordCard.svelte";
  import * as api from "./api";
  import { untrack } from "svelte";
  import type { PhraseBandView, PhraseView, Word } from "./types";
  import { voiceNote, type VoiceStatus } from "./speech";
  import { firstNonEmptyBand, phraseSource, totalPhrases } from "./phrases";
  import { needsRuby, tokenIsTappable } from "./words";

  interface Props {
    /**
     * What the `voice` command answered, asked once by `App.svelte`.
     *
     * Handed down rather than fetched here, so one missing Japanese voice disables
     * every "Hear it" in the app together and the reason is said once on this screen
     * instead of once per row.
     */
    voice: VoiceStatus;
  }

  let { voice }: Props = $props();

  let bands = $state<PhraseBandView[]>([]);
  /** The band on screen, or `null` when the corpus holds no phrase at all. */
  let band = $state<number | null>(null);
  let phrases = $state<PhraseView[]>([]);
  let error = $state<string | null>(null);
  let busy = $state(false);
  /** The word whose card is open under a phrase, and the phrase it belongs to. */
  let opened = $state<{ id: number; word: Word } | null>(null);
  /**
   * The word being fetched, so a slow answer cannot land on a phrase the learner has
   * moved on from — the two can arrive in either order, and the last tap counts.
   */
  let asked = $state<string | null>(null);

  const why = $derived(voiceNote(voice));
  const total = $derived(totalPhrases(bands));
  const chosen = $derived(bands.find((entry) => entry.band === band) ?? null);

  async function loadBands() {
    try {
      error = null;
      bands = await api.phraseBands();
      band = firstNonEmptyBand(bands);
    } catch (e) {
      error = String(e);
    }
  }

  async function loadBand(band: number) {
    try {
      error = null;
      busy = true;
      phrases = await api.phrasesInBand(band);
    } catch (e) {
      error = String(e);
      phrases = [];
    } finally {
      busy = false;
    }
  }

  async function openWord(phrase: number, word: string) {
    const key = `${phrase}:${word}`;
    try {
      error = null;
      asked = key;
      const found = await api.wordOfText(word);
      if (untrack(() => asked) !== key) return;
      opened = { id: phrase, word: found };
    } catch (e) {
      error = String(e);
    }
  }

  // Choosing a band drops the open card: it belongs to a phrase in the list that is
  // no longer on screen, and leaving it up would draw a word over a sentence it did
  // not come from.
  $effect(() => {
    const next = band;
    opened = null;
    asked = null;
    if (next === null) {
      phrases = [];
      return;
    }
    void loadBand(next);
  });

  void loadBands();
</script>

<section class="phrases">
  <header>
    <h2>Phrases</h2>
    <p class="sub">
      {total.toLocaleString()} short sentences to read at the level you are at — every kanji in
      them is one you have a card for. Tap a word to open its card, or hear the whole
      sentence in the system's Japanese voice.
    </p>
    {#if chosen}
      <p class="hint">
        These are <strong>{chosen.name}</strong> phrases: a phrase's band is the band of its
        <strong>hardest word</strong>, on the same ladder the Words screen uses — a word's band
        being the highest school grade among its kanji. The ladder is <strong>ours</strong>, not
        a JLPT level; there has been no official JLPT list since 2010.
      </p>
    {/if}
    <p class="provenance">
      Sentences from <a href="https://tatoeba.org" rel="noreferrer">Tatoeba</a>, contributed
      under CC BY 2.0 FR; each one keeps its sentence id and its contributor below. The
      segmentation and the readings were computed here, at build time.
    </p>
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if why}
    <p class="note">{why}</p>
  {/if}

  {#if bands.length > 0}
    <div class="bands" role="tablist" aria-label="Bands">
      {#each bands as entry (entry.band)}
        <button
          role="tab"
          aria-selected={band === entry.band}
          class:active={band === entry.band}
          disabled={entry.phrases === 0}
          title={entry.phrases === 0 ? `No phrases in ${entry.name}` : undefined}
          onclick={() => (band = entry.band)}
        >
          <span class="label">{entry.name}</span>
          <span class="count">{entry.phrases}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if busy}
    <p class="hint">Loading…</p>
  {:else if bands.length === 0 && !error}
    <p class="hint">Loading the phrases…</p>
  {:else if band === null}
    <!--
      Not an error and not a loader: the artifact decoded, it simply holds nothing.
      That is what a build with too narrow a filter produces, and saying so is more
      use than an empty list.
    -->
    <p class="status">
      No phrases are bundled in this build. The corpus is imported and filtered when the
      artifact is built, and this one kept none — see <code>pnpm run prepare-phrases</code>.
    </p>
  {:else if phrases.length === 0}
    <p class="status">No phrases in this band.</p>
  {:else}
    <ol class="list">
      {#each phrases as phrase (phrase.id)}
        <li>
          <p class="sentence" lang="ja">
            {#each phrase.tokens as token, i (i)}
              {#if tokenIsTappable(token)}
                <button
                  class="token"
                  title="Open {token.word}"
                  onclick={() => void openWord(phrase.id, token.word as string)}
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

          {#if phrase.english}
            <p class="english" lang="en">{phrase.english}</p>
          {/if}

          <div class="meta">
            <SpeakButton text={phrase.text} {voice} label="Hear it" note={false} />
            <span class="source">{phraseSource(phrase)}</span>
          </div>

          {#if opened?.id === phrase.id}
            <div class="card-slot">
              <WordCard word={opened.word} {voice} />
            </div>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .phrases header h2 {
    margin: 0;
  }

  .sub {
    margin: 0.2rem 0 0;
    color: var(--muted, #6b6b6b);
    max-width: 70ch;
  }

  .hint {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: var(--muted, #6b6b6b);
    max-width: 70ch;
  }

  .provenance {
    margin: 0.5rem 0 0;
    font-size: 0.8rem;
    color: var(--muted, #6b6b6b);
    max-width: 70ch;
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

  .bands button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .bands button.active {
    border-color: var(--accent, #2f6f4f);
    background: var(--accent-soft, #eaf3ed);
  }

  .bands .count {
    font-size: 0.8rem;
    color: var(--muted, #6b6b6b);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.6rem;
    max-width: 70ch;
  }

  .list li {
    display: grid;
    gap: 0.3rem;
    padding: 0.6rem 0.7rem;
    border: 1px solid var(--line, #dcdcd6);
    border-radius: 10px;
    background: var(--panel, #fff);
  }

  /* The sentence is the thing being read, so it gets the size; the reading over a
     kanji is drawn by <rt> and stays small, as it is on the Read screen. */
  .sentence {
    margin: 0;
    font-size: 1.3rem;
    line-height: 2.3;
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

  .english {
    margin: 0;
    color: var(--muted, #6b6b6b);
    font-size: 0.9rem;
  }

  /* The control and the attribution share a line, and the attribution is allowed to
     wrap under it on a narrow screen rather than squeezing the button. */
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.6rem;
    min-height: 44px;
  }

  .source {
    font-size: 0.75rem;
    color: var(--muted, #6b6b6b);
  }

  .card-slot {
    max-width: 60ch;
  }

  .status,
  .note {
    color: var(--muted, #6b6b6b);
    font-size: 0.85rem;
  }

  .error {
    color: var(--bad, #a3341f);
  }
</style>
