<script lang="ts">
  /**
   * Tone Trainer — hear the minimal pairs, then say them.
   *
   * Two screens, because they are two different exercises, and mixing them is how
   * the full app's Tones page ended up needing a "Practise" button to hand the
   * learner somewhere else:
   *
   * - **Hear** is a quiz. The app says one member of a set with the system voice
   *   and the learner picks which reading it was. The readings are shown, and that
   *   is deliberate: the exercise is mapping a *sound* to a tone, so making them
   *   recall which character carries which tone would test character knowledge
   *   instead, which this app does not teach at all.
   * - **Say** is a drill. The learner picks a tone of a syllable, holds the button
   *   and says it; the pitch is measured and drawn against the shape it asks for.
   *
   * ## Why there is no practice board here
   *
   * The full app hands a tone set to its handwriting board, because it teaches
   * writing. This app does not, and that is its whole scope: a learner who wants
   * to drill tone should not have to install a character course, and one who is in
   * the middle of that course should not have to open it to hear four tones of one
   * syllable. The sets are the same derivation, read through the same
   * `hanzi_core::Dataset::tone_sets`, so the two cannot disagree about which
   * characters form a minimal pair.
   *
   * ## The voice is the system's, and its absence is said out loud
   *
   * Every hearing half needs a Chinese voice installed. With none, the listening
   * exercises are disabled **with the reason** — and saying still works, because
   * that half needs nothing but a microphone.
   */
  import { onMount } from "svelte";
  import * as api from "./lib/api";
  import { fold, searchFamilies } from "./lib/familySearch";
  import ModelPanel from "./lib/ModelPanel.svelte";
  import RecognitionPanel from "./lib/RecognitionPanel.svelte";
  import ToneChart from "./lib/ToneChart.svelte";
  import type {
    AppInfo,
    AsrStatus,
    MicrophoneStatus,
    ScoreResult,
    ToneSet,
    ToneSetMember,
    WordMember,
    WordSet,
  } from "./lib/types";

  type Screen = "hear" | "say";

  /** The tone names, in the words the scorer uses. */
  const TONE_NAME: Record<number, string> = {
    1: "high level",
    2: "rising",
    3: "dipping",
    4: "falling",
  };

  let screen = $state<Screen>("hear");
  let sets = $state<ToneSet[]>([]);
  /** Word families, loaded with the tone sets. Empty until the call answers. */
  let words = $state<WordSet[]>([]);
  /**
   * Which half of the drill the set list is showing.
   *
   * Characters are the default because that is the exercise the app is named for:
   * one tone, alone. Words are where the tones are *used*, and sandhi means they
   * are not the tones of the characters said one at a time — which is why they are
   * a separate list rather than another column.
   */
  let kind = $state<"character" | "word">("character");
  let loading = $state(true);
  let failure = $state<string | null>(null);
  /** Set when pronouncing failed, which is a different problem from loading. */
  let speechFailure = $state<string | null>(null);

  /** A name, `null` when the system has no Chinese voice, `undefined` while looking. */
  let voice = $state<string | null | undefined>(undefined);
  let microphone = $state<MicrophoneStatus | null>(null);
  let info = $state<AppInfo | null>(null);

  // ---- the optional recognition model --------------------------------------
  //
  // `null` until the first answer. The status is polled only while a download is
  // running — there are no events in this app, so progress is read rather than
  // pushed — and the timer stops itself the moment the state leaves
  // `downloading`. A learner who never installs the model never starts it.
  let asr = $state<AsrStatus | null>(null);
  let asrPolling = $state(false);
  let asrFailure = $state<string | null>(null);
  let recordLimit = $state(10);

  let query = $state("");
  /** Which set is open. Its `base` rather than the object, so a filter keeps the place. */
  let openBase = $state<string | null>(null);

  /**
   * The set the quiz is running in, what was asked and what was picked.
   *
   * `base` ties the state to its row: two sets can hold the same character, so the
   * character alone would not say which row is live.
   */
  let quiz = $state<{ base: string; answer: string; picked: string | null } | null>(null);

  /**
   * The member being drilled on the Say screen, held as `base` + character.
   *
   * Two fields rather than the object so that a filter or a reload cannot swap what
   * is being drilled out from under a result that is still on screen.
   *
   * Declared here, with the rest of the state, and not next to the Say screen's
   * markup below: `onMount` assigns the first member from the loaded sets, and a
   * `let` further down the script would be a temporal dead zone the compiler does
   * not complain about and the browser would only hit on the first load.
   */
  /**
   * What is being drilled: a character's chosen tone, or a whole word.
   *
   * One state rather than two, because they differ only in where the tones come
   * from — the learner picks one for a character, the dictionary and sandhi give
   * them for a word — and everything downstream (the microphone, the scoring call,
   * the chart) treats them identically.
   *
   * Held as identifiers rather than the objects, so that a filter or a reload
   * cannot swap what is being drilled out from under a result that is still on
   * screen.
   */
  let drill = $state<
    | { kind: "character"; base: string; ch: string }
    | { kind: "word"; key: string; text: string }
    | null
  >(null);
  /** True while the microphone is open. */
  let listening = $state(false);
  /** True between pressing the button and the device actually opening. */
  let starting = $state(false);
  let result = $state<ScoreResult | null>(null);
  let scoreFailure = $state<string | null>(null);
  /**
   * What the result on screen was judged against.
   *
   * Remembered rather than read from the drill, because the learner can switch
   * target while a result is still showing, and the recognition block must keep
   * naming what was actually scored.
   */
  let judged = $state<{ text: string; reading: string } | null>(null);

  /**
   * Which utterance is current.
   *
   * A plain variable, not state: it stops a "hear the set" sequence that is still
   * speaking from carrying on after the learner asked for something else, and
   * nothing renders it.
   */
  let sayToken = 0;

  const shown = $derived.by(() => {
    const text = query.trim();
    if (text === "") return sets;
    return sets.filter(
      (set) =>
        fold(set.base).includes(fold(text)) ||
        set.members.some((m) => m.ch === text || fold(m.reading).includes(fold(text))),
    );
  });

  /** Word families matching the query; the rule and why it is shaped that way
   *  live in `lib/familySearch.ts`, where they are tested. */
  const shownWords = $derived(searchFamilies(words, query));

  const open = $derived(sets.find((set) => set.base === openBase) ?? null);

  /**
   * Keep the drill in step with the list being browsed.
   *
   * The two are otherwise independent, and that showed: switching the list to Words
   * left the Say panel still drilling whichever *character* had been chosen, so the
   * only way to reach a word was the **Say** button on one family — and the screen
   * looked like it had no place to speak. A learner browsing words means to drill a
   * word; browsing characters means to drill a character.
   *
   * Depends on `kind` alone, so choosing a target by hand is never undone — only a
   * change of list moves the drill.
   *
   * The "previous" values start deliberately impossible (`""` is not a kind, and
   * `-1` is not a length) rather than being seeded from the current state: seeding
   * would tell the effect nothing had changed on its first run, which is why Svelte
   * warns about reading reactive state into a plain `let`.
   */
  let lastKind: "character" | "word" | "" = "";
  let lastWords = -1;
  $effect(() => {
    const switched = kind !== lastKind;
    // A list that has only just arrived also needs a first target, or the panel is
    // empty until the learner picks one.
    const arrived = words.length !== lastWords;
    lastKind = kind;
    lastWords = words.length;
    if (!switched && !arrived) return;

    const live = drill;
    if (kind === "character") {
      if (live?.kind !== "character") {
        drillSetTo(sets[0] ?? null);
      }
    } else if (live?.kind !== "word") {
      drillFamilyTo(words[0] ?? null);
    }
  });

  onMount(() => {
    void (async () => {
      try {
        const [loaded, families, status, app] = await Promise.all([
          api.toneSets(),
          api.wordSets(),
          api.microphoneStatus(),
          api.appInfo(),
        ]);
        sets = loaded;
        words = families;
        microphone = status;
        info = app;
        failure = null;
      } catch (cause) {
        failure = `Could not work out the tone pairs: ${cause}`;
      } finally {
        loading = false;
      }
      try {
        voice = await api.voice();
      } catch (cause) {
        speechFailure = `Could not ask the system for a voice: ${cause}`;
        voice = null;
      }
      try {
        recordLimit = await api.maxRecordSecs();
      } catch {
        // Cosmetic only: it appears in one sentence. A failure here must not
        // become the thing the learner sees.
      }
      // Open the first set, so the Say screen is never empty on arrival and the
      // drill is one tap away rather than two.
      openBase ??= sets[0]?.base ?? null;
      drill ??= sets[0]?.members[0]
        ? { kind: "character", base: sets[0].base, ch: sets[0].members[0].ch }
        : null;

      // The model's state, once, so the screen can describe a download nobody has
      // agreed to yet. Asking is not installing, and it opens no socket.
      await refreshAsr();
    })();
  });

  /** Read the recognition model's state. */
  async function refreshAsr() {
    try {
      asr = await api.asrStatus();
      asrFailure = null;
    } catch (cause) {
      asrFailure = `Could not ask about the recognition model: ${cause}`;
    }
  }

  /**
   * Fetch the model, then follow it until it settles.
   *
   * The install call returns as soon as the transfer has *started* — 163 MB takes
   * minutes and holding the IPC call open for it would be wrong — so progress is
   * read by polling, and the loop stops itself when the state is no longer
   * `downloading`. This is the only thing in this app that ever touches the
   * network, and only because the button was pressed.
   */
  async function installAsr() {
    asrFailure = null;
    asrPolling = true;
    try {
      await api.asrInstall();
      // Poll until it settles. A failed or finished download ends the loop; the
      // bound is a backstop against a state that never changes, so a wedged
      // thread cannot poll forever.
      for (let attempt = 0; attempt < 2400; attempt += 1) {
        await new Promise((resolve) => setTimeout(resolve, 500));
        await refreshAsr();
        if (asr?.state !== "downloading") break;
      }
    } catch (cause) {
      asrFailure = `Could not install the recognition model: ${cause}`;
    } finally {
      asrPolling = false;
    }
  }

  async function removeAsr() {
    asrFailure = null;
    try {
      asr = await api.asrRemove();
    } catch (cause) {
      asrFailure = `Could not remove the recognition model: ${cause}`;
    }
  }

  /** Say one character, stopping whatever was being said first. */
  async function say(ch: string) {
    const token = ++sayToken;
    try {
      await api.stopSpeaking();
      if (token !== sayToken) return;
      await api.speak(ch);
      speechFailure = null;
    } catch (cause) {
      if (token === sayToken) speechFailure = `Could not pronounce ${ch}: ${cause}`;
    }
  }

  /**
   * Say every member of a set, in tone order, with a gap between them.
   *
   * The gap is timed rather than measured: nothing tells the interface when the
   * synthesiser has finished, and hearing 妈麻马骂 as one run would be worse than
   * hearing them a beat apart.
   */
  async function hearSet(set: ToneSet) {
    const token = ++sayToken;
    try {
      await api.stopSpeaking();
      for (const member of set.members) {
        if (token !== sayToken) return;
        await api.speak(member.ch);
        await new Promise((resolve) => setTimeout(resolve, 900));
      }
      if (token === sayToken) speechFailure = null;
    } catch (cause) {
      if (token === sayToken) speechFailure = `Could not pronounce the set: ${cause}`;
    }
  }

  /** Ask a member at random, speaking it. Never twice in a row for one set. */
  function ask(set: ToneSet) {
    const last = quiz?.base === set.base ? quiz.answer : null;
    let index = Math.floor(Math.random() * set.members.length);
    if (set.members.length > 1 && last !== null) {
      let guard = 0;
      while (set.members[index].ch === last && guard < 8) {
        index = Math.floor(Math.random() * set.members.length);
        guard += 1;
      }
    }
    const member = set.members[index];
    quiz = { base: set.base, answer: member.ch, picked: null };
    openBase = set.base;
    void say(member.ch);
  }

  function answer(set: ToneSet, member: ToneSetMember) {
    if (quiz === null || quiz.base !== set.base || quiz.picked !== null) return;
    quiz = { ...quiz, picked: member.ch };
  }

  /** The character that was asked, for the feedback line and "hear it again". */
  const quizAnswer = $derived.by(() => {
    const live = quiz;
    if (live === null) return null;
    const set = sets.find((candidate) => candidate.base === live.base);
    return set?.members.find((member) => member.ch === live.answer) ?? null;
  });
  const quizPicked = $derived.by(() => {
    const live = quiz;
    if (live === null || live.picked === null) return null;
    const set = sets.find((candidate) => candidate.base === live.base);
    return set?.members.find((member) => member.ch === live.picked) ?? null;
  });

  /** True when the scorer cannot separate this set's two tones. */
  const flatTrap = (set: ToneSet) =>
    set.members.length === 2 && set.members[0].tone === 1 && set.members[1].tone === 3;

  /** `undefined` is "still looking"; anything but a name means no audio. */
  const noVoice = $derived(typeof voice !== "string");
  const voiceNote = $derived(
    voice === undefined
      ? "Looking for a Chinese voice…"
      : "No Chinese voice is installed, so nothing here can be heard or quizzed. Saying a tone still works — it needs only the microphone.",
  );

  const micNote = $derived.by(() => {
    const status = microphone;
    if (status === null) return null;
    return status.available
      ? null
      : `Saying a tone needs a microphone this app can open, and there is none. ${status.detail}`;
  });

  // ---------------------------------------------------------------------------
  // The Say screen
  //
  // Its state is declared with everything else, near the top: `onMount` assigns
  // the first drill member, so a declaration here would be read before it exists.
  // ---------------------------------------------------------------------------

  /**
   * What the Say panel is drilling, resolved against the loaded lists.
   *
   * One of the two is null depending on `drill.kind`. Both are derived rather than
   * stored so that a reload of the lists cannot leave a stale member behind.
   *
   * The narrowed `drill` is bound to a local first because TypeScript does not keep
   * a narrowing across a callback — `drill` is mutable state, so inside `find` it is
   * the whole union again.
   */
  const drilling = $derived(drill);
  const drillSet = $derived.by(() => {
    const live = drilling;
    return live?.kind === "character"
      ? (sets.find((set) => set.base === live.base) ?? open)
      : null;
  });
  const drillMember = $derived.by(() => {
    const live = drilling;
    if (live?.kind !== "character") return null;
    return (
      drillSet?.members.find((member) => member.ch === live.ch) ?? drillSet?.members[0] ?? null
    );
  });
  const drillFamily = $derived.by(() => {
    const live = drilling;
    return live?.kind === "word" ? (words.find((family) => family.key === live.key) ?? null) : null;
  });
  const drillWord = $derived.by(() => {
    const live = drilling;
    if (live?.kind !== "word") return null;
    return drillFamily?.words.find((word) => word.text === live.text) ?? drillFamily?.words[0] ?? null;
  });

  /** What the microphone will be judged against: a character or a word. */
  const drillText = $derived(
    drilling?.kind === "word" ? (drillWord?.text ?? null) : (drillMember?.ch ?? null),
  );
  /**
   * The reading for that text.
   *
   * A word's comes from the dictionary, which is what resolves a polyphone; a
   * character's is the reading of the tone the learner picked, which no dictionary
   * has because the tone was their choice.
   *
   * **A word's reading carries its citation tones**, not its spoken ones: the sandhi
   * is applied on the Rust side by the same `tone_target` the full app uses, so
   * there is one implementation of those rules rather than two that can disagree.
   */
  const drillReading = $derived(
    drilling?.kind === "word"
      ? (drillWord?.reading ?? null)
      : (drillMember?.reading ?? null),
  );

  /** Open a character set for drilling without speaking; the list's play button does that. */
  function drillSetTo(set: ToneSet | null) {
    openBase = set?.base ?? null;
    drill = set?.members[0]
      ? { kind: "character", base: set.base, ch: set.members[0].ch }
      : null;
    result = null;
    judged = null;
    scoreFailure = null;
  }

  /** Open a word family for drilling. */
  function drillFamilyTo(family: WordSet | null) {
    drill = family?.words[0]
      ? { kind: "word", key: family.key, text: family.words[0].text }
      : null;
    result = null;
    judged = null;
    scoreFailure = null;
  }

  /** Choose what to say: speak it as a model, and clear the last judgement. */
  function choose(set: ToneSet, member: ToneSetMember) {
    drill = { kind: "character", base: set.base, ch: member.ch };
    openBase = set.base;
    result = null;
    judged = null;
    scoreFailure = null;
    void say(member.ch);
  }

  /** The same, for a word: spoken as the whole word so the sandhi is audible. */
  function chooseWord(family: WordSet, word: WordMember) {
    drill = { kind: "word", key: family.key, text: word.text };
    result = null;
    judged = null;
    scoreFailure = null;
    void say(word.text);
  }

  async function startListening() {
    if (listening || starting) return;
    starting = true;
    scoreFailure = null;
    try {
      // Stop playback first: the voice would otherwise be recorded alongside the
      // learner's own, and a rising tone leaking from the speaker is the one thing
      // that could make a wrong answer score as right.
      await api.stopSpeaking();
      await api.listenStart();
      listening = true;
      result = null;
      judged = null;
    } catch (cause) {
      scoreFailure = `Could not open the microphone: ${cause}`;
    } finally {
      starting = false;
    }
  }

  async function stopListening() {
    const text = drillText;
    const reading = drillReading;
    if (!listening || text === null || reading === null) return;
    listening = false;
    try {
      // What was on screen while the learner spoke, sent so the judgement cannot be
      // made against something the interface invented. One call for a character and
      // for a word: the backend resolves the tones either way.
      const scored = await api.listenStop(text, reading);
      result = scored;
      judged = { text, reading };
      scoreFailure = null;
    } catch (cause) {
      scoreFailure = `Could not judge that: ${cause}`;
    }
  }

  /**
   * Push-to-talk from the keyboard.
   *
   * A button that listens only while the pointer is down is unusable for anyone
   * who cannot hold a mouse button and speak at once, and a drill is exactly where
   * someone will do it many times. Space is the same key for starting and stopping.
   */
  function onKey(event: KeyboardEvent) {
    if (screen !== "say" || event.repeat) return;
    if (event.code !== "Space" && event.key !== " ") return;
    const target = event.target as HTMLElement | null;
    // Never steal the key from a field being typed in.
    if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;
    if (drillText === null) return;
    event.preventDefault();
    if (listening) void stopListening();
    else void startListening();
  }

  /**
   * Let go of the microphone when the learner leaves the drill.
   *
   * Holding the button and clicking "Hear" leaves the pointer-up unread — the
   * button is gone by then — so without this the recorder stays open and the
   * system's recording indicator stays lit with nothing that can close it. The
   * recording is *cancelled* rather than judged: the learner left the screen, so
   * there is no drill to judge it against, and posting a result onto a screen they
   * are no longer looking at is worse than throwing the audio away.
   */
  $effect(() => {
    if (screen !== "say" && listening) {
      listening = false;
      void api.listenCancel().catch(() => {
        // Nothing to report: this is cleanup, not a drill.
      });
    }
  });

  // The same on the way out of the window entirely. `$effect` does not re-run on
  // teardown, so this needs its own listener.
  $effect(() => {
    const release = () => {
      if (listening) void api.listenCancel().catch(() => {});
    };
    window.addEventListener("pagehide", release);
    return () => window.removeEventListener("pagehide", release);
  });
</script>

<svelte:window on:keydown={onKey} />

<div class="app">
  <header class="top">
    <div class="brand">
      <h1>Tone Trainer</h1>
      <p class="tagline">Tones only: hear the pairs, then say them.</p>
    </div>
    <nav class="screens" aria-label="Which exercise">
      <button class:on={screen === "hear"} onclick={() => (screen = "hear")}>Hear</button>
      <button class:on={screen === "say"} onclick={() => (screen = "say")}>Say</button>
    </nav>
  </header>

  {#if failure}
    <p class="warning">{failure}</p>
  {/if}
  {#if speechFailure}
    <p class="warning">{speechFailure}</p>
  {/if}

  {#if loading && sets.length === 0}
    <p class="empty">Working out the tone pairs…</p>
  {:else if sets.length === 0}
    <p class="empty">
      No tone pairs could be derived from the character set. That should not be
      possible — every set comes from the dataset the app ships — so this means the
      build is broken rather than that there is nothing to practise.
    </p>
  {:else}
    <div class="toolbar">
      <form class="find" onsubmit={(event) => event.preventDefault()}>
        <input
          bind:value={query}
          placeholder={kind === "word"
            ? "Word, or a character — 你好, ni, ma"
            : "Syllable — ma, shi — or a character"}
          aria-label={kind === "word"
            ? "Find a word family by word, character or reading"
            : "Find a tone set by syllable or character"}
          autocomplete="off"
          spellcheck="false"
        />
        {#if query}
          <button type="button" onclick={() => (query = "")} title="Clear the search">
            Clear
          </button>
        {/if}
      </form>
      <!-- Which half of the drill. A segmented control, like the Hear/Say switch,
           because both lists are the same kind of thing: a set of things to say. -->
      <div class="kinds" role="group" aria-label="Characters or words">
        <button
          class:on={kind === "character"}
          onclick={() => (kind = "character")}
          title="One syllable, one tone at a time"
        >
          Characters
        </button>
        <button
          class:on={kind === "word"}
          onclick={() => (kind = "word")}
          title="Whole words, with the tones they are spoken with"
        >
          Words
        </button>
      </div>
      <span class="count">
        {#if kind === "word"}
          {shownWords.length === words.length
            ? `${words.length} families`
            : `${shownWords.length} of ${words.length} families`}
        {:else}
          {shown.length === sets.length
            ? `${sets.length} sets`
            : `${shown.length} of ${sets.length} sets`}
        {/if}
      </span>
    </div>

    {#if noVoice}
      <p class="note">{voiceNote}</p>
    {/if}

    <main class="body">
      {#if kind === "word"}
        <ul class="sets" aria-label="Word families">
          {#each shownWords as family (family.key)}
            <li class="set">
              <div class="head">
                <span class="base static" lang="zh-Hans">{family.key}</span>
                <span class="tones">{family.contrast}</span>
                <span class="actions">
                  <button
                    class="primary"
                    onclick={() => {
                      drillFamilyTo(family);
                      screen = "say";
                    }}
                    title="Drill these words: hold the button and say each one"
                  >
                    Say
                  </button>
                </span>
              </div>
              <div class="members">
                {#each family.words as word (word.text)}
                  <div class="member word">
                    <button
                      class="say"
                      onclick={() => void say(word.text)}
                      disabled={noVoice}
                      aria-label="Hear {word.reading}"
                      title={noVoice ? voiceNote : `Hear ${word.reading}`}
                    >
                      <span aria-hidden="true">🔊</span>
                    </button>
                    <span class="glyph" lang="zh-Hans">{word.text}</span>
                    <span class="reading">{word.reading}</span>
                    <span class="meaning">{word.meaning || "—"}</span>
                  </div>
                {/each}
              </div>
            </li>
          {/each}

          {#if shownWords.length === 0}
            <li class="none">
              Nothing matches. Try a word (<em>你好</em>), a character, or a reading
              without tone marks (<em>ni</em>).
            </li>
          {/if}
        </ul>
      {:else}
      <ul class="sets" aria-label="Tone sets">
        {#each shown as set (set.base)}
          {@const live = quiz?.base === set.base}
          {@const openHere = openBase === set.base}
          <li class="set" class:live class:open={openHere}>
            <div class="head">
              <button
                class="base"
                onclick={() => (openBase = openHere ? null : set.base)}
                aria-expanded={openHere}
                title={openHere ? "Close this set" : "Open this set"}
              >
                {set.base}
                <span class="chev" aria-hidden="true">{openHere ? "▾" : "▸"}</span>
              </button>
              <span class="tones">
                {set.members.length} {set.members.length === 1 ? "tone" : "tones"}
              </span>
              <span class="actions">
                <button
                  onclick={() => hearSet(set)}
                  disabled={noVoice}
                  title={noVoice ? voiceNote : "Hear every tone of this syllable, in order"}
                >
                  Hear
                </button>
                {#if live}
                  <button onclick={() => (quiz = null)} title="Stop quizzing this set">Stop</button>
                {:else}
                  <button
                    onclick={() => ask(set)}
                    disabled={noVoice}
                    title={noVoice ? voiceNote : "Hear one at random and pick which reading it was"}
                  >
                    Quiz
                  </button>
                {/if}
                <button
                  class="primary"
                  onclick={() => {
                    drillSetTo(set);
                    screen = "say";
                  }}
                  title="Drill these tones: hold the button and say each one"
                >
                  Say
                </button>
              </span>
            </div>

            {#if live && quiz && quizAnswer}
              <div class="quiz">
                <p class="prompt">
                  {#if quiz.picked === null}
                    Which reading did you hear?
                  {:else if quiz.picked === quiz.answer}
                    Yes — <strong>{quizAnswer.reading}</strong>, {quizAnswer.definition}.
                  {:else}
                    That was <strong>{quizAnswer.reading}</strong>, not {quizPicked?.reading}.
                  {/if}
                </p>
                <div class="answers">
                  {#each set.members as member (member.ch)}
                    <button
                      class="answer"
                      class:right={quiz.picked !== null && member.ch === quiz.answer}
                      class:wrong={quiz.picked === member.ch && member.ch !== quiz.answer}
                      onclick={() => answer(set, member)}
                      disabled={quiz.picked !== null}
                    >
                      <span class="glyph" lang="zh-Hans">{member.ch}</span>
                      <span class="reading">{member.reading}</span>
                    </button>
                  {/each}
                </div>
                <div class="quiz-actions">
                  <button onclick={() => void say(quizAnswer.ch)} disabled={noVoice}>
                    Hear it again
                  </button>
                  {#if quiz.picked !== null}
                    <button onclick={() => ask(set)} disabled={noVoice}>Another</button>
                  {/if}
                  <button onclick={() => (quiz = null)}>Done</button>
                </div>
              </div>
            {:else if openHere}
              <div class="members">
                {#each set.members as member (member.ch)}
                  <div class="member">
                    <button
                      class="say"
                      onclick={() => void say(member.ch)}
                      disabled={noVoice}
                      aria-label="Hear {member.reading}"
                      title={noVoice ? voiceNote : `Hear ${member.reading}`}
                    >
                      <span aria-hidden="true">🔊</span>
                    </button>
                    <span class="glyph" lang="zh-Hans">{member.ch}</span>
                    <span class="reading">{member.reading}</span>
                    <span class="meaning">{member.definition || "—"}</span>
                  </div>
                {/each}
              </div>
              {#if flatTrap(set)}
                <p class="caveat">
                  Tone 1 and a level tone 3 look the same to the scorer from one
                  syllable, so the score cannot tell these two apart. Hearing them is
                  the honest half here.
                </p>
              {/if}
            {/if}
          </li>
        {/each}

        {#if shown.length === 0}
          <li class="none">
            Nothing matches. Try a syllable without tone marks (<em>ma</em>), or clear
            the search.
          </li>
        {/if}
      </ul>
      {/if}

      <aside class="panel">
        {#if screen === "hear"}
          <div class="card">
            <h2>Hear</h2>
            <p>
              Press <strong>Quiz</strong> beside a set: one of its characters is said
              and you pick which reading it was. The readings are shown on purpose —
              the exercise is hearing the tone, not recalling which character carries
              it.
            </p>
            <p>
              <strong>Hear</strong> plays the whole set in tone order, which is the
              comparison the quiz is testing.
            </p>
          </div>
        {:else}
          <div class="card">
            <h2>Say</h2>
            {#if drillText === null}
              <p>
                Pick a {kind === "word" ? "word family" : "set"} on the left to drill it.
              </p>
            {:else if drill?.kind === "word" && drillFamily && drillWord}
              <p class="chosen-line">
                Drilling <strong>{drillWord.text}</strong> in the family of
                <strong>{drillFamily.key}</strong> — hold the button and say the whole
                word.
              </p>
              <ol class="chooser words">
                {#each drillFamily.words as word (word.text)}
                  <li>
                    <button
                      class:on={drillWord.text === word.text}
                      onclick={() => chooseWord(drillFamily, word)}
                      title="Drill {word.reading}"
                    >
                      <span class="glyph" lang="zh-Hans">{word.text}</span>
                      <span class="reading">{word.reading}</span>
                      <span class="tone-name">
                        {word.spoken.map((t) => TONE_NAME[t] ?? "?").join(" · ")}
                      </span>
                    </button>
                  </li>
                {/each}
              </ol>
              <p class="caveat">
                The tones are the ones it is <em>spoken</em> with, which for a word is
                not always what a dictionary prints: 你好 is written tone 3 + tone 3
                and said 2 + 3, and the chart below shows both when they differ.
              </p>
            {:else if drillSet && drillMember}
              <p class="chosen-line">
                Drilling <strong>{drillSet.base}</strong> — pick a tone, then hold the
                button and say it.
              </p>
              <ol class="chooser">
                {#each drillSet.members as member (member.ch)}
                  <li>
                    <button
                      class:on={drillMember.ch === member.ch}
                      onclick={() => choose(drillSet, member)}
                      title="Drill {member.reading}"
                    >
                      <span class="glyph" lang="zh-Hans">{member.ch}</span>
                      <span class="reading">{member.reading}</span>
                      <span class="tone-name">{TONE_NAME[member.tone] ?? ""}</span>
                    </button>
                  </li>
                {/each}
              </ol>
            {/if}

            {#if drillText !== null}
              <button
                class="mic"
                class:listening
                disabled={starting || (microphone !== null && !microphone.available)}
                onpointerdown={(event) => {
                  event.preventDefault();
                  void startListening();
                }}
                onpointerup={() => void stopListening()}
                onpointerleave={() => listening && void stopListening()}
                onpointercancel={() => listening && void stopListening()}
                title="Hold to record — or hold the space bar"
              >
                {#if starting}
                  Opening the microphone…
                {:else if listening}
                  ● Listening — release to judge
                {:else}
                  Hold to say {drill?.kind === "word" ? (drillWord?.text ?? "") : (drillMember?.reading ?? "")}
                {/if}
              </button>
              <p class="mic-hint">
                Say it while the button is held, or hold the space bar. The recording
                stops at {recordLimit} seconds and nothing is written to disk.
              </p>
              {#if micNote}
                <p class="caveat">{micNote}</p>
              {/if}
            {/if}
          </div>

          {#if scoreFailure}
            <p class="warning">{scoreFailure}</p>
          {/if}

          {#if result && judged}
            <!-- What was said, when a model is installed. Above the chart because
                 it is the coarser judgement: it names the sounds, and the charts
                 then show how the pitch of each was shaped. -->
            {#if result.heard}
              <RecognitionPanel
                heard={result.heard}
                targetCharacters={result.syllables.map((s) => s.ch)}
                error={result.heardError}
              />
            {:else if result.heardError}
              <p class="warning">
                The sound could not be recognised this time: {result.heardError}
                The tones below were measured from the pitch and are unaffected.
              </p>
            {/if}
            <ToneChart {result} />
          {/if}

          <!-- The optional model lives here rather than behind a settings screen:
               this app has no preferences to put beside it, and the drill is where
               a learner discovers that tone alone cannot tell 四 from 是. -->
          <ModelPanel
            status={asr}
            polling={asrPolling}
            onInstall={() => void installAsr()}
            onRemove={() => void removeAsr()}
            failure={asrFailure}
          />
        {/if}
      </aside>
    </main>
  {/if}

  <footer class="foot">
    <span>{info ? `${info.name} ${info.version} · ${info.licence}` : ""}</span>
    <span class="voice">
      {#if typeof voice === "string"}
        Voice: {voice}
      {:else if voice === undefined}
        Looking for a voice…
      {:else}
        No Chinese voice installed
      {/if}
    </span>
  </footer>
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 100%;
    padding: calc(var(--safe-top) + 14px) 16px calc(var(--safe-bottom) + 14px);
    max-width: 1100px;
    margin: 0 auto;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .brand h1 {
    margin: 0;
    font-size: 1.15rem;
    letter-spacing: -0.01em;
  }
  .tagline {
    margin: 1px 0 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  /* Which exercise. A segmented control rather than tabs, because there are two
     and neither hides the other's data — the set list stays underneath. */
  .screens {
    display: flex;
    margin-left: auto;
    border: 1px solid var(--line);
    border-radius: 9px;
    overflow: hidden;
    background: var(--surface);
  }
  .screens button {
    padding: 8px 18px;
    border: 0;
    background: transparent;
    font-size: 0.86rem;
    color: var(--muted-strong);
    cursor: pointer;
  }
  .screens button.on {
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 200px;
  }
  .find input {
    flex: 1;
    padding: 8px 11px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    font: inherit;
    font-size: 0.88rem;
    color: var(--muted-strong);
  }
  .find input:focus {
    outline: 2px solid var(--accent-soft);
    border-color: var(--accent);
  }
  .count {
    font-size: 0.78rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  /* Which half of the drill. The same segmented look as the Hear/Say switch, so
     the two read as the same kind of control. */
  .kinds {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
    background: var(--surface);
  }
  .kinds button {
    padding: 7px 12px;
    border: 0;
    border-radius: 0;
    background: transparent;
    font-size: 0.8rem;
  }
  .kinds button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  /* Two columns on anything wide enough: the sets to browse on the left, the
     exercise being run on the right. They stack in a narrow window, which is what
     a phone is. */
  .body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 14px;
    align-items: start;
    min-height: 0;
  }
  @media (max-width: 780px) {
    .body {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .sets {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 62vh;
    overflow-y: auto;
  }
  .set {
    padding: 8px 10px;
    border: 1px solid transparent;
    border-radius: 10px;
    background: var(--surface);
  }
  .set:hover {
    background: var(--hover);
  }
  .set.open {
    background: var(--surface);
  }
  .set.live {
    border-color: var(--accent);
    background: var(--surface);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .base {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    padding: 2px 4px;
    border: 0;
    background: transparent;
    font: inherit;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--accent-ink);
    cursor: pointer;
  }
  /* A word family's key is not a button: there is nothing to expand, because the
     words are always shown under it. */
  .base.static {
    cursor: default;
    font-family: var(--hanzi-font);
    font-size: 1.15rem;
  }
  .member.word .glyph {
    font-size: 1.35rem;
  }
  .chev {
    font-size: 0.7rem;
    color: var(--muted);
  }
  .tones {
    flex: 1;
    min-width: 0;
    font-size: 0.72rem;
    color: var(--muted);
  }
  .actions {
    display: flex;
    gap: 6px;
  }

  button {
    padding: 7px 13px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    font: inherit;
    font-size: 0.84rem;
    color: var(--muted-strong);
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  button.primary:hover:not(:disabled) {
    background: var(--accent-ink);
    color: #fff;
  }
  .actions button {
    padding: 4px 10px;
    font-size: 0.76rem;
  }

  .members {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  .member {
    display: grid;
    grid-template-columns: auto auto;
    grid-template-rows: auto auto;
    align-items: center;
    gap: 0 8px;
    padding: 5px 9px 5px 5px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--bg);
  }
  .say {
    grid-row: 1 / span 2;
    padding: 4px 6px;
    border: 0;
    background: transparent;
    font-size: 0.95rem;
    line-height: 1;
  }
  .say:hover:not(:disabled) {
    background: var(--accent-soft);
    border-color: transparent;
  }
  .glyph {
    font-family: var(--hanzi-font);
    font-size: 1.5rem;
    line-height: 1.15;
    color: var(--muted-strong);
  }
  .reading {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--accent-ink);
  }
  .member .meaning {
    grid-column: 2;
    font-size: 0.72rem;
    color: var(--muted);
  }

  /* The quiz replaces the member cards rather than sitting beside them: the cards
     already show the answer, and a button inside a button is neither valid nor
     clickable. */
  .quiz {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
    padding: 9px 10px;
    border: 1px dashed var(--line);
    border-radius: 9px;
  }
  .prompt {
    margin: 0;
    font-size: 0.82rem;
    color: var(--muted-strong);
  }
  .answers {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .answer {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    min-width: 66px;
    padding: 6px 10px;
  }
  .answer .glyph {
    font-size: 1.7rem;
    line-height: 1.1;
  }
  .answer.right {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .answer.wrong {
    border-color: #fca5a5;
    background: var(--danger-soft);
  }
  .answer.wrong .reading {
    color: var(--danger-ink);
  }
  .quiz-actions {
    display: flex;
    gap: 6px;
  }
  .quiz-actions button {
    padding: 4px 10px;
    font-size: 0.76rem;
  }

  /* ---- The Say panel ---------------------------------------------------- */
  .panel {
    display: flex;
    flex-direction: column;
    gap: 10px;
    position: sticky;
    top: 10px;
  }
  .card {
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
  }
  .card h2 {
    margin: 0 0 6px;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .card p {
    margin: 0 0 8px;
    font-size: 0.84rem;
    line-height: 1.5;
  }
  .card p:last-child {
    margin-bottom: 0;
  }
  .chosen-line {
    color: var(--muted-strong);
  }

  .chooser {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0 0 10px;
    padding: 0;
    list-style: none;
  }
  .chooser button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0;
    min-width: 74px;
    padding: 7px 10px;
  }
  .chooser button.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .chooser .reading {
    font-size: 0.82rem;
  }
  .chooser .tone-name {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--muted);
  }

  /* A word family is a different shape from a tone set: four tone keys at most
     against twenty-odd words, and 中's family ran the microphone button off the
     bottom of the panel — so the one control this screen exists for could not be
     found without scrolling. Capped and scrolled instead, with `scroll-margin` so
     the selected word is never flush against the edge when it is brought into view.
     Tiles are `flex: 0 1 auto` rather than growing: a wrapped flex item that grows
     stretches its whole line, which is what spread two words across the panel. */
  .chooser.words {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(78px, 1fr));
    gap: 6px;
    max-height: 30vh;
    overflow-y: auto;
    padding-right: 4px;
  }
  .chooser.words button {
    min-width: 0;
    width: 100%;
    scroll-margin: 8px;
  }
  .chooser.words .reading {
    font-size: 0.74rem;
  }
  .chooser.words .tone-name {
    font-size: 0.62rem;
  }

  /* The one red control, and a solid one: it is the thing being done, and it is
     open only while it is held. */
  .mic {
    width: 100%;
    padding: 14px;
    border-color: var(--danger);
    background: var(--danger);
    color: #fff;
    font-size: 0.95rem;
    font-weight: 600;
    touch-action: none;
  }
  .mic:hover:not(:disabled) {
    background: var(--danger-ink);
    border-color: var(--danger-ink);
    color: #fff;
  }
  .mic.listening {
    background: var(--danger-ink);
    border-color: var(--danger-ink);
    color: #fff;
    /* A pulse while it is open, so a held button is visibly live even when the
       pointer is not moving. */
    animation: pulse 1.1s ease-in-out infinite;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.72;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .mic.listening {
      animation: none;
    }
  }

  .mic-hint {
    margin: 8px 0 0;
    font-size: 0.75rem;
    line-height: 1.5;
    color: var(--muted);
  }

  .caveat,
  .none {
    margin: 8px 0 0;
    font-size: 0.76rem;
    line-height: 1.45;
    color: var(--muted);
  }
  .none {
    margin: 0;
    padding: 16px;
    border: 1px dashed var(--line);
    border-radius: 12px;
    background: var(--surface);
    font-size: 0.8rem;
  }

  .note {
    margin: 0;
    padding: 8px 11px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-size: 0.82rem;
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
  .empty {
    margin: 0;
    padding: 16px;
    border: 1px dashed var(--line);
    border-radius: 12px;
    background: var(--surface);
    font-size: 0.82rem;
    color: var(--muted);
  }

  .foot {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: auto;
    padding-top: 8px;
    font-size: 0.72rem;
    color: var(--muted);
  }
  .foot .voice {
    margin-left: auto;
  }
</style>
