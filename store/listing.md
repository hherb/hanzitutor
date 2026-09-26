# Google Play listing copy

Everything here is ready to paste into the Play Console. Character limits are
Play's, and the copy is written to stay inside them.

## App name (max 30 characters)

```
Hanzi Tutor
```

## Short description (max 80 characters)

```
Learn to write Chinese characters. Every stroke graded, and it works offline.
```

*(76 characters.)*

## Full description (max 4000 characters)

```
Hanzi Tutor teaches you to read and write simplified Chinese by hand — and it
tells you what you actually got wrong.

Write a character with your finger or a stylus and the app grades every stroke:
its shape, where you put it, how much ink you used, and whether it came in the
right order. You get a score out of 100 and, more usefully, the reasons behind
it — "not yet legible", "stroke order off", "wrong place" — so the next attempt
is a correction rather than a guess.

WHAT IT DOES

• A frequency-ordered course of 7,744 characters in 775 lessons, so the
  characters you meet first are the ones you will actually read.
• Honest grading against real stroke geometry, not a shape-drawing toy.
• Trace mode with a faint guide, and recall mode when you want to be tested.
• Stroke-order animation for any character, showing the order it should be
  written in.
• The full HSK 3.0 word list — search it by character, by reading with or
  without tone marks, or by meaning in English.
• Your own vocabulary list, with groups, for the words you meet in your own
  reading.
• Spaced repetition: the characters you find hard come back when you need
  them, and the ones you know stop wasting your time.
• Colour by tone. Every character is drawn in the colour of the tone it is read
  with — on the practice board, in the word list, in your own vocabulary, in the
  lessons and in the tone pairs — and the pinyin is coloured with it, syllable by
  syllable, so 妈, 麻, 马 and 骂 are told apart at a glance. The tone is the one
  the character is learnt with, out of a dictionary, and a character whose tone
  is not known keeps the ordinary ink rather than being guessed at.
• Tone practice. Hold the button, say the character out loud, and the app
  scores your tone — no model to download and nothing sent anywhere.
• Hear any character or word spoken by the system's own voice.

OFFLINE, AND PRIVATE BY DESIGN

Everything the course teaches is in the app: the character set, the stroke data,
the word list and the font. There is nothing to download on first run and no
account to create. No analytics, advertising or crash reporting is built in.

Nothing at all is sent anywhere unless you choose it. There are two things you
can choose, and both are off by default. One is an optional on-device speech
model, about 163 MB, that recognises *which* syllables you said rather than only
how your tone sounded — fetched from the settings screen, only if you press the
button there, after being told the address, the size and the licence. Decline it
and nothing changes; tone practice, pronunciation and the whole course work with
no model and no network. The other is syncing between your own devices, which
sends your practice history, vocabulary list and place in the course to a Dropbox
account **you** connect, so that your phone and your laptop agree. Connect
nothing and it stays on this device.

Your practice history and vocabulary list are stored on your device, in the
app's own private storage, and never leave it. You can export them to a file
whenever you like, and import them on another device.

The microphone is used for one thing: tone practice, and only while you are
holding the button. What it hears is analysed on the device, is never written
to storage, and is never transmitted. Decline the permission and everything
else still works.

WHAT IT IS NOT

There is no AI chatbot, no subscription and no streak to protect. It is a
practice tool: you write, it tells you the truth about what you wrote.

Requires no account. No ads. No in-app purchases.
```

## Categorisation

- **Category:** Education
- **Tags:** Language learning, Handwriting
- **Contact email:** support@hherb.com
- **Privacy policy URL:** https://hherb.com/hanzi-tutor/privacy

## Data safety

The answers below are the honest ones for this app, and they are checkable
against the artifact rather than being a promise.

| Question | Answer |
| --- | --- |
| Does your app collect or share any of the required user data types? | **Nothing is collected by the developer**, and nothing is sent to us or to any service we choose. If the learner connects their own Dropbox account, syncing sends their study data to *that* account — see the note below |
| Is any collected data transmitted off the device? | Only by that sync, to the learner's own Dropbox account, and only once they have connected one. Otherwise: nothing |
| Does your app use the microphone? | **Yes**, for tone practice, processed on the device only |
| Is audio recorded by the app sent off the device or stored? | **No** |
| Do you provide a way for users to delete their data? | Yes — uninstalling removes everything; the app also lets the user clear or export it |
| Is data encrypted in transit? | **Yes**, for the one case where data is transmitted: syncing goes to Dropbox over HTTPS. The speech model is a plain file download with nothing attached to it |

<!--
  RECONFIRM THE TWO ANSWERS ABOVE BEFORE SUBMITTING. Play treats "transmitted off
  the device" as collection, and syncing does transmit — to a cloud account the
  learner owns and connects themselves, which is not the same thing as sending
  data to the developer or to a service the developer picked. How the Console wants
  a user-owned cloud account declared is the thing to check, and it should be
  answered from the artifact rather than from this table. What is certain and
  checkable: nothing is transmitted at all until the learner connects an account,
  and nothing is ever sent to the developer.
-->

Play asks separately whether the app requests **microphone** access as a
sensitive permission, and requires a privacy policy for it. The policy at
`docs/privacy-policy.md` covers exactly that, and must be published at a public
URL before the listing is submitted.

Play also lists the permissions the artifact declares, and a signed Android build
declares four: `RECORD_AUDIO`, `INTERNET`, **`USE_BIOMETRIC`** and
**`USE_FINGERPRINT`**. The two biometric ones arrive with the AndroidX library that
shows the prompt, not from code written here; both are *normal* permissions that
Android grants at install with no dialog of their own, and neither gives the app
access to any data. They are used only if the learner switches the fingerprint
prompt on for a connected Dropbox sign-in, and what the app is told is whether the
check succeeded — the fingerprint or face is compared by Android and never reaches
the app. See `docs/privacy-policy.md`, "The fingerprint prompt".

## Content rating

The IARC questionnaire has nothing to declare: no violence, no sexuality, no
profanity, no gambling, no user-generated content sharing, no location, no
personal information. It should come out as **Everyone / 3+**, and the app is
not designed for children specifically (it is not enrolled in Designed for
Families).

## Required assets, and where they are

| Play Console field | File | Status |
| --- | --- | --- |
| App icon (512 × 512 PNG) | `src-tauri/icons/icon.png` | ready (512×512) |
| Feature graphic (1024 × 500) | `store/feature-graphic-1024x500.png` | ready |
| Phone screenshots (min 2) | `store/phone-screenshots/*.png` | 3 ready, 1080×1920 — but all three predate the tone colouring and the current board controls, so replace them first |
| Tablet screenshots | — | not required to publish |

Regenerate the artwork after a UI change with:

```bash
python3 scripts/make-store-assets.py feature-graphic store/feature-graphic-1024x500.png
python3 scripts/make-store-assets.py screenshot raw.png store/phone-screenshots/03-name.png
```

## Release notes (max 500 characters)

The Console asks for these on each track release. This is 428 characters.

```
First release.

Every stroke is graded — shape, placement, ink and order — with the reasons, not
just a score. 7,744 characters in 775 lessons, the HSK 3.0 word list, your own
vocabulary, spaced repetition, stroke-order animation and tone practice.

New: colour by tone. Every character is drawn in the colour of its tone, and its
pinyin with it, syllable by syllable.

Entirely offline. No account, no ads, no in-app purchases.
```

## The build for this upload

Built from the tree at version `0.5.11`, for every ABI the app can run on:

```bash
./scripts/with-build-caches.sh ./scripts/with-cargo-env.sh \
  ./scripts/tauri-cli.sh android build --apk --aab
```

| | |
| --- | --- |
| Bundle | `src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab` |
| Size | 993,207,126 bytes (947 MiB) |
| SHA-256 | `93715ef33407e94cbfc98f08e51a450062aef31662ebc404b3ae2f9c3990690d` |
| `versionCode` / `versionName` | `5011` / `0.5.11` |
| `minSdk` / `targetSdk` | 26 / 36 |
| ABIs | `arm64-v8a`, `armeabi-v7a`, `x86`, `x86_64` |
| Permissions | `INTERNET`, `RECORD_AUDIO`, `USE_BIOMETRIC`, `USE_FINGERPRINT`, `com.hanzitutor.app.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION` |

The permissions and the ABI list were read off the artifact built in the same run
(`aapt2 dump permissions`, `aapt2 dump badging`), not copied from this file. The
permissions are the five the Data safety answers above rest on.

Omitting `--target` is what makes this a four-ABI bundle, and a four-ABI bundle is
what needs the Gradle heap raised to 8 GB in
`src-tauri/gen/android/gradle.properties` — signing it under the template's 2 GB
fails in a way that never mentions memory. HANDOVER has the diagnosis.

The bundle is big because it carries the native library once per ABI plus the
native debug symbols — 562.8 MB of the total, in eight files — that Play keeps for
symbolicating crashes and never sends to a device. **What a device downloads is a
much smaller number**, below.

**Size against Play's limit.** A module's compressed download must not exceed
**200 MB**. A device fetches only its own ABI's libraries, and those are the bulk
of it: 146 MB for arm64, 133 MB for 32-bit ARM, 152.5 MB for x86 and 151 MB for
x86-64, all stored uncompressed so Android can map them rather than unpack them.
The one-ABI arm64 APK built from this same code measured 148.7 MiB (156,070,700
bytes) in total, so every ABI stays inside the limit — x86 is the one with the
least room. Read the figure the Console reports at upload rather than this one;
`store/README.md` has the checks to run first.

## Before submitting

**The app is ready; the developer account is the gate.** The plan had been a
business (organization) account, which is exempt from Google's
12-testers-for-14-days closed-test rule, but Google has not yet accepted the
organization's **D-U-N-S number** and that is at least two weeks away. The
decision is to publish under the existing **personal** account (Developer ID
`8700454726233630990`) in the meantime.

One thing to confirm in the Console before assuming a straight run to production:
**personal accounts created on or after 13 November 2023 must run a closed test
with 12 testers for 14 continuous days before production access is granted.** An
account created before that date is exempt. Which side this account falls on is
read from the Console's own Production access page, not from this file. See
`docs/privacy-policy.md` for the same note.

`docs/privacy-policy.md` is published at **https://hherb.com/hanzi-tutor/privacy**,
and the listing points there. Keep the file and the page in step.

1. Upload `app-universal-release.aab` — **not** the APK; Play only accepts
   bundles for new apps.
2. Enrol in Play App Signing. The key in `~/.android/hanzitutor-upload.jks` is
   then the *upload* key; Google holds the app signing key, and a lost upload
   key can be reset from the Play Console.
3. Check the version code. Play needs it to increase with every upload, and it
   is derived from the app version (`0.2.0` → `2000`) in `tauri.properties`.
   Bump the version in `Cargo.toml` / `tauri.conf.json` before a second upload.
4. Complete the Data safety and content rating questionnaires using the answers
   above. The permission list Play shows is read from the artifact, so check it
   there rather than here: `aapt2 dump permissions` on the `.aab` or the APK should
   list four `uses-permission` lines — `RECORD_AUDIO`, `INTERNET`, `USE_BIOMETRIC`
   and `USE_FINGERPRINT` — and one more the app declares for itself,
   `com.hanzitutor.app.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION`, which AndroidX
   adds to guard a broadcast receiver.
5. Test on a device from the internal testing track before promoting.
