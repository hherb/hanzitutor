//! The course: the gojūon grid for the kana, the grade ladder for the kanji, and
//! the kana learners actually confuse.
//!
//! # The kana course
//!
//! The grid is the curriculum. A kana is learned as part of a row — あ い う え お
//! together, then か き く け こ — because the row is what makes the five vowels
//! audible as the same sound in different consonants, and because it is the order
//! every textbook and every kana chart uses. Code-point order happens to be
//! gojūon order for the base kana, but **not** for the voiced ones: が sits
//! between か and き in code points, so the rows cannot be recovered by slicing
//! the sorted kana and are written out here instead.
//!
//! Katakana are not listed twice. They are the same sounds in the same order, one
//! code point apart, so the rows hold hiragana and [`Row::kana`] converts.
//!
//! # The kanji course
//!
//! The kanji have no gojūon: their order is the one Japanese schools teach in,
//! which is the **kyōiku grade** — grades 1 to 6 and then the jōyō remainder —
//! and it comes from KANJIDIC2 rather than from a list here, because the ministry
//! moves characters between grades and the current assignment is the one a
//! learner meets. [`kanji_lessons`] slices each grade into lessons, most frequent
//! first, and the ladder is deliberately the *same* one the vocabulary's bands
//! use (see [`grade_name`]): a word enters the course when its kanji do.
//!
//! Both courses filter through their dataset, so neither can list something the
//! board cannot draw.

use crate::{words::band_name, JOYO_GRADES};
use crate::{KanaDataset, Kanji, KanjiDataset, Script};

/// Convert a hiragana to its katakana counterpart, if it has one.
///
/// The two blocks are the same 86 characters, `0x60` apart. ー and the v-series
/// katakana have no hiragana form and are not produced by this.
pub fn to_katakana(ch: char) -> Option<char> {
    match ch as u32 {
        0x3041..=0x3096 => char::from_u32(ch as u32 + 0x60),
        _ => None,
    }
}

/// Convert a katakana to its hiragana counterpart, if it has one.
pub fn to_hiragana(ch: char) -> Option<char> {
    match ch as u32 {
        0x30A1..=0x30F6 => char::from_u32(ch as u32 - 0x60),
        _ => None,
    }
}

/// Convert a hiragana to the same kana in `script`, where the script has one.
fn in_script(ch: char, script: Script) -> Option<char> {
    match script {
        Script::Hiragana => Some(ch),
        Script::Katakana => to_katakana(ch),
    }
}

/// The five columns of the gojūon grid, in order. The index is the column.
pub const VOWEL_COLUMNS: [char; 5] = ['a', 'i', 'u', 'e', 'o'];

/// One row of the gojūon grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// The kana the row is named for, in hiragana.
    pub head: char,
    /// The row's Hepburn label — `a`, `ka`, `sha`-less `sa`, and so on.
    pub sound: &'static str,
    /// The row's five **slots** — a, i, u, e, o — in hiragana, with `None` where
    /// the row has no kana.
    ///
    /// The slots are the grid rather than a short list, because a row's kana are
    /// not contiguous: や's three sit in the a, u and o columns, so drawing them
    /// left-aligned would put ゆ under い and teach the wrong vowel. The holes are
    /// therefore data, and `every_cell_sits_in_the_column_its_vowel_names` checks
    /// each filled slot against the kana's own reading rather than trusting the
    /// table above to have been typed correctly.
    pub cells: [Option<char>; 5],
    /// True for a row that is the voiced or semi-voiced form of another row.
    /// These are taught after the plain rows, not beside them.
    pub voiced: bool,
}

impl Row {
    /// The row's kana in the given script, in gojūon order, holes skipped.
    pub fn kana(&self, script: Script) -> impl Iterator<Item = char> + '_ {
        self.cells.iter().flatten().copied().filter_map(move |ch| in_script(ch, script))
    }

    /// The row's five slots in the given script, `None` where the row has no
    /// kana — the row as a chart draws it.
    pub fn scripted_cells(&self, script: Script) -> [Option<char>; 5] {
        self.cells.map(|cell| cell.and_then(|ch| in_script(ch, script)))
    }

    /// How many kana the row has.
    pub fn len(&self) -> usize {
        self.cells.iter().flatten().count()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(Option::is_none)
    }

    /// A stable key for the row, e.g. `"ka"`, for use as a DOM key or a test id.
    pub fn key(&self) -> String {
        self.sound.to_string()
    }
}

/// The gojūon grid: eleven plain rows (46 kana) then the five voiced rows
/// (25 kana — 20 dakuten and 5 handakuten).
///
/// Each row is written as its five slots, so the holes are visible in the table
/// itself: や has none in the i and e columns, わ has none but a and o, and ん is
/// a row of one. Nine of the eighty slots are holes.
pub const ROWS: &[Row] = &[
    // The plain rows: the 46 kana of the gojūon proper.
    Row { head: 'あ', sound: "a", cells: [Some('あ'), Some('い'), Some('う'), Some('え'), Some('お')], voiced: false },
    Row { head: 'か', sound: "ka", cells: [Some('か'), Some('き'), Some('く'), Some('け'), Some('こ')], voiced: false },
    Row { head: 'さ', sound: "sa", cells: [Some('さ'), Some('し'), Some('す'), Some('せ'), Some('そ')], voiced: false },
    Row { head: 'た', sound: "ta", cells: [Some('た'), Some('ち'), Some('つ'), Some('て'), Some('と')], voiced: false },
    Row { head: 'な', sound: "na", cells: [Some('な'), Some('に'), Some('ぬ'), Some('ね'), Some('の')], voiced: false },
    Row { head: 'は', sound: "ha", cells: [Some('は'), Some('ひ'), Some('ふ'), Some('へ'), Some('ほ')], voiced: false },
    Row { head: 'ま', sound: "ma", cells: [Some('ま'), Some('み'), Some('む'), Some('め'), Some('も')], voiced: false },
    // や has three: the i and e positions were never filled.
    Row { head: 'や', sound: "ya", cells: [Some('や'), None, Some('ゆ'), None, Some('よ')], voiced: false },
    Row { head: 'ら', sound: "ra", cells: [Some('ら'), Some('り'), Some('る'), Some('れ'), Some('ろ')], voiced: false },
    // を is the particle, and it is the row's o column; ゐ and ゑ are obsolete and
    // are taught separately.
    Row { head: 'わ', sound: "wa", cells: [Some('わ'), None, None, None, Some('を')], voiced: false },
    Row { head: 'ん', sound: "n", cells: [Some('ん'), None, None, None, None], voiced: false },
    // The voiced rows.
    Row { head: 'が', sound: "ga", cells: [Some('が'), Some('ぎ'), Some('ぐ'), Some('げ'), Some('ご')], voiced: true },
    Row { head: 'ざ', sound: "za", cells: [Some('ざ'), Some('じ'), Some('ず'), Some('ぜ'), Some('ぞ')], voiced: true },
    Row { head: 'だ', sound: "da", cells: [Some('だ'), Some('ぢ'), Some('づ'), Some('で'), Some('ど')], voiced: true },
    Row { head: 'ば', sound: "ba", cells: [Some('ば'), Some('び'), Some('ぶ'), Some('べ'), Some('ぼ')], voiced: true },
    Row { head: 'ぱ', sound: "pa", cells: [Some('ぱ'), Some('ぴ'), Some('ぷ'), Some('ぺ'), Some('ぽ')], voiced: true },
];

/// The small kana that modify the kana before them rather than standing alone.
pub const SMALL_KANA: &[char] = &['ぁ', 'ぃ', 'ぅ', 'ぇ', 'ぉ', 'ゃ', 'ゅ', 'ょ', 'っ', 'ゎ'];

/// The kana that are real but rare: obsolete ゐ ゑ, the modern ゔ, and the small
/// ゕ ゖ.
pub const RARE_KANA: &[char] = &['ゐ', 'ゑ', 'ゔ', 'ゕ', 'ゖ'];

/// The v-series, which exists only in katakana, for foreign words.
pub const KATAKANA_ONLY: &[char] = &['ヷ', 'ヸ', 'ヹ', 'ヺ'];

/// The prolonged sound mark. Not a sound: it lengthens the vowel before it.
pub const CHOONPU: char = 'ー';

/// One lesson: a titled group of kana to learn together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lesson {
    /// Stable key, e.g. `"hiragana-ka"`.
    pub key: String,
    /// What the sidebar should call it, e.g. `"か き く け こ — ka"`.
    pub title: String,
    /// The kana of the lesson, in teaching order.
    pub kana: Vec<char>,
    /// True for the voiced lessons, so a screen can group or badge them.
    pub voiced: bool,
}

impl Lesson {
    pub fn len(&self) -> usize {
        self.kana.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kana.is_empty()
    }
}

/// The course for one script: the plain rows, then the voiced rows, then the
/// small kana, then the rare ones.
///
/// Only kana present in `dataset` are included, so a lesson can never ask for
/// something the board cannot draw or grade.
pub fn lessons(dataset: &KanaDataset, script: Script) -> Vec<Lesson> {
    let mut out: Vec<Lesson> = ROWS
        .iter()
        .filter_map(|row| {
            let kana: Vec<char> = row.kana(script).filter(|ch| dataset.get(*ch).is_some()).collect();
            if kana.is_empty() {
                return None;
            }
            let drawn = draw(&kana);
            Some(Lesson {
                key: format!("{}-{}", script.name(), row.sound),
                title: format!("{drawn} — {}", row.sound),
                kana,
                voiced: row.voiced,
            })
        })
        .collect();
    out.extend(off_grid(dataset, script));
    out
}

/// The kana a chart shows **outside** the grid proper, as lessons: the small
/// kana, the rare ones, and — in katakana only — the v-series and the prolonged
/// sound mark.
///
/// This is what makes the course and the chart one answer rather than two:
/// [`lessons`] is the grid plus this, and the chart draws the grid and this side
/// by side, so neither can teach a kana the other has never heard of.
pub fn off_grid(dataset: &KanaDataset, script: Script) -> Vec<Lesson> {
    let mut out: Vec<Lesson> = Vec::new();
    let mut push = |key: &str, label: &str, members: &[char], mirror: bool| {
        let kana: Vec<char> = members
            .iter()
            .copied()
            .filter_map(|ch| if mirror { in_script(ch, script) } else { Some(ch) })
            .filter(|ch| dataset.get(*ch).is_some())
            .collect();
        if kana.is_empty() {
            return;
        }
        // The title is drawn from the kana themselves rather than written out,
        // which is what keeps the katakana course from listing the hiragana
        // small kana beside the katakana ones.
        out.push(Lesson {
            key: format!("{}-{key}", script.name()),
            title: format!("{label} — {}", draw(&kana)),
            kana,
            voiced: false,
        });
    };

    push("small", "Small kana", SMALL_KANA, true);
    push("rare", "Rare kana", RARE_KANA, true);
    // Katakana has five characters hiragana does not: the v-series, which exists
    // only for foreign words, and the prolonged sound mark, which is not a sound
    // at all but lengthens the vowel before it. Without these the katakana course
    // would teach 86 of its 91 kana and quietly omit ヷ ヸ ヹ ヺ ー.
    if script == Script::Katakana {
        push("v", "V-series", KATAKANA_ONLY, false);
        push("choonpu", "Prolonged sound mark", &[CHOONPU], false);
    }

    out
}

/// A lesson's or a row's kana, written out for a title: `"あ い う え お"`.
fn draw(kana: &[char]) -> String {
    kana.iter().map(|ch| ch.to_string()).collect::<Vec<_>>().join(" ")
}

/// How many characters a kanji lesson holds.
///
/// Ten, the same group size the Chinese course uses: small enough that a lesson
/// is a sitting rather than a project, large enough that the grade ladder's 2,136
/// characters are 214 lessons instead of a wall.
pub const KANJI_LESSON_SIZE: usize = 10;

/// One lesson of the kanji course: a group of characters from one grade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanjiLesson {
    /// Stable key, e.g. `"g1-3"` — grade 1, third lesson.
    pub key: String,
    /// Where the lesson sits inside its grade, e.g. `"21–30"`. The grade is on
    /// the lesson view beside it, so this need not repeat it.
    pub title: String,
    /// KANJIDIC2's grade: 1–6 kyōiku, 8 the jōyō remainder. See [`grade_name`].
    pub grade: u8,
    /// The characters to study, in teaching order.
    pub kanji: Vec<char>,
}

impl KanjiLesson {
    pub fn len(&self) -> usize {
        self.kanji.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kanji.is_empty()
    }
}

/// What a screen calls one grade of the kanji course.
///
/// The names are the **vocabulary ladder's** ([`crate::band_name`]), because the
/// two are one ladder: a word's band is the highest grade among its kanji, so
/// "kyōiku 3" means the same thing whether it labels a band of words or a grade of
/// characters. `ROADMAP_NIHONGO.md` left that as an open question for this
/// milestone; this is it answered, and the answer is that a learner meets a
/// character and the vocabulary that uses it in the same band.
pub fn grade_name(grade: u8) -> &'static str {
    band_name(crate::words::band_for([grade]))
}

/// The kanji course: every grade in teaching order, each sliced into lessons.
///
/// The order is [`JOYO_GRADES`] — kyōiku 1 to 6, then the jōyō remainder — and
/// inside a grade the most frequent characters come first, by KANJIDIC2's
/// frequency rank, with the ones it does not rank last and in code-point order.
/// That is the only ordering the data supports: the grades are a fixed set with
/// no sequence inside them, and a learner meeting 日 before 鬱 is worth more than
/// one meeting them in code-point order.
///
/// Only characters in `dataset` are included, so a lesson can never ask for
/// something the board cannot draw.
pub fn kanji_lessons(dataset: &KanjiDataset, lesson_size: usize) -> Vec<KanjiLesson> {
    let size = lesson_size.max(1);
    let mut out = Vec::new();

    for &grade in &JOYO_GRADES {
        let mut of_grade: Vec<&Kanji> = dataset.of_grade(grade).collect();
        of_grade.sort_by_key(|k| (k.frequency.is_none(), k.frequency, k.ch as u32));

        for (index, chunk) in of_grade.chunks(size).enumerate() {
            let first = index * size + 1;
            let last = first + chunk.len() - 1;
            out.push(KanjiLesson {
                key: format!("g{grade}-{}", index + 1),
                title: if first == last {
                    first.to_string()
                } else {
                    format!("{first}\u{2013}{last}")
                },
                grade,
                kanji: chunk.iter().map(|k| k.ch).collect(),
            });
        }
    }

    out
}

/// One yōon: a kana from the i-column followed by a small ゃ, ゅ or ょ, which
/// together make a single sound rather than two.
///
/// The digraph is two characters but one mora, and the small kana must be small —
/// きや is `kiya`, two morae, while きゃ is `kya`, one. That distinction is the
/// whole point, and it is why these cannot be taught as an ordinary two-kana
/// sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yoon {
    /// Stable key, e.g. `"hiragana-kyu"`.
    pub key: String,
    /// The digraph as written, e.g. `"きゅ"`.
    pub display: String,
    /// Hepburn, e.g. `"kyu"`.
    pub hepburn: String,
    /// Kunrei-shiki, which is the same as Hepburn except after し ち じ.
    pub kunrei: String,
    /// The two characters, base first, in the requested script.
    pub kana: Vec<char>,
    /// The same consonant and vowel written with **full-size** kana: きゃ → きや.
    ///
    /// This is the contrast the yōon drill teaches — one mora against two — and
    /// it belongs to the yōon rather than to the drill, so it is derived here
    /// beside the digraph it is derived from.
    pub plain: Vec<char>,
    /// Its reading: きや is `kiya`.
    pub plain_hepburn: String,
    /// Its Kunrei-shiki reading: しや is `siya`.
    pub plain_kunrei: String,
}

/// The i-column kana that take a small ゃ ゅ ょ, with the consonant each
/// contributes. Hepburn and Kunrei differ only in how they write the s, t and z
/// rows, which is why there are two stems.
const YOON_BASES: &[(char, &str, &str)] = &[
    // (hiragana, Hepburn stem, Kunrei stem)
    ('き', "ky", "ky"),
    ('し', "sh", "sy"),
    ('ち', "ch", "ty"),
    ('に', "ny", "ny"),
    ('ひ', "hy", "hy"),
    ('み', "my", "my"),
    ('り', "ry", "ry"),
    ('ぎ', "gy", "gy"),
    ('じ', "j", "zy"),
    ('び', "by", "by"),
    ('ぴ', "py", "py"),
];

/// The three small kana that form a yōon, each with the **full-size** kana it
/// stands for and the vowel it contributes.
///
/// The full-size form is what makes the contrast a learner has to hear: きゃ is
/// one mora, きや is two, and they are written with the same two code points in
/// different sizes. Hepburn and Kunrei both spell the small kana's vowel the same
/// way — the two systems differ over the consonant (し is `shi`/`si`) and never
/// over this — so one vowel per row is enough.
const YOON_SMALL: &[(char, char, &str)] = &[('ゃ', 'や', "a"), ('ゅ', 'ゆ', "u"), ('ょ', 'よ', "o")];

/// Every yōon for one script: eleven bases × three small kana = 33 digraphs.
pub fn yoon(script: Script) -> Vec<Yoon> {
    let mut out = Vec::new();
    for &(base, hepburn_stem, kunrei_stem) in YOON_BASES {
        for &(small, full, vowel) in YOON_SMALL {
            let (Some(base), Some(small), Some(full)) = (
                in_script(base, script),
                in_script(small, script),
                in_script(full, script),
            ) else {
                continue;
            };
            // The two-mora counterpart's reading is composed from the two kana
            // that spell it — き ("ki") and や ("ya") make きや — from the same
            // table the rest of the app reads, rather than written out a second
            // time. `the_plain_counter_of_every_yoon_is_what_the_input_engine_types`
            // is the check on that composition.
            let (Some(base_reading), Some(full_reading)) =
                (crate::reading(base), crate::reading(full))
            else {
                continue;
            };
            let (Some(base_hepburn), Some(full_hepburn)) =
                (base_reading.hepburn.first(), full_reading.hepburn.first())
            else {
                continue;
            };
            let base_kunrei = base_reading.kunrei.first().copied().unwrap_or(base_hepburn);
            let full_kunrei = full_reading.kunrei.first().copied().unwrap_or(full_hepburn);
            let hepburn = format!("{hepburn_stem}{vowel}");
            out.push(Yoon {
                key: format!("{}-{hepburn}", script.name()),
                display: format!("{base}{small}"),
                kunrei: format!("{kunrei_stem}{vowel}"),
                hepburn,
                kana: vec![base, small],
                plain: vec![base, full],
                plain_hepburn: format!("{base_hepburn}{full_hepburn}"),
                plain_kunrei: format!("{base_kunrei}{full_kunrei}"),
            });
        }
    }
    out
}

/// A pair of kana that learners mix up, and what tells them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Confusable {
    pub a: char,
    pub b: char,
    /// The distinguishing feature, in one line, to show beside the two.
    pub tell: &'static str,
}

/// The classic confusions, katakana first because that is where most of them
/// live — the shapes are angular and the differences are direction, not topology.
///
/// This is the set every kana teacher reaches for; it is practitioner consensus
/// rather than a cited taxonomy, and the app should grow it from the learner's own
/// wrong answers rather than treating it as complete.
pub const CONFUSABLE: &[Confusable] = &[
    Confusable { a: 'シ', b: 'ツ', tell: "シ's strokes run left to right and up; ツ's run top to bottom" },
    Confusable { a: 'ソ', b: 'ン', tell: "ソ's first stroke falls left to right; ン's rises left to right" },
    Confusable { a: 'ク', b: 'ワ', tell: "ク has two strokes; ワ is one" },
    Confusable { a: 'ク', b: 'フ', tell: "ク's first stroke is short and set high; フ's is the whole top" },
    Confusable { a: 'チ', b: 'テ', tell: "チ's first stroke is short and across the top; テ's is long and level" },
    Confusable { a: 'ア', b: 'マ', tell: "ア's second stroke is a short down-stroke; マ's crosses and hooks" },
    Confusable { a: 'ね', b: 'れ', tell: "ね closes its loop; れ does not" },
    Confusable { a: 'れ', b: 'わ', tell: "れ's last stroke curls right and stops; わ's loops round" },
    Confusable { a: 'る', b: 'ろ', tell: "る finishes with a loop; ろ stops at the curve" },
    Confusable { a: 'は', b: 'ほ', tell: "は has two strokes after the vertical; ほ has three" },
    Confusable { a: 'ぬ', b: 'め', tell: "ぬ ends in a loop; め crosses and stops" },
    Confusable { a: 'り', b: 'リ', tell: "the hiragana り joins at the bottom; the katakana リ does not" },
    Confusable { a: 'わ', b: 'ゐ', tell: "ゐ is obsolete: two strokes and no loop" },
];

/// Every kana that appears in a confusion pair, with the ones it is confused
/// with, in the order [`CONFUSABLE`] lists them.
pub fn confusions_for(dataset: &KanaDataset, ch: char) -> Vec<Confusable> {
    let _ = dataset;
    CONFUSABLE
        .iter()
        .copied()
        .filter(|pair| pair.a == ch || pair.b == ch)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dataset() -> KanaDataset {
        KanaDataset::from_gzip_bytes(include_bytes!("../data/kana.bin.gz"))
            .expect("the committed artifact decodes")
    }

    #[test]
    fn the_plain_rows_are_the_forty_six_kana_of_the_gojuon() {
        let plain: usize = ROWS.iter().filter(|r| !r.voiced).map(Row::len).sum();
        assert_eq!(plain, 46);
    }

    #[test]
    fn the_voiced_rows_are_twenty_dakuten_and_five_handakuten() {
        let voiced: usize = ROWS.iter().filter(|r| r.voiced).map(Row::len).sum();
        assert_eq!(voiced, 25);
    }

    #[test]
    fn the_rows_cover_every_kana_the_lessons_leave_out() {
        // Plain + voiced + small + rare must account for all 86 hiragana, or a
        // kana exists that no lesson teaches.
        let dataset = dataset();
        let mut covered: Vec<char> = ROWS
            .iter()
            .flat_map(|r| r.cells.iter().flatten().copied())
            .chain(SMALL_KANA.iter().copied())
            .chain(RARE_KANA.iter().copied())
            .collect();
        let total = covered.len();
        covered.sort_unstable();
        covered.dedup();
        assert_eq!(total, covered.len(), "a kana is listed in two groups");

        let missing: Vec<char> = dataset
            .of_script(Script::Hiragana)
            .map(|k| k.ch)
            .filter(|ch| !covered.contains(ch))
            .collect();
        assert!(missing.is_empty(), "no lesson teaches {missing:?}");

        let extra: Vec<char> = covered
            .iter()
            .copied()
            .filter(|ch| dataset.get(*ch).is_none())
            .collect();
        assert!(extra.is_empty(), "the grid lists kana the app cannot draw: {extra:?}");
    }

    #[test]
    fn katakana_mirror_hiragana_in_the_rows() {
        for row in ROWS {
            let hira: Vec<char> = row.kana(Script::Hiragana).collect();
            let kata: Vec<char> = row.kana(Script::Katakana).collect();
            assert_eq!(hira.len(), kata.len(), "{} changes length across scripts", row.sound);
            for (h, k) in hira.iter().zip(&kata) {
                assert_eq!(to_hiragana(*k), Some(*h), "{k} should mirror {h}");
            }
        }
    }

    #[test]
    fn every_row_has_five_slots_and_the_holes_are_nine() {
        // The grid is sixteen rows of five slots, and the language leaves nine of
        // them empty: や's i and e, わ's i, u and e, and ん's four.
        let mut holes = 0;
        for row in ROWS {
            assert_eq!(row.cells.len(), 5, "{} is not a five-column row", row.sound);
            holes += row.cells.iter().filter(|cell| cell.is_none()).count();
        }
        assert_eq!(holes, 9);
        assert_eq!(ROWS.iter().map(Row::len).sum::<usize>(), 71, "46 plain + 25 voiced");
    }

    /// The layout above is stated rather than derived, so it is checked against
    /// something independent: a kana's own reading says which vowel it has, and
    /// the column it sits in has to be that vowel. This is what would catch ゆ
    /// drifting under い, which is the mistake a chart makes when it draws a short
    /// row left-aligned.
    #[test]
    fn every_cell_sits_in_the_column_its_vowel_names() {
        for row in ROWS {
            for (column, cell) in row.cells.iter().enumerate() {
                let Some(ch) = cell else { continue };
                // ん is the one kana with no vowel, and a chart puts it in the
                // first slot because its row holds nothing else.
                if *ch == 'ん' {
                    assert_eq!(column, 0, "ん is a row of one");
                    continue;
                }
                let reading = crate::reading(*ch)
                    .unwrap_or_else(|| panic!("{ch} in {} has no reading", row.sound));
                let vowel = reading.hepburn[0]
                    .chars()
                    .last()
                    .expect("every reading has letters");
                assert_eq!(
                    vowel, VOWEL_COLUMNS[column],
                    "{ch} is in the {} column of row {}, but it is read {}",
                    VOWEL_COLUMNS[column], row.sound, reading.hepburn[0]
                );
            }
        }
    }

    #[test]
    fn the_off_grid_groups_are_the_ones_the_course_adds_to_the_grid() {
        // The course *is* the grid plus the off-grid groups, so the chart can
        // draw one and the course teach the other without either inventing a kana.
        let dataset = dataset();
        for script in [Script::Hiragana, Script::Katakana] {
            let course = lessons(&dataset, script);
            let grid: Vec<char> = ROWS
                .iter()
                .flat_map(|row| row.kana(script).collect::<Vec<_>>())
                .collect();
            let off: Vec<char> = off_grid(&dataset, script)
                .iter()
                .flat_map(|lesson| lesson.kana.clone())
                .collect();
            let mut together: Vec<char> = grid.iter().copied().chain(off.iter().copied()).collect();
            let total = together.len();
            together.sort_unstable();
            together.dedup();
            assert_eq!(total, together.len(), "{script:?} lists a kana twice");

            let taught: Vec<char> = course.iter().flat_map(|l| l.kana.iter().copied()).collect();
            assert_eq!(together, {
                let mut taught = taught;
                taught.sort_unstable();
                taught
            });
        }
    }

    #[test]
    fn a_katakana_lesson_names_katakana_kana() {
        // The small and rare lessons are built from hiragana and converted, so a
        // title written out by hand would list ゃ ゅ ょ っ in the katakana course.
        let dataset = dataset();
        for (key, kana) in [
            ("katakana-small", vec!['ァ', 'ィ', 'ゥ', 'ェ', 'ォ', 'ャ', 'ュ', 'ョ', 'ッ', 'ヮ']),
            ("katakana-rare", vec!['ヰ', 'ヱ', 'ヴ', 'ヵ', 'ヶ']),
        ] {
            let lesson = off_grid(&dataset, Script::Katakana)
                .into_iter()
                .find(|l| l.key == key)
                .unwrap_or_else(|| panic!("{key} is an off-grid lesson"));
            assert_eq!(lesson.kana, kana);
            let drawn = kana.iter().map(|ch| ch.to_string()).collect::<Vec<_>>().join(" ");
            assert!(lesson.title.contains(&drawn), "{} names the wrong kana", lesson.title);
        }
    }

    #[test]
    fn conversion_round_trips_for_the_shared_block() {
        for cp in 0x3041u32..=0x3096 {
            let hira = char::from_u32(cp).expect("valid");
            let kata = to_katakana(hira).expect("every hiragana has a katakana");
            assert_eq!(to_hiragana(kata), Some(hira));
        }
        assert_eq!(to_katakana('ー'), None);
        assert_eq!(to_katakana('一'), None);
        assert_eq!(to_hiragana('ヺ'), None);
    }

    #[test]
    fn the_course_covers_each_script_completely() {
        let dataset = dataset();
        for script in [Script::Hiragana, Script::Katakana] {
            let course = lessons(&dataset, script);
            let taught: usize = course.iter().map(Lesson::len).sum();
            assert_eq!(
                taught,
                dataset.of_script(script).count(),
                "{script:?} lessons must teach every kana of the script"
            );
            let mut all: Vec<char> = course.iter().flat_map(|l| l.kana.iter().copied()).collect();
            let total = all.len();
            all.sort_unstable();
            all.dedup();
            assert_eq!(total, all.len(), "{script:?} teaches a kana twice");
        }
    }

    #[test]
    fn the_plain_rows_come_before_the_voiced_ones() {
        let dataset = dataset();
        let course = lessons(&dataset, Script::Hiragana);
        let first_voiced = course.iter().position(|l| l.voiced).expect("there are voiced rows");
        assert!(
            course[..first_voiced].iter().all(|l| !l.voiced),
            "a voiced row appears among the plain ones"
        );
        assert_eq!(course[first_voiced].kana[0], 'が');
    }

    #[test]
    fn the_first_lesson_is_the_five_vowels_in_order() {
        let dataset = dataset();
        let course = lessons(&dataset, Script::Hiragana);
        assert_eq!(course[0].kana, vec!['あ', 'い', 'う', 'え', 'お']);
        assert_eq!(course[0].key, "hiragana-a");
        assert!(course[0].title.contains("あ い う え お"));

        let katakana = lessons(&dataset, Script::Katakana);
        assert_eq!(katakana[0].kana, vec!['ア', 'イ', 'ウ', 'エ', 'オ']);
        assert_eq!(katakana[0].key, "katakana-a");
    }

    #[test]
    fn the_voiced_lesson_order_follows_the_grid() {
        let dataset = dataset();
        let course = lessons(&dataset, Script::Hiragana);
        let voiced: Vec<&str> = course
            .iter()
            .filter(|l| l.voiced)
            .map(|l| l.key.as_str())
            .collect();
        assert_eq!(
            voiced,
            vec!["hiragana-ga", "hiragana-za", "hiragana-da", "hiragana-ba", "hiragana-pa"]
        );
    }

    #[test]
    fn the_rare_and_small_kana_get_their_own_lessons() {
        let dataset = dataset();
        let course = lessons(&dataset, Script::Hiragana);
        let keys: Vec<&str> = course.iter().map(|l| l.key.as_str()).collect();
        assert!(keys.contains(&"hiragana-small"));
        assert!(keys.contains(&"hiragana-rare"));

        let rare = course.iter().find(|l| l.key == "hiragana-rare").expect("present");
        assert_eq!(rare.kana, vec!['ゐ', 'ゑ', 'ゔ', 'ゕ', 'ゖ']);
    }

    /// Katakana has five characters hiragana does not, and they are exactly the
    /// ones a course assembled from hiragana would silently omit.
    #[test]
    fn the_katakana_only_characters_are_taught() {
        let dataset = dataset();
        let course = lessons(&dataset, Script::Katakana);

        let v = course.iter().find(|l| l.key == "katakana-v").expect("the v-series is taught");
        assert_eq!(v.kana, vec!['ヷ', 'ヸ', 'ヹ', 'ヺ']);

        let mark = course
            .iter()
            .find(|l| l.key == "katakana-choonpu")
            .expect("the prolonged sound mark is taught");
        assert_eq!(mark.kana, vec!['ー']);

        // And hiragana has no such lessons, because it has no such characters.
        let hiragana = lessons(&dataset, Script::Hiragana);
        assert!(!hiragana.iter().any(|l| l.key == "hiragana-v"));
        assert!(!hiragana.iter().any(|l| l.key == "hiragana-choonpu"));
    }

    #[test]
    fn yoon_are_three_per_base_and_no_more() {
        for script in [Script::Hiragana, Script::Katakana] {
            let all = yoon(script);
            assert_eq!(all.len(), 33, "{script:?} has eleven bases of three");
            let mut keys: Vec<&str> = all.iter().map(|y| y.key.as_str()).collect();
            let total = keys.len();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(total, keys.len(), "{script:?} repeats a yōon key");
        }
    }

    #[test]
    fn yoon_are_written_as_the_base_then_the_small_kana() {
        let all = yoon(Script::Hiragana);
        let kyu = all.iter().find(|y| y.hepburn == "kyu").expect("きゅ is generated");
        assert_eq!(kyu.display, "きゅ");
        assert_eq!(kyu.kana, vec!['き', 'ゅ']);
        assert_eq!(kyu.key, "hiragana-kyu");

        // The small kana really is the small one — きゆ would be two morae.
        assert_eq!(kyu.kana[1], 'ゅ');
        assert_ne!(kyu.kana[1], 'ゆ');

        let katakana = yoon(Script::Katakana);
        let kyu = katakana.iter().find(|y| y.hepburn == "kyu").expect("キュ is generated");
        assert_eq!(kyu.display, "キュ");
        assert_eq!(kyu.kana, vec!['キ', 'ュ']);
    }

    #[test]
    fn the_sounds_hepburn_and_kunrei_disagree_on_are_right_in_yoon() {
        let all = yoon(Script::Hiragana);
        let find = |hepburn: &str| -> (String, String) {
            let y = all
                .iter()
                .find(|y| y.hepburn == hepburn)
                .unwrap_or_else(|| panic!("{hepburn} is generated"));
            (y.hepburn.clone(), y.kunrei.clone())
        };
        assert_eq!(find("sha"), ("sha".into(), "sya".into()));
        assert_eq!(find("cha"), ("cha".into(), "tya".into()));
        assert_eq!(find("ja"), ("ja".into(), "zya".into()));
        // The rows where the two systems agree must not drift apart.
        assert_eq!(find("kya"), ("kya".into(), "kya".into()));
        assert_eq!(find("nyu"), ("nyu".into(), "nyu".into()));
        assert_eq!(find("ryo"), ("ryo".into(), "ryo".into()));
    }

    #[test]
    fn every_yoon_names_kana_the_app_can_draw() {
        let dataset = dataset();
        for script in [Script::Hiragana, Script::Katakana] {
            for y in yoon(script) {
                assert_eq!(y.kana.len(), 2);
                for ch in y.kana.iter().chain(&y.plain) {
                    assert!(
                        dataset.get(*ch).is_some(),
                        "{} names {ch}, which the app cannot draw",
                        y.key
                    );
                }
            }
        }
    }

    /// The two-mora counterpart is composed from the base's reading and the
    /// full-size kana's, so it has to come out as what the input engine produces
    /// for that spelling. Two independent paths to the same string: a composition
    /// that drifted would type something else.
    #[test]
    fn the_plain_counter_of_every_yoon_is_what_the_input_engine_types() {
        for script in [Script::Hiragana, Script::Katakana] {
            for y in yoon(script) {
                let plain: String = y.plain.iter().collect();
                assert_eq!(
                    crate::to_kana_in(script, &y.plain_hepburn).ok(),
                    Some(plain.clone()),
                    "{} is read {}, which types as something else",
                    plain,
                    y.plain_hepburn
                );
                assert_eq!(
                    crate::to_kana_in(script, &y.hepburn).ok(),
                    Some(y.display.clone()),
                    "{} is read {}",
                    y.display,
                    y.hepburn
                );
                // And the plain spelling is not the digraph: that difference is
                // the whole exercise.
                assert_ne!(plain, y.display);
            }
        }
    }

    #[test]
    fn the_plain_counter_is_full_size_kana_of_the_same_row() {
        let all = yoon(Script::Hiragana);
        let kya = all.iter().find(|y| y.hepburn == "kya").expect("きゃ is generated");
        assert_eq!(kya.plain, vec!['き', 'や']);
        assert_eq!(kya.plain_hepburn, "kiya");
        assert_eq!(kya.plain_kunrei, "kiya");

        // Kunrei differs over the consonant, not the vowel: しゃ is `sha`/`sya`
        // and its counter is `shiya`/`siya`.
        let sha = all.iter().find(|y| y.hepburn == "sha").expect("しゃ is generated");
        assert_eq!(sha.plain, vec!['し', 'や']);
        assert_eq!(sha.plain_hepburn, "shiya");
        assert_eq!(sha.plain_kunrei, "siya");

        let katakana = yoon(Script::Katakana);
        let kya = katakana.iter().find(|y| y.hepburn == "kya").expect("キャ is generated");
        assert_eq!(kya.display, "キャ");
        assert_eq!(kya.plain, vec!['キ', 'ヤ']);
        assert_eq!(kya.plain_hepburn, "kiya");
    }

    /// Every yōon has one counterpart and the counterparts are their own set, so
    /// the drill cannot ask the same question twice under two keys.
    #[test]
    fn every_yoon_has_exactly_one_plain_counterpart() {
        for script in [Script::Hiragana, Script::Katakana] {
            let all = yoon(script);
            let mut plains: Vec<String> = all.iter().map(|y| y.plain.iter().collect()).collect();
            let total = plains.len();
            plains.sort();
            plains.dedup();
            assert_eq!(total, plains.len(), "{script:?} repeats a two-mora spelling");
            for y in &all {
                assert_eq!(y.plain.len(), 2);
                assert_eq!(y.plain[0], y.kana[0], "{} changes its base", y.display);
            }
        }
    }

    #[test]
    fn every_confusable_pair_is_drawable() {
        let dataset = dataset();
        for pair in CONFUSABLE {
            for ch in [pair.a, pair.b] {
                assert!(
                    dataset.get(ch).is_some(),
                    "the pair {}/{} names {ch}, which the app cannot draw",
                    pair.a,
                    pair.b
                );
            }
            assert_ne!(pair.a, pair.b, "a kana cannot be confusable with itself");
            assert!(!pair.tell.trim().is_empty(), "{} / {} has no explanation", pair.a, pair.b);
        }
    }

    #[test]
    fn confusions_are_found_from_either_side() {
        let dataset = dataset();
        let from_sharp = confusions_for(&dataset, 'シ');
        assert!(from_sharp.iter().any(|p| p.b == 'ツ'));
        let from_tsu = confusions_for(&dataset, 'ツ');
        assert!(from_tsu.iter().any(|p| p.a == 'シ'));
        assert!(confusions_for(&dataset, 'あ').is_empty());
    }

    /// A kanji with the minimum the course reads: a character, a grade and a
    /// rank. The real ones come from the committed artifact.
    fn kanji(ch: char, grade: u8, frequency: Option<u16>) -> Kanji {
        Kanji {
            ch,
            grade,
            stroke_count: 1,
            frequency,
            radical: '一',
            radical_note: None,
            radical_number: 1,
            on: Vec::new(),
            kun: Vec::new(),
            meanings: Vec::new(),
            nanori: Vec::new(),
            decomposition: String::new(),
            outlines: vec!["M0,0".to_string()],
            medians: vec![vec![crate::Point::new(0.0, 0.0)]],
        }
    }

    #[test]
    fn the_kanji_course_is_the_grades_in_order_and_then_the_remainder() {
        let dataset = KanjiDataset::from_kanji(
            vec![
                kanji('鬱', 8, Some(2_000)),
                kanji('一', 1, Some(2)),
                kanji('学', 1, Some(63)),
                kanji('亜', 8, Some(1_500)),
            ],
            crate::KanjiSource::default(),
        );
        let lessons = kanji_lessons(&dataset, KANJI_LESSON_SIZE);
        assert_eq!(
            lessons.iter().map(|l| l.grade).collect::<Vec<_>>(),
            vec![1, 8],
            "kyōiku first, then the jōyō remainder"
        );
        assert_eq!(lessons[0].kanji, vec!['一', '学'], "and inside a grade, frequency");
        assert_eq!(lessons[1].kanji, vec!['亜', '鬱']);
        assert_eq!(lessons[0].key, "g1-1");
        assert_eq!(lessons[1].key, "g8-1");
    }

    #[test]
    fn a_lesson_holds_ten_and_the_last_holds_the_rest() {
        let characters: Vec<Kanji> = (0..25)
            .map(|i| {
                let ch = char::from_u32(0x4E00 + i).expect("valid");
                kanji(ch, 1, Some(i as u16 + 1))
            })
            .collect();
        let dataset = KanjiDataset::from_kanji(characters, crate::KanjiSource::default());
        let lessons = kanji_lessons(&dataset, KANJI_LESSON_SIZE);

        assert_eq!(lessons.len(), 3);
        assert_eq!(lessons[0].len(), 10);
        assert_eq!(lessons[1].len(), 10);
        assert_eq!(lessons[2].len(), 5);
        assert_eq!(lessons[0].title, "1\u{2013}10");
        assert_eq!(lessons[2].title, "21\u{2013}25");
        assert_eq!(lessons.iter().map(KanjiLesson::len).sum::<usize>(), 25);

        // And every lesson key is its own.
        let mut keys: Vec<&str> = lessons.iter().map(|l| l.key.as_str()).collect();
        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(total, keys.len(), "two lessons share a key");
    }

    #[test]
    fn an_unranked_kanji_sorts_after_the_ranked_ones() {
        let dataset = KanjiDataset::from_kanji(
            vec![
                kanji('乙', 1, None),
                kanji('甲', 1, Some(500)),
                kanji('一', 1, Some(2)),
            ],
            crate::KanjiSource::default(),
        );
        let lessons = kanji_lessons(&dataset, KANJI_LESSON_SIZE);
        // One lesson of three, in rank order with the unranked one last.
        assert_eq!(lessons.len(), 1);
        assert_eq!(lessons[0].kanji, vec!['一', '甲', '乙']);
    }

    #[test]
    fn the_kanji_course_teaches_every_character_exactly_once() {
        let dataset = KanjiDataset::from_kanji(
            (0..23)
                .map(|i| {
                    let ch = char::from_u32(0x4E00 + i).expect("valid");
                    kanji(ch, if i < 12 { 2 } else { 8 }, Some(i as u16))
                })
                .collect(),
            crate::KanjiSource::default(),
        );
        let lessons = kanji_lessons(&dataset, KANJI_LESSON_SIZE);
        let mut taught: Vec<char> = lessons.iter().flat_map(|l| l.kanji.iter().copied()).collect();
        assert_eq!(taught.len(), dataset.len());
        taught.sort_unstable();
        let total = taught.len();
        taught.dedup();
        assert_eq!(total, taught.len(), "a character is taught twice");
        for lesson in &lessons {
            for ch in &lesson.kanji {
                assert!(dataset.get(*ch).is_some(), "{ch} is not in the dataset");
            }
        }
    }

    #[test]
    fn the_kanji_grades_are_named_by_the_vocabularys_ladder() {
        assert_eq!(grade_name(1), "kyōiku 1");
        assert_eq!(grade_name(6), "kyōiku 6");
        assert_eq!(grade_name(8), "jōyō beyond the school grades");
        // The point of naming them here rather than duplicating the strings: a
        // kanji grade and a vocabulary band are the same rung.
        for band in 1..=6u8 {
            assert_eq!(grade_name(band), crate::band_name(band));
        }
        assert_eq!(grade_name(8), crate::band_name(crate::REMAINDER_BAND));
    }
}
