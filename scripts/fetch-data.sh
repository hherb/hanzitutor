#!/usr/bin/env bash
#
# Download the upstream material the app is built from: the datasets, the
# licence texts that must travel with them, and the interface font.
#
#   data/raw/    the datasets, ~33 MB of text. Not committed: they can always be
#                re-fetched, and `pnpm run prepare-data` compacts them into a
#                single artifact.
#   licences/    the upstream notice and licence texts. Committed, because every
#                one of them has to ship inside the application bundle. See
#                licences/README.md.
#   src/assets/fonts/  Noto Sans SC, the CJK face the interface draws text with.
#                Committed, so a clone builds without a download.
#
# Every destination here is skipped when it already exists, so this is safe to
# re-run and does nothing on a fresh clone of a complete checkout.
#
#   graphics.txt, dictionary.txt  Make Me a Hanzi
#                                 (Arphic Public License / LGPL-3.0-or-later)
#   hanziDB.csv                   MIT, derived from Jun Da's frequency list
#   hsk-words.json                HSK 2.0/3.0 vocabulary, MIT compilation whose
#                                 definitions come from CC-CEDICT (CC BY-SA 4.0)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RAW="$ROOT/data/raw"
LICENCES="$ROOT/licences"
FONTS="$ROOT/src/assets/fonts"
mkdir -p "$RAW" "$LICENCES" "$FONTS"

MMAH="https://raw.githubusercontent.com/skishore/makemeahanzi/master"
HANZIDB="https://raw.githubusercontent.com/ruddfawcett/hanziDB.csv/master"
HSKVOCAB="https://raw.githubusercontent.com/drkameleon/complete-hsk-vocabulary/main"
GOOGLEFONTS="https://raw.githubusercontent.com/google/fonts/main/ofl/notosanssc"

# fetch <url> <destination> [optional]
fetch() {
  local url="$1" dest="$2" optional="${3:-}"
  if [ -s "$dest" ]; then
    echo "  have  $(basename "$dest")  ($(du -h "$dest" | cut -f1))"
    return 0
  fi
  echo "  get   $(basename "$dest")"
  if curl --fail --location --silent --show-error "$url" --output "$dest.partial"; then
    mv "$dest.partial" "$dest"
    echo "        -> $(du -h "$dest" | cut -f1)"
  else
    rm -f "$dest.partial"
    if [ -n "$optional" ]; then
      echo "        warning: not available at $url" >&2
      return 0
    fi
    echo "        failed: $url" >&2
    return 1
  fi
}

echo "fetching datasets into $RAW"
fetch "$MMAH/graphics.txt"   "$RAW/graphics.txt"
fetch "$MMAH/dictionary.txt" "$RAW/dictionary.txt"
fetch "$HANZIDB/data/hanziDB.csv" "$RAW/hanziDB.csv"
fetch "$HSKVOCAB/complete.min.json" "$RAW/hsk-words.json"

# The notices are committed and bundled, so this only restores one that was
# deleted. They are fetched verbatim rather than transcribed, because a notice
# that has been reworded by hand is no longer the notice the licence requires.
echo "fetching upstream licence texts into $LICENCES"
fetch "$MMAH/COPYING"               "$LICENCES/MakeMeAHanzi-COPYING.txt"
fetch "$MMAH/LGPL"                  "$LICENCES/LGPL-3.0.txt"
fetch "$MMAH/APL/english/ARPHICPL.TXT" "$LICENCES/Arphic-Public-License.txt"
fetch "$HANZIDB/LICENSE"            "$LICENCES/MIT-hanziDB.txt"
fetch "$HSKVOCAB/LICENSE"           "$LICENCES/MIT-complete-hsk-vocabulary.txt"
fetch "https://creativecommons.org/licenses/by-sa/4.0/legalcode.txt" \
      "$LICENCES/CC-BY-SA-4.0.txt"
fetch "$GOOGLEFONTS/OFL.txt"        "$LICENCES/OFL-1.1.txt"

# Noto Sans SC, OFL-1.1. Variable weight, so one ~17 MB file covers every weight
# the interface asks for. The canvas does not use it: it draws the stored vector
# outlines, so the font is only for Chinese rendered as text.
echo "fetching the interface font into $FONTS"
fetch "$GOOGLEFONTS/NotoSansSC%5Bwght%5D.ttf" "$FONTS/NotoSansSC-VF.ttf"

echo "done"
