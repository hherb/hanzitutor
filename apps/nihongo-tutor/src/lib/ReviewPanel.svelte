<script lang="ts">
  /**
   * The review queue: what the board has taught this learner, and what is due.
   *
   * ## What is scheduled, and what a review is
   *
   * Every character graded on a board — a kana, a jōyō kanji, or one of the 214
   * radical head forms — has a card, and the card is keyed by the character.
   * Scheduling is SM-2 from `hanzi_core::progress`, the same code the Chinese app
   * runs, and the rule that makes a review a review is in the app's `store.rs`:
   * an attempt counts only when the character is **new or due**. Writing あ five
   * times in one sitting is practice; it is graded, and the schedule does not
   * move, because otherwise five attempts in a minute would push あ out by a year.
   *
   * ## One course at a time
   *
   * This screen is reached from both courses and shows one of them: the schedule
   * is a single file of characters (invariant 15), and `section` is what decides
   * which of them are offered and counted here — the kana course its kana, the
   * character course its kanji and head forms. The panel keeps its own board: the
   * two screens that already have one are about their own lists (Practice is a
   * script's course, Kanji is a grade's), and routing a due character through
   * either would fight `App.svelte`'s course-reload effect for the selection.
   *
   * ## The verdict's own way on, and what the queue says when it is empty
   *
   * The list of what is due is **above** the board, so after grading the next
   * character used to be a scroll away — and this is the one screen whose whole
   * subject is "what next". The primary control is therefore the verdict's own way
   * on, as on both board stages (invariant 36): `Grade` while there is no verdict,
   * `Next` once there is one, which opens the next due character, and
   * **`Nothing due`** when the queue has nothing left to offer. The queue's end is
   * a state rather than the stage's `finish` — there is nowhere to go and nothing
   * to go back to — and `board.ts`'s `afterGradeInQueue` is the rule. The next
   * character is asked of the **refreshed** queue: grading reloads it, and an
   * attempt that counted as a review takes its character out of the due list.
   *
   * ## Recall, not copying
   *
   * A review asks for the character from its prompt alone — a kana's sound and
   * script, a kanji's glosses, each one no other kana or kanji shares (Rust's
   * `hint_for`, shown by `review.ts`'s `reviewPrompt`). So the character is not
   * printed above the board or in the queue, the board has no guide, and the
   * voice's tooltip does not name it. It appears once the attempt is graded, as
   * the answer. A radical head form is shown from the start: the table has no
   * names for the 214, so its review is of how the shown form is written.
   *
   * Two things can still show the character before an attempt, and both make it
   * **traced**, which is graded and never scheduled: the stroke animation, and a
   * verdict already given on it (a second try after seeing the answer is not
   * recall either). Both are remembered per card for as long as the panel is up,
   * so choosing the same card again does not make it look unseen.
   *
   * A grade or a geometry fetch that comes back after another card was chosen is
   * dropped rather than applied to the card now on the board, and Grade waits
   * until the board holds the character the chosen card asks for.
   */
  import { SvelteSet } from "svelte/reactivity";
  import KanaCanvas from "./KanaCanvas.svelte";
  import SpeakButton from "./SpeakButton.svelte";
  import { afterGradeInQueue, traceReason } from "./board";
  import { joinedLabel } from "./kana";
  import { courseOf, type Section } from "./nav";
  import { VERDICT_COLOUR, VERDICT_LABEL } from "./render";
  import { everyStrokeFine, strokeCountNote } from "./verdict";
  import {
    cardKey,
    hasNextDue,
    isSameCard,
    nextDue,
    overdueLabel,
    reviewPrompt,
    scheduleNote,
    upcomingLabel,
  } from "./review";
  import * as api from "./api";
  import type { Drawable, DueItem, GradeReport, Point, ReviewQueueView } from "./types";
  import type { VoiceStatus } from "./speech";

  /** Which course's queue this is, and the voice `App.svelte` asked once for. */
  let { section, voice }: { section: Section; voice: VoiceStatus } = $props();

  /** The course being reviewed, for the labels and the two empty states. */
  const course = $derived(courseOf(section));

  /** How many due characters one page carries. The header still counts them all. */
  const PAGE = 40;

  let queue = $state<ReviewQueueView | null>(null);
  /** The clock the labels are relative to, taken when the queue was fetched. */
  let now = $state(new Date().toISOString());
  let limit = $state(PAGE);

  let chosen = $state<DueItem | null>(null);
  let drawn = $state<Drawable | null>(null);
  let report = $state<GradeReport | null>(null);
  /**
   * The taught strokes the last verdict read as drawn joined, 1-based — see
   * `joinedLabel`. It travels with `report`: the per-stroke list below numbers the
   * *drawn* strokes, so a joined kana needs this line to read honestly.
   */
  let joined = $state<number[][]>([]);
  let note = $state<{ tone: "ok" | "plain" | "bad"; text: string } | null>(null);
  let attempt = $state<Point[][]>([]);
  let board = $state<ReturnType<typeof KanaCanvas> | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);
  /** The cards the stroke animation has drawn while this panel has been up. */
  const watchedCards = new SvelteSet<string>();
  /** The cards a verdict has been shown for, which showed their character. */
  const answeredCards = new SvelteSet<string>();
  const watched = $derived(chosen !== null && watchedCards.has(cardKey(chosen)));
  const answered = $derived(chosen !== null && answeredCards.has(cardKey(chosen)));
  /**
   * Whether the board holds the character the chosen card asks for. Between a
   * choice and its geometry arriving it holds the previous one, and an attempt
   * drawn on that is not an answer to this card.
   */
  const ready = $derived(drawn !== null && chosen !== null && drawn.ch === chosen.ch);

  async function load() {
    try {
      error = null;
      queue = await api.reviewQueue(limit, section);
      now = new Date().toISOString();
    } catch (e) {
      error = String(e);
    }
  }
  // Loaded once, for the course this panel was mounted in: `App.svelte` keys the
  // panel on the section, so changing course mounts a new one rather than asking
  // this one to change the meaning of what is on the board.
  void load();

  /** Put a due character on the board, fetching the geometry its kind implies. */
  async function open(item: DueItem) {
    // The card already on the board: nothing to do. Starting it again would
    // clear the verdict and the attempt while the board kept the strokes, and
    // would hide a character the learner has just been shown.
    if (isSameCard(item, chosen) && drawn?.ch === item.ch) return;
    try {
      error = null;
      chosen = item;
      report = null;
      joined = [];
      note = null;
      attempt = [];
      const found =
        item.kind === "kana"
          ? await api.kana(item.ch)
          : item.kind === "kanji"
            ? await api.kanji(item.ch)
            : await api.radical(item.radical ?? 0);
      // Another card was chosen while this one's geometry was on its way.
      if (chosen !== item) return;
      drawn = found;
    } catch (e) {
      error = String(e);
    }
  }

  function onStrokes(strokes: Point[][]) {
    attempt = strokes;
    // A verdict belongs to the attempt it judged; the moment the attempt changes
    // it is stale, and leaving it up would colour the new strokes with the old
    // reading.
    report = null;
    joined = [];
    note = null;
  }

  async function grade() {
    if (!drawn || !chosen || !ready) return;
    const item = chosen;
    try {
      busy = true;
      error = null;
      // Recall is the rule here: there is no guide, so the attempt is traced only
      // when the animation, or an earlier verdict, has already shown the character.
      const reason = traceReason({ guide: false, watched, answered });
      const graded = await api.gradeAttempt(drawn.ch, attempt, reason !== null);
      if (chosen !== item) {
        // The learner chose another card while this was being graded. The
        // schedule has its answer; the verdict belongs to a card no longer on
        // the board, so it is not put up over the new one.
        await load();
        return;
      }
      report = graded.report;
      joined = graded.joined;
      answeredCards.add(cardKey(item));
      note = scheduleNote(graded, new Date().toISOString(), reason);
      // The character may have left the due list; the verdict stays up either
      // way, so the learner sees what they earned before the list moves.
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  /** The sentences under the per-stroke list; see `verdict.ts`. */
  const summary = $derived.by(() => {
    if (!report) return null;
    return { countNote: strokeCountNote(report), fine: everyStrokeFine(report) };
  });

  /** Whether this item is the one already on the board. */
  /**
   * The next due character among those the page holds, or `null` — `review.ts`'s
   * arithmetic, which answers it by identity rather than by position because the
   * queue is ordered by how overdue each character is and that order moves as
   * characters are graded.
   */
  const nextItem = $derived(queue ? nextDue(queue.items, chosen) : null);

  /**
   * Whether the queue can offer anything at all after the character on the board.
   *
   * Not simply "is there another item in the page": a queue with more due than one
   * page holds has a next character even when the page has run out, which is why
   * `review.ts` counts the section's uncapped `due` against the page's length.
   */
  const hasNext = $derived(!!queue && hasNextDue(queue, chosen));

  /**
   * What the primary control offers: the verdict, then the next due character,
   * then the state of the queue itself — invariant 36, and `board.ts`'s rule for a
   * board that has no lesson behind it.
   */
  const after = $derived(afterGradeInQueue(report !== null, hasNext));

  /**
   * Open the next due character, fetching another page first when the queue holds
   * more than the page does.
   *
   * One action rather than two: the learner pressed the control where Grade was,
   * and what they asked for is the next character — not a page of them to choose
   * from again.
   */
  async function advance() {
    if (nextItem) {
      void open(nextItem);
      return;
    }
    if (!hasNext) return;
    limit += PAGE;
    await load();
    const found = queue ? nextDue(queue.items, chosen) : null;
    if (found) void open(found);
  }
</script>

<section class="review">
  <header>
    <h2>Review</h2>
    <p class="scope">{course.label} — {course.blurb}</p>
    {#if queue}
      <p class="sub">
        {queue.due} due · {queue.cards} scheduled
      </p>
    {/if}
    <p class="hint">
      Every character graded on a board comes back here on SM-2's schedule. Write
      it from the prompt alone: the character is shown once the attempt is graded.
      An attempt after watching the stroke order is practice, as is writing it
      again before it is due, and the schedule does not move.
    </p>
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  {#if queue?.warning}
    <p class="warning" role="status">{queue.warning}</p>
  {/if}

  <div class="layout">
    <nav aria-label="Due for review">
      {#if queue && queue.due > 0}
        {#each queue.items as item (item.ch)}
          {@const prompt = reviewPrompt(item, false)}
          <button
            class="due"
            class:active={isSameCard(item, chosen)}
            onclick={() => open(item)}
          >
            <!-- The prompt, never the character: the list is above the board, and
                 a glyph here would be the answer to the question below it. Only a
                 radical head form, which is shown on the board too, has one. -->
            {#if prompt.glyph}<span class="glyph">{prompt.glyph}</span>{/if}
            <span class="what">{prompt.hint}</span>
            <span class="when">{overdueLabel(item.due, now)}</span>
          </button>
        {/each}
        {#if queue.due > queue.items.length}
          <button
            class="more"
            onclick={() => {
              limit += PAGE;
              void load();
            }}
          >
            Show {Math.min(PAGE, queue.due - queue.items.length)} more of {queue.due}
          </button>
        {/if}
      {:else if queue}
        <p class="empty">
          {#if queue.cards === 0}
            {#if section === "kana"}
              Nothing is scheduled yet. Write a kana from memory on Practice —
              with the hint off — and it will come back here. A traced kana is
              practice and is not scheduled.
            {:else}
              Nothing is scheduled yet. Write a character from memory on Kanji, or
              a head form on Radicals — with the hint off — and it will come back
              here. A traced one is practice and is not scheduled.
            {/if}
          {:else if queue.nextDue}
            Nothing is due right now —
            {#if queue.cards === 1}
              one character is scheduled, and the next comes back
            {:else}
              {queue.cards} characters are scheduled, and the next comes back
            {/if}
            {upcomingLabel(queue.nextDue, now)}.
          {:else}
            Nothing is due right now.
          {/if}
        </p>
      {:else if !error}
        <p class="hint">Loading the schedule…</p>
      {/if}
    </nav>

    <section class="practice">
      {#if drawn && chosen}
        {@const prompt = reviewPrompt(chosen, answered)}
        <div class="prompt">
          <span class="big" aria-label={prompt.glyph ? undefined : "the character to write"}
            >{prompt.glyph ?? "？"}</span
          >
          <span class="reading">{prompt.hint}</span>
          <span class="strokes">{drawn.strokeCount}
            {drawn.strokeCount === 1 ? "stroke" : "strokes"}</span>
        </div>

        <KanaCanvas
          bind:this={board}
          character={drawn}
          {report}
          guide={false}
          onchange={onStrokes}
          onanimate={() => {
            if (chosen) watchedCards.add(cardKey(chosen));
          }}
        />

        <div class="actions">
          {#if chosen?.kind === "kana"}
            <!--
              Only a kana is spoken, and the reason is the app's reading rule
              rather than a missing feature: a bare kanji has a dozen readings and
              this app never chooses between them, while a radical's prompt is a
              Kangxi number. A kana is the one due item whose character *is* its
              pronunciation, so it is the one that gets a button.
            -->
            <SpeakButton text={drawn.ch} {voice} title="Hear the sound" />
          {/if}
          <button
            onclick={() => board?.animate()}
            title="Shows the character — an attempt after it is practice, not a review"
          >
            Show stroke order
          </button>
          <button onclick={() => board?.undo()}>Undo</button>
          <button onclick={() => board?.clear()}>Clear</button>
          {#if after === "next"}
            <!--
              The verdict's own way on, in the place Grade was — invariant 36. It
              matters more here than on a stage: the queue that says what is due is
              *above* the board, so without this the next character is a scroll
              away, and this is the one screen whose whole subject is "what next".
            -->
            <button
              class="primary"
              onclick={() => void advance()}
              aria-label="Next due character{nextItem
                ? `: ${reviewPrompt(nextItem, false).hint}`
                : ""}"
              title={nextItem
                ? `On to ${reviewPrompt(nextItem, false).hint}`
                : "Showing the next due characters"}
            >
              Next
            </button>
          {:else if after === "nothing"}
            <!-- The queue's end is a state, not an action: every due character of
                 this course has been written. -->
            <button
              class="primary"
              disabled
              title="Every due character of this course has been written, and the next one comes back on the schedule"
            >
              Nothing due
            </button>
          {:else}
            <button
              class="primary"
              disabled={busy || !ready || attempt.length === 0}
              onclick={grade}
            >
              Grade
            </button>
          {/if}
        </div>

        {#if note}
          <p class="note" class:bad={note.tone === "bad"} class:ok={note.tone === "ok"}>
            {note.text}
          </p>
        {/if}

        {#if report && summary}
          {@const drawnJoined = joinedLabel(joined)}
          <div class="verdict" class:ok={report.legible}>
            <p class="overall">
              {report.overall.toFixed(0)}<span>/100</span>
              <em>{report.legible ? "legible" : "not yet"}</em>
            </p>
            <dl class="scores">
              <div><dt>shape</dt><dd>{report.shapeScore.toFixed(2)}</dd></div>
              <div><dt>place</dt><dd>{report.positionScore.toFixed(2)}</dd></div>
              <div><dt>ink</dt><dd>{report.inkScore.toFixed(2)}</dd></div>
              <div><dt>order</dt><dd>{report.orderScore.toFixed(2)}</dd></div>
            </dl>
            <ul class="strokes-list">
              {#each report.strokes as stroke (stroke.refIndex)}
                <li style="--c:{VERDICT_COLOUR[stroke.verdict]}">
                  <span class="dot"></span>
                  stroke {stroke.refIndex + 1}: {VERDICT_LABEL[stroke.verdict]}
                </li>
              {/each}
            </ul>
            {#if drawnJoined}
              <p class="fine">Taught strokes {drawnJoined} were drawn as one.</p>
            {/if}
            {#if summary.countNote}
              <p class="fine">{summary.countNote}</p>
            {/if}
            {#if summary.fine}
              <p class="fine">Every stroke the right shape, in the right place, in order.</p>
            {/if}
          </div>
        {/if}
      {:else}
        <p class="hint">
          Choose a due character on the left and write it — the verdict also
          decides when it comes back.
        </p>
      {/if}
    </section>
  </div>
</section>

<style>
  .review header h2 {
    margin: 0;
  }

  .sub {
    margin: 0.2rem 0 0;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  /* Which course's queue this is. It is not decoration: the same screen shows
     either the kana or the characters, and the two are not interchangeable. */
  .scope {
    margin: 0.2rem 0 0;
    color: var(--muted);
    font-size: 0.84rem;
  }

  .hint {
    max-width: 72ch;
  }

  /* Two columns, not the three the course screens use: a due list and the board
     for whatever is chosen from it. Beat the shared `.layout` by being scoped. */
  .layout {
    grid-template-columns: 240px minmax(280px, 1fr);
    margin-top: 12px;
  }

  @media (max-width: 940px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }

  .due {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    padding: 7px 10px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
    text-align: left;
  }

  .due .glyph {
    font-size: 1.4rem;
    line-height: 1.2;
  }

  .due .what {
    font-size: 0.95rem;
    line-height: 1.3;
  }

  .due .when {
    font-size: 0.72rem;
    color: var(--bad);
  }

  .due.active {
    border-color: var(--accent);
  }

  .more {
    padding: 6px 10px;
    border: 1px dashed var(--line);
    border-radius: var(--radius);
    background: none;
    color: var(--muted);
    cursor: pointer;
    text-align: left;
  }

  .empty {
    margin: 0;
    color: var(--muted);
    font-size: 0.82rem;
    max-width: 34ch;
  }

  .warning {
    margin: 8px 0 0;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--bad) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--bad) 40%, transparent);
    color: var(--bad);
    font-size: 0.84rem;
  }
</style>
