<script lang="ts">
  /**
   * The grammar drill: the course's own sentence with its pattern taken out.
   *
   * ## What it asks, and why that is a question about a pattern
   *
   * The corpus wrote two or three sentences for every pattern, and this shows one of
   * them with the pattern's own text removed and three patterns to choose between.
   * Nothing is invented to build it: the sentence, its English, the furigana over it
   * and the options are the artifact's, and the answer is the point the sentence
   * belongs to. The rule that finds the blank — which lump of the pattern, how long,
   * once only, never through a furigana pair — is `nihongo_core::grammar_drill`'s and
   * is measured there (363 of the 526 patterns, over 903 sentences).
   *
   * ## The question is drawn in Rust, and judged there
   *
   * Which pattern comes up is weighted by what this learner has missed, so the draw
   * cannot be a decision of this component's: only the asker knows which pattern it
   * asked, and the answer is remembered against it. The same goes for rightness —
   * this screen posts the pattern asked about and the one that was picked, and
   * `record_grammar_answer` decides. What this file knows is how to *draw* a judged
   * question, which is `grammarDrill.ts`'s arithmetic.
   *
   * ## What is spoken, and when
   *
   * The sentence, and only **after** it has been answered: before that, the voice
   * would read out the very pattern the blank took away. It is the whole sentence as
   * the corpus wrote it (invariant 37's exception, and N19's own rule), never a
   * pattern — `〜に対して` is not something a synthesiser should be asked to read.
   */
  import Icon from "./Icon.svelte";
  import SpeakButton from "./SpeakButton.svelte";
  import * as api from "./api";
  import type { GrammarDrillSize, GrammarQuestion, GrammarTally } from "./types";
  import type { VoiceStatus } from "./speech";
  import { needsRuby } from "./words";
  import { optionState, outcomeLine, poolNote, recordLine, sessionLine } from "./grammarDrill";

  interface Props {
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice: VoiceStatus;
    /** Back to the course — the way a stage always leads back to its list. */
    onleave: () => void;
  }

  let { voice, onleave }: Props = $props();

  let question = $state<GrammarQuestion | null>(null);
  let picked = $state<string | null>(null);
  let tally = $state<GrammarTally | null>(null);
  let size = $state<GrammarDrillSize | null>(null);
  let asked = $state(0);
  let right = $state(0);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /**
   * A ticket per ask, so an earlier request cannot land on top of a later one.
   *
   * `Next` can be pressed twice while the first round trip is still in flight, and
   * without this the first answer could be drawn under the second question — a
   * sentence with the wrong options beside it. The answer would still be recorded
   * against the pattern its own question named, so only the screen would be wrong,
   * which is the kind of wrong a learner reads as the app being broken.
   */
  let ticket = 0;

  async function next() {
    const mine = ++ticket;
    try {
      busy = true;
      error = null;
      const drawn = await api.nextGrammarQuestion();
      if (mine !== ticket) return;
      question = drawn;
      picked = null;
      tally = null;
    } catch (e) {
      if (mine === ticket) error = String(e);
    } finally {
      if (mine === ticket) busy = false;
    }
  }

  async function pick(id: string) {
    if (picked !== null || !question) return;
    picked = id;
    asked += 1;
    if (id === question.point) right += 1;
    try {
      // Recorded against the pattern the question named, which is why the question
      // carries it. A failed write does not undo the answer: the screen still shows
      // the verdict, and the message says what could not be saved (invariant 15).
      tally = await api.recordGrammarAnswer(question.point, id);
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    void (async () => {
      try {
        size = await api.grammarDrillSize();
      } catch {
        // The count is a courtesy: a screen that cannot say how much is askable still
        // asks, and the error is not repeated here — the question fetch reports one.
      }
    })();
  });

  void next();
</script>

<section class="drill">
  <header class="head">
    <!-- The way back, named for the list it returns to, as every stage's first
         control is. -->
    <button class="back" onclick={onleave} title="Back to the grammar course">
      <Icon name="back" />
      <span>Grammar</span>
    </button>
    <span class="score">{sessionLine(right, asked)}</span>
  </header>

  {#if size}
    <p class="pool">{poolNote(size)}</p>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if question}
    <!--
      The sentence with the pattern taken out. It is handed over in two halves with
      the corpus's own furigana on each, and the gap between them is the question.
    -->
    <p class="sentence" lang="ja">
      {#each question.before as part, i (i)}
        {#if needsRuby(part)}<ruby>{part.ruby}<rt>{part.rt}</rt></ruby>{:else}{part.ruby}{/if}
      {/each}
      <span class="blank" aria-label="the pattern that is missing">？</span>
      {#each question.after as part, i (i)}
        {#if needsRuby(part)}<ruby>{part.ruby}<rt>{part.rt}</rt></ruby>{:else}{part.ruby}{/if}
      {/each}
    </p>
    <p class="english" lang="en">{question.en}</p>

    <div class="options">
      {#each question.options as option (option.id)}
        <button
          class="option"
          class:right={optionState(option, question, picked) === "right"}
          class:wrong={optionState(option, question, picked) === "wrong"}
          disabled={picked !== null}
          lang="ja"
          onclick={() => pick(option.id)}
        >
          <span class="pattern">{option.pattern}</span>
          {#if option.reading}
            <span class="reading">{option.reading}</span>
          {/if}
        </button>
      {/each}
    </div>

    {#if picked}
      <p class="outcome">{outcomeLine(question, picked)}</p>

      <!--
        The way on sits where the answer was, which is N16's rule for the boards
        applied here: measured at 390px, an explanation above this row put `Next
        question` at y=871 on an 812px screen — a scroll away from the answer the
        learner had just given.
      -->
      <div class="after">
        <button class="next" onclick={next} disabled={busy}>Next question</button>
        <!-- Only now: before the answer, the voice would read out the pattern the
             blank took away. -->
        <SpeakButton text={question.ja} {voice} label="Hear it" note={false} />
      </div>

      <div class="reveal">
        <p class="answer" lang="ja">
          {#if question.reading}<span class="reading">{question.reading}</span>{/if}
          <span class="pattern">{question.pattern}</span>
        </p>
        <p class="formation"><span class="label">Written with</span> {question.formation}</p>
        {#if question.notes}
          <p class="note">{question.notes}</p>
        {/if}
        <!-- The sentence whole again, with the pattern's own lump back in the hole
             it came out of and the corpus's furigana still on the kanji around it. -->
        <p class="whole" lang="ja">
          {#each question.before as part, i (i)}
            {#if needsRuby(part)}<ruby>{part.ruby}<rt>{part.rt}</rt></ruby>{:else}{part.ruby}{/if}
          {/each}
          <mark class="filled">{question.blank}</mark>
          {#each question.after as part, i (i)}
            {#if needsRuby(part)}<ruby>{part.ruby}<rt>{part.rt}</rt></ruby>{:else}{part.ruby}{/if}
          {/each}
        </p>
      </div>

      {#if tally}
        <p class="record">{recordLine(tally, question.pattern)}</p>
      {/if}
    {:else}
      <p class="hint">Which pattern does this sentence take?</p>
    {/if}
  {:else if !error}
    <p class="hint">{busy ? "Drawing a question…" : "Loading…"}</p>
  {/if}
</section>

<style>
  .drill {
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

  .sentence {
    margin: 1rem 0 0.25rem;
    font-size: 1.35rem;
    line-height: 2.3;
  }

  .sentence rt {
    font-size: 0.7rem;
    color: var(--muted);
  }

  /* The hole. Drawn as a rule the eye lands on rather than a box, because the answer
     is a pattern of four or five characters and a full-width box would suggest its
     length. */
  .blank {
    display: inline-block;
    min-width: 3.5rem;
    text-align: center;
    border-bottom: 2px solid var(--accent);
    color: var(--muted);
    font-size: 1rem;
  }

  .english {
    margin: 0 0 0.8rem;
    color: var(--muted);
    font-size: 0.95rem;
  }

  .options {
    display: grid;
    gap: 8px;
  }

  .option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    min-height: 44px;
    padding: 8px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .option:not(:disabled):hover {
    border-color: var(--accent);
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
    opacity: 0.75;
  }

  .option .pattern {
    font-size: 1.2rem;
  }

  .option .reading,
  .reveal .reading {
    font-size: 0.85rem;
    color: var(--muted);
  }

  .outcome {
    margin: 1rem 0 0;
    font-size: 1rem;
  }

  .reveal {
    margin-top: 0.6rem;
    padding: 0.7rem 0.8rem;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel);
  }

  .reveal .answer {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .reveal .answer .pattern {
    font-size: 1.5rem;
  }

  .formation,
  .note,
  .whole {
    margin: 0.5rem 0 0;
    font-size: 0.9rem;
    color: var(--muted);
  }

  .formation .label {
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.72rem;
    margin-right: 0.35rem;
  }

  .whole {
    font-size: 1rem;
    color: var(--ink);
    line-height: 2.1;
  }

  .whole rt {
    font-size: 0.7rem;
    color: var(--muted);
  }

  .whole .filled {
    background: none;
    color: var(--ink);
    border-bottom: 2px solid var(--ok);
  }

  .record {
    margin: 0.6rem 0 0;
    font-size: 0.8rem;
    color: var(--muted);
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

  .hint {
    margin: 0.6rem 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .error {
    color: var(--bad);
  }
</style>
