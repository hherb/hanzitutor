# Roadmap

What to build next, in priority order. Each milestone states **why** it matters,
**how** it would be approached, and **acceptance criteria** — so "done" is
unambiguous.

Sizes are rough: **S** under an hour, **M** about half a day, **L** multi-day.

See [`HANDOVER.md`](HANDOVER.md) for how to build, test and verify; see
[`README.md`](README.md) for how the app and its grading engine currently work.

## Summary

| # | Milestone | Why it matters | Size | Status |
| --- | --- | --- | --- | --- |
| M1 | Personal vocabulary list | Track and drill your own lesson material | M | **done** |
| M2 | Per-character progress + spaced repetition | Practice history still does not persist | M | **done** |
| M3 | Words and sentences | Single characters are not reading | L | **done** |
| M4 | Raster legibility (IoU) | Catches errors centrelines cannot | M | **done** |
| M5 | Distribution readiness | Licence notices and signed bundles | M | **done** |
| M6 | Pronunciation on Windows/Linux | macOS-only today | S | not started |
| M7 | Centreline stroke animation | Nicer, more accurate "show me" | S | **done** |
| M8 | Input ergonomics | Long strokes on a trackpad | S | **done** |
| M9 | Mobile shells | A stylus is the right input device | L | **in progress (iOS + Android)** |
| M10 | Durable study store (SQLite) | The JSON format caps the attempt log the grading work needs | M | **done** |
| M11 | Tone practice (speech recognition, model-free) | Tone is the error handwriting cannot see, and it needs no model | L | **done** |
| M12 | Speech recognition: text (optional download) | Recognise *what* was said, which needs a ~155 MB model the user installs from settings | L | **done** |
| M14 | Graded phrase audio (MeloTTS) | The app can pronounce graded phrases in one consistent voice, and speak any phrase on demand | L | **in progress** |

---

## M1 — Personal vocabulary list

**Status: done.**

Records the characters and words met in your own lessons, files them under your
own group names, and drills exactly those. `crates/hanzi-core/src/vocab.rs` holds
the store (no Tauri dependency, so it stays testable without a window),
`src-tauri/src/commands.rs` exposes it, and `src/lib/VocabularyPanel.svelte` with
the sidebar renders it.

What shipped:

- Entries of one character or several, each with pinyin, meaning and a group.
  A single character fills its reading and meaning from the bundled dataset. A
  word gets its reading **composed** from its characters — word pinyin is the
  readings run together, so 学习 → `xuéxí` — but its meaning is deliberately left
  blank, because a word's meaning cannot be derived from its parts and a
  plausible-looking invention would be worse than an empty field. The
  per-character readings and meanings are shown as a hint to write from.
- Groups are the user's own lesson names. Removing a group keeps its entries and
  leaves them unfiled by default; deleting them is a separate, confirmed action.
- Drilling one group or the whole list, reusing `grade_attempt`. A
  multi-character entry is written one character at a time and scored by the mean,
  so one good character cannot carry a word.
- Persistence as readable JSON in the application data directory, written
  atomically. A file that cannot be parsed raises a warning and **blocks saving**,
  so a parse error can never silently erase the list.
- JSON export and import (lossless; merge or replace) plus CSV export, through
  native file dialogs.

**Follow-up: the card grew a third action, and the phone row was the constraint.**

An entry had Edit and Remove as words; Practise is now the third action, and all
three are glyphs so they hold one row. Making them fit turned out to be a layout
change rather than a smaller button: at a 360 px phone the panel is 320 px
(`main` is padded 20 px a side), and three actions beside the group chip and the
practice record left the reading column **zero** width — the card fitted and the
reading vanished. The chip and the record moved under the reading, where they
wrap, and the glyph is capped so a long entry cannot take the row.

Alongside it, the pinyin field gained a tone key row, because `xuéxí` is two taps
per accented vowel on a phone: `mark_tone` writes the mark into the syllable at
the cursor, with the placement rule in `pinyin.rs` beside the code that reads
those marks back. Each field's action key now advances to the next rather than
submitting, so the key does what `enterkeyhint` says it does. See HANDOVER §6 for
both, and for why the keyboard's *language* is the one thing an app cannot touch.

Deliberately left out, and why:

- **CSV import** — CSV export exists for spreadsheets but there is no matching
  import, because it is lossy. JSON is the round-trip format.
- **Scheduling** — no due dates yet; see M2.
- **Word meanings and tone sandhi** — a word's reading is composed from its
  characters, so a tone change such as 你好 → `níhǎo` is not applied and the
  meaning is not filled in at all. Both want a real word dictionary; see M3.

---

## M2 — Per-character progress and spaced repetition

**Status: done.**

Practice history now persists, and what is due comes back on its own.
`crates/hanzi-core/src/progress.rs` holds the schedule (no Tauri dependency, so it
stays testable without a window), `src-tauri/src/commands.rs` exposes it, and the
sidebar and board surface it.

What shipped:

- **One card per character**, keyed by the character itself, holding attempts,
  lapses, best and last score, the last attempt time, a bounded recent history
  (the newest 20), the interval, the ease factor and the due date. Both sources
  write to the same card: a character met in a lesson and the same character met
  inside a word are the same thing to learn.
- **SM-2 behind a `Scheduler` trait.** An attempt's 0..=100 score becomes one of
  *again / hard / good / easy* using the same bands the feedback panel already
  shows (`Grade::from_score`), and the rating sets the next interval: a failure
  returns within the minute, a pass starts at twelve hours, a day or two days
  (so a good attempt is always scheduled further out than a poor one), then 1 day,
  6 days, and the previous interval times the ease factor, capped at a year.
  A second `Scheduler` implementation is exercised in the tests to prove the
  policy is genuinely swappable.
- **A review queue** drawn from both the course and the vocabulary list, most
  overdue first. A character that belongs to a word is offered as that word, once,
  however many of its characters are due; a character in neither source is skipped.
  The interface takes the most overdue 20 as one session and reports the true
  total beside it.
- **Per-lesson completion in the sidebar** — practised/total per lesson, a mark on
  each practised character, a dot on anything due — and the **course cursor** is
  persisted so the app opens where you left off, which also fixes how easy it was
  to end up far from where you were while exploring.
- **Three separate files**, deliberately: `vocabulary.json`, `progress.json` and
  `course-cursor.json`. A corrupt schedule cannot lose your place, and a corrupt
  cursor cannot take your history with it. All three follow the M1 rule — a file
  that cannot be parsed is reported, never overwritten — now enforced in one place
  (`Persisted<S>` in `src-tauri/src/state.rs`) instead of three. (**M10 replaced
  the three files with one database.** The rule survived, and so did the
  isolation: the import of those files is per document, so one unreadable document
  still blocks only its own store.)
- Timestamps are ISO-8601 UTC **strings**, so "is this due?" and "which is most
  overdue?" are string comparisons; `crates/hanzi-core/src/time.rs` owns the
  formatting, parsing and whole-second arithmetic, including the range guard that
  stops a runaway interval from formatting a year the parser could not read back.

Deliberately left out, and why:

- **FSRS** — it wants a review history far longer than one learner produces
  quickly. SM-2 is behind a trait precisely so this can be revisited with data.
- **Export/import of the schedule** — the vocabulary list has it; the schedule is
  derived from practice, so rebuilding it is cheap and a merge format would be
  guesswork. The files are plain JSON if a backup is wanted.
- **Per-character notes or a browsable history screen** — the board shows attempts,
  best score and the next review, and the sidebar shows what is due. A history
  browser would want the attempt log that M4's tuning also wants; defer to the
  cross-cutting "attempt logging" item.

---

## M3 — Words and sentences

**Status: done.**

Reading is more than single characters, and a word is also what disambiguates a
polyphonic character. The word list now ships in the same artifact as the
characters, `crates/hanzi-core/src/dataset.rs` indexes and searches it,
`src-tauri/src/commands.rs` exposes it, and `src/lib/WordsPanel.svelte` with the
sidebar renders it.

What shipped:

- **9,443 HSK 3.0 words** in the artifact, each with its characters, its own
  reading and an English definition. Words are the multi-character entries only:
  a single character is the course's job, and the character dataset already holds
  its most common reading, so a second entry for the same glyph would be a
  competing source of truth.
- **Every word is practisable**: a word is kept only when all of its characters
  have stroke geometry and a frequency rank, which is checked at preparation
  time — `0 words with an unteachable character` is part of `prepare-data`'s
  output. Writing a word reuses the existing one-character-at-a-time flow, with
  per-character feedback and the mean as the entry's score; nothing about the
  grader changed.
- **The dictionary now answers for a word, not just for a character.** The
  lookup used by the vocabulary add form returns the dictionary's reading and
  meaning, so 着急 is `zháojí`, not the `zhe` the isolated 着 gets, and a word's
  meaning is a definition rather than a blank. A word the dictionary does not
  know still gets its readings composed and its meaning left empty — inventing
  one is still refused.
- **Search by character, reading or meaning.** Readings match with or without
  tone marks and with `v` for `ü` (`xuexi`, `xuéxí`, `xüexi`), and results are
  ranked so an exact word beats a prefix, which beats a reading, which beats a
  definition. A single character is a browse-by-character request: every word
  containing it, most useful first. Every character in a result is clickable and
  performs that search, which is the natural way to browse once a character is
  known. One capped page of 100 is returned with the true total beside it, and
  the sidebar narrows everything to one HSK level.
- **Speaking a word speaks the word**, which is the whole fix for polyphonic
  characters: the synthesiser gets context, and `着急` is heard as a word.
- **Sentences work through the same path without any sentence data.** Any
  multi-character text — in the list, or drilled straight from the dictionary —
  is written one character at a time, and characters the board cannot draw
  (punctuation, an unknown glyph) are skipped rather than dead-ending the board.
  There is no segmentation, and no sentence corpus: see below.
- **A box per character of a multi-character entry**, added after the milestone
  shipped: the boxes under the board show a miniature of each attempt already
  made, with its score, and an empty box for each character still to write. Any
  box can be clicked to put that character back on the board, which means a word
  no longer has to be written strictly in order — it is recorded once every
  character has a grade.
- **Data and notices.** The word list comes from the MIT-licensed
  `complete-hsk-vocabulary`, whose readings and definitions are drawn from
  CC-CEDICT under **CC BY-SA 4.0** — the first obligation here that reaches the
  derived data rather than only the notices. It is recorded in `LICENSES.md`,
  the licence text is fetched by `scripts/fetch-data.sh`, and the fields that
  would have added a third licence (SUBTLEX-CH frequency, HanLP
  part-of-speech) are deliberately **not** bundled. The word rank the app shows
  is computed locally from the MIT character frequency list.

Deliberately left out, and why:

- **Sentence data and segmentation** — a sentence needs to be cut into words and
  characters, and the useful version of that is a segmentation model or a tagged
  corpus, which is a different project from the grading engine. The practice
  flow already handles arbitrary text, so the missing half is *which* text to
  study, and that is what the user's own vocabulary list is for. Revisit as its
  own milestone if a corpus appears.
- **Word meanings inside CC-CEDICT's share-alike are the only dictionary text
  bundled**; the app carries no glosses of its own, and the definitions are
  displayed verbatim rather than rewritten, so nothing new is derived from them.
- **Frequency data from SUBTLEX-CH** — it is free for research rather than for
  redistribution, so it is not used even though the upstream file carries it.
- **A per-word progress card.** Progress is still per character, which is what
  the schedule is built on; a word practised from the dictionary records its
  characters and leaves no entry behind. Adding word-level cards would want the
  attempt log that M4's tuning also wants — the cross-cutting item again.

---

## M4 — Raster legibility (IoU)

**Status: done.**

Grading now also measures the ink, not only the path. `hanzi-core/src/raster.rs`
holds a small pure-Rust rasteriser (no new dependency), `grade.rs` folds the
result into every `GradeReport`, and the feedback panel shows it.

What shipped:

- **A scanline rasteriser for both sides.** The attempt is drawn as round-capped
  pen strokes at the width the canvas paints them with — `INK_WIDTH` in
  `src/lib/render.ts`, passed through `GradeOptions.inkWidth` so the two cannot
  drift apart. The reference is the stored stroke outline, filled — the same SVG
  paths the interface draws as the faint guide. 256 cells across the box (4
  design units each) is plenty, and a grade costs **0.8 ms** including both,
  against a budget of 20.
- **Two measures, because one conflated two things.** *Ink amount* — the
  attempt's inked area against what a correct trace at the canvas pen width would
  put down — is the headline (`inkScore`, and `ink` on every stroke). It is blind
  to where the ink went on purpose: placement is graded separately, and judging
  it twice is a double-counted fault. *Coverage* — how much of the character's own
  ink was reached — is reported (`inkCoverage`) as the "you never drew that part"
  signal, and deliberately not scored.
- **A `faint` verdict.** The right stroke in the right place with too little ink
  is now named, coloured and explained, where before it was indistinguishable
  from a perfect stroke.
- **The headline score has four equal quarters**, `0.25` each for shape,
  placement, ink and order, all exact binary fractions so a flawless attempt still
  sums to exactly 100. Before this, a character drawn with a third of its ink
  scored 92 and read "Excellent"; it now scores 85 and is not legible.
- **`selfcheck` reports it, and it is what tuned the bars.** Self-consistency is
  still 100 for all 7,744 teachable characters, with an ink score of exactly 1.000
  and no faint stroke anywhere — that is what keeps "perfect" reachable. A pen a
  third of the width scores 0.31 and puts all 7,744 below the bar; a stroke drawn
  three times too long is faint on 1,774 characters.

Deliberately not as originally planned, and why:

- **Not intersection-over-union.** IoU against the outline was the plan and it is
  wrong, measurably: a perfect trace tops out near 0.7 against a calligraphic
  glyph, so "perfect" would be unreachable; and worse, IoU also falls when a
  correctly-sized band lands slightly off the guide. Jittering a right-width trace
  by 3% of the box dropped the ratio to 0.47 and marked **96%** of sloppy but
  correct attempts illegible, against 1% before. Splitting it into area (amount)
  and coverage restores the tolerance table to 99.1% at 3% jitter while still
  failing a third-width pen on every character. The measurement is in
  `selfcheck`'s tolerance table and the reasoning is in `raster.rs`'s module docs.
- **No new dependency.** `tiny-skia`, `lyon` and `usvg` were the candidates, but
  the dataset's outlines use only absolute `M`, `L`, `Q`, `C` and `Z` in a single
  simple closed subpath, so a scanline fill is about eighty lines and the pen is
  an exact distance-to-segment test. Nothing is gained by 20 transitive crates.
- **The thin-stroke case is not reachable from today's trackpad.** The canvas
  paints every stroke at one fixed width, so a learner cannot put down less ink
  than that; the width discrimination is proven by tests and over the whole
  dataset, and it starts to bite the moment input can report a real pen width — a
  stylus, or M8's velocity-based brush. What M4 *does* catch in daily use is
  overshoot and short strokes, proportionally, and it is visible in the score.
- **Coverage does not gate legibility.** It would have cost 5% of the 3%-jitter
  tolerance row for nothing, since the outline a wobbling stroke misses is a
  placement fault the position score already owns.

---

## M5 — Distribution readiness

**Status: done.**

The app can now leave this machine: the notices travel inside it, a fresh clone
builds without downloading anything, and CI runs the suite on every push.
`src-tauri/src/licences.rs` is the catalogue, `src/lib/LicencesPanel.svelte` is
the screen, `scripts/build-release.sh` is the build.

What shipped:

- **The notices are a catalogue, not a copy-paste.** `src-tauri/src/licences.rs`
  names every one — the app's own AGPL text, this project's provenance record, the
  Arphic and Make Me a Hanzi pointers, the LGPL, the MIT texts, the CC-CEDICT
  attribution, the CC BY-SA 4.0 legal code, the font's OFL, and the *code* notices
  added since (SQLite, `rusqlite`, `cpal`, `sherpa-onnx`, ONNX Runtime, espeak-ng
  and the Android libraries) — each with what it covers, where it came from and
  where its bundle copy sits. The text is pulled
  in with `include_str!`, so it is **compiled into the binary** and cannot go
  missing at packaging time.
- **`src-tauri/tests/licences.rs` holds the three-way correspondence together.**
  It fails if a file in `licences/` is not catalogued, if a catalogued file is
  missing, if the file on disk differs from the text compiled in, or if
  `tauri.conf.json`'s `bundle.resources` does not copy exactly that set. Adding a
  notice therefore cannot be half-done — which is the failure mode the roadmap
  warned about, where the omission "only shows up in the packaged app".
- **An About and licences screen**, the fourth entry in the sidebar, listing the
  app's name, version, licence and repository above each notice with its full
  text. It is fed by two new commands, `app_info` and `licence_notices`, whose
  JSON shape the IPC contract test pins. The screen shows addresses as text
  rather than as links, deliberately: the app's promise is that it makes no
  network requests, and a click that navigated the single webview to a website
  would both break that and strand the window there.
- **The Arphic Public License is now actually bundled, and was not before.**
  `fetch-data.sh` had been fetching Make Me a Hanzi's `COPYING`, which *names* the
  Arphic licence and points at its text — a pointer is not the licence a reader is
  entitled to. The real text is now fetched from `APL/english/ARPHICPL.TXT` and
  ships as `licences/Arphic-Public-License.txt`. The CC BY-SA 4.0 legal code was
  missing for the same reason and now ships too, with a hand-written CC-CEDICT
  notice that records what the pipeline changed, as the licence requires.
- **The generated artifact is committed** — the open question this roadmap left
  from M3, decided the other way. It is ~13 MB, and committing it means a clone
  and a CI run go from `pnpm install` straight to a build, with no 33 MB download
  and no `fetch-data && prepare-data` step. `.gitignore` records the reversal and
  how to regenerate it. The 33 MB of upstream text stays ignored.
- **The interface font ships too.** Noto Sans SC (SIL OFL 1.1, ~17 MB, variable
  weight) is committed and declared in `src/app.css` ahead of the system CJK
  stack, so Chinese rendered as text looks the same on a machine with no CJK
  fonts installed. The family is named `Noto Sans SC Bundled` so an installed
  copy of Noto cannot be substituted for the one this bundle licenses.
- **`pnpm run build` produces a signed `.app` and `.dmg`.**
  `scripts/build-release.sh` takes the identity from `$APPLE_SIGNING_IDENTITY`,
  else the first `Developer ID Application` certificate in the keychain, else
  ad-hoc — which still launches locally, since Apple Silicon refuses a completely
  unsigned binary. The identity is not written into `tauri.conf.json`, because
  that file is committed.
- **CI**, `.github/workflows/ci.yml`: `pnpm test`, `check:rust` and `check:web` on
  push and pull request, on a macOS runner, with no data step.
- **The version string is pinned in one test.** `Cargo.toml`, `tauri.conf.json`
  and `package.json` each carry a version, and the About screen reports a fourth
  copy; `tests/licences.rs` fails if they disagree.

Deliberately not done, and why:

- **Notarisation.** It needs Apple credentials and uploads the build, so it is
  left as an operator step with the required environment variables documented in
  `README.md`. The bundle is signed, so the only consequence is the standard
  right-click-Open on a Mac that has never seen the build.
- **No links out of the licences screen.** Tauri does not open external URLs
  without the opener plugin, and adding a capability so a licence screen can
  browse the web is a poor trade for an app whose stated value is that it is
  offline and private. This is why the full legal texts are bundled rather than
  referenced.
- **No bundled speech.** Apple's voices cannot be redistributed, so the only way
  to make pronunciation independent of the system voices would be a new
  open-source synthesiser and a ~50 MB model. The app already degrades honestly
  when no Chinese voice is installed, and a current macOS ships several. A
  decision for a future milestone, not a gap in this one.

---

## M6 — Pronunciation on Windows and Linux

**Why.** `src/speech.rs` implements macOS only; elsewhere it returns a clear "not
implemented" error rather than shelling out to something unverified. The app is
meant to be cross-platform.

**Before starting this, settle the App Sandbox question** (see *Known weak
spots*): if `/usr/bin/say` is refused from a sandboxed build, the macOS backend
needs `AVSpeechSynthesizer` in-process, and that is the same shape the iOS
backend needs in M9. Discovering it after writing three platform backends would
mean writing four. The alternative that avoids the whole question is the
pre-rendered audio pack in the speech research document, which takes synthesis
off the runtime path for everything the app itself teaches.

**Approach.**

- Linux: `spd-say` (speech-dispatcher) with `espeak-ng` as a fallback. Language
  selection differs per backend, so probe for what is installed, mirroring the
  `parse_voices` / `pick_voice` structure already used for macOS.
- Windows: PowerShell with `System.Speech.Synthesis.SpeechSynthesizer`, selecting
  a `zh-CN` voice and installing one if absent.
- Keep the `Speaker` interface unchanged so nothing above it moves. Add the
  backend behind `#[cfg(target_os = ...)]` and keep the pure parsing logic
  (voice list → chosen voice) testable with fixtures on every platform — that is
  the part that had the real bug on macOS.

**Acceptance criteria.**

- Speaking works on at least one Linux and one Windows machine, verified by
  someone with access to them (do not claim it works from a macOS-only test run).
- Voice-list parsing for each platform has fixture-based unit tests that run on
  all platforms.
- Where no Chinese voice exists, the UI still disables the control and explains.

---

## M7 — Centreline stroke animation

**Status: done.**

"Show stroke order" now *writes* the character instead of switching it on a
stroke at a time: a pen travels along each stroke's centre-line and the stroke's
outline appears behind it, so *how* a stroke is drawn is visible and not only
*which* strokes there are. The drawing is in `src/lib/render.ts` (`Sweep`,
`sweptBand`, `strokeRadii`), the clock is in `src/App.svelte`
(`playStrokeOrder`, `strokeTimeline`), and `PracticeCanvas.svelte` passes the
pen's position through to each frame.

What shipped:

- **The outline is clipped to the swept region, not faded in.** A pen walks the
  centre-line by arc length — never by sample index, because the samples are not
  evenly spaced — and the stroke's outline is filled, clipped to the band the pen
  has covered. The band is one path: a disc at every centre-line point the pen has
  passed, and a quad joining consecutive ones, which union under the non-zero
  winding rule. The stroke is filled whole the instant the sweep completes, so
  the last frame is the true outline rather than the band's leftovers.
- **The band's width is measured from the stroke, not guessed.** It has to cover
  the stroke's full width or the outline's edges arrive late in disconnected
  fragments, which looks like a rendering fault; and a constant wide enough for
  the widest stroke flashes a short 点 in whole, which is the popping this
  milestone exists to remove. So each stroke's half-width is measured once, on
  the first frame of its first animation, by walking outward from points along its
  centre-line with `ctx.isPointInPath` until the outline is left, and cached per
  character. Measured across the shipped data (87,609 strokes), an outline point
  sits 34.8 units from its centre-line at the median, 55.0 at the 90th
  percentile, 85.7 at the 99th and 274 at the worst — a range no single constant
  covers.
- **The pace follows the stroke, and the whole character is bounded.** Each stroke
  takes a bounded time proportional to its centre-line's length (a 点 is not a
  blink, a 捺 is not a wait), with a short pen-lift between strokes; the timeline
  is then scaled once so that even 囊 finishes inside seven seconds.
- **Stopping is a real cancel.** The button is a toggle — "Show stroke order" /
  "Stop" — and `S` toggles too. Navigating to another character, switching mode,
  clearing the board, or simply starting to write all end the animation at the
  end of the stroke the pen is on. Cancellation is one token checked after every
  `await`, and the loop is driven by animation frames rather than a timer, so
  nothing is left running when it stops.
- **Nothing runs while it is idle.** The width measurement is lazy and cached;
  with no pen on the board there is no timer, no frame loop and no work in the
  paint path beyond what the board already did.

Verified by capturing the app's own window: a stroke half-revealed with the pen
at its head and the ink contiguous behind it — 的 with its 白 written and its 勺
still to come, and, in a run that stepped through the course, 在 and 我. Drawing
cannot be automated here (HANDOVER §6), so the animation was started from a
temporary seed in the same function the button calls; the log showed exactly one
start per seed, a clean stop mid-animation, and no further strokes after a stop
or a navigation.

Deliberately not as originally planned, and why:

- **The sweep is not a clip against the outline's own arc length.** Revealing the
  outline progressively along *its* contour would trace the shape's edge rather
  than the stroke's path, which teaches nothing about direction. Clipping against
  the swept centre-line is what shows the pen going the right way.
- **The whole character is not revealed faintly underneath.** In trace mode the
  guide is on the board anyway and the animation replaces it; drawing it under the
  sweep as well would hide the very thing the animation is demonstrating.
- **There is no speed control *here*.** One pace, proportional to each stroke,
  bounded for a whole character. A learner who wants to go slower has the
  step-through button and the ghost. (**The settings screen added `Slow` and
  `Fast` afterwards**, and it scales the whole timeline — bounds included, so
  `Slow` is genuinely half speed for a character whose strokes are already at
  `MAX_STROKE_MS`. The *default* is still the single pace this milestone chose.)

---

## M8 — Input ergonomics

**Status: done.**

Drawing was press-and-drag only, which is awkward on a trackpad: a long stroke
means holding the button down for a long time. There is now a **click to draw**
mode, off by default, in which one click starts a stroke, moving the pointer
extends it with no button held, and a second click ends it. The state machine
lives in `src/lib/PracticeCanvas.svelte`; `App.svelte` owns the setting.

It first shipped as a quick switch on the board's control row, beside the
*corrections* checkbox. The preference has since moved to the settings screen —
the one place with room for the sentence that explains what the two gestures are
— and the corrections toggle is now the row's labelled **Show target** switch, so
nothing left on the row needs a paragraph to explain it.

What shipped:

- **Two ways to draw, one path.** Both modes start their stroke with
  `toDisplay` and extend it with the same `push` (same display-space conversion,
  same `MIN_SPACING` filter, same `MAX_POINTS` cap), so the points that reach the
  grader do not depend on which one drew them — only what starts and ends the
  stroke differs. That is asserted below, not assumed.
- **A hover stroke is really open.** In click-to-draw mode the release is *not*
  the end: the stroke stays open across as many pointer moves as it takes, which
  is the whole point, and the ink is painted live the same way a drag paints it.
- **Escape or Backspace abandons an unfinished stroke.** Both listeners already
  exist and mean something else in the app, so the cancel listens in the *capture*
  phase and stops propagation: with a stroke open, Backspace cancels that stroke
  and does **not** also undo the last committed one. With no stroke open, both
  keep their old meaning.
- **Switching mid-stroke keeps the ink.** Toggling the mode with a stroke open
  commits it rather than dropping it, because it is already on the board and
  discarding it would look like the app losing the learner's work. What is *not*
  touched is the committed attempt: the mode switch neither clears the board nor
  invalidates the recorded strokes.
- **A cancelled pointer commits.** `pointercancel` (a system gesture taking the
  pointer, a window losing it) finishes the open stroke instead of leaving ink
  that nothing can finish.

Verified by driving the real handlers with synthetic `PointerEvent`s of the same
canvas coordinates in both modes — the sequence a click-to-draw user performs,
with no button held for the moves. The log from that run: the drag produced one
stroke of four points; the click sequence produced nothing at the release, then
the same four points at the second press, **byte-identical to the drag stroke**;
Escape left zero strokes; Backspace with one stroke already committed left one;
and switching the mode mid-draft committed exactly one. The board was captured
with the click-drawn stroke on it and the switch checked.

What synthetic events cannot prove is the operating system's own delivery of
hover moves, so the last step was a human at the trackpad: the author confirmed
it, in their words, as *"much better UX with the click then draw then click to end
on the trackpad"* — which is the acceptance criterion this milestone was written
for, and the one thing about it that no test in this repository can establish.

Deliberately not done, and why:

- **The default was left to the device, and then remembered.** Clicking was the
  better gesture on a trackpad from the day it landed, so the mode now starts
  *on* wherever there is a hover — a mouse or a trackpad — and stays *off* for a
  finger or a stylus, which have no hover to plan a second click with. The
  learner's own flip wins from then on and is stored in the study database's
  `settings` table, so the app stops second-guessing them. That storage and the
  two commands behind it arrived with the settings seam described under
  [Cross-cutting polish](#cross-cutting-polish); only the dialog is missing.
- **No velocity- or pressure-based stroke width.** The roadmap called it
  "cosmetic only", and that stopped being true in M4: the canvas tells the grader
  the width it painted with (`GradeOptions.inkWidth`) and **ink amount is a
  quarter of the score**, so a brush that thinned with speed would make a fast
  stroke score worse for being fast — the same class of fault invariant 15
  exists to prevent, one level up. A real variable-width brush therefore needs
  the grader to take a width per stroke, and probably to re-tune `INK_OK` against
  real attempts, which is the attempt log's job (M10). This is a decision to make
  deliberately, not a pen stroke to slip into an input change.
- **The mode is not remembered between runs.** Nothing else about the board is
  either — trace/recall, the corrections switch — and there is no settings store
  to put it in. M10 is the natural home for one.

---

## M9 — Mobile shells

**Status: in progress, iOS and Android.** Both build, run, draw and speak. On
Android a signed release bundle exists and has been run on a device; what is
outstanding there is the Play submission itself (a published privacy policy, the
listing, the Console questionnaires). On iOS the release build is still
outstanding. What is verified, and how, is below.

**Why.** A touchscreen with a stylus is the right input device for handwriting
practice; a trackpad is a compromise. Tauri 2 supports iOS and Android. On this
project's own terms the motivation is sharper than "mobile would be nice": the
iPhone is carried when the iPad is not, and practice on it is the point.

**What is done.**

- **The Rust side cross-compiles unchanged.**
  `cargo check -p hanzi-tutor --target aarch64-apple-ios` succeeds with the
  engine, the bundled SQLite, the embedded 13 MB dataset and Tauri — nothing in
  `hanzi-core`, `hanzi-store` or the command layer needed a platform `cfg`. The
  only iOS-specific tidying was three dead-code warnings in `speech.rs`, whose
  macOS-only pieces were not gated tightly enough for the target's `-D warnings`.
- **It runs on the simulator.** Built with `tauri ios build --debug --target
  aarch64-sim`, installed with `simctl install`, launched, and looked at: the
  course loads ("7,744 characters in 775 lessons"), the character and its
  readings come over IPC, the data directory resolves inside the app sandbox, and
  the board draws. The screenshot is the evidence, and the traps that got in the
  way are in `HANDOVER.md` §6.
- **A phone layout, made for practice.** Below 760px the shell becomes one
  column in the order practice happens — a top bar with a navigation button, the
  character and its reading, the *board* filling the width, the controls at
  touch size, and only then the explanation or the graded report. The sidebar
  becomes a sheet over the board, safe-area insets are honoured, the keyboard
  hint is dropped (a phone has no Enter key), and the viewport refuses zoom so a
  double tap on the board cannot magnify it. Verified by simulator screenshots.
- **The device default from M8 pays off here.** On a touch device the app starts
  on *drag* and leaves click-to-draw off — visible in the simulator screenshot —
  which is the right gesture for a finger and is exactly what that rule was for.

- **Android runs on a phone and an emulator, and was verified by hand.** The
  whole chain works on a physical NX809J: it installs, starts in 262 ms, loads
  the course, renders the guide for 的, takes a stroke from a real touch, grades
  it, writes the attempt to the study database and puts the character in the
  review queue. An `adb shell input swipe` across the board is the evidence for
  the stroke, and the board's own counter going `0 / 8` → `1 / 8` with
  Undo/Clear coming alive is the evidence that the platform delivered it as
  pointer events. Six things had to be fixed first, all of them recorded in
  `HANDOVER.md` §6: the missing `cdylib`, AAudio's API 26 floor, Gradle's
  inability to find the `tauri` CLI under pnpm, a startup race that made the
  frontend's first commands fail, an unstripped 212 MB native library, and a
  status bar the page could not measure.
- **Android pronunciation, through the system synthesiser.** A Kotlin plugin
  (`PlatformPlugin`) owns a `TextToSpeech` engine and is called from Rust over
  Tauri's mobile-plugin bridge — the same seam, and the same shape, as the iOS
  `AVSpeechSynthesizer` backend. The phone reports 16 Chinese voices and the
  automatic choice is `cmn-cn-x-ccc-local`, a **local** voice: the Kotlin side
  sorts on-device voices ahead of network ones precisely so that an offline app
  does not quietly need a network to pronounce anything. "Hear it" is live
  rather than greyed, and `speak` resolves without error. Confirmed by ear.
- **Which voices need a network is said in the settings screen.** Android offers
  a network voice beside an on-device one for the same locale — 9 offline and 7
  needing a connection, on the test phone — and the app carries that flag all the
  way to the list the learner chooses from, because this app on a train is the
  whole point. The automatic choice is an on-device voice, so the default is
  right; the flag is there for the person who goes looking.
- **The microphone works on Android as well.** `cpal`'s AAudio input does **not**
  — it opens a stream, reports success and then never calls back — so capture is
  per-platform: `cpal` everywhere else, Kotlin's `AudioRecord` over the platform
  bridge on Android, with the samples written to a scratch file and deleted after
  they are read. `HANDOVER.md` §9 has the AAudio log and why the source is
  `VOICE_RECOGNITION`. Verified on the phone end to end: hold the button, speak,
  and the panel draws the pitch against the tone's template; the recording keeps
  running for as long as the button is held, which took three separate fixes
  (§9) — the button moving out from under the finger, `touch-action`, and
  Android's long-press gesture cancelling the pointer at 555 ms.
- **The back gesture belongs to the app, not to Android.** `MainActivity` asks
  the page whether it used the press and only finishes the activity when the
  answer is no, so back closes the navigation sheet first — the same thing
  Escape does at a keyboard. Verified on the emulator: with the sheet open the
  press closed it and the process stayed alive.
- **A signed release, and the offline claim made checkable.** The release bundle
  and APK are signed with an upload key kept out of the repository, and the
  release manifest no longer declares `INTERNET` at all — `aapt2 dump
  permissions` shows `RECORD_AUDIO` and nothing else, so "it works offline" is a
  property of the artifact rather than a promise. The debug build keeps the
  permission for its development server, in a debug-only manifest. Verified by
  installing the release APK on a device, drawing on the board and grading, which
  is also what proves that minification left the reflective Kotlin plugin bridge
  intact.

**What is left.**

- **The physical device — running, confirmed by hand.** A debug device build
  signs with the team (`X5DWXB4283`, from the environment), exports an IPA, and
  installs onto the connected iPhone with `devicectl`. Getting it to *run* took
  two steps, both recorded in `HANDOVER.md` §6: iOS 26 and later kill an app that
  has not adopted the scene life cycle (a black flash and a crash report, and
  nothing in the app's own log), and the obvious repair — declaring
  `UIApplicationSceneManifest` with `UIApplicationSupportsMultipleScenes` false —
  turns the crash into a **black screen**, because `tao` only enters scene mode
  when that key is true. With it true the app renders, on the simulator and on the
  phone, and the device run is confirmed by hand: it opens, the board is there,
  and strokes can be drawn. That is M9's device criterion met for iOS; the release
  build and pronunciation are what remain.
- **iOS release builds.** `ios build` in release fails to link Tauri's Swift glue
  because the release Swift product keeps those symbols local; debug links. This
  is a toolchain/Tauri-version question rather than a change here, and it is
  recorded with the evidence in `HANDOVER.md` §6. Until it is resolved, a device
  build is a debug build, which is fine for practice and not for distribution.
- **Pronunciation — done.** `speech.rs` now has an iOS backend that speaks
  through `AVSpeechSynthesizer` in process, so the "Hear it" control is live
  rather than disabled. It shares the voice-selection rule with the macOS
  backend, so mainland Mandarin is preferred on both — and on iOS that turns out
  to be the same voice name, `Tingting`. Verified on the simulator (65 voices
  installed, three Chinese, `Tingting (zh-CN)` chosen; speak, stop a live
  synthesizer, and speak again all logged as succeeding) and confirmed by ear:
  the character 的 was heard. The same build was then silent on the phone, with
  the Ring/Silent switch on: the simulator has no such switch, so it could not
  show that the default audio-session category is muted by it. Pronunciation now
  takes the session as `playback` + `spokenAudio` + `duckOthers` for the duration
  of an utterance and gives it back when the synthesizer says the utterance has
  finished, so an explicit tap is audible either way and ducked music returns.
  The AVFoundation constraints are recorded in `HANDOVER.md` §6 — the objects are
  not `Send`, so the work goes to the main thread, and the delegate that ends the
  session is why the synthesizer and its delegate are kept together.
- **The Play submission itself.** A signed release bundle is built
  (`app-universal-release.aab`), signed with an upload key that lives outside the
  repository, and the same code has been installed and used on a device as a
  release APK — the course loads, a stroke grades, and pronunciation is live. What
  remains is the paperwork: publishing `docs/privacy-policy.md` at a public URL
  (Play requires one because of the microphone), pasting the copy from
  `store/listing.md`, and answering the Data safety and content-rating
  questionnaires whose answers are written out there. Two things still need a
  person rather than a program: hearing the pronunciation, and a stylus run on
  the board to check palm rejection.
**Approach.**

- The Tauri config and the `lib` target with
  `#[cfg_attr(mobile, tauri::mobile_entry_point)]` are already in place, and the
  Rust core has no platform dependencies — this was designed for. That held up:
  neither mobile port needed a `cfg` in `hanzi-core` or `hanzi-store`.
- **Larger touch targets and a phone layout came with the iOS half** and needed
  nothing for Android — the same `max-width: 760px` layout serves both. The one
  thing it did need was a way to measure the system bars (see
  `android_insets`): `env(safe-area-inset-*)` is the display cutout on Android,
  not the status bar, so the layout was right on a notched phone and wrong on
  every device without a notch.
- **What is left is packaging, not code.** A release build needs a signing key
  and a Play Store listing; the app itself runs.
- **App size, measured.** The debug APK is 76 MB, of which a stripped
  `libhanzi_tutor_lib.so` is 36 MB and the bundled interface font 17.7 MB. The
  unstripped library is 203 MB, which is why Gradle is told to strip it: without
  the `ndkVersion` that lets Gradle find `llvm-strip`, it cannot, and the APK
  becomes 210 MB and will not fit an emulator's data partition.

**Acceptance criteria.**

- Runs on iOS Simulator and one physical iOS device with working drawing.
- Runs on an Android emulator and one physical Android device with working
  drawing, and leaves the app when there is nothing left for back to close.
- Text and controls are legible and reachable at phone sizes, clear of the
  status bar on a device with no display cutout as well as one with.
- Pronunciation works on iOS through `AVSpeechSynthesizer` and on Android
  through the system `TextToSpeech`, both without the `say` binary, and both
  preferring a voice that needs no network.

---

## M10 — Durable study store (SQLite)

**Status: done.**

Study data was three pretty-printed JSON documents, each rewritten *whole* on
every change. That was comfortable at today's sizes and is not what this
milestone was about; it was about a ceiling on the thing the project most wants.
The schedule kept only the newest 20 attempts per character, and an unbounded
attempt log in a whole-document format is quadratic — after N attempts the file is
O(N) and every attempt rewrites all of it. It is now one SQLite database,
`hanzi.db`, holding every attempt ever recorded.

What shipped:

- **One database, not three.** `crates/hanzi-store` opens `hanzi.db` and the three
  stores share it: `progress_card` (what the scheduler needs), `attempt` (the
  log), `vocab_entry` and `vocab_group`, `course_cursor`, and `meta` for the
  schema version and the import markers. Progress and the list are read together
  on every review-queue build, so two files would have meant two connections and a
  cross-file consistency problem for nothing. The `-wal` and `-shm` files beside
  it are SQLite's, and write-ahead logging is what makes an interrupted save
  survivable — a test opens a write transaction, drops it uncommitted, and
  requires that the committed data is intact and the uncommitted rows are gone.
- **The log is the log.** `ProgressSink::save` is handed the cards that changed
  *and* every attempt recorded since the last save, so the log is not limited by
  the twenty attempts a card keeps for display. The test records 25 attempts on
  one character: the card shows the newest 20, `card.attempts` says 25, and the
  `attempt` table holds all 25 in order. `Db::attempts` and `Db::attempt_count`
  read it back, which is what the analysis in the cross-cutting "attempt logging"
  item will want.
- **The engine does not know.** `ProgressSink`, `VocabSink` and `CursorSink` in
  `hanzi-core` are the seam; `hanzi-store` is the only thing that knows SQL. The
  engine's tests are **unchanged** — they open a JSON file or nothing at all — and
  `selfcheck` is byte-identical, because nothing in the grading path moved.
- **One import, per document, never destructive.** The first time each store is
  opened, if its marker is not in `meta`, the corresponding JSON document is read
  with *the engine's own reader* (so the import cannot disagree with the app about
  what "corrupt" means), copied into the tables, and the marker recorded — in one
  transaction, so a crash cannot half-import. The files are left **byte for byte**
  as they were. Because the markers are per document, a corrupt
  `course-cursor.json` blocks the cursor and nothing else, which is the isolation
  the three separate files used to provide; that document is simply not marked, so
  fixing it and restarting imports it.
- **A fresh install writes no JSON at all**, and the only file it creates is the
  database. `--user-dir` and `HANZI_TUTOR_DATA_DIR` keep working and now select
  the directory that holds `hanzi.db`, verified on the built binary: it created
  the database in the chosen directory, imported all three legacy documents,
  recorded the markers, and left the JSON untouched.
- **The app's first third-party *code*** is now compiled in, so SQLite and
  `rusqlite` joined the notice catalogue — twelve notices, each still pinned
  three ways by `tests/licences.rs`.

Deliberately not done, and why:

- **No export of the log yet.** The ceiling is gone and the rows are readable
  (`Db::attempts`), but nothing writes them out and `selfcheck` still tunes
  against synthetic jitter. That is the second half of the cross-cutting
  "attempt logging" item, and it is now a feature to build rather than a format to
  change first — which was the point of doing this milestone with it in mind.
- **Achievements are still not a thing.** If they are added they must be derived
  from the log: a count stored separately from the attempts it counts is the
  second source of truth this project has twice refused.
- **The old JSON is never rewritten or removed.** It is the only copy of the data
  until the database has it, and leaving it means an older build still opens the
  study data. It therefore goes stale the moment the new build runs, and that is
  intended; the vocabulary list's own JSON export is the user-facing escape hatch.
- **No `rusqlite` in `hanzi-core`**, as planned: the engine keeps no native
  dependency, and the store crate is the only place SQL appears.

## Cross-cutting polish

Small, independently shippable, roughly in value order:

- **Search and browse characters** — by character, pinyin or meaning. Needed to
  find anything outside the linear course.
- **Import / export the vocabulary list** — CSV and JSON, so the list is not
  trapped in one machine's app data.
- **Keyboard shortcut help** — a `?` overlay. The shortcuts exist and nothing in
  the app now documents them: the "How this works" disclosure that listed them was
  removed with the rest of the board's chrome, so this is the only route left.
- **An onboarding wizard** — where the board's explanation went. The "How this
  works" disclosure and the sentence under the controls were removed to give a
  phone back the height they took, on the understanding that what they taught —
  the four grading measures, and that a faint copy is there to trace — belongs in
  something read once at the start rather than in a row that is on screen for ever.
  Until it is built, the measures are named on the report itself and described in
  full in [`README.md`](README.md), and the only sentence left on the board's
  screen is the one that says why a disabled control is disabled.
- **Dark mode** — the canvas is deliberately paper-white, so this needs thought
  rather than a colour swap.
- **UI scale / accessibility** — larger text and a bigger board; the board already
  sizes itself from its container.
- **Tone practice** — visualise the tone contour, and practise tone pairs, which
  is where most learners actually struggle.
- **Component / radical teaching** — the decomposition and radical fields are
  already in the dataset but only displayed, not taught. Radical-based groupings
  would help retention.
- **Beginner stroke hints** — mark each stroke's start point and direction on the
  guide for the first attempts.
- **Attempt logging for tuning** — record real attempts (locally, opt-in) so
  `selfcheck`'s tolerance analysis can be re-run against real handwriting instead
  of synthetic jitter. This is how the shape tolerance should eventually be set,
  and it is now **half built**: [M10](#m10--durable-study-store-sqlite) removed
  the ceiling, so every attempt is recorded in `hanzi.db`'s `attempt` table and
  readable through `Db::attempts`. What is missing is the rest — getting those
  attempts out (`hanzi-store` has no export), turning them into the shape/ink/order
  distributions `selfcheck` prints today for synthetic jitter, and the consent and
  privacy wording that "opt-in" implies. Do that next, against real handwriting,
  rather than growing the schema again.
- **Interface localisation** — the app teaches Chinese but speaks English.
- **A settings screen** — **done**, and it holds four preferences rather than the
  one that was on the board's control row: how a stroke is committed
  (*Automatic* / click to draw / drag), the stroke-order animation's pace, the
  board size, and the pronunciation voice. `src/lib/SettingsPanel.svelte` is the
  dialog, the fourth sidebar screen. What the roadmap asked to settle when it was
  built is settled: an **Automatic** choice is offered for the two preferences
  whose absence has a device or system answer (click-to-draw, the voice) and the
  stored value stays the tri-state it already was, with clearing spelled per
  preference on the wire — `click_to_draw` has its own command, because a missing
  argument and a `null` one are indistinguishable once they are on the wire,
  while a voice clears with `""`. A save failure **is** reported, in
  `SettingsView.warning`, on the screen itself and with the change standing for
  the session. The two preferences with no device signal
  (`animation_pace`, `board_size`) are plain values rather than tri-states, and a
  value at its default is stored as *no row* so that a fresh install is still an
  empty table. Still open, only because nothing needs them yet: a colour scheme
  (which wants the dark-mode thought below) and the speech rate.
- **A settings screen for the model download** — M12 needs somewhere to say what
  would be downloaded, from where, at what size and under which licence, with
  declining as a first-class state. The settings screen now exists; what it does
  not yet have is that section, and it should not grow one until M12 is built.

## Known weak spots

Recorded honestly, because they bound how much the current scores mean:

- **Ink width cannot vary yet** (M4). The measure is in place and proven, but the
  canvas paints every stroke at one fixed width, so a learner cannot put down
  *less* ink than that and the `faint` verdict cannot fire from a trackpad. What
  the measure catches today is overshoot and short strokes; a stylus that reports
  real width is what makes the width half of it live. **M8 deliberately did not
  add the velocity-thickened brush it had listed as cosmetic**: ink amount is a
  quarter of the score, so stroke width is now a grading input, and a brush that
  thinned with speed would penalise drawing quickly. See M8. Do not mistake "the
  measure works" for "the case happens".
- **The ink weight is a first guess.** Shape, placement, ink and order each carry
  a quarter of the headline score, chosen for symmetry and to stop a third-inked
  character reading "Excellent" rather than from any data. Like the shape
  tolerance, it wants real attempts to tune against — the same attempt log the
  cross-cutting item asks for.
- **An isolated character has no context, so a polyphonic one may be read wrong.**
  着 on its own is read whichever way the synthesiser prefers. M3 fixed this for
  words — the word carries the reading, and 着急 is `zháojí` — but a single
  character met in the course still has nothing to disambiguate it. The course's
  own pinyin list has the same limit: it shows every reading, most common first,
  and does not know which one the text means.
- **The word dictionary's reading can be the wrong one for a minority of words.**
  Where CC-CEDICT lists several readings for one headword, the reading chosen is
  the first, except that a capitalised proper-noun reading is passed over when an
  ordinary one exists — which is what stops 安 being taught as the surname `Ān`.
  That leaves roughly 1% of words, mostly single-character-sized ambiguities such
  as 便宜 (`biànyí` "convenient" rather than `piányi` "cheap"), with the less
  common reading. The dictionary cannot tell which sense a learner wants without
  the sentence; M3's search shows the definitions, so the mismatch is visible
  rather than silent.
- **The word rank is derived, not published.** It is the rarest character's
  frequency rank, so it orders the list sensibly but is not a corpus frequency
  and should not be presented as one (the UI does not show it).
- **Shape tolerance is tuned on synthetic Gaussian jitter**, not real learners. It
  is deliberately loose to suit a trackpad; that leniency may well be wrong. The
  jitter is also blind to anything that depends on *how the pointer sampled* a
  stroke rather than where it went, which is how a placement metric built on the
  sample mean survived tuning while marking 18,763 strokes wrong under realistic
  input. `selfcheck` now carries a density-perturbed pass as well as a noisy one;
  the remaining gap is real handwriting rather than either synthetic case.
- **CI checks the code, not the bundle.** The workflow runs `pnpm test`,
  `check:rust` and `check:web` on a macOS runner; it does not run
  `pnpm run build`, so a packaging regression — a resource path, a signing
  identity, a missing notice *file* — is still caught by hand. Building on every
  push costs a release compile plus a DMG, and the notices themselves are pinned
  by tests that do run, so this is a deliberate trade rather than an oversight.
- **The bundle is signed but not notarised**, so the first launch on a Mac that
  has not seen the build needs a right-click-Open. See M5.
- **The attempt log has no way out of the app yet.** It is recorded, unbounded
  and readable from Rust (`Db::attempts`), but there is no export and no screen
  that shows it, so its value is still latent — see the cross-cutting "attempt
  logging" item. A migrated card also keeps a pre-import `attempts` *count* larger
  than the rows in the log (the JSON it came from held only the newest 20), so
  anything that reads the log as a complete history must not do that.
- **The store is one file, so its failure modes are shared.** Losing `hanzi.db` —
  or corrupting it — costs the schedule, the list and the cursor at once, where
  three files used to fail independently. The isolation that replaces it is at the
  *import* rather than at steady state, and the app refuses to write rather than
  starting empty, so the failure is loud; but there is no backup, no export of the
  schedule, and no "open the folder" affordance. A corrupt database is a support
  question with no good answer yet.
- **Rust crate licences are not individually catalogued.** The notice catalogue
  covers the bundled *data*, the interface font and SQLite; the several hundred
  Rust crates in the dependency graph — Tauri's own, and now the `objc2`
  bindings for AVFAudio — are MIT/Apache-2.0 and are not listed one by one. If
  the app is ever distributed widely, that is a sweep worth doing rather than a
  gap to discover during one; it is recorded here because this milestone added
  to that set.
- **Pronunciation is macOS and iOS only.**
- **The app's "no network" claim is now conditional, and one place still has to
  keep saying so.** M12 added a download, so the promise is "nothing unless you ask
  it to". The README, `LICENSES.md` and the bundle's own description were restated
  when it shipped; anything else that repeats the flat version is now wrong and
  should be corrected rather than left to be discovered. In particular, M6's
  pre-rendered audio pack (§4.4 of the research) would keep its advantage — it
  needs no download at all — and that remains the cheapest way to finish the
  output side.
- **No CI builds the bundle** (see M5), and **the App Sandbox has never been
  tested — where the speech backend probably does not survive it.** A Mac App
  Store build must be sandboxed, and `src/speech.rs` pronounces by spawning
  `/usr/bin/say`, which is exactly the kind of thing a sandbox restricts.
  `scripts/probe-app-sandbox.sh` exists to settle it: it builds two minimal
  applications, one signed with `com.apple.security.app-sandbox` and one without,
  and packs the findings into the exit status because that is the only channel out
  of a launchd-launched app. **It could not answer the question here**, and says so
  rather than guessing: applying any sandbox profile is refused in this
  development environment — `sandbox-exec -p '(version 1)(allow default)' …`
  reports `sandbox_apply: Operation not permitted` — and the entitled application
  ran with its entitlement present but unenforced, writing to the real home, which
  the App Sandbox forbids. Settle it from a normal login session, or better where
  the sandbox is certainly enforced: put an App Store build on TestFlight and try
  *hear it* in the sandboxed build. If `say` is refused, the macOS backend needs an
  in-process synthesiser (`AVSpeechSynthesizer`) — the same conclusion M9 reaches
  for iOS, so the two should then be designed together rather than twice. There is
  also a third answer that removes the question: the **pre-rendered audio pack**
  proposed in [`docs/research/ASR_TTS_CLAUDE_RESEARCH.md`](docs/research/ASR_TTS_CLAUDE_RESEARCH.md)
  §4.4 takes the external process off the runtime path entirely for the bundled
  curriculum, which is the only one of the three that is *known* to be
  sandbox-safe. That research document is worth reading before M6 in any case —
  its §8 concludes the pack plus system TTS for user-entered text beats embedding
  a model, on every axis but one.
- **The course is frequency-ordered only.** It starts at 的 (8 strokes), which is
  right for reading but a demanding first character to *write*. A hand-ordered or
  stroke-count-ascending mode may suit a beginner better.
- **The stroke-order animation has no automated test.** Drawing cannot be driven
  from here (HANDOVER §6), and the frontend has no test runner at all, so the
  sweep was checked by capturing the app's own window rather than by asserting
  anything: `pnpm test` and `check:web` cover it only as far as it type-checks.
  The pure geometry behind it — `prefixAt`, `sampleAlong`, `strokeRadii` in
  `render.ts` — is the obvious first thing a `vitest` run should pin, since it is
  arithmetic over arrays and needs no canvas.
- **The sweep uses one band radius per stroke**, taken from its widest point, so a
  strongly tapered stroke starts revealing a little ahead of the pen at its thin
  end. The alternative is a radius per centre-line segment; it is only visible on
  a slow capture, so it waits for a reason to exist.
- **One dependency advisory is accepted, not fixed.** Dependabot reports a
  moderate advisory against `glib` 0.18.5 — unsoundness in the `Iterator` and
  `DoubleEndedIterator` impls for `glib::VariantStrIter` — fixed in `glib` 0.20.0.
  It arrives through Tauri's Linux GTK stack (`tauri → muda`/`tao → gtk 0.18.2 →
  glib`) and appears **nowhere** in the macOS dependency graph, so it is not
  compiled into the app as built. It also cannot be resolved here: `gtk 0.18`
  requires `glib ^0.18`, so cargo refuses 0.20.0 outright. Revisit when Tauri
  moves to gtk-rs 0.20, or before shipping a Linux build — whichever comes first.
  Do not re-investigate it from scratch; confirm the scope with
  `cargo tree --target aarch64-apple-darwin -e normal | grep glib`, which returns
  nothing.

---

## M11 — Tone practice (speech recognition, model-free)

**Status: done, for characters and words. Recognising the *text* is M12.**

**Why.** Stroke grading cannot see the most common beginner error in Mandarin,
which is the tone. A learner can write 妈 perfectly and say it as `má`. Tone is an
F0 contour, and a contour is not something a recogniser's text output contains —
so the useful half of "speech recognition for a language tutor" needs no model at
all. That is the argument in
[`docs/research/ASR_TTS_CLAUDE_RESEARCH.md`](docs/research/ASR_TTS_CLAUDE_RESEARCH.md)
§6, and this milestone is that half.

The app's promise is that it downloads nothing and never touches the network, and
this milestone keeps it — and M12, which later narrowed that promise for an
optional model, left this half untouched. There are no model weights here, no inference runtime and
no new build-time binary fetch. The one new dependency is `cpal`, to open the
microphone.

**What shipped.**

- `crates/hanzi-core/src/tone.rs` — pitch tracking and tone scoring, pure and
  dependency-free. YIN for F0 (de Cheveigné & Kawahara, 2002), written here rather
  than taken as a crate so that the one piece of signal processing in the project
  is owned and testable; a log-F0 contour over the voiced span; dynamic time
  warping against the four canonical tone shapes of the five-level scale.
- **The neutral tone is scored too**, having been refused at first on the grounds
  that it is short and takes its pitch from the syllable before it — both true,
  and neither judgeable from one syllable. What *is* judgeable is that it is
  level, which is enough to tell 的 from 得 said as a full tone, and refusing it
  left the most common character in the language as the one the panel would not
  look at. What the judgement cannot see is said to the learner every time
  (`tone::NEUTRAL_LIMIT`). The refusal itself turned out to be in the wrong
  module: `pinyin.rs` was deciding what could be *judged*, which it knows nothing
  about.
- `tone_from_pinyin` in the same module reads the tone out of the pinyin the
  dataset **already carries**, which is why no grapheme-to-phoneme work was
  needed at all — the asymmetry §2 of the research says to exploit.
- `crates/hanzi-core/src/pinyin.rs` — splitting a **word's** reading into
  syllables, and tone sandhi. The dataset stores a word's reading run together
  (`学习` → `"xuéxí"`), so scoring a word needs it taken apart; the splitter is a
  rule rather than a 400-entry syllable table, and the caller checks the result
  against the character count, so a reading it gets wrong is refused rather than
  mis-aligned against the recording.
- **Tone sandhi**, which is what makes words correct rather than merely possible.
  The dictionary reads 你好 as tone 3 + tone 3 and nobody says that: it is spoken
  2 + 3. Scoring against the dictionary tones would flag correct speech as wrong,
  so the three rules that matter are applied — third-before-third, 不 before a
  fourth tone, and 一 before anything else — and both readings are reported, so a
  learner seeming "tone 2" is told why it is not what their dictionary prints.
- **Syllable segmentation**, so one recording can be scored as a whole word. The
  boundaries come from a shortest-path search over the frame costs in which an
  *unvoiced* frame is free: the consonant between two syllables is exactly where a
  listener hears the break. The shortest segment allowed is the least voice a
  syllable needs to be judged, and when the span cannot hold the syllables asked
  for, nothing is scored rather than something being scored wrongly.
- `src-tauri/src/capture.rs` — `cpal` capture, downmixed to mono, opened on
  button-press and dropped on release. Short, because the interesting part is the
  analyser.
- Four commands. `speech_target` and `listen_stop` both take the **text** being
  practised rather than a single character, so a word is scored as a word and the
  tones are derived in Rust rather than sent from the interface. `speech_target`
  also reports whether a recognition model is installed, which is the other half
  of whether the microphone has anything to answer with.
- `src/lib/TonePanel.svelte` — the learner's contour drawn over the expected
  shape, which is the part that teaches. The wording of the judgement comes from
  Rust; the panel only styles it.
- A push-to-talk control on the practice screen, disabled with a reason only when
  there is no microphone, or when there is neither a tone to score nor a
  recognition model to read the text back.

**What is not in this milestone.**

- **Recognising text.** "You said `sì` where `shì` was wanted" needs a Mandarin
  ASR model. That is M12, and it is where the 155 MB download belongs.
- **Tone-scoring longer than a word.** Up to four syllables. A sentence's
  syllables run together with no consonant to cut at, so the boundaries cannot be
  found from energy alone; the pitch is not judged rather than divided into four
  pieces and scored as though the pieces were words. **Recognition still runs for
  that text** — the syllables the learner was asked for are known whether or not
  their tones can be judged — which is the follow-up noted under M12.
- **The neutral tone.** Short, and pitched by the syllable before it, so it is
  carried in the target and reported but not scored. A word containing one is
  still judged on its other syllables, which is why 妈妈 works.
- **Tone sandhi beyond the three rules.** The half-third-tone realisation in
  running speech is not modelled, and neither is the optional sandhi of 一 in
  very casual speech.

**Known limits, stated rather than hidden.**

- **Segmentation is a heuristic, and it is the weak point.** Energy and voicing
  gaps find the boundary for `xuéxí` and `nǐhǎo` — the consonant is unvoiced and
  unmistakable — but two syllables that run together have no such gap, and the
  search then cuts at the quietest frame, which may be the wrong one. It is tested
  against synthesised words whose boundary is known by construction, and **not yet
  against real multi-syllable speech**. `boundariesMs` is reported to the
  interface for exactly this reason: a learner told the wrong syllable was wrong
  needs to be able to see where the app thought the split was.

- **Amplitude is not scored.** The comparison is normalised to unit RMS, so a
  tone 4 that falls one semitone scores as well as one that falls eight. That is
  deliberate — the alternative flagged correct speech as wrong — and the panel
  shows the measured movement in semitones so the learner can see it. A separate
  "too shallow" band is the obvious refinement once there are real recordings to
  calibrate against.
- **Tone 1 and a flat tone 3 cannot be told apart** from one syllable, because the
  speaker's register is not known. A flat contour is accepted for both, and the
  panel says so in words. Under-claiming is the right failure here.
- **A denied microphone permission looks like silence.** macOS hands out a device
  either way, so `microphone_status` cannot report it, and the attempt comes back
  as "I could not hear enough voice to judge" rather than as a wrong tone.
- **The score constants are calibrated against synthetic contours**, not real
  voices — `SCORE_DECAY_ST`, `FLAT_ST` and `DECIDE_MARGIN` in `tone.rs`. They are
  named, documented and in one place precisely so real recordings can move them.

**Acceptance criteria.**

- [x] A synthetic syllable of each tone is scored against its own tone and against
      the other three, with the verdict asserted — including silence, a click, and
      a shape that matches no tone at all.
- [x] A synthesised two- and three-syllable word is split at the known consonant
      between its syllables (asserted within 25 ms), each syllable is scored
      separately, and the summary names the syllable that was wrong.
- [x] Tone sandhi is applied and both readings are reported, checked across all
      three rules.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` is clean and the
      whole workspace suite passes.
- [x] The microphone opens, records at the device rate and stops, verified
      against real hardware (`cargo test -p hanzi-tutor --lib -- --ignored`).
- [x] The licence catalogue, the `licences/` directory and the bundle config agree
      about the new notice.
- [x] **A human says a syllable into the running app and gets a sensible score.**
      Confirmed by hand. This is the criterion a machine cannot check, and it is
      what the whole engine was waiting on.
- [x] **A human says a multi-syllable word and the split lands on the right
      syllables.** Confirmed by hand on 不对: both tones scored, the sandhi
      sentence explained the `2 + 4`, and the split came out mid-word.
      **It also found a display bug on the first try** — the boundary was reported
      from the start of the *recording* rather than the start of *speech*, so the
      panel read "Split at 1463 ms" beside "Voiced 308 ms". The split was right
      and the number was inexplicable, which is worse than a wrong number because
      it discredits a correct result. Both are now measured from the first voiced
      frame, and a test asserts every boundary falls inside the voiced span.
- [ ] The constants re-tuned against a handful of real recordings, from at least
      two voices.
---

## M12 — Speech recognition: text

**Status: done.** The optional-download decision below was taken and is now
implemented: `sherpa-onnx` runs SenseVoiceSmall, the model is installed by the
learner from the settings screen, and nothing changes until they ask.

**Why.** M11 judges *how* something was said. It cannot say *what* was said, and
there is no non-neural substitute for that — you cannot pre-render a learner's
voice. Recognising the syllable would let the app distinguish "you said the wrong
tone" from "you said the wrong word", which is the difference between a tone
exercise and a pronunciation exercise.

**Approach.** Per
[`docs/research/ASR_TTS_CLAUDE_RESEARCH.md`](docs/research/ASR_TTS_CLAUDE_RESEARCH.md)
§5 and §7, which is the research for exactly this:

- `sherpa-onnx` (Apache-2.0) with `sherpa-onnx-sense-voice-...-int8-2024-07-17`,
  ~155 MB download / 228 MB on disk. **Not** Whisper: `whisper-tiny` is about 67%
  CER on Mandarin and `whisper-base` about 51%, against SenseVoice's ~8%, and the
  small Whisper models are the ones a Rust developer finds first.
- Compare transcripts as **pinyin, not characters**, which collapses the
  homophones that make character comparison useless for a single syllable. The
  character→pinyin mapping is already in the dataset.
- `sherpa-onnx-sys` downloads a prebuilt native library during `cargo build`
  unless `SHERPA_ONNX_LIB_DIR` points at a vendored copy. For a repository whose
  data comes from a reviewed fetch script and whose notices are pinned three ways,
  pulling an unpinned binary during compilation is a real change in posture:
  vendor and pin it, or the build is not reproducible.

**What shipped.**

- `scripts/fetch-sherpa.sh` — unpacks the native library against a **pinned
  SHA-256** into the gitignored `.sherpa-onnx/`, and **refuses** a platform whose
  digest has not been recorded rather than downloading something unverified.
  `scripts/with-cargo-env.sh` points `SHERPA_ONNX_LIB_DIR` at it, so `cargo build`
  never fetches a binary of its own. Static linking, so the app stays one file; a
  shared build would leave `.dylib`s that a bundled `.app` would have to declare
  as frameworks or fail to launch.
- `src-tauri/src/asr.rs` — the model manifest (address, measured sizes, digest,
  licence), a download that verifies as it streams and stages its work so an
  interrupted install leaves the previous state untouched, and the recogniser
  itself. It is the only module in the app that opens a socket.
- **The model is pinned by digest too, at run time.** Measured rather than
  estimated: 163,002,883 bytes compressed, 240,506,435 bytes unpacked, exactly the
  numbers the settings screen shows before anybody agrees to the download.
- `crates/hanzi-core/src/pinyin.rs` gained `base` and `heard_against`: the
  comparison rules — readings rather than characters, tone stripped from both sides
  — plus the wording, in the module that already owns what a reading is.
- **The comparison no longer needs a tone target.** `heard_against_readings` reads
  a transcription against the readings a learner was asked for, with or without
  the tones; `AppState::wanted_readings` resolves those readings for any text, not
  only a word. That is what lets a phrase longer than four syllables — the text
  tone practice deliberately refuses — still be spoken and read back. `listen_stop`
  no longer returns an error for it: the result carries `toneScored: false` and the
  transcription is the answer. `speech_target` reports the tone target and whether
  a model is installed together, which is what the microphone button gates on, so
  it is offered for a long phrase exactly when recognition can answer for it.
- A settings row that states the address, both sizes and the licence **before** the
  button, then shows real progress and a retryable failure. Polled rather than
  event-driven, because the app has no event channel and one command that answers
  "where has it got to" is less machinery than adding one for a single feature.
- The tone panel gained a second half: what was recognised, as plain pinyin, with
  each syllable marked against what was asked for — and nothing at all when no
  model is installed.

**The decision, and the constraints it implies.**

A 155 MB model cannot be bundled, so it must be downloaded on demand, which breaks
the README's flat "no model downloads and no network access at runtime".
**Decided: permitted, provided the download is optional, user-triggered, and
shipped through the settings screen.** That is a much narrower change than it
sounds, and the constraints are the real design work:

- **Nothing degrades when the model is absent.** Tone practice — the feature that
  exists today — must keep working with no model, no network and no prompt. The
  app must never ask for the download on its own.
- **The settings screen states what would be downloaded**, from where, at what
  size and under which licence, and that it is the only thing in the app that
  touches the network.
- **The download is verified** — a checksum, and a retryable failure path — and
  cached in the application data directory.
- **Declining is a first-class state**, not a nag. It must be possible to use the
  app for years and never see this.
- **The README's promise is restated rather than deleted**: the app downloads
  nothing *unless you ask it to*, and everything the bundled curriculum teaches
  continues to need nothing.

**Acceptance criteria.**

- [x] The README and `LICENSES.md` state exactly what is downloaded, from where, at
      what size and under which licence, and the model tarball's own `LICENSE` is read
      and recorded rather than assumed. **It was read, and it is not what the
      research assumed** — see the note below.
- [x] With the model absent the app behaves exactly as it does today: tone practice
      works, text recognition is unavailable, and nothing prompts unless asked.
      `asr::tests::a_fresh_install_has_no_model_and_says_so` asserts it.
- [x] Recognising a recorded syllable returns its pinyin, and a deliberate
      wrong-syllable recording comes back as a different syllable. Verified end to
      end on the model's own `test_wavs/zh.wav` — `开饭时间早上九点至下午五点` —
      and the homophone rule is asserted in `pinyin.rs`: 是 and 事 are the same
      syllable, 是 and 四 are not.
- [x] The hotwords/contextual-biasing API is **not** pointed at the expected answer.
      It biases decoding toward the target, which is the right tool for rare
      vocabulary and the wrong one for assessment. No hotwords file is ever set.
- [x] The whole install path — fetch, verify, unpack, recognise, remove — is
      exercised by `asr::tests::downloads_verifies_and_installs_the_model`, which
      is the only test that pins the digest against what GitHub actually serves.

**The licence is not what the research said, and this is the finding to carry
forward.**

The research's §9 licence table does not list SenseVoice at all: it recommended the
model in §5.3 and never checked its terms. The tarball's `LICENSE` is a one-line
pointer to FunASR, and FunASR separates its **MIT toolkit** from its **model
weights**, which are under the *FunASR Model Open Source License Agreement v1.1*
(Alibaba Group). That agreement permits use and redistribution but requires
attribution, states that the weights are provided "for reference and learning
purposes", is revisable by its publisher, and carries a conduct clause whose breach
terminates the licence. It is not a free licence in the sense the rest of
`LICENSES.md` uses.

That is compatible with what this milestone actually does — **the app points at the
model rather than redistributing it**, which is why the weights are not in
`licences/` and why the settings screen shows the terms before the download. It
would **not** be compatible with bundling the weights or shipping a pre-seeded
cache. Anyone who wants to do that must read the agreement and decide for
themselves. If a permissively-licensed Chinese model of comparable accuracy
appears, it should displace this one.

**Follow-up: a phrase with no tone target is recognised, not refused.**

By the end of M12 the two halves of an attempt were still joined at the wrong
place. Tone scoring stops at four syllables because the recording cannot be
divided any further, and the microphone button was disabled whenever there was no
target — so a learner's own six-character vocabulary, which is exactly the text a
personal word list collects, could not be recorded at all. The model was installed
and idle for it.

The halves are now independent. A target needs tones; a *comparison* needs only
the readings wanted for each character, and those are known for any text the
dataset can read. So `listen_stop` scores the pitch when it can and otherwise
returns the transcription alone, `toneScored` says which happened, and the panel
shows the tone half only when there is one. The button is offered when either half
can run — gated on `recognize`, because a long phrase on a device with no model
would record into an empty answer — and the reason it is not is said in the same
one expression as before.

What is still refused: the tone score. That refusal is the point of the cap, and
this change does not weaken it.

**Known limits, stated rather than hidden.**

- **The recogniser repairs the error being looked for.** Its language model is
  built to be robust to exactly the mistakes a learner makes, so it under-reports
  them — worst for the learners who most need telling. This is why the panel says
  which syllables were heard and never that pronunciation was good, and why no
  hotwords bias is set.
- **Syllable-level, not phone-level.** No forced alignment and no
  goodness-of-pronunciation score exists in this toolkit, so it cannot say which
  *sound* was wrong. The UI is worded to stay inside that.
- **Isolated syllables are the hard case**, not the easy one: they are unusual
  input for a recogniser. A correct syllable may occasionally be reported wrong,
  which is the safe direction to be wrong in — and the tone verdict is unaffected,
  since it never consults the transcript.
- **`use_itn` is off on purpose.** With inverse text normalisation on, 一 comes
  back as `1`, which cannot be read as a syllable and so cannot be compared.
- **Only the macOS arm64 native archive is pinned.** The script refuses other
  platforms rather than fetching them unverified; pinning one is a one-off download
  and a recorded digest, described in its header.
- **The static archive vendors more than it names.** ONNX Runtime (MIT) and the
  `kaldi-*` components (Apache-2.0) are linked and their notices ship. `espeak-ng`
  (GPL-3.0-or-later, speech synthesis only) is *absent from the macOS binary*,
  because the linker drops what nothing references — and that check was once
  written down as a claim about the app, which is false on Android and iOS: those
  builds link an already-linked `libsherpa-onnx` that has espeak-ng inside it, so
  the APK, the AAB and the iOS framework redistribute GPL code whether or not a
  line of it runs. Its notice and the GPL-3.0 text now ship with it. The lesson
  generalises: anything arriving inside a prebuilt mobile binary has to be checked
  *in that binary*, not in the desktop build. See LICENSES.md.
- **Android's release build now declares `INTERNET`, which reversed a deliberate
  property.** It used to be scoped to the debug source set so that `aapt2 dump
  permissions` on a signed APK showed only `RECORD_AUDIO`, making "works offline"
  checkable on the artifact rather than merely promised. A runtime download cannot
  work without the permission, so it moved into the main manifest, and the privacy
  policy and the Play listing were restated in the same change rather than left
  claiming there are no network requests. Nothing about what the app *collects*
  changed — it collects nothing, and it sends nothing to the developer — but "no
  INTERNET permission"
  is no longer available as evidence, and anyone re-using that argument should
  know it. **M13 owed the same restatement and was late with it**: syncing sends
  the learner's study data to a Dropbox account they connect themselves, and the
  privacy policy and the listing went on saying "nothing is transmitted" and "the
  only network request the app can make" until it was corrected — with the Data
  safety answers flagged for reconfirmation in the Play Console rather than changed
  silently.
- **The download was never exercised on Android or iOS.** It is verified end to
  end on macOS. The permission change above is exactly the class of thing that
  only shows up on a device, so a signed release APK should have *Download and
  install* pressed once before this is trusted there.
- **The linked size was not optimised.** The bundled executable went from about
  36 MB to about 62 MB — roughly 26 MB of added native code, its largest single
  increase. Trimming it — building ONNX Runtime without the execution providers
  this app never uses, for instance — is possible and was not attempted.


---

## M13 — Cross-device sync

**Status: done.** Two devices connected to the
same Dropbox account merge their schedules: what travels is the attempt log, each
device rebuilds its schedule from the whole of it, and the settings screen drives
connect, sync and disconnect. It also **syncs by itself** when the app starts and
when it comes back, which is the shape the feature was always meant to have. **Dropbox
is the first transport**; others can follow behind the same three-method trait without
the merge changing at all.

**What has shipped.** Schema 3, which is the part everything else stands on, and
which is useful on its own because it is what makes an attempt *nameable*:

- **`meta.device_id`** — a random UUID, generated on first open and stable for the
  life of the database. Random rather than derived from the machine: a hostname or
  a MAC address would be an identifier the learner cannot reset, and would collide
  the moment two devices were cloned from one image.
- **`attempt.device_id` and `attempt.seq`** — the pair that names one attempt on
  every device. The existing `id` could not do it: it is this file's rowid, so two
  devices both reach 1, 2, 3 and a merged log would collide on every row. `seq` is
  numbered per device, and `Db::attempts` exposes the pair as `origin()`.
- **A unique index on `(device_id, seq)`** — the merge's safety property, not
  tidiness. A sync that runs twice, or is retried after a failure it cannot tell
  happened, must not double a learner's history.
- **Backfill rather than renumber.** A log written before these columns existed
  enters the scheme with `seq = id`, which is already the order it happened in, and
  with this device's identity, because there was no other writer. Nothing is
  invented and no attempt moves.
- **`ATTEMPT_ORDER = (at, device_id, seq)`** for reading the log and for filling a
  card's recent history. This is a behaviour change, not a refactor: `id` is
  insertion order, and once a peer's attempts have been merged in, the row that
  arrived last is not the attempt that happened last. Since the schedule is folded
  in the order the log is read, reading by `id` would silently reorder a learner's
  reviews. `at` is only accurate to the second, so the tiebreak has to be
  something every device computes identically — SM-2's ease factor accumulates in
  `f32`, and folding the same log in two orders would leave two devices subtly
  disagreeing with no way to notice.
- Columns arrive through a new `schema::upgrade`, which asks each table what it
  already has and runs on every open, fresh install included. `SCHEMA` stays at the
  shape each table was first created with, so a fresh install and a five-year-old
  file take exactly the same path and no definition is kept in two places.

Five tests in `crates/hanzi-store/tests/store.rs`: the identity survives a restart
and differs between databases; an attempt carries its device and its number; a peer
sending an attempt twice is refused by the index; the log reads in the order things
happened rather than the order they arrived; and a schema-2 database gains the
columns with its attempts attributed and nothing renumbered.

**`crates/hanzi-sync`.** The format, the merge and the fold — the part that has to
be *right*, kept apart from the part that has to work, and tested against a
temporary directory with no account, no network and no credentials:

- **The shard format.** `devices/<device-id>/attempts/<first-seq:012>.jsonl`, one
  JSON object per line in sequence order, 500 to a shard. The device is in the path
  rather than in every line, because the path is what a store can list without
  fetching. A shard's name is the sequence it *starts* at, so a device cannot
  overwrite a peer's work even by accident and a retried sync writes the same bytes
  to the same names — there is no file-level conflict to have, which is what makes a
  dumb transport safe. Deliberately **no content hash in the name**: that would make
  the name depend on the contents, so a device that retried a partial write would
  create a second shard instead of finishing the first.
- **The merge is a union.** Attempts are identified by `(device_id, seq)` — the pair,
  never the number alone. An attempt both sides have is the *same* attempt, so there
  is nothing to resolve; and if two copies of one pair disagree, a shard was
  rewritten, which is the one thing the format promises cannot happen. That is
  reported as `SyncError::Rewritten` rather than quietly resolved, because a merge
  that picked a winner would be inventing history for a learner who cannot check it.
- **The fold recomputes, never merges, a schedule.** `hanzi_core::fold_attempts`
  rebuilds each card from its attempts in canonical order. This is the claim the
  whole design rests on, and it is asserted directly in `hanzi-core`: twenty mixed
  attempts recorded through the store, then folded from the log alone, must give a
  `CardState` equal in every field to the one the store built. To make that true by
  construction rather than by coincidence, the per-attempt update was pulled out of
  `ProgressStore::record_with` into one `apply_attempt` that both paths call — if
  they could drift, two devices would hold different schedules for one history and
  neither could tell which was right.
- **Canonical order is `(at, device_id, seq)`**, applied by both the merge and the
  fold. `at` is only accurate to the second, so ties are real and need a tiebreak
  every device computes identically: SM-2's ease factor accumulates in `f32`, and
  two devices folding one log in different orders would drift with no way to notice.
- **`RemoteStore`** is three methods — list, get, put — with a revision per entry so
  a transport can skip re-reading a shard that has not changed. `FolderStore` is the
  directory implementation, which is both the desktop "bring your own folder" story
  and what the merge is tested against. It refuses any name that would escape its
  root: a shard name arrives from a remote store, so it is not this program's text.
- **`Rating::name`/`from_name`** moved into the engine, which now owns the one
  spelling of a rating for the database, the JSON documents and the wire. A test
  asserts it against serde's own form, so a renamed variant cannot leave a database
  and an interface disagreeing about what a row means.

Sixteen tests across `crates/hanzi-core/src/progress.rs` and
`crates/hanzi-sync/tests/convergence.rs`. The headline one is convergence: a laptop
and a phone that practised 好 while apart, each folding the union of its own log and
the peer's shards, must produce *the same* card — and the merged result must not
depend on which log the merge was given first. Others cover a retried sync being a
no-op, the same sequence number on two devices staying two attempts, an `f32` score
surviving the JSON round trip exactly, a truncated shard being an error rather than
a silently shorter log, and a shard name not reaching outside the store.

**The local adapter**, which is where the database and the merge meet and the only
place they do:

- **`publish`** writes this device's attempts that no shard holds yet and moves a
  watermark kept in `meta` (`sync:published_seq`). The watermark advances only after
  the shards are written, so a failure re-publishes rather than skipping — which is
  safe, because the same attempts produce the same names and the same bytes.
- **It publishes `own_attempts`, never `attempts`.** After one sync the local log
  holds a peer's attempts too, so "everything in the log" and "my attempts" are
  different questions; publishing the first under this device's name would relabel
  the peer's work, and `(device_id, seq)` is the identity the whole merge rests on.
  The two queries have different names for that reason, and a test asserts the
  published shards are three attempts each rather than six of one.
- **`pull` merges in memory before writing.** The database's `DO NOTHING` cannot
  tell "already have it" from "have a different copy of it", so the in-memory merge
  runs first and a rewritten shard is reported rather than swallowed.
- **`recompute` rebuilds schedules, and folds the one case it cannot rebuild from a
  baseline.** A card whose count exceeds its rows in the log had history before the
  log existed — a card migrated from the pre-M10 JSON, which kept only its newest
  twenty attempts — and folding those rows alone would rebuild a schedule out of the
  tail of one. See "The baseline" below for what happens instead. A card with no
  baseline to fold from is left exactly as it was and reported, which is what this
  did *before* the baseline existed: the fallback is the conservative answer, never an
  invented one. It also skips a card the fold already agrees with, so a sync that
  changes nothing reports that it changed nothing and `Summary::is_empty` means what
  it says.
- **Every store a sync can rewrite is reloaded after it, in one method.** Progress,
  the vocabulary list and the course cursor. The vocabulary list is the one that
  bites rather than merely annoys: `save` reads an entry missing from the document as
  one the learner removed and tombstones it, so a save from a document predating a
  sync deletes everything the sync brought in, and those tombstones then travel.
  Progress was done first and the list was forgotten, which is why the set lives in
  `AppState::reload_after_sync` rather than in three calls at the command. And the
  backend reload is only half of it: **the interface keeps its own copies of the same
  views** and replaces them only when a command hands over a new one, so a sync also
  has to make the screen re-read — `onSynced` from the settings screen, which is
  `App.svelte`'s `refreshAfterSync`, covering vocabulary, progress and the review
  queue together. Without that half, a successful sync looks exactly like a sync that
  did nothing until the app restarts. The original note on the schedule's store:
  A `ProgressStore` holds the document in memory and writes through it, so an open
  store goes stale the moment a sync rewrites the `progress_card` rows — and its
  next `save`, which every review performs, would write the stale card back over the
  synced one. That is why `sync_now` takes the `AppState` as well as the sync
  service: it is not a forward, it is the reload. It reloads on failure too, since a
  sync that died partway through `recompute` may still have written some of them.
  The damage was always recoverable — the log kept every attempt and the next sync
  rebuilt from it — but it was a silently wrong due date until then, which is the
  kind of failure worth a command taking two pieces of state rather than one. A
  store already in its failed state is left alone: refusing to save is the point of
  that state, and reviving it because the database happened to read this time would
  undo a decision the reader has not made.

Five more tests in `crates/hanzi-sync/tests/two_devices.rs`, against two real
SQLite databases and one folder: they converge on one schedule; each device
publishes its own work and attributes a peer's to the peer; a new device that has
never seen a character learns its whole schedule; a card whose log is short of its
count is left alone and reported; and an unreloaded store can undo a sync and the
next sync heals it.

The suite is green at 344 tests, with 4 more ignored unless a microphone or the
speech model is present.

**The baseline, which is how a card whose log is too short still folds.** The one
thing the "the log syncs and the schedule is recomputed" design could not do is a card
whose log does not reach its first attempt. The pre-M10 JSON files kept only the newest
twenty attempts of a character, so a migrated card honestly remembers more than the log
holds, and folding the rows that survived would rebuild a schedule out of the *tail* of
a history. Three things make it foldable again.

- **The state is captured, because it cannot be derived.** What the missing attempts
  left behind is not recoverable from the two that survived — SM-2's ease accumulates,
  and nothing in the log records what it was before. So the device that owns such a card
  writes the state down once, in `meta` beside the publish watermark, and publishes it
  as its own `baseline.json`.
- **The capture is lazy, not an upgrade step.** It happens on the first sync after this
  existed, which is the first moment anybody needs it, and a database that migrated
  years ago gets one the same way a fresh one does. No schema change, and nothing to get
  wrong on the upgrade path. An empty capture is written down and *not* published, so
  the scan happens once and a peer is not handed a file to learn nothing from.
- **What it covers is exactly this device's own rows so far.** Every one of them is
  genuinely inside the captured state, because an attempt this device records goes
  through `apply_attempt` and is written back with its card in the same `save` — the two
  have always moved together. A *peer's* rows are deliberately outside it: those arrived
  through a pull, and for precisely the cards that need a baseline the recompute that
  follows a pull has always skipped them. Leaving them unfolded here would be the bug
  the baseline exists to fix.
- **The range is per character, and that is not tidiness.** A baseline's range is a
  stretch of one device's log, and all it claims is that *the cards it names* already
  hold those rows. The first version treated it as a global range and a test caught what
  that costs: a device with an incomplete history for one character and a complete one
  for another would have had the second character's attempts silently dropped, leaving
  its card at whatever it was. This is the kind of bug that produces two devices
  disagreeing with nothing to notice it by, which is why the test exists.
- **Two baselines for one character are settled by the one that remembers more**, with
  the device id breaking a tie. Two rival accounts of one history is a degenerate case —
  it means two JSON files had got out of step — and nothing here can tell which is
  right, so the rule is total and deterministic rather than clever. A rule that could
  tie would let two devices pick different winners out of the same shards.
- **The fallback is the old behaviour.** A card with no baseline to fold from is left
  exactly as it was and counted in `Summary::left_alone`, so a missing baseline degrades
  to a stale schedule and never to an invented one.

The acceptance criterion this was written for is that **a card whose `attempts` exceeds
its logged rows keeps its schedule through the baseline path**, and the test that
carries it goes further than that: it proves the *new* attempt a peer records afterwards
is folded on top of the baseline, because ignoring it is exactly what "left alone" used
to do and is the reason this was worth building. Two more tests cover the mixed device
above and the fallback, and three unit tests in `document.rs` cover the resolution
itself — the winner, the tie, the per-character scoping, and an empty baseline being
inert.

**The Dropbox client.** Hand-written over `ureq`, because `ureq`, `sha2` and
`base64` are all already in the build graph and Dropbox's own Rust SDK would be a
new upstream project for four endpoints:

- **Three HTTP shapes, one trait.** RPC (JSON in, JSON out), content (arguments in a
  `Dropbox-API-Arg` header, bytes as the body, either direction) and form (no
  authorization, for the token endpoint). `Http` is that shape rather than "an HTTP
  client", so everything built on it is tested against an in-memory Dropbox and only
  `UreqHttp` touches a socket.
- **PKCE, and no secret anywhere.** The app is open source and ships no server, which
  is Dropbox's own description of the case PKCE exists for. A test asserts the token
  request carries no `client_secret`.
- **No redirect URI at all.** Dropbox refuses custom schemes — every redirect must be
  HTTPS except `localhost` — and its code flow makes the parameter optional for
  exactly this case. So the authorization page shows a code and the learner pastes
  it in: one paste per device, once, in exchange for no server, no hosted domain and
  no per-platform URL registration. A test asserts no `redirect_uri` is sent.
- **The verifier is two UUIDs.** RFC 7636 wants 43–128 characters from a restricted
  alphabet; two version-4 UUIDs are 64 such characters and 244 bits of entropy, drawn
  from the same crate that generates the device id. No new dependency.
- **The refusal is read before the access token is required.** Dropbox reports a
  refusal as a 200 with `error_description` and no `access_token`, so a struct that
  demanded one would show the learner a missing-field parse error instead of
  Dropbox's own sentence. That was a bug until a test caught it.
- **`list` follows the cursor** through `files/list_folder/continue` until
  `has_more` is false, and reports Dropbox's `rev` as the entry revision.
- **`put` overwrites, deliberately.** A shard name is written once by this program, so
  a re-upload is identical bytes and overwriting is harmless — whereas a create-only
  upload breaks a retry after a timeout the device could not distinguish from a
  failure. Immutability is the writer's discipline, and what checks it is the merge,
  which refuses two shards that disagree about one attempt.
- **Unions go on the wire as objects.** `"mode": "overwrite"` looked right and is
  answered by the live API with a 400 whose summary is `other/...` — naming neither
  the field nor the reason. The schema types every `WriteMode` variant as an object
  with a required `.tag`, and Dropbox's own examples use the object form even for
  variants that carry no value (`{".tag": "home"}`), so `{"mode": {".tag":
  "overwrite"}}` is what it wants. A test pins the shape, and a 400 now reports the
  whole response body rather than only `error_summary`, because a 400 is a request
  this program built wrongly and its body is the only thing that says how.
- **`SyncError::Unauthorized` is its own failure.** It is the one error with an
  obvious next move — refresh and retry — and reporting it as a network fault would
  send somebody to check a connection that is working.
- **A missing scope does not arrive as a 401.** The error guide says a missing scope
  is a 401, and the first real sync proved otherwise: an app without
  `files.content.write` gets a **400** whose `error_summary` is `other/...` and whose
  `user_message` names the scope *and* says which App Console tab enables it. So a
  400 prefers `user_message` over the summary, because the summary named nothing and
  the sentence under it was the whole of the fix. Worth knowing when enabling a
  scope: Dropbox bakes scopes into the token at authorization time, so an existing
  connection has to be disconnected and reconnected before a new scope takes effect.
- **Drawing the screen never unlocks anything, and the sign-in asks for nothing by
  default.** The refresh token goes into the *data-protection* keychain with an access
  control that carries **no constraint at all** and the mode
  `AccessibleAfterFirstUnlockThisDeviceOnly`: encrypted at rest, released only to this
  app, not carried to the learner's other devices, and readable without asking anybody.
  A fingerprint is a switch — the same item created instead with a *user presence*
  constraint — and it is asked for when the token is about to be used, never to draw a
  screen. This reverses what was built first, and the reason is worth recording because
  the first version looked correct on paper: an item with a user-presence constraint
  asks on **every read** — there is no "always allow" for one built that way — so
  reading the token to answer "is this device connected?" prompted for a fingerprint
  merely for opening Settings, one sync could prompt three times, and the automatic
  sync at launch this milestone still owes would have prompted every launch, at a moment
  nobody chose. Two rules now hold it apart. **The token is read at most once per run of
  the app**, cached for the life of the process, and only when something is about to use
  it. **"Connected" is answered from a record that is not a secret** — Dropbox's
  `account_id` and the protection the item actually got, in the database's `meta` table
  beside the publish watermark — so `sync_status` costs no keychain access at all. A
  device that connected under the earlier build has a token and no record, so the one
  exception is deliberate and runs once: an absent record plus a sign-in in the store is
  adopted, and read from the store it was found in rather than guessed at.
  `SyncView.protection` still says which of the four states a device is in, and the
  fingerprint switch is offered only where the platform can honour it (`canLock`).
- **A store the system cannot identify falls back, and says so.** Access controls need
  an application identifier, so an ad-hoc signed development build cannot have one: the
  data-protection keychain refuses the item with `errSecMissingEntitlement` and it goes
  to the login keychain instead. That is the one case that can ask for the **keychain
  password**, because the system does not recognise a rebuilt binary as the one that
  wrote the item — which is why `SyncView.protection` distinguishes `deviceOnly` (asks
  nothing), `userPresence` (asks for a fingerprint), and `keychainOnly` (the login
  keychain, may ask for the password). Reading the keychain now falls back on
  `errSecMissingEntitlement` as well as `errSecItemNotFound`, because a build the system
  cannot identify is refused the keychain as a whole rather than told about one item in
  it; without that, a connection written by the fallback path would not be found again.
  Only the missing-entitlement refusal falls back. Any other failure is reported, because
  a second attempt would only fail more quietly.
- **Android keeps the sign-in in its keystore, which is a different shape of thing.**
  There is no keychain of the Apple kind there: the Android keystore holds *keys*, not
  secrets. So the Kotlin side generates an AES-256-GCM key inside it — never
  exportable, in secure hardware where there is any — and writes only the ciphertext,
  to the app's own preferences. The file on disk is useless without the device.
- **And asks for the learner, in a way that had to be arranged differently.** An
  Apple keychain item carries its own access control, so the prompt is a property of
  the *item*. Keystore has no such thing: `setUserAuthenticationRequired(true)` is a
  property of the **key**, fixed when the key is made and unchangeable afterwards. So
  the Kotlin side records the mode beside the blob and rebuilds the key when the
  learner flips the switch — deleting the blob with it, rather than leaving a token
  encrypted under a key nothing will use again. The prompt itself is a
  `BiometricPrompt` carrying the cipher as its `CryptoObject`, which is how the system
  ties one successful authentication to one keystore operation; `androidx.biometric` is
  a dependency this app now carries for it, and the only one it has ever added for a
  single call.
- **Writing never prompts, and reading only sometimes does.** Keystore permits
  *encryption* with an authentication-required key and refuses only decryption, so
  connecting asks for nothing — which is what the module note requires, since
  connecting happens on a button press and a prompt there would be a prompt to create
  the very thing the prompt protects. It is also why the constraint is compatible with
  a sync that runs by itself: the prompt is per *operation*, and the token is read once
  per run.
- **A device that cannot ask is not a device that cannot connect.** A phone with no
  screen lock cannot have such a key made at all, so the request degrades to a key that
  asks nothing, and the answer says which of the two the token actually got — the same
  honesty as `Protection`, and the reason `saveSecret` answers with the mode rather
  than resolving an empty object. `canLock` asks
  `BiometricManager.canAuthenticate(BIOMETRIC_WEAK or DEVICE_CREDENTIAL)` first, so the
  switch is offered only where a prompt would actually appear: a PIN-only phone counts
  as askable, because a key made this way can be satisfied by one.
- **An undecryptable blob is forgotten rather than reported.** That is what a restored
  backup looks like on Android: the preferences come back and the keystore key does
  not, because the key is bound to the device. It is also what a fingerprint enrolled
  *after* the key was made looks like, because `setInvalidatedByBiometricEnrollment`
  makes keystore invalidate the key rather than let the old one keep decrypting — which
  is the right direction to be wrong in for a secret. An undecryptable token is worth
  nothing to anybody, and failing would leave a learner staring at an error they cannot
  act on when the useful thing is to be told they are not connected and offered
  Connect.

Eleven tests in `crates/hanzi-sync/tests/dropbox.rs`, all against an in-memory
Dropbox that answers the same four request shapes. The one that matters most is the
last: the same shards written through the Dropbox client and through a directory
must read back identically, which is the claim that a transport is interchangeable.
`UreqHttp` itself is the one part not covered — it is the part with no decisions in
it.

The suite is green at 355 tests, with 4 more ignored unless a microphone or the
speech model is present.

**The app side.** `src-tauri/src/sync.rs`, plus six commands
(`sync_status`, `sync_connect`, `sync_connect_finish`, `sync_now`,
`sync_disconnect`, `sync_set_lock`):

- **The refresh token goes in the platform's secret store** — Apple's Keychain, via
  `security-framework`, which is the same API on macOS and iOS — and **not** in
  `hanzi.db`. That file is an ordinary file in an ordinary directory that a backup
  tool copies to a second disk and a cloud service; a database is the wrong place
  for a credential even when it is the right place for study data, and those are not
  the same claim. On a platform whose secret store is not wired up (Windows, Linux),
  connecting is **refused** rather than quietly written somewhere less safe: a refusal
  is a bug report, a plaintext credential is a vulnerability nobody notices.
- **What is *not* a credential does go in the database.** Whether a sign-in exists,
  whose it is, and how well it is protected is not worth protecting, and putting it in
  `meta` is what lets the screen be drawn without opening the keychain at all. That is
  the difference the fingerprint default turns on: the record is read for every
  `sync_status`, so a record that lived in the keychain would be a prompt for opening
  Settings.
- **The token store's answer is read once per process.** `load` is called when the
  token is about to be *used*, and the answer is cached in an `Open` enum whose third
  state is the point — `Mutex<Option<Account>>` could not tell "not read yet" from
  "read, and there is nothing there", so a learner who has never connected would have
  been sent to the keychain every time the screen was drawn.
- **`save` returns what the item actually got.** A store that could not honour a
  request for a fingerprint says so rather than letting the screen claim a prompt that
  will never appear, which is what the fourth `Protection` state is for.
- **The store and the HTTP client are trait objects with production defaults**,
  which is not ceremony — it is what lets the tests drive a whole
  connect-then-sync-then-disconnect pass against a temporary directory and an
  in-memory token store. A test that wrote to the developer's real Keychain would be
  a test that deleted their account. The in-memory store counts its reads, which is
  how the rule above is asserted: a read is the one moment a learner is asked for
  anything, so "the screen was drawn five times and nothing was read" is a test.
- **Disconnect revokes before it forgets.** Dropping the local token would leave the
  authorization standing on Dropbox's side, which is not what "disconnect" means to
  somebody who pressed it. A failure to revoke is reported and still clears locally.
- **Connecting is refused if the connection cannot be written down.** A refresh token
  the screen does not know about is one the learner cannot disconnect, so a record
  that cannot be written takes the token back out again rather than leaving it where
  nothing can reach it.
- **The app key is a constant, not a build secret.** It travels in the authorization
  URL, which is why PKCE exists; a fresh clone therefore builds something that
  works, with `HANZI_DROPBOX_APP_KEY` as the override for a fork with its own app.
- **The authorization page opens in the system browser** through
  `tauri-plugin-opener` — never a webview, which Dropbox asks against and Google's
  policy forbids for the accounts that sign in through them.
- **`Http` is `Send + Sync`**, because the app holds one inside Tauri's managed
  state. That was a compile error rather than a design note, and it is the reason
  the bound is written on the trait instead of on the one implementation.

Twenty-four tests in that module, and none of them touches the Keychain or a socket. The
one that carries the weight does the whole pass: connect against a fake token
endpoint, keep the refresh token, practise a character, sync over a real directory,
find nothing to do the second time, then disconnect and prove the token is gone. Five
of the others are about the friction this design exists to remove: the screen is drawn
five times without a single read; the token is read once and not again however many
times it is used; a connection survives a restart with a store that refuses to be read
at all; a sign-in stored before the record existed is adopted once and read from the
store it was found in; and both access controls the switch can ask for are ones the
system will actually build, which is the one platform call that cannot be covered
without a keychain.

The suite is green at 409 tests, with 4 more ignored unless a microphone or the
speech model is present.

**The settings screen.** A fifth row in the settings panel — the fourth was the
recognition model — with connect, the pasted code, Sync now, Disconnect, a switch for
the fingerprint prompt, and a line saying what the last sync did. Five things about it
are deliberate:

- **The authorization address is offered to copy, not to click.** A link in this
  webview would load Dropbox *inside* it, which is the one thing the whole flow
  exists to avoid; and reopening the address would mean starting a *new*
  authorization, which would invalidate the code on the page already open. So it is
  plain, selectable text.
- **The button is not offered where there is no secret store.** The screen asks
  `canConnect` first, so a learner on such a platform is told why rather than
  watching a button fail. A refusal is a bug report; a button that fails is a
  mystery.
- **The fingerprint switch is not offered where the platform cannot ask.** `canLock`
  is the same idea as `canConnect`, one capability further out: a switch that does
  nothing is worse than no switch, so on Android it is answered by asking
  `BiometricManager` whether a prompt would actually appear — and a phone with no
  screen lock answers no.
- **A mistyped code keeps what was typed.** The field is cleared only on success,
  because the page it came from has usually been closed by then and retyping a long
  code from a page that no longer exists is not recoverable.
- **It says what it is doing.** A sync at launch has nobody watching it who chose it,
  so the app puts a line on the screen while one is running and takes it away when it
  is done — see "Syncing that nobody asked for" below. Without that, a slow sync is
  indistinguishable from a hung app, which is the failure this feedback exists to
  prevent.

**Syncing that nobody asked for.** The settings screen starts a sync; so does the app
itself, when it opens and when it comes back. Everything below is about the second
one, which is the case where a design mistake is invisible until it happens to
somebody who did not ask for it.

- **Two triggers, one cooldown.** Launch is one. Coming back is the other, and it is
  two events because the platforms disagree: a phone hides the page, so it is
  `visibilitychange`, while a desktop window that is merely unfocused fires nothing —
  so `focus` is listened for as well. Focus fires on every alt-tab, and a sync per
  alt-tab is a request per alt-tab to somebody else's servers for data that has not
  changed, so a foreground attempt is refused within **a minute** of the last one. An
  in-flight guard sits in front of that, because the cooldown is about not asking too
  often and not about two at once.
- **Three refusals before anything is sent, in ascending order of cost.** *No
  account*: read from the non-secret record, so no keychain and no socket — this is
  the case for everybody who never connects Dropbox, which is most people, and it has
  to be free. *A sign-in that asks for a fingerprint*: a sync that runs on its own has
  nobody to satisfy that, so it does not run, and nothing is unlocked to find out.
  *No network*: a bounded probe, which is the next bullet. Only after all three does
  anything go over the wire.
- **The network is asked about before the request, not by the request failing.**
  `UreqHttp` allows ten seconds for a connect — deliberately, because the slow case it
  is written for is a bad mobile connection rather than a dead one — and a device with
  the radio off would spend all ten, per request, before concluding anything. At
  launch that is a stall with somebody watching it, so `crates/hanzi-sync/src/reach.rs`
  opens a TCP connection to the API host first: no TLS, no request, no data. Two
  details are the whole of it. The probe runs on a **thread** with a deadline, because
  name resolution has no timeout this code can set and a device attached to a network
  with no working DNS is exactly the launch-time hang this exists to prevent — so
  silence past the deadline means "no". And the host comes from `crate::dropbox`'s
  `API_HOST`, built into the RPC root rather than written twice, because a probe of a
  machine the client never talks to is worse than no probe at all. A `true` is not a
  promise (a captive portal answers TCP and then refuses the request); a `false` is:
  nothing was attempted and nothing would have worked.
- **`AutoSync` is four outcomes rather than a `Result`.** "Nothing happened" is the
  ordinary answer on most launches, and the screen has to tell the cases apart: not
  connected and locked are silent, offline is one calm line — a phone on a train is
  not an error — and only a failure is worth alarming anybody about, because an
  automatic sync that fails quietly is a device falling out of step with the others.
- **The commands that touch the network are `async`, and that is load-bearing.** A
  plain `#[tauri::command]` runs on the main thread — the thread the webview draws on
  — so a sync declared that way freezes the window for as long as it takes, and the
  line that is supposed to say it is syncing cannot be painted until the sync it
  describes is over. The `async` attribute on a **synchronous** function moves it to
  Tauri's runtime with its signature unchanged, which is what `State` needs. That
  makes two syncs possible at once, which is what the gate in `SyncService` is for:
  the merge is idempotent so nothing would be corrupted, but two passes interleaving
  their publish and pull would report nonsense. A refusal from the gate is not a
  failure — for the automatic path it is silence, and for a pressed *Sync now* it is a
  sentence saying another sync is running.
- **The feedback says nothing when there is nothing to say.** A line appears after
  400 ms — a sync that finishes sooner should not flash — and the working line is
  replaced by the result when there is one. "There is one" means something moved:
  attempts sent or received, schedules rebuilt, the list changed, the course position
  moved. A launch that found nothing to do is silent, because the working line
  disappearing is itself the answer to "did it sync?", and a sentence saying "already
  up to date" on every launch is noise nobody can switch off. `leftAlone` is left out
  of that judgement on purpose: it describes a schedule this device cannot rebuild,
  which is a standing condition rather than news.
- **The reload discipline, which matters most here because nobody chose the moment.**
  `sync_now` already reloaded the backend stores and told the screen to re-read; the
  automatic path does both too, and the second half is the one that is easy to miss. A
  sync at launch writes the right things into the database and, without the screen
  re-reading, goes on showing the old ones until the app is restarted — the exact bug
  this milestone has already had twice. So the automatic sync bumps a counter the
  settings screen watches, because that screen holds its own copy of the sync state and
  may be open while a foreground sync runs.
- **A sign-in left locked by an earlier build writes its own switch down.** This is
  the upgrade path, and it is the one case the design would otherwise get badly wrong.
  A device that connected before the fingerprint was a switch has a token behind a
  fingerprint and no preference saying so; left at that, every launch would read a
  locked item — a prompt at a moment nobody chose, for ever. So adopting a sign-in
  records what the keychain says about it, and the switch and the item agree from then
  on. It costs one prompt, once, on the first launch after the upgrade, which is the
  price of not asking the learner to paste a new authorization code on every device.
- **What decides is the *item*, not the switch.** A learner who asked for a fingerprint
  on a build that could not provide one has the switch on and an item that asks
  nothing; reading that is silent, so it is safe to sync. The reverse is the case that
  matters, and it is the one above.

Nine tests in `src-tauri/src/sync.rs` and four in `crates/hanzi-sync/src/reach.rs`
cover it. The four that carry the weight are a launch with nothing connected (nothing
sent anywhere), a launch that really syncs over the HTTP seam, a launch with no
network (not one request attempted, which is the claim the pre-check exists for), and
an overlap of two syncs (not reported as a failure). The probe's own tests are the
ordinary three — something listening, a port with nothing behind it, a name that does
not resolve — plus the one that pins the probed host to the host the client calls.

**Verified where it counts.** A Dropbox account of the author's own, and **three
devices at once** — a MacBook, an iPhone 13 Pro Max and an Android phone — each
connected separately and syncing against the same app folder. Every pair merged in
both directions, so the log on each ended up the union of all three; and a device
that had never seen a character picked up its schedule from another's log, which is
the whole design in one observation. The Android AES round trip is covered by that
too, since it only runs when somebody completes an authorization there.

The vocabulary list and the course position were confirmed the same way: a word
entered on the Android phone appeared on the other two, and the list and the sidebar
retired themselves **while the apps were still running**. That last part took two
attempts, because reloading the backend stores is not the same thing as the screen
re-reading them — a distinction recorded in `HANDOVER.md`, since it is the shape of a
mistake rather than a slip.

**The vocabulary list and the course position travel too.** These are the harder
half, and the reason is worth stating once: an attempt is *appended* and never
changes, so a merged log is a union with nothing to settle; an entry is *edited and
deleted*, so two devices can hold different versions of one row and something has to
say which is right.

- **A stamp of three parts: `(updated_at, device_id, revision)`.** The time is the
  ordinary answer. The device settles two devices writing inside one second. The
  revision settles two writes by *one* device inside one second — which is not
  hypothetical: adding an entry and deleting it again takes well under a second, and
  without the third part a peer holding the first of those writes has nothing to
  compare against, keeps the entry, and stays different from the other device for
  ever. That is a bug this work found in itself, by a test that failed.
- **A device's own view is published whole, every sync.** There is nothing to append
  for an entry that can be edited, so the shard is a document that gets rewritten.
  That gives up the immutability the attempt log has — and it is worth being exact
  about what that property was for: never the *merge*, always the *dumb transport*.
  The safety survives, because every writer still owns its own directory, Dropbox
  uploads are atomic, and the merge is a pure function of the records rather than of
  their history. What is genuinely lost is that a shard is no longer a permanent
  record of what a device once said; that is fine here and would not be for attempts,
  which is why the two are different files and different formats.
- **The stamp moves only when what the learner typed moves.** One stamp per entry, so
  if practising moved it, a learner practising on the phone could silently undo an
  edit made on the laptop. `attempts`, `best_score` and `last_practised` therefore stay
  this device's own, and a `uuid`-addressed row arriving from a peer starts its
  counters empty. **What syncs is what the learner wrote.**
- **A removal is a tombstone, not an erasure.** Erasing would be worse than losing the
  record: a peer still holding its own copy would put the entry straight back, since
  "I have no row for this" and "I have not heard about this yet" are indistinguishable
  to anybody else. A tombstone for something a device has never held is skipped rather
  than created, so deletions do not accumulate as rows on every device.
- **The course cursor is the same shape, one row.** Whoever moved it last wins, and
  the same three-part stamp settles it. Its importer no longer writes a default row
  when there is no document to import — which it used to, and which would have
  overwritten a *synced* position with 0 on any device that had not been paged
  through yet.

Four more tests in `crates/hanzi-sync/tests/two_devices.rs` for the vocabulary list and
the cursor: an entry added on one device arriving on two others with its group; the same
entry edited on two devices ending up the same way on both; a removal staying removed
rather than being resurrected by the copy that was still there; and the course position
following whichever device moved it last. Three more cover the baseline: the acceptance
criterion above, a device with one incomplete character and one complete one, and the
fallback for a card with no baseline to fold from. Five unit tests in `crates/hanzi-sync/src/document.rs`
cover the ordering itself, including the tiebreak, and four in
`crates/hanzi-store/tests/store.rs` cover the storage the merge rests on.

**The order these were built in was the work.** Nothing here was left for later, and
the sequence is worth keeping because each step is the reason the next one is safe:
asking for nothing by default and reading the token once per run had to come *before*
the automatic sync, or a sync nobody asked for would have stopped to ask for something
at a moment nobody chose; and the baseline had to come before that too, because a
schedule a sync cannot rebuild is a schedule a sync can quietly get wrong. The
fingerprint on Android came last and is the same shape of thing as the first step — the
mode a sign-in is kept in, and what that costs the learner when it is read.

**Why.** Practice happens on whichever device is at hand — the laptop at a desk,
the phone on a train — and a schedule that exists on only one of them is a
schedule the learner cannot trust. The requirement is narrower than "cloud": no
paid service, no account of ours, and nothing that makes the app worse for
somebody who declines it.

**What makes this tractable.** Two properties are already in the code, and
neither was added for sync:

- `attempt` is append-only and never rewritten (M10). That is a grow-only set —
  the one shape that merges across devices with no conflict to resolve.
- `Sm2::review` is a pure function of `(CardState, Rating, at)`. A card is
  therefore a *fold over the attempt log*, not a thing to be merged. Two devices
  that practised the same character offline both append; both fold the union and
  agree.

So **the log syncs, the schedule is recomputed, and `hanzi.db` itself is never
synced.** Putting a live SQLite database in a cloud folder is the approach this
milestone exists to avoid: WAL and SHM sidecars are copied out of order, a client
can snapshot mid-transaction, and a conflict leaves a "conflicted copy" nobody
reads. It also cannot work on a phone at all, since neither Android nor iOS
exposes a Dropbox or Drive folder as a filesystem.

**Approach.**

- **Device identity.** A `device_id` (UUID, generated once) in the database's
  `meta` table. A synced row is identified by `(device_id, id)`, so two devices'
  `AUTOINCREMENT` values can no longer collide.
- **Shards, one directory per device, immutable once written.**
  `devices/<device_id>/attempts/NNNNNN.jsonl` (append-only chunks; a closed chunk
  is never rewritten), `vocab.json`, `cursor.json`, `baseline.json`. A shard's
  filename carries the hash of its contents, which makes an upload idempotent and
  gives a sync client nothing to make a conflicted copy *of*. This is what lets a
  dumb transport — a cloud folder — be safe.
- **Ordering.** `at` is ISO-8601 to the whole second, so ties are real. The fold
  runs in `(at, device_id, id)` order: a total order every device computes the
  same way. Without the tiebreak the ease factor, which accumulates in `f32`,
  would drift between devices.
- **The baseline, and the one card that cannot be replayed.** A migrated card has
  `attempts` greater than its rows in `attempt`, because the JSON it came from
  kept only the newest 20 — HANDOVER records this. Fold exactly those from a
  captured `baseline.json` rather than the log, and take the branch with more
  attempts when two devices disagree. Every other card folds exactly.
- **Vocabulary entries are the only genuinely concurrent edits.** They are
  user-authored, editable and deletable, so they gain a UUID, an `updated_at` and
  a tombstone; merge is last-writer-wins per entry on `(updated_at, device_id)`.
  There is no pretending that a sentence edited on two phones can be merged.
- **The course cursor is last-writer-wins** on its existing `updated_at`.
- **`settings` does not sync.** `hanzi-core/src/settings.rs` is built on "an
  absent preference asks the device", so a phone's answer must never overwrite a
  desktop's. That is the invariant, not an oversight.
- **Schema version 3** carries the above: `device_id` in `meta`, `device_id` on
  `attempt`, and `uuid`/`updated_at`/`deleted` on `vocab_entry` — additive, in the
  shape M10 established.
- **A `RemoteStore` trait** (`list`, `get`, `put`) in a new `crates/hanzi-sync`,
  which depends on `hanzi-core` and `hanzi-store`. The merge is pure and tested
  against a local directory; the transport is interchangeable. `hanzi-core` stays
  free of I/O and native dependencies, as its crate note requires.

**Dropbox, and why it is first.** Of the cross-platform options it is the only
one that is both free and available on all three of macOS, Android and iOS
without a paid third party. iCloud Drive has no Android client, so it cannot
serve the phone-to-phone case at all; Syncthing is excellent on macOS and Android
but reaches iOS only through Möbius Sync, which is paid. The specifics:

- An App Console app with **app-folder access**, so the integration can only ever
  see `Apps/HanziTutor` and never the learner's own files.
- **OAuth 2.0 with PKCE** and `token_access_type=offline`, so a refresh token is
  issued and no client secret needs to exist in an AGPL repository — Dropbox names
  "open source applications" and "desktop and mobile apps without a server" as the
  PKCE case. Scopes `account_info.read` (to name the connected account on the
  settings screen), `files.metadata.read`, `files.content.read`,
  `files.content.write`. The refresh token does not expire on its own and is
  reused for every access token after the first.
- **No redirect URI at all, because Dropbox forbids custom schemes.** Every
  redirect URI must be HTTPS, with `localhost` the only exception — so
  `hanzi-tutor://` cannot be registered at all, and a `tauri-plugin-deep-link`
  registration would be dead weight on three platforms. Dropbox's code flow makes
  `redirect_uri` *optional* for exactly this case: the authorization page displays
  the code and the learner pastes it into the app, once per device. The app opens
  that page in the **system browser** — the API says it must not be shown in a
  webview, which also matters for accounts that sign in to Dropbox through Google
  — using the official `tauri-plugin-opener`. One paste per device is the entire
  cost, and it buys a flow needing no server, no hosted domain and no per-platform
  URL registration. This is worth re-testing if the goal is a seamless mobile
  sign-in: Google Drive does allow custom schemes on iOS and Android, which is the
  one axis on which it is the smoother of the two.
- Four endpoints: token, `list_folder` (with its cursor and `/continue`),
  download, upload. `list_folder` returns a `rev` and a `content_hash` per entry,
  so a device downloads only what changed and needs no state beyond the cursor.

**Acceptance criteria.**

- Two sets of shards merged in either order produce byte-identical `CardState`s
  for every character.
- Merging the same shards twice changes nothing.
- Practising offline for any length of time and then syncing loses no attempt.
- A card whose `attempts` exceeds its logged rows keeps its schedule through the
  baseline path.
- With sync unconfigured the app makes no network request, and `hanzi-sync` is
  the only module besides `asr.rs` that can open a socket.

**Left out, and why.**

- **Background sync.** iOS gives an app no background execution, so syncing
  happens at launch, on foreground and on demand. Append-only shards are why that
  is acceptable: how long a device slept cannot cost data.
- **Syncing the schedule as state.** It would hand two devices a conflict to
  resolve where the log hands them a computation.
- **A loopback listener or a hosted HTTPS redirect.** Both are permitted by
  Dropbox and both are worse here than a paste. A hosted redirect means running a
  server, which this milestone exists to avoid. A loopback listener
  (`http://localhost:PORT`) is allowed and is the usual installed-app pattern, but
  on iOS the app is suspended when the system browser comes forward, so the socket
  it is supposed to receive the code on is the fragile part — and it would earn
  only the removal of one paste per device.
- **Dropbox's `dropbox-sdk` crate.** The four endpoints above are small enough to
  call through `ureq` and `serde_json`, which the ASR download already puts in the
  graph, so `LICENSES.md` need not grow. Revisit if the OAuth flow proves
  fiddlier than it looks.
- **Google Drive, Syncthing and a LAN peer-to-peer transport.** Behind the same
  `RemoteStore` trait, later. Google Drive also keeps a personal OAuth app in
  "Testing" mode, where refresh tokens expire weekly — a real trap for this use.

**Privacy, restated.** M12 moved `INTERNET` into the main Android manifest, so
sync adds no permission; what it does add is a reason for the privacy policy and
the Play listing to be restated in the same change, exactly as M12 did, rather
than left claiming the app contacts nothing. Sync ships **off by default**, with
nothing enabled until the learner connects an account, and the app stays complete
without it.

---

## M14 — Graded phrase audio

**Status: in progress.** The build-time clips and the on-device path are built
and tested; the app-level check has not been done.

Pronunciation so far has been the *operating system's* synthesiser
([`src-tauri/src/speech.rs`](src-tauri/src/speech.rs)). That is the right default
— it needs no download and no model — but it has two gaps the research had
already recorded: a desktop build has no backend at all (M6), and a device can
have the language without a voice installed. A graded course also wants something
different from arbitrary speech: the *same* voice on every platform, saying a
fixed, curated list of phrases, so that a learner is not comparing their
pronunciation against a different speaker every time they switch machines.

### What shipped

- **A Rust synthesis crate**, `crates/hanzi-say`, wrapping `sherpa-onnx`'s
  `OfflineTts` — the same toolkit crate and version `src-tauri` already links for
  recognition. The build-time clips and any on-demand speech therefore come from
  one implementation rather than two that drift.
- **`scripts/fetch-phrases.sh`** fetches the two corpora, pinned by SHA-256, into
  `data/phrases/` (gitignored). `scripts/fetch-tts.sh` fetches the MeloTTS
  weights the same way into `.melo-tts/`, checking every file against a recorded
  digest and refusing an unpinned one.
- **`synthesize-audio`**, a binary that turns a corpus into MP3s plus a
  `manifest.json`, and **`scripts/check-audio.py`**, the gate that runs before
  those clips are committed.
- **The clips themselves** under `public/audio/<corpus>/`, committed, because a
  clone should build and run without a 58 MB model download.
- **An on-device path** ([`src-tauri/src/say.rs`](src-tauri/src/say.rs)) for a
  phrase with no clip, following M12's rules exactly: opt-in, verified against a
  pinned digest, nothing degraded without it.
- **The notices**: both corpora and the model are in `licences/`, catalogued in
  `licences.rs` and recorded in `LICENSES.md`.

### Why the corpora are kept apart

`no7z/hsk-sentences-audio` (HSK 1–6 sentences, pinyin, translations) is
**CC BY-SA 4.0**; `harukicoder/hsk30-graded-readers` (102 readers as 1,185
word-aligned sentences) is **CC BY 4.0**. Separate directories, separate
manifests, separate tabs in the interface — a merged list would make it possible
to ship one set under the other's notice.

**Only the text is used, so far.** The `no7z` dataset also publishes its own
MP3s, synthesised with CosyVoice2, and this app has been regenerating every clip
with MeloTTS instead — so that the voice that ships is the voice the app can
synthesise, and CosyVoice2 drops out of the licence catalogue entirely.

**That trade is now in doubt, because MeloTTS mispronounces about 7.5% of these
phrases** while the dataset's own CosyVoice2 audio scored 0% on the same sample
with the same recogniser. The defect is in the syllables produced, not the
signal, so no level or duration check can see it; a pronunciation tutor cannot
ship it. The measurements, what was ruled out, and the options are in
[`docs/research/MELOTTS_PRONUNCIATION_ACCURACY.md`](docs/research/MELOTTS_PRONUNCIATION_ACCURACY.md).
A decision has not been taken yet, and the bundled clips are **not** final.

### Four defects the work turned up, and how each was found

These are recorded because every one of them was invisible in the source and
would have shipped:

1. **The model is very quiet.** MeloTTS returns peaks around 0.06–0.08 — about
   −22 dBFS. Written out unchanged, a clip is audible only with the volume at
   maximum, and everything audible at that gain is the noise floor. Found by
   measuring the raw model output after a listener reported crackle. Fixed by
   normalising each utterance to a peak of 0.7.
2. **Splitting on punctuation produced clicks.** The first chunker cut on every
   sentence mark and glued the pieces with no crossfade, so each phrase's prosody
   restarted at the join. Fixed by sending a whole phrase to the model and
   splitting only when the text is over 60 characters.
3. **An exclamation mark truncates the model.** `谢谢你！不客气。` came back as
   0.96 s with the first clause missing; `谢谢你。不客气。` is 1.57 s and complete;
   `谢谢你！` alone returns 0.12 s of digital silence. Fixed by rewriting `！` to
   `。` for synthesis while the learner still reads the original text. Found only
   because an early ASR round-trip had transcribed the phrase as just
   `不客气。` — which was recorded at the time as an "ASR artefact" and was in
   fact the model dropping a phrase.
4. **`num_threads` made synthesis 45% slower.** Asking for one thread per core
   (16 here) measured 3.40 s per clip against 2.31 s at four threads: the model
   is small enough that extra threads synchronise rather than compute. Found by
   comparing the batch's rate against an earlier measured figure. Fixed at four,
   with the measurements recorded beside the constant.

The silence guard that came out of (3) is worth naming: output below a peak of
1e-3 is a distinct `Silent` error rather than a shipped clip, checked *before*
normalisation so a silent buffer cannot become amplified hiss.

### What is deliberately not done

- **No third-party audio is redistributed**, from either corpus.
- **The upstream level labels are not treated as authoritative.** The `no7z`
  project claims no sentence contains vocabulary above its own level; re-grading
  all 4,354 against this app's own word list found about 5% carrying a token
  above it. The readers' own datasheet records that its shelf target was hit
  61.8% of the time. Both are presented as bands, and the two official HSK 3.0
  documents disagree on a large share of their shared vocabulary anyway.
- **The graded readers' audio is not finished**, and its size is a live question:
  1,185 sentences is about 172 minutes of audio against the phrases' 76.

---

## Explicitly out of scope

To keep the project honest about what it is:

- General handwriting **recognition** (identifying an unknown character from
  strokes). The app grades against a known target, which is a much easier and
  more useful problem for learning. A recogniser would be a separate feature.
- **Traditional characters.** The dataset contains them, but the curriculum is
  simplified-only, which was the original requirement.
- **Cloud accounts and social features.** The whole value of this app is that it
  is offline and private, and nothing here changes that: there is no account to
  make and no server of ours to talk to.
- **Syncing is no longer out of scope** — it is M13, on narrow terms. The
  original objection was that the app must work offline and keep the learner's
  data to themselves; that is a property of the design rather than of refusing
  the feature, so M13 syncs through storage the learner already owns, ships off
  by default, and leaves the app complete without it. Anything that would need an
  account of ours, a server of ours, or a paid service is still out.
- **Speech recognition for tones is no longer out of scope** — it is M11 and it
  has shipped, for characters and words, with no model. Recognising *text* was
  M12 and has now shipped too, as an optional model the learner installs from the
  settings screen; the app still downloads nothing on its own.
- **Speech *synthesis* is no longer out of scope either** — it is M14. The system
  synthesiser is still what speaks text the learner types, and still needs no
  model; M14 adds a fixed set of graded recordings that ship with the app, plus
  an optional second model for a phrase with no recording. The rule M12 set is
  unchanged: nothing is fetched unless the learner presses something.
