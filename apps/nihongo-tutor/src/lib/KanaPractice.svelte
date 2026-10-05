<script lang="ts">
  /**
   * The kana stage: one kana on the board, its sound above it, and the board's
   * controls below it — and everything else behind one `More` fold.
   *
   * ## Why it is a screen of its own
   *
   * The kana course was one page: the script toggle, the lesson list, the lesson's
   * kana and the board — with the verdict, the typing box and the confusions under
   * it — stacked in three columns. That is the shape N12 took out of the kanji
   * course for the phone's sake: the board sat below the lesson list and the kana
   * grid, so a learner scrolled *past the course* to write one kana and back to
   * choose the next. N13 is the same repair for the same reason (`KanaPanel` is
   * the course and this is what a lesson card opens), and
   * `HANDOVER_NIHONGO.md` invariants 33 and 34 are the rule.
   *
   * ## The order of what is here
   *
   * The sound is **above** the board and the tools are **below** it, which is the
   * kanji stage's arrangement and Hanzi Tutor's: what you are being asked to
   * write, then the thing you write on, then the tools. Inside a lesson the arrows
   * either side of the kana are the only way on — no strip of five, which would be
   * a second grid above a board that already has the whole lesson one arrow away.
   *
   * ## The fold
   *
   * What a kana is written with, the kana it is confused with, and the typing
   * exercise are all *about the kana* rather than about the board, so they wait
   * behind `More`. Folded, the whole action — sound, board, tools, Grade — is what
   * fits a phone's screen without scrolling, which is the measurement N13 records;
   * it is also what keeps `KanaCanvas`'s own promise in `board.ts` that the board
   * and the controls stay above the fold.
   *
   * ## The kana is handed to the voice whole
   *
   * A kana's character **is** its pronunciation, so "Hear it" hands over `kana.ch`
   * and never a reading composed from anything — `HANDOVER_NIHONGO.md` invariant
   * 27. The `h` key is the same control: `isHearItKey` in `speech.ts` decides which
   * presses count, because `h` is exactly what is typed in the reading box below.
   *
   * ## And the verdict belongs to the attempt it judged
   *
   * `Hint` switches between tracing and recall and **clears the board**, which is
   * Hanzi Tutor's own behaviour — half an attempt traced and half remembered
   * answers neither question — so the verdict goes with the attempt it judged.
   * `KanaCanvas` tells this screen when it empties itself; that is why the panel
   * below cannot outlive the strokes it describes.
   */
  import Icon from "./Icon.svelte";
  import KanaCanvas from "./KanaCanvas.svelte";
  import SpeakButton from "./SpeakButton.svelte";
  import { untrack } from "svelte";
  import { VERDICT_COLOUR, VERDICT_LABEL } from "./render";
  import { scheduleNote } from "./review";
  import { canHear, isHearItKey, type VoiceStatus } from "./speech";
  import { joinedLabel, lessonLabel, lessonOf, neighbourIn, positionIn } from "./kana";
  import { afterGrade } from "./board";
  import * as api from "./api";
  import type { GradeReport, Kana, LessonView, Point, ReadingCheck } from "./types";

  interface Props {
    /**
     * The kana to open on. A **new value remounts this component** (`KanaPanel`
     * keys it), so the state below is read once and a later open does not have to
     * be merged into a screen that is already up.
     */
    start: string;
    /**
     * The whole script's course, so a kana this screen opens — from the confusions
     * list, which crosses lessons — finds **its own** lesson's arrows rather than
     * keeping the ones it was opened with.
     */
    course: LessonView[];
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice?: VoiceStatus;
    /** Back to the course. */
    onleave: () => void;
  }

  let { start, course, voice, onleave }: Props = $props();

  /**
   * The kana on the board.
   *
   * Read from `start` once, deliberately, and `untrack` is how that is said: a new
   * `start` **remounts** this component, so there is nothing here to react to.
   */
  let ch = $state<string>(untrack(() => start));

  let kana = $state<Kana | null>(null);
  let report = $state<GradeReport | null>(null);
  /**
   * The taught strokes the last verdict read as drawn joined, 1-based — `[[1, 2]]`
   * for a さ written in two strokes. It travels with `report`, and is what makes
   * the per-stroke list honest: its numbers are the *drawn* strokes.
   */
  let joined = $state<number[][]>([]);
  let note = $state<{ tone: "ok" | "plain" | "bad"; text: string } | null>(null);
  let attempt = $state<Point[][]>([]);
  let board = $state<ReturnType<typeof KanaCanvas> | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let typed = $state("");
  let check = $state<ReadingCheck | null>(null);

  /**
   * Whether the faint copy is on the board: **tracing** while it is, **recall**
   * while it is not. The board itself clears the attempt when this changes.
   */
  let guide = $state(true);
  /** Whether the facts, the typing exercise and the confusions are folded out. */
  let showDetails = $state(false);

  /**
   * The lesson the kana on the board belongs to, derived from the course rather
   * than passed in: the stage can change its own kana, and the arrows have to be
   * the *new* kana's lesson's.
   */
  const lesson = $derived(lessonOf(course, ch));
  const previous = $derived(neighbourIn(lesson, ch, -1));
  const next = $derived(neighbourIn(lesson, ch, 1));
  const at = $derived(positionIn(lesson, ch));
  const of = $derived(lesson?.kana.length ?? 0);

  /**
   * What the primary control under the board offers: the verdict, then the way on.
   *
   * `board.ts`'s rule, shared with the kanji stage, so the two cannot disagree
   * about what "graded" means for the control.
   */
  const after = $derived(afterGrade(report !== null, lesson !== null, next !== null));

  /**
   * Load whatever character is on the board, and start it clean.
   *
   * The effect reads `ch` and writes the screen's own state, which is the shape
   * `HANDOVER_NIHONGO.md` trap 9 is about: an effect that read what it writes
   * would never settle. Stepping through a lesson and tapping a confusion are both
   * writes to `ch`, and that is what re-runs it.
   */
  $effect(() => {
    const target = ch;
    void open(target);
  });

  async function open(target: string) {
    try {
      error = null;
      report = null;
      joined = [];
      note = null;
      attempt = [];
      check = null;
      typed = "";
      showDetails = false;
      const found = await api.kana(target);
      // A response that a later step or tap has overtaken is dropped rather than
      // applied: the two can arrive in either order, and the board has to hold the
      // kana the learner last asked for.
      if (untrack(() => ch) !== target) return;
      kana = found;
    } catch (e) {
      error = String(e);
    }
  }

  /** Step through the lesson, one kana at a time. */
  function step(delta: number) {
    const target = neighbourIn(lesson, ch, delta);
    if (target) ch = target;
  }

  function onStrokes(strokes: Point[][]) {
    attempt = strokes;
    // A verdict belongs to the attempt it judged: the moment the attempt changes
    // it is stale, and leaving it up would colour the new strokes with the old
    // reading.
    report = null;
    joined = [];
    note = null;
  }

  async function askForAVerdict() {
    if (!kana) return;
    try {
      busy = true;
      error = null;
      // The command grades *and* offers the attempt to the review schedule, so
      // what comes back is the verdict plus the schedule's answer to it.
      const graded = await api.gradeAttempt(ch, attempt);
      report = graded.report;
      joined = graded.joined;
      note = scheduleNote(graded, new Date().toISOString());
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function checkTyped() {
    try {
      check = await api.checkReading(ch, typed);
    } catch (e) {
      error = String(e);
    }
  }

  /**
   * The "hear it" shortcut, live only while a kana is on the board.
   *
   * Handled here rather than by the app because this screen is the only one with
   * a kana to say: the course it came from has a lesson list and no board, and a
   * window-level letter key that fired there would speak whatever was last
   * selected on a screen the learner is not looking at.
   */
  function onKey(event: KeyboardEvent) {
    if (!isHearItKey(event)) return;
    if (!kana || !canHear(voice)) return;
    event.preventDefault();
    void api.speak(kana.ch).catch((e) => {
      error = String(e);
    });
  }

  /**
   * Choosing another kana ends whatever is being said.
   *
   * A voice that keeps talking over the next thing the learner looks at is the
   * one way an audio feature becomes an annoyance. Speaking never changes `ch` —
   * the button does that — so this effect only ever stops, and the same effect in
   * `App.svelte` stops it when the learner leaves the screen.
   */
  $effect(() => {
    void ch;
    void api.stopSpeaking().catch(() => {
      // Nothing to report: this is cleanup, and the usual answer is that nothing
      // was being said.
    });
  });

  const summary = $derived.by(() => {
    if (!report) return null;
    return {
      total: report.strokes.length,
      wrong: report.strokes.filter((s) => s.verdict !== "correct" && s.userIndex !== null),
    };
  });
</script>

<section class="stage">
  <header class="head">
    <!--
      Out of the board and back to the course. The word says where it goes rather
      than leaving an arrow to be guessed at, and it is the one control here that
      is not a board tool — which is why it is in the header row and not in the
      tool groups below the board.
    -->
    <button class="back" onclick={onleave} title="Back to the lessons">
      <Icon name="back" />
      <span>Lessons</span>
    </button>

    {#if lesson}
      <div class="nav" role="group" aria-label="Kana in this lesson">
        <button
          class="step"
          disabled={!previous}
          onclick={() => step(-1)}
          aria-label={previous
            ? `Previous kana: ${previous}`
            : "This is the first kana of the lesson"}
          title={previous
            ? `Previous kana (${previous})`
            : "This is the first kana of the lesson"}
        >
          <span class="flip"><Icon name="next" /></span>
        </button>
        <span class="position" title="{lessonLabel(lesson)} · kana {at} of {of}">
          {at} / {of}
        </span>
        <button
          class="step"
          disabled={!next}
          onclick={() => step(1)}
          aria-label={next ? `Next kana: ${next}` : "This is the last kana of the lesson"}
          title={next ? `Next kana (${next})` : "This is the last kana of the lesson"}
        >
          <Icon name="next" />
        </button>
      </div>
    {/if}
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if kana}
    <!--
      The sound, above the board: the kana itself, what it is read as, and the one
      control that says it. `Hear it` hands over the *character*, which for a kana
      is also its pronunciation — invariant 27 — and a kana with no sound of its
      own (っ, ー) says so instead of offering a button that would say nothing.
    -->
    <div class="meta" class:open={showDetails}>
      <div class="who">
        <span class="big" lang="ja">{kana.ch}</span>
        <div class="gloss">
          <dl class="readings">
            <div>
              <dt>sound</dt>
              <dd>
                {#if kana.silent}
                  <em>no sound of its own</em>
                {:else}
                  {kana.hepburn}{#if kana.romaji.length > 1}<span class="aside">
                    (or {kana.romaji.slice(1).join(", ")})</span>{/if}
                {/if}
              </dd>
            </div>
          </dl>

          <div class="rows">
            <SpeakButton text={kana.ch} {voice} label="Hear it (H)" />
            <button
              class="fold"
              type="button"
              aria-expanded={showDetails}
              aria-controls="kana-details"
              onclick={() => (showDetails = !showDetails)}
            >
              {showDetails ? "Less" : "More"}
              <span class="chevron" class:open={showDetails} aria-hidden="true">›</span>
            </button>
          </div>

          <div class="details" id="kana-details">
            <ul class="facts">
              <li>{kana.strokeCount} {kana.strokeCount === 1 ? "stroke" : "strokes"}</li>
              <li>{kana.script}</li>
            </ul>

            <!--
              The reading half of the lesson, which is the one exercise here that
              is not the board: typing the sound is how the kana is checked by
              something other than a hand.
            -->
            <div class="typing">
              <label for="typed">Type the reading</label>
              <div class="row">
                <input
                  id="typed"
                  bind:value={typed}
                  onkeydown={(e) => e.key === "Enter" && checkTyped()}
                  placeholder={kana.romaji[0] ?? ""}
                  autocomplete="off"
                  autocapitalize="off"
                  spellcheck="false"
                />
                <button onclick={checkTyped}>Check</button>
              </div>
              {#if check}
                <p class="result" class:ok={check.correct}>
                  {#if check.correct}
                    Correct.
                  {:else}
                    That is {check.produced || "not a reading"} — try again.
                  {/if}
                </p>
              {/if}
              <p class="hint">
                Both romanisations are accepted: Hepburn <code>shi tsu fu</code> and Kunrei
                <code>si tu fu</code> alike.
              </p>
            </div>

            {#if kana.confusions.length > 0}
              <div class="confusions">
                <h2>Not to be confused with</h2>
                <ul>
                  {#each kana.confusions as c (c.ch)}
                    <li>
                      <button
                        class="cell small"
                        onclick={() => (ch = c.ch)}
                        title="Write {c.ch} on the board"
                      >
                        {c.ch}
                      </button>
                      <span>{c.tell}</span>
                    </li>
                  {/each}
                </ul>
              </div>
            {/if}

            <p class="hint">Yōon are two kana that make one mora — type them together.</p>
          </div>
        </div>
      </div>
    </div>

    <KanaCanvas bind:this={board} character={kana} {report} {guide} onchange={onStrokes} />

    <!--
      Below the board, only the tools: nothing that is a *fact* about the kana lives
      down here. Three groups, as the kanji stage and Hanzi Tutor draw them — what
      the board shows you, what it can take back, and the verdict — because a row
      of five identical buttons is a row a learner has to read.
    -->
    <div class="controls">
      <div class="toolgroups">
        <div class="tools" role="group" aria-label="Writing help">
          <button
            class="tool"
            class:on={!guide}
            aria-pressed={!guide}
            onclick={() => (guide = !guide)}
            aria-label={guide
              ? "Hint: a faint copy of the kana is on the board. Switch to recall, which hides it"
              : "Hint: the kana is hidden while you write it. Switch to tracing, where a faint copy is on the board"}
            title={guide
              ? "The kana is on the board to follow. Press to hide it and write it from memory — either way the board is cleared."
              : "Write it from memory. Press to put a faint copy back on the board — either way the board is cleared."}
          >
            <span class="tool-glyph"><Icon name={guide ? "eye" : "eye-off"} /></span>
            <span class="tool-word">Hint</span>
          </button>
          <button
            class="tool"
            onclick={() => board?.animate()}
            disabled={kana.strokeCount === 0}
            aria-label="Strokes: watch the kana written one stroke at a time"
            title="Watch it written, one stroke at a time"
          >
            <span class="tool-glyph"><Icon name="play" /></span>
            <span class="tool-word">Strokes</span>
          </button>
        </div>

        <div class="tools" role="group" aria-label="Drawing">
          <button
            class="tool"
            onclick={() => board?.undo()}
            aria-label="Undo the last stroke"
            title="Take the last stroke back"
          >
            <span class="tool-glyph"><Icon name="undo" /></span>
            <span class="tool-word">Undo</span>
          </button>
          <button
            class="tool"
            onclick={() => board?.clear()}
            disabled={attempt.length === 0}
            aria-label="Clear the board"
            title="Clear every stroke off the board"
          >
            <span class="tool-glyph"><Icon name="trash" /></span>
            <span class="tool-word">Clear</span>
          </button>
        </div>

        <div class="tools" role="group" aria-label="Verdict">
          {#if after === "next"}
            <!--
              The verdict's own way on, in the place Grade was: the learner has
              just pressed the button at the bottom of the screen, and the lesson's
              arrows are in the top corner. Hanzi Tutor's arrangement and
              `board.ts`'s `afterGrade`. The corner arrows keep their own job,
              which is **skipping**: they step through the lesson without grading,
              so this only becomes Next once something has been judged.
            -->
            <button
              class="primary"
              onclick={() => step(1)}
              aria-label="Next kana: {next}"
              title="On to {next}"
            >
              <span class="primary-glyph"><Icon name="next" /></span>
              <span>Next</span>
            </button>
          {:else if after === "finish"}
            <!-- The last kana of the lesson: there is no next one, and the way on
                 is the course the stage was opened from. -->
            <button
              class="primary"
              onclick={onleave}
              aria-label="Back to the lessons: this is the last kana of the lesson"
              title="The lesson is finished — back to the lessons"
            >
              <span class="primary-glyph"><Icon name="tick" /></span>
              <span>Finish</span>
            </button>
          {:else}
            <button
              class="primary"
              disabled={busy || attempt.length === 0}
              onclick={askForAVerdict}
              aria-label={busy ? "Grading…" : "Grade this attempt"}
              title="Grade what you have drawn"
            >
              <span class="primary-glyph"><Icon name="tick" /></span>
              <span>{busy ? "Grading…" : "Grade"}</span>
            </button>
          {/if}
        </div>
      </div>

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
          {#if summary.wrong.length === 0}
            <p class="fine">Every stroke the right shape, in the right place, in order.</p>
          {/if}
        </div>
      {/if}

      {#if note}
        <p class="note" class:ok={note.tone === "ok"} class:bad={note.tone === "bad"}>
          {note.text}
        </p>
      {/if}
    </div>
  {/if}
</section>

<svelte:window onkeydown={onKey} />

<!--
  The stage's own styles, and they are `KanjiPractice`'s where the two stages have
  to look the same — the head, the meta panel and the fold, the tool row, and the
  verdict under it. A Svelte component's styles are scoped to it, so the same look
  in two stages is two copies rather than one rule, which is the arrangement
  `app.css` already has with `KanjiPractice`'s verdict block. **If one of these
  changes, change it in both stages**, the way `render.ts` and `Icon.svelte` are
  changed in both apps.
-->
<style>
  .stage {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
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

  /* The lesson's own navigation, in the corner where the kana is not: the board
     below is the thing being looked at, and this says which of the lesson's kana
     it is holding. */
  .nav {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
  }

  /* 44px is the size a finger needs, and every control on this stage is that tall
     on a phone: measured in the harness at 390px on the kanji stage, the back
     button was 25px and the fold 40px before those, and the arrows and the Grade
     button were 40 as well — all four are fixed, and both stages carry the same
     values because it is one shared row. */
  .step {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border: 1px solid var(--line);
    border-radius: 50%;
    background: var(--panel);
    color: var(--ink);
    font-size: 16px;
    cursor: pointer;
  }

  .step:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .step:disabled {
    opacity: 0.35;
    cursor: default;
  }

  /* The same arrow, mirrored, for the step back. */
  .flip {
    display: grid;
    place-items: center;
    transform: scaleX(-1);
  }

  .position {
    min-width: 3.4rem;
    text-align: center;
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }

  /* The sound and the board's explanation, above the board. */
  .meta {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 10px 12px;
  }

  .who {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .big {
    font-size: 2.4rem;
    line-height: 1;
  }

  .gloss {
    flex: 1;
    min-width: 0;
  }

  .readings {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 4px 12px;
    margin: 0;
  }

  .readings div {
    min-width: 0;
  }

  .readings dt {
    font-size: 0.66rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .readings dd {
    margin: 0;
    font-size: 1rem;
  }

  .readings dd em {
    font-size: 0.78rem;
    font-style: normal;
    color: var(--muted);
  }

  .aside {
    color: var(--muted);
    font-size: 0.78rem;
  }

  .rows {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    margin-top: 6px;
  }

  /* The fold, as the kanji stage draws it: a text control with a 44px touch
     target, which is what that stage's own measurement at 390px required — the
     fold was 40px there before it. Everything that is prose about the kana waits
     behind it, so the board follows the sound rather than following the card. */
  .fold {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-height: 44px;
    margin-top: 4px;
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 650;
    color: var(--accent);
    cursor: pointer;
  }

  .fold .chevron {
    color: var(--muted);
    transition: transform 0.15s ease;
  }

  .fold .chevron.open {
    transform: rotate(90deg);
  }

  .meta:not(.open) .details {
    display: none;
  }

  /* The fold is not a clamp on the sound above the board: the sound is the whole
     of what that block is for, and everything that is prose about the kana is
     inside `.details`, which the rule above takes out of the way. */
  .details {
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px dashed var(--line);
    margin-top: 10px;
    padding-top: 10px;
  }

  .facts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    font-size: 0.82rem;
    color: var(--muted);
  }

  /* The board's own controls: a glyph on a disc with its short name under it, two
     or three tools to a card, and the cards centred under the board — Hanzi
     Tutor's control row, which is where the glyphs and the short names come from,
     and the kanji stage's values. */
  .controls {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .toolgroups {
    display: flex;
    align-items: stretch;
    justify-content: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .tools {
    display: flex;
    align-items: stretch;
    padding: 5px 6px;
    border: 1px solid var(--line);
    border-radius: 13px;
    background: var(--panel);
  }

  .tool {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    min-width: 0;
    padding: 4px 9px;
    border: 0;
    border-radius: 9px;
    background: none;
    font: inherit;
    font-size: 0.72rem;
    line-height: 1.15;
    color: var(--muted);
    text-align: center;
    cursor: pointer;
  }

  .tool-glyph {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--line) 45%, transparent);
    font-size: 19px;
    color: var(--ink);
  }

  .tool:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .tool.on .tool-glyph {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
    box-shadow: inset 0 0 0 2px var(--accent);
  }

  .tool.on {
    color: var(--ink);
  }

  .primary {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: 44px;
    padding: 0 16px;
    border: 1px solid var(--accent);
    border-radius: 11px;
    background: var(--accent);
    font: inherit;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--accent-ink);
    cursor: pointer;
  }

  .primary-glyph {
    display: grid;
    place-items: center;
    font-size: 19px;
  }

  .primary:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .verdict {
    border-top: 1px solid var(--line);
    padding-top: 12px;
  }

  .verdict .overall {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 0 0 8px;
    font-size: 1.5rem;
    font-variant-numeric: tabular-nums;
  }

  .verdict .overall span {
    font-size: 0.85rem;
    color: var(--muted);
  }

  .verdict .overall em {
    font-size: 0.85rem;
    font-style: normal;
    color: var(--bad);
  }

  .verdict.ok .overall em {
    color: var(--ok);
  }

  .scores {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
    margin: 0 0 10px;
  }

  .scores div {
    background: color-mix(in srgb, var(--line) 35%, transparent);
    border-radius: 8px;
    padding: 5px 8px;
  }

  .scores dt {
    font-size: 0.68rem;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .scores dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .strokes-list {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 0.84rem;
  }

  .strokes-list li {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .strokes-list .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--c);
  }

  .fine {
    color: var(--muted);
    font-size: 0.8rem;
    margin: 8px 0 0;
  }

  .note {
    margin: 0;
    font-size: 0.84rem;
    color: var(--muted);
  }

  .note.ok {
    color: var(--ok);
  }

  .note.bad {
    color: var(--bad);
  }

  @media (max-width: 760px) {
    .meta {
      padding: 8px 10px;
    }

    .big {
      font-size: 2.1rem;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .fold .chevron {
      transition: none;
    }
  }
</style>
