<script lang="ts">
  /**
   * The grammar stage: one pattern and everything the corpus says about it.
   *
   * ## What the card draws, and in what order
   *
   * The thing being studied comes first — the pattern, with its kana reading above
   * it when the pattern itself is written with a kanji, and its romaji under it —
   * then what it means, then the form it attaches to, then the sentences that show
   * it, then the note about nuance and the corpus's tags. That is invariant 33's
   * arrangement with no board in the middle: the shape of the thing above, the
   * explanation under it.
   *
   * ## Why it is a screen of its own, and why it has arrows
   *
   * The course screen is a list of 526 patterns and a search box; a card with three
   * sentences and a paragraph of nuance is not something to draw under a row of it.
   * The arrows step through the **course order**, which is the only sequence there
   * is here — a pattern has no band to be sorted into (`nihongo_core::grammar` has
   * the measurement that settled that), so the order the corpus teaches them in is
   * what "next" means. At the end of a page the panel fetches the next one; the
   * arrows are disabled only at the ends of the course.
   *
   * ## What is spoken
   *
   * Each **example sentence as read**, and never the pattern: a pattern like
   * `〜に対して` is not a sentence, and the placeholder `〜` is not something a
   * synthesiser should be asked to read. The readings are the corpus's own furigana
   * joined (`spokenRuby`), because the voice does not have to be trusted with a
   * written sentence it can parse its own way — `お腹痛いの？` is said `おふくつういの`
   * by Apple's Japanese front end. See `words.ts` for the measurement and
   * `HANDOVER_NIHONGO.md` invariant 37 for the rule this replaces.
   *
   * ## In the course's words, and the pattern's own characters
   *
   * The grammar course is imported text, and until now it was an island of it: a
   * learner met 526 patterns in sentences made of words the course teaches and was
   * never shown which. The section under the examples is that tie. **The words** are
   * the ones this pattern's own sentences use, from the artifact's own link — UniDic's
   * segmentation, written at build time because the corpus's furigana marks readings
   * rather than word boundaries (`nihongo_core::grammar` measured the difference) —
   * and each opens the word's own card, the same one `Words`, `Read` and `Phrases`
   * show. **The characters** are the ones the pattern itself is written with, and
   * each opens that character on the kanji board.
   *
   * Neither is computed here: the words arrive resolved and the characters arrive
   * filtered to the jōyō set, so a tap always has somewhere to go.
   */
  import Icon from "./Icon.svelte";
  import SpeakButton from "./SpeakButton.svelte";
  import WordCard from "./WordCard.svelte";
  import * as api from "./api";
  import type { GrammarLinks, GrammarPoint } from "./types";
  import type { VoiceStatus } from "./speech";
  import { charactersNote, positionLabel, wordsNote } from "./grammar";
  import { needsRuby, spokenRuby } from "./words";

  interface Props {
    /** The point this stage is about, as the course screen's row already holds it. */
    point: GrammarPoint;
    /** Where it sits in the course, counted from 1. */
    position: number;
    /** How many points the course holds, for the "12 of 526" line. */
    total: number;
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice: VoiceStatus;
    /** Back to the list. */
    onleave: () => void;
    /** Step to the previous or the next pattern in course order. */
    onstep: (direction: -1 | 1) => void;
    /** Whether each arrow can be followed — the course's ends, not the page's. */
    canPrev: boolean;
    canNext: boolean;
    /**
     * Open a character on the kanji board, asked of the app rather than done here.
     *
     * The board belongs to the other course, so this stage cannot draw it: the
     * radicals screen's "write it on the board" travels by the same road
     * (invariant 33's arrangement, one course over).
     */
    onkanji: (ch: string) => void;
  }

  let { point, position, total, voice, onleave, onstep, canPrev, canNext, onkanji }: Props =
    $props();

  /** What the pattern links to, or `null` until the answer arrives. */
  let links = $state<GrammarLinks | null>(null);
  /** The word whose card is open under the chips, by its text. */
  let opened = $state<string | null>(null);
  let linkError = $state<string | null>(null);

  /**
   * A ticket per fetch, so an answer cannot land on the pattern after it.
   *
   * The panel keys this stage on the pattern's id, so in practice one stage sees one
   * point; the ticket is here because the arrows can outrun a round trip, and a list
   * of words drawn under the wrong pattern is a wrong answer rather than a slow one.
   */
  let ticket = 0;

  $effect(() => {
    const id = point.id;
    const mine = ++ticket;
    links = null;
    opened = null;
    linkError = null;
    void (async () => {
      try {
        const answer = await api.grammarLinks(id);
        if (mine !== ticket) return;
        links = answer;
      } catch (e) {
        if (mine === ticket) linkError = String(e);
      }
    })();
  });

  /** The word whose card is open, if any — one lookup, and `null` while it closes. */
  const openedWord = $derived(
    opened === null ? null : (links?.words.find((word) => word.text === opened) ?? null),
  );
</script>

<section class="stage">
  <header class="head">
    <!-- Out of the card and back to the list, named for the list it returns to. -->
    <button class="back" onclick={onleave} title="Back to the grammar course">
      <Icon name="back" />
      <span>Grammar</span>
    </button>

    <span class="where">{positionLabel(position, total)}</span>

    <!-- The course order is the sequence, so these are the way on — there is no
         board here and nothing to grade. -->
    <span class="arrows">
      <button
        class="arrow"
        disabled={!canPrev}
        onclick={() => onstep(-1)}
        title="The previous pattern in this course"
        aria-label="Previous pattern"
      >
        <Icon name="back" />
      </button>
      <button
        class="arrow"
        disabled={!canNext}
        onclick={() => onstep(1)}
        title="The next pattern in this course"
        aria-label="Next pattern"
      >
        <Icon name="next" />
      </button>
    </span>
  </header>

  <div class="pattern" lang="ja">
    {#if point.reading}
      <span class="reading">{point.reading}</span>
    {/if}
    <span class="written">{point.pattern}</span>
    <span class="romaji">{point.romaji}</span>
  </div>

  <p class="meaning">{point.meaning}</p>

  <p class="formation"><span class="label">Written with</span> {point.formation}</p>

  <section class="examples" aria-label="Examples">
    {#each point.examples as example, i (i)}
      <article class="example">
        <p class="sentence" lang="ja">
          {#each example.ruby as part, j (j)}
            {#if needsRuby(part)}
              <ruby>{part.ruby}<rt>{part.rt}</rt></ruby>
            {:else}
              {part.ruby}
            {/if}
          {/each}
        </p>
        <p class="english" lang="en">{example.en}</p>
        <div class="say">
          <SpeakButton text={spokenRuby(example.ruby)} {voice} label="Hear it" note={i === 0} />
        </div>
      </article>
    {/each}
  </section>

  <!--
    The tie to the rest of the course, under the sentences it comes from. Hidden
    until the answer arrives rather than drawn empty: a section that appears with a
    list in it is honest about a slow round trip, and one that draws its heading
    first would flash a claim about a count it does not have.
  -->
  {#if linkError}
    <p class="error" role="alert">{linkError}</p>
  {:else if links}
    <section class="links" aria-label="In the course">
      {#if links.characters.length > 0}
        <p class="label">{charactersNote(links.characters.length)}</p>
        <div class="chips">
          {#each links.characters as ch (ch)}
            <button
              class="chip character"
              lang="ja"
              title="Open {ch} on the kanji board"
              onclick={() => onkanji(ch)}
            >
              {ch}
              <span class="hint">kanji</span>
            </button>
          {/each}
        </div>
      {/if}

      <p class="label">{wordsNote(links.words.length)}</p>
      {#if links.words.length > 0}
        <div class="chips">
          {#each links.words as word (word.text)}
            <button
              class="chip word"
              class:open={opened === word.text}
              lang="ja"
              title="Open {word.text}'s card"
              onclick={() => (opened = opened === word.text ? null : word.text)}
            >
              {word.text}
              <span class="hint">{word.reading}</span>
            </button>
          {/each}
        </div>
      {/if}

      {#if openedWord}
        <!-- The word's own card, the same one every other screen opens: its
             dictionary reading, its band, and the box that asks the learner to type
             it. -->
        <div class="card">
          <WordCard word={openedWord} {voice} />
        </div>
      {/if}
    </section>
  {/if}

  {#if point.notes}
    <p class="note">{point.notes}</p>
  {/if}

  {#if point.tags.length > 0}
    <!-- The corpus's own tags, drawn as the words they are: they are what its
         search and its own grouping use, and they are honest about how loose a
         taxonomy this is. -->
    <p class="tags">
      {#each point.tags as tag, i (tag)}
        {#if i > 0}<span class="sep">·</span>{/if}<span class="tag">{tag}</span>
      {/each}
    </p>
  {/if}
</section>

<style>
  .stage {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
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

  .where {
    margin-left: auto;
    font-size: 0.78rem;
    color: var(--muted);
  }

  .arrows {
    display: inline-flex;
    gap: 4px;
  }

  .arrow {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 44px;
    min-height: 44px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
  }

  .arrow:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .arrow:not(:disabled):hover {
    border-color: var(--accent);
  }

  .pattern {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.2rem 0;
  }

  .pattern .reading {
    font-size: 0.95rem;
    color: var(--muted);
  }

  .pattern .written {
    font-size: 2rem;
    line-height: 1.25;
  }

  .pattern .romaji {
    font-size: 0.9rem;
    color: var(--muted);
  }

  .meaning {
    margin: 0;
    font-size: 1.1rem;
  }

  .formation {
    margin: 0;
    font-size: 0.9rem;
    color: var(--muted);
  }

  .formation .label {
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.72rem;
    margin-right: 0.35rem;
  }

  .examples {
    display: grid;
    gap: 0.6rem;
    margin-top: 0.3rem;
  }

  .example {
    display: grid;
    gap: 0.35rem;
    padding: 0.6rem 0.7rem;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel);
  }

  /* The sentence is the thing being read, so it gets the size; the reading over a
     kanji is drawn by <rt> and stays small, as it is on the Phrases screen. */
  .sentence {
    margin: 0;
    font-size: 1.25rem;
    line-height: 2.2;
  }

  .sentence rt {
    font-size: 0.7rem;
    color: var(--muted);
  }

  .english {
    margin: 0;
    color: var(--muted);
    font-size: 0.9rem;
  }

  .say {
    min-height: 44px;
    display: flex;
    align-items: center;
  }

  .note {
    margin: 0;
    font-size: 0.9rem;
    color: var(--muted);
  }

  .links {
    display: grid;
    gap: 0.35rem;
    margin-top: 0.2rem;
    padding-top: 0.6rem;
    border-top: 1px solid var(--line);
  }

  .links .label {
    margin: 0;
    font-size: 0.85rem;
    color: var(--muted);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  /* A chip is a word, so it is written the way the word is: 44px tall for a thumb,
     the kana reading under the text rather than beside it, because a chip is narrow
     and 大人's reading belongs under the word the way its furigana would. */
  .chip {
    display: inline-flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0;
    min-height: 44px;
    padding: 5px 10px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: 1rem;
    cursor: pointer;
  }

  .chip:hover {
    border-color: var(--accent);
  }

  .chip.open {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  .chip .hint {
    font-size: 0.7rem;
    color: var(--muted);
  }

  .chip.character {
    justify-content: center;
    align-items: center;
    font-size: 1.1rem;
  }

  .card {
    margin-top: 0.4rem;
  }

  .error {
    margin: 0;
    color: var(--bad);
    font-size: 0.85rem;
  }

  .tags {
    margin: 0;
    font-size: 0.78rem;
    color: var(--muted);
  }

  .tags .sep {
    margin: 0 0.35rem;
  }
</style>
