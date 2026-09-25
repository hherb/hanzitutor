<script lang="ts">
  /**
   * The pitch chart: what the learner's voice did, drawn against the shape each
   * tone asks for.
   *
   * **Lifted from the full app's `src/lib/TonePanel.svelte`**, and that is the one
   * genuinely shared piece of interface between the two apps. What survived the
   * lift is the drawing, the badge, the sentence the scorer wrote and the pitch
   * statistics; what did not is the board's business — recognition (the trainer
   * draws that in its own block) and the sandhi wording.
   *
   * **One chart per syllable**, which is what makes a *word* useful rather than
   * merely scored: it says which syllable went wrong, and a single aggregate number
   * cannot. A character is simply the one-syllable case.
   *
   * The wording of the judgement is **not** reworded here. `verdict` picks the
   * styling and `detail` is the sentence the Rust scorer wrote, so a judgement is
   * expressed in exactly one place across both apps.
   */
  import type { ScoreResult, SyllableResult } from "./types";

  interface Props {
    result: ScoreResult;
  }

  let { result }: Props = $props();

  const GRADE_LABEL: Record<string, string> = {
    excellent: "Excellent",
    good: "Good",
    fair: "Fair",
    poor: "Needs work",
  };

  /** Which tone the learner was aiming for, in words rather than a number. */
  const TONE_NAME: Record<number, string> = {
    1: "high level",
    2: "rising",
    3: "dipping",
    4: "falling",
    5: "neutral",
  };

  const WIDTH = 200;
  const HEIGHT = 110;
  const PAD = 10;

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

  /** One syllable's caption: its character, reading, and the tone asked for. */
  function label(syllable: SyllableResult): string {
    const name = TONE_NAME[syllable.spoken] ?? "?";
    // When sandhi moved the tone, say what the dictionary had too, or the learner
    // will think the app has misread their dictionary.
    return syllable.citation === syllable.spoken
      ? `tone ${syllable.spoken} · ${name}`
      : `tone ${syllable.spoken} · ${name} (dictionary ${syllable.citation})`;
  }

  /** A word has more than one chart, and the heading should say so. */
  const many = $derived(result.syllables.length > 1);
</script>

<section class="tone" class:uncertain={result.verdict === "uncertain"} class:wrong={result.verdict === "off_target"}>
  <header>
    <h3>{many ? "Tones" : "Tone"}</h3>
    {#if result.verdict === "uncertain"}
      <span class="badge uncertain">Not sure</span>
    {:else}
      <span class="badge {result.grade}">{GRADE_LABEL[result.grade]}</span>
      <span class="score">{Math.round(result.score)}</span>
    {/if}
  </header>

  <p class="detail">{result.detail}</p>

  <div class="syllables">
    {#each result.syllables as syllable (syllable.position)}
      <figure class="syllable" class:wrong={syllable.attempt.verdict === "off_target"}>
        <figcaption>
          <span class="glyph" lang="zh-Hans">{syllable.ch}</span>
          <span class="reading">{syllable.reading}</span>
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
    {:else}
      <span class="hint">No pitch line: there was not enough voice to measure one.</span>
    {/if}
    {#if many}
      <span class="hint">One chart per syllable, so you can see which one went wrong.</span>
    {/if}
  </div>

  {#if result.voicedMs > 0}
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
        <!-- "after", not "at": the number is measured from the first voiced frame,
             so it reads against "Voiced" above. -->
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
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 0.85rem 1rem 1rem;
    background: var(--surface);
  }
  .tone.uncertain {
    border-style: dashed;
  }
  .tone.wrong {
    border-color: #fca5a5;
  }

  header {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  h3 {
    margin: 0;
    font-size: 0.95rem;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--muted);
  }

  /* One chart per syllable, wrapping when the window is narrow. A character is the
     one-chart case and looks the same as it always did. */
  .syllables {
    display: flex;
    flex-wrap: wrap;
    gap: 0.7rem;
  }
  .syllable {
    margin: 0;
    flex: 1 1 190px;
    min-width: 150px;
  }
  figcaption {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.35rem;
    margin-bottom: 0.25rem;
  }
  .glyph {
    font-family: var(--hanzi-font);
    font-size: 1.6rem;
    line-height: 1.1;
    color: var(--muted-strong);
  }
  .reading {
    font-weight: 600;
    color: var(--accent-ink);
    font-variant-numeric: tabular-nums;
  }
  .expected {
    flex: 1 1 100%;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--muted);
  }

  .badge {
    margin-left: auto;
    font-size: 0.75rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: #e8e4da;
    color: #5a5446;
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
    background: var(--danger-soft);
    color: var(--danger-ink);
  }

  .score {
    font-variant-numeric: tabular-nums;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .detail {
    margin: 0.5rem 0 0.6rem;
    line-height: 1.45;
    font-size: 0.9rem;
  }

  svg {
    display: block;
    width: 100%;
    height: auto;
    background: #fdfbf5;
    border-radius: 8px;
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
    color: var(--muted);
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
    color: var(--muted);
  }
  .stats dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
    font-size: 0.9rem;
  }
</style>
