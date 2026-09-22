<script lang="ts">
  /**
   * The personal vocabulary list.
   *
   * Groups are the user's own lesson names ("Lesson 3"), not the app's
   * frequency lessons, because the point is to track whatever course you are
   * actually taking. An entry's text may be a single character or a word.
   */
  import type { CharacterHint, EntryProgress, TextLookup, VocabEntry, VocabView } from "./types";
  import Icon from "./Icon.svelte";
  import * as api from "./api";
  import { tick } from "svelte";

  /**
   * What each of the schedule's four states is called on a card, and what it means.
   *
   * `Record<EntryProgress, …>` rather than a lookup with a fallback, so adding a
   * state in Rust is a compile error here rather than a blank chip. The words are
   * the panel's — they are a label for a state the backend decided, not a
   * judgement of its own — but the note is where the *rule* is stated, so it has
   * to stay in step with `KNOWN_INTERVAL_DAYS` in `hanzi-core`: that constant is
   * named in weeks here, and changing one means changing the other.
   */
  const PROGRESS: Record<EntryProgress, { label: string; note: string }> = {
    new: {
      label: "New",
      note: "Nothing in this entry has been practised yet, on any device that has synced.",
    },
    learning: {
      label: "Learning",
      note:
        "Begun, but not settled: at least one character has never been written, or is " +
        "still being reviewed only days apart.",
    },
    due: {
      label: "Due",
      note: "At least one of its characters has come up for review now.",
    },
    known: {
      label: "Known",
      note:
        "Every character has been practised, none is due, and each is scheduled at " +
        "least three weeks out.",
    },
  };

  /** `null` selects everything, `""` the unfiled entries, otherwise a group. */
  export type Selection = string | null;

  interface Props {
    view: VocabView;
    selection: Selection;
    message: string | null;
    busy: boolean;
    /** Resolve a character or word into a draft reading and meaning. */
    lookup: (text: string) => Promise<TextLookup>;
    onAdd: (text: string, pinyin: string, meaning: string, group: string | null) => void;
    onUpdate: (id: number, pinyin: string, meaning: string, group: string | null) => void;
    onRemove: (id: number) => void;
    onAddGroup: (name: string) => void;
    onRenameGroup: (from: string, to: string) => void;
    onRemoveGroup: (name: string, purge: boolean) => void;
    /**
     * Drill these entries on the board.
     *
     * `group` is the group being drilled, or `null` when the selection is not one
     * group — everything, or the unfiled remainder — and for a row's own Practise
     * button, which is a one-off and must not overwrite the place the learner had
     * reached in the group that entry belongs to.
     */
    onPractise: (entries: VocabEntry[], group: string | null) => void;
    onExport: (format: "json" | "csv") => void;
    onImport: (merge: boolean) => void;
  }

  let {
    view,
    selection,
    message,
    busy,
    lookup,
    onAdd,
    onUpdate,
    onRemove,
    onAddGroup,
    onRenameGroup,
    onRemoveGroup,
    onPractise,
    onExport,
    onImport,
  }: Props = $props();

  // ---- draft form ---------------------------------------------------------
  let text = $state("");
  let pinyin = $state("");
  let meaning = $state("");
  let group = $state("");
  /** Set while editing an existing entry rather than adding a new one. */
  let editingId = $state<number | null>(null);
  /** Once a field is typed in by hand, stop overwriting it from the dataset. */
  let pinyinEdited = $state(false);
  let meaningEdited = $state(false);
  let formError = $state<string | null>(null);
  /** Per-character readings for a multi-character draft. */
  let hints = $state<CharacterHint[]>([]);
  /** A note about the draft, e.g. characters the dataset does not know. */
  let lookupNote = $state<string | null>(null);
  /**
   * The text the current field values were derived for. A plain `let`, not
   * state: it is a guard, not something the interface renders.
   */
  let lastText = "";

  // ---- the fields themselves ----------------------------------------------
  /**
   * The draft inputs, in the order the keyboard's action key walks them.
   *
   * `group` has no entry here beyond the ref: it is the last field, so its action
   * key submits the form rather than advancing, which is what `done` promises.
   */
  let textInput = $state<HTMLInputElement | null>(null);
  let pinyinInput = $state<HTMLInputElement | null>(null);
  let meaningInput = $state<HTMLInputElement | null>(null);
  let groupInput = $state<HTMLInputElement | null>(null);

  /**
   * The tone keys, one per tone, labelled with the mark each one writes.
   *
   * The label is a sample rather than the vowel it will land on: which vowel
   * takes the mark is `pinyin.rs`'s rule, and the learner should not have to know
   * it. The name is spelled out for a tooltip and a screen reader, because "ā" on
   * its own does not say "first tone".
   */
  const TONES = [
    { tone: 1, mark: "ā", name: "First tone (high level)" },
    { tone: 2, mark: "á", name: "Second tone (rising)" },
    { tone: 3, mark: "ǎ", name: "Third tone (dipping)" },
    { tone: 4, mark: "à", name: "Fourth tone (falling)" },
    { tone: 5, mark: "a", name: "Neutral tone (no mark)" },
  ] as const;

  /**
   * Write a tone into the syllable the cursor is in.
   *
   * The caret is converted both ways between the DOM's UTF-16 offset and the
   * character offset the Rust side works in. They agree for pinyin, which is all
   * this field is for, but the conversion is what keeps them from disagreeing on
   * a field that was pasted into.
   */
  async function applyTone(tone: number) {
    const field = pinyinInput;
    if (!field) return;
    const typed = field.value.slice(0, field.selectionStart ?? field.value.length);
    const caret = [...typed].length;
    try {
      const marked = await api.markTone(field.value, caret, tone);
      pinyin = marked.text;
      // A mark written by hand is the learner's, so the dataset stops
      // overwriting it — the same rule as typing in the field.
      pinyinEdited = true;
      formError = null;
      await tick();
      const at = [...marked.text].slice(0, marked.caret).join("").length;
      // The keyboard must stay open and the cursor must land where the next tone
      // belongs, or the row is four taps and a re-aim instead of four taps.
      field.focus();
      field.setSelectionRange(at, at);
    } catch (cause) {
      formError = `Could not write the tone mark: ${cause}`;
    }
  }

  /**
   * Move to the next field when the keyboard's action key is pressed.
   *
   * The key says what it will do — `enterkeyhint` — so on every field but the
   * last it advances, and on the last the form submits, which is what `done`
   * promises. A key pressed in the middle of an IME composition belongs to the
   * keyboard, because that is what commits a candidate, so it is left alone.
   */
  function advance(event: KeyboardEvent, next: HTMLInputElement | null) {
    if (event.key !== "Enter" || event.isComposing) return;
    event.preventDefault();
    next?.focus();
  }

  // ---- group management ---------------------------------------------------
  let newGroup = $state("");
  let renamingFrom = $state<string | null>(null);
  let renameTo = $state("");

  const entriesShown = $derived.by(() => {
    if (selection === null) return view.entries;
    if (selection === "") return view.entries.filter((e) => e.group === null);
    return view.entries.filter((e) => e.group === selection);
  });

  const hasDraft = $derived(text.trim().length > 0);

  /**
   * Derive pinyin and meaning from whatever text is in the form.
   *
   * This re-runs on every change to the text, rather than only when the text is
   * a single character, because otherwise nothing ever clears what a previous
   * draft filled in: typing 学 and then 习 would leave 学's reading and meaning
   * sitting in the fields as if they described the word.
   *
   * The delay means typing a word costs one lookup instead of one per keystroke,
   * and the cancellation stops a slow reply from overwriting a newer one.
   */
  $effect(() => {
    const candidate = text.trim();
    if (candidate === lastText) return;
    lastText = candidate;
    // Anything typed by hand described the previous text, so it is dropped.
    pinyinEdited = false;
    meaningEdited = false;
    hints = [];
    lookupNote = null;

    let cancelled = false;
    const timer = setTimeout(() => {
      void (async () => {
        const found = await lookup(candidate);
        // A newer keystroke has already scheduled its own derivation.
        if (cancelled || text.trim() !== candidate) return;
        hints = found.characters;
        if (!pinyinEdited) pinyin = found.pinyin;
        if (!meaningEdited) meaning = found.meaning;
        if (candidate.length > 0 && !found.complete) {
          lookupNote =
            "Some of these characters are not in the dataset, so fill in the rest yourself.";
        }
      })();
    }, 200);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  });

  function resetDraft() {
    text = "";
    lastText = "";
    pinyin = "";
    meaning = "";
    group = selection && selection !== "" ? selection : "";
    editingId = null;
    pinyinEdited = false;
    meaningEdited = false;
    hints = [];
    lookupNote = null;
    formError = null;
  }

  function submit() {
    formError = null;
    const trimmed = text.trim();
    if (!trimmed) {
      formError = "Enter a character or word.";
      return;
    }
    if (editingId !== null) {
      onUpdate(editingId, pinyin, meaning, group.trim() || null);
    } else {
      onAdd(trimmed, pinyin, meaning, group.trim() || null);
    }
    resetDraft();
  }

  function startEdit(entry: VocabEntry) {
    editingId = entry.id;
    // Record the text first, so the derivation effect treats the stored values
    // as belonging to it and leaves them alone.
    lastText = entry.text.trim();
    text = entry.text;
    pinyin = entry.pinyin;
    meaning = entry.meaning;
    group = entry.group ?? "";
    // Existing values are the user's, so leave them alone.
    pinyinEdited = true;
    meaningEdited = true;
    hints = [];
    lookupNote = null;
    formError = null;
  }

  function practise() {
    if (entriesShown.length === 0) return;
    // A named group is the only selection that has a position to keep: "all
    // entries" and the unfiled remainder are not one list, and have nowhere to
    // remember a place.
    onPractise([...entriesShown], selection === null || selection === "" ? null : selection);
  }

  const selectionLabel = $derived(
    selection === null
      ? "All entries"
      : selection === ""
        ? "Unfiled"
        : selection,
  );
</script>

<section class="panel">
  <header class="toolbar">
    <button class="primary" onclick={practise} disabled={entriesShown.length === 0 || busy}>
      Practise {selectionLabel} ({entriesShown.length})
    </button>

    <div class="files">
      <button onclick={() => onExport("json")} disabled={busy}>Export…</button>
      <button onclick={() => onImport(true)} disabled={busy}>Import…</button>
    </div>
  </header>

  {#if message}
    <p class="message">{message}</p>
  {/if}

  {#if view.warning}
    <p class="warning">{view.warning}</p>
  {/if}

  <form
    class="draft"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <div class="row">
      <label class="field text">
        <span>Character or word</span>
        <!-- The language attributes below are the only language signal a page can
             give: they tell a screen reader which voice to use and a spell
             checker which dictionary to reach for. They do **not** switch the
             operating system's keyboard — no page can, and Android's WebView
             does not even pass them to the input method (see HANDOVER §6). The
             system keyboard's own language key is what changes the layout, so
             the fields are arranged to need it as little as possible: the
             character here, Latin pinyin beside it, English meaning after. -->
        <input
          bind:this={textInput}
          bind:value={text}
          placeholder="学习"
          lang="zh-Hans"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          enterkeyhint="next"
          onkeydown={(event) => advance(event, pinyinInput)}
        />
      </label>
      <label class="field">
        <span>Pinyin</span>
        <!-- Tagged as pinyin rather than as Chinese on purpose: this field holds
             the romanisation (`xuéxí`), so a Chinese input method would put 学习
             in it, which is not a reading. -->
        <input
          bind:this={pinyinInput}
          bind:value={pinyin}
          oninput={() => (pinyinEdited = true)}
          placeholder="xuéxí"
          lang="zh-Latn-pinyin"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          enterkeyhint="next"
          onkeydown={(event) => advance(event, meaningInput)}
        />
      </label>
      <label class="field wide">
        <span>Meaning</span>
        <input
          bind:this={meaningInput}
          bind:value={meaning}
          oninput={() => (meaningEdited = true)}
          placeholder="to study"
          lang="en"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="true"
          enterkeyhint="next"
          onkeydown={(event) => advance(event, groupInput)}
        />
      </label>
      <label class="field">
        <span>Group</span>
        <input
          bind:this={groupInput}
          bind:value={group}
          list="vocab-groups"
          placeholder="Lesson 3"
          lang="en"
          autocomplete="off"
          autocapitalize="words"
          spellcheck="false"
          enterkeyhint="done"
        />
        <datalist id="vocab-groups">
          {#each view.groups as name (name)}
            <option value={name}></option>
          {/each}
        </datalist>
      </label>
    </div>

    <!-- The tone keys. Typing `xuéxí` on a phone is two taps per accented vowel
         on a keyboard that hides them, so the app writes the mark into the
         syllable the cursor is in instead. `onmousedown` is prevented so the
         pinyin field keeps the keyboard: without it the first tap blurs the
         field and closes it, and the second tap is aimed at a form that has
         moved. The key is disabled with an empty field, because there is nothing
         to mark — the reason is the empty box itself, so no sentence is owed. -->
    <div
      class="tones"
      role="group"
      aria-label="Write a tone on the pinyin syllable at the cursor"
    >
      <span class="tones-label">Tone</span>
      {#each TONES as option (option.tone)}
        <button
          type="button"
          onpointerdown={(event) => event.preventDefault()}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => void applyTone(option.tone)}
          disabled={busy || !pinyin}
          aria-label={option.name}
          title={`${option.name} — writes the mark into the syllable at the cursor`}
        >
          {option.mark}
        </button>
      {/each}
      <span class="tones-note">Marks the syllable at the cursor.</span>
    </div>

    {#if hints.length > 1}
      <div class="hints">
        <span class="hints-label">Per character</span>
        <ul>
          {#each hints as hint, position (position)}
            <li>
              <span class="hint-ch" lang="zh-Hans">{hint.ch}</span>
              <span class="hint-pinyin">{hint.pinyin.join(" / ") || "unknown"}</span>
              <span class="hint-meaning">{hint.meaning || "—"}</span>
            </li>
          {/each}
        </ul>
        <span class="hints-note">
          The reading above is composed from these, so check it: tone changes such
          as 你好 → níhǎo are not applied. The meaning is yours to write.
        </span>
      </div>
    {/if}

    <div class="actions">
      <button class="primary" type="submit" disabled={busy || !hasDraft}>
        {editingId === null ? "Add" : "Save"}
      </button>
      {#if editingId !== null}
        <button type="button" onclick={resetDraft} disabled={busy}>Cancel</button>
      {/if}
      <span class="hint">
        A character fills both fields automatically. A word gets its reading
        composed from its characters, and you write the meaning.
      </span>
    </div>
    {#if lookupNote}
      <p class="warning">{lookupNote}</p>
    {/if}
    {#if formError}
      <p class="warning">{formError}</p>
    {/if}
  </form>

  <details class="groups">
    <summary>Groups ({view.groups.length})</summary>
    <div class="group-list">
      {#each view.groups as name (name)}
        <div class="group-row">
          {#if renamingFrom === name}
            <input bind:value={renameTo} aria-label="New group name" />
            <button
              onclick={() => {
                if (renameTo.trim()) onRenameGroup(name, renameTo.trim());
                renamingFrom = null;
              }}>Save</button
            >
            <button onclick={() => (renamingFrom = null)}>Cancel</button>
          {:else}
            <span class="name">{name}</span>
            <span class="count">{view.entries.filter((e) => e.group === name).length}</span>
            <button
              onclick={() => {
                renamingFrom = name;
                renameTo = name;
              }}>Rename</button
            >
            <button
              title="Keeps the entries and leaves them unfiled"
              onclick={() => onRemoveGroup(name, false)}>Remove label</button
            >
            <button
              class="danger"
              title="Removes the group and deletes its entries"
              onclick={() => onRemoveGroup(name, true)}>Delete entries</button
            >
          {/if}
        </div>
      {/each}

      <form
        class="group-row"
        onsubmit={(event) => {
          event.preventDefault();
          if (newGroup.trim()) onAddGroup(newGroup.trim());
          newGroup = "";
        }}
      >
        <input bind:value={newGroup} placeholder="New group, e.g. Lesson 5" />
        <button type="submit" disabled={!newGroup.trim() || busy}>Add group</button>
      </form>
    </div>
  </details>

  {#if view.entries.length === 0}
    <p class="empty">
      Nothing here yet. Add characters and words as you meet them — from the
      practice screen with the <em>+</em> beside the character's name, or with
      the form above.
    </p>
  {:else if entriesShown.length === 0}
    <p class="empty">No entries in this group.</p>
  {:else}
    <ul class="entries">
      {#each entriesShown as entry (entry.id)}
        <li>
          <span class="glyph" lang="zh-Hans">{entry.text}</span>
          <span class="reading">
            <span class="pinyin">{entry.pinyin || "—"}</span>
            <span class="meaning">{entry.meaning || "—"}</span>
            <!-- The group and what is known of the entry sit under the reading
                 rather than beside it. Beside it they were two more fixed
                 columns, and with three actions in the row the reading was
                 squeezed to nothing on a phone — the one thing the row exists to
                 show. Here they wrap among themselves and cost the reading no
                 width. Re-measured at 360 px and 320 px when the progress tag
                 arrived; see the note on it below. -->
            <span class="meta">
              {#if entry.group}
                <span class="chip">{entry.group}</span>
              {/if}
              <!-- How well it is known, from the schedule, and the only progress
                   signal on the card.

                   This *replaces* the practice record that used to sit here
                   ("4× · best 88"), for three reasons. It is the better data: the
                   record counts this device's attempts, which are not part of
                   what syncs, so a list that had just synced read "not
                   practised" for a word the other device has known for months,
                   while this is folded from the synced attempt log. Measuring the
                   card at 320 px — where the panel is 280 px — showed a third
                   item costing a line of height on every card. And two progress
                   signals on one row are two sources of truth for one question.
                   `attempts` and `bestScore` are still on the wire; nothing on
                   this screen reads them. -->
              {#if entry.standing}
                <span
                  class="progress {entry.standing.progress}"
                  title={PROGRESS[entry.standing.progress].note}
                >{PROGRESS[entry.standing.progress].label}</span>
              {/if}
            </span>
          </span>
          <!-- Three actions, as glyphs so they hold one row on a phone, where two
               word buttons already crowded the reading. The words are not lost:
               each button's accessible name says what it acts on and its tooltip
               what it does. Practise leads, because writing the entry is what the
               list is for; Remove is last and carries the danger colour, because
               it is the one that cannot be undone. -->
          <span class="row-actions">
            <button
              class="icon"
              onclick={() => onPractise([entry], null)}
              disabled={busy}
              aria-label={`Practise ${entry.text}`}
              title="Practise this entry on the board"
            >
              <Icon name="practise" />
            </button>
            <button
              class="icon"
              onclick={() => startEdit(entry)}
              disabled={busy}
              aria-label={`Edit ${entry.text}`}
              title="Edit the reading, meaning or group"
            >
              <Icon name="pencil" />
            </button>
            <button
              class="icon danger"
              onclick={() => onRemove(entry.id)}
              disabled={busy}
              aria-label={`Remove ${entry.text}`}
              title="Remove this entry"
            >
              <Icon name="trash" />
            </button>
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .files {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
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
  button.danger:hover:not(:disabled) {
    border-color: #dc2626;
    color: #b91c1c;
  }

  .message,
  .warning,
  .empty {
    margin: 0;
    font-size: 0.84rem;
    line-height: 1.5;
  }
  .message {
    color: var(--muted-strong);
  }
  .warning {
    padding: 9px 12px;
    border: 1px solid #fca5a5;
    border-radius: 9px;
    background: #fef2f2;
    color: #991b1b;
  }
  .empty {
    color: var(--muted);
    padding: 12px;
    border: 1px dashed var(--line);
    border-radius: 10px;
  }

  .draft {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
  }
  .row {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1 1 130px;
    min-width: 0;
  }
  .field.text {
    flex: 0 1 150px;
  }
  .field.wide {
    flex: 2 1 220px;
  }
  .field span {
    font-size: 0.72rem;
    color: var(--muted);
  }
  input {
    padding: 7px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg);
    font: inherit;
    font-size: 0.9rem;
    color: var(--muted-strong);
    min-width: 0;
  }
  .field.text input {
    font-family: var(--hanzi-font);
    font-size: 1.15rem;
  }
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  /* The tone keys: small, close together, and under the fields they write into. */
  .tones {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .tones-label,
  .tones-note {
    font-size: 0.72rem;
    color: var(--muted);
  }
  .tones button {
    width: 32px;
    height: 30px;
    padding: 0;
    font-size: 1rem;
    line-height: 1;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .hint {
    font-size: 0.74rem;
    color: var(--muted);
  }

  /* Per-character breakdown of a multi-character draft. */
  .hints {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--bg);
  }
  .hints-label {
    font-size: 0.72rem;
    color: var(--muted);
  }
  .hints ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
  }
  .hints li {
    display: flex;
    align-items: baseline;
    gap: 6px;
    font-size: 0.8rem;
  }
  .hint-ch {
    font-family: var(--hanzi-font);
    font-size: 1.05rem;
    color: var(--muted-strong);
  }
  .hint-pinyin {
    color: var(--accent-ink);
    font-weight: 550;
  }
  .hint-meaning {
    color: var(--muted);
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hints-note {
    font-size: 0.72rem;
    line-height: 1.45;
    color: var(--muted);
  }

  .groups summary {
    cursor: pointer;
    font-size: 0.84rem;
    color: var(--muted-strong);
  }
  .group-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 8px;
  }
  .group-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.82rem;
  }
  .group-row .name {
    min-width: 140px;
    color: var(--muted-strong);
  }
  .group-row .count {
    font-size: 0.74rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .group-row input {
    flex: 1 1 200px;
  }

  .entries {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow-y: auto;
    min-height: 0;
  }
  .entries li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface);
  }
  .glyph {
    font-family: var(--hanzi-font);
    font-size: 1.5rem;
    line-height: 1.1;
    /* Wide enough for two characters, which most entries are, and bounded so a
       six-character entry wraps here instead of taking the reading's width. */
    flex: 0 1 auto;
    min-width: 2.4em;
    max-width: 34%;
    overflow-wrap: anywhere;
    color: var(--muted-strong);
  }
  .reading {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    min-width: 0;
  }
  .pinyin {
    font-size: 0.88rem;
    font-weight: 550;
    color: var(--accent-ink);
    /* A long reading ellipsises rather than setting the column's minimum, so it
       can never be what pushes the actions off the row. */
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meaning {
    font-size: 0.8rem;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* The group and the practice record, under the reading and wrapping among
     themselves — the one place in the card that may grow downward. */
  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    min-width: 0;
    margin-top: 2px;
  }
  .chip {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-size: 0.72rem;
    /* A group name is the learner's own text and can be long; the column is
       about 100 px on a phone. It used to be `nowrap` with no bound, which put a
       long name *under* the row's three action buttons — measured at 360 px, a
       219 px chip in a 118 px column, crossing the icons at 229. So it wraps
       inside the pill instead, and never grows past the column. */
    max-width: 100%;
    overflow-wrap: anywhere;
  }
  /* How well an entry is known. A pill like the group chip, but never the same
     colour: the group is a label the learner chose and this is a state the
     schedule decided, and telling them apart at a glance is the point.
     The colour is decoration on top of the word, never the message — which is
     why every state carries a word, and why none of them is red: red is spoken
     for by the two controls that cannot be taken back (see `app.css`). */
  .progress {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 550;
    max-width: 100%;
    white-space: nowrap;
  }
  .progress.new {
    background: var(--hover);
    color: var(--muted);
  }
  .progress.learning {
    background: var(--surface);
    color: var(--muted-strong);
    border: 1px solid var(--line);
  }
  /* What to do next, so it is the one allowed to be loud. The accent is the same
     one the sidebar's due dot and the course's due marks use. */
  .progress.due {
    background: var(--accent);
    color: #fff;
  }
  /* The green the sidebar already uses for a lesson that is done. */
  .progress.known {
    background: #f0fdf4;
    color: #15803d;
  }
  .row-actions {
    flex: none;
    display: flex;
    gap: 4px;
  }
  /* Glyph buttons, so three actions fit the row where two word buttons already
     crowded it. `font-size` is the glyph size — every icon is drawn at 1em — and
     the box is a thumb-sized target so none of the three needs careful aim. */
  .row-actions button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    padding: 0;
    font-size: 1.05rem;
  }
</style>
