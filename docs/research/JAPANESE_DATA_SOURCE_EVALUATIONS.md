# Japanese data sources considered for Nihongo Tutor

Three sources were put forward while the graded-phrases work (N18) was being built, and each
was measured rather than eyeballed: what it contains, what it is licensed under **verbatim**,
and — the part that decides most of them — whether the licence claim rests on the
rights-holder or only on a repackager.

That distinction is not pedantry. `LICENSES.md` already records it for CC-CEDICT and
`docs/research/JAPANESE_TUTOR_FEASIBILITY.md` §6.4 records it for every "JLPT N5 list" in
circulation: *a downstream MIT/CC grant covers that licensor's contribution, not the
rights-holder's*. It cannot launder the underlying list. All three sources below are judged on
that rule, and two of the three fail it somewhere.

**The baseline they are judged against**, because it changes every verdict: this app already
embeds JMdict and KANJIDIC2 (CC BY-SA 4.0, EDRDG), JmdictFurigana (MIT), KanjiVG and AnimCJK
stroke geometry, a derived vocabulary ladder, UniDic segmentation at build time, and — as of
N18 — 1,400 Tatoeba sentences (CC BY 2.0 FR). A source that republishes those adds nothing but
its own processing.

---

## 1. OpenJLPT — <https://github.com/evanclan/OpenJLPT>

**Verdict: use its *grammar* dataset when grammar becomes a milestone; do not use it as the
phrase corpus, and do not ship its levels.**

| | |
| --- | --- |
| Version measured | `0.3.0` (`data/json/meta.json`), tree of 2026-10-05 |
| Contents | 7,811 words (N5 674 / N4 630 / N3 1,659 / N2 1,778 / N1 3,070), 2,383 kanji, **526 grammar points**, 13,718 example sentences (12,615 unique by `tatoeba_id`), 7,245 words with at least one example |
| Formats | JSON, CSV, SQLite, npm, PyPI, CDN |
| Licence | **CC BY-SA 4.0** for the dataset (its own `LICENSE`); MIT for code |
| Upstreams, per its `NOTICE.md` | Jonathan Waller's JLPT lists (asserted **CC BY**), JMdict + KANJIDIC2 (CC BY-SA 4.0, EDRDG), **Tatoeba** (CC BY 2.0 FR), KanjiVG (website only, CC BY-SA 3.0), its own grammar content (CC BY-SA 4.0) |

### Measured: the sentences are the ones N18 already has

Every one of the 12,615 example sentences is present in the Tatoeba per-language export N18
imports — checked id by id against `jpn_sentences_detailed.tsv`. 12,188 of them (99.7%) pass
N18's shape filter. So OpenJLPT's sentences add **curation and levels, not text**: its value is
that each sentence was chosen as the example for a particular word.

Its own README claims *"Short, checked sentences come first"* and that 93% of words have
examples; 7,245 of 7,811 is 92.8%, so that figure holds. Its `furigana` field (an inline
`{会|あ}` markup) is unused here for the same reason Tatoeba's is: N18 computes its own with
UniDic and this project's vocabulary, and 56.5% of Tatoeba's transcriptions are unreviewed
machine output.

### The level problem, which is §6.4's problem exactly

OpenJLPT's N5–N1 vocabulary and kanji levels come from **Jonathan Waller's lists** —
`tanos.co.uk`, the same lists Jisho.org uses. Two things are wrong with shipping them as
levels:

1. **The rights-holder's grant is unversioned and its site is gone.** Waller's sharing page said
   *"Everything on this site (that I'm not selling), is licenced under Creative Commons
   "BY""* — no version. `tanos.co.uk` does not resolve today (NXDOMAIN); the statement survives
   only in the Wayback Machine. It is **asserted**, not verified in the sense `LICENSES.md`
   uses the word.
2. **Downstream repackagers version-bump it.** OpenJLPT's `NOTICE.md` records the licence as
   CC BY; other datasets in the same lineage record "CC BY 4.0". Neither is a grant the
   rights-holder made, and the exclusion Waller himself named ("that I'm not selling") is the
   kind of clause a repackager cannot decide the scope of.

OpenJLPT is honest about all of this — its NOTICE has an "Important note on JLPT levels"
section saying the levels are unofficial community approximations, and its README says the same
— but honesty about a list's status is not a licence for it. This project's own answer, from
§6.4 and invariant 20, is to **derive its own ladder** from the kyōiku grades and EDRDG's
frequency (which is what `words.rs` does) and to label it as its own. N18 levels its phrases
the same way, so a phrase's band is the band of its hardest word rather than an N-number.

### What is genuinely worth taking: the grammar

The 526 grammar points are a different work under a different provenance: explanations and
examples **written by OpenJLPT's contributors**, released CC BY-SA 4.0, not a laundered
upstream list. `ROADMAP_NIHONGO.md` names grammar as "the largest gap in the product rather
than in the kana course". That makes this the strongest candidate for a future grammar
milestone, and N18's roadmap entry records it as such.

If it is taken: the artifact would be CC BY-SA 4.0 (share-alike), which this repository already
handles for EDRDG's data — the same `ccbysa` notice, the same "state your modifications"
discipline — plus a notice naming OpenJLPT. The level labels would **not** come with it.

---

## 2. jkindrix/japanese-language-data — <https://github.com/jkindrix/japanese-language-data>

**Verdict: usable, with a narrower delta than it looks. Take pitch accent and register
frequency; treat its grammar as unusable as teaching content; do not take what this app already
has.**

| | |
| --- | --- |
| State measured at HEAD | last push 2026-04-14 (against its own monthly-rebuild pledge), 14 tags, `v0.9.0` 23 commits behind `main`, no release assets, GitHub's licence detection `NOASSERTION` |
| Licence | **CC BY-SA 4.0** for "the aggregated dataset, the build pipeline source code, the schemas, and the documentation", with upstream terms flowing through |
| Data files | `words.json` 23,119 · `kanji.json` 13,108 · `sentences.json` 25,980 · `furigana.json` 28,920 · `jlpt-classifications.json` 11,099 · `pitch-accent.json` 124,011 · `frequency-subtitles.json` 8,598 · `grammar/expressions.json` 13,220 · `grammar/grammar.json` 595 · `grammar/conjugations.json` 3,511 · `kana.json` 215 |
| Audio | **none** — no media in the tree, `has_audio: false` on all 25,980 rows |

**Provenance is unusually good for a repackager.** Every data file carries a `metadata` block
naming its source, licence and attribution; the build is SHA-256 pinned; and it explicitly
**refuses** to integrate JPDB and WaniKani because their licences are incompatible or absent —
the opposite of the laundering pattern §6.4 warns about. Verified upstreams include EDRDG
(JMdict/KANJIDIC2), KanjiVG, Kanjium (CC BY-SA 4.0), Tatoeba, JESC, KFTT (BY-SA 3.0 via
Wayback), and JmdictFurigana (whose real licence is MIT, which the repo *over*-states as
CC BY-SA 4.0 — harmless).

**The weak links, named:** the Waller JLPT layer (same dead domain and unversioned grant as
above, arriving here through two further repackagers); the OpenSubtitles frequency list
(verified MIT/CC BY-SA for the *tooling*, but the underlying corpus is film and TV dialogue in
which the repackager holds no rights — low risk for bare counts, but it is an unlicensed
rights-holder in the chain); and the Leeds frequency list, whose CC BY 2.5 reference licence
covers the crawled pages rather than the list.

### Delta against what this app already ships

| jkindrix file | Delta |
| --- | --- |
| `words.json`, `kanji.json` | **none** — JMdict/KANJIDIC2, already embedded |
| `furigana.json` | **none** — JmdictFurigana, already embedded |
| `sentences.json` | **none of substance** — Tatoeba, of which N18 imports 1,400; its contributor fields are `null` in **all** 25,980 rows, so its own attribution is not includable either |
| `grammar/expressions.json` | **none** — 13,220 JMdict `exp` entries, an index rather than content |
| `pitch-accent.json` | **real** — 124,011 entries, Kanjium CC BY-SA 4.0 verified. This is also **N5's blocker**, so it is a lead for N5, not for phrases |
| `grammar/conjugations.json` | **real, small** — 3,511, EDRDG-derived |
| `frequency-*.json` | **real, caveated** — five registers where this app has only EDRDG's newspaper `nf` ranking |
| `jlpt-classifications.json` | **caveated** — 11,099 rows on the Waller layer; 595 of them are the project's *own* grammar levels wearing a Waller `source` label |
| `grammar/grammar.json` | **risky** — 595 points, but its own manifest says `review_coverage: {draft: 595, community_reviewed: 0, native_speaker_reviewed: 0}`, its docs say *"Zero native-speaker reviewers are engaged as of v0.9.0"*, and its README says *"Not authoritative for grammar."* Its headline *"90.3% with sentence matches"* is a regex `pattern_match_pct`; the real curated-example link rate is **0.22% (4 of 1,787)** |

So the recommendation is narrow and specific: **pitch accent** (for N5, which is blocked on
exactly this kind of provenance check — and here the Kanjium upstream *is* verified, with
`Uros O.` to be credited alongside the existing Kanjium attribution) and **register
frequencies**. Nothing else changes what the app shows.

---

## 3. bunkocommons.org ("Bunko Commons") — <https://bunkocommons.org/>

**Verdict: do not use, and do not revisit unless the site changes hands and terms appear.**

The site publishes lesson-by-lesson word lists and grammar notes for **GENKI 1 and 2, Minna no
Nihongo I and II, and Tobira**, plus one online novel, with a footer claim that "All content
[is] licensed under CC BY-SA 4.0". The measurements that decide it:

* **There is no licence file and no terms page.** `/terms/`, `/terms`, `/license/`, `/license`,
  `/licensing/`, `/privacy/`, `/faq/`, `/legal/`, `/tos/` all return **404**. The licence exists
  only as marketing text in a footer and an About card, and the attribution it asks for names
  the platform ("Credits should go to Bunko Commons"), not the authors — CC BY-SA 4.0 §3(a)(2)
  requires crediting the author(s).
* **The grant is made by someone who does not hold the rights.** The site's own announcement
  says it is *"a site to host Japanese language learning materials that are Creative Commons
  licensed… We are starting out with a few textbooks and online novels that I have read"* and
  *"At this moment, there is only 1 user (that's me) contributing to the site."* Its source list
  names **Tobira**, with `"author": "Kurosio Publishers"`, in its own data. GENKI is The Japan
  Times's; Minna no Nihongo is 3A Corporation's. A downstream CC grant cannot launder an
  upstream work whose rights-holder never licensed it — the same rule that rules out OpenJLPT's
  levels and the same one §6.4 applies to the "JLPT lists". Trademark exposure is separate and
  arguably larger: "GENKI lesson 1–12 vocabulary" is GENKI's.
* **The grammar pages have no provenance at all** — 451 detail pages, none attributed to JMdict,
  Tatoeba or any other source, and its own About page says only that it "uses content from other
  projects with similar licenses: Tatoeba, JMDict-Simplified".
* **There is no way to pin it.** The exports carry no `ETag`, no `Last-Modified`, no version and
  no checksum; only a rolling "latest".
* **Its advertised audio is a dead link.** The one audio-typed source embeds an `.mp3` that
  returns **404**; the media directory returns 403.

What it would add if it were licensable: terms exports (4,398 rows across 79 lessons) and
grammar notes (348 reachable from lessons, 451 pages in the sitemap) with English *and* Japanese
explanations and textbook lesson mappings. The definitions in it are JMdict verbatim, which this
app already has directly. Nothing else here is both new and clean.

---

## What this leaves for later milestones

| Candidate | Where it would go | What it would need first |
| --- | --- | --- |
| OpenJLPT's 526 grammar points | A grammar screen — the largest gap in the product | A CC BY-SA 4.0 artifact + notice, share-alike stated; **levels dropped** |
| jkindrix's pitch accent (Kanjium) | **N5**, which is blocked on exactly this provenance check | A Kanjium notice crediting `Uros O.`; the offline pitch data is a derived redistribution |
| jkindrix's register frequencies | Possibly the vocabulary ladder, which today has only EDRDG's `nf` | A rebuild from verified upstreams rather than taking the aggregation |
| Tatoeba audio | Nowhere | 79% of rows carry no reuse licence and 20% are CC BY-NC 4.0; the phrases stay spoken by the system voice (invariant 37) |

**Not candidates, with the reason recorded so the question is not reopened:** OpenJLPT's and
jkindrix's JLPT levels (unverified upstream grant), jkindrix's grammar (0 of 595 reviewed, its
own README says not authoritative), and everything on bunkocommons.org.
