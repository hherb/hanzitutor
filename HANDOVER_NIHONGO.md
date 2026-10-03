# Handover — the Japanese part

This is the Japanese counterpart to [`HANDOVER.md`](HANDOVER.md), and it is the
document that describes it in full. The repository root now points here: the
`README.md` names the Japanese part in its "Next steps", `HANDOVER.md` lists this
file in its "Read in this order", and `ROADMAP.md` says plainly that the Japanese
work is not on its milestones. Until recently none of those was true —
`grep -c 'nihongo\|kana'` over the three returned **0**, so the work was
unfindable from the root.

The two documents divide the same way the Chinese ones do. This file is what you
need to *work on it*: the build, the shape of the code, the invariants that must
not break, how to verify a change, and the traps that cost time. The roadmap is
what to build *next*.

**The invariants in §4 are numbered in their own series.** "Invariant 3" here is
not invariant 3 of `HANDOVER.md`. Where a Chinese invariant governs the Japanese
code too — the engine is shared — the text says which.

---

## 1. Get a working build first

Everything is committed, including the dataset artifact, so a clone builds
without fetching anything.

```bash
# From the repository root.
pnpm install                       # root: the shared scripts and the check tooling
pnpm --dir apps/nihongo-tutor install

# The engine, the kana data, and the shared speech crate the app speaks through.
./scripts/with-cargo-env.sh cargo test -p nihongo-core -p nihongo-tutor -p hanzi-voice

# The app, on a display.
pnpm --dir apps/nihongo-tutor run dev      # dev server on :1422, and the window
```

There is no data step. `crates/nihongo-core/data/kana.bin.gz` is committed
(70,917 bytes, 177 kana) precisely so that it does not need one — see invariant 1
— and so are `kanji.bin.gz` (3,236,713 bytes, 2,136 jōyō kanji, their geometry and
the 214 radical head forms — invariant 24) and `words.bin.gz`
(601,816 bytes, 16,073 words), which are the same decision made again for much
larger files (invariants 16 and 20). The vocabulary inputs are 118 MB of JSON,
63 MB of XML and 33 MB of furigana; none of it is needed to build or to test.

If you *do* need to regenerate them, that is the one path that fetches:

```bash
./scripts/fetch-data.sh        # also fetches the Chinese data; all are skipped if present
pnpm run prepare-kana
pnpm run prepare-kanji
pnpm run prepare-words         # needs the kanji artifact first — it decides what is teachable
./scripts/fetch-unidic.sh      # once only: 134 MB of UniDic, built into .lindera/
pnpm run prepare-passages      # needs the words artifact — it holds the passages to it
```

`fetch-unidic.sh` is separate from `fetch-data.sh` on purpose, and so is the
`tokenize` feature: the analyser is 100-odd crates and a 134 MB dictionary that only
the passage pipeline needs, and nothing that ships depends on it. lindera's build
script downloads **nothing** unless that script names a cache directory, so no
ordinary `cargo test` or `cargo clippy` ever touches the network.

`fetch-data.sh` pulls three things for the Japanese part, into `data/raw/`
(which is gitignored): `graphicsJaKana.txt`, 177 SVGs under `svgsJaKana/`, and
177 KanjiVG SVGs under `kvgJa/`. The last is **not** an input to the artifact —
it is the oracle the stroke counts are checked against, and without it
`prepare-kana` refuses to run. See §4.3.

For the kanji it pulls four more. `graphicsJa.txt` (7,007 characters of geometry)
and `dictionaryJa.txt` (the grade sets, radicals and IDS decompositions) come from
AnimCJK; `kanjidic2-all.json` — EDRDG's readings, glosses, grades and frequency,
pinned to one `scriptin/jmdict-simplified` release and unpacked from its `.tgz` —
is the authority for the curriculum; and the same `kvgJa/` directory is extended
to all 2,136 jōyō characters as the oracle. It does **not** fetch `svgsJa/`:
AnimCJK splits a kana stroke that crosses itself but not a kanji one, so the kanji
pipeline has no element ids to regroup anything, and that is measured rather than
assumed (invariant 17).

For the vocabulary it pulls three more: `jmdict-eng.json` (EDRDG's dictionary as
JSON, pinned with KANJIDIC2 by the same release tag) and `JMdict_e.gz` — the *same
dictionary* as XML, fetched because the JSON reformatting drops the `nf01`–`nf48`
priority tags outright (invariant 21) — plus `JmdictFurigana.json`, which aligns
each reading to the characters it belongs to. The kanji artifact `prepare-words`
also reads is a **committed** file rather than an upstream one, because it is what
decides which words are teachable.

### The root `Cargo.toml` is the only place the workspace is declared

`crates/nihongo-core` and `apps/nihongo-tutor/src-tauri` are members. Removing
either line silently drops the crate from every `cargo test --workspace` run
while leaving it on disk, which is a failure mode worth knowing about because
nothing complains.

---

## 2. What already works

Measured, not remembered. `cargo test -p nihongo-core -p nihongo-tutor` is **302
tests** — every `#[test]` in the two suites (193 in `nihongo-core`, 109 in
`nihongo-tutor`), plus one doc-test; the frontend's are counted separately below.
`pnpm run test:web` runs 142, of which **73** are this app's.
`cargo test -p hanzi-voice`, the shared crate the audio half lives in, is 29.

| | |
| --- | --- |
| Kana | 177 — 86 hiragana (U+3041–3096), 91 katakana (U+30A1–30FA plus ー) |
| Strokes | 516, average 2.92; あ is 3, not the 4 upstream stores (§4.2) |
| Course | 18 hiragana lessons, 20 katakana, 38 in all |
| Chart | 16 rows × 5 columns: 71 kana on the grid and **9 holes** the language leaves; 15 kana off it in hiragana, 20 in katakana (invariant 29) |
| Yōon | 33 digraphs per script, 11 bases × 3 — each one drilled against its own two-mora spelling, so きゃ is set against きや |
| Voicing | 25 contrasts per script — か/が, は/ば and は/ぱ — each drilled with the mark named |
| Confusions | 13 pairs, each with the one feature that tells them apart |
| Kanji | 2,136 jōyō — 1,026 kyōiku (grades 1–6) and 1,110 in the remainder (grade 8) |
| Kanji grades | 80/160/200/202/193/191 by grade, KANJIDIC2's **current** assignment |
| Kanji strokes | 22,367, average 10.47; 一 is 1 and 鬱 is 29 |
| Kanji readings | 2,854 on, 3,904 kun — 2,551 of them carrying okurigana and 364 affixes |
| Kanji course | 216 lessons of ten — kyōiku 1–6, then the remainder; the first is 日 一 人 年 大 十 二 本 中 出 |
| Radicals | 214 head forms with geometry, 1,222 strokes, 198 used by jōyō; 18 characters are classified under a different number than their note names |
| Kanji structure | 2,134 IDS decompositions, 2,037 frequency ranks |
| Kanji oracle | 2,135 of 2,136 agree with KanjiVG; 衷 is the one written exception (invariant 19) |
| Vocabulary | 16,073 words — every one with a kanji the board can draw, from EDRDG's common vocabulary (invariant 23) |
| Bands | 585 / 1,781 / 2,408 / 2,232 / 2,412 / 1,834 / 4,821 — this project's ladder, not the JLPT's (invariant 20) |
| Furigana | 16,022 of 16,073 aligned (99.7%); the other 51 carry their reading and no ruby |
| Passages | 3, written here — 37 tokens, 14 linked to a word, every kanji-bearing one taught (invariant 22) |
| Words on a card | 16,073 words over 2,136 characters — 一 in 223, 人 in 218, and **57 characters in none**; a page of 12, in course order (invariant 28) |
| Audio | the machine's own Japanese voice — `Kyoko (ja-JP)` here — behind a **Hear it** button, the `h` key, and a control on **every reading of a kanji card**; nothing bundled, nothing downloaded (invariants 26 and 27) |
| Screens | Practice, **Kana chart**, Tell them apart, **Review**, **Kanji**, **Radicals**, Words, Read, Licences |
| Tests | nihongo-core 193, nihongo-tutor 109, hanzi-voice 29, frontend 73 of the 142 |
| Artifact | kana 70,917, kanji 3,236,713, words 601,816 and passages 502 bytes — all gzip + magic + postcard, all four embedded with `include_bytes!` |
| Kanji format | version 2 (`KANJD002`): the 214-radical table sits between the characters and the source (invariant 24) |
| Learner data | two files, `confusions.json` and `review.json`, in the app's own data directory |
| Version | both crates 0.1.0, and the app's to match |

The app runs and has been looked at, all of it: the course, the board, stroke-order
animation, handwriting grading, the discrimination drill, the typing box, the
katakana tab, the Licences panel and the review queue have each been seen working
on a display, and so have the two kanji screens (below). The review queue has also
been *used* — a character drawn on it was graded and rescheduled, which is not the
same thing as being seen.

**And it has been heard.** N1 put the machine's own Japanese voice behind a button
and the `h` key, and behind **every reading on a kanji card** — so a character can
be met by ear or by hand, and the learner chooses. The check that mattered was
listening: あ from Practice, ひとつ for 一つ from a word card, た.べる read as
*たべる* from 食's card, a due あ offering the control where a due 学 offered none.
It was verified by probe first and then confirmed by ear by the maintainer, which
is the only half of it a test cannot reach — `speak` returning `Ok(())` says the
synthesiser accepted the utterance, not that anything came out of a speaker.

**Everything the app embeds is on screen now.** **Words** browses the ladder band
by band with furigana over every word and grades a typed reading; **Read** draws a
passage with a reading over each kanji and opens a word card when one is tapped;
**Kanji** is the character course — grades, lessons of ten, a grid, the board, and
a card carrying the readings, the glosses, the radical in both of its shapes, the
IDS components **and the words the character is written in**, each opening its own
word card; and **Radicals** shows all 214 head forms, each with the jōyō
characters classified under it, searchable and ordered by what a radical unlocks.
The kanji artifact was embedded and read by nothing for a whole milestone; N8 is
what read it, and the radical table it now carries is what makes the second screen
possible. Each milestone's own record says what its measurement corrected.

**And the last of it is the vocabulary on the character's card**, which §9 below had
called the obvious next thing after N8 — the thing `words::of_kanji` had been
waiting for. The card lists the words the course
teaches that use the character, in the course's own order — band, then EDRDG's
frequency — a page of twelve at a time, and tapping one opens the **word's own
card**, the same one `Words` and `Read` show, so the reading is the dictionary's
and never composed here. The numbers are measured: **一 is written in 223 of the
16,073 words and 人 in 218**, and **57 jōyō characters are in no word at all**,
which the card states in words rather than drawing an empty list. Invariant 28 is
the part that must not be broken. The live check drove it through a DOM probe,
because there was no display to capture: the Kanji tab came up on 日 with
`140 words this course teaches are written with 日 page 1 of 12`, twelve rows
beginning 日本人 にほんじん, a tap on that row opened the word card with its own
reading and band, `Next` turned to page 2 of 12, and 且 — found through the
course's own lesson list — read `No word this course teaches is written with 且.`

**The kana chart is the one screen that is not a lesson, and the drill now has five
exercises over one question shape.** The chart draws the gojūon grid row by row —
16 rows of five columns, with the nine slots the language never filled left as
**holes**, so ゆ sits under う and not under い — and the characters that are off
the grid at all: the small kana, the rare ones, and katakana's ヷ ヸ ヹ ヺ ー.
Tapping any of them opens it on the board, and it lands in its own lesson rather
than the course's first: the live check opened ゆ from the chart and found
`lesson=や ゆ よ` highlighted, then opened ヷ — which is on no grid — in `V-series`.
The drill's second kind of question is what a pair of unrelated kana cannot
express, and it has two forms: the **yōon contrast** (the digraph against the same
consonant and vowel written long — きゃ against きや, one mora against two) and the
**voicing contrast** (か against が, は against ば and ぱ — the mark a beginner
leaves off). It is one question shape, one weighting rule and one file
(invariant 29), because confusing きゃ with きや is a confusion like any other.
There was no display for this milestone either, so it was checked the way N8's
screens were — by probe over the dev server's log (§5) — which reported the grid,
the holes, both scripts, all five exercises, the yōon tell (*ビョ is byo, not ビヨ.
ビョ is one mora — the small ョ; ビヨ is two, ビ + ヨ*) and the voicing tell in both
of its forms (*て is the plain kana; で is the same kana with the dakuten ゛* and
*ホ is the plain kana; ポ is the same kana with the handakuten ゜*).

**Two parts of the app learn about their learner now.** The drill is **N3**'s: it
used to draw a kana from a pool and throw the answer away, and it now draws a
*pair*, weighted by how often this learner has missed it, and records what was
answered in `confusions.json`. The rule, the record and the reason it is not a
scheduler are in `nihongo_core::drill`.

**And N2 added the schedule, which is the other half and the reason the app is a
tutor rather than a drill.** Every character graded on a board is offered to
`review.json` — SM-2 from `hanzi_core::progress`, the same code the Chinese app
runs — and **Review** lists what is due, most overdue first. An attempt is a review
only when the character is new or due (invariant 25); the kana ride along on a
schedule the kanji course is what actually needs. Both files are this app's own,
and invariant 15 is the part that must not be undone: no other app reads either.

**Three of the surfaces were watched for the first time in the session before this
one, and two of them were broken.** The test suite was green for both, which is the
point of watching:

* **The Grade button could not grade anything.** The interface posts
  `{ inkWidth }` and nothing else — deliberately, because the other three tunables
  are the values the tolerance study was fitted against — and `GradeOptions` had
  only `ink_width` defaulted on deserialisation, so *every* attempt came back
  `invalid args 'options' for command 'grade_attempt': missing field 'resampleK'`.
  The verdict panel, the four scores and the stroke colours had therefore never
  been on screen. Fixed in `grade.rs` with a struct-level `#[serde(default)]`, and
  `ipc_contract.rs` now deserialises the options the way the command does, which
  is the half of the contract it was not checking. Invariant 14.
* **The board threw `effect_update_depth_exceeded` on mount and the window came up
  blank** — about half the cold starts, and every later reload looked fine, so it
  read as a flaky webview rather than a bug in the board. The reset effect wrote
  `strokes`/`ghostCount`/`sweep` and then called `paint()`, which reads them, so
  the effect depended on what it wrote. Trap 9.

**And the kanji screens were watched the same way, which found an interface bug
that no test could have.** A temporary probe in `index.html` (§5) drove the three
of them: a synthetic trace of 日's centre-lines graded **100/100** with all four
scores 1.00, a member of 人's family opened 会 on the board, and the card's "See the
family" link landed back on radical 9. One bug came out of it: the shared `.tabs`
rule capitalises its labels for `hiragana` and `katakana`, so the grade tabs
rendered as *Jōyō Beyond The School Grades* — this project's own ladder names,
title-cased into proper nouns, which `svelte-check` and every test were happy with.

### What is genuinely reusable, and was reused

The engine was not reimplemented, and that is the whole reason this was feasible:
`hanzi-core`'s `geom`, `raster` and `grade` grade strokes against strokes and
never look at the character, so a handwritten あ is judged by exactly the code
that judges 一. Measured when this was planned: `geom` (450 lines), `raster`
(624), `grade` (1,695) and `time` (311) have **zero** Chinese coupling, and
`hanzi-sync` depends only on language-neutral progress types.

`hanzi-store` and `hanzi-sync` are **not wired up yet** and are the obvious next
reuse — see `ROADMAP_NIHONGO.md` N2.

---

## 3. Where the code lives

```
crates/nihongo-core/              the data layer. No UI, no Tauri.
  src/kana.rs            (499)    Script, Kana, KanaDataset, the artifact format,
                                  segment_to_stroke and merge_strokes — the repair
  src/kanji.rs           (771)    Kanji, KanjiDataset, KanjiSource, parse_radical,
                                  Radical and RadicalFamily, and the modules'
                                  account of the grade reconciliation — the kanji
                                  half, N6, plus the 214-radical table, N8
  src/words.rs           (526)    Word, Ruby, WordDataset, the ladder's band_for
                                  and band_name, and `of_kanji` — the vocabulary
                                  half, N7, and the character card's list, invariant 28
  src/passages.rs        (387)    Passage, PassageToken, PassageDataset and the
                                  source-file parser — the reading half, N7
  src/readings.rs        (386)    Hepburn and Kunrei-shiki, and the enumeration
                                  hiragana_with_readings the input table is built from
  src/curriculum.rs     (1030)    the gojūon rows **as five-slot grids**, the
                                  lessons, yōon and the two-mora spellings they
                                  contrast with, the confusions, and the
                                  grade-ordered kanji course — N8, N4
  src/input.rs           (523)    romaji → kana, and the one-kana and whole-word checks
  src/drill.rs           (930)    the pairs the drill asks (the classic confusions,
                                  the yōon contrasts and the voicing contrasts), the
                                  weights, the tally, and the draw with its
                                  **spelling** keys — N3, N4
  src/review.rs          (322)    what a scheduled character is, the prompt beside
                                  it and the due queue — N2. The schedule itself is
                                  hanzi-core's; this is the Japanese half
  src/lib.rs              (88)    re-exports, including hanzi-core's grade, its
                                  decomposition parser and its SM-2 schedule
  src/bin/prepare_kana.rs(246)    AnimCJK + KanjiVG → the artifact
  src/bin/prepare_kanji.rs(821)   AnimCJK + KANJIDIC2 + KanjiVG → the artifact,
                                  including the one written oracle exception, the
                                  214-radical table and its two unnumbered forms
  src/bin/prepare_words.rs(565)   JMdict (JSON and XML) + JmdictFurigana + the
                                  committed kanji artifact → the words artifact
  src/bin/prepare_passages.rs(357) lindera + UniDic → the segmented passages.
                                  Needs `--features nihongo-core/tokenize` and
                                  scripts/fetch-unidic.sh to have run once
  data/kana.bin.gz                COMMITTED, 70,917 bytes
  data/kanji.bin.gz               COMMITTED, 3,236,713 bytes (format v2)
  data/words.bin.gz               COMMITTED, 601,816 bytes
  data/passages/*.txt             COMMITTED: the passage text and its glosses,
                                  which is what a person edits
  data/passages.bin.gz            COMMITTED, 502 bytes — the segmentation
  tests/kana_artifact.rs (261)    12 tests over the committed kana artifact
  tests/kanji_artifact.rs(841)    26 tests over the committed kanji artifact: the
                                  whole grade reconciliation, the 214-radical
                                  table and the course
  tests/words_artifact.rs(473)    17 tests over the committed words artifact,
                                  recomputing every band from the kanji one
  tests/passages_artifact.rs(247) 9 tests over the committed passages, including
                                  that they still match data/passages/*.txt

scripts/fetch-unidic.sh           fetches and builds the UniDic dictionary into
                                  .lindera/ (gitignored). 134 MB down, 190 MB
                                  built, and the only thing that needs either

apps/nihongo-tutor/               the app
  src-tauri/src/lib.rs  (2591)    AppState, the speaker, the chart and the 28
                                  commands, all thin
  src-tauri/src/store.rs (582)    the two files of this app's own: confusions.json
                                  (load, record, atomic write — N3) and review.json
                                  (the SM-2 schedule, and the rule for when an
                                  attempt is a review — N2)
  src-tauri/src/licences.rs(157)  the notice catalogue, with the text compiled in
  src-tauri/tests/ipc_contract.rs (1220)  41 tests locking the JSON the webview
                                  reads *and the arguments it posts* (invariant 14)
  src-tauri/tests/licences.rs     (246)  the three-way notice check
  src-tauri/licences/AnimCJK-COPYING.txt  the one notice specific to this app
  src/App.svelte         (539)    the nine views, the course, the board wiring, the
                                  one place the voice status is asked for — N1 —
                                  and the one `openKana` a screen calls to put a
                                  kana on the board — N4
  src/lib/KanaCanvas.svelte(248)  pointer capture and the animation frame. Draws
                                  any `Drawable`, so a kana, a kanji and a radical
                                  all come through it
  src/lib/ConfusionDrill.svelte(392)  the drill: five exercises, one question, one
                                  answer, remembered — N3, N4
  src/lib/KanaChart.svelte(234)   the gojūon grid with its holes, and the characters
                                  off it — N4
  src/lib/LicencesPanel.svelte(116)   the notices, fetched over IPC
  src/lib/ReviewPanel.svelte(346) what is due, and a board to write it on — N2
  src/lib/KanjiPanel.svelte(869)  the kanji course, the board and the card — N8,
                                  whose readings are each a control — N1, and which
                                  lists the words the character is written in — 28
  src/lib/RadicalsPanel.svelte(501)   the 214 head forms and their families — N8
  src/lib/VocabularyPanel.svelte(309) the ladder, a band's words, the card — N7
  src/lib/PassagePanel.svelte(265)  a passage with furigana and a tap on any word — N7
  src/lib/WordCard.svelte(265)      one word: its furigana, its reading, its band
  src/lib/SpeakButton.svelte(168)   the one "Hear it" control: a button for the
                                  three standalone callers, and a link for a
                                  reading on the kanji card — N1
  src/lib/kanji.ts       (138)    the course's and the radicals' arithmetic, and
                                  `spokenReading`, as pure functions — N8, N1
  src/lib/kanji.test.ts  (170)    run by the ROOT project's vitest
  src/lib/kana.ts         (71)    which lesson a kana belongs to, which kana a
                                  loaded course should open on, how wide the chart's
                                  grid is, and whether a drill answer is one kana — N4
  src/lib/kana.test.ts   (128)    run by the ROOT project's vitest
  src/lib/words.ts       (101)    furigana and paging arithmetic, as pure functions
  src/lib/words.test.ts  (131)    run by the ROOT project's vitest
  src/lib/review.ts       (89)    "2 days overdue" and "in 2 days", as pure
                                  functions — N2
  src/lib/review.test.ts (131)    run by the ROOT project's vitest
  src/lib/speech.ts      (123)    what can be heard, and which key asks for it, as
                                  pure functions — N1, and invariant 26's rules
  src/lib/speech.test.ts  (81)    run by the ROOT project's vitest
  src/lib/board.ts        (97)    the board's arithmetic, as pure functions
  src/lib/board.test.ts   (99)    run by the ROOT project's vitest
  src/lib/render.ts      (547)    LIFTED FROM THE CHINESE APP, UNCHANGED (§4.6)
  src/lib/types.ts       (547)    the IPC shapes, `Character = Drawable`, and the
                                  chart's slots as `(string | null)[]`
  src/lib/api.ts         (303)    one wrapper per command

crates/hanzi-voice/               SHARED with the two Chinese apps, and not this
                                  part's data layer
  src/speech.rs         (1644)    the system synthesiser and the voice rules — and
                                  the `Language` this app taught it (invariant 26)
  src/lib.rs              (48)    the re-exports; `capture`, the microphone half,
                                  is a default feature this app turns off
```

The line counts are the current tree's, and they were last checked while writing
invariants 26 and 27; a few of the earlier numbers in this table were stale by
then, which is worth knowing before treating one as a measurement.

`src-tauri/gen/schemas/` is committed, as it is for the other two apps.

---

## 3a. Working conventions

The two rules in `HANDOVER.md` §3a — **print long jobs rather than starting
them**, and **checkpoint long passes every item** — apply here unchanged. Nothing
in the Japanese part takes long enough to need either yet: regenerating the
artifact is a couple of seconds.

One thing that is specific to this part: **the window-capture recipe in §5 is how
you see whether the interface works**, and it is worth using rather than asking
someone to describe what they see. This part was developed without a display for
most of its life, and the two bugs that shipped — a rectangular board and a
heading behind the title bar — were both invisible to a green test suite and
obvious in one screenshot.

---

## 4. Invariants — do not break these

### 1. **The kana artifact is committed, and that is the opposite of the Chinese one.**

`crates/nihongo-core/data/kana.bin.gz` is in the repository. The Chinese artifact
is generated and gitignored (`HANDOVER.md` invariant 7). Do not "fix" this to
match: the Japanese one is 71 KB, and committing it means a clone builds with no
data step at all.

`src-tauri/build.rs` checks the file exists before `tauri_build::build()`, so a
missing artifact is a readable message rather than a cryptic `include_bytes!`
error.

### 2. **A kana is stored with the strokes it is *taught* with, and the grouping comes from AnimCJK's SVG element ids.**

This is the repair that took the most care and the one most likely to be undone
by accident.

AnimCJK animates a kana whose stroke crosses itself by splitting that stroke into
several drawing segments, and `graphicsJaKana.txt` stores one entry per
*segment*. So あ arrives with **four** strokes where three are taught. Taken at
face value the grader looks for a stroke the learner was never taught to draw,
flags a correct three-stroke あ as missing one, and scores the order wrongly.

**Twenty-five kana are affected, not twenty-one.** A first pass looked for
medians landing outside the design box and missed す, ず, ね and の, whose
displaced medians stay inside it. The authoritative answer comes from the SVG
ids — `z12354d3a` and `z12354d3b` are two segments of taught stroke 3 — read by
`segment_to_stroke`, and folded by `merge_strokes`:

* **outlines concatenate**; the segments are genuine complementary halves of the
  shape;
* **medians keep the first**; the later ones are copies displaced out of the box
  so that their dash animation does not show. Measured over all 25 affected
  groups, the first centre-line spans 73–96% of its stroke's outline width and
  92–98% of its height, which is what a centre-line inside a round-capped outline
  should do.

27 segments fold back. `ぬ` and `ゐ` are split in two places each, which the rule
handles because it is read from the ids rather than inferred from a count.

`tests/kana_artifact.rs` freezes the taught stroke count for all 25, so a
regression fails in CI without any upstream download.

### 3. **KanjiVG is the oracle, and `prepare-kana` refuses to write an artifact it could not check.**

"How many strokes is あ" must not be answered by the code being tested. KanjiVG
is a separate project whose per-stroke paths are an independent statement, and
every one of the **177** counts is checked against it before the artifact is
written. A mismatch aborts the build.

`--allow-unchecked` exists and must be passed *deliberately*; an artifact built
with it rests on this code's own assumptions and should not be committed.

KanjiVG is **not** redistributed and earns no notice: only a stroke *count* is
read, and a count is a fact about the language rather than a copy of the work.
`data/raw/kvgJa/` is gitignored.

### 4. **Kana outlines are in Make Me a Hanzi's font space, and no conversion changed.**

Verified rather than assumed, because getting it wrong would have flipped every
kana. Over all 7,007 kanji in `graphicsJa.txt` the medians span x 22–1012 and
y −108–884, against Make Me a Hanzi's box of x 0–1024, y −124–900, and there are
**zero** entries where `strokes.len() != medians.len()` and zero with an
out-of-box median. The y-up convention is confirmed independently by 二: the
short top stroke has the *larger* mean y (599 against 203).

So `Point::from_font(x, y) = (x, 900 − y)` is correct for Japanese as it stands,
and `render.ts`'s `scale(1, −1) translate(0, −900)` is correct for a kana.

**Kana outlines are in that box. The kana *medians* in the raw file are not** —
the displaced copies in §2 are the reason the raw file's aggregate looks wrong.
After the merge, all merged medians are inside 0–1024 on both axes, which the
artifact test asserts.

### 5. **The Japanese crates version themselves.**

`crates/nihongo-core` and `apps/nihongo-tutor` are `version = "0.1.0"`, **not**
`version.workspace = true`. This is not tidiness. While they inherited the
workspace version they could not be committed independently of a Hanzi Tutor
release: committing them on top of a 0.6.0 HEAD made the workspace 0.7.0, which
failed the existing tests asserting Cargo's version equals `tauri.conf.json`'s in
the main app and the tone trainer. A Hanzi Tutor release must not drag them
along.

The app has its own test for this: `the_app_info_and_the_bundle_config_agree`
checks `APP.version` against `tauri.conf.json`'s.

### 6. **`render.ts` is lifted from the Chinese app, and stays unchanged.**

`apps/nihongo-tutor/src/lib/render.ts` is byte-identical to
`src/lib/render.ts`. It paints SVG outlines in font space and centre-lines in
display space, and because AnimCJK publishes kana in Make Me a Hanzi's own frame
(§4), the transform is already right for a kana.

`types.ts` does **not** fork it. It declares the name the file was written with,
`export type Character`, and what that name is aliased to is this app's business:
it was `= Kana` until the kanji screens were built, and it is now `= Drawable` — a
structural shape with the four fields `render.ts` actually reads (`ch`,
`strokeCount`, `outlines`, `medians`). That is what lets one canvas draw a kana, a
kanji and one of the 214 radical head forms while the lifted file stays
byte-identical. If `render.ts` is ever changed here, change it in the Chinese app
too — or record why they diverge. This follows the pattern the repository already
uses between its apps: Tone Trainer lifted `ToneChart.svelte` and `transcript.ts`
the same way.

`KanaCanvas.svelte` and `board.ts` are new, not lifted: Hanzi Tutor's
`PracticeCanvas` carries tone-panel and verdict-display concerns this app does
not have. `KanaCanvas` keeps its name after growing a second and third caller, and
its prop is `character` rather than `kana` for the same reason.

### 7. **The board's geometry belongs to the stylesheet; the script only sets resolution.**

`.board` is `width: min(100%, 44vh); aspect-ratio: 1 / 1`, and the canvas is
`width: 100%; height: 100%` inside it. One `ResizeObserver` reads the canvas's
own rectangle and sets the backing store, and nothing else writes a size.

An earlier version had the stylesheet say `aspect-ratio: 1` **and** `max-height:
62vh`, while the script measured the board and wrote both a width and a height
from that measurement. A `max-height` clamps the height without narrowing the
box, so the board became a wide rectangle with the kana stretched across it, and
the canvas matched the distortion. **Do not reintroduce a max-height on the
board, and do not let the script compute a second dimension.**

### 8. **The course covers every kana of the script, including katakana's five extra.**

Katakana has five characters hiragana does not — ヷ ヸ ヹ ヺ and ー — and a course
assembled from the hiragana grid silently teaches 86 of katakana's 91. A test
caught exactly this during development.

`lessons()` therefore has a katakana-only branch, and
`the_course_covers_each_script_completely` asserts that the lessons teach
*every* kana the dataset holds, for both scripts. Removing a group fails it.

This matters beyond kana: the same trap is waiting for the kanji course.

### 9. **The romaji input table is derived from the readings, not written twice.**

`input.rs` builds its table from `readings::hiragana_with_readings()` and
`curriculum::yoon()`, so a spelling added to a reading becomes typeable with no
second edit. `the_table_covers_every_spelling_the_readings_offer` is the check.

Where two kana share a spelling — じ and ぢ are both `ji`, ず and づ both `zu`,
お and を both `o` — the **first in code-point order keeps it**, so the common
kana wins and the uncommon ones stay reachable as `di`, `du`, `wo`. The two
scripts share one table: each entry carries a hiragana and a katakana rendering,
because a hiragana-first pass would otherwise leave katakana entries unclaimed
and `to_kana_in(Katakana, "ka")` would return か.

### 10. **Script conversion is the 0x60 offset, and it does not cover everything.**

The hiragana and katakana blocks are the same 86 characters `0x60` apart, so
`to_katakana` and `to_hiragana` are an addition and a subtraction. The v-series
(ヷ ヸ ヹ ヺ) and ー have **no hiragana**, and both functions return `None`
rather than guessing; the input table carries them as their own entries, and
`Script::of('ー')` is `Katakana`.

### 11. **Small kana are reachable only through the `x`/`l` escapes.**

The small kana sit *before* their full-size counterparts in code-point order —
ぁ is U+3041 and あ is U+3042 — so a table that claims spellings in code-point
order has ぁ take `a` first and every vowel type small. `input.rs` skips
`SMALL_KANA` when claiming base readings; `xa` gives ぁ, `a` gives あ.

### 12. **The notices are app-local, and the AnimCJK licence ambiguity is recorded rather than resolved.**

`apps/nihongo-tutor/src-tauri/licences/` and its `licences.rs` catalogue, **not**
the repository's shared `licences/`. The shared directory's own test
(`src-tauri/tests/licences.rs` in the main app) requires every file in it to be
catalogued by the *Chinese* app, so a kana notice there would make that app ship
and display a notice for data it does not contain.

The two licence *texts* AnimCJK's statement points at are referenced out of the
shared directory, because those genuinely are shared.

**The ambiguity, which is real and should not be quietly resolved:** AnimCJK's
`COPYING.txt` assigns *"text files prefixed by `graphics`"* to the **Arphic
Public License**, and *"SVG files … representing kana or strokes"* to the
**LGPL-3.0-or-later** *because kana are not derived from the Arphic fonts*.
`graphicsJaKana.txt` — which is where the geometry comes from — is a `graphics*`
file whose content is kana, so it falls between the two clauses.

Both texts ship, so either reading is noticed, and this is not a blocker. It is
**upstream's to resolve**: an issue should be raised asking which clause they
intend for the kana `graphics` files. See also `LICENSES.md`'s "The Japanese kana
data" section, which is the public record.

### 13. **A lesson never lists something the board cannot draw.**

`lessons()` filters through `dataset.get()`, and `KanaDataset::from_kana_teachable`
drops anything without geometry, so the course cannot offer a kana the board
cannot grade. `kanji_lessons()` holds the same line for the character course: it is
built from `KanjiDataset::of_grade`, so a grade can only offer characters the
artifact actually holds, and every one of the 2,136 opens on the board. This is the
Japanese form of the Chinese invariants about not dead-ending a practice session.

**And the board resolves all three kinds of drawable thing**, which is the other
half of the same rule: `grade_attempt` looks a character up as a kana, then a jōyō
kanji, then a radical head form, because 92 of the 214 are not jōyō characters and
the radicals screen would otherwise offer things it could not grade. A character
that is none of the three — 鳩, say — is a message rather than a panic.

### 14. **An interface may post only the `GradeOptions` fields it means to change.**

`GradeOptions` lives in `hanzi-core` and is shared, and its `#[serde(default)]` is
on the **struct**, not on one field: a caller that posts `{ inkWidth }` gets the
other three tunables from `Default` rather than
`missing field 'resampleK'`. That is the shape the interfaces want — the defaults
are the ones the tolerance study was fitted against, so an interface should not
have to restate them to avoid being rejected — but it has a consequence worth
knowing: **a field renamed on one side is silently replaced by its default here
rather than failing.** So the options the interface actually posts are pinned by a
test that deserialises them the way the command does
(`ipc_contract.rs::grading_reads_the_attempt_the_interface_sends`), not just by a
test that grades with `GradeOptions::default()` and never touches serde.

This is the request half of `HANDOVER.md` invariant 2 — that the IPC contract is
camelCase on both sides — and the half this app got wrong: 110 tests, clippy and
`svelte-check` all passed while the Grade button could not grade a single attempt,
because nothing deserialised the options the frontend sends. When a command takes
a struct, test the *payload*, not the struct.

### 15. **The learner's data is this app's, and it stays this app's.**

Kana Tutor writes exactly one thing about its learner — the per-pair tallies in
`confusions.json`, in its own directory under the app's data directory — and no
other app reads it, imports it, or shares a store with it. **The Japanese and
Chinese apps are separate products; what is shared between them is code, never user
data.** So `hanzi-store` is not the answer for the drill's tallies, and it is not
the answer for the review queue that `ROADMAP_NIHONGO.md` N2 plans either, however
convenient its language-neutral types look. That was the maintainer's decision, and
it is a boundary rather than an implementation detail: a shared store for two
separate apps is what turns "separate" into "tangled", and the tangle is discovered
on the day one app's schema change breaks the other's history.

Two practical consequences:

* **The file is written by Rust, not by the webview.** The app still has *no*
  filesystem permission in `capabilities/default.json`, and that comment there is
  accurate: the webview cannot name, read or write a path. `std::fs` in the app
  crate does the writing, which also means the file needs no plugin, no permission
  and no path crossing the IPC boundary.
* **A write that fails is reported, not swallowed.** The answer is still counted in
  memory and the learner is told that it was not saved. Under the harness's own
  file sandbox this is the *normal* result — see trap 5 — because the sandbox denies
  writes to `~/Library`, so a live drill run in an agent session shows
  "the answer was counted but could not be saved: Operation not permitted" and the
  app is working correctly. To exercise the real file from a sandboxed session,
  start the app with `HOME` pointing inside the workspace — the data directory
  follows `HOME` — and pass rustup's real home as an explicit path, because writing
  `RUSTUP_HOME="$HOME/.rustup"` on the same command line does not do what it looks
  like it does. The recipe and the reason are in §5, under "Seeing what the drill
  remembered".

---

### 16. **The kanji artifact is committed too, and at 3 MB that is a decision.**

`crates/nihongo-core/data/kanji.bin.gz` is in the repository — 2,136 jōyō kanji,
**3,236,713 bytes** since the radical table moved in (invariant 24), against the
kana artifact's 70,917. Same reasoning as invariant 1, and it needs saying
separately because the size makes the opposite instinct reasonable: the Chinese
artifact is generated and gitignored (`HANDOVER.md` invariant 7), and this one is
forty-five times the kana's. It is committed because a clone must build without the
network *and* because it cannot be regenerated cheaply — it takes three upstreams,
one of which is a pinned EDRDG snapshot.

`prepare-kanji` refuses to write an artifact when the jōyō set is not exactly
2,136 characters, when a character's geometry count is not one KANJIDIC2 lists,
when KanjiVG disagrees and the character is not in the written exception table,
when a jōyō character has no geometry or no oracle, when any character would be
written without usable geometry, or when the radical table is not the 214 head
forms in order with geometry (invariant 24). `--allow-unchecked` is the only way
past a missing oracle and must be passed deliberately — an artifact built with it
rests on this code's own reading of the geometry and should not be committed.

`apps/nihongo-tutor/src-tauri/build.rs` checks **both** artifacts before
`tauri_build::build()`, which N8 added with the `include_bytes!` as this invariant
required. The kanji artifact had sat in the repository unread for a whole
milestone, which is exactly the state in which nobody notices it going missing —
and that is the argument for the check being in the same commit as the reader,
rather than one milestone later.

### 17. **A kanji's taught stroke count is the geometry's, and no merge is needed — measured, not assumed.**

The kana repair in §4.2 is kana-specific, and the reason is upstream's: AnimCJK
splits a stroke that crosses itself *for the animation*, which kana do (あ's loop)
and kanji, in this dataset, do not. So `merge_strokes` is not used on kanji, the
SVG element ids are not read, and `svgsJa/` is not fetched at all.

Two measurements over all 2,136 back that, and they are the ones to re-run rather
than trust if this is ever doubted:

* the geometry's array length is one of KANJIDIC2's `strokeCounts` for **every**
  character — the pipeline fails if that stops being true;
* for the **91** characters whose KANJIDC2 entry lists more than one count — the
  only place a split could hide behind a legitimate alternative — AnimCJK's own
  SVG element ids show one segment per stroke. Checked by fetching
  `svgsJa/<decimal code point>.svg` for those 91.

**And the count in the artifact is the geometry's**, not a chosen member of
KANJIDIC2's list, because the geometry is what the grader has reference strokes
for. For the ten characters where those differ, the alternative is the smaller or
larger historical count and is not the one a learner is taught.

### 18. **KANJIDIC2's grade is the authority, and its grade 8 is AnimCJK's `g7` — not its `g8`.**

The numbering trap first, because it is the thing that breaks silently: AnimCJK's
`g1`–`g6` are KANJIDIC2's grades 1–6, **AnimCJK's `g7` is KANJIDIC2's grade 8**
(the jōyō remainder), and AnimCJK's `g8`/`g9` are KANJIDIC2's 9/10 — jinmeiyō and
hyōgai, which this artifact does not carry. Reading the two numberings as one is
wrong for all 1,110 characters in the remainder.

The two sources also disagree about **59** characters, and the reconciliation is
two effects rather than one:

* the **20 prefecture kanji** added to kyōiku in 2017, every one of them grade 4 in
  KANJIDIC2 and `g7` in AnimCJK — this reconciles the jōyō total, and on its own
  it predicts the wrong per-grade counts;
* **39 characters the two sources place in different grades** — 夫 (3→4), 央 (4→3),
  21 from 4→5, 胃 腸 (4→6), 富 徳 群 賀 (5→4), nine from 5→6, 城 (6→4) — and *this*
  is what turns AnimCJK's 200/185/181 into KANJIDIC2's 202/193/191.

Both are pinned separately in `tests/kanji_artifact.rs`, so a reader cannot
re-derive only the misleading half the way this document's earlier draft did.

Two related source shapes, both of which cost time: KANJIDIC2's `strokeCounts` is
a **list** (91 jōyō have two entries) and `dictionaryJa.txt`'s `set` is a list too
(一 is `["g1", "radical"]`). A first element is not "the" value and the field is
not a string — the "nine KanjiVG exceptions" in `ROADMAP_NIHONGO.md`'s earlier
draft came from exactly that mistake (invariant 19).

Readings and glosses come from KANJIDIC2 and **never** from `dictionaryJa.txt`,
which carries `on`, `kun` and `definition` fields of its own. That is a provenance
decision: EDRDG is the attributed authority for them, and the annotated source is
the one the licence notice names. Do not start reading those fields for
convenience.

### 19. **An oracle disagreement is written down with both numbers, never silently passed.**

`衷` is the one character KanjiVG counts differently: 10 strokes by the geometry
and by KANJIDIC2, 9 paths in KanjiVG. `prepare-kanji`'s `KANJIVG_DISAGREES`
carries `(character, taught, KanjiVG says)` and checks **both** halves, so a
change to the geometry, to KanjiVG, or to the table fails the build. The taught
count stays the geometry's, for invariant 17's reason.

The same rule for the kana is §4.3's: `prepare-kana` refuses to write an artifact
it could not check. Neither pipeline is allowed to warn and continue.

**The nine exceptions `ROADMAP_NIHONGO.md` used to name were not real** — 謎 賭 葛
餌 遜 僅 遡 餅 牙 are the characters where KANJIDIC2 lists two stroke counts and
the first is not the taught one, so comparing against "the" count invented nine
disagreements and hid the one that exists. `kanji.rs`'s module docs carry the
account and the artifact test pins all ten characters.

### 20. **The vocabulary's level ladder is this project's, derived, and has to say so.**

Each word carries a `band`, and the rule is: **the highest kyōiku grade among the
word's kanji**, 1–6, with band 7 for a word containing a jōyō-remainder kanji;
EDRDG's `nf` rank (a block of 500 words, 1–48) orders the words *within* a band.
Measured sizes: 655 / 2,284 / 3,284 / 3,072 / 3,408 / 2,705 / 6,494.

**It is not the JLPT's and the interface may not imply that it is.** There has been
no official JLPT kanji or vocabulary list since 2010, and the best-licensed
community list chains to a source that asserts no licence, so the ladder is derived
instead. `words::band_name` returns the names a screen should use
("kyōiku 3", "jōyō beyond the school grades") and the artifact test asserts none of
them says JLPT — a small test for a real obligation.

**The stored band is not trusted.** `tests/words_artifact.rs` recomputes every
word's band from `kanji.bin.gz`, so if a KANJIDIC2 snapshot regrades a character the
words artifact fails rather than leaving the course quietly wrong. The alternative
ladder — frequency quantiles of roughly equal size — was rejected on record because
it mixes grade-1 and grade-6 kanji in one band, putting 秘密 before 六.

### 21. **A word's reading is the dictionary's, and furigana is joined, never invented.**

Two different refusals, and both matter.

**The reading is never composed from the characters.** It is chosen from JMdict's
own kana forms — the ones whose `appliesToKanji` covers the written form, common
first — which is the whole reason `Word::reading` exists as a field. 大人 is
おとな and not だいじん; 今日 is きょう; 一人 is ひとり; 明日 is あした. Those four are
asserted, because they are exactly what a "reading per character" design gets
wrong, and a course that composes readings teaches them wrong.

**The furigana is joined from JmdictFurigana on `(text, reading)` and never
filled in.** 21,836 of 21,902 words are aligned; the other **66** keep their reading
and carry no ruby. Do not derive an alignment to close that gap — inventing one is
the same mistake as composing a reading, one step further out. The alignment has two
properties the artifact test asserts over the whole set: the `ruby` parts concatenate
to the text, and the readings concatenate to the reading *after folding katakana to
hiragana*. That fold is load-bearing for exactly two words — 生ゴミ (read なまごみ,
written なま**ゴミ**) and タンパク質 — where the written form's kana and the
dictionary's reading differ in type. `Word::furigana_spells_reading` is the check,
and the test names both words so a third is noticed rather than tolerated.

**And the membership rule is the committed kanji artifact, not a second opinion.** A
word is carried only when every one of its kanji is in `kanji.bin.gz`, so the
vocabulary course can never offer a word the board cannot draw. That is also what
keeps jinmeiyō and hyōgai vocabulary out (513 ranked entries) until those sets
exist — not a filter to loosen without them.

One trap inside that rule: **"has a kanji form" is not "contains a kanji".** JMdict
lists full-width numerals such as `１０００` under `kanji`, so 15 words have no kanji
character at all; without an explicit `is_kanji` check they are kept and land in band
1, because an empty grade list falls back to band 1.

### 22. **A passage is held to the vocabulary, and the analyser runs at build time.**

`prepare-passages` **refuses to write** an artifact containing a kanji the words
artifact does not teach, naming every offending token. A passage with a character
the learner has no card for is one they can neither read nor tap, so this is an
error rather than a warning — and `tests/passages_artifact.rs` re-checks the
committed artifact against the committed vocabulary, which catches a passage edited
*after* it was generated. Kana, particles, punctuation and numbers are free; the
constraint is on kanji.

**The analyser is a build-time dependency and must stay one.** `lindera` (MIT) and
UniDic segment the passages when the artifact is built; the app renders tokens it
was handed and tokenises nothing. That is not an optimisation: UniDic's
`unidic-mebac-2.1.2` archive is a 134 MB download and 190 MB built, and shipping it
would break the promise the rest of the app keeps. Two things follow:

* the pipeline is behind the **`tokenize` feature**, which `pnpm test` and
  `pnpm run check:rust` deliberately do **not** enable — they use `prepare`, which
  covers the three pipelines that write shipped artifacts, while this one needs 100
  more crates. `lint it deliberately` when you touch it (see §5);
* lindera's build script downloads **nothing** unless
  `LINDERA_BUILD_DICTIONARY_CACHE_DIR` names a directory, which only
  `scripts/fetch-unidic.sh` does. Never call the fetch from a build.

### 23. **A word's membership is EDRDG's judgement, not one of its fields.**

The vocabulary's rule is EDRDG's own "this word is worth teaching", expressed
through **two** of its signals, plus one condition on the spelling:

* **`nf01`–`nf48`, or `ichi1`/`ichi2`.** Either is enough. The rank alone drops 行く
  (to go) and 本 (book), which carry `ichi1` and no rank — `nf05` belongs to a
  different entry of 本, read もと — and 2,471 entries are in that position. That is
  also why `Word::nf` is an `Option`, and why unranked words sort *after* the ranked
  ones in their band rather than in front of them.
* **The written form must be one EDRDG marks common.** No fallback to an unmarked
  form: 7,629 entries are common words whose kanji spelling is not what anyone
  writes — 彼処 for あそこ, お握り for おにぎり, 先 for さっき, 型録 for カタログ — and
  a further 173 carry an explicit `rK`/`sK`/`ateji` tag on their only common form.
  A kanji course that taught those would teach spellings its learner will never
  meet, so they are left to the kana course. **This is why the vocabulary is 16,073
  and not 21,902: smaller and right rather than larger and wrong.**
* **Every kanji must be in the committed kanji artifact** (invariant 21), which is
  what keeps jinmeiyō and hyōgai vocabulary out until those sets exist.

`spec1`/`spec2` (specialist vocabulary) and `gai1` (loanwords) are deliberately not
part of membership: they would add about 1,200 words of technical and foreign
terminology to a beginner's course.

### 24. **The radical table is the 214 head forms, its order is the numbering, and a character's number is not its note.**

Three claims, and each was measured rather than assumed. The table is what the
radicals screen is built from, and the artifact carries it (format version 2,
`KANJD002`) because it is **not derivable from the characters**: a character stores
the *combining* form of its radical — 扌 in 持 — and the screen needs the head form
手, for all 214, sixteen of which no jōyō character uses at all (90 爿 … 214 龠).

* **The numbering is `dictionaryJa.txt`'s file order.** The file tags exactly 214
  entries `radical`, and their position is the Kangxi number: 一, 丨, 丶 … That is
  checked against KANJIDIC2's classical radical number, which agrees for **212** of
  the 214. The other two — 戶 (63) and 靑 (174) — have no classical number in
  KANJIDIC2 because EDRDG records the other form of the same radical (戸, 青), and
  they are a written exception checked in both directions, so either file changing
  fails the build.
* **The entries' own glosses are never read.** Three of the 214 state the wrong
  number — 耒 at position 127 says "Kangxi radical 136", 臼 at 134 says 133, 足 at
  157 says 156 — so a pipeline that parsed the number out of the text would be
  wrong three times in a way nothing else would catch. `prepare_kanji` does not
  read the field at all, for the provenance reason invariant 18 gives as well.
* **A character's classification and its annotation are two different things, and
  they disagree for 18 characters.** `Kanji::radical_number` is KANJIDIC2's
  classical radical, which is the *Kangxi dictionary's* index; `Kanji::radical`
  and `radical_note` are AnimCJK's "the radical this character is written with" —
  巡's note is ⻌ (辵), and Kangxi files 巡 under 47 巛; 郭's note says 阜 and
  KANJIDIC2 classifies it under 163 邑, which is the correct side of 阝. The
  families group by the **number**, because that is the classification and the
  artifact is keyed by it, and `tests/kanji_artifact.rs` names all 18 so nobody
  re-derives the other 2,118 as agreement.

Two consequences worth carrying. **A radical the jōyō set never reaches is still
returned**, with an empty family, because "no character in this set uses it" is a
fact about the set and not a hole in the panel — and the head form has geometry
regardless, so it can be written on the board. **And the board takes a radical as
readily as a character**: `grade_attempt` resolves kana, jōyō kanji and radical head
form, in that order, which is what makes "Write 水 on the board" work for the 92
head forms that are not jōyō characters.

### 25. **A graded attempt is a review only when the character is new or due.**

The rule N2 added, the one that is not obvious, and the one most likely to be
"simplified" away. `ReviewStore::record_attempt` looks at the card *before* it
records: if the character has one and its `due` is still ahead, the attempt is
graded and the schedule is left exactly as it was.

The reason is SM-2's own arithmetic. A learner who writes あ five times in one
sitting and gets it right each time would otherwise advance the interval five
times — 1 day, 6 days, ~39, ~253, 365 — and not meet あ again for a year on the
strength of a minute's practice. Nothing would fail; the schedule would simply be
wrong, and wrong in the direction that looks like success. A failed attempt is
different and does come straight back: SM-2 schedules `Again` sixty seconds out,
which is a review again by the time the learner has drawn it twice.

Three things follow, and each is pinned:

* **`grade_attempt` returns a wrapper** — `{ report, scheduled, nextDue, warning }`
  — not a bare `GradeReport`. `scheduled` is what tells the three screens a counted
  attempt from a practice one, and the contract test asserts the response keys
  *and* the request payload, which is the half invariant 14 exists for.
* **A write that fails still counts.** The card is updated in memory, the failure
  travels as `warning`, and the verdict is still returned: a learner asked for a
  grade and the grade happened. Under the harness's file sandbox this is the
  *normal* result (trap 5), and it is what the live check shows.
* **A schedule this build cannot read is never overwritten.** A file that will not
  parse is moved aside as `review.json.corrupt` and a fresh one started; a file
  from a newer build is left untouched and the session runs in memory with a
  warning. This is `hanzi_core::progress`'s own rule, applied in the app for the
  one file where guessing would lose real work — and it is the same refusal
  `ConfusionStore` already made.

The other half of the milestone is a **decision, not a mechanism**, and it is
recorded in `ROADMAP_NIHONGO.md` N2's Shipped section: the schedule exists because
kanji study needs it, and the kana ride along on the same unit; FSRS is the better
algorithm and is deliberately not in, because its pretrained weights are not
openly licensed and one learner cannot fit them. `Scheduler` is the seam.

### 26. **The Japanese app speaks Japanese, and the voice is the machine's own.**

`hanzi_voice::Speaker` now carries a `Language`, and `Speaker::default()` is
still **Chinese**, because the two Hanzi Tutor apps were written against it and
this crate is shared. A kana tutor that forgot to say otherwise would read あ with
the Mandarin system voice, and **every test in this document would still pass**:
a synthesiser accepts any string, the command returns `Ok(())`, the button reports
no error, and the learner hears the wrong language. Two tests exist for exactly
that, and neither is about audio quality:

* `the_app_speaks_japanese_and_not_chinese` (the app's own suite) checks the
  language the state was built with, and, where the machine has a voice at all,
  that the resolved description names a Japanese one. A machine with no Japanese
  voice installed is a legitimate `None` and not a failure — the interface
  disables every button and says why;
* `the_voice_the_interface_is_offered_is_japanese` (the contract suite) is the
  same check on the other side of the IPC.

Per language: the **preferred names** (Kyoko, then Otoya; Tingting, then Ting-Ting,
then Meijia), the **home locale** (`ja_jp`, `zh_cn`), the **prefix filter**
(`ja`, `zh`) and the **override variable**. That last one is separate on purpose —
`NIHONGO_TUTOR_VOICE` here, `HANZI_TUTOR_VOICE` there — because a kana tutor obeying
a variable named after the other product is invariant 15's mistake in settings
form. The Tone Trainer's decision is repeated too: one voice, chosen
automatically, no voice-picking screen.

**A measurement worth keeping, because it is not what reading `say -v '?'`
suggests.** `AVSpeechSynthesisVoice.speechVoices()` — what this crate actually
reads on macOS — reports a voice as the **bare name with a BCP-47 tag**:
`Kyoko`, `ja-JP`. The legacy command prints the qualified form,
`Kyoko (Japanese (Japan)) ja_JP`. Both have to resolve to the same voice, and both
are now pinned by a fixture, so a rule that only understands one spelling fails a
test rather than silently falling through to another voice. The two normalisations
are `locale_key` (lower-case, `-` to `_`) and `base_name` (strip the ` (` onward),
and every locale comparison in the file goes through one of them.

**Nothing is bundled and nothing is fetched.** The audio is the platform's own,
spoken in process by `AVSpeechSynthesizer`; `apps/nihongo-tutor/src-tauri/src/lib.rs`
still says "there is no network path in this crate at all" and N1 did not change
that. `Speaker::prime` on macOS builds the synthesiser and resolves the list on a
thread of the app's own (`warm_voice`), so the first tap pays for nothing — the
line it prints, `[speech] using voice Kyoko (ja-JP)`, is the cheapest proof that a
built app reached the real voice list.

**And the microphone is deliberately not linked.** `hanzi-voice`'s capture half
carries `cpal`, this app never opens a microphone, so `capture` is a *default*
feature and this app takes the crate with `default-features = false`. The two
tone-scoring apps are unaffected because the feature is on by default; a new app
that only speaks turns it off. Same instinct as refusing to bundle the analyser.

### 27. **Nothing is spoken that is not kana the app stored, and the shortcut is `h`.**

What may be handed to `speak` is a **kana character**, a **word's own stored
reading**, or **one reading of a kanji**, and the list is exhaustive rather than
indicative:

* never a reading composed from a written form — 大人 is おとな, and invariant 21
  is the same rule one step out;
* never a bare kanji. 生 has a dozen readings and the app does not choose between
  them; it lists them and lets the learner choose, which is the difference between
  this rule and a reading picker. **The Review board still offers the button only
  for a `kind == "kana"` item**, which is why a due 学 shows *Show stroke order ·
  Undo · Clear · Grade* and no "Hear it". That absence is the rule being visible;
* never a gloss (the Review prompt for a kanji is English) and never a radical's
  Kangxi number.

The word card is the one that shows the difference most plainly: it speaks
`word.reading`, so 一つ says **ひとつ** and not the written form. The live check's
log recorded `speak "ひとつ"` and never `speak "一つ"`, which is the assertion in a
form a log can carry.

**A kanji card offers hearing and writing together, and the learner picks.** Every
reading it lists — on, kun and nanori — is its own control, and the board above it
is the writing half; the card says so in one line. This is the maintainer's
decision, made explicitly (*"offer both, the user can decide what he wants
(hearing or writing)"*), and it is the one place the app puts the two ways of
meeting a character side by side instead of choosing for the learner.

**And the markers come off only for the voice.** KANJIDIC2 writes a reading with
its okurigana attached, and both marks are **teaching information** — they say how
the character is written — so they stay on screen and go only to the synthesiser:

| on the card | handed to the voice | why |
| --- | --- | --- |
| `た.べる` | `たべる` | the dot ends the stem and starts the okurigana |
| `ひと.つ` | `ひとつ` | the same, which is why the dot is always **medial** |
| `ひと-` | `ひと` | the dash marks an affix that is not used on its own |
| `-ノウ` `-ネン` `-ノン` | `ノウ` `ネン` `ノン` | the same mark, **leading**, and on *on*-readings |
| `イチ` `ガク` | unchanged | katakana is what a Japanese voice reads |

`spokenReading` in `src/lib/kanji.ts` is that table and nothing else. It is a
one-liner because it was measured first: over all **9,364** readings there is no
character outside hiragana, katakana, ー and the two marks, every reading keeps at
least one kana once the marks come off, the dot is never at an end and never twice
in one reading, and the dash is never medial.
`every_reading_is_kana_once_its_markers_are_stripped` in `tests/kanji_artifact.rs`
pins all of it, including the four marked on-readings (応 王 縁 音) — the case a
"the dash is a kun thing" reading of the field misses. The control's own title
names the spoken form, so `た.べる` on screen hovers as *"Say たべる …"*.

Two smaller decisions belong to the same card. **The readings are drawn as text,
not as buttons** — 生 has twenty readings, and twenty bordered boxes would bury
the thing being read — so they borrow the passage's dotted-underline affordance,
and a disabled one is *plain text*, which is what a machine with no voice should
see. That last part hides a CSS trap: a disabled `<button>` is coloured grey by
the user agent, so `opacity: 1` alone leaves a card of grey readings that reads as
a card of failures. `color: inherit` is the other half, and it is why the no-voice
card was captured rather than assumed.

**The shortcut is a letter.** `h`, for "hear", and *not* Space — Space is what the
Tone Trainer gives its microphone, but on a page it already means "activate the
control that has focus", and a learner who has just clicked a kana cell still has
that cell focused. A Space shortcut would re-press the cell instead of speaking,
which is the commonest way to reach for it and would have looked like a broken
feature. The rule — a repeat does not count, `⌘`/`Ctrl`/`Alt` do not count (⌘H
hides the window), `h` in an `INPUT`/`TEXTAREA`/`SELECT`/contenteditable does not
count — is `isHearItKey` in `src/lib/speech.ts`, a pure function with ten tests,
because the reading box is exactly where `は` is typed as `ha`.

### 28. **A character's card lists the words the course teaches, in the course's order — and says so when there are none.**

`WordDataset::of_kanji` is a filter over the set the artifact already holds, in the
order `from_words` put it in, and `words_of_kanji` pages it. Three things about
that are decisions rather than implementation:

* **The order is the course's** — band, then EDRDG's frequency — so the first page
  is the vocabulary whose *other* characters have been taught first. It is a filter
  over the order the artifact is already stored in, not a second sort: ordering the
  page by frequency alone would put a word written with a remainder-grade character
  at the top of a grade-1 card. `tests/words_artifact.rs` walks five characters and
  asserts the key never goes backwards, which is the property the screen's promise
  rests on.
* **The count is the character's and the page is twelve**, because it is measured:
  **一 is written in 223 of the 16,073 words, 人 in 218**, and 86 characters are in
  more than sixty. A card that shipped them all would be a megabyte of JSON to draw
  a list — the same reason a band is paged (`MAX_WORD_PAGE`, the same clamp).
* **A jōyō character in no word is a state the card states**, not an empty list:
  **57 of the 2,136** are in that position (且, 貞, 隻 …). And a character **outside**
  the jōyō set is an *error* from the command, not an empty page — the vocabulary
  holds no word such a character could be in (invariant 21), so an empty list there
  would make a typo look like a gap in the data. That is invariant 13's rule, one
  command over: a message, not a panic, and not a silent nothing.

Two smaller halves of the same contract. The response is a **page shape with `ch`
echoed back** — `{ ch, total, offset, words }` — so a screen can tell an answer
about the character it asked for from one about the character that was up a moment
ago, and `ipc_contract.rs` pins both that shape and the `{ ch, offset, limit }`
payload the interface posts (invariant 14's request half). And **the readings are
still the dictionary's**: the card links to `WordCard`, which speaks
`word.reading` and grades against it, so nothing here composes a reading or hands a
bare kanji to the voice (invariants 21 and 27).

---

### 29. **A grid row is five slots with holes, and a drill pair is two spellings.**

Five rules N4 added, and each one is a thing that looks like a simplification and
is not.

* **A row's holes are data, not a shorter list.** `Row::cells` is five slots —
  a, i, u, e, o — with `None` where the language never filled one, and `Row::kana`
  is derived from it, so the course and the chart read one table. や has three kana
  and they are in the a, u and o columns: a chart built from a list draws ゆ under
  い and teaches the wrong vowel. **16 rows, 80 slots, 71 kana, 9 holes** (や's i
  and e, わ's i, u and e, ん's four). The layout is stated in the table and then
  *checked* against something independent — a kana's own Hepburn vowel, which is
  the column it has to be in (`every_cell_sits_in_the_column_its_vowel_names`) —
  because a hand-typed grid is exactly the kind of table that is wrong once and
  never noticed.
* **The chart offers nothing that cannot be opened.** `curriculum::off_grid` is the
  small kana, the rare ones and katakana's v-series and ー, it is the *same*
  function `lessons()` adds to the grid, and `kana_chart` serves geometry through
  the same `kana` command everything else uses. So acceptance criterion 3 of N4 —
  a kana offered anywhere can be opened and graded — is a property of the data
  rather than a promise in a comment, and `every_kana_the_chart_offers_can_be_opened`
  is what holds it. This is invariant 13's rule one screen out.
* **A drill pair is two *spellings*, and the keys already in the learner's file
  must not move.** A yōon contrast's two answers are two characters each (きゃ
  against きや) while a voicing contrast's are one (か against が), so the pair is a
  pair of strings and `key_of` orders them. It reproduces `pair_key` exactly for all
  thirteen classic pairs — `a_kana_pair_keeps_the_key_it_was_stored_under` is the
  check — because `confusions.json` already holds them and invariant 15's file is
  not something to re-key. And the lookup **canonicalises before comparing**: a
  caller may name a pair either way round (`ツ|シ`, `が|か`), which is what the
  contract test posts even though the app's own payload is always canonical, and
  what the first version of this generalisation broke.
* **A side carries its own prompt, and a prompt is a reading the app can type.**
  Which of a pair is asked for is a roll, and each spelling has its own reading, so
  the prompt travels with the side rather than being looked up afterwards. The
  two-mora counterpart is *composed* — きや is き's reading plus や's, and しや's
  Kunrei is `siya` — and then held to a second path:
  `the_plain_counter_of_every_yoon_is_what_the_input_engine_types` asks the romaji
  engine to type both spellings of all 66 contrasts. Hand-writing the counterpart's
  reading would be the same mistake as composing a kanji's (invariant 21), one step
  further out.

  **With four written exceptions, which are the yotsugana.** ぢ's Hepburn reading is
  `ji` and づ's is `zu` — the same as じ's and ず's — so those four sides of the
  voicing drill's hundred are the only ones whose prompt does not type back to its
  own kana: `to_kana_in` answers じ and ず, and it is *right* to, because
  `matches_reading('ぢ', "ji")` is true. Kunrei distinguishes them (`di`, `du`) and
  swapping the prompts would teach a romanisation the Practice screen never shows,
  so they are pinned instead —
  `four_voicing_prompts_are_the_yotsugana_and_type_back_to_their_pair` names all
  four, asserts the other 96 round-trip, and asserts the pair is still answerable
  because its two prompts differ. What it means is that a prompt identifies the
  *contrast*, and only for these four does it not also identify the character.
* **A voiced row is related to its plain row, not to a code point.** The voicing
  exercise's 50 pairs (25 per script) come from `VOICED_FROM`, which names the five
  voiced rows and the plain row each is written from — は twice, because ば carries
  the dakuten and ぱ the handakuten — and the pairs are zipped from the two rows'
  slots. The kana in a pair differ by one code point (dakuten) or two (handakuten),
  and `the_voicing_pairs_differ_only_by_the_mark` checks that *after the fact*
  rather than the pairs being built from it: deriving the language from the encoding
  would work for all 25 and be the wrong reason, and it would break silently the day
  a character is added to the block in another order.

What is **not** changed by this: the record is still this app's own file under this
app's own directory (invariant 15), the classic pairs are still the classic thirteen
with the flat list's discovery gap standing, and nothing about the yōon or voicing
exercises touches the kanji or the schedule.

## 5. The verification loop

Four layers, cheapest first. All of them are worth running before a commit that
touches the engine or the data; the last two before one that touches the
interface.

```bash
# 1. The data layer, the app, and the shared crate the audio half lives in —
#    including the IPC contract and the notices.
./scripts/with-cargo-env.sh cargo test -p nihongo-core -p nihongo-tutor -p hanzi-voice

# 2. Nothing else broke. Fast when the tree is warm.
./scripts/with-cargo-env.sh cargo test --workspace \
  --features hanzi-core/prepare --features nihongo-core/prepare

# 3. Lints. -D warnings, so a warning is a failure.
./scripts/with-cargo-env.sh cargo clippy --workspace --all-targets \
  --features hanzi-core/prepare --features nihongo-core/prepare -- -D warnings

# 4. The frontend: types, then the pure arithmetic.
pnpm --dir apps/nihongo-tutor run check:web     # svelte-check
pnpm run test:web                               # from the ROOT — it owns vitest
```

**Layers 2 and 3 carry both `prepare` features deliberately, and layers 1 and 4
being narrower is deliberate too.** `prepare-kana`, `prepare-kanji` and
`prepare-words` declare `required-features = ["prepare"]`, so a plain
`cargo clippy --workspace` does not compile them at all — the binaries that write
the committed artifacts would be the only Rust in the tree that no lint ever sees.
`--features nihongo-core/prepare` closes that, and it is why `pnpm test` and
`pnpm run check:rust` (what CI runs, see `.github/workflows/ci.yml`) spell it out
rather than relying on `--all-targets`.

**`tokenize` is the one feature deliberately left out of that list** (invariant 22):
`prepare-passages` needs `lindera`, which is 100-odd crates for a binary whose
output is committed and whose tests need no analyser at all. Lint it by name when
you touch it — the command is in "Regenerating the artifacts" above — and note that
enabling it still downloads nothing, because lindera's build script is inert unless
`LINDERA_BUILD_DICTIONARY_CACHE_DIR` is set.

`apps/nihongo-tutor/src/lib/board.test.ts` is picked up by the **root** project's
vitest, through the `include` list in `vitest.config.ts`. The app itself installs
no test runner, deliberately — the same arrangement the tone trainer uses. A test
added under `apps/nihongo-tutor/src/**` and not named `*.test.ts` will silently
never run.

### Regenerating the artifacts

```bash
./scripts/fetch-data.sh
pnpm run prepare-kana
pnpm run prepare-kanji
git diff --stat crates/nihongo-core/data/kana.bin.gz crates/nihongo-core/data/kanji.bin.gz
```

`prepare-kana` prints what it did, and the numbers are the check: **177 kana, 86
hiragana, 91 katakana, 516 strokes, 25 split characters, 27 segments folded back,
177 checked against KanjiVG**. If `checked against KanjiVG` is not 177, the run
failed rather than warned — unless `--allow-unchecked` was passed.

`prepare-kanji` prints its own, and they are the numbers
`tests/kanji_artifact.rs` pins: **KANJIDIC2 3.6.2 (2026-09-28), 7,007 graphics
rows, 2,136 kanji (1,026 kyōiku, 1,110 remainder), 22,367 strokes, grades
g1:80 g2:160 g3:200 g4:202 g5:193 g6:191 g8:1110, radicals 214 (198 used by jōyō,
16 unused), 1,222 strokes, 212 of the 214 numbers agreeing with KANJIDIC2 and 2 it
cannot number (戶:63 靑:174), dictionaryJa 2,077 agree and 59 differ (20 added in
2017, 39 reassigned), 2,136 checked against KanjiVG with 衷 as the one written
disagreement, artifact 3,237 KB from 6,675 KB**. Moving any of those numbers is a
decision to take and record, not a rebuild: a different KANJIDIC2 snapshot, a
different jōyō set, a new oracle disagreement or a changed radical order all mean
the tests need updating in the same commit.

**The radical lines are the ones N8 added, and the two exceptions are the ones to
read.** `radicals 214 (198 used by jōyō, 16 unused)` moving means the jōyō set or
the classification changed; `radical numbering 212 agree …, 2 it cannot number
(戶:63 靑:174)` moving means either a new disagreement — which the build refuses
rather than prints — or that KANJIDIC2 has started numbering one of the two, in
which case the written exception comes out of `RADICALS_KANJIDIC2_CANNOT_NUMBER`
in the same commit.

Both artifacts should be **byte-identical** to the committed ones. If one is not,
something upstream moved, and the artifact test will say what changed.

`prepare-words` prints its own, and they are the numbers `tests/words_artifact.rs`
pins: **JMdict 3.6.2 (2026-09-28), the priority markers from a JMdict_e created
2026-10-02 — 22,430 entries ranked and 2,471 marked common without a rank — furigana
2.3.1+2026-09-25, 24,901 of 218,840 entries considered (193,939 not marked common),
16,073 words in bands b1:585 b2:1781 b3:2408 b4:2232 b5:2412 b6:1834 b7:4821, 754
left out for having no kanji at all, 7,629 whose kanji spelling is not the one
EDRDG marks common, 445 with a kanji the kanji artifact does not hold, 16,022 with
furigana, artifact 602 KB from 1,382 KB**. The vocabulary was **21,902** words
before the common-spelling rule of invariant 23 was applied, which is why that
invariant says "smaller and right rather than larger and wrong": the difference is
7,629 spellings nobody writes plus the kana-only entries. It reads 118 MB of JSON
and 63 MB of XML to do it, which takes a few seconds and is why the result is
committed.

`prepare-passages` prints its own, and they are what `tests/passages_artifact.rs`
pins: **lindera 6.2 with unidic-mecab-2.1.2, 3 passages, 7 lines, 37 tokens with 14
linked to a vocabulary word, every kanji-bearing token taught, 502 bytes**. It
refuses to write at all if a passage uses a kanji the vocabulary does not hold,
naming the tokens. It needs the dictionary first:

```bash
./scripts/fetch-unidic.sh      # once: 134 MB down, 190 MB built into .lindera/
pnpm run prepare-passages
```

To lint or test the passage pipeline you have to ask for its feature, because
`pnpm test` and `pnpm run check:rust` deliberately do not:

```bash
./scripts/with-cargo-env.sh cargo clippy -p nihongo-core --features nihongo-core/tokenize \
  --all-targets -- -D warnings
```

`fetch-data.sh` fetches the kanji oracle with `xargs -P 4` and every download
carries `--retry 4`, which is not decoration — see trap 11. `JmdictFurigana.json`
arrives with a UTF-8 BOM — see trap 12.

### Seeing the interface

There is no display in every environment this gets worked on, so this recipe
matters. It captures **only the app's window**, which is both more reliable than
the app being frontmost and the right thing to do on someone else's machine.

```bash
# Start the app (dev server on :1422 plus the window) in the background.
pnpm --dir apps/nihongo-tutor run dev > /tmp/kana-dev.log 2>&1 &

# Find the window by TITLE — the owner name is `nihongo-tutor`, which contains
# no "kana", so matching on the owner silently finds nothing.
WID=$(python3 -c "
import Quartz
for w in Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionAll, Quartz.kCGNullWindowID):
    if str(w.get('kCGWindowName')) == 'Kana Tutor':
        print(w.get('kCGWindowNumber')); break
")
screencapture -x -o -l"$WID" /tmp/kana.png
```

`-l<id>` captures that window's contents even when it is not frontmost, and `-o`
omits the shadow. Read the result; do not assume.

**And on a machine with no active display it captures a frozen frame.** N1's check
hit exactly that: `CGGetActiveDisplayList` answered `0`, and every capture of the
window — before a reload, after one, and minutes apart — came back with the **same
md5**. The window had rendered once, at startup, and was never composited again, so
the image was of the first frame for the life of the process. That is worse than
trap 4's "capture whatever is in front", because it looks like the current screen.
The discriminator costs nothing:

```bash
python3 -c "import Quartz; print(Quartz.CGGetActiveDisplayList(8, None, None)[0])"
```

Zero means no rectangle is being composited, so **do not believe a capture** and
use the HTTP probe below instead. `screencapture` still works there, and the file
it writes still looks right — which is the whole trap.

To see a view that needs a click, change the initial value of `view` in
`App.svelte` and **restart the app** — see trap 3, HMR preserves state.

### Driving the interface, since it cannot be clicked from here

System Events is blocked (trap 6), so nothing outside the app can click it, draw
on it or read its DOM. Two things work, and between them they are enough to check
any screen:

* **A temporary script in `index.html`** — it is served by Vite for both the dev
  server and the webview, it can click anything, and editing it makes Vite
  **reload the page**, which re-runs the script against a fresh app. A `MODE`
  constant plus a few lines that click a tab by its label and then report
  `document.querySelector(...).innerText` back over the dev server is the whole
  harness. Delete it before committing; it is a diagnostic, not a feature.

  **When a capture cannot be trusted (above), the report goes over HTTP and is
  read from the dev server's log** — a temporary Vite plugin in `vite.config.ts`
  whose middleware answers `/__probe` with 204 and prints the line:

  ```ts
  server.middlewares.use((req, res, next) => {
    if (req.url?.startsWith("/__probe")) {
      server.config.logger.info(`[probe] ${decodeURIComponent(req.url)}`);
      res.statusCode = 204;
      res.end();
      return;
    }
    next();
  });
  ```

  That is better than a file sink, because the dev server's own log is already the
  place the app's stderr lands — no path, no permission, nothing to write. Report
  **one step at a time** rather than one object at the end: a probe that throws
  half way through an async function is otherwise silent, since the DOM `error`
  event does not fire for a rejected promise (`unhandledrejection` does). N1's
  first probe reported nothing at all for exactly that reason, and the step-by-step
  form said which line had failed.
* **And a temporary `eprintln!` in the command under test**, which is how N1
  proved the *interface's* press reached Rust rather than only its own JS: the
  probe clicked the button, and `[probe] speak "あ" -> Ok(())` appeared in the dev
  log. It is worth reaching for whenever the question is "did the app really do
  it" — the webview cannot see its own failed `invoke`, and this can. Delete it
  with the rest of the probe.
* **`webview_log` does not exist in this app.** The Tone Trainer has one; adding
  it here just for a check would put diagnostic code in a shipped command, and the
  dev log carries the app's own `eprintln!`s already.
* **Synthetic pointer events on the board**, which is how the verdict was finally
  seen. `kana.medians` are in display space, `pointerToDisplay` maps the canvas's
  client rectangle into that same box, so a driver can interpolate along each
  centre-line and dispatch `pointerdown`/`pointermove`/`pointerup` — an attempt
  that should score ~99. Two details: read the centre-lines over the app's own IPC
  (`window.__TAURI_INTERNALS__.invoke("kana", { ch })`) rather than guessing them,
  and **stub `canvas.setPointerCapture` first**, because a synthetic pointer was
  never really down and the real call throws, which would abandon `down()` before
  it records the stroke.

* **And a read of the DOM need not go over the dev server at all.** N2's live check
  collected the facts — the active tab, the header counts, every due item's text,
  the character on the board, the note under it — into a JSON string and appended a
  `position: fixed` `<pre>` with it to `document.body`, then captured the window.
  If the report is in the image, the image is *current*, which is the one thing a
  `-l` capture cannot tell you on its own; and it needs no Vite middleware, because
  the probe's output *is* the screen.
* **A surprising capture is not automatically a stale one.** One from N2's check
  showed more due characters than the file held, a grade nobody had recorded, and
  a hand-drawn character — and it read as the stale backing store above. It was
  real: the maintainer had been drawing on the window while the check ran. Ask the
  DOM what it holds, then ask the person, and only then call an image an artefact.
  (The repository's own history has both cases: the N8 captures *were* stale, and
  this one was not.)

### Hearing it speak

The audio half has one check no test can make and one line that comes for free.

**The free line first.** The app resolves the voice and builds the synthesiser on a
thread of its own at startup, and says what it found: `[speech] using voice Kyoko
(ja-JP)` in the dev log means the real `AVSpeechSynthesisVoice` list was read and a
Japanese voice was chosen. `no Japanese voice installed; pronunciation will be
unavailable` means the machine has none and every button will be disabled — which
is a legitimate answer and not a failure to chase. This costs nothing and catches
the one catastrophic case, a Chinese voice reading Japanese.

**Then the press, which is the interface's half.** The DOM probe clicks "Hear it"
and dispatches a synthetic `KeyboardEvent`, and a temporary `eprintln!` in the
`speak` command prints `[probe] speak "あ" -> Ok(())` — so the log says *which
string* reached Rust, which is how "the word card speaks the reading and not the
written form" was checked (`speak "ひとつ"`, never `speak "一つ"`). Four presses are
worth repeating after any change here: the button, `h` on the window (both must
speak), `h` inside the reading box and `⌘H` (neither may).

**`Ok(())` is not a sound, and the machine is not a test.** The synthesiser
returning success means it accepted the utterance; whether anything came out of a
speaker is the maintainer's to confirm, and N1's was. Do not record "audio works"
on the strength of a return value.

**And the kanji card's readings are worth pressing after any change to the
artifact.** They are the one place the app speaks a string it transformed rather
than one it stored whole, so the marker stripping is what a probe should watch:
tapping `ひと.つ` must log `speak "ひとつ"`, `ひと-` must log `speak "ひと"`, and
日's `-び` must log `speak "び"`. The control's `title` is the other half of the
check — it names the spoken form, so the DOM says what will be heard without
pressing anything.

**A due item on demand.** Anything that offers a control only for one kind of
scheduled character needs two cards due at once, and grading cannot produce that —
a failed attempt comes back in a minute and a good one tomorrow. Run the app with
`HOME` inside the workspace (the recipe under "Seeing what the drill remembered")
and **hand-write the schedule** instead; `.tmp-*/` is gitignored, and
`review.json` is `{ version, cards }` keyed by the character, so a card with `due`
in the past is due now:

```json
{ "version": 1,
  "cards": { "あ": { "attempts": 1, "lapses": 1, "due": "2026-10-01T09:01:00Z",
                     "intervalDays": 0.000694, "ease": 2.5, "repetitions": 0 } } }
```

Two of those — one kana, one kanji — is what showed あ with a "Hear it" button and
学 with none, in one run. The same trick is how a queue with anything in it can be
seen at all in a sandboxed session, where `review.json` cannot otherwise be written.

### When the window is blank

A blank white window is not a webview problem to be worked around; it is usually a
JavaScript error, and the webview will not tell you — Tauri does not forward the
console, and `log show` on the app's process showed nothing. What does work is a
**beacon**: add to `index.html` a `window.addEventListener("error", …)` that
`fetch`es `/__probe?error=…&stack=…`, and a Vite plugin (in `vite.config.ts`) whose
middleware appends every request to a log file. The request log then says exactly
how far the page got and the probe carries the message and stack. This is how
`effect_update_depth_exceeded` was found after a screenshot had said only "blank".

Two things to know about reading that log: WebKit serves already-fetched modules
**from its cache**, so a reload shows the `index.html` request and nothing else —
a cold app start is the honest measurement; and give each beacon a `?t=<now>` so
its own fetches are not cached either.

**Two things about reading a capture.** A window captured with `-l` while it is not
frontmost can come back as a **stale backing store** — blank, or showing a scroll
position that the DOM does not have — and the DOM is the truth: a probe that reports
`scrollY=0` and the header at y36 while the image shows neither is a capture
artefact, not a layout bug. Capture twice, and give the window a moment after a
reload before believing what it shows.

**N8 showed how far that can go.** Two captures of the same window, minutes apart
and with nothing driving the app, came back as two *different views*: one showed the
practice board with a confident hand-drawn あ that no learner had written, and the
next showed the discrimination drill with a question answered and a save error. The
DOM, asked directly, said `activeView=Practice`, `inkStrokes=0` — which is what a
cold start is. Both images were artefacts. So the rule is not "capture twice and
compare": it is **ask the DOM what it has, and treat the image as a second
opinion**; and give the window a repaint (a reload) before a capture you mean to
believe, because the two identical captures after that reload agreed with the DOM
exactly.

And when a window is blank, the harness above is the way in — but note that the
window will also be blank if the page never ran at all. A probe log with *nothing*
in it, not even a load line, means the script never executed: suspect the file you
just edited (and check whether `svelte-check` sees it) before suspecting the
webview.

**And a false lead worth not repeating.** Vite binds loopback as **IPv6 only** here:
`lsof` shows `[::1]:1422 (LISTEN)`, `curl http://[::1]:1422/` answers 200 and
`curl http://127.0.0.1:1422/` gets connection refused. That looks exactly like "the
webview cannot reach the dev server, hence the blank page" — but the webview had
already connected (`com.apple.WebKit.Networking` holds an established connection to
`[::1]:1422`) and the page's own modules were being served. Check
`lsof -nP -iTCP:1422` before believing the DNS story.

### Seeing what the drill remembered

The drill's record is a file, which makes it the one part of this app that can be
*read* rather than looked at:

```bash
cat ~/Library/Application\ Support/com.hanzitutor.kana/confusions.json
```

It is written after every answer, pretty-printed, and keyed by the pair
(`"シ|ツ"`, lower code point first) — so a change to the weighting rule can be seen
in the numbers rather than inferred from which kana came up. To prove that a
restart reads it back, answer a few questions, quit, start again, and answer a few
more: a pair's `asked` must continue from where it left off rather than restarting
at 1. That is also the check the tests do over a temporary directory, and it is the
one worth repeating by hand after touching `store.rs`.

Under the agent harness the write is **denied** — the sandbox refuses `~/Library`,
so the drill shows "the answer was counted but could not be saved" and is working
correctly (trap 5, invariant 15). To exercise the real file there, point `HOME` at
a directory inside the workspace **and give rustup its real home explicitly** —
`RUSTUP_HOME="$HOME/.rustup"` on the same line does *not* work, and why is worth
reading:

```bash
# Bash expands the assignments left to right, so `$HOME` on the right of
# `RUSTUP_HOME=` is the **new** home, not the one this shell started with. The
# result is `RUSTUP_HOME=<workspace>/.tmp-kana-home/.rustup`, which has no
# toolchain in it, and the app never starts: rustup answers "could not choose a
# version of cargo to run, because one wasn't specified explicitly, and no default
# is configured". Measured, on the second attempt at this recipe.
REAL_RUSTUP="$HOME/.rustup"
HOME="$PWD/.tmp-kana-home" RUSTUP_HOME="$REAL_RUSTUP" \
  pnpm --dir apps/nihongo-tutor run dev
```

The toolchain is the *only* thing that has to stay real: `scripts/with-cargo-env.sh`
already points `CARGO_HOME` and `CARGO_TARGET_DIR` inside the repository, so cargo's
registry and build output follow the project rather than `HOME`.


---

## 6. Traps that cost time here

### 1. A clamp on one axis and `aspect-ratio` on the other gives a rectangle

The board shipped as a wide rectangle with あ stretched across it and a second
clipped copy at the edge. The stylesheet had `aspect-ratio: 1` **and**
`max-height: 62vh`; the `max-height` clamps the height without narrowing the
box, and the script then sized the canvas from a measurement of the distorted
result.

It was invisible to `svelte-check`, to the tests, and to me reading my own
screenshot — it took the maintainer saying "the drawing box is vertically
compressed to a rectangle". The fix is invariant 7. The general shape of the
mistake: **two things deciding one dimension.**

### 2. macOS gives the webview the whole window, and draws the title bar over it

The heading sat half-hidden behind the traffic lights and was only obvious in a
screenshot. 20px of top padding is not enough; the strip is about 28pt. `main`'s
padding is now `calc(var(--safe-top, 0px) + 2.25rem)`, reading the same
`--safe-top` the other two apps define.

**The other two apps may have the same latent problem** — their first element sits
close enough to the top to clip. Worth checking on a display before assuming
their layouts are fine.

### 3. Vite HMR preserves a component's state, so changing an initial value changes nothing

Changing `let view = $state<…>("practice")` to `("drill")` and waiting for the
reload does **not** switch the view: Svelte's HMR preserves the existing state.
The file is correct and the running app is unchanged, which looks exactly like
the edit having failed. Kill the app and restart it.

It cost two capture cycles to notice, and the giveaway was that the file on disk
said `licences` while the window still showed Practice.

### 4. `screencapture` by region captures whatever is in front

The first captures were by region and were fine until the window lost front
position, at which point they silently captured a *different* application. Use
`-l<window id>`, by title, per §5. Matching Quartz windows on the owner name
finds nothing — the owner is `nihongo-tutor`, which contains no "kana".

### 5. The harness's file sandbox makes the app print alarming WebKit errors

Roughly a dozen lines like:

```
could not create directory ".../Library/WebKit/nihongo-tutor/WebsiteData/MediaKeys/v1"
for future sandbox extension, error … Code=513 "You don't have permission"
```

These are the *sandbox the agent runs under* denying writes to `~/Library`. They
are not the app failing, the window comes up regardless, and they should not be
chased. On a normal run they do not appear.

### 6. `System Events` is blocked, so the interface cannot be driven

`osascript -e 'tell application "System Events" to get name of first process'`
fails with `A privilege violation occurred. (-10004)`. There is no clicking
buttons or drawing strokes from here. Screenshots and code changes are the only
levers — which is why invariant 7's "look at it" recipe is worth the setup, and §5's
"Driving the interface" is how a click that matters still gets made. (Activating the
app by name does not work either: it is a bare dev binary with no bundle, so
`tell application "nihongo-tutor" to activate` answers `Can't get application`.)

### 7. Two `serde`/`format!` details that are each a compile error with a confusing message

* `assert_eq!` cannot take **both** an inline captured format arg and a named one
  (`"{script:?} …", script = script`). Use one or the other.
* A raw string containing SVG is terminated early by `"#` inside it —
  `href="#z12354d1"` — so test fixtures need `r##"…"##`.

And `sed -i ''` on macOS is BSD `sed`, which does **not** support `\b`; a
`sed 's/\bfoo/bar/'` silently does nothing rather than failing, so verify the
edit landed.

### 8. The curve that took longest to get right was a table's *order*, not its content

Both of the input engine's real bugs were ordering, not spelling: small kana
claiming vowels because they come first in code-point order (invariant 11), and
the katakana half of the table never being claimed because hiragana was iterated
first (invariant 9). The readings themselves were never wrong. When the romaji
engine misbehaves, suspect the loop before the table.

### 9. An effect that reads what it writes never settles, and the window just goes blank

`KanaCanvas`'s reset effect wrote `strokes`, `current`, `ghostCount` and `sweep`
and then called `paint()`, which **reads all four**. So the effect depended on the
state it wrote, and `strokes = []` is a fresh array — never `===` the previous
value — so the write always notified: Svelte ran it until
`effect_update_depth_exceeded` and the window came up blank on about half the cold
starts. The fix is to let the *repaint* effect (which only reads) do the painting;
`strokes = []` notifies it every time, so nothing is lost.

The same mistake in a milder form was next door: the `ResizeObserver` effect called
`measure()`, which calls `paint()`, from its own body, so it depended on the whole
picture and rebuilt its observer **once per animation frame**. `untrack(measure)`
fixes that — setup work that must not subscribe is wrapped, not avoided.

Three things made this expensive rather than obvious: the failure is a race (a
reload after the throw looked perfectly fine, so it read as webview flakiness); the
error never reaches the terminal (`svelte-check`, the tests and clippy are all
happy — an effect graph is not something any of them model); and a blank window
says nothing. §5's "When the window is blank" is the way in. **The rule to carry to
the kanji screens: an effect may read state or write it, and if it does both, one
of those two has to be `untrack`ed.**

### 10. `cargo test … | tail` throws the exit code away

A pipeline's `$?` is the **last** command's, so
`cargo test --workspace … 2>&1 | tail -60` reports `tail`'s status and a failing
suite reads as `exit=0`. That nearly recorded a broken layer as green here. Either
`set -o pipefail`, or send the output to a file and grep it, and check `$?` on the
unpiped command: the same run as a file showed `exit=0` honestly and 762 passing,
which is what the tailed form could not distinguish.

### 11. `curl: (56)` is a reset, not a missing file — and parallel output hides which URL you asked for

The kanji oracle is 2,136 files, so `fetch-data.sh` runs them through
`xargs -P 4`. At `-P 8`, `raw.githubusercontent.com` reset the connections instead
of answering, and curl reported the pair confusingly:

```
curl: (56) The requested URL returned error: 404
```

Error 56 is `CURLE_RECV_ERROR` — a *transfer* failure — and the 404 text came from
`--fail` reacting to a response that arrived before the reset. Eight processes
writing their own stderr interleaved the messages into noise, and the files were
there all along: fetching any single one by hand answered 200. Every download now
carries `--retry 4 --retry-delay 1 --retry-connrefused`, and at `-P 4` all 2,136
arrive with zero warnings. `--retry-all-errors` is deliberately **not** used — a
genuine 404 has to stay a 404, because a missing oracle file is something
`prepare-kanji` diagnoses properly by name.

The lesson is the file's own, one trap earlier: **fetch one of them by hand before
believing a bulk failure.**

**And check the variable in the URL.** The first run of that loop used
`$ANIMCJK` where it meant `$KANJIVG` — AnimCJK has no `kanji/` directory, its
kanji SVGs live in `svgsJa/` — so all 2,136 requests were genuine 404s for files
that do not exist, from a URL that looked entirely plausible in the interleaved
warning text. One hand-fetch of one character showed it immediately. When a bulk
fetch fails *completely*, suspect the URL you built before the upstream you
blame.

### 12. A UTF-8 BOM is not a syntax error, but `serde_json` reports it as one

JmdictFurigana's `JmdictFurigana.json` begins with a byte-order mark, and
`serde_json` answers with:

```
expected value at line 1 column 1
```

which says nothing about a BOM and sends the reader looking for a syntax error that
is not there. `prepare_words.rs` reads it through `read_json_stripping_bom`, which
strips `EF BB BF` before parsing. One line, and the difference between a confusing
half-hour and none.

Two related facts about the vocabulary inputs, worth knowing before touching the
fetch: `jmdict-eng.json` is **118 MB** uncompressed and `JMdict_e.xml` is **63 MB**
(10.6 MB gzipped), so `prepare-words` reads a great deal to produce 834 KB. That is
why the artifacts are committed and why no test ever touches `data/raw/`.

### 13. A word's okurigana is not an ungraded kanji — filter by `is_kanji` first

The vocabulary's membership rule is "every kanji is in the committed artifact", and
the obvious implementation — look up every character of the word in the kanji set
and reject the word if any is missing — is wrong in a way that produces a
*plausible* number rather than an error. Kana are not in the kanji set, so that
version rejects every word written with any kana: 食べる, 大きい, お金, all of them.
It reported **17,366** words where the answer is **21,902**, and nothing failed: the
pipeline ran, the artifact was written, and the count was a third too small.

Filter by `is_kanji` first, then check membership — and check the arithmetic against
a second, independent measurement, which is how this was found. The general shape:
**a filter that silently matches nothing looks exactly like a filter that found
nothing to exclude.**

The same trap has a second face in the same file: JMdict lists full-width numerals
such as `１０００` under `kanji`, so "has a kanji form" is not "contains a kanji".
Those 15 words have no kanji character at all and would land in band 1, because an
empty grade list falls back to it.

### 14. UniDic's `reading` field is the **base** form's reading, and the field next to it is pronunciation

Segmenting a passage looks like a two-line job until the furigana comes out wrong.
Two fields look like "how is this word read", and **neither is right on its own**:

| token | `reading` | `phonological_surface_form` |
| --- | --- | --- |
| 行き (行きます) | イク ✗ | イキ ✓ |
| 食べ (食べます) | タベル ✗ | タベ ✓ |
| 今日 | キョウ ✓ | キョー ✗ |
| は (particle) | ハ ✓ | ワ ✗ |

`reading` is the reading of the **base** form for an inflected word, so using it
draws 行き(いく) and 食べ(たべる)ます. The phonological field is the surface's, but
it is *pronunciation*: きょー for きょう, and ワ for a particle written は. The rule
that works is to compare `orthographic_surface_form` with `orthographic_base_form`
and use `reading` when they are equal, the pronunciation field when they are not —
and to let the **course's own reading win** when the token is written exactly as the
vocabulary word is, so a passage saying 私 cannot disagree with the card behind it
about わたし and わたくし. `tests/passages_artifact.rs` asserts all four rows.

Two smaller ones from the same afternoon. The link a tapped word opens is
`lexeme`, the dictionary form, because the surface is 行き and the entry is 行く;
and `lexeme` for 私 is `私-代名詞`, so the surface is the fallback rather than the
other way round.

### 15. `find` on a directory that does not exist kills a `set -e` script silently

`scripts/fetch-unidic.sh` opened with

```bash
found=$(find "$CACHE" -maxdepth 2 -type d -name unidic 2>/dev/null | head -1)
```

under `set -euo pipefail`. `find` exits 1 when the directory is not there, `2>/dev/null`
hides why, `pipefail` propagates the 1, and `set -e` aborts — **before a single line
is printed**. The log was empty and the exit status was 1, which is the least
debuggable shape a failure can have. The directory is now created first, the
pipeline ends in `|| true`, and the script says what it is doing before it does it.

The same evening's second version of that mistake was a name: the built dictionary
is at `.lindera/<version>-fmt<n>/lindera-unidic`, from lindera-unidic's own
`FetchParams::output_dir`. Both the script and `prepare-passages` were looking for a
directory called `unidic`, found nothing, and the pipeline reported a missing
dictionary that was sitting right there. Look for the name the producer chose, and
check for `metadata.json` so a half-written directory is not mistaken for a built
one.

### 16. The interface is driven from `index.html`, and a patch script that writes the wrong file looks exactly like a broken app

The recipe in §5 works and was used to verify every screen of N7: a temporary
`<script>` in `index.html` that clicks, types and reports over `fetch("/__probe?m=…")`,
plus a Vite middleware that appends each request to a log file. It is the only way to
answer "does tapping a word open the card?" from here — System Events is blocked
(trap 6) and Tauri does not forward the console.

Two ways it cost time.

**A blank window with an empty probe log is not a webview problem.** When the log has
*nothing* in it — not even the load line — the page never ran, which means the file
that builds it is broken. That is worth knowing because the symptom is identical to
trap 9's effect cycle and to a stale capture, and the fastest discriminator is
`svelte-check`: it named the broken file immediately. (The break, in this session,
was a patch script that read `index.html` and wrote the result to `App.svelte` —
`App.svelte` became an HTML document. `git show HEAD:…/App.svelte` plus re-applying
the edits fixed it in a minute, and the lesson is to have the script name both the
file it reads and the file it writes.)

**Captures lie when the window is not frontmost.** See §5: a `-l` capture can be a
stale backing store, and the DOM is the truth. A probe that reports `scrollY=0` with
the header at y36 while the image shows neither is a capture artefact.

### 17. A capture on a machine with no active display is frozen, and it looks current

N1's live check began with a window capture that looked perfect — the app rendered,
the course on the left, the board on the right — and was **wrong for the rest of the
session**. Three captures across twenty minutes, one of them after a Vite reload
that redrew the page, came back with the same md5. The window had been composited
once, at startup, and never again: `CGGetActiveDisplayList` answered `0` active
displays, so there was nothing to composite into.

This is worse than trap 4's version of the mistake. There, a region capture showed
another application, which is at least obviously wrong. Here the image is *of the
right window*, at a plausible scroll position, with plausible content — just not of
now. The DOM had moved on every time and nothing in the image said so.

```bash
python3 -c "import Quartz; print(Quartz.CGGetActiveDisplayList(8, None, None)[0])"
```

Zero means stop believing captures and use §5's HTTP probe, which needs no display
at all. Capturing twice and comparing is not a substitute: on a frozen window all
captures are identical, so the comparison that is supposed to reveal staleness
confirms it instead.

### 18. `tauri dev` leaves its Vite server behind, and the port is `strictPort`

Killing the app binary — or `tauri-cli`, or both — leaves the `vite:dev` child
running. The next `pnpm --dir apps/nihongo-tutor run dev` then fails with

```
Error: Port 1422 is already in use
Error The "beforeDevCommand" terminated with a non-zero status code.
```

and **no window opens at all**, which reads as the app having broken rather than as
a leftover process. `apps/nihongo-tutor/vite.config.ts` sets `strictPort: true` on
purpose (three apps, three fixed ports), so it will not quietly move aside.

```bash
lsof -nP -iTCP:1422          # who has it
kill $(lsof -t -nP -iTCP:1422 -sTCP:LISTEN)
```

One thing to know before tidying up: **under this harness the node processes are
hosted by the `DSH Desktop` binary**, so `lsof` and `pgrep` report them under that
name — `/Applications/DSH Desktop.app/Contents/MacOS/DSH Desktop … vite/bin/vite.js`
— and not as `node`. A `pkill node`-shaped cleanup finds nothing and a
`pkill -f "DSH Desktop"` would be catastrophic. Kill by the port's owner, which is
unambiguous.

---

## 7. Open decisions

Ordered by how much they block.

### The pitch-accent lexicon — research done, decision not taken

`ROADMAP_NIHONGO.md` N5. The short version: **no audit-clean pitch-accent
lexicon can be bundled.** Kanjium's `accents.txt` carries a genuine CC BY-SA 4.0
file and the author states in issue #13 that its source is withheld "due to
potential copyright issues"; joining it against Wadoku's dump shows **88.9% of
pairs identical including list order**. Wadoku and OJAD are openly non-free.
The clean route is to **generate** accent patterns — `tdmelodic` is BSD-3-Clause
— and the open verification is the licence and provenance of its **model
weights** and the training labels. `docs/research/JAPANESE_TUTOR_FEASIBILITY.md`
§6.1 has the full working.

### Whether kana want spaced repetition at all — decided, and shipped

**Decided: yes, but not because the kana want it, and the reasoning is invariant
25's and `ROADMAP_NIHONGO.md` N2's Shipped section.** An SRS over 179 kana would
not earn a screen against the drill's per-pair weights; what earns it is the kanji
course, which is where the years go and which has no other memory at all. So the
schedule is over every character a board can grade and the kana ride along.

The reusable *code* was real: `hanzi-core::progress`'s SM-2, its due-date
arithmetic and its attempt log are language-neutral and are used unchanged through
`Scheduler`. `hanzi-store` remains the answer to nothing here — invariant 15 — and
`fsrs-rs` remains the better algorithm that is not in, for the licence-and-weights
reason that section records.

### Where kana audio comes from

**The desktop half of this is decided and shipped** — N1, and its Shipped section in
`ROADMAP_NIHONGO.md`. macOS's own Japanese voices are what the app speaks with
(`Kyoko (ja-JP)` on the machine this was built on), through the same in-process
`AVSpeechSynthesizer` the other two apps already used, and the only thing the crate
needed was to be told which language it speaks (invariant 26). Nothing is
downloaded, nothing is bundled, and **the app has no audio files to license**.

What remains open is only the bundled clips, and only if they are wanted: they
would need generating — **MeloTTS-Japanese is MIT for both code and weights**,
which is the cleanest option — and Commons has only
about ten isolated kana clips with per-file licences. Nothing in the app needs them
today, so the decision can wait; if it is taken, invariant 26's "nothing is bundled"
changes and `licences.rs` grows a notice for the set.

### Whether this stays a separate app

It is `apps/nihongo-tutor/` with its own identifier `com.hanzitutor.kana`, which
follows the repository's multi-app pattern and keeps the binary small. A learner
studying both languages might want one app — but **the data half of this question
is settled**: the two apps' learner data is separate and stays separate
(invariant 15), so "one app" would mean one window over two stores rather than one
store. The shared *code* is what makes that cheap, and it is already shared. What
remains open is the shell, not the storage.

### The kanji level ladder — decided

The Japanese feasibility report §6.4: the JLPT publishes **no official kanji or
vocabulary list**, and the best-licensed community list chains to tanos.co.uk,
which asserts no licence, so bands are derived instead and the UI must say so.

**Decided, and shipped for the vocabulary in `words.bin.gz` (invariant 20):** a
word's band is the highest kyōiku grade among its kanji, 1–6, with band 7 for the
jōyō remainder, and EDRDG's `nf` rank orders the words within a band. The
alternative — frequency quantiles of roughly equal size — was rejected because a
band would mix grade-1 and grade-6 kanji.

**It was open whether the *kanji course* uses the same ladder or a different one,
and N8 settled it: the same one.** The course orders characters by kyōiku grade —
which is what `dictionaryJa.txt` supplies and what Japanese children actually learn
in — and its grade labels are `nihongo_core::grade_name`, which *is*
`words::band_name`, so "kyōiku 3" means one rung in both screens and a learner
meets a character and the vocabulary built on it in the same band (大人 arrives in
the grade-1 band, because 大 is grade 1 and 人 is grade 1). `curriculum.rs`'s unit
test asserts the two names cannot drift apart; a future divergence would have to be
a deliberate second ladder rather than an accident of two string tables.

### What the app should say to a learner who already reads Chinese

**A product question rather than a technical one, and the maintainer is best
placed to answer it because they read Japanese first and learned Chinese second —
the opposite direction to the one it is usually asked in.**

**First, the part that is not a question.** The transferability that matters
architecturally is the *mechanics*: counting strokes, pairing them and judging
order and direction is one software problem whatever the script, which is why
`nihongo-core` uses `hanzi-core`'s `geom`, `raster` and `grade` unchanged. **The
data is a separate matter and stays Japanese** — AnimCJK for kanji geometry,
KanjiVG as the stroke oracle, exactly as for the kana. The Chinese dataset is not
an input to the kanji course: not a source, not a cross-check, not a fallback.

**Then the product question.** The characters overlap enough that a Chinese reader
arrives with a real head start, and the app should say something about it rather
than pretend otherwise. The maintainer's summary of what actually carries over:

> In many cases he will be familiar with the **meaning**, but the **phonemes will
> be entirely different**, and sometimes even the meaning diverges. The only thing
> that is basically the same is **stroke order and shape**, and to a degree the
> meaning.

So: **shape and stroke order transfer; pronunciation does not transfer at all;
meaning transfers partly and unreliably.** The measurements behind that — 1,200 of
the 2,136 jōyō are the same simplified character the HSK course teaches, 702 exist
only as traditional forms, 234 are shinjitai with no counterpart, and the shape
agrees about nine times in ten where both exist — are in `ROADMAP_NIHONGO.md`
§"Why the overlap still matters", which is explicit that they inform a *screen* and
not a build.

The framing is probably "you have met this character — here is what Japanese reads
it as" rather than "you know this", and the 234 両 乗 亀 亜 仏 仮 伝 価 働 児 剣 労 単
厳 収 営 団 are worth knowing as a set, because they are where the head start runs
out.

**The non-negotiable part: glosses come from JMdict senses and never from a Chinese
dictionary.** Meaning transfers often enough to be tempting and not often enough to
be safe — 娘, 手紙, 汽車, 勉強, 丈夫 and 走る are actively wrong across the two
languages, and a Chinese gloss on a Japanese card is worse than no card.

**And the direction works both ways.** The maintainer's own path — Japanese first,
Chinese second — is served today by Hanzi Tutor itself, so anything that records
"this character is already known from the other language" must not assume which
language came first.

### The `assets/website` duplication

`assets/README.md` records that all twelve files in `assets/website/` also exist
byte-identical inside `assets/showcase/` — 10 MB carried twice. Plausibly
deliberate (a flat set is easier to upload from). Decide whether it stays a copy.

### The two Dependabot warnings that stand

`glib` 0.18.5 (unsound) and `proc-macro-error` (unmaintained, via `glib-macros`)
are one chain — `glib ← atk ← gtk ← muda/tao ← tauri` — and it is **GTK, so
Linux-only**: `cargo tree -i glib` prints nothing for the host target or
`aarch64-apple-darwin`. A fix needs gtk-rs 0.20, which needs a `muda`/`tao`/
`tauri` that has not shipped it. Accepted, and revisited only if the Linux
milestone happens.

---

## 8. The notices

Five, in `apps/nihongo-tutor/src-tauri/src/licences.rs`, with the text compiled
in by `include_str!` and copied into the bundle as plain text:

| id | What it covers |
| --- | --- |
| `agpl` | the app's own code, from the repository `LICENSE` |
| `provenance` | `LICENSES.md`, bundled as `licences/PROVENANCE.md` |
| `animcjk` | the kana and kanji geometry, the 214 radical head forms, **and the statement of what was changed** |
| `lgpl` | the licence the kana SVGs and the kana `graphics` file are under |
| `arphic` | the other licence that file could be under, per invariant 12 |

`tests/licences.rs` holds the three-way correspondence the main app's does:
every catalogued notice exists on disk *and* is the text compiled into the
binary, every file in the app's notice directory is catalogued, and
`tauri.conf.json`'s `bundle.resources` copies exactly that set. Two further tests
check that the AnimCJK notice actually **records the modification** — LGPL-3.0 §2
requires it — and that none of the notice files is gitignored, since a
gitignored notice vanishes from a clone and the bundle.

**N1 added none of them, and that is itself a licence decision**: the voice is the
operating system's, so there are no audio files to attribute. A notice for the
synthesiser would be a notice for something this app does not ship.

The app currently ships only `LICENSE`, `LICENSES.md` and the three notice files
above. **Before distributing it**, read `LICENSES.md`'s "Before you distribute":
the same rules apply, including the EDRDG update obligation that arrives with the
first KANJIDIC2 data.

---

## 9. What is deliberately not built

* **The words a character is used in, on the character's card — built.** It was
  the obvious next thing after N8 and it is now shipped: the card lists them in
  the course's order, a page of twelve, and each opens the word's own card
  (invariant 28). Recorded here because the bullet that *was* here is what the
  reader of an older copy remembers; `ROADMAP_NIHONGO.md`'s known weak spots had
  the same line and now records it shipped.
* **The kana chart and the yōon and voicing drills — built.** N4 shipped all three:
  a chart of the gojūon grid with its holes and the characters off it, every kana on
  it openable on the board, the 33 yōon contrasts of each script drilled against
  their own two-mora spellings, and the 25 voicing contrasts of each script drilled
  with the mark that makes them named (invariant 29). The chart is a lookup and not
  a lesson, which is why it has no romaji under each cell; the prose is in §2.
* **Kanji beyond the jōyō set.** 2,136 characters, their radical table and their
  course are shipped (N6, N8), and so is the vocabulary they are taught through
  (N7's `words.bin.gz`, 16,073 words with their own readings and furigana) and the
  three reading passages (`passages.bin.gz`, segmented by UniDic at build time).
  Jinmeiyō and hyōgai are deliberately out (invariant 16's refusal conditions and
  invariant 21's membership rule), and adding them moves the vocabulary's size and
  the course's shape with them. The feasibility work is in
  `docs/research/JAPANESE_TUTOR_FEASIBILITY.md`. **Two corrections this bullet has
  carried**: the KanjiVG-as-oracle trick *does* transfer for stroke counts — 2,135
  of 2,136 agree and 衷 is the written exception (invariant 19) — and "kanji need
  `dictionaryJa.txt`'s own grade sets" is the trap rather than the answer, since
  those are the pre-2017 grades and KANJIDIC2's are the authority (invariant 18).
* **Audio beyond the system voice.** N1 shipped the desktop half: a kana and a
  word's own reading are spoken with the machine's Japanese voice, by button and by
  the `h` key, and every reading on a kanji card is a control of its own
  (invariants 26 and 27). What is deliberately absent is **bundled clips** — nothing
  needs them while the system voice works. The drill is silent on purpose: hearing
  シ and ツ would answer the question the drill asks. §7 has the clip position.
* **Spaced repetition for the vocabulary.** N2's schedule is over *characters* —
  what a board grades — and it ships. A word is checked by typing, so it has no
  handwritten attempt to schedule; a word queue would need its own attempt source
  and its own decision about what a wrong reading costs. §7 and
  `ROADMAP_NIHONGO.md`'s known weak spots record it.
* **A per-learner confusability *matrix*.** The drill does now learn which of the
  13 pairs *this* learner gets wrong, and weights them (invariant 15, roadmap N3) —
  and N4's yōon contrasts are weighted and recorded the same way, so the record
  covers more than the thirteen. What is not built is a matrix: a screen that shows
  the learner what they mix up, or a drill that can *discover* a fourteenth kana
  pair. The record is there to build both on.
* **Grammar and particles.** "Kanji won't teach you to read" is the defining
  Japanese failure mode, and a kana tutor with no grammar is a kana tutor only.
  Out of scope for now, and the largest thing missing from the product.
