/**
 * Run asynchronous tasks one at a time, in the order they were asked for.
 *
 * The vocabulary commands go through one of these (issue #33). They used to be
 * refused while another was running, which dropped whatever was asked for in the
 * meantime — a practice attempt being recorded included, and that one has no
 * button to press again. Waiting its turn drops nothing.
 *
 * A task that fails hands its failure to its own caller and does not stop the
 * ones behind it. `onBusy` is told `true` when the first task is asked for and
 * `false` once the last has finished, and is told before that task's caller sees
 * the result, so a button disabled while the queue is busy is enabled again by
 * the time anything is said about the outcome.
 */
export interface SerialQueue {
  run<T>(task: () => Promise<T>): Promise<T>;
}

export function serialQueue(onBusy: (busy: boolean) => void = () => {}): SerialQueue {
  let tail: Promise<unknown> = Promise.resolve();
  let pending = 0;
  return {
    run<T>(task: () => Promise<T>): Promise<T> {
      pending += 1;
      if (pending === 1) onBusy(true);
      const done = tail
        .then(() => task())
        .finally(() => {
          pending -= 1;
          if (pending === 0) onBusy(false);
        });
      // The next task waits for this one to settle, however it settles.
      tail = done.then(
        () => undefined,
        () => undefined,
      );
      return done;
    },
  };
}
