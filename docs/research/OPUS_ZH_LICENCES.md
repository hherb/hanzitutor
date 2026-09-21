# OPUS zh (cmn) license research — commercial bundled offline app

Research date: 2026-09-21. Method: direct `curl` of primary sources + Wayback Machine where a site is dead/blocked. All quotes are verbatim from the cited URL. Wayback snapshots are labelled. Secondary/corroborating sources are labelled.

Environment note: `https://www.opensubtitles.org` and `https://www.opensubtitles.com` block plain `curl` (000 / Cloudflare 403); I used the vendor's own help-center copy plus Wayback. `data.statmt.org` serves only over plain HTTP (HTTPS returns 000), so its URLs are cited as `http://`.

---

## 0. What OPUS itself says (all targets)

The new OPUS dataset pages are client-rendered; the metadata is in the embedded `self.__next_f` payload. Extracted verbatim from each `https://opus.nlpl.eu/datasets/<Name>`:

| OPUS dataset | Version | OPUS "License" field | OPUS "Copyright" field |
|---|---|---|---|
| News-Commentary | v16 | **none** | none |
| WMT-News | v2019 | **none** | none |
| MultiUN | v1 | **none** | none |
| UNPC | v1.0 | **none** | none |
| OpenSubtitles | v2024 | **none** | none |
| QED | v2.0a | `The QED Corpus is made public for RESEARCH purpose only.` | `Copyright Qatar Computing Research Institute. All rights reserved.` |
| TED2020 | v1 | `Please respect the <a href=https://www.ted.com/about/our-organization/our-policies-terms/ted-talks-usage-policy>TED Talks Usage Policy</a>` | none |
| TED2013 | v1.1 | **none** | none |
| NeuLab-TedTalks | v1 | **none** | `Please respect the <a href="https://www.ted.com/about/our-organization/our-policies-terms" target="_blank">TED Talks Usage Policy</a>` (under an `h2` heading literally titled "Copyright") |

OPUS descriptions verbatim:
- News-Commentary: `A parallel corpus of News Commentaries provided by WMT for training SMT. The source is taken from WMT 19: http://data.statmt.org/news-commentary/v16/documents.tgz`
- WMT-News: `A parallel corpus of News Test Sets provided by WMT for training SMT: http://www.statmt.org/wmt19/`
- MultiUN: `This is a collection of translated documents from the United Nations originally compiled by Andreas Eisele and Yu Chen (see http://www.euromatrixplus.net/multi-un/).`
- UNPC: `United Nations Parallel Corpus  Source: https://cms.unov.org/UNCorpus` followed by the UN disclaimer (see §4).
- OpenSubtitles: `This is a new collection of translated movie subtitles from http://www.opensubtitles.org/. IMPORTANT: If you use the OpenSubtitle corpus: Please, add a link to http://www.opensubtitles.org/ to your website and to your reports and publications produced with the data! I promised this when I got the data from the providers of that website!`
- QED: `The QCRI Educational Domain Corpus (formerly QCRI AMARA Corpus) is an open multilingual collection of subtitles for educational videos and lectures collaboratively transcribed and translated over the AMARA web-based platform. ... Copyright Qatar Computing Research Institute. All rights reserved.`
- TED2020: `...contains a crawl of nearly 4000 TED and TED-X transcripts from July 2020. The transcripts have been translated by a global community of volunteers to more than 100 languages.`
- TED2013: `A parallel corpus of TED talk subtitles provided by CASMACAT: http://www.casmacat.eu/corpus/ted2013.html. The files are originally provided by https://wit3.fbk.eu.`
- NeuLab-TedTalks: `This dataset is compiled from TED talk subtitles and distributed through phontron.com. ... The transcripts have been translated by a global community of volunteers to more than 100 languages.`

OPUS site-wide disclaimer (primary, still live on the legacy corpus pages, e.g. `https://opus.nlpl.eu/legacy/News-Commentary.php` and `https://opus.nlpl.eu/legacy/MultiUN.php`):

> We do not own any of the text from which the data has been extracted. We only offer files that we believe we are free to redistribute. If any doubt occurs about the legality of any of our file downloads we will take them off right away after contacting us.

So OPUS grants nothing; it relays upstream terms and disclaims ownership/offers take-down only. The `zh` bitexts exist for every target (OPUS API `https://opus.nlpl.eu/api/corpora-table?dataset=<Name>&source=en&target=zh`): News-Commentary 133,476; WMT-News 19,965; MultiUN 9,564,315; UNPC 19,944,041; OpenSubtitles v2016 10,329,887; QED 20,768; TED2020 16,382; TED2013 156,811; NeuLab-TedTalks 8,076 en–zh sentence pairs.

---

## 1. OPUS News-Commentary v16

**(a) Contents / zh.** WMT news commentary, v16 subdirectory, `documents.tgz` + `training/` + `training-monolingual/`; languages include zh (`http://data.statmt.org/news-commentary/v16/`). OPUS en–zh: 133,476 pairs (v11 in the API is the OPUS version tag).

**(b) Upstream license — verbatim.** The WMT directory README, `http://data.statmt.org/news-commentary/README`:

> Licence
> -------
> The data is released on the same terms as the ParaCrawl (www.paracrawl.eu) data, i.e.:
> - We do not own any of the text from which this data has been extracted.
> - We license the actual packaging of this parallel data under the Creative Commons CC0 license ("no rights reserved").
> For take-down policy, see https://www.paracrawl.eu/releases.html

The referenced ParaCrawl terms, `https://www.paracrawl.eu/releases`, verbatim:

> License
> These data are released under this licensing scheme:
> We do not own any of the text from which these data has been extracted. We license the actual packaging of these parallel data under the Creative Commons CC0 license ("no rights reserved") .
> Notice and take down policy
> Notice: Should you consider that our data contains material that is owned by you and should therefore not be reproduced here, please: ...

**(c) Commercial / redistribution / modification.** The *packaging* is CC0 1.0 ("no rights reserved"): commercial use, redistribution inside a closed-source app artifact, and modification of the packaging are all permitted, with no attribution obligation and no ShareAlike. There is no NonCommercial and no research-only term.

**(d) Gotchas.** WMT explicitly says it does not own the text. News commentary articles are copyrighted by the news publishers; WMT's only mechanism is a take-down policy. OPUS repeats the same non-ownership disclaimer. Shipping the underlying Chinese/English news sentences is a third-party-copyright exposure that CC0 on the packaging does not cure.

**(e) Verdict: OK for commercial bundled app on the packaging (CC0); NOT clean — the underlying news text is third-party copyrighted and is shipped on a take-down-only basis. No publisher license exists.**

---

## 2. OPUS WMT-News v2019

**(a) Contents / zh.** The WMT19 news translation task test sets (newstest2019), distributed at `http://data.statmt.org/wmt19/` (`translation-task/test.tgz`); the WMT-News OPUS corpus is those news test sets. zh pairs present; OPUS en–zh 19,965.

**(b) Upstream terms — verbatim.** WMT19 news translation task page, `http://www.statmt.org/wmt19/translation-task.html`, section `DATA` → `LICENSING OF DATA`:

> The data released for the WMT19 news translation task  can be freely used for research purposes, we just ask that you cite the WMT19 shared task overview paper, and respect any additional citation requirements on the individual data sets. For other uses of the data, you should consult with original owners of the data sets.

Same page on provenance:

> The 2019 test sets will be created from a sample of online newspapers from September-November 2018.

There is no separate LICENSE file on the WMT19 site (directory listings show none), and the WMT news-commentary CC0 README covers the news-commentary corpus, not these test sets. `https://github.com/wmt-conference` has no news-commentary/WMT-News data repo with a license (checked the org repo list: only website/format-tooling repos, Apache-2.0 where a license exists).

**(c) Commercial / redistribution / modification.** "Freely used for research purposes" only. "For other uses … consult with original owners" = no commercial grant, no redistribution grant, no modification grant from WMT.

**(d) Gotchas.** Text is sampled from online newspapers → the newspapers hold copyright; WMT tells you to negotiate with them directly.

**(e) Verdict: NOT OK (research purposes only; commercial use requires permission from the original newspaper owners, which WMT does not grant).**

---

## 3. OPUS MultiUN

**(a) Contents / zh.** UN documents 2000–2009 (v1) and up to 2011 (v2), cleaned/aligned at DFKI LT-Lab; 6 UN languages + German, including zh. OPUS en–zh 9,564,315 pairs.

**(b) Upstream terms — verbatim.** Live `http://www.euromatrixplus.net/multi-un/` is dead (returns a 4-line JS redirect to `/lander`). No license file exists. The archived (Wayback) release page, `https://web.archive.org/web/20181125203735id_/http://www.euromatrixplus.net/multi-un/` — **Wayback snapshot, labelled**:

> The MultiUN parallel corpus is extracted from the United Nations Website , and then cleaned and converted to XML at Language Technology Lab in DFKI GmbH (LT-DFKI), Germany. The documents were published by UN from 2000 to 2009. For a detailed description of this corpus, please read: MultiUN: A Multilingual corpus from United Nation Documents , Andreas Eisele and Yu Chen , LREC 2010. Please cite the paper, if you use this corpus in your work.

Page footer, same snapshot:

> All content is &copy; 2009-2012 by the EuroMatrixPlus Consortium, all rights reserved, see also legal information and data protection .

The archived "legal information" page (`https://web.archive.org/web/20170610111901id_/http://www.euromatrixplus.net/legal-information/`) contains only DFKI postal/liability/privacy text — **no data license**. No MultiUN README or LICENSE is retrievable: DFKI URLs return 403, the release directory was never archived (Wayback CDX for `euromatrixplus.net/media/un-release*` is empty).

The compiler's own papers state the copyright basis. Eisele & Chen, LREC 2010 (`http://www.lrec-conf.org/proceedings/lrec2010/pdf/686_Paper.pdf`):

> The documents we collected are in public domain according to the Administrative Instruction

Chen & Eisele, LREC 2012, "MultiUN v2" (`https://mt-archive.net/10/LREC-2012-Chen.pdf`), the sentence completed:

> The documents we collected are in public domain according to the Administrative Instruction from the United Nations (ST/AI/189/Add.9/Rev.2) (United Nations Secretariat, 1987).

The cited UN instruction, `ST/AI/189/Add.9`, 29 March 1972, "COPYRIGHT IN UNITED NATIONS PUBLICATIONS: GENERAL PRINCIPLES, PRACTICE AND PROCEDURE" (`https://documents.un.org/api/symbol/access?s=ST/AI/189/ADD.9&l=en&t=pdf`), verbatim (OCR):

> 1. The United Nations does not normalJ~ retain copyright~ its pOlicy being rather to facilitate dissemination of'the contents of its publications as widely as possible by all reasonable means. General retention of copyri~ht would give an impression of restriction and of setting up a procedural barrier - namely, the need to re~uest permission to use material.

Secondary corroboration, labelled: HuggingFace dataset card `Helsinki-NLP/multiun` (`https://huggingface.co/datasets/Helsinki-NLP/multiun/raw/main/README.md`) specifies `license: unknown` and `### Licensing Information` → `[More Information Needed]`.

**(c) Commercial / redistribution / modification.** No license is granted by the compiler at all. The *content* is UN official documents, which the UN treats as public domain (no copyright retained), so the copyright status of the text itself supports commercial reuse; but there is no explicit written permission from DFKI/EuroMatrixPlus and the project site says "all rights reserved" for the site content.

**(d) Gotchas.** No explicit upstream license → no clean chain of title, only the "UN documents are public domain" argument from the papers. Attribution expected (Eisele & Chen LREC 2010; Tiedemann LREC 2012).

**(e) Verdict: OK with attribution on the basis that the UN documents are public domain (no license is stated by the compiler; the CE "all rights reserved" notice covers the website, and no LICENSE/README file exists upstream).**

---

## 4. OPUS UNPC (United Nations Parallel Corpus v1.0)

**(a) Contents / zh.** Official UN records and parliamentary documents 1990–2014, six official languages including zh, sentence-aligned; ~799k documents / 1.73M aligned pairs, plus dev/test sets. OPUS en–zh 19,944,041 pairs.

**(b) Upstream terms — verbatim.** `https://cms.unov.org/UNCorpus` and `https://conferences.unite.un.org/uncorpus` both now redirect (HTTP 200) to `https://www.un.org/dgacm/en/content/uncorpus`, canonical `<link rel="canonical" href="https://www.un.org/dgacm/en/content/uncorpus" />`. Verbatim from that page:

> The United Nations Parallel Corpus v1.0 is composed of official records and other parliamentary documents of the United Nations that are in the public domain. These documents are mostly available in the six official languages of the United Nations.

> When using the United Nations Parallel Corpus, the user must acknowledge the United Nations as the source of the information.

> Disclaimer and terms of use
> The following disclaimer, an integral part of the United Nations Parallel Corpus, shall be respected with regard to the Corpus (no other restrictions apply):
> - The United Nations Parallel Corpus is made available without warranty of any kind, explicit or implied. The United Nations specifically makes no warranties or representations as to the accuracy or completeness of the information contained in the United Nations Corpus.
> - Under no circumstances shall the United Nations be liable for any loss, liability, injury or damage incurred or suffered that is claimed to have resulted from the use of the United Nations Corpus. The use of the United Nations Corpus is at the user's sole risk. The user specifically acknowledges and agrees that the United Nations is not liable for the conduct of any user. If the user is dissatisfied with any of the material provided in the United Nations Corpus, the user's sole and exclusive remedy is to discontinue using the United Nations Corpus.
> - When using the United Nations Corpus, the user must acknowledge the United Nations as the source of the information. For references, please cite this reference: Ziemski, M., Junczys-Dowmunt, M., and Pouliquen, B., (2016), The United Nations Parallel Corpus, Language Resources and Evaluation (LREC'16), Portorož, Slovenia, May 2016.
> - Nothing herein shall constitute or be considered to be a limitation upon or waiver, express or implied, of the privileges and immunities of the United Nations, which are specifically reserved.

**(d) Gotchas — the general UN site terms conflict.** The corpus page says public domain + "no other restrictions apply". The general UN web terms are narrower. `https://www.un.org/en/about-us/terms-of-use`, verbatim:

> The United Nations grants permission to Users to visit the Site and to download and copy the information, documents and materials (collectively, "Materials") from the Site for the User's personal, non-commercial use, without any right to resell or redistribute them or to compile or create derivative works therefrom, subject to the terms and conditions outlined below, and also subject to more specific restrictions that may apply to specific Material within this Site.

`https://www.un.org/en/about-us/copyright`, verbatim:

> Copyright © United Nations All rights reserved. None of the materials provided on this web site may be used, reproduced or transmitted, in whole or in part, in any form or by any means, electronic or mechanical, including photocopying, recording or the use of any information storage and retrieval system, except as provided for in the Terms and Conditions of Use of United Nations Web Sites, without permission in writing from the publisher.

Read together, the general terms are expressly subordinate to "more specific restrictions that may apply to specific Material", and the UNPC page states "no other restrictions apply" and "public domain". The corpus-specific terms therefore govern.

**(c) Commercial / redistribution / modification.** Permitted: the corpus is public domain, "no other restrictions apply"; mandatory acknowledgment of the UN as source (and citation). No NonCommercial, no ShareAlike, no research-only clause on the corpus page.

**(e) Verdict: OK with attribution (public domain official records; mandatory UN acknowledgment + citation; "no other restrictions apply"). Residual risk: the general UN website ToU/copyright notice says non-commercial — rely on the corpus-specific terms, which are the more specific and later statement.**

---

## 5. OPUS OpenSubtitles v2018 / v2024

**(a) Contents / zh.** Aligned movie/TV subtitles from opensubtitles.org; v2024 = 93 languages, 3,377 bitexts, 41.03G tokens (OPUS legacy page). zh present; OPUS en–zh 10,329,887 (API returns the v2016 record for en–zh).

**(b) Upstream terms — verbatim.** OpenSubtitles' official Terms of Service, `https://opensubtitles.tawk.help/article/terms-of-service` (the vendor's help centre; the same text is the ToS at `https://www.opensubtitles.com/en/tos/`, which I verified via Wayback `https://web.archive.org/web/20241213212909/https://www.opensubtitles.com/en/tos/` — **Wayback snapshot, labelled**):

> OpenSubtitles.com ("this website") offers direct downloads off our own server. These files are NOT illegal warez downloads, we only offer files that we believe we are free to redistribute. If any doubt occurs about the legality of any of our file downloads we will take them off right away after contacting us. We do not offer any kind of video/movie (avi, divx, xvid...) files or audio (mp3, wma, waw) files - we mainly provide movie's subtitles translated by users. **Commercial use prohibited.**

The classic OpenSubtitles.org disclaimer, live path `http://www.opensubtitles.org/en/disclaimer` (blocked to curl; quoted from Wayback `https://web.archive.org/web/20160602044251/http://www.opensubtitles.org/en/disclaimer` — **Wayback snapshot, labelled**):

> Disclaimer: The creator of OpenSubtitles.org ("this website") takes no responsibility or liability for anything that happens as a result of reading or downloading anything on this website or anything contained in subsequent pages. These documents may not be freely distributed and used for non-commercial, scientific and educational purposes. Commercial use of the documents available from this website is protected under the U.S. and Foreign Copyright Laws.

(The "may not be freely distributed and used for non-commercial …" wording is grammatically contradictory as published; the operative sentence is "Commercial use … is protected under the U.S. and Foreign Copyright Laws", and the current ToS resolves the ambiguity with the flat "Commercial use prohibited.")

OPUS's own instruction (from `https://opus.nlpl.eu/datasets/OpenSubtitles` and the legacy page `https://opus.nlpl.eu/legacy/OpenSubtitles.php`):

> IMPORTANT: If you use the OpenSubtitle corpus: Please, add a link to http://www.opensubtitles.org/ to your website and to your reports and publications produced with the data! I promised this when I got the data from the providers of that website!

The OPUS OpenSubtitles2018 paper (Lison, Tiedemann, Kouylekov, LREC 2018, `http://www.lrec-conf.org/proceedings/lrec2018/pdf/294.pdf`) contains **no license statement** — it describes extraction/alignment only.

**(c) Commercial / redistribution / modification.** Explicitly prohibited: "Commercial use prohibited." Bulk redistribution of the files inside a closed-source commercial app is not licensed.

**(d) Gotchas.** Ownership is layered: the site only claims it "believe[s] we are free to redistribute"; the subtitle text is uploaded by anonymous users, and the underlying dialogue/film is copyrighted by the studios. No rights-holder grants you a license.

**(e) Verdict: NOT OK (site says "Commercial use prohibited"; underlying movie/TV dialogue and user-uploaded subtitle text are third-party copyrighted; OPUS only asks for a link, which is not a license).**

---

## 6. OPUS QED v2.0a

**(a) Contents / zh.** QCRI Educational Domain Corpus (formerly QCRI AMARA Corpus): subtitles of educational videos/lectures transcribed and translated on the Amara platform; v1.4 = 20 languages incl. `zhs`/`zht`, 44,620 files; MT dataset (IWSLT 2016 permissible data) + raw corpus. OPUS en–zh 20,768 pairs.

**(b) Upstream license — verbatim.** OPUS `https://opus.nlpl.eu/datasets/QED` License field:

> The QED Corpus is made public for RESEARCH purpose only.

and Copyright block:

> Copyright Qatar Computing Research Institute. All rights reserved.

Upstream QCRI ALT page, `https://alt.qcri.org/resources/qedcorpus/`, section "License", verbatim:

> Developed by: Qatar Computing Research Institute Arabic Language Technologies Group
> The QED Corpus is made public for RESEARCH purpose only. The corpus is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE

**(d) Gotchas.** Two layers. (i) QCRI: research only, all rights reserved — no commercial use. (ii) The subtitle text originates from Amara volunteers. Amara's parent, the Participatory Culture Foundation, `https://pculture.org/terms-of-service`, verbatim:

> You retain the rights to your intellectual property, but also give PCF (and those we work with) a worldwide license to use, host, store, reproduce, modify, create derivative works (such as those resulting from translations, adaptations or other changes we make so that your content works better with our Services), communicate, publish, publicly perform, publicly display and distribute such content. ... The rights granted in this license are limited to the sole purpose of running, publicizing, and enhancing our Services.

> Content not owned by PCF. Our Services may display content that is not PCF's. ... You may not use this content without permission from the owner …

So PCF passes no downstream redistribution right, and the volunteer authors retain theirs.

**(c)/(e) Verdict: NOT OK (explicit "RESEARCH purpose only", "All rights reserved"; plus volunteer-author and educational-video third-party rights).**

---

## 7. OPUS TED2020 v1 and TED2013

**(a) Contents / zh.** TED2020: crawl of ~4,000 TED/TEDx transcripts (July 2020) with volunteer translations, 100+ languages; OPUS en–zh 16,382. TED2013: casmacat/WIT3 TED talk subtitle release, v1.1; OPUS en–zh 156,811. Both are TED volunteer-translated subtitles.

**(b) Upstream license — verbatim.**
OPUS TED2020 License field: `Please respect the TED Talks Usage Policy` → `https://www.ted.com/about/our-organization/our-policies-terms/ted-talks-usage-policy`. OPUS TED2013 states no license.

TED Talks Usage Policy (`https://www.ted.com/about/our-organization/our-policies-terms/ted-talks-usage-policy`), verbatim:

> Personal use only (non-commercial): We encourage you to share TED Talks, under our Creative Commons license, or ( CC BY–NC–ND 4.0 International, which means it may be shared under the conditions below:

> NC: means you cannot use TED Talks in any commercial context or to gain any type of revenue, payment or fee from the license sublicense, access or usage of TED Talks in an app of any kind for any advertising, or in exchange for payment of any kind, including in any ad supported content or format.

> ND: means that no derivative works are permitted so you cannot edit, remix, create, modify or alter the form of the TED Talks in any way. This includes using the TED Talks as the basis for another work, including dubbing, voice-overs, or other translations not authorized by TED.

> Transcripts and subtitles may be used under the same Creative Commons license in conjunction with the TED Talk video. Copyright on the transcripts is owned by TED and any edits, alternate usage rights or changes to these documents are not permitted without permission. Therefore, if you wanted to publish a TED Talk in a book, test, play, or any other publication, permission is required.

> Commercial / Corporate use
> Using a TED Talk in a commercial context requires a license. This type of use is not limited to uses that generate profit, but includes employee learning, on an intranet system, at a company event, in training and educational materials, within a TED-branded video offering or editorial program, in tv/film including documentaries, in an in-person, online or virtual course, in a podcast, and more.

TED.com Terms of Use (`https://www.ted.com/about/our-organization/our-policies-terms/ted-com-terms-of-use`, page states `Date updated: May 7, 2024`), verbatim:

> 6.1 Published TED Content. Unless otherwise indicated, published TED Talks, TED-Ed videos, transcripts, and related speaker information may be made available under a Creative Commons Attribution–NonCommercial–NoDerivatives 4.0 International license (CC BY-NC-ND 4.0), which permits limited personal and educational uses consistent with that license and TED's Usage Policy. Any use beyond the scope of the applicable license or Usage Policy requires TED's prior written authorization.

> 6.3 Educational Use Clarification. For purposes of these Terms, "educational use" means use for private study, classroom instruction, discussion, commentary, or scholarly citation ... Educational or academic use does not include publishing, hosting, or storing TED Content on external websites or repositories; redistributing TED Content beyond the immediate instructional setting; incorporating TED Content into research datasets, archives, or collections; reproducing TED Content within academic publications or research outputs beyond brief quotation consistent with applicable law; creating derivative works; or using TED Content for the development, training, evaluation, fine-tuning, or operation of machine learning or artificial intelligence systems.

> 6.4 Automated Access, Scraping, and Artificial Intelligence Use. You may not use automated means — including scraping tools, bots, crawlers, harvesting technologies, or similar mechanisms — to access, collect, download, extract, or analyze TED Content in a manner inconsistent with these Terms. Embedding, reproducing, excerpting, downloading, scraping, or incorporating TED Content into artificial intelligence or machine learning workflows, tools, datasets, analyses, or research projects — including non-commercial or academic research — is prohibited absent a separate written license agreement with TED …

Historical confirmation — the same CC BY-NC-ND 4.0 terms and an even more explicit app clause, Wayback snapshot of the TED Talks Usage Policy, `https://web.archive.org/web/20201230185259/https://www.ted.com/about/our-organization/our-policies-terms/ted-talks-usage-policy` — **Wayback snapshot, labelled**:

> Personal Use only (non-commercial): We encourage you to share TED Talks, which TED makes available for distribution under our Creative Commons license, Attribution–Noncommercial–No Derivatives (or the CC BY–NC–ND 4.0 International) …

> Do not use the TED site content for any commercial purposes , for sale, for profit, sublicense or in an app of any kind for any advertising, or in exchange for payment of any kind …

> Transcripts and subtitles may be used under the same Creative Commons license in conjunction with the TED Talk video.

Secondary corroboration, labelled: HuggingFace dataset card `IWSLT/ted_talks_iwslt` (`https://huggingface.co/datasets/IWSLT/ted_talks_iwslt/raw/main/README.md`) declares `license: cc-by-nc-nd-4.0` and says the content "is published under the Creative Commons BYNC-ND license". `https://www.ted.com/participate/translate` (live) no longer states a license; it only links the Usage Policy.

**License change / timing.** The NC + ND conditions are unchanged (CC BY-NC-ND 4.0 since at least the 2020 snapshot). What changed with the **May 7, 2024** TED.com Terms of Use is the explicit bar on putting TED content into datasets and AI/ML workflows (§6.3, §6.4, "including non-commercial or academic research").

**(c) Commercial / redistribution / modification.** Commercial use requires a paid TED license. Redistribution bundled in a closed-source commercial app is squarely "in an app of any kind" and "commercial context" → not covered by the CC license. ND additionally forbids modification/derivative works.

**(d) Gotchas.** Even a research/non-commercial bundling of TED transcripts into a parallel corpus is now expressly prohibited absent a TED agreement. The OPUS TED2020 corpus is itself exactly such a dataset.

**(e) Verdict: NOT OK (CC BY-NC-ND 4.0: NonCommercial + NoDerivatives; commercial use requires a TED license; and the 2024 TED ToU expressly forbids incorporating TED content into datasets/AI/ML workflows even non-commercially).**

---

## 8. OPUS NeuLab-TedTalks v1

**(a) Contents / zh.** TED talk subtitle parallel data collected from the TED Open Translation Project, tokenized/detokenized, train split only; OPUS en–zh 8,076 pairs.

**(b) Upstream terms — verbatim.** OPUS (`https://opus.nlpl.eu/datasets/NeuLab-TedTalks`) states **no License field** but a "Copyright" block:

> Please respect the <a href="https://www.ted.com/about/our-organization/our-policies-terms" target="_blank">TED Talks Usage Policy</a>

and:

> This dataset is compiled from TED talk subtitles and distributed through phontron.com. The package here includes the training data only (development and test data are not included in this package). The transcripts have been translated by a global community of volunteers to more than 100 languages.

Upstream URL `http://phontron.com/data/ted_talks.tar.gz` is **dead**: it now serves Graham Neubig's site HTML (HTTP 200, 705 bytes, `<title>Graham Neubig</title>`), not a tarball. The source repo is `https://github.com/neulab/word-embeddings-for-nmt`; its README (`https://raw.githubusercontent.com/neulab/word-embeddings-for-nmt/master/Readme.md`) verbatim:

> In order to perform experiments, we collected (during early 2017) a common corpus of TED talks which has been translated into many low-resource languages. Under the [Open Translation project](https://www.ted.com/participate/translate), TED talks transcripts are available for more than 2400 talks in 109 languages.

There is **no LICENSE file** in that repo (`https://raw.githubusercontent.com/neulab/word-embeddings-for-nmt/master/LICENSE` → 404). It asks only for a citation (Ye et al., HLT-NAACL 2018). The paper is "When and Why are Pre-trained Word Embeddings Useful for Neural Machine Translation?" (Ye, Sachan, Felix, Padmanabhan, Neubig).

**(c) Commercial / redistribution / modification.** No license granted by NeuLab; the data is TED volunteer subtitles → TED's CC BY-NC-ND 4.0 / Usage Policy govern (see §7).

**(d) Gotchas.** Original distribution URL is gone, so no upstream LICENSE/README can be inspected even in principle; the only rights statement is TED's.

**(e) Verdict: NOT OK (derived from TED volunteer subtitles, no independent license; TED CC BY-NC-ND 4.0 applies — no commercial use, no derivatives).**

---

## Summary table

| # | Target | Upstream license (verbatim gist) | Commercial | Redistribute in closed app | Modify | Verdict |
|---|---|---|---|---|---|---|
| 1 | News-Commentary v16 | WMT README: packaging under **CC0** ("no rights reserved"); "We do not own any of the text" + ParaCrawl take-down | Yes (packaging) | Yes (packaging) | Yes (packaging) | **OK with attribution/no-obligation packaging — but underlying news text is third-party copyrighted; take-down-only basis. Not clean.** |
| 2 | WMT-News v2019 | WMT19 task page: "can be freely used for **research purposes** … For other uses … consult with original owners" | No | No | No | **NOT OK (research only; newspaper copyright)** |
| 3 | MultiUN | No license stated; compiler papers: UN documents "in public domain according to … ST/AI/189/Add.9/Rev.2"; site "all rights reserved" | Argued yes (UN PD) | Argued yes | Argued yes | **OK with attribution (UN docs public domain; no explicit license from compiler)** |
| 4 | UNPC v1.0 | UN page: "official records … in the **public domain**"; "no other restrictions apply"; must acknowledge UN | Yes | Yes | Yes | **OK with attribution (public domain + mandatory UN acknowledgment/citation)** |
| 5 | OpenSubtitles v2018/v2024 | ToS: "**Commercial use prohibited.**"; disclaimer: "Commercial use … protected under U.S. and Foreign Copyright Laws" | No | No | No | **NOT OK (explicit commercial prohibition + movie/user copyright)** |
| 6 | QED v2.0a | QCRI: "made public for **RESEARCH purpose only**"; "All rights reserved" | No | No | No | **NOT OK (research only)** |
| 7 | TED2020 v1 / TED2013 | TED: **CC BY-NC-ND 4.0**; commercial use requires a license; 2024 ToU bars ML datasets | No | No | No | **NOT OK (NC + ND; AI/ML dataset bar)** |
| 8 | NeuLab-TedTalks v1 | No license; OPUS + repo point to TED policy; data URL dead | No | No | No | **NOT OK (TED CC BY-NC-ND 4.0 applies)** |

### Bottom line for a commercial offline app shipping zh pairs
- **Clearable with attribution:** UNPC (§4) and, with more caveats, MultiUN (§3).
- **Packaging-licensed but text-risky:** News-Commentary (§1) — CC0 packaging, third-party news text.
- **Do not ship:** WMT-News (§2), OpenSubtitles (§5), QED (§6), TED2020/TED2013 (§7), NeuLab-TedTalks (§8).
- OPUS's own "we do not own the text / take-down on request" disclaimer is not a license and does not make any of the above commercially usable.
