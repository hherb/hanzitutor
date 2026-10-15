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
  GrammarDrillSize,
  GrammarLinks,
  GrammarPage,
  GrammarProductionOutcome,
  GrammarProductionQuestion,
  GrammarQuestion,
  GrammarTally,
  CompositionOutcome,
  CompositionPrompt,
  CompositionSize,
  ModelStatus,
  ModelVerdict,
  Kana,
  KanjiLessonView,
  KanjiView,
  LessonView,
  LicenceNotice,
  PassageSummary,
  PassageView,
  PhraseBandView,
  PhraseView,
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
 * One word, by the text a sentence used and the reading drawn over it.
 *
 * What a tapped token has: the token carries the word's *text* and the *reading*
 * the screen drew above it. The text alone does not identify the entry — 106
 * written forms in the vocabulary are read more than one way, and 人 is three words
 * in it (ひと, にん, じん) — while the reading does, because the token's reading is
 * the one the analyser gave that sentence and was what chose the entry. A reading
 * that names no entry is the fallback: a token's reading is its *surface's* (行き is
 * read いき while the word is 行く, read いく), and a caller with only the text
 * passes nothing, which means the same fallback.
 */
export function wordOfText(text: string, reading?: string | null): Promise<Word> {
  return invoke<Word>("word_of_text", { text, reading: reading ?? null });
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
 * The bands the graded phrases are levelled into, with their counts.
 *
 * Every band comes back even when the corpus filled none of it, so the screen can
 * draw the same seven chips it draws for the vocabulary and let the counts say
 * which are empty. The names are `nihongo_core::band_name`'s, because it is the
 * same ladder a word's band comes from — a phrase's band is its hardest word's.
 */
export function phraseBands(): Promise<PhraseBandView[]> {
  return invoke<PhraseBandView[]>("phrase_bands");
}

/**
 * One band's graded phrases, shortest first.
 *
 * Not paged, and that is a property of the data rather than a shortcut:
 * `prepare-phrases` caps each band when it builds the artifact, so a band is
 * already the size of a list the screen can draw.
 */
export function phrasesInBand(band: number): Promise<PhraseView[]> {
  return invoke<PhraseView[]>("phrases_in_band", { band });
}

/**
 * One page of the grammar course, in the order the corpus teaches it.
 *
 * No band, and that is the data rather than a shortcut: the ladder the vocabulary
 * and the phrases share puts 344 of these 526 patterns in its top rung, so the
 * course is one ordered sequence — see `nihongo_core::grammar`. `limit` is clamped
 * by the command to the largest page it will serve.
 */
export function grammarPage(offset = 0, limit = 20): Promise<GrammarPage> {
  return invoke<GrammarPage>("grammar_page", { offset, limit });
}

/**
 * The next grammar question: one of the corpus's own sentences with its pattern
 * taken out, and three patterns to choose between.
 *
 * Drawn in Rust, weighted by how often this learner has missed the pattern, rather
 * than drawn here from a list — only the asker knows which pattern it asked, and the
 * answer has to be remembered against it. `null` means nothing at all could be
 * asked, which the screen says in words rather than leaving blank.
 */
export function nextGrammarQuestion(): Promise<GrammarQuestion | null> {
  return invoke<GrammarQuestion | null>("next_grammar_question", {});
}

/**
 * What the grammar drill can ask of the course, in numbers.
 *
 * Asked for once, so the screen can say how much of the course is askable instead of
 * leaving a learner wondering why a pattern they know never comes up.
 */
export function grammarDrillSize(): Promise<GrammarDrillSize> {
  return invoke<GrammarDrillSize>("grammar_drill_size", {});
}

/**
 * Record what the learner answered, and get the pattern's record back.
 *
 * `point` is the pattern the question asked for and `picked` is the one that was
 * chosen. There is deliberately no way to say whether it was right: correctness is
 * decided in Rust from those two, against the pattern the question named.
 */
export function recordGrammarAnswer(point: string, picked: string): Promise<GrammarTally> {
  return invoke<GrammarTally>("record_grammar_answer", { point, picked });
}

/**
 * The next pattern to write: the corpus's sentence with its pattern's own text taken
 * out, and the learner writes what is missing.
 *
 * The same draw as the drill's, over the production exercise's own pool and weighted
 * by how often this learner has missed the pattern — and deliberately a **smaller**
 * payload: the question carries nothing that answers it, which is what makes writing
 * it worth asking. `null` means nothing at all could be asked, which the screen says
 * in words rather than leaving blank.
 */
export function nextGrammarProduction(): Promise<GrammarProductionQuestion | null> {
  return invoke<GrammarProductionQuestion | null>("next_grammar_production", {});
}

/**
 * What the production exercise can write of the course, in numbers.
 *
 * The same three numbers `grammarDrillSize` reports, over this exercise's own pool
 * (`GrammarDrillSize` rather than a second name for the same shape): the drill needs
 * two distractors and this needs none, and the two come to the same measured reach —
 * 363 of the 526 patterns, over 903 sentences, asked one way and the other.
 */
export function grammarProductionSize(): Promise<GrammarDrillSize> {
  return invoke<GrammarDrillSize>("grammar_production_size", {});
}

/**
 * Grade a written answer, record it, and get the lesson and the pattern's record.
 *
 * `point` names the pattern and `example` is which of its sentences was shown — a
 * **position**, because the sentence contains the answer and the question never
 * carried it. There is deliberately no way to say whether the answer was right:
 * `record_grammar_production` decides in Rust from the corpus's own reading of the
 * hole, exactly as `record_grammar_answer` decides the drill's.
 */
export function recordGrammarProduction(
  point: string,
  example: number,
  typed: string,
): Promise<GrammarProductionOutcome> {
  return invoke<GrammarProductionOutcome>("record_grammar_production", { point, example, typed });
}

/**
 * The next pattern to write a sentence of one's own about.
 *
 * The one grammar prompt that carries the pattern: the exercise is *write a sentence
 * with this one*, so there is nothing to withhold — the corpus never wrote the
 * sentence being asked for. It comes with the pattern's own sentences as a model and
 * the sentences this learner has already written for it, read back out of the app's
 * fifth file.
 */
export function nextGrammarComposition(): Promise<CompositionPrompt | null> {
  return invoke<CompositionPrompt | null>("next_grammar_composition", {});
}

/**
 * How much of the course can be written about, in numbers.
 *
 * Two numbers, and the reach is the whole course: the prompt is the pattern itself,
 * so nothing about a point can make it unaskable the way the blank rule does for the
 * other two exercises (363 of the 526 each).
 */
export function grammarCompositionSize(): Promise<CompositionSize> {
  return invoke<CompositionSize>("grammar_composition_size", {});
}

/**
 * Read the sentence the learner wrote of their own, keep it, and say what was seen.
 *
 * There is deliberately no way to say whether the sentence was right, and none is
 * returned: free composition has no answer to grade against, and
 * `record_grammar_composition` reports only what the machine can see — whether the
 * pattern's own text is in the sentence, whether some of it is not Japanese script,
 * which characters the course does not teach, and whether it is a sentence at all.
 */
export function recordGrammarComposition(
  point: string,
  typed: string,
): Promise<CompositionOutcome> {
  return invoke<CompositionOutcome>("record_grammar_composition", { point, typed });
}

/**
 * The grammar points a query matches, in course order.
 *
 * `total` is how many match altogether and `points` at most `limit` of them, so the
 * screen can say *"showing the first 60 of 111"* rather than implying the page is
 * all of them. An empty query matches nothing: the screen draws the course's own
 * pages while its box is empty.
 */
export function grammarSearch(query: string, limit = 60): Promise<GrammarPage> {
  return invoke<GrammarPage>("grammar_search", { query, limit });
}

/**
 * What one pattern links to: the words its sentences use, and its own characters.
 *
 * The link is the artifact's — `prepare-grammar` wrote it from UniDic's segmentation
 * of the corpus's own sentences — so this is a read, not a decision about what a
 * word is. An unknown id is an error rather than an empty answer, because a pattern
 * with no links and a pattern that is not in the course would draw the same screen.
 */
export function grammarLinks(id: string): Promise<GrammarLinks> {
  return invoke<GrammarLinks>("grammar_links", { id });
}

/**
 * Say a sentence the learner wrote, in the form this app read it as.
 *
 * The command takes what was **typed**, not what was read: `nihongo_core::composition`
 * decides the read-back form, so a screen cannot hand a Japanese voice a line of romaji
 * and the rule about what may be spoken stays in one place. A sentence the app could not
 * read is an error rather than a sound (`HANDOVER_NIHONGO.md` invariant 27, as N24
 * widened it).
 */
export function speakSentence(typed: string): Promise<void> {
  return invoke<void>("speak_sentence", { typed });
}

/**
 * What the learner has configured about the model check — and never the key.
 *
 * Asked once by the composition stage, so the fold can say whether there is an endpoint
 * to send to before offering the button.
 */
export function modelStatus(): Promise<ModelStatus> {
  return invoke<ModelStatus>("model_status");
}

/**
 * Remember the endpoint, the model name and, when one was typed, the key.
 *
 * `key` of `null` means **leave the stored key alone**: the field shows a placeholder
 * rather than the secret, so an untouched field must not clear it. An empty string is
 * the request to forget it.
 */
export function saveModelSettings(
  endpoint: string,
  model: string,
  key: string | null,
): Promise<ModelStatus> {
  return invoke<ModelStatus>("save_model_settings", { endpoint, model, key });
}

/**
 * Ask the configured model to check a sentence the learner wrote.
 *
 * The request carries the pattern and the sentence — in the form the app read it back —
 * and nothing else: none of the corpus's explanations, notes or examples goes to a third
 * party (`crate::model` has the reasoning). The answer is returned, shown and forgotten;
 * nothing about it is written to any of the app's files.
 */
export function checkSentence(point: string, typed: string): Promise<ModelVerdict> {
  return invoke<ModelVerdict>("check_sentence", { point, typed });
}

/**
 * Say `text` in the system's Japanese voice, cutting off anything already being
 * said.
 *
 * `text` is always **kana**: a kana character, a word's own stored reading, the
 * stored readings of a corpus sentence, or a sentence the learner wrote read back —
 * never a written sentence. Nothing here composes a reading from a written form —
 * 大人 is おとな, and a bare kanji has a dozen readings — and nothing hands the voice
 * a written sentence to parse either, because it parses `お腹痛いの？` as
 * `おふくつういの` (see `spokenRuby` in `words.ts`). A screen whose only text is
 * characters does not call this at all.
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
