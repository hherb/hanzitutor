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
| M2 | Per-character progress + spaced repetition | Practice history still does not persist | M | not started |
| M3 | Words and sentences | Single characters are not reading | L | not started |
| M4 | Raster legibility (IoU) | Catches errors centrelines cannot | M | not started |
| M5 | Distribution readiness | Licence notices and signed bundles | M | not started |
| M6 | Pronunciation on Windows/Linux | macOS-only today | S | not started |
| M7 | Centreline stroke animation | Nicer, more accurate "show me" | S | not started |
| M8 | Input ergonomics | Long strokes on a trackpad | S | not started |
| M9 | Mobile shells | A stylus is the right input device | L | not started |

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

**Why.** Your saved words persist, but course practice does not: which characters
you have seen, how you scored, and what is due for review are all forgotten when
the app closes. For studying, this is now the single biggest gap.

**Approach.** The persistence pattern already exists — `hanzi_core::vocab` writes
a versioned, atomically-saved JSON document and refuses to overwrite a file it
could not read. Follow it rather than inventing a second mechanism, and keep the
new store in `hanzi-core` so it stays testable without a window.

- Persist per-character history: attempts, best score, last practised, and a
  scheduling state.
- Scheduling: **SM-2** is simple and well understood; **FSRS** is better but wants
  more data. Start with SM-2-style intervals (again / hard / good / easy derived
  from the attempt score) and keep the algorithm behind a small trait so it can be
  swapped. Timestamps are already ISO-8601 UTC strings, which sort
  chronologically as plain text — compare them as strings.
- A review queue: "due now", ordered by urgency, drawn from both the course and
  the M1 vocabulary list.
- Surface per-lesson completion in the sidebar, and persist the cursor for
  "continue where I left off" — which also fixes how easy it is to end up far from
  where you were while exploring.
- Keep the schedule and the cursor in separate files or keys, so a corrupt
  schedule cannot lose your place.

**Acceptance criteria.**

- Practising then relaunching shows the attempt in history and does not re-show a
  character as "new".
- A character answered well is scheduled further out than one answered badly.
- The review queue is empty immediately after a good session and non-empty once
  items come due.
- Grading output is unchanged (this milestone changes *scheduling*, not
  *grading*); `selfcheck` output is identical.
- A corrupt or missing state file degrades safely — the app tells you and refuses
  to overwrite it, exactly as the vocabulary store does.

---

## M3 — Words and sentences

**Why.** The app teaches single characters. Reading needs vocabulary in context.
This also fixes a correctness problem: polyphonic characters are currently always
spoken with the synthesiser's preferred reading, so 着 is read `zhe` even when the
text means `zháo`. A word carries enough context to know the reading.

**Approach.**

- Add a word data source. Options, in order of preference:
  - **HSK 3.0 vocabulary lists** — small, curated, directly useful, and they align
    with a learner's goals. Check the licence before bundling.
  - **CC-CEDICT** — comprehensive and offline, but large (~10 MB+) and
    CC-BY-SA-4.0, which adds attribution obligations to `LICENSES.md`.
  - User-typed only — no new data, but tedious, and it is what M1 already allows.
- Extend the prepared artifact, or add a second one, with word entries:
  `{ text, pinyin, meaning, hsk, characters }`.
- Grading a word: grade each character in turn with the existing engine, then
  aggregate (mean score, and report the worst character). Do **not** invent a
  whole-word geometric comparison — character-by-character is what a learner
  needs and reuses everything already tested.
- Sentences are a further step: they need segmentation into characters and a
  sensible "one character at a time" flow. Treat as a separate milestone if it
  grows.
- Search: once words exist, "find every word containing this character" becomes
  the natural way to browse.

**Acceptance criteria.**

- A word can be practised by writing each of its characters, with per-character
  feedback and an aggregate score.
- Pronunciation speaks the word, not the isolated characters, so polyphonic
  readings are correct.
- New data is reflected in `LICENSES.md` with its obligations, and the app still
  builds and runs offline.
- The artifact size stays reasonable (target under ~30 MB compressed) or words are
  loaded on demand.

---

## M4 — Raster legibility (IoU)

**Why.** Legibility is currently judged from stroke centrelines. That cannot see
how much ink you actually put down, so a stroke that follows the right path but is
drawn far too thin, or that overshoots wildly, still scores well. Overlapping
strokes and gaps are likewise invisible.

**Approach.**

- Rasterise the user's strokes: draw the polylines with the pen width into an
  offscreen bitmap. A small pure-Rust scanline rasteriser is enough — the drawing
  is just thick line segments with round caps.
- Rasterise the reference: fill the stored SVG outline paths. This needs a path
  rasteriser; `tiny-skia` plus `usvg`, or `lyon` for tessellation, are the
  candidates. Budget for a new dependency here.
- Compare with intersection-over-union, plus a coverage check for areas of the
  reference that received no ink at all (the "you never drew that part" signal).
- Combine with the existing scores rather than replacing them: centrelines judge
  *form and order*, rasterisation judges *ink*. Keep `GradeReport`'s existing
  fields and add the raster measure alongside, so no UI breaks.
- Watch performance: rasterising at 256x256 is plenty and keeps a grade in the
  low milliseconds.

**Acceptance criteria.**

- A stroke drawn along the correct path but roughly a third of the correct width
  scores below the legibility bar, while the same stroke at the correct width
  passes.
- A perfect attempt still scores 100, and `pnpm run selfcheck` remains perfect on
  all 7,744 characters **using the raster measure too**.
- Grading stays comfortably interactive (target under ~20 ms per attempt).
- The new measure is exposed per stroke, so the UI can explain it.

---

## M5 — Distribution readiness

**Why.** The data licences require notices to travel with the app, and an unsigned
bundle is awkward for anyone else to run. Until this is done the app is
personal-use only.

**Approach.**

- An **About / Licences** screen listing the app's AGPL-3.0 licence and the three
  upstream data notices. `LICENSES.md` already records exactly what must ship;
  `scripts/fetch-data.sh` fetches the texts into `data/raw/`.
- Bundle those texts as Tauri resources so they are present in the `.app`.
- Verify the notices survive bundling — this is easy to get wrong and only shows
  up in the packaged app.
- Then: `pnpm run build` producing a DMG, an app icon set (already generated), a
  version string that matches `tauri.conf.json` and `Cargo.toml`, and macOS
  signing/notarisation if the app is to leave this machine.
- Add a GitHub Actions workflow running `pnpm test`, `check:rust` and
  `check:web`. **It needs the data step** — the artifact is gitignored, so either
  run `fetch-data && prepare-data` in CI (slow, ~33 MB download) or cache the
  artifact. Decide and document.

**Acceptance criteria.**

- The bundled `.app` contains the Arphic, LGPL and MIT notices, reachable from the
  UI.
- `pnpm run build` produces a launchable bundle on a clean checkout after the
  documented data steps.
- CI runs the suite on push and is green.

---

## M6 — Pronunciation on Windows and Linux

**Why.** `src/speech.rs` implements macOS only; elsewhere it returns a clear "not
implemented" error rather than shelling out to something unverified. The app is
meant to be cross-platform.

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

**Why.** "Show stroke order" currently reveals strokes cumulatively, which shows
*which* strokes but not *how* each is drawn. The reference centrelines are already
in the dataset, so animating a pen along them is cheap and much better teaching.

**Approach.**

- For each stroke, walk a pen along its median over time, revealing the outline
  progressively — clip the outline fill to the region already swept.
- In canvas, `ctx.save()` / `ctx.clip()` with a polygon covering the swept part,
  or an offscreen buffer where the stroke is drawn and then masked. The median
  points are already in display space, so the sweep is direct.
- Keep it skippable and cancellable; it runs on a `playToken` guard today, which
  is the right pattern to keep.

**Acceptance criteria.**

- The animation visibly traces each stroke rather than popping it in.
- Cancelling (navigating away, pressing the button again) stops cleanly with no
  runaway timers.
- No measurable cost when the animation is not running.

---

## M8 — Input ergonomics

**Why.** Drawing is currently press-and-drag, which is awkward on a trackpad for
long strokes — the practical reason someone would give up on 囊.

**Approach.**

- An optional **click-to-start / click-to-end** mode: the first click begins the
  stroke, movement extends it, a second click commits it. Backspace or Escape
  cancels.
- Consider velocity- or pressure-based stroke width for a more natural line. The
  reference outlines have real width, but grading uses centrelines, so this is
  cosmetic only.
- Offer the choice in the UI rather than forcing it; the current behaviour is fine
  for stylus users who said so.

**Acceptance criteria.**

- A long stroke can be drawn without holding the trackpad button.
- Switching modes does not corrupt an in-progress attempt.
- The stored stroke geometry is identical in shape to the drag version, so grading
  is unaffected.

---

## M9 — Mobile shells

**Why.** A touchscreen with a stylus is the right input device for handwriting
practice; a trackpad is a compromise. Tauri 2 supports iOS and Android.

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

## Cross-cutting polish

Small, independently shippable, roughly in value order:

- **Search and browse characters** — by character, pinyin or meaning. Needed to
  find anything outside the linear course.
- **Import / export the vocabulary list** — CSV and JSON, so the list is not
  trapped in one machine's app data.
- **Keyboard shortcut help** — a `?` overlay; the shortcuts exist but are only
  documented in an empty-state panel.
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
  of synthetic jitter. This is how the shape tolerance should eventually be set.
- **Interface localisation** — the app teaches Chinese but speaks English.

## Known weak spots

Recorded honestly, because they bound how much the current scores mean:

- **Legibility is centreline-only** (see M4). Thin or overshooting strokes pass.
- **Course practice does not persist.** Your vocabulary list does; your place in
  the course and your per-character history do not (see M2).
- **Polymorphic characters are mispronounced** — 着 is read `zhe` whichever reading
  the synthesiser prefers, because there is no context (see M3).
- **Shape tolerance is tuned on synthetic Gaussian jitter**, not real learners. It
  is deliberately loose to suit a trackpad; that leniency may well be wrong.
- **No CI**, so nothing enforces the test suite on push.
- **Pronunciation is macOS-only.**
- **The course is frequency-ordered only.** It starts at 的 (8 strokes), which is
  right for reading but a demanding first character to *write*. A hand-ordered or
  stroke-count-ascending mode may suit a beginner better.
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
