# Mandarin Audio+Transcript Dataset License Report
### For: offline desktop/mobile app, AGPL-3.0, bundling audio + text inside the app (commercial-ish)

Research date: **2026-09-21** (all pages, export files and APIs fetched on this date; export counts are from the Tatoeba weekly export of the preceding Saturday). Every quote below is verbatim from the URL given. Items that could not be confirmed are marked **UNVERIFIED** and collected in §11.
Legend for verdict: **YES** = bundle & redistribute OK · **YES-attr** = OK with attribution · **NO-NC** = non-commercial, disqualified · **NO-ND** = no-derivatives, disqualified · **NO-other** = disqualified for another stated reason · **UNCLEAR** = conflicting statements, do not rely on it.

---

## 1. Summary table

| Dataset | Content / count | License (as stated) | Redistribution verdict | URL |
|---|---|---|---|---|
| **CSS10 Chinese** ⭐ | 6:27:04 audio + aligned text, 1 speaker (Jing Li), 2.04 GB | **CC0 1.0** ("CC0: Public Domain") | **YES** | https://www.kaggle.com/datasets/bryanpark/chinese-single-speaker-speech-dataset |
| **LibriVox (zh)** | 27 items, ≈125 h audio; **no transcripts** | Public domain (US) dedication | **YES** (audio) | https://librivox.org/pages/public-domain/ |
| **Tatoeba cmn — TEXT** | 89,065 Mandarin sentences | CC BY 2.0 FR (default) | **YES-attr** | https://tatoeba.org/en/downloads |
| **Tatoeba cmn — AUDIO** | 5,826 clips; 5,742 = "No license for offsite use", 84 = CC BY-NC 4.0 | per-clip; mostly none | **NO-other** (98.5% no offsite license; rest NC) | https://downloads.tatoeba.org/exports/per_language/cmn/ |
| **Mozilla Common Voice zh-CN** | 852,410 clips, 1,074.96 h (240.11 h validated), 7,601 spk | CC0-1.0 **but** datasheet forbids re-hosting | **NO-other** (platform prohibition, not the CC0) | https://mozilladatacollective.com/datasets/cmu5lcunh00qimh078sdd5so8 |
| Mozilla Common Voice zh-TW | 131.7 h recorded (79.92 h validated), 2,336 spk, 22,778 sentences | CC0-1.0 + same prohibition | **NO-other** | https://mozilladatacollective.com/datasets/cmu5wkw5i00c6o107jnm3t51u |
| Mozilla Common Voice zh-HK | 124,379 clips, 143.39 h (108.56 h validated), 3,114 spk | CC0-1.0 + same prohibition | **NO-other** | https://mozilladatacollective.com/datasets/cmu5wp2bp00dgnq07ylmgbnrz |
| Mozilla Common Voice yue (Cantonese) | 279,396 clips, 307.44 h (210.78 h validated), 1,188 spk | CC0-1.0 + same prohibition | **NO-other** | https://mozilladatacollective.com/datasets/cmu5x3i5w00d6o107yvwox9lg |
| **AISHELL-1** (SLR33) | 178 h, 400 spk, transcripts | OpenSLR "Apache License v.2.0" **vs** vendor "Commercial use is forbidden" | **UNCLEAR** (treat as NC) | https://www.openslr.org/33/ |
| **AISHELL-2** | 1000 h, 1991 spk | vendor: "free for academic research, not in the commerce" | **NO-NC** | http://web.archive.org/web/20180602070422/http://www.aishelltech.com/aishell_2 |
| **AISHELL-3** (SLR93) | 85 h, 88,035 utts, 218 spk, char+pinyin | OpenSLR "Apache License v.2.0" **vs** vendor "...not in the commerce" | **UNCLEAR** (treat as NC) | https://www.openslr.org/93/ |
| **AISHELL-4** (SLR111) | 211 meetings, 120 h, 8-ch | **CC BY-SA 4.0** | **YES-attr** | https://www.openslr.org/111/ |
| AISHELL-5 (SLR159) | in-car multi-channel | CC BY-SA 4.0 | **YES-attr** | https://www.openslr.org/159/ |
| **MAGICDATA Read Speech** (SLR68) | 755 h, 1080 spk | CC BY-NC-ND 4.0 | **NO-NC + NO-ND** | https://www.openslr.org/68/ |
| MAGICDATA Conversational (SLR123) | 180 h | CC BY-NC-ND 4.0 | **NO-NC + NO-ND** | https://www.openslr.org/123/ |
| **Primewords Set 1** (SLR47) | 100 h, 296 spk | **CC BY-NC-ND 4.0** (not CC BY-SA) | **NO-NC + NO-ND** | https://www.openslr.org/47/ |
| **aidatatang_200zh** (SLR62) | 200.38 h, 237,265 utts, 600 spk | CC BY-NC-ND 4.0 · **resource retracted** | **NO-other** (retracted + NC-ND) | https://web.archive.org/web/2024id_/https://www.openslr.org/62/ |
| **ST-CMDS** (SLR38) | 855 spk, 102,600 utts | CC BY-NC-ND 4.0 | **NO-NC + NO-ND** | https://www.openslr.org/38/ |
| **THCHS-30** (SLR18) | 30 h+ (6.4 G audio) | "Apache License v.2.0" but "totally free to academic users" | **UNCLEAR** (leaning YES-attr) | https://www.openslr.org/18/ |
| AliMeeting (SLR119) | 120 h meetings, multi-ch | CC BY-SA 4.0 | **YES-attr** | https://www.openslr.org/119/ |
| CN-Celeb (SLR82) | speaker-recognition corpus | CC BY-SA 4.0 | **YES-attr** | https://www.openslr.org/82/ |
| HI-MIA (SLR85) | far-field wake-word | Apache 2.0 | **YES-attr** | https://www.openslr.org/85/ |
| MobvoiHotwords (SLR87) | hotword detection | Apache 2.0 | **YES-attr** | https://www.openslr.org/87/ |
| SHALCAS22A (SLR138) | Mandarin corpus | CC BY-NC-ND 4.0 | **NO-NC + NO-ND** | https://www.openslr.org/138/ |
| **WenetSpeech** (SLR121) | 10,000+ h | OpenSLR says CC BY 4.0; **official site says non-commercial** | **NO-NC** | https://wenet-e2e.github.io/WenetSpeech/ |
| **KeSpeech** | 1,542 h, 27,237 spk, Mandarin + 8 subdialects | Non-commercial + **No Distribution** + No Adaptations | **NO-NC + NO-other** | https://raw.githubusercontent.com/tzyll/KeSpeech/main/dataset_license.md |
| **GigaSpeech** | **English only**, 10,000 h transcribed | HF tag apache-2.0; gating = non-commercial research only | **NO-NC** (+ no Mandarin) | https://huggingface.co/api/datasets/speechcolab/gigaspeech |
| **GigaSpeech 2** | Thai / Indonesian / Vietnamese — **no Chinese** | HF tag apache-2.0; gating = non-commercial research only | **NO-NC** (+ no Mandarin) | https://huggingface.co/api/datasets/speechcolab/gigaspeech2 |
| **WenetSpeech4TTS** | 12,800 h Mandarin TTS | "non-commercial purposes under a Creative Commons Attribution 4.0" | **NO-NC** | https://huggingface.co/api/datasets/Wenetspeech4TTS/WenetSpeech4TTS |
| **Emilia** | 101k h, 6 langs incl. zh | CC BY-NC 4.0; gated "ONLY for non-commercial research and educational purposes" | **NO-NC** | https://huggingface.co/api/datasets/amphion/Emilia |
| **Emilia-YODAS** | multi-lang incl. zh | **CC BY 4.0** (per gating terms) | **YES-attr** (gated acceptance) | https://huggingface.co/api/datasets/amphion/Emilia-Dataset |
| **CML-TTS** (SLR146) | 3,233.43 h, 613 spk, 7 languages | CC BY 4.0 | **NO-other** — **no Chinese in the corpus** | https://www.openslr.org/146/ |
| MagicData Dialect TTS-Lite (Cantonese/Sichuanese/Wu/Northeastern/Henan) | ~10 min each, 1 speaker | HF tag `apache-2.0` **vs** README "CC BY-NC-ND 4.0" | **UNCLEAR → assume NO-NC-ND** | https://huggingface.co/datasets/MagicDataTech/magicdata-dialect-cantonese-tts-lite |
| **MSR / Microsoft Speech Corpus (Msr)** | **does not exist** for Mandarin | NO LICENSE FOUND | **not found** | (see §9.3) |
| MSR-86K | 86,300 h, 15 langs — **no Chinese** | cc-by-nc-nd-4.0, gated manual | **NO-NC** (+ no Mandarin) | https://huggingface.co/api/datasets/Alex-Song/MSR-86K |
| MS-SNSD | English only | code MIT, data "original terms Microsoft received" | **NO** (no Mandarin) | https://github.com/microsoft/MS-SNSD |
| **Fisher Chinese** | **no LDC Fisher Mandarin corpus exists** | — | **not found** | (see §9.3) |
| **HKUST Mandarin Telephone Speech** (LDC2005S15) | ~149 h Mandarin CTS | LDC User Agreement for Non-Members | **NO-NC** (+ no redistribution) | https://catalog.ldc.upenn.edu/license/ldc-non-members-agreement.pdf |
| zhvoice | ~900 h, ~3,200 spk, ~1.13M utts | **NO LICENSE FOUND** | **UNCLEAR** | https://github.com/fighting41love/zhvoice |
| DiDiSpeech | ~800 h, 48 kHz, 6,000 spk | **NO LICENSE FOUND** | **UNCLEAR** | https://github.com/athena-team/DiDiSpeech |
| Magicoder | **code LLM, not a speech dataset** (MIT) | MIT | **n/a** | https://github.com/ise-uiuc/magicoder |

**Practical recommendation for the app:** the only clean, low-risk *learner-oriented* audio+text bundle found is **CSS10 Chinese (CC0)**. For text-only drills, **Tatoeba cmn** (CC BY 2.0 FR). For multi-speaker/read speech, **AISHELL-4 / AliMeeting (CC BY-SA 4.0)** are the only large corpora whose stated license is unambiguous — but their transcripts are *meeting* speech. Everything else is NC/ND/retracted or contradicts itself.

---

## 2. Mozilla Common Voice (zh-CN / zh-TW / zh-HK) and Mozilla Data Collective

### 2.1 Licensing
Every Chinese Common Voice datasheet on MDC states the **same** license and the **same** prohibition. Verbatim from the zh-CN datasheet (https://mozilladatacollective.com/datasets/cmu5lcunh00qimh078sdd5so8):

> **Licensing** — `Creative Commons Zero v1.0 Universal (CC0-1.0)` — `https://spdx.org/licenses/CC0-1.0.html`
>
> **Restrictions/Special Constraints** — `None provided.`
>
> **Forbidden Usage** — `It is forbidden to attempt to determine the identity of speakers in the Common Voice datasets. It is forbidden to re-host or re-share this dataset.`
>
> **Intended Use** — `This dataset is intended to be used for training and evaluating automatic speech recognition (ASR) models. It may also be used for applications relating to computer-aided language learning (CALL) and language or heritage revitalisation.`

Identical wording appears on the zh-TW (`cmu5wkw5i00c6o107jnm3t51u`), zh-HK (`cmu5wp2bp00dgnq07ylmgbnrz`) and yue (`cmu5x3i5w00d6o107yvwox9lg`) datasheets.

This is reinforced by MDC's platform terms, Appendix 1 (https://mozilladatacollective.com/terms):

> `Dataset Misuse and Enforcement. Data Consumer shall not, and shall not permit any third party to, bypass or attempt to bypass any security, technical, or access controls relating to the Dataset; scrape, mirror, or redistribute the Dataset; or otherwise misuse the Dataset.`

And by MDC's own FAQ, *"FAQ: Why can't I re-host or share Common Voice datasets that I download from MDC?"* (https://community.mozilladatacollective.com/faq-why-cant-i-re-host-or-share-common-voice-datasets-that-i-download-from-mdc/, 06 Oct 2025):

> `CC0 remains the license for computational use, whilst not allowing mirroring the datasets is a platform term.`

### 2.2 Verdict: **NO** (with an important legal nuance)
- The **data** is dedicated to CC0-1.0. CC0 is a public-domain dedication; a downstream platform cannot attach new restrictions *to the licensed data itself*.
- But **downloading from MDC requires an account and acceptance of the MDC ToS**, and both the datasheet and the ToS forbid re-hosting/re-sharing/mirroring. So a bundle sourced from MDC is a **contract breach even though the license is CC0**.
- Practical consequence: **do not plan a bundle that depends on MDC.** If you need Common Voice, you must source it from a pre-MDC or third-party mirror under the pure CC0 grant (see §2.5) — and accept that MDC's reading is that this is not intended.

### 2.3 Downloads now require an account
- `https://commonvoice.mozilla.org/en/datasets` is a JS app (returns `Please enable JavaScript to run this app`); it no longer serves direct bundle links. The legacy bundler URLs are dead:
  - `https://voice-prod-bundler-ee1969a6ce8178826482b88e843c335139bd3fb4.s3.amazonaws.com/cv-corpus-19.0-2024-09-13/zh-CN.tar.gz` → **HTTP 403 Forbidden**
  - `https://storage.googleapis.com/common-voice-prod-prod-datasets/cv-corpus-19.0-2024-09-13/zh-CN.tar.gz` → **HTTP 403**
- MDC requires sign-up/log-in ("Sign up / Log in", `Data Consumer Account`, ToS acceptance, `In order to access or download a Dataset, you must first review and accept the applicable Data Consumer License`).

### 2.4 Counts (cv-corpus-27.0-2026-09-11 / CC0 MDC datasheets + CV stats API)
| Locale | Clips | Recorded h | Validated h | Speakers | Sentences | Size |
|---|---|---|---|---|---|---|
| zh-CN | 852,410 | 1,074.96 | **240.11** | 7,601 | 60,191 | 21.40 GB |
| zh-TW | — | 131.7 | **79.92** | 2,336 | 22,778 | 3.17 GB |
| zh-HK | 124,379 | 143.39 | **108.56** | 3,114 | 20,209 | 3.68 GB |
| yue (Cantonese) | 279,396 | 307.44 | **210.78** | 1,188 | 28,727 | 6.43 GB |
| zh-CN-beijing (v26 subset) | 2,860 | ≈4.33 | — | — | — | 118.9 MB |
| zh-CN-shanghai / Wu (v26 subset) | 5,102 | ≈7.2 | — | — | — | 200.5 MB |
| zh-CN Spontaneous Speech 5.0 | 1 | ~0 | 0 | 1 | — | 0.12 MB |

Source for hours: `https://commonvoice.mozilla.org/api/v1/stats/languages` (zh-CN `recordedHours: 1095, validatedHours: 240`; zh-TW `132/78`; zh-HK `144/109`; yue `309/204`; `lastFetched 2026-09-20`).

**Transcripts ship as TSV alongside the audio — confirmed.** The MDC v26 subset datasheets describe the layout verbatim:

> `The dataset follows the Mozilla Common Voice format: The clips directory contains all of the .mp3 files, and there is a separate tsv file for each data partition, containing the following fields: client_id, path, sentence_id, sentence, sentence_domain, up_votes, down_votes, age, gender, accents, variant, locale, segment`

### 2.5 CC0 mirrors (not MDC)
| Mirror | Locale tags | License tag | Gated | Note |
|---|---|---|---|---|
| `legacy-datasets/common_voice` | includes `zh-CN`, `zh-HK`, `zh-TW` | `cc0-1.0` | no | last modified 2024-08-22; `viewer: false` |
| `fsicoli/common_voice_22_0` | `yue`, `zh` | `cc0-1.0` | no | |
| `OpenFormosa/common_voice_25_zh-TW` | `zh` (Taiwan) | `cc0-1.0` | no | |
| `mozilla-foundation/common_voice_17_0` | — | **no license in cardData** | no | last modified 2025-10-24; card lists only `task_categories` |
| `deepdml/common_voice_26_0` | `yue`, `zh` | — | **gated manual** | |

These mirrors are third-party re-hosts. Their CC0 tag is consistent with the original grant, but **they are exactly what MDC's FAQ objects to**, and none of them carries an independent warranty. **UNVERIFIED:** whether any mirror still serves the *validated* splits you'd need, and whether their contents are current.

---

## 3. Tatoeba (Mandarin Chinese = `cmn`)

### 3.1 Sentences — license (usable)
https://tatoeba.org/en/downloads states verbatim:

> `Creative commons — These files are released under CC BY 2.0 FR.`
> `A part of our sentences are also available under CC0 1.0.`

https://en.wiki.tatoeba.org/articles/show/using-the-tatoeba-corpus:

> `Tatoeba's technical infrastructure uses the default Creative Commons Attribution 2.0 France license (CC-BY 2.0 FR) for the use of textual sentences. The BY mention implies a single restriction on the use, reuse, modification and distribution of the sentence: a condition of attribution.`

Terms of Use §6.2 (https://tatoeba.org/en/terms_of_use) repeats it and adds:

> `We are not generally opposed to using our content for commercial purposes. However, this choice depends primarily on contributors. Certain phrases, in particular audio, may be contributed with a non-marketing condition, and therefore must not be marketed.`

**Measured counts** (weekly export `cmn_sentences.tsv.bz2`, pulled from `https://downloads.tatoeba.org/exports/per_language/cmn/`):
- **89,065** Mandarin sentences total.
- Per-sentence license tags **do exist** (the `sentences_detailed` export carries a license column and a date column).
- **CC0 for Mandarin is effectively nil:** `cmn_sentences_CC0.tsv.bz2` is **102 bytes = 1 sentence**. So the "some CC0" claim is technically true but useless for zh.

**Attribution (verbatim, from https://en.wiki.tatoeba.org/articles/show/faq):**
> `Basically you just need to write somewhere that some/all of your sentences are from Tatoeba, with a link to https://tatoeba.org, and mention that Tatoeba's data is released under CC-BY 2.0 FR.`

**Verdict for Tatoeba cmn TEXT: YES-attr** (CC BY 2.0 FR; commercial use not opposed).

### 3.2 Audio — license (NOT usable) ⚠️
The same FAQ is explicit that audio is different:

> `Our audio corpus has a wider range of licenses and isn't just restricted to CC-BY. You should therefore be more careful about which audio you are using, especially if your project/app is commercial. You can check the license of each audio recording from the file we release under "Sentences with audio" on our Downloads page.`

The wiki adds:

> `Note that the terms of use for the audio files are not the same as for the text of sentences. ... You should verify these licenses by clicking "audio files" on each member's profile.`

**Decisive measurement.** The export `cmn_sentences_with_audio.tsv.bz2` has columns `sentence_id, audio_id, username, license, attribution_url`. For Mandarin:

| License value in export | Count | Share |
|---|---|---|
| *(empty — no offsite license)* | **5,742** | 98.5% |
| `CC BY-NC 4.0` | **84** | 1.4% |
| **Total** | **5,826** | 100% |

Contributors: `LeviHighway` 4,066 · `fucongcong` 1,676 · `GlossaMatik` 69 · `zhoucantd` 15.

The empty value is **not** an accident — Tatoeba's UI maps it explicitly. From the sentence page for cmn sentence 1 (audio id 1276691, LeviHighway), the inline license options object is verbatim:

> `{"":{"name":"No license for offsite use"},"CC0 1.0":{"url":"https://creativecommons.org/publicdomain/zero/1.0/"},"CC BY 4.0":{...}`

and that recording's own metadata is `"license":""`.

So: **5,742 of 5,826 Mandarin recordings (98.5%) are "No license for offsite use"** → cannot be redistributed off tatoeba.org at all. The remaining **84 are CC BY-NC 4.0** → disqualified by the NC term.

**Verdict for Tatoeba cmn AUDIO: NO.** There is no meaningful pool of CC0/CC-BY Mandarin audio on Tatoeba.

### 3.3 Bulk export locations
- Root: `https://downloads.tatoeba.org/exports/` — `sentences.csv` (757,736,270 B), `sentences_detailed.csv` (1,406,540,711 B), `sentences_CC0.csv` (41,850,592 B), `sentences_with_audio.csv` (71,715,888 B), `sentences_base.csv`, `links.csv`, `tags.csv`, `transcriptions.tar.bz2`, `user_languages.csv`, `wwwjdic.csv`, `jpn_indices.csv`.
- Per-language Mandarin: `https://downloads.tatoeba.org/exports/per_language/cmn/` — `cmn_sentences.tsv.bz2`, `cmn_sentences_detailed.tsv.bz2`, `cmn_sentences_CC0.tsv.bz2`, `cmn_sentences_with_audio.tsv.bz2`, `cmn_tags.tsv.bz2`, `cmn_transcriptions.tsv.bz2`, plus `cmn-<lang>_links.tsv.bz2` pairs.
- Updated **every Saturday at 6:30 a.m. UTC** (stated on the downloads page).
- Note: Mandarin is coded **`cmn`** (Cantonese is `yue`; there is a separate "Cantonese" and "Literary Chinese" in the language list). Audio exports include **transcriptions** (`cmn_transcriptions.tsv.bz2` — pinyin/Latin transliterations), which is a bonus for pronunciation drills.

---

## 4. LibriVox (Chinese)

### 4.1 License — decisive
https://librivox.org/pages/public-domain/ verbatim:

> `LibriVox records only texts that are in the public domain (in the USA – see below for why), and all our recordings are public domain (definitely in the USA, and maybe in your country as well, see below). This means anyone can use all our recordings however they wish (even to sell them).`
>
> `In addition, book summaries, CD cover art, and any other material that goes into our catalog with the audio recordings are in the public domain.`
>
> `if you record for LibriVox, all your recordings will be donated to the public domain`
> `you may do whatever you like with our recordings – you don't need permission`

Wiki (https://wiki.librivox.org/index.php?title=Copyright_and_Public_Domain):

> `This means that if you volunteer to record for LibriVox, you are agreeing to release the audio files you make into the public domain.`

**Verdict: YES** — US public domain; **no attribution required** ("there is no need to credit LibriVox, although of course we much prefer if you do credit us"). Caveat stated by LibriVox itself: `all our recordings are public domain in the USA, but not necessarily in other countries` — check your jurisdiction.

Note: `https://librivox.org/pages/copyright-and-public-domain/` → **HTTP 404** (does not exist); the live page is `/pages/public-domain/`.

### 4.2 Count of Chinese works
The librivox.org search is a JS/POST form with no server-rendered result count, so this was measured from the Internet Archive collection (which is where LibriVox deposits all recordings):

- Query: `collection:librivoxaudio AND language:(zho OR chi OR yue)` → **27 items**
  - `zho` = 25, `chi` = 1 (`art_of_war_chinese_1506_librivox`), `yue` = 1 (`english_and_cantonese_dictionary_2005_librivox`, bilingual eng+yue)
  - ≈**125 h** total runtime
  - URL: `https://archive.org/advancedsearch.php?q=collection%3Alibrivoxaudio+AND+language%3A%28zho+OR+chi+OR+yue%29&output=json`
- Representative titles: 孫子兵法, 唐诗三百首 vols 1–5, 呐喊, 朝花夕拾, 徬徨, 狄公案, 聊齋誌異, 論語, 長生殿, 牛郎織女傳, 熱風, and several 聖經(和合本) books.
- A `subject:"Chinese"` search returns 55, but that is inflated with **English** books about China — not Chinese-language audio.

**Catalog API caveat:** `https://librivox.org/api/feed/audiobooks/?language=Chinese` returns HTTP 200 JSON but the `language` parameter is **not supported** (the API info page lists only `id, since, author, title, genre, extended, coverart, limit, offset`) — it happily returns English/French/German/Spanish books. `format=csv` is broken. Use archive.org to enumerate Chinese items.

### 4.3 Transcripts: **do not align**
- LibriVox ships **audio only**. The catalog's `url_text_source` field is a *pointer* (Project Gutenberg / Wikisource), not a transcript.
- **No general "LibriVox Mandarin forced-alignment corpus" exists.** The HF repo `AdoCleanCode/librivox_spanish_mfa_aligned_train` shows the pattern but there is **no Mandarin equivalent**.
- Text source: Project Gutenberg Chinese texts exist (https://www.gutenberg.org/browse/languages/zh). License (https://www.gutenberg.org/policy/license.html): `"This eBook is for the use of anyone anywhere in the United States and most other parts of the world at no cost and with almost no restrictions whatsoever."` — the only real constraint is the PG **trademark** license: `"If you strip the Project Gutenberg license and all references to Project Gutenberg from the text, you are left with a text unrestricted by U.S. intellectual property law."` PG has **no audio** (`/browse/categories/audio` → 404).

### 4.4 ⭐ The aligned LibriVox Mandarin route: **CSS10 Chinese**
**https://www.kaggle.com/datasets/bryanpark/chinese-single-speaker-speech-dataset**

- Source: LibriVox recordings of 朝花夕拾 (Chao Hua Si She) and 呐喊 (Call to Arms) by 魯迅, reader **Jing Li** — both public domain, so the CC0 tag is coherent.
- CSS10 abstract (https://raw.githubusercontent.com/Kyubyong/css10/master/README.md): `"It is composed of short audio clips from LibriVox audiobooks and their aligned texts."` — **this is aligned Mandarin audio + text**.
- **License (verbatim, Kaggle page JSON-LD):** `"license":{"@type":"CreativeWork","name":"CC0: Public Domain","url":"https://creativecommons.org/publicdomain/zero/1.0/"}`; Kaggle API `licenseNameNullable` = `"CC0: Public Domain"`.
- Size **2,040,891,168 bytes (≈2.04 GB)**; runtime **06:27:04**; single speaker.
- **Verdict: YES** — CC0, no NC, no ND, no attribution, no share-alike. **The best bundleable Mandarin audio+aligned-text source found.**
- Caveat: the *code* repo `Kyubyong/css10` ships an Apache-2.0 `LICENSE`; the *data* on Kaggle is CC0.

---

## 5. AISHELL-1 / 2 / 3 / 4

### 5.1 AISHELL-1 — OpenSLR SLR33
https://www.openslr.org/33/ verbatim:
> `License: Apache License v.2.0`
> `The data is free for academic use. We hope to provide moderate amount of data for new researchers in the field of speech recognition.`

`https://www.openslr.org/resources/33/` directory contains only `about.html`, `checksum.md5`, `info.txt`, `data_aishell.tgz`, `resource_aishell.tgz`; `info.txt` repeats `license: Apache License v.2.0`. **There is no separate LICENSE file inside the payload beyond `info.txt`.**

**Vendor contradiction** — aishelltech.com now serves a JS SPA with **no license text**; the terms are only recoverable via Wayback. http://web.archive.org/web/20170724041458/http://www.aishelltech.com/kysjcp:
> `(The data is free for academic use. Commercial use is forbidden.)`
> CN: `( 支持学术研究，未经允许禁止商用。 )`

**HF:** `AISHELL/AISHELL-1` exists, `gated=false`, `cardData.license = "apache-2.0"`, but the card body still says `"The data is free for academic use."` `CAiRE/aishell1`, `speechcolab/aishell1`, `Bingsu/aishell1` → **all 404**.

**Verdict: UNCLEAR → treat as NO-NC.** 178 h, 400 speakers. `data_aishell.tgz` 15 G + `resource_aishell.tgz` 1.2 M.

### 5.2 AISHELL-2
Not on OpenSLR. Vendor page via Wayback (http://web.archive.org/web/20180602070422/http://www.aishelltech.com/aishell_2):
> `( This database is free for academic research, not in the commerce, if without permission. )`

Plus a `数据使用申请` (data-use application) with split contacts (`bd@aishelldata.com` commercial / `aishell.foundation@gmail.com` academic). **`AISHELL/AISHELL-2` does not exist on HF.**

**Verdict: NO-NC.** 1000 h, 1991 speakers.

### 5.3 AISHELL-3 — OpenSLR SLR93 — *the "non-commercial only" claim is CONFIRMED*
https://www.openslr.org/93/ verbatim: `License: Apache License v.2.0`.

But the **same archived vendor page** (http://web.archive.org/web/20201130200931/http://aishelltech.com/aishell_3) carries **both**:
> `License: Apache License v.2.0`
> `( This database is free for academic research, not in the commerce, if without permission. )`

So the widely-cited claim that **AISHELL-3 is free for non-commercial research only is supported by the vendor's own text, and it directly contradicts OpenSLR's Apache 2.0.** HF `AISHELL/AISHELL-3` exists, `gated=false`, front-matter `license: apache-2.0`.

**Verdict: UNCLEAR → do not rely on the Apache-2.0 tag alone.** 85 h, 88,035 utterances, 218 speakers, char + pinyin transcripts (`data_aishell3.tgz` 18 G). This is the most TTS-friendly AISHELL, which makes the ambiguity costly.

### 5.4 AISHELL-4 — OpenSLR SLR111 — **clean**
https://www.openslr.org/111/ verbatim: `License: CC BY-SA 4.0`. `info.txt` repeats it. **No separate agreement:** a grep of the full page for `agreement|form|sign` returns zero matches, and all four tarballs are direct downloads. The vendor page also states `License: CC BY-SA 4.0`; the `数据使用申请` application applies only to the non-open-source portion.

⚠️ The HF mirror `AISHELL/AISHELL-4` front-matter says `license: apache-2.0` — a mismatch. **Cite OpenSLR's CC BY-SA 4.0.**

**Verdict: YES-attr.** 211 meetings, 120 h, 8-channel arrays; `train_L` 7.0 G + `train_M` 25 G + `train_S` 14 G + `test` 5.2 G. **Fit caveat:** transcripts are overlapping conference-meeting speech — poor for beginner pronunciation drills.

### 5.5 AISHELL-5 (SLR159) / AISHELL-6 / -7
SLR159 states `License: CC BY-SA 4.0` (https://www.openslr.org/159/) — in-car multi-channel. `SMIIP-lab/AISHELL6-Whisper` on HF is tagged `cc-by-nc-sa-4.0` → **NO-NC**. AISHELL-6/-7 were **not independently cleared** here.

---

## 6. MAGICDATA SLR68, Primewords SLR47, aidatatang SLR62, ST-CMDS SLR38, THCHS-30 SLR18

### 6.1 MAGICDATA Mandarin Chinese Read Speech Corpus — OpenSLR SLR68 — **NO**
https://www.openslr.org/68/ verbatim:
> `License: Attribution-NonCommercial-NoDerivatives 4.0 International Public License (CC BY-NC-ND 4.0)`
> `MAGICDATA Mandarin Chinese Read Speech Corpus was developed by MAGIC DATA Technology Co., Ltd. and freely published for non-commercial use.`
> `the corpus is totally free for academic use.`

755 h, 1080 speakers, segmented transcripts, `train_set.tar.gz` 52 G. **Verdict: NO** — NC *and* ND. Bundling would violate both.

### 6.2 Primewords Chinese Corpus Set 1 — OpenSLR SLR47 — **NO** (corrects a common assumption)
https://www.openslr.org/47/ verbatim:
> `License: Attribution-NonCommercial-NoDerivatives 4.0 International (CC BY-NC-ND 4.0)`
> `It is free for academic use.`

**This is NOT CC BY-SA 4.0** — the OpenSLR page says **CC BY-NC-ND 4.0**. 100 h, 296 speakers, JSON transcript mapping, `primewords_md_2018_set1.tar.gz` 9.0 G.
**Verdict: NO** (NC + ND).

### 6.3 aidatatang_200zh — OpenSLR SLR62 — **NO, and RETRACTED**
`https://www.openslr.org/62/` now returns `Resource not found: 62`. The archived page (https://web.archive.org/web/2024id_/https://www.openslr.org/62/) verbatim:
> `License: Attribution-NonCommercial-NoDerivatives 4.0 International (CC BY-NC-ND 4.0)`
> `About this resource: Resource retracted as per the data owner wish.`
> Contact: `services@datatang.com` · `https://www.datatang.ai`

The **live MagicData mirror still serves the same page**: `https://openslr.magicdatatech.com/62/` shows `License: Attribution-NonCommercial-NoDerivatives 4.0 International (CC BY-NC-ND 4.0)` and `Resource retracted as per the data owner wish.`

Independent confirmation of the license and the statistics (MFA model card, https://raw.githubusercontent.com/MontrealCorpusTools/mfa-models/main/corpus/mandarin/ai_datatang_corpus/README.md): `Number of hours: 200.38`, `Number of utterances: 237,265`, `Number of speakers: 600`, `License: CC BY-NC-ND 4.0`.

**The prompt's belief (CC BY-NC-ND 4.0) is CONFIRMED.** **Verdict: NO** — NC-ND *and* the resource was withdrawn by the owner. There is no legitimate download route.

### 6.4 Free ST Chinese Mandarin Corpus (ST-CMDS) — OpenSLR SLR38 — **NO**
https://www.openslr.org/38/ verbatim:
> `License: Creative Common BY-NC-ND 4.0 (Attribution-NonCommercial-NoDerivatives 4.0 International)`

855 speakers × 120 utterances = 102,600 utterances, `ST-CMDS-20170001_1-OS.tar.gz` 8.2 G. **Verdict: NO** (NC + ND).

### 6.5 THCHS-30 — OpenSLR SLR18 — **UNCLEAR (best guess YES-attr)**
https://www.openslr.org/18/ verbatim:
> `License: Apache License v.2.0`
> `THCHS30 is an open Chinese speech database published by Center for Speech and Language Technology (CSLT) at Tsinghua University.`
> `Therefore, the database is totally free to academic users.`

The Apache-2.0 field and the "academic users" sentence pull in different directions, but **unlike AISHELL there is no vendor clause forbidding commercial use.** Downloads: `data_thchs30.tgz` 6.4 G, `test-noise.tgz` 1.9 G, `resource.tgz` 24 M. Original CSLT page `http://data.cslt.org/thchs30/README.html` was **unreachable (HTTP 000)** on this check.
**Verdict: UNCLEAR, leaning YES-attr** (stated license is Apache-2.0; no NC clause found).

---

## 7. Other OpenSLR Mandarin resources (for completeness)

| SLR | Name | OpenSLR license field (verbatim) | Verdict |
|---|---|---|---|
| SLR119 | AliMeeting | `CC BY-SA 4.0` | YES-attr (120 h meeting speech) |
| SLR111 | AISHELL-4 | `CC BY-SA 4.0` | YES-attr |
| SLR159 | AISHELL-5 | `CC BY-SA 4.0` | YES-attr |
| SLR82 | CN-Celeb | `Attribution-ShareAlike 4.0 International` | YES-attr (speaker-recognition; not a transcript corpus) |
| SLR85 | HI-MIA | `Apache License v.2.0` | YES-attr (far-field wake word) |
| SLR87 | MobvoiHotwords | `Apache License v.2.0` | YES-attr |
| SLR93 | AISHELL-3 | `Apache License v.2.0` | UNCLEAR (§5.3) |
| SLR33 | Aishell (AISHELL-1) | `Apache License v.2.0` | UNCLEAR (§5.1) |
| SLR121 | WenetSpeech | `Creative Commons Attribution 4.0 International License (CC BY 4.0)` | **NO** — see §8.1 |
| SLR68 | MAGICDATA Read Speech | `CC BY-NC-ND 4.0` | NO |
| SLR123 | MAGICDATA Conversational | `CC BY-NC-ND 4.0` | NO |
| SLR47 | Primewords Set 1 | `CC BY-NC-ND 4.0` | NO |
| SLR38 | ST-CMDS | `CC BY-NC-ND 4.0` | NO |
| SLR138 | SHALCAS22A | `Attribution-NonCommercial-NoDerivatives 4.0 International (CC BY-NC-ND 4.0)` | NO |
| SLR62 | aidatatang_200zh | *retracted* | NO (§6.3) |
| SLR146 | CML-TTS | `CC-BY 4.0 license` | NO — **no Chinese** (§9.3) |
| SLR50 | MADCAT Chinese data splits | (splits only; underlying LDC corpus) | NO — LDC terms |
| SLR55 | CLMAD | text-only | n/a |

---

## 8. WenetSpeech, KeSpeech, GigaSpeech 1/2, Emilia / Emilia-YODAS

### 8.1 WenetSpeech (SLR121 + SLR121 official site) — **NO** ⚠️
OpenSLR (https://www.openslr.org/121/) verbatim: `License: Creative Commons Attribution 4.0 International License (CC BY 4.0)`.

**But the official project site is decisive** (https://wenet-e2e.github.io/WenetSpeech/, "License" section) verbatim:
> `The WenetSpeech dataset is available to download for non-commercial purposes under a Creative Commons Attribution 4.0 International License. WenetSpeech doesn't own the copyright of the audios, the copyright remains with the original owners of the video or audio, and the public URL is given for the original video or audio.`

Two independent disqualifiers: (a) **"for non-commercial purposes"** despite the CC BY label; (b) **the publisher does not own the audio copyright**, so it cannot grant redistribution rights at all. Downloads require a Google Form to obtain a password (https://github.com/wenet-e2e/WenetSpeech: `read the license, and follow the instruction to apply for the PASSWORD to download the data`). `https://wenet.org.cn/WenetSpeech` was **unreachable (HTTP 000)** on this check.

**Verdict: NO-NC (+ copyright defect).** 10,000 h high-label / 2,400 h weak-label / 22,400 h total.

### 8.2 KeSpeech — **NO, three times over**
License file https://raw.githubusercontent.com/tzyll/KeSpeech/main/dataset_license.md verbatim (note: `github.com/KeSpeech/KeSpeech` 200-redirects to `tzyll/KeSpeech`):
> `Non-commercial. You may not use this datasets for any commercial purposes, including not limitation to, any ideas, plans, conducts and activities primarily intended for or directed towards commercial advantage, monetary compensation or data's exchange (even if there is no payment of monetary compensation in connection with the exchange).`
>
> `No Distribution. You may not distribute this dataset to any third parties; for the purposes of this license, "third parties" excludes any employees or subcontractor who work for you on a contract basis, provided that such employees and subcontractor shall comply with all the terms of this license.`
>
> `Technical Modifications Allowed; No Adaptations. You may use this datasets in all media and formats whether now known or hereafter created, and to make technical modifications necessary to do so, but otherwise you have no rights to make Adaptations on this datasets.`

Content: **1,542 h, 27,237 speakers, 34 cities**, standard Mandarin + 8 subdialects; labels = transcription + speaker ID + subdialect.

Download route (https://github.com/KeSpeech/KeSpeech): `When you download the data (passwd: b6fy), you agree to the license(dataset_license.md)` — a Baidu pan link. The paper's abstract also says `"The dataset is free for all academic usage."`

**Premise correction:** **arXiv 2112.13463 is NOT KeSpeech** — that ID is *"Bilingual Speech Recognition by Estimating Speaker Geometry from Video Data"*. KeSpeech is a NeurIPS 2021 Datasets & Benchmarks paper with no arXiv version found.

**Verdict: NO-NC + NO-other (no distribution + no adaptations).** The HF copies (`TwinkStart/KeSpeech`, `miaocongxin/KeSpeech`, `danya-pixel/chinese-speech-dataset-kespeech`) carry **no license tag**, and `TwinkStart/KeSpeech` contains **test data only**.

### 8.3 GigaSpeech / GigaSpeech 2 — **NO-NC, and neither has Mandarin** ⚠️
HF gating terms for `speechcolab/gigaspeech` (https://huggingface.co/api/datasets/speechcolab/gigaspeech) verbatim — note the `apache-2.0` tag coexisting with this:
> `SpeechColab does not own the copyright of the audio files. For researchers and educators who wish to use the audio files for non-commercial research and/or educational purposes, we can provide access through the Hub under certain conditions and terms.`
> `1. Researcher shall use the Database only for non-commercial research and educational purposes.`
> `6. If Researcher is employed by a for-profit, commercial entity, Researcher's employer shall also be bound by these terms and conditions...`

`speechcolab/gigaspeech2` carries the **same** `extra_gated_prompt` (tagged `apache-2.0`, gated `auto`). Both require a Google Form (`https://forms.gle/UuGQAPyscGRrUMLq6`).

**Premise correction — these are not Mandarin corpora:**
- **GigaSpeech (v1) cardData/README language = English** (10,000 h transcribed, 33,005 h total).
- **GigaSpeech 2 cardData language = `th, id, vi`** (Thai, Indonesian, Vietnamese) — **no Chinese at all**.

**Verdict: NO-NC** for both, plus the same audio-copyright defect, and neither is usable for zh.

### 8.4 Emilia / Emilia-YODAS — split verdict
Gating terms on `amphion/Emilia-Dataset` (https://huggingface.co/api/datasets/amphion/Emilia-Dataset) verbatim:
> `1. The researcher shall use the Emilia dataset under the CC-BY-NC license and the Emilia-YODAS dataset under the CC-BY license.`

and the description adds: `"the original 101k-hour Emilia dataset (licensed under CC BY-NC 4.0)"`.

- **Emilia** (original, **101k h**, 6 languages incl. `zh`; `amphion/Emilia` cardData `cc-by-nc-4.0`): its gated prompt says `"The researcher shall use the dataset ONLY for non-commercial research and educational purposes."` → **NO-NC.**
- **Emilia-YODAS**: **CC BY 4.0** → **YES-attr.** `amphion/Emilia-Dataset` cardData is `license: cc-by-4.0`, languages `zh, en, ja, fr, de, ko`; gated `auto` (must accept the terms). The canonical `amphion/Emilia-YODAS` path returns **401**; the mirror `TTS-AGI/emilia-yodas` confirms `"license":"cc-by-4.0"`. **A standalone Emilia-YODAS license document was UNVERIFIED** — the only verbatim grant found is the gating sentence quoted above. Chinese-subset hours were **not verified**.

### 8.5 WenetSpeech4TTS — **NO-NC**
`https://huggingface.co/api/datasets/Wenetspeech4TTS/WenetSpeech4TTS` states it is available `"for non-commercial purposes under a Creative Commons Attribution 4.0 International License"` with `"Researcher shall use the Database only for non-commercial research and educational purposes."` 12,800 h Mandarin TTS subset. **Verdict: NO-NC** — the same CC BY label with an NC condition as WenetSpeech itself.

---

## 9. Remaining long-tail items

### 9.1 CML-TTS (SLR146) — **CC BY 4.0, but no Chinese**
https://www.openslr.org/146/ verbatim: `License: CC-BY 4.0 license`, and:
> `CML-TTS is a dataset composed of reading audiobooks from the LibriVox2 project, which uses books from Project Gutenberg3, released in the public domain. It consists of recordings in Dutch, German, French, Italian, Polish, Portuguese, and Spanish, with a sampling rate of 24kHz.`

**No Mandarin/Chinese subset exists.** The corpus is **3,233.43 h, 613 speakers**, 24 kHz audiobooks; the paper states `"consisting of audiobooks in seven languages: Dutch, French, German, Italian, Portuguese, Polish, and Spanish"` (https://ar5iv.labs.arxiv.org/html/2306.10097) and the OpenSLR download list contains only those seven tarballs. Verdict: license is fine (**YES-attr**) but **not usable for zh**.

### 9.2 MagicData HF "Dialect TTS-Lite" datasets — **UNCLEAR, assume NO**
Example: https://huggingface.co/datasets/MagicDataTech/magicdata-dialect-cantonese-tts-lite
- HF metadata says `license: apache-2.0` (front-matter).
- The **README body says otherwise** — it is titled `## MAGIC DATA OPEN-SOURCE LICENSE` (and mis-titled "Northeastern Chinese" even on the Cantonese card), with the table row:
  > `| License | Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International License |`
- Content: `| Cantonese | Guangzhou | GUD | 10 minutes | 54 sentences | 1 female, 55 years old |`; 48 kHz/16-bit WAV; transcript file `ProsodyLabeling/txt.txt`. Siblings: Cantonese, Sichuanese, Wu, Northeastern, Henan (each ~10 min).
- **Verdict: UNCLEAR → assume NO-NC-ND** because the README states CC BY-NC-ND while the metadata tag claims Apache-2.0. Do not bundle on the strength of the tag. (Sibling `MagicHub/*` copies carry **no license tag at all**, last modified 2026-06-15.)

### 9.3 MSR / Microsoft, Fisher Chinese, HKUST/MTS — **NO / not found**
- **Microsoft "MSR Mandarin" / "Microsoft Speech Corpus (Msr)" does NOT exist.** A full grep of the OpenSLR index and HF searches (`msr`, `msr-cn`, `mandarin-read`, `microsoft-chinese`) turn up **no such Mandarin corpus**. **NO LICENSE FOUND / NOT FOUND.**
  - Closest is **MSR-86K** (86,300 h, 15 languages from YouTube, Interspeech 2024) — `Alex-Song/MSR-86K` is `cc-by-nc-nd-4.0`, `gated: manual`, language tags `es, ko, en, fr, de, hi, vi, it, nl, pt, th, ru, id, ja, ar` — **no Chinese** → **NO-NC**.
  - **MS-SNSD** is English-only (code MIT; data under `"the original terms that Microsoft received such datasets"`) → **NO** (no Mandarin).
- **There is no LDC "Fisher Mandarin" corpus.** `LDC2010S05` is **"Asian Elephant Vocalizations"**, not Fisher Mandarin. LDC Fisher is **English** (LDC2004S13 / LDC2005S13) plus **Spanish** (LDC2010S01). **Fisher Chinese: NOT FOUND.**
- **HKUST Mandarin Telephone Speech (LDC2005S15, ~149 h Mandarin conversational telephone speech)** — the LDC User Agreement for Non-Members (https://catalog.ldc.upenn.edu/license/ldc-non-members-agreement.pdf) verbatim:
  > `User agrees to use the LDC Databases received under this Agreement only for noncommercial linguistic education, research and technology development... User must join LDC as a For-Profit Member and pay all applicable fees prior to release of said commercial product.`
  >
  > `Unless explicitly permitted herein, User shall not otherwise publish, retransmit, disclose, display, copy, reproduce or redistribute the LDC Databases to others outside of User's Research Group.`

  **Verdict: NO-NC + explicit no-redistribution.** This applies to the whole LDC catalog (including the SLR50 MADCAT splits, which only redistribute *splits*, not the underlying corpus).

### 9.4 zhvoice / DiDiSpeech / Magicoder
| Item | Finding | Verdict |
|---|---|---|
| **zhvoice** | ~**900 h, ~3,200 speakers, ~1.13M utterances**, `metadata.csv` + mp3. **NO LICENSE FOUND AT** `https://github.com/fighting41love/zhvoice`, `.../master/README.md` (contains no license or copyright text), `.../master/LICENSE`, `https://github.com/KuangDD/zhvoice` (404), `.../KuangDD/zhvoice/master/LICENSE` (404). HF: only `SeanSleat/zhvoice` — `NO LICENSE TAG`. | **UNABLE TO VERIFY — do not bundle** |
| **DiDiSpeech** | ~**800 h, 48 kHz, 6,000 speakers**, Mandarin + text. **NO LICENSE FOUND AT** `https://arxiv.org/abs/2010.09275`, `https://raw.githubusercontent.com/athena-team/DiDiSpeech/master/README.md`, `https://athena-team.github.io/DiDiSpeech/`, `https://outreach.didichuxing.com/research/opendata/` (now a login-gated Gaia portal; archived copy also gated), OpenDataLab (no page found). The paper says only `"The corpus is available at https://outreach.didichuxing.com/research/opendata/."` | **UNABLE TO VERIFY — do not bundle** |
| **Magicoder** | This is the **Magicoder LLM code-generation dataset** (https://github.com/ise-uiuc/magicoder, `LICENSE` = `"MIT License Copyright (c) 2023 iSE-UIUC"`; released models carry Llama2/DeepSeek licenses). **Not a Mandarin speech corpus** — no audio, no transcripts. MagicData's speech corpora are the SLR68/SLR123 items in §6.1 and the TTS-Lite items in §9.2. | **n/a — premise conflation** |

### 9.5 Sherpa-onnx and TTS-friendly public Mandarin datasets
- **sherpa-onnx code and pretrained models**: the `csukuangfj/*` HF model repos carry **no license metadata** (`cardData: null` on e.g. `csukuangfj/sherpa-onnx-paraformer-zh-2024-03-09`; `tags: ['onnx','region:us']`); the `vits-zh-*` and `sherpa-onnx-vits-zh-*` repos likewise show **no `license:` tag**. Licensing is governed by the upstream `k2-fsa/sherpa-onnx` project (Apache-2.0) **and by the terms of whatever corpus each model was trained on** — several zh voices are trained on AISHELL-3, which is exactly the ambiguous-license corpus from §5.3. **Do not assume a model is freely redistributable because the framework is Apache-2.0.**
- **Publicly-licensed TTS-friendly Mandarin data actually found:** **CSS10 Chinese (CC0)** is the only clean one. AISHELL-3 (multi-speaker, char+pinyin) is the natural TTS corpus but its license is contested (§5.3). The MagicData dialect TTS-Lite zips are ~10 min each and carry the tag/README contradiction (§9.2).
- **Note:** this item is the least complete area of this report. A dedicated audit of per-model training-data provenance for sherpa-onnx zh voices is recommended before shipping any bundled voice.

---

## 10. Corrections to premises stated in the research brief

| Premise in the brief | Verified reality |
|---|---|
| "Primewords (SLR47) — license **CC BY-SA 4.0**?" | **Wrong.** OpenSLR states **CC BY-NC-ND 4.0**. Disqualified. |
| "aidatatang_200zh (SLR62) — CC BY-NC-ND 4.0?" | **Correct**, and additionally **retracted by the data owner**. |
| "GigaSpeech2 ... useful here" | **GigaSpeech 2 is Thai/Indonesian/Vietnamese — no Chinese.** GigaSpeech 1 is English. Both NC-gated. |
| "AISHELL-3 is often cited as free for non-commercial research only — verify" | **Verified as a real contradiction**: OpenSLR says Apache-2.0, the vendor says `"...not in the commerce, if without permission."` |
| "AISHELL-1 is on OpenSLR (SLR33) and HuggingFace" | **Correct** for `AISHELL/AISHELL-1` (apache-2.0 tag). `CAiRE/aishell1`, `speechcolab/aishell1`, `Bingsu/aishell1` **do not exist**. |
| "MSR / Microsoft Speech Corpus" | **No Microsoft Mandarin corpus exists** on OpenSLR or HF under any searched name. |
| "Fisher Chinese" | **No LDC Fisher Mandarin corpus exists.** LDC2010S05 = *Asian Elephant Vocalizations*. LDC Fisher = English + Spanish. |
| "CML-TTS" | **CC BY 4.0 but contains no Chinese** (7 non-zh languages). |
| "Magicoder" | **A code LLM (MIT), not a speech dataset.** |
| "Common Voice ... CC0 1.0? the Mozilla Data Collective terms?" | **Both**: the data is CC0-1.0 **and** the MDC datasheet/ToS forbid re-hosting. The prohibition wins in practice. |
| "Tatoeba audio (CC BY 4.0? some CC0?)" | **Neither for Mandarin**: 98.5% of zh audio is "No license for offsite use"; the other 1.4% is CC BY-NC 4.0. zh sentences are CC BY 2.0 FR. |
| "AISHELL-4 license terms" | **CC BY-SA 4.0, no separate agreement** — the one clean AISHELL. |
| KeSpeech reference "arXiv 2112.13463" | **That ID is a different paper.** KeSpeech = NeurIPS 2021 D&B, no arXiv found. |

---

## 11. What could NOT be verified (explicit)

1. Any standalone **AISHELL `数据使用申请` agreement document text** — none on OpenSLR, none in the Wayback CDX, and current `aishelltech.com` is a JS SPA serving no license text. Vendor terms are recoverable **only via Wayback**.
2. Whether the AISHELL vendor academic-only clause **legally overrides** the OpenSLR Apache-2.0 tag for AISHELL-1 / AISHELL-3.
3. **Exact** Chinese counts on librivox.org's own search UI (JS/POST form, no server-rendered count); the 27-item figure is from archive.org.
4. **zhvoice** and **DiDiSpeech** upstream license clauses — **NO LICENSE FOUND** at every URL reachable here.
5. **MSR / Microsoft Mandarin** corpus with any license — **does not appear to exist**.
6. Whether the CC0 HF Common Voice mirrors (`legacy-datasets/common_voice`, `fsicoli/common_voice_22_0`, `OpenFormosa/common_voice_25_zh-TW`) still carry validated splits and whether their re-hosting is sanctioned — **UNVERIFIED**.
7. **Emilia-YODAS** standalone license document and its Chinese-subset hours — the only verbatim grant found is the `amphion/Emilia-Dataset` gating sentence; canonical `amphion/Emilia-YODAS` returns 401.
8. Per-model training-data provenance for sherpa-onnx Chinese TTS voices — **NOT AUDITED**.
9. AISHELL-6 / AISHELL-7 licenses — **not independently cleared**.
10. **MagicHub** (magichub.com) license text — JS SPA, **not retrievable**; the HF `MagicHub/*` dialect-TTS copies carry **no license tag**.
11. **zake7749/chinese-speech-corpus** (HF) is tagged `cc` but the exact CC variant was **UNVERIFIED**.
