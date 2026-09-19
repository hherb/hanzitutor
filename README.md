# Hanzi Tutor

A desktop app for learning to **read and write simplified Chinese characters**.
You write a character with a mouse, trackpad or stylus, and the app tells you
whether it was written in the correct stroke order, whether the strokes are the
right shape, in the right place and with enough ink, and whether the result is
legible — all offline, with no model downloads and no network access at runtime.

Built with **Tauri 2 + Rust** for the engine and **Svelte 5 + TypeScript** for the
interface. Primary target is macOS; the same code builds for Windows and Linux,
and it builds for iOS today — it runs on the iPhone simulator and installs on a
physical iPhone, with a phone layout made for practice rather than for fitting.
Pronunciation on iOS and a release (rather than debug) device build are the next
steps (see [ROADMAP.md](ROADMAP.md) M9). The Rust core has no platform code at
all, which is why that cost nothing.

## Status

Working end to end. The grading engine, the dataset pipeline, the Tauri command
layer, the drawing UI, pronunciation, the personal vocabulary list, per-character
progress with spaced repetition, the HSK 3.0 **word list**, and the **raster ink
measure** and the **durable study store** are all implemented and tested; 228
automated tests pass. What is not built yet is listed under
[Next steps](#next-steps).

## What it does

- **9,574 characters** with real stroke geometry, of which **7,744** are in the
  frequency list and organised into **775 lessons** of ten.
- **9,443 HSK 3.0 words**, searchable by character, reading (with or without tone
  marks) or English meaning, and practisable straight from the list. Clicking a
  character in a word finds every word that uses it.
- **Two practice modes.** *Trace* puts a faint copy of the character on the board
  to follow. *Recall* shows only the pinyin and meaning, and grades what you
  write from memory.
- **Two ways to draw.** Press-and-drag, which is what a stylus does, or **click to
  draw**: one click starts a stroke, moving the pointer extends it, and a second
  click ends it — no button to hold down for a long stroke on a trackpad. Escape
  or Backspace abandons an unfinished stroke. Both modes put down identical
  geometry, so the grade does not depend on which one you used. On a trackpad or
  with a mouse, click-to-draw is what you get to begin with; a touch or pen device
  still starts on dragging. Flip the switch and your choice is remembered.
- **Stroke-order animation** — a pen walks each stroke's centre-line and the
  outline appears behind it, so the direction a stroke is written in is shown and
  not only the order the strokes come in. It can be stopped at any point, and
  starting to write ends it for you.
- **Grading with specific feedback**, not just a number: which stroke is the
  wrong shape, which is misplaced, which was drawn back to front, which has too
  little ink, which is missing, and which are out of order.
- **Colour-coded overlay** — your strokes are tinted by verdict, and reference
  shapes are ghosted in red where a stroke should have gone.
- **Readings and meanings** — pinyin, English gloss, radical, stroke count, HSK
  level and frequency rank, plus the etymology mnemonic where one exists.
- **Pronunciation** — hear any character or word through the system's own speech
  synthesiser. Nothing is downloaded and nothing leaves the machine.
- **Your own vocabulary list** — record the characters and words from your own
  lessons, file them under your own group names, and drill exactly those. A
  character fills in its pinyin and meaning automatically; so does a word, from
  the HSK dictionary, and a word the dictionary does not know still gets its
  reading composed from its characters. It can hold vocabulary the built-in course
  never covers. Export to JSON (lossless) or CSV for a spreadsheet.
- **An explanation you can ignore.** What the four grading measures mean is one
  quiet row — "ⓘ How this works" — that expands when asked and stays out of the
  way otherwise, which matters most on a phone, where the board is the point.
- **Progress that persists, and a review queue.** Every graded character is
  remembered — attempts, best score, a short history and a due date — whether it
  came from the course or from a word in your list. Answer well and it comes back
  later; answer badly and it comes back within the minute. The course opens where
  you left off, each lesson shows how much of it you have practised, and
  *Review due* drills what has come back, most overdue first.
- **A study store with room to grow.** The schedule, the list and your place in
  the course live in one SQLite database, and every attempt ever made is kept in
  an unbounded log rather than a twenty-entry history — which is what the grading
  tolerances will be tuned against, once there are real attempts to look at.
- **Sentences, one character at a time.** Any multi-character text written into
  the list — a word, a phrase, a sentence — is practised character by character,
  with anything the board cannot draw (punctuation, an unknown glyph) skipped
  rather than dead-ending the attempt.
- **A box per character of a word.** Under the board, a multi-character entry
  shows one box per character: a character already written appears as a miniature
  of your own attempt with the score it got, and one still to write is an empty
  box. Click any box to put that character back on the board — a written one
  comes back with its drawing and its grade, to look at again or improve, and an
  unwritten one is ready to write. The word is recorded once every character has
  been written, not necessarily in order.
- **About and licences.** The fourth screen in the sidebar names the app's own
  licence and shows the full text of every third-party licence its data and font
  are under, with what each source contributes and where the notice sits inside
  the bundle. Nothing on it is fetched, and nothing it names is downloaded: it is
  the receipt for the claims in [Data and licences](#data-and-licences).

## Quick start

Requires Rust (1.77+), Node 20+ and pnpm. On macOS you also need Xcode command
line tools.

```bash
pnpm install
pnpm run dev                # launches the app
```

There is no data step. The ~13 MB dataset artifact, the interface font and every
licence notice are committed, so a clone builds and runs offline from the first
command — and CI needs no download either. (The 33 MB of upstream text in
`data/raw/` and the build output are still gitignored for size; neither is needed
to build. `./scripts/fetch-data.sh` followed by `pnpm run prepare-data` restores
them, and is only wanted when changing the data pipeline.)

`pnpm run dev` runs Vite and the Tauri CLI together. `pnpm run build` produces a
signed `.app` and `.dmg` — see [Shipping a build](#shipping-a-build).

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

A **word** is spoken whole, which is the only way to get a polyphonic character
right. 着 on its own is read whichever way the synthesiser prefers; 着急 is
`zháojí`, and the word is what carries that context.

Override the voice with an environment variable:

```bash
HANZI_TUTOR_VOICE="Meijia" pnpm run dev
```

Enumerating voices takes about a second, so it runs on a background thread at
startup rather than on the first click.

### Your study data

Everything you do lives in one SQLite database, `hanzi.db`, in the platform's
application data directory —
`~/Library/Application Support/com.hanzitutor.app/hanzi.db` on macOS. Nothing is
sent anywhere, and nothing else is written: the database holds the vocabulary
list, the per-character schedule with its log of every attempt, and your place in
the course.

Where that directory is can be overridden, which is useful for a portable
install, for keeping study data outside the application support folder, or for
running under a sandbox that cannot write there. `--user-dir` sets it for one
run and takes precedence; `HANZI_TUTOR_DATA_DIR` does the same from the
environment:

```bash
HANZI_TUTOR_DATA_DIR="$PWD/.study" pnpm run dev

# The built app takes the flag directly (`--user_dir` is accepted too), which is
# the useful form when running a bundle instead of the dev server.
".cargo-target/release/bundle/macos/Hanzi Tutor.app/Contents/MacOS/hanzi-tutor" \
  --user-dir "$PWD/.study"
```

The resolved location is logged at startup as `[data] study files in …`, which
is the first thing worth knowing when a save misbehaves. A `--user-dir` with no
usable value stops the app with an error rather than falling back to the default,
so a testing session cannot quietly write into your real study data.

If the database cannot be opened, the app says so and **refuses to save** rather
than starting empty over it. Fix or move the file, then restart.

### Upgrading from an older version

Earlier builds kept three plain-JSON documents in the same directory: `vocabulary.json`, `progress.json` and `course-cursor.json`. The first
time the app runs against that directory it **imports** all three — vocabulary,
schedule, history and course position — and leaves the files exactly where they
are, byte for byte. Nothing is deleted, so the JSON is still there afterwards if
you want to look at it or roll back to an older build.

Two things are worth knowing about that first run:

- The import is recorded in the database (`meta`), so it happens once. After
  that the JSON files are not read again, and editing them changes nothing.
- It is per document. A `course-cursor.json` that cannot be parsed blocks the
  cursor and nothing else: the schedule and the list still load, that document is
  simply not marked as imported, and fixing it and restarting imports it. It is
  never overwritten.

### Your progress and what to review

A preference you have not chosen has **no row at all**, which is not the same as
one set to off: that is what lets the app follow the device until you decide, and
then stop second-guessing you. `settings` holds only what you have chosen.

| Table | Holds |
| --- | --- |
| `vocab_entry`, `vocab_group` | your list: entries, groups, per-entry attempts |
| `progress_card` | one row per practised character: attempts, best and last score, the interval, ease and when it is next due |
| `attempt` | **every attempt ever recorded**, in order — not a bounded history |
| `course_cursor` | where you were in the course, so the app opens there |
| `settings` | the preferences you have actually chosen, one row each |
| `meta` | the schema version and the record of the one-time import |

The card and the attempt log are deliberately different things. The card holds
what the Scheduler needs plus the newest twenty attempts for the board to show
you; the log holds everything, and nothing rewrites it. That is what the database
is for: a document that is rewritten whole on every attempt could never grow past
a bounded array, so the log had a ceiling that rows do not.

The database uses SQLite's write-ahead journal, so a write that is interrupted —
the app killed mid-save — is rolled back rather than left half-applied. The
`hanzi.db-wal` and `hanzi.db-shm` files beside it are SQLite's own; deleting them
while the app is closed is safe, deleting `hanzi.db` resets everything.

Scheduling is **SM-2**: an attempt's 0..=100 score becomes one of four ratings
(*again* / *hard* / *good* / *easy*, using the same grade bands the feedback panel
shows), and the rating sets the next interval. A failure returns within the minute;
a pass starts at twelve hours, a day or two days depending on the rating, then
stretches — one day, six days, and then by the ease factor, up to a year. The
algorithm sits behind a small `Scheduler` trait so it can be replaced (FSRS wants
far more data than one learner produces quickly) without touching the store or the
interface.

### The HSK word list

The **HSK words** screen carries the 9,443 multi-character words of the official
HSK 3.0 vocabulary, with each word's own reading and English definition. All of it
is compiled into the same offline artifact as the characters, so it costs no
network access and adds about 280 KB compressed.

Search takes any of three things, and does not care which you meant:

| You type | You get |
| --- | --- |
| `学` (a single character) | every word containing it, most common first |
| `xuexi`, `xuéxí` or `xüexi` | words whose reading matches — tone marks, spacing and `ü` are all folded away |
| `teacher` | words whose definition contains the word |

Results are one capped page of 100 with the true total reported beside it, and the
sidebar narrows everything to a single HSK level. **Practise** drills a word
without saving it; **+ List** puts it in your own vocabulary list with the reading
and meaning already filled in.

Two deliberate choices are worth knowing about:

- **Single characters are not in the word list.** They are the course's job, and
  the character dataset already carries each one's most common reading. A second
  entry for the same glyph would be a competing source of truth — the dictionary
  lists 安 as the surname `Ān` before `ān`, "peaceful".
- **The frequency shown for a word is derived, not published.** It is the rank of
  the word's rarest character, taken from the same MIT frequency list the course
  is built from: a word is no more common than its least common character. It
  exists to order the list and should not be read as a corpus frequency.

## How grading works

This is the heart of the app and the part worth understanding.

Make Me a Hanzi provides, for each character, both the **SVG outline of every
stroke** and a **centre-line ("median") for every stroke**, both stored in stroke
order. That is enough to grade handwriting as pure geometry — no machine
learning, no network, no model weights.

Asking "did you write this character correctly?" actually conflates three
independent questions, so the engine answers them separately.

### 1. Did you write the right strokes, in the right places?

Each of your strokes is scored against each reference stroke on two measures:

- **Shape** — mean distance between the two centrelines after resampling to
  equal length and normalising away position and scale. This is deliberately
  scale- and translation-invariant: it captures whether a stroke is *that kind*
  of stroke, not where or how big it is.
- **Placement** — centroid offset plus bounding-box size mismatch, measured
  absolutely in the 1024×1024 character box. The offset is measured between
  *length* centroids — the centre of the stroke as a line — rather than the mean
  of the recorded points, because points arrive with the pointer in real time:
  the same stroke drawn slowly at one end and quickly at the other would
  otherwise have its samples bunched at the slow end and be marked as misplaced
  for it.

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

### 3. Did you put down the right amount of ink?

Shape and placement are both computed from centrelines, and a centreline has no
width. A stroke that follows exactly the right path in exactly the right place but
is drawn a third of the width the character needs therefore reads as perfect to
both of them — the shape score is deliberately scale-invariant, so it cannot see
width at all, and placement only looks at where the stroke sits and how long it is.

So the engine also rasterises the attempt as **round-capped pen strokes** at the
width the canvas painted them, and compares the result with the character's own
ink — the stored SVG stroke outlines the interface already draws as the faint
guide. Two numbers come out of that:

- **Ink amount** (`inkScore`, per-stroke `ink`) — the attempt's inked area against
  the area a correct trace at the canvas pen width would put down. It is
  deliberately blind to *where* the ink went, because that is what placement
  measures; judging both would count a wobbly hand twice. A correct trace scores
  1.0, a pen a third of the width about 0.31, and a wildly overshooting stroke
  the same from the other side. This is the measure behind the `faint` verdict.
- **Ink coverage** (`inkCoverage`) — how much of the character's own ink was
  reached at all. This is reported rather than scored: a wobbly but correctly
  inked stroke genuinely misses part of the outline, and that is a placement
  fault already covered by the previous measure.

The first attempt at this used intersection-over-union against the outline, which
is the obvious choice and the wrong one, and `selfcheck` is what showed it: IoU
falls when a correctly-sized band lands slightly off the guide, so jittering a
right-width trace by 3% of the box dropped the ratio to 0.47 and marked a sloppy
hand illegible 96% of the time — from 99% before. Splitting the measure into amount
and coverage fixed it (back to 99.1%, with the ink fault still caught), and both
halves are cheap: a grade costs under 1 ms including the rasterisation.

### Aggregation

All four headline scores — shape, placement, ink and order — are measured across
the **whole** reference character, with unwritten strokes counting as zero. An
incomplete attempt is penalised consistently in all four, and a blank canvas
scores 0 rather than collecting easy marks for a flawless ordering of nothing.

The overall score gives each measure an equal quarter:
`100 × (0.25 × shape + 0.25 × placement + 0.25 × ink + 0.25 × order)`. The weights
are exact binary fractions, so a flawless attempt sums to exactly 1.0 and scores
exactly 100 rather than 99.999… — a property the interface's contract test pins.

`legible` and `order_correct` are reported independently, because they are
independent facts: a character written beautifully in the wrong order is still
legible, and is reported as legible with the ordering faults called out.
`legible` requires the shape, placement and ink means to clear their bars; order
is deliberately not part of it.

### Tolerances were measured, not guessed

`cargo run -p hanzi-core --example selfcheck` runs the engine over the whole real
dataset. Its output on the shipped data:

- **Self-consistency** — grading every character's own reference strokes against
  itself scores a perfect 100 for **all 7,744** teachable characters, on all four
  measures. Anything less would mean resampling, normalisation or the raster
  measure misbehaves on some real stroke. The ink figure is exactly 1.0 for every
  one of them, with no `faint` stroke anywhere — which is what makes "perfect"
  reachable rather than merely close.
- **Tolerance under a wobbly hand** (reference strokes jittered, then graded):

  | Jitter | Judged legible | Mean score | Shape | Ink |
  | --- | --- | --- | --- | --- |
  | 0.5% of the box | 100% | 97.6 | 0.96 | 0.97 |
  | 1.5% | 100% | 92.4 | 0.87 | 0.91 |
  | 3% | 99.1% | 84.6 | 0.75 | 0.80 |
  | 5% | 52.4% | 75.6 | 0.61 | 0.68 |

  Since input is a trackpad rather than a stylus, the shape tolerance is set
  deliberately loose so a shaky but correct attempt is never failed for shape
  alone, and the `overall` score carries the quality gradient instead. The ink
  measure follows it: a jittered trace keeps its ink *amount* (the band is the
  same width wherever it wandered) and only loses coverage.

- **Discrimination** — the nearest-wrong pairing overlaps the correct
  distribution heavily (91.9% of correct strokes sit beyond the 5th percentile of
  wrong ones), which is not a defect but the reason placement exists.
- **What the ink measure can see** — across all 7,744 teachable characters,
  a correct trace scores 1.0 on ink; a pen a third of the width scores 0.31 and
  puts every one of them below the legibility bar; a stroke drawn 40% too long
  costs a little ink (0.97) and a stroke drawn three times too long costs a lot
  (0.92 on the character mean, `faint` on the stroke itself in 1,774 characters).
  Overshoot is a proportional fault rather than a cliff, because too much ink is
  still readable — it is chiefly a placement fault, which is why the position
  score flags it too.

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
│  render.ts        Path2D,    │        │    ├── lessons, character     │
│     font↔display transforms  │◀───────│    ├── search_words           │
│  FeedbackPanel    verdicts   │  JSON  │    ├── grade_attempt          │
│  LessonSidebar    course and │        │    ├── vocab_*                │
│     words, review, progress  │        │    └── progress, review_queue │
│  WordsPanel       HSK list,  │        │         │                     │
│     search by character     │        │         ▼                     │
│  VocabularyPanel  your list  │        │  hanzi-core                   │
│                              │        │    geom   resample, distance  │
│                              │        │    grade  Hungarian + Kendall │
│                              │        │    raster pen strokes + ink   │
│                              │        │    dataset  chars + 9k words  │
│                              │        │    curriculum  frequency      │
│                              │        │    vocab    the list          │
│                              │        │    progress SM-2, due dates   │
│                              │        │    time     ISO-8601 text     │
│                              │        │         │ sink trait          │
│                              │        │         ▼                     │
│                              │        │  hanzi-store  SQLite          │
│                              │        │    hanzi.db: cards, the       │
│                              │        │    attempt log, the list      │
└──────────────────────────────┘        └──────────────────────────────┘
```

The Rust core has no UI or platform dependency, so it can be driven from a CLI, a
test harness or a mobile shell unchanged. The command layer is deliberately thin —
each `#[tauri::command]` forwards to a method on `AppState` — which is what makes
the whole webview-facing surface testable without opening a window.

The store is a crate of its own for the same reason: `hanzi-core` decides *what* to
remember and this decides *where*, behind a trait the engine defines, so SQL never
enters the engine and the engine's tests never need a database.

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
crates/hanzi-core/          engine + data, no UI dependency
  src/store trait           ProgressSink / VocabSink / CursorSink, in
                            progress.rs and vocab.rs
  src/geom.rs               resampling, normalisation, distance measures
  src/grade.rs              pairing, order analysis, verdicts, scoring
  src/dataset.rs            characters, the word dictionary, artifact loading
  src/curriculum.rs         frequency list → lessons
  src/vocab.rs              the personal vocabulary list
  src/progress.rs           per-character history, SM-2 scheduling, review queue
  src/time.rs               ISO-8601 timestamps and date arithmetic
  src/bin/prepare_data.rs   upstream data → compact artifact
  examples/selfcheck.rs     self-consistency and tolerance measurement
crates/hanzi-store/         the SQLite store. Native dependency, so it is
                            separate from the engine
  src/schema.rs             the tables, and applying them
  src/migrate.rs            the once-only import of the old JSON documents
  src/lib.rs                the sinks, and the attempt log's reader
  tests/store.rs            the M10 acceptance criteria
src-tauri/                  Tauri shell
  src/commands.rs           the IPC surface
  src/state.rs              embedded dataset, speech warm-up, the three stores
  src/speech.rs             pronunciation via the system synthesiser
  src/licences.rs           the notices that ship
  tests/ipc_contract.rs     locks the JSON contract the UI reads
src/lib/                    Svelte components
  PracticeCanvas.svelte     pointer capture, stroke recording
  render.ts                 canvas painting, the stroke-order sweep, verdict colours
  WordsPanel.svelte         the HSK word list: search, browse, practise
  VocabularyPanel.svelte    the vocabulary list: add, group, export, import
  LessonSidebar.svelte      course, list and word navigation, progress marks
scripts/                    data fetching, cargo env, CLI selection
```

## Testing

```bash
pnpm test             # the whole Rust suite: 228 tests
pnpm run test:core    # just the engine, store and data-pipeline unit tests
pnpm run selfcheck    # engine behaviour over the whole real dataset
pnpm run check:web    # svelte-check
pnpm run check:rust   # clippy, warnings denied
```

`pnpm test` enables hanzi-core's `prepare` feature so that the data pipeline's
parsing — which upstream fields are trusted, and how a word's reading is chosen —
is covered by the same run as everything else.

The tests that earned their place:

- the Hungarian solver is checked against **brute-force optimal assignment** on
  random matrices up to 6×6, not just on hand-picked cases;
- the IPC contract tests assert the exact set of JSON keys the TypeScript client
  reads, because a field-name mismatch fails *silently* — the UI would just show
  blanks;
- `selfcheck` compares the engine against all 7,744 real characters, which is how
  the 黧 stray-tap bug and the scoring bugs were found. It also resamples every
  stroke with realistically uneven density and requires that no verdict changes,
  because synthetic jitter preserves sampling density and so cannot see a
  placement metric that depends on how fast the pointer moved — which is exactly
  the bug that section now guards;
- the speech tests use the **genuine** voice-list strings. A fixture with tidy
  names (`Tingting`) passed while the real list (`Tingting (Chinese (China
  mainland))`) never matched the preference, so the app silently used a
  different voice;
- the scheduler tests pin every interval and due date, and a second `Scheduler`
  implementation is exercised to prove the policy really is swappable;
- persistence is tested through the **state layer** and the real file names as
  well as the store, including the case that matters most: a corrupt document is
  reported and the file on disk is left byte-for-byte unchanged;
- the store's own tests start from **real saved files**, not fixtures: they write
  the JSON documents with the app's own stores, open the database over them, and
  check the import, the markers, the untouched bytes, the unbounded log past the
  twenty a card shows, and that an uncommitted write leaves nothing behind;
- the word tests run against the **shipped artifact**, not fixtures: every one of
  the 9,443 words is checked to be drawable character by character, 着急 is
  checked to read `zháojí` where the isolated 着 does not, and an unknown pairing
  of real characters is checked to still invent no meaning.

## Shipping a build

```bash
pnpm run build              # signed .app and .dmg
pnpm run build:unsigned     # the plain Tauri build, if you would rather not sign
```

`scripts/build-release.sh` picks a codesigning identity in this order: whatever
is in `$APPLE_SIGNING_IDENTITY`, then the first *Developer ID Application*
certificate in your keychain, then ad-hoc (`-`) as a last resort — which still
produces a bundle that launches on this machine, because Apple Silicon refuses to
run a completely unsigned binary. The identity is deliberately not written into
`tauri.conf.json`: that file is committed, and one person's certificate does not
belong in it. Nothing here notarises; see below.

The result lands in `.cargo-target/release/bundle/` — in this checkout the
project's own target directory, or `src-tauri/target/` on a machine that has not
set `CARGO_TARGET_DIR`:

```
bundle/macos/Hanzi Tutor.app
bundle/dmg/Hanzi Tutor_0.1.0_aarch64.dmg
```

### What is inside the bundle

Everything. There is nothing to download on first run and nothing to install
besides the app itself:

| Part | How it gets in | Size |
| --- | --- | --- |
| Characters, words, stroke geometry | `include_bytes!` in `src-tauri/src/state.rs` | ~13 MB |
| The interface, including the Noto Sans SC font | Tauri embeds `frontendDist` into the executable | ~18 MB |
| Twelve licence notices, as plain text | `bundle.resources` → `Contents/Resources/licences/` | ~65 KB |

So the executable is about 35 MB and `Contents/Resources/` holds only the icon
and the notices. The notices are **also** compiled into the binary, which is why
the About screen cannot come up blank in a packaged build: the loose files are
for a redistributor who wants to read them without launching the app. Both copies
come from the same source file at build time, and a test requires them to agree,
so they cannot drift.

Check the copies survived a build — a resource path is exactly the kind of thing
that breaks only in the packaged app:

```bash
APP=".cargo-target/release/bundle/macos/Hanzi Tutor.app"
ls "$APP/Contents/Resources/licences"       # ten files, named in src-tauri/src/licences.rs
ls -lh "$APP/Contents/MacOS/hanzi-tutor"    # ~35 MB: the data and the font are in here
codesign -dv --verbose=4 "$APP" 2>&1 | grep -E "Authority|TeamIdentifier"
open "$APP"                                 # then look at About and licences
```

The tests in `src-tauri/tests/licences.rs` are what keep the catalogue, the files
on disk and the bundle config in step; the four commands above are the part they
cannot cover.

### Notarisation

Notarisation is not attempted, because it needs Apple credentials and uploads the
build. To do it, either set `APPLE_ID`, `APPLE_PASSWORD` (an app-specific
password) and `APPLE_TEAM_ID`, or an App Store Connect API key in
`APPLE_API_ISSUER` / `APPLE_API_KEY` / `APPLE_API_KEY_PATH`, and let the bundler
staple the ticket. Until then a signed-but-unnotarised `.dmg` copied to another
Mac needs a right-click-Open the first time, which is the standard Gatekeeper
prompt for a build Apple has not seen.

### CI

`.github/workflows/ci.yml` runs `pnpm test`, `pnpm run check:rust` and
`pnpm run check:web` on every push to `main` and every pull request, on a macOS
runner. It needs no data step, because the artifact is committed. A Linux runner
would work too, but `cargo test --workspace` would first need Tauri's system
dependencies (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`,
`librsvg2-dev`, `patchelf`).

## Data and licences

Hanzi Tutor's own source code is licensed under the **GNU Affero General Public
License, version 3** (see [`LICENSE`](LICENSE)). It bundles third-party **data**,
one third-party **font**, and — since the study store became a database — one
third-party **library**: SQLite, compiled in from the vendored amalgamation.
Every one of them carries a notice obligation. See
**[LICENSES.md](LICENSES.md)**.

| Bundled | Source | Licence |
| --- | --- | --- |
| Stroke outlines and centrelines | Make Me a Hanzi | Arphic Public License |
| Etymology hints | Make Me a Hanzi `dictionary.txt` | LGPL-3.0-or-later |
| Frequency rank, pinyin, meaning, radical, HSK | hanziDB.csv | MIT |
| Word list, HSK 3.0 levels, derived rank | complete-hsk-vocabulary | MIT |
| Word readings and definitions | CC-CEDICT | CC BY-SA 4.0 |
| Interface font, Noto Sans SC | noto-cjk / Google Fonts | SIL OFL 1.1 |
| The study database engine, SQLite 3.45.0 | sqlite.org, via `libsqlite3-sys` | Public domain |
| The SQLite bindings, `rusqlite` | rusqlite | MIT |

The generated artifact **is committed** (about 13 MB), so a clone and a CI run
need no data step; `./scripts/fetch-data.sh` followed by `pnpm run prepare-data`
regenerates it when the pipeline changes. Every notice it obliges ships as a
compiled-in text *and* as a file in `Resources/licences/`, catalogued in
`src-tauri/src/licences.rs` and readable from the app's **About and licences**
screen. Note that the **word readings and definitions carry a share-alike
licence** (CC BY-SA 4.0), which is the one obligation here that reaches the
derived data rather than only the notices — see [`LICENSES.md`](LICENSES.md).

## Next steps

See **[ROADMAP.md](ROADMAP.md)** for what to build next, in priority order, with
approach notes and acceptance criteria. Distribution is done — the notices ship
in the bundle, the data and font need no download, and CI runs the suite on every
push — and so are the two interface milestones: the stroke-order animation now
draws each stroke along its centre-line, and the board draws either by dragging
or by clicking. The headline gaps are now:

1. **Pronunciation on Windows and Linux**, so the app is not macOS-only.
2. **Mobile shells**, since a touchscreen with a stylus is the right input device.
3. **The durable study store** (SQLite) and the unbounded attempt log it exists
   for, which is also what the grading tolerances want tuned against.

If you are picking this project up to continue development, read
**[HANDOVER.md](HANDOVER.md)** first — it covers the build environment, the
invariants that must not be broken, and the traps that cost time.
