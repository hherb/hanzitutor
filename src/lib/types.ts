/**
 * Types mirroring the Rust structures exposed over Tauri.
 *
 * The Rust side serialises with camelCase field names, so these line up
 * one-for-one with `hanzi_core::{Character, Lesson, GradeReport}`.
 */

/** A point in the 1024x1024 character box, y increasing downwards. */
export interface Point {
  x: number;
  y: number;
}

export type Verdict =
  | "correct"
  | "shape_off"
  | "position_off"
  | "wrong_direction"
  | "out_of_order"
  | "faint"
  | "missing";

export type Grade = "excellent" | "good" | "fair" | "poor";

export interface StrokeVerdict {
  refIndex: number;
  /** Index into the attempt's strokes, or null when the stroke was never written. */
  userIndex: number | null;
  verdict: Verdict;
  shape: number;
  position: number;
  /**
   * How much of the ink a correct stroke needs that this one put down, 0..1.
   * The only measure that can see a stroke drawn too thin: the shape score is
   * scale-invariant, so it cannot see width at all.
   */
  ink: number;
  score: number;
}

/** How the attempt sat in the box before any fitting was applied. */
export interface FitInfo {
  scale: number;
  offset: number;
}

export interface GradeReport {
  expectedStrokes: number;
  givenStrokes: number;
  strayStrokes: number;
  countOk: boolean;
  strokes: StrokeVerdict[];
  /** `assignment[userStrokeIndex]` = reference stroke index, or null if extra. */
  assignment: (number | null)[];
  shapeScore: number;
  positionScore: number;
  /**
   * How much ink was put down, 0..1, averaged over the character's strokes with
   * anything unwritten counting zero. 1 means every stroke laid down as much ink
   * as a correct trace at the canvas pen width; about 0.33 means a pen a third
   * of that width.
   */
  inkScore: number;
  /**
   * How much of the character's own ink was reached, 0..1 — the "you never drew
   * that part" signal. Reported rather than scored: a wobbly but correctly inked
   * stroke also misses part of the outline, and that is a placement fault the
   * position score already covers.
   */
  inkCoverage: number;
  orderScore: number;
  overall: number;
  legible: boolean;
  orderCorrect: boolean;
  grade: Grade;
  firstError: number | null;
  fit: FitInfo | null;
}

/**
 * The measures behind one attempt's headline score, as they are stored.
 *
 * This is `GradeReport` reduced to the part that cannot be recomputed once the
 * strokes are gone. It is sent back with the score when an attempt is recorded,
 * so the grader can be checked against real handwriting later — the four
 * weights and the shape tolerance are still a judgement, and this is the only
 * evidence that can settle them.
 */
export interface AttemptMeasures {
  shape: number;
  position: number;
  ink: number;
  inkCoverage: number;
  order: number;
  legible: boolean;
  orderCorrect: boolean;
}

export interface Character {
  ch: string;
  /** Frequency rank, 1 = most common; 0 when not in the frequency list. */
  rank: number;
  /** HSK level, or 0 when not in the HSK lists. */
  hsk: number;
  strokeCount: number;
  radical: string;
  pinyin: string[];
  definition: string;
  etymology: string;
  /** SVG path data in font space (y up), one per stroke, in stroke order. */
  outlines: string[];
  /** Stroke centre-lines in display space (y down), in stroke order. */
  medians: Point[][];
}

export interface Lesson {
  index: number;
  title: string;
  characters: string[];
  firstRank: number;
  lastRank: number;
}

export interface DatasetStats {
  characters: number;
  teachable: number;
  lessons: number;
  lessonSize: number;
  /** How many words the HSK dictionary holds. */
  words: number;
  /** How many words sit at each HSK level, lowest first. */
  wordLevels: LevelCount[];
  /** How many characters sit at each HSK level, lowest first. */
  characterLevels: CharacterLevelCount[];
}

/** How many words one HSK level holds. */
export interface LevelCount {
  level: number;
  words: number;
}

/** How many characters one HSK level holds. */
export interface CharacterLevelCount {
  level: number;
  characters: number;
}

// ---- the character dictionary ---------------------------------------------

/**
 * One character as a search result.
 *
 * This is a {@link Character} without its stroke geometry. Outlines and
 * centre-lines are tens of kilobytes each, and a page of results is a list, so
 * the board's own `getCharacter` is what fetches the one character it is about
 * to teach.
 */
export interface CharacterSummary {
  ch: string;
  /** Frequency rank, 1 = most common; 0 when the course does not teach it. */
  rank: number;
  /** HSK level, or 0 when not in the HSK lists. */
  hsk: number;
  strokeCount: number;
  /** Kangxi radical, or `"\0"` when unknown. */
  radical: string;
  /** Every reading the dataset knows, most common first. */
  pinyin: string[];
  definition: string;
  etymology: string;
  /**
   * True when the character is in the course's frequency order, so it can be
   * found by browsing and shown in a lesson. A character found by search can be
   * `false` here, and the panel says so rather than hiding it.
   */
  inCourse: boolean;
}

/** One page of a character search, with the number of matches behind it. */
export interface CharacterSearchView {
  characters: CharacterSummary[];
  /** How many characters matched in total; the page is capped. */
  total: number;
}

// ---- tone pairs -----------------------------------------------------------

/**
 * One character of a tone set: a syllable read at one tone.
 *
 * `reading` is the reading that placed it here, which for the drill's sake is
 * always the character's first — the one a voice says for the glyph on its own.
 */
export interface ToneSetMember {
  ch: string;
  /** The reading with its tone mark, e.g. `"mā"`. */
  reading: string;
  /** The tone of that reading, 1..=4. */
  tone: number;
  definition: string;
  /** Frequency rank; orders the set and is shown beside the character. */
  rank: number;
}

/**
 * Characters that differ only in tone: one syllable, one character per tone.
 *
 * Derived in Rust from the dataset's own readings (see
 * `hanzi_core::Dataset::tone_sets`), because nothing stores this grouping.
 */
export interface ToneSet {
  /** The syllable with tone marks stripped, e.g. `"ma"`. `ü` stays distinct. */
  base: string;
  /** The members, tone 1 first. Two or more by construction. */
  members: ToneSetMember[];
}

// ---- the word dictionary --------------------------------------------------

/**
 * One word from the HSK 3.0 vocabulary.
 *
 * Single characters are deliberately absent: the course already teaches every
 * character with its most common reading, so a word entry is always several
 * characters written in turn.
 */
export interface Word {
  /** The word in simplified characters, e.g. `"学习"`. */
  text: string;
  /**
   * The reading of the whole word, e.g. `"xuéxí"`. Taken from a dictionary, so
   * a polyphonic word is right: 着急 is `zháojí`, not `zhejí`.
   */
  pinyin: string;
  meaning: string;
  /** Lowest HSK 3.0 level the word appears in, 1..=7. */
  hsk: number;
  /** Derived frequency: the rarest character's rank. Ordering only. */
  rank: number;
}

/** One page of a word search, with the number of matches behind it. */
export interface WordSearchView {
  words: Word[];
  /** How many words matched in total; `words` is capped to one page. */
  total: number;
}

export interface GradeOptions {
  resampleK: number;
  minStrokeLen: number;
  globalFit: boolean;
  /**
   * Width, in design units, of the ink the canvas paints the attempt with. The
   * grader rasterises the strokes at this width, so it must be the width the
   * board actually drew them with.
   */
  inkWidth: number;
}

// ---- personal vocabulary list ---------------------------------------------

/** What one character of a piece of study text contributes. */
export interface CharacterHint {
  ch: string;
  /** Every reading the dataset knows, most common first. */
  pinyin: string[];
  meaning: string;
}

/**
 * A draft reading and meaning for some study text.
 *
 * A single character gets both. A word gets its readings composed — word pinyin
 * is the characters' readings run together — but its meaning is left blank,
 * because a word's meaning cannot be derived from its characters.
 */
export interface TextLookup {
  pinyin: string;
  meaning: string;
  characters: CharacterHint[];
  /** True when every character was found and has a reading. */
  complete: boolean;
}

/**
 * A reading with a tone mark written into one syllable of it.
 *
 * `caret` is where the cursor should land afterwards, as a **character** offset
 * into `text` — not the UTF-16 index an `HTMLInputElement` reports. The pinyin
 * tone row is the one place the two numbers meet, so it is the panel's job to
 * convert on the way in and on the way out; see `api.markTone`.
 */
export interface MarkedTone {
  text: string;
  caret: number;
}

/**
 * One item in the personal vocabulary list.
 *
 * `text` may be a single character or a word. Single characters get their pinyin
 * and meaning filled from the dataset; words carry whatever the user typed,
 * because the dataset has no word data.
 */
export interface VocabEntry {
  id: number;
  text: string;
  pinyin: string;
  meaning: string;
  /** Group name, or null when the entry is not filed under a lesson. */
  group: string | null;
  /** ISO-8601 UTC timestamp, which also sorts chronologically as text. */
  addedAt: string;
  attempts: number;
  bestScore: number | null;
  lastPractised: string | null;
  /**
   * What the **schedule** says about this entry, or `null` when it was not
   * consulted.
   *
   * Both answers in there are derived from the SM-2 cards, which are folded from
   * the synced attempt log, so they mean the same thing on every device. The three
   * fields above are the opposite kind of fact: this device's own. They are kept
   * as a local record and nothing on the vocabulary screen decides anything from
   * them — in particular the drill's queue does not, because a device that had
   * just synced would otherwise offer everything the other device had finished.
   */
  standing: EntryStanding | null;
}

/**
 * What the schedule says about one entry.
 *
 * `null` for the whole object means the schedule was not consulted, which is
 * **not** the same as a `new`/`false` standing — the distinction schema 5 draws
 * between a null measure and a zero one. A reader that conflates them reports
 * something it invented.
 */
export interface EntryStanding {
  /** How well the entry is known, for the tag on the card. */
  progress: EntryProgress;
  /**
   * Whether every character the board can draw for this entry has been practised
   * at least once — the question the drill's queue asks, since an entry is
   * recorded only when all of its characters are written.
   *
   * Strict on purpose: one character still untouched keeps the entry in the queue,
   * so half-written work is offered again rather than counted as done.
   */
  allCharactersPractised: boolean;
}

/**
 * How well a vocabulary entry is known, in the four states the schedule can be in.
 *
 * `new` is no character of the entry ever practised, `due` is something due now,
 * `known` is every character scheduled weeks out with none due, and `learning` is
 * everything between — including an entry with one character never practised at
 * all. See `hanzi_core::progress::entry_standing` for the rule.
 */
export type EntryProgress = "new" | "learning" | "due" | "known";

export interface VocabView {
  entries: VocabEntry[];
  groups: string[];
  /**
   * Set when a change was applied in memory but not saved, so the interface can
   * say so instead of appearing to have lost data.
   */
  warning: string | null;
}

/** A vocabulary change plus a note about what happened. */
export interface VocabOutcome {
  view: VocabView;
  message: string;
}

// ---- practice progress and review -----------------------------------------

/**
 * How well an attempt went, in the four grades a review offers. Derived by the
 * backend from the attempt score, using the same bands as `Grade`.
 */
export type Rating = "again" | "hard" | "good" | "easy";

/** One recorded attempt. */
export interface Attempt {
  /** ISO-8601 UTC timestamp. */
  at: string;
  score: number;
  rating: Rating;
}

/** Everything remembered about one character. */
export interface ProgressCard {
  ch: string;
  attempts: number;
  lapses: number;
  bestScore: number | null;
  lastScore: number | null;
  /** ISO-8601 UTC timestamp. */
  lastPractised: string | null;
  /** When it should next be reviewed, ISO-8601 UTC. Sorts as text. */
  due: string;
  intervalDays: number;
  ease: number;
  repetitions: number;
  /** Recent attempts, oldest first. */
  history: Attempt[];
  /** Whether the due date has already passed. */
  dueNow: boolean;
}

export interface ProgressView {
  /** Every practised character, most overdue first. A character absent from
   * this list has never been attempted. */
  cards: ProgressCard[];
  /** Set when a change was kept in memory but not saved. */
  warning: string | null;
}

/** Which of the two sources a due character came from. */
export type ReviewSource = "course" | "vocabulary";

/**
 * One thing to review.
 *
 * For a word this describes the entry, not just the character that came due: a
 * word is practised whole, so it appears once however many of its characters
 * are due.
 */
export interface ReviewItem {
  /** The character whose card came due. */
  ch: string;
  due: string;
  source: ReviewSource;
  /** The vocabulary entry, when the character belongs to one. */
  entryId: number | null;
  /** The text to write: an entry's word, or the character itself. */
  text: string;
}

export interface ReviewView {
  items: ReviewItem[];
  /** How many items are due in total; `items` is capped to one session. */
  dueCount: number;
  warning: string | null;
}

/** Where the reader was in the course. */
export interface CursorView {
  index: number;
  updatedAt: string | null;
  warning: string | null;
}

/**
 * How fast the stroke-order animation runs.
 *
 * A closed set, and `normal` is what the animation has always done. The scale
 * each name maps to lives in `App.svelte`, beside the animation's own bounds.
 */
export type Pace = "slow" | "normal" | "fast";

/** How large the practice board is drawn. */
export type BoardSize = "compact" | "normal" | "large";

/**
 * The learner's settings.
 *
 * A field is `null` when nobody has chosen it, which is *not* the same as
 * `false`: the interface resolves an unchosen preference from the device and only
 * writes a value once the learner has changed it themselves. Only the
 * preferences a *device* can answer for are nullable — click-to-draw, and the
 * voice. A pace or a board size has no device default to follow, so it is always
 * one of its values and the stored absence simply means the default.
 */
export interface SettingsView {
  /** `true` click to start and click to finish; `false` press and drag; `null` let the device decide. */
  clickToDraw: boolean | null;
  /** The pronunciation voice by name, or `null` for the automatic choice. */
  voice: string | null;
  animationPace: Pace;
  boardSize: BoardSize;
  /**
   * Whether the introduction has been read and dismissed.
   *
   * The one field here the app writes rather than the settings screen: the
   * introduction is shown once, on the first run that has not seen it. Not
   * nullable — like a pace or a board size there is no device answer to resolve
   * a missing one from, so `false` is simply "not seen yet".
   */
  introSeen: boolean;
  /**
   * The version whose "what's new" pages have been read, or `null` for none.
   *
   * The upgrade half of the same idea, and a *version* rather than a flag
   * because the question is "have you read the notes for this release": a
   * boolean would either show every release's notes again or none of them.
   * `null` is what an installation upgrading from a build without them has.
   */
  whatsNewSeen: string | null;
  /** Set when a change was applied in memory but could not be saved. */
  warning: string | null;
}

/**
 * What the settings screen changed.
 *
 * Every field is optional, and an absent one means **leave that preference
 * alone**: the screen sends only the control the learner touched, so changing
 * the voice must not reset the board size on the way past. Clearing is spelled
 * per preference — `clickToDraw` is `null` to go back to what the device wants
 * (which is its own command, see `api.clearClickToDraw`), and `voice` is `""` to
 * go back to the automatic voice.
 *
 * The index signature is what lets this go straight to `invoke`, which takes a
 * plain object; the named fields above are still the whole of what may be sent,
 * because Tauri rejects an argument the command does not declare.
 */
export interface SettingsPatch {
  clickToDraw?: boolean;
  voice?: string;
  animationPace?: Pace;
  boardSize?: BoardSize;
  /**
   * `true` once the introduction has been dismissed.
   *
   * There is no `false`: the settings screen's "show it again" replays the
   * introduction in place and deliberately sends nothing, so reading it a second
   * time never depends on clearing the record first. Absent — like every other
   * field — means leave it alone.
   */
  introSeen?: boolean;
  /**
   * The version whose "what's new" pages have been read.
   *
   * Sent when those pages are dismissed. Like `voice` this is a name that is
   * never legitimately blank, so an empty string is not a value the caller
   * sends; absent — as with every other field — means leave it alone.
   */
  whatsNewSeen?: string;
  [key: string]: unknown;
}

/**
 * What the first screen needs: whether this app has run here before, and which
 * version is running.
 *
 * The two travel together because they are one decision — a first run is shown
 * the introduction, an installation that has run before is shown what changed —
 * and because the version is what the "what's new" pages are keyed to. The
 * version is the *binary's*, the same one the About screen names.
 */
export interface StartupView {
  /** `true` when this app had never opened its database here before this launch. */
  firstRun: boolean;
  version: string;
}

/** One voice the settings screen can offer. */
export interface VoiceOption {
  /** The name as the system reports it, which is what a choice is stored as. */
  name: string;
  /** The locale, e.g. `zh_CN` — the only thing that tells similar voices apart. */
  locale: string;
  /**
   * Whether this voice needs a network connection to speak.
   *
   * Always false on macOS and iOS, where every voice the system lists is on the
   * device. Android offers a network voice and an on-device one for the same
   * locale, and this is the only thing that tells them apart — which matters
   * here, because the app is meant to work with no network at all.
   */
  network: boolean;
}

/**
 * The voices this machine offers, and the one in use.
 *
 * `active` is what a choice *resolved to*, which is not always what was chosen:
 * a preference naming a voice this machine does not have falls back to the
 * automatic choice, and the screen says so rather than showing the stored name
 * as though it were in use.
 */
export interface VoicesView {
  available: VoiceOption[];
  active: string | null;
}

/**
 * One thing the board is asking for: a single character, or a word written one
 * character at a time. Both the vocabulary list and a review session hand the
 * board a queue of these, so practice has one path.
 */
export interface PracticeItem {
  text: string;
  /** The vocabulary entry this came from, or null for a course character. */
  entryId: number | null;
  pinyin: string;
  meaning: string;
}

// ---- what the app is, and what it ships under ------------------------------

/** The app's own identity, shown on the About screen. */
export interface AppInfo {
  name: string;
  version: string;
  identifier: string;
  licence: string;
  copyright: string;
  repository: string;
}

/**
 * One licence or attribution notice, with its full text.
 *
 * `covers` says what the app takes from that work and `bundlePath` names where
 * the plain-text copy lives inside the application bundle, so a reader can
 * check either without leaving the screen. The text itself is compiled into the
 * binary, so it is present even if the bundle's resource copy were lost.
 */
export interface LicenceNotice {
  id: string;
  title: string;
  licence: string;
  source: string;
  covers: string;
  /** Repository-relative path the text came from. */
  file: string;
  /** Where the copy in the application bundle sits. */
  bundlePath: string;
  text: string;
}

// ---- tone practice ---------------------------------------------------------

/** How a spoken syllable was judged. */
export type ToneVerdict = "match" | "off_target" | "uncertain";

/**
 * The result of scoring one spoken syllable.
 *
 * `contour` and `reference` are both on the same 0..1 scale — low pitch at 0 —
 * so they can be drawn over each other as two lines. `reference` is the expected
 * tone's canonical shape and is always present; `contour` is empty when nothing
 * could be heard, which is the only case where there is no learner's line to
 * draw.
 *
 * `score` is 0..100 on the same scale as a handwriting score, and `grade` is the
 * same band, so a tone and a stroke can be styled alike.
 */
export interface ToneAttempt {
  expectedTone: number;
  /** The tone the contour looks most like, or null when there was no contour. */
  heardTone: number | null;
  score: number;
  grade: Grade;
  verdict: ToneVerdict;
  /** One plain sentence, worded by the Rust side. Show this, do not reword it. */
  detail: string;
  contour: number[];
  reference: number[];
  medianHz: number;
  rangeSemitones: number;
  voicedMs: number;
  spanMs: number;
}

/**
 * One syllable of a word, with the tone to score it against.
 *
 * `citation` is what a dictionary prints and `spoken` is what is actually said:
 * they differ when tone sandhi applies, which is why both are sent. 你好 is
 * `3 + 3` in a dictionary and `2 + 3` out loud.
 */
export interface TargetSyllable {
  ch: string;
  /** The reading as a dictionary writes it, tone mark included. */
  reading: string;
  citation: number;
  spoken: number;
}

/** The tones a character or word should be practised with. */
export interface ToneTarget {
  syllables: TargetSyllable[];
  /** True when sandhi changed a tone, so the interface can say why. */
  sandhiApplied: boolean;
  detail: string;
}

/**
 * What the microphone can do with the text currently on the board.
 *
 * Two independent answers, because the two halves of an attempt can run
 * separately. `tone` is null for text longer than a word, for a reading that
 * will not divide one syllable per character, and for characters the dataset
 * does not know. `recognize` says whether a recognition model is installed,
 * which is what makes a recording worth taking when there is no tone target.
 *
 * The microphone is offered when either is true. Text where neither holds is the
 * one case a button would produce nothing for, and `sayBlocked` says so.
 */
export interface SpeechTarget {
  tone: ToneTarget | null;
  recognize: boolean;
}

/** One syllable's worth of a scored utterance. */
export interface ToneSyllableResult extends TargetSyllable {
  /** Which syllable this is, counting from 1. */
  position: number;
  attempt: ToneAttempt;
}

/**
 * A judged character, word or longer phrase.
 *
 * `syllables` has one entry per syllable of the target, whatever was heard, so
 * the interface can pair them with the characters without guessing — **unless no
 * tone was scored at all**, which is what `toneScored` reports. For text longer
 * than a word the pitch is not measured, `syllables` is empty, and `detail` says
 * why rather than carrying a judgement. Do not read a score from an empty list:
 * zero would look like a perfectly flat attempt instead of an unmeasured one.
 */
export interface ToneResult {
  syllables: ToneSyllableResult[];
  /**
   * True when the pitch was measured, so `syllables` holds one judgement per
   * syllable. False when the text was too long to divide into syllables and the
   * recording was recognised instead: the panel then shows the transcription
   * alone, with no tone header and no charts.
   */
  toneScored: boolean;
  verdict: ToneVerdict;
  /** Mean of the scoreable syllables, 0..100; 0 when none could be scored. */
  score: number;
  grade: Grade;
  /** One plain sentence, worded by the Rust side. Style it; do not reword it. */
  detail: string;
  sandhiApplied: boolean;
  /**
   * Where the syllables were divided, in ms **from the start of speech**.
   * Empty for a single syllable. Same baseline as `voicedMs`, so a boundary can
   * be read against it — the recording itself starts whenever the button was
   * pressed, which is mostly the learner's own pause.
   */
  boundariesMs: number[];
  voicedMs: number;
  spanMs: number;
  medianHz: number;
  /**
   * What a recognition model heard, when one is installed; `null` otherwise.
   *
   * `null` is the ordinary case — it is how the app ships, and every learner who
   * has not installed the model sees it on every attempt. Render nothing at all
   * for `null`: no empty box, no hint that something is missing, and above all no
   * prompt to download anything.
   */
  heard: Heard | null;
  /**
   * Why recognition failed, when a model *is* installed and could not be used.
   *
   * Distinct from `heard: null`: the tone score is still good, and this says the
   * other half of the panel could not be filled in. Show it beside the tone
   * verdict, not instead of it.
   */
  heardError: string | null;
}

/**
 * One syllable of a transcription, beside the one the exercise asked for.
 *
 * Both readings are **plain letters with the tone mark removed** (`shi`, not
 * `shì`). That is not a simplification: a recogniser's output implies a tone that
 * the learner may never have produced — the language model repairs a wrong tone
 * toward the likely word — so the dictionary tone of a transcribed character is
 * evidence about the model, not about the voice. The tone comes from the pitch
 * and from nowhere else. Do not put a tone mark on these.
 */
export interface HeardSyllable {
  /** The syllable as heard: `shi`. */
  base: string;
  /** The syllable the exercise asked for: `si`. */
  wanted: string;
  /** True when they are the same sound once the tone is set aside. */
  matches: boolean;
}

/**
 * What a speech recogniser made of one recording, read against the target.
 *
 * This answers *which syllables were said*, never *how well*. It is a
 * transcription, and the sentence in `detail` is written by the Rust side to keep
 * it from being read as a pronunciation score — show that sentence rather than
 * inventing a shorter one.
 */
export interface Heard {
  /** What was transcribed, in characters. Empty when nothing was recognised. */
  text: string;
  /** The same as plain letters with the syllables spaced: `shi shi`. */
  base: string;
  syllables: HeardSyllable[];
  /** How many syllables were the sound asked for. */
  matched: number;
  /** True when the transcription had as many syllables as the target. */
  sameCount: boolean;
  /** One plain sentence, worded by the Rust side. */
  detail: string;
}

/**
 * Whether a recognition model is installed, and how to describe one that is not.
 *
 * This is also how the settings screen follows a download: it carries both the
 * byte count of the model and how much of it has arrived, so one poll answers
 * "what is happening" and "how far along is it".
 */
export interface AsrStatus {
  /** `absent` — never asked for; `downloading`; `installed`; `failed`. */
  state: "absent" | "downloading" | "installed" | "failed";
  installed: boolean;
  /** The model's name, always present so the screen can describe it up front. */
  model: string;
  /** Where it comes from: the app's only network access. */
  url: string;
  licence: string;
  licenceUrl: string;
  /** How large the download is, in bytes. */
  downloadBytes: number;
  /** How large it becomes once unpacked, in bytes. */
  unpackedBytes: number;
  /** Bytes fetched so far, while `state` is `downloading`. */
  downloaded: number;
  /** Where it is on disk, once it is anywhere. */
  path: string | null;
  /** Why the last attempt failed, when one did. */
  error: string | null;
  /** One plain sentence, worded by the Rust side. */
  detail: string;
}

/**
 * Whether the microphone can be used.
 *
 * `available` cannot see a *denied permission*: the system still hands out a
 * device and the stream then delivers silence. That shows up as a tone attempt
 * with no contour, which is reported as "I could not hear enough voice".
 */
export interface MicrophoneStatus {
  available: boolean;
  device: string | null;
  sampleRate: number;
  detail: string;
}

/**
 * What one sync did, in the shape the settings screen reads.
 *
 * `leftAlone` is the honest one: a character whose schedule this device cannot
 * rebuild from its log — because the log does not go back to its first attempt —
 * is left as it is rather than guessed at. See ROADMAP M13.
 */
export interface SyncSummaryView {
  published: number;
  pulled: number;
  recomputed: number;
  leftAlone: number;
  /** Vocabulary entries and groups whose stored form changed. */
  vocabChanged: number;
  /** Whether the place in the course moved on this device. */
  cursorMoved: boolean;
}

/**
 * Whether this device is connected to Dropbox, and what to say about it.
 *
 * `canConnect` is false on a platform with no secure store for the refresh token.
 * The screen asks before it offers a button, so a learner is told why rather than
 * watching a button fail — the token is never written somewhere it could be read.
 *
 * `canLock` is the same question about a different capability: whether this device
 * can ask for a fingerprint — or a face, or the device PIN — at all. A phone with no
 * screen lock cannot, so it answers no and the switch is not offered there rather than
 * offered and ignored.
 */
export interface SyncView {
  connected: boolean;
  /** Which Dropbox account, when Dropbox said. */
  accountId: string | null;
  canConnect: boolean;
  /** Whether this platform can ask for a fingerprint at all. */
  canLock: boolean;
  /** Whether the learner has asked for one. */
  locked: boolean;
  /** How the stored sign-in is protected. */
  protection: SyncProtection;
  /** The last sync this session, if there has been one. */
  last: SyncSummaryView | null;
  /** One plain sentence, worded by the Rust side. */
  message: string;
}

/**
 * How well the stored Dropbox sign-in is protected.
 *
 * `deviceOnly` is the default and the point of it is that it asks nothing: the item
 * is encrypted at rest, readable only by this app, and not carried to the learner's
 * other devices, but the system releases it without a prompt. `userPresence` is the
 * same item with a fingerprint — a face, a print, or the device password — asked for
 * when the token is about to be used, which is once per run of the app and never
 * merely to draw this screen. `keychainOnly` means an item released without asking
 * anybody, which is what a Mac whose build the system cannot identify falls back to
 * and what an Android phone with no screen lock can only ever have; on a Mac it is
 * also the one case that may ask for the login keychain password instead.
 * `unknown` means nothing is stored.
 */
export type SyncProtection =
  | "unknown"
  | "deviceOnly"
  | "userPresence"
  | "keychainOnly";

/**
 * What an automatic sync did, or why it did not run.
 *
 * `skipped` is the ordinary answer on most launches — nothing is connected, or the
 * sign-in is behind a fingerprint and a sync that runs by itself has nobody to ask —
 * and it is deliberately **silent**: both of those are already said on the settings
 * screen, and saying them on every launch would be noise nobody can switch off.
 * `offline` is a phone on a train rather than anything going wrong, so it is worth
 * one calm line and no alarm. Only `failed` is worth alarming anybody about: an
 * automatic sync that fails quietly is a device falling out of step.
 */
export type AutoSync =
  | { outcome: "skipped"; reason: string }
  | { outcome: "offline"; reason: string }
  | { outcome: "synced"; view: SyncView }
  | { outcome: "failed"; reason: string };

// ---- Graded phrase audio -------------------------------------------------

/**
 * One phrase's two clips, as root-relative URLs the webview can play.
 *
 * Produced by `scripts/synthesize-audio.py`'s Rust replacement, whose manifest
 * is written to `public/audio/<source>/manifest.json`. See
 * `crates/hanzi-say/src/bin/synthesize_audio.rs`.
 */
export interface PhraseAudio {
  normal: string;
  slow: string;
}

/** One graded phrase with its clips. */
export interface GradedPhrase {
  id: string;
  /**
   * HSK level for the graded sentences, or a reader shelf name
   * (`newbie`, `beginner`, …) for the graded readers. A string rather than a
   * number because the two corpora do not use the same scale.
   */
  level: string;
  text: string;
  pinyin: string;
  translation: string;
  audio: PhraseAudio;
  /** Total bytes of both clips, for the size line on the licences screen. */
  bytes: number;
}

/**
 * One corpus's clips.
 *
 * The two corpora are shipped and read separately because their licences differ
 * — `no7z` is CC BY-SA 4.0 and `harukicoder` is CC BY 4.0 — so a merged list
 * would make it easy to attribute one set under the other's notice.
 */
export interface AudioManifest {
  source: string;
  /** Model directory that produced the clips, for provenance. */
  model: string;
  /** Speeds written, normal first (e.g. `[1.0, 0.7]`). */
  speeds: number[];
  phrases: GradedPhrase[];
}

// ---- On-device speech synthesis -----------------------------------------

/**
 * How the synthesis model is doing, and what it would cost to install.
 *
 * Deliberately shaped like [`AsrStatus`]: both are an optional model the learner
 * downloads from the settings screen, and a second screen inventing its own
 * vocabulary for the same idea would be two things to learn instead of one.
 * The difference is the size — this is about 58 MB against the recogniser's
 * 163 MB — and that it arrives as a handful of files rather than one archive.
 */
export interface SayStatus {
  /** `absent` — never asked for; `downloading`; `installed`; `failed`. */
  state: "absent" | "downloading" | "installed" | "failed";
  /** True when the model is on disk and usable. */
  installed: boolean;
  /** What the model is called, so the screen can name it up front. */
  name: string;
  /** Total bytes to fetch. */
  bytes: number;
  /** The licence the weights are under, stated before the download. */
  licence: string;
  /** Bytes fetched so far, while `state` is `downloading`. */
  downloaded: number;
  /** One plain sentence, worded by the Rust side. */
  detail: string;
}

/**
 * What the synthesiser produced for one phrase: mono samples and their rate.
 *
 * Samples rather than an encoded file because the frontend already has a
 * decoder, and because a raw buffer needs no container.
 */
export interface SpokenAudio {
  samples: number[];
  sampleRate: number;
}
