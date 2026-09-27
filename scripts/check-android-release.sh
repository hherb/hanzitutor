#!/usr/bin/env bash
#
# Check an Android release bundle before it goes to the Play Console.
#
# Everything here is checked against the artifact rather than against the notes in
# `store/listing.md`, because the notes can be stale and the artifact cannot: the
# digest, the signature, the manifest Play will read, the native libraries of every
# ABI, and the download size Play will enforce. What it does not do is decide
# whether the *answers* in the Console are right — that is `store/listing.md` and
# `docs/privacy-policy.md`, and no script can read a policy.
#
# Usage:
#
#     scripts/check-android-release.sh [--install] [path/to/app.aab]
#
# Defaults to the release bundle the Tauri CLI writes. `--install` then installs
# the splits generated from it onto the connected device, which is the check that
# says the bundle is not merely well-formed but is the thing a phone will run.
# Exits non-zero if the bundle is missing, unsigned, or over Play's download limit.
#
# **bundletool is not downloaded.** The Play Console runs it on upload, and this
# needs the same reading of the bundle, but fetching `bundletool-all.jar` from
# GitHub would put an unpinned binary into a release check — the opposite of what
# this project does with `scripts/fetch-sherpa.sh`. It is already on the machine:
# the Android Gradle Plugin depends on `com.android.tools.build:bundletool`, so a
# build leaves it in the Gradle cache, and it can be run from there. Two things
# about that are not obvious and cost time to find:
#
#   * The cached jar is the **library**, not the `-all` fat jar, so it needs a
#     classpath of everything else in the same cache to start at all.
#   * It needs `--aapt2` pointed at the SDK's, or it dies constructing its Dagger
#     graph with an error that names aapt2 only in a stack frame.
#
# `get-size` also measures **APKs, not a bundle**, so `build-apks` has to run
# first — which is why this takes a few minutes and a few hundred MB of scratch
# space rather than being a one-liner.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
install_to_device=0
bundle=""
for arg in "$@"; do
  case "$arg" in
    --install) install_to_device=1 ;;
    -h | --help) sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) bundle="$arg" ;;
  esac
done
bundle="${bundle:-$repo/src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab}"

# Play's limit on a module's compressed download, which is what get-size reports.
limit_mb=200

[ -f "$bundle" ] || {
  echo "error: no bundle at $bundle" >&2
  echo "       build one: ./scripts/with-build-caches.sh ./scripts/with-cargo-env.sh \\" >&2
  echo "                    ./scripts/tauri-cli.sh android build --apk --aab" >&2
  exit 1
}

java_bin="${JAVA_HOME:+$JAVA_HOME/bin/java}"
java_bin="${java_bin:-$(command -v java || true)}"
[ -x "$java_bin" ] || { echo "error: no java; set JAVA_HOME" >&2; exit 1; }

# ---- bundletool, from the Gradle cache, without downloading it --------------
cache="$repo/.gradle-home/caches/modules-2/files-2.1"
[ -d "$cache" ] || cache="$HOME/.gradle/caches/modules-2/files-2.1"
classpath="$(find "$cache" -name '*.jar' ! -name '*-sources.jar' 2>/dev/null | tr '\n' ':')"
[ -n "$classpath" ] || { echo "error: no jars under $cache; build once first" >&2; exit 1; }
version_of() { printf '%s\n' "$1" | sed 's/.*bundletool-\([0-9.]*\)\.jar/\1/'; }
bundletool_jar="$(find "$cache" -name 'bundletool-*.jar' ! -name '*-sources.jar' 2>/dev/null | sort -V | tail -1)"
[ -n "$bundletool_jar" ] || { echo "error: bundletool is not in the Gradle cache" >&2; exit 1; }

# `--aapt2` is required, and the newest build-tools is the sensible one.
aapt2="$(find "${ANDROID_HOME:-$HOME/Library/Android/sdk}/build-tools" -maxdepth 2 -name aapt2 2>/dev/null | sort -V | tail -1)"
[ -x "$aapt2" ] || { echo "error: no aapt2 under build-tools" >&2; exit 1; }

bundletool() { "$java_bin" -cp "$classpath" com.android.tools.build.bundletool.BundleToolMain "$@"; }

echo "bundle:     $bundle"
echo "bundletool: $(version_of "$bundletool_jar") (from the Gradle cache)"
echo

# ---- 1. Is it signed, and by the key we think? ------------------------------
echo "== signature =="
jarsigner="${JAVA_HOME:+$JAVA_HOME/bin/jarsigner}"
jarsigner="${jarsigner:-$(command -v jarsigner || true)}"
if [ -x "$jarsigner" ]; then
  # The self-signed and no-timestamp warnings are what an upload key looks like;
  # an unsigned bundle is the failure worth stopping for.
  if "$jarsigner" -verify "$bundle" 2>&1 | grep -q "jar verified"; then
    keytool="${JAVA_HOME:+$JAVA_HOME/bin/keytool}"
    keytool="${keytool:-$(command -v keytool || true)}"
    echo "signed — signer SHA-256: $("$keytool" -printcert -jarfile "$bundle" 2>/dev/null \
      | sed -n 's/.*SHA256: //p' | head -1)"
    echo "        (compare with store/README.md; the self-signed warning is expected)"
  else
    echo "error: the bundle is NOT signed — Play will reject it" >&2
    exit 1
  fi
else
  echo "skipped: no jarsigner on PATH"
fi
echo

# ---- 2. What Play will read out of it --------------------------------------
echo "== manifest, as bundletool reads it =="
manifest="$(bundletool dump manifest --bundle="$bundle" 2>/dev/null)"
# The fields worth reading, one per line: the `<manifest>` element is one long
# XML line, and everything interesting in it is an attribute.
printf '%s\n' "$manifest" \
  | grep -oE 'package="[^"]*"|android:versionCode="[^"]*"|android:versionName="[^"]*"' \
  | sed 's/^/  /'
printf '%s\n' "$manifest" | grep -E "uses-sdk|uses-permission" | sed 's/^ */  /'
echo

# ---- 3. Is every ABI's native code complete? --------------------------------
# Play serves four ABIs and this machine can run one of them — a phone, or the
# arm64 emulator image the SDK ships. A library missing from one ABI, or linking
# against something that ABI's directory does not carry, is a crash for the
# devices on it, and nothing else in this script would notice: the bundle is
# well-formed and the sizes are fine. So the check is static, which is exactly
# what the loader does — every `DT_NEEDED` has to be either another library in the
# same directory or one Android itself provides.
echo "== native libraries per ABI =="
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# The NDK, and its `llvm-readelf`. Note the shape of this: the SDK's `ndk/<version>`
# is a *symlink* here (to Homebrew's ndk), and a plain `find "$sdk/ndk" -name
# llvm-readelf` therefore returns nothing at all — find does not descend into a
# symlinked directory without `-L`, and it reports no error either, so the check
# below silently skips. Globbing the path is both shorter and correct.
ndk_root="${ANDROID_NDK_HOME:-}"
if [ -z "$ndk_root" ]; then
  ndk_root="$(ls -d "${ANDROID_HOME:-$HOME/Library/Android/sdk}/ndk/"* 2>/dev/null | sort -V | tail -1)"
fi
readelf="$(ls "$ndk_root"/toolchains/llvm/prebuilt/*/bin/llvm-readelf 2>/dev/null | head -1)"
if [ -x "${readelf:-}" ]; then
  # What the platform provides. Anything else has to travel in the bundle.
  system_libs="libc.so libm.so libdl.so liblog.so libandroid.so libz.so libOpenSLES.so libaaudio.so libstdc++.so libjnigraphics.so libEGL.so libGLESv2.so libvulkan.so ld-android.so"
  native_ok=1
  for abi in $(unzip -l "$bundle" | grep -oE "base/lib/[a-z0-9_-]+/" | sort -u | cut -d/ -f3); do
    dir="$scratch/$abi"
    unzip -o -q "$bundle" "base/lib/$abi/*" -d "$dir"
    libs="$(ls "$dir/base/lib/$abi")"
    bad=""
    for so in $libs; do
      for need in $("$readelf" -d "$dir/base/lib/$abi/$so" 2>/dev/null \
        | sed -n 's/.*NEEDED.*\[\(.*\)\]/\1/p'); do
        if ! printf '%s\n' "$libs" | grep -qx "$need" \
          && ! printf '%s\n' $system_libs | grep -qx "$need"; then
          bad="$bad ${so}→${need}"
        fi
      done
    done
    if [ -n "$bad" ]; then
      echo "  $abi: UNRESOLVED:$bad" >&2
      native_ok=0
    else
      echo "  $abi: $(printf '%s\n' "$libs" | wc -l | tr -d ' ') libraries, all dependencies resolve"
    fi
  done
  [ "$native_ok" = 1 ] || { echo "error: a native dependency does not resolve" >&2; exit 1; }
else
  echo "  skipped: no llvm-readelf under the NDK"
fi
echo

# ---- 4. What a device will actually download --------------------------------
# This is the number Play enforces, and it is nothing like the bundle's own size:
# the bundle carries every ABI and the native debug symbols, and Play delivers
# neither to a device.
echo "== download size (Play's limit: ${limit_mb} MB per module) =="

apks_args=(--bundle="$bundle" --output="$scratch/app.apks" --aapt2="$aapt2")
key_properties="$repo/src-tauri/gen/android/key.properties"
if [ -f "$key_properties" ]; then
  # Signing is not needed to *measure*, but a signed set can also be installed on
  # a phone, which is the check that says the bundle is not merely well-formed.
  store_file="$(sed -n 's/^storeFile=//p' "$key_properties")"
  store_pass="$(sed -n 's/^storePassword=//p' "$key_properties")"
  alias_name="$(sed -n 's/^keyAlias=//p' "$key_properties")"
  key_pass="$(sed -n 's/^keyPassword=//p' "$key_properties")"
  if [ -f "$store_file" ]; then
    apks_args+=(--ks="$store_file" --ks-key-alias="$alias_name" \
      --ks-pass="pass:$store_pass" --key-pass="pass:$key_pass")
  fi
fi

echo "  building the APK set from the bundle (takes a few minutes)…"
bundletool build-apks "${apks_args[@]}" >/dev/null 2>&1

echo "  by ABI:"
sizes="$(bundletool get-size total --apks="$scratch/app.apks" --dimensions=ABI \
  --human-readable-sizes 2>/dev/null)"
printf '%s\n' "$sizes" | tail -n +2 | sed 's/^/    /'

# The limit applies to the largest figure any device is served, so that is the
# one to compare — not the bundle's own size, which is several times larger.
over_limit=0
worst="$(printf '%s\n' "$sizes" | tail -n +2 \
  | awk -F, '{gsub(/ MB/,"",$3); v=$3+0; if (v>m) m=v} END {printf "%.2f", m}')"
if awk "BEGIN{exit !($worst > $limit_mb)}"; then
  echo "  OVER THE LIMIT: the largest ABI is ${worst} MB, over Play's ${limit_mb} MB" >&2
  over_limit=1
else
  echo "  largest ABI: ${worst} MB — inside Play's ${limit_mb} MB limit"
fi

# With a phone attached, the figure for that device is the one that matters — and
# it is the check the person doing the release can see for themselves.
adb="${ANDROID_HOME:-$HOME/Library/Android/sdk}/platform-tools/adb"
have_device=0
if [ -x "$adb" ] && [ -n "$("$adb" devices | sed -n '2p')" ]; then
  have_device=1
  if bundletool get-device-spec --output="$scratch/spec.json" --adb="$adb" >/dev/null 2>&1; then
    echo "  for the connected device:"
    bundletool get-size total --apks="$scratch/app.apks" \
      --device-spec="$scratch/spec.json" --human-readable-sizes 2>/dev/null \
      | tail -n +2 | sed 's/^/    /'
  fi
fi

# Installing the splits Play would serve is a stronger statement than any size:
# it is the same code path a phone takes from the store, and it keeps the
# learner's data, because the package and the upload key are unchanged.
#
# The one exception is the 0.5.11 rename to `com.hherb.hanzitutor`: a device still
# carrying the old `com.hanzitutor.app` install is a *different* app to Android, so
# this one lands beside it rather than over it, and the old install's data stays
# behind under the old name until that is uninstalled. Nothing had been published
# when the rename happened, so no learner was affected by it.
if [ "$install_to_device" = 1 ]; then
  [ "$have_device" = 1 ] || { echo "error: --install but no device attached" >&2; exit 1; }
  echo
  echo "  installing the generated splits on the connected device…"
  bundletool install-apks --apks="$scratch/app.apks" --adb="$adb" 2>&1 | tail -2 | sed 's/^/    /'
  # The component name is applicationId/namespace.Activity, and those two differ
  # on purpose here: Play fixes the package name, so `applicationId` is
  # `com.hherb.hanzitutor` while the Kotlin namespace stayed
  # `com.hanzitutor.app`. See the comment on `applicationId` in
  # `app/build.gradle.kts`; `am start` with the short `.MainActivity` form does
  # not resolve, because it expands against the applicationId.
  echo "    launch it with: $adb shell am start -n com.hherb.hanzitutor/com.hanzitutor.app.MainActivity"
elif [ "$have_device" = 1 ]; then
  echo
  echo "  re-run with --install to put these splits on the connected device"
fi
echo

# ---- 5. The digest, for store/listing.md -----------------------------------
echo "== digest =="
echo "  $(shasum -a 256 "$bundle" | cut -d' ' -f1)  ($(wc -c < "$bundle" | tr -d ' ') bytes)"
echo
echo "Recorded in store/listing.md; the size figures above are the ones Play enforces."

exit "$over_limit"
