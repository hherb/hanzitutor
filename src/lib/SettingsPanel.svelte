<script lang="ts">
  /**
   * The settings screen.
   *
   * Four preferences, each one a thing the app would otherwise decide for the
   * learner: how a stroke is committed, how fast the stroke order is shown, how
   * big the board is, and which voice pronounces. Two rules run through the whole
   * screen and are worth stating once.
   *
   * **A change is written when it is made.** There is no Save button, because
   * there is nothing to lose by writing: a preference is one row in a database
   * that is already open. The switch moves at once and the backend is told after,
   * which is what the other stores in this app do — a preference that waits for a
   * round trip feels broken. If the write fails, `warning` comes back set and is
   * shown here rather than swallowed; the change still stands for this session,
   * so the screen says *that*, rather than claiming nothing happened.
   *
   * **An unchosen preference is a real state.** Click-to-draw and the voice can
   * both be left to the machine, and the screen offers that as a first-class
   * choice rather than hiding it: it is what a fresh install has, and what lets a
   * mouse get click-to-draw while a stylus gets dragging without anybody
   * deciding. That is also why "Automatic" is spelled out even when the
   * automatic answer could be named.
   */
  import * as api from "./api";
  import type { BoardSize, Pace, SettingsPatch, SettingsView, VoicesView } from "./types";

  interface Props {
    /** The learner's settings, as the backend has them. */
    settings: SettingsView;
    /**
     * What the device would do for the preferences with a device answer.
     *
     * Shown beside "Automatic" so the option is not a leap in the dark: a
     * learner can see that this machine currently wants click-to-draw, and can
     * tell whether they are agreeing with it or overruling it.
     */
    deviceWantsClickToDraw: boolean;
    /** The voices this machine offers, and the one in use. */
    voices: VoicesView;
    /** True while the voice list is still being read. */
    voicesLoading: boolean;
    /** Called with only what changed; the parent persists it. */
    onChange: (patch: SettingsPatch) => void;
    /** Called to forget the click-to-draw choice and follow the device again. */
    onClearClickToDraw: () => void;
  }

  let {
    settings,
    deviceWantsClickToDraw,
    voices,
    voicesLoading,
    onChange,
    onClearClickToDraw,
  }: Props = $props();

  /**
   * The voice the dropdown shows.
   *
   * The stored name, which is what a choice *is* — so a name this machine does
   * not have stays selected and the status line below explains why another voice
   * is being heard. Showing `active` instead would silently rewrite the
   * learner's choice to the fallback.
   */
  const selected = $derived(settings.voice ?? "");

  /**
   * What will actually be spoken with.
   *
   * `active` comes from the backend's resolution, so it accounts for the
   * environment override and for a stored name that is not installed. When the
   * two disagree the screen says so, because otherwise the learner would hear a
   * voice other than the one named.
   */
  const voiceFellBack = $derived(
    settings.voice !== null && voices.active !== null && voices.active !== settings.voice,
  );

  /** True while a test utterance is in flight. */
  let speaking = $state(false);
  /** Set when the test utterance could not be played. */
  let speechError = $state<string | null>(null);

  /**
   * Say one character, so a voice is chosen by ear rather than by name.
   *
   * 汉 because the app is called Hanzi Tutor and it is a character the bundled
   * course teaches early; the point is only to give the voice something Chinese
   * to read.
   */
  async function tryVoice() {
    speaking = true;
    speechError = null;
    try {
      await api.speak("汉");
    } catch (cause) {
      speechError = `Could not pronounce anything: ${cause}`;
    } finally {
      speaking = false;
    }
  }

  const clickToDrawLabel = $derived(deviceWantsClickToDraw ? "click to draw" : "drag");
</script>

<section class="panel">
  <header>
    <h2>Settings</h2>
    <p class="lede">
      Everything here is remembered between sessions. There is no Save button:
      each change is written as you make it, and a change that could not be
      written says so underneath.
    </p>
  </header>

  {#if settings.warning}
    <p class="warning">
      {settings.warning} Your change applies to this session; it will be gone
      when the app is restarted.
    </p>
  {/if}

  <div class="rows">
    <!-- Drawing a stroke ------------------------------------------------ -->
    <div class="row">
      <div class="what">
        <span class="name" id="set-input">Drawing a stroke</span>
        <span class="why">
          Click once to start a stroke and once to finish it, or hold the button
          down and drag. Both put down identical ink; only the gesture differs.
        </span>
      </div>
      <div class="how">
        <div class="segmented" role="group" aria-labelledby="set-input">
          <button
            class:on={settings.clickToDraw === null}
            onclick={onClearClickToDraw}
            title="Follow this machine: {deviceWantsClickToDraw
              ? "it has a hover, so click to draw"
              : "it has no hover, so hold and drag"}"
          >
            Automatic
          </button>
          <button
            class:on={settings.clickToDraw === true}
            onclick={() => onChange({ clickToDraw: true })}
          >
            Click to draw
          </button>
          <button
            class:on={settings.clickToDraw === false}
            onclick={() => onChange({ clickToDraw: false })}
          >
            Drag
          </button>
        </div>
        <span class="status">
          {#if settings.clickToDraw === null}
            Following this machine, which is set up for <strong>{clickToDrawLabel}</strong>.
          {:else}
            Chosen by you; this machine would have picked
            <strong>{clickToDrawLabel}</strong>.
          {/if}
          The quick switch is also on the board's control row, where it is
          reachable mid-practice.
        </span>
      </div>
    </div>

    <!-- Stroke-order speed ---------------------------------------------- -->
    <div class="row">
      <div class="what">
        <span class="name" id="set-pace">Stroke order speed</span>
        <span class="why">
          How fast the character is written when you press <em>Show stroke
          order</em>. A long stroke still takes longer than a short one, at every
          speed — the whole animation is scaled, not each stroke cut short.
        </span>
      </div>
      <div class="how">
        <div class="segmented" role="group" aria-labelledby="set-pace">
          {#each [["slow", "Slow"], ["normal", "Normal"], ["fast", "Fast"]] as const as [value, label] (value)}
            <button
              class:on={settings.animationPace === value}
              onclick={() => onChange({ animationPace: value as Pace })}
            >
              {label}
            </button>
          {/each}
        </div>
        <span class="status">
          {#if settings.animationPace === "slow"}
            Half speed, for meeting stroke order for the first time.
          {:else if settings.animationPace === "fast"}
            Twice as fast, for revising a character you already know.
          {:else}
            The pace the animation has always had.
          {/if}
        </span>
      </div>
    </div>

    <!-- Board size ------------------------------------------------------ -->
    <div class="row">
      <div class="what">
        <span class="name" id="set-board">Board size</span>
        <span class="why">
          How much of the space beside the feedback the board takes. It is always
          square, and it always fits the window whatever this is set to.
        </span>
      </div>
      <div class="how">
        <div class="segmented" role="group" aria-labelledby="set-board">
          {#each [["compact", "Compact"], ["normal", "Normal"], ["large", "Large"]] as const as [value, label] (value)}
            <button
              class:on={settings.boardSize === value}
              onclick={() => onChange({ boardSize: value as BoardSize })}
            >
              {label}
            </button>
          {/each}
        </div>
      </div>
    </div>

    <!-- Pronunciation --------------------------------------------------- -->
    <div class="row">
      <div class="what">
        <span class="name" id="set-voice">Pronunciation voice</span>
        <span class="why">
          Which installed voice reads a character or a word aloud. Only Chinese
          voices are listed: an English voice handed 汉 guesses at it. The
          automatic choice is an on-device voice, so pronunciation keeps working
          with no network; a voice marked <em>needs internet</em> is one the
          system offers that would not.
        </span>
      </div>
      <div class="how">
        {#if voicesLoading}
          <span class="status">Looking for the voices this machine has…</span>
        {:else if voices.available.length === 0}
          <span class="status">
            {#if voices.active === null}
              No Chinese voice is installed, so pronunciation is unavailable. Add
              one in System Settings → Accessibility → Spoken Content → System
              Voice → Manage Voices.
            {:else}
              This machine does not report its voice list, so the voice in use
              ({voices.active}) is the one chosen automatically.
            {/if}
          </span>
        {:else}
          <div class="voicerow">
            <select
              aria-labelledby="set-voice"
              value={selected}
              onchange={(event) => {
                const name = event.currentTarget.value;
                // The empty option is the automatic choice, and the backend
                // takes `""` for exactly that.
                onChange({ voice: name });
              }}
            >
              <option value="">Automatic</option>
              {#each voices.available as voice (voice.name)}
                <option value={voice.name}>
                  {voice.name} — {voice.locale}{voice.network ? " · needs internet" : ""}
                </option>
              {/each}
            </select>
            <button
              class="try"
              onclick={() => void tryVoice()}
              disabled={speaking || voices.active === null}
              title={voices.active === null
                ? "There is no Chinese voice to hear"
                : `Hear 汉 read in ${voices.active}`}
            >
              {speaking ? "Speaking…" : "Hear it"}
            </button>
          </div>
          <span class="status">
            {#if voiceFellBack}
              <strong>{settings.voice}</strong> is not installed on this machine,
              so <strong>{voices.active}</strong> is being used instead rather
              than leaving pronunciation silent. Turning the app off and on does
              not change that — it is what the name resolves to here.
            {:else if settings.voice === null && voices.active !== null}
              Automatic: <strong>{voices.active}</strong> is the best mainland
              Mandarin voice installed here.
            {:else if voices.active !== null}
              Reading with <strong>{voices.active}</strong>.
            {:else}
              No Chinese voice is installed, so the button above will stay
              silent.
            {/if}
          </span>
          <span class="status fine">
            The environment variable <code>HANZI_TUTOR_VOICE</code> outranks this
            setting, for a run that has to be reproducible.
          </span>
        {/if}
        {#if speechError}
          <p class="warning">{speechError}</p>
        {/if}
      </div>
    </div>
  </div>

  <p class="footnote">
    Your study data — the vocabulary list, the practice schedule and your place
    in the course — is not affected by anything on this screen. It lives in one
    database, and where that is is in <em>About and licences</em>.
  </p>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
    /* Full width, capped: the parent is a flex column, so without this the panel
       shrinks to fit its content and the preference rows collapse — the control
       column takes what it wants and the explanation beside it is squeezed into
       a one-word-wide column. */
    width: 100%;
    max-width: 900px;
    min-width: 0;
  }

  h2 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .lede {
    margin: 6px 0 0;
    font-size: 0.84rem;
    line-height: 1.5;
    color: var(--muted-strong);
  }

  .rows {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
    overflow: hidden;
  }
  .row {
    display: grid;
    /* The control column is a *fixed* 320px rather than `auto`: an `auto` track
       is sized to the column's max-content width, and the status lines beside a
       control are long sentences, so `auto` would hand the control most of the
       row and squeeze the explanation into a one-word column. */
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: 16px;
    padding: 14px;
    align-items: start;
  }
  .row + .row {
    border-top: 1px solid var(--line);
  }

  .what {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .name {
    font-size: 0.88rem;
    font-weight: 640;
    color: var(--muted-strong);
  }
  .why {
    font-size: 0.79rem;
    line-height: 1.5;
    color: var(--muted);
  }

  .how {
    display: flex;
    flex-direction: column;
    gap: 7px;
    min-width: 0;
  }

  .segmented {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
  }
  .segmented button {
    flex: 1;
    padding: 6px 10px;
    border: 0;
    border-left: 1px solid var(--line);
    background: var(--surface);
    font: inherit;
    font-size: 0.78rem;
    color: var(--muted-strong);
    white-space: nowrap;
    cursor: pointer;
  }
  .segmented button:first-child {
    border-left: 0;
  }
  .segmented button:hover {
    background: var(--hover);
  }
  .segmented button.on {
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  .voicerow {
    display: flex;
    gap: 8px;
    min-width: 0;
  }
  select {
    flex: 1;
    min-width: 0;
    padding: 6px 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    font: inherit;
    font-size: 0.8rem;
    color: var(--muted-strong);
  }
  .try {
    flex: none;
    padding: 6px 12px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    font: inherit;
    font-size: 0.78rem;
    color: var(--accent-ink);
    cursor: pointer;
  }
  .try:hover:not(:disabled) {
    background: var(--accent-soft);
  }
  .try:disabled {
    color: var(--muted);
    cursor: default;
  }

  .status {
    font-size: 0.75rem;
    line-height: 1.5;
    color: var(--muted);
  }
  .status.fine {
    font-size: 0.72rem;
  }
  .status strong {
    color: var(--muted-strong);
  }
  code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.94em;
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

  /* One column on a narrow window: the control under the explanation it
     belongs to, rather than squeezed beside it. */
  @media (max-width: 720px) {
    .row {
      grid-template-columns: minmax(0, 1fr);
      gap: 10px;
    }
  }
</style>
