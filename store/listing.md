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
account to create. No analytics, advertising or crash reporting is built in, and
nothing about you ever leaves the device.

There is one optional download, and it is the only network request the app can
make: an on-device speech model, about 163 MB, that recognises *which* syllables
you said rather than only how your tone sounded. It is fetched from the settings
screen, only if you press the button there, after being told the address, the
size and the licence. Decline it and nothing changes — tone practice, pronunciation
and the whole course work with no model and no network.

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
- **Contact email:** *<!-- TODO --> the same address as the privacy policy*
- **Privacy policy URL:** *<!-- TODO: publish `docs/privacy-policy.md` and link it here -->*

## Data safety

The answers below are the honest ones for this app, and they are checkable
against the artifact rather than being a promise.

| Question | Answer |
| --- | --- |
| Does your app collect or share any of the required user data types? | **No** |
| Is any collected data transmitted off the device? | **No** — nothing is transmitted |
| Does your app use the microphone? | **Yes**, for tone practice, processed on the device only |
| Is audio recorded by the app sent off the device or stored? | **No** |
| Do you provide a way for users to delete their data? | Yes — uninstalling removes everything; the app also lets the user clear or export it |
| Is data encrypted in transit? | Not applicable — no data is transmitted |

Play asks separately whether the app requests **microphone** access as a
sensitive permission, and requires a privacy policy for it. The policy at
`docs/privacy-policy.md` covers exactly that, and must be published at a public
URL before the listing is submitted.

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

1. Publish `docs/privacy-policy.md` at a public URL and put it in the listing.
2. Replace the placeholder contact address in the privacy policy.
3. Upload `app-universal-release.aab` — **not** the APK; Play only accepts
   bundles for new apps.
4. Enrol in Play App Signing. The key in `~/.android/hanzitutor-upload.jks` is
   then the *upload* key; Google holds the app signing key, and a lost upload
   key can be reset from the Play Console.
5. Check the version code. Play needs it to increase with every upload, and it
   is derived from the app version (`0.2.0` → `2000`) in `tauri.properties`.
   Bump the version in `Cargo.toml` / `tauri.conf.json` before a second upload.
6. Complete the Data safety and content rating questionnaires using the answers
   above.
7. Test on a device from the internal testing track before promoting.
