#!/usr/bin/env bash
#
# Fetch and build the UniDic dictionary the passage pipeline segments with.
#
#   .lindera/   the built dictionary. Gitignored: it is ~500 MB unpacked, it is
#               regenerable, and nothing that ships needs it. `scripts/
#               with-cargo-env.sh` does not point at it — `prepare-passages` does,
#               through the environment variable below.
#
# **This is a long job and it prints almost nothing while it runs**, which is why
# it is a script of its own rather than something a `cargo build` triggers by
# accident. It downloads ~134 MB and then builds a MeCab dictionary from it,
# which takes a few minutes on an M-series Mac.
#
# What it downloads is `unidic-mecab-2.1.2.tar.gz` from lindera's own mirror. The
# URL and the **MD5** are lindera-unidic's, not this project's — they live in its
# `build.rs` (`FetchParams`), which verifies the hash itself and retries before
# it will build anything. So this script does not re-implement the check; it
# names the pin here so that a change to it is visible in a diff rather than
# buried in a dependency's build script:
#
#   md5 f4502a563e1da44747f61dcd2b269e35  unidic-mecab-2.1.2.tar.gz
#
# UniDic's notice is BSD-3-Clause-shaped (Copyright (c) 2011-2017, The UniDic
# Consortium); LICENSES.md records it, and nothing from it is redistributed: the
# segmentation this produces goes into the artifact, and the dictionary stays in
# .lindera/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CACHE="$ROOT/.lindera"

# Made before anything looks inside it: `find` on a directory that does not exist
# exits non-zero, and under `set -e` with `pipefail` that aborted this script
# before it printed a single line — an empty log and exit 1, which is the least
# debuggable shape a failure can have.
mkdir -p "$CACHE"

# The dictionary directory the build writes, and the only thing
# `prepare-passages` needs to find. Versioned by lindera-unidic's own crate
# version and dictionary format version, so this looks for it rather than
# hard-coding a path that would rot at the next upgrade.
dictionary_dir() {
  local found
  found=$(find "$CACHE" -maxdepth 2 -type d -name lindera-unidic 2>/dev/null | head -1 || true)
  [ -n "$found" ] && printf '%s' "$found"
  return 0
}

echo "checking for a built UniDic dictionary in $CACHE"
existing=$(dictionary_dir)
if [ -n "$existing" ] && [ -s "$existing/metadata.json" ]; then
  echo "have  UniDic dictionary in $existing"
  echo "      ($(du -sh "$existing" 2>/dev/null | cut -f1) built; delete .lindera/ to rebuild)"
  exit 0
fi

echo "fetching and building the UniDic dictionary into $CACHE"
echo "  this downloads about 134 MB (unidic-mecab-2.1.2, MD5-verified by lindera)"
echo "  and then builds a dictionary from it — expect a few minutes and no"
echo "  further output until it is done"
echo

# The variable the whole thing hangs on: lindera's build script returns
# immediately unless this names a directory, which is what keeps the download out
# of every ordinary build.
#
# `--lib` rather than a binary: what has to be built is `lindera-unidic`'s build
# script, and this way the dictionary is fetched and built even before
# `prepare-passages` compiles, or if it does not compile at all.
LINDERA_BUILD_DICTIONARY_CACHE_DIR="$CACHE" \
  "$ROOT/scripts/with-cargo-env.sh" cargo build \
    -p nihongo-core --features nihongo-core/tokenize --lib

built=$(dictionary_dir)
if [ -z "$built" ] || [ ! -s "$built/metadata.json" ]; then
  echo "the build finished but no dictionary is in $CACHE — see the output above" >&2
  exit 1
fi
echo
echo "done  UniDic dictionary in $built ($(du -sh "$built" | cut -f1))"
