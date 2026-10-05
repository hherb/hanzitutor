#!/usr/bin/env bash
#
# Download the Tatoeba exports the phrase pipeline reads.
#
#   data/raw/tatoeba/   the exports, as downloaded and decompressed. Gitignored:
#                       they are ~30 MB compressed and ~130 MB unpacked, they can
#                       always be re-fetched, and nothing that ships needs them.
#                       `prepare-phrases` reads the `.tsv` files and records what
#                       it read; the artifact is committed.
#
# **This is not part of `fetch-data.sh`**, and the reason is the same one that
# keeps `fetch-unidic.sh` separate: nothing that builds or tests the app needs it.
# The phrases artifact is committed, so a clone builds and tests without this
# script; only a rebuild of the corpus runs it. Unlike UniDic, this one is a short
# job — about 30 MB and well under a minute — but it is still a corpus with a
# licence attached, and it deserves a script that says what it is.
#
# # What is downloaded, and why each file
#
#   jpn_sentences_detailed.tsv.bz2   the Japanese sentences: id, lang, text,
#                                    contributor, date added, date modified. There
#                                    is **no licence column here** — see below.
#   jpn-eng_links.tsv.bz2            which Japanese sentence is a translation of
#                                    which English one: `jpn_id<TAB>eng_id`.
#   eng_sentences.tsv.bz2            the English sentences, so a phrase can carry
#                                    the translation the corpus pairs with it.
#   jpn_sentences_CC0.tsv.bz2        the Japanese sentences contributed under
#                                    **CC0** rather than the corpus default. It is
#                                    228 bytes and holds two sentences; it is
#                                    fetched so that the pipeline can *say* they
#                                    are CC0 rather than assume they are not.
#   jpn_tags.tsv.bz2                 the community's tags on the Japanese
#                                    sentences: `id<TAB>tag`. Two of the tags are
#                                    used as a hard exclusion — `not a sentence`
#                                    and `@possible copyright infringement` — and
#                                    the rest are left alone.
#
# # The licence question, measured
#
# Tatoeba's own downloads page states: "These files are released under CC BY 2.0
# FR. A part of our sentences are also available under CC0 1.0." The per-language
# exports carry **no per-sentence licence column** — only the 303 MiB all-languages
# `sentences_detailed.tar.bz2` does, and it holds the same Japanese subset for
# about 10x the size — so the corpus's own statement is the basis, and
# `jpn_sentences_CC0.tsv.bz2` is what separates the CC0 part from it.
#
# The set of licences a Tatoeba *sentence* can carry was then checked against the
# corpus's own API, whose validation enumerates it: `CC BY 2.0 FR`, `CC0 1.0` and
# `PROBLEM` — and it counts 249,077 CC BY, 2 CC0 and 0 PROBLEM for Japanese. There
# is **no NoDerivatives and no NonCommercial variant of a sentence text at all**;
# NonCommercial occurs on *audio* only, which this project does not bundle. Both
# sentence licences permit redistribution and adaptation. LICENSES.md records the
# measurement, and ROADMAP_NIHONGO.md's milestone carries the attribution
# consequence: 42.7% of the Japanese sentences name no contributor, and CC BY
# requires the author to be named, so `prepare-phrases` does not ship those.
#
# The exports are regenerated weekly and are **not** pinned by hash, unlike the
# Japanese dictionaries. Instead the script records what it actually fetched —
# URL, `Last-Modified`, size and SHA-256 of each archive — into
# `data/raw/tatoeba/PROVENANCE.txt`, and `prepare-phrases` writes that export date
# into the artifact it builds. A rebuild from a different week therefore *says* it
# was built from a different week, which is the honest arrangement for a corpus
# that has no immutable release.
#
# Re-running is safe: an existing download is kept unless `--refresh` is passed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/data/raw/tatoeba"
mkdir -p "$DEST"

REFRESH=0
case "${1:-}" in
  --refresh) REFRESH=1 ;;
  "") ;;
  *) echo "usage: $0 [--refresh]" >&2; exit 2 ;;
esac

BASE="https://downloads.tatoeba.org/exports/per_language"

FILES=(
  jpn_sentences_detailed.tsv.bz2
  jpn-eng_links.tsv.bz2
  eng_sentences.tsv.bz2
  jpn_sentences_CC0.tsv.bz2
  jpn_tags.tsv.bz2
)

# The per-language directory differs per language, and two of the four files are
# English, so the URL cannot be derived from the name.
url_for() {
  case "$1" in
    eng_*) printf '%s/eng/%s' "$BASE" "$1" ;;
    *) printf '%s/jpn/%s' "$BASE" "$1" ;;
  esac
}

PROVENANCE="$DEST/PROVENANCE.txt"
{
  echo "# What data/raw/tatoeba/ holds, and where it came from."
  echo "# Written by scripts/fetch-tatoeba.sh; regenerated on every run."
  echo "#"
  echo "# url<TAB>last-modified<TAB>archive-bytes<TAB>sha256<TAB>text-bytes"
} > "$PROVENANCE"

echo "fetching the Tatoeba Japanese exports into $DEST"
echo

for name in "${FILES[@]}"; do
  url=$(url_for "$name")
  archive="$DEST/$name"
  text="${archive%.bz2}"

  if [ -s "$archive" ] && [ -s "$text" ] && [ "$REFRESH" -eq 0 ]; then
    echo "have  $name ($(du -h "$archive" | cut -f1) archive, $(du -h "$text" | cut -f1) text)"
  else
    echo "fetch  $name"
    if [ "$REFRESH" -eq 1 ]; then
      rm -f "$archive" "$text"
    fi
    # `-z` re-downloads only when the copy on the server is newer, so a re-run a
    # week later picks up the new export without fetching what did not move.
    curl -sS --fail --retry 3 --retry-connrefused -z "$archive" -o "$archive" "$url"
    # Decompressed here rather than in the pipeline: a bzip2 reader would be
    # another dependency in `nihongo-core` for a step only this script needs, and
    # the pipeline then reads plain TSV it can stream.
    bzip2 -dc "$archive" > "$text"
    echo "      $(du -h "$archive" | cut -f1) archive -> $(du -h "$text" | cut -f1) text"
  fi

  last_modified=$(curl -sS -I --fail --retry 3 "$url" \
    | tr -d '\r' \
    | awk 'tolower($1) == "last-modified:" { $1=""; sub(/^ /, ""); print; exit }')
  sha=$(shasum -a 256 "$archive" | awk '{print $1}')
  printf '%s\t%s\t%s\t%s\t%s\n' \
    "$url" "${last_modified:-unknown}" "$(wc -c < "$archive" | tr -d ' ')" "$sha" \
    "$(wc -c < "$text" | tr -d ' ')" >> "$PROVENANCE"
done

echo
echo "done  $DEST"
echo "      $(du -sh "$DEST" | cut -f1) on disk; provenance in $PROVENANCE"
echo
echo "about the licence: the corpus says its exports are CC BY 2.0 FR, with a part"
echo "under CC0 1.0; there is no per-sentence licence column in these files. See the"
echo "header of this script and LICENSES.md."
