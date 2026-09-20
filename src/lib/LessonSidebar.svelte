<script lang="ts">
  /**
   * Navigation for the five screens.
   *
   * Course: the built-in frequency-ordered lessons, one lesson open at a time,
   * with how much of each has been practised and what is due for review.
   * Vocabulary: the user's own groups, acting as the filter for the list panel.
   * Words: the HSK dictionary, filtered by level.
   * Settings: the learner's own preferences.
   * About: what the app is, and the licence notices it ships under.
   */
  import type { Lesson, LevelCount, ProgressCard, ReviewView, VocabEntry } from "./types";

  interface Props {
    view: "course" | "vocabulary" | "words" | "settings" | "about";
    onSwitchView: (view: "course" | "vocabulary" | "words") => void;
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
    /** How many words sit at each HSK level, for the words screen. */
    wordLevels: LevelCount[];
    wordsTotal: number;
    /** The level the words screen is filtered to, or null for all of them. */
    wordLevel: number | null;
    onSelectWordLevel: (level: number | null) => void;
    /** Open the Settings screen. */
    onShowSettings: () => void;
    /** Open the About and licences screen. */
    onShowLicences: () => void;
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
    wordLevels,
    wordsTotal,
    wordLevel,
    onSelectWordLevel,
    onShowSettings,
    onShowLicences,
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

  /** The lesson list, so the active lesson can be brought into view. */
  let list = $state<HTMLOListElement | null>(null);

  /**
   * Keep the active lesson on screen.
   *
   * The course reopens where it was left, which can be hundreds of rows down a
   * 775-lesson list: restoring the position without showing it leaves the
   * sidebar apparently stuck at the beginning, with the reader's place and
   * anything due there hidden. Only this list's own scroll position is touched,
   * and only when the row is genuinely out of view, so scrolling by hand is
   * never fought.
   */
  $effect(() => {
    // Read both, so the effect runs when either the lesson or the character
    // within it changes.
    const character = activeCharacter;
    void activeLesson;
    const root = list;
    if (!root) return;
    // Wait for the character grid to lay out: the active row is only its full
    // height once the lesson is expanded.
    const frame = requestAnimationFrame(() => {
      const row =
        (character ? root.querySelector<HTMLElement>(".char.current") : null) ??
        root.querySelector<HTMLElement>("li.active > .lesson");
      if (!row) return;
      const box = row.getBoundingClientRect();
      const view = root.getBoundingClientRect();
      if (box.top < view.top) {
        root.scrollTop += box.top - view.top - 6;
      } else if (box.bottom > view.bottom) {
        root.scrollTop += box.bottom - view.bottom + 6;
      }
    });
    return () => cancelAnimationFrame(frame);
  });
</script>

<nav class="sidebar">
  <header>
    <h1>Hanzi Tutor</h1>
    <p>
      {#if view === "course"}
        {summary}
      {:else if view === "words"}
        {wordsTotal.toLocaleString()} HSK words
      {:else if view === "settings"}
        your preferences
      {:else if view === "about"}
        licences and attribution
      {:else}
        {total} entries · {practised} practised
      {/if}
    </p>
  </header>

  <div class="switch" role="group" aria-label="Screen">
    <button class:on={view === "course"} onclick={() => onSwitchView("course")}>
      Course
    </button>
    <button class:on={view === "vocabulary"} onclick={() => onSwitchView("vocabulary")}>
      My list
    </button>
    <button class:on={view === "words"} onclick={() => onSwitchView("words")}>
      HSK words
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
    <ol bind:this={list}>
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
  {:else if view === "words"}
    <ul class="groups">
      <li>
        <button class:current={wordLevel === null} onclick={() => onSelectWordLevel(null)}>
          <span class="title">All levels</span>
          <span class="count">{wordsTotal}</span>
        </button>
      </li>
      {#each wordLevels as entry (entry.level)}
        <li>
          <button
            class:current={wordLevel === entry.level}
            onclick={() => onSelectWordLevel(entry.level)}
          >
            <span class="title">HSK {entry.level}</span>
            <span class="count">{entry.words}</span>
          </button>
        </li>
      {/each}
    </ul>
    <p class="footnote">
      The official HSK 3.0 word lists. Practise a word without saving it, or use
      <em>+ List</em> to keep it with your own lesson material.
    </p>
  {:else if view === "vocabulary"}
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
      Add characters from the practice screen with the <em>+</em> beside the
      character's name, or add words directly in the panel.
    </p>
  {:else if view === "settings"}
    <div class="about">
      <p>
        How a stroke is drawn, how fast the stroke order is shown, how big the
        board is and which voice pronounces. Each change is written as you make
        it.
      </p>
    </div>
  {:else}
    <div class="about">
      <p>
        Hanzi Tutor runs entirely offline. The character data, the word
        dictionary, the interface font and every licence notice it depends on
        are inside this app.
      </p>
    </div>
  {/if}

  <div class="footer">
    <button class:on={view === "settings"} onclick={onShowSettings}>
      Settings
    </button>
    <button class:on={view === "about"} onclick={onShowLicences}>
      About and licences
    </button>
  </div>
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

  /* The About screen's sidebar body: it has no list, so it carries the space
     that keeps the footer pinned to the bottom. */
  .about {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 16px 14px;
  }
  .about p {
    margin: 0;
    font-size: 0.78rem;
    line-height: 1.5;
    color: var(--muted);
  }

  .footer {
    flex: none;
    padding: 6px;
    border-top: 1px solid var(--line);
  }
  .footer button {
    width: 100%;
    padding: 7px 10px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    font: inherit;
    font-size: 0.78rem;
    color: var(--muted);
    text-align: left;
    cursor: pointer;
  }
  .footer button:hover {
    background: var(--hover);
    color: var(--muted-strong);
  }
  .footer button.on {
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-weight: 600;
  }
</style>
