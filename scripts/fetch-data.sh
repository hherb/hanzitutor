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
#   graphicsJa.txt, dictionaryJa.txt, kanjidic2-all.json
#                                 the Japanese kanji material: geometry, the
#                                 grade sets and decompositions, and EDRDG's
#                                 readings and glosses. See the two sections
#                                 below for what each file is and is not for.
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
ANIMCJK="https://raw.githubusercontent.com/parsimonhi/animCJK/master"
KANJIVG="https://raw.githubusercontent.com/KanjiVG/kanjivg/master"

# How every download here retries. `raw.githubusercontent.com` resets the
# connection (`curl: (56)`) rather than answering when it is asked for hundreds of
# files at once — which is exactly what the kanji cross-check does — and a retry
# is the whole difference between "the file is not there" and "the fetch was
# unlucky". Deliberately *not* `--retry-all-errors`: a genuine 404 should stay a
# 404, because a missing file is something the caller diagnoses properly.
#
# Written into both functions rather than shared through a variable, because
# `fetch_quiet` is exported to a subshell for the per-character fetches and an
# exported function does not carry a variable with it.

# fetch <url> <destination> [optional]
fetch() {
  local url="$1" dest="$2" optional="${3:-}"
  if [ -s "$dest" ]; then
    echo "  have  $(basename "$dest")  ($(du -h "$dest" | cut -f1))"
    return 0
  fi
  echo "  get   $(basename "$dest")"
  if curl --fail --location --silent --show-error --retry 4 --retry-delay 1 --retry-connrefused "$url" --output "$dest.partial"; then
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
# The GPL-3.0 text, for espeak-ng. It is not a dependency of this project in any
# package manager's sense: it is inside the prebuilt sherpa-onnx library that the
# *mobile* builds link, which is why it is only owed on Android and iOS. See
# LICENSES.md — the macOS build does not contain it, and the notice says so.
fetch "https://www.gnu.org/licenses/gpl-3.0.txt" "$LICENCES/GPL-3.0.txt"

# Noto Sans SC, OFL-1.1. Variable weight, so one ~17 MB file covers every weight
# the interface asks for. The canvas does not use it: it draws the stored vector
# outlines, so the font is only for Chinese rendered as text.
echo "fetching the interface font into $FONTS"
fetch "$GOOGLEFONTS/NotoSansSC%5Bwght%5D.ttf" "$FONTS/NotoSansSC-VF.ttf"

# ---------------------------------------------------------------------------
# Japanese kana, for `pnpm run prepare-kana`.
#
# Three sources, and they are not interchangeable:
#   graphicsJaKana.txt  the geometry, one object per kana (LGPL-3.0-or-later).
#   svgsJaKana/*.svg    the same characters as SVG. Only their element ids are
#                       read, to recover which drawing segments make up one
#                       taught stroke — the graphics file has lost that, and
#                       without it あ would be stored as four strokes instead of
#                       three. LGPL-3.0-or-later.
#   kvgJa/*.svg         KanjiVG (CC BY-SA 3.0). NOT bundled and NOT an input to
#                       the artifact: it is the second opinion the stroke counts
#                       are checked against, because "how many strokes is あ"
#                       should not be answered by the code being tested.
#
# The AnimCJK kana set is the hiragana block U+3041..U+3096, the katakana block
# U+30A1..U+30FA, and the prolonged sound mark U+30FC — 177 characters, which is
# what AnimCJK publishes and what the artifact asserts.

# fetch_quiet <url> <destination>
#
# One file per URL, for the per-character material below: skipped when it is
# already there, and a failure is a warning rather than a stop, because a missing
# SVG is something `prepare-kana`/`prepare-kanji` diagnose properly — it knows
# which characters it needs and which it could not check.
fetch_quiet() {
  local url="$1" dest="$2"
  [ -s "$dest" ] && return 0
  if curl --fail --location --silent --show-error --retry 4 --retry-delay 1 --retry-connrefused "$url" --output "$dest.partial"; then
    mv "$dest.partial" "$dest"
  else
    rm -f "$dest.partial"
    echo "        warning: not available at $url" >&2
  fi
}

echo "fetching the kana material into $RAW"
fetch "$ANIMCJK/graphicsJaKana.txt" "$RAW/graphicsJaKana.txt"

KANA_SVG="$RAW/svgsJaKana"
KVG_SVG="$RAW/kvgJa"
mkdir -p "$KANA_SVG" "$KVG_SVG"

# U+3041..U+3096 and U+30A1..U+30FA, plus U+30FC.
kana_codepoints() {
  seq $((0x3041)) $((0x3096))
  seq $((0x30A1)) $((0x30FA))
  echo $((0x30FC))
}

fetched_kana=0
for cp in $(kana_codepoints); do
  before=$(ls "$KANA_SVG" 2>/dev/null | wc -l | tr -d ' ')
  fetch_quiet "$ANIMCJK/svgsJaKana/$cp.svg" "$KANA_SVG/$cp.svg"
  after=$(ls "$KANA_SVG" 2>/dev/null | wc -l | tr -d ' ')
  [ "$before" != "$after" ] && fetched_kana=$((fetched_kana + 1))
  fetch_quiet "$KANJIVG/kanji/$(printf '%05x' "$cp").svg" "$KVG_SVG/$(printf '%05x' "$cp").svg"
done
echo "  kana SVGs        $(ls "$KANA_SVG" | wc -l | tr -d ' ') in $KANA_SVG ($fetched_kana fetched)"
echo "  KanjiVG SVGs     $(ls "$KVG_SVG" | wc -l | tr -d ' ') in $KVG_SVG (the cross-check, not shipped)"

# ---------------------------------------------------------------------------
# Japanese kanji, for `pnpm run prepare-kanji`.
#
# Four sources, and what each one is and is not for:
#
#   graphicsJa.txt      geometry: the SVG outline of every stroke AND its
#                       centre-line, for all 7,007 characters AnimCJK publishes,
#                       in Make Me a Hanzi's own font space. Arphic PL — the same
#                       licence, and the same already-recorded position, as the
#                       Chinese geometry.
#   dictionaryJa.txt    the kyōiku grade sets, the 214 Kangxi radicals and the IDS
#                       decompositions. LGPL-3.0-or-later. It *also* carries
#                       `on`, `kun` and `definition` fields; `prepare-kanji` does
#                       not read them, and says why: readings and glosses come
#                       from EDRDG, which is the attributed authority for them.
#   kanjidic2-all.json  EDRDG's KANJIDIC2, republished as one JSON document by
#                       `scriptin/jmdict-simplified` (CC BY-SA 4.0). The
#                       authority for the *current* kyōiku grade, the readings
#                       with their okurigana, the stroke counts, the frequency
#                       rank and the English glosses.
#   kvgJa/*.svg         KanjiVG (CC BY-SA 3.0). NOT bundled and NOT an input: the
#                       independent second opinion a kanji stroke count is
#                       checked against, exactly as for the kana.
#
# The KANJIDIC2 snapshot is **pinned**, not `latest`. The artifact it produces is
# committed, so a rebuild has to be reproducible, and moving to a newer EDRDG
# snapshot should be a deliberate act with its own commit — that commit is the
# refresh procedure, and LICENSES.md's EDRDG section is where it is written down.
JMDICT_SIMPLIFIED="3.6.2+20260928191014"
KD2_TGZ="$RAW/kanjidic2-all-$JMDICT_SIMPLIFIED.json.tgz"
KD2_JSON="$RAW/kanjidic2-all.json"

echo "fetching the kanji material into $RAW"
fetch "$ANIMCJK/graphicsJa.txt"   "$RAW/graphicsJa.txt"
fetch "$ANIMCJK/dictionaryJa.txt" "$RAW/dictionaryJa.txt"
# The release URL carries the tag percent-encoded (`+` -> `%2B`); the asset name
# does not. The tarball holds one document whose own file name has no build
# timestamp in it — the dictionary version and the date it was built from are
# *inside* the document, which is where `prepare-kanji` reads them from so that
# the artifact records what it was built from.
fetch "https://github.com/scriptin/jmdict-simplified/releases/download/${JMDICT_SIMPLIFIED//+/%2B}/kanjidic2-all-$JMDICT_SIMPLIFIED.json.tgz" "$KD2_TGZ"
if [ ! -s "$KD2_JSON" ]; then
  echo "  unpack kanjidic2-all.json"
  tar -xzf "$KD2_TGZ" -C "$RAW"
  mv "$RAW/kanjidic2-all-${JMDICT_SIMPLIFIED%%+*}.json" "$KD2_JSON"
fi

# The cross-check, for exactly the characters the artifact will carry: the jōyō
# set, which is KANJIDIC2's grades 1-6 (kyōiku) plus grade 8 (the rest of jōyō).
# Reading the list out of KANJIDIC2 rather than hard-coding 2,136 code points
# keeps the two in step: a newer snapshot that moves a character between grades
# cannot silently leave the oracle one character short.
echo "fetching the KanjiVG cross-check for the jōyō set into $KVG_SVG"
joyo_codepoints() {
  python3 - "$KD2_JSON" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    document = json.load(handle)
for entry in document["characters"]:
    grade = (entry.get("misc") or {}).get("grade")
    if grade is not None and 1 <= grade <= 8:
        print("%05x" % ord(entry["literal"]))
PY
}

JOYO=$(joyo_codepoints)
echo "  jōyō characters  $(printf '%s\n' "$JOYO" | wc -l | tr -d ' ') to check"

# Parallel, because this is 2,136 files where the kana set was 177 and each one
# is a couple of kilobytes. Every fetch writes its own destination and skips what
# is already there, so the concurrency needs no coordination — but the function,
# and the two directory variables it reads, have to be exported for the `bash -c`
# that runs it to see them.
export KANJIVG KVG_SVG
export -f fetch_quiet
printf '%s\n' "$JOYO" | xargs -P 4 -n 1 bash -c 'fetch_quiet "$KANJIVG/kanji/$0.svg" "$KVG_SVG/$0.svg"'
echo "  KanjiVG SVGs     $(ls "$KVG_SVG" | wc -l | tr -d ' ') in $KVG_SVG (the cross-check, not shipped)"

echo "done"
