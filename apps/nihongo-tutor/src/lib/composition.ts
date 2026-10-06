/**
 * The composition exercise's arithmetic, as pure functions.
 *
 * The same split the other screens use: what can be decided without a window lives
 * here and is unit-tested by the root project's vitest, and the component only draws.
 *
 * The one thing these functions exist to keep straight is that **there is no answer**.
 * `checks` turns the verdict Rust sends into the lines the learner reads, and it never
 * says "right" or "wrong": the pattern's own text is either in the sentence or not, and
 * the second case is stated with the reason it can happen to a good sentence (the
 * corpus's own examples inflect their patterns). `limitationNote` is the line that owes
 * the learner the rest of the truth — what the check cannot see at all — and
 * `keptLine` is the only place a decision is reported, which is the one Rust made:
 * whether the input was a sentence.
 */

import type { CompositionSize, CompositionVerdict } from "./types";

/**
 * The shortest thing the exercise keeps, in kana/kanji characters.
 *
 * Mirrors `nihongo_core::composition::MIN_COMPOSITION`, which is the authority —
 * Rust decides, and `verdict.kept` is what actually gates the record. It is repeated
 * here only so the line about a short input can name the bar; the number is the
 * corpus's own shortest sentence (私は学生です。), which
 * `tests/grammar_artifact.rs` re-derives from the committed artifact.
 */
export const COMPOSITION_MIN = 6;

/** How one checked fact reads: satisfied, worth knowing, or something to fix. */
export type CheckState = "ok" | "note" | "warn";

/** One line of the checklist. */
export interface Check {
  state: CheckState;
  text: string;
}

/** Whether there is anything to send back. */
export function canSubmit(typed: string): boolean {
  return typed.trim() !== "";
}

/** What the learner is being asked for, naming the pattern. */
export function instructionLine(pattern: string): string {
  return `Write a sentence of your own with ${pattern}. Type it in kana, or in romaji and it is read back as kana.`;
}

/**
 * The checklist: what the app could see, each fact stated rather than scored.
 *
 * Five things, and only one of them can be a problem the learner has to fix — an
 * input that is not Japanese script. The pattern's absence is deliberately a `note`
 * rather than a `warn`: the check looks for the pattern's own letters, and the
 * corpus's own 1,526 sentences include 255 that inflect their pattern (〜ている is
 * taught with しています), so a good sentence can legitimately come back without it.
 */
export function checks(verdict: CompositionVerdict, typed: string): Check[] {
  const out: Check[] = [];
  const asTyped = typed.trim();
  if (verdict.text !== "" && verdict.text !== asTyped) {
    // No full stop after it: the sentence brings its own, and a kana sentence with a
    // `。` and an English-looking `.` after it reads as two.
    out.push({ state: "note", text: `Read back as: ${verdict.text}` });
  }
  if (!verdict.readable) {
    out.push({
      state: "warn",
      text: "Some of what you wrote is not Japanese script, so it could not be read as kana. Write the whole sentence in kana, or the whole of it in romaji.",
    });
  } else if (verdict.characters < COMPOSITION_MIN) {
    out.push({
      state: "warn",
      text: `That is ${verdict.characters} characters of Japanese. A sentence needs at least ${COMPOSITION_MIN} — this is a word rather than a sentence.`,
    });
  } else {
    out.push({ state: "ok", text: `Readable as Japanese: ${verdict.characters} characters.` });
  }
  if (verdict.found.length > 0) {
    out.push({
      state: "ok",
      text: `The pattern's own text is in your sentence: ${verdict.found.slice(0, 3).join(" · ")}.`,
    });
  } else {
    out.push({
      state: "note",
      text: `The pattern's own text (${verdict.pattern}) is not in your sentence. That is not necessarily wrong: this check looks for the pattern's own letters, and the corpus's own sentences inflect it — 〜ている is taught with しています.`,
    });
  }
  if (verdict.outside.length > 0) {
    out.push({
      state: "note",
      text: `These characters are not ones this course teaches, so this app has no card behind them: ${verdict.outside.join(" ")}.`,
    });
  }
  if (verdict.copied !== null) {
    out.push({
      state: "note",
      text: "This is one of the corpus's own sentences for this pattern, written back — a model rather than a sentence of your own.",
    });
  }
  return out;
}

/**
 * What happened to the sentence: the one decision the app made.
 *
 * Kept means it was a sentence and is in the learner's own file; not kept means it was
 * not a sentence, and the reason is the script or the length. Nothing here is a mark.
 */
export function keptLine(verdict: CompositionVerdict, count: number): string {
  if (!verdict.kept) {
    return verdict.readable
      ? "Not kept: it is shorter than a sentence."
      : "Not kept: it could not be read as Japanese.";
  }
  const noun = count === 1 ? "sentence" : "sentences";
  return `Kept — that is ${count} ${noun} of your own for this pattern.`;
}

/**
 * What the check cannot see, which the screen owes the learner.
 *
 * N11's lesson is the reason it is a function with a test rather than a sentence typed
 * into a component: prose about a language is the one thing here no other test reads.
 */
export function limitationNote(): string {
  return "This checks what a machine can see: the pattern's own letters, the script and the length. It cannot tell whether a sentence is grammatical or natural, or whether the pattern is used in the right sense — that needs a reader.";
}

/** How much of the course can be written about, in one sentence. */
export function poolNote(size: CompositionSize): string {
  if (size.total <= 0 || size.points <= 0) {
    return "No pattern in this course can be written about yet.";
  }
  if (size.points >= size.total) {
    return `Every one of the ${size.total} patterns can be written about — the whole course, because the prompt is the pattern itself and needs no sentence of it.`;
  }
  return `${size.points} of the ${size.total} patterns can be written about.`;
}

/**
 * What the learner's own sentences for this pattern are, stated.
 *
 * Empty when there are none, so the screen draws no heading over an empty list.
 */
export function ownNote(written: string[]): string {
  if (written.length === 0) return "";
  if (written.length === 1) return "The one sentence you have written with this pattern:";
  return `The ${written.length} sentences you have written with this pattern, oldest first:`;
}
