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
   * Since N4 it asks that question in two exercises. The second is the **yōon
   * contrast**: きゃ against きや, one mora against two, which differ only in the
   * size of the second character and which no pair of single kana can express. The
   * script is part of that exercise rather than a setting beside it, because `kya`
   * names きゃ in hiragana and キャ in katakana — a question that mixed them would
   * have two right answers.
   *
   * The question comes from the backend, which chooses the pair, weights it by how
   * often this learner has got it wrong, and records the answer against it. The
   * rule itself is in `nihongo_core::drill`, and it is told to the learner below
   * the answer rather than left as a mystery.
   */
  import * as api from "./api";
  import { isSingleKana } from "./kana";
  import type { DrillKind, DrillQuestion, DrillTally } from "./types";

  /**
   * Open a kana on the practice board, for the learner who would rather write
   * what they just got wrong than pick it again.
   */
  let { onopen }: { onopen?: (ch: string) => void } = $props();

  /** The exercises, in the order they are offered: the classic pairs first. */
  const EXERCISES: { kind: DrillKind; label: string; note: string }[] = [
    {
      kind: "confusion",
      label: "Confusable kana",
      note: "Shapes that differ by stroke direction: シ and ツ, ソ and ン.",
    },
    {
      kind: "yoon-hiragana",
      label: "Yōon きゃ",
      note: "One mora against two: きゃ is the small ゃ, きや is き + や.",
    },
    {
      kind: "yoon-katakana",
      label: "Yōon キャ",
      note: "The same contrast in katakana: キャ against キヤ.",
    },
    {
      kind: "voicing-hiragana",
      label: "Voiced が",
      note: "The mark a beginner leaves off: か against が, and は against ば and ぱ.",
    },
    {
      kind: "voicing-katakana",
      label: "Voiced ガ",
      note: "The same in katakana: カ against ガ, and ハ against バ and パ.",
    },
  ];

  let kind = $state<DrillKind>("confusion");
  let question = $state<DrillQuestion | null>(null);
  /** The spellings on screen, in display order: the pair, shuffled. */
  let options = $state<string[]>([]);
  let picked = $state<string | null>(null);
  let tally = $state<DrillTally | null>(null);
  let asked = $state(0);
  let right = $state(0);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const exercise = $derived(
    EXERCISES.find((e) => e.kind === (question?.kind ?? kind)) ?? EXERCISES[0],
  );
  /**
   * The spelling to offer the board, when the board can take it.
   *
   * Only for a single kana: a yōon answer is a digraph, and the board grades one
   * character at a time, so offering it there would open half an answer. The
   * classic pairs — which is where a learner most wants to write what they missed
   * — are all single kana.
   */
  const writable = $derived(
    question && picked && isSingleKana(question.ch) ? question.ch : null,
  );

  function shuffle<T>(items: T[]): T[] {
    const out = [...items];
    for (let i = out.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [out[i], out[j]] = [out[j], out[i]];
    }
    return out;
  }

  /**
   * A ticket per ask, so an earlier request cannot land on top of a later one.
   *
   * `Next` and a chip press can both be in flight — the IPC round trip is fast but
   * not instant — and without this the *first* answer could be drawn under the
   * exercise the learner has since chosen: the chip and the note would name one
   * exercise and the options would be another's. The answer would still be
   * recorded against the pair its own question named, so only the screen would be
   * wrong — which is the kind of wrong a learner reads as the app being broken.
   */
  let ask = 0;

  /** Ask for the next question. */
  export async function next() {
    const ticket = ++ask;
    const wanted = kind;
    try {
      busy = true;
      // Named `drawn` and not `asked`, which is the session's score above.
      const drawn = await api.nextDrillQuestion(wanted);
      // Superseded: a later ask owns the screen now, so this answer is stale.
      if (ticket !== ask) return;
      // And the answer says which exercise it is for, so a response that is not
      // the one asked for is dropped rather than drawn under the wrong note.
      if (drawn && drawn.kind !== wanted) return;
      question = drawn;
      // Two answers, so the right one is never in the same place twice — which
      // would let a learner learn the position instead of the shape.
      options = drawn ? shuffle(drawn.options) : [];
      picked = null;
      tally = null;
      error = null;
    } catch (e) {
      if (ticket === ask) error = String(e);
    } finally {
      if (ticket === ask) busy = false;
    }
  }

  /**
   * Change exercise, and start a new score with it.
   *
   * The question on screen goes with the old exercise: the note under the chips
   * and the "Write it" button both belong to it, and leaving it up while the new
   * one is asked is how a screen ends up describing a question it is not showing.
   * The session's score is not carried across either — a yōon contrast is a
   * different question from a classic pair, so a percentage spanning both would be
   * a number about nothing.
   */
  function choose(which: DrillKind) {
    if (which === kind) return;
    kind = which;
    asked = 0;
    right = 0;
    question = null;
    options = [];
    picked = null;
    tally = null;
    void next();
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

  <div class="modes" role="tablist" aria-label="Exercise">
    {#each EXERCISES as option (option.kind)}
      <button
        role="tab"
        aria-selected={kind === option.kind}
        class:active={kind === option.kind}
        onclick={() => choose(option.kind)}>{option.label}</button
      >
    {/each}
  </div>
  <p class="note">{exercise.note}</p>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if question}
    <p class="ask">Which one is <strong>{question.hepburn}</strong>?</p>

    <div class="options">
      {#each options as ch (ch)}
        <button
          class="option"
          class:long={!isSingleKana(ch)}
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
      <div class="after">
        <button class="next" onclick={next} disabled={busy}>Next</button>
        {#if writable && onopen}
          <button class="write" onclick={() => onopen?.(writable)}>
            Write {writable} on the board
          </button>
        {/if}
      </div>
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
  /* The exercise chips, styled here rather than with the shared `.tabs` rule:
     that rule capitalises its labels, and "Yōon きゃ" title-cased into proper
     nouns is the mistake the kanji grade tabs already made once. */
  .modes {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin: 12px 0 6px;
  }
  .modes button {
    padding: 5px 12px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: var(--bg);
    color: var(--ink);
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
  }
  .modes button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .note {
    margin: 0 0 12px;
    color: var(--muted);
    font-size: 0.8rem;
    max-width: 62ch;
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
  /* A digraph is two characters in one box, so it needs the smaller type to sit
     inside the same square as a single kana. */
  .option.long {
    font-size: 1.7rem;
    letter-spacing: -0.02em;
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
  .after {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
    margin-top: 12px;
  }
  .next,
  .write {
    padding: 6px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--line);
    cursor: pointer;
    font: inherit;
  }
  .next {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .write {
    background: var(--bg);
    color: var(--ink);
  }
  .next:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
