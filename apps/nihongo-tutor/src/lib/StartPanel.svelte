<script lang="ts">
  /**
   * Start here: what the writing system is for, before the first lesson.
   *
   * This is the app's one screen that is not a course and not a notice — it is
   * about the language rather than about a set of characters — so it belongs to
   * neither course and hangs off the footer beside Licences (`lib/nav.ts`). It is
   * reachable from both, and the two buttons at the bottom are how a learner
   * leaves it for whichever course they want.
   *
   * The claim it makes that a beginner most often gets wrong is that romanised
   * writing is a way to read Japanese. The counter-evidence is the app's own
   * vocabulary, fetched over IPC: one reading, several unrelated words, each with
   * the reading and the gloss the dictionary gives it. Nothing is typed into this
   * file as a fact about Japanese — the words, their readings and their glosses
   * all come from `words.bin.gz`, and the counts come with them.
   */
  import * as api from "./api";
  import type { Section } from "./nav";
  import { ALIKE_PAGE, ALIKE_READINGS, alikeNote } from "./start";
  import type { WordsOfReading } from "./types";

  /** Open one of the two courses, from the buttons at the bottom. */
  let { oncourse }: { oncourse: (section: Section) => void } = $props();

  let alike = $state<WordsOfReading[]>([]);
  let error = $state<string | null>(null);

  // Asked once. The effect reads nothing that it writes — the pages arrive in a
  // continuation — so there is no loop to fall into (trap 9).
  $effect(() => {
    Promise.all(ALIKE_READINGS.map((reading) => api.wordsOfReading(reading, 0, ALIKE_PAGE)))
      .then((pages) => (alike = pages))
      .catch((e) => (error = String(e)));
  });
</script>

<section class="start">
  <h2>Start here</h2>
  <p class="lede">
    Japanese is written with two kinds of character, and the reason is worth five
    minutes before the first lesson: it is what makes the characters look like a
    system rather than a list.
  </p>

  <h3>The characters came from China</h3>
  <p>
    Japanese had no writing system of its own. Chinese characters arrived from about
    the fifth century onwards — along the routes Buddhism travelled, and later with
    the Japanese missions to Sui and Tang China — and Japanese was written with them.
    What the Japanese took from each character was its <strong>meaning</strong>, and
    they read it with their own words: 山 is “mountain” wherever it appears, while
    its sound depends on the word it stands in — やま on its own, さん in 富士山. The
    shape carries the meaning; the sound comes from the word.
  </p>

  <h3>The kana carry the sound and the grammar</h3>
  <p>
    Japanese grammar lives in endings and particles, and those had to be written
    too. Chinese needs no such machinery to the same degree: its grammar is carried
    by word order and a small set of particles, so a text in characters can do most
    of the job on its own. Japanese inflects every verb and adjective — 食べる,
    食べた, 食べない, 食べれば — and marks the role of every part of the sentence with
    a particle, so the endings matter as much as the stems. Two syllabaries were
    derived from simplified characters to write exactly that: hiragana and katakana,
    one character to one sound, so they are also where a reading is written down.
    <strong>The kanji carry the meaning; the kana carry the sound and the
    grammar.</strong>
  </p>

  <h3>Why romanisation is not a substitute</h3>
  <p>
    Roman letters can spell the sounds, and that is all they can do. They cannot say
    <strong>which word</strong> is meant: Japanese has few distinct sounds and no tones
    of the kind that keep Chinese words apart (it has a pitch accent, but it is not
    written and it separates far fewer words), so a great many words sound alike — and
    on the page it is the character that tells them apart. In speech the sentence and
    the situation do that work; a spelling has nothing to do it with.
  </p>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#each alike as page (page.reading)}
    <div class="alike">
      <p class="alike-head">
        <span class="alike-reading">{page.reading}</span>
        <span class="alike-note">{alikeNote(page.total, page.words.length, page.reading)}</span>
      </p>
      <ul>
        {#each page.words as word (word.text + word.reading)}
          <li>
            <span class="alike-text">{word.text}</span>
            <span class="alike-said">{word.reading}</span>
            <span class="alike-gloss">{word.meaning}</span>
          </li>
        {/each}
      </ul>
    </div>
  {/each}

  {#if alike.length > 0}
    <p>
      Every one of those is correct, and no romanisation can choose between them.
      It is not one word's accident either: this is how the language is built.
    </p>
  {/if}

  <p>
    And a romanised spelling does not say how the word sounds either, because it is
    read with the reader's own sound system. <em>hashi</em> to an English reader is an
    English word — an English <em>h</em>, a vowel Japanese does not have, no pitch —
    and ら is an English <em>r</em>, not the Japanese flap. Three readers given the
    same spelling produce three different pronunciations and none of them is Japanese.
    Kana have no such latitude: one character is one mora, and the same sound every
    time.
  </p>

  <h3>What that means while you learn</h3>
  <p>
    The two scripts do the two jobs. <strong>Kana are the sound</strong> — every
    reading in this app is written in them, and so is the grammar, because every
    ending and every particle is kana. That is why the kana course comes first, and
    why it is only a few days' work. <strong>Kanji are the meaning</strong> — each one
    is a word-part with more than one possible sound, which is why this course teaches
    them inside words (大人 is おとな, 今日 is きょう) rather than with one reading per
    character. The romanisation in the app is there to type with and to fall back on;
    it is not how the language is written.
  </p>
  <p class="closing">
    The commonest mistake a beginner makes is to treat a romanised spelling as if it
    were Japanese. It is not: it leaves out which word was meant, and it invites the
    reader to supply sounds the language does not have. Kana for the sound, kanji for
    the meaning — that is the whole design.
  </p>

  <div class="go">
    <button class="primary" onclick={() => oncourse("kana")}>Learn the kana first</button>
    <button onclick={() => oncourse("kanji")}>Go to the kanji course</button>
  </div>
</section>

<style>
  .start {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 16px;
    padding: 16px 18px 18px;
  }

  h2 {
    font-size: 1.15rem;
    margin: 0 0 6px;
  }

  h3 {
    font-size: 0.95rem;
    margin: 18px 0 6px;
    letter-spacing: 0.01em;
  }

  p {
    margin: 0 0 10px;
    max-width: 72ch;
    font-size: 0.9rem;
  }

  .lede {
    color: var(--muted);
    margin-bottom: 14px;
  }

  /* The argument's evidence: one reading, and the words this course teaches that
     carry it. A card of its own so that the numbers and the glosses read as data
     rather than as part of the prose. */
  .alike {
    background: color-mix(in srgb, var(--line) 26%, transparent);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 10px 12px;
    margin: 10px 0;
    max-width: 72ch;
  }

  .alike-head {
    margin: 0 0 6px;
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
  }

  .alike-reading {
    font-size: 1.5rem;
    line-height: 1.1;
    letter-spacing: 0.05em;
  }

  .alike-note {
    color: var(--muted);
    font-size: 0.8rem;
  }

  .alike ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 3px;
  }

  .alike li {
    display: grid;
    grid-template-columns: minmax(3.5rem, auto) minmax(4rem, auto) minmax(0, 1fr);
    gap: 10px;
    align-items: baseline;
  }

  .alike-text {
    font-size: 1.25rem;
  }

  .alike-said {
    color: var(--muted);
    font-size: 0.85rem;
  }

  .alike-gloss {
    color: var(--muted);
    font-size: 0.82rem;
  }

  .closing {
    border-left: 3px solid var(--accent);
    padding-left: 10px;
    color: var(--ink);
  }

  .go {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 16px;
  }

  .go button {
    padding: 7px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--ink);
    cursor: pointer;
    font: inherit;
  }

  .go button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
</style>
