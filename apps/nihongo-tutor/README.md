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

## The datasets

Four artifacts, all committed, all gzip + magic + `postcard`, and all embedded
with `include_bytes!` — so a clone builds and a release runs with no data step.

| Artifact | Size | What it holds |
| --- | --- | --- |
| `kana.bin.gz` | 70,917 | 177 kana — 86 hiragana, 91 katakana — with their outlines, centre-lines and course |
| `kanji.bin.gz` | 3,236,713 | 2,136 jōyō kanji, their geometry, readings, glosses, grades and IDS decompositions, plus the 214 radical head forms |
| `words.bin.gz` | 601,816 | 16,073 words with their own readings, furigana, glosses and ladder band |
| `passages.bin.gz` | 502 | the three reading passages, segmented into tokens |

`prepare-kana` does not merely copy: AnimCJK stores a kana whose stroke crosses
itself as several drawing segments, so **25 kana arrive with more "strokes" than
are taught** and the pipeline folds them back from the SVG element ids.
`prepare-kana` refuses to write an artifact whose stroke counts it could not check
against KanjiVG, and `prepare-kanji` refuses a jōyō set that is not exactly 2,136
characters or a KanjiVG disagreement that is not written down.

Provenance, licences and what was changed are recorded in
[`../../LICENSES.md`](../../LICENSES.md); the handover is
[`../../HANDOVER_NIHONGO.md`](../../HANDOVER_NIHONGO.md) and what to build next is
[`../../ROADMAP_NIHONGO.md`](../../ROADMAP_NIHONGO.md).

## Notices

`src-tauri/src/licences.rs` is this app's catalogue: ten notices, the text
compiled in with `include_str!` and also copied into the bundle. Five of them are
files in `src-tauri/licences/` — the AnimCJK statement, EDRDG's JMdict/KANJIDIC2
attribution, the MIT text for JmdictFurigana and its provenance notice, and the
UniDic/lindera analyser notice — and they are kept app-local rather than in the
repository's shared `licences/` directory because that directory's own test
requires every file in it to be catalogued by the **Chinese** app; a kana notice
there would make that app ship and display a notice for data it does not contain.
The licence *texts* the app references out of the shared directory — the AGPL,
the LGPL, the Arphic Public License and the CC BY-SA legal code — genuinely are
shared and stay there.

The notices are not decoration: EDRDG's terms require a software package that uses
its dictionaries to acknowledge them on a screen reached from a menu, to ship the
licence files, and to keep the data updated. The Licences screen is the first,
`bundle.resources` is the second, and `LICENSES.md` records the refresh procedure
for the third.

`tests/licences.rs` holds the same three-way correspondence the main app's does:
every catalogued notice exists on disk and is the text compiled into the binary,
every file in the app's notice directory is catalogued, and the bundle config
copies exactly that set. It also reads the text of **every** catalogued notice, so
a notice added to the catalogue and not to its table fails rather than passing as
a placeholder.

## Shipping it

## Not done yet

That must hold all ten files the catalogue names. See `LICENSES.md`,
"Before you distribute", for the EDRDG and CC BY-SA obligations a distributor
keeps, and the repository `README.md`, "Shipping a build", for signing and
notarisation.
