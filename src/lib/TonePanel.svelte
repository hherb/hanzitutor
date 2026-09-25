<script lang="ts">
  /**
   * Tone feedback: what the learner's pitch did, drawn against the shape each
   * tone asks for.
   *
   * The drawing is the point. A number cannot show a learner that their tone 4
   * was `56` rather than `51`, but two lines over each other can — and the
   * research is explicit that a tutor must show *why* something scored as it did,
   * the way the stroke panel already does.
   *
   * A character has one syllable and a word has several, so this draws one small
   * chart per syllable with the character above it. That is what makes a word
   * useful rather than just scored: it says *which* syllable was wrong, which a
   * single aggregate number cannot.
   *
   * **The drawing is also the half that can be absent.** Text longer than a word
   * is not scored on tone at all — its syllable boundaries cannot be found from
   * the recording — and what comes back is the transcription alone. That case is
   * `toneScored: false`: the heading, the badge and the charts all go, and what
   * is left is what the model heard. Showing a zero score there would read as a
   * perfectly flat attempt rather than an unmeasured one.
   *
   * The wording of the judgement comes from the Rust side and is not reworded
   * here: `verdict` picks the styling, `detail` is the sentence.
   */
  import type { Grade, ToneResult, ToneSyllableResult } from "./types";
  import { TONE_NAME, transcriptDisplay } from "./transcript";
  import TonedPinyin from "./TonedPinyin.svelte";
  import TonedText from "./TonedText.svelte";

  interface Props {
    result: ToneResult;
  }

  let { result }: Props = $props();

  const GRADE_LABEL: Record<Grade, string> = {
    excellent: "Excellent",
    good: "Good",
    fair: "Fair",
    poor: "Needs work",
  };

  const WIDTH = 132;
  const HEIGHT = 74;
  const PAD = 7;

  /**
   * A contour as an SVG polyline.
   *
   * Both lines arrive on the same 0..1 scale with low pitch at 0, so they share
   * one mapping — which is what makes them comparable when overdrawn. The y axis
   * is flipped because higher pitch belongs higher on the screen.
   */
  function line(points: number[]): string {
    if (points.length < 2) return "";
    const step = (WIDTH - PAD * 2) / (points.length - 1);
    return points
      .map((value, index) => {
        const x = PAD + index * step;
        const y = PAD + (1 - value) * (HEIGHT - PAD * 2);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }

  /** One syllable's chart label: its character, reading and the tone asked for. */
  function label(syllable: ToneSyllableResult): string {
    const name = TONE_NAME[syllable.spoken] ?? "?";
    // When sandhi moved the tone, say what the dictionary had too, or the
    // learner will think the app has misread their dictionary.
    return syllable.citation === syllable.spoken
      ? `tone ${syllable.spoken} · ${name}`
      : `tone ${syllable.spoken} · ${name} (dictionary ${syllable.citation})`;
  }

  const showSummary = $derived(result.syllables.length > 1);

  /**
   * True when a recogniser heard every syllable that was asked for.
   *
   * Only meaningful when `sameCount` — a transcription with the wrong number of
   * syllables cannot be read as "all of them matched", whatever the pairwise
   * comparison says.
   */
  const heardAllMatched = $derived(
    result.heard !== null &&
      result.heard.sameCount &&
      result.heard.syllables.length > 0 &&
      result.heard.matched === result.heard.syllables.length,
  );

  /**
   * True when the transcription was actually read against what was asked for.
   *
   * With nothing to compare — the board's characters could not all be read — the
   * words are a transcription alone. That must not be styled as a failure: there
   * is no comparison to have failed, and a red edge would say the learner got it
   * wrong when the app is the one that could not line the text up.
   */
  const heardCompared = $derived(
    result.heard !== null && result.heard.syllables.length > 0,
  );

  /**
   * What to call the recognition, in words.
   *
   * Deliberately about *syllables* and never about pronunciation. A recogniser
   * repairs a learner's errors toward the likely word — that is what its language
   * model is for — so a match here is not praise and must not read as any. The
   * sentence under it, worded in Rust, says the same thing at length.
   *
   * With no syllables to compare — the transcription could not be read against
   * the board, or nothing was said at all — there is no comparison to name, so
   * this says only what happened rather than claiming a difference.
   */
  const heardLabel = $derived.by(() => {
    if (result.heard === null) return "";
    if (result.heard.syllables.length === 0) {
      return result.heard.text ? "transcribed" : "nothing recognised";
    }
    const many = result.heard.syllables.length > 1;
    return heardAllMatched
      ? `heard the ${many ? "syllables" : "syllable"} asked for`
      : `heard ${many ? "different syllables" : "a different syllable"}`;
  });

  /**
   * What to show for the transcription's characters and its reading.
   *
   * All of the reasoning lives in `transcript.ts`, where it is testable: a
   * syllable the recogniser heard correctly **at the same tone** shows the character
   * the exercise asked for, because within one syllable a character carries no
   * information beyond its reading — while a syllable it heard *differently*, in
   * sound or in tone, keeps the model's character, which is the evidence this block
   * exists to report. The letters follow the same question: when the transcription
   * differs from what was asked for, every syllable takes the dictionary tone mark
   * of the character the model wrote (`cóng` for 从), so the learner sees the
   * tonality of what was recognised; an identical transcription stays plain.
   */
  const transcript = $derived(
    transcriptDisplay(
      result.heard,
      result.syllables.map((syllable) => syllable.ch),
    ),
  );
</script>

<section class="tone" class:uncertain={result.toneScored && result.verdict === "uncertain"}>
  <header>
    <!-- The heading says which judgement this is. For text longer than a word
         there is no tone judgement, and a "Tone" heading over a transcription
         would promise one. -->
    <h3>{result.toneScored ? "Tone" : "Recognition"}</h3>
    {#if !result.toneScored}
      <span class="badge uncertain">Not measured</span>
    {:else if result.verdict === "uncertain"}
      <span class="badge uncertain">Not sure</span>
    {:else}
      <span class="badge {result.grade}">{GRADE_LABEL[result.grade]}</span>
      <span class="score">{Math.round(result.score)}</span>
    {/if}
  </header>

  <p class="detail">{result.detail}</p>

  <!-- What was said, when a speech model is installed. Absent entirely
       otherwise: no empty box and no hint that something is missing, because
       most learners will never install one and the panel they had before is
       still complete. -->
  {#if result.heard}
    <div class="words" class:off={heardCompared && !heardAllMatched}>
      <div class="words-head">
        <span class="words-tag">Recognised</span>
        {#if result.heard.text}
          <!-- The target character stands in where the model wrote a homophone:
               the sound was measured to match, so the spelling was never in
               question. See `transcript.ts` for the rule. -->
          <span class="words-glyphs" lang="zh-Hans"><TonedText text={transcript.text} /></span>
        {/if}
        {#if transcript.reading}
          <span class="words-base">{transcript.reading}</span>
        {/if}
        <span class="badge" class:good={heardAllMatched} class:uncertain={!heardAllMatched}>
          {heardLabel}
        </span>
      </div>
      {#if transcript.substituted}
        <p class="words-note">
          The model wrote a different character for {result.heard.syllables.length > 1
            ? "one of these syllables"
            : "this syllable"} — a homophone, so the same sound. Only the sound was
          compared, and it matched; the character is not judged and the tone below
          comes from the pitch.
        </p>
      {/if}
      {#if result.heard.syllables.length > 0}
        <ul class="words-syllables">
          {#each result.heard.syllables as syllable, index (index)}
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
      <p class="words-detail">{result.heard.detail}</p>
    </div>
  {:else if result.heardError}
    <p class="words-error">
      The syllables could not be recognised this time: {result.heardError}
      {#if result.toneScored}The tone above was measured from the pitch and is unaffected.{/if}
    </p>
  {/if}


  <!-- The charts, the key and the pitch statistics are all the tone half. A
       recognition-only result has none of them and must not draw an empty
       frame: `toneScored` false means the pitch was never looked at. -->
  {#if result.toneScored}
    <div class="syllables">
      {#each result.syllables as syllable (syllable.position)}
        <figure class="syllable" class:wrong={syllable.attempt.verdict === "off_target"}>
          <figcaption>
            <!--
              Coloured by the **spoken** tone, not the dictionary one, and this is
              the one screen where those differ. Everywhere else a colour is a
              memory aid for the character, so 你好 is brown then brown; here the
              panel's own sentence beside the glyph says "tone 2 · rising
              (dictionary 3)", and a colour that contradicted the words next to it
              would read as a bug rather than as the sandhi rule it is illustrating.
            -->
            <span class="glyph" lang="zh-Hans"
              ><TonedText text={syllable.ch} tones={[syllable.spoken]} /></span
            >
            <span class="reading"
              ><TonedPinyin
                text={syllable.reading}
                syllables={[syllable.reading]}
                tones={[syllable.spoken]}
              /></span
            >
            <span class="expected">{label(syllable)}</span>
          </figcaption>

          <svg
            viewBox="0 0 {WIDTH} {HEIGHT}"
            role="img"
            aria-label={`Your pitch for ${syllable.ch} against the shape of tone ${syllable.spoken}`}
          >
            <!-- Five faint bands for the classical five-level scale the tones are
                 described on: decoration, but it gives the eye something to judge
                 the two lines against. -->
            {#each [0.1, 0.3, 0.5, 0.7, 0.9] as level}
              <line
                class="level"
                x1={PAD}
                x2={WIDTH - PAD}
                y1={PAD + (1 - level) * (HEIGHT - PAD * 2)}
                y2={PAD + (1 - level) * (HEIGHT - PAD * 2)}
              />
            {/each}

            <polyline class="reference" points={line(syllable.attempt.reference)} />
            {#if syllable.attempt.contour.length > 1}
              <polyline class="heard" points={line(syllable.attempt.contour)} />
            {/if}
          </svg>
        </figure>
      {/each}
    </div>

    <div class="key">
      <span><i class="swatch reference"></i> the tone</span>
      {#if result.syllables.some((s) => s.attempt.contour.length > 1)}
        <span><i class="swatch heard"></i> you</span>
      {/if}
      {#if showSummary}
        <span class="hint">
          One chart per syllable, so you can see which one went wrong.
        </span>
      {/if}
    </div>
  {/if}

  {#if result.toneScored && result.voicedMs > 0}
    <dl class="stats">
      <div>
        <dt>Pitch</dt>
        <dd>{Math.round(result.medianHz)} Hz</dd>
      </div>
      <div>
        <dt>Voiced</dt>
        <dd>{result.voicedMs} ms</dd>
      </div>
      {#if result.boundariesMs.length > 0}
        <!-- "after", not "at": the number is measured from the first voiced
             frame, so it reads against "Voiced" above. An absolute offset from
             the start of the recording would be mostly the length of the pause
             before the learner spoke, which made a correct split look impossible
             (1463 ms into a 308 ms utterance). -->
        <div>
          <dt>Split after</dt>
          <dd>{result.boundariesMs.join(", ")} ms</dd>
        </div>
      {/if}
    </dl>
  {/if}
</section>

<style>
  .tone {
    border: 1px solid var(--line, #d8d2c4);
    border-radius: 10px;
    padding: 0.85rem 1rem 1rem;
    background: var(--panel, #fffdf7);
  }
  .tone.uncertain {
    border-style: dashed;
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  h3 {
    margin: 0;
    font-size: 0.95rem;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--muted, #6b6455);
  }

  .badge {
    font-size: 0.75rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: #eee8da;
    color: #4a4437;
  }
  .badge.excellent {
    background: #d9f0d4;
    color: #23571c;
  }
  .badge.good {
    background: #e4f0d0;
    color: #3c5518;
  }
  .badge.fair {
    background: #fbeccd;
    color: #6b4c11;
  }
  .badge.poor {
    background: #f7dcd6;
    color: #7a2a1c;
  }
  .badge.uncertain {
    background: #e8e4da;
    color: #5a5446;
  }

  .score {
    margin-left: auto;
    font-variant-numeric: tabular-nums;
    font-size: 1.15rem;
    font-weight: 600;
  }

  .detail {
    margin: 0.5rem 0 0.6rem;
    line-height: 1.45;
    font-size: 0.9rem;
  }

  /* ---- What was recognised ------------------------------------------------
     Set apart from the charts below it, because it is a different kind of claim:
     the charts are a measurement of the voice, and this is a transcription that
     can be wrong in ways the learner did not cause. */
  .words {
    margin: 0 0 0.7rem;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--line, #d8d2c4);
    border-left: 3px solid #2f6f4f;
    border-radius: 6px;
    background: #fbfaf4;
  }
  .words.off {
    border-left-color: #a8402c;
  }
  .words-head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.45rem;
  }
  .words-tag {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted, #6b6455);
  }
  .words-glyphs {
    font-size: 1.3rem;
    line-height: 1.1;
    color: var(--ink, #2c2a24);
  }
  /* A tone mark appears when the transcription differs from what was asked for, and
     it is the *character's* dictionary reading — what makes `cóng` identify 从 —
     not the voice. An identical transcription stays plain; the tone is measured
     from the pitch. */
  .words-base {
    font-size: 0.85rem;
    color: var(--muted, #6b6455);
    letter-spacing: 0.02em;
  }
  .words-head .badge {
    margin-left: auto;
  }
  .words-syllables {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin: 0.4rem 0 0;
    padding: 0;
    list-style: none;
  }
  .words-syllables li {
    display: inline-flex;
    align-items: baseline;
    gap: 0.3rem;
    padding: 0.1rem 0.45rem;
    border-radius: 999px;
    background: #f4e6e1;
    font-size: 0.78rem;
  }
  .words-syllables li.ok {
    background: #e3efe0;
  }
  .plain {
    font-weight: 600;
  }
  .wanted {
    color: #7a2a1c;
    font-size: 0.72rem;
  }
  .words-detail {
    margin: 0.45rem 0 0;
    font-size: 0.78rem;
    line-height: 1.45;
    color: var(--muted, #6b6455);
  }
  /* Why the character above is not the one the model chose. Kept quiet: it is an
     explanation of a display decision, not a finding. */
  .words-note {
    margin: 0.4rem 0 0;
    font-size: 0.72rem;
    line-height: 1.45;
    color: var(--muted, #6b6455);
  }
  .words-error {
    margin: 0 0 0.7rem;
    padding: 0.45rem 0.6rem;
    border-radius: 6px;
    background: #fdf3f0;
    color: #7a2a1c;
    font-size: 0.78rem;
    line-height: 1.45;
  }

  /* One chart per syllable, wrapping when the window is narrow. */
  .syllables {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
  }
  .syllable {
    margin: 0;
    flex: 1 1 132px;
    min-width: 118px;
  }
  figcaption {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.35rem;
    margin-bottom: 0.25rem;
    font-size: 0.78rem;
    color: var(--muted, #6b6455);
  }
  .glyph {
    font-size: 1.25rem;
    line-height: 1;
    color: var(--ink, #2c2a24);
  }
  .reading {
    font-variant-numeric: tabular-nums;
  }
  .expected {
    flex: 1 1 100%;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  svg {
    display: block;
    width: 100%;
    height: auto;
    background: #fdfbf5;
    border-radius: 6px;
  }
  .wrong svg {
    background: #fdf3f0;
  }

  .level {
    stroke: #e6e0d2;
    stroke-width: 1;
  }
  .reference {
    fill: none;
    stroke: #b9b2a0;
    stroke-width: 2;
    stroke-dasharray: 5 4;
  }
  .heard {
    fill: none;
    stroke: #2f6f4f;
    stroke-width: 3;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .wrong .heard {
    stroke: #a8402c;
  }

  .key {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    margin-top: 0.55rem;
    font-size: 0.8rem;
    color: var(--muted, #6b6455);
  }
  .key span {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }
  .key .hint {
    flex: 1 1 100%;
    font-size: 0.75rem;
  }
  .swatch {
    width: 14px;
    height: 3px;
    border-radius: 2px;
    display: inline-block;
  }
  .swatch.reference {
    background: repeating-linear-gradient(to right, #b9b2a0 0 5px, transparent 5px 9px);
  }
  .swatch.heard {
    background: #2f6f4f;
  }

  .stats {
    display: flex;
    gap: 1.25rem;
    margin: 0.7rem 0 0;
  }
  .stats div {
    display: flex;
    flex-direction: column;
  }
  .stats dt {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--muted, #6b6455);
  }
  .stats dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
    font-size: 0.9rem;
  }
</style>
