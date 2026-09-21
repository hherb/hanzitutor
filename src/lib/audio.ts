/**
 * Playing the bundled pronunciation clips.
 *
 * ## Why the webview plays these, and not Rust
 *
 * Recording goes through Rust (`src-tauri/src/capture.rs`) because the samples
 * have to reach the pitch tracker on the same side of the IPC boundary. Playback
 * has the opposite shape: the clip is already a file, and the webview's own
 * `<audio>` element decodes MP3 on every platform this app ships to — macOS,
 * iOS and Android all include an MP3 decoder, and Tauri serves `public/` through
 * the app's own scheme, so nothing is fetched over a network. A Rust playback
 * path would mean linking a decoder and, on the two mobile targets, writing the
 * platform output bridges that `capture.rs` needed for the microphone.
 *
 * The system synthesiser stays where it is for arbitrary text the learner types
 * (`speech.rs`). A phrase on the practice screen with no recording goes to the
 * optional on-device model instead, whose output arrives as samples rather than a
 * file — that is [`playSamples`] below, and it is the one case where this module
 * plays something it did not load from `public/`.
 *
 * ## One element, reused
 *
 * A single `Audio` object is rewound rather than a new one per tap. Two taps in
 * quick succession are then one sound rather than two overlapping ones, which is
 * what a learner drilling a phrase actually wants.
 */

import type { AudioManifest, GradedPhrase } from "./types";

/** The corpora that ship clips, and the licence each one is under. */
export const AUDIO_SOURCES = ["no7z", "harukicoder"] as const;
export type AudioSource = (typeof AUDIO_SOURCES)[number];

const theAudio = new Audio();
/** Loaded manifests, so tapping the same corpus twice reads the file once. */
const manifests = new Map<AudioSource, Promise<AudioManifest>>();

/**
 * Load one corpus's manifest.
 *
 * Relative, not absolute: Tauri serves the frontend from `dist/`, and a leading
 * slash would be resolved against the custom scheme's host rather than the app
 * root on some platforms.
 */
export function loadManifest(source: AudioSource): Promise<AudioManifest> {
  const cached = manifests.get(source);
  if (cached) return cached;

  const pending = fetch(`audio/${source}/manifest.json`)
    .then((response) => {
      if (!response.ok) {
        throw new Error(`audio/${source}/manifest.json: HTTP ${response.status}`);
      }
      return response.json() as Promise<AudioManifest>;
    })
    .catch((cause) => {
      // Dropped from the cache so a transient failure is retryable rather than
      // permanent for the life of the window.
      manifests.delete(source);
      throw cause;
    });

  manifests.set(source, pending);
  return pending;
}

/** Every phrase from every corpus, in corpus order. */
export async function loadAllPhrases(): Promise<GradedPhrase[]> {
  const loaded = await Promise.all(
    AUDIO_SOURCES.map(async (source) => (await loadManifest(source)).phrases),
  );
  return loaded.flat();
}

/**
 * Play one clip.
 *
 * Resolves when playback *starts*, not when it ends: the caller is a tap
 * handler, and making it await a whole utterance would keep a button in a
 * pending state for a second and a half for no reason. Rejects only if the file
 * cannot be played at all.
 */
export function play(url: string): Promise<void> {
  theAudio.pause();
  theAudio.currentTime = 0;
  theAudio.src = url;
  return theAudio.play();
}

/** Stop whatever is playing, if anything. */
export function stop(): void {
  theAudio.pause();
  theAudio.currentTime = 0;
}

/** True while a clip is sounding. */
export function isPlaying(): boolean {
  return !theAudio.paused && !theAudio.ended;
}

/**
 * Play raw mono samples — what the on-device synthesiser returns.
 *
 * A separate path from [`play`] because the two arrive in different forms: a
 * clip is a file the `<audio>` element decodes, while synthesised speech is
 * already samples. Web Audio's `AudioBuffer` is the shortest way to turn the
 * second into sound, and it shares the same output device as the element, so
 * the two cannot overlap on separate outputs.
 *
 * `sampleRate` comes from the model rather than being assumed: MeloTTS reports
 * 44100, but a different voice would report something else and playing it at
 * the wrong rate is heard as the wrong pitch and speed.
 *
 * Resolves when playback *starts*, matching [`play`], so a tap handler is not
 * held for the length of the utterance.
 */
export function playSamples(samples: number[], sampleRate: number): Promise<void> {
  // Stop the element first: the two share an output device, and a clip still
  // sounding underneath synthesised speech is two voices at once.
  stop();

  const context = new AudioContext({ sampleRate });
  const buffer = context.createBuffer(1, samples.length, sampleRate);
  buffer.copyToChannel(Float32Array.from(samples), 0);

  const source = context.createBufferSource();
  source.buffer = buffer;
  source.connect(context.destination);
  // Closed when the utterance ends, so a screen visited repeatedly does not
  // accumulate audio contexts — browsers cap how many can exist at once.
  source.onended = () => void context.close();
  source.start();
  return Promise.resolve();
}
