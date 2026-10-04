<script lang="ts">
  /**
   * The notices this app owes, in full.
   *
   * The text arrives over IPC from the Rust catalogue rather than being copied
   * into the frontend, so there is exactly one copy of each licence in the
   * repository and the two cannot drift. The same files are also copied into the
   * application bundle beside the binary, which is what a redistributor reads.
   */
  import * as api from "./api";
  import type { LicenceNotice } from "./types";

  let notices = $state<LicenceNotice[]>([]);
  let error = $state<string | null>(null);

  $effect(() => {
    api
      .licences()
      .then((all) => (notices = all))
      .catch((e) => (error = String(e)));
  });
</script>

<section class="licences">
  <h2>Licences</h2>
  <p class="lede">
    Nihongo Tutor's own code is AGPL-3.0. The stroke data comes from AnimCJK, and the
    readings, glosses, grades and frequency ranks from EDRDG's JMdict and KANJIDIC2
    under CC BY-SA 4.0, with the furigana alignments from JmdictFurigana and the
    passages' segmentation from UniDic through lindera. All of them are modified
    for this course: each notice below says what was taken and what was changed,
    and every one of them ships with the app.
  </p>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#each notices as notice (notice.id)}
    <details>
      <summary>
        <span class="title">{notice.title}</span>
        <span class="licence">{notice.licence}</span>
      </summary>
      <p class="covers">{notice.covers}</p>
      <p class="meta">
        <a href={notice.source} target="_blank" rel="noreferrer noopener">{notice.source}</a>
        <span class="path">shipped as {notice.bundlePath}</span>
      </p>
      <pre>{notice.text}</pre>
    </details>
  {/each}
</section>

<style>
  .licences {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 16px;
    padding: 16px;
  }
  h2 {
    font-size: 1.05rem;
    margin: 0 0 6px;
  }
  .lede {
    margin: 0 0 14px;
    color: var(--muted);
    font-size: 0.86rem;
    max-width: 68ch;
  }
  details {
    border-top: 1px solid var(--line);
    padding: 10px 0;
  }
  summary {
    cursor: pointer;
    display: flex;
    gap: 10px;
    align-items: baseline;
    flex-wrap: wrap;
  }
  summary .title {
    font-weight: 600;
  }
  summary .licence {
    color: var(--muted);
    font-size: 0.82rem;
  }
  .covers {
    margin: 8px 0 4px;
    font-size: 0.86rem;
    max-width: 72ch;
  }
  .meta {
    margin: 0 0 8px;
    font-size: 0.78rem;
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .meta a {
    color: var(--accent);
  }
  .path {
    color: var(--muted);
  }
  pre {
    max-height: 320px;
    overflow: auto;
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 10px;
    font-size: 0.74rem;
    line-height: 1.45;
    white-space: pre-wrap;
  }
</style>
