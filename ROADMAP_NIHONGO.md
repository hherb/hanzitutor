# Roadmap — the Japanese part

The counterpart to [`ROADMAP.md`](ROADMAP.md) for the kana and kanji work. This
file is what to build next; [`HANDOVER_NIHONGO.md`](HANDOVER_NIHONGO.md) is how
to work on it.

The two roadmaps are separate because the two parts are: they share the geometry
engine and nothing else that a milestone needs to care about, and they version
independently (Japanese at 0.1.0). Nothing in `ROADMAP.md` mentions this work, and
nothing here mentions the Chinese milestones except where code is reused.

---

## Summary

**Kana are the on-ramp; kanji are the product.** That ordering is the maintainer's
and it should govern every priority below. A beginner memorises the kana in a few
days — 177 characters and a handful of digraphs is a weekend's work, not a course
— so the kana half being *finished to a usable standard* is most of what it needs
to be. Kanji is where the years go, and it is where a sibling to Hanzi Tutor earns
its existence.

That has one large consequence, and it is measured rather than assumed: **most
Japanese kanji are not the simplified characters this repository already knows.**
Of the 2,136 jōyō, only 1,200 (56%) are the simplified character the HSK course
teaches; 702 (33%) reach Hanzi Tutor's dataset only in their **traditional** form,
and 234 (11%) are Japanese shinjitai with no Chinese counterpart at all. So the
kanji geometry must come from AnimCJK, and the Chinese dataset is a cross-check
and an inventory — never a substitute. §"The kanji Hanzi Tutor already knows" has
the working and the spot checks.

In priority order. **The N-numbers are labels, not a ranking** — they are what
`HANDOVER_NIHONGO.md` §7 refers to, and renumbering them would break those
references for no gain.

| | Milestone | Why here |
| --- | --- | --- |
| **N0** | *(shipped)* the kana foundation | Done. Treat it as the gateway it is and do not gold-plate it. |
| **N3** | Per-learner confusability | **First, because it is nearly free.** The drill already knows which pairs it asked and which answer was picked, and throws it away. A few hours, and it is the only kana polish with real value left. |
| **N1** | Audio | Early, because audio is the one thing both halves need: a kana wants a sound and so does every kanji word. The system voices already work, so the desktop is wiring. |
| **N2** | A review queue | Early, for the same reason — kanji study without spaced repetition is not study. Wire it while the tree is small, and answer "does kana even want SRS?" on the way. |
| **N6** | **The kanji data layer** | **The long pole, and the real work.** Everything after it depends on it; see the measurement above for why it is not the Chinese data. |
| **N7** | Kanji through vocabulary | Reading, furigana, okurigana. This is where a Japanese tutor becomes one, and where the "near identical with the hanzi" instinct has to be resisted most: the *characters* overlap, the *readings and meanings do not*. |
| **N8** | The kanji course and screens | After the data. Mostly reuse of `RadicalsPanel` and the decomposition machinery. |
| **N9** | Distribution | Notices into the bundle, a listing, and the mobile shells. |
| **N5** | Pitch accent | Wanted and genuinely differentiating, but **blocked on a provenance check rather than on code**, and it is polish beside the kanji work. See the milestone for exactly what can kill it. |
| **N4** | Yōon drills and a kana chart | Last, and possibly never. The course teaches them and the input engine types them; a chart is a lookup convenience for a set a learner knows within the week. |

Three things are deliberately **not** milestones, and are recorded as known weak
spots instead: the three interface surfaces nobody has watched, the grading
tolerances never re-fitted for kana, and grammar — the last of which is the
largest gap in the product rather than in the kana course.

---

## N0 — What shipped (the kana foundation)

Recorded here rather than in a milestone because it is done.

### The data layer

`crates/nihongo-core`, 2,316 lines of Rust and 71 KB of committed artifact.
AnimCJK's kana geometry for all 177 characters, with the **split-stroke repair**
that the upstream format needs: 25 kana arrive with more drawing segments than
taught strokes — あ is stored as 4 and taught as 3 — and are folded back, with
the grouping read from AnimCJK's own SVG element ids and every count checked
against **KanjiVG** before the artifact is written. 27 segments fold back.

Also the readings (Hepburn and Kunrei-shiki for every kana, with っ and ー marked
silent), the gojūon course as 18 hiragana and 20 katakana lessons, 33 yōon
digraphs per script, 13 confusion pairs, and a romaji input engine whose table is
derived from the readings.

### The app

`apps/nihongo-tutor`, a Tauri app at 0.1.0 with ten thin commands and three
views: **Practice** (course, board, animation, grading, typing),
**Tell them apart** (the discrimination drill), and **Licences**.

`src/lib/render.ts` is lifted from the Chinese app unchanged — AnimCJK publishes
kana in Make Me a Hanzi's own font space, so the transform was already right.

### Acceptance criteria — met

* 177 kana: 86 hiragana, 91 katakana, 516 strokes.
* Every taught stroke count checked against KanjiVG, and `prepare-kana` refuses
  to write an artifact it could not check.
* The artifact is committed and a clone builds with no data step.
* 109 Rust tests and 11 frontend tests, clippy clean, `svelte-check` clean.
* Run on a display: the course, the board, the animation, the drill and a
  hand-drawn あ in ink all seen working.

---

## The kanji Hanzi Tutor already knows — and the ones it does not

Measured, because the intuition "kanji are near identical with the hanzi" is
**half right in a way that would cost a milestone to discover late.** The
characters overlap; the *glyphs do not*, because Japanese kanji lean on the
traditional forms and Chinese here is simplified.

Of the 2,136 jōyō kanji, against this repository's own data
(`data/raw/graphics.txt` for Make Me a Hanzi, `data/raw/hanziDB.csv` for what the
course actually teaches):

| | jōyō | kyōiku |
| --- | --- | --- |
| The **same simplified character** the HSK course teaches | **1,200 (56%)** | 651 (65%) |
| Present in Make Me a Hanzi only as a **traditional** form | **702 (33%)** | 260 (26%) |
| **No Chinese counterpart** — Japanese shinjitai | **234 (11%)** | 95 (9%) |

So **44% of jōyō glyphs must come from Japanese data**, and the third row is why:
両 乗 亀 亜 仏 仮 伝 価 働 児 剣 労 単 厳 収 営 団 and their like are Japanese forms
with their own code points and no ancestor in the Chinese set at all. The middle
row is the subtler one — 愛 華 樂 學 國 會 體 數 萬 舊 價 傳 兩 亞 圖 歸 are all
present, but only as traditional forms, while it is 爱 华 乐 学 国 会 体 数 万 旧
价 传 两 亚 图 归 that the course teaches. A Hanzi Tutor learner knows the
*character* and not this *glyph*.

Spot-checked rather than assumed: for each of those pairs, the traditional form is
in Make Me a Hanzi and **not** in the HSK set, and its simplified counterpart is in
the HSK set.

**What follows for the milestones.**

* **The geometry must be AnimCJK's.** Substituting the Chinese outlines would
  teach a wrong glyph for 44% of jōyō outright, and AnimCJK warns that even among
  the ones that look the same, stroke order, stroke direction or glyph may differ
  regionally. A stroke count that agrees is necessary and not sufficient — of the
  1,902 jōyō present in both datasets, **166 have a different stroke count**, and
  the rest have never been compared stroke by stroke.
* **The Chinese data is still useful, for two things.** It is a *cross-check* on
  the Japanese geometry, in the way KanjiVG was for kana; and it is an
  *inventory* — knowing that a character is one a Hanzi Tutor learner has already
  met is a real teaching advantage, and 1,200 of them are.
* **Readings and meanings do not transfer at all**, and the false friends are
  actively dangerous: 娘 (JP daughter / ZH mother), 手紙 (JP letter / ZH toilet
  paper), 汽車 (JP steam train / ZH automobile), 勉強 (JP study / ZH reluctantly),
  丈夫 (JP sturdy / ZH husband), 走る (JP run / ZH walk). Every gloss must come
  from JMdict senses and never from a Chinese dictionary. See
  `docs/research/JAPANESE_TUTOR_FEASIBILITY.md` §4.1.

**One more thing to verify before N6, and it is cheap:** the comparison above is
by stroke count, which is a proxy. Before committing to a curriculum, spot-check
stroke *order* against AnimCJK for a sample of the 1,200 shared simplified
characters, because that is the number that decides whether a "you already know
this character" claim can be shown at all. If the orders diverge often, the claim
has to be softened to "you have met this character" rather than "you know how to
write it".

---

## N1 — Audio

**Approach.** The desktop needs almost nothing. macOS ships Japanese system
voices — `Kyoko`, `Eddy` and others, `ja_JP` — and `crates/hanzi-voice`'s
`speech` module already drives the system synthesiser for the Chinese app, so it
is a wiring job: a `speak` command, a button beside the kana, and the same
`Speaker` the other two apps use.

Bundled clips are the other half and are **optional**. For ~120 kana the clean
source is generation: **MeloTTS-Japanese is MIT for both the code and the model
weights** (verified from the GitHub API and the Hugging Face card), which makes
it the only option here with no licence question attached. Commons has only about
ten isolated kana clips and Lingua Libre's Japanese recordings are per-file CC0
**or** CC BY-SA 4.0, so a bulk import would need a licence manifest. Neither is
worth doing for a first pass when the system voice works.

**Acceptance criteria.**

* A kana can be heard from the keyboard and by button, on macOS, with no
  download.
* If clips are bundled, each carries its licence, and the set is catalogued in
  the app's `licences.rs` like every other notice.
* The app still has no network path of its own.

**Deliberately not done.** Windows and Linux voices — `ROADMAP.md` M6 is where
that lives and it is unsolved there too.

---

## N2 — A review queue

**Approach.** Reuse rather than invent. `hanzi-core::progress` carries the
scheduler (`Sm2`, behind the `Scheduler` trait), the due-date arithmetic and the
attempt log; `hanzi-store` is the SQLite side and depends only on those
language-neutral types — measured at planning time, `hanzi-sync` depends on
nothing but them either. That last part is a reading of the manifests rather than
a demonstration — **nothing has yet used `hanzi-store` from a second app.** The
tone trainer is evidence for the pattern, not for this crate: it shares
`hanzi-core`, `hanzi-voice` and `hanzi-hearing` and does not touch the store at
all. Expect to find the seams when you first wire it up.

The open question, and it should be answered before building: **does kana want
spaced repetition at all?** 179 characters is small enough that exposure may be
enough, and `ROADMAP.md`'s own note about SM-2 versus FSRS applies — if a
scheduler is worth having, `fsrs-rs` (BSD-3-Clause) is the better default, with
the caveat that its pretrained weights are not openly licensed and should be
fitted on-device instead.

**Acceptance criteria.**

* A learner's attempts persist across restarts, in their own file, under
  `com.hanzitutor.kana`.
* A due queue exists and is reachable from the sidebar.
* Whatever scheduler ships, the choice is recorded with its reasoning — including
  if the answer is "no SRS, and here is why".

**Deliberately not done.** Cross-device sync. `hanzi-sync` would give it, but
sync is the wrong thing to add to an app nobody is yet studying with.

---

## N3 — Per-learner confusability

**Approach.** The drill already knows which pairs it asked and which answer was
picked; it throws that away. Keep it, count it per pair, and let the drill weight
towards the pairs this learner fails.

The 13 pairs in `curriculum::CONFUSABLE` are practitioner consensus — the set
every kana teacher reaches for — and were never claimed to be complete. The
learner's own wrong answers are the better list, and they are free.

**Acceptance criteria.**

* Wrong answers persist, per pair, and survive a restart.
* The drill prefers a pair the learner has got wrong, and can be shown to.
* A pair the learner never gets wrong stops being asked, or is asked rarely — and
  the rule is written down rather than emergent.

---

## N4 — Yōon and dakuten drills, and a kana chart

**Approach.** The course *teaches* yōon (33 per script) and the voiced rows, and
the input engine can type them, but nothing drills them, and there is no screen
that shows the gojūon grid at once — which is the thing a learner actually uses
to look something up.

A chart is cheap: `curriculum::ROWS` is already the grid, in order, and
`Row::kana(script)` gives either script from the one definition. A yōon drill is
the drill from N3 with digraphs as the answers, which needs the pool extending
rather than a second component.

**Acceptance criteria.**

* One screen shows the whole grid for either script, with the voiced rows
  distinguished, and tapping a kana opens it on the board.
* Yōon can be drilled, and きゃ is distinguished from きや — they are one mora
  against two, which is the point.
* A kana offered anywhere in the interface can be opened and graded.

---

## N5 — Pitch accent

**The milestone that is blocked on a licence check, not on engineering.**

**What is settled.** Japanese pitch accent is per *word*, not per syllable, and
it is not recoverable from the kana — so it must be stored data, visualised, and
modelled in audio. `hanzi-core::tone`'s pitch tracking, contour extraction and
DTW are language-neutral DSP and are directly reusable for scoring.

**What is blocked.** *No audit-clean pitch-accent lexicon can be bundled.*

* **Kanjium** `accents.txt` — 124,137 entries, mora-level, and a genuine
  CC BY-SA 4.0 `LICENSE.txt`. **Not bundlable**: the author states in issue #13
  that its source is withheld "due to potential copyright issues", issue #15
  clarifies that the README's credit covers only the arrow notation, and joining
  it against Wadoku's dump gives **88.9% of pairs identical including list
  order**. A CC BY-SA grant is only valid if the grantor holds the rights.
* **Wadoku** — custom non-free (annual written permission for anything beyond
  unmodified no-fee distribution). **OJAD** and Suzuki-kun — "refrain from using
  this site in for-profit ventures". **NHK** and **新明解** — proprietary.
  **CSJ** and **MULTEXT-J** — paid, non-redistributable.

**The route.** Generate the patterns instead of licensing them:

* **`tdmelodic`** — BSD-3-Clause, verified from its `LICENSE`. A neural generator
  of a Tokyo-dialect accent dictionary from surface + reading, at NEologd scale.
* **UniDic-CWJ 2.1.2's `kana-accent` variant** — NINJAL states on its own page
  that UniDic "Ver.2.x onwards … are now completely free software … you may also
  freely use them for business purposes", under GPLv2.0 / LGPLv2.1 / BSD New, so
  the **BSD option is AGPL-compatible**.

**Verify before writing code, and these are the only two things that can kill
this milestone:** (i) the licence and provenance of **tdmelodic's model
weights** — the repository ships none; and (ii) what its **training labels**
derive from. If they trace to the UniDic accent column the chain is clean; if
they trace to NHK or 新明解, use the UniDic column directly instead.

**Design so it degrades.** Not all of this needs a lexicon — mora timing,
devoicing, pitch range and "exactly one clear drop" need none; downstep and
contour *shape* need accent-phrase structure, which can be predicted. Only
per-word correctness needs a target, and only for curriculum vocabulary.

**Acceptance criteria.**

* A word's accent is displayed as a mora-level high/low contour.
* A spoken attempt is scored against it, and the score degrades sensibly when no
  target exists (contour shape only) rather than failing.
* Every accent pattern shipped has its provenance recorded, and the chain from
  generator to labels is written down.

**Deliberately not done.** Shipping any human-curated accent dictionary.

---

## N6 — The kanji data layer

**Approach.** `docs/research/JAPANESE_TUTOR_FEASIBILITY.md` §4 and §7 have the
working; the measured facts are:

| Source | Gives | Licence | Size |
| --- | --- | --- | --- |
| AnimCJK `graphicsJa.txt` | outlines **and** medians, 7,007 kanji, in Make Me a Hanzi's font space | Arphic PL | 21.9 MB |
| AnimCJK `dictionaryJa.txt` | 7,184 entries: the 214 Kangxi radicals, 6,948 IDS decompositions, and the **kyōiku grade sets** in `set` | LGPL-3.0+ | 1.36 MB |
| KANJIDIC2 | 13,108 entries: `grade`, `freq`, `stroke_count`, `radical`, `ja_on`, `ja_kun` with okurigana markers, `nanori`, meanings | CC BY-SA 4.0 | 1.49 MB gz |
| JMdict_e | 218,849 entries, with `nf01`–`nf48` frequency bands | CC BY-SA 4.0 | 10.6 MB gz |
| `scriptin/kanji-frequency` | frequency from Aozora / Wikipedia / Wikinews | CC BY 4.0 | small |

Three things make this cheaper than it looks. AnimCJK's kanji geometry is in the
**same font space** as the kana and as Make Me a Hanzi, and there are zero entries
where the stroke and median counts disagree — so `prepare-kana`'s geometry half
becomes `prepare-kanji` almost unchanged. `dictionaryJa.txt`'s `decomposition`
field is a standard **IDS string**, so `hanzi-core::decompose` parses it as it
stands. And the kyōiku grades give a ready-made curriculum — `g1`…`g6` are the
primary grades and `g7` completes jōyō to exactly 2,136.

**Two traps, both verified.** The grades need reconciling: `dictionaryJa.txt`'s
`g1`…`g6` total **1,006** (the pre-2017 split, with the 20 prefecture kanji in
`g7`), while KANJIDIC2's `grade` totals **1,026** — so take the ladder from the
one and the current assignment from the other, or the app teaches 2016's
curriculum. And **KanjiVG cannot substitute here**: the kana pipeline used it as
an oracle, but for kanji it supplies only centre-lines and no ink outlines, so it
is a stroke-order cross-check and nothing more.

**And the trap that would cost the most to find late: do not reach for the
Chinese geometry.** It is the obvious shortcut — 1,200 of the 2,136 jōyō are
characters this repository already draws — and it is wrong for 44% of them,
because Japanese leans traditional where Chinese here is simplified. The
measurement, the three-way split and the spot checks are in §"The kanji Hanzi
Tutor already knows" above, and that section also names the one cheap comparison
worth doing before the curriculum is fixed: stroke *order* for the shared 1,200.

**Acceptance criteria.**

* A committed `kanji.bin.gz`, built by `prepare-kanji`, with the same
  refuses-to-write-unchecked discipline as the kana one.
* Jōyō 2,136 with the current kyōiku grades, and the reconciliation between the
  two grade sources is written down.
* Readings carry okurigana, and a word's reading is not composed from its
  characters.
* Every source's licence is in `LICENSES.md` before any of it is bundled, and
  the EDRDG update obligation is designed rather than discovered.

**Deliberately not done.** The 3,783 `g9` hyōgai characters, and the jinmeiyō
`g8` set, until the course needs them.

---

## N7 — Kanji through vocabulary

**The milestone that makes it a Japanese tutor rather than a character drill.**

**Approach.** A character is *not* the unit. 生 has a dozen readings that only
resolve in a word; 食べる is 食 plus okurigana; and a kanji grind with no
vocabulary produces the classic "I know 1,500 kanji and cannot read a paragraph".
So the course teaches **words**, with the characters arriving through them, and
every screen is about reading.

`hanzi-core`'s word model already carries a whole-word reading rather than
composing one from the characters — added for Chinese polyphones — which is
exactly the shape Japanese needs.

**What is needed that does not exist.**

* **A level ladder, chosen and documented.** The JLPT publishes no official kanji
  or vocabulary list; the best-licensed community list chains to tanos.co.uk,
  which asserts no licence. Derive bands from the kyōiku grades and JMdict's
  `nf01`–`nf48`, and say in the UI that they are an approximation.
* **Furigana**, which needs morphological analysis — Japanese has no whitespace
  and no per-character reading. `lindera` (MIT), `vibrato` (MIT), `jpreprocess`
  (BSD-3) and UniDic (triple-licensed, take the BSD option) are all clean.
* **Okurigana** handling, and a decision about 送り仮名 variants.

**Acceptance criteria.**

* A word is graded as a word: its own reading, shown as furigana over the right
  characters.
* A learner can read a short passage with furigana and tap any word.
* The level bands are derived from licensed data, the derivation is reproducible,
  and the UI says they are ours rather than the JLPT's.

---

## N8 — The kanji course and screens

**Approach.** Mostly reuse, and the reuse is already measured: `hanzi-core`'s
`geom`, `raster` and `grade` are language-neutral; `curriculum.rs` becomes a
grade-ordered course from `dictionaryJa.txt`; `RadicalsPanel` and the
decomposition panel map onto the 214 radicals and the IDS strings.

**Acceptance criteria.**

* A course in kyōiku grade order, then jōyō, and every character opens on the
  board.
* The 214 radicals, each with the characters that share it.
* Components shown by decomposition, from the IDS string.
* The Chinese course still passes its own suite unchanged.

---

## N9 — Distribution

**Approach.** Follow `ROADMAP.md` M5 and M9, which have done this twice.

**Acceptance criteria.**

* Every notice in `licences.rs` is in the bundle, and
  `tests/licences.rs`'s three-way check covers the set that actually ships.
* A signed desktop bundle, and a Play listing if the mobile shell is wanted.
* The EDRDG update procedure exists and is documented, because the first KANJIDIC2
  data brings that obligation with it.
* The listing copy is written, and the screenshots are the ones in
  `assets/showcase/` — see `assets/README.md`.

---

## Cross-cutting polish

Recorded here rather than as milestones because none of them is a feature.

* **The three surfaces nobody has watched.** The grading verdict, the Licences
  panel and the katakana tab are tested and typechecked but were never seen
  running. `HANDOVER_NIHONGO.md` §5 has the window-capture recipe; this is
  fifteen minutes of work and should happen before anything else.
* **The Chinese docs now point here, and should keep doing so.** `README.md`,
  `ROADMAP.md` and `HANDOVER.md` each gained a pointer when this file was written;
  until then `grep -c 'nihongo\|kana'` over the three returned 0. If the Japanese
  part is ever renamed or its documents move, those three pointers are what breaks
  first, and nothing tests them.
* **The three interface bugs that shipped** are fixed and two are invariants now
  (the board's geometry, the title bar inset), but the *class* of mistake — two
  things deciding one dimension, and a layout that is only wrong on a display —
  is worth remembering when the kanji screens are built. Kanji screens will have
  a taller board and a longer sidebar, which is more of the same.
* **The two Dependabot warnings that stand** (`glib`, `proc-macro-error`) are
  Linux-only and upstream-blocked. Nothing to do; recorded so they are not
  re-investigated.
* **The `assets/website` duplication** — 10 MB carried twice. Decide whether it
  stays a copy.

---

## Known weak spots

* **The grading tolerances were fitted on Chinese characters.** `grade.rs`'s
  constants were tuned against the real hanzi dataset and carried over
  unchanged. Kana are simpler and fewer-stroked, which makes them *noisier* to
  score, not easier: わ/ね/れ, る/ろ, シ/ツ and ソ/ン differ by stroke direction
  and a couple of degrees. `selfcheck` should be re-run against kana, and the
  confusable pairs are the obvious test set.
* **Kana handwriting has legitimate variants the app may reject.** き and さ are
  taught as three strokes but are very commonly handwritten connected as two,
  and ふ, そ, な and む have well-known variants. A kana tutor that rejects a
  legitimate hand is worse than no tutor, and nothing has tested this.
* **No grammar, and that is the largest gap in the product.** Particles, the
  copula and every inflectional ending are kana, so a learner who finishes the
  kana course has unlocked grammar and been given nothing to use it on. This is
  not a kana-tutor weakness so much as the reason a kana tutor is not a Japanese
  tutor.
* **The 13 confusion pairs are a fixed list.** N3 addresses it.
* **`hanzi-store` and `hanzi-sync` are unused.** The measurement says they are
  reusable; nothing has proved it yet.
* **The kanji milestones are plans, not measurements.** The kana estimates in the
  feasibility report were good, but the kanji data sizes are from upstream
  listings rather than from a build, and the artifact size is unknown.

---

## Explicitly out of scope

* **Classical Japanese (文語) and hentaigana.** Not taught, not planned.
* **Korean hanja and Chinese traditional forms.** AnimCJK has the data and the
  other apps do not want it; if it is ever built it belongs to the Chinese app.
* **Handwriting recognition beyond geometric matching.** No ML model is needed or
  wanted: the medians make template matching work, and a model would add a
  download and a licence for something the geometry already does.
* **Cloud accounts, leaderboards, and anything that needs a server.** The app's
  promise is that nothing is downloaded and nothing phones home, and that is
  worth more than any feature it forecloses.
* **Copying anyone's mnemonics.** Heisig's stories, WaniKani's radical names and
  Kanji Koohii's community stories are all off-limits — see
  `docs/research/JAPANESE_TUTOR_FEASIBILITY.md` §6.2. Original mnemonics or the
  CC0/CC BY-SA sets on Commons.
* **Kanji etymology.** There is no open source comparable to Make Me a Hanzi for
  Japanese, and the usual imagery is proprietary. A real capability regression
  against the Chinese app, and the honest position is to say so rather than
  scrape it.
