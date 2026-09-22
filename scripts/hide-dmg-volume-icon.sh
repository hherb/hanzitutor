#!/usr/bin/env bash
#
# Make the .dmg's volume icon invisible, so Finder stops drawing it in the window.
#
# Tauri's disk-image script — a fork of create-dmg — copies the app icon to
# `.VolumeIcon.icns` at the root of the mounted volume and marks the volume as
# having a custom icon, but it never sets the *invisible* flag on that file:
#
#     cp "$VOLUME_ICON_FILE" "$MOUNT_DIR/.VolumeIcon.icns"
#     SetFile -c icnC "$MOUNT_DIR/.VolumeIcon.icns"     # creator, not visibility
#     ...
#     SetFile -a C "$MOUNT_DIR"                          # the volume's own flag
#
# `SetFile -a V` is the line that is missing, and without it the mounted image
# shows a large dimmed duplicate of the app icon as a file sitting above the two
# items the layout places — which reads as a rendering fault rather than as the
# volume's icon. `scripts/build-release.sh` calls this after a successful build.
#
# A compressed image cannot be mounted read-write, so the image is converted to
# UDRW, patched, and converted back. That rewrites the file, which **invalidates
# the signature Tauri applied to the .dmg**, so it is re-signed here with the
# same identity the build used — an unsigned image would turn a
# right-click-Open into a "damaged" dialog.
#
# The .app inside is untouched, and so is its signature: only the container's
# icon flag and its own signature change.
#
# Usage: scripts/hide-dmg-volume-icon.sh <file.dmg>
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $(basename "$0") <file.dmg>" >&2
  exit 2
fi

dmg="$1"
if [ ! -f "$dmg" ]; then
  echo "no such disk image: $dmg" >&2
  exit 1
fi

work="$(mktemp -d)"
mnt="$work/mnt"
mkdir -p "$mnt"
cleanup() {
  hdiutil detach "$mnt" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT

hdiutil convert "$dmg" -format UDRW -o "$work/rw" >/dev/null
hdiutil attach "$work/rw.dmg" -readwrite -nobrowse -noverify -mountpoint "$mnt" >/dev/null

icon="$mnt/.VolumeIcon.icns"
if [ ! -e "$icon" ]; then
  echo "$(basename "$dmg"): no volume icon to hide"
  exit 0
fi

SetFile -a V "$icon"
echo "hid $(basename "$icon") on the volume"

hdiutil detach "$mnt" >/dev/null
hdiutil convert "$work/rw.dmg" -format UDZO -o "$work/final" >/dev/null
mv "$work/final.dmg" "$dmg"

# Tauri signed the image before this ran; the file is different now, so the
# signature has to be redone or removed. Ad-hoc ("-") is what build-release.sh
# falls back to when there is no certificate, and signing ad-hoc here would
# claim a signature the original did not have — so only a real identity re-signs.
identity="${APPLE_SIGNING_IDENTITY:-}"
if [ -n "$identity" ] && [ "$identity" != "-" ]; then
  codesign --force --sign "$identity" "$dmg"
  codesign --verify --verbose=1 "$dmg"
  echo "re-signed $(basename "$dmg") as: $identity"
else
  echo "note: APPLE_SIGNING_IDENTITY is unset or ad-hoc; the image is left unsigned" >&2
fi
