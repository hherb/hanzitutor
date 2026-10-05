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

Measured, not remembered. `cargo test -p nihongo-core -p nihongo-tutor` is **331
tests** — every `#[test]` in the two suites (206 in `nihongo-core`, 125 in
`nihongo-tutor`), plus two doc-tests (`lib.rs`'s and `variants.rs`'s);
the frontend's are counted separately below.
`pnpm run test:web` runs 187, of which **118** are this app's.
`cargo test -p hanzi-voice`, the shared crate the audio half lives in, is 29.

| | |
| --- | --- |
| Kana | 177 — 86 hiragana (U+3041–3096), 91 katakana (U+30A1–30FA plus ー) |
| Strokes | 516, average 2.92; あ is 3, not the 4 upstream stores (§4.2) |
| Joined hands | 331 of 339 single adjacent joins legible at 15/1024 jitter, 0 of 497 single omissions accepted (§4.30) |
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
| One reading | **はし is 橋, 端 and 箸 and かみ is five words** (上 紙 神 加味 髪) — the homophones the **Start here** screen argues with, drawn from the artifact rather than typed into the interface (invariant 32) |
| Audio | the machine's own Japanese voice — `Kyoko (ja-JP)` here — behind a **Hear it** button, the `h` key, and a control on **every reading of a kanji card**; nothing bundled, nothing downloaded (invariants 26 and 27) |
| Notices | 10 — five of them added at N9, when the EDRDG, JmdictFurigana and UniDic obligations that had been recorded in `LICENSES.md` since N6 and N7 were found to satisfy nothing in the bundle (§8) |
| Bundle | built at N9 as `Kana Tutor.app`, 15.25 MiB, and **rebuilt after the N10 rename as `Nihongo Tutor.app`** — 15 MiB, `codesign --verify --deep --strict` clean, `Identifier=com.hanzitutor.kana`, Team `X5DWXB4283`, and all ten notices world-readable. The `.dmg` needs Tauri's own `bundle_dmg.sh` run outside the sandbox (trap 21); the N10 one is **7,957,111 bytes**, `hdiutil verify` checksum VALID, signed `Identifier=Nihongo Tutor_0.1.0_aarch64`, and mounting it shows `Nihongo Tutor.app` plus the `Applications` link with the inner app still passing `--deep --strict`. The N9 figures were 15.25 MiB and an 8,186,456-byte image; the icon N9 drew has since been **redesigned at N14** — white field, the flag's red あ — and both the desktop set and the iOS set were regenerated from the new master, so a build from this tree wears a different face than the 15 MiB one described here; `/Applications/Kana Tutor.app` is still the N9 install, and the N10 build has not been installed there |
| Bundle (iOS) | a **development-signed** device build exists: `tauri ios build --debug --target aarch64 --ci` → `src-tauri/gen/apple/build/arm64/Nihongo Tutor.ipa`, installed on HHIP1 (iPhone 13 Pro Max, iOS 27) and driven by hand — 日 drawn on the board, graded 87/100 and "Saved for review", and the kana course seen on the device at N13. See §5, "Building for iOS", for the three things `tauri ios init` does not provide |
| Screens | **two courses**, one at a time, the last one remembered: *Kana* — Practice, Chart, Tell them apart, Review — and *Kanji* — Kanji, Radicals, Words, Read, Review, where **five screens are each two screens**: the course and the thing it opens — Kanji and Practice open a board, Radicals a family, Words a card, Read a passage (invariants 33, 34 and 35). **Start here** and Licences hang off the footer, in neither course (invariant 31) |
| Tests | nihongo-core 206, nihongo-tutor 125, hanzi-voice 29, frontend 118 of the 187 |
| Artifact | kana 70,917, kanji 3,236,713, words 601,816 and passages 502 bytes — all gzip + magic + postcard, all four embedded with `include_bytes!` |
| Kanji format | version 2 (`KANJD002`): the 214-radical table sits between the characters and the source (invariant 24) |
| Learner data | three files, `confusions.json`, `review.json` and `prefs.json`, in the app's own data directory |
| Version | both crates 0.1.0, and the app's to match |

The app runs and has been looked at, all of it: the course, the board, stroke-order
animation, handwriting grading, the discrimination drill, the typing box, the
katakana tab, the Licences panel and the review queue have each been seen working
on a display, and so have the two kanji screens (below). The review queue has also
been *used* — a character drawn on it was graded and rescheduled, which is not the
same thing as being seen. **The two-course division of N10 was driven rather than
looked at**: there was no active display in that session either, so the switch, the
per-course tab row, both halves of the queue, the remembered section and the
switch's two-line budget were all measured through the DOM probe in §5 — the recipe
that works when a capture cannot be believed. The screens listed in the paragraph
above were seen on a display when they were built; the header compaction is the
part that has only ever been measured, and it is recorded that way on purpose.
**Start here (N11) was measured before it was seen, and then seen on a phone.** The
§5 DOM probe drove the link, both readings and both buttons, and the
headless-Chrome harness in trap 23 measured the layout at 320, 360, 430, 500 and
980 px — and the first thing the maintainer did with the iOS build was **read it**,
which is how the romanisation claim in it was found to be wrong (N11's correction).
So its behaviour and its words have both been through a reader; its typography on a
desktop window still has not. **N12's stage has now been seen on the phone too** —
the maintainer drove it, and its two screenshots are what produced the two
corrections after it (the chrome stepping aside and the meaning folding in). The
rule that remains: a screen that has only been *measured* has not been seen, and a
reader finds what a probe cannot.

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
**Kanji** is the character course — and since N12 it is **two screens**: the course
itself (the grade chips and one card per lesson of ten, each card showing its ten
characters) and the board a card opens, where the readings sit above the board, the
tools below it, and the radical in both of its shapes, the IDS components **and the
words the character is written in** wait behind one `More` fold, each word opening
its own card; and **Radicals** shows all 214 head forms, each with the jōyō
characters classified under it, searchable and ordered by what a radical unlocks.
The kanji artifact was embedded and read by nothing for a whole milestone; N8 is
what read it, and the radical table it now carries is what makes the second screen
possible. Each milestone's own record says what its measurement corrected.

**And the board no longer sits at the bottom of the kanji course.** N12 split that
screen in two, because on a phone the board was *below* the lesson list and the
character grid: a learner scrolled past the course to write one character and back
to choose the next. The course is now the grades and **one card per lesson of ten**,
each card showing its ten characters — which is what the choice is between — and
tapping a card opens the stage: **the readings above the board, the board, and the
tools below it**, with everything else about the character behind one `More` fold.
Inside a lesson the arrows either side of the character are the only way on, the
board's faint guide became a **Hint** toggle (tracing or recall, Hanzi Tutor's), and
the tool row is Hanzi Tutor's icon row — `Icon.svelte` having been lifted
byte-identically from the Chinese app, as `render.ts` was before it. Invariant 33 is
the rule; the measurements, including the one that made the fold universal rather
than phone-only, are in `ROADMAP_NIHONGO.md` N12.

**And N13 gave the kana course the same shape.** It had the same complaint — the
script toggle, the lesson list, the lesson's kana and the board with the verdict,
the typing box and the confusions under it, stacked in three columns, so a phone made
a learner scroll past the course to write one kana — and N12 recorded it as the next
piece rather than doing it in passing. `KanaPanel` is now the course (the script
toggle, one card per lesson, each card carrying its own kana) and `KanaPractice` is
the stage (the sound above the board, the board, the tools below it, and what the
kana is written with, the typing exercise and the kana it is confused with behind one
`More`). The chrome steps aside while a stage is up, `Lessons` is the way back, and
the arrows either side of the kana are **the lesson of whatever kana is on the
board** — derived from the course on every change, because the confusions list
crosses lessons. Invariant 34 is the rule. Measuring it found two bugs, and one of
them was N12's: the board cleared itself on a Hint toggle without telling the screen
above it, so the verdict stayed up over strokes that had been discarded. The
measurements are in `ROADMAP_NIHONGO.md` N13.

**And N15 gave the last three screens the same treatment, and the tool row a thumb's
width.** N13 named both as the next piece: the **Radicals**, **Words** and **Read**
screens each drew one column, so on a phone the thing a tap asked for sat below or beside
the list the learner had to scroll back through. Each is now a course screen and a stage —
`RadicalsPanel`/`RadicalStage`, `VocabularyPanel`/`WordStage`,
`PassagePanel`/`PassageStage` — the chrome steps aside for all five two-screen panels, and
the arrows and the Grade button grew from 40px to the **44px** a finger needs, in both
existing stages at once. The panels report it with `bind:stage` now
(`bind:practice` was the name while the two screens that had a stage were the two
practice screens), `nav.ts`'s `STAGE_VIEWS` is the list of five, and `App.svelte` throws
for a sixth. Invariant 35 is the rule. Measuring it found the app's own tab row
overflowing by **13px** at a phone's width — a bug in the chrome that predates the
milestone and that none of the three new screens had — and `.views` wraps now. The
measurements, and what the three stages deliberately do not have (arrows, or a board),
are in `ROADMAP_NIHONGO.md` N15.

**And N16 is what N15's first device trials found, on an iPhone 13 Pro Max and an iPad
Air.** The maintainer's report: after grading, the way on was the lesson's arrows in the
top corner, and it should be the control beside Grade — Hanzi Tutor's arrangement, where
the commit button *becomes* `Next`. So the primary control under the board is now the
**verdict's own way on**: `Grade` while there is no verdict, `Next` once there is one,
`Finish` at the end of a lesson (there is no next, and the way on is the course), and
`Grade` again for a radical, which belongs to no lesson. The corner arrows keep their own
job, which is **skipping** — they step through the lesson without grading, so the primary
still reads `Grade` after a skip. It is one function, `board.ts`'s `afterGrade`, shared by
both stages and unit-tested in its four cases, and invariant 36 is the rule. The same
trials settled one thing *not* to change: the iPad's lesson cards wrap their kana at full
width, and the maintainer's verdict is that it *"makes good use of screen real estate"*, so
the track stays as it is.

**And N17 finished the rule for the third board, at the maintainer's word the same day.**
Review's list of what is due sits *above* its board, so after grading the next character
was a scroll away — the one screen whose whole subject is "what next". Its primary control
is now `Grade`, then `Next` (which opens the next due character), and **`Nothing due`**
when the queue is empty: the maintainer's wording, and a **state** rather than the stages'
`Finish`, because an empty queue is nowhere to go and nothing to go back to. It is
`board.ts`'s `afterGradeInQueue` with `review.ts`'s `nextDue`/`hasNextDue` — identity
rather than position, and the *queue* rather than the page — and it was measured against a
real `review.json`: three hand-written due kana graded one after another until the control
read `Nothing due`, with all three cards SM-2'd from 1–3 October to **13 October**.

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

**And a learner can now be told why the characters are worth learning before the
first lesson.** **Start here** — in neither course, hanging off the footer beside
Licences — says what the writing is for: the characters came from China and were
kept for their meaning, the kana fix the sound (**one character, one mora**) as well
as carrying the grammar, and Japanese's few distinct sounds and absent tones mean a
reading alone names several words. So romanisation answers neither question: it
loses which word was meant, and it is read with the reader's own sounds, which is
how an English speaker comes to say *hashi* as an English word. The word half is
shown rather than asserted, from the app's own vocabulary: **はし is 橋, 端 and 箸,
and かみ is five words**. Invariant 32 is the rule; the measurements are in
`ROADMAP_NIHONGO.md` N11, which also records the wording the screen first shipped
with and why it was wrong.

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

**And the board accepts a hand that joins strokes, which it did not before.** さ is
taught in three strokes and is very commonly written in two, き in three of its
four, and `ROADMAP_NIHONGO.md` had carried the worry in the maintainer's words —
*a kana tutor that rejects a legitimate hand is worse than no tutor*. The
measurement was blunt: `GradeReport::legible` requires the attempt's stroke count
to equal the reference's, so **0 of 339** adjacent joins of the 177 kana were
legible and every one of the six named variants failed. `nihongo_core::variants`
now grades a joined hand against a reference put into the same grouping, and
**331 of 339** hand-drawn joins are legible at 15/1024 jitter (259 at 30), every
named variant scoring 94–98. The eight it still refuses are the joins of a dakuten
stroke to the one beside it, named rather than tolerated. The risk it carries — a
dropped stroke arriving as "one fewer stroke" and being waved through as a join —
is measured too: **0 of 497** single omissions across every kana are accepted, and
**0 of 95** mid-stroke splits. The verdict panel says which taught strokes were
joined, because the per-stroke list under it numbers the *drawn* strokes. Invariant
30 is the part that must not be undone, and `crates/nihongo-core/examples/selfcheck.rs`
is where to re-measure it.

Both screens that grade were then watched, by probe rather than by eye because there
was no display (§5). The Practice board opened さ from the chart, drew the taught
three strokes as two with synthetic pointers and pressed Grade, and its verdict came
back `100 /100 legible … stroke 1: Correct stroke 2: Correct Taught strokes 1+2 were
drawn as one`; the Review panel repeated it from a hand-written due card, and with
`HOME` inside the workspace the schedule answered `Saved for review — next in 2 days`,
which is the half a sandboxed session normally reports as a failed write (trap 5).

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
  src/words.rs           (570)    Word, Ruby, WordDataset, the ladder's band_for
                                  and band_name, `of_kanji` — the vocabulary half,
                                  N7, and the character card's list, invariant 28 —
                                  plus `of_reading`, the homophones invariant 32's
                                  screen argues with
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
  src/review.rs          (563)    what a scheduled character is, the prompt beside
                                  it, which of the **two sections** teaches it, and
                                  the due queue that section sees — N2, N10. The
                                  schedule itself is hanzi-core's; this is the
                                  Japanese half
  src/variants.rs        (480)    a hand that joins strokes, and grading one: the
                                  regrouping, the per-stroke guard that tells a join
                                  from a dropped stroke, and the measurements
                                  invariant 30 pins. Kana only
  src/lib.rs              (89)    re-exports, including hanzi-core's grade, its
                                  decomposition parser and its SM-2 schedule
  examples/selfcheck.rs  (785)    the kana self-check: self-consistency, the jitter
                                  tolerance table, the classic pairs graded against
                                  each other, the joined-stroke and omission
                                  measurements, sampling, ink and cost. Prints; it
                                  asserts nothing — the tests pin what it finds
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
  tests/words_artifact.rs(511)    18 tests over the committed words artifact,
                                  recomputing every band from the kanji one, and
                                  pinning the homophones はし and かみ (invariant 32)
  tests/passages_artifact.rs(247) 9 tests over the committed passages, including
                                  that they still match data/passages/*.txt

scripts/fetch-unidic.sh           fetches and builds the UniDic dictionary into
                                  .lindera/ (gitignored). 134 MB down, 190 MB
                                  built, and the only thing that needs either

apps/nihongo-tutor/               the app
  src-tauri/src/lib.rs  (2900)    AppState, the speaker, the chart and the 31
                                  commands, all thin — and the one place a kana is
                                  graded through `nihongo_core::grade_kana` rather
                                  than the shared engine — invariant 30 —
                                  `words_of_reading` being the newest of them
                                  (invariant 32)
  src-tauri/src/store.rs (754)    the three files of this app's own: confusions.json
                                  (load, record, atomic write — N3), review.json
                                  (the SM-2 schedule, and the rule for when an
                                  attempt is a review — N2) and prefs.json (which
                                  course was open — N10). One generic read/write
                                  pair, because the ritual is the same for all
                                  three — invariant 15
  src-tauri/src/licences.rs(157)  the notice catalogue, with the text compiled in
  src-tauri/tests/ipc_contract.rs (1420)  46 tests locking the JSON the webview
                                  reads *and the arguments it posts* (invariant 14),
                                  including the `joined` field a joined kana crosses
                                  with — invariant 30 — its empty twin on a kanji,
                                  the `section` both queue halves are asked for, and
                                  the `{ reading, offset, limit }` the Start screen
                                  posts (invariant 32)
  src-tauri/tests/licences.rs     (246)  the three-way notice check
  src-tauri/licences/AnimCJK-COPYING.txt  the one notice specific to this app
  src/App.svelte         (465)    the two courses and the switch between them, the
                                  `chromeVisible` that stands the switch and the tab
                                  row aside while any of the five two-screen panels
                                  has a stage up (33, 34, 35) — the lookup that
                                  throws for a sixth, because `nav.ts` owns the list
                                  — the one place the voice status and the
                                  remembered course are asked for — N1, N10 — the one
                                  `openKana` a screen calls to put a kana on the
                                  board, carrying the script it is written in — N4,
                                  N13 — and the two footer links, Licences and Start
                                  here (31, 32)
  src/lib/nav.ts         (147)    **the division itself**: the two courses, their
                                  screens, their defaults, and the membership test
                                  `goTo` is guarded by — N10 — with Start here and
                                  Licences the two screens in neither course (32) —
                                  and `STAGE_VIEWS`, the five screens that are two
                                  screens (35)
  src/lib/nav.test.ts    (205)    run by the ROOT project's vitest; asserts every
                                  screen has one home, Review is the only screen
                                  two courses share, the two footer screens are
                                  in neither, and the five stage screens are exactly
                                  the five (35)
  src/lib/StartPanel.svelte(248)  **Start here** — what the writing system is for,
                                  and the homophone demonstration built from the
                                  app's own vocabulary — N11, invariant 32
  src/lib/start.ts        (41)    the readings the demonstration asks about, a page's
                                  size, and the one sentence that states a count —
                                  invariant 32
  src/lib/start.test.ts   (58)    run by the ROOT project's vitest
  src/lib/KanaCanvas.svelte(274)  pointer capture and the animation frame. Draws
                                  any `Drawable`, so a kana, a kanji and a radical
                                  all come through it; `guide` is the faint copy, and
                                  turning it off clears the attempt **and reports the
                                  emptied board** to the screen above — N12, N13
  src/lib/ConfusionDrill.svelte(392)  the drill: five exercises, one question, one
                                  answer, remembered — N3, N4
  src/lib/KanaChart.svelte(253)   the gojūon grid with its holes, and the characters
                                  off it — N4
  src/lib/KanaPanel.svelte(306)   **the kana course**: the script toggle and one card
                                  per lesson, each card carrying that lesson's own
                                  kana, and the request from another screen consumed
                                  once its script's course is loaded — N13, invariant
                                  34
  src/lib/KanaPractice.svelte(974)  **the kana stage**: one kana, its sound above the
                                  board, the board, the tool row below it, and the
                                  typing exercise and the confusions behind the
                                  `More` fold — N13, invariant 34 — with the arrows
                                  and Grade at the 44px a finger needs — N15 — and
                                  Grade becoming the verdict's own way on — N16,
                                  invariant 36
  src/lib/LicencesPanel.svelte(116)   the notices, fetched over IPC
  src/lib/ReviewPanel.svelte(471) what is due **in one course**, and a board to
                                  write it on — N2, N10 — with the joined-stroke
                                  line its verdict owes a hand that joined —
                                  invariant 30 — and the verdict's own way on, whose
                                  end state is `Nothing due` — N17, invariant 36
  src/lib/KanjiPanel.svelte(309)  **the kanji course**: the grade chips and one card
                                  per lesson of ten, each card carrying its ten
                                  characters — N8, and the first of N12's two screens
  src/lib/KanjiPractice.svelte(1277)  **the stage**: one character, its readings above
                                  the board, the board, the tool row below it, and
                                  everything else behind the `More` fold — N12,
                                  invariant 33 — with the arrows and Grade at the
                                  44px a finger needs — N15 — and Grade becoming the
                                  verdict's own way on — N16, invariant 36
  src/lib/Icon.svelte    (144)    LIFTED FROM THE CHINESE APP, BYTE-IDENTICAL — the
                                  board's tool glyphs, drawn rather than taken from
                                  an icon set — N12, invariant 33
  src/lib/RadicalsPanel.svelte(374)   **the radicals course**: the search, the two
                                  orders and the 214 rows — N8, N15
  src/lib/RadicalStage.svelte(254)  **the radical stage**: one head form, its number
                                  and stroke count, the family it heads, and the one
                                  button that writes it on the kanji board — N15,
                                  invariant 35
  src/lib/VocabularyPanel.svelte(319) **the words course**: the ladder and a page of
                                  one band — N7, N15
  src/lib/WordStage.svelte(108)   **the word stage**: one word's own card — N15,
                                  invariant 35
  src/lib/PassagePanel.svelte(174)  **the read course**: the passage cards, and the
                                  gloss deliberately not on them — N7, N15
  src/lib/PassageStage.svelte(271)  **the passage stage**: one passage with furigana,
                                  a tap on any word, and the tapped word's card
                                  below it — N7, N15, invariant 35
  src/lib/WordCard.svelte(265)      one word: its furigana, its reading, its band
  src/lib/SpeakButton.svelte(168)   the one "Hear it" control: a button for the
                                  three standalone callers, and a link for a
                                  reading on the kanji card — N1
  src/lib/kanji.ts       (166)    the course's and the radicals' arithmetic,
                                  `spokenReading`, and `neighbour`/`positionIn` —
                                  the arrows either side of the board — N8, N1, N12
  src/lib/kanji.test.ts  (203)    run by the ROOT project's vitest
  src/lib/kana.ts         (123)    which lesson a kana belongs to, what to call that
                                  lesson on a card, and how the arrows step through
                                  it — N13 — how wide the chart's grid is, whether a
                                  drill answer is one kana — N4 — and `joinedLabel`,
                                  the one line that renders which taught strokes were
                                  joined — invariant 30
  src/lib/kana.test.ts   (182)    run by the ROOT project's vitest
  src/lib/words.ts       (101)    furigana and paging arithmetic, as pure functions
  src/lib/words.test.ts  (131)    run by the ROOT project's vitest
  src/lib/review.ts      (131)    "2 days overdue" and "in 2 days", `nextDue` and
                                  `hasNextDue` — which due character comes next, and
                                  whether the *queue* has one at all — as pure
                                  functions — N2
  src/lib/review.test.ts (190)    run by the ROOT project's vitest
  src/lib/speech.ts      (123)    what can be heard, and which key asks for it, as
                                  pure functions — N1, and invariant 26's rules
  src/lib/speech.test.ts  (81)    run by the ROOT project's vitest
  src/lib/board.ts        (146)   the board's arithmetic, as pure functions — the
                                  pointer mapping, the sweep, the stray-tap filter,
                                  and `afterGrade`/`afterGradeInQueue`, the rules
                                  that put the verdict's own way on where Grade was
                                  (36)
  src/lib/board.test.ts   (150)   run by the ROOT project's vitest
  src/lib/render.ts      (547)    LIFTED FROM THE CHINESE APP, UNCHANGED (§4.6)
  src/lib/types.ts       (617)    the IPC shapes, `Character = Drawable`, the
                                  chart's slots as `(string | null)[]`, the
                                  `Prefs` the remembered course crosses in,
                                  `WordsOfReading` for the Start screen — invariant 32
                                  — and `KanaPick`, the script-carrying request that
                                  opens the kana stage — invariant 34
  src/lib/api.ts         (345)    one wrapper per command

crates/hanzi-voice/               SHARED with the two Chinese apps, and not this
                                  part's data layer
  src/speech.rs         (1644)    the system synthesiser and the voice rules — and
                                  the `Language` this app taught it (invariant 26)
  src/lib.rs              (48)    the re-exports; `capture`, the microphone half,
                                  is a default feature this app turns off
```

The line counts are the current tree's for the rows N11, N12, N13, N15, N16 and N17
touched —
`words.rs`, the two artifacts' tests, the app's `lib.rs` and `ipc_contract.rs`,
`App.svelte`, `nav.ts`, `nav.test.ts`, `types.ts`, `api.ts`, the three `start*`
files, the six kanji files (`KanjiPanel`, `KanjiPractice`, `Icon`,
`KanaCanvas`, `kanji.ts`, `kanji.test.ts`), the kana course's own
(`KanaPanel`, `KanaPractice`, `KanaCanvas`, `kana.ts`, `kana.test.ts`), the six
N15 touched (`RadicalsPanel`, `RadicalStage`, `VocabularyPanel`, `WordStage`,
`PassagePanel`, `PassageStage`), the two N16 touched (`board.ts`, `board.test.ts`,
plus the two stages above), and the three N17 touched (`review.ts`,
`review.test.ts`, `ReviewPanel`) —
and were last checked while writing invariants 26 and 27 for the rest; a few of the
earlier numbers were stale even then, which is worth knowing before treating one as
a measurement. N15's own six, N16's two and N17's three were counted from the tree
they wrote.

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

**And N12 lifted a second file the same way**: `apps/nihongo-tutor/src/lib/Icon.svelte`
is byte-identical to `src/lib/Icon.svelte`, because the kanji stage's tool row is
that app's tool row — a glyph on a tinted disc with its short name under it. It uses
only the names this app draws (`back`, `next`, `play`, `eye`, `eye-off`, `undo`,
`trash`, `tick`); the rest ship with it so that the two copies stay one file, and the
same rule applies. A third lift is a pattern rather than a coincidence, and the file
to check first when the two apps' interfaces are compared.

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

Nihongo Tutor writes exactly three things about its learner — the per-pair tallies
in `confusions.json`, the SM-2 schedule in `review.json`, and the remembered course
in `prefs.json`, all in its own directory under the app's data directory — and no
other app reads them, imports them, or shares a store with them. **The Japanese and
Chinese apps are separate products; what is shared between them is code, never user
data.** So `hanzi-store` is not the answer for the drill's tallies, and it is not
the answer for the review queue that `ROADMAP_NIHONGO.md` N2 plans either, however
convenient its language-neutral types look. That was the maintainer's decision, and
it is a boundary rather than an implementation detail: a shared store for two
separate apps is what turns "separate" into "tangled", and the tangle is discovered
on the day one app's schema change breaks the other's history.

The three files are also **three files** rather than one, and that is part of the
same invariant rather than tidiness: a mistake in writing which screen was open
must not be able to reach a schedule. `prefs.json` is the newest (N10) and the
cheapest, and it went in its own file for exactly that reason. The three share one
generic read/write pair in `store.rs`, because the ritual — a missing file is a
first run, an unreadable one is moved aside rather than overwritten, a write is a
temporary plus a rename — is the same for a preference as for a schedule.

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

---

### 30. **A hand that joins strokes is graded as the kana it drew, and a hand that drops one is not.**

The measurement that produced this is the kana `selfcheck` (§5), and the rule lives
in `nihongo_core::variants`. It is here because it is the one place this app does
**not** hand a character straight to the shared engine, and because the failure it
fixes was invisible to every test the app had.

**What was wrong.** `GradeReport::legible` requires the attempt's stroke count to
equal the reference's, so a connected hand was refused before shape was looked at.
Measured over all 177 kana: **0 of 339** single adjacent joins were legible against
the taught reference, and every one of the six variants `ROADMAP_NIHONGO.md` named
failed — さ written in two strokes scored 52, き's last two joined scored 60. さ is
taught in three strokes and two is the form most hands use; that is the maintainer's
own product rule, *a kana tutor that rejects a legitimate hand is worse than no
tutor*.

**The rule.** When an attempt arrives with **fewer** strokes than the kana is taught
with, every way of drawing the taught strokes as that many by joining adjacent ones
is graded, and the best accepted one is the verdict. Five parts of it are decisions
rather than implementation:

* **A regrouped form counts only when every drawn stroke clears the shared
  per-stroke bars** — `SHAPE_OK`, `POSITION_OK` and `INK_OK`, the same bars the
  grader uses to call one stroke faulty. Mean-based legibility *cannot* make this
  call, which is the trap: a missing stroke costs only its share of the average, and
  the omission measured **legible in 339 of 339** candidate groupings. With the bars,
  **0 of 497** single omissions across every kana are accepted, and **331 of 339**
  hand-drawn joins are, at 15/1024 jitter (259 at 30). `every_group_is_solid` is the
  function and the measurement is its doc comment.
* **The joins it refuses are named, not tolerated** — ぎ 5+6, ぜ 4+5, ぶ 5+6, ぷ 3+4,
  ズ 3+4, バ 3+4, ポ 2+3 and ヹ 1+2 — and every one joins a dakuten or handakuten
  stroke to the stroke beside it, which is not a form anyone teaches. A refused join
  falls back to the taught reference's verdict rather than being accepted as
  something it is not, and `variants`' test pins the list so a ninth is noticed.
* **An attempt with *more* strokes than the reference gets no regroup at all.** There
  is no honest way to split a taught stroke, so a hand that lifts mid-stroke is still
  told its count is wrong: **0 of 95** such splits are legible, in the selfcheck and
  in a test over the whole set. (An offcut below `min_stroke_len` is a stray tap, not
  a second stroke, and that case is accepted — the remaining piece covers the stroke.)
* **The grouping travels to the screen**, because the report's own stroke numbers are
  the *drawn* strokes: without it a per-stroke list would label the third drawn stroke
  of a joined き "3" when it is taught strokes 3 and 4. `grade_attempt`'s response
  gained `joined` (1-based, e.g. `[[3, 4]]`), `ipc_contract.rs` pins the request and
  the response — invariant 14's request half included — and `joinedLabel` in
  `src/lib/kana.ts` is the one line that renders it.
* **It is a kana rule, and kanji is excluded by decision rather than by omission.**
  A kanji keeps the **strict taught stroke count and order** and accepts no combined
  strokes: `AppState::grade` does the kana branch itself and sends a kanji or a
  radical through the shared engine directly, so their `joined` is always empty and a
  kanji drawn with two strokes joined is refused with the taught count. This is the
  maintainer's call, made explicitly — kanji stroke order *is* the thing being
  taught, and the Chinese app grades its characters the same strict way. It is
  therefore not a gap waiting on a measurement and not a flag to flip; the rule in
  `variants` is a kana rule.

**Nothing in the shared engine changed.** The count gate, the bars and the Chinese
app's grading are what they were, which is why its numbers did not move — the rule is
built *around* `grade_with_outlines`, not inside it. And `ROADMAP_NIHONGO.md`'s
record that き is taught in three strokes was wrong: the artifact says four, and the
selfcheck prints what the artifact says.

---

### 31. **The app is two courses, a screen belongs to one of them, and the switch says which.**

**Kana are the on-ramp and kanji are the product** — that ordering is the premise
`ROADMAP_NIHONGO.md` opened with, and N10 is where the interface stopped contradicting
it. The app was nine flat tabs in one row, so "Kanji" sat seventh in a list whose
first five entries were kana, the window was called *Kana Tutor*, and the bundle's own
description sold hiragana and katakana. The division is now explicit:

* **`src/lib/nav.ts` is the authority**, as data: two courses, their screens, their
  labels, their blurbs, and the first screen each opens on. `nav.test.ts` asserts the
  properties that make it a division — **every screen has exactly one home, Review is
  the only screen two courses share, and Licences and Start here are in neither.** A
  screen added to one course's template and to neither tab list fails there rather
  than shipping as a tab that shows nothing.
* **The section is passed beside the view, never derived from it.** Review is a screen
  of both courses, so a function from view to section has no honest answer for it;
  `goTo` takes both and **checks them against `nav.ts`** rather than assuming they
  agree, which is the one way the division could come apart with no type error.
* **The section scopes the review queue, and nothing else.** `nihongo_core::Section`
  decides which `ReviewKind`s a course is shown, and `review::queue` returns the due
  items, the card count and the next date **for that section** — a kana screen that
  said "12 scheduled" while eleven were kanji would be describing a queue it cannot
  show. The cards themselves are untouched: one file, one key per character
  (invariant 15), so the split is in what a screen offers and never in what is stored.
  `the_two_sections_partition_every_kind_of_character` pins that the two sections
  claim each kind exactly once, and `a_section_offers_only_what_it_teaches` that the
  two halves add up to the whole schedule.
* **Licences is an obligation, not a course, and Start here is about the language
  rather than about a set of characters.** Neither gets a tab in either course; both
  are links in the footer, reachable from both, and `nav.test.ts` names them as the
  two exceptions so a third has to be argued for there. Start here is the one screen
  a learner should meet before choosing a course, and its own two buttons are how it
  hands them to one — through `goTo`, the same guarded door every other screen moves
  through. Invariant 32 is what it says and how it proves it.
* **The app's name is not on screen, and the switch is two lines.** The window title
  carries the name on the desktop and the footer carries it at the bottom, so the
  `<h1>` is `sr-only` — present for a reader, 1×1 px for the eye, measured — and the
  old subtitle is gone. Each course button is exactly two lines: the name, then the
  tagline and the counts (`The on-ramp · 177 kana · 38 lessons`). `nav.test.ts` asserts
  the tagline is short enough to stay inside that budget, and the live check measured
  it rather than trusting it: **52 px per button at 980×728**, with a count line that
  was *longer* than the one that shipped not overflowing — which is the check that
  matters, because the one way "two lines" becomes three is a long count line at a
  narrow width.
* **The course last open is remembered** in `prefs.json` (invariant 15), and a first
  run opens on **Kana** — the default a new learner needs before any character. This is
  the one place the two-course split is a preference rather than a rule, and the file is
  its own so that a bad preference cannot damage a schedule.
* **The product is called `Nihongo Tutor` (N10) and the identifier is still
  `com.hanzitutor.kana`.** That is not an oversight: on macOS the identifier *is* the
  app's data directory, so changing it would strand the learner's three files under the
  old name. A name a person reads and a key a file lives under are two decisions, and
  `app_info_names_the_app_the_bundle_config_names` pins both so neither drifts.

---

### 32. **The Start screen argues from the artifact, and a reading is kana.**

The one screen in the app that is about the language rather than about a set of
characters, and the only one that makes a claim *for* the writing system instead of
teaching a piece of it. It is **N11**, and three things about it are decisions
rather than prose:

* **The evidence is the app's own vocabulary, never an example typed into the
  interface.** `WordDataset::of_reading` selects from readings the artifact already
  stores, and `words_of_reading` serves a page of them, so the demonstration cannot
  drift away from the data it rests on: **はし is 橋, 端 and 箸 and かみ is five
  words**, pinned by `tests/words_artifact.rs` against the committed artifact and by
  the app's own suite against the page. A reading printed from a literal in a
  component would be invariant 21's mistake — a reading the dictionary did not give
  — one step further out. `src/lib/start.ts` is the only thing in the frontend that
  knows anything, and it knows which readings to ask about and how to state a count.
* **The command takes kana and refuses anything else.** Every reading in the
  artifact is kana, so a romaji query matching nothing would be a typo dressed as a
  gap in the data — the same refusal `words_of_kanji` makes for a character outside
  the jōyō set (invariant 13, invariant 28). A **kana** reading the course does not
  carry is a legitimate empty page and the screen states it in words.
* **It is not the first-run screen, and it is not a gate.** A first run opens on
  **Kana**, which invariant 31 pins; Start here is a link in the footer beside
  Licences and is in neither course. Showing it on a first run would change a
  recorded default, and a learner who wants it knows where it is.

**Two smaller consequences.** The footer now carries two links, which is one more
than the `.utility` span was drawn for: they share one line at 980×728 and wrap on
a phone, which is what a footer is for, and `.utility` is a flex row for exactly
that. And the claim it makes is **not** a licence to invent Japanese: the screen's
prose is about the writing system and the evidence is drawn, so a hand-written
sentence about 山 or 食べる is the only kind of claim in this app that no test
pins — which is why the wording is short, and why the numbers in it (three words,
five words) are the artifact's.

**And that untested prose got a claim wrong on its first outing**, which is the
evidence for the paragraph above. The screen shipped saying romanisation "throws
away the grammar, which lives in the endings"; the maintainer caught it and it is
false — *taberu* romanises 食べる's ending perfectly well. What romanisation cannot
recover is the **meaning** the characters carry and the **sound** the kana fix.
N11 carries the correction; the lesson is that prose about a language is the one
thing here no test reads, so it needs a reader.

---

### 33. **The kanji course is two screens, and the board is the second one.**

The course was one page — grades, lesson cards, the ten characters of a lesson, the
board and the character's whole card, stacked in three columns. On a window that is
a lot of scrolling; **on a phone it is unusable**, because the board sits below the
lesson list and the character grid: a learner scrolls *past the course* to write one
character and back to choose the next. N12 splits it, and the maintainer's own
proposal is the shape that shipped.

* **`KanjiPanel` is the course**: the grade chips and one card per lesson of ten,
  each card carrying its ten characters. The title "1–10" names a range and says
  nothing about whether 日 is in it, so the cluster is on the card — and tapping a
  card is what opens the board.
* **`KanjiPractice` is the stage, and while it is up it is the whole screen.**
  The course switch and the tab row are the *course's*, so the panel tells
  `App.svelte` that a stage is up (`bind:stage`, named `bind:practice` until N15) and
  neither row is drawn: on a
  phone they cost the board about 150px, and `Lessons` — the stage's first control —
  is the way back. The footer stays; Licences and Start here are not the course's
  either.
* **The order on the stage is fixed: readings above the board, the board, then the
  tools below it.** Above the board is the *sound* — on, then kun, each its own
  control (`invariant 27`), or a radical's number and family size, which is the same
  kind of fact. Everything that is prose about the character — **including the
  meaning** — is inside the fold, so the row under the board is only what a board
  needs: Hint, Strokes, Undo, Clear, Grade — and Grade is the verdict's own way on
  once there is a verdict, which is invariant 36. Folding the meaning in was the
  maintainer's instruction and it is 92px of a phone's screen.
* **Inside a lesson, the arrows either side of the character are the only way on.**
  No strip of ten: it would be a second grid above a board whose ten characters are
  one arrow away. That was the maintainer's explicit choice (arrows only), not an
  oversight.
* **Everything else about the character is behind one `More` fold** — nanori, the
  radical, the IDS components, the words it is written in — which is Hanzi Tutor's
  arrangement. **The fold is at every width here, unlike Hanzi Tutor's phone-only
  one**, because that app's stage has a sidebar for the facts and this one is a
  single column: measured at the app's own 980×728, an open fold pushed the board
  to **y=1160**, which is the scrolling the milestone exists to remove. Folded, the
  board follows the readings at y=333.
* **The board's guide is a control now.** `KanaCanvas` gained `guide` — the faint
  copy on the board, or off while the learner writes from memory — and changing it
  **clears the attempt**, which is Hanzi Tutor's own trace/recall behaviour: half an
  attempt traced and half remembered answers neither question. It defaults to `true`,
  which is what the kana screens, Review and the radicals panel already had, so
  none of them changed.
* **`Icon.svelte` is lifted, byte-identically, from the Chinese app's** — the
  arrangement invariant 6 records for `render.ts`, and the same rule: if one copy
  changes, both change, or the divergence is written down. The glyphs are drawn
  rather than taken from an icon set because a kit would be a licence to notice for
  shapes this simple, and this app's notice list is pinned by a test.

**Measured, because the phone is the width it was rebuilt for.** The flow was
driven over §5's DOM probe with no display: seven grade chips, eight grade-1 lesson
cards with their clusters, the switch and tab row **absent from the DOM** while a
stage is up and back after Lessons, a card opening the stage on 日 at `1 / 10` with
the first arrow disabled, the board following the readings (`meta` y=136, board
y=254, controls y=585), the arrow stepping 日 → 一 to `2 / 10`, Grade disabled with an
empty board, a component tapped inside the fold opening at `2 / 10`, a **radical**
opening with no arrows at all (it belongs to no lesson), and Lessons returning to the
eight cards. Then the headless-Chrome harness measured the stage in the app's real
stylesheet at a phone viewport: no horizontal overflow, all five tools on one row,
`main`'s top padding **8px** rather than the desktop's 36, and the whole action —
readings, board, tools, Grade — **598px** tall in a 789px viewport, against 690px
before the meaning was folded in and the chrome stepped aside. Two touch targets were
under the 44px a finger needs (the back button at 25px, the fold at 40) and both were
fixed because of that measurement.

---

### 34. **The kana course is two screens too, and the stage is the whole screen.**

Invariant 33's shape, one course over. N13 is where the kana course got it, because
the complaint was the same one: the script toggle, the lesson list, the lesson's kana
and the board — with the verdict, the typing box and the confusions under it — were
stacked in three columns, so on a phone a learner scrolled *past the course* to write
one kana and back to choose the next. The rule, in the same parts:

* **`KanaPanel` is the course and `KanaPractice` is the stage.** The panel owns the
  script toggle, one card per lesson (each carrying that lesson's own kana), and the
  `bind:stage` report that tells `App.svelte` a stage is up; the stage is one kana,
  the sound above the board, the board, and the tools below it. `chromeVisible`
  stands the switch and the tab row aside while **any** of the five two-screen panels
  has a stage up (invariant 35), and
  `Lessons` — the first control on the stage — is the way back. Invariant 33's
  arrangement, applied to the other course rather than re-decided.
* **The order on the stage is 33's**: what you are being asked to write above the
  board (the kana, its reading, and the one control that says it), the board, then
  the tools — Hint, Strokes, Undo, Clear, Grade — and everything else about the kana
  behind one `More` fold, at every width. The fold is not a phone-only control, for
  33's measured reason. The last of those tools is the verdict's own way on once
  there is a verdict (invariant 36), which is N16.
* **The arrows either side of the kana are the lesson of the kana on the board, and
  the lesson is derived from the course on every change.** The confusions list
  crosses lessons — シ and ツ are different rows — so a stage that remembered the
  lesson it was opened with would offer the wrong arrows the moment one was tapped.
  `kana.ts`'s `lessonOf`, `neighbourIn` and `positionIn` are the arithmetic, and
  `lessonLabel` is what a card calls a lesson: a lesson's title is two halves and
  **which half is the name depends on the kind of lesson it is**, so it is read from
  the data (the half that is not `kana.join(" ")`) rather than by splitting the
  string — reading the first half as the kana called the small-kana group `Small kana`
  its lesson and ゃ ゅ ょ っ its sound.
* **A request that opens a kana carries the script it is written in.** `KanaPick` is
  `{ ch, script }`: the kana course is one course per script, so `KanaPanel` switches
  to the request's script and **waits for that script's course to be the loaded one**
  before honouring it. That is N4's `focusFor` rule made structural — a request
  naming only a character could otherwise open a katakana kana among the hiragana
  lessons — and `focusFor` is gone with the single-course design that needed it. The
  panel consumes the request rather than passing it down, so returning to the tab
  does not reopen what the learner left.
* **The board tells its screen when it empties itself.** `KanaCanvas` calls
  `onchange([])` when `guide` changes, because a verdict belongs to the attempt it
  judged: without it, Hint cleared the board while the verdict above it stayed up and
  `Grade` still held the strokes that had been thrown away. Every other path that
  empties the board — `undo` and `clear` — already told its caller. **The same bug
  was in N12's kanji stage**, which is why this is written as a rule about the board
  rather than a fact about the kana screen.

What is **not** changed: `nav.ts` and hence invariant 31's division (Practice is still
the kana course's first screen, and the stage is inside it), the review schedule and
its file (invariant 15/25), what may be handed to the voice (invariant 27 — the stage
speaks `kana.ch`, and `h` still only fires on the screen with a kana on the board),
and the joined-stroke rule (invariant 30 — the stage shows the same line, because it
is the same `joinedLabel`). The Radicals, Words and Read screens each still drew one
column when N13 shipped; **N15 is where they were treated this way**, and invariant 35
is that rule.

### 35. **Five screens are two screens, and the chrome steps aside for every one of them.**

Invariant 33's arrangement — a course and the stage one of its cards opens, with the
switch and the tab row taken off the screen while a stage is up — covered the two
practice screens from N12 and N13. **N15 extended it to the last three**, which is what
N13's "deliberately not done" had named as the next piece: **Radicals**, **Words** and
**Read**, none of which has a board.

* **The panels are the courses**: `RadicalsPanel` is the search box, the two orders and
  the 214 rows; `VocabularyPanel` is the ladder and a page of one band; `PassagePanel` is
  the passage cards. **The stages are what a card opens**: `RadicalStage` is one head
  form and its family, `WordStage` is one word's own card, `PassageStage` is one passage
  with its tokens and the tapped word's card below it. Every stage's first control is the
  way back, named for the list it returns to (`Radicals`, `Words`, `Passages`).
* **The prop is `stage`, and `nav.ts` owns the list.** It was `bind:practice`, which
  stopped being true the moment a stage could be a family or a passage;
  `STAGE_VIEWS` in `nav.ts` is the five, and `App.svelte`'s lookup **throws** for a sixth,
  so a screen cannot keep a stage flag without being listed there. One flag per panel:
  "See the family" moves the learner from the kanji stage to the radicals panel while the
  kanji panel's own stage is still up, and a single shared flag would leave the chrome
  hidden on a course screen.
* **These stages have no arrows, and that is a decision rather than an omission.** The
  kanji and kana stages' arrows step through a *lesson* — a sequence the course defines —
  and none of these three screens has one: a radical is found by number, by head form or
  by a character that uses it; a word is one of a page of twenty-four the learner is
  choosing from; a passage is read whole. The rule the kana chart is held to is the same
  one — **a chart is looked *up***, trap 23 — and none of these three is worked through
  either.
* **They draw no board either, by decision.** A radical's head form and each of its
  family members go to the **kanji** board — one board, one grading path, three kinds of
  drawable thing (invariant 13) — so `RadicalStage` hands the writing next door. A word is
  checked by typing and a passage by reading; neither has a handwritten attempt to grade.
* **The tab row wraps, because measuring the phone found it overflowing.** The five kanji
  tabs are 374px of button together and `.views` was a single-line flex row, so at 390×844
  — where the desktop webview's space-taking scrollbar leaves 373px of client width — it
  ran **13px past the page** and gave the whole app a horizontal scroll, on the course
  screens of all three new panels. The row is `flex-wrap: wrap` now and every screen
  measures 0 overflow. It is recorded here rather than only in the stylesheet because the
  class of mistake is the handover's own: **a probe that reads `innerText` cannot see a
  layout**, and this one was invisible at the desktop width where it had been measured
  before.
* **What did not change**: the vocabulary's readings and furigana, the schedule and its
  file, what may be handed to the voice, the joined-stroke rule, and Review — which still
  draws its queue above its board, is the same complaint, and is the next piece of this
  work rather than something N15 did in passing.

### 36. **The control under the board is the verdict's own way on, and the corner arrows are skipping.**

Found in N15's first device trials, in the maintainer's words: *"when grading a drawn
character, in order to get to the next character, the user has to move the pointer to the
top right corner. The 'Next' button should appear next to the 'grade' button after grading
similar as in HanziTutor — the top 'next' is for skipping a character."*

* **One rule, in `board.ts`'s `afterGrade`, used by both board stages** — so the kanji
  stage and the kana stage cannot disagree about what "graded" means for the control.
  The primary asks for a verdict while there is none; once there is one the same place
  holds the way on: **`next`** mid-lesson, **`finish`** at the lesson's last item (there
  is no next, and the way on is the course the stage was opened from), and **`grade`**
  again for a **radical**, which belongs to no lesson and has nothing to advance to.
  Hanzi Tutor does the same with its commit button, and its "Finish" at the end of an
  entry is where ours comes from.
* **Skipping is a different act, and the corner arrows keep it.** They step through the
  lesson **without grading**, so the primary still reads `Grade` after a skip — which is
  the whole distinction the maintainer drew, and the reason this is not "put a Next
  button in the corner's place unconditionally".
* **It is measured rather than reasoned about**: あ graded → `Next` → い; the corner arrow
  instead → う with no verdict and `Grade` back; ん, whose lesson holds one kana, →
  `Finish` → the course; 日 → `Next` → 一 in the kanji stage; a radical → `Grade` still.
  The run is `ROADMAP_NIHONGO.md` N16, and `afterGrade`'s four cases are unit-tested in
  `board.test.ts`.
* **The review queue has the same control, and its own end state** (N17). Its due list is
  *above* its board, so after grading the next character was a scroll away — the same
  complaint one screen further out, which N16 recorded as the next piece and the maintainer
  asked for the same day, with the wording of the end state. The rule is
  `afterGradeInQueue`, beside `afterGrade` in `board.ts`:

  * **`Next`** opens the next due character. `review.ts`'s `nextDue` answers it **by
    identity rather than by position** — the queue is ordered by how overdue each character
    is and that order moves as characters are graded, and the kind is part of the identity
    because 人 is both a jōyō character and radical 9;
  * **`Nothing due`**, disabled, is what the queue says when it has nothing left. That is
    the maintainer's own wording, and it is a **state** rather than the stage's `finish`:
    an empty queue is nowhere to go and nothing to go back to, where a finished lesson has
    a course behind it;
  * `hasNextDue` asks the **queue** rather than the page, so a queue with more due than one
    page holds does not read "nothing due" over characters nobody has fetched yet.

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

### The kana selfcheck

The Japanese counterpart of `hanzi-core`'s, and the measurement the tolerances are
supposed to stand on. It **prints** rather than asserts — what it finds is pinned by
`nihongo_core::variants`' tests, which is why the two must be read together.

```bash
./scripts/with-cargo-env.sh cargo run --release -p nihongo-core --example selfcheck
```

What it reports, and what a healthy run looks like:

* **self-consistency** — every one of the 177 kana grades 100.00 against its own
  strokes, with 0 below the ink bar and 0 Faint strokes. Anything else is a bug in
  resampling or normalisation, not in the data.
* **stroke lengths against `min_stroke_len`** — the shortest kana stroke is 157
  design units against a threshold of 12, so the Chinese-fitted constant discards
  nothing here. This is the row that would have caught a kana being eaten as a stray
  tap, and the reason it is first.
* **the jitter tolerance table** — legible 100.0% at sigma 5 and 15, 98.9% at 30,
  90.4% at 50, with the worst kana named (the voiced ones: が, ぐ, づ, ブ). The
  tolerances are the Chinese ones, and this is the number that says they carry over.
* **discrimination** — each classic pair member written against the other. **12 of
  26 directions are legible as the wrong kana**, and that is *recorded rather than
  fixed*: the board shows the character being drawn, so the learner is copying a
  visible shape, and tightening the tolerance enough to tell れ from わ would refuse
  the wobbly hands the tolerance exists for. It is here because it is the honest
  limit of geometric grading, not because it is a bug.
* **connected-stroke variants and the safety property** — invariant 30's numbers:
  331 of 339 joins legible at sigma 15, the 8 refusals named, 0 of 497 omissions
  accepted, 0 of 95 mid-stroke splits legible.
* **sampling robustness** — 0 verdict changes when the geometry is unchanged and
  only the sample density differs. This is the failure a jitter test cannot find.
* **ink** — a correct trace reaches 1.000 everywhere, and a third-width pen makes all
  177 not legible.
* **cost** — 0.12 ms per grade on the widest kana, against a 20 ms target.

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

# Find the window by TITLE. It is not the owner name: the owner is
# `nihongo-tutor` and the title is `Nihongo Tutor`, and they have been different
# from each other on both sides of the N10 rename — matching on the owner finds
# nothing.
WID=$(python3 -c "
import Quartz
for w in Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionAll, Quartz.kCGNullWindowID):
    if str(w.get('kCGWindowName')) == 'Nihongo Tutor':
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

To see a screen that needs a click, change the initial values of `section` and
`view` in `App.svelte` **together** — a `view` the section does not have is refused
by `goTo`'s guard (invariant 31) — and **restart the app**: see trap 3, HMR
preserves state.

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
* **And a layout can be measured with no Chrome and no capture at all** — point the
  app's own window at the phone's size and read the numbers out of the page. Trap 23
  closes with the recipe and with why the harness was not available to N13.
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

**And on the iPhone the Ring/Silent switch is the first thing to check.** The first
on-device report of the N12 stage was "tapping a reading should speak"; the feature
had shipped at N1, and the phone was simply on silent — from the stage, tapping a
reading does speak. `engage_session` sets `AVAudioSessionCategoryPlayback` precisely
so that an explicit tap is heard through the switch, and its comment says so, but
**activation is asynchronous and nothing waits for it**, which that same comment
records as a deliberate trade: the main-thread hang risk against a session that
"costs volume, not words". So a tap made while the session is still activating can
be silent with the switch on, which is what the maintainer met. **Ask about the
switch before instrumenting anything** — and if a build really is silent, the probe
is a temporary `eprintln!` in the app's `speak` command plus
`xcrun devicectl device process launch --console`, which streams a bundled app's own
stderr and is the only place a phone build can say what it did. (The build with that
probe in it was never installed, and was deleted rather than left to be handed to
someone.)

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
DIR=~/Library/Application\ Support/com.hanzitutor.kana
cat "$DIR/confusions.json"    # the drill's tallies, keyed "シ|ツ"
cat "$DIR/review.json"        # the SM-2 cards, keyed by character
cat "$DIR/prefs.json"         # just {"section": "kanji" | "kana"}
```

It is written after every answer, pretty-printed, and keyed by the pair
(`"シ|ツ"`, lower code point first) — so a change to the weighting rule can be seen
in the numbers rather than inferred from which kana came up. To prove that a
restart reads it back, answer a few questions, quit, start again, and answer a few
more: a pair's `asked` must continue from where it left off rather than restarting
at 1. That is also the check the tests do over a temporary directory, and it is the
one worth repeating by hand after touching `store.rs`. `prefs.json` is the same
check one line long: switch to Kanji, quit, start again, and it must open there —
and its `section` is what the app reads, so a section that is not `kana` or `kanji`
opens the on-ramp rather than throwing.

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

### Building for iOS and putting it on the phone

The app has a mobile shell as of N10 — `tauri ios init` has been run, and the
generated project is `apps/nihongo-tutor/src-tauri/gen/apple/`. The recipe is the
same as `HANDOVER.md` §6 gives for the Chinese app, and so are its two hard
constraints: **`xcodebuild` cannot run inside the file sandbox** (DerivedData, the
module cache and the profiles live under `~/Library`, and the Rust step's `swift-rs`
build fails with `sandbox_apply: Operation not permitted`), and **the phone must be
unlocked** for the install and the launch.

```bash
cd apps/nihongo-tutor
rm -rf src-tauri/gen/apple/build          # a stale archive cannot be replaced
APPLE_DEVELOPMENT_TEAM=X5DWXB4283 TAURI_ROOT="$PWD" \
  ../../scripts/with-cargo-env.sh ../../scripts/tauri-cli.sh ios build --debug --target aarch64 --ci

cd src-tauri/gen/apple/build && mkdir -p payload && cd payload
unzip -q "../arm64/Nihongo Tutor.ipa"    # a copy also lives beside the .dmg:
                                         # .cargo-target/release/bundle/ios/
D=00008110-0010252A0293801E               # HHIP1, the iPhone 13 Pro Max
P=00008103-001904501153001E               # Horst's iPad; `xcrun devicectl list devices`
xcrun devicectl device install app --device "$D" "Payload/Nihongo Tutor.app"
xcrun devicectl device process launch --device "$D" com.hanzitutor.kana
xcrun devicectl device capture screenshot --device "$D" --destination /tmp/phone.png
```

**A device that has never been paired is visible but not installable**, and that is
not one of this app's own three things — it is the first thing to check when a second
device joins. An iPad attached for the first time showed `available` in
`xcrun devicectl list devices` where the phone showed `available (paired)`, and the
install answered *"The device must be paired before it can be connected"*
(`RemotePairingError 2`), which names neither the device nor the fix. `xcrun devicectl
manage pair --device <UDID>` pairs it in one step — no Xcode window, no prompt — and
the install then proceeds. The word to read in the listing is `(paired)`; one build
serves both devices while the profile is the team's wildcard, which is what
`ios build --ci` picks.

Three things are this app's own, and the first two are the ones that stop a build:

* **`gen/apple/tauri` is not generated.** The Xcode *Build Rust Code* phase runs
  `node tauri ios xcode-script …` from `gen/apple`, so without that shim the build
  dies in `PhaseScriptExecution` with `Cannot find module '…/gen/apple/tauri'`.
  It is a committed one-line forwarder to `@tauri-apps/cli/tauri.js`, copied from
  the Chinese app's, and re-running `ios init` deletes it.
* **AVFAudio has to be named in `gen/apple/project.yml`.** The shared speech code
  engages the iOS audio session, and only this app lacks the `cpal` objects whose
  clang autolink hints would pull the framework in — it builds `hanzi-voice`
  without its `capture` feature on purpose. Without the line the link ends in
  `Undefined symbols: _AVAudioSessionCategoryPlayback`. `cargo --print
  native-static-libs` *does* list `-framework AVFAudio` for this app, but nothing
  hands that list to Xcode. `ios init` regenerates `project.yml`, so the line is
  documented here rather than only in the file. After editing it, regenerate with
  `xcodegen generate` **in `gen/apple`** — `ios build` reads the `.xcodeproj`, not
  the YAML.
* **The iOS icon set is not the app's either** — `ios init` writes Tauri's template
  logo into `gen/apple/Assets.xcassets/AppIcon.appiconset/`, and it did so here
  *after* N9 had generated the desktop icons, so the phone drew the template's "T"
  until N14. It is one command, run from `apps/nihongo-tutor/` whenever the artwork
  changes or the iOS project is re-initialised (and `--ios-color` is what fills the
  transparent corners, because an iOS icon has no alpha):

  ```bash
  TAURI_ROOT="$PWD" ../../scripts/with-cargo-env.sh ../../scripts/tauri-cli.sh \
    icon app-icon.png --ios-color "#ffffff"
  ```

  `store/README.md` records the same obligation for the Chinese app and the same
  consequence of skipping it. **Nothing in the tree names the iOS set**, which is
  why this is a step to remember rather than one a check would catch; the artwork is
  `apps/nihongo-tutor/app-icon.png` and its generation is `ROADMAP_NIHONGO.md` N9
  and N14.
* **`src-tauri/Info.ios.plist`** carries the scene manifest iOS 26 and later
  require; the CLI merges it at build time, and the built app's `Info.plist` is
  where to check it landed. There is no microphone string here, unlike the Chinese
  app's: this app never records.

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

### 19. Tauri's two version numbers live in different files, and only a bundle notices

`pnpm --dir apps/nihongo-tutor run build` failed before it compiled anything:

```
Error Found version mismatched Tauri packages. Make sure the NPM package and
Rust crate versions are on the same major/minor releases:
tauri (v2.12.0) : @tauri-apps/api (v2.11.1)
```

The app has its **own** `package.json` and its **own** `pnpm-lock.yaml`, so its
`@tauri-apps/api` and `@tauri-apps/cli` are pinned independently of the repository
root's — while the Rust side resolves through the workspace's shared `Cargo.lock`,
where `tauri` had moved to 2.12. `cargo test`, `cargo clippy`, `svelte-check` and
`vitest` are all silent about this: nothing but a `tauri build` compares the two
numbers, so **the app had no signed bundle at all** from the day the pin drifted
until N9 checked.

`apps/tone-trainer` was in exactly the same state, for the same reason and through
the same shared lockfile — so this is a property of the three-app layout, not of
this app. When either the Rust `tauri` version or an app's lockfile moves, check
the other side; `pnpm --dir apps/<app> run build` is the only cheap way to see it.

**And a third one, found by asking the same question, which is why it is here.**
`scripts/tauri-cli.sh` probes `"$ROOT/node_modules/@tauri-apps/cli/tauri.js"`
before falling back to a `cargo-tauri` on the PATH, but `TAURI_ROOT` comes in
**relative** — `apps/nihongo-tutor`'s own `package.json` sets `TAURI_ROOT="$PWD"`,
which is the app directory, not the repository root — and `$ROOT` was still
relative when the probe ran. So the check looked under
`apps/nihongo-tutor/apps/nihongo-tutor/node_modules/…`, failed, and the fallback
was used: a CLI reporting **2.11.1** against the app's own pinned **2.12.1**, which
is a silent use of a different tool than the one the app names. `tauri info` shows
it in one line when `TAURI_ROOT` is relative (no `@tauri-apps/cli` row at all) and
does not when it is absolute. `ROOT` is now resolved once, before anything reads
it, and the row is back. Nothing about this is DSH-specific; it is an ordering bug
of the same shape as trap 8's "suspect the loop before the table".

### 20. The bundle's file modes are the source files' modes, and the fix can be skipped

Five of the ten licence texts in `Kana Tutor.app/Contents/Resources/licences/` —
the bundle as it was named then; N10 renamed it, the paths below are current —
were `-rw-------` — readable by the machine that built the bundle and by nobody
else, which is a redistributor's problem with a licence-compliance flavour (a
notice a recipient cannot read is not a notice they received). Tauri copies a
resource with the modes it finds, and this checkout's files were `600`.

`scripts/build-release.sh` already had a `chmod -R a+rX` for exactly this, and it
did not run — because it sat **inside** the script's success branch, and `tauri
build` writes and signs the `.app` *before* it attempts the `.dmg`. `.dmg` creation
is refused by this harness (trap 21), so the build returned non-zero with a
complete, freshly signed `.app` on disk and skipped the one line that makes it
shippable. The `chmod` now runs whenever the bundle exists, not whenever the build
succeeds, and the licence directory in particular is re-checked by name.

The check that catches it costs one command, and it is worth keeping in the loop
for a release:

```bash
ls -l ".cargo-target/release/bundle/macos/Nihongo Tutor.app/Contents/Resources/licences/"
```

Every line must be `-rw-r--r--`.

### 21. Building the `.dmg` — two failures that look identical and are not

The transcripts in this trap are from N9, when the product was still named `Kana
Tutor`; the commands are unchanged, the file names in them are not current. N10
renamed the product and did **not** rebuild a bundle, so a `.dmg` after it will be
`Nihongo Tutor_0.1.0_aarch64.dmg` — the identifier inside is still
`com.hanzitutor.kana` (invariant 31).

`pnpm run build` ends with

```
Error failed to bundle project error running bundle_dmg.sh
```

while the line above it says `Signing with identity "Developer ID Application: …"`.
The `.app` is complete and signed; only the disk image is missing. **Tauri swallows
the script's output**, and with `RUST_LOG=tauri_bundler=trace` the real message
appears. There are **two** different ways this ends, and an earlier draft of this
trap collapsed them into one — do not repeat that.

**Cause 1 — the agent harness's sandbox, and it is `hdiutil` rather than Finder.**
This is what the trace shows first:

```
Running Command `…/bundle_dmg.sh … "Kana Tutor_0.1.0_aarch64.dmg" "Kana Tutor.app"`
hdiutil: create failed - Operation not permitted
```

It fails on the first `hdiutil create` and for *any* source folder, so it is not
about this app's size or configuration:

```bash
mkdir -p /tmp/probe/src && echo hi > /tmp/probe/src/hello.txt
cd /tmp/probe && hdiutil create -srcfolder src -volname T -format UDRO rw.dmg
# → hdiutil: create failed - Operation not permitted
```

Widening the sandbox makes that same command succeed, so this is the file sandbox
and nothing else. **It is not the Finder-prettifying AppleScript** — with the
sandbox widened the script reaches `Running AppleScript to make Finder stuff
pretty`, `Done running the AppleScript` and `Fixing permissions`, so the automation
is available and was never the obstacle.

**Cause 2 — `Resource busy` on unmount, which is ordinary and self-inflicted.** The
run gets all the way through the layout and then:

```
Unmounting disk image...
hdiutil: couldn't unmount "disk36" - Resource busy
The volume can’t be ejected because it’s currently in use.
```

**The commonest reason on this machine is that the volume is being used — a Finder
window open on it, or the app launched from it.** That is not hypothetical: two
attempts in a row left `dmg.DhcY23` and `dmg.zHi9AK` mounted for exactly this
reason, and the operator was looking at and running the mounted copy at the time.
Check first, then retry:

```bash
mount | grep -i /Volumes                           # what is left over
hdiutil info | grep -E '/dev/disk'                 # the device names
hdiutil detach -force "/Volumes/dmg.XXXXXX"        # or: diskutil eject
```

Only a *detach* is needed; the intermediate image is still there, so the run can be
repeated without rebuilding the app.

**Building it, which is what to do when someone needs the installer.** The script
Tauri generated is self-contained, and running it outside the sandbox produces the
image Tauri would have produced. Three steps, in order, from
`.cargo-target/release/bundle/dmg/`:

```bash
# 1. Tauri's own script, with the arguments the trace log prints.
./bundle_dmg.sh --volname "Kana Tutor" --icon "Kana Tutor.app" 180 170 \
  --app-drop-link 480 170 --window-size 660 400 --hide-extension "Kana Tutor.app" \
  --volicon "$PWD/icon.icns" "Kana Tutor_0.1.0_aarch64.dmg" "../macos/Kana Tutor.app"

# 2. Hide the stray volume icon and sign the image — the helper only signs when the
#    identity is set, and step 1 produced an unsigned image. Give it an ABSOLUTE
#    path: `../../..` from this directory is `.cargo-target/`, not the repo root.
APPLE_SIGNING_IDENTITY="Developer ID Application: Horst Herb (X5DWXB4283)" \
  "$REPO/scripts/hide-dmg-volume-icon.sh" "Kana Tutor_0.1.0_aarch64.dmg"

# 3. Verify.
hdiutil verify "Kana Tutor_0.1.0_aarch64.dmg"    # checksum VALID
codesign --verify "Kana Tutor_0.1.0_aarch64.dmg" # satisfies its Designated Requirement
```

Measured once through: 8,186,456 bytes, `hdiutil verify` VALID, image signed
`Identifier=Kana Tutor_0.1.0_aarch64`, `TeamIdentifier=X5DWXB4283`, mounting with
`Kana Tutor.app` and the `Applications` link, the app inside still passing
`codesign --verify --deep --strict`, and all ten notices under
`Contents/Resources/licences/`. **The `--bundles app` shortcut does not produce that
app** — see trap 22 for why, because it is the trap that bites when you take the
shortcut on purpose.

**One more thing to know before starting.** `bundle_dmg.sh` leaves the *uncompressed*
volume image (`rw.<pid>.<name>.dmg`, tens of MB) in that directory while it works.
Its presence means a run started and did not finish; the finished artifact is the
small `Kana Tutor_0.1.0_aarch64.dmg` and nothing else.

**The Chinese app is not a precedent here, and it is worth knowing why.** The one
`Hanzi.Tutor_0.6.0_aarch64.dmg` in this checkout was built on 27 September from the
tree saved in `.tmp-release-060/`, before the `tauri` 2.12 upgrade — nothing in the
current tree had produced a `.dmg` under this harness before this milestone. So this
was never "the Japanese app cannot do what the Chinese one can"; it was the first
attempt from either app since the toolchain moved.

### 22. `tauri build --bundles app` writes an **ad-hoc, linker-signed** bundle that fails verification

The obvious shortcut when only the `.app` is wanted — and the one
`build-release.sh`'s own error message recommends — is:

```bash
TAURI_ROOT="$PWD/apps/nihongo-tutor" ./scripts/with-cargo-env.sh \
  ./scripts/tauri-cli.sh build --bundles app
```

It exits 0 and prints `Finished 1 bundle at: … (15.25 MiB)`. **The bundle it leaves
is not signed by Tauri.** `codesign -d` on it says:

```
Identifier=nihongo_tutor-f8564bfd63853d4c      ← the *binary's* name, not the app's
flags=0x20002(adhoc,linker-signed)
Sealed Resources=none
```

and there is **no `Contents/_CodeSignature` directory at all**, because what is
there is the ad-hoc signature `rustc`'s linker put on the executable, carried into
the bundle. `codesign --verify --deep --strict` exits **1**:

```
code has no resources but signature indicates they must be present
```

The full `pnpm run build` signs properly and the same commands then read
`Identifier=com.hanzitutor.kana`, `TeamIdentifier=X5DWXB4283`,
`Sealed Resources version=2 rules=13 files=11`, exit **0**. So the two paths differ
in whether the app is signed at all, and **the failure is silent in the build log**:
the only difference in the output is the absence of the
`Signing with identity "Developer ID Application: …"` lines.

What follows: after any `--bundles app` build, **re-run the full build before
judging a signature**, and never hand that bundle to anyone. It is fine for a local
smoke test and for checking that a resource landed.

**And beware the exit code you read.** `codesign --verify … | tail -1` reports
*tail*'s status, exactly as trap 10 says about `cargo test … | tail`. The bundle
above "passed" for one round of checking because of that pipe.

### 23. A class name in `app.css` reaches a component, and a probe cannot see a layout

`app.css` is a plain stylesheet imported by `main.ts`, so its selectors are global.
Svelte scoping adds a hash class to **this component's** selectors; it keeps nothing
*out*. A name is therefore shared whether that was meant or not.

`KanaChart.svelte` drew the chart into `<div class="grid">` with no rule of its own,
and `app.css` owns `.grid` for the practice picker:

```css
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(56px, 1fr)); gap: 6px; }
```

Applied to the chart's wrapper that made the sixteen **rows** the grid's items:
fifteen of them on one line in 57px columns, two on the next. Each row's own grid —
`2.6rem repeat(5, …)` — then had ~72px of content to fit into 57px, so its five cells
measured **2px wide** and 24px kana glyphs drew on top of one another. The symptom
was the whole chart "all squashed together", while the off-grid strip below it — the
same `.cell` class, but *outside* `.grid` — was perfect, which is what sends a first
look at the CSS in the wrong direction.

**It survived a whole milestone because that milestone was checked by probe.** N4's
verification read the chart over `/__probe` (§5) and found sixteen rows, the right
kana in the right columns, every one of them openable. All of it was true —
`innerText` is not layout. A screen that has only ever been *reported* has never been
*seen*, and this is the general shape of that mistake rather than a chart bug.

What finds it, cheapest first:

* `getComputedStyle(box)` — the chart's wrapper answered `display: "grid"` and
  `gridTemplateColumns: "57.45px 57.47px … ×15"`, neither of which appears in this
  component's stylesheet. That one line is the whole diagnosis.
* `getClientRects()` on the cells: 2×26 for a control whose own rule says
  `aspect-ratio: 1 / 1`.
* **The §5 HTTP probe is the right tool for "did the press reach Rust" and the wrong
  one for "is this readable".** Layout needs an engine that lays it out.

The recipe that did it needs no display: a temporary second Vite entry
(`apps/nihongo-tutor/chart-harness.html`) that stubs the IPC, mounts the component on
its own and writes `getBoundingClientRect` numbers into the page, so `--dump-dom`
answers with measurements rather than an image. `window.__TAURI_INTERNALS__ = {
invoke: (cmd) => Promise.resolve(FAKE[cmd]) }` is the entire stub — `invoke` from
`@tauri-apps/api` is a call to that and nothing else. Two details:

```bash
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new \
  --virtual-time-budget=5000 --dump-dom http://localhost:1423/chart-harness.html
```

* **Serve it with a config whose `server.hmr` is false, on its own port.** An open
  HMR socket keeps `--virtual-time-budget` from ever expiring, so `--dump-dom` hangs
  with no error and no output — which reads exactly like a broken harness.
* **Delete the harness.** `frontendDist` is `../dist` and the release build embeds
  it, so an entry left in the app's root is a file in the shipped bundle.

**Three things the harness cost on its second use (N11), all of them this harness
rather than this app.** Chrome must be given a profile **inside the workspace** —
`--user-data-dir="$PWD/.tmp-chrome"` — or it dies on Crashpad's first write to
`~/Library` under the file sandbox, and the failure looks like Chrome producing no
output at all; the dev server is bound **IPv6-only** here, so the URL is
`http://[::1]:1423/…` and `localhost` may resolve to the refused `127.0.0.1`; and
**Chrome will not open a window narrower than about 500px**, so a phone width is
measured by constraining the *container* (`target.style.width = "360px"`) inside a
wider window rather than by asking for a 360px window — which the reported
`innerWidth` will otherwise quietly lie about. The first two were a silent no-output
run and a `NO REPORT`, and none of the three is a bug in the app being measured.

And the second half of the fix is not optional. With the collision gone the row's
`minmax(0, 1fr)` tracks stretched a cell to **173px** across a 980pt window — a 24px
kana in a 173px box, sixteen rows of it, 2890px of scrolling. A chart is looked *up*.
The tracks are capped at the off-grid strip's own `3rem` and the rows are
`width: fit-content`; because a capped track is still `minmax(0, …)`, a narrow
window shrinks the rows instead of overflowing them, and because the row's box now
stops at its last cell, the voiced rows' accent band marks the row rather than
running on across the empty half of the panel. **Check the rendered result and not
only the corrected rule** — the first fix above was correct and still wrong.

**And on a machine where headless Chrome will not run at all, the app's own webview
measures the same thing.** N13's check had no harness: `--dump-dom` hung on *every*
page, including a bare `data:text/html,<h1>hi</h1>`, with Chrome 154 under this
harness's sandbox — the profile, the crash reporter and Chrome's own process sandbox
are all refused outside the workspace, and the flags that usually paper over that
(`--no-sandbox --disable-breakpad --disable-crash-reporter --disable-dev-shm-usage`)
did not make it return. The fallback needs no Chrome and is closer to the thing being
measured: **set `tauri.conf.json`'s window to the phone's size** (390×844, with
`minWidth`/`minHeight` to match or the window will not go that small), start the dev
app, and read the numbers out of the page with §5's DOM probe. It is the engine the
phone runs, it lays the layout out for real, and both stages can be measured in the
same window for comparison — which is how N13 found that the tool row wraps to two
group cards at 390px in **both** stages rather than only in the new one. Revert the
window size afterwards, and remember that changing it restarts the app (trap 3's
rule, the other way round: the file is right and the window is not what it says).

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

**N10 settled the naming half of it, and only the naming half: it is one app with
two courses, called *Nihongo Tutor*.** Calling it *Kana Tutor* while the kanji are
the bulk of the study was the same mistake as listing "Kanji" seventh among nine
tabs (invariant 31), and the fix was to name the product after the language rather
than after its first few days. The **identifier deliberately did not move**: it is
the data directory, and a new one would strand the learner's own files. If the app
is ever genuinely split into two products — which is still open — that is the day
the identifier has to change and a migration has to be written with it.

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

Ten, in `apps/nihongo-tutor/src-tauri/src/licences.rs`, with the text compiled
in by `include_str!` and copied into the bundle as plain text:

| id | What it covers |
| --- | --- |
| `agpl` | the app's own code, from the repository `LICENSE` |
| `provenance` | `LICENSES.md`, bundled as `licences/PROVENANCE.md` |
| `animcjk` | the kana and kanji geometry, the 214 radical head forms, **and the statement of what was changed** |
| `edrdg` | EDRDG's JMdict and KANJIDIC2 — the readings, glosses, grades and ranks — **and the statement of what was selected, changed and dropped** |
| `ccbysa` | the CC BY-SA 4.0 legal code those dictionaries are under |
| `jmdict-furigana` | the furigana alignments, and that none is invented here |
| `jmdict-furigana-mit` | the MIT text the notice above must accompany |
| `unidic` | UniDic's segmentation of the passages, and that no dictionary entry is redistributed |
| `lgpl` | the licence the kana SVGs and the kana `graphics` file are under |
| `arphic` | the other licence that file could be under, per invariant 12 |

`tests/licences.rs` holds the three-way correspondence the main app's does:
every catalogued notice exists on disk *and* is the text compiled into the
binary, every file in the app's notice directory is catalogued, and
`tauri.conf.json`'s `bundle.resources` copies exactly that set. Three further
tests check that the AnimCJK notice actually **records the modification** —
LGPL-3.0 §2 requires it — that the share-alike notices say what was selected and
what was dropped, and that none of the notice files is gitignored, since a
gitignored notice vanishes from a clone and the bundle.

**Five of the ten arrived at N9, and the reason is worth more than the list.**
The app shipped EDRDG's dictionary text from N6 and JmdictFurigana's alignments
and UniDic's segmentation from N7, and the only notice any of the three had was a
bullet inside `PROVENANCE.md`. That is the failure mode this section exists to
warn about, and it was invisible for two milestones because **the test that reads
the notice texts named five ids in a hardcoded table** — so a sixth notice was
never read by anything, and a stub would have passed. The table now ends by
asserting that *every* catalogued id appears in it, so the next notice cannot be
added and forgotten. `LICENSES.md`'s KANJIDIC2 section records each of EDRDG's
four requirements against the place it is met, and the three that were unmet.

**N1 added none of them, and that is itself a licence decision**: the voice is the
operating system's, so there are no audio files to attribute. A notice for the
synthesiser would be a notice for something this app does not ship.

The app ships `LICENSE`, `LICENSES.md` and the five notice files above.
**Before distributing it**, read `LICENSES.md`'s "Before you distribute": the
same rules apply, and N9 is where the EDRDG update obligation — which its terms
enforce, unlike a courtesy attribution — was checked against this app rather than
assumed to be covered by the Chinese one.

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
  Japanese failure mode, and a tutor without grammar teaches characters rather
  than reading. Out of scope for now, and the largest thing missing from the
  product. N11's Start screen says *why* the kana carry the grammar; it teaches
  none of it, which is the gap stated rather than closed.
* **The kana course in the kanji course's new shape — built.** N12 split the kanji
  screen in two because a phone made one page unusable, and the same complaint
  applied to Practice: on a phone the lesson list and the kana grid came before the
  board, so a learner scrolled to write. It was deliberately **not** changed with the
  kanji work — the kana course is the on-ramp and its screens are shallow — and
  **N13 is where it was treated the same way**: `KanaPanel` is the script toggle and
  one card per lesson, `KanaPractice` is the stage, and the chrome steps aside while
  one is up (invariant 34). **N15 finished the sentence** and this bullet is kept
  rather than deleted so a reader of the earlier copy sees it answered: the Radicals,
  Words and Read screens are each a course screen and a stage now, and the two touch
  targets on the shared tool row that were 40px are 44 at both stages — invariant 35,
  and `ROADMAP_NIHONGO.md` N15 for the measurement, which also caught the app's own
  tab row overflowing at a phone's width.
