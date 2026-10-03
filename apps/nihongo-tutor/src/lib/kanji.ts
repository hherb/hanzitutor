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

import type { KanjiLessonView, RadicalFamilyView } from "./types";

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
