<script lang="ts">
  /**
   * Nihongo Tutor: two courses, one at a time.
   *
   * **Kana** is the on-ramp — the course and the board a lesson opens — and
   * **Kanji** is the course itself: the character course, the radical table, the
   * vocabulary and the passages. Which of the two is open is remembered in the
   * app's own file, and the tabs under the switch are that course's own screens,
   * from `lib/nav.ts`.
   *
   * Every course screen that opens something is **two screens**: the list of what to
   * study, and the stage for one of them — a board, a radical's family, a word's card,
   * a passage — taken by a panel that reports it (`bind:stage`) so the app can step
   * its own chrome aside. `HANDOVER_NIHONGO.md` invariants 33, 34 and 35 are the rule,
   * and this file is where the panels are handed the request that opened one.
   *
   * Every character on the board, kana or kanji, is graded by the same engine that
   * grades a Chinese character.
   */
  import ConfusionDrill from "./lib/ConfusionDrill.svelte";
  import KanaChart from "./lib/KanaChart.svelte";
  import KanaPanel from "./lib/KanaPanel.svelte";
  import KanjiPanel from "./lib/KanjiPanel.svelte";
  import LicencesPanel from "./lib/LicencesPanel.svelte";
  import PassagePanel from "./lib/PassagePanel.svelte";
  import PhrasesPanel from "./lib/PhrasesPanel.svelte";
  import RadicalsPanel from "./lib/RadicalsPanel.svelte";
  import ReviewPanel from "./lib/ReviewPanel.svelte";
  import StartPanel from "./lib/StartPanel.svelte";
  import VocabularyPanel from "./lib/VocabularyPanel.svelte";
  import {
    COURSES,
    courseOf,
    covers,
    defaultView,
    hasStage,
    isSection,
    tabsOf,
    type Section,
    type View,
  } from "./lib/nav";
  import { canHear, type VoiceStatus } from "./lib/speech";
  import * as api from "./lib/api";
  import type {
    AppInfo,
    DatasetStats,
    KanaPick,
    KanjiPick,
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
  let error = $state<string | null>(null);

  /**
   * A kana another screen has asked the kana board to open, waiting for that
   * course to be the one on screen.
   *
   * The chart, the drill and the stage's own confusions list all end up here. The
   * request carries the script it is written in, because the kana course is one
   * course per script: a request that did not say which would let a katakana kana
   * be looked up in the hiragana lessons.
   */
  let kanaPick = $state<KanaPick | null>(null);

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
   * Whether a course screen's stage is up, which its panel tells us.
   *
   * A stage takes the whole screen: the course switch and the tab row are the
   * *course's*, and while one thing is being worked on they cost the phone about
   * 150px of the height the thing needs. The stage's own first control is the way
   * back, and it brings both rows with it (invariants 33, 34 and 35).
   *
   * One flag per panel rather than one for the app, because the screens hand each
   * other the learner: a component tapped inside the kanji stage's fold sends them
   * to the radicals panel while the kanji panel's own stage is still up, and a
   * shared flag would then keep the chrome hidden on a course screen.
   */
  let kanaStage = $state(false);
  let kanjiStage = $state(false);
  let radicalsStage = $state(false);
  let wordsStage = $state(false);
  let readStage = $state(false);

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
   * Open a kana on the practice stage, from wherever it was offered.
   *
   * The chart, the drill and the confusions list on a stage are all promises that
   * tapping a kana writes it. The script is not worked out from the character
   * here: the app's own dataset is the authority on which of り and リ is which and
   * on ー being katakana, and it already says so in the `Kana` it returns. A caller
   * that knows the script — the chart does — passes it and saves the round trip.
   *
   * The request is handed to `KanaPanel` whole, script included, and the panel is
   * what decides when it can be honoured: the kana's own course has to be the one
   * loaded, or a katakana kana would land among the hiragana lessons. The last tap
   * wins, because the panel consumes the request it finds rather than a queue of
   * them.
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
    kanaPick = { ch, script: target };
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

  /**
   * Whether a stage is up on the screen that is open.
   *
   * A stage belongs to one panel, and `bind:stage` is how that panel says so; the
   * five that can are the five `nav.ts` names as two-screen. A sixth that kept a
   * flag without being listed there, or was listed there and not handled here, is a
   * programming error rather than a course screen that quietly keeps its chrome —
   * the same refusal `goTo` makes for a screen its course does not have.
   */
  const stageUp = $derived.by(() => {
    switch (view) {
      case "practice":
        return kanaStage;
      case "kanji":
        return kanjiStage;
      case "radicals":
        return radicalsStage;
      case "words":
        return wordsStage;
      case "read":
        return readStage;
      default:
        if (hasStage(view)) throw new Error(`${view} has a stage and no flag`);
        return false;
    }
  });

  /**
   * Whether the app's own chrome — the switch and the open course's tab row —
   * should be drawn. Licences and Start here hang off the footer either way.
   */
  const chromeVisible = $derived(!stageUp);

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
   * Leaving the screen ends whatever is being said.
   *
   * A voice that keeps talking over the next thing the learner looks at is the
   * one way an audio feature becomes an annoyance. The stage stops it when the
   * character on the board changes; this is the half that covers leaving the
   * screen altogether, and speaking never changes `view` — the button does that —
   * so this effect only ever stops.
   */
  $effect(() => {
    void view;
    void api.stopSpeaking().catch(() => {
      // Nothing to report: this is cleanup, and the usual answer is that nothing
      // was being said.
    });
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
    is not. Both rows step aside while a stage is up: the stage is the screen then,
    and it carries its own way back.
  -->
  {#if chromeVisible}
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
  {/if}

  {#if view === "practice"}
    <!--
      The kana course, and the stage a lesson opens: two screens inside one panel,
      because the request that opens a kana has to arrive with the script it is
      written in and the panel is what owns the course that request belongs to
      (invariant 34).
    -->
    <KanaPanel bind:pick={kanaPick} bind:stage={kanaStage} {voice} />
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
    <KanjiPanel bind:pick={kanjiPick} bind:stage={kanjiStage} onradical={seeRadical} {voice} />
  {:else if view === "radicals"}
    <RadicalsPanel
      bind:focus={selectedRadical}
      bind:stage={radicalsStage}
      onopen={openKanji}
      onpractise={openRadical}
    />
  {:else if view === "words"}
    <VocabularyPanel bind:stage={wordsStage} {voice} />
  {:else if view === "phrases"}
    <!--
      No `stage` binding: a phrase's word card opens under the row it was tapped in,
      so this screen never becomes a stage and the app's chrome stays where it is
      (`nav.ts`'s `STAGE_VIEWS` is the list of screens that do, and `nav.test.ts`
      asserts Phrases is not one of them).
    -->
    <PhrasesPanel {voice} />
  {:else if view === "read"}
    <PassagePanel bind:stage={readStage} {voice} />
  {:else if view === "start"}
    <!--
      Start here belongs to neither course: it is about the language rather than
      about a set of characters, so it hangs off the footer beside Licences. Its
      own two buttons are how it hands the learner to a course, which is why the
      callback goes through `goTo` — the same guarded door every other screen
      moves through, so a course and the screen it opens on cannot disagree.
    -->
    <StartPanel oncourse={(next) => goTo(next, defaultView(next))} />
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
      Licences and Start here belong to neither course — one is a notice the app
      owes, the other is about the language rather than about a set of characters —
      so both hang off the footer instead of taking a tab away from a course. They
      stay reachable from either course, and the tab row above is how the learner
      leaves again.
    -->
    <span class="utility">
      <button
        class="notice-link"
        class:active={view === "start"}
        aria-pressed={view === "start"}
        onclick={() => (view = "start")}>Start here</button
      >
      <button
        class="notice-link"
        class:active={view === "licences"}
        aria-pressed={view === "licences"}
        onclick={() => (view = "licences")}>Licences</button
      >
    </span>
  </footer>
</main>
