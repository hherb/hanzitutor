#!/usr/bin/env bash
#
# Download the graded phrase corpora the pronunciation clips are synthesised from.
#
#   data/phrases/   the two corpora as published, about 5.6 MB. Not committed:
#                   they can always be re-fetched, and only their *derived* audio
#                   is what the app ships.
#
# Like `data/raw/`, this is an input to a build step rather than something the
# app carries. The clips are committed; the text they came from is not.
#
# ## The two corpora, and why they are kept apart
#
#   no7z/hsk-sentences-audio     HSK 1-6 graded sentences, with pinyin, an English
#                                translation and per-word glosses. CC BY-SA 4.0.
#   harukicoder/hsk30-graded-readers
#                                102 reading passages as 1,185 word-aligned
#                                sentences on six difficulty shelves. CC BY 4.0.
#
# `no7z` is share-alike and `harukicoder` is attribution-only, so they are
# downloaded to separate directories, synthesised into separate directories, and
# listed separately in the app. A merged set of files would make it possible to
# ship one under the other's notice by accident.
#
# ## What is used from each
#
# Only the **text**. `no7z` also publishes MP3s of its own, synthesised with
# CosyVoice2-0.5B; this project does not use or redistribute them, and
# regenerates every clip with MeloTTS so that the voice in the app is the voice
# the app can synthesise. See LICENSES.md.
#
# Usage:
#   scripts/fetch-phrases.sh          fetch both corpora
#   scripts/fetch-phrases.sh --check  report which are present and their digests
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/data/phrases"
mkdir -p "$DEST"

# One record per line, fields separated by `|` — not `:`, which the URLs
# themselves contain.
#
# The digests were recorded on 2026-09-21 by downloading each file and hashing
# it. They pin the *revision* as much as the contents: both datasets are served
# from a moving `main` branch, so a re-upload upstream would be caught here
# rather than silently changing what the app's audio says.
SOURCES=(
  "no7z-sentences.jsonl|https://huggingface.co/datasets/no7z/hsk-sentences-audio/resolve/main/data/train.jsonl|e150c3e18293f14cee196bc3a0471d63ca0b18bba7d4bc526cf30b5c3e3ccac7"
  "harukicoder-readers.jsonl|https://huggingface.co/datasets/harukicoder/hsk30-graded-readers/resolve/main/hsk30_graded_readers.jsonl|2ed8f41f6175210933591a8c723435cfb1bf9f7a393d0c3f41fe1be3b3b9fc40"
)

if command -v sha256sum >/dev/null 2>&1; then
  hash_of() { sha256sum "$1" | cut -d' ' -f1; }
else
  hash_of() { shasum -a 256 "$1" | cut -d' ' -f1; }
fi

CHECK_ONLY=0
[ "${1:-}" = "--check" ] && CHECK_ONLY=1

status=0
for row in "${SOURCES[@]}"; do
  IFS="|" read -r name url sha <<<"$row"
  target="$DEST/$name"

  if [ -f "$target" ] && [ "$(hash_of "$target")" = "$sha" ]; then
    if [ "$CHECK_ONLY" = 1 ]; then
      echo "  ok    $name  ($(du -h "$target" | cut -f1))"
    else
      echo "  have  $name  ($(du -h "$target" | cut -f1))"
    fi
    continue
  fi

  if [ "$CHECK_ONLY" = 1 ]; then
    echo "  bad   $name is missing or has changed upstream" >&2
    status=1
    continue
  fi

  echo "  get   $name"
  if ! curl --fail --location --silent --show-error "$url" --output "$target.partial"; then
    rm -f "$target.partial"
    echo "        failed: $url" >&2
    exit 1
  fi
  mv "$target.partial" "$target"

  got="$(hash_of "$target")"
  if [ "$got" != "$sha" ]; then
    rm -f "$target"
    echo "        digest mismatch — deleted; refusing to use it" >&2
    echo "        expected $sha" >&2
    echo "        got      $got" >&2
    echo "        (a re-uploaded dataset is the likely cause; re-record the" >&2
    echo "         digest here only after reading what changed)" >&2
    exit 1
  fi
  echo "        -> $(du -h "$target" | cut -f1)"
done

exit "$status"
