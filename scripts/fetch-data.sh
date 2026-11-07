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
#
# ## Every source is pinned
#
# Each repository is read at a **commit**, not at a branch head, and each single
# file is checked against a **SHA-256** before anything uses it — the same
# arrangement as `fetch-sherpa.sh`, `fetch-tts.sh` and `fetch-phrases.sh`. A
# branch head can move between two runs of this script, and `hanzi.bin.gz` and the
# Japanese artifacts are committed: a re-fetch that silently pulled different
# upstream text would make the next `prepare-*` run quietly produce a different
# artifact. A file that does not match its digest is refused, whether it was just
# downloaded or was already on disk.
#
# The revisions are the ones the committed artifacts were built from: the head of
# each repository on the day its artifact was last regenerated (2026-09-23 for
# `hanzi.bin.gz`, 2026-10-02/03 for the kana and kanji). `hanzi.bin.gz`,
# `kana.bin.gz` and `kanji.bin.gz` were checked to rebuild **byte-identically**
# from them on 2026-11-07, and `words.bin.gz` identically but for the one thing
# no pin can hold, the `JMdict_e.gz` date below. Moving a pin is a deliberate
# commit: change the revision, re-record the digests from a download you have read,
# re-run the `prepare-*` step, and commit the artifact with it.
#
# Two exceptions, each for a reason that is the upstream's rather than this
# script's:
#
#   * The per-character SVGs (AnimCJK's kana, KanjiVG's cross-check) are a few
#     thousand files, so they are pinned by commit alone. A commit fixes their
#     content just as firmly; what it does not add is a second check against the
#     transport, which HTTPS already provides.
#   * `JMdict_e.gz` cannot be pinned at all: EDRDG rebuilds it daily and keeps no
#     archive of earlier days. It is fetched over HTTPS, its digest is printed and
#     recorded in `data/raw/JMdict_e.PROVENANCE.txt`, and `prepare-words` records
#     its own `JMdict created` date in the artifact. `JMDICT_E_SHA256=<digest>`
#     checks a copy against a digest you recorded earlier.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RAW="$ROOT/data/raw"
LICENCES="$ROOT/licences"
FONTS="$ROOT/src/assets/fonts"
mkdir -p "$RAW" "$LICENCES" "$FONTS"

# The upstream revisions, one commit per repository. Recorded 2026-11-07 from each
# repository's history: the commit that was its head when the artifact it feeds
# was last regenerated. Make Me a Hanzi, hanziDB and complete-hsk-vocabulary had
# not moved since well before then (2026-03-08, 2023-10-18 and 2026-03-23), and
# the google/fonts revision is a later one whose two files are byte-identical to
# the committed font and OFL text.
MMAH_REV="bddc96d41bef78427ed0e034e9f7e31d71fd1b92"     # 2026-03-08
HANZIDB_REV="3e9f908d25f70674862343b201f6b641a36cd226"  # 2023-10-18
HSKVOCAB_REV="7ac65bf1a6387d35f1ade478906172a19311c7f9" # 2026-03-23
GOOGLEFONTS_REV="86aced91de05cd4f96a06838124cca45be337b2b"
ANIMCJK_REV="ec5e17cca76c87587790bcbce5ea0b4d4fb753d6"  # 2026-05-13
KANJIVG_REV="70a0b7ae0c18ceb5cb358274b029cce0234a43bc"  # 2026-09-30
# The commit JmdictFurigana's release tag points at, for its licence text.
FURIGANA_REV="8ab94725bedd23f5a566556519a892f78127fffb"

MMAH="https://raw.githubusercontent.com/skishore/makemeahanzi/$MMAH_REV"
HANZIDB="https://raw.githubusercontent.com/ruddfawcett/hanziDB.csv/$HANZIDB_REV"
HSKVOCAB="https://raw.githubusercontent.com/drkameleon/complete-hsk-vocabulary/$HSKVOCAB_REV"
GOOGLEFONTS="https://raw.githubusercontent.com/google/fonts/$GOOGLEFONTS_REV/ofl/notosanssc"
ANIMCJK="https://raw.githubusercontent.com/parsimonhi/animCJK/$ANIMCJK_REV"
KANJIVG="https://raw.githubusercontent.com/KanjiVG/kanjivg/$KANJIVG_REV"

# The pinned Japanese dictionary snapshots, used by both the kanji and the
# vocabulary sections below. Pinned rather than `latest` because the artifacts
# they produce are committed and a rebuild has to be reproducible; moving either
# is a deliberate commit, and LICENSES.md's EDRDG section is the procedure.
#
# `scriptin/jmdict-simplified` republishes EDRDG's dictionaries as JSON documents,
# one release tag covering both. JmdictFurigana is pinned to match the constant of
# the same name in `crates/nihongo-core/src/bin/prepare_words.rs`, because half a
# pin is worse than none: the fetch would take one release and the artifact would
# claim another.
JMDICT_SIMPLIFIED="3.6.2+20260928191014"
JMDICT_FURIGANA="2.3.1+2026-09-25"

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

# macOS ships `shasum`; coreutils gives `sha256sum`. Same digest either way.
if command -v sha256sum >/dev/null 2>&1; then
  hash_of() { sha256sum "$1" | cut -d' ' -f1; }
else
  hash_of() { shasum -a 256 "$1" | cut -d' ' -f1; }
fi

# fetch <url> <destination> <sha256>
#
# Skips a destination that is already there, but checks it either way: a copy
# left by an older, unpinned run of this script is exactly what the digest is
# for. A fresh download that does not match is deleted; one that was already on
# disk is left alone (it may be a committed file) and named, so the person
# running this decides. An empty digest is only for `JMdict_e.gz` — see the
# header.
fetch() {
  local url="$1" dest="$2" want="$3" got
  if [ -s "$dest" ]; then
    echo "  have  $(basename "$dest")  ($(du -h "$dest" | cut -f1))"
  else
    echo "  get   $(basename "$dest")"
    if ! curl --fail --location --silent --show-error --retry 4 --retry-delay 1 --retry-connrefused "$url" --output "$dest.partial"; then
      rm -f "$dest.partial"
      echo "        failed: $url" >&2
      return 1
    fi
    got="$(hash_of "$dest.partial")"
    if [ -n "$want" ] && [ "$got" != "$want" ]; then
      rm -f "$dest.partial"
      echo "error: $(basename "$dest") did not match its recorded digest; deleted." >&2
      echo "  from     $url" >&2
      echo "  expected $want" >&2
      echo "  got      $got" >&2
      echo "Re-record the digest in this script only after reading what changed." >&2
      return 1
    fi
    mv "$dest.partial" "$dest"
    echo "        -> $(du -h "$dest" | cut -f1)"
  fi

  if [ -z "$want" ]; then
    return 0
  fi
  got="$(hash_of "$dest")"
  if [ "$got" != "$want" ]; then
    echo "error: $dest does not match its recorded digest." >&2
    echo "  expected $want" >&2
    echo "  got      $got" >&2
    echo "It was already on disk, so it was left alone. If it came from an earlier," >&2
    echo "unpinned run of this script, delete it and re-run; if it is committed," >&2
    echo "run git diff on it: the committed copy is what the digest was recorded from." >&2
    return 1
  fi
}

echo "fetching datasets into $RAW"
fetch "$MMAH/graphics.txt"   "$RAW/graphics.txt" \
      a28c478b5178e98f67f510b2d52fde08a69dc664654ef43498253b9b764d46ee
fetch "$MMAH/dictionary.txt" "$RAW/dictionary.txt" \
      744bb05d5b0742e9ee35c37791f94d56a173349b3367569e7ca11e510364d203
fetch "$HANZIDB/data/hanziDB.csv" "$RAW/hanziDB.csv" \
      03f809543a2bd0d32c8f65f98567cbaa01527f2eefbdb0ea214a1abddc0b03a0
fetch "$HSKVOCAB/complete.min.json" "$RAW/hsk-words.json" \
      52d8e64ba65a6db4a38ea34302c6de5df53cdb5145254b25edcbf93b80676434

# The notices are committed and bundled, so this only restores one that was
# deleted. They are fetched verbatim rather than transcribed, because a notice
# that has been reworded by hand is no longer the notice the licence requires.
# Each digest is the committed copy's, so a restored notice is the same notice.
# The CC and GNU texts come from their publishers' sites, which have no revision
# to pin to; the digest is what holds them still.
echo "fetching upstream licence texts into $LICENCES"
fetch "$MMAH/COPYING"               "$LICENCES/MakeMeAHanzi-COPYING.txt" \
      26fa6b699b32401de4b75a4af574c2e0a99b3df6cb12df9313d257ee4634359c
fetch "$MMAH/LGPL"                  "$LICENCES/LGPL-3.0.txt" \
      eeb6fd06beac4a94a12ffb66876ecd6af68d5b6f38650d5f11aa90125933098b
fetch "$MMAH/APL/english/ARPHICPL.TXT" "$LICENCES/Arphic-Public-License.txt" \
      3a5e90c0957524a89e48203febcd4492ca4393678abaa7e5b4d70f3ff32b386d
fetch "$HANZIDB/LICENSE"            "$LICENCES/MIT-hanziDB.txt" \
      321fbc7640cdd947a44d17b9810e273a3ba7ac75e4d32ab1aee9adfa58bc122f
fetch "$HSKVOCAB/LICENSE"           "$LICENCES/MIT-complete-hsk-vocabulary.txt" \
      21094287f6e487f6978cf240c67603e4a0e9f341c6e732558d052feb44210c8f
fetch "https://creativecommons.org/licenses/by-sa/4.0/legalcode.txt" \
      "$LICENCES/CC-BY-SA-4.0.txt" \
      28a9529c7d0bb4dc51f4bf5c116a3d16ef247a052f7591466768ddf563fd1cf5
fetch "$GOOGLEFONTS/OFL.txt"        "$LICENCES/OFL-1.1.txt" \
      1c05c68c34f9708415aada51f17e1b0092d2cea709bf4a94cd38114f9e73d7d9
# The GPL-3.0 text, for espeak-ng. It is not a dependency of this project in any
# package manager's sense: it is inside the prebuilt sherpa-onnx library that the
# *mobile* builds link, which is why it is only owed on Android and iOS. See
# LICENSES.md — the macOS build does not contain it, and the notice says so.
fetch "https://www.gnu.org/licenses/gpl-3.0.txt" "$LICENCES/GPL-3.0.txt" \
      3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986

# Noto Sans SC, OFL-1.1. Variable weight, so one ~17 MB file covers every weight
# the interface asks for. The canvas does not use it: it draws the stored vector
# outlines, so the font is only for Chinese rendered as text.
echo "fetching the interface font into $FONTS"
fetch "$GOOGLEFONTS/NotoSansSC%5Bwght%5D.ttf" "$FONTS/NotoSansSC-VF.ttf" \
      a3041811a78c361b1de50f953c805e0244951c21c5bd412f7232ef0d899af0da

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
fetch "$ANIMCJK/graphicsJaKana.txt" "$RAW/graphicsJaKana.txt" \
      50041c8fb477e08d77bcf96716003989b8b1f95a751e4d2628b7339de3b58f38

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
KD2_TGZ="$RAW/kanjidic2-all-$JMDICT_SIMPLIFIED.json.tgz"
KD2_JSON="$RAW/kanjidic2-all.json"

echo "fetching the kanji material into $RAW"
fetch "$ANIMCJK/graphicsJa.txt"   "$RAW/graphicsJa.txt" \
      2bcd1c6d186e5376c5f9202b4eae2eaeff13c2566675a55f1c9dd548a2ef31c8
fetch "$ANIMCJK/dictionaryJa.txt" "$RAW/dictionaryJa.txt" \
      0d80195ac4821d2075bd277934c4eda4c8d98ede047275921e9ece73c660b378
# The release URL carries the tag percent-encoded (`+` -> `%2B`); the asset name
# does not. The tarball holds one document whose own file name has no build
# timestamp in it — the dictionary version and the date it was built from are
# *inside* the document, which is where `prepare-kanji` reads them from so that
# the artifact records what it was built from.
# A release asset can be replaced under the same tag, so the tag is not the whole
# pin; the digest is the rest of it.
fetch "https://github.com/scriptin/jmdict-simplified/releases/download/${JMDICT_SIMPLIFIED//+/%2B}/kanjidic2-all-$JMDICT_SIMPLIFIED.json.tgz" "$KD2_TGZ" \
      0607ff2415f907ae412b387b6965a5cdc7ab1f8f59c233ebf619567ee716af54
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

# ---------------------------------------------------------------------------
# Japanese vocabulary, for `pnpm run prepare-words`.
#
# Three sources, and the first two are the same dictionary twice — which needs
# saying, because it is not redundancy but a hole in the reformatting:
#
#   jmdict-eng.json     EDRDG's JMdict (CC BY-SA 4.0), republished as one JSON
#                       document by `scriptin/jmdict-simplified`. The written
#                       forms, the readings, the English glosses.
#   JMdict_e.gz         EDRDG's own XML, from edrdg.org. Read for **one field**:
#                       the `nf01`-`nf48` priority rank. The JSON above drops
#                       those tags completely — measured, not assumed — so the
#                       ladder's frequency component has no other home.
#   JmdictFurigana.json JmdictFurigana (MIT), which aligns each reading to the
#                       characters it belongs to. Committed into the artifact;
#                       the release is pinned to match FURIGANA_RELEASE in
#                       prepare_words.rs.
#
# JMdict is pinned like KANJIDIC2 and for the same reason. `JMdict_e.gz` is not
# pinned because EDRDG does not version it — it is rebuilt daily — so the artifact
# records that file's own `JMdict created` date instead, and the counts the words
# artifact test pins are what catch it moving. It is fetched over HTTPS from
# `www.edrdg.org`: the same server and the same `/pub/Nihongo/` tree as
# `ftp.edrdg.org` (same file, same ETag, checked 2026-11-07), but the server's
# certificate names only `www.edrdg.org`, so the `ftp.` name could only ever be
# reached over plain HTTP — a download anybody on the path can rewrite.
JD_TGZ="$RAW/jmdict-eng-$JMDICT_SIMPLIFIED.json.tgz"
JD_JSON="$RAW/jmdict-eng.json"

echo "fetching the vocabulary material into $RAW"
fetch "https://github.com/scriptin/jmdict-simplified/releases/download/${JMDICT_SIMPLIFIED//+/%2B}/jmdict-eng-$JMDICT_SIMPLIFIED.json.tgz" "$JD_TGZ" \
      a4851e4dc63de5ac9d1a2af9d0a2b1d3c2c0e8811e72331a7dd3cab4b3e0f786
if [ ! -s "$JD_JSON" ]; then
  echo "  unpack jmdict-eng.json"
  tar -xzf "$JD_TGZ" -C "$RAW"
  mv "$RAW/jmdict-eng-${JMDICT_SIMPLIFIED%%+*}.json" "$JD_JSON"
fi
JMDICT_E_URL="https://www.edrdg.org/pub/Nihongo/JMdict_e.gz"
fetch "$JMDICT_E_URL" "$RAW/JMdict_e.gz" "${JMDICT_E_SHA256:-}"
# What arrived, since nothing upstream will say later. Rewritten on every run, so
# it always describes the copy that is on disk.
JMDICT_E_SHA="$(hash_of "$RAW/JMdict_e.gz")"
# `q` stops at the first match, near the top of the file; gzip then exits on a
# closed pipe, which is why the status is not this assignment's business.
JMDICT_E_CREATED="$(gzip -dc "$RAW/JMdict_e.gz" 2>/dev/null \
  | sed -n 's/.*JMdict created: *\([0-9-]*\).*/\1/p;/JMdict created:/q' || true)"
{
  echo "# The JMdict_e.gz in this directory. Written by scripts/fetch-data.sh;"
  echo "# EDRDG rebuilds the file daily and keeps no archive, so this is the record."
  echo "url      $JMDICT_E_URL"
  echo "sha256   $JMDICT_E_SHA"
  echo "bytes    $(wc -c <"$RAW/JMdict_e.gz" | tr -d ' ')"
  echo "created  ${JMDICT_E_CREATED:-unknown}"
} >"$RAW/JMdict_e.PROVENANCE.txt"
if [ -z "${JMDICT_E_SHA256:-}" ]; then
  echo "        JMdict_e.gz is not pinnable: sha256 $JMDICT_E_SHA, created ${JMDICT_E_CREATED:-unknown}"
fi
# The release URL percent-encodes the `+`, as the jmdict-simplified ones do.
fetch "https://github.com/Doublevil/JmdictFurigana/releases/download/${JMDICT_FURIGANA//+/%2B}/JmdictFurigana.json.tar.gz" "$RAW/JmdictFurigana.json.tar.gz" \
      2327ccc8dfb5181e5aabdb0bc540464caec93cb9ce902d3b6832b841221d392d
if [ ! -s "$RAW/JmdictFurigana.json" ]; then
  echo "  unpack JmdictFurigana.json  (it starts with a UTF-8 BOM — see prepare_words.rs)"
  tar -xzf "$RAW/JmdictFurigana.json.tar.gz" -C "$RAW"
fi

# JmdictFurigana's own licence text, which MIT requires to travel with a
# substantial portion of the work and which the app therefore ships. It belongs to
# the app rather than to the shared `licences/` directory, because that directory's
# own test requires every file in it to be catalogued by the *Chinese* app — the
# mistake HANDOVER_NIHONGO.md invariant 12 records. Fetched from the commit the
# pinned release's tag points at, so the notice and the data cannot come from two
# releases — and a tag that is moved later cannot change which one that is.
echo "fetching the Japanese app's own notices"
fetch "https://raw.githubusercontent.com/Doublevil/JmdictFurigana/$FURIGANA_REV/LICENSE" \
      "$ROOT/apps/nihongo-tutor/src-tauri/licences/MIT-JmdictFurigana.txt" \
      c7d0d41a2df5c30b2d9c45dabc224103fa897021ab444fc0f109feb89b64f190

echo "done"
