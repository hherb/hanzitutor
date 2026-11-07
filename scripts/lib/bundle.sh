# shellcheck shell=bash
#
# Where a `tauri build` left its bundles, and which of them belong to the app
# being built. Sourced by `build-release.sh` and `build-appstore.sh`, and by
# `scripts/tests/test-scripts.sh`, which exercises it on a fake directory tree
# because the scripts that use it only run on macOS.
#
# Two rules every function here follows, because both scripts got them wrong
# before this file existed (issue #38):
#
#   * **Ask after the build, not before.** On a fresh target directory there is
#     no `release/bundle` until `tauri build` has made one, so a lookup done
#     first finds nothing and keeps finding nothing: the App Store script then
#     failed a build that had succeeded, and the release script skipped every
#     post-build step without a word.
#   * **Filter by the app's own name.** All three apps build into the one
#     `.cargo-target/release/bundle/`, so "every `.app` and `.dmg` in there"
#     includes the other apps' leftovers — which the release script used to
#     re-sign and list as if it had just built them.
#
# Each function prints what it finds, one path per line, and prints nothing —
# successfully — when there is nothing, so it is safe inside `$(...)` under
# `set -euo pipefail`; the caller decides what an empty answer means. Written for
# bash 3.2, which is what `/usr/bin/env bash` finds on a stock Mac.

# bundle_conf_field <tauri.conf.json> <field>
#
# A top-level string field of a Tauri config (`productName`, `version`). A
# line-based read rather than a JSON parser, as it always was here: the field is
# on a line of its own in every config this repository has, and the first match
# is the top-level one because Tauri puts both at the top.
bundle_conf_field() {
  [ -f "$1" ] || return 0
  sed -n "s/.*\"$2\": *\"\\([^\"]*\\)\".*/\\1/p" "$1" | head -n 1 || true
}

# bundle_dir <repo> <app-dir>
#
# The `release/bundle` directory the build wrote: under the cargo target
# directory `with-cargo-env.sh` points the build at — an absolute
# `$CARGO_TARGET_DIR` if one is set, otherwise `<repo>/.cargo-target` — or else
# the app's own `src-tauri/target`. A *relative* `$CARGO_TARGET_DIR` is not
# guessed at: cargo resolves it against whichever directory the Tauri CLI runs it
# from, which is the CLI's business, so it falls through to the two defaults.
bundle_dir() {
  local repo="$1" app_dir="$2" target="" dir
  case "${CARGO_TARGET_DIR:-}" in
    /*) target="$CARGO_TARGET_DIR/release/bundle" ;;
  esac
  for dir in "$target" "$repo/.cargo-target/release/bundle" "$app_dir/src-tauri/target/release/bundle"; do
    if [ -n "$dir" ] && [ -d "$dir" ]; then
      printf '%s\n' "$dir"
      return 0
    fi
  done
  return 0
}

# bundle_app <bundle-dir> <productName>
#
# The `.app` Tauri names after `productName`. Not a glob over `macos/*.app`,
# which would also match the other apps sharing the directory.
bundle_app() {
  local app="$1/macos/$2.app"
  if [ -n "$1" ] && [ -n "$2" ] && [ -d "$app" ]; then
    printf '%s\n' "$app"
  fi
  return 0
}

# bundle_dmgs <bundle-dir> <productName> [version]
#
# This app's disk images. Tauri names them `<productName>_<version>_<arch>.dmg`,
# so the underscore keeps "Hanzi Tutor" from matching a "Hanzi Tutor Lite", and
# the version — when given — keeps an earlier release's image, still sitting in
# the same directory, from being re-signed and listed as this build's.
bundle_dmgs() {
  local dir="$1" name="$2" version="${3:-}" prefix image
  [ -n "$dir" ] && [ -n "$name" ] || return 0
  prefix="${name}_"
  if [ -n "$version" ]; then
    prefix="${name}_${version}_"
  fi
  for image in "$dir/dmg/$prefix"*.dmg; do
    if [ -f "$image" ]; then
      printf '%s\n' "$image"
    fi
  done
  return 0
}
