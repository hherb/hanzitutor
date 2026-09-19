# Neural ASR and TTS for Hanzi Tutor — viability research

**Status:** research only. No code in this repository was changed.
**Date:** 2026-09-19.
**Question asked:** what are the smallest ASR and TTS models, and the Rust crates
or Rust-reachable libraries around them, that would replace the current
`say`-based pronunciation and add speech recognition, while still performing
acceptably on CPU on a machine with 8 GB of RAM?

Every size in this document is either a measured HTTP `Content-Length`, a
measured file size after unpacking, or a figure quoted from upstream
documentation — each is marked. Throughput figures that are not measured are
labelled as estimates, because no benchmark was run on target hardware as part
of this work.

---

## 1. Summary

**One crate does the whole job, and it is not the obvious one.** The official
`sherpa-onnx` Rust crate (v1.13.8, Apache-2.0, published 2026-09-11) covers ASR,
TTS, VAD, keyword spotting and punctuation behind one API, on Windows, macOS,
Linux, Android and iOS. The widely-linked `sherpa-rs` crate is a third-party
binding that upstream now deprecates in favour of this one, and its last release
was 2025-10-05.

**For TTS the smallest model is a trap.** `vits-icefall-zh-aishell3` is the
fastest and smallest Chinese voice in the catalogue by a wide margin — 30 MB,
and 4–10× faster than every other Chinese model on the same hardware. It
synthesises at **8 kHz**, which I verified directly from the ONNX metadata. An
8 kHz signal has a 4 kHz ceiling, and that is precisely the band that separates
`s` / `sh` / `x` and `c` / `ch` / `q`. For a general reading app it would be
fine. For an app whose stated purpose includes teaching pronunciation, the
cheapest model is the one that removes the distinction learners most need to
hear. This is the single most important finding here.

**For ASR, Whisper is not a candidate at these sizes.** Published Mandarin CER
for `whisper-tiny` is ~67% and for `whisper-base` ~51%. `SenseVoice-Small` and
`Paraformer` are in the 8–10% range on the same class of benchmark. The Rust
ecosystem's best-known speech crate (`whisper-rs`, 670k recent downloads) is the
wrong tool for Chinese specifically.

**And ASR will not, on its own, grade pronunciation.** A recogniser's language
model actively repairs a learner's errors before you see the output — it is
built to. Tone in particular has to be assessed from the F0 contour, not from
recognised text. Section 6 sets out a design that uses ASR for *segmentation*
and a pure-Rust pitch tracker for *scoring*.

**Recommended shape**, in the order I would build it:

| Stage | What | Cost |
| --- | --- | --- |
| 0 | Finish M6 with system TTS (SAPI / speech-dispatcher), as already planned | no models, no new deps |
| 1 | Ship a **pre-rendered audio pack** for the bundled HSK curriculum | ~15–25 MB, zero inference, preserves "no model weights" |
| 2 | Optional neural TTS download for arbitrary text (personal vocabulary) | ~140 MB, Apache-2.0 |
| 3 | Optional ASR download + F0 tone scoring | ~155 MB + ~25 MB native |

Stage 1 is the one I would argue hardest for, and it is explained in §4.4.

---

## 2. What the app does today, and what follows from it

`src-tauri/src/speech.rs` shells out to `/usr/bin/say` with a Chinese voice
(`Tingting`, `Ting-Ting` or `Meijia`), passing the **character** rather than its
pinyin so the system lexicon resolves the reading. Other platforms return an
honest "not implemented" error. ROADMAP M6 plans `spd-say`/`espeak-ng` on Linux
and `System.Speech.Synthesis` on Windows.

Three properties of the project constrain everything below.

1. **The README promises no model downloads and no network at runtime**
   ("all offline, with no model downloads and no network access at runtime";
   "no network, no model weights"). Any neural model breaks that promise unless
   it is bundled — and no worthwhile Chinese model is small enough to bundle
   comfortably. This is a product decision, not a technical one, and it should
   be made deliberately rather than by accident.
2. **`LICENSES.md` tracks provenance carefully**, down to which upstream notice
   files must travel with a redistribution. Model weights carry licences too,
   and several of the best-performing Chinese models carry terms that do not
   survive contact with AGPL-3.0. See §9.
3. **The vocabulary is a closed set.** The app ships an HSK 3.0 word list and
   per-character pinyin, already compiled into `hanzi.bin.gz`. It does not need
   open-vocabulary synthesis for its own curriculum — only for the user's
   personal vocabulary list. That asymmetry is worth exploiting (§4.4).

There is also an existing asset that matters for §6: the app **already holds
per-character pinyin with tone marks**. Grapheme-to-phoneme, which is the
awkward part of Chinese speech work, is largely solved inside this codebase
already.

---

## 3. The library layer

### 3.1 `sherpa-onnx` — the recommended base

| | |
| --- | --- |
| Crate | `sherpa-onnx` 1.13.8, plus `sherpa-onnx-sys` |
| Licence | Apache-2.0 (both) |
| Published | 2026-09-11; first release 2026-02-23 |
| Downloads | 414k total, 366k in the last 90 days |
| Upstream | `k2-fsa/sherpa-onnx` — the toolkit's own maintainers |
| Platforms | Windows, macOS, Linux, Android, iOS |

It wraps the sherpa-onnx C API with owned Rust types. The Rust examples
directory carries ~50 runnable examples, including Chinese-specific ones:
Matcha TTS (zh), Kokoro TTS (zh+en), streaming Zipformer (zh+en), SenseVoice,
Paraformer, Silero VAD, keyword spotting, and punctuation.

**Build behaviour, which matters for a Tauri app.** `sherpa-onnx-sys` has
exactly four build-dependencies: `ureq`, `tar`, `bzip2`, `zip`. That tells you
what the build script does — **it downloads a prebuilt native library archive
from GitHub releases at build time** unless `SHERPA_ONNX_LIB_DIR` points at one
you already have. Static linking is the default; `--features shared` switches to
dynamic, in which case the build script copies the runtime libraries next to the
binary and sets rpath on Linux/macOS.

For this project that has two consequences:

- CI and offline builds need `SHERPA_ONNX_LIB_DIR` set against a vendored copy,
  or `cargo build` acquires a binary over the network. For a repository that
  fetches its data through a reviewed `scripts/fetch-data.sh`, pulling an
  unpinned binary during compilation is a meaningful change in posture.
- Measured archive sizes: `linux-x64-shared` **26.9 MB** compressed,
  `osx-universal2-shared` **41.5 MB** compressed. The static archives are far
  larger (`linux-x64-static` 404 MB, `osx-universal2-static` 512 MB) because
  they carry unstripped `.a` files; the *linked* binary is much smaller than
  that, but I did not link one to measure it. Budget roughly **25–45 MB of
  native code per platform** on top of the current binary.

### 3.2 Alternatives, and why they lose

| Crate | Version / updated | Verdict for this project |
| --- | --- | --- |
| `sherpa-rs` 0.6.8 | 2025-10-05 | Third-party binding to the same toolkit; upstream deprecates it in favour of the official crate. Do not start here. |
| `whisper-rs` 0.16.0 | 2026-03-12 | Mature, popular (670k recent downloads), and the wrong model family for Mandarin at small sizes (§5.1). |
| `ort` 2.0.0-rc.13 | 2026-07-28 | ONNX Runtime bindings, MIT/Apache-2.0, 7.1M recent downloads. The right choice **if** you run one model and do your own front-end — which is more plausible here than usual, because you already have the pinyin lexicon. Still a release candidate. |
| `piper-rs` 0.2.0 | 2026-05-21 | Piper TTS via `ort`. Piper's Chinese coverage is one mediocre voice; its strength is European languages. |
| `candle-transformers` 0.11.0 | 2026-06-26 | Pure Rust, no C++ toolchain, no build-time download — genuinely attractive for packaging. But you would be implementing the Chinese TTS front-end and model plumbing yourself, and the Whisper example inherits Whisper's Mandarin weakness. |
| `vosk` 0.3.1 | 2024-10-27 | Stale, and requires a separately-shipped native library. |
| `tts` (tts-rs) 0.26.3 | 2024-07-06 | Cross-platform *system* TTS — SAPI, AVSpeechSynthesizer, speech-dispatcher, Web Speech. Not neural, but it is the cheapest possible way to complete M6, and worth considering on its own merits. Last release 2024. |

### 3.3 Supporting crates

| Crate | Version | Licence | Use |
| --- | --- | --- | --- |
| `cpal` | 0.18.2 (2026-08-16) | Apache-2.0 | Microphone capture, all three desktop platforms |
| `hound` | 3.5.1 | Apache-2.0 | WAV read/write for fixtures and debugging |
| `rubato` | 5.0.0 | MIT/Apache-2.0 | Resample capture to the 16 kHz the ASR models expect |
| `earshot` | 1.2.2 (2026-08-19) | MIT/Apache-2.0 | Pure-Rust WebRTC-style VAD — **no model file at all** |
| `pitch-detection` | 0.3.0 (2022) | MIT/Apache-2.0 | McLeod/YIN F0 estimation for tone scoring. Unmaintained but small and self-contained; the algorithms do not rot. |
| `pinyin` | 0.11.0 (2026-01-01) | MIT | Hanzi→pinyin, if you ever need it beyond the bundled data |
| `jieba-rs` | 0.11.0 (2026-09-16) | MIT | Word segmentation, if multi-character TTS input needs it |
| `aubio-rs` | 0.2.0 (2021) | **GPL-3.0** | Alternative pitch tracker. GPL-3.0 is compatible with AGPL-3.0, but it is a stale C binding; prefer `pitch-detection`. |

---

## 4. Text to speech

### 4.1 The candidates, measured

Download sizes are measured `Content-Length` values from the sherpa-onnx
release assets. RTF figures are upstream's, on a Raspberry Pi 4 Model B — useful
for *relative* ranking, pessimistic in absolute terms for a desktop.

| Model | Download | Sample rate | RTF 1 thread | RTF 4 threads | Licence |
| --- | ---: | ---: | ---: | ---: | --- |
| `vits-icefall-zh-aishell3` | **30.1 MB** | **8 kHz** ⚠ | **0.365** | **0.156** | AISHELL-3 (permissive) |
| `matcha-icefall-zh-baker` + `hifigan_v2` | 72.0 + 3.6 MB | 22.05 kHz | 0.892 | 0.391 | **non-commercial** ⚠ |
| `sherpa-onnx-vits-zh-ll` | 113.3 MB | 16 kHz | 4.275 | 1.593 | not stated |
| `vits-zh-hf-fanchen-C` | ~116 MB | 16 kHz | 4.306 | 1.600 | not stated |
| `vits-zh-hf-theresa` / `-eula` | ~117 MB | 22.05 kHz | 6.03 | 2.21 | not stated |
| `vits-melo-tts-zh_en` | 159.3 MB | 44.1 kHz | 6.727 | 2.518 | MeloTTS (MIT) |
| `kokoro-int8-multi-lang-v1_1` | **140.2 MB** | 24 kHz | — | — | Apache-2.0 |
| `kokoro-multi-lang-v1_1` (fp32) | 347.9 MB | 24 kHz | ~7.6 * | ~3.2 * | Apache-2.0 |

\* upstream RTF is published for `kokoro-multi-lang-v1_0` fp32; the int8 v1_1
build should be materially faster and I did not find a published figure for it.

Sample rates for the two models I could check directly are verified; the rest
are from the upstream model table, which in one place contradicts itself (it
lists `vits-model-aishell3` at 116 MB / 8 kHz in a summary table and 29–30 MB in
both the detail section and the RTF table). Treat the table's sizes as
indicative and the measured downloads above as authoritative.

### 4.2 The 8 kHz problem, verified

I downloaded `vits-icefall-zh-aishell3.tar.bz2` and read the ONNX metadata
directly:

```
model_type = vits
model_author = k2-fsa
comment = icefall
language = Chinese
n_speakers = 174
sample_rate = 8000
```

8 kHz sampling gives a 4 kHz Nyquist limit. Mandarin's sibilant series —
`s` /s/, `sh` /ʂ/, `x` /ɕ/, and the affricates `c`, `ch`, `q` — is distinguished
largely by spectral energy *above* 4 kHz; /s/ in particular carries most of its
energy in the 5–8 kHz band. At 8 kHz they collapse toward one another. A learner
using this voice as their pronunciation reference would be trained on a signal
from which the contrast has been removed.

This model remains excellent for anything that is not pronunciation teaching,
and it is by far the best fit for a low-power device. It should not be the
reference voice in a tutor.

A secondary note: unpacked, this model directory is 213 MB, but 180 MB of that
is `rule.far`, an OpenFst rule archive that the documented invocation does not
use (it passes `phone.fst`, `date.fst`, `number.fst` instead). The files
actually needed total **~33 MB**. The same pattern — a large unused artefact in
the tarball — is worth checking for any model you ship.

### 4.3 Recommendation

**`kokoro-int8-multi-lang-v1_1`**, 140 MB, 24 kHz, Apache-2.0.

It is the only candidate that is simultaneously: high enough bandwidth to teach
sibilants, unambiguously free-licensed, actively maintained, and shipped with
its own Chinese lexicon (`lexicon-zh.txt`, 2.3 MB) so that **no Python G2P
dependency is needed** — which is the thing that usually makes Kokoro awkward to
embed.

The reservation is speed. Upstream's Pi 4 figure for the fp32 v1_0 build is
RTF 7.6 single-threaded. A modern desktop core is roughly an order of magnitude
faster on this kind of workload, and int8 buys more on top, so I would *estimate*
RTF ~0.1–0.3 on four threads on a typical laptop — a few hundred milliseconds
for a single character or short word. **That is an estimate and should be
measured before anyone commits to it.** On an older or fanless machine it could
be several times worse.

`matcha-icefall-zh-baker` is the better engineering answer — half the size,
several times faster, 22.05 kHz — and is blocked on licensing, not on merit. Its
training data (Databaker's free dataset) is **non-commercial use only**, which
cannot be reconciled with AGPL-3.0 redistribution (§9). If that restriction were
ever lifted or an equivalently-trained model appeared under free terms, it would
displace Kokoro as the recommendation.

**Ruled out:** Supertonic 3 looked ideal on paper — 99M parameters, 31
languages, designed for on-device use, and already wired into sherpa-onnx. Its
language list does **not include Chinese**. The project is also archived
(no further updates or security patches), and its weights are OpenRAIL-M, which
carries behavioural use restrictions and is not a free licence. Not a candidate.

### 4.4 The option that avoids the model entirely

The bundled curriculum is a **closed set**: roughly 3,000 characters and a few
thousand HSK words, all known at build time. Nothing about them requires
synthesis at runtime.

Rendering them once and shipping the audio costs, very roughly: 8,000 items ×
~1.2 s × Opus mono at ~20 kbps ≈ **20 MB**, with a realistic range of 15–25 MB
depending on bitrate and how many items you cover. That is *one seventh* of the
Kokoro download, needs no inference, no native library, no build-time binary
fetch, adds no CPU or RAM cost, works identically on every platform, and keeps
the README's "no model weights at runtime" promise intact for everything the app
itself teaches.

It also lets you use a *better* voice than you could ever ship a model for,
since rendering happens once on a developer machine — including, if licensing
allows it, `vits-melo-tts-zh_en` at 44.1 kHz, whose RTF of 6.7 is irrelevant
offline.

What it does not cover is the **personal vocabulary list**, which is
user-authored and open-ended. That is the actual, narrow case for an embedded
TTS model — and it is an optional download, not a bundled one.

I would treat the pre-rendered pack as the primary plan and the neural model as
the fallback for user-entered text, rather than the other way round. The
licensing work for the pack is the same either way: whatever voice renders it,
its terms must permit redistributing the audio.

---

## 5. Speech recognition

### 5.1 Whisper is the wrong family here

Published Mandarin character error rates:

| Model | Mandarin CER |
| --- | ---: |
| `whisper-tiny` | ~67.6% |
| `whisper-base` | ~51.5% |
| `whisper-large-v3` | ~20.0% |
| `Paraformer` | ~10.2% |
| `SenseVoice-Small` | ~7.8% |

The small Whisper models are not usable for Chinese at all, and even
`large-v3` — far outside the RAM budget — is beaten roughly 2.5× by a 234M
Chinese-specialised model. SenseVoice and Paraformer are also
*non-autoregressive*: one forward pass yields the whole transcript, where
Whisper decodes token by token, which is most of the CPU speed difference.

This matters because `whisper-rs` is the crate a Rust developer finds first. For
this app it is a dead end.

### 5.2 The candidates, measured

| Model | Download | Unpacked (int8) | Notes |
| --- | ---: | ---: | --- |
| `streaming-zipformer-zh-14M-2023-02-23` | 74.0 MB | ~24.5 MB | Tiny: 21 MB encoder + 1.8 + 1.7. Streaming. Weakest accuracy. |
| `paraformer-zh-small-2024-03-09` | 74.3 MB | 79 MB | zh+en, non-streaming. Good middle. |
| `sense-voice-zh-en-ja-ko-yue-int8-2024-07-17` | 155.5 MB | 228 MB | Best Chinese accuracy of the group; also ja/ko/yue. |
| `streaming-zipformer-zh-int8-2025-06-30` | 126.5 MB | ~160 MB | Newest streaming Chinese model. |
| `streaming-zipformer-small-bilingual-zh-en-2023-02-16` | 437.0 MB | ~47.5 MB | Tarball carries fp32 and int8; only int8 is needed. |
| `silero_vad.onnx` | 0.6 MB | 0.6 MB | Endpointing. Or use `earshot` and ship nothing. |

Upstream's SenseVoice RTF on an RK3588 (ARM, int8) is 0.049–0.436 depending on
core and thread count — comfortably real-time on ARM, and a desktop x86 core
will be well under that.

### 5.3 Recommendation

**`sherpa-onnx-sense-voice-...-int8-2024-07-17`**, 155 MB download / 228 MB on
disk, paired with `earshot` or Silero VAD for endpointing.

The utterances this app needs to recognise are *short* — one character, one
word, occasionally one sentence. That is the regime where a non-autoregressive
model with a strong Chinese prior is at its best, and where a 234M model's RAM
cost (call it ~300–400 MB resident with ONNX Runtime arenas, for a few hundred
milliseconds at a time) is affordable even on an 8 GB machine, because nothing
else in this app is memory-hungry.

If 228 MB proves unacceptable, `paraformer-zh-small` at 79 MB is the fallback,
at roughly 30% higher error rate. The 14M zipformer is small enough to bundle,
but I would not build pronunciation feedback on it.

---

## 6. The part that is harder than choosing a model

Picking an ASR model is the easy half. Turning its output into useful
pronunciation feedback is where this kind of feature usually fails, for three
reasons.

### 6.1 The recogniser repairs the learner's mistakes

A Chinese ASR model carries a strong language-model prior. Say `shì` when the
target was `sì` and a good recogniser will often still emit the character you
were aiming for, because that is what the context makes likely. **It is designed
to be robust to exactly the errors you are trying to detect.** Naively comparing
recognised text to the target will systematically under-report errors, and will
do so most for the learners who need feedback most.

Note that sherpa-onnx's hotwords / contextual biasing API
(`hotwords_file`, `hotwords_score`, `SherpaOnnxCreateOnlineStreamWithHotwords`)
makes this *worse* if pointed at the expected answer — it biases decoding toward
the target. It is the right tool for recognising rare vocabulary and the wrong
tool for assessment. One legitimate use: decode twice, with and without bias, and
treat a large divergence as evidence the learner did not produce the target.

### 6.2 Tone is not in the text

Mandarin tone is an F0 contour. The right way to score it is to measure F0
directly, normalise it per-speaker, and compare its shape to the expected tone —
not to ask a recogniser which character it heard.

This is cheap and needs no model:

1. Capture with `cpal`, resample to 16 kHz with `rubato`.
2. Endpoint with `earshot` (pure Rust, no model file).
3. Run ASR with **token-level timestamps enabled** — the C API exposes
   `enable_token_timestamps` and a `timestamps` array parallel to the tokens,
   on both the online and offline recognisers. This gives you per-syllable
   boundaries.
4. Within each syllable window, estimate F0 with `pitch-detection`
   (McLeod pitch method), discard unvoiced frames, normalise to the speaker's
   own range over the utterance.
5. Compare the normalised contour to the target tone's canonical shape — either
   by DTW distance, or by the classical five-level scale (tone 1 高平 55,
   tone 2 rising 35, tone 3 dipping 214, tone 4 falling 51).

The target tone comes from data the app **already has**: per-character pinyin
with tone marks is in `hanzi.bin.gz` today.

Two caveats worth designing around. Tone 3 in running speech is usually realised
as a low contour without the textbook dip, so scoring it against 214 will flag
correct speech as wrong. And tone sandhi is real — ROADMAP already notes that
你好 → `níhǎo` is not currently applied. Both argue for starting with
single-syllable practice, where the citation forms actually hold.

### 6.3 What sherpa-onnx does not give you

I read the full C API header (4,756 lines). There is **no forced alignment, no
phone-level posterior, and no goodness-of-pronunciation scoring**. The academic
approach to computer-assisted pronunciation training — GOP scores from acoustic
model posteriors, optionally combined with DTW over pitch contours — is not
available through this library. You would need Kaldi-style alignment machinery,
which is a much larger undertaking than anything else in this document.

What you *can* build without it: syllable-level correct/incorrect from ASR text
compared as **pinyin rather than characters** (which correctly collapses the
homophones that make character-level comparison useless for single syllables —
and you already have the character→pinyin mapping), plus a genuine tone score
from F0. That is a useful tutor. It is not phone-level diagnosis, and the UI
should not imply that it is.

### 6.4 Suggested scope for a first version

Push-to-talk, one syllable or one word at a time, against a known target, with
three outputs: *heard as* (pinyin), *tone* (score plus a contour drawn against
the expected shape), and an honest "not sure" when VAD or confidence is poor.
The existing stroke-grading UI already sets a good precedent for showing a
learner *why* something scored as it did rather than just a number.

---

## 7. Packaging and platform consequences

**Binary size.** Add roughly 25–45 MB of native code per platform (measured
compressed shared-library archives: 26.9 MB linux-x64, 41.5 MB
osx-universal2). Against a current bundle that is essentially code plus one
compressed data artefact, this is a large relative increase.

**Build-time download.** `sherpa-onnx-sys` fetches a prebuilt archive during
`cargo build` unless `SHERPA_ONNX_LIB_DIR` is set. Vendor and pin it if this
matters to you — and given how `scripts/fetch-data.sh` and `LICENSES.md` are
written, I think it does.

**Microphone permissions.** ASR needs capture, and this is fiddly on macOS:
`NSMicrophoneUsageDescription` in `Info.plist` *and* the
`com.apple.security.device.audio-input` entitlement when hardened runtime is
enabled. The common failure mode is that capture works in `tauri dev` and in
unsigned builds, then silently fails once signed. This affects `cpal` and
WKWebView `getUserMedia` alike.

**Capture in Rust or in the webview?** `cpal` in `src-tauri` is the more
predictable choice: WKWebView's `getUserMedia` has a history of permission
prompts not appearing on macOS, and keeping audio in Rust puts the samples where
the model already is, avoiding a round trip through IPC.

**Model delivery.** 140 MB (TTS) + 155 MB (ASR) cannot go in the repository or a
`.dmg`. It needs a download-on-demand flow with progress, checksum verification,
a cache in the app data directory, and a clear statement to the user that this
is the one thing that touches the network. That is a real feature, not a
detail — and it is the thing the pre-rendered pack in §4.4 avoids entirely for
the bundled curriculum.

**Memory on 8 GB.** Not the binding constraint. SenseVoice int8 resident is
perhaps 300–400 MB during a recognition, Kokoro int8 rather less, and neither
needs to stay loaded between interactions. Lazy-load, and unload after an idle
timeout. The existing `OnceLock` warm-up pattern in `state.rs` is the right shape
for this already.

---

## 8. Honest assessment of whether this is worth doing

The TTS case is strong but the neural part of it is weak. What the app needs —
correct, clear readings of a fixed curriculum — is better served by pre-rendered
audio than by a 140 MB model, on every axis except handling user-entered words.
M6 as currently scoped (system TTS on Windows and Linux) plus an audio pack
would deliver more of the actual value than embedding a model does, at a
fraction of the cost and with none of the packaging complications.

The ASR case is the interesting one, because there is no non-neural substitute:
you cannot pre-render a learner's voice. But it is also the one where the gap
between "recognises speech" and "teaches pronunciation" is widest, and where a
mediocre implementation is worse than none — a tutor that tells a learner their
tones are fine when they are not has actively harmed them. ROADMAP's existing
note that speech recognition for tones is "a different problem from handwriting"
is, on this evidence, correct.

If I were sequencing it: audio pack first (largest benefit per unit of risk),
then ASR with tone scoring as a clearly-labelled experimental feature, then
neural TTS last and only if the personal vocabulary list turns out to be heavily
used.

---

## 9. Licence compatibility

The app is **AGPL-3.0-only**, and `LICENSES.md` sets a standard of tracking
exactly what must travel with a redistribution. Model weights need the same
treatment.

| Component | Licence | Compatible with shipping under AGPL-3.0? |
| --- | --- | --- |
| `sherpa-onnx`, `sherpa-onnx-sys` | Apache-2.0 | Yes — Apache-2.0 combines into (A)GPLv3 |
| ONNX Runtime (vendored inside) | MIT | Yes |
| `kokoro-*` weights | Apache-2.0 | Yes |
| `vits-melo-tts-zh_en` | MIT (MeloTTS) | Yes — verify the shipped `LICENSE` in the tarball |
| `vits-icefall-zh-aishell3` | AISHELL-3, permissive | Likely yes — verify before shipping |
| `matcha-icefall-zh-baker` | **Non-commercial only** | **No.** AGPL-3.0 guarantees downstream freedom to use for any purpose, including commercially. A non-commercial restriction contradicts that. |
| Supertonic 3 weights | OpenRAIL-M | **No** — behavioural use restrictions; not a free licence. (Moot: no Chinese support.) |
| `cpal`, `rubato`, `earshot`, `pitch-detection`, `pinyin`, `jieba-rs` | Apache-2.0 / MIT | Yes |
| `aubio-rs` | GPL-3.0 | Yes — GPLv3 and AGPLv3 are explicitly compatible |
| `ort`, `ort-sys` | MIT/Apache-2.0 | Yes |

Two things to check before shipping anything:

1. **Model tarballs carry their own `LICENSE` files** (Kokoro's is 11 KB,
   SenseVoice's is 71 bytes). These need reading and recording in `LICENSES.md`
   the way the data sources already are.
2. **A model's licence and its training data's licence can differ.** The Matcha
   case is exactly this: the code is free, the Databaker dataset is not, and the
   restriction follows the weights. Several of the `vits-zh-hf-*` models state no
   licence at all, which should be treated as "cannot ship" rather than
   "probably fine".

If audio is pre-rendered (§4.4), the same question applies to the **rendered
audio**, not just the model — a non-commercial model cannot be laundered by
shipping its output instead of its weights.

---

## 10. What I did not verify

Stated plainly, because these are the points where the recommendations could
change:

- **No benchmarks were run.** Every desktop throughput figure here is
  extrapolated from upstream's Raspberry Pi 4 and RK3588 numbers. The Kokoro
  int8 latency estimate in §4.3 is the one most worth measuring first, since the
  TTS recommendation rests on it.
- **Nothing was built.** I did not compile `sherpa-onnx` into a Tauri app, so
  the final binary size, the static-linking experience on Windows, and the
  macOS universal2 story are unconfirmed. The Windows release-asset names I
  guessed did not resolve, so Windows packaging specifically is unverified.
- **Sample rates** are verified only for `vits-icefall-zh-aishell3` (measured
  from ONNX metadata). The rest come from an upstream table that contradicts
  itself elsewhere. Verify the sample rate of whichever model you choose, the
  same way — it takes one download and a metadata read.
- **Token timestamp support is model-dependent.** The C API exposes it; the
  header says the field "may be NULL when the model does not provide
  timestamps". Zipformer transducers are the safe bet. Whether SenseVoice gives
  usable per-token timestamps — which §6.2's design depends on — needs
  confirming empirically.
- **CER figures are third-party**, from benchmark write-ups rather than a test I
  ran on this app's actual vocabulary. Short isolated syllables are a different
  regime from the meeting audio these benchmarks use, and are probably harder
  relative to the models' training distribution, not easier.
- **The pre-rendered pack size** (§4.4) is an arithmetic estimate from item
  count × duration × bitrate, not a measured encode.

---

## 11. Sources

Library and crate metadata:
[sherpa-onnx on crates.io](https://crates.io/crates/sherpa-onnx) ·
[sherpa-onnx Rust API examples](https://github.com/k2-fsa/sherpa-onnx/tree/master/rust-api-examples) ·
[sherpa-rs (deprecated)](https://github.com/thewh1teagle/sherpa-rs) ·
[whisper-rs](https://codeberg.org/tazz4843/whisper-rs) ·
[ort](https://crates.io/crates/ort) ·
[cpal](https://crates.io/crates/cpal) ·
[earshot](https://crates.io/crates/earshot) ·
[pitch-detection](https://crates.io/crates/pitch-detection) ·
[tts-rs](https://github.com/ndarilek/tts-rs)

Models and benchmarks:
[sherpa-onnx TTS pretrained models](https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/) ·
[sherpa-onnx TTS RTF table](https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/rtf.html) ·
[SenseVoice pretrained models](https://k2-fsa.github.io/sherpa/onnx/sense-voice/pretrained.html) ·
[SenseVoiceSmall model card](https://huggingface.co/FunAudioLLM/SenseVoiceSmall) ·
[FunASR vs Whisper Chinese benchmark](https://www.funasr.com/en/blog/funasr-vs-whisper-benchmark.html) ·
[Which FunASR model (2026)](https://www.funasr.com/en/blog/which-funasr-model.html) ·
[Supertonic](https://github.com/supertone-inc/supertonic) ·
[On-device TTS comparison 2026](https://picovoice.ai/blog/on-device-tts/) ·
[Kokoro local setup notes](https://localaimaster.com/blog/kokoro-tts-local-setup)

Pronunciation assessment background:
[Improving Mandarin tone mispronunciation detection for non-native speakers](https://oar.a-star.edu.sg/storage/d/d3m0z07vqn/wei-published-paper.pdf) ·
[Automatic detection of tone mispronunciation in Mandarin](https://link.springer.com/chapter/10.1007/11939993_61) ·
[End-to-end Mandarin tone classification with short-term context](https://arxiv.org/pdf/2104.05657) ·
[Data mining Mandarin tone contour shapes](https://arxiv.org/pdf/1907.01668)

Tauri integration:
[wry #1195 — getUserMedia permission prompt on macOS](https://github.com/tauri-apps/wry/issues/1195) ·
[tauri #9928 — microphone access from Rust on macOS](https://github.com/tauri-apps/tauri/issues/9928) ·
[tauri #11951 — macOS microphone permission not prompted](https://github.com/tauri-apps/tauri/issues/11951)
