<script lang="ts">
  /**
   * The grammar composition exercise: one pattern, and a sentence of the learner's own.
   *
   * ## Why this exists beside the drill and the written cloze
   *
   * N20 made the course ask a learner to *choose* the pattern a sentence takes and N22
   * made it ask them to *write* it — both inside the corpus's own sentence, with the
   * hole already cut. Neither asks for a sentence the learner thought of, and
   * `ROADMAP_NIHONGO.md`'s known weak spots has carried that as the honest remaining
   * gap since the grammar course shipped. This is the stage that asks.
   *
   * ## There is no answer, so there is no mark
   *
   * The corpus never wrote the sentence being asked for, so nothing can be graded
   * against it. What Rust sends back is a **checklist**: whether the pattern's own
   * text is in the sentence, how many characters it holds, whether all of it could be
   * read as Japanese, which characters this course has no card for, and whether one of
   * the corpus's own sentences was written back. The only decision taken anywhere is
   * whether the input is a sentence at all — and `limitationNote` is the line that says
   * outright what the check cannot see.
   *
   * ## The prompt carries the pattern, which is the opposite of N22
   *
   * The written cloze could not be given its answer. This one cannot avoid giving the
   * pattern, because the pattern *is* the question; the corpus's own sentences travel
   * with it as a model a beginner can borrow the shape of, and the learner's own
   * earlier sentences for the same pattern are drawn above the box so the exercise
   * accumulates writing rather than a score. That is the app's fifth file, read back.
   *
   * ## What is spoken
   *
   * The corpus's own model sentences, and — since N24 — **the learner's own sentence**,
   * which is a deliberate widening of `HANDOVER_NIHONGO.md` invariant 27 at the
   * maintainer's word: hearing what you have just written is how you notice it is wrong.
   * What is handed to the voice is not what was typed but what Rust read it as
   * (`speak_sentence`), so romaji is heard as kana and a sentence the app could not read
   * is a message rather than a noise.
   *
   * ## And the model check, which is opt-in and off until it is configured
   *
   * The same sentence can be sent to an OpenAI-compatible endpoint — a local server or a
   * cloud one, the learner's choice of address, model and key — for a second opinion.
   * Four things about it are visible on the screen rather than in a document: it says
   * **what leaves the machine** before anything does, it names **which model and host**
   * answered, it echoes the model's **own** word for its judgement instead of mapping it
   * onto this app's, and it says in one line that the model is not this app and can be
   * wrong. Nothing about the answer is recorded anywhere (invariant 43).
   */
  import Icon from "./Icon.svelte";
  import SpeakButton from "./SpeakButton.svelte";
  import * as api from "./api";
  import type {
    CompositionOutcome,
    CompositionPrompt,
    CompositionSize,
    ModelStatus,
    ModelVerdict,
  } from "./types";
  import { canHear, voiceNote, type VoiceStatus } from "./speech";
  import {
    attributionLine,
    correctedLine,
    disclosureLine,
    explanationLine,
    failureLine,
    reviewHeadline,
    settingsNote,
  } from "./model";
  import { needsRuby, spokenRuby } from "./words";
  import {
    canSubmit,
    checks,
    instructionLine,
    keptLine,
    limitationNote,
    ownNote,
    poolNote,
  } from "./composition";

  interface Props {
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice: VoiceStatus;
    /** Back to the course — the way a stage always leads back to its list. */
    onleave: () => void;
  }

  let { voice, onleave }: Props = $props();

  let prompt = $state<CompositionPrompt | null>(null);
  let outcome = $state<CompositionOutcome | null>(null);
  let typed = $state("");
  let size = $state<CompositionSize | null>(null);
  /** How many sentences this sitting has kept, which is the whole of its scoreboard. */
  let kept = $state(0);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /**
   * The learner's own sentence, read back.
   *
   * A control of this screen's own rather than `SpeakButton`, because what is spoken is
   * not the text in the box: it is the text **as Rust read it**, and only Rust knows
   * that. The no-voice rule is still the shared one (`canHear`/`voiceNote`), which is
   * what that pair of functions exists for.
   */
  let hearError = $state<string | null>(null);

  /** The model check: what is configured, what the learner is typing, what came back. */
  let model = $state<ModelStatus | null>(null);
  let endpoint = $state("");
  let modelName = $state("");
  let key = $state("");
  let saveError = $state<string | null>(null);
  let checking = $state(false);
  let checkError = $state<string | null>(null);
  let verdict = $state<ModelVerdict | null>(null);
  let showingSettings = $state(false);

  /**
   * A ticket per ask, so an earlier request cannot land on top of a later one.
   *
   * `Next pattern` can be pressed while the answer's round trip is still in flight,
   * and without this the verdict for the last sentence could be drawn under the new
   * pattern — a checklist about a question that is no longer on screen.
   */
  let ticket = 0;

  async function next() {
    const mine = ++ticket;
    try {
      busy = true;
      error = null;
      const drawn = await api.nextGrammarComposition();
      if (mine !== ticket) return;
      prompt = drawn;
      outcome = null;
      typed = "";
    } catch (e) {
      if (mine === ticket) error = String(e);
    } finally {
      if (mine === ticket) busy = false;
    }
  }

  async function submit() {
    const askedPattern = prompt;
    if (!askedPattern || outcome !== null || !canSubmit(typed)) return;
    try {
      busy = true;
      error = null;
      // Recorded against the pattern the prompt named, never against the text: what
      // the app could see is decided in Rust, from the artifact. A failed write does
      // not lose the sentence — it is kept in memory and the warning says so
      // (invariant 15).
      const answered = await api.recordGrammarComposition(askedPattern.point, typed);
      if (prompt !== askedPattern) return;
      outcome = answered;
      if (answered.verdict.kept) kept += 1;
    } catch (e) {
      if (prompt === askedPattern) error = String(e);
    } finally {
      busy = false;
    }
  }

  /** Hear the sentence as this app read it. */
  async function hear() {
    try {
      hearError = null;
      await api.speakSentence(typed);
    } catch (e) {
      hearError = String(e);
    }
  }

  function fillSettings(status: ModelStatus) {
    model = status;
    endpoint = status.endpoint;
    modelName = status.model;
    key = "";
    showingSettings = !status.configured;
  }

  /** Save the address, the model name and — when one was typed — the key. */
  async function save(forget = false) {
    try {
      saveError = null;
      // `null` means "leave the stored key alone": the field is a placeholder when a key
      // is saved, so an untouched field must not clear it. `""` is the explicit request
      // to forget it, which the button beside the field makes.
      const status = await api.saveModelSettings(
        endpoint,
        modelName,
        forget ? "" : key.trim() === "" ? null : key,
      );
      // Saving switches the fold to what it will send, which is the confirmation: a
      // "Saved." line under fields that are no longer drawn would never be seen.
      fillSettings(status);
    } catch (e) {
      saveError = String(e);
    }
  }

  /** Ask the model about the sentence in the box. */
  async function check() {
    const askedPattern = prompt;
    if (!askedPattern || !canSubmit(typed) || checking) return;
    try {
      checking = true;
      checkError = null;
      const answered = await api.checkSentence(askedPattern.point, typed);
      if (prompt !== askedPattern) return;
      verdict = answered;
    } catch (e) {
      if (prompt === askedPattern) checkError = failureLine(e);
    } finally {
      checking = false;
    }
  }

  $effect(() => {
    void (async () => {
      try {
        model = await api.modelStatus();
        endpoint = model.endpoint;
        modelName = model.model;
        showingSettings = !model.configured;
      } catch {
        // The fold says nothing about a status it could not read; the check itself
        // reports the failure that matters.
      }
    })();
  });

  // An answer belongs to the sentence it was asked about: editing the sentence drops
  // it, so a model's verdict for old text is never drawn under new text.
  $effect(() => {
    void typed;
    verdict = null;
    checkError = null;
  });

  $effect(() => {
    void (async () => {
      try {
        size = await api.grammarCompositionSize();
      } catch {
        // The count is a courtesy: a stage that cannot say how much of the course can
        // be written about still asks, and the question fetch reports its own error.
      }
    })();
  });

  void next();
</script>

<section class="composition">
  <header class="head">
    <!-- The way back, named for the list it returns to, as every stage's first
         control is. -->
    <button class="back" onclick={onleave} title="Back to the grammar course">
      <Icon name="back" />
      <span>Grammar</span>
    </button>
    <span class="score">{kept === 0 ? "" : kept === 1 ? "1 sentence kept" : `${kept} sentences kept`}</span>
  </header>

  {#if size}
    <p class="pool">{poolNote(size)}</p>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if prompt}
    <p class="ask">{instructionLine(prompt.pattern)}</p>
    <p class="pattern-line" lang="ja">
      {#if prompt.reading}<span class="reading">{prompt.reading}</span>{/if}
      <span class="pattern">{prompt.pattern}</span>
    </p>
    <p class="meaning">{prompt.meaning}</p>
    <p class="formation">
      <span class="label">Written with</span>
      {prompt.formation}
    </p>
    {#if prompt.notes}
      <p class="note">{prompt.notes}</p>
    {/if}

    <!--
      The learner's own writing for this pattern, above the box rather than below it:
      it is the thing this exercise accumulates, and it belongs where they can see it
      while they write the next one.
    -->
    {#if prompt.written.length > 0}
      <div class="own">
        <p class="own-head">{ownNote(prompt.written)}</p>
        <ul class="own-list">
          {#each prompt.written as sentence, i (i)}
            <li lang="ja">{sentence}</li>
          {/each}
        </ul>
      </div>
    {/if}

    <!--
      The corpus's own sentences for this pattern: the model, folded away because a
      learner composing their own sentence should not have one in front of them
      unless they want it. Copying one is noticed by the check rather than punished.
    -->
    <details class="model">
      <summary>How the corpus uses it</summary>
      <ol class="examples">
        {#each prompt.examples as example, i (i)}
          <li>
            <p class="ja" lang="ja">
              {#each example.ruby as part, j (j)}
                {#if needsRuby(part)}<ruby>{part.ruby}<rt>{part.rt}</rt></ruby>{:else}{part.ruby}{/if}
              {/each}
            </p>
            <p class="en" lang="en">{example.en}</p>
            <!-- The model is drawn as the corpus wrote it and spoken as the corpus
                 read it: the prompt's examples are the artifact's own sentences, so
                 their readings are here to be joined (`spokenRuby`) rather than a
                 second field carrying them. The voice would otherwise parse
                 `お腹痛いの？` as お + 腹痛(ふくつう) + い. -->
            <SpeakButton text={spokenRuby(example.ruby)} {voice} label="Hear it" note={false} />
          </li>
        {/each}
      </ol>
    </details>

    <div class="answer">
      <!-- The label is above the field at every width: measured on the written-cloze
           screen at 390px (N22), a label beside the row pushes the one control below
           the box the learner has just typed in. -->
      <label for="own-sentence">Your sentence, in Japanese</label>
      <div class="row">
        <textarea
          id="own-sentence"
          rows="2"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          placeholder="いぬのほうがねこよりすきです。 ・ inu no houga neko yori suki desu."
          bind:value={typed}
          disabled={outcome !== null}
        ></textarea>
        <button
          class="submit"
          onclick={() => void submit()}
          disabled={outcome !== null || busy || !canSubmit(typed)}>Check</button
        >
      </div>
      <!--
        The learner's own sentence, read back. Shown as soon as there is something in
        the box, before or after the checklist: hearing what you wrote is how you notice
        it is wrong. What is spoken is the read-back form, decided in Rust.
      -->
      {#if canSubmit(typed)}
        <div class="hear">
          <button
            class="hear-it"
            type="button"
            onclick={() => void hear()}
            disabled={!canHear(voice)}
            title={voiceNote(voice) ?? "Say your sentence in the system's Japanese voice"}
          >
            Hear it
          </button>
          {#if voiceNote(voice)}
            <span class="note">{voiceNote(voice)}</span>
          {/if}
          {#if hearError}
            <span class="error" role="alert">{hearError}</span>
          {/if}
        </div>
      {/if}
    </div>

    {#if outcome}
      <ul class="checks">
        {#each checks(outcome.verdict, typed) as line (line.text)}
          <li class={line.state}>{line.text}</li>
        {/each}
      </ul>

      <p class="kept">{keptLine(outcome.verdict, outcome.written.length)}</p>
      {#if outcome.warning}
        <p class="warn" role="status">{outcome.warning}</p>
      {/if}

      <!-- The way on sits directly under the answer, as it does on the other two
           stages: an explanation above this row puts it a scroll away. -->
      <div class="after">
        <button class="next" onclick={next} disabled={busy}>Next pattern</button>
      </div>

      <p class="limitation">{limitationNote()}</p>
    {:else}
      <p class="hint">A sentence of your own — one this app has never seen.</p>
    {/if}

    <!--
      The second opinion, after everything this app decided: folded away, opt-in, and
      off until the learner names an endpoint. The corpus's model sentences above carry a
      `Hear it`; this asks a model about the learner's own.
    -->
    <details class="model-check" open={showingSettings}>
      <summary>Ask a model about this sentence</summary>
      {#if !model}
        <p class="hint">Loading the model settings…</p>
      {:else if showingSettings || !model.configured}
        <p class="hint">{settingsNote()}</p>
        <div class="fields">
          <label for="model-endpoint">Endpoint</label>
          <input
            id="model-endpoint"
            type="text"
            autocomplete="off"
            autocapitalize="off"
            spellcheck="false"
            placeholder="http://127.0.0.1:11434/v1 ・ https://api.openai.com/v1"
            bind:value={endpoint}
          />
          <label for="model-name">Model</label>
          <input
            id="model-name"
            type="text"
            autocomplete="off"
            autocapitalize="off"
            spellcheck="false"
            placeholder="llama3.1 ・ gpt-4o-mini"
            bind:value={modelName}
          />
          <label for="model-key">API key <span class="optional">— a local server needs none</span></label>
          <input
            id="model-key"
            type="password"
            autocomplete="off"
            placeholder={model.keySet ? "a key is saved — leave empty to keep it" : "sk-…"}
            bind:value={key}
          />
          <div class="row">
            <button class="save" onclick={() => void save()} disabled={checking}>Save</button>
            {#if model.keySet}
              <button class="forget" onclick={() => void save(true)} disabled={checking}>
                Forget the saved key
              </button>
            {/if}
            {#if model.configured}
              <button class="forget" onclick={() => (showingSettings = false)}>Done</button>
            {/if}
          </div>
          {#if saveError}<p class="error" role="alert">{saveError}</p>{/if}
        </div>
      {:else}
        <p class="hint">{disclosureLine(model, prompt.pattern)}</p>
        <div class="row">
          <button class="save" onclick={() => void check()} disabled={checking || !canSubmit(typed)}>
            {checking ? "Asking…" : "Check with the model"}
          </button>
          <button class="forget" onclick={() => (showingSettings = true)}>Change it</button>
        </div>
        {#if checkError}
          <p class="error" role="alert">{checkError}</p>
        {/if}
        {#if verdict}
          <div class="answer">
            <p class="headline">{reviewHeadline(verdict.review)}</p>
            {#if correctedLine(verdict.review)}
              <p class="corrected" lang="ja">{correctedLine(verdict.review)}</p>
            {/if}
            {#if explanationLine(verdict.review)}
              <p class="explanation">{explanationLine(verdict.review)}</p>
            {/if}
            <p class="attribution">{attributionLine(verdict)}</p>
          </div>
        {/if}
      {/if}
    </details>
  {:else if !error}
    <p class="hint">{busy ? "Choosing a pattern…" : "Loading…"}</p>
  {/if}
</section>

<style>
  .composition {
    max-width: 70ch;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
  }

  .back {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 44px;
    padding: 5px 12px 5px 9px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .back:hover {
    border-color: var(--accent);
  }

  .score {
    margin-left: auto;
    font-size: 0.85rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .pool {
    margin: 0.2rem 0 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .ask {
    margin: 1rem 0 0.2rem;
    font-size: 0.95rem;
  }

  .pattern-line {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pattern-line .pattern {
    font-size: 1.6rem;
  }

  .pattern-line .reading {
    font-size: 0.85rem;
    color: var(--muted);
  }

  .meaning,
  .formation,
  .note {
    margin: 0.4rem 0 0;
    font-size: 0.9rem;
    color: var(--muted);
  }

  .formation .label {
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.72rem;
    margin-right: 0.35rem;
  }

  .own {
    margin: 0.9rem 0 0;
    padding: 0.6rem 0.8rem;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel);
  }

  .own-head {
    margin: 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .own-list {
    list-style: none;
    margin: 0.35rem 0 0;
    padding: 0;
    display: grid;
    gap: 0.2rem;
    font-size: 1.05rem;
  }

  .model {
    margin: 0.9rem 0 0;
    font-size: 0.9rem;
  }

  .model summary {
    cursor: pointer;
    min-height: 44px;
    display: flex;
    align-items: center;
    color: var(--muted);
  }

  .examples {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.7rem;
  }

  .examples .ja {
    margin: 0;
    font-size: 1.15rem;
    line-height: 2.1;
  }

  .examples .ja rt {
    font-size: 0.65rem;
    color: var(--muted);
  }

  .examples .en {
    margin: 0.1rem 0 0.3rem;
    font-size: 0.9rem;
    color: var(--muted);
  }

  .answer {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 1rem;
  }

  .answer label {
    font-size: 0.85rem;
    color: var(--muted);
  }

  .answer .row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .answer textarea {
    flex: 1;
    min-width: 0;
    min-height: 60px;
    padding: 8px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    resize: vertical;
  }

  .submit {
    min-height: 44px;
    padding: 6px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    cursor: pointer;
  }

  .submit:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .checks {
    list-style: none;
    margin: 1rem 0 0;
    padding: 0;
    display: grid;
    gap: 0.35rem;
    font-size: 0.95rem;
  }

  .checks li {
    padding-left: 1.4rem;
    position: relative;
  }

  .checks li::before {
    position: absolute;
    left: 0;
    top: 0;
  }

  .checks .ok::before {
    content: "✓";
    color: var(--ok);
  }

  .checks .note::before {
    content: "•";
    color: var(--muted);
  }

  .checks .warn::before {
    content: "!";
    color: var(--bad);
  }

  .kept {
    margin: 0.8rem 0 0;
    font-size: 0.95rem;
  }

  .warn {
    margin: 0.4rem 0 0;
    font-size: 0.85rem;
    color: var(--bad);
  }

  .after {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    margin-top: 0.8rem;
  }

  .next {
    min-height: 44px;
    padding: 6px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    cursor: pointer;
  }

  .next:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .limitation {
    margin: 0.8rem 0 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  /* The learner's own sentence, read back. */
  .hear {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 0.5rem;
  }

  .hear-it {
    min-height: 44px;
    padding: 6px 14px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .hear-it:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .hear-it:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .hear .note {
    font-size: 0.8rem;
    color: var(--muted);
    max-width: 40ch;
  }

  /* The model check: folded away, and the fields are one per line so a long endpoint
     and a long model name are both readable while they are typed. */
  .model-check {
    margin: 1.2rem 0 0;
    border-top: 1px solid var(--line);
    padding-top: 0.6rem;
    font-size: 0.9rem;
  }

  .model-check summary {
    cursor: pointer;
    min-height: 44px;
    display: flex;
    align-items: center;
    color: var(--muted);
  }

  .model-check .fields {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 0.4rem;
  }

  .model-check .fields label {
    font-size: 0.8rem;
    color: var(--muted);
    margin-top: 0.4rem;
  }

  .model-check .optional {
    color: var(--muted);
  }

  .model-check .fields input {
    min-height: 44px;
    padding: 6px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
  }

  .model-check .row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 0.6rem;
  }

  .save {
    min-height: 44px;
    padding: 6px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    cursor: pointer;
  }

  .save:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .forget {
    min-height: 44px;
    padding: 6px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }

  /* The model's answer, drawn as its own thing: a border the checklist above does not
     have, so the two are never read as one screen's verdict. */
  .model-check .answer {
    margin-top: 0.7rem;
    padding: 0.7rem 0.8rem;
    border: 1px solid var(--line);
    border-left: 3px solid var(--muted);
    border-radius: 10px;
    background: var(--panel);
  }

  .model-check .headline {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .model-check .corrected {
    margin: 0.4rem 0 0;
    font-size: 1.2rem;
    line-height: 2;
  }

  .model-check .explanation {
    margin: 0.4rem 0 0;
  }

  .model-check .attribution {
    margin: 0.5rem 0 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .hint {
    margin: 0.6rem 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .error {
    color: var(--bad);
  }
</style>
