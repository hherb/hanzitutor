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

# 2. Deps.
pnpm install

# 3. The one build input that is not committed: the pinned sherpa-onnx native
#    library (~20 MB, SHA-256 checked, into .sherpa-onnx/). Without it the build
#    fetches an unpinned archive of its own — see invariant 26.
pnpm run fetch-sherpa

# 4. Confirm the baseline is green. There is no *data* step: the dataset
#    artifact, the interface font and the licence texts are all committed, so a
#    clone builds without downloading anything else.
pnpm test                        # expect 491 passed, 0 failed, 4 ignored
pnpm run check:rust && pnpm run check:web
```

Then `pnpm run dev` to launch it. `pnpm run build` makes a **signed** `.app` and
`.dmg` — see §8 for what that involves and what ends up inside.

**If the app keeps asking for your login password, use `pnpm run dev:signed`.**
The Dropbox sign-in lives in the login keychain, and macOS grants a keychain item
to a specific program by its designated requirement. An unsigned dev build's
requirement is its code hash, which changes on every rebuild, so the keychain
treats each build as a stranger and prompts. `pnpm run dev:signed` builds the dev
binary, signs it with a stable identity (`scripts/sign-dev-binary.sh`, which picks
an Apple Development certificate and uses the app's own identifier), and then runs
`tauri dev`. It will prompt **once** to let the new signature at the existing item,
and not again — until a Rust change relinks the binary, when it needs signing
again. That is dev-only: no hardened runtime, no timestamp, no notarisation.

`./scripts/fetch-data.sh` is only wanted when you are changing the data pipeline:
it re-downloads the ~33 MB of upstream text into gitignored `data/raw/`, restores
any deleted licence text or font, and `pnpm run prepare-data` then rebuilds the
artifact. Nothing in the normal build path needs either. The phrase-audio
scripts (`fetch-phrases.sh`, `fetch-tts.sh`, `synthesize-audio`) are build-host
tools for M14, not part of an app build — see ROADMAP M14.

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

The wrapper also points `SHERPA_ONNX_LIB_DIR` at the fetched library — but only
when it is really there, because handing the crate a missing path gives a *worse*
error than the download it exists to prevent (§4, invariant 26). Which copy it
picks depends on the arguments: iOS takes `.sherpa-onnx/current-ios/lib`,
Android takes the archive directory, everything else the macOS static libraries.

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
- Running the binary directly produces harmless WebKit noise
  (`could not create directory ~/Library/WebKit/...`) because the sandbox blocks
  WebKit's cache directories. It is not an app fault.

## 2. What already works

| Area | State |
| --- | --- |
| Grading engine | Done — shape, placement, ink (M4) and order, a quarter of the score each; `selfcheck` scores every teachable character a perfect 100 |
| Dataset pipeline | Done — 9,574 characters with stroke geometry, 7,744 in a frequency-ordered course of 775 lessons, 9,443 HSK 3.0 words, in one committed 13 MB artifact |
| Drawing canvas | Done, human-confirmed — trace and recall modes, drag and click-to-draw, colour-coded verdicts |
| Board control row | Done, checked as a render — three cards (writing help, pronunciation, drawing) centred under the board, one row at 390 px and at 1280 px |
| Stroke-order animation (M7) | Done — the pen sweeps each centre-line with the outline revealed behind it |
| Personal vocabulary list (M1) | Done — groups, drilling, JSON and CSV export, JSON import (merge or replace) |
| Progress and SRS (M2) | Done — SM-2 behind a `Scheduler` trait; the review queue is drawn from the course and the list together |
| Words (M3) | Done — search by character, reading or meaning; a word is read and spoken whole (着急 is `zháojí` where the isolated 着 has no context) |
| Settings screen | Done — four preferences; an unchosen one is resolved from the device or the system |
| Durable study store (M10) | Done — one `hanzi.db`; the old JSON imported once and left byte-identical; an unbounded attempt log |
| Tone practice (M11) | Done, human-confirmed for characters and words — pitch contour against the expected shape, with no model |
| Speech recognition (M12) | Done — an optional ~163 MB SenseVoice model, installed from the settings screen |
| Cross-device sync (M13) | Done — an optional Dropbox account, off by default, merging the attempt log (§7) |
| Mobile shells (M9) | Runs on a physical iPhone and on Android hardware; the iOS release build and the Play paperwork are outstanding (ROADMAP M9) |
| Graded phrase audio (M14) | In progress — the HSK 1–2 clips are the corpus's own recordings, and the Phrases screen and optional synthesis model are built; the graded readers' voice is still open (ROADMAP M14) |
| Pronunciation | The system synthesiser on macOS, iOS and Android; the bundled clips cover the graded phrases on any platform |
| Network | Off unless the learner asks: two optional model downloads (recognition, synthesis) and an optional Dropbox sync |
| Licence notices | Done — 21 notices over 19 bundled files, pinned three ways (§8) |

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
# [webview] licences: 21 notices bundled
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
lines are from the M4 run (a fresh data directory, hence the different counts);
they are the current format, and the two `graded` lines are the same trace with
the pen declared first at full width and then at a third of it.

`drawable characters` is the word-list counterpart of the course load: 9,574
characters have stroke geometry against 7,744 with a frequency rank. The
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
  src/settings.rs           Settings, Pace, BoardSize, SettingsSink, and the
                            tri-state (None = nobody has chosen) — see §7
  src/progress.rs           per-character cards, SM-2 scheduling, the review
                            queue, and the fold that rebuilds a card from a log
  src/tone.rs               YIN pitch tracking, tone contours, syllable
                            segmentation, DTW scoring. Pure DSP: samples in,
                            numbers out (see §9)
  src/pinyin.rs             splitting a reading into syllables, reading a tone off
                            a diacritic, tone sandhi, and the heard-against
                            comparison (see §9)
  src/time.rs               ISO-8601 formatting, parsing, date arithmetic
  src/bin/prepare_data.rs   upstream data -> compact artifact (feature = "prepare")
  examples/selfcheck.rs     whole-dataset measurement and tolerance tuning
crates/hanzi-say/           speech synthesis, wrapping sherpa-onnx's OfflineTts.
                            The build host's half: samples and WAVs, no app state.
  src/lib.rs                the model, normalisation, the silence guard, chunking
  src/sentences.rs          reading the two graded corpora into one Phrase shape
  src/phonetics.rs          splitting a syllable into initial, final and tone, so
                            a mispronunciation can be told from a spelling
                            difference a recogniser prefers
  src/judge.rs              the judgement half of `verify-audio`, kept apart from
                            the recogniser so it is testable without a model
  src/bin/synthesize_audio.rs   corpus -> MP3 clips + manifest.json. Resumable;
                            the only place that needs ffmpeg. See §3a
  src/bin/verify_audio.rs   does a clip say what its text says? It drives a
                            downloaded recogniser, so it is run by hand
                            (ROADMAP M14; docs/research/MELOTTS_*.md)
crates/hanzi-store/         the study database: SQLite, and nothing else.
                            Separate from the engine so the engine keeps no
                            native dependency
  src/schema.rs             the tables, and applying them. Bump SCHEMA_VERSION
                            with any change; the guard reads it *before* apply()
  src/migrate.rs            the once-only, per-document import of the old JSON
  src/lib.rs                ProgressSink / VocabSink / CursorSink, and the
                            attempt log's reader (attempt_count, attempts)
  src/export.rs             the log as JSON Lines or CSV — pure functions over rows
  src/analyse.rs            what the log implies about the grader: percentiles,
                            the mass near each bar, pinned measures, pass/fail
                            separation, and the score-vs-measures check
  examples/analyse-attempts.rs   `pnpm run analyse-attempts` prints a report for a
                            study directory. Reads only; writes nothing
  tests/store.rs            the M10 acceptance criteria, from real saved files
crates/hanzi-sync/          the sync format, the merge and the fold — no account
                            and no network in the merge itself
  src/document.rs           shard naming and the merge rules, per document
  src/local.rs              the adapter: publish, pull, recompute
  src/http.rs, oauth.rs, dropbox.rs   the hand-written Dropbox client over `ureq`
  src/reach.rs              the network probe a launch sync makes first
  tests/                    convergence, two-device and Dropbox tests
src-tauri/
  src/commands.rs           the IPC surface; thin wrappers over AppState methods
  src/state.rs              embedded dataset, speech warm-up, the stores
  src/asr.rs                the recognition model: manifest, download, recogniser.
                            One of the only modules that opens a socket
  src/say.rs                the synthesis model, on asr.rs's rules exactly
  src/sync.rs               the sync account and the platform secret store
  src/licences.rs           the catalogue of notices that ship; see §8
  src/platform.rs           the mobile plugin bridge (Android insets, back,
                            keystore) and the Kotlin/AVFoundation seams
  src/capture.rs            microphone capture: cpal everywhere but Android
  src/speech.rs             the platform synthesiser: macOS `say`, iOS
                            AVSpeechSynthesizer, Android TextToSpeech
  tests/ipc_contract.rs     locks the JSON contract the UI reads
  tests/licences.rs         pins the notices, the version and the bundle config
  Info.plist                NSMicrophoneUsageDescription, merged over the
                            generated plist at build time
  Info.ios.plist            the scene manifest, plus the same microphone string
  Entitlements.plist        com.apple.security.device.audio-input. Without it a
                            hardened-runtime build captures silence; see §6
src/
  App.svelte                shell: modes, navigation, keyboard, state ownership,
                            the practice queue and the stroke-order animation clock
  lib/PracticeCanvas.svelte pointer capture, coalesced sampling, display space,
                            drag and click-to-draw input modes
  lib/CharacterThumb.svelte one small picture of one character's attempt
  lib/render.ts             canvas painting, the stroke-order sweep,
                            font<->display transforms, colours
  lib/FeedbackPanel.svelte  report -> readable advice
  lib/TonePanel.svelte      the learner's pitch contour drawn over the expected
                            tone shape, with the verdict Rust worded
  lib/WordsPanel.svelte     the HSK word list: search, browse, practise
  lib/VocabularyPanel.svelte the personal list: groups, entries, import/export
  lib/PhrasesPanel.svelte   graded phrases with their bundled clips
  lib/audio.ts              clip playback, rate, and on-device samples
  lib/LicencesPanel.svelte  About and licences: the notices, with their texts
  lib/SettingsPanel.svelte  the four preferences, plus the two optional model
                            downloads; each change is written at once
  lib/LessonSidebar.svelte  course, list, word and phrase navigation, progress marks
  lib/Icon.svelte           the board's control glyphs
  lib/types.ts              TS mirror of the Rust structs
  lib/api.ts                typed invoke wrappers
  assets/fonts/             Noto Sans SC, the bundled interface face (OFL)
public/audio/<corpus>/      committed phrase clips + manifest.json; see §8
licences/                   every notice text that ships, plus README.md
docs/research/              research notes behind the open decisions
docs/privacy-policy.md      the privacy policy Play requires (TODOs remain)
store/                      the Play listing's copy and artwork
scripts/                    fetch-data, fetch-sherpa, fetch-phrases, fetch-tts,
                            with-cargo-env, tauri-cli, build-release,
                            probe-app-sandbox, check-audio.py, cosyvoice-say.py
.github/workflows/ci.yml    test + clippy + svelte-check on push
```

**The seam to preserve:** `hanzi-core` must stay free of Tauri and platform
dependencies. That is what makes the engine testable without a window, and what
will let it run behind a mobile shell or a CLI unchanged. Anything that needs the
window goes in `src-tauri`; anything that is *logic* goes in `hanzi-core`. The
same rule put `hanzi-store` and `hanzi-sync` in crates of their own.

## 3a. Working conventions

Two rules about long jobs, learned the hard way while generating the HSK phrase
audio (M14). Both are about not wasting the operator's time.

### Long runs are given to the operator, not run here

**Anything expected to take more than about ten minutes is printed, not started.**
Give the command, say what it will do, and let the operator run it in their own
terminal where they can watch the progress, stop it, and see it fail.

The reason is not politeness. A long job started in the background is invisible
while it runs, so the operator cannot tell a slow job from a hung one, cannot see
a warning scroll past, and cannot interrupt it when something looks wrong. In the
phrase-audio work this mattered concretely: the run printed *"synthesis text …
too short than prompt text … this may lead to bad performance"* on nearly every
phrase — a real voice-consistency risk — and it was buried in a log nobody was
watching until the end.

Short jobs, and anything a test or a check needs, still run directly. This is
about *long* work: corpus synthesis, model downloads, whole-suite runs.

### Anything over ten minutes checkpoints every item

A pass over many items must be **resumable**, saving each item's result as it is
produced, so that an interruption costs the work in flight rather than the whole
run.

The phrase-audio run is the cautionary example: its first revision wrote a single
manifest at the very end, so stopping it after 421 of 1,638 items would have
discarded every one of them. Hours of compute, thrown away by a `Ctrl-C` — and
the same is true of a crash, a laptop sleeping, or a sandbox being torn down.

The shape to use:

* write each item's output as soon as it exists, under the final name;
* record it in a **progress file** written after each item, so a re-run can skip
  what is already done and verified;
* make skipping safe by checking the artifact, not just the bookkeeping — a
  truncated file must not be mistaken for a finished one;
* summarise only at the end, and say what was skipped.

`crates/hanzi-say/src/bin/synthesize_audio.rs` (which checks the clip's size, not
just its name) and `scripts/cosyvoice-say.py` (`progress.json`, renamed into place
after each item) both do this now. The shape is the rule, not either file.

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
    (92 before M4, 85 after); it is a judgement, not a measurement, and the attempt
    log now carries the measures to check it against — `pnpm run analyse-attempts`.

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

    **A notice can be owed for code that is compiled in and never called, and
    whether it is there can differ by platform.** `espeak-ng` (GPL-3.0-or-later) is
    inside the prebuilt `sherpa-onnx` library that the Android and iOS builds link,
    so those artifacts redistribute it even though this app only ever *recognises*
    speech and never synthesises it; macOS links the same archive's components
    individually and the linker drops what nothing references. An early `nm` and
    `strings` check on macOS therefore concluded, correctly, that the desktop
    binary does not contain it — and that conclusion was then written down as a
    claim about the app, which the mobile builds make false. So: anything that
    arrives inside a prebuilt mobile binary has to be checked **in that binary** —
    the `.so` for each ABI, the iOS framework slice — and a licences test, not a
    one-off check, is what keeps the answer current. GPLv3 §6 conditions *conveying*
    the object code rather than using it, so "we never call it" is not an answer.

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

26. **The speech engine's native library is pinned by digest, and the build must
    not fetch it.** `sherpa-onnx-sys` downloads a prebuilt archive from GitHub
    during `cargo build` unless `SHERPA_ONNX_LIB_DIR` is set. Left alone, a build
    acquires an unreviewed binary over the network, which is the opposite of what
    the rest of this repository does with its inputs. So
    `scripts/fetch-sherpa.sh` fetches it against a **recorded SHA-256** and
    **refuses** a platform whose digest has not been recorded, and
    `scripts/with-cargo-env.sh` points `SHERPA_ONNX_LIB_DIR` at the result. Do not
    "simplify" that away, and do not let a fallback to the crate's own download
    creep in — a build that silently tolerates an unpinned binary is the failure.
    CI runs the fetch as its own step, which is why the env var is only set when
    the unpacked library is really there: set to a missing path the crate fails
    with a *worse* error than the one it gives when it simply fetches.
27. **Tone practice never depends on the speech model, and the transcript is never
    a pronunciation score.** Two rules that protect the same thing from opposite
    sides. `asr.rs` holds the only code in the app that opens a socket, and it runs
    only because somebody pressed the install button; with no model installed
    `Asr::recognize` returns `Ok(None)`, the panel draws exactly what it always
    drew, and nothing prompts. That is not a fallback to be tidied up — it is the
    state the app ships in. And a recogniser's language model is built to repair
    the very errors a learner makes, so its output must never be presented as
    praise: compare transcripts as **readings with the tone stripped**
    (`pinyin::heard_against`), never as characters, never with tone marks, and
    never with a hotwords file pointed at the expected answer. The tone comes from
    the pitch contour and from nowhere else.
28. **A control's accessible name contains the words printed on it.** Every button
    on the board's control row carries a visible name — Hint, Strokes, Listen,
    (hold…), Undo, Clear, Back, Show target, Check/Next — and WCAG 2.5.3 is the
    reason `aria-label` reads `"Strokes: watch the character written one stroke at
    a time"` rather than the sentence it used to be: a voice-control user has to be
    able to say what they can see, and an `aria-label` that replaced the visible
    word broke exactly that. Where the visible text is enough on its own, there is
    **no** `aria-label` at all and the element's own content is the name — which is
    why the Show target switch deliberately has none. The comparison is on the
    words, not on the punctuation: the microphone is labelled `(hold…)` and its
    name is "Hold to speak …", which is the word a user reading that label would
    say. The long sentence did not go away; it moved to `title`, which is a
    description once a name exists, and the reason a disabled control cannot be
    used still rides on the `.hint` under the row.
29. **An attempt's measures are written all together or not at all, and `NULL` is
    not `0.0`.** Schema 5 stores what an attempt was graded from — `shape`,
    `position`, `ink`, `ink_coverage`, `order_score`, `legible`, `order_correct` —
    beside its headline score. Every one of those columns is nullable, and that is
    load-bearing: a row written before schema 5, and a row merged in from a peer,
    has only the score and the time, because `hanzi-sync`'s shard format carries
    only those. `NULL` means "not measured"; `0.0` means "measured, and wrong". A
    reader that conflates them turns every unmeasured attempt into a perfectly bad
    one, and `hanzi_store::analyse` would report a distribution it had invented.
    The same rule governs the export: a row with no measures is written out with
    empty measure fields rather than dropped, because "unmeasured" is not
    "worthless". If measures are ever to travel, the shard format needs a version
    first — a closed shard is never rewritten, so old shards will always be
    measureless and must keep parsing.
30. **A vocabulary position belongs to one group, and is settled only against that
    group.** The row holds the entry's **uuid** — never its local id, which names
    a different word on every device — and the store is the only place that
    translates between the two. `merge_vocab_cursors` compares stamps *within* a
    group and never across groups: a position means nothing except against the
    list it was taken in, so a later stamp in one group must not move another.
    The row is keyed by the group's **name**, because a group has nothing else to
    be named by — a stable id would have to be minted per device, and already-
    synced devices would then fork a shared group and its cursor would never
    merge. A rename therefore *moves* the row (`vocab_rename_group` does it in the
    same call) and a deletion drops it; do not "fix" the name key without solving
    that fork first.

## 5. The verification loop

Run before every commit:

```bash
pnpm test           # 515 tests: engine + data-pipeline units, the SQLite store,
                    # sync convergence, IPC contract, speech, notices, data-dir flag
pnpm run check:rust # clippy with -D warnings
pnpm run check:web  # svelte-check
```

These three are exactly what `.github/workflows/ci.yml` runs on every push and
pull request, on a macOS runner, with no data step — plus `pnpm run fetch-sherpa`,
because that is the only input cargo does not obtain for itself.

`pnpm test` enables hanzi-core's `prepare` feature deliberately, so the data
pipeline's parsing — which upstream fields are trusted and how a word's reading is
chosen — is covered by the same run. Without the feature flag the nine
`prepare-data` tests silently do not run.

**If you touched the grading path**, also run `pnpm run selfcheck`:

```bash
pnpm run selfcheck
```

It reports self-consistency, tolerance under jitter, shape-metric
discrimination, robustness to how the pointer sampled the stroke, what the ink
measure can see, and the cost of a grade. Treat a regression in its output as a
failing test. The checks to read first, and what "healthy" looks like:

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

The `0` on the verdict line is a hard invariant, not a statistic: if sampling
alone changes a verdict, placement is being measured from the sample mean again
and the number explodes into the thousands. The ink lines are the M4 tripwires.
`a correct trace below the ink bar` and `characters with a Faint stroke` must both
be **0**, because a correct trace is normalised to score exactly 1.000 on ink by
construction — the moment that stops being true, "perfect" is unreachable for some
character. `third-width pen ... below 0.6: 7744` is the measure working. The
tolerance rows `sigma=15` and `sigma=30` must stay at 100% and 99.1%; if they
collapse, the ink measure has started double-counting placement (which is what IoU
did) rather than measuring ink.

**Unless you changed grading on purpose, `selfcheck` must come out
byte-identical before and after your change.** That is the tripwire for everything
else: M2's scheduling, M3's new artifact payload and M7's canvas-only animation
were all checked that way. The figures above are the ones M4 was allowed to move —
the tolerance table's mean score drops a few points as shape, placement and order
lost weight to ink, and `sigma=50` legibility went 53.2% → 52.4% because the ink
bar now fails 8% of very rough attempts; `sigma=15` and `sigma=30` are unchanged.
If you touch the rasteriser or the weights, re-read that table rather than
assuming.

**`selfcheck` is the synthetic half; `pnpm run analyse-attempts` is the real half.**

```bash
pnpm run analyse-attempts            # the app's own data directory
pnpm run analyse-attempts -- DIR     # or a copy of one
```

It reads a study database and prints, for the attempts that carry measures, the
percentiles of each measure, the share of attempts within ten points of each bar,
which measures are pinned at 1.0, whether a measure's mean separates passes from
failures, and how far the recorded score is from the four weights applied to the
recorded measures. That last figure must be near zero: if it is not, the stored
measures are not the ones that produced the stored score and nothing else in the
report can be trusted. **It only reports** — it cannot know whether an attempt was
*right* — so read it as "where is the bar deciding" and "which measure is not
earning its quarter", not as an accuracy. **Run it against a copy, not the live
directory**, and note that opening a database migrates it: analysing the real
one applies schema 5 to it.

**If you touched the store**, `crates/hanzi-store/tests/store.rs` is the suite that
matters: it starts from documents written by the app's own JSON stores rather than
from fixtures, and covers the import, the markers, the untouched bytes, the log
past the 20 a card shows, WAL, and an uncommitted write.
`src-tauri/tests/ipc_contract.rs` covers the same ground through `AppState`, where
the `Persisted` rules live. Check it **on the built binary** too, because the data
directory is the part a unit test cannot see:

```bash
./.cargo-target/debug/hanzi-tutor --user-dir "$PWD/.tmp-db"   # then look in .tmp-db
sqlite3 .tmp-db/hanzi.db "select key, value from meta; select ch, attempts from progress_card;"
```

A plain `cargo build` binary does not render (see §6) — for a run you can *look
at*, use `pnpm run dev`.

**If you touched the scheduler**, the numbers to hold still are in `progress.rs`'s
tests, which pin every interval and due date outright: a failure is due in 60 s,
and passes at 12 h / 1 d / 2 d rising to 1 d / 6 d / `interval × ease`, capped at a
year. They will tell you which end of the policy you moved.

Then exercise the app and read its logs (§2). `graded …` proves a real attempt went
through; `progress …` follows it when the attempt was recorded, and `review queue`
says what came due — but **drawing cannot be automated here** (§6), so those two
lines need a human at the trackpad. Everything downstream of them is covered by
the IPC tests, which drive `AppState::load` → record → save → reload against real
temp files.

## 6. Traps that cost time here

- **Look, don't try to act.** `screencapture` works (window-targeted; recipe in
  §1). Acting does not: AppleScript window inspection and `System Events`
  keystrokes fail with `A privilege violation occurred`, needing Accessibility
  rather than Screen Recording. Anything that clicks, types or scrolls needs the
  human.
- **You can screenshot a graded panel without drawing.** `pnpm run dev` serves the
  frontend over HMR, so a temporary two-line seed in `loadCharacter` (grade
  `next.medians` and call `check()`) renders a real report for a capture, and
  reverting is instant. **M7's stroke-order animation needs the same trick** and is
  otherwise unreachable: `setTimeout(() => void playStrokeOrder(), 1500)` in
  `loadCharacter`, then capture every ~0.3 s — a capture runs slower than the
  animation, so expect a few frames per stroke. Log a line per stroke
  (`TEMP play stroke i/n`) to see that stop and navigate-away really end the loop.
  Take the seed out before committing.
- **A phone-width layout can be checked without a phone, a Tauri window, or
  Screen Recording.** `invoke` is the whole boundary `src/lib/api.ts` crosses, so a
  scratch HTML entry defining `window.__TAURI_INTERNALS__.invoke` with canned
  answers mounts the real `App.svelte` and CSS in a plain browser
  (`./node_modules/.bin/vite --port 1420 --strictPort`). Take the character's
  `outlines` and `medians` from `data/raw/graphics.txt` (`medians` are font-space,
  so flip with `900 - y`) rather than committing a fixture: `data/raw/` is
  gitignored, and a tracked copy of LGPL geometry is a second source of truth. Drive
  headless Chrome over the DevTools Protocol (`--remote-debugging-port`,
  `Emulation.setDeviceMetricsOverride`, `Page.captureScreenshot`) to capture at a
  chosen device pixel ratio — `--screenshot` cannot set one, and the ratio is the
  whole question when the check is "does a row of tools fit a 390 px phone at 3x".
  Two things about that Chrome: the DevTools socket **rejects the `Origin` header
  Node's `WebSocket` sends**, so drive it from Python (`websocket-client` with
  `suppress_origin=True`); and it needs **`--no-sandbox`**, because crashpad cannot
  write `~/Library/Application Support/Google/Chrome/Crashpad` here and the browser
  dies with `Trace/BPT trap: 5`, which reads as "headless Chrome does not work"
  rather than as a permission problem. Attaching straight to the page target
  (`/json/list`) is steadier than creating a target and attaching a session.
- **A page cannot change the phone's keyboard.** Entering a word means three
  scripts in three fields, and the keyboard cannot be made to follow the field. No
  web API switches an input method's language: `lang` is a hint, and Chromium's
  Android WebView does not even pass it on (`ImeUtils.computeEditorInfo` fills in
  `inputType`, the autocorrect flags and `imeOptions`, never
  `EditorInfo.hintLocales`), while `inputmode` has no `latin` value (its keywords are `none`, `text`, `decimal`,
  `numeric`, `tel`, `search`, `email`, `url` — none names a language). Android gives
  an app no public API to pick an input-method subtype either, so the keyboard's
  own language key is the only switch. `VocabularyPanel.svelte` therefore sets
  `lang` on each field for the screen reader and the spell checker, and otherwise
  leaves the layout alone. Do not add an attribute expecting the keyboard to move.
- **The pinyin tone key writes the mark; the interface never guesses where it
  goes.** Typing `xuéxí` on a phone is two taps per accented vowel, so
  `VocabularyPanel.svelte` has a tone row that calls `mark_tone`. The placement
  rule — `a`, then `o`, then `e`, then the last of `i`/`u`/`ü`, which is what puts
  the mark on the `u` of `iu` and the `i` of `ui` — lives in `pinyin.rs`, beside
  the code that reads those marks back: a mark written one way and read another is
  two rules for one thing. The caret crosses the boundary as a **character**
  offset, not the UTF-16 index `selectionStart` reports. Do not move the rule into
  TypeScript, and do not insert a bare accented vowel instead.
- **A phone's 360 px is the panel's 320 px, and the vocabulary card was measured,
  not guessed.** `main` is padded 20 px a side, so design against the viewport
  minus 40. With three 30 px glyph buttons in the row, the group chip and the
  practice record left the reading column **zero** width at 320 px — the card
  fitted and the reading vanished, which is the worse failure. The chip and the
  record now sit under the reading in `.meta` where they may wrap, and the glyph is
  capped at 34 % so a six-character entry cannot take the row. Re-measure before
  adding a fourth thing there. The action key is `enterkeyhint="next"` on the first
  three fields and `done` on the last, with `advance()` moving focus; a key pressed
  during an IME composition is left to the keyboard, because that is what commits a
  candidate.
- **The stroke-order sweep is a clip, not a fade, and the band's width comes from
  the outline.** `drawSweptStroke` in `render.ts` fills the outline clipped to the
  band the pen has covered. A constant band gives one of two visible failures: too
  narrow and the outline's edges arrive in disconnected fragments; wide enough for
  the widest stroke and a short 点 flashes in whole. So `strokeRadii` measures each
  stroke's half-width once per character with `ctx.isPointInPath` and caches it.
  The trap inside that: `isPointInPath` takes its point in *canvas* coordinates
  while the path is transformed by the current matrix, so the measurement clears the
  transform and works in font space at 1:1 — measuring under the drawing transform
  silently reports nonsense.
- **A key that means two things has to be cancelled in the capture phase.** In
  click-to-draw mode (M8) Backspace abandons the open stroke, and with no stroke
  open it is the app's undo — two listeners on `window`, one in `PracticeCanvas`
  and one in `App`. Registering the cancel with
  `addEventListener("keydown", …, true)` puts it in the capture phase, where
  `stopPropagation()` keeps the event from reaching `App`'s bubble-phase handler.
  Both listeners on the *same* node would not do it: at-target phase runs capture
  and bubble listeners in registration order, so only `stopImmediatePropagation`
  would separate them — and that is not what a real keypress does, where the target
  is `body` and `window` is an ancestor.
- **Pointer work can be verified here after all — with synthetic events.** A
  temporary seed can dispatch `new PointerEvent("pointerdown" | "pointermove" |
  "pointerup", {clientX, clientY, buttons, pointerId: 1, bubbles: true})` at the
  canvas with coordinates computed from the canvas rect, and the app's own log (a
  `TEMP …` line through `api.log`) reports what the state became. Stub
  `setPointerCapture` / `releasePointerCapture` first, because a synthetic pointer
  id is not a real pointer and the call throws; and dispatch key events at
  `document.body` with `bubbles: true`, not at `window`, or the capture trick above
  is not exercised. Take the seed out afterwards. This does not prove the *platform*
  delivers hover moves as the handlers assume, so the last step is a human at the
  trackpad — that is what the §2 "human-confirmed" means.
- **A stroke has to end where the pointer was released.** `handleUp` in
  `PracticeCanvas.svelte` appends the `pointerup` position before committing the
  stroke. Otherwise a quick flick whose only sample arrives with the release
  collapses to a single point, and the grader discards it as an accidental tap — so
  the learner watches a stroke disappear and gets "1 mark was too small to be a
  stroke" instead of a grade. `push` ignores a sample that repeats the previous one,
  so appending it is free.
- **Pointer samples arrive by time, not by distance.** Sample density records
  drawing *speed*, so anything judging where a stroke sits must not use a sample
  mean (invariant 15): the mean moves when the learner speeds up, and the verdict —
  "wrong place" on a stroke they traced, with the shape score reading 95% — is
  impossible to act on. And **synthetic jitter cannot find this class of bug**,
  because jittering a reference preserves its sample density: the whole tolerance
  table stayed healthy while 18,763 strokes changed verdict under realistic
  sampling. `selfcheck`'s fourth section is the density-perturbed case.
- **The stray-tap filter is not a scoring rule.** Marks below
  `min(min_stroke_len, shortest_reference * 0.5)` are removed *before* strokes are
  paired, so they cannot influence any verdict or score (a test proves the report is
  identical with and without one). The threshold is 12 of 1024 units; only 2 of
  112,617 reference strokes in the dataset are that short. Keep the UI's wording
  free of any causal link to the stroke verdicts.
- **Do not measure the element you are sizing.** The canvas is sized in CSS from
  `side`, which starts at 0, so an observer on *the canvas* can never see a size to
  grow into. `PracticeCanvas.svelte` observes the surrounding board instead. This
  was a silent deadlock — the window opened with an invisible canvas.
- **The report column must not size the board.** `.workspace` is one grid row,
  `minmax(0, 1fr)`, pinned to the height `main` left it: a row with an `auto` size
  grows to whatever is in it, and the tallest thing in it is the feedback column.
  Sizing it that way handed the board its height from the report — the square was
  re-fitted, `place-items: center` pushed it down by half the extra, and the
  controls slid off the bottom when a tone was scored (on a 1180×840 window the
  board's top went from 120 to 247). `main` also carries `scrollbar-gutter: stable`,
  without which the page scrollbar the report brings takes its width out of the
  board. The board's *size* is deliberately still a fit to the window; what it must
  not do is change because a report appeared.
- **The phone's top bar carries the navigation, never the character.** It held the
  character's own glyph, which in recall mode is exactly the answer the mode exists
  to withhold. The `← n / N →` widget now comes from one `characterNav()` snippet,
  drawn in the top bar on a phone and the header on a wide screen; both move
  whatever `nav` says is live (the course, or a list, word or review queue), and
  `practising` decides whether the top bar draws it at all — the branch order of the
  markup written as one condition, so the two change together. The character's
  details fold there too (`.meta.open`, `showDetails`): folded, the meaning keeps two
  lines through `-webkit-line-clamp` and the rest sits in `#character-details` behind a
  More button a wide screen never draws. Checked at 390×844; one
  width-based media query, so iOS and Android are the same layout.
- **iOS decides whether there is an input at all from the audio *category*, and
  `cpal` asks it.** The first device build with tone practice failed every recording
  with **"channel count must be at least 1"**: `cpal` reads the input channel count
  from `AVAudioSession.inputNumberOfChannels()`, and the session this app shares
  with its speech was on `playback` — which has no input, so the count was zero and
  the stream was refused. That reads like a broken microphone and is not one.
  `capture.rs` now takes the session as `playAndRecord` (measurement mode,
  `defaultToSpeaker`, `allowBluetoothHFP`), *activates* it, and only then asks the
  device for its configuration; it gives the session back when the stream is gone.
  *Second half:* taking the session **is** a route change, which `cpal` reports to
  the stream's error callback as "Audio route changed" (`StreamInvalidated`, or
  `DeviceChanged` for a removed device). Its iOS backend only refreshes its latency
  estimate for those and leaves the stream running, which its own `ErrorKind`
  documents — so treating the callback as fatal threw away an utterance that was
  being captured perfectly well, as "The microphone stopped: Audio route changed".
  Both kinds are now logged and ignored on iOS; `DeviceNotAvailable` still fails the
  recording. Nothing here affects Android, which records through Kotlin's
  `AudioRecord`.
- **macOS voice names carry a locale qualifier**: `Tingting (Chinese (China
  mainland))`, not `Tingting`. Compare `base_name()`. A fixture with tidy names
  passed while the real list never matched, so the app quietly used another voice.
- **`say -v '?'` takes ~1 s** and lists every voice twice. Resolve it lazily and
  warm it on a background thread; never on the startup path.
- **Keep pointer-frequency data out of `$state`.** The in-progress stroke lives in
  a plain variable in `PracticeCanvas.svelte` and only schedules a repaint; making
  it reactive would deep-proxy on every pointer move.
- **Vite must ignore `.cargo-target/` and `.cargo-home/`** (already configured) or
  the dev server thrashes watching build output.
- **`cargo build`: the dev profile is `opt-level = 1`** for both the workspace and
  dependencies, so grading feels instant while iterating. Do not remove it.
- **The cursor row only changes when the cursor moves.** `set_index` returns false
  for the same position, and the UI debounces by 400 ms and skips a write matching
  what it just restored, so a fresh install writes no cursor row until you actually
  navigate. If you are checking persistence by hand and see nothing in
  `course_cursor`, that is why — move a step, then look.
- **A plain `cargo build` binary renders a blank window.** The frontend comes from
  `build.devUrl` (`http://localhost:1420`) unless the Tauri CLI builds it for
  production, so `.cargo-target/debug/hanzi-tutor` run by hand is a white window
  with no `[webview]` lines — only `[data]` and `[speech]` — unless Vite is up. Use
  `pnpm run dev` to look at the app, or the binary inside a built `.app` to test the
  packaged path. This is not a broken build, and it wasted an hour once.
- **The Rust side cross-compiles for iOS unchanged.** `cargo check -p hanzi-tutor
  --target aarch64-apple-ios` succeeds — engine, bundled SQLite, Tauri and all.
  `speech.rs`'s macOS-only pieces are gated tightly enough for that target's
  `-D warnings` now; that was the one tidy-up M9 needed.
- **An iOS build cannot finish inside this file sandbox.** The check gets through
  the dependency graph and then dies in Tauri's Swift glue: `swift-rs` builds
  `tauri/mobile/ios-api` with `swift build`, which wants the swiftpm caches under
  `~/Library` and applies its *own* nested sandbox — `sandbox_apply: Operation not
  permitted`, the same wall `scripts/probe-app-sandbox.sh` documents in §7. It is
  the environment, not the code: with a wider sandbox the check finishes in 20 s.
  Expect `tauri ios init`, `ios dev` and `xcodebuild` to need the same, since they
  write to `~/Library/Developer` and the CocoaPods caches.
- **The study data is a database, so "look at your data" means `sqlite3`.** The
  tables are `progress_card`, `attempt` (every attempt, ever), `vocab_entry`,
  `vocab_group`, `course_cursor`, `settings` and `meta`. `hanzi.db-wal` and
  `hanzi.db-shm` are SQLite's write-ahead log and shared memory, not stray files;
  the `-wal` file is where a save lives until the next checkpoint, which is what
  makes a kill mid-write survivable. The three JSON documents of an older install
  are **imports**, not outputs: after the import they are never read or written
  again.
- **`tauri ios dev` is for *devices*; simulators are a different route.** The
  `[DEVICE]` argument is matched against connected hardware, and the CLI prints
  simulators in that same "Detected connected device" list — so passing a
  simulator's name makes it build with `-sdk iphoneos`, which then fails asking for
  a development team. That error is misleading: the target was a simulator all
  along. The two routes that work are `tauri ios dev --open` (opens Xcode; you pick
  a simulator and press Run — the only path with hot reload) and, headlessly,
  `tauri ios build --debug --target aarch64-sim --ci` followed by
  `xcrun simctl install <udid> "<app>"`, `xcrun simctl launch <udid>
  com.hanzitutor.app`, and `xcrun simctl io <udid> screenshot shot.png`.
- **The test device is a *simulator* unless `xcrun devicectl list devices` says
  otherwise.** This machine has an iPhone for every model name and five iOS
  runtimes, and `devicectl` is what distinguishes hardware ("available, paired")
  from the `simulated` column. The one real phone is `HHIP1`, an iPhone14,3.
- **AVFoundation objects are not `Send`, so iOS speech runs on the main thread.**
  `Retained<AVSpeechSynthesizer>` cannot live in `Speaker` — Tauri requires shared
  state to be `Send + Sync`, and objc2 marks these classes `MainThreadOnly` — so the
  synthesiser lives in a `thread_local` on the main thread and every call goes
  through `with_main`, which runs inline when it is already there (dispatching
  synchronously to the queue you are standing on is a deadlock) and otherwise hops
  the queue and waits on a channel. Only plain data (`Vec<Voice>`, a `Result`) crosses back. On iOS the
  `Utterance` field does not exist at all, which is why the struct has a `#[cfg]` on
  it.
- **iOS speech takes the audio session, and gives it back — a few seconds later.**
  The device build spoke on the simulator and was silent on the phone with no error
  anywhere, because the default `soloAmbient` category is muted by the Ring/Silent
  switch — and the simulator has no such switch. Pronunciation now sets `playback` +
  `spokenAudio` + `duckOthers` for an utterance and deactivates the session
  (`notifyOthersOnDeactivation`) `SESSION_HOLD_MS` after the synthesizer reports it
  finished or cancelled, so an explicit tap is audible whatever the switch says and
  ducked music returns shortly after the word ends. **The hold is not slack: handing
  the session straight back powered the route down between words, and the next one
  crackled.** Four details are load-bearing:
  * The hold is armed only when `isSpeaking` is false. `Speaker::speak` stops the
    previous utterance before starting the next, and AVFoundation may deliver that
    cancellation *after* its replacement has begun: releasing then cuts the new word
    off mid-syllable.
  * `SESSION_GENERATION` makes a stale timer stand down. Every utterance bumps it
    through `take_session`; a timer that wakes to find it moved on leaves the
    session alone. Check and release both run on the main thread, where `take_session`
    runs, so a tap landing while the timer sleeps wins.
  * `stop_on_main` deliberately releases nothing. The cancellation callback arms the
    hold, and `Speaker::speak` stops before it speaks — releasing there would be the
    cold route the hold exists to avoid.
  * `AVSpeechSynthesizer.delegate` is a **weak** property, which is why the
    synthesizer lives in a `Speech` struct beside its `Retained` delegate rather than
    alone in the `thread_local`. Failing to take the session is logged and otherwise
    ignored: it costs volume, not speech.
- **`Speaker::prime` builds the synthesiser and starts the route before anyone
  asks**, from the existing background voice warm-up thread — that is what makes the
  *first* tap warm rather than only the second. Without it the first tap cracked,
  because AVFoundation feeds buffers into a route that has not finished starting.
  Related and not changed: `audio.ts`'s `playSamples` builds a fresh `AudioContext`
  per utterance and closes it on `ended`, the same class of glitch — look there if a
  crackle turns up on a *phrase* rather than a character.
- **Never hold the synthesizer's `RefCell` borrow across an AVFoundation call.** A
  delegate callback can run inline on the main thread during
  `speakUtterance`/`stopSpeakingAtBoundary` and borrows the same `thread_local`; a
  live `RefMut` would panic. Both call sites clone the
  `Retained<AVSpeechSynthesizer>` out of the borrow and call through the clone.
- **iOS 26 and 27 kill an app that has not adopted the scene life cycle — and it
  looks like nothing at all.** On the phone the app showed a black flash, closed, and
  printed nothing to its own log; the only evidence was a crash report
  (`idevicecrashreport -u <udid> -k <dir>`) naming
  `___UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption_block_` at
  `EXC_BREAKPOINT (SIGTRAP)`. The simulator did not complain because it runs iOS 18.
  `tao` already implements the scene delegate (`TaoSceneDelegate`, in
  `tao/src/platform_impl/ios/scene.rs`), so nothing needs patching — what was
  missing is `UIApplicationSceneManifest`, which Tauri's iOS template does not add.
  It lives in `src-tauri/Info.ios.plist`, which the CLI **merges at build time**
  (not at `ios init`: re-initialising alone left the generated `Info.plist` without
  it). Two details are load-bearing:
  * `UIApplicationSupportsMultipleScenes` must be **true**. That is not a claim that
    this app wants several windows: it is the switch that puts `tao` into scene mode
    at all (`multiple_scenes_enabled()`). With it **false** the crash is gone but the app shows a **black screen**,
    because tao then creates the window in `didFinishLaunching` before any scene
    exists, and a window not attached to a scene is invisible once a manifest is
    present. A black screen instead of a crash is much harder to read: nothing is
    logged and the crash reports stop.
  * There must be **no `UISceneConfigurations`**. tao answers UIKit's
    `configurationForConnectingSceneSession` with its own `UISceneConfiguration` named
    `TaoScene`, with the delegate class set; naming a delegate here as well is a second, competing source
    of truth.

  Diagnose this on the **simulator**: the same manifest black-screens both, and the
  simulator can be screenshotted.
- **A device build needs `~/Library/Developer/Xcode`, and the sandbox that
  withholds it fails in a way that reads like a signing problem.** Under a
  workspace-write file policy `xcodebuild` dies in *Build Preparation* — before it
  compiles or signs anything: `Couldn't create workspace arena folder
  '…/DerivedData/hanzi-tutor-…': Unable to write to info file`, then `Error saving
  log: … Code=1 "Operation not permitted"` for the `.xcactivitylog`, and exit 65.
  The giveaway is the phase: no signing step is named, and an ordinary `touch` into
  `~/Library/Developer/Xcode/DerivedData` fails the same way while a write inside
  the repository succeeds. `xcodebuild` needs DerivedData, the module cache and the
  provisioning profiles; nothing else here does — `devicectl device install` and
  `process launch` are happy sandboxed. **Do not "fix" this by putting
  `DEVELOPMENT_TEAM` into `project.yml`.** That was tried on 2026-09-22 and is
  unnecessary: `APPLE_DEVELOPMENT_TEAM=X5DWXB4283` on the command is sufficient by
  itself (re-verified with `project.yml` reverted), and the signing identity Xcode
  picks is the one whose *name* says `(Y38YQNR57Q)` while the bundle's
  `TeamIdentifier` is `X5DWXB4283`. Because the block is host policy rather than
  anything in the repository, **a build that worked in an earlier session can start
  failing with no repo change at all**; check the phase in the log before touching
  any signing setting. A device build also rewrites `DEVELOPMENT_TEAM` into the
  generated `project.pbxproj` and reorders keys in `hanzi-tutor_iOS/Info.plist` (the
  `CFBundleVersion` step); both are build churn, semantically identical to `HEAD`, and belong reverted rather
  than committed.
- **`ios build` cannot replace a stale archive.** A second build fails with
  `failed to rename app …/hanzi-tutor_iOS.xcarchive/Products/Applications/Hanzi
  Tutor.app: Directory not empty (os error 66)`. `rm -rf src-tauri/gen/apple/build`
  first, every time; it is gitignored, so nothing is lost.
- **A device run is `ios build` + `devicectl`, and the phone must be unlocked.**
  `tauri ios build --debug --target aarch64 --ci` produces an **IPA**
  (`src-tauri/gen/apple/build/arm64/Hanzi Tutor.ipa`), not a loose bundle: unzip it
  and install `Payload/Hanzi Tutor.app` with `xcrun devicectl device install app
  --device <udid> <app>`, then `xcrun devicectl device process launch --device
  <udid> com.hanzitutor.app`. A locked phone refuses the launch with `Unable to
  launch … because the device was not, or could not be, unlocked` — the only step
  here that needs a human. `idevicescreenshot` (libimobiledevice) reports "No device
  found" for this iPhone, but that is that tool's limitation and not this setup's:
  **`xcrun devicectl device capture screenshot --device <udid> --destination
  shot.png` does screenshot the physical phone** (verified on HHIP1, iOS 27,
  1284×2778), so "what does the screen actually show?" is a question this machine
  answers itself. `devicectl … --console` is the intended channel for the app's own
  log lines, though it did not forward Rust's stderr here.
- **iOS *release* builds do not link; debug does.** `ios build` without `--debug`
  fails at the app link with `symbol(s) not found for architecture arm64` for every
  Tauri Swift entry point (`_run_plugin_command`, `_register_plugin`,
  `_on_webview_created`, `_log_stdout`, `_init_plugin_dialog`). In the archives, those
  symbols are **local** (`t`) in `Products/Release-iphoneos/libTauri.a` where
  `Products/Debug-iphoneos/libTauri.a` exports them (`T`), so the Rust staticlib can
  only bundle them in a debug build. That wants a Tauri or Swift toolchain version,
  not a change here; until then a device build is `--debug`.
- **`crate-type` carries `staticlib`, `cdylib` and `rlib`, and it cannot be
  per-target.** Tauri's template includes `cdylib` for Android; cargo also builds it
  for iOS, where its link fails (`-lTauri` is not passed) and cargo treats that as
  fatal before Xcode runs — hence the `staticlib` iOS links. The risk runs the other
  way now: removing `cdylib` breaks Android.
- **The team ID belongs in the environment, not in a committed file.**
  `APPLE_DEVELOPMENT_TEAM=X5DWXB4283` on the build command is enough for
  `-allowProvisioningUpdates` to provision the app. Xcode writes `DEVELOPMENT_TEAM`
  into the generated `project.pbxproj` when it does; that line is **not** committed,
  for the same reason the macOS signing identity is not in `tauri.conf.json`.
- **A build produced by `tauri ios build` embeds the frontend; it does not use the
  dev server.** A layout change therefore needs `vite:build` *and* a Rust rebuild to
  re-embed — a couple of minutes per look. `ios dev --open` is the only HMR route on
  iOS. Do not drive `xcodebuild` at the project directly: the "Build Rust Code" phase
  asks the parent CLI for its options over a WebSocket and panics with `failed to
  read CLI options … Connection refused` without one.
- **`gen/apple/tauri` is a file this project adds, and a full re-init deletes it.**
  The npm CLI's template runs `node tauri ios xcode-script …` from
  `src-tauri/gen/apple`, but `ios init` does not write that entry point, so the build
  stops at `Cannot find module '…/gen/apple/tauri'`. The committed shim forwards to
  `@tauri-apps/cli`; it must be an **ES module** (`import`, not `require`) because
  the root `package.json` sets `"type": "module"`. The standalone CLI generates a
  different phase (`cargo tauri …`) that needs no shim.
- **Xcode 27 refuses an iOS deployment target below 15.0,** and Tauri's template
  defaulted to 14.0. It is set in `tauri.conf.json` as
  `bundle.iOS.minimumSystemVersion` (which maps to `IPHONEOS_DEPLOYMENT_TARGET`).
  `tauri ios init` **leaves an existing
  `project.yml` alone**: to make the config take effect you must delete
  `src-tauri/gen/apple` and re-init, which also deletes the shim above.
- **The iOS framework dependency is a hand-edit to `project.yml`, and a re-init
  loses it.** `bundle.iOS.frameworks` is the *wrong* key for sherpa-onnx despite the
  name: Tauri renders it as `- sdk: {{this}}.framework`, which is for Apple system
  frameworks, so pointing it at an xcframework yields a nonsense `sdk:` entry. What
  this project has instead is a `- framework: SherpaOnnxC.xcframework` dependency
  with `embed: true` written directly into `src-tauri/gen/apple/project.yml`.
  `embed` is load-bearing: iOS links sherpa-onnx **dynamically**, so without it the
  app builds and then dies at launch, because dyld cannot find the dylib.
  `scripts/fetch-sherpa.sh --ios` stages the framework at
  `gen/apple/SherpaOnnxC.xcframework` against a pinned digest, so that path and the
  script's staging directory have to agree.
- **Regenerating the iOS project after a build bundles 405 MB of static library
  into the app.** `project.yml` lists `Externals` as a source path, and the build
  writes the Rust static library to `Externals/arm64/<config>/libapp.a`. XcodeGen
  walks that path and treats the `.a` as a file to copy, so any `xcodegen generate`
  (which `ios init` runs) *after* a build adds "libapp.a in Resources" and the app
  ships the entire build intermediate — it took the IPA from **46 MB to 163 MB**. A
  fresh `ios init` never sees it, because `Externals/` is empty until something has
  been built. The source entry now carries `excludes: ["**/libapp.a"]`; the library
  is still linked, because that comes from the `dependencies` entry.
- **Compiling the Android Kotlin from here needs three deviations, and the obvious
  command fails for a reason that looks like a broken toolchain.** `./gradlew -g
  <workspace dir>` cannot work: the wrapper insists on writing a `.zip.lck` beside
  the distribution, and `~/.gradle` is read-only in this sandbox, while pointing
  `-g` at a workspace that symlinks the (read-only) distribution fails the same way.
  So call the **already-unpacked launcher** directly, give Gradle a read-only
  dependency cache so it never downloads, and ask for the task by its
  **flavour-specific** name — the Rust plugin adds `abi` product flavours, so there
  is no plain `compileDebugKotlin`:

  ```bash
  cd src-tauri/gen/android
  GRADLE_RO_DEP_CACHE="$HOME/.gradle/caches" \
    ~/.gradle/wrapper/dists/gradle-8.14.3-bin/*/gradle-8.14.3/bin/gradle \
    -g "$PWD/../../../.gradle-home" --no-daemon :app:compileUniversalDebugKotlin
  ```

  The workspace `-g` directory is gitignored and regenerable — delete it when you are
  done rather than leaving a gigabyte in the tree. Note also that a host `cargo
  clippy` **never compiles the `#[cfg(target_os = "android")]` code**, so a change to
  `AndroidKeystore` in `src-tauri/src/sync.rs` can pass every check here and still
  not type-check; run `cargo check -p hanzi-tutor --target aarch64-linux-android`
  with the NDK's `aarch64-linux-android26-clang` as `CC`/linker (and `llvm-ar` as
  `AR`, `llvm-ranlib` as `RANLIB`) to cover it.
- **Both mobile targets need their own pinned sherpa-onnx artefact, and
  `with-cargo-env.sh` has to know which.** `sherpa-onnx-sys` forces shared linking on
  Android as well as iOS, so `SHERPA_ONNX_LIB_DIR` pointing at the macOS *static*
  libraries panics the build script with "No shared runtime libraries found in …".
  iOS gets a lib directory (`fetch-sherpa.sh --ios`), Android an archive directory
  (`fetch-sherpa.sh --android`, through `SHERPA_ONNX_ARCHIVE_DIR`, so the crate picks
  the ABI matching the architecture being built). The choice is made from the
  command's arguments, because the `--target` triple is constructed inside Tauri and
  never visible to the wrapper — and it matches target triples as well as CLI
  subcommands, since a bare `ios`/`android` word misses `cargo check --target
  aarch64-linux-android`, which then fails with that same misleading panic.
- **The crate cannot stage either mobile artefact into this project itself.** Its
  `find_tauri_project_dir` looks for `tauri.conf.json` in `target_dir.parent()`,
  which assumes the default `src-tauri/target/`. This project sets
  `CARGO_TARGET_DIR=<repo>/.cargo-target`, so that parent is the repository root, the
  lookup finds nothing, and the copy is **silently skipped** — for the iOS
  xcframework and for Android's jniLibs alike. Both are staged by
  `scripts/fetch-sherpa.sh` instead. Android's staging copies into the existing
  per-ABI directories rather than over them, because Gradle's own build leaves a
  symlink to `libhanzi_tutor_lib.so` in the very same place.
- **A signing identity's parenthetical is not the team ID.** `cargo-mobile2`
  reports `Apple Development: someone@example.com (Y38YQNR57Q)`; passing that value
  as `APPLE_DEVELOPMENT_TEAM` gives `No Account for Team "Y38YQNR57Q"`. The team
  this project signs with is `X5DWXB4283`, and it belongs in the build environment.
- **SQLite does not create the directory for you.** `Connection::open` fails with
  "unable to open database file" if the data directory is missing, which is exactly
  the state a first run is in — and the warning blames the database, not the missing
  folder. `Db::open` therefore does `create_dir_all` first. If that line ever moves,
  every fresh install breaks, and `--user-dir` at a path that does not exist yet
  breaks with it. There is a test that starts with no directory.
- **Port 1420 may be held by an orphaned Vite.** `pkill -f vite` does not always
  match it, because `pnpm` here runs Electron as Node, and a survivor makes `pnpm
  run dev` fail in `beforeDevCommand` while the app still starts against the *old*
  dev server — so a stale bundle can be what you are looking at. Check `curl -s -o
  /dev/null -w '%{http_code}' http://localhost:1420` before trusting a run.
- **A compiled-in dependency is a shipped notice.** SQLite and `rusqlite` were the
  first third-party *code* in the binary, and they were added to
  `src-tauri/src/licences.rs`, `tauri.conf.json` and `licences/` together because the
  tests only check the three against *each other* — a dependency nobody catalogued is
  invisible to them. Adding one means checking its licence by hand and adding a
  notice; the count in the §2 log line moves with it.
- **An "again" card is due 60 seconds later, not tomorrow.** Deliberate (see
  `AGAIN_SECONDS`), and it is why the interface refreshes the queue on a 60-second
  heartbeat as well as after every answer. Change the interval and the heartbeat
  together, or the badge will look stuck.
- **`src-tauri/gen/schemas/`** is generated and gitignored; `capabilities/default.json`
  references it with `$schema`, so editors warn until the first build. Expected.
- **A stale artifact fails on the magic, not on the JSON.**
  `crates/hanzi-core/data/hanzi.bin.gz` is gitignored, so a checkout that pulled an
  artifact-changing commit keeps the old file and the app refuses to decode it. The
  message says *re-run `prepare-data`*; do that rather than hunting for a bug in the
  loader. `build.rs` checks only that the file *exists*, not that it is current, so
  this is a runtime error rather than a build one.
- **`prepare-data` is the only place the word list is filtered.** Words whose
  characters lack geometry or a frequency rank are dropped there, not at load, so
  "why is this word missing?" is answered by its `words from …: N kept` line. If you
  add a rule, add it there and print the count it dropped.
- **Clicking a character in a word is a search, not navigation.** `WordsPanel` turns
  the click into a query for that character. The course jump lives on the sidebar.
- **A word can repeat a character, so never key a per-character `{#each}` by the
  character.** 是不是 is one of the first words, and keying the glyph tiles by `ch`
  throws `each_key_duplicate`. The nastier half is *how* it fails: the error is raised
  inside Svelte's render flush, so the window simply stops updating — the panel sat on
  "Searching…" with the correct data already in state, and nothing appeared in the
  terminal, because a webview's console is not visible from here. Key by index. The
  `error` / `unhandledrejection` handler in `App.svelte` forwards that class of
  failure to `[webview] webview error: …` — keep it.
- **A notice that names a licence is not the licence.** `fetch-data.sh` had been
  fetching Make Me a Hanzi's `COPYING`, which describes what `graphics.txt` and
  `dictionary.txt` derive from and points at a URL for the Arphic Public License —
  which was never followed, so the app would have shipped the notice *about* the
  licence without the licence itself. The same was true of the CC BY-SA legal code.
  If you touch the data pipeline or the notices, check that every licence a notice
  points at is also present as a **file**; `tests/licences.rs` requires the text and
  looks for a phrase only the genuine text contains, so a stub cannot pass.
- **`include_str!` makes the licence texts part of the Rust build.** Editing
  `LICENSES.md` or anything in `licences/` recompiles `hanzi-core` and `hanzi-tutor`
  — surprising when a build you expected to be instant takes ten seconds. It also
  produced the only false alarm this suite has given: a test compares the compiled-in
  text with the file on disk, so editing a text *while* cargo is compiling leaves the
  binary holding the old copy and the comparison fails. Re-run before investigating.
- **Bundle resource paths are relative to `src-tauri/`**, not the repository root,
  so a notice at `licences/x.txt` is written `"../licences/x.txt"` in
  `tauri.conf.json`. `tests/licences.rs` derives that string from each catalogue entry
  and compares the whole map, so a config that drifts from the catalogue fails the
  suite rather than the bundle.
- **`pnpm run build` signs.** `scripts/build-release.sh` picks a Developer ID
  certificate from the keychain automatically, so a build on someone else's Mac will
  be signed as *them* unless they set `APPLE_SIGNING_IDENTITY` — and on a machine
  with no certificate it falls back to ad-hoc, which still launches locally. `pnpm
  run build:unsigned` skips the wrapper entirely.
- **A microphone needs two things, and only one of them is obvious.** A signed,
  hardened-runtime build cannot open the microphone without
  `com.apple.security.device.audio-input` in `src-tauri/Entitlements.plist`, and the
  *user* is asked by `NSMicrophoneUsageDescription` in `src-tauri/Info.plist`. They
  are not alternatives: the plist key is the prompt, the entitlement is the kernel's
  permission. **The failure is silent and looks identical in both cases** — capture
  opens, streams, and delivers zeros, which the analyser reports as "I could not hear
  enough voice". Worse, it works in `tauri dev` and in an unsigned build and then
  stops once signed, so check the two commands in `README.md` on any build someone is
  going to run. A plain `tauri build` without `build-release.sh` is only
  *linker-signed* and never applies the entitlements at all. Verify with `codesign -d
  --entitlements - "$APP"`, which prints the dictionary when it is right and nothing
  when the file was not applied.
- **A terminal cannot be granted the microphone the way an app can**, and on this
  machine it has not been. So `cargo test -- --ignored` records silence here while the
  packaged app works, and **no test in this repository has ever heard a human voice**.
  Do not read a passing ignored test as proof that capture works; read the peak level
  it prints. See §9.

### Android

- **Every Gradle and emulator command needs a wider sandbox than `workspace-write`.**
  Gradle writes its dependency cache under `~/.gradle`, the emulator writes its lock
  and userdata files under `~/.android/avd`, and `adb` wants `~/.android` for its
  keys. All are refused with `Operation not permitted`, and the failure mode is not a
  clean error: a Gradle build *hangs* with no output because `cmd | tail` swallows
  the progress, and the emulator dies with `A snapshot operation … is pending and
  timeout has expired`, which reads like a stale snapshot rather than a permissions
  problem. Run these with `danger-full-access`, and do not pipe a long build to
  `tail` while diagnosing.
- **`minSdk` is 26 because of AAudio, and it has to be set in two places that
  agree.** `cpal` pins the `ndk` crate to its `api-level-26` feature, so the library
  needs `libaaudio.so`, which does not exist before Android 8. At `minSdk = 24` the
  link fails with `unable to find library -laaudio` — note *which* clang it used,
  `aarch64-linux-android24-clang`: that API level comes from
  `bundle.android.minSdkVersion` in `tauri.conf.json`, which is what the CLI uses to
  pick the linker, while the manifest's `minSdk` comes from the generated
  `build.gradle.kts`. Setting only the Gradle one still fails to link.
- **Gradle cannot find the Tauri CLI under pnpm, and the error is a bare `Cannot
  find module`.** `gen/android/buildSrc/.../BuildTask.kt` runs `node tauri android
  android-studio-script` from `src-tauri/`, expecting a package literally named
  `tauri` to be resolvable from there. pnpm does not flatten `@tauri-apps/cli` into
  such a name, so the task dies with `Cannot find module '<root>/src-tauri/tauri'`
  after the Rust library has already linked. `src-tauri/tauri.js` is a committed shim
  that loads the package's real entry point — `tauri.js`, not `main.js`, which only
  exports an API and would silently do nothing. It is an `import`, not a `require`,
  because the root `package.json` says `"type": "module"`.
- **The webview starts loading *while* `setup` runs, so nothing slow may happen
  there.** This is the trap that cost the most. On desktop the windows are created
  after `setup`, so a slow setup is invisible; on Android the frontend is running by
  ~300 ms and its first commands arrive before `app.manage()`, and a command that
  finds no state is **rejected, not queued** — the phone showed *"state not managed
  for field `state` on command `review_queue`"* and an empty board, and a second
  launch lost the commands entirely and sat on "Loading the character set…" forever.
  Decoding the 13 MB dataset and ordering the course now happen in
  `AppState::prepare()` **before the Tauri builder exists**, and `setup` only opens
  the study database. The fix is placement, not speed: the decode is only ~1 s of
  CPU, still 3× the webview's head start. Anything else slow added to `setup` — or to
  `AppState::assemble` — reintroduces this. Two cheap diagnostics:
  * `adb shell am start -W -n com.hanzitutor.app/.MainActivity` gives the real
    cold-start time (`TotalTime`). 262 ms once the one-off ART compilation after an
    install is past — the first launch after `adb install` takes ~90 s and is not the
    app's fault.
  * The webview is debuggable in a debug build: `adb forward tcp:9222
    localabstract:webview_devtools_remote_<pid>`, then `curl
    http://127.0.0.1:9222/json` for a WebSocket URL. Node 22+ has a global
    `WebSocket`, so no dependency is needed. This is how the pending-promise state,
    the missing managed state, and the `env()` value were confirmed.
- **Logcat is unusable on the test phone.** `adb logcat -d` returns only kernel and
  radio lines — no app output at all, on any buffer, including device-side `logcat`,
  on a RedMagic NX809J. Do not plan on `[data] …` or `eprintln!` lines there; use
  the DevTools probe above, or read the app's own data directory with `adb shell
  run-as com.hanzitutor.app ls`.
- **`env(safe-area-inset-*)` is the display cutout, not the status bar.** Android's
  WebView reports the cutout, so on a phone with a punch-hole the CSS was right by
  luck and on the emulator it was zero — the header was drawn under the clock. Both
  are edge-to-edge and cannot opt out (targetSdk 35+), so the page is told the real
  system bar insets by the `android_insets` command, and `app.css` takes
  `max(env(…), var(--inset-…))`. Taking the maximum makes one rule right on both: on a
  notched phone the two agree instead of summing.
- **Tauri dispatches mobile plugin commands on the Android main thread.**
  `run_command` goes through `run_on_android_context`, so a Kotlin plugin method that
  blocks freezes the UI, and one that waits on a latch deadlocks. The speech plugin
  resolves *later* instead: commands that arrive before `TextToSpeech` reports ready
  are queued and answered when it does, which is the normal case because the
  pronunciation warm-up asks for the voice list while the app is still starting.
- **Android offers a network voice beside the on-device one for the same locale.**
  The Kotlin side sorts `isNetworkConnectionRequired` first, so the automatic pick is
  `cmn-cn-x-ccc-local` rather than a network-backed voice that would fail offline.
  The Rust `Voice` model needed no new field — `pick_voice` takes the first mainland
  voice in the order the platform reported it.
- **An unset `ndkVersion` silently costs 170 MB.** Gradle needs the NDK to find
  `llvm-strip`; without a `ndkVersion` it gives up with one line — *"Unable to strip
  the following libraries, packaging them as they are"* — and ships the 203 MB debug
  library verbatim. With it set, the same APK is 76 MB. It is taken from
  `ANDROID_NDK_HOME` with the development version as a fallback so a build from
  Android Studio, where no such variable is set, still works.
- **A back press has to be answered synchronously.** `OnBackPressedCallback` must
  decide *now*, so the page is asked with `evaluateJavascript` and a global function
  (`window.__hanziHandleBack`) whose return value settles it; an event listener would
  need a round trip that cannot be waited for on the main thread. And the callback
  must be registered **enabled** — `OnBackPressedCallback(false)` is never invoked and
  the platform default quietly applies, which looks exactly like the feature not
  working.
- **`adb shell input` is a real enough input device to verify drawing.** `input
  swipe` on the board produced a stroke, moved the counter to `1 / 8`, enabled
  Undo/Clear and graded to a score — real touch events through the WebView's pointer
  handling. `input tap` drives buttons the same way. What it does not prove is palm
  rejection or stylus behaviour, which still wants a person.
- **The emulator that ships with Android Studio was full, and it is not ours to
  wipe.** `Medium_Phone_API_36.1` had 299 MB free of 6 GB with three of the owner's
  own test apps on it. `HanziTutor_API36` is a second AVD cloned from its
  `config.ini` with a 12 GB data partition instead — `avdmanager` is not installed
  here (there is no `cmdline-tools/`), so the two files were written by hand:
  `~/.android/avd/HanziTutor_API36.ini` pointing at
  `HanziTutor_API36.avd/config.ini`, whose `image.sysdir.1` ties it to the
  already-installed system image. No image had to be downloaded.
- **`key.properties` belongs to the Android project root, not to `app/`.** The
  `.gitignore` there excludes it, and Gradle's module directory is `gen/android/app/`,
  so `file("key.properties")` looks in the wrong place and finds nothing — silently.
  The build does not fail; it just signs nothing, and the only signal is that the
  artifact is called `app-universal-release-unsigned.apk` instead of
  `app-universal-release.apk`. Read that filename literally. Use
  `rootProject.file("key.properties")`.
- **R8 does not break the Kotlin plugin bridge, but not because of anything here.**
  Release builds minify, and `register_android_plugin` instantiates `PlatformPlugin`
  **by name** — a reflective lookup R8 cannot see. What saves it is that Tauri's
  Android library ships *consumer* ProGuard rules (`-keep
  @app.tauri.annotation.TauriPlugin public class *` and the same for `@InvokeArg`),
  which apply to the app automatically. Debug-only verification would never catch a
  problem here.
- **The release build is not debuggable, so it cannot be probed over DevTools.** Wry
  enables webview debugging in debug builds only, so `webview_devtools_remote` has no
  socket in a release build. Verify the release by installing it, screenshotting, and
  driving it with `adb shell input`.
- **`INTERNET` is declared for release, and that reversed a deliberate property.** It
  used to be scoped to `app/src/debug/AndroidManifest.xml`, so the released manifest
  declared only `RECORD_AUDIO` and `aapt2 dump permissions` made "works offline"
  checkable on the artifact. M12's optional model download is a runtime request, so
  the permission moved into the main manifest; the debug copy is now redundant and
  kept only because `tauri android init` regenerates it. **The privacy policy and the
  Play listing were restated at the same time** — `docs/privacy-policy.md` and
  `store/listing.md` both used to say "no network requests at all", and a listing that
  says that while the app offers a 163 MB download is a mismatch Play's Data safety
  declaration would catch. The substance is unchanged: nothing is collected, nothing
  is uploaded, and a request happens only when the learner presses the button. If a
  future change removes the download, put the permission back in the debug source set
  and restore the claims rather than leaving both stale.
- **The model downloads are unverified on Android.** The recognition one is exercised
  end to end on macOS (`asr::tests::downloads_verifies_and_installs_the_model`), but
  no Android release build has run either download here, and the permission change
  above is exactly the sort of thing that only shows up on a device. Install a signed
  release APK and press each install button once before trusting it.
- **Play needs more than an AAB.** Because the app asks for the microphone, the
  listing requires a published privacy policy and a Data safety declaration, and the
  answers have to match what the app really does. `docs/privacy-policy.md` and
  `store/listing.md` hold both; the policy still needs a real contact address and a
  public URL before submission, and both are marked with TODOs.
- **The version code comes from the app version.** `tauri.properties` derives `3000`
  from `0.3.0`, and Play requires it to increase with every upload, so a second upload
  means bumping the version in `Cargo.toml` and `tauri.conf.json` first.
  `bundle.android.autoIncrementVersionCode` exists for people who would rather not
  remember.

### Android speech and the microphone

- **`TextToSpeech.speak` returning `SUCCESS` means nothing was heard.** It means the
  engine *accepted* the utterance. The voice's data may be missing, a network voice
  may have no network, the output may not open — all producing silence from a call
  that reported success. `PlatformPlugin.speak` now resolves when
  `UtteranceProgressListener.onStart` fires and **rejects with the engine's own error
  code** when it does not, with a three-second guard. Both `onStart` and `onError` are
  matched by **utterance id**: `speak` cuts off the previous utterance, and the engine
  reports that cancellation in its own time, so without the match a stale failure gets
  blamed on the next request.
- **A listed voice is not necessarily a usable one, and on a phone that has never had
  a network none of them are.** Android's engine advertises every voice it knows and
  marks the ones whose data has never been downloaded with
  `Engine.KEY_FEATURE_NOT_INSTALLED`. The test phone reported **16 Chinese voices and
  0 installed** — no connectivity at all — and asking it to speak produced a service
  error or silence. With WiFi on, the same query answered **14 installed** and speech
  worked, using `cmn-cn-x-ccc-local`, an on-device voice. So: sort installed voices
  first (Rust takes the first Mandarin voice it sees), prefer on-device over network,
  and keep the *runtime* needing no network — the voice data is a one-time device
  setup step, like installing a font. (The *app* does declare `INTERNET` now, for
  M12's optional model download; that is a separate thing from the voice data being
  installed.)
- **A vendor ROM may mute the synthesiser outright, and the app has to object.** The
  RedMagic build logs, on every attempt: `AudioHardening background playback would be
  muted for com.google.android.tts`, with the music stream at full volume and the app
  in the foreground. This is the Android twin of the iOS Ring/Silent problem. `speak`
  now takes audio focus (`AUDIOFOCUS_GAIN_TRANSIENT_MAY_DUCK`, borrowed from the iOS
  session's `duckOthers`) and sets `USAGE_MEDIA` + `CONTENT_TYPE_SPEECH`, giving the
  focus back on `onDone`, on error, and on `stop`.
- **Android 11 and later hide the speech engine unless it is asked for by intent.**
  `<queries><intent><action
  android:name="android.intent.action.TTS_SERVICE"/></intent></queries>` in the
  manifest, or `getDefaultEngine()` answers null on a phone that plainly has an engine
  installed.
- **`TextToSpeech.getDefaultEngine()` does not resolve against this compile SDK.**
  Read `Settings.Secure.TTS_DEFAULT_SYNTH` instead — the same value the platform uses,
  and on the test phone it is *empty*, which is itself the answer to "why is this
  phone silent": no synthesiser had ever been chosen there.
- **The microphone has to be asked about more than once.** The status was fetched once
  at startup, with the comment that "whether the machine has one does not change while
  the app runs". On Android that is false in the most annoying way: the permission is
  granted through a dialog dismissed *after* that first read, so the answer is always
  "no" and a control disabled on it stays disabled for ever.
  `MainActivity.onRequestPermissionsResult` now tells the page to ask again, and the
  page also re-asks whenever it comes back to the foreground.
- **On a phone there is no tooltip, so a control that cannot be used has to say why
  somewhere the learner will look.** The microphone button was greyed out and read as
  a microphone fault, when the microphone was fine and **的** was believed to have no
  judgeable tone — it is a neutral-tone particle. (It is scored now, on being level;
  see §9.) The reason is carried by the `.hint` under the row, and `sayBlocked` in
  `App.svelte` is the one expression the tooltip, the accessible name and the hint all
  read, so the three cannot drift. A **separate** explanatory paragraph is the wrong
  answer — it wraps to its own row and pushes the rest of the controls off the screen.
  That line is now the `.hint`'s *only* job: it renders when a control is disabled and
  not otherwise. Three shapes were tried (four wrapped rows of labelled buttons — too
  tall; bare glyphs with the word only in `title`/`aria-label` — no tooltip on a
  phone; and the mock-up's **three cards**: a glyph on a tinted disc with its short
  name under it, two tools to a card, centred under the board — **74 px** on a 390 px
  phone, one line, words visible). Two things were cut from the mock-up after using it
  on a phone: the captions under each card, whose names moved onto the cards as
  `aria-label`, and "Hold to speak", which wrapped and made its card taller — now
  "(hold…)" on the button with the sentence in `title`. The visible words are the
  accessible names, so a voice-control user can say what is on the button — see
  invariant 28.
- **Push-to-talk has three ways to stop itself, and the third is not obvious.** The
  button moved out from under the finger (clearing the previous judgement removed the
  panel above it and pulled the row up, which fired the `pointerleave` wired to
  `stopListening`), `touch-action` let the browser claim the touch
  for a scroll, and — the one that survived both fixes — **Android's long-press
  selection gesture takes the pointer at 555 ms** and sends `pointercancel`. The button
  now captures the pointer, leaves the previous judgement on screen, sets
  `touch-action: none` and `user-select: none`, and swallows `contextmenu`, which is
  what the practice board had been doing all along. §9 has the measurements. The label
  that used to narrow to "Listening…" is gone and the button is a fixed-size mouth
  glyph, so one cause is now structurally impossible; the panel above it can still
  appear and vanish, so the pointer capture and the untouched previous judgement stay.
  Instrument the events rather than guessing: listeners for
  `pointerdown`/`pointerup`/`pointercancel`/`lostpointercapture` with timestamps say
  in one run what took three rebuilds to infer.
- **`cpal`'s Android input does not work; `AudioRecord` does.** Capture is
  per-platform for that reason — `cpal` elsewhere, Kotlin's `AudioRecord` on Android —
  and §9 has the AAudio log, the five-source probe and the three details of the Kotlin
  backend that are easy to get wrong.


## 6a. Resuming inside a user vocabulary list

**Built.** A drill opens at the learner's stored position in the group, and that
position travels between devices. It was a missing feature, not a bug — nothing
failed to save — and it was reported again after being written up here as "not
started", which is the cost of a deferral nobody can see from the roadmap. It is
listed in `ROADMAP.md` as well, so the next session finds it.

The course remembers where you were; a vocabulary list used not to, and always
restarted at its first entry. Confirmed against a real list while fixing it:
entries are ordered by `(group, id)`, and in one group of 29 the **first** entry
was the one already practised, so every session re-drilled it and started over —
the exact symptom reported.

### How the resume decides

1. **The stored position wins** when it names an entry still in the list. It is
   exactly where the learner stopped, and it is the only thing that can carry a
   position *between* devices.
2. Otherwise the drill starts at **the first entry not yet practised**
   (`last_practised`), which needs no storage and is the right answer for a list
   worked through in order.
3. If every entry has been practised, it starts at **the top**: such a list is due
   for review, not finished.
4. A drill that is not one named group — everything, or the unfiled remainder —
   keeps no position, because there is no list to keep a place in.

### What was established, so it is not redone

* `vocab_entry` carries `attempts` and `last_practised`; `vocab_record_attempt`
  writes both; both are on the IPC boundary as `VocabEntry.attempts` /
  `.lastPractised`. That is what makes rule 2 above free.
* A position is per **group**, so `VocabularyPanel` passes the group alongside the
  entries, and a row's own Practise button passes none.

### Decisions taken, and the one the plan got wrong

* **Groups are keyed by name, not by a new stable id — and this is the plan
  correcting itself.** What stood here said "groups get a stable id", on the
  grounds that a position keyed by name is orphaned by a rename. The reasoning is
  sound; the cost was not priced. A *random* id is minted per device, so two
  devices that already share a group would each invent a different one for it: the
  group would stop merging, and the cursors keyed by those ids would never meet.
  Three devices are already synced here, so it is not hypothetical. Keying by name
  leaves the merge that M13 verified on three devices exactly as it was, and a
  rename is handled where the rename happens — `vocab_rename_group` moves the row
  in the same call. The cost is narrow: a peer that has not heard about the rename
  keeps a row under the old name until it next practises, and a group deleted and
  recreated with the same name can inherit the old position once. Both are benign,
  and neither justifies a schema-wide identity change.
* **Last write wins per group.** One person across a phone and a laptop, so there
  is nothing to reconcile per device: the stamp settles it. Per *group* is the
  load-bearing part — a position only means anything against the list it was taken
  in, so `merge_vocab_cursors` never compares one group's stamp with another's.
* **Resuming skips entries already practised** (`last_practised IS NOT NULL`) when
  there is no stored position, and **the drill restarts from the top** when every
  entry has been practised: such a list is due for review, not finished.
* **Deletion clears a position** (the group is gone, so `delete_vocab_cursor`
  drops its row).

### What implementing it took

Five modules, which is why it was not done as a fragment:

1. `crates/hanzi-store/src/schema.rs` — the `vocab_cursor` table, schema 6.
2. `crates/hanzi-store/src/lib.rs` — `vocab_cursor` / `set_vocab_cursor` per
   group, with the same three-part stamp as the course cursor, plus
   `rename_vocab_cursor`, `delete_vocab_cursor` and `apply_vocab_cursors`. It also
   does the **id ↔ uuid translation**, so nothing above it has to know.
3. `crates/hanzi-sync/src/document.rs` — `merge_vocab_cursors`, last-write-wins
   **per group**, and the `vocab-cursor.json` shard.
4. `crates/hanzi-sync/src/local.rs` — publishing the positions and pulling them,
   alongside the course cursor.
5. `src-tauri/src/commands.rs` + `src/lib/api.ts` + `src/App.svelte` +
   `VocabularyPanel.svelte` — the commands, the wrappers, and the resume itself.
   The panel passes the group with the entries, and a row's own Practise button
   passes none, so a one-off drill cannot overwrite the place the learner reached
   in the group that entry came from.

**Nothing was needed in `hanzi-core`.** The engine's document type has no uuid and
did not gain one: the store already owns entry uuids, so it translates, and the
interface keeps working in ids. That is also why the synced half was additive
rather than a migration — the row stored a portable uuid from the first day.

### Why the position is a uuid and not an id

An id is handed out per device, so id 3 names a different word on the phone than
on the laptop. Storing the uuid is what makes the row portable and what let the
sync land without touching anything already deployed.

### Why a position that syncs was worth the care

It is the one area where an incomplete change is worse than none: a position that
publishes but cannot be merged leaves two devices disagreeing about where the
learner is, which is the exact failure M13 exists to avoid. Hence the per-group
merge, and hence the tests below — the merge is the part that has to be right.

The skip-practised resume was safe to ship on its own, and that is the test to
apply to the rest of this: it adds no state that sync can disagree about.

### Tests

Five on the store — round trip (and that the **uuid**, not the id, is what is
stored), a rename moving the row, a deletion clearing it, a position whose entry
is gone reading as no position, and clearing — one through the state layer, three
on the merge (including that positions for different groups are never compared),
and one end-to-end on two databases syncing through a folder store.

## 7. Open decisions

### Still open

- **The App Store path is untested, and one part of it is at risk.** A Mac App
  Store build must be sandboxed, and `speech.rs` pronounces by spawning
  `/usr/bin/say`, which a sandbox may refuse. `scripts/probe-app-sandbox.sh` was
  written to settle it and **could not do so from here**: applying any sandbox
  profile is refused in this environment (`sandbox-exec -p '(version 1)(allow
  default)' …` → `sandbox_apply: Operation not permitted`), and an app signed with
  `com.apple.security.app-sandbox` and launched through launchd ran with the
  entitlement present but unenforced. The script reports "inconclusive" rather than
  a false answer. Run it from a normal login session, or put a build on TestFlight
  and try *hear it* there. If `say` is refused, the macOS backend wants
  `AVSpeechSynthesizer` in process — **not a new design**: `speech.rs` already
  speaks that way on iOS, so it is the same backend behind a `cfg`. Settle this
  before writing M6's three backends. There is also an answer that removes the
  question: the pre-rendered audio pack in
  [`docs/research/ASR_TTS_CLAUDE_RESEARCH.md`](docs/research/ASR_TTS_CLAUDE_RESEARCH.md)
  §4.4 takes synthesis off the runtime path for the bundled curriculum, and M14's
  clips are that answer in part already.
- **The ink measure is proven, but half of it cannot fire yet.** The canvas paints
  every stroke at one fixed width, so nothing a learner does on a trackpad can put
  down *less* ink than `INK_WIDTH` and the `faint` verdict is unreachable in daily
  use; what M4 catches today is overshoot and short strokes. The width half becomes
  live when input can report a real pen width — a stylus. **M8 chose not to add the
  velocity-thickened brush it had listed as "cosmetic only"**, and the reason is
  worth keeping: ink amount became a quarter of the score in M4, so stroke width is
  a grading input now, and a brush that thinned with speed would score a fast stroke
  worse for being fast. Doing it properly means the grader takes a width per stroke
  and `INK_OK` is re-tuned against real attempts.
- **The four headline weights are a judgement, not a measurement.** An equal
  quarter each, chosen so a third-inked character cannot read "Excellent". The
  honest way to set them is the attempt log below, on real handwriting.
- **Shape tolerance is tuned on synthetic jitter**, not on real learners. It wants
  revisiting once there are real attempts to look at: schema 5 records the measures, so
  `pnpm run analyse-attempts` is the instrument and the missing input is the attempts.
- **The attempt log is built, and what is short is the data.** M10 shipped the
  unbounded log (`attempt`, with `Db::attempts` and `Db::attempt_count` to read it) and
  the schedule shows the newest 20 attempts per character. Schema 5 then added what an
  attempt was graded from, `export_practice_log` (the settings screen's two buttons, or
  the command) writes the log out as JSON Lines or CSV, and `hanzi_store::analyse` —
  printed by `pnpm run analyse-attempts` — reports the distributions, the mass near each
  bar, and which measures are pinned or fail to separate passes from failures. It
  reports rather than concludes, because it cannot know whether an attempt was right, so
  what remains is enough *measured* attempts to re-set the tolerance and the weights
  against. On this machine's log that was 124 of 124 attempts unmeasured, all predating
  schema 5 — which the analysis says plainly instead of inventing a distribution. Two
  things to keep in mind when reading it: a migrated card's `attempts` count can exceed
  the rows in the log (the JSON it came from kept only the newest 20), so the log starts
  at the import rather than at the learner's first attempt; and a peer's attempts arrive
  without measures, so a synced log is only measurable where it was written.
- **Graded phrase audio (M14): the bundled voice is settled, the readers' is not.** The
  HSK 1–2 clips are the corpus's own **Apache-2.0 CosyVoice2 recordings**, fetched by
  `scripts/fetch-clip-audio.py` (819 phrases, 1,638 clips, zero missing takes) — decided on
  measurement, after generated MeloTTS audio failed 57 of 208 phrases where the published
  recordings failed 15. MeloTTS stays for **on-device speech for a phrase with no clip**.
  Still open: the **graded readers'** audio (text only upstream; a MeloTTS first pass exists
  uncommitted and is not shippable on the same measure), the app-level check, and playback
  speed. The measurements are in
  [`docs/research/MELOTTS_PRONUNCIATION_ACCURACY.md`](docs/research/MELOTTS_PRONUNCIATION_ACCURACY.md);
  `verify-audio` (§3) is the instrument.
- **Tone scoring against real voices is unfinished** — segmentation on words, and
  scoring constants never fitted to a recording. See §9.
- **An accepted dependency advisory.** Dependabot flags `glib` 0.18.5 (moderate,
  fixed in 0.20.0). It is Linux-GTK-only and absent from the macOS build graph, and
  it is not fixable from here because `gtk 0.18` pins `glib ^0.18` — cargo rejects
  the upgrade. **Do not spend time on it**: `cargo tree --target
  aarch64-apple-darwin -e normal | grep glib` returns nothing. ROADMAP has the
  detail.
- **Resuming inside a user vocabulary list is not started** — specified in §6a.

### Settled, and not to be re-litigated

- **Network and privacy posture.** Nothing is fetched unless the learner asks, and
  that is the sentence to keep true. Three things can reach the network, all
  opt-in: `asr.rs`, which downloads the ~163 MB recognition model only when the
  install button is pressed (M12); `say.rs`, on exactly the same rules (M14); and
  `sync.rs`, which reaches the learner's own Dropbox once an account is connected
  and then syncs at launch and on return — which is why it asks whether there is a
  network before it tries. Both downloads verify against a pinned SHA-256, stage
  their work so an interrupted install leaves the previous state untouched, state
  the address, size and licence before the button, and change nothing when
  declined. Sync is off by default. There is no server of ours and no account of
  ours. **Whoever adds another download must restate the README's promise — in
  `README.md`, `LICENSES.md` and the bundle's own `longDescription` — and add the
  model's licence notice the way `cpal`'s and MeloTTS's were added.** For that
  person: `tauri.conf.json`'s `longDescription` names both optional models now, and
  the same restatement is owed the next time a download is added.
- **Committing the artifact** — deliberate. The 13 MB artifact and the 17 MB
  interface font are both committed, so a clone and a CI run go from `pnpm install`
  to a build with no download. The cost is ~30 MB of binary in the repository; the
  33 MB of upstream text stays ignored. `.gitignore` records the reversal and the
  command that regenerates the artifact. The phrase clips are committed on the same
  reasoning.
- **Vocabulary list scope** — M1 shipped auto-fill for single characters, a composed
  reading for words, and hand-typed meanings; M3 changed one half of that, so a word
  in the HSK dictionary gets its real reading *and* meaning, and only an unknown word
  falls back to a composed reading with a blank meaning. The remaining intentional
  gaps (CSV import, tone sandhi, sentence segmentation) are recorded at the end of
  ROADMAP M1 and M3.
- **Word dictionary scope** — the HSK 3.0 lists, multi-character entries only, MIT
  compilation with CC-CEDICT readings and definitions (CC BY-SA 4.0). The
  share-alike obligation is real and recorded in `LICENSES.md`; it is why the
  upstream fields that would add a third licence (SUBTLEX-CH frequency, HanLP
  part-of-speech) are not bundled.
- **A word's reading is chosen by a rule, not resolved by context.** The first
  dictionary form wins unless it is a capitalised proper noun, in which case the
  first ordinary reading wins. That fixes 安 (`Ān` the surname → `ān` peaceful) but
  leaves a minority of genuinely ambiguous headwords on their less common reading
  (便宜). Without the sentence there is no context to do better, so this is
  documented rather than papered over.
- **Scheduling scope** — SM-2 behind a `Scheduler` trait, with the intervals
  recorded at the end of ROADMAP M2. FSRS was deliberately not attempted: it wants a
  review history one learner will not produce quickly, and the trait is the seam for
  revisiting it. There is no export/import for the schedule either — it is derived
  from practice, and a merge format would be guesswork.
- **Where study data lives** — the platform's application data directory, resolved
  through the platform API rather than assembled from `$HOME`, overridable with
  `--user-dir` (which wins) or `HANZI_TUTOR_DATA_DIR`. Not a `~/.hanzi-tutor` of our
  own: a Mac App Store build is sandboxed, the real home is not writable there, and
  some home-directory APIs still return the real home inside a sandbox — so a
  hand-built path fails only at save time. The format is one SQLite database,
  `hanzi.db`, imported once from the JSON documents an older build left behind.
- **The JSON documents are never removed, and nothing exports back to them.** The
  import leaves `vocabulary.json`, `progress.json` and `course-cursor.json`
  untouched on purpose — they are the only copy of the data until the database has
  it, and a rollback to an older build is then possible. Nothing writes them again,
  so they go stale the moment the app runs; that is intended, and the vocabulary
  list's own JSON export is the user-facing escape hatch.
- **Where settings live, and what an unchosen one means.** Preferences are a
  `Settings` document in `hanzi-core`, stored as rows in `hanzi.db`'s `settings`
  table, with **no row** meaning "nobody has chosen" rather than "off". That
  distinction is the whole point: the interface resolves an unchosen preference from
  the device (click-to-draw where there is a hover, dragging for a finger or a
  stylus) or from the system (the pronunciation voice) and only writes a value once
  the learner has made a choice. `src/lib/SettingsPanel.svelte` is the screen — the
  fifth sidebar entry — and three rules there are not to be undone:
  - **Only a preference with a device or system answer is a tri-state.**
    `click_to_draw` and `voice` are `Option`; `animation_pace` and `board_size` are
    plain enums with a `Default`. `Pace`/`BoardSize` carry `ALL` (the order the
    screen offers) and the scale the app applies, so a new choice cannot be added in
    one place and forgotten in the other.
  - **Clearing is spelled per preference on the wire**, because a *missing* argument
    already means "leave this one alone". Click-to-draw therefore has its own
    command (`clear_click_to_draw`), and a voice clears with `""`. Do not fold them
    into `null`.
  - **A preference at its default is stored as no row**, or the settings table stops
    being a record of decisions somebody took.
- **The voice preference only works if it reaches the speaker before the warm-up.**
  `AppState::load` sets it on the `Speaker` *before* `warm_voice` spawns. Resolution
  is not cached (only the ~1s voice *list* is), so a change takes effect on the next
  utterance; resolving first would mean the session's first utterance and the startup
  log both name a voice that is not in use. `HANZI_TUTOR_VOICE` deliberately
  **outranks** the stored preference, and `resolve_voice` in
  `src-tauri/src/speech.rs` is a pure function of (installed voices, preference,
  override) precisely so that ordering is testable without a synthesiser.
- **Cross-device sync: built (M13), off by default, and it runs by itself at launch
  and on return.** **[`ROADMAP.md`](ROADMAP.md) M13 owns the design, the tests and the
  reasoning** — read it before touching `crates/hanzi-sync` or `src-tauri/src/sync.rs`.
  The rules whose violation is silent, and which must not be undone:
  - **Never read the attempt log by `id`.** Order is `ATTEMPT_ORDER = (at, device_id,
    seq)`; `id` is this file's insertion order, so after a merge it would reorder a
    learner's reviews without saying so. `seq` is numbered per device — the pair is the
    identity, never the number alone.
  - **A closed shard is never rewritten**, and a shard's name carries no content hash,
    so a retried partial write finishes a shard rather than creating a second.
  - **The merge is a union, not a reconciliation**: two disagreeing copies of one
    `(device_id, seq)` are `SyncError::Rewritten`, never a winner.
  - **`hanzi_core::apply_attempt` is the only per-attempt update**, called by both
    `ProgressStore::record_with` and `fold_attempts`; do not inline a copy into either.
  - **Sync publishes `Db::own_attempts`, never `Db::attempts`** — after one sync the log
    holds a peer's work, and publishing that under this device's name would relabel it.
  - **Every store a sync can rewrite is reloaded afterwards**
    (`AppState::reload_after_sync`: progress, vocabulary, course cursor) **and the
    frontend re-reads too** (`refreshAfterSync`). Missing either looks like a sync that
    did nothing until a restart; for the vocabulary list it is worse, because a stale
    document's save tombstones what the sync brought in and the tombstones travel.
    `sync_auto` skips the reload **only** when nothing was attempted at all.
  - **A migrated card folds from a per-character baseline**; with none it is left alone
    and reported, never invented. **The log syncs; `hanzi.db` never does**, and sync
    must not carry `settings`.
  - **The refresh token lives in the platform's secret store, never in `hanzi.db`**,
    and drawing the settings screen never reads the keychain — "connected?" comes from a
    `meta` record. `can_lock` blocks until Android's main thread answers, so every
    caller must stay a `#[tauri::command(async)]`; a plain blocking command deadlocks
    the first settings screen.

## 8. The app bundle, and the notices inside it

This section is what M5 added. Read it before changing anything under `licences/`,
`src-tauri/src/licences.rs` or `tauri.conf.json`'s `bundle` block.

### Building it

```bash
pnpm run build            # signed .app + .dmg (scripts/build-release.sh)
pnpm run build:unsigned   # the plain Tauri build, no signing wrapper
```

`scripts/build-release.sh` resolves a signing identity in this order:
`$APPLE_SIGNING_IDENTITY`, then the first *Developer ID Application* certificate in
the keychain, then ad-hoc (`-`) — which still launches on this machine, since Apple
Silicon refuses a completely unsigned binary. The identity is **not** in
`tauri.conf.json`, because that file is committed and one person's certificate does
not belong in it.

Output, under the project's own target directory (`.cargo-target/` here, because of
`with-cargo-env.sh`; `src-tauri/target/` otherwise):

```
bundle/macos/Hanzi Tutor.app
bundle/dmg/Hanzi Tutor_0.3.0_aarch64.dmg
```

### What is inside, and why the course needs no network

| Part | How it gets in | Size |
| --- | --- | --- |
| Characters, words, stroke geometry | `include_bytes!` in `src-tauri/src/state.rs` | ~13 MB |
| Interface font, Noto Sans SC | Vite, from `src/assets/fonts/`, via `src/app.css` | ~17 MB |
| Graded phrase clips | Vite, from `public/audio/<corpus>/` | ~91 MB in the working tree; ~32 MB of it (`no7z`) is committed, and the graded readers' set is generated but not committed (ROADMAP M14) |
| SQLite, for the study store | compiled from the amalgamation by `libsqlite3-sys` | ~1.5 MB |
| 21 licence notices, over 19 files | `bundle.resources` → `Contents/Resources/licences/` | ~230 KB |

Everything the course teaches is inside the app, and that is a build-time property:
the artifact, the font and the clips are committed rather than fetched. It is **not**
a claim that the app never talks to anything — three modules can open a socket, and
every one is off until the learner chooses it: `asr.rs` (recognition model),
`say.rs` (synthesis model) and `sync.rs` (their own Dropbox account). See §7.

### The notices are pinned three ways

`src-tauri/src/licences.rs` is the catalogue. For each notice it holds the text
(`include_str!`, so it is compiled into the binary), the file under `licences/`, and
the path the bundle copies it to. `tauri.conf.json` copies the same files.
`src-tauri/tests/licences.rs` fails unless all three agree — in either direction, so
an uncatalogued file and a missing one are both failures. **Adding a notice means
editing the catalogue and the bundle config; the test is what stops you forgetting
the second one.** There are 21 notices over 19 distinct files (several notices share
one text); SQLite and `rusqlite` were the first third-party *code* in the binary,
since everything before them was data or a font. Note what the test cannot do: it
compares the catalogue, the files and the bundle config with each other, so a
dependency nobody catalogued is invisible. Adding a crate means reading its licence
by hand and adding a notice. A notice can also be owed for code compiled in and never
called, and whether it is there can differ by platform — see invariant 23.

Two copies is not redundancy for its own sake. The compiled-in copy is what the About
screen shows, and it cannot be lost in packaging. The file copy is what a
redistributor can read without launching the app, which is the conventional form of
the obligation.

### Verifying a build, by hand

There is no test for the packaged artefact, because there is no packaged artefact in
CI. After a build, check these four things:

```bash
APP=".cargo-target/release/bundle/macos/Hanzi Tutor.app"

# 1. Every notice survived as a file — 19 of them.
ls "$APP/Contents/Resources/licences" | wc -l

# 2. The font made it into the frontend bundle.
ls "$APP/Contents/Resources/assets" | grep -i noto

# 3. It is signed, and by whom.
codesign --verify --deep --strict --verbose=2 "$APP"
codesign -dv --verbose=4 "$APP" 2>&1 | grep -E "Authority|TeamIdentifier"

# 4. It opens, and the About screen shows the notices.
open "$APP"
```

A failure of step 1 or 2 is the class of bug the tests cannot see, so it is worth
doing after any change to `bundle.resources`, the font path or `vite.config.ts`. Step
4 also confirms the compiled-in notices reached the interface: the log line
`[webview] licences: 21 notices bundled` appears on stderr at startup, and the About
sidebar entry renders them.

### Releasing it

The version lives in **three files** — the workspace `version` in `Cargo.toml`,
`package.json` and `src-tauri/tauri.conf.json` — and
`tests/licences.rs::the_version_is_the_same_in_every_file_that_carries_one` fails if
any two disagree. Bump all three together: the About screen, the bundle's
`Info.plist` and the `.dmg` filename all read from them. `Cargo.lock` follows on the
next cargo run, and `pnpm test` is the check that the three still agree. Two
generated files carry a version as well and nothing tests them, so keep them in step
by hand: `gen/apple/project.yml` (and the `Info.plist` XcodeGen writes from it), and
`gen/android/app/tauri.properties`, which is autogenerated during the Android build
and derives the Play `versionCode` — `0.3.0` becomes `3000`, and Play requires it to
increase with every upload.

Then build, tag and publish:

```bash
pnpm run build                          # the signed .app and .dmg

./scripts/fetch-sherpa.sh --android     # once, if it has not been run
./scripts/with-cargo-env.sh ./scripts/tauri-cli.sh android build --apk --aab --target aarch64

git tag v0.5.0
git push origin v0.5.0
gh release create v0.5.0 --prerelease --title "0.5.0 — alpha" \
  --notes-file <notes> \
  ".cargo-target/release/bundle/dmg/Hanzi Tutor_0.5.0_aarch64.dmg" \
  "src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk" \
  "src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab"
```

Every release so far is an **alpha**, hence `--prerelease`. 0.2.0 was macOS-only on
purpose; 0.3.0 added the Android APK and AAB, because those became real release builds
— signed with the project's own upload key, and the artifact that has run on a
physical phone with speech recognition. **There was no 0.4.0**, so do not go looking
for one. 0.5.0 adds the attempt log's measures and the export and analysis over them
(§7), on top of the graded phrase audio. **iOS is still absent on purpose.** The shell
does run on a physical iPhone, but only from a `--debug` build installed with
`devicectl`: an iOS *release* build fails to link Tauri's Swift glue (§6), so there
is no IPA worth attaching until that toolchain question is settled.

### Notarisation, if the app is to leave this machine

Signing is automatic; notarisation is not attempted, because it needs Apple
credentials and uploads the build. To do it, provide either `APPLE_ID`,
`APPLE_PASSWORD` (an app-specific password) and `APPLE_TEAM_ID`, or an App Store
Connect API key (`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`), and let
the bundler staple the ticket. Without it a signed `.dmg` copied to another Mac needs
a right-click-Open the first time — the standard Gatekeeper prompt for a build Apple
has not seen, not a defect.

### What is deliberately not in the bundle

- **A recorded voice for arbitrary text.** The graded phrases ship as clips, but
  anything else is spoken by the system synthesiser. Apple's voices cannot be
  redistributed, so there is no lawful way to bundle one; the app disables the control
  with an explanation when no Chinese voice is installed, and M14's optional model is
  the answer for a platform with no voice at all.
- **Any browser-opening capability.** The licences screen shows source addresses as
  text rather than links, because opening one would need the opener plugin and a new
  permission, and would contradict the app's "nothing leaves the machine unless you
  ask" promise for no gain — the full licence texts are already bundled.

## 9. Tone practice (M11) — what to know before touching it

Tone practice records one utterance — a character or a whole word — and scores its
pitch contour against the tones asked for. **It is not speech recognition**: the pitch
half needs no model, transcribes nothing and downloads nothing. ROADMAP M11 is the
scope; the optional transcription half is M12, and §6 of
[`docs/research/ASR_TTS_CLAUDE_RESEARCH.md`](docs/research/ASR_TTS_CLAUDE_RESEARCH.md)
is the argument.

An utterance is judged in **two independent halves**, and they do not share a
precondition. The pitch is scored when the text is a word short enough to divide; the
transcription is compared when a model is installed and the readings are known. Either
can happen without the other, which is what `ToneResult.toneScored` reports — see "A
phrase with no tone target" below.

### Two modules, and the seam between them

| Module | What it owns | Why there |
| --- | --- | --- |
| `crates/hanzi-core/src/pinyin.rs` | Splitting a reading into syllables, reading a tone off a diacritic, and **tone sandhi** | Reading rules, not signal processing. Also the half M12 reuses |
| `crates/hanzi-core/src/tone.rs` | YIN, contours, syllable segmentation, DTW, scoring | Pure DSP: samples in, numbers out |

`tone.rs` knows nothing about pinyin and `pinyin.rs` knows nothing about audio. The
app joins them: `AppState::tone_target` builds a `ToneTarget` (characters, readings,
citation tones, spoken tones) and `AppState::score_tones` zips it against a
`ToneReport` (one judgement per syllable). **Both sides are one entry per syllable in
the same order** — that invariant is what lets the interface label each chart, and
`analyze` guarantees it by returning one entry per tone asked for, whatever it heard.

### Words are the reason sandhi exists here

A **character's** reading is stored as a list (`好` → `["hǎo", "hào"]`) but a **word's**
run together (`学习` → `"xuéxí"`), so a word's reading must be taken apart before
anything can be scored — and it needs the tones *colloquially*: 你好 is `3 + 3` in a
dictionary and `2 + 3` out loud. `pinyin::spoken_tones` applies the three rules
(third-before-third, 不 before a fourth tone, 一 before anything else) and both readings
are reported, so the interface can show "tone 2 (dictionary 3)". Rules 2 and 3 depend on
*which character* it is, not which tone, which is why the function takes the characters
as well as the tones. README has the table and the full argument.

**Do not "simplify" this by scoring character by character.** Sandhi is a word-level
phenomenon; a per-character loop cannot see it.

### Tuning against real voices is unfinished

The engine is tested against synthetic contours whose true F0 is known by
construction, which is the only way to test a pitch tracker — but a synthetic suite
cannot see everything, and four real defects surfaced only once a human spoke into a
microphone. Single characters are in good shape now (seven rounds of 是, all seven
scored, six judging the fall correctly); **words are the open work**:

- **Segmentation on words.** A boundary can still land in the wrong place: a segment
  holding two syllables' worth of pitch (a 750 ms "syllable", a 26-semitone range) is
  judged against the shape of one tone. That is the largest remaining source of wrong
  scores.
- **The scoring constants.** Correct falls scored anywhere from 64 to 95, and one
  round in thirteen measured as level when the learner believed they had said a
  fourth tone. Whether that is a lapse, a variation in how the fall is measured, or
  `SCORE_DECAY_ST` being wrong has not been established.

`records_from_the_real_microphone` is the instrument; it takes the character or word
on `HANZI_TUTOR_TONE_TEST` and prints per-syllable voicing and the split points,
which are the first thing to look at:

```bash
HANZI_TUTOR_TONE_TEST=是 HANZI_TUTOR_TONE_SECS=3 \
  ./scripts/with-cargo-env.sh cargo test -p hanzi-tutor --lib -- --ignored --nocapture \
  records_from_the_real_microphone
```

**That test waits for you**, and it did not always. It prints a prompt and records
nothing until Enter, then says `>>> RECORDING — say X now <<<`, records, and prints
the peak level, median pitch, voicing, the floor it was judged against and the margin;
Enter repeats, `q` finishes. Three things about it are load-bearing: the prompt is
written with `println!` and never `print!`, because Rust's stdout is line-buffered and
an unflushed prompt sits in the buffer while `read_line` blocks; the device is opened
*before* the prompt to speak, so nobody talks into a microphone that is still opening.
The voicing it prints is measured by `contour_in`, not read from the report, which
zeroes those fields when it refuses. And **a peak level of `0.0000` (or under the
printed silence gate) means nothing reached the analyser** — either nobody spoke or the
terminal has no microphone permission. The app bundle is a separate process with its own grant and can work there
while the terminal does not.

### What the real recordings changed

Five changes, all in `crates/hanzi-core/src/tone.rs`. Keep them together:

| Change | Before | After |
| --- | --- | --- |
| `PitchConfig::f_min` — which sets the analysis window | 60 Hz, window 33 ms | 80 Hz, window 25 ms |
| `MIN_VOICED_MS` | 90 ms | 30 ms |
| The frame guard | A separate `MIN_VOICED_FRAMES = 6`, unrelated to the hop | `min_voiced_frames`, derived from `MIN_VOICED_MS` and the hop |
| `track_pitch`'s hop | `window / 4` = 8.3 ms | `window / 8` = 3.1 ms |
| The splitter's floor | `MIN_SYLLABLE_FRAMES = 6` frames = 50 ms, length only | `min_voiced_frames`, as both a length floor *and* a voice floor |

Four rules came out of that work, each invisible in the source:

- **The analysis window must be short enough to hold a fast fall.** `f_min` chooses
  the window, because YIN searches two periods at the longest lag it covers: 60 Hz
  buys a 33 ms window, and inside 33 ms a real fourth tone (51) moves so far that no
  lag holds a period — so every frame came back unvoiced and a correct 是 was refused
  for having no voice in it. 25 ms (`f_min = 80`) is the value; **lower leaves the
  fast fall invisible, higher stops tracking a low voice at all** (a 90 Hz male voice
  measured 84 ms at 80 and **0 ms** at 100). `a_low_voice_is_still_tracked` is the
  test that must fail before anyone shortens the window further.
- **A gate placed inside the distribution of correct attempts will refuse correct
  attempts, however carefully the number is argued for.** `MIN_VOICED_MS` went
  90 → 60 → 50 → **30** ms, and every step but the last was argued from a model — a
  synthetic vowel, an arithmetic correction, a probe on a signal generator. At 50 ms,
  two of five natural-rate rounds of 是 were refused by a few milliseconds. 30 ms is
  ten frames at this hop and those frames overlap: their windows cover ~53 ms of
  signal between them, against the one or two frames a transient manages. Its only job
  is rejecting transients.
- **The splitter must require voice, not merely length.** A boundary's cost is the
  frame's RMS plus a penalty for carrying voice, so with two boundaries to place the
  search put both inside one silence — 31 to 34 ms apart (the `split after:`
  diagnostic's tell), exactly the minimum the old
  floor allowed — leaving the middle segment voiceless and the third holding two
  syllables (a 750 ms segment reported as one syllable, 26 semitones of range). Each
  segment now needs `min_voiced` frames of *voice*, which makes that placement
  impossible and puts the cuts at each end of the silence, where a listener hears the
  break. `find_boundaries` takes length and voice as two parameters deliberately: they
  say different things, and the first does not imply the second. Both constraints
  shrink the legal predecessors to a *prefix* of the DP row, so the single-pass running
  minimum still works; the prefix-sum of the voiced flag makes the voice constraint a
  subtraction. `a_silence_between_two_syllables_is_not_cut_twice` pins it (built with
  `say_word_pausing` — a fricative does not provoke this), and
  `a_word_never_scores_more_syllables_than_it_was_asked_for` asserts every boundary
  falls inside the voiced span.
- **One bad frame must not set the range.** `range_st` is `max − min`, so a single
  octave error — YIN locking onto half the true frequency, which creak at the end of a
  syllable provokes — turned a level tone 1 into "heard falling", and
  `Contour::is_flat` keys off `range_st`. `reject_outliers` drops any frame more than
  four semitones from the median of its neighbours *before* the gap fill, so a dropped
  frame becomes a gap `interpolate_gaps` already fills. Four semitones is generous: a
  fourth tone falls its whole twelve over 150–300 ms, well under **one** semitone
  between frames twelve milliseconds apart. A *consistent* octave error is deliberately
  left alone — the contour has its mean removed before comparison, so a track an octave
  out throughout has the right shape. Measured on the next run: tracking jumps 51 → 8,
  largest range 27.4 → 9.2 st, segments over 10 st 7 of 18 → 0 of 15.
  `one_frame_at_the_wrong_frequency_does_not_set_the_range` pins it.

**The tests could not see any of this.** Every short-syllable test builds its voice
with `say`/`say_at` — a perfect tone with three steady harmonics, which YIN finds a
period in almost anywhere — so all 32 passed while a real 是 was refused on a phone.
`say_rough` adds the cycle-to-cycle jitter real phonation has, and
`a_short_syllable_with_a_fast_fall_is_heard_at_all` is built on it: flip `f_min` back
to 60 and it fails with the learner's exact message. An even fall is measured fine
through a 33 ms window, which is precisely why those tests passed. **Prefer a rough
synthetic voice over a clean one for anything about what can be heard at all.**

Two things were tested against the same data and **ruled out** — do not re-open them
without new evidence:

- **The 16 kHz resampler.** `resample` decimates by linear interpolation with no
  anti-alias filter, and its doc comment calls the aliasing "a cosmetic problem for
  pitch". A synthetic 是 through 16 kHz direct, through 48 kHz clean, and through
  48 kHz with a broadband noise floor measured identically. A real resampler is still
  wanted for the ASR path, but not for this.
- **YIN's threshold.** Loosening it from 0.15 to 0.25 gains about 15% more frames on
  a jittery vowel and calls neither white noise nor a fricative voiced at 0.30, so it
  is a real lever and it is safe. It is left at 0.15 because the floor was the binding
  problem and changing both at once would have made the next measurement
  unattributable. **If a voice still falls short, this is the next thing to move.**

**What is left, deliberately.** The tracker still makes the jumps; `reject_outliers`
only stops them reaching the score. The survivors sit in 人, the final syllable, where
creak is most likely — a *run* of wrong frames is self-consistent, so the local median
agrees with them, and widening the window would start reading a real fourth-tone fall
as movement. The root cause is that YIN runs frame by frame with no knowledge of the
frame before it, so nothing stops it choosing a different multiple of the period. A
continuity constraint (preferring the candidate lag nearest the previous frame's
estimate) would fix it at source and improve `median_hz` too; it is a larger change to
an estimator whose tests are all synthetic, so it wants its own round with a real
voice.

### The four numbers that are judgement, not measurement

All in `crates/hanzi-core/src/tone.rs`, all named and documented, and all calibrated
against synthetic contours:

| Constant | What it is |
| --- | --- |
| `SCORE_DECAY_ST` | How fast the score falls off with shape distance. A wrong tone currently scores in the 20s–50s and a match in the 90s |
| `FLAT_ST` | Below this peak-to-peak span, in semitones, a contour is "level". Its input is `range_st`, which is `max − min` — see `reject_outliers` for why one bad frame must not reach it |
| `DECIDE_MARGIN` | How much closer one tone must be before the difference is called real rather than "uncertain" |
| `FLAT_MATCH_SCORE` / `FLAT_OFF_TARGET_SCORE` | What a level contour scores, since a level tone is settled by a rule rather than by a distance |

Two more, for the word path:

| Constant | What it is |
| --- | --- |
| `VOICED_CUT_PENALTY` | What it costs to put a syllable boundary inside voiced speech. Larger than any plausible RMS, so a boundary prefers an unvoiced frame — which is where a consonant is, and where a listener hears the break |
| `min_voiced_frames` | Shortest segment the splitter will make, and the floor the scorer applies, in one function. Derived from `MIN_VOICED_MS` and the hop. Used twice — as a length floor and as a *voice* floor |

And the floor itself, which is *not* one of the judgement constants:

| Constant | What it is |
| --- | --- |
| `MIN_VOICED_MS` | How much measured voicing a segment needs before a tone can be read from it. 30 ms, after 90 → 60 → 50 → 30. Public, because `records_from_the_real_microphone` prints it beside what a real voice measured |

Re-tune these only against real recordings, and say in the commit what they were
tuned against. `MIN_VOICED_MS` is the one re-tuned *without* one, from arithmetic and
synthetic contours; the test named above is the instrument for correcting that.
Anything that changes one floor has to change the other, which is why they are one
function.

### Three decisions that look wrong and are deliberate

README argues all three; the short form is what must not be undone.

1. **The comparison is about shape, not height**, because each contour's mean is
   removed first — register is unknowable from one syllable. A flat contour is
   therefore *accepted* for both tone 1 and a level tone 3, and this under-claims on
   purpose. Without the mean removal a falling contour scores closer to a **rising**
   template than to a level one (a test pins it), which is the worse error.
2. **Amplitude is not scored.** Contours are normalised to unit RMS before the distance
   is taken, so a shallow tone 4 scores as well as a deep one; the span is reported in
   `rangeSemitones`. Scoring depth on one syllable flagged correct speech as wrong.
3. **A level contour is not scored by distance at all.** Normalising a near-flat
   contour to unit RMS amplifies its own measurement noise into what looks like a large
   movement, so a *correct* level tone would score badly. It is settled by `FLAT_ST` and
   given a fixed score.

### The neutral tone, and a refusal that was in the wrong module

Neutral tone was refused for a long time: it is short and its pitch is set by the
syllable before it, so scoring it from one syllable looked like it needed context this
module does not have. What that missed is that "level" is still judgeable — and that the
cost was paid on 的, the most common character in the language, whose tone button could
not be pressed at all. (README's "What it deliberately does not do" still says neutral
tone is not scored; the code and this file are right.)

Three things were wrong, and only one was in `tone.rs`:

1. **The refusal lived in `pinyin.rs`.** `tone_target` returned `None` when no syllable
   carried a tone in `1..=4`. Whether a tone can be *judged* is a question about the
   analysis, and `pinyin.rs` knows nothing about that — it is the seam described above.
   The guard is gone: `pinyin.rs` builds a target from whatever tones the reading has,
   and `tone.rs` decides what it can say about them.
2. **`tone.rs` refused it in two more places.** `is_scorable` is now the one predicate
   both call sites use, so "which tones can be judged" has a single answer rather than
   `(1..=4)` written out four times.
3. **A neutral target has no template, and the code assumed every expected tone had
   one.** `score_contour`'s moving branch would have panicked on tone 5. A moving
   contour against a neutral target is now answered by a rule: any clear movement is
   wrong for a neutral tone, so the tone it moved like is named and it is marked
   off-target, with no distance taken. Check the `is_scorable` guards before teaching
   this module a sixth tone.

What is judged is that it was level. What is **not** judged is how high or how long it
was: the mean is removed from every contour because register is unknowable, and duration
is not scored at all. That limit is said to the learner every time (`NEUTRAL_LIMIT`),
because 85 for a neutral tone means less than 85 for a rising one. If that ever needs to
be more than "level", duration is the thing to add — the part of a neutral tone a
listener actually hears — and it would need real recordings to calibrate.

### A phrase with no tone target is recognised, not refused

Tone scoring stops at `MAX_TONE_SYLLABLES` because a longer run's syllable boundaries
cannot be found from energy alone. That refusal is right, and it is a refusal of the
*tone score* only — it was briefly a refusal of the whole attempt (the microphone button
was disabled, and `listen_stop` errored), which hit exactly the text a personal
vocabulary list collects, so the model could sit installed and idle while the entry the
learner most wanted to say could not be recorded.

The fix is a seam, not a feature. `heard_against` needed a `ToneTarget` only to get at
the readings wanted for each character; the tones were never used for the comparison
(they are dropped from both sides — that is what `base` is for).
`heard_against_readings` takes those readings directly, and `AppState::wanted_readings`
resolves them for any text the dataset can read, using the word's own entry when it
divides and the characters otherwise — the same resolution `tone_target` does, minus the
tones and minus the length cap.

What changed, in the order the recording meets it:

- `speech_target` answers with **`tone`** (the target, or none) and **`recognize`**
  (whether a model is installed). The button is offered when either is true, so a long
  phrase on a device with no model is still disabled — with the reason, in `sayBlocked`
  — because recording it would produce an empty answer.
- `listen_stop` never errors for the text. It scores the pitch when there is a target
  and otherwise transcribes, returning `toneScored: false`.
- `TonePanel.svelte` branches on `toneScored`: the heading, badge, charts, key and pitch
  statistics are the tone half, and a recognition-only result shows none of them.
  **Never render a zero score for an unmeasured attempt** — it reads as "you said it
  perfectly flat" rather than "this was not measured", which is why the flag exists
  instead of an empty `syllables` being sniffed for.
- `SpeechTarget.recognize` is read again when the screen changes, not only when the text
  does: the model can be installed on the settings screen while a character sits on the
  board, and the answer for that same text changes when it is.

Two things to keep true. The **readings** path must keep the alignment rule — a reading
that does not divide into one syllable per character is refused rather than compared out
of step, because a wrong split reports a syllable the learner never said. And an empty
`wanted` list is a legitimate state, not an error: the dataset could not read a
character, so the transcription is shown without a comparison.

### Android capture: `cpal`'s AAudio input starts and then never calls back

**Android does not use `cpal` for capture, and this is why.** The shape of the failure
looks exactly like a dead microphone, and nothing reports an error:

- `open_stream()` returns **`AAUDIO_OK`**, `request_start()` returns 0, and the stream
  reaches **`Started`** (state 4). It stays there.
- The data callback is **never invoked** — not once, in three seconds — so the
  recording comes back with **zero samples**, `span_ms` and `voiced_ms` are both 0, and
  the learner is told "I could not hear enough voice to judge".
- The registration is real (cpal sets `.data_callback(...)` before `open_stream`), and
  cpal's error callback is registered too — and never fires.

The emulator's logcat is what settled it, since the phone's is unreadable (§6):

```
AAudioStreamBuilder_openStream() got Legacy, devIds = [8], perf = NO, burst = 768
AAudioStreamBuilder_openStream() returns 0 = AAUDIO_OK for s#1
AAudioStream_requestStart(s#1) called
AAudioStream: setState(s#1) from 3 to 4       ← Starting → Started
… three seconds, nothing …
AAudioStream: setState(s#1) from 4 to 11      ← closed on stop
```

Setting a fixed callback size — which AAudio needs for input and cpal leaves unset, on
advice that is about *output* latency — **does not fix it**, and that was tried first.

**What replaced it.** `AudioRecord` was probed through the Kotlin bridge on the same
two devices and delivered the right number of samples on both, 19,200 for 1.2 s at
16 kHz, across five sources:

| Source | Emulator peak | Phone peak |
| --- | --- | --- |
| `DEFAULT` | 8 | 647 |
| `MIC` | 32768 | 510 |
| `VOICE_RECOGNITION` | 32768 | **2050** |
| `UNPROCESSED` | 32768 | 219 |
| `CAMCORDER` | 8 | 581 |

`VOICE_RECOGNITION` is the source used: it is tuned for speech, and on the phone it is
ten times the level of `UNPROCESSED`, which is the theoretically purer choice for pitch
and too quiet here to be one in practice. `MIC` is the fallback.

The samples do not cross the bridge as JSON — ten seconds of 16 kHz mono is 320 kB.
Kotlin writes them to `cacheDir/tone-recording.pcm` and Rust reads that file and
**deletes it**, because an app that stores only what it has to should not leave a
learner's voice in the cache.

Three things about this backend are easy to get wrong again:

- **The platform is the authority on whether a recording is live.** Kotlin's audio
  thread ends by itself at its cap or on an error, and Rust was not told — so its own
  `active` flag went stale and every later press failed with "Already listening." for
  the life of the process, reachable by holding the button past the cap once.
  `Recorder::start` on Android now asks `recordStatus` first and drops a stale flag.
- **Nothing blocks the main thread.** The commands run on it (§6), so `recordStop` does
  not wait for the audio thread: it clears the flag and the thread answers when the file
  is closed, which is also the only moment Rust can safely read it.
- **The rate is 16 kHz**, which is `tone::TARGET_SAMPLE_RATE` — so Android skips the
  resampling the `cpal` path needs rather than adding a step.

### Things that will surprise you

- **Anything measured in time here is measured from the first voiced frame.** Syllable
  boundaries were once reported as offsets from the start of the recording while
  `voiced_ms` is a *duration*, so the panel showed `Split at 1463 ms` beside
  `Voiced 308 ms` — impossible-looking, though the split was in the right place.
  `a_word_never_scores_more_syllables_than_it_was_asked_for` asserts every boundary
  falls inside the voiced span, so the two cannot drift apart again. If you add a new
  timing field, give it the same baseline.

- **`cpal::Stream` is not `Send` on every backend**, so it cannot live in Tauri's
  shared state. `capture.rs` owns one thread per recording and only plain data crosses
  back. Do not store the stream in `AppState`.
- **The microphone is opened and closed per utterance**, deliberately. It costs tens
  of milliseconds and means the system's recording indicator is lit only while the
  learner is holding the button. A latency complaint and a privacy property are the
  same line of code here.
- **`Recording` must stay `Debug`**, because `Recorder::stop()`'s error path uses
  `unwrap_err()`. That only fails in the `--lib` test target, so `cargo check` will not
  catch its removal.
- **A neutral-tone syllable is carried, and judged only on being level.** It appears
  in the target and in the result; what can and cannot be seen from one syllable is
  `tone::NEUTRAL_LIMIT`, and the learner is told it. Text past four syllables has no
  tone half at all and is recognised instead — the button is disabled only when there
  is neither a target nor a model, which is the one board a recording would answer
  nothing about.
- **Segmentation will cut where you did not mean it to.** Two syllables that run
  together with no consonant between them — a vowel-initial second syllable — have no
  unvoiced frame to cut at, and the search falls back to the quietest frame, which may
  be wrong. This is why `boundariesMs` is reported to the interface: it is the
  difference between a learner seeing a puzzling score and seeing that the app
  mis-heard where the syllables were. Anything that improves this should improve it
  *here*, and the tests to extend are the `say_word` ones.
- **A `ToneResult` carries one entry per syllable of the target when `toneScored` is
  true.** The interface zips them positionally against the characters and readings, and
  `analyze` guarantees the length on that path. With no target — text longer than a
  word — the list is empty and the transcription is the whole answer. **Read
  `toneScored`, never the list's length**, or a long phrase will be treated as a silent
  attempt; the IPC test `a_tone_result_carries_one_entry_per_syllable` covers the tone
  path and `text_too_long_for_tones_is_recognised_instead` covers the other.
- **The verdict words (`match`, `off_target`, `uncertain`) live in Rust, not in the
  panel.** `TonePanel.svelte` styles `detail`; it must not reword it, or the same judgement will be expressed in two
  places that drift.
