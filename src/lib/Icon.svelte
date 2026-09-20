<script lang="ts">
  /**
   * One practice-control glyph.
   *
   * These exist because the board's control row is the one part of the screen
   * that was spending width on words rather than on the board. Four wrapped rows
   * of labelled buttons sat under a square board on a phone and cost the board
   * the height it needed. A glyph costs a fixed 44 px however long its name is,
   * so the same controls fit in one row on a wide window and two on a phone.
   *
   * The word did not go away, it moved: every button that draws one of these
   * carries the label in `title` for a pointer and in `aria-label` for a screen
   * reader. On a phone, where there is no hover at all, the sentence under the
   * row ([`App.svelte`]'s `.hint`) is what explains a control that cannot be
   * used and why — which is the same rule the old text labels followed, and the
   * reason a disabled button is never silent.
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
    | "ear"
    | "mouth"
    | "stroke-order"
    | "undo"
    | "trash"
    | "plus"
    | "back"
    | "target"
    | "tick"
    | "next";

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
  {:else if name === "ear"}
    <!-- Hear it: the character read aloud. An ear rather than a speaker,
         because a speaker says "sound comes out of this machine" and the thing
         being offered is listening to the pronunciation. -->
    <path
      d="M8 9.9a4.5 4.5 0 0 1 9 0c0 2.1-1.3 3.2-2.3 4.4-.8 1-1 1.7-1 2.6a2.4 2.4 0 0 1-4.6 1"
    />
    <path d="M10.8 10.1a1.6 1.6 0 0 1 3.2 0v.7a1.4 1.4 0 0 1-1.4 1.4" />
  {:else if name === "mouth"}
    <!-- Hold to say: an open mouth, because the learner does the talking. -->
    <path d="M3.8 9.6h16.4c0 4.9-3.6 8.8-8.2 8.8s-8.2-3.9-8.2-8.8Z" />
    <path d="M9.3 14.4c.7-1.3 4.7-1.3 5.4 0" />
  {:else if name === "stroke-order"}
    <!-- Stroke order: 1, 2, 3 being written, in that order. The numerals are
         drawn as strokes rather than set in text — at 24 px three digits have
         to keep their shape whatever font the platform would have handed us. -->
    <path d="M3.7 4.2L5.9 2.4V11.4" />
    <path d="M9.6 4.9A2.4 2.4 0 0 1 12.0 2.6C14.1 2.8 14.3 5.1 11.8 7.1L9.6 11.2H14.7" />
    <path d="M15.5 4.8A2.2 2.2 0 0 1 17.9 2.6C19.9 2.7 20.1 4.8 18.1 6.3C20.1 7.8 19.9 11.2 17.9 11.2A2.2 2.2 0 0 1 15.5 8.9" />
    <path d="M20.2 20.6L16.0 18.6L5.2 18.6L5.2 22.6L16.0 22.6Z" />
    <path d="M8.4 18.6L8.4 22.6" />
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
         arrow out rather than a word, because the destination is in the
         tooltip and the direction is the same whichever list it is. -->
    <path d="M19.4 12H4.6" />
    <path d="M11 5.4 4.4 12l6.6 6.6" />
  {:else if name === "target"}
    <!-- Corrections: where the strokes should have been. A target says
         "accuracy", which is what pressing Check measures. -->
    <circle cx="12" cy="12" r="7.4" />
    <circle cx="12" cy="12" r="2.6" />
    <path d="M12 2.4v3.2M12 18.4v3.2M2.4 12h3.2M18.4 12h3.2" />
  {:else if name === "tick"}
    <path d="M4.6 12.6 9.6 17.6 19.4 6.4" />
  {:else if name === "next"}
    <path d="M4.6 12h14.8" />
    <path d="M13 5.4 19.6 12 13 18.6" />
  {/if}
</svg>

<style>
  /* A flex item in every button that draws one, so no baseline gap. */
  svg {
    display: block;
  }
</style>
