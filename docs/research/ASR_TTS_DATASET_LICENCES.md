# Mandarin Chinese Audio+Transcript Dataset License Research

Target: offline desktop/mobile app, AGPL-3.0, **bundling** audio files offline, commercial-ish use.
Verdict rule: CC BY-SA / GPL / Apache-2.0 acceptable with attribution. NC, ND, research-only = **NO**.

> Method: web_search + `curl -sL`. Quotes below are verbatim from the cited URL. Claims I could not
> confirm from a primary source are marked **UNVERIFIED**.

---

## 1) KeSpeech

**Content:** 1,542 hours; 27,237 speakers; 34 cities in China; standard Mandarin + 8 subdialects.
Labels: content transcription + speaker identity + subdialect. (source: NeurIPS 2021 abstract below)
**Transcript format:** text transcription, speaker ID, subdialect ID (per abstract).

**License:** custom "KeSpeech License" (dataset_license.md) — non-commercial + no-derivatives + **no distribution**.

Verbatim (https://raw.githubusercontent.com/tzyll/KeSpeech/main/dataset_license.md):

> "Non-commercial. You may not use this datasets for any commercial purposes, including not limitation to, any ideas, plans, conducts and activities primarily intended for or directed towards commercial advantage, monetary compensation or data's exchange (even if there is no payment of monetary compensation in connection with the exchange)."

> "Technical Modifications Allowed; No Adaptations. You may use this datasets in all media and formats whether now known or hereafter created, and to make technical modifications necessary to do so, but otherwise you have no rights to make Adaptations on this datasets."

> "No Distribution. You may not distribute this dataset to any third parties; for the purposes of this license, "third parties" excludes any employees or subcontractor who work for you on a contract basis, provided that such employees and subcontractor shall comply with all the terms of this license."

README binding statement (https://raw.githubusercontent.com/tzyll/KeSpeech/main/README.md):
> "When you download the data ... you agree to the license."

**URL:** https://github.com/tzyll/KeSpeech (note: the prompt's `github.com/KeSpeech/KeSpeech` → HTTP 200 but
redirects to `github.com/tzyll/KeSpeech`, confirmed with `%{url_effective}`)
**Last-updated shown:** none on license/README.

**Correction — paper ID:** arXiv **2112.13463 is NOT the KeSpeech paper**. Verified title at
https://arxiv.org/abs/2112.13463 = *"Bilingual Speech Recognition by Estimating Speaker Geometry from Video Data"*.
The actual KeSpeech paper has no arXiv entry found (arXiv API `all:KeSpeech` returned only third-party papers
citing it); it is the NeurIPS 2021 Datasets & Benchmarks paper at
https://datasets-benchmarks-proceedings.neurips.cc/paper_files/paper/2021/hash/0336dcbab05b9d5ad24f4333c7658a0e-Abstract-round2.html
("The dataset is free for all academic usage." — note: "academic usage", not commercial).

**Verdict: NO-because-NC** (also ND and an explicit no-redistribution clause).

---

## 2) WenetSpeech (+ WenetSpeech4TTS)

**Content:** 10,000+ h high-label, 2,400+ h weak-label, ~10,000 h unlabeled, **22,400+ h total**; Mandarin,
multi-domain (YouTube + Podcast), labeled via OCR/ASR. Transcripts: text.

**OpenSLR SLR121 License field** (https://www.openslr.org/121/):
> "License: Creative Commons Attribution 4.0 International License (CC BY 4.0)"

**BUT the official project page imposes non-commercial access** (https://wenet-e2e.github.io/WenetSpeech/):
> "The WenetSpeech dataset is available to download for non-commercial purposes under a Creative Commons Attribution 4.0 International License. WenetSpeech doesn't own the copyright of the audios, the copyright remains with the original owners of the video or audio, and the public URL is given for the original video or audio."

**Access gate / agreement required** (same page):
> "Please fill out the Google Form here, checkout your mailbox, and follow the instructions to download the WenetSpeech dataset. If you fail to get the email, please write to binbzha@gmail.com."

GitHub (https://github.com/wenet-e2e/WenetSpeech): code LICENSE = Apache-2.0 (toolkit only), README states:
> "Please visit the official website, read the license, and follow the instruction to apply for the `PASSWORD` to download the data."

HF mirror `WenetspeechGroup/WenetSpeech` → **HTTP 401** (not publicly readable). `wenet-e2e/wenetspeech` exists but
`gated: "auto"`. `https://www.openslr.org/121/` has no form, but the official site does.
**Last-updated shown:** HF `wenet-e2e/wenetspeech` lastModified 2024-07-04.

**Verdict: NO-because-NC** — the operative access terms say "non-commercial purposes" even though the tag is CC BY 4.0. (Conflict noted; do not rely on the CC BY 4.0 label.)

### WenetSpeech4TTS (TTS subset)
**Content:** 12,800 hours Mandarin paired audio-text (TTS-oriented). Source: https://huggingface.co/api/datasets/Wenetspeech4TTS/WenetSpeech4TTS
Gated terms (same API URL, `cardData.extra_gated_prompt`):
> "The WenetSpeech4TTS dataset, derived from the open-source WenetSpeech dataset, is available for download for non-commercial purposes under a Creative Commons Attribution 4.0 International License."
> "1. Researcher shall use the Database only for non-commercial research and educational purposes."

Hub license tag: `license:cc-by-4.0`; `gated: "auto"`. There is **no** `wenet-e2e/WenetSpeech4TTS` GitHub repo
(404); the family index is https://github.com/wenet-e2e/WenetSpeech-Family.
**Verdict: NO-because-NC.**

---

## 3) Emilia and Emilia-YODAS (Amphion)

**Content:** Emilia = "over 101k hours of speech across six languages" (zh, en, ja, ko, de, fr), in-the-wild,
speech-generation oriented. Emilia-YODAS = 114k hours YODAS-derived (additional languages).
**Transcript format:** text + speech annotations from Emilia-Pipe.

**amphion/Emilia (original repo)** — license tag `cc-by-nc-4.0`; gated terms (https://huggingface.co/api/datasets/amphion/Emilia):
> "1. The researcher shall use the dataset ONLY for non-commercial research and educational purposes."

**amphion/Emilia-Dataset** — license tag shows `cc-by-4.0`, but the gated terms and description say otherwise.
https://huggingface.co/api/datasets/amphion/Emilia-Dataset (`cardData.extra_gated_prompt`):
> "1. The researcher shall use the Emilia dataset under the CC-BY-NC license and
>    the Emilia-YODAS dataset under the CC-BY license."

Same API URL (`description`):
> "Emilia-Large combines the original 101k-hour Emilia dataset (licensed under CC BY-NC 4.0) with the brand-new 114k-hour…"

**Emilia-YODAS:** the canonical `amphion/Emilia-YODAS` page returns **HTTP 401** (gated; API returns
`{"error":"Invalid username or password."}`), so the only verbatim public license statement is the gated prompt
above ("the Emilia-YODAS dataset under the CC-BY license", CC-BY = CC BY 4.0). A mirror
https://huggingface.co/api/datasets/TTS-AGI/emilia-yodas carries `license:cc-by-4.0`
(cardData `"license": "cc-by-4.0"`), lastModified 2025-03-08.

**Verdicts:**
- **Emilia → NO-because-NC** (CC BY-NC 4.0 + "ONLY for non-commercial research and educational purposes").
- **Emilia-YODAS → YES-with-attribution** (CC BY 4.0), *caveat*: it is distributed inside a gated repo whose
  combined Emilia-Dataset card mixing NC (Emilia) and BY (YODAS) terms; pull only the YODAS portion and attribute.
  I could not fetch the standalone YODAS card text directly (401), so the CC-BY label is from the parent card +
  an independent mirror — treat the exact standalone wording as **UNVERIFIED**.

---

## 4) GigaSpeech / GigaSpeech 2

**GigaSpeech 1** — https://github.com/SpeechColab/GigaSpeech
**Content:** **Language: English** (verbatim from README): "Language: English"; 10,000 h transcribed,
33,005 h total (audiobook/podcast/YouTube).
Code LICENSE = Apache-2.0 (toolkit). **Data** is separately gated:
README: "Step 1: Please fill out the Google Form here" (https://forms.gle/UuGQAPyscGRrUMLq6).
HF card gated terms (https://huggingface.co/api/datasets/speechcolab/gigaspeech, `extra_gated_prompt`):
> "SpeechColab does not own the copyright of the audio files. For researchers and educators who wish to use the audio files for non-commercial research and/or educational purposes, we can provide access through the Hub under certain conditions and terms."
> "1. Researcher shall use the Database only for non-commercial research and educational purposes."

**Verdict: NO-because-NC** (and no Mandarin subset exists — English only).

**GigaSpeech 2** — https://github.com/SpeechColab/GigaSpeech2 and https://huggingface.co/datasets/speechcolab/gigaspeech2
**Content:** README: "Language: Thai, Indonesian, Vietnamese"; raw 30,000 h, refined ~10k TH / 6k ID / 6k VI;
DEV/TEST 10 h per language. **No Mandarin.**
HF license tag = `apache-2.0`, but gated terms (https://huggingface.co/api/datasets/speechcolab/gigaspeech2):
> "1. Researcher shall use the Database only for non-commercial research and educational purposes."

**Last-updated shown:** HF gigaspeech2 lastModified 2026-03-26; gigaspeech lastModified 2026-02-07.
**Verdict: NO-because-NC** (and no Mandarin).

---

## 5) Microsoft MSR / Microsoft Speech Corpus Mandarin

**No "Microsoft Speech Corpus (Msr)" Mandarin resource exists on OpenSLR or HF** as far as I could verify
(searched the full OpenSLR index at https://www.openslr.org/resources.php for "Microsoft"/"MSR" — zero hits;
HF searches for `msr`, `msr-cn`, `mandarin-read`, `microsoft-chinese`, `microsoft speech` returned no Mandarin
speech corpus). Mark **UNVERIFIED / NOT FOUND**.

Closest Microsoft corpus: **MSR-86K** (Interspeech 2024, https://www.isca-archive.org/interspeech_2024/li24s_interspeech.pdf):
86,300 h, 15 languages derived from YouTube. HF: https://huggingface.co/api/datasets/Alex-Song/MSR-86K —
`gated: "manual"`, `license:cc-by-nc-nd-4.0`. The 15 language tags are
`es, ko, en, fr, de, hi, vi, it, nl, pt, th, ru, id, ja, ar` — **Chinese/Mandarin is NOT among them** (the
"Chinese" rows in the paper are comparison/eval corpora such as AISHELL/WenetSpeech, not MSR-86K data).
**Verdict: NO-because-NC** (and no Mandarin).

**Microsoft Scalable Noisy Speech Dataset (MS-SNSD)** — https://github.com/microsoft/MS-SNSD: **English only**
(clean speech from PTDB-TUG and the Edinburgh 56-speaker set, plus noise). Code LICENSE = MIT.
Data terms verbatim (README): "The datasets are provided under the original terms that Microsoft received such
datasets." (PTDB-TUG ODbL, Edinburgh license, Freesound CC0, DEMAND CC BY-SA 3.0).
**Verdict: NO** for this task (no Mandarin); its own code is MIT but the audio is a mix.

---

## 6) Fisher Chinese / LDC, and HKUST/MTS

**LDC2010S05 is NOT Fisher Mandarin.** Verified title at https://catalog.ldc.upenn.edu/LDC2010S05 =
**"Asian Elephant Vocalizations"** (57.5 h elephant audio). The prompt's assumption is wrong.

**I found no LDC catalog entry for a "Fisher Mandarin/Chinese" corpus.** LDC Fisher sets I verified:
- LDC2004S13 "Fisher English Training Speech Part 1 Speech" (984 h English)
- LDC2005S13 "Fisher English Training Part 2, Speech"
- LDC2010S01 "Fisher Spanish Speech"
So "Fisher Mandarin" is **NOT FOUND / UNVERIFIED** at LDC (searched catalog IDs and search pages; the catalog
search UI is JS-only and DuckDuckGo `site:catalog.ldc.upenn.edu fisher chinese` returned nothing).

**HKUST Mandarin Telephone Speech, Part 1 — LDC2005S15** (https://catalog.ldc.upenn.edu/LDC2005S15):
"approximately 149 hours of conversational telephone speech (CTS) in Mandarin". License field:
> "License(s): LDC User Agreement for Non-Members"

Decisive clauses (https://catalog.ldc.upenn.edu/license/ldc-non-members-agreement.pdf):
> "User agrees to use the LDC Databases received under this Agreement only for noncommercial linguistic education, research and technology development. In the event that User's use of the LDC Databases results in the development of a commercial product, User must join LDC as a For-Profit Member and pay all applicable fees prior to release of said commercial product."

> "Unless explicitly permitted herein, User shall not otherwise publish, retransmit, disclose, display, copy, reproduce or redistribute the LDC Databases to others outside of User's Research Group. User shall have no right to copy, redistribute, transmit, publish or otherwise use the LDC Databases for any other purpose."

**Verdict: NO** (non-commercial + explicit no-redistribution; non-member/for-profit membership fees required).

---

## 7) zhvoice

**Content:** merges 8 open-source Chinese corpora after denoise/silence removal; ~3,200 speakers, **~900 hours**,
~1.13M utterances, ~13M characters. Files: `metadata.csv` (path\ttext), per-speaker mp3 dirs, text/pinyin.
Source README: https://raw.githubusercontent.com/fighting41love/zhvoice/master/README.md

**License: NO LICENSE FOUND AT**
- https://github.com/fighting41love/zhvoice (default branch `master`; repo contains only `README.md` + `zhvoice.png`; no LICENSE file, no license badge, no license/copyright statement in the README)
- https://raw.githubusercontent.com/fighting41love/zhvoice/master/LICENSE
- https://raw.githubusercontent.com/fighting41love/zhvoice/master/README.md (grep for licen*/许可/协议/商业/版权 = 0 hits)
- https://github.com/KuangDD/zhvoice → **HTTP 404** (repo referenced by the README does not exist)
- https://raw.githubusercontent.com/KuangDD/zhvoice/master/LICENSE and .../README.md → 404

**Last-updated shown:** none.
**Verdict: UNCLEAR (no license → cannot bundle).**

---

## 8) DiDiSpeech

**Content:** ~800 h, 48 kHz, 6,000 speakers, Mandarin, with texts (DiDiSpeech-1 = 572 h / 4,500 speakers;
DiDiSpeech-2 = 1,500 speakers). Paper: https://arxiv.org/abs/2010.09275

**License: NO LICENSE FOUND AT**
- https://arxiv.org/abs/2010.09275 (paper states availability only: "The corpus is available at https://outreach.didichuxing.com/research/opendata/."; no license field)
- https://raw.githubusercontent.com/athena-team/DiDiSpeech/master/README.md (audio-samples repo; no license section, no LICENSE file)
- https://athena-team.github.io/DiDiSpeech/ (no license)
- https://outreach.didichuxing.com/research/opendata/ (now redirects to the outreach portal / "盖亚开放数据计划"; the
  archived 2021-12-10 copy at web.archive.org is a login/registration-gated JS app — no license text retrievable)
- OpenDataLab: no `DiDiSpeech` dataset page found (searches returned nothing)

**Verdict: UNCLEAR (no license found → cannot bundle).** The paper's own wording ("suitable for ... practical
application") is not a license — do not rely on it.

---

## 9) CML-TTS (OpenSLR SLR146)

**Content:** 3,233.43 hours, 613 speakers, audiobooks from LibriVox, 24 kHz.
**Languages (verbatim, paper):** "consisting of audiobooks in seven languages: Dutch, French, German, Italian, Portuguese, Polish, and Spanish."
**Chinese is NOT included** — confirmed by both the paper and the OpenSLR file list (only
dutch/french/german/italian/polish/portuguese/spanish tarballs).
**Transcript format:** sentence-level text (punctuated; derived from MLS + Gutenberg alignment).

**License** — OpenSLR (https://www.openslr.org/146/):
> "License: CC-BY 4.0 license"
Paper (https://ar5iv.labs.arxiv.org/html/2306.10097):
> "The dataset is publicly available under the CC-BY 4.0 license"

**Verdict: YES-with-attribution** (CC BY 4.0, share-alike-compatible with AGPL, commercial OK) — **but no Mandarin
content, so not usable for this app's Mandarin dataset.**

---

## 10) Magicoder / MagicData

**"Magicoder" in a speech context is a conflation.** **Magicoder** is a *code-generation LLM*, not a speech dataset:
https://github.com/ise-uiuc/magicoder — LICENSE file verbatim:
> "MIT License
> Copyright (c) 2023 iSE-UIUC"
However, its released **models** are under Llama2 / DeepSeek model licenses (README model table), not MIT.
**Not a Mandarin speech dataset → not applicable.**

**MagicData (Beijing Magic Data) open corpora on OpenSLR — all CC BY-NC-ND 4.0:**
- **SLR68 — MAGICDATA Mandarin Chinese Read Speech Corpus**, 755 h, 1,080 speakers (https://www.openslr.org/68/):
  > "License: Attribution-NonCommercial-NoDerivatives 4.0 International Public License (CC BY-NC-ND 4.0)"
- **SLR123 — MAGICDATA Mandarin Chinese Conversational Speech Corpus (MagicData-RAMC)**, 180 h (https://www.openslr.org/123/):
  > "License: Attribution-NonCommercial-NoDerivatives 4.0 International Public License (CC BY-NC-ND 4.0)"
- **SLR47 — Primewords Chinese Corpus Set 1**, 100 h (https://www.openslr.org/47/):
  > "License: Attribution-NonCommercial-NoDerivatives 4.0 International (CC BY-NC-ND 4.0)"

MagicHub (https://magichub.com/) is a JS SPA; I could not retrieve any license text from it — **UNVERIFIED**
for MagicHub-made corpora; the OpenSLR-published MagicData corpora above are the verifiable ones.
**Verdict: NO-because-NC (and ND) for all three.**

---

## 11) Other requested corpora

- **AISHELL-1** — OpenSLR SLR33 (https://www.openslr.org/33/): > "License: Apache License v.2.0";
  HF https://huggingface.co/api/datasets/AISHELL/AISHELL-1 → `license:apache-2.0`, `gated: false`,
  lastModified 2024-01-08. **Verdict: YES-with-attribution.**
  (AISHELL-3 SLR93 = "License: Apache License v.2.0" too; AISHELL-4 SLR111 and HI-MIA SLR85 also Apache-2.0 on HF.)
- **ST-CMDS** — OpenSLR SLR38 (https://www.openslr.org/38/), "A free Chinese Mandarin corpus by Surfingtech
  (www.surfing.ai), containing utterances from 855 speakers, 102600 utterances":
  > "License: Creative Common BY-NC-ND 4.0 (Attribution-NonCommercial-NoDerivatives 4.0 International)"
  **Verdict: NO-because-NC (and ND).**
- **THCHS-30** — OpenSLR SLR18 (https://www.openslr.org/18/), "A Free Chinese Speech Corpus Released by
  CSLT@Tsinghua University":
  > "License: Apache License v.2.0"
  **Verdict: YES-with-attribution.**
- **aidatatang_200zh** — `https://www.openslr.org/62/` now literally shows "Resource not found: 62".
  It still lives on the MagicData mirror **https://openslr.magicdatatech.com/62/**:
  > "Identifier: SLR62 ... Summary: A Chinese Mandarin speech corpus by Beijing DataTang Technology Co., Ltd, containing 200 hours of speech data from 600 speakers. The transcription accuracy for each sentence is larger than 98%."
  > "License: Attribution-NonCommercial-NoDerivatives 4.0 International (CC BY-NC-ND 4.0)"
  > "About this resource: Resource retracted as per the data owner wish."
  Corroboration — Kaldi recipe (https://raw.githubusercontent.com/kaldi-asr/kaldi/master/egs/aidatatang_200zh/README.md):
  > "Aidatatang_200zh is a free Chinese Mandarin speech corpus provided by Beijing DataTang Technology Co., Ltd under Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International Public License."
  MFA corpus card (https://raw.githubusercontent.com/MontrealCorpusTools/mfa-models/main/corpus/mandarin/ai_datatang_corpus/README.md):
  200.38 h, 237,265 utterances, 600 speakers, "**License:** CC BY-NC-ND 4.0".
  **Verdict: NO-because-NC (prompt's belief — CC BY-NC-ND 4.0 — is CONFIRMED).** Note the resource is **retracted**,
  so even the NC license may be moot; site says get it from datatang.com.

---

## Summary table

| # | Dataset | License | Verdict | URL |
|---|---------|---------|---------|-----|
| 1 | KeSpeech (1,542 h, 27,237 spk, Mandarin+8 subdialects) | Custom: non-commercial, no adaptations, **no distribution** | **NO-because-NC** | https://raw.githubusercontent.com/tzyll/KeSpeech/main/dataset_license.md |
| 2 | WenetSpeech (22,400+ h total; Mandarin) | Tagged CC BY 4.0 but official terms "for **non-commercial** purposes"; Google Form + password | **NO-because-NC** | https://wenet-e2e.github.io/WenetSpeech/ ; https://www.openslr.org/121/ |
| 2b | WenetSpeech4TTS (12,800 h Mandarin) | "non-commercial purposes under a CC BY 4.0"; terms "only for non-commercial research and educational purposes" | **NO-because-NC** | https://huggingface.co/api/datasets/Wenetspeech4TTS/WenetSpeech4TTS |
| 3 | Emilia (101k h, 6 langs, incl. zh) | CC BY-NC 4.0 + "ONLY for non-commercial research and educational purposes" | **NO-because-NC** | https://huggingface.co/api/datasets/amphion/Emilia |
| 3b | Emilia-YODAS (114k h YODAS) | CC BY 4.0 ("the Emilia-YODAS dataset under the CC-BY license") | **YES-with-attribution** | https://huggingface.co/api/datasets/amphion/Emilia-Dataset ; mirror https://huggingface.co/api/datasets/TTS-AGI/emilia-yodas |
| 4 | GigaSpeech (10,000 h, **English only**) | Code Apache-2.0; data "non-commercial research and educational purposes" + Google Form | **NO-because-NC** (also no Mandarin) | https://huggingface.co/api/datasets/speechcolab/gigaspeech |
| 4b | GigaSpeech 2 (TH/ID/VI only) | Tag apache-2.0; gated "only for non-commercial research and educational purposes" | **NO-because-NC** (also no Mandarin) | https://huggingface.co/api/datasets/speechcolab/gigaspeech2 |
| 5 | Microsoft MSR Mandarin | **NO LICENSE FOUND** (no such corpus located) | **UNCLEAR / not found** | https://www.openslr.org/resources.php ; https://huggingface.co/api/datasets?search=mandarin |
| 5b | MSR-86K (86,300 h, **no Chinese** among 15 langs) | cc-by-nc-nd-4.0, manual gated | **NO-because-NC** | https://huggingface.co/api/datasets/Alex-Song/MSR-86K |
| 5c | MS-SNSD (English only) | Code MIT; data "under the original terms that Microsoft received" | **NO** (no Mandarin) | https://github.com/microsoft/MS-SNSD |
| 6 | LDC2010S05 | **Asian Elephant Vocalizations** — not Fisher Mandarin | **NO / wrong ID** | https://catalog.ldc.upenn.edu/LDC2010S05 |
| 6b | Fisher Mandarin | **NO LICENSE FOUND / NOT FOUND** at LDC (LDC Fisher = English + Spanish only) | **UNCLEAR / not found** | https://catalog.ldc.upenn.edu/LDC2004S13 ; https://catalog.ldc.upenn.edu/LDC2010S01 |
| 6c | HKUST Mandarin Telephone Speech (LDC2005S15, ~149 h) | LDC User Agreement for Non-Members: "only for noncommercial linguistic education, research and technology development"; no redistribution | **NO-because-NC** | https://catalog.ldc.upenn.edu/license/ldc-non-members-agreement.pdf |
| 7 | zhvoice (~900 h, ~3,200 spk) | **NO LICENSE FOUND** | **UNCLEAR** | https://github.com/fighting41love/zhvoice |
| 8 | DiDiSpeech (~800 h, 6,000 spk) | **NO LICENSE FOUND** | **UNCLEAR** | https://github.com/athena-team/DiDiSpeech |
| 9 | CML-TTS (3,233 h, 7 langs) | CC-BY 4.0 | **YES-with-attribution, but NO Chinese** | https://www.openslr.org/146/ |
| 10 | Magicoder (code LLM) | MIT (repo); Llama2 / DeepSeek (models) | **N/A — not speech** | https://github.com/ise-uiuc/magicoder |
| 10b | MAGICDATA Read SLR68 (755 h) | CC BY-NC-ND 4.0 | **NO-because-NC** | https://www.openslr.org/68/ |
| 10c | MAGICDATA/RAMC SLR123 (180 h) | CC BY-NC-ND 4.0 | **NO-because-NC** | https://www.openslr.org/123/ |
| 10d | Primewords SLR47 (100 h) | CC BY-NC-ND 4.0 | **NO-because-NC** | https://www.openslr.org/47/ |
| 11 | AISHELL-1 (178 h, Mandarin) | Apache-2.0 | **YES-with-attribution** | https://www.openslr.org/33/ ; https://huggingface.co/api/datasets/AISHELL/AISHELL-1 |
| 11b | ST-CMDS SLR38 | CC BY-NC-ND 4.0 | **NO-because-NC** | https://www.openslr.org/38/ |
| 11c | THCHS-30 SLR18 | Apache-2.0 | **YES-with-attribution** | https://www.openslr.org/18/ |
| 11d | aidatatang_200zh (200 h, 600 spk) | CC BY-NC-ND 4.0; **resource retracted** | **NO-because-NC** | https://openslr.magicdatatech.com/62/ |

## Bottom line for an AGPL-3.0 offline bundling app

**Bundleable with attribution (Mandarin, permissive/share-alike):**
- **AISHELL-1** (Apache-2.0), **THCHS-30** (Apache-2.0) — Apache-2.0 is AGPL-compatible; keep NOTICE/attribution.
- **Emilia-YODAS** (CC BY 4.0) — attribution only; verify standalone card (currently 401).

**Disqualified:** KeSpeech (NC+ND+no-dist), WenetSpeech & WenetSpeech4TTS (NC terms), Emilia (NC), GigaSpeech/2 (NC),
MSR-86K (NC-ND), all LDC/Fisher/HKUST (NC + no redistribution), ST-CMDS / aidatatang / primewords / MAGICDATA (NC-ND).

**No license at all → do not bundle:** zhvoice, DiDiSpeech.
