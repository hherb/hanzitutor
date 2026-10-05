<script lang="ts">
  /**
   * The kanji practice stage: one character on the board, its readings above it,
   * and the board's controls below it — and nothing else.
   *
   * ## Why it is a screen of its own
   *
   * The course used to be one page: grades, the lesson list, the ten characters
   * of a lesson, the board and the character's whole card, all stacked. On a
   * window that is a lot of scrolling and on a phone it is unusable — the board
   * sits below a lesson list and a character grid, so a learner scrolls past the
   * course to write one character and scrolls back to choose the next. N12 splits
   * it in two, which is the shape `HANDOVER_NIHONGO.md` invariant 33 records:
   * `KanjiPanel` is the course (grades and the lesson cards) and this is what a
   * card opens.
   *
   * ## The order of what is here
   *
   * The readings are **above** the board and the controls are **below** it, which
   * is the maintainer's own arrangement and Hanzi Tutor's: what you are writing
   * while you write it, then the thing you write on, then the tools. Inside a
   * lesson the arrows either side of the character are the only way on — no strip
   * of ten, which would be a second grid above a board that already has the ten
   * characters of the lesson one arrow away.
   *
   * ## The fold
   *
   * A character's card is much more than a reading — nanori, the radical, the
   * components it is built from, and the vocabulary written with it. All of that
   * between the readings and the board would put the board below the fold, so it
   * hides behind one `More` button, exactly as Hanzi Tutor folds its facts and
   * etymology. Unlike Hanzi Tutor the button is here at **every** width: that
   * app's stage has a sidebar to hold the facts on a window, and this one is a
   * single column, which the measurement in the stylesheet records.
   *
   * ## What is spoken, and what is never spoken
   *
   * Every reading is its own control and hands the **reading** to the voice, never
   * the character: `た.べる` is said as `たべる`, and 生 — twenty readings — is
   * never handed over at all, because the app does not choose between them. That
   * is `spokenReading` and invariant 27.
   */
  import Icon from "./Icon.svelte";
  import KanaCanvas from "./KanaCanvas.svelte";
  import SpeakButton from "./SpeakButton.svelte";
  import WordCard from "./WordCard.svelte";
  import { untrack } from "svelte";
  import { afterGrade } from "./board";
  import { VERDICT_COLOUR, VERDICT_LABEL } from "./render";
  import { scheduleNote } from "./review";
  import { canHear, voiceNote, type VoiceStatus } from "./speech";
  import { neighbour, positionIn, spokenReading } from "./kanji";
  import { pageWindow } from "./words";
  import * as api from "./api";
  import type {
    Drawable,
    GradeReport,
    KanjiLessonView,
    KanjiPick,
    KanjiView,
    Point,
    RadicalView,
    Word,
  } from "./types";

  interface Props {
    /**
     * Where to open: a character of a lesson, or a radical head form. The same
     * pair of shapes a deep link carries (`KanjiPick`), because it is the same
     * question — 92 of the 214 radicals are not jōyō characters, so a stage that
     * only understood characters could not be opened on one.
     *
     * A **new value remounts this component** (`KanjiPanel` keys it), so the
     * initial state below is read once and a later open does not have to be
     * merged into a screen that is already up.
     */
    start: KanjiPick;
    /**
     * The lesson whose characters the arrows step through, or `null` for a
     * radical head form — 92 of the 214 are not jōyō characters and belong to no
     * lesson at all.
     */
    lesson?: KanjiLessonView | null;
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice?: VoiceStatus;
    /** Open the radicals panel at one of the 214. */
    onradical?: (number: number) => void;
    /** Back to the course. */
    onleave: () => void;
  }

  let { start, lesson = null, voice, onradical, onleave }: Props = $props();

  /**
   * The character on the board, or null when a radical is.
   *
   * Read from `start` once, deliberately, and `untrack` is how that is said: a
   * new `start` **remounts** this component (the panel keys it), so there is
   * nothing here to react to, and the compiler's "this reference only captures
   * the initial value" warning is exactly the thing being asserted.
   */
  let ch = $state<string | null>(untrack(() => (start.kind === "kanji" ? start.ch : null)));
  let radicalNumber = $state<number | null>(
    untrack(() => (start.kind === "radical" ? start.number : null)),
  );

  let kanji = $state<KanjiView | null>(null);
  let radical = $state<RadicalView | null>(null);
  let report = $state<GradeReport | null>(null);
  let note = $state<{ tone: "ok" | "plain" | "bad"; text: string } | null>(null);
  let attempt = $state<Point[][]>([]);
  let board = $state<ReturnType<typeof KanaCanvas> | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  /**
   * Whether the faint copy is on the board: **tracing** while it is, **recall**
   * while it is not. The board itself clears the attempt when this changes.
   */
  let guide = $state(true);
  /** Whether the card's other facts are folded out. A phone control only. */
  let showDetails = $state(false);

  let words = $state<Word[]>([]);
  let wordTotal = $state(0);
  let wordPage = $state(1);
  let wordsBusy = $state(false);
  let openedWord = $state<Word | null>(null);

  /** How many words the card lists at once; the command is asked for the same. */
  const WORD_PAGE = 12;

  const drawn = $derived<Drawable | null>(kanji ?? radical);
  const wordWindow = $derived(pageWindow(wordTotal, WORD_PAGE, wordPage));
  const previous = $derived(ch ? neighbour(lesson, ch, -1) : null);
  const next = $derived(ch ? neighbour(lesson, ch, 1) : null);
  const at = $derived(ch ? positionIn(lesson, ch) : 0);
  const of = $derived(lesson?.kanji.length ?? 0);

  /**
   * What the primary control under the board offers: the verdict, then the way on.
   *
   * `board.ts`'s rule, shared with the kana stage, so the two cannot disagree
   * about what "graded" means for the control — and so that a **radical**, which
   * belongs to no lesson, keeps asking for a verdict rather than offering a way on
   * that does not exist.
   */
  const after = $derived(afterGrade(report !== null, lesson !== null, next !== null));

  /** One page of the words written with `ch`, and the count they are a page of. */
  async function loadWords(target: string, page: number) {
    try {
      error = null;
      wordsBusy = true;
      openedWord = null;
      const window_ = pageWindow(wordTotal, WORD_PAGE, page);
      const found = await api.wordsOfKanji(target, window_.offset, window_.limit);
      words = found.words;
      wordTotal = found.total;
      wordPage = window_.page;
    } catch (e) {
      error = String(e);
    } finally {
      wordsBusy = false;
    }
  }

  async function openKanji(target: string) {
    try {
      error = null;
      kanji = await api.kanji(target);
      radical = null;
      radicalNumber = null;
      report = null;
      note = null;
      attempt = [];
      showDetails = false;
      await loadWords(target, 1);
    } catch (e) {
      error = String(e);
    }
  }

  /** A radical head form, which the character course cannot put on the board. */
  async function openRadical(number: number) {
    try {
      error = null;
      radical = await api.radical(number);
      kanji = null;
      report = null;
      note = null;
      attempt = [];
      showDetails = false;
      words = [];
      wordTotal = 0;
      wordPage = 1;
      openedWord = null;
    } catch (e) {
      error = String(e);
    }
  }

  /**
   * Open whatever is asked for, and only that.
   *
   * The effect reads the two slots it acts on and writes neither of them, which is
   * the shape `HANDOVER_NIHONGO.md` trap 9 is about: an effect that read what it
   * writes would never settle. Stepping through a lesson is a write to `ch`, and
   * that is what re-runs this.
   */
  $effect(() => {
    const target = ch;
    const number = radicalNumber;
    if (target) void openKanji(target);
    else if (number !== null) void openRadical(number);
  });

  function step(delta: number) {
    const target = neighbour(lesson, ch ?? "", delta);
    if (target) ch = target;
  }

  function onStrokes(strokes: Point[][]) {
    attempt = strokes;
    // A verdict belongs to the attempt it judged: the moment the attempt changes
    // it is stale, and leaving it up colours the new strokes with the old reading.
    report = null;
    note = null;
  }

  async function askForAVerdict() {
    if (!drawn) return;
    try {
      busy = true;
      error = null;
      // Grading and scheduling are one command: the report comes back with what
      // the review schedule did with the attempt beside it.
      const graded = await api.gradeAttempt(drawn.ch, attempt);
      report = graded.report;
      note = scheduleNote(graded, new Date().toISOString());
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function turnWordPage(delta: number) {
    if (!kanji) return;
    void loadWords(kanji.ch, wordWindow.page + delta);
  }

  function isOpen(word: Word): boolean {
    return openedWord?.text === word.text && openedWord?.reading === word.reading;
  }

  const summary = $derived.by(() => {
    if (!report) return null;
    return {
      total: report.strokes.length,
      wrong: report.strokes.filter((s) => s.verdict !== "correct" && s.userIndex !== null),
    };
  });
</script>

{#snippet readingRow(list: string[])}
  {#each list as reading, index (reading)}
    <!-- The separator belongs to the reading *before* it, so a line that wraps
         ends with a "·" rather than beginning with one. -->
    <span class="reading">
      <SpeakButton
        text={spokenReading(reading)}
        {voice}
        label={reading}
        note={false}
        appearance="link"
      />
      {#if index < list.length - 1}<span class="sep">·</span>{/if}
    </span>
  {/each}
{/snippet}

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
      <div class="nav" role="group" aria-label="Characters in this lesson">
        <button
          class="step"
          disabled={!previous}
          onclick={() => step(-1)}
          aria-label={previous ? `Previous character: ${previous}` : "This is the first character of the lesson"}
          title={previous ? `Previous character (${previous})` : "This is the first character of the lesson"}
        >
          <span class="flip"><Icon name="next" /></span>
        </button>
        <span class="position" title="{lesson.title} · lesson {at} of {of}">
          {at} / {of}
        </span>
        <button
          class="step"
          disabled={!next}
          onclick={() => step(1)}
          aria-label={next ? `Next character: ${next}` : "This is the last character of the lesson"}
          title={next ? `Next character (${next})` : "This is the last character of the lesson"}
        >
          <Icon name="next" />
        </button>
      </div>
    {/if}
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if drawn}
    <!--
      The readings, above the board. `on` first because that is the order
      KANJIDIC2 gives them and the order a dictionary lists them in, and every
      reading is a control that **says the reading** rather than the character:
      invariant 27.
    -->
    <div class="meta" class:open={showDetails}>
      <div class="who">
        <span class="big" lang="ja">{drawn.ch}</span>
        <div class="gloss">
          <!--
            Above the board: **the readings and nothing else**. The meaning, the
            stroke count, the grade, the frequency, the radical, the components and
            the words are all behind `More` — the maintainer's own instruction, and
            the reason is the screen: at 390px the meaning alone pushed the board
            down by a line, and the board is the action this screen exists for.
          -->
          <dl class="readings">
            {#if kanji && kanji.on.length}
              <div><dt>on</dt><dd lang="ja">{@render readingRow(kanji.on)}</dd></div>
            {/if}
            {#if kanji && kanji.kun.length}
              <div><dt>kun</dt><dd lang="ja">{@render readingRow(kanji.kun)}</dd></div>
            {/if}
            <!-- A radical has no readings; what identifies it is its number and
                 how much it unlocks, which is the same kind of fact. -->
            {#if radical}
              <div><dt>radical</dt><dd>{radical.number} of 214</dd></div>
              <div><dt>used by</dt>
                <dd>
                  {radical.characters.length}
                  {radical.characters.length === 1 ? "jōyō character" : "jōyō characters"}
                </dd>
              </div>
            {/if}
          </dl>

          <button
            class="fold"
            type="button"
            aria-expanded={showDetails}
            aria-controls="kanji-details"
            onclick={() => (showDetails = !showDetails)}
          >
            {showDetails ? "Less" : "More"}
            <span class="chevron" class:open={showDetails} aria-hidden="true">›</span>
          </button>

          <div class="details" id="kanji-details">
            {#if kanji}
              <p class="hint">
                {#if canHear(voice)}
                  Write it on the board, or tap any reading to hear it.
                {:else}
                  {voiceNote(voice)}
                {/if}
              </p>
              {#if kanji.meanings.length}
                <dl class="readings">
                  <div><dt>meaning</dt><dd>{kanji.meanings.join("; ")}</dd></div>
                </dl>
              {/if}
              {#if kanji.nanori.length}
                <dl class="readings">
                  <div><dt>nanori</dt><dd lang="ja">{@render readingRow(kanji.nanori)}</dd></div>
                </dl>
              {/if}

              <ul class="facts">
                <li>{drawn.strokeCount} {drawn.strokeCount === 1 ? "stroke" : "strokes"}</li>
                <li>{kanji.gradeName}</li>
                {#if kanji.frequency}<li>frequency rank {kanji.frequency}</li>{/if}
              </ul>

              <div class="structure">
                <p class="structure-line">
                  Radical <strong>{kanji.radical.number}</strong>
                  <span class="glyph">{kanji.radical.ch}</span>
                  {#if kanji.radical.form !== kanji.radical.ch}
                    <span class="aside">written {kanji.radical.form} here</span>
                  {/if}
                  {#if kanji.radical.note && kanji.radical.note !== kanji.radical.ch}
                    <span class="aside">the source notes {kanji.radical.note}</span>
                  {/if}
                  <span class="aside">{kanji.radical.characters} share it</span>
                  <button
                    class="link"
                    onclick={() => onradical?.(kanji!.radical.number)}
                    title="Show every character that uses this radical"
                  >
                    See the family
                  </button>
                </p>

                {#if kanji.decomposition.raw}
                  <p class="structure-line">
                    Made of {kanji.decomposition.layout}
                    <code>{kanji.decomposition.raw}</code>
                  </p>
                  <ul class="parts" aria-label="Components">
                    {#each kanji.decomposition.parts as part, index (index)}
                      <li>
                        {#if part.ch && part.drawable}
                          <button
                            class="cell small"
                            onclick={() => (ch = part.ch as string)}
                            title="Write {part.ch} on the board"
                          >
                            {part.ch}
                          </button>
                        {:else}
                          <span
                            class="cell small unknown"
                            title="Not one of the jōyō characters, so the board cannot write it"
                          >
                            {part.ch ?? "？"}
                          </span>
                        {/if}
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <p class="hint">
                    The source gives no decomposition for this character: it is a single
                    glyph rather than a composition.
                  </p>
                {/if}
              </div>
            {:else if radical}
              <div class="structure">
                {#if radical.characters.length > 0}
                  <p class="structure-line">The characters classified under it, most frequent first</p>
                  <ul class="parts" aria-label="Characters using this radical">
                    {#each radical.characters as member (member)}
                      <li>
                        <button
                          class="cell small"
                          onclick={() => (ch = member)}
                          title="Write {member} on the board"
                        >
                          {member}
                        </button>
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <p class="hint">
                    No jōyō character uses this radical, which is why the course never asks
                    for it. It is here so the set of 214 is complete.
                  </p>
                {/if}
                <p class="structure-line">
                  <button
                    class="link"
                    onclick={() => onradical?.(radical!.number)}
                    title="Open this radical in the radicals panel"
                  >
                    See it in the radicals panel
                  </button>
                </p>
              </div>
            {/if}

            <!--
              The words the character is written in: the vocabulary is what a
              character arrives through, and each row opens that word's own card,
              which is the same affordance a tapped passage token has.
            -->
            {#if kanji}
              <div class="words">
                {#if wordsBusy}
                  <p class="hint">Loading…</p>
                {:else}
                  <p class="structure-line">
                    {#if wordTotal === 0}
                      <span class="aside">
                        No word this course teaches is written with {kanji.ch}.
                      </span>
                    {:else}
                      <strong>{wordTotal.toLocaleString()}</strong>
                      {wordTotal === 1 ? "word" : "words"} this course teaches
                      {wordTotal === 1 ? "is" : "are"} written with {kanji.ch}
                      {#if wordWindow.pages > 1}
                        <span class="aside">page {wordWindow.page} of {wordWindow.pages}</span>
                      {/if}
                    {/if}
                  </p>

                  {#if words.length > 0}
                    <ul class="word-list" aria-label="Words written with {kanji.ch}">
                      {#each words as entry (entry.text + entry.reading)}
                        <li>
                          <button
                            class="word-row"
                            class:active={isOpen(entry)}
                            title={isOpen(entry)
                              ? "Put this word's card away"
                              : "Open this word's own card"}
                            onclick={() => (openedWord = isOpen(entry) ? null : entry)}
                          >
                            <span class="word-text">{entry.text}</span>
                            <span class="word-reading">{entry.reading}</span>
                            <span class="word-gloss">{entry.meaning}</span>
                          </button>
                        </li>
                      {/each}
                    </ul>

                    {#if wordWindow.pages > 1}
                      <nav class="pager" aria-label="Word pages">
                        <button disabled={wordWindow.page <= 1} onclick={() => turnWordPage(-1)}>
                          Previous
                        </button>
                        <span>page {wordWindow.page} of {wordWindow.pages}</span>
                        <button
                          disabled={wordWindow.page >= wordWindow.pages}
                          onclick={() => turnWordPage(1)}
                        >
                          Next
                        </button>
                      </nav>
                    {/if}
                  {/if}
                {/if}

                {#if openedWord}
                  <div class="word-slot">
                    <WordCard word={openedWord} {voice} />
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      </div>
    </div>

    <KanaCanvas bind:this={board} character={drawn} {report} {guide} onchange={onStrokes} />

    <!--
      Below the board, only the tools: nothing that is a *fact* about the
      character lives down here. Three groups, as Hanzi Tutor draws them — what
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
              ? "Hint: a faint copy of the character is on the board. Switch to recall, which hides it"
              : "Hint: the character is hidden while you write it. Switch to tracing, where a faint copy is on the board"}
            title={guide
              ? "The character is on the board to follow. Press to hide it and write it from memory — either way the board is cleared."
              : "Write it from memory. Press to put a faint copy back on the board — either way the board is cleared."}
          >
            <span class="tool-glyph"><Icon name={guide ? "eye" : "eye-off"} /></span>
            <span class="tool-word">Hint</span>
          </button>
          <button
            class="tool"
            onclick={() => board?.animate()}
            disabled={drawn.strokeCount === 0}
            aria-label="Strokes: watch the character written one stroke at a time"
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
              aria-label="Next character: {next}"
              title="On to {next}"
            >
              <span class="primary-glyph"><Icon name="next" /></span>
              <span>Next</span>
            </button>
          {:else if after === "finish"}
            <!-- The last character of the lesson: there is no next one, and the way
                 on is the course the stage was opened from. -->
            <button
              class="primary"
              onclick={onleave}
              aria-label="Back to the lessons: this is the last character of the lesson"
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

      {#if note}
        <p class="note" class:ok={note.tone === "ok"} class:bad={note.tone === "bad"}>
          {note.text}
        </p>
      {/if}
    </div>
  {/if}
</section>

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

  /* The lesson's own navigation, in the corner where the character is not: the
     board below is the thing being looked at, and this says which of the ten it
     is holding. */
  .nav {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
  }

  /* 44px is the size a finger needs, and every control on this stage is that tall
     on a phone: measured in the harness at 390px, the back button was 25px and the
     fold 40px, and the arrows and the Grade button were 40 as well — all four are
     fixed, and the arrows are 44 here because a stage is where a thumb reaches. */
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

  /* The readings and the board's explanation, above the board. */
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
    overflow-wrap: anywhere;
  }

  .readings .reading {
    /* Reading and separator wrap together, so the "·" never opens a line. */
    display: inline-block;
    white-space: nowrap;
  }

  .readings .sep {
    margin: 0 0.35rem;
    color: var(--muted);
  }

  .hint {
    margin: 6px 0 0;
    font-size: 0.78rem;
    color: var(--muted);
    max-width: 72ch;
  }

  /* The fold. Hanzi Tutor shows it only on a phone, because on a window its
     stage sits beside a sidebar with room for the facts. This stage is **one
     column at every width**, so the fold is always here and the details are
     always behind it: measured at 980px, the expanded card pushed the board to
     y=1160 — the scrolling this milestone exists to remove, moved rather than
     fixed. Folded, the board follows the readings. */
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

  /* The readings above the board are **not** clamped, even when they are long:
     they are the whole of what that block is for, and 生's twenty would be
     hidden rather than folded. Everything that is prose about the character is
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

  .structure {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .structure-line {
    margin: 0;
    font-size: 0.86rem;
    display: flex;
    align-items: center;
    gap: 7px;
    flex-wrap: wrap;
  }

  .structure-line code {
    font-size: 0.8rem;
    color: var(--muted);
  }

  .glyph {
    font-size: 1.4rem;
    line-height: 1;
  }

  .aside {
    color: var(--muted);
    font-size: 0.78rem;
  }

  .link {
    padding: 3px 9px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
    font-size: 0.78rem;
  }

  .link:hover {
    border-color: var(--accent);
  }

  .parts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }

  .parts .cell.small {
    font-size: 1.25rem;
    width: 2.2rem;
    height: 2.2rem;
    aspect-ratio: auto;
  }

  .unknown {
    display: grid;
    place-items: center;
    border: 1px dashed var(--line);
    border-radius: var(--radius);
    background: none;
    color: var(--muted);
    cursor: default;
  }

  .words {
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px dashed var(--line);
    padding-top: 10px;
  }

  .word-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  .word-row {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(3.5rem, auto) minmax(3.5rem, auto) 1fr;
    gap: 0.6rem;
    align-items: baseline;
    text-align: left;
    font: inherit;
    padding: 0.3rem 0.45rem;
    border: 1px solid transparent;
    border-radius: 6px;
    background: none;
    color: var(--ink);
    cursor: pointer;
  }

  .word-row:hover {
    background: var(--hover, #f4f4f1);
  }

  .word-row.active {
    border-color: var(--accent);
    background: var(--accent-soft, #eaf3ed);
  }

  .word-text {
    font-size: 1.1rem;
  }

  .word-reading {
    color: var(--muted);
    font-size: 0.88rem;
  }

  .word-gloss {
    color: var(--muted);
    font-size: 0.82rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pager {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 0.82rem;
    color: var(--muted);
  }

  .pager button {
    font: inherit;
    padding: 0.25rem 0.55rem;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
  }

  .pager button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* The board's own controls: a glyph on a disc with its short name under it,
     two or three tools to a card, and the cards centred under the board — Hanzi
     Tutor's control row, which is where the glyphs and the short names come
     from. The colours are this app's variables rather than that app's. */
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

  /* On a phone the details fold away, which is Hanzi Tutor's arrangement: the
     readings and the meaning stay above the board and the strokes, the radical,
     the components and the words wait behind the button. A window wide enough to
     hold them shows them, and has no button to fold. */
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
