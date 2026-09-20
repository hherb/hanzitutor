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
}

/** How many words one HSK level holds. */
export interface LevelCount {
  level: number;
  words: number;
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
}

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
  [key: string]: unknown;
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

/** One syllable's worth of a scored utterance. */
export interface ToneSyllableResult extends TargetSyllable {
  /** Which syllable this is, counting from 1. */
  position: number;
  attempt: ToneAttempt;
}

/**
 * A scored character or word.
 *
 * `syllables` always has one entry per syllable of the target, whatever was
 * heard, so the interface can pair them with the characters without guessing.
 */
export interface ToneResult {
  syllables: ToneSyllableResult[];
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
 * `canLock` is the same question about a different capability: whether this platform
 * can ask for a fingerprint at all. Android can keep the sign-in and cannot yet ask,
 * so the switch is not offered there rather than offered and ignored.
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
 * merely to draw this screen. `keychainOnly` means an ordinary login-keychain item,
 * which is where a build the system cannot identify lands, and the one case that can
 * ask for the login keychain password instead. `unknown` means nothing is stored.
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
