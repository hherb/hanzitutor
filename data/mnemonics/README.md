# Mnemonics

Memory hooks for characters: a short, concrete story per character that ties the
components and the sound to the meaning.

The rule that shapes everything here: **the facts are open data, the story is
ours.** Decomposition, pinyin, rank and component glosses are joined in from
`data/raw/`; only the story text is authored, and it is licensed CC BY-SA 4.0.
No mnemonic here is copied from Remembering the Hanzi/Kanji, WaniKani, KKLC,
Henshall, Kanji Damage or any Anki deck — those are copyrighted, and none of
their text is an input to this build.

## The sound column is not a pronunciation guide

`sound` is an English approximation chosen to jog the memory — *duh* for 的,
*way* for 为, *my* for 买. English spelling and Chinese readings disagree, and a
hook only has to be close enough to call the character back. **Read the pinyin
for the actual sound.** The note is shown under every hook in the app, not just
here, because a hook presented as a pronunciation would teach the wrong thing.

## Files

| File | Role |
| --- | --- |
| `zh-hsk1.source.tsv`, `zh-hsk2.source.tsv` | **The source of truth.** One character per line, tab separated: `character <TAB> sound <TAB> meaning <TAB> mnemonic`. Edit these. |
| `zh-hsk1.jsonl`, `zh-hsk2.jsonl` | Generated records, enriched with pinyin, rank, IDS decomposition and component glosses. Do not edit. |
| `zh-hsk1.preview.md`, `zh-hsk2.preview.md` | Generated review sheets — the same records as tables, for reading and marking up. Do not edit. |
| `mnemonics.json` | Generated lookup `{character: {level, sound, mnemonic}}`, bundled by `src/lib/mnemonics.ts`. Do not edit. |

`data/raw/` is git-ignored because it can be re-fetched; this directory is not,
so the authored TSVs and the generated records commit normally.

## Build

```sh
python3 scripts/build-mnemonics.py
```

For every level in the script's `LEVELS`, it fills the records' context from:

* `hanziDB.csv` — pinyin, frequency rank, HSK level. Note it carries duplicated
  rows: **174 distinct** HSK 1 characters (not the 231 rows) and **173 distinct**
  HSK 2 characters (not 194). The build de-duplicates by character.
* `dictionary.txt` (Make Me a Hanzi) — the IDS decomposition and each component's
  definition. Component glosses are the first sense of that definition, with a
  short override table in the script for the handful whose upstream gloss is a
  philological aside rather than an image (`疋` → foot, `未` → wheat, `丷` → two
  dots, and so on).

The build fails on a malformed line, a duplicate character, a mnemonic outside
10–400 characters, a character outside its level's set, or a level file with no
records. Coverage is printed per level, so a gap shows up rather than hiding.

Current coverage: **347 characters** — HSK 1 complete (174/174), HSK 2 complete
(173/173).

## Record shape

```json
{
  "char": "的",
  "pinyin": "de",
  "meaning": "possessive particle",
  "sound": "duh",
  "mnemonic": "The white (白) spoon (勺) on the table is obviously mine — duh. It marks who owns what.",
  "decomposition": "⿰白勺",
  "components": [{"char": "白", "gloss": "white", "definition": "white"}, …],
  "level": "hsk1",
  "rank": 1,
  "license": "CC-BY-SA-4.0",
  "source": "original",
  "generator": "deepseek-flash",
  "generated": "2026-10-17",
  "reviewed": false
}
```

`meaning` is deliberately the sense the course teaches, not the dictionary's
full entry: 后 is "after, behind", not the dictionary's first sense "queen".
`reviewed` stays `false` until a human has read the story; treat every record as
a draft.

## Method

* **Fixed images, reused everywhere.** 亻 is a person, 扌 a hand, 讠 speech,
  氵 water, 木 a tree, 宀 a roof, 心 a heart, 灬 fire, ⺼ flesh. Reusing the same
  image across characters is what makes the set compound: once 扌 is a hand in
  我, it is a hand in 打, 找, 把.
* **Say the sound.** The `sound` field is the English hook the story leans on
  (妈 *ma*, 买 *my*, 冷 *lung*). It is optional; leave it empty when there is no
  honest hook in English.
* **One scene, two sentences.** Put the components in the scene and land on the
  meaning. If the story needs a paragraph it is not working.
* **Not etymology.** Several of these (面, 后, 京) are sound or shape stories
  because the character's real history is contested or boring. The Make Me a
  Hanzi etymology hint the app also displays is the etymology; this field is the
  hook. Do not present one as the other.
* **No borrowed text.** If a story is recognisable from a book or an app, it is
  not usable here — rewrite it from the components.

## Licence and provenance

The mnemonic text is **CC BY-SA 4.0**. It is the project's own writing, drafted
with a language model and edited here, and the share-alike matches the rest of
the project's content.

When it ships, the notice does too:

* `licences/MNEMONICS-CC-BY-SA-4.0.txt` — attribution, the licence, and the two
  caveats (not an etymology, not a pronunciation);
* the `mnemonics` entry in `src-tauri/src/licences.rs`, which puts it on the
  Licences screen and into the bundle;
* the `LICENSES.md` section that records the provenance.

## Showing it in the app

**Done: route 1, the bundled asset.** `mnemonics.json` is imported by
`src/lib/mnemonics.ts` and looked up by character; `CharacterPanel.svelte` and
`App.svelte` render the story under the etymology, with the sound caveat beneath
any hook. No artifact change, no format bump, so the prose can be corrected and
rebuilt without touching the 13 MB dataset.

**Still open: route 2, the artifact.** A `mnemonic: String` (and `sound`) next to
`etymology` in `Character` (`crates/hanzi-core/src/dataset.rs`), with
`ARTIFACT_MAGIC` bumped to `HANZID04` the way `03` added the decomposition. That
is the shape to move to once the prose has been reviewed — one data path instead
of two.

## Extending

* **HSK 3–6**: add a line to `LEVELS` and a `zh-hsk3.source.tsv`; the coverage
  check does the rest. 290 characters in HSK 3, 449 in HSK 4, 621 in HSK 5, 977
  in HSK 6 (rows before de-duplication).
* **Japanese**: the same component table carries over — kanji use the same
  shapes. A `ja-grade1.source.tsv` over the 80 kyōiku grade 1 kanji, with
  KANJIDIC2 glosses and the IDS decompositions the kanji artifact already holds,
  would need only the enrichment step changed.
* **German**: the `sound`/`mnemonic` pair is per-language by nature. A parallel
  `mnemonic_de` field (or a sibling file) is cheap to add; a native speaker's
  hooks will beat translated ones.
