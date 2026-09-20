#!/usr/bin/env bash
#
# Fetch the sherpa-onnx native library that `src-tauri` links against, and check
# it against a pinned digest before anything is built with it.
#
#   .sherpa-onnx/<archive stem>/lib/   the unpacked libraries
#   .sherpa-onnx/current               a symlink to the stem above, which is
#                                      what scripts/with-cargo-env.sh points
#                                      SHERPA_ONNX_LIB_DIR at
#
# Both are gitignored. This is a *build input* — around 20 MB of prebuilt
# objects — not source that belongs in a commit.
#
# ## Why this script exists at all
#
# `sherpa-onnx-sys`'s build script downloads a prebuilt native archive from
# GitHub releases during `cargo build`, unless `SHERPA_ONNX_LIB_DIR` names a
# directory that is already here. Left alone, a `cargo build` therefore acquires
# a binary over the network — and this repository's whole posture is the
# opposite: its data arrives through reviewed scripts, its notices are pinned in
# three places at once, and nothing appears in a build that a reviewer did not
# agree to. ROADMAP.md M12 says so directly: *"vendor and pin it, or the build is
# not reproducible."* So the download happens **here**, where it is a deliberate
# act that checks a digest, and the build itself only ever reads what this left
# behind.
#
# ## Why an unpinned platform is refused rather than fetched
#
# A digest that was never checked is not a pin. No release publishes checksums
# alongside these archives, so the only way to pin a platform is to download it
# once on that platform and record what arrived. For a platform that is not in
# the table below this script therefore **stops and says how to add it**, rather
# than quietly pulling whatever the network hands back. On a new platform, do
# that once, by hand, on a machine you trust:
#
#   1. download the archive named in the error message;
#   2. check it however you like, and read what you can of the release notes;
#   3. add a `"<os>-<arch>:<archive>:<sha256>"` row to ARCHIVES below, with a
#      comment saying when and by whom it was recorded;
#   4. re-run this script.
#
# `SHERPA_ALLOW_UNPINNED=1` is the escape hatch for someone who knowingly wants
# the archive without a recorded digest — cross-compiling once to see whether it
# even links, for instance. It prints a warning and does not create the symlink
# the build reads, so an unpinned copy cannot be mistaken for a pinned one by a
# later `cargo build`.
#
# ## Which archive
#
# The **static** libraries, because the alternative puts the packaging problem
# somewhere worse. A shared build would have the build script drop `.dylib`s
# next to the binary and set an `@loader_path` rpath: fine for `cargo run`, but a
# bundled `.app` keeps its executable in `Contents/MacOS/` and would need those
# libraries declared as bundle frameworks, or it would fail to launch only once
# somebody else installed it. Static linking keeps the app one file. See
# LICENSES.md for the notices that travel with it either way.
#
# Usage: scripts/fetch-sherpa.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/.sherpa-onnx"

# The release every archive below is cut from. Both the URL and the archive names
# are derived from it, so a bump is one edit — and the digests have to be
# re-recorded with it, which the refusal path above is there to force.
SHERPA_VERSION="1.13.8"
BASE_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/v${SHERPA_VERSION}"

# `<os>-<arch>` (as `uname` reports them, normalised by host_key below) to the
# archive name and its SHA-256.
#
# Only platforms somebody has actually pinned appear here. Everything else is
# refused — see the note above.
ARCHIVES=(
  # macOS on Apple Silicon, first recorded 2026-09-20 by hand from the v1.13.8
  # release, which is also the version crates.io publishes.
  "macos-arm64:sherpa-onnx-v${SHERPA_VERSION}-osx-arm64-static-lib.tar.bz2:9091bf160dc7fdacedbc906b212badf53c2993f4e5277a0e03998e96c31d60da"
)

# What `uname` says, as the key the table is written in.
host_key() {
  local os arch
  case "$(uname -s)" in
    Darwin) os="macos" ;;
    Linux) os="linux" ;;
    *) os="$(uname -s | tr '[:upper:]' '[:lower:]')" ;;
  esac
  case "$(uname -m)" in
    arm64 | aarch64) arch="arm64" ;;
    x86_64 | amd64) arch="x86_64" ;;
    *) arch="$(uname -m)" ;;
  esac
  echo "${os}-${arch}"
}

# The name the build script would have chosen, so the error message can name the
# exact file to download.
expected_archive() {
  case "$1" in
    macos-arm64) echo "sherpa-onnx-v${SHERPA_VERSION}-osx-arm64-static-lib.tar.bz2" ;;
    macos-x86_64) echo "sherpa-onnx-v${SHERPA_VERSION}-osx-x64-static-lib.tar.bz2" ;;
    linux-x86_64) echo "sherpa-onnx-v${SHERPA_VERSION}-linux-x64-static-lib.tar.bz2" ;;
    linux-arm64) echo "sherpa-onnx-v${SHERPA_VERSION}-linux-aarch64-static-lib.tar.bz2" ;;
    *) echo "(unknown for this platform)" ;;
  esac
}

KEY="$(host_key)"

ARCHIVE=""
DIGEST=""
for row in "${ARCHIVES[@]}"; do
  if [ "${row%%:*}" = "$KEY" ]; then
    rest="${row#*:}"
    ARCHIVE="${rest%%:*}"
    DIGEST="${rest#*:}"
    break
  fi
done

if [ -z "$ARCHIVE" ]; then
  NAME="$(expected_archive "$KEY")"
  echo "error: no pinned sherpa-onnx archive for $KEY." >&2
  echo >&2
  echo "  Would be: $NAME" >&2
  echo "  From:     $BASE_URL" >&2
  echo >&2
  echo "Record its SHA-256 in ARCHIVES in this script (see the header) before" >&2
  echo "building on this platform. To fetch it unpinned instead — for a one-off" >&2
  echo "link check, not for a build anyone will ship — re-run with:" >&2
  echo >&2
  echo "  SHERPA_ALLOW_UNPINNED=1 scripts/fetch-sherpa.sh" >&2
  echo >&2
  if [ "${SHERPA_ALLOW_UNPINNED:-}" != "1" ]; then
    exit 1
  fi
  echo "warning: proceeding unpinned, as asked." >&2
  ARCHIVE="$NAME"
fi

STEM="${ARCHIVE%.tar.bz2}"
LIB_DIR="$DEST/$STEM/lib"

if [ -d "$LIB_DIR" ]; then
  echo "  have  $STEM  ($(du -sh "$LIB_DIR" | cut -f1))"
else
  mkdir -p "$DEST"
  ARCHIVE_PATH="$DEST/$ARCHIVE"

  if [ -s "$ARCHIVE_PATH" ]; then
    echo "  have  $ARCHIVE"
  else
    echo "  get   $ARCHIVE"
    echo "        $BASE_URL/$ARCHIVE"
    # `--fail` so an error page is not saved as if it were the archive.
    curl --fail --location --retry 3 --output "$ARCHIVE_PATH.part" "$BASE_URL/$ARCHIVE"
    mv "$ARCHIVE_PATH.part" "$ARCHIVE_PATH"
  fi

  if [ -n "$DIGEST" ]; then
    ACTUAL="$(shasum -a 256 "$ARCHIVE_PATH" | cut -d' ' -f1)"
    if [ "$ACTUAL" != "$DIGEST" ]; then
      # Removed rather than kept: an archive that failed its digest should not
      # be sitting there to be picked up by a later run's "have" check.
      rm -f "$ARCHIVE_PATH"
      echo "error: $ARCHIVE did not match its recorded digest." >&2
      echo "  expected $DIGEST" >&2
      echo "  got      $ACTUAL" >&2
      echo "The archive was deleted. If the release was re-cut, re-record the" >&2
      echo "digest in ARCHIVES in this script rather than ignoring this." >&2
      exit 1
    fi
    echo "  ok    digest matches"
  else
    echo "  WARN  no digest recorded for $ARCHIVE; nothing was verified" >&2
  fi

  tar xjf "$ARCHIVE_PATH" -C "$DEST"
fi

if [ ! -d "$LIB_DIR" ]; then
  echo "error: the archive unpacked but $LIB_DIR is not there." >&2
  exit 1
fi

# The symlink the build follows, so `with-cargo-env.sh` needs no table of its
# own and cannot disagree with this one about a name. Only written once the
# libraries are really present and, above, only for a pinned archive.
ln -sfn "$STEM" "$DEST/current"

echo
echo "sherpa-onnx native libraries in $LIB_DIR"
echo "SHERPA_ONNX_LIB_DIR will be set to $DEST/current/lib by scripts/with-cargo-env.sh"
