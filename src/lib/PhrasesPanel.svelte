<script lang="ts">
  /**
   * Graded phrases with bundled pronunciation.
   *
   * The clips are the graded corpus's own recordings, made with CosyVoice2 and
   * shipped in the app — nothing is fetched, and the audio is identical on every
   * platform. They are **synthetic speech**: the corpus generated them with a
   * model, which its attribution asks to be disclosed and a learner is entitled
   * to know, since the alternative is assuming a person recorded them.
   *
   * The disclosure names the model deliberately. An earlier revision of this
   * panel said "MeloTTS", which was wrong once the bundled clips changed source:
   * the text a learner reads about where the audio comes from has to match the
   * audio.
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
    MAX_RATE,
    MIN_RATE,
    loadManifest,
    play,
    playSamples,
    setRate,
    stop,
  } from "./audio";
  import type { GradedPhrase } from "./types";
  import TonedText from "./TonedText.svelte";

  interface Props {
    /** Read the manifests; injected so the panel is testable without a fetch. */
    load?: (source: AudioSource) => Promise<{ phrases: GradedPhrase[] } | null>;
  }

  let { load }: Props = $props();

  /** Which corpus is on screen. */
  let source = $state<AudioSource>("no7z");
  /** The phrases of the corpus on screen. */
  let phrases = $state<GradedPhrase[]>([]);
  let loading = $state(true);
  /** Set when a manifest could not be read, so the screen is not silently empty. */
  let error = $state<string | null>(null);
  /**
   * Set when this build has no list for the corpus at all.
   *
   * A state rather than a failure, and the two are shown differently: the graded
   * readers' recordings are not committed, so a shipped build has no manifest for
   * that corpus and its tab has nothing behind it. Saying so is the honest answer;
   * an error about a syntax error in a file that is not there is not.
   */
  let missing = $state(false);
  /** The phrase id currently sounding, so its row can show it. */
  let playingId = $state<string | null>(null);
  /** Whether the slow take is the one played. */
  let slow = $state(false);
  /**
   * Playback speed, as a multiplier.
   *
   * The corpus ships exactly two takes — normal and slow — so between them there
   * is nothing to adjust, and the normal take was reported as a little quick.
   * This scales whichever take is playing, for both the clip path and the
   * synthesised one, so the two cannot disagree.
   */
  let rate = $state(1);

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
    missing = false;
    try {
      const manifest = load ? await load(next) : await loadManifest(next);
      // `null` is this build having no list for that corpus, which is a state to
      // name rather than an error to report.
      if (manifest === null) {
        missing = true;
        phrases = [];
      } else {
        phrases = manifest.phrases;
      }
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

  // Kept applied rather than set at each play: a learner adjusting the slider
  // while a phrase is sounding should hear it change, not wait for the next tap.
  $effect(() => {
    setRate(rate);
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
        await playSamples(spoken.samples, spoken.sampleRate, rate);
        return;
      }
      await play(clip, rate);
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

  <!-- The controls and the disclosure belong to the clips on this tab, so a
       corpus with none carries neither: there is no take to slow, no speed that
       does anything, and nothing synthetic to disclose. -->
  {#if !missing}
    <div class="controls">
      <label class="slow">
        <input type="checkbox" bind:checked={slow} />
        Play the slow take
      </label>
      <label class="rate">
        <span>Speed</span>
        <input
          type="range"
          min={MIN_RATE}
          max={MAX_RATE}
          step="0.05"
          bind:value={rate}
          aria-label="Playback speed"
        />
        <span class="rate-value">{rate.toFixed(2)}×</span>
      </label>
      {#if phrases.length > 0}
        <span class="size">
          {phrases.length} phrases · {(totalBytes / 1e6).toFixed(2)} MB
        </span>
      {/if}
    </div>

    <p class="disclosure">
      Audio is <strong>synthetic speech</strong> — generated by a voice model
      (CosyVoice2), not recorded by a person.
    </p>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if loading}
    <p class="status">Loading the phrase list…</p>
  {:else if missing}
    <!-- Not an error: this build simply does not carry this corpus's list. Said
         plainly, and pointed at the tab that does have recordings. -->
    <p class="status">
      No recordings are bundled for this corpus in this build.
    </p>
    <p class="status">
      The corpus is listed so the app can offer it once its audio is finished; the
      HSK 1–2 phrases on the other tab are complete and play offline.
    </p>
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
            <!--
              The characters carry their tone colours; the pinyin under them is
              left as it is. Its tokens are **words**, not syllables — `kàndào` is
              看 and 到 run together — and the corpus wrote some of them with the
              tone they are *spoken* with (一只 is `yì zhī` here), so colouring it
              from the marks would put a different tone on 一 in one row and call
              the rest of a word neutral. The board and the word lists have the
              reading split by Rust and colour it there; a phrase has no such split
              and is not guessed at.
            -->
            <span class="text"><TonedText text={phrase.text} /></span>
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

  .rate {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .rate input[type="range"] {
    width: 110px;
  }

  .rate-value {
    font-variant-numeric: tabular-nums;
    min-width: 3.2em;
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
