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

/**
 * The part of a keystroke a shortcut is matched on.
 *
 * A `KeyboardEvent` is one, which is how the handler passes it; the narrower
 * type is what lets the routing below be tested without a DOM.
 */
export interface KeyChord {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
}

export interface Shortcut {
  id: ShortcutId;
  /** The keys as the card writes them, e.g. `"⌫ or ⌘Z"`. */
  keys: string;
  /** One line for the card: what pressing it does. */
  what: string;
  /** True for the keystroke this shortcut answers. */
  match: (event: KeyChord) => boolean;
}

/** The letter a key carries, whatever the layout puts on it. */
const letter = (event: KeyChord, ch: string) => event.key.toLowerCase() === ch;

/**
 * True for a key pressed **plain** — no ⌘, ⌃ or ⌥.
 *
 * The letter shortcuts use this, because those chords belong to the system and to
 * the app's own menus: `⌘H` hides the window on macOS, and a board that answered
 * it would swallow the standard shortcut. The undo entry is the exception that
 * proves the rule — `⌘Z` is exactly what undo should be — so it asks for the
 * chord deliberately.
 */
const plain = (event: KeyChord) => !event.metaKey && !event.ctrlKey && !event.altKey;

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
 * Deliberately only a lookup: whether a key should be honoured at all is
 * [`routeKey`]'s business, because that depends on what has focus and what is
 * on the screen.
 */
export function shortcutFor(event: KeyChord): Shortcut | null {
  return SHORTCUTS.find((shortcut) => shortcut.match(event)) ?? null;
}

// ---- whether a key reaches the board ----------------------------------------

/**
 * What a keystroke landed on, as far as the board's keys care.
 *
 * - `text`: something that takes typing — a field, a select, an editable
 *   region. Every key is its own, Backspace and the arrows above all.
 * - `control`: something Enter or Space *activates* — a button, a link, a
 *   disclosure, anything given a place in the tab order. Those two keys are the
 *   control's; the rest of the board's keys still work, since a button does
 *   nothing with ← or S.
 * - `other`: the page itself, the board, a sheet that took focus only so it
 *   could be read.
 */
export type TargetKind = "text" | "control" | "other";

/**
 * The few things [`targetKind`] reads off an element. An `Element` is one; the
 * tests pass a plain object.
 */
export interface TargetLike {
  tagName: string;
  isContentEditable?: boolean;
  hasAttribute(name: string): boolean;
  getAttribute(name: string): string | null;
}

/** Elements Enter or Space activates whatever their attributes say. */
const CONTROL_TAGS = new Set(["BUTTON", "SUMMARY"]);
/** Elements that take typing — every input, as before, checkboxes included. */
const TEXT_TAGS = new Set(["INPUT", "TEXTAREA", "SELECT"]);
/** ARIA roles whose activation key is Enter or Space. */
const CONTROL_ROLES = new Set([
  "button",
  "link",
  "checkbox",
  "radio",
  "switch",
  "tab",
  "menuitem",
  "menuitemcheckbox",
  "menuitemradio",
  "option",
]);

/** Sort the element a key was pressed in — see [`TargetKind`]. `null` is the page. */
export function targetKind(target: TargetLike | null): TargetKind {
  if (target === null) return "other";
  const tag = target.tagName.toUpperCase();
  if (TEXT_TAGS.has(tag) || target.isContentEditable) return "text";
  if (CONTROL_TAGS.has(tag)) return "control";
  if (tag === "A" && target.hasAttribute("href")) return "control";
  const role = target.getAttribute("role");
  if (role !== null && CONTROL_ROLES.has(role.trim().toLowerCase())) return "control";
  // A tab stop someone made: it is focusable on purpose, so it may well answer
  // Enter. `tabindex="-1"` is the opposite — focusable by script only, which is
  // how the startup sheet takes focus to be read, not to be pressed.
  const tabindex = target.getAttribute("tabindex");
  if (tabindex !== null && Number.parseInt(tabindex, 10) >= 0) return "control";
  return "other";
}

/** Everything about the screen a key's meaning depends on. */
export interface KeyContext {
  /** What the key was pressed in. */
  target: TargetKind;
  /** The startup reading is up. It covers everything, so only Escape passes. */
  startupSheet: boolean;
  /** The phone's navigation sheet is open over the board. */
  navOpen: boolean;
  /** The key list card is showing. It covers nothing, so the board stays live. */
  shortcutsOpen: boolean;
  /**
   * The practice board is the screen showing — `App.svelte`'s `practising`,
   * false on Settings, About and the list screens.
   */
  practising: boolean;
}

/**
 * What the handler does with a key: one of the board's shortcuts (`help`
 * toggling the card), or Escape dismissing whatever is on top.
 */
export type KeyAction = ShortcutId | "finish-startup" | "close-nav" | "close-shortcuts";

export interface KeyRoute {
  /** What to do, or `null` for "this key is not the app's". */
  action: KeyAction | null;
  /**
   * Whether to stop the browser's own handling. True exactly when there is an
   * action: a key the app acts on is claimed outright, so Enter does not also
   * press a button and an arrow does not also scroll a list; a key it does not
   * act on is left entirely alone.
   */
  preventDefault: boolean;
}

const act = (action: KeyAction): KeyRoute => ({ action, preventDefault: true });
const PASS: KeyRoute = { action: null, preventDefault: false };

/**
 * Decide what one keystroke does (issue #31).
 *
 * The board's keys act on the board, so they act **only while the board is the
 * screen showing** and nothing covers it. On Settings an arrow used to move the
 * course behind the screen (and save the move), Enter graded a board nobody
 * could see, and Backspace undid strokes on it.
 *
 * In order:
 *
 * 1. A text field keeps every key.
 * 2. The startup reading covers everything: only Escape, which dismisses it.
 * 3. The phone's navigation sheet covers the board: only Escape, which closes it
 *    — the same as Back on Android (`__hanziHandleBack`).
 * 4. Escape closes the key list wherever it is open.
 * 5. Off the board, nothing.
 * 6. Enter and Space belong to a focused control. A keyboard user who tabs to a
 *    button and presses Enter means *that button*; claiming the key for Check
 *    left the button dead and graded the board instead.
 * 7. Otherwise, the shortcut the key names, if any.
 */
export function routeKey(chord: KeyChord, context: KeyContext): KeyRoute {
  if (context.target === "text") return PASS;
  const escape = chord.key === "Escape";

  if (context.startupSheet) return escape ? act("finish-startup") : PASS;
  if (context.navOpen) return escape ? act("close-nav") : PASS;
  if (escape) return context.shortcutsOpen ? act("close-shortcuts") : PASS;
  if (!context.practising) return PASS;

  if (context.target === "control" && (chord.key === "Enter" || chord.key === " ")) return PASS;

  const shortcut = shortcutFor(chord);
  return shortcut === null ? PASS : act(shortcut.id);
}
