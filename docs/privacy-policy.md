# Hanzi Tutor — privacy policy

**Last updated: 23 September 2026**

Hanzi Tutor is an offline application for practising simplified Chinese
handwriting. This policy describes what the app does with information. It is
short because the app does almost nothing with it.

## What is collected

**Nothing is collected by us.** Hanzi Tutor has no accounts with the developer, no
analytics, no advertising, no crash reporting and no telemetry. There is no server
of the developer's for it to send anything to.

Two of the three things below are model downloads and the third is syncing, and
**none of them happens unless you choose it**. Everything else the app does — the
course, grading, tone practice, pronunciation — works with no network and no model.

**An optional speech-recognition model.** **Settings → Recognising what was said**
offers to download one, 163 MB, from its publisher
(`github.com/k2-fsa/sherpa-onnx` releases). The screen states the address, the size
and the licence before you press anything, the download is checked against a
published checksum, nothing is uploaded, and declining is a normal state rather
than something the app nags about. No identifier of yours is attached to that
request.

**An optional speech-synthesis model.** **Settings → Speaking phrases with no
recording** offers to download one, about 61 MB, from its publisher
(`huggingface.co/csukuangfj/vits-melo-tts-zh_en`). It follows exactly the same
rules: the address, the size and the licence are stated before you press anything,
every file is checked against a pinned checksum, an interrupted download leaves the
previous state untouched, and nothing is uploaded. The bundled phrase recordings and
the system's own voice need no download, so declining this changes only the phrases
that have no bundled recording.

**Syncing between your own devices.** This is off until you connect a Dropbox
account of your own from the settings screen. If you do, the app sends your
practice history (which characters you attempted, your scores, when each is due),
your vocabulary list and your place in the course to **that account**, in a folder
only this app can see, and reads back what your other devices have put there. Each
device is named to the others by a random identifier generated on the device —
not your name, not your email, not anything about your Dropbox account. Once you
have connected, it syncs when the app starts, when you return to it, and when you
press *Sync now*; disconnecting stops it and revokes the app's access.

Because these exist, the released Android build declares the `INTERNET` permission.
Earlier releases did not, which made "it works offline" checkable on the artifact;
a build that offers these and cannot perform them would be the worse trade.

## What is stored, and where

Everything the app remembers is kept on the device, inside the app's own private
storage, and is readable only by the app:

- **Your practice history** — which characters you have attempted, your scores
  and when each one is next due for review.
- **Your vocabulary list** — anything you have added, with its groups.
- **Your settings** — for example the voice used for pronunciation and the input
  mode of the practice board.

You can export the vocabulary list and your progress to a file of your choosing
from inside the app, and importing them back is how you move your study data to
another device. Whether uninstalling the app also removes its data is controlled by
the operating system.

## The fingerprint prompt

The released Android build declares `USE_BIOMETRIC` and `USE_FINGERPRINT` in its
manifest. Both arrive with the AndroidX library that shows the prompt rather than
from code written here, both are *normal* permissions — Android grants them when
the app is installed, with no dialog — and neither gives the app access to any
data. They are what lets an app **ask** the system to check a fingerprint or a
face.

They are used for exactly one thing, and only if you ask for it. If you connect a
Dropbox account for syncing, the settings screen offers **Ask for my fingerprint
before the sign-in is used**. Switch it on and the sign-in is stored in Android's
keystore under a key that will not decrypt until Android has checked that it is
you. What the app is given back is a **yes or a no**: the fingerprint or face is
compared by Android, inside the device's secure hardware, and is never handed to
this app, never stored by it, and never leaves the device. Nor does it authenticate
anything to Dropbox — it unlocks a key that is already on the device.

If you never connect Dropbox, or leave that switch off, no prompt ever appears.

## The microphone

Tone practice listens to you pronounce a character, and that is the only reason
the app ever asks for microphone access.

- The microphone is open **only while you are holding the button**. The app never
  listens in the background.
- What it records is analysed **on the device**, by code that runs inside the app.
  The audio is not written to storage, not added to your practice history, and
  not transmitted anywhere. It is discarded when the attempt has been scored.
- You can decline the microphone permission and the rest of the app is
  unaffected; only tone practice becomes unavailable.

## Pronunciation

Speech is produced by the speech synthesiser that is part of your device's
operating system. The text to be spoken is handed to that system component on
the device. Some system voices are marked by the operating system as requiring a
network connection; Hanzi Tutor prefers a voice that does not, so pronunciation
works offline.

## Children

The app collects no information from anyone, including children.

## Third parties

Nothing is sent to the developer, to an advertiser, to an analytics service or to
anybody the developer has chosen. The only two services the app can talk to are
ones **you** pick: the publisher of the optional speech model, and, if you switch
it on, your own Dropbox account. Everything else is bundled — the character
dataset, stroke data, word list, interface font and licence notices all ship with
the app.

## Changes

If this policy changes, the date at the top changes with it, and the new version
is published at the same address.

## Contact

Questions about this policy can be sent to **support@hherb.com**.

<!--
  This policy is published at **https://hherb.com/hanzi-tutor/privacy**, which is
  the URL the Play Console listing points at. The contact address is real and
  monitored (set 23 September 2026, replacing a placeholder). Keep this file and
  the published page in step — the page is what Play reads, and the project's rule
  is that the listing, the README and `LICENSES.md` name every way the app can
  reach the network. On 23 September 2026 this file was brought back in step with
  the page: it had said "two things" and omitted the synthesis download.

  What still blocks submission is **not** this file: it is the Play developer
  account. The plan is a **business (organization) account**, which is exempt from
  the 12-testers-for-14-days closed-test rule that applies to personal accounts
  created on or after 13 November 2023; Google has not yet accepted the
  organization's D-U-N-S number and that is with an accountant. Until it clears
  there is nothing to upload to, so the listing waits.

  Check the claims above still hold before republishing:
    * `grep -rln "ureq\|reqwest" src-tauri/src` finds **two** modules that make
      requests — `asr.rs` (the recognition model) and `say.rs` (the synthesis
      model) — each gated behind its own install, which runs only when the settings
      button is pressed. A third module needs another paragraph above.
    * `src-tauri/gen/android/app/src/main/AndroidManifest.xml` declares
      `INTERNET` for that download and nothing else uses it, so the Play Console's
      Data safety answers stay "no data collected" — a plain file download with no
      identifier attached is not collection, but say so explicitly in the
      declaration rather than leaving it implicit.
    * The microphone is opened in `src-tauri/src/capture.rs` only between
      `Recorder::start` and `Recorder::stop`, and no recording is written to disk
      or sent anywhere.
    * `aapt2 dump permissions` on a signed Android artifact lists exactly four
      `uses-permission` lines — `RECORD_AUDIO`, `INTERNET`, `USE_BIOMETRIC` and
      `USE_FINGERPRINT` — plus one the app declares for itself,
      `com.hanzitutor.app.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION`, which AndroidX
      adds and which guards a broadcast receiver rather than collecting anything.
      The microphone and the two model downloads are the first three; the biometric pair
      arrives with `androidx.biometric` through manifest merging, so it reappears
      on any rebuild and cannot be removed by editing this project's manifest. If
      the fingerprint switch is ever dropped, the library goes with it and the
      section above goes too.
-->
