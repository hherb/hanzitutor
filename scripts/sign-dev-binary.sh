#!/usr/bin/env bash
#
# Give the dev binary a stable code signature.
#
# Why this exists: the Dropbox sign-in is kept in the login keychain, and macOS
# grants a keychain item to a *specific* program by its designated requirement.
# A dev build is unsigned, so its requirement is its code hash — which changes on
# every rebuild — and the keychain treats each rebuild as a stranger and asks for
# the login password. Signing with a real identity makes the requirement
# "this certificate signed a binary with this identifier", which survives the
# rebuild, so the prompt happens once and then stops.
#
# **The microphone has the same problem, and the same fix.** `TCC` grants
# microphone access to a program by the same designated requirement, so an
# ad-hoc-signed binary is a stranger on every rebuild: macOS shows the permission
# dialog again, and a build that is never granted one records **silence** rather
# than failing — which reads as "the app does not hear me at all". Signing makes
# the grant stick to the identifier instead of to one build.
#
# That is why `apps/tone-trainer` signs its dev binary too, through the two
# environment variables below. Two apps, two identities, one script.
#
# It is deliberately *not* a substitute for the release signature: no hardened
# runtime, no timestamp, no notarisation. The dev binary is a local build for
# this machine, and the only thing being asked of it is a stable identity.
#
# Usage:
#   scripts/sign-dev-binary.sh                 # sign the main app's debug binary
#   APPLE_SIGNING_IDENTITY="Developer ID Application: …" scripts/sign-dev-binary.sh
#   HANZI_DEV_BINARY=…/tone-trainer HANZI_DEV_IDENTIFIER=com.hanzitutor.tone \
#     scripts/sign-dev-binary.sh               # sign the tone trainer's
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The dev binary lives in the project's own target directory — see HANDOVER §1
# for why cargo is kept out of `~/.cargo` and the target out of `target/`.
BIN="${HANZI_DEV_BINARY:-$ROOT/.cargo-target/debug/hanzi-tutor}"
# The same identifier the bundle uses, so the keychain sees one program across a
# dev run and an installed build rather than two.
IDENTIFIER="${HANZI_DEV_IDENTIFIER:-com.hanzitutor.app}"

if [ ! -f "$BIN" ]; then
  echo "error: no dev binary at $BIN" >&2
  echo "Build it first, e.g. ./scripts/with-cargo-env.sh cargo build -p hanzi-tutor --no-default-features" >&2
  echo "(or set HANZI_DEV_BINARY to the binary you mean)" >&2
  exit 1
fi

if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  # Prefer a local development certificate: it is the one that exists on a
  # machine that can build for a device, and it is not a distribution identity
  # being spent on a debug build.
  APPLE_SIGNING_IDENTITY="$(
    security find-identity -v -p codesigning 2>/dev/null \
      | grep -m1 'Apple Development' \
      | sed 's/.*"\(.*\)"/\1/'
  )"
fi
if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  echo "error: no code-signing identity found; set APPLE_SIGNING_IDENTITY" >&2
  exit 1
fi

codesign --force --sign "$APPLE_SIGNING_IDENTITY" --identifier "$IDENTIFIER" "$BIN"
echo "signed $BIN as $APPLE_SIGNING_IDENTITY"
codesign --display --verbose=2 "$BIN" 2>&1 | grep -E 'Identifier|Authority|TeamIdentifier' | sed 's/^/  /'
