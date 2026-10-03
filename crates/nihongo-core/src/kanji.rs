//! The kanji dataset: the jōyō set, with the geometry and the readings a course
//! needs.
//!
//! # Where each field comes from, and why that one
//!
//! Three upstream sources, and they are not interchangeable:
//!
//! * **Geometry** — outlines *and* centre-lines — comes from AnimCJK's
//!   `graphicsJa.txt` (Arphic Public License). It is the only source that
//!   publishes both: KanjiVG has centre-lines and no ink, so using it would mean
//!   synthesising the outlines a grading engine compares against.
//! * **The curriculum and the structure** — the grade sets, the radical, the IDS
//!   decomposition — come from AnimCJK's `dictionaryJa.txt` (LGPL-3.0-or-later).
//! * **Readings, glosses, frequency and the authoritative grade** come from
//!   EDRDG's KANJIDIC2 (CC BY-SA 4.0). Not from `dictionaryJa.txt`, which also
//!   carries `on`, `kun` and `definition` fields: EDRDG is the attributed source
//!   for readings and meanings, and a pipeline that quietly took a second-hand
//!   copy of them would be recording the wrong provenance for the data a learner
//!   actually reads.
//!
//! # The grade reconciliation, which is not the one-line story it looks like
//!
//! KANJIDIC2's `grade` is the **current** assignment and AnimCJK's `set` is the
//! pre-2017 one, so once AnimCJK's `g7` is read as KANJIDIC2's grade 8 — they are
//! the same set — the two files disagree about **59 of the 2,136** characters.
//! The differences are of two kinds, and `tests/kanji_artifact.rs` pins both:
//!
//! * the **20 prefecture kanji** added to kyōiku in 2017 (茨 媛 岡 潟 岐 熊 香 佐
//!   埼 崎 滋 鹿 縄 井 沖 栃 奈 梨 阪 阜) — every one of them grade 4 in
//!   KANJIDIC2 and `g7` in AnimCJK, which is what reconciles the jōyō total but
//!   *not* the per-grade counts;
//! * **39 characters the two sources put in different grades**, which is what
//!   does explain them: 夫 (AnimCJK 3→4) and 央 (4→3), 21 from 4→5, two from 4→6
//!   (胃 腸), four from 5→4 (富 徳 群 賀), nine from 5→6 (俵 券 恩 承 敵 舌 退 銭
//!   預) and one from 6→4 (城).
//!
//! Those two effects interleave, which is why the head-line count is misleading
//! on its own: the 20 additions give grade 4 +20, the 39 moves give it −18, and
//! the observed difference is +2. The per-grade totals therefore come out
//! 80/160/200/202/193/191 with KANJIDIC2's numbering against AnimCJK's
//! 80/160/200/200/185/181 — the same 1,026 against 1,006 kyōiku — and the ladder
//! taught is the one in force rather than 2016's.
//!
//! # KANJIDIC2's grade 8 is not AnimCJK's `g8`
//!
//! A trap worth naming: in KANJIDIC2 grade **8** is the rest of jōyō, and
//! jinmeiyō is 9 and 10; in `dictionaryJa.txt` `g7` is that same jōyō remainder
//! and its `g8` is jinmeiyō. Every grade this module carries is KANJIDIC2's.
//!
//! # The stroke count, and the oracle that checks it
//!
//! A kana's taught stroke count is read out of the SVG instead of trusted,
//! because the upstream file inflates it. A kanji's is simply the geometry's own
//! count, and two independent checks say that is safe: KANJIDIC2 lists it (see
//! [`Kanji::stroke_count`]), and **KanjiVG's per-stroke paths agree for 2,135 of
//! the 2,136**.
//!
//! The exception is 衷, which the geometry and KANJIDIC2 both count as 10 and
//! KanjiVG draws in 9. `prepare-kanji` carries it as a written exception with its
//! expected KanjiVG value, so a change on either side fails the build rather than
//! passing quietly — and the taught count in the artifact is the geometry's,
//! because that is the count the grader has reference strokes for.
//!
//! `ROADMAP_NIHONGO.md` records "nine named exceptions" where KanjiVG and
//! KANJIDIC2 disagree. That list is a measurement artefact and was checked
//! against the files: it is the nine characters whose *first* listed KANJIDIC2
//! count is not the taught one (謎 賭 葛 餌 遜 僅 遡 餅 list the smaller value
//! first, 牙 the larger). Compared against the taught count rather than against
//! whichever value happens to be first, all nine agree, and only 衷 is left.
//!
//! # The radical table, and why it is in the artifact
//!
//! The characters carry the **combining** form of their radical — 扌 in 持, not
//! 手 — because that is what is written inside the character. A radicals screen
//! needs the other shape: the **head form** the family is named by, all 214 of
//! them, including the 16 no jōyō character uses. That table is not derivable
//! from the characters, so the artifact carries it (format version 2, see
//! [`Radical`]) and `prepare-kanji` refuses to write an artifact whose table is
//! not the 214, in order, with geometry.
//!
//! # Not carried, deliberately
//!
//! * **JLPT levels.** KANJIDIC2 has the pre-2010 1–4 values. There has been no
//!   official JLPT kanji list since 2010, so shipping one would be shipping an
//!   approximation as if it were a specification. The level bands are a chosen,
//!   documented derivation instead — see `ROADMAP_NIHONGO.md` N7.
//! * **Jinmeiyō and hyōgai**, until a course needs them: 863 jinmeiyō and 3,783
//!   hyōgai characters, which is most of the file by count and none of it by
//!   teaching order.

use hanzi_core::Point;
use serde::{Deserialize, Serialize};

/// Magic bytes at the head of a kanji artifact, checked before decoding.
///
/// `postcard` is not self-describing, so a stale or foreign artifact would
/// otherwise decode into nonsense. The trailing digits are the format version.
///
/// **Version 2 added the radical table**, and the number moved with it: the
/// field sits between the characters and the source, `postcard` writes it
/// positionally, and an artifact written by version 1 would decode its source
/// into it. Bumping the magic is what turns that into "re-run `prepare-kanji`"
/// rather than a dataset with 214 fields of nonsense.
pub const KANJI_ARTIFACT_MAGIC: &[u8; 8] = b"KANJD002";

/// The KANJIDIC2 grades the artifact carries: 1–6 are kyōiku, 8 is the rest of
/// jōyō. See the module docs for why 7 and 9/10 are not here.
pub const JOYO_GRADES: [u8; 7] = [1, 2, 3, 4, 5, 6, 8];

/// How many characters those grades hold — the jōyō set.
pub const JOYO_COUNT: usize = 2_136;

/// How many Kangxi radicals there are, and how many the artifact carries.
pub const RADICAL_COUNT: usize = 214;

/// One kanji, with everything a course screen needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kanji {
    pub ch: char,
    /// KANJIDIC2's grade: 1–6 kyōiku, 8 the rest of jōyō.
    pub grade: u8,
    /// How many strokes the character is written with: the number of outline and
    /// centre-line pairs the geometry carries.
    ///
    /// **This needs none of the repair a kana does.** AnimCJK splits a *kana*
    /// stroke that crosses itself and stores one entry per drawing segment, so あ
    /// arrives with four where three are taught (see [`crate::kana`]); it does
    /// not do that for kanji. Measured over all 2,136 jōyō: the array length is
    /// in KANJIDIC2's `strokeCounts` for every one of them, and for the 91 whose
    /// entry lists more than one count — the only place a split could hide — the
    /// AnimCJK SVG's own `d<n>` element ids carry no split segment either.
    pub stroke_count: u8,
    /// KANJIDIC2's frequency rank (the Mainichi newspaper ranking), or `None`
    /// for the characters it does not rank.
    pub frequency: Option<u16>,
    /// The radical as this character is written with it: 扌 for 持, 子 for 学.
    pub radical: char,
    /// The note the upstream dictionary puts in parentheses after the radical,
    /// verbatim, because it is not always a character: `手` for 扌, and the words
    /// `阜 or 邑` for 阝, which stands for two different radicals depending on
    /// which side of the character it sits. `None` when it says nothing.
    pub radical_note: Option<String>,
    /// KANJIDIC2's classical radical number, 1–214.
    pub radical_number: u8,
    /// On'yomi in katakana, in KANJIDIC2's order.
    pub on: Vec<String>,
    /// Kun'yomi, carrying KANJIDIC2's okurigana markers exactly as it writes
    /// them: a `.` ends the stem and starts the okurigana (た.べる is 食べる), and
    /// a trailing `-` marks an affix that is not used on its own (ひと-).
    pub kun: Vec<String>,
    /// English glosses, from KANJIDIC2's senses — never a Chinese dictionary's.
    pub meanings: Vec<String>,
    /// Readings used in names (名乗り).
    pub nanori: Vec<String>,
    /// The IDS decomposition, e.g. `⿱艹次`. Empty for the two jōyō characters
    /// AnimCJK gives none for (一 and 乙).
    pub decomposition: String,
    /// SVG path data in **font space**, one entry per stroke, in stroke order.
    /// Render within `scale(1, -1) translate(0, -900)` over a 1024×1024 box,
    /// exactly as the kana outlines are rendered.
    pub outlines: Vec<String>,
    /// Stroke centre-lines in **display space**, in stroke order. Compared
    /// against a learner's strokes by `hanzi_core::grade`.
    pub medians: Vec<Vec<Point>>,
}

impl Kanji {
    /// True for the six kyōiku grades, which is what a school course teaches
    /// first.
    pub fn is_kyoiku(&self) -> bool {
        (1..=6).contains(&self.grade)
    }

    /// The reference to grade an attempt against.
    pub fn reference_medians(&self) -> &[Vec<Point>] {
        &self.medians
    }

    /// True when this character can be practised: it has geometry to grade
    /// against, and as many centre-lines as outlines.
    pub fn is_practisable(&self) -> bool {
        !self.medians.is_empty() && self.medians.len() == self.outlines.len()
    }
}

/// One of the 214 Kangxi radicals: the head form a learner studies, and the
/// geometry that lets the board write it.
///
/// # The head form is not the shape inside the character
///
/// A radical has two shapes, and the difference is the lesson. The head form is
/// the whole radical as it is taught and as it heads its own family — 手, 水, 言
/// — and this table carries that. The shape inside a character is a combining
/// form of the same radical: 扌 in 持, 氵 in 池, 讠 in 説 in Chinese. A character
/// stores the *combining* form in [`Kanji::radical`] and its number in
/// [`Kanji::radical_number`], which is what [`KanjiDataset::radical_families`]
/// groups by.
///
/// # Where the table comes from, and the trap in it
///
/// `dictionaryJa.txt` carries **214** entries whose `set` contains `radical`,
/// one per Kangxi radical, and their order in the file is the numbering: the
/// first is 一, the second 丨, the third 丶. That claim is checked rather than
/// assumed — KANJIDIC2 gives each of 212 of them its classical radical number,
/// and every one agrees with its position.
///
/// The **two exceptions** are written down in `prepare-kanji`, not skipped: 戶
/// (63) and 靑 (174) have no classical number in KANJIDIC2, because EDRDG
/// records the other form of those two radicals (戸 and 青).
///
/// Three of the 214 entries carry a **typo in their own `definition` text** —
/// 耒 at position 127 says "Kangxi radical 136", 臼 at 134 says 133, 足 at 157
/// says 156 — which is why the number is read from the position and checked
/// against KANJIDIC2, and never parsed out of the gloss. The pipeline never
/// reads that field at all (see `prepare_kanji`'s module docs on provenance), so
/// the typos are recorded here rather than tested: a test may not open
/// `data/raw/`, which is gitignored and absent from a clone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Radical {
    /// The classical number, 1–214.
    pub number: u8,
    /// The head form, e.g. 手 for 64 — never the combining form 扌.
    pub ch: char,
    /// How many strokes the head form is written with.
    pub stroke_count: u8,
    /// SVG path data in font space, one per stroke, as [`Kanji::outlines`].
    pub outlines: Vec<String>,
    /// Centre-lines in display space, as [`Kanji::medians`].
    pub medians: Vec<Vec<Point>>,
}

impl Radical {
    /// True when the board can draw and grade this radical: it has geometry, and
    /// as many centre-lines as outlines.
    ///
    /// Every radical in the shipped artifact satisfies this — `prepare-kanji`
    /// refuses to write one that does not — but the shape a course screen needs
    /// to test is still the character one's, so it is here rather than assumed
    /// at the call site.
    pub fn is_practisable(&self) -> bool {
        !self.medians.is_empty() && self.medians.len() == self.outlines.len()
    }
}

/// One radical and the course's characters that are classified under it.
///
/// Derived rather than stored: a character already carries its radical number,
/// and this is that grouping. The grouping is by **number**, not by glyph,
/// which is the point — 持 writes 扌 and shares nothing visible with 手, and
/// 手's family is a list of characters that look nothing like each other and
/// belong together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadicalFamily {
    /// The classical number, 1–214.
    pub number: u8,
    /// The head form, from the radical table.
    pub ch: char,
    /// The head form's stroke count.
    pub stroke_count: u8,
    /// The jōyō characters classified under it, most frequent first. Empty for
    /// the 16 radicals no jōyō character uses — the family still exists, and a
    /// screen should say so rather than hide the radical.
    pub characters: Vec<char>,
}

/// Which EDRDG snapshot the readings in an artifact came from.
///
/// Both fields are read out of the KANJIDIC2 document itself rather than being
/// passed in beside it, so the artifact cannot claim a version its data does not
/// have. EDRDG's licence obliges the app to keep the data current, and an update
/// obligation that cannot be checked against the shipped artifact is one nobody
/// will notice lapsing — see `LICENSES.md`'s EDRDG section for the refresh
/// procedure this exists to make auditable.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanjiSource {
    /// KANJIDIC2's own `version`, e.g. `3.6.2`.
    pub kanjidic2_version: String,
    /// The dictionary date it was built from, e.g. `2026-09-28`.
    pub kanjidic2_date: String,
}

/// The decoded payload of the shipped kanji artifact.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KanjiArtifact {
    pub kanji: Vec<Kanji>,
    /// The 214 Kangxi radicals, in number order. Added in format version 2.
    pub radicals: Vec<Radical>,
    pub source: KanjiSource,
}

impl KanjiArtifact {
    pub fn new(kanji: Vec<Kanji>, radicals: Vec<Radical>, source: KanjiSource) -> Self {
        Self {
            kanji,
            radicals,
            source,
        }
    }
}

/// An indexed collection of kanji, ordered by code point, and the radical table.
///
/// Code-point order is the storage order and nothing more: it is deterministic,
/// which is what makes a rebuild byte-identical, and the *teaching* order is the
/// grade ladder, which [`KanjiDataset::of_grade`] serves and
/// [`crate::kanji_lessons`] turns into lessons.
#[derive(Clone, Debug, Default)]
pub struct KanjiDataset {
    kanji: Vec<Kanji>,
    radicals: Vec<Radical>,
    source: KanjiSource,
}

impl KanjiDataset {
    /// A dataset of characters and no radical table.
    ///
    /// What the unit tests and the words pipeline's fixtures want: grouping
    /// characters is the part under test, and the table is 214 rows of geometry
    /// that say nothing about it. [`KanjiDataset::from_parts`] is what the
    /// artifact decodes through.
    pub fn from_kanji(mut kanji: Vec<Kanji>, source: KanjiSource) -> Self {
        kanji.sort_by_key(|k| k.ch as u32);
        Self {
            kanji,
            radicals: Vec::new(),
            source,
        }
    }

    /// A dataset with its radical table, both sorted deterministically.
    pub fn from_parts(
        mut kanji: Vec<Kanji>,
        mut radicals: Vec<Radical>,
        source: KanjiSource,
    ) -> Self {
        kanji.sort_by_key(|k| k.ch as u32);
        radicals.sort_by_key(|r| r.number);
        Self {
            kanji,
            radicals,
            source,
        }
    }

    /// Drop any character that cannot be practised, so a course can never offer
    /// something the board cannot grade.
    pub fn from_kanji_teachable(kanji: Vec<Kanji>, source: KanjiSource) -> Self {
        Self::from_kanji(
            kanji.into_iter().filter(Kanji::is_practisable).collect(),
            source,
        )
    }

    /// Decode an artifact produced by `prepare-kanji`: gzip-compressed, magic
    /// prefixed, then a `postcard`-encoded [`KanjiArtifact`].
    pub fn from_gzip_bytes(bytes: &[u8]) -> std::io::Result<Self> {
        use std::io::Read;

        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut raw = Vec::new();
        decoder.read_to_end(&mut raw)?;

        if raw.len() < KANJI_ARTIFACT_MAGIC.len()
            || &raw[..KANJI_ARTIFACT_MAGIC.len()] != KANJI_ARTIFACT_MAGIC
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "not a kanji dataset artifact (bad or outdated magic); re-run `prepare-kanji`",
            ));
        }
        let artifact: KanjiArtifact =
            postcard::from_bytes(&raw[KANJI_ARTIFACT_MAGIC.len()..]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("corrupt kanji dataset artifact: {e}"),
                )
            })?;
        Ok(Self::from_parts(
            artifact.kanji,
            artifact.radicals,
            artifact.source,
        ))
    }

    pub fn kanji(&self) -> &[Kanji] {
        &self.kanji
    }

    pub fn len(&self) -> usize {
        self.kanji.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kanji.is_empty()
    }

    pub fn get(&self, ch: char) -> Option<&Kanji> {
        self.kanji.iter().find(|k| k.ch == ch)
    }

    /// Which EDRDG snapshot these readings came from.
    pub fn source(&self) -> &KanjiSource {
        &self.source
    }

    /// Every character of one grade, in code-point order.
    pub fn of_grade(&self, grade: u8) -> impl Iterator<Item = &Kanji> {
        self.kanji.iter().filter(move |k| k.grade == grade)
    }

    /// How many characters each grade holds, in [`JOYO_GRADES`] order.
    pub fn grade_counts(&self) -> Vec<(u8, usize)> {
        JOYO_GRADES
            .iter()
            .map(|&grade| (grade, self.of_grade(grade).count()))
            .collect()
    }

    /// The 214 radicals, in number order. Empty for a dataset built without a
    /// table — see [`KanjiDataset::from_kanji`].
    pub fn radicals(&self) -> &[Radical] {
        &self.radicals
    }

    /// One radical by its classical number.
    pub fn radical(&self, number: u8) -> Option<&Radical> {
        self.radicals.iter().find(|r| r.number == number)
    }

    /// The head form of the radical a character is classified under.
    ///
    /// `None` when the dataset carries no radical table, or when the character's
    /// number is outside 1–214 — which is a data error rather than a case to
    /// paper over with the combining form, because the two shapes mean different
    /// things on a screen.
    pub fn head_form_of(&self, kanji: &Kanji) -> Option<&Radical> {
        self.radical(kanji.radical_number)
    }

    /// Every radical with the characters the course classifies under it.
    ///
    /// All 214 are returned, in number order, including the ones no jōyō
    /// character uses — a learner looking up 黹 should find it with an empty
    /// family rather than find nothing, because "no jōyō character uses this" is
    /// a fact about the set and not a hole in the panel.
    ///
    /// Members are ordered the way the course orders characters: most frequent
    /// first, and the ones KANJIDIC2 does not rank last, by code point.
    pub fn radical_families(&self) -> Vec<RadicalFamily> {
        self.radicals
            .iter()
            .map(|radical| RadicalFamily {
                number: radical.number,
                ch: radical.ch,
                stroke_count: radical.stroke_count,
                characters: self.characters_under(radical.number),
            })
            .collect()
    }

    /// One radical's family, for a screen that has opened it.
    ///
    /// `None` when the dataset carries no radical table, or when the number is
    /// outside it — 0 and 215 are not radicals, and a caller that asks for one
    /// should be told rather than handed an empty family.
    pub fn radical_family(&self, number: u8) -> Option<RadicalFamily> {
        self.radical(number).map(|radical| RadicalFamily {
            number: radical.number,
            ch: radical.ch,
            stroke_count: radical.stroke_count,
            characters: self.characters_under(radical.number),
        })
    }

    /// The course's characters classified under one radical number, in course
    /// order.
    fn characters_under(&self, number: u8) -> Vec<char> {
        let mut members: Vec<&Kanji> = self
            .kanji
            .iter()
            .filter(|k| k.radical_number == number)
            .collect();
        members.sort_by_key(|k| (k.frequency.is_none(), k.frequency, k.ch as u32));
        members.into_iter().map(|k| k.ch).collect()
    }
}

/// Split the upstream `radical` field into the character and its note.
///
/// The field is either the radical alone (`子`) or the radical followed by what
/// it stands for in parentheses (`扌 (手)`). The note is kept as text rather than
/// as a character because it is not always one: 阝 is `阝 (阜 or 邑)`.
///
/// A field with no radical in it is a bug in the pipeline rather than something
/// to paper over, so the caller gets `None` and says so.
pub fn parse_radical(field: &str) -> Option<(char, Option<String>)> {
    let field = field.trim();
    let (main, note) = match field.find('(') {
        Some(open) => {
            let note = field[open + 1..]
                .trim_end()
                .strip_suffix(')')
                .unwrap_or_else(|| field[open + 1..].trim_end())
                .trim();
            (
                field[..open].trim(),
                if note.is_empty() {
                    None
                } else {
                    Some(note.to_string())
                },
            )
        }
        None => (field, None),
    };

    let mut chars = main.chars();
    let radical = chars.next()?;
    // More than one character before the parenthesis would mean this is not the
    // shape the field is documented to have, and taking the first and dropping
    // the rest would silently mis-assign a radical.
    if chars.next().is_some() {
        return None;
    }
    Some((radical, note))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_radical_is_split_from_its_note() {
        assert_eq!(parse_radical("子"), Some(('子', None)));
        assert_eq!(
            parse_radical("扌 (手)"),
            Some(('扌', Some("手".to_string())))
        );
        // The one radical whose note is prose rather than a character.
        assert_eq!(
            parse_radical("阝 (阜 or 邑)"),
            Some(('阝', Some("阜 or 邑".to_string())))
        );
        // Tolerated shapes, because the field comes from a text file.
        assert_eq!(parse_radical("  子  "), Some(('子', None)));
        assert_eq!(parse_radical("扌 (手)  "), Some(('扌', Some("手".to_string()))));
        assert_eq!(parse_radical("子 ()"), Some(('子', None)));
    }

    #[test]
    fn a_radical_that_is_not_one_character_is_refused_rather_than_truncated() {
        assert_eq!(parse_radical(""), None);
        assert_eq!(parse_radical(" ()"), None);
        assert_eq!(parse_radical("日月"), None);
    }

    #[test]
    fn grades_split_into_kyoiku_and_the_rest() {
        let kanji = |grade| Kanji {
            ch: '一',
            grade,
            stroke_count: 1,
            frequency: None,
            radical: '一',
            radical_note: None,
            radical_number: 1,
            on: Vec::new(),
            kun: Vec::new(),
            meanings: Vec::new(),
            nanori: Vec::new(),
            decomposition: String::new(),
            outlines: vec!["M0,0".to_string()],
            medians: vec![vec![Point::new(0.0, 0.0)]],
        };
        assert!(kanji(1).is_kyoiku());
        assert!(kanji(6).is_kyoiku());
        assert!(!kanji(8).is_kyoiku(), "grade 8 is the jōyō remainder");
        assert!(kanji(1).is_practisable());
    }

    #[test]
    fn the_grades_the_artifact_carries_are_seven_and_sum_to_joyo() {
        assert_eq!(JOYO_GRADES.len(), 7);
        assert_eq!(JOYO_GRADES.iter().filter(|g| **g == 8).count(), 1);
        assert!(!JOYO_GRADES.contains(&7), "KANJIDIC2 has no grade 7");
        assert!(!JOYO_GRADES.contains(&9), "grade 9 is jinmeiyō");
    }

    /// A `Kanji` with geometry; the tests above need one shape, the artifact
    /// tests take theirs from the real thing.
    fn sample() -> Kanji {
        Kanji {
            ch: '学',
            grade: 1,
            stroke_count: 2,
            frequency: Some(63),
            radical: '子',
            radical_note: None,
            radical_number: 39,
            on: vec!["ガク".to_string()],
            kun: vec!["まな.ぶ".to_string()],
            meanings: vec!["study".to_string()],
            nanori: vec!["たか".to_string()],
            decomposition: "⿳𰃮子".to_string(),
            outlines: vec!["M0,0".to_string(), "M1,1".to_string()],
            medians: vec![
                vec![Point::new(0.0, 0.0)],
                vec![Point::new(1.0, 1.0)],
            ],
        }
    }

    #[test]
    fn a_dataset_is_ordered_by_code_point_whatever_order_it_was_built_in() {
        let mut later = sample();
        later.ch = '鳥';
        let early = sample();
        let dataset = KanjiDataset::from_kanji(vec![later, early], KanjiSource::default());
        assert_eq!(dataset.kanji()[0].ch, '学');
        assert_eq!(dataset.kanji()[1].ch, '鳥');
        assert_eq!(dataset.get('学').map(|k| k.grade), Some(1));
        assert!(dataset.get('一').is_none());
    }

    #[test]
    fn a_dataset_drops_what_cannot_be_drawn() {
        let mut no_geometry = sample();
        no_geometry.ch = '乙';
        no_geometry.medians.clear();
        let dataset =
            KanjiDataset::from_kanji_teachable(vec![sample(), no_geometry], KanjiSource::default());
        assert_eq!(dataset.len(), 1);
        assert_eq!(dataset.kanji()[0].ch, '学');
    }

    #[test]
    fn grade_counts_follow_the_grades_the_artifact_carries() {
        let mut fifth = sample();
        fifth.grade = 5;
        let dataset = KanjiDataset::from_kanji(vec![sample(), fifth], KanjiSource::default());
        let counts = dataset.grade_counts();
        assert_eq!(counts.len(), JOYO_GRADES.len());
        assert_eq!(counts[0], (1, 1));
        assert_eq!(counts[4], (5, 1));
        assert_eq!(counts.iter().map(|(_, n)| n).sum::<usize>(), 2);
    }

    /// A `Radical` with geometry, for the grouping tests.
    fn radical(number: u8, ch: char) -> Radical {
        Radical {
            number,
            ch,
            stroke_count: 2,
            outlines: vec!["M0,0".to_string(), "M1,1".to_string()],
            medians: vec![
                vec![Point::new(0.0, 0.0)],
                vec![Point::new(1.0, 1.0)],
            ],
        }
    }

    /// One character, classified under a radical number, with the combining form
    /// and the frequency the grouping reads.
    fn under(ch: char, radical_number: u8, form: char, frequency: Option<u16>) -> Kanji {
        let mut k = sample();
        k.ch = ch;
        k.radical = form;
        k.radical_number = radical_number;
        k.frequency = frequency;
        k
    }

    #[test]
    fn families_group_by_radical_number_and_not_by_the_glyph() {
        // 持 writes 扌 and 手 writes 手: nothing visible in common, one radical
        // number, one family — and the family is named by the head form.
        let dataset = KanjiDataset::from_parts(
            vec![
                under('持', 64, '扌', Some(500)),
                under('手', 64, '手', Some(100)),
                under('口', 30, '口', Some(7)),
            ],
            vec![radical(64, '手'), radical(30, '口'), radical(1, '一')],
            KanjiSource::default(),
        );

        let families = dataset.radical_families();
        assert_eq!(families.len(), 3, "a radical with no characters is still a family");
        assert_eq!(
            families.iter().map(|f| f.number).collect::<Vec<_>>(),
            vec![1, 30, 64],
            "number order, whatever order the table arrived in"
        );

        let hand = families.iter().find(|f| f.number == 64).expect("64 is a family");
        assert_eq!(hand.ch, '手', "the head form, not the 扌 inside 持");
        assert_eq!(hand.characters, vec!['手', '持'], "the more frequent first");
        assert_eq!(hand.stroke_count, 2);

        let empty = families.iter().find(|f| f.number == 1).expect("1 is a family");
        assert!(
            empty.characters.is_empty(),
            "16 of the 214 radicals have no jōyō character, and they are still listed"
        );

        let held = dataset.get('持').expect("持 is held");
        assert_eq!(dataset.head_form_of(held).map(|r| r.ch), Some('手'));
    }

    #[test]
    fn an_unranked_character_sorts_after_the_ranked_ones_in_its_family() {
        let dataset = KanjiDataset::from_parts(
            vec![
                under('甲', 1, '一', None),
                under('乙', 1, '一', Some(900)),
            ],
            vec![radical(1, '一')],
            KanjiSource::default(),
        );
        let family = &dataset.radical_families()[0];
        assert_eq!(
            family.characters,
            vec!['乙', '甲'],
            "no frequency is not a very high frequency"
        );
    }

    #[test]
    fn a_dataset_without_the_radical_table_has_no_families_and_names_no_head_form() {
        // `from_kanji` is what the fixtures use. It must not invent a head form
        // out of the combining shape, which is a different thing on a screen.
        let dataset =
            KanjiDataset::from_kanji(vec![under('持', 64, '扌', Some(500))], KanjiSource::default());
        assert!(dataset.radicals().is_empty());
        assert!(dataset.radical_families().is_empty());
        assert!(dataset.radical(64).is_none());
        assert!(dataset.head_form_of(dataset.get('持').expect("持")).is_none());
    }

    #[test]
    fn a_radical_is_practisable_only_with_geometry_for_every_stroke() {
        let mut table = radical(64, '手');
        assert!(table.is_practisable());
        table.medians.pop();
        assert!(!table.is_practisable(), "two outlines and one centre-line is not a radical");
        table.medians.clear();
        assert!(!table.is_practisable());
    }

    #[test]
    fn an_artifact_that_is_not_one_is_refused_by_its_magic() {
        // A kana artifact is the realistic mistake: the two live side by side.
        let kana = b"KANAD001\x00\x00\x00\x00";
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut encoder, kana).unwrap();
        let bytes = encoder.finish().unwrap();
        let err = KanjiDataset::from_gzip_bytes(&bytes).unwrap_err();
        assert!(err.to_string().contains("not a kanji dataset artifact"), "{err}");
    }

    #[test]
    fn source_defaults_to_nothing_rather_than_to_a_claim() {
        let dataset = KanjiDataset::default();
        assert_eq!(dataset.source().kanjidic2_version, "");
        assert!(dataset.is_empty());
    }
}
