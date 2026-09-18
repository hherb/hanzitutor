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

exec "$@"
