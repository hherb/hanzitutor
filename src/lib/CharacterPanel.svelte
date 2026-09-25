<script lang="ts">
  /**
   * Looking a character up.
   *
   * The course teaches in frequency order, one lesson at a time, which is the
   * right way to learn and the wrong way to find something: a character met on a
   * sign, or a meaning somebody wants the character for, has no route through ten
   * characters at a time. This screen is that route. It searches the same dataset
   * the course is built from — by the character, by its reading, by its English
   * definition, or by a whole word typed as characters or as pinyin — and shows
   * for each hit what it means, how it reads, how common it is, and every word in
   * the HSK list that uses it.
   *
   * A result is not a lesson: a character outside the frequency list can be found
   * here and is honestly labelled as being in no lesson, because finding a
   * character and being able to place it in the course are different answers.
   */
  import type { CharacterSearchView, CharacterSummary, ProgressCard, Word, WordSearchView } from "./types";
  import { dueLabel } from "./due";
  import ReadingList from "./ReadingList.svelte";
  import TonedPinyin from "./TonedPinyin.svelte";
  import TonedText from "./TonedText.svelte";

  interface Props {
    /**
     * The search box's contents, owned by the parent so that a search survives
     * practising a character and coming back to the list.
     */
    query: string;
    /** One page of results plus the true total; see `api.searchCharacters`. */
    search: (query: string, level: number | null) => Promise<CharacterSearchView>;
    /** Every HSK word containing one character, most useful first. */
    wordsWith: (ch: string) => Promise<WordSearchView>;
    /** The HSK level filter the sidebar has selected, or null for all. */
    level: number | null;
    message: string | null;
    busy: boolean;
    /** Progress by character, for the practised and due marks. */
    cards: ProgressCard[];
    onPractise: (character: CharacterSummary) => void;
    /** Drill dictionary words straight from a character's page. */
    onPractiseWords: (words: Word[]) => void;
    /** Add a character to the personal list, reading and meaning filled in. */
    onAddToList: (character: CharacterSummary) => void;
    /** Put the character on the board in the course it belongs to. */
    onShowInCourse: (ch: string) => void;
    /**
     * Open the radical's family on the radicals screen: every character that
     * shares it, and what the radical means. The radical shown here is the
     * Kangxi head form, and the shapes inside the characters are its combining
     * forms — which is exactly what that screen draws out.
     */
    onShowRadical: (radical: string) => void;
    /**
     * Write one component of the shown character on the board.
     *
     * A component that is itself a character — 兑 in 说 — is worth writing on its
     * own, which is how a character stops being a picture and becomes parts. The
     * panel only offers this for parts the dataset can draw, which the backend
     * has already marked.
     */
    onPractisePart: (ch: string) => void;
  }

  let {
    query = $bindable(),
    search,
    wordsWith,
    level,
    message,
    busy,
    cards,
    onPractise,
    onPractiseWords,
    onAddToList,
    onShowInCourse,
    onShowRadical,
    onPractisePart,
  }: Props = $props();

  let characters = $state<CharacterSummary[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let failure = $state<string | null>(null);
  /** The character whose page is open. `null` shows the first result. */
  let selected = $state<string | null>(null);
  /**
   * The request the shown results belong to. A plain variable, not state: it is
   * a guard against a slow reply overwriting a newer one, nothing renders it.
   */
  let inFlight = 0;

  let words = $state<Word[]>([]);
  let wordsTotal = $state(0);
  let wordsFailure = $state<string | null>(null);
  /** Which character the shown words belong to, so a re-search with the same
   *  first hit does not refetch them. */
  let wordsFor: string | null = null;
  let wordsInFlight = 0;

  /**
   * Re-run the search when the query or the level filter changes.
   *
   * Debounced, so typing a reading costs one search rather than one per
   * keystroke, and the sequence guard stops a slow reply from landing on top of
   * a newer one.
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
          characters = found.characters;
          total = found.total;
          failure = null;
        } catch (cause) {
          if (ticket !== inFlight) return;
          failure = `Could not search the characters: ${cause}`;
        } finally {
          if (ticket === inFlight) loading = false;
        }
      })();
    }, 180);

    return () => clearTimeout(timer);
  });

  /**
   * The character whose page is open: the selected one, or the best match when
   * nothing has been clicked.
   *
   * Resolving it against the results rather than holding the summary itself means
   * a new search cannot leave a stale card on screen — the character shown is
   * always one of the characters listed.
   */
  const shown = $derived(
    characters.find((character) => character.ch === selected) ?? characters[0] ?? null,
  );

  /** Load the words containing the open character. */
  $effect(() => {
    const ch = shown?.ch ?? null;
    if (ch === wordsFor) return;
    wordsFor = ch;
    const ticket = ++wordsInFlight;

    words = [];
    wordsTotal = 0;
    wordsFailure = null;
    if (ch === null) return;

    void (async () => {
      try {
        const found = await wordsWith(ch);
        if (ticket !== wordsInFlight) return;
        words = found.words;
        wordsTotal = found.total;
      } catch (cause) {
        if (ticket !== wordsInFlight) return;
        wordsFailure = `Could not list the words using ${ch}: ${cause}`;
      }
    })();
  });

  /** Progress by character, for the marks and the page's own line. */
  const cardFor = $derived(new Map(cards.map((card) => [card.ch, card])));

  /**
   * How the level filter is said in the summary line.
   *
   * A phrase that reads with the sentence rather than a bare label: `in HSK 2`
   * and `in all HSK levels` take the preposition, while level 0 is already a
   * place ("outside the HSK lists") and reads "1 character outside the HSK
   * lists", never "in outside the HSK lists".
   */
  const scope = $derived(
    level === null
      ? "in all HSK levels"
      : level === 0
        ? "outside the HSK lists"
        : `in HSK ${level}`,
  );
  const summary = $derived.by(() => {
    if (loading && characters.length === 0) return "Searching…";
    if (total === 0) return `No characters ${scope} match that`;
    if (total <= characters.length) {
      return `${total} ${total === 1 ? "character" : "characters"} ${scope}`;
    }
    return `showing ${characters.length} of ${total} characters ${scope}`;
  });

  const strokeLabel = (character: CharacterSummary) =>
    `${character.strokeCount} ${character.strokeCount === 1 ? "stroke" : "strokes"}`;
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
        placeholder="学, xue, “to study” or 医院"
        aria-label="Search characters by character, reading or meaning"
        autocomplete="off"
        spellcheck="false"
      />
      {#if query}
        <button type="button" onclick={() => (query = "")} title="Clear the search">
          Clear
        </button>
      {/if}
    </form>
  </div>

  <p class="summary">{summary}</p>

  {#if failure}
    <p class="warning">{failure}</p>
  {/if}
  {#if message}
    <p class="note">{message}</p>
  {/if}

  {#if characters.length === 0 && !loading && !failure}
    <p class="empty">
      Nothing matches. Try one character (<span lang="zh-Hans">学</span>), a reading
      without tone marks (<em>xue</em>), an English word, or a word you have met —
      <span lang="zh-Hans">医院</span> or <em>yisheng</em> — to see the characters it
      is made of.
    </p>
  {:else}
    <div class="split">
      <ul class="characters">
        {#each characters as character (character.ch)}
          {@const card = cardFor.get(character.ch)}
          <li class:selected={character.ch === shown?.ch}>
            <button
              class="row"
              onclick={() => (selected = character.ch)}
              aria-current={character.ch === shown?.ch ? "true" : undefined}
            >
              <span class="glyph" lang="zh-Hans"><TonedText text={character.ch} /></span>
              <span class="reading">
                <span class="pinyin"><ReadingList readings={character.pinyin} /></span>
                <span class="meaning">{character.definition || "—"}</span>
              </span>
              <span class="marks">
                {#if character.hsk > 0}
                  <span class="chip" title="HSK 3.0 level">HSK {character.hsk}</span>
                {/if}
                {#if !character.inCourse}
                  <span class="chip outside" title="Not in the course's frequency list"
                    >no lesson</span
                  >
                {/if}
                {#if card?.dueNow}
                  <span class="dot" title="Due for review"></span>
                {/if}
              </span>
            </button>
            <span class="row-actions">
              <button
                onclick={() => onPractise(character)}
                disabled={busy}
                title="Write this character on the board"
                >Practise</button
              >
              <button
                onclick={() => onAddToList(character)}
                disabled={busy}
                title="Add this character to my vocabulary list">+ List</button
              >
            </span>
          </li>
        {/each}
      </ul>

      {#if shown}
        {@const card = cardFor.get(shown.ch)}
        <aside class="detail" aria-label="Character details">
          <div class="head">
            <span class="big" lang="zh-Hans"><TonedText text={shown.ch} /></span>
            <span class="head-text">
              <span class="detail-pinyin"><ReadingList readings={shown.pinyin} /></span>
              <span class="detail-meaning">{shown.definition || "—"}</span>
            </span>
          </div>

          <ul class="facts">
            <li>{strokeLabel(shown)}</li>
            {#if shown.radical && shown.radical !== "\u0000"}
              <li>
                radical <span lang="zh-Hans"><TonedText text={shown.radical} /></span>{#if shown.radicalMeaning}
                  — {shown.radicalMeaning}{/if}
              </li>
            {/if}
            {#if shown.hsk > 0}<li>HSK {shown.hsk}</li>{/if}
            {#if shown.rank > 0}<li>frequency #{shown.rank}</li>{/if}
            {#if card}
              <li>practised {card.attempts}× · best {Math.round(card.bestScore ?? 0)}</li>
              <li class:due={card.dueNow}>
                {card.dueNow ? "due for review" : `next review ${dueLabel(card.due)}`}
              </li>
            {:else}
              <li>not practised yet</li>
            {/if}
          </ul>

          {#if shown.etymology}
            <p class="etymology">{shown.etymology}</p>
          {/if}

          {#if shown.components.parts.length > 0}
            <!-- What the character is built from. The parts are drawn rather
                 than described, and each one the board can write is a button:
                 writing 兑 on its own is how 说 stops being a picture. The
                 arrangement comes from the outermost IDS operator, and the raw
                 string is the source, so it is in the title rather than lost. -->
            <div
              class="components"
              title="Make Me a Hanzi decomposition: {shown.components.raw}"
            >
              <p class="components-title">
                Built from
                {#if shown.components.layout}
                  <span class="components-layout">({shown.components.layout})</span>
                {/if}
              </p>
              <ul class="parts">
                {#each shown.components.parts as part, index (index)}
                  <li>
                    {#if part.ch && part.drawable}
                      <button
                        class="part"
                        lang="zh-Hans"
                        onclick={() => onPractisePart(part.ch!)}
                        disabled={busy}
                        title="Write {part.ch} on the board"
                      >
                        <TonedText text={part.ch} />
                      </button>
                    {:else if part.ch}
                      <span
                        class="part fixed"
                        lang="zh-Hans"
                        title="The board has no strokes for this part, so it cannot be written on its own"
                      >
                        <TonedText text={part.ch} />
                      </span>
                    {:else}
                      <span class="part unknown" title="Make Me a Hanzi could not name this part"
                        >?</span
                      >
                    {/if}
                  </li>
                {/each}
              </ul>
            </div>
          {/if}

          <div class="actions">
            <button class="primary" onclick={() => onPractise(shown)} disabled={busy}>
              Practise
            </button>
            <button onclick={() => onAddToList(shown)} disabled={busy}>+ List</button>
            {#if shown.radical && shown.radical !== "\u0000"}
              <button
                onclick={() => onShowRadical(shown.radical)}
                disabled={busy}
                title="Every character that shares this radical, and what it means"
              >
                Radical family
              </button>
            {/if}
            {#if shown.inCourse}
              <button
                onclick={() => onShowInCourse(shown.ch)}
                disabled={busy}
                title="Show it on the board in the course"
              >
                In the course
              </button>
            {/if}
          </div>

          {#if !shown.inCourse}
            <p class="aside-note">
              This character is not in the course's frequency list, so no lesson
              holds it. It can still be written here, and saved to your list.
            </p>
          {/if}

          <div class="words-block">
            <p class="words-title">
              {#if words.length === 0}
                Words using <span lang="zh-Hans"><TonedText text={shown.ch} /></span>
              {:else}
                {wordsTotal} {wordsTotal === 1 ? "word" : "words"} using
                <span lang="zh-Hans"><TonedText text={shown.ch} /></span>
              {/if}
            </p>
            {#if wordsFailure}
              <p class="warning">{wordsFailure}</p>
            {:else if words.length > 0}
              <ul class="words">
                {#each words as word (word.text)}
                  <li>
                    <span class="word-glyph" lang="zh-Hans"
                      ><TonedText text={word.text} tones={word.tones.length > 0 ? word.tones : null} /></span
                    >
                    <span class="word-text">
                      <span class="word-pinyin"
                        >{#if word.pinyin}<TonedPinyin
                            text={word.pinyin}
                            syllables={word.syllables}
                            tones={word.tones}
                          />{:else}—{/if}</span
                      >
                      <span class="word-meaning">{word.meaning || "—"}</span>
                    </span>
                    <button
                      onclick={() => onPractiseWords([word])}
                      disabled={busy}
                      title="Write this word, one character at a time">Practise</button
                    >
                  </li>
                {/each}
              </ul>
              {#if wordsTotal > words.length}
                <p class="aside-note">
                  Showing the {words.length} most useful. The HSK words screen lists
                  them all.
                </p>
              {/if}
            {:else}
              <p class="aside-note">No word in the HSK list uses it.</p>
            {/if}
          </div>
        </aside>
      {/if}
    </div>
  {/if}

  {#if total > characters.length}
    <p class="footnote">
      Only the first {characters.length} are listed. Narrow the search, or pick an
      HSK level on the left, to see the rest.
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

  /* The list and the open character's page side by side, and stacked when there
     is no room for two columns. */
  .split {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(250px, 0.8fr);
    gap: 16px;
    align-items: start;
    min-height: 0;
  }
  @media (max-width: 860px) {
    .split {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .characters {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    min-height: 0;
  }
  .characters li {
    display: flex;
    align-items: center;
    gap: 8px;
    border-radius: 9px;
  }
  .characters li:hover {
    background: var(--hover);
  }
  .characters li.selected {
    background: var(--accent-soft);
  }

  /* The row is one button, so the whole row opens the character's page. The
     actions beside it are siblings, not children: a button inside a button is
     neither valid nor clickable. */
  .row {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 7px 10px;
    border: 0;
    background: transparent;
    text-align: left;
  }
  .row:hover:not(:disabled) {
    border-color: transparent;
    color: inherit;
  }

  .glyph {
    flex: none;
    width: 2.2rem;
    font-family: var(--hanzi-font);
    font-size: 1.6rem;
    line-height: 1.2;
    color: var(--muted-strong);
    text-align: center;
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

  .marks {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .chip {
    padding: 2px 7px;
    border: 1px solid var(--line);
    border-radius: 20px;
    background: var(--bg);
    font-size: 0.68rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .chip.outside {
    border-style: dashed;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
  }

  .row-actions {
    flex: none;
    display: flex;
    gap: 6px;
    padding-right: 8px;
  }
  .row-actions button {
    padding: 5px 10px;
    font-size: 0.78rem;
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 13px 14px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .big {
    font-family: var(--hanzi-font);
    font-size: 3rem;
    line-height: 1;
    color: var(--muted-strong);
  }
  .head-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .detail-pinyin {
    font-size: 1rem;
    font-weight: 600;
    color: var(--accent-ink);
  }
  .detail-meaning {
    font-size: 0.82rem;
    color: var(--muted-strong);
  }

  .facts {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    font-size: 0.75rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .facts li.due {
    color: var(--accent-ink);
    font-weight: 600;
  }

  .etymology {
    margin: 0;
    padding: 8px 10px;
    border-left: 2px solid var(--accent-soft);
    font-size: 0.78rem;
    line-height: 1.5;
    color: var(--muted-strong);
  }

  /* What the character is built from: the parts, in reading order, each one
     writable when the board has strokes for it. */
  .components {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .components-title {
    margin: 0;
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--muted-strong);
  }
  .components-layout {
    font-weight: 400;
    color: var(--muted);
  }
  .parts {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
  }
  .parts li {
    display: flex;
  }
  .part {
    width: 2.3rem;
    height: 2.3rem;
    padding: 0;
    font-family: var(--hanzi-font);
    font-size: 1.25rem;
    line-height: 1;
    text-align: center;
  }
  .part.fixed,
  .part.unknown {
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--line);
    border-radius: 8px;
    color: var(--muted);
    cursor: default;
  }
  .part.unknown {
    font-family: inherit;
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .actions button {
    padding: 5px 10px;
    font-size: 0.78rem;
  }

  .words-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
    border-top: 1px solid var(--line);
    padding-top: 10px;
  }
  .words-title {
    margin: 0;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .words {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 15rem;
    overflow-y: auto;
  }
  .words li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
  }
  .word-glyph {
    flex: none;
    font-family: var(--hanzi-font);
    font-size: 1.05rem;
    color: var(--muted-strong);
  }
  .word-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .word-pinyin {
    font-size: 0.76rem;
    font-weight: 600;
    color: var(--accent-ink);
  }
  .word-meaning {
    font-size: 0.72rem;
    color: var(--muted-strong);
  }
  .words button {
    padding: 3px 8px;
    font-size: 0.72rem;
  }

  .aside-note,
  .footnote {
    margin: 0;
    font-size: 0.75rem;
    line-height: 1.5;
    color: var(--muted);
  }

  .empty {
    margin: 0;
    padding: 16px;
    border: 1px dashed var(--line);
    border-radius: 12px;
    background: var(--surface);
    font-size: 0.8rem;
    line-height: 1.5;
    color: var(--muted);
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
