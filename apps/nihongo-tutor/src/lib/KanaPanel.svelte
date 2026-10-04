<script lang="ts">
  /**
   * The kana course: which script, and one card per lesson.
   *
   * ## Two screens, and this is the first one
   *
   * The kana course used to be a single page — the script toggle, the lesson list,
   * the lesson's kana and the board with everything under it, stacked in three
   * columns — which meant scrolling past the course to reach the board on a phone
   * and scrolling back to choose the next kana. N12 took that shape out of the
   * kanji course and N13 is the same repair here: this screen is **the course**
   * (which script, which lesson), and choosing a lesson opens `KanaPractice`,
   * which is the board. `HANDOVER_NIHONGO.md` invariants 33 and 34 are the rule.
   *
   * ## A card shows its kana
   *
   * Each card carries the kana of its lesson, because that is what the choice is
   * between: a lesson's title names its own kana and its sound in one string — and
   * which half comes first depends on the kind of lesson it is (`Kana`'s docs and
   * `kana.ts`'s `lessonLabel`) — so the card draws the kana itself and the sound
   * beside it. Tapping a card opens the lesson's first kana on the board.
   *
   * ## The script toggle stays on this screen
   *
   * Hiragana and katakana are two courses of the same 86-odd characters, and the
   * toggle is how a learner chooses which to work on: it belongs to the course, not
   * to the board. The stage takes the whole screen, so the toggle is not drawn
   * while one is up — `Lessons` is the way back to it, which is invariant 33's
   * arrangement and 150px of a phone's height returned to the board.
   */
  import KanaPractice from "./KanaPractice.svelte";
  import { lessonLabel } from "./kana";
  import { untrack } from "svelte";
  import * as api from "./api";
  import type { KanaPick, LessonView, ScriptName } from "./types";
  import type { VoiceStatus } from "./speech";

  interface Props {
    /**
     * A kana another screen has asked for, with the script it is written in. It is
     * consumed here — cleared as soon as it has been opened — so that returning to
     * the tab later does not reopen something the learner left.
     */
    pick?: KanaPick | null;
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice?: VoiceStatus;
    /**
     * Whether a stage is up, told to `App.svelte` so it can take the course switch
     * and the tab row out of the way while one is.
     */
    stage?: boolean;
  }

  let { pick = $bindable(null), voice, stage = $bindable(false) }: Props = $props();

  const SCRIPTS: readonly ScriptName[] = ["hiragana", "katakana"];

  /**
   * The script on screen — started on the script of the request that opened the
   * course, when there is one.
   *
   * That initial value is not a micro-optimisation: a kana opened from the chart
   * arrives with its script, and a panel that always started on hiragana would
   * load the wrong course, drop the answer, and load the right one. The request
   * itself is still honoured below, whatever the order the two arrive in.
   */
  let script = $state<ScriptName>(untrack(() => pick?.script ?? "hiragana"));
  let course = $state<LessonView[]>([]);
  /**
   * The script whose course is in `course`, or `null` while one loads.
   *
   * `script` is the toggle and this is what is actually loaded, and they disagree
   * for as long as a load takes — which is exactly when a request has to know
   * better than to look a kana up in the wrong course.
   */
  let loadedScript = $state<ScriptName | null>(null);
  let error = $state<string | null>(null);
  /** The kana a lesson card or another screen opened, or `null` for the course. */
  let practising = $state<string | null>(null);

  const kanaCount = $derived(course.reduce((sum, lesson) => sum + lesson.count, 0));

  /**
   * Keep the app told whether a stage is up.
   *
   * One place rather than three: `practising` is set by a card, by a request from
   * another screen and by leaving, and a flag written at each of those would be
   * wrong the day a fourth caller arrives. The effect reads `practising` and
   * writes the prop, which the app never writes back — so there is nothing here to
   * loop.
   */
  $effect(() => {
    stage = practising !== null;
  });

  $effect(() => {
    void load(script);
  });

  async function load(which: ScriptName) {
    try {
      error = null;
      const lessons = await api.lessons(which);
      // A load that a later toggle has overtaken is dropped rather than applied:
      // the two answers can arrive in either order, and the course on screen has
      // to be the script the learner last chose.
      if (untrack(() => script) !== which) return;
      course = lessons;
      loadedScript = which;
    } catch (e) {
      error = String(e);
      loadedScript = null;
      // The request goes with the course that failed to load, rather than staying
      // to be applied to whichever script loads next. That is the race N4 fixed
      // once for a single course, and the reason the request carries its script.
      if (untrack(() => pick)?.script === which) pick = null;
    }
  }

  /**
   * What another screen asked for, opened once the course it belongs to is up.
   *
   * A request whose script is not the loaded one **switches to it and waits**,
   * which is the whole reason `KanaPick` carries a script: honouring it against
   * the course that happens to be loaded would open a katakana kana among the
   * hiragana lessons. It is read and cleared here rather than passed down, so
   * returning to the tab later does not reopen what the learner left.
   *
   * The effect writes `script` only on the run that has not switched yet, and
   * `pick` only once — an effect that read what it wrote would never settle
   * (`HANDOVER_NIHONGO.md` trap 9).
   */
  $effect(() => {
    const wanted = pick;
    if (!wanted) return;
    if (loadedScript !== wanted.script) {
      if (script !== wanted.script) script = wanted.script;
      return;
    }
    if (course.length === 0) return;
    pick = null;
    practising = wanted.ch;
  });

  /** Open a lesson on its first kana, which is the one the course teaches first. */
  function openLesson(lesson: LessonView) {
    const first = lesson.kana[0];
    if (!first) return;
    practising = first;
  }
</script>

{#if practising}
  <!--
    Keyed on the kana it was opened on, so a request from another screen mounts a
    fresh stage rather than merging the new kana into the state of the old one.
  -->
  {#key practising}
    <KanaPractice
      start={practising}
      {course}
      {voice}
      onleave={() => (practising = null)}
    />
  {/key}
{:else}
  <section class="course">
    <header>
      <h2>Kana</h2>
      {#if course.length > 0}
        <p class="sub">
          {kanaCount} kana · {course.length} lessons
        </p>
      {/if}
      <p class="hint">
        A lesson is one row of the syllabary, in the order the language lists it, plus
        the characters that are not on the grid at all. Choose a lesson to open its kana
        on the board.
      </p>
    </header>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <div class="tabs" role="tablist" aria-label="Script">
      {#each SCRIPTS as name (name)}
        <button
          role="tab"
          aria-selected={script === name}
          class:active={script === name}
          onclick={() => (script = name)}
        >
          {name}
        </button>
      {/each}
    </div>

    <ul class="lessons" aria-label="Lessons">
      {#each course as lesson (lesson.key)}
        <li>
          <button
            class="lesson"
            class:voiced={lesson.voiced}
            onclick={() => openLesson(lesson)}
            title="Open {lessonLabel(lesson)} on the board"
          >
            <span class="title">{lessonLabel(lesson)}</span>
            <span class="cluster" lang="ja" aria-hidden="true">
              {#each lesson.kana as ch (ch)}<span class="glyph">{ch}</span>{/each}
            </span>
            <span class="count">
              {lesson.count} kana{lesson.voiced ? " · voiced" : ""}
            </span>
          </button>
        </li>
      {/each}
    </ul>

    {#if course.length === 0 && !error}
      <p class="hint">Loading the course…</p>
    {/if}

    <p class="hint">Yōon are two kana that make one mora — type them together.</p>
  </section>
{/if}

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

  /* The lesson cards: a lesson is a row of five kana at most, so they fill the
     width three or four to a row rather than sitting in a column beside a board —
     the board is a screen of its own now. */
  .lessons {
    list-style: none;
    margin: 14px 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 8px;
  }

  .lesson {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .lesson:hover {
    border-color: var(--accent);
  }

  /* A voiced row is written from its plain row rather than learned from nothing;
     the same mark the old lesson list made on the left edge. */
  .lesson.voiced {
    border-left: 3px solid color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .lesson .title {
    font-size: 0.9rem;
    color: var(--muted);
  }

  /* The kana of the lesson, which is what the choice is between. */
  .cluster {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    font-size: 1.5rem;
    line-height: 1.2;
  }

  .glyph {
    min-width: 1.15em;
    text-align: center;
  }

  .lesson .count {
    font-size: 0.74rem;
    color: var(--muted);
  }
</style>
