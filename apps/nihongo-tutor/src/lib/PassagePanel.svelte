<script lang="ts">
  /**
   * Reading: the course screen — the passages there are — and the stage one of them
   * opens.
   *
   * ## Two screens, and this is the first one
   *
   * The passage used to be drawn below a row of chips with the tapped word's card
   * beside it, which on a phone meant scrolling past the passage to the card and back
   * to the passage. `PassageStage` is the second screen, the chrome steps aside while
   * one is up, and `HANDOVER_NIHONGO.md` invariants 33 and 34 are the rule.
   *
   * ## The segmentation is not done here
   *
   * Japanese has no spaces, so words have to be found by a morphological analyser —
   * and the analyser and its 134 MB dictionary run at **build time**, in
   * `prepare-passages`, so the artifact carries the tokens and the stage only draws
   * them. That is what keeps the app's promise that nothing is downloaded: there is
   * no tokeniser here to download one for.
   *
   * ## The gloss is not on this screen
   *
   * A passage's English gloss is shown under the passage it belongs to, in the stage.
   * Putting it on the card a learner chooses from would answer the reading before it
   * was attempted, which is the one thing a reading exercise cannot afford.
   */
  import PassageStage from "./PassageStage.svelte";
  import * as api from "./api";
  import type { PassageSummary } from "./types";
  import type { VoiceStatus } from "./speech";

  interface Props {
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice: VoiceStatus;
    /**
     * Whether a stage is up, told to `App.svelte` so it can take the course switch
     * and the tab row out of the way while one is (invariants 33 and 34).
     */
    stage?: boolean;
  }

  let { voice, stage = $bindable(false) }: Props = $props();

  let summaries = $state<PassageSummary[]>([]);
  let error = $state<string | null>(null);
  /** The passage whose stage is up, or `null` while the course itself is showing. */
  let open = $state<PassageSummary | null>(null);

  async function loadList() {
    try {
      error = null;
      summaries = await api.passages();
    } catch (e) {
      error = String(e);
    }
  }
  void loadList();

  /**
   * Keep the app told whether a stage is up.
   *
   * One place rather than two: `open` is set by a card and by leaving, and a flag
   * written at each would be wrong the day a third caller arrives.
   */
  $effect(() => {
    stage = open !== null;
  });
</script>

{#if open}
  <!--
    Keyed on the passage, so a second card mounts a fresh stage rather than merging
    the new passage into the text, the tapped word and the error of the old one.
  -->
  {#key open.key}
    <PassageStage start={open} {voice} onleave={() => (open = null)} />
  {/key}
{:else}
  <section class="reading">
    <header>
      <h2>Read</h2>
      <p class="sub">
        Short passages, written for this course and held to the words it teaches — every kanji
        here is one you have a card for. Choose a passage to read it, and tap a word inside it
        to open that word.
      </p>
    </header>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <ul class="passages">
      {#each summaries as entry (entry.key)}
        <li>
          <button class="passage" onclick={() => (open = entry)} title="Read {entry.title}">
            <span class="title">{entry.title}</span>
            <span class="count">
              {entry.lines} {entry.lines === 1 ? "line" : "lines"} · {entry.tokens}
              {entry.tokens === 1 ? "word" : "words"}
            </span>
          </button>
        </li>
      {/each}
    </ul>

    {#if summaries.length === 0 && !error}
      <p class="hint">Loading the passages…</p>
    {/if}
  </section>
{/if}

<style>
  .reading header h2 {
    margin: 0;
  }

  .sub {
    margin: 0.2rem 0 0;
    color: var(--muted, #6b6b6b);
    max-width: 65ch;
  }

  /* The passages, as cards to choose from rather than a row of chips above the text
     — the text is a screen of its own now. */
  .passages {
    list-style: none;
    margin: 1rem 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 0.5rem;
    max-width: 90ch;
  }

  .passage {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.25rem;
    font: inherit;
    padding: 0.7rem 0.9rem;
    border: 1px solid var(--line, #dcdcd6);
    border-radius: 10px;
    background: var(--panel, #fff);
    color: var(--ink);
    text-align: left;
    cursor: pointer;
  }

  .passage:hover {
    border-color: var(--accent, #2f6f4f);
  }

  .title {
    font-size: 1rem;
    font-weight: 600;
  }

  .count {
    font-size: 0.8rem;
    color: var(--muted, #6b6b6b);
  }

  .hint {
    margin-top: 1rem;
    color: var(--muted, #6b6b6b);
  }

  .error {
    color: var(--bad, #a3341f);
  }
</style>
