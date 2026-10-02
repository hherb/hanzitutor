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

And the thing that transfers is the **mechanics**, not the data: counting strokes,
pairing them and judging their order and direction is one software problem whatever
the script — which is why `nihongo-core` uses `hanzi-core`'s `geom`, `raster` and
`grade` unchanged. **The kanji data still comes from Japanese sources**, AnimCJK
for geometry with KanjiVG as the stroke oracle, exactly as the kana did. The
Chinese dataset is not an input. §"The mechanics transfer; the data does not" has
the reasoning, and the learner-facing overlap figures that decide what a *screen*
says rather than what the build reads.

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

## The mechanics transfer; the data does not

Two earlier versions of this section had this backwards, and the maintainer's
correction is the point of it:

> …technically counting strokes, assessing stroke order/direction etc is the same,
> so the mechanics are transferable.

So what is "near identical" between kanji and hanzi is the **software problem**,
not the data. Counting strokes, pairing them, judging their order and direction,
measuring ink and placement — that is one problem, whatever the script, and it is
why this is a sibling app rather than a new one.

**That is demonstrated already, not planned.** `hanzi-core`'s `geom` (450 lines),
`raster` (624) and `grade` (1,695) grade strokes against strokes and never look at
the character, and `nihongo-core` uses all three unchanged: a handwritten あ is
judged by exactly the code that judges 一. Measured before any of it was built —
and recorded in `docs/research/JAPANESE_TUTOR_FEASIBILITY.md` §2.1 — those three
plus `time` have **zero** Chinese coupling, and `hanzi-sync` depends only on
language-neutral progress types.

### The data stays Japanese

**Use a Japanese dataset, and there is one.** The kanji half takes its geometry
from AnimCJK's `graphicsJa.txt` for exactly the reasons the kana half did: 7,007
characters with **outlines and medians**, in Make Me a Hanzi's own font space, so
`prepare-kanji` is `prepare-kana`'s geometry half with a different file.

The stroke-count and stroke-order **oracle is KanjiVG** — the same Japanese source
the kana pipeline already checks against (CC BY-SA 3.0), and **measured against
KANJIDIC2 across all 2,136 jōyō before writing this down: 2,127 agree, 99.6%.** The
nine that do not are named and can be handled explicitly rather than discovered:
謎 (16/17), 賭 (15/16), 葛 (11/12), 餌 (14/15), 遜 (13/14), 僅 (12/13), 遡 (13/14),
餅 (14/15) — KanjiVG one higher in eight of them — and 牙 (5/4), the only one where
KANJIDIC2 is higher. Verified agreeing examples: 愛 13, 学 8, 国 8, 鳥 11, 鳩 13,
韓 18. `prepare-kanji` should refuse to write an artifact whose counts it could not
check, exactly as `prepare-kana` does, and the nine are the list to decide about
rather than to paper over.

**The Chinese dataset is not an input to the kanji course.** It is not a data
source, not a cross-check, and not a fallback — an earlier draft proposed using it
as a second opinion, and that is the wrong instinct: a Japanese course built on
Chinese geometry would be checked against the thing it is trying not to be. The
only shared thing is the code.

### Why the overlap still matters — as product, not as pipeline

The characters do overlap enough that a learner who reads Chinese arrives with a
real head start, and the app should say so. Measured against this repository's own
data, of the 2,136 jōyō:

| | jōyō | kyōiku |
| --- | --- | --- |
| The **same simplified character** the HSK course teaches | **1,200 (56%)** | 651 (65%) |
| Present in the dataset, but only as a **traditional** form | **702 (33%)** | 260 (26%) |
| **No Chinese counterpart at all** — Japanese shinjitai | **234 (11%)** | 95 (9%) |

And where a character exists in both, the **shape** agrees about nine times in ten
— 1,117 of the 1,200 the course teaches (93.1%), and 619 of the 702 traditional
forms (88.2%), comparing stroke counts at the same code point.

This is for deciding what a **screen says**, not what the build reads, and it is
what makes the maintainer's transfer rules concrete:

* **Shape and stroke order transfer.** A learner who knows 学 knows how to write
  it. So does one who knows 學, for the 702.
* **Pronunciation does not transfer at all** — which is the good news, because
  there is no wrong habit to unlearn, only new material to teach.
* **Meaning transfers partly and unreliably**, which is the dangerous amount. The
  glosses come from JMdict senses and never from a Chinese dictionary: 娘 (JP
  daughter / ZH mother), 手紙 (JP letter / ZH toilet paper), 汽車 (JP steam train /
  ZH automobile), 勉強 (JP study / ZH reluctantly), 丈夫 (JP sturdy / ZH husband),
  走る (JP run / ZH walk). See `docs/research/JAPANESE_TUTOR_FEASIBILITY.md` §4.1.
* **The 234 shinjitai are genuinely new** — 両 乗 亀 亜 仏 仮 伝 価 働 児 剣 労 単 厳
  収 営 団 — and they are worth knowing as a set, because they are where a Chinese
  reader's head start runs out.

**And the direction works both ways.** The maintainer read Japanese first and
learned Chinese second, a path served today by Hanzi Tutor itself. Anything that
records "this character is already known from the other language" must not assume
which language came first.

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

**And the thing to get right early: Japanese data, Japanese oracle.** The
temptation is to reach for the Chinese geometry — 1,902 of the 2,136 jōyō have an
equivalent in this repository, and where both exist the shape agrees about nine
times in ten — but the kanji course is not built on it. Geometry from AnimCJK's
`graphicsJa.txt`, and the stroke-count and stroke-order check against **KanjiVG**,
the same Japanese source the kana pipeline uses — measured at **99.6% agreement
with KANJIDIC2 across the 2,136 jōyō**, with nine named exceptions. `prepare-kanji`
refuses to write an artifact whose counts it could not check. §"The mechanics transfer; the data does not" above has the
reasoning.

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
* **The nine jōyō where KanjiVG and KANJIDIC2 disagree on stroke count.** 謎, 賭,
  葛, 餌, 遜, 僅, 遡, 餅 (KanjiVG one higher) and 牙 (KANJIDIC2 higher). It is 0.4%
  of jōyō and it is a real decision rather than a rounding error: the artifact
  checks against KanjiVG, so these nine need either an explicit exception list with
  the count taken from KANJIDIC2, or a look at whether KanjiVG has split a stroke.
  Either way it should be a written list in the pipeline, not a silent pass — the
  kana equivalent is the 25 split characters, and that one turned out to be a
  genuine upstream format quirk rather than an error.

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
