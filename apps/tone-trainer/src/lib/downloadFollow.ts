/**
 * Following the recognition model's download until it settles.
 *
 * The install command returns as soon as the transfer has *started* — 163 MB takes
 * minutes, and holding the IPC call open for it would be wrong — so the screen
 * learns the progress and the outcome by asking `asr_status` again and again.
 *
 * ## Why there is no bound on the number of polls
 *
 * There used to be one: 2400 polls of half a second, twenty minutes, as a
 * backstop against a state that never changed. On a link slower than about
 * 1.1 Mbit/s the download outlasted it, and with nothing else asking again the
 * panel said "Downloading…" until the app was restarted (issue #41). The backend
 * already bounds the download — a read that stalls for two minutes fails it — so
 * the state *does* settle, and the screen follows it until it has.
 *
 * ## Why one follower
 *
 * The install button, the app returning to the foreground and the screen opening
 * can all start following. Two loops would poll twice as often and race to write
 * the status, so a second request joins the one already running.
 */

/** What the follower needs to know about a status: only whether it moved on. */
export interface Followed {
  state: string;
}

/** True once a status is known and is anything but a download in progress. */
export function isSettled(status: Followed | null): boolean {
  return status !== null && status.state !== "downloading";
}

/** The pause between two questions while the backend is answering. */
export const POLL_MS = 500;
/** The longest pause, once the questions keep failing. */
export const MAX_POLL_MS = 10_000;

export class DownloadFollower<S extends Followed> {
  #current: Promise<S | null> | null = null;
  /** Bumped by `stop`, so a loop started before it knows to end. */
  #generation = 0;

  /**
   * @param refresh Ask the backend once and return what it said, or `null` (or
   *   throw) when it could not be asked. Either failure keeps the following going:
   *   it says nothing about the download, which runs on its own thread. It does
   *   double the pause, up to `MAX_POLL_MS`, until the next answer.
   * @param wait The pause between two questions, of the length it is given.
   */
  constructor(
    private readonly refresh: () => Promise<S | null>,
    private readonly wait: (ms: number) => Promise<void>,
  ) {}

  /** True while a download is being followed. */
  get running(): boolean {
    return this.#current !== null;
  }

  /**
   * Follow until the state settles, and resolve with the status that settled it,
   * or `null` if `stop` ended it first. A call while one is already running
   * returns that one.
   */
  follow(): Promise<S | null> {
    if (this.#current === null) {
      const current = this.#run(this.#generation).finally(() => {
        if (this.#current === current) this.#current = null;
      });
      this.#current = current;
    }
    return this.#current;
  }

  /** End the following, for a screen that is going away. Asks nothing more. */
  stop(): void {
    this.#generation += 1;
    this.#current = null;
  }

  async #run(generation: number): Promise<S | null> {
    let pause = POLL_MS;
    for (;;) {
      await this.wait(pause);
      if (generation !== this.#generation) return null;
      let status: S | null;
      try {
        status = await this.refresh();
      } catch {
        status = null;
      }
      if (generation !== this.#generation) return null;
      if (isSettled(status)) return status;
      pause = status === null ? Math.min(pause * 2, MAX_POLL_MS) : POLL_MS;
    }
  }
}
