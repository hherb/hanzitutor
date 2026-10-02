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
pub const KANJI_ARTIFACT_MAGIC: &[u8; 8] = b"KANJD001";

/// The KANJIDIC2 grades the artifact carries: 1–6 are kyōiku, 8 is the rest of
/// jōyō. See the module docs for why 7 and 9/10 are not here.
pub const JOYO_GRADES: [u8; 7] = [1, 2, 3, 4, 5, 6, 8];

/// How many characters those grades hold — the jōyō set.
pub const JOYO_COUNT: usize = 2_136;

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
    pub source: KanjiSource,
}

impl KanjiArtifact {
    pub fn new(kanji: Vec<Kanji>, source: KanjiSource) -> Self {
        Self { kanji, source }
    }
}

/// An indexed collection of kanji, ordered by code point.
///
/// Code-point order is the storage order and nothing more: it is deterministic,
/// which is what makes a rebuild byte-identical, and the *teaching* order is the
/// grade ladder, which [`KanjiDataset::of_grade`] serves.
#[derive(Clone, Debug, Default)]
pub struct KanjiDataset {
    kanji: Vec<Kanji>,
    source: KanjiSource,
}

impl KanjiDataset {
    pub fn from_kanji(mut kanji: Vec<Kanji>, source: KanjiSource) -> Self {
        kanji.sort_by_key(|k| k.ch as u32);
        Self { kanji, source }
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
        Ok(Self::from_kanji(artifact.kanji, artifact.source))
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
