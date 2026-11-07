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
  it("keeps recording when the button goes down again before the device opens", async () => {
    // Held, let go and held again while the device opens — a finger that slid
    // off and back, or a key tapped twice. What counts is that it is down when
    // the device is open, so this records rather than throwing the attempt away
    // under a finger that is on the button.
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    expect(await talk.release()).toEqual({ kind: "deferred" });
    expect(await talk.press()).toEqual({ kind: "deferred" });
    fake.starts[0].resolve();
    expect(await pressed).toEqual({ kind: "listening" });

    const released = talk.release();
    fake.stops[0].resolve("a judgement");
    expect(await released).toEqual({ kind: "scored", result: "a judgement" });
    expect(fake.calls).toEqual(["start", "stop"]);
  });

  it("still cancels when the second hold is let go before the device opens", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    await talk.release();
    await talk.press();
    await talk.release();
    fake.starts[0].resolve();

    expect(await pressed).toEqual({ kind: "released-early" });
    expect(fake.calls).toEqual(["start", "cancel"]);
  });

  it("ignores a press while the button is already held and the device opens", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    expect(await talk.press()).toEqual({ kind: "ignored" });
    fake.starts[0].resolve();
    expect(await pressed).toEqual({ kind: "listening" });
    expect(fake.calls).toEqual(["start"]);
  });

  it("cancels a running recording when the learner goes away", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    fake.starts[0].resolve();
    await pressed;
    await talk.abandon();

    expect(fake.calls).toEqual(["start", "cancel"]);
    expect(talk.phase).toBe("idle");
    expect(await talk.release()).toEqual({ kind: "ignored" });
  });

  it("cancels a device that opens after the learner went away", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    await talk.abandon();
    fake.starts[0].resolve();

    expect(await pressed).toEqual({ kind: "dropped" });
    expect(fake.calls).toEqual(["start", "cancel"]);
    expect(talk.phase).toBe("idle");
  });

  it("keeps a judgement already on its way when the learner goes away", async () => {
    // Let go, then switched away: the recording was finished on purpose, and the
    // board it belongs to is still there to come back to.
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    const pressed = talk.press();
    fake.starts[0].resolve();
    await pressed;
    const released = talk.release();
    await talk.abandon();
    fake.stops[0].resolve("still wanted");

    expect(await released).toEqual({ kind: "scored", result: "still wanted" });
    expect(fake.calls).toEqual(["start", "stop"]);
  });

  it("does nothing when the learner goes away with nothing in flight", async () => {
    const fake = backend();
    const talk = new PushToTalk<string>(fake.io);

    await talk.abandon();
    expect(fake.calls).toEqual([]);
    expect(talk.phase).toBe("idle");
    const pressed = talk.press();
    fake.starts[0].resolve();
    expect(await pressed).toEqual({ kind: "listening" });
  });

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

describe.each([
  ["the full app", here.HeldKey],
  ["the tone trainer", trainer.HeldKey],
] as const)("a held key in %s", (_, HeldKey) => {
  it("starts a hold on the first press and ends it on the release", () => {
    const key = new HeldKey();
    expect(key.down(false)).toBe(true);
    expect(key.held).toBe(true);
    expect(key.up()).toBe(true);
    expect(key.held).toBe(false);
  });

  it("ignores the key repeating while it is held", () => {
    // Holding Space sends a keydown every few tens of milliseconds; each one is
    // not a new press, or the second would be taken as a release.
    const key = new HeldKey();
    expect(key.down(false)).toBe(true);
    expect(key.down(true)).toBe(false);
    expect(key.down(true)).toBe(false);
    expect(key.held).toBe(true);
  });

  it("ignores a repeat that arrives without a press", () => {
    // The key was already down when focus came back to the window.
    const key = new HeldKey();
    expect(key.down(true)).toBe(false);
    expect(key.held).toBe(false);
  });

  it("does not start a second hold while one is held", () => {
    // Some platforms do not mark a repeat; a second press is still the same hold.
    const key = new HeldKey();
    key.down(false);
    expect(key.down(false)).toBe(false);
    expect(key.up()).toBe(true);
  });

  it("ends a hold once, however many ways it ends", () => {
    // The window losing focus ends it too — the key's own release then goes to
    // another window — and the release that may follow is not a second one.
    const key = new HeldKey();
    key.down(false);
    expect(key.up()).toBe(true);
    expect(key.up()).toBe(false);
  });

  it("ignores a release it never saw pressed", () => {
    // Space pressed in a text field, released after focus left it.
    const key = new HeldKey();
    expect(key.up()).toBe(false);
  });
});
