# Hanzi Tutor

A desktop app for learning to **read and write simplified Chinese characters**.
You write a character with a mouse, trackpad or stylus, and the app tells you
whether it was written in the correct stroke order, whether the strokes are the
right shape and in the right place, and whether the result is legible — all
offline, with no model downloads and no network access at runtime.

Built with **Tauri 2 + Rust** for the engine and **Svelte 5 + TypeScript** for the
interface. Primary target is macOS; the same code builds for Windows and Linux,
and the Rust core is written to be reusable from a mobile shell later.

## Status

Working end to end. The grading engine, the dataset pipeline, the Tauri command
layer, the drawing UI and pronunciation are all implemented and tested; 50
automated tests pass. What is not built yet is listed under
[Next steps](#next-steps).

## What it does

- **9,574 characters** with real stroke geometry, of which **7,744** are in the
  frequency list and organised into **775 lessons** of ten.
- **Two practice modes.** *Trace* puts a faint copy of the character on the board
  to follow. *Recall* shows only the pinyin and meaning, and grades what you
  write from memory.
- **Stroke-order animation** — step through the character one stroke at a time.
- **Grading with specific feedback**, not just a number: which stroke is the
  wrong shape, which is misplaced, which was drawn back to front, which is
  missing, and which are out of order.
- **Colour-coded overlay** — your strokes are tinted by verdict, and reference
  shapes are ghosted in red where a stroke should have gone.
- **Readings and meanings** — pinyin, English gloss, radical, stroke count, HSK
  level and frequency rank, plus the etymology mnemonic where one exists.
- **Pronunciation** — hear any character through the system's own speech
  synthesiser. Nothing is downloaded and nothing leaves the machine.

## Quick start

Requires Rust (1.77+), Node 20+ and pnpm. On macOS you also need Xcode command
line tools.

```bash
./scripts/fetch-data.sh     # ~33 MB of upstream data
pnpm install
pnpm run prepare-data       # builds the compact dataset artifact (~13 MB)
pnpm run dev                # launches the app
```

`pnpm run dev` runs Vite and the Tauri CLI together. `pnpm run build` produces a
bundled `.app` / `.dmg`.

### This checkout uses a project-local Cargo home

Cargo normally writes its registry cache and build output to `~/.cargo` and
`./target`. Under a restricted file sandbox those writes are refused, so every
cargo invocation in this project goes through `scripts/with-cargo-env.sh`, which
points `CARGO_HOME` at `.cargo-home/` and `CARGO_TARGET_DIR` at `.cargo-target/`
inside the project. Both are gitignored. Nothing else changes: the scripts work
identically on an unrestricted machine.

If you want your existing global crate cache to be reused, seed the project one:

```bash
mkdir -p .cargo-home/registry
cp -Rc ~/.cargo/registry/cache/* .cargo-home/registry/cache/   # APFS clonefile
```

### The Tauri CLI

`pnpm run dev` and `pnpm run build` go through `scripts/tauri-cli.sh`, which
probes the standard npm CLI and falls back to a standalone `cargo-tauri`. The npm
CLI is a native napi addon that parses the *operating system* process arguments,
which breaks on a host that runs Node inside another application (the DSH desktop
app's pnpm shim launches Electron with `ELECTRON_RUN_AS_NODE=1`, so the CLI sees
the host's own argv). To make this project self-contained:

```bash
pnpm run install:cli   # installs a matching tauri-cli into .cargo-tools/
```

### Pronunciation

Speaking a character uses the operating system's own synthesiser — on macOS,
`say` — so nothing is downloaded and no audio leaves the machine. The voice is
chosen automatically: mainland Mandarin (`zh_CN`) is preferred and, within that,
the long-standing `Tingting` voice, with other Chinese locales as fallbacks. If
no Chinese voice is installed the control is disabled with an explanation rather
than reading the character aloud in English.

The **character** is spoken rather than its pinyin: `say` has a Chinese lexicon,
so 汉 is read correctly, whereas an English-trained voice handed `hàn` would be
guessing at the diacritics. This also makes the control safe in recall mode —
hearing the sound does not give away the glyph, so it doubles as a dictation
exercise.

Override the voice with an environment variable:

```bash
HANZI_TUTOR_VOICE="Meijia" pnpm run dev
```

Enumerating voices takes about a second, so it runs on a background thread at
startup rather than on the first click.

## How grading works

This is the heart of the app and the part worth understanding.

Make Me a Hanzi provides, for each character, both the **SVG outline of every
stroke** and a **centre-line ("median") for every stroke**, both stored in stroke
order. That is enough to grade handwriting as pure geometry — no machine
learning, no network, no model weights.

Asking "did you write this character correctly?" actually conflates two
independent questions, so the engine answers them separately.

### 1. Did you write the right strokes, in the right places?

Each of your strokes is scored against each reference stroke on two measures:

- **Shape** — mean distance between the two centrelines after resampling to
  equal length and normalising away position and scale. This is deliberately
  scale- and translation-invariant: it captures whether a stroke is *that kind*
  of stroke, not where or how big it is.
- **Placement** — centroid offset plus bounding-box size mismatch, measured
  absolutely in the 1024×1024 character box.

Those costs form a matrix, and the globally cheapest one-to-one pairing is found
with the **Hungarian algorithm** (Kuhn–Munkres, O(n³), written from scratch and
verified against brute force for all matrices up to 6×6). Strokes on either side
may go unmatched, at a penalty higher than any real pairing, so a missing or
extra stroke is reported as such instead of being forced onto a partner.

Shape and placement are complementary, and the measurements show why both are
needed. Across the real dataset, the nearest *wrong* stroke of the same character
is often nearly as close in shape as the right one (two 横 strokes differ only in
length and position, and shape ignores both). Discrimination therefore leans on
placement; shape is what catches a stroke of the wrong kind entirely, which sits
around 0.8 normalised distance versus roughly 0.16 for a correct stroke drawn
with a realistic hand wobble.

### 2. Did you write them in the right order?

The pairing assigns each of your strokes a reference index. Read those indices in
the order you drew them, and the **normalised inversion count** (Kendall tau
distance) gives the order score: 1.0 for a perfect sequence, 0.0 for a character
written completely backwards.

Individually flagged strokes are exactly those that participate in an inversion —
both halves of a swap, not an arbitrary one. Defining it that way makes the
per-stroke flags and the aggregate score provably consistent.

An earlier version used "longest increasing subsequence ÷ stroke count". It was
wrong in a way worth recording: a three-stroke character written *backwards* still
contains a trivially increasing subsequence of length one, so it scored a
flattering ⅓.

### Aggregation

All three headline scores — shape, placement and order — are measured across the
**whole** reference character, with unwritten strokes counting as zero. An
incomplete attempt is penalised consistently in all three, and a blank canvas
scores 0 rather than collecting easy marks for a flawless ordering of nothing.
The overall score is `100 × (0.70 × content + 0.30 × order)`, where
`content = 0.6 × shape + 0.4 × placement`.

`legible` and `order_correct` are reported independently, because they are
independent facts: a character written beautifully in the wrong order is still
legible, and is reported as legible with the ordering faults called out.

### Tolerances were measured, not guessed

`cargo run -p hanzi-core --example selfcheck` runs the engine over the whole real
dataset. Its output on the shipped data:

- **Self-consistency** — grading every character's own reference strokes against
  itself scores a perfect 100 for **all 7,744** teachable characters. Anything
  less would mean resampling or normalisation misbehaves on some real stroke.
- **Tolerance under a wobbly hand** (reference strokes jittered, then graded):

  | Jitter | Judged legible | Mean score | Shape |
  | --- | --- | --- | --- |
  | 0.5% of the box | 100% | 97.6 | 0.96 |
  | 1.5% | 100% | 92.7 | 0.87 |
  | 3% | 99.1% | 85.3 | 0.75 |
  | 5% | 53.7% | 77.1 | 0.61 |

  Since input is a trackpad rather than a stylus, the shape tolerance is set
  deliberately loose so a shaky but correct attempt is never failed for shape
  alone, and the `overall` score carries the quality gradient instead.

- **Discrimination** — the nearest-wrong pairing overlaps the correct
  distribution heavily (91.9% of correct strokes sit beyond the 5th percentile of
  wrong ones), which is not a defect but the reason placement exists.

One bug this found: a single character (黧) has a genuinely tiny 4-unit stroke,
which a fixed "is this a stray tap?" cutoff discarded, making a correct attempt
impossible to score. The cutoff is now relative to the character's own shortest
stroke as well as absolute.

## Architecture

```
┌──────────────────────────────┐        ┌──────────────────────────────┐
│  Svelte 5 + TypeScript       │        │  Rust                        │
│                              │        │                              │
│  PracticeCanvas   pointer →  │ invoke │  commands (thin IPC shell)    │
│    display space 0..1024     │───────▶│    ├── dataset_stats          │
│  render.ts        Path2D,    │        │    ├── lessons                │
│     font↔display transforms  │◀───────│    ├── character              │
│  FeedbackPanel    verdicts   │  JSON  │    └── grade_attempt          │
│  LessonSidebar    course     │        │         │                     │
└──────────────────────────────┘        │         ▼                     │
                                        │  hanzi-core                   │
                                        │    geom   resample, distance  │
                                        │    grade  Hungarian + Kendall │
                                        │    dataset  13 MB artifact    │
                                        │    curriculum  frequency      │
                                        └──────────────────────────────┘
```

The Rust core has no UI or platform dependency, so it can be driven from a CLI, a
test harness or a mobile shell unchanged. The command layer is deliberately thin —
each `#[tauri::command]` forwards to a method on `AppState` — which is what makes
the whole webview-facing surface testable without opening a window.

### Coordinate systems

Two systems meet in this codebase, and keeping them straight is most of the
subtlety:

- **font space** — the published Make Me a Hanzi outlines. The character box's
  upper-left is `(0, 900)` and its lower-right is `(1024, -124)`, so y *increases
  upwards*. Outlines are stored this way because they are rendered verbatim as
  canvas paths, via `scale(1,-1) translate(0,-900)`.
- **display space** — a 1024×1024 box with y increasing *downwards*, matching the
  canvas. All grading happens here.

`Point::from_font` converts between them, and the conversion happens once at data
preparation so the grader never has to think about the flip.

## Project layout

```
crates/hanzi-core/          grading engine, no UI dependency
  src/geom.rs               resampling, normalisation, distance measures
  src/grade.rs              pairing, order analysis, verdicts, scoring
  src/dataset.rs            the character model and artifact loading
  src/curriculum.rs         frequency list → lessons
  src/bin/prepare_data.rs   upstream data → compact artifact
  examples/selfcheck.rs     self-consistency and tolerance measurement
src-tauri/                  Tauri shell
  src/commands.rs           the IPC surface
  src/state.rs              embedded dataset, speech warm-up
  src/speech.rs             pronunciation via the system synthesiser
  tests/ipc_contract.rs     locks the JSON contract the UI reads
src/lib/                    Svelte components
  PracticeCanvas.svelte     pointer capture, stroke recording
  render.ts                 canvas painting, verdict colours
scripts/                    data fetching, cargo env, CLI selection
```

## Testing

```bash
pnpm test             # the whole Rust suite: 50 tests
pnpm run test:core    # just the 29 engine unit tests
pnpm run selfcheck    # engine behaviour over the whole real dataset
pnpm run check:web    # svelte-check
pnpm run check:rust   # clippy, warnings denied
```

The tests that earned their place:

- the Hungarian solver is checked against **brute-force optimal assignment** on
  random matrices up to 6×6, not just on hand-picked cases;
- the IPC contract tests assert the exact set of JSON keys the TypeScript client
  reads, because a field-name mismatch fails *silently* — the UI would just show
  blanks;
- `selfcheck` compares the engine against all 7,744 real characters, which is how
  the 黧 stray-tap bug and the scoring bugs were found;
- the speech tests use the **genuine** voice-list strings. A fixture with tidy
  names (`Tingting`) passed while the real list (`Tingting (Chinese (China
  mainland))`) never matched the preference, so the app silently used a
  different voice.

## Data and licences

Hanzi Tutor's own source code is licensed under the **GNU Affero General Public
License, version 3** (see [`LICENSE`](LICENSE)). The app bundles no third-party
code, but it does bundle third-party **data**, and under terms that carry notice
obligations. See **[LICENSES.md](LICENSES.md)**.

| Data | Source | Licence |
| --- | --- | --- |
| Stroke outlines and centrelines | Make Me a Hanzi | Arphic Public License |
| Etymology hints | Make Me a Hanzi `dictionary.txt` | LGPL-3.0-or-later |
| Frequency rank, pinyin, meaning, radical, HSK | hanziDB.csv | MIT |

The generated artifact is not committed; `./scripts/fetch-data.sh` followed by
`pnpm run prepare-data` rebuilds it. Upstream licence texts are fetched into
`data/raw/` and must ship with any distribution.

## Next steps

Not yet built, roughly in the order they would pay off:

1. **Progress that persists.** Lesson completion, per-character history and a
   spaced-repetition schedule. Nothing is stored between launches today.
2. **Reading practice with words and sentences.** The app teaches single
   characters; real reading needs vocabulary in context. This would also
   disambiguate polyphonic characters, which today are always spoken with the
   synthesiser's preferred reading.
3. **A stricter legibility measure.** The current one is centreline geometry.
   Rasterising the user's strokes and comparing coverage with the reference
   outline (IoU) would catch errors centreline matching misses, such as a stroke
   that is the right path but far too thin.
4. **Stroke-order animation along the centreline** rather than a cumulative
   reveal.
5. **Pronunciation on Windows and Linux.** Only macOS is implemented; the other
   platforms return a clear "not implemented" error rather than shelling out to
   something unverified. `spd-say` and PowerShell's `System.Speech` are the
   obvious backends.
6. **A trackpad-friendly input mode.** Long strokes are awkward to draw while
   holding a trackpad button down.
7. **Mobile shells.** The Tauri config and the Rust core are already arranged for
   it; the canvas needs touch and stylus handling verified.
