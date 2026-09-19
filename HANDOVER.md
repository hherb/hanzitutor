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
pnpm test                        # expect 196 passed, 0 failed
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
| Pronunciation | Done on macOS | 9 tests; human-confirmed speaking |
| Personal vocabulary list | Done | 27 store unit tests; persistence tested through the state layer |
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
# [speech] using voice Tingting (Chinese (China mainland)) (zh_CN)
# [webview] course loaded: 7744 characters in 775 lessons
# [webview] vocabulary: 2 entries in 2 groups
# [webview] progress: 4 practised, 3 due
# [webview] review queue: 2 due, 2 in this session
# [webview] drawable characters: 9574
# [webview] speech: using Tingting (Chinese (China mainland)) (zh_CN)
# [webview] licences: 10 notices bundled
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
  src/vocab.rs              the personal vocabulary list and its JSON file
  src/progress.rs           per-character cards, SM-2 scheduling, review queue
  src/time.rs               ISO-8601 formatting, parsing, date arithmetic
  src/bin/prepare_data.rs   upstream data -> compact artifact (feature = "prepare")
  examples/selfcheck.rs     whole-dataset measurement and tolerance tuning
src-tauri/
  src/commands.rs           the IPC surface; thin wrappers over AppState methods
  src/state.rs              embedded dataset, speech warm-up, the three stores
  src/licences.rs           the catalogue of notices that ship; see §8
  src/speech.rs             macOS `say` backend, voice selection
  tests/ipc_contract.rs     locks the JSON contract the UI reads
  tests/licences.rs         pins the notices, the version and the bundle config
src/
  App.svelte                shell: modes, navigation, keyboard, state ownership
  lib/PracticeCanvas.svelte pointer capture, coalesced sampling, display space
  lib/CharacterThumb.svelte one small picture of one character's attempt
  lib/render.ts             canvas painting, font<->display transforms, colours
  lib/FeedbackPanel.svelte  report -> readable advice
  lib/WordsPanel.svelte     the HSK word list: search, browse, practise
  lib/LicencesPanel.svelte  About and licences: the notices, with their texts
  lib/LessonSidebar.svelte  course, list and word navigation, progress marks
  lib/types.ts              TS mirror of the Rust structs
  lib/api.ts                typed invoke wrappers
  assets/fonts/             Noto Sans SC, the bundled interface face (OFL)
licences/                   every notice text that ships, plus README.md
scripts/                    fetch-data, with-cargo-env, tauri-cli, build-release
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

## 5. The verification loop

Run before every commit:

```bash
pnpm test           # 196 tests: engine + store + data pipeline units, IPC contract, speech, notices
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

## 7. Open decisions

- **Distribution** — settled by M5. The notices ship inside the bundle and are
  surfaced from the About and licences screen, with the whole arrangement
  described in §8 and the reasoning in `ROADMAP.md` M5. What is deliberately left
  open is **notarisation**, which needs Apple credentials and uploads the build;
  it is an operator step, documented in `README.md`, not a gap in the app.
- **Committing the artifact** — settled the other way, deliberately. The 13 MB
  artifact and the 17 MB interface font are both committed, so a clone and a CI
  run go straight from `pnpm install` to a build with no download. The cost is
  ~30 MB of binary in the repository; the 33 MB of upstream text stays ignored.
  `.gitignore` records the reversal and the command that regenerates the artifact.
- **The ink measure is proven, but half of it cannot fire yet.** The canvas paints
  every stroke at one fixed width, so nothing a learner does on a trackpad can put
  down *less* ink than `INK_WIDTH` and the `faint` verdict is unreachable in daily
  use; what M4 does catch today is overshoot and short strokes. The width half
  becomes live when input can report a real pen width — a stylus, or the
  velocity-thickened brush M8 calls "cosmetic only", which stops being true now
  that this measure exists. Whichever comes first wants real attempts to re-tune
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
bundle/dmg/Hanzi Tutor_0.1.0_aarch64.dmg
```

### What is inside, and why nothing is downloaded

| Part | How it gets in | Size |
| --- | --- | --- |
| Characters, words, stroke geometry | `include_bytes!` in `src-tauri/src/state.rs` | ~13 MB |
| Interface font, Noto Sans SC | Vite, from `src/assets/fonts/`, via `src/app.css` | ~17 MB |
| Ten licence notices, as text | `bundle.resources` → `Contents/Resources/licences/` | ~60 KB |

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
forgetting the second one.**

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
`[webview] licences: 10 notices bundled` appears on stderr at startup, and the
fourth sidebar entry renders them.

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
