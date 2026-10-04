<script lang="ts">
  /**
   * The reading stage: one passage, with a reading over every kanji, and the card
   * for any word that is tapped — and nothing else.
   *
   * ## Why it is a screen of its own
   *
   * `PassagePanel` is the course: the passages there are. The passage used to be
   * drawn below a row of chips with the tapped word's card in a second column, which
   * on a phone meant scrolling past the passage to the card and back again.
   * `HANDOVER_NIHONGO.md` invariants 33 and 34 are the rule, and this is the one of
   * the three stages that has no board at all.
   *
   * ## A token is tappable when the pipeline linked it to a word
   *
   * Every kanji-bearing token is one of those — `prepare-passages` refuses to build
   * a passage that uses a kanji the vocabulary does not hold, because such a passage
   * would ask a learner to read a character they have no card for (invariant 22). So
   * a tap always works, and the underline under a tappable word says so.
   *
   * ## The card is drawn under the passage, which is measured rather than assumed
   *
   * The three shipped passages are seven lines between them, so a tapped word's card
   * lands on the same phone screen as the text it came from, and nothing is brought
   * into view. A passage long enough to push the card below the fold would need that
   * decision taken again — the card belongs to the word that was tapped, and a tap
   * whose answer is off screen reads as a tap that did nothing.
   */
  import Icon from "./Icon.svelte";
  import WordCard from "./WordCard.svelte";
  import * as api from "./api";
  import { untrack } from "svelte";
  import type { PassageSummary, PassageView, Word } from "./types";
  import type { VoiceStatus } from "./speech";
  import { needsRuby, tokenIsTappable } from "./words";

  interface Props {
    /**
     * The passage to open, as the course screen's card already holds it: the key the
     * text is fetched by, and the title and size the card showed.
     *
     * A **new value remounts this component** (`PassagePanel` keys it), so the state
     * below is read once and a later open does not have to be merged into a screen
     * that is already up.
     */
    start: PassageSummary;
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice: VoiceStatus;
    /** Back to the passages. */
    onleave: () => void;
  }

  let { start, voice, onleave }: Props = $props();

  let passage = $state<PassageView | null>(null);
  let opened = $state<Word | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /**
   * The token whose card is being fetched, so a slow answer cannot land on a word the
   * learner has moved on from — the two can arrive in either order, and the last tap
   * is the one that counts.
   */
  let asked = $state<string | null>(null);

  async function load(key: string) {
    try {
      error = null;
      busy = true;
      opened = null;
      asked = null;
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
      asked = word;
      const found = await api.wordOfText(word);
      if (untrack(() => asked) !== word) return;
      opened = found;
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    const key = start.key;
    void load(key);
  });
</script>

<section class="stage">
  <header class="head">
    <!--
      Out of the passage and back to the list. The word says where it goes rather
      than leaving an arrow to be guessed at — the kanji and kana stages' own
      arrangement.
    -->
    <button class="back" onclick={onleave} title="Back to the passages">
      <Icon name="back" />
      <span>Passages</span>
    </button>
    <span class="where">
      {start.tokens} {start.tokens === 1 ? "word" : "words"}
    </span>
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if busy}
    <p class="hint">Loading…</p>
  {/if}

  {#if passage}
    <article class="text" aria-label="Passage">
      <h3>{passage.title}</h3>
      {#each passage.lines as line, i (i)}
        <p class="line">
          {#each line as token, j (j)}
            {#if tokenIsTappable(token)}
              <button
                class="token"
                title="Open {token.surface}"
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
    </article>

    {#if opened}
      <div class="card-slot">
        <WordCard word={opened} {voice} />
      </div>
    {:else}
      <p class="hint">Tap any underlined word to open its card here.</p>
    {/if}
  {/if}
</section>

<style>
  .stage {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 90ch;
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

  .text {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 12px 14px;
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
    color: var(--muted);
  }

  .gloss {
    margin: 1rem 0 0;
    color: var(--muted);
    font-size: 0.9rem;
  }

  .card-slot {
    max-width: 60ch;
  }

  .hint {
    color: var(--muted);
    font-size: 0.85rem;
    margin: 0;
  }

  .error {
    color: var(--bad);
  }
</style>
