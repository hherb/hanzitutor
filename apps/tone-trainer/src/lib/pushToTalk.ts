/**
 * One push-to-talk attempt at a time, and only the latest one's answer.
 *
 * **Copied verbatim to `apps/tone-trainer/src/lib/pushToTalk.ts`**, the way
 * `transcript.ts` is (HANDOVER §10), and `pushToTalk.test.ts` runs the same cases
 * against both copies so they cannot drift apart in behaviour.
 *
 * The microphone commands run off the window's thread (issue #26), so the window
 * keeps taking input while they work — which is the point, and which makes two
 * things reachable that a frozen window hid:
 *
 * - **A release while the device is still opening.** A learner who lets go to
 *   answer the microphone permission prompt used to find the recording running
 *   with no finger on the button once the device opened. Here a release during
 *   `starting` is remembered, and the recording is thrown away as soon as the
 *   device is open: nothing the learner said was captured, because nothing was
 *   listening yet.
 * - **An answer for something no longer on screen.** The board or the drill can
 *   move on while a recording is being scored. Every attempt carries a
 *   generation; [`PushToTalk.retarget`] moves it on, and an answer for an older
 *   one is dropped rather than shown against the new target. A recording still
 *   running when the target moves is cancelled.
 *
 * What this does **not** do is the rest of issue #27: the button stays the only
 * input, the Tone Trainer's Space bar still toggles, and the recording cap is the
 * capture crate's business.
 */

/** Where the one attempt is. `idle` is the only phase a press starts from. */
export type Phase = "idle" | "starting" | "listening" | "stopping" | "scoring";

/** The three calls an attempt makes. */
export interface TalkIo<R> {
  /** Open the device; resolves once it is recording. */
  start(): Promise<void>;
  /** Stop and judge the recording. */
  stop(): Promise<R>;
  /** Stop and throw the recording away. */
  cancel(): Promise<void>;
}

/** What one press or release came to, for the screen to act on. */
export type Outcome<R> =
  /** The device is open and recording. */
  | { kind: "listening" }
  /** The judgement of the latest attempt. */
  | { kind: "scored"; result: R }
  /** The latest attempt could not be started or judged. */
  | { kind: "failed"; during: "start" | "stop"; cause: unknown }
  /** Released before the device was open: nothing was recorded, so nothing is judged. */
  | { kind: "released-early" }
  /** A release during `starting`: the press's own outcome will say what came of it. */
  | { kind: "deferred" }
  /** The answer belonged to an attempt the screen has since moved on from. */
  | { kind: "dropped" }
  /** A press or release that means nothing in the current phase. */
  | { kind: "ignored" };

export class PushToTalk<R> {
  #io: TalkIo<R>;
  #onPhase: (phase: Phase) => void;
  #phase: Phase = "idle";
  /** The current attempt's generation. Anything carrying an older one is stale. */
  #attempt = 0;
  /** A release arrived while the device was opening. */
  #released = false;

  constructor(io: TalkIo<R>, onPhase: (phase: Phase) => void = () => {}) {
    this.#io = io;
    this.#onPhase = onPhase;
  }

  get phase(): Phase {
    return this.#phase;
  }

  #set(phase: Phase) {
    this.#phase = phase;
    this.#onPhase(phase);
  }

  /** The button went down. Resolves once the device is open, or with why not. */
  async press(): Promise<Outcome<R>> {
    if (this.#phase !== "idle") return { kind: "ignored" };
    const attempt = ++this.#attempt;
    this.#released = false;
    this.#set("starting");
    try {
      await this.#io.start();
    } catch (cause) {
      this.#set("idle");
      return attempt === this.#attempt
        ? { kind: "failed", during: "start", cause }
        : { kind: "dropped" };
    }
    if (attempt !== this.#attempt || this.#released) {
      // Still `starting` while the cancel runs, so a new press cannot open the
      // device underneath a cancel that is about to close it.
      const early = attempt === this.#attempt;
      this.#released = false;
      try {
        await this.#io.cancel();
      } catch {
        // Nothing open is what was wanted — and a cancel that throws rather than
        // rejects must not leave the phase at `starting` for good.
      }
      this.#set("idle");
      return early ? { kind: "released-early" } : { kind: "dropped" };
    }
    this.#set("listening");
    return { kind: "listening" };
  }

  /** The button came up. Resolves with the judgement, once it is back. */
  async release(): Promise<Outcome<R>> {
    if (this.#phase === "starting") {
      this.#released = true;
      return { kind: "deferred" };
    }
    if (this.#phase !== "listening") return { kind: "ignored" };
    const attempt = this.#attempt;
    this.#set("scoring");
    try {
      const result = await this.#io.stop();
      return attempt === this.#attempt ? { kind: "scored", result } : { kind: "dropped" };
    } catch (cause) {
      return attempt === this.#attempt
        ? { kind: "failed", during: "stop", cause }
        : { kind: "dropped" };
    } finally {
      this.#set("idle");
    }
  }

  /**
   * What is being practised changed, or the screen is being left: nothing in
   * flight may land on what comes next.
   *
   * A recording that is running is cancelled, a device still opening is cancelled
   * once it is open, and an answer being worked out is dropped when it arrives —
   * the phase stays `scoring` until then, so a new recording does not start while
   * the last one is still being decoded.
   */
  async retarget(): Promise<void> {
    this.#attempt++;
    if (this.#phase !== "listening") return;
    this.#set("stopping");
    try {
      await this.#io.cancel();
    } catch {
      // Nothing open is what was wanted.
    } finally {
      this.#set("idle");
    }
  }
}
