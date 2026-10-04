<script lang="ts">
  /**
   * The radicals: the course screen — all 214 head forms, searchable and ordered
   * by what they unlock — and the stage that one of them opens.
   *
   * ## Two screens, and this is the first one
   *
   * The panel used to be one screen: the search box, the list of 214 and the
   * chosen family's details in a second column. That is N12's complaint one screen
   * further out — the thing the learner came for (the family, the head form, the
   * characters it unlocks) sat *below* or beside the list, so on a phone a tap
   * meant scrolling past the course to read it and back to choose the next
   * radical. `RadicalStage` is the second screen, the chrome steps aside while one
   * is up, and `HANDOVER_NIHONGO.md` invariants 33 and 34 are the rule this
   * follows.
   *
   * ## Both shapes of a radical are shown, and the difference is the lesson
   *
   * A radical has a **head form** — 手, the one it is taught and listed as — and a
   * **combining form** written inside a character: 扌 in 持, 氵 in 池. The
   * characters are grouped by their classical number, so 持 belongs to 手's family
   * even though nothing in 持 looks like 手. That is the whole point of learning
   * radicals: a family is a list of characters that look nothing like each other
   * and turn out to belong together.
   *
   * ## All 214, including the sixteen nothing uses
   *
   * The jōyō set reaches 198 of the 214. The other sixteen — 爿 瓜 禸 … 龍 龜 龠 —
   * are listed with an empty family, because a learner looking up 黹 should find
   * it and be told that no jōyō character uses it, rather than find nothing and
   * conclude the panel is broken.
   *
   * ## Why neither screen owns a board
   *
   * A member and a head form both go to the **kanji screen's** board: the family
   * list asks the course for a character, and the head form is asked for by
   * number. One board, one grading path, three kinds of thing — see
   * `KanjiPanel.svelte`. So this pair of screens is two screens without a board,
   * which is the part that is *not* invariant 33's arrangement: the stage draws the
   * family and hands the writing next door, rather than grading it here.
   */
  import RadicalStage from "./RadicalStage.svelte";
  import { findFamilies, sortFamilies, type RadicalOrder } from "./kanji";
  import * as api from "./api";
  import type { RadicalFamilyView } from "./types";

  interface Props {
    /**
     * The family to open, asked for by the kanji screen's "See the family". It is
     * consumed here — cleared once it is on screen — so leaving and returning to
     * the panel does not reopen something the learner has moved on from.
     */
    focus?: number | null;
    /** Write this jōyō character on the kanji board. */
    onopen?: (ch: string) => void;
    /** Write this radical's head form on the kanji board. */
    onpractise?: (number: number) => void;
    /**
     * Whether a stage is up, told to `App.svelte` so it can take the course switch
     * and the tab row out of the way while one is — invariant 33's arrangement,
     * applied to the screens that are not boards.
     */
    stage?: boolean;
  }

  let { focus = $bindable(null), onopen, onpractise, stage = $bindable(false) }: Props =
    $props();

  let families = $state<RadicalFamilyView[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let query = $state("");
  let order = $state<RadicalOrder>("size");
  /** The family whose stage is up, or `null` while the course itself is showing. */
  let open = $state<number | null>(null);

  async function load() {
    try {
      families = await api.radicals();
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
  void load();

  const shown = $derived(sortFamilies(findFamilies(families, query), order));
  /**
   * The family on the stage, resolved by number against what was loaded.
   *
   * By number rather than by position, because the list can be showing either
   * order and can be filtered: a stage that held an index would open a different
   * radical the moment the order changed.
   */
  const opened = $derived(families.find((family) => family.number === open) ?? null);
  const inUse = $derived(families.filter((family) => family.characters.length > 0).length);

  /**
   * Keep the app told whether a stage is up.
   *
   * One place rather than three: `open` is set by a row, by a request from the
   * kanji screen and by leaving, and a flag written at each of those would be wrong
   * the day a fourth caller arrives. The effect reads `open` and writes the prop,
   * which the app never writes back — so there is nothing here to loop.
   */
  $effect(() => {
    stage = open !== null;
  });

  /**
   * A family another screen asked for, opened once.
   *
   * It waits for the list before consuming the request — the family has to be a
   * real one before a stage can name it — and the list is searched by number rather
   * than by position, because the panel can be showing either order.
   */
  $effect(() => {
    const wanted = focus;
    if (wanted === null || families.length === 0) return;
    focus = null;
    open = wanted;
  });
</script>

{#if opened}
  <!--
    Keyed on the radical it was opened on, so a request from the kanji screen mounts
    a fresh stage rather than merging the new family into the state of the old one.
  -->
  {#key opened.number}
    <RadicalStage radical={opened} {onopen} {onpractise} onleave={() => (open = null)} />
  {/key}
{:else}
  <section class="radicals">
    <header>
      <h2>Radicals</h2>
      <p class="sub">
        {families.length > 0 ? `${inUse} of ${families.length}` : "The 214"} Kangxi radicals are
        used by the jōyō set; the rest are listed so the set is complete.
      </p>
      <p class="hint">
        A family is grouped by the radical's <strong>classical number</strong>, which is
        KANJIDIC2's classification, so 持 (written 扌) and 手 are one family. The head form
        shown is the whole radical; the shape inside the character is a variant of it. Choose
        a radical to see its family and to write it on the board.
      </p>
    </header>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <div class="toolbar">
      <label class="find">
        <span class="sr-only">Find a radical</span>
        <input
          type="search"
          bind:value={query}
          placeholder="Find a radical by glyph, number, or a character that uses it"
          autocomplete="off"
          spellcheck="false"
        />
      </label>
      <button onclick={() => (query = "")} disabled={query === ""}>Clear</button>
      <div class="order" role="group" aria-label="Order">
        <button class:active={order === "size"} onclick={() => (order = "size")}>
          Biggest first
        </button>
        <button class:active={order === "number"} onclick={() => (order = "number")}>
          By number
        </button>
      </div>
    </div>

    {#if loading}
      <p class="hint">Loading the 214…</p>
    {:else if families.length === 0}
      <p class="hint">The artifact names no radicals.</p>
    {:else}
      <p class="summary">{shown.length} of {families.length} shown.</p>

      <ul class="families" aria-label="The 214 radicals">
        {#each shown as family (family.number)}
          <li>
            <button
              class="row"
              onclick={() => (open = family.number)}
              title="Show the family of {family.ch} and open it on the board"
            >
              <span class="glyph" lang="ja">{family.ch}</span>
              <span class="text">
                <span class="number">radical {family.number}</span>
                <span class="count">
                  {family.characters.length === 0
                    ? "no jōyō character uses it"
                    : `${family.characters.length} ${family.characters.length === 1 ? "character" : "characters"}`}
                </span>
              </span>
              <span class="chip" title="How many characters it unlocks">
                {family.characters.length}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
  .radicals {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
  }

  header h2 {
    margin: 0;
  }

  .sub {
    margin: 0.2rem 0 0;
    color: var(--muted);
  }

  .hint {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: var(--muted);
    max-width: 76ch;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .find {
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 240px;
  }

  .find input {
    width: 100%;
    padding: 7px 11px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: 0.88rem;
  }

  .toolbar button {
    padding: 6px 12px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
    font-size: 0.84rem;
  }

  .toolbar button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .order {
    display: flex;
    gap: 4px;
  }

  .order button.active {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
  }

  .summary {
    margin: 0;
    font-size: 0.78rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  /* The 214, as a list of cards to choose from rather than a column beside a
     detail pane — the detail is a screen of its own now. */
  .families {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 2px 10px;
  }

  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 10px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: none;
    color: var(--ink);
    text-align: left;
    cursor: pointer;
    font: inherit;
  }

  .row:hover {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--line) 40%, transparent);
  }

  .glyph {
    flex: none;
    width: 2rem;
    font-size: 1.5rem;
    line-height: 1.2;
    text-align: center;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .number {
    font-size: 0.86rem;
    font-variant-numeric: tabular-nums;
  }

  .count {
    font-size: 0.76rem;
    color: var(--muted);
  }

  .chip {
    flex: none;
    padding: 1px 8px;
    border: 1px solid var(--line);
    border-radius: 999px;
    font-size: 0.7rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
    border: 0;
  }

  .error {
    color: var(--bad);
  }
</style>
