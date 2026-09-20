<script lang="ts">
  import { onMount } from "svelte";
  import {
    confirm as confirmDialog,
    open as pickFile,
    save as pickSavePath,
  } from "@tauri-apps/plugin-dialog";
  import * as api from "./lib/api";
  import CharacterThumb from "./lib/CharacterThumb.svelte";
  import FeedbackPanel from "./lib/FeedbackPanel.svelte";
  import Icon from "./lib/Icon.svelte";
  import LessonSidebar from "./lib/LessonSidebar.svelte";
  import LicencesPanel from "./lib/LicencesPanel.svelte";
  import PracticeCanvas from "./lib/PracticeCanvas.svelte";
  import SettingsPanel from "./lib/SettingsPanel.svelte";
  import TonePanel from "./lib/TonePanel.svelte";
  import VocabularyPanel from "./lib/VocabularyPanel.svelte";
  import WordsPanel from "./lib/WordsPanel.svelte";
  import { INK_WIDTH, polylineLength } from "./lib/render";
  import type { Sweep } from "./lib/render";
  import type {
    AppInfo,
    Character,
    DatasetStats,
    GradeReport,
    Lesson,
    LicenceNotice,
    Point,
    PracticeItem,
    ProgressView,
    ReviewView,
    SettingsPatch,
    SettingsView,
    SyncView,
    MicrophoneStatus,
    ToneResult,
    ToneTarget,
    VocabEntry,
    VocabView,
    VoicesView,
    Word,
    WordSearchView,
  } from "./lib/types";

  type Mode = "trace" | "recall";
  /** Which of the five top-level screens is showing. */
  type View = "course" | "vocabulary" | "words" | "settings" | "about";
  /** Where the current practice session draws its characters from. */
  type Source = "course" | "vocabulary" | "review" | "words";

  /** One character's worth of a multi-character entry: what was drawn, and the
   * grade it got. `report` is null when the character is still to be written. */
  interface SlotState {
    strokes: Point[][];
    report: GradeReport | null;
  }

  /** A character of the entry that has not been touched. One shared object, so
   * re-rendering the strip does not hand the thumbnails a new array each time. */
  const EMPTY_SLOT: SlotState = { strokes: [], report: null };

  const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));
  /** Resolve on the next animation frame, so the pen is paced by the display. */
  const nextFrame = () =>
    new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

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
   * The learner's own settings, as the backend has them.
   *
   * `clickToDraw` is `null` until they choose; see [`clickToDraw`] for how an
   * unchosen value is resolved. The pace and the board size are always a value:
   * there is no device signal to read one from, so "nobody chose" and "chose the
   * default" are the same thing and the stored absence means the default.
   */
  let settings = $state<SettingsView>({
    clickToDraw: null,
    voice: null,
    animationPace: "normal",
    boardSize: "normal",
    warning: null,
  });

  /**
   * The voices this machine offers, and the one in use.
   *
   * Read when the settings screen is opened rather than at startup: it is a
   * cached list on the backend, but nothing else in the app needs it, and the
   * practice screen only needs to know *whether* there is a voice at all
   * ([`voice`]).
   */
  let voices = $state<VoicesView>({ available: [], active: null });
  /** True while the voice list is still being read. */
  let voicesLoading = $state(true);

  /**
   * True while the "How this works" explanation is expanded.
   *
   * Collapsed by default: on a phone the paragraph is taller than the board's
   * controls, and what a learner needs in front of them is the board, not an
   * essay about the grading. The heading is the disclosure, so the explanation
   * is one tap away and costs one row when it is not wanted.
   */
  let showHelp = $state(false);

  /**
   * True while the character's details are unfolded.
   *
   * Only the phone honours it: there the header keeps the reading and the first
   * two lines of the meaning, and the strokes, radical, HSK level, frequency,
   * progress and etymology are one tap away — a phone that spent four lines on
   * facts it was not asked for had no room left for the board. A wide screen
   * shows all of it whatever this says, and the control that sets it is not
   * drawn there at all.
   */
  let showDetails = $state(false);

  /**
   * True while the navigation sheet is open.
   *
   * Only meaningful at phone widths, where the sidebar is a fixed sheet over the
   * board rather than a column beside it; on a wide screen the same element is
   * just the first column and this is never set.
   */
  let navOpen = $state(false);

  /**
   * Wrap a navigation callback so that using it also closes the sheet.
   *
   * Every route out of the sidebar changes what is on the board, and leaving the
   * sheet open over the character the reader just chose would hide the result.
   */
  function navigating<A extends unknown[]>(action: (...args: A) => void) {
    return (...args: A) => {
      navOpen = false;
      action(...args);
    };
  }

  /**
   * Whether this device wants click-to-draw *by default*.
   *
   * A trackpad or a mouse has hover, so a stroke can be started and finished
   * with two clicks and nothing held down — which is the only comfortable way to
   * write a long stroke with them, and the reason the feature exists. A stylus
   * or a finger has no hover: dragging is the natural gesture there, and
   * click-to-draw would be a mode to escape from. `hover: none` is the honest
   * signal for that; `pointer: coarse` is there because the two disagree on some
   * platforms, and either one being true means this is not a mouse.
   */
  const deviceWantsClickToDraw = !window.matchMedia("(hover: none), (pointer: coarse)")
    .matches;

  /**
   * How a stroke is committed: dragged with the button held, or started and
   * finished with two clicks.
   *
   * The learner's choice wins when they have made one; otherwise the device
   * decides. `null` in the stored settings is what makes that distinguishable
   * from a deliberate "off".
   */
  const clickToDraw = $derived(settings.clickToDraw ?? deviceWantsClickToDraw);

  /**
   * How much of the room available the board takes, from the board-size
   * preference.
   *
   * A share of the container rather than a pixel side: the board fits whatever
   * window it is in, and the preference says how much of that fit to use, so a
   * large board stays large on a big screen and a compact one stays usable on a
   * small one.
   *
   * **This is the only place a board size becomes a number.** `BoardSize` in
   * `hanzi-core` carries the name, not the fraction, precisely so that the value
   * cannot exist in two places and drift; the same goes for the pace below.
   */
  const boardFraction = $derived(
    settings.boardSize === "compact" ? 0.72 : 1,
  );

  /**
   * The scale applied to the whole stroke-order timeline, from the pace
   * preference.
   *
   * Scaling the timeline rather than the bounds is the point: the animation caps
   * a single stroke and the total (`MAX_STROKE_MS`, `MAX_TOTAL_MS`), and leaving
   * those fixed would mean `Slow` changed nothing for a character at the cap.
   */
  const paceScale = $derived(
    settings.animationPace === "slow" ? 2 : settings.animationPace === "fast" ? 0.5 : 1,
  );

  /**
   * The TTS voice, once known: a name when pronunciation is available, `null`
   * when the system has no Chinese voice, `undefined` while still resolving.
   */
  let voice = $state<string | null | undefined>(undefined);

  // ---- tone practice --------------------------------------------------------
  /**
   * The microphone, once known.
   *
   * `undefined` while it is being resolved, and `null` when the question itself
   * failed. The control is offered only when `available` is true, so a machine
   * with no microphone gets a disabled button with a reason rather than a button
   * that silently records nothing.
   */
  let microphone = $state<MicrophoneStatus | null | undefined>(undefined);
  /**
   * What the tones should be, or `null` when nothing can score this text — more
   * than a word, or not in the dataset. It used to be `null` for a neutral-tone
   * reading as well, which is why the most common character in the language had
   * a tone button that could not be pressed.
   */
  let toneTarget = $state<ToneTarget | null>(null);
  /** The last judgement, or `null` before anything has been said. */
  let toneResult = $state<ToneResult | null>(null);
  /** True between pressing and releasing the button. */
  let listening = $state(false);
  /** True while the device is opening or the recording is being scored. */
  let toneBusy = $state(false);
  /** Why the last attempt could not even be recorded. */
  let toneError = $state<string | null>(null);
  /** The text captured when the button went down, which is what gets judged. */
  let scoredText = $state("");

  let view = $state<View>("course");

  // ---- about and licences ---------------------------------------------------
  /**
   * The app's own name, version and licence.
   *
   * Loaded once with the course rather than when the screen is opened: it is a
   * handful of strings, and the About screen should never show a spinner.
   */
  let appInfo = $state<AppInfo | null>(null);
  /** The bundled notices, full text included. */
  let licenceList = $state<LicenceNotice[]>([]);
  /**
   * Set if the notices could not be read.
   *
   * They are compiled into the binary, so this should be unreachable — but it is
   * shown on the About screen rather than swallowed, because an empty notice
   * list would otherwise look like an app that ships nothing to declare.
   */
  let licenceError = $state<string | null>(null);

  // ---- syncing that nobody asked for --------------------------------------

  /**
   * What the launch or foreground sync has to say, or `null` for silence.
   *
   * `working` makes the line read as an activity rather than a result, which is the
   * whole of the feedback this feature has: a sync that runs by itself has to look
   * like something happening, or a slow one looks like a hang.
   */
  let syncNotice = $state<{
    text: string;
    working: boolean;
    trouble: boolean;
  } | null>(null);
  /** True while an automatic sync is in flight, so a second cannot start on top. */
  let syncRunning = false;
  /**
   * Bumped after every automatic sync, so the settings screen re-reads its own copy
   * of the sync state.
   *
   * It holds the same facts the backend does — whether an account is connected, how
   * the sign-in is protected, what the last sync did — and it is mounted only while
   * it is open. Without this, leaving the app on the settings screen, going away and
   * coming back would show the sync before last.
   */
  let syncPulse = $state(0);
  /** When the last automatic attempt started, for the cooldown below. */
  let lastAutoSync = 0;

  /**
   * How often coming back to the app may start a sync.
   *
   * Focus fires whenever the window is raised, which on a desktop is every alt-tab —
   * and a sync per alt-tab is a request per alt-tab to somebody else's servers for
   * data that has not changed. A minute is longer than a learner can notice missing
   * and far longer than alt-tabbing takes.
   */
  const AUTO_SYNC_COOLDOWN = 60_000;

  /** How long a sync may take before it is worth saying that it is happening. */
  const SYNC_NOTICE_DELAY = 400;

  let noticeTimer: ReturnType<typeof setTimeout> | undefined;
  let workingTimer: ReturnType<typeof setTimeout> | undefined;

  /**
   * Put a line on the screen, or take it away.
   *
   * A result stays long enough to read and then leaves on its own. There is no
   * dismiss button: this is not a dialog, it is the app saying what it is doing, and
   * anything that has to be dismissed is worse than the silence it replaced.
   */
  function showSyncNotice(
    text: string | null,
    options: { working?: boolean; trouble?: boolean } = {},
  ) {
    clearTimeout(noticeTimer);
    if (text === null) {
      syncNotice = null;
      return;
    }
    const working = options.working ?? false;
    syncNotice = { text, working, trouble: options.trouble ?? false };
    // The working line is replaced by a result rather than expiring, so its own
    // timeout is only a backstop against a backend that never answers.
    noticeTimer = setTimeout(() => (syncNotice = null), working ? 30_000 : 6_000);
  }

  /**
   * The one line worth showing after an automatic sync, or `null`.
   *
   * Silence when nothing changed, which is the common case and the one that decides
   * whether this feature is welcome or annoying: the working line disappearing is
   * itself the answer to "did it sync?", and a sentence saying "already up to date"
   * on every launch is noise nobody can switch off. `leftAlone` is left out of that
   * judgement deliberately — it describes a schedule this device cannot rebuild,
   * which is a standing condition rather than news, and the settings screen says it
   * in full.
   */
  function autoSyncLine(view: SyncView): string | null {
    const last = view.last;
    const changed =
      last !== null &&
      (last.published > 0 ||
        last.pulled > 0 ||
        last.recomputed > 0 ||
        last.vocabChanged > 0 ||
        last.cursorMoved);
    return changed ? view.message : null;
  }

  /**
   * Sync because the app started or came back.
   *
   * The backend refuses three cases before it sends anything — no account, a locked
   * sign-in, and no network — and each of those answers in a moment, so the cost of
   * asking on every launch is a round trip to Rust rather than a round trip to
   * Dropbox. What comes back decides whether there is anything to say.
   *
   * The reload discipline matters here more than anywhere else, because this fires
   * at moments nobody chose: the backend has re-read its stores by the time this
   * returns, and `refreshAfterSync` is what re-reads *this* side's copies of them.
   * Without it a launch sync would write the right things into the database and go
   * on showing the old ones until the app was restarted — which is the exact bug
   * this app has already had twice.
   */
  async function autoSync(reason: "launch" | "foreground") {
    if (syncRunning) return;
    const started = Date.now();
    if (started - lastAutoSync < AUTO_SYNC_COOLDOWN) return;
    lastAutoSync = started;
    syncRunning = true;

    workingTimer = setTimeout(
      () => showSyncNotice("Syncing with your other devices…", { working: true }),
      SYNC_NOTICE_DELAY,
    );

    try {
      const outcome = await api.syncAuto();
      const quiet = outcome.outcome === "skipped" || outcome.outcome === "offline";
      void api.log(
        `sync on ${reason}: ${outcome.outcome}` +
          (quiet ? ` (${outcome.reason})` : ""),
      );
      clearTimeout(workingTimer);
      switch (outcome.outcome) {
        case "synced":
          refreshAfterSync();
          syncPulse += 1;
          showSyncNotice(autoSyncLine(outcome.view));
          break;
        case "offline":
          showSyncNotice(
            "No connection, so nothing was synced. Your practice is safe on this device.",
          );
          break;
        case "failed":
          showSyncNotice(`Could not sync: ${outcome.reason}`, { trouble: true });
          break;
        case "skipped":
          // Not connected, or the sign-in is locked. Both are said in full on the
          // settings screen, and neither is worth interrupting a lesson for.
          showSyncNotice(null);
          break;
      }
    } catch (cause) {
      clearTimeout(workingTimer);
      showSyncNotice(`Could not sync: ${cause}`, { trouble: true });
    } finally {
      syncRunning = false;
    }
  }

  // ---- personal vocabulary list -------------------------------------------
  let vocab = $state<VocabView>({ entries: [], groups: [], warning: null });
  /** `null` shows everything, `""` the unfiled entries, otherwise a group. */
  let vocabSelection = $state<string | null>(null);
  let statusMessage = $state<string | null>(null);
  let vocabBusy = $state(false);

  // ---- the word dictionary -------------------------------------------------
  /** The HSK level the words screen is filtered to, or null for all of them. */
  let wordLevel = $state<number | null>(null);
  /**
   * What is in the words screen's search box.
   *
   * Held here rather than inside the panel so that drilling a word and coming
   * back does not lose the search that found it.
   */
  let wordQuery = $state("");
  /** A note from the words screen, e.g. what was just added to the list. */
  let wordMessage = $state<string | null>(null);
  /**
   * Every character the board can draw.
   *
   * A word — or a sentence — is written one character at a time, so anything in
   * the text with no strokes to grade (a comma, an unknown glyph) has to be
   * skipped rather than dead-ending the board. Loaded once, like the course.
   * While it is empty the text is split as typed, which is only reachable in the
   * moment before startup finishes.
   */
  let drawable = $state<ReadonlySet<string>>(new Set());

  // ---- progress and review -------------------------------------------------
  /** Per-character history and due dates, as the backend has them. */
  let progress = $state<ProgressView>({ cards: [], warning: null });
  /** What is due for review now. `items` is one session; `dueCount` is all. */
  let review = $state<ReviewView>({ items: [], dueCount: 0, warning: null });
  /** Set when the saved course cursor could not be read. */
  let cursorWarning = $state<string | null>(null);
  /** False until the saved cursor has been restored, so the first render does
   * not write back the position it is about to adopt. */
  let cursorRestored = $state(false);
  /** The position last handed to the backend, so a redraw is not a save. */
  let cursorSaved = -1;
  let cursorTimer: ReturnType<typeof setTimeout> | undefined;

  // ---- practising the list -------------------------------------------------
  /**
   * Which sequence the board is working through. The course is one character
   * per step; a vocabulary entry or a review item may be several characters,
   * written in turn.
   */
  let source = $state<Source>("course");
  /** Snapshot of what is being drilled, taken when practice starts. */
  let queue = $state<PracticeItem[]>([]);
  let queueCursor = $state(0);
  /** Which character of the current entry is being written. */
  let charCursor = $state(0);
  /**
   * What has been written for each character of the current entry, one slot per
   * `entryCharacters` index.
   *
   * A word is written one character at a time, and the boxes under the board
   * show how far through it the learner is, so each character keeps its own
   * attempt and grade instead of one running list. A slot is a *snapshot taken
   * when the board leaves it*: whatever is on the board now is the current
   * character's attempt, and `slotAt` reads it from the live state so the
   * thumbnail follows the pen. One slot holds one grade, so revisiting a
   * character to improve it replaces its score rather than adding a second.
   */
  let wordSlots = $state<SlotState[]>([]);
  /** True once the last item of a list or review session has been finished, so
   * the last entry cannot be recorded twice. */
  let sessionDone = $state(false);

  /** Position in the flattened course; lessons are just a view onto it. */
  let index = $state(0);
  /** How much of the reference character is on the board. */
  let revealed = $state(0);
  /** True while the stroke-order animation runs. */
  let playing = $state(false);
  /**
   * Where the stroke-order pen is: which reference stroke, and how far along it.
   *
   * `revealed` counts the strokes already finished, so the two together say how
   * much of the character is drawn. Null whenever no pen is on the board.
   */
  let sweep = $state<Sweep | null>(null);
  let playToken = 0;

  const allCharacters = $derived(lessons.flatMap((lesson) => lesson.characters));
  const courseChar = $derived(allCharacters[index] ?? null);
  const currentItem = $derived(
    source === "course" ? null : (queue[queueCursor] ?? null),
  );
  /**
   * The text tone practice is scoring — a character, or a whole word.
   *
   * A word is scored as a word, not character by character, because tone sandhi
   * happens *between* the syllables of a word: 你好 is spoken 2 + 3 where the
   * dictionary has 3 + 3 for each character alone. It is the same text the "Hear
   * it" button speaks, so the two controls always agree about what is on the
   * board.
   */
  const toneText = $derived(currentItem?.text ?? character?.ch ?? "");
  /**
   * What the microphone button offers to score, when there is anything to score.
   *
   * The syllables are named rather than left to "it": on a phone there is no
   * tooltip, so a button that only says "hold to say" leaves the learner
   * guessing what the microphone is listening for.
   */
  const sayPrompt = $derived(
    toneTarget === null
      ? ""
      : `Hold and say ${toneText} — ${toneTarget.syllables
          .map((s) => (s.spoken === 5 ? "neutral" : `tone ${s.spoken}`))
          .join(", ")}${toneTarget.sandhiApplied ? " as it is spoken in this word" : ""}`,
  );
  /**
   * Why the microphone button cannot be used, or `null` when it can.
   *
   * One expression, said in three places, so they cannot drift apart: the
   * button's tooltip, its accessible name, and the hint under the row. The hint
   * is what carries the reason on a phone, where an icon has no room for the
   * sentence the old text label used to spell out and there is no hover to put
   * a tooltip on. Two reasons are worth telling apart: 的 is a neutral-tone
   * particle with nothing to score, which is not the learner's fault and not
   * fixable, whereas a missing microphone is.
   */
  const sayBlocked = $derived(
    microphone === undefined
      ? "Looking for a microphone…"
      : microphone === null || !microphone.available
        ? (microphone?.detail ?? "No microphone is available")
        : toneTarget === null
          ? "Nothing on this board has a tone to score (too long to score, or a neutral-tone particle like 的)"
          : null,
  );
  /** Code-point split, matching Rust's `chars()`. */
  const entryCharacters = $derived(
    currentItem
      ? [...currentItem.text].filter((ch) => drawable.size === 0 || drawable.has(ch))
      : [],
  );
  /**
   * The character the board is asking for. Everything downstream — loading,
   * grading, the ghost and the hint — keys off this, so all four sources share
   * one practice path.
   */
  const targetChar = $derived(
    source === "course" ? courseChar : (entryCharacters[charCursor] ?? null),
  );
  /** What has been recorded about the character on the board, if anything. */
  const activeCard = $derived(
    targetChar
      ? (progress.cards.find((card) => card.ch === targetChar) ?? null)
      : null,
  );
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
  /**
   * Whether the top-left box shows the character or hides it behind a `?`.
   *
   * Recall mode withholds the answer until the learner has committed to one, and
   * **grading is that commitment**: pressing Check is the moment the attempt is
   * finished, so the correct form is then what they need to see — comparing what
   * they wrote against the real character is the whole of the feedback. Before
   * this, only the stroke-order animation revealed it, so a graded recall left a
   * `?` sitting beside the score, which is the one moment the box has something
   * to say.
   *
   * Writing again, undoing or switching mode clears the report, which hides the
   * answer again — so the reveal is tied to the grade it belongs to rather than
   * to a flag that could outlive it.
   */
  const answerVisible = $derived(mode === "trace" || revealed > 0 || report !== null);

  /**
   * True while the board is the screen showing.
   *
   * The phone's top bar carries the character navigation, which belongs to the
   * board: on the four list screens there is nothing of the sort to navigate, and
   * a ← that moved the course cursor behind the settings screen would be a
   * control acting on something nobody can see. This is the same condition as the
   * branch order in the markup below, which the type checker needs written as
   * `!character` to know the board's character exists — keep the two together.
   */
  const practising = $derived(
    !loading &&
      character !== null &&
      !(view === "vocabulary" && source !== "vocabulary") &&
      !(view === "words" && source !== "words") &&
      view !== "settings" &&
      view !== "about",
  );

  /**
   * What ← and → do, and what the "8 / 7744" between them counts.
   *
   * Both sources of characters end in the same two arrows, but they move through
   * different sequences: the course steps through the whole frequency list, while
   * a word list, the vocabulary list and a review session step through their own
   * queue. Which one is live is decided here rather than in the markup, because
   * the widget is drawn in two places — the phone's top bar and the header beside
   * the board — and both have to move the same thing.
   */
  const nav = $derived(
    currentItem
      ? {
          back: () => moveQueue(-1),
          forward: () => moveQueue(1),
          position: queueCursor + 1,
          total: queue.length,
          first: queueCursor === 0,
          last: queueCursor >= queue.length - 1,
        }
      : {
          back: () => move(-1),
          forward: () => move(1),
          position: index + 1,
          total: allCharacters.length,
          first: index === 0,
          last: index >= allCharacters.length - 1,
        },
  );

  const summary = $derived(
    stats
      ? `${stats.teachable.toLocaleString()} characters in ${stats.lessons.toLocaleString()} lessons`
      : "loading…",
  );

  /**
   * A short, human label for a due date: "today", "tomorrow", "in 5 days".
   *
   * The backend compares due timestamps as text; here the label only has to be
   * readable, so it is measured from the clock in whole days.
   */
  function dueLabel(due: string): string {
    const remaining = Date.parse(due) - Date.now();
    if (Number.isNaN(remaining)) return due;
    if (remaining <= 0) return "now";
    if (remaining < 86_400_000) return "today";
    const days = Math.round(remaining / 86_400_000);
    if (days <= 1) return "tomorrow";
    if (days < 30) return `in ${days} days`;
    const months = Math.round(days / 30);
    return months <= 1 ? "next month" : `in ${months} months`;
  }

  onMount(() => {
    /**
     * Bring runtime errors out to the terminal.
     *
     * A webview's console is invisible from here, so an exception thrown while
     * rendering — a duplicate key in an `each`, a bad property access — shows up
     * only as a window that stops updating, with nothing to go on. Svelte
     * reports these through `window.onerror`, so forwarding them to the same log
     * the rest of the startup uses turns a blank screen into a line of text.
     */
    const onError = (event: ErrorEvent) => {
      void api.log(`webview error: ${event.message}`);
    };
    const onRejection = (event: PromiseRejectionEvent) => {
      void api.log(`webview rejection: ${event.reason}`);
    };
    window.addEventListener("error", onError);
    window.addEventListener("unhandledrejection", onRejection);

    /**
     * Publish the system bar insets the platform measured.
     *
     * Android's webview answers `env(safe-area-inset-*)` with the display cutout
     * rather than the status bar, so on a phone without a notch or punch-hole
     * those are zero and the header is drawn underneath the clock. `app.css`
     * takes the larger of that and these, so this only has to report the
     * platform's own numbers — and on iOS they are zero and change nothing.
     * Re-read on resize, which is what a rotation is, because the insets swap
     * sides when the phone turns.
     */
    const publishInsets = () => {
      void api
        .androidInsets()
        .then((insets) => {
          const root = document.documentElement.style;
          root.setProperty("--inset-top", `${insets.top}px`);
          root.setProperty("--inset-bottom", `${insets.bottom}px`);
        })
        .catch(() => {
          // Every platform but Android reports zero, and a failure there leaves
          // the CSS `env()` values in charge. Nothing to report to the learner.
        });
    };
    publishInsets();
    window.addEventListener("resize", publishInsets);

    /**
     * What Android's back gesture does before it leaves the app.
     *
     * Back is the platform's own version of Escape, and the first thing a
     * learner expects it to dismiss is whatever is covering the board — the
     * navigation sheet, then the explanation. Answering `true` says the press
     * was used; `false` lets Android finish the activity. `MainActivity.kt` is
     * the only caller, so on every other platform this is never invoked.
     */
    const platform = window as unknown as { __hanziHandleBack?: () => boolean };
    platform.__hanziHandleBack = () => {
      if (navOpen) {
        navOpen = false;
        return true;
      }
      if (showHelp) {
        showHelp = false;
        return true;
      }
      return false;
    };

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
        // Only now is the course length known, which is what a saved position
        // has to be checked against.
        await restoreCursor();
      } catch (cause) {
        error = `Could not load the course: ${cause}`;
      } finally {
        loading = false;
      }
    })();

    void refreshVocabulary();
    void refreshSettings();
    void refreshProgress();
    void refreshReview();


    // What the app is and what it ships under. Small, and wanted the instant the
    // About screen is opened, so it is fetched with the rest of the startup
    // rather than on demand.
    void api
      .appInfo()
      .then((loaded) => {
        appInfo = loaded;
      })
      .catch((cause) => {
        void api.log(`could not read the app info: ${cause}`);
      });
    void api
      .licenceNotices()
      .then((notices) => {
        licenceList = notices;
        void api.log(`licences: ${notices.length} notices bundled`);
      })
      .catch((cause) => {
        licenceError = `Could not read the bundled licence notices: ${cause}`;
        void api.log(`webview error: ${licenceError}`);
      });

    // Which characters the board can draw. Only needed to write a word or a
    // sentence one character at a time, but it is small and wanted immediately.
    void api
      .teachableCharacters()
      .then((characters) => {
        drawable = new Set(characters);
        void api.log(`drawable characters: ${characters.length}`);
      })
      .catch((cause) => {
        void api.log(`could not list the drawable characters: ${cause}`);
      });

    // Resolving the voice runs the system voice list, which takes about a
    // second, so it is deliberately not part of the course load above. This also
    // fills in the settings screen's list of voices, from the same cached
    // enumeration.
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
    void refreshVoices();

    // What the synthesiser is really doing, said once in the log. A phone that
    // lists Chinese voices and then makes no sound is a real failure mode, and
    // this line is what tells the two apart: no engine, an engine with no
    // Chinese voice, a voice whose data was never downloaded, or a voice that
    // needs a network — which this app will not use at runtime.
    void api
      .speechReport()
      .then((report) =>
        void api.log(
          `speech: default=${report.defaultEngine || "none"} voice=${report.engine || "none"} ` +
            `locale=${report.locale || "none"} chinese=${report.chineseInstalled}` +
            `/${report.chineseVoices} installed (${report.chineseAvailable || "unknown"}) ` +
            `network=${report.networkRequired}`,
        ),
      )
      .catch(() => {
        // Nothing to report on platforms whose synthesiser Rust talks to itself.
      });

    /**
     * Ask whether a microphone is usable.
     *
     * Deliberately repeatable rather than a one-off, however much it looks like
     * it should not change while the app runs. On Android the permission is
     * granted through a dialog that is dismissed *after* this first runs, so the
     * first answer there is always "no" — and a control disabled on that answer
     * would stay disabled for ever. It is asked again when the dialog is
     * answered (see `__hanziPermissionsChanged`, which `MainActivity` calls) and
     * whenever the app comes back to the foreground, which is what catching a
     * permission granted from the system settings looks like.
     */
    const refreshMicrophone = () => {
      void api
        .microphoneStatus()
        .then((status) => {
          microphone = status;
          void api.log(`microphone: ${status.detail}`);
        })
        .catch((cause) => {
          microphone = null;
          void api.log(`could not ask about the microphone: ${cause}`);
        });
    };
    refreshMicrophone();

    const onVisibility = () => {
      if (!document.hidden) {
        refreshMicrophone();
        void autoSync("foreground");
      }
    };
    document.addEventListener("visibilitychange", onVisibility);
    // The desktop's version of the same thing. A phone hides the page when the app
    // goes away and shows it again on return, so `visibilitychange` is the whole
    // story there; a window that is merely not focused still fires nothing, which is
    // why both are listened for.
    const onFocus = () => void autoSync("foreground");
    window.addEventListener("focus", onFocus);

    /**
     * The permission dialog has been answered.
     *
     * Called from `MainActivity.onRequestPermissionsResult`, because the answer
     * the page was given at startup was necessarily given too early.
     */
    const platformHooks = window as unknown as {
      __hanziPermissionsChanged?: () => boolean;
    };
    platformHooks.__hanziPermissionsChanged = () => {
      refreshMicrophone();
      return true;
    };

    // And last, the one thing here that goes to the network. Deliberately after
    // everything the learner is looking at has been asked for: a launch sync must
    // never be something the course waits behind.
    void autoSync("launch");

    // A character answered badly comes back within the minute, so "due" is not
    // a one-off computed at startup: give the badge a slow heartbeat.
    const reviewTimer = setInterval(() => {
      void refreshProgress();
      void refreshReview();
    }, 60_000);

    const onKey = (event: KeyboardEvent) => {
      // The vocabulary screen has text fields; never steal their keystrokes.
      const target = event.target as HTMLElement | null;
      if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;

      const meta = event.metaKey || event.ctrlKey;
      if (event.key === "Enter") {
        event.preventDefault();
        if (report) advance();
        else void check();
      } else if (event.key === "Backspace" || (meta && event.key.toLowerCase() === "z")) {
        event.preventDefault();
        undo();
      } else if (event.key === "ArrowRight") {
        navigate(1);
      } else if (event.key === "ArrowLeft") {
        navigate(-1);
      } else if (event.key.toLowerCase() === "s") {
        toggleStrokeOrder();
      } else if (event.key.toLowerCase() === "h") {
        void hear();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("error", onError);
      window.removeEventListener("unhandledrejection", onRejection);
      window.removeEventListener("resize", publishInsets);
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisibility);
      clearTimeout(noticeTimer);
      clearTimeout(workingTimer);
      delete platformHooks.__hanziPermissionsChanged;
      delete (window as unknown as { __hanziHandleBack?: () => boolean }).__hanziHandleBack;
      clearInterval(reviewTimer);
      clearTimeout(cursorTimer);
    };
  });

  // Follow the cursor: load whichever character it now points at. All sources
  // funnel through here, so the course, the vocabulary list and a review
  // session share one path.
  $effect(() => {
    const next = targetChar;
    if (!next || character?.ch === next) return;
    void loadCharacter(next, charCursor);
  });

  // Follow what is being practised: ask which tones it should be judged against.
  // Both the pending result and any recording in flight belong to the *previous*
  // text, so they are dropped rather than shown against the new one — a tone
  // score next to the wrong word is worse than no score.
  $effect(() => {
    const text = toneText;
    toneResult = null;
    toneError = null;
    if (!text) {
      toneTarget = null;
      return;
    }
    let current = true;
    void api
      .toneTarget(text)
      .then((target) => {
        if (current) toneTarget = target;
        void api.log(
          target
            ? `tone target ${text}: ${target.syllables.map((s) => s.spoken).join("+")}` +
                (target.sandhiApplied ? " (after sandhi)" : "")
            : `tone target ${text}: not scorable`,
        );
      })
      .catch(() => {
        if (current) toneTarget = null;
      });
    return () => {
      current = false;
    };
  });

  /**
   * Press to talk.
   *
   * The device is opened on press, before the learner has started speaking, so
   * the recording includes the whole syllable rather than losing its onset to
   * the device's start-up. A press that never releases is capped in Rust.
   */
  async function startListening() {
    if (toneTarget === null || listening || toneBusy) return;
    toneError = null;
    // The previous judgement is deliberately *left on screen*. Clearing it here
    // removed the tone panel from the document, the page got shorter, and every
    // control below it — including the button under the learner's finger — moved
    // up. That is what made holding the button look like the interface jumping
    // and the recording stopping. The new judgement replaces it when it arrives.
    scoredText = toneText;
    toneBusy = true;
    try {
      await api.listenStart();
      listening = true;
    } catch (cause) {
      toneError = String(cause);
    } finally {
      toneBusy = false;
    }
  }

  /**
   * Release to be judged.
   *
   * `scoredText` is captured on press, so the judgement is against the text that
   * was on screen when the learner started speaking rather than whatever the
   * board has moved on to.
   */
  async function stopListening() {
    if (!listening) return;
    listening = false;
    toneBusy = true;
    // The text is frozen for the duration of the attempt: the Rust side derives
    // the tones from it, so what is scored is what was on screen when the learner
    // pressed, even if the board advanced while they were speaking.
    const text = scoredText;
    try {
      toneResult = await api.listenStop(text);
    } catch (cause) {
      toneError = String(cause);
    } finally {
      toneBusy = false;
    }
  }

  /**
   * Remember where in the course the reader is.
   *
   * Debounced, so holding an arrow key writes once at the end rather than on
   * every step. The write is deliberately not fed back into `index`: the
   * position is the interface's, the file only remembers it.
   */
  $effect(() => {
    const position = index;
    if (!cursorRestored || position === cursorSaved) return;
    clearTimeout(cursorTimer);
    cursorTimer = setTimeout(() => {
      cursorSaved = position;
      void api
        .setCourseCursor(position)
        .then((saved) => {
          cursorWarning = saved.warning;
        })
        .catch((cause) => {
          cursorWarning = `Could not remember your place in the course: ${cause}`;
        });
    }, 400);
    return () => clearTimeout(cursorTimer);
  });

  /** Open where the reader left off, or at the beginning on a first run. */
  async function restoreCursor() {
    try {
      const saved = await api.courseCursor();
      cursorWarning = saved.warning;
      const last = Math.max(0, allCharacters.length - 1);
      const position = Math.min(Math.max(0, saved.index), last);
      index = position;
      cursorSaved = position;
      if (position > 0) {
        void api.log(`course cursor: resuming at character ${position + 1}`);
      }
    } catch (cause) {
      // Not fatal: starting at the beginning is a perfectly good fallback.
      cursorWarning = `Could not restore your place in the course: ${cause}`;
    } finally {
      cursorRestored = true;
    }
  }

  /**
   * Read the learner's settings, and say what they resolved to.
   *
   * The log line is the same kind of evidence as `review queue`: it says whether
   * the board is following the device or a stored choice, which is the first
   * thing worth knowing when the input mode surprises someone.
   */
  async function refreshSettings() {
    try {
      const loaded = await api.settings();
      settings = loaded;
      if (loaded.warning) void api.log(`settings warning: ${loaded.warning}`);
      void api.log(
        `settings: click to draw ${clickToDraw ? "on" : "off"} ` +
          `(${loaded.clickToDraw === null ? "this device's default" : "chosen"}), ` +
          `pace ${loaded.animationPace}, board ${loaded.boardSize}, ` +
          `voice ${loaded.voice ?? "automatic"}`,
      );
    } catch (cause) {
      error = `Could not load your settings: ${cause}`;
    }
  }

  /**
   * Read the voices this machine offers, for the settings screen.
   *
   * Loading it at startup as well as on demand is deliberate: the resolved voice
   * is what `voice` (the practice screen's state) needs, and the settings screen
   * should not open on a spinner.
   */
  async function refreshVoices() {
    try {
      voices = await api.voices();
      // `active` is what the backend actually resolves a choice to, which is how
      // a preference naming a missing voice is told apart from one in use.
      voice = voices.active;
    } catch (cause) {
      void api.log(`could not list the voices: ${cause}`);
      // Leave `voice` as the speech status said; the list is only for the
      // settings screen.
    } finally {
      voicesLoading = false;
    }
  }

  /**
   * Change preferences, and remember them.
   *
   * The controls move immediately and the backend is told after, because a
   * preference that waits for a round trip feels broken; if the save fails the
   * warning comes back and the settings screen shows it, which is the same
   * bargain the study stores make. Only what changed is sent, so the backend
   * leaves the other preferences alone.
   */
  async function updateSettings(patch: SettingsPatch) {
    const previous = settings;
    settings = { ...settings, ...patch, warning: null };
    try {
      const saved = await api.updateSettings(patch);
      settings = saved;
      if (saved.warning) void api.log(`settings warning: ${saved.warning}`);
      // A voice change is the one preference the rest of the app can see: the
      // button's tooltip names the voice and `voice === null` disables it.
      if (patch.voice !== undefined) void refreshVoices();
    } catch (cause) {
      settings = previous;
      error = `Could not save your setting: ${cause}`;
    }
  }

  /** Forget the click-to-draw choice, so the device decides again. */
  async function clearClickToDraw() {
    const previous = settings;
    settings = { ...settings, clickToDraw: null, warning: null };
    try {
      settings = await api.clearClickToDraw();
    } catch (cause) {
      settings = previous;
      error = `Could not save your setting: ${cause}`;
    }
  }

  async function loadCharacter(ch: string, slot: number) {
    try {
      const next = await api.getCharacter(ch);
      character = next;
      reset();
      // A character that has already been written comes back as it was, which is
      // what makes the boxes under the board useful for reviewing rather than
      // only for looking at.
      restoreSlot(slot);
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
    sweep = null;
    playToken += 1;
    // Don't let the previous character keep talking over the next one.
    void api.stopSpeaking();
  }

  /**
   * What has been written for one character of the current entry.
   *
   * The character on the board is answered from the live strokes rather than
   * from its recorded slot, so the box under the board follows the pen instead
   * of lagging a character behind. A slot is written only when the board leaves
   * it, in [`saveSlot`].
   */
  function slotAt(i: number): SlotState {
    if (i === charCursor) return { strokes, report };
    return wordSlots[i] ?? EMPTY_SLOT;
  }

  /**
   * Tooltip for one of the boxes under the board.
   *
   * The character itself is only named once the answer is visible: in recall
   * mode a tooltip is as good a way as any of giving the word away.
   */
  function slotTitle(ch: string, i: number, slot: SlotState): string {
    const where = `Character ${i + 1} of ${entryCharacters.length}`;
    const named = answerVisible ? ` (${ch})` : "";
    if (slot.report) {
      return `${where}${named} — wrote ${Math.round(slot.report.overall)}/100. Click to look at it again.`;
    }
    if (slot.strokes.length > 0) {
      return `${where}${named} — drawn, not checked yet. Click to go back to it.`;
    }
    return `${where} — not written yet. Click to write it.`;
  }

  /** Keep the board's attempt for the character it is about to leave. */
  function saveSlot() {
    if (source === "course" || charCursor >= entryCharacters.length) return;
    const slots = [...wordSlots];
    while (slots.length < entryCharacters.length) {
      slots.push(EMPTY_SLOT);
    }
    slots[charCursor] = { strokes: [...strokes], report };
    wordSlots = slots;
  }

  /** Put a character's recorded attempt back on the board. */
  function restoreSlot(i: number) {
    if (source === "course") return;
    const slot = wordSlots[i];
    if (!slot) return;
    strokes = slot.strokes;
    report = slot.report;
  }

  /**
   * Put another character of the current entry on the board.
   *
   * This is what the boxes under the board click into. A character that is still
   * to be written arrives blank; one that has been written comes back with its
   * drawing and its grade, so it can be looked at or improved — clearing the
   * board and writing again replaces its score rather than adding a second one.
   */
  function selectSlot(i: number) {
    if (source === "course" || i < 0 || i >= entryCharacters.length || i === charCursor) {
      return;
    }
    saveSlot();
    charCursor = i;
    // Moving to a different glyph reloads the character, and the load restores
    // this slot. A word that repeats a character (是不是) does not, so the board
    // is swapped here instead rather than leaving the previous glyph's strokes
    // underneath a new box.
    if (character?.ch === entryCharacters[i]) {
      reset();
      restoreSlot(i);
    }
  }

  /** Begin a fresh entry: no character written, nothing recorded. */
  function beginEntry() {
    charCursor = 0;
    wordSlots = [];
    sessionDone = false;
    reset();
  }

  /**
   * Pronounce what is being practised.
   *
   * The character itself is spoken rather than its pinyin: the system voice has
   * a Chinese lexicon, so 汉 is read correctly, whereas `hàn` would be guessed
   * at. A vocabulary entry is spoken whole, so a word is heard as a word — which
   * also gives the synthesiser the context to pick the right reading for a
   * polyphonic character. This is safe to offer in recall mode: hearing the
   * sound does not give away how to write the glyph.
   */
  async function hear() {
    if (voice === null) return;
    const spoken = currentItem?.text ?? character?.ch;
    if (!spoken) return;
    try {
      await api.speak(spoken);
      void api.log(`spoke ${spoken}`);
    } catch (cause) {
      error = `Could not pronounce ${spoken}: ${cause}`;
    }
  }

  function addStroke(stroke: Point[]) {
    // A learner who starts writing has stopped watching: end the animation
    // rather than drawing over a pen that is still travelling.
    stopStrokeOrder();
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

  /**
   * Flip between tracing the guide and writing from memory.
   *
   * The two are opposites and the control row has one button for them, so this
   * is the same state change the pair of segments used to make. It is spelled
   * out rather than inlined into the click handler so the tooltip, the
   * accessible name and the click cannot disagree about which way round it is.
   */
  function toggleMode() {
    switchMode(mode === "trace" ? "recall" : "trace");
  }

  async function check() {
    if (!character || strokes.length === 0 || grading) return;
    // A report is about the board as it stands, so the guide stops growing the
    // moment it is asked for.
    stopStrokeOrder();
    grading = true;
    error = null;
    try {
      const graded = await api.gradeAttempt(character.ch, strokes, {
        resampleK: 16,
        minStrokeLen: 12,
        // Tracing a visible guide is graded where the guide actually is;
        // writing from memory should not be punished for sitting slightly off.
        globalFit: mode === "recall",
        // The width the board painted the strokes with, so the grader's ink
        // measure compares like with like rather than guessing.
        inkWidth: INK_WIDTH,
      });
      report = graded;
      void api.log(
        `graded ${character.ch}: ${Math.round(graded.overall)}/100, ` +
          `ink=${graded.inkScore.toFixed(2)}/${graded.inkCoverage.toFixed(2)}, ` +
          `legible=${graded.legible}, order=${graded.orderCorrect}`,
      );
      await recordProgress(character.ch, graded.overall);
    } catch (cause) {
      error = `Grading failed: ${cause}`;
    } finally {
      grading = false;
    }
  }

  // ---- progress and review -------------------------------------------------

  function applyProgress(next: ProgressView) {
    progress = next;
    if (next.warning) void api.log(`progress warning: ${next.warning}`);
  }

  async function refreshProgress() {
    try {
      const loaded = await api.progress();
      applyProgress(loaded);
      void api.log(
        `progress: ${loaded.cards.length} practised, ` +
          `${loaded.cards.filter((card) => card.dueNow).length} due`,
      );
    } catch (cause) {
      error = `Could not load your practice progress: ${cause}`;
    }
  }

  async function refreshReview() {
    try {
      review = await api.reviewQueue();
      if (review.warning) void api.log(`review warning: ${review.warning}`);
      void api.log(
        `review queue: ${review.dueCount} due, ${review.items.length} in this session`,
      );
    } catch (cause) {
      error = `Could not work out what is due for review: ${cause}`;
    }
  }

  /**
   * Record a graded character in the practice schedule.
   *
   * All four sources funnel through here: a character met in a lesson and the
   * same character met inside a word are the same thing to learn, so they share
   * one card and one due date.
   */
  async function recordProgress(ch: string, score: number) {
    try {
      const next = await api.recordProgress(ch, score);
      applyProgress(next);
      const card = next.cards.find((candidate) => candidate.ch === ch);
      void api.log(
        `progress ${ch}: ${Math.round(score)}/100, ${card ? `due ${card.due}` : "unscheduled"}`,
      );
      // Answering something changes what is due, so the queue is refreshed
      // rather than left stale until the next launch.
      await refreshReview();
    } catch (cause) {
      // Grading itself already succeeded; say only why the history did not.
      error = `Could not record your progress for ${ch}: ${cause}`;
    }
  }

  /** Milliseconds of pen travel per design unit of centre-line. */
  const MS_PER_UNIT = 1.0;
  /** Bounds on one stroke, so a 点 is not a blink and a 捺 is not a wait. */
  const MIN_STROKE_MS = 100;
  const MAX_STROKE_MS = 520;
  /** Pause between strokes, so the lift and the next pen-down are visible. */
  const BETWEEN_STROKES_MS = 110;
  /** However many strokes a character has, animating it takes at most this. */
  const MAX_TOTAL_MS = 7000;

  /**
   * How long each stroke takes, and the pause after it.
   *
   * Built as a whole timeline rather than a per-stroke delay because the total
   * has to be bounded: a length-proportional pace alone would animate 囊 (22
   * strokes) for far longer than anyone will watch. Everything is scaled by the
   * same factor once the natural total is known, so the character keeps its
   * rhythm — the long strokes still take longer than the short ones.
   *
   * The settings screen's pace is applied to the *bounds* rather than to the
   * result, so `Slow` really is half speed even for a character whose every
   * stroke is already at `MAX_STROKE_MS`: scaling afterwards would be capped
   * away by that same bound and change nothing for exactly the characters a
   * learner most wants slowed down.
   */
  function strokeTimeline(
    medians: Point[][],
    total: number,
  ): { duration: number; pause: number }[] {
    const scale = paceScale;
    const steps = Array.from({ length: total }, (_, i) => ({
      duration: Math.min(
        MAX_STROKE_MS * scale,
        Math.max(MIN_STROKE_MS * scale, polylineLength(medians[i] ?? []) * MS_PER_UNIT * scale),
      ),
      pause: BETWEEN_STROKES_MS * scale,
    }));
    const natural = steps.reduce((sum, step) => sum + step.duration + step.pause, 0);
    const cap = MAX_TOTAL_MS * scale;
    const squeeze = natural > cap ? cap / natural : 1;
    return steps.map((step) => ({
      duration: step.duration * squeeze,
      pause: step.pause * squeeze,
    }));
  }

  /**
   * Animate the character being written, one stroke at a time.
   *
   * A pen walks along each stroke's centre-line and the stroke's outline is
   * revealed behind it, so *how* each stroke is drawn is visible rather than
   * only *which* strokes there are. The pen's pace is the stroke's own length,
   * so a long sweeping stroke takes longer than a 点, and the whole character is
   * squeezed into [`MAX_TOTAL_MS`] however many strokes it has — 囊 has 22 and
   * should not take half a minute.
   *
   * The loop is driven by animation frames rather than a timer, and every await
   * is followed by a check of `playToken`: that token is the only cancellation
   * mechanism, so navigating away, stopping the animation or resetting the board
   * ends it at the next frame with nothing left running.
   */
  async function playStrokeOrder() {
    const total = strokeTotal;
    if (total === 0 || playing) return;
    const token = ++playToken;
    const timeline = strokeTimeline(character?.medians ?? [], total);
    playing = true;
    revealed = 0;
    sweep = null;
    for (let i = 0; i < total; i++) {
      const { duration, pause } = timeline[i];
      const started = performance.now();
      revealed = i;
      for (;;) {
        const elapsed = (performance.now() - started) / duration;
        const progress = Math.min(1, elapsed);
        sweep = { index: i, progress };
        if (progress >= 1) break;
        await nextFrame();
        if (token !== playToken) return;
      }
      // The pen lifts: the stroke is whole now, so it is filled rather than
      // being left half-swept under a pen that has moved on.
      sweep = null;
      revealed = i + 1;
      if (pause > 0) {
        await sleep(pause);
        if (token !== playToken) return;
      }
    }
    playing = false;
  }

  /** Stop the animation where it stands, at the end of the current stroke. */
  function stopStrokeOrder() {
    if (!playing) return;
    // A half-swept stroke with no pen on it reads as a rendering fault, so a
    // stop finishes the stroke the pen is on rather than freezing mid-stroke.
    if (sweep) revealed = sweep.index + 1;
    sweep = null;
    playing = false;
    playToken += 1;
  }

  function toggleStrokeOrder() {
    if (playing) stopStrokeOrder();
    else void playStrokeOrder();
  }

  // ---- personal vocabulary list -------------------------------------------

  /** Adopt a view returned by a command. */
  function applyVocab(next: VocabView) {
    vocab = next;
    if (next.warning) void api.log(`vocabulary warning: ${next.warning}`);
  }

  /**
   * Re-read everything a sync can have changed.
   *
   * The backend reloads its stores after a sync — see `AppState::reload_after_sync`
   * — but this side holds its own copies of them and only replaces them when a
   * command hands over a new view. Without this the sync succeeded and the screen
   * went on showing what it had, which is what "the list only updates after a
   * restart" was: the restart was the next thing that called these.
   *
   * All three, because one sync does all three: it rebuilds schedules, settles the
   * vocabulary list, and moves the course position. Refreshing only the list would
   * have replaced one half-fixed bug with another.
   */
  function refreshAfterSync() {
    void refreshVocabulary();
    void refreshProgress();
    void refreshReview();
  }

  async function refreshVocabulary() {
    try {
      const loaded = await api.vocabulary();
      applyVocab(loaded);
      void api.log(
        `vocabulary: ${loaded.entries.length} entries in ${loaded.groups.length} groups`,
      );
    } catch (cause) {
      error = `Could not load your vocabulary list: ${cause}`;
    }
  }

  /**
   * Run a vocabulary command.
   *
   * A failure is reported but never clears the list: the store refuses to save
   * over a file it could not read, and that must stay visible rather than
   * looking like the entries were lost.
   */
  async function withVocab(
    action: () => Promise<VocabView>,
    note?: string,
    after?: (view: VocabView) => void,
  ) {
    if (vocabBusy) return;
    vocabBusy = true;
    error = null;
    try {
      const next = await action();
      applyVocab(next);
      if (note) statusMessage = note;
      after?.(next);
    } catch (cause) {
      error = String(cause);
    } finally {
      vocabBusy = false;
    }
  }

  function addToVocabulary(
    text: string,
    pinyin: string,
    meaning: string,
    group: string | null,
  ) {
    void withVocab(
      () => api.vocabAdd(text, pinyin, meaning, group),
      `Added ${text} to your list`,
    );
  }

  /**
   * Capture whatever character is on the board.
   *
   * This is the main way entries get in: meet a new character in a lesson, add
   * it in one click, then drill the list later.
   */
  function addCurrentToVocabulary() {
    if (!character) return;
    const group = vocabSelection && vocabSelection !== "" ? vocabSelection : null;
    addToVocabulary(
      character.ch,
      character.pinyin.join(" / "),
      character.definition,
      group,
    );
  }

  /**
   * Resolve study text for the add form.
   *
   * A single character comes back with a reading and a meaning. A word comes back
   * with its readings composed into a draft pinyin and a per-character breakdown,
   * but no meaning — a word's meaning cannot be derived from its characters, so
   * the learner writes it.
   */
  function lookupText(text: string) {
    return api.lookupText(text);
  }

  async function removeGroup(name: string, purge: boolean) {
    if (purge) {
      const agreed = await confirmDialog(
        `Delete every entry in “${name}”? This cannot be undone.`,
        { title: "Delete entries", kind: "warning" },
      );
      if (!agreed) return;
    }
    await withVocab(
      () => api.vocabRemoveGroup(name, purge),
      purge
        ? `Deleted the group “${name}” and its entries`
        : `Unfiled the entries in “${name}”`,
      () => {
        // Do not leave the filter pointing at a group that no longer exists.
        if (vocabSelection === name) vocabSelection = null;
      },
    );
  }

  async function exportVocabulary(format: "json" | "csv") {
    try {
      const extension = format === "json" ? "json" : "csv";
      const path = await pickSavePath({
        title: `Export vocabulary as ${format.toUpperCase()}`,
        defaultPath: `hanzi-vocabulary.${extension}`,
        filters: [{ name: format.toUpperCase(), extensions: [extension] }],
      });
      if (!path) return;
      vocabBusy = true;
      statusMessage = await api.vocabExport(path, format);
      void api.log(statusMessage);
    } catch (cause) {
      error = `Export failed: ${cause}`;
    } finally {
      vocabBusy = false;
    }
  }

  async function importVocabulary(merge: boolean) {
    try {
      const path = await pickFile({
        title: merge ? "Add to your list" : "Replace your list",
        multiple: false,
        directory: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (typeof path !== "string") return;
      vocabBusy = true;
      const outcome = await api.vocabImport(path, merge);
      applyVocab(outcome.view);
      statusMessage = outcome.message;
      void api.log(`vocabulary import: ${outcome.message}`);
    } catch (cause) {
      error = `Import failed: ${cause}`;
    } finally {
      vocabBusy = false;
    }
  }

  // ---- practising the list ------------------------------------------------

  /**
   * Start a session from any source.
   *
   * Everything that decides what the board asks for lives in `targetChar`, so a
   * review session and the vocabulary list differ only in where the queue came
   * from. A review list is for recall: tracing a character you have already
   * been shown teaches nothing.
   */
  function startPractice(items: PracticeItem[], from: Source) {
    queue = items;
    queueCursor = 0;
    source = from;
    mode = "recall";
    beginEntry();
    statusMessage = null;
    wordMessage = null;
  }

  /** Start drilling a snapshot of the given entries. */
  function practiseQueue(entries: VocabEntry[]) {
    if (entries.length === 0) return;
    startPractice(
      entries.map((entry) => ({
        text: entry.text,
        entryId: entry.id,
        pinyin: entry.pinyin,
        meaning: entry.meaning,
      })),
      "vocabulary",
    );
    void api.log(`practising ${entries.length} vocabulary entries`);
  }

  /** Start drilling what is due for review, most overdue first. */
  function startReview() {
    if (review.items.length === 0) return;
    const items: PracticeItem[] = review.items.map((item) => {
      // A due character of a word is practised as that word: the entry carries
      // the reading and the meaning, and a word is written whole.
      const entry =
        item.entryId === null
          ? undefined
          : vocab.entries.find((candidate) => candidate.id === item.entryId);
      return entry
        ? {
            text: entry.text,
            entryId: entry.id,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
          }
        : { text: item.text, entryId: null, pinyin: "", meaning: "" };
    });
    startPractice(items, "review");
    void api.log(
      `reviewing ${items.length} of ${review.dueCount} due ` +
        `${review.dueCount === 1 ? "item" : "items"}`,
    );
  }

  // ---- practising the word dictionary --------------------------------------

  /** Drill words straight from the dictionary, without saving them first. */
  function practiseWords(words: Word[]) {
    if (words.length === 0) return;
    startPractice(
      words.map((word) => ({
        text: word.text,
        entryId: null,
        pinyin: word.pinyin,
        meaning: word.meaning,
      })),
      "words",
    );
    void api.log(`practising ${words.length} dictionary words`);
  }

  /** Put a dictionary word in the personal list, reading and meaning filled in. */
  function addWordToList(word: Word) {
    const group = vocabSelection && vocabSelection !== "" ? vocabSelection : null;
    void withVocab(
      () => api.vocabAdd(word.text, word.pinyin, word.meaning, group),
      undefined,
      () => {
        wordMessage = `Added ${word.text} to your vocabulary list`;
      },
    );
  }

  /** One page of the word list; the panel owns the query and the debounce. */
  function searchWords(query: string, level: number | null): Promise<WordSearchView> {
    return api.searchWords(query, level);
  }

  /** Return to the course sequence. */
  function leavePractice() {
    source = "course";
    queue = [];
    queueCursor = 0;
    beginEntry();
  }

  /** Move within the practice queue, abandoning an unfinished entry. */
  function moveQueue(delta: number) {
    const next = queueCursor + delta;
    if (next < 0 || next >= queue.length) return;
    queueCursor = next;
    beginEntry();
  }

  /**
   * Record the graded character and move on.
   *
   * A word is written one character at a time and scored by the mean of its
   * characters, so a long word cannot be credited on one good stroke. Each
   * character already has its own card by the time this runs; the mean is what
   * the vocabulary entry itself remembers.
   *
   * Every character of the entry has to be written before it counts, but not
   * necessarily in order: the boxes under the board can put any character on the
   * board, and this moves to whichever ones are still outstanding.
   */
  async function finishCharacter() {
    // Once the queue is exhausted the last entry must not be finished again: a
    // second press would credit it with an attempt nobody made.
    if (sessionDone || !currentItem || !report) return;
    saveSlot();

    const outstanding = entryCharacters.findIndex((_, i) => !wordSlots[i]?.report);
    if (outstanding >= 0) {
      selectSlot(outstanding);
      return;
    }

    const scores = entryCharacters.map((_, i) => wordSlots[i]!.report!.overall);
    const mean = scores.reduce((total, value) => total + value, 0) / scores.length;
    const finished = currentItem;
    const entryId = finished.entryId;

    const isLast = queueCursor + 1 >= queue.length;
    if (isLast) {
      charCursor = 0;
      wordSlots = [];
      sessionDone = true;
    } else {
      queueCursor += 1;
      beginEntry();
    }

    const characters = `${scores.length} ${scores.length === 1 ? "character" : "characters"}`;
    const note =
      `${finished.text}: ${Math.round(mean)}/100 recorded (${characters})` +
      (isLast
        ? source === "review"
          ? " — that was everything due"
          : " — that was the last entry in this list"
        : "");

    if (entryId !== null) {
      await withVocab(() => api.vocabRecordAttempt(entryId, mean), note);
    } else {
      statusMessage = note;
    }
    // Answering may have cleared the queue, so the badge is brought up to date.
    await refreshReview();
  }

  // ---- moving around ------------------------------------------------------

  function switchView(next: View) {
    if (next === view) return;
    view = next;
    statusMessage = null;
    wordMessage = null;
    // Leaving for the course abandons a list, word or review session; the
    // course is always there to come back to. Moving between the vocabulary
    // list and the words screen keeps whatever is on the board.
    if (next === "course" && source !== "course") leavePractice();
  }

  /** Arrow keys follow whichever sequence is active. */
  function navigate(delta: number) {
    if (source === "course") move(delta);
    else moveQueue(delta);
  }

  /** Move on after a graded attempt. */
  function advance() {
    if (source === "course") move(1);
    else void finishCharacter();
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

  /** Stop drilling and go back to what was being drilled. */
  function stopPractising() {
    const target: View = source === "words" ? "words" : "vocabulary";
    leavePractice();
    view = target;
    statusMessage = null;
    wordMessage = null;
  }

  /** Stop reviewing and show what is left of the queue. */
  function stopReview() {
    leavePractice();
    void refreshReview();
  }
</script>

<!--
  ← n / N →. One widget in two places: on a phone it is the top bar's right-hand
  corner, and on a wide screen it is the header beside the board, where there is
  room for it next to the meaning. It was drawn twice, once for each branch of
  the header, which is one copy more than there is behaviour to go round.
-->
{#snippet characterNav()}
  <div class="nav">
    <button onclick={nav.back} disabled={nav.first} aria-label="Previous">←</button>
    <span>{nav.position} / {nav.total}</span>
    <button onclick={nav.forward} disabled={nav.last} aria-label="Next">→</button>
  </div>
{/snippet}

<!--
  The phone's fold for the character's details. Folded, the meaning keeps its
  first two lines and the strokes, radical, level, frequency, progress and
  etymology are one tap away; a wide screen has room for all of it and does not
  draw this at all.
-->
{#snippet detailFold()}
  <button
    class="fold"
    type="button"
    aria-expanded={showDetails}
    aria-controls="character-details"
    onclick={() => (showDetails = !showDetails)}
  >
    {showDetails ? "Less" : "More"}
    <span class="chevron" class:open={showDetails} aria-hidden="true">›</span>
  </button>
{/snippet}

<div class="app">
  <!--
    The sidebar is a column on a wide screen and a sheet over the board on a
    phone; the scrim is what closes it, and both only exist below the phone
    breakpoint in the styles below.
  -->
  {#if navOpen}
    <button
      class="scrim"
      type="button"
      aria-label="Close navigation"
      onclick={() => (navOpen = false)}
    ></button>
  {/if}
  <div class="nav-pane" class:open={navOpen}>
    <LessonSidebar
      view={view}
      onSwitchView={navigating(switchView)}
      {lessons}
      {activeLesson}
      activeCharacter={courseChar}
      {summary}
      onSelectLesson={navigating(goToLesson)}
      onSelectCharacter={navigating(goToCharacter)}
      progressCards={progress.cards}
      {review}
      reviewing={source === "review"}
      onStartReview={navigating(startReview)}
      vocabGroups={vocab.groups}
      vocabEntries={vocab.entries}
      vocabSelection={vocabSelection}
      onSelectVocabGroup={navigating((selection: string | null) => (vocabSelection = selection))}
      wordLevels={stats?.wordLevels ?? []}
      wordsTotal={stats?.words ?? 0}
      {wordLevel}
      onSelectWordLevel={navigating((level: number | null) => (wordLevel = level))}
      onShowSettings={navigating(() => switchView("settings"))}
      onShowLicences={navigating(() => switchView("about"))}
    />
  </div>

  <main>
    <!-- Phone only: the way back to the navigation the sheet hides. -->
    <div class="topbar">
      <button
        class="nav-toggle"
        type="button"
        aria-label="Open navigation"
        onclick={() => (navOpen = true)}
      >
        <span aria-hidden="true">☰</span>
      </button>
      <span class="topbar-title">{appInfo?.name ?? "Hanzi Tutor"}</span>
      <!-- The character used to be shown here, and in recall mode it gave the
           answer away the moment the mode was chosen — the one thing the mode
           exists to withhold. What belongs in this corner is the way to the next
           character instead, which is also what frees the header below to give
           the whole width to the meaning. -->
      {#if practising}
        {@render characterNav()}
      {/if}
    </div>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    {#if syncNotice}
      <!-- `role="status"` rather than an alert: this is the app saying what it is
           doing, and it should be announced once without stealing focus from the
           board. -->
      <p
        class="sync-note"
        class:trouble={syncNotice.trouble}
        role="status"
        aria-live="polite"
      >
        {#if syncNotice.working}
          <span class="sync-pulse" aria-hidden="true"></span>
        {/if}
        {syncNotice.text}
      </p>
    {/if}

    {#if cursorWarning}
      <p class="warning">
        {cursorWarning} Meanwhile the course starts from the beginning.
      </p>
    {/if}

    {#if progress.warning}
      <p class="warning">
        {progress.warning} Practice still works; your history is just not being
        written to disk.
      </p>
    {/if}

    {#if loading}
      <p class="status">Loading the character set…</p>
    {:else if view === "vocabulary" && source !== "vocabulary"}
      <VocabularyPanel
        view={vocab}
        selection={vocabSelection}
        message={statusMessage}
        busy={vocabBusy}
        lookup={lookupText}
        onAdd={addToVocabulary}
        onUpdate={(id, pinyin, meaning, group) =>
          void withVocab(() => api.vocabUpdate(id, pinyin, meaning, group), "Saved")}
        onRemove={(id) =>
          void withVocab(() => api.vocabRemove(id), "Removed from your list")}
        onAddGroup={(name) =>
          void withVocab(() => api.vocabAddGroup(name), `Created the group “${name}”`)}
        onRenameGroup={(from, to) =>
          void withVocab(
            () => api.vocabRenameGroup(from, to),
            `Renamed “${from}” to “${to}”`,
          )}
        onRemoveGroup={removeGroup}
        onPractise={practiseQueue}
        onExport={(format) => void exportVocabulary(format)}
        onImport={(merge) => void importVocabulary(merge)}
      />
    {:else if view === "words" && source !== "words"}
      <WordsPanel
        bind:query={wordQuery}
        search={searchWords}
        level={wordLevel}
        message={wordMessage}
        busy={vocabBusy}
        onPractise={practiseWords}
        onAddToList={addWordToList}
      />
    {:else if view === "settings"}
      <SettingsPanel
        {settings}
        {deviceWantsClickToDraw}
        {voices}
        {voicesLoading}
        onChange={(patch) => void updateSettings(patch)}
        onClearClickToDraw={() => void clearClickToDraw()}
        onSynced={refreshAfterSync}
        {syncPulse}
      />
    {:else if view === "about"}
      <LicencesPanel info={appInfo} notices={licenceList} error={licenceError} />
    {:else if !character}
      <!-- `practising` above is this condition and the four screens before it
           written as one; the phone's top bar navigates by that, so the two have
           to move together. -->
      <p class="status">No character selected.</p>
    {:else}
      <header class="meta" class:open={showDetails}>
        {#if currentItem}
          <!-- The whole entry, not only the character the board is asking for. A
               word is practised one character at a time, so the box used to show
               just that character and the learner could not see the word they
               were part of. Now every character is shown and the one being
               written is picked out — which is also the answer to "where am I in
               this word", the question the box was quietly answering before.
               `--chars` drives the size, so a long entry shrinks to stay in the
               header instead of pushing the reading off the screen. -->
          <div
            class="glyph"
            lang="zh-Hans"
            class:masked={!answerVisible}
            style="--chars: {Math.max(entryCharacters.length, 1)}"
          >
            {#if !answerVisible}
              ?
            {:else if entryCharacters.length > 1}
              {#each entryCharacters as ch, i (i)}
                <span class="glyph-char" class:current={i === charCursor}>{ch}</span>
              {/each}
            {:else}
              {entryCharacters[charCursor] ?? "?"}
            {/if}
          </div>
          <div class="detail">
            <p class="pinyin">{currentItem.pinyin || character.pinyin.join("  ·  ") || "—"}</p>
            <p class="meaning">{currentItem.meaning || character.definition || "—"}</p>
            <div class="details" id="character-details">
              <ul class="facts">
                {#if entryCharacters.length > 1}
                  <li>character {charCursor + 1} of {entryCharacters.length}</li>
                {/if}
                <li>
                  {source === "review"
                    ? "review"
                    : source === "words"
                      ? "word"
                      : "entry"} {queueCursor + 1} of {queue.length}
                </li>
                <li>{strokeTotal} {strokeTotal === 1 ? "stroke" : "strokes"}</li>
                {#if activeCard}
                  <li>
                    practised {activeCard.attempts}× · best {Math.round(
                      activeCard.bestScore ?? 0,
                    )}
                  </li>
                  <li class:due={activeCard.dueNow}>
                    {activeCard.dueNow ? "due for review" : `next review ${dueLabel(activeCard.due)}`}
                  </li>
                {:else}
                  <li>new character</li>
                {/if}
              </ul>
              {#if answerVisible}
                <p class="etymology" lang="zh-Hans">{currentItem.text}</p>
              {/if}
            </div>
            {@render detailFold()}
          </div>
        {:else}
          <div class="glyph" lang="zh-Hans" class:masked={!answerVisible}>
            {answerVisible ? character.ch : "?"}
          </div>
          <div class="detail">
            <p class="pinyin">{character.pinyin.join("  ·  ") || "—"}</p>
            <p class="meaning">{character.definition || "—"}</p>
            <div class="details" id="character-details">
              <ul class="facts">
                <li>{strokeTotal} {strokeTotal === 1 ? "stroke" : "strokes"}</li>
                {#if character.radical && character.radical !== "\u0000"}
                  <li>radical <span lang="zh-Hans">{character.radical}</span></li>
                {/if}
                {#if character.hsk > 0}<li>HSK {character.hsk}</li>{/if}
                {#if character.rank > 0}<li>frequency #{character.rank}</li>{/if}
                {#if activeCard}
                  <li>
                    practised {activeCard.attempts}× · best {Math.round(
                      activeCard.bestScore ?? 0,
                    )}
                  </li>
                  <li class:due={activeCard.dueNow}>
                    {activeCard.dueNow ? "due for review" : `next review ${dueLabel(activeCard.due)}`}
                  </li>
                {:else}
                  <li>not practised yet</li>
                {/if}
              </ul>
              {#if answerVisible && character.etymology}
                <p class="etymology">{character.etymology}</p>
              {/if}
            </div>
            {@render detailFold()}
          </div>
        {/if}

        <!-- Adding the character to your list is a fact about the character, not
             about the board, so the button belongs on the row that names the
             character rather than down among the drawing controls. It is drawn
             only when the character came from the course: from your own list, or
             from a review of it, it is already in there. -->
        {#if source === "course"}
          <button
            class="icon"
            onclick={addCurrentToVocabulary}
            disabled={vocabBusy}
            aria-label="Add this character to your vocabulary list"
            title="Add this character to your vocabulary list"
          >
            <Icon name="plus" />
          </button>
        {/if}
        {@render characterNav()}
      </header>

      <div class="workspace">
        <section class="stage">
          <PracticeCanvas
            {character}
            {strokes}
            {report}
            {ghostCount}
            {ghostStyle}
            {showCorrections}
            {sweep}
            {clickToDraw}
            {boardFraction}
            onStroke={addStroke}
          />

          {#if source !== "course" && entryCharacters.length > 1}
            <div class="slots" role="group" aria-label="Characters in this entry">
              {#each entryCharacters as ch, i (i)}
                {@const slot = slotAt(i)}
                <button
                  type="button"
                  class="slot"
                  class:current={i === charCursor}
                  class:graded={slot.report !== null}
                  aria-current={i === charCursor ? "true" : undefined}
                  title={slotTitle(ch, i, slot)}
                  onclick={() => selectSlot(i)}
                >
                  {#if slot.strokes.length > 0}
                    <CharacterThumb strokes={slot.strokes} report={slot.report} />
                  {:else if answerVisible}
                    <span class="slot-glyph" lang="zh-Hans">{ch}</span>
                  {/if}
                  {#if slot.report}
                    <span class="slot-score">{Math.round(slot.report.overall)}</span>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}

          <div class="controls">
            <!-- What the board shows. Trace and recall are opposites, so one
                 toggle says what the two segments it replaces said, in half the
                 width: the eye is the answer in sight, the same eye struck
                 through is the answer hidden. -->
            <div class="cluster" role="group" aria-label="Board view">
              <button
                class="icon"
                class:on={mode === "recall"}
                aria-pressed={mode === "recall"}
                aria-label={mode === "trace"
                  ? "Trace mode: a faint copy of the character is on the board. Switch to recall"
                  : "Recall mode: the character is hidden until you check. Switch to trace"}
                title={mode === "trace"
                  ? "Trace: a faint copy is on the board to follow. Press for recall, which hides the character until you press ✓. Either way the board is cleared."
                  : "Recall: the character is hidden until you press ✓. Press for trace, where a faint copy is on the board. Either way the board is cleared."}
                onclick={toggleMode}
              >
                <Icon name={mode === "trace" ? "eye" : "eye-off"} />
              </button>
            </div>

            <!-- Sound in and sound out: the character read aloud, and the
                 learner reading it back. -->
            <div class="cluster" role="group" aria-label="Pronunciation and speaking">
              <button
                class="icon"
                onclick={hear}
                disabled={!character || voice === null}
                aria-label="Hear this character pronounced"
                title={voice === undefined
                  ? "Looking for a Chinese voice…"
                  : voice === null
                    ? "No Chinese voice is installed, so pronunciation is unavailable"
                    : `Pronounce this character (${voice})`}
              >
                <Icon name="ear" />
              </button>

              <!-- Push to talk. Held, not clicked: the microphone is open only
                   between press and release, so the system's recording
                   indicator is lit only while the learner is deliberately
                   speaking. -->
              <button
                class="icon say"
                class:listening
                onpointerdown={(event) => {
                  event.preventDefault();
                  // Take the pointer, so that every later event for it comes
                  // here wherever the finger travels. Without this the browser
                  // sends `pointerleave` as soon as the button stops being under
                  // the finger — which it did whenever the result panel above
                  // appeared or vanished and moved the row — and the recording
                  // ended the instant it began. Push-to-talk should survive a
                  // finger that slides, which is why there is no `pointerleave`
                  // handler here at all.
                  event.currentTarget.setPointerCapture(event.pointerId);
                  void startListening();
                }}
                onpointerup={(event) => {
                  event.currentTarget.releasePointerCapture(event.pointerId);
                  void stopListening();
                }}
                onpointercancel={() => void stopListening()}
                oncontextmenu={(event) => event.preventDefault()}
                disabled={toneTarget === null || !microphone?.available || toneBusy}
                aria-label={listening
                  ? "Listening — release to score what you said"
                  : (sayBlocked ?? `Hold to say ${toneText}`)}
                title={sayBlocked ?? sayPrompt}
              >
                <Icon name="mouth" />
              </button>
            </div>

            <!-- Ink: watch the character written, take the last stroke back, or
                 start again. The three gestures on the board itself, in the
                 order they are reached for. -->
            <div class="cluster" role="group" aria-label="Writing">
              <button
                class="icon"
                class:on={playing}
                aria-pressed={playing}
                onclick={toggleStrokeOrder}
                disabled={strokeTotal === 0}
                aria-label={playing
                  ? "Stop the stroke-order animation"
                  : "Show the character written stroke by stroke"}
                title={playing
                  ? "Stop the animation where it is"
                  : "Watch the character written, one stroke at a time (S)"}
              >
                <Icon name="stroke-order" />
              </button>
              <button
                class="icon"
                onclick={undo}
                disabled={strokes.length === 0}
                aria-label="Undo the last stroke"
                title="Take the last stroke back (⌫)"
              >
                <Icon name="undo" />
              </button>
              <button
                class="icon"
                onclick={reset}
                disabled={strokes.length === 0}
                aria-label="Clear the board"
                title="Clear every stroke off the board"
              >
                <Icon name="trash" />
              </button>
            </div>

            <!-- Leaving the board. The course has nothing to leave — the way out
                 of the course is the sidebar — so this is drawn only where the
                 practice started from a list of your own. -->
            {#if source !== "course"}
              <div class="cluster">
                {#if source === "vocabulary"}
                  <button
                    class="icon"
                    onclick={stopPractising}
                    aria-label="Back to your vocabulary list"
                    title="Back to your vocabulary list"
                  >
                    <Icon name="back" />
                  </button>
                {:else if source === "words"}
                  <button
                    class="icon"
                    onclick={stopPractising}
                    aria-label="Back to the word list"
                    title="Back to the word list"
                  >
                    <Icon name="back" />
                  </button>
                {:else}
                  <button
                    class="icon"
                    onclick={stopReview}
                    aria-label="Stop this review session"
                    title="Stop reviewing — what you have done so far is kept"
                  >
                    <Icon name="back" />
                  </button>
                {/if}
              </div>
            {/if}

            <!-- How much has been drawn, whether the corrections are marked, and
                 ✓. The first is the state and the last is the action of the same
                 judgement, so all three are one group: they wrap to the next line
                 together rather than leaving the count stranded above the button
                 it belongs to. Whether corrections show sits beside ✓ because
                 that is the moment it starts to matter — everything to its left
                 acts on the attempt before it is graded.
                 The group is also what takes up the slack on the line it lands
                 on, which is why there is no separate spacer: it is what holds
                 the count and ✓ against the right edge, on one line or two. -->
            <div class="cluster commit">
              <span class="count" title={report
                ? `${report.givenStrokes} of ${report.expectedStrokes} strokes were graded; ignored stray marks are not counted`
                : `${strokes.length} marks drawn, ${strokeTotal} strokes expected`}>
                {report ? report.givenStrokes : strokes.length} / {strokeTotal}
              </span>

              <button
                class="icon"
                class:on={showCorrections}
                aria-pressed={showCorrections}
                aria-label="Mark the corrections on the board"
                title={showCorrections
                  ? "The strokes you missed or misplaced are marked on the board, over your own. Press to hide them."
                  : "Corrections are hidden. Press to mark the strokes you missed or misplaced, over your own."}
                onclick={() => (showCorrections = !showCorrections)}
              >
                <Icon name="target" />
              </button>

              {#if report && !sessionDone}
                <button
                  class="primary icon"
                  onclick={advance}
                  aria-label={source === "course"
                    ? "Next character"
                    : charCursor + 1 < entryCharacters.length
                      ? "Next character of this entry"
                      : "Finish this entry"}
                  title={source === "course"
                    ? "Next character (Enter)"
                    : charCursor + 1 < entryCharacters.length
                      ? "Next character of this entry (Enter)"
                      : "Finish this entry (Enter)"}
                >
                  <Icon name="next" />
                </button>
              {:else if !sessionDone}
                <button
                  class="primary icon"
                  onclick={check}
                  disabled={strokes.length === 0 || grading}
                  aria-label={grading ? "Checking your writing…" : "Check your writing"}
                  title={grading
                    ? "Checking your writing…"
                    : "Check your writing against the character (Enter)"}
                >
                  <Icon name="tick" />
                </button>
              {/if}
            </div>
          </div>

          {#if statusMessage}
            <p class="note">{statusMessage}</p>
          {/if}

          <p class="hint">
            {#if voice === null}
              No Chinese voice is installed, so the ear button is disabled. Add
              one in System Settings → Accessibility → Spoken Content → System
              Voice → Manage Voices.
            {:else if sayBlocked !== null && microphone !== undefined}
              {sayBlocked} — so the mouth button, which scores a spoken tone, is
              disabled.
            {:else if source !== "course" && entryCharacters.length > 1}
              Write the word one character at a time. Its score is the average
              across its characters, so each one has to be right.
            {:else if source === "review"}
              This came due for review. Write it from memory, then press ✓ —
              how well you do sets when you see it again.
            {:else if mode === "trace"}
              A faint copy of the character is on the board: trace over it in the
              correct stroke order.
            {:else}
              Write the character from memory, then press ✓. Press S to see the
              stroke order.
            {/if}
          </p>
        </section>

        <aside class="feedback">
          <!-- The handwriting report leads and a scored tone follows it. The two
               are judged independently — a spoken syllable has no strokes to
               report on, and a silent attempt has no pitch — but the report is
               what pressing ✓ is for, and it is read while the learner's own
               strokes are still in front of them. With the tone panel first, the
               stroke-by-stroke verdict was pushed below the fold whenever a tone
               had been scored, which is exactly when comparing your writing with
               the reference matters most. -->
          {#if report}
            <FeedbackPanel {report} />
          {:else}
            <div class="empty">
              <button
                class="help-toggle"
                type="button"
                aria-expanded={showHelp}
                aria-controls="how-it-works"
                onclick={() => (showHelp = !showHelp)}
              >
                <span class="info" aria-hidden="true">ⓘ</span>
                How this works
                <span class="chevron" class:open={showHelp} aria-hidden="true">›</span>
              </button>
              {#if showHelp}
                <div id="how-it-works" class="help-body">
                  <p>
                    Your strokes are compared with the reference on four measures:
                    shape, placement, ink and order — whether each stroke is the
                    right kind of stroke in the right place, whether as much ink
                    went down as the character needs (a stroke traced correctly but
                    drawn far too thin is not legible), and whether they were
                    written in sequence. Writing a legible character in the wrong
                    order is reported as exactly that.
                  </p>
                  <p class="keys">
                    <kbd>Enter</kbd> check · <kbd>S</kbd> stroke order ·
                    <kbd>H</kbd> hear it · <kbd>⌫</kbd> undo ·
                    <kbd>←</kbd> <kbd>→</kbd> move
                  </p>
                </div>
              {/if}
            </div>
          {/if}

          <!-- A tone that was scored, under the report, and the reason one could
               not be, above the panel it belongs to. -->
          {#if toneError}
            <p class="tone-error">{toneError}</p>
          {/if}
          {#if toneResult}
            <TonePanel result={toneResult} />
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
    /* A report that is longer than its column makes the page scroll, and a
       classic scrollbar takes its width out of the page. Reserving that width
       whether or not the page happens to scroll is what keeps the board the same
       size before and after a tone is scored, rather than shrinking it by the
       width of a scrollbar that the report brought with it. */
    scrollbar-gutter: stable;
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
    /* A minimum rather than a fixed width: one character is still the 92px box it
       always was, and a longer entry grows it along the word — up to the cap in
       the font size below, after which the characters shrink instead. That keeps
       a four-character word from being four times as wide as a character, which
       would push the reading and the meaning out of the header. */
    min-width: 92px;
    height: 92px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.04em;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
    font-family: var(--hanzi-font);
    /* One character keeps its size; several share the available width, so the
       whole entry stays legible without the box running away. `--chars` is set on
       the element from the entry's length. */
    font-size: min(3.4rem, calc(230px / var(--chars, 1)));
    line-height: 1;
    color: var(--muted-strong);
  }
  .glyph.masked {
    color: var(--line);
    font-size: 2.6rem;
  }
  /* The character the board is asking for, against the rest of the entry, which
     is context rather than the task. Weight alone would shift the characters
     sideways as the cursor moves, so the difference is colour. */
  .glyph-char {
    color: var(--muted);
  }
  .glyph-char.current {
    color: var(--muted-strong);
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
  /* Due for review now: the one fact worth catching the eye. */
  .facts li.due {
    color: var(--accent-ink);
    font-weight: 650;
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
  /* The character's details fold only where the room is short. A wide screen
     shows the meaning, the facts and the etymology in full and has no button to
     fold them away; everything that makes this a phone control — its size, the
     chevron, and the two rules that do the folding — is in the phone block. */
  .fold {
    display: none;
  }
  .nav button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  /* The character row's own action — adding the character to your list — drawn
     like the navigation buttons it sits beside rather than like the board's
     control row: it belongs to the character, and the row it is on has a
     different scale. The glyph is sized off the font, as everywhere else. */
  .meta button.icon {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    color: var(--muted-strong);
    font-size: 19px;
    cursor: pointer;
  }
  .meta button.icon:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .meta button.icon:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .workspace {
    /* The practice column is given the height the header leaves, and the row is
       *pinned* to it rather than grown to fit what is in it. The feedback column
       is the tallest thing on the row whenever there is a tone report or a
       grading report to show, and a row sized by its contents took the board's
       height from the report: the square was re-fitted to the taller box and
       centred lower in it, and every control below slid down as well. A report
       describes the attempt; it does not get to resize the board that produced
       it. What does not fit in the column now overflows the row and scrolls the
       page, which is where the rest of the report already was. */
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 330px;
    grid-template-rows: minmax(0, 1fr);
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

  /* One box per character of a multi-character entry. A written character shows
     a miniature of the attempt; one still to write is an empty box. */
  .slots {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
  }
  .slot {
    position: relative;
    display: grid;
    place-items: center;
    width: 54px;
    height: 54px;
    padding: 4px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface);
    font: inherit;
    cursor: pointer;
  }
  .slot:hover {
    border-color: var(--accent);
  }
  /* Anything not written yet reads as a slot to fill in. */
  .slot:not(.graded) {
    border-style: dashed;
  }
  .slot.current {
    border-style: solid;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }
  .slot-glyph {
    font-size: 1.6rem;
    line-height: 1;
    /* The same faintness as the guide on the board: a target, not the answer. */
    color: #c3cfdd;
    user-select: none;
  }
  .slot-score {
    position: absolute;
    right: 1px;
    bottom: 1px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-size: 0.62rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  /* A cluster is one flex item, so the row breaks between groups of related
     controls instead of in the middle of one: undo and clear belong on the same
     line, and a bare row of ten buttons would happily separate them. */
  .cluster {
    display: flex;
    align-items: center;
    gap: 6px;
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
  /* An icon button is square, and the glyph is sized off the font so the phone
     can draw the same artwork larger without a second copy: `Icon.svelte` is
     `1em` wide. */
  .controls button.icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 34px;
    padding: 0;
    font-size: 21px;
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
  }
  .controls button.primary:hover:not(:disabled) {
    background: var(--accent-ink);
    color: #fff;
  }
  /* A toggle that is on: recall rather than trace, or corrections showing. Tinted
     rather than filled, so the one solid accent on the row stays the primary
     action — Check — and "which of these is pressed" is still legible without
     competing with it. */
  .controls button.icon.on {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .controls button.primary.icon {
    width: 46px;
  }

  /* Push to talk. Green while the microphone is open, and pulsing, because the
     word "Listening…" that used to say so is gone: an icon cannot spell out
     that it is recording, so the colour and the movement have to. */
  .controls button.say {
    /* The page scrolls, and a finger held still on a button inside a scrolling
       page is a gesture the browser wants to claim: it sends `pointercancel`
       once it decides the touch is a scroll, and `pointercancel` ends the
       recording. Saying the button owns its own touches is what stops a held
       press from being cancelled a fraction of a second after it starts — which
       is what "I hold it and it stops immediately" turned out to be, after the
       layout shift had been ruled out. The layout no longer shifts on press
       either, now that the label is a fixed-width glyph, but this is what keeps
       a sliding finger recording. */
    touch-action: none;
    /* And the *other* gesture a held finger starts: a long press on text. At
       about half a second Android takes the pointer for its selection gesture
       and cancels ours — measured at 555 ms, which is why this is not a
       cosmetic rule. The practice board has guarded against it all along; a
       push-to-talk button needs it for the same reason. */
    user-select: none;
    -webkit-user-select: none;
    -webkit-touch-callout: none;
  }
  .controls button.say.listening {
    background: #2f6f4f;
    border-color: #2f6f4f;
    color: #fff;
    animation: listening 1.1s ease-in-out infinite;
  }
  @keyframes listening {
    50% {
      opacity: 0.62;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .controls button.say.listening {
      animation: none;
    }
  }
  .tone-error {
    /* Above whatever it explains, which is now the tone panel below it. */
    margin: 12px 0 0;
    padding: 10px 12px;
    border: 1px solid #e3bdb4;
    border-radius: 10px;
    background: #fdf1ee;
    color: #7a2a1c;
    font-size: 0.9rem;
    line-height: 1.45;
  }
  /* The tone panel follows the report now, so the gap between the two is above
     it rather than below. */
  .feedback > :global(.tone) {
    margin-top: 12px;
  }
  /* The last group takes the rest of the line and keeps its own controls at the
     right edge, whether the row fits on one line or wraps: the count and ✓ are
     where the eye goes after a stroke, and a fixed place for them is worth more
     than the few pixels a spacer would leave. `auto` basis rather than `0`, so
     wrapping still measures the group by what is in it. */
  .cluster.commit {
    flex: 1 1 auto;
    justify-content: flex-end;
  }
  .count {
    font-size: 0.8rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .hint {
    margin: 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  /* A vocabulary status line, e.g. "学习: 88/100 recorded". */
  .note {
    margin: 0;
    padding: 8px 11px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-size: 0.82rem;
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
  /* The explanation is a disclosure rather than a wall of text: the heading is
     the button, and the panel is one quiet row until it is asked for. */
  .help-toggle {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    font-size: 0.9rem;
    font-weight: 650;
    color: var(--muted-strong);
    text-align: left;
    cursor: pointer;
  }
  .help-toggle:hover {
    color: var(--accent-ink);
  }
  .help-toggle .info {
    font-size: 0.95rem;
    color: var(--accent);
  }
  .help-toggle .chevron {
    margin-left: auto;
    color: var(--muted);
    transition: transform 0.15s ease;
  }
  .help-toggle .chevron.open {
    transform: rotate(90deg);
  }
  .help-body {
    margin-top: 10px;
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

  /* What the app is doing rather than what is wrong: a sync that started by itself
     has to look like something happening, and it must not be mistaken for a warning
     or for an error. */
  .sync-note {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 8px 12px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--surface);
    color: var(--muted-strong);
    font-size: 0.8rem;
  }
  .sync-note.trouble {
    border-color: #fcd34d;
    background: #fffbeb;
    color: #92400e;
  }
  /* The only moving thing in the app that is not the learner's own writing. Small
     and slow on purpose: it has to be visible out of the corner of an eye and must
     not pull attention off the board. */
  .sync-pulse {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    animation: sync-pulse 1.1s ease-in-out infinite;
  }
  @keyframes sync-pulse {
    0%,
    100% {
      opacity: 0.25;
      transform: scale(0.8);
    }
    50% {
      opacity: 1;
      transform: scale(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .sync-pulse {
      animation: none;
      opacity: 1;
    }
  }

  /* Something is wrong with a study file, but practice still works. */
  .warning {
    margin: 0;
    padding: 9px 12px;
    border: 1px solid #fcd34d;
    border-radius: 9px;
    background: #fffbeb;
    color: #92400e;
    font-size: 0.8rem;
    line-height: 1.45;
  }

  /* ---- phone ---------------------------------------------------------------
     The shell above is three columns wide: a 280px sidebar, the board, and a
     330px panel. On a phone that leaves the board off the screen entirely — the
     one thing this app exists to show. Below the breakpoint the same markup
     becomes one scrolling column with the sidebar as a sheet over it, and the
     board sized from the screen's width.

     Several rules have to reach into child components (`.board` inside
     `PracticeCanvas`, `.sidebar` inside `LessonSidebar`), which is why they are
     wrapped in `:global()`: a scoped selector here would carry this component's
     hash and match nothing. The phone layout is deliberately kept in one place,
     so that "what does this look like on a phone" has one answer. */
  .topbar {
    display: none;
  }
  /* On a wide screen the pane is not a pane at all: the sidebar is simply the
     first flex child of `.app`, exactly as it was before this existed. */
  .nav-pane {
    display: contents;
  }
  .scrim {
    display: none;
  }

  @media (max-width: 760px) {
    .app {
      display: block;
    }

    main {
      /* Sized to the *visible* viewport so the browser chrome cannot crop the
         board; `100vh` first for iOS before `dvh` was supported. */
      height: 100vh;
      height: 100dvh;
      overflow-y: auto;
      gap: 10px;
      padding: calc(var(--safe-top) + 8px) 12px calc(var(--safe-bottom) + 12px);
    }

    .topbar {
      display: flex;
      align-items: center;
      gap: 10px;
      min-height: 44px;
    }
    .nav-toggle {
      flex: none;
      width: 44px;
      height: 44px;
      display: grid;
      place-items: center;
      border: 1px solid var(--line);
      border-radius: 10px;
      background: var(--surface);
      color: var(--muted-strong);
      font-size: 1.1rem;
      cursor: pointer;
    }
    .topbar-title {
      flex: 1;
      min-width: 0;
      font-size: 0.92rem;
      font-weight: 600;
      color: var(--muted-strong);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    /* The character navigation moves up here on a phone: beside the meaning it
       took a third of the header's width, and this corner is where the character
       itself used to be — which in recall mode was the answer, shown. */
    .topbar .nav {
      flex: none;
    }

    /* The character's details fold away. Folded, the header keeps the reading and
       the first two lines of the meaning; the strokes, radical, level, frequency,
       progress and etymology wait behind the button. */
    .fold {
      display: inline-flex;
      align-items: center;
      gap: 5px;
      min-height: 44px;
      padding: 0;
      border: 0;
      background: none;
      font: inherit;
      font-size: 0.82rem;
      font-weight: 650;
      color: var(--accent-ink);
      cursor: pointer;
    }
    .fold .chevron {
      color: var(--muted);
      transition: transform 0.15s ease;
    }
    .fold .chevron.open {
      transform: rotate(90deg);
    }
    .meta:not(.open) .meaning {
      display: -webkit-box;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 2;
      line-clamp: 2;
      overflow: hidden;
    }
    .meta:not(.open) .details {
      display: none;
    }
    /* The navigation is one widget drawn in two rows: the top bar carries it on a
       phone, and the header carries it beside the meaning on a wide screen. */
    .meta .nav {
      display: none;
    }

    /* The navigation sheet. */
    .nav-pane {
      display: block;
      position: fixed;
      top: 0;
      bottom: 0;
      left: 0;
      z-index: 40;
      width: min(86vw, 320px);
      padding-top: var(--safe-top);
      padding-bottom: var(--safe-bottom);
      background: var(--surface);
      box-shadow: 0 10px 40px rgb(15 23 42 / 0.3);
      transform: translateX(-102%);
      transition: transform 0.22s ease;
      /* Nothing inside should be reachable while it is off-screen. */
      visibility: hidden;
    }
    .nav-pane.open {
      transform: none;
      visibility: visible;
    }
    /* The sidebar fills the sheet rather than sitting at its fixed 280px. */
    .nav-pane :global(.sidebar) {
      width: 100%;
      height: 100%;
      border-right: 0;
    }
    .scrim {
      display: block;
      position: fixed;
      inset: 0;
      z-index: 30;
      border: 0;
      padding: 0;
      background: rgb(15 23 42 / 0.35);
      cursor: pointer;
    }

    /* One column, in the order practice actually happens: the board, then what
       the board said about the attempt, then the controls you press, then the
       one-line explanation. `display: contents` dissolves the stage wrapper so
       those four can be ordered against each other — on a wide screen the stage
       is a column beside the feedback panel and its own order is right. */
    .workspace {
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .stage {
      display: contents;
    }
    :global(.board) {
      order: 1;
    }
    .slots {
      order: 2;
    }
    /* The controls come before the explanation: what a learner needs after
       drawing is Check, not a paragraph about how grading works. The panel below
       carries the score when there is one, which is worth a short scroll. */
    .controls {
      order: 3;
    }
    aside.feedback {
      order: 4;
    }
    .controls + .note,
    .hint {
      order: 5;
    }
    /* The board is square and as wide as the phone: at this size it is the
       screen's job to fit the board, not the board's to fit a column. */
    :global(.board) {
      flex: none;
      width: 100%;
      aspect-ratio: 1;
      min-height: 0;
    }

    /* Fingers, not a pointer: every control reaches a comfortable target, and
       the glyph grows with it. */
    .controls {
      gap: 7px;
    }
    .controls .cluster {
      gap: 7px;
    }
    .controls button {
      min-height: 44px;
      padding: 10px 14px;
      font-size: 0.95rem;
    }
    .controls button.icon,
    .controls button.primary.icon,
    .controls button.icon.on {
      width: 44px;
      height: 44px;
      padding: 0;
      font-size: 25px;
    }
    .nav button {
      min-height: 44px;
      min-width: 44px;
    }
    .slots :global(button) {
      min-width: 54px;
      min-height: 54px;
    }

    /* A phone has no Enter, S, H or arrow keys to document. */
    .keys {
      display: none;
    }
    /* The disclosure is a comfortable target, and the panel loses some of its
       padding so the collapsed row does not read as a stray box. */
    .help-toggle {
      min-height: 44px;
    }
    .empty {
      padding: 10px 14px;
    }

    /* The character and its reading, compact enough to leave the board room. */
    .meta {
      gap: 12px;
    }
    .meta button.icon {
      width: 44px;
      height: 44px;
      font-size: 23px;
    }
    .glyph {
      /* `min-width`, matching the base rule: setting `width` here would leave the
         92px minimum in force and the box would ignore this entirely. */
      min-width: 68px;
      height: 68px;
      padding: 0 7px;
      font-size: min(2.4rem, calc(160px / var(--chars, 1)));
      border-radius: 10px;
    }
    .glyph.masked {
      font-size: 1.9rem;
    }
    .pinyin {
      font-size: 1.2rem;
      margin: 0 0 2px;
    }
  }
</style>
