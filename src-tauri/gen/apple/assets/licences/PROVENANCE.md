# Data provenance and licences

Hanzi Tutor's own source code is licensed under the **GNU Affero General Public
License, version 3** (see `LICENSE`). The **data** it ships with comes from
several upstream projects under different licences, and all of them require
their notices to be included with any redistribution. This file records what
came from where, and what has to travel with it.

Everything described in "What is bundled" is **already inside the built app**.
There is no first-run data step for the reader and no network access while the
app runs: the dataset is compiled into the executable and the notices are
compiled in beside it, with plain-text copies in the bundle's
`Resources/licences/`. This document is the human-readable record of that
arrangement.

There is exactly **one** exception, and it is not in that table. Recognising what
a learner *said* — as opposed to how they said it — needs a speech model, and no
worthwhile Chinese model is small enough to bundle. So that model is an optional
download the learner starts from the settings screen. This app does not
redistribute its weights and does not fetch them on anyone's behalf; it shows the
address, the size and the licence, and downloads only when the button is pressed.
See "The speech model is downloaded, not shipped" below. The README's promise is
restated the same way: the app downloads nothing *unless you ask it to*, and
everything the bundled course teaches still needs nothing.

Some of the entries below are **code rather than data**: SQLite and its `rusqlite`
bindings, `cpal`, and the speech-recognition stack. Each is recorded here rather
than left implicit.

## What is bundled

| Data | Source | Licence |
| --- | --- | --- |
| Stroke outlines (`strokes`) | Make Me a Hanzi `graphics.txt` | Arphic Public License |
| Stroke centre-lines (`medians`) | Make Me a Hanzi `graphics.txt` | Arphic Public License |
| Etymology hints | Make Me a Hanzi `dictionary.txt` | LGPL-3.0-or-later |
| Frequency rank, pinyin, meaning, radical, HSK level | [`hanziDB.csv`](https://github.com/ruddfawcett/hanziDB.csv) | MIT |
| Words: characters, HSK 3.0 level, derived rank | [complete-hsk-vocabulary](https://github.com/drkameleon/complete-hsk-vocabulary) | MIT |
| Word readings and definitions | CC-CEDICT (via complete-hsk-vocabulary) | CC BY-SA 4.0 |
| Interface font, Noto Sans SC | [noto-cjk](https://github.com/notofonts/noto-cjk) / [Google Fonts](https://fonts.google.com/noto/specimen/Noto+Sans+SC) | SIL OFL 1.1 |
| Study database engine, SQLite 3.45.0 | [sqlite.org](https://sqlite.org/), vendored by `libsqlite3-sys` | Public domain |
| SQLite bindings, `rusqlite` | [rusqlite](https://github.com/rusqlite/rusqlite) | MIT |
| Audio capture, `cpal` | [cpal](https://github.com/RustAudio/cpal) | Apache-2.0 |
| Speech recognition engine, `sherpa-onnx` 1.13.8 | [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) | Apache-2.0 |
| Model inference, ONNX Runtime | [onnxruntime](https://github.com/microsoft/onnxruntime) | MIT |
| Text-to-phoneme front end, `espeak-ng` (**mobile builds only**) | [espeak-ng](https://github.com/espeak-ng/espeak-ng), vendored by sherpa-onnx 1.13.8 | GPL-3.0-or-later |
| The Android app's libraries: AndroidX, Material, Kotlin | [AndroidX](https://developer.android.com/jetpack/androidx) and [Material](https://github.com/material-components/material-components-android) | Apache-2.0 |
| Graded phrases: HSK 1–2 sentences, pinyin, translations | [no7z/hsk-sentences-audio](https://huggingface.co/datasets/no7z/hsk-sentences-audio) | CC BY-SA 4.0 |
| Graded readers: passages with word-aligned pinyin and gloss | [harukicoder/hsk30-graded-readers](https://huggingface.co/datasets/harukicoder/hsk30-graded-readers) | CC BY 4.0 |
| The bundled pronunciation clips (MP3) | [no7z/hsk-sentences-audio](https://huggingface.co/datasets/no7z/hsk-sentences-audio) — the corpus's own recordings, made with CosyVoice2-0.5B | Apache-2.0 |

The first six rows are compacted by `prepare-data` into one generated artifact,
`crates/hanzi-core/data/hanzi.bin.gz`, which `src-tauri/src/state.rs` embeds with
`include_bytes!`. That artifact **is committed** (about 13 MB) so that a clone,
and CI, build without downloading the 33 MB of upstream text. Removing the
artifact removes all third-party data from the build.

The last three rows are the **pronunciation audio**, which is a separate pipeline
with a separate artifact. See "The pronunciation clips" below.

The font is committed at `src/assets/fonts/NotoSansSC-VF.ttf` (about 17 MB) and
is copied into the frontend bundle by Vite. See "Fonts in the UI" below.

The notice texts live in [`licences/`](licences/), committed. They are listed in
`src-tauri/src/licences.rs` — that catalogue is what the app's Licences screen
shows, and what `tauri.conf.json` copies into the bundle. A test fails if a file
in `licences/` is not catalogued, if a catalogued file is missing, or if the
bundle config does not copy exactly that set, so a notice cannot be half-added.

**The catalogue is curated rather than exhaustive, and it is worth saying where the
line falls.** Every licence the bundled data, the interface font and the
compiled-in third-party code are under is here in full, but not every dependency
is *named*: the Rust build graph and the npm tree run into the hundreds, almost all
of them MIT or Apache-2.0, and both of those texts are already carried — so the
obligation those licences impose on an unmodified binary, which is to pass the
licence on, is met by the text rather than by a name. A dependency earns an entry
of its own when a reader would otherwise be misled about it: a copyleft term, an
attribution requirement that goes beyond the licence text, a source that arrives
outside a package manager, or a work a reader can actually recognise in the app.
That is a judgement, not a threshold, and this paragraph is where it is recorded.

### Make Me a Hanzi — <https://github.com/skishore/makemeahanzi>

`graphics.txt` and `svgs.tar.gz` are derived from two free fonts, **Arphic PL
KaitiM GB** and **Arphic PL UKai**, released by Arphic Technology Co., Ltd. under
the Arphic Public License. Redistribution and modification are permitted,
provided the licence text is included and modified versions are clearly marked
as such. The app does not modify the outlines; it only re-encodes them.

The upstream `COPYING` names the sources but is not the licence text itself — it
points at one. The **full Arphic Public License** is therefore fetched
separately, from the `APL/english/ARPHICPL.TXT` file the same repository
distributes, and ships as `licences/Arphic-Public-License.txt`. (It is worth
being explicit about this: the notice that was being fetched before this was
only the pointer, and a pointer is not the licence a reader is entitled to.)

`dictionary.txt` is derived from [Unihan](https://unicode.org/charts/unihan.html)
and [CJKlib](https://github.com/cburgmer/cjklib) and is distributed under the
**GNU Lesser General Public License, version 3 or later**. That file,
`licences/LGPL-3.0.txt`, also carries the Unicode/Unihan notice that the derived
dictionary inherits.

All three texts — `Arphic-Public-License.txt`, `MakeMeAHanzi-COPYING.txt` and
`LGPL-3.0.txt` — must be included when redistributing the app.

### hanziDB.csv — <https://github.com/ruddfawcett/hanziDB.csv>

MIT licensed. The list is derived from Jun Da's Modern Chinese Character
Frequency List and uses simplified characters. Its `LICENSE` text ships as
`licences/MIT-hanziDB.txt`.

### complete-hsk-vocabulary — <https://github.com/drkameleon/complete-hsk-vocabulary>

MIT licensed (Copyright © Yanis Zafirópulos). This is the source of the **word
list**: each word's simplified characters, its HSK 3.0 level, and the derived
frequency used for ordering. The list itself is compiled from the official
HSK 2.0/3.0 vocabulary, including
[elkmovie/hsk30](https://github.com/elkmovie/hsk30) (MIT, extracted from the
official Ministry of Education PDF).

Its `LICENSE` ships as `licences/MIT-complete-hsk-vocabulary.txt`.

**The readings and definitions carry a second licence.** That project draws its
word pinyin and English meanings from **[CC-CEDICT](https://cc-cedict.org/)**
(via <https://www.mdbg.net/chinese/dictionary>), which is licensed
**Creative Commons Attribution-ShareAlike 4.0 International (CC BY-SA 4.0)**.
Two consequences follow, and both matter:

1. **Attribution is required.** The CC-CEDICT notice, its licence and a link to
   the licence text must travel with the app, beside the other notices above.
   CC-CEDICT is itself derived from the Unihan database and from
   [CJKlib](https://github.com/cburgmer/cjklib); its own notice names them.
2. **ShareAlike applies to the derived definition text.** The definitions are
   stored in `hanzi.bin.gz`. That extracted text, and anything built from it,
   stays under CC BY-SA 4.0. This does not reach the program code, the stroke
   geometry, or the app's own interface — only the derived dictionary text.

Both the attribution (`licences/CC-CEDICT.txt`, which also records what this app
changed) and the **full legal text** (`licences/CC-BY-SA-4.0.txt`) ship. The
CC-CEDICT wiki's front page still describes the project as CC BY-SA 3.0; the
distribution actually used — MDBG, the publisher named in the dictionary's own
header — states 4.0, and that is the licence recorded here.

Note also what is deliberately **not** taken from that file: its `q` (frequency
from SUBTLEX-CH) and `p` (part-of-speech) fields are ignored, so the app carries
nothing derived from those sources. Its `r` radical field is ignored too, the
radical coming from Make Me a Hanzi as before. The rank the app does show for a
word is computed here from the MIT character frequency list.

### SQLite — <https://sqlite.org/>

The study data — schedule, attempt log, vocabulary list and course cursor — lives
in one SQLite database, `hanzi.db`, in the user's study data directory. SQLite is
compiled into the app from the amalgamation vendored by the `libsqlite3-sys` crate
(SQLite 3.45.0), so there is no external process, nothing to install, and no
system SQLite is linked against.

SQLite is in the **public domain**. Its authors' statement of that is the blessing
at the top of `sqlite3.c`, reproduced in
[`licences/SQLite-Public-Domain.txt`](licences/SQLite-Public-Domain.txt), which
also records what this app does with it. That file ships, and is catalogued in
`src-tauri/src/licences.rs`, along with [`licences/MIT-rusqlite.txt`](licences/MIT-rusqlite.txt)
for the MIT-licensed `rusqlite` and `libsqlite3-sys` crates that bind it to Rust.
Neither adds a condition to redistribution beyond the MIT notice, and the database
holds study data rather than content — nothing from SQLite is redistributed with
the dataset.

### cpal — <https://github.com/RustAudio/cpal>

Tone practice records a single spoken syllable and scores its pitch contour. The
capture is `cpal`, a cross-platform audio I/O crate under **Apache-2.0**, whose
notice ships as [`licences/Apache-2.0.txt`](licences/Apache-2.0.txt). Apache-2.0
combines into AGPL-3.0, which is what this project is under, so there is no
conflict; the notice travels because Apache-2.0 requires it.

On macOS and iOS `cpal` drives CoreAudio through `coreaudio-rs`, which is
MIT/Apache-2.0. The crate also carries Linux (ALSA), Windows (WASAPI/ASIO) and
Android (AAudio/OpenSL) backends; they are compiled only for those targets and
are not part of a macOS build. **No audio is written to disk and none leaves the
machine**: the buffer lives in memory for one utterance, is scored, and is
dropped. This is the only part of the app that takes input from a microphone, and
the device is open only while the learner is holding the button.

The app's own pitch tracking, tone templates and scoring are **not** third-party
code — they are in `crates/hanzi-core/src/tone.rs` under this project's licence —
so judging *how* something was said involves no model weights and no recognition
library at all. That is a deliberate boundary, argued in
[`docs/research/ASR_TTS_CLAUDE_RESEARCH.md`](docs/research/ASR_TTS_CLAUDE_RESEARCH.md)
§6, and it is what keeps tone practice working with no download, no model and no
network. Recognising *which* syllable was said is the separate feature below, and
it is optional.

### sherpa-onnx and ONNX Runtime

Recognising what was said runs `sherpa-onnx` 1.13.8 under **Apache-2.0**, with
**ONNX Runtime** (MIT) executing the model. Both are *compiled into the binary*,
so both notices travel with it: [`licences/Apache-2.0.txt`](licences/Apache-2.0.txt)
and [`licences/MIT-onnxruntime.txt`](licences/MIT-onnxruntime.txt). Apache-2.0 and
MIT both combine into AGPL-3.0, so there is no conflict with this project's own
licence.

Two things about how they arrive are worth recording, because both are easy to get
wrong at packaging time:

- **They are not obtained through cargo.** `sherpa-onnx-sys` downloads a prebuilt
  native archive from GitHub releases *during `cargo build`* unless
  `SHERPA_ONNX_LIB_DIR` says otherwise. For a repository that fetches its data
  through a reviewed script and pins its notices three ways, an unpinned binary
  arriving at compile time is a different posture — so the download happens in
  [`scripts/fetch-sherpa.sh`](scripts/fetch-sherpa.sh), against a **pinned
  SHA-256**, and the build only reads what that left behind. The static archive is
  used, so the app stays one file; a shared build would leave `.dylib`s that a
  bundled `.app` would have to declare as frameworks or fail to launch.
- **It vendors more than it names.** The archive statically includes several
  further libraries — ONNX Runtime, kaldi-native-fbank and the other `kaldi-*`
  components, kissfft, `ssentencepiece`, `piper_phonemize`, and `espeak-ng`. Most
  are permissive (Apache-2.0, MIT, BSD-3-Clause) and are covered by the two
  notices above. One is not, and it has its own section below.

### espeak-ng is redistributed by the mobile builds

`espeak-ng` is the one component of that archive under a copyleft licence —
**GPL-3.0-or-later** — and it is *speech synthesis only*, which this app never
does: it recognises speech, it does not produce it, and no learner action reaches
a synthesiser.

That was verified once, on macOS, and it is true there. The archive's components
are linked individually on that platform, so the linker drops what nothing
references, and neither `nm` nor `strings` finds espeak-ng in the linked binary.
**It is not true on the mobile targets, and the difference is the whole point.**
Android and iOS are not given that archive; they are given a `libsherpa-onnx` that
has already been linked, with espeak-ng inside it. The APK, the AAB and the iOS
framework therefore redistribute espeak-ng whether or not a line of it ever runs —
and GPLv3 §6 conditions *conveying* the object code rather than using it, so "we
never call the synthesiser" is not an answer to it.

The check is easy to repeat, and worth repeating on any sherpa-onnx bump: take
strings that appear only in `libespeak-ng.a` and look for them in the shipped
library. In the macOS binary they are absent; in the Android `.so` for every ABI,
and in the iOS framework, they are present.

What travels for it is [`licences/GPL-3.0.txt`](licences/GPL-3.0.txt), catalogued
as its own notice so that the Licences screen shows it and the bundle carries it.
The "Corresponding Source" is the unmodified espeak-ng revision vendored by the
sherpa-onnx release pinned in [`scripts/fetch-sherpa.sh`](scripts/fetch-sherpa.sh)
(v1.13.8), published at <https://github.com/espeak-ng/espeak-ng>; nothing in the
vendored copy is modified, so that repository is the source for the code that ships
here.

The licence is compatible with this project's own, and it is worth being exact
about why, because "GPL in an AGPL app" reads worse than it is: **GPLv3 §13 and
AGPLv3 §13 each expressly permit linking or combining the two into one work and
conveying the result**, with the combined work's AGPL part staying under the AGPL
and the GPL part staying under the GPL. So the app remains **AGPL-3.0-only** and
nothing has to be relicensed. The consequence runs the other way: while espeak-ng
is inside the mobile binaries, this app cannot be offered under anything more
permissive than the AGPL.

### The Android build's libraries

The Android app is built against rather than linked into a set of Apache-2.0
libraries, and they ship as compiled bytecode inside the APK and the AAB:
**AndroidX** (`biometric`, which shows the fingerprint prompt; `webkit`,
`appcompat` and `activity`, which the activity and the webview the interface runs
in are built on; `lifecycle-process`), **com.google.android.material** for the
theme, and the **Kotlin** standard library and coroutines — along with the
transitive `androidx.*` artifacts those resolve to.

One licence covers all of them, and it is the same
[`licences/Apache-2.0.txt`](licences/Apache-2.0.txt) that `cpal` and `sherpa-onnx`
already use, so the catalogue names them together and nothing new is bundled for
them. What Apache-2.0 requires of an unmodified binary is §4(a) — that the
recipients be given a copy of the licence — which the catalogue, the bundle and the
Licences screen all do. §4(d), the `NOTICE` clause, is conditional on the work
actually carrying a `NOTICE` file, and none of these does: their repositories ship
`LICENSE.txt` and no `NOTICE`.

The set is the build's rather than this document's. It is worth reading
`build.gradle.kts` rather than this list when it matters, and worth knowing that a
declared version is not always the resolved one — `material:1.12.0` currently
resolves to 1.13.0 — so a list that has to be exact should be generated from the
build rather than transcribed from it.

### The speech model is downloaded, not shipped

`sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17` (SenseVoiceSmall, int8)
is about **163 MB compressed and 241 MB unpacked**, so it cannot be bundled. It is
fetched by the learner, from the settings screen, over the app's only network
call, and cached in the application data directory. Before the button is pressed
the screen states the address, both sizes, and the licence.

**This app does not redistribute the weights**, so their licence is not this
project's to comply with in the way the bundled notices are — the learner takes
them directly from their publisher. It is recorded here anyway, because a
redistributor should not have to discover it, and because it is *not* a free
licence in the sense the rest of this file uses:

| | |
| --- | --- |
| Model | SenseVoiceSmall, int8, converted for sherpa-onnx |
| Terms | [FunASR Model Open Source License Agreement v1.1](https://github.com/modelscope/FunASR/blob/main/MODEL_LICENSE), Alibaba Group |
| Pointer | the archive's own `LICENSE` is a one-line reference to the FunASR repository |
| Requires | attribution, and that the model's own names are retained |
| Notes | the agreement is revisable by its publisher, states that the weights are provided "for reference and learning purposes", and includes a conduct clause whose breach terminates the licence |

The toolkit that runs the model is MIT (`FunASR`'s source) and the model's weights
are licensed separately — the distinction the research warns about in §9, where
"a model's licence and its training data's licence can differ". That section did
**not** evaluate SenseVoice's terms; it recommended the model in §5.3 and left it
out of the compatibility table. That gap is recorded here rather than papered
over: the terms are compatible with what this app does (it points at the model
rather than shipping it, attributes it, and redirects to the upstream text), but
anyone intending to **redistribute the weights themselves** — in a bundled or
offline installer, say — must read the agreement first and decide for
themselves. Nothing here is legal advice.

### Noto Sans SC — <https://github.com/notofonts/noto-cjk>

The interface font is licensed under the **SIL Open Font License 1.1**, which
permits bundling and redistribution and requires the licence to travel with the
font. It ships as `licences/OFL-1.1.txt`. Noto is derived from Adobe's Source Han
Sans, which is why the OFL file carries Adobe's copyright line as well; the
font's reserved name is `Source`, not `Noto`, so no rename is required.

### The pronunciation clips — the corpus's own recordings

The phrase screen plays short recordings of graded phrases. They are **synthetic
speech**, generated when the app was built, and three separate things have to be
accounted for: the model that spoke, the text that was spoken, and the files
themselves.

**The clips are the corpus's own MP3s, under Apache-2.0.** Every bundled clip is
the file the upstream project published for that sentence, at both speeds — 819
phrases, 1,638 files, 30 MB. Its attribution file grants redistribution of that
synthesised output under Apache-2.0 and asks for one thing: that it be disclosed
as synthetic speech, which the phrase screen does.

**This reverses an earlier decision, on measurement.** The app used to generate
its own clips, first with MeloTTS and then with CosyVoice 3. Over 208 HSK-1
phrases, through one recogniser and one judge, the generated MeloTTS audio failed
57 where these recordings failed 15 — 46 phrases that only the generated audio got
wrong. The published recordings were better, already in the required format, and
permissively licensed. `docs/research/MELOTTS_PRONUNCIATION_ACCURACY.md` has the
comparison and the three synthesis paths built before it was made.

**The on-device synthesiser is still MeloTTS, and that is a different job.**
`licences/MIT-MeloTTS.txt` carries the licence as
published with the ONNX conversion this project uses, naming MyShell.ai. The
model is **not compiled into the app and not downloaded by it**: a build that
wants to synthesise runs `scripts/fetch-tts.sh`, which fetches it into
`.melo-tts/` and checks every file against a SHA-256 recorded in that script. The
clips are a product of the model, which is why the notice ships anyway.

It is worth recording how that licence was established, because the first answer
was wrong. The Hugging Face API reports **no licence at all** for the conversion
repository (`cardData: null`), and an earlier draft of this file concluded the
weights were of unknown licence and treated that as an accepted risk. It is not:
the repository contains a `LICENSE` file that the card simply does not surface,
and it is MIT. The general lesson is that "the metadata does not say" is not the
same as "the publisher granted nothing", and the two lead to opposite decisions —
one is a reason to go and read the file, the other a reason to stop.

**The sentences are two corpora under two different licences, kept apart.**

* `no7z/hsk-sentences-audio` — the HSK 1–2 sentences, their pinyin and their
  English translations, under **CC BY-SA 4.0**. This is the same share-alike
  licence the CC-CEDICT definitions already carry, and the same consequences
  follow: the extracted text and anything built from it stay under CC BY-SA 4.0,
  which does not reach the program code.
* `harukicoder/hsk30-graded-readers` — the reading passages with word-aligned
  pinyin and gloss, under **CC BY 4.0**. Attribution only, no share-alike. Its
  full legal text ships as `licences/CC-BY-4.0.txt`.

They are stored under separate directories and listed separately on the phrase
screen so that what a learner is hearing always has one attributable source. A
merged list would make it possible to ship one set under the other's notice.

**The bundled audio is third-party audio, redistributed under its own grant.**
The `no7z`
dataset ships its own MP3s, synthesised with **CosyVoice2-0.5B** — and those are
what this app bundles, unmodified, at both speeds. Its notice is reproduced
inside `licences/NO7Z-hsk-sentences-audio.txt` so the upstream chain of provenance
stays legible.

**The readers' audio, in contrast, is this project's own.** That corpus publishes
text only, so every one of its 1,184 clips was synthesised here with **MeloTTS**
(MIT, noticed separately), from text under CC BY 4.0. Nothing was taken from
upstream but the sentences, their pinyin and their glosses, and the changes are
recorded in that corpus's notice. Its text carries **no share-alike** condition,
which is what makes it the simplest of the two to redistribute.

**Each corpus's directory carries its own note.** `public/audio/no7z/README.md`
and `public/audio/harukicoder/README.md` state the source, the licence, the
attribution the licence requires and the changes made, so the obligation travels
with the files a reader actually receives and not only with the installed app.

The bundled recordings and the on-device synthesiser are therefore **two different
voices**, deliberately: a phrase with a recording is heard in the course's voice, a
phrase without one is spoken by the device.

**The upstream level labels are not treated as authoritative.** The `no7z`
project claims none of its sentences contain vocabulary above their own level.
Re-grading all 4,354 of its records against this app's own bundled word list
found about 5% carrying a token above their label. The app therefore presents a
level as a rough band rather than a fact. The same caution applies to the graded
readers, whose own datasheet records that its author hit the shelf target 61.8%
of the time — and to HSK grading in general, because the two official HSK 3.0
documents disagree on a large share of their shared vocabulary. The research
behind both numbers is in `docs/research/`.

## Before you distribute

1. **Nothing has to be gathered by hand.** The notices are in `licences/`, are
   compiled into the binary, and are copied into `Resources/licences/` by the
   bundle step. They are reachable from the app's **About and licences** screen,
   which is the fourth entry in the sidebar.
2. Run `pnpm run build` (see README.md, "Shipping a build"). It produces a signed
   `.app` and `.dmg`. Confirm the copies survived, since a resource path is the
   kind of thing that only breaks in the packaged app:

   ```bash
   APP=".cargo-target/release/bundle/macos/Hanzi Tutor.app"   # or src-tauri/target/...
   ls "$APP/Contents/Resources/licences"
   ```

   That directory must hold all fifteen files named in
   `src-tauri/src/licences.rs`. The in-app screen works even if it does not — the
   text is compiled in — but a redistributor who wants to read the notices out of
   the bundle would be stuck.
3. Serve the CC-CEDICT definitions under CC BY-SA 4.0 if you redistribute them,
   and keep the statement of what was changed in `licences/CC-CEDICT.txt`
   accurate if you change the data pipeline.
4. If you intend to distribute commercially, confirm the terms yourself. Nothing
   here is legal advice. The Arphic Public License is a permissive free-font
   licence rather than a copyleft one, but it does carry notice obligations; the
   LGPL has its own conditions on the derived `dictionary.txt`; and CC BY-SA is a
   share-alike licence, which is the one with real consequences for derived data.
5. **Keep the espeak-ng pointer reachable if you ship a mobile build.** The Android
   and iOS artifacts redistribute GPL-3.0-or-later code (see "espeak-ng is
   redistributed by the mobile builds" above), so the licence text and the route to
   its source have to travel with them — which they do, through the catalogue and
   the bundle. What a distributor has to keep true is the other half: that the
   source named there stays reachable, and that the vendored copy really is
   unmodified. If you patch anything under the GPL, that patch becomes yours to
   publish.
6. **Re-check this set rather than trusting it.** A dependency added since the last
   release can bring a licence with it — that is exactly how the espeak-ng
   obligation above went unnoticed for three milestones — so this project re-reads
   this file and
   [`docs/research/ANDROID_LICENCE_ATTRIBUTION.md`](docs/research/ANDROID_LICENCE_ATTRIBUTION.md)
   before every shippable release. What has to hold is narrower than completeness:
   **nothing shipped may be under a licence this project's AGPL-3.0-only cannot
   live with.** At this commit nothing is: the set is Apache-2.0, MIT, the Arphic
   Public License, CC BY-SA 4.0, the SIL OFL, public domain, and the
   GPL-3.0-or-later that §13 permits.
7. **Do not bundle the speech model without reading its terms.** This app fetches
   it for the learner rather than shipping it, which is why its licence is
   recorded above rather than satisfied here. An offline installer, a mirror or a
   pre-seeded cache changes that, and the FunASR model agreement is not a free
   licence in the sense the rest of this file uses.

## Building from source

A clone needs one build input that is not committed and not obtained by cargo:

```bash
scripts/fetch-sherpa.sh      # unpacks the pinned sherpa-onnx native library
```

It records the archive's SHA-256 and refuses a platform whose digest has not been
recorded, rather than downloading something unverified; the header explains how to
add one. Without it the crate's own build script would fetch a binary during
`cargo build`, which is the thing this script exists to prevent. CI runs it as its
own step.

## Fonts in the UI

The interface renders Chinese text — lesson lists, word tiles, headings — in
**Noto Sans SC**, bundled at `src/assets/fonts/NotoSansSC-VF.ttf` and declared in
`src/app.css`. Bundling it means the app looks the same on a machine that has no
CJK fonts installed and downloads nothing on first run. The system CJK stack
(PingFang SC, Hiragino Sans GB, Heiti SC, Noto Sans CJK SC, Microsoft YaHei) is
still declared behind it as a fallback.

The practice canvas does not use a font at all: it draws the stored vector
outlines.
