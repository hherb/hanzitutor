<script lang="ts">
  /** Turns a grading report into something a learner can act on. */
  import { VERDICT_COLOUR, VERDICT_LABEL } from "./render";
  import type { Grade, GradeReport } from "./types";

  interface Props {
    report: GradeReport;
  }

  let { report }: Props = $props();

  const GRADE_LABEL: Record<Grade, string> = {
    excellent: "Excellent",
    good: "Good",
    fair: "Fair",
    poor: "Needs work",
  };

  const percent = (value: number) => Math.round(value * 100);

  /** The three independent measures, kept typed as an array of records. */
  const metrics = $derived([
    { label: "Shape", value: report.shapeScore },
    { label: "Placement", value: report.positionScore },
    { label: "Order", value: report.orderScore },
  ]);

  /** The strokes needing attention, in the order the character is written. */
  const problems = $derived(report.strokes.filter((s) => s.verdict !== "correct"));

  /** Concrete, specific things to fix next, rather than just a number. */
  const advice = $derived.by(() => {
    const notes: string[] = [];
    const list = (verdict: string) =>
      report.strokes
        .filter((s) => s.verdict === verdict)
        .map((s) => s.refIndex + 1);

    if (!report.countOk) {
      const plural = report.expectedStrokes === 1 ? "" : "s";
      notes.push(
        `This character has ${report.expectedStrokes} stroke${plural}, but you wrote ${report.givenStrokes}.`,
      );
    }
    if (report.strayStrokes > 0) {
      // This is the accidental-touch filter, and it is deliberately reported
      // *after* the stroke verdicts and with no causal wording: a mark below the
      // tap threshold is removed before strokes are paired, so it cannot have
      // caused any of the verdicts above. Reading it as "my stroke 8 was thrown
      // away, so stroke 8 was marked wrong" is the natural mistake, and it is
      // worth a sentence to prevent.
      const one = report.strayStrokes === 1;
      notes.push(
        `${report.strayStrokes} ${one ? "mark was" : "marks were"} too small to be a ` +
          `stroke and ${one ? "was" : "were"} ignored as accidental ` +
          `${one ? "touch" : "touches"}. That happens before the strokes are compared, ` +
          `so it is not why anything above was marked wrong.`,
      );
    }

    const missing = list("missing");
    if (missing.length > 0) notes.push(`Stroke ${missing.join(", ")} was never written.`);

    const shape = list("shape_off");
    if (shape.length > 0) {
      notes.push(
        `Stroke ${shape.join(", ")} ${shape.length === 1 ? "has" : "have"} the wrong shape.`,
      );
    }

    const position = list("position_off");
    if (position.length > 0) {
      notes.push(
        `Stroke ${position.join(", ")} ${position.length === 1 ? "is" : "are"} in the wrong place or the wrong size.`,
      );
    }

    const backwards = list("wrong_direction");
    if (backwards.length > 0) {
      notes.push(
        `Stroke ${backwards.join(", ")} ${backwards.length === 1 ? "was" : "were"} drawn back to front.`,
      );
    }

    const order = list("out_of_order");
    if (order.length > 0) {
      notes.push(
        `Stroke ${order.join(", ")} ${order.length === 1 ? "is" : "are"} out of order.`,
      );
    }

    if (report.fit) {
      if (report.fit.scale < 0.85) notes.push("Your character is much smaller than the box.");
      if (report.fit.scale > 1.2) notes.push("Your character overflows the box.");
      if (report.fit.offset > 140) notes.push("The whole character sits off-centre.");
    }

    return notes;
  });
</script>

<section class="panel">
  <header class="score">
    <div class="value" data-grade={report.grade}>
      <strong>{Math.round(report.overall)}</strong><span>/100</span>
    </div>
    <div class="verdict">
      <p class="grade" data-grade={report.grade}>{GRADE_LABEL[report.grade]}</p>
      <div class="badges">
        <span class="badge" class:ok={report.legible} class:bad={!report.legible}>
          {report.legible ? "Legible" : "Not yet legible"}
        </span>
        <span class="badge" class:ok={report.orderCorrect} class:bad={!report.orderCorrect}>
          {report.orderCorrect ? "Stroke order correct" : "Stroke order off"}
        </span>
      </div>
    </div>
  </header>

  <div class="metrics">
    {#each metrics as metric (metric.label)}
      <div class="metric">
        <span class="metric-label">{metric.label}</span>
        <span class="bar"><span style="width: {percent(metric.value)}%"></span></span>
        <span class="metric-value">{percent(metric.value)}%</span>
      </div>
    {/each}
  </div>

  <div class="strokes">
    <span class="strokes-label">Strokes, in writing order</span>
    <ol>
      {#each report.strokes as stroke (stroke.refIndex)}
        <li
          title={`Stroke ${stroke.refIndex + 1}: ${VERDICT_LABEL[stroke.verdict]}`}
          style="--colour: {VERDICT_COLOUR[stroke.verdict]}"
        >
          {stroke.refIndex + 1}
        </li>
      {/each}
    </ol>
  </div>

  {#if advice.length > 0}
    <ul class="advice">
      {#each advice as note (note)}
        <li>{note}</li>
      {/each}
    </ul>
  {:else}
    <p class="perfect">Every stroke was correct and in order.</p>
  {/if}

  {#if problems.length > 0}
    <details class="detail">
      <summary>Stroke-by-stroke detail</summary>
      <ul>
        {#each report.strokes as stroke (stroke.refIndex)}
          <li>
            <span class="chip" style="--colour: {VERDICT_COLOUR[stroke.verdict]}">
              {stroke.refIndex + 1}
            </span>
            <span class="verdict-name">{VERDICT_LABEL[stroke.verdict]}</span>
            <span class="scores">
              shape {percent(stroke.shape)}% · place {percent(stroke.position)}%
            </span>
          </li>
        {/each}
      </ul>
    </details>
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
  }

  .score {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .value {
    display: flex;
    align-items: baseline;
    gap: 2px;
    font-variant-numeric: tabular-nums;
  }
  .value strong {
    font-size: 2.6rem;
    line-height: 1;
    font-weight: 650;
    color: var(--muted-strong);
  }
  .value span {
    font-size: 0.95rem;
    color: var(--muted);
  }
  .value[data-grade="excellent"] strong {
    color: #15803d;
  }
  .value[data-grade="good"] strong {
    color: #4d7c0f;
  }
  .value[data-grade="fair"] strong {
    color: #b45309;
  }
  .value[data-grade="poor"] strong {
    color: #be123c;
  }

  .verdict {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .grade {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
    color: var(--muted-strong);
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .badge {
    padding: 3px 9px;
    border-radius: 999px;
    font-size: 0.76rem;
    font-weight: 550;
    border: 1px solid transparent;
  }
  .badge.ok {
    background: #dcfce7;
    border-color: #86efac;
    color: #166534;
  }
  .badge.bad {
    background: #fee2e2;
    border-color: #fca5a5;
    color: #991b1b;
  }

  .metrics {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .metric {
    display: grid;
    grid-template-columns: 76px 1fr 42px;
    align-items: center;
    gap: 10px;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .metric-label {
    color: var(--muted-strong);
  }
  .metric-value {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--line);
    overflow: hidden;
  }
  .bar > span {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: linear-gradient(90deg, #60a5fa, #2563eb);
  }

  .strokes {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .strokes-label {
    font-size: 0.8rem;
    color: var(--muted);
  }
  .strokes ol {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .strokes li {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 7px;
    font-size: 0.75rem;
    font-weight: 600;
    color: #fff;
    background: var(--colour);
  }

  .advice {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.86rem;
    color: var(--muted-strong);
  }
  .perfect {
    margin: 0;
    font-size: 0.88rem;
    color: #15803d;
    font-weight: 550;
  }

  .detail {
    font-size: 0.82rem;
    color: var(--muted);
  }
  .detail summary {
    cursor: pointer;
  }
  .detail ul {
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .detail li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .chip {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 5px;
    background: var(--colour);
    color: #fff;
    font-size: 0.68rem;
    font-weight: 600;
  }
  .verdict-name {
    color: var(--muted-strong);
    min-width: 108px;
  }
  .scores {
    font-variant-numeric: tabular-nums;
  }
</style>
