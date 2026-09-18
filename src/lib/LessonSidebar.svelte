<script lang="ts">
  /** Lesson navigation: the course in frequency order, one lesson open at a time. */
  import type { Lesson } from "./types";

  interface Props {
    lessons: Lesson[];
    activeLesson: number;
    activeCharacter: string | null;
    summary: string;
    onSelectLesson: (index: number) => void;
    onSelectCharacter: (ch: string) => void;
  }

  let {
    lessons,
    activeLesson,
    activeCharacter,
    summary,
    onSelectLesson,
    onSelectCharacter,
  }: Props = $props();
</script>

<nav class="sidebar">
  <header>
    <h1>Hanzi Tutor</h1>
    <p>{summary}</p>
  </header>

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
    padding: 16px 16px 12px;
    border-bottom: 1px solid var(--line);
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

  ol {
    margin: 0;
    padding: 6px;
    list-style: none;
    overflow-y: auto;
    min-height: 0;
    flex: 1;
  }

  .lesson {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
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
  .lesson:hover {
    background: var(--hover);
  }
  li.active > .lesson {
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
</style>
