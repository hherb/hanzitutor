<script lang="ts">
  /**
   * The kana chart: the whole gojūon grid for one script, and the characters that
   * are not on it.
   *
   * A chart is the one kana screen that is not a lesson. A learner looks something
   * up in it — which column is ゆ in, is that ヌ or ス — and the answer is a shape,
   * so every cell is a button that opens that kana on the practice board rather
   * than a picture of one. The grid arrives with its holes as data (`cells` is
   * five slots, null where the row has no kana), which is what keeps ゆ in the u
   * column instead of under い.
   *
   * The rows are the same table the course is built from, and the off-grid groups
   * are the groups the course adds to it, so this screen cannot offer a kana the
   * lessons do not teach.
   */
  import * as api from "./api";
  import { untrack } from "svelte";
  import { chartColumns } from "./kana";
  import type { ChartView, ScriptName } from "./types";

  let { onopen }: { onopen: (ch: string, script: ScriptName) => void } = $props();

  let script = $state<ScriptName>("hiragana");
  let view = $state<ChartView | null>(null);
  let error = $state<string | null>(null);

  const columns = $derived(view ? chartColumns(view) : 5);

  $effect(() => {
    const which = script;
    api
      .kanaChart(which)
      .then((chart) => {
        // A response the toggle has moved on from is dropped rather than drawn and
        // hidden: two toggles can arrive in either order, and assigning the stale
        // one would leave `view` describing the script the learner left while the
        // template hides it — a permanent "Loading the chart…" with no error and
        // nothing left to re-request it.
        if (untrack(() => script) !== which) return;
        view = chart;
        error = null;
      })
      .catch((e) => {
        if (untrack(() => script) !== which) return;
        error = String(e);
      });
  });
</script>

<section class="chart">
  <header>
    <h2>Kana chart</h2>
    <p class="score">
      {#if view}
        {view.rows.length} rows{view.offGrid.length > 0
          ? ` · ${view.offGrid.reduce((n, group) => n + group.kana.length, 0)} off the grid`
          : ""}
      {/if}
    </p>
  </header>

  <div class="tabs" role="tablist">
    {#each ["hiragana", "katakana"] as const as name (name)}
      <button
        role="tab"
        aria-selected={script === name}
        class:active={script === name}
        onclick={() => (script = name)}>{name}</button
      >
    {/each}
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if view}
    <p class="lede">
      Tap any kana to open it on the board. The voiced rows — dakuten and
      handakuten — are set apart below the plain ones.
    </p>

    <div class="chart-grid" style="--columns:{columns}">
      <!-- The columns are the five vowels, so they are labelled: a chart whose
           columns are unlabelled hides the thing that makes ゆ sit under う. -->
      <div class="row head" aria-hidden="true">
        <span class="sound"></span>
        {#each view.vowels as vowel (vowel)}
          <span class="vowel">{vowel}</span>
        {/each}
      </div>
      {#each view.rows as row (row.sound)}
        <div class="row" class:voiced={row.voiced}>
          <span class="sound">{row.sound}</span>
          {#each row.cells as cell, index (index)}
            {#if cell}
              <button class="cell" onclick={() => onopen(cell as string, script)}>{cell}</button>
            {:else}
              <span class="hole" aria-hidden="true"></span>
            {/if}
          {/each}
        </div>
      {/each}
    </div>

    {#if view.offGrid.length > 0}
      <h3>Off the grid</h3>
      {#each view.offGrid as group (group.key)}
        <div class="group">
          <span class="label">{group.title}</span>
          <div class="strip">
            {#each group.kana as ch (ch)}
              <button class="cell" onclick={() => onopen(ch, script)}>{ch}</button>
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  {:else if !error}
    <p class="hint">Loading the chart…</p>
  {/if}
</section>

<style>
  .chart {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 16px;
    padding: 16px;
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }
  h2 {
    font-size: 1.05rem;
    margin: 0;
  }
  h3 {
    font-size: 0.9rem;
    margin: 18px 0 8px;
    color: var(--muted);
    text-transform: none;
  }
  .score {
    margin: 0;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    font-size: 0.85rem;
  }
  .lede {
    margin: 0 0 14px;
    color: var(--muted);
    font-size: 0.86rem;
    max-width: 68ch;
  }
  .hint {
    color: var(--muted);
    font-size: 0.9rem;
  }
  /* The chart's own box, and deliberately not called `.grid`: `app.css` owns
     that name for the practice picker's `auto-fill` grid, which is shared
     rather than scoped, so it reaches this element too. Applied here it made
     the sixteen rows *its* items — each row one 57px column, five kana crushed
     into two pixels each — because a scoped rule only wins where this component
     writes one, and this box had none. It holds rows, so it is a block. */
  .chart-grid {
    display: block;
  }
  /* One row is a grid of its own, so the label sits outside the five vowel
     columns and every row's cells line up under the same vowels.
     The tracks are capped at the size of a cell rather than left to share the
     row: a chart is looked *up*, so the whole gojūon has to be readable at once
     — stretched across this panel a `1fr` cell is a 170px box with a 24px kana
     in it, and sixteen rows of those are 2900px of scrolling. The cap is the
     off-grid strip's cell below, and on a narrow window the rows still shrink
     to fit rather than overflow, because a capped track is `minmax(0, …)`. */
  .row {
    display: grid;
    grid-template-columns: 2.6rem repeat(var(--columns, 5), minmax(0, 3rem));
    justify-content: start;
    /* The row's own box stops at its last cell, so the voiced band behind a row
       marks that row rather than running on across the empty half of the panel. */
    width: fit-content;
    gap: 6px;
    align-items: center;
    margin-bottom: 6px;
  }
  .row.voiced {
    background: color-mix(in srgb, var(--accent) 7%, transparent);
    border-radius: 10px;
    padding: 3px;
    margin-left: -3px;
    margin-right: -3px;
  }
  .sound {
    color: var(--muted);
    font-size: 0.78rem;
    text-align: right;
    padding-right: 4px;
  }
  .row.head {
    margin-bottom: 2px;
  }
  .vowel {
    color: var(--muted);
    font-size: 0.72rem;
    text-align: center;
    letter-spacing: 0.06em;
  }
  .cell {
    aspect-ratio: 1 / 1;
    display: grid;
    place-items: center;
    font-size: clamp(1.1rem, 2.4vw, 1.5rem);
    line-height: 1;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--bg);
    color: var(--ink);
    cursor: pointer;
    font-family: inherit;
    padding: 0;
  }
  .cell:hover {
    border-color: var(--accent);
  }
  /* A slot the language never filled. It keeps the column, so ゆ stays under う,
     and it is deliberately invisible rather than an empty button. */
  .hole {
    aspect-ratio: 1 / 1;
  }
  .group {
    margin-bottom: 8px;
  }
  .group .label {
    display: block;
    color: var(--muted);
    font-size: 0.78rem;
    margin-bottom: 4px;
  }
  .strip {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .strip .cell {
    width: 3rem;
    aspect-ratio: 1 / 1;
  }
</style>
