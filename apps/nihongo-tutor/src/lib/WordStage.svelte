<script lang="ts">
  /**
   * The word stage: one word, with its furigana, its reading, the band it sits on
   * and the box that checks a typed reading — and nothing else.
   *
   * ## Why it is a screen of its own
   *
   * `VocabularyPanel` is the course: the ladder and the band's words. The card used
   * to be drawn in a second column beside that list, which is the same shape N12
   * fixed on the kanji course — on a phone a learner tapped a row and then had to
   * scroll *past* twenty-four rows to reach the card, and back to reach the next
   * word. `HANDOVER_NIHONGO.md` invariants 33 and 34 are the rule; the chrome steps
   * aside while a stage is up and `Words` is the way back.
   *
   * ## What is spoken, and what is never spoken
   *
   * The card speaks the word's **own reading** — ひとつ for 一つ — and never the
   * written form: a synthesiser handed 大人 would choose a reading, and the app does
   * not choose readings (invariants 21 and 27). That is `WordCard`'s rule and this
   * screen adds nothing to it.
   *
   * ## No board, and no arrows
   *
   * A word is checked by typing, not by a hand, so there is no board here and nothing
   * to schedule; and the band's order is the list's business rather than a sequence
   * this screen steps through, which is why — unlike the kanji and kana stages — it
   * has no arrows either side of the thing it shows.
   */
  import Icon from "./Icon.svelte";
  import WordCard from "./WordCard.svelte";
  import type { Word } from "./types";
  import type { VoiceStatus } from "./speech";

  interface Props {
    /**
     * The word this stage is about, as the course screen's row already holds it —
     * text, reading, gloss, band and the furigana alignment, all of which the card
     * draws.
     */
    word: Word;
    /** What the `voice` command answered, asked once by `App.svelte`. */
    voice: VoiceStatus;
    /** Back to the band. */
    onleave: () => void;
  }

  let { word, voice, onleave }: Props = $props();
</script>

<section class="stage">
  <header class="head">
    <!--
      Out of the card and back to the band's list. The word says where it goes
      rather than leaving an arrow to be guessed at — the kanji and kana stages'
      own arrangement.
    -->
    <button class="back" onclick={onleave} title="Back to this band's words">
      <Icon name="back" />
      <span>Words</span>
    </button>
    <!-- Which rung of the ladder the word is on. The card says it too, under the
         meaning; here it is the one thing above the fold on a phone. -->
    <span class="where">{word.bandName}</span>
  </header>

  <WordCard {word} {voice} />
</section>

<style>
  .stage {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 90ch;
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
</style>
