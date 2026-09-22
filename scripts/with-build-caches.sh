#!/usr/bin/env bash
#
# Run a build with every tool's scratch space inside the repository, so that the
# workspace-write file policy is enough — no full-access grant, and no
# per-command approval.
#
#   ./scripts/with-build-caches.sh ./scripts/with-cargo-env.sh \
#     ./scripts/tauri-cli.sh android build --apk --aab --target aarch64
#
# Two of the tools reach outside the workspace by default, and both are
# redirected here. What was measured, and what did not work, is in HANDOVER §8.
#
#   Gradle  writes its wrapper distribution and its dependency cache under
#           ~/.gradle, and refuses to start when it cannot. GRADLE_USER_HOME
#           moves both into .gradle-home/, and GRADLE_RO_DEP_CACHE lets it *read*
#           the real cache rather than downloading everything again. The
#           distribution is seeded by copy — an APFS clone, so it costs no disk —
#           when .gradle-home has none. The Kotlin daemon still tries
#           ~/Library/Application Support/kotlin and logs a FileSystemException
#           per marker file: that is noise, and the build succeeds.
#
#   Xcode   keeps its build directory under the *user's* home, and no environment
#           variable moves it — not HOME, and not CFFIXED_USER_HOME; xcodebuild
#           asks the directory services rather than the environment. It does
#           honour `-derivedDataPath`, which the Tauri CLI never passes, so a
#           shim goes in front of it on PATH and adds the flag. The export action
#           rejects that flag, so the shim only adds it to `build` and `archive`.
#
# The disk image is the one step this cannot help with: `hdiutil create` is
# refused in the sandbox however the paths are arranged (HANDOVER §8 has the
# evidence), so the .dmg is made by hand or under a wider policy. The .app is
# unaffected — it is written and signed before the image is attempted.
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $(basename "$0") <command> [args...]" >&2
  exit 2
fi

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# ---- Gradle: its home, and a read-only view of the real dependency cache ----
export GRADLE_USER_HOME="$repo/.gradle-home"
if [ -d "$HOME/.gradle/caches" ]; then
  export GRADLE_RO_DEP_CACHE="$HOME/.gradle/caches"
fi
if [ ! -d "$GRADLE_USER_HOME/wrapper/dists" ] && [ -d "$HOME/.gradle/wrapper/dists" ]; then
  mkdir -p "$GRADLE_USER_HOME/wrapper"
  # `-c` clones on APFS; the fallback is for a filesystem that has no such thing.
  cp -Rc "$HOME/.gradle/wrapper/dists" "$GRADLE_USER_HOME/wrapper/dists" 2>/dev/null ||
    cp -R "$HOME/.gradle/wrapper/dists" "$GRADLE_USER_HOME/wrapper/dists"
fi

# ---- Xcode: the DerivedData path the Tauri CLI does not expose --------------
shim="$repo/.build-tools/bin"
mkdir -p "$shim" "$repo/.xcode-derived"
cat > "$shim/xcodebuild" <<SHIM
#!/bin/bash
flags=()
case " \$* " in
  *" -exportArchive "*) ;;
  *" build "*|*" archive "*) flags=(-derivedDataPath "$repo/.xcode-derived") ;;
esac
exec /usr/bin/xcodebuild "\$@" "\${flags[@]}"
SHIM
chmod +x "$shim/xcodebuild"
export PATH="$shim:$PATH"

exec "$@"
