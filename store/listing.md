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

Nothing at all is sent anywhere unless you choose it. There are three things you
can choose, all of them off by default, and each one is started from the settings
screen after it tells you the address, the size and the licence. The first is an
optional on-device speech model, about 163 MB, that recognises *which* syllables
you said rather than only how your tone sounded. Decline it and nothing changes:
tone practice, pronunciation and the whole course work with no model and no
network. The second speaks a phrase the app has no recording of, about 61 MB of
synthesis files; decline it and only the phrases that have no bundled recording
are lost. The third is syncing between your own devices, which sends your
practice history, vocabulary list and place in the course to a Dropbox account
**you** connect, so that your phone and your laptop agree. Connect nothing and it
stays on this device.

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
| Does your app collect or share any of the required user data types? | **Yes — one type, and only if the learner switches syncing on.** Their study data (practice history, vocabulary, place in the course) leaves the device when *they* connect their own Dropbox account. Nothing is ever sent to the developer, and nothing leaves the device until that happens |
| Is any collected data transmitted off the device? | Yes, in that one case, over HTTPS to the learner's own Dropbox account. On a fresh install with syncing unconnected: nothing at all |
| Does your app use the microphone? | **Yes**, for tone practice, processed on the device only |
| Is audio recorded by the app sent off the device or stored? | **No** |
| Do you provide a way for users to delete their data? | Yes — uninstalling removes everything; the app also lets the user clear or export it |
| Is data encrypted in transit? | **Yes**, for the one case where data is transmitted: syncing goes to Dropbox over HTTPS. The speech model is a plain file download with nothing attached to it |
| If declaring the type, which one? | *App activity → Other user-generated content*. Purpose **App functionality**, declared **optional** (not required), and **not shared with the developer** |

<!--
  RESOLVED — and the one answer in this file that is a person's call, not a
  measurement.

  Play's "collected" is not the everyday word. Its form asks whether data is
  **transmitted off the device at all**, to the developer or to anyone else,
  including through a library the app carries; see "Provide information for Google
  Play's Data safety section",
  support.google.com/googleplay/android-developer/answer/10787469. On that
  definition the Dropbox sync *is* collection, so the "nothing is collected"
  answer that used to be in the first row was wrong in the Console's terms even
  though it is true in the developer's: nothing is sent to us, and the learner's
  own cloud account is not a service we chose for them.

  The answers above are therefore the conservative, defensible set: declare the
  study data as collected, optional, for app functionality, encrypted in transit,
  with the developer receiving nothing. Two consequences to accept knowingly:

    * The listing then shows "Data may be collected" instead of "No data
      collected". That is a worse badge for an app whose whole claim is privacy,
      and it is still the honest one.
    * The privacy policy's "Nothing is collected by us" has to be read as *by the
      developer* — which is what it says — alongside this form rather than instead
      of it. If that ever reads as a contradiction, the policy's wording is what
      should change, not the form.

  The alternative is to answer "no data collected" on the grounds that a
  user-owned cloud account is not the developer. That is a public declaration
  about the app, so it is not this file's call to make. If it is worth certainty,
  ask Play support: what a reviewer can check is the artifact — an `INTERNET`
  permission and a documented sync — and a declaration they read as inaccurate is
  a policy problem rather than a wording one.
-->

Play asks separately whether the app requests **microphone** access as a
sensitive permission, and requires a privacy policy for it. The policy at
`docs/privacy-policy.md` covers exactly that, and is published at a public URL —
see the checklist at the end of this file.

Play also lists the permissions the artifact declares, and a signed Android build
declares four of its own: `RECORD_AUDIO`, `INTERNET`, **`USE_BIOMETRIC`** and
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

## The Console's other questions

Play's **App content** checklist asks these, in this order, before the Data safety
form. Every answer here is checkable against the artifact rather than a promise.

| Console question | Answer |
| --- | --- |
| Privacy policy URL | `https://hherb.com/hanzi-tutor/privacy` — live, and the same text as `docs/privacy-policy.md` |
| Does your app contain ads? | **No.** There is no ad SDK of any kind: the Android build's dependencies are AndroidX (webkit, appcompat, activity-ktx, biometric, lifecycle-process) and Material and nothing else, there is no advertising identifier, and no analytics or crash-reporting SDK is present either |
| Is all functionality available without special access? | **Yes** — nothing is behind a sign-in of ours, and there are no credentials to give a reviewer. The course, the board, grading, tone practice and pronunciation all work with no account. The one thing a reviewer cannot exercise is syncing, which needs *their own* Dropbox account; it is off by default and the app is complete without it |
| Target audience and content | **13+ / adults.** Not designed for children, not enrolled in Designed for Families, no child-directed content |
| Content rating questionnaire | IARC, every answer "no" → **Everyone / 3+** |
| Is your app a government app? | No |
| Does it offer financial features? | No |
| Is it a health app? | No |
| Is it a news app? | No |
| Does it need a data-deletion URL? | No. The app has no accounts and no server of the developer's, so there is no data held by us to delete; the Data safety row above says what deleting looks like |

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
| Size | 993,211,420 bytes (947 MiB) |
| SHA-256 | `426fe1affa05063f174392020c49ab4388517cd7698a3a82af485dd3e226df57` |
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

## Submitting it

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

Then, in this order — the Console's own, because its Data safety form builds on the
answers given before it:

1. **Create the app** in the Console, named `Hanzi Tutor`. Paste the short and full
   descriptions from this file, and the release notes when a track release asks
   for them.
2. **App content**, in the Console's order: privacy policy URL, the ads
   declaration, app access, target audience and content — then the Data safety
   form and the content-rating questionnaire, whose answers are the two sections
   above.
3. **Upload `app-universal-release.aab`** — **not** the APK; Play accepts only a
   bundle for a new app. Enrol in **Play App Signing** at the same time: the key
   in `~/.android/hanzitutor-upload.jks` becomes the *upload* key, Google holds
   the app signing key, and a lost upload key can be reset from the Console.
4. **Check the permission list the Console shows against the artifact.** It is read
   from the bundle rather than from this file, and it should name four
   `uses-permission` lines — `RECORD_AUDIO`, `INTERNET`, `USE_BIOMETRIC` and
   `USE_FINGERPRINT` — plus
   `com.hanzitutor.app.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION`, which AndroidX
   adds to guard a broadcast receiver.
5. **Check the version code before every later upload.** Play requires it to
   increase, and `tauri.properties` derives it from the app version (`0.5.11` →
   `5011`). Bump the version in `Cargo.toml`, `package.json` and
   `tauri.conf.json` together — a test fails if any two disagree — and look at
   `NOTES` in `src/lib/startupPages.ts` in the same breath.
6. **Test from the internal testing track** before promoting. A closed-test tester
   installs the app **from Play**, through the opt-in link the Console gives them:
   they need no APK, and no separate build is involved, so the same bundle that
   goes to production is the one they test.
