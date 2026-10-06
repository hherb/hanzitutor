<script lang="ts">
  /**
   * The grammar production exercise: the corpus's own sentence with the pattern's
   * own text taken out, and the learner **writes** what is missing.
   *
   * ## Why this exists beside the drill
   *
   * The drill (`GrammarDrill.svelte`) asks the learner to *recognise* a pattern among
   * three; this asks them to *produce* it. `ROADMAP_NIHONGO.md`'s known weak spots had
   * carried the second half as missing since the grammar course shipped — "nothing
   * asks a learner to produce or choose a pattern" — and a learner who can pick
   * 〜の方が out of a row is not yet a learner who can reach for it.
   *
   * ## The question cannot answer itself
   *
   * The recognition drill hands its screen the answer, because the answer is one of
   * its three options and the reveal needs it. A question with no options does not
   * have to: what Rust sends is the sentence in two halves with the hole between them,
   * the corpus's English, and the position of the sentence in its pattern's own list —
   * no pattern, no meaning, no formation, and neither the text taken out nor its
   * reading. So there is nothing here to hide from the learner and nothing to leak,
   * and the lesson — the pattern, its reading, its meaning, its formation, its note,
   * the text that was taken out and what it is read as — arrives with the verdict.
   *
   * ## What is typed, and what is spoken
   *
   * The learner writes romaji or kana; the answer is the **reading the corpus wrote**
   * over the hole (`nihongo_core::input`'s `matches_kana_text` accepts either
   * romanisation, kana typed directly, and the two yotsugana pairs as one sound), and
   * whether it is right is decided in Rust rather than here. `Hear it` appears with
   * the verdict and speaks the sentence whole: before an answer it would read out the
   * very pattern the hole took away (invariant 27's rule about what may be spoken,
   * and N20's about when).
   */
  import Icon from "./Icon.svelte";
  import SpeakButton from "./SpeakButton.svelte";
  import * as api from "./api";
  import type {
    GrammarDrillSize,
    GrammarProductionOutcome,
    GrammarProductionQuestion,
  } from "./types";
  import type { VoiceStatus } from "./speech";
  import { needsRuby } from "./words";
  import { recordLine, sessionLine } from "./grammarDrill";
  import { canSubmit, outcomeLine, poolNote, wholeSentence } from "./grammarProduction";

  interface Props {
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice: VoiceStatus;
    /** Back to the course — the way a stage always leads back to its list. */
    onleave: () => void;
  }

  let { voice, onleave }: Props = $props();

  let question = $state<GrammarProductionQuestion | null>(null);
  let outcome = $state<GrammarProductionOutcome | null>(null);
  let typed = $state("");
  let size = $state<GrammarDrillSize | null>(null);
  let asked = $state(0);
  let right = $state(0);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /**
   * A ticket per ask, so an earlier request cannot land on top of a later one.
   *
   * `Next question` can be pressed while the answer's round trip is still in flight,
   * and without the identity check in `submit` the verdict for the last question could
   * be drawn under the new sentence — a lesson about a pattern that is not on screen.
   */
  let ticket = 0;

  async function next() {
    const mine = ++ticket;
    try {
      busy = true;
      error = null;
      const drawn = await api.nextGrammarProduction();
      if (mine !== ticket) return;
      question = drawn;
      outcome = null;
      typed = "";
    } catch (e) {
      if (mine === ticket) error = String(e);
    } finally {
      if (mine === ticket) busy = false;
    }
  }

  async function submit() {
    const askedQuestion = question;
    if (!askedQuestion || outcome !== null || !canSubmit(typed)) return;
    try {
      busy = true;
      error = null;
      // Recorded against the pattern the question named and the sentence's position
      // in it — never against the text, which this screen was not given. A failed
      // write does not undo the answer: the verdict still arrives, and the message
      // says what could not be saved (invariant 15).
      const answered = await api.recordGrammarProduction(
        askedQuestion.point,
        askedQuestion.example,
        typed,
      );
      if (question !== askedQuestion) return;
      outcome = answered;
      asked += 1;
      if (answered.verdict.correct) right += 1;
    } catch (e) {
      if (question === askedQuestion) error = String(e);
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    void (async () => {
      try {
        size = await api.grammarProductionSize();
      } catch {
        // The count is a courtesy: a screen that cannot say how much can be written
        // still asks, and the error is not repeated here — the question fetch reports
        // one.
      }
    })();
  });

  void next();
</script>

<section class="production">
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
      The sentence with the pattern taken out, in the two halves it arrived in with
      the corpus's own furigana on each. The gap between them is the question, and once
      it is answered the text that was taken out goes back into it.
    -->
    <p class="sentence" lang="ja">
      {#each question.before as part, i (i)}
        {#if needsRuby(part)}<ruby>{part.ruby}<rt>{part.rt}</rt></ruby>{:else}{part.ruby}{/if}
      {/each}
      {#if outcome}
        <mark class="filled">{outcome.verdict.written}</mark>
      {:else}
        <span class="blank" aria-label="the text that is missing">？</span>
      {/if}
      {#each question.after as part, i (i)}
        {#if needsRuby(part)}<ruby>{part.ruby}<rt>{part.rt}</rt></ruby>{:else}{part.ruby}{/if}
      {/each}
    </p>
    <p class="english" lang="en">{question.en}</p>

    <div class="answer">
      <label for="written">Write the missing text</label>
      <!-- The field and the button are one row at every width: measured at 390px,
           the label beside them pushed `Check` onto a line of its own, which put the
           one control the question has below the box the learner had just typed in. -->
      <div class="row">
        <input
          id="written"
          type="text"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          placeholder="nohouga ・ のほうが"
          bind:value={typed}
          disabled={outcome !== null}
          onkeydown={(event) => event.key === "Enter" && void submit()}
        />
        <button
          class="submit"
          onclick={() => void submit()}
          disabled={outcome !== null || busy || !canSubmit(typed)}>Check</button
        >
      </div>
    </div>

    {#if outcome}
      <p class="outcome">{outcomeLine(outcome.verdict)}</p>

      <!--
        The way on sits where the answer was — N16's rule for the boards, and N20's
        measurement for this screen: an explanation above this row puts `Next question`
        a scroll away from the answer the learner has just given.
      -->
      <div class="after">
        <button class="next" onclick={next} disabled={busy}>Next question</button>
        <!-- Only now: before the answer, the voice would read out the pattern the
             blank took away. -->
        <SpeakButton
          text={wholeSentence(question, outcome.verdict.written)}
          {voice}
          label="Hear it"
          note={false}
        />
      </div>

      <div class="reveal">
        <p class="answer-head" lang="ja">
          {#if outcome.verdict.reading}<span class="reading">{outcome.verdict.reading}</span>{/if}
          <span class="pattern">{outcome.verdict.pattern}</span>
        </p>
        <p class="meaning">{outcome.verdict.meaning}</p>
        <p class="formation">
          <span class="label">Written with</span>
          {outcome.verdict.formation}
        </p>
        {#if outcome.verdict.notes}
          <p class="note">{outcome.verdict.notes}</p>
        {/if}
      </div>

      <p class="record">{recordLine(outcome.record, outcome.verdict.pattern)}</p>
    {:else}
      <p class="hint">
        Which pattern does this sentence take? Write what is missing — romaji or kana.
      </p>
    {/if}
  {:else if !error}
    <p class="hint">{busy ? "Drawing a question…" : "Loading…"}</p>
  {/if}
</section>

<style>
  .production {
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

  /* The hole: a rule the eye lands on rather than a box, because a box would suggest
     how long the answer is. */
  .blank {
    display: inline-block;
    min-width: 3.5rem;
    text-align: center;
    border-bottom: 2px solid var(--accent);
    color: var(--muted);
    font-size: 1rem;
  }

  .filled {
    background: none;
    color: var(--ink);
    border-bottom: 2px solid var(--ok);
  }

  .english {
    margin: 0 0 0.8rem;
    color: var(--muted);
    font-size: 0.95rem;
  }

  .answer {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .answer label {
    font-size: 0.85rem;
    color: var(--muted);
  }

  .answer .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .answer input {
    flex: 1;
    min-width: 0;
    min-height: 44px;
    padding: 6px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
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

  .reveal .answer-head {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .reveal .answer-head .pattern {
    font-size: 1.5rem;
  }

  .reveal .reading {
    font-size: 0.85rem;
    color: var(--muted);
  }

  .meaning,
  .formation,
  .note {
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
