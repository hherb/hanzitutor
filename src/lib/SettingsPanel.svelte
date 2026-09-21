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
  import type {
    AsrStatus,
    SayStatus,
    BoardSize,
    Pace,
    SettingsPatch,
    SettingsView,
    SyncView,
    VoicesView,
  } from "./types";
  import { onMount } from "svelte";

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
    /**
     * Called after a sync, so the screens that show study data can re-read it.
     *
     * The backend has already reloaded its stores by the time this runs; what is
     * stale by then is *this* side's copies of them.
     */
    onSynced: () => void;
    /**
     * Counts the syncs this screen did not start.
     *
     * A sync at launch or on foreground runs while this screen may be open, and it
     * changes exactly the things shown here: whether a sign-in is stored, how it is
     * protected, and what the last sync did. This screen keeps its own copy of all
     * three, and its own copy is only replaced when a command hands over a new one —
     * so without this it would show the state from before, which is the shape of bug
     * this app has already had twice.
     */
    syncPulse: number;
  }

  let {
    settings,
    deviceWantsClickToDraw,
    voices,
    voicesLoading,
    onChange,
    onClearClickToDraw,
    onSynced,
    syncPulse,
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

  // ---- Speech recognition ---------------------------------------------------
  //
  // The one control on this screen with consequences outside the machine: the
  // model is 163 MB and fetching it is the app's only network access. So the row
  // says what would be downloaded, from where, how large and under which licence
  // *before* the button is pressed, and the button is the only thing that starts
  // it. Nothing here runs on its own, and a learner who never presses it never
  // sees a prompt.

  /** The backend's answer, or `null` until the first one arrives. */
  let asr = $state<AsrStatus | null>(null);
  /** True while a press is in flight, so the button cannot be double-pressed. */
  let asrBusy = $state(false);
  /** Set when the screen could not even ask, or the press was refused. */
  let asrError = $state<string | null>(null);

  async function refreshAsr() {
    try {
      asr = await api.asrStatus();
    } catch (cause) {
      asrError = `Could not ask about the recognition model: ${cause}`;
    }
  }

  onMount(() => void refreshAsr());

  /**
   * Follow a download while one is running.
   *
   * The download happens on its own thread and reports progress through
   * `asr_status`, so this polls rather than waiting on an event — there are no
   * events in this app, and one command that answers "where has it got to" is
   * less machinery than adding them for one feature. The effect re-runs when the
   * state changes and the cleanup stops the timer, so nothing polls once the
   * download has finished or failed.
   */
  $effect(() => {
    if (asr?.state !== "downloading") return;
    const timer = setInterval(() => void refreshAsr(), 400);
    return () => clearInterval(timer);
  });

  async function installModel() {
    asrBusy = true;
    asrError = null;
    try {
      // Resolves when the download has *started*; the effect above takes over
      // from there.
      await api.asrInstall();
      await refreshAsr();
    } catch (cause) {
      asrError = `The model could not be fetched: ${cause}`;
    } finally {
      asrBusy = false;
    }
  }

  async function removeModel() {
    asrBusy = true;
    asrError = null;
    try {
      asr = await api.asrRemove();
    } catch (cause) {
      asrError = `The model could not be removed: ${cause}`;
    } finally {
      asrBusy = false;
    }
  }

  /** A byte count as whole megabytes, rounded up — never under-reported. */
  function mb(bytes: number): string {
    return `${Math.ceil(bytes / 1_000_000)} MB`;
  }

  /** How far along a download is, 0..100. */
  const asrPercent = $derived(
    asr && asr.downloadBytes > 0
      ? Math.min(100, Math.round((asr.downloaded / asr.downloadBytes) * 100))
      : 0,
  );

  // ---- Speaking phrases -----------------------------------------------------
  //
  // The second optional model, and the same rules: nothing about the app depends
  // on it, it downloads only when pressed, and every file is checked against a
  // recorded digest. It exists for the case the platform cannot cover — a
  // device with no Chinese system voice — and it is the *same* voice the bundled
  // recordings were made with, so a learner does not hear two different speakers.

  /** The backend's answer, or `null` until the first one arrives. */
  let say = $state<SayStatus | null>(null);
  let sayBusy = $state(false);
  let sayError = $state<string | null>(null);

  async function refreshSay() {
    try {
      say = await api.sayStatus();
    } catch (cause) {
      sayError = `Could not ask about the speech model: ${cause}`;
    }
  }

  onMount(() => void refreshSay());

  // Same polling shape as the recognition model: the download runs on its own
  // thread and reports through `say_status`, and the cleanup stops the timer so
  // nothing polls once it has finished or failed.
  $effect(() => {
    if (say?.state !== "downloading") return;
    const timer = setInterval(() => void refreshSay(), 400);
    return () => clearInterval(timer);
  });

  async function installSay() {
    sayBusy = true;
    sayError = null;
    try {
      await api.sayInstall();
      await refreshSay();
    } catch (cause) {
      sayError = `The speech model could not be fetched: ${cause}`;
    } finally {
      sayBusy = false;
    }
  }

  async function removeSay() {
    sayBusy = true;
    sayError = null;
    try {
      say = await api.sayRemove();
    } catch (cause) {
      sayError = `The speech model could not be removed: ${cause}`;
    } finally {
      sayBusy = false;
    }
  }

  /** How far along the synthesis download is, 0..100. */
  const sayPercent = $derived(
    say && say.bytes > 0
      ? Math.min(100, Math.round((say.downloaded / say.bytes) * 100))
      : 0,
  );

  // Cross-device sync --------------------------------------------------------
  //
  // Unlike the model above, this is not a download: it is the first thing in this
  // app that sends anything *about the learner's study data* anywhere, and it stays
  // entirely absent until they connect an account. The flow is a paste rather than
  // a redirect, because Dropbox will not send a code back to an app — so the page
  // opens in the browser, shows a code, and this screen asks for it. That is why
  // there is a text field here and why nothing waits on a callback.

  /** The backend's answer, or `null` until the first one arrives. */
  let sync = $state<SyncView | null>(null);
  /** True while a press is in flight, so a button cannot be double-pressed. */
  let syncBusy = $state(false);
  /** Set when the screen could not ask, or the press was refused. */
  let syncError = $state<string | null>(null);
  /** True between opening the authorization page and the code arriving. */
  let awaitingCode = $state(false);
  /** What the learner pasted. */
  let code = $state("");
  /** The URL, kept so the page can be reopened if the browser did not appear. */
  let authorizeUrl = $state<string | null>(null);

  async function refreshSync() {
    try {
      sync = await api.syncStatus();
    } catch (cause) {
      syncError = `Could not ask about sync: ${cause}`;
    }
  }

  onMount(() => void refreshSync());

  // Where the count stood at the last look. A plain local rather than `$state`,
  // because it is a note of what has been seen rather than something to render, and
  // `-1` because the first run of the effect is the mount, which `onMount` above has
  // already answered.
  let syncPulseSeen = -1;
  $effect(() => {
    const pulse = syncPulse;
    if (syncPulseSeen === -1 || pulse === syncPulseSeen) {
      syncPulseSeen = pulse;
      return;
    }
    syncPulseSeen = pulse;
    void refreshSync();
  });

  async function beginConnect() {
    syncBusy = true;
    syncError = null;
    try {
      authorizeUrl = await api.syncConnect();
      awaitingCode = true;
    } catch (cause) {
      syncError = `${cause}`;
    } finally {
      syncBusy = false;
    }
  }

  async function finishConnect() {
    syncBusy = true;
    syncError = null;
    try {
      sync = await api.syncConnectFinish(code.trim());
      // Cleared only on success, so a mistyped code can be corrected rather than
      // retyped — the page it came from has usually been closed by now.
      code = "";
      awaitingCode = false;
      authorizeUrl = null;
    } catch (cause) {
      syncError = `${cause}`;
    } finally {
      syncBusy = false;
    }
  }

  async function syncNow() {
    syncBusy = true;
    syncError = null;
    try {
      sync = await api.syncNow();
      // After the answer, not before: the views this reloads come from the same
      // database the sync has just finished writing.
      onSynced();
    } catch (cause) {
      syncError = `${cause}`;
    } finally {
      syncBusy = false;
    }
  }

  async function disconnect() {
    syncBusy = true;
    syncError = null;
    try {
      sync = await api.syncDisconnect();
      awaitingCode = false;
      code = "";
      authorizeUrl = null;
    } catch (cause) {
      syncError = `${cause}`;
    } finally {
      syncBusy = false;
    }
  }

  /**
   * Turn the fingerprint prompt on or off.
   *
   * Offered only where the platform can honour it — a switch that does nothing is
   * worse than no switch. Turning it off reads the stored sign-in one last time,
   * because the constraint is a property of the keychain item and rewriting it is
   * the only way to remove it; that read is the last thing in this panel that ever
   * asks for anything.
   *
   * The box is put back by hand when the backend refuses, because a checkbox has
   * already moved by the time this runs and nothing would tell it to move back —
   * the answer it is bound to has not changed. A switch left showing something the
   * app did not do is worse than an error message beside it.
   */
  async function setLock(locked: boolean, box: HTMLInputElement) {
    syncBusy = true;
    syncError = null;
    try {
      sync = await api.syncSetLock(locked);
    } catch (cause) {
      box.checked = !locked;
      syncError = `${cause}`;
    } finally {
      syncBusy = false;
    }
  }
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
          This screen is the only place it is set. The board's control row is
          icons only, and a preference that changes how the board is held is not
          one to reach for mid-stroke.
        </span>
      </div>
    </div>

    <!-- Stroke-order speed ---------------------------------------------- -->
    <div class="row">
      <div class="what">
        <span class="name" id="set-pace">Stroke order speed</span>
        <span class="why">
          How fast the character is written when you press the stroke-order
          button on the board — the pencil beside the 1, 2, 3. A long stroke
          still takes longer than a short one, at every speed — the whole
          animation is scaled, not each stroke cut short.
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

    <!-- Recognising what was said ---------------------------------------- -->
    <div class="row">
      <div class="what">
        <span class="name" id="set-asr">Recognising what was said</span>
        <span class="why">
          Tone practice judges <em>how</em> you said something, from the pitch, and
          needs no model at all — it is what this app has always done, and it keeps
          working whatever this is set to. Recognising <em>which</em> syllable you
          said is a different problem, with no model-free answer: a learner's own
          voice cannot be pre-recorded. That needs a speech model, and this app does
          not ship one. It is the only thing this app ever <em>downloads</em>, and it
          happens only if you press the button beside this.
        </span>
      </div>
      <div class="how">
        {#if asr === null}
          <span class="status">Asking whether a recognition model is installed…</span>
        {:else}
          {#if asr.state === "downloading"}
            <div
              class="progress"
              role="progressbar"
              aria-valuenow={asrPercent}
              aria-valuemin="0"
              aria-valuemax="100"
              aria-label="Downloading the speech recognition model"
            >
              <div class="bar" style="width: {asrPercent}%"></div>
            </div>
          {:else}
            <div class="voicerow">
              {#if asr.installed}
                <button class="try" onclick={() => void removeModel()} disabled={asrBusy}>
                  {asrBusy ? "Removing…" : "Remove the model"}
                </button>
              {:else}
                <button class="try" onclick={() => void installModel()} disabled={asrBusy}>
                  {asrBusy ? "Starting…" : "Download and install"}
                </button>
              {/if}
            </div>
          {/if}

          <span class="status">{asr.detail}</span>
          <span class="status fine">
            From <code>{asr.url}</code> — {mb(asr.downloadBytes)} to fetch, about
            {mb(asr.unpackedBytes)} once unpacked
            {#if asr.path}, into <code>{asr.path}</code>{/if}. The weights are under
            the <strong>{asr.licence}</strong>; they are not redistributed with this
            app, they are fetched from that address for you, and the terms are at
            <code>{asr.licenceUrl}</code>.
          </span>
        {/if}
        {#if asrError}
          <p class="warning">{asrError}</p>
        {/if}
      </div>
    </div>

    <!-- Speaking a phrase with no recording ------------------------------- -->
    <div class="row">
      <div class="what">
        <span class="name" id="set-say">Speaking phrases with no recording</span>
        <span class="why">
          Phrases that ship with a recording need nothing here. For anything else
          the app asks the operating system to speak, which is what it has always
          done. What the system cannot promise is that a Chinese voice is
          <em>installed</em>: a desktop build has no speech at all, and a device can
          have the language without a voice. This model answers that case, and it is
          the same voice the bundled recordings were made with, so you hear one
          speaker rather than two. It is optional, it is only fetched when you press
          the button, and every file is checked against a recorded digest.
        </span>
      </div>
      <div class="how">
        {#if say === null}
          <span class="status">Asking whether a speech model is installed…</span>
        {:else}
          {#if say.state === "downloading"}
            <div
              class="progress"
              role="progressbar"
              aria-valuenow={sayPercent}
              aria-valuemin="0"
              aria-valuemax="100"
              aria-label="Downloading the speech synthesis model"
            >
              <div class="bar" style="width: {sayPercent}%"></div>
            </div>
          {:else}
            <div class="voicerow">
              {#if say.installed}
                <button class="try" onclick={() => void removeSay()} disabled={sayBusy}>
                  {sayBusy ? "Removing…" : "Remove the model"}
                </button>
              {:else}
                <button class="try" onclick={() => void installSay()} disabled={sayBusy}>
                  {sayBusy ? "Starting…" : "Download and install"}
                </button>
              {/if}
            </div>
          {/if}

          <span class="status">{say.detail}</span>
          <span class="status fine">
            <strong>{say.name}</strong> — {mb(say.bytes)} to fetch, under the
            <strong>{say.licence}</strong> licence. The weights are not
            redistributed with this app; they are fetched for you from their
            publisher, and the notice is on the licences screen.
          </span>
        {/if}
        {#if sayError}
          <p class="warning">{sayError}</p>
        {/if}
      </div>
    </div>

    <!-- Syncing between devices ------------------------------------------- -->
    <div class="row">
      <div class="what">
        <span class="name" id="set-sync">Syncing between devices</span>
        <span class="why">
          Practice on the laptop and on the phone and you end up with two
          schedules, because neither device has ever seen the other's attempts.
          Connecting a Dropbox account joins them up. What travels is the
          <em>attempt log</em> — every attempt, with when it happened — and each
          device works out its own schedule from the whole of it, so there is no
          schedule to reconcile and no device that wins. The attempts are kept in
          your own Dropbox, in a folder only this app can see; there is no account
          with us and no server of ours. Nothing at all is sent until you connect an
          account here. Once you have, this app syncs on its own when it starts and
          when you come back to it, and <em>Sync now</em> is how to do it immediately
          — including after turning the fingerprint switch on, which is the one
          setting that makes syncing something you always press.
        </span>
      </div>
      <div class="how">
        {#if sync === null}
          <span class="status">Reading the sync settings…</span>
        {:else if !sync.canConnect}
          <!-- A platform with no secret store. Said plainly rather than offering a
               button that would fail: the token is never written somewhere it could
               be read, and that is a deliberate refusal, not a missing feature. -->
          <span class="status">{sync.message}</span>
        {:else if sync.connected}
          <div class="voicerow">
            <button class="try" onclick={() => void syncNow()} disabled={syncBusy}>
              {syncBusy ? "Syncing…" : "Sync now"}
            </button>
            <button
              class="try plain"
              onclick={() => void disconnect()}
              disabled={syncBusy}>Disconnect</button
            >
          </div>
          {#if sync.accountId}
            <span class="status fine">
              Connected as <code>{sync.accountId}</code>.
            </span>
          {/if}
          {#if sync.protection === "deviceOnly"}
            <span class="status fine">
              The sign-in is kept in the system keychain: encrypted at rest, readable
              only by this app, and not carried to your other devices. Nothing has to be
              unlocked to sync.
            </span>
          {:else if sync.protection === "userPresence"}
            <span class="status fine">
              The sign-in is kept behind your fingerprint, your face or your device PIN.
              It is asked for when the token is about to be used — once each time you
              open the app, not once per sync, and never merely to show this screen.
            </span>
          {:else if sync.protection === "keychainOnly"}
            <!-- Said out loud rather than left to be discovered from a prompt, and it
                 covers two things that look identical from here: a build the system
                 cannot identify — an unsigned development build on a Mac — and a
                 device that cannot ask at all, which is an Android phone with no
                 screen lock. Both mean the same thing to the learner: the sign-in is
                 encrypted on this device and released without asking anybody. The
                 last sentence is deliberately scoped to the Mac, which is the only
                 platform where this state can cost a password prompt. -->
            <span class="status fine">
              The sign-in is kept in this device's own store — the system keystore on
              Android, the keychain on a Mac or an iPhone — encrypted at rest and
              released to this app without asking anybody{#if sync.locked}, so the
              fingerprint you asked for could not be applied{/if}. On a Mac this is also
              the one case that may ask for your keychain password, because the system
              cannot identify an unsigned build.
            </span>
          {/if}
        {:else if awaitingCode}
          <span class="status">
            A Dropbox page has opened in your browser. Sign in and approve, and it
            will show you a code. Copy that code and paste it here — Dropbox will
            not send it back to the app by itself.
          </span>
          <div class="voicerow">
            <input
              class="code"
              type="text"
              bind:value={code}
              placeholder="Paste the code"
              aria-label="The code Dropbox showed you"
              spellcheck="false"
              autocapitalize="off"
              autocomplete="off"
            />
            <button
              class="try"
              onclick={() => void finishConnect()}
              disabled={syncBusy || code.trim() === ""}
            >
              {syncBusy ? "Connecting…" : "Connect"}
            </button>
          </div>
          <span class="status fine">
            {#if authorizeUrl}
              <!-- Plain text, not a link. Clicking a link here would load Dropbox
                   inside this webview, which is the one thing the whole flow avoids
                   — and opening the address again would have to start a *new*
                   authorization, which would invalidate the code on the page you
                   already have open. So it is offered to copy, not to click. -->
              If the browser did not come forward, open this address yourself:
              <code class="url">{authorizeUrl}</code>
            {/if}
          </span>
        {:else}
          <div class="voicerow">
            <button class="try" onclick={() => void beginConnect()} disabled={syncBusy}>
              {syncBusy ? "Opening…" : "Connect Dropbox"}
            </button>
          </div>
          <span class="status fine">
            You will need a Dropbox account; the free one is more than enough, as
            the attempts are a few kilobytes.
          </span>
        {/if}

        {#if sync?.canConnect && sync.canLock && !awaitingCode}
          <!-- Offered before connecting as well as after, because the answer is
               remembered and applies to whichever account is connected next. It only
               appears where the platform can honour it: a switch that does nothing is
               worse than no switch. -->
          <label class="lock">
            <input
              type="checkbox"
              checked={sync.locked}
              disabled={syncBusy}
              onchange={(event) => void setLock(event.currentTarget.checked, event.currentTarget)}
            />
            <span>
              Ask for my fingerprint before the sign-in is used
              <span class="why">
                Off by default, so that a sync which starts on its own — at launch, or
                when you come back to the app — never stops to ask you for anything. On,
                the token is released only for a fingerprint, a face or your device
                password, and syncing becomes something you press: a sync that runs by
                itself has nobody to ask.
              </span>
            </span>
          </label>
        {/if}

        {#if sync?.connected || sync?.last}
          <span class="status">{sync.message}</span>
        {/if}
        {#if sync?.last}
          <span class="status fine">
            Last time: {sync.last.published} sent, {sync.last.pulled} received,
            {sync.last.recomputed} schedule{sync.last.recomputed === 1 ? "" : "s"} updated{#if sync
              .last.vocabChanged > 0}, {sync.last.vocabChanged} list {sync.last.vocabChanged === 1
              ? "entry"
              : "entries"} updated{/if}{#if sync.last.cursorMoved}, course position
              moved{/if}{#if sync
              .last.leftAlone > 0}, {sync.last.leftAlone} left as {sync.last.leftAlone === 1
              ? "it was"
              : "they were"}{/if}.
          </span>
        {/if}
        {#if syncError}
          <p class="warning">{syncError}</p>
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
  /* The second action in a row — Disconnect beside Sync now. It should be
     available without looking like the thing to press. */
  .try.plain {
    color: var(--muted-strong);
  }
  /* The code Dropbox shows. Long and paste-only, so it gets the room a `select`
     would and the monospace its look deserves. */
  .code {
    flex: 1;
    min-width: 0;
    padding: 6px 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.78rem;
    color: var(--muted-strong);
  }
  /* An address to copy by hand. Long enough to need wrapping rather than the
     panel growing sideways. */
  .url {
    display: inline-block;
    word-break: break-all;
    font-size: 0.72rem;
    user-select: all;
  }

  /* A download of 163 MB takes minutes, so "working" and "hung" have to look
     different. The bar is the honest width of what has arrived; the percentage
     is repeated in words beside it, because a bar alone cannot be read. */
  .progress {
    height: 8px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--surface);
    overflow: hidden;
  }
  .bar {
    height: 100%;
    background: var(--accent);
    /* Short, so a poll every 400 ms reads as movement rather than as a jump. */
    transition: width 300ms linear;
  }

  /* The fingerprint switch. Laid out like a preference rather than a button,
     because it is one: it says what will happen the next time the token is used,
     and nothing happens when it is flipped except that the keychain item is
     rewritten under the new rule. */
  .lock {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    font-size: 0.78rem;
    line-height: 1.45;
    color: var(--muted-strong);
    cursor: pointer;
  }
  .lock input {
    flex: none;
    margin: 2px 0 0;
    accent-color: var(--accent);
  }
  .lock span {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .lock:has(input:disabled) {
    cursor: default;
    opacity: 0.6;
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
    /* The model's download and licence addresses are long and have no spaces in
       them. Without this a single "word" widens the control column and squeezes
       the explanation next to it. */
    overflow-wrap: anywhere;
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
