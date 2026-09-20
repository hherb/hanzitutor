#!/usr/bin/env bash
#
# Run a command with this project's own CARGO_HOME and target directory.
#
# Why: cargo normally writes its registry cache and build output under
# ~/.cargo and <project>/target. Under a restricted (workspace-write) file
# sandbox, writes outside the project are refused, so the project keeps its own
# cargo home in-tree. It is seeded from your global cache by
# scripts/seed-cargo-home.sh, and both directories are gitignored.
#
# Usage: scripts/with-cargo-env.sh cargo test
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export CARGO_HOME="${CARGO_HOME:-$ROOT/.cargo-home}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/.cargo-target}"

# Point `sherpa-onnx-sys` at the native library `scripts/fetch-sherpa.sh`
# unpacked, so the build never downloads one of its own. Left unset, that build
# script fetches a prebuilt archive from GitHub during `cargo build`, which would
# put an unreviewed binary into the build; see the header of fetch-sherpa.sh.
#
# Deliberately only when `current` is there. Set to a path that does not exist,
# the crate's build script fails with "SHERPA_ONNX_LIB_DIR does not exist or is
# not a directory"; unset, it simply fetches. A worse error is not an
# improvement, so a checkout that has not run the fetch yet still builds — it
# just does the thing this variable exists to prevent, which is why CI runs the
# fetch script as its own step rather than trusting this.
#
# Which copy depends on the target, because the mobile targets cannot use the
# host's. On both, `sherpa-onnx-sys` forces *shared* linking, so handing it the
# static macOS libraries does not merely mis-link — it panics the build script
# with "No shared runtime libraries found in …".
#
# iOS takes a lib directory of its own (`fetch-sherpa.sh --ios`). Android takes
# the *archive* instead, through SHERPA_ONNX_ARCHIVE_DIR: the crate then unpacks
# it and picks the ABI matching the arch being built, which one
# SHERPA_ONNX_LIB_DIR could not do for a build covering several.
#
# The target is read from the command rather than from cargo's own `--target`
# because this wrapper runs *above* the CLI: a device build is
# `with-cargo-env.sh tauri-cli.sh ios build …`, and the `--target
# aarch64-apple-ios` that eventually reaches rustc is constructed inside Tauri,
# long after this has run. So the arguments are what is actually available here,
# and both spellings matter: the CLI's subcommand (`… ios build`) and a target
# triple given directly (`cargo check --target aarch64-linux-android`). Matching
# only the first leaves a direct cross-check pointing at the macOS libraries and
# failing with the panic this whole block exists to prevent.
sherpa_target() {
  local arg
  for arg in "$@"; do
    case "$arg" in
      ios | *-apple-ios* | *-apple-ios) echo "ios"; return ;;
      android | *-linux-android* | *-android) echo "android"; return ;;
    esac
  done
  echo "host"
}

SHERPA_LIB_DIR=""
SHERPA_ARCHIVE_DIR=""
case "$(sherpa_target "$@")" in
  ios) SHERPA_LIB_DIR="$ROOT/.sherpa-onnx/current-ios/lib" ;;
  android) SHERPA_ARCHIVE_DIR="$ROOT/.sherpa-onnx" ;;
  *) SHERPA_LIB_DIR="$ROOT/.sherpa-onnx/current/lib" ;;
esac

if [ -n "$SHERPA_ARCHIVE_DIR" ] && [ -d "$SHERPA_ARCHIVE_DIR" ]; then
  export SHERPA_ONNX_ARCHIVE_DIR="$SHERPA_ARCHIVE_DIR"
elif [ -d "$SHERPA_LIB_DIR" ]; then
  export SHERPA_ONNX_LIB_DIR="$SHERPA_LIB_DIR"
fi

exec "$@"
