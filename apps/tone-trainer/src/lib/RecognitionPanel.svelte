<script lang="ts">
  /**
   * What the recogniser heard, read against what the drill asked for.
   *
   * Shown only when a model is installed — `heard` is `null` otherwise, which is
   * how the app ships, so most learners never see this block at all. That is the
   * design rather than a gap: the tone half is complete on its own.
   *
   * ## What it is evidence of
   *
   * *Which syllable was said*, never *how well*. A recogniser carries a strong
   * language-model prior and is built to be robust to the errors a learner makes,
   * so a match here is not praise and must not read as any. The sentence under it
   * is worded in Rust (`heard.detail`), and the label above is deliberately about
   * *syllables* — see the full app's panel for the same wording and why.
   *
   * The characters are resolved by `transcriptDisplay`: a syllable the model heard
   * correctly **at the same tone** shows the character the drill asked for, because
   * within one syllable a character carries no information beyond its reading, while
   * a syllable it heard differently — a different sound, or the same sound at a
   * different tone — keeps the model's own character. That last case is the whole
   * reason the rule has a tone in it: showing the target's character above the
   * model's tone would contradict itself.
   *
   * The letters show the same thing. When the model's transcription is not the text
   * the drill asked for, every syllable is shown with the **dictionary's** reading
   * of the character the model wrote, tone mark included — `cóng` for 从 — so the
   * learner sees what was actually recognised, tonality included. That covers a
   * syllable whose *sound* matched but whose tone did not, which is exactly the
   * difference a tone drill is about. When the transcription is what was asked for,
   * nothing was misheard and the letters stay plain.
   */
  import { transcriptDisplay } from "./transcript";
  import type { Heard } from "./types";

  interface Props {
    heard: Heard;
    /** The characters this drill asked for, one per syllable. */
    targetCharacters: string[];
    /** Why recognition failed, when it did and something was still heard. */
    error: string | null;
  }

  let { heard, targetCharacters, error }: Props = $props();

  const transcript = $derived(transcriptDisplay(heard, targetCharacters));

  /** True when every syllable the model produced was the sound asked for. */
  const allMatched = $derived(
    heard.sameCount && heard.syllables.length > 0 && heard.matched === heard.syllables.length,
  );

  /** True when the transcription was actually compared against the target. */
  const compared = $derived(heard.syllables.length > 0);

  const label = $derived.by(() => {
    if (heard.syllables.length === 0) {
      return heard.text ? "transcribed" : "nothing recognised";
    }
    return allMatched ? "heard the sound asked for" : "heard a different sound";
  });
</script>

<div class="words" class:off={compared && !allMatched}>
  <div class="head">
    <span class="tag">Recognised</span>
    {#if heard.text}
      <span class="glyphs" lang="zh-Hans">{transcript.text}</span>
    {/if}
    {#if transcript.reading}
      <span class="base">{transcript.reading}</span>
    {/if}
    <span class="badge" class:good={allMatched} class:uncertain={!allMatched}>{label}</span>
  </div>

  {#if transcript.substituted}
    <p class="note">
      The model wrote a different character for this syllable — a homophone, so the
      same sound. Only the sound was compared, and it matched; the character is not
      judged and the tone above comes from the pitch.
    </p>
  {/if}

  {#if heard.syllables.length > 0}
    <ul class="syllables">
      {#each heard.syllables as syllable, index (index)}
        <li class:ok={syllable.matches}>
          <span class="plain"
            >{transcript.differs ? syllable.reading || syllable.base : syllable.base}</span
          >
          {#if !syllable.matches}
            <span class="wanted">wanted {syllable.wanted}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <p class="detail">{heard.detail}</p>
  {#if error}
    <p class="error">The syllables could not be recognised this time: {error}</p>
  {/if}
</div>

<style>
  .words {
    padding: 9px 11px;
    border: 1px solid var(--line);
    border-left: 3px solid #2f6f4f;
    border-radius: 9px;
    background: var(--surface);
  }
  .words.off {
    border-left-color: #a8402c;
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 8px;
  }
  .tag {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .glyphs {
    font-family: var(--hanzi-font);
    font-size: 1.3rem;
    line-height: 1.1;
    color: var(--muted-strong);
  }
  /* A tone mark appears when the transcription differs from what was asked for, and
     it is the *character's* dictionary reading — what makes `cóng` identify 从 —
     not the learner's pitch. An identical transcription stays plain; the tone is
     measured from the pitch below. */
  .base {
    font-size: 0.85rem;
    color: var(--muted);
    letter-spacing: 0.02em;
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

  .syllables {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0.45rem 0 0;
    padding: 0;
    list-style: none;
  }
  .syllables li {
    display: inline-flex;
    align-items: baseline;
    gap: 5px;
    padding: 0.1rem 0.45rem;
    border-radius: 999px;
    background: #f4e6e1;
    font-size: 0.78rem;
  }
  .syllables li.ok {
    background: #e3efe0;
  }
  .plain {
    font-weight: 600;
  }
  .wanted {
    color: var(--danger-ink);
    font-size: 0.72rem;
  }

  .detail {
    margin: 0.45rem 0 0;
    font-size: 0.78rem;
    line-height: 1.45;
    color: var(--muted);
  }
  .note {
    margin: 0.4rem 0 0;
    font-size: 0.72rem;
    line-height: 1.45;
    color: var(--muted);
  }
  .error {
    margin: 0.4rem 0 0;
    padding: 5px 8px;
    border-radius: 6px;
    background: var(--danger-soft);
    color: var(--danger-ink);
    font-size: 0.75rem;
  }
</style>
