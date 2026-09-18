<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "./lib/api";
  import FeedbackPanel from "./lib/FeedbackPanel.svelte";
  import LessonSidebar from "./lib/LessonSidebar.svelte";
  import PracticeCanvas from "./lib/PracticeCanvas.svelte";
  import type { Character, DatasetStats, GradeReport, Lesson, Point } from "./lib/types";

  type Mode = "trace" | "recall";

  const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

  let stats = $state<DatasetStats | null>(null);
  let lessons = $state<Lesson[]>([]);
  let character = $state<Character | null>(null);
  let mode = $state<Mode>("trace");
  let strokes = $state<Point[][]>([]);
  let report = $state<GradeReport | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let grading = $state(false);
  let showCorrections = $state(true);
  /**
   * The TTS voice, once known: a name when pronunciation is available, `null`
   * when the system has no Chinese voice, `undefined` while still resolving.
   */
  let voice = $state<string | null | undefined>(undefined);

  /** Position in the flattened course; lessons are just a view onto it. */
  let index = $state(0);
  /** How much of the reference character is on the board. */
  let revealed = $state(0);
  /** True while the stroke-order animation runs. */
  let playing = $state(false);
  let playToken = 0;

  const allCharacters = $derived(lessons.flatMap((lesson) => lesson.characters));
  const currentChar = $derived(allCharacters[index] ?? null);
  const strokeTotal = $derived(character?.outlines.length ?? 0);

  const lessonStarts = $derived.by(() => {
    const starts: number[] = [];
    let running = 0;
    for (const lesson of lessons) {
      starts.push(running);
      running += lesson.characters.length;
    }
    return starts;
  });

  const activeLesson = $derived.by(() => {
    let found = 0;
    for (let i = 0; i < lessonStarts.length; i++) {
      if (lessonStarts[i] <= index) found = i;
      else break;
    }
    return found;
  });

  /**
   * In trace mode the whole character sits on the board as a guide. In recall
   * mode nothing is shown until the learner asks for the answer, which is the
   * whole point of the mode.
   */
  const ghostCount = $derived.by(() => {
    if (playing) return revealed;
    if (mode === "trace") return strokeTotal;
    return revealed;
  });
  const ghostStyle = $derived(mode === "trace" && !playing ? "faint" : "highlight");
  const answerVisible = $derived(mode === "trace" || revealed > 0);

  const summary = $derived(
    stats
      ? `${stats.teachable.toLocaleString()} characters in ${stats.lessons.toLocaleString()} lessons`
      : "loading…",
  );

  onMount(() => {
    void (async () => {
      try {
        const [loadedStats, loadedLessons] = await Promise.all([
          api.datasetStats(),
          api.listLessons(),
        ]);
        stats = loadedStats;
        lessons = loadedLessons;
        void api.log(
          `course loaded: ${loadedStats.teachable} characters in ${loadedStats.lessons} lessons`,
        );
      } catch (cause) {
        error = `Could not load the course: ${cause}`;
      } finally {
        loading = false;
      }
    })();

    // Resolving the voice runs the system voice list, which takes about a
    // second, so it is deliberately not part of the course load above.
    void api
      .speechStatus()
      .then((status) => {
        voice = status;
        void api.log(
          status ? `speech: using ${status}` : "speech: no Chinese voice installed",
        );
      })
      .catch(() => {
        voice = null;
      });

    const onKey = (event: KeyboardEvent) => {
      const meta = event.metaKey || event.ctrlKey;
      if (event.key === "Enter") {
        event.preventDefault();
        if (report) move(1);
        else void check();
      } else if (event.key === "Backspace" || (meta && event.key.toLowerCase() === "z")) {
        event.preventDefault();
        undo();
      } else if (event.key === "ArrowRight") {
        move(1);
      } else if (event.key === "ArrowLeft") {
        move(-1);
      } else if (event.key.toLowerCase() === "s") {
        void playStrokeOrder();
      } else if (event.key.toLowerCase() === "h") {
        void hear();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // Follow the cursor: load whichever character it now points at.
  $effect(() => {
    const next = currentChar;
    if (!next || character?.ch === next) return;
    void loadCharacter(next);
  });

  async function loadCharacter(ch: string) {
    try {
      const next = await api.getCharacter(ch);
      character = next;
      reset();
      void api.log(
        `character ${next.ch}: ${next.pinyin.join("/") || "?"}, ${next.outlines.length} strokes`,
      );
    } catch (cause) {
      error = `Could not load ${ch}: ${cause}`;
    }
  }

  /** Return the board to a blank state, invalidating any grade. */
  function reset() {
    strokes = [];
    report = null;
    revealed = 0;
    playing = false;
    playToken += 1;
    // Don't let the previous character keep talking over the next one.
    void api.stopSpeaking();
  }

  /**
   * Pronounce the current character.
   *
   * The character itself is spoken rather than its pinyin: the system voice has
   * a Chinese lexicon, so 汉 is read correctly, whereas `hàn` would be guessed
   * at. This is safe to offer in recall mode — hearing the sound does not give
   * away how to write the glyph.
   */
  async function hear() {
    if (!character || voice === null) return;
    const spoken = character.ch;
    try {
      await api.speak(spoken);
      void api.log(`spoke ${spoken}`);
    } catch (cause) {
      error = `Could not pronounce ${spoken}: ${cause}`;
    }
  }

  function addStroke(stroke: Point[]) {
    strokes = [...strokes, stroke];
    // The old verdict no longer describes what is on the board.
    report = null;
  }

  function undo() {
    if (strokes.length === 0) return;
    strokes = strokes.slice(0, -1);
    report = null;
  }

  function switchMode(next: Mode) {
    if (next === mode) return;
    mode = next;
    reset();
  }

  async function check() {
    if (!character || strokes.length === 0 || grading) return;
    grading = true;
    error = null;
    try {
      const graded = await api.gradeAttempt(character.ch, strokes, {
        resampleK: 16,
        minStrokeLen: 12,
        // Tracing a visible guide is graded where the guide actually is;
        // writing from memory should not be punished for sitting slightly off.
        globalFit: mode === "recall",
      });
      report = graded;
      void api.log(
        `graded ${character.ch}: ${Math.round(graded.overall)}/100, ` +
          `legible=${graded.legible}, order=${graded.orderCorrect}`,
      );
    } catch (cause) {
      error = `Grading failed: ${cause}`;
    } finally {
      grading = false;
    }
  }

  async function playStrokeOrder() {
    const total = strokeTotal;
    if (total === 0 || playing) return;
    const token = ++playToken;
    playing = true;
    revealed = 0;
    // Quick enough for a 20-stroke character, slow enough to follow.
    const step = Math.max(170, 900 - total * 30);
    for (let i = 1; i <= total; i++) {
      if (token !== playToken) return;
      revealed = i;
      await sleep(step);
    }
    if (token !== playToken) return;
    playing = false;
  }

  function move(delta: number) {
    const total = allCharacters.length;
    if (total === 0) return;
    index = Math.min(total - 1, Math.max(0, index + delta));
  }

  function goToLesson(lessonIndex: number) {
    const start = lessonStarts[lessonIndex];
    if (start !== undefined) index = start;
  }

  function goToCharacter(ch: string) {
    const found = allCharacters.indexOf(ch);
    if (found >= 0) index = found;
  }
</script>

<div class="app">
  <LessonSidebar
    {lessons}
    {activeLesson}
    activeCharacter={currentChar}
    {summary}
    onSelectLesson={goToLesson}
    onSelectCharacter={goToCharacter}
  />

  <main>
    {#if loading}
      <p class="status">Loading the character set…</p>
    {:else if !character}
      <p class="status">No character selected.</p>
    {:else}
      <header class="meta">
        <div class="glyph" lang="zh-Hans" class:masked={!answerVisible}>
          {answerVisible ? character.ch : "?"}
        </div>
        <div class="detail">
          <p class="pinyin">{character.pinyin.join("  ·  ") || "—"}</p>
          <p class="meaning">{character.definition || "—"}</p>
          <ul class="facts">
            <li>{strokeTotal} {strokeTotal === 1 ? "stroke" : "strokes"}</li>
            {#if character.radical && character.radical !== "\u0000"}
              <li>radical <span lang="zh-Hans">{character.radical}</span></li>
            {/if}
            {#if character.hsk > 0}<li>HSK {character.hsk}</li>{/if}
            {#if character.rank > 0}<li>frequency #{character.rank}</li>{/if}
          </ul>
          {#if answerVisible && character.etymology}
            <p class="etymology">{character.etymology}</p>
          {/if}
        </div>
        <div class="nav">
          <button onclick={() => move(-1)} disabled={index === 0}>←</button>
          <span>{index + 1} / {allCharacters.length}</span>
          <button onclick={() => move(1)} disabled={index >= allCharacters.length - 1}>→</button>
        </div>
      </header>

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <div class="workspace">
        <section class="stage">
          <PracticeCanvas
            {character}
            {strokes}
            {report}
            {ghostCount}
            {ghostStyle}
            {showCorrections}
            onStroke={addStroke}
          />

          <div class="controls">
            <div class="segmented" role="group" aria-label="Practice mode">
              <button class:on={mode === "trace"} onclick={() => switchMode("trace")}>
                Trace
              </button>
              <button class:on={mode === "recall"} onclick={() => switchMode("recall")}>
                Recall
              </button>
            </div>

            <button
              class="speak"
              onclick={hear}
              disabled={!character || voice === null}
              title={voice === undefined
                ? "Looking for a Chinese voice…"
                : voice === null
                  ? "No Chinese voice is installed, so pronunciation is unavailable"
                  : `Pronounce this character (${voice})`}
            >
              <span aria-hidden="true">🔊</span> Hear it
            </button>

            <button onclick={playStrokeOrder} disabled={playing || strokeTotal === 0}>
              {playing ? "Playing…" : "Show stroke order"}
            </button>
            <button onclick={undo} disabled={strokes.length === 0}>Undo</button>
            <button onclick={reset} disabled={strokes.length === 0}>Clear</button>

            <span class="spacer"></span>

            <label class="toggle">
              <input type="checkbox" bind:checked={showCorrections} />
              corrections
            </label>
            <span class="count">{strokes.length} / {strokeTotal}</span>
            <button class="primary" onclick={check} disabled={strokes.length === 0 || grading}>
              {grading ? "Checking…" : "Check"}
            </button>
          </div>

          <p class="hint">
            {#if voice === null}
              No Chinese voice is installed, so the pronunciation button is
              disabled. Add one in System Settings → Accessibility → Spoken
              Content → System Voice → Manage Voices.
            {:else if mode === "trace"}
              A faint copy of the character is on the board: trace over it in the
              correct stroke order.
            {:else}
              Write the character from memory, then press Check. Press S to see the
              stroke order.
            {/if}
          </p>
        </section>

        <aside class="feedback">
          {#if report}
            <FeedbackPanel {report} />
          {:else}
            <div class="empty">
              <h2>How this works</h2>
              <p>
                The reference strokes are compared with yours in two independent ways:
                whether each stroke is the right shape and in the right place, and
                whether they were written in the right order. Writing a legible
                character in the wrong order is reported as exactly that.
              </p>
              <p class="keys">
                <kbd>Enter</kbd> check · <kbd>S</kbd> stroke order ·
                <kbd>H</kbd> hear it · <kbd>⌫</kbd> undo ·
                <kbd>←</kbd> <kbd>→</kbd> move
              </p>
            </div>
          {/if}
        </aside>
      </div>
    {/if}
  </main>
</div>

<style>
  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 16px 20px 20px;
    gap: 12px;
    overflow-y: auto;
  }

  .status {
    margin: auto;
    color: var(--muted);
  }

  .meta {
    display: flex;
    align-items: flex-start;
    gap: 18px;
  }
  .glyph {
    flex: none;
    width: 92px;
    height: 92px;
    display: grid;
    place-items: center;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
    font-family: var(--hanzi-font);
    font-size: 3.4rem;
    line-height: 1;
    color: var(--muted-strong);
  }
  .glyph.masked {
    color: var(--line);
    font-size: 2.6rem;
  }

  .detail {
    flex: 1;
    min-width: 0;
  }
  .pinyin {
    margin: 2px 0 2px;
    font-size: 1.4rem;
    font-weight: 600;
    color: var(--accent-ink);
  }
  .meaning {
    margin: 0 0 6px;
    font-size: 0.92rem;
    color: var(--muted-strong);
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 0.76rem;
    color: var(--muted);
  }
  .etymology {
    margin: 6px 0 0;
    font-size: 0.82rem;
    font-style: italic;
    color: var(--muted);
  }

  .nav {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.78rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .nav button {
    width: 30px;
    height: 30px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    cursor: pointer;
    font-size: 0.9rem;
    color: var(--muted-strong);
  }
  .nav button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .workspace {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 330px;
    gap: 20px;
    /* Let the stage fill the row so the board has a real height to fit into. */
    align-items: stretch;
  }

  .stage {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
    min-height: 0;
  }

  .controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .controls button {
    padding: 7px 13px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    font: inherit;
    font-size: 0.84rem;
    color: var(--muted-strong);
    cursor: pointer;
  }
  .controls button:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .controls button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .controls button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
    padding-inline: 20px;
  }
  .controls button.primary:hover:not(:disabled) {
    background: var(--accent-ink);
    color: #fff;
  }
  .controls button.speak span[aria-hidden] {
    margin-right: 3px;
  }
  .spacer {
    flex: 1;
  }
  .count {
    font-size: 0.8rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 0.78rem;
    color: var(--muted);
    cursor: pointer;
  }

  .segmented {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
  }
  .segmented button {
    border: 0;
    border-radius: 0;
    padding: 7px 15px;
  }
  .segmented button.on {
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  .hint {
    margin: 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .feedback {
    position: sticky;
    top: 0;
  }

  .empty {
    padding: 16px;
    border: 1px dashed var(--line);
    border-radius: 12px;
    background: var(--surface);
  }
  .empty h2 {
    margin: 0 0 8px;
    font-size: 0.9rem;
    font-weight: 650;
  }
  .empty p {
    margin: 0 0 10px;
    font-size: 0.84rem;
    line-height: 1.5;
    color: var(--muted-strong);
  }
  .keys {
    color: var(--muted) !important;
    font-size: 0.76rem !important;
    margin-bottom: 0 !important;
  }
  kbd {
    padding: 1px 5px;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: 5px;
    background: var(--bg);
    font-family: inherit;
    font-size: 0.72rem;
  }

  .error {
    margin: 0;
    padding: 9px 12px;
    border: 1px solid #fca5a5;
    border-radius: 9px;
    background: #fef2f2;
    color: #991b1b;
    font-size: 0.84rem;
  }
</style>
