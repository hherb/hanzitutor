<script lang="ts">
  /**
   * The kanji course: the grades, and the lesson cards they hold.
   *
   * ## Two screens, and this is the first one
   *
   * The course used to be a single page — grades, lesson list, character grid,
   * board and the character's whole card stacked in three columns — which meant
   * scrolling past the course to reach the board on a phone and scrolling back to
   * choose the next character. N12 splits it: this screen is **the course** (which
   * grade, which lesson of ten), and choosing a lesson opens `KanjiPractice`,
   * which is the board. `HANDOVER_NIHONGO.md` invariant 33 is the rule.
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
   * ## A card shows its cluster
   *
   * Each card carries the ten characters of its lesson, because that is what the
   * choice is between: "1–10" names a range and tells a learner nothing about
   * whether 日 is in it. Tapping one opens the lesson's first character on the
   * board.
   */
  import KanjiPractice from "./KanjiPractice.svelte";
  import { gradeTabs, lessonHolding, lessonsIn } from "./kanji";
  import * as api from "./api";
  import type { KanjiLessonView, KanjiPick } from "./types";
  import type { VoiceStatus } from "./speech";

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
    /**
     * Whether a practice stage is up, told to `App.svelte` so it can take the
     * course switch and the tab row out of the way while one is.
     *
     * The maintainer's instruction, and the reason is the screen: with the switch
     * and the row above it the board had about 150px less than the phone could
     * give it. A stage is then the whole screen and **`Lessons` is the way out**
     * — which is why that button is the first thing on it.
     */
    practice?: boolean;
  }

  let { pick = $bindable(null), onradical, voice, practice = $bindable(false) }: Props = $props();

  let lessons = $state<KanjiLessonView[]>([]);
  let grade = $state(1);
  let error = $state<string | null>(null);

  /**
   * The lesson being practised, and what to open on — or null while the course
   * itself is showing.
   *
   * One value rather than two: which lesson and which of its characters are the
   * same decision, and a stage that could be up with no lesson would be a board
   * with no arrows.
   */
  let practising = $state<{ lesson: KanjiLessonView | null; start: KanjiPick } | null>(null);

  const tabs = $derived(gradeTabs(lessons));
  const inGrade = $derived(lessonsIn(lessons, grade));

  /**
   * Keep the app told whether a stage is up.
   *
   * One place rather than three: `practising` is set by a card, by a deep link and
   * by leaving, and a flag written at each of those would be wrong the day a
   * fourth caller arrives. The effect reads `practising` and writes the prop,
   * which the app never writes back — so there is nothing here to loop.
   */
  $effect(() => {
    practice = practising !== null;
  });

  async function loadLessons() {
    try {
      lessons = await api.kanjiLessons();
      const first = lessons[0];
      if (first) grade = first.grade;
    } catch (e) {
      error = String(e);
    }
  }
  void loadLessons();

  /** Open a lesson on its first character, which is the one the course teaches first. */
  function openLesson(lesson: KanjiLessonView) {
    const first = lesson.kanji[0];
    if (!first) return;
    practising = { lesson, start: { kind: "kanji", ch: first } };
  }

  function chooseGrade(next: number) {
    grade = next;
  }

  /**
   * What another screen asked for, opened once.
   *
   * It waits for the course before consuming the request: a character asked for
   * from a component has to land in the lesson that holds it — the arrows either
   * side of the board are that lesson's — and the lessons are what say where it
   * is. A radical head form belongs to no lesson, and opens with no arrows.
   */
  $effect(() => {
    const wanted = pick;
    if (!wanted) return;
    if (lessons.length === 0) return;
    pick = null;
    if (wanted.kind === "kanji") {
      const lesson = lessonHolding(lessons, wanted.ch) ?? null;
      if (lesson) grade = lesson.grade;
      practising = { lesson, start: { kind: "kanji", ch: wanted.ch } };
    } else {
      practising = { lesson: null, start: { kind: "radical", number: wanted.number } };
    }
  });
</script>

{#if practising}
  <!--
    Keyed on what it was opened on, so a second request from another screen — a
    component tapped inside the stage, say — mounts a fresh stage rather than
    merging the new character into the state of the old one.
  -->
  {#key practising.start.kind === "kanji" ? practising.start.ch : `r${practising.start.number}`}
    <KanjiPractice
      start={practising.start}
      lesson={practising.lesson}
      {voice}
      {onradical}
      onleave={() => (practising = null)}
    />
  {/key}
{:else}
  <section class="course">
    <header>
      <h2>Kanji</h2>
      {#if tabs.length > 0}
        <p class="sub">
          {tabs.reduce((sum, tab) => sum + tab.kanji, 0).toLocaleString()} jōyō characters ·
          {tabs.filter((t) => t.grade <= 6).reduce((sum, tab) => sum + tab.kanji, 0).toLocaleString()}
          kyōiku · {tabs.reduce((sum, tab) => sum + tab.lessons, 0)} lessons
        </p>
      {/if}
      <p class="hint">
        The grades are KANJIDIC2's current assignment and the order is the one schools
        teach in; inside a grade the most frequent characters come first. Choose a lesson
        to open its characters on the board.
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

    <ul class="lessons" aria-label="Lessons">
      {#each inGrade as lesson (lesson.key)}
        <li>
          <button
            class="lesson"
            onclick={() => openLesson(lesson)}
            title="Open {lesson.title} on the board"
          >
            <span class="title">{lesson.title}</span>
            <span class="cluster" lang="ja" aria-hidden="true">
              {#each lesson.kanji as ch (ch)}<span class="glyph">{ch}</span>{/each}
            </span>
            <span class="count">{lesson.count} characters</span>
          </button>
        </li>
      {/each}
    </ul>

    {#if inGrade.length === 0}
      <p class="hint">This grade holds no lessons.</p>
    {/if}
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

  /* The lesson cards. A grade's lessons are a list to choose from, not a column
     beside a board — the board is a screen of its own now — so they fill the
     width, two or three to a row where there is room. */
  .lessons {
    list-style: none;
    margin: 14px 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 8px;
  }

  .lesson {
    width: 100%;
    display: flex;
    flex-direction: column;
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

  .lesson .title {
    font-size: 1rem;
    font-variant-numeric: tabular-nums;
  }

  /* The ten characters of the lesson, which is what the choice is between: the
     title names a range and says nothing about whether 日 is inside it. */
  .cluster {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    font-size: 1.3rem;
    line-height: 1.2;
  }

  .glyph {
    min-width: 1.2em;
    text-align: center;
  }

  .lesson .count {
    font-size: 0.74rem;
    color: var(--muted);
  }
</style>
