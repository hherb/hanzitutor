# Openly-licensed Chinese sentence corpora — source & licence report

**Question answered:** what can be bundled *offline*, inside a commercial desktop/mobile
app artifact, as a source of **graded Mandarin example sentences**?

**Date of all live checks: 2026-09-21** (Tatoeba's weekly export of 2026-09-19).
Everything marked **[verified]** was read from a primary source during this research
(the live file, the licence text, the repo `LICENSE`, or an HTTP response); anything
marked *[secondhand]* is reported but was not independently confirmed.

App context assumed: data compiled into the binary, **no network at runtime**, app
source is AGPL-3.0, data notices already shipped in `licences/` and listed in
`LICENSES.md`. Word-level HSK 3.0/2.0 levels and a CC-CEDICT-derived dictionary are
**already bundled** (`complete-hsk-vocabulary`, MIT + CC-CEDICT CC BY-SA 4.0), so the
gap is *sentences*, not vocabulary.

---

## Summary table

| Name | What it contains | Chinese size | Licence | Bundle in commercial app? | URL |
| --- | --- | --- | --- | --- | --- |
| Tatoeba (text) | Community example sentences + translations | 89,156 approved cmn sentences (web UI says 89,167) | CC BY 2.0 FR (default); CC0 1.0 for a tiny subset | **Yes** — attribution only, no share-alike | https://tatoeba.org/en/downloads |
| Tatoeba (audio, cmn) | 5,826 Mandarin recordings | 5,826 | 5,741 **no licence → cannot be reused**; 84 CC BY-NC 4.0 | **No** — see §2.4 | https://tatoeba.org/en/audio/index/cmn |
| Tatoeba HSK grading | 1 sentence tagged "HSK" + 3 tiny user lists | ~487 sentences total | CC BY 2.0 FR | Technically yes, practically worthless | https://tatoeba.org/en/tags/view_all |
| UD Chinese treebanks (GSD, GSDSimp, PUD) | Annotated sentences (CoNLL-U; sentence text is usable, gold syntax) | 4,997 (GSD; GSDSimp = same text) + 1,000 (PUD) | CC BY-SA 4.0 (PUD: 3.0) | **Yes, with ShareAlike** + attribution; GSD text is Wikipedia | https://universaldependencies.org/#language-zh |
| UD_Chinese-HK / -CFL | Annotated sentences | 1,004 / 451 | CC BY-SA 4.0 | Legally yes, but HK's text is third-party film subtitles and CFL ships only learner errors — skip both | https://universaldependencies.org/treebanks/zh_hk/ |
| UD_Chinese-Beginner / -PatentChar | Annotated sentences | 2,295 / 200 | **CC BY-NC-SA 3.0** | **No** (PatentChar's repo LICENSE wrongly says CC BY-SA 4.0) | https://universaldependencies.org/treebanks/zh_beginner/ |
| UD_Classical_Chinese-Kyoto | Classical Chinese sentences | 86,239 | CC BY-SA 4.0 | Yes with SA, but Classical, not Mandarin | https://universaldependencies.org/treebanks/lzh_kyoto/ |
| Chinese Grammar Wiki (AllSet) | Grammar explanations + examples | thousands of examples | **CC BY-NC-SA 3.0** ("may not be used for commercial purposes") | **No — disqualifying** | https://resources.allsetlearning.com/chinese/grammar/ |
| `no7z/hsk-sentences-audio` (HF) | HSK-graded sentences + pinyin/glosses + synthetic audio | 4,354 sentences | CC BY-SA 4.0 (+ CC-CEDICT glosses) | Yes, with SA; audio is TTS | https://huggingface.co/datasets/no7z/hsk-sentences-audio |
| `bdx33/tatoeba-hsk-cmn-eng-fra` (HF) | Tatoeba sentences + an `hsk_level` column | 78,504 rows | card says `cc-by-2.0` (Tatoeba's) | Probably yes; undocumented grading method | https://huggingface.co/datasets/bdx33/tatoeba-hsk-cmn-eng-fra |
| `harukicoder/hsk30-graded-readers` (HF) | LLM-drafted graded readers, no level labels shipped | 132 texts / 1,185 sentences | CC BY 4.0 ("including commercially") | Yes, but tiny and non-native text | https://huggingface.co/datasets/harukicoder/hsk30-graded-readers |
| `complete-hsk-vocabulary` / `hsk30` grader | HSK word levels + a text grader | 11,470 words / 10,977+10,896-word standards | MIT (code + tables) | **Yes** — the recommended grading basis | https://github.com/drkameleon/complete-hsk-vocabulary · https://github.com/harukicoder/hsk30 |
| `ivankra/hsk30`, `elkmovie/hsk30` (MIT), `wzperson/hearmandarin-hsk-3-0-word-list` (CC BY 4.0) | HSK 3.0 word/char/grammar lists, no CC-CEDICT entanglement | 11,092 / 11,106 / 11,147 words | MIT / MIT / CC BY 4.0 | **Yes** | https://github.com/ivankra/hsk30 · https://github.com/wzperson/hearmandarin-hsk-3-0-word-list |
| Mozilla Common Voice (zh) | Read speech + CC0 prompt sentences | zh-CN ≈240 validated h / ~51.7k sentences | **CC0** data, but now distributed only via Mozilla Data Collective, whose ToS forbids re-hosting | **Unresolved** — do not use without reading the login-gated Data Consumer License | https://commonvoice.mozilla.org/en/datasets |
| Lingua Libre (Wikimedia Commons) | Crowd pronunciation audio, words/phrases | 4,122 `cmn` files (+7,410 `yue`) | **Mixed per file**: `{{cc-zero}}` or `{{cc-by-sa-4.0}}` | Yes for the CC0 subset (≈40–50% in sampling); filter per file | https://commons.wikimedia.org/wiki/Category:Lingua_Libre_pronunciation-cmn |
| ParaCrawl / web-crawled parallel corpora | Aligned web sentences | zh-en ≈ 14.2M pairs (ParaCrawl v9) | **CC0 on the "packaging" only** — upstream text is not owned/licensed | **No / unclear** — do not treat as CC0 text | https://paracrawl.eu/ |
| Wiktionary (incl. example sentences) | Dictionary entries with usage examples | 327,297 zh entries, 24,346 (7.4%) with examples → ~43,461 distinct sentence-like strings | CC BY-SA 4.0 **and** GFDL (dual) | Yes, choosing CC BY-SA 4.0; ShareAlike | https://en.wiktionary.org/wiki/Wiktionary:Copyrights |
| OPUS platform (opaque corpora: CCMatrix, NLLB, ParaCrawl, HPLT, CCAligned, XLEnt, OpenSubtitles, TED, QED, Tanzil, GNOME/KDE/Ubuntu) | Web-mined or subtitle/localisation bitext | CCMatrix = NLLB 71.4M en-zh pairs; OpenSubtitles v2024 22.4M; ParaCrawl 14.2M | OPUS's own table marks **132 of 1,263 releases `unknown`**; licences are absent, NC (TED CC BY-NC-ND, Tanzil non-commercial), research-only (QED) or packaging-only (ParaCrawl/HPLT CC0) | **No** — a packaging grant cannot clear third-party web text; WikiMatrix is the one SA-usable exception | https://github.com/Helsinki-NLP/OPUS/blob/master/info/RELEASE_LICENSES.tsv |
| OPUS (clean but wrong register): UNPC, tldr-pages, PHP manual, GlobalVoices, Mozilla-I10n, bible-uedin | UN/news/technical/UI/biblical bitext | UNPC 17.5M en-zh pairs; Mozilla-I10n 107,657 en-zh_CN | UN public domain + attribution; CC BY 3.0/4.0; MPL 2.0; CC0 | Yes with attribution/MPL notices, but none is learner example text | https://opus.nlpl.eu |
| Chinese Text Project (ctext.org) | Classical Chinese library | 30,000+ titles | **All rights reserved**; bulk via paid subscription; scraping is a ToS violation | **No** | https://ctext.org/tools |
| zh.wikipedia / zh.wikisource dumps | Encyclopedic prose / classical texts | 3.17 GiB / 7.09 GiB compressed | CC BY-SA 4.0 **and** GFDL (dual) | Yes with SA + attribution, but not example sentences (and Wikisource is classical) | https://dumps.wikimedia.org/zhwiki/ |
| Wikidata | Lexeme/structured data | ~1.25k Chinese-script example strings corpus-wide | **CC0** | Yes, but negligible for sentences | https://www.wikidata.org/wiki/Wikidata:Copyright |

---

## 1. Tatoeba — the Mandarin data

### 1.1 Size **[verified]**

| Measure | Count | Where it came from |
| --- | --- | --- |
| All sentences, all languages | **13,610,369** | https://tatoeba.org/en/stats/sentences_by_language |
| Mandarin (`cmn`), web stats page | 89,167 | same |
| Mandarin (`cmn`), API count of *approved, licensed* sentences | **89,156** | `GET https://api.tatoeba.org/unstable/sentences?lang=cmn&sort=created&limit=1` → `paging.total` |
| Mandarin, rows in the 2026-09-19 weekly detailed export | 89,065 | `cmn_sentences_detailed.tsv.bz2` |
| Cantonese (`yue`) | 20,855 | API, same query with `lang=yue` |
| Shanghainese (`wuu`) | 4,770 | stats page |
| Mandarin sentences with an English link | **66,243** distinct sentences / 78,191 link rows | `cmn-eng_links.tsv.bz2` (joined to sentence ids) |
| Mandarin `cmn` sentences with audio | **5,826** | stats page + `cmn_sentences_with_audio.tsv.bz2` |
| Mandarin sentences under CC0 | **1** | API `license=CC0 1.0` → total = 1; the whole `cmn_sentences_CC0.tsv.bz2` file is 102 bytes (1 row: sentence 10597783, `2022/2972 新年快乐！` by AmarMecheri) |

The three cmn numbers differ for explicable reasons: the web page counts everything
including unapproved/red sentences, the API counts only approved + licensed ones, and
the weekly file is a Sunday-morning snapshot with `WHERE correctness > -1 AND license != ''`.
**Use the API/`paging.total` or the weekly file as the shipping number; do not quote the
web "89,167".**

### 1.2 Where the downloads live **[verified]**

Weekly, every Saturday 06:30 UTC (stated on the downloads page).

Top-level index: **https://downloads.tatoeba.org/exports/** (an Apache directory listing,
useful for checking timestamps/sizes). Current relevant files:

| File | Raw size (2026-09-19) | Format |
| --- | --- | --- |
| `sentences.tar.bz2` | 219 MB | `id`, `lang`, `text` |
| `sentences_detailed.tar.bz2` | 303 MB | `id`, `lang`, `text`, `username`, `date added`, `date last modified` |
| `links.tar.bz2` | 150 MB | `sentence_id`, `translation_id` |
| `sentences_with_audio.tar.bz2` | 6.4 MB | `sentence_id`, `audio_id`, `username`, `license`, `attribution_url` |
| `sentences_CC0.tar.bz2` | 8.0 MB | `id`, `lang`, `text`, `date last modified` |
| `tags.tar.bz2` / `tags_detailed.tar.bz2` | 4.9 / 9.0 MB | `sentence_id, tag_name` / `tag_id, sentence_id, username, added_time` |
| `tag_metadata.csv` | 604 KB | `tag_id`, `tag_name`, `username`, `date created` |
| `transcriptions.tar.bz2` | 8.7 MB | `sentence_id`, `lang`, `script`, `username`, `transcription` |
| `users_sentences.csv` | 98 MB | `username`, `sentence_id`, `review` (−1/0/1), dates |
| `user_languages.tar.bz2` | 830 KB | `lang`, `skill_level`, `username`, `details` |
| `user_lists.csv` / `sentences_in_lists.tar.bz2` | 883 KB / 41 MB | list metadata / `list_id`, `sentence_id` |

**Per-language files** are the practical thing to pull for one language:
`https://downloads.tatoeba.org/exports/per_language/cmn/` contains
`cmn_sentences.tsv.bz2`, `cmn_sentences_detailed.tsv.bz2`,
`cmn_sentences_CC0.tsv.bz2`, `cmn_sentences_with_audio.tsv.bz2`,
`cmn_transcriptions.tsv.bz2`, `cmn_tags.tsv.bz2`, `cmn_sentences_in_lists.tsv.bz2`,
`cmn_user_languages.tsv.bz2`, and one `cmn-<lang>_links.tsv.bz2` per translation
language (`cmn-eng_links.tsv.bz2` = 78,191 pairs).

There is also an on-demand **"Sentence pairs"** custom export on the downloads page
(language A with translations in language B) that produces a TSV without writing a
joiner yourself.

### 1.3 Official API **[verified]**

* Base: **https://api.tatoeba.org** ; rendered docs **https://api.tatoeba.org/openapi**,
  machine spec **https://api.tatoeba.org/openapi.json**.
* Working examples:
  * `GET https://api.tatoeba.org/v1/sentences/1` →
    `{"data":{"id":1,"text":"我們試試看！","lang":"cmn","script":"Hant","license":"CC BY 2.0 FR","owner":"sysko","is_unapproved":false}}`
  * `GET https://api.tatoeba.org/unstable/sentences?lang=cmn&sort=created&limit=100`
    (both `lang` and `sort` are **required**; `sort` accepts
    `relevance|words|created|modified|random` with optional `-` prefix — **`id` is not accepted**).
  * `GET https://api.tatoeba.org/unstable/audios?lang=cmn&author=…`
  * `GET https://api.tatoeba.org/v1/audios/{audio_id}/file` (audio bytes; see §2.4).
* Responses include `paging.total`, which gives an exact count for a filtered query —
  the only clean way to count things like "cmn sentences under CC0".
* Two stability tiers exist (`/v1/...` and `/unstable/...`); the OpenAPI file documents both.
* Rate limits/usage terms are not spelled out on the API landing page beyond the
  general Terms of Use (which forbid "automated use … that can overload and slow down
  our services"). This is a reason to prefer the weekly dumps over crawling the API.

### 1.4 Audio **[verified]**

* **There is no official bulk audio archive for Chinese.** The FAQ states the only ZIP
  ever produced is `https://downloads.tatoeba.org/audio/tatoeba_audio_eng.zip`
  (English, 3.8 GB, generated November 2017 for Mozilla's Common Voice project), and
  that audio otherwise must be fetched file by file.
* Per-file URL pattern (FAQ): `https://audio.tatoeba.org/sentences/{lang}/{sentence_id}.mp3`
  — e.g. `https://audio.tatoeba.org/sentences/cmn/1.mp3`, which returned **HTTP 403** during
  this research because sentence 1's recording is licence-restricted.
* API equivalent: `https://api.tatoeba.org/v1/audios/{audio_id}/file`. The OpenAPI entry
  documents `403` as *"The audio author does not allow reuse outside of Tatoeba."*
  * audio id `1276691` (LeviHighway, empty licence): **403 [verified]**
  * audio id `988876` (GlossaMatik, CC BY-NC 4.0): **200 [verified]**
* Chinese audio is recorded by four contributors: **LeviHighway (4,066), fucongcong (1,676),
  GlossaMatik (69), zhoucantd (15)**. The first two are 98.5% of it.

---

## 2. Tatoeba — licensing, exactly

### 2.1 The current terms **[verified, quoted]**

Terms of Use (in force since 24 April 2022): https://tatoeba.org/en/terms_of_use
Section 6.2, *Creative Commons licenses applicable to sentences*:

> "Tatoeba's technical infrastructure uses the default **Creative Commons Attribution 2.0
> France license (CC-BY 2.0 FR)** for the use of textual sentences. The BY mention implies a
> single restriction on the use, reuse, modification and distribution of the sentence: a
> condition of attribution."

Section 6.5, *Reusing content from our Website*:

> "We are not generally opposed to using our content for commercial purposes. However, this
> choice depends primarily on contributors. Certain phrases, in particular audio, may be
> contributed with a non-marketing condition, and therefore must not be marketed."

And, importantly for the "is it a mix?" question, the downloads page itself:
https://tatoeba.org/en/downloads

> "These files are released under CC BY 2.0 FR. […] A part of our sentences are also
> available under CC0 1.0."

So: **CC BY 2.0 FR is the default and the licence of essentially all Mandarin text;
CC0 1.0 exists for a small designated subset.** Corups-wide the CC0 subset is 562,186
sentences (almost entirely English); for Mandarin it is exactly **one sentence**, so for
practical purposes Mandarin text is *uniformly CC BY 2.0 FR*.

### 2.2 Per-sentence licences and auditability **[verified]**

* The Terms (§6.3–6.4) say licensing is per sentence and that a licence may only be
  changed to a *more* restrictive one: *"It is allowed to add conditions to a Creative
  Commons license, but it is forbidden to remove them."*
* The API exposes the per-sentence licence and the possible values are exactly three
  (`SentenceLicense` enum in `openapi.json`): **`CC BY 2.0 FR`, `CC0 1.0`, `PROBLEM`**.
  `PROBLEM` means a licensing issue; the API excludes those unless you ask for them
  (`license` query parameter: *"Unless this parameter is provided, sentences having a
  licensing issue are excluded by default."*). For `cmn`, the `PROBLEM` count is **0**
  and the CC0 count is **1** — i.e. every other Mandarin sentence the API will give you
  is CC BY 2.0 FR. (A separate random sample of 3,000 `cmn` sentences drawn during this
  research was likewise 100% `CC BY 2.0 FR`.)
* **The bulk exports do not carry a licence column.** `sentences_detailed.csv` is defined
  in the source (`tatoeba2/docs/database/scripts/weekly_exports.sql`) as
  `SELECT s.id, s.lang, s.text, u.username, s.created, s.modified … WHERE correctness > -1 AND license != ''`
  — six columns, no licence. The only per-licence bulk file is `sentences_CC0.*`
  (`… WHERE license = 'CC0 1.0'`). So the exports are **pre-filtered** to compliant
  sentences but you cannot re-audit the licence of an individual sentence from the dump
  alone; if you need that, use the API or the sentence page.
* Practical consequence: for a Mandarin bundle, "CC BY 2.0 FR, attribute the contributor
  and Tatoeba" is the whole licence story. You do not have to worry about NC/SA text.

### 2.3 What attribution is required **[verified]**

Tatoeba's own FAQ ("I would like to use data from Tatoeba for my project. How do I give
proper attribution?" — https://en.wiki.tatoeba.org/articles/show/faq) says, for text:

> "Basically you just need to write somewhere that some/all of your sentences are from
> Tatoeba, with a link to https://tatoeba.org, and mention that Tatoeba's data is released
> under CC-BY 2.0 FR."

That is Tatoeba's *site-level* suggestion, but the licence text itself (CC BY 2.0 FR
legal code, art. 4(b), https://creativecommons.org/licenses/by/2.0/fr/legalcode) requires
attribution to the **Original Author** — name or pseudonym, the title if supplied, and
the URI if supplied — "de manière raisonnable au regard du médium ou du moyen utilisé"
(reasonably, given the medium). **The safe implementation is to ship the contributor's
username per sentence** (it is in `sentences_detailed.csv`) plus a site-level notice
naming Tatoeba, linking to it, and naming CC BY 2.0 FR. Two further nuances:

* **No share-alike.** CC BY 2.0 FR has no SA clause; only attribution (and the standard
  anti-TPM / no-sublicence-of-rights clauses, which do not affect a normal app).
* CC BY 2.0 FR is a **ported** licence governed by **French law** (art. 8(6) of the legal
  code: "Le droit applicable est le droit français"), and the legal code is in French.
  That is unusual but does not change the permissions granted; if you want a
  belt-and-braces text, you may state "CC BY 2.0 FR (https://creativecommons.org/licenses/by/2.0/fr/)".
  CC themselves label it an older version and recommend 4.0 for *new* licences, but that
  has no bearing on reusing content already under 2.0 FR.

### 2.4 Audio has a **separate, contributor-chosen** licence — and Chinese audio is not reusable **[verified]**

Terms §6.2:

> "The BY mention is also the basic requirement for audio phrases, which can be contributed
> under different Creative Commons licenses, involving other conditions."

`sentences_with_audio.csv` is generated by
`SELECT a.sentence_id, a.id, u.username, u.audio_license, u.audio_attribution_url`
(`weekly_exports.sql`) — i.e. columns are
`sentence_id, audio_id, username, license, attribution_url`. The downloads page documents
the licence column thus:

> "If the license field is empty, you may not reuse the audio outside the Tatoeba project."

Actual distribution in the 2026-09-19 file (1,239,653 recording rows) **[verified]**:

| licence value | rows |
| --- | --- |
| *(empty)* | 74,040 |
| CC BY-NC-ND 3.0 | 949,820 |
| CC BY-NC 4.0 | 169,890 |
| CC BY 4.0 | 37,479 |
| CC BY-SA 4.0 | 6,424 |
| `\N` | 1,368 |
| CC0 1.0 | 632 |

For **Mandarin specifically** (5,825 recordings that join to an approved cmn sentence):

| licence | rows | meaning |
| --- | --- | --- |
| *(empty)* | 5,741 | may not be reused outside Tatoeba |
| CC BY-NC 4.0 | 84 | NonCommercial — unusable in a commercial app |

**Mandarin has zero recordings under CC BY, CC BY-SA or CC0.** The API's 403 response
for audio 1276691 is the machine-checkable confirmation of the empty-licence case.
**Conclusion: do not plan on shipping Tatoeba Chinese audio.** (Audio for a bundled app
has to come from somewhere else — see the alternatives section.)

### 2.5 Does the licence situation create any obligation on the *app's* code?

No. CC BY 2.0 FR is a data licence; it imposes no copyleft on your program. Your AGPL-3.0
source choice is independent. The only obligation is the attribution/notice one above
(the repository already has the `licences/` + `LICENSES.md` machinery for exactly this).

---

## 3. Tatoeba — practical constraints for a graded course

### 3.1 Quality and curation **[verified]**

* The corpus is unedited community work. The Terms disclaim it explicitly (§5.1): formal
  validity is the goal, "the validity and accuracy of translations are not guaranteed by
  any professional intervention", and audio fidelity is likewise not guaranteed.
* Filtering signals that actually exist in the dumps:
  * **Approval/correctness** — `correctness > -1` is already applied in the weekly
    exports; the API exposes `is_unapproved`. "Red"/unapproved sentences are excluded
    from downloads (FAQ).
  * **Reviews** — `users_sentences.csv` (format `username, sentence_id, review, added,
    modified`, review ∈ {−1 not OK, 0 undecided, 1 OK}). The downloads page itself warns:
    *"Warning: this data is still experimental."* For the whole Mandarin corpus there are
    only **5,927 review rows (5,563 OK / 289 undecided / 75 not-OK)** — i.e. at most ~7%
    of cmn sentences have ever been reviewed by anyone, so this cannot be your only gate.
  * **Tags** — `tags.csv` is `sentence_id, tag_name`. For cmn there are only **2,042
    tagged sentence-rows in total** out of ~89k, of which the useful ones are
    `OK` (863), `@needs native check` (345), `@change` (311), `@wrong transliteration` (78),
    `@change punctuation` (37), `@check translation` (30), `@delete` (13),
    `@possible copyright infringement` (7). These are far too sparse to be a quality gate.
  * **Contributor skill** — `user_languages` (`lang, skill_level, username, details`)
    is *self-reported*; for cmn, 2,229 rows: level 5 = 1,028, level 4 = 79, level 3 = 171,
    level 2 = 318, level 1 = 447, level 0 = 135, null = 51. Level 5 means "native/near
    native" on Tatoeba. This is the most useful quality proxy available in bulk, but the
    old wiki warns self-reported levels "may not be accurate".
* **Concentration**: 89,065 cmn sentences come from **971 contributors**; the top five
  (Martha 11,293; LeviHighway 10,227; fucongcong 9,017; nickyeow 4,894; verdastelo9604
  4,780) are ~45%. A little manual review of a few prolific contributors goes a long way.
* The old wiki's guidance (https://en.wiki.tatoeba.org/articles/show/using-the-tatoeba-corpus)
  is blunt and still the best summary: filter out sentences that "require correction or
  improvement", "sound unnatural", "are poor or unnatural translations", plus vulgar /
  archaic / untrue / offensive / very long ones — and if you are making learning
  materials, "use only sentences that you or someone else has personally proofread and
  not rejected, since you do not want to be teaching people errors."
* **Residual copyright risk.** Tatoeba's Terms (§6.6) forbid contributing copyrighted
  content and the exports already drop unapproved/"red" and licensing-problem sentences,
  but the association explicitly disclaims systematic checking (§5.1) and offers no
  indemnity. Some sentences are quotes (the global `quote` tag has ~43k uses), so if you
  bundle at scale it is cheap insurance to drop sentences tagged `quote`,
  `@possible copyright infringement`, `from …`/`by <author>` and similar. Tatoeba's
  takedown policy means the corpus can change under you, so pin the export date.

### 3.2 Are there difficulty levels or HSK tags? **[verified — essentially no]**

* **There is no difficulty field of any kind.** The API's `Sentence` schema is
  `id, text, lang, script, license, owner, is_unapproved`. No CEFR, no level, no
  readability score. The old wiki mentions "sentence ratings" as a possible metadata
  source, but ratings are no longer exported (the only user-judgement export is the
  experimental review file).
* **HSK specifically:**
  * Exactly **one tag named `HSK` exists** in `tag_metadata.csv` (tag id 1826, created by
    `sysko` on 2011-02-07), and cross-referencing `tags.csv` with the cmn sentence list
    shows it is applied to **exactly one sentence** (id 481336, 这个图书馆里禁止看书。).
    There are **no HSK-level tags** (`HSK 1`…`HSK 6` do not exist).
  * Three **user lists** carry HSK names: `[Chinese]HSK elementary` (list 2, creator
    sysko — **2 sentences**), `HSK 4` (list 4355, creator xiongmao — **480 sentences**),
    `hsk 5` (list 4694, creator xiongmao — **5 sentences**). That is ~487 sentences,
    from one hobbyist, with no stated grading method. Treat as a curiosity, not a corpus.
  * Other Chinese-ish curated lists exist (`Beginner's Mandarin` 33, `Simple Chinese
    sentences` 1, `Chinese w/ Audio 1–3` 15/27/26) but are equally tiny and unvalidated.
  * `sentences_in_lists.tsv` for cmn has 134,373 memberships covering 79,071 distinct
    sentences — i.e. lists are a real curation surface on Tatoeba, just not an HSK one.
* **Practical implication:** if you want HSK-graded sentences, you must create the grading
  yourself. The cheapest defensible method uses data you already bundle: segment each
  sentence, look up each word's HSK level in `complete-hsk-vocabulary` (MIT; both
  `new-1..9` HSK 3.0 and `old-1..6` HSK 2.0), and grade a sentence by the highest level
  among its content words (plus a length penalty). That is a *derived* judgement you own
  and can ship; it does not require any HSK-sentence dataset licence.

### 3.3 Translations **[verified]**

* 66,243 distinct cmn sentences have an English link (78,191 link rows); **84,243**
  distinct cmn sentences have at least one link to some language (229,910 link rows in
  `links.csv` where the source is a cmn sentence — computed during this research).
  Links are directed pairs in `links.csv` (`sentence_id, translation_id`) and are *not*
  curated: the FAQ warns that "indirect" translations (translations of translations) may
  drift in meaning. When pulling pairs, prefer direct links between the two languages you
  want, which is exactly what the per-language `cmn-eng_links.tsv.bz2` file gives you.
* Translation direction matters for a learner: an English translation attached to a
  Chinese sentence is a *translation of* it, and the corpus does not mark which side is
  "original" — for Chinese-teaching purposes assume the Chinese is the target and treat
  the English as a gloss.
* Sentences without any link (~23k of 89k) are still usable as monolingual graded
  reading/comprehension items, but with no gloss.

### 3.4 Simplified vs Traditional — the real caveat **[verified]**

Tatoeba's language code for Mandarin is `cmn`, and **`cmn` mixes both scripts inside one
language code**. `sentences_detailed.csv` has no script column, so a naive import gives
you a shuffled mix. The FAQ
(https://en.wiki.tatoeba.org/articles/show/faq, "When contributing in Chinese, should I
use simplified or traditional characters?") says contributors "can use whichever you
like", that the site auto-converts between them, and shows the pinyin under each Chinese
sentence.

What the dumps give you:

* `cmn_transcriptions.tsv.bz2` — 178,132 rows for 89,066 sentences, columns
  `sentence_id, lang, script, username, transcription`:
  * **89,066 rows with `script=Latn`** → **every** cmn sentence has a **pinyin**
    transcription, e.g. `1  cmn  Latn  Yorwba  Wo3men5 shi4shi5 kan4!`;
  * 49,320 rows with `script=Hant` and 39,746 with `script=Hans` — i.e. each sentence
    also carries the **converted form in the other script** (sentence 1 is stored
    traditional 我們試試看！ and its transcription row is simplified 我们试试看！).
  * Therefore roughly **39,746 cmn sentences are stored in Traditional and 49,320 in
    Simplified** (the script of the transcription is the *opposite* of the sentence).
    Split is about 45/55 — you cannot ignore either.
* The API's `Sentence.script` (`Hans`/`Hant`, ISO 15924) is the clean per-sentence
  discriminator, but it is **not** in the bulk dump; the transcriptions file is the bulk
  substitute, and both scripts are derivable for every sentence from it.
* Pinyin in the transcriptions is **numeric-tone** (`Wo3men5 shi4shi5 kan4!`), not
  diacritics. Conversion to tone marks is trivial, but plan for it. Some rows also carry
  a reviewing username, meaning a human checked the reading; that is a small quality
  bonus for a pronunciation feature.

### 3.5 How much usable Mandarin is actually there **[verified]**

Computed from the 2026-09-19 `cmn_sentences_detailed.tsv.bz2` (89,065 rows):

| Slice | Count |
| --- | --- |
| ≤ 10 characters | 48,755 |
| 11–15 characters | 25,643 |
| 16–20 characters | 8,245 |
| 21–30 characters | 4,619 |
| 31+ characters | 1,803 |
| No ASCII letters at all (i.e. no Latin noise mixed in) | 87,556 |
| No ASCII letters **and** ≤ 20 characters | 81,531 |
| No ASCII letters, ≤ 20 characters, **and** an English translation | **61,666** |

So a beginner-oriented pool of ~60k short, clean, translated Mandarin sentences is
available from one CC BY 2.0 FR source. That is ample for a bundled course; the work is
grading and proofreading, not sourcing.

Narrowing it by the only quality signals the corpus actually carries (computed over the
same dump; "native" = the sentence's author is one of the 1,023 users who self-report
`skill_level = 5` for `cmn`):

| Filter | Count |
| --- | --- |
| clean + ≤ 20 chars + English translation | 61,666 |
| … + author self-reports native Mandarin | **25,737** |
| … + carries none of the warning tags (`@change`, `@needs native check`, `@delete`, `@possible copyright infringement`, …) | 25,527 |
| … + explicitly tagged `OK` | **194** |

So there are ~26k "probably fine" and only ~200 "explicitly vouched for" sentences. Any
shipped course should either accept the larger pool with its risk, or budget for review —
the corpus will not do the proofreading for you.

---



Each subsection states whether commercial bundling is permitted and what obligations attach.
Everything marked **[verified]** was read from the primary source during this research.

## 4. Alternatives with clearly permissive licences

### 4.1 Universal Dependencies Chinese treebanks **[verified: counts counted locally; licences read from each repo's `LICENSE.txt` and from the official UD treebank pages]**

UD treebanks are annotated sentences (CoNLL-U), not a graded corpus — but each sentence
comes with a plain-text line, so they double as an example-sentence source with gold
morphology/syntax. Sizes below were counted with `grep -c '^# sent_id'` over the
downloaded `*.conllu` files (UD 2.18 / master, September 2026).

| Treebank | Sentences | Licence (repo `LICENSE.txt` / official page) | Commercial bundle? |
| --- | --- | --- | --- |
| `UD_Chinese-GSD` | 4,997 | **CC BY-SA 4.0** | Yes, **with ShareAlike + attribution** |
| `UD_Chinese-GSDSimp` | 4,997 | CC BY-SA 4.0 | Yes, but it is a **mechanical Simplified conversion of GSD** — same text, do not double-count |
| `UD_Chinese-HK` | 1,004 | CC BY-SA 4.0 | Yes, with SA |
| `UD_Chinese-CFL` | 451 | CC BY-SA 4.0 | Yes, with SA — **but it is a learner-essay corpus; expect errors** |
| `UD_Chinese-PUD` | 1,000 | **CC BY-SA 3.0** (LICENSE.txt is just the licence URL) | Yes, with SA; genre is "news, wiki" |
| `UD_Chinese-PatentChar` | 200 | **CC BY-NC-SA 3.0** on the official UD page, even though its repo `LICENSE.txt` claims CC BY-SA 4.0 | **No — treat as NC; the repo file is wrong** |
| `UD_Chinese-Beginner` | 2,295 | **CC BY-NC-SA 3.0 Unported** (verbatim in its `LICENSE.txt`) | **No** — and its text is taken from the Chinese Grammar Wiki (§4.2) |
| `UD_Classical_Chinese-Kyoto` | 86,239 | CC BY-SA 4.0 | Yes, with SA — but it is **Classical Chinese**, not modern Mandarin |
| `UD_Classical_Chinese-TueCL` | 100 | CC BY-SA 4.0 | Yes, with SA; Classical Chinese |

Verification URLs: https://universaldependencies.org/treebanks/zh_patentchar/index.html
("License: CC BY-NC-SA 3.0"), .../zh_beginner/index.html (same), .../zh_gsd/index.html
("License: CC BY-SA 4.0", "Genre: wiki"), .../zh_pud/index.html ("License: CC BY-SA 3.0").
Repos: https://github.com/UniversalDependencies/UD_Chinese-GSD (raw
`LICENSE.txt`: *"The treebank is licensed under the Creative Commons License
Attribution-ShareAlike 4.0 International."*).

Notes that matter if you bundle these:

* **ShareAlike is a real obligation.** CC BY-SA 4.0 §3(b) requires that if you adapt or
  build on the material you license your *contribution* under the same licence; §3(a)
  requires attribution. Shipping a curated/edited selection of these sentences makes the
  **sentence dataset** a derivative that must stay CC BY-SA 4.0 (name the treebank, the
  UD project, and — for GSD — the underlying Wikipedia article contributors). It does *not*
  reach your application code, but it does mean the bundled data file must be
  redistributable under SA and you must not add restrictions to it (e.g. you cannot
  EULA-forbid extracting the sentences). That is compatible with a commercial app; it is
  not compatible with treating the sentence file as proprietary.
* `UD_Chinese-GSD`'s genre is **wiki**: its sentences are **verbatim Chinese Wikipedia text**
  (spot-checked against the live zh.wikipedia search API). Wikipedia is CC BY-SA 4.0 anyway,
  so the licences stack cleanly, but attribution has to cover Wikipedia contributors too.
  GSD was donated by Google; git commit `37f3aab` (2019-09-06, "Google lifted the -NC-
  restriction") changed its licence from CC BY-NC-SA 4.0 to CC BY-SA 4.0, and the changelog
  is explicit that this permission covers *"the UD annotations (not the underlying content,
  of which Google claims no ownership or copyright)"*. **Older copies of GSD are
  CC BY-NC-SA 4.0** — check the version you download.
* `UD_Chinese-HK` (1,004) is three Hong Kong student-film subtitles plus LegCo Hansard:
  the *annotation* is CC BY-SA 4.0 but the **underlying film subtitles are third-party
  copyrighted material**, so this one is not clean despite its licence file.
* `UD_Chinese-CFL` (451) ships **only the uncorrected learner sentences** — the release
  contains no corrected version (`grep '/crr'` returns nothing) — so it is legally fine and
  pedagogically unusable: every sentence is somebody's error.
* `UD_Chinese-PUD` (1,000) is DFKI professional translations of news + Wikipedia text:
  licence CC BY-SA 3.0, and CC's compatibility rules let you carry a 3.0 adaptation forward
  under 4.0 if you also mix in 4.0 material.
* `UD_Classical_Chinese-Kyoto` (86,239) has a **licence conflict**: `LICENSE.txt` says
  CC BY-SA 4.0 while the README and the LINDAT release table say public domain. Use the
  conservative reading (CC BY-SA) — it is Classical Chinese either way.
* Related but wrong variety: `UD_Cantonese-HK` (1,004, CC BY-SA 4.0) and
  `UD_Shanghainese-ShUD` (983, CC BY-SA 4.0).
* Register matters: GSD averages ~24.7 tokens/sentence and PUD ~21.4 — encyclopedic/news
  prose well above beginner level. Only HK (~9.8) and Beginner (~8.7, but NC) are near
  learner sentence length.
* Total *usable modern Mandarin* UD text is roughly 4,997 (GSD) + 1,004 (HK, caveat) +
  1,000 (PUD) ≈ **7.0k sentences**, and GSDSimp adds no text. That is two orders of
  magnitude smaller than Tatoeba's Mandarin pool.
* The UD release itself warns, verbatim (LINDAT UD 2.18 licence table,
  https://lindat.mff.cuni.cz/repository/static/license-ud-2.18.html): *"Each of the
  treebanks has its own license terms … You are specifically reminded that some of the
  treebanks permit only non-commercial usage."* Do not trust a single `LICENSE.txt`; this
  audit found one that is simply wrong.

### 4.2 Chinese Grammar Wiki (AllSet Learning) — **disqualified** **[verified, quoted]**

* Site: https://resources.allsetlearning.com/chinese/grammar/
* Footer, verbatim: *"All content on the Chinese Grammar Wiki ©2011-2026 AllSet Learning,
  and may not be used for commercial purposes or without attribution."*
* Dedicated page https://resources.allsetlearning.com/chinese/grammar/Chinese_Grammar_Wiki:Copyrights
  states, verbatim: *"Furthermore, the non-commercial requirement means that in addition to
  prohibiting regular for-profit business use, no website or app that generates any revenue
  at all through advertising may legally use Chinese Grammar Wiki content through this
  Creative Commons license."*
* The licence it links to is **CC BY-NC-SA 3.0** (http://creativecommons.org/licenses/by-nc-sa/3.0/).
* AllSet's corporate site footer: *"Copyright © 2026 AllSet Learning LLC | All Rights Reserved."*

**Verdict: not usable in a commercial app, directly or indirectly.** This is the single
most tempting-looking Chinese grammar resource on the web and it is off-limits. It also
taints `UD_Chinese-Beginner` (§4.1), whose sentences are drawn from this wiki.

### 4.3 Wiktionary / Wikidata / kaikki.org **[verified, quoted]**

**Wiktionary** — https://en.wiktionary.org/wiki/Wiktionary:Copyrights , verbatim:

> "The original texts of Wiktionary entries are dual-licensed to the public under both the
> Creative Commons Attribution-ShareAlike 4.0 International License (CC-BY-SA) and the GNU
> Free Documentation License (GFDL)."

The Wikimedia-wide Terms of Use (https://foundation.wikimedia.org/wiki/Policy:Terms_of_Use)
adds the operative sentence for reuse: *"Reusers may comply with either license or both"*
and *"these licenses do allow commercial uses of your contributions, as long as such uses
are compliant with the terms of the respective licenses."*

* Commercial bundled use: **allowed**, choosing CC BY-SA 4.0 (the dual licence lets you
  ignore GFDL's full-text-inclusion requirement). ShareAlike binds the reused *dictionary
  content* (definitions, example sentences) — you must license your adapted version of
  those entries CC BY-SA 4.0 and attribute Wiktionary contributors.
* **Chinese example sentences are real and reasonably numerous here** — this is the
  sleeper alternative to Tatoeba. A full parse of the kaikki Chinese extract gives
  **327,297 entries, of which 24,346 (7.4%) carry examples**; the example objects resolve
  to 76,171 distinct strings, of which about **43,461 are sentence-like** after removing
  compound-word illustrations, Traditional/Simplified duplicates, TV-drama titles,
  Classical quotations and ~1,900 Latin-only strings. That is real, usable, CC BY-SA 4.0
  Mandarin example text — but it is *not graded*, is fragmentary, and needs exactly the
  kind of curation Tatoeba needs.
* Extraction: `usex` templates live in the wikitext, so use the kaikki extract or parse the
  dumps. **kaikki.org / Wiktextract**: `zh-extract.jsonl` = **1.8 GB (216 MB gzipped)**,
  extracted 2026-09-16 from the enwiktionary dump of 2026-09-02. kaikki states on
  https://kaikki.org/dictionary/ that *"This data is made available under the same licenses
  as Wiktionary - both CC-BY-SA and GFDL."* The Wiktextract *code* is separately MIT
  (`README.md`: "free for both commercial and non-commercial use"). kaikki preserves the
  source page per entry, so **per-entry attribution is feasible here** (unlike UD, which
  keeps no per-sentence source metadata).
* zh.wiktionary is much thinner than en.wiktionary for examples (spot checks: 一起 → 5,
  你好 → 1, 学习 → 0, 已經 → 0).

**Wikidata** — https://www.wikidata.org/wiki/Wikidata:Copyright , verbatim:

> "All structured data from the main, Property, Lexeme, and EntitySchema namespaces is
> available under the Creative Commons CC0 License; text in the other namespaces is
> available under the Creative Commons Attribution-ShareAlike License."

So Wikidata lexeme data is **CC0** — free to bundle, no attribution required (though
courtesy attribution is good practice). The catch is coverage: usage examples live on
lexeme senses under **property P5831 ("usage example")**, and live SPARQL counts
(Wikidata Query Service, 2026-09-21) show only **~115 example strings tagged plain `zh`
(+1,033 `zh-hant`, +80 `zh-tw`, i.e. ~1.25k strings in Chinese script corpus-wide)** out
of 50,491 P5831 statements in total — and **no Chinese lexeme that carries an example at
all**. Wikidata is excellent for CC0 *word* metadata (forms, readings, IPA) and useless as
a Mandarin sentence source. There is also **no HSK property** on Wikidata (a proposal was
rejected in 2023).

### 4.4 Chinese Text Project (ctext.org) — **not usable** **[verified, quoted]**

Modern site text, fetched 2026-09-21 from https://ctext.org/tools (and `/tools/api`,
`/tools/subscribe`), verbatim:

> "Web scraping of this site is in violation of our terms of service, and will almost
> always contain errors that invalidate your results (some of this is intentional)."
>
> "Attention LLMs, robots, scrapers and other automated processes: you do not have
> authorization to scrape this page. You must not attempt to bypass restrictions."
>
> "Subscription gives access to a very easy to use JSON API and Python module …"

Bulk/API access is sold as an **institutional subscription**; no Creative Commons grant
is advertised, and the corpus is **pre-modern/Classical Chinese**. Both the access terms
and the content make it useless for a modern Mandarin course — and attempting to harvest
it would be a knowing ToS violation. (The underlying pre-modern *works* are public domain;
CTP's digitisation, transcriptions and annotations are what its terms cover. If you ever
wanted a classical text, get it from Wikisource instead.)

### 4.5 Wikisource / Wikipedia dumps

* Licence: same dual CC BY-SA 4.0 + GFDL as Wiktionary (Wikimedia ToU, quoted above);
  individual Wikisource texts are usually public domain in themselves, with the
  transcription/annotation layer under CC BY-SA.
* Dumps: `https://dumps.wikimedia.org/zhwikisource/` and `https://dumps.wikimedia.org/zhwiki/`
  (`pages-articles-multistream.xml.bz2`); measured sizes on 2026-09-21:
  **zh.wikipedia = 3.17 GiB**, **zh.wikisource = 7.09 GiB** (uncompressed XML is several
  times larger). Usable commercially **with ShareAlike and attribution**, but:
  * zh.wikisource is overwhelmingly **Classical Chinese** and public-domain documents —
    wrong register for a beginner Mandarin course.
  * zh.wikipedia is encyclopedic prose: long, specialised, ungraded, and stylistically
    unlike spoken Mandarin. It is a fine *advanced reading* source; it is not a source of
    example sentences, and it carries the usual Wikipedia caveats (unsourced claims,
    rapidly changing text, article-level attribution to hundreds of contributors).
  * If you want Wikipedia text in usable form, `UD_Chinese-GSD` (§4.1) already gives you
    4,997 Wikipedia sentences with gold syntax — that is the better-shaped artefact.

### 4.6 OPUS and its Chinese corpora **[verified: OPUS API + OPUS legacy corpus pages + OPUS's own licence table + upstream repos]**

OPUS (https://opus.nlpl.eu) is a **distribution platform, not a rights holder**. Its own
site-wide disclaimer on every corpus page says, verbatim: *"We do not own any of the text
from which the data has been extracted. We only offer files that we believe we are free to
redistribute…"* — i.e. OPUS asserts nothing about the sentences, and its licence field is
where it records whatever the upstream project claimed.

Better still, **OPUS publishes its own licence table**:
`https://github.com/Helsinki-NLP/OPUS/blob/master/info/RELEASE_LICENSES.tsv`. In it,
**132 of 1,263 releases are literally `unknown`** — including CCMatrix, XLEnt,
OpenSubtitles (all versions), News-Commentary, MultiUN, UNPC, GNOME, KDE4, Ubuntu,
GlobalVoices, Wikipedia, Tanzil, Books, OpenOffice, WMT-News, ParaCrawl, CCAligned, HPLT
and TED2013. Where OPUS's dataset page shows no licence, that is what it means.

Sizes come from the API (`https://opus.nlpl.eu/opusapi?corpus=…&source=en&target=zh`, no
trailing slash); note that OPUS's per-corpus HTML pages have moved (old
`https://opus.nlpl.eu/<Corpus>/` 404s; use `/datasets/<Corpus>` or `/legacy/<Corpus>.php`).

| OPUS corpus | en↔zh pairs (OPUS API, 2026-09-21) | Licence OPUS states | Bundle? |
| --- | --- | --- | --- |
| **Tatoeba** | `cmn`–en 49,851 (note: `cmn`, not `zh`) | **"CC BY 2.0 FR"** | Yes, **but use Tatoeba's own dumps** — see the attribution gap below |
| **UNPC** v1.0 | 17,451,549 | `unknown` | Upstream UN terms: *"in the public domain"*, *"user must acknowledge the United Nations as the source"*, *"no other restrictions apply"* → clean, but UN prose is a terrible register for a learner app |
| **bible-uedin** v1 | 124,378 | **CC0 1.0** (matches upstream `LICENSE`) | Yes (archaic biblical Chinese — not teaching material) |
| **tldr-pages** | 17,514 | **CC BY 4.0** (matches upstream `LICENSE.md`) | Yes with attribution (CLI help text — wrong register) |
| **PHP manual** v1 | 41,706 | `unknown` | Upstream: *"distributed only subject to … Creative Commons Attribution 3.0 License or later"* → yes with attribution (technical prose) |
| **GlobalVoices** v2018q4 | 144,224 (`zht`) | `unknown` | Upstream footer (2018 and today): **CC BY 3.0** — *not* BY-SA; news prose |
| **Mozilla-I10n** v1 | en–zh_CN **107,657** (zh_TW 81,464, zh_HK 1,159) | **MPL 2.0** | Yes with MPL file-level notice duties — UI strings, not sentences |
| **WikiMatrix** v1 | 786,512 | **"CC-BY-SA 4.0"** | **Attribution-by-inference**: FAIR licenses only the LASER *software* (BSD); the mined data has no upstream licence, and the CC BY-SA claim is defensible only because the sentences are Wikipedia text (CC BY-SA 4.0 + GFDL). Yes with SA |
| wikimedia / WikiTitles / LinguaTools-WikiTitles / MDN_Web_Docs | 471,224 / 921,959 / 6,664,332 / 66,641 | CC-BY-SA (4.0 / 4.0 / unversioned / 2.5) | Yes with SA; all are Wikipedia/MDN text |
| CCMatrix | 71,383,325 | **`unknown`** — and FAIR's README licenses only its *scripts*, not the data; Common Crawl ToU pushes third-party rights onto you | **No — the single biggest zh source and the one to avoid** |
| NLLB | 71,383,325 (byte-identical to CCMatrix en–zh) | **"ODC-By"** | **Not safely** — see below |
| ParaCrawl v9 / MultiParaCrawl / HPLT | 14,170,869 / 2,090,780 / 5,306,624 | `unknown` (HPLT: packaging CC0) | **No** — upstream states it owns nothing and licenses only the *packaging* under CC0 |
| News-Commentary v16 | 125,996 | `unknown` | Upstream README: CC0 on the *packaging* only → **No** (newspaper text is third-party) |
| CCAligned / MultiCCAligned / XLEnt | 15,181,417 / 15,181,416 / 6,292,330 | `unknown` | **No** — a disclaimer is not a grant |
| MultiUN | 9,564,315 | `unknown` | **Risky** — no licence file anywhere; public-domain status is *inferred* from UN ST/AI/189/Add.9/Rev.2 in the LREC papers, not granted |
| OpenSubtitles | v2024 en–zh_CN 22,394,812 / zh_TW 18,583,480; v2016 9,304,777 (**v2018 has no Chinese at all**) | `unknown` | **No** — upstream ToS: *"Commercial use prohibited."* |
| TED2020 / TED2013 / NeuLab-TedTalks | 16,382 / 154,579 / 218,034 | *"respect the TED Talks Usage Policy"* / `unknown` | **No** — TED's terms are CC BY-NC-ND 4.0 and say talks *"cannot [be used] in any commercial context … in an app of any kind"*, and TED's ToU §6.4 bars AI/ML dataset use |
| QED v2.0a | 13,123 | *"made public for RESEARCH purpose only"* + *"All rights reserved"* | **No — research-only, disqualifying** |
| Tanzil | 187,092 | `unknown` | **No** — *"for non-commercial purposes only"*, redistribution *"not allowed"* (terms at tanzil.net/trans) |
| GNOME / KDE4 / Ubuntu / KDEdoc / OpenOffice | 85,698 / 142,472 / — / 190 / 69,399 | `unknown`, no licence in `info.yaml` | **No** — per-module GPL/LGPL/PDL copyleft, provenance not preserved, and KDE `.po` files carry no licence header at all |
| WMT-News v2019 | 19,965 | `unknown` | **No** — *"freely used for research purposes"*, other uses by arrangement |

**The attribution gap in OPUS's Tatoeba package [verified by downloading it].** The release
`OPUS-Tatoeba/v2026-07-08/moses/cmn-en.txt.zip` contains only parallel plain-text lines plus
a LICENSE (the French CC BY 2.0 legal code) and a README — **no sentence IDs and no author
usernames**. CC BY 2.0 FR requires crediting the original author, so the OPUS packaging
*cannot* support compliant attribution on its own; you must re-join against Tatoeba's own
`cmn_sentences_detailed` TSV (which carries `username` per sentence ID). OPUS's blanket
"CC BY 2.0 FR" is also over-broad relative to Tatoeba's per-sentence licensing. This is a
second, independent reason to take Tatoeba data from Tatoeba.

Three conclusions:

1. **Mined-web corpora are a trap.** CCMatrix, NLLB, ParaCrawl, HPLT, CCAligned and XLEnt
   are built by mining Common Crawl; the sentences are other people's web text. ParaCrawl
   and HPLT say outright that they own nothing and license only the *packaging* under CC0;
   CCMatrix, CCAligned and XLEnt state no licence at all; OPUS's NLLB entry says "ODC-By",
   which is a *database* licence over the alignments — ODC-By §2.4 expressly does **not**
   cover rights in the individual contents. The NLLB lineage is NC in two places as well:
   Meta's **NLLB-200 model** and its **mined-bitext metadata** are CC BY-NC 4.0, NLLB-Seed
   is CC BY-SA 4.0 *but contains no Chinese*, and the Chinese sentence data that circulates
   is AllenAI's ODC-By reconstruction of the mined bitext (`allenai/nllb`). For a bundled
   commercial app, treat all of these as unusable however permissive the label looks.
   (FLORES-200 — the NLLB evaluation set — *is* CC BY-SA 4.0 with Chinese, but it is a
   dev/test set, not a corpus to teach from, and the NLLB paper's corpus inventory contains
   no per-dataset licence column at all, contrary to a common claim.)
2. **A handful of OPUS corpora are genuinely clean, but they are the wrong genre** for a
   Mandarin course: UN documents (public domain + UN acknowledgement), PHP/tldr-pages/MDN
   technical text (CC BY 3.0+/4.0), GlobalVoices news (CC BY 3.0), Mozilla UI strings
   (MPL 2.0), a CC0 Bible translation. They are useful evidence that OPUS *can* carry clean
   data; none of them supplies learner-register example sentences.
3. **The only OPUS-hosted Chinese corpus that is both clean and usable as example sentences
   is Tatoeba itself** (CC BY 2.0 FR), and its OPUS packaging is strictly worse than
   Tatoeba's own weekly dumps — no IDs, no usernames.

Unrelated but worth recording: OPUS also hosts the **whole-corpus TMX/Moses files** for
Windows/GNOME/KDE/Ubuntu localisation memories, whose upstream licences are the projects'
own (mostly GPL/LGPL) — that is a copyleft layer on top of the SA/attribution question and
is not a good sentence source for a learner app.

### 4.7 Audio alternatives **[verified]**

Tatoeba's Chinese audio is out (§2.4). The two credible open alternatives:

* **Mozilla Common Voice — CC0 data, but the distribution channel is the problem.**
  The official Community Playbook
  (https://common-voice.github.io/community-playbook/sub_pages/text.html) states, verbatim:
  *"Mozilla Common Voice datasets are released under a CC0 'No Rights Reserved' License and
  are part of the public domain."* The prompt text is contributed under CC0 as well, and
  the MDC dataset record for the Chinese sets confirms a CC0-1.0 licence; Mandarin coverage
  is substantial (roughly **zh-CN 240 validated hours / ~51.7k sentences / ~7.6k speakers**,
  plus zh-TW, zh-HK and Cantonese sets). **But** since October 2025 Common Voice is
  *"exclusively available through Mozilla Data Collective"*
  (https://community.mozilladatacollective.com/faq-can-i-get-the-common-voice-or-other-mdc-datasets-from-other-platforms-like-github-or-hugging-face/),
  and the MDC platform terms forbid, verbatim:
  > "(ix) copy, scrape, download or otherwise acquire any Dataset or portion thereof from
  > the Platform for the purpose of … (B) hosting, storing or making the Dataset available
  > on any platform, server or repository other than the Platform, except as expressly
  > permitted under the applicable Data Consumer License for that Dataset";
  > "…(xi) circumvent or bypass any technical measures that control access to a Dataset or
  > limit their download, reproduction or redistribution"

  Mozilla's own FAQ frames the two layers as *"CC0 remains the license for computational
  use, whilst not allowing mirroring the datasets is a platform term."* The old S3 dataset
  URLs now return **403**, the HuggingFace `common_voice_17_0` mirror is empty, and the
  per-dataset **Data Consumer License is behind a login and was not retrievable** in this
  research. **Verdict: unresolved — CC0 permits bundling, the MDC contract appears to
  forbid shipping the dataset; do not rely on Common Voice without reading the Data
  Consumer License and taking legal advice.** Also disqualifying if you are tempted by the
  derived speech-translation corpora: **CoVoST 2 en→zh is CC BY-NC-4.0, "Research and
  non-commercial use only"**
  (https://mozilladatacollective.com/datasets/cmpqxcoy400zinv07bmrdx59x).
* **Lingua Libre (Wikimedia France) — CC0 and CC BY-SA 4.0, mixed per file.** Mandarin
  recordings exist under the language item **Q9192**: `Category:Lingua Libre pronunciation-cmn`
  on Wikimedia Commons held **4,122 audio files** on 2026-09-21 (`prop=categoryinfo` →
  `files: 4122`), e.g. `File:LL-Q9192 (cmn)-CanonNi-一目了然.wav` (wikitext licence
  `{{cc-zero}}`) and `File:LL-Q9192 (cmn)-Assassas77-下载.wav` (`{{cc-by-sa-4.0}}`).
  **The licences are genuinely mixed** — two independent samples of the category (200
  files, then a random 60) gave CC0 shares of 38% and 50% respectively; do **not** assume
  CC0. Commons' `extmetadata.LicenseShortName` also under-reports CC0 on dual-licensed
  files, so **read the wikitext or the `Category:CC-Zero` membership per file** before
  shipping. Also note API rate-limiting (HTTP 429) when harvesting. The limitation is
  granularity: these are **words and short phrases, not sentences** — which is exactly what
  a vocabulary app needs, and it pairs with the already-bundled word list — and speakers are
  volunteers, so voice consistency varies.
* **Synthesised audio you generate yourself.** Given that a licence-clean *sentence* audio
  corpus does not exist, the realistic way to get sentence audio is to synthesise it at
  build time with a permissively-licensed TTS model and ship the resulting files. The
  `no7z/hsk-sentences-audio` dataset shows the pattern (CosyVoice2-0.5B, Apache-2.0, with
  an explicit "audio is synthetic" disclosure). Check the *voice* licence as well as the
  model licence, and disclose synthesis in the app.
* Not usable: **Forvo** (proprietary), **Tatoeba audio** (§2.4), **CoVoST 2** (NC),
  **any dataset whose audio licence is "see the contributor"**.

---

## 5. Datasets that specifically grade Chinese sentences by HSK level

Short answer: **sentence-level HSK grading exists in the wild only as third-party
derivations, and the only ones with clean licences are small.** The official HSK word
lists are not a citable open dataset; they are transcribed (MIT) by other projects, and
"HSK 3.0" is *two different official documents* that disagree substantially.

### 5.1 What Tatoeba itself offers — nothing usable **[verified]**

See §3.2: one sentence tagged `HSK`, three tiny user lists (~487 sentences total,
one hobbyist, no stated method), no difficulty field in the schema. Tatoeba is the
*source text*, not the grading.

### 5.2 Tatoeba + HSK labels (third-party derivations)

| Dataset | Contents | Size | Licence claimed | Bundle? |
| --- | --- | --- | --- | --- |
| `bdx33/tatoeba-hsk-cmn-eng-fra` | Tatoeba sentences, **simplified Chinese + English + French, with an `hsk_level`** | **78,504 rows**, last updated 2025-08-20 | card says `cc-by-2.0`, source "https://tatoeba.org/downloads" | **Probably yes** (it is Tatoeba text under CC BY 2.0 FR + an added level label), but the card documents **no grading method**; the middleman adds risk without adding data you cannot regenerate |
| `no7z/hsk-sentences-audio` | 4,354 Chinese sentences "graded against the official HSK 3.0 levels 1–6", pinyin, English, per-word glosses, grammar tags, **synthetic** audio (2 MP3s each) | 4,354 sentences / 8,708 MP3s; HSK 1: 281, 2: 538, 3: 727, 4: 801, 5: 965, 6: 1,042 | **CC BY-SA 4.0** (card); glosses derived from CC-CEDICT (CC BY-SA) | Yes, **with ShareAlike**; audio is TTS (CosyVoice2), must be disclosed as synthetic |
| `harukicoder/hsk30-graded-readers` | Word-aligned graded readers with per-word pinyin/gloss; ships **no** level labels | **132 texts / 1,185 sentences / 8,682 tokens** | **CC BY 4.0** ("including commercially", per `corpus/LICENSE` in the repo) | Yes, but LLM-drafted, explicitly "not naturally occurring Chinese", and tiny |

URLs: https://huggingface.co/datasets/bdx33/tatoeba-hsk-cmn-eng-fra ,
https://huggingface.co/datasets/no7z/hsk-sentences-audio
(repo: https://github.com/no7z/hsk-sentences-audio , code MIT, 8,708 MP3s verified present),
https://huggingface.co/datasets/harukicoder/hsk30-graded-readers (repo:
https://github.com/harukicoder/hsk30). Licences above are the ones stated on the dataset
cards / repo `LICENSE` files, read directly; the cards are the uploader's own claims, so
treat the *labels* as unvalidated even where the underlying text is clearly Tatoeba's.

### 5.3 Grading it yourself — the defensible route **[verified]**

Because the licence-clean graded corpora are small while the licence-clean *ungraded*
corpus (Tatoeba Mandarin) is ~89k sentences, the robust plan is to ship Tatoeba and
compute the HSK level yourself. Two components, both permissively licensed:

* **Word/character level tables** — already in this repo:
  `complete-hsk-vocabulary` (https://github.com/drkameleon/complete-hsk-vocabulary),
  **MIT**, 11,470 entries mapping each word to `old-1..6` (HSK 2.0) and `new-1..9`
  (HSK 3.0/2021) levels; it also carries CC-CEDICT-derived glosses (CC BY-SA 4.0 — already
  recorded in `LICENSES.md`). Its HSK lists are transcriptions of the official documents;
  the compilation is MIT.
* **Word/character/grammar tables with no ShareAlike at all**, if the CC-CEDICT SA
  entanglement is unwanted:
  * `ivankra/hsk30` — **MIT** (LICENSE: "Copyright (c) 2023 Ivan Krasilnikov / Copyright
    (c) 2021 Shawky / Copyright (c) 2021 Pleco Inc."), 11,092 HSK 3.0 words across bands
    1–6 and 7–9 with pinyin/traditional/POS, plus 3,000 characters and 624 grammar points.
    https://github.com/ivankra/hsk30
  * `elkmovie/hsk30` — **MIT**, "Copyright (c) 2021 Pleco Inc."; `wordlist.txt` has 11,106
    entries and is an OCR of the official Ministry-of-Education PDF that Pleco then
    MIT-licensed. https://github.com/elkmovie/hsk30
  * `wzperson/hearmandarin-hsk-3-0-word-list` — **CC BY 4.0**, 11,147 words, HSK 3.0 bands
    1–7 (300/200/500/1,000/1,600/1,800/5,600) plus an HSK 2.0 alignment, with **in-house
    glosses deliberately not taken from CC-CEDICT**, so it carries no ShareAlike.
    https://github.com/wzperson/hearmandarin-hsk-3-0-word-list
* **A grader with a stated method** — `hsk30` (https://github.com/harukicoder/hsk30 ,
  https://pypi.org/project/hsk30 , **MIT for code and derived level tables**, CC BY 4.0
  for its small corpus). It grades text against *either* official document and exposes a
  coverage curve (`--curve --target N`), which is much closer to what a learner-facing
  difficulty knob needs than a single "level" number.
* **Wiktionary has the full HSK 2.0 and HSK 3.0 word lists** in Appendix pages, plus
  per-entry level data for the obsolete pre-2010 HSK 1.0, under **CC BY-SA 4.0** (dual with
  GFDL, §4.3) — a ShareAlike alternative to the MIT tables above.

The caveat that the `hsk30` project documents and that any honest HSK feature must
repeat: **"HSK 3.0" is two documents.** GF 0025-2021 (in force 2021-07-01, 10,977 words /
3,000 characters) and the 新版HSK考试大纲 (published Nov 2025, in force Jul 2026, 10,896
words / 3,088 characters) **disagree on 41.5% of shared vocabulary and 40.7% of shared
characters**, and regrading 102 authentic readers against one rather than the other moves
**48% of them by a level** — almost always upward. A sentence's "HSK level" is therefore
meaningless without naming the document; ship the label *and* the standard. Grading is
also simplified-character-only in `hsk30` (convert Traditional with OpenCC, Apache-2.0),
which matters because ~45% of Tatoeba's Mandarin is Traditional (§3.4).

### 5.4 What is *not* usable

* **Chinese Grammar Wiki** and anything derived from it (`UD_Chinese-Beginner`) —
  CC BY-NC-SA 3.0 (§4.2).
* **The upstream official HSK lists are not openly licensed.** GF 0025-2021 is a
  《语言文字规范》 (not one of the categories that Chinese copyright law exempts), the free
  Ministry-of-Education PDF is watermarked 仅供查阅 on every page, the commercial edition is
  BLCU Press all-rights-reserved, and both the testing bodies (CLEC / CTI) assert
  copyright — CTI permits reproduction only of 新闻性或资料性公共免费信息, and CLEC says
  未经许可不得转载. Every MIT/CC grant in §5.3 is a **third-party grant over a transcription**
  and binds only its own authors. The individual words are arguably uncopyrightable facts,
  but the *compilation* could attract protection (著作权法 Art. 15 for collections), and no
  controlling precedent was found. Practical mitigation: use an MIT/CC-BY transcription,
  credit it, keep the HSK level as a *derived* label, do not ship official definitions,
  glosses or example sentences verbatim from the official documents, and consider deriving
  your own assignments.
* **HSK Standard Course / BLCUP textbooks**, **CTRDG** (5,721 textbook texts, no LICENSE
  file, BLCUP upstream), a **Mendeley HSK 1–9 OCR of BLCUP lessons** (CC BY 4.0 tag on
  BLCUP-owned text — the tag cannot bind BLCUP), **`Roxaleen/hsk-annotated-corpus`**
  (260k sentences, no LICENSE), **HSK动态作文语料库 2.0** (forbids commercial use),
  **LDC** (no Chinese HSK/learner corpus exists there, and LDC terms are non-commercial),
  **HSK Academy** (all rights reserved), **ChinesePod / DuChinese / Mandarin Companion /
  Immersive Chinese** (all rights reserved; Immersive Chinese publishes no terms at all),
  **AnkiWeb HSK decks** (personal use only; ~383 of 423 decks state no licence).
* **`Tiagodfs/hsk-3.0-dataset` (HF, `cc0-1.0` tag)** — **mislabeled**: its per-level counts
  (500/772/973/1,000/1,071/1,140) are the *old* HSK 2.0 word count, despite the name. A
  reminder that an HF `license:` field is an uploader claim, not a verified fact.
* **Wikidata has no HSK property** — the sole proposal was closed `not done` in 2023
  ("no consensus"), and there is no CEFR property either — so there is no CC0 HSK mapping
  to prefer over the lists above. Wiktionary's only per-entry levels are the obsolete
  pre-2010 HSK 1.0; its HSK 2.0/3.0 material is confined to Appendix word-list pages under
  CC BY-SA 4.0.

---

## Bottom line

For an app that bundles its data and never touches the network, the licensing question has
a surprisingly clean answer: **one corpus does the sentence work, and the grading is
something you compute rather than licence.**

### The recommendation

1. **Sentences: Tatoeba Mandarin, CC BY 2.0 FR.** ~89k approved sentences, no share-alike,
   commercial use explicitly not opposed, obligation is attribution only. Pull three
   per-language files and join them:
   * `cmn_sentences_detailed.tsv.bz2` — `id, lang, text, username, created, modified`
     (the `username` is what makes per-sentence attribution possible);
   * `cmn-eng_links.tsv.bz2` — the English translations (66,243 sentences have one);
   * `cmn_transcriptions.tsv.bz2` — pinyin for **every** sentence, plus the
     Simplified↔Traditional conversion Tatoeba already computed.
   After filtering (clean text, ≤ 20 characters, has English) you have **61,666** candidate
   sentences, of which **25,737** come from contributors who self-report native Mandarin.
   That is a complete beginner-to-intermediate course's worth of material from one source.
   Pin the export date (the weekly of **2026-09-19** was current for this report) and keep
   the sentence IDs you shipped: Tatoeba has a takedown process, so the corpus can change
   underneath you.
2. **Grading: compute it, don't licence it.** There is no open HSK-graded *sentence* corpus
   of meaningful size — the best is 4,354 sentences (`no7z`, CC BY-SA) and the HSK lists on
   Tatoeba itself amount to one tagged sentence. But word-level HSK data is MIT/CC BY, and
   this repo already bundles it. Grade each sentence by the highest HSK level among its
   content words (`complete-hsk-vocabulary`, MIT, already embedded; optionally add
   `wzperson/hearmandarin-hsk-3-0-word-list`, CC BY 4.0 and **no ShareAlike**, to avoid
   CC-CEDICT entanglement in the level table). For the coverage-curve method use the MIT
   `harukicoder/hsk30` grader. **Name the standard in the UI**: GF 0025-2021 and the 2025
   exam syllabus (in force since July 2026) disagree on 41.5% of shared vocabulary and move
   48% of texts by a level. Grade simplified text only (convert Traditional with OpenCC,
   Apache-2.0) — ~45% of Tatoeba's Mandarin is Traditional, so the script choice is a real
   product decision, not a detail.
3. **Audio: do not ship Tatoeba's.** 5,741 of 5,826 Mandarin recordings carry no reuse
   licence at all (the API returns 403 for them) and the other 84 are NonCommercial. If you
   need audio, the licence-clean options are (a) **Lingua Libre `cmn`** — 4,122 files on
   Commons, individual words/phrases, **mixed CC0 / CC BY-SA 4.0, so filter per file**; and
   (b) **synthesising it yourself** at build time from a permissively licensed TTS model for
   your chosen sentences, with a synthetic-audio disclosure. **Common Voice is CC0 data but
   is now gated by Mozilla Data Collective terms that appear to forbid shipping it — treat
   as unresolved and get legal input before relying on it.**
4. **Optional, with obligations:** if you want gold syntax alongside the sentences,
   `UD_Chinese-GSD` (4,997 sentences, CC BY-SA 4.0, verbatim Wikipedia text) and
   `UD_Chinese-PUD` (1,000, CC BY-SA 3.0) are the clean ones; English Wiktionary's Chinese
   examples (~43k sentence-like strings via kaikki, CC BY-SA 4.0) are a second pool. All
   ShareAlike: the *shipped sentence file* must stay CC BY-SA 4.0, be attributed, and not be
   locked down — the app's own code is untouched by that. Skip `UD_Chinese-HK` (its text is
   third-party film subtitles), `UD_Chinese-CFL` (learner errors only), and everything NC
   (§4.1–4.2).
5. **Hard no:** Chinese Grammar Wiki / AllSet (CC BY-NC-SA 3.0, explicitly no commercial and
   no ad-supported use), `UD_Chinese-Beginner` (derived from it, NC),
   `UD_Chinese-PatentChar` (NC despite its wrong `LICENSE.txt`), the official MOE/BLCUP/CLEC
   HSK documents, LDC and the HSK essay corpus, and every mined-web corpus — CCMatrix, NLLB,
   ParaCrawl, OpenSubtitles, TED2020 — whose "licence" covers alignments, not sentences.

### What compliance actually costs

CC BY 2.0 FR requires attributing the **Original Author** — so the app needs *per-sentence*
contributor credit, not just "sentences from Tatoeba". The good news is that
`cmn_sentences_detailed.tsv.bz2` carries the username for every sentence, so this is a
build-time table, not manual work: generate a provenance table
(`sentence_id → username`) plus a Sources/Licences screen that names Tatoeba, links to
https://tatoeba.org, names the licence as CC BY 2.0 FR (link
https://creativecommons.org/licenses/by/2.0/fr/), and dates the export. The repository
already has the right machinery (`licences/`, `LICENSES.md`, the Licences screen,
`src-tauri/src/licences.rs`) — this is one more generated notice, not a new mechanism. No
share-alike attaches to the Tatoeba text, so adding it does not change the app's own
licensing position.

### Residual risks to accept knowingly

* Tatoeba is a volunteer corpus with no professional review: expect to filter, and expect a
  small error rate even after filtering (only ~194 of the filtered sentences are explicitly
  tagged `OK`). If you need a guarantee, budget for review of the ~25.7k native-authored
  pool rather than the full 61.7k.
* Some Tatoeba sentences are quotations. The exports exclude unapproved and licensing-flagged
  sentences, but Tatoeba offers no indemnity and disclaims systematic checking; dropping
  sentences tagged `quote`/`from …`/`by …` is cheap insurance.
* The official HSK word lists themselves have no open grant; the MIT/CC-BY tables are
  third-party transcriptions. The words are facts, but the compilation argument has no
  controlling precedent — this is the one place where "openly licensed" rests on a
  reasonable rather than a certain position.

---

## Appendix — evidence trail

* Live counts and file formats in this report were produced on **2026-09-21** against
  Tatoeba's weekly export dated **2026-09-19** and the live Tatoeba API; the UD sentence
  counts were produced by downloading each treebank and running
  `grep -c '^# sent_id'` over its `.conllu` files.
* Companion research files left in this workspace by the delegated investigations:
  * `_license_research/REPORT.md` — full UD / Grammar Wiki / Wiktionary / kaikki / Wikidata /
    ctext / Wikisource / Lingua Libre / Common Voice detail (tables, verbatim quotes, size
    measurements).
  * `docs/hsk-graded-data-license-report.md` — full HSK-graded dataset audit, including the
    upstream official-document copyright analysis and the list of proprietary/ARR
    resources.
  * `zh-corpus-license-report.md` (workspace root) and
    `_license_research/nllb-opus-cluster-report.md` — the full OPUS / NLLB / FLORES audit
    behind §4.6, per-corpus verbatim licence quotes, the OPUS `RELEASE_LICENSES.tsv`
    analysis, and the upstream corrections (WikiMatrix's missing upstream grant, CCMatrix's
    absent licence, TED's NC-ND app ban, OpenSubtitles' commercial prohibition).
  * `_license_research/ud/` and `_license_research/raw/` — downloaded treebanks
    (`UD_Chinese-*`, `UD_Classical_Chinese-*`, ~86 MB including tarballs) and raw page
    captures. **These are scratch evidence and can be deleted** (`rm -rf
    _license_research/ud`); nothing in either report depends on them being present.
* Where this report says *[verified]*, the figure or quotation was read from the primary
  source during this research. Two items remain deliberately unresolved rather than
  guessed: the **Mozilla Data Collective Data Consumer License** (login-gated; Common Voice
  bundling therefore unresolved) and the exact **version of the CC-CEDICT licence** the
  bundled dictionary actually carries (MDBG says 4.0, the cc-cedict.org wiki says 3.0 —
  `LICENSES.md` already records 4.0 and the repository's copy of the CC-CEDICT header says
  4.0, which is the stronger evidence, but it is worth a one-line confirmation upstream).
