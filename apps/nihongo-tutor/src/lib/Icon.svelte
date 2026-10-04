<script lang="ts">
  /**
   * One practice-control glyph.
   *
   * These exist because the board's control row is the one part of the screen
   * that was spending width on words rather than on the board. Four wrapped rows
   * of wide labelled buttons sat under a square board on a phone and cost the
   * board the height it needed, and the row became glyphs for a while.
   *
   * The word is back, in the form the mock-up set it: a glyph on a tinted disc
   * with its short name under it, two tools to a card, three cards under the
   * board. That is a word again, but not the wide button that started this — the
   * name is one line of small text under the disc rather than the thing the
   * border is drawn around, so a card is about as wide as two thumbs and the
   * whole set is one row on a phone and one on a window. The long sentence did
   * not come back either: `title` and `aria-label` still carry it for a pointer
   * and a screen reader, and on a phone the sentence under the row
   * ([`App.svelte`]'s `.hint`) is still what explains a control that cannot be
   * used and why, which is the reason a disabled button is never silent.
   *
   * Every shape is drawn here rather than taken from an icon set. The app ships
   * a pinned list of licence notices with a test that fails when the list and
   * the bundle disagree, and borrowing a kit would mean a notice to add and a
   * licence to honour for shapes this simple. They are stroke-only and take
   * `currentColor`, so a button's own colour — rest, `on`, hover, disabled —
   * carries into the drawing without a second rule per state.
   *
   * The size is `1em`: a button sets its `font-size` and the glyph follows, so
   * the phone can draw them larger without a second copy of the artwork.
   */
  type IconName =
    | "eye"
    | "eye-off"
    | "speaker"
    | "mic"
    | "play"
    | "undo"
    | "trash"
    | "plus"
    | "back"
    | "tick"
    | "next"
    | "practise"
    | "pencil";

  interface Props {
    name: IconName;
  }

  let { name }: Props = $props();
</script>

<svg
  viewBox="0 0 24 24"
  width="1em"
  height="1em"
  fill="none"
  stroke="currentColor"
  stroke-width="1.8"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#if name === "eye"}
    <!-- Trace: the character is on the board, so the answer is in sight. -->
    <path d="M2.6 12C5.1 7.9 8.2 5.9 12 5.9s6.9 2 9.4 6.1c-2.5 4.1-5.6 6.1-9.4 6.1S5.1 16.1 2.6 12Z" />
    <circle cx="12" cy="12" r="2.8" />
  {:else if name === "eye-off"}
    <!-- Recall: the same eye with the answer struck out of it. -->
    <path d="M2.6 12C5.1 7.9 8.2 5.9 12 5.9s6.9 2 9.4 6.1c-2.5 4.1-5.6 6.1-9.4 6.1S5.1 16.1 2.6 12Z" />
    <circle cx="12" cy="12" r="2.8" />
    <path d="M4.4 19.6 19.6 4.4" />
  {:else if name === "speaker"}
    <!-- Listen: the character read aloud. A speaker with sound coming out of it,
         which is what the mock-up draws. It replaced an ear, and the reasoning
         for the ear is worth keeping because it has not stopped being true — a
         speaker says "sound comes out of this machine" where the thing on offer
         is listening to the pronunciation. What outweighs it is the word now
         printed under the glyph: "Listen" beside a speaker is read at a glance,
         while an ear on its own had to be guessed at. -->
    <path d="M4.2 9.4h3.1l4.6-3.8v12.8l-4.6-3.8H4.2Z" />
    <path d="M15.4 9.4a3.7 3.7 0 0 1 0 5.2" />
    <path d="M18 6.9a7.2 7.2 0 0 1 0 10.2" />
  {:else if name === "mic"}
    <!-- Hold to speak: the learner does the talking, so the glyph is the
         microphone that is open exactly while the button is held. -->
    <path d="M12 3.4a2.7 2.7 0 0 1 2.7 2.7v5.3a2.7 2.7 0 0 1-5.4 0V6.1A2.7 2.7 0 0 1 12 3.4Z" />
    <path d="M6 11.4v.5a6 6 0 0 0 12 0v-.5" />
    <path d="M12 17.9v2.8" />
    <path d="M8.8 20.7h6.4" />
  {:else if name === "play"}
    <!-- Strokes: watch the character written one stroke at a time, so the glyph
         is the play triangle that means that everywhere. It replaced three
         written numerals, which is a fine mnemonic for someone who already knows
         what the button does and a puzzle to everyone else; the word under it
         now names the thing being played. -->
    <path d="M9.2 5.8 18.4 12l-9.2 6.2Z" />
  {:else if name === "undo"}
    <!-- Undo: the arrow that means "take the last thing back" everywhere. -->
    <path d="M4.4 9.6h9.4a5.4 5.4 0 0 1 0 10.8H8.6" />
    <path d="M8.2 5.2 4 9.6l4.2 4.4" />
  {:else if name === "trash"}
    <!-- Clear: the bin, which is the standard icon for throwing the lot away
         — distinct from undo, which steps back one stroke. -->
    <path d="M4 6.8h16" />
    <path d="M9.6 6.8V5.1c0-.8.6-1.4 1.4-1.4h2c.8 0 1.4.6 1.4 1.4v1.7" />
    <path d="M6.6 6.8l.8 12.2c.06.9.8 1.6 1.7 1.6h5.8c.9 0 1.64-.7 1.7-1.6l.8-12.2" />
    <path d="M10.3 10.6v6.2M13.7 10.6v6.2" />
  {:else if name === "plus"}
    <path d="M12 5.2v13.6M5.2 12h13.6" />
  {:else if name === "back"}
    <!-- Leaving the board: back to the list, or out of a review session. An
         arrow out rather than a picture of the destination, because the
         direction is the same whichever list it is — the caption under the card
         is what names where it goes. -->
    <path d="M19.4 12H4.6" />
    <path d="M11 5.4 4.4 12l6.6 6.6" />
  {:else if name === "tick"}
    <path d="M4.6 12.6 9.6 17.6 19.4 6.4" />
  {:else if name === "next"}
    <path d="M4.6 12h14.8" />
    <path d="M13 5.4 19.6 12 13 18.6" />
  {:else if name === "practise"}
    <!-- Practise: the board, opened on one entry. A dumbbell is the shape study
         apps use for "drill this", and it is deliberately not a second writing
         tool — a brush and a pencil at 1em are the same smudge, so "write it"
         and "edit it" would take a guess to tell apart. -->
    <path d="M6.6 8.6v6.8M3.4 10.4v3.2M17.4 8.6v6.8M20.6 10.4v3.2M6.6 12h10.8" />
  {:else if name === "pencil"}
    <!-- Edit: the pencil that means "change this" everywhere. It edits the
         entry's reading, meaning and group, which is not the same thing as
         writing the character — hence the dumbbell for that. -->
    <path d="M4.4 19.6l.8-3.4 9.9-9.9 2.6 2.6-9.9 9.9-3.4.8Z" />
    <path d="M12.6 8.9l2.6 2.6" />
  {/if}
</svg>

<style>
  /* A flex item in every button that draws one, so no baseline gap. */
  svg {
    display: block;
  }
</style>
