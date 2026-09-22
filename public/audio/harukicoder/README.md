# Graded readers — text and audio, and what they are under

This directory holds the app's copy of the **HSK 3.0 graded readers**: a
`manifest.json` with 1,184 sentences (their pinyin and English glosses) and two
MeloTTS clips per sentence, `<id>.mp3` and `<id>_slow.mp3`.

## Upstream, and the attribution the licence requires

| | |
| --- | --- |
| Work | **"HSK 3.0 Graded Reader Corpus"** (102 texts, 1,185 sentences, word-aligned with pinyin and gloss, six difficulty shelves, version 1.1, 2026) |
| Creator | **Alvaro Serrano**, as the reading corpus for [pinyora.com](https://pinyora.com), a free Mandarin reading site |
| Source | <https://huggingface.co/datasets/harukicoder/hsk30-graded-readers> |
| Licence | **Creative Commons Attribution 4.0 International (CC BY 4.0)** — <https://creativecommons.org/licenses/by/4.0/> |

Attribution, as the licence requires:

> **Alvaro Serrano / pinyora.com — "HSK 3.0 Graded Reader Corpus", licensed CC BY 4.0.**

CC BY 4.0 requires attribution and nothing further — it has **no share-alike
condition**, unlike the `no7z` sentences next door. The licence's full legal text
ships in this repository as [`licences/CC-BY-4.0.txt`](../../../licences/CC-BY-4.0.txt),
and the notice the app itself carries is
[`licences/HARUKICODER-hsk30-graded-readers.txt`](../../../licences/HARUKICODER-hsk30-graded-readers.txt).

## Changes made here

CC BY 4.0 also asks that modifications be indicated. They are:

* **Extracted a subset** of the corpus into `manifest.json`.
* **Generated the audio** — the upstream dataset publishes **text only**, so every
  clip here is this project's own output, synthesised with **MeloTTS**
  (`vits-melo-tts-zh_en`) through sherpa-onnx. No audio came from upstream.
* Wrote the sentences, pinyin and glosses into `manifest.json` unchanged.

Nothing was re-levelled, retranslated or otherwise edited. The shelf labels are
the upstream ones.

## Two cautions the upstream datasheet states, kept here

* **The passages were drafted with the assistance of large language models**, then
  reviewed, edited, re-levelled and in several cases rewritten by the author. They
  are pedagogical material written to a level target, **not a sample of naturally
  occurring Chinese.**
* **The shelf is a band, not a point claim:** measured against the HSK 3.0
  character standard, the author hit the shelf target 61.8% of the time,
  overshooting on the easy shelves and undershooting on the hard ones. The app
  presents the shelf as a rough guide and does not label a reader with an exact
  level.

## The audio is a first pass, and is labelled as one

MeloTTS is the app's *on-device* voice for a phrase with no recording. As bundled
course audio its accuracy was measured below the `no7z` recordings — on the same
208 HSK-1 phrases, through one recogniser and one judge, MeloTTS failed 57 where
the published recordings failed 15. The readers' voice is therefore still an open
decision (ROADMAP M14), and the app says so on the Phrases screen rather than
presenting this set as finished. The measurements are in
`docs/research/MELOTTS_PRONUNCIATION_ACCURACY.md`.

MeloTTS's own notice is [`licences/MIT-MeloTTS.txt`](../../../licences/MIT-MeloTTS.txt).
