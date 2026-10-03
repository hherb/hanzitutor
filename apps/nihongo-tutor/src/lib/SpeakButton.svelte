<script lang="ts">
  /**
   * One "Hear it" control, shared by every screen that has something to say.
   *
   * It exists because there are three of them and they have to behave the same
   * way: the kana on Practice, a word's own reading on its card, and a due kana on
   * the review board. What they share is not the button — it is the rule about
   * what happens when the machine has no Japanese voice, which is why the note
   * lives in `speech.ts` as a pure function and this file only renders it.
   *
   * Two decisions are in the props rather than in here:
   *
   * * **what is spoken is passed in, and is always kana.** A kana character, or a
   *   word's own stored reading. Nothing is composed from a written form, for
   *   `HANDOVER_NIHONGO.md` invariant 21's reason — 大人 is おとな, and a bare kanji
   *   has a dozen readings the app has no business choosing between. A screen with
   *   only characters to offer therefore offers no button, rather than an
   *   arbitrary reading.
   * * **the voice status is passed in** rather than fetched here, so one screen
   *   asks once and every button on it agrees. A list of sixty word cards would
   *   otherwise ask sixty times.
   *
   * Speaking cuts off whatever was already being said — that is `Speaker::speak`'s
   * own contract — so there is no Stop control: a kana or a word's reading is over
   * in a moment, and a second press is the way to hear it again.
   */
  import * as api from "./api";
  import { canHear, voiceNote, type VoiceStatus } from "./speech";

  let {
    /** What to say. Kana: a character, a word's stored reading, a kanji reading. */
    text,
    /** What the `voice` command answered, asked once by the screen above. */
    voice,
    /**
     * What the control says.
     *
     * The kanji card passes the **reading itself**, so the thing the learner reads
     * is the thing they press — `た.べる` with its okurigana mark, which is the
     * teaching information, while `text` is the same reading with the mark off.
     */
    label = "Hear it",
    /**
     * Whether to say why nothing can be heard.
     *
     * On for a control that stands alone — Practice, a word card, a due character
     * — where the reason belongs next to the thing that does not work. Off for the
     * kanji card's readings, which are twenty controls on one screen in the worst
     * case (生): the panel says it once instead, and twenty copies of a settings
     * path is not more helpful than one.
     */
    note = true,
    /**
     * How the control is drawn.
     *
     * `button` is the boxed control the three standalone callers use. `link` is a
     * reading, drawn as the text it already was: on the kanji card the readings are
     * a list to read, and a grid of bordered buttons would bury the readings
     * themselves. In `link` form a disabled control is drawn as plain text rather
     * than a greyed-out button, so a machine with no Japanese voice sees exactly
     * the card it saw before any of this existed.
     */
    appearance = "button",
  }: {
    text: string;
    voice: VoiceStatus;
    label?: string;
    note?: boolean;
    appearance?: "button" | "link";
  } = $props();

  let error = $state<string | null>(null);
  const available = $derived(canHear(voice));
  const why = $derived(voiceNote(voice));
  const title = $derived(why ?? `Say ${text} in the system's Japanese voice`);

  async function hear() {
    try {
      error = null;
      await api.speak(text);
    } catch (e) {
      // The synthesiser itself refused — a voice that vanished between the list
      // being resolved and the press, say. Said rather than swallowed, and the
      // button stays usable so a second press can work.
      error = String(e);
    }
  }
</script>

<span class="speak">
  <button
    type="button"
    class={appearance}
    onclick={() => void hear()}
    disabled={!available}
    {title}
  >
    {label}
  </button>
  {#if note && why}
    <span class="note">{why}</span>
  {/if}
  {#if error}
    <span class="error" role="alert">{error}</span>
  {/if}
</span>

<style>
  .speak {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  button {
    font: inherit;
    padding: 0.4rem 0.8rem;
    border: 1px solid var(--line, #dcdcd6);
    border-radius: 6px;
    background: var(--panel, #fff);
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* The reading itself, drawn as the text it is. The dotted underline is the same
     affordance the passage uses for a tappable word, so "this can be pressed" reads
     the same way in both places. */
  button.link {
    padding: 0;
    border: 0;
    border-bottom: 1px dotted var(--line, #dcdcd6);
    border-radius: 0;
    background: none;
    line-height: 1.35;
    cursor: pointer;
  }

  button.link:hover:not(:disabled) {
    border-bottom-style: solid;
  }

  /* With no voice installed this is not a broken control, it is the reading the
     card always showed. `color: inherit` is load-bearing: a disabled `<button>`
     gets the user agent's grey `GrayText`, and a card of grey readings looks like
     a card of failures rather than a card of readings. */
  button.link:disabled {
    opacity: 1;
    color: inherit;
    border-bottom: 0;
  }

  .note,
  .error {
    font-size: 0.8rem;
    color: var(--muted, #6b6b6b);
    /* The note is long — a settings path — and must not push the board around. */
    max-width: 34ch;
  }

  .error {
    color: var(--bad, #a3341f);
  }
</style>
