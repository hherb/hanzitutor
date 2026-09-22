# MeloTTS pronunciation accuracy on short Mandarin phrases

**Measured 2026-09-21**, while building M14 (graded phrase audio).

This records a defect found in the chosen synthesis model, and the comparison
that led to reconsidering it. It is written to be read before anyone regenerates
the bundled clips or changes the synthesis model.

## The defect

MeloTTS mispronounces some short phrases. It is not damage to the audio — the
clips are healthy by every signal measurement (peak normalised to ~0.67, no
clipping, no silence, durations in band for their text). The model simply
produces **different syllables**.

Measured on 40 HSK-1 phrases, one recogniser (SenseVoice, the model this app
already ships for speech recognition), one character-error-rate threshold of
0.34:

| Engine | Phrases mispronounced |
| --- | --- |
| MeloTTS (`csukuangfj/vits-melo-tts-zh_en`, int8) | **3 / 40 — 7.5%** |
| CosyVoice2 (the `no7z` dataset's own published audio) | **0 / 40 — 0%** |

The failure mode is specific and consistent: **the phrase-initial syllable is
what corrupts**, and a whole clause can be dropped.

| Text (what the audio should say) | MeloTTS audio, transcribed |
| --- | --- |
| 请进请坐 | 笔迹你做 |
| 先生您找谁 | 你你找谁 |
| 弟弟六岁妹妹三岁 | 你弟岁你妹岁 |
| 我饿了也渴了 | 我问问也可能 |
| 到了请下车 | 到了 (second clause absent) |
| 北边有山西边有树 | 里边有山一边有 |

Across the failures, the corrupted syllables are overwhelmingly phrase-initial:
请 → 谨/听/点/眼/银/笔, 先 → 姐/今生, 弟 → 你.

## What was ruled out, and how

Each of these was tested rather than assumed:

- **Not thread count.** Regenerating each failing phrase three times at
  `num_threads` 1, 2, 4 and 16 gave **0/3 correct at every setting**. (A separate
  finding from the same investigation: 16 threads is 45% *slower* than 4 on this
  model, which is why `SYNTHESIS_THREADS` is pinned to 4.)
- **Not punctuation.** `，` `。` `、` `；` and no separator were all tried across
  every failing phrase. `到了、请下车。` happens to produce correct audio while
  `到了，请下车。` does not, but nothing fixes 请进 请坐, and the results are
  inconsistent between phrases — so there is no rewrite rule to find. (A
  different punctuation defect *was* real and is fixed: an internal `！` makes the
  model drop what follows it. See `ROADMAP.md` M14.)
- **Not encoding or level.** The clips pass every faithfulness check in
  `scripts/check-audio.py`. The defect is in what was generated, not in how it was
  written out.
- **Retrying does not fix it.** Generation is stochastic — six calls for the same
  text returned six different outputs — but the outputs are *all* corruptions.
  Retrying only helps if good can be told from bad, and the CER screen alone
  cannot: it passed `眼镜请坐` and `请进星坐`, which are both wrong.

## Why the character-error screen is not enough

A plain CER threshold under-detects, because the recogniser often writes a
*plausible* neighbour rather than the expected character:

- `请进星坐` — 进 → 星 is a different syllable, but scores under threshold.
- `眼镜请坐` — 请 → 眼 and 进 → 镜, again under threshold.

What distinguishes a mere spelling difference from a mispronunciation is
**phonetic**, not orthographic:

| Difference | Meaning | Verdict |
| --- | --- | --- |
| Same initial, same final, different tone | 渴 → 可 | the recogniser's spelling — **tolerate** |
| Same initial, different final | 进 → 星 | a different syllable — **corrupt** |
| Different initial, different final | 请 → 笔 | **corrupt** |
| Length differs | 到了请下车 → 到了 | clause dropped — **corrupt** |

A prototype of that comparison — splitting each syllable into initial, final and
tone, and comparing the reading sets of the expected and heard characters —
correctly separates the measured cases: it flags `请→笔` and `进→迹` as
corruption while accepting `渴→可` as a tone-only difference. It is a prototype in
Python; the screening in `verify-audio` is still CER-only and should be extended
to this before it is trusted as a gate.

## A single-threshold verifier cannot be trusted either way

`verify-audio` now judges phonetically (`hanzi_say::judge`), which fixed two real
bugs in the first attempt — a same-final pair was accepted even when the initials
were unrelated sounds, and a lenient verdict could mask a corruption. But
tightening it exposed the harder problem, and it is worth recording so nobody
trusts the tool more than it deserves.

Run over the same 208 clips:

| Rule | Reported failures |
| --- | --- |
| Character-error rate only | 9 / 208 (4.3%) |
| Phonetic, tolerating any same-final initial | 9 / 208 |
| Phonetic, tolerating only *confusable* initials | **57 / 208 (27%)** |

The third is not 27% bad clips. Inspecting them shows the gate is reporting the
**recogniser's** errors as often as the model's:

- `洗手间在哪里` → `你首间在哪里` — audio that is correct; the recogniser misheard it.
- `小女孩儿和小男孩儿一起玩儿` → 13 chars wanted, 11 heard — the recogniser
  struggling with a long phrase, not a dropped clause.
- `病人在医院休息` → `一人在医院休息` — one character, plausibly a real 病→一 defect
  and indistinguishable from a recogniser slip by this method alone.

So the honest position is: **a recogniser-based screen cannot tell "the model said
it wrong" from "the recogniser heard it wrong"**, because it only has one
transcription to work from and no ground truth to compare against.

### What would settle it

A **differential** test. The `no7z` dataset publishes its own audio for the same
sentences, generated by CosyVoice2, which measured 0/40 failures on the sample in
this document. Run both through the same recogniser and the same judge, and a
phrase where the reference passes while ours fails is a MeloTTS defect; a phrase
where both fail is the recogniser's limitation. That comparison needs no listening
and no judgement call, and it is the check that should decide which clips ship.

It has now been run — see "The differential test has now been run" above. The 46
phrases MeloTTS gets wrong and the reference gets right are listed in
`docs/research/melotts-defects-hsk1.txt`, one id per line, reproducibly.

### The differential test has now been run — it settles the comparison

Run over 208 HSK-1 phrases, both engines through the **same recogniser and the
same judge** (`verify-audio` with `--base` pointed at each tree):

| | Clips failing |
| --- | --- |
| MeloTTS (ours) | **57 / 208 — 27.4%** |
| CosyVoice2 (the dataset's own audio) | **15 / 208 — 7.2%** |

A raw count is not the answer, because some failures are the *recogniser's*
limitation rather than either engine's fault. Splitting the two sets:

| | Phrases | Reading |
| --- | --- | --- |
| Both engines fail | **11** | the recogniser cannot transcribe these — not an engine defect |
| **Only MeloTTS fails** | **46 (22.1%)** | a MeloTTS defect, isolated by construction |
| Only the reference fails | 4 | 1.9% — the noise floor of this method |

That 46 is the number that matters, and it needs no listening to trust: the
reference audio for the same sentence passes the identical test, so the failure
cannot be the recogniser or the judge.

**MeloTTS gets roughly one phrase in five wrong on HSK 1.** The earlier 7.5%
figure from a hand-checked sample of 40 understated it; this is the same
measurement done properly, against a control.

The 11 both-fail cases are also worth keeping: they name the phrases this method
cannot judge, so they should be reviewed by ear rather than trusted either way.

### Both speeds checked

The differential test was run on the **normal** takes above. Repeating it on the
**slow** takes (`_slow.mp3`, which the reference dataset also publishes) gives the
same verdict, so the choice does not depend on which speed is used:

| Speed | MeloTTS | CosyVoice2 |
| --- | --- | --- |
| normal | 57 / 208 fail | 15 / 208 |
| slow | 44 / 208 fail | 15 / 208 |

The reference's 15 failures are the identical phrases at both speeds, which is
what a recogniser limitation looks like — the same audio content defeats it
either way. MeloTTS's counts differ between speeds (57 against 44), which is the
signature of a *generation* problem: the model is re-rolled per call, so whether a
given phrase comes out wrong varies with the take.

A consequence worth noting for anyone regenerating: **checking only the normal
take is not sufficient for MeloTTS**, because a phrase that is right at one speed
can be wrong at the other. For the reference audio it makes no difference.

## Consequence for the bundled clips

**A 7.5% mispronunciation rate is not shippable in a pronunciation tutor.** A
learner repeating 笔迹你做 is being taught the wrong thing, and no measurement
short of transcribing the audio can see it.

The options, with what each costs:

1. **Use the dataset's own MP3s** (CosyVoice2, Apache-2.0) for the bundled
   phrases — measured 0% on the same sample, no synthesis time. Cost: the bundled
   voice differs from the on-device MeloTTS voice, so the "same voice everywhere"
   claim in `ROADMAP.md` M14 and the licence notices would have to be withdrawn,
   and the dataset's slow takes need checking.
2. **Keep MeloTTS and reject failing phrases** — roughly 60 of 819 phrases
   silently absent from a graded course. Not acceptable.
3. **MeloTTS on demand only, dataset audio bundled** — two voices by design, each
   doing its own job.

No decision recorded yet. The MeloTTS path itself is sound and worth keeping
either way: it is MIT, runs on the `sherpa-onnx` the app already links, and is the
only way to speak text with no recording.

## The chosen direction: CosyVoice 3 at build time, MeloTTS on device

Decided after the differential test above. **CosyVoice 3 can be *told* the
pronunciation**, which turns the defect from something to avoid into something to
fix.

### Why it answers the problem

[CosyVoice 3](https://github.com/FunAudioLLM/CosyVoice)
(`FunAudioLLM/Fun-CosyVoice3-0.5B-2512`) supports **pronunciation inpainting of
Chinese pinyin**: a syllable can be forced by writing it into the text. The
model card's own example is

```
高管也通过电话、短信、微信等方式对报道[j][ǐ]予好评。
```

where `[j][ǐ]` pins 给's reading. The mechanism is in
`cosyvoice/tokenizer/tokenizer.py`, which registers the whole pinyin inventory as
special tokens — every initial (`[q]`, `[zh]`, `[x]`), every final (`[ing]`,
`[uan]`, `[üe]`) and every tone-marked vowel (`[ǐ]`, `[à]`, `[ǚ]`).

That is exactly the lever this defect needs. The 46 phrases listed in
`melotts-defects-hsk1.txt` are not *wrong text* — they are right text the model
mis-says. Inpainting the offending syllable fixes the audio without touching the
sentence, so the text stays verbatim from the corpus and the learner still reads
what the dataset published.

It is also the same engine the `no7z` dataset used, which is why its audio
measured 15/208 against MeloTTS's 57/208 on the differential test.

### Licence and size

- **Apache-2.0**, both the repository (23.7k stars) and the model card.
- Published test-zh CER **1.21%** (0.81% for the RL variant), speaker similarity 78%.
- **~9.7 GB** of weights — llm.pt and llm.rl.pt are 2.0 GB each, flow.pt 1.3 GB,
  the speech tokenizer 0.97 GB.

That size is disqualifying for a phone and irrelevant for a build: **the weights
never ship, only the MP3s they generate.** So the split is:

| | Engine | Why |
| --- | --- | --- |
| **Build time** (bundled clips) | CosyVoice 3 | quality and pinyin control; size is a build-host concern, not a shipping one |
| **On device** (a phrase with no clip) | MeloTTS | 53 MB int8; runs on the `sherpa-onnx` the app already links; built and pinned already |

This is what the objective's "both" already implies — the two paths were only
ever tied together by a wish for one voice, and that wish is not worth 22% of
phrases mispronounced. The notices change accordingly: the bundled clips become
CosyVoice2/CosyVoice3-derived (Apache-2.0) rather than MeloTTS, and the MeloTTS
notice stays for the on-device path.

### The plan

1. Add a CosyVoice 3 backend to `synthesize-audio`, behind the same interface, so
   the source of the clips is a choice rather than a rewrite.
2. Generate the HSK 1–2 set, then run the differential test again — the target is
   the reference's 15/208, not zero, since those 11 both-fail phrases are the
   recogniser's limit.
3. For any phrase that still fails, **inpaint the offending syllable by pinyin**
   and regenerate. This is the step MeloTTS could not offer, and it is why
   CosyVoice 3 is worth the setup.
4. Re-check the slow takes separately; for MeloTTS they differed from the normal
   ones (57 against 44), so they cannot be assumed.

Steps 1–4 have not been done. Nothing described in this section has been
measured yet — it is a plan, recorded so it is not re-derived.

### Running it: the environment is the real cost

Checked before committing to a 9.7 GB download, and it is the part that will cost
time. CosyVoice's `requirements.txt` pins:

| Package | Pinned | This machine |
| --- | --- | --- |
| `torch` | **2.3.1** | 2.7.0 |
| `torchaudio` | 2.3.1 | present |
| `numpy` | 1.26.4 | (2.x likely) |
| `transformers` | 4.51.3 | — |
| `onnxruntime` | 1.18.0 (darwin) | — |

Plus `lightning`, `deepspeed` (linux only), `gradio`, `tensorboard`, `openai-whisper`,
`pyworld`, `wetext`, `modelscope` and others.

**So it needs its own virtualenv.** Installing those pins over a torch 2.7
environment would break one or the other, and the model is Python-package
distributed with a git submodule (`third_party/Matcha-TTS`) rather than an ONNX
graph — it is not something the Rust `sherpa-onnx` runtime can execute the way it
executes MeloTTS. A `torch==2.3.1` CPU wheel for macOS arm64 is also not a given;
if it is unavailable, the fallback is the model card's Linux/CUDA path or an MLX
port (`mlx-community/Fun-CosyVoice3-0.5B-2512-8bit`, which is smaller and
Apple-Silicon native, though its card carries no licence metadata so its terms
need checking separately).

A zero-shot model also needs a **reference clip and its transcript** to clone a
voice. That is a decision this project has not made yet: whose voice the bundled
clips should be in, and under what licence that recording may be redistributed.
It matters because the bundled MP3s are derived from it.

**Resolved, 2026-09-21.** The environment was built and does work:

- `torch==2.3.1` **does** have a macOS-arm64 CPU wheel
  (`torch-2.3.1-cp312-none-macosx_11_0_arm64.whl`, 61 MB, from
  `download.pytorch.org/whl/cpu`) — the feared blocker was not one.
- `AutoModel` imports and loads the weights, so the Python path is viable.
- **The model is 5.1 GB, not 9.7.** `CosyVoice3` loads only `llm.pt`, `flow.pt`,
  `hift.pt`, `campplus.onnx` and `speech_tokenizer_v3.onnx`;
  `llm.rl.pt` (2.0 GB) and the `flow.decoder.estimator.fp32.onnx` copies
  (1.3 GB) are never read. It also needs `cosyvoice3.yaml` — `AutoModel`
  picks the class from that file's presence, and without it the failure is
  `TypeError: No valid model type found!`.

The exact working set, including two traps worth not rediscovering, is in
[`cosyvoice3-build-environment.txt`](cosyvoice3-build-environment.txt):

- `openai-whisper==20231117` (their pin) **will not build on Python 3.12**;
  the current release does.
- `setuptools>=81` removed `pkg_resources`, which `lightning` needs at import
  time. Pinning `setuptools<81` fixes it.

The build-time adapter is written — `scripts/cosyvoice-say.py` — including the
pinyin inpainting (`syllables` per phrase, spliced as `[j][ǐ]` control tokens).
`crates/hanzi-say` has not yet been given the backend switch that calls it, and
**nothing in this section has been run**.

### A warning the full run surfaces: prompt length

CosyVoice is zero-shot — it clones the voice from a reference clip — and it warns
on nearly every phrase:

> synthesis text 他是谁？ too short than prompt text … this may lead to bad
> performance

The 819 HSK 1–2 phrases average 9 characters; the reference clip shipped with the
repository is 12. CosyVoice expects the prompt to be *comparable in length* to
what it is asked to say, so a long prompt for a three-character phrase is a
mismatch it does not like.

**This matters for voice consistency, not just quality.** Every clip should sound
like the same speaker; if the clone drifts with phrase length, a learner drilling
a set would hear the voice change between phrases. It has not been measured yet,
because it needs listening — the one thing the automated checks in this project
cannot do.

The fix, if it is needed, is straightforward: use a **short prompt clip**, or a
per-length pair of prompts, rather than one 12-character reference for
three-character phrases. The prompt is a parameter (`--prompt-wav`,
`--prompt-text`), so this is a choice rather than a rewrite. Whose voice the
bundled clips should be in, and where that recording comes from, is still an
open decision.

## Resolution: the dataset's own audio is bundled

**Decided and done, 2026-09-21.** The bundled HSK 1-2 clips are now the dataset's
own CosyVoice2 recordings, not generated here.

The dataset publishes an MP3 for every sentence at both speeds — 8,708 files —
and its attribution file grants redistribution of that synthesised output under
**Apache-2.0**, asking only for disclosure that it is synthetic:

> 音频由 CosyVoice2-0.5B 在本地合成，**合成语音**（synthetic voice）…
> 本项目依 Apache-2.0 再分发合成输出。

Measured on the same 208 phrases with the same recogniser and judge, the
reference audio failed **15** where the MeloTTS output failed **57**. Generating
was ~4x worse than the audio that already shipped with the text.

`scripts/fetch-clip-audio.py` fetches them, resumably, into
`public/audio/no7z/` — **819 phrases, 1,638 clips, 30.2 MB, zero missing takes**.
The filenames are exactly what `synthesize-audio` produced, so the manifest, the
frontend and the gates cannot tell the difference.

### Full-scale verification, and what it does and does not say

`verify-audio` over all 819 phrases: **78 failed (9.5%)**. The 208-phrase sample
had put the reference at 15/208 (7.2%), so this is consistent rather than a
regression — but it is *not* the same kind of number as the differential, because
there is no longer a control to subtract. The 78 include the recogniser's own
errors.

The pattern says most of them are exactly that: **70 of the 78 are "dropped or
short"**, and the archetype is a number word vanishing —

    我今年二十岁      heard 我今年岁
    一年有十二个月    heard 一年有个月
    我们班有四十个学生 heard 我们班有个学生

Those are the same phrases that appeared in the **both-fail** set of the earlier
differential, where the *reference* audio failed them too. A recogniser does not
silently drop 二十 from audio that says it; this looks far more like the
recogniser's number handling than a defect in the clips. `check-audio.py` reports
78 as a failure count, not a defect rate, and should be read that way.

### The lesson worth keeping

The audio was in the corpus from the start, in the needed format, under a licence
that permitted redistribution, and measurably better than anything generated here.
Three synthesis paths were built before that was noticed — MeloTTS (22%
mispronounced), a phonetic verifier to catch it, and a 5.1 GB CosyVoice 3
environment — to regenerate audio that was already available.

The synthesis work is not wasted, because it answers a different question:
**on-device speech for a phrase with no recording**, where MeloTTS is the right
choice (53 MB, runs on the linked `sherpa-onnx`) and CosyVoice cannot play (5.1 GB,
Python-only). But for the bundled course, the existing recordings were the answer
from the beginning, and "we could generate it" was not a reason not to use them.

## Reproducing any of this

The recogniser used for every number above is the app's own, at
`.tmp-asr/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17`. The TTS
weights are pinned by `scripts/fetch-tts.sh`; the corpus by
`scripts/fetch-phrases.sh`.

```bash
# Synthesise, then screen what was produced against the source text.
scripts/with-cargo-env.sh cargo run --release -p hanzi-say --bin verify-audio -- \
    --manifest public/audio/no7z/manifest.json \
    --corpus data/phrases/no7z-sentences.jsonl \
    --asr .tmp-asr/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17
```

`verify-audio` exits non-zero when any clip is over threshold, so it can gate a
release. Its tests pin the measured failures and the clips that were fine, so the
threshold cannot be loosened without a test failing.
