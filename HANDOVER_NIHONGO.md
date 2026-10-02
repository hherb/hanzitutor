# Handover — the Japanese part

This is the Japanese counterpart to [`HANDOVER.md`](HANDOVER.md), and it is the
document that describes it in full. The repository root now points here: the
`README.md` names the Japanese part in its "Next steps", `HANDOVER.md` lists this
file in its "Read in this order", and `ROADMAP.md` says plainly that the Japanese
work is not on its milestones. Until recently none of those was true —
`grep -c 'nihongo\|kana'` over the three returned **0**, so the work was
unfindable from the root.

The two documents divide the same way the Chinese ones do. This file is what you
need to *work on it*: the build, the shape of the code, the invariants that must
not break, how to verify a change, and the traps that cost time. The roadmap is
what to build *next*.

**The invariants in §4 are numbered in their own series.** "Invariant 3" here is
not invariant 3 of `HANDOVER.md`. Where a Chinese invariant governs the Japanese
code too — the engine is shared — the text says which.

---

## 1. Get a working build first

Everything is committed, including the dataset artifact, so a clone builds
without fetching anything.

```bash
# From the repository root.
pnpm install                       # root: the shared scripts and the check tooling
pnpm --dir apps/nihongo-tutor install

# The engine and the kana data.
./scripts/with-cargo-env.sh cargo test -p nihongo-core -p nihongo-tutor

# The app, on a display.
pnpm --dir apps/nihongo-tutor run dev      # dev server on :1422, and the window
```

There is no data step. `crates/nihongo-core/data/kana.bin.gz` is committed
(70,917 bytes, 177 kana) precisely so that it does not need one — see invariant 1.

If you *do* need to regenerate it, that is the one path that fetches:

```bash
./scripts/fetch-data.sh        # also fetches the Chinese data; both are skipped if present
pnpm run prepare-kana
```

`fetch-data.sh` pulls three things for the Japanese part, into `data/raw/`
(which is gitignored): `graphicsJaKana.txt`, 177 SVGs under `svgsJaKana/`, and
177 KanjiVG SVGs under `kvgJa/`. The last is **not** an input to the artifact —
it is the oracle the stroke counts are checked against, and without it
`prepare-kana` refuses to run. See §4.3.

### The root `Cargo.toml` is the only place the workspace is declared

`crates/nihongo-core` and `apps/nihongo-tutor/src-tauri` are members. Removing
either line silently drops the crate from every `cargo test --workspace` run
while leaving it on disk, which is a failure mode worth knowing about because
nothing complains.

---

## 2. What already works

Measured, not remembered. `cargo test -p nihongo-core -p nihongo-tutor` is **134
tests**; adding `pnpm run test:web` brings in 11 more over the board arithmetic.

| | |
| --- | --- |
| Kana | 177 — 86 hiragana (U+3041–3096), 91 katakana (U+30A1–30FA plus ー) |
| Strokes | 516, average 2.92; あ is 3, not the 4 upstream stores (§4.2) |
| Course | 18 hiragana lessons, 20 katakana, 38 in all |
| Yōon | 33 digraphs per script, 11 bases × 3 |
| Confusions | 13 pairs, each with the one feature that tells them apart |
| Tests | nihongo-core 84, nihongo-tutor 50, frontend 11 of the 80 |
| Artifact | 70,917 bytes, gzip + magic + postcard |
| Learner data | one file, `confusions.json`, in the app's own data directory |
| Version | both crates 0.1.0, and the app's to match |

The app runs and has been looked at, all of it: the course, the board, stroke-order
animation, handwriting grading, the discrimination drill, the typing box, the
katakana tab and the Licences panel have each been seen working on a display.

**The drill is the one part that learns anything about its learner** (roadmap N3).
It used to draw a kana from a pool and throw the answer away; it now draws a *pair*,
weighted by how often this learner has missed it, and records what was answered in
`confusions.json` under the app's own data directory. The rule, the record and the
reason it is not a scheduler are in `nihongo_core::drill`, and invariant 15 is the
part that must not be undone: the file is this app's, and no other app reads it.

**Three of the surfaces were watched for the first time in the session before this
one, and two of them were broken.** The test suite was green for both, which is the
point of watching:

* **The Grade button could not grade anything.** The interface posts
  `{ inkWidth }` and nothing else — deliberately, because the other three tunables
  are the values the tolerance study was fitted against — and `GradeOptions` had
  only `ink_width` defaulted on deserialisation, so *every* attempt came back
  `invalid args 'options' for command 'grade_attempt': missing field 'resampleK'`.
  The verdict panel, the four scores and the stroke colours had therefore never
  been on screen. Fixed in `grade.rs` with a struct-level `#[serde(default)]`, and
  `ipc_contract.rs` now deserialises the options the way the command does, which
  is the half of the contract it was not checking. Invariant 14.
* **The board threw `effect_update_depth_exceeded` on mount and the window came up
  blank** — about half the cold starts, and every later reload looked fine, so it
  read as a flaky webview rather than a bug in the board. The reset effect wrote
  `strokes`/`ghostCount`/`sweep` and then called `paint()`, which reads them, so
  the effect depended on what it wrote. Trap 9.

### What is genuinely reusable, and was reused

The engine was not reimplemented, and that is the whole reason this was feasible:
`hanzi-core`'s `geom`, `raster` and `grade` grade strokes against strokes and
never look at the character, so a handwritten あ is judged by exactly the code
that judges 一. Measured when this was planned: `geom` (450 lines), `raster`
(624), `grade` (1,695) and `time` (311) have **zero** Chinese coupling, and
`hanzi-sync` depends only on language-neutral progress types.

`hanzi-store` and `hanzi-sync` are **not wired up yet** and are the obvious next
reuse — see `ROADMAP_NIHONGO.md` N2.

---

## 3. Where the code lives

```
crates/nihongo-core/              the kana data layer. No UI, no Tauri.
  src/kana.rs            (499)    Script, Kana, KanaDataset, the artifact format,
                                  segment_to_stroke and merge_strokes — the repair
  src/readings.rs        (386)    Hepburn and Kunrei-shiki, and the enumeration
                                  hiragana_with_readings the input table is built from
  src/curriculum.rs      (606)    the gojūon rows, lessons, yōon, and the confusions
  src/input.rs           (523)    romaji → kana, and the one-kana and whole-word checks
  src/drill.rs           (377)    the pair weights, the tally, and the draw — N3
  src/lib.rs              (56)    re-exports, including hanzi-core's grade
  src/bin/prepare_kana.rs(246)    AnimCJK + KanjiVG → the artifact
  data/kana.bin.gz                COMMITTED, 70,917 bytes
  tests/kana_artifact.rs (261)    12 tests over the committed artifact

apps/nihongo-tutor/               the app
  src-tauri/src/lib.rs   (836)    AppState and the eleven commands, all thin
  src-tauri/src/store.rs (280)    confusions.json: load, record, atomic write — N3
  src-tauri/src/licences.rs(150)  the notice catalogue, with the text compiled in
  src-tauri/tests/ipc_contract.rs (368)  16 tests locking the JSON the webview
                                  reads *and the arguments it posts* (invariant 14)
  src-tauri/tests/licences.rs     (246)  the three-way notice check
  src-tauri/licences/AnimCJK-COPYING.txt  the one notice specific to this app
  src/App.svelte         (312)    the three views, the course, the board wiring
  src/lib/KanaCanvas.svelte(243)  pointer capture and the animation frame
  src/lib/ConfusionDrill.svelte(225)  the drill: one pair, one answer, remembered
  src/lib/LicencesPanel.svelte(116)   the notices, fetched over IPC
  src/lib/board.ts        (97)    the board's arithmetic, as pure functions
  src/lib/board.test.ts   (99)    run by the ROOT project's vitest
  src/lib/render.ts      (547)    LIFTED FROM THE CHINESE APP, UNCHANGED (§4.6)
  src/lib/types.ts       (198)    the IPC shapes, and `Character = Kana`
  src/lib/api.ts         (117)    one wrapper per command
```

`src-tauri/gen/schemas/` is committed, as it is for the other two apps.

---

## 3a. Working conventions

The two rules in `HANDOVER.md` §3a — **print long jobs rather than starting
them**, and **checkpoint long passes every item** — apply here unchanged. Nothing
in the Japanese part takes long enough to need either yet: regenerating the
artifact is a couple of seconds.

One thing that is specific to this part: **the window-capture recipe in §5 is how
you see whether the interface works**, and it is worth using rather than asking
someone to describe what they see. This part was developed without a display for
most of its life, and the two bugs that shipped — a rectangular board and a
heading behind the title bar — were both invisible to a green test suite and
obvious in one screenshot.

---

## 4. Invariants — do not break these

### 1. **The kana artifact is committed, and that is the opposite of the Chinese one.**

`crates/nihongo-core/data/kana.bin.gz` is in the repository. The Chinese artifact
is generated and gitignored (`HANDOVER.md` invariant 7). Do not "fix" this to
match: the Japanese one is 71 KB, and committing it means a clone builds with no
data step at all.

`src-tauri/build.rs` checks the file exists before `tauri_build::build()`, so a
missing artifact is a readable message rather than a cryptic `include_bytes!`
error.

### 2. **A kana is stored with the strokes it is *taught* with, and the grouping comes from AnimCJK's SVG element ids.**

This is the repair that took the most care and the one most likely to be undone
by accident.

AnimCJK animates a kana whose stroke crosses itself by splitting that stroke into
several drawing segments, and `graphicsJaKana.txt` stores one entry per
*segment*. So あ arrives with **four** strokes where three are taught. Taken at
face value the grader looks for a stroke the learner was never taught to draw,
flags a correct three-stroke あ as missing one, and scores the order wrongly.

**Twenty-five kana are affected, not twenty-one.** A first pass looked for
medians landing outside the design box and missed す, ず, ね and の, whose
displaced medians stay inside it. The authoritative answer comes from the SVG
ids — `z12354d3a` and `z12354d3b` are two segments of taught stroke 3 — read by
`segment_to_stroke`, and folded by `merge_strokes`:

* **outlines concatenate**; the segments are genuine complementary halves of the
  shape;
* **medians keep the first**; the later ones are copies displaced out of the box
  so that their dash animation does not show. Measured over all 25 affected
  groups, the first centre-line spans 73–96% of its stroke's outline width and
  92–98% of its height, which is what a centre-line inside a round-capped outline
  should do.

27 segments fold back. `ぬ` and `ゐ` are split in two places each, which the rule
handles because it is read from the ids rather than inferred from a count.

`tests/kana_artifact.rs` freezes the taught stroke count for all 25, so a
regression fails in CI without any upstream download.

### 3. **KanjiVG is the oracle, and `prepare-kana` refuses to write an artifact it could not check.**

"How many strokes is あ" must not be answered by the code being tested. KanjiVG
is a separate project whose per-stroke paths are an independent statement, and
every one of the **177** counts is checked against it before the artifact is
written. A mismatch aborts the build.

`--allow-unchecked` exists and must be passed *deliberately*; an artifact built
with it rests on this code's own assumptions and should not be committed.

KanjiVG is **not** redistributed and earns no notice: only a stroke *count* is
read, and a count is a fact about the language rather than a copy of the work.
`data/raw/kvgJa/` is gitignored.

### 4. **Kana outlines are in Make Me a Hanzi's font space, and no conversion changed.**

Verified rather than assumed, because getting it wrong would have flipped every
kana. Over all 7,007 kanji in `graphicsJa.txt` the medians span x 22–1012 and
y −108–884, against Make Me a Hanzi's box of x 0–1024, y −124–900, and there are
**zero** entries where `strokes.len() != medians.len()` and zero with an
out-of-box median. The y-up convention is confirmed independently by 二: the
short top stroke has the *larger* mean y (599 against 203).

So `Point::from_font(x, y) = (x, 900 − y)` is correct for Japanese as it stands,
and `render.ts`'s `scale(1, −1) translate(0, −900)` is correct for a kana.

**Kana outlines are in that box. The kana *medians* in the raw file are not** —
the displaced copies in §2 are the reason the raw file's aggregate looks wrong.
After the merge, all merged medians are inside 0–1024 on both axes, which the
artifact test asserts.

### 5. **The Japanese crates version themselves.**

`crates/nihongo-core` and `apps/nihongo-tutor` are `version = "0.1.0"`, **not**
`version.workspace = true`. This is not tidiness. While they inherited the
workspace version they could not be committed independently of a Hanzi Tutor
release: committing them on top of a 0.6.0 HEAD made the workspace 0.7.0, which
failed the existing tests asserting Cargo's version equals `tauri.conf.json`'s in
the main app and the tone trainer. A Hanzi Tutor release must not drag them
along.

The app has its own test for this: `the_app_info_and_the_bundle_config_agree`
checks `APP.version` against `tauri.conf.json`'s.

### 6. **`render.ts` is lifted from the Chinese app, and stays unchanged.**

`apps/nihongo-tutor/src/lib/render.ts` is byte-identical to
`src/lib/render.ts`. It paints SVG outlines in font space and centre-lines in
display space, and because AnimCJK publishes kana in Make Me a Hanzi's own frame
(§4), the transform is already right for a kana.

`types.ts` does **not** fork it: it declares `export type Character = Kana`, so
the file keeps the name it was written with and needs no edit. If `render.ts` is
ever changed here, change it in the Chinese app too — or record why they diverge.
This follows the pattern the repository already uses between its apps: Tone
Trainer lifted `ToneChart.svelte` and `transcript.ts` the same way.

`KanaCanvas.svelte` and `board.ts` are new, not lifted: Hanzi Tutor's
`PracticeCanvas` carries tone-panel and verdict-display concerns this app does
not have.

### 7. **The board's geometry belongs to the stylesheet; the script only sets resolution.**

`.board` is `width: min(100%, 44vh); aspect-ratio: 1 / 1`, and the canvas is
`width: 100%; height: 100%` inside it. One `ResizeObserver` reads the canvas's
own rectangle and sets the backing store, and nothing else writes a size.

An earlier version had the stylesheet say `aspect-ratio: 1` **and** `max-height:
62vh`, while the script measured the board and wrote both a width and a height
from that measurement. A `max-height` clamps the height without narrowing the
box, so the board became a wide rectangle with the kana stretched across it, and
the canvas matched the distortion. **Do not reintroduce a max-height on the
board, and do not let the script compute a second dimension.**

### 8. **The course covers every kana of the script, including katakana's five extra.**

Katakana has five characters hiragana does not — ヷ ヸ ヹ ヺ and ー — and a course
assembled from the hiragana grid silently teaches 86 of katakana's 91. A test
caught exactly this during development.

`lessons()` therefore has a katakana-only branch, and
`the_course_covers_each_script_completely` asserts that the lessons teach
*every* kana the dataset holds, for both scripts. Removing a group fails it.

This matters beyond kana: the same trap is waiting for the kanji course.

### 9. **The romaji input table is derived from the readings, not written twice.**

`input.rs` builds its table from `readings::hiragana_with_readings()` and
`curriculum::yoon()`, so a spelling added to a reading becomes typeable with no
second edit. `the_table_covers_every_spelling_the_readings_offer` is the check.

Where two kana share a spelling — じ and ぢ are both `ji`, ず and づ both `zu`,
お and を both `o` — the **first in code-point order keeps it**, so the common
kana wins and the uncommon ones stay reachable as `di`, `du`, `wo`. The two
scripts share one table: each entry carries a hiragana and a katakana rendering,
because a hiragana-first pass would otherwise leave katakana entries unclaimed
and `to_kana_in(Katakana, "ka")` would return か.

### 10. **Script conversion is the 0x60 offset, and it does not cover everything.**

The hiragana and katakana blocks are the same 86 characters `0x60` apart, so
`to_katakana` and `to_hiragana` are an addition and a subtraction. The v-series
(ヷ ヸ ヹ ヺ) and ー have **no hiragana**, and both functions return `None`
rather than guessing; the input table carries them as their own entries, and
`Script::of('ー')` is `Katakana`.

### 11. **Small kana are reachable only through the `x`/`l` escapes.**

The small kana sit *before* their full-size counterparts in code-point order —
ぁ is U+3041 and あ is U+3042 — so a table that claims spellings in code-point
order has ぁ take `a` first and every vowel type small. `input.rs` skips
`SMALL_KANA` when claiming base readings; `xa` gives ぁ, `a` gives あ.

### 12. **The notices are app-local, and the AnimCJK licence ambiguity is recorded rather than resolved.**

`apps/nihongo-tutor/src-tauri/licences/` and its `licences.rs` catalogue, **not**
the repository's shared `licences/`. The shared directory's own test
(`src-tauri/tests/licences.rs` in the main app) requires every file in it to be
catalogued by the *Chinese* app, so a kana notice there would make that app ship
and display a notice for data it does not contain.

The two licence *texts* AnimCJK's statement points at are referenced out of the
shared directory, because those genuinely are shared.

**The ambiguity, which is real and should not be quietly resolved:** AnimCJK's
`COPYING.txt` assigns *"text files prefixed by `graphics`"* to the **Arphic
Public License**, and *"SVG files … representing kana or strokes"* to the
**LGPL-3.0-or-later** *because kana are not derived from the Arphic fonts*.
`graphicsJaKana.txt` — which is where the geometry comes from — is a `graphics*`
file whose content is kana, so it falls between the two clauses.

Both texts ship, so either reading is noticed, and this is not a blocker. It is
**upstream's to resolve**: an issue should be raised asking which clause they
intend for the kana `graphics` files. See also `LICENSES.md`'s "The Japanese kana
data" section, which is the public record.

### 13. **A lesson never lists something the board cannot draw.**

`lessons()` filters through `dataset.get()`, and `KanaDataset::from_kana_teachable`
drops anything without geometry, so the course cannot offer a kana the board
cannot grade. This is the Japanese form of the Chinese invariants about not
dead-ending a practice session.

### 14. **An interface may post only the `GradeOptions` fields it means to change.**

`GradeOptions` lives in `hanzi-core` and is shared, and its `#[serde(default)]` is
on the **struct**, not on one field: a caller that posts `{ inkWidth }` gets the
other three tunables from `Default` rather than
`missing field 'resampleK'`. That is the shape the interfaces want — the defaults
are the ones the tolerance study was fitted against, so an interface should not
have to restate them to avoid being rejected — but it has a consequence worth
knowing: **a field renamed on one side is silently replaced by its default here
rather than failing.** So the options the interface actually posts are pinned by a
test that deserialises them the way the command does
(`ipc_contract.rs::grading_reads_the_attempt_the_interface_sends`), not just by a
test that grades with `GradeOptions::default()` and never touches serde.

This is the request half of `HANDOVER.md` invariant 2 — that the IPC contract is
camelCase on both sides — and the half this app got wrong: 110 tests, clippy and
`svelte-check` all passed while the Grade button could not grade a single attempt,
because nothing deserialised the options the frontend sends. When a command takes
a struct, test the *payload*, not the struct.

### 15. **The learner's data is this app's, and it stays this app's.**

Kana Tutor writes exactly one thing about its learner — the per-pair tallies in
`confusions.json`, in its own directory under the app's data directory — and no
other app reads it, imports it, or shares a store with it. **The Japanese and
Chinese apps are separate products; what is shared between them is code, never user
data.** So `hanzi-store` is not the answer for the drill's tallies, and it is not
the answer for the review queue that `ROADMAP_NIHONGO.md` N2 plans either, however
convenient its language-neutral types look. That was the maintainer's decision, and
it is a boundary rather than an implementation detail: a shared store for two
separate apps is what turns "separate" into "tangled", and the tangle is discovered
on the day one app's schema change breaks the other's history.

Two practical consequences:

* **The file is written by Rust, not by the webview.** The app still has *no*
  filesystem permission in `capabilities/default.json`, and that comment there is
  accurate: the webview cannot name, read or write a path. `std::fs` in the app
  crate does the writing, which also means the file needs no plugin, no permission
  and no path crossing the IPC boundary.
* **A write that fails is reported, not swallowed.** The answer is still counted in
  memory and the learner is told that it was not saved. Under the harness's own
  file sandbox this is the *normal* result — see trap 5 — because the sandbox denies
  writes to `~/Library`, so a live drill run in an agent session shows
  "the answer was counted but could not be saved: Operation not permitted" and the
  app is working correctly. To exercise the real file from a sandboxed session,
  start the app with `HOME` pointing inside the workspace (and `RUSTUP_HOME` left
  alone, or cargo cannot find its toolchain): the data directory follows `HOME`.

---

## 5. The verification loop

Four layers, cheapest first. All of them are worth running before a commit that
touches the engine or the data; the last two before one that touches the
interface.

```bash
# 1. The data layer and the app, including the IPC contract and the notices.
./scripts/with-cargo-env.sh cargo test -p nihongo-core -p nihongo-tutor

# 2. Nothing else broke. Fast when the tree is warm.
./scripts/with-cargo-env.sh cargo test --workspace --features hanzi-core/prepare

# 3. Lints. -D warnings, so a warning is a failure.
./scripts/with-cargo-env.sh cargo clippy --workspace --all-targets -- -D warnings

# 4. The frontend: types, then the pure arithmetic.
pnpm --dir apps/nihongo-tutor run check:web     # svelte-check
pnpm run test:web                               # from the ROOT — it owns vitest
```

`apps/nihongo-tutor/src/lib/board.test.ts` is picked up by the **root** project's
vitest, through the `include` list in `vitest.config.ts`. The app itself installs
no test runner, deliberately — the same arrangement the tone trainer uses. A test
added under `apps/nihongo-tutor/src/**` and not named `*.test.ts` will silently
never run.

### Regenerating the artifact

```bash
./scripts/fetch-data.sh
pnpm run prepare-kana
git diff --stat crates/nihongo-core/data/kana.bin.gz
```

`prepare-kana` prints what it did, and the numbers are the check: **177 kana, 86
hiragana, 91 katakana, 516 strokes, 25 split characters, 27 segments folded back,
177 checked against KanjiVG**. If `checked against KanjiVG` is not 177, the run
failed rather than warned — unless `--allow-unchecked` was passed.

A regenerated artifact should be **byte-identical** to the committed one. If it
is not, something upstream moved, and the artifact test will say which stroke
count changed.

### Seeing the interface

There is no display in every environment this gets worked on, so this recipe
matters. It captures **only the app's window**, which is both more reliable than
the app being frontmost and the right thing to do on someone else's machine.

```bash
# Start the app (dev server on :1422 plus the window) in the background.
pnpm --dir apps/nihongo-tutor run dev > /tmp/kana-dev.log 2>&1 &

# Find the window by TITLE — the owner name is `nihongo-tutor`, which contains
# no "kana", so matching on the owner silently finds nothing.
WID=$(python3 -c "
import Quartz
for w in Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionAll, Quartz.kCGNullWindowID):
    if str(w.get('kCGWindowName')) == 'Kana Tutor':
        print(w.get('kCGWindowNumber')); break
")
screencapture -x -o -l"$WID" /tmp/kana.png
```

`-l<id>` captures that window's contents even when it is not frontmost, and `-o`
omits the shadow. Read the result; do not assume.

To see a view that needs a click, change the initial value of `view` in
`App.svelte` and **restart the app** — see trap 3, HMR preserves state.

### Driving the interface, since it cannot be clicked from here

System Events is blocked (trap 6), so nothing outside the app can click it, draw
on it or read its DOM. Two things work, and between them they are enough to check
any screen:

* **A temporary script in `index.html`** — it is served by Vite for both the dev
  server and the webview, it can click anything, and editing it makes Vite
  **reload the page**, which re-runs the script against a fresh app. A `MODE`
  constant plus a few lines that click a tab by its label and then report
  `document.querySelector(...).innerText` back over the dev server is the whole
  harness. Delete it before committing; it is a diagnostic, not a feature.
* **Synthetic pointer events on the board**, which is how the verdict was finally
  seen. `kana.medians` are in display space, `pointerToDisplay` maps the canvas's
  client rectangle into that same box, so a driver can interpolate along each
  centre-line and dispatch `pointerdown`/`pointermove`/`pointerup` — an attempt
  that should score ~99. Two details: read the centre-lines over the app's own IPC
  (`window.__TAURI_INTERNALS__.invoke("kana", { ch })`) rather than guessing them,
  and **stub `canvas.setPointerCapture` first**, because a synthetic pointer was
  never really down and the real call throws, which would abandon `down()` before
  it records the stroke.

### When the window is blank

A blank white window is not a webview problem to be worked around; it is usually a
JavaScript error, and the webview will not tell you — Tauri does not forward the
console, and `log show` on the app's process showed nothing. What does work is a
**beacon**: add to `index.html` a `window.addEventListener("error", …)` that
`fetch`es `/__probe?error=…&stack=…`, and a Vite plugin (in `vite.config.ts`) whose
middleware appends every request to a log file. The request log then says exactly
how far the page got and the probe carries the message and stack. This is how
`effect_update_depth_exceeded` was found after a screenshot had said only "blank".

Two things to know about reading that log: WebKit serves already-fetched modules
**from its cache**, so a reload shows the `index.html` request and nothing else —
a cold app start is the honest measurement; and give each beacon a `?t=<now>` so
its own fetches are not cached either.

**And a false lead worth not repeating.** Vite binds loopback as **IPv6 only** here:
`lsof` shows `[::1]:1422 (LISTEN)`, `curl http://[::1]:1422/` answers 200 and
`curl http://127.0.0.1:1422/` gets connection refused. That looks exactly like "the
webview cannot reach the dev server, hence the blank page" — but the webview had
already connected (`com.apple.WebKit.Networking` holds an established connection to
`[::1]:1422`) and the page's own modules were being served. Check
`lsof -nP -iTCP:1422` before believing the DNS story.

### Seeing what the drill remembered

The drill's record is a file, which makes it the one part of this app that can be
*read* rather than looked at:

```bash
cat ~/Library/Application\ Support/com.hanzitutor.kana/confusions.json
```

It is written after every answer, pretty-printed, and keyed by the pair
(`"シ|ツ"`, lower code point first) — so a change to the weighting rule can be seen
in the numbers rather than inferred from which kana came up. To prove that a
restart reads it back, answer a few questions, quit, start again, and answer a few
more: a pair's `asked` must continue from where it left off rather than restarting
at 1. That is also the check the tests do over a temporary directory, and it is the
one worth repeating by hand after touching `store.rs`.

Under the agent harness the write is **denied** — the sandbox refuses `~/Library`,
so the drill shows "the answer was counted but could not be saved" and is working
correctly (trap 5, invariant 15). To exercise the real file there, point `HOME` at
a directory inside the workspace and leave `RUSTUP_HOME` alone:

```bash
# The `$HOME` on the right is expanded by your shell *before* the assignment on
# the left takes effect, which is the point: that is the real one, where cargo's
# toolchain lives. Without it, rustup claims no default toolchain is configured.
HOME="$PWD/.tmp-kana-home" RUSTUP_HOME="$HOME/.rustup" \
  pnpm --dir apps/nihongo-tutor run dev
```


---

## 6. Traps that cost time here

### 1. A clamp on one axis and `aspect-ratio` on the other gives a rectangle

The board shipped as a wide rectangle with あ stretched across it and a second
clipped copy at the edge. The stylesheet had `aspect-ratio: 1` **and**
`max-height: 62vh`; the `max-height` clamps the height without narrowing the
box, and the script then sized the canvas from a measurement of the distorted
result.

It was invisible to `svelte-check`, to the tests, and to me reading my own
screenshot — it took the maintainer saying "the drawing box is vertically
compressed to a rectangle". The fix is invariant 7. The general shape of the
mistake: **two things deciding one dimension.**

### 2. macOS gives the webview the whole window, and draws the title bar over it

The heading sat half-hidden behind the traffic lights and was only obvious in a
screenshot. 20px of top padding is not enough; the strip is about 28pt. `main`'s
padding is now `calc(var(--safe-top, 0px) + 2.25rem)`, reading the same
`--safe-top` the other two apps define.

**The other two apps may have the same latent problem** — their first element sits
close enough to the top to clip. Worth checking on a display before assuming
their layouts are fine.

### 3. Vite HMR preserves a component's state, so changing an initial value changes nothing

Changing `let view = $state<…>("practice")` to `("drill")` and waiting for the
reload does **not** switch the view: Svelte's HMR preserves the existing state.
The file is correct and the running app is unchanged, which looks exactly like
the edit having failed. Kill the app and restart it.

It cost two capture cycles to notice, and the giveaway was that the file on disk
said `licences` while the window still showed Practice.

### 4. `screencapture` by region captures whatever is in front

The first captures were by region and were fine until the window lost front
position, at which point they silently captured a *different* application. Use
`-l<window id>`, by title, per §5. Matching Quartz windows on the owner name
finds nothing — the owner is `nihongo-tutor`, which contains no "kana".

### 5. The harness's file sandbox makes the app print alarming WebKit errors

Roughly a dozen lines like:

```
could not create directory ".../Library/WebKit/nihongo-tutor/WebsiteData/MediaKeys/v1"
for future sandbox extension, error … Code=513 "You don't have permission"
```

These are the *sandbox the agent runs under* denying writes to `~/Library`. They
are not the app failing, the window comes up regardless, and they should not be
chased. On a normal run they do not appear.

### 6. `System Events` is blocked, so the interface cannot be driven

`osascript -e 'tell application "System Events" to get name of first process'`
fails with `A privilege violation occurred. (-10004)`. There is no clicking
buttons or drawing strokes from here. Screenshots and code changes are the only
levers — which is why invariant 7's "look at it" recipe is worth the setup, and §5's
"Driving the interface" is how a click that matters still gets made. (Activating the
app by name does not work either: it is a bare dev binary with no bundle, so
`tell application "nihongo-tutor" to activate` answers `Can't get application`.)

### 7. Two `serde`/`format!` details that are each a compile error with a confusing message

* `assert_eq!` cannot take **both** an inline captured format arg and a named one
  (`"{script:?} …", script = script`). Use one or the other.
* A raw string containing SVG is terminated early by `"#` inside it —
  `href="#z12354d1"` — so test fixtures need `r##"…"##`.

And `sed -i ''` on macOS is BSD `sed`, which does **not** support `\b`; a
`sed 's/\bfoo/bar/'` silently does nothing rather than failing, so verify the
edit landed.

### 8. The curve that took longest to get right was a table's *order*, not its content

Both of the input engine's real bugs were ordering, not spelling: small kana
claiming vowels because they come first in code-point order (invariant 11), and
the katakana half of the table never being claimed because hiragana was iterated
first (invariant 9). The readings themselves were never wrong. When the romaji
engine misbehaves, suspect the loop before the table.

### 9. An effect that reads what it writes never settles, and the window just goes blank

`KanaCanvas`'s reset effect wrote `strokes`, `current`, `ghostCount` and `sweep`
and then called `paint()`, which **reads all four**. So the effect depended on the
state it wrote, and `strokes = []` is a fresh array — never `===` the previous
value — so the write always notified: Svelte ran it until
`effect_update_depth_exceeded` and the window came up blank on about half the cold
starts. The fix is to let the *repaint* effect (which only reads) do the painting;
`strokes = []` notifies it every time, so nothing is lost.

The same mistake in a milder form was next door: the `ResizeObserver` effect called
`measure()`, which calls `paint()`, from its own body, so it depended on the whole
picture and rebuilt its observer **once per animation frame**. `untrack(measure)`
fixes that — setup work that must not subscribe is wrapped, not avoided.

Three things made this expensive rather than obvious: the failure is a race (a
reload after the throw looked perfectly fine, so it read as webview flakiness); the
error never reaches the terminal (`svelte-check`, the tests and clippy are all
happy — an effect graph is not something any of them model); and a blank window
says nothing. §5's "When the window is blank" is the way in. **The rule to carry to
the kanji screens: an effect may read state or write it, and if it does both, one
of those two has to be `untrack`ed.**

### 10. `cargo test … | tail` throws the exit code away

A pipeline's `$?` is the **last** command's, so
`cargo test --workspace … 2>&1 | tail -60` reports `tail`'s status and a failing
suite reads as `exit=0`. That nearly recorded a broken layer as green here. Either
`set -o pipefail`, or send the output to a file and grep it, and check `$?` on the
unpiped command: the same run as a file showed `exit=0` honestly and 762 passing,
which is what the tailed form could not distinguish.

---

## 7. Open decisions

Ordered by how much they block.

### The pitch-accent lexicon — research done, decision not taken

`ROADMAP_NIHONGO.md` N5. The short version: **no audit-clean pitch-accent
lexicon can be bundled.** Kanjium's `accents.txt` carries a genuine CC BY-SA 4.0
file and the author states in issue #13 that its source is withheld "due to
potential copyright issues"; joining it against Wadoku's dump shows **88.9% of
pairs identical including list order**. Wadoku and OJAD are openly non-free.
The clean route is to **generate** accent patterns — `tdmelodic` is BSD-3-Clause
— and the open verification is the licence and provenance of its **model
weights** and the training labels. `docs/research/JAPANESE_TUTOR_FEASIBILITY.md`
§6.1 has the full working.

### Whether kana want spaced repetition at all

179 kana and 33 digraphs is small enough to learn by exposure, and the Chinese
app's scheduler (`hanzi-core::progress`, SM-2) is reusable **as code** — it is
language-neutral types and arithmetic, which is the kind of thing the two apps
share. `hanzi-store` is not: it is a store, and a store holds a learner's data,
which invariant 15 keeps app-local. But "reuse the SRS because it is there" is not
an argument either way, and the honest question is whether a review queue helps
kana or just adds a screen. The drill has now answered a smaller version of it —
per-pair weights, no schedule, no due dates, no queue — and that is the thing to
weigh a real scheduler against.

### Where kana audio comes from

macOS ships Japanese system voices (`Kyoko`, `Eddy` and others, `ja_JP`) and
`hanzi-voice::speech` already drives the system synthesiser, so **the desktop
needs no work**. Bundled clips would need generating — **MeloTTS-Japanese is MIT
for both code and weights**, which is the cleanest option — and Commons has only
about ten isolated kana clips with per-file licences. Undecided.

### Whether this stays a separate app

It is `apps/nihongo-tutor/` with its own identifier `com.hanzitutor.kana`, which
follows the repository's multi-app pattern and keeps the binary small. A learner
studying both languages might want one app — but **the data half of this question
is settled**: the two apps' learner data is separate and stays separate
(invariant 15), so "one app" would mean one window over two stores rather than one
store. The shared *code* is what makes that cheap, and it is already shared. What
remains open is the shell, not the storage.

### The kanji level ladder

The Japanese feasibility report §6.4: the JLPT publishes **no official kanji or
vocabulary list**, and the best-licensed community list chains to tanos.co.uk,
which asserts no licence. The recommendation is to derive bands from
`dictionaryJa.txt`'s kyōiku grades and JMdict's `nf01`–`nf48`, and to say so in
the UI. Not yet decided, and it shapes the kanji course.

### What the app should say to a learner who already reads Chinese

**A product question rather than a technical one, and the maintainer is best
placed to answer it because they read Japanese first and learned Chinese second —
the opposite direction to the one it is usually asked in.**

**First, the part that is not a question.** The transferability that matters
architecturally is the *mechanics*: counting strokes, pairing them and judging
order and direction is one software problem whatever the script, which is why
`nihongo-core` uses `hanzi-core`'s `geom`, `raster` and `grade` unchanged. **The
data is a separate matter and stays Japanese** — AnimCJK for kanji geometry,
KanjiVG as the stroke oracle, exactly as for the kana. The Chinese dataset is not
an input to the kanji course: not a source, not a cross-check, not a fallback.

**Then the product question.** The characters overlap enough that a Chinese reader
arrives with a real head start, and the app should say something about it rather
than pretend otherwise. The maintainer's summary of what actually carries over:

> In many cases he will be familiar with the **meaning**, but the **phonemes will
> be entirely different**, and sometimes even the meaning diverges. The only thing
> that is basically the same is **stroke order and shape**, and to a degree the
> meaning.

So: **shape and stroke order transfer; pronunciation does not transfer at all;
meaning transfers partly and unreliably.** The measurements behind that — 1,200 of
the 2,136 jōyō are the same simplified character the HSK course teaches, 702 exist
only as traditional forms, 234 are shinjitai with no counterpart, and the shape
agrees about nine times in ten where both exist — are in `ROADMAP_NIHONGO.md`
§"Why the overlap still matters", which is explicit that they inform a *screen* and
not a build.

The framing is probably "you have met this character — here is what Japanese reads
it as" rather than "you know this", and the 234 両 乗 亀 亜 仏 仮 伝 価 働 児 剣 労 単
厳 収 営 団 are worth knowing as a set, because they are where the head start runs
out.

**The non-negotiable part: glosses come from JMdict senses and never from a Chinese
dictionary.** Meaning transfers often enough to be tempting and not often enough to
be safe — 娘, 手紙, 汽車, 勉強, 丈夫 and 走る are actively wrong across the two
languages, and a Chinese gloss on a Japanese card is worse than no card.

**And the direction works both ways.** The maintainer's own path — Japanese first,
Chinese second — is served today by Hanzi Tutor itself, so anything that records
"this character is already known from the other language" must not assume which
language came first.

### The `assets/website` duplication

`assets/README.md` records that all twelve files in `assets/website/` also exist
byte-identical inside `assets/showcase/` — 10 MB carried twice. Plausibly
deliberate (a flat set is easier to upload from). Decide whether it stays a copy.

### The two Dependabot warnings that stand

`glib` 0.18.5 (unsound) and `proc-macro-error` (unmaintained, via `glib-macros`)
are one chain — `glib ← atk ← gtk ← muda/tao ← tauri` — and it is **GTK, so
Linux-only**: `cargo tree -i glib` prints nothing for the host target or
`aarch64-apple-darwin`. A fix needs gtk-rs 0.20, which needs a `muda`/`tao`/
`tauri` that has not shipped it. Accepted, and revisited only if the Linux
milestone happens.

---

## 8. The notices

Five, in `apps/nihongo-tutor/src-tauri/src/licences.rs`, with the text compiled
in by `include_str!` and copied into the bundle as plain text:

| id | What it covers |
| --- | --- |
| `agpl` | the app's own code, from the repository `LICENSE` |
| `provenance` | `LICENSES.md`, bundled as `licences/PROVENANCE.md` |
| `animcjk` | the kana geometry, **and the statement of what was changed** |
| `lgpl` | the licence the kana SVGs and the kana `graphics` file are under |
| `arphic` | the other licence that file could be under, per invariant 12 |

`tests/licences.rs` holds the three-way correspondence the main app's does:
every catalogued notice exists on disk *and* is the text compiled into the
binary, every file in the app's notice directory is catalogued, and
`tauri.conf.json`'s `bundle.resources` copies exactly that set. Two further tests
check that the AnimCJK notice actually **records the modification** — LGPL-3.0 §2
requires it — and that none of the notice files is gitignored, since a
gitignored notice vanishes from a clone and the bundle.

The app currently ships only `LICENSE`, `LICENSES.md` and the three notice files
above. **Before distributing it**, read `LICENSES.md`'s "Before you distribute":
the same rules apply, including the EDRDG update obligation that arrives with the
first KANJIDIC2 data.

---

## 9. What is deliberately not built

* **Kanji, in any form.** No data layer, no course, no screen. The feasibility
  work is in `docs/research/JAPANESE_TUTOR_FEASIBILITY.md` and the plan is
  `ROADMAP_NIHONGO.md` N6 onwards. The short of it: KANJIDIC2 and JMdict are
  clean (CC BY-SA 4.0 via EDRDG, the same licence the repository already ships
  for CC-CEDICT), AnimCJK's `dictionaryJa.txt` supplies the kyōiku grades and
  214 radicals under LGPL, and the KanjiVG-as-oracle trick does not transfer —
  kanji need `dictionaryJa.txt`'s own grade sets.
* **Audio.** §7.
* **Spaced repetition.** §7.
* **A per-learner confusability *matrix*.** The drill does now learn which of the
  13 pairs *this* learner gets wrong, and weights them (invariant 15, roadmap N3).
  What is not built is a matrix: pairs beyond the fixed 13, or a screen that shows
  the learner what they mix up. The record is there to build one on.
* **Grammar and particles.** "Kanji won't teach you to read" is the defining
  Japanese failure mode, and a kana tutor with no grammar is a kana tutor only.
  Out of scope for now, and the largest thing missing from the product.
