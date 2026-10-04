<script lang="ts">
  /**
   * Nihongo Tutor: two courses, one at a time.
   *
   * **Kana** is the on-ramp — a course down the side, the board in the middle, and
   * the verdict and the reading beside it — and **Kanji** is the course itself:
   * the character course, the radical table, the vocabulary and the passages.
   * Which of the two is open is remembered in the app's own file, and the tabs
   * under the switch are that course's own screens, from `lib/nav.ts`.
   *
   * Every character on the board, kana or kanji, is graded by the same engine that
   * grades a Chinese character.
   */
  import { untrack } from "svelte";
  import KanaCanvas from "./lib/KanaCanvas.svelte";
  import ConfusionDrill from "./lib/ConfusionDrill.svelte";
  import KanaChart from "./lib/KanaChart.svelte";
  import LicencesPanel from "./lib/LicencesPanel.svelte";
  import SpeakButton from "./lib/SpeakButton.svelte";
  import VocabularyPanel from "./lib/VocabularyPanel.svelte";
  import PassagePanel from "./lib/PassagePanel.svelte";
  import KanjiPanel from "./lib/KanjiPanel.svelte";
  import RadicalsPanel from "./lib/RadicalsPanel.svelte";
  import ReviewPanel from "./lib/ReviewPanel.svelte";
  import { focusFor, joinedLabel, lessonKeyOf } from "./lib/kana";
  import {
    COURSES,
    courseOf,
    covers,
    defaultView,
    isSection,
    tabsOf,
    type Section,
    type View,
  } from "./lib/nav";
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
  /**
   * The course that is open, and the screen of it.
   *
   * The two are kept apart on purpose: `review` is a screen of *both* courses and
   * the section says whose queue it is, so the section cannot be derived from the
   * view.
   */
  let section = $state<Section>("kana");
  let view = $state<View>("practice");
  let script = $state<ScriptName>("hiragana");
  let course = $state<LessonView[]>([]);
  let lessonKey = $state<string | null>(null);
  let selected = $state<string | null>(null);
  let kana = $state<Kana | null>(null);
  let report = $state<GradeReport | null>(null);
  /**
   * The taught strokes the last verdict read as drawn joined, 1-based — `[[1, 2]]`
   * for a さ written in two strokes. It travels with `report`, and is what makes
   * the per-stroke list below honest: its numbers are the *drawn* strokes.
   */
  let joined = $state<number[][]>([]);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /**
   * A kana another screen asked the practice board to open, waiting for the
   * course of the script it belongs to to load.
   *
   * Only ever set when the course for that script is not loaded yet: within a
   * script that is already loaded the kana opens at once. It carries the script
   * with it, so a slow load can tell whether the request is still the one being
   * waited for — a single slot without it would let a load that has been overtaken
   * apply the kana from the tap *before* the last one.
   */
  let wanted = $state<{ ch: string; script: ScriptName } | null>(null);

  /**
   * The script whose course is in `course` right now, or `null` while one loads.
   *
   * `script` is the toggle and `loadedScript` is what is actually on screen, and
   * they disagree for as long as a load takes — which is exactly when a tap has to
   * know better than to look the kana up in the wrong course.
   */
  let loadedScript = $state<ScriptName | null>(null);

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

  /**
   * A warning about the app rather than about an attempt: at present, that the
   * section could not be remembered. Kept beside `error` rather than in it, because
   * losing a preference is not the same as failing to grade.
   */
  let warning = $state<string | null>(null);

  /**
   * Open a screen, moving to another course first when it is not this one's.
   *
   * A screen belongs to a course, so a deep link is a promise about which course
   * the learner ends up in — `nav.ts` is the authority, and this is why the
   * section is passed explicitly rather than derived from the view: Review belongs
   * to both, and deriving it would move a kanji reviewer into the kana course.
   *
   * The promise is checked rather than assumed: drawing a kanji screen under the
   * kana tab row is the one way the division could come apart with no type error,
   * so a call that names a screen the course does not have is a programming error
   * and says so.
   */
  function goTo(next: Section, target: View) {
    if (!covers(next, target)) {
      throw new Error(`${target} is not a screen of ${next}`);
    }
    if (next !== section) {
      section = next;
      remember(next);
    }
    view = target;
  }

  /**
   * The course switch: choose a course and open on its first screen.
   *
   * Clicking the course already open is deliberately a no-op rather than a reset,
   * so the switch never throws away the screen the learner is working on.
   */
  function chooseSection(next: Section) {
    if (next === section) return;
    goTo(next, defaultView(next));
  }

  /**
   * Write the section down, so the next start returns here.
   *
   * Failure is reported and not retried: the learner did move, this session knows
   * where they are, and all that was lost is that the next start will open on the
   * on-ramp instead.
   */
  function remember(next: Section) {
    void api
      .setSection(next)
      .then((said) => {
        // The warning travels with the last write rather than accumulating, so a
        // switch that succeeds clears what an earlier one could not do.
        warning = said;
      })
      .catch((e) => (warning = String(e)));
  }

  /** Open a jōyō character on the kanji board. */
  function openKanji(ch: string) {
    kanjiPick = { kind: "kanji", ch };
    goTo("kanji", "kanji");
  }

  /** Open one of the 214 radical head forms on the kanji board. */
  function openRadical(number: number) {
    kanjiPick = { kind: "radical", number };
    goTo("kanji", "kanji");
  }

  /** The same panel, from its other side: the family a character belongs to. */
  function seeRadical(number: number) {
    kanjiPick = null;
    selectedRadical = number;
    goTo("kanji", "radicals");
  }

  /**
   * What one course holds, in numbers, for the switch that offers it.
   *
   * The counts are the reason the switch leads with the kanji course's size rather
   * than the kana's: 177 characters in 38 lessons is a few days, and 2,136
   * characters behind 16,073 words is the course itself.
   *
   * Characters and words, and not a count per screen: the tab row directly below
   * already names the radicals and the passages, and the line has to survive a
   * phone's width without being cut — which is the one thing that would turn two
   * lines into three.
   */
  function courseSize(id: Section): string {
    if (!stats) return "";
    return id === "kana"
      ? `${stats.kana} kana · ${stats.lessons} lessons`
      : `${stats.kanji.toLocaleString()} kanji · ${stats.words.toLocaleString()} words`;
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
      const lessons = await api.lessons(which);
      // A load that has been overtaken by a later toggle is dropped rather than
      // applied: the two responses can arrive in either order, and the course on
      // screen has to be the script the learner last chose.
      if (untrack(() => script) !== which) return;
      course = lessons;
      loadedScript = which;
      // A kana another screen asked for wins over the first of the course — but
      // only the request that belongs to *this* script. Cleared either way, so a
      // request whose script is not this one cannot be applied to the next load.
      // Read untracked deliberately: this effect must not depend on the request it
      // is consuming, or setting it would re-run the load that clears it.
      const asked = untrack(() => wanted);
      wanted = null;
      const focus = focusFor(asked, which, lessons);
      const inLesson = focus ? lessonKeyOf(lessons, focus) : null;
      lessonKey = inLesson ?? lessons[0]?.key ?? null;
      if (focus) await select(focus);
    } catch (e) {
      // The request goes with the course that failed to load, rather than staying
      // to be applied to whichever script loads next.
      wanted = null;
      loadedScript = null;
      error = String(e);
    }
  }

  /**
   * Open a kana on the practice board, from wherever it was offered.
   *
   * The chart, the lesson grid, the confusions list and the drill's own feedback
   * all end up here, and all of them are promises that tapping a kana writes it.
   * The script is not worked out from the character here: the app's own dataset is
   * the authority on which of り and リ is which and on ー being katakana, and it
   * already says so in the `Kana` it returns. A caller that knows the script —
   * the chart does — passes it and saves the round trip.
   *
   * The last tap wins. A kana is opened directly only when its script's course is
   * the one on screen; otherwise it is handed to the load that is bringing that
   * course, which leaves the *latest* request in force — so a tap that overtakes an
   * earlier one is the one that lands, and a load that fails takes its request with
   * it.
   */
  async function openKana(ch: string, where?: ScriptName) {
    goTo("kana", "practice");
    error = null;
    let target = where;
    if (!target) {
      try {
        target = (await api.kana(ch)).script;
      } catch (e) {
        error = String(e);
        return;
      }
    }
    if (target !== script || loadedScript !== target) {
      wanted = { ch, script: target };
      script = target;
      return;
    }
    lessonKey = lessonKeyOf(course, ch) ?? lessonKey;
    await select(ch);
  }

  async function boot() {
    try {
      [info, stats] = await Promise.all([api.appInfo(), api.datasetStats()]);
    } catch (e) {
      error = String(e);
    }
    try {
      // The course the learner left in, if there is one. A stored value is
      // narrowed rather than trusted: it crosses a file boundary, and a hand
      // edit there must open the on-ramp rather than throw the switch at a
      // screen that does not exist.
      const saved = await api.prefs();
      if (isSection(saved.section)) {
        section = saved.section;
        view = defaultView(saved.section);
      }
    } catch (e) {
      // Read once, and said if it fails: the only way `prefs` rejects is the IPC
      // itself, and the app opens on the on-ramp either way.
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
      joined = [];
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
    joined = [];
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
      joined = graded.joined;
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
  <!--
    The app's name is the window's own title on the desktop and is not repeated on
    screen: it is four words of the most valuable space in the interface, and every
    screen underneath is worth more (`ROADMAP_NIHONGO.md` N10). The heading stays
    for a reader that cannot see the title bar or the footer, at no cost to either.
  -->
  <h1 class="sr-only">Nihongo Tutor</h1>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  {#if warning}
    <p class="warning" role="status">{warning}</p>
  {/if}

  <!--
    The course switch, not a tab: each course is a thing to study rather than a
    screen. Two lines each and no more — the name, then what it is and how much of
    it there is, because 177 characters is a few days and 2,136 behind 16,073 words
    is not.
  -->
  <nav class="courses" aria-label="Courses">
    {#each COURSES as course (course.id)}
      <button
        class="course"
        class:active={section === course.id}
        aria-pressed={section === course.id}
        onclick={() => chooseSection(course.id)}
      >
        <span class="name">{course.label}</span>
        <span class="info">{course.tagline} · {courseSize(course.id)}</span>
      </button>
    {/each}
  </nav>

  <div class="views" role="tablist" aria-label={`${courseOf(section).label} screens`}>
    {#each tabsOf(section) as tab (tab.id)}
      <button
        role="tab"
        aria-selected={view === tab.id}
        class:active={view === tab.id}
        onclick={() => (view = tab.id)}>{tab.label}</button
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
          {@const drawnJoined = joinedLabel(joined)}
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
            {#if drawnJoined}
              <p class="fine">Taught strokes {drawnJoined} were drawn as one.</p>
            {/if}
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
                  <button class="cell small" onclick={() => openKana(c.ch)}>{c.ch}</button>
                  <span>{c.tell}</span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      {/if}
    </section>
  </div>

  {:else if view === "chart"}
    <KanaChart onopen={openKana} />
  {:else if view === "drill"}
    <ConfusionDrill onopen={openKana} />
  {:else if view === "review"}
    <!--
      The same screen in both courses, told which one is open: the schedule is one
      file of characters (invariant 15) and the section is what decides which of
      them this queue offers and counts. Keyed on the section so that changing
      course mounts a fresh panel — the due list, the chosen item and the board on
      it are all that course's, and none of them can be carried across.
    -->
    {#key section}
      <ReviewPanel {section} {voice} />
    {/key}
  {:else if view === "kanji"}
    <KanjiPanel bind:pick={kanjiPick} onradical={seeRadical} {voice} />
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
    <span>Everything the courses teach is in this bundle. Nothing is downloaded.</span>
    <span>
      {#if canHear(voice)}
        Pronunciation uses the system's own Japanese voice. Nothing is downloaded.
      {:else if voice === null}
        No Japanese voice is installed, so nothing here can be heard.
      {:else}
        Looking for a Japanese voice…
      {/if}
    </span>
    <!--
      Licences belongs to neither course — it is a notice the app owes rather than
      something it teaches — so it hangs off the footer instead of taking a tab
      away from one of them. It stays reachable from either course, and the tab row
      above is how the learner leaves again.
    -->
    <span class="utility">
      <button
        class="notice-link"
        class:active={view === "licences"}
        aria-pressed={view === "licences"}
        onclick={() => (view = "licences")}>Licences</button
      >
    </span>
  </footer>
</main>

<svelte:window onkeydown={onKey} />
