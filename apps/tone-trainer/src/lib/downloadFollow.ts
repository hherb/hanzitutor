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

export class DownloadFollower<S extends Followed> {
  #current: Promise<S | null> | null = null;

  /**
   * @param refresh Ask the backend once and return what it said, or `null` (or
   *   throw) when it could not be asked. Either failure keeps the following going:
   *   it says nothing about the download, which runs on its own thread.
   * @param wait The pause between two questions.
   */
  constructor(
    private readonly refresh: () => Promise<S | null>,
    private readonly wait: () => Promise<void>,
  ) {}

  /** True while a download is being followed. */
  get running(): boolean {
    return this.#current !== null;
  }

  /**
   * Follow until the state settles, and resolve with the status that settled it.
   * A call while one is already running returns that one.
   */
  follow(): Promise<S | null> {
    if (this.#current === null) {
      this.#current = this.#run().finally(() => {
        this.#current = null;
      });
    }
    return this.#current;
  }

  async #run(): Promise<S | null> {
    for (;;) {
      await this.wait();
      let status: S | null;
      try {
        status = await this.refresh();
      } catch {
        status = null;
      }
      if (isSettled(status)) return status;
    }
  }
}
