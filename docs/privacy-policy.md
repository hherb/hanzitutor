# Hanzi Tutor — privacy policy

**Last updated: 20 September 2026**

Hanzi Tutor is an offline application for practising simplified Chinese
handwriting. This policy describes what the app does with information. It is
short because the app does almost nothing with it.

## What is collected

**Nothing.** Hanzi Tutor has no accounts, no analytics, no advertising, no crash
reporting and no telemetry. It does not transmit any information off the device,
and there is nothing about you for it to transmit.

There is exactly one network request the app can make, and it only happens if you
ask for it: **Settings → Recognising what was said** offers to download an
optional speech-recognition model, 163 MB, from its publisher
(`github.com/k2-fsa/sherpa-onnx` releases). The screen states the address, the
size and the licence before you press anything, the download is checked against a
published checksum, and it is the only request in the app. Nothing is uploaded,
no identifier is sent, the request is a plain file download, and declining is a
normal state rather than something the app nags about — everything else it does,
including tone practice and pronunciation, works with no model and no network.

Because that download exists, the released Android build declares the
`INTERNET` permission. Earlier releases did not, which made "it works offline"
checkable on the artifact; a build that offers the download but cannot perform it
would be the worse trade.

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

There are none. The app bundles everything it needs — its character dataset,
stroke data, word list, interface font and licence notices — and downloads
nothing.

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
-->
