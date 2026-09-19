# ASR and TTS for HanziTutor

**Research date:** 2026-09-19  
**Project:** [HanziTutor](https://github.com/hherb/hanzitutor)  
**Scope:** Small, local Mandarin ASR and TTS suitable for a Rust/Tauri application running primarily on CPU, including machines with 8 GB RAM.

## Executive summary

HanziTutor already has a sensible pronunciation architecture: the Rust-side `Speaker` in `src-tauri/src/speech.rs` owns speech output and currently delegates to the operating system (macOS `say`). That should be retained as a fallback, but generalized into pluggable speech backends.

For a cross-platform neural implementation, **sherpa-onnx is the strongest overall fit**. It now exposes safe Rust bindings and provides offline ASR, streaming ASR, TTS, VAD, resampling, and keyword spotting through one runtime. It supports macOS, Windows, Linux, Android, and iOS, and its small models are explicitly intended for embedded/resource-constrained systems.

The smallest useful Mandarin ASR candidate found is the **sherpa-onnx streaming Zipformer 14M Chinese model**. Its INT8 encoder, decoder, and joiner total about **25.3 MB of ONNX weights**. For a larger but still lightweight recognizer, the **Paraformer zh-small INT8** model is about **79 MiB**.

For TTS, the smallest technically attractive model found is **Piper Huayan x-low**, whose ONNX model is about **20.6 MB**, but its model card lists the training dataset licence as **Unknown**. It should therefore **not be bundled** until provenance/licensing is clarified.

The safest small sherpa-native Mandarin TTS option is **AISHELL-3 VITS**. Its ONNX network is only **29 MB**, although sherpa's summary table lists a larger overall model/package footprint of **116 MB** once supporting resources are considered. It runs comfortably faster than real time on a Raspberry Pi 4. Its main weakness for a pronunciation-learning app is its **8 kHz output**, which is adequate for intelligibility but not ideal as a high-fidelity pronunciation reference.

For better TTS quality, **Kokoro 82M Chinese** is attractive. A Rust implementation exposes a roughly **92 MB INT8 ONNX model**, Mandarin voices, CPU fallback, and higher-quality output. The Chinese model is published as Apache-2.0. This is likely the better long-term learner-facing reference voice if its Mandarin polyphone handling passes HanziTutor-specific tests.

The most important architectural conclusion is that **ASR alone should not be treated as pronunciation grading**. General ASR is optimized to infer likely text, not to diagnose Mandarin initials, finals, or tones. A learner can pronounce the wrong tone and still receive the correct character from ASR. HanziTutor should therefore combine:

1. **ASR / target recognition**: did the learner say the expected syllable or word?
2. **F0 contour analysis**: did the pitch trajectory resemble the expected Mandarin tone?
3. Later, if necessary, **phone-level confidence/GOP-style scoring** for initials and finals.

A small pure-Rust pitch detector such as `pitch-estimate` is enough for the first useful tone evaluator.

---

## Goals and constraints

The intended speech subsystem should:

- remain fully offline at runtime;
- work on macOS, Windows, and Linux, with a path to Android/iOS;
- be callable naturally from Rust;
- run well on CPU-only machines;
- remain comfortable on systems with **8 GB RAM**;
- keep downloadable model size modest;
- preserve HanziTutor's existing privacy/offline guarantees;
- synthesize Mandarin characters, words, and short phrases correctly;
- recognize learner speech for short words and syllables;
- support pronunciation feedback rather than merely transcription;
- have redistribution terms compatible with HanziTutor's AGPL-3.0 distribution model.

The app does **not** need a general dictation engine. Most recognition requests will have a known target or a very small set of expected targets. That distinction strongly favours small specialized recognizers plus explicit pronunciation analysis over a large general-purpose speech model.

---

## Current HanziTutor architecture

The existing implementation in `src-tauri/src/speech.rs` is a good starting point.

Current behaviour:

- pronunciation is owned on the Rust side;
- macOS delegates to `/usr/bin/say`;
- Chinese voices are discovered and selected explicitly;
- mainland Mandarin is preferred;
- only one utterance is active at a time;
- word-level synthesis is deliberately used for polyphonic characters;
- pronunciation is exposed through thin Tauri commands;
- `hanzi-core` remains independent from platform speech APIs.

This separation should be preserved. Neural speech support belongs in the Tauri/platform layer or in a new speech crate, not inside the handwriting/data core.

The current `Speaker` should evolve into a facade over one or more TTS backends instead of being replaced wholesale.

---

# ASR research

## 1. sherpa-onnx

### Why it fits HanziTutor

The current Rust crate exposes safe wrappers for:

- `OfflineRecognizer`;
- `OnlineRecognizer`;
- `OfflineTts`;
- `VoiceActivityDetector`;
- keyword spotting;
- resampling;
- WAV I/O;
- other speech utilities.

As of this research, docs.rs lists `sherpa-onnx 1.13.8`. The crate statically links by default and can automatically obtain matching native archives. For reproducible/offline builds, `SHERPA_ONNX_ARCHIVE_DIR` can point to pre-downloaded archives instead.

This gives HanziTutor one native inference stack for ASR and TTS rather than separate whisper.cpp, Piper, ONNX Runtime, and VAD integrations.

### 1.1 Streaming Zipformer Chinese 14M

**Model:** `sherpa-onnx-streaming-zipformer-zh-14M-2023-02-23`

The INT8 files are approximately:

| Component | Size |
|---|---:|
| encoder INT8 | 21.6 MB |
| decoder INT8 | 1.89 MB |
| joiner INT8 | 1.8 MB |
| **Total neural weights** | **~25.3 MB** |

The model is Mandarin/Chinese only and is trained from WenetSpeech. The Hugging Face model repository declares **Apache-2.0**.

sherpa specifically lists this model among its small models suitable for resource-constrained systems. Its documentation notes that all sherpa-onnx models can run in real time on a Raspberry Pi 4, with the small-model list aimed at systems with even fewer resources.

### Suitability

**Advantages**

- extremely small;
- streaming;
- Chinese-specific;
- natural Rust integration through sherpa-onnx;
- CPU friendly;
- straightforward microphone UX;
- low RAM pressure relative to the 8 GB target.

**Limitations**

- small size necessarily trades recognition accuracy for footprint;
- generic ASR output is not a pronunciation score;
- isolated syllables are a difficult regime for language-model-assisted recognizers;
- Mandarin tone errors may not change the recognized Hanzi.

### Recommendation

This should be the **first ASR model benchmarked** for HanziTutor.

It is small enough that even if a larger model is eventually preferred for the default install, it could remain useful as a "tiny" or low-resource model.

---

## 1.2 Paraformer zh-small

**Model:** `sherpa-onnx-paraformer-zh-small-2024-03-09`

The INT8 ONNX model is approximately **79 MiB**.

It supports Chinese and English and the sherpa documentation notes support for Mandarin plus several Chinese regional varieties. It is an offline/non-streaming model and does not provide timestamps.

### Suitability

For HanziTutor's typical utterances—one syllable, one word, or a short phrase—offline recognition is not a disadvantage. In fact, a small offline recognizer may be simpler than maintaining a streaming decoder when the app already knows when the learner presses/releases a record control.

### Recommendation

Benchmark this directly against the 14M Zipformer. Do not assume the larger model is automatically necessary; use learner-speech accuracy to decide.

---

## 2. Vosk

The Vosk Chinese small model, `vosk-model-small-cn-0.22`, is approximately **42 MB** and is explicitly described as a lightweight Android/Raspberry Pi model. It is Apache-2.0.

Published Chinese test error rates for the small model are notably worse than the large server model, which is expected at this footprint.

### Advantages

- mature;
- very light;
- proven on low-power hardware;
- dynamic vocabulary/grammar support is useful for constrained recognition.

### Disadvantages

- separate inference stack from TTS;
- Rust integration is less unified than sherpa;
- accuracy is likely inferior to newer small neural recognizers;
- still does not solve tone/pronunciation assessment.

### Recommendation

Keep Vosk as a **fallback comparison**, especially because grammar-constrained recognition maps well to a tutor where the expected word is known. It is not the preferred first implementation.

---

## 3. Whisper / whisper-rs

`whisper-rs` wraps whisper.cpp and is technically mature enough for local Rust use.

The standard multilingual Whisper tiny model is about:

- **75 MiB on disk**
- approximately **273 MB inference memory** according to the current whisper.cpp README.

whisper.cpp supports integer quantization, reducing disk/RAM further.

### Advantages

- excellent ecosystem;
- multilingual;
- robust on unrestricted conversational speech;
- mature Rust binding;
- well understood deployment model.

### Disadvantages for HanziTutor

Whisper is solving a broader problem than HanziTutor needs. It is optimized for unconstrained transcription and natural speech context. HanziTutor frequently needs to distinguish a known target from a tiny set of alternatives.

For a one-syllable Mandarin pronunciation exercise, a general decoder can use linguistic priors to return the intended Hanzi despite an incorrect tone. That is helpful in dictation but counterproductive for pronunciation assessment.

### Recommendation

Do **not** make Whisper the default pronunciation recognizer. Keep `whisper-rs` as a useful benchmark and possible later feature for free-form sentence transcription.

---

## 4. sherpa keyword spotting: interesting future direction

sherpa-onnx also exposes a Rust `KeywordSpotter` with dynamically supplied keywords.

That is conceptually a very strong match for HanziTutor:

> Given one or a few expected Chinese words, which one was spoken?

This is closer to the tutor problem than open-ended transcription.

Some sherpa Chinese keyword-spotting models are only a few million parameters. However, model-weight/training-data licensing for individual KWS releases has not always been as explicit as for the 14M Zipformer.

### Recommendation

Treat target-aware keyword recognition as a **future optimization** after the initial ASR path works. Do not make it the first shipping implementation until the exact model's redistribution terms are verified.

---

# TTS research

## 1. Existing OS TTS should remain

The existing system voice backend has important strengths:

- zero model download;
- excellent integration with the host;
- low application footprint;
- likely hardware/platform optimization;
- already tested in HanziTutor;
- strong macOS Mandarin voices.

It should remain available as:

- the default when no neural model is installed;
- an explicit "System voice" option;
- a regression/reference backend.

The goal of neural TTS is cross-platform consistency and quality—not replacing a working OS facility for its own sake.

---

## 2. sherpa-onnx AISHELL-3 VITS

**Model:** `vits-icefall-zh-aishell3`

Properties:

- Chinese only;
- 174 speakers;
- ONNX network: **~29 MB**;
- sherpa summary table: **116 MB model/package footprint**;
- output sample rate: **8 kHz**;
- trained from AISHELL-3;
- runs through the same `sherpa-onnx` Rust API as ASR.

The distinction between 29 MB and 116 MB matters: the network itself is tiny, but a distributable TTS bundle also requires lexical/text-normalization resources. HanziTutor's packaging estimate should therefore be based on the complete downloaded model directory, not only `model.onnx`.

### CPU performance

sherpa reports the following real-time factors on a Raspberry Pi 4 for this model:

| Threads | RTF |
|---:|---:|
| 1 | 0.365 |
| 2 | 0.220 |
| 3 | 0.171 |
| 4 | 0.156 |

An RTF below 1 means faster-than-real-time synthesis, so even one Pi 4 CPU thread is comfortably sufficient.

### Advantages

- tiny neural network;
- extremely light CPU use;
- same runtime as recommended ASR;
- many speakers;
- good low-resource baseline.

### Main weakness

**8 kHz is low for a pronunciation tutor.**

It is sufficient for intelligibility, but HanziTutor is teaching details such as initials/finals and contrasts involving high-frequency consonant energy. A 16–24 kHz reference voice is preferable if practical.

### Recommendation

Use AISHELL-3 as the **small sherpa-native TTS baseline**, but do not assume it should be the final reference voice.

Before redistribution, verify the exact model archive's licence/notice requirements in addition to the AISHELL-3 dataset terms.

---

## 3. Kokoro 82M Chinese

Kokoro is a small modern TTS family. The Chinese v1.1 model is published as **Apache-2.0** and adds Mandarin voices from a professional Chinese dataset that the model card says was permissively provided.

The Rust crate `kokoro-en`—despite its name—documents Mandarin female/male voices for v1.1.

Available ONNX variants documented by the crate include:

| Model | Approx. size |
|---|---:|
| FP32 | 325 MB |
| mixed INT8/FP16 | 160 MB |
| INT8 quantized | **92 MB** |

The library supports CPU fallback and platform accelerators where available.

### Advantages

- substantially better quality potential than an 8 kHz VITS baseline;
- still small enough for an optional download;
- CPU viable;
- Mandarin voices;
- Apache-2.0 Chinese model;
- attractive as a learner-facing reference voice.

### Risks / validation requirements

- verify that the exact Rust implementation and the desired Chinese v1.1 ONNX/voice assets work together;
- evaluate Chinese text normalization;
- test polyphonic characters and word context rigorously;
- test tone fidelity against native recordings;
- measure cold-start latency and RAM on the lowest target hardware.

### Recommendation

**Best quality-oriented TTS candidate to benchmark.**

A likely final configuration is system TTS by default plus an optional Kokoro "neural voice" download.

---

## 4. Piper Huayan

Piper's `zh_CN-huayan-x_low` is extremely small:

- ONNX model: **20.6 MB**
- sample rate: **16 kHz**

The medium version is about **63.2 MB**.

This would otherwise be an excellent footprint/quality candidate.

However, the model card explicitly lists:

> Dataset licence: Unknown

The repository itself being MIT does not resolve the dataset/model provenance problem.

### Recommendation

**Do not bundle Huayan in HanziTutor unless the training-data/model redistribution status is clarified.**

Absolute model size is not worth a licensing ambiguity when clean alternatives exist.

---

# Rust libraries

## sherpa-onnx

Recommended primary inference library.

Use for:

- streaming/offline ASR;
- TTS;
- optional VAD;
- resampling;
- future keyword spotting.

This reduces duplicated native dependencies and gives the project one cross-platform speech runtime.

---

## cpal

Recommended microphone/audio I/O layer.

`cpal` is a low-level cross-platform Rust audio library that supports device enumeration and input/output streams. It maps naturally onto:

- CoreAudio on Apple platforms;
- WASAPI on Windows;
- ALSA/PipeWire/PulseAudio depending Linux configuration;
- Android audio backends.

HanziTutor should capture microphone input into normalized mono `f32` PCM and keep platform audio APIs behind a small Rust abstraction.

---

## pitch-estimate

Recommended initial pitch/F0 library.

`pitch-estimate` provides pure-Rust monophonic pitch detection using:

- McLeod Pitch Method (MPM);
- YIN.

It returns frequency and clarity per frame and is tiny compared with any neural speech model.

The current crate is stateless per frame, which is fine for a first implementation. HanziTutor can smooth/interpolate the resulting F0 series itself.

A future dedicated streaming tracker could replace it behind the same interface.

---

## whisper-rs

Keep as an optional benchmark/future free-form transcription backend, not the first tutor-oriented ASR implementation.

---

# Why ASR must not be the pronunciation score

This is the most important design constraint.

Suppose the learner is asked to pronounce:

- 妈 — `mā`
- 麻 — `má`
- 马 — `mǎ`
- 骂 — `mà`

A general ASR system is trained to determine likely text. If context or a constrained vocabulary strongly suggests 马, it may output 马 even when the acoustic pitch contour is closer to another tone.

That means:

```text
ASR says "马"
```

does **not** imply:

```text
the learner pronounced mǎ correctly
```

Likewise, ASR failure does not explain whether the problem was:

- onset consonant;
- final/vowel;
- nasal ending;
- tone;
- timing;
- microphone noise;
- speaking too softly.

HanziTutor should therefore treat recognition and pronunciation assessment as related but separate measurements.

---

# Proposed pronunciation evaluator

## Phase-one signal path

```text
microphone
    |
    v
audio capture (cpal)
    |
    +----------------------+
    |                      |
    v                      v
VAD / trimming       F0 extraction
    |                      |
    v                      v
small Mandarin ASR   tone contour
    |                      |
    +----------+-----------+
               |
               v
      pronunciation result
```

The result should initially expose independent dimensions rather than a single opaque score.

For example:

```rust
pub struct PronunciationResult {
    pub recognized_text: Option<String>,
    pub target_match: f32,
    pub tone_match: Option<f32>,
    pub signal_quality: f32,
    pub feedback: Vec<PronunciationFeedback>,
}
```

Later fields can be added for initials/finals or phone-level scoring without breaking the conceptual model.

---

## Tone contour analysis

### Why pitch contour is attractive

Mandarin tone is strongly represented in the F0 trajectory and can be evaluated without a large neural model.

For each voiced frame:

1. estimate F0;
2. reject frames below a clarity threshold;
3. convert frequency to a log scale;
4. normalize to the speaker/utterance;
5. smooth short dropouts;
6. compare the contour with the expected tone pattern.

Approximate isolated-syllable expectations:

| Tone | Expected normalized contour |
|---|---|
| 1 | high and relatively level |
| 2 | rising |
| 3 | low / dipping, context dependent |
| 4 | sharply falling |
| neutral | short, reduced, context dependent |

Do not hard-code textbook five-level tone diagrams as literal acoustic templates. Natural tone realization varies by speaker, vowel, phrase position, neighbouring tones, and speaking rate.

The first production implementation should classify broad contour properties rather than demand exact curves.

---

## Sandhi and lexical context

Tone scoring must use the **word-level target pronunciation**, not only dictionary tone numbers for isolated characters.

Important cases include:

- third-tone sandhi;
- `一` tone changes;
- `不` tone changes;
- neutral tone;
- lexical polyphones;
- connected-speech reduction.

This aligns well with HanziTutor's current design: the project already prefers whole-word pronunciation because word context resolves polyphonic characters such as `着` in `着急`.

Pronunciation practice should reuse that same contextual reading source.

---

# Proposed architecture

## Preserve a backend-independent facade

A possible direction:

```rust
pub trait TtsBackend: Send + Sync {
    fn id(&self) -> &'static str;
    fn status(&self) -> SpeechBackendStatus;
    fn synthesize(&self, request: &TtsRequest)
        -> Result<SynthesizedAudio, SpeechError>;
}

pub trait AsrBackend: Send + Sync {
    fn id(&self) -> &'static str;
    fn recognize(&self, request: &AsrRequest)
        -> Result<RecognitionResult, SpeechError>;
}

pub trait PronunciationEvaluator: Send + Sync {
    fn evaluate(
        &self,
        audio: &AudioBuffer,
        target: &PronunciationTarget,
    ) -> Result<PronunciationResult, SpeechError>;
}
```

Do not force the traits to be asynchronous unless the implementation genuinely needs it. Tauri commands can move CPU work onto blocking worker threads.

Suggested implementations:

```text
TTS
 ├── SystemTtsBackend      # existing macOS behaviour; later OS implementations
 ├── SherpaVitsBackend     # compact neural option
 └── KokoroBackend         # optional quality-oriented model

ASR
 └── SherpaAsrBackend
      ├── Zipformer14M
      └── ParaformerSmall

Pronunciation
 └── MandarinPronunciationEvaluator
      ├── target ASR match
      ├── pitch contour
      └── signal-quality checks
```

---

# Suggested Rust/module layout

Avoid putting all neural/model-management logic into the existing `speech.rs`.

One possible structure:

```text
src-tauri/src/speech/
    mod.rs
    error.rs
    audio.rs
    model.rs

    tts/
        mod.rs
        system.rs
        sherpa.rs
        kokoro.rs

    asr/
        mod.rs
        sherpa.rs

    pronunciation/
        mod.rs
        pitch.rs
        mandarin.rs

    capture/
        mod.rs
        cpal.rs
```

Alternatively, if mobile reuse becomes important early, extract most of this into:

```text
crates/hanzi-speech/
```

with only microphone permissions and Tauri command wiring in `src-tauri`.

The latter is preferable once speech is more than an experiment.

---

# Model management

## Do not bake all models into the base application

The current HanziTutor application is compact and fully usable without neural speech. Users should not be forced to download hundreds of megabytes for an optional pronunciation feature.

Recommended approach:

1. base application ships with system TTS support;
2. neural speech is presented as an optional offline language pack;
3. models download once;
4. model files live in HanziTutor's application-data/cache area;
5. inference never needs network access.

Example conceptual layout:

```text
<app-data>/
    speech/
        manifest.json
        asr/
            zipformer-zh-14m/
                ...
        tts/
            kokoro-zh/
                ...
```

---

## Model manifest

Maintain an application-owned manifest rather than scattering model URLs and hashes through Rust source.

Example:

```json
{
  "id": "zipformer-zh-14m-int8",
  "kind": "asr",
  "language": "zh-CN",
  "version": "2023-02-23",
  "runtime": "sherpa-onnx",
  "download_bytes": 0,
  "installed_bytes": 0,
  "sha256": "...",
  "license": "Apache-2.0",
  "license_url": "...",
  "source_url": "...",
  "files": []
}
```

The real manifest should contain the exact archive/file checksums and sizes measured at integration time.

Requirements:

- HTTPS only;
- SHA-256 verification before activation;
- atomic install into a temporary directory then rename;
- explicit model version;
- explicit licence text/notice;
- ability to delete models from settings;
- no silent model replacement.

This is consistent with HanziTutor's existing cautious approach to persistence and corrupted state.

---

# Recommended configurations

## Minimal-footprint configuration

### ASR

**sherpa Zipformer Chinese 14M INT8**

~25.3 MB of neural weights.

### TTS

**AISHELL-3 VITS** as the sherpa-only baseline.

29 MB ONNX network, but budget for the complete model package rather than only the network.

### Pronunciation

**pitch-estimate** plus HanziTutor's contextual pinyin target.

### Strength

One speech inference runtime and extremely modest compute requirements.

### Weakness

AISHELL-3's 8 kHz synthesis is not ideal as a pronunciation reference.

---

## Preferred quality-oriented configuration to test

### ASR

Benchmark:

1. Zipformer 14M INT8
2. Paraformer zh-small INT8

Choose based on actual learner speech.

### TTS

**Kokoro Chinese INT8 (~92 MB)** if it passes pronunciation fidelity and polyphone tests.

### Pronunciation

ASR target match + F0/tone contour.

This configuration should still be trivial for an 8 GB desktop/laptop while providing a substantially nicer reference voice.

---

# Implementation plan

## Phase 0 — Build an evaluation corpus first

Do this before choosing final models.

Create a small checked-in metadata corpus with audio stored either in a dedicated test-data download or an optional repository fixture set.

Cover approximately 100–200 utterances including:

### Tones

- all four lexical tones;
- neutral tone;
- same-final tone quartets where possible;
- third-tone + third-tone sequences;
- third tone before tones 1/2/4;
- `一`;
- `不`.

### Initials

Include contrasts likely to challenge non-native learners:

- `j/q/x`;
- `zh/ch/sh/r`;
- `z/c/s`;
- aspirated vs unaspirated pairs;
- `n/l`.

### Finals

Include:

- `-n` vs `-ng`;
- `ü` contrasts;
- `i` variants after sibilants/retroflexes;
- compound finals;
- erhua examples if HanziTutor intends to teach it.

### Lexical/context cases

Include HanziTutor vocabulary containing:

- polyphonic characters;
- `着急` and similar context-dependent readings;
- short two-character words;
- common function words;
- short learner-level phrases.

Use recordings from:

- multiple native Mandarin speakers;
- several learners if available;
- different microphones;
- quiet and moderate real-room noise.

The test corpus should contain the expected Hanzi, pinyin, contextual tone sequence, and speaker metadata.

---

## Phase 1 — Refactor current TTS without changing behaviour

Goal: create the backend abstraction while keeping existing macOS speech working exactly as it does now.

Tasks:

- move current macOS implementation into `SystemTtsBackend`;
- introduce common TTS request/error/status types;
- preserve `speak`, `stop_speaking`, and `speech_status` Tauri behaviour;
- preserve existing voice-selection tests;
- add backend-selection tests.

Acceptance criterion:

> Existing pronunciation UX and tests behave identically with only the system backend installed.

---

## Phase 2 — Audio capture

Add `cpal`.

Tasks:

- enumerate/default microphone;
- request permission appropriately per platform;
- capture mono PCM;
- normalize sample format to `f32`;
- resample to ASR requirements where necessary;
- add explicit start/stop/cancel recording state;
- enforce maximum utterance duration;
- expose microphone status/errors cleanly to Svelte.

Do not persist recordings by default.

Acceptance criteria:

- stable capture on macOS/Windows/Linux;
- no UI-thread blocking;
- bounded memory;
- recording cancellation always releases device resources.

---

## Phase 3 — sherpa ASR proof of concept

Add `sherpa-onnx` behind `SherpaAsrBackend`.

Initially support one model at a time.

Benchmark:

1. Zipformer 14M INT8;
2. Paraformer zh-small INT8.

Measure:

- cold model-load time;
- warm recognition latency;
- peak RSS;
- recognition accuracy on the HanziTutor corpus;
- isolated-character accuracy;
- word accuracy;
- behaviour with incorrect tones;
- behaviour with learner accents.

Acceptance criterion:

> Short Mandarin targets are recognized with useful accuracy on CPU without perceptible UI stalls on the lowest supported hardware.

Do not yet call the ASR result a pronunciation score.

---

## Phase 4 — Tone/F0 evaluator

Add `pitch-estimate`.

Pipeline:

1. trim silence/VAD;
2. window the voiced region;
3. obtain F0 + clarity per frame;
4. remove unreliable frames;
5. interpolate short gaps;
6. convert to log-frequency/semitone-like representation;
7. normalize per utterance/speaker;
8. compute simple contour features.

Initial features may include:

- start pitch;
- end pitch;
- min/max position;
- overall slope;
- first-half slope;
- second-half slope;
- curvature/dip;
- voiced duration.

Start with interpretable rules or a tiny classifier trained only on these features. Avoid introducing a neural tone model until the simple method is measured.

Acceptance criterion:

> Tone feedback discriminates deliberately wrong tones materially better than chance without rejecting normal native variation excessively.

---

## Phase 5 — First pronunciation UX

Expose separate feedback:

- **Word/syllable recognized**
- **Tone**
- **Recording quality**

Avoid presenting a single pseudo-precise percentage initially.

Example UX:

```text
Word:  ✓ recognized as 马
Tone:  needs work — your pitch stayed too level
Audio: good
```

or:

```text
Word:  not confidently recognized
Tone:  contour looked like a 3rd tone
Audio: good
```

This is more educational than a mysterious "72% pronunciation score".

---

## Phase 6 — Neural TTS benchmark

Implement sherpa AISHELL-3 first because it shares the ASR runtime and is easy to evaluate.

Then benchmark Kokoro Chinese.

Test:

- tone correctness;
- polyphonic words;
- neutral tone;
- sandhi;
- words already covered by HanziTutor tests;
- speed;
- cold start;
- RAM;
- intelligibility on small laptop speakers;
- learner preference versus system Mandarin voice.

Acceptance criterion:

> The neural backend must equal or exceed OS TTS on Mandarin pronunciation correctness before becoming the default.

Naturalness alone is not sufficient.

---

## Phase 7 — Model manager

Add optional model download/install/delete UI.

Requirements:

- explicit download size;
- licence display;
- checksum verification;
- progress;
- cancellation;
- rollback on incomplete download;
- "System speech only" remains a valid configuration;
- app remains functional without any neural model.

Suggested presets:

### Speech: system

No download.

### Speech: compact

Small sherpa ASR + compact TTS.

### Speech: enhanced

Best validated ASR + Kokoro-quality TTS.

Do not expose model implementation jargon to ordinary learners unless they open advanced settings.

---

## Phase 8 — Cross-platform packaging

Test release builds for:

- macOS arm64;
- macOS x86_64 if still supported;
- Windows x86_64;
- Linux x86_64;
- later Android/iOS.

Use reproducible sherpa native archives rather than build-time network fetches in release CI.

Record:

- exact sherpa version;
- native archive hashes;
- ONNX model hashes;
- licences and notices.

---

## Phase 9 — Possible phone-level assessment

Only add this if ASR + tone feedback proves insufficient.

A more advanced pronunciation evaluator could score:

- initials;
- finals;
- tone.

Possible approaches:

- CTC token posterior scoring;
- forced alignment;
- GOP-like phone likelihood ratios;
- a small Mandarin acoustic model with known target transcription.

This is more complex and should be driven by observed learner-feedback failures rather than implemented speculatively.

---

# Performance expectations

An 8 GB RAM minimum is generous for all recommended initial models.

Approximate order of magnitude:

| Component | Model footprint | Expected RAM concern on 8 GB |
|---|---:|---|
| Zipformer 14M INT8 ASR | ~25 MB weights | negligible |
| Paraformer zh-small INT8 | ~79 MiB | low |
| Vosk small-cn | 42 MB | low |
| Whisper tiny | 75 MiB disk, ~273 MB runtime reported | low |
| AISHELL-3 VITS | 29 MB ONNX; larger full package | low |
| Kokoro INT8 | ~92 MB ONNX | low |
| F0 detector | no neural weights | negligible |

The more important performance measures are:

- model cold-start latency;
- CPU utilization while the learner waits;
- battery impact;
- audio-device reliability;
- native-library package size;
- simultaneous memory if ASR and TTS are both kept loaded.

The implementation should therefore load models lazily and consider releasing them after an idle period on memory-constrained/mobile devices.

---

# Licensing and redistribution

HanziTutor's own AGPL-3.0 licence does not automatically make third-party speech models safe to redistribute. Track runtime code, model weights, and training-data provenance separately.

| Component | Current status | Recommendation |
|---|---|---|
| sherpa-onnx runtime | Apache-2.0 | suitable |
| Zipformer zh 14M model repo | Apache-2.0 metadata | suitable, retain notices |
| Vosk small-cn | Apache-2.0 | suitable |
| Kokoro Chinese model | Apache-2.0 | suitable subject to exact asset verification |
| Piper Huayan | repository MIT, **dataset licence unknown** | do not bundle |
| AISHELL-3 VITS | known source dataset/model chain, but exact redistributable bundle should be checked | verify archive before shipping |
| Whisper code/model ecosystem | permissive/open, exact shipped artifacts still require notices | suitable benchmark |

For every bundled/downloadable model, add:

- source;
- model version;
- licence;
- licence text;
- required attribution;
- checksum;
- training dataset/provenance note where available.

Do not treat a parent repository licence as sufficient when the model card says otherwise.

---

# Recommended decision

## Primary runtime

**sherpa-onnx**

It provides the best combination of:

- Rust API quality;
- cross-platform support;
- ASR + TTS in one runtime;
- small Mandarin models;
- CPU performance;
- future VAD/KWS support;
- mobile path.

## First ASR benchmark

**Zipformer Chinese 14M INT8**

If learner-speech accuracy is insufficient, move to **Paraformer zh-small INT8**.

## TTS strategy

Keep the current **system TTS** backend.

Benchmark:

1. **AISHELL-3 VITS** as the smallest sherpa-native implementation;
2. **Kokoro Chinese INT8** as the likely quality-oriented neural option.

Do not bundle Piper Huayan until its dataset licensing is clarified.

## Pronunciation assessment

Implement **ASR target matching + explicit F0/tone analysis**.

Do not use ASR text output alone as a pronunciation grade.

---

# Suggested first development milestone

A useful, bounded first milestone would be:

1. refactor `speech.rs` into a backend interface without changing current behaviour;
2. add `cpal` microphone capture;
3. add sherpa-onnx with Zipformer 14M;
4. add a hidden/developer pronunciation test screen;
5. display:
   - captured waveform duration;
   - ASR result;
   - expected target;
   - F0 contour;
   - expected tone;
6. collect measurements before designing learner-facing grading thresholds.

This produces an empirical foundation quickly while avoiding premature scoring logic.

After that milestone, the next decision should be based on measured results:

- if Zipformer is accurate enough, keep it;
- if not, test Paraformer;
- if tone contours already provide useful feedback, proceed;
- only then consider heavier phone-level pronunciation models.

---

# References

## HanziTutor

- https://github.com/hherb/hanzitutor
- `src-tauri/src/speech.rs`
- `src-tauri/Cargo.toml`

## sherpa-onnx

- Rust API: https://docs.rs/sherpa-onnx/latest/sherpa_onnx/
- Small ASR models: https://k2-fsa.github.io/sherpa/onnx/pretrained_models/small-online-models.html
- Zipformer models: https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/zipformer-transducer-models.html
- Zipformer 14M model repository: https://huggingface.co/csukuangfj/sherpa-onnx-streaming-zipformer-zh-14M-2023-02-23
- Paraformer models: https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-paraformer/paraformer-models.html
- VITS TTS models: https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/vits.html

## Kokoro

- Rust crate: https://docs.rs/crate/kokoro-en/latest
- Chinese model: https://huggingface.co/hexgrad/Kokoro-82M-v1.1-zh

## Piper

- Huayan x-low files: https://huggingface.co/rhasspy/piper-voices/tree/main/zh/zh_CN/huayan/x_low
- Huayan model card: https://huggingface.co/rhasspy/piper-voices/blob/main/zh/zh_CN/huayan/x_low/MODEL_CARD

## Vosk

- Model list: https://alphacephei.com/vosk/models

## Whisper

- whisper-rs: https://docs.rs/crate/whisper-rs/latest
- whisper.cpp: https://github.com/ggml-org/whisper.cpp

## Rust audio / pitch

- cpal: https://docs.rs/crate/cpal/latest
- pitch-estimate: https://docs.rs/crate/pitch-estimate/latest

---

## Bottom line

The practical solution is considerably smaller than might be expected.

A local Mandarin learning app does not require a multi-gigabyte speech model. A roughly 25 MB Mandarin ASR model can already run through a production-grade cross-platform Rust-accessible runtime, and high-quality TTS can remain under 100 MB of neural weights. On an 8 GB machine, memory is not the limiting constraint.

The key design choice is instead pedagogical: **recognition and pronunciation are not the same task**. HanziTutor will gain more from combining a tiny ASR model with explicit Mandarin tone analysis than from simply replacing that model with a much larger general-purpose recognizer.
