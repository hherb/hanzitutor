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
pnpm test                        # expect 50 passed, 0 failed
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

- **The vocabulary list's default location is not writable under this sandbox.**
  It lives in the platform application data directory
  (`~/Library/Application Support/com.hanzitutor.app/vocabulary.json` on macOS),
  which is *outside* the workspace, so a sandboxed run cannot save it. Point it
  somewhere writable instead:

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
| Per-character progress, SRS | **Not built** | see `ROADMAP.md` M2 |

Verified end-to-end by reading the app's own logs:

```bash
./.cargo-target/debug/hanzi-tutor 2>&1 | grep --line-buffered -vE "could not create directory|WebKit"
# [speech] using voice Tingting (Chinese (China mainland)) (zh_CN)
# [webview] course loaded: 7744 characters in 775 lessons
# [webview] vocabulary: 2 entries in 2 groups
# [webview] speech: using Tingting (Chinese (China mainland)) (zh_CN)
# [webview] character 的: de, 8 strokes
# [webview] spoke 面
# [webview] graded 十: 100/100, legible=true, order=true
```

Use `grep --line-buffered` when capturing those logs, or grep's block buffering
will hide everything and you will conclude the frontend never started.

## 3. Where the code lives

```
crates/hanzi-core/          grading engine. No Tauri, no UI, no platform code.
  src/geom.rs               resampling, normalisation, distances, similarity fit
  src/grade.rs              Hungarian pairing, order analysis, verdicts, scoring
  src/dataset.rs            Character model + artifact loading
  src/curriculum.rs         frequency list -> lessons
  src/bin/prepare_data.rs   upstream data -> compact artifact (feature = "prepare")
  examples/selfcheck.rs     whole-dataset measurement and tolerance tuning
src-tauri/
  src/commands.rs           the IPC surface; thin wrappers over AppState methods
  src/state.rs              embedded dataset + speech warm-up
  src/speech.rs             macOS `say` backend, voice selection
  tests/ipc_contract.rs     locks the JSON contract the UI reads
src/
  App.svelte                shell: modes, navigation, keyboard, state ownership
  lib/PracticeCanvas.svelte pointer capture, coalesced sampling, display space
  lib/render.ts             canvas painting, font<->display transforms, colours
  lib/FeedbackPanel.svelte  report -> readable advice
  lib/LessonSidebar.svelte  course navigation
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

8. **A vocabulary file that cannot be parsed is never overwritten.**
   `VocabState::load_error` holds the reason, `VocabState::save` refuses while it
   is set, and `VocabView.warning` carries it to the UI. Losing someone's study
   notes to a parse error would be far worse than refusing to write. There is a
   test for this; do not "simplify" it away by falling back to an empty list.

9. **Keyboard shortcuts must not fire while a text field has focus.** The
   vocabulary screen has inputs, and `S`/`H`/`Enter` would otherwise trigger
   stroke order, pronunciation and advance mid-word. The `onKey` handler in
   `App.svelte` bails out for `INPUT`, `TEXTAREA` and `SELECT` targets.

10. **Practice has one path for both sources.** `targetChar` in `App.svelte` is
    the only thing that decides which character the board asks for: either the
    course cursor or the current character of a vocabulary entry. Loading,
    grading, the ghost and the hints all key off it. Add a third source by
    extending that derivation, not by forking the practice code.

## 5. The verification loop

Run before every commit:

```bash
pnpm test           # 83 tests: engine + store units, IPC contract, speech, docs
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

Then exercise the app and read its logs (see §2). The `graded …` line proves a
real attempt went through; its absence means nobody has graded anything in that
run.

## 6. Traps that cost time here

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
- **Vocabulary list scope** is being settled with the user; see `ROADMAP.md` M1.
