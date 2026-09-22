/**
 * The board's keyboard shortcuts: the keys, what they do, and the one place the
 * handler and the card both read them from.
 *
 * ## Why this is data
 *
 * Two things have to agree about a shortcut: the code that runs it
 * (`App.svelte`'s key handler) and the card that lists it
 * (`ShortcutCard.svelte`). They are the two halves of one fact, so the keys live
 * here rather than in a comment beside the handler, and the card is a rendering
 * of this array. Adding a key is adding an entry — and because the handler's
 * `switch` is exhaustive over [`ShortcutId`], an entry with no action is a
 * compile error rather than a key that silently does nothing.
 *
 * ## What belongs here
 *
 * Only keys that work **on the board**. The list was kept out of the
 * introduction on purpose (see `startupPages.ts`): a sheet that covers the board
 * must not name keys the reader cannot try. The card is not that — see its own
 * note — but this file must still not promise a key from a screen it does not
 * work on, which is why every entry says what it does *to the board*.
 */

/** What one key does. The handler switches on this. */
export type ShortcutId = "check" | "undo" | "previous" | "next" | "strokes" | "hear" | "help";

export interface Shortcut {
  id: ShortcutId;
  /** The keys as the card writes them, e.g. `"⌫ or ⌘Z"`. */
  keys: string;
  /** One line for the card: what pressing it does. */
  what: string;
  /** True for the keystroke this shortcut answers. */
  match: (event: KeyboardEvent) => boolean;
}

/** The letter a key carries, whatever the layout puts on it. */
const letter = (event: KeyboardEvent, ch: string) => event.key.toLowerCase() === ch;

/**
 * True for a key pressed **plain** — no ⌘, ⌃ or ⌥.
 *
 * The letter shortcuts use this, because those chords belong to the system and to
 * the app's own menus: `⌘H` hides the window on macOS, and a board that answered
 * it would swallow the standard shortcut. The undo entry is the exception that
 * proves the rule — `⌘Z` is exactly what undo should be — so it asks for the
 * chord deliberately.
 */
const plain = (event: KeyboardEvent) => !event.metaKey && !event.ctrlKey && !event.altKey;

/**
 * Every shortcut, in the order the card lists them and the order they are
 * matched in — so `?` is tried last and never shadows a letter.
 *
 * The first four are the ones a learner reaches for mid-attempt; watching and
 * hearing are the two the board's own controls carry; the last is the way back
 * to this list.
 */
export const SHORTCUTS: Shortcut[] = [
  {
    id: "check",
    keys: "Enter",
    what: "Check the attempt — or move on once it has been graded",
    match: (event) => event.key === "Enter",
  },
  {
    id: "undo",
    keys: "⌫ or ⌘Z",
    what: "Take the last stroke back",
    match: (event) =>
      event.key === "Backspace" || ((event.metaKey || event.ctrlKey) && letter(event, "z")),
  },
  {
    id: "previous",
    keys: "←",
    what: "The character before this one",
    match: (event) => event.key === "ArrowLeft",
  },
  {
    id: "next",
    keys: "→",
    what: "The character after this one",
    match: (event) => event.key === "ArrowRight",
  },
  {
    id: "strokes",
    keys: "S",
    what: "Watch the character written, one stroke at a time",
    match: (event) => plain(event) && letter(event, "s"),
  },
  {
    id: "hear",
    keys: "H",
    what: "Hear the character pronounced",
    match: (event) => plain(event) && letter(event, "h"),
  },
  {
    id: "help",
    keys: "?",
    what: "This card — Esc closes it, or the key again",
    match: (event) => event.key === "?" || (plain(event) && event.key === "/"),
  },
];

/**
 * The shortcut a keystroke answers, or `null` when the app has no use for it.
 *
 * Deliberately only a lookup: whether a key should be honoured at all is the
 * caller's business, because only the caller knows what has focus and what is
 * covering the board.
 */
export function shortcutFor(event: KeyboardEvent): Shortcut | null {
  return SHORTCUTS.find((shortcut) => shortcut.match(event)) ?? null;
}
