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
| **N3** | *(shipped)* per-learner confusability | Done, and it was as cheap as predicted: the drill weighs the 13 pairs by what this learner gets wrong and remembers it in the app's own file. See the milestone for the rule, and for what is deliberately not there. |
| **N1** | **Audio** | **Shipped.** The system's own Japanese voice, behind a button and a key. Nothing is bundled and nothing is downloaded; see the milestone for what the crate had to learn first. |
| **N2** | **A review queue** | **Shipped.** The schedule the boards' attempts feed, in the app's own file, with the scheduler's code shared and the data never shared (invariant 15). See the milestone for the question it was asked to answer first. |
| **N6** | **The kanji data layer** | **Shipped.** The artifact is committed and verified; see the milestone for the three things the measurement corrected. Vocabulary moved to N7, where it belongs. |
| **N7** | Kanji through vocabulary | **Shipped.** Words with their own readings and furigana, a derived ladder, and two screens — Words and Read. The kanji artifact it embedded unread is now the character course's, at N8. |
| **N8** | **The kanji course and screens** | **Shipped.** The course, the 214-radical table and the components screen, all served from the artifact the vocabulary already needed. See the milestone for what the measurement added — and for the one thing it decided that was open. |
| **N9** | Distribution | Notices into the bundle, a listing, and the mobile shells. |
| **N5** | Pitch accent | Wanted and genuinely differentiating, but **blocked on a provenance check rather than on code**, and it is polish beside the kanji work. See the milestone for exactly what can kill it. |
| **N4** | Yōon drills and a kana chart | **Shipped.** The grid for either script, the characters off it, and the yōon contrasts drilled against their long spellings — one mora against two. Every kana on the chart opens on the board. See the milestone for what the measurement corrected. |

Three things are deliberately **not** milestones. Two are recorded as known weak
spots below — the grading tolerances never re-fitted for kana, and grammar, which
is the largest gap in the product rather than in the kana course. The third was the
set of three interface surfaces nobody had watched; that one is done, and it found
two bugs rather than none (see "Cross-cutting polish").

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
* 110 Rust tests and 11 frontend tests, clippy clean, `svelte-check` clean. (109
  when N0 was accepted; the contract test for the grading options payload was
  added with `HANDOVER_NIHONGO.md` invariant 14.)
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

> **Correction, from building it.** The 2,127 and the nine are both artefacts of
> reading KANJIDIC2's `strokeCounts` as a single value: it is a **list**, 91 jōyō
> have two entries, and in exactly those nine the *first* is not the count the
> character is taught with — so comparing against "the" count invented them. Against
> the taught count all nine agree and **one** real disagreement is left, 衷 (10
> against KanjiVG's 9), which `prepare-kanji` records with both numbers. The route
> is right and the number was wrong; the Shipped section below has the measurement,
> and `HANDOVER_NIHONGO.md` invariant 19 is the rule.

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

### Shipped

All three criteria are met, and the second is met by **bundling nothing at all**.

**The wiring was not the hard part; the language was.** `hanzi_voice::Speaker`
already drove `AVSpeechSynthesizer` in process, as this milestone predicted — but
it picked a voice by construction: its list was the Chinese one, its preference
was `Tingting`, and its message said "no Chinese voice is installed". It now
carries a `Language`, and the default is still `Chinese`, so the two apps that
were built against it are unchanged by the fact that a third one exists. Japanese
prefers **Kyoko**, then Otoya, then any `ja` locale, which is the same three-step
rule the mainland preference already used. **Each language also has its own
override variable**: `NIHONGO_TUTOR_VOICE`, because a kana tutor obeying
`HANZI_TUTOR_VOICE` would be the settings version of the shared-store mistake
(invariant 15).

**The microphone is a feature now.** `hanzi-voice`'s capture half carries `cpal`,
and this app never opens a microphone, so `capture` is a **default** feature and
the kana tutor takes the crate with `default-features = false`. The two
tone-scoring apps get the whole crate without editing a line. Same instinct as
refusing to bundle the analyser, one crate down.

**Three thin commands** — `speak`, `stop_speaking`, `voice` — and one decision
that governs all of them: **what is spoken is always kana.** A kana character, or a
word's *own* reading, so the word card says ひとつ for 一つ. A bare kanji is never
spoken, because it has a dozen readings and this app does not choose readings
(`HANDOVER_NIHONGO.md` invariant 21), and a radical's prompt is a Kangxi number.
The Review board therefore offers the button for a `kana` item and for nothing
else — measured, below — which is the one place that rule is visible as a missing
control rather than as a wrong sound.

**The shortcut is `h`, and that is a decision rather than a preference.** Space is
what the Tone Trainer gives its microphone, but on a page it already means
"activate the control that has focus" — and a learner who has just clicked a kana
cell still has that cell focused, so a Space shortcut would re-press the cell
instead of speaking. The rule (which presses count, and which are a letter meant
for the reading box) is a pure function with ten tests of its own; `command-H`
hides the window and is deliberately not stolen.

**Verified on a real window, by probe and by ear.** The app was driven through a
DOM probe against a hand-written schedule with two characters due — the recipe is
`HANDOVER_NIHONGO.md` §5 — and it reported:

* `voice()` → `Kyoko (ja-JP)`, and `[speech] using voice Kyoko (ja-JP)` from the
  app's own warm-up thread. **Measured, and not what was expected**:
  `AVSpeechSynthesisVoice` reports the bare name and a BCP-47 tag, where the legacy
  `say -v '?'` prints `Kyoko (Japanese (Japan)) ja_JP`. Both spellings resolve to
  the same voice and both are now pinned.
* the "Hear it (H)" button spoke あ, and pressing `h` on the window spoke it again;
* `h` typed **into the reading box** and `command-H` spoke nothing, which is the
  half that would have silently broken the course — `は` is typed `ha`;
* the word card spoke **ひとつ** for 一つ, the reading and not the written form;
* a due **あ** offered the button, and a due **学** offered none: *Show stroke
  order · Undo · Clear · Grade*, and no "Hear it".

The maintainer then heard it, which is the one thing no test can do. 5 new Rust
tests (2 in the app, 2 in the contract suite, 1 more `hanzi-voice` fixture for the
AVFoundation spelling) and 10 frontend ones; `hanzi-voice` is 29.

**What is deliberately not here.** Bundled clips — the criterion for them is met
by shipping none, and the licence position for a set is unchanged (§7 of the
handover). The discrimination drill is silent for a reason rather than for
omission: シ against ツ is an exercise in telling two shapes apart, and handing the
learner the sound would answer the question.

### Shipped — the kanji card, which offers both

**A reading the card lists can be heard, and the character on the board can still
be written.** The two are offered together rather than one being assumed, at the
maintainer's instruction: *"offer both, the user can decide what he wants (hearing
or writing)"*. On the `Kanji` screen the board is the writing half and every
reading — on, kun and nanori — is now its own control, so the choice is the
learner's at the moment they make it. The card says so in one line: *"Write 日 on
the board above, or tap any reading to hear it."*

**The readings stay text.** They are drawn as the reading itself with the passage's
own dotted-underline affordance, not as a grid of buttons: 生 has **twenty**
readings, and twenty bordered boxes would bury the thing being read. With no
Japanese voice installed they are drawn as the plain text they were before this
existed, greyed out nowhere, with one note rather than one per reading.

**And the markers come off only for the voice.** KANJIDIC2 writes a reading with
its okurigana attached — `た.べる`, `ひと-`, and 応 and 王's `-ノウ` — and the dot and
the dash are **teaching information**: they are what says 食べる is written with kana.
They are stripped for speech (`spokenReading`: た.べる → たべる, ひと- → ひと,
-ノウ → ノウ), so the card shows `た.べる` while the button's own title says
*"Say たべる …"* — the teaching form on screen, the spoken form one hover away.

**Measured before it was a one-liner.** Over all 9,364 on, kun and nanori readings
there is **no** character outside the hiragana and katakana blocks, ー and the two
marks, and every reading keeps at least one kana once the marks come off — so
"strip the marks and hand it to the synthesiser" is the whole transformation, with
nothing guessed at and nothing spelled out. The two marks also have different
shapes, which a single rule would flatten: the dot is **always medial** (2,551 of
them, never two in one reading) and the dash is **always at an end** (364 on kun
readings, and exactly four on on-readings, which a "the dash is a kun thing"
reading of the field would miss). `every_reading_is_kana_once_its_markers_are_stripped`
in `tests/kanji_artifact.rs` pins all of it, including the four: 応 王 `-ノウ`,
縁 `-ネン`, 音 `-ノン`.

**Verified on a running window.** Lesson 1's 一 and 日 between them carry every
shape: tapping `ひと.つ` logged `speak "ひとつ"`, `ひと-` logged `speak "ひと"`,
`イチ` passed through as katakana untouched, and 日's leading-mark `-び` logged
`speak "び"`. The meanings row stayed plain text, and the no-voice card was
captured too — readings as plain text, one settings-path note, nothing greyed.

---

## N2 — A review queue

**Approach.** Reuse the *code*, keep the *data* here. `hanzi-core::progress`
carries the language-neutral scheduler (`Sm2`, behind the `Scheduler` trait), the
due-date arithmetic and the attempt log, and that is the part worth reusing.

**`hanzi-store` is not the answer, and that is settled rather than open.** It is a
SQLite store for the Chinese app's learner data — attempts, vocabulary, cursors,
settings — and the Japanese app keeps its own learner data in its own file
(`HANDOVER_NIHONGO.md` invariant 15). The two apps are separate products and their
learners' data is separate with them: what is shared between them is code, never
user data. So the store for a kana review queue is `apps/nihongo-tutor`'s, next to
`confusions.json`, and the precedent is already written: `store.rs` is 280 lines
that load, record and write atomically, with tests for the missing file, the
corrupt file and the failed write. The temptation was real — the manifests say
`hanzi-store` depends only on language-neutral types, and that is true — but a
shared store for two separate apps is the thing that turns "separate" into
"tangled", and the tangle shows up the day one app's schema change breaks the
other's history.

The open question, and it should be answered before building: **does kana want
spaced repetition at all?** 179 characters is small enough that exposure may be
enough, and `ROADMAP.md`'s own note about SM-2 versus FSRS applies — if a
scheduler is worth having, `fsrs-rs` (BSD-3-Clause) is the better default, with
the caveat that its pretrained weights are not openly licensed and should be
fitted on-device instead. N3's per-pair weights are the cheap answer to the same
question and they ship; a scheduler has to earn its screen against them.

**Acceptance criteria.**

* A learner's attempts persist across restarts, in their own file, under
  `com.hanzitutor.kana`. *(N3's tallies already do this; a queue would add to the
  same file or a sibling of it.)*
* A due queue exists and is reachable from the sidebar.
* Whatever scheduler ships, the choice is recorded with its reasoning — including
  if the answer is "no SRS, and here is why".

**Deliberately not done.** Cross-device sync. `hanzi-sync` would give it, and it
is the same kind of shared-data mistake as `hanzi-store` for two separate apps;
sync is also the wrong thing to add to an app nobody is yet studying with.

### Shipped

**The question this milestone said to answer before building anything, answered:
yes — but not because the kana need it.** An SRS over the 177 kana alone would not
earn a screen against N3's drill. The kana are a week's work and the drill already
weights the thirteen pairs that are actually confusable; a due-date list of
あいうえお would be ceremony. What earns the screen is the other half of the app:
**kanji study without spaced repetition is not study** is this roadmap's own
sentence, and the board that grades a kanji is the board that grades a kana. So
the schedule is over the characters the board can draw — kana, jōyō kanji and
radical head forms — and the kana ride along because they are the same unit rather
than because 179 characters need a scheduler.

**SM-2, reused rather than reimplemented.** `hanzi_core::progress` carries the
scheduler behind a `Scheduler` trait, the due-date arithmetic and the attempt log,
and language-neutral is exactly what those are: they count attempts and multiply
intervals and never look at the character. `nihongo-core` re-exports them the way
it already re-exports `grade`, and its `review.rs` adds only the Japanese half —
what a character the board can draw *is*, and what to prompt the learner with when
it comes back. **FSRS is deliberately not used**: it is the better algorithm, but
its pretrained weights are not openly licensed, fitting them on-device needs far
more attempts than one learner produces quickly, and `hanzi_core`'s own note says
the same. `Scheduler` is the seam it would arrive through.

**The data is the app's own file, and that is invariant 15.** The schedule is
`review.json` in this app's own data directory, beside `confusions.json`, written
by Rust rather than by the webview, which still holds no filesystem permission.
`hanzi-store` is not used: the two apps share code and never learner data. The
failure paths are the drill's, made again — a file that will not parse is kept
aside rather than overwritten, a file written by a newer build is left untouched
and the session runs in memory, and a write that fails is reported to the learner
while the attempt stays counted.

**And one rule that is not obvious, which is why it is written down.** An attempt
advances the schedule only when the character is **new or due**. Recording every
Grade press would be wrong in a way that is easy to miss: a learner who writes あ
five times in one sitting would advance SM-2's interval five times and not see あ
again for a year, on the strength of one minute's practice. A second attempt
inside the interval is still graded, and the schedule is left exactly as it was.
The three outcomes — counted, not counted, and counted-but-not-saved — are told
apart on screen rather than collapsed into a score.

**What is on screen.** A **Review** view in the sidebar lists what is due, most
overdue first, with each character resolved to what the board will draw and a
prompt beside it: a kana's reading, a kanji's gloss, or a radical's Kangxi number,
which is also how the panel fetches the geometry of a head form the character
course cannot reach. Choosing one puts it on a board; Grade grades it and
reschedules it. The queue is paged — `due` counts the whole backlog, `items` are
the most overdue page of it — and when nothing is due the screen names when the
next character comes back rather than only saying "nothing". `grade_attempt` now
returns `{ report, scheduled, nextDue, warning }` instead of a bare report, so all
three boards — Practice, Kanji and Review — say what the schedule did with the
attempt. That is the shape a request/response contract test pins, both halves
(`HANDOVER_NIHONGO.md` invariant 14).

**Verified on a display, and by writing on it.** The Review screen was captured on
a running window; characters drawn on the board were graded — 学 at 87/100 and あ
at 100/100 — and both appeared in `review.json` under `com.hanzitutor.kana` with
SM-2's own fields (学 rescheduled six days out, あ two), and a cold restart read
them back. Looking at it also found two interface bugs that every test and
`svelte-check` were happy with: "1 characters are scheduled", and an interval
label that floored 1.9997 days to "in 1 day" beside the two-day interval the same
screen was showing. Both are fixed and both now have a test.

**24 new Rust tests and 11 new frontend ones.** `nihongo-core` 175 (was 168) and
`nihongo-tutor` 93 (was 76); the frontend's 35 became 46. The scheduler's own
arithmetic is `hanzi_core`'s and already tested there — what is new here is the
Japanese half, the store's failure paths, and the IPC shapes.

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

### Shipped

All three criteria are met, and the rule is written down in
`nihongo_core::drill` — `weight = max(0.25, 1 + 2 × wrong − correct)`, drawn
weighted at random, with the floor so that a pair the learner has stopped missing
becomes rare rather than impossible and can still come back.

**What changed, and why the drill is not a pool any more.** The interface used to
be handed a pool of kana and draw from it, which cannot work once an answer has to
be remembered: only the asker knows which pair it asked. So `drill_pool` is gone;
`next_drill_question` draws the pair in Rust and returns it with the question, and
`record_drill_answer` takes that pair back with the two kana and decides
correctness itself — the interface has no field in which to claim it was right.
The question also shrank to the pair's own two kana. The old four-option question
tested more at once and taught less: with three wrong answers on screen a miss
cannot be attributed to a pair, which is exactly what has to be measured.

**The store is the app's own** (`apps/nihongo-tutor/src-tauri/src/store.rs`):
`confusions.json` in the app's data directory, written by Rust after every answer,
atomically, by `std::fs` — the webview still has no filesystem permission, and the
app has no new dependency for it. A file that will not parse is moved aside rather
than overwritten, and a write that fails still counts the answer and tells the
learner it was not saved. The decision that this is app-local and stays app-local
is `HANDOVER_NIHONGO.md` invariant 15, and it is why N2 below no longer reaches for
`hanzi-store`.

**Verified live, not only in tests**: eight answers written to
`Library/Application Support/com.hanzitutor.kana/confusions.json`, the app
restarted, and the next six questions continued those counts (ク|ワ 3→5, ソ|ン 1→2)
while the weighted draw went to the pairs the first session had missed. 24 new
tests cover the rule's arithmetic, the floor, the deterministic draw, the
persistence round trip, the corrupt-file path and the payload the interface posts.

**What is deliberately *not* here.** The pool is still the 13 classic pairs. The
Approach above says the learner's own wrong answers are the better list, and they
are — but a drill that only ever asks about the 13 cannot discover a 14th, because
it never asks a question the 13 do not cover (あ/お, say). Growing the list means
asking about kana rather than about pairs, which is a different drill and a
different record. N4's yōon drills are the natural place for it.

---

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

### Shipped

All three criteria are met, and the first one needed the grid to stop being a
list. **The title names two drills and the criteria name one** — yōon and dakuten —
so both are drilled: the acceptance list's きゃ/きや is the yōon contrast and the
voiced rows got the same treatment, because "the course teaches them and nothing
drills them" was true of both.

**A row's holes are now data, and that is the whole chart.** `Row` held a row's
kana in gojūon order, which is enough to *teach* a row and not enough to *draw* one:
や has three kana and they sit in the a, u and o columns, so a chart built from the
list draws ゆ under い and teaches the wrong vowel. Each row now holds its five
slots — a, i, u, e, o — with `None` where the language never filled one, and
`Row::kana` is derived from them, so the course and the chart read the same table.
Measured: **16 rows, 80 slots, 71 kana, 9 holes** (や's i and e, わ's i, u and e,
ん's four), and the layout is checked against something independent rather than
trusted — `every_cell_sits_in_the_column_its_vowel_names` reads each filled slot's
own Hepburn vowel and fails if a kana is in the wrong column.

**The chart is the grid plus everything off it.** `off_grid` is now a function of
its own — the small kana, the rare ones, and in katakana the v-series and the
prolonged sound mark — and the course *is* the grid plus it, so the two cannot
disagree about what exists. Hiragana's chart is **16 rows and 15 kana off the
grid**, katakana's is **16 rows and 20 off it** (small 10 + rare 5 + v-series 4 +
ー), which is every kana of the script and not most of them. Tapping any of them
opens it on the board: `kana_chart` serves the geometry through the same `kana`
command the course uses, so acceptance criterion 3 is a property of the data
rather than a promise in a comment.

**The columns are labelled with the vowels**, from the engine's own table
(`VOWEL_COLUMNS`) rather than a string in the component: a chart whose columns are
unlabelled hides the thing that makes ゆ sit under う.

**The yōon drill is the same drill, not a second component.** The pool grew from a
list of kana pairs to a list of **pairs of spellings** — a side carries its reading
with it — so one question shape now serves シ/ツ, きゃ/きや and か/が, and the
command's `kind` picks the pool. Five exercises: the classic `confusion`, the yōon
and voicing contrasts of each script. The script is part of the exercise rather
than a setting beside it, and that is not tidiness: `kya` names きゃ in hiragana and
キャ in katakana, and `ga` is が and not ガ, so a question that mixed them would have
two right answers. The classic pairs are the exception — り/リ is a pair *across*
the scripts, which is the point of it — so that one carries no script. **33 yōon
contrasts and 25 voicing contrasts per script**, 129 pairs in all.

**The voicing exercise is the mark, and it is derived from the rows.** か against
が, は against ば *and* against ぱ: the two kana are the same character with a
dakuten or a handakuten, which is exactly what a beginner leaves off, and the two
shapes are near identical on purpose. The relation is stated between **rows**
(`VOICED_FROM`: ga←ka, za←sa, da←ta, ba←ha, pa←ha) rather than by arithmetic on code
points, because は has two voiced forms and a rule that subtracted one from が and
two from ぱ would be inferring the language from the encoding. The pairs are zipped
from the two rows' slots, and the independent half is the test: the two kana of
every one of the 50 pairs really are one code point apart (dakuten) or two
(handakuten), and the tell names the mark that the difference is.

**The two-mora counterpart is derived, not written out.** `Yoon` now carries the
full-size spelling (`きや`), its reading (`kiya`) and its Kunrei reading (`siya` for
しゃ), composed from the base's own reading and the full-size kana's — and checked
against a second path to the same string,
`the_plain_counter_of_every_yoon_is_what_the_input_engine_types`, which asks the
romaji engine to type both spellings of all 66 contrasts. The drill's prompt is
that same reading, which the live check cross-checked by asking the app's own
`romaji_to_kana` what the prompt spelled.

**The record is the same file, under a new kind of key.** A yōon contrast is
recorded in `confusions.json` under `きゃ|きや` and weighted by N3's rule unchanged,
because confusing きゃ with きや is a confusion like any other — and the two apps'
data boundary (invariant 15) is untouched, since it is still this app's own file.
Widening the key from a pair of kana to a pair of spellings moved **no existing
key**: `key_of(a, b)` orders two strings and reproduces `pair_key` exactly, which
`a_kana_pair_keeps_the_key_it_was_stored_under` pins over all thirteen.

**Four things the work corrected, and one of them was a bug in this document's
own milestone.**

* **The katakana small and rare lessons listed hiragana kana.** Their titles were
  written out by hand — `Small kana — ゃ ゅ ょ っ` — while their members were
  converted, so the katakana course drew ャ ュ ョ ッ and named ゃ ゅ ょ っ beside
  them. The title is now drawn from the lesson's own kana, which is also what makes
  it right for the ten small kana rather than the four a hand-written title
  remembered.
* **Generalising the drill's record lookup lost the either-order key.** The first
  version looked a pair up by comparing the key literally, and the existing test
  caught it immediately: `ツ|シ` is what a caller may post and `シ|ツ` is what the
  file stores, and the app has already written files under the canonical spelling.
  The key is now canonicalised before the lookup, and
  `an_answer_is_recorded_against_the_pair_and_the_arithmetic_says_so` is why that
  was caught rather than shipped.
* **`YOON_SMALL` carried each vowel twice** — once as the Hepburn vowel and once as
  the Kunrei one, which are the same thing, because the two systems disagree over
  the consonant (し is `shi`/`si`) and never over the vowel. It now carries the
  full-size kana instead, which is the fact the drill actually needed.
* **The voicing tell ended in a full stop, and the screen adds one of its own.** The
  live check printed the learner's feedback as *で is the same kana with the dakuten
  ゛..* — two stops, from a tell that was a sentence where every other tell is a
  clause. No test looked for it, because the thirteen classic tells happen to have
  no stop at all; there is one now, over every pool, since a generated tell is
  exactly where the mistake comes back. This is the class of bug the probe exists
  for: the suite was green, `svelte-check` was happy, and the extra character was
  only visible in words a learner would read.

**Verified on a running window, by probe.** There is no active display in this
session, so §5's capture recipe is a frozen frame (trap 17) and the check went over
HTTP to the dev server's log. It reported, in its own words:

* the chart: `vowels=[a,i,u,e,o] ka=[か,き,く,け,こ] ya=[や,_,ゆ,_,よ]
  wa=[わ,_,_,_,を] n=[ん,_,_,_,_]` — the holes in the columns the language leaves
  them, which is the measurement the first acceptance criterion turns on;
* tapping ゆ opened `big=ゆ reading=yu lesson=や ゆ よ` — the board *and* the lesson
  highlight, so a kana opened from the chart lands in its own row rather than the
  first lesson's;
* the katakana chart read `ya=[ヤ,_,ユ,_,ヨ]`, and ヷ — a character that is on no
  grid at all — opened with `lesson=V-series`;
* the yōon drill asked `Which one is byo?` with `opts=ビヨ,ビョ`, the app's own
  romaji engine confirmed `byo -> ビョ`, and answering the long spelling gave
  *ビョ is byo, not ビヨ. ビョ is one mora — the small ョ; ビヨ is two, ビ + ヨ.* with
  `record=ビョ|ビヨ: missed 1 of 1`, `score=0 / 1` and no "write it" button — a
  digraph is not one character and the board is not offered half an answer;
* all five exercises rendered (`Confusable kana|Yōon きゃ|Yōon キャ|Voiced が|
  Voiced ガ`), and the voicing drill asked `Which one is te?` with `opts=て,で`,
  answering with *て is te, not で. て is the plain kana; で is the same kana with
  the dakuten ゛* and `record=て|で: missed 1 of 1`. The **handakuten** was seen too
  — katakana asked `Which one is ho?` with `opts=ホ,ポ` and answered *ホ is ho, not
  ポ. ホ is the plain kana; ポ is the same kana with the handakuten ゜* — which is
  the half of the voiced rows a rule that only knew about dakuten would have got
  wrong;
* the classic drill's `Write り on the board` opened り in `ら り る れ ろ`, and the
  same path from ク crossed the script toggle on its way;
* and the script handoff was driven **in both directions** after the race fix
  below rewrote it: ヷ from the katakana chart opened with `lesson=V-series`, then
  ゆ from the hiragana chart opened with `lesson=や ゆ よ` — two crossings, each
  landing on the kana's own lesson rather than the first lesson of the course.

One scratch run's file held all three kinds of key at once, which is the record
claim in one image — a classic pair, a voicing contrast and a katakana yōon:

```json
{"pairs": {"ソ|ン":   {"asked": 1, "correct": 0, "wrong": 1},
           "す|ず":   {"asked": 1, "correct": 0, "wrong": 1},
           "ジョ|ジヨ": {"asked": 1, "correct": 0, "wrong": 1}}}
```

**And three frontend races an adversarial review found, all fixed — recorded
because the suite could not see any of them.** The frontend has no component test
harness (vitest runs pure TypeScript in node, deliberately), so a component's state
handling is checked by reading it and by driving it, and these were found by
reading it:

* **The drill could draw one exercise's question under another's chip.** `Next` and
  a chip press can both be in flight, and the older response landing last put a
  きゃ/きや pair on screen under the "Voiced が" note, with the score, the note and
  the "Write it" target belonging to the wrong exercise. `DrillQuestion::kind` was
  added on the Rust side for exactly this ("echoed back so a screen can tell a
  question from the exercise it is showing now from one it left") and the component
  never read it. There is now a ticket per ask, a check that the answer's kind is
  the one asked for, the note derived from the **question's** kind rather than from
  the selected chip, and the previous question cleared when the exercise changes.
* **`openKana`'s script handoff held one un-tagged kana.** A tap that overtook an
  earlier one could be replaced by it — the board ending on the *first* tap — and a
  load that failed left its request to be applied to whichever course loaded next,
  opening one script's kana under the other's lessons. The request now carries its
  script, only a load for that script may consume it, a superseded load is dropped,
  and the rule itself is `focusFor` in `src/lib/kana.ts` with four tests, because a
  race in a component with no test harness is a rule that should not live inline.
* **The chart could stick on "Loading the chart…".** A response for a script the
  toggle had left was assigned to `view` and then hidden at render, so two quick
  toggles arriving out of order left nothing to show and nothing left to re-request
  it. A stale answer is now dropped where it arrives.

The fix for the first two was the same shape in both places — **tag a request with
what it was for, and let only the answer that matches the current ask land** — which
is worth carrying: this app has three asynchronous panels over one IPC boundary, and
"the last click wins" is not something a screenshot or a unit test will notice.

**What is deliberately not here.** The chart is a lookup, not a lesson: it has no
romaji under each cell (the Practice screen shows the reading when the kana opens)
and no stroke-order animation of its own. The voicing exercise covers the grid's
five voiced rows, so **う/ゔ is the one voicing contrast the course teaches and does
not drill** — ゔ is in the rare-kana lesson and the chart offers it, but it is off the
grid and its katakana ヴ belongs to loanword spelling rather than to the ladder the
rest of this course is built on. The drill now covers the three things the course
teaches that a single kana cannot express — the classic confusions, the yōon
contrasts and the voicing contrasts — but it still cannot *discover* a fourteenth
kana pair: the pool grew two new exercises, not a new way of asking about kana, so
the gap the N3 note names stands. Asking about kana generally is a different drill
and a different record, and it is still not built.

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
with KANJIDIC2 across the 2,136 jōyō**, with nine named exceptions *(in fact one:
see the correction above and the Shipped section)*. `prepare-kanji`
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

### Shipped

`crates/nihongo-core/data/kanji.bin.gz` — **2,136 jōyō, 22,367 strokes, 3,074 KB**
committed, built by `prepare-kanji`, byte-identical on a rebuild, and held by 21
tests in `crates/nihongo-core/tests/kanji_artifact.rs` that run without any
upstream download. The pipeline refuses to write an artifact whose jōyō set is not
2,136 characters, whose geometry disagrees with KANJIDIC2's stroke count, or whose
count it could not check against KanjiVG — `--allow-unchecked` has to be passed
deliberately.

**Three things the measurement corrected, and all three were in this file.**

* **The "nine named exceptions" were a measurement artefact, and there is one real
  disagreement.** The nine (謎 賭 葛 餌 遜 僅 遡 餅 牙) are exactly the characters
  where KANJIDIC2 lists **more than one** stroke count and the *first* is not the
  taught one — 91 jōyō do that. Compared against the taught count, all nine agree
  and only 衷 differs (10 by the geometry and KANJIDIC2, 9 paths in KanjiVG).
  `prepare-kanji` records that one with both numbers, so a change either way fails
  the build. The lesson is the file's own: a multi-valued field has no "first".
* **The grade reconciliation is not the 20 prefecture kanji.** Those 20 are all
  grade 4 in KANJIDIC2 and `g7` in AnimCJK, which reconciles the jōyō *total* but
  not the per-grade counts: the +2/+8/+10 come from **39 further characters the two
  sources place in different grades** (夫 央, 21 from 4→5, 胃 腸, 富 徳 群 賀, nine
  from 5→6, 城). Both effects are pinned separately in the artifact test, so the
  next reader cannot re-derive the misleading half.
* **No stroke merging is needed for kanji.** AnimCJK splits a *kana* stroke that
  crosses itself; it does not do that to a kanji, and that is measured two ways
  over all 2,136 — the geometry's array length is always one of KANJIDIC2's counts,
  and for the 91 where a split could hide, AnimCJK's own SVG element ids show no
  split segment. `svgsJa/` is therefore not fetched at all, and the taught stroke
  count is simply the geometry's.

**Also decided rather than assumed.** `dictionaryJa.txt` carries `on`, `kun` and
`definition` fields; the pipeline **does not read them**, because EDRDG is the
attributed authority for readings and glosses and a second-hand copy would
misrecord where the text comes from — and would redistribute unspecified text
under AnimCJK's LGPL. **JMdict is deferred to N7**, where vocabulary is the
milestone: nothing in this artifact's acceptance criteria needs a word list, and
`scriptin/kanji-frequency` is deferred with it, because its per-corpus CSVs need a
merge rule that belongs to the level ladder N7 has to choose and document anyway.
KANJIDIC2's own `freq` (2,037 of the 2,136 are ranked) is what the artifact
carries.

The licence position is recorded in `LICENSES.md`'s "The Japanese kanji data",
including the answer the kana file could not give: AnimCJK's `COPYING.txt` puts
`graphics*` files **and character SVGs** under the Arphic Public License and only
kana/stroke SVGs under the LGPL, so the kanji geometry is unambiguously Arphic.
The EDRDG update obligation is designed there too — the snapshot is pinned, the
artifact records the version and date it was built from, the test pins both, and
the four-step refresh procedure is written down.

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

### Shipped — the words artifact, and the passages

`crates/nihongo-core/data/words.bin.gz` — **16,073 words, 601,816 bytes** —
committed, byte-identical on a rebuild, and held by 15 tests in
`crates/nihongo-core/tests/words_artifact.rs` that need none of the 118 MB of JSON,
63 MB of XML or 33 MB of furigana the pipeline reads. `prepare-words` joins three
upstreams with the committed kanji artifact, which is what decides membership: a
word is carried only when **every one of its kanji is a character the board can
draw**, so the course can never offer a word it cannot teach.

**The ladder is decided, and it is ours.** A word's band is the **highest kyōiku
grade among its kanji** (1–6), with band 7 for a word containing a jōyō-remainder
kanji; EDRDG's `nf` rank orders the words *within* a band, and the unranked ones
follow the ranked. Measured band sizes: **585 / 1,781 / 2,408 / 2,232 / 2,412 /
1,834 / 4,821**. The alternative — frequency-quantile bands of roughly equal size —
was rejected because it mixes grade-1 and grade-6 kanji in one band, so a learner
meets 秘密 before 六. The UI must present these as our approximation, not the JLPT's;
there has been no official JLPT list since 2010. `band_name` exists so that it can,
and `tests/words_artifact.rs` recomputes every band from the kanji artifact rather
than trusting the stored number.

**Readings are the dictionary's, never composed.** 大人 is おとな and not だいじん,
今日 is きょう, 一人 is ひとり, 明日 is あした — the test asserts those four, because
they are what a "reading per character" design gets wrong. Furigana comes from
JmdictFurigana and puts each reading over the characters it belongs to: 16,022 of
16,073 words are aligned (99.7%), and the 51 that are not keep their reading and
carry no ruby — **gathered, never invented**.

**Five things measuring changed, and two of them were this author's errors.**

* **The JSON has no `nf` tags at all**, so the frequency half of the ladder has no
  home in the reformatting this pipeline otherwise reads: the full document carries
  eleven tags total, all spelling-variant markers. EDRDG's own XML is therefore
  fetched as well and read for its priority markers, joined on `ent_seq` (22,430 of
  22,430 ids matched).
* **Membership takes both of EDRDG's signals, because the rank alone drops the
  beginner core.** 行く (to go) and 本 (book) carry `ichi1` — EDRDG's
  most-common-word marker — and **no `nf` rank**: `nf05` belongs to a different
  entry of 本, read もと. 2,471 entries are in that position, and 1,201 of them are
  teachable; they are why `Word::nf` is an `Option`.
* **And the written form has to be one EDRDG marks common.** 7,629 entries are
  common words whose *kanji* spelling is not what anyone writes — 彼処 for あそこ,
  お握り for おにぎり, さっき as 先, 型録 for カタログ — and 173 of those carry an
  explicit `rK`/`sK`/`ateji` tag. Both are now excluded, because a kanji course that
  taught them would teach spellings its learner will never meet. This is why the
  vocabulary is 16,073 and not 21,902: **smaller and right rather than larger and
  wrong.**
* **"Has a kanji form" is not "contains a kanji."** JMdict lists full-width
  numerals such as `１０００` under `kanji`, so 15 words arrived with no kanji
  character at all and were landing in band 1 before the rule was tightened.
* **An intermediate figure for this milestone was wrong, and the error was this
  author's**: it tested *every* character of a word — including its okurigana —
  against KANJIDIC2's grades, which silently excludes every word written with any
  kana, and reported 17,366. The pipeline and an independent re-measurement now
  agree to the word. The handover carries the trap.

### Shipped — the passages

`crates/nihongo-core/data/passages.bin.gz` — **three passages, 502 bytes**, with the
text and the English glosses in `data/passages/*.txt` where a person can edit them,
and 9 tests in `tests/passages_artifact.rs`. They are **written for this course**,
which is a decision rather than a default: Tatoeba is per-sentence licensed (some
CC0, some BY, some **ND**, which would have to be filtered at import) and gives
sentences rather than passages; Aozora Bunko is free but is pre-1930s literary
Japanese that uses none of this vocabulary; Wikipedia is modern and unlevelled.
Writing them removes the licence question and buys the property none of the corpora
have: **the text is held to the vocabulary the course teaches, and asserted**.
`prepare-passages` refuses to write an artifact containing a kanji the words
artifact does not hold, and a test re-checks it against the committed artifact, so a
passage edited afterwards fails rather than shipping.

**The analyser runs at build time**, and that is what keeps the app's promise: the
artifact carries the segmentation, so the shipped app holds no tokeniser, no
dictionary and no network path. `lindera` (MIT) segments with **UniDic**, whose
`unidic-mecab-2.1.2` archive is a **134 MB** download — not the 529 MB CWJ zip this
milestone first budgeted for, because lindera's own dictionary crate uses the MeCab
variant. `scripts/fetch-unidic.sh` fetches and builds it into `.lindera/`
(gitignored, 190 MB built), pinned by an MD5 that lives in `lindera-unidic`'s
`build.rs`; lindera's build script downloads **nothing** unless that script names a
cache directory, so no ordinary `cargo test` or `cargo clippy` touches the network.
UniDic is BSD-3-Clause-equivalent (Copyright (c) 2011-2017, The UniDic Consortium)
and nothing it contains is redistributed; `LICENSES.md` records both notices.

**Two traps the analyser set, both now tests.** UniDic's `reading` field is the
**base** form's reading for an inflected word — 行き gives イク and 食べ gives タベル
— so reading it blindly draws 行き(いく) and 食べ(たべる)ます; the surface's own
reading is in `phonological_surface_form`, which is pronunciation notation and gives
きょー for 今日 and ワ for the particle は, so the pipeline takes `reading` for an
uninflected word and the pronunciation field for an inflected one. And the link a
tapped word opens has to be the **dictionary form** (UniDic's `lexeme`), so 行き
opens 行く and 食べ opens 食べる — and where a token is written exactly as the course
writes it, the course's own reading wins, so a passage saying 私 and the card behind
it cannot disagree about わたし and わたくし.

### Shipped — the screens

The two views the acceptance criteria name, and both were **driven and looked at**
rather than assumed (the recipe is `HANDOVER_NIHONGO.md` §5):

* **Words** — the ladder as seven band chips with their measured sizes
  (585 / 1,781 / 2,408 / 2,232 / 2,412 / 1,834 / 4,821), a paged list of the band's
  words with their readings and glosses, and a card for the selected word: its
  **per-character furigana** — 一方 comes out with いっ over 一 and ぽう over 方 — its
  meaning, its band and EDRDG's frequency block, and a box to type the reading.
  Typing いっぽう for 一方 answers *"Correct — 一方 is いっぽう."*; typing だいじん for
  大人 answers with what it produced, だいじん, and what the word is, おとな.
* **Read** — the passages with a reading over every kanji and every kanji-bearing
  word underlined and tappable, so tapping 私 in 私は学生です。 opens the card for
  私 read **わたし** — the course's reading, not the analyser's わたくし — and the
  gloss for the passage below it.

**The ladder is presented as ours, in two places**: the panel's own paragraph and
every word card say the bands come from the school grades of a word's kanji and
EDRDG's frequency ranking, "not from the JLPT, which publishes no list". A test
asserts that no band name says JLPT, and `tests/words_artifact.rs` recomputes every
band from the kanji artifact, so the label and the derivation cannot drift.

The screens are covered by 32 unit tests, 6 new IPC-contract tests (including the
*request* halves, which is the shape of invariant 14), and 12 frontend tests over the
furigana and paging arithmetic. What N7 does **not** have is a review queue, and
N2 has shipped one since — but it schedules **characters**, not words, because a
word is checked by typing rather than written on a board and so has no handwritten
attempt to schedule. Scheduling the vocabulary is a separate decision with its own
attempt source; the words artifact and `words::of_kanji` are what it would use,
and this milestone does not claim them.

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

### Shipped

All four criteria are met, and one of them needed data the artifact did not have.

**The radical table is in the artifact (format version 2).** The characters carry
the *combining* form of their radical — 扌 in 持 — and a radicals screen needs the
**head form** 手, for all 214, sixteen of which no jōyō character uses. That table
is not derivable from the characters, so `kanji.bin.gz` carries it: all 214 head
forms with their geometry, 1,222 strokes in all, 198 in use. The artifact grew from
3,073,516 to 3,236,713 bytes (+5%), the magic moved to `KANJD002` because
`postcard` writes the new field positionally, and `prepare-kanji` now refuses to
write a table that is not the 214, in order, with usable geometry.

**The numbering is a measurement, not a convention.** `dictionaryJa.txt` tags
exactly **214** entries `radical`, and their file order is the Kangxi number: the
first is 一, the second 丨, the third 丶. That claim was checked against KANJIDIC2's
classical radical number, which agrees for **212** of the 214 and is *absent* for
the other two — 戶 (63) and 靑 (174), which EDRDG files under the other form of the
same radical (戸, 青). Both are written exceptions in the pipeline, checked in both
directions. The entries' own glosses are not read at all, and that too was
measured: **three of the 214 state the wrong number**.

**A character's classification and its annotation are two different things, and
they disagree for 18 characters.** KANJIDIC2's classical radical is the Kangxi
dictionary's index — 巡 is 47 巛 there — while AnimCJK's note says which radical the
character is written with (⻌ (辵)). The panel groups by the classification, because
that is the one that is a number, and the artifact test names all 18 so the other
2,118 are not assumed to agree.

**The course.** Grades in `JOYO_GRADES` order — kyōiku 1 to 6, then the jōyō
remainder — each sliced into lessons of ten, most frequent first inside a grade
(KANJIDIC2's rank, unranked last). 2,136 characters become **216 lessons**, and the
grade labels are the **vocabulary ladder's** names, which answers the question
`HANDOVER_NIHONGO.md` §7 left open: the two are one ladder, so a learner meets a
character and the words built from it in the same band. The first lesson is 日 一 人
年 大 十 二 本 中 出.

**The screens.** `Kanji` — grades, lessons, a character grid, the board and a card
with the readings, the glosses, the radical in both shapes and the IDS components —
and `Radicals` — all 214, ordered by what each unlocks or by number, searchable by
glyph, number or a character that uses it. Every member of every family and every
drawable component opens on the same board, and the board's `grade_attempt` command
now resolves a kana, a jōyō kanji *or* a radical head form, because 92 of the 214
are not jōyō characters and the course cannot reach them.

**Reuse was real, not aspirational.** `hanzi-core::decompose::parse` reads
AnimCJK's IDS strings unchanged — the twelve operators are the same twelve — and
`render.ts` is untouched: `types.ts`'s `Character` became a structural `Drawable`,
which is what let one canvas draw a kana, a kanji and a radical with no edit to the
lifted file. `RadicalsPanel` is new rather than lifted (the Chinese one takes an
injected loader and carries pinyin and etymology this data does not have), but its
shape — the split, the size-first ordering, the family detail — is the same and is
recorded as such.

**What the live check found.** The three screens were driven through a DOM probe
rather than assumed: a synthetic trace of 日's centre-lines graded **100/100** with
all four scores 1.00, a family member opened 会 on the board, and the card's "See
the family" link landed back on radical 9 人. One interface bug was caught this way:
the shared `.tabs` rule capitalises its labels for `hiragana`/`katakana`, so the
grade tabs rendered as *Jōyō Beyond The School Grades* — the ladder's own names,
title-cased into proper nouns. The panel now overrides it and wraps.

**And the one thing the card was still missing, added after the milestone.** The
card showed what a character *is* — its readings, its radical, its components — and
not the words it is *used in*, which is the half of "a character arrives through the
words that use it" the course is built on. It now lists them, from
`WordDataset::of_kanji`, in the course's own order — band, then EDRDG's frequency —
a page of twelve, and each opens that word's own card, the same one `Words` and
`Read` show. Three measurements decided the shape: **一 is written in 223 of the
16,073 words and 人 in 218** (so it pages, like a band), **57 jōyō characters are in
no word at all** (so the card says that rather than drawing an empty list), and a
character outside the jōyō set is an error rather than an empty list, because the
vocabulary holds no word such a character could be in. `HANDOVER_NIHONGO.md`
invariant 28 is the part that must not be undone.

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

* **The kana grader was measured, and one of the two weak spots above was real.**
  `crates/nihongo-core/examples/selfcheck.rs` now exists — the kana counterpart of
  `hanzi-core`'s, printing rather than asserting — and running it settled both open
  questions with numbers rather than instinct. The tolerances carry over; the
  *stroke-count gate* did not, because it refuses a connected hand before shape is
  looked at. `nihongo_core::variants` is the fix, invariant 30 is the rule, and the
  safety property (a dropped stroke must not pass as a join) is the part with the
  most measurement behind it. Two claims in this file were corrected by the run:
  き is taught in **four** strokes and not three, and the nine "KanjiVG exceptions"
  of an earlier draft were already known to be a stroke-count-list mistake. The
  general lesson is the one the watched surfaces taught: **a green suite said
  nothing about this either** — 302 tests, clippy and `svelte-check` were all happy
  while the app refused the commonest way to write さ.
* **The three surfaces nobody had watched — done, and two of the three were
  broken.** The grading verdict, the Licences panel and the katakana tab have now
  been seen running, and watching them was worth more than the fifteen minutes it
  cost: the Grade button could not grade anything (the interface posts
  `{ inkWidth }` and `GradeOptions` rejected the partial object with `missing field
  'resampleK'`, so no verdict had ever reached the screen), and the board threw
  `effect_update_depth_exceeded` on mount and came up blank on about half the cold
  starts. Both are fixed, both now have the test that was missing, and they are
  `HANDOVER_NIHONGO.md` invariant 14 and trap 9. The lesson for the kanji screens
  is the general one: **a green suite, a clean `svelte-check` and clippy said
  nothing about either bug**, so a screen that has never been looked at is not a
  screen that works.
* **The three interface bugs that shipped** are fixed and two are invariants now
  (the board's geometry, the title bar inset), but the *class* of mistake — two
  things deciding one dimension, and a layout that is only wrong on a display —
  is worth remembering when the kanji screens are built. Kanji screens will have
  a taller board and a longer sidebar, which is more of the same. Two more of the
  class were found by watching the three surfaces above: an effect cycle that is
  only wrong at runtime, and an IPC payload that is only wrong in the direction the
  contract test did not check.
* **The Chinese docs now point here, and should keep doing so.** `README.md`,
  `ROADMAP.md` and `HANDOVER.md` each gained a pointer when this file was written;
  until then `grep -c 'nihongo\|kana'` over the three returned 0. If the Japanese
  part is ever renamed or its documents move, those three pointers are what breaks
  first, and nothing tests them.
* **The two Dependabot warnings that stand** (`glib`, `proc-macro-error`) are
  Linux-only and upstream-blocked. Nothing to do; recorded so they are not
  re-investigated.
* **The `assets/website` duplication** — 10 MB carried twice. Decide whether it
  stays a copy.

---

## Known weak spots

* **The grading tolerances were fitted on Chinese characters — and the
  measurement says they carry over.** `grade.rs`'s constants were tuned against
  the real hanzi dataset and carried over unchanged, and the worry was that kana
  are simpler and fewer-stroked, which makes them *noisier* to score. The kana
  `selfcheck` (`crates/nihongo-core/examples/selfcheck.rs`, the counterpart of
  `hanzi-core`'s) answers it: **100.0%** of kana legible at 15/1024 jitter,
  **98.9%** at 30 and **90.4%** at 50, with the worst kana named and all of them
  voiced (が, ぐ, づ, ブ — the mark is small ink); self-consistency perfect for all
  177; the shortest kana stroke 157 units against a 12-unit stray threshold; 0.12
  ms per grade. **The one thing that did not carry over is discrimination**: 12 of
  the 26 classic-pair directions are legible as the wrong kana (れ/わ at 92, ぬ/め
  at 89). That is recorded rather than fixed, and the reason is product rather than
  technical — the board draws the character being copied, and tightening the
  tolerance enough to tell れ from わ would refuse the wobbly hands the tolerance
  exists for. `HANDOVER_NIHONGO.md` §5 is the run and what each row means.
* **Kana handwriting has legitimate variants the app may reject — shipped, and
  the measurement corrected the claim.** さ is taught in three strokes and very
  commonly written in two, but **き is taught in four, not the three this bullet
  used to say**; ふ, そ, な and む have variants too. Nothing had tested any of it,
  and the measurement was blunt: `legible` requires the attempt's stroke count to
  equal the reference's, so **0 of 339** adjacent joins passed and every named
  variant failed. `nihongo_core::variants` now puts the reference into the hand's
  grouping, and **331 of 339** hand-drawn joins are legible at 15/1024 jitter,
  with the eight refusals named (all dakuten joins: ぎ 5+6, ぜ 4+5, ぶ 5+6, ぷ 3+4,
  ズ 3+4, バ 3+4, ポ 2+3, ヹ 1+2). The half that matters is the safety property, and
  it is measured: **0 of 497** single omissions are accepted as joins and **0 of
  95** mid-stroke splits are legible. `HANDOVER_NIHONGO.md` invariant 30 is the
  rule that must not be undone. **Kanji is excluded by decision, not by omission**:
  a kanji keeps the strict taught stroke count and order and accepts no combined
  strokes — kanji stroke order is the thing being taught, and the Chinese app has
  always graded its characters that way. A kanji drawn with two strokes joined is
  refused with the taught count, and `joined` is empty for a kanji and a radical.
* **No grammar, and that is the largest gap in the product.** Particles, the
  copula and every inflectional ending are kana, so a learner who finishes the
  kana course has unlocked grammar and been given nothing to use it on. This is
  not a kana-tutor weakness so much as the reason a kana tutor is not a Japanese
  tutor.
* **The 13 confusion pairs are still a fixed list, and the yōon drill did not
  change that.** N3 weighs them by what this learner gets wrong, which is the half
  that was worth having — and N4's second exercise records its 33 yōon contrasts
  under the same rule and in the same file, so the *record* now covers more than
  the thirteen. What is still not built is a drill that can discover a fourteenth
  *kana* pair (あ/お, say) that this learner actually confuses: the pool grew a
  second exercise, not a second way of asking about kana. See the note at the end
  of N3 and N4's Shipped section.
* **`hanzi-store` and `hanzi-sync` are not for this app, by decision.** They are
  the Chinese app's learner data, and invariant 15 keeps the two apps' data
  separate: shared crates carry code, never user data. So "the measurement says
  they are reusable" was true of the *types* and beside the point for the store.
  `nihongo-tutor`'s own `store.rs` is the precedent N2 followed — and the review
  schedule is a second store in that same file and that same directory.
* **Scheduling the vocabulary is not built.** N2's queue is over characters,
  because a word has no handwritten attempt to schedule: it is checked by typing.
  A word queue would need its own attempt source and its own decision about what a
  wrong reading costs, and the words artifact is the thing it would read.
* **The kanji milestones are plans, not measurements.** The kana estimates in the
  feasibility report were good, but the kanji data sizes are from upstream
  listings rather than from a build, and the artifact size is unknown. *(N8 made
  this one measurable: 3,236,713 bytes, +163 KB for the radical table.)*
* **A radical's classical number and the radical a character is written with are
  two different things, and they disagree for 18 characters.** 巡 is radical 47 巛
  in the Kangxi dictionary — which is where a Kangxi-ordered dictionary files it —
  while AnimCJK's note says ⻌ (辵); 郭's note says 阜 while KANJIDIC2 classifies it
  under 163 邑, on the correct side of 阝. The panel groups by the **number**,
  because that is the classification and it is the one the artifact is keyed by,
  and `tests/kanji_artifact.rs` names all 18 so the other 2,118 are not assumed to
  agree. A panel that grouped by the note instead would need a second, mixed rule.
* **The kanji course has words attached to a character now — shipped.** A card
  showed the readings, the glosses, the radical and the components, and it now
  also lists the vocabulary the course teaches that uses the character, each
  opening its own word card (N8's Shipped section, `HANDOVER_NIHONGO.md`
  invariant 28). This bullet stood here while `words::of_kanji` went unused; it is
  recorded rather than deleted so a reader of the earlier version can see it was
  answered.
* **The nine jōyō where KanjiVG and KANJIDIC2 were said to disagree are not
  real.** The measurement was corrected while building the data layer: 謎 賭 葛 餌
  遜 僅 遡 餅 and 牙 are the characters where KANJIDIC2 lists **two** stroke counts
  and the first is not the taught one, so comparing against "the" count invented
  nine disagreements and hid the one that does exist. **衷** is the single written
  exception — 10 strokes by the geometry and KANJIDIC2, 9 paths in KanjiVG — and
  `prepare-kanji` checks both numbers. The nine are pinned in
  `tests/kanji_artifact.rs` so the artefact cannot come back.

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
