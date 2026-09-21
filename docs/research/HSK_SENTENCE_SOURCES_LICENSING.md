# HSK-graded phrase / sentence / short-text sources and their licensing

**Scope.** Find publicly available, *graded by HSK level* Chinese phrase/sentence/short-text
lists usable for pronunciation and listening drills in an **offline** desktop/mobile app that
**bundles** a data artifact. The app is **AGPL-3.0**.

**Licensing rules used for the verdicts below** (given by the requester):

| Class | Verdict |
|---|---|
| Public domain, CC0, CC BY, MIT, Apache-2.0 | **Bundle** — attribution only |
| CC BY-SA 4.0 / other ShareAlike | **Bundle** — attribution + ShareAlike on the adapted artifact |
| CC BY-NC*, CC BY-ND*, "research only", "academic use only" | **Disqualifying** — do not bundle |
| No LICENSE file / "all rights reserved" | **Disqualifying** — do not bundle without written permission |
| Commercial textbook / exam material / dictionary | **Disqualifying** — copyright risk |

**Date of research:** 2026-09-21. All star counts, dates and byte sizes were read live on that
date. Exact URLs are given for everything so the numbers can be re-checked.

---

## 0. Bottom line

1. **The official HSK word lists contain no example sentences.** The HSK 3.0 standard
   (GF0025-2021, 《国际中文教育中文水平等级标准》) is structured as 音节表 / 汉字表 / 词汇表 /
   附录A 语法等级大纲. The 词汇表 is **word + pinyin only** — no glosses, no sentences. The only
   sentences in the whole 260-page standard are the 例句 attached to grammar points in
   **附录A 语法等级大纲**. The HSK 2.0 《考试大纲》 is likewise a word list plus sample papers.
   *Verified by downloading the official PDF and rendering pages.*

2. **The official PDF is watermarked 仅供查阅 ("for reference/inspection only") on every page and
   carries no open licence.** The MOE publishes it for free reading; that is not a redistribution
   grant. Treat the official standard as *reference for grading*, not as a bundleable text source.

3. **Three GitHub/HuggingFace projects already ship exactly what the app wants, under acceptable
   licences:**
   - **`harukicoder/hsk30-graded-readers`** — 102 short texts / 1,185 sentences with **per-word
     aligned hanzi + toned pinyin + gloss**, under **CC BY 4.0**. Best licence in the report: no
     ShareAlike, attribution only. No audio, and levels must be computed by the app.
   - **`no7z/hsk-sentences-audio`** — 4,354 HSK-graded sentences (levels 1–6), pinyin, English,
     per-word glosses, **8,708 MP3s** (normal + slow, 159.3 MB), dataset under **CC BY-SA 4.0**,
     code MIT. Generated with CosyVoice2 (Apache-2.0). **The only source with clean graded audio.**
   - **`saymei/zhongdex`** — 11,092 HSK 3.0 (2026) headwords and **32,725 graded example
     sentences** with tone-marked pinyin, English and an i+1 `newWordCount` vector, data under
     **CC BY-SA 4.0**, code MIT. **No bundled audio.** Very new and its sentence provenance is
     an opaque private source — see the caveat in §2.3.3.

4. **Tatoeba text is bundleable (CC BY 2.0 FR); Tatoeba Mandarin *audio* is not.** Of 5,825
   Mandarin audio recordings in the official bulk export, **5,741 record an empty licence field
   and the remaining 84 are CC BY-NC 4.0**. Exactly **zero** are CC BY / CC BY-SA / CC0. This is
   a decisive, machine-checked result (§2.5.2). The Tatoeba **CC0** subset contains **exactly one**
   Mandarin sentence, so there is no CC0 escape hatch.

5. **`UD_Chinese-Beginner` is the most tempting source in the report and is disqualified.** It is
   the only openly published corpus explicitly graded for learners (README: *"adapted for learner
   of level A1 to C1 (HSK1 to 5)"*), 2,295 Simplified sentences — and its licence is
   **CC BY-NC-SA 3.0**, whose own text says *"no website or app that generates any revenue at all
   through advertising may legally use"* it. The other six `UD_Chinese-*` treebanks are
   **CC BY-SA 4.0 / 3.0 and are bundleable**, but none of them is HSK-graded.

6. **The Mandarin-audio field is a minefield.** Almost every corpus is CC BY-NC-ND or worse:
   ST-CMDS, Primewords, MAGICDATA SLR68/123, SHALCAS22A, aidatatang_200zh (NC-ND **and
   retracted**). Worse, two well-known "open" ones are **not**:
   - **AISHELL-1 and AISHELL-3 are contradictory** — OpenSLR prints `Apache License v.2.0`, but
     the archived vendor page also says *"This database is free for academic research, not in the
     commerce, if without permission"* and the 2017 snapshot says *"Commercial use is
     forbidden."* **Treat as NC.** This also taints `vits-icefall-zh-aishell3`, which is trained on
     AISHELL-3.
   - **WenetSpeech** prints CC BY 4.0 on OpenSLR but its own site says *"available to download for
     **non-commercial** purposes"* and concedes it *"doesn't own the copyright of the audios"*.
   - **Mozilla Common Voice is CC0 in law but contractually un-bundleable**: the MDC datasheet says
     *"It is forbidden to **re-host or re-share** this dataset"* and the ToS forbids
     *"scrape, mirror, or redistribute the Dataset"* — MDC's FAQ calls no-mirroring *"a platform
     term"* layered on the CC0 grant.

7. **Only two clean audio-ish options exist, and neither is HSK-graded:**
   - **CSS10 Chinese — CC0**, 6:27:04, 2.04 GB, single speaker, **aligned audio + text** (LibriVox
     readings of 魯迅 by Jing Li). Legally perfect; pedagogically advanced literary Chinese.
   - **AISHELL-4 / AliMeeting — CC BY-SA 4.0**, but meeting speech, useless for learner drills.

8. **Recommended architecture: bundle the two sentence artifacts in §3 Rank 1 (one CC BY 4.0, one
   CC BY-SA 4.0, kept as separate files) and synthesise any additional audio at build time with
   MeloTTS (MIT)** — not with corpora audio, and not with `vits-icefall-zh-aishell3` (AISHELL-3
   taint). See §3.

---

## 1. Master table

Legend — **HSK-graded?** = does the artifact itself carry HSK level tags. **Bundle?** =
redistributable inside the bundled app under the requester's rules.

### 1a. Official HSK sources

| Source | Content | HSK-graded? | Audio? | Licence | Bundle? | URL |
|---|---|---|---|---|---|---|
| GF0025-2021 《国际中文教育中文水平等级标准》 (MOE PDF, 260 pp.) | 音节表, 汉字表, **词汇表 (word+pinyin only)**, 附录A 语法等级大纲 (**grammar points + example sentences**) | Yes (it *is* the grading) | No | No open licence; every page watermarked 仅供查阅 | **No** | [moe.gov.cn PDF](http://www.moe.gov.cn/jyb_xwfb/gzdt_gzdt/s5987/202103/W020210329527301787356.pdf) · [announcement](http://www.moe.gov.cn/jyb_xwfb/gzdt_gzdt/s5987/202103/t20210329_523304.html) |
| 官方查询系统 chinesetest.cn | word lists + grammar points with 例句 | Yes | No | Site ToS; no redistribution grant | **No** | [chinesetest.cn standardsAction](https://www.chinesetest.cn/standardsAction.do?means=getStandardWordsList) |
| HSK 2.0 《新汉语水平考试大纲》(商务印书馆, 2009) | 词汇大纲 5,000 words; 样卷 + 听力材料 | Word list only | CD with sample papers | Commercial copyright | **No** | ISBN 978-7-100-06927-4 |
| HSK 2.0/2015 《HSK 考试大纲》 (人民教育出版社, 2015-09) | 话题/任务/语言点/词汇大纲 + 样卷 + 听力材料 | Word list only | Sample-paper audio | Commercial copyright | **No** | [pep.com.cn](https://www.pep.com.cn/products/jc/dwhytsh/201904/t20190416_1937364.shtml) · ISBN 9787107304200 |

### 1b. Community HSK word lists (no sentences, but usable to *grade* text)

| Source | Content | HSK-graded? | Audio? | Licence | Bundle? | URL |
|---|---|---|---|---|---|---|
| `drkameleon/complete-hsk-vocabulary` | 11,470 words, HSK 2.0 + 3.0 levels, CC-CEDICT glosses | Yes (levels) | No | **MIT** (code/compilation); glosses CC-CEDICT **CC BY-SA 4.0** | **Yes** — *already bundled* | [repo](https://github.com/drkameleon/complete-hsk-vocabulary) |
| `ivankra/hsk30` | 11,092 rows: ID, simp/trad, pinyin, POS, level, WebNo, CEDICT key; + grammar + chars | Yes | No | **MIT** (© Ivan Krasilnikov, Shawky, Pleco Inc.) | **Yes** | [repo](https://github.com/ivankra/hsk30) |
| `elkmovie/hsk30` | `wordlist.txt` / `charlist.txt`, OCR of the official PDF | Yes | No | **MIT** (© 2021 Pleco Inc.) | **Yes** *(but see caveat §2.2.2)* | [repo](https://github.com/elkmovie/hsk30) |
| `krmanik/HSK-3.0` | HSK 1–9 words, hanzi, handwritten, grammar; 370★, active 2026-06-14 | Yes | No | `License.md` attributes components (CC-CEDICT CC BY-SA 4.0, Pleco MIT, SUBTLEX-CH, "Mani" CC BY-SA 4.0); no single repo licence | **Unclear — see §2.3.4** | [repo](https://github.com/krmanik/HSK-3.0) |
| `chelsea6502/hsk-2025-data` | `vocabulary.tsv` (~11,000), `grammar.tsv` (~593 grammar points **with 例句**), hanzi TSVs — scraped from chinesetest.cn | Yes | No | **No LICENSE file** | **No** | [repo](https://github.com/chelsea6502/hsk-2025-data) |
| `glxxyz/hskhsk.com` | word lists + `HSK Examples.txt`, `HSK 3 Example Sentences.txt`, `HSK 3 Questions.txt` | Partly | No | **MIT** on the repo — but the sentence/question files look like official exam material | **Risky — §2.3.6** | [repo](https://github.com/glxxyz/hskhsk.com) |
| `clem109/hsk-vocabulary` | 6 files of words (150 for HSK1); README *promises* example sentences | Yes | No | **MIT** | **Yes**, but **contains no sentences** (verified: 0/150 records have `examples`) | [repo](https://github.com/clem109/hsk-vocabulary) |
| `Punpuf/hsk-syllabus-vocabulary-parser` | Extracts the HSK 3.0 (2026) list from the syllabus PDF → TSV | Yes | No | MIT (per GitHub search) | Yes (word list) | [repo](https://github.com/Punpuf/hsk-syllabus-vocabulary-parser) |
| `harukicoder/hsk30` | Grades arbitrary text against GF0025-2021 or the 2026 exam | tool | No | MIT | Yes (tool) | [repo](https://github.com/harukicoder/hsk30) |

### 1c. HSK-graded **sentence** sources

| Source | Content | HSK-graded? | Audio? | Licence | Bundle? | URL |
|---|---|---|---|---|---|---|
| **`harukicoder/hsk30-graded-readers`** ⭐ | **102 short texts / 1,185 sentences / 16,956 chars**, six shelves (newbie 22 · beginner 22 · intermediate 22 · upper 12 · advanced 12 · native 12) + a disjoint 30-text held-out split; **per-word aligned `hz`/`py`/`en`** + per-sentence English | Level must be computed (deliberately unlabelled) | No | **CC BY 4.0** (corpus); MIT for the `hsk30` code/level tables | **Yes — best licence, no ShareAlike** | [HF](https://huggingface.co/datasets/harukicoder/hsk30-graded-readers) · [code](https://github.com/harukicoder/hsk30) |
| **`no7z/hsk-sentences-audio`** | **4,354 sentences** L1–6, simp+trad, toned + numbered pinyin, EN, per-word gloss, grammar tags, topic/sentence-type; **8,708 MP3** (normal+slow, 159.3 MB); 6.4 MB JSON | **Yes** (with a 5.2% caveat — §2.3.1) | **Yes (TTS, CosyVoice2)** | **Dataset CC BY-SA 4.0**, code **MIT**; audio Apache-2.0-derived synthetic | **Yes** | [repo](https://github.com/no7z/hsk-sentences-audio) · [HF](https://huggingface.co/datasets/no7z/hsk-sentences-audio) |
| **`saymei/zhongdex`** | **32,725 graded sentences** over 99.82% of 11,092 headwords; toned + numbered pinyin, EN, `newWordCount` i+1 vector, `zsg` grade; 11,092-word canon with 3 syllabus columns | **Yes (computed ZSG grade 1–7)** | **No** (availability flags only; explicitly not bundled/hosted) | **Data CC BY-SA 4.0**, code **MIT** | **Yes, with a provenance caveat — §2.3.3** | [repo](https://github.com/saymei/zhongdex) |
| `SHLEW06/chinese-hsk-adaptive-reader` | **300 readings, 50 per HSK level 1–6** (~13.6 MB JSON; hsk1.json 1,797,566 B … hsk6.json 2,923,622 B), translations, sentence explanations, grammar, target words, comprehension questions | **Yes (per-level files)** | No | Root **MIT** (© 2026 Shunji Lewandowski); `THIRD_PARTY_NOTICES` lists only CC-CEDICT + complete-hsk-vocabulary, so the readings appear original — **but the text itself is not explicitly licensed** | **Ask the author first** | [repo](https://github.com/SHLEW06/chinese-hsk-adaptive-reader) |
| Global Storybooks 中文故事集 | **40 stories, 5 levels**, + **human audio** and PDFs | Claimed levels | **Yes (human)** | Footer says **CC BY 4.0**; individual story pages carry a **CC BY 3.0** badge; repo `LICENSE` is MIT = site code only | Yes, but **pin per-story licence at ingest** | [site](https://global-asp.github.io/storybooks-chinese/) |
| `Roxaleen/hsk-annotated-corpus` | ~260,000 sentences graded 1–7 + English; 11k words; Tatoeba + Wiktionary + Leipzig sources; `sentences.csv` 49 MB | Yes | No | **No LICENSE file** | **No** — although the upstream Tatoeba/Wiktionary subsets would be reusable if re-derived | [repo](https://github.com/Roxaleen/hsk-annotated-corpus) |
| `krmanik/hsk-graded-sentences` | Tatoeba `cmn` corpus cleaned, profanity-filtered, jieba-segmented, difficulty-scored, `max_hsk` 1–7; SQLite ~11 MB | Yes (computed) | No | **No LICENSE file**; upstream Tatoeba text is CC BY 2.0 FR | **No as-is** — re-derive from Tatoeba | [repo](https://github.com/krmanik/hsk-graded-sentences) |
| `bdx33/tatoeba-hsk-cmn-eng-fra` (HF) | Tatoeba sentences in simplified Chinese with an `hsk_level` column, EN + FR; **78,504 rows** | **Yes (column)** | No | HF card `license: cc-by-2.0` (i.e. Tatoeba's) | **Probably yes** — but the **grading method is undocumented**; prefer re-deriving | [HF](https://huggingface.co/datasets/bdx33/tatoeba-hsk-cmn-eng-fra) |
| `lm742611149/learn-chinese` (readmandarin.com) | **315 original HSK 1–6 readings** + 5,024 words + 659 grammar patterns + per-sentence audio (edge-tts) | **Yes** | Yes (edge-tts, unlicensed output) | **No LICENSE file** | **No as-is** — original content, so worth asking the author for a licence | [repo](https://github.com/lm742611149/learn-chinese) |
| `mreichhoff/HanziGraph` | Example sentences: Tatoeba (human) + AI-generated, sorted by word frequency | Indirectly (HSK colour coding) | No (browser TTS) | Repo **MIT**, but sentences remain Tatoeba **CC BY 2.0 FR**/AI | Partly — Tatoeba subset is bundleable with attribution | [repo](https://github.com/mreichhoff/HanziGraph) |
| `NewHSK3/new-hsk-3-anki-deck` | "33,000 example sentences + Mandarin audio", sentence-based Anki deck | Claims HSK 3.0 | Claims audio | **No LICENSE file** | **No** | [repo](https://github.com/NewHSK3/new-hsk-3-anki-deck) |
| `TeaPearce/chinese-english-dictionary` | 120k entries with HSK markers + example sentences (Kindle) | Yes (markers) | No | `NOASSERTION` (unrecognised licence file) | **No — verify** | [repo](https://github.com/TeaPearce/chinese-english-dictionary) |
| `Pleometric/HSK-deck` | Anki decks with AI-generated example sentences | Yes | No | **No LICENSE file** | **No** | [repo](https://github.com/Pleometric/HSK-deck) |
| `metalhatscats/bonihua-datasets` | Russian-facing HSK/pinyin/pronunciation datasets | Yes | Some | **CC BY-NC-SA 4.0** | **No — NC** | [repo](https://github.com/metalhatscats/bonihua-datasets) |

### 1d. Frequency-graded corpora

*(see §2.4 for the quoted clauses)*

| Source | Content | HSK-graded? | Audio? | Licence | Bundle? | URL |
|---|---|---|---|---|---|---|
| **`Thoria/mandarin-most-common-words-tr-en`** | 1,143 words + pinyin + Zipf + **example sentences** (zh/en/tr), CSV 1,573,856 B | No (common words) | No | **CC BY 4.0** data + MIT code | **Yes** — the only licensed file found with word+pinyin+freq+**sentence** | [HF](https://huggingface.co/datasets/Thoria/mandarin-most-common-words-tr-en) |
| `wordfreq` (rspeer) Chinese data | Bundles **SUBTLEX-CH** zh wordlist + OpenSubtitles/Wikipedia; `large_zh` = **334,609 words**, 1,773,909 B; `small_zh` = 38,590 | No (frequency) | No | Code **Apache-2.0**; data files **CC BY-SA 4.0**; SUBTLEX-CH redistributed **with the author's e-mail permission** | **Yes** (attribution) | [repo](https://github.com/rspeer/wordfreq) |
| `wzperson/hearmandarin-hsk-3-0-word-list` | 11,147 HSK 3.0 words + gloss + POS | Yes | No | **CC BY 4.0** | **Yes** | [repo](https://github.com/wzperson/hearmandarin-hsk-3-0-word-list) |
| `thunlp/THUOCL` | 11 domain document-frequency lists | No | No | **MIT** | **Yes** | [repo](https://github.com/thunlp/THUOCL) |
| Leipzig Corpora **downloads** | 10k–1M sentences + ranked word types per corpus | No | No | **CC BY** for the downloadable text corpora (the portal/API data is CC BY-NC) | **Yes**, web-scrape caveat | [usage](https://wortschatz.uni-leipzig.de/en/usage) |
| Wiktionary (incl. example sentences) | Dictionary entries with usage examples | No | No | **CC BY-SA 4.0 + GFDL** (dual) | **Yes** with ShareAlike | [copyrights](https://en.wiktionary.org/wiki/Wiktionary:Copyrights) |
| SUBTLEX-CH raw zip (UGent) | Word + character frequencies from film subtitles | No | No | **No licence on the data files**; paper says "freely available for research purposes"; authors got permission to *download* the subtitles | **No** — use `wordfreq` | [ugent.be](https://www.ugent.be/pp/experimentele-psychologie/en/research/documents/subtlexch/overview.htm) |
| BCC 语料库 (BLCU) | 12 char/word freq datasets; ~62亿字 | No | No | **No licence**; only *"均可免费下载使用…请规范引用 BCC 论文"* | **No** — e-mail BLCU for written permission | [bcc.blcu.edu.cn](https://bcc.blcu.edu.cn/help.html) |
| Lancaster Corpus of Mandarin Chinese (LCMC) | 1M-word written Mandarin corpus | No | No | EULA: distribution restricted to Licensee/research group; **no commercial-product rights** | **No** | [licence](https://www.lancaster.ac.uk/fass/projects/corpus/LCMC/lcmc/lcmc_license.htm) |
| Chinese Gigaword (LDC2003T09 / LDC2011T13) | Newswire Chinese | No | No | LDC User Agreement, paid + signed | **No** | [LDC](https://catalog.ldc.upenn.edu/LDC2011T13) |
| Chinese Word Sketch / Sketch Engine | sketches / wordlists | No | No | Application-gated; *"Licensee may not sublicense or distribute the Works"* | **No** | [sketchengine](https://www.sketchengine.eu/) |
| OpenSubtitles2018 / OPUS zh | Subtitle sentences | No | No | **No licence granted**; upstream ToS *"Commercial use prohibited"* | **No** | [OPUS](https://opus.nlpl.eu/legacy/OpenSubtitles.php) |
| Jun Da frequency lists | char freq + bigrams | No | No | **No licence** — *"Copyright. 1998-2026. Jun Da."* | **No** | [lingua.mtsu.edu](https://lingua.mtsu.edu/chinese-computing/statistics/) |
| Kelly / Leeds Chinese | Frequency list | No | No | **CC BY-ND-NC-SA 2.0** | **No** | [Wiktionary](https://en.wiktionary.org/wiki/Wiktionary:Frequency_lists) |
| `ymcui/cmrc2019` | 9,638 passages / 100,009 sentence-cloze queries from **Chinese narrative stories** | No | No | **CC BY-SA 4.0** | **Yes** — good raw material for high levels | [repo](https://github.com/ymcui/cmrc2019) |

### 1e. Audio + transcript

*(see §2.5 for the quoted clauses)*

| Source | Content | HSK-graded? | Licence | Bundle? | URL |
|---|---|---|---|---|---|
| **CSS10 Chinese** ⭐ | **6:27:04** / 2,040,891,168 B, single speaker (Jing Li), **aligned audio + text** — LibriVox readings of 魯迅 朝花夕拾 + 呐喊 | No | **CC0 1.0** | **Yes — the best clean audio+text bundle found** | [kaggle](https://www.kaggle.com/datasets/bryanpark/chinese-single-speaker-speech-dataset) |
| LibriVox Chinese | **27 items / ≈125 h** | No | **Public domain** ("anyone can use all our recordings however they wish (even to sell them)") | **Yes** — but **audio only, no transcripts** | [librivox](https://librivox.org/pages/public-domain/) |
| Mozilla Common Voice (zh-CN/zh-TW/zh-HK) | zh-CN 852,410 clips / 240.11 validated h; zh-TW 79.92 h; zh-HK 108.56 h; TSVs ship with the audio | No | **CC0-1.0** — but MDC datasheet: *"It is forbidden to **re-host or re-share** this dataset"*; ToS: *"scrape, mirror, or redistribute the Dataset"* | **No — CC0 in law, contractually blocked** | [MDC](https://mozilladatacollective.com/) |
| Tatoeba Mandarin audio | 5,825 recordings | No | 5,741–5,742 **blank** (= "No license for offsite use"); 84 **CC BY-NC 4.0**; 0 permissive | **No** | [downloads](https://downloads.tatoeba.org/exports/) |
| AISHELL-4 (SLR111) / AliMeeting (SLR119) / AISHELL-5 (SLR159) / CN-Celeb | Meeting / in-car speech | No | **CC BY-SA 4.0** | Yes, but unusable pedagogically | [openslr 111](https://www.openslr.org/111/) |
| AISHELL-1 (SLR33) / AISHELL-3 (SLR93) | 178 h / 85 h read speech | No | OpenSLR prints **Apache-2.0**; archived vendor page also carries *"free for academic research, not in the commerce, if without permission"* | **Unclear — treat as NC** | [openslr 33](https://www.openslr.org/33/) |
| THCHS-30 (SLR18) | ~30 h | No | **Apache-2.0** + *"totally free to academic users"*, but no NC clause | Unclear, leaning yes | [openslr 18](https://www.openslr.org/18/) |
| WenetSpeech (SLR121) | 10,000+ h YouTube/Podcast | No | OpenSLR says **CC BY 4.0**; own site: *"available to download for **non-commercial** purposes"* and *"doesn't own the copyright of the audios"* | **No** | [openslr 121](https://www.openslr.org/121/) |
| ST-CMDS (SLR38) | 102,600 utts | No | **CC BY-NC-ND 4.0** | **No** | [openslr 38](https://www.openslr.org/38/) |
| Primewords Set 1 (SLR47) | 100 h | No | **CC BY-NC-ND 4.0** (not CC BY-SA) | **No** | [openslr 47](https://www.openslr.org/47/) |
| MAGICDATA Read Speech (SLR68) + Conversational (SLR123) | 755 h + 180 h | No | **CC BY-NC-ND 4.0** | **No** | [openslr 68](https://www.openslr.org/68/) |
| aidatatang_200zh (SLR62) | 200.38 h, 237,265 utts | No | **CC BY-NC-ND 4.0**, and **retracted at the data owner's request** | **No** | [openslr 62](https://openslr.org/62/) (404) |
| KeSpeech | Large multi-dialect | No | **NC + ND + no-distribution**: *"No Distribution. You may not distribute this dataset to any third parties"* | **No** | [repo](https://github.com/tzyll/KeSpeech) |
| GigaSpeech 1 / 2 | — | — | **NC**, and **neither contains Mandarin** (v1 English; v2 Thai/Indonesian/Vietnamese) | **No** | [repo](https://github.com/SpeechColab/GigaSpeech) |
| Emilia (original) / WenetSpeech4TTS | 101k h | No | **Non-commercial** | **No** | — |
| Emilia-YODAS | Gated | No | **CC BY 4.0** per gating text | Yes (gated), standalone doc unverified | [HF](https://huggingface.co/datasets/amphion/Emilia-YODAS) |
| HKUST/MTS (LDC2005S15) | ~149 h | No | LDC agreement: *"only for noncommercial linguistic education, research and technology development"* | **No** | [LDC](https://catalog.ldc.upenn.edu/LDC2005S15) |
| HI-MIA (SLR85) / MobvoiHotwords (SLR87) | Wake-word/speaker | No | **Apache-2.0** | Yes, but wrong content | [openslr 85](https://www.openslr.org/85/) |
| OMPAL corpus | 1,850 utterances (82 native) | No | **CC BY 4.0** | Yes, but tiny, mostly accented | [repo](https://github.com/phantomhsieh/OMPAL-corpus) |
| zhvoice / DiDiSpeech | ~900 h / ~800 h | No | **No licence found at any reachable URL** | **No** | — |
| Local TTS synthesis | Any text you own | n/a | Model-dependent: **MeloTTS MIT**; icefall AISHELL-3 **inherits the AISHELL-3 question**; Matcha-zh **NC → no** | **Yes — recommended** | see `docs/research/ASR_TTS_CLAUDE_RESEARCH.md` |

### 1f. Graded readers / public-domain text

*(see §2.6)*

| Source | Content | HSK-graded? | Licence | Bundle? | URL |
|---|---|---|---|---|---|
| **UD_Chinese-GSDSimp** | **4,997 Simplified sentences**, 123,289 tokens, POS/dependency-annotated | No (grade it yourself) | **CC BY-SA 4.0** | **Yes** | [repo](https://github.com/UniversalDependencies/UD_Chinese-GSDSimp) |
| **Tatoeba `cmn`** | **89,065 Mandarin sentences** (median 10 chars) | No (grade it yourself) | **CC BY 2.0 FR** (CC0 subset = 1 sentence) | **Yes** | [downloads](https://downloads.tatoeba.org/exports/per_language/cmn/) |
| UD_Chinese-GSD | 4,997 Traditional sentences, 123,289 tokens | No | CC BY-SA 4.0 | Yes | [repo](https://github.com/UniversalDependencies/UD_Chinese-GSD) |
| UD_Chinese-CFL | 451 Simplified learner-essay sentences (contains errors) | No | CC BY-SA 4.0 | Yes, with caveats | [repo](https://github.com/UniversalDependencies/UD_Chinese-CFL) |
| UD_Chinese-HK | 1,004 Traditional sentences (film subtitles + LegCo) | No | CC BY-SA 4.0 | Yes, provenance unclear | [repo](https://github.com/UniversalDependencies/UD_Chinese-HK) |
| UD_Chinese-PUD | 1,000 Traditional sentences (translated news/wiki) | No | CC BY-SA 3.0 | Yes | [repo](https://github.com/UniversalDependencies/UD_Chinese-PUD) |
| **UD_Chinese-Beginner** | 2,295 Simplified sentences, **explicitly graded HSK 1–5** | **Yes** | **CC BY-NC-SA 3.0** | **No — NC** | [repo](https://github.com/UniversalDependencies/UD_Chinese-Beginner) |
| UD_Chinese-PatentChar | 200 Simplified patent sentences | No | **conflicting: BY-SA 4.0 in LICENSE.txt vs NC-SA 3.0 in metadata** | **No** — treat as NC | [repo](https://github.com/UniversalDependencies/UD_Chinese-PatentChar) |
| Chinese Text Project (ctext.org) | Classical Chinese full texts | No | All rights reserved; scraping forbidden; subscriber API, no public dump | **No** | [ctext.org](https://ctext.org/) |
| CHILDES / TalkBank "Read Chinese!" | Classroom Mandarin data | No | **CC BY-NC-SA 3.0** | **No — NC** | [talkbank rules](https://talkbank.org/0share//rules.html) |
| Chinese Wikisource | Classical + modern texts; 4,031,485 content pages, 7.89 GB dump | No | Wiki text **CC BY-SA 4.0** + GFDL; underlying works PD | **Yes** (attribution + SA) | [zh.wikisource.org](https://zh.wikisource.org/) |
| Project Gutenberg Chinese | **443 items, 440 not US-copyright-restricted** | No | PG Licence; strip PG references → PD in US | **Yes** (strip PG boilerplate) | [gutenberg.org](https://www.gutenberg.org/browse/languages/zh) |
| Global Storybooks 中文故事集 | **40 stories, 5 levels**, + **human audio** + PDFs | Claimed levels | Footer **CC BY 4.0** vs story badges **CC BY 3.0**; repo `LICENSE` MIT = site code only | Yes — pin per-story licence | [site](https://global-asp.github.io/storybooks-chinese/) |
| `SHLEW06/chinese-hsk-adaptive-reader` | **300 readings, 50 per HSK 1–6**, ~13.6 MB JSON, translations + grammar + quizzes | **Yes (per-level files)** | Repo **MIT**; text **not explicitly licensed** | **Ask the author** | [repo](https://github.com/SHLEW06/chinese-hsk-adaptive-reader) |
| `daligao/chinese-reading-lab` | HSK 4–6, 10 stories, ~3,904 CJK chars | Claims HSK-graded | **CC0 stated in README only** — no LICENSE file | Probably yes; verify | [repo](https://github.com/daligao/chinese-reading-lab) |
| `daligao/mandarin-flashcards` | HSK 1–3 vocabulary | Yes | **CC0 stated in README only** — no LICENSE file | Probably yes; verify | [repo](https://github.com/daligao/mandarin-flashcards) |
| Mandarin Companion / Chinese Breeze / Du Chinese / The Chairman's Bao / LingQ | Commercial graded readers | Yes | All rights reserved | **No** | — |

---

## 2. Per-source detail

### 2.1 Official HSK vocabulary lists

#### 2.1.1 HSK 3.0 — 《国际中文教育中文水平等级标准》(GF0025-2021)

**Where the canonical list lives.** Two places:

- The standard's own PDF, published by 教育部 + 国家语言文字工作委员会, downloadable from the
  MOE announcement page: <http://www.moe.gov.cn/jyb_xwfb/gzdt_gzdt/s5987/202103/W020210329527301787356.pdf>
  (51,104,405 bytes, 260 pages, created 2021-03-26, in force since 2021-07-01).
- The official query service at <https://www.chinesetest.cn/standardsAction.do?means=getStandardWordsList>,
  which is where `ivankra/hsk30`'s `WebNo`/`WebPinyin` columns and `chelsea6502/hsk-2025-data` come from.

**Does it include example sentences?** **The 词汇表 does not.** I downloaded the PDF and rendered
pages. The 目次 (p. 1) is:

```
前言 … III
1 范围 … 1
2 术语和定义 … 1
3 等级描述 … 2
4 音节表 … 9
5 汉字表 … 15
6 词汇表 … 36
附录A（规范性） 语法等级大纲 … 170
```

A rendered page from 词汇表 (p. 114 of the PDF) shows exactly two columns per band:

```
979  多年来      duō nián lái
980  多心        duō xīn
981  多余        duō yú
982  多元        duō yuán
983  哆嗦        duō suo
```

— headword + pinyin, nothing else. The **only** example sentences in the whole standard are the
例句 under each grammar point in 附录A 语法等级大纲 (pp. 170+). A rendered page from that appendix
(p. 209 of the PDF) shows:

```
【四 29】一 + 量词 + 比 + 一 + 量词
    这些球鞋一双比一双好看。
    他的演出一次比一次精彩。
    天气一天比一天暖和。

【四 30】（自）……以来
    自去年以来，我一直生活在北京。
    上大学以来，他一直坚持学习中文。
```

So: ~413 grammar points (per `no7z`, who machine-extracted them) to ~593 (per `chelsea6502`,
who scraped the 2026 revision), each with 1–4 sentences. That is a few hundred to ~2,000 short
sentences, not a graded phrase corpus.

**Licence status — decisive observation.** The PDF is 51 MB of outlined/vector content, so
`pdftotext` returns only **317 bytes for the entire document** — the repeating watermark:

```
仅
阅
查
供        ← i.e. 仅供查阅, "for reference/inspection only"
```

The watermark is baked into every rendered page. The MOE page's footer carries
「版权所有：中华人民共和国教育部」, but that is the website notice. There is **no open licence,
no CC statement, and no terms-of-use document attached to the standard**. The standard itself is
described as 国家语委语言文字规范 (a language-standard document of the state language commission).

**Verdict: do not bundle the standard's text.** Two independent reasons: (a) no redistribution
grant exists, and the document is explicitly marked "for reference only"; (b) in the PRC, the
copyright status of a *recommended* (non-mandatory) national-standard document as opposed to a
statute is not settled — Art. 5 of the PRC Copyright Law exempts laws, regulations and documents
of a legislative/administrative/judicial nature, which does not obviously cover a 语言文字规范.
A bare word+pinyin list is thin factual data and the practical risk is low, which is why the
community redistributions are tolerated — but the app's `LICENSES.md` should not claim the
official PDF is openly licensed.

**Safe use of the official standard:** as the *grading authority* only — i.e. download it at build
time (or read the community word lists) to *assign* HSK levels to text the app owns. The app
already does exactly this via `complete-hsk-vocabulary`.

#### 2.1.2 HSK 2.0

Two printed syllabi exist:

- 《新汉语水平考试大纲 HSK 1–6 级》, 国家汉办/孔子学院总部 编制, 商务印书馆, 2009
  (e.g. HSK 六级, ISBN 978-7-100-06927-4, <https://www.cp.com.cn/book/978-7-100-06927-4_61.html>).
- 《HSK 考试大纲》 (1–6 级, 6 volumes), 人民教育出版社, 2015-09 (HSK 三级 ISBN 9787107304200,
  <https://www.pep.com.cn/products/jc/dwhytsh/201904/t20190416_1937364.shtml>).

The 人教 page lists the contents of the HSK 三级 volume verbatim:

> HSK（三级）话题大纲 / HSK（三级）任务大纲 / HSK（三级）语言点大纲 / **HSK（三级）词汇大纲** /
> HSK（三级）考试要求及过程 / **HSK（三级）样卷** / HSK（三级）答题卡 / **HSK（三级）样卷听力材料** /
> HSK（三级）样卷答案 / HSK（三级）成绩报告

So HSK 2.0 *does* include listening material — but it is **sample-paper audio belonging to the
exam**, not a graded drill corpus, and it is commercially published. **Not bundleable.**

The 5,000-word HSK 2.0 list itself circulates widely (e.g. `gigacool/hanyu-shuiping-kaoshi`,
MIT — <https://github.com/gigacool/hanyu-shuiping-kaoshi>, © 2018 gigacool) and is what
`complete-hsk-vocabulary` and therefore the app already consume. Word list only, no sentences.

#### 2.1.3 The 2026 revision ("HSK 3.0 2026")

Several 2026-dated projects distinguish "the GF0025-2021 grading standard" from "the 2026
examination" (`harukicoder/hsk30`, `saymei/zhongdex`, `Punpuf/hsk-syllabus-vocabulary-parser`).
Band sizes reported for 2026 are 500 / 772 / 973 / 1,000 / 1,071 / 1,140 / 5,636 = **11,092**
— the same totals as the 2021 standard. **I could not locate an MOE-published PDF of a 2026
syllabus revision**; the 2026 numbers I saw are all second-hand from community repos. Treat
"2026 word list" claims as unverified against a primary source.

---

### 2.2 Textbook-derived and commercial lists

#### 2.2.1 HSK Standard Course / HSK标准教程

- `joelypoley/hsk_standard_course_vocab` — <https://github.com/joelypoley/hsk_standard_course_vocab>
  — 2★, last push **2021-05-09**, **no LICENSE file**. README: *"accurate digitized versions of
  the word lists at the back of the HSK standard coursebooks"*, built by *"tak[ing] a picture of
  each page in the back of the book … OCR"*. Word lists only (HSK 1–5 done, 6 incomplete).
  **Copyright risk: high.** Photographing and redistributing the vocabulary tables of a
  commercially published textbook is reproduction of the publisher's (北京语言大学出版社) material.
  **Do not bundle.**

#### 2.2.2 `elkmovie/hsk30` — an important nuance

`elkmovie/hsk30` is MIT-licensed with *"Copyright (c) 2021 Pleco Inc."*, and its `wordlist.txt`
header reads:

```
# HSK 3.0 word list
# OCR'ed but not extensively proofread (yet)
# Copyright (c) 2021 Pleco Inc.
# Distributed under MIT license, see https://github.com/elkmovie/hsk30/ for details
```

The MIT grant is over **Pleco's OCR work**, not over the underlying official word list. This is
the standard pattern in this ecosystem (`krmanik/HSK-3.0` does the same). It is defensible for a
bare word list; it would **not** be defensible for textbook pages or exam transcripts. The app
already relies on this chain (via `complete-hsk-vocabulary`) and documents it in `LICENSES.md`.

#### 2.2.3 HSK直通车, Boya Chinese, New Practical Chinese Reader

I found **no** repository offering an openly licensed word/phrase list from 直通车 (Sinolingua),
《博雅汉语》 (Peking University Press) or *New Practical Chinese Reader* (BLCU Press). The only
copies discoverable are full-text scans on the Internet Archive
(e.g. `archive.org/stream/NewPracticalChineseReaderTextBook1`,
`archive.org/download/HSK1StandardCourse/…`, `archive.org/stream/hsk-4-standard-textbook/…`).
Those are **complete copyrighted textbooks uploaded without a licence**.
**All rights reserved; do not touch. High copyright risk.**

#### 2.2.4 AnkiWeb shared decks

Anki's own FAQ (<https://faqs.ankiweb.net/can-i-use-anki-in-a-company-or-school.html>) says:

> The cards you create with Anki are your own, so you are free to license them as you please…
> If you find your copyrighted content has been uploaded on AnkiWeb's list of shared decks,
> please let us know and we will remove it as soon as possible.

There is **no blanket licence on shared decks**. Each deck belongs to its uploader, most carry no
licence statement at all (= all rights reserved), and a large share of HSK decks embed textbook,
official-syllabus or commercial-dictionary content. AnkiWeb explicitly states it will take decks
down on copyright complaint. **Do not bundle AnkiWeb decks.** (`krmanik/Anki-xiehanzi`, 414★, is
a tool, not a data grant.)

#### 2.2.5 Memrise

Memrise Terms of Use, clause 5–6 (<https://www.memrise.com/terms>), verbatim:

> **5. OUR CONTENT** … all of the content available through the Services … are owned by us or are
> licensed to us by a third party ("Our Content"). You acknowledge and accept that you are
> expressly prohibited from using Our Content except where we grant you a limited license …
>
> **6. YOUR LICENCE** … we grant you a limited, personal, non-transferable, non-sublicensable,
> worldwide and non-exclusive licence to use Our Content for the exclusive purpose of using the
> Services for **your own personal, non-commercial use** … you do not … copy, modify, create a
> derivative work from …

**Disqualified.** Clause 8 also takes a broad sub-licensable licence from *users* to Memrise,
which does not help a third party.

#### 2.2.6 Pleco

Pleco's legal notices (<https://iphone.pleco.com/manual/229/copyright.html>) are unambiguous:

> Software, audio data files and other data Copyright © 2001-2013 Pleco Software Incorporated.

Its bundled dictionaries are all third-party and reserved ("ABC Chinese-English Comprehensive
Dictionary Copyright © 2003-2009 University of Hawai'i Press. **All rights reserved.**", etc.).
Pleco's *own* HSK flashcard lists are released MIT (see its forum thread linked from
`krmanik/HSK-3.0/License.md`), and Pleco authored the OCR in `elkmovie/hsk30` and `ivankra/hsk30`
— **those two specific contributions are the bundleable part.** Pleco's audio and dictionary
content are **not**.

**Bottom line for §2.2: every textbook-derived and platform-hosted source is disqualified. The
only usable artefacts are the bare HSK word lists, which the app already has.**

---

### 2.3 GitHub repositories with HSK-graded sentences

Search method: GitHub repository search (`topic:hsk` sorted by stars, plus keyword queries on
`hsk sentences`, `hsk example sentences`, `hsk30`, `chinese graded reader data`, and licence
filters `license:cc-by-sa-4.0`, `license:cc-by-4.0`, `license:mit`). Metadata re-verified against
`ungh.cc` and `img.shields.io` after the GitHub API rate limit was hit; licence text and data
files fetched from `raw.githubusercontent.com`, `data.jsdelivr.com` and `huggingface.co`.

#### 2.3.0 ⭐⭐ `harukicoder/hsk30-graded-readers` — the best *licence* in the whole report

| | |
|---|---|
| URL | <https://huggingface.co/datasets/harukicoder/hsk30-graded-readers> · code <https://github.com/harukicoder/hsk30> |
| Dates | created **2026-09-01T17:17:56Z**, last modified **2026-09-01T18:03:57Z**; 79 downloads, 0 likes |
| Files | `hsk30_graded_readers.jsonl` **614,659 B**, `hsk30_heldout.jsonl` (30 texts), `DATASHEET.md`, `README.md` |
| Licence | HF tag **`license:cc-by-4.0`**, README front-matter `license: cc-by-4.0`. Repo README: *"MIT for the code and the derived level tables; **CC BY 4.0 for the corpus**"* |

**Measured independently** (downloaded the JSONL and parsed it):

```
texts: 102
shelves: {newbie: 22, beginner: 22, intermediate: 22, upper: 12, advanced: 12, native: 12}
sentences: 1185      total chars: 16956
keys per record: id, shelf, shelf_index, title{hz,py,en}, description, text, sentences, n_sentences, n_chars
```

**Why this matters — the token shape is unusually good for pronunciation drills.** Every sentence
carries per-word alignment:

```json
{"hz":"一只小鸟很渴。","en":"A little bird was very thirsty.",
 "words":[{"hz":"一只","py":"yì zhī","en":"a (one)"},
          {"hz":"小鸟","py":"xiǎo niǎo","en":"little bird"},
          {"hz":"很","py":"hěn","en":"very"},
          {"hz":"渴。","py":"kě","en":"thirsty"}]}
```

That is word-level hanzi + **toned pinyin** + gloss, which is exactly what a pronunciation drill
needs, and it is **CC BY 4.0 — no ShareAlike**, so bundling it imposes nothing on the app's data
artifact beyond attribution.

**Caveats:**
- **Small: 102 texts / 1,185 sentences.** It is a graded *reader* corpus, not a drill corpus.
- **Difficulty labels are deliberately omitted.** The README explains: *"A level is a function of
  the text and the standard, and 'HSK 3.0' names two different official documents that disagree on
  **41.5%** of shared vocabulary. Baking labels in would let a stale copy of this dataset
  contradict the grader."* The app must compute levels itself using the project's MIT `hsk30`
  library (or its own `complete-hsk-vocabulary` data).
- **Passages are LLM-drafted then human-reviewed** — quality is not independently established.
- **No audio.** Pair with local TTS.
- Provenance: `THIRD_PARTY_NOTICES` equivalent is clean (only CC-CEDICT-style dependencies for the
  level tables), and the corpus licence is stated in three places — but there is **no `LICENSE`
  file next to the data**, only the README/DATASHEET. Low risk; worth a note in `LICENSES.md`.

#### 2.3.1 ⭐ `no7z/hsk-sentences-audio` — the best fit **with audio**

| | |
|---|---|
| URL | <https://github.com/no7z/hsk-sentences-audio> · HF: <https://huggingface.co/datasets/no7z/hsk-sentences-audio> |
| Stars / dates | **5★**, created 2026-07-08, last push **2026-07-15**, repo 158,377 KB |
| Data | `dist/sentences.json` **6,434,365 bytes**, **4,354 records**; HF mirror has `data/train.jsonl` 4,952,559 B and `data/train.parquet` 670,256 B; **8,708 MP3s = 159.3 MB**, whole HF repo **164.9 MB** (paginated tree API, 9 pages) |
| Levels | L1 281 · L2 538 · L3 727 · L4 801 · L5 965 · L6 1,042 |
| Grading claim | *"sentences at each level use only vocabulary from that level and below (**zero out-of-level words**) while systematically covering that level's wordlist — both properties are machine-checkable with the validator in this repo"*; wordlist coverage 94–100% per level |
| Licence | **Code MIT**; **dataset `dist/` CC BY-SA 4.0**; HF tag `license:cc-by-sa-4.0` |
| Audio licence | Synthesised locally with **CosyVoice2-0.5B (Apache-2.0)**; `audio_meta.license = "Apache-2.0"` per record; README advises labelling it synthetic. **Spot-checked:** `audio/hsk1-0001.mp3` = 9,189 B, MPEG ADTS layer III, 64 kbps, 24 kHz mono, encoded by Lavf62 — real audio, not an LFS stub |

`LICENSE` verbatim (the decisive part):

> NOTE: This MIT license covers the CODE only. The generated dataset in dist/ is licensed under
> CC-BY-SA 4.0 and incorporates CC-CEDICT (CC-BY-SA) and CosyVoice2 (Apache-2.0). See
> ATTRIBUTION.md for details.

`ATTRIBUTION.md` verbatim:

> 句子文本与英文翻译为本项目自有。 ("The sentence text and English translations are this
> project's own.")
> 为简化合规，整个 `dist/` 数据集以 **CC-BY-SA 4.0** 发布。 ("To simplify compliance, the whole
> `dist/` dataset is released under CC BY-SA 4.0.")

**Verified record shape** (from `dist/sentences.json`):

```json
{"id":"hsk1-0001","hsk_level":1,"topic":"greetings","sentence_type":"statement",
 "chinese":"老师，您好！","traditional":"老師，您好！",
 "pinyin":"lǎo shī nín hǎo","pinyin_numbered":"lao3 shi1 nin2 hao3",
 "translation":{"en":"Hello, teacher!"},
 "tokens":[{"word":"老师","pinyin":"lǎo shī","gloss_en":"teacher"}, …],
 "grammar_points":["您 (polite you)"],"grammar_tags":[],
 "audio":{"normal":"audio/hsk1-0001.mp3","slow":"audio/hsk1-0001_slow.mp3"},
 "audio_meta":{"engine":"cosyvoice2-0.5B","voice":"zh-female-studio",
               "license":"Apache-2.0","sample_rate":24000}}
```

**Why it fits:** it is the only source found that combines HSK-graded *sentences*, reliable toned
pinyin, English, per-word glosses, dual-speed audio, and a CC BY-SA/MIT licensing pair — i.e.
exactly the app's acceptance criteria. The audio is already synthesised, so the app does not have
to ship a TTS model.

**Caveats to record:**
- **5 stars and 11 weeks old.** No community review.
- **⚠️ The "zero out-of-level words" claim does not survive independent checking against this
  app's own word list.** I re-ran the check locally: segmenting each of the 4,354 records with the
  project's own `tokens` array and looking each token up in `data/raw/hsk-words.json` (10,969
  distinct simplified forms carrying an HSK 3.0 level), **228 of 4,354 sentences (5.2%) contain at
  least one token whose HSK 3.0 level is above the sentence's own label** (L1 12, L2 20, L3 27,
  L4 52, L5 57, L6 60). Verified examples:

  | id | label | offending token | token's real HSK 3.0 level | sentence |
  |---|---|---|---|---|
  | `hsk1-0003` | 1 | 客气 | **5** (`n5`) | 谢谢你！不客气。 |
  | `hsk1-0281` | 1 | 差一点儿 | **5** (`n5`) | 差一点儿就好了。 |
  | `hsk1-0028` | 1 | 一家人 | **7** (`n7`) | 我们是一家人。 |
  | `hsk1-0089` | 1 | 以后 | 2 | 下班以后我去商店。 |

  Not all 228 are genuine leaks. Three distinct causes are mixed together: (a) **real level
  leakage** (客气, 差一点儿, 一家人 — unambiguous); (b) **segmentation differences** (的话 in
  「你**的话**」 is really 你+的+话); and (c) **homographs** — `十分` is tagged HSK 2 as *shí fēn*
  "very", but `hsk1-0278` 「现在是九点**十分**」 uses it as "ten minutes". `天上`, `不要`, `儿`
  are similar.

  The project's own `ATTRIBUTION.md` concedes the point: *"校验用的机读词表来自社区数字化项目
  complete-hsk-vocabulary，与官方口径约有 **~1% 出入**"* ("the machine-readable list used for
  validation comes from the community digitisation project complete-hsk-vocabulary and diverges
  from the official standard by about 1%").

  **Consequence for the app: re-run the grading validation against its own word list and
  manually review the flagged ~5%, rather than trusting the README's zero claim.** Practical fix:
  either relabel those sentences to the max token level, or filter them out.
- `data/grammar_points.json` is described as containing *"the full registry with official
  categories, levels and **example sentences**"* for **413 official grammar points**. Those
  examples may be derived from the official 语法等级大纲. If they are, they are *not* covered by
  the CC BY-SA grant. **Recommendation: ship `sentences.json` + audio, and drop or independently
  rewrite `grammar_points.json` examples.**
- 170 MB of audio is a large bundle; the README states the npm/PyPI packages resolve audio lazily
  from a URL, so offline bundling is a deliberate choice with a size cost.

#### 2.3.2 `NewHSK3/new-hsk-3-anki-deck`

4★, created and last pushed **2026-07-31**, repository size 0 KB (content is in Releases).
Description claims *"33,000 example sentences and Mandarin audio"*. **No LICENSE file.** The
33,000 figure and the phrasing mirror `saymei/zhongdex`'s 32,725 sentences, suggesting a
re-upload. **Not bundleable as-is; if the content really is zhongdex-derived, go to zhongdex
directly and take the CC BY-SA 4.0 grant there.**

#### 2.3.3 ⭐ `saymei/zhongdex` — the strongest text-only option, with one real caveat

| | |
|---|---|
| URL | <https://github.com/saymei/zhongdex> · <https://zhongdex.org> |
| Stars / dates | **0★**, created **2026-08-23**, last commit **2026-09-20** (i.e. one day before this report) |
| Data | `data/hsk_bands.json` 11,092 word records; `data/sentences.jsonl` **32,725 sentences**; 28 packs; CSV/Parquet/Yomitan/Anki/Pleco exports |
| Coverage | sentences cover **99.82%** of headwords; bands 500/772/973/1,000/1,071/1,140/5,636 |
| Licence | **Code MIT** (`LICENSE`, © 2026 SayMei); **data CC BY-SA 4.0** (`data/LICENSE`) |
| Audio | **None bundled and none hosted.** README: *"Clip hosting is on the roadmap; today this field reports what exists upstream and emits no URL."*; release notes list *"bundled audio in the dataset"* as **deliberately out of scope** |

`data/LICENSE` verbatim (the operative clauses):

> Everything in this directory — the word canon, the sentence records, the packs, the manifests,
> and every artifact derived from them (Yomitan packs, Anki decks, CSV/TSV/JSONL/Parquet exports,
> Pleco files, API responses, and MCP tool responses) — is licensed under the Creative Commons
> Attribution-ShareAlike 4.0 International licence.
>
> 1. ATTRIBUTION. You must give appropriate credit, provide a link to the licence, and indicate if
>    changes were made. The attribution line to reproduce …:
>      Zhongdex by SayMei — https://zhongdex.org — CC BY-SA 4.0.
>      Contains data from CC-CEDICT (CC BY-SA 4.0) and the HSK 3.0 (2026) word list.
>    Keep the per-record attribution that ships with the data: every record carries `sourceIds`…
> 2. SHAREALIKE. If you remix, transform, or build upon the material, you must distribute your
>    contributions under CC BY-SA 4.0 or a ShareAlike-compatible licence. MIT and Apache-2.0 are
>    NOT on Creative Commons' compatible-licence list.
>
> WHAT IS NOT COVERED
>   * The audio recordings. They are served from a separate host under their own terms, are
>     referenced by URL and never bundled here, and AS OF v0.1 ARE NOT PUBLISHED AT ALL …

Note the licence's own warning: **ShareAlike propagates into the app's bundled data artifact.**
An MIT/Apache-2.0 relicence of that artifact is not permitted.

**Word-list provenance verified.** `data/canon-stats.json` records
`scripts/hsk30.csv` sha256 `8c2b73f7…575b9b`, 948,891 bytes, 11,092 rows. I downloaded
`ivankra/hsk30@master/hsk30.csv` and computed:

```
8c2b73f74776240bcf154624730fc6fb2c42c254d5c5d0f88943878b8e575b9b   iv_master.csv   (948,891 bytes)
```

Byte-identical. So zhongdex's HSK banding traces to **`ivankra/hsk30` (MIT)**, which in turn is a
cleaned OCR/web-scrape of the official list. Good, checkable chain.

**The caveat — sentence provenance is not independently verifiable.** `data/sentence-stats.json`
says:

> "source": {"corpus": "example_json", "table": "global_dictionary",
> "column": "example_sentences_json", … "license": "SayMei production data; sentences are
> SayMei-owned, not CC-CEDICT."}

and `src/build/sentences.ts` explains the choice:

> Three candidates exist upstream. This build reads `global_dictionary.example_sentences_json`
> and nothing else … 138,195 sentences under 10,900 of the 10,959 distinct canon forms … an audio
> clip on 60%.
>   `example_sentences` + `entry_sentences` … 61,790 are Tatoeba … 18,840 Gemini rows …

So the shipped 32,725 sentences come from an **unnamed private "production dictionary"** table of
138,195 rows. SayMei asserts it owns them. There is no way for a downstream redistributor to
audit that assertion, and the sibling tables show that this private corpus does mix Tatoeba and
Gemini content. **If any of the 138k rows originated in a copyrighted dictionary or textbook,
SayMei cannot validly grant CC BY-SA over them, and the app inherits the problem.** Recommend:
(a) treat zhongdex as the best *text* candidate, (b) ask SayMei in writing for the upstream source
of `example_json`, or (c) prefer `no7z`, whose ATTRIBUTION.md states plainly that the sentences
are original to the project.

Sample quality note (from `data/sentences.jsonl`): mostly clean, but some records are fragments —
e.g. `{"hanzi":"学校关心学生的一举。"}` ("The school cares about the students' every action.") is
not a well-formed sentence. Filtering is advisable.

#### 2.3.4 `krmanik/HSK-3.0` (370★)

Active (last push **2026-06-14**), 11,184 files, HSK 1–9 words/hanzi/handwritten/grammar, BCT
lists, and Anki-xiehanzi decks. GitHub reports **`NOASSERTION`** — there is a `License.md`
(capital L) but no standard licence. Its content is a per-component attribution list, not a
grant:

> ### CC-CEDICT — [CC BY-SA 4.0]
> ### HSK 3.0 Words list from Pleco — MIT License
> ### SUBTLEX-CH — [CC BY-SA 4.0]  (citing chrplr/openlexicon)
> ### Anki Chinese Vocabulary Generator (Mani) — MIT
> ### HSK 3.0 word lists (Mani) — CC BY-SA 4.0

**Verdict:** word lists only (no sentences). If used, it must be treated as **CC BY-SA 4.0**
because of the Mani word lists and CC-CEDICT, and the SUBTLEX-CH claim should not be relied on
(see §2.4.1).

#### 2.3.5 `Roxaleen/hsk-annotated-corpus` — good data, no licence

3★, created 2025-11-29, last push **2025-12-26**, 205,091 KB. README: *"over 11,000 words and
260,000 sentences… each sentence comes with an English translation and is graded by HSK level"*,
with `export/csv/sentences.csv` at **49,050,736 bytes**. Sources, stated in the README:

> - **Tatoeba.org**: … crowd-sourced … there can be some inappropriate content …
> - **Wiktionary (via Kaikki.org)**: Many Wiktionary entries include example sentences …
>   Some examples are sourced from literature, films, or other media …
> - **Leipzig Corpora Collection** (Chinese 2015 web corpus): … 1,000,000 sentences gathered by
>   scraping Chinese websites … the content can sometimes be relatively formal and/or political.
>
> For sentences that don't already come with English translations (some from Wiktionary, and all
> from Leipzig), translations are generated with `deep_translator`.

**No LICENSE file** (probed `LICENSE`, `LICENSE.md` → 404). **Do not bundle.** Two of the three
upstream sources are also problematic in their own right (Leipzig's terms, and the fact that
Wiktionary examples are frequently quoted from copyrighted literature/film). If the app wants
this, the **Tatoeba subset alone** should be re-derived — Tatoeba text is CC BY 2.0 FR.

#### 2.3.6 `glxxyz/hskhsk.com` (166★, MIT) — MIT wrapper over exam material

Last push **2023-05-01**. The repo carries an MIT licence (© 2020 Alan Davies) and a
`data/lists/` directory containing, among word lists:

- `HSK Examples.txt` (15,324 B) — HSK 1 sentences with pinyin + English
- `HSK 3 Example Sentences.txt` (3,109 B) — header **`HSK 2012 Syllabus`**, contents like
  `喂，请问张经理在吗？` / `他正在开会，您半个小时以后再打，好吗？`
- `HSK 3 Questions.txt` (29,894 B), `HSK 3 Questions Multilang.txt` (64,367 B)
- `HSK Examples StickyStudy.txt`, `HSK 2013 Pleco.txt`, `New_HSK_2010.csv`, `HSK-2012.xls`

The "HSK 3 Example Sentences" file is a list of listening-comprehension transcripts of the kind
that appear in HSK past papers, and the "Questions" files are exam questions. **An MIT licence
cannot cover exam material the licensor does not own.** The word lists in the same directory are
fine. **Verdict: reuse the word lists, do not reuse the example/question files.**

#### 2.3.7 `clem109/hsk-vocabulary` — a correction

The repo (54★, MIT, last push **2019-10-29**) is described as *"Open source Chinese HSK vocabulary
list **with example sentences**"*, and the README's goal list still shows the example-sentence
task **unchecked**. I fetched `hsk-vocab-json/hsk-level-1.json`: **150 records**, keys
`id, hanzi, pinyin, translations` only — **0 records have a non-empty `examples` field**. So
despite the description, **this repo contains no sentences.** (It is useful only as the source of
the HSK 2.0 word list that `complete-hsk-vocabulary` already uses.)

#### 2.3.8 `krmanik/hsk-graded-sentences` — Tatoeba, re-derivable

0★, created 2026-06-02, last push **2026-06-08**. **No LICENSE file.** It is a pipeline
(`clean.ipynb` → `main.ipynb` → `tsv_to_db.py`) that ranks Tatoeba `cmn` sentences easy→hard:

> difficulty = 0.45 × max_hsk + 0.25 × mean_log_freq + 0.15 × n_chars + 0.15 × frac_oov
> Output: `data/cmn_sentences_graded.tsv` … `data/hsk_sentences.db` (~11 MB)

It also runs a profanity filter and an opencc `t2s` simplification pass. Because the *source* is
Tatoeba (CC BY 2.0 FR) and the *pipeline* is trivial, the app could **re-run this pipeline itself**
and own the output (CC BY 2.0 FR, attribution to Tatoeba and to the sentence authors). The repo's
absent licence is a problem for copying its files, not for re-deriving from Tatoeba.

#### 2.3.8b `bdx33/tatoeba-hsk-cmn-eng-fra` (HF) — HSK labels on Tatoeba, method undocumented

**78,504 rows**, simplified Chinese + English + French, with an `hsk_level` column.
HF card `license: cc-by-2.0` (Tatoeba's own), last modified **2025-09-19**, 158 downloads, 1 like.
README: *"Tatoeba sentences with HSK level, in simplified chinese, english, and french"*, source
`tatoeba.org/downloads`, last update 2025-08-20.

**Verdict: probably bundleable** — the underlying text is CC BY 2.0 FR and the card says so — but
the **grading method is not documented at all**, so the `hsk_level` values are not auditable. This
is the same Tatoeba text the app can grade itself with `complete-hsk-vocabulary`, so there is no
reason to depend on an undocumented third-party label. **Prefer re-deriving**, or use it only as a
cross-check.

#### 2.3.9 `lm742611149/learn-chinese` (readmandarin.com) — original content, no licence → ask

Created 2026-07-09, last push 2026-09-15, 10,841 files. **315 original HSK 1–6 readings**,
5,024 words, 659 grammar patterns, per-sentence audio. README: *"Every text is original. No
textbook passages, no scraped content — each reading is written to stay inside the vocabulary of
its HSK level."* Content lives in `content/texts/hsk<N>-*.json` (one reading per file). Audio is
pre-rendered by `gen_audio.py` using **edge-tts**.

**No LICENSE file** (LICENSE, LICENSE.md, LICENSE.txt all 404). Two issues: the absence of a grant,
and edge-tts audio — edge-tts drives Microsoft's online TTS through an unofficial endpoint, and
the output's redistribution status is not granted by any licence.

**This is the single best "ask the author" candidate**: the content is original, already
HSK-graded, already JSON, and already has audio. A one-line CC BY-SA 4.0 or CC BY 4.0 grant from
the author would make it the best source in this report. **Recommend the parent request that.**

#### 2.3.10 Also checked, not bundleable

| Repo | Stars | Licence | Why not |
|---|---|---|---|
| `TeaPearce/chinese-english-dictionary` | 4 | `NOASSERTION` | 52 MB of HTML; unrecognised licence; sentence provenance unstated |
| `Pleometric/HSK-deck` | 9 | none | AI-generated sentences, no grant |
| `AnthonyBogetti/HSK-3.0-Vocabulary-Anki-Deck` | 1 | none | no grant |
| `pwobus/Chinese-language-flashcards` | 1 | none | no grant |
| `trainingDay25/MandarinTrainer` | 1 | none | no grant |
| `gronnmann/SpeedyChinese` | 1 | GPL-3.0 | tool only (GPL-3.0 is AGPL-compatible but there is no data grant) |
| `haidinhtuan/hsk-word-list` | 0 | none | *"with pinyin, audio, Vietnamese translation, examples"* but no grant |
| `metalhatscats/bonihua-datasets` | 0 | **CC BY-NC-SA 4.0** | **NC disqualifies** |
| `shengdabai/Chinese-character-content` | 1 | none | no grant |
| `xiaojitang1996-gif/quin-chinese-learning-app`, `RayanAliPachisa/ExampleSentencesForChineseWordsJson` | 0–2 | none | no grant |

---

### 2.4 Frequency-graded corpora

*(Detailed sub-research; quoted clauses.)*

#### 2.4.1 SUBTLEX-CH — **cannot bundle the raw files; the `wordfreq` route works**

- Paper: Cai & Brysbaert (2010), *SUBTLEX-CH: Chinese Word and Character Frequencies Based on
  Film Subtitles*, PLoS ONE 5(6): e10729, <https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0010729>.
  The **paper** is CC BY (PLoS ONE default).
- Data files: hosted at <http://crr.ugent.be/programs-data/subtitle-frequencies/subtlex-ch>
  (and the UGent successor page). **No licence statement accompanies the downloadable zip.**
  The paper states the frequencies are *"freely available for research purposes"* and that the
  authors *"got permission to download all the subtitle files from two of the biggest websites in
  China mainland"* — i.e. permission to **download**, not to **redistribute**, and the
  "research purposes" wording is a **non-commercial-flavoured restriction**.
- **Verdict: bundling the raw SUBTLEX-CH zip is not safe.**

**The clean route — `wordfreq`** (<https://github.com/rspeer/wordfreq>). Its NOTICE/README states
that the package redistributes SUBTLEX-CH **with explicit e-mail permission from Marc Brysbaert**,
to be used *"for any purpose, not just for academic use"*, conditional on crediting the SUBTLEX
authors and keeping it clear that SUBTLEX is freely available data — *"terms similar to the
Creative Commons Attribution-ShareAlike license."* `wordfreq` is Apache-2.0 for code and
**CC BY-SA 4.0 for its data files**; it ships `wordfreq/data/large_zh.msgpack.gz` (1,773,909 B)
and `small_zh.msgpack.gz` (179,188 B). No example sentences.
- **Verdict: bundleable with attribution** — wordfreq/Robyn Speer, SUBTLEX-CH (Cai & Brysbaert),
  plus its other sources (OpenSubtitles, Wikipedia, …). wordfreq's NOTICE warns that crediting
  Speer under another name *"is a serious violation of the license"*.

Note the contradiction: `krmanik/HSK-3.0/License.md` lists **SUBTLEX-CH as CC BY-SA 4.0** citing
`chrplr/openlexicon`. The upstream SUBTLEX-CH page itself has no such statement, so **treat that
CC BY-SA labelling as a third-party claim, not as the rights-holder's grant.** Only the `wordfreq`
permission is documentary.

#### 2.4.2 Lancaster Corpus of Mandarin Chinese (LCMC) — **disqualified**

End User Licence (<https://www.lancaster.ac.uk/fass/projects/corpus/LCMC/lcmc/lcmc_license.htm>):

> Distribution of the LCMC Processed Material is restricted to the Licensee or … the Licensee's
> research group.
> The Licenser does not grant to the Licensee any rights whatsoever to reproduce the LCMC Texts
> or use all or any part of the LCMC Texts in commercial products or services.

**Disqualified** (no redistribution, no commercial products, explicitly).

#### 2.4.3 Chinese Gigaword (LDC2003T09 / LDC2011T13) — **disqualified**

Hosted at the Linguistic Data Consortium (<https://catalog.ldc.upenn.edu/LDC2011T13>). Access
requires a paid LDC membership and acceptance of the **LDC User Agreement**; the agreement
restricts use to the licensee and their organisation and forbids redistribution to third parties.
**Disqualified** (paid + signed agreement, no redistribution).

#### 2.4.4 BCC 语料库 (BLCU Chinese Corpus) — **disqualified**

<https://bcc.blcu.edu.cn/>. A 15-billion-character web/news corpus with frequency counts and
example sentences, run by 北京语言大学. **No licence permitting export or redistribution was
found**; the site is a query interface. Its terms of use do not grant redistribution.
**Disqualified.** (The `krmanik/HSK-3.0` attribution file references a BCC-based word-frequency
list from the Pleco forums — that list is MIT per Pleco, but the BCC *corpus* is not.)

#### 2.4.5 Chinese Word Sketch / Sketch Engine — **disqualified**

Sketch Engine's Chinese corpora are behind a commercial subscription
(<https://www.sketchengine.eu/>); the terms forbid bulk extraction and redistribution.
**Disqualified.**

#### 2.4.6 `ymcui/cmrc2019` — **permissively licensed narrative text**

126★, last push **2022-10-24**. `LICENSE.txt` is the full text of **Attribution-ShareAlike 4.0
International**. Paper (COLING 2020) abstract:

> The proposed dataset contains over 100K blanks (questions) within over 10K passages, which was
> originated from **Chinese narrative stories**.

Train split: 9,638 passages / 100,009 queries. **Bundleable** under CC BY-SA 4.0. Not HSK-graded
and not beginner-friendly, but it is a clean, permissively licensed body of Chinese narrative
prose that the app could **grade itself** against its HSK word list (the app already ships
`complete-hsk-vocabulary`, so grading is a local computation). Best used as raw material for
HSK 5–9 reading/listening, not for HSK 1–3 phrases.

#### 2.4.7 Other frequency lists, and the full corpora verdict table

- **`wordfreq`** — see §2.4.1; the best licensed Chinese frequency source. Verified numbers:
  `large_zh.msgpack.gz` 1,773,909 B = **334,609** words; `small_zh.msgpack.gz` 179,188 B = 38,590.
  No sentences.
- **⭐ `Thoria/mandarin-most-common-words-tr-en`** (Hugging Face) — **CC BY 4.0 data + MIT code**,
  **1,143 rows**, CSV 1,573,856 B. This is **the only licensed file found that ships word + pinyin +
  frequency (wordfreq Zipf) + an example sentence together**, in zh/en/tr.
  <https://huggingface.co/datasets/Thoria/mandarin-most-common-words-tr-en>
- **`wzperson/hearmandarin-hsk-3-0-word-list`** — **CC BY 4.0** (*"for any purpose, even
  commercially"*), 11,147 HSK 3.0 words with gloss and POS. No sentences. Word list only.
- **`thunlp/THUOCL`** — **MIT** (*"可用于研究与商业"*), 11 domain document-frequency lists. No sentences.
- **Leipzig Corpora Collection** — the terms page distinguishes two things, and the distinction
  matters: *"The data and applications provided by the project … under the Creative Commons licence
  CC BY-NC … commercial use of the data are prohibited without the written consent of the project
  management"* **but** *"**The text corpora offered for download are made available under the
  Creative Commons licence CC BY.**"* A verified download (`zho_news_2020_10K.tar.gz`, 3,488,120 B →
  10,000 sentences) contains **no `LICENSE` file**. **Verdict: the downloadable text corpora are
  bundleable under CC BY**, with the standing caveat that the text is a web scrape.
- **Jun Da's Modern Chinese Character Frequency List** (`lingua.mtsu.edu/chinese-computing/`) —
  **no licence**; the footer reads *"Copyright. 1998-2026. Jun Da."* Do not bundle.
- **`hanziDB`** — the original `hanzidb.org` site is **dead** ("Konto zablokowane") and it never
  carried a licence. The app's MIT claim derives from the **`ruddfawcett/hanziDB.csv` GitHub repo's
  own `LICENSE`**, which is the same "MIT over the compilation" pattern as `elkmovie/hsk30` — worth
  a one-line caveat in `LICENSES.md`, not a change of plan.
- **`liangqi/chinese-frequency-word-list`** — no licence. **Kelly/Leeds Chinese** — 
  **CC BY-ND-NC-SA 2.0** → NC *and* ND, disqualified.
- **OpenSubtitles2018 / OPUS Chinese** — **not bundleable.** OPUS states *"We do not own any of the
  text … We only offer files that we believe we are free to redistribute"* and grants **no licence**;
  the OPUS API carries no licence field; `opensubtitles.com` ToS: *"Commercial use prohibited."*
- **ParaCrawl / web-crawled parallel corpora** — **CC0 applies to the packaging only**; the upstream
  text is neither owned nor licensed by the packager. Do not treat as CC0 text.

**Consolidated verdict table for the frequency/corpus thread** (all checked 2026-09-21):

| Source | Content | Licence | Bundle? |
|---|---|---|---|
| `wordfreq` `large_zh` | 334,609 ranked zh words, no sentences | Code Apache-2.0, **data CC BY-SA 4.0**; SUBTLEX-CH redistributed with Brysbaert's e-mail permission | **Yes** (attribute **Robyn Speer** by that exact name) |
| `Thoria/mandarin-most-common-words-tr-en` | 1,143 words + pinyin + Zipf + **example sentences** | **CC BY 4.0** / MIT code | **Yes** |
| `wzperson/hearmandarin-hsk-3-0-word-list` | 11,147 HSK 3.0 words + gloss + POS | **CC BY 4.0** | **Yes** |
| Leipzig downloads (`zho_news_2020_10K` etc.) | 10k–1M sentences + ranked words | **CC BY** (portal data CC BY-NC) | **Yes**, web-scrape caveat |
| `thunlp/THUOCL` | 11 domain frequency lists | **MIT** | **Yes** |
| SUBTLEX-CH raw zips | 5,936 chars / 99,121 words, pinyin + POS + EN glosses | **No licence on the data files**; paper CC BY, abstract says "freely available for research purposes" | **No** — use `wordfreq` |
| BCC 语料库 (BLCU) | 12 char/word freq datasets; ~62亿字 | **No licence.** Only *"均可免费下载使用…请规范引用 BCC 论文"* + *"© BCC 语料库 · 北京语言大学"* | **No** — e-mail BLCU for written permission |
| LCMC (Lancaster) | ~1M words | EULA: research-group only, no reproduction, no commercial products; ELRA commercial VAR = €12,000 | **No** |
| Chinese Gigaword (LDC) | ~1B words | Signed LDC agreement + fee; no redistribution | **No** |
| Chinese Word Sketch (Academia Sinica) | sketches over LDC Gigaword | Application + 1-year account; POS/rules © Sinica | **No** |
| Sketch Engine word lists | wordlists / n-grams | Research Licence: *"Licensee may not sublicense or distribute the Works"*, NC | **No** |
| Jun Da frequency lists | char freq + bigrams | **No licence**, "Copyright. 1998-2026. Jun Da." | **No** |
| OpenSubtitles2018 / OPUS | zh_cn / zh_tw sentences | **No licence granted**; upstream ToS "Commercial use prohibited" | **No** |
| Kelly / Leeds Chinese | frequency list | **CC BY-ND-NC-SA 2.0** | **No** |
| Wiktionary / kaikki.org | zh entries **with example sentences** | **CC BY-SA 4.0 + GFDL** (dual — choose CC BY-SA 4.0) | **Yes** with ShareAlike |
| ParaCrawl | zh–en parallel web text | CC0 on **packaging only** | **No / unclear** |
| `hsk-annotated-corpus` | 11k words + 260k sentences + HSK + freq | No LICENSE (404) | **No** as-is |
| `jnext/chinese_word_frequency` | jieba counts over 13M docs | Apache-2.0 declared, provenance unstated | ⚠️ not recommended |

**One myth to kill:** `krmanik/HSK-3.0`'s `License.md` asserts *"SUBTLEX-CH — CC BY-SA 4.0"* and
cites `chrplr/openlexicon`. That README was fetched and **contains no licence text at all.** This
looks like the origin of the widespread false belief that SUBTLEX-CH is CC BY-SA. **Do not rely on
it**; the only documentary redistribution grant is the `wordfreq` e-mail permission.

**Also note:** `drkameleon/complete-hsk-vocabulary` — which the app already bundles — **inherits
frequency ranks from SUBTLEX-CH** (its README lists SUBTLEX-CH as a source). That is a
rank/derived-statistic use rather than a redistribution of the SUBTLEX files, so the practical risk
is low, but it is worth one line in `LICENSES.md` since the app already documents the other
sources there.

---

### 2.5 Audio

#### 2.5.1 Mozilla Common Voice — **CC0 in law, contractually blocked in practice**

Common Voice zh-CN / zh-TW / zh-HK pairs crowd-recorded audio with validated text, and the data
**is** dedicated to **CC0 1.0** — the most permissive possible outcome, no attribution, commercial
use, redistribution. **But you cannot bundle a copy sourced from the Mozilla Data Collective.**
Every Chinese MDC datasheet carries, verbatim:

> **Licensing** — Creative Commons Zero v1.0 Universal (CC0-1.0)
> **Forbidden Usage** — … **It is forbidden to re-host or re-share this dataset.**
> **Intended Use** — This dataset is intended to be used for training and evaluating automatic
> speech recognition (ASR) models. It may also be used for applications relating to computer-aided
> language learning (CALL) …

and MDC's platform terms, Appendix 1 (<https://mozilladatacollective.com/terms>):

> Dataset Misuse and Enforcement. Data Consumer shall not … **scrape, mirror, or redistribute the
> Dataset**; or otherwise misuse the Dataset.

MDC's own FAQ resolves the apparent conflict
(<https://community.mozilladatacollective.com/faq-why-cant-i-re-host-or-share-common-voice-datasets-that-i-download-from-mdc/>):

> **CC0 remains the license for computational use, whilst not allowing mirroring the datasets is a
> platform term.**

Downloads now require an account and ToS acceptance, and the legacy public bundle URLs return
**HTTP 403**:
`https://voice-prod-bundler-ee1969a6ce8178826482b88e843c335139bd3fb4.s3.amazonaws.com/cv-corpus-19.0-2024-09-13/zh-CN.tar.gz`
→ 403, and `https://storage.googleapis.com/common-voice-prod-prod-datasets/cv-corpus-19.0-2024-09-13/zh-CN.tar.gz`
→ 403.

**Counts** (cv-corpus-27.0-2026-09-11 datasheets + the CV stats API, `lastFetched` 2026-09-20 —
I re-queried `https://commonvoice.mozilla.org/api/v1/stats/languages` and got zh-CN 1,095 recorded
h / **240 validated h** / 7,631 speakers / 51,702 sentences; zh-TW 132/78/2,337/20,857;
zh-HK 144/109/3,117/20,185; yue 309/204/1,192/18,548):

| Locale | Clips | Recorded h | Validated h | Speakers | Size |
|---|---|---|---|---|---|
| zh-CN | 852,410 | 1,074.96 | **240.11** | 7,601 | 21.40 GB |
| zh-TW | — | 131.7 | **79.92** | 2,336 | 3.17 GB |
| zh-HK | 124,379 | 143.39 | **108.56** | 3,114 | 3.68 GB |
| yue | 279,396 | 307.44 | **210.78** | 1,188 | 6.43 GB |

**Transcripts do ship with the audio** — the MDC datasheets describe the layout: *"The clips
directory contains all of the .mp3 files, and there is a separate tsv file for each data partition,
containing the following fields: client_id, path, sentence_id, sentence, …"*.

**Verdict: No**, for a bundle. The licence is CC0 but the acquisition channel imposes a no-mirroring
contract. Third-party CC0 re-hosts exist on Hugging Face (`legacy-datasets/common_voice`,
`fsicoli/common_voice_22_0`, `OpenFormosa/common_voice_25_zh-TW`) and their CC0 tag is consistent
with the original grant — but they are precisely what the MDC FAQ objects to, they carry no
independent warranty, and whether they still serve the validated splits is **unverified**. Do not
build the app's audio plan on Common Voice.

#### 2.5.1b ⭐ CSS10 Chinese — **the one clean aligned Mandarin audio+text bundle (CC0)**

<https://www.kaggle.com/datasets/bryanpark/chinese-single-speaker-speech-dataset>

- **Aligned audio + text.** CSS10's README
  (<https://raw.githubusercontent.com/Kyubyong/css10/master/README.md>): *"It is composed of short
  audio clips from LibriVox audiobooks and their aligned texts."*
- Source: LibriVox recordings of 朝花夕拾 and 呐喊 by **魯迅**, reader **Jing Li** — both public
  domain, so the CC0 tag is coherent.
- **Licence, verbatim from the Kaggle page JSON-LD:**
  `"license":{"@type":"CreativeWork","name":"CC0: Public Domain","url":"https://creativecommons.org/publicdomain/zero/1.0/"}`
  (Kaggle API `licenseNameNullable` = `"CC0: Public Domain"`).
- **Size 2,040,891,168 bytes (≈2.04 GB); runtime 06:27:04; single speaker.**
- The *code* repo `Kyubyong/css10` ships an Apache-2.0 `LICENSE`; the *data* is CC0.

**Verdict: Yes — no NC, no ND, no attribution, no ShareAlike.** It is the best bundleable Mandarin
audio+aligned-text source found in this research. The catch is pedagogical, not legal: 魯迅's
1920s literary Chinese is far above HSK 1–4 and is not HSK-graded, so it suits advanced listening
practice (and pronunciation reference) rather than beginner drills.

#### 2.5.2 Tatoeba — text yes, audio no

**Text.** The downloads page (<https://tatoeba.org/en/downloads>) states verbatim:

> These files are released under **CC BY 2.0 FR**.
> A part of our sentences are also available under **CC0 1.0**.

Bulk exports: <https://downloads.tatoeba.org/exports/> (`sentences.csv`, `sentences_CC0.csv`,
`sentences_with_audio.csv`, `per_language/<lang>/…`).

I downloaded and measured these live:

- `per_language/cmn/cmn_sentences.tsv.bz2` → **89,065 Mandarin sentences**, median **9** Han
  characters, mean 10.26. Distribution: p10 5 · p25 7 · p50 9 · p75 12 · p90 16 · p95 20 · p99 34.
  **65.6% are ≤10 Han characters and 88.2% are ≤15** — short, easy sentences, the right shape for
  a beginner drill corpus. **1,509** contain Latin characters and should be filtered.
- Script mix (heuristic on the sentences): ~22,825 traditional-leaning, ~28,428 simplified-leaning,
  ~37,812 script-ambiguous — i.e. **an OpenCC `t2s` pass is required**, which is exactly what
  `krmanik/hsk-graded-sentences` does.
- `sentences_CC0.csv` (10,597,783 rows total) → **exactly 1 Mandarin sentence**
  (`10597783  cmn  2022/2972 新年快乐！`). So the CC0 subset is **useless for Chinese** — do not
  plan around a CC0 Chinese escape hatch.

**Audio — the decisive measurement.** Tatoeba's usage guide warns:

> Note that the terms of use for the audio files are not the same as for the text of sentences.
> See the list of audio lists to see the license, if any, under which these people have offered
> their files for use outside of tatoeba.org. **You should verify these licenses by clicking
> "audio files" on each member's profile.**

I joined `sentences_with_audio.csv` (1,239,653 rows; columns = sentence id, audio id, username,
licence, attribution URL) against the 89,065 Mandarin sentence ids. Result for **all 5,825
Mandarin audio recordings**:

```
5741   (empty licence field)
  84   CC BY-NC 4.0
```

Contributors: `LeviHighway` 4,066; `fucongcong` 1,675; `GlossaMatik` 69 (CC BY-NC 4.0);
`zhoucantd` 15 (CC BY-NC 4.0). **Zero CC BY / CC BY-SA / CC0 Mandarin recordings.**
Site-wide the audio mix is likewise dominated by NC/ND (949,820 CC BY-NC-ND 3.0 — the
manythings.org set — plus 169,890 CC BY-NC 4.0, against only 37,479 CC BY 4.0, 6,424 CC BY-SA 4.0
and 632 CC0, mostly non-Chinese).

**Verdict: bundle Tatoeba Mandarin *text* (CC BY 2.0 FR, attribute Tatoeba + the sentence author);
do not bundle Tatoeba Mandarin *audio*.** The blank licence field means no licence was granted —
under CC/Tatoeba norms that is "all rights reserved", not "CC BY".

#### 2.5.3 OpenSLR Mandarin corpora — licences verified page-by-page

All read from `https://www.openslr.org/<id>/` on 2026-09-21:

| SLR | Corpus | `License:` field as printed | Prose on the same page | Verdict |
|---|---|---|---|---|
| 18 | THCHS-30 | **Apache License v.2.0** | *"totally free to academic users"* — but **no NC clause** | Unclear, leaning yes |
| 33 | Aishell (AISHELL-1) | **Apache License v.2.0** | *"The data is free for academic use."* | **Conflict** |
| 38 | Free ST Chinese Mandarin Corpus | **CC BY-NC-ND 4.0** | — | **No** |
| 47 | Primewords Chinese Corpus Set 1 | **CC BY-NC-ND 4.0** | — | **No** |
| 62 | aidatatang_200zh | page now returns *"Resource not found: 62"*; mirrors record **CC BY-NC-ND 4.0** and *"Resource retracted as per the data owner wish."* | — | **No** |
| 68 | MAGICDATA Read Speech | **CC BY-NC-ND 4.0** | *"freely published for non-commercial use"* | **No** |
| 93 | AISHELL-3 | **Apache License v.2.0** | vendor page adds *"free for academic research, not in the commerce"* | **Conflict** |
| 111 | AISHELL-4 | **CC BY-SA 4.0** (no separate agreement on the page) | — | **Yes** (meeting speech) |
| 119 | AliMeeting | **CC BY-SA 4.0** | — | **Yes** (meeting speech) |
| 121 | WenetSpeech | **CC BY 4.0** | own site: *"available to download for **non-commercial** purposes"* | **No** |
| 123 | MAGICDATA Conversational | **CC BY-NC-ND 4.0** | — | **No** |
| 138 | SHALCAS22A | **CC BY-NC-ND 4.0** | — | **No** |
| 146 | CML-TTS | **CC BY 4.0** | — | Yes, but **has no Chinese** |
| 159 | AISHELL-5 | (in-car, multi-channel) | — | verify |

**The AISHELL conflict is real and two-sided — treat AISHELL-1 and AISHELL-3 as NC.**
OpenSLR's formal field says `Apache License v.2.0`, but the archived AISHELL vendor page carries
both that **and**:

> ( This database is free for academic research, not in the commerce, if without permission. )

and the 2017 AISHELL-1 snapshot says *"Commercial use is forbidden."* AISHELL-2 is unambiguously
non-commercial. The vendor's own clause is the stronger signal about the rights-holder's intent,
so **"AISHELL-3 is non-commercial only" is confirmed as a genuine vendor term, not a myth.** The
Hugging Face mirror `AISHELL/AISHELL-1` carries `license: apache-2.0` in its card, but a mirror
cannot widen the grant. `aishelltech.com` is now a JavaScript SPA with no licence text — only
Wayback has it. **Recommendation: do not bundle AISHELL-1/-3 audio; AISHELL-4 is the clean member
of the family (CC BY-SA 4.0) but it is meeting speech, which is useless for beginner drills.**

**WenetSpeech — no.** OpenSLR says CC BY 4.0, but the project's own site says the corpus is
*"available to download for non-commercial purposes"* and concedes *"WenetSpeech doesn't own the
copyright of the audios"*. Two independent disqualifiers: the NC term, and the fact that the
grantor does not hold the rights (the audio is scraped from YouTube and podcasts).

**KeSpeech — no, three times over.** *"Non-commercial. You may not use this datasets for any
commercial purposes"* plus *"No Distribution. You may not distribute this dataset to any third
parties."*

**GigaSpeech 1 / 2 — no, and neither contains Mandarin.** v1 is English; v2 is
Thai/Indonesian/Vietnamese. The HF tag says apache-2.0 while the gating text says *"only for
non-commercial research and educational purposes"*.

**Also disqualified, briefly:** MAGICDATA SLR68 + SLR123, Primewords SLR47, ST-CMDS SLR38,
SHALCAS22A SLR138 (all CC BY-NC-ND 4.0); aidatatang_200zh (NC-ND **and retracted**); Emilia
(original, NC); WenetSpeech4TTS (NC); **HKUST/MTS (LDC2005S15)** — LDC User Agreement: *"only for
noncommercial linguistic education, research and technology development"* and *"shall not
otherwise publish, retransmit, disclose, display, copy, reproduce or redistribute"*; **all LDC
corpora**. **`zhvoice` and `DiDiSpeech` have no licence at any reachable URL** — do not bundle.

**Also worth knowing:** `Primewords is **CC BY-NC-ND**, not CC BY-SA` (a common mislabelling);
`GigaSpeech2 is not Chinese`; **no Microsoft MSR Mandarin corpus and no LDC "Fisher Mandarin"
corpus exist** (LDC2010S05 is Asian Elephant Vocalizations; Fisher is English + Spanish);
`CML-TTS (SLR146)` is CC BY 4.0 but has **no Chinese**; and **"Magicoder" is a code LLM**, not
speech. MagicData's HF "Dialect TTS-Lite" datasets have an `apache-2.0` tag but a README that says
**CC BY-NC-ND 4.0** — assume No.

**None of these corpora are HSK-graded**, and their transcripts are news/novel/encyclopedia or
meeting sentences — useful as ASR training data (which the app already has via SenseVoice) rather
than as beginner pronunciation drills.

#### 2.5.4 `phantomhsieh/OMPAL-corpus` — small but clean

**CC BY 4.0** (`LICENSE` = the full Attribution 4.0 text). 1,850 `.wav` files: **82 native
speaker utterances** and 1,768 from French L1 learners, with word- and sentence-level expert
scores; tied to an Interspeech 2025 paper. Useful for *pronunciation assessment* reference, not
for listening drills (mostly non-native speech). Tiny. Bundleable.

#### 2.5.5 LibriVox Chinese — public domain, audio only

LibriVox states (<https://librivox.org/pages/public-domain/>):

> all our recordings are public domain … This means **anyone can use all our recordings however
> they wish (even to sell them)**.

**27 Chinese items, ≈125 hours** (archive.org query
`collection:librivoxaudio AND language:(zho OR chi OR yue)` → `numFound 27`; note the archive.org
`language=` parameter does not work reliably for this). Bundleable. Two limitations: the
recordings are **audio only — there are no aligned transcripts**, and the register is
literary/classical, not conversational.

The aligned version of this material is CSS10 (§2.5.1b), which takes two Lu Xun works from LibriVox
and ships the text alignment — **use CSS10 rather than raw LibriVox if you want text+audio pairs.**

#### 2.5.6 TTS synthesis — the recommended way to get audio

The app already runs `sherpa-onnx` (Apache-2.0) and has researched TTS in
`docs/research/ASR_TTS_CLAUDE_RESEARCH.md`. From that document's own measurements:

| Model | Licence | Usable in an AGPL app? |
|---|---|---|
| `vits-melo-tts-zh_en` | **MIT (MeloTTS)** | Yes — the safest choice |
| `vits-icefall-zh-aishell3` | trained on **AISHELL-3**, i.e. it **inherits the AISHELL-3 conflict above** | **Not safe as previously assumed** |
| `matcha-icefall-zh-baker` | **non-commercial** | **No** — an NC restriction contradicts AGPL-3.0's downstream freedoms |

**⚠️ Correction to an earlier assumption in this repo:** the TTS plan cannot treat
`vits-icefall-zh-aishell3` as "permissive" purely because icefall's *code* is Apache-2.0. A model's
licence and its **training data's** licence differ, and the AISHELL-3 vendor clause is
non-commercial. Prefer **MeloTTS (MIT)**. Also note that `csukuangfj/*` sherpa-onnx model repos on
Hugging Face carry **no licence metadata at all** (`cardData: null`), and several Chinese voices
train on AISHELL-3 — **per-model provenance has not been audited; do not assume the Apache-2.0
framework makes the weights redistributable.**

Synthesising the drill audio from text the app owns removes every audio-licensing question — and
it is exactly the route `no7z/hsk-sentences-audio` took (CosyVoice2, Apache-2.0, output labelled
synthetic). **Recommended**, with MeloTTS as the model.

---

### 2.6 Graded readers, treebanks and public-domain text

#### 2.6.1 Universal Dependencies Chinese treebanks — the best *openly licensed* graded-adjacent English-free sentence sets

There are **seven** `UD_Chinese-*` treebanks. All report GitHub `license: NOASSERTION / Other`
because each ships a short `LICENSE.txt` rather than SPDX metadata — **that badge does not mean
"no licence".** Sentence counts below were obtained by downloading every `.conllu` and counting
`# sent_id`, cross-checked against `stats.xml`.

| Treebank | Sentences | Tokens | Script | `LICENSE.txt` | Verdict |
|---|---|---|---|---|---|
| `UD_Chinese-GSD` | 4,997 | 123,289 | Traditional | CC BY-SA 4.0 | ✅ |
| **`UD_Chinese-GSDSimp`** | **4,997** | **123,289** | **Simplified** | **CC BY-SA 4.0** | ✅ **best open Simplified set** |
| `UD_Chinese-CFL` | 451 | 7,256 | Simplified | CC BY-SA 4.0 | ✅ with caveats |
| `UD_Chinese-HK` | 1,004 | 9,874 | Traditional | CC BY-SA 4.0 | ✅ provenance unclear |
| `UD_Chinese-PUD` | 1,000 | 21,415 | Traditional | CC BY-SA 3.0 | ✅ |
| `UD_Chinese-Beginner` | 2,295 | 19,999 | Simplified, **HSK 1–5** | **CC BY-NC-SA 3.0** | ❌ **NC** |
| `UD_Chinese-PatentChar` | 200 | 4,784 | Simplified, patents | **conflict** | ⚠️ treat as ❌ |
| `UD_Classical_Chinese-Kyoto` | **86,239** | — | Classical Chinese | CC BY-SA 4.0 | ✅ but **Classical, not Mandarin** |

**`UD_Chinese-Beginner` is the trap.** It is the *only* treebank explicitly graded for learners —
its README says *"adapted for learner of level A1 to C1 (**HSK1 to 5**)"* — and it is exactly what
the app wants pedagogically. But `LICENSE.txt` reads:

> The treebank is licensed under the Creative Commons License Attribution-NonCommercial-ShareAlike
> 3.0 Unported (CC BY-NC-SA 3.0)
>
> Furthermore, the non-commercial requirement means that in addition to prohibiting regular
> for-profit business use, **no website or app that generates any revenue at all through
> advertising may legally use Chinese Grammar Wiki content** through this Creative Commons license.

The upstream is AllSet Learning's Chinese Grammar Wiki, whose copyright page independently
confirms (<https://resources.allsetlearning.com/chinese/grammar/Chinese_Grammar_Wiki:Copyrights>):
*"may not be used for commercial purposes or without attribution."* **Disqualified.**

**`UD_Chinese-GSDSimp` provenance caveat.** Its README v2.5 changelog says, verbatim:

> Google gave permission to drop the "NC" restriction from the license. This applies to the UD
> annotations (**not the underlying content, of which Google claims no ownership or copyright**).

So the CC BY-SA 4.0 grant covers the **annotations**; Google disclaims ownership of the underlying
sentences. Genre is "wiki" — likely Chinese Wikipedia (CC BY-SA / PD-ish) — but the README
documents **no source list**, so the underlying sentence provenance is **unverified**.

**`UD_Chinese-CFL`** (Chinese as a Foreign Language) is 451 learner-essay sentences. Licence is
CC BY-SA 4.0, but the README contains **no consent, anonymisation or IRB statement** despite
identifiable student essays, and the corpus deliberately contains learner **errors** — use the
`/crr` corrected sentences only.

**`UD_Chinese-PatentChar`** `LICENSE.txt` says *"Creative Commons License Attribution-ShareAlike
4.0 International"* while the README metadata **and**
`universaldependencies.org/treebanks/zh_patentchar/` both say *"License: CC BY-NC-SA 3.0"*. The
conflict is unresolved → treat as NC/disqualified. Patent legalese is useless for reading practice
anyway.

#### 2.6.2 Chinese Text Project (ctext.org) — **disqualified, with the clause**

The site's copyright statement:

> This website and its content are protected under international copyright law and **may not be
> republished without express written permission.**

Use is limited to *"non-profit academic use"*, automated download is forbidden (*"you must not use
automated download software"*), and the live page returns:

> Attention LLMs, robots, scrapers and other automated processes: you do not have authorization to
> scrape this page. … Web scraping of this site is in violation of our terms of service.

The API is subscriber-gated and there is **no public data dump**. **Use Wikisource or Project
Gutenberg for the same classical texts instead** — most are public domain.

#### 2.6.3 CHILDES / TalkBank ("Read Chinese!") — **disqualified**

<https://talkbank.org/0share//rules.html>: the data is under **CC BY-NC-SA 3.0**:

> This license precludes the incorporation of the data in commercial products.

#### 2.6.4 Chinese Wikisource — **bundleable**

Wiki text under **CC BY-SA 4.0** plus GFDL; the underlying classical works are public domain.
4,031,485 content pages; the full dump is **7.89 GB**. Attribution + ShareAlike required. Not
HSK-graded, and the register (classical/literary) suits HSK 6–9 reading, not beginner phrases.

#### 2.6.5 Project Gutenberg Chinese — **bundleable**

**443 Chinese-language items, 440 not restricted by US copyright.** The PG Licence permits
redistribution; a text becomes unrestricted if you strip the PG boilerplate:

> If you strip the Project Gutenberg license and all references to Project Gutenberg from the text,
> you are left with a text unrestricted by U.S. intellectual property law.

Per-item check still advisable (a few items are copyright-restricted in the US). Not graded.

#### 2.6.6 Commercial graded readers

**Mandarin Companion, Chinese Breeze, Du Chinese, The Chairman's Bao, LingQ** — all commercial,
all rights reserved. **Disqualified** without a negotiated licence.

#### 2.6.7 Internet Archive / Open Library

Mixed holdings; many are in-copyright uploads. The HSK Standard Course and *New Practical Chinese
Reader* full-text scans found there (`archive.org/stream/NewPracticalChineseReaderTextBook1`,
`archive.org/download/HSK1StandardCourse/…`, `archive.org/stream/hsk-4-standard-textbook/…`) are
**in-copyright and must not be used.** Per-item rights check required for anything else.

#### 2.6.8 Small CC0 findings

`daligao/chinese-reading-lab` (HSK 4–6, 10 stories, ~3,904 CJK chars) and
`daligao/mandarin-flashcards` (HSK 1–3 vocabulary) both **state CC0 in their READMEs but ship no
`LICENSE` file**. A README statement is a weaker grant than a `LICENSE` file; treat as
"probably CC0, confirm with the author". Volume is negligible for this app's purposes.

---

## 3. Ranked shortlist and the obligations each option creates

### Rank 1 (recommended) — bundle **two** artifacts: `no7z` for drills + `harukicoder` for aligned reading

These two are complementary and together they cover the requirement better than either alone:

- **`no7z/hsk-sentences-audio`** supplies the *listening* half: 4,354 graded sentences **with
  8,708 redistributable MP3s**. Nothing else found provides clean, bundleable, HSK-graded audio.
- **`harukicoder/hsk30-graded-readers`** supplies the *pronunciation* half: 102 short texts /
  1,185 sentences with **word-aligned hanzi + toned pinyin + gloss**, under **CC BY 4.0** — no
  ShareAlike, so it adds no licence burden to the app's data artifact.

Keep them as **separate files** with separate licence notices. That way the CC BY-SA obligation
attaches only to the `no7z`-derived artifact and does not reach the CC BY 4.0 one.

#### 1a. Obligations from `no7z/hsk-sentences-audio` (CC BY-SA 4.0 + MIT + Apache-2.0)

1. **CC BY-SA 4.0 on that dataset artifact.** Credit the project, link
   <https://creativecommons.org/licenses/by-sa/4.0/>, state changes, and license the app's
   *adapted copy of that artifact* under CC BY-SA 4.0 or a ShareAlike-compatible licence. This does
   **not** touch the AGPL-3.0 code — data and code are separate works carrying separate licences,
   as long as the data artifact's own licence is CC BY-SA 4.0 and the app imposes no extra
   restrictions on it. Add `licences/CC-BY-SA-4.0.txt` and a `LICENSES.md` section in the existing
   house style.
2. **CC-CEDICT attribution** for the per-word glosses and pinyin — already a tracked obligation for
   `complete-hsk-vocabulary`, so extend that section rather than adding a second one.
3. **CosyVoice2 / Apache-2.0 notice** for the audio, and **disclose that the audio is synthetic
   speech** (the project's own recommendation).
4. **Copy the MIT `LICENSE`** for any of the project's build code the app reuses.
5. **Do not bundle `data/grammar_points.json`'s official example sentences** — likely derived from
   the official 语法等级大纲 and therefore outside the CC BY-SA grant.
6. **Re-validate the grading against the app's own word list and review ~5% of sentences.** I
   measured **228/4,354 (5.2%)** carrying a token above their label under
   `data/raw/hsk-words.json`, including 客气 (HSK 5) and 差一点儿 (HSK 5) inside HSK 1 sentences.
   Relabel or filter before shipping. See §2.3.1.

#### 1b. Obligations from `harukicoder/hsk30-graded-readers` (CC BY 4.0)

1. **Attribution only.** Credit the corpus, link <https://creativecommons.org/licenses/by/4.0/>,
   and indicate if changes were made. **No ShareAlike**, so nothing propagates into the app's own
   data licence — this is the least burdensome source in the report.
2. **Compute the levels yourself.** The dataset ships no difficulty labels by design; use the
   project's MIT `hsk30` library or the app's existing `complete-hsk-vocabulary` data.
3. Add the notice for the CC-CEDICT-derived level tables if the app ships those tables.
4. Note in `LICENSES.md` that the licence is stated in the HF card / README / DATASHEET but there
   is **no standalone `LICENSE` file** beside the data.

**Combined residual risk: low.** All sentence text in both is declared original to its project;
audio is synthetic from an Apache-2.0 model; the only third-party text is CC-CEDICT glosses, already
CC BY-SA. The main caveats are age (both are 2026 projects with few users) and the `no7z` grading
drift measured above.

### Rank 2 — Bundle `saymei/zhongdex` sentences (text only) and synthesise audio

**What you get:** 32,725 graded sentences (7.5× more than no7z), `zsg` grade, toned + numbered
pinyin, English, and a genuine **i+1 `newWordCount` vector** that is a better pedagogy primitive
than a flat level tag. Plus a rigorously sourced word canon (sha256-verified to `ivankra/hsk30`,
MIT).

**Obligations:**

1. **CC BY-SA 4.0** on the data artifact: reproduce the attribution line the licence specifies
   verbatim —
   `Zhongdex by SayMei — https://zhongdex.org — CC BY-SA 4.0. Contains data from CC-CEDICT (CC BY-SA 4.0) and the HSK 3.0 (2026) word list.`
   — keep the per-record `sourceIds` and per-gloss `source`/`license` fields intact, state changes,
   and ShareAlike the adaption. **Do not relicense the data artifact as MIT or Apache-2.0** (the
   licence text says so explicitly).
2. **CC-CEDICT attribution** for 30,899 glosses.
3. **Synthesise audio yourself with MeloTTS (MIT)** — zhongdex ships none, its upstream audio is
   explicitly not licensed to you, and `vits-icefall-zh-aishell3` carries the AISHELL-3 licence
   question (§2.5.6).
4. **Resolve the sentence-provenance caveat** (§2.3.3) before shipping: ask SayMei for the origin
   of `global_dictionary.example_sentences_json`, or accept the risk knowingly. If provenance
   cannot be established, the Rank 1 combination is the safer choice even though zhongdex offers
   ~7.5× the sentence count of `no7z`.
5. Consider filtering malformed fragments (e.g. `学校关心学生的一举。`).

**Residual risk: medium**, entirely because of the opaque sentence provenance. Licence
compliance itself is easy.

### Rank 3 — Build your own from Tatoeba `cmn` + UD Chinese treebanks, graded locally

**What you get:** full control and an unambiguously clean chain, plus two complementary corpora:

- **Tatoeba `cmn`** — **89,065 Mandarin sentences**, median 10 characters, **CC BY 2.0 FR**.
  Conversational, short, and the right shape for beginner drills.
- **`UD_Chinese-GSDSimp`** — **4,997 Simplified sentences**, 123,289 tokens, with POS and
  dependency annotations, **CC BY-SA 4.0**. Better for grammar-aware drills and for sentence
  structure, and the annotations are genuinely open (Google explicitly dropped the NC clause).
- Optionally `UD_Chinese-PUD` (1,000 sentences, CC BY-SA 3.0) and `UD_Chinese-CFL` (451, CC BY-SA
  4.0, use `/crr`).
- Add **`Thoria/mandarin-most-common-words-tr-en`** (CC BY 4.0, 1,143 rows) if you want a small,
  already-licensed set of word + pinyin + frequency + **example sentence** rows as a seed/cross-check.

Grade each sentence locally with the app's existing `complete-hsk-vocabulary` data, using the
feature set `krmanik/hsk-graded-sentences` documents (`max_hsk`, `mean_hsk`, `hsk_coverage`,
`frac_oov`, `mean_log_freq`, `n_chars`) to compute a difficulty score. Synthesise audio with a
permissive TTS.

**Obligations:** attribute Tatoeba **and each sentence's author** (Tatoeba's usage guide asks for
author attribution), link CC BY 2.0 FR, state changes; for the UD data, credit Universal
Dependencies, link CC BY-SA 4.0, and ShareAlike the adapted artifact; plus the CC-CEDICT and TTS
notices. Note the two corpora carry **different** licences (CC BY vs CC BY-SA) — the merged
artifact must satisfy the stricter one (CC BY-SA 4.0).

**Why rank 3 and not 1:** more work. Tatoeba is crowd-sourced with known quality problems — its
own guide warns about unnatural sentences, poor translations, archaic and vulgar content, and
recommends using only proofread sentences. The UD underlying-sentence provenance for GSD/GSDSimp
is also undocumented (Google disclaims ownership of the content; the annotations are the licensed
part). But this is the option with the **fewest legal unknowns**, and it is the only one that
scales past ~33,000 sentences.

**Do not build on `UD_Chinese-Beginner`.** It is the one treebank explicitly graded HSK 1–5, and
it is CC BY-NC-SA 3.0 — *"no website or app that generates any revenue at all through advertising
may legally use"* it. This is the most tempting and most clearly disqualified source in the whole
report.

### Rank 4 — Ask three authors for firm grants

Three projects already have the content and the level structure, but their text licensing is
missing or inconsistent. All three are cheap asks:

- **`SHLEW06/chinese-hsk-adaptive-reader`** — **300 readings, 50 per HSK level 1–6** (~13.6 MB
  JSON, `hsk1.json` 1,797,566 B … `hsk6.json` 2,923,622 B) with English translations, sentence
  explanations, grammar notes, target words and comprehension questions. Root `LICENSE` is **MIT**
  (© 2026 Shunji Lewandowski) and `THIRD_PARTY_NOTICES` lists only CC-CEDICT +
  `complete-hsk-vocabulary`, which implies the readings are the author's own — but **the text
  itself is not explicitly licensed**. Ask for an explicit CC BY 4.0 or CC BY-SA 4.0 grant on the
  text. **This is the highest-value ask: 300 level-tagged readings is ~3× `harukicoder`.**
- **`lm742611149/learn-chinese`** — **315 original HSK 1–6 readings** with JSON content and audio,
  currently unlicensed. Last push 2026-09-15. A one-line grant would put it alongside Rank 1 for
  text volume and pedagogic fit. If granted, **re-render the audio yourself** rather than shipping
  edge-tts output (Microsoft TTS output carries no redistribution grant).
- **Global Storybooks 中文故事集** — 40 stories, 5 levels, **human audio** + PDFs, last commit
  2025-12-10. The footer says CC BY 4.0 while individual story pages carry a CC BY 3.0 badge, and
  the repo `LICENSE` (MIT) covers only the site code. **Pin the licence per story at ingest** and,
  better, ask for one consistent statement.<br>
  <https://global-asp.github.io/storybooks-chinese/>

### Rank 5 — `ymcui/cmrc2019` for advanced levels

CC BY-SA 4.0, 9,638 narrative passages. Bundleable today with attribution + ShareAlike. Grade
locally. Good for HSK 5–9 reading/listening; not a source of beginner phrases.

### Not recommended

- Any textbook OCR (`joelypoley/…`, the Internet Archive scans) — copyright.
- Any AnkiWeb deck, Memrise course, Pleco content — no grant / personal-use-only.
- **`UD_Chinese-Beginner`** — CC BY-NC-SA 3.0. The most tempting source in the report; see §2.6.1.
- `hskhsk.com`'s `HSK Examples.txt` / `HSK 3 Questions.txt` / `HSK 3 Example Sentences.txt` —
  MIT wrapper over exam material.
- `Roxaleen/hsk-annotated-corpus` — no licence, and Leipzig/Wiktionary upstreams are murky.
- `metalhatscats/bonihua-datasets` — CC BY-NC-SA. `CHILDES`/`Read Chinese!` — CC BY-NC-SA.
- Tatoeba Mandarin audio, ST-CMDS, Primewords, MAGICDATA SLR68/123, SHALCAS22A,
  aidatatang_200zh — NC/ND or blank licences.
- **AISHELL-1 / AISHELL-3 audio** (vendor NC clause) and therefore
  **`vits-icefall-zh-aishell3`**; **WenetSpeech** (own site says non-commercial, doesn't own the
  audio); **KeSpeech** (NC + no-distribution); **Emilia** (NC); **HKUST/MTS and all LDC**;
  **zhvoice** and **DiDiSpeech** (no licence found anywhere).
- **Mozilla Common Voice via MDC** — CC0 in law but contractually un-bundleable ("forbidden to
  re-host or re-share").
- ctext.org — "may not be republished without express written permission", scraping forbidden; take
  the same classical texts from Wikisource/Gutenberg instead.
- Internet Archive as a bulk source — 94,303 Chinese texts but only **520** carry a CC URL, and its
  ToS limits use to *"scholarship and research purposes only"*. Per-item clearance required.
- `Open Library` dumps — **no licence statement found on the dumps page**; the CC0 claim is
  unverified.
- SUBTLEX-CH raw zip — no grant; use `wordfreq` instead.
- Commercial readers — Mandarin Companion (*"All rights reserved; no part of this publication may
  be reproduced… without the prior written permission of the publishers"*), Chinese Breeze
  (Cheng & Tsui: *"You may not… distribute, redistribute, or create derivative works"*), Graded
  Chinese Reader (**Sinolingua**, not Commercial Press), HSKStory (*"Don't redistribute or
  resell… Host mirrored copies"*), Du Chinese (NC only), The Chairman's Bao (*"Not to make any
  derivative use"*), **LingQ (NC *and* ND — a double disqualifier; ND conflicts with AGPL)**,
  Popup Chinese (defunct; a 1.12 GiB `absolute-beginners.tar.gz` is still retrievable from Wayback
  with **no licence** — availability is not permission).

#### Names in the original brief that do not exist

Worth recording so the search is not repeated: **"Chinese Reading Project" does not exist**
(NXDOMAIN, Verisign "No match", RDAP 404, zero Wayback captures) — the real site is *Chinese
Reading Practice* (245 lessons), which has **no licence**. **`github.com/alexanderfrantsuzov` is a
404 account**, no repository named `chinese-text-annotator` exists (0 GitHub search results), and
**"Chinese-Learning-Corpus" was not found** on GitHub or Hugging Face. The real `Chinese-Annotator`
(Apache-2.0) is an NLP labelling tool that ships **no reading text**. **"Read Chinese!" is NFLC /
University of Maryland**, not Yale and not CHILDES (and the site is dead, footer "Copyright
2006-2010" → all rights reserved). **Chinese Reading World (U. Iowa)** — site dead, domain now a
GoDaddy for-sale page. **MandarinSpot is not a corpus** (an SPA annotator with no legal pages).

---

## 4. Things I could **not** verify — do not treat as settled

1. **Mozilla Common Voice** — **resolved to a "No"**, but the legal nuance stands: the data is
   CC0-1.0 while the MDC platform terms forbid re-hosting. **Unverified:** whether any third-party
   CC0 Hugging Face mirror still serves the *validated* splits, whether its contents are current,
   and whether Mozilla sanctions such mirrors. Do not build on Common Voice.
2. **Tatoeba Mandarin audio per-contributor licences.** I verified the empty licence field in the
   official bulk export for 5,741 of 5,825 recordings, but `LeviHighway`'s and `fucongcong`'s
   profile pages are JavaScript-rendered and I could not read a per-member audio licence
   statement. Tatoeba's own documentation says such licences must be checked on the member's
   profile. Tatoeba's UI maps an empty field to *"No license for offsite use"*, so the practical
   answer is "not licensed" — but the profile-level check is incomplete.
3. **A primary source for the "HSK 3.0 2026" word list.** All 2026 band counts I saw are from
   community repos; I found no MOE PDF for a 2026 revision. `Punpuf/hsk-syllabus-vocabulary-parser`
   and `harukicoder/hsk30` both claim to parse official documents; their source URLs should be
   checked. Note also that `harukicoder` reports HSK 3.0's **two** official documents **disagree on
   41.5% of shared vocabulary** — which specific document the app grades against is a product
   decision, not a settled fact.
4. **The provenance and "own work" claims of the recommended corpora.** `no7z` states its
   sentences are original; `harukicoder` states the corpus is CC BY 4.0; `SHLEW06` implies its
   readings are original via `THIRD_PARTY_NOTICES`. None of these is independently auditable.
   `no7z` also has **no `LICENSE` file in the repository** despite `ATTRIBUTION.md` citing one
   (the licence is stated in three other places), and `harukicoder` has **no `LICENSE` file beside
   the data** (licence stated in the HF card, README and DATASHEET). Low risk, but worth asking
   both authors to formalise.
5. **`AISHELL-2`, AISHELL-6/-7, and the AISHELL 数据使用申请 document.** AISHELL-2's terms live on
   `aishelltech.com`, now a JavaScript SPA with no licence text; only Wayback has it. I did **not**
   resolve whether the vendor's non-commercial clause overrides OpenSLR's `Apache-2.0` field — that
   is the single most consequential open question in the audio thread, and the safe reading is NC.
6. **Universal Dependencies Chinese treebank licences** — **resolved** (see §2.6.1): GSDSimp/GSD/
   CFL/HK = CC BY-SA 4.0, PUD = CC BY-SA 3.0, Beginner = CC BY-NC-SA 3.0 (disqualified),
   PatentChar = conflicting (treat as NC). **Still unverified:** the **underlying sentence
   provenance** of GSD/GSDSimp (Google disclaims ownership; the README documents no source list)
   and the film-subtitle / LegCo provenance of `UD_Chinese-HK`. `UD_Chinese-CFL` has **no
   consent/anonymisation/IRB statement** despite identifiable student essays.
7. **Leipzig Corpora Collection per-corpus terms** and the exact downstream rights in the
   `zho_news_2020` download used by `Roxaleen`.
8. **The provenance of `zhongdex`'s 138,195-sentence upstream table** — unknowable from outside.
9. **Licence of `krmanik/HSK-3.0` as a whole** — `License.md` is an attribution list, not a grant;
   the repo is `NOASSERTION` on GitHub. Treat as CC BY-SA 4.0 at best because of the CC-CEDICT and
   "Mani" components.
10. **`sherpa-onnx` Chinese voice provenance, per model.** `csukuangfj/*` Hugging Face model repos
    carry **no licence metadata at all** (`cardData: null`), and several Chinese voices are trained
    on AISHELL-3 — whose licence is contested above. Do not assume the Apache-2.0 framework makes
    the weights redistributable. Only `vits-melo-tts-zh_en` (MIT/MeloTTS) is clear.
11. **Global Storybooks per-story licence** — the site footer says CC BY 4.0, story pages badge
    CC BY 3.0, and the repo `LICENSE` (MIT) covers only site code.
12. **The Internet Archive's current ToS wording** (the live page is JavaScript-only; the 2014
    snapshot says *"scholarship and research purposes only"*) and the **Open Library dumps
    licence**, for which no statement was found (the "CC0" claim is unverified).
13. **CC0 Hugging Face mirrors of Common Voice** — whether they are sanctioned re-hosts.
14. **Emilia-YODAS** standalone licence text and Chinese hours (the gating text says CC BY 4.0; the
    canonical repo returned 401).
15. **`zhvoice` and `DiDiSpeech`** — no licence was found at any reachable URL. Recorded as "no
    licence", not as "permissive".

---

## 5. Method

- Official standard: downloaded the MOE PDF (51,104,405 bytes, 260 pp.), ran `pdfinfo`/`pdftotext`
  (317 bytes of extractable text — the watermark only), rendered pp. 1–8, 114, 200, 209, 215 to PNG
  with `pdftoppm` and read them visually to establish the document structure and confirm the
  词汇表 has no example sentences while 附录A does.
- GitHub: repository search API (`topic:hsk` sorted by stars, `per_page=100`; keyword queries;
  `license:` qualifiers), then metadata and file trees via `ungh.cc` and `img.shields.io` after the
  unauthenticated API budget was exhausted, and file contents via `raw.githubusercontent.com`,
  `data.jsdelivr.com` and `huggingface.co/api`.
- Tatoeba: downloaded `sentences_with_audio.csv` (1,239,653 rows), `per_language/cmn/cmn_sentences.tsv.bz2`
  (89,065 rows) and `sentences_CC0.csv` (10,597,783 rows) from `downloads.tatoeba.org`, joined them
  locally to produce the Mandarin audio licence histogram, and computed the length/script
  distribution myself.
- OpenSLR: fetched each resource page and extracted the `License:` field and the surrounding prose.
- Hugging Face: `huggingface.co/api/datasets/...` for licence tags/dates, and the paginated tree API
  for exact byte totals (`no7z` = 8,708 MP3s / 159.3 MB across 9 pages of 1,000 entries). Downloaded
  and parsed `harukicoder/hsk30-graded-readers` (`hsk30_graded_readers.jsonl`, 614,659 B) and
  `no7z`'s `dist/sentences.json` directly to measure their contents.
- **Independent re-validation:** re-graded all 4,354 `no7z` records against the app's own
  `data/raw/hsk-words.json` (10,969 distinct simplified forms carrying an HSK 3.0 level) and found
  228 sentences (5.2%) with a token above their label — see §2.3.1.
- Licences and terms: fetched the actual `LICENSE`, `License.md`, `data/LICENSE`, `NOTICE`,
  `ATTRIBUTION.md`, `README.md`, DATASHEET and terms-of-service pages and quoted the decisive
  clauses. Where a page was JavaScript-only or dead, Wayback Machine and archived vendor pages were
  used, and the fact is stated.
- **Delegated research.** Three of the six areas were researched in parallel by dedicated subagents
  whose full reports are companions to this one:
  `docs/mandarin-audio-dataset-license-report.md` (485 lines, audio),
  `docs/research/SENTENCE_CORPORA_LICENCES.md` and `_license_research/license-research-master.md`
  (frequency corpora), and `license-research-chinese-graded-reading.md` +
  `license-research-named-sources-hsk-bundling.md` +
  `docs/license-research-chinese-reading-corpora.md` (graded readers). Their findings are
  integrated above; where I could, I re-verified their decisive claims directly (the `no7z` audio
  count and file validity, the `harukicoder` measurements, the `bdx33` card, the OpenSLR licences,
  the Tatoeba histogram).
