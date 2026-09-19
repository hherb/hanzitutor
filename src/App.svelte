<script lang="ts">
  import { onMount } from "svelte";
  import {
    confirm as confirmDialog,
    open as pickFile,
    save as pickSavePath,
  } from "@tauri-apps/plugin-dialog";
  import * as api from "./lib/api";
  import FeedbackPanel from "./lib/FeedbackPanel.svelte";
  import LessonSidebar from "./lib/LessonSidebar.svelte";
  import PracticeCanvas from "./lib/PracticeCanvas.svelte";
  import VocabularyPanel from "./lib/VocabularyPanel.svelte";
  import WordsPanel from "./lib/WordsPanel.svelte";
  import { INK_WIDTH } from "./lib/render";
  import type {
    Character,
    DatasetStats,
    GradeReport,
    Lesson,
    Point,
    PracticeItem,
    ProgressView,
    ReviewView,
    VocabEntry,
    VocabView,
    Word,
    WordSearchView,
  } from "./lib/types";

  type Mode = "trace" | "recall";
  /** Which of the three top-level screens is showing. */
  type View = "course" | "vocabulary" | "words";
  /** Where the current practice session draws its characters from. */
  type Source = "course" | "vocabulary" | "review" | "words";

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

  let view = $state<View>("course");

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
  /** Scores for the characters of the current entry, averaged when it is done. */
  let wordScores = $state<number[]>([]);
  /** True once the last item of a list or review session has been finished, so
   * the last entry cannot be recorded twice. */
  let sessionDone = $state(false);

  /** Position in the flattened course; lessons are just a view onto it. */
  let index = $state(0);
  /** How much of the reference character is on the board. */
  let revealed = $state(0);
  /** True while the stroke-order animation runs. */
  let playing = $state(false);
  let playToken = 0;

  const allCharacters = $derived(lessons.flatMap((lesson) => lesson.characters));
  const courseChar = $derived(allCharacters[index] ?? null);
  const currentItem = $derived(
    source === "course" ? null : (queue[queueCursor] ?? null),
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
  const answerVisible = $derived(mode === "trace" || revealed > 0);

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
    void refreshProgress();
    void refreshReview();

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
        void playStrokeOrder();
      } else if (event.key.toLowerCase() === "h") {
        void hear();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("error", onError);
      window.removeEventListener("unhandledrejection", onRejection);
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
    void loadCharacter(next);
  });

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

  // ---- personal vocabulary list -------------------------------------------

  /** Adopt a view returned by a command. */
  function applyVocab(next: VocabView) {
    vocab = next;
    if (next.warning) void api.log(`vocabulary warning: ${next.warning}`);
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
    charCursor = 0;
    wordScores = [];
    sessionDone = false;
    source = from;
    mode = "recall";
    reset();
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
    charCursor = 0;
    wordScores = [];
    sessionDone = false;
    reset();
  }

  /** Move within the practice queue, abandoning an unfinished entry. */
  function moveQueue(delta: number) {
    const next = queueCursor + delta;
    if (next < 0 || next >= queue.length) return;
    queueCursor = next;
    charCursor = 0;
    wordScores = [];
    sessionDone = false;
    reset();
  }

  /**
   * Record the graded character and move on.
   *
   * A word is written one character at a time and scored by the mean of its
   * characters, so a long word cannot be credited on one good stroke. Each
   * character already has its own card by the time this runs; the mean is what
   * the vocabulary entry itself remembers.
   */
  async function finishCharacter() {
    // Once the queue is exhausted the last entry must not be finished again: a
    // second press would credit it with an attempt nobody made.
    if (sessionDone || !currentItem || !report) return;
    const scores = [...wordScores, report.overall];

    if (charCursor + 1 < entryCharacters.length) {
      wordScores = scores;
      charCursor += 1;
      reset();
      return;
    }

    const mean = scores.reduce((total, value) => total + value, 0) / scores.length;
    const finished = currentItem;
    const entryId = finished.entryId;
    wordScores = [];

    const isLast = queueCursor + 1 >= queue.length;
    if (isLast) {
      charCursor = 0;
      sessionDone = true;
    } else {
      queueCursor += 1;
      reset();
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

<div class="app">
  <LessonSidebar
    view={view}
    onSwitchView={switchView}
    {lessons}
    {activeLesson}
    activeCharacter={courseChar}
    {summary}
    onSelectLesson={goToLesson}
    onSelectCharacter={goToCharacter}
    progressCards={progress.cards}
    {review}
    reviewing={source === "review"}
    onStartReview={startReview}
    vocabGroups={vocab.groups}
    vocabEntries={vocab.entries}
    vocabSelection={vocabSelection}
    onSelectVocabGroup={(selection) => (vocabSelection = selection)}
    wordLevels={stats?.wordLevels ?? []}
    wordsTotal={stats?.words ?? 0}
    {wordLevel}
    onSelectWordLevel={(level) => (wordLevel = level)}
  />

  <main>
    {#if error}
      <p class="error">{error}</p>
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
    {:else if !character}
      <p class="status">No character selected.</p>
    {:else}
      <header class="meta">
        {#if currentItem}
          <div class="glyph" lang="zh-Hans" class:masked={!answerVisible}>
            {answerVisible ? (entryCharacters[charCursor] ?? "?") : "?"}
          </div>
          <div class="detail">
            <p class="pinyin">{currentItem.pinyin || character.pinyin.join("  ·  ") || "—"}</p>
            <p class="meaning">{currentItem.meaning || character.definition || "—"}</p>
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
          <div class="nav">
            <button onclick={() => moveQueue(-1)} disabled={queueCursor === 0}>←</button>
            <span>{queueCursor + 1} / {queue.length}</span>
            <button
              onclick={() => moveQueue(1)}
              disabled={queueCursor >= queue.length - 1}>→</button
            >
          </div>
        {:else}
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
          <div class="nav">
            <button onclick={() => move(-1)} disabled={index === 0}>←</button>
            <span>{index + 1} / {allCharacters.length}</span>
            <button onclick={() => move(1)} disabled={index >= allCharacters.length - 1}>
              →
            </button>
          </div>
        {/if}
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

            {#if source === "course"}
              <button
                onclick={addCurrentToVocabulary}
                disabled={vocabBusy}
                title="Add this character to your vocabulary list"
              >
                + Add to my list
              </button>
            {:else if source === "vocabulary"}
              <button onclick={stopPractising}>Back to list</button>
            {:else if source === "words"}
              <button onclick={stopPractising}>Back to words</button>
            {:else}
              <button onclick={stopReview}>Stop reviewing</button>
            {/if}

            <span class="spacer"></span>

            <label class="toggle">
              <input type="checkbox" bind:checked={showCorrections} />
              corrections
            </label>
            <span class="count" title={report
              ? `${report.givenStrokes} of ${report.expectedStrokes} strokes were graded; ignored stray marks are not counted`
              : `${strokes.length} marks drawn, ${strokeTotal} strokes expected`}>
              {report ? report.givenStrokes : strokes.length} / {strokeTotal}
            </span>

            {#if report && !sessionDone}
              <button class="primary" onclick={advance}>
                {source === "course"
                  ? "Next →"
                  : charCursor + 1 < entryCharacters.length
                    ? "Next character →"
                    : "Finish entry →"}
              </button>
            {:else if !sessionDone}
              <button
                class="primary"
                onclick={check}
                disabled={strokes.length === 0 || grading}
              >
                {grading ? "Checking…" : "Check"}
              </button>
            {/if}
          </div>

          {#if statusMessage}
            <p class="note">{statusMessage}</p>
          {/if}

          <p class="hint">
            {#if voice === null}
              No Chinese voice is installed, so the pronunciation button is
              disabled. Add one in System Settings → Accessibility → Spoken
              Content → System Voice → Manage Voices.
            {:else if source !== "course" && entryCharacters.length > 1}
              Write the word one character at a time. Its score is the average
              across its characters, so each one has to be right.
            {:else if source === "review"}
              This came due for review. Write it from memory, then press Check —
              how well you do sets when you see it again.
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
                Your strokes are compared with the reference on four measures:
                shape, placement, ink and order — whether each stroke is the right
                kind of stroke in the right place, whether as much ink went down as
                the character needs (a stroke traced correctly but drawn far too
                thin is not legible), and whether they were written in sequence.
                Writing a legible character in the wrong order is reported as
                exactly that.
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
</style>
