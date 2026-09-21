# License research: bundleable Mandarin graded-reading text

Research date: 2026-09-21 (all URLs fetched live; dates below are as observed from the source).
Target use: bundle simplified-Chinese graded reading text inside an offline AGPL-3.0 desktop/mobile app.
Disqualifying: NC, ND, research-only. Acceptable: CC BY, CC BY-SA (with attribution), public domain.

---

## Summary table

| Source | Content type | HSK-graded? | License | Redistribution verdict | URL |
|---|---|---|---|---|---|
| Tatoeba `cmn` exports | 89,065 sentences (mixed simp/trad), ~5,826 with audio | No (ungraded) | CC BY 2.0 FR default; 1 cmn sentence CC0 | **OK** (attribution required) | https://tatoeba.org/en/downloads |
| Tatoeba `sentences_CC0` (cmn) | 1 sentence | No | CC0 1.0 | OK but useless (1 sentence) | https://downloads.tatoeba.org/exports/per_language/cmn/cmn_sentences_CC0.tsv.bz2 |
| `harukicoder/hsk30-graded-readers` (HF) / `harukicoder/hsk30` (GH) | 102 graded readers (1,185 sentences) + 30 held-out (180 sents) | Yes, 6 shelves | **CC BY 4.0** (data); code MIT | **OK — best fit** | https://huggingface.co/datasets/harukicoder/hsk30-graded-readers |
| `no7z/hsk-sentences-audio` (HF) | 4,354 HSK-graded sentences + pinyin/gloss/translation; synthetic audio partial | Yes, HSK 1–6 | **CC BY-SA 4.0** (dataset); code MIT | **OK** (share-alike + attribution) | https://huggingface.co/datasets/no7z/hsk-sentences-audio |
| `bdx33/tatoeba-hsk-cmn-eng-fra` (HF) | 78,504 Tatoeba sentences w/ HSK level, simplified + EN + FR | Yes (derived) | CC BY 2.0 (upstream CC BY 2.0 FR) | **OK** (attribution; upstream terms apply) | https://huggingface.co/datasets/bdx33/tatoeba-hsk-cmn-eng-fra |
| `SHLEW06/chinese-hsk-adaptive-reader` (GH) | 300 original HSK readings (50/level), 520–1,625 chars each | Yes, HSK 1–6 | MIT (repo root); text license **not separately stated** | **Likely OK but ambiguous** | https://github.com/SHLEW06/chinese-hsk-adaptive-reader |
| CHILDES / TalkBank | Spoken child-language transcripts | No | **CC BY-NC-SA 3.0** (footer badge: 4.0) | **DISQUALIFIED (NC)** | https://childes.talkbank.org/ |
| "Read Chinese!" (NFLC/Univ. of Maryland) | 140 lessons × 2 scripts (60 novice, 69 interm., 11 cultural) | Level-graded (novice/intermediate) | **© 2006–2010, all rights reserved** | **DISQUALIFIED (no license)** | archived: https://web.archive.org/web/20160318002649/http://readchinese.nflc.org/ |
| "Chinese Reading World" (Univ. of Iowa) | Graded readings site | Yes (level-based) | **No license found; site dead** | **DISQUALIFIED (dead + unlicensed)** | domain parked/for sale |
| MandarinSpot | Annotation tool + dictionary lookup | HSK filter (tool feature) | **No license on site; ships no text** | N/A (no text to take) | https://mandarinspot.com/ |
| `26597925/Chinese-Annotator` (GH) | NLP labelling tool | No | Apache-2.0 (code only) | Code OK; **ships no reading text** | https://github.com/26597925/Chinese-Annotator |
| CC-CEDICT | 125,083-entry dictionary | HSK metadata no | **CC BY-SA 4.0** | OK (dict, not graded reading) | https://www.mdbg.net/chinese/dictionary?page=cc-cedict |
| "Chinese-Learning-Corpus" | — | — | — | **NOT FOUND** | — |
| `alexanderfrantsuzov/chinese-text-annotator` | — | — | — | **DOES NOT EXIST (404)** | — |

---

## 1. Tatoeba (Mandarin `cmn`)

**Content:** 89,065 `cmn` sentences (2026-09-19 weekly export; `cmn_sentences.tsv.bz2`, 1,257,035 bytes compressed; `cmn_sentences_detailed.tsv.bz2`, 1,722,240 bytes, includes per-sentence username). Median length 10 chars, p90 = 18, max 500. Script is mixed: 13,291 lines carry a simplified-only marker (们/说/国/学/这), 11,893 traditional-only (們/說/國/學/這), 63,877 neither (too short/ambiguous). The HF derivative `bdx33` reports **78,504 simplified** rows, which is a better estimate of the simplified subset. 5,826 cmn sentences have audio.

**License stated at the downloads page** (https://tatoeba.org/en/downloads), verbatim:
> "Creative commons — These files are released under CC BY 2.0 FR. A part of our sentences are also available under CC0 1.0."

**License stated in the Terms of Use §6.2** (https://tatoeba.org/en/terms_of_use), verbatim:
> "Tatoeba's technical infrastructure uses the default Creative Commons Attribution 2.0 France license (CC-BY 2.0 FR) for the use of textual sentences. The BY mention implies a single restriction on the use, reuse, modification and distribution of the sentence: a condition of attribution. That is, using, reusing, modifying and distributing the sentence is only allowed if the name of the author is cited."

**§6.5 (reuse), verbatim:**
> "You are responsible for your use, reuse, modification and dissemination of the content available on Tatoeba. Thus, if you circulate a sentence under license, it is your responsibility to circulate it with its license. For example, in the case of a CC-BY license, it is your responsibility to quote the author of the sentence."
> "We are not generally opposed to using our content for commercial purposes. However, this choice depends primarily on contributors. Certain phrases, in particular audio, may be contributed with a non-marketing condition, and therefore must not be marketed."
> "Certain sentences, including those in audio form, cannot be modified: those contributed with a condition of non-modification (for example. Creative Commons ND: No-Derivative) or a condition of sharing under the same conditions (for example the Creative Commons SA: Share-Alike)."

**CC0 subset is effectively empty for Chinese.** The per-language CC0 export contains exactly **one** sentence:
```
10597783	cmn	2022/2972 新年快乐！	2021-12-31 15:06:47
```
(`https://downloads.tatoeba.org/exports/per_language/cmn/cmn_sentences_CC0.tsv.bz2`, 102 bytes compressed, 1 line.)

**Attribution is workable for offline bundling.** The Tatoeba wiki FAQ gives a project-level attribution recipe (https://en.wiki.tatoeba.org/articles/show/faq), verbatim:
> "For the textual data — Basically you just need to write somewhere that some/all of your sentences are from Tatoeba, with a link to https://tatoeba.org, and mention that Tatoeba's data is released under CC-BY 2.0 FR."

Practical recommendation: an in-app credits/About screen plus a bundled `ATTRIBUTION.txt` containing that sentence, the CC BY 2.0 FR deed URL (https://creativecommons.org/licenses/by/2.0/fr/), the export date, and — to be safe beyond Tatoeba's own minimal recipe — the per-sentence usernames from `cmn_sentences_detailed.tsv.bz2`. All of this is offline-safe (no network needed at runtime), and CC BY 2.0 FR imposes no share-alike, so it does not interfere with AGPL-3.0 app code.

**Caveats / risks:**
- Audio is *not* uniformly CC BY: per-contributor licenses (some NC/ND). Do not bundle Tatoeba audio without checking each contributor; the FAQ says the same ("Our audio corpus has a wider range of licenses ... we recommend that you mention the username of each member whose audio you are reusing, as well as the license they chose").
- The public text exports carry **no per-sentence license column** (the detailed export's 6 columns are id, lang, text, username, date_added, date_modified), so a small number of sentences may in principle be under other licenses (CC0/SA/ND). Tatoeba's stated default for the bulk export is CC BY 2.0 FR. Treat this as a residual, low-but-nonzero risk.
- CC BY 2.0 FR is an older ported license (CC notes on the deed: "This is an older version of this license. Compared to previous versions, the 4.0 versions of all CC licenses are more user-friendly"; the 2.0 FR deed has no explicit sui-generis-database-rights clause, unlike 4.0).

---

## 2. `harukicoder/hsk30-graded-readers` — HF dataset + `harukicoder/hsk30` GitHub repo

**Best single fit found.** Content type: graded *passages* (not isolated sentences), word-aligned with pinyin and English gloss.

**Counts (verified by downloading and parsing the JSONL):**
- `hsk30_graded_readers.jsonl` — 614,659 bytes, **102 texts, 1,185 sentences, 14,426 chars**.
- Shelves: newbie 22, beginner 22, intermediate 22, upper 12, advanced 12, native 12.
- `hsk30_heldout.jsonl` — 119,709 bytes, **30 texts, 180 sentences** (disjoint by id).
- Schema per record: `id, shelf, shelf_index, title{hz,py,en}, description, text` (Simplified), `sentences[]{hz,en,words[]{hz,py,en}}`, `n_sentences, n_chars`.
- HF `lastModified`: 2026-09-01. No explicit HSK level labels are baked in by design; grade with the `hsk30` library.

**License:** dataset card front-matter `license: cc-by-4.0`. DATASHEET.md, verbatim:
> "**Version** 1.1 · **Released** 2026 · **Licence** CC BY 4.0 · **Size** 102 texts, 1,185 sentences, 8,682 word tokens, 14,417 graded characters, plus a disjoint 30-text held-out split"
> "Distributed with the `hsk30` repository and on HuggingFace under **CC BY 4.0** — reuse and adaptation permitted with attribution."

**CODE vs TEXT license distinction:** the GitHub repo `harukicoder/hsk30` `LICENSE` file is **MIT** ("MIT License / Copyright (c) 2026 Alvaro Serrano") and covers the *software*; the corpus is separately and explicitly CC BY 4.0 per the datasheet. Bundling the text requires CC BY 4.0 attribution, not the MIT notice.

**Provenance caveat (stated plainly by the author):**
> "The passages were **drafted with large-language-model assistance and then reviewed, edited, re-levelled and in several cases rewritten by the author.** They are pedagogical material written to a level target — not naturally occurring Chinese"
> "Named characters are invented. There is no personal data about real people."

This is a *quality/provenance* caveat, not a licensing one. Author: Alvaro Serrano (pinyora.com).

URLs: https://huggingface.co/datasets/harukicoder/hsk30-graded-readers · https://github.com/harukicoder/hsk30

---

## 3. `no7z/hsk-sentences-audio` — HF dataset

**Content:** 4,354 HSK-3.0-graded Chinese sentences with pinyin, per-word gloss, English translation, grammar tags, and synthetic audio. Verified from `data/train.parquet` (670,256 bytes): 4,354 rows; columns `id, hsk_level, topic, sentence_type, chinese, traditional, pinyin, pinyin_numbered, translation, tokens, grammar_points, grammar_tags, audio, audio_meta, audio_normal, audio_slow`. Level counts (verified): HSK1 281, HSK2 538, HSK3 727, HSK4 801, HSK5 965, HSK6 1,042. HF `lastModified` 2026-07-15; 700 downloads.

**License (README front-matter):** `license: cc-by-sa-4.0`. ATTRIBUTION.md, verbatim:
> "本项目的**代码**以 MIT 许可发布（见 `LICENSE`）。生成的**数据集**（`dist/`）由下列组件产生"
> "| CC-CEDICT | 词义、逐词标准拼音 | CC-BY-SA 4.0 | 需署名，衍生数据同以 CC-BY-SA 分享 |"
> "句子文本与英文翻译为本项目自有。" (sentence text and English translations are the project's own)
> "为简化合规，整个 `dist/` 数据集以 **CC-BY-SA 4.0** 发布。" (for compliance simplicity the whole `dist/` dataset is released under CC BY-SA 4.0)
> "音频由 CosyVoice2-0.5B 在本地合成，**合成语音**（synthetic voice）；输入文本为本项目自有内容。"

**Verdict: OK** — CC BY-SA 4.0 is explicitly allowed by the parent's constraints (share-alike with attribution). Note the practical consequence: the bundled sentence text becomes CC BY-SA 4.0, so any modified/extended version of *that text* must be shared alike. This does not affect AGPL-3.0 app code (separate works), but the parent should not promise permissive re-licensing of the text.

**Caveats:**
- No `LICENSE` file is present in the HF repo despite the ATTRIBUTION.md reference ("见 `LICENSE`"); the license is asserted in README/metadata only. Minor.
- **Audio in the HF repo is incomplete.** README claims "The complete export contains 8,708 MP3 files", but the repo currently holds **995 MP3s / 18.3 MB**, covering only HSK1 (281) and HSK2 (219 sentences × normal+slow). Treat audio availability as partial; text is complete.
- Provenance of the sentence text is self-declared ("自有") with no independent verification possible → mark the "own work" claim as **unverified**.

URL: https://huggingface.co/datasets/no7z/hsk-sentences-audio

---

## 4. `bdx33/tatoeba-hsk-cmn-eng-fra` — HF dataset (Tatoeba derivative, already simplified + HSK-tagged)

**Content:** 78,504 rows; simplified Chinese + English + French with HSK level. README verbatim:
> "- **Source:** https://tatoeba.org/downloads
> - **License:** [cc-by-2.0](https://tatoeba.org/about)
> - **Last update:** 2025-08-20
> - **Row count:** 78,504
> - **Language:** Simplified chinese, english and french"

HF front-matter `license: cc-by-2.0`; repo `lastModified` 2025-09-19; 158 downloads. Files: `train.jsonl`, `test.jsonl`.

**Verdict: OK**, but note the license is inherited from Tatoeba and the precise upstream license is **CC BY 2.0 FR**, not plain "CC BY 2.0" (the card cites https://tatoeba.org/about, which is not the license page). Attribution to Tatoeba + the 78,504 underlying sentence authors' usernames would still be required; this derivative does not ship the usernames, so for strict compliance prefer building from Tatoeba's own `cmn_sentences_detailed` export (which has usernames).

URL: https://huggingface.co/datasets/bdx33/tatoeba-hsk-cmn-eng-fra

---

## 5. `SHLEW06/chinese-hsk-adaptive-reader` — GitHub

**Content shipped:** YES — bundled graded reading library, verified by tarball:
- `src/data/library/hsk1.json` … `hsk6.json` — **50 readings per level, 300 total**, e.g. HSK1 520–529 chars, HSK2 650–667, HSK3 850–871, HSK4 1,050–1,080, HSK5 1,300–1,327, HSK6 1,600–1,625.
- Each record carries `textZh`, `translationEn`, `paragraphTranslations`, `sentenceExplanations`, grammar, `targetWords`/`coveredHskWords`, comprehension questions; records are tagged `"sourceType": "original"`.
- Plus `src/data/library/corpus-summary.json`, `starterContent.ts`, `placementQuestions.ts`, `mockExams.ts`, HSK1–6 glossaries, and a pinned CC-CEDICT snapshot (`data-pins/cedict-2026-07-13.txt.gz`).

**License:** root `LICENSE` = **MIT** ("MIT License / Copyright (c) 2026 Shunji Lewandowski") — this is the CODE license. `THIRD_PARTY_NOTICES.md` names only two third-party inputs:
> "## CC-CEDICT ... License: Creative Commons Attribution-ShareAlike 4.0 International (CC BY-SA 4.0) ... The generated dictionary data is a derivative work distributed under CC BY-SA 4.0."
> "## complete-hsk-vocabulary ... MIT License"

The 300 readings themselves are **not separately licensed anywhere**, and are not listed as third-party. So the text is either (a) covered by the repo-root MIT license, or (b) unlicensed/ambiguous. **Mark as "MIT by repo root; text license not explicitly restated — verify with the author before shipping."** MIT is permissive enough for bundling *if* it applies, and the project declares the content original, so risk is low but not zero.

Also note the repo's own QA note: "This is a substantial first-pass static corpus. It is mostly original/reflective ... It should be run through your real `npm run hsk:coverage` dictionary pipeline before production."

URL: https://github.com/SHLEW06/chinese-hsk-adaptive-reader (MIT; 0 stars; pushed 2026-07-27)

---

## 6. CHILDES / TalkBank — **DISQUALIFIED (non-commercial)**

CHILDES/TalkBank is the child-language database, **not** a graded reading corpus. Its Chinese index (https://childes.talkbank.org/access/Chinese/) lists only spoken child-language corpora (Cantonese: HKU-70, Lee/Wong/Leung, MOST, PaidoCantonese; Mandarin: AcadLang, BJCMC, Chang1/2, Erbaugh, LiReading, LiZhou, TCCM, Tong, Zhou*, etc.). There is **no "Read Chinese!" corpus** in CHILDES.

Ground Rules (https://talkbank.org/0share/rules.html), verbatim:
> "**Copyright:** Except where otherwise indicated, the use of TalkBank data is governed by the Creative Commons CC BY-NC-SA 3.0 copyright license. **This license precludes the incorporation of the data in commercial products**, including systems such as large language models (LLMs) such as ChatGPT. Commercial enterprises that have been allowed access to password-protected data can use the data for the development of algorithms, such as for automatic speech recognition or clinical assessment, but the data themselves cannot be included in models. These materials are intended to be used by researchers, teachers, and clinicians for professional and non-commercial use."

Also (same page):
> "**Web Processing:** Users cannot upload TalkBank data to web-based systems unless those systems include assurance that they will not keep the data."
> "To maintain the NIH Certificate of Confidentiality (CoC), downloaded identificable data can only be stored on a local device for analysis and cannot be circulated further."

The CHILDES site footer additionally displays a **CC BY-NC-SA 4.0** badge linking to https://creativecommons.org/licenses/by-nc-sa/4.0/deed.en (version mismatch with the rules page's 3.0 — both are NC). Access levels (https://childes.talkbank.org/access.html): "Registration required: CHILDES transcript and media data are open to all, but users need to sign in and register."

**Verdict: NC ⇒ DISQUALIFYING for a bundled app.** Do not bundle.

---

## 7. "Read Chinese!" — **DISQUALIFIED (all rights reserved)**

Correction to the brief: this is **not** Yale and **not** CHILDES. It was a project of the **National Foreign Language Center (NFLC), University of Maryland**, developed with University of Iowa and University of Hawaii content developers, under U.S. Dept. of Education grants P017A060025 and P017A090366. Site was `readchinese.nflc.org` — **now dead** (DNS does not resolve; HTTP 000).

**Size:** Novice 60 + Intermediate 69 + Cultural 11 = **140 lessons**, available in both Simplified and Traditional (≈280 texts).

**License: none.** Footer, verbatim (archived 2016-03-18):
> "READ CHINESE! is a project of the National Foreign Language Center at the University of Maryland, developed under grants (Nos. P017A060025 and P017A090366) from the International Research and Studies Program of the United States Department of Education.
> Copyright 2006-2010  ♦   webmaster@nflc.org"

No CC license, no terms of use, no permission grant anywhere on the archived site (checked Home, Credits, Downloads, Guide, System Requirements pages). Content includes adaptations of newspaper extracts, so third-party rights likely sit on top of NFLC's copyright.

**Verdict: no license found + explicit © notice ⇒ cannot redistribute.** Only route is written permission from NFLC/UMD.

URLs: https://web.archive.org/web/20160318002649/http://readchinese.nflc.org/ (archived index); credits: https://web.archive.org/web/20160318002649/http://readchinese.nflc.org/?page=credits_sheet

---

## 8. "Chinese Reading World" (University of Iowa) — **DISQUALIFIED (dead site, no license)**

- `chinesereadingworld.org` was a frameset pointing at `http://chinese-readings.its.uiowa.edu/` (University of Iowa ITS). That target URL is **not archived** by the Wayback Machine.
- The Wayback CDX history shows the frameset through 2009; from 2010 onward the domain served ~343-byte parking pages, and 2014+ snapshots are GoDaddy/Forsale pages.
- **Live status (2026-09-21):** `https://chinesereadingworld.org/` returns a 114-byte JS redirect to `/lander`, which returns **"Access Denied ... forsale.godaddy.com/forsale/chinesereadingworld.org"**. The domain is **parked for sale**.
- `http://www.uiowa.edu/~chnread/` → 404 on uiowa.edu.

**Verdict: no license found; content is offline and the domain is for sale. Unusable.** (Even the historical content had no visible open license; only a "Review of a Website — Chinese Reading World" in a journal, and Iowa Libraries' general appropriate-use policy.)

---

## 9. MandarinSpot — no text license, no text shipped

`https://mandarinspot.com/` is an annotation/dictionary *tool*: "Add pinyin pronunciation, use our dictionary, and annotate any Chinese text." It publishes **no downloadable corpus**. `/terms`, `/about`, `/contribute` were probed: all return the same SPA shell with **no license, copyright, or terms text at all** (no matches for licen[cs]e / copyright / CC BY / creative commons / redistribution). Its GitHub presence is not evident from the site.

Its two data dependencies are separately licensed and are the only reusable text assets in its stack — both stated on the site's Acknowledgments:
> "This site uses the CC-CEDICT dictionary maintained and made available by MDBG Chinese-English dictionary. The version used on this site contains over 100,000 entries."
> "The HSK vocabulary list used by the annotator was taken from HSK Flashcards website."

CC-CEDICT's own license page (https://www.mdbg.net/chinese/dictionary?page=cc-cedict), verbatim:
> "This work is licensed under a Creative Commons Attribution-ShareAlike 4.0 International License ... It more or less means that you are allowed to use this data for both non-commercial and commercial purposes provided that you: mention where you got the data from (attribution) and that in case you improve / add to the data you will share these changes under the same license (share alike). Latest release: 2026-09-20 08:35:24 GMT Number of entries: 125083"

**Verdict: MandarinSpot itself — no license found, and no text to bundle (tool only). Its CC-CEDICT dependency is CC BY-SA 4.0 and is a dictionary, not graded reading.**

---

## 10. `chinese-text-annotator` repos — the named repo does not exist

- `https://github.com/alexanderfrantsuzov` → **HTTP 404**; `https://github.com/alexanderfrantsuzov/chinese-text-annotator` → **HTTP 404**. The user account does not exist. **Nothing to license.**
- GitHub search `chinese-text-annotator in:name` → **0 results**; `chinese_text_annotator in:name` → 0; `text-annotator chinese in:name` → 0.
- The generic query `chinese-text-annotator` (any field) returns only loosely-matching repos — none ships graded reading text:
  - `fendaq/Chinese-Annotator` (0★, 2017-11-09, no license)
  - `smithyworks/chinese-annotator` (0★, 2018-04-27, no license)
  - `cjw322/chn_translator` (0★, 2024-09-24, no license)
  - `edwardstopher/pinyinreader` (0★, 2026-03-25, **GPL-3.0**) — tarball contains only `index.html`, `LICENSE`, `README.md`, robots/sitemap; **no bundled text data**.
  - `matturche/pinyin_annotator` (0★, 2026-03-14, **MIT**, "annotates chinese texts with pinyin, allows to only annotates unknown words based on HSK level") — tarball ships **only** `dist/data/cedict_ts.u8` and `public/data/cedict_ts.u8` as text data, i.e. a CC-CEDICT copy (CC BY-SA 4.0), **no reading passages**.
  - `uranbekanarbaev/chinese-to-pinyin` (0★, MIT), `imjagpul/pinyin-tools` (0★), `dolly17107/chinese-pronunciation` (4★), `qlHee/SEcourse_iAnctChinese` (10★, 2026-06-25, no license).

**The canonical "Chinese-Annotator" (the NLP labelling tool) — `26597925/Chinese-Annotator`** (fork of `crownpku/Chinese-Annotator`; the upstream `crownpku/Chinese-Annotator` now returns **404**, i.e. deleted/renamed):
- README: "Annotator for Chinese Text Corpus ... Many NLP tasks require lots of labelling data. Current annotators are mostly for English."
- `master/LICENSE` = **Apache License 2.0**. Repo structure: `chi_annotator/{algo_factory,preprocess,online}`, `docs`, `tests/data` ("Raw data for tests").
- **No bundled reading corpus** — it is an annotation tool; only test fixtures. Code license Apache-2.0; **no separate text/data license, and no text worth taking.**

---

## 11. Other HSK-graded / beginner-graded corpora and vocabulary datasets found

| Repo / dataset | What it is | License | Verdict |
|---|---|---|---|
| `Tiagodfs/hsk-3.0-dataset` (HF) | HSK 3.0 **vocabulary** list, single `hsk.csv` (id, level, chinese, pinyin, english) | **CC0-1.0** (`license: cc0-1.0`) | Public domain; vocabulary only, no passages. 2,053 downloads; lastMod 2026-03-11 |
| `willfliaw/hsk-dataset` (HF) | HSK **word list** CSV with pinyin/english/pos/tts_url | **CC BY 4.0** | OK; vocabulary only. lastMod 2025-08-31 |
| `infinite-dataset-hub/Hsk2Corpus`, `infinite-dataset-hub/HSK_Level1_Words` (HF) | HSK word lists | MIT | OK; vocabulary only |
| `clem109/hsk-vocabulary` (GH) | HSK vocabulary JSON; README shows an `examples[]` schema with zh/pinyin/en | **MIT** (repo LICENSE, © 2018 Clement Venard); source is `gigacool/hanyu-shuiping-kaoshi` (also **MIT**, © 2018 gigacool) | MIT applied to a word list. Example sentences are crowd-contributed PRs; README still lists "Add example sentences" as an open goal and the sentence data is thin. 54★, last push 2019-10-29 |
| `NewHSK3/new-hsk-3-anki-deck` (GH) | "33,000 example sentences and Mandarin audio" | **No license** | **DISQUALIFIED (no license)**. 4★, 2026-07-31 |
| `lm742611149/learn-chinese` (GH) | 315 original HSK 1–6 readings, 5,024 words, audio+quizzes | **No license file** | **DISQUALIFIED (no license)** despite "Free" branding. README: "Every text is original." 0★, 2026-09-15 |
| `Pleometric/HSK-deck`, `alfredmastan/mandarin-graded-readers`, `joey-kilgore/chinese-graded-readers`, `meijer1973/chinese-anki-graded-readers`, `gronnmann/HanTales` | Generators that produce HSK decks/readers via LLM/TTS | Mostly **no license**; `HanTales` GPL-3.0, `lexweave` Apache-2.0 | Code-only; no pre-built licensed text corpus bundled |
| `ymcui/Chinese-Cloze-RC` (GH) | "Chinese Cloze-style RC Dataset: People's Daily & Children's Fairy Tale (CFT)" | **CC-BY-SA-4.0** | Share-alike acceptable **but provenance is risky**: People's Daily is a copyrighted newspaper. Treat the People's Daily portion as **not safely redistributable**; the "Children's Fairy Tale" portion's provenance is unverified. 173★, last push 2019-03-26 |
| CC-CEDICT | 125,083-entry zh→en dictionary | **CC BY-SA 4.0** | OK; dictionary, not graded reading. Needed if you want per-word glosses |
| `drkameleon/complete-hsk-vocabulary` | HSK 3.0 machine-readable word list | **MIT** | OK; vocabulary only (used by no7z for grading, per its ATTRIBUTION.md) |

Not found: any repository or dataset actually named **"Chinese-Learning-Corpus"** on GitHub or Hugging Face (GitHub `in:name` → 2 unrelated hits; HF search → 0; the GitHub org `Chinese-Learning-Corpus` → 404). **Mark as "not found / unverifiable".**

---

## Bottom line for the parent

1. **Recommended bundle (no share-alike):** `harukicoder/hsk30-graded-readers` — 102 CC BY 4.0 passages (1,185 sentences), Simplified Chinese, word-aligned pinyin/gloss, 6 difficulty shelves, 615 KB. Attribution: "HSK 3.0 Graded Reader Corpus, Alvaro Serrano, CC BY 4.0". Note the code is MIT but the *text* is CC BY 4.0 — carry the CC BY notice, not the MIT one.
2. **Recommended bundle (volume, with share-alike):** `no7z/hsk-sentences-audio` — 4,354 HSK 1–6 sentences under CC BY-SA 4.0 (670 KB parquet; audio only partially present, 995 MP3s). Share-alike on the text is acceptable per the stated constraints.
3. **Largest pool, attribution-heavy:** Tatoeba `cmn` — 89,065 sentences, CC BY 2.0 FR (only 1 CC0 sentence). Offline bundling is fine with an in-app credits screen + CC BY 2.0 FR notice; include per-sentence usernames for strict compliance. Do not bundle Tatoeba audio.
4. **Do not use:** CHILDES/TalkBank (CC BY-NC-SA — NC), "Read Chinese!" (© all rights reserved), "Chinese Reading World" (dead domain, no license), `lm742611149/learn-chinese` and `NewHSK3/new-hsk-3-anki-deck` (no license), `ymcui/Chinese-Cloze-RC` People's Daily portion (third-party copyright).
5. **Nothing exists** for `alexanderfrantsuzov/chinese-text-annotator` (404) or any repo named `chinese-text-annotator` (0 search results). "Chinese-Learning-Corpus" was not found under that name.
