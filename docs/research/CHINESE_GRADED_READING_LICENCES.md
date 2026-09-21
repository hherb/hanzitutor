# License research: bundleable Chinese (Mandarin) graded reading text

**Target:** offline desktop/mobile app, **AGPL-3.0**, wants to **bundle** short graded Chinese
passages/phrases (HSK-graded) for pronunciation + listening practice.
CC BY-SA acceptable with attribution. **NC / ND / research-only are disqualifying.** Public domain ideal.

**Research date:** 2026-09-21. Method: live `curl`, GitHub/HuggingFace APIs, `raw.githubusercontent.com`,
Wayback Machine, publisher copyright pages. Counts marked **measured** were obtained by downloading the
data and counting records, not from a website's own claim.

**Verification provenance:** findings were produced by three parallel workstreams. Items I re-verified
myself are marked **[verified]**. Items quoted from a delegating workstream, with the URL given, are
marked **[quoted]** — the URL and verbatim clause are supplied so they can be spot-checked.

> **Companion files in this repo** (produced by the parallel workstreams; this file supersedes and
> consolidates them):
> - `license-research-named-sources-hsk-bundling.md` — the six named sources, in full depth
> - `docs/license-research-chinese-reading-corpora.md` — corpora, annotators, HSK datasets
>
> ⚠️ My first-choice filename collided with a sibling file because macOS is case-insensitive; the
> write policy correctly blocked an overwrite, which is why the sibling files have distinct names.

---

## BOTTOM LINE (this changed during research)

The premise of the brief — that the open HSK-graded material is the Universal Dependencies
`UD_Chinese-Beginner` treebank — **is a trap**: that treebank is **CC BY-NC-SA 3.0 and is disqualified.**
Every *named* commercial source (Mandarin Companion, Chinese Breeze, Du Chinese, Chairman's Bao, LingQ)
is all-rights-reserved, and **"Chinese Reading Project" does not exist at all.**

However, the research turned up **four genuinely bundleable, actually-HSK-graded corpora** that were not
in the brief. Two of them are better than anything in the brief:

| Rank | Source | Licence | Why |
|---|---|---|---|
| **1** | **`harukicoder/hsk30-graded-readers`** | **CC BY 4.0** | 102 passages / 1,185 sentences, word-aligned pinyin + gloss. **No share-alike** → cleanest for AGPL. |
| **2** | **`no7z/hsk-sentences-audio`** | **CC BY-SA 4.0** | 4,354 HSK 1–6 sentences + **8,708 MP3s** (normal + slow) + pinyin/gloss/translation. Text + audio. |
| **3** | **`SHLEW06/chinese-hsk-adaptive-reader`** | MIT (text not separately stated — ⚠️ ambiguous) | **300** original HSK readings, 50 per level. |
| **4** | **Global Storybooks 中文故事集** | CC BY 4.0 (site) / CC BY 3.0 (story pages) | 40 stories + human audio + PDFs; not HSK-graded. |

Volume fallback: **Tatoeba `cmn`** — 89,065 sentences, **CC BY 2.0 FR**, ungraded (you grade it).

---

## Master summary table

### ✅ Bundleable

| # | Source | Content | HSK-graded? | License | Verdict | URL |
|---|---|---|---|---|---|---|
| 1 | **harukicoder/hsk30-graded-readers** | 102 texts / 1,185 sentences / 16,956 chars; 6 shelves; 30-text held-out | **Yes** (6 shelves; labels computed, not baked in) | **CC BY 4.0** | ✅ **BEST — no share-alike** | [HF](https://huggingface.co/datasets/harukicoder/hsk30-graded-readers) |
| 2 | **no7z/hsk-sentences-audio** | 4,354 sentences + **8,708 MP3s** + pinyin/gloss/translation/grammar tags | **Yes — official HSK 3.0 L1–6** | **CC BY-SA 4.0** | ✅ (share-alike) | [HF](https://huggingface.co/datasets/no7z/hsk-sentences-audio) |
| 3 | **SHLEW06/chinese-hsk-adaptive-reader** | **300** readings (50 × HSK1–6), ~13.6 MB JSON, + translations/glosses | **Yes — HSK 1–6** | **MIT** (repo root); text licence **not separately stated** ⚠️ | ⚠️ Likely OK — confirm with author | [GitHub](https://github.com/SHLEW06/chinese-hsk-adaptive-reader) |
| 4 | **Global Storybooks 中文故事集** | 40 stories, 5 length/vocab levels, human MP3 + PDF | No (5 local levels) | **CC BY 4.0** / story pages **CC BY 3.0** | ✅ attribution only | [site](https://global-asp.github.io/storybooks-chinese/) |
| 9 | **Tatoeba `cmn`** | **89,065** sentences (median 10 chars) | No (ungraded) | **CC BY 2.0 FR** (CC0 subset = **1** cmn sentence) | ✅ **no share-alike** | [downloads](https://tatoeba.org/en/downloads) |
| 9 | **bdx33/tatoeba-hsk-cmn-eng-fra** (HF) | 78,504 simplified rows, HSK-tagged | **Yes** (derived) | CC BY 2.0 (upstream CC BY 2.0 FR) | ✅ derived; verify derivation | [HF](https://huggingface.co/datasets/bdx33/tatoeba-hsk-cmn-eng-fra) |
| 5 | **Chinese Wikisource** | 4,031,485 content pages; dump **7.89 GB** | No | **CC BY-SA 4.0 + GFDL**; underlying classical texts PD | ✅ | [版权信息](https://zh.wikisource.org/wiki/Wikisource:%E7%89%88%E6%9D%83%E4%BF%A1%E6%81%AF) |
| 6 | **Project Gutenberg (zh)** | **443** Chinese Text items; 440 not US-copyright-restricted | No (classical, mostly Traditional) | PG License + US PD | ✅ strip all PG references | [license](https://www.gutenberg.org/policy/license.html) |
| 9 | **CC-CEDICT** | 125,083-entry dictionary | No | **CC BY-SA 4.0** | ✅ (dictionary, not reading) | [mdbg.net](https://www.mdbg.net/chinese/dictionary?page=cc-cedict) |
| 9 | **daligao/chinese-reading-lab** | ~3,904 CJK chars, 10 stories, HSK4–6 | **Yes (HSK4–6)** | **CC0 1.0** (README only; no LICENSE file) | ✅ | [GitHub](https://github.com/daligao/chinese-reading-lab) |
| 9 | **daligao/mandarin-flashcards** | HSK1–3 vocabulary | **Yes (HSK1–3)** | **CC0 1.0** (README only) | ✅ | [GitHub](https://github.com/daligao/mandarin-flashcards) |
| 10 | **UD_Chinese-GSDSimp** | 4,997 sentences / 123,289 tokens, Simplified | No | **CC BY-SA 4.0** | ✅ | [LICENSE.txt](https://raw.githubusercontent.com/UniversalDependencies/UD_Chinese-GSDSimp/master/LICENSE.txt) |
| 10 | **UD_Chinese-GSD** | 4,997 sentences / 123,289 tokens, Traditional | No | **CC BY-SA 4.0** | ✅ | [LICENSE.txt](https://raw.githubusercontent.com/UniversalDependencies/UD_Chinese-GSD/master/LICENSE.txt) |
| 10 | **UD_Chinese-CFL** | 451 sentences / 7,256 tokens, Simplified, learner essays | CFL learner data | **CC BY-SA 4.0** | ✅ (⚠️ no consent statement) | [LICENSE.txt](https://raw.githubusercontent.com/UniversalDependencies/UD_Chinese-CFL/master/LICENSE.txt) |
| 10 | **UD_Chinese-PUD** | 1,000 sentences / 21,415 tokens, Traditional, news+wiki | No | **CC BY-SA 3.0** | ✅ (⚠️ v3.0, not 4.0) | [LICENSE.txt](https://raw.githubusercontent.com/UniversalDependencies/UD_Chinese-PUD/master/LICENSE.txt) |
| 10 | **UD_Chinese-HK** | 1,004 sentences / 9,874 tokens, Traditional | No | **CC BY-SA 4.0** | ✅ (⚠️ subtitle/LegCo provenance) | [LICENSE.txt](https://raw.githubusercontent.com/UniversalDependencies/UD_Chinese-HK/master/LICENSE.txt) |

### ❌ Disqualified

| # | Source | Content | License | Decisive reason | URL |
|---|---|---|---|---|---|
| 10 | **UD_Chinese-Beginner** | 2,295 sentences, **HSK1–5** — the graded one | **CC BY-NC-SA 3.0** | **NC disqualifying** | [LICENSE.txt](https://raw.githubusercontent.com/UniversalDependencies/UD_Chinese-Beginner/master/LICENSE.txt) |
| 10 | **UD_Chinese-PatentChar** | 200 sentences, patents | **CONFLICT** BY-SA 4.0 vs BY-NC-SA 3.0 | unresolved → treat as NC | [LICENSE.txt](https://raw.githubusercontent.com/UniversalDependencies/UD_Chinese-PatentChar/master/LICENSE.txt) |
| 4 | **Chinese Text Project (ctext.org)** | Pre-modern classical corpus | **All rights reserved** | "may not be republished without express written permission"; scraping forbidden | [ctext.org/faq](https://ctext.org/faq) |
| 1 | **"Chinese Reading Project"** | — | **Site does not exist** | NXDOMAIN, Verisign "No match", RDAP 404, zero Wayback captures | `chinesereadingproject.com` |
| 1b | **Chinese Reading Practice** | 245 lessons | **No licence found** | default ARR; © 2026 | [chinesereadingpractice.com](https://chinesereadingpractice.com/) |
| 2 | **Mandarin Companion** | Commercial graded readers | **ARR** | "All rights reserved; no part… may be reproduced" | [sample PDF](https://mandarincompanion.com/wp-content/uploads/2022/03/Just-Friends-Mandarin-Companion-Breakthrough-Level-SAMPLE.pdf) |
| 3 | **Chinese Breeze** | Commercial graded readers | **ARR** | "You may not… distribute, redistribute, or create derivatives" | [Cheng & Tsui ToS](https://cdn.cheng-tsui.com/terms-of-use) |
| 3 | **Graded Chinese Reader (Shi Ji)** | Commercial | **ARR** | **Publisher is Sinolingua, NOT Commercial Press** (brief was wrong) | [sinolingua](http://www.sinolingua.com.cn/) |
| 3 | **HSK Standard Course** | BLCU Press textbook | **ARR** | "版权所有… All Rights Reserved Copyright 2026" | [blcup.com](https://www.blcup.com/) |
| 3 | **HSK Academy / GC Readers** | HSK-graded, sold on Amazon | **Commercial** | sold, not licensed | [gradedchinesereaders.com](https://www.gradedchinesereaders.com/hsk-academy) |
| 3 | **HSKStory** | HSK 1–9 stories + audio, free to read | **Explicitly forbids redistribution** | "Don't redistribute or resell… Host mirrored copies" | [hskstory.com/copyright](https://hskstory.com/copyright) |
| 8 | **Du Chinese** | HSK-graded lessons | **ARR + NC** | download allowance is "sole personal and non-commercial use" | [T&C](https://www.iubenda.com/terms-and-conditions/27013293) |
| 8 | **The Chairman's Bao** | Graded news lessons | **ARR** | "Not to make any derivative use" | [ToS](https://www.thechairmansbao.com/terms-of-use/) |
| 8 | **LingQ** | Lessons + audio | **ARR/NC + fallback CC BY-ND 3.0** | **NC *and* ND — double disqualifier** | [terms](https://www.lingq.com/en/terms/) |
| 8 | **Popup Chinese** | Podcast lessons | ARR, **defunct** | © Language Systems Ltd; no licence | `popupchinese.com` → `saito.io/popup/` |
| 8 | **MandarinSpot** | Annotator tool | No ToS found | **Not a corpus — nothing to bundle** | [mandarinspot.com](https://mandarinspot.com/) |
| 9 | **CHILDES / TalkBank** | Spoken child-language corpora | **CC BY-NC-SA 3.0** | "precludes the incorporation of the data in commercial products" | [rules](https://talkbank.org/0share//rules.html) |
| 9 | **"Read Chinese!"** | 140 lessons × 2 scripts | **© 2006–2010 ARR** | **Correction: NFLC / U. Maryland — not Yale, not CHILDES.** Site dead | `readchinese.nflc.org` (dead) |
| 9 | **Chinese Reading World** (U. Iowa) | Graded readings | **No licence; site dead** | domain is a GoDaddy "for sale" page | `chinesereadingworld.org` |
| 9 | **`lm742611149/learn-chinese`**, **`NewHSK3/new-hsk-3-anki-deck`** | 315 HSK readings / 33,000 sentences | **No licence file** | default ARR | GitHub |
| 9 | **`ymcui/Chinese-Cloze-RC`** | Cloze corpus, CC-BY-SA-4.0 tag | Mixed | **People's Daily portion is copyrighted newspaper** — not safely redistributable | [HF](https://huggingface.co/datasets/ymcui/Chinese-Cloze-RC) |
| 7 | **Internet Archive** | 94,303 Chinese texts (only **520** with CC URL) | Per-item; ToS = scholarship/research only | per-item clearance required | [terms](https://archive.org/about/terms) |
| 8 | **"Chinese Text Annotator"** | — | — | **Does not exist** — `alexanderfrantsuzov` is a 404 user; 0 GitHub search results | — |
| 9 | **"Chinese-Learning-Corpus"** | — | — | **Not found** on GitHub or HuggingFace | — |

---

## PART A — The four genuinely bundleable HSK-graded sources

### 1. `harukicoder/hsk30-graded-readers` — **CC BY 4.0** ✅ **TOP RECOMMENDATION** [verified]

URL: <https://huggingface.co/datasets/harukicoder/hsk30-graded-readers> ·
repo: <https://github.com/harukicoder/hsk30>

**Measured** by downloading `hsk30_graded_readers.jsonl` (614,659 bytes) and parsing it:

| Metric | Value |
|---|---|
| Texts | **102** |
| Shelves | newbie 22 · beginner 22 · intermediate 22 · upper 12 · advanced 12 · native 12 |
| Sentences | **1,185** |
| Characters (len of `text`) | 16,956 (DATASHEET says "14,417 graded characters", i.e. excluding punctuation) |
| Held-out split | separate file `hsk30_heldout.jsonl`, 119,709 bytes, **30 texts / 180 sentences** |
| Created / last modified | 2026-09-01 (both) |
| Downloads | 79 |

Schema: `id`, `shelf`, `title{hanzi,pinyin,english}`, `text`, and `sentences[]` — each sentence with its
own English translation and a `words[]` list of `{hz, py, en}`. **This is word-aligned**: per-word pinyin
and gloss come free, which is exactly what a pronunciation/listening app needs.

Licence — HF API returns `license: cc-by-4.0`; README front-matter `license: cc-by-4.0`.
GitHub `README.md`, *verbatim* **[verified]**:

> MIT for the code and the derived level tables; **CC BY 4.0** for the corpus

and, *verbatim*:

> | `src/hsk30/` | The library and its six graded lists (MIT) |
> | `corpus/` | 102 aligned graded readers + a 30-text held-out split (CC BY 4.0) |

`DATASHEET.md`, *verbatim* **[verified]**:

> **Version** 1.1 · **Released** 2026 · **Licence** CC BY 4.0 · **Size** 102 texts, 1,185 sentences, 8,682 word tokens, 14,417 graded characters, plus a disjoint [30-text held-out split]
> …
> **CC BY 4.0** — reuse and adaptation permitted with attribution.

**Why this is #1:** CC BY 4.0 has **no share-alike**, so it does not push any licence obligation onto
your compiled data bundle — the cleanest possible fit alongside AGPL-3.0 code. (The GitHub repo's `LICENSE`
is MIT, but that is the **code**; the corpus is separately CC BY 4.0 — **carry the CC BY notice, not MIT.**)

⚠️ **Caveats, stated by the dataset itself:**
- **Difficulty labels are deliberately not included.** README, *verbatim*: "A level is a function of the
  text *and the standard*, and 'HSK 3.0' names two different official documents that disagree on 41.5% of
  shared vocabulary. Baking labels in would let a stale copy of this dataset contradict the grader."
  You must compute levels with their `hsk30.grade_tokens()` library (MIT). "Roughly half of these texts
  grade differently under the two documents."
- **LLM-assisted provenance.** README/DATASHEET, *verbatim*: "The passages were **drafted with
  large-language-model assistance and then reviewed, edited, re-levelled and in several cases rewritten by
  the author.**" This is a **quality** caveat (and a copyrightability nuance — purely AI-generated text may
  attract no copyright), not a licensing obstacle.
- It is **small** (102 passages). Use it as a high-quality core, not as your whole library.

### 2. `no7z/hsk-sentences-audio` — **CC BY-SA 4.0** ✅ (text + audio) [verified]

URL: <https://huggingface.co/datasets/no7z/hsk-sentences-audio>

**Measured:**

| Metric | Value |
|---|---|
| Sentences | **4,354**, graded against official HSK 3.0 levels 1–6 |
| Per level | HSK1 **281** · HSK2 **538** · HSK3 **727** · HSK4 **801** · HSK5 **965** · HSK6 **1,042** |
| Text files | `data/train.jsonl` **4,952,559 B** · `data/train.parquet` **670,256 B** |
| Audio | **8,708 MP3s** (normal + `_slow` for each sentence), **≈130 MB** (estimated from sampled file sizes) |
| Created / last modified | 2026-07-14 / 2026-07-15 |
| Downloads | 700 |

⚠️ **Correcting a claim made by a workstream:** one report said the repo contains "only 995 audio files"
and that audio is incomplete. **That is wrong — it is an artifact of the HuggingFace tree API's
1,000-entry hard cap.** I probed the first, middle and last file of *every* level (normal and `_slow`) and
**all returned HTTP 200**, with a true-negative control (`hsk6-1043.mp3` → 404, `hsk7-0001.mp3` → 404):

```
hsk1-0001.mp3 200   hsk1-0281.mp3 200   hsk2-0538.mp3 200   hsk3-0727_slow.mp3 200
hsk4-0801.mp3 200   hsk5-0965.mp3 200   hsk6-1042.mp3 200   hsk6-1042_slow.mp3 200
hsk6-1043.mp3 404   hsk7-0001.mp3 404
```

Because the last index of each level equals that level's advertised sentence count, the audio set is
complete. (The manifest's `"audio_included": false` refers to the *export manifest*, not the HF repo.)
Treat the full 8,708-file audio set as present.

Fields: `id`, `hsk_level`, `topic`, `sentence_type`, `chinese`, `traditional`, `pinyin`,
`pinyin_numbered`, `translation.en`, token-level `word`/`pinyin`/`gloss_en`, `grammar_points`,
official `grammar_tags`, `audio.normal`, `audio.slow`.

README, *verbatim* **[verified]**:

> 4,354 Chinese sentences graded against the official HSK 3.0 levels 1–6, with pinyin, English translations, per-word glosses, grammar tags, normal/slow synthetic speech. The complete export contains 8,708 MP3 files.
> …
> Dataset: CC-BY-SA-4.0. Glosses and some pinyin data derive from CC-CEDICT (CC-BY-SA); audio was synthesized with Apache-2.0 CosyVoice2.

`ATTRIBUTION.md`, *verbatim* **[verified]** (Chinese, translated):

> 本项目的**代码**以 MIT 许可发布（见 `LICENSE`）。 ["this project's **code** is released under MIT (see `LICENSE`)"]
> | CC-CEDICT | 词义、逐词标准拼音 | CC-BY-SA 4.0 | 需署名，衍生数据同以 CC-BY-SA 分享 |
> 句子文本与英文翻译为本项目自有。为简化合规，整个 `dist/` 数据集以 **CC-BY-SA 4.0** 发布。
> ["Sentence text and English translations are this project's own. To simplify compliance, the entire `dist/` dataset is released under CC-BY-SA 4.0."]
> 音频由 CosyVoice2-0.5B 在本地合成，**合成语音**；输入文本为本项目自有内容。
> ["Audio is synthesized locally by CosyVoice2-0.5B, **synthetic speech**; the input text is this project's own content."]

Provenance chain — all clean: text/translations original to the project · audio synthetic
CosyVoice2-0.5B (Apache-2.0) · pypinyin + jieba (MIT) · OpenCC (Apache-2.0) ·
`complete-hsk-vocabulary` (MIT, used for level validation only, not ingested).

⚠️ **Caveats:**
- **No `LICENSE` file exists in the HF repo** (I probed `LICENSE`, `LICENSE.md`, `LICENSE.txt` → all 404)
  even though `ATTRIBUTION.md` says "见 `LICENSE`". The licence is nonetheless stated in **three**
  independent places (README front-matter, HF API `license:cc-by-sa-4.0`, `export-manifest.json`
  `"license": "CC-BY-SA-4.0"`) plus `ATTRIBUTION.md` — so this is a documentation inconsistency, not a
  missing grant. It is still worth asking the author to add the file.
- **Audio is synthetic** — the README asks that this be disclosed downstream. Do so.
- **Levelling validated against a community word list**, and the dataset self-reports
  "与官方口径约有 ~1% 出入" (~1% divergence from the official standard), recommending spot-checks against
  <https://admin.chinesetest.cn/standardsAction.do?means=standardInfo>. Treat levels as ~99% accurate.
- The "own work" provenance claim is **not independently verifiable**.
- Share-alike binds the dataset and your adaptations of it — keep it in its own directory with a
  `LICENSE`/`ATTRIBUTION` file to hold the boundary against your AGPL code.

### 3. `SHLEW06/chinese-hsk-adaptive-reader` — **MIT** (text licence ambiguous ⚠️) [verified]

URL: <https://github.com/SHLEW06/chinese-hsk-adaptive-reader>

**Measured** by downloading `src/data/library/hsk{1..6}.json`:

| File | Bytes | Records |
|---|---|---|
| `hsk1.json` | 1,797,566 | 50 |
| `hsk2.json` | 1,966,880 | 50 |
| `hsk3.json` | 2,094,599 | 50 |
| `hsk4.json` | 2,257,837 | 50 |
| `hsk5.json` | 2,562,556 | 50 |
| `hsk6.json` | 2,923,622 | 50 |
| **Total** | **≈13.6 MB** | **300 readings (50 per HSK level 1–6)** |

Records carry `id`, `slug`, `titleZh`, `titleEn`, `hskLevel`, `category`, `sourceType`, `difficulty`,
plus `textZh`, `translationEn`, `sentenceExplanations`, grammar, target words and comprehension
questions. Text is Simplified and level-appropriate (HSK1: "我叫小明。今天我想谈谈《我的家》。…";
HSK6: "要理解《为什么唐诗到今天还有人读？》，不能只把它当成一个孤立的社会现象。…").

`LICENSE`, *verbatim* **[verified]**:

> MIT License
> Copyright (c) 2026 Shunji Lewandowski

`THIRD_PARTY_NOTICES.md` lists **only two** components **[verified]** — CC-CEDICT (CC BY-SA 4.0) and
`complete-hsk-vocabulary` (MIT). The **300 readings are not listed as third-party**, which implies they
are the author's own work covered by the root MIT grant (records are tagged `"sourceType": "original"`).

⚠️ **The ambiguity to resolve:** the readings are **not explicitly licensed in their own right**. MIT at
the repo root almost certainly covers them, but "MIT by repo root; text licence not restated" is exactly
the kind of gap that is cheap to close — **ask the author to add an explicit content licence statement**
before shipping. Also note the bundled CC-CEDICT-derived dictionary is a **CC BY-SA 4.0 derivative**.

### 4. Global Storybooks 中文故事集 — **CC BY 4.0 / CC BY 3.0** ✅ [quoted]

Site: <https://global-asp.github.io/storybooks-chinese/> · repo:
<https://github.com/global-asp/storybooks-chinese> · upstream: <https://www.africanstorybook.org/>

**40 stories**, 5 length/vocabulary levels, **human-read MP3 audio and downloadable PDFs**, all in
Chinese. Repo HEAD `df012889a863b89f65717818929662c1ea92b2b5`, **last commit 2025-12-10**, 68 commits.

FAQ, *verbatim*:

> Global Storybooks is an open source project, and all content on this site has been released under an open license.
> The African Storybook initiative makes hundreds of stories freely available under the Creative Commons license

Footer: `© 中文故事集. 保留部份版权.` → links <https://creativecommons.org/licenses/by/4.0/deed.zh>.
Upstream africanstorybook.org states: "Creative Commons Licence CC-BY-4.0".

⚠️ **Two things to log:**
- **Licence-version discrepancy:** the site/footer says **CC BY 4.0**, but **individual story pages carry a
  CC BY 3.0 badge** (`creativecommons.org/licenses/by/3.0/deed.zh`). Both are attribution-only and both
  acceptable — but **record the exact per-story licence at ingest**.
- The repo's `LICENSE` file is **MIT** (`Copyright (c) 2018 中文故事集`) — that is the **site code**, not the
  story content. Do not treat MIT as the content licence.
- **Not HSK-graded** — it has its own 5 local levels; you must re-level.

---

## PART B — Universal Dependencies Chinese treebanks (item 10, HIGH PRIORITY)

Repos: `github.com/UniversalDependencies/UD_Chinese-*`. **Sentence and token counts measured** by
downloading every `*.conllu` split and counting `# sent_id`, cross-checked against each repo's `stats.xml`.

| Treebank | Sentences | Tokens | train/dev/test | `LICENSE.txt` | README metadata | UD website | Verdict |
|---|---|---|---|---|---|---|---|
| UD_Chinese-GSD | 4,997 | 123,289 | 3997/500/500 | CC BY-SA 4.0 | CC BY-SA 4.0 | CC BY-SA 4.0 | ✅ |
| UD_Chinese-GSDSimp | 4,997 | 123,289 | 3997/500/500 | CC BY-SA 4.0 | CC BY-SA 4.0 | CC BY-SA 4.0 | ✅ |
| UD_Chinese-Beginner | 2,295 | 19,999 | test only | **CC BY-NC-SA 3.0** | **CC BY-NC-SA 3.0** | **CC BY-NC-SA 3.0** | ❌ NC |
| UD_Chinese-CFL | 451 | 7,256 | test only | CC BY-SA 4.0 | CC BY-SA 4.0 | CC BY-SA 4.0 | ✅ |
| UD_Chinese-PUD | 1,000 | 21,415 | test only | **CC BY-SA 3.0** | CC BY-SA 3.0 | CC BY-SA 3.0 | ✅ |
| UD_Chinese-HK | 1,004 | 9,874 | test only | CC BY-SA 4.0 | CC BY-SA 4.0 | CC BY-SA 4.0 | ✅ (⚠️) |
| UD_Chinese-PatentChar | 200 | 4,784 | test only | **CC BY-SA 4.0** | **CC BY-NC-SA 3.0** | **CC BY-NC-SA 3.0** | ⚠️ conflict |

**There are SEVEN zh treebanks, not six — the brief missed `UD_Chinese-PUD`.**

The six original repos all report GitHub `license: NOASSERTION / Other` because the licence lives in a
short plain-text `LICENSE.txt`, not a recognisable SPDX file — **do not read the GitHub "Other" badge as
"no licence"; read `LICENSE.txt`.** (PUD's badge was not re-checked — API rate limit — but its
`LICENSE.txt` contains the full CC BY-SA 3.0 legalcode.)

### UD_Chinese-GSD / GSDSimp — CC BY-SA 4.0 ✅

`LICENSE.txt` (202 bytes) for both, *verbatim*:

> The treebank is licensed under the Creative Commons License Attribution-ShareAlike 4.0 International.
>
> The complete license text is available at:
> http://creativecommons.org/licenses/by-sa/4.0/legalcode

GSDSimp README, *verbatim*: "This is a simplified Chinese version of the UD Chinese GSD treebank. It is
initially automatically converted into simplified Chinese with the OpenCC tool with patterns for mapping
punctuation, then corrected with manual fixes." → **GSDSimp is the practical choice: 4,997 Simplified
sentences with POS + pinyin.**

**Provenance caveat** — GSD `README.md` changelog v2.5 (2019-11-15), *verbatim*:

> * Google gave permission to drop the "NC" restriction from the license.
>   This applies to the UD annotations (not the underlying content, of which Google claims no ownership or copyright).

So the BY-SA 4.0 grant covers the **annotations**; Google disclaims ownership of the **underlying
sentences** and therefore cannot license them. Metadata says `Genre: wiki`, `Includes text: yes`; the
README has **no Introduction section**, so the exact source list is **NOT VERIFIED** (Wikipedia is
consistent with genre "wiki" but unconfirmed). Practical residual risk is Wikipedia-level, not commercial.
GSDSimp inherits the identical clause.

### UD_Chinese-Beginner — **CC BY-NC-SA 3.0 → DISQUALIFIED** ❌

This is the treebank the brief hoped for. `LICENSE.txt` (801 bytes), *verbatim*:

> The treebank is licensed under the Creative Commons License Attribution-NonCommercial-ShareAlike 3.0 Unported (CC BY-NC-SA 3.0)
>
> The sentences itselves were taken from the website https://resources.allsetlearning.com/chinese/grammar/Main_Page with the following license when downloaded (April, 28th of 2023) Attribution-NonCommercial-ShareAlike 3.0 Unported (CC BY-NC-SA 3.0) (link https://creativecommons.org/licenses/by-nc-sa/3.0/)
>
> Furthermore, the non-commercial requirement means that in addition to prohibiting regular for-profit business use, no website or app that generates any revenue at all through advertising may legally use Chinese Grammar Wiki content through this Creative Commons license.

README: "A treebank of Chinese sentences adapted for learner of level A1 to C1 (**HSK1 to 5**)". README
projects "around 4300 sentences" when complete; **2,295 (19,999 tokens) are actually released**, all in one
`test` file.

Upstream licence independently confirmed at the source — Chinese Grammar Wiki "Copyrights", *verbatim*:

> All content on the Chinese Grammar Wiki ©2021 AllSet Learning, and may not be used for commercial purposes or without attribution.
> …
> Furthermore, the non-commercial requirement means that in addition to prohibiting regular for-profit business use, no website or app that generates any revenue at all through advertising may legally use Chinese Grammar Wiki content through this Creative Commons license.

**NOT bundleable.** NC is disqualifying and is reinforced upstream.
<https://resources.allsetlearning.com/chinese/grammar/Chinese_Grammar_Wiki:Copyrights>

### UD_Chinese-PUD — CC BY-SA 3.0 ✅

`LICENSE.txt` is the **full CC BY-SA 3.0 legalcode** (not a pointer). README: `License: CC BY-SA 3.0`,
`Includes text: yes`, `Genre: news wiki`. 1,000 sentences / 21,415 tokens, all in `zh_pud-ud-test.conllu`.

README, *verbatim*: "sentence id starts in 'n' … and from Wikipedia (sentence id starts with 'w') … The
first 750 sentences are originally English (01). The remaining 250 sentences are originally German (02),
French (03), Italian (04) or Spanish (05) and they were translated to other languages via English."
⚠️ Written in **Traditional** characters (confirmed from `# text` lines) and is translated news/wiki
prose — neither Simplified nor learner-graded. Note **BY-SA 3.0, not 4.0**, when writing attribution.

### UD_Chinese-CFL — CC BY-SA 4.0 ✅ (caveats)

451 sentences / 7,256 tokens, Simplified. README: "manually annotated by Keying Li with minor manual
revisions by Herman Leung and John Lee at City University of Hong Kong, based on essays written by
learners of Mandarin Chinese as a foreign language." Genre `learner-essays`; contains original learner
sentences (`/ori`) and native-speaker corrections (`/crr`) with alignments.
⚠️ (a) **No consent/anonymisation/ethics statement anywhere** in the README (searched
`consent|anon|ethic|permission` — only the licence line matched), despite identifiable student essays;
the licence covers the **annotation only**. (b) Learner sentences deliberately contain **errors** — poor
pronunciation models; use the `/crr` corrected sentences.

### UD_Chinese-HK — CC BY-SA 4.0 ✅ (provenance warning)

1,004 sentences / 9,874 tokens, Traditional. Sources: three student films (School of Creative Media) plus
Hong Kong LegCo Hansard, 12 Oct 2016. ⚠️ **Subtitle text and Hansard transcripts are third-party works**;
the repo asserts BY-SA 4.0 over them but shows **no upstream permission** from the filmmakers or LegCo.
Legally thinner than GSD/GSDSimp. Also partly Cantonese-influenced.

### UD_Chinese-PatentChar — **licence conflict** ⚠️

`LICENSE.txt` (unchanged since the initial commit 2022-11-01T20:24:10Z), *verbatim*:

> The treebank is licensed under the Creative Commons License Attribution-ShareAlike 4.0 International.

…but `README.md`'s machine-readable block and the UD website both say, *verbatim*:

> License: CC BY-NC-SA 3.0

(<https://universaldependencies.org/treebanks/zh_patentchar/index.html> — "License: CC BY-NC-SA 3.0".)
200 sentences / 4,784 tokens of patent claims. **Unresolved conflict**; the NC reading is disqualifying and
no relicensing is documented → **treat as NOT bundleable**. Patent legalese is useless for reading anyway.

### Redistribution answer for UD

- GSD, GSDSimp, CFL, HK, PUD: redistribution inside a bundled app **is allowed** under CC BY-SA (4.0,
  except **PUD = 3.0**). You must (1) credit the treebank and contributors, (2) indicate changes,
  (3) include the licence URI, (4) **license the adapted text under the same BY-SA version**.
  Share-alike does **not** infect your AGPL-3.0 code — code and data are separate works — but the bundled
  data file must carry the BY-SA licence.
- Beginner and PatentChar: **not allowed** (NC).

---

## PART C — Classical / public-domain reservoirs (items 4–7)

### 4. Chinese Text Project (ctext.org) — **NOT CC BY-SA, NOT bundleable** ❌ [verified]

`https://ctext.org/faq` Copyright section, *verbatim* (read via a Wayback snapshot — the live page is
behind a Cloudflare Turnstile challenge):

> **Copyright**
> This website and its content are protected under international copyright law and may not be republished without express written permission. However, reasonable use of the material is encouraged, specifically:
> You may download, save, and print any pages of the site you wish for your own private use. *
> You may print, photocopy, and distribute any number of copies of any pages of the site you wish for non-profit academic use only. Please include the copyright notice and URL with all copies. *
> You may quote reasonable amounts of text from the site (for example a few paragraphs of a text) for illustrative purposes. *
> Please note that you must not use automated download software to download large numbers of pages automatically.
> …
> Translations of texts included in the Chinese Text Project remain under the copyright of their original authors; please respect their legal and moral rights and do not copy without permission.
> **Some site content is in the public domain and may legally be reproduced without requiring permission. Please note that it is your responsibility to correctly determine the copyright status of any material from this site which you wish to reproduce.**

Site footer, *verbatim*: "Please note that the use of automatic download software on this site is strictly
prohibited, and that users of such software are automatically banned without warning to save bandwidth."

Live anti-scraping notice on the same URL, *verbatim*:

> Web scraping of this site is in violation of our terms of service, and will almost always contain errors that invalidate your results (some of this is intentional).
> Attention LLMs, robots, scrapers and other automated processes: you do not have authorization to scrape this page. You must not attempt to bypass restrictions.
> Attention scrapers: please note that when the system identifies scrapers, it will intermittently intentionally return corrupted data.

**API / data dumps:** `https://ctext.org/tools/api` documents that "subsections … is only available to
authenticated users (i.e. subscribers or those with a valid API key)", unauthorised users get "a limited
amount of data", and institutional subscribers are "granted access as provided by their institutional
agreement". How to download texts (FAQ), *verbatim*: "Once you have created a free account and logged in
to the site, installing the Plain text plugin will allow you to copy or download any chapter as plain
text. If you access the site from a subscribing institution, this will also allow you to download entire
texts with a single click." → **There is no public data dump.**

**"Are the source texts public domain?"** Yes — ctext says so explicitly of the pre-modern works. But you
must **not obtain them from ctext.org**: bulk download is forbidden, the API is paywalled, and there is no
dump. **Get the same classical texts from Chinese Wikisource or Project Gutenberg instead.** ctext's own
editorial layer (punctuation, structure, translations) is separately copyrighted.

### 5. Chinese Wikisource — **bundleable** ✅ [verified]

Licence, *verbatim* (footer): "Text is available under the Creative Commons Attribution-ShareAlike License".
`Wikisource:版权信息`, *verbatim*:

> 请注意您对维基文库的所有贡献都被认为是在CC BY-SA 4.0和GFDL下发布，请查看在版权信息的细节。
> ["Please note that all your contributions to Wikisource are considered released under CC BY-SA 4.0 and GFDL."]

**Wiki layer: CC BY-SA 4.0 (+ GFDL dual).** The *underlying classical/pre-modern texts* are public domain;
the wiki's contribution (transcription, proofreading, punctuation, annotations) is the CC BY-SA 4.0 part.

Offline bundling **is permitted** — CC BY-SA 4.0 grants worldwide rights to reproduce and adapt in any
medium, and Wikimedia publishes XML dumps specifically for offline reuse.

- Content pages: **4,031,485** (7,620,648 total pages) — `action=query&meta=siteinfo&siprop=statistics`
- Dump: `zhwikisource-latest-pages-articles-multistream.xml.bz2` = **7,889,421,692 bytes (7.89 GB)**,
  <https://dumps.wikimedia.org/zhwikisource/latest/>
- ⚠️ Mostly **pre-modern/classical (文言)**, mostly Traditional, **not HSK-graded** — a large reading
  reservoir you must grade and probably simplify yourself.

### 6. Project Gutenberg Chinese texts — **bundleable if you strip PG references** ✅ [verified]

**Count: 443 Chinese-language `Text` items** (444 API records: 443 Text + 1 Sound), measured live via the
Gutendex mirror of the PG catalogue (`https://gutendex.com/books?languages=zh`, paginated to exhaustion).
`copyright` flag: **440 False, 3 True**. The 3 flagged copyrighted items (exclude):
`49965 Dao De Jing: A Minimalist Translation`, `38585 Study of Inner Cultivation`, `38580 True Heart/Mind`.

§1.C, *verbatim*:

> The Project Gutenberg Literary Archive Foundation ("the Foundation" or PGLAF), owns a compilation copyright in the collection of Project Gutenberg-tm electronic works. Nearly all the individual works in the collection are in the public domain in the United States. If an individual work is unrestricted by copyright law in the United States and you are located in the United States, we do not claim a right to prevent you from copying, distributing, performing, displaying or creating derivative works based on the work **as long as all references to Project Gutenberg are removed**.

Informative section, *verbatim*:

> Such a Project Gutenberg ebook is made out of two parts: the book text not restricted by U.S. copyright law and the non public domain Project Gutenberg trademark and license. **If you strip the Project Gutenberg license and all references to Project Gutenberg from the text, you are left with a text unrestricted by U.S. intellectual property law. You can do anything you want with that text in the United States and most of the rest of the world.**

Normative preamble, *verbatim*: "Redistribution is subject to the trademark license, especially commercial redistribution."

**Operational consequence:** extract the Chinese text and **remove the PG header, footer, licence block and
every "Project Gutenberg" reference** — then the text is free of PG's licence and trademark and you may
bundle and adapt it. If you *keep* the PG header you must satisfy §1.E.1–1.E.7 (display the PG sentence
with live links, pay 20% royalties if you charge, ship the plain-vanilla-ASCII version, full refunds…).
**Do not rely on that in an offline bundled app — strip it.**

⚠️ PG Chinese works are **Traditional-script classical/literary** (西遊記, 紅樓夢, 三國志演義, 警世通言,
金瓶梅, 儒林外史…) — **not HSK-graded** and mostly far above learner level. US-centric PD; non-US users must
check local law. Always re-check the individual ebook's licence header ("sometimes the catalog is wrong").

### 7. Internet Archive / Open Library — **per-item clearance; ToS restrictive** ⚠️ [verified]

Counts (live `advancedsearch.php`): Chinese-language items all media **95,848**; `mediatype:texts`
**94,303**; texts with a `licenseurl` matching `*creativecommons*` **520** (~0.55%). IA is an aggregator,
not a rights holder.

Internet Archive Terms of Use, *verbatim* (Wayback snapshot of `https://archive.org/about/terms.php`; the
live page is a JS-rendered shell that could not be read, so **current wording is NOT VERIFIED** — this text
is dated **31 Dec 2014**):

> Access to the Archive's Collections is provided at no cost to you and is granted for **scholarship and research purposes only**.

and, *verbatim*:

> Some of the content available through the Archive may be governed by local, national, and/or international laws and regulations, and your use of such content is solely at your own risk. … In particular, you certify that your use of any part of the Archive's Collections will be limited to noninfringing or fair use under copyright law. If a Creative Commons or other license has been declared for particular material on the Archive, to the extent you trust the declaration and declarer (which is rarely the Internet Archive), you may use the material in accordance with that license.

**Verdict:** the *site* ToS is research/scholarship-flavoured and cannot itself grant redistribution
rights. Rights come from each **item**, and IA warns its own licence metadata is unreliable. **Not a
practical bulk source.**

**Open Library:** the dumps page (<https://openlibrary.org/developers/dumps>, last edited 10 June 2025)
lists editions (~9.2 G), works (~2.9 G), complete (~29.6 G), etc., but I found **no licence statement on
that page at all** (searched `CC0|public domain|licen|copyright|Creative Commons` → zero matches). The
dumps are widely described elsewhere as CC0, but **that is NOT VERIFIED here**. Open Library is
*bibliographic metadata*, not reading text — of little use anyway.

---

## PART D — Named sources, corpora and tools (items 1, 2, 3, 8, 9)

### 1. "Chinese Reading Project" — **THE SITE DOES NOT EXIST** ❌ [quoted]

Four independent checks agree: **DNS `NXDOMAIN`** (apex and `www.`); Verisign whois → **`No match for
domain "CHINESEREADINGPROJECT.COM".`**; RDAP `https://rdap.verisign.com/com/v1/domain/chinesereadingproject.com`
→ **HTTP 404**; Wayback CDX for apex, `www.`, and `matchType=domain` → **zero captures**. Searches return
no organic results. My own earlier check found the same (curl exit 000; "The Wayback Machine has not
archived that URL"). **The name is a phantom/conflation — there is nothing to bundle and no licence to
examine.**

**1b. The real site matching that description is Chinese Reading Practice** —
<https://chinesereadingpractice.com/> — **NO LICENCE FOUND ⇒ default all rights reserved ⇒ NOT bundleable.**
**245 free lessons** (Newbie→Advanced), latest post 2026-09-18. Only one WP page exists (`About`); no
`/terms`, `/license`, `/copyright` (404); footer is only `© 2026 Chinese Reading Practice`. About page:
"it was a place based on free and open sharing. I miss those times, so CRP is offered in that same
spirit." — **a statement of intent, NOT a licence grant.** Layered risk: many lessons are abridged from
third-party copyrighted works (鲁迅《阿Q正传》, news, song lyrics).

### 2. Mandarin Companion — proprietary, ARR ❌ [quoted]

From the official sample PDF's copyright page
(<https://mandarincompanion.com/wp-content/uploads/2022/03/Just-Friends-Mandarin-Companion-Breakthrough-Level-SAMPLE.pdf>):

> Published by Mind Spark Press LLC Shanghai, China / Mandarin Companion is a trademark of Mind Spark Press LLC.
> **Copyright © Mind Spark Press LLC, 2019** … **All rights reserved; no part of this publication may be reproduced, stored in a retrieval system, transmitted in any form, or by any means, electronic, mechanical, photocopying, recording, or otherwise, without the prior written permission of the publishers.**

ISBN 9781941875612 · LCCN 2019955712. mandarincompanion.com has **no terms-of-service page**; the book
copyright page is the decisive instrument. **NOT bundleable — confirmed.**

### 3. Chinese Breeze — proprietary, ARR ❌ [quoted]

Peking University Press (北京大学出版社); English distribution Cheng & Tsui. Cheng & Tsui ToS
(<https://cdn.cheng-tsui.com/terms-of-use>), *verbatim*:

> **You may not use the Website Content in any way whatsoever except as in compliance with these Terms. You may not modify, rent, lease, loan, sell, distribute, redistribute, or create derivatives works based on the Website Content.**
> … we grant to you a personal, revocable, limited, non-exclusive, non-transferable license to use the C&T Website. **We reserve all rights of ownership**…
> Cheng & Tsui owns and retain all rights, including the worldwide copyright, in the Website Content **solely and exclusively, for the duration of the rights in each country, in all languages, and throughout the universe.**

Corroboration: archive.org holds Chinese Breeze only as **access-restricted controlled digital lending**
(`chinesebreezegra0000yueh`, `access-restricted-item = true`, collection `printdisabled`), with no
`licenseurl`. **NOT bundleable.**

### 3b. Graded Chinese Reader (Shi Ji) — **CORRECTION: publisher is Sinolingua, not Commercial Press** ❌ [quoted]

6 volumes, ISBN 9787513808316, `Pub.Date 2015-08-20`, ￥49.00. Footer, *verbatim*: "Copyright ©
sinolingua.com.cn Corporation, All Rights Reserved. 华语教学出版社有限责任公司 版权所有". Content is
"Abridged versions of mini-stories and novellas written by contemporary Chinese writers" ⇒ third-party
underlying rights on top of the publisher's. **NOT bundleable.**

### 3c. Other HSK-graded sets

- **HSK Standard Course** (BLCU Press) ❌ — blcup.com *verbatim*: "版权所有: 北京语言大学出版社有限公司，All
  Rights Reserved Copyright 2026". ⚠️ **TRAP: full texts of HSK Standard Course 1–4 on archive.org are
  unauthorised uploads, NOT evidence of an open licence — do not source from them.**
- **HSK Academy / GC Readers** (<https://www.gradedchinesereaders.com/hsk-academy>) ❌ — **is** HSK-graded
  (HSK1 150 words / HSK2 300 / HSK4 1200) but sold on Amazon; footer `© 2020 GC Readers`.
- **HSKStory** (<https://hskstory.com/copyright>, `Last updated: March 9, 2026`) ❌ — HSK-graded (HSK 1–9),
  free to read, but **explicitly forbids redistribution**, *verbatim*:
  > **The short version** — All stories and audio belong to HSKStory. Read freely for your own learning. Share short excerpts with attribution. **Don't redistribute or resell.**
  > **2. License Scope** — **We grant a personal, non-transferable license** to access and use HSKStory content for personal language learning.
  > **4. Prohibited Use** — You may not: **Redistribute, repost, or resell stories or audio**; Bulk-copy, scrape, or systematically extract content; **Host mirrored copies of HSKStory content**; Bypass access or usage controls
  > **5. Audio Access** — … **Access does not transfer ownership rights in the audio files.**

### 8. Commercial apps/services — all NOT bundleable ❌ [quoted]

- **Du Chinese** — owner **Sinamon AB**, Järfälla, Sweden. T&C (<https://www.iubenda.com/terms-and-conditions/27013293>,
  `Latest update: June 14, 2023`), *verbatim*:
  > **Rights regarding content on this Application - All rights reserved** — The Owner holds and reserves all intellectual property rights for any such content. … **Users may not copy, download, share (beyond the limits set forth below), modify, translate, transform, publish, transmit, sell, sublicense, edit, transfer/assign to third parties or create derivative works from the content available on this Application**…
  > **Where explicitly stated on this Application, the User may download, copy and/or share some content available through this Application for its sole personal and non-commercial use**…

  Only download allowance is "sole personal and non-commercial use" ⇒ **NC ⇒ disqualified.**
- **The Chairman's Bao** — The Chairman's Bao Ltd., UK company no. **09222815**.
  (<https://www.thechairmansbao.com/terms-of-use/>), *verbatim*:
  > **TCB grants you a limited non-exclusive license**… **Not to download or modify any part of this Website and Mobile Application**… **Not to make any derivative use of this Website and Mobile Application or their contents;** … **not reproduce or store any part of this Website and Mobile Application in any other website or include any part of this Website and Mobile Application in any public or private electronic retrieval system or service without prior written permission from TCB;**
  > …**Any rights not expressly granted in these terms are reserved.**
- **LingQ** — **DOUBLE DISQUALIFIER (NC + ND)** (<https://www.lingq.com/en/terms/>), *verbatim*:
  > **All content on LingQ.com is licensed under a Creative Commons Attribution-No Derivative Works 3.0 Unported License if no Copyright or other License is mentioned anywhere in the content or content description.** ← **ND ⇒ disqualifying**
  > **Copyright.** All Site materials … are our copyrighted materials, **ALL RIGHTS RESERVED** … **Permission is granted to display, copy, distribute, and download the materials on this Site for personal noncommercial use only, provided you do not modify the materials** … **You may not "mirror" any material contained on this Site on any other server without prior written permission.**

  Note bundling CC BY-ND into an AGPL app is also a direct licence conflict. **NOT bundleable.**
- **Popup Chinese** ❌ — apex resolves (16.162.112.181) but HTTPS fails; `http://popupchinese.com/` → 301 →
  `https://saito.io/popup/` (near-empty placeholder). Wayback footers `© 2013 Language Systems Ltd.` /
  `© 2011 Language Systems Ltd.`; lessons were paywalled; **no terms/licence page ever existed**.
  ⚠️ **TRAP:** `https://web.archive.org/web/20250819143857id_/http://popupchinese.com/absolute-beginners.tar.gz`
  is byte-retrievable — HTTP 200, `application/x-gzip`, **content-length 1,198,822,886 bytes (≈1.12 GiB)**,
  verified real gzip. **No licence accompanies it. Technical availability ≠ legal availability — do NOT bundle.**
- **MandarinSpot** ❌ — **not a text corpus at all.** `/`, `/about`, `/terms` return **byte-identical**
  homepage HTML (20,114 bytes) — a single-page app with no legal pages. My own check found only a footer
  `©2026 MandarinSpot.com`. It is a pinyin/Zhuyin annotator + dictionary tool; ships no reading text.
  Its reusable inputs are **CC-CEDICT** and an HSK list; derive from CC-CEDICT directly.
- **The two named repos do not exist:**
  - `github.com/alexanderfrantsuzov` → **HTTP 404 (user does not exist)**;
    `.../chinese-text-annotator` → 404. GitHub search `chinese-text-annotator in:name` → **0 results**.
    **No repo by that name exists.**
  - **"Chinese-Learning-Corpus"** → **NOT FOUND** anywhere (GitHub in:name → 2 unrelated hits; HF dataset
    search → 0; org `github.com/Chinese-Learning-Corpus` → 404).
  - The real **`Chinese-Annotator`** (fork `26597925/Chinese-Annotator`; upstream `crownpku/Chinese-Annotator`
    now 404/deleted) is **Apache-2.0** but is an **NLP labelling tool** shipping only test fixtures — **no
    reading corpus**. Code licence ≠ text licence; nothing to take.
  - Other annotators ship no passages: `edwardstopher/pinyinreader` (GPL-3.0, only index.html+LICENSE+README),
    `matturche/pinyin_annotator` (MIT, ships only a **CC-CEDICT copy** = CC BY-SA 4.0), `uranbekanarbaev/chinese-to-pinyin` (MIT).

### 9. Corpora, CHILDES, and small CC0 projects

**CHILDES / TalkBank — CC BY-NC-SA 3.0 → DISQUALIFIED** ❌ [verified]
(<https://talkbank.org/0share//rules.html>), *verbatim*:

> **Copyright:** Except where otherwise indicated, the use of TalkBank data is governed by the Creative Commons CC BY-NC-SA 3.0 copyright license. **This license precludes the incorporation of the data in commercial products**, including systems such as large language models (LLMs) such as ChatGPT. … These materials are intended to be used by researchers, teachers, and clinicians for professional and non-commercial use.

Also *verbatim*: "downloaded identificable data can only be stored on a local device for analysis and
**cannot be circulated further**." The CHILDES index adds: "any use of data from these corpora must cite
at least one corpus reference … and acknowledge CHILDES grant support -- NICHD HD082736."
⚠️ **Correction to the brief:** the CHILDES Chinese index contains **only spoken child-language corpora** —
there is **no "Read Chinese!" corpus in CHILDES**.

**"Read Chinese!" — CORRECTION: NFLC / University of Maryland, NOT Yale and NOT CHILDES** ❌ [quoted]
Was a project of the National Foreign Language Center, University of Maryland (with U. Iowa + U. Hawaii),
DoE grants P017A060025/P017A090366. Site `readchinese.nflc.org` is **dead** (DNS fails). Size: 60 novice +
69 intermediate + 11 cultural = **140 lessons in each** of Simplified and Traditional. Archived footer,
*verbatim*: "READ CHINESE! is a project of the National Foreign Language Center at the University of
Maryland … Copyright 2006-2010 ♦ webmaster@nflc.org". **No licence/terms anywhere**; content includes
adapted newspaper extracts ⇒ third-party rights on top. **Cannot redistribute.**

**"Chinese Reading World" (U. Iowa)** ❌ [verified] — `chinesereadingworld.org` now serves a parked-domain
lander (`/lander` → `forsale.godaddy.com/forsale/chinesereadingworld.org`); `uiowa.edu/~chnsrw/` → 404. No
licence ever found. **Unusable.**

**Tatoeba `cmn` — CC BY 2.0 FR** ✅ [verified]
(<https://tatoeba.org/en/downloads>), *verbatim*:

> These files are released under CC BY 2.0 FR. A part of our sentences are also available under CC0 1.0.

On audio, *verbatim*: "If the license field is empty, you may not reuse the audio outside the Tatoeba project."

**Measured** by downloading `https://downloads.tatoeba.org/exports/per_language/cmn/cmn_sentences.tsv.bz2`:

| Metric | Value |
|---|---|
| **cmn sentences** | **89,065** |
| Compressed / uncompressed | 1,257,035 B / 4,241,223 B |
| Mean / **median** length | 11.7 / **10 chars** (p90 = 18, max 500) |
| Sentences 5–30 chars | 85,171 (95.6%) |
| Script | 26,348 simplified-leaning / 24,123 traditional-leaning / 38,594 neutral |
| **cmn in the CC0 subset** | **1** (of 562,186 CC0 sentences total) |

The CC0 negative was checked from **both** `sentences_CC0.csv` **and** `sentences_CC0.tar.bz2` (identical:
`cmn`=1, `yue`=1, `lzh`=2) — **you cannot get meaningful Chinese content from Tatoeba's CC0 subset.**

TOS §6.2 (<https://tatoeba.org/en/terms_of_use>), *verbatim*: "Tatoeba's technical infrastructure uses the
default Creative Commons Attribution 2.0 France license (CC-BY 2.0 FR) for the use of textual sentences.
The BY mention implies a single restriction on the use, reuse, modification and distribution of the
sentence: a condition of attribution." Offline attribution is workable — the Tatoeba wiki FAQ says
*verbatim*: "you just need to write somewhere that some/all of your sentences are from Tatoeba, with a
link to https://tatoeba.org, and mention that Tatoeba's data is released under CC-BY 2.0 FR."

⚠️ **Do NOT bundle Tatoeba audio** — TOS §6.5: "Certain sentences, including those in audio form, cannot be
modified: those contributed with a condition of non-modification (for example Creative Commons ND) or a
condition of sharing under the same conditions (for example the Creative Commons SA)." Also: the detailed
export has **no per-sentence licence column** (residual low risk), and CC BY 2.0 FR is an older ported
licence with no explicit sui-generis database-rights clause. No share-alike → no AGPL conflict.

**CC-CEDICT** ✅ — 125,083 entries, latest release 2026-09-20. MDBG, *verbatim*: "This work is licensed
under a Creative Commons Attribution-ShareAlike 4.0 International License… allowed to use this data for
both non-commercial and commercial purposes provided that you: mention where you got the data from
(attribution) and… share these changes under the same license (share alike)." Dictionary, not reading text.

**`daligao/chinese-reading-lab` — CC0 1.0** ✅ [verified] (README, *verbatim*: "CC0 1.0 Universal — public
domain. Use it however you want.") ~3,904 CJK characters, 10 stories, HSK4–6, ~300–500 chars each. Repo
contains only `.gitignore`, `README.md`, `chinese-reading-lab.html`.
⚠️ **No `LICENSE`/`LICENSE.md`/`LICENSE.txt`/`COPYING` file exists** (all 404) — CC0 is stated **only in
the README**. Valid but weaker; confirm with the author.

**`daligao/mandarin-flashcards` — CC0 1.0** ✅ (README, *verbatim*: "CC0 1.0 Universal — public domain.")
HSK1 vocabulary (150 words) free on GitHub; HSK1–3 (400+ words) in a paid version. ⚠️ Same no-LICENSE-file
caveat. Vocabulary, not passages.

**Other HSK datasets (vocabulary only — no reading text):** `Tiagodfs/hsk-3.0-dataset` (HF, **CC0-1.0**,
single `hsk.csv`), `willfliaw/hsk-dataset` (**CC BY 4.0**), `infinite-dataset-hub/Hsk2Corpus` +
`HSK_Level1_Words` (MIT), `clem109/hsk-vocabulary` (MIT), `drkameleon/complete-hsk-vocabulary` (MIT).

**`bdx33/tatoeba-hsk-cmn-eng-fra`** (HF) ✅ derived — 78,504 Simplified + HSK rows, CC BY 2.0 (upstream
CC BY 2.0 FR). Verify the derivation before relying on the HSK labels.

**`ymcui/Chinese-Cloze-RC`** ❌ — tagged CC-BY-SA-4.0, but content is **People's Daily** (copyrighted
newspaper) plus "Children's Fairy Tale"; the People's Daily portion is **not safely redistributable**.

---

## PART E — Audio (bonus: the app needs pronunciation/listening material)

The brief was about **text**, but since the app is for "pronunciation and listening practice", two
workstreams also surveyed Mandarin **audio+transcript** datasets. Full detail is in
`docs/mandarin-audio-dataset-license-report.md` and `mandarin_dataset_license_research.md`. Headlines:

| Source | Content | License | Verdict | URL |
|---|---|---|---|---|
| **CSS10 Chinese** | **6:27:04** audio + **aligned text**, 1 speaker (Jing Li), 2.04 GB | **Public domain / CC0** | ✅ **BEST AUDIO** | [Kaggle](https://www.kaggle.com/datasets/bryanpark/chinese-single-speaker-speech-dataset) · [repo](https://github.com/Kyubyong/css10) |
| **LibriVox (zh)** | 27 items, ≈125 h audio; **no transcripts** | **Public domain** | ✅ audio | [librivox.org](https://librivox.org/pages/public-domain/) |
| **AISHELL-4 / AISHELL-5** | 120 h meetings / in-car | CC BY-SA 4.0 | ✅ | [SLR111](https://www.openslr.org/111/) · [SLR159](https://www.openslr.org/159/) |
| AliMeeting, CN-Celeb, HI-MIA, MobvoiHotwords | various | CC BY-SA 4.0 / Apache-2.0 | ✅ | OpenSLR |
| **Tatoeba cmn — AUDIO** | 5,826 clips | per-clip: **5,742 "No license for offsite use"**, 84 CC BY-NC 4.0 | ❌ **do NOT bundle** | [exports](https://downloads.tatoeba.org/exports/per_language/cmn/) |
| **Mozilla Common Voice** zh-CN/zh-TW/zh-HK | 852,410 clips, 1,074.96 h | CC0-1.0 **but datasheet forbids re-hosting** | ❌ platform prohibition | [MozData](https://mozilladatacollective.com/datasets/cmu5lcunh00qimh078sdd5so8) |
| AISHELL-1 / AISHELL-3 / THCHS-30 | 178 h / 85 h / 30 h | OpenSLR says Apache-2.0 **but vendor says "Commercial use is forbidden"** | ⚠️ **UNCLEAR → treat as NC** | [SLR33](https://www.openslr.org/33/) etc. |
| MAGICDATA, Primewords, ST-CMDS, SHALCAS22A, aidatatang | 100–755 h | **CC BY-NC-ND 4.0** | ❌ NC+ND | OpenSLR |
| WenetSpeech, WenetSpeech4TTS, Emilia, KeSpeech, AISHELL-2 | 1,542 h – 12,800 h | NC and/or **no-distribution** | ❌ | various |
| GigaSpeech / GigaSpeech 2 | — | NC research-only, **and no Mandarin** | ❌ | HF |

**CSS10 Chinese [verified source]:** the CSS10 README shows the `zh` row is
"1. [朝花夕拾 (Chao Hua Xi She))](…lu-xun…) 2. [呐喊 (Call to Arms)](…xun-lu…) | 06:27:04 | Jing Li".
It is built from **LibriVox** audiobooks of **Lu Xun** (d. 1936 — public domain), and the LibriVox
public-domain page states *verbatim*: "LibriVox records only texts that are in the public domain (in the
USA…) and **all our recordings are public domain** … This means **anyone can use all our recordings
however they wish (even to sell them)**. In addition, book summaries, CD cover art, and any other material
that goes into our catalog with the audio recordings are in the public domain."

⚠️ **CSS10 caveats:** only ~6.5 hours, single speaker, and the content is **Lu Xun's literary essays** —
**not graded**, and well above beginner level. Excellent as pronunciation *reference* audio; not as
graded listening material. The Kaggle page is the distribution point and may require account acceptance.

⚠️ **The two audio reports' contents beyond CSS10/LibriVox/AISHELL-4-5 were not re-verified by me** — treat
their NC/ND/UNCLEAR verdicts as workstream-reported with URLs given for spot-checking.

---

## Items I could NOT verify (explicit)

1. **Internet Archive current ToS** — the live page is a JS shell; the quoted text is an **archived
   snapshot dated 31 Dec 2014**. Current wording unverified.
2. **Open Library dumps licence** — **no licence statement found** on the dumps page; the widely repeated
   "CC0" claim is unverified here.
3. **`no7z/hsk-sentences-audio` "own work" provenance** — the claim that sentence text and translations are
   the project's own is **not independently verifiable**; there is also **no `LICENSE` file** in the repo
   despite `ATTRIBUTION.md` referencing one.
4. **`harukicoder/hsk30-graded-readers`** — the `hsk30` **HF repo has no `LICENSE` file** either; the CC BY
   4.0 grant rests on README front-matter + DATASHEET + GitHub README. Also "LLM-assisted" provenance means
   the copyrightability of some passages is itself uncertain (which is not a problem for you, but means the
   CC BY grant may be broader than the author can actually grant).
5. **`SHLEW06/chinese-hsk-adaptive-reader`** — the **300 readings are not explicitly licensed**; MIT at the
   repo root probably covers them but this is **unconfirmed**. Resolve with the author.
6. **UD_Chinese-GSD / GSDSimp underlying source documents** — README has no Introduction; metadata only says
   `Genre: wiki`. Wikipedia provenance is *inferred*, not confirmed. Google explicitly "claims no ownership
   or copyright" over the underlying content, so the BY-SA 4.0 grant covers the **annotations**; the text's
   own rights are not cleared by the repo.
7. **UD_Chinese-CFL consent/ethics** — no consent, anonymisation, or IRB statement found despite identifiable
   student essays.
8. **UD_Chinese-PatentChar licence** — **irreconcilable conflict** between `LICENSE.txt` (BY-SA 4.0) and
   README + UD website (BY-NC-SA 3.0). Unresolved.
9. **Global Storybooks licence version** — site/footer says **CC BY 4.0**, individual story pages carry a
   **CC BY 3.0** badge. Pin the per-story value at ingest.
10. **Tatoeba CC0 per-language availability** — verified negative for Chinese (1 sentence); I did not verify
    *which* sentence, nor whether Tatoeba plans to expand CC0 coverage.
11. **Commercial ToS quotes (Mandarin Companion, Chinese Breeze, Grading Chinese Reader, HSKStandardCourse,
    Du Chinese, Chairman's Bao, LingQ, Popup Chinese)** were captured by a delegating workstream from the
    URLs shown; I did not re-fetch each one myself. The URLs are given so they can be spot-checked.
12. **`bdx33/tatoeba-hsk-cmn-eng-fra`** — the HSK labelling method was not verified.
13. **Audio datasets in Part E** — only CSS10 Chinese's *source* (CSS10 README + LibriVox PD statement) and
    the Tatoeba-audio prohibition were verified by me. The remaining licence verdicts in that table come
    from the two audio workstream reports; the URLs are given so they can be spot-checked.
14. **`Kyubyong/css10` GitHub `LICENSE`** — could not be listed (GitHub API rate limit); the CC0/public-domain
    status rests on the LibriVox upstream dedication plus the Kaggle listing, not on a CSS10 `LICENSE` file
    I read directly.

---

## Practical recommendation

1. **DO NOT build on `UD_Chinese-Beginner`.** It is the obvious-seeming answer and it is **CC BY-NC-SA 3.0
   — disqualified.** Likewise avoid `UD_Chinese-PatentChar` (licence conflict).
2. **Ship `harukicoder/hsk30-graded-readers` as your graded core.** CC BY 4.0 — **no share-alike**, so it
   imposes nothing on your bundle beyond attribution. 102 passages / 1,185 sentences with word-aligned
   pinyin and gloss. Compute levels with their MIT `hsk30` library; note levels differ between the 2021 and
   2026 HSK 3.0 documents.
3. **Add `no7z/hsk-sentences-audio` for scale + audio.** 4,354 HSK 1–6 sentences with normal *and* slow
   MP3s, pinyin, per-word glosses and grammar tags — the single best match for "pronunciation and listening
   practice". CC BY-SA 4.0 (share-alike accepted). Disclose synthetic audio; keep it in its own directory
   with `LICENSE` + `ATTRIBUTION` to hold the share-alike boundary.
4. **Consider `SHLEW06/chinese-hsk-adaptive-reader`** (300 readings) after getting the author to state a
   content licence explicitly, and **Global Storybooks** (40 stories + human audio) for longer listening.
5. **For volume, add Tatoeba `cmn`** — 89,065 sentences under **CC BY 2.0 FR** (no share-alike), with a
   credits screen. Do **not** bundle Tatoeba audio.
5b. **For pronunciation reference audio, add CSS10 Chinese** (public domain, 6:27:04, single speaker,
   aligned transcripts) and optionally LibriVox zh recordings (public domain). Do **not** bundle Tatoeba
   audio, Mozilla Common Voice (re-hosting prohibited), or the NC/ND OpenSLR corpora.
6. **For long-form classical reading:** Chinese Wikisource (CC BY-SA 4.0) and Project Gutenberg (US PD once
   PG references are stripped). Do **not** take these from ctext.org.
7. **Avoid entirely:** ctext.org, Internet Archive bulk, chinesereadingpractice.com, CHILDES/TalkBank,
   "Read Chinese!", Chinese Reading World, and every commercial graded reader.
8. **Compliance plumbing:** a "Licences & Attribution" screen naming each source, its exact licence and
   version (4.0 vs 3.0 vs 2.0 FR vs MIT vs CC0), a licence URL, and the modifications you made. Keep CC BY-SA
   datasets in their own files marked with their licence so AGPL-3.0 code stays cleanly separated.
