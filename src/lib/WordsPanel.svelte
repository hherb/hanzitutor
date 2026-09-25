<script lang="ts">
  /**
   * The HSK word list.
   *
   * This is where words live: the course teaches single characters in frequency
   * order, and a word is several of them learned together. The search box takes
   * a character, a reading or an English meaning, and clicking any character of
   * a result turns the search into "every word containing that character",
   * which is the natural way to browse once a character is known.
   */
  import type { Word, WordSearchView } from "./types";
  import TonedPinyin from "./TonedPinyin.svelte";
  import TonedText from "./TonedText.svelte";

  interface Props {
    /**
     * The search box's contents, owned by the parent so that a search survives
     * practising a word and coming back to the list.
     */
    query: string;
    /** One page of results plus the true total; see `api.searchWords`. */
    search: (query: string, level: number | null) => Promise<WordSearchView>;
    /** The HSK level filter the sidebar has selected, or null for all. */
    level: number | null;
    message: string | null;
    onPractise: (words: Word[]) => void;
    /** Add a word to the personal list, reading and meaning filled in. */
    onAddToList: (word: Word) => void;
    busy: boolean;
  }

  let { query = $bindable(), search, level, message, onPractise, onAddToList, busy }: Props =
    $props();

  let words = $state<Word[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let failure = $state<string | null>(null);
  /**
   * The request the shown results belong to. A plain variable, not state: it is
   * a guard against a slow reply overwriting a newer one, nothing renders it.
   */
  let inFlight = 0;

  /**
   * Re-run the search when the query or the level filter changes.
   *
   * Debounced, so typing a word costs one search rather than one per keystroke,
   * and the sequence guard stops a slow reply from landing on top of a newer
   * one.
   */
  $effect(() => {
    const text = query.trim();
    const filter = level;
    const ticket = ++inFlight;

    loading = true;
    const timer = setTimeout(() => {
      void (async () => {
        try {
          const found = await search(text, filter);
          if (ticket !== inFlight) return;
          words = found.words;
          total = found.total;
          failure = null;
        } catch (cause) {
          if (ticket !== inFlight) return;
          failure = `Could not search the word list: ${cause}`;
        } finally {
          if (ticket === inFlight) loading = false;
        }
      })();
    }, 180);

    return () => clearTimeout(timer);
  });

  const levelLabel = $derived(level === null ? "all HSK levels" : `HSK ${level}`);
  const summary = $derived.by(() => {
    if (loading && words.length === 0) return "Searching…";
    if (total === 0) return `No words in ${levelLabel} match that`;
    if (total <= words.length) {
      return `${total} ${total === 1 ? "word" : "words"} in ${levelLabel}`;
    }
    return `showing ${words.length} of ${total} words in ${levelLabel}`;
  });
</script>

<section class="panel">
  <div class="toolbar">
    <form
      class="find"
      onsubmit={(event) => {
        event.preventDefault();
      }}
    >
      <input
        bind:value={query}
        placeholder="学, xuexi or “to study”"
        aria-label="Search words by character, reading or meaning"
        autocomplete="off"
        spellcheck="false"
      />
      {#if query}
        <button type="button" onclick={() => (query = "")} title="Clear the search">
          Clear
        </button>
      {/if}
    </form>
    <button
      class="primary"
      disabled={words.length === 0 || busy}
      onclick={() => onPractise(words)}
      title="Write every word shown, one character at a time"
    >
      Practise {words.length === 1 ? "this word" : `these ${words.length}`}
    </button>
  </div>

  <p class="summary">{summary}</p>

  {#if failure}
    <p class="warning">{failure}</p>
  {/if}
  {#if message}
    <p class="note">{message}</p>
  {/if}

  {#if words.length === 0 && !loading && !failure}
    <p class="empty">
      Nothing matches. Try a single character (<span lang="zh-Hans">学</span>) to
      see every word that uses it, a reading without tone marks
      (<em>xuexi</em>), or an English word.
    </p>
  {:else}
    <ul class="words">
      {#each words as word (word.text)}
        <li>
          <span class="glyph" lang="zh-Hans">
            {#each [...word.text] as ch, index (index)}
              <button
                class="piece"
                onclick={() => (query = ch)}
                title="Show every word containing {ch}"
                ><TonedText text={ch} tones={word.tones.length > 0 ? [word.tones[index] ?? null] : null} /></button
              >
            {/each}
          </span>
          <span class="reading">
            <span class="pinyin"
              >{#if word.pinyin}<TonedPinyin
                  text={word.pinyin}
                  syllables={word.syllables}
                  tones={word.tones}
                />{:else}—{/if}</span
            >
            <span class="meaning">{word.meaning || "—"}</span>
          </span>
          <span class="chip" title="HSK 3.0 level">HSK {word.hsk}</span>
          <span class="row-actions">
            <button onclick={() => onPractise([word])} disabled={busy}>Practise</button>
            <button onclick={() => onAddToList(word)} disabled={busy} title="Add to my vocabulary list">
              + List
            </button>
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  {#if total > words.length}
    <p class="footnote">
      Only the first {words.length} are listed. Narrow the search, or pick an HSK
      level on the left, to see the rest.
    </p>
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 240px;
  }
  .find input {
    flex: 1;
    padding: 8px 11px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg);
    font: inherit;
    font-size: 0.88rem;
    color: var(--muted-strong);
  }
  .find input:focus {
    outline: 2px solid var(--accent-soft);
    border-color: var(--accent);
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

  .summary {
    margin: 0;
    font-size: 0.78rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .words {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    min-height: 0;
  }
  .words li {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 7px 10px;
    border-radius: 9px;
  }
  .words li:hover {
    background: var(--hover);
  }

  /* Each character is its own button: clicking one searches for it. */
  .glyph {
    flex: none;
    display: flex;
    gap: 1px;
  }
  .piece {
    padding: 0 2px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    font-family: var(--hanzi-font);
    font-size: 1.5rem;
    line-height: 1.2;
    color: var(--muted-strong);
    cursor: pointer;
  }
  .piece:hover {
    background: var(--accent-soft);
    color: var(--accent-ink);
  }

  .reading {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .pinyin {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--accent-ink);
  }
  .meaning {
    font-size: 0.78rem;
    color: var(--muted-strong);
  }

  .chip {
    flex: none;
    padding: 2px 7px;
    border: 1px solid var(--line);
    border-radius: 20px;
    background: var(--bg);
    font-size: 0.68rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .row-actions {
    flex: none;
    display: flex;
    gap: 6px;
  }
  .row-actions button {
    padding: 5px 10px;
    font-size: 0.78rem;
  }

  .empty,
  .footnote {
    margin: 0;
    font-size: 0.8rem;
    line-height: 1.5;
    color: var(--muted);
  }
  .empty {
    padding: 16px;
    border: 1px dashed var(--line);
    border-radius: 12px;
    background: var(--surface);
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
</style>
