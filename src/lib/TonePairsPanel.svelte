<script lang="ts">
  /**
   * Tone pairs: the characters that differ only in tone.
   *
   * This is where most learners actually struggle, and the app could not practise
   * it. It could *score* a tone — `TonePanel.svelte` draws the learner's contour
   * over the expected shape — but a contour is only useful next to another one, and
   * nothing told a learner which characters form a minimal pair to compare. That
   * grouping is derived in Rust (`hanzi_core::Dataset::tone_sets`): one syllable,
   * one character per tone, 妈 mā / 麻 má / 马 mǎ / 骂 mà.
   *
   * ## Two halves, and they are different exercises
   *
   * - **Hearing** is turned into a **quiz**: the app says one of the set and the
   *   learner picks which reading it was. The readings are shown, deliberately —
   *   the exercise is mapping a sound to a tone, not recalling which character is
   *   which tone by sight, which would test character knowledge instead.
   * - **Saying** is handed to the board (`Practise`), where push-to-talk and the
   *   contour panel already live. Nothing about the microphone is duplicated here.
   *
   * The audio is the **system voice**, the same one the board's Listen button uses
   * — the bundled recordings are whole phrases from the graded corpora and there is
   * no clip per syllable. So a device with no Chinese voice disables the two
   * listening halves, with the reason, rather than pretending.
   */
  import { onMount } from "svelte";
  import * as api from "./api";
  import Icon from "./Icon.svelte";
  import type { ToneSet, ToneSetMember } from "./types";

  interface Props {
    /** The whole derived list. Injected so the panel owns no IPC of its own. */
    load: () => Promise<ToneSet[]>;
    /**
     * The voice pronunciation will use: a name, `null` when the system has no
     * Chinese voice, `undefined` while that is still being looked up. The same
     * tri-state the board's Listen button uses.
     */
    voice: string | null | undefined;
    /** Write these characters on the board, in this order. */
    onPractise: (members: ToneSetMember[]) => void;
  }

  let { load, voice, onPractise }: Props = $props();

  let sets = $state<ToneSet[]>([]);
  let loading = $state(true);
  let failure = $state<string | null>(null);
  /** Set when pronouncing failed, which is a different problem from loading. */
  let speechFailure = $state<string | null>(null);

  /**
   * Which sets are worth showing. The pairs are the classic confusions — a rising
   * tone against a dipping one, a level tone against a falling one — and a learner
   * working on one of them does not want to scroll past the other 300 sets.
   */
  type Filter = "all" | "2-3" | "1-4" | "four";
  const FILTERS: { id: Filter; label: string; note: string }[] = [
    { id: "all", label: "All", note: "Every set, most useful first" },
    {
      id: "2-3",
      label: "2 · 3",
      note: "Rising against dipping — the pair beginners confuse most",
    },
    { id: "1-4", label: "1 · 4", note: "Level against falling" },
    { id: "four", label: "Four tones", note: "One syllable at all four tones" },
  ];
  let filter = $state<Filter>("all");
  let query = $state("");

  /** Fold a query for comparison: tone marks gone, `v` and `u` the same letter. */
  const fold = (text: string) =>
    text
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/v/g, "u");

  const has = (set: ToneSet, tone: number) => set.members.some((m) => m.tone === tone);

  const shown = $derived.by(() => {
    const text = query.trim();
    return sets.filter((set) => {
      if (filter === "2-3" && !(has(set, 2) && has(set, 3))) return false;
      if (filter === "1-4" && !(has(set, 1) && has(set, 4))) return false;
      if (filter === "four" && set.members.length !== 4) return false;
      if (text === "") return true;
      return (
        fold(set.base).includes(fold(text)) ||
        set.members.some((m) => m.ch === text || fold(m.reading).includes(fold(text)))
      );
    });
  });

  /**
   * The set being quizzed, and what was heard and answered.
   *
   * One at a time, and `base` is what ties the state to its row: two sets can hold
   * the same character, so the character alone would not say which row is live.
   */
  let quiz = $state<{ base: string; answer: string; picked: string | null } | null>(null);

  onMount(() => {
    void (async () => {
      try {
        sets = await load();
        failure = null;
      } catch (cause) {
        failure = `Could not work out the tone pairs: ${cause}`;
      } finally {
        loading = false;
      }
    })();
  });

  /**
   * Which utterance is current.
   *
   * A plain variable, not state: it stops a "hear the set" sequence that is still
   * speaking from carrying on after the learner has asked for something else, and
   * nothing renders it.
   */
  let sayToken = 0;

  /** Say one character, stopping whatever was being said first. */
  async function say(ch: string) {
    const token = ++sayToken;
    try {
      await api.stopSpeaking();
      if (token !== sayToken) return;
      await api.speak(ch);
      speechFailure = null;
    } catch (cause) {
      speechFailure = `Could not pronounce ${ch}: ${cause}`;
    }
  }

  /**
   * Say every member of a set, in tone order, with a gap between them.
   *
   * The gap is timed rather than measured: nothing tells the interface when the
   * synthesiser has finished, and hearing 妈麻马骂 as one run would be worse than
   * hearing them a beat apart.
   */
  async function hearSet(set: ToneSet) {
    const token = ++sayToken;
    try {
      await api.stopSpeaking();
      for (const member of set.members) {
        if (token !== sayToken) return;
        await api.speak(member.ch);
        await new Promise((resolve) => setTimeout(resolve, 900));
      }
      if (token === sayToken) speechFailure = null;
    } catch (cause) {
      if (token === sayToken) speechFailure = `Could not pronounce the set: ${cause}`;
    }
  }

  /** Ask a member at random, speaking it. Never twice in a row for one set. */
  function ask(set: ToneSet) {
    const last = quiz?.base === set.base ? quiz.answer : null;
    let index = Math.floor(Math.random() * set.members.length);
    if (set.members.length > 1 && last !== null) {
      let guard = 0;
      while (set.members[index].ch === last && guard < 8) {
        index = Math.floor(Math.random() * set.members.length);
        guard += 1;
      }
    }
    const member = set.members[index];
    quiz = { base: set.base, answer: member.ch, picked: null };
    void say(member.ch);
  }

  function answer(set: ToneSet, member: ToneSetMember) {
    if (quiz === null || quiz.base !== set.base || quiz.picked !== null) return;
    quiz = { ...quiz, picked: member.ch };
  }

  /** The character that was asked, for the feedback line and "hear it again". */
  const quizAnswer = $derived.by(() => {
    const live = quiz;
    if (live === null) return null;
    const set = sets.find((candidate) => candidate.base === live.base);
    return set?.members.find((member) => member.ch === live.answer) ?? null;
  });
  const quizPicked = $derived.by(() => {
    const live = quiz;
    if (live === null || live.picked === null) return null;
    const set = sets.find((candidate) => candidate.base === live.base);
    return set?.members.find((member) => member.ch === live.picked) ?? null;
  });

  /** True when the scorer cannot separate this set's two tones. */
  const flatTrap = (set: ToneSet) =>
    set.members.length === 2 && set.members[0].tone === 1 && set.members[1].tone === 3;

  /** `undefined` is "still looking"; anything but a name means no audio. */
  const noVoice = $derived(typeof voice !== "string");
  const voiceNote = $derived(
    voice === undefined
      ? "Looking for a Chinese voice…"
      : "No Chinese voice is installed, so nothing here can be heard or quizzed. The sets can still be written on the board.",
  );

  const summary = $derived.by(() => {
    if (loading && sets.length === 0) return "Working out the pairs…";
    if (sets.length === 0) return "No tone pairs";
    if (shown.length === sets.length && query.trim() === "") {
      return `${sets.length} sets: one syllable, one character per tone`;
    }
    return `showing ${shown.length} of ${sets.length} sets`;
  });
</script>

<section class="panel">
  <div class="toolbar">
    <form
      class="find"
      onsubmit={(event) => {
        event.preventDefault();
      }}
    >
      <input
        bind:value={query}
        placeholder="Syllable — ma, shi — or a character"
        aria-label="Find a tone set by syllable or character"
        autocomplete="off"
        spellcheck="false"
      />
      {#if query}
        <button type="button" onclick={() => (query = "")} title="Clear the search">Clear</button>
      {/if}
    </form>
    <div class="filters" role="group" aria-label="Which pairs">
      {#each FILTERS as option (option.id)}
        <button
          class:on={filter === option.id}
          onclick={() => (filter = option.id)}
          title={option.note}
        >
          {option.label}
        </button>
      {/each}
    </div>
  </div>

  <p class="summary">{summary}</p>

  {#if failure}
    <p class="warning">{failure}</p>
  {/if}
  {#if speechFailure}
    <p class="warning">{speechFailure}</p>
  {/if}
  {#if noVoice}
    <p class="note">{voiceNote}</p>
  {/if}

  {#if shown.length === 0 && !loading && !failure}
    <p class="empty">
      Nothing matches. Try a syllable without tone marks (<em>ma</em>), or clear the
      filter to see every set.
    </p>
  {:else}
    <ul class="sets">
      {#each shown as set (set.base)}
        {@const live = quiz?.base === set.base}
        <li class="set" class:live>
          <div class="head">
            <span class="base">{set.base}</span>
            <span class="count">
              {set.members.length} {set.members.length === 1 ? "tone" : "tones"}
            </span>
            <span class="actions">
              <button
                onclick={() => hearSet(set)}
                disabled={noVoice}
                title={noVoice ? voiceNote : "Hear every tone of this syllable, in order"}
              >
                Hear
              </button>
              {#if live}
                <button onclick={() => (quiz = null)} title="Stop quizzing this set">Stop</button>
              {:else}
                <button
                  onclick={() => ask(set)}
                  disabled={noVoice}
                  title={noVoice
                    ? voiceNote
                    : "Hear one at random and pick which reading it was"}
                >
                  Quiz
                </button>
              {/if}
              <button
                class="primary"
                onclick={() => onPractise(set.members)}
                title="Write these on the board, then hold the microphone and say each one"
              >
                Practise
              </button>
            </span>
          </div>

          {#if live && quiz && quizAnswer}
            <div class="quiz">
              <p class="prompt">
                {#if quiz.picked === null}
                  Which reading did you hear?
                {:else if quiz.picked === quiz.answer}
                  Yes — <strong>{quizAnswer.reading}</strong>, {quizAnswer.definition}.
                {:else}
                  That was <strong>{quizAnswer.reading}</strong>, not {quizPicked?.reading}.
                {/if}
              </p>
              <div class="answers">
                {#each set.members as member (member.ch)}
                  <button
                    class="answer"
                    class:right={quiz.picked !== null && member.ch === quiz.answer}
                    class:wrong={quiz.picked === member.ch && member.ch !== quiz.answer}
                    onclick={() => answer(set, member)}
                    disabled={quiz.picked !== null}
                  >
                    <span class="glyph" lang="zh-Hans">{member.ch}</span>
                    <span class="reading">{member.reading}</span>
                  </button>
                {/each}
              </div>
              <div class="quiz-actions">
                <button onclick={() => void say(quizAnswer.ch)} disabled={noVoice}>
                  Hear it again
                </button>
                {#if quiz.picked !== null}
                  <button onclick={() => ask(set)} disabled={noVoice}>Another</button>
                {/if}
                <button onclick={() => (quiz = null)}>Done</button>
              </div>
            </div>
          {:else}
            <div class="members">
              {#each set.members as member (member.ch)}
                <div class="member">
                  <button
                    class="say"
                    onclick={() => void say(member.ch)}
                    disabled={noVoice}
                    aria-label="Hear {member.reading}"
                    title={noVoice ? voiceNote : `Hear ${member.reading}`}
                  >
                    <Icon name="speaker" />
                  </button>
                  <span class="glyph" lang="zh-Hans">{member.ch}</span>
                  <span class="reading">{member.reading}</span>
                  <span class="meaning">{member.definition || "—"}</span>
                </div>
              {/each}
            </div>
            {#if flatTrap(set)}
              <p class="caveat">
                Tone 1 and a level tone 3 look the same to the scorer from one
                character, so the score cannot tell these two apart. Hearing them is
                the honest half here.
              </p>
            {/if}
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 220px;
  }
  .find input {
    flex: 1;
    padding: 8px 11px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg);
    font: inherit;
    font-size: 0.88rem;
    color: var(--muted-strong);
  }
  .find input:focus {
    outline: 2px solid var(--accent-soft);
    border-color: var(--accent);
  }

  /* Which pairs are worth showing: a filter, not a search, so it reads as one
     segmented control rather than four buttons. */
  .filters {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
  }
  .filters button {
    padding: 7px 11px;
    border: 0;
    border-radius: 0;
    background: transparent;
    font-size: 0.8rem;
  }
  .filters button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  button {
    padding: 7px 13px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    font: inherit;
    font-size: 0.84rem;
    color: var(--muted-strong);
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  button.primary:hover:not(:disabled) {
    background: var(--accent-ink);
    color: #fff;
  }

  .summary {
    margin: 0;
    font-size: 0.78rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .sets {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow-y: auto;
    min-height: 0;
  }
  .set {
    padding: 8px 10px;
    border: 1px solid transparent;
    border-radius: 10px;
    background: var(--surface);
  }
  .set:hover {
    background: var(--hover);
  }
  .set.live {
    border-color: var(--accent);
    background: var(--surface);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .base {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--accent-ink);
  }
  .count {
    flex: 1;
    min-width: 0;
    font-size: 0.72rem;
    color: var(--muted);
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .actions button {
    padding: 4px 10px;
    font-size: 0.76rem;
  }

  .members {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  .member {
    display: grid;
    grid-template-columns: auto auto;
    grid-template-rows: auto auto;
    align-items: center;
    gap: 0 8px;
    padding: 5px 9px 5px 5px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--bg);
  }
  .say {
    grid-row: 1 / span 2;
    padding: 4px 6px;
    border: 0;
    background: transparent;
    color: var(--muted);
    line-height: 0;
  }
  .say:hover:not(:disabled) {
    background: var(--accent-soft);
    border-color: transparent;
  }
  .glyph {
    font-family: var(--hanzi-font);
    font-size: 1.5rem;
    line-height: 1.15;
    color: var(--muted-strong);
  }
  .reading {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--accent-ink);
  }
  .member .meaning {
    grid-column: 2;
    font-size: 0.72rem;
    color: var(--muted);
  }

  /* The quiz replaces the member cards rather than sitting beside them: the cards
     already show the answer, and a button inside a button is neither valid nor
     clickable. */
  .quiz {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
    padding: 9px 10px;
    border: 1px dashed var(--line);
    border-radius: 9px;
  }
  .prompt {
    margin: 0;
    font-size: 0.8rem;
    color: var(--muted-strong);
  }
  .answers {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .answer {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    min-width: 66px;
    padding: 6px 10px;
  }
  .answer .glyph {
    font-size: 1.7rem;
    line-height: 1.1;
  }
  .answer.right {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .answer.wrong {
    border-color: #fca5a5;
    background: #fef2f2;
  }
  .answer.wrong .reading {
    color: #b91c1c;
  }
  .quiz-actions {
    display: flex;
    gap: 6px;
  }
  .quiz-actions button {
    padding: 4px 10px;
    font-size: 0.76rem;
  }

  .caveat,
  .empty {
    margin: 8px 0 0;
    font-size: 0.76rem;
    line-height: 1.45;
    color: var(--muted);
  }
  .empty {
    margin: 0;
    padding: 16px;
    border: 1px dashed var(--line);
    border-radius: 12px;
    background: var(--surface);
    font-size: 0.8rem;
  }

  .note {
    margin: 0;
    padding: 8px 11px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-size: 0.82rem;
  }
  .warning {
    margin: 0;
    padding: 9px 12px;
    border: 1px solid #fcd34d;
    border-radius: 9px;
    background: #fffbeb;
    color: #92400e;
    font-size: 0.8rem;
  }
</style>
