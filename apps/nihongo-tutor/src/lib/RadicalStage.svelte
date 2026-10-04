<script lang="ts">
  /**
   * The radical stage: one of the 214 head forms, the family it heads, and the way
   * to write it — and nothing else.
   *
   * ## Why it is a screen of its own
   *
   * `RadicalsPanel` is the course: the search box, the order, and all 214. This is
   * what a row opens. The panel used to draw the family in a second column beside
   * the list, which is N12's complaint in the shape that screen could have it — on
   * a phone the thing a learner tapped for was *below* the list they had to scroll
   * back through. `HANDOVER_NIHONGO.md` invariants 33 and 34 are the rule; this
   * pair is those two screens without a board, because the writing happens on the
   * kanji board next door.
   *
   * ## The order of what is here
   *
   * The head form and what it is, then the one thing to do with it, then the
   * family. The board is not here and is not drawn: a head form and a member both
   * go to the **kanji** screen's board, which is one grading path for three kinds
   * of thing (`KanjiPanel.svelte`, invariant 13).
   *
   * ## No arrows, and that is a decision
   *
   * The kanji and the kana stages step through a *lesson* with the arrows either
   * side of the board: ten characters, or five kana, in the order the course teaches
   * them. A radical is not looked up in a lesson — it is looked up by number, by
   * head form, or by a character that uses it, and the list it came from can be
   * filtered and ordered two ways. An arrow that meant "the next one in whatever
   * list the learner last built" would be a second, invisible ordering rule; the
   * way on is the list itself, one tap behind `Radicals`.
   */
  import Icon from "./Icon.svelte";
  import type { RadicalFamilyView } from "./types";

  interface Props {
    /**
     * The family this stage is about.
     *
     * Passed whole rather than fetched by number: the course screen already holds
     * all 214 with their members — the list draws the counts — so a command for one
     * of them would be a second answer to a question the panel has answered.
     */
    radical: RadicalFamilyView;
    /** Write this jōyō character on the kanji board. */
    onopen?: (ch: string) => void;
    /** Write this radical's head form on the kanji board. */
    onpractise?: (number: number) => void;
    /** Back to the 214. */
    onleave: () => void;
  }

  let { radical, onopen, onpractise, onleave }: Props = $props();
</script>

<section class="stage">
  <header class="head">
    <!--
      Out of the stage and back to the list. The word says where it goes rather
      than leaving an arrow to be guessed at — the kanji and kana stages' own
      arrangement, and the reason it is in the header row rather than below.
    -->
    <button class="back" onclick={onleave} title="Back to the 214 radicals">
      <Icon name="back" />
      <span>Radicals</span>
    </button>
    <span class="where">Radical {radical.number} of 214</span>
  </header>

  <div class="meta">
    <div class="who">
      <span class="big" lang="ja">{radical.ch}</span>
      <div class="facts">
        <p class="line">
          {radical.strokeCount} {radical.strokeCount === 1 ? "stroke" : "strokes"} ·
          {radical.characters.length === 0
            ? "no jōyō character uses it"
            : `${radical.characters.length} ${radical.characters.length === 1 ? "character" : "characters"}`}
        </p>
        <p class="aside">
          The head form is the whole radical; the shape written inside a character is a
          variant of it.
        </p>
      </div>
    </div>

    <div class="actions">
      <button
        class="primary"
        onclick={() => onpractise?.(radical.number)}
        title="Write the head form itself on the board"
      >
        Write {radical.ch} on the board
      </button>
    </div>
  </div>

  {#if radical.characters.length > 0}
    <p class="members-title">Characters using this radical, most frequent first</p>
    <ul class="members" aria-label="Characters using this radical">
      {#each radical.characters as ch (ch)}
        <li>
          <button class="member" onclick={() => onopen?.(ch)} title="Open {ch} on the board">
            {ch}
          </button>
        </li>
      {/each}
    </ul>
    <p class="aside-note">
      Each one is a jōyō character the course teaches, and opens on the same board the kana
      are written on.
    </p>
  {:else}
    <p class="aside-note">
      No jōyō character uses this radical, so there is no family to show — the head form
      itself can still be written on the board.
    </p>
  {/if}
</section>

<style>
  .stage {
    display: flex;
    flex-direction: column;
    gap: 10px;
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
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }

  .meta {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .big {
    font-size: 3rem;
    line-height: 1;
  }

  .facts {
    min-width: 0;
  }

  .line {
    margin: 0;
    font-size: 0.86rem;
  }

  .aside {
    margin: 2px 0 0;
    font-size: 0.78rem;
    color: var(--muted);
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  /* A finger's width, as every control on a stage is (invariant 33). */
  .primary {
    min-height: 44px;
    padding: 0 16px;
    border-radius: var(--radius);
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    cursor: pointer;
  }

  .members-title {
    margin: 0;
    font-size: 0.78rem;
    font-weight: 600;
  }

  .members {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .member {
    width: 2.6rem;
    height: 2.6rem;
    padding: 0;
    font-size: 1.4rem;
    line-height: 1;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
  }

  .member:hover {
    border-color: var(--accent);
  }

  .aside-note {
    margin: 0;
    font-size: 0.76rem;
    line-height: 1.45;
    color: var(--muted);
    max-width: 72ch;
  }
</style>
