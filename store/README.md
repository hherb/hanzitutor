# Store assets and the release build

What is in here is for the Google Play listing; `listing.md` has the copy and
the Console answers, and the images are the ones it refers to.

## Building a signed release

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"
export NDK_HOME="$ANDROID_NDK_HOME"

./scripts/with-cargo-env.sh ./scripts/tauri-cli.sh android build --apk --aab --target aarch64
```

Outputs:

- `src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab`
  — **this** is what Play accepts for a new app.
- `src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk`
  — for installing by hand. Useful because it is the same code the bundle
  carries, so it can be tested on a device before uploading.

The APK filename says `-unsigned` when signing did not happen. That is worth
reading literally: a signed release is `app-universal-release.apk` and an
unsigned one is `app-universal-release-unsigned.apk`, and the difference is not
visible any other way until Play rejects the upload.

## The signing key

- Keystore: `~/.android/hanzitutor-upload.jks` (outside the repository, on
  purpose — a signing key that can be committed eventually will be).
- Passwords and the alias: `src-tauri/gen/android/key.properties`. That file is
  gitignored, so **copy it somewhere safe**. Losing it means generating a new
  upload key; with Play App Signing that is recoverable from the Play Console,
  because Google holds the app signing key.
- Fingerprint of the current key:
  `82:80:5B:DA:3C:99:56:83:84:B3:DD:10:AF:D4:76:A6:3E:13:29:31:BD:B0:E6:DC:E9:31:73:3A:E1:DF:3B:6E`

Without `key.properties` the build still runs and produces an **unsigned**
bundle, which is the honest outcome for a clone that has no key.

## Checking a release before uploading

```bash
# Signed, and by the key you expect
apksigner verify --print-certs .../app-universal-release.apk

# No network permission in the release (the app's whole claim)
aapt2 dump permissions .../app-universal-release.apk

# Side-load and use it: the debug build's webview is debuggable and the
# release build's is not, so check the release by hand, not over DevTools.
adb uninstall com.hanzitutor.app
adb install .../app-universal-release.apk
```

## Artwork

Generated, not hand-drawn, from the icon and the bundled interface font:

```bash
python3 scripts/make-store-assets.py feature-graphic store/feature-graphic-1024x500.png
python3 scripts/make-store-assets.py screenshot raw.png store/phone-screenshots/01-name.png
```

Requires Pillow. Screenshots are padded to 1080×1920 with the app's own
background colour, because Play wants 16:9 or 9:16 and a modern phone is
taller than that.
