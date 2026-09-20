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
if [ -d "$ROOT/.sherpa-onnx/current/lib" ]; then
  export SHERPA_ONNX_LIB_DIR="$ROOT/.sherpa-onnx/current/lib"
fi

exec "$@"
