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
  import SpeakButton from "./lib/SpeakButton.svelte";
  import VocabularyPanel from "./lib/VocabularyPanel.svelte";
  import PassagePanel from "./lib/PassagePanel.svelte";
  import KanjiPanel from "./lib/KanjiPanel.svelte";
  import RadicalsPanel from "./lib/RadicalsPanel.svelte";
  import ReviewPanel from "./lib/ReviewPanel.svelte";
  import { VERDICT_COLOUR, VERDICT_LABEL } from "./lib/render";
  import { scheduleNote } from "./lib/review";
  import { canHear, isHearItKey, type VoiceStatus } from "./lib/speech";
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
    "practice" | "drill" | "review" | "kanji" | "radicals" | "words" | "read" | "licences"
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
  /**
   * The voice pronunciation will use, or `null` when the machine has none.
   *
   * Asked once, here, and handed to every screen that offers a "Hear it" button,
   * so one answer governs them all: a machine with no Japanese voice disables
   * every button and says why, rather than each panel discovering it separately.
   * `undefined` is the moment before the answer arrives, which is why the three
   * states are kept apart — see `lib/speech.ts`.
   */
  let voice = $state<VoiceStatus>(undefined);
  /**
   * What the review schedule did with the last attempt.
   *
   * Grading and scheduling are one command, so the verdict and the next due date
   * arrive together; this is the half the panel did not have before.
   */
  let note = $state<{ tone: "ok" | "plain" | "bad"; text: string } | null>(null);

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
    try {
      voice = await api.voice();
    } catch (e) {
      // The command answers `null` when the machine has no Japanese voice, so a
      // rejection is the IPC itself failing. Treated as "nothing can be heard"
      // — the buttons stay off — and said rather than swallowed.
      voice = null;
      error = String(e);
    }
  }
  void boot();

  /**
   * The "hear it" shortcut, live only on the screen that has a kana on the board.
   *
   * One view rather than the whole app deliberately: a window-level letter key
   * that fired on every screen would speak whatever happened to be selected on a
   * screen the learner is not looking at. The rule about which presses count is
   * `isHearItKey`, which is a pure function because it is about typing.
   */
  function onKey(event: KeyboardEvent) {
    if (!isHearItKey(event)) return;
    if (view !== "practice" || !selected || !canHear(voice)) return;
    event.preventDefault();
    void api.speak(selected).catch((e) => {
      error = String(e);
    });
  }

  /**
   * Choosing another character, or leaving the screen, ends whatever is being
   * said.
   *
   * A voice that keeps talking over the next thing the learner looks at is the
   * one way an audio feature becomes an annoyance, and speaking never changes
   * either of these — the button does that — so this effect only ever stops.
   */
  $effect(() => {
    view;
    selected;
    void api.stopSpeaking().catch(() => {
      // Nothing to report: this is cleanup, and the usual answer is that nothing
      // was being said.
    });
  });

  async function select(ch: string) {
    try {
      error = null;
      selected = ch;
      report = null;
      note = null;
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
    note = null;
  }

  async function grade() {
    if (!selected) return;
    try {
      busy = true;
      error = null;
      // The command grades *and* offers the attempt to the review schedule, so
      // what comes back is the verdict plus the schedule's answer to it.
      const graded = await api.gradeAttempt(selected, attempt);
      report = graded.report;
      note = scheduleNote(graded, new Date().toISOString());
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
    {#each [["practice", "Practice"], ["drill", "Tell them apart"], ["review", "Review"], ["kanji", "Kanji"], ["radicals", "Radicals"], ["words", "Words"], ["read", "Read"], ["licences", "Licences"]] as const as [id, label] (id)}
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
          <SpeakButton text={kana.ch} {voice} label="Hear it (H)" />
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

        {#if note}
          <p class="note" class:ok={note.tone === "ok"} class:bad={note.tone === "bad"}>
            {note.text}
          </p>
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
  {:else if view === "review"}
    <ReviewPanel {voice} />
  {:else if view === "kanji"}
    <KanjiPanel bind:pick={kanjiPick} onradical={seeRadical} />
  {:else if view === "radicals"}
    <RadicalsPanel bind:focus={selectedRadical} onopen={openKanji} onpractise={openRadical} />
  {:else if view === "words"}
    <VocabularyPanel {voice} />
  {:else if view === "read"}
    <PassagePanel {voice} />
  {:else}
    <LicencesPanel />
  {/if}

  <footer>
    {#if info}
      <span>{info.name} {info.version} · {info.licence}</span>
    {/if}
    <span>Everything the course teaches is in this bundle. Nothing is downloaded.</span>
    <span>
      {#if canHear(voice)}
        Pronunciation uses the system's own Japanese voice. Nothing is downloaded.
      {:else if voice === null}
        No Japanese voice is installed, so nothing here can be heard.
      {:else}
        Looking for a Japanese voice…
      {/if}
    </span>
  </footer>
</main>

<svelte:window onkeydown={onKey} />
