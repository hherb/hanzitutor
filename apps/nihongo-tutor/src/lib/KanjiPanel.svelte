<script lang="ts">
  /**
   * The kanji course: the grades, one lesson's characters, the board, and the
   * card for whatever is on it.
   *
   * ## The order, and why it is this one
   *
   * The grades are KANJIDIC2's current kyōiku assignment — 1 to 6 and then the
   * jōyō remainder — because that is the order Japanese schools teach in, and
   * inside a grade the most frequent characters come first. There is no
   * frequency list *within* a grade in the data, only between grades, so
   * frequency is the honest way to slice each into lessons of ten.
   *
   * ## The ladder is the vocabulary's
   *
   * A grade's name comes from the payload (`nihongo_core::grade_name`) and is the
   * same rung the vocabulary's bands use, so "kyōiku 3" means one thing in both
   * screens. `ROADMAP_NIHONGO.md` left that open for this milestone; this is it
   * answered, and the screen says it rather than leaving a learner to notice.
   *
   * ## One board for three kinds of thing
   *
   * The card can be a **jōyō character**, selected from the course or asked for by
   * a component, or a **radical head form**, asked for by the radicals panel —
   * 92 of the 214 are not jōyō characters, so the board would have no way to draw
   * them otherwise. Both are `Drawable`, both are graded by the same command, and
   * the board measures the two identically.
   */
  import KanaCanvas from "./KanaCanvas.svelte";
  import SpeakButton from "./SpeakButton.svelte";
  import { VERDICT_COLOUR, VERDICT_LABEL } from "./render";
  import { scheduleNote } from "./review";
  import { canHear, voiceNote, type VoiceStatus } from "./speech";
  import { gradeTabs, lessonHolding, lessonsIn, spokenReading } from "./kanji";
  import * as api from "./api";
  import type {
    Drawable,
    GradeReport,
    KanjiLessonView,
    KanjiPick,
    KanjiView,
    Point,
    RadicalView,
  } from "./types";

  interface Props {
    /**
     * A character or radical another screen has asked for. It is consumed here —
     * cleared as soon as it has been opened — so that returning to the tab later
     * does not reopen something the learner left.
     */
    pick?: KanjiPick | null;
    /** Open the radicals panel at one of the 214. */
    onradical?: (number: number) => void;
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice?: VoiceStatus;
  }

  let { pick = $bindable(null), onradical, voice }: Props = $props();

  let lessons = $state<KanjiLessonView[]>([]);
  let grade = $state(1);
  let lessonKey = $state<string | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /** The character whose card is up, and the radical being written: one of them. */
  let kanji = $state<KanjiView | null>(null);
  let radical = $state<RadicalView | null>(null);
  let report = $state<GradeReport | null>(null);
  /** What the review schedule did with the last attempt, if anything. */
  let note = $state<{ tone: "ok" | "plain" | "bad"; text: string } | null>(null);
  let attempt = $state<Point[][]>([]);
  let board = $state<ReturnType<typeof KanaCanvas> | null>(null);

  const tabs = $derived(gradeTabs(lessons));
  const inGrade = $derived(lessonsIn(lessons, grade));
  const activeLesson = $derived(inGrade.find((l) => l.key === lessonKey) ?? null);
  /** What the board draws and the grader is asked about. */
  const drawn = $derived<Drawable | null>(kanji ?? radical);

  async function loadLessons() {
    try {
      lessons = await api.kanjiLessons();
      const first = lessons[0];
      if (first) {
        grade = first.grade;
        lessonKey = first.key;
        if (first.kanji[0]) await select(first.kanji[0]);
      }
    } catch (e) {
      error = String(e);
    }
  }
  void loadLessons();

  /** Put a jōyō character on the board, and land on the lesson that teaches it. */
  async function select(ch: string) {
    try {
      error = null;
      kanji = await api.kanji(ch);
      radical = null;
      report = null;
      note = null;
      attempt = [];
      const found = lessonHolding(lessons, ch);
      if (found) {
        grade = found.grade;
        lessonKey = found.key;
      }
    } catch (e) {
      error = String(e);
    }
  }

  /** Put a radical head form on the board, which the character course cannot. */
  async function selectRadical(number: number) {
    try {
      error = null;
      radical = await api.radical(number);
      kanji = null;
      report = null;
      note = null;
      attempt = [];
    } catch (e) {
      error = String(e);
    }
  }

  function chooseGrade(next: number) {
    grade = next;
    lessonKey = lessonsIn(lessons, next)[0]?.key ?? null;
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

  const summary = $derived.by(() => {
    if (!report) return null;
    return {
      total: report.strokes.length,
      wrong: report.strokes.filter((s) => s.verdict !== "correct" && s.userIndex !== null),
    };
  });

  /**
   * What another screen asked for, opened once.
   *
   * It waits for the course before consuming the request, because a character
   * asked for from a component has to land on the right lesson — and the lessons
   * are what say where that is.
   */
  $effect(() => {
    const wanted = pick;
    if (!wanted) return;
    if (lessons.length === 0) return;
    pick = null;
    if (wanted.kind === "kanji") void select(wanted.ch);
    else void selectRadical(wanted.number);
  });
</script>

<section class="course">
  <header>
    <h2>Kanji</h2>
    {#if tabs.length > 0}
      <p class="sub">
        {tabs.reduce((sum, tab) => sum + tab.kanji, 0).toLocaleString()} jōyō characters ·
        {tabs.filter((t) => t.grade <= 6).reduce((sum, t) => sum + t.kanji, 0).toLocaleString()}
        kyōiku · {tabs.reduce((sum, tab) => sum + tab.lessons, 0)} lessons
      </p>
    {/if}
    <p class="hint">
      The grades are KANJIDIC2's current assignment and the order is the one schools
      teach in; inside a grade the most frequent characters come first. A character is
      drawn and graded by the same engine that grades a kana.
    </p>
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <div class="tabs" role="tablist" aria-label="Grades">
    {#each tabs as tab (tab.grade)}
      <button
        role="tab"
        aria-selected={grade === tab.grade}
        class:active={grade === tab.grade}
        onclick={() => chooseGrade(tab.grade)}
      >
        {tab.name} <span class="count">{tab.kanji}</span>
      </button>
    {/each}
  </div>

  <div class="layout">
    <nav aria-label="Lessons">
      {#each inGrade as lesson (lesson.key)}
        <button
          class="lesson"
          class:active={lesson.key === lessonKey}
          onclick={() => (lessonKey = lesson.key)}
        >
          <span class="title">{lesson.title}</span>
          <span class="sound">{lesson.count} characters</span>
        </button>
      {/each}
    </nav>

    <section class="picker" aria-label="Characters in this lesson">
      {#if activeLesson}
        <div class="grid">
          {#each activeLesson.kanji as ch (ch)}
            <button class="cell" class:active={ch === kanji?.ch} onclick={() => select(ch)}>
              {ch}
            </button>
          {/each}
        </div>
        <p class="hint">
          {activeLesson.gradeName} · lesson {activeLesson.title}
        </p>
      {/if}
    </section>

    <section class="practice">
      {#if drawn}
        <div class="prompt">
          <span class="big">{drawn.ch}</span>
          <span class="reading">
            {#if kanji}
              {kanji.gradeName}{#if kanji.frequency}
                <small>· frequency rank {kanji.frequency}</small>{/if}
            {:else if radical}
              Kangxi radical {radical.number}
            {/if}
          </span>
          <span class="strokes">{drawn.strokeCount}
            {drawn.strokeCount === 1 ? "stroke" : "strokes"}</span>
        </div>

        <KanaCanvas bind:this={board} character={drawn} {report} onchange={onStrokes} />

        <div class="actions">
          <button onclick={() => board?.animate()}>Show stroke order</button>
          <button onclick={() => board?.undo()}>Undo</button>
          <button onclick={() => board?.clear()}>Clear</button>
          <button
            class="primary"
            disabled={busy || attempt.length === 0}
            onclick={askForAVerdict}
          >
            Grade
          </button>
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

        {#if kanji}
          <div class="card">
            <!--
              The hearing half of the card, beside the board that is its writing
              half. A learner may want either, and the two are offered together
              rather than one being assumed: `h` speaks what is on the board on
              Practice, and here every reading the card lists can be tapped.

              What is spoken is the *reading*, never the character: `た.べる` is
              handed to the voice as `たべる`, and 生 is never handed over at all,
              because it has twenty readings and the app does not choose between
              them. See `HANDOVER_NIHONGO.md` invariant 27.
            -->
            <p class="hint">
              {#if canHear(voice)}
                Write {kanji.ch} on the board above, or tap any reading to hear it.
              {:else}
                {voiceNote(voice)}
              {/if}
            </p>

            {#snippet readingRow(list: string[])}
              {#each list as reading, index (reading)}
                <!-- The separator belongs to the reading *before* it, so a line
                     that wraps ends with a "·" rather than beginning with one —
                     which is what a row of twelve nanori does at this width. -->
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

            <dl class="readings">
              {#if kanji.on.length}
                <div><dt>on</dt><dd lang="ja">{@render readingRow(kanji.on)}</dd></div>
              {/if}
              {#if kanji.kun.length}
                <div><dt>kun</dt><dd lang="ja">{@render readingRow(kanji.kun)}</dd></div>
              {/if}
              {#if kanji.meanings.length}
                <div><dt>meaning</dt><dd>{kanji.meanings.join("; ")}</dd></div>
              {/if}
              {#if kanji.nanori.length}
                <div><dt>nanori</dt><dd lang="ja">{@render readingRow(kanji.nanori)}</dd></div>
              {/if}
            </dl>

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
                          onclick={() => select(part.ch as string)}
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
          </div>
        {:else if radical}
          <div class="card">
            <dl class="readings">
              <div><dt>radical</dt><dd>{radical.number} of 214</dd></div>
              <div><dt>strokes</dt><dd>{radical.strokeCount}</dd></div>
              <div>
                <dt>used by</dt>
                <dd>
                  {radical.characters.length}
                  {radical.characters.length === 1 ? "jōyō character" : "jōyō characters"}
                </dd>
              </div>
            </dl>

            <div class="structure">
              {#if radical.characters.length > 0}
                <p class="structure-line">The characters classified under it, most frequent first</p>
                <ul class="parts" aria-label="Characters using this radical">
                  {#each radical.characters as ch (ch)}
                    <li>
                      <button
                        class="cell small"
                        onclick={() => select(ch)}
                        title="Write {ch} on the board"
                      >
                        {ch}
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
          </div>
        {/if}
      {/if}
    </section>
  </div>
</section>

<style>
  .course header h2 {
    margin: 0;
  }

  .sub {
    margin: 0.2rem 0 0;
    color: var(--muted);
  }

  .hint {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: var(--muted);
    max-width: 72ch;
  }

  /* The shared `.tabs` rule capitalises its labels, which suits `hiragana` and
     `katakana`. These are the ladder's own names — "kyōiku 1", "jōyō beyond the
     school grades" — and title-casing them turns a grade into a proper noun. They
     also wrap, because seven of them are wider than one row at this width. */
  .tabs {
    flex-wrap: wrap;
  }

  .tabs button {
    text-transform: none;
  }

  .tabs .count {
    color: var(--muted);
    font-size: 0.8rem;
  }

  .tabs button.active .count {
    color: var(--accent-ink);
    opacity: 0.8;
  }

  /* The lesson row's label is a position range rather than the kana a kana lesson
     shows, so it needs no letter-spacing. */
  .lesson .title {
    font-size: 1rem;
    font-variant-numeric: tabular-nums;
  }

  .card {
    margin-top: 14px;
    border-top: 1px solid var(--line);
    padding-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .readings {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 6px 12px;
    margin: 0;
  }

  .readings div {
    min-width: 0;
  }

  .readings dt {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .readings dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  /* The separator between two readings, now that each reading is its own control
     rather than one joined string. */
  .readings .reading {
    /* Reading and separator wrap together, so the "·" never opens a line. */
    display: inline-block;
    white-space: nowrap;
  }

  .readings .sep {
    margin: 0 0.35rem;
    color: var(--muted);
  }

  .card .hint {
    margin: 0;
    font-size: 0.8rem;
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
    font-size: 1.5rem;
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
</style>
