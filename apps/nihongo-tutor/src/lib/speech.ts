/**
 * What can be heard, and which key asks for it — as pure functions.
 *
 * The speaker itself is the operating system's and lives on the other side of the
 * IPC: `speak`, `stop_speaking` and `voice` in `api.ts`, over the same
 * `hanzi_voice::Speaker` the two Hanzi Tutor apps use, told once that this app
 * speaks Japanese. What is left on this side is two decisions, and both are rules
 * rather than rendering:
 *
 * * **whether anything can be heard at all.** A machine with no Japanese voice
 *   installed has to be told about, rather than handed a button that silently does
 *   nothing — and the answer has three states, not two, because there is a moment
 *   between the screen appearing and the answer arriving.
 * * **whether a keypress means "pronounce this".** The shortcut is a letter, and
 *   letters are what a learner typing a reading is pressing, so the rule that
 *   keeps the two apart is worth a test rather than a look at the running app.
 *
 * So both are here, where `vitest` runs them without a window, and the panels
 * only render what they return — the same arrangement as `board.ts`, `words.ts`,
 * `kanji.ts` and `review.ts`.
 */

/**
 * What the `voice` command answers: a description of the voice in use, `null`
 * when the machine has none, or `undefined` while the answer is still on its way.
 *
 * Three states rather than a `string | null` with an assumed default, because the
 * screen shows a different thing for each and only one of them is a problem.
 */
export type VoiceStatus = string | null | undefined;

/** Whether a "Hear it" control can do anything. */
export function canHear(voice: VoiceStatus): boolean {
  return typeof voice === "string";
}

/**
 * Why pronunciation is unavailable, or `null` when it is available.
 *
 * The instruction is macOS's own, because that is the only platform this app
 * speaks on today, and it names the environment override as well: a machine whose
 * voice list is fine but whose *chosen* voice is missing is a case the settings
 * path does not fix, and `nihongo_core`'s `Speaker` reads that variable under this
 * app's own name rather than the Chinese app's.
 *
 * `undefined` is not a failure. It is the moment between the screen appearing and
 * the answer arriving, and saying "no voice" for it would be a claim the app is
 * about to contradict.
 */
export function voiceNote(voice: VoiceStatus): string | null {
  if (voice === undefined) return "Looking for a Japanese voice…";
  if (voice === null) {
    return (
      "No Japanese voice is installed, so nothing here can be heard. Add one in " +
      "System Settings → Accessibility → Spoken Content → System Voice → " +
      "Manage Voices, or set NIHONGO_TUTOR_VOICE."
    );
  }
  return null;
}

/** The shape of a keydown this module needs, so a test needs no DOM. */
export interface KeyLike {
  key: string;
  repeat?: boolean;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  /**
   * Where the press landed. `KeyboardEvent.target` is an `EventTarget`, which has
   * neither a tag name nor `isContentEditable`, so the DOM's own type and the
   * plain object a test writes are both accepted here and narrowed by
   * [`elementOf`] rather than at every call site.
   */
  target?: EventTarget | { tagName?: string; isContentEditable?: boolean } | null;
}

/**
 * The element a keypress landed on, as much of it as this rule needs.
 *
 * The one cast in this file, and it is here rather than in the caller so that a
 * screen passes its event untouched and a test passes a two-field object.
 */
function elementOf(target: KeyLike["target"]): {
  tagName?: string;
  isContentEditable?: boolean;
} {
  return (target ?? {}) as { tagName?: string; isContentEditable?: boolean };
}

/**
 * Whether a keypress asks for pronunciation.
 *
 * **`h`**, for "hear". Not Space, and that is the decision worth recording:
 * Space is what the Tone Trainer gives its microphone and what a hand reaches for
 * without looking, but on a page it already means "activate the control that has
 * focus". A learner who has just clicked a kana cell still has that cell focused,
 * so a Space shortcut would press the cell again instead of speaking — the key
 * would look broken in the commonest way to reach it. `h` has no default action
 * anywhere and no other meaning in this app, so it can be claimed outright.
 *
 * Four things make a keypress not this key:
 *
 * * a **repeat**, which is a key held down rather than a request;
 * * a keypress with **Command, Control or Option** held — `⌘H` hides the window,
 *   and a shortcut that ate it would be worse than one that did not exist;
 * * the wrong letter, in either case, so `Shift+H` works and `h` in the middle of
 *   a word does not;
 * * a keypress that lands in a **field being typed in**, or in a `<select>`, or in
 *   anything contenteditable. This is the one that matters most: the reading box
 *   on Practice and the one on every word card are where an `h` is meant to be a
 *   letter, and the kana `は` is typed as `ha`.
 */
export function isHearItKey(event: KeyLike): boolean {
  if (event.repeat) return false;
  if (event.metaKey || event.ctrlKey || event.altKey) return false;
  if (event.key.toLowerCase() !== "h") return false;

  const target = elementOf(event.target);
  if (target.isContentEditable) return false;
  const tag = target.tagName?.toUpperCase() ?? "";
  return tag !== "INPUT" && tag !== "TEXTAREA" && tag !== "SELECT";
}
