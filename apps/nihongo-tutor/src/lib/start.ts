/**
 * The Start screen's few facts and its one sentence, as pure functions.
 *
 * The screen argues that the characters are worth learning because a reading
 * alone does not identify a word, and it argues from the app's own vocabulary
 * rather than from a typed-in example: the readings below are queried against
 * `words.bin.gz`, and every word drawn arrives with the dictionary's own reading
 * and gloss (invariant 21). What is left for the frontend is which readings to
 * ask about, how many words a page draws, and the sentence that states a count —
 * all of it here, so it can be asserted without a window.
 */

/**
 * The readings the screen demonstrates with.
 *
 * Both are the textbook case and both are real in this vocabulary: はし is 橋
 * (bridge), 箸 (chopsticks) and 端 (edge), and かみ is five words including 紙
 * (paper), 神 (god) and 髪 (hair). They are **kana**, deliberately: the command
 * rejects anything else, because a reading is written in kana and a romaji query
 * would come back empty for a reason that is not about the language.
 */
export const ALIKE_READINGS = ["はし", "かみ"] as const;

/** How many words of one reading the screen draws; any beyond it are counted. */
export const ALIKE_PAGE = 12;

/**
 * What to say about a reading, given how many words carry it.
 *
 * Three shapes, and the third is the one worth having: a reading this course does
 * not carry is stated rather than drawn as an empty list — invariant 28's rule for
 * a character in no word, one command over — and a reading with more words than a
 * page says so instead of looking complete at twelve.
 */
export function alikeNote(total: number, shown: number, reading: string): string {
  const count = total.toLocaleString();
  if (total === 0) return `Nothing this course teaches is read ${reading}.`;
  if (total === 1) return `One word this course teaches is read ${reading}.`;
  if (total <= shown) return `${count} words this course teaches are read ${reading}.`;
  return `${count} words this course teaches are read ${reading}; the first ${shown} are below.`;
}
