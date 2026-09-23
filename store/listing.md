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
| Phone screenshots (min 2) | `store/phone-screenshots/*.png` | 2 ready, 1080×1920 |
| Tablet screenshots | — | not required to publish |

Regenerate the artwork after a UI change with:

```bash
python3 scripts/make-store-assets.py feature-graphic store/feature-graphic-1024x500.png
python3 scripts/make-store-assets.py screenshot raw.png store/phone-screenshots/03-name.png
```

## Before submitting

**The current blocker is the developer account, not the app.** The plan is a
business (organization) Play account, which is exempt from Google's
12-testers-for-14-days closed-test rule (that rule applies to personal accounts
created on or after 13 November 2023); Google has not yet accepted the
organization's D-U-N-S number, and that is with an accountant. Nothing here can be
uploaded until it clears. See `docs/privacy-policy.md` for the same note.

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
