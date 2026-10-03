<script lang="ts">
  /**
   * The kana tutor: a course down the side, the board in the middle, and the
   * verdict and the reading beside it.
   *
   * The course is the gojūon — plain rows first, then the voiced ones, then the
   * small, rare and katakana-only characters — and every kana on the board is
   * graded by the same engine that grades a Chinese character.
   */
  import KanaCanvas from "./lib/KanaCanvas.svelte";
  import ConfusionDrill from "./lib/ConfusionDrill.svelte";
  import LicencesPanel from "./lib/LicencesPanel.svelte";
  import VocabularyPanel from "./lib/VocabularyPanel.svelte";
  import PassagePanel from "./lib/PassagePanel.svelte";
  import KanjiPanel from "./lib/KanjiPanel.svelte";
  import RadicalsPanel from "./lib/RadicalsPanel.svelte";
  import { VERDICT_COLOUR, VERDICT_LABEL } from "./lib/render";
  import * as api from "./lib/api";
  import type {
    AppInfo,
    DatasetStats,
    GradeReport,
    Kana,
    KanjiPick,
    LessonView,
    ReadingCheck,
    ScriptName,
  } from "./lib/types";

  let info = $state<AppInfo | null>(null);
  let stats = $state<DatasetStats | null>(null);
  let view = $state<
    "practice" | "drill" | "kanji" | "radicals" | "words" | "read" | "licences"
  >("practice");
  let script = $state<ScriptName>("hiragana");
  let course = $state<LessonView[]>([]);
  let lessonKey = $state<string | null>(null);
  let selected = $state<string | null>(null);
  let kana = $state<Kana | null>(null);
  let report = $state<GradeReport | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /**
   * A character or radical another screen has asked the kanji board to open.
   *
   * The radicals panel and the kanji course's own component chips both end up
   * here: neither owns a board, and the kanji screen consumes this when it mounts
   * or when it changes. `kind` says which lookup to make, because 92 of the 214
   * head forms are not jōyō characters and cannot be fetched as one.
   */
  let kanjiPick = $state<KanjiPick | null>(null);

  /** The radical family the radicals panel should open, if any. */
  let selectedRadical = $state<number | null>(null);

  /** Open a jōyō character on the kanji board. */
  function openKanji(ch: string) {
    kanjiPick = { kind: "kanji", ch };
    view = "kanji";
  }

  /** Open one of the 214 radical head forms on the kanji board. */
  function openRadical(number: number) {
    kanjiPick = { kind: "radical", number };
    view = "kanji";
  }

  /** The same panel, from its other side: the family a character belongs to. */
  function seeRadical(number: number) {
    kanjiPick = null;
    selectedRadical = number;
    view = "radicals";
  }

  let typed = $state("");
  let check = $state<ReadingCheck | null>(null);

  let board = $state<ReturnType<typeof KanaCanvas> | null>(null);
  /**
   * The attempt as the canvas last reported it, for grading.
   *
   * Reactive because the Grade button's enabled state reads it: a plain `let`
   * would leave the button disabled after the first stroke.
   */
  let attempt = $state<{ x: number; y: number }[][]>([]);

  const activeLesson = $derived(course.find((l) => l.key === lessonKey) ?? null);

  $effect(() => {
    void loadCourse(script);
  });

  async function loadCourse(which: ScriptName) {
    try {
      course = await api.lessons(which);
      lessonKey = course[0]?.key ?? null;
      const first = course[0]?.kana[0] ?? null;
      if (first) await select(first);
    } catch (e) {
      error = String(e);
    }
  }

  async function boot() {
    try {
      [info, stats] = await Promise.all([api.appInfo(), api.datasetStats()]);
    } catch (e) {
      error = String(e);
    }
  }
  void boot();

  async function select(ch: string) {
    try {
      error = null;
      selected = ch;
      report = null;
      check = null;
      typed = "";
      attempt = [];
      kana = await api.kana(ch);
    } catch (e) {
      error = String(e);
    }
  }

  function onStrokes(strokes: { x: number; y: number }[][]) {
    attempt = strokes;
    // A verdict belongs to the attempt it judged; the moment the attempt changes
    // it is stale, and leaving it up would colour the new strokes with the old
    // reading.
    report = null;
  }

  async function grade() {
    if (!selected) return;
    try {
      busy = true;
      error = null;
      report = await api.gradeAttempt(selected, attempt);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function checkTyped() {
    if (!selected) return;
    try {
      check = await api.checkReading(selected, typed);
    } catch (e) {
      error = String(e);
    }
  }

  /** The verdict line, in words, for the report that is up. */
  const summary = $derived.by(() => {
    if (!report) return null;
    const wrong = report.strokes.filter(
      (s) => s.verdict !== "correct" && s.userIndex !== null,
    );
    return {
      total: report.strokes.length,
      wrong,
    };
  });
</script>

<main>
  <header>
    <h1>Kana Tutor</h1>
    {#if stats}
      <p class="sub">
        {stats.kana} kana · {stats.hiragana} hiragana · {stats.katakana} katakana ·
        {stats.lessons} lessons · {stats.kanji} kanji · {stats.radicals} radicals ·
        {stats.words.toLocaleString()} words
      </p>
    {/if}
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <div class="views" role="tablist">
    {#each [["practice", "Practice"], ["drill", "Tell them apart"], ["kanji", "Kanji"], ["radicals", "Radicals"], ["words", "Words"], ["read", "Read"], ["licences", "Licences"]] as const as [id, label] (id)}
      <button
        role="tab"
        aria-selected={view === id}
        class:active={view === id}
        onclick={() => (view = id)}>{label}</button
      >
    {/each}
  </div>

  {#if view === "practice"}
  <div class="tabs" role="tablist">
    {#each ["hiragana", "katakana"] as const as name (name)}
      <button
        role="tab"
        aria-selected={script === name}
        class:active={script === name}
        onclick={() => (script = name)}>{name}</button
      >
    {/each}
  </div>

  <div class="layout">
    <nav aria-label="Lessons">
      {#each course as lesson (lesson.key)}
        <button
          class="lesson"
          class:active={lesson.key === lessonKey}
          class:voiced={lesson.voiced}
          onclick={() => (lessonKey = lesson.key)}
        >
          <span class="kana">{lesson.title.split(" — ")[0]}</span>
          <span class="sound">{lesson.title.split(" — ")[1] ?? ""}</span>
        </button>
      {/each}
    </nav>

    <section class="picker" aria-label="Kana in this lesson">
      {#if activeLesson}
        <div class="grid">
          {#each activeLesson.kana as ch (ch)}
            <button class="cell" class:active={ch === selected} onclick={() => select(ch)}>
              {ch}
            </button>
          {/each}
        </div>
      {/if}

      <div class="yoon">
        <p class="hint">Yōon are two kana that make one mora — type them together.</p>
      </div>
    </section>

    <section class="practice">
      {#if kana}
        <div class="prompt">
          <span class="big">{kana.ch}</span>
          <span class="reading">
            {#if kana.silent}
              <em>no sound of its own</em>
            {:else}
              {kana.hepburn}{#if kana.romaji.length > 1}
                <small>(or {kana.romaji.slice(1).join(", ")})</small>{/if}
            {/if}
          </span>
          <span class="strokes">{kana.strokeCount}
            {kana.strokeCount === 1 ? "stroke" : "strokes"}</span>
        </div>

        <KanaCanvas bind:this={board} character={kana} {report} onchange={onStrokes} />

        <div class="actions">
          <button onclick={() => board?.animate()}>Show stroke order</button>
          <button onclick={() => board?.undo()}>Undo</button>
          <button onclick={() => board?.clear()}>Clear</button>
          <button class="primary" disabled={busy || attempt.length === 0} onclick={grade}>
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

        <div class="typing">
          <label for="typed">Type the reading</label>
          <div class="row">
            <input
              id="typed"
              bind:value={typed}
              onkeydown={(e) => e.key === "Enter" && checkTyped()}
              placeholder={kana.romaji[0] ?? ""}
              autocomplete="off"
              autocapitalize="off"
              spellcheck="false"
            />
            <button onclick={checkTyped}>Check</button>
          </div>
          {#if check}
            <p class="result" class:ok={check.correct}>
              {#if check.correct}
                Correct.
              {:else}
                That is {check.produced || "not a reading"} — try again.
              {/if}
            </p>
          {/if}
          <p class="hint">
            Both romanisations are accepted: Hepburn <code>shi tsu fu</code> and Kunrei
            <code>si tu hu</code> alike.
          </p>
        </div>

        {#if kana.confusions.length > 0}
          <div class="confusions">
            <h2>Not to be confused with</h2>
            <ul>
              {#each kana.confusions as c (c.ch)}
                <li>
                  <button class="cell small" onclick={() => select(c.ch)}>{c.ch}</button>
                  <span>{c.tell}</span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      {/if}
    </section>
  </div>

  {:else if view === "drill"}
    <ConfusionDrill />
  {:else if view === "kanji"}
    <KanjiPanel bind:pick={kanjiPick} onradical={seeRadical} />
  {:else if view === "radicals"}
    <RadicalsPanel bind:focus={selectedRadical} onopen={openKanji} onpractise={openRadical} />
  {:else if view === "words"}
    <VocabularyPanel />
  {:else if view === "read"}
    <PassagePanel />
  {:else}
    <LicencesPanel />
  {/if}

  <footer>
    {#if info}
      <span>{info.name} {info.version} · {info.licence}</span>
    {/if}
    <span>Everything the course teaches is in this bundle. Nothing is downloaded.</span>
  </footer>
</main>
