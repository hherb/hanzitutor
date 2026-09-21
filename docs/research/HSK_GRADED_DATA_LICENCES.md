# HSK-graded Chinese data: license report for a commercial OFFLINE app

Research date: **2026-09-21**. All URLs and license claims verified against primary sources (live HTTP fetches, full file downloads, API queries). Sources with no verifiable license are labelled as such rather than guessed.

Target use: commercial app, **all data bundled on-device, no network**.

---

## 1. Bottom line

**Yes — there is legally usable HSK-graded data, but the honest position has two layers.**

1. **The word→HSK-level mapping is available under real open licenses** (MIT and CC BY 4.0), from several independent projects that publish it commercially-reusable and state their provenance openly. A bare word→level table is also the *weakest* copyright subject matter available (facts / 通用数表 argument under 著作权法 Art. 5(3)).
2. **The upstream official lists are NOT openly licensed.** GF 0025-2021 is a 语言文字规范 (not a 法律/法规/决议/决定/命令), the free MOE PDF is watermarked 仅供查阅 on every page, the commercial edition is BLCU Press All-Rights-Reserved, and both CLEC and CTI assert copyright and grant no commercial reuse. Every third-party MIT/CC grant covers *that licensor's contribution*; none came from the rights-holder. This does not make the open licenses worthless, but it means the residual risk sits in the *official selection/arrangement* (著作权法 Art. 15 compilation), not in the code.

**Practical consequence:** bundle the open-licensed *mappings* and *authored sentence corpora* with full attribution; do **not** copy the official PDF's text, BLCUP textbook content, or the AllSet Grammar Wiki. For a clean, defensible bundle, the safest sentence data is the corpus whose text the licensor wrote themselves.

---

## 2. Master table

Legend for **Bundle?**: ✅ = openly licensed for commercial bundling · ⚠️ = bundleable but with a real catch (ShareAlike / upstream-copyright residual risk) · ❌ = disqualified (ARR, NC, no license, or research-only).

| Name | What it actually contains | Size | HSK version / levels | License (primary-source URL) | Bundle? | URL |
|---|---|---|---|---|---|---|
| **ivankra/hsk30** | Words: simplified, traditional, pinyin, POS, level, variants, CC-CEDICT key | **11,092 rows** (L1 500, L2 772, L3 973, L4 1000, L5 1071, L6 1140, L7-9 5636) + 3,000 chars + 624 grammar points | **HSK 3.0 / GF 0025-2021**, levels 1-6 + 7-9 | **MIT** — "Copyright (c) 2023 Ivan Krasilnikov / (c) 2021 Shawky / (c) 2021 Pleco Inc." — https://github.com/ivankra/hsk30/blob/master/LICENSE | ⚠️ | https://github.com/ivankra/hsk30 |
| **elkmovie/hsk30** (Pleco) | Words only + separate charlist; "OCR'ed but not extensively proofread" | wordlist.txt **11,110 lines** (incl. 4 comment lines); charlist.txt ~4,220 | HSK 3.0 / GF 0025-2021, 7 level sections | **MIT**, header in-file: "# Copyright (c) 2021 Pleco Inc. / # Distributed under MIT license" — https://github.com/elkmovie/hsk30/blob/main/LICENSE | ⚠️ | https://github.com/elkmovie/hsk30 |
| **drkameleon/complete-hsk-vocabulary** | Words: radical, level[], frequency, POS, traditional, pinyin/numeric/wadegiles/bopomofo/romatzyh, meanings, classifiers | **11,470 entries**, complete.json 9.7 MB. Inclusive counts: old 150/297/595/1193/2491/4991; new 506/1256/2209/3181/4240/5363/10969; newest 294/491/978/1950/3497/5181/10057 | **All three**: HSK 2.0 (`old-*`), GF 0025-2021 (`new-*`), 2025 exam syllabus (`newest-*`) | **MIT** — https://github.com/drkameleon/complete-hsk-vocabulary/blob/main/LICENSE | ⚠️ | https://github.com/drkameleon/complete-hsk-vocabulary |
| **wzperson/hearmandarin-hsk-3-0-word-list** | Words: simplified, traditional, pinyin, pinyin_num, hsk3_band, hsk3_seq, hsk2_level, pos_zh, pos_en, **in-house gloss_en** | **11,147 rows**; bands 1-7 = 300/200/500/1000/1600/1800/5600 (+147 HSK2.0-only); 5,000 rows carry HSK 2.0 level | **HSK 3.0 (2025 syllabus)**, bands 1-7, + HSK 2.0 alignment | **CC BY 4.0** — "You may copy, redistribute, and adapt this dataset for any purpose, including commercially" — https://github.com/wzperson/hearmandarin-hsk-3-0-word-list/blob/main/LICENSE | ✅ *(cleanest word list: explicitly avoids CC-CEDICT, so no ShareAlike)* | https://github.com/wzperson/hearmandarin-hsk-3-0-word-list |
| **harukicoder/hsk30-graded-readers** | **Word-aligned graded readers**: full passages + per-sentence English + per-word `{hz, py, en}` | **132 texts** (102 main + 30 disjoint held-out), 1,185 sentences, 8,682 word tokens, 14,417 graded chars | **HSK 3.0**, 6 shelves (newbie→native); labels deliberately not baked in — compute at runtime | **CC BY 4.0** — "share and adapt the material for any purpose, **including commercially**" — https://github.com/harukicoder/hsk30/blob/main/corpus/LICENSE | ✅ | https://huggingface.co/datasets/harukicoder/hsk30-graded-readers |
| **no7z/hsk-sentences-audio** | **Sentences + audio**: chinese, traditional, pinyin, pinyin_numbered, English, per-word gloss, grammar tags, normal+slow MP3 | **4,354 sentences** (L1 281 / L2 538 / L3 727 / L4 801 / L5 965 / L6 1042) + **8,708 MP3s** (verified real: MPEG layer III 64 kbps 24 kHz mono) | **HSK 3.0 (GF 0025-2021)**, levels 1-6 | Dataset **CC BY-SA 4.0**; code MIT — ATTRIBUTION.md: "整个 dist/ 数据集以 CC-BY-SA 4.0 发布" — https://github.com/no7z/hsk-sentences-audio/blob/main/README.md | ⚠️ *ShareAlike propagates to your derived data bundle* | https://huggingface.co/datasets/no7z/hsk-sentences-audio |
| **harukicoder/hsk30** (library) | **Grader**: word/char tables + `grade()` / `grade_tokens()` returning a level + coverage curve | Graders against 2.0 (**4,991 words**), 2021 (**10,977**), 2025 syllabus (**10,896** words / 3,088 chars) | HSK 2.0, GF 0025-2021, and 2025 syllabus | **MIT for the code and the derived level tables**; CC BY 4.0 for the corpus — https://github.com/harukicoder/hsk30 | ✅ | https://github.com/harukicoder/hsk30 |
| **krmanik/HSK-3.0-words-list** | Words, hanzi, handwritten chars, **grammar** (txt + JSON) | ~11k words, 3,000 hanzi, 624 grammar points; covers 2021 + "New HSK (2025)" | HSK 3.0 (2021) **and** 2025 syllabus | **CC BY-SA 4.0 claimed by repo author** (`License.md`; no root `LICENSE`) — https://github.com/krmanik/HSK-3.0-words-list | ⚠️ | https://github.com/krmanik/HSK-3.0-words-list |
| **glxxyz/hskhsk.com** | HSK 2.0 2012 official lists + "With Definitions" TSV + HSK Examples.txt (226 rows) | L1 149 / L2 151 / L3 302 / L4 600 / L5 1300 / L6 2499 lines | HSK 2.0 (Hanban 2012) | **MIT** repo — https://github.com/glxxyz/hskhsk.com/blob/main/LICENSE — **CONFLICTS** with the website footer: "…for non-commercial purposes… Please don't use anything on this site for commercial purposes without obtaining my permission" | ❌ *until clarified in writing* | https://github.com/glxxyz/hskhsk.com |
| **CC-CEDICT** (dictionary, not HSK-graded) | Chinese→English dictionary: traditional, simplified, pinyin, definitions | **125,083 entries** (release 2026-09-20) | n/a | **CC BY-SA 4.0** — "you are allowed to use this data for both non-commercial and commercial purposes provided that you: mention where you got the data from (attribution) and… share these changes under the same license" — https://www.mdbg.net/chinese/dictionary?page=cc-cedict | ✅ *with attribution + ShareAlike on the dictionary data* | https://www.mdbg.net/chinese/dictionary?page=cc-cedict |
| **Tatoeba** (ungraded sentence pool) | Chinese sentences + translations; per-sentence license via API | **89,065 cmn sentences** | **No HSK grading** (see §4) | Text default **CC BY 2.0 FR** — https://tatoeba.org/en/terms_of_use (§6.2); "We are not generally opposed to using our content for commercial purposes. However, this choice depends primarily on contributors." (§6.5) | ✅ *for sentences; but you must grade them yourself* | https://tatoeba.org/en/downloads |
| **bdx33/tatoeba-hsk-cmn-eng-fra** | Tatoeba sentences + a (undocumented) HSK level | **78,504 rows** (L1 9,745 / L2 13,368 / L3 14,121 / L4 10,032 / L5 7,971 / L6 5,398 / L7 13,855 / L8 4,005 / ungraded 9); 64,735 with English | Labelled 1-8 (9-band-ish) | Dataset card claims **cc-by-2.0** (inherited from Tatoeba) — https://huggingface.co/datasets/bdx33/tatoeba-hsk-cmn-eng-fra | ⚠️ *license plausible, but grading method undocumented and data quality poor* | https://huggingface.co/datasets/bdx33/tatoeba-hsk-cmn-eng-fra |
| **Tiagodfs/hsk-3.0-dataset** | Word CSV: id, hsk_level, chinese, pinyin, english | **5,456 rows** (500/772/973/1000/1071/1140) | **Mislabelled**: these are the **old HSK 2.0** counts, not HSK 3.0 | Tagged **cc0-1.0** — https://huggingface.co/datasets/Tiagodfs/hsk-3.0-dataset | ⚠️ *CC0 tag is the uploader's; content is the old official list* | https://huggingface.co/datasets/Tiagodfs/hsk-3.0-dataset |
| **willfliaw/hsk-dataset** / **joshuaDami/hsk-dataset** | Same HSK word CSV (level, hanzi, pinyin, pinyin_tone/num, english, pos, tts_url) | "selected levels" | Unspecified HSK version | **cc-by-4.0**; README itself warns: "Verify original source terms if you plan to redistribute or use commercially." | ⚠️ | https://huggingface.co/datasets/willfliaw/hsk-dataset |
| **Universal Dependencies UD_Chinese-Beginner** | Manually annotated sentences from the AllSet Grammar Wiki with `level = A1..C1` | 2,295 sentences / ~20k tokens (of ~4,300 planned) | CEFR A1-C1 ≈ HSK 1-5 | **CC BY-NC-SA 3.0** — https://github.com/UniversalDependencies/UD_Chinese-Beginner/blob/master/LICENSE.txt | ❌ **NC** | https://github.com/UniversalDependencies/UD_Chinese-Beginner |
| **CTRDG** (Chinese Text ReaDability Grading) | Text\tlabel pairs OCR'd from 《HSK 标准教程》 + workbooks + 真题集 (249 source docs) | 5,721 texts / 24,291 sentences (4,576/572/573) | HSK 1-6 | **No LICENSE file (404)**; upstream is BLCUP copyright | ❌ | https://github.com/CocoTan1020/CTRDG |
| **Mendeley "HSK1-9 dataset level grading model"** | OCR'd lesson texts from HSK Standard Course, 真题集, Developing Chinese, K-12 Standard Chinese | 13 text classes | HSK 1-9 | Marked **CC BY 4.0**, but built from BLCUP textbooks; Mendeley adds "further permission may be required for any content… belonging to a third party" — https://data.mendeley.com/datasets/mmch43zw7y/1 | ❌ | https://data.mendeley.com/datasets/mmch43zw7y/1 |
| **Roxaleen/hsk-annotated-corpus** | 260k sentences w/ English + POS tags; 11k words w/ HSK level, pinyin, CC-CEDICT defs | "over 11,000 words and 260,000 sentences" | HSK 1-7 (7 = 7-9) | **No LICENSE file (404)** = all rights reserved; mixed upstream (Tatoeba/Wiktionary/Leipzig) | ❌ | https://github.com/Roxaleen/hsk-annotated-corpus |
| **AllSet Chinese Grammar Wiki** | Grammar points w/ example sentences, pinyin; tagged HSK 1-6 **and CEFR A1/A2/B1/B2** (no HSK 3.0) | 510 points (HSK1 54 / 2 79 / 3 87 / 4 115 / 5 109 / 6 66) | HSK 1-6 (old) + CEFR | **CC BY-NC-SA 3.0** — © page: "may not be used for commercial purposes or without attribution… no website or app that generates any revenue at all through advertising may legally use Chinese Grammar Wiki content" — https://resources.allsetlearning.com/chinese/grammar/Chinese_Grammar_Wiki:Copyrights | ❌ **NC** | https://resources.allsetlearning.com/chinese/grammar/ |
| **HSK Academy** | Word lists + quizzes + PDFs | HSK 1-6: 150/150/300/600/1300/2500 | HSK 2.0 | **All Rights Reserved** — "Any attempt of use, reproduction… without prior written permission from HSK Academy, is absolutely forbidden" — https://hsk.academy/en/privacy_policy | ❌ | https://hsk.academy |
| **ChinesePod / DuChinese / Mandarin Companion / Immersive Chinese** | Lessons / graded readings / graded readers / serial course | 4,000+ lessons / 3,000+ readings / 18 titles / 185 lessons | Own levels (not HSK, or loosely mapped) | **All Rights Reserved** in every case. Immersive Chinese has **no terms page at all** (/terms, /privacy, /eula, /about, /faq all 404) → default copyright. | ❌ | https://chinesepod.com/terms · https://duchinese.net/legal · https://mandarincompanion.com · https://immersivechinese.com |
| **AnkiWeb shared HSK decks** | Words/sentences/audio, highly variable | 423 decks match "HSK" | Both HSK 2.0 and 3.0 | Site-wide: "This license is for **personal use only**, and the deck may not be redistributed…" — https://ankiweb.net/account/terms. ~383 of 423 state no license at all. | ❌ | https://ankiweb.net/shared/decks?search=HSK |
| **HSK Standard Course** (HSK标准教程, BLCUP) | Textbooks: vocab, dialogues, sentences, audio | Series | HSK 2.0 (1-6) | **All Rights Reserved** — "版权所有: 北京语言大学出版社有限公司，All Rights Reserved" — http://www.blcup.com/ | ❌ | http://www.blcup.com/PInfo/Index/14190 |
| **Official GF 0025-2021 / 2025 syllabus / chinesetest.cn** | Official 词汇表 / 汉字表 / 语法大纲 | 11,092 words (2021); 406-pp 2025 syllabus | HSK 3.0 (both documents) | **No open license.** MOE PDF watermarked 仅供查阅 on every page; BLCU commercial edition ARR; CTI: reproduction only for "新闻性或资料性公共免费信息"; CLEC: 「未经许可不得转载」. See §6. | ❌ | http://www.moe.gov.cn/jyb_xwfb/gzdt_gzdt/s5987/202103/W020210329527301787356.pdf |
| **LDC (all Chinese holdings)** | **No HSK corpus and no Chinese learner corpus exists at all** | — | — | **Non-commercial only** — "only for non-commercial linguistic education, research and technology development… must join LDC as a For-Profit Member… prior to release" — https://catalog.ldc.upenn.edu/license/ldc-non-members-agreement.pdf | ❌ | https://catalog.ldc.upenn.edu |
| **HSK动态作文语料库 2.0** (BLCU) | HSK 高等 exam essays 1992-2005 | 11,569 essays / ~4.24M chars | Old HSK 高等 | 「任何单位或个人未经允许不得将有关内容用于商业或其他营利性用途」; web query only, auto-download capped at 500 — http://yuyanziyuan.blcu.edu.cn/info/1043/1501.htm | ❌ | http://yuyanziyuan.blcu.edu.cn/info/1043/1501.htm |
| **MCTS / CSS / CSSWiki / HanLS / YACLC / CCL-CLTC** | Chinese simplification & learner-error corpora | 383-8,000 items each | None carry HSK levels | No LICENSE (YACLC, CLTC, CTRDG, CSS, CSSWiki); **GPL-3.0** (MCTS); HanLS dataset never de-anonymised ("available at github.com/anonymous") | ❌ | https://github.com/blcuicall/mcts |
| **Wikidata** | — | — | **No HSK property exists** | CC0 1.0 — https://www.wikidata.org/wiki/Wikidata:Licensing | ❌ *zero coverage* | https://www.wikidata.org/wiki/Wikidata:Property_proposal/HSK_ID |
| **Wiktionary (en/zh)** | HSK **2.0** word→level lists (~5,000 words, exact official split), HSK **3.0** levels 1-9 tables, old HSK 1.0 per-entry categories (~20,619 pages) | Appendix pages; 152 links on v2.0/L1, 507 on v3.0/L1 | HSK 1.0 (per-entry), 2.0, 3.0 (appendix lists) | **CC BY-SA 4.0** + GFDL — https://foundation.wikimedia.org/wiki/Policy:Terms_of_Use | ⚠️ *ShareAlike on the derived dataset; no per-entry modern levels* | https://en.wiktionary.org/wiki/Appendix:HSK_list_of_Mandarin_words_v2.0/level_1 |

---

## 3. Per-source notes that actually change a decision

### 3.1 Tatoeba — confirmed useless for HSK grading (all three claims verified)

- **Exactly one tag matching "HSK" exists in the entire tag list.** `tag_metadata.csv` (11,179 rows, downloaded from https://downloads.tatoeba.org/exports/tag_metadata.csv) yields a single match: `1826  HSK  sysko  2011-02-07 15:49:18`. **There are no HSK-level tags — no HSK1…HSK6, no `hsk-3.0`, nothing.**
- **Exactly one sentence carries that tag.** Verified three independent ways:
  1. Full `tags_detailed.csv` download (67,225,247 bytes) → `awk '$1==1826'` returns 1 row: `1826  481336  sysko  2011-02-07 15:49:18`.
  2. Web UI: `https://tatoeba.org/en/sentences/search?query=&from=cmn&to=eng&tags=HSK` renders **"Sentences in Mandarin Chinese translated into English (1 result)"**.
  3. API: sentence 481336 = `这个图书馆里禁止看书。` ("This library forbids reading books."), license `CC BY 2.0 FR`, owner `fucongcong`.
  (A sentence whose meaning is that you may not read in a library is not a useful beginner item anyway.)
- **Tatoeba has no difficulty rating field.** `sentences_detailed.csv` — the full export and the per-language `cmn_sentences_detailed.tsv` — has **6 columns only**: `id, lang, text, username, date_added, date_modified`. The database model (`src/Model/Table/SentencesTable.php`) defines `correctness` (`MIN_CORRECTNESS = -1`, `MAX_CORRECTNESS = 0`), a moderation flag, and **no difficulty column anywhere**. Tatoeba's historical difficulty-rating feature is gone.
- **License is good, though.** TOS §6.2: "Tatoeba's technical infrastructure uses the default Creative Commons Attribution 2.0 France license (CC-BY 2.0 FR) for the use of textual sentences." §6.5: "We are not generally opposed to using our content for commercial purposes. However, this choice depends primarily on contributors." Caveats: licenses are **per-sentence** and can be overridden (ND/SA/non-commercial, especially for audio), and there is **no bulk license export** — you must read `license` per sentence from the API. A 3,000-sentence random sample of `cmn` returned **100% "CC BY 2.0 FR"**.
- Size: **89,065 Chinese sentences**.
- **Verdict:** Tatoeba is an excellent CC BY 2.0 FR *sentence pool* (89k sentences, attribution-only, commercial OK) but supplies **zero HSK grading**. Grade it yourself with `harukicoder/hsk30`.

### 3.2 The two "HSK 3.0" documents — this is why level numbers disagree

There are two different official documents both called "HSK 3.0", and they disagree on **41.5% of shared vocabulary** and **40.7% of shared characters**:

| Document | Issuer | Date | Words | Chars |
|---|---|---|---|---|
| 《国际中文教育中文水平等级标准》 **GF 0025-2021** | 教育部 + 国家语委 (national 语言文字规范) | in force 2021-07-01 | 10,977 | 3,000 |
| **新版HSK考试大纲** (2025 exam syllabus) | 中外语言交流合作中心 (CLEC) | published 2025-11, **in force 2026-07** | 10,896 | 3,088 |

Regrading the same 102 graded readers against one rather than the other changes the level of **48%** of them. **Since today is 2026-09, the 2025 syllabus is the one in force and the one learners are tested on.** Any dataset tagged "HSK 3.0" without naming which document is ambiguous — prefer sources that state it (`wzperson/hearmandarin` = 2025 syllabus; `ivankra/hsk30` and `elkmovie/hsk30` = 2021 standard; `drkameleon` carries all three; `no7z` uses GF 0025-2021).

### 3.3 HF dataset cards that look promising but are not

Sampled every HSK-matching dataset card on the HuggingFace hub. Beyond the table above:

- `TwinkStart/speech-HSK` — **no license tag at all**, and tiny (hsk1 = 5 examples, hsk2 = 15, hsk3 = 20). ❌
- `swaption2009/20k-en-zh-translation-pinyin-hsk` — **no license**; 20k sentences with HSK level 1-4, derived from a Mnemosyne flashcard deck by Brian Vaughan. ❌
- `infinite-dataset-hub/HSK_Level1_Words`, `infinite-dataset-hub/Hsk2Corpus` — MIT but **AI-generated with Phi-3-mini**, card says "may be inaccurate or false". Not HSK data. ❌
- `MariyaMegre/hsk-dataset` — apache-2.0 tag over an unmodified dataset-card *template* ("[More Information Needed]" throughout); one file, `hsk4_dataset.csv`. Unverifiable. ❌
- `twnlp/lang8_hsk` — MIT, but this is a **Chinese spelling/grammar error-correction** corpus (Lang8 + HSK learner data, 1,568,885 items), not HSK grading. ❌
- `hskfd/NLP`, `hskfd/NLP_text`, `Finraeth/Hskkx`, `Finraeth/Hskxns`, `hsk24889/record-test`, `hskha21/Brain-Stroke-Diagnosis`, `hs-knowledge/*`, `hskang0906/*`, `hskhyl/*` — name collisions on "hsk", unrelated or empty. ❌
- Searches for `chinese readability`, `mandarin graded`, `chinese graded reader`, and `filter=language:zh + readability` returned **zero** licensed Chinese readability datasets on the hub.

### 3.4 Academic corpora — the gap is real

- **LDC has no Chinese learner or HSK corpus at all.** `name_cont=learner` returns an Arabic Learner Corpus, an English learner treebank, and the Xi'an Multi-Language Learner Corpus (LDC2025T03, 15 languages — **no Chinese**). Its license is non-commercial regardless.
- **UD_Chinese-Beginner** is the only academic CEFR/HSK-adjacent *sentence* resource and it is **CC BY-NC-SA 3.0** — the copyright page adds that this bans even ad-supported apps.
- **CTRDG** and the **Mendeley HSK1-9** dataset are the only academic HSK-readability corpora, and both are OCR'd from BLCUP 《HSK 标准教程》/真题集. CTRDG has no license; Mendeley's CC BY 4.0 tag does not launder BLCUP copyright.
- Simplification corpora (`MCTS` GPL-3.0; `CSS`, `CSSWiki`, `YACLC`, `CLTC` with no license; `HanLS` never released) are all unusable.
- 语料库在线 / 国家语委现代汉语语料库 appears **retired** (TLS failure; `cncorpus.org` is a parked domain-for-sale page). JCLC's homepage returns HTTP 410. BCC is query-only with a frequency-download centre and no commercial grant.

### 3.5 Wikidata / Wiktionary — verified negative and partial

- **Wikidata has NO HSK-level property.** `wbsearchentities&type=property&search=HSK` → 0 results; namespace-120 search → 0 hits; SPARQL over property labels/descriptions → 0 hits. The one attempt, **Wikidata:Property proposal/HSK ID** ("HSK level" / "HSK词汇等级", domain `lexeme`, quantity 1-6), was closed **`not done` on 2023-06-01**: *"no consensus of proposed property at this time."* There is **no CEFR property** either, and no HSK 3.0 level items. **Coverage = zero; do not plan around it.**
- **Wiktionary carries levels, but only obsolete ones per-entry.** en.wiktionary `Category:Mandarin by difficulty level` = Beginning 2,492 / Elementary 4,226 / Intermediate 4,363 / Advanced 6,049 (17,130 pages) — all the **pre-2010 HSK 1.0 (甲乙丙丁)** scheme, applied inconsistently (你好 and 学习 carry none). zh.wiktionary adds 3,489 pages of the same 1.0 scheme.
- **The useful part is the Appendix word lists**, which are complete: `Appendix:HSK list of Mandarin words v2.0/level 1-6` (150/150/300/600/1300/2500 — exact official split; I verified 152 word links on level 1) and `v3.0/level 1-6` + `v3.0/level 7-9` (level 1 = 507 unique links; the 7-9 page is **not** official — it reproduces an older 2010 subdivision).
- **No per-entry HSK 2.0 or 3.0 labels exist anywhere, no sentence-level HSK data, and no CEFR mapping for Chinese.** License is **CC BY-SA 4.0** (+GFDL), so a compiled word→level table derived from the appendices would inherit **ShareAlike**.

### 3.6 Commercial and community sites — all disqualifying

Confirmed All-Rights-Reserved or NC, with the clauses quoted in the table: HSK Academy (privacy policy doubles as the IP notice; its robots.txt is a bare EU DSM Art. 4 rights reservation), AllSet Grammar Wiki (**CC BY-NC-SA 3.0**, explicitly also banning ad-supported apps; HSK tags are 1-6 + CEFR, **no HSK 3.0**), AllSet Vocabulary Wiki (BY-NC-SA), ChinesePod, DuChinese (Sinamon AB), Mandarin Companion (Mind Spark Press), Immersive Chinese (no terms page → default copyright), AnkiWeb (site-wide personal-use-only grant), BLCUP, Pleco (per-dictionary proprietary; only its CC-CEDICT/CFDICT/HanDeDict components are open, and those should be taken from upstream not from Pleco), and hskhsk.com's **website** footer (non-commercial only) which conflicts with its **GitHub** MIT license.

### 3.7 CC-CEDICT — the one clean dictionary

CC-CEDICT is **CC BY-SA 4.0**, explicitly permits commercial use with attribution and ShareAlike — quoted verbatim in the table. Note a version discrepancy to resolve before shipping: mdbg.net says **4.0** while cc-cedict.org/wiki still says **3.0**. Because most HF/community HSK datasets derive their glosses and pinyin from CC-CEDICT, **those datasets carry ShareAlike into your derived data** — that is exactly why `wzperson/hearmandarin` wrote its glosses in-house, and it is the reason to prefer it.

---

## 4. The upstream copyright crux (read this before relying on any MIT/CC word list)

**The official HSK vocabulary is not openly licensed.** Verified:

- **GF 0025-2021 is a 语言文字规范, not an exempt government document.** Cover: 「中华人民共和国教育部 国家语言文字工作委员会 发布」; MOE's own announcement calls it 「国家语委语言文字规范」. 著作权法 Art. 5 exempts 「法律、法规，国家机关的决议、决定、命令和其他具有立法、行政、司法性质的文件，及其官方正式译文」 — a language-script technical specification is a different category, so the exemption **does not apply** on its face.
- **The free government PDF is not a license.** http://www.moe.gov.cn/jyb_xwfb/gzdt_gzdt/s5987/202103/W020210329527301787356.pdf (51,104,405 bytes, 260 pages) carries a diagonal **仅供查阅** ("for reference only") watermark on every page and its text is not extractable.
- **It is also a commercial publication.** BLCU Press, ISBN **9787561957196**, 254 pp, 「版权所有: 北京语言大学出版社有限公司，All Rights Reserved」, e-book listed as 在线阅读（**不可下载**）.
- **The 2025 syllabus is explicitly no-copy.** https://hsk.cn-bj.ufileos.com/3.0/新版HSK考试大纲1219.pdf — 406 pages, `pdfinfo`: **"Encrypted: yes (print:yes copy:no change:no addNotes:no algorithm:RC4)"** (I reproduced this independently).
- **Both rights-holders assert copyright and grant nothing commercial.** CTI's legal notice: 「本网所有内容，版权均属汉考国际或者内容提供者所有」 and reproduction is permitted only 「以新闻性或资料性公共免费信息为使用目的」 ("for news or informative public **free** information purposes"). CLEC's portal: 「版权所有 ©2014-2016 中外语言交流合作中心，未经许可不得转载」.
- **Every open-licensed list states its official derivation in its own README** — elkmovie: "Extracted from the official PDF"; ivankra: cites the MOE PDF + chinesetest.cn `WebNo` index; drkameleon's README Sources section names clem109 + elkmovie + CC-CEDICT. A third-party MIT/CC grant binds only that party and cannot convey rights it does not own.

**Balancing this honestly:** the *bare word + pinyin + level tuple* is close to 通用数表 / 单纯事实消息 territory (Art. 5(2)-(3)), and a bare leveled word list has thin originality. The real exposure is the **selection and arrangement** — which 11,092 words sit at which of 9 levels — which Art. 15 protects as a compilation. Chinese case law on government-issued 规范 data tables is thin; I found no controlling precedent. **Treat this as a low-to-moderate, uncompensated residual risk that you mitigate by (a) shipping attribution and license text, (b) not copying the official definitions or example sentences, and (c) optionally commissioning your own level assignments.** Getting a written license from CLEC/CTI or BLCUP is the only way to zero it out.

---

## 5. Recommended bundle (in priority order)

1. **Word → HSK level table:** `wzperson/hearmandarin-hsk-3-0-word-list` (**CC BY 4.0**, 11,147 words, 2025 syllabus bands 1-7 + HSK 2.0 alignment, **in-house glosses so no ShareAlike**). Fallback / cross-check: `ivankra/hsk30` (**MIT**, 11,092 rows, 2021 standard) and `drkameleon/complete-hsk-vocabulary` (**MIT**, all three word lists). Ship the license text + attribution line (CC BY 4.0 requires it: `HearMandarin (https://hearmandarin.com)`).
2. **Graded sentences (safest):** `harukicoder/hsk30-graded-readers` — **CC BY 4.0**, text authored by the licensor, license says "including commercially". 132 word-aligned texts.
3. **Sentences + audio:** `no7z/hsk-sentences-audio` — **CC BY-SA 4.0**, 4,354 HSK 3.0-graded sentences with pinyin/English/gloss and 8,708 synthetic MP3s. Accept that ShareAlike attaches to your **derived data bundle** (mere aggregation with the app binary does not relicense the app).
4. **Dictionary:** **CC-CEDICT** (CC BY-SA 4.0, 125,083 entries) straight from https://www.mdbg.net/chinese/dictionary?page=cc-cedict — not via a third-party mirror.
5. **Your own sentences:** take the **Tatoeba** cmn pool (89,065 sentences, CC BY 2.0 FR) and grade it yourself with `harukicoder/hsk30` (**MIT**, grades against 2.0 / 2021 / 2025 and reports which standard it used). This gives you unlimited commercially-clean graded sentences with attribution only. Read `license` per sentence from the API (no bulk license export exists).
6. **Attribution to ship:** a LICENSES/attribution screen covering CC BY 4.0 (HearMandarin, harukicoder), MIT (ivankra/elkmovie/drkameleon/harukicoder), and CC BY-SA 4.0 (no7z, CC-CEDICT) — plus the ShareAlike notice on whichever data bundle inherits it.

**Do not bundle:** the AllSet Grammar Wiki (NC), official MOE/CTI/CLEC PDF text, BLCUP textbook content, HSK Academy, Mandarin Companion, DuChinese, ChinesePod, Immersive Chinese, AnkiWeb decks, LDC data, HSK动态作文语料库, or CTRDG/Mendeley readability corpora.

---

## 6. Open items

- **CC-CEDICT license version:** mdbg.net says CC BY-SA **4.0**; cc-cedict.org/wiki says **3.0**. Confirm which the specific download carries.
- **hskhsk.com:** website footer says non-commercial; its GitHub repo is MIT. Resolve in writing with alan@hskhsk.com before relying on it (its "HSK Examples.txt" sentences are also partly taken from official HSK documentation).
- **Which "HSK 3.0":** decide whether your app targets the GF 0025-2021 standard or the 2025 exam syllabus (in force since 2026-07). They disagree on 41.5% of shared vocabulary, so this choice visibly changes level labels.
- **No Chinese CEFR mapping exists** in any open source found; CEFR-side coverage comes only from UD_Chinese-Beginner, which is NC.
