/**
 * The pages of the startup sheet: the introduction, and what's new.
 *
 * Both are the same kind of thing — a handful of pages read once and then
 * dismissed — so they are the same shape, are rendered by the same component
 * (`StartupWizard.svelte`), and live here as **data rather than markup**. That is
 * what keeps the page count, the "Step n of m" and the dots from disagreeing with
 * the pages: adding one is adding an entry to an array.
 *
 * ## Which one is shown
 *
 * `App.svelte` decides, from two facts the backend reports together
 * (`commands::startup`): whether the app has run on this device before, and which
 * version is running. A **first run** gets [`INTRO_PAGES`]; an **installation that
 * has run before** gets the notes for the running version, because the tutorial is
 * not news to somebody who has been using the app. Either way the sheet's "has
 * been read" records are written together — see `App.svelte`'s `finishStartup`.
 *
 * ## The names are the app's own
 *
 * The four measures and the two modes are called here exactly what the board's
 * buttons and the report call them, because this is the page that teaches them.
 * Changing one here without the others is how the app ends up with two names for
 * one thing.
 */

/** One labelled idea inside a page: a mode, a grading measure, a screen. */
export interface Point {
  /** The word the app itself uses for it, so the two agree. */
  name: string;
  /** One sentence on what it does. */
  what: string;
}

/** One page of the startup sheet. */
export interface Page {
  title: string;
  /** The paragraph a page opens with. */
  lead: string;
  /** Labelled ideas, listed under the lead. */
  points?: Point[];
  /** A closing paragraph, under the list. */
  note?: string;
}

/**
 * What a learner who has never seen the app is shown.
 *
 * This is where the board's own explanation went. The "ⓘ How this works"
 * disclosure and the sentence under the controls were removed to give a phone
 * back the height they took, on the understanding that what they taught is read
 * **once at the start** rather than sitting on the screen for ever.
 *
 * Deliberately no keyboard-shortcut list: the board's keys cannot be tried while
 * a sheet covers the board, so that list is the `?` card's job — a card that
 * leaves the board visible (`ShortcutCard.svelte`).
 */
export const INTRO_PAGES: Page[] = [
  {
    title: "Welcome to Hanzi Tutor",
    lead:
      "This app teaches you to write simplified Chinese by hand. You write a " +
      "character with a finger, a stylus or a mouse, and your writing is graded " +
      "against a reference — not merely recognised, so it can tell you how it " +
      "was wrong and not only that it was.",
    note:
      "Everything happens on this machine. Nothing you write leaves it, and " +
      "nothing is downloaded unless you ask for it.",
  },
  {
    title: "Two ways to practise",
    lead:
      "The Hint button at the left of the board switches between two ways of " +
      "practising the same character. Either way, switching clears the board.",
    points: [
      {
        name: "Trace",
        what:
          "A faint copy of the character is on the board. Follow it, stroke by " +
          "stroke, in the correct order.",
      },
      {
        name: "Recall",
        what:
          "The character is hidden until you press ✓. Write it from memory, " +
          "then see how you did.",
      },
    ],
    note:
      "The faint copy is there to write over, not to copy by eye: the grading " +
      "is the same in both modes, so tracing loosely is not a way to score " +
      "better.",
  },
  {
    title: "What your writing is graded on",
    lead:
      "Every attempt is compared with the reference on four measures, a quarter " +
      "of the score each.",
    points: [
      {
        name: "Shape",
        what: "Whether each stroke is the right kind of stroke.",
      },
      {
        name: "Placement",
        what: "Whether it sits in the right place, and is the right size.",
      },
      {
        name: "Ink",
        what:
          "Whether as much ink went down as the character needs. A stroke " +
          "traced correctly but drawn far too thin is not legible.",
      },
      {
        name: "Order",
        what:
          "Whether the strokes were written in the right sequence. A legible " +
          "character written in the wrong order is reported as exactly that.",
      },
    ],
    note:
      "The report after each attempt says which strokes were wrong and how — " +
      "misplaced, out of order, missing, or faint. Press S to watch the " +
      "character drawn one stroke at a time.",
  },
  {
    title: "Finding your way around",
    lead: "The sidebar holds the rest of the app.",
    points: [
      {
        name: "Course",
        what:
          "Characters in frequency order, the ones you will meet most often " +
          "first, in lessons you work through.",
      },
      {
        name: "Vocabulary",
        what:
          "Characters and words from your own lessons, filed under group names " +
          "of your own.",
      },
      {
        name: "Words",
        what: "The HSK 3.0 word list, searchable by character, pinyin or meaning.",
      },
      {
        name: "Phrases",
        what: "Graded sentences with bundled audio, for listening and reading.",
      },
      {
        name: "Settings",
        what:
          "Drawing, stroke-order speed, board size and the pronunciation voice.",
      },
    ],
    note:
      "What you practise is scheduled for you, so it comes back when it is " +
      "worth seeing again: Review due at the top of the sidebar holds the " +
      "characters that are ready.",
  },
];

/**
 * What an installation that has **run before** is shown, keyed by the version
 * whose notes these are.
 *
 * ## Maintaining this
 *
 * **When the version is bumped, add an entry here for the new version** — or
 * deliberately leave it out, which shows nothing. Both are safe; what is not
 * safe is leaving an *old* entry in place and bumping the version, because the
 * sheet would then present the previous release's notes as this one's. The
 * lookup is [`notesFor`], and it shows nothing for a version it has no entry
 * for, so the failure mode of forgetting is silence rather than a lie.
 *
 * `App.svelte` passes the version the *binary* reports, so the key is the same
 * string `Cargo.toml` carries — which `tests/licences.rs` already holds equal to
 * `tauri.conf.json`, `package.json` and the About screen.
 *
 * ## Writing an entry
 *
 * For someone who has been using the app, so: what is different, not what the
 * app is. Anything a learner would not notice does not belong here — it is a
 * short list, and one honest page beats three padded ones.
 */
export const NOTES: Record<string, Page[]> = {
  "0.5.10": [
    {
      title: "What's new in 0.5.10",
      lead:
        "Characters are shown as the parts they are built from. A new Radicals " +
        "screen collects them, and every character's page now says what it is made " +
        "of. Nothing about how your writing is graded has changed, and your " +
        "practice history, schedule and vocabulary are untouched.",
      points: [
        {
          name: "Radicals",
          what:
            "A new screen in the sidebar, ordered by how many characters each part " +
            "unlocks — so the ones worth learning first are at the top. Each row " +
            "gives the part, what it means and the count; open one to see every " +
            "character that uses it.",
        },
        {
          name: "What a character is built from",
          what:
            "A character's page lists its parts in the order they are read and " +
            "names the arrangement: 说 is 讠 and 兑 side by side, 草 is 艹 over 早, " +
            "言 is 亠, 二 and 口 stacked. Every part the board can draw is a button " +
            "that puts it on the board on its own.",
        },
        {
          name: "Practising a family",
          what:
            "From a radical you can write the part itself, or every character that " +
            "uses it, most common first — 言 opens over three hundred of them, " +
            "including 说, 话 and 请.",
        },
      ],
      note:
        "The part a character is classified under is the full dictionary form (言, " +
        "人, 水), while the shape inside the character is that same part written " +
        "small (讠, 亻, 氵). Seeing the two together is the point of the screen.",
    },
  ],
  "0.5.9": [
    {
      title: "What's new in 0.5.9",
      lead:
        "A screen for tone pairs — the characters that differ only in tone, which " +
        "is where most learners actually struggle. Nothing about how your writing " +
        "is graded has changed, and your practice history, schedule and vocabulary " +
        "are untouched.",
      points: [
        {
          name: "Tones",
          what:
            "A new screen in the sidebar. Each row is one syllable at two, three or " +
            "four tones — 妈 mā, 麻 má, 马 mǎ, 骂 mà — placed by how common the " +
            "characters are, with a filter for the pairs worth drilling (2 against " +
            "3, 1 against 4) and a box to find a syllable.",
        },
        {
          name: "Hearing them",
          what:
            "Press Hear for the whole set in order, or the speaker beside one for " +
            "that one alone. Press Quiz and the app says one at random: pick the " +
            "reading you heard. Getting it wrong is the useful part — the answer is " +
            "shown straight away.",
        },
        {
          name: "Saying them",
          what:
            "Press Practise and the set goes to the board in tone order, where the " +
            "microphone and the contour chart already are. Saying 妈 then 麻 then 马 " +
            "then 骂 and watching the four contours is the other half of the drill.",
        },
      ],
      note:
        "The voice is the same system voice the board's Listen button uses, so a " +
        "device with no Chinese voice can still write these on the board but cannot " +
        "hear or quiz them.",
    },
  ],
  "0.5.8": [
    {
      title: "What's new in 0.5.8",
      lead:
        "The board's keyboard shortcuts are written down again. Nothing about how " +
        "your writing is graded has changed, and your practice history, schedule " +
        "and vocabulary are untouched.",
      points: [
        {
          name: "Press ?",
          what:
            "A small card lists every key the board answers to — Enter to check, " +
            "⌫ or ⌘Z to take a stroke back, ← and → to move, S to watch the " +
            "character written, H to hear it, and ? for the list itself. Escape " +
            "closes it.",
        },
        {
          name: "The card does not cover the board",
          what:
            "That is the point of it: a key can be read and then pressed straight " +
            "away, with the card still there. It sits in the corner and takes no " +
            "keys away from you.",
        },
        {
          name: "Keyboard shortcuts in the sidebar",
          what:
            "The same card opens from the bottom of the sidebar, for anybody who " +
            "did not know there was a key for it.",
        },
      ],
    },
  ],
  "0.5.7": [
    {
      title: "What's new in 0.5.7",
      lead:
        "A new screen for looking a character up. Nothing about how your writing " +
        "is graded has changed, and your practice history, schedule and " +
        "vocabulary are untouched.",
      points: [
        {
          name: "Characters",
          what:
            "A new screen in the sidebar, between HSK words and Phrases. It " +
            "searches the whole character set — by the character, by its reading " +
            "with or without tone marks (xue, xué), by an English meaning, or by " +
            "a word you have met, typed as characters (医院) or as its reading " +
            "(yisheng), which reaches the characters it is made of.",
        },
        {
          name: "A character's own page",
          what:
            "Every result opens onto what the character means, how it reads, its " +
            "radical, stroke count, HSK level and frequency place, how well you " +
            "know it, and the HSK words that use it. From there it can be written " +
            "on the board, added to your list, or shown where it sits in the " +
            "course.",
        },
        {
          name: "Levels, and what is outside HSK",
          what:
            "The same HSK levels the word list offers, filtered on the left, plus " +
            "an Outside HSK row: the course teaches thousands of characters no " +
            "HSK list names, and until now they could only be reached by " +
            "scrolling the course.",
        },
      ],
      note:
        "The course is still the way to learn, ten characters at a time. This " +
        "screen is for finding something — the sign you walked past, the word you " +
        "heard — and for seeing what a character is used in.",
    },
  ],
  "0.5.6": [
    {
      title: "What's new in 0.5.6",
      lead:
        "A small update. Nothing about how your writing is graded has changed, " +
        "and your practice history, schedule and vocabulary are untouched.",
      points: [
        {
          name: "The introduction",
          what:
            "The board used to carry an explanation of the four grading " +
            "measures behind a disclosure. It is now a short introduction, read " +
            "once and out of the way afterwards, so the board has its height " +
            "back on a phone. The Settings screen can show it again.",
        },
        {
          name: "A phrase set with no recordings says so",
          what:
            "The Phrases screen listed a corpus whose audio was missing as a " +
            "syntax error. It now says plainly that the set has no recordings, " +
            "which is what the reader actually needs to know.",
        },
      ],
    },
  ],
};

/**
 * The pages to show a device that has already read the notes for `version`, or
 * `null` when there are none to show.
 *
 * `null` covers two cases that want the same answer: a version with no entry,
 * and a version whose entry is empty. The caller shows nothing rather than an
 * empty sheet.
 */
export function notesFor(version: string): Page[] | null {
  const pages = NOTES[version];
  return pages && pages.length > 0 ? pages : null;
}
