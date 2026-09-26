# App Store listing copy

Everything here is ready to paste into App Store Connect, for both the iOS/iPadOS
app and the Mac app (one listing text serves both; the fields are per platform in
App Store Connect but the copy is the same). Character limits are Apple's, and the
copy is written to stay inside them — the counts beside each block were measured,
not estimated. The Google Play copy is in [`listing.md`](listing.md); the two say
the same things, but Apple's fields are shaped differently.

Reflects version **0.5.11**.

## App Name (max 30 characters)

```
Hanzi Tutor
```

## Subtitle (max 30 characters)

```
Write & say Chinese, offline
```

*(28 characters.)* Alternatives, if you would rather lead with something else:

- `Chinese handwriting & tones` (27)
- `Hanzi strokes & tones, offline` (30)

## Promotional Text (max 170 characters)

This field can be changed at any time without a new review, so it is the place for
news.

```
Write a character, say it aloud, and see exactly what was off — stroke order, shape, tone. Everything runs on your device: no account, no tracking, no subscription.
```

*(164 characters.)*

## Description (max 4000 characters)

*(3948 characters.)*

```
Hanzi Tutor teaches you to read, write and pronounce simplified Chinese — and tells you precisely what you got wrong. It runs entirely on your device: no account, no ads, no analytics, no subscription.

WRITE IT, AND SEE WHAT WAS OFF
Write a character with your finger, Apple Pencil, trackpad or mouse. Every stroke is graded against real stroke geometry: its shape, where you put it, how much ink you used, and whether it came in the right order. You get a score out of 100 and the reasons — which stroke is misshapen, misplaced, backwards, missing or out of order — in colour on the board.

• Trace mode with a faint guide, and Recall mode that shows only the pinyin and meaning
• Stroke-order animation that shows the direction of each stroke, not just the order
• 7,744 characters in 775 frequency-ordered lessons of ten
• Spaced repetition: hard characters come back soon, known ones stop wasting your time

SAY IT, AND SEE YOUR TONE
Hold the button and say the character or word. Your voice's pitch is drawn as a curve over the shape each tone should have — one chart per syllable, so you see which syllable went wrong and how. The neutral tone is scored too. Words are scored as actually spoken, with tone sandhi applied (你好 is níhǎo), and the app says when it has. If there was not enough voice to judge, it says "not sure" instead of guessing.

Tone scoring measures pitch directly. It needs no speech model and no network, and works from the first launch.

DRILL THE TONES THAT TRIP YOU UP
Minimal tone pairs — 妈 mā, 麻 má, 马 mǎ, 骂 mà — drawn from the app's own dictionary, most common first, with filters for the hard contrasts (2 vs 3, 1 vs 4). Hear a set in order, quiz yourself on which tone you heard, then say each one and see your pitch against the target.

ON-DEVICE SPEECH RECOGNITION (OPTIONAL)
A tone can be perfect on the wrong syllable: 四 sì and 是 shì can sound alike in pitch. Install the optional recognition model and the app also tells you which syllables it heard — "heard shi where si was asked for" — beside the tone verdict. Recognition runs on your device; your voice is never uploaded.

COLOUR CHARACTERS BY TONE
Turn on tone colours and every character and its pinyin are drawn in the colour of their tone — on the board, in the word list, in your vocabulary, lessons and radical families. A visual memory aid that ties sound to shape. Off until you switch it on.

AND MORE
• The full HSK 3.0 word list (9,443 words), searchable by character, pinyin with or without tone marks, or English
• Look up any of 9,574 characters: readings, meaning, radical, stroke count, HSK level and every word that uses it
• Radicals taught as families, and each character's components, each writable on its own
• Your own vocabulary lists with groups — words, phrases, whole sentences — exportable to JSON or CSV
• Hear any character or word in your device's own Chinese voice, and graded phrases from bundled recordings

PRIVATE AND OFFLINE BY DESIGN
The whole course is inside the app: characters, stroke data, word list and audio. Nothing to download on first launch, nothing to sign up for. No analytics, advertising, crash reporting or tracking, and no server of ours.

The microphone is on only while you hold the button. What it hears is analysed on the device, never saved and never sent anywhere.

The app reaches the network only for three things, each off until you choose it:
• An optional speech-recognition model (163 MB), downloaded only when you press the button
• An optional speech-synthesis model (about 61 MB) for phrases that have no bundled recording
• Optional syncing between your own devices through your own Dropbox account
Each download shows its address, size and licence before you start, and is checked against a pinned checksum. Sync sends your study data only to your Dropbox, never to us, and can be protected with Face ID or Touch ID.

No account. No ads. No in-app purchases. No streaks to protect.
```

## Keywords (max 100 characters, comma-separated)

Apple already indexes the app name, subtitle and category, so none of those words
is repeated here; spaces after commas waste characters and are left out.

```
mandarin,hanzi,pinyin,HSK,handwriting,stroke order,tones,pronunciation,characters,writing,learn
```

*(95 characters.)* "Hanzi" is in the name and is repeated only because the
name is two words and the singular search is common; drop it for `radicals` (8)
if you prefer.

## What's New in This Version (max 4000 characters)

For the first App Store release, Apple still shows this field; a short line is
enough.

```
First release on the App Store.

• Colour characters by tone: an optional setting draws every character and its pinyin in the colour of its tone, everywhere in the app
• Speech models are now kept in the cache folder, so they stay out of your iCloud backup
• Pronunciation on the Mac now uses the same built-in speech engine as iPhone and iPad
```

## URLs

| Field | Value |
| --- | --- |
| Support URL (required) | https://github.com/hherb/hanzitutor/issues — or a support page on hherb.com, if you would rather not send users to GitHub |
| Marketing URL (optional) | https://hherb.com/hanzi-tutor |
| Privacy Policy URL (required) | https://hherb.com/hanzi-tutor/privacy |
| Contact email | support@hherb.com |

*Check the marketing URL exists before entering it; only the privacy page is known
to be published.*

## Categories

- **Primary:** Education
- **Secondary:** Reference *(the dictionary, word list and character lookup)*

## Copyright

```
© 2026 Horst Herb and contributors
```

## Age rating

Every answer in the questionnaire is **None / No**: no violence, sexual content,
profanity, drugs, gambling, horror, medical information, unrestricted web access,
user-generated content or messaging, and no advertising. The app is not made for
children specifically (do not enrol it in the Kids category). Expected result:
**4+**.

## App Privacy (the "nutrition label")

**Recommended answer: "Data Not Collected".**

- **Developer collection:** none. There is no analytics, advertising, crash
  reporting, telemetry or account, and no developer server to receive anything.
- **Audio:** processed on the device only, never stored or transmitted. Apple's
  definition of *collect* is transmitting data off the device, so on-device
  processing is not collection.
- **Model downloads:** plain file downloads from the publishers
  (`github.com/k2-fsa/sherpa-onnx`, `huggingface.co/csukuangfj`), with no user
  identifier or data attached.
- **Dropbox sync:** sends practice history, vocabulary list and course position to
  the learner's **own** Dropbox account, authorised by them, in an app folder. The
  developer cannot read it. Devices are identified to each other by a random ID
  generated on the device.
- **Tracking:** none. No App Tracking Transparency prompt is needed.

<!--
  RECONFIRM THE DROPBOX ANSWER BEFORE SUBMITTING. Apple counts data as "collected"
  when it is transmitted off the device and stored by the developer *or a
  third-party partner*. Dropbox stores the synced data, but as the learner's own
  storage provider acting on the learner's instruction, not as a partner of the
  developer — the same reasoning the Play listing uses. If you would rather be
  conservative, declare instead:
    • User Content → Other User Content (the vocabulary list)
    • Usage Data → Product Interaction (the practice history)
  both "Not linked to identity", "Not used for tracking", purpose "App
  Functionality". Either way, nothing is sent until a Dropbox account is connected.
-->

## Export compliance

`ITSAppUsesNonExemptEncryption` is already `false` in the iOS `Info.plist`: the only
encryption is standard HTTPS to Dropbox and the model publishers, and the system
Keychain. Answer **No** to proprietary/non-exempt encryption; no documentation is
needed.

## App Review notes

Paste into *App Review Information → Notes*. No demo account is needed: the app has
no sign-in.

```
Hanzi Tutor needs no account and no network to review. Everything the course teaches is bundled.

To see the main features:
1. Pick any character from the lesson list, write it on the board with a finger or Apple Pencil, then tap Check to see the grade.
2. Hold the "Hold to speak" microphone button and say the character; the tone chart appears when you release. The microphone is used only while the button is held, and audio is analysed on the device and discarded.
3. Settings → "Colour by tone" turns on the tone colours.

Optional network features, all off by default:
• Settings → "Recognising what was said" downloads a 163 MB speech-recognition model from github.com/k2-fsa/sherpa-onnx after the user presses the button. It runs on the device.
• Settings → "Speaking phrases with no recording" downloads a ~61 MB speech-synthesis model from huggingface.co, likewise.
• Settings → "Syncing between devices" connects the user's own Dropbox account to sync study data between their devices. Face ID / Touch ID is used only if the user switches on "Ask for my fingerprint before the sign-in is used", to unlock the stored Dropbox token on the device.

No data is sent to the developer. There is no analytics, advertising or tracking.
```

*Check the step labels against the build before pasting — they name the screens as
of 0.5.11.*

## Permission strings (already in the build)

These are what the system prompts show; listed here so the listing and the prompts
agree.

| Key | Text |
| --- | --- |
| `NSMicrophoneUsageDescription` | Hanzi Tutor listens while you hold the button, to check the tone of what you said. The recording is scored on this device and is never saved or sent anywhere. |
| `NSFaceIDUsageDescription` | Hanzi Tutor can use Face ID to protect your Dropbox sign-in, if you turn that on in Settings. |

The Mac build is sandboxed with `com.apple.security.device.audio-input` and
`com.apple.security.network.client` (the latter only for the optional downloads and
Dropbox sync).

## Screenshots

App Store Connect needs its own sizes, which the Play set (1080×1920) is not:

| Device | Required size (portrait) | Status |
| --- | --- | --- |
| iPhone 6.9" | 1320 × 2868 (or 1290 × 2796) | **to do** |
| iPad 13" | 2064 × 2752 (or 2048 × 2732) | **to do** — required, the app runs on iPad |
| Mac | 1280 × 800, 1440 × 900, 2560 × 1600 or 2880 × 1800 | **to do** |

Suggested set, in order, with caption lines for the top of each frame:

1. **A graded attempt** — "Every stroke graded: shape, place, ink and order"
2. **Stroke order flagged** — "See exactly which strokes were out of order"
3. **The tone panel** — "Say it aloud and see your tone against the target"
4. **Tone colours on** (word list or lesson) — "Colour characters by tone to remember them"
5. **Tone pairs** — "Drill the tones that trip you up: mā má mǎ mà"
6. **Recognition result** — "On-device recognition: hear which syllable you really said"
7. **Settings, downloads section** — "Private by design: nothing leaves your device unless you choose"

`docs/screenshots/` and `store/phone-screenshots/` hold the first three from
Android; the iOS set should be taken from the simulator at the sizes above
(`xcrun simctl io booted screenshot shot.png`).

## App Preview video (optional)

15–30 s, portrait. One idea that shows the two halves together: write 马 slowly,
the grade appears; hold the button and say `mǎ`; the pitch curve dips and rises
over the third-tone shape; switch on tone colours and the word list turns brown,
blue, yellow and purple.
