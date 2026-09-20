#!/usr/bin/env bash
#
# Fetch the sherpa-onnx native libraries that `src-tauri` links against, and
# check them against a pinned digest before anything is built with them.
#
#   .sherpa-onnx/<archive stem>/lib/   the unpacked libraries
#   .sherpa-onnx/current               a symlink to the stem above, which is
#                                      what scripts/with-cargo-env.sh points
#                                      SHERPA_ONNX_LIB_DIR at for a host build
#   .sherpa-onnx/current-ios           the same, for an iOS build
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
# ## iOS is a different artefact, from a different release tag
#
# iOS cannot use the above. `sherpa-onnx-sys` forces **shared** linking there
# (`resolve_link_mode`), so a static archive is not merely wrong, it panics the
# build script with "No shared runtime libraries found" — which is what happened
# the first time an iOS build was attempted with `SHERPA_ONNX_LIB_DIR` pointing
# at the macOS libraries. iOS also needs a different file: the release carries no
# iOS asset at all under the `v<version>` tag, and the xcframework lives under a
# separate long-lived `xcframework` tag instead. So `--ios` fetches that, and
# pins it the same way.
#
# `--ios` additionally stages the xcframework at `src-tauri/gen/apple/`, because
# the crate cannot do it here: `sherpa-onnx-sys` copies the xcframework into the
# Tauri project by looking for `tauri.conf.json` in `target_dir.parent()`, which
# assumes the default `src-tauri/target/`. This project sets `CARGO_TARGET_DIR`
# to `<repo>/.cargo-target`, so that parent is the repository root, the lookup
# finds nothing and the copy is silently skipped. The stage is therefore done
# here, deliberately, next to the digest check — and `gen/apple/project.yml`
# names the result, so the two have to agree.
#
# Android is in the same position for the same reason and is handled the same
# way: shared linking, so the host's static libraries are fatal, and a jniLibs
# copy the crate cannot make. The difference is what is pinned — one archive
# holding a `.so` per ABI, from the ordinary versioned tag — and that Android
# gets `SHERPA_ONNX_ARCHIVE_DIR` rather than a lib directory, so the crate picks
# the ABI matching the architecture being built. Staging happens here for all
# ABIs at once, because an APK may carry several.
#
# Usage:
#   scripts/fetch-sherpa.sh            the host's libraries (macOS on Apple Silicon)
#   scripts/fetch-sherpa.sh --ios      the iOS xcframework, for a device build
#   scripts/fetch-sherpa.sh --android  the Android `.so` files, for an APK build
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/.sherpa-onnx"

# The release every archive below is cut from. Both the URL and the archive names
# are derived from it, so a bump is one edit — and the digests have to be
# re-recorded with it, which the refusal path above is there to force.
SHERPA_VERSION="1.13.8"
BASE_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/v${SHERPA_VERSION}"

# iOS artefacts are not versioned into a `v<version>` tag; they live on one
# rolling tag that is re-cut per release, so the file name carries the version
# and the URL does not.
XCFRAMEWORK_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/xcframework"

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

# The one iOS archive: an xcframework holding the device (arm64) slice and the
# simulator slices, with onnxruntime linked into the dylib statically so there is
# no second thing to embed.
#
# First recorded 2026-09-20 during the phone spike, by downloading the asset and
# hashing it — the same procedure the refusal path above describes. Everything
# the app can do to the phone's build hangs off this digest being right.
IOS_ARCHIVE="sherpa-onnx-v${SHERPA_VERSION}-ios-shared-onnxruntime-static.xcframework.zip"
IOS_SHA256="e259a7d3b38ad7dec49bb078252a30bb42ede8355e2bb130cf8c1c78ed131f75"
# Where XcodeGen is told to look for it, relative to `src-tauri/gen/apple`. The
# upstream name, so the framework inside the bundle is recognisable as upstream's.
IOS_STAGE="$ROOT/src-tauri/gen/apple/SherpaOnnxC.xcframework"

# Android takes the same shape as iOS — a shared-linking target that cannot use
# the host's static libraries — but a different artefact: one archive holding a
# `.so` per ABI under `jniLibs/`, from the ordinary versioned tag rather than the
# rolling `xcframework` one.
#
# The archive is left where the crate can reach it, and
# `SHERPA_ONNX_ARCHIVE_DIR` is what `with-cargo-env.sh` sets for Android: the
# crate then unpacks it and picks the right ABI itself, which a single
# `SHERPA_ONNX_LIB_DIR` could not do for a build that covers several.
ANDROID_ARCHIVE="sherpa-onnx-v${SHERPA_VERSION}-android.tar.bz2"
ANDROID_SHA256="2ff63469a71cb6009aa2e3ed5f4a670f8abdcbe4bb9ffd23776afc792a6b4f44"
# Where Gradle picks native libraries up from. The crate would stage these
# itself, but cannot find this project's `gen/android` for the same reason it
# cannot find `gen/apple` (see the header).
ANDROID_STAGE="$ROOT/src-tauri/gen/android/app/src/main/jniLibs"

MODE="host"
case "${1:-}" in
  --ios) MODE="ios" ;;
  --android) MODE="android" ;;
  "") ;;
  *)
    echo "error: unknown argument '$1'." >&2
    echo "usage: scripts/fetch-sherpa.sh [--ios|--android]" >&2
    exit 1
    ;;
esac

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

# Download `$2` from `$1` into $DEST unless it is already there, then check it
# against `$3`. An archive that fails its digest is deleted rather than kept, so
# a later run's "have" check cannot pick it up.
fetch_checked() {
  local url="$1" archive="$2" digest="$3"
  local path="$DEST/$archive"

  if [ -s "$path" ]; then
    echo "  have  $archive"
  else
    echo "  get   $archive"
    echo "        $url/$archive"
    # `--fail` so an error page is not saved as if it were the archive.
    curl --fail --location --retry 3 --output "$path.part" "$url/$archive"
    mv "$path.part" "$path"
  fi

  if [ -n "$digest" ]; then
    local actual
    actual="$(shasum -a 256 "$path" | cut -d' ' -f1)"
    if [ "$actual" != "$digest" ]; then
      rm -f "$path"
      echo "error: $archive did not match its recorded digest." >&2
      echo "  expected $digest" >&2
      echo "  got      $actual" >&2
      echo "The archive was deleted. If the release was re-cut, re-record the" >&2
      echo "digest in this script rather than ignoring this." >&2
      exit 1
    fi
    echo "  ok    digest matches"
  else
    echo "  WARN  no digest recorded for $archive; nothing was verified" >&2
  fi
}

if [ "$MODE" = "android" ]; then
  fetch_checked "$BASE_URL" "$ANDROID_ARCHIVE" "$ANDROID_SHA256"

  STEM="${ANDROID_ARCHIVE%.tar.bz2}"
  DIR="$DEST/$STEM"

  if [ ! -d "$DIR/jniLibs" ]; then
    mkdir -p "$DIR"
    tar xjf "$DEST/$ANDROID_ARCHIVE" -C "$DIR"
  fi

  if [ ! -d "$DIR/jniLibs" ]; then
    echo "error: the archive unpacked but $DIR/jniLibs is not there." >&2
    exit 1
  fi

  # Copied in beside whatever is already there rather than over the directory:
  # Gradle's own build drops a symlink to the app's `libhanzi_tutor_lib.so` in
  # these same per-ABI directories, and replacing one would take the app's own
  # library with it.
  staged=0
  for abi_dir in "$DIR"/jniLibs/*/; do
    [ -d "$abi_dir" ] || continue
    abi="$(basename "$abi_dir")"
    mkdir -p "$ANDROID_STAGE/$abi"
    for so in "$abi_dir"*.so; do
      [ -f "$so" ] || continue
      cp -f "$so" "$ANDROID_STAGE/$abi/"
      staged=$((staged + 1))
    done
  done

  if [ "$staged" -eq 0 ]; then
    echo "error: no .so files found under $DIR/jniLibs." >&2
    exit 1
  fi

  echo
  echo "sherpa-onnx Android libraries in $DIR/jniLibs"
  echo "staged $staged .so file(s) into $ANDROID_STAGE"
  echo "SHERPA_ONNX_ARCHIVE_DIR will be set to $DEST by scripts/with-cargo-env.sh"
  echo
  echo "The staging is what the APK packages. Without it the build still links and"
  echo "the app then fails to start, because the libraries are not in the bundle."
  exit 0
fi

if [ "$MODE" = "ios" ]; then
  fetch_checked "$XCFRAMEWORK_URL" "$IOS_ARCHIVE" "$IOS_SHA256"

  STEM="${IOS_ARCHIVE%.zip}"
  DIR="$DEST/$STEM"
  XCFRAMEWORK="$DIR/SherpaOnnxC.xcframework"

  if [ ! -d "$XCFRAMEWORK" ]; then
    mkdir -p "$DEST"
    unzip -q -o "$DEST/$IOS_ARCHIVE" -d "$DIR"
  fi

  BINARY="$XCFRAMEWORK/ios-arm64/SherpaOnnxC.framework/SherpaOnnxC"
  if [ ! -f "$BINARY" ]; then
    echo "error: $XCFRAMEWORK has no device slice at ios-arm64/." >&2
    exit 1
  fi

  # The crate's build script insists on finding a `.dylib` in the directory it is
  # pointed at — that is the check iOS fails when handed the macOS static
  # libraries. The xcframework holds the same file under the framework's name, so
  # the lib directory is a symlink to it, which is exactly what the crate's own
  # iOS setup does when it downloads the archive itself.
  LIB_DIR="$DIR/lib"
  mkdir -p "$LIB_DIR"
  ln -sfn "../SherpaOnnxC.xcframework/ios-arm64/SherpaOnnxC.framework/SherpaOnnxC" \
    "$LIB_DIR/libsherpa-onnx-c-api.dylib"

  # Only written once the libraries are really present and verified above.
  ln -sfn "$STEM" "$DEST/current-ios"

  # Staged for Xcode, because the crate's own copy cannot find this project's
  # `gen/apple` (see the header). Always refreshed, so a version bump cannot
  # leave a stale framework behind for the linker to find.
  rm -rf "$IOS_STAGE"
  cp -R "$XCFRAMEWORK" "$IOS_STAGE"

  echo
  echo "sherpa-onnx iOS xcframework in $XCFRAMEWORK"
  echo "SHERPA_ONNX_LIB_DIR will be set to $DEST/current-ios/lib by scripts/with-cargo-env.sh"
  echo "staged for Xcode at $IOS_STAGE"
  echo
  echo "That staging is what src-tauri/gen/apple/project.yml names; if the two"
  echo "disagree, the app fails to link rather than to build."
  exit 0
fi

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
  fetch_checked "$BASE_URL" "$ARCHIVE" "$DIGEST"
  tar xjf "$DEST/$ARCHIVE" -C "$DEST"
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
