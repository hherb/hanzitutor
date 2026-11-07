/**
 * The kanji course's and the radicals panel's arithmetic, as pure functions.
 *
 * The two screens are lists over the same payloads, and everything that can be
 * got wrong about them — which grade a lesson belongs to, how big a grade is,
 * which lesson holds a character another screen asked for, how the 214 radicals
 * are ordered and searched — is arithmetic rather than drawing. So it is here,
 * where `vitest` runs it without a window, and the components only render what
 * these return. `words.ts` and `board.ts` are the same arrangement for the
 * furigana and the board.
 */

import type { KanjiLessonView, KanjiPick, RadicalFamilyView } from "./types";

/** One grade of the kanji course, as the tabs above the course show it. */
export interface GradeTab {
  grade: number;
  /** "kyōiku 1" … "jōyō beyond the school grades", from the payload. */
  name: string;
  /** How many characters the grade holds. */
  kanji: number;
  lessons: number;
}

/**
 * The grade tabs, derived from the course itself.
 *
 * Derived rather than asked for over IPC: the lessons arrive in grade order and
 * carry their grade and their size, so a second command could only disagree with
 * the first. The order is the payload's, which is the teaching order.
 */
export function gradeTabs(lessons: KanjiLessonView[]): GradeTab[] {
  const tabs: GradeTab[] = [];
  for (const lesson of lessons) {
    const found = tabs.find((tab) => tab.grade === lesson.grade);
    if (found) {
      found.kanji += lesson.count;
      found.lessons += 1;
    } else {
      tabs.push({
        grade: lesson.grade,
        name: lesson.gradeName,
        kanji: lesson.count,
        lessons: 1,
      });
    }
  }
  return tabs;
}

/** The lessons of one grade, in teaching order. */
export function lessonsIn(lessons: KanjiLessonView[], grade: number): KanjiLessonView[] {
  return lessons.filter((lesson) => lesson.grade === grade);
}

/**
 * The lesson a character belongs to, or null when the course does not teach it.
 *
 * This is what makes a tap on a component, a radical's family member or a word's
 * character land on the right lesson instead of only on the right board.
 */
export function lessonHolding(lessons: KanjiLessonView[], ch: string): KanjiLessonView | null {
  return lessons.find((lesson) => lesson.kanji.includes(ch)) ?? null;
}

/**
 * The character `delta` places from `ch` inside a lesson — the arrow either side
 * of the board.
 *
 * `null` at either end rather than wrapping: the ten characters of a lesson are a
 * sequence a learner works through, and an arrow that jumped from the tenth back
 * to the first would hide the fact that the lesson is finished. A character the
 * lesson does not hold, and a lesson of one, both answer `null` — there is
 * nowhere to step to.
 */
export function neighbour(
  lesson: KanjiLessonView | null,
  ch: string,
  delta: number,
): string | null {
  if (!lesson || delta === 0) return null;
  const at = lesson.kanji.indexOf(ch);
  if (at < 0) return null;
  return lesson.kanji[at + delta] ?? null;
}

/** Where `ch` sits in its lesson, 1-based, for a screen that says so. */
export function positionIn(lesson: KanjiLessonView | null, ch: string): number {
  if (!lesson) return 0;
  const at = lesson.kanji.indexOf(ch);
  return at < 0 ? 0 : at + 1;
}

/**
 * Whether `place` is one of the stage's own — somewhere the stage's navigation
 * describes — rather than a detour from it.
 *
 * Inside a lesson that is any of the lesson's characters: the arrows and the
 * `3 / 10` are about those, and nothing else. A stage with no lesson (a radical
 * head form, or a character no lesson holds) has exactly one place of its own,
 * the one it was opened on.
 *
 * A component chip and a radical-family member can both put a character on the
 * board that is *not* home: 林 is in its lesson and 木, tapped from it, is not.
 * Issue #34 was the stage treating that character as if it were — `0 / 10` with
 * both arrows dead, and `Finish` after grading it.
 */
export function isHomePlace(
  lesson: KanjiLessonView | null,
  start: KanjiPick,
  place: KanjiPick,
): boolean {
  if (lesson) return place.kind === "kanji" && lesson.kanji.includes(place.ch);
  return samePick(place, start);
}

/**
 * Where to come back to after leaving `leaving` for somewhere else.
 *
 * The last home the board held: leaving a lesson character makes it the anchor,
 * and leaving a detour (木 for one of its own parts) keeps the anchor it had — so a
 * chain of taps still comes back to the character of the lesson it started from.
 */
export function anchorAfter(
  lesson: KanjiLessonView | null,
  start: KanjiPick,
  leaving: KanjiPick,
  anchor: KanjiPick,
): KanjiPick {
  return isHomePlace(lesson, start, leaving) ? leaving : anchor;
}

/**
 * The way back the stage offers, or `null` when there is none to offer.
 *
 * Only off home — on a lesson character the arrows are the way on — and only to a
 * place that is home, so the control can never lead into a second detour.
 */
export function wayBack(
  lesson: KanjiLessonView | null,
  start: KanjiPick,
  here: KanjiPick,
  anchor: KanjiPick,
): KanjiPick | null {
  if (isHomePlace(lesson, start, here)) return null;
  return isHomePlace(lesson, start, anchor) ? anchor : null;
}

/** Whether two picks name the same place on the board. */
function samePick(a: KanjiPick, b: KanjiPick): boolean {
  if (a.kind === "kanji") return b.kind === "kanji" && a.ch === b.ch;
  return b.kind === "radical" && a.number === b.number;
}

/** How the radicals panel orders the 214. */
export type RadicalOrder = "number" | "size";

/**
 * The families in the order the panel shows them.
 *
 * `number` is the Kangxi order, which is the one a dictionary uses; `size` is
 * the order to learn in, because a radical that unlocks a hundred characters is
 * worth meeting before one that appears in a single character. Ties in size fall
 * back to the number, so the order is total and does not move between renders.
 */
export function sortFamilies(
  families: RadicalFamilyView[],
  order: RadicalOrder,
): RadicalFamilyView[] {
  const sorted = [...families];
  if (order === "size") {
    sorted.sort(
      (a, b) => b.characters.length - a.characters.length || a.number - b.number,
    );
  } else {
    sorted.sort((a, b) => a.number - b.number);
  }
  return sorted;
}

/**
 * The families a search box keeps.
 *
 * Three ways in, because a learner arrives with any of the three: the head form
 * itself (手), the classical number (64), or a character they met and want to
 * take apart (持) — the last finds the family by its members, which is the one
 * that answers "what is this made of".
 */
export function findFamilies(
  families: RadicalFamilyView[],
  query: string,
): RadicalFamilyView[] {
  const text = query.trim();
  if (text === "") return families;
  return families.filter(
    (family) =>
      family.ch === text ||
      String(family.number) === text ||
      family.characters.includes(text),
  );
}

/**
 * A reading as the synthesiser should say it.
 *
 * KANJIDIC2 marks its readings rather than writing them as prose, and the two
 * marks mean different things:
 *
 * * a **`.`** separates the stem from its okurigana and is always in the middle —
 *   `た.べる` is 食べる, spoken たべる;
 * * a **`-`** marks an affix that is not used on its own and is always at one end
 *   — `ひと-` trails, and 259 kun readings plus four on-readings lead with it
 *   (`-ノウ` on 応 and 王, `-ネン` on 縁, `-ノン` on 音).
 *
 * Both come off for speech and **neither comes off in the artifact**: the marks
 * are what tell a learner that 食べる is written with kana attached, which is why
 * `kanji_artifact.rs` keeps them and pins their exact shapes.
 *
 * Measured over all 9,364 readings — pinned by
 * `every_reading_is_kana_once_its_markers_are_stripped` in that file — the marks
 * are the *only* characters that are not kana, so this is the whole of the
 * transformation: strip them and what is left is a reading a Japanese voice can
 * say, with nothing guessed at and nothing spelled out.
 */
export function spokenReading(reading: string): string {
  return reading.replace(/[.-]/g, "");
}
