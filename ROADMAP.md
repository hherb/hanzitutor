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
| M9 | Mobile shells | A stylus is the right input device | L | **in progress (iOS)** |
| M10 | Durable study store (SQLite) | The JSON format caps the attempt log the grading work needs | M | **done** |

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
  names all ten — the app's own AGPL text, this project's provenance record, the
  Arphic and Make Me a Hanzi pointers, the LGPL, the two MIT texts, the CC-CEDICT
  attribution, the CC BY-SA 4.0 legal code and the font's OFL — each with what it
  covers, where it came from and where its bundle copy sits. The text is pulled
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
- **There is no speed control.** One pace, proportional to each stroke, bounded
  for a whole character. A learner who wants to go slower has the step-through
  button and the ghost.

---

## M8 — Input ergonomics

**Status: done.**

Drawing was press-and-drag only, which is awkward on a trackpad: a long stroke
means holding the button down for a long time. There is now a **click to draw**
mode beside the *corrections* switch, off by default, in which one click starts a
stroke, moving the pointer extends it with no button held, and a second click
ends it. The state machine lives in `src/lib/PracticeCanvas.svelte`; `App.svelte`
owns the switch.

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

**Status: in progress, iOS only.** It builds and runs on the iOS Simulator, with
a phone layout made for practice rather than for fitting; the device build and
pronunciation are still to come. What is verified, and how, is below.

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

**What is left.**

- **The physical device — installed, and one check short.** A debug device build
  signs with the team (`X5DWXB4283`, from the environment), exports an IPA, and
  installs onto the connected iPhone with `devicectl`; it has been installed and
  it launches. What is *not* established from here is what it looks like on the
  device: this setup cannot screenshot a physical iPhone, and the app's console
  did not come back through `devicectl --console`. Somebody has to say what they
  see on the screen, and that is the difference between "installed" and "verified
  on a device" for M9's acceptance criteria.
- **iOS release builds.** `ios build` in release fails to link Tauri's Swift glue
  because the release Swift product keeps those symbols local; debug links. This
  is a toolchain/Tauri-version question rather than a change here, and it is
  recorded with the evidence in `HANDOVER.md` §6. Until it is resolved, a device
  build is a debug build, which is fine for practice and not for distribution.
- **Pronunciation.** `speech.rs` is still macOS-only, so on iOS the control is
  disabled and the app says why — honest, but it is one of M9's acceptance
  criteria. It wants `AVSpeechSynthesizer` in-process, which is the same backend
  the macOS sandbox case in §7 needs, so the two should be designed together.
- **Android**, which nothing here has touched.

**Approach.**

- The Tauri config and the `lib` target with
  `#[cfg_attr(mobile, tauri::mobile_entry_point)]` are already in place, and the
  Rust core has no platform dependencies — this was designed for.
- Needs: touch and stylus handling in `PracticeCanvas.svelte` (pointer events
  already cover this, but coalesced-event behaviour and palm rejection need
  verifying), larger touch targets, a layout that works on a phone, and iOS
  speech via `AVSpeechSynthesizer` rather than the `say` binary.
- The 13 MB embedded artifact makes app size acceptable but worth measuring.

**Acceptance criteria.**

- Runs on iOS Simulator and one physical iOS device with working drawing.
- Text and controls are legible and reachable at phone sizes.
- Pronunciation works on iOS without the `say` binary.

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
- **Keyboard shortcut help** — a `?` overlay; the shortcuts exist but are only
  documented inside the "How this works" disclosure (which is collapsed by
  default as of the iOS work, so on a phone they are two taps away).
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
- **A settings screen** — there is now a place for preferences to live: a
  `Settings` document in `hanzi-core`, a `settings` table in `hanzi.db`, and a
  `settings` / `update_settings` pair on the IPC surface. What is missing is the
  dialog, and the second and third preferences to put in it. The first one,
  click-to-draw, is on the board's control row because it is worth reaching
  mid-session; the ones that follow — a colour scheme, board size, the animation's
  pace, a voice — are not, and they want a screen. Two small things to settle
  when it is built: an **Automatic** choice (the stored value is already
  `Option<bool>`, so `null` — "follow the device" — can be offered without a
  schema change) and whether the dialog reports a save failure the way every other
  store does (it should; `SettingsView.warning` is already there for it).

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
- **Pronunciation is macOS-only.**
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

## Explicitly out of scope

To keep the project honest about what it is:

- General handwriting **recognition** (identifying an unknown character from
  strokes). The app grades against a known target, which is a much easier and
  more useful problem for learning. A recogniser would be a separate feature.
- **Traditional characters.** The dataset contains them, but the curriculum is
  simplified-only, which was the original requirement.
- **Cloud accounts, syncing, social features.** The whole value of this app is
  that it is offline and private.
- **Speech recognition for tones.** A different problem from handwriting, and the
  system TTS already covers the output side.
