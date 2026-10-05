<script lang="ts">
  /**
   * The grammar course: the course screen — one ordered list of 526 patterns and a
   * search box — and the stage one of them opens.
   *
   * ## No ladder, and that is the measurement rather than a shortcut
   *
   * The Words and Phrases screens are levelled by the band a word's kanji give it.
   * The obvious move was to level a pattern the same way, by the hardest character
   * in its pattern and its examples, and it was **measured before it was rejected**: 344 of
   * the 526
   * points land in band 7 and only 2 reach band 1, because an example sentence is
   * three times the length of a word and one jōyō-remainder character is enough to
   * put the whole pattern on the top rung. So this screen offers the corpus's own
   * teaching order instead, and says so.
   *
   * ## The levels are dropped, and the screen says whose order this is not
   *
   * The corpus labels every point N5–N1. None of that is in the artifact — not as a
   * field, not as a tag, and not inside the notes, where 38 cross-references to
   * other patterns by level were removed at build time
   * (`HANDOVER_NIHONGO.md` invariant 20). What the screen claims is the order, not
   * a level, and the header says the JLPT publishes no official grammar list.
   *
   * ## Imported text, and the one promise it replaces
   *
   * Every other reading screen holds its text to the vocabulary: a kanji-bearing
   * token is a word the course teaches, so a tap always has a card behind it
   * (invariant 37). A grammar example is the corpus's own teaching sentence and
   * cannot be — 557 of the 1,526 contain a jōyō-remainder character — so what is
   * promised instead is the corpus's furigana: **a reading over every kanji**,
   * checked at build time and asserted in `tests/grammar_artifact.rs`. The screen
   * states that, and the words in an example are not tappable.
   */
  import GrammarDrill from "./GrammarDrill.svelte";
  import GrammarStage from "./GrammarStage.svelte";
  import * as api from "./api";
  import { untrack } from "svelte";
  import type { GrammarDrillSize, GrammarPage, GrammarPoint } from "./types";
  import type { VoiceStatus } from "./speech";
  import { GRAMMAR_PAGE, GRAMMAR_SEARCH, searchNoteFor, stepTo } from "./grammar";
  import { pageWindow } from "./words";

  interface Props {
    /**
     * What the `voice` command answered.
     *
     * Handed down rather than fetched here, so one missing Japanese voice disables
     * every "Hear it" in the app together and the reason is said once.
     */
    voice: VoiceStatus;
    /**
     * Whether a stage is up, told to `App.svelte` so it can take the course switch
     * and the tab row out of the way while one is (invariants 33 and 35).
     */
    stage?: boolean;
    /**
     * Open a character on the kanji board, handed down to the stage.
     *
     * The board belongs to the kanji course, so this panel cannot draw it: the
     * request travels to `App.svelte` exactly as the radicals screen's does.
     */
    onkanji: (ch: string) => void;
  }

  let { voice, stage = $bindable(false), onkanji }: Props = $props();

  let total = $state(0);
  let page = $state(1);
  let points = $state<GrammarPoint[]>([]);
  /** The query in the box, and the answer to it once one has been asked. */
  let query = $state("");
  let found = $state<GrammarPage | null>(null);
  /** The pattern whose stage is up, and where it sits in the course. */
  let open = $state<{ point: GrammarPoint; position: number } | null>(null);
  /** The position an arrow asked for, waiting for the page that holds it to load. */
  let pendingPosition = $state<number | null>(null);
  /**
   * Whether the drill is up, and what it can ask.
   *
   * The drill is the panel's **second stage**, beside the pattern one of its rows
   * opens: it is about the course rather than about one pattern, so it belongs to
   * this screen rather than to a tab of its own, and it reports `stage` like any
   * other stage so the app's chrome steps aside for it (`nav.ts`'s `STAGE_VIEWS`
   * already names Grammar, so nothing outside this file changes).
   */
  let drilling = $state(false);
  let drillSize = $state<GrammarDrillSize | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const window_ = $derived(pageWindow(total, GRAMMAR_PAGE, page));
  const searching = $derived(query.trim() !== "");
  // The note is the answer's and nothing else's: `found` is cleared the moment the
  // query changes, so neither a stale list nor a stale count is ever drawn against
  // the new words in the box.
  const note = $derived(searchNoteFor(found, query));

  /**
   * Whether each arrow can be followed.
   *
   * The ends are the course's, not the page's: at the last row of a page the next
   * arrow fetches the next page rather than going dead.
   */
  const canPrev = $derived(open !== null && stepTo(open.position, -1, total) !== null);
  const canNext = $derived(open !== null && stepTo(open.position, 1, total) !== null);

  $effect(() => {
    stage = open !== null || drilling;
  });

  async function loadPage() {
    try {
      error = null;
      busy = true;
      const answered = await api.grammarPage(window_.offset, GRAMMAR_PAGE);
      points = answered.points;
      total = answered.total;
      // A position the arrows asked for is opened on whatever page arrived; a page
      // the learner chose opens nothing.
      const wanted = pendingPosition;
      if (wanted !== null) {
        const index = wanted - window_.offset - 1;
        if (index >= 0 && index < points.length) {
          open = { point: points[index], position: wanted };
        }
      }
      pendingPosition = null;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function runSearch(text: string) {
    const asked = text.trim();
    // Dropped before the answer is asked for, so a keystroke never draws the last
    // query's matches or its count beside the new words.
    found = null;
    if (asked === "") {
      return;
    }
    try {
      error = null;
      const answered = await api.grammarSearch(asked, GRAMMAR_SEARCH);
      // The last keystroke counts: a slow answer for a query the learner has moved
      // on from must not land on the box.
      if (untrack(() => query).trim() !== asked) return;
      found = answered;
    } catch (e) {
      error = String(e);
      found = null;
    }
  }

  /**
   * Step along the course order.
   *
   * A step inside the loaded page is immediate; one that crosses a page boundary
   * asks for the page that holds the neighbour and opens it when it arrives. The
   * position is what the step is about, so this works from a search result too —
   * the neighbour is found in the course, not in whatever list is on screen.
   */
  function step(direction: -1 | 1) {
    if (!open) return;
    const target = stepTo(open.position, direction, total, GRAMMAR_PAGE);
    if (!target) return;
    const index = target.position - window_.offset - 1;
    if (target.page === window_.page && index >= 0 && index < points.length) {
      open = { point: points[index], position: target.position };
      return;
    }
    pendingPosition = target.position;
    page = target.page;
  }

  // What the drill can ask, asked for once, so its door can say how much of the
  // course is behind it rather than leaving a learner to wonder.
  $effect(() => {
    void (async () => {
      try {
        drillSize = await api.grammarDrillSize();
      } catch (e) {
        // A count is a courtesy. The course itself still loads, and the panel says
        // nothing about a number it does not have.
        void e;
      }
    })();
  });

  // A page change is what fetches; the arrows only set where to land.
  $effect(() => {
    void window_.offset;
    void window_.limit;
    void loadPage();
  });

  // Choosing a page by hand drops the stage: it belongs to a pattern in the list
  // that is no longer on screen.
  function goToPage(next: number) {
    open = null;
    drilling = false;
    page = next;
  }
</script>

<!--
  Two stages, and either of them is **the whole screen**: the pager and the pattern
  one of its rows opens both step aside for one, exactly as the kana course's lesson
  list and the kanji course's grade chips do (`KanaPanel` and `KanjiPanel` draw their
  stage in this same position, before the course section). The panel's own header goes
  with them — on a phone it is four paragraphs of course preamble, and measured at
  390px it put the drill's own first control at y=371.
-->
{#if drilling}
  <!--
    The drill is a stage of this panel rather than a screen of its own: it asks about
    the course, not about one pattern, and a seventh tab would cost the phone width
    the tab row does not have (invariant 35). Its own first control is the way back,
    as every stage's is.
  -->
  <GrammarDrill {voice} onleave={() => (drilling = false)} />
{:else if open}
  <!--
    Keyed on the pattern, so a second row mounts a fresh stage rather than merging the
    new pattern into the last one's sentences.
  -->
  {#key open.point.id}
    <GrammarStage
      point={open.point}
      position={open.position}
      {total}
      {voice}
      canPrev={canPrev}
      canNext={canNext}
      onstep={step}
      onleave={() => (open = null)}
      {onkanji}
    />
  {/key}
{:else}
  <section class="grammar">
  <header>
    <h2>Grammar</h2>
    <p class="sub">
      {total.toLocaleString()} patterns, each with the form it is built from and two or three
      whole sentences showing it. Every kanji in an example has a reading over it. Read them in
      the order they are given, or search for one.
    </p>
    <p class="hint">
      The order is <strong>the corpus's own teaching order</strong> — OpenJLPT's — and it is
      <strong>not the JLPT's</strong>: the Japan Foundation publishes no official grammar list,
      and the level labels the corpus carries have been dropped rather than shown. This is a
      course order, not a level.
    </p>
    <p class="provenance">
      Patterns and sentences from <a href="https://github.com/evanclan/OpenJLPT" rel="noreferrer"
        >OpenJLPT</a
      >, its contributors' own writing under CC BY-SA 4.0. The furigana is the corpus's and no
      reading here is invented; the notes have had their level references removed, and the app's
      licence screen records that.
    </p>
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

    <!--
      The way into the drill, at the top of the course screen where the learner meets
      it before the list: reading 526 patterns without ever being asked one back is
      what this milestone exists to fix.
    -->
    <div class="practise">
      <button class="start" onclick={() => (drilling = true)}>Practise the patterns</button>
      <span class="count">
        {#if drillSize}
          {drillSize.points} of the {drillSize.total} can be asked
        {:else}
          Ask one back
        {/if}
      </span>
    </div>

    <div class="find">
      <label for="grammar-search">Find a pattern</label>
      <input
        id="grammar-search"
        type="search"
        placeholder="〜てください, tekudasai, request…"
        bind:value={query}
        oninput={(event) => void runSearch(event.currentTarget.value)}
      />
    </div>

    {#if searching}
      {#if note}
        <p class="note" role="status">{note}</p>
      {/if}
      {#if found && found.points.length > 0}
        <ol class="list" aria-label="Matching patterns">
          {#each found.points as point (point.id)}
            <li>
              <button
                class="row"
                onclick={() => (open = { point, position: point.position })}
                title="{point.pattern} — {point.meaning}"
              >
                <span class="number">{point.position}</span>
                <span class="pattern" lang="ja">{point.pattern}</span>
                <span class="romaji">{point.romaji}</span>
                <span class="gloss">{point.meaning}</span>
              </button>
            </li>
          {/each}
        </ol>
      {/if}
    {:else}
      {#if busy}
        <p class="hint">Loading…</p>
      {/if}
      <ol class="list" aria-label="The course, in order">
        {#each points as point, i (point.id)}
          <li>
            <button
              class="row"
              onclick={() => (open = { point, position: window_.offset + i + 1 })}
              title="{point.pattern} — {point.meaning}"
            >
              <span class="number">{window_.offset + i + 1}</span>
              <span class="pattern" lang="ja">{point.pattern}</span>
              <span class="romaji">{point.romaji}</span>
              <span class="gloss">{point.meaning}</span>
            </button>
          </li>
        {/each}
      </ol>

      {#if window_.pages > 1}
        <nav class="pager" aria-label="Pages">
          <button disabled={window_.page <= 1} onclick={() => goToPage(window_.page - 1)}>
            Previous
          </button>
          <span>page {window_.page} of {window_.pages}</span>
          <button disabled={window_.page >= window_.pages} onclick={() => goToPage(window_.page + 1)}>
            Next
          </button>
        </nav>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .grammar header h2 {
    margin: 0;
  }

  .sub {
    margin: 0.2rem 0 0;
    color: var(--muted);
    max-width: 70ch;
  }

  .hint,
  .provenance {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: var(--muted);
    max-width: 70ch;
  }

  .provenance {
    font-size: 0.8rem;
  }

  .practise {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin: 0.9rem 0 0;
  }

  .start {
    min-height: 44px;
    padding: 0.4rem 0.9rem;
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    cursor: pointer;
  }

  .practise .count {
    font-size: 0.85rem;
    color: var(--muted);
  }

  .find {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 1rem 0 0.4rem;
    max-width: 70ch;
  }

  .find label {
    font-size: 0.85rem;
    color: var(--muted);
  }

  .find input {
    flex: 1;
    font: inherit;
    padding: 0.45rem 0.6rem;
    min-height: 44px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    color: var(--ink);
  }

  .note {
    margin: 0.2rem 0 0.6rem;
    font-size: 0.85rem;
    color: var(--muted);
  }

  .list {
    list-style: none;
    margin: 0.4rem 0 0;
    padding: 0;
    display: grid;
    gap: 0.25rem;
    max-width: 90ch;
  }

  .row {
    width: 100%;
    display: grid;
    grid-template-columns: 3rem minmax(6rem, auto) minmax(6rem, auto) 1fr;
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
    border-color: var(--accent);
    background: var(--hover, #f4f4f1);
  }

  .number {
    font-size: 0.75rem;
    color: var(--muted);
  }

  .pattern {
    font-size: 1.1rem;
  }

  .romaji {
    color: var(--muted);
    font-size: 0.9rem;
  }

  .gloss {
    color: var(--muted);
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
    color: var(--muted);
  }

  .pager button {
    font: inherit;
    padding: 0.3rem 0.6rem;
    min-height: 44px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
  }

  .pager button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .error {
    color: var(--bad);
  }
</style>
