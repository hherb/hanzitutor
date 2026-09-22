#!/usr/bin/env bash
#
# Build the distributable bundle: a signed .app and a .dmg.
#
# Signing is what makes the bundle launchable on another Mac without a
# right-click-Open, so it is not left to chance here. The identity is taken
# from, in order:
#
#   1. $APPLE_SIGNING_IDENTITY, if you set it;
#   2. the first "Developer ID Application" certificate in your keychain;
#   3. otherwise ad-hoc ("-"), which still produces a bundle that runs on this
#      machine — Apple Silicon refuses to launch an entirely unsigned one.
#
# The identity is deliberately *not* written into tauri.conf.json: that file is
# committed, and one person's certificate name does not belong in it. Tauri
# reads this environment variable itself (tauri-bundler checks
# APPLE_SIGNING_IDENTITY before the config value).
#
# Notarisation is a separate step and is not attempted here: it needs Apple
# credentials (APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID, or an API key) and it
# uploads the build. See README.md, "Shipping a build".
#
# One `tauri build` does both bundles, in the order tauri.conf.json lists them,
# so the .app is written and signed before the .dmg is attempted. That ordering
# is what makes a disk-image failure survivable: making a .dmg means mounting
# one and asking Finder to lay it out, and a restricted file sandbox or a
# headless session can refuse either — but the .app is already on disk by then.
# Do not be tempted to run the two as separate passes: a `--bundles dmg` pass
# cleans up the .app it wrapped.
#
# Usage: scripts/build-release.sh [extra tauri build arguments]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  DETECTED="$(security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -1)"
  if [ -n "$DETECTED" ]; then
    export APPLE_SIGNING_IDENTITY="$DETECTED"
  else
    export APPLE_SIGNING_IDENTITY="-"
  fi
fi

if [ "$APPLE_SIGNING_IDENTITY" = "-" ]; then
  echo "note: no Developer ID certificate found; the bundle will be ad-hoc signed." >&2
  echo "      It runs here, but not on another Mac without a right-click-Open." >&2
else
  echo "signing as: $APPLE_SIGNING_IDENTITY"
fi

# The CLI is reached through the two wrappers for the reasons HANDOVER.md gives:
# cargo needs this project's own CARGO_HOME, and the npm CLI cannot parse its
# own arguments on a host that runs Node inside another application. A function
# rather than a variable, so a path with a space survives.
tauri() {
  "$ROOT/scripts/with-cargo-env.sh" "$ROOT/scripts/tauri-cli.sh" "$@"
}

# The bundle directory follows CARGO_TARGET_DIR, which this project points at
# .cargo-target/ but a plain checkout leaves at src-tauri/target/.
BUNDLE=""
for dir in "$ROOT/.cargo-target/release/bundle" "$ROOT/src-tauri/target/release/bundle"; do
  if [ -d "$dir" ]; then
    BUNDLE="$dir"
    break
  fi
done

if tauri build "$@"; then
  # Tauri's disk-image script copies the app icon to `.VolumeIcon.icns` on the
  # volume but never marks that file invisible, so the mounted image shows a
  # large dimmed duplicate of the app icon as a stray file. Everything else in
  # the window — `.DS_Store` included — is hidden, which is what makes it read as
  # a rendering fault. Patched after the build because Tauri generates that
  # script itself, and re-signed inside the patch because rewriting an image
  # invalidates the signature Tauri put on it.
  if [ -n "$BUNDLE" ]; then
    for image in "$BUNDLE"/dmg/*.dmg; do
      [ -f "$image" ] && "$ROOT/scripts/hide-dmg-volume-icon.sh" "$image"
    done
  fi

  echo
  echo "built:"
  if [ -n "$BUNDLE" ]; then
    find "$BUNDLE" -maxdepth 2 \( -name '*.app' -o -name '*.dmg' \) -print
  fi
  exit 0
fi

cat >&2 <<EOF

error: the build failed.

If the failure was the disk-image step, everything the reader actually needs is
already there: Tauri writes and signs the .app before it attempts the .dmg, so
the application bundle survives a .dmg failure. Making a disk image means
mounting one and then asking Finder to lay its window out, and a restricted file
sandbox, a headless session or a terminal without Automation permission can
refuse either one.

Look for the app under:
  ${BUNDLE:-<target>/release/bundle}/macos
and if it is there and freshly built, it is complete. To build only the app,
without the disk image:

  ./scripts/with-cargo-env.sh ./scripts/tauri-cli.sh build --bundles app
EOF
exit 1
