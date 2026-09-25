#!/usr/bin/env bash
#
# Run a Tauri app in dev, with the **signed** binary actually running.
#
# ## Why the documented `dev:signed` one-liner cannot work
#
# Both apps sign their debug binary so that macOS keeps granting it the same
# permissions across rebuilds — the Keychain for sync, the **microphone** for tone
# practice. `TCC` and the Keychain both key a grant to a program's *designated
# requirement*, and an ad-hoc signature's requirement is its code hash, which
# changes on every rebuild. That is why an unsigned dev build asks for the
# microphone again after every `cargo build`, and why a build that is never granted
# it records **silence** rather than failing: the app says "I could not hear enough
# voice to judge" and nothing anywhere reports a permission problem.
#
# The obvious fix — `cargo build && sign && tauri dev` — does not work, and the way
# it fails is worth knowing:
#
#     cargo run --no-default-features      # what `tauri dev` does to launch
#     Finished `dev` profile in 0.17s      # nothing to compile
#     codesign -dv target/debug/app        # Identifier=<app>-<build-hash>, adhoc
#
# **`cargo run` re-links the binary even when it compiles nothing**, and re-linking
# replaces the code signature with the linker's ad-hoc one. So the signature is
# always stripped a fraction of a second after it is applied, and the permission
# prompt returns on every run. Measured, not inferred: the signature is present
# after `sign-dev-binary.sh` and gone after the `cargo run` that follows it.
#
# ## What this does instead
#
# Start Vite, build and sign the binary, then **exec it directly** — no cargo in
# between, so nothing can re-link it. The app still talks to the Vite server, so
# frontend edits hot-reload exactly as they do under `tauri dev`; what is lost is
# Rust hot-restart, which is the rebuild that would invalidate the signature
# anyway. Run the script again after a Rust change.
#
# Usage:
#   scripts/dev-signed.sh --root apps/tone-trainer --package tone-trainer --identifier com.hanzitutor.tone
#   scripts/dev-signed.sh                      # the main app, with its own defaults
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

APP_ROOT="$ROOT"
PACKAGE="hanzi-tutor"
IDENTIFIER="com.hanzitutor.app"
BINARY_NAME=""
PORT=""

while [ $# -gt 0 ]; do
  case "$1" in
    --root) APP_ROOT="$2"; shift 2 ;;
    --package) PACKAGE="$2"; shift 2 ;;
    --identifier) IDENTIFIER="$2"; shift 2 ;;
    --binary) BINARY_NAME="$2"; shift 2 ;;
    --port) PORT="$2"; shift 2 ;;
    -h | --help)
      sed -n '2,40p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "error: unknown argument $1" >&2
      exit 2
      ;;
  esac
done

APP_ROOT="$(cd "$APP_ROOT" && pwd)"
# The CLI finds `src-tauri/tauri.conf.json` relative to the directory it runs in,
# which is also where node resolves `@tauri-apps/cli` from — see tauri-cli.sh.
BINARY_NAME="${BINARY_NAME:-$PACKAGE}"
CONF="$APP_ROOT/src-tauri/tauri.conf.json"

[ -f "$CONF" ] || {
  echo "error: no tauri.conf.json under $APP_ROOT/src-tauri" >&2
  exit 2
}

# Read the dev port and the window label out of the app's own config, so this
# script cannot drift from it.
if [ -z "$PORT" ]; then
  PORT="$(sed -n 's/.*"devUrl"[[:space:]]*:[[:space:]]*"[^"]*:\([0-9]\{2,\}\)".*/\1/p' "$CONF" | head -1)"
fi
[ -n "$PORT" ] || {
  echo "error: no devUrl port in $CONF" >&2
  exit 2
}

# The vite binary comes from the app's own node_modules, the way `pnpm run
# vite:dev` would resolve it.
VITE="$APP_ROOT/node_modules/.bin/vite"
[ -x "$VITE" ] || {
  echo "error: no vite at $VITE — run pnpm install in $APP_ROOT" >&2
  exit 2
}

cleanup() {
  if [ -n "${VITE_PID:-}" ] && kill -0 "$VITE_PID" 2>/dev/null; then
    kill "$VITE_PID" 2>/dev/null || true
    wait "$VITE_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

echo "==> starting vite on port $PORT"
(cd "$APP_ROOT" && exec "$VITE" --port "$PORT" --strictPort) &
VITE_PID=$!

# Wait for the server to answer rather than sleeping a guessed interval: the app
# opens the dev URL immediately, and a webview pointed at a socket that is not
# listening yet shows a blank window.
for _ in $(seq 1 100); do
  if curl -fsS -o /dev/null "http://localhost:$PORT/" 2>/dev/null; then
    break
  fi
  sleep 0.1
done
curl -fsS -o /dev/null "http://localhost:$PORT/" 2>/dev/null || {
  echo "error: vite did not come up on port $PORT" >&2
  exit 1
}
echo "==> vite is up"

echo "==> building $PACKAGE"
"$ROOT/scripts/with-cargo-env.sh" cargo build -p "$PACKAGE" --no-default-features

BIN="${CARGO_TARGET_DIR:-$ROOT/.cargo-target}/debug/$BINARY_NAME"
[ -f "$BIN" ] || {
  echo "error: no binary at $BIN" >&2
  exit 1
}

echo "==> signing"
HANZI_DEV_BINARY="$BIN" HANZI_DEV_IDENTIFIER="$IDENTIFIER" \
  "$ROOT/scripts/sign-dev-binary.sh"

# The whole point: no cargo from here on, so nothing re-links the binary and the
# signature survives. `exec` so the app owns the terminal and Ctrl-C reaches it.
echo "==> running $BIN (signed; run this script again after a Rust change)"
cd "$APP_ROOT"
exec "$BIN"
