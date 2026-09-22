<script lang="ts">
  /**
   * The startup sheet: a few pages read once, then out of the way.
   *
   * It renders one of two sets — the introduction, for a device that has never
   * run the app, or what changed, for one that has (see `startupPages.ts` for
   * both, and for the rule that decides which). This component is deliberately
   * about neither: it is handed `pages` and it shows them. Where the content
   * comes from, whether it should be shown at all, and what dismissing it
   * records are the *host's* business, which is what keeps the whole
   * "shown once" rule in one place instead of split between a sheet and its
   * caller.
   *
   * ## Why the content is data
   *
   * [`pages`] is an array, not markup, so the step count, the "n of m" and the
   * dots cannot disagree with what is on screen: adding a page is adding an
   * entry. The pages themselves live in `startupPages.ts`.
   *
   * ## Deliberately not here
   *
   * - **No backdrop dismissal.** A mis-tap on a phone would throw the whole
   *   reading away. Every route out is a control the learner aimed at: Skip,
   *   Finish, Escape, or Back on Android.
   * - **No "don't show this again" checkbox.** Declining to read it once is the
   *   whole of what Skip says, and the settings screen can show it again.
   */
  import { onMount } from "svelte";
  import type { Page } from "./startupPages";

  interface Props {
    /**
     * The pages to read, in order. **Must not be empty** — a sheet with no pages
     * is a bug in the caller, and this renders nothing rather than an empty card.
     */
    pages: Page[];
    /**
     * Called when the reading is finished *or* skipped.
     *
     * The two are one act — "do not show me this again" — so they arrive as one
     * callback rather than two the host would have to remember to treat alike.
     * What the host does with it is its own business; here it only means the
     * sheet should go.
     */
    onDone: () => void;
    /**
     * What the last page's primary button says.
     *
     * The host's business because it depends on what is being read: the
     * introduction ends by sending a learner to the board ("Start practising"),
     * while release notes end by closing something already read ("Done"). The
     * default suits the introduction, which is the longer and more common case.
     */
    finishLabel?: string;
  }

  let { pages, onDone, finishLabel = "Start practising" }: Props = $props();

  /** Which page is showing. */
  let step = $state(0);
  /** The last page's index, so nothing counts down a hard-coded number. */
  const lastStep = $derived(pages.length - 1);
  /** The page itself, so the markup never indexes the array twice. */
  const page = $derived<Page | undefined>(pages[step]);
  /** True on the last page, where the primary button finishes instead. */
  const onLastStep = $derived(step === lastStep);

  /**
   * The sheet itself, focused when it opens.
   *
   * Focus goes to the sheet and not to a button, so a screen reader announces
   * the dialog and the page's heading before the controls, and so Space and
   * Enter cannot fire a control the learner has not heard yet.
   */
  let sheet = $state<HTMLElement | null>(null);
  onMount(() => sheet?.focus());

  function next() {
    if (onLastStep) onDone();
    else step += 1;
  }
</script>

<!-- Nothing rather than an empty card: see the `pages` precondition above. -->
{#if page}
  <!-- The overlay covers the whole window and swallows pointer events, which is
       what a modal should do — the board behind it is not reachable while this is
       up (the same reason `App.svelte`'s key handler returns early). It carries no
       click handler on purpose: see the note above on why clicking outside does
       not dismiss this. -->
  <div class="overlay">
    <div
      class="sheet"
      role="dialog"
      aria-modal="true"
      aria-labelledby="startup-title"
      tabindex="-1"
      bind:this={sheet}
    >
      <header>
        <h2 id="startup-title">{page.title}</h2>
        <p class="count">Step {step + 1} of {pages.length}</p>
      </header>

      <div class="page">
        <p class="lead">{page.lead}</p>

        {#if page.points}
          <ul class="points">
            {#each page.points as point (point.name)}
              <li>
                <span class="point-name">{point.name}</span>
                <span class="point-what">{point.what}</span>
              </li>
            {/each}
          </ul>
        {/if}

        {#if page.note}
          <p class="note">{page.note}</p>
        {/if}
      </div>

      <!-- The dots are decoration: "Step n of m" above is the same fact in words,
           which is what a screen reader reads and what survives a zoom. -->
      <div class="dots" aria-hidden="true">
        {#each pages as _, index (index)}
          <span class="dot" class:on={index === step}></span>
        {/each}
      </div>

      <footer>
        <button class="skip" type="button" onclick={onDone}>Skip</button>

        <div class="step-controls">
          {#if step > 0}
            <button type="button" onclick={() => (step -= 1)}>Back</button>
          {/if}
          <button class="primary" type="button" onclick={next}>
            {onLastStep ? finishLabel : "Next"}
          </button>
        </div>
      </footer>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    /* Above the phone's navigation sheet (40) and its scrim (30), because this
       covers the whole app rather than one column of it. */
    z-index: 50;
    /* A flex row with an auto-margined sheet, rather than `place-items: center`:
       a grid or a centred flex item clips the *top* of its content once the sheet
       is taller than the window, and a short landscape phone does exactly that.
       The overlay scrolls instead, and the sheet keeps its own internal scroll
       for the page body once it reaches the window's height. */
    display: flex;
    align-items: flex-start;
    justify-content: center;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: calc(var(--safe-top) + 16px) 16px calc(var(--safe-bottom) + 16px);
    background: rgb(15 23 42 / 0.45);
  }

  .sheet {
    display: flex;
    flex-direction: column;
    gap: 14px;
    /* `auto` centres it in a window with room to spare, and leaves it at the top
       of a scrolled overlay when there is not. */
    margin: auto;
    width: 100%;
    /* Narrow enough to stay readable, wide enough for the longest measure's
       sentence to sit on two lines rather than six. */
    max-width: 560px;
    /* Never taller than the window the overlay is sized to, so the header and
       the buttons stay reachable on a phone. */
    max-height: 100%;
    padding: 20px;
    border-radius: 14px;
    background: var(--surface);
    box-shadow: 0 18px 50px rgb(15 23 42 / 0.3);
  }
  .sheet:focus {
    /* Focused on open for the screen reader, and focus-visible would then ring
       the whole sheet, which reads as an error rather than as focus. The
       controls inside keep their own rings. */
    outline: none;
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .count {
    margin: 0;
    flex: none;
    font-size: 0.76rem;
    color: var(--muted);
  }

  /* The page scrolls, not the sheet: the header and the buttons stay put on a
     short phone, and a long list of measures never pushes Finish off-screen. */
  .page {
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
    overscroll-behavior: contain;
    min-height: 0;
  }

  p {
    margin: 0;
  }
  .lead,
  .note {
    font-size: 0.86rem;
    line-height: 1.55;
    color: var(--muted-strong);
  }
  .note {
    color: var(--muted);
  }

  .points {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 12px;
    background: var(--bg);
  }
  .points li {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .point-name {
    font-size: 0.82rem;
    font-weight: 640;
    color: var(--muted-strong);
  }
  .point-what {
    font-size: 0.8rem;
    line-height: 1.5;
    color: var(--muted);
  }

  .dots {
    display: flex;
    justify-content: center;
    gap: 6px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--line);
  }
  .dot.on {
    background: var(--accent);
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    /* The row a phone wraps: Skip on the left, the step controls on the right,
       and nothing else competing for the width. */
    flex-wrap: wrap;
  }
  .step-controls {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }
  footer button {
    padding: 8px 14px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--surface);
    font: inherit;
    font-size: 0.83rem;
    color: var(--muted-strong);
    cursor: pointer;
  }
  footer button:hover {
    background: var(--hover);
  }
  footer .skip {
    border-color: transparent;
    color: var(--muted);
  }
  footer .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  footer .primary:hover {
    background: var(--accent-ink);
  }

  @media (max-width: 760px) {
    .sheet {
      padding: 16px;
      gap: 12px;
    }
  }
</style>
