#!/usr/bin/env bash
#
# Tests for the shell scripts in `scripts/` that can be exercised without the
# platform they are written for: the macOS packaging scripts' bundle lookup, the
# silent-exit path of `sign-dev-binary.sh`, and `fetch-sherpa.sh`'s first run on a
# fresh checkout. Every one of these is a regression test for issue #38.
#
# Nothing here touches the network, a keychain, or the repository's own build
# output: each test builds a throwaway directory tree, puts stub commands
# (`security`, `curl`, the Tauri CLI wrapper) first on PATH or beside a copy of
# the script under test, and checks what the script did. It runs on Linux and on
# a stock Mac's bash 3.2 alike.
#
# Usage: scripts/tests/test-scripts.sh
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/hanzi-script-tests.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

failures=0
passes=0

pass() {
  passes=$((passes + 1))
  echo "  ok    $1"
}

fail() {
  failures=$((failures + 1))
  echo "  FAIL  $1" >&2
  shift
  for line in "$@"; do
    echo "        $line" >&2
  done
}

# expect_eq <name> <expected> <actual>
expect_eq() {
  if [ "$2" = "$3" ]; then
    pass "$1"
  else
    fail "$1" "expected: $(printf '%q' "$2")" "actual:   $(printf '%q' "$3")"
  fi
}

# expect_contains <name> <needle> <haystack>
expect_contains() {
  case "$3" in
    *"$2"*) pass "$1" ;;
    *) fail "$1" "expected to contain: $2" "actual: $3" ;;
  esac
}

# expect_not_contains <name> <needle> <haystack>
expect_not_contains() {
  case "$3" in
    *"$2"*) fail "$1" "expected not to contain: $2" "actual: $3" ;;
    *) pass "$1" ;;
  esac
}

# A file's permission bits, on either stat.
mode_of() {
  stat -c '%a' "$1" 2>/dev/null || stat -f '%Lp' "$1"
}

# ---------------------------------------------------------------------------
echo "scripts/lib/bundle.sh"

# shellcheck source-path=SCRIPTDIR source=../lib/bundle.sh
. "$REPO/scripts/lib/bundle.sh"

expect_eq "productName of the main app" "Hanzi Tutor" \
  "$(bundle_conf_field "$REPO/src-tauri/tauri.conf.json" productName)"
expect_eq "productName of the Japanese app" "Nihongo Tutor" \
  "$(bundle_conf_field "$REPO/apps/nihongo-tutor/src-tauri/tauri.conf.json" productName)"
expect_eq "productName of the tone trainer" "Tone Trainer" \
  "$(bundle_conf_field "$REPO/apps/tone-trainer/src-tauri/tauri.conf.json" productName)"
expect_eq "a missing config reads as empty, not as a failure" "" \
  "$(bundle_conf_field "$WORK/no-such.json" productName)"

# The lookups below are about this project's default target directory, so one
# set in the caller's environment (CI sets a relative one) must not leak in.
unset CARGO_TARGET_DIR

fake="$WORK/bundle-repo"
mkdir -p "$fake/apps/tone-trainer"
expect_eq "a fresh target directory has no bundle directory yet" "" \
  "$(bundle_dir "$fake" "$fake")"
bundle="$fake/.cargo-target/release/bundle"
mkdir -p "$bundle/macos" "$bundle/dmg"
expect_eq "the bundle directory once the build has made it" "$bundle" \
  "$(bundle_dir "$fake" "$fake")"
expect_eq "a second app in the repository shares it" "$bundle" \
  "$(bundle_dir "$fake" "$fake/apps/tone-trainer")"

# What the shared directory looks like after all three apps have been built,
# across two releases of the main one, with a disk-image scratch file left
# behind by a failed `.dmg` step.
mkdir -p "$bundle/macos/Hanzi Tutor.app" "$bundle/macos/Tone Trainer.app"
: >"$bundle/macos/rw.4242.Hanzi Tutor_0.7.0_aarch64.dmg"
for image in "Hanzi Tutor_0.7.0_aarch64" "Hanzi Tutor_0.6.0_aarch64" \
  "Tone Trainer_0.7.0_aarch64" "Hanzi Tutor Lite_0.7.0_aarch64"; do
  : >"$bundle/dmg/$image.dmg"
done

expect_eq "the .app is the one named after productName" "$bundle/macos/Hanzi Tutor.app" \
  "$(bundle_app "$bundle" "Hanzi Tutor")"
expect_eq "another app's .app is not reported as this one's" "" \
  "$(bundle_app "$bundle" "Nihongo Tutor")"
expect_eq "no bundle directory means no .app" "" "$(bundle_app "" "Hanzi Tutor")"
expect_eq "only this app's disk image of this version" \
  "$bundle/dmg/Hanzi Tutor_0.7.0_aarch64.dmg" \
  "$(bundle_dmgs "$bundle" "Hanzi Tutor" 0.7.0)"
expect_eq "without a version: every release of this app, and nothing else" \
  "$(printf '%s\n' "$bundle/dmg/Hanzi Tutor_0.6.0_aarch64.dmg" "$bundle/dmg/Hanzi Tutor_0.7.0_aarch64.dmg")" \
  "$(bundle_dmgs "$bundle" "Hanzi Tutor" | sort)"
expect_eq "an app with no image yet has none" "" \
  "$(bundle_dmgs "$bundle" "Nihongo Tutor" 0.2.3)"

own="$WORK/own-target"
mkdir -p "$own/src-tauri/target/release/bundle"
expect_eq "falls back to the app's own src-tauri/target" \
  "$own/src-tauri/target/release/bundle" "$(bundle_dir "$WORK/no-repo" "$own")"
mkdir -p "$WORK/elsewhere/release/bundle"
expect_eq "an absolute CARGO_TARGET_DIR is where the build went" \
  "$WORK/elsewhere/release/bundle" "$(CARGO_TARGET_DIR="$WORK/elsewhere" bundle_dir "$WORK/no-repo" "$own")"
expect_eq "a relative CARGO_TARGET_DIR is not guessed at" \
  "$own/src-tauri/target/release/bundle" "$(CARGO_TARGET_DIR=".cargo-target" bundle_dir "$WORK/no-repo" "$own")"

# ---------------------------------------------------------------------------
echo "scripts/build-release.sh, with the Tauri CLI stubbed out"

# A copy of the script with stand-ins for everything it calls, in a repository
# that has never been built: the case where the bundle directory used to be
# looked up before it existed. The stub "build" writes the main app's bundle the
# way Tauri does, into a directory that already holds another app's leftovers.
release="$WORK/release-repo"
mkdir -p "$release/scripts/lib" "$release/src-tauri" "$release/bin"
cp "$REPO/scripts/build-release.sh" "$release/scripts/"
cp "$REPO/scripts/lib/bundle.sh" "$release/scripts/lib/"
cat >"$release/src-tauri/tauri.conf.json" <<'EOF'
{
  "productName": "Hanzi Tutor",
  "version": "0.7.0",
  "identifier": "com.hanzitutor.app"
}
EOF
cat >"$release/scripts/with-cargo-env.sh" <<'EOF'
#!/usr/bin/env bash
exec "$@"
EOF
cat >"$release/scripts/tauri-cli.sh" <<'EOF'
#!/usr/bin/env bash
# Stands in for `tauri build`: writes a bundle into a fresh target directory,
# beside another app's, with a licence file at a restrictive mode.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bundle="$repo/.cargo-target/release/bundle"
mkdir -p "$bundle/macos/Hanzi Tutor.app/Contents/Resources/licences" "$bundle/dmg" \
  "$bundle/macos/Tone Trainer.app"
echo "notice" >"$bundle/macos/Hanzi Tutor.app/Contents/Resources/licences/LICENCE.txt"
chmod 600 "$bundle/macos/Hanzi Tutor.app/Contents/Resources/licences/LICENCE.txt"
: >"$bundle/dmg/Hanzi Tutor_0.7.0_aarch64.dmg"
: >"$bundle/dmg/Tone Trainer_0.7.0_aarch64.dmg"
EOF
cat >"$release/scripts/hide-dmg-volume-icon.sh" <<'EOF'
#!/usr/bin/env bash
echo "$1" >>"$(dirname "${BASH_SOURCE[0]}")/../hidden.log"
EOF
cat >"$release/bin/security" <<'EOF'
#!/usr/bin/env bash
echo "     0 valid identities found"
EOF
chmod +x "$release/scripts/"*.sh "$release/bin/security"

status=0
output="$(PATH="$release/bin:$PATH" APPLE_SIGNING_IDENTITY="" CARGO_TARGET_DIR="" \
  "$release/scripts/build-release.sh" 2>&1)" || status=$?
bundle="$release/.cargo-target/release/bundle"
expect_eq "a first build succeeds" 0 "$status"
expect_contains "it lists the app it built" "$bundle/macos/Hanzi Tutor.app" "$output"
expect_contains "and the disk image it built" "$bundle/dmg/Hanzi Tutor_0.7.0_aarch64.dmg" "$output"
expect_not_contains "but not the other app's" "Tone Trainer" "$output"
expect_eq "the volume icon is hidden on this app's image only" \
  "$bundle/dmg/Hanzi Tutor_0.7.0_aarch64.dmg" "$(cat "$release/hidden.log" 2>/dev/null || true)"
expect_eq "the new bundle's licence texts are made readable" 644 \
  "$(mode_of "$bundle/macos/Hanzi Tutor.app/Contents/Resources/licences/LICENCE.txt")"

# ---------------------------------------------------------------------------
echo "scripts/sign-dev-binary.sh"

sign="$WORK/sign"
mkdir -p "$sign/bin"
: >"$sign/hanzi-tutor"
cat >"$sign/bin/security" <<'EOF'
#!/usr/bin/env bash
echo "  1) 0123456789ABCDEF \"Developer ID Application: Someone (TEAMID)\""
echo "     1 valid identities found"
EOF
chmod +x "$sign/bin/security"

status=0
output="$(PATH="$sign/bin:$PATH" APPLE_SIGNING_IDENTITY="" HANZI_DEV_BINARY="$sign/hanzi-tutor" \
  "$REPO/scripts/sign-dev-binary.sh" 2>&1)" || status=$?
expect_eq "no Apple Development identity is a failure" 1 "$status"
expect_contains "and it says so instead of exiting silently" \
  "no code-signing identity found" "$output"

printf '#!/usr/bin/env bash\nexit 1\n' >"$sign/bin/security"
status=0
output="$(PATH="$sign/bin:$PATH" APPLE_SIGNING_IDENTITY="" HANZI_DEV_BINARY="$sign/hanzi-tutor" \
  "$REPO/scripts/sign-dev-binary.sh" 2>&1)" || status=$?
expect_contains "a failing keychain query says so too" "no code-signing identity found" "$output"

# ---------------------------------------------------------------------------
echo "scripts/fetch-sherpa.sh on a fresh checkout"

# A copy of the script in a tree with no `.sherpa-onnx/`, and a `curl` that, like
# the real one, cannot write into a directory that does not exist. What it writes
# is not the archive, so the run ends at the digest check — which it can only
# reach if the download had somewhere to go.
for mode in --android --ios; do
  sherpa="$WORK/sherpa$mode"
  mkdir -p "$sherpa/scripts" "$sherpa/bin"
  cp "$REPO/scripts/fetch-sherpa.sh" "$sherpa/scripts/"
  cat >"$sherpa/bin/curl" <<'EOF'
#!/usr/bin/env bash
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output | -o) out="$2"; shift ;;
  esac
  shift
done
[ -d "$(dirname "$out")" ] || { echo "curl: (23) Failure writing output to destination" >&2; exit 23; }
echo "not the archive" >"$out"
EOF
  chmod +x "$sherpa/bin/curl"
  status=0
  output="$(PATH="$sherpa/bin:$PATH" "$sherpa/scripts/fetch-sherpa.sh" "$mode" 2>&1)" || status=$?
  expect_eq "$mode: stops at the digest check, not at the download" 1 "$status"
  expect_contains "$mode: the download landed and was checked" \
    "did not match its recorded digest" "$output"
  expect_not_contains "$mode: curl had a directory to write into" "Failure writing output" "$output"
done

# ---------------------------------------------------------------------------
echo
if [ "$failures" -ne 0 ]; then
  echo "$failures failed, $passes passed" >&2
  exit 1
fi
echo "all $passes passed"
