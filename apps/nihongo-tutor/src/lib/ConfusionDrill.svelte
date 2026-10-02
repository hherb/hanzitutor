<script lang="ts">
  /**
   * The discrimination drill — and the only thing in the app that remembers
   * anything about this learner.
   *
   * The hard part of kana is not any one character, it is telling シ from ツ and
   * ソ from ン: shapes that differ by stroke *direction*, which no amount of
   * repetition of a single kana teaches. So this asks the other question — given
   * the sound, which of these two shapes is it?
   *
   * The question comes from the backend, which chooses the pair, weights it by how
   * often this learner has got it wrong, and records the answer against it. The
   * component used to draw from a pool the backend handed it, which cannot work
   * once answers are remembered: only the asker knows which pair it asked. The
   * rule itself is in `nihongo_core::drill`, and it is told to the learner below
   * the answer rather than left as a mystery.
   */
  import * as api from "./api";
  import type { DrillQuestion, DrillTally } from "./types";

  let question = $state<DrillQuestion | null>(null);
  /** The kana on screen, in display order: the pair, shuffled. */
  let options = $state<string[]>([]);
  let picked = $state<string | null>(null);
  let tally = $state<DrillTally | null>(null);
  let asked = $state(0);
  let right = $state(0);
  let error = $state<string | null>(null);
  let busy = $state(false);

  function shuffle<T>(items: T[]): T[] {
    const out = [...items];
    for (let i = out.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [out[i], out[j]] = [out[j], out[i]];
    }
    return out;
  }

  /** Ask for the next question. */
  export async function next() {
    try {
      error = null;
      busy = true;
      // Named `drawn` and not `asked`, which is the session's score above.
      const drawn = await api.nextDrillQuestion();
      question = drawn;
      // Two kana, so the answer is never in the same place twice — which would
      // let a learner learn the position instead of the shape.
      options = drawn ? shuffle(drawn.options) : [];
      picked = null;
      tally = null;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function pick(ch: string) {
    if (picked !== null || !question) return;
    picked = ch;
    asked += 1;
    if (ch === question.ch) right += 1;
    try {
      // Recorded against the pair the question named, which is why the question
      // carries it. Failing to record does not undo the answer.
      tally = await api.recordDrillAnswer(question.pair, question.ch, ch);
    } catch (e) {
      error = String(e);
    }
  }

  void next();
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

  {#if question}
    <p class="ask">Which one is <strong>{question.hepburn}</strong>?</p>

    <div class="options">
      {#each options as ch (ch)}
        <button
          class="option"
          class:right={picked !== null && ch === question.ch}
          class:wrong={picked === ch && ch !== question.ch}
          disabled={picked !== null}
          onclick={() => pick(ch)}
        >
          {ch}
        </button>
      {/each}
    </div>

    {#if picked}
      <p class="outcome" class:ok={picked === question.ch}>
        {#if picked === question.ch}
          Yes — {question.ch} is {question.hepburn}.
        {:else}
          {question.ch} is {question.hepburn}, not {picked}. {question.tell}.
        {/if}
      </p>
      {#if tally}
        <p class="record">
          {#if picked === question.ch}
            {tally.pair}: {tally.correct} of {tally.asked} right — this pair comes up
            less often now.
          {:else}
            {tally.pair}: missed {tally.wrong} of {tally.asked} — this pair comes up
            more often until you stop missing it.
          {/if}
        </p>
      {/if}
      <button class="next" onclick={next} disabled={busy}>Next</button>
    {:else}
      <p class="hint">Pick the kana that reads {question.hepburn}.</p>
    {/if}
  {:else if !error}
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
  .record {
    margin: 8px 0 0;
    color: var(--muted);
    font-size: 0.8rem;
    max-width: 62ch;
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
  .next:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
