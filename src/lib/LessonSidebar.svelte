<script lang="ts">
  /**
   * Navigation for the two screens.
   *
   * Course: the built-in frequency-ordered lessons, one lesson open at a time,
   * with how much of each has been practised and what is due for review.
   * Vocabulary: the user's own groups, acting as the filter for the list panel.
   */
  import type { Lesson, ProgressCard, ReviewView, VocabEntry } from "./types";

  interface Props {
    view: "course" | "vocabulary";
    onSwitchView: (view: "course" | "vocabulary") => void;
    lessons: Lesson[];
    activeLesson: number;
    activeCharacter: string | null;
    summary: string;
    onSelectLesson: (index: number) => void;
    onSelectCharacter: (ch: string) => void;
    /** Every practised character, so a lesson can show how far it got. */
    progressCards: ProgressCard[];
    review: ReviewView;
    /** True while a review session is on the board. */
    reviewing: boolean;
    onStartReview: () => void;
    vocabGroups: string[];
    vocabEntries: VocabEntry[];
    /** `null` is everything, `""` the unfiled entries, otherwise a group. */
    vocabSelection: string | null;
    onSelectVocabGroup: (selection: string | null) => void;
  }

  let {
    view,
    onSwitchView,
    lessons,
    activeLesson,
    activeCharacter,
    summary,
    onSelectLesson,
    onSelectCharacter,
    progressCards,
    review,
    reviewing,
    onStartReview,
    vocabGroups,
    vocabEntries,
    vocabSelection,
    onSelectVocabGroup,
  }: Props = $props();

  const total = $derived(vocabEntries.length);
  const unfiled = $derived(vocabEntries.filter((entry) => entry.group === null).length);
  const practised = $derived(vocabEntries.filter((entry) => entry.attempts > 0).length);

  const countIn = (group: string) =>
    vocabEntries.filter((entry) => entry.group === group).length;

  /** Progress by character, for the marks and the per-lesson counts. */
  const cardFor = $derived(
    new Map(progressCards.map((card) => [card.ch, card])),
  );

  /** How much of a lesson has been attempted at least once. */
  const practisedIn = (lesson: Lesson) =>
    lesson.characters.filter((ch) => cardFor.has(ch)).length;

  /** How much of a lesson is due for review now. */
  const dueIn = (lesson: Lesson) =>
    lesson.characters.filter((ch) => cardFor.get(ch)?.dueNow).length;
</script>

<nav class="sidebar">
  <header>
    <h1>Hanzi Tutor</h1>
    <p>{view === "course" ? summary : `${total} entries · ${practised} practised`}</p>
  </header>

  <div class="switch" role="group" aria-label="Screen">
    <button class:on={view === "course"} onclick={() => onSwitchView("course")}>
      Course
    </button>
    <button class:on={view === "vocabulary"} onclick={() => onSwitchView("vocabulary")}>
      My vocabulary
    </button>
  </div>

  {#if view === "course"}
    <div class="review">
      <button
        class="reviewButton"
        class:ready={review.dueCount > 0}
        disabled={review.dueCount === 0}
        onclick={onStartReview}
        title={review.dueCount === 0
          ? "Nothing is due yet — practice in the course and characters come back here"
          : "Practise what is due, most overdue first"}
      >
        <span class="title">{reviewing ? "Reviewing…" : "Review due"}</span>
        <span class="count">{review.dueCount}</span>
      </button>
    </div>
    <ol>
      {#each lessons as lesson (lesson.index)}
        <li class:active={lesson.index === activeLesson}>
          <button class="lesson" onclick={() => onSelectLesson(lesson.index)}>
            <span class="title">{lesson.title}</span>
            <span class="count" class:done={practisedIn(lesson) === lesson.characters.length}>
              {#if dueIn(lesson) > 0}
                <span class="dot" title="{dueIn(lesson)} due for review"></span>
              {/if}
              {practisedIn(lesson)}/{lesson.characters.length}
            </span>
          </button>
          {#if lesson.index === activeLesson}
            <div class="chars">
              {#each lesson.characters as ch (ch)}
                <button
                  class="char"
                  class:current={ch === activeCharacter}
                  class:practised={cardFor.has(ch)}
                  class:due={cardFor.get(ch)?.dueNow ?? false}
                  onclick={() => onSelectCharacter(ch)}
                  title={cardFor.has(ch)
                    ? `practised ${cardFor.get(ch)?.attempts}×`
                    : "not practised yet"}
                  lang="zh-Hans">{ch}</button
                >
              {/each}
            </div>
          {/if}
        </li>
      {/each}
    </ol>
  {:else}
    <ul class="groups">
      <li>
        <button class:current={vocabSelection === null} onclick={() => onSelectVocabGroup(null)}>
          <span class="title">All entries</span>
          <span class="count">{total}</span>
        </button>
      </li>
      {#each vocabGroups as name (name)}
        <li>
          <button class:current={vocabSelection === name} onclick={() => onSelectVocabGroup(name)}>
            <span class="title">{name}</span>
            <span class="count">{countIn(name)}</span>
          </button>
        </li>
      {/each}
      <li>
        <button class:current={vocabSelection === ""} onclick={() => onSelectVocabGroup("")}>
          <span class="title">Unfiled</span>
          <span class="count">{unfiled}</span>
        </button>
      </li>
    </ul>
    <p class="footnote">
      Add characters from the practice screen with <em>Add to my list</em>, or add
      words directly in the panel.
    </p>
  {/if}
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    min-height: 0;
    width: 280px;
    flex: none;
    border-right: 1px solid var(--line);
    background: var(--surface);
  }

  header {
    padding: 16px 16px 10px;
  }
  h1 {
    margin: 0 0 4px;
    font-size: 1rem;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  header p {
    margin: 0;
    font-size: 0.75rem;
    color: var(--muted);
  }

  .switch {
    display: flex;
    margin: 0 10px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
  }
  .switch button {
    flex: 1;
    padding: 7px 6px;
    border: 0;
    background: transparent;
    font: inherit;
    font-size: 0.8rem;
    color: var(--muted-strong);
    cursor: pointer;
  }
  .switch button:hover {
    background: var(--hover);
  }
  .switch button.on {
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  ol,
  ul.groups {
    margin: 0;
    padding: 6px;
    list-style: none;
    overflow-y: auto;
    min-height: 0;
    flex: 1;
  }
  ul.groups button {
    width: 100%;
  }

  .lesson,
  ul.groups button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 10px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    font: inherit;
    font-size: 0.82rem;
    color: var(--muted-strong);
    text-align: left;
    cursor: pointer;
  }
  .lesson:hover,
  ul.groups button:hover {
    background: var(--hover);
  }
  li.active > .lesson,
  ul.groups button.current {
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-weight: 600;
  }
  .count {
    font-size: 0.7rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .count.done {
    color: #15803d;
    font-weight: 600;
  }
  /* A lesson with something due for review. */
  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    margin-right: 4px;
    border-radius: 50%;
    background: var(--accent);
    vertical-align: middle;
  }

  /* The review entry point, above the lesson list. */
  .review {
    padding: 0 6px 6px;
  }
  .reviewButton {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    border: 1px dashed var(--line);
    border-radius: 7px;
    background: transparent;
    font: inherit;
    font-size: 0.82rem;
    color: var(--muted-strong);
    text-align: left;
    cursor: pointer;
  }
  .reviewButton:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .reviewButton.ready {
    border-style: solid;
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-weight: 600;
  }
  .reviewButton.ready:hover {
    background: var(--accent);
    color: #fff;
  }

  .chars {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
    padding: 6px 4px 10px;
  }
  .char {
    position: relative;
    aspect-ratio: 1;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--bg);
    font-family: var(--hanzi-font);
    font-size: 1.25rem;
    line-height: 1;
    color: var(--muted-strong);
    cursor: pointer;
    padding: 0;
  }
  .char:hover {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .char.current {
    border-color: var(--accent);
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  /* Practised at least once: a quiet mark, not a badge. */
  .char.practised {
    border-color: #86efac;
    background: #f0fdf4;
  }
  .char.current.practised {
    border-color: var(--accent);
    background: var(--accent);
  }
  /* Due for review, whichever else is true of it. */
  .char.due::after {
    content: "";
    position: absolute;
    top: 3px;
    right: 3px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .char.current.due::after {
    background: #fff;
  }

  .footnote {
    margin: 0;
    padding: 10px 16px 14px;
    border-top: 1px solid var(--line);
    font-size: 0.72rem;
    line-height: 1.45;
    color: var(--muted);
  }
</style>
