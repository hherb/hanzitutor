<script lang="ts">
  /**
   * Graded phrases with bundled pronunciation.
   *
   * The clips are MeloTTS synthesis, produced at build time by
   * `crates/hanzi-say` and shipped in the app — nothing is fetched, and the
   * audio is identical on every platform. They are **synthetic speech**, and the
   * panel says so rather than letting a learner assume a person recorded them.
   *
   * ## Two corpora, two licences, two lists
   *
   * `no7z` is CC BY-SA 4.0 and `harukicoder` is CC BY 4.0. They are offered as
   * separate tabs rather than one merged list, so that what a learner is hearing
   * always has one identifiable source and one attributable licence. The notices
   * are on the About screen with the rest of them.
   */
  import { onMount } from "svelte";
  import * as api from "./api";
  import {
    AUDIO_SOURCES,
    type AudioSource,
    loadManifest,
    play,
    playSamples,
    stop,
  } from "./audio";
  import type { GradedPhrase } from "./types";

  interface Props {
    /** Read the manifests; injected so the panel is testable without a fetch. */
    load?: (source: AudioSource) => Promise<{ phrases: GradedPhrase[] }>;
  }

  let { load }: Props = $props();

  /** Which corpus is on screen. */
  let source = $state<AudioSource>("no7z");
  /** The phrases of the corpus on screen. */
  let phrases = $state<GradedPhrase[]>([]);
  let loading = $state(true);
  /** Set when a manifest could not be read, so the screen is not silently empty. */
  let error = $state<string | null>(null);
  /** The phrase id currently sounding, so its row can show it. */
  let playingId = $state<string | null>(null);
  /** Whether the slow take is the one played. */
  let slow = $state(false);

  /**
   * Load the selected corpus.
   *
   * Re-run on every switch rather than preloading both: a learner on a phone
   * looking at one list has no use for the other in memory, and the manifests
   * are a few kilobytes each.
   */
  async function loadSource(next: AudioSource) {
    loading = true;
    error = null;
    try {
      const manifest = load ? await load(next) : await loadManifest(next);
      phrases = manifest.phrases;
    } catch (cause) {
      error = `Could not read the ${next} phrase list: ${cause}`;
      phrases = [];
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void loadSource(source);
  });

  function select(next: AudioSource) {
    if (next === source) return;
    stop();
    playingId = null;
    source = next;
    void loadSource(next);
  }

  async function speak(phrase: GradedPhrase) {
    playingId = phrase.id;
    try {
      // A phrase with no recording is spoken by the app itself, using the
      // optional model from the settings screen. That resolves `null` when no
      // model is installed — not an error, just nothing to say — and the message
      // says where to get it rather than failing silently.
      const clip = slow ? phrase.audio?.slow : phrase.audio?.normal;
      if (!clip) {
        const spoken = await api.saySpeak(phrase.text);
        if (!spoken) {
          error =
            "That phrase has no recording, and no speech model is installed. " +
            "See Settings → Speaking phrases with no recording.";
          playingId = null;
          return;
        }
        await playSamples(spoken.samples, spoken.sampleRate);
        return;
      }
      await play(clip);
    } catch (cause) {
      // A clip that will not play is worth saying out loud rather than leaving a
      // row that looks like it played and made no sound.
      error = `Could not play ${phrase.text}: ${cause}`;
      playingId = null;
    }
  }

  /** Total size of the corpus on screen, for the provenance line. */
  const totalBytes = $derived(
    phrases.reduce((sum, phrase) => sum + (phrase.bytes ?? 0), 0),
  );
</script>

<div class="phrases">
  <header class="tabs" role="group" aria-label="Phrase source">
    {#each AUDIO_SOURCES as option (option)}
      <button
        type="button"
        class:on={source === option}
        onclick={() => select(option)}
      >
        {option === "no7z" ? "Graded sentences" : "Graded readers"}
      </button>
    {/each}
  </header>

  <div class="controls">
    <label class="slow">
      <input type="checkbox" bind:checked={slow} />
      Play the slow take
    </label>
    {#if phrases.length > 0}
      <span class="size">
        {phrases.length} phrases · {(totalBytes / 1e6).toFixed(2)} MB
      </span>
    {/if}
  </div>

  <p class="disclosure">
    Audio is <strong>synthetic speech</strong> (MeloTTS), generated when the app
    was built — not a recording of a person.
  </p>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if loading}
    <p class="status">Loading the phrase list…</p>
  {:else if phrases.length === 0}
    <p class="status">No phrases in this set.</p>
  {:else}
    <ul class="list">
      {#each phrases as phrase (phrase.id)}
        <li class:playing={playingId === phrase.id}>
          <button
            type="button"
            class="phrase"
            lang="zh-Hans"
            onclick={() => void speak(phrase)}
            aria-label="Play {phrase.text}"
          >
            <span class="text">{phrase.text}</span>
            {#if phrase.pinyin}
              <span class="pinyin">{phrase.pinyin}</span>
            {/if}
            {#if phrase.translation}
              <span class="translation">{phrase.translation}</span>
            {/if}
          </button>
          {#if !phrase.audio?.normal}
            <span class="level" title="No recording — spoken by the app">synth</span>
          {/if}
          <span class="level">{phrase.level}</span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .phrases {
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
    padding: 14px 16px 18px;
    overflow-y: auto;
  }

  .tabs {
    display: flex;
    gap: 6px;
    margin-bottom: 10px;
  }

  .tabs button {
    flex: 1;
    padding: 7px 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: transparent;
    font: inherit;
    font-size: 0.85rem;
    color: var(--muted-strong);
    cursor: pointer;
  }

  .tabs button:hover {
    background: var(--hover);
  }

  .tabs button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  .controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 8px;
    font-size: 0.82rem;
    color: var(--muted-strong);
  }

  .slow {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }

  .disclosure {
    margin: 0 0 12px;
    font-size: 0.78rem;
    color: var(--muted-strong);
  }

  .status,
  .error {
    margin: 8px 0;
    font-size: 0.88rem;
  }

  .error {
    color: var(--danger, #b3261e);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .list li {
    display: flex;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0 10px 0 0;
  }

  .list li.playing {
    border-color: var(--accent);
  }

  .phrase {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 9px 10px;
    border: 0;
    background: transparent;
    font: inherit;
    text-align: left;
    color: inherit;
    cursor: pointer;
  }

  .phrase:hover {
    background: var(--hover);
  }

  .text {
    font-size: 1.05rem;
  }

  .pinyin {
    font-size: 0.82rem;
    color: var(--muted-strong);
  }

  .translation {
    font-size: 0.82rem;
    color: var(--muted);
  }

  .level {
    flex-shrink: 0;
    font-size: 0.75rem;
    color: var(--muted);
    text-transform: capitalize;
  }
</style>
