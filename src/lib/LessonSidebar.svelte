<script lang="ts">
  /**
   * Navigation for the two screens.
   *
   * Course: the built-in frequency-ordered lessons, one lesson open at a time.
   * Vocabulary: the user's own groups, acting as the filter for the list panel.
   */
  import type { Lesson, VocabEntry } from "./types";

  interface Props {
    view: "course" | "vocabulary";
    onSwitchView: (view: "course" | "vocabulary") => void;
    lessons: Lesson[];
    activeLesson: number;
    activeCharacter: string | null;
    summary: string;
    onSelectLesson: (index: number) => void;
    onSelectCharacter: (ch: string) => void;
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
    <ol>
      {#each lessons as lesson (lesson.index)}
        <li class:active={lesson.index === activeLesson}>
          <button class="lesson" onclick={() => onSelectLesson(lesson.index)}>
            <span class="title">{lesson.title}</span>
            <span class="count">{lesson.characters.length}</span>
          </button>
          {#if lesson.index === activeLesson}
            <div class="chars">
              {#each lesson.characters as ch (ch)}
                <button
                  class="char"
                  class:current={ch === activeCharacter}
                  onclick={() => onSelectCharacter(ch)}
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

  .chars {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
    padding: 6px 4px 10px;
  }
  .char {
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

  .footnote {
    margin: 0;
    padding: 10px 16px 14px;
    border-top: 1px solid var(--line);
    font-size: 0.72rem;
    line-height: 1.45;
    color: var(--muted);
  }
</style>
