# Kana Tutor

An offline app for learning hiragana and katakana. It is the third app in this
workspace, after Hanzi Tutor (Chinese characters) and Tone Trainer (Mandarin
tones), and it shares the geometry engine with both.

## What it teaches

The course is the **gojūon**, in the order it is taught:

| | Lessons |
| --- | --- |
| Hiragana | 18 — eleven plain rows (あ か さ た な は ま や ら わ ん), five voiced rows (が ざ だ ば ぱ), the small kana, the rare ones |
| Katakana | 20 — the same, plus the v-series ヷ ヸ ヹ ヺ and the prolonged sound mark ー, which hiragana does not have |

Each kana can be **animated** stroke by stroke, **traced** on the board and
**graded** — shape, placement, ink and stroke order, against real stroke geometry
— and its **reading typed** in either Hepburn or Kunrei-shiki romanisation.

There are three views. **Practice** is the course and the board. **Tell them
apart** is a discrimination drill: it prompts with a reading and offers the kana
that shape is actually confused with, because シ and ツ differ only in stroke
*direction* and repeating either one alone never teaches the difference. **Licences**
shows every notice this app owes, in full.

Everything is embedded. There is no network path in this app at all.

## Running it

```bash
cd apps/nihongo-tutor
pnpm install
pnpm run dev          # dev server on :1422, and the Tauri window
pnpm run check:web    # svelte-check
pnpm run test:rust    # from the repo root: cargo test -p nihongo-tutor
```

From the repository root, `pnpm run test:web` runs this app's pure-TypeScript
tests alongside the other two projects'.

`pnpm run build` from here bundles it, through the repository's
`scripts/build-release.sh`, which is parameterised by `TAURI_ROOT`.

## Where the code comes from

The engine is **not** reimplemented. `crates/hanzi-core`'s `geom`, `raster` and
`grade` modules grade strokes against strokes and never look at the character, so
they are language-neutral; `crates/nihongo-core` supplies the kana data, the
course and the romaji input on top, and the scoring that judges a handwritten あ
is the same code that judges 一.

Two frontend files are **lifted** from Hanzi Tutor rather than shared through a
package, which is the pattern this repository already uses between its apps (Tone
Trainer lifted its pitch chart and its transcript rule the same way):

| File | What changed |
| --- | --- |
| `src/lib/render.ts` | **Nothing.** It paints SVG outlines in font space and centre-lines in display space. AnimCJK publishes its kana in Make Me a Hanzi's own frame — same box, same y-up convention — so the transform and the painting are already correct for a kana. `types.ts` aliases `Character = Kana` so the file can stay untouched. |
| `src/lib/KanaCanvas.svelte` | **New**, not lifted. It is the interaction only — pointer capture, the animation frame — since Hanzi Tutor's `PracticeCanvas` carries tone-panel and verdict-display concerns this app does not have. |
| `src/lib/board.ts` | **New.** The board's arithmetic as pure functions — the pointer-to-display mapping and the animation's progress — so it can be tested without a canvas. `board.test.ts` is run by the **root** project's vitest. |

If `render.ts` is ever changed here, change it in `src/lib/render.ts` too, or
record why they diverge.

## The dataset

`crates/nihongo-core/data/kana.bin.gz` — committed, 177 kana, gzip + magic +
`postcard`. It is built by `pnpm run prepare-kana` from AnimCJK, and the build
does not merely copy: AnimCJK stores a kana whose stroke crosses itself as several
drawing segments, so **25 kana arrive with more "strokes" than are taught** and
the pipeline folds them back. `prepare-kana` refuses to write an artifact whose
stroke counts it could not check against KanjiVG.

Provenance, licences and what was changed are recorded in
[`../../LICENSES.md`](../../LICENSES.md).

## Notices

`src-tauri/src/licences.rs` is this app's catalogue, and
`src-tauri/licences/AnimCJK-COPYING.txt` is the one notice specific to it. It is
kept app-local rather than in the repository's shared `licences/` directory
because that directory's own test requires every file in it to be catalogued by
the **Chinese** app — adding a kana notice there would make that app ship and
display a notice for data it does not contain. The two licence *texts* the
AnimCJK statement points at are referenced out of the shared directory, because
those genuinely are shared.

`tests/licences.rs` holds the same three-way correspondence the main app's does:
every catalogued notice exists on disk and is the text compiled into the binary,
every file in the app's notice directory is catalogued, and the bundle config
copies exactly that set.

## Not done yet

- No audio. macOS ships Japanese system voices and `hanzi-voice` already drives
  them, so this is wiring rather than research.
- No spaced repetition. `hanzi-store` and `hanzi-core`'s scheduler are reusable
  as-is, and a kana course wants a review queue.
- Dakuten and yōon are taught as rows and digraphs; there is no drill that hunts
  for a learner's *own* confusions yet, which is what `KanaDataset` and the
  confusable set are there to support.
