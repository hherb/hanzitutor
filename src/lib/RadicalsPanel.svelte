<script lang="ts">
  /**
   * Radicals: the parts the characters are built from.
   *
   * The course teaches characters in frequency order and the character screen
   * looks one up, but neither says *why* a character looks the way it does. The
   * dataset has held each character's radical all along and only printed it. This
   * screen is the teaching half: the radicals the course uses, what each one
   * means, and the characters that share it — which is the grouping that makes a
   * family of unfamiliar characters suddenly readable.
   *
   * ## What is derived, and where
   *
   * Nothing here is stored. `hanzi_core::Dataset::radicals` groups the course's
   * own characters by the radical each already carries, and takes a radical's
   * meaning from that glyph's **own entry** — 言 is a character with the
   * definition "words, speech; speak, say", so the family carries that rather
   * than a second table that could disagree with the character page. The panel
   * owns no IPC of its own; the list is injected, the way the tone screen's is.
   *
   * ## Why the head form and not the shape in the character
   *
   * The radical shown is the **Kangxi head form** the characters are classified
   * under — 言, 人, 水 — while the shape inside the character is a combining form
   * of the same radical: 讠, 亻, 氵. That difference *is* the lesson, so a family
   * is a list of characters that look different and belong together.
   *
   * ## Ranked by what they unlock
   *
   * Families come in order of how many characters they open up, so the screen
   * answers "which parts are worth learning first" rather than listing 214
   * glyphs alphabetically.
   */
  import { onMount } from "svelte";
  import type { RadicalGroup } from "./types";
  import ReadingList from "./ReadingList.svelte";
  import TonedText from "./TonedText.svelte";

  interface Props {
    /** The whole derived list. Injected so the panel owns no IPC of its own. */
    load: () => Promise<RadicalGroup[]>;
    /** Write these characters on the board, in this order. */
    onPractise: (characters: string[]) => void;
    /**
     * The radical whose page is open, owned by the parent so that the character
     * screen can open a family with the "Family" button and land here.
     */
    selected?: string | null;
  }

  let { load, onPractise, selected = $bindable(null) }: Props = $props();

  let families = $state<RadicalGroup[]>([]);
  let loading = $state(true);
  let failure = $state<string | null>(null);
  let query = $state("");

  onMount(() => {
    void (async () => {
      try {
        families = await load();
        failure = null;
      } catch (cause) {
        failure = `Could not work out the radicals: ${cause}`;
      } finally {
        loading = false;
      }
    })();
  });

  /** Fold a query for comparison: case and tone marks are not the point. */
  const fold = (text: string) =>
    text
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase();

  /**
   * The families the search box keeps.
   *
   * Three ways in, because a learner arrives with any of the three: the radical
   * itself (言), what it means (speech), or a character they met and want to take
   * apart (说) — the last finds the family by its members, which is the one that
   * answers "what is this made of".
   */
  const shown = $derived.by(() => {
    const text = query.trim();
    if (text === "") return families;
    const folded = fold(text);
    return families.filter(
      (family) =>
        family.radical === text ||
        family.pinyin.some((reading) => fold(reading).includes(folded)) ||
        fold(family.meaning).includes(folded) ||
        family.characters.some((ch) => ch === text),
    );
  });

  /**
   * The family whose page is open: the selected one, or the best match when
   * nothing has been clicked.
   *
   * Resolving it against the shown list rather than holding the group itself
   * means a search cannot leave a family on screen that is not in the results.
   */
  const open = $derived(
    shown.find((family) => family.radical === selected) ?? shown[0] ?? null,
  );

  /** Clicking a row opens its page, and clears it when it is already open. */
  function select(radical: string) {
    selected = selected === radical ? null : radical;
  }
</script>

<section class="radicals" aria-label="Radicals">
  <div class="toolbar">
    <label class="find">
      <span class="sr-only">Find a radical</span>
      <input
        type="search"
        bind:value={query}
        placeholder="Find a radical by glyph, meaning or a character that uses it"
        autocomplete="off"
        spellcheck="false"
      />
    </label>
    <button onclick={() => (query = "")} disabled={query === ""}>Clear</button>
  </div>

  {#if loading}
    <p class="status">Working out the radicals…</p>
  {:else if failure}
    <p class="warning">{failure}</p>
  {:else if families.length === 0}
    <p class="status">The character set names no radicals.</p>
  {:else}
    <p class="summary">
      {families.length} radicals, {shown.length} shown. Ordered by how many
      characters each one unlocks.
    </p>

    <div class="split">
      <ul class="families">
        {#each shown as family (family.radical)}
          <li class:selected={open?.radical === family.radical}>
            <button
              class="row"
              onclick={() => select(family.radical)}
              title="Show the characters that use this radical"
            >
              <span class="glyph" lang="zh-Hans"><TonedText text={family.radical} /></span>
              <span class="text">
                <span class="reading"><ReadingList readings={family.pinyin} /></span>
                <span class="meaning">{family.meaning || "no meaning recorded"}</span>
              </span>
              <span class="chip" title="Characters in the course that use it">
                {family.characters.length}
              </span>
            </button>
          </li>
        {/each}
      </ul>

      {#if open}
        <aside class="detail" aria-label="Radical details">
          <div class="head">
            <span class="big" lang="zh-Hans"><TonedText text={open.radical} /></span>
            <span class="head-text">
              <span class="detail-pinyin"><ReadingList readings={open.pinyin} /></span>
              <span class="detail-meaning">{open.meaning || "no meaning recorded"}</span>
            </span>
          </div>

          <ul class="facts">
            {#if open.strokeCount > 0}
              <li>{open.strokeCount} {open.strokeCount === 1 ? "stroke" : "strokes"}</li>
            {/if}
            <li>
              {open.characters.length}
              {open.characters.length === 1 ? "character" : "characters"} use it
            </li>
          </ul>

          {#if open.etymology}
            <p class="etymology">{open.etymology}</p>
          {/if}

          <div class="actions">
            <button
              class="primary"
              onclick={() => onPractise([open.radical])}
              title="Write the radical itself on the board"
            >
              Practise the radical
            </button>
            <button
              onclick={() => onPractise(open.characters)}
              title="Write every character that uses it, most common first"
            >
              Practise all {open.characters.length}
            </button>
          </div>

          <div class="members-block">
            <p class="members-title">Characters using this radical</p>
            <ul class="members">
              {#each open.characters as ch (ch)}
                <li>
                  <button
                    class="member"
                    lang="zh-Hans"
                    onclick={() => onPractise([ch])}
                    title="Write {ch} on the board"
                  >
                    <TonedText text={ch} />
                  </button>
                </li>
              {/each}
            </ul>
            <p class="aside-note">
              Each one is written on its own. The characters screen shows what a
              character means before you practise it.
            </p>
          </div>
        </aside>
      {/if}
    </div>
  {/if}
</section>

<style>
  .radicals {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
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
    gap: 6px;
    flex: 1;
    min-width: 240px;
  }
  .find input {
    flex: 1;
    padding: 8px 11px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg);
    font: inherit;
    font-size: 0.88rem;
    color: var(--muted-strong);
  }
  .find input:focus {
    outline: 2px solid var(--accent-soft);
    border-color: var(--accent);
  }

  button {
    padding: 7px 13px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    font: inherit;
    font-size: 0.84rem;
    color: var(--muted-strong);
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  button.primary:hover:not(:disabled) {
    background: var(--accent-ink);
    color: #fff;
  }

  .summary,
  .status {
    margin: 0;
    font-size: 0.78rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .warning {
    margin: 0;
    font-size: 0.82rem;
    color: var(--warning, #b45309);
  }

  /* The index and the open family's page side by side, stacked when there is no
     room for two columns — the same split the character screen uses. */
  .split {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(260px, 0.85fr);
    gap: 16px;
    align-items: start;
    min-height: 0;
  }
  @media (max-width: 860px) {
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
    overflow-y: auto;
    min-height: 0;
  }
  .families li {
    border-radius: 9px;
  }
  .families li:hover {
    background: var(--hover);
  }
  .families li.selected {
    background: var(--accent-soft);
  }

  /* The row is one button, so the whole row opens the family's page. */
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 7px 10px;
    border: 0;
    background: transparent;
    text-align: left;
  }
  .row:hover:not(:disabled) {
    border-color: transparent;
    color: inherit;
  }

  .glyph {
    flex: none;
    width: 2.2rem;
    font-family: var(--hanzi-font);
    font-size: 1.6rem;
    line-height: 1.2;
    color: var(--muted-strong);
    text-align: center;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .reading {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--accent-ink);
  }
  .meaning {
    font-size: 0.78rem;
    color: var(--muted-strong);
  }

  .chip {
    flex: none;
    padding: 2px 7px;
    border: 1px solid var(--line);
    border-radius: 20px;
    background: var(--bg);
    font-size: 0.68rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 13px 14px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .big {
    font-family: var(--hanzi-font);
    font-size: 3rem;
    line-height: 1;
    color: var(--muted-strong);
  }
  .head-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .detail-pinyin {
    font-size: 1rem;
    font-weight: 600;
    color: var(--accent-ink);
  }
  .detail-meaning {
    font-size: 0.82rem;
    color: var(--muted-strong);
  }

  .facts {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    font-size: 0.75rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .etymology {
    margin: 0;
    padding: 8px 10px;
    border-left: 2px solid var(--accent-soft);
    font-size: 0.78rem;
    line-height: 1.5;
    color: var(--muted-strong);
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .actions button {
    padding: 6px 11px;
    font-size: 0.8rem;
  }

  .members-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }
  .members-title {
    margin: 0;
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--muted-strong);
  }
  .members {
    margin: 0;
    padding: 0;
    list-style: none;
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
    font-family: var(--hanzi-font);
    font-size: 1.25rem;
    line-height: 1;
    text-align: center;
  }

  .aside-note {
    margin: 0;
    font-size: 0.72rem;
    line-height: 1.45;
    color: var(--muted);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
    border: 0;
  }
</style>
