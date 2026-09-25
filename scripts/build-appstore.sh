#!/usr/bin/env bash
#
# Build the Mac App Store submission: a signed .app, embedded with a Mac App
# Store provisioning profile, packaged as a signed .pkg.
#
# This is deliberately a separate script from `build-release.sh`, not a mode
# flag on it: the signing story is genuinely different (Apple Distribution
# rather than Developer ID) and so is the output (one `.pkg` for App Store
# Connect, no `.dmg` — App Review replaces notarisation, so there is nothing
# to staple either).
#
# ## The one thing this script cannot do for you
#
# It needs a **Mac App Store provisioning profile** for this app's bundle ID,
# saved at `src-tauri/embedded.provisionprofile`. That file is gitignored —
# like `src-tauri/gen/apple/Signing.xcconfig`, it is tied to your Apple
# Developer account, not something to commit — and nothing here can create
# it: get one at
#
#   https://developer.apple.com/account
#     → Certificates, IDs & Profiles → Profiles → +
#     → "Mac App Store Connect" (sometimes labelled "Mac App Store")
#     → App ID: com.hanzitutor.app
#     → certificate: your "Apple Distribution" one
#     → download, save as src-tauri/embedded.provisionprofile
#
# Everything else — signing, embedding, packaging, verifying — is scripted.
#
# ## Signing identities
#
# Taken from, in order, for each of the two roles a Mac App Store submission
# needs (an app-signing certificate and a separate installer-signing one —
# `productbuild` will not accept the app one):
#
#   1. $APPLE_APPSTORE_SIGNING_IDENTITY / $APPLE_INSTALLER_SIGNING_IDENTITY,
#      if you set them;
#   2. the first "Apple Distribution" / "3rd Party Mac Developer Installer"
#      certificate in your keychain.
#
# Unlike `build-release.sh`'s Developer ID identity, neither falls back to
# ad-hoc: an ad-hoc-signed, unprovisioned bundle cannot reach App Store
# Connect, so a missing certificate is a script failure here rather than a
# degraded build. Identities are deliberately not written into
# `tauri.conf.json` for the same reason `build-release.sh`'s aren't — a
# personal certificate name does not belong in a committed file.
#
# ## Where the App ID and team ID come from
#
# Read out of the provisioning profile itself, at sign time, rather than
# hardcoded or parsed out of a certificate name — the profile is the one
# place both are guaranteed to agree with what Apple's own validation checks
# against. `codesign` does not merge these into the signature automatically
# from an embedded profile the way Xcode's own
# `CODE_SIGN_INJECT_BASE_ENTITLEMENTS` build setting does, so this script
# builds the combined entitlements file by hand.
#
# Usage: scripts/build-appstore.sh
#
# Set `TAURI_ROOT` to build one of the repository's other Tauri apps, the
# same way `build-release.sh` does — though a second app would need its own
# provisioning profile at `<app>/src-tauri/embedded.provisionprofile` before
# this does anything useful for it.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${TAURI_ROOT:-$REPO}"

PROFILE="$ROOT/src-tauri/embedded.provisionprofile"
if [ ! -f "$PROFILE" ]; then
  cat >&2 <<EOF
error: no Mac App Store provisioning profile at:
  $PROFILE

Get one at https://developer.apple.com/account, under Certificates, IDs &
Profiles → Profiles → + → "Mac App Store Connect" → App ID com.hanzitutor.app
→ your "Apple Distribution" certificate → download, and save it at the path
above. See this script's own header comment for the full walkthrough.
EOF
  exit 1
fi

if [ -z "${APPLE_APPSTORE_SIGNING_IDENTITY:-}" ]; then
  APPLE_APPSTORE_SIGNING_IDENTITY="$(security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*"\(Apple Distribution: [^"]*\)".*/\1/p' | head -1)"
fi
if [ -z "${APPLE_APPSTORE_SIGNING_IDENTITY:-}" ]; then
  echo "error: no \"Apple Distribution\" certificate found in the keychain, and" >&2
  echo "       APPLE_APPSTORE_SIGNING_IDENTITY is not set." >&2
  exit 1
fi
export APPLE_APPSTORE_SIGNING_IDENTITY

if [ -z "${APPLE_INSTALLER_SIGNING_IDENTITY:-}" ]; then
  # Not `-p codesigning`: installer certificates are a different policy and
  # do not show up in that list, only in the unfiltered one.
  APPLE_INSTALLER_SIGNING_IDENTITY="$(security find-identity -v 2>/dev/null \
    | sed -n 's/.*"\(3rd Party Mac Developer Installer: [^"]*\)".*/\1/p' | head -1)"
fi
if [ -z "${APPLE_INSTALLER_SIGNING_IDENTITY:-}" ]; then
  echo "error: no \"3rd Party Mac Developer Installer\" certificate found in the" >&2
  echo "       keychain, and APPLE_INSTALLER_SIGNING_IDENTITY is not set." >&2
  exit 1
fi

echo "app identity:       $APPLE_APPSTORE_SIGNING_IDENTITY"
echo "installer identity:  $APPLE_INSTALLER_SIGNING_IDENTITY"

# The two entitlements Apple's Mac App Store validation requires and Xcode
# would otherwise inject automatically. Read from the profile itself — see
# the header comment for why — not reconstructed from parts.
PROFILE_PLIST="$(security cms -D -i "$PROFILE")"
# The dots inside these key names are literal, not keypath nesting — plutil's
# keypath syntax uses `.` as a separator, so each one needs escaping or
# `application-identifier` reads as `Entitlements -> com -> apple ->
# application-identifier`, which does not exist.
APP_IDENTIFIER="$(echo "$PROFILE_PLIST" \
  | plutil -extract 'Entitlements.com\.apple\.application-identifier' raw -o - -)"
TEAM_IDENTIFIER="$(echo "$PROFILE_PLIST" \
  | plutil -extract 'Entitlements.com\.apple\.developer\.team-identifier' raw -o - -)"
if [ -z "$APP_IDENTIFIER" ] || [ -z "$TEAM_IDENTIFIER" ]; then
  echo "error: could not read application-identifier/team-identifier out of" >&2
  echo "       $PROFILE" >&2
  exit 1
fi

tauri() {
  (cd "$ROOT" && "$REPO/scripts/with-cargo-env.sh" "$REPO/scripts/tauri-cli.sh" "$@")
}

APP_DIR="${APP_ROOT:-$ROOT}"
BUNDLE=""
for dir in "$REPO/.cargo-target/release/bundle" "$APP_DIR/src-tauri/target/release/bundle"; do
  if [ -d "$dir" ]; then
    BUNDLE="$dir"
    break
  fi
done

# `-b app`: no .dmg, and no `ios` bundle Tauri would otherwise also attempt
# on a machine that has an iOS toolchain configured.
APPLE_SIGNING_IDENTITY="$APPLE_APPSTORE_SIGNING_IDENTITY" tauri build -b app

if [ -z "$BUNDLE" ]; then
  echo "error: could not find the bundle directory after the build." >&2
  exit 1
fi

PRODUCT_NAME="$(sed -n 's/.*"productName": *"\([^"]*\)".*/\1/p' "$ROOT/src-tauri/tauri.conf.json" | head -1)"
APP="$BUNDLE/macos/$PRODUCT_NAME.app"
if [ ! -d "$APP" ]; then
  echo "error: expected $APP after the build, and it is not there — other bundles" >&2
  echo "       (this repo builds more than one Tauri app into the same" >&2
  echo "       .cargo-target/release/bundle/macos/) share that directory, so this" >&2
  echo "       looks up productName from tauri.conf.json rather than globbing for" >&2
  echo "       any *.app in it." >&2
  exit 1
fi
echo "built app: $APP"

# Embedded *after* Tauri's own signing pass, which is why the whole bundle is
# re-signed below: adding a file invalidates the resource seal a signature
# already covers, the same reason iOS's `embedded.mobileprovision` has to be
# in place before Xcode's own signing step rather than after.
cp "$PROFILE" "$APP/Contents/embedded.provisionprofile"

COMBINED_ENTITLEMENTS="$(mktemp -t hanzi-tutor-appstore-entitlements).plist"
trap 'rm -f "$COMBINED_ENTITLEMENTS"' EXIT
cp "$ROOT/src-tauri/Entitlements.plist" "$COMBINED_ENTITLEMENTS"
# Escaped for the same reason the `-extract` calls above are: plutil's keypath
# syntax reads an unescaped `.` as nesting, not as part of the key's own name.
plutil -insert 'com\.apple\.application-identifier' -string "$APP_IDENTIFIER" "$COMBINED_ENTITLEMENTS"
plutil -insert 'com\.apple\.developer\.team-identifier' -string "$TEAM_IDENTIFIER" "$COMBINED_ENTITLEMENTS"

# No `--deep`: the first pass already correctly signed anything nested (the
# TTS/ASR frameworks, if any are bundled), and re-signing them a second time
# from here risks getting that wrong rather than leaving a valid signature
# alone. `--options runtime`: Tauri's own signing pass already applies the
# hardened runtime (see `Entitlements.plist`'s header comment), and this
# re-sign should not quietly drop it.
codesign --force --options runtime \
  --entitlements "$COMBINED_ENTITLEMENTS" \
  --sign "$APPLE_APPSTORE_SIGNING_IDENTITY" \
  "$APP"

echo "verifying the re-signed app…"
codesign --verify --deep --strict "$APP"
codesign -d --entitlements - "$APP"

# `plutil`, not PlistBuddy: PlistBuddy opens the file expecting write access
# even for `Print`, and a signed bundle's Info.plist is read-only — harmless
# (it still reads and prints correctly) but it logs a scary-looking "write:
# Permission denied" that has nothing to do with this script's own signing.
VERSION="$(plutil -extract CFBundleShortVersionString raw -o - "$APP/Contents/Info.plist")"
APP_NAME="$(basename "$APP" .app)"
OUT_DIR="$BUNDLE/appstore"
mkdir -p "$OUT_DIR"
PKG="$OUT_DIR/${APP_NAME}_${VERSION}.pkg"

# A handful of "write: Permission denied" lines on stderr here are
# `productbuild`'s own noise, not this script's — reproduced by running
# `productbuild` on its own with no relation to anything above, and the
# package it writes is still correctly signed either way (verified below).
productbuild --component "$APP" /Applications \
  --sign "$APPLE_INSTALLER_SIGNING_IDENTITY" \
  "$PKG"

echo "verifying the package signature…"
pkgutil --check-signature "$PKG"

cat <<EOF

built: $PKG

To upload it to App Store Connect:
  - Transporter (free from the Mac App Store): open it, sign in, drag the
    .pkg in, and click Deliver. This is the primary, supported path.
  - Or from the command line:
      xcrun altool --upload-app -f "$PKG" -t macos \\
        -u <your Apple ID> -p <an app-specific password>
    (generate an app-specific password at appleid.apple.com; this script
    never asks for or stores one)

Uploading puts a build on App Store Connect for you to see; it does not
submit anything for review — that is still your call to make there.
EOF
