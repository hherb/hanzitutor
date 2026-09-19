#!/usr/bin/env bash
#
# Download the upstream datasets that the app is built from.
#
# Everything lands in data/raw/, which is not committed: it is ~33 MB of text
# and can always be re-fetched. `pnpm run prepare-data` then compacts it into a
# single artifact.
#
# The upstream licence texts are fetched alongside the data, because both
# datasets require their notices to travel with any redistribution. See
# LICENSES.md.
#
#   graphics.txt, dictionary.txt  Make Me a Hanzi
#                                 (Arphic Public License / LGPL-3.0-or-later)
#   hanziDB.csv                   MIT, derived from Jun Da's frequency list
#   hsk-words.json                HSK 2.0/3.0 vocabulary, MIT compilation whose
#                                 definitions come from CC-CEDICT (CC BY-SA 4.0)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RAW="$ROOT/data/raw"
mkdir -p "$RAW"

MMAH="https://raw.githubusercontent.com/skishore/makemeahanzi/master"
HANZIDB="https://raw.githubusercontent.com/ruddfawcett/hanziDB.csv/master"
HSKVOCAB="https://raw.githubusercontent.com/drkameleon/complete-hsk-vocabulary/main"

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

echo "fetching upstream licence texts"
fetch "$MMAH/COPYING"    "$RAW/COPYING-makemeahanzi" optional
fetch "$MMAH/LGPL"       "$RAW/LGPL-makemeahanzi"    optional
fetch "$HANZIDB/LICENSE" "$RAW/LICENSE-hanziDB"      optional
fetch "$HSKVOCAB/LICENSE" "$RAW/LICENSE-hsk-vocabulary" optional
echo "done"
