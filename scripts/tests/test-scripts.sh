#!/usr/bin/env bash
#
# Tests for the shell scripts in `scripts/` that can be exercised without the
# platform they are written for: the macOS packaging scripts' bundle lookup (and
# both packaging scripts end to end, with every Apple tool stubbed), the
# silent-exit path of `sign-dev-binary.sh`, `fetch-sherpa.sh`'s first run on a
# fresh checkout and what `with-cargo-env.sh` makes of its leftovers, and the
# digest-checked download `fetch-data.sh` is built on. Every one of these is a
# regression test for issue #38.
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
# macOS's $TMPDIR ends in a slash, so the path above holds a doubled one
# (…/T//hanzi-script-tests.…). The scripts under test name their paths through
# `cd … && pwd`, which folds it, so the expectations here have to be named the
# same way or every path comparison fails on a Mac and nowhere else.
WORK="$(cd "$WORK" && pwd)"

# Said first, because CI runs this twice — once under whatever bash is first on
# PATH and once under macOS's own /bin/bash 3.2 — and the log should say which.
echo "bash $BASH_VERSION"

# The scripts under test read these, and a caller's own values (CI sets a
# relative CARGO_TARGET_DIR; a dev shell may carry a sherpa path or a signing
# identity) must not leak into what is being tested.
unset CARGO_TARGET_DIR SHERPA_ONNX_ARCHIVE_DIR SHERPA_ONNX_LIB_DIR APPLE_SIGNING_IDENTITY

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

# Where the build will write, as an absolute path, whatever form the caller gave.
fake="$WORK/bundle-repo"
mkdir -p "$fake/apps/tone-trainer"
expect_eq "no CARGO_TARGET_DIR: the project's own .cargo-target" "$fake/.cargo-target" \
  "$(bundle_target_dir "$fake")"
expect_eq "an absolute CARGO_TARGET_DIR as given" "$WORK/elsewhere" \
  "$(CARGO_TARGET_DIR="$WORK/elsewhere" bundle_target_dir "$fake")"
expect_eq "a relative one, as CI sets it, against the repository root" "$fake/.cargo-target" \
  "$(CARGO_TARGET_DIR=".cargo-target" bundle_target_dir "$fake")"
expect_eq "a ./relative one too" "$fake/build/target" \
  "$(CARGO_TARGET_DIR="./build/target" bundle_target_dir "$fake")"

expect_eq "a fresh target directory has no bundle directory yet" "" \
  "$(bundle_dir "$fake")"
bundle="$fake/.cargo-target/release/bundle"
mkdir -p "$bundle/macos" "$bundle/dmg"
expect_eq "the bundle directory once the build has made it" "$bundle" \
  "$(bundle_dir "$fake")"
expect_eq "a relative CARGO_TARGET_DIR does not fall back to a stale .cargo-target" "" \
  "$(CARGO_TARGET_DIR="other-target" bundle_dir "$fake")"
mkdir -p "$fake/src-tauri/target/release/bundle"
expect_eq "nor to a stale src-tauri/target" "" \
  "$(CARGO_TARGET_DIR="other-target" bundle_dir "$fake")"

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

# wants_dmg <args...>: "yes" or "no", for one assertion each.
wants_dmg() {
  if bundle_args_want_dmg "$@"; then echo yes; else echo no; fi
}
expect_eq "a plain build makes a disk image" yes "$(wants_dmg)"
expect_eq "other arguments do not change that" yes "$(wants_dmg --config '{"version":"9.9.9"}' --verbose)"
expect_eq "--bundles app does not" no "$(wants_dmg --bundles app)"
expect_eq "-b app, followed by another option, does not" no "$(wants_dmg -b app --verbose)"
expect_eq "-b app dmg does" yes "$(wants_dmg -b app dmg)"
expect_eq "--bundles=app,dmg does" yes "$(wants_dmg --bundles=app,dmg)"
expect_eq "--bundles=app does not" no "$(wants_dmg --bundles=app)"

# ---------------------------------------------------------------------------
# A copy of a packaging script in a repository that has never been built, with
# stand-ins for everything it calls. The stub "build" behaves the way cargo does
# about the target directory — it runs from `src-tauri/`, as the Tauri CLI may,
# and resolves a relative `CARGO_TARGET_DIR` from there — so a script that left
# the variable relative would build in one place and look in another. It writes
# the main app's bundle beside another app's leftovers, with a licence file at a
# restrictive mode; `STUB_DMG_VERSION` names its disk image's version and
# `STUB_NO_DMG=1` makes none.
#
# make_fake_repo <dir> <script...>
make_fake_repo() {
  local repo="$1" script
  shift
  mkdir -p "$repo/scripts/lib" "$repo/src-tauri" "$repo/bin"
  for script in "$@"; do
    cp "$REPO/scripts/$script" "$repo/scripts/"
  done
  cp "$REPO/scripts/lib/bundle.sh" "$repo/scripts/lib/"
  cat >"$repo/src-tauri/tauri.conf.json" <<'EOF'
{
  "productName": "Hanzi Tutor",
  "version": "0.7.0",
  "identifier": "com.hanzitutor.app"
}
EOF
  cat >"$repo/scripts/with-cargo-env.sh" <<'EOF'
#!/usr/bin/env bash
exec "$@"
EOF
  cat >"$repo/scripts/tauri-cli.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo/src-tauri"
target="${CARGO_TARGET_DIR:-$repo/.cargo-target}"
mkdir -p "$target"
bundle="$(cd "$target" && pwd)/release/bundle"
app="$bundle/macos/Hanzi Tutor.app"
mkdir -p "$app/Contents/Resources/licences" "$bundle/dmg" "$bundle/macos/Tone Trainer.app"
echo "notice" >"$app/Contents/Resources/licences/LICENCE.txt"
chmod 600 "$app/Contents/Resources/licences/LICENCE.txt"
: >"$app/Contents/Info.plist"
: >"$bundle/dmg/Tone Trainer_0.7.0_aarch64.dmg"
if [ "${STUB_NO_DMG:-}" != 1 ]; then
  : >"$bundle/dmg/Hanzi Tutor_${STUB_DMG_VERSION:-0.7.0}_aarch64.dmg"
fi
EOF
  cat >"$repo/scripts/hide-dmg-volume-icon.sh" <<'EOF'
#!/usr/bin/env bash
echo "$1" >>"$(dirname "${BASH_SOURCE[0]}")/../hidden.log"
EOF
  chmod +x "$repo/scripts/"*.sh
}

echo "scripts/build-release.sh, with the Tauri CLI stubbed out"

release_stub_security() {
  cat >"$1/bin/security" <<'EOF'
#!/usr/bin/env bash
echo "     0 valid identities found"
EOF
  chmod +x "$1/bin/security"
}

release="$WORK/release-repo"
make_fake_repo "$release" build-release.sh
release_stub_security "$release"
status=0
output="$(PATH="$release/bin:$PATH" "$release/scripts/build-release.sh" 2>&1)" || status=$?
bundle="$release/.cargo-target/release/bundle"
expect_eq "a first build succeeds" 0 "$status"
expect_contains "it lists the app it built" "$bundle/macos/Hanzi Tutor.app" "$output"
expect_contains "and the disk image it built" "$bundle/dmg/Hanzi Tutor_0.7.0_aarch64.dmg" "$output"
expect_not_contains "but not the other app's" "Tone Trainer" "$output"
expect_not_contains "and has nothing to warn about" "note: no Hanzi Tutor disk image" "$output"
expect_eq "the volume icon is hidden on this app's image only" \
  "$bundle/dmg/Hanzi Tutor_0.7.0_aarch64.dmg" "$(cat "$release/hidden.log" 2>/dev/null || true)"
expect_eq "the new bundle's licence texts are made readable" 644 \
  "$(mode_of "$bundle/macos/Hanzi Tutor.app/Contents/Resources/licences/LICENCE.txt")"

# A relative CARGO_TARGET_DIR, beside a stale bundle in the default place.
release="$WORK/release-relative"
make_fake_repo "$release" build-release.sh
release_stub_security "$release"
stale="$release/.cargo-target/release/bundle"
mkdir -p "$stale/macos/Hanzi Tutor.app" "$stale/dmg"
: >"$stale/dmg/Hanzi Tutor_0.7.0_aarch64.dmg"
status=0
output="$(PATH="$release/bin:$PATH" CARGO_TARGET_DIR="build-out" \
  "$release/scripts/build-release.sh" 2>&1)" || status=$?
bundle="$release/build-out/release/bundle"
expect_eq "relative CARGO_TARGET_DIR: the build succeeds" 0 "$status"
expect_contains "it builds into the target directory taken against the repository root" \
  "$bundle/macos/Hanzi Tutor.app" "$output"
expect_not_contains "and does not report the stale bundle" "$stale" "$output"
expect_eq "nor patch the stale image" \
  "$bundle/dmg/Hanzi Tutor_0.7.0_aarch64.dmg" "$(cat "$release/hidden.log" 2>/dev/null || true)"
expect_eq "and nothing was built under src-tauri/ where the CLI ran" no \
  "$([ -d "$release/src-tauri/build-out" ] && echo yes || echo no)"

# The version overridden on the command line: the .app is found, the image is not.
release="$WORK/release-version"
make_fake_repo "$release" build-release.sh
release_stub_security "$release"
status=0
output="$(PATH="$release/bin:$PATH" STUB_DMG_VERSION=9.9.9 \
  "$release/scripts/build-release.sh" --config '{"version":"9.9.9"}' 2>&1)" || status=$?
expect_eq "an overridden version still builds" 0 "$status"
expect_contains "it says that no image of the configured version was found" \
  "note: no Hanzi Tutor disk image for version 0.7.0" "$output"
expect_contains "and names the one that is there" \
  "$release/.cargo-target/release/bundle/dmg/Hanzi Tutor_9.9.9_aarch64.dmg" "$output"
expect_eq "which it did not patch" "" "$(cat "$release/hidden.log" 2>/dev/null || true)"

# `--bundles app` makes no image, and that is not worth a note.
release="$WORK/release-app-only"
make_fake_repo "$release" build-release.sh
release_stub_security "$release"
status=0
output="$(PATH="$release/bin:$PATH" STUB_NO_DMG=1 \
  "$release/scripts/build-release.sh" --bundles app 2>&1)" || status=$?
expect_eq "an app-only build succeeds" 0 "$status"
expect_not_contains "and does not warn about the image it was not asked for" \
  "note: no Hanzi Tutor disk image" "$output"

# ---------------------------------------------------------------------------
echo "scripts/build-appstore.sh, with the Tauri CLI and Apple's tools stubbed out"

# The script's lookup after the build is the part #38 broke: on a fresh target
# directory it failed a build that had succeeded. The stubs answer just enough
# for it to get from the identity lookup to the package; productbuild writes the
# package it is asked for and records what it packaged.
store="$WORK/appstore-repo"
make_fake_repo "$store" build-appstore.sh
: >"$store/src-tauri/embedded.provisionprofile"
echo "<plist/>" >"$store/src-tauri/Entitlements.plist"
mkdir -p "$store/tmp"
cat >"$store/bin/security" <<'EOF'
#!/usr/bin/env bash
case "$*" in
  "find-identity -v -p codesigning") echo '  1) ABC "Apple Distribution: Someone (TEAMID)"' ;;
  "find-identity -v") echo '  2) DEF "3rd Party Mac Developer Installer: Someone (TEAMID)"' ;;
  cms*) echo "<plist>profile</plist>" ;;
esac
EOF
cat >"$store/bin/plutil" <<'EOF'
#!/usr/bin/env bash
# Read stdin when told to, as plutil does, so the `echo |` feeding it is not cut off.
[ "${!#}" = "-" ] && cat >/dev/null
case "$1:${2:-}" in
  -extract:*application-identifier*) echo "TEAMID.com.hanzitutor.app" ;;
  -extract:*team-identifier*) echo "TEAMID" ;;
  -extract:CFBundleShortVersionString) echo "0.7.0" ;;
esac
EOF
cat >"$store/bin/productbuild" <<'EOF'
#!/usr/bin/env bash
echo "$2" >>"$(dirname "${BASH_SOURCE[0]}")/../packaged.log"
: >"${!#}"
EOF
cat >"$store/bin/mktemp" <<'EOF'
#!/usr/bin/env bash
path="$(dirname "${BASH_SOURCE[0]}")/../tmp/entitlements"
: >"$path"
echo "$path"
EOF
for tool in xattr codesign pkgutil; do
  printf '#!/usr/bin/env bash\nexit 0\n' >"$store/bin/$tool"
done
chmod +x "$store/bin/"*
status=0
output="$(PATH="$store/bin:$PATH" APPLE_APPSTORE_SIGNING_IDENTITY="" APPLE_INSTALLER_SIGNING_IDENTITY="" \
  "$store/scripts/build-appstore.sh" 2>&1)" || status=$?
bundle="$store/.cargo-target/release/bundle"
expect_eq "a first App Store build succeeds" 0 "$status"
expect_not_contains "it finds the bundle directory the build made" \
  "could not find the bundle directory" "$output"
expect_contains "and packages this app" "built: $bundle/appstore/Hanzi Tutor_0.7.0.pkg" "$output"
expect_eq "the package holds this app's .app, not the other one" \
  "$bundle/macos/Hanzi Tutor.app" "$(cat "$store/packaged.log" 2>/dev/null || true)"
expect_eq "with the provisioning profile embedded" yes \
  "$([ -f "$bundle/macos/Hanzi Tutor.app/Contents/embedded.provisionprofile" ] && echo yes || echo no)"

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
echo "scripts/probe-app-sandbox.sh"

# The same lookup, the same failure: a copy in a fake root, so the scratch
# directory it makes beside itself is a throwaway one.
probe="$WORK/probe"
mkdir -p "$probe/scripts" "$probe/bin"
cp "$REPO/scripts/probe-app-sandbox.sh" "$probe/scripts/"
printf '#!/usr/bin/env bash\nexit 1\n' >"$probe/bin/security"
chmod +x "$probe/bin/security"
status=0
output="$(PATH="$probe/bin:$PATH" "$probe/scripts/probe-app-sandbox.sh" 2>&1)" || status=$?
expect_eq "no Developer ID identity is a failure" 1 "$status"
expect_contains "and it says so instead of exiting silently" \
  "no Developer ID certificate found" "$output"

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
  expect_eq "$mode: a refused run leaves no empty .sherpa-onnx/ behind" no \
    "$([ -d "$sherpa/.sherpa-onnx" ] && echo yes || echo no)"
done

# ---------------------------------------------------------------------------
echo "scripts/with-cargo-env.sh"

# What it hands an Android build: the archive directory only once the archive is
# in it. A copy of the script in a fake root, running a command that reports the
# variable; `android` among the arguments is what selects the Android branch.
cargo_env="$WORK/cargo-env"
mkdir -p "$cargo_env/scripts"
cp "$REPO/scripts/with-cargo-env.sh" "$cargo_env/scripts/"
cat >"$cargo_env/report" <<'EOF'
#!/usr/bin/env bash
echo "${SHERPA_ONNX_ARCHIVE_DIR:-unset}"
EOF
chmod +x "$cargo_env/report"
archive_dir_for_android() {
  "$cargo_env/scripts/with-cargo-env.sh" "$cargo_env/report" android
}
expect_eq "no .sherpa-onnx/: nothing is exported" unset "$(archive_dir_for_android)"
mkdir -p "$cargo_env/.sherpa-onnx"
expect_eq "an empty .sherpa-onnx/: still nothing" unset "$(archive_dir_for_android)"
: >"$cargo_env/.sherpa-onnx/sherpa-onnx-v1.13.8-android.tar.bz2.part"
expect_eq "a half-downloaded .part: still nothing" unset "$(archive_dir_for_android)"
: >"$cargo_env/.sherpa-onnx/sherpa-onnx-v1.13.8-android.tar.bz2"
expect_eq "the archive itself: the directory is exported" "$cargo_env/.sherpa-onnx" \
  "$(archive_dir_for_android)"

# ---------------------------------------------------------------------------
echo "scripts/lib/fetch.sh"

# shellcheck source-path=SCRIPTDIR source=../lib/fetch.sh
. "$REPO/scripts/lib/fetch.sh"

# The SHA-256 of "hello\n", and of the file every stub download writes.
HELLO_SHA="5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
fetchlib="$WORK/fetch"
mkdir -p "$fetchlib/bin" "$fetchlib/dl"
printf 'hello\n' >"$fetchlib/hello"
# A name GNU sha256sum would escape, and prefix its output line with `\` for.
printf 'hello\n' >"$fetchlib/back\\slash"
expect_eq "hash_of" "$HELLO_SHA" "$(hash_of "$fetchlib/hello")"
expect_eq "hash_of a name with a backslash in it" "$HELLO_SHA" "$(hash_of "$fetchlib/back\\slash")"

# The fallback, on a PATH that has `shasum` and not `sha256sum`.
if command -v shasum >/dev/null 2>&1; then
  mkdir -p "$fetchlib/only-shasum"
  for tool in shasum cut; do
    ln -s "$(command -v "$tool")" "$fetchlib/only-shasum/$tool"
  done
  cat >"$fetchlib/hash-with-path.sh" <<'EOF'
# Sourced afresh, because the choice of tool is made when the file is sourced.
. "$1"
hash_of "$2"
EOF
  expect_eq "hash_of falls back to shasum -a 256" "$HELLO_SHA" \
    "$(env PATH="$fetchlib/only-shasum" "$BASH" "$fetchlib/hash-with-path.sh" \
      "$REPO/scripts/lib/fetch.sh" "$fetchlib/hello")"
else
  echo "  skip  hash_of's shasum fallback: no shasum here"
fi

# A curl that writes "hello\n" wherever it is told to, or fails like a 404 when
# `STUB_FAIL=1`.
cat >"$fetchlib/bin/curl" <<'EOF'
#!/usr/bin/env bash
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output | -o) out="$2"; shift ;;
  esac
  shift
done
if [ "${STUB_FAIL:-}" = 1 ]; then
  echo "curl: (22) The requested URL returned error: 404" >&2
  exit 22
fi
printf 'hello\n' >"$out"
EOF
chmod +x "$fetchlib/bin/curl"

# The stub, from here to the end of the file, which is only this section.
PATH="$fetchlib/bin:$PATH"

# run_fetch <args...>: runs `fetch`, setting $status and $output.
run_fetch() {
  status=0
  output="$(fetch "$@" 2>&1)" || status=$?
}

dest="$fetchlib/dl/good.txt"
run_fetch "https://example.invalid/good.txt" "$dest" "$HELLO_SHA"
expect_eq "a matching download is kept" "0:hello" "$status:$(cat "$dest" 2>/dev/null || true)"
run_fetch "https://example.invalid/good.txt" "$dest" "$HELLO_SHA"
expect_eq "and checked, not fetched, the next time" "0" "$status"
expect_contains "  (it says it already has it)" "have  good.txt" "$output"

dest="$fetchlib/dl/bad.txt"
run_fetch "https://example.invalid/bad.txt" "$dest" "0000"
expect_eq "a download that does not match fails" 1 "$status"
expect_contains "  and says so" "bad.txt did not match its recorded digest; deleted." "$output"
expect_eq "  and is deleted, with no .partial left" "no:no" \
  "$([ -e "$dest" ] && echo yes || echo no):$([ -e "$dest.partial" ] && echo yes || echo no)"

dest="$fetchlib/dl/on-disk.txt"
printf 'changed\n' >"$dest"
run_fetch "https://example.invalid/on-disk.txt" "$dest" "$HELLO_SHA"
expect_eq "a copy already on disk that does not match fails" 1 "$status"
expect_contains "  and says it was left alone" "It was already on disk, so it was left alone." "$output"
expect_eq "  and really is left alone" "changed" "$(cat "$dest")"

run_fetch "https://example.invalid/on-disk.txt" "$dest" "$HELLO_SHA" "Use the copy you saved."
expect_contains "advice replaces the generic message for a copy on disk" "Use the copy you saved." "$output"
expect_not_contains "  (which is not also printed)" "left alone" "$output"
run_fetch "https://example.invalid/new.txt" "$fetchlib/dl/new.txt" "0000" "Use the copy you saved."
expect_contains "and for a fresh download" "Use the copy you saved." "$output"
expect_not_contains "  (which is not also printed)" "Re-record the digest" "$output"

dest="$fetchlib/dl/unpinned.txt"
run_fetch "https://example.invalid/unpinned.txt" "$dest" ""
expect_eq "an empty digest downloads without checking" "0:hello" "$status:$(cat "$dest" 2>/dev/null || true)"
printf 'anything\n' >"$dest"
run_fetch "https://example.invalid/unpinned.txt" "$dest" ""
expect_eq "and accepts whatever is on disk" 0 "$status"

dest="$fetchlib/dl/missing.txt"
STUB_FAIL=1 run_fetch "https://example.invalid/missing.txt" "$dest" "$HELLO_SHA"
expect_eq "a failed download fails" 1 "$status"
expect_eq "  and leaves nothing behind" "no:no" \
  "$([ -e "$dest" ] && echo yes || echo no):$([ -e "$dest.partial" ] && echo yes || echo no)"

# ---------------------------------------------------------------------------
echo
if [ "$failures" -ne 0 ]; then
  echo "$failures failed, $passes passed" >&2
  exit 1
fi
echo "all $passes passed"
