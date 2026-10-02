# A Japanese sibling for Hanzi Tutor: "Kana Tutor" / "Kanji Tutor" — feasibility and data-licence report

Research date: **2026-10-02**. Every licence claim below was read from a primary source — the
project's own `LICENSE`/`COPYING`, the rights-holder's licence page, a Creative Commons or GNU
deed — or from a live download of the dataset itself. Claims that could not be confirmed that way
are marked **UNVERIFIED** rather than smoothed over. Quantities, byte sizes and field lists were
measured against downloaded files, not quoted from documentation.

Target use: an **AGPL-3.0-only** offline app, distributed as a desktop bundle **and** to Google
Play / the App Store, bundling all data on-device.

---

## 1. Bottom line

**Feasible — more so than expected, and for a structural reason: Hanzi Tutor is not really a
Chinese app. It is a CJK stroke-tutoring engine that currently happens to have a Chinese data
layer.** The expensive parts — geometric stroke grading, the SQLite store, cross-device sync,
microphone capture, pitch tracking, the Tauri/Svelte shell — are language-neutral, and they were
already factored across crates once for the standalone tone trainer. `hanzi-core::geom`,
`raster`, `grade` and `time` have **zero** Chinese coupling; the only real coupling anywhere is
`vocab.rs` and `settings.rs`.

**The data question answers yes, cleanly, on almost every axis**, because the Japanese stack turns
out to use *the same licences Hanzi Tutor already ships and already documents*:

| What Hanzi Tutor ships today | Its licence | The Japanese equivalent | Same licence? |
| --- | --- | --- | --- |
| Make Me a Hanzi `graphics.txt` (stroke outlines + medians) | Arphic Public License | **AnimCJK `graphicsJa.txt`** (7,007 kanji, outlines + medians) | **Identical licence** |
| Make Me a Hanzi `dictionary.txt` | LGPL-3.0-or-later | **AnimCJK `dictionaryJa.txt`** (7,184 entries: radicals, IDS, school grades) | **Identical licence** |
| CC-CEDICT word readings and definitions | CC BY-SA 4.0 | **KANJIDIC2 + JMdict** (EDRDG) | **Identical licence** |
| hanziDB.csv frequency rank | MIT | KANJIDIC2 `freq` / scriptin `kanji-frequency` | CC BY-SA 4.0 / CC BY 4.0 |
| complete-hsk-vocabulary (graded word list) | MIT | *(no clean equivalent — see §6.4; derive your own)* | — |
| Noto Sans SC | SIL OFL 1.1 | Noto Sans JP / BIZ UD Gothic / Klee One | **Identical licence** |

Every license obligation a Japanese app would incur is one this repository **already satisfies and
already has a process for** — the `licences/` directory, the catalogue in `src-tauri/src/licences.rs`,
the "About and licences" screen, and the test that fails if a notice is half-added.

The build tooling is ready too: `scripts/build-release.sh` is already parameterised by `TAURI_ROOT`
for multiple apps, and each app carries its own `com.hanzitutor.*` identifier.

**Three things are genuinely hard, and none of them is the kanji.**

1. **Pitch accent has no repository-safe lexicon.** This is the one place where the answer is an
   emphatic *no*. Kanjium's `accents.txt` carries a CC BY-SA 4.0 file, and the author states in
   issue #13 that its source is *withheld* "due to potential copyright issues"; joining it against
   Wadoku's dump shows **88.9% of pairs identical including list order**. The licence grant is not
   the licensor's to give. Wadoku and OJAD are openly non-free. The clean route is to **generate**
   accent patterns (tdmelodic, BSD-3-Clause) rather than license them. See §6.1.
2. **There is no open kanji etymology source.** Hanzi Tutor's etymology hints come free with Make Me
   a Hanzi (LGPL). Japanese has no equivalent; the usual imagery is © Richard Sears. The feature
   cannot be shipped without original prose or a proprietary source. A genuine capability
   regression versus the Chinese app. See §6.3.
3. **Mnemonics cannot be copied.** Heisig, WaniKani and Kanji Koohii's community stories are all
   off-limits (§6.2). Mnemonics must be authored.

**Recommendation: one app, not two, and kana first.** Kana are the gateway to everything else —
okurigana, particles and every inflectional ending are kana — and no open-source project does kana
and kanji in one coherent progression with shared SRS state (§8). Ship it as a third app in this
workspace, `apps/nihongo-tutor/`, sharing the neutral crates.

---

## 2. What the engine actually gives you, for free

Measured against the working tree (Rust source lines, excluding tests where noted).

### 2.1 Genuinely language-neutral — liftable as-is

| Module | Lines | Internal deps | Chinese mentions |
| --- | --- | --- | --- |
| `hanzi-core::geom` | 450 | none | 0 (comments only) |
| `hanzi-core::raster` | 624 | `geom` | 0 |
| `hanzi-core::grade` | 1,695 | `geom`, `raster` | 0 |
| `hanzi-core::time` | 311 | none | 0 |

That is the crown jewel — **3,080 lines with zero Chinese coupling**: Hungarian-algorithm stroke
pairing, Kendall-tau stroke-order scoring, the rasterised ink measures, and the coordinate
transforms. `grade.rs` not depending on the dataset at all is what makes it portable: it grades
*geometry against geometry*, and AnimCJK's Japanese geometry is the same shape of data (§4).

### 2.2 Reusable after a shallow refactor

| Module / crate | Lines | What it needs |
| --- | --- | --- |
| `hanzi-core::progress` | 2,000 | Its only coupling is `vocab::{Entry, EntryProgress, EntryStanding}` — the personal word-list types, which are Chinese-agnostic in shape but live in the Chinese module |
| `hanzi-core::settings` | 977 | Mostly generic; names HSK levels where it should name "levels" |
| `hanzi-core::curriculum` | 115 | Frequency list → lessons. Generic already |
| `hanzi-store` | 3,826 | Depends on `hanzi_core::{Attempt, CardState, Rating, Sm2, fold_attempts, ProgressSink, VocabSink, Settings}` — all neutral except `vocab::Document` |
| `hanzi-sync` | 4,475 | Depends **only** on neutral progress types (`CardState`, `ProgressSink`, `fold_attempts`, `Rating`, `Sm2`). Reusable essentially untouched |
| `hanzi-voice` | 2,229 | Mic capture + system TTS. Language-free |
| `hanzi-hearing` | 1,053 | Download/verify/run the ASR model. Needs a Japanese model instead of the Chinese one |
| `tone.rs` §"Pitch tracking" + §"Contours" | ~600–900 | Pure DSP: `resample`, `track_pitch`, `contour`, `PitchTrack`, `dtw_distance`. Reusable verbatim for pitch accent |
| Frontend (`PracticeCanvas`, `render.ts`, `FeedbackPanel`, `LessonSidebar`, `VocabularyPanel`, `SettingsPanel`, `due.ts`, `LicencesPanel`) | ~10,000 of 17,179 | Canvas rendering, stroke sweep, verdict colours, scheduling UI |

The Rust core is **~36,000 lines**; roughly **19,000 of it is reusable**, and it is the hard 19,000.
`hanzi-sync` being dependent only on neutral types is a notable result — cross-device sync for a
Japanese app costs nothing.

### 2.3 What must be built new

| New work | Est. lines | Note |
| --- | --- | --- |
| `nihongo-core` data layer (kana + kanji, readings, okurigana, grades) | 2,000–2,500 | `Character` maps over almost 1:1 — see §3 |
| `prepare-data` for Japanese | 600–900 | Existing pipeline is only 626 lines and its `GraphicsLine` parser matches AnimCJK exactly (§4) |
| Romaji / kana input engine | 300–500 | Or reuse WanaKana (MIT) |
| Kana pedagogy (dakuten/yōon tables, confusable pairs) | 500–800 | §5 |
| Pitch-accent scoring (accent patterns on top of the reusable DSP) | 500–800 | §6.1 |
| Furigana + morphological analysis | 400–700 | Needs a tokeniser (§7) |
| Frontend Japanese components | 2,000–3,000 | Kana chart, furigana, pitch contour |
| **Total new** | **≈6,000–8,000** | Against ~19,000 reused |

For scale: the standalone tone trainer already proved this pattern works — it lifted
`hanzi-core::tone`, `hanzi-voice` and `hanzi-hearing` and shared *no* interface with the main app.

---

## 3. The data model maps over almost unchanged

`hanzi-core::dataset::Character` is nearly language-neutral already:

```rust
pub struct Character {
    pub ch: char,
    pub rank: u32,          // frequency rank
    pub hsk: u8,            // level  →  Japanese: kyōiku grade, then jōyō band
    pub stroke_count: u8,
    pub radical: char,
    pub pinyin: Vec<String>,  //        →  on'yomi + kun'yomi (with okurigana marks)
    pub definition: String,
    pub etymology: String,    //        →  no source; see §6.3
    pub decomposition: String,// IDS    →  AnimCJK also emits IDS; parser unchanged
    pub outlines: Vec<String>,// SVG, font space
    pub medians: Vec<Vec<Point>>, // display space, used for grading
}
```

The two geometry fields are exactly what AnimCJK provides, and `decompose.rs`'s IDS parser reads
AnimCJK's `decomposition` field unchanged because it is a standard IDS string
(`⿱丶丶`, `⿰丿𠃌`). KANJIDIC2 supplies readings, meanings, `grade`, `freq`, `stroke_count` and
`radical` field-for-field (§4.2).

---

## 4. Geometry: AnimCJK is a drop-in, and it is the only source that will do

This is the single most useful technical finding, and it was verified by downloading and parsing
the files.

### 4.1 Kanji geometry is in Make Me a Hanzi's own coordinate space

`graphicsJa.txt` — **21,862,957 bytes, 7,007 entries**, each
`{"character", "strokes", "medians"}` — the exact JSON-lines shape of Make Me a Hanzi's
`graphics.txt`, which is the shape `prepare_data.rs`'s `GraphicsLine` already deserialises:

```rust
struct GraphicsLine { character: String, strokes: Vec<String>, medians: Vec<Vec<[f32; 2]>> }
```

Measured over all 7,007 entries:

| Property | AnimCJK kanji | Make Me a Hanzi box |
| --- | --- | --- |
| x range (medians) | 22 … 1012 | 0 … 1024 |
| y range (medians) | −108 … 884 | −124 … 900 |
| entries where `strokes.len() != medians.len()` | **0** | — |
| entries with any out-of-box median | **0** | — |

So the kanji are in **Make Me a Hanzi's font space, including the y-up convention** — confirmed
independently by a known-asymmetric test: for 二, the short top stroke has mean y = 599 and the
long bottom stroke mean y = 203, i.e. *larger y is higher*, exactly as the existing
`Point::from_font` and the `scale(1,−1) translate(0,−900)` render transform assume.

**Practical consequence: the geometry half of `prepare-data` needs no changes at all.** Point it at
`graphicsJa.txt`, and the existing space conversion, resampling and grading constants apply. That
`selfcheck` measurement work (`grade.rs`'s tolerances were fitted against the real Chinese dataset)
carries over, though it should be re-run against Japanese geometry.

### 4.2 `dictionaryJa.txt` gives the curriculum for free

**1,356,219 bytes, 7,184 entries**, LGPL-3.0-or-later, one JSON object per line:
`{"character", "set", "radical", "decomposition", "acjk"}`. The `set` field encodes the Japanese
school grades directly:

| `set` | Count | What it is |
| --- | --- | --- |
| `g1` | 80 | kyōiku grade 1 (the official 80) |
| `g2` | 160 | kyōiku grade 2 (the official 160) |
| `g3` | 200 | kyōiku grade 3 (the official 200) |
| `g4` | 200 | kyōiku grade 4 — **the pre-2017 split; see below** |
| `g5` | 185 | kyōiku grade 5 — pre-2017 |
| `g6` | 181 | kyōiku grade 6 — pre-2017 |
| `g7` | 1,130 | jōyō remainder — **g1…g7 sums to exactly 2,136, the jōyō count** |
| `g8` | 863 | jinmeiyō |
| `g9` | 3,783 | hyōgai / other |
| `gc` / `radical` / `stroke` | 187 / 214 / 38 | components, the 214 Kangxi radicals, the 38 basic strokes |
| `hiragana` / `katakana` | 86 / 91 | the kana |

It also carries **6,948 IDS decompositions** and the **214 Kangxi radicals** with a Japanese
component in parentheses (e.g. `"radical": "⺀ (⼎)"`). So the course ordering, the radicals panel and
the decomposition panel all come from one permissively-licensed file — and unlike a frequency list,
the kyōiku grade order is *the order Japanese children actually learn them in*, which is better
pedagogy than a corpus ranking. **No Japanese equivalent of the MIT HSK word list is needed for
kanji ordering.**

**Grade reconciliation — AnimCJK's grades are the pre-2017 split; use KANJIDIC2's `grade` as the
authority.** Measured over the two files, the difference is exactly accounted for:

| | g1 | g2 | g3 | g4 | g5 | g6 | **1–6** | remainder | **jōyō total** |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AnimCJK `dictionaryJa.txt` | 80 | 160 | 200 | 200 | 185 | 181 | **1,006** | g7 = 1,130 | **2,136** |
| KANJIDIC2 `grade` | 80 | 160 | 200 | **202** | **193** | **191** | **1,026** | grade 8 = 1,110 | **2,136** |

The 20-character difference is exactly the prefecture-name kanji added to kyōiku in 2017 (茨 媛 岡
潟 岐 熊 香 佐 埼 崎 滋 鹿 縄 井 沖 栃 奈 梨 阪 阜). Both files agree the jōyō total is 2,136, and both
agree jinmeiyō is 863 (KANJIDIC2 splits it `9` = 651 + `10` = 212). **So: take the curriculum ladder
and the geometry/decomposition/radical linkage from `dictionaryJa.txt`, but take the *current* grade
assignment from KANJIDIC2's `grade` field**, or the app will teach 2016's curriculum.

### 4.3 The kana file has two defects that must be handled

`graphicsJaKana.txt` — **202,547 bytes, 177 entries** (86 hiragana + 91 katakana),
**LGPL-3.0-or-later**. The outlines are clean and in the same box (aggregate x 35…1012,
y −90…877), but:

**Defect 1 — 25 of 177 kana store one taught stroke as two array entries.** Because AnimCJK
animates a self-overlapping stroke by splitting it, a kana is stored with more "strokes" than are
taught. あ is stored as **4** strokes and 4 medians; it is taught as **3**. The affected set is
exactly: ぁ あ ぉ お す ず な ぬ ね の は ば ぱ ほ ぼ ぽ ま み む め ょ よ る ゐ ゑ — 27 segments in
total, since ぬ and ゐ are split in two places. The extra entries' medians are **displaced copies of
the same centre-line** (あ's first three points are shifted to x = −210…845) so that they do not
draw, while their outlines are genuine complementary halves of the shape.

> **Correction.** This section originally said 21, from a heuristic that looked for medians landing
> outside the design box. That test misses す, ず, ね and の, whose displaced medians stay inside it.
> The authoritative count comes from AnimCJK's SVG element ids, which name the stroke each segment
> belongs to, and it is **25**. The implementation found this; see
> `crates/nihongo-core/src/kana.rs`.

Left unfixed this **mis-grades handwriting**: the grader would look for four reference strokes and
flag a correct three-stroke あ as missing a stroke, and the Kendall order score would be wrong. The
fix is mechanical and authoritative — the SVG ids encode the grouping (`z12354d3a`, `z12354d3b` →
taught stroke 3), so the importer groups segments by stroke index rather than trusting the array
length. `dictionaryJa.txt` carries no stroke count for kana, so the SVG ids are the source of truth,
and **KanjiVG is the independent check on the answer** — a different project's per-stroke paths say
how many strokes each kana has, and all 177 must agree before an artifact is written.

**Defect 2 — the file's own licence is ambiguous.** AnimCJK's `COPYING.txt` assigns *"text files
prefixed by `graphics`"* to the Arphic Public License, while explicitly putting *"SVG files …
representing kana or strokes"* under LGPL-3.0-or-later. `graphicsJaKana.txt` is a `graphics*` file
whose content is kana. Both licences are usable, but for the kana the **SVG files under
`svgsJaKana/` carry explicit per-file LGPL headers** and are the better-attested source. Worth an
upstream issue.

### 4.4 KanjiVG cannot replace AnimCJK, because it has no ink

KanjiVG is the obvious-looking alternative (`kanji/03042.svg` is あ, verified), and it is CC BY-SA
3.0. But its stroke paths are **open centre-lines**, not filled outlines — no `Z`, control points
within ~2 units — and the repository contains **no "median" data at all**. Hanzi Tutor's grading
needs *both*:

* `medians` for shape and placement scoring, and
* `outlines` for the faint guide the canvas draws **and** for the "character's own ink" that
  `inkScore` compares against.

KanjiVG gives the first by conversion and not the second, so using it would mean synthesising
outlines by stroking the centre-lines. **AnimCJK is the only source that supplies both in one
file** — which is why it, and not KanjiVG, is the recommendation. (KanjiVG remains valuable as an
independent **stroke-order cross-check**, which is exactly how AnimCJK's own author treats it.)

### 4.5 The Arphic Public License question — pre-existing, not new

One research stream concluded that AnimCJK's kanji SVGs are "AGPL-incompatible" and should be
avoided, on the grounds that the FSF lists the Arphic Public License as copyleft and GPL-incompatible.
**That FSF classification is real** — the licence list says, verbatim: *"This is a copyleft free
software license, incompatible with the GPL. Its normal use is for fonts, and in that use, the
incompatibility does not cause a problem."*

But the conclusion drawn from it is wrong in this context, for a decisive reason: **Hanzi Tutor
already ships Arphic-licensed data.** Make Me a Hanzi's `graphics.txt` is Arphic PL, it is recorded
in `LICENSES.md`, its full text ships as `licences/Arphic-Public-License.txt`, and the app is
published on Google Play. AnimCJK's kanji files carry the **identical** licence.

So this is not a Japan-specific blocker. It is a **pre-existing, already-adjudicated position** in
this repository, and `LICENSES.md` records the reading explicitly: the Arphic Public License is
treated as *"a permissive free-font licence rather than a copyleft one"* that *"does carry notice
obligations"*, and the project's stated rule is satisfied because the shipped set — *"Apache-2.0,
MIT, the Arphic Public License, CC BY-SA 4.0, the SIL OFL, public domain, and the GPL-3.0-or-later
that §13 permits"* — is a set AGPL-3.0-only can live with.

**What a careful reader should take from this:** the Japanese kanji path inherits an obligation
that is already documented, already satisfied, and already shipping. If the Arphic position is ever
revisited, it must be revisited for *both* apps at once — and note that the strictly FSF-conservative
reading would require treating the Arphic data as a separately-licensed component rather than
merged into a single combined work, which is a larger question than this report. Everything here is
consistent with `LICENSES.md` as written; nothing here is legal advice.

---

## 5. Kana: the first app, and the smaller half

**Scope is larger than it looks.** 46 base kana + 25 dakuten/handakuten + 33 yōon digraphs ≈ 104
mora **per script**, so ~208 glyphs before a single kanji. That is a complete beginner course on its
own, and it is the honest reason to ship kana as the *entry point* rather than a warm-up.

**Stroke data:** AnimCJK `svgsJaKana/` (LGPL-3.0-or-later) with the §4.3 merge fix. The existing
grading engine then works as-is.

**Mnemonics — openly licensed sets do exist, and this is a pleasant surprise:**

| Source | Coverage | Licence | Note |
| --- | --- | --- | --- |
| Commons `Category:Hiragana mnemonics` (User:Ikou) | ~47 hiragana | **CC0** | Zero obligations. The best base. Verify each file |
| Commons `Category:Katakana mnemonics` (same author) | katakana | **CC BY-SA 3.0** | Ship verbatim with attribution; adaptations stay 3.0 |
| `File:Japanese Kana Mnemonic Chart.png` | both scripts, one chart | **CC BY-SA 4.0** | 4.0, so the GPLv3 bridge exists |
| Wikibooks `Japanese/Kana` | tables, romaji, yōon/sokuon — **no mnemonics** | CC BY-SA 4.0 + GFDL | Reference text only |

Nothing exists for dakuten/handakuten/yōon, so those must be authored — unencumbered original work.

**Romaji input is a real subsystem.** Learners meet Hepburn (`shi`, `tsu`, `fu`, `ji`), Kunrei-shiki
(`si`, `tu`, `hu`, `zi` — which matches the gojūon grid and is pedagogically attractive) and wāpuro
(IME keystrokes). The hard cases are gemination (`katta` → かった, `xtu`), ん disambiguation
(`shinbun` vs `shimbun`, `n'` before vowels), long vowels, and ぢ/づ vs じ/ず. **WanaKana (MIT)** already
implements most of this and is the highest-ROI reuse target in the project. Note the naming trap:
WanaKana is MIT even though WaniKani's *content* is proprietary.

**Confusable pairs must be a first-class feature, not an edge case** — シ/ツ, ソ/ン, ね/れ/わ, る/ろ,
は/ほ, ぬ/め, ク/ワ/フ, チ/テ. They are distinguished by stroke **direction and curvature**, not
topology, which makes them a natural per-learner confusability matrix and a good SRS "leech" signal
with no Hanzi Tutor analogue. (The confusability literature is real but largely Japanese-language;
the pair list is practitioner consensus rather than a citable benchmark.)

**Handwriting tolerance matters more than for kanji, not less.** き and さ are taught as 3 strokes
but are very commonly handwritten connected as 2; ふ, そ, な and む have well-known variants. **A kana
tutor that rejects a legitimate handwriting variant is worse than no tutor.** The grading tolerances
should be looser for kana than for kanji even though the shapes are simpler.

**Audio: no work needed on the desktop.** Verified on this machine — macOS ships Japanese system
voices (`Kyoko`, `Eddy`, and others, `ja_JP`), which is the same `hanzi-voice::speech` path the app
already uses. For bundled clips, **MeloTTS-Japanese is MIT for both code and weights** (verified
from the GitHub API and the HF model card), so generating the ~120 kana clips at build time is
licence-clean. Commons has only ~10 isolated kana clips and Lingua Libre's Japanese recordings are
**per-file** CC0 or CC BY-SA 4.0 — usable with a licence manifest, not as a bulk assumption.

---

## 6. The three genuine gaps

### 6.1 Pitch accent — the one real "no", and how to route around it

**No high-coverage pitch-accent lexicon can safely be bundled.** This is the sharpest finding in
the report and it contradicts the received wisdom.

**Kanjium is not safe.** Its `LICENSE.txt` is the genuine, complete CC BY-SA 4.0 legal code
(18,131 bytes), and the README says everything is CC BY-SA 4.0 and free for commercial use — which
is why blogs and Reddit threads say it is fine. But three verified facts defeat the grant:

1. **The author says so himself.** In issue #13 (`mifunetoshiro`, 2020-09-20) the accent source is
   described as *not listed* "due to potential copyright issues", from "2-3 legitimate sources".
   Issue #15 clarifies that the README's credit to "Uros O." covers only the *CSS/arrow notation*,
   not the data.
2. **The data tracks commercial dictionaries.** Issues #18/#20 correct entries against 大辞林 and
   新明解日本語アクセント辞典, and describe `accents.txt` as a raw file with "countless mistakes and
   typos".
3. **An independent join confirms derivation.** Joining `accents.txt` (124,137 lines / 118,011 unique
   words, TSV `word ⇥ reading ⇥ accent-mora-list`, verified by download) against Wadoku's full XML
   dump (446,501 entries, 289,531 `<accent>` values) gives: **79,092 of 88,997 pairs identical
   *including list order* (88.9%)**, ~89.3% identical as a set, and **98.3% agreeing on at least one
   accent value**. Data from ~2014–2021 matching a 2026 Wadoku dump that closely is overwhelming
   evidence of a shared — almost certainly commercial — ancestor.

A CC BY-SA grant is only valid if the grantor holds the rights. Here the grantor's own issue tracker
says he does not. **Do not bundle `accents.txt`.**

The alternatives are worse: **Wadoku** is a custom non-free licence (§5.7 requires annual written
permission from Ulrich Apel *and* Wadoku e.V. for anything beyond unmodified no-fee distribution);
**OJAD** and its Suzuki-kun prosody tutor ask users to "refrain from using this site in for-profit
ventures"; **NHK日本語発音アクセント新辞典** and **新明解** are proprietary; **CSJ** is paid and
non-commercial; **MULTEXT-J** is sold through ELRA. `accentjiten.com` ships a 3.3 MB data file with
**no licence at all**.

**The route around it — generate the accent patterns instead of licensing them.** Two licence-clean
generators:

* **tdmelodic** (`PKSHATechnology-Research/tdmelodic`) — **BSD-3-Clause**, verified from `LICENSE`.
  A neural generator producing a Tokyo dialect accent dictionary from surface + reading, at
  NEologd/IPADIC scale (Tachibana & Katayama, ICASSP 2020).
* **UniDic-CWJ 2.1.2 `unidic-mecab_kana-accent`** — NINJAL's own page states verbatim: *"Ver.2.x
  onwards … are now completely free software. As they fall under the GPLv2.0 / LGPLv2.1 / BSD New
  triple license, you may also freely use them for business purposes."* The BSD option is
  AGPL-compatible. Mainline UniDic has no accent column; the `kana-accent` variant does.

**Before relying on either, verify two things:** the licence/provenance of tdmelodic's **model
weights** (the repo ships none), and which labels the accents were trained on. If the labels trace to
UniDic's `kana-accent` column the chain is clean; if they trace to NHK or 新明解, fall back to the
UniDic column directly, or to rules plus a model trained on UniDic labels. This is the single most
important open verification item in the whole plan.

**Design the scoring so it degrades gracefully**, because it does not all need a lexicon:

| Layer | Needs a lexicon? | How |
| --- | --- | --- |
| Mora timing/length, devoiced vowels, pitch range, "exactly one clear drop", fluency | **No** | Learner F0 + forced alignment |
| Downstep and contour *shape* across a phrase | **No** — needs accent-phrase structure, which can be *predicted* | tdmelodic / Open JTalk labels |
| Absolute per-word correctness (箸 vs 橋 vs 端) | **Yes** — but only for curriculum words | Generated targets |

Reuse `tone.rs`'s `track_pitch`, `contour` and `dtw_distance` for the acoustic layers; that is
genuinely language-neutral DSP. Note two things Hanzi Tutor's tone display does not prepare you for:
Japanese accent is **per-word, not per-syllable**, and heiban and odaka are **identical inside the
word** — they differ only in the following particle's pitch. Condensed to one line: **you can build a
good pitch-accent feature from generated data; you cannot bundle a good pitch-accent lexicon.**

### 6.2 Mnemonics and keywords — the traps

| Source | Status |
| --- | --- |
| Heisig *Remembering the Kanji* / *Remembering the Kana* stories | Copyrighted book text. Kanji Koohii's own help page says the RTK stories "SHOULD NOT BE REPRODUCED" |
| RTK index numbers and keywords | Kanji Koohii's AGPL grant **explicitly excludes** them — permission was personal to Fabrice Denis and "does not extend to derived works". Note KANJIDIC2's `heisig`/`heisig6` fields hold only **index numbers**, which are facts and low-risk |
| Heisig keywords on EDRDG | Published with Heisig's approval but **no licence stated**, and not covered by the EDRDG licence. Needs written clarification |
| WaniKani radical names, mnemonics, level ordering | Proprietary Tofugu content. **Watch for WaniKani-derived fields inside MIT-licensed datasets** — e.g. `davidluzgouveia/kanji-data` is MIT packaging that includes WaniKani level and radical fields |
| Kanji Koohii community stories | Code is AGPL-3.0, but the shipped sample database of top-voted stories has **no content licence**, and community mnemonics are the category most likely to themselves infringe Heisig |
| Tofugu's written guides | All rights reserved (`/terms/` 404s). Their *audio*, separately, is CC BY-SA 4.0 |
| Anki decks | 5mdld's JLPT decks are **CC BY-NC 4.0**; most JLPT lists descend from tanos.co.uk, which asserts **no licence**; textbook-scoped decks (Genki, Tobira, Minna) reproduce copyrighted sequencing |

**Conclusion: write original mnemonics and original radical names, or use the CC0/CC BY-SA mnemonic
art in §5.** Budget this as real content cost, not a data-fetch. If a community mnemonic pool is
wanted later, it must be built with an explicit inbound=outbound licence (CC0 or CC BY-SA 4.0) from
day one.

### 6.3 Etymology — a capability regression

Hanzi Tutor shows an etymology hint that arrives free with Make Me a Hanzi (`dictionary.txt`,
LGPL-3.0-or-later). **Japanese has no equivalent.** Neither AnimCJK's `dictionaryJa.txt` nor
KANJIDIC2 nor Unihan carries etymology; the standard imagery is © Richard Sears
(chineseetymology.org / hanziyuan.net); Kanjium's `/origin/` images are the same and are inside a
package that otherwise claims CC BY-SA 4.0.

Options: author original etymology/component prose (unencumbered), use Wiktionary etymology sections
(CC BY-SA 4.0, but they trace *Chinese* origins and coverage is uneven), or link out rather than
embed. **The structural component decomposition from `dictionaryJa.txt` is not a substitute** — it
is structural, not historical, and it is what WaniKani-style radical learning actually uses.

---

### 6.4 JLPT labelling — do not ship a "JLPT list"

**The JLPT publishes no official kanji or vocabulary list.** KANJIDIC2's `jlpt` field carries only
the pre-2010 1–4 scheme, and EDRDG states outright that no official lists exist for the new levels.
Every "N5 list" in circulation is a community reconstruction.

The best-licensed candidate, `jamsinclair/open-anki-jlpt-decks`, is **MIT** and well-formed
(**8,131 data rows** across N1–N5, measured: N1 2,699 / N2 1,906 / N3 2,140 / N4 668 / N5 718;
columns `expression, reading, meaning, tags, guid`, with a `guid` column enabling stable updates).
But its README states the data came from `chyyran/jlpt-anki-decks`, which is
**`NOASSERTION` / "Other"** — no licence — and itself "based on tanos.co.uk", which asserts no
licence at all. This is precisely the shape of problem `LICENSES.md` already documents for CC-CEDICT:
*every third-party MIT/CC grant covers that licensor's contribution, not the rights-holder's.* The
MIT covers the compilation; it cannot launder the underlying list.

**Recommendation, matching how the repo already handles this class of problem:** derive your own
bands from licensed data and label them as your own approximation —

1. **kanji order** from `dictionaryJa.txt`'s kyōiku grades `g1…g6`, then `g7`, refined within each
   grade by KANJIDIC2 `freq` or `kanji-frequency`;
2. **vocabulary order** from JMdict's `nf01`–`nf48` bands plus the `news1`/`ichi1`/`spec1`/`gai1`
   priority tags;
3. map the result onto N5…N1 and say so in the UI.

That is defensible, reproducible, needs no third-party list, and is arguably better pedagogy than
JLPT order anyway — the grade ladder is the order Japanese children learn in.

### 6.5 Scheduler: switch to FSRS

The engine currently uses SM-2. **FSRS is free and open** — the algorithm is published science
(DSR model; Ye et al., KDD 2022 and IEEE TKDE 2023) with permissively licensed implementations, and
**`fsrs-rs` is BSD-3-Clause** and Rust-native with an included optimiser, so it drops into an
AGPL-3.0 workspace. Structurally it beats SM-2 for this app: it models stability and difficulty
separately rather than one ease factor, and it exposes **desired-retention targeting** — the exact
knob a learner wants.

One caveat that is easy to miss: FSRS's *default pretrained weights* were fitted on
`open-spaced-repetition/anki-revlogs-10k`, whose Hugging Face licence is tagged **`license: other`**.
The algorithm and code are safely open; shipping those exact parameters *as if openly licensed* is
not clearly safe. Optimise on-device from the user's own reviews, or derive and document your own
defaults. Keep SM-2 as a cold-start default until there is history to fit.

## 7. Supporting stack: what else is clean

| Need | Choice | Licence | Note |
| --- | --- | --- | --- |
| Kanji dictionary | **KANJIDIC2** via `scriptin/jmdict-simplified` JSON | CC BY-SA 4.0 | 13,108 entries, 1.49 MB gz. Verified fields: `grade`, `freq` (Mainichi rank), `stroke_count`, `radical`, `ja_on`, `ja_kun` **with okurigana markers**, `nanori`, meanings, `jlpt` (pre-2010 1–4 only), ~25 dictionary index sets |
| Vocabulary | **JMdict_e** via `jmdict-eng` JSON | CC BY-SA 4.0 | 218,849 entries, 10.6 MB gz. `ke_pri`/`re_pri` (`news1`, `ichi1`, `spec1`, `gai1`) and **`nf01`–`nf48`** give a fully-licensed difficulty/frequency proxy |
| Frequency | KANJIDIC2 `freq` + **`scriptin/kanji-frequency`** | CC BY-SA 4.0 / **CC BY 4.0** | Built from Aozora Bunko, Japanese Wikipedia, Wikinews |
| Radicals | `dictionaryJa.txt` (214, with Japanese component) + KANJIDIC2 `rad_name` (hiragana) + **Unicode character names** (`KANGXI RADICAL SUN`) | LGPL-3.0+ / CC BY-SA 4.0 / **Unicode License v3** | Unicode names are the best legal footing for English radical names |
| Decomposition | `dictionaryJa.txt` IDS (6,948) or CHISE IDS | LGPL-3.0+ / GPL-2.0-**or-later** | `cjk-decomp` is six-choose-one incl. Apache-2.0, but unmaintained |
| Curriculum banding | KANJIDIC2 `grade` + `freq` | CC BY-SA 4.0 | See §6.4 |
| Examples | **Tatoeba** (CC BY 2.0 FR, partly CC0; Tanaka-derived subset) | CC BY 2.0 FR | Filter per-sentence ND variants at import |
| Tokeniser (furigana) | **lindera** (MIT) / **vibrato** (MIT) / **jpreprocess** (BSD-3) / **UniDic** (triple, **pick BSD**) | permissive | Japanese has no whitespace; tokenisation is a hard dependency Hanzi Tutor never needed |
| Furigana alignment | **JmdictFurigana** | MIT | Aligns furigana to JMdict entries |
| Romaji/kana | **WanaKana** | MIT | The romaji input engine, essentially solved |
| Scheduling | **fsrs-rs** | BSD-3-Clause | See §6.5 |
| Fonts | **Noto Sans JP**, **BIZ UD Gothic/Mincho**, Klee One, Yusei Magic | OFL-1.1 | All redistributable and webfont-embeddable |
| Stroke-number font | **KanjiStrokeOrders** | **BSD-3-Clause** | The only open font that prints stroke-order numbers — good for printable sheets |
| TTS | macOS/Windows system voices; **MeloTTS-Japanese (MIT)** for build-time clips | MIT | Verified `ja_JP` system voices present |

**Fonts caveat — and a correction to a common claim.** There is **no OFL kyokasho-tai (教科書体)
font**. UD Digi Kyokasho and Yu Kyokasho are proprietary. Google's own description of **Klee One**
says it "sets itself apart from traditional script and textbook fonts", so the widely repeated claim
that Klee One *is* a textbook font is not supported by the primary source. For school letterforms
you must license a kyokasho face or draw your own — the UI font is cosmetic anyway, since the canvas
draws stored vector outlines.

---

## 8. Architecture and phasing

### 8.1 One app or two?

**One app.** Reasons:

* **The progression is the product.** Kana unlock grammar; kanji are learned *through* vocabulary.
  Splitting them produces a kana app that dead-ends and a kanji app that cannot teach reading —
  "I know 1,500 kanji and can't read a paragraph" is the defining Japanese failure mode. Kanji must
  be taught **with vocabulary**, not in isolation. This is also the honest gap in the market (§9).
* **Shared SRS state.** A learner's schedule, leech data and vocabulary list should not fork across
  two databases. `hanzi-store` and `hanzi-sync` are reusable as-is, so this is nearly free.
* **The repo already supports it.** Two apps exist; `TAURI_ROOT` and per-app `com.hanzitutor.*`
  identifiers are established.

Ship it as **`apps/nihongo-tutor/`**, reusing the neutral crates. Do *not* rename `hanzi-*` to
something generic as a prerequisite — a new crate can `use hanzi_core::grade` without a rename, and
given this project's invariants and handover discipline, the zero-refactor path is worth the
cosmetic oddity. Extract a neutral `script-core` only when a third consumer actually appears.

### 8.2 Phasing

| Phase | Content | Why this order |
| --- | --- | --- |
| **0. Verify** | tdmelodic weights + training-label provenance; the `kana-accent` column's licence; EDRDG §4 update procedure; confirm the Arphic position is formally recorded for the Japanese data too | These are the only items that can invalidate a phase |
| **1. Data pipeline** | `prepare-data` for Japanese: `graphicsJa.txt` + `graphicsJaKana.txt` (with the §4.3 stroke merge) + `dictionaryJa.txt` + KANJIDIC2 + JMdict + `kanji-frequency`. Emit one artifact, committed, as the Chinese one is | Everything downstream depends on it; the existing pipeline and `GraphicsLine` parser make this the cheapest phase |
| **2. Kana app** | Hiragana → katakana, stroke order, tracing with variant tolerance, dakuten/handakuten/yōon, confusable-pair drilling, romaji input, audio | A complete, shippable beginner product on ~200 glyphs, reusing the whole grading engine |
| **3. Kanji through vocabulary** | Kyōiku g1→g6 then jōyō g7; readings with okurigana; furigana via a tokeniser; a chosen, documented level ladder; radicals | The core product claim |
| **4. Pitch accent** | Mora-contour display + acoustic scoring; generated per-word targets for curriculum vocabulary only | Deliberately last, because it is the one thing that cannot be sourced today |
| **5. Provenance surface** | `licences/` entries, `licences.rs` catalogue, an About screen, and a documented data-refresh path | EDRDG *requires* an About/Sources screen and periodic updates — a start-up mention is explicitly not enough |

### 8.3 Non-negotiable licence mechanics

1. Each dataset in a clearly separated directory with its own `LICENSE`/`NOTICE`.
2. An in-app **"About and licences"** screen — EDRDG requires it for apps, and says a launch-page
   mention is insufficient. The existing `LicencesPanel.svelte` and the `licences.rs` catalogue test
   already do this; extend them.
3. **Implement and document an EDRDG data-refresh procedure.** §4 states that failure to keep the
   data updated "is a violation of the licence to use the data". A pinned artifact with no update
   path is the trap here.
4. Never claim copyright over EDRDG material; credit Jack Halpern and kanji.org for SKIP codes.
5. State modifications for all share-alike works.
6. Remember the asymmetry: **CC BY-SA 4.0 → GPLv3 bridging exists; CC BY-SA 3.0 has none.** KanjiVG
   adaptations stay CC BY-SA 3.0 and cannot be folded into the AGPL-licensed portion.
7. Never imply the bundled data is AGPL-3.0.

---

## 9. Market reality check

Worth stating plainly, because it affects how much to invest.

**The free tier is excellent and mostly open.** Anki (AGPL-3.0) + Yomitan (GPL-3.0) + a mining note
type is a zero-cost, largely offline community-standard workflow. Renshuu, Takoboto, jisho.org,
Kanji Koohii, kana-dojo (AGPL-3.0), jiten and the Genki study resources are free. WaniKani gives
three levels free and costs **$9/month, $89/year or $299 lifetime** (verified). Nobody is extracting
rents a free alternative can undercut.

**Structural headwinds.** Japanese learners study on **phones**, in short bursts; the polished
competitors are mobile-first, and desktop is where people *mine* and *immerse* rather than grind
reviews. An offline Tauri desktop app also faces macOS notarisation, Linux packaging, and no app
store — and AGPL-3.0 on the App Store is itself an awkward combination (Apple's DRM/ToS terms sit
badly with GPLv3-style installation requirements), a question Hanzi Tutor will meet independently.

**Where a real gap exists:**

1. **No-account, local-only, subscription-free, telemetry-free.** Every polished competitor with a
   good SRS *is a server*. "Everything local, nothing phones home" is a real constituency — and it
   is the same promise this project already makes.
2. **Coherence over assembly.** Today's open stack is a Rube Goldberg machine: Anki + Yomitan +
   dictionaries + a note type + a pitch add-on + a furigana add-on + an OCR tool. **Nobody
   open-source does kana and kanji in one coherent progression with shared SRS state** — that is the
   strongest single reason for one app rather than two.
3. **Principled, auditable provenance.** Every card can cite EDRDG / AnimCJK / CC BY-SA with the
   licence shown in-app. An AGPL project can do this; a commercial one structurally cannot. This is
   the differentiator that fits this repository's existing character best.
4. **Furigana and pitch accent as first-class and offline**, with audio, rather than requiring a
   browser extension wired to a running desktop Anki.
5. **On-device FSRS optimisation** with desired-retention control and no cloud account.

**Where it will lose, and should say so:** kana alone is a two-week problem that free sites already
solve; stroke order and SRS review are solved by KanjiVG + AnkiDroid; frequency ranking is solved by
jpdb and Jiten; grammar explanation is solved by free resources. It cannot out-polish WaniKani's
mnemonics — legally *cannot* copy them — and it cannot offer etymology comparable to Make Me a
Hanzi because no open equivalent exists.

**Honest positioning:** *the offline, account-free, openly-licensed Japanese tutor that does kana
through reading in one place, with correct provenance and FSRS* — aimed at the self-hosting and
anti-subscription niche, and as a forkable reference implementation. Not "the WaniKani killer."
Expect mobile to be the first feature request.

---

## 10. Open items to verify before committing

1. **tdmelodic**: licence and provenance of the model **weights**, and what the accent training
   labels derive from. **Blocks any pitch-accent feature.**
2. **UniDic-CWJ `unidic-mecab_kana-accent` 2.1.2**: whether the accent column is covered by the
   GPLv2 / LGPLv2.1 / BSD-New triple licence, and its own provenance.
3. **`graphicsJaKana.txt` licence ambiguity** — confirm with upstream, or use `svgsJaKana/*.svg`
   whose per-file LGPL headers are explicit.
4. ~~**The 21 kana stroke merges**~~ — **done, and the count was wrong: it is 25.** The SVG-id
   grouping rule does reproduce the taught stroke count, verified against KanjiVG for all 177, and
   `prepare-kana` now refuses to write an artifact it could not check. See
   `crates/nihongo-core/`. ぬ and ゐ are each split in two places, which the grouping handles because
   it is read from the ids rather than inferred from the number of entries.
5. **`cjkvi-ids`** "GPLv2": or-later vs v2-only. If v2-only it is AGPL-incompatible; prefer CHISE or
   `cjk-decomp`'s Apache-2.0 option.
6. **Sudachi/SudachiDict** and **MeCab/IPADic** licences — unverified; prefer UniDic's BSD option.
7. **EDRDG update procedure** — design the refresh path before shipping a pinned artifact.
8. **Tatoeba per-sentence ND filtering** and per-contributor audio licences at import time.
9. **Japanese ASR model** for pronunciation feedback — Hanzitutor's current model is a non-free
   Chinese one; a Japanese replacement's licence is unverified and would be a separately-downloaded
   model under the same "we don't redistribute the weights" pattern.
10. **Formally record the Arphic position for the Japanese geometry** in `LICENSES.md`, so the
    identical obligation is stated for both apps rather than inherited implicitly.

---

## 11. Verdict

**Yes — and the licence question, which was the reasonable thing to worry about, is the *easy* part.**
AnimCJK supplies kanji outlines *and* medians in Make Me a Hanzi's exact JSON format and coordinate
space, under the Arphic and LGPL notices this repository already ships; KANJIDIC2 and JMdict supply
readings, meanings, grades and frequency under the CC BY-SA 4.0 notice it already ships; the kyōiku
grades come free and give a better curriculum than a frequency list; the grading engine, store, sync,
microphone and pitch DSP are language-neutral and already factored; and the multi-app build tooling
exists.

The hard parts are **content, not licensing**: a pitch-accent feature must be *generated* rather than
licensed, mnemonics and radical names must be *authored*, and etymology cannot be offered at all.
Budget those as writing work and the project is a well-bounded extension of what already exists —
one new app in `apps/`, roughly 6,000–8,000 new lines against ~19,000 reused.
