# License research: the 6 named Chinese graded-reading sources (bundleability)

Scope: the specific sources named in the request — Chinese Reading Project, Mandarin Companion,
Chinese Breeze, Graded Chinese Reader (Shi Ji), "New HSK"/HSK Academy/HSK Standard Course,
and Du Chinese / Chairman's Bao / LingQ / Popup Chinese / MandarinSpot.

Target: offline desktop/mobile app, **AGPL-3.0**, wants to **bundle** short graded Chinese
passages/phrases (HSK-graded) for pronunciation + listening practice.
CC BY-SA and share-alike are acceptable with attribution. **NC / ND / research-only are disqualifying.**
Public domain is ideal.

Research date: 2026-09-21. Method: live HTTP (`curl`), Wayback CDX, Verisign RDAP/whois,
GitHub API + `raw.githubusercontent.com`, HuggingFace API, publisher pages, book copyright pages.

> **Companion file:** a sibling agent's `license-research-chinese-graded-reading.md` in this repo
> covers open corpora (Universal Dependencies Chinese treebanks, kaikki/Wiktionary extracts, etc.).
> This file covers **only the six named sources** and does not duplicate that work.

---

## TL;DR

- **None** of the six named sources is bundleable. Every one is all-rights-reserved, NC-only, or ND.
- **Two genuinely bundleable sources were found while resolving item 5** (both attribution-compatible,
  no NC, no ND):

| Source | Content | Grading | License |
|---|---|---|---|
| **`no7z/hsk-sentences-audio`** (HuggingFace) | 4,354 sentences + 8,708 MP3s + pinyin/gloss/translation | **Official HSK 3.0 levels 1–6** | **CC BY-SA 4.0** |
| **Global Storybooks 中文故事集** (`global-asp/storybooks-chinese`) | 40 stories, text + MP3 + PDF | 5 length/vocab levels (not HSK) | **CC BY 4.0** (site) / **CC BY 3.0** (story pages) |

- Supplementary, unambiguously open but **not** HSK-graded: **Tatoeba** cmn sentences (CC BY 2.0 FR),
  **CC-CEDICT** (CC BY-SA 4.0).

---

## Per-source summary table

| # | Source | Content type | HSK-graded? | License | Redistribution verdict | URL |
|---|---|---|---|---|---|---|
| 1 | Chinese Reading Project | — | — | **no site exists** | ❌ N/A | `chinesereadingproject.com` (unregistered) |
| 1b | Chinese Reading Practice | Short passages + pinyin + translation | No (Newbie→Advanced) | **no license found** | ❌ **NOT bundleable** (default ARR) | https://chinesereadingpractice.com/ |
| 2 | Mandarin Companion | Graded-reader novellas | No (Breakthrough/L1/L2) | **proprietary, ARR** | ❌ **NOT bundleable** | https://mandarincompanion.com/ |
| 3 | Chinese Breeze | Graded-reader novellas | No (300/500/750/1100 words) | **proprietary, ARR** | ❌ **NOT bundleable** | https://cdn.cheng-tsui.com/terms-of-use |
| 4 | Graded Chinese Reader (Shi Ji) | Abridged modern short stories, 6 vols | No (ICCE 2008 / CLPS 2009 grades) | **proprietary, ARR** | ❌ **NOT bundleable** | http://www.sinolingua.com.cn/index.php?m=content&c=index&a=show&catid=10&id=779 |
| 5a | `no7z/hsk-sentences-audio` | HSK sentences + audio + pinyin/gloss | **YES — HSK 3.0 L1–6** | **CC BY-SA 4.0** | ✅ **BUNDLEABLE** | https://huggingface.co/datasets/no7z/hsk-sentences-audio |
| 5b | Global Storybooks 中文故事集 | 40 stories, text + MP3 + PDF | No (5 length/vocab levels) | **CC BY 4.0 / 3.0** | ✅ **BUNDLEABLE** | https://global-asp.github.io/storybooks-chinese/ |
| 5c | HSK Standard Course | Textbook series | Yes | **proprietary, ARR** | ❌ **NOT bundleable** | https://www.blcup.com/ |
| 5d | HSK Academy / GC Readers | HSK-vocab graded stories (Amazon) | **Yes** | **proprietary, commercial** | ❌ **NOT bundleable** | https://www.gradedchinesereaders.com/hsk-academy |
| 5e | HSKStory | HSK-levelled stories + audio | **Yes (HSK 1–9)** | **explicitly forbids redistribution** | ❌ **NOT bundleable** | https://hskstory.com/copyright |
| 6a | Du Chinese | HSK-graded lessons + audio | Yes | **proprietary, ARR** | ❌ **NOT bundleable** | https://www.iubenda.com/terms-and-conditions/27013293 |
| 6b | The Chairman's Bao | News-based graded lessons | Yes | **proprietary, ARR** | ❌ **NOT bundleable** | https://www.thechairmansbao.com/terms-of-use/ |
| 6c | LingQ | User/staff lessons + audio | No | **ARR + NC; fallback CC BY-ND 3.0** | ❌ **NOT bundleable** (NC **and** ND) | https://www.lingq.com/en/terms/ |
| 6d | Popup Chinese | Podcast lessons + transcripts | No | **ARR, © Language Systems Ltd** | ❌ **NOT bundleable** (defunct) | `popupchinese.com` → `https://saito.io/popup/` |
| 6e | MandarinSpot | **Annotator/dictionary tool — no corpus** | n/a | **no ToS found** | ❌ **nothing to bundle** | https://mandarinspot.com/ |

---

## 1. "Chinese Reading Project" / chinesereadingproject.com — **DOES NOT EXIST**

| Field | Finding |
|---|---|
| Content type | n/a — no site |
| HSK-graded? | n/a |
| License | **No license found — no site exists** |
| Redistribution verdict | **N/A (nothing to bundle)** |
| URL | `chinesereadingproject.com` — **unregistered** |

Evidence (all four independent checks agree):
- DNS: **`NXDOMAIN`** for both apex and `www.` (`nslookup` via 192.168.8.1).
- `whois -h whois.verisign-grs.com chinesereadingproject.com` → **`No match for domain "CHINESEREADINGPROJECT.COM".`**
- `https://rdap.verisign.com/com/v1/domain/chinesereadingproject.com` → **HTTP 404**, empty body.
- Wayback CDX for the apex, the `www.` host, and `matchType=domain` → **zero captures** (`[]` on all three).
  So it never had archived content either.
- DuckDuckGo HTML search for `"chinesereadingproject"` → **no organic results**.

**Conclusion: the name is a phantom / conflation.** The real, live site matching that description is
**Chinese Reading Practice** at `chinesereadingpractice.com` — see §1b.
(Two unrelated near-misses that are *not* this: *The Great Mandarin Reading Project* at
ignitechinese.org, a PDF guideline, and 阅读中国 (FLTRP) — neither is a `chinesereadingproject.com`.)

### 1b. Chinese Reading Practice (chinesereadingpractice.com) — live, but **no license**

| Field | Finding |
|---|---|
| Content type | Short graded reading passages with pinyin + English translation |
| HSK-graded? | **No** — own 4 tiers: Newbie / Beginner / Intermediate / Advanced, plus topic tags |
| License | **No license found. No ToS, no copyright page, no CC statement** |
| Redistribution verdict | **NOT BUNDLEABLE — default all rights reserved** |
| URL | https://chinesereadingpractice.com/ |
| Size/count | **245 free lessons**; latest post **2026-09-18**; ~1 new lesson per weekday |
| Last updated | Lesson dated **2026-09-18** ("小明的减法课 – Xiao Ming's Subtraction Lesson") |

Evidence:
- Self-description: *"We're building the world's biggest resource of free Chinese reading study materials. As of today, CRP has a total of 245 free lessons."*
- `/wp-json/wp/v2/pages?per_page=50` returns **exactly one** page: `About`. **No** terms/licence/legal page exists.
- `/terms` and `/license` do not exist. Footer is only: `© 2026 Chinese Reading Practice · Powered by WordPress`.
- The About page states *"Plus, I'm old enough to remember the internet before big tech, and it was a place based on free and open sharing. I miss those times, so CRP is offered in that same spirit."*
  — this is a **statement of intent, not a licence grant**. It conveys no redistribution right.
- **Additional layered risk:** many lessons are abridged/translated from third-party copyrighted works —
  e.g. 鲁迅《阿Q正传》 / Lu Xun's *The True Story of Ah Q*, news items, and song lyrics
  (`famous-song 月亮代表我的心`). Even a hypothetical site-wide grant would not clear the underlying text.

**Verdict: "no license found" ⇒ treat as all rights reserved.** Not bundleable.

---

## 2. Mandarin Companion — COMMERCIAL, ALL RIGHTS RESERVED ❌

| Field | Finding |
|---|---|
| Content type | Full graded-reader novellas (book-length) — Breakthrough / Level 1 / Level 2 |
| HSK-graded? | **No** — own levels (e.g. Breakthrough ≈150 unique characters) |
| License | **Proprietary, all rights reserved** |
| Redistribution verdict | **NOT BUNDLEABLE** |
| Publisher | **Mind Spark Press LLC**, Shanghai, China |
| Authors | John Pasden, Jared Turner |
| URL | https://mandarincompanion.com/ |

**Verbatim, from the copyright page of the official sample PDF** (*Just Friends?*, Breakthrough Level,
Simplified Chinese Edition) — https://mandarincompanion.com/wp-content/uploads/2022/03/Just-Friends-Mandarin-Companion-Breakthrough-Level-SAMPLE.pdf
(the URL's `.pdf` served as `application/pdf`, HTTP 200, 517,116 bytes; text extracted with `pdftotext`):

> Published by Mind Spark Press LLC Shanghai, China
>
> Mandarin Companion is a trademark of Mind Spark Press LLC.
>
> Copyright © Mind Spark Press LLC, 2019
>
> For information about educational or bulk purchases, please contact Mind Spark Press at
> BUSINESS@MANDARINCOMPANION.COM.
>
> …
>
> **All rights reserved; no part of this publication may be reproduced, stored in a retrieval system,
> transmitted in any form, or by any means, electronic, mechanical, photocopying, recording, or
> otherwise, without the prior written permission of the publishers.**

Bibliographic detail from the same page:
`First paperback print edition 2019` · `ISBN: 9781941875612 (Paperback)` ·
`ISBN: 9781941875636 (Paperback/traditional ch)` · `ISBN: 9781941875629 (ebook)` ·
`Library of Congress Control Number: 2019955712` · `MCID: SSS20220926T174333`.

**Note:** `mandarincompanion.com` **has no terms-of-service page** — `/pages/terms-of-service`,
`/policies/terms-of-service`, `/pages/terms-of-use`, `/pages/copyright` all return **HTTP 404**;
the only policy page is `/privacy-policy-2/`, which contains no IP clause. The **book copyright page
above is therefore the decisive instrument.**

---

## 3. Chinese Breeze (汉语风) — COMMERCIAL, ALL RIGHTS RESERVED ❌

| Field | Finding |
|---|---|
| Content type | Graded-reader novellas, Level 1 (300 words) → Level 4+ (1100+ words) |
| HSK-graded? | **No** — word-count levels (300 / 500 / 750 / 1100 words) |
| License | **Proprietary, all rights reserved** |
| Redistribution verdict | **NOT BUNDLEABLE** |
| Publisher | **Peking University Press** (北京大学出版社); English-market distribution by **Cheng & Tsui** |
| Authors | Liu Yuehua (刘月华), Chu Chengzhi (储诚志) |

**Verbatim — Cheng & Tsui Terms of Use** (https://cdn.cheng-tsui.com/terms-of-use, HTTP 200):

> Conditioned on your compliance with these Terms, we grant to you a **personal, revocable, limited,
> non-exclusive, non-transferable license** to use the C&T Website. **We reserve all rights of ownership
> in and to the C&T Website not expressly granted to you.**

> **You may not use the Website Content in any way whatsoever except as in compliance with these Terms.
> You may not modify, rent, lease, loan, sell, distribute, redistribute, or create derivatives works
> based on the Website Content.** You may not alter or delete any proprietary notices from Website Content.

> Cheng & Tsui owns and retain all rights, including the worldwide copyright, in the Website Content
> **solely and exclusively, for the duration of the rights in each country, in all languages, and
> throughout the universe.**

Corroborating evidence (Internet Archive — books held **only** as access-restricted controlled digital lending):

| Identifier | Title | Publisher | Date | ISBN | Restricted |
|---|---|---|---|---|---|
| `chinesebreezegra0000yueh` | Level 4: *Vick The Good Dog* | **Peking University Press** | 2016-10-01 | 9787301275627 | `access-restricted-item = true`; collection `printdisabled` |
| `chinesebreezegra0000yueh_u8q4` | Level 1: *Wrong, Wrong, Wrong!* | **Peking University Press** | 2017-05-01 | 9787301282519 | `access-restricted-item = true`; collection `printdisabled` |

No `licenseurl` field on either record. (A separate identifier,
`256887017-wo-yiding-yao-zhaodao-ta-chinese-breeze-graded-reader-series-level-1-300-word-level`, is a
**user upload** in `booksbylanguage_chinese` with no publisher/licence metadata — an infringing copy,
**not** evidence of an open licence. Do not treat it as a source.)

---

## 4. Graded Chinese Reader (Shi Ji) — COMMERCIAL; **publisher is Sinolingua, not Commercial Press** ❌

| Field | Finding |
|---|---|
| Content type | Abridged mini-stories / novellas by contemporary Chinese writers; pinyin + English notes + MP3 CD |
| HSK-graded? | **No** — 6 grades keyed to *International Curriculum for Chinese Language Education* (2008) and *Chinese Proficiency Test Syllabus* (2009) |
| License | **Proprietary, all rights reserved** |
| Redistribution verdict | **NOT BUNDLEABLE** |
| Publisher | **华语教学出版社 / Sinolingua** — ⚠️ **NOT Commercial Press** |
| Author/editor | Ji Shi (Shi Ji) |
| URL | http://www.sinolingua.com.cn/index.php?m=content&c=index&a=show&catid=10&id=779 |
| Size/count | **6 volumes**; the listed volume is *Graded Chinese Reader 1000 words* |
| Dates | `Pub.Date 2015-08-20`; price **￥49.00**; **ISBN 9787513808316** |

**Correction to the brief:** the publisher is **Sinolingua (华语教学出版社)**, not Commercial Press
(商务印书馆). (商务印书馆 does publish other graded series, but not this one.)

Verbatim site footer:

> Copyright © sinolingua.com.cn Corporation, All Rights Reserved. 华语教学出版社有限责任公司 版权所有

Verbatim series description (shows the third-party rights layer):

> The series are divided into six grades based on the vocabulary in International Curriculum for
> Chinese Language Education(2008) and Chinese Proficiency Test Syllabus(2009).
> 1. **Abridged versions of mini-stories and novellas written by contemporary Chinese writers**,
> reflecting the everyday lives of ordinary Chinese people;
> …
> 5. Accompanied by original illustrations and a CD in MP3 format.

Because the text is an **abridgement of third-party contemporary fiction**, there are underlying
author rights on top of the publisher's — so even short excerpting is higher risk than a plain
publisher-owned work.

---

## 5. Free / open HSK-graded sets

### 5a. ✅ BUNDLEABLE — `no7z/hsk-sentences-audio` (HuggingFace) — **CC BY-SA 4.0**

| Field | Finding |
|---|---|
| Content type | HSK-graded sentences: hanzi (simplified + traditional), `pinyin`, `pinyin_numbered`, English translation, **per-word glosses**, grammar points/tags, and **normal + slow audio** for each |
| HSK-graded? | **YES — against the official HSK 3.0 levels 1–6** |
| License | **CC BY-SA 4.0** (applied dataset-wide, deliberately) |
| Redistribution verdict | ✅ **BUNDLEABLE** with attribution + share-alike (matches this project's CC BY-SA acceptance) |
| URL | https://huggingface.co/datasets/no7z/hsk-sentences-audio |
| Count | **4,354 sentences**; **8,708 MP3 files** (4,354 × {normal, slow}) |
| Counts by level | HSK 1: **281** · HSK 2: **538** · HSK 3: **727** · HSK 4: **801** · HSK 5: **965** · HSK 6: **1,042** |
| Size | `data/train.jsonl` **4.95 MB**; `data/train.parquet` **0.67 MB**; audio **≈18.2 MB per 995 files** → full audio **≈150 MB** (extrapolated; HF tree API caps at 1000 entries) |
| Dates | created **2026-07-14**; **last modified 2026-07-15**; 700 downloads |

Verbatim, `README.md`:

> 4,354 Chinese sentences graded against the official HSK 3.0 levels 1–6, with pinyin, English
> translations, per-word glosses, grammar tags, normal/slow synthetic speech. The complete export
> contains 8,708 MP3 files.

> Suitable for flashcards, shadowing, listening practice, SRS, pronunciation interfaces, and
> Chinese-learning research prototypes. Audio is synthetic (CosyVoice2-0.5B, `zh-female-studio`) and
> should be disclosed as such in downstream products.

> **License and attribution**
> Dataset: **CC-BY-SA-4.0**. Glosses and some pinyin data derive from CC-CEDICT (CC-BY-SA); audio was
> synthesized with Apache-2.0 CosyVoice2. See `ATTRIBUTION.md` in this dataset export for detailed
> provenance.

YAML front-matter: `license: cc-by-sa-4.0`.

Verbatim, `ATTRIBUTION.md`:

> 句子文本与英文翻译为本项目自有。
> 为简化合规，整个 `dist/` 数据集以 **CC-BY-SA 4.0** 发布。

("The sentence text and English translations are this project's own. To simplify compliance, the
entire `dist/` dataset is released under CC BY-SA 4.0.")

The same file lists the full dependency/licence chain:

| Component | Use | License |
|---|---|---|
| CC-CEDICT | word senses, per-word standard pinyin | CC BY-SA 4.0 |
| CosyVoice2-0.5B | audio synthesis (TTS) | Apache-2.0 |
| `pypinyin` | pinyin annotation | MIT |
| `jieba` | segmentation | MIT |
| OpenCC | simplified→traditional | Apache-2.0 |
| `complete-hsk-vocabulary` | HSK-levelled word list (validation only, not ingested) | MIT |

**Caveats to record before ingest:**
- Sentence text and English translations are **original to this project** → clean provenance. ✅
- Audio is **synthetic** (CosyVoice2-0.5B, `zh-female-studio`); README asks that synthetic speech be
  disclosed downstream. Voice-model IP stays with CosyVoice's authors; the project redistributes
  synthetic output under Apache-2.0.
- HSK levelling was validated against a **community-digitised** word list, not the official database.
  ATTRIBUTION.md self-reports *"与官方口径约有 ~1% 出入"* (~1% divergence from the official standard)
  and recommends spot-checking against the official query system:
  https://admin.chinesetest.cn/standardsAction.do?means=standardInfo
- Share-alike attaches to the dataset and its adaptations — **not** to the whole AGPL app. Keep the
  dataset in its own directory with a `LICENSE`/`ATTRIBUTION` file to keep the boundary clean.

### 5b. ✅ BUNDLEABLE — Global Storybooks 中文故事集 — **CC BY 4.0 / CC BY 3.0**

| Field | Finding |
|---|---|
| Content type | Children's picture-story text + human-read audio (MP3) + downloadable PDF |
| HSK-graded? | **No** — 5 levels by length/vocabulary (Level 1 ≤75 words … Level 5 ≥800 words) |
| License | **CC BY 4.0** (site-level) / **CC BY 3.0** (per-story badge) — attribution only; **no NC, no ND, no SA** |
| Redistribution verdict | ✅ **BUNDLEABLE** with attribution |
| Live site | https://global-asp.github.io/storybooks-chinese/ |
| Repo | https://github.com/global-asp/storybooks-chinese |
| Source trail | https://www.globalstorybooks.net/ contains `href="https://global-asp.github.io/storybooks-chinese/"` |
| Count | **40 stories** (verified: 40 unique `/stories/zh/NNNN/` links on the index) |
| Dates | repo HEAD `df012889a863b89f65717818929662c1ea92b2b5`; **last commit 2025-12-10**; 68 commits |

Verbatim, Global Storybooks FAQ, *"Can I reuse the content on Global Storybooks for other purposes?"*:

> **Global Storybooks is an open source project, and all content on this site has been released under
> an open license.** You can find more detailed information on our Source page.

Provenance of the stories (same FAQ, *"Why are so many of the stories from the African Storybook?"*):

> **The African Storybook initiative makes hundreds of stories freely available under the Creative
> Commons license**, providing picture storybooks for children's literacy, enjoyment and imagination.
> We are grateful to the South African organization, Saide, for making these wonderful stories freely
> available under an open license.

Chinese site footer: `© 中文故事集. 保留部份版权.` ("Some rights reserved"), hyperlinked to
`https://creativecommons.org/licenses/by/4.0/deed.zh` (**CC BY 4.0**).

Chinese site self-description:

> 《中文故事集》是一个完全免费而开放的教育资源，旨在促进社区语言的识字和读写能力 …
> **这40篇**译成各种不同语言的故事来自非洲故事书项目，以下按**长度和词汇量分成五个阅读级别**，
> 并为每一个语言版本录制语音故事以增加阅读的乐趣。

Audio/PDF availability (from the site's own *如何使用本站资源* page): per-sentence audio playback,
full-story audio, adjustable speed, and a **下载 PDF** (download PDF) button; MP3s are retrievable via
the audio control's save-as.

**⚠️ Verification flag — licence-version discrepancy.** The **site/footer level says CC BY 4.0**, but
**individual story pages carry a CC BY 3.0 badge** (`https://creativecommons.org/licenses/by/3.0/deed.zh`;
badge image `…/l/by/3.0/88x31.png`). Both are attribution-only and both are acceptable here, but the
version difference is real — **record the exact per-story licence at ingest time** rather than assuming 4.0.

**⚠️ Note on the repo `LICENSE` file.** `https://raw.githubusercontent.com/global-asp/storybooks-chinese/master/LICENSE`
is **MIT** — `Copyright (c) 2018 中文故事集`. That covers the **site code**, **not** the story content.
The content licence is the CC BY grant above. Upstream African Storybook (https://www.africanstorybook.org/)
states: `Creative Commons Licence CC-BY-4.0`.

### 5c. ❌ NOT OPEN — HSK Standard Course (BLCU Press)

Publisher: **北京语言大学出版社有限公司 / Beijing Language and Culture University Press**.
Verbatim footer of https://www.blcup.com/ :

> 版权所有: 北京语言大学出版社有限公司，All Rights Reserved Copyright 2026

Commercial textbook series by 姜丽萍 / Jiang Liping. **NOT BUNDLEABLE.**

⚠️ **Trap to avoid:** full texts of HSK Standard Course 1–4 are on `archive.org`
(`hsk-1-workbook_202404`, `hsk-4-standard-textbook`) as **unauthorised uploads**. These are infringing
copies and are **not** evidence of an open licence. Do not treat them as a source.

### 5d. ❌ NOT OPEN — HSK Academy / GC Readers (gradedchinesereaders.com)

| Field | Finding |
|---|---|
| Content type | HSK-vocabulary-controlled graded stories (*Xiaoming's Day* for HSK 1, *My Birthday* for HSK 2, *The Art of War* for HSK 4, …) |
| HSK-graded? | **Yes — explicitly HSK 1 / 2 / 4 vocabulary (150 / 300 / 1200 words)** |
| License | **Proprietary, commercial** — sold via Amazon (paperback / Kindle) |
| Redistribution verdict | **NOT BUNDLEABLE** |
| URL | https://www.gradedchinesereaders.com/hsk-academy |
| Size/count | HSK 1: **32 pp**, 172 unique words, **620 total characters** · HSK 2: **40 pp**, 362 unique words, 1,510 chars · HSK 4: **115 pp**, 1,003 unique words, 6,100 chars |

Page footer: `© 2020 GC Readers`. Every title is a paid Amazon listing (`Buy on Amazon`).
No open licence anywhere on the page. (The same site is a comparison directory for *other*
commercial series — Chinese Breeze, Imagin8, Mandarin Companion, Rainbow Bridge, Sinolingua —
all likewise commercial.)

### 5e. ❌ NOT OPEN — HSKStory (hskstory.com) — **explicitly forbids redistribution**

| Field | Finding |
|---|---|
| Content type | HSK-levelled short stories with audio |
| HSK-graded? | **Yes** — "HSK 1–9" |
| License | **Explicitly forbids redistribution** |
| Redistribution verdict | **NOT BUNDLEABLE** |
| URL | https://hskstory.com/copyright |
| Last updated | page states **`Last updated: March 9, 2026`** |

Verbatim, Copyright Policy:

> **The short version**
> All stories and audio belong to HSKStory. Read freely for your own learning. Share short excerpts
> with attribution. **Don't redistribute or resell.**

> **1. Ownership**
> HSKStory owns all rights in the platform, story text, audio narration, and related materials unless
> otherwise stated.
>
> **2. License Scope**
> **We grant a personal, non-transferable license** to access and use HSKStory content for personal
> language learning.
>
> **3. Short Excerpts**
> Short excerpts are allowed for sharing or review with attribution to HSKStory and a source link.
>
> **4. Prohibited Use**
> You may not:
> - **Redistribute, repost, or resell stories or audio**
> - Bulk-copy, scrape, or systematically extract content
> - **Host mirrored copies of HSKStory content**
> - Bypass access or usage controls
>
> **5. Audio Access**
> Audio is provided for streaming/listening use under this license. **Access does not transfer ownership
> rights in the audio files.**

"Personal, non-transferable" + explicit no-redistribution/no-mirroring ⇒ disqualified on two counts.

### 5f. Supplementary open corpora (NOT HSK-graded — for reference only)

**Tatoeba** — https://tatoeba.org/en/terms_of_use. Verbatim:

> Tatoeba's technical infrastructure uses the default **Creative Commons Attribution 2.0 France
> license (CC-BY 2.0 FR)** for the use of textual sentences. The BY mention implies a single
> restriction on the use, reuse, modification and distribution of the sentence: a condition of
> attribution.

Chinese export: `https://downloads.tatoeba.org/exports/per_language/cmn/cmn_sentences.tsv.bz2` —
**1,257,035 bytes (≈1.2 MB)**, `last-modified: Sat, 19 Sep 2026 06:31:32 GMT`.
**No HSK grading** in the export (would need local levelling); **sentence-level, not passage-level**.
Audio licences vary — *"audio sentences may be under other licenses than Creative Commons, especially
if the contributor has not validated the authorization to use the audio phrase elsewhere"* —
**verify per-clip before bundling any Tatoeba audio.**

**CC-CEDICT** — https://www.mdbg.net/chinese/dictionary?page=cc-cedict — **CC BY-SA 4.0**.
Dictionary/gloss data, not reading text; useful with attribution + share-alike.

---

## 6. Commercial platforms — terms of service / copyright status

### 6a. Du Chinese — ALL RIGHTS RESERVED ❌

| Field | Finding |
|---|---|
| Content type | Graded lessons (HSK 1–6+ and "New HSK" tracks) with native audio |
| HSK-graded? | **Yes** |
| License | **Proprietary, all rights reserved** |
| Redistribution verdict | **NOT BUNDLEABLE** |
| Owner | **Sinamon AB**, Kvarnvingevägen 2, 177 41 Järfälla, Sweden (`privacy@sinamon.org`) |
| URL | https://www.duchinese.app/legal → T&C hosted at https://www.iubenda.com/terms-and-conditions/27013293 |
| Date | **`Latest update: June 14, 2023`** |

Verbatim:

> **Rights regarding content on this Application - All rights reserved**
> The Owner holds and reserves all intellectual property rights for any such content.
> Users may not therefore use such content in any way that is not necessary or implicit in the proper
> use of the Service.
> In particular, but without limitation, **Users may not copy, download, share (beyond the limits set
> forth below), modify, translate, transform, publish, transmit, sell, sublicense, edit,
> transfer/assign to third parties or create derivative works from the content available on this
> Application**, nor allow any third party to do so through the User or their device, even without the
> User's knowledge.
> **Where explicitly stated on this Application, the User may download, copy and/or share some content
> available through this Application for its sole personal and non-commercial use** and provided that
> the copyright attributions and all the other attributions requested by the Owner are correctly
> implemented.
> Any applicable statutory limitation or exception to copyright shall stay unaffected.

> **Service reselling**
> Users may not reproduce, duplicate, copy, sell, resell or exploit any portion of this Application and
> of its Service without the Owner's express prior written permission, granted either directly or
> through a legitimate reselling programme.

> **Intellectual property rights**
> Without prejudice to any more specific provision of these Terms, any intellectual property rights,
> such as copyrights, trademark rights, patent rights and design rights related to this Application
> are the exclusive property of the Owner or its licensors and are subject to the protection granted by
> applicable laws or international treaties relating to intellectual property.

The only download allowance is **"sole personal and non-commercial use"** ⇒ **NC ⇒ disqualified**,
independently of the blanket no-copy/no-derivative prohibition.

### 6b. The Chairman's Bao (TCB) — ALL RIGHTS RESERVED ❌

| Field | Finding |
|---|---|
| Content type | Graded news-based lessons with audio, HSK 1–6+ |
| HSK-graded? | **Yes** |
| License | **Proprietary, all rights reserved** |
| Redistribution verdict | **NOT BUNDLEABLE** |
| Owner | **The Chairman's Bao Ltd.**, UK company no. **09222815** |
| URL | https://www.thechairmansbao.com/terms-of-use/ |

Verbatim, section "License and Conditions":

> **TCB grants you a limited non-exclusive license** to access and make use of this Website and Mobile
> Application and their features. As a condition of such license, you agree:
> - **Not to download or modify any part of this Website and Mobile Application**, except with the
>   express and prior written consent of TCB;
> - Not to download or copy any account information for the benefit of another party;
> - Not to collect or make any use of any product listings, descriptions, or prices;
> - **Not to resell or make any commercial use of this Website and Mobile Application or their contents;**
> - **Not to reproduce, duplicate, copy, sell, resell or otherwise exploit** this Website and Mobile
>   Application for any commercial purpose without express written consent of TCB;
> - **Not to make any derivative use of this Website and Mobile Application or their contents;**
> - Not to frame or utilise framing techniques to enclose any trademark, logo, or other proprietary
>   information (including images, text, page layout, or form) of TCB and its group companies without
>   express written consent of TCB;
> - Not to use any meta tags or any other "hidden text" utilising the TCB name or trademarks without
>   the express written consent of TCB; and **not reproduce or store any part of this Website and
>   Mobile Application in any other website or include any part of this Website and Mobile Application
>   in any public or private electronic retrieval system or service without prior written permission
>   from TCB**;
> - Not to utilise the material available with an individual subscription account for use by any
>   institution (school, university, or language teaching institution);
> - Not to permit the use of an individual account for other people or third parties;
> - Without the prior consent of TCB, you agree not to display or use in any manner any trademarks or
>   copyright owned by TCB.

> This Website and Mobile Application contain material which is **owned by or licensed to TCB**. This
> material includes, but is not limited to, the lessons we write, the design, layout, look and
> appearance.

> If you breach any of the terms in this legal notice, **your permission to use this Website and Mobile
> Application automatically terminates.** You are also advised that **TCB will at all times enforce its
> intellectual property rights to the fullest extent of the law, including the seeking of criminal
> prosecution.**

> **Any rights not expressly granted in these terms are reserved.**

"Owned by **or licensed to** TCB" implies third-party rights in some lessons as well.

### 6c. LingQ — ALL RIGHTS RESERVED + a CC BY-**ND** carve-out ❌ (ND is disqualifying)

| Field | Finding |
|---|---|
| Content type | User- and staff-uploaded lessons with audio; Chinese among many languages |
| HSK-graded? | **No** (LingQ levels, not HSK) |
| License | **Default: all rights reserved, personal/non-commercial only.** Fallback for unmarked content: **CC BY-ND 3.0** |
| Redistribution verdict | **NOT BUNDLEABLE — NC on the main grant, and ND on the fallback** |
| Owner | **LingQ Languages Ltd.** |
| URL | https://www.lingq.com/en/terms/ |

Verbatim, section "E. Copyright and Content Ownership":

> **Proprietary Material.** The Content and all other information, data, text, graphics, images,
> photographs, audio and video clips, logos, icons and software appearing on the Service ("Content")
> are and will remain our property, our suppliers, our agents and our licensors. … **You may not copy,
> distribute, prepare derivative works from or otherwise use Content for any public or commercial
> purpose without written permission.**

> **All content on LingQ.com is licensed under a Creative Commons Attribution-No Derivative Works 3.0
> Unported License if no Copyright or other License is mentioned anywhere in the content or content
> description.**

> The look and feel of the Service is copyright©2002-2026, LingQ Languages Ltd. All rights reserved.
> You may not duplicate, copy, or reuse any portion of the HTML/CSS or visual design elements without
> express written permission from LingQ.

> **Copyright.** All Site materials, including, without limitation, text, pictures, graphics and other
> files and the selection and arrangement thereof are our copyrighted materials, **ALL RIGHTS
> RESERVED**, or by the original creator of the material. **Permission is granted to display, copy,
> distribute, and download the materials on this Site for personal, noncommercial use only, provided
> you do not modify the materials** and that you retain all copyright and other proprietary notices
> contained in the materials. You may not, however, distribute, copy, reproduce, display, republish,
> download, or transmit any material on this Site for commercial use without prior written approval.
> **You may not "mirror" any material contained on this Site on any other server without prior written
> permission.**

Two **independent** disqualifiers: **NC** ("personal, noncommercial use only") and **ND**
("Attribution-No Derivative Works 3.0"). Note also that bundling CC BY-ND text into an AGPL app creates
a direct licence conflict, since AGPL permits and encourages modification and redistribution.

### 6d. Popup Chinese — DEFUNCT; copyright retained ❌

| Field | Finding |
|---|---|
| Content type | Chinese podcast lessons + transcripts + vocabulary (largely login-gated) |
| HSK-graded? | **No** — own levels (Absolute Beginners / Elementary / …) |
| License | **No open licence. © Language Systems Ltd.** |
| Redistribution verdict | **NOT BUNDLEABLE** |
| Live URL | `popupchinese.com` apex resolves (16.162.112.181) but HTTPS fails; **`http://popupchinese.com/` → 301 → `https://saito.io/popup/`**, a near-empty placeholder that renders only the words "Popup Chinese" |

Evidence:
- `www.popupchinese.com` → **no DNS answer**. `https://popupchinese.com/` → connection failure (exit 000).
- Wayback `/web/2020/http://www.popupchinese.com/` footer: **`© 2013 Language Systems Ltd.`**
  Archived 2011 `/about-us` footer: **`© 2011 Language Systems Ltd.`**
- Wayback CDX shows lessons were **behind `/account/login?redirect=/lessons/…`** — i.e. paywalled.
  The archived About page confirms a paid tier: *"Basic Plus Subscription: In addition to our free
  Chinese podcasts, get full access to all of our downloadable mp3 files, pdf transcripts, character
  writing sheets…"*
- **No terms-of-service, copyright, or licence page exists** in the Wayback index for the domain:
  `url=popupchinese.com/terms*` → **`[]`**; a `terms|copyright|legal|about|faq` filter returns only
  `/about-us` and `/account/faq`, neither with a licence grant.
- ⚠️ **A large archived bundle does exist** and is byte-retrievable:
  `https://web.archive.org/web/20250819143857id_/http://popupchinese.com/absolute-beginners.tar.gz`
  → HTTP 200, `content-type: application/x-gzip`, **`content-length: 1,198,822,886` bytes (≈1.12 GiB)**,
  verified as real gzip (`1f 8b 08 08 …`, inner name `absolute-beginners.tar`).
  **But no licence grant accompanies it** — it is the vendor's commercial content.
  **Technical availability ≠ legal availability. Do NOT bundle.**

### 6e. MandarinSpot — NOT A TEXT CORPUS; no ToS; **nothing to bundle**

| Field | Finding |
|---|---|
| Content type | **Client-side pinyin/Zhuyin annotator + dictionary lookup tool. It hosts no graded reading passages.** |
| HSK-graded? | Only as a *filter* over user-supplied text (HSK 1+ … HSK 9+) |
| License | **No ToS / no copyright page found** |
| Redistribution verdict | **N/A — there is no text corpus to redistribute.** Its upstream *dependencies* are open. |
| URL | https://mandarinspot.com/ — ⚠️ TLS chain incomplete (`curl` needs `-k`; browsers may warn) |

Evidence: `/`, `/about`, and `/terms` return **byte-identical homepage HTML** (`20,114` bytes each) —
it is a single-page app with **no legal pages at all**. No `©`/copyright statement anywhere in the markup.
The nav exposes "Annotation", "Dictionary", "Extension", "API" — i.e. a tool, not a library.

Its acknowledgements name its only data dependencies:

> This site uses the **CC-CEDICT** dictionary maintained and made available by **MDBG Chinese-English
> dictionary**. The version used on this site contains over 100,000 entries.
>
> The HSK vocabulary list used by the annotator was taken from **HSK Flashcards** website.

So MandarinSpot is useful as a **tool reference**, not a content source. If pinyin-annotation logic is
wanted, derive it from **CC-CEDICT (CC BY-SA 4.0)** directly. **Do not scrape MandarinSpot's annotated
output**: it is a derived work of CC-CEDICT, the site grants no rights of its own, and the annotation
formatting/presentation is the site's own expression.

---

## Final recommendation

| Rank | Source | License | Why |
|---|---|---|---|
| **1** | **`no7z/hsk-sentences-audio`** (HuggingFace) | **CC BY-SA 4.0** | The only source found that is **actually HSK-graded (official HSK 3.0 L1–6)**, has **both text and audio**, has 4,354 items, and carries a licence this project explicitly accepts. Share-alike attaches to the dataset/adaptations, not the app. Record the ~1% levelling divergence; disclose synthetic audio. |
| **2** | **Global Storybooks 中文故事集** | **CC BY 4.0 / 3.0** | 40 human-translated stories, 5 length levels, human-recorded audio, downloadable PDFs. Attribution-only — even cleaner than SA. Not HSK-graded; would need local levelling. Pin the exact per-story licence (3.0 vs 4.0 discrepancy). |
| **3** | **Tatoeba** cmn sentences | **CC BY 2.0 FR** | Large, genuinely open, ~1.2 MB export. Sentence-level and **ungraded**; per-clip audio licences vary. |
| — | **CC-CEDICT** | CC BY-SA 4.0 | Glosses/pinyin only, not reading text. |

**Do NOT bundle:** Chinese Reading Project (nonexistent) · Chinese Reading Practice (no licence) ·
Mandarin Companion · Chinese Breeze · Graded Chinese Reader (Sinolingua) · HSK Standard Course ·
HSK Academy / GC Readers · HSKStory · Du Chinese · The Chairman's Bao · LingQ · Popup Chinese ·
MandarinSpot (no corpus).

**Untraceable / flagged:** the `archive.org` Chinese Breeze and HSK Standard Course uploads and the
Wayback `popupchinese.com/absolute-beginners.tar.gz` are **technically downloadable but unlicensed** —
explicitly out of scope for bundling.
