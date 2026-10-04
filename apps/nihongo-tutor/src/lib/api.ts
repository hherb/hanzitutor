/**
 * The IPC surface, in one place.
 *
 * Every function here is a thin wrapper over `invoke`, so a screen never writes
 * a command name as a string literal in two places. The names match the
 * `#[tauri::command]` functions in `src-tauri/src/lib.rs`, and the payload keys
 * match the field names this app's `types.ts` declares.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  BandView,
  ChartView,
  DatasetStats,
  DrillKind,
  DrillQuestion,
  DrillTally,
  GradedAttempt,
  Kana,
  KanjiLessonView,
  KanjiView,
  LessonView,
  LicenceNotice,
  PassageSummary,
  PassageView,
  Point,
  Prefs,
  RadicalFamilyView,
  RadicalView,
  ReadingCheck,
  ReviewQueueView,
  ScriptName,
  Word,
  WordPage,
  WordsOfKanji,
  WordsOfReading,
  YoonView,
} from "./types";
import type { Section } from "./nav";

/**
 * The pen width the canvas paints with, in design units out of 1024.
 *
 * The grader is told this explicitly so that the ink measure compares what was
 * drawn with what a correct trace at the same width would put down. It is the
 * same number as `INK_WIDTH` in `render.ts` and in
 * `crates/hanzi-core/src/raster.rs`; passing it rather than letting the grader
 * assume keeps the three from drifting apart silently.
 */
import { INK_WIDTH } from "./render";

export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}

export function datasetStats(): Promise<DatasetStats> {
  return invoke<DatasetStats>("dataset_stats");
}

export function lessons(script: ScriptName): Promise<LessonView[]> {
  return invoke<LessonView[]>("lessons", { script });
}

/**
 * The whole kana chart for one script: the gojūon grid and the characters that
 * are not on it.
 *
 * One call rather than two, because the grid and the off-grid groups are one
 * screen's answer and asking separately would let the two halves describe
 * different scripts. Every kana it offers can be opened on the board.
 */
export function kanaChart(script: ScriptName): Promise<ChartView> {
  return invoke<ChartView>("kana_chart", { script });
}

export function kana(ch: string): Promise<Kana> {
  return invoke<Kana>("kana", { ch });
}

/**
 * Grade a handwritten attempt, and let the schedule decide when it comes back.
 *
 * The response wraps the report rather than being it: `scheduled` says whether
 * this attempt counted as a review (the character was new or due), `nextDue` when
 * it comes back, and `warning` when the attempt could not be written to the
 * learner's own file. Grading and scheduling are one command because they are one
 * action — the learner wrote a character and pressed Grade — and separating them
 * would let the two disagree about what was graded.
 *
 * Only the pen width is sent; the other three tunables are left to the Rust
 * default, because those defaults are the ones the whole tolerance study was
 * fitted against and the interface has no business overriding them piecemeal.
 * `GradeOptions` therefore deserialises a partial object — an interface that
 * sends some fields and not others must not be rejected for the ones it left
 * out.
 */
export function gradeAttempt(ch: string, strokes: Point[][]): Promise<GradedAttempt> {
  return invoke<GradedAttempt>("grade_attempt", {
    ch,
    strokes,
    options: { inkWidth: INK_WIDTH },
  });
}

/**
 * What the board has taught and what is due, for one course.
 *
 * `limit` caps the items returned, not the count: the screen shows the top of a
 * backlog and says how big the backlog is. It is paged rather than whole because
 * a learner working through the kanji course can have hundreds of characters due
 * at once, and none of the rest of the app ships that much JSON to draw a list.
 *
 * `section` is which course is asking, and every count in the answer is that
 * course's. The schedule behind it is one file of characters — the split is in
 * what a screen offers, never in what is stored — so the kana screen is not shown
 * a due kanji and the character screen carries the radical head forms with it.
 */
export function reviewQueue(limit = 40, section?: Section): Promise<ReviewQueueView> {
  return invoke<ReviewQueueView>("review_queue", { limit, section: section ?? null });
}

/** Which half of the app to open on, as the learner left it. */
export function prefs(): Promise<Prefs> {
  return invoke<Prefs>("prefs");
}

/**
 * Remember the half of the app the learner moved to.
 *
 * The answer is a warning rather than a failure — the learner did move, and this
 * session knows where they are; what may be lost is only that the next start
 * returns there — so it is returned rather than thrown.
 */
export function setSection(section: Section): Promise<string | null> {
  return invoke<string | null>("set_section", { section });
}

/** Check a typed reading, accepting either romanisation. */
export function checkReading(ch: string, typed: string): Promise<ReadingCheck> {
  return invoke<ReadingCheck>("check_reading", { ch, typed });
}

/** Turn typed romaji into kana. */
export function romajiToKana(input: string, script?: ScriptName): Promise<string> {
  return invoke<string>("romaji_to_kana", { input, script: script ?? null });
}

/**
 * The next question for the discrimination drill, or `null` if no pair can be
 * asked at all.
 *
 * The pair is drawn in Rust, weighted by how often this learner has missed it —
 * the rule is in `nihongo_core::drill` — rather than drawn here from a pool. That
 * is not a layering preference: only the asker knows which pair it asked, and the
 * answer has to be remembered against that pair.
 *
 * `kind` picks the exercise: the classic confusions, or the yōon contrasts of one
 * script. The association matters — `kya` names きゃ in hiragana and キャ in
 * katakana — which is why the script is part of the kind rather than a second
 * argument.
 */
export function nextDrillQuestion(kind: DrillKind = "confusion"): Promise<DrillQuestion | null> {
  return invoke<DrillQuestion | null>("next_drill_question", { kind });
}

/**
 * Record what the learner answered, and get the pair's record back.
 *
 * `target` is the spelling the question asked for and `picked` is the one that
 * was chosen — spellings, not kana, because a yōon answer is two characters.
 * There is deliberately no way to say whether it was right: correctness is
 * decided in Rust from those two, against the pair the question named.
 */
export function recordDrillAnswer(
  pair: string,
  target: string,
  picked: string,
): Promise<DrillTally> {
  return invoke<DrillTally>("record_drill_answer", { pair, target, picked });
}

/** Every notice this app owes, with its text. */
export function licences(): Promise<LicenceNotice[]> {
  return invoke<LicenceNotice[]>("licences");
}

/** The yōon digraphs for a script, for an input helper. */
export function yoon(script: ScriptName): Promise<YoonView[]> {
  return invoke<YoonView[]>("yoon", { script });
}

/**
 * The kanji course: every grade in teaching order, sliced into lessons.
 *
 * Whole rather than paged, unlike a band of words: it is 2,136 characters and
 * their lesson keys, where a band is nearly five thousand words with readings,
 * glosses and furigana each.
 */
export function kanjiLessons(): Promise<KanjiLessonView[]> {
  return invoke<KanjiLessonView[]>("kanji_lessons");
}

/** One kanji: the geometry the board writes, the readings, and the structure. */
export function kanji(ch: string): Promise<KanjiView> {
  return invoke<KanjiView>("kanji", { ch });
}

/** The 214 Kangxi radicals, each with the characters that share it. */
export function radicals(): Promise<RadicalFamilyView[]> {
  return invoke<RadicalFamilyView[]>("radicals");
}

/**
 * One radical, with the geometry the board writes it with.
 *
 * A head form is not always a jōyō character — 92 of the 214 are not — so this is
 * how the board gets something to draw for the radicals the character course
 * cannot reach.
 */
export function radical(number: number): Promise<RadicalView> {
  return invoke<RadicalView>("radical", { number });
}

/** The vocabulary ladder: each band's name and how many words it holds. */
export function wordBands(): Promise<BandView[]> {
  return invoke<BandView[]>("word_bands");
}

/**
 * One page of a band's words, in course order.
 *
 * Paged because band 7 holds nearly five thousand words: the screen draws a list,
 * and the whole band as JSON would be a megabyte to show sixty rows.
 */
export function wordsInBand(band: number, offset = 0, limit = 60): Promise<WordPage> {
  return invoke<WordPage>("words_in_band", { band, offset, limit });
}

/**
 * One page of the words this course teaches that are written with `ch`, in course
 * order — what a character's card lists beside its readings and its radical.
 *
 * Paged for the same reason a band is, measured: 一 is written in 223 of the
 * 16,073 words. A jōyō character no word uses answers with an empty page and a
 * total of zero, which a card says in words; a character outside the jōyō set is
 * an error, because the vocabulary holds no word it could appear in.
 */
export function wordsOfKanji(ch: string, offset = 0, limit = 12): Promise<WordsOfKanji> {
  return invoke<WordsOfKanji>("words_of_kanji", { ch, offset, limit });
}

/**
 * One page of the words this course teaches that are read `reading`, in course
 * order — the Start screen's demonstration that one sound names several words.
 *
 * The reading must be **kana**: a romaji query is rejected rather than answered
 * with an empty page, because every reading in the vocabulary is kana and an
 * empty page would let a typo read as a gap in the data. A kana reading the
 * course does not carry is a legitimate empty page, which the screen states in
 * words.
 */
export function wordsOfReading(
  reading: string,
  offset = 0,
  limit = 12,
): Promise<WordsOfReading> {
  return invoke<WordsOfReading>("words_of_reading", { reading, offset, limit });
}

/** One word, by its text and its reading. */
export function word(text: string, reading: string): Promise<Word> {
  return invoke<Word>("word", { text, reading });
}

/**
 * One word, by its text alone.
 *
 * What a tapped passage token has: the token carries the word's *text*, and its
 * own reading is the surface's (行き is read いき while the word is 行く, read
 * いく), so the reading cannot identify the entry. Where a text is read two ways
 * this returns the one the course reaches first.
 */
export function wordOfText(text: string): Promise<Word> {
  return invoke<Word>("word_of_text", { text });
}

/**
 * Check a typed reading against a word.
 *
 * The word is graded *as a word*: its own reading, taken from the dictionary and
 * never composed from its characters. Romaji and kana are both accepted — a
 * learner who can read kana should not have to transliterate to answer.
 */
export function checkWord(text: string, reading: string, typed: string): Promise<ReadingCheck> {
  return invoke<ReadingCheck>("check_word", { text, reading, typed });
}

/** The passages a learner can read. */
export function passages(): Promise<PassageSummary[]> {
  return invoke<PassageSummary[]>("passages");
}

/** One passage, segmented, with a reading over every kanji. */
export function passage(key: string): Promise<PassageView> {
  return invoke<PassageView>("passage", { key });
}

/**
 * Say `text` in the system's Japanese voice, cutting off anything already being
 * said.
 *
 * `text` is always **kana**: a kana character, or a word's own stored reading.
 * Nothing here composes a reading from a written form — 大人 is おとな, and a bare
 * kanji has a dozen readings — so a screen whose only text is characters does not
 * call this at all.
 *
 * The utterance is the operating system's, spoken in process, and nothing is
 * downloaded to produce it: the app still has no network path.
 */
export function speak(text: string): Promise<void> {
  return invoke<void>("speak", { text });
}

/**
 * Stop the current utterance.
 *
 * Separate from [`speak`], which starts one; the app calls this when the learner
 * moves to another character or another screen, so a voice cannot keep talking
 * over the thing they have moved on to.
 */
export function stopSpeaking(): Promise<void> {
  return invoke<void>("stop_speaking");
}

/**
 * The voice pronunciation will use, or `null` when the machine has none.
 *
 * Asked once at startup so every "Hear it" button can be disabled together and
 * the reason shown, rather than offering a control that silently does nothing.
 * The answer is cached in Rust after the first call, so this is cheap to ask.
 */
export function voice(): Promise<string | null> {
  return invoke<string | null>("voice");
}
