<script lang="ts">
  /**
   * The optional recognition model: describe it, install it, remove it.
   *
   * The same decision the full app's settings screen offers, in a fraction of the
   * space, because this app has exactly one optional download and no preferences
   * to sit beside it. The order is deliberate and copied from there: **say what it
   * is, how large it is, where it comes from and under which licence *before*
   * offering the button**, so nobody spends 163 MB without knowing what for.
   *
   * ## What the model is for, in this app
   *
   * Tone practice measures the pitch, and pitch alone cannot tell 四 from 是 — the
   * contour is the same. Without the model this app can say your tone was right
   * while you said the wrong word; with it, it can say which sound it heard. That
   * is the whole gain, and the copy says so rather than promising pronunciation
   * scoring, which no transcript can give.
   */
  import type { AsrStatus } from "./types";

  interface Props {
    status: AsrStatus | null;
    /** True while a poll is in flight, so the progress line is not stale. */
    polling: boolean;
    onInstall: () => void;
    onRemove: () => void;
    /** A failure from the last call — distinct from `status.error`, which is Rust's. */
    failure: string | null;
  }

  let { status, polling, onInstall, onRemove, failure }: Props = $props();

  /** Bytes as something a person can judge, since this is a size decision. */
  function size(bytes: number): string {
    if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
    return `${Math.round(bytes / 1_000_000)} MB`;
  }

  const downloading = $derived(status?.state === "downloading");
  const installed = $derived(status?.installed === true);

  /** How far along, 0..1, while a download runs. */
  const progress = $derived.by(() => {
    const live = status;
    if (live === null || live.downloadBytes === 0) return 0;
    return Math.min(1, live.downloaded / live.downloadBytes);
  });
</script>

<section class="asr">
  <header>
    <h3>Recognising what you said</h3>
    {#if installed}
      <span class="badge good">Installed</span>
    {:else if downloading}
      <span class="badge">Downloading…</span>
    {:else if status?.state === "failed"}
      <span class="badge bad">Failed</span>
    {:else}
      <span class="badge">Optional</span>
    {/if}
  </header>

  <p class="detail">{status?.detail ?? "Asking the backend about the model…"}</p>

  {#if status && !installed}
    <dl class="sizes">
      <div>
        <dt>Download</dt>
        <dd>{size(status.downloadBytes)}</dd>
      </div>
      <div>
        <dt>On disk</dt>
        <dd>{size(status.unpackedBytes)}</dd>
      </div>
      <div>
        <dt>Licence</dt>
        <dd>
          <a href={status.licenceUrl} target="_blank" rel="noreferrer">{status.licence}</a>
        </dd>
      </div>
    </dl>
  {/if}

  {#if downloading}
    <div
      class="bar"
      role="progressbar"
      aria-label="Download progress"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={Math.round(progress * 100)}
    >
      <span style="width: {Math.round(progress * 100)}%"></span>
    </div>
    <p class="progress">
      {size(status?.downloaded ?? 0)} of {size(status?.downloadBytes ?? 0)}
      {polling ? "" : "— still running on its own thread"}
    </p>
  {/if}

  {#if failure}
    <p class="warning">{failure}</p>
  {/if}
  {#if status?.error}
    <p class="warning">{status.error}</p>
  {/if}

  <div class="actions">
    {#if installed}
      <button onclick={onRemove} title="Delete the model from this device">
        Remove the model
      </button>
      <span class="hint">Recognising stops; tone practice is unaffected.</span>
    {:else}
      <button class="primary" onclick={onInstall} disabled={downloading || status === null}>
        {downloading ? "Downloading…" : "Download the model"}
      </button>
      <span class="hint">
        The only thing in this app that uses the network, and it is never fetched
        unless you press this.
      </span>
    {/if}
  </div>
</section>

<style>
  .asr {
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 10px 12px;
    background: var(--surface);
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  h3 {
    margin: 0;
    font-size: 0.78rem;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .badge {
    margin-left: auto;
    font-size: 0.72rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: #e8e4da;
    color: #5a5446;
  }
  .badge.good {
    background: #d9f0d4;
    color: #23571c;
  }
  .badge.bad {
    background: var(--danger-soft);
    color: var(--danger-ink);
  }

  .detail {
    margin: 0.45rem 0 0.6rem;
    font-size: 0.8rem;
    line-height: 1.5;
    color: var(--muted-strong);
  }

  .sizes {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    margin: 0 0 0.6rem;
  }
  .sizes div {
    display: flex;
    flex-direction: column;
  }
  .sizes dt {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--muted);
  }
  .sizes dd {
    margin: 0;
    font-size: 0.82rem;
    font-variant-numeric: tabular-nums;
  }
  .sizes a {
    color: var(--accent-ink);
  }

  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--hover);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
  }
  .progress {
    margin: 0.35rem 0 0;
    font-size: 0.72rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 0.6rem;
  }
  .actions button {
    padding: 6px 12px;
    font-size: 0.8rem;
  }
  .actions button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  .actions button.primary:hover:not(:disabled) {
    background: var(--accent-ink);
    color: #fff;
  }
  .hint {
    flex: 1;
    min-width: 180px;
    font-size: 0.72rem;
    line-height: 1.45;
    color: var(--muted);
  }

  .warning {
    margin: 0.4rem 0 0;
    padding: 6px 9px;
    border: 1px solid #fcd34d;
    border-radius: 7px;
    background: #fffbeb;
    color: #92400e;
    font-size: 0.75rem;
    line-height: 1.45;
  }
</style>
