# Data provenance and licences

Hanzi Tutor's own source code is licensed under the **GNU Affero General Public
License, version 3** (see `LICENSE`). The **data** it ships with comes from
several upstream projects under different licences, and all of them require
their notices to be included with any redistribution. This file records what
came from where, and what has to travel with it.

Everything described in "What is bundled" is **already inside the built app**.
There is no first-run data step for the reader: the dataset is compiled into the
executable and the notices are compiled in beside it, with plain-text copies in
the bundle's `Resources/licences/`. Nothing in that table needs the network to be
read, and this document is the human-readable record of that arrangement.

Nothing *needs* the network, but three things can **use** it, and not one of them
happens unless the learner asks for it by name. The check is the project's own —
`grep -rln "ureq\|reqwest" src-tauri/src crates/*/src` — which finds exactly these
three modules:

| Feature | Module | What it sends |
| --- | --- | --- |
| Recognising *which* syllable was said. Optional: a button in the settings screen | `crates/hanzi-hearing/src/asr.rs` | nothing. A 163 MB model is fetched, checked against a published digest, and run on the device |
| Speaking a phrase the app has no recording of. Optional, the same way | `src-tauri/src/say.rs` | nothing. About 61 MB of synthesis files are fetched and checked the same way |
| Syncing between the learner's own devices. Off until they connect **their own** Dropbox account | `crates/hanzi-sync/src/http.rs` | the study data they chose to sync, to that account and nowhere else, over HTTPS |

The two models are downloads the learner starts from the settings screen. This app
does not redistribute their weights and does not fetch them on anyone's behalf; it
shows the address, the size and the licence, and downloads only when the button is
pressed. See "The speech model is downloaded, not shipped" below. The README's
promise is restated the same way: the app downloads nothing *unless you ask it
to*, and everything the bundled course teaches still needs nothing.

`docs/privacy-policy.md` is the public statement of these same three paths, and
the Play listing's Data safety answers are built from it. If a fourth is ever
added, all of them have to change together: the project's rule is that the
listing, the README and this file name every way the app can reach the network,
and this paragraph is what going stale looks like — it said "exactly one
exception" while there were three, and never mentioned syncing at all.

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

## The Japanese kana data — committed, and shipped by the Japanese app

`crates/nihongo-core/` holds the Japanese tutor's data layer: a kana dataset built
from **AnimCJK**, and the `merge_strokes` repair that source needs. The **artifact
is committed** — `crates/nihongo-core/data/kana.bin.gz`, 177 kana — and
`apps/nihongo-tutor` embeds it. Its notice therefore travels in that app's own
catalogue, `apps/nihongo-tutor/src-tauri/src/licences.rs`, rather than in the
shared `licences/` directory, which the *Chinese* app catalogues and which must not
gain a notice for data it does not contain.

Three upstream sources, and they are not interchangeable:

| Source | What is taken | Licence |
| --- | --- | --- |
| [AnimCJK](https://github.com/parsimonhi/animCJK) `graphicsJaKana.txt` | Stroke outlines and centre-lines. **Redistributed**, compacted into the artifact | **Ambiguous — see below.** Either the Arphic Public License or LGPL-3.0-or-later, and both travel with this repository |
| [AnimCJK](https://github.com/parsimonhi/animCJK) `svgsJaKana/*.svg` | Only the **element ids**, read at build time to recover which drawing segments make up one taught stroke. Not redistributed | LGPL-3.0-or-later |
| [KanjiVG](https://github.com/KanjiVG/kanjivg) `kanji/*.svg` | Only the **stroke count**, read at build time as an independent check on the merge. Not redistributed | CC BY-SA 3.0 |

AnimCJK splits its licence by file kind, and the split is awkward for exactly the file this project
uses. Its `COPYING.txt` assigns *"text files prefixed by `graphics`"* to the **Arphic Public
License**, but the only reason its character SVGs are Arphic-licensed is that they were traced from
the Arphic fonts — and the same file puts *"SVG files … representing kana or strokes"* under
**LGPL-3.0-or-later** precisely *because kana are not so derived*. `graphicsJaKana.txt` is a
`graphics*` file whose content is kana, so it falls in the gap between the two clauses.

**This is not a blocker, and it is not worked around by taking a view.** Both candidate licences are
already satisfied by notices this repository ships: `licences/Arphic-Public-License.txt` (for Make
Me a Hanzi, which is under the identical licence) and `licences/LGPL-3.0.txt`. Both permit
redistribution and modification of the data with the notice attached. The ambiguity is recorded
rather than resolved because it is upstream's to resolve, and an issue should be raised with AnimCJK
to say which clause they intend for the kana `graphics` files.

The copyright holders are the AnimCJK project (FM&SH); upstream's own statement is
[`licenses/COPYING.txt`](https://github.com/parsimonhi/animCJK/blob/master/licenses/COPYING.txt).

**What this project changed**, as LGPL-3.0 §2 requires be stated: nothing in the
outlines or centre-lines themselves. AnimCJK stores a stroke that crosses itself
as several drawing segments — あ arrives as four where three are taught — so the
build folds each such stroke back into one, concatenating the outline paths and
keeping the first centre-line (the later ones are displaced copies for the
animation). Over all 177 kana that collapses 27 segments; 25 characters are
affected. The grouping is read out of AnimCJK's own SVG element ids, and every
one of the 177 resulting stroke counts is checked against KanjiVG before the
artifact is written — `prepare-kana` refuses to write one it could not check,
unless `--allow-unchecked` is passed deliberately.

KanjiVG earns no entry of its own, and that is a judgement rather than an
oversight: no KanjiVG file, and no adaptation of one, is redistributed. It is
read for a number — how many strokes a kana has — and a stroke count is a fact
about the language, not a copy of the work. The pipeline only needs it at build
time, and it is fetched into `data/raw/kvgJa/`, which is not committed.

## The Japanese kanji data — committed, and shipped by the Japanese app

`crates/nihongo-core/data/kanji.bin.gz` is the jōyō set: **2,136 characters** with
their outlines, centre-lines, readings, English glosses, grades, radicals and IDS
decompositions, **plus the 214 Kangxi radical head forms** and their geometry,
about 3.2 MB. It is committed for the same reason the kana artifact is — a clone
builds without the network — and `apps/nihongo-tutor` embeds it and serves it from
its kanji and radicals screens. The notices below therefore travel with it, in that
app's own `licences/` directory and `licences.rs` catalogue, for the same reason
the kana notices are app-local.

Four sources, and what each contributes is separated deliberately:

| Source | What is taken | Licence |
| --- | --- | --- |
| [AnimCJK](https://github.com/parsimonhi/animCJK) `graphicsJa.txt` | Every stroke's outline **and** its centre-line. **Redistributed**, compacted into the artifact | **Arphic Public License** |
| [AnimCJK](https://github.com/parsimonhi/animCJK) `dictionaryJa.txt` | The grade sets, the radical the character is written with and its note, the IDS decomposition, and the **214 radical head forms** in file order. **Redistributed**, compacted into the artifact | LGPL-3.0-or-later |
| [KANJIDIC2](https://www.edrdg.org/wiki/index.php/KANJIDIC_Project) via [`scriptin/jmdict-simplified`](https://github.com/scriptin/jmdict-simplified) | Readings with their okurigana, English glosses, the frequency rank, and the **current** kyōiku grade. **Redistributed**, compacted into the artifact | **CC BY-SA 4.0** (EDRDG) |
| [KanjiVG](https://github.com/KanjiVG/kanjivg) `kanji/*.svg` | Only the **stroke count**, read at build time as an independent check. Not redistributed | CC BY-SA 3.0 |

### The Arphic question, which for kanji has an answer

The kana section above records an ambiguity: AnimCJK's `COPYING.txt` assigns
*"text files prefixed by `graphics`"* to the Arphic Public License and *"SVG files
… representing kana or strokes"* to LGPL-3.0-or-later, and the kana `graphics`
file falls between the two clauses. **For kanji it does not.** The same statement
reads, verbatim:

> You can redistribute and/or modify text files prefixed by "graphics" and SVG
> files of AnimCJK project representing a character under the terms of the Arphic
> Public License … You can redistribute and/or modify all other files (including
> SVG files of AnimCJK project representing kana or strokes) under the terms of
> the GNU Lesser General Public License …

`graphicsJa.txt` is a `graphics*` file, and its content is characters rather than
kana, so both limbs of the first sentence cover it. The kanji geometry is
**Arphic, unambiguously** — and that is the licence this repository already ships
and has already adjudicated for Make Me a Hanzi (`licences/Arphic-Public-License.txt`,
and `HANDOVER.md`'s record of the position). AnimCJK's own derivation list names
Make Me a Hanzi and the two Arphic fonts, which is consistent: the kanji geometry
is traced from Arphic-derived material in the first place, which is *why* the
LGPL clause exists for the kana alone.

The copyright holders are the AnimCJK project (FM&SH); upstream's statement is
[`licenses/COPYING.txt`](https://github.com/parsimonhi/animCJK/blob/master/licenses/COPYING.txt).

### `dictionaryJa.txt` — LGPL-3.0-or-later, and the fields this project declines

AnimCJK grants everything that is not a `graphics*` file or a character SVG under
LGPL-3.0-or-later, and `dictionaryJa.txt` is such a file. AnimCJK also states that
its `dictionary` files derive in part from the **Unihan** database (Unicode License
v3), which is why the fields this project reads from it are the ones AnimCJK
itself classifies — the grade set, the radical and its note, the IDS
decomposition — and not Unihan text.

The file carries `on`, `kun` and `definition` fields as well, and
`prepare-kanji` **does not read them**. That is a provenance decision, not an
oversight: readings and glosses in this artifact come from EDRDG, which is the
attributed authority for them, so a second-hand copy would misrecord where the
text a learner reads came from — and it would redistribute unspecified text under
AnimCJK's LGPL rather than under the licence that governs it.

**What this project changed**, as LGPL-3.0 §2 and CC BY-SA both require be stated:
the outlines and centre-lines are unmodified, and unlike the kana no stroke is
merged — AnimCJK splits a kana stroke that crosses itself and does not do that to a
kanji (measured over all 2,136, and over the 91 whose KANJIDIC2 entry lists more
than one stroke count, which is the only place a split could hide). What *is*
changed: the set is narrowed from AnimCJK's 7,007 characters to the 2,136 jōyō;
each centre-line is converted from Make Me a Hanzi's font space into the display
space the grader works in, exactly as the kana centre-lines are; and the data is
re-encoded from JSON lines into the compact artifact.

**The radical table** is the same file's 214 entries whose `set` contains
`radical`, kept in file order — **the position is the Kangxi number** — with each
head form's geometry from `graphicsJa.txt` and its number checked against
KANJIDIC2's classical radical number. Two of the 214 are recorded rather than
checked, because KANJIDIC2 files them under the other form of the same radical:
戶 (63) and 靑 (174), where EDRDG has 戸 and 青. The entries' own `definition`
glosses are not read, which is a measurement rather than tidiness: three of the
214 state the wrong number ("Kangxi radical 136" for 耒 at position 127, 133 for
臼 at 134, 156 for 足 at 157), so the number could not be taken from them even if
the provenance allowed it.

### KANJIDIC2 — CC BY-SA 4.0, with the update obligation designed

KANJIDIC2 is the work of the **Electronic Dictionary Research and Development
Group** (EDRDG), and this project takes it in the JSON form published by
`scriptin/jmdict-simplified` rather than the XML, so that the build needs no new
XML parser. The reformatting is a derivative under the same licence.

* **No copyright is claimed over it.** The glosses, readings, grades, frequency
  ranks and stroke counts that reach a learner's screen are EDRDG's work.
  KANJIDIC2's SKIP codes are Jack Halpern's, and EDRDG asks that they be credited
  to him and to <https://www.kanji.org/>; this artifact does not carry them,
  because `prepare-kanji` reads no query codes, but anything that later does must
  credit him.
* **Share-alike, discharged in this file and in the app.** The artifact is a
  derivative of CC BY-SA 4.0 material and stays under it. A future Japanese app
  must show the attribution on its **About and licences** screen — a start-up
  mention is explicitly not enough — which is the screen
  `apps/nihongo-tutor/src-tauri/src/licences.rs` already builds.
* **What EDRDG actually asks a software package for, and where each part ships.**
  The Group's statement is narrower than "attribute us" and has four limbs, so it
  is worth checking against the app rather than assuming: the *usage and source* of
  the files must be acknowledged in the documentation and on a screen reached from
  a menu; *copies of the documentation and licence files* must be provided; *links*
  to local copies or to EDRDG's own pages must be given; and the data must be kept
  up to date. In this app:

  | EDRDG's requirement | Where it is met |
  | --- | --- |
  | acknowledge usage and source, from a menu | the `edrdg` notice on the Licences screen, whose text names the Group, both dictionaries and both projects' pages at `edrdg.org` |
  | provide copies of the licence files | `licences/CC-BY-SA-4.0.txt` ships in the bundle (`ccbysa` in the catalogue), as does the notice itself |
  | provide links to local copies or to EDRDG | the same notice carries the licence statement's URL and the two project URLs; the screen renders them |
  | keep the data updated | the refresh procedure below, with the pinned snapshot recorded in the artifact and asserted by its test |

  Three of those four were **not** met before `ROADMAP_NIHONGO.md` N9 and are worth
  naming as the shape of the mistake: the only EDRDG text that shipped was a bullet
  inside `PROVENANCE.md`, no CC BY-SA text travelled with the app at all (though the
  *Chinese* app has shipped the identical legal code for two milestones), and no
  notice said that the data was a modified subset. `tests/licences.rs` now reads
  every catalogued notice's text rather than only the five it used to name, and
  `the_share_alike_notices_say_what_was_selected_and_changed` fails if the notice
  stops stating what was taken and what was dropped.
* **The snapshot is pinned and recorded.** `scripts/fetch-data.sh` pins the
  `scriptin/jmdict-simplified` release tag rather than following `latest`, and
  `prepare-kanji` copies that document's own `version` and `dictDate` into the
  artifact. `crates/nihongo-core/tests/kanji_artifact.rs` then pins both, so the
  snapshot a release shipped is readable from the test rather than from memory.

**The refresh procedure**, which is the obligation EDRDG's licence actually
imposes — its terms say that failing to keep the data updated "is a violation of
the licence to use the data", so a pinned artifact with no path forward is the
trap here rather than a safe default:

1. Set `JMDICT_SIMPLIFIED` in `scripts/fetch-data.sh` to the new release tag.
2. Remove the cached document and re-run the fetch, which also re-fetches the
   KanjiVG cross-check for whatever the new snapshot calls jōyō:

   ```bash
   rm -f data/raw/kanjidic2-all.json data/raw/kanjidic2-all-*.json.tgz
   ./scripts/fetch-data.sh
   pnpm run prepare-kanji
   ```

3. Read what moved. `prepare-kanji` prints every number the artifact test pins —
   the grade totals, the count of characters the two grade sources disagree
   about, and the KanjiVG agreement — and it *fails* rather than warns if a stroke
   count no longer checks out. `git diff --stat crates/nihongo-core/data/kanji.bin.gz`
   says whether the artifact changed at all.
4. Update the pinned version and date in
   `tests/kanji_artifact.rs` and commit them **with** the artifact, so the two can
   never disagree in the history.

A snapshot that moves a character between grades, or that adds one to jōyō, is a
decision rather than a rebuild: `prepare-kanji` refuses to write an artifact whose
jōyō set is not 2,136 characters, and says so.

### KanjiVG earns no entry here either

For the same reason as the kana: no KanjiVG file, and no adaptation of one, is
redistributed. Only a stroke count is read, at build time, as the independent
check on the number the grading engine depends on, and `data/raw/kvgJa/` is not
committed. One character's count differs from KanjiVG's (衷: 10 against 9) and
that exception is written into `prepare-kanji` with both numbers, so the check is
two-sided rather than a silent pass.

## The Japanese vocabulary data — committed, and not yet in any bundle

`crates/nihongo-core/data/words.bin.gz` is the vocabulary the kanji course teaches
*through*: **21,902 words**, each with its written form, its own reading, the
furigana that puts that reading over the right characters, an English gloss, EDRDG's
frequency block and this project's ladder band. **833,858 bytes**, committed for the
same reason the other two artifacts are.

Three sources, and the first two are the same dictionary twice:

| Source | What is taken | Licence |
| --- | --- | --- |
| [JMdict](https://www.edrdg.org/jmdict/j_jmdict.html) via [`scriptin/jmdict-simplified`](https://github.com/scriptin/jmdict-simplified) (`jmdict-eng-…`) | The written form, the reading, and the English glosses. **Redistributed**, compacted into the artifact | **CC BY-SA 4.0** (EDRDG) |
| [JMdict_e](http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz) — EDRDG's own XML | **One field**: the `nf01`–`nf48` priority rank. Nothing else is parsed, and no text from it is redistributed | **CC BY-SA 4.0** (EDRDG) |
| [JmdictFurigana](https://github.com/Doublevil/JmdictFurigana) | The alignment of each reading to the characters it belongs to. **Redistributed**, compacted into the artifact | **MIT**, Copyright (c) 2025 Doublevil |

EDRDG's licence statement names both dictionaries in its scope — it applies to
"JMDICT … the Japanese and English components" as well as to KANJIDIC2 — and it
applies to "any data files which are derived from them", which is exactly what this
artifact is. So everything the KANJIDIC2 section above says about attribution,
share-alike and the update obligation holds here too, and the two are refreshed
together.

### The same dictionary, fetched twice — and why that is not redundancy

`scriptin/jmdict-simplified`'s JSON reformatting **drops the `nf01`–`nf48` priority
tags entirely**. That was measured rather than assumed: the full `jmdict-eng`
document — 118 MB, 218,840 entries — carries eleven distinct tags in total
(`sK rK sk ok ateji io iK oK ik gikun rk`), all spelling-variant markers, and not one
`nf`, `news1`, `ichi1`, `spec1` or `gai1`. EDRDG's own XML has them: 22,430 entries
carry an nf rank. So the fetch takes both files, and `prepare-words` joins them on
`ent_seq` — 22,430 of 22,430 ids matched — scanning the XML for `<ent_seq>` and the
`nf` tokens in `<ke_pri>`/`<re_pri>` and nothing else. Reading two fields out of a
63 MB document with a targeted scan is deliberate; an XML parser for this would be
the tail wagging the dog, and the assumption it rests on (ASCII tokens, no nesting)
is one the pipeline fails loudly on rather than silently mis-reading.

### English only, twice over

The `-eng` documents are used rather than `-all`, and the gloss filter keeps only
`lang == "eng"`, for two reasons that happen to agree. A screen that mixed four
languages in one gloss list would be unusable; and **EDRDG's statement says the
translational equivalents in other languages "are covered by separate copyright held
by the compilers of that material"** — so they are not this project's to
redistribute under the Group's grant in the first place. The same reasoning applies
to the French, Spanish and Portuguese meanings that `kanjidic2-all.json` also
carries, which is why `prepare-kanji` filters them out too.

### The ladder is this project's, and the app has to say so

Each word carries a **band**, and the band is *not* the JLPT's. There has been no
official JLPT kanji or vocabulary list since 2010, and the best-licensed community
list chains to a source that asserts no licence, so a band is derived instead: **the
highest kyōiku grade among the word's kanji**, 1–6, with band 7 for a word
containing a kanji from the jōyō remainder, and EDRDG's `nf` ranking ordering the
words *within* a band. `crates/nihongo-core/src/words.rs` documents the rule and
`tests/words_artifact.rs` recomputes every band from the kanji artifact, so the two
committed artifacts cannot drift apart. A future screen must present the bands as
this project's approximation — `word::band_name` exists so that it can.

### MIT, and where its notice goes

JmdictFurigana is MIT, Copyright (c) 2025 Doublevil. MIT asks for its notice to
accompany copies and substantial portions, and 21,836 aligned words is a substantial
portion, so the attribution is recorded here and the licence text travels with the
app. **It lives in `apps/nihongo-tutor/src-tauri/licences/`** — the app's own notice
directory, not the shared one — because the shared `licences/` directory is
catalogued by the *Chinese* app, which contains none of this; that is
`HANDOVER_NIHONGO.md` invariant 12's rule, and it is what the sentence that used to
stand here ("it joins them on the commit that embeds this artifact") was waiting
for. The artifact was embedded at N7 and the notice did **not** join it — the gap
N9 closed. Two catalogue entries carry it, both app-local:
`PROVENANCE-JmdictFurigana.txt` is the attribution and the statement of what was
changed, and `MIT-JmdictFurigana.txt` is the project's own licence text, fetched
verbatim from the **same pinned release** as the data
(`fetch-data.sh`, from the `${JMDICT_FURIGANA}` tag) so the two cannot come from
different releases.

`the_texts_are_the_real_licences_and_not_placeholders` in `tests/licences.rs` now
ends by asserting that **every** catalogued notice appears in its expectation table,
so a notice added to the catalogue and forgotten there — which is how this one went
unread — fails the suite rather than passing quietly.

### Refreshing it

The procedure in the KANJIDIC2 section above is the same one, with two changes:
`pnpm run prepare-words` joins step 2, and step 4's pinned version and date are the
ones in `tests/words_artifact.rs`. Both dictionaries come from the same
`scriptin/jmdict-simplified` release tag, so a single pin moves them together.
`JMdict_e.gz` is the exception: EDRDG rebuilds it daily and does not version it, so
there is nothing to pin — `prepare-words` records that file's own `JMdict created`
date in the artifact, and the counts the test pins are what catch it moving.
JmdictFurigana is released monthly and **is** pinned, in two places that must agree:
`JMDICT_FURIGANA` in `scripts/fetch-data.sh` and `FURIGANA_RELEASE` in
`prepare_words.rs`.

## The reading passages, and the analyser that segments them

`crates/nihongo-core/data/passages.bin.gz` holds the short passages the reading
screen draws, each segmented into words with a reading over every kanji. **The
passage text is this project's own writing** — nothing is quoted, adapted or
scraped — which is why the file is 502 bytes and why there is no licence question
about the prose itself. The licence question is about the **analyser**, and it is
answered below because the segmentation is derived from it.

### The passages

Written for the course, in `crates/nihongo-core/data/passages/*.txt`, and held to
the vocabulary by `prepare-passages`, which refuses to write an artifact containing
a kanji the words artifact does not teach. `ROADMAP_NIHONGO.md` N7 records why
corpora were rejected: Tatoeba is per-sentence licensed (some CC0, some BY, some
**ND**) and gives sentences rather than passages; Aozora Bunko is free but is
pre-1930s literary Japanese that uses none of this course's vocabulary; Wikipedia is
modern and unlevelled. Writing them costs writing and removes the question.

### UniDic and lindera — what the segmentation is derived from

The segmentation is **not** redistributed text and it is **not** a dictionary: it is
one analyser's answer about three short passages, stored as token boundaries and
readings. It is still derived from the analyser and its dictionary, so both are
recorded.

| Source | What the artifact takes | Licence |
| --- | --- | --- |
| [lindera](https://github.com/lindera/lindera) 6.2 | The segmentation and the readings, computed by `prepare-passages` | **MIT** |
| `unidic-mecab-2.1.2`, via [`lindera-unidic`](https://crates.io/crates/lindera-unidic) | The dictionary the analyser reads | **BSD-3-Clause-equivalent**, Copyright (c) 2011-2017, The UniDic Consortium |

UniDic's terms are the three-clause BSD form, verbatim in the crate's own
`NOTICE.txt` and reproduced here as the attribution it requires:

> Copyright (c) 2011-2017, The UniDic Consortium
> All rights reserved.
>
> Redistribution and use in source and binary forms, with or without modification,
> are permitted provided that the following conditions are met:
>
> * Redistributions of source code must retain the above copyright notice, this
>   list of conditions and the following disclaimer.
> * Redistributions in binary form must reproduce the above copyright notice, this
>   list of conditions and the following disclaimer in the documentation and/or
>   other materials provided with the distribution.
> * Neither the name of the UniDic Consortium nor the names of its contributors may
>   be used to endorse or promote products derived from this software without
>   specific prior written permission.

**Nothing UniDic contains is redistributed.** The dictionary is built into
`.lindera/`, which is gitignored, is about 500 MB unpacked, and is not part of any
bundle; only the token boundaries it produced for three passages are. The fetch is
`scripts/fetch-unidic.sh`, and the archive is pinned by MD5 in `lindera-unidic`'s
own `build.rs` (repeated in that script so a change to it is visible in a diff):

```
md5 f4502a563e1da44747f61dcd2b269e35  unidic-mecab-2.1.2.tar.gz
```

**That the notice travels with the artifact is a correction N9 made.** BSD-3-Clause
asks for the copyright notice, the conditions and the disclaimer to accompany a
binary redistribution, and the passages artifact is a derived redistribution
however small it is — but until N9 the only copy of that text was *this file*,
which is neither compiled into the app nor copied into its bundle, and the
catalogue named neither lindera nor UniDic at all. Both now ship:
`apps/nihongo-tutor/src-tauri/licences/ANALYSER-UniDic-lindera.txt` holds the
two licences verbatim and the statement that no dictionary entry is redistributed,
and it is catalogued twice — as `unidic` (BSD-3-Clause, the dictionary) and as
`lindera` (MIT, the analyser) — because the two are different works under
different licences and a reader looking for either should find it.

The dictionary is fetched from lindera's mirror, `https://Lindera.dev/`, rather
than from NINJAL, and lindera verifies that hash before it will build anything.

**The analyser is not in any application bundle.** The embedding is a build-time
step, so the shipped app carries the artifact and no Japanese dictionary, no
tokeniser and no network path — which is the same promise the rest of the app
makes, kept by moving the cost to the build.

## Before you distribute

1. **Nothing has to be gathered by hand.** The notices are in `licences/`, are
   compiled into the binary, and are copied into `Resources/licences/` by the
   bundle step. They are reachable from the app's **About and licences** screen,
   which is the fourth entry in the sidebar.
2. Run `pnpm run build` (see README.md, "Shipping a build"). It produces a signed
   `.app` and `.dmg`. Confirm the copies survived, since a resource path is the
   kind of thing that only breaks in the packaged app:

   ```bash
   # The Chinese app — fifteen files named in its src-tauri/src/licences.rs.
   APP=".cargo-target/release/bundle/macos/Hanzi Tutor.app"   # or src-tauri/target/...
   ls "$APP/Contents/Resources/licences"

   # The Japanese app — ten files named in its own catalogue.
   KANA=".cargo-target/release/bundle/macos/Nihongo Tutor.app"
   ls "$KANA/Contents/Resources/licences"
   ```

   Those directories must hold all fifteen files named in the main app's
   `src-tauri/src/licences.rs` and all ten named in
   `apps/nihongo-tutor/src-tauri/src/licences.rs` respectively. The in-app screen
   works even if they do not — the text is compiled in — but a redistributor who
   wants to read the notices out of the bundle would be stuck.
3. Serve the CC-CEDICT definitions under CC BY-SA 4.0 if you redistribute them,
   and keep the statement of what was changed in `licences/CC-CEDICT.txt`
   accurate if you change the data pipeline.
4. **The Japanese app owes EDRDG the same three things, and they are separate from
   item 3.** `apps/nihongo-tutor` embeds KANJIDIC2 and JMdict text, so it must
   acknowledge EDRDG's usage and source on a screen reached from a menu (the
   `edrdg` notice on its Licences screen), ship copies of the documentation and
   licence files (`licences/CC-BY-SA-4.0.txt`, `EDRDG-JMdict-KANJIDIC2.txt`), and
   keep the data updated (the refresh procedure in the KANJIDIC2 section above,
   which is the obligation EDRDG's terms actually enforce). All three were
   completed at N9; the section above records what was missing before it. If the
   vocabulary or kanji pipeline changes, the "what this app did to the data" list
   in `EDRDG-JMdict-KANJIDIC2.txt` has to change with it, and
   `the_share_alike_notices_say_what_was_selected_and_changed` in
   `apps/nihongo-tutor/src-tauri/tests/licences.rs` is what notices.
5. If you intend to distribute commercially, confirm the terms yourself. Nothing
   here is legal advice. The Arphic Public License is a permissive free-font
   licence rather than a copyleft one, but it does carry notice obligations; the
   LGPL has its own conditions on the derived `dictionary.txt`; and CC BY-SA is a
   share-alike licence, which is the one with real consequences for derived data.
6. **Keep the espeak-ng pointer reachable if you ship a mobile build.** The Android
   and iOS artifacts redistribute GPL-3.0-or-later code (see "espeak-ng is
   redistributed by the mobile builds" above), so the licence text and the route to
   its source have to travel with them — which they do, through the catalogue and
   the bundle. What a distributor has to keep true is the other half: that the
   source named there stays reachable, and that the vendored copy really is
   unmodified. If you patch anything under the GPL, that patch becomes yours to
   publish.
7. **Re-check this set rather than trusting it.** A dependency added since the last
   release can bring a licence with it — that is exactly how the espeak-ng
   obligation above went unnoticed for three milestones, and how the Japanese app's
   three missing notices went unnoticed from N7 to N9 — so this project re-reads
   this file and
   [`docs/research/ANDROID_LICENCE_ATTRIBUTION.md`](docs/research/ANDROID_LICENCE_ATTRIBUTION.md)
   before every shippable release. What has to hold is narrower than completeness:
   **nothing shipped may be under a licence this project's AGPL-3.0-only cannot
   live with.** At this commit nothing is: the set is Apache-2.0, MIT, BSD-3-Clause,
   the Arphic Public License, CC BY-SA 4.0, the SIL OFL, public domain, and the
   GPL-3.0-or-later that §13 permits.
8. **Do not bundle the speech model without reading its terms.** This app fetches
   it for the learner rather than shipping it, which is why its licence is
   recorded above rather than satisfied here. An offline installer, a mirror or a
   pre-seeded cache changes that, and the FunASR model agreement is not a free
   licence in the sense the rest of this file uses.
9. **No dependency's source is modified.** This is worth stating because it was not
   true for the 0.6.0 iOS resubmission: `vendor/tao` was the published `tao` 0.35.3
   (Apache-2.0) with upstream's one-line fix for the iOS launch crash applied, and
   Apache-2.0 §4(b) required the changed file to say so, which it did. That copy is
   gone — the fix is in released `tao` 0.36.0 and later, and `tauri` 2.12.0 brings it
   in (HANDOVER invariant 38). **If a dependency is ever vendored or patched again,
   the §4(b) notice comes with it:** a comment in the changed file naming the
   modification, and the crate's own licence files in the same directory.

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
