/**
 * Following the model download until it settles (issue #41).
 *
 * The screen used to give up after 2400 polls of half a second — twenty minutes —
 * and nothing else asked again, so a 163 MB download on a link slower than about
 * 1.1 Mbit/s finished after the screen had stopped looking, and the panel said
 * "Downloading…" until the app was restarted. These pin the replacement: no count
 * at all, one follower however many places ask for one, and a failed question
 * that does not end the following.
 */

import { describe, expect, it } from "vitest";
import { DownloadFollower, isSettled } from "./downloadFollow";

type Status = { state: "absent" | "downloading" | "installed" | "failed" };

/** A backend that reports `downloading` for `polls` answers, then `last`. */
function backend(polls: number, last: Status["state"] = "installed") {
  let asked = 0;
  return {
    get asked() {
      return asked;
    },
    refresh: async (): Promise<Status | null> => {
      asked += 1;
      return { state: asked > polls ? last : "downloading" };
    },
  };
}

/** No real waiting: the tests are about the count, not the clock. */
const noWait = () => Promise.resolve();

describe("following a download", () => {
  it("keeps following a download that outlasts twenty minutes of polling", async () => {
    // 2400 polls was the old bound. A slow link takes longer than that.
    const slow = backend(5000);
    const follower = new DownloadFollower(slow.refresh, noWait);
    const settled = await follower.follow();
    expect(settled?.state).toBe("installed");
    expect(slow.asked).toBe(5001);
    expect(follower.running).toBe(false);
  });

  it("stops at the first answer that is not a download, whichever it is", async () => {
    for (const last of ["installed", "failed", "absent"] as const) {
      const quick = backend(3, last);
      const settled = await new DownloadFollower(quick.refresh, noWait).follow();
      expect(settled?.state).toBe(last);
      expect(quick.asked).toBe(4);
    }
  });

  it("is one follower however many places start one", async () => {
    // The install button, the app coming back into focus and the screen opening
    // can all start following; two loops would poll twice as often and race to
    // write the status.
    const shared = backend(10);
    const follower = new DownloadFollower(shared.refresh, noWait);
    const first = follower.follow();
    const second = follower.follow();
    expect(follower.running).toBe(true);
    expect(second).toBe(first);
    await Promise.all([first, second]);
    expect(shared.asked).toBe(11);

    // And once it has settled, a new download can be followed again.
    const again = backend(2);
    const next = new DownloadFollower(again.refresh, noWait);
    await next.follow();
    await next.follow();
    expect(again.asked).toBe(4);
  });

  it("does not give up because one question went unanswered", async () => {
    // A failed status call says nothing about the download, which is still
    // running on its own thread; the following goes on and the next answer counts.
    let asked = 0;
    const refresh = async (): Promise<Status | null> => {
      asked += 1;
      if (asked === 2) throw new Error("the backend was busy");
      if (asked === 3) return null;
      return { state: asked >= 5 ? "installed" : "downloading" };
    };
    const settled = await new DownloadFollower(refresh, noWait).follow();
    expect(settled?.state).toBe("installed");
    expect(asked).toBe(5);
  });

  it("waits between questions", async () => {
    const events: string[] = [];
    const refresh = async (): Promise<Status> => {
      events.push("ask");
      return { state: events.length > 4 ? "installed" : "downloading" };
    };
    const wait = async () => {
      events.push("wait");
    };
    await new DownloadFollower(refresh, wait).follow();
    expect(events).toEqual(["wait", "ask", "wait", "ask", "wait", "ask"]);
  });
});

describe("a settled state", () => {
  it("is anything known that is not a download", () => {
    expect(isSettled(null)).toBe(false);
    expect(isSettled({ state: "downloading" })).toBe(false);
    expect(isSettled({ state: "installed" })).toBe(true);
    expect(isSettled({ state: "failed" })).toBe(true);
    expect(isSettled({ state: "absent" })).toBe(true);
  });
});
