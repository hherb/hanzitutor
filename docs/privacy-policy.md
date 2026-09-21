# Hanzi Tutor — privacy policy

**Last updated: 21 September 2026**

Hanzi Tutor is an offline application for practising simplified Chinese
handwriting. This policy describes what the app does with information. It is
short because the app does almost nothing with it.

## What is collected

**Nothing is collected by us.** Hanzi Tutor has no accounts with the developer, no
analytics, no advertising, no crash reporting and no telemetry. There is no server
of the developer's for it to send anything to.

Two things can reach the network, and **neither happens unless you choose it**.
Everything else the app does — the course, grading, tone practice, pronunciation —
works with no network and no model.

**An optional speech-recognition model.** **Settings → Recognising what was said**
offers to download one, 163 MB, from its publisher
(`github.com/k2-fsa/sherpa-onnx` releases). The screen states the address, the size
and the licence before you press anything, the download is checked against a
published checksum, nothing is uploaded, and declining is a normal state rather
than something the app nags about. No identifier of yours is attached to that
request.

**Syncing between your own devices.** This is off until you connect a Dropbox
account of your own from the settings screen. If you do, the app sends your
practice history (which characters you attempted, your scores, when each is due),
your vocabulary list and your place in the course to **that account**, in a folder
only this app can see, and reads back what your other devices have put there. Each
device is named to the others by a random identifier generated on the device —
not your name, not your email, not anything about your Dropbox account. Once you
have connected, it syncs when the app starts, when you return to it, and when you
press *Sync now*; disconnecting stops it and revokes the app's access.

Because both exist, the released Android build declares the `INTERNET` permission.
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
another device. Uninstalling the app removes all of it.

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

Questions about this policy can be sent to **<!-- TODO: replace before publishing -->privacy@example.invalid**.

<!--
  BEFORE PUBLISHING: the contact address above is a placeholder and must be
  replaced with a mailbox that is actually monitored. Play requires a privacy
  policy URL to be reachable and to name a real contact, and this file is
  intended to be published verbatim at a public URL (for example a GitHub Pages
  page or the project's own site) and that URL given in the Play Console listing.

  Check the claims above still hold before publishing:
    * `grep -rn "ureq\|reqwest" src-tauri/src` finds exactly one module,
      `src-tauri/src/asr.rs`, and every request in it is gated behind
      `Asr::install`, which only runs when the settings button is pressed. If a
      second module appears, this policy needs another paragraph.
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
      The microphone and the model download are the first two; the biometric pair
      arrives with `androidx.biometric` through manifest merging, so it reappears
      on any rebuild and cannot be removed by editing this project's manifest. If
      the fingerprint switch is ever dropped, the library goes with it and the
      section above goes too.
-->
