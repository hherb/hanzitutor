# Data provenance and licences

Hanzi Tutor's own source code is licensed under the **GNU Affero General Public
License, version 3** (see `LICENSE`). The **data** it ships with comes from
several upstream projects under different licences, and all of them require
their notices to be included with any redistribution. This file records what
came from where, and what has to travel with it.

Everything described here is **already inside the built app**. There is no
first-run download, no data step for the reader, and no network access at run
time: the dataset is compiled into the executable and the notices are compiled
in beside it, with plain-text copies in the bundle's `Resources/licences/`. This
document is the human-readable record of that arrangement.

One of the entries below is **code rather than data**. There are in fact three,
and they are the only third-party libraries compiled into the binary: SQLite and
its `rusqlite` bindings, which hold the study database, and `cpal`, which opens
the microphone for tone practice. The rest is data or a font. Each is recorded
here rather than left implicit, and none of them adds a condition beyond its own
notice.

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

The first six rows are compacted by `prepare-data` into one generated artifact,
`crates/hanzi-core/data/hanzi.bin.gz`, which `src-tauri/src/state.rs` embeds with
`include_bytes!`. That artifact **is committed** (about 13 MB) so that a clone,
and CI, build without downloading the 33 MB of upstream text. Removing the
artifact removes all third-party data from the build.

The font is committed at `src/assets/fonts/NotoSansSC-VF.ttf` (about 17 MB) and
is copied into the frontend bundle by Vite. See "Fonts in the UI" below.

The notice texts live in [`licences/`](licences/), committed. They are listed in
`src-tauri/src/licences.rs` — that catalogue is what the app's Licences screen
shows, and what `tauri.conf.json` copies into the bundle. A test fails if a file
in `licences/` is not catalogued, if a catalogued file is missing, or if the
bundle config does not copy exactly that set, so a notice cannot be half-added.

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
so no model weights and no speech-recognition library are involved. That is a
deliberate boundary, argued in
[`docs/research/ASR_TTS_CLAUDE_RESEARCH.md`](docs/research/ASR_TTS_CLAUDE_RESEARCH.md)
§6, and it is what keeps the app's "no downloads, no network" promise intact.

### Noto Sans SC — <https://github.com/notofonts/noto-cjk>

The interface font is licensed under the **SIL Open Font License 1.1**, which
permits bundling and redistribution and requires the licence to travel with the
font. It ships as `licences/OFL-1.1.txt`. Noto is derived from Adobe's Source Han
Sans, which is why the OFL file carries Adobe's copyright line as well; the
font's reserved name is `Source`, not `Noto`, so no rename is required.

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

   That directory must hold all thirteen files named in `src-tauri/src/licences.rs`.
   The in-app screen works even if it does not — the text is compiled in — but a
   redistributor who wants to read the notices out of the bundle would be stuck.
3. Serve the CC-CEDICT definitions under CC BY-SA 4.0 if you redistribute them,
   and keep the statement of what was changed in `licences/CC-CEDICT.txt`
   accurate if you change the data pipeline.
4. If you intend to distribute commercially, confirm the terms yourself. Nothing
   here is legal advice. The Arphic Public License is a permissive free-font
   licence rather than a copyleft one, but it does carry notice obligations; the
   LGPL has its own conditions on the derived `dictionary.txt`; and CC BY-SA is a
   share-alike licence, which is the one with real consequences for derived data.

## Fonts in the UI

The interface renders Chinese text — lesson lists, word tiles, headings — in
**Noto Sans SC**, bundled at `src/assets/fonts/NotoSansSC-VF.ttf` and declared in
`src/app.css`. Bundling it means the app looks the same on a machine that has no
CJK fonts installed and downloads nothing on first run. The system CJK stack
(PingFang SC, Hiragino Sans GB, Heiti SC, Noto Sans CJK SC, Microsoft YaHei) is
still declared behind it as a fallback.

The practice canvas does not use a font at all: it draws the stored vector
outlines.
