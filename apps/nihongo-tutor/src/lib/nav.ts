/**
 * The two courses, and which screens belong to each.
 *
 * The app is **not** one list of screens with a kanji tab in it. Kana are the
 * on-ramp — 177 characters, a few days' work — and the kanji are the product, the
 * thing the years go into. So the interface is two courses, one of which is shown
 * at a time, and this file is that structure as plain data so it can be asserted
 * on rather than read out of a template.
 *
 * The division is a partition with two deliberate exceptions, and both are stated
 * here rather than left to whoever edits the template next:
 *
 * - **Review is in both courses.** The queue is asked for one section at a time
 *   (see `api.reviewQueue`), because the schedule is one file of characters and a
 *   kana screen that listed due kanji would be describing a queue it cannot show.
 * - **Licences is in neither.** It is an obligation of the app rather than a thing
 *   the app teaches, so it hangs off the footer instead of taking a tab away from
 *   a course.
 */

/** Which half of the app is open. Matches `nihongo_core::review::Section`. */
export type Section = "kana" | "kanji";

/** The screens of the kana course. */
export type KanaView = "practice" | "chart" | "drill" | "review";

/** The screens of the kanji course. */
export type KanjiView = "kanji" | "radicals" | "words" | "read" | "review";

/** Every screen, including the one that belongs to neither course. */
export type View = KanaView | KanjiView | "licences";

/** One tab, as the row under the course switch draws it. */
export interface Tab {
  readonly id: View;
  readonly label: string;
}

/** One course: its name, what it is, and its own screens in order. */
export interface Course {
  readonly id: Section;
  readonly label: string;
  /**
   * The few words the switch puts beside the counts — the line that says what the
   * course *is*, where the counts say how big it is.
   */
  readonly tagline: string;
  /** The one sentence under the label where there is room for a sentence. */
  readonly blurb: string;
  readonly tabs: readonly Tab[];
}

/**
 * The courses, in the order the switch shows them: the on-ramp, then the product.
 *
 * The kana first is not a judgement about which matters — it is the order a
 * learner meets them in, and the switch is a choice rather than a sequence.
 */
export const COURSES: readonly Course[] = [
  {
    id: "kana",
    label: "Kana",
    tagline: "The on-ramp",
    blurb: "The on-ramp: learn to read and write the kana in a few days.",
    tabs: [
      { id: "practice", label: "Practice" },
      { id: "chart", label: "Chart" },
      { id: "drill", label: "Tell them apart" },
      { id: "review", label: "Review" },
    ],
  },
  {
    id: "kanji",
    label: "Kanji",
    tagline: "The main course",
    blurb: "The main course: 2,136 jōyō characters, taught through words.",
    tabs: [
      { id: "kanji", label: "Kanji" },
      { id: "radicals", label: "Radicals" },
      { id: "words", label: "Words" },
      { id: "read", label: "Read" },
      { id: "review", label: "Review" },
    ],
  },
];

/** The course a section names, with a caller's string narrowed to it. */
export function courseOf(section: Section): Course {
  const course = COURSES.find((c) => c.id === section);
  // Unreachable while `Section` is the two values above: the lookup is total by
  // construction, so a miss is a programming error rather than a state to draw.
  if (!course) throw new Error(`unknown section ${section}`);
  return course;
}

/** The tabs of one course, in the order the row draws them. */
export function tabsOf(section: Section): readonly Tab[] {
  return courseOf(section).tabs;
}

/** The screen a course opens on — the first of its own tabs. */
export function defaultView(section: Section): View {
  const first = courseOf(section).tabs[0];
  if (!first) throw new Error(`${section} has no screens`);
  return first.id;
}

/** Whether this screen is one of the course's own. */
export function covers(section: Section, view: View): boolean {
  return courseOf(section).tabs.some((tab) => tab.id === view);
}

/** Whether a string from outside (a stored preference) names a course. */
export function isSection(value: unknown): value is Section {
  return value === "kana" || value === "kanji";
}
