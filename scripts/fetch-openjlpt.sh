#!/usr/bin/env bash
#
# Download the OpenJLPT grammar dataset the grammar pipeline reads.
#
#   data/raw/openjlpt/   the published JSON, its NOTICE and its LICENSE, as
#                        downloaded. Gitignored: under a megabyte, but it can
#                        always be re-fetched, and nothing that ships needs it —
#                        `prepare-grammar` reads it once and the artifact is
#                        committed.
#
# **This is not part of `fetch-data.sh`**, for the reason that keeps
# `fetch-unidic.sh` and `fetch-tatoeba.sh` separate: nothing that builds or tests
# the app needs it. `data/grammar.bin.gz` is committed, so a clone builds and tests
# without this script, and only a rebuild of the grammar course runs it.
#
# # What is downloaded, and why each file
#
#   data/json/grammar/n5.json … n1.json   the dataset, one file per level, each in
#                                         teaching order. The published files
#                                         rather than `sources/grammar/`, because
#                                         the published ones carry the corpus's
#                                         stable per-point `id` (a slug of the
#                                         romaji) — the key the artifact uses.
#   data/json/meta.json                   the version and the counts the dataset
#                                         claims; `prepare-grammar` checks what it
#                                         read against them and fails if upstream
#                                         moved.
#   LICENSE                               CC BY-SA 4.0, the licence the artifact is
#                                         redistributed under.
#   NOTICE.md                             the attribution the licence requires,
#                                         naming the upstream sources and stating
#                                         that the grammar content is the project's
#                                         own writing.
#
# # The licence question, measured
#
# OpenJLPT's dataset is **CC BY-SA 4.0** and is a derived work: its vocabulary and
# kanji levels come from Jonathan Waller's lists, its dictionary fields from EDRDG
# and its sentences from Tatoeba. **None of that is used here.** This app takes the
# grammar dataset and nothing else, and that is a different provenance: its
# `NOTICE.md` and `sources/grammar/README.md` both state that the explanations and
# every example sentence were **written for OpenJLPT** by its contributors and
# released under CC BY-SA 4.0. That is what makes it usable where the Waller levels
# are not — see `docs/research/JAPANESE_DATA_SOURCE_EVALUATIONS.md` §1 and
# `HANDOVER_NIHONGO.md` invariant 20.
#
# The share-alike obligation is met by the artifact's own source record and by the
# `openjlpt` notice the app ships: both name the corpus, its version, the licence
# and **what was changed** (the level references were dropped from the notes, and
# the examples' furigana markup was parsed into ruby segments).
#
# # The dataset has releases, but the raw files are not pinned by tag
#
# It publishes versioned npm/PyPI packages, and `meta.json` carries the version
# (0.3.0 when this was written). The files are fetched from `main` and the version
# is recorded in the artifact, so a rebuild from a different release *says* it was
# built from a different release. `PROVENANCE.txt` records URL, `Last-Modified`,
# size and SHA-256 for each file, which is what a later reader needs to see what
# actually arrived.
#
# Re-running is safe: an existing download is kept unless `--refresh` is passed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/data/raw/openjlpt"
mkdir -p "$DEST"

REFRESH=0
case "${1:-}" in
  --refresh) REFRESH=1 ;;
  "") ;;
  *) echo "usage: $0 [--refresh]" >&2; exit 2 ;;
esac

BASE="https://raw.githubusercontent.com/evanclan/OpenJLPT/main"

FILES=(
  data/json/grammar/n5.json
  data/json/grammar/n4.json
  data/json/grammar/n3.json
  data/json/grammar/n2.json
  data/json/grammar/n1.json
  data/json/meta.json
  LICENSE
  NOTICE.md
)

PROVENANCE="$DEST/PROVENANCE.txt"
PREVIOUS_DATE=""
if [ -f "$PROVENANCE" ]; then
  PREVIOUS_DATE=$(awk '/^# fetched /{print $3; exit}' "$PROVENANCE")
fi

echo "fetching the OpenJLPT grammar dataset into $DEST"
echo

FETCHED=0
TABLE=""
for path in "${FILES[@]}"; do
  name=$(basename "$path")
  url="$BASE/$path"
  target="$DEST/$name"

  if [ -s "$target" ] && [ "$REFRESH" -eq 0 ]; then
    echo "have   $name ($(wc -c < "$target" | tr -d ' ') bytes)"
  else
    echo "fetch  $name"
    if [ "$REFRESH" -eq 1 ]; then
      rm -f "$target"
    fi
    # `-z` re-downloads only when the copy on the server is newer, so a re-run
    # later picks up a new release without fetching what did not move — and the
    # raw.githubusercontent host answers 200 for `main` either way.
    curl -sS --fail --retry 4 --retry-delay 1 --retry-connrefused \
      -z "$target" -o "$target" "$url"
    FETCHED=1
  fi

  # `raw.githubusercontent.com` answers with no `Last-Modified` at all — measured,
  # not assumed — so the column records what the server did say and the fetch date
  # at the head of this file is the only date it can be held to.
  last_modified=$(curl -sS -I --fail --retry 4 "$url" \
    | tr -d '\r' \
    | awk 'tolower($1) == "last-modified:" { $1=""; sub(/^ /, ""); print; exit }')
  sha=$(shasum -a 256 "$target" | awk '{print $1}')
  TABLE+="$(printf '%s\t%s\t%s\t%s\n' \
    "$url" "${last_modified:--}" "$(wc -c < "$target" | tr -d ' ')" "$sha")"$'\n'
done

if [ "$FETCHED" -eq 1 ] || [ -z "$PREVIOUS_DATE" ]; then
  DATE=$(date -u +%Y-%m-%d)
else
  DATE="$PREVIOUS_DATE"
fi

{
  echo "# What data/raw/openjlpt/ holds, and where it came from."
  echo "# Written by scripts/fetch-openjlpt.sh; regenerated on every run."
  echo "#"
  echo "# fetched $DATE"
  echo "#"
  echo "# The grammar dataset is OpenJLPT's own writing under CC BY-SA 4.0. Its"
  echo "# vocabulary and kanji levels (Waller), dictionary fields (EDRDG) and"
  echo "# sentences (Tatoeba) are NOT used by this project — see the script header."
  echo "#"
  echo "# The host answers no Last-Modified header (measured), so that column is '-'"
  echo "# throughout, and the fetch date above is the only date this file can give."
  echo "#"
  echo "# url<TAB>last-modified<TAB>bytes<TAB>sha256"
  printf '%s' "$TABLE"
} > "$PROVENANCE"

echo
echo "done  $DEST"
echo "      $(du -sh "$DEST" | cut -f1) on disk; provenance in $PROVENANCE"
echo
echo "next: pnpm run prepare-grammar    (writes crates/nihongo-core/data/grammar.bin.gz)"
