<script lang="ts">
  /**
   * The grammar composition exercise: one pattern, and a sentence of the learner's own.
   *
   * ## Why this exists beside the drill and the written cloze
   *
   * N20 made the course ask a learner to *choose* the pattern a sentence takes and N22
   * made it ask them to *write* it — both inside the corpus's own sentence, with the
   * hole already cut. Neither asks for a sentence the learner thought of, and
   * `ROADMAP_NIHONGO.md`'s known weak spots has carried that as the honest remaining
   * gap since the grammar course shipped. This is the stage that asks.
   *
   * ## There is no answer, so there is no mark
   *
   * The corpus never wrote the sentence being asked for, so nothing can be graded
   * against it. What Rust sends back is a **checklist**: whether the pattern's own
   * text is in the sentence, how many characters it holds, whether all of it could be
   * read as Japanese, which characters this course has no card for, and whether one of
   * the corpus's own sentences was written back. The only decision taken anywhere is
   * whether the input is a sentence at all — and `limitationNote` is the line that says
   * outright what the check cannot see.
   *
   * ## The prompt carries the pattern, which is the opposite of N22
   *
   * The written cloze could not be given its answer. This one cannot avoid giving the
   * pattern, because the pattern *is* the question; the corpus's own sentences travel
   * with it as a model a beginner can borrow the shape of, and the learner's own
   * earlier sentences for the same pattern are drawn above the box so the exercise
   * accumulates writing rather than a score. That is the app's fifth file, read back.
   *
   * ## What is spoken
   *
   * The corpus's own model sentences, and only those: invariant 27 allows a stored
   * sentence to the voice, and a sentence the learner has just composed is neither
   * stored nor verified — this app does not speak text it cannot stand behind.
   */
  import Icon from "./Icon.svelte";
  import SpeakButton from "./SpeakButton.svelte";
  import * as api from "./api";
  import type { CompositionOutcome, CompositionPrompt, CompositionSize } from "./types";
  import type { VoiceStatus } from "./speech";
  import { needsRuby } from "./words";
  import {
    canSubmit,
    checks,
    instructionLine,
    keptLine,
    limitationNote,
    ownNote,
    poolNote,
  } from "./composition";

  interface Props {
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice: VoiceStatus;
    /** Back to the course — the way a stage always leads back to its list. */
    onleave: () => void;
  }

  let { voice, onleave }: Props = $props();

  let prompt = $state<CompositionPrompt | null>(null);
  let outcome = $state<CompositionOutcome | null>(null);
  let typed = $state("");
  let size = $state<CompositionSize | null>(null);
  /** How many sentences this sitting has kept, which is the whole of its scoreboard. */
  let kept = $state(0);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /**
   * A ticket per ask, so an earlier request cannot land on top of a later one.
   *
   * `Next pattern` can be pressed while the answer's round trip is still in flight,
   * and without this the verdict for the last sentence could be drawn under the new
   * pattern — a checklist about a question that is no longer on screen.
   */
  let ticket = 0;

  async function next() {
    const mine = ++ticket;
    try {
      busy = true;
      error = null;
      const drawn = await api.nextGrammarComposition();
      if (mine !== ticket) return;
      prompt = drawn;
      outcome = null;
      typed = "";
    } catch (e) {
      if (mine === ticket) error = String(e);
    } finally {
      if (mine === ticket) busy = false;
    }
  }

  async function submit() {
    const askedPattern = prompt;
    if (!askedPattern || outcome !== null || !canSubmit(typed)) return;
    try {
      busy = true;
      error = null;
      // Recorded against the pattern the prompt named, never against the text: what
      // the app could see is decided in Rust, from the artifact. A failed write does
      // not lose the sentence — it is kept in memory and the warning says so
      // (invariant 15).
      const answered = await api.recordGrammarComposition(askedPattern.point, typed);
      if (prompt !== askedPattern) return;
      outcome = answered;
      if (answered.verdict.kept) kept += 1;
    } catch (e) {
      if (prompt === askedPattern) error = String(e);
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    void (async () => {
      try {
        size = await api.grammarCompositionSize();
      } catch {
        // The count is a courtesy: a stage that cannot say how much of the course can
        // be written about still asks, and the question fetch reports its own error.
      }
    })();
  });

  void next();
</script>

<section class="composition">
  <header class="head">
    <!-- The way back, named for the list it returns to, as every stage's first
         control is. -->
    <button class="back" onclick={onleave} title="Back to the grammar course">
      <Icon name="back" />
      <span>Grammar</span>
    </button>
    <span class="score">{kept === 0 ? "" : kept === 1 ? "1 sentence kept" : `${kept} sentences kept`}</span>
  </header>

  {#if size}
    <p class="pool">{poolNote(size)}</p>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if prompt}
    <p class="ask">{instructionLine(prompt.pattern)}</p>
    <p class="pattern-line" lang="ja">
      {#if prompt.reading}<span class="reading">{prompt.reading}</span>{/if}
      <span class="pattern">{prompt.pattern}</span>
    </p>
    <p class="meaning">{prompt.meaning}</p>
    <p class="formation">
      <span class="label">Written with</span>
      {prompt.formation}
    </p>
    {#if prompt.notes}
      <p class="note">{prompt.notes}</p>
    {/if}

    <!--
      The learner's own writing for this pattern, above the box rather than below it:
      it is the thing this exercise accumulates, and it belongs where they can see it
      while they write the next one.
    -->
    {#if prompt.written.length > 0}
      <div class="own">
        <p class="own-head">{ownNote(prompt.written)}</p>
        <ul class="own-list">
          {#each prompt.written as sentence, i (i)}
            <li lang="ja">{sentence}</li>
          {/each}
        </ul>
      </div>
    {/if}

    <!--
      The corpus's own sentences for this pattern: the model, folded away because a
      learner composing their own sentence should not have one in front of them
      unless they want it. Copying one is noticed by the check rather than punished.
    -->
    <details class="model">
      <summary>How the corpus uses it</summary>
      <ol class="examples">
        {#each prompt.examples as example, i (i)}
          <li>
            <p class="ja" lang="ja">
              {#each example.ruby as part, j (j)}
                {#if needsRuby(part)}<ruby>{part.ruby}<rt>{part.rt}</rt></ruby>{:else}{part.ruby}{/if}
              {/each}
            </p>
            <p class="en" lang="en">{example.en}</p>
            <SpeakButton text={example.ja} {voice} label="Hear it" note={false} />
          </li>
        {/each}
      </ol>
    </details>

    <div class="answer">
      <!-- The label is above the field at every width: measured on the written-cloze
           screen at 390px (N22), a label beside the row pushes the one control below
           the box the learner has just typed in. -->
      <label for="own-sentence">Your sentence, in Japanese</label>
      <div class="row">
        <textarea
          id="own-sentence"
          rows="2"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          placeholder="いぬのほうがねこよりすきです。 ・ inu no houga neko yori suki desu."
          bind:value={typed}
          disabled={outcome !== null}
        ></textarea>
        <button
          class="submit"
          onclick={() => void submit()}
          disabled={outcome !== null || busy || !canSubmit(typed)}>Check</button
        >
      </div>
    </div>

    {#if outcome}
      <ul class="checks">
        {#each checks(outcome.verdict, typed) as line (line.text)}
          <li class={line.state}>{line.text}</li>
        {/each}
      </ul>

      <p class="kept">{keptLine(outcome.verdict, outcome.written.length)}</p>
      {#if outcome.warning}
        <p class="warn" role="status">{outcome.warning}</p>
      {/if}

      <!-- The way on sits directly under the answer, as it does on the other two
           stages: an explanation above this row puts it a scroll away. -->
      <div class="after">
        <button class="next" onclick={next} disabled={busy}>Next pattern</button>
      </div>

      <p class="limitation">{limitationNote()}</p>
    {:else}
      <p class="hint">A sentence of your own — one this app has never seen.</p>
    {/if}
  {:else if !error}
    <p class="hint">{busy ? "Choosing a pattern…" : "Loading…"}</p>
  {/if}
</section>

<style>
  .composition {
    max-width: 70ch;
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

  .score {
    margin-left: auto;
    font-size: 0.85rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .pool {
    margin: 0.2rem 0 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .ask {
    margin: 1rem 0 0.2rem;
    font-size: 0.95rem;
  }

  .pattern-line {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pattern-line .pattern {
    font-size: 1.6rem;
  }

  .pattern-line .reading {
    font-size: 0.85rem;
    color: var(--muted);
  }

  .meaning,
  .formation,
  .note {
    margin: 0.4rem 0 0;
    font-size: 0.9rem;
    color: var(--muted);
  }

  .formation .label {
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.72rem;
    margin-right: 0.35rem;
  }

  .own {
    margin: 0.9rem 0 0;
    padding: 0.6rem 0.8rem;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel);
  }

  .own-head {
    margin: 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .own-list {
    list-style: none;
    margin: 0.35rem 0 0;
    padding: 0;
    display: grid;
    gap: 0.2rem;
    font-size: 1.05rem;
  }

  .model {
    margin: 0.9rem 0 0;
    font-size: 0.9rem;
  }

  .model summary {
    cursor: pointer;
    min-height: 44px;
    display: flex;
    align-items: center;
    color: var(--muted);
  }

  .examples {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.7rem;
  }

  .examples .ja {
    margin: 0;
    font-size: 1.15rem;
    line-height: 2.1;
  }

  .examples .ja rt {
    font-size: 0.65rem;
    color: var(--muted);
  }

  .examples .en {
    margin: 0.1rem 0 0.3rem;
    font-size: 0.9rem;
    color: var(--muted);
  }

  .answer {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 1rem;
  }

  .answer label {
    font-size: 0.85rem;
    color: var(--muted);
  }

  .answer .row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .answer textarea {
    flex: 1;
    min-width: 0;
    min-height: 60px;
    padding: 8px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    resize: vertical;
  }

  .submit {
    min-height: 44px;
    padding: 6px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    cursor: pointer;
  }

  .submit:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .checks {
    list-style: none;
    margin: 1rem 0 0;
    padding: 0;
    display: grid;
    gap: 0.35rem;
    font-size: 0.95rem;
  }

  .checks li {
    padding-left: 1.4rem;
    position: relative;
  }

  .checks li::before {
    position: absolute;
    left: 0;
    top: 0;
  }

  .checks .ok::before {
    content: "✓";
    color: var(--ok);
  }

  .checks .note::before {
    content: "•";
    color: var(--muted);
  }

  .checks .warn::before {
    content: "!";
    color: var(--bad);
  }

  .kept {
    margin: 0.8rem 0 0;
    font-size: 0.95rem;
  }

  .warn {
    margin: 0.4rem 0 0;
    font-size: 0.85rem;
    color: var(--bad);
  }

  .after {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    margin-top: 0.8rem;
  }

  .next {
    min-height: 44px;
    padding: 6px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    cursor: pointer;
  }

  .next:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .limitation {
    margin: 0.8rem 0 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .hint {
    margin: 0.6rem 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .error {
    color: var(--bad);
  }
</style>
