# Chinese (zh/cmn) sentence corpora — commercial offline-bundling license audit

**Date:** 2026-09-21 · **Scope:** OPUS and upstream parallel corpora containing Chinese; suitability for shipping data inside a closed-source commercial desktop/mobile app.

## 0. Two meta-facts that change how to read every OPUS page

**(a) OPUS grants nothing and says so.** Every legacy corpus page carries:

> "Disclaimer — We do not own any of the text from which the data has been extracted. We only offer files that we believe we are free to redistribute. If any doubt occurs about the legality of any of our file downloads we will take them off right away after contacting us."
> — https://opus.nlpl.eu/legacy/WikiMatrix.php (footer of every `/legacy/*.php` corpus page)

**(b) OPUS publishes its own machine-readable license table.** `info/RELEASE_LICENSES.tsv` in the OPUS repo is the authoritative statement of what OPUS claims per release; **132 of 1,263 releases are literally `unknown`** (i.e. OPUS asserts no license at all, which is *not* a permission):
https://github.com/Helsinki-NLP/OPUS/blob/master/info/RELEASE_LICENSES.tsv

**URL pattern warning:** the old `https://opus.nlpl.eu/<Corpus>/` URLs now 404. Use `https://opus.nlpl.eu/datasets/<Corpus>` (JS-rendered) or `https://opus.nlpl.eu/legacy/<Corpus>.php` (server-rendered, the only one with the Disclaimer footer). Sizes below are from the OPUS API: `https://opus.nlpl.eu/opusapi/?corpus=<C>&source=en&target=<zhcode>&preprocessing=moses`.

---

## 1. Master table

`zh size` = sentence pairs involving Chinese in the **latest** OPUS release (API-verified 2026-09-21). "OPUS says" = license string in `RELEASE_LICENSES.tsv` / on the dataset page.

| Corpus (OPUS latest) | zh content (OPUS API) | OPUS says | Upstream license actually says | Commercial bundled redistribution? | Evidence |
|---|---|---|---|---|---|
| **Tatoeba** v2026-07-08 | `cmn–en` **49,851**; also yue 5,978, wuu 1,104, lzh 545, nan 153 | CC BY 2.0 FR | CC BY 2.0 FR (default); "A part of our sentences are also available under CC0 1.0"; per-sentence licenses vary | ✅ **OK with attribution** (see gotcha §2.1) | [OPUS](https://opus.nlpl.eu/datasets/Tatoeba) · [tatoeba.org/en/downloads](https://tatoeba.org/en/downloads) · [terms](https://tatoeba.org/en/terms_of_use) |
| **WikiMatrix** v1 | `en–zh` **786,512** (110 zh dirs) | **CC-BY-SA 4.0** | No license stated by FAIR for the *data* (LASER **software** is BSD); source text is Wikipedia → CC BY-SA 4.0 + GFDL | ⚠️ **OK with ShareAlike obligations** | [OPUS](https://opus.nlpl.eu/datasets/WikiMatrix) · [LASER LICENSE (BSD, software only)](https://raw.githubusercontent.com/facebookresearch/LASER/main/LICENSE) · [Wikipedia:Copyrights](https://en.wikipedia.org/wiki/Wikipedia:Copyrights) |
| **CCMatrix** v1 | `en–zh` **71,383,325** (90 zh dirs) | **unknown** | FAIR states **no data license** (only "we open-source our scripts"); text mined from Common Crawl / CCNet | ❌ **NOT OK (license unknown + third-party web text)** | [OPUS](https://opus.nlpl.eu/datasets/CCMatrix) · [LASER CCMatrix README](https://raw.githubusercontent.com/facebookresearch/LASER/main/tasks/CCMatrix/README.md) · [Common Crawl ToU](https://commoncrawl.org/terms-of-use) |
| **NLLB** v1 (OPUS) | `en–zh` **71,383,325** (byte-identical to CCMatrix en–zh) | **ODC-By** | OPUS repackaging of Meta's mined-bitext metadata via AllenAI's HF package; OpenAIRE/AI2 card says ODC-BY + "you are also bound to the respective Terms of Use and License of the original source" | ⚠️ **OK with attribution for the database; underlying web text uncleared** | [OPUS](https://opus.nlpl.eu/datasets/NLLB) · [ODC-By 1.0](https://opendatacommons.org/licenses/by/1-0/) |
| **OpenSubtitles** v2024 | `en–zh_CN` **22,394,812**; `en–zh_TW` **18,583,480**, `zh_ze` 543,966. (**v2018 contains NO Chinese at all**; v2016 `en–zh` 9,304,777) | **unknown** | OpenSubtitles ToS: *"we only offer files that we believe we are free to redistribute… **Commercial use prohibited.**"* | ❌ **NOT OK** | [OPUS](https://opus.nlpl.eu/datasets/OpenSubtitles) · [ToS](https://opensubtitles.tawk.help/article/terms-of-service) (live copy of opensubtitles.com ToS) |
| **TED2020** v1 | `zh_tw` **404,726** + `zh_cn` **402,919** | "Please respect the TED Talks Usage Policy" | TED Talks Usage Policy: **CC BY–NC–ND 4.0 International**; *"you cannot use TED Talks in any commercial context… in an app of any kind"*; ND = no derivatives | ❌ **NOT OK (NC + ND)** | [OPUS](https://opus.nlpl.eu/datasets/TED2020) · [TED policy](https://www.ted.com/about/our-organization/our-policies-terms/ted-talks-usage-policy) |
| **TED2013** v1.1 | `en–zh` **154,579** | **unknown** | same TED CC BY-NC-ND 4.0 policy | ❌ **NOT OK** | [OPUS](https://opus.nlpl.eu/datasets/TED2013) |
| **NeuLab-TedTalks** v1 | `zh_tw` **218,034** + `zh_cn` **215,340** | **unknown** (Copyright field = TED policy) | TED volunteer subtitles → CC BY-NC-ND; upstream tarball dead; no LICENSE in source repo | ❌ **NOT OK** | [OPUS](https://opus.nlpl.eu/datasets/NeuLab-TedTalks) |
| **News-Commentary** v16 | `en–zh` **125,996** (14 zh dirs) | **unknown** | Distribution README: *"The data is released on the same terms as the ParaCrawl data… We license the actual packaging of this parallel data under the Creative Commons CC0 license ("no rights reserved")… We do not own any of the text"* | ⚠️ **OK on packaging (CC0); underlying news text is third-party copyrighted → take-down risk** | [OPUS](https://opus.nlpl.eu/datasets/News-Commentary) · [README](https://data.statmt.org/news-commentary/README) |
| **MultiUN** v1 | `en–zh` **9,564,315** (6 zh dirs) | **unknown** | No license file anywhere; compiler papers state the UN documents are public domain per UN ST/AI/189/Add.9/Rev.2 | ⚠️ **OK with attribution** (no explicit grant from compiler) | [OPUS](https://opus.nlpl.eu/datasets/MultiUN) · [Eisele & Chen LREC 2010](https://aclanthology.org/L10-1052/) |
| **UNPC** v1.0 | `en–zh` **17,451,549** (OPUS re-alignment); official UN stats: `en–zh` 15,886,041 lines | **unknown** | *"composed of official records and other parliamentary documents of the United Nations that are **in the public domain**"*; *"the user must acknowledge the United Nations as the source"*; *"…shall be respected with regard to the Corpus (**no other restrictions apply**)"* | ✅ **OK with attribution (cleanest large source)** | [OPUS](https://opus.nlpl.eu/datasets/UNPC) · [un.org/dgacm/en/content/uncorpus](https://www.un.org/dgacm/en/content/uncorpus) |
| **QED** v2.0a | `*–zh` **13,123** (106 zh dirs) | *"The QED Corpus is made public for RESEARCH purpose only."* + "Copyright Qatar Computing Research Institute. All rights reserved." | same | ❌ **NOT OK (research only)** | [OPUS](https://opus.nlpl.eu/datasets/QED) · [QCRI QED](https://alt.qcri.org/resources/qedcorpus/) |
| **GNOME** v1 | `zh_CN` 85,698; `zh_TW` 54,669; `zh_HK` 54,599. **But `en–zh_CN` is only 78** (the big pairings are with de/fr/etc.) | **unknown** | Per-module: GNOME legal handbook — GPL-2/3, LGPL-2, or CC-BY-SA, *"Each module's license should be included in its COPYING file"*; `.po` headers say *"This file is distributed under the same license as the gnome-shell package."* | ❌ **NOT OK (per-module copyleft, provenance not preserved)** | [OPUS](https://opus.nlpl.eu/datasets/GNOME) · [handbook.gnome.org/development/legal.html](https://handbook.gnome.org/development/legal.html) |
| **KDE4** v2 | `zh_TW` **142,472**; `zh_CN` **139,983**; `zh_HK` 11,331 | **unknown** | KDE Licensing Policy §7: translations of section-4 files must be LGPL-2.1/3.0; section-5 files GPL-2/3. **KDE `.po` files carry no license header at all.** | ❌ **NOT OK (copyleft; no per-file license metadata)** | [OPUS](https://opus.nlpl.eu/datasets/KDE4) · [community.kde.org/Policies/Licensing_Policy](https://community.kde.org/Policies/Licensing_Policy) |
| **Ubuntu** v14.10 | zh files exist (`en–zh`, `zh_CN/HK/TW`) but OPUS reports **no alignment count** (blank) | **unknown** | Launchpad: *"Translations of individual strings are made available… under the BSD license (revised)… Translations of groups of strings… are made available… under the licence applicable to the project"* | ❌ **NOT OK as a whole** (commingled BSD-original + GPL/LGPL project strings) | [OPUS](https://opus.nlpl.eu/datasets/Ubuntu) · [Launchpad policies](https://ubuntu.com/docs/launchpad/user/reference/launchpad-and-community/legal/launchpad-policies/) |
| **Mozilla-I10n** v1 | `en–zh_CN` **107,657**; `en–zh_TW` 81,464; `zh_HK` 1,159; `yue` 149 | **Mozilla Public License 2.0** | Upstream `mozilla-l10n/firefox-l10n` and `mozilla-l10n/mt-training-data` both ship MPL 2.0 | ⚠️ **OK with MPL-2.0 obligations** (file-level copyleft: keep notices, make MPL-covered files' source available) | [OPUS](https://opus.nlpl.eu/datasets/Mozilla-I10n) · [LICENSE](https://raw.githubusercontent.com/mozilla-l10n/mt-training-data/main/LICENSE) |
| **GlobalVoices** v2018q4 | `zht` **144,224** | **unknown** | globalvoices.org: *"This site is licensed as Creative Commons Attribution 3.0"* — the **same in the 2018-11-02 Wayback snapshot**, i.e. the common "it was CC BY-SA" claim is **unsupported** | ✅ **OK with attribution** (no author metadata exported by OPUS) | [OPUS](https://opus.nlpl.eu/datasets/GlobalVoices) · [attribution policy](https://globalvoices.org/about/global-voices-attribution-policy/) |
| **wikimedia** v20260327 | `zh` 471,224 (+ yue/nan/wuu/lzh) | **CC-BY-SA 4.0** | Wikimedia ContentTranslation dumps → Wikipedia text, CC BY-SA 4.0 + GFDL | ⚠️ **OK with ShareAlike obligations** | [OPUS](https://opus.nlpl.eu/datasets/wikimedia) · [Wikipedia:Copyrights](https://en.wikipedia.org/wiki/Wikipedia:Copyrights) |
| **Wikipedia** v1.0 | **no Chinese** (36 pairs, all European) | **unknown** | Wołk & Marasek Wikipedia extraction; Wikipedia text CC BY-SA 4.0 + GFDL | n/a (no zh) | [OPUS](https://opus.nlpl.eu/datasets/Wikipedia) |
| **WikiTitles** v3 / **LinguaTools-WikiTitles** v2014 | `en–zh` **921,959** / **6,664,332** | **CC-BY-SA 4.0** / **CC-BY-SA** (unversioned) | Wikipedia article titles/text → CC BY-SA 4.0 + GFDL | ⚠️ **OK with ShareAlike obligations** | [OPUS WikiTitles](https://opus.nlpl.eu/datasets/WikiTitles) · [OPUS LinguaTools](https://opus.nlpl.eu/datasets/LinguaTools-WikiTitles) |
| **XLEnt** v1.2 | `zh` 6,292,330 (+ wuu 20,295) | **unknown** | statmt XLEnt: no license, only *"No claims of intellectual property are made on the work of preparation of the corpus"* (a disclaimer, not a grant) | ❌ **NOT OK (license unknown; components CCAligned+CCMatrix unlicensed)** | [OPUS](https://opus.nlpl.eu/datasets/XLEnt) · [data.statmt.org/xlent](http://data.statmt.org/xlent/) |
| **ParaCrawl** v9 (+Bonus) | `en–zh` **14,170,869** | **unknown** | ParaCrawl: *"We do not own any of the text… We license the actual packaging of these parallel data under the Creative Commons CC0 license ("no rights reserved")"* | ⚠️ **OK on packaging (CC0); web text uncleared → take-down risk** | [OPUS](https://opus.nlpl.eu/datasets/ParaCrawl) · [paracrawl.eu](https://paracrawl.eu/) |
| **CCAligned** v1 / **MultiCCAligned** v1.1 | `zh_CN` **15,181,417** / **15,181,416**; `zh_TW` 8,778,973 / 8,778,972 | **unknown** | statmt CCAligned: only *"No claims of intellectual property are made on the work of preparation of the corpus"* — no license | ❌ **NOT OK** | [OPUS CCAligned](https://opus.nlpl.eu/datasets/CCAligned) · [statmt.org/cc-aligned](https://www.statmt.org/cc-aligned/) |
| **HPLT** v1.1 / **MultiHPLT** v2 | `zh_hant` **5,306,624** | **unknown** | HPLT: *"We license the actual packaging of these text data under the Creative Commons CC0 license ("no rights reserved")"* + *"It is your responsibility that any use of the data complies with any applicable legal framework, such as… the EU Copyright Directive 2019/790"* | ⚠️ **OK on packaging (CC0); web text uncleared** | [OPUS](https://opus.nlpl.eu/datasets/HPLT) · [hplt-project.org/datasets](https://hplt-project.org/datasets) |
| **MultiParaCrawl** v9b | `zh` **2,090,780** | **unknown** | ParaCrawl-derived → CC0 packaging | ⚠️ **OK on packaging (CC0); web text uncleared** | [OPUS](https://opus.nlpl.eu/datasets/MultiParaCrawl) · [paracrawl.eu](https://paracrawl.eu/) |
| **Tanzil** v1 | `*–zh` **187,092** | **unknown** | *"The translations provided at this page are **for non-commercial purposes only**… Redistributing the following list in another website is not allowed, unless direct permission is granted"* | ❌ **NOT OK (NC)** | [OPUS](https://opus.nlpl.eu/datasets/Tanzil) · [tanzil.net/trans](https://tanzil.net/trans/) (note: `tanzil.net/docs/terms` is a dead wiki page) |
| **bible-uedin** v1 | `*–zh` **124,378** | **CC0 1.0** | `christos-c/bible-corpus` LICENSE = CC0 1.0 | ✅ **OK for commercial bundled app** | [OPUS](https://opus.nlpl.eu/datasets/bible-uedin) · [github.com/christos-c/bible-corpus](https://github.com/christos-c/bible-corpus) |
| **tldr-pages** v2026-07-07 | `*–zh` **17,514** | **CC-BY-4.0** | `LICENSE.md`: *"This work is licensed under the Creative Commons Attribution 4.0 International License (CC-BY)"* | ✅ **OK with attribution** | [OPUS](https://opus.nlpl.eu/datasets/tldr-pages) · [LICENSE.md](https://github.com/tldr-pages/tldr/blob/main/LICENSE.md) |
| **PHP** v1 | `en–zh` **41,706** | **unknown** | php.net: *"This material may be distributed only subject to the terms and conditions set forth in the Creative Commons Attribution 3.0 License or later"* | ✅ **OK with attribution** | [OPUS](https://opus.nlpl.eu/datasets/PHP) · [php.net/manual/en/copyright.php](https://www.php.net/manual/en/copyright.php) |
| **MDN_Web_Docs** v2023-09-25 | `en–zh_CN` **66,641**, `en–zh_TW` 7,142 | **CC-BY-SA 2.5** | MDN content is CC BY-SA 2.5 | ⚠️ **OK with ShareAlike obligations** | [OPUS](https://opus.nlpl.eu/datasets/MDN_Web_Docs) |
| **OpenOffice** v3 | `zh_CN` **69,399** | **unknown** | Legacy OpenOffice.org: LGPLv3 + Public Document License, "in some cases… Attribution-NoDerivs 2.5"; per-page provenance not preserved | ❌ **NOT OK (unpreserved per-page legacy licenses incl. ND)** | [OPUS](https://opus.nlpl.eu/datasets/OpenOffice) · [openoffice.org/license.html](https://www.openoffice.org/license.html) |
| **translatewiki** v2026-07-01 | `zh` 177 (+ `zh_hk` 5,304, `lzh` 3,138) | **CC BY 3.0** | translatewiki: *"Translations by translators are licensed CC BY 3.0"*; user pages ARR | ✅ **OK with attribution** (tiny zh slice) | [OPUS](https://opus.nlpl.eu/datasets/translatewiki) · [Project:About](https://translatewiki.net/wiki/Project:About) |
| **WMT-News** v2019 | `en–zh` **19,965** | **unknown** | WMT19: *"The data released for the WMT19 news translation task can be **freely used for research purposes**… For other uses of the data, you should consult with original owners"* | ❌ **NOT OK (research only)** | [OPUS](https://opus.nlpl.eu/datasets/WMT-News) · [wmt19 task page](http://www.statmt.org/wmt19/translation-task.html) |
| **KDEdoc** v1 | `zh_TW` 190 | **unknown** | KDE docs — CC-BY-SA/GPL per module | ❌ **NOT OK** (same as KDE4) | [OPUS](https://opus.nlpl.eu/datasets/KDEdoc) |
| **JParaCrawl** v3.0 | `ja–zh` **83,893** | **unknown** ("Terms of use") | NTT JParaCrawl terms; research-oriented | ❌ **NOT OK (terms-of-use / unclear)** | [OPUS](https://opus.nlpl.eu/datasets/JParaCrawl) |
| **Books** v1 | **no Chinese** | **unknown** | Farkas bilingual books: *"All texts are freely available for personal, educational and research use. **Commercial use… not granted.**"* | ❌ **NOT OK** — and no zh | [OPUS](https://opus.nlpl.eu/datasets/Books) |
| **EuroPat** v1–v3 | **no Chinese** | Creative Commons CC0 | europat.net: CC0 packaging + "contains data sourced from EPO databases, © European Patent Organisation" | n/a (no zh) | [OPUS](https://opus.nlpl.eu/datasets/EuroPat) · [europat.net](https://europat.net/) |
| **FLORES-200 / FLORES-101** (not in OPUS) | `zho_Hans`, `zho_Hant`, `yue_Hant` — **dev/devtest/test only, not training data** | — | **CC-BY-SA 4.0** (`facebookresearch/flores` README: "* FLORES-200: CC-BY-SA 4.0") | ⚠️ **OK with ShareAlike obligations but eval-only** | [flores README](https://raw.githubusercontent.com/facebookresearch/flores/main/README.md) · [HF card](https://huggingface.co/datasets/facebook/flores) |

Legend: ✅ OK · ⚠️ OK with obligations/caveats · ❌ NOT OK.

---

## 2. Per-corpus notes, verbatim clauses and gotchas

### 2.1 Tatoeba (OPUS subsample) — ✅ OK with attribution
OPUS page states `CC BY 2.0 FR` and `Copyright: see https://tatoeba.org/eng/terms_of_use`. Upstream terms:
> "Tatoeba's technical infrastructure uses the default Creative Commons Attribution 2.0 France license (CC-BY 2.0 FR) for the use of textual sentences. The BY mention implies a single restriction on the use, reuse, modification and distribution of the sentence: a condition of attribution."
> "We are not generally opposed to using our content for commercial purposes. However, this choice depends primarily on contributors. Certain phrases, in particular audio, may be contributed with a non-marketing condition, and therefore must not be marketed."
> — https://tatoeba.org/en/terms_of_use

Downloads page:
> "These files are released under CC BY 2.0 FR. A part of our sentences are also available under CC0 1.0."
> — https://tatoeba.org/en/downloads

**Gotchas (important):**
1. **Per-sentence licenses vary.** Tatoeba allows CC0 / CC BY / CC BY-SA / CC BY-ND per sentence ("Certain sentences… cannot be modified: those contributed with a condition of non-modification… or a condition of sharing under the same conditions"). The OPUS Tatoeba package states CC BY 2.0 FR for the whole bundle — that is an **over-broad statement** if the export mixes ND/SA sentences.
2. **OPUS does not export attribution metadata.** Verified by downloading `OPUS-Tatoeba/v2026-07-08/moses/cmn-en.txt.zip`: it contains only `Tatoeba.cmn-en.cmn` / `.en` plain parallel lines plus a `LICENSE` (the French CC BY 2.0 legalcode) and `README`. **No sentence IDs, no author usernames.** CC BY requires crediting the author, so you must re-join against Tatoeba's own `sentences_detailed` TSV (which carries `username` and `license`) using Tatoeba's sentence IDs — the OPUS files cannot support compliant attribution on their own.
3. OPUS uses **`cmn`** (not `zh`) for Mandarin. Latest cmn–en = 49,851 pairs; it is small, high quality, beginner-friendly — the best "example sentence" source by content, with a real attribution workflow needed.

### 2.2 WikiMatrix — ⚠️ OK with ShareAlike obligations
OPUS states on the dataset page and in `info.yaml`:
> "License: CC-BY-SA 4.0" / "The data is released under the Creative Commons Attribution-ShareAlike" — https://opus.nlpl.eu/datasets/WikiMatrix

**Skeptical check:** the popular claim "WikiMatrix is CC BY-SA" is **not stated by FAIR anywhere**. The WikiMatrix README (https://raw.githubusercontent.com/facebookresearch/LASER/main/tasks/WikiMatrix/README.md) describes the mining and gives download URLs, with **no license for the data**; the only FAIR license is the LASER repo's **BSD** license, which covers the *software* (`tasks/CCMatrix`, `tasks/WikiMatrix` scripts), not the mined TSVs. OPUS is therefore **asserting** CC BY-SA 4.0 by inference from Wikipedia's own terms — which happens to be defensible, since the sentences are extracted from Wikipedia text, and Wikipedia states:
> "Most of Wikipedia's text… co-licensed under the Creative Commons Attribution-ShareAlike 4.0 International License (CC BY-SA) and the GNU Free Documentation License (GFDL)… If you make modifications or additions to the page you re-use, you must license them under the Creative Commons Attribution-ShareAlike 4.0 International License or later."
> — https://en.wikipedia.org/wiki/Wikipedia:Copyrights

**Gotcha:** under CC BY-SA 4.0 you must give attribution (typically a link to the article), keep the license notice, and license **adapted material** under BY-SA. A verbatim collection shipped in a proprietary app does not automatically re-license your app code, but any edited/re-annotated sentence text does become Adapted Material. There is also a residual risk that OPUS's assertion is wrong for a given sentence (Wikipedia has some non-CC-BY-SA imports).

### 2.3 CCMatrix — ❌ NOT OK
**OPUS states no license**: the `/datasets/CCMatrix` page has no License field, `corpus/CCMatrix/v1/info.yaml` has no `license:` key, and `RELEASE_LICENSES.tsv` records `unknown`. The FAIR README says only:
> "We open-source our scripts in this directory so that others may reproduce the data, evaluation and results reported in the CCMatrix paper." — https://raw.githubusercontent.com/facebookresearch/LASER/main/tasks/CCMatrix/README.md

So: **no data license grant, no license at the source.** Additionally the text is mined from Common Crawl:
> "CC strongly recommends that you obtain the advice of legal counsel before making any use, including commercial use… BY USING THE CRAWLED CONTENT, YOU AGREE TO RESPECT THE COPYRIGHTS AND OTHER APPLICABLE RIGHTS OF THIRD PARTIES." — https://commoncrawl.org/terms-of-use

This is the single largest zh source in OPUS (71.4M en–zh pairs) and it is the one to avoid.

### 2.4 NLLB / NLLB-Seed / FLORES — split verdicts
- **NLLB-200 model**: CC-BY-NC 4.0 → **NOT OK** (but it's a model, not the sentence data).
- **Meta's mined-bitext metadata**: CC-BY-NC 4.0 per the NLLB paper's appendix → NOT OK.
- **NLLB-Seed**: CC BY-SA 4.0 — but it contains **no Chinese** (39 languages).
- **`allenai/nllb` (the actual sentence data on HF)**: *"The dataset is released under the terms of ODC-BY… By using this, you are also bound to the respective Terms of Use and License of the original source."* Has `eng_Latn–zho_Hans` / `zho_Hant`.
- **FLORES-200**: README states "* FLORES-200: CC-BY-SA 4.0"; it is **dev/devtest/test only** — an evaluation benchmark, explicitly not training data. Contains `zho_Hans`, `zho_Hant`, `yue_Hant`. **Do not treat FLORES as a training-data license**; and note the NLLB paper has **no per-dataset license table** (its corpus inventory Table 52 has no license column) — a common misstatement.
- **OPUS "NLLB" v1**: OPUS states **ODC-By** ([ODC-By 1.0](https://opendatacommons.org/licenses/by/1-0/); §3.1 "These rights explicitly include commercial use, and do not exclude any field of endeavour"; §4.2 requires shipping the license/URI and intact notices). OPUS describes it as: *"created based on metadata for mined bitext released by Meta AI… This release is based on the data package released at huggingface [by] AllenAI."* API check: OPUS NLLB `en–zh` = 71,383,325 = **exactly OPUS CCMatrix `en–zh`** (identical pair and token counts) — so the "ODC-By" grant sits on top of the same unlicensed web text. Verdict: **OK with attribution for the database; underlying sentences remain uncleared.**

### 2.5 OpenSubtitles — ❌ NOT OK
OPUS states no license for any version. Upstream Terms of Service:
> "These files are NOT illegal warez downloads, we only offer files that we believe we are free to redistribute. If any doubt occurs about the legality of any of our file downloads we will take them off right away after contacting us… **Commercial use prohibited.**"
> — https://opensubtitles.tawk.help/article/terms-of-service

OPUS additionally asks (legacy page): *"IMPORTANT: If you use the OpenSubtitle corpus: Please, add a link to http://www.opensubtitles.org/ to your website and to your reports and publications produced with the data!"* — a request, not a license.

**Version gotcha (verified via API):** `OpenSubtitles v2018` contains **zero** Chinese pairs. `v2024` uses `zh_CN`/`zh_TW` codes (22.4M / 18.6M en-pairs — far larger than v2016's 9.3M `en–zh`). Movie/TV subtitle text is also third-party/user copyright.

### 2.6 TED2020 / TED2013 / NeuLab-TedTalks — ❌ NOT OK
OPUS TED2020 states only "Please respect the TED Talks Usage Policy"; TED2013 and NeuLab-TedTalks have no license at all. The policy is explicit:
> "We encourage you to share TED Talks, under our Creative Commons license… CC BY–NC–ND 4.0 International"
> "**NC:** means you cannot use TED Talks in any commercial context or to gain any type of revenue, payment or fee from the license sublicense, access or usage of TED Talks **in an app of any kind** for any advertising, or in exchange for payment of any kind."
> "**ND:** means that no derivative works are permitted so you cannot edit, remix, create, modify or alter the form of the TED Talks in any way."
> "Transcripts and subtitles may be used under the same Creative Commons license in conjunction with the TED Talk video. Copyright on the transcripts is owned by TED and any edits, alternate usage rights or changes to these documents are not permitted without permission."
> — https://www.ted.com/about/our-organization/our-policies-terms/ted-talks-usage-policy

TED.com Terms of Use §6.4 (updated 2024-05-07) further bars *"incorporating TED Content into artificial intelligence or machine learning workflows, tools, datasets, analyses, or research projects — including non-commercial or academic research."* NC/ND has been constant (the 2020 Wayback snapshot already said CC BY-NC-ND 4.0 "in an app of any kind"); 2024 only added the AI/dataset bar.

### 2.7 News-Commentary v16 — ⚠️ OK on packaging, third-party text
OPUS records `unknown`. The distribution README (verified live, after retries — statmt.org rate-limits hard):
> "Licence — The data is released on the same terms as the ParaCrawl (www.paracrawl.eu) data, i.e.: We do not own any of the text from which this data has been extracted. We license the actual packaging of this parallel data under the Creative Commons CC0 license ("no rights reserved")."
> — https://data.statmt.org/news-commentary/README

Conflict to note: the WMT19 task page says the data "can be freely used for research purposes… For other uses of the data, you should consult with original owners of the data sets." The CC0 statement is in the actual news-commentary distribution README and covers the *packaging* only; the articles themselves are newspaper copyright. Practically: legally-defensible on the compilation, exposed on the content.

### 2.8 MultiUN — ⚠️ OK with attribution (weaker)
No license file exists at any OPUS or DFKI location; the original euromatrixplus release directory was never archived. The compiler papers (Eisele & Chen, LREC 2010; Chen & Eisele, LREC 2012) state the documents are public domain under UN Administrative Instruction ST/AI/189/Add.9/Rev.2, which provides that UN documents do "not normally retain copyright… to facilitate dissemination". No explicit grant accompanies the corpus itself, so this is an inference from the source documents' status.

### 2.9 UNPC v1.0 — ✅ OK with attribution (best large option)
Verified live at https://www.un.org/dgacm/en/content/uncorpus (the old `cms.unov.org/UNCorpus` now redirects there):
> "The United Nations Parallel Corpus v1.0 is composed of official records and other parliamentary documents of the United Nations that are in the public domain."
> "When using the United Nations Parallel Corpus, the user must acknowledge the United Nations as the source of the information."
> "Disclaimer and terms of use — The following disclaimer, an integral part of the United Nations Parallel Corpus, shall be respected with regard to the Corpus (**no other restrictions apply**): …"

Requirement: attribute the UN as source and cite Ziemski, Junczys-Dowmunt & Pouliquen (LREC 2016). No NC clause, no ShareAlike. ~15.9M en–zh lines of formal UN prose — excellent for a bundled offline corpus, though the register is bureaucratic, not conversational.

### 2.10 QED v2.0a — ❌ NOT OK
OPUS states: *"The QED Corpus is made public for RESEARCH purpose only."* and *"Copyright Qatar Computing Research Institute. All rights reserved."* Upstream at QCRI repeats the research-only restriction; the underlying AMARA subtitles are volunteer-licensed with platform-limited rights.

### 2.11 GNOME / KDE4 / Ubuntu / KDEdoc — ❌ NOT OK
All four are `unknown` in OPUS's own license table and have **no `license:` key in their `info.yaml`**. They are localization corpora where every string's license follows its source module, and OPUS does **not preserve per-segment module provenance**.
- GNOME: *"GNOME requires that all its modules be licensed using an OSI approved license… The primary licenses used in GNOME are: GPL, versions 2 and 3 / LGPL version 2 / CC BY-SA. Each module's license should be included in its COPYING file."* (https://handbook.gnome.org/development/legal.html). Sample `.po` header: *"This file is distributed under the same license as the gnome-shell package."*
- KDE: Licensing Policy §7 — translations of section-4 files must be LGPL-2.1/3.0; section-5 files GPL-2/3. **KDE `.po` files carry no license header whatsoever** (checked current plasma/kstars/digikam/kate `zh_CN` files and KDE4-era `stable5`), so you cannot even determine the license per string.
- Ubuntu/Launchpad: strings are BSD (revised) for individual strings, but *"Translations of groups of strings from the same project, to the extent forming a derivative work of the project, are made available… under the licence applicable to the project"* — i.e. mostly GPL/LGPL, with external imports retaining their original license.
- Practical: filtering to a clean subset means per-file auditing of thousands of modules — not viable for a bundled commercial app.

### 2.12 Mozilla-I10n — ⚠️ OK with MPL-2.0 obligations
OPUS states **Mozilla Public License 2.0**, sourced from `https://github.com/mozilla-l10n/mt-training-data`. Both that repo and `mozilla-l10n/firefox-l10n` ship an MPL-2.0 `LICENSE`. MPL 2.0 permits commercial use, redistribution and modification; it is **file-level** copyleft — you must keep notices/license text and make the source of MPL-covered files available, but your own separate code is not infected. This is a large, under-appreciated zh localization source (~108k `en–zh_CN` pairs).

### 2.13 GlobalVoices — ✅ OK with attribution
OPUS records `unknown` and cites CASMACAT's corpus page (now dead). Upstream is unambiguous and stable:
> "This site is licensed as Creative Commons Attribution 3.0." — https://globalvoices.org/ (footer)
> "Unless otherwise stated, all content created by Global Voices is published under a Creative Commons Attribution-Only license… Adapt — remix, transform, and build upon the material for any purpose, **even commercially**." — https://globalvoices.org/about/global-voices-attribution-policy/

**Correction to received wisdom:** the often-repeated claim that GlobalVoices was CC BY-SA 3.0 at the 2018 crawl is **not supported** — the Wayback snapshot of 2018-11-02 shows the same "Creative Commons Attribution 3.0" footer. Attribution is required; OPUS exports no per-story author metadata.

### 2.14 wikimedia / WikiTitles / LinguaTools-WikiTitles / Wikipedia — ⚠️ OK with ShareAlike obligations
OPUS states **CC-BY-SA 4.0** for wikimedia and WikiTitles, **CC-BY-SA** (unversioned) for LinguaTools-WikiTitles. Sources: `https://dumps.wikimedia.org/other/contenttranslation` (wikimedia) and Wikipedia article titles. Underlying Wikipedia terms are quoted in §2.2. Note OPUS's **Wikipedia** v1.0 corpus (Wołk & Marasek) is marked `unknown` and contains **no Chinese**.

### 2.15 XLEnt — ❌ NOT OK
OPUS records `unknown`, no License field on the page, no `license:` in `info.yaml`. Upstream statmt states only *"No claims of intellectual property are made on the work of preparation of the corpus"* — a disclaimer, not a grant. Its three components (CCAligned, CCMatrix, WikiMatrix) are respectively unlicensed, unlicensed, and CC-BY-SA-by-assertion, so the combination inherits the worst case.

### 2.16 ParaCrawl / HPLT / MultiParaCrawl — ⚠️ OK on packaging only
Both projects use the identical wording:
> "We do not own any of the text from which these data has been extracted. **We license the actual packaging of these parallel data under the Creative Commons CC0 license ("no rights reserved")**."
> — https://paracrawl.eu/ (License) and https://hplt-project.org/datasets (Terms of Use)

HPLT adds: *"It is your responsibility that any use of the data complies with any applicable legal framework, such as, among others, the EU Copyright Directive 2019/790 and the General Data Protection Regulation 2018."* CC0 means no attribution obligation and full commercial freedom **for the compilation**; the sentences themselves remain third-party web text on a take-down-only basis. OPUS records `unknown` for all of these, so the CC0 claim comes from upstream, not OPUS.

### 2.17 Tanzil — ❌ NOT OK
OPUS records `unknown`. The real terms are **not** at `tanzil.net/docs/terms` (dead wiki topic) but at `https://tanzil.net/trans/`:
> "Terms of Use — The translations provided at this page are **for non-commercial purposes only**. If used otherwise, you need to obtain necessary permission from the translator or the publisher… Redistributing the following list in another website is not allowed, unless direct permission is granted by the Tanzil Project."

### 2.18 bible-uedin — ✅ OK
OPUS states **CC0 1.0**; upstream `christos-c/bible-corpus` LICENSE is CC0 1.0. CC0 waives the compiler's rights, but does not audit the copyright status of each Bible translation; the Chinese tokenised subset originates from Goethe University (hucompute.org). 124k zh pairs of archaic/religious register — limited use for everyday Mandarin, but clean.

### 2.19 tldr-pages — ✅ OK with attribution
> "Copyright © 2014—present the tldr-pages team and contributors. **This work is licensed under the Creative Commons Attribution 4.0 International License (CC-BY).**"
> — https://github.com/tldr-pages/tldr/blob/main/LICENSE.md

No ShareAlike. 17.5k zh pairs of short, practical CLI explanations — good register for beginner example sentences.

### 2.20 PHP manual — ✅ OK with attribution
OPUS records `unknown`, but upstream is explicit:
> "Copyright © 1997 - 2026 by the PHP Documentation Group. This material may be distributed only subject to the terms and conditions set forth in the Creative Commons Attribution 3.0 License or later."
> — https://www.php.net/manual/en/copyright.php (Chinese: https://www.php.net/manual/zh/copyright.php)

The PHP License covers the *software*, not the manual.

### 2.21 OpenOffice v3 — ❌ NOT OK
OPUS records `unknown`. Upstream mixes legacy OpenOffice.org-era licenses:
> "For past releases under the SUN/Oracle umbrella… The source-code license was the GNU Lesser General Public License. Effective OpenOffice.org 3.0 Beta, OpenOffice.org used the LGPL v3. The document license was the Public Document License (PDL)." / "In some cases, the use of the Creative Commons Attribution License ("Attribution-NoDerivs 2.5") was also permitted."
> — https://www.openoffice.org/license.html

Because the OPUS scrape does not preserve which page each string came from, you cannot exclude the ND-licensed subset.

### 2.22 MDN_Web_Docs — ⚠️ OK with ShareAlike obligations
OPUS states **CC-BY-SA 2.5**; MDN content is CC BY-SA 2.5 (attribution + ShareAlike). 66.6k `en–zh_CN` pairs of technical prose.

### 2.23 translatewiki — ✅ OK with attribution (tiny)
OPUS states **CC BY 3.0**; upstream:
> "Translations by translators are licensed CC BY 3.0, and derivative works may also be licensed under the licenses of the respective Free and Open Source projects the translations have been or will be added to. Content of user pages are considered to be "All rights reserved" by the author. All other content is licensed CC BY 3.0 unless a different license or copyright is stated explicitly."
> — https://translatewiki.net/wiki/Project:About (Cloudflare-blocked to scripts; quoted from the 2025-01-14 Wayback snapshot)

Only 177 `en–zh` pairs (`zh_hk` 5,304, `lzh` 3,138 are larger but nearly useless for modern Mandarin).

### 2.24 WMT-News v2019 — ❌ NOT OK
> "LICENSING OF DATA — The data released for the WMT19 news translation task can be **freely used for research purposes**, we just ask that you cite the WMT19 shared task overview paper… For other uses of the data, you should consult with original owners of the data sets."
> — http://www.statmt.org/wmt19/translation-task.html

### 2.25 FLORES-200 (context only)
CC BY-SA 4.0, dev/devtest/test only. It is an **evaluation benchmark**, not training/example data, and it is not part of OPUS.

---

## 3. Bottom line for a commercial offline bundled app

**Ship-ready Chinese sources (in rough order of usefulness):**
1. **UNPC v1.0** — ~15.9M en–zh pairs, public domain + mandatory UN attribution; the only large corpus with an explicit, unrestricted upstream grant.
2. **Tatoeba (cmn)** — ~50k pairs, CC BY 2.0 FR, best register for learners; must re-join Tatoeba's own exports to obtain per-sentence license + author attribution.
3. **tldr-pages** (CC BY 4.0) and **PHP manual** (CC BY 3.0+) and **GlobalVoices** (CC BY 3.0) — modest but clean.
4. **bible-uedin** (CC0) — clean but archaic register.
5. **Mozilla-I10n** (MPL 2.0, ~108k en–zh_CN) — clean-ish with file-level copyleft obligations, good for short UI-register strings.

**Usable with ShareAlike obligations:** WikiMatrix, wikimedia, WikiTitles/LinguaTools-WikiTitles, MDN_Web_Docs, FLORES-200 (eval only).

**Packaging-only CC0 (third-party web text, take-down risk):** ParaCrawl, MultiParaCrawl (2.1M zh pairs), HPLT (5.3M zh_hant), News-Commentary, OPUS-NLLB (ODC-By).

**Hard NO:** CCMatrix (unknown/no grant, 71M pairs), OpenSubtitles (commercial use prohibited), TED2020/TED2013/NeuLab-TedTalks (CC BY-NC-ND), QED (research only), Tanzil (non-commercial), GNOME/KDE4/Ubuntu/KDEdoc (per-module copyleft, provenance lost), OpenOffice (unpreserved legacy/ND), XLEnt/CCAligned/MultiCCAligned (no license), WMT-News (research only), Books (commercial not granted), JParaCrawl (terms-of-use unclear). Meta's NLLB-200 model and mined-bitext metadata are CC-BY-NC 4.0; NLLB-Seed is CC BY-SA 4.0 but has no Chinese.

**Cross-cutting caution:** for every web-mined corpus the CC0/ODC-By/CC-BY-SA label covers the *compilation*, not the individual sentences (ParaCrawl and HPLT state this explicitly; ODC-By §2.4 does not cover the contents). Shipping 70M uncleared web sentences in a commercial artifact is a take-down and reputational exposure even where the packaging license looks permissive.

## 4. Verification status

**Primary-verified in this session (fetched directly):** OPUS `RELEASE_LICENSES.tsv`; OPUS dataset pages + `info.yaml` files; OPUS API pair counts; OPUS site Disclaimer; Tatoeba terms + downloads page + the actual `cmn-en` package contents; LASER/WikiMatrix/CCMatrix READMEs and LASER LICENSE; Wikipedia:Copyrights; UNPC terms page; TED Talks Usage Policy; OpenSubtitles ToS; news-commentary README; ParaCrawl and HPLT license sections; WMT19 licensing section; Tanzil terms; php.net copyright; tldr-pages LICENSE.md; Mozilla `mt-training-data` LICENSE; Community/KDE and GNOME legal clauses; Launchpad policies.

**Labeled secondhand / not independently re-verified:** MultiUN's public-domain basis (from the LREC papers' description of UN ST/AI/189/Add.9/Rev.2 — the original euromatrixplus release is dead and unarchived); the `allenai/nllb` HF card wording and florex/nllb README license lines (relayed from the delegated research, not re-fetched here); translatewiki's CC BY 3.0 clause (Cloudflare-blocked; quoted from a 2025-01-14 Wayback snapshot).

**Known-unreachable:** `data.statmt.org` is behind aggressive rate-limiting (intermittent connection refused) — the news-commentary README was retrieved only after repeated retries; `statmt.org` hosts for CCMatrix/WikiMatrix did not serve license text; `cms.unov.org` now redirects to un.org; `tanzil.net/docs/terms` is a dead page; `gitlab.gnome.org` and `translatewiki.net` block scripted fetches.
