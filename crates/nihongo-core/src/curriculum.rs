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

/// One row of the gojūon grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// The kana the row is named for, in hiragana.
    pub head: char,
    /// The row's Hepburn label — `a`, `ka`, `sha`-less `sa`, and so on.
    pub sound: &'static str,
    /// The row's kana in gojūon order (a, i, u, e, o), in hiragana.
    pub hiragana: &'static [char],
    /// True for a row that is the voiced or semi-voiced form of another row.
    /// These are taught after the plain rows, not beside them.
    pub voiced: bool,
}

impl Row {
    /// The row's kana in the given script, in gojūon order.
    pub fn kana(&self, script: Script) -> impl Iterator<Item = char> + '_ {
        self.hiragana.iter().copied().filter_map(move |ch| match script {
            Script::Hiragana => Some(ch),
            Script::Katakana => to_katakana(ch),
        })
    }

    /// How many kana the row has.
    pub fn len(&self) -> usize {
        self.hiragana.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hiragana.is_empty()
    }

    /// A stable key for the row, e.g. `"ka"`, for use as a DOM key or a test id.
    pub fn key(&self) -> String {
        self.sound.to_string()
    }
}

/// The gojūon grid: eleven plain rows (46 kana) then the five voiced rows
/// (25 kana — 20 dakuten and 5 handakuten).
pub const ROWS: &[Row] = &[
    // The plain rows: the 46 kana of the gojūon proper.
    Row { head: 'あ', sound: "a", hiragana: &['あ', 'い', 'う', 'え', 'お'], voiced: false },
    Row { head: 'か', sound: "ka", hiragana: &['か', 'き', 'く', 'け', 'こ'], voiced: false },
    Row { head: 'さ', sound: "sa", hiragana: &['さ', 'し', 'す', 'せ', 'そ'], voiced: false },
    Row { head: 'た', sound: "ta", hiragana: &['た', 'ち', 'つ', 'て', 'と'], voiced: false },
    Row { head: 'な', sound: "na", hiragana: &['な', 'に', 'ぬ', 'ね', 'の'], voiced: false },
    Row { head: 'は', sound: "ha", hiragana: &['は', 'ひ', 'ふ', 'へ', 'ほ'], voiced: false },
    Row { head: 'ま', sound: "ma", hiragana: &['ま', 'み', 'む', 'め', 'も'], voiced: false },
    // や has three: the i and e positions were never filled.
    Row { head: 'や', sound: "ya", hiragana: &['や', 'ゆ', 'よ'], voiced: false },
    Row { head: 'ら', sound: "ra", hiragana: &['ら', 'り', 'る', 'れ', 'ろ'], voiced: false },
    // を is the particle; ゐ and ゑ are obsolete and are taught separately.
    Row { head: 'わ', sound: "wa", hiragana: &['わ', 'を'], voiced: false },
    Row { head: 'ん', sound: "n", hiragana: &['ん'], voiced: false },
    // The voiced rows.
    Row { head: 'が', sound: "ga", hiragana: &['が', 'ぎ', 'ぐ', 'げ', 'ご'], voiced: true },
    Row { head: 'ざ', sound: "za", hiragana: &['ざ', 'じ', 'ず', 'ぜ', 'ぞ'], voiced: true },
    Row { head: 'だ', sound: "da", hiragana: &['だ', 'ぢ', 'づ', 'で', 'ど'], voiced: true },
    Row { head: 'ば', sound: "ba", hiragana: &['ば', 'び', 'ぶ', 'べ', 'ぼ'], voiced: true },
    Row { head: 'ぱ', sound: "pa", hiragana: &['ぱ', 'ぴ', 'ぷ', 'ぺ', 'ぽ'], voiced: true },
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
    let mut out = Vec::new();
    let prefix = script.name();

    for row in ROWS {
        let kana: Vec<char> = row
            .kana(script)
            .filter(|ch| dataset.get(*ch).is_some())
            .collect();
        if kana.is_empty() {
            continue;
        }
        let drawn: String = kana
            .iter()
            .map(|&ch| ch.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        out.push(Lesson {
            key: format!("{prefix}-{}", row.sound),
            title: format!("{drawn} — {}", row.sound),
            kana,
            voiced: row.voiced,
        });
    }

    // A group whose members are written in hiragana and converted, for the two
    // sets that exist in both scripts.
    let group = |members: &[char], key: &str, title: &str| -> Option<Lesson> {
        let kana: Vec<char> = members
            .iter()
            .copied()
            .filter_map(|ch| match script {
                Script::Hiragana => Some(ch),
                Script::Katakana => to_katakana(ch),
            })
            .filter(|ch| dataset.get(*ch).is_some())
            .collect();
        if kana.is_empty() {
            return None;
        }
        Some(Lesson {
            key: format!("{prefix}-{key}"),
            title: title.to_string(),
            kana,
            voiced: false,
        })
    };

    if let Some(lesson) = group(SMALL_KANA, "small", "Small kana — ゃ ゅ ょ っ") {
        out.push(lesson);
    }
    if let Some(lesson) = group(RARE_KANA, "rare", "Rare kana — ゐ ゑ ゔ ゕ ゖ") {
        out.push(lesson);
    }

    // Katakana has five characters hiragana does not: the v-series, which exists
    // only for foreign words, and the prolonged sound mark, which is not a sound
    // at all but lengthens the vowel before it. Without these the katakana course
    // would teach 86 of its 91 kana and quietly omit ヷ ヸ ヹ ヺ ー.
    if script == Script::Katakana {
        let katakana_only = |members: &[char], key: &str, title: &str| -> Option<Lesson> {
            let kana: Vec<char> = members
                .iter()
                .copied()
                .filter(|ch| dataset.get(*ch).is_some())
                .collect();
            if kana.is_empty() {
                return None;
            }
            Some(Lesson {
                key: format!("{prefix}-{key}"),
                title: title.to_string(),
                kana,
                voiced: false,
            })
        };

        if let Some(lesson) = katakana_only(KATAKANA_ONLY, "v", "V-series — ヷ ヸ ヹ ヺ") {
            out.push(lesson);
        }
        if let Some(lesson) = katakana_only(&[CHOONPU], "choonpu", "Prolonged sound mark — ー") {
            out.push(lesson);
        }
    }

    out
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

/// The three small kana that form a yōon, with the vowel each contributes in
/// Hepburn and in Kunrei (the same, except that Kunrei keeps the vowel letter).
const YOON_SMALL: &[(char, &str, &str)] = &[('ゃ', "a", "a"), ('ゅ', "u", "u"), ('ょ', "o", "o")];

/// Every yōon for one script: eleven bases × three small kana = 33 digraphs.
pub fn yoon(script: Script) -> Vec<Yoon> {
    let mut out = Vec::new();
    for &(base, hepburn_stem, kunrei_stem) in YOON_BASES {
        for &(small, hepburn_vowel, kunrei_vowel) in YOON_SMALL {
            let (Some(base), Some(small)) = (
                match script {
                    Script::Hiragana => Some(base),
                    Script::Katakana => to_katakana(base),
                },
                match script {
                    Script::Hiragana => Some(small),
                    Script::Katakana => to_katakana(small),
                },
            ) else {
                continue;
            };
            let hepburn = format!("{hepburn_stem}{hepburn_vowel}");
            let kunrei = format!("{kunrei_stem}{kunrei_vowel}");
            out.push(Yoon {
                key: format!("{}-{}", script.name(), hepburn),
                display: format!("{base}{small}"),
                hepburn,
                kunrei,
                kana: vec![base, small],
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
            .flat_map(|r| r.hiragana.iter().copied())
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
                for ch in &y.kana {
                    assert!(
                        dataset.get(*ch).is_some(),
                        "{} names {ch}, which the app cannot draw",
                        y.key
                    );
                }
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
