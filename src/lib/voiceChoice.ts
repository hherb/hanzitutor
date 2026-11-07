/**
 * Which entry the settings screen's voice list shows, and whether the stored
 * choice fell back to another voice.
 *
 * Pure functions of the stored value and the backend's `VoicesView`, so the two
 * rules are tested without rendering the screen. They exist because a choice
 * is a voice's `id` since issue #39 — Apple's novelty voices share one bare
 * name across every language, so `Eddy` alone did not say which Eddy — while a
 * document an older build wrote still holds a bare name, which the backend
 * resolves to one entry (`chosen`).
 */
import type { VoicesView } from "./types";

/**
 * The option value the dropdown shows: `""` for the automatic choice, the
 * stored value when it is one of the listed ids, else the entry the backend
 * resolved it to, else the stored value as it is.
 *
 * The stored value is checked against the list *first* because `chosen` is
 * only refreshed after a save round-trips: right after the learner picks a
 * voice, `chosen` still names the previous one, and preferring it would snap
 * the dropdown back. Falling back to the stored value keeps a voice this
 * machine does not have selected, so the status line can explain why another
 * is heard — showing `active` instead would silently rewrite the choice.
 */
export function selectedVoice(stored: string | null, voices: VoicesView): string {
  if (stored === null) return "";
  if (voices.available.some((voice) => voice.id === stored)) return stored;
  return voices.chosen ?? stored;
}

/**
 * Whether a stored choice names no voice installed here, so another is heard.
 *
 * A stored value that is a listed id is installed, whatever a not-yet-refreshed
 * `chosen` says; otherwise `chosen` — the backend's resolution of the stored
 * value — is `null` exactly when nothing here matches it.
 */
export function voiceFellBack(stored: string | null, voices: VoicesView): boolean {
  if (stored === null || voices.active === null) return false;
  if (voices.available.some((voice) => voice.id === stored)) return false;
  return voices.chosen === null;
}
