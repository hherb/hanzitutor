import { describe, expect, it } from "vitest";
import { serialQueue } from "./serialQueue";

/** A promise and the means to settle it from the test. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (cause: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

/** Let every queued callback run. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("serialQueue", () => {
  it("runs one task at a time, in the order they were asked for", async () => {
    const queue = serialQueue();
    const started: string[] = [];
    const first = deferred<string>();
    const a = queue.run(() => {
      started.push("a");
      return first.promise;
    });
    const b = queue.run(async () => {
      started.push("b");
      return "b";
    });
    await settle();
    // The second waits for the first rather than being refused or run beside it.
    expect(started).toEqual(["a"]);
    first.resolve("a");
    expect(await a).toBe("a");
    expect(await b).toBe("b");
    expect(started).toEqual(["a", "b"]);
  });

  it("drops nothing: a task asked for while one is running still runs", async () => {
    const queue = serialQueue();
    const ran: number[] = [];
    const results = await Promise.all(
      [1, 2, 3, 4].map((n) =>
        queue.run(async () => {
          ran.push(n);
          return n * 10;
        }),
      ),
    );
    expect(ran).toEqual([1, 2, 3, 4]);
    expect(results).toEqual([10, 20, 30, 40]);
  });

  it("goes on after a task that fails, and hands the failure to its caller", async () => {
    const queue = serialQueue();
    const failing = queue.run(async () => {
      throw new Error("already in that group");
    });
    const next = queue.run(async () => "next");
    await expect(failing).rejects.toThrow("already in that group");
    expect(await next).toBe("next");
  });

  it("goes on after a task that throws before it returns a promise", async () => {
    const queue = serialQueue();
    const failing = queue.run(() => {
      throw new Error("synchronous");
    });
    await expect(failing).rejects.toThrow("synchronous");
    expect(await queue.run(async () => 1)).toBe(1);
  });

  it("says it is busy from the first task asked for until the last has finished", async () => {
    const changes: boolean[] = [];
    const queue = serialQueue((busy) => changes.push(busy));
    const first = deferred<void>();
    const second = deferred<void>();
    const a = queue.run(() => first.promise);
    const b = queue.run(() => second.promise);
    expect(changes).toEqual([true]);
    first.resolve();
    await a;
    // Still busy: the second is running now.
    expect(changes).toEqual([true]);
    second.reject(new Error("no"));
    await b.catch(() => undefined);
    expect(changes).toEqual([true, false]);
    // And busy again for the next one.
    await queue.run(async () => undefined);
    expect(changes).toEqual([true, false, true, false]);
  });

  it("is no longer busy by the time the caller sees the result", async () => {
    let busy = false;
    const queue = serialQueue((value) => (busy = value));
    await queue.run(async () => {
      expect(busy).toBe(true);
    });
    expect(busy).toBe(false);
  });
});
