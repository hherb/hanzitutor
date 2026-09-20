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

# 2. Deps, then confirm the baseline is green. There is no data step: the
#    dataset artifact, the interface font and the licence texts are all
#    committed, so a clone builds without downloading anything.
pnpm install
pnpm test                        # expect 300 passed, 0 failed, 1 ignored
pnpm run check:rust && pnpm run check:web
```

Then `pnpm run dev` to launch it. `pnpm run build` makes a **signed** `.app` and
`.dmg` — see §8 for what that involves and what ends up inside.

`./scripts/fetch-data.sh` is only wanted when you are changing the data pipeline:
it re-downloads the ~33 MB of upstream text into gitignored `data/raw/`, restores
any deleted licence text or font, and `pnpm run prepare-data` then rebuilds the
artifact. Nothing in the normal build path needs either.

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

- **The study data's default location is not writable under this sandbox.** It
  is one SQLite database, `hanzi.db` (with SQLite's own `-wal` and `-shm`
  sidecars), in the platform application data directory
  (`~/Library/Application Support/com.hanzitutor.app/` on macOS), which is
  *outside* the workspace, so a sandboxed run cannot save it. Point it somewhere
  writable instead — either way works, and `--user-dir` is the one to use for a
  built binary:

  ```bash
  HANZI_TUTOR_DATA_DIR="$PWD/.tmp-vocab" ./.cargo-target/debug/hanzi-tutor
  ./.cargo-target/debug/hanzi-tutor --user-dir "$PWD/.tmp-vocab"
  ```

  The resolved directory is logged at startup (`[data] study files in …`), and the
  app degrades honestly — it reports the save failure in the UI rather than losing
  data — but for testing persistence you must set one of these.
- **`npm` is broken** for this user (`~/.npm/_cacache/tmp` contains root-owned
  files). Use `pnpm`; it works, with a project-local store.
- **You can screenshot the app, but not drive it.** Screen Recording is granted
  (it was not, once — if `screencapture` starts failing with "could not create
  image from display", ask for the permission again). Capture the app's **own
  window** rather than the desktop, so nothing else leaks into the shot:

  ```bash
  # The window id comes from Quartz, which needs no extra permission.
  python3 -c "
  import Quartz
  for w in Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements, Quartz.kCGNullWindowID):
      if 'hanzi' in str(w.get('kCGWindowOwnerName','')).lower():
          print(w.get('kCGWindowNumber'))
  " | head -1 | xargs -I{} screencapture -x -o -l {} /tmp/app.png
  ```

  `-l <id>` captures that window alone and `-o` drops its shadow; the result is a
  2x Retina PNG that crops and zooms well. Anything that *acts* is still blocked —
  AppleScript window inspection and `System Events` keystrokes both fail with
  `A privilege violation occurred`, which needs Accessibility, a separate grant —
  so drawing a stroke, clicking *Review due* or scrolling a list still needs the
  human. Window geometry is readable too, via `CGWindowListCopyWindowInfo`.

  This has already earned its place: the progress marks, the review badge and the
  due dots were confirmed from captures, and the same captures showed that a
  restored course position was not scrolled into view (now fixed in
  `LessonSidebar.svelte`).
- Running the binary directly produces harmless WebKit noise
  (`could not create directory ~/Library/WebKit/...`) because the sandbox blocks
  WebKit's cache directories. It is not an app fault.

## 2. What already works, and how it was verified

| Area | State | Evidence |
| --- | --- | --- |
| Grading engine | Done | 34 unit tests (geom + grade); self-consistent on all 7,744 teachable characters |
| Raster ink measure (M4) | Done | 9 rasteriser tests + 8 grading tests; a third-width pen fails all 7,744 characters, a correct trace scores exactly 1.000 on every one |
| Dataset pipeline | Done | 9,574 characters, 13 MB artifact |
| IPC surface | Done | contract tests asserting exact JSON key sets |
| Drawing canvas | Done, human-confirmed | trace + recall modes, colour-coded feedback |
| Stroke-order animation (M7) | Done | pen sweeps each centre-line, the outline revealed behind it; band width measured per stroke; confirmed by window capture |
| Input ergonomics (M8) | Done, human-confirmed | click-to-draw beside the corrections switch, on by default where there is a hover and remembered once chosen; both paths produce identical geometry; driven with synthetic pointer events, and the trackpad behaviour confirmed by hand |
| Pronunciation | Done on macOS and iOS | 19 tests; the iOS voice list is pinned from the simulator's log; the voice preference outranks the automatic choice but yields to the environment override, and falls back rather than going silent; human-confirmed hearing 的 on both |
| Settings screen | Runs, and the store behind it is verified | the fourth sidebar screen. 12 `settings.rs` tests + the store round trip + the IPC contract, which pins the enum names, that a one-field change leaves the rest alone, and that a *deduplicated* voice list is what the screen is offered. Against the running app: a fresh install leaves the `settings` table **empty**, a change writes the expected rows (`voice = Meijia`, `animation_pace = slow`, `board_size = compact`), a chosen voice is applied **before** the warm-up (`[speech] using voice Meijia` on the next launch), and a compact board is visibly smaller. The pace is scale arithmetic with unit tests behind it; the sweep itself was not watched at two speeds (drawing cannot be driven from here, §6) |
| Personal vocabulary list | Done | 27 store unit tests; persistence tested through the state layer |
| iOS app (M9) | Runs on a physical iPhone, human-confirmed | the Rust side cross-compiles unchanged; phone layout verified by simulator screenshot; the scene-lifecycle crash and the black screen behind it are recorded in §6 |
| Durable study store (M10) | Done | one `hanzi.db`; the old JSON imported once and left byte-identical; the attempt log past the 20 a card shows; WAL, and an uncommitted write leaves nothing |
| Tone practice, model-free (M11) | Done, human-confirmed for characters **and words** | 16 `pinyin.rs` tests (syllable splitting, tone reading, sandhi) + 25 `tone.rs` tests (YIN, contours, DTW, segmentation, scoring) + the IPC contract; the device open/record/stop path verified against real hardware (§6); a human confirmed tone hearing and a multi-syllable word (不对, both tones, sandhi explained) |
| Per-character progress, SRS | Done | 30 store/scheduler unit tests; record → relaunch → due-date cycle tested through the state layer |
| HSK 3.0 word list | Done | 9,443 words in the artifact; every one drawable character by character, checked against the shipped dataset |
| Word search and browsing | Done | by character, reading (tones/spacing/`ü` folded) or meaning; exact-beats-prefix ranking proven in tests |
| Words spoken and read whole | Done | the dictionary's own reading is used, so 着急 is `zháojí` where the isolated 着 has no context |
| Licence notices in the bundle | Done | 7 tests pin the catalogue, the files on disk, the compiled-in text and the bundle config against each other; the packaged `.app` was checked by hand (see §8) |
| Nothing downloaded, ever | Done | no HTTP client anywhere in the dependency graph; the dataset, the font and the notices are committed, so a clone and CI build offline |

Verified end-to-end by reading the app's own logs:

```bash
HANZI_TUTOR_DATA_DIR="$PWD/.tmp-data" ./.cargo-target/debug/hanzi-tutor 2>&1 \
  | grep --line-buffered -vE "could not create directory|WebKit"
# [data] study files in /Users/hherb/Library/Application Support/com.hanzitutor.app
# [speech] using voice Tingting (Chinese (China mainland)) (zh_CN)
# [webview] course loaded: 7744 characters in 775 lessons
# [webview] vocabulary: 2 entries in 2 groups
# [webview] progress: 4 practised, 3 due
# [webview] review queue: 2 due, 2 in this session
# [webview] drawable characters: 9574
# [webview] speech: using Tingting (Chinese (China mainland)) (zh_CN)
# [webview] licences: 12 notices bundled
# [webview] course cursor: resuming at character 413
# [webview] character 的: de, 8 strokes
# [webview] spoke 面
# [webview] graded 的: 100/100, ink=1.00/1.00, legible=true, order=true
# [webview] progress 的: 100/100, due 2026-09-21T01:05:42Z
# ...and with the pen declared a third of the width, on the same trace:
# [webview] graded 的: 83/100, ink=0.31/0.34, legible=false, order=true
```

The `graded …` line carries both ink figures — the score first, then coverage —
so the M4 measure can be watched in the field without a debugger. The last three
lines above are from the M4 run (a fresh data directory, hence the different
counts); they are the current format, and the two `graded` lines are the same
trace with the pen declared first at full width and then at a third of it.

`drawable characters` is the word-list counterpart of the course load: 9,574
characters have stroke geometry (against 7,744 with a frequency rank). The
interface needs that set to skip the punctuation in a sentence instead of asking
the board to draw a comma. A `webview error:` or `webview rejection:` line means
an exception reached the window — the handler exists because a rendering error
otherwise shows up only as a window that silently stops updating.

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
  src/raster.rs             stroke ink: scanline fill, pen bands, amount/coverage
  src/grade.rs              Hungarian pairing, order analysis, verdicts, scoring
  src/dataset.rs            Character + Word models, word search, artifact loading
  src/curriculum.rs         frequency list -> lessons
  src/vocab.rs              the personal vocabulary list, and VocabSink
  src/settings.rs           the learner's settings: Settings, Pace, BoardSize,
                            SettingsSink, and the tri-state (None = nobody has
                            chosen) — see §7 for which fields are which
  src/progress.rs           per-character cards, SM-2 scheduling, review queue
  src/tone.rs               YIN pitch tracking, tone contours, syllable
                            segmentation, DTW scoring. Pure DSP: samples in,
                            numbers out, so the scoring is testable against
                            synthetic contours and synthetic words (see §9)
  src/pinyin.rs             splitting a word's reading into syllables, reading a
                            tone off a diacritic, and tone sandhi. Reading rules
                            rather than signal processing
  src/time.rs               ISO-8601 formatting, parsing, date arithmetic
  src/bin/prepare_data.rs   upstream data -> compact artifact (feature = "prepare")
  examples/selfcheck.rs     whole-dataset measurement and tolerance tuning
crates/hanzi-store/         the study database: SQLite, and nothing else.
                            Separate from the engine so the engine keeps no
                            native dependency
  src/schema.rs             the tables, and applying them. Bump SCHEMA_VERSION
                            with any change; the guard reads it *before* apply()
  src/migrate.rs            the once-only, per-document import of the old JSON
  src/lib.rs                ProgressSink / VocabSink / CursorSink, and the
                            attempt log's reader (attempt_count, attempts)
  tests/store.rs            the M10 acceptance criteria, from real saved files
src-tauri/
  src/commands.rs           the IPC surface; thin wrappers over AppState methods
  src/state.rs              embedded dataset, speech warm-up, the three stores
                            (all three opened over the one database)
  src/licences.rs           the catalogue of notices that ship; see §8
  src/capture.rs            microphone capture (cpal): open on press, dropped on
                            release. The stream lives on its own thread because
                            cpal::Stream is not Send on every backend
  src/speech.rs             macOS `say` backend, voice selection
  tests/ipc_contract.rs     locks the JSON contract the UI reads
  tests/licences.rs         pins the notices, the version and the bundle config
  Info.plist                NSMicrophoneUsageDescription, merged over the
                            generated plist at build time
  Info.ios.plist            the scene manifest, plus the same microphone string
  Entitlements.plist        com.apple.security.device.audio-input. Without it a
                            hardened-runtime build captures silence; see §6
src/
  App.svelte                shell: modes, navigation, keyboard, state ownership,
                            and the stroke-order animation's clock
  lib/PracticeCanvas.svelte pointer capture, coalesced sampling, display space,
                            drag and click-to-draw input modes
  lib/CharacterThumb.svelte one small picture of one character's attempt
  lib/render.ts             canvas painting, the stroke-order sweep,
                            font<->display transforms, colours
  lib/FeedbackPanel.svelte  report -> readable advice
  lib/TonePanel.svelte      the learner's pitch contour drawn over the expected
                            tone shape, with the verdict Rust worded
  lib/WordsPanel.svelte     the HSK word list: search, browse, practise
  lib/LicencesPanel.svelte  About and licences: the notices, with their texts
  lib/SettingsPanel.svelte  the four preferences; each change is written at once,
                            and an unchosen one is offered as Automatic
  lib/LessonSidebar.svelte  course, list and word navigation, progress marks
  lib/types.ts              TS mirror of the Rust structs
  lib/api.ts                typed invoke wrappers
  assets/fonts/             Noto Sans SC, the bundled interface face (OFL)
licences/                   every notice text that ships, plus README.md
docs/research/              independent research notes behind open decisions
scripts/                    fetch-data, with-cargo-env, tauri-cli, build-release,
                            probe-app-sandbox
.github/workflows/ci.yml    test + clippy + svelte-check on push
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

3. **All four scores are measured across the whole reference character**, with
   unwritten strokes counting as zero (`grade.rs`, step 8). Do not "fix" this into
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

8. **Study data that cannot be read is never overwritten.** One rule, one place:
   `Persisted<S>` in `src-tauri/src/state.rs` keeps the reason in `load_error`,
   refuses to save while it is set, and carries it to the UI as `warning`. Losing
   someone's study notes to a read error would be far worse than refusing to
   write. There are tests for this; do not "simplify" it away by falling back to
   an empty document. **What "read" means changed in M10** and the rule did not:
   the data is one SQLite database (`hanzi.db`), and the failures it can have are
   a database that will not open (a corrupt file, a schema from a newer build) or
   — once, on the first run — a legacy JSON document that will not parse.
   The three *separate files* this invariant used to name bought isolation from
   that second failure, and M10 keeps it **where it still applies**: the import is
   per document, each records its own marker in `meta`, and a bad
   `course-cursor.json` blocks the cursor and nothing else. Do not collapse that
   into one all-or-nothing import, and do not delete or rewrite the JSON files —
   they are read, and left byte for byte.


9. **Keyboard shortcuts must not fire while a text field has focus.** The
   vocabulary screen has inputs, and `S`/`H`/`Enter` would otherwise trigger
   stroke order, pronunciation and advance mid-word. The `onKey` handler in
   `App.svelte` bails out for `INPUT`, `TEXTAREA` and `SELECT` targets.

10. **Practice has one path for all four sources.** `targetChar` in `App.svelte`
    is the only thing that decides which character the board asks for: the course
    cursor, the current character of a vocabulary entry, the current character of
    a review item, or the current character of an HSK word drilled from the word
    list. Loading, grading, the ghost, the hints and the recording of progress all
    key off it. Add a fifth source by extending that derivation and the
    `PracticeItem` queue, not by forking the practice code.

    A multi-character entry adds one thing on top: `wordSlots` holds one
    `{strokes, report}` per character, which is what the boxes under the board
    show and click into. The rule that keeps it honest is that **the current
    character is never read from its slot** — `slotAt` answers it from the live
    `strokes`/`report`, so the thumbnail follows the pen — and a slot is written
    only when the board *leaves* it (`saveSlot`, called from `selectSlot` and
    `finishCharacter`). One slot therefore holds one grade, so going back to
    improve a character replaces its score instead of averaging it in twice, and
    an entry finishes once every slot has a grade rather than once the last one
    does. Two consequences worth keeping: `reset()` clears the board but not the
    slots (a mode switch abandons the attempt, and the next `saveSlot` writes the
    abandoned state over the slot), and `selectSlot` has to swap the board itself
    when the target glyph does not change, because a word may repeat a character
    (是不是) and the loader effect deliberately does nothing for the same glyph.

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

15. **Placement is measured along the stroke, never from the sample mean.**
    Pointer samples arrive by *time*, not distance, so a stroke drawn slowly at
    one end and flicked at the other reaches the grader with its samples bunched:
    identical geometry, identical ink, identical endpoints. Anything that judges
    where a stroke sits must use `geom::length_centroid` (the centre of the
    polyline as a wire of uniform density), not `geom::centroid` (the mean of the
    samples). Getting this wrong cost up to a fifth of a long stroke's length and
    turned perfect traces into "wrong place" — with the *shape* score still
    reading 95%, because shape is compared after even resampling and so never saw
    it. `position_score` and the global fit both go through the length centroid;
    `fit_on_matched` resamples each matched stroke evenly before deriving the fit
    for the same reason. There are tests for all three.

16. **The artifact is versioned by its magic, and the magic must move with the
    payload.** `ARTIFACT_MAGIC` is `HANZID02` since M3 added the word list; `01`
    carried a bare `Vec<Character>`. `postcard` is not self-describing, so
    decoding an old payload with the new struct would produce plausible nonsense
    rather than an error. Anything that changes the payload shape — adding a
    field to `Character` or `Word`, or adding a third list — must bump the magic
    and regenerate. There is a test that feeds the old `HANZID01` bytes in and
    requires a loud failure.

17. **The word dictionary holds multi-character words only.** A single character
    is the course's job, and `Character` already carries its most common reading
    first. A word entry for the same glyph would be a second, competing source of
    truth: the dictionary lists 安 as the surname `Ān` before `ān` "peaceful".
    `Dataset::lookup_text` enforces the same precedence — one character is
    answered from the character dataset, a longer text from the dictionary if it
    is there, and otherwise by composing the readings with **no** meaning. Never
    let a word entry answer for a single character.

18. **Every listed word must be drawable character by character.** `prepare-data`
    keeps a word only when all of its characters have stroke geometry *and* a
    frequency rank, and prints how many it dropped; the count must stay `0` for
    the "unteachable character" case, and there is an IPC test that walks all
    9,443 words checking each character against `practisable_characters()`. A
    listed word that the board cannot ask for is a dead end in the interface.

19. **Practice skips characters the board cannot draw; it never dead-ends.**
    `entryCharacters` in `App.svelte` filters a word or sentence through the
    character set the backend reports, so the punctuation in a sentence is passed
    over instead of being handed to `getCharacter` and failing. That filter is
    why any text can be practised one character at a time without sentence data.
    It falls back to splitting the text as typed while the set is still loading,
    so a practice session started in the first milliseconds still works.

20. **Ink is measured as *amount*, not intersection-over-union.** `raster.rs`
    rasterises the attempt at the width the canvas actually painted it with, and
    the reference as the filled stroke outline. The headline ink score is the
    attempt's **area** against what a correct trace at the nominal pen width
    (`INK_WIDTH`) would put down; `inkCoverage` separately reports how much of the
    outline was reached. IoU against the outline was tried and is wrong — it falls
    when a correctly-sized band lands slightly off the guide, which is a
    *placement* fault `position_score` already owns. Measured: IoU at 3% jitter
    drops a right-width trace to 0.47 and marked 96% of sloppy-but-correct
    attempts illegible, against 1% before; area keeps that row at 99.1% while
    still failing a third-width pen on all 7,744 characters. Do not "simplify"
    either half back into IoU. Two consequences to preserve: the ink score is
    blind to *where* the ink went (by design), and `legible` is gated on ink
    amount but **not** on coverage, because coverage would double-count the
    wobble again. The module docs in `raster.rs` carry the full argument.

21. **`GradeOptions.ink_width` is the width the ink was drawn with, and
    `INK_WIDTH` is the width a correct trace is judged against — they are not the
    same number.** If the baseline band were rendered at the attempt's own width
    the comparison would cancel and no pen could ever be judged too thin. The
    baseline takes `INK_WIDTH.max(attempt width)`, so a fatter pen is never
    punished. `src/lib/render.ts`'s `INK_WIDTH` and the Rust constant are the same
    value and the interface sends its own through `inkWidth`; if they drift, every
    correct trace is flagged `faint`, which is loud rather than silent but is
    still a bug. Changing the canvas pen width means changing both.

22. **The ink measure's weight in `overall` is a first guess.** Shape, placement,
    ink and order each carry `0.25`, and the fractions are exact binary numbers so
    a flawless attempt sums to exactly `1.0` and scores exactly `100` — the IPC
    contract test asserts `overall == 100.0`, so keep them dyadic. The equal split
    is what stops a character drawn with a third of its ink reading "Excellent"
    (92 before M4, 85 after); it is a judgement, not a measurement, and the
    attempt log the roadmap keeps asking for is what should tune it.

23. **Every notice that ships is named in one catalogue, and three things are
    checked against it.** `src-tauri/src/licences.rs` lists, for each notice, the
    text, its file under `licences/` and its path inside the bundle. The text is
    pulled in with `include_str!`, so it is **compiled into the binary** and
    cannot be lost at packaging time; `tauri.conf.json`'s `bundle.resources`
    copies the same files into `Contents/Resources/licences/` for anyone auditing
    the bundle without launching it. `tests/licences.rs` fails if the three
    disagree in *any* direction: a file in `licences/` that is not catalogued, a
    catalogued file that is missing, a file edited without the compiled copy
    following, or a bundle config that copies a different set. Adding a notice
    means touching all three, and the test is what makes forgetting one loud
    instead of a licence breach discovered after shipping. The same file pins the
    version across `Cargo.toml`, `tauri.conf.json` and `package.json`, which
    otherwise drift apart in silence.

24. **The bundled font is a file this project licenses, under a name it
    controls.** The interface's Chinese face is Noto Sans SC, committed at
    `src/assets/fonts/NotoSansSC-VF.ttf` and declared in `src/app.css` as
    `"Noto Sans SC Bundled"` — deliberately *not* `"Noto Sans SC"`, because a
    machine with that family installed would otherwise match the installed copy
    rather than the one the bundle licenses and the About screen credits. The
    system CJK stack stays declared behind it, so a missing font file degrades to
    the old behaviour rather than to blank boxes. It is only for Chinese rendered
    as *text*; the board draws the stored outlines and uses no font at all.

25. **The engine decides what to remember; the store decides where.** Three traits
    in `hanzi-core` — `ProgressSink`, `VocabSink` and `CursorSink`, at the bottom
    of `progress.rs` and `vocab.rs` — are the seam, and `crates/hanzi-store` is the
    only thing that knows SQL. The engine must stay free of `rusqlite`: it is a
    native dependency, and the point of `hanzi-core` is that it can be tested
    without a filesystem and reused behind a mobile shell. Two consequences to
    keep. **The engine's tests must not know which store is in use** — they open a
    JSON file or nothing at all, and they were not touched when the app moved to
    SQLite. And `ProgressSink::save` is handed the changed cards *and* every
    attempt recorded since the last save, deliberately: `CardState::history` is
    capped at `MAX_HISTORY`, so a sink left to infer the log from it would silently
    drop the 21st attempt of a burst. Do not "simplify" that into a whole-document
    save — a whole-document save is the ceiling the database exists to remove.

## 5. The verification loop

Run before every commit:

```bash
pnpm test           # 300 tests: engine + data pipeline units, the SQLite store, IPC contract, speech, notices, data-dir flag
pnpm run check:rust # clippy with -D warnings
pnpm run check:web  # svelte-check
```

These three are exactly what `.github/workflows/ci.yml` runs on every push and
pull request, on a macOS runner, with no data step.

`pnpm test` enables hanzi-core's `prepare` feature deliberately, so the data
pipeline's parsing — which upstream fields are trusted and how a word's reading is
chosen — is covered by the same run. Without the feature flag the nine
`prepare-data` tests silently do not run.

Additionally, **if you touched anything in the grading path**:

```bash
pnpm run selfcheck
```

It reports self-consistency (must be a perfect 100 for *every* teachable
character, on all four measures), tolerance under jitter, shape-metric
discrimination, robustness to how the pointer sampled the stroke, what the ink
measure can see, and the cost of a grade. It has found five real bugs already: the
stray-tap cutoff, the blank-canvas scoring, the order metric, the sample-mean
placement metric, and — in M4 — the attempt at measuring ink with
intersection-over-union, which the tolerance table exposed by dropping a sloppy
hand from 99% legible to 4%. Treat a regression in its output as a failing test.

The checks to read first, and what "healthy" looks like:

```
  0 of 7744 teachable characters are not perfect
  ink (M4, 1.0 = as much as a correct trace reaches): min 1.000  ...  mean 1.000
  a correct trace below the 0.6 ink bar: 0  <- must be 0
  characters with a Faint stroke: 0  <- must be 0
  normal  sigma=15   legible 100.0%  ...
  sloppy  sigma=30   legible  99.1%  ...
  worst overall drop 1.75  worst stroke placement drop 0.137 (覷 stroke 4)
  strokes whose verdict changed with the sampling: 0  <- must be 0
  third-width pen  min 0.295  ...  below 0.6: 7744  <- every character
  cost: 0.8 ms per grade on 鱻 (33 strokes)  <- target is under 20 ms
```

The `0` on the verdict line is a hard invariant, not a statistic. If sampling
alone changes a verdict, placement is being measured from the sample mean again
and the number explodes into the thousands.

The ink lines are the M4 tripwires. `a correct trace below the ink bar` and
`characters with a Faint stroke` must both be **0** — a correct trace is
normalised to score exactly 1.000 on ink by construction, and the moment that
stops being true, "perfect" is unreachable for some character and the bar is
wrong. `third-width pen ... below 0.6: 7744` is the measure actually working. The
tolerance rows `sigma=15` and `sigma=30` must stay at 100% and 99.1%: if they
collapse again, the ink measure has started double-counting placement (which is
what IoU did) rather than measuring ink.

`selfcheck` is also the tripwire for milestone M2: it must be **byte-identical**
before and after a scheduling change, because scheduling interprets the score and
must never alter it. Extracting the grade bands into `Grade::from_score` was
checked that way. M3 was checked the same way for a different reason: it changed
the artifact (a new payload struct, 9,443 words added), and every figure — the
self-consistency 0 included — is unchanged, which is what proves the word list
touched no geometry.

M4 is the first milestone that was *allowed* to move these numbers, because it
changed grading on purpose. What it moved: the self-consistency scores stay
exactly 100 and the verdict-change count stays 0; the tolerance table's mean score
drops a few points (shape/placement/order lost weight to ink) and `sigma=50`
legibility went 53.2% → 52.4% because the ink bar now fails 8% of very rough
attempts. `sigma=15` and `sigma=30` legibility are unchanged. If you touch the
rasteriser or the weights, re-read that table rather than assuming.

M7 changed no Rust at all — it is the canvas and the state above it — so
`selfcheck` must come out **byte-identical**, and it does: 0 of 7,744 not perfect,
0 verdict changes under resampling, the same tolerance rows. Reading it is still
worth the minute, because it is the cheapest proof that an interface change did
not reach into the grader. M10 moved the study data and touched no grading code
either, and the same reasoning applies: `selfcheck` is unchanged.

**If you touched the store**, the tests that matter are `crates/hanzi-store/tests/store.rs`:
they start from documents written by the app's own JSON stores rather than
fixtures, and they cover the import, the markers, the untouched bytes, the log
past the 20 a card shows, WAL, and an uncommitted write. `src-tauri/tests/ipc_contract.rs`
then covers the same ground through `AppState`, which is where the `Persisted`
rules live. And check it **on the built binary** with `--user-dir`, because the
data directory is the part a unit test cannot see:

```bash
./.cargo-target/debug/hanzi-tutor --user-dir "$PWD/.tmp-db"   # then look in .tmp-db
sqlite3 .tmp-db/hanzi.db "select key, value from meta; select ch, attempts from progress_card;"
```

That is how M10 was checked: the binary created `hanzi.db` in the chosen
directory, imported all three legacy documents, recorded the markers, and left the
JSON byte-identical. Remember that a plain `cargo build` binary does not render
(see §6) — for a run you can *look at*, use `pnpm run dev`.

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

- **Look, don't try to act.** `screencapture` now works (window-targeted; recipe
  in §1), so verify visual work yourself instead of asking. Acting is still
  blocked: AppleScript window inspection and `System Events` keystrokes both fail
  with `A privilege violation occurred`, needing Accessibility rather than Screen
  Recording. Anything that clicks, types or scrolls needs the human.
- **You can still screenshot a graded panel without drawing.** Because the board
  cannot be driven, feedback-panel changes look unverifiable — but `pnpm run dev`
  serves the frontend over HMR, so a two-line temporary seed in `loadCharacter`
  (grade `next.medians` and call `check()`) renders a real report for a capture,
  and reverting is instant. M4's panel, its advice lines and the `faint` colour
  were confirmed that way, and the seeded 100/100 and 83/100 runs reproduce the
  `graded …` log lines quoted in §2. Take the seed out again before committing.
  **M7's stroke-order animation needs the same trick** and is otherwise
  unreachable: `setTimeout(() => void playStrokeOrder(), 1500)` in
  `loadCharacter`, then capture the window every ~0.3 s for a few seconds. A
  capture runs slower than the animation, so expect a handful of frames per
  stroke — enough to see a stroke half-revealed with the pen at its head. Logging
  a line per stroke (`TEMP play stroke i/n`) is what showed the stop and
  navigate-away paths really do end the loop rather than leaving a timer behind.
- **The stroke-order sweep is a clip, not a fade, and the band's width comes from
  the outline.** `drawSweptStroke` in `render.ts` fills the outline clipped to
  the band the pen has covered. Make the band a constant and you get one of two
  failures, both visible immediately: too narrow and the outline's edges arrive
  in disconnected fragments that look like a rendering fault; wide enough for the
  widest stroke and a short 点 flashes in whole. So `strokeRadii` measures each
  stroke's half-width once per character by walking outward from its centre-line
  with `ctx.isPointInPath` and caches it. The trap inside that: `isPointInPath`
  takes its point in *canvas* coordinates while the path is transformed by the
  current matrix, so the measurement clears the transform and works in font space
  at 1:1 — measuring under the drawing transform silently reports nonsense.
- **A key that means two things has to be cancelled in the capture phase.** In
  click-to-draw mode (M8) Backspace abandons the open stroke, and with no stroke
  open it is still the app's undo — two listeners on `window`, one in
  `PracticeCanvas` and one in `App`. Registering the cancel with
  `addEventListener("keydown", …, true)` puts it in the capture phase, where
  `stopPropagation()` keeps the event from ever reaching `App`'s bubble-phase
  handler. Both listeners on the *same* node would not do it: at-target phase runs
  capture and bubble listeners in registration order, so only
  `stopImmediatePropagation` would separate them — and that is not what a real
  keypress does, where the target is `body` and `window` is an ancestor.
- **Pointer work can be verified here after all — with synthetic events.** Drawing
  cannot be *driven* (§1, §6), but `PracticeCanvas`'s handlers are ordinary DOM
  handlers, so a temporary seed can dispatch `new PointerEvent("pointerdown" |
  "pointermove" | "pointerup", {clientX, clientY, buttons, pointerId: 1,
  bubbles: true})` at the canvas with coordinates computed from the canvas rect,
  and the app's own log (a `TEMP …` line through `api.log`) reports what the state
  became. Two things to know: stub `setPointerCapture` / `releasePointerCapture`
  first, because a synthetic pointer id is not a real pointer and the call throws;
  and dispatch key events at `document.body` with `bubbles: true`, not at
  `window`, or the capture trick above is not exercised. That is how M8's six
  cases were checked — the same geometry from both modes, Escape, Backspace,
  `pointercancel` and a mid-draft mode switch — and it is worth reaching for
  before declaring any input change unverifiable. Take the seed out afterwards.
  Synthetic events still do not prove that the *platform* delivers hover moves
  the way the handlers assume, so the last step is a human at the trackpad:
  click-to-draw was confirmed that way after M8 landed, and that confirmation is
  what the §2 row means by "human-confirmed".
- **A stroke has to end where the pointer was released.** `handleUp` in
  `PracticeCanvas.svelte` appends the `pointerup` position before committing the
  stroke. Without that, a quick flick whose only sample arrives with the release
  collapses to a single point, and the grader then discards it as an accidental
  tap — so the learner watches a stroke disappear and gets a puzzling "1 mark was
  too small to be a stroke" note instead of a grade. `push` ignores a sample that
  merely repeats the previous one, so appending it is free.
- **Pointer samples arrive by time, not by distance.** This is the trap that hid
  the placement bug for a long time, and it has two halves. First, never measure
  position with a sample mean (invariant 15): sample density is a record of
  drawing *speed*, so the mean moves when the learner speeds up, and the resulting
  verdict — "wrong place" on a stroke they traced, with the shape score reading
  95% — is impossible to act on. Second, **synthetic jitter cannot find this class
  of bug**, because jittering a reference preserves its sample density: the whole
  tolerance table stayed healthy while 18,763 strokes changed verdict under
  realistic sampling. Anything that consumes pointer geometry needs a
  density-perturbed case as well as a noisy one; `selfcheck`'s fourth section is
  that case, and it is the only reason this is now visible.
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
- **The cursor row only changes when the cursor moves.** `set_index` returns false
  for the same position, and the UI debounces by 400 ms and skips a write that
  matches what it just restored, so a fresh install writes no cursor row until you
  actually navigate. If you are checking persistence by hand and see nothing in
  `course_cursor`, that is why — move a step, then look.
- **A plain `cargo build` binary renders a blank window.** The frontend comes from
  `build.devUrl` (`http://localhost:1420`) unless the Tauri CLI builds it for
  production, so `.cargo-target/debug/hanzi-tutor` run by hand is a white window
  with no `[webview]` lines — only `[data]` and `[speech]` — unless Vite is up.
  Use `pnpm run dev` to look at the app, or the binary inside a built `.app` to
  test the packaged path. This is not a broken build, and it wasted an hour once.
- **The Rust side already cross-compiles for iOS.** `cargo check -p hanzi-tutor
  --target aarch64-apple-ios` succeeds unchanged — engine, bundled SQLite, Tauri
  and all — which is worth knowing before estimating M9. It emits three dead-code
  warnings in `speech.rs` (the `Command`/`Stdio` imports, `hold`, and
  `parse_voices`; the macOS-only pieces are not gated tightly enough), and clippy
  with `-D warnings` would fail on them, so that is the first thing to tidy.
- **An iOS build cannot finish inside this file sandbox.** The same check gets all
  the way through the dependency graph and then dies in Tauri's Swift glue:
  `swift-rs` builds `tauri/mobile/ios-api` with `swift build`, which wants the
  swiftpm caches under `~/Library` and applies its *own* nested sandbox, both of
  which workspace-write refuses — `sandbox_apply: Operation not permitted`, the
  same wall `scripts/probe-app-sandbox.sh` documents in §7. It is the environment
  rather than the code: with a wider sandbox the check finishes in 20 s. Expect
  `tauri ios init`, `ios dev` and `xcodebuild` to need the same, since they write
  to `~/Library/Developer` and the CocoaPods caches.
- **The study data is a database, so "look at your data" means `sqlite3`.** The
  tables are `progress_card`, `attempt` (every attempt, ever), `vocab_entry`,
  `vocab_group`, `course_cursor` and `meta`. `hanzi.db-wal` and `hanzi.db-shm`
  beside it are SQLite's write-ahead log and shared memory, not stray files; the
  `-wal` file is where a save lives until the next checkpoint, which is exactly
  what makes a kill mid-write survivable. The three JSON documents of an older
  install are **imports**, not outputs: after the import they are never read or
  written again, so do not "tidy them up" and do not add code that rewrites them.
- **`tauri ios dev` is for *devices*; simulators are a different route.** The
  `[DEVICE]` argument is matched against connected hardware, and the CLI prints
  simulators in that same "Detected connected device" list — so passing a
  simulator's name makes it build with `-sdk iphoneos`, which then fails asking
  for a development team and provisioning profiles. That error is misleading: the
  target was a simulator all along. The two routes that work here are
  `tauri ios dev --open` (opens Xcode; you pick a simulator and press Run — the
  only path that gives hot reload) and, headlessly,
  `tauri ios build --debug --target aarch64-sim --ci` followed by
  `xcrun simctl install <udid> "<app>"`, `xcrun simctl launch <udid> com.hanzitutor.app`,
  and `xcrun simctl io <udid> screenshot shot.png` to look at it.
- **The test device is a *simulator* unless `xcrun devicectl list devices` says
  otherwise.** This machine has an iPhone for every model name and five iOS
  runtimes, and `xcrun devicectl` is what distinguishes hardware ("available,
  paired") from the `simulated` column. The one real phone is `HHIP1`, an
  iPhone14,3.
- **AVFoundation objects are not `Send`, so iOS speech runs on the main thread.**
  `Retained<AVSpeechSynthesizer>` cannot live in `Speaker` — Tauri requires the
  shared state to be `Send + Sync`, and objc2 marks these classes main-thread-only
  by default — so the synthesiser lives in a `thread_local` on the main thread and
  every call goes through `with_main`, which runs inline when it is already there
  (dispatching synchronously to the queue you are standing on is a deadlock) and
  otherwise hops the queue and waits on a channel. Only plain data (`Vec<Voice>`,
  a `Result`) crosses back. On iOS the `Utterance` field does not exist at all:
  the macOS backend holds a `say` process, iOS holds nothing, which is why the
  struct has a `#[cfg]` on that field.
- **iOS speech takes the audio session, and gives it back.** The device build
  spoke on the simulator and was silent on the phone with no error anywhere,
  because the default `soloAmbient` category is muted by the Ring/Silent switch —
  and the simulator cannot reproduce that, having no such switch. Pronunciation
  now sets `playback` + `spokenAudio` + `duckOthers` for the duration of an
  utterance and deactivates the session (`notifyOthersOnDeactivation`) when the
  synthesizer reports it finished or cancelled. A deliberate tap on "Hear it" is
  therefore audible whatever the switch says, while a learner's music is ducked
  rather than stopped and returns to volume when the word ends. Two details are
  load-bearing:
  * The session is released only when `isSpeaking` is false. `Speaker::speak`
    stops the previous utterance before starting the next, and AVFoundation may
    deliver that cancellation *after* its replacement has begun: releasing
    unconditionally then cuts the new word off mid-syllable.
  * `AVSpeechSynthesizer.delegate` is a **weak** property, which is why the
    synthesizer lives in a `Speech` struct beside its `Retained` delegate rather
    than alone in the `thread_local`. The delegate is ordinary Rust with no
    `MainThreadOnly`, and its callbacks arrive on the main thread, which is also
    where the session calls belong.
  * Failing to take the session is logged and otherwise ignored: it costs volume,
    not speech.
- **Never hold the synthesizer's `RefCell` borrow across an AVFoundation call.**
  A delegate callback can run inline on the main thread during
  `speakUtterance`/`stopSpeakingAtBoundary`, and it borrows the same
  `thread_local`; a live `RefMut` would panic. Both call sites clone the
  `Retained<AVSpeechSynthesizer>` out of the borrow and call through the clone.
- **iOS 26 and 27 kill an app that has not adopted the scene life cycle — and it
  looks like nothing at all.** On the phone the app showed a black flash, closed,
  and printed nothing to its own log; the only evidence was a crash report
  (`idevicecrashreport -u <udid> -k <dir>`), whose faulting frame names it
  exactly:

  ```
  EXC_BREAKPOINT (SIGTRAP)
  UIKitCore  ___UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption_block_
  ```

  The simulator did not complain because it runs iOS 18. `tao` already implements
  the scene delegate (`TaoSceneDelegate`, in
  `tao/src/platform_impl/ios/scene.rs`), so nothing needs patching — what was
  missing is the declaration that makes UIKit create a scene at all:
  `UIApplicationSceneManifest`, which Tauri's iOS template does not add. It lives
  in `src-tauri/Info.ios.plist`, which the CLI **merges at build time** (not at
  `ios init`: re-initialising alone left the generated `Info.plist` without it,
  while the built app's did have it). Two details are load-bearing and both were
  learned the hard way:

  * `UIApplicationSupportsMultipleScenes` must be **true**. That is not a claim
    that this app wants several windows: it is the switch that puts `tao` into
    scene mode at all (`multiple_scenes_enabled()` in
    `tao/src/platform_impl/ios/scene.rs`). With it **false** the crash above is
    gone but the app shows a **black screen** — tao takes its pre-scene path,
    creates the window in `didFinishLaunching` before any scene exists, and a
    window that is not attached to a scene is invisible once a manifest is
    present. A black screen instead of a crash is much harder to read: nothing
    is logged and the crash reports stop.
  * There must be **no `UISceneConfigurations`**. tao answers UIKit's
    `configurationForConnectingSceneSession` with a `UISceneConfiguration` named
    `TaoScene` and sets its delegate class to `TaoSceneDelegate` itself; naming a
    delegate in this file as well is a second, competing source of truth.

  Diagnose this class of failure on the **simulator**, not the phone: the same
  manifest that black-screens an iOS 27 phone black-screens an iOS 18 simulator,
  and the simulator can be screenshotted.
- **A device build writes `DEVELOPMENT_TEAM` into the generated
  `project.pbxproj`.** It is Xcode's doing, not the project's; revert that line
  before committing, so the file does not carry one person's team.
- **`ios build` cannot replace a stale archive.** A second build fails with
  `failed to rename app …/hanzi-tutor_iOS.xcarchive/Products/Applications/Hanzi
  Tutor.app: Directory not empty (os error 66)` — the previous app is still in the
  archive's products directory. `rm -rf src-tauri/gen/apple/build` first, every
  time; it is gitignored, so nothing is lost.
- **A device run is `ios build` + `devicectl`, and the phone must be unlocked.**
  `tauri ios build --debug --target aarch64 --ci` produces an **IPA**
  (`src-tauri/gen/apple/build/arm64/Hanzi Tutor.ipa`), not a loose bundle:
  unzip it and install `Payload/Hanzi Tutor.app` with
  `xcrun devicectl device install app --device <udid> <app>`, then
  `xcrun devicectl device process launch --device <udid> com.hanzitutor.app`.
  A locked phone refuses the launch with `Unable to launch … because the device
  was not, or could not be, unlocked` — the only step here that needs a human.
  `idevicescreenshot` (libimobiledevice) reports "No device found" for this
  iPhone, so **screenshots of the physical device are not available** from here;
  `devicectl … --console` is the intended channel for the app's own log lines,
  though it did not forward Rust's stderr on this setup. Ask the person holding
  the phone what they see — that is the honest check, and it is what M9's device
  criterion rests on.
- **iOS *release* builds do not link; debug does.** `ios build` without `--debug`
  fails at the app link with `symbol(s) not found for architecture arm64` for
  every Tauri Swift entry point (`_run_plugin_command`, `_register_plugin`,
  `_on_webview_created`, `_log_stdout`, `_init_plugin_dialog`). The cause is
  visible in the archives: in `Products/Release-iphoneos/libTauri.a` those
  symbols are **local** (`t`), where `Products/Debug-iphoneos/libTauri.a` exports
  them (`T`), so the Rust staticlib can only bundle them in a debug build. That
  wants a Tauri or Swift toolchain version, not a change here; until then a
  device build is `--debug`.
- **`crate-type` deliberately has no `cdylib`.** Tauri's template includes it for
  Android, but cargo builds it for iOS too, where its link fails the same way
  (Swift search paths are passed, `-lTauri` is not) and cargo treats that as fatal
  before Xcode ever runs. iOS links the `staticlib`; Android will need `cdylib`
  back.
- **The team ID belongs in the environment, not in a committed file.**
  `APPLE_DEVELOPMENT_TEAM=X5DWXB4283` on the build command is enough for
  `-allowProvisioningUpdates` to provision the app. Xcode will write
  `DEVELOPMENT_TEAM` into the generated `project.pbxproj` when it does; that line
  is **not** committed, for the same reason the macOS signing identity is not in
  `tauri.conf.json`.
- **A build produced by `tauri ios build` embeds the frontend; it does not use
  the dev server.** So a layout change is not a hot reload — it needs
  `vite:build` (which `ios build` runs) *and* a Rust rebuild to re-embed, which
  is a couple of minutes per look. `ios dev --open` is the only HMR route on iOS.
  Do not drive `xcodebuild` at the project directly: the "Build Rust Code" phase
  asks the parent CLI for its options over a WebSocket and panics with
  `failed to read CLI options … Connection refused` without one.
- **`gen/apple/tauri` is a file this project adds, and a full re-init deletes
  it.** The npm CLI's template runs `node tauri ios xcode-script …` from
  `src-tauri/gen/apple`, but `ios init` does not write that entry point, so the
  build stops at `Cannot find module '…/gen/apple/tauri'`. The committed shim
  forwards to `@tauri-apps/cli`; it has to be an **ES module** (`import`, not
  `require`) because the repository's `package.json` sets `"type": "module"`. The
  standalone CLI generates a different phase (`cargo tauri …`) that needs no
  shim — whichever CLI initialised the project decides which you have.
- **Xcode 27 refuses an iOS deployment target below 15.0,** and Tauri's template
  defaulted to 14.0. It is set in `tauri.conf.json` as
  `bundle.iOS.minimumSystemVersion` (which maps to `IPHONEOS_DEPLOYMENT_TARGET`).
  Note that `tauri ios init` **leaves an existing `project.yml` alone**: to make
  the config take effect you must delete `src-tauri/gen/apple` and re-init, which
  also deletes the shim above.
- **A signing identity's parenthetical is not the team ID.** `cargo-mobile2`
  reports `Apple Development: someone@example.com (Y38YQNR57Q)`; passing that
  value as `APPLE_DEVELOPMENT_TEAM` gives `No Account for Team "Y38YQNR57Q"`.
  The team this project signs with is `X5DWXB4283`, and it belongs in the build
  environment rather than in `tauri.conf.json`, for the same reason the macOS
  identity is not in there.
- **SQLite does not create the directory for you.** `Connection::open` fails with
  "unable to open database file" if the data directory is missing, which is
  exactly the state a first run is in — and the warning it produces blames the
  database, not the missing folder. `Db::open` therefore does `create_dir_all`
  first, which is what the JSON stores used to do as they wrote. If that line
  ever moves, every fresh install breaks, and `--user-dir` at a path that does
  not exist yet breaks with it. There is a test that starts with no directory.
- **Port 1420 may be held by an orphaned Vite.** `pkill -f vite` does not always
  match it, because `pnpm` here runs Electron as Node, and a survivor makes
  `pnpm run dev` fail in `beforeDevCommand` while the app itself still starts
  against the *old* dev server — so a stale bundle can be what you are looking
  at. Check `curl -s -o /dev/null -w '%{http_code}' http://localhost:1420` before
  trusting a run; a leftover server serving the same directory will happily serve
  current sources, which is convenient but makes "which bundle is this?" a real
  question.
- **A compiled-in dependency is a shipped notice.** SQLite and `rusqlite` are the
  first third-party *code* in the binary, and they were added to
  `src-tauri/src/licences.rs`, `tauri.conf.json` and `licences/` together because
  the tests only check the three against *each other* — a dependency nobody
  catalogued is invisible to them. Adding one means checking its licence by hand
  and adding a notice, and the count in the §2 log line moves with it.
- **An "again" card is due 60 seconds later, not tomorrow.** That is deliberate
  (see `AGAIN_SECONDS`), and it is why the interface refreshes the queue on a
  60-second heartbeat as well as after every answer. If you change the interval,
  change the heartbeat with it or the badge will look stuck.
- **`src-tauri/gen/schemas/`** is generated and gitignored; `capabilities/default.json`
  references it with `$schema`, so editors will warn until the first build. Expected.
- **A stale artifact fails on the magic, not on the JSON.** `crates/hanzi-core/data/hanzi.bin.gz`
  is gitignored, so a checkout that pulled an artifact-changing commit keeps the
  old file and the app refuses to decode it. The message says *re-run
  `prepare-data`*; do that rather than hunting for a bug in the loader. `build.rs`
  checks only that the file *exists*, not that it is current, so this is a runtime
  error rather than a build one.
- **`prepare-data` is the only place the word list is filtered.** Words whose
  characters lack geometry or a frequency rank are dropped there, not at load, so
  "why is this word missing?" is answered by its `words from …: N kept` line. If
  you add a rule, add it there and print the count it dropped — the silent
  version of that filter is how a word list ends up with entries the board cannot
  draw.
- **Clicking a character in a word is a search, not navigation.** `WordsPanel`
  turns the click into a query for that character, which is the browse-by-radical
  route the roadmap asked for. The course jump lives on the sidebar instead.
- **A word can repeat a character, so never key a per-character `{#each}` by the
  character.** 是不是 is one of the first words in the list, and keying the glyph
  tiles by `ch` throws `each_key_duplicate`. The nastier half is *how* it fails:
  the error is raised inside Svelte's render flush, so the window simply stops
  updating — the panel sat on "Searching…" with the correct data already in
  state, and nothing appeared in the terminal, because a webview's console is not
  visible from here. Key by index for character tiles. The `error` /
  `unhandledrejection` handler in `App.svelte` now forwards that class of failure
  to `[webview] webview error: …`, which is what turned this from a mystery into
  a one-line answer — keep it.
- **A notice that names a licence is not the licence.** `fetch-data.sh` had been
  fetching Make Me a Hanzi's `COPYING`, which describes what `graphics.txt` and
  `dictionary.txt` are derived from and sends the reader to a URL for the Arphic
  Public License. That URL was never followed, so the app would have shipped the
  notice *about* the licence without the licence itself. The same was true of the
  CC BY-SA legal code. If you touch the data pipeline or the notices, check that
  every licence a notice points at is also present as a **file**;
  `tests/licences.rs` now requires the text, and looks for a phrase only the
  genuine text contains, so a stub cannot pass as a licence.
- **`include_str!` makes the licence texts part of the Rust build.** Editing
  `LICENSES.md` or anything in `licences/` recompiles `hanzi-core` and
  `hanzi-tutor`. That is correct, and surprising when a build you expected to be
  instant takes ten seconds. It also produced the only false alarm this suite has
  given: a test compares the compiled-in text with the file on disk, so editing a
  text *while* cargo is compiling leaves the binary holding the old copy and the
  comparison fails. Re-run before investigating.
- **Bundle resource paths are relative to `src-tauri/`**, not to the repository
  root, so a notice at `licences/x.txt` is written `"../licences/x.txt"` in
  `tauri.conf.json`. `tests/licences.rs` derives that string from each catalogue
  entry and compares the whole map, so a config that drifts from the catalogue
  fails the suite rather than the bundle.
- **`pnpm run build` signs.** `scripts/build-release.sh` picks a Developer ID
  certificate from the keychain automatically, so a build on someone else's Mac
  will be signed as *them* unless they set `APPLE_SIGNING_IDENTITY` — and on a
  machine with no certificate it falls back to ad-hoc, which still launches
  locally. `pnpm run build:unsigned` skips the wrapper entirely.
- **A microphone needs two things, and only one of them is obvious.** A signed,
  hardened-runtime build cannot open the microphone without
  `com.apple.security.device.audio-input` in `src-tauri/Entitlements.plist`, and
  the *user* is asked by `NSMicrophoneUsageDescription` in `src-tauri/Info.plist`.
  They are not alternatives: the plist key is the prompt, the entitlement is the
  kernel's permission. **The failure is silent and looks identical in both
  cases** — capture opens, streams, and delivers zeros, which the analyser reports
  as "I could not hear enough voice". Worse, it works in `tauri dev` and in an
  unsigned build and then stops working once signed, so check the two commands in
  `README.md` ("What is inside the bundle") on any build someone is going to run.
  Note that a plain `tauri build` without `build-release.sh` is only
  *linker-signed* and never applies the entitlements at all — that is not a
  failure of the config, it is the signing step not having run. Verify with
  `codesign -d --entitlements - "$APP"`, which prints the dictionary when it is
  right and nothing when the file was not applied.
- **A terminal cannot be granted the microphone the way an app can**, and on this
  machine it has not been. So `cargo test -- --ignored` records silence here while
  the packaged app works, and **no test in this repository has ever heard a human
  voice**. Do not read a passing ignored test as proof that capture works; read
  the peak level it prints. See §9.

### Android

- **Every Gradle and emulator command needs a wider sandbox than `workspace-write`.**
  Gradle writes its dependency cache under `~/.gradle`, the emulator writes its
  lock and userdata files under `~/.android/avd`, and `adb` wants `~/.android`
  for its keys. All of those are refused with `Operation not permitted`, and the
  failure mode is not a clean error: a Gradle build *hangs* with no output
  because `cmd | tail` swallows the progress, and the emulator dies with
  `A snapshot operation … is pending and timeout has expired`, which reads like a
  stale snapshot rather than a permissions problem. Run these with
  `danger-full-access`, and do not pipe a long build to `tail` while diagnosing.
- **Android needs the `cdylib` crate type, and there is no way to have it only
  there.** `crate-type` is not per-target. `src-tauri/Cargo.toml` carries all
  three now and says why; the risk is that the iOS *link* trips over the dylib
  again, which `cargo check --target aarch64-apple-ios` will not show because it
  does not link.
- **`minSdk` is 26 because of AAudio, and it has to be set in two places that
  agree.** `cpal` pins the `ndk` crate to its `api-level-26` feature, so the
  library needs `libaaudio.so`, which does not exist before Android 8. At
  `minSdk = 24` the link fails with `unable to find library -laaudio` — note
  *which* clang it used, `aarch64-linux-android24-clang`: that API level comes
  from `bundle.android.minSdkVersion` in `tauri.conf.json`, which is what the CLI
  uses to pick the linker, while the manifest's `minSdk` comes from the generated
  `build.gradle.kts`. Setting only the Gradle one still fails to link.
- **Gradle cannot find the Tauri CLI under pnpm, and the error is a bare
  `Cannot find module`.** `gen/android/buildSrc/.../BuildTask.kt` runs
  `node tauri android android-studio-script` from `src-tauri/`, expecting a
  package literally named `tauri` to be resolvable from there. pnpm does not
  flatten `@tauri-apps/cli` into such a name, so the task dies with
  `Cannot find module '<root>/src-tauri/tauri'` after the Rust library has
  already linked. `src-tauri/tauri.js` is a committed shim that loads the
  package's real entry point — `tauri.js`, not `main.js`, which only exports an
  API and would silently do nothing. It is an `import`, not a `require`, because
  the root `package.json` says `"type": "module"`.
- **The webview starts loading *while* `setup` runs, so nothing slow may happen
  there.** This is the trap that cost the most. On desktop the windows are created
  after `setup`, so a slow setup is invisible; on Android the frontend is running
  by ~300 ms and its first commands arrive before `app.manage()`, and a command
  that finds no state is **rejected, not queued** — the phone showed *"state not
  managed for field `state` on command `review_queue`"* and an empty board, and a
  second launch lost the commands entirely and sat on "Loading the character
  set…" forever. Decoding the 13 MB dataset and ordering the course now happen in
  `AppState::prepare()` **before the Tauri builder exists**, and `setup` only
  opens the study database. Note that the fix is placement, not speed: the decode
  is only ~1 s of CPU, which is still 3× the webview's head start. Anything else
  slow added to `setup` — or to `AppState::assemble` — reintroduces this. Two
  useful diagnostics, both cheap:
  * `adb shell am start -W -n com.hanzitutor.app/.MainActivity` gives the real
    cold-start time (`TotalTime`). 262 ms once the one-off ART compilation after
    an install is past — the first launch after `adb install` takes ~90 s and is
    not the app's fault.
  * The webview is debuggable in a debug build, so the page can be inspected and
    driven over the DevTools protocol: `adb forward tcp:9222
    localabstract:webview_devtools_remote_<pid>`, then `curl
    http://127.0.0.1:9222/json` for a WebSocket URL. Node 22+ has a global
    `WebSocket`, so no dependency is needed. This is how the pending-promise
    state, the missing managed state, and the `env()` value were all confirmed
    rather than guessed.
- **Logcat is unusable on the test phone.** `adb logcat -d` returns only kernel
  and radio lines — no app output at all, on any buffer, including device-side
  `logcat`, on a RedMagic NX809J. Do not plan on `[data] …` or `eprintln!` lines
  there; use the DevTools probe above, or read the app's own data directory with
  `adb shell run-as com.hanzitutor.app ls`.
- **`env(safe-area-inset-*)` is the display cutout, not the status bar.** Android's
  WebView reports the cutout, so on a phone with a punch-hole the CSS was right by
  luck and on the emulator it was zero — the header was drawn under the clock.
  Both are edge-to-edge and cannot opt out (targetSdk 35+), so the page is told
  the real system bar insets by the `android_insets` command, and `app.css` takes
  `max(env(…), var(--inset-…))`. Taking the maximum is what makes one rule right
  on both: on a notched phone the two agree instead of summing.
- **Tauri dispatches mobile plugin commands on the Android main thread.**
  `run_command` goes through `run_on_android_context`, so a Kotlin plugin method
  that blocks freezes the UI, and one that waits on a latch deadlocks. The
  speech plugin resolves *later* instead: commands that arrive before
  `TextToSpeech` reports ready are queued and answered when it does, which is the
  normal case because the pronunciation warm-up asks for the voice list while the
  app is still starting.
- **Android offers a network voice beside the on-device one for the same
  locale.** For an app whose whole premise is that it needs no network, the
  default choice matters: the Kotlin side sorts `isNetworkConnectionRequired`
  first, so the automatic pick is `cmn-cn-x-ccc-local` rather than a
  network-backed voice that would fail offline. The Rust `Voice` model needed no
  new field — `pick_voice` takes the first mainland voice in the order the
  platform reported it.
- **An unset `ndkVersion` silently costs 170 MB.** Gradle needs the NDK to find
  `llvm-strip`; without a `ndkVersion` it gives up with a single line —
  *"Unable to strip the following libraries, packaging them as they are"* — and
  ships the 203 MB debug library verbatim. With it set, the same APK is 76 MB.
  It is taken from `ANDROID_NDK_HOME` with the development version as a fallback
  so that a build from Android Studio, where no such variable is set, still works.
- **A back press has to be answered synchronously.** `OnBackPressedCallback` must
  decide *now*, so the page is asked with `evaluateJavascript` and a global
  function (`window.__hanziHandleBack`) whose return value settles it; an event
  listener would need a round trip that cannot be waited for on the main thread.
  And the callback must be registered **enabled** — `OnBackPressedCallback(false)`
  is never invoked and the platform default quietly applies, which looks exactly
  like the feature not working.
- **`adb shell input` is a real enough input device to verify drawing.** `input
  swipe` on the board produced a stroke, moved the counter to `1 / 8`, enabled
  Undo/Clear and graded to a score — real touch events through the WebView's
  pointer handling. `input tap` drives buttons the same way. What it does not
  prove is palm rejection or stylus behaviour, which still wants a person.
- **The emulator that ships with Android Studio was full, and it is not ours to
  wipe.** `Medium_Phone_API_36.1` had 299 MB free of 6 GB with three of the
  owner's own test apps on it, so installing there would have meant deleting
  someone else's data. `HanziTutor_API36` is a second AVD cloned from its
  `config.ini` with a 12 GB data partition instead — `avdmanager` is not
  installed on this machine (there is no `cmdline-tools/`), so the two files were
  written by hand: `~/.android/avd/HanziTutor_API36.ini` pointing at
  `HanziTutor_API36.avd/config.ini`, whose `image.sysdir.1` is what ties it to the
  already-installed system image. No image had to be downloaded.
- **`key.properties` belongs to the Android project root, not to `app/`.** The
  `.gitignore` there is the one that excludes it, and Gradle's module directory is
  `gen/android/app/`, so `file("key.properties")` looks in the wrong place and
  finds nothing — silently. The build does not fail; it just signs nothing, and
  the only signal is that the artifact is called
  `app-universal-release-unsigned.apk` instead of `app-universal-release.apk`.
  Read that filename literally. Use `rootProject.file("key.properties")`.
- **R8 does not break the Kotlin plugin bridge, but not because of anything
  here.** Release builds minify, and `register_android_plugin` instantiates
  `PlatformPlugin` **by name** — a reflective lookup that R8 cannot see. What
  saves it is that Tauri's Android library ships *consumer* ProGuard rules
  (`-keep @app.tauri.annotation.TauriPlugin public class *` and the same for
  `@InvokeArg`), which apply to the app automatically. Debug-only verification
  would never have caught a problem here, so the release build was run on a
  device: the course loads, a stroke grades, and "Hear it" is live — which is
  what proves `voices` answered through the reflective bridge.
- **The release build is not debuggable, so it cannot be probed over DevTools.**
  Wry enables webview debugging in debug builds only, so `webview_devtools_remote`
  has no socket in a release build. Verify the release by installing it,
  screenshotting, and driving it with `adb shell input` — which is what caught
  the difference between "it built" and "it works".
- **`INTERNET` is scoped to the debug source set.** The app downloads nothing and
  makes no requests at runtime, so the released manifest does not declare the
  permission at all — `aapt2 dump permissions` on the signed APK shows only
  `RECORD_AUDIO`, which makes "works offline" checkable by anyone holding the
  artifact. Debug builds keep it in `app/src/debug/AndroidManifest.xml` because
  `tauri android dev` loads the interface from a development server.
- **Play needs more than an AAB.** Because the app asks for the microphone, the
  listing requires a published privacy policy and a Data safety declaration, and
  the answers have to match what the app really does. `docs/privacy-policy.md`
  and `store/listing.md` hold both; the policy still needs a real contact
  address and a public URL before submission, and both are marked with TODOs.
- **The version code comes from the app version.** `tauri.properties` derives
  `2000` from `0.2.0`, and Play requires it to increase with every upload, so a
  second upload means bumping the version in `Cargo.toml` and `tauri.conf.json`
  first. `bundle.android.autoIncrementVersionCode` exists for people who would
  rather not remember.

### Android speech and the microphone

- **`TextToSpeech.speak` returning `SUCCESS` means nothing was heard.** It means
  the engine *accepted* the utterance. The voice's data may be missing, a network
  voice may have no network, the output may not open — all of which produce
  silence, or a progress callback nobody is listening to, from a call that
  reported success. The first version of `PlatformPlugin.speak` resolved as soon
  as `speak()` returned, so on a phone that could not make a sound the app said
  everything was fine. It now resolves when `UtteranceProgressListener.onStart`
  fires and **rejects with the engine's own error code** when it does not, with a
  three-second guard so a silent engine cannot hang the caller. Both `onStart` and
  `onError` are matched by **utterance id**: `speak` cuts off the previous
  utterance, and the engine reports that cancellation in its own time, so without
  the match a stale failure gets blamed on the next request.
- **A listed voice is not necessarily a usable one, and on a phone that has never
  had a network none of them are.** Android's engine advertises every voice it
  knows and marks the ones whose data has never been downloaded with
  `Engine.KEY_FEATURE_NOT_INSTALLED`. The test phone reported **16 Chinese voices
  and 0 installed** — it had no connectivity at all, so nothing had ever been
  fetched — and asking it to speak produced a service error or silence depending
  on the moment. With WiFi on, the same query answered **14 installed** and
  speech worked, using `cmn-cn-x-ccc-local`, an on-device voice. So: sort
  installed voices first (Rust takes the first Mandarin voice it sees, so this is
  what decides the default), prefer on-device over network, and keep the
  promise that the *runtime* needs no network — the download is a one-time
  device setup step, like installing a font, which is why the app can still ship
  with no `INTERNET` permission.
- **A vendor ROM may mute the synthesiser outright, and the app has to object.**
  The RedMagic build logs, on every attempt:
  `AudioHardening background playback would be muted for com.google.android.tts`,
  with the music stream at full volume and the app in the foreground. This is the
  Android twin of the iOS Ring/Silent problem in this same file: a synthesiser
  that works for another app is silenced for this one unless the app says the
  sound is deliberate. `speak` now takes audio focus
  (`AUDIOFOCUS_GAIN_TRANSIENT_MAY_DUCK`, borrowed from the iOS session's
  `duckOthers`) and sets `USAGE_MEDIA` + `CONTENT_TYPE_SPEECH`, giving the focus
  back on `onDone`, on error, and on `stop`.
- **Android 11 and later hide the speech engine unless it is asked for by
  intent.** `<queries><intent><action
  android:name="android.intent.action.TTS_SERVICE"/></intent></queries>` in the
  manifest, or `getDefaultEngine()` answers null on a phone that plainly has an
  engine installed.
- **`TextToSpeech.getDefaultEngine()` does not resolve against this compile SDK.**
  Read `Settings.Secure.TTS_DEFAULT_SYNTH` instead — it is the same value the
  platform uses, and on the test phone it is *empty*, which is itself the answer
  to "why is this phone silent": no synthesiser had ever been chosen there.
- **The microphone has to be asked about more than once.** The status was fetched
  once at startup, with the comment that "whether the machine has one does not
  change while the app runs". On Android that is false in the most annoying
  possible way: the permission is granted through a dialog that is dismissed
  *after* that first read, so the answer is always "no" and a control disabled on
  it stays disabled for ever. `MainActivity.onRequestPermissionsResult` now tells
  the page to ask again, and the page also re-asks whenever it comes back to the
  foreground, which is what catching a permission granted from the system
  settings looks like.
- **On a phone there is no tooltip, so a disabled control has to say why.** This
  cost a round of confusion: `Hold to say it` was greyed out and read as a
  microphone fault, when in fact the microphone was fine and **的** simply has no
  judgeable tone — it is a neutral-tone particle, and `tone_target` returns
  `None` for it. The button now labels itself `No tone to score` or
  `No microphone` rather than `Hold to say it`. A separate explanatory paragraph
  was tried first and rejected: it wrapped to its own row and pushed the rest of
  the controls off the screen.

- **Push-to-talk has three ways to stop itself, and the third is not obvious.**
  The button moved out from under the finger (the label narrows on press *and*
  clearing the previous judgement removed the panel above it), `touch-action` let
  the browser claim the touch for a scroll, and — the one that survived both
  fixes — **Android's long-press selection gesture takes the pointer at 555 ms**
  and sends `pointercancel`. The button now captures the pointer, keeps a
  `min-width`, leaves the previous judgement on screen, sets `touch-action: none`
  and `user-select: none`, and swallows `contextmenu`, which is what the practice
  board had been doing all along. §9 has the measurements and the event log that
  found it.
- **`cpal`'s Android input does not work; `AudioRecord` does.** Capture is
  per-platform for that reason — `cpal` elsewhere, Kotlin's `AudioRecord` on
  Android — and §9 has the AAudio log, the five-source probe and the three
  details of the Kotlin backend that are easy to get wrong.

## 7. Open decisions

- **Speech recognition of *text* (M12) — the product decision is taken, the code is
  not written.** Tone practice needs no model and shipped without one. Recognising
  *what* was said is different: every usable Mandarin recogniser is a neural model
  of ~155 MB (`sherpa-onnx` + SenseVoice), which cannot be bundled.
  **Decided by the project owner: a download is acceptable provided it is optional,
  user-triggered, and installed from the settings screen.** The app must keep
  working exactly as it does today for anyone who declines — no prompt, no nag, no
  degradation. The full set of constraints is in ROADMAP M12, and the model choice,
  packaging and licensing research is in
  `docs/research/ASR_TTS_CLAUDE_RESEARCH.md` §5, §7 and §9. **Until that is built,
  the app makes no HTTP request at all**, which is asserted in §2 and checked with
  `cargo tree`; whoever adds the download is the one who restates the README's
  promise as "nothing is downloaded unless you ask", and adds the model's own
  licence notice the way `cpal`'s was added.

- **Committing the artifact** — settled the other way, deliberately. The 13 MB
  artifact and the 17 MB interface font are both committed, so a clone and a CI
  run go straight from `pnpm install` to a build with no download. The cost is
  ~30 MB of binary in the repository; the 33 MB of upstream text stays ignored.
  `.gitignore` records the reversal and the command that regenerates the artifact.
- **The ink measure is proven, but half of it cannot fire yet.** The canvas paints
  every stroke at one fixed width, so nothing a learner does on a trackpad can put
  down *less* ink than `INK_WIDTH` and the `faint` verdict is unreachable in daily
  use; what M4 does catch today is overshoot and short strokes. The width half
  becomes live when input can report a real pen width — a stylus. **M8 chose not
  to add the velocity-thickened brush it had listed as "cosmetic only"**, and the
  reason is worth keeping: ink amount became a quarter of the score in M4, so
  stroke width is a grading input now, and a brush that thinned with speed would
  score a fast stroke worse for being fast. Doing it properly means the grader
  takes a width per stroke and `INK_OK` is re-tuned against real attempts — the
  attempt log's job (M10). Whichever comes first wants real attempts to re-tune
  `INK_OK` against.
- **The four headline weights are a judgement, not a measurement.** An equal
  quarter each, chosen so a third-inked character cannot read "Excellent". The
  honest way to set them is the attempt log below, on real handwriting.
- **Shape tolerance is tuned on synthetic jitter**, not on real learners. It wants
  revisiting once there are real attempts to look at — ideally by logging
  attempts and re-running the distribution analysis in `selfcheck`.
- **Vocabulary list scope** — settled: M1 shipped with auto-fill for single
  characters, a composed reading for words, and hand-typed meanings. M3 changed
  one half of that: a word in the HSK dictionary now gets its real reading *and*
  its real meaning, and only a word the dictionary does not know falls back to a
  composed reading with a blank meaning. The remaining intentional gaps (CSV
  import, tone sandhi, sentence segmentation) are recorded at the end of
  `ROADMAP.md` M1 and M3.
- **Word dictionary scope** — settled: the HSK 3.0 lists, multi-character entries
  only, MIT compilation with CC-CEDICT readings and definitions (CC BY-SA 4.0).
  The share-alike obligation is real and is recorded in `LICENSES.md`; it is why
  the upstream fields that would add a third licence (SUBTLEX-CH frequency, HanLP
  part-of-speech) are not bundled. If a future milestone wants published word
  frequencies, that is a licensing decision, not just a data one.
- **A word's reading is chosen by a rule, not resolved by context.** The first
  dictionary form wins unless it is a capitalised proper noun, in which case the
  first ordinary reading wins. That fixes 安 (`Ān` the surname → `ān` peaceful)
  but leaves a minority of genuinely ambiguous headwords on their less common
  reading (便宜). There is no context to do better without the sentence, so this
  is documented rather than papered over.
- **Scheduling scope** — settled: M2 shipped SM-2 behind a `Scheduler` trait, with
  the intervals and file layout recorded at the end of `ROADMAP.md` M2. FSRS was
  deliberately not attempted: it wants a review history one learner will not
  produce quickly, and the trait is the seam for revisiting it. There is no
  export/import for the schedule either — it is derived from practice, and a
  merge format would be guesswork.
- **The attempt log exists; what reads it is still to come.** M10 shipped the
  unbounded log (`attempt`, with `Db::attempts` and `Db::attempt_count` to read
  it) and the schedule now shows the newest 20 attempts per character from it, but
  nothing yet *exports* it and `selfcheck` still tunes the tolerances against
  synthetic jitter. The cross-cutting "attempt logging" item is therefore half
  done on purpose: the ceiling is gone, and the analysis that wanted it is the
  next thing to build. A migrated card's `attempts` count can exceed the rows in
  the log — the JSON it came from kept only the newest 20 — so any analysis must
  treat the log as starting at the import, not at the learner's first attempt.
- **Where study data lives** — settled: the platform's application data directory,
  resolved through the platform API rather than assembled from `$HOME`, and
  overridable with `--user-dir` (which wins) or `HANZI_TUTOR_DATA_DIR`. Not a
  `~/.hanzi-tutor` of our own: a Mac App Store build is sandboxed, the real home is
  not writable there, and some home-directory APIs still return the real home
  inside a sandbox — so a hand-built path fails only at save time. **The format is
  settled too** (M10): one SQLite database, `hanzi.db`, holding all three stores,
  imported once from the JSON documents an older build left behind.
- **Where settings live, what an unchosen one means, and the screen that edits
  them** — settled, and built. Preferences are a `Settings` document in
  `hanzi-core`, stored as rows in `hanzi.db`'s `settings` table, with **no row**
  meaning "nobody has chosen" rather than "off". That distinction is the whole
  point: the interface resolves an unchosen preference from the device
  (click-to-draw where there is a hover, dragging for a finger or a stylus) or
  from the system (the pronunciation voice) and only writes a value once the
  learner has actually made a choice. `src/lib/SettingsPanel.svelte` is the screen
  — the fourth sidebar entry — and it edits four preferences: click-to-draw, the
  stroke-order pace, the board size, and the voice. Three rules there are worth
  not undoing:
  - **Only a preference with a device or system answer is a tri-state.**
    `click_to_draw` and `voice` are `Option`; `animation_pace` and `board_size`
    are plain enums with a `Default`, because nothing can resolve their absence
    and `Option` would only add a state no one can observe. `Pace`/`BoardSize`
    carry `ALL` (the order the screen offers) and the scale the app applies, so a
    new choice cannot be added in one place and forgotten in the other.
  - **Clearing is spelled per preference on the wire**, because a *missing*
    argument already means "leave this one alone" — that is what lets the screen
    send only the control the learner touched without resetting the other three.
    Click-to-draw therefore has its own command (`clear_click_to_draw`), and a
    voice clears with `""`. Do not try to fold these into `null`.
  - **A preference at its default is stored as no row.** Otherwise a fresh
    install writes two rows saying "normal", and the settings table stops being a
    record of decisions somebody took.
- **The voice preference only works if it reaches the speaker before the
  warm-up.** `AppState::load` sets it on the `Speaker` *before* `warm_voice`
  spawns. Resolution is not cached (only the ~1s voice *list* is), so a change
  takes effect on the next utterance; but resolving first would mean the session's
  first utterance and the startup log both name a voice that is not the one in
  use. The environment variable `HANZI_TUTOR_VOICE` deliberately **outranks** the
  stored preference — see `resolve_voice` in `src-tauri/src/speech.rs`, which is a
  pure function of (installed voices, preference, override) precisely so that the
  ordering is testable without a synthesiser.
- **The JSON documents are never removed, and nothing exports back to them.** The
  import leaves `vocabulary.json`, `progress.json` and `course-cursor.json`
  untouched on purpose — they are the only copy of the data until the database has
  it, and a rollback to an older build is then possible. Nothing writes them
  again, so they go stale the moment the app runs; that is intended, and the
  vocabulary list's own JSON export (File → export in the list screen) is the
  user-facing escape hatch, not these files.
- **The App Store path is untested, and one part of it is at risk.** A Mac App
  Store build must be sandboxed, and `src/speech.rs` pronounces by spawning
  `/usr/bin/say` — which a sandbox may refuse. `scripts/probe-app-sandbox.sh` was
  written to settle it and **could not do so from here**: applying any sandbox
  profile is refused in this development environment (`sandbox-exec -p '(version
  1)(allow default)' …` → `sandbox_apply: Operation not permitted`), and an app
  signed with `com.apple.security.app-sandbox` and launched through launchd ran
  with the entitlement present but unenforced. The script detects exactly that and
  reports "inconclusive" rather than a false answer. Run it from a normal login
  session, or put a build on TestFlight and try *hear it* there. If `say` is
  refused, the macOS backend needs `AVSpeechSynthesizer` in-process — the same
  shape M9 needs for iOS, so settle it before writing M6's three backends. A third
  option — and now there is a fourth thing to know: **the iOS backend is the
  shape that fix would take.** `speech.rs` already speaks through
  `AVSpeechSynthesizer` in process on iOS, so the macOS sandbox case is that same
  backend behind a `cfg`, not a new design. A third
  option removes the question: the pre-rendered audio pack in
  `docs/research/ASR_TTS_CLAUDE_RESEARCH.md` §4.4 takes synthesis off the runtime
  path for the bundled curriculum and is the only one of the three that is
  *known* to be sandbox-safe.
- **An accepted dependency advisory.** Dependabot flags `glib` 0.18.5 (moderate,
  fixed in 0.20.0). It is Linux-GTK-only and absent from the macOS build graph,
  and it is not fixable from here because `gtk 0.18` pins `glib ^0.18` — cargo
  rejects the upgrade. `ROADMAP.md` records the detail. **Do not spend time on
  it**: if you want to confirm the scope, `cargo tree --target
  aarch64-apple-darwin -e normal | grep glib` returns nothing.

## 8. The app bundle, and the notices inside it

This section is what M5 added. Read it before changing anything under `licences/`,
`src-tauri/src/licences.rs` or `tauri.conf.json`'s `bundle` block.

### Building it

```bash
pnpm run build            # signed .app + .dmg (scripts/build-release.sh)
pnpm run build:unsigned   # the plain Tauri build, no signing wrapper
```

`scripts/build-release.sh` resolves a signing identity in this order:
`$APPLE_SIGNING_IDENTITY`, then the first *Developer ID Application* certificate
in the keychain, then ad-hoc (`-`) — which still launches on this machine, since
Apple Silicon refuses a completely unsigned binary. The identity is **not** in
`tauri.conf.json`, because that file is committed and one person's certificate
does not belong in it.

Output, under the project's own target directory (`.cargo-target/` here, because
of `with-cargo-env.sh`; `src-tauri/target/` otherwise):

```
bundle/macos/Hanzi Tutor.app
bundle/dmg/Hanzi Tutor_0.2.0_aarch64.dmg
```

### What is inside, and why nothing is downloaded

| Part | How it gets in | Size |
| --- | --- | --- |
| Characters, words, stroke geometry | `include_bytes!` in `src-tauri/src/state.rs` | ~13 MB |
| Interface font, Noto Sans SC | Vite, from `src/assets/fonts/`, via `src/app.css` | ~17 MB |
| SQLite, for the study store | compiled from the amalgamation by `libsqlite3-sys` | ~1.5 MB |
| Twelve licence notices, as text | `bundle.resources` → `Contents/Resources/licences/` | ~65 KB |

The app makes no network requests at all — there is no HTTP client anywhere in the
dependency graph — so "everything the reader needs" is a claim that has to hold at
build time, which is why the artifact and the font are committed rather than
fetched.

### The notices are pinned three ways

`src-tauri/src/licences.rs` is the catalogue. For each notice it holds the text
(`include_str!`, so it is compiled into the binary), the file under `licences/`,
and the path the bundle copies it to. `tauri.conf.json` copies the same files.
`src-tauri/tests/licences.rs` fails unless all three agree — in either direction,
so an uncatalogued file and a missing one are both failures. **Adding a notice
means editing the catalogue and the bundle config; the test is what stops you
forgetting the second one.** There are twelve since M10 added SQLite and
`rusqlite`: the app's first third-party *code*, since everything before it was
data or a font. Note what the test cannot do for you — it compares the catalogue,
the files and the bundle config with each other, so a dependency nobody
catalogued is invisible. Adding a crate means reading its licence by hand and
adding a notice.

Two copies is not redundancy for its own sake. The compiled-in copy is what the
About screen shows, and it cannot be lost in packaging — the roadmap's warning was
that notices are "easy to get wrong and only shows up in the packaged app". The
file copy is what a redistributor can read without launching the app, which is the
conventional form of the obligation.

### Verifying a build, by hand

There is no test for the packaged artefact, because there is no packaged artefact
in CI. After a build, check these four things:

```bash
APP=".cargo-target/release/bundle/macos/Hanzi Tutor.app"

# 1. Every notice survived as a file, and there are ten of them.
ls "$APP/Contents/Resources/licences"

# 2. The font made it into the frontend bundle.
ls "$APP/Contents/Resources/assets" | grep -i noto

# 3. It is signed, and by whom.
codesign --verify --deep --strict --verbose=2 "$APP"
codesign -dv --verbose=4 "$APP" 2>&1 | grep -E "Authority|TeamIdentifier"

# 4. It opens, and the About screen shows the notices.
open "$APP"
```

A failure of step 1 or 2 is the class of bug the tests cannot see, so it is worth
doing after any change to `bundle.resources`, the font path or `vite.config.ts`.
Step 4 also confirms the compiled-in notices reached the interface: the log line
`[webview] licences: 12 notices bundled` appears on stderr at startup, and the
fourth sidebar entry renders them.

### Releasing it

The version lives in **three files** — the workspace `version` in `Cargo.toml`,
`package.json` and `src-tauri/tauri.conf.json` —
and `tests/licences.rs::the_version_is_the_same_in_every_file_that_carries_one`
fails if any two disagree. Bump all three together: the About screen, the
bundle's `Info.plist` and the `.dmg` filename all read from them. `Cargo.lock`
follows on the next cargo run, and `pnpm test` is the check that the three still
agree.

Then build, tag and publish:

```bash
pnpm run build                       # the signed .app and .dmg
git tag v0.2.0
git push origin v0.2.0
gh release create v0.2.0 --prerelease --title "0.2.0 — alpha" \
  --notes-file <notes> \
  ".cargo-target/release/bundle/dmg/Hanzi Tutor_0.2.0_aarch64.dmg"
```

0.2.0 is both the first tagged release and an **alpha**, hence `--prerelease`. It
is **macOS-only on purpose**, and that is what to tell a tester who asks for a
phone build: the iOS shell runs on a physical iPhone from a *debug* build
installed with `devicectl`, because iOS *release* builds still fail to link
Tauri's Swift glue (see the traps above), so there is no IPA to attach until that
toolchain question is settled.

### Notarisation, if the app is to leave this machine

Signing is automatic; notarisation is not attempted, because it needs Apple
credentials and uploads the build. To do it, provide either `APPLE_ID`,
`APPLE_PASSWORD` (an app-specific password) and `APPLE_TEAM_ID`, or an App Store
Connect API key (`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`), and
let the bundler staple the ticket. Without it a signed `.dmg` copied to another
Mac needs a right-click-Open the first time — the standard Gatekeeper prompt for a
build Apple has not seen, not a defect.

### What is deliberately not in the bundle

- **Speech.** Pronunciation uses the system synthesiser. Apple's voices cannot be
  redistributed, so there is no lawful way to bundle one; the app disables the
  control with an explanation when no Chinese voice is installed. A current macOS
  ships several.
- **Any browser-opening capability.** The licences screen shows source addresses
  as text rather than links, because opening one would need the opener plugin and
  a new permission, and would contradict the app's "nothing leaves the machine"
  promise for no gain — the full licence texts are already bundled.

## 9. Tone practice (M11) — what to know before touching it

Tone practice records one utterance — a character or a whole word — and scores its
pitch contour against the tones asked for. It is **not** speech recognition:
nothing is transcribed, there is no model, and nothing is downloaded. ROADMAP M11
is the scope, M12 is the part that would need a model; §6 of
`docs/research/ASR_TTS_CLAUDE_RESEARCH.md` is the argument.

### Two modules, and the seam between them

| Module | What it owns | Why there |
| --- | --- | --- |
| `crates/hanzi-core/src/pinyin.rs` | Splitting a reading into syllables, reading a tone off a diacritic, and **tone sandhi** | It is reading rules, not signal processing. It is also the half M12 reuses whatever happens to the audio side |
| `crates/hanzi-core/src/tone.rs` | YIN, contours, syllable segmentation, DTW, scoring | Pure DSP: samples in, numbers out |

`tone.rs` knows nothing about pinyin and `pinyin.rs` knows nothing about audio.
The app joins them: `AppState::tone_target` builds a `ToneTarget` (characters,
readings, citation tones, spoken tones) and `AppState::score_tones` zips it against
a `ToneReport` (one judgement per syllable). **Both sides are one entry per
syllable in the same order** — that invariant is what lets the interface label each
chart, and `analyze` guarantees it by returning one entry per tone asked for,
whatever it heard.

### Words are the reason sandhi exists here

The dataset stores a **character's** reading as a list (`好` → `["hǎo", "hào"]`)
but a **word's** reading run together (`学习` → `"xuéxí"`). So a word needs the
reading taken apart before anything can be scored, and it needs the tones
*colloquially* rather than as a dictionary prints them: 你好 is `3 + 3` in a
dictionary and `2 + 3` out loud. Scoring the dictionary tones would mark correct
speech wrong.

Three rules are applied (`pinyin::spoken_tones`): third-before-third, 不 before a
fourth tone, and 一 before anything else. Both readings are reported, and the
interface shows "tone 2 (dictionary 3)" so a learner is never told their
dictionary is wrong. Rules 2 and 3 are about *which character* it is, not which
tone, which is why the function takes the characters as well as the tones.

**Do not "simplify" this by scoring character by character.** Sandhi is a
word-level phenomenon; a per-character loop cannot see it, and that was the whole
reason words were out of scope until now.

### The first thing to do: hear it with a real voice

The engine is tested against synthetic contours whose true F0 is known by
construction, which is the only way to test a pitch tracker — but **no human
recording has ever been through it**. The development environment's terminal has
no microphone permission, so every capture there comes back silent (§6). Until
someone speaks into the running app, the score constants are unvalidated.

Characters and words are **human-confirmed** — 不对 was the first word tried, and
it scored both tones with the sandhi explained. What is still untuned is the
*scoring constants*, which have never been fitted to a real voice, and
segmentation's behaviour on words that run together (see the limits below).

### A timing bug to not repeat

The first hand test found a display bug worth recording, because it is the kind
that makes a working result look broken. Syllable boundaries were reported as
offsets from the **start of the recording**, while `voiced_ms` is a *duration*.
Since the learner holds the button before speaking, the panel showed
`Split at 1463 ms` beside `Voiced 308 ms` — impossible-looking, though the split
was in the right place. **Anything measured in time here is measured from the
first voiced frame**, and the test
`a_word_never_scores_more_syllables_than_it_was_asked_for` asserts every boundary
falls inside the voiced span, so the two cannot drift apart again. If you add a
new timing field, give it the same baseline.

Two ways in. Through the app: hold **Hold to say it** on the practice screen and
release. Or from a terminal, which also prints the contour statistics:

```bash
./scripts/with-cargo-env.sh cargo test -p hanzi-tutor --lib -- --ignored --nocapture \
  records_from_the_real_microphone
```

That test opens the device, records 1.5 s and prints the peak level, the median
pitch and the verdict. **A peak level of `0.0000` means the terminal has no
microphone permission**, not that the code is broken — the app bundle is a
separate process with its own grant, so it can work there while this does not.
Say a syllable while it runs and check that the verdict is sensible.

### The four numbers that are judgement, not measurement

All in `crates/hanzi-core/src/tone.rs`, all named and documented, and all
calibrated against synthetic contours:

| Constant | What it is |
| --- | --- |
| `SCORE_DECAY_ST` | How fast the score falls off with shape distance. A wrong tone currently scores in the 20s–50s and a match in the 90s |
| `FLAT_ST` | Below this peak-to-peak span, in semitones, a contour is "level" |
| `DECIDE_MARGIN` | How much closer one tone must be before the difference is called real rather than "uncertain" |
| `FLAT_MATCH_SCORE` / `FLAT_OFF_TARGET_SCORE` | What a level contour scores, since a level tone is settled by a rule rather than by a distance |

Two more, for the word path:

| Constant | What it is |
| --- | --- |
| `VOICED_CUT_PENALTY` | What it costs to put a syllable boundary inside voiced speech. Larger than any plausible RMS, so a boundary prefers an unvoiced frame — which is where a consonant is, and where a listener hears the break |
| `MIN_SYLLABLE_FRAMES` | Shortest segment the splitter will make. Tied to `MIN_VOICED_FRAMES`, because a shorter segment could not hold enough voice to be judged |

Re-tune these only against real recordings, and say in the commit what they were
tuned against.

### Three decisions that look wrong and are deliberate

1. **The comparison is about shape, not height.** Each contour's mean is removed
   before comparison, so tone 1 (high level) and a level tone 3 are
   indistinguishable from one syllable — the speaker's register is not knowable.
   A flat contour is therefore *accepted* for both tones 1 and 3, with wording
   that says so. This under-claims on purpose. Without the mean removal there is a
   worse bug, which a test now pins: a falling contour scores closer to a
   **rising** template than to a level one, because time warping can slide a fall
   onto its own mirror while tone 2's shape is half as tall as tone 4's.
2. **Amplitude is not scored.** Contours are normalised to unit RMS before the
   distance is taken, so a shallow tone 4 scores as well as a deep one. The
   measured span is reported in `rangeSemitones` and the panel shows it. Scoring
   depth on one syllable flagged correct speech as wrong, which is the worse
   error.
3. **A level contour is not scored by distance at all.** Normalising a near-flat
   contour to unit RMS amplifies its own measurement noise into what looks like a
   large movement, so a *correct* level tone would score badly for having been
   measured imperfectly. It is settled by `FLAT_ST` and given a fixed score.

### The neutral tone, and a refusal that was in the wrong module

Neutral tone was refused for a long time, and the reasoning was sound as far as it
went: a neutral tone is short, and its pitch is set by the syllable before it, so
scoring it from one syllable looks like it needs context this module does not
have. What that missed is that "level" is still judgeable — and that the cost was
paid on 的, the most common character in the language, whose tone button could not
be pressed at all.

Three things were wrong, and only one of them was in `tone.rs`:

1. **The refusal lived in `pinyin.rs`.** `tone_target` returned `None` when no
   syllable carried a tone in `1..=4`. Whether a tone can be *judged* is a
   question about the analysis, and `pinyin.rs` knows nothing about that — it is
   the seam this file describes two sections up. The guard is gone: `pinyin.rs`
   builds a target from whatever tones the reading has, and `tone.rs` decides what
   it can say about them.
2. **`tone.rs` refused it in two more places.** `is_scorable` is new and is the
   one predicate both call sites use, so "which tones can be judged" has a single
   answer rather than `(1..=4)` written out four times.
3. **A neutral target has no template, and the code assumed every expected tone
   had one.** `score_contour`'s moving branch does
   `(1..=4).find(|t| *t == expected_tone).expect("the expected tone is one of the
   four")`, which tone 5 would have panicked on — unreachable only because of the
   guards in step 2. A moving contour against a neutral target is now answered by
   a rule instead: any clear movement is wrong for a neutral tone, so the tone it
   moved like is named and it is marked off-target, with no distance taken. Check
   the `is_scorable` guards before teaching this module a sixth tone.

What is judged is that it was level. What is **not** judged is how high or how
long it was: the mean is removed from every contour because register is
unknowable, and duration is not scored at all. That limit is said to the learner
every time (`NEUTRAL_LIMIT`) rather than left as a footnote here, because 85 for a
neutral tone means less than 85 for a rising one. If that ever needs to be more
than "level", duration is the thing to add — it is the part of a neutral tone a
listener actually hears — and it would need real recordings to calibrate, which
this module still has none of.

### Android capture: `cpal`'s AAudio input starts and then never calls back

**Android does not use `cpal` for capture, and this is why.** `capture.rs` uses
`cpal` on every platform but Android, where the microphone is Kotlin's
`AudioRecord` behind the platform bridge. The shape of the `cpal` failure is worth
knowing, because it looks exactly like a dead microphone and nothing reports an
error:

- `open_stream()` returns **`AAUDIO_OK`**, `request_start()` returns 0, and the
  stream reaches **`Started`** (state 4). It stays there.
- The data callback is **never invoked** — not once, in three seconds — so the
  recording comes back with **zero samples**, `span_ms` and `voiced_ms` are both
  0, and the learner is told "I could not hear enough voice to judge".
- The registration is real (cpal sets `.data_callback(...)` before
  `open_stream`), and cpal's error callback is registered too — and never fires.

The emulator's logcat is what settled it, since the phone's is unreadable (§6):

```
AAudioStreamBuilder_openStream() got Legacy, devIds = [8], perf = NO, burst = 768
AAudioStreamBuilder_openStream() returns 0 = AAUDIO_OK for s#1
AAudioStream_requestStart(s#1) called
AAudioStream: setState(s#1) from 3 to 4       ← Starting → Started
… three seconds, nothing …
AAudioStream: setState(s#1) from 4 to 11      ← closed on stop
```

Setting a fixed callback size — which AAudio needs for input and cpal leaves
unset, on advice that is about *output* latency — **does not fix it**, and that
was tried first.

**What replaced it, and the evidence.** `AudioRecord` was probed through the
Kotlin bridge on the same two devices and delivered the right number of samples on
both, 19,200 for 1.2 s at 16 kHz, across five sources:

| Source | Emulator peak | Phone peak |
| --- | --- | --- |
| `DEFAULT` | 8 | 647 |
| `MIC` | 32768 | 510 |
| `VOICE_RECOGNITION` | 32768 | **2050** |
| `UNPROCESSED` | 32768 | 219 |
| `CAMCORDER` | 8 | 581 |

`VOICE_RECOGNITION` is the source used: it is the one tuned for speech, and on
the phone it is ten times the level of `UNPROCESSED`, which is the theoretically
purer choice for pitch and too quiet here to be one in practice. `MIC` is the
fallback, for devices that will not admit to the first.

The samples do not cross the bridge as JSON — ten seconds of 16 kHz mono is
320 kB. Kotlin writes them to `cacheDir/tone-recording.pcm` and Rust reads that
file and **deletes it**, because an app that stores only what it has to should not
leave a learner's voice in the cache. Verified end to end on the phone: a 2 s hold
produces a 64,000-byte file, 85% of its samples non-zero.

Three things about this backend are easy to get wrong again:

- **The platform is the authority on whether a recording is live.** Kotlin's
  audio thread ends by itself at its cap or on an error, and Rust was not told —
  so its own `active` flag went stale and every later press failed with "Already
  listening." for the life of the process, reachable by holding the button past
  the cap once. `Recorder::start` on Android now asks `recordStatus` first and
  drops a stale flag rather than trusting it.
- **Nothing blocks the main thread.** The commands run on it (§6), so `recordStop`
  does not wait for the audio thread: it clears the flag and the thread answers
  when the file is closed, which is also the only moment Rust can safely read it.
- **The rate is 16 kHz**, which is `tone::TARGET_SAMPLE_RATE` — so Android skips
  the resampling the `cpal` path needs rather than adding a step.

### The push-to-talk button, and the gesture that cancelled it

Three separate things made holding the button stop the recording, and only the
first was obvious. All three are fixed; the second and third are the ones to
remember, because both look like a broken microphone.

1. **The button moved out from under the finger.** The label narrows to
   "Listening…" on press, and `startListening` cleared the previous judgement,
   which removed the tone panel and pulled every control below it upwards. Either
   one fires `pointerleave`, and `pointerleave` was wired to `stopListening`.
   Fixed with `setPointerCapture`, a `min-width` so the label cannot reflow the
   row, and by leaving the previous judgement on screen while listening.
2. **`touch-action`.** A finger held on a button inside a scrolling page is a
   gesture the browser wants: it takes the pointer for a scroll and sends
   `pointercancel`. `touch-action: none` on the button settles that.
3. **The long press.** Android's text-selection gesture takes the pointer at
   about half a second — **measured at 555 ms** — and sends `pointercancel`. This
   is the one that survived every other fix, and it is why the practice board has
   carried `user-select: none` and a `contextmenu` guard all along. The button
   needed the same. It also explains a stray text-selection popup that appeared
   over the app in a screenshot, which was the same gesture showing its face.

Instrument the events rather than guessing here: listeners on the button for
`pointerdown`/`pointerup`/`pointercancel`/`lostpointercapture` with timestamps
say in one run what took three rebuilds to infer.

### Things that will surprise you

- **`cpal::Stream` is not `Send` on every backend**, so it cannot live in Tauri's
  shared state. `capture.rs` owns one thread per recording and only plain data
  crosses back. Do not "simplify" this by storing the stream in `AppState`.
- **The microphone is opened and closed per utterance**, deliberately. It costs
  tens of milliseconds and it means the system's recording indicator is lit only
  while the learner is holding the button. A latency complaint and a privacy
  property are the same line of code here.
- **`Recording` must stay `Debug`**, because `Recorder::stop()`'s error path uses
  `unwrap_err()`. That only fails in the `--lib` test target, so `cargo check`
  will not catch its removal.
- **A neutral-tone syllable is carried but never scored.** It appears in the
  target and in the result, and `ToneReport::finish` excludes it from the
  aggregate so that 妈妈 is still judged on 妈. A target where *nothing* is
  scorable (的 on its own) is refused entirely, which is what disables the button.
- **Segmentation will cut where you did not mean it to.** Two syllables that run
  together with no consonant between them — a vowel-initial second syllable — have
  no unvoiced frame to cut at, and the search falls back to the quietest frame,
  which may be wrong. This is why `boundariesMs` is reported to the interface: it
  is the difference between a learner seeing a puzzling score and seeing that the
  app mis-heard where the syllables were. Anything that improves this should
  improve it *here*, and the tests to extend are the `say_word` ones.
- **A `ToneResult` must always carry one entry per syllable of the target.** The
  interface zips them positionally against the characters and readings. `analyze`
  guarantees the length; if a change ever breaks that, the IPC test
  `a_tone_result_carries_one_entry_per_syllable` is what should catch it.
- **The verdict words live in Rust, not in the panel.** `TonePanel.svelte` styles
  `detail`; it must not reword it, or the same judgement will be expressed in two
  places that drift.
