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
   * ## Why this screen has its own board
   *
   * The due character has to be *written* to be reviewed, and the two screens that
   * already have a board are about their own lists: Practice is a script's course
   * and Kanji is a grade's. Routing a due character through either would fight
   * `App.svelte`'s course-reload effect for the selection, so this panel owns the
   * board, the way `ConfusionDrill` owns its question.
   */
  import KanaCanvas from "./KanaCanvas.svelte";
  import { VERDICT_COLOUR, VERDICT_LABEL } from "./render";
  import { overdueLabel, scheduleNote, upcomingLabel } from "./review";
  import * as api from "./api";
  import type { Drawable, DueItem, GradeReport, Point, ReviewQueueView } from "./types";

  /** How many due characters one page carries. The header still counts them all. */
  const PAGE = 40;

  let queue = $state<ReviewQueueView | null>(null);
  /** The clock the labels are relative to, taken when the queue was fetched. */
  let now = $state(new Date().toISOString());
  let limit = $state(PAGE);

  let chosen = $state<DueItem | null>(null);
  let drawn = $state<Drawable | null>(null);
  let report = $state<GradeReport | null>(null);
  let note = $state<{ tone: "ok" | "plain" | "bad"; text: string } | null>(null);
  let attempt = $state<Point[][]>([]);
  let board = $state<ReturnType<typeof KanaCanvas> | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function load() {
    try {
      error = null;
      queue = await api.reviewQueue(limit);
      now = new Date().toISOString();
    } catch (e) {
      error = String(e);
    }
  }
  void load();

  /** Put a due character on the board, fetching the geometry its kind implies. */
  async function open(item: DueItem) {
    try {
      error = null;
      chosen = item;
      report = null;
      note = null;
      attempt = [];
      drawn =
        item.kind === "kana"
          ? await api.kana(item.ch)
          : item.kind === "kanji"
            ? await api.kanji(item.ch)
            : await api.radical(item.radical ?? 0);
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
    note = null;
  }

  async function grade() {
    if (!drawn) return;
    try {
      busy = true;
      error = null;
      const graded = await api.gradeAttempt(drawn.ch, attempt);
      report = graded.report;
      note = scheduleNote(graded, new Date().toISOString());
      // The character may have left the due list; the verdict stays up either
      // way, so the learner sees what they earned before the list moves.
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const summary = $derived.by(() => {
    if (!report) return null;
    return {
      total: report.strokes.length,
      wrong: report.strokes.filter(
        (s) => s.verdict !== "correct" && s.userIndex !== null,
      ),
    };
  });
</script>

<section class="review">
  <header>
    <h2>Review</h2>
    {#if queue}
      <p class="sub">
        {queue.due} due · {queue.cards} scheduled
      </p>
    {/if}
    <p class="hint">
      Every character graded on a board comes back here on SM-2's schedule. A
      character that is new or due counts as a review; writing it again before
      then is practice, and the schedule does not move.
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
          <button
            class="due"
            class:active={item.ch === chosen?.ch}
            onclick={() => open(item)}
          >
            <span class="glyph">{item.ch}</span>
            <span class="what">{item.hint} · {item.kind}</span>
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
            Nothing is scheduled yet. Grade a character on Practice, Kanji or
            Radicals and it will come back here.
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
      {#if drawn}
        <div class="prompt">
          <span class="big">{drawn.ch}</span>
          <span class="reading">{chosen?.hint ?? ""}</span>
          <span class="strokes">{drawn.strokeCount}
            {drawn.strokeCount === 1 ? "stroke" : "strokes"}</span>
        </div>

        <KanaCanvas bind:this={board} character={drawn} {report} onchange={onStrokes} />

        <div class="actions">
          <button onclick={() => board?.animate()}>Show stroke order</button>
          <button onclick={() => board?.undo()}>Undo</button>
          <button onclick={() => board?.clear()}>Clear</button>
          <button class="primary" disabled={busy || attempt.length === 0} onclick={grade}>
            Grade
          </button>
        </div>

        {#if note}
          <p class="note" class:bad={note.tone === "bad"} class:ok={note.tone === "ok"}>
            {note.text}
          </p>
        {/if}

        {#if report && summary}
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
            {#if summary.wrong.length === 0}
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
    font-size: 0.74rem;
    color: var(--muted);
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
