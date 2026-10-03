<script lang="ts">
  /**
   * The radicals: the 214 Kangxi head forms, and the characters that share each.
   *
   * ## Both shapes of a radical are here, and the difference is the lesson
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
   * ## Why the panel owns no board
   *
   * A member and a head form both go to the **kanji screen's** board: the family
   * list asks the course for a character, and the head form is asked for by
   * number. One board, one grading path, three kinds of thing — see
   * `KanjiPanel.svelte`.
   */
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
  }

  let { focus = $bindable(null), onopen, onpractise }: Props = $props();

  let families = $state<RadicalFamilyView[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let query = $state("");
  let order = $state<RadicalOrder>("size");
  let selected = $state<number | null>(null);

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
   * The family whose page is open: the selected one, or the first of the results.
   *
   * Resolved against the *shown* list rather than held as a group, so a search
   * cannot leave a family on screen that is not in the results.
   */
  const open = $derived(shown.find((family) => family.number === selected) ?? shown[0] ?? null);
  const inUse = $derived(families.filter((family) => family.characters.length > 0).length);

  function choose(number: number) {
    selected = selected === number ? null : number;
  }

  /**
   * A family another screen asked for, opened once.
   *
   * The list is searched by number rather than by position, because the panel can
   * be showing either order — and a request that arrived while the size order was
   * on has to open the same family it would have in number order.
   */
  $effect(() => {
    const wanted = focus;
    if (wanted === null || families.length === 0) return;
    focus = null;
    selected = wanted;
  });
</script>

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
      shown is the whole radical; the shape inside the character is a variant of it.
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

    <div class="split">
      <ul class="families">
        {#each shown as family (family.number)}
          <li class:selected={open?.number === family.number}>
            <button
              class="row"
              onclick={() => choose(family.number)}
              title="Show the characters that use this radical"
            >
              <span class="glyph">{family.ch}</span>
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

      {#if open}
        <aside class="detail" aria-label="Radical details">
          <div class="head">
            <span class="big">{open.ch}</span>
            <span class="head-text">
              <span class="detail-number">Radical {open.number} of 214</span>
              <span class="detail-facts">
                {open.strokeCount} {open.strokeCount === 1 ? "stroke" : "strokes"} ·
                {open.characters.length} {open.characters.length === 1 ? "character" : "characters"}
              </span>
            </span>
          </div>

          <div class="actions">
            <button
              class="primary"
              onclick={() => onpractise?.(open.number)}
              title="Write the head form itself on the board"
            >
              Write {open.ch} on the board
            </button>
          </div>

          {#if open.characters.length > 0}
            <p class="members-title">Characters using this radical, most frequent first</p>
            <ul class="members">
              {#each open.characters as ch (ch)}
                <li>
                  <button
                    class="member"
                    onclick={() => onopen?.(ch)}
                    title="Open {ch} on the board"
                  >
                    {ch}
                  </button>
                </li>
              {/each}
            </ul>
            <p class="aside-note">
              Each one is a jōyō character the course teaches, and opens on the same board
              the kana are written on.
            </p>
          {:else}
            <p class="aside-note">
              No jōyō character uses this radical, so there is no family to show — the head
              form itself can still be written on the board.
            </p>
          {/if}
        </aside>
      {/if}
    </div>
  {/if}
</section>

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

  .split {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(280px, 0.85fr);
    gap: 16px;
    align-items: start;
    min-height: 0;
  }

  @media (max-width: 880px) {
    .split {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .families {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 62vh;
    overflow-y: auto;
  }

  .families li {
    border-radius: var(--radius);
  }

  .families li.selected {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
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

  .detail {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 13px 14px;
    border: 1px solid var(--line);
    border-radius: 16px;
    background: var(--panel);
    min-width: 0;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .big {
    font-size: 3rem;
    line-height: 1;
  }

  .head-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .detail-number {
    font-weight: 600;
  }

  .detail-facts {
    font-size: 0.8rem;
    color: var(--muted);
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .actions button {
    padding: 6px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
  }

  .actions button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }

  .members-title {
    margin: 0;
    font-size: 0.78rem;
    font-weight: 600;
  }

  .members {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    max-height: 16rem;
    overflow-y: auto;
  }

  .member {
    width: 2.3rem;
    height: 2.3rem;
    padding: 0;
    font-size: 1.3rem;
    line-height: 1;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
  }

  .member:hover {
    border-color: var(--accent);
  }

  .aside-note {
    margin: 0;
    font-size: 0.74rem;
    line-height: 1.45;
    color: var(--muted);
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
</style>
