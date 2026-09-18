#!/usr/bin/env bash
#
# Run the Tauri CLI, working around a hosting quirk.
#
# `@tauri-apps/cli` is a native napi addon that parses the *operating system*
# process arguments. That is fine under a normal Node, but a host that runs Node
# inside another application breaks it: the DSH desktop app's pnpm shim launches
# Electron with ELECTRON_RUN_AS_NODE=1, so the addon sees the host's own argv and
# the CLI fails with "unrecognized subcommand '/Applications/DSH Desktop.app/...'".
#
# So the npm CLI is probed first and a standalone `cargo-tauri` used as fallback.
# Nothing here is DSH-specific: the probe simply asks whether the CLI can report
# its own version.
#
# Usage: scripts/tauri-cli.sh <tauri arguments>
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# 1. A CLI installed into the project, if one has been (`pnpm run install:cli`).
if [ -x "$ROOT/.cargo-tools/bin/cargo-tauri" ]; then
  exec "$ROOT/.cargo-tools/bin/cargo-tauri" "$@"
fi

# 2. The standard npm CLI, if this host can actually run it.
NPM_CLI="$ROOT/node_modules/@tauri-apps/cli/tauri.js"
if [ -f "$NPM_CLI" ] && node "$NPM_CLI" --version >/dev/null 2>&1; then
  exec node "$NPM_CLI" "$@"
fi

# 3. A standalone CLI from the toolchain.
if command -v cargo-tauri >/dev/null 2>&1; then
  echo "note: the npm Tauri CLI cannot run on this host; using $(command -v cargo-tauri)" >&2
  exec cargo-tauri "$@"
fi

cat >&2 <<'EOF'
error: no usable Tauri CLI found.

Install one, either into this project:

  pnpm run install:cli

or onto your toolchain:

  cargo install tauri-cli --version '^2'
EOF
exit 1
