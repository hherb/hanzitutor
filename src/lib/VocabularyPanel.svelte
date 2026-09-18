<script lang="ts">
  /**
   * The personal vocabulary list.
   *
   * Groups are the user's own lesson names ("Lesson 3"), not the app's
   * frequency lessons, because the point is to track whatever course you are
   * actually taking. An entry's text may be a single character or a word.
   */
  import type { CharacterHint, TextLookup, VocabEntry, VocabView } from "./types";

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
    onPractise: (entries: VocabEntry[]) => void;
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
    if (entriesShown.length > 0) onPractise([...entriesShown]);
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
        <input
          bind:value={text}
          placeholder="学习"
          lang="zh-Hans"
          autocomplete="off"
          spellcheck="false"
        />
      </label>
      <label class="field">
        <span>Pinyin</span>
        <input
          bind:value={pinyin}
          oninput={() => (pinyinEdited = true)}
          placeholder="xuéxí"
          autocomplete="off"
        />
      </label>
      <label class="field wide">
        <span>Meaning</span>
        <input
          bind:value={meaning}
          oninput={() => (meaningEdited = true)}
          placeholder="to study"
          autocomplete="off"
        />
      </label>
      <label class="field">
        <span>Group</span>
        <input bind:value={group} list="vocab-groups" placeholder="Lesson 3" autocomplete="off" />
        <datalist id="vocab-groups">
          {#each view.groups as name (name)}
            <option value={name}></option>
          {/each}
        </datalist>
      </label>
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
      practice screen with <em>Add to my list</em>, or with the form above.
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
          </span>
          {#if entry.group}
            <span class="chip">{entry.group}</span>
          {/if}
          <span class="stats" title="attempts and best score">
            {entry.attempts === 0
              ? "not practised"
              : `${entry.attempts}× · best ${Math.round(entry.bestScore ?? 0)}`}
          </span>
          <span class="row-actions">
            <button onclick={() => startEdit(entry)} disabled={busy}>Edit</button>
            <button class="danger" onclick={() => onRemove(entry.id)} disabled={busy}>
              Remove
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
    gap: 12px;
    padding: 9px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface);
  }
  .glyph {
    font-family: var(--hanzi-font);
    font-size: 1.5rem;
    line-height: 1.1;
    min-width: 2.4em;
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
  }
  .meaning {
    font-size: 0.8rem;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chip {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-size: 0.72rem;
    white-space: nowrap;
  }
  .stats {
    font-size: 0.74rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .row-actions {
    display: flex;
    gap: 6px;
  }
  .row-actions button {
    padding: 4px 9px;
    font-size: 0.76rem;
  }
</style>
