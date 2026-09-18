# Data provenance and licences

Hanzi Tutor's own source code is licensed under the **GNU Affero General Public
License, version 3** (see `LICENSE`). The **data** it ships with comes from two
upstream projects under different licences, and both require their notices to be
included with any redistribution. This file records what came from where, and
what has to travel with it.

## What is bundled

The app embeds a single generated file, `crates/hanzi-core/data/hanzi.bin.gz`,
built by `prepare-data` from three upstream files. Removing that artifact removes
all third-party data from the build.

| Data | Source | Licence |
| --- | --- | --- |
| Stroke outlines (`strokes`) | Make Me a Hanzi `graphics.txt` | Arphic Public License |
| Stroke centre-lines (`medians`) | Make Me a Hanzi `graphics.txt` | Arphic Public License |
| Etymology hints | Make Me a Hanzi `dictionary.txt` | LGPL-3.0-or-later |
| Frequency rank, pinyin, meaning, radical, HSK level | [`hanziDB.csv`](https://github.com/ruddfawcett/hanziDB.csv) | MIT |

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

## Before you distribute

1. Copy `data/raw/COPYING-makemeahanzi` (Arphic Public License) and
   `data/raw/LGPL-makemeahanzi` into the app bundle, and surface them from an
   "About / Licences" screen.
2. Include `data/raw/LICENSE-hanziDB` (MIT) too.
3. If you intend to distribute commercially, confirm the terms yourself. Nothing
   here is legal advice. The Arphic Public License is a permissive free-font
   licence rather than a copyleft one, but it does carry notice obligations, and
   the LGPL has its own conditions on the derived `dictionary.txt`.

## Fonts in the UI

The interface uses the system CJK font stack — PingFang SC, Hiragino Sans GB,
Heiti SC, Noto Sans CJK SC, Microsoft YaHei — and bundles no font files. The
practice canvas does not use a font at all: it draws the stored vector outlines.
