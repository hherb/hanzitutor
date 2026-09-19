# Data provenance and licences

Hanzi Tutor's own source code is licensed under the **GNU Affero General Public
License, version 3** (see `LICENSE`). The **data** it ships with comes from
several upstream projects under different licences, and all of them require
their notices to be included with any redistribution. This file records what
came from where, and what has to travel with it.

## What is bundled

The app embeds a single generated file, `crates/hanzi-core/data/hanzi.bin.gz`,
built by `prepare-data` from four upstream files. Removing that artifact removes
all third-party data from the build.

| Data | Source | Licence |
| --- | --- | --- |
| Stroke outlines (`strokes`) | Make Me a Hanzi `graphics.txt` | Arphic Public License |
| Stroke centre-lines (`medians`) | Make Me a Hanzi `graphics.txt` | Arphic Public License |
| Etymology hints | Make Me a Hanzi `dictionary.txt` | LGPL-3.0-or-later |
| Frequency rank, pinyin, meaning, radical, HSK level | [`hanziDB.csv`](https://github.com/ruddfawcett/hanziDB.csv) | MIT |
| Words: characters, HSK 3.0 level, derived rank | [complete-hsk-vocabulary](https://github.com/drkameleon/complete-hsk-vocabulary) | MIT |
| Word readings and definitions | CC-CEDICT (via complete-hsk-vocabulary) | CC BY-SA 4.0 |

### Make Me a Hanzi — <https://github.com/skishore/makemeahanzi>

`graphics.txt` and `svgs.tar.gz` are derived from two free fonts, **Arphic PL
KaitiM GB** and **Arphic PL UKai**, released by Arphic Technology Co., Ltd. under
the Arphic Public License. Redistribution and modification are permitted,
provided the licence text is included and modified versions are clearly marked
as such. The app does not modify the outlines; it only re-encodes them.

`dictionary.txt` is derived from [Unihan](https://unicode.org/charts/unihan.html)
and [CJKlib](https://github.com/cburgmer/cjklib) and is distributed under the
**GNU Lesser General Public License, version 3 or later**.

The upstream `COPYING` and `LGPL` texts are fetched into
`data/raw/COPYING-makemeahanzi` and `data/raw/LGPL-makemeahanzi` by
`scripts/fetch-data.sh`. **Both must be included when redistributing the app.**

### hanziDB.csv — <https://github.com/ruddfawcett/hanziDB.csv>

MIT licensed. The list is derived from Jun Da's Modern Chinese Character
Frequency List and uses simplified characters. Its `LICENSE` text is fetched into
`data/raw/LICENSE-hanziDB`.

### complete-hsk-vocabulary — <https://github.com/drkameleon/complete-hsk-vocabulary>

MIT licensed (Copyright © Yanis Zafirópulos). This is the source of the **word
list**: each word's simplified characters, its HSK 3.0 level, and the derived
frequency used for ordering. The list itself is compiled from the official
HSK 2.0/3.0 vocabulary, including
[elkmovie/hsk30](https://github.com/elkmovie/hsk30) (MIT, extracted from the
official Ministry of Education PDF).

Its `LICENSE` is fetched into `data/raw/LICENSE-hsk-vocabulary`.

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

Note also what is deliberately **not** taken from that file: its `q` (frequency
from SUBTLEX-CH) and `p` (part-of-speech) fields are ignored, so the app carries
nothing derived from those sources. Its `r` radical field is ignored too, the
radical coming from Make Me a Hanzi as before. The rank the app does show for a
word is computed here from the MIT character frequency list.

## Before you distribute

1. Copy `data/raw/COPYING-makemeahanzi` (Arphic Public License) and
   `data/raw/LGPL-makemeahanzi` into the app bundle, and surface them from an
   "About / Licences" screen.
2. Include `data/raw/LICENSE-hanziDB` (MIT) and
   `data/raw/LICENSE-hsk-vocabulary` (MIT) too.
3. Include the **CC-CEDICT** attribution and a link to
   <https://creativecommons.org/licenses/by-sa/4.0/> for the word readings and
   definitions, and mark that dictionary text as CC BY-SA 4.0.
4. If you intend to distribute commercially, confirm the terms yourself. Nothing
   here is legal advice. The Arphic Public License is a permissive free-font
   licence rather than a copyleft one, but it does carry notice obligations; the
   LGPL has its own conditions on the derived `dictionary.txt`; and CC BY-SA is a
   share-alike licence, which is the one with real consequences for derived data.

## Fonts in the UI

The interface uses the system CJK font stack — PingFang SC, Hiragino Sans GB,
Heiti SC, Noto Sans CJK SC, Microsoft YaHei — and bundles no font files. The
practice canvas does not use a font at all: it draws the stored vector outlines.
