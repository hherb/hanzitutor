<script lang="ts">
  /**
   * The keys, in a card that deliberately does **not** cover the board.
   *
   * This is the whole design of it, and it is the answer to a question the
   * introduction already ran into: a sheet over the board cannot list the board's
   * keys, because the reader cannot try any of them while it is up (see
   * `startupPages.ts`). So this is not a sheet. It sits in a corner of the
   * workspace, takes no focus, traps nothing, and leaves every key live — read
   * one, press it, watch it happen, with the card still there. Escape, the same
   * key again, or the ✕ closes it.
   *
   * It is also not modal in the other direction: a key that closes it is listed
   * in the card, and every key that is *not* listed still does whatever it did
   * before. Nothing about the board changes while this is open.
   */
  import { SHORTCUTS } from "./shortcuts";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();
</script>

<aside class="card" aria-label="Keyboard shortcuts">
  <header>
    <h2>Keys</h2>
    <button class="close" type="button" onclick={onClose} aria-label="Close the key list">✕</button>
  </header>

  <p class="lead">These work on the board. The card stays out of its way, so any of them can be
    tried while this is open.</p>

  <ul>
    {#each SHORTCUTS as shortcut (shortcut.id)}
      <li>
        <kbd>{shortcut.keys}</kbd>
        <span>{shortcut.what}</span>
      </li>
    {/each}
  </ul>

  <p class="note">
    Backspace and the arrow keys in a text field are the field's own: the board's
    keys are ignored while one has focus.
  </p>
</aside>

<style>
  .card {
    position: fixed;
    right: 18px;
    bottom: 18px;
    /* Above the panels, below the phone's navigation sheet (40) and the startup
       reading (50): both of those are meant to cover things. */
    z-index: 20;
    width: 292px;
    max-width: calc(100vw - 24px);
    padding: 12px 14px 14px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
    box-shadow: 0 6px 22px rgba(0, 0, 0, 0.12);
  }
  @media (max-width: 640px) {
    .card {
      right: 12px;
      bottom: 12px;
    }
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  h2 {
    margin: 0;
    font-size: 0.72rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  .close {
    padding: 2px 6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    font: inherit;
    font-size: 0.78rem;
    line-height: 1.4;
    color: var(--muted);
    cursor: pointer;
  }
  .close:hover {
    background: var(--hover);
    color: var(--muted-strong);
  }

  .lead {
    margin: 6px 0 10px;
    font-size: 0.75rem;
    line-height: 1.45;
    color: var(--muted);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  li {
    display: grid;
    grid-template-columns: 4.6rem minmax(0, 1fr);
    align-items: baseline;
    gap: 8px;
  }
  kbd {
    justify-self: start;
    padding: 1px 6px;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: 5px;
    background: var(--bg);
    font-family: inherit;
    font-size: 0.72rem;
    white-space: nowrap;
    color: var(--muted-strong);
  }
  li span {
    font-size: 0.76rem;
    line-height: 1.35;
    color: var(--muted-strong);
  }

  .note {
    margin: 10px 0 0;
    padding-top: 8px;
    border-top: 1px solid var(--line);
    font-size: 0.7rem;
    line-height: 1.45;
    color: var(--muted);
  }
</style>
