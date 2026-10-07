/**
 * The model check's arithmetic, as pure functions.
 *
 * The composition exercise's second screenful, and the same split the rest of the app
 * uses: what can be decided without a window lives here and is unit-tested by the root
 * project's vitest, and the component only draws.
 *
 * Two of these are rules rather than arithmetic, and both exist because a language
 * model's opinion must not be able to pass for this app's own:
 *
 * * `disclosureLine` says **what leaves the machine** before anything does — the
 *   pattern, the sentence, and the host they go to — because a feature that sends a
 *   learner's writing somewhere has to say so in the same breath as it offers the
 *   button;
 * * `attributionLine` says **whose answer it is**, naming the model and the host, and
 *   `reviewHeadline` echoes the model's own word for its judgement rather than mapping
 *   it onto anything of this app's. The checklist on the same screen is this app's; this
 *   is not, and the wording keeps the two apart (`HANDOVER_NIHONGO.md` invariant 43).
 */

import type { ModelReview, ModelStatus, ModelVerdict } from "./types";

/**
 * Where the settings live, said plainly.
 *
 * The one screen that asks for a key owes the learner the two facts that matter: it is
 * the machine's own file, and a local endpoint needs no key at all. Long on purpose —
 * this is the sentence a person reads before typing a secret into a text box.
 */
export function settingsNote(): string {
  return "The endpoint, the model name and any key are kept in this app's own data directory, in a file only you can read. A local server needs no key. Nothing is sent anywhere until you press the button below.";
}

/** What leaves the machine, before it does. */
export function disclosureLine(status: ModelStatus, pattern: string): string {
  const host = hostOf(status.endpoint);
  return `Sends the pattern ${pattern} and your sentence — in the form this app reads it back — to ${host} (${status.model}). Nothing else: no notes, no examples, and nothing is kept.`;
}

/** The endpoint's host, for a line that names where a sentence goes. */
export function hostOf(endpoint: string): string {
  const trimmed = endpoint.trim();
  const afterScheme = trimmed.includes("://") ? trimmed.split("://")[1] : trimmed;
  const host = afterScheme.split("/")[0];
  return host === "" ? trimmed : host;
}

/**
 * The model's own word for its judgement, as a heading.
 *
 * Echoed, not translated: a model that answers "understandable" is not quietly promoted
 * to "correct", and one that answers something unexpected is shown as it answered.
 */
export function reviewHeadline(review: ModelReview): string {
  const verdict = review.verdict.trim();
  if (verdict === "") return "The model's answer";
  return verdict.charAt(0).toUpperCase() + verdict.slice(1);
}

/** What the model would write instead, when it offered a version. */
export function correctedLine(review: ModelReview): string {
  const corrected = review.corrected?.trim();
  if (!corrected) return "";
  return `The model would write: ${corrected}`;
}

/** The model's explanation, when it gave the reply as prose rather than as fields. */
export function explanationLine(review: ModelReview): string {
  return review.explanation.trim();
}

/**
 * Whose answer this is, and the one caveat that goes with it.
 *
 * The same obligation the phrases screen meets for Tatoeba and the grammar screen for
 * OpenJLPT, applied to a source that is not a corpus: name it, and do not let a machine's
 * opinion read as this app's finding.
 */
export function attributionLine(verdict: ModelVerdict): string {
  return `${verdict.model} at ${verdict.host} — a language model, not this app. It can be wrong, and the checklist above is the only part of this screen that this app decided.`;
}

/** The one line the screen shows when a check did not finish. */
export function failureLine(error: unknown): string {
  const text = String(error);
  return text.trim() === "" ? "The model check did not finish." : text;
}
