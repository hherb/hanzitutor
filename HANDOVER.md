# Handover

Notes for whoever picks this up next — human or agent. It assumes you have just
landed in this repository with no memory of how it was built.

Read in this order:

1. **this file** — how to get a working build and what not to break;
2. [`ROADMAP.md`](ROADMAP.md) — what to do next, in priority order;
3. [`README.md`](README.md) — what the app is, and how grading works;
4. [`LICENSES.md`](LICENSES.md) — the data notice obligations.

## 1. Get a working build first

Do this before touching anything. Two environment quirks will otherwise waste
your time.

```bash
# From the repository root (on this machine: /Users/hherb/src/hanzitutor).
cd /path/to/hanzitutor

# 1. Cargo cannot write to ~/.cargo here (see below), so the project keeps its
#    own. If .cargo-home/ is missing or small, seed it from the global cache:
ls .cargo-home 2>/dev/null || {
  mkdir -p .cargo-home/registry/{cache,index}
  REG=index.crates.io-1949cf8c6b5b557f
  cp -Rc ~/.cargo/registry/cache/$REG .cargo-home/registry/cache/$REG
  cp -Rc ~/.cargo/registry/index/$REG .cargo-home/registry/index/$REG
}

# 2. Data + deps, then confirm the baseline is green.
./scripts/fetch-data.sh          # ~33 MB upstream, into data/raw/ (gitignored)
pnpm install
pnpm run prepare-data            # builds the 13 MB artifact (gitignored)
pnpm test                        # expect 136 passed, 0 failed
pnpm run check:rust && pnpm run check:web
```

Then `pnpm run dev` to launch it. `pnpm run build` makes a bundled `.app`.

### Quirk 1 — cargo must use a project-local CARGO_HOME

The file sandbox is `workspace-write`, so writes to `~/.cargo` are refused
(`Operation not permitted`). **Every** cargo invocation must go through
`scripts/with-cargo-env.sh`, which points `CARGO_HOME` at `.cargo-home/` and
`CARGO_TARGET_DIR` at `.cargo-target/`. Both are gitignored. Calling `cargo`
directly will fail in a confusing way deep inside the registry cache.

To run any cargo command:

```bash
./scripts/with-cargo-env.sh cargo <args>
```

### Quirk 2 — `pnpm tauri` does not work; use the wrapper

`pnpm` here is a shim that runs **Electron as Node**
(`ELECTRON_RUN_AS_NODE=1`). `@tauri-apps/cli` is a napi addon that parses the
*operating system* argv, so under Electron it sees the host application's
arguments and fails with:

```
error: unrecognized subcommand '/Applications/DSH Desktop.app/Contents/MacOS/DSH Desktop'
```

`scripts/tauri-cli.sh` probes the npm CLI and falls back to a standalone
`cargo-tauri` (2.11.1, at `~/.cargo/bin`). **Always** reach the Tauri CLI through
it, or through the `pnpm run dev` / `pnpm run build` scripts which use it:

```bash
./scripts/with-cargo-env.sh ./scripts/tauri-cli.sh <args>
```

To make the project independent of the host's global install:

```bash
pnpm run install:cli     # installs a matching tauri-cli into .cargo-tools/
```

### Other environment facts

- **The study files' default location is not writable under this sandbox.**
  They live in the platform application data directory
  (`~/Library/Application Support/com.hanzitutor.app/` on macOS: `vocabulary.json`,
  `progress.json` and `course-cursor.json`), which is *outside* the workspace, so a
  sandboxed run cannot save them. Point them somewhere writable instead:

  ```bash
  HANZI_TUTOR_DATA_DIR="$PWD/.tmp-vocab" ./.cargo-target/debug/hanzi-tutor
  ```

  The app degrades honestly — it reports the save failure in the UI rather than
  losing data — but for testing persistence you must set this.
- **`npm` is broken** for this user (`~/.npm/_cacache/tmp` contains root-owned
  files). Use `pnpm`; it works, with a project-local store.
- **You cannot screenshot the app.** `screencapture` fails with "could not create
  image from display" because the session lacks Screen Recording permission, and
  AppleScript window inspection is blocked too. Window geometry *is* readable:
  `python3 -c "import Quartz; ..."` with `CGWindowListCopyWindowInfo`. For
  anything visual, **ask the human to look at it** — that is how the canvas and
  the vocabulary screen were confirmed.
- Running the binary directly produces harmless WebKit noise
  (`could not create directory ~/Library/WebKit/...`) because the sandbox blocks
  WebKit's cache directories. It is not an app fault.

## 2. What already works, and how it was verified

| Area | State | Evidence |
| --- | --- | --- |
| Grading engine | Done | 29 unit tests; self-consistent on all 7,744 teachable characters |
| Dataset pipeline | Done | 9,574 characters, 13 MB artifact |
| IPC surface | Done | contract tests asserting exact JSON key sets |
| Drawing canvas | Done, human-confirmed | trace + recall modes, colour-coded feedback |
| Pronunciation | Done on macOS | 9 tests; human-confirmed speaking |
| Personal vocabulary list | Done | 27 store unit tests; persistence tested through the state layer |
| Per-character progress, SRS | Done | 30 store/scheduler unit tests; record → relaunch → due-date cycle tested through the state layer |

Verified end-to-end by reading the app's own logs:

```bash
HANZI_TUTOR_DATA_DIR="$PWD/.tmp-data" ./.cargo-target/debug/hanzi-tutor 2>&1 \
  | grep --line-buffered -vE "could not create directory|WebKit"
# [speech] using voice Tingting (Chinese (China mainland)) (zh_CN)
# [webview] course loaded: 7744 characters in 775 lessons
# [webview] vocabulary: 2 entries in 2 groups
# [webview] progress: 4 practised, 3 due
# [webview] review queue: 2 due, 2 in this session
# [webview] speech: using Tingting (Chinese (China mainland)) (zh_CN)
# [webview] course cursor: resuming at character 413
# [webview] character 的: de, 8 strokes
# [webview] spoke 面
# [webview] graded 十: 100/100, legible=true, order=true
# [webview] progress 十: 100/100, due 2026-09-20T09:00:00Z
```

`progress` and `review queue` are the lines to watch: with four practised
characters and three of them overdue, the queue is **2** items, not 3 — the two
characters of the saved word 学习 are offered as one word. That single pair of
numbers proves loading, the schedule, deduplication and both sources at once.

Use `grep --line-buffered` when capturing those logs, or grep's block buffering
will hide everything and you will conclude the frontend never started.

## 3. Where the code lives

```
crates/hanzi-core/          grading engine. No Tauri, no UI, no platform code.
  src/geom.rs               resampling, normalisation, distances, similarity fit
  src/grade.rs              Hungarian pairing, order analysis, verdicts, scoring
  src/dataset.rs            Character model + artifact loading
  src/curriculum.rs         frequency list -> lessons
  src/vocab.rs              the personal vocabulary list and its JSON file
  src/progress.rs           per-character cards, SM-2 scheduling, review queue
  src/time.rs               ISO-8601 formatting, parsing, date arithmetic
  src/bin/prepare_data.rs   upstream data -> compact artifact (feature = "prepare")
  examples/selfcheck.rs     whole-dataset measurement and tolerance tuning
src-tauri/
  src/commands.rs           the IPC surface; thin wrappers over AppState methods
  src/state.rs              embedded dataset, speech warm-up, the three stores
  src/speech.rs             macOS `say` backend, voice selection
  tests/ipc_contract.rs     locks the JSON contract the UI reads
src/
  App.svelte                shell: modes, navigation, keyboard, state ownership
  lib/PracticeCanvas.svelte pointer capture, coalesced sampling, display space
  lib/render.ts             canvas painting, font<->display transforms, colours
  lib/FeedbackPanel.svelte  report -> readable advice
  lib/LessonSidebar.svelte  course navigation, progress marks, review entry
  lib/types.ts              TS mirror of the Rust structs
  lib/api.ts                typed invoke wrappers
scripts/                    fetch-data, with-cargo-env, tauri-cli
```

**The seam to preserve:** `hanzi-core` must stay free of Tauri and platform
dependencies. That is what makes the engine testable without a window, and what
will let it run behind a mobile shell or a CLI unchanged. Anything that needs the
window goes in `src-tauri`; anything that is *logic* goes in `hanzi-core`.

## 4. Invariants — do not break these

1. **Two coordinate systems.** *Font space* is the published Make Me a Hanzi
   space: upper-left `(0, 900)`, lower-right `(1024, -124)`, so **y increases
   upwards**. Outlines are stored in font space because they render verbatim.
   *Display space* is `0..=1024` on both axes with **y downwards**, matching the
   canvas, and is where **all grading happens**. `Point::from_font` converts, and
   the conversion is applied once at data preparation. Never grade in font space.

2. **The IPC contract is camelCase fields and snake_case enum values**, mirrored
   exactly in `src/lib/types.ts`. `src-tauri/tests/ipc_contract.rs` asserts the
   *exact set* of keys, because a mismatch fails **silently** — the UI just shows
   blanks. Change Rust and TypeScript together; let the test catch you.

3. **All three scores are measured across the whole reference character**, with
   unwritten strokes counting as zero (`grade.rs`, step 7). Do not "fix" this into
   a mean over only the strokes that were written: that reintroduces a bug where a
   blank canvas scored 30/100 and writing half a character scored 62.

4. **Shape cannot discriminate similar strokes; placement does.** Shape distance is
   deliberately scale-invariant, so two 横 of different lengths are "the same
   shape". If you change `SHAPE_TOL`, `POSITION_TOL`, `SIZE_TOL` or the `0.60`
   legibility bar, **re-run `pnpm run selfcheck`** and check the tolerance table
   still behaves (a normal hand should be ~100% legible; the current values give
   100% at 1.5% jitter and 99% at 3%).

5. **`min_stroke_len` is relative as well as absolute** (`min(min_stroke_len,
   shortest_reference * 0.5)`). A fixed cutoff alone discarded a real 4-unit
   stroke in 黧, making a correct attempt impossible to score.

6. **Order scoring is the normalised inversion count, and per-stroke out-of-order
   flags are exactly the strokes taking part in an inversion.** That makes the
   flags and the score provably consistent. A longest-increasing-subsequence
   metric was tried and scored a fully reversed three-stroke character at ⅓.

7. **The artifact is generated, not committed.** `src-tauri/src/state.rs` embeds
   it with `include_bytes!`, and `src-tauri/build.rs` panics early with
   instructions if it is missing. Keep that guard — otherwise a fresh clone fails
   with an inscrutable macro error.

8. **A study file that cannot be parsed is never overwritten.** There are three of
   them now — `vocabulary.json`, `progress.json` and `course-cursor.json` — and
   all three follow the same rule, enforced in one place: `Persisted<S>` in
   `src-tauri/src/state.rs` keeps the reason in `load_error`, refuses to save
   while it is set, and carries it to the UI as `warning`. Losing someone's study
   notes to a parse error would be far worse than refusing to write. There are
   tests for this; do not "simplify" it away by falling back to an empty document.
   The schedule and the cursor are **separate files** on purpose, so one bad file
   cannot take the other down.

9. **Keyboard shortcuts must not fire while a text field has focus.** The
   vocabulary screen has inputs, and `S`/`H`/`Enter` would otherwise trigger
   stroke order, pronunciation and advance mid-word. The `onKey` handler in
   `App.svelte` bails out for `INPUT`, `TEXTAREA` and `SELECT` targets.

10. **Practice has one path for all three sources.** `targetChar` in `App.svelte`
    is the only thing that decides which character the board asks for: the course
    cursor, the current character of a vocabulary entry, or the current character
    of a review item. Loading, grading, the ghost, the hints and the recording of
    progress all key off it. Add a fourth source by extending that derivation and
    the `PracticeItem` queue, not by forking the practice code.

11. **A rating is the grade, and the grade bands live in one place.**
    `Rating::from_score` maps the 0..=100 headline score through
    `Grade::from_score`, so *poor* on screen and *again* in the schedule can never
    diverge. If you move a threshold, move it in `grade.rs` only — and remember the
    UI's colour bands read from the same enum.

12. **Timestamps are ISO-8601 UTC text, and the format is fixed-width.** They sort
    chronologically as plain strings, which is how "is this due?" (`card.due <=
    now`) and the queue's ordering work. `time.rs` owns both directions plus
    `add_seconds`, which **refuses** to produce a year the parser could not read
    back (`MAX_UNIX` is `9999-12-31`). That guard is why the scheduler caps
    intervals at a year: SM-2 multiplies without limit, and an unbounded interval
    eventually writes a five-digit year that parses as garbage. Do not widen the
    format without handling that.

13. **Progress is recorded where the character is graded**, in `check()`, not when
    the learner advances. That is what makes one card cover the course, the
    vocabulary list and review alike, and what makes "practise, relaunch, and it
    is no longer new" true. The vocabulary entry's own mean score is still written
    by `finishCharacter`; the two are different records of the same attempt and
    both are wanted.

14. **The scheduler is behind the `Scheduler` trait.** `Sm2` is the shipped
    policy, and the store takes `&dyn Scheduler` in `record_with`. FSRS is the
    obvious next step and should not require touching the store, the commands or
    the interface. The tests exercise a second implementation to keep that honest.

## 5. The verification loop

Run before every commit:

```bash
pnpm test           # 136 tests: engine + store units, IPC contract, speech
pnpm run check:rust # clippy with -D warnings
pnpm run check:web  # svelte-check
```

Additionally, **if you touched anything in the grading path**:

```bash
pnpm run selfcheck
```

It reports self-consistency (must be a perfect 100 for *every* teachable
character), tolerance under jitter, and shape-metric discrimination. It has found
three real bugs already: the stray-tap cutoff, the blank-canvas scoring, and the
order metric. Treat a regression in its output as a failing test.

`selfcheck` is also the tripwire for milestone M2: it must be **byte-identical**
before and after a scheduling change, because scheduling interprets the score and
must never alter it. Extracting the grade bands into `Grade::from_score` was
checked that way.

**If you touched the scheduler**, the numbers to hold still are in
`progress.rs`'s tests, which pin every interval and due date outright: a failure
is due in 60 s, and passes at 12 h / 1 d / 2 d rising to 1 d / 6 d / `interval ×
ease`, capped at a year. They will tell you which end of the policy you moved.

Then exercise the app and read its logs (see §2). The `graded …` line proves a
real attempt went through; its absence means nobody has graded anything in that
run. `progress …` follows it when the attempt was recorded, and the `review
queue` line says what came due — but note that **drawing cannot be automated
here** (see §6), so those two lines need a human at the trackpad. Everything
downstream of them is covered by the IPC tests, which drive
`AppState::load` → record → save → reload against real temp files.

## 6. Traps that cost time here

- **You cannot drive the window.** `screencapture`, AppleScript window inspection
  and `System Events` keystrokes are all blocked by the sandbox (`A privilege
  violation occurred`). Window geometry is readable with
  `CGWindowListCopyWindowInfo`, and that is the limit. Anything interactive —
  drawing a stroke, clicking *Review due* — needs the human.
- **A stroke has to end where the pointer was released.** `handleUp` in
  `PracticeCanvas.svelte` appends the `pointerup` position before committing the
  stroke. Without that, a quick flick whose only sample arrives with the release
  collapses to a single point, and the grader then discards it as an accidental
  tap — so the learner watches a stroke disappear and gets a puzzling "1 mark was
  too small to be a stroke" note instead of a grade. `push` ignores a sample that
  merely repeats the previous one, so appending it is free.
- **The stray-tap filter is not a scoring rule.** Marks below
  `min(min_stroke_len, shortest_reference * 0.5)` are removed *before* strokes are
  paired, so they cannot influence any verdict or score (there is a test proving
  the report is identical with and without one). The threshold is 12 of 1024
  units; only 2 of 112,617 reference strokes in the whole dataset are that short,
  so it is a genuine accidental-touch filter and not a limit on short strokes.
  Keep the UI's wording free of any causal link to the stroke verdicts.
- **Do not measure the element you are sizing.** The canvas is sized in CSS from
  `side`, which starts at 0, so an observer on *the canvas* can never see a size
  to grow into. `PracticeCanvas.svelte` observes the surrounding board instead.
  This was a silent deadlock — the window opened with an invisible canvas.
- **macOS voice names carry a locale qualifier**: `Tingting (Chinese (China
  mainland))`, not `Tingting`. Compare `base_name()`. A fixture with tidy names
  passed while the real list never matched, so the app quietly used another voice.
- **`say -v '?'` takes ~1 s** and lists every voice twice. Resolve it lazily and
  warm it on a background thread; never on the startup path.
- **Keep pointer-frequency data out of `$state`.** The in-progress stroke lives in
  a plain variable in `PracticeCanvas.svelte` and only schedules a repaint;
  making it reactive would deep-proxy on every pointer move.
- **Vite must ignore `.cargo-target/` and `.cargo-home/`** (already configured) or
  the dev server thrashes watching build output.
- **`cargo build`: the dev profile is `opt-level = 1`** for both the workspace and
  dependencies, so grading feels instant while iterating. Do not remove it.
- **The cursor is only written when it moves.** `set_index` returns false for the
  same position, and the UI debounces by 400 ms and skips a write that matches
  what it just restored. A fresh install therefore creates no `course-cursor.json`
  until you actually navigate. If you are checking persistence by hand and see no
  file, that is why — move a step, then look.
- **An "again" card is due 60 seconds later, not tomorrow.** That is deliberate
  (see `AGAIN_SECONDS`), and it is why the interface refreshes the queue on a
  60-second heartbeat as well as after every answer. If you change the interval,
  change the heartbeat with it or the badge will look stuck.
- **`src-tauri/gen/schemas/`** is generated and gitignored; `capabilities/default.json`
  references it with `$schema`, so editors will warn until the first build. Expected.

## 7. Open decisions

- **Distribution.** For real distribution the Arphic Public License and LGPL texts
  must ship inside the bundle and be surfaced from an About/Licences screen.
  `LICENSES.md` says so; nothing implements it yet. See `ROADMAP.md`.
- **Committing the artifact.** It is gitignored, so a fresh clone needs
  `fetch-data && prepare-data` before it compiles. That is documented and the
  failure is a clear message, but it does mean CI needs a data step. If that
  becomes annoying, commit the 13 MB artifact instead.
- **Shape tolerance is tuned on synthetic jitter**, not on real learners. It wants
  revisiting once there are real attempts to look at — ideally by logging
  attempts and re-running the distribution analysis in `selfcheck`.
- **Vocabulary list scope** — settled: M1 shipped with auto-fill for single
  characters, a composed reading for words, and hand-typed meanings. The
  remaining intentional gaps (CSV import, word meanings, tone sandhi) are
  recorded at the end of `ROADMAP.md` M1.
- **Scheduling scope** — settled: M2 shipped SM-2 behind a `Scheduler` trait, with
  the intervals and file layout recorded at the end of `ROADMAP.md` M2. FSRS was
  deliberately not attempted: it wants a review history one learner will not
  produce quickly, and the trait is the seam for revisiting it. There is no
  export/import for the schedule either — it is derived from practice, and a
  merge format would be guesswork.
- **What the schedule does not yet record**: only a bounded recent history per
  character (20 attempts). A longer log is what M4's tolerance tuning and the
  cross-cutting "attempt logging" item both want, so the two should be designed
  together rather than growing the card format twice.
- **An accepted dependency advisory.** Dependabot flags `glib` 0.18.5 (moderate,
  fixed in 0.20.0). It is Linux-GTK-only and absent from the macOS build graph,
  and it is not fixable from here because `gtk 0.18` pins `glib ^0.18` — cargo
  rejects the upgrade. `ROADMAP.md` records the detail. **Do not spend time on
  it**: if you want to confirm the scope, `cargo tree --target
  aarch64-apple-darwin -e normal | grep glib` returns nothing.
