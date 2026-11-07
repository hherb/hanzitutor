import { describe, expect, it } from "vitest";
import * as here from "./pushToTalk";
import * as trainer from "../../apps/tone-trainer/src/lib/pushToTalk";

/** A promise the test settles by hand, so each call resolves when the test says. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (cause: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

/** A fake backend that records every call and answers when told to. */
function backend() {
  const calls: string[] = [];
  const starts: ReturnType<typeof deferred<void>>[] = [];
  const stops: ReturnType<typeof deferred<string>>[] = [];
  return {
    calls,
    starts,
    stops,
    io: {
      start() {
        calls.push("start");
        const next = deferred<void>();
        starts.push(next);
        return next.promise;
      },
      stop() {
        calls.push("stop");
        const next = deferred<string>();
        stops.push(next);
        return next.promise;
      },
      cancel() {
        calls.push("cancel");
        return Promise.resolve();
      },
    },
  };
}

/** Let every already-settled promise run its continuations. */
const settle = () => new Promise((done) => setTimeout(done, 0));

// Both copies, the same cases: the tone trainer's is a copy on purpose, and this
// is what keeps it one.
describe.each([
  ["the full app", here.PushToTalk],
  ["the tone trainer", trainer.PushToTalk],
] as const)("push-to-talk in %s", (_, PushToTalk) => {
  it("returns to idle when a cancel throws instead of rejecting", async () => {
    const fake = backend();
    fake.io.cancel = () => {
      fake.calls.push("cancel");
      throw new Error("the bridge went away");
    };
    const talk = new PushToTalk<string>(fake.io, () => {});

    const pressed = talk.press();
    await talk.release();
    fake.starts[0].resolve();
    expect(await pressed).toEqual({ kind: "released-early" });
    expect(talk.phase).toBe("idle");
    expect(fake.calls).toEqual(["start", "cancel"]);
  });

  it("records between press and release, and hands back the judgement", async () => {
    const fake = backend();
    const phases: string[] = [];
    const talk = new PushToTalk<string>(fake.io, (phase) => phases.push(phase));

    const pressed = talk.press();
    expect(talk.phase).toBe("starting");
    fake.starts[0].resolve();
    expect(await pressed).toEqual({ kind: "listening" });

    const released = talk.release();
    expect(talk.phase).toBe("scoring");
    fake.stops[0].resolve("a judgement");
    expect(await released).toEqual({ kind: "scored", result: "a judgement" });
    expect(talk.phase).toBe("idle");
    expect(fake.calls).toEqual(["start", "stop"]);
    expect(phases).toEqual(["starting", "listening", "scoring", "idle"]);
  });

  it("does not leave the device open after a release while it was opening", async () => {
    // The permission prompt case: the learner lets go to answer it, and the
    // device opens afterwards with no finger on the button.
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    expect(await talk.release()).toEqual({ kind: "deferred" });
    fake.starts[0].resolve();

    expect(await pressed).toEqual({ kind: "released-early" });
    expect(fake.calls).toEqual(["start", "cancel"]);
    expect(talk.phase).toBe("idle");
    // Nothing is left to stop.
    expect(await talk.release()).toEqual({ kind: "ignored" });
  });

  it("ignores a second press while one attempt is in flight", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    expect(await talk.press()).toEqual({ kind: "ignored" });
    fake.starts[0].resolve();
    await pressed;
    const released = talk.release();
    // While the last recording is being judged, no new one starts: two
    // judgements in flight could come back in either order.
    expect(await talk.press()).toEqual({ kind: "ignored" });
    fake.stops[0].resolve("done");
    await released;
    expect(fake.calls).toEqual(["start", "stop"]);
  });

  it("drops a judgement that comes back after the target moved on", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    fake.starts[0].resolve();
    await pressed;
    const released = talk.release();
    await talk.retarget();
    // Still scoring: the old decode is still running behind it.
    expect(talk.phase).toBe("scoring");
    fake.stops[0].resolve("for the old target");

    expect(await released).toEqual({ kind: "dropped" });
    expect(talk.phase).toBe("idle");
  });

  it("drops a failure that comes back after the target moved on", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    fake.starts[0].resolve();
    await pressed;
    const released = talk.release();
    await talk.retarget();
    fake.stops[0].reject(new Error("for the old target"));

    expect(await released).toEqual({ kind: "dropped" });
  });

  it("cancels a recording that is running when the target moves on", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    fake.starts[0].resolve();
    await pressed;
    await talk.retarget();

    expect(fake.calls).toEqual(["start", "cancel"]);
    expect(talk.phase).toBe("idle");
    // The release that follows has nothing to judge.
    expect(await talk.release()).toEqual({ kind: "ignored" });
  });

  it("cancels a device that opens after the target moved on", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    await talk.retarget();
    fake.starts[0].resolve();

    expect(await pressed).toEqual({ kind: "dropped" });
    expect(fake.calls).toEqual(["start", "cancel"]);
    expect(talk.phase).toBe("idle");
  });

  it("reports a device that would not open, and can be pressed again", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    fake.starts[0].reject("no microphone");
    expect(await pressed).toEqual({ kind: "failed", during: "start", cause: "no microphone" });
    expect(talk.phase).toBe("idle");

    const again = talk.press();
    fake.starts[1].resolve();
    expect(await again).toEqual({ kind: "listening" });
  });

  it("reports a judgement that failed, and returns to idle", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    fake.starts[0].resolve();
    await pressed;
    const released = talk.release();
    fake.stops[0].reject("Not listening.");

    expect(await released).toEqual({ kind: "failed", during: "stop", cause: "Not listening." });
    expect(talk.phase).toBe("idle");
  });

  it("keeps the next attempt's answer when an older one is dropped", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    // An attempt whose device opened after the target moved on…
    const first = talk.press();
    await talk.retarget();
    fake.starts[0].resolve();
    expect(await first).toEqual({ kind: "dropped" });

    // …does not take the next one's judgement with it.
    const second = talk.press();
    fake.starts[1].resolve();
    await second;
    const released = talk.release();
    await settle();
    fake.stops[0].resolve("for this target");
    expect(await released).toEqual({ kind: "scored", result: "for this target" });
  });
});
