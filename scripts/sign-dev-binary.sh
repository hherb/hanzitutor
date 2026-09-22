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
# It is deliberately *not* a substitute for the release signature: no hardened
# runtime, no timestamp, no notarisation. The dev binary is a local build for
# this machine, and the only thing being asked of it is a stable identity.
#
# Usage:
#   scripts/sign-dev-binary.sh                 # sign the debug binary
#   APPLE_SIGNING_IDENTITY="Developer ID Application: …" scripts/sign-dev-binary.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The dev binary lives in the project's own target directory — see HANDOVER §1
# for why cargo is kept out of `~/.cargo` and the target out of `target/`.
BIN="${HANZI_DEV_BINARY:-$ROOT/.cargo-target/debug/hanzi-tutor}"
# The same identifier the bundle uses, so the keychain sees one program across a
# dev run and an installed build rather than two.
IDENTIFIER="com.hanzitutor.app"

if [ ! -f "$BIN" ]; then
  echo "error: no dev binary at $BIN" >&2
  echo "Build it first: ./scripts/with-cargo-env.sh cargo build -p hanzi-tutor --no-default-features" >&2
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
