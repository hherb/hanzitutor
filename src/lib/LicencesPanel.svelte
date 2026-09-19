<script lang="ts">
  /**
   * About and licences.
   *
   * The obligations this screen discharges are real: the stroke outlines come
   * from the Arphic PL fonts, the etymology hints are LGPL, the frequency list
   * and the word list are MIT, and the word readings and definitions are
   * CC BY-SA 4.0 — a share-alike licence, the one whose terms reach the derived
   * data. Every one of them requires its notice to travel with the app, so the
   * full texts are bundled and shown here, not linked out to.
   *
   * Nothing on this screen is fetched: the notices are compiled into the binary
   * (see `src-tauri/src/licences.rs`), so the screen works offline and cannot
   * come up empty in a packaged build. The same files are also copied into the
   * bundle as plain text, and `bundlePath` names where.
   */
  import type { AppInfo, LicenceNotice } from "./types";

  interface Props {
    /** Null only in the moment before the backend answers. */
    info: AppInfo | null;
    notices: LicenceNotice[];
    /** Set if the notices could not be read; shown instead of an empty list. */
    error?: string | null;
  }

  let { info, notices, error = null }: Props = $props();
</script>

<section class="panel">
  <header class="about">
    <div class="mark" lang="zh-Hans" aria-hidden="true">汉</div>
    <div class="identity">
      <h2>{info?.name ?? "Hanzi Tutor"}</h2>
      <p class="version">Version {info?.version ?? "—"}</p>
      <p class="licence">{info?.licence ?? ""}</p>
      <p class="fine">{info?.copyright ?? ""}</p>
      <p class="fine">
        Source: <code>{info?.repository ?? ""}</code>
      </p>
    </div>
  </header>

  <p class="lede">
    Everything this app needs is inside it: the character data, the word
    dictionary, the interface font and every notice below. It makes no network
    requests and downloads nothing, so the addresses here are shown as text
    rather than as links — nothing on this screen will open a browser.
  </p>

  <p class="lede">
    The application's own code is <strong>AGPL-3.0-only</strong>. The data it
    embeds comes from four upstream projects under four further licences, and
    each notice is reproduced in full.
  </p>

  <h3>Notices</h3>
  {#if error}
    <p class="warning">{error}</p>
  {:else if notices.length === 0}
    <p class="warning">The bundled notices have not loaded.</p>
  {/if}
  <ul class="notices">
    {#each notices as notice (notice.id)}
      <li>
        <details>
          <summary>
            <span class="title">{notice.title}</span>
            <span class="chip">{notice.licence}</span>
          </summary>
          <p class="covers">{notice.covers}</p>
          <dl class="where">
            <dt>Source</dt>
            <dd><code>{notice.source}</code></dd>
            <dt>In this bundle</dt>
            <dd>
              <code>{notice.bundlePath}</code>
              <span class="hint">
                — a plain-text copy; the text below is compiled into the app, so
                it is present either way
              </span>
            </dd>
          </dl>
          <pre class="text">{notice.text}</pre>
        </details>
      </li>
    {/each}
  </ul>

  <p class="footnote">
    Data provenance, and what each licence obliges a redistributor to do, is
    recorded in <code>licences/PROVENANCE.md</code> — the first entry above.
  </p>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
    max-width: 900px;
  }

  .about {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .mark {
    flex: none;
    display: grid;
    place-items: center;
    width: 68px;
    height: 68px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--surface);
    font-family: var(--hanzi-font);
    font-size: 2.4rem;
    line-height: 1;
    color: var(--accent-ink);
  }
  .identity h2 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .identity p {
    margin: 2px 0 0;
  }
  .version {
    font-size: 0.82rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .licence {
    font-size: 0.84rem;
    color: var(--muted-strong);
  }
  .fine {
    font-size: 0.76rem;
    color: var(--muted);
  }
  code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.94em;
    word-break: break-all;
  }

  .lede {
    margin: 0;
    font-size: 0.84rem;
    line-height: 1.5;
    color: var(--muted-strong);
  }

  h3 {
    margin: 4px 0 0;
    font-size: 0.95rem;
    font-weight: 650;
  }

  .notices {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .notices li {
    border: 1px solid var(--line);
    border-radius: 11px;
    background: var(--surface);
    overflow: hidden;
  }

  summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 13px;
    cursor: pointer;
    list-style: none;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  summary:hover {
    background: var(--hover);
  }
  summary::before {
    content: "▸";
    flex: none;
    width: 12px;
    color: var(--muted);
    transition: transform 0.12s ease;
  }
  details[open] > summary::before {
    transform: rotate(90deg);
  }
  .title {
    flex: 1;
    font-size: 0.86rem;
    font-weight: 600;
    color: var(--muted-strong);
  }
  .chip {
    flex: none;
    padding: 2px 8px;
    border: 1px solid var(--line);
    border-radius: 20px;
    background: var(--bg);
    font-size: 0.68rem;
    color: var(--muted);
    white-space: nowrap;
  }

  .covers {
    margin: 0;
    padding: 0 13px 10px 37px;
    font-size: 0.8rem;
    line-height: 1.5;
    color: var(--muted-strong);
  }

  .where {
    margin: 0;
    padding: 0 13px 12px 37px;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 2px 10px;
    font-size: 0.76rem;
    color: var(--muted);
  }
  .where dt {
    font-weight: 600;
  }
  .where dd {
    margin: 0;
    min-width: 0;
  }
  .hint {
    color: var(--muted);
  }

  .text {
    margin: 0;
    max-height: 420px;
    overflow: auto;
    padding: 12px 14px;
    border-top: 1px solid var(--line);
    background: var(--bg);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.72rem;
    line-height: 1.45;
    color: var(--muted-strong);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .footnote {
    margin: 0;
    font-size: 0.78rem;
    line-height: 1.5;
    color: var(--muted);
  }

  .warning {
    margin: 0;
    padding: 9px 12px;
    border: 1px solid #fcd34d;
    border-radius: 9px;
    background: #fffbeb;
    color: #92400e;
    font-size: 0.8rem;
  }
</style>
