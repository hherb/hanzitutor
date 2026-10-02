<script lang="ts">
  /**
   * The discrimination drill.
   *
   * The hard part of kana is not any one character, it is telling シ from ツ and
   * ソ from ン — shapes that differ by stroke *direction*, which no amount of
   * repetition of a single kana teaches. So this asks the other question: given
   * the sound, which of these shapes is it? The wrong answers are the kana that
   * shape is actually confused with, taken from the same set the practice screen
   * uses.
   *
   * The pool comes from the backend, so the drill can never ask about a kana the
   * board could not draw, and a kana that confuses nobody is never asked.
   */
  import * as api from "./api";
  import type { DrillKana } from "./types";

  let pool = $state<DrillKana[]>([]);
  let target = $state<DrillKana | null>(null);
  let options = $state<string[]>([]);
  let picked = $state<string | null>(null);
  let asked = $state(0);
  let right = $state(0);
  let error = $state<string | null>(null);

  /** How many wrong answers to offer beside the right one. */
  const OPTIONS = 4;

  function shuffle<T>(items: T[]): T[] {
    const out = [...items];
    for (let i = out.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [out[i], out[j]] = [out[j], out[i]];
    }
    return out;
  }

  export async function reset() {
    try {
      error = null;
      pool = await api.drillPool();
      asked = 0;
      right = 0;
      next();
    } catch (e) {
      error = String(e);
    }
  }

  export function next() {
    if (pool.length === 0) return;
    const choice = pool[Math.floor(Math.random() * pool.length)];
    target = choice;
    // The right answer plus as many of its partners as there are, capped. Some
    // kana have only one partner (る/ろ), so a four-option question is not always
    // possible and a two-option one is still a real question.
    const wrong = shuffle(choice.partners).slice(0, OPTIONS - 1);
    options = shuffle([choice.ch, ...wrong]);
    picked = null;
  }

  function pick(ch: string) {
    if (picked) return;
    picked = ch;
    asked += 1;
    if (ch === target?.ch) right += 1;
  }

  /** The explanation for the pair the learner just met, whichever way it went. */
  const tell = $derived.by(() => {
    if (!picked || !target) return null;
    // The tell comes from the kana the learner picked, read the other way round,
    // so the sentence describes the pair actually on screen.
    return picked === target.ch
      ? null
      : `${target.ch} is not ${picked}.`;
  });

  void reset();
</script>

<section class="drill">
  <header>
    <h2>Tell them apart</h2>
    <p class="score">
      {right} / {asked}
      {#if asked > 0}· {Math.round((right / asked) * 100)}%{/if}
    </p>
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if target}
    <p class="ask">Which one is <strong>{target.hepburn}</strong>?</p>

    <div class="options">
      {#each options as ch (ch)}
        <button
          class="option"
          class:right={picked !== null && ch === target.ch}
          class:wrong={picked === ch && ch !== target.ch}
          disabled={picked !== null}
          onclick={() => pick(ch)}
        >
          {ch}
        </button>
      {/each}
    </div>

    {#if picked}
      <p class="outcome" class:ok={picked === target.ch}>
        {#if picked === target.ch}
          Yes — {target.ch} is {target.hepburn}.
        {:else}
          {tell} {target.ch} is {target.hepburn}.
        {/if}
      </p>
      <button class="next" onclick={next}>Next</button>
    {:else}
      <p class="hint">Pick the kana that reads {target.hepburn}.</p>
    {/if}
  {:else}
    <p class="hint">Loading the pairs…</p>
  {/if}
</section>

<style>
  .drill {
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
  .score {
    margin: 0;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    font-size: 0.85rem;
  }
  .ask {
    margin: 14px 0 10px;
    font-size: 1.05rem;
  }
  .ask strong {
    font-size: 1.3rem;
    letter-spacing: 0.02em;
  }
  .options {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .option {
    width: 84px;
    height: 84px;
    display: grid;
    place-items: center;
    font-size: 2.4rem;
    line-height: 1;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--bg);
    color: var(--ink);
    cursor: pointer;
    font-family: inherit;
  }
  .option:disabled {
    cursor: default;
  }
  .option.right {
    border-color: var(--ok);
    box-shadow: inset 0 0 0 2px var(--ok);
  }
  .option.wrong {
    border-color: var(--bad);
    box-shadow: inset 0 0 0 2px var(--bad);
    opacity: 0.7;
  }
  .outcome {
    margin: 14px 0 0;
    font-size: 0.9rem;
    color: var(--bad);
  }
  .outcome.ok {
    color: var(--ok);
  }
  .next {
    margin-top: 12px;
    padding: 6px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--line);
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
    cursor: pointer;
    font: inherit;
  }
</style>
