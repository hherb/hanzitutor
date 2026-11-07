//! Turning a line of Japanese into tokens — the analyser half that both pipelines
//! share.
//!
//! Japanese has no spaces, so a text has to be *segmented* before a learner can tap
//! a word, and the reading over a word comes from knowing what the word is rather
//! than from its characters. Both the passages and the phrases need exactly that,
//! and they need the *same* answer: a word in a phrase and the same word in a
//! passage must not come out with two readings because two binaries each wrote
//! their own version of the rule.
//!
//! So the three decisions live here and only here:
//!
//! * **Which vocabulary word a token is** ([`word_for`]): the analyser's *lexeme*,
//!   so 食べた links to 食べる, falling back to the surface when the analyser names
//!   no lexeme it is written as.
//! * **What reading to draw over it** ([`reading_for`]): UniDic publishes two
//!   candidates and neither is right on its own — the full reasoning is on the
//!   function, and it is the trap that made the reading 食べ(たべる)ます once.
//! * **That whitespace is not a token** ([`analyse`]): a line's spaces are
//!   layout, not words, and neither artifact carries them.
//!
//! Gated behind the `tokenize` feature with the `lindera` dependency it uses: the
//! analyser runs at build time, and the app ships no tokeniser at all.

use std::borrow::Cow;

use lindera::segmenter::Segmenter;
use lindera::token::Token;

use crate::input::normalise_to_hiragana;
use crate::words::{is_kanji, Word, WordDataset};

/// One token of a text, as the analyser segmented it and the vocabulary read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segmented {
    /// The text as written.
    pub surface: String,
    /// The reading to draw over it, in hiragana. `None` for a token whose surface
    /// has no kanji, which needs no ruby.
    pub rt: Option<String>,
    /// The vocabulary word this token is, by its dictionary form, when the course
    /// teaches it.
    pub word: Option<String>,
}

impl Segmented {
    pub fn has_kanji(&self) -> bool {
        self.surface.chars().any(is_kanji)
    }
}

/// Segment `text` into words, each read and each linked to the vocabulary.
///
/// A token whose surface is only whitespace is dropped: it is the space between
/// two words, not a word. The caller decides what to do about a kanji-bearing token
/// that the vocabulary does not hold — `prepare-passages` refuses to write the
/// artifact, and `prepare-phrases` drops the sentence — which is why this reports
/// it rather than judging it.
pub fn analyse(
    segmenter: &Segmenter,
    text: &str,
    words: &WordDataset,
) -> Result<Vec<Segmented>, String> {
    let mut analysed = segmenter
        .segment(Cow::Borrowed(text))
        .map_err(|e| format!("could not segment {text:?}: {e}"))?;

    // The tokens that are words. A whitespace-only token is the space between two
    // of them, and it cannot be the number a counter follows, so the spaces are
    // dropped *before* the readings are worked out: the decision below is about
    // the token immediately before this one.
    let kept: Vec<usize> = (0..analysed.len())
        .filter(|index| !analysed[*index].surface.trim().is_empty())
        .collect();

    let mut out: Vec<Segmented> = Vec::with_capacity(kept.len());
    for (position, &index) in kept.iter().enumerate() {
        let surface = analysed[index].surface.to_string();
        // Whether a number is counting this token. It decides both which entry the
        // token is (see [`word_for`]) and whether the card may read it at all (see
        // [`reading_for`]), and it is read from the *previous* token, so it is
        // known before this token's reading is.
        let counted = position > 0 && is_numeral_surface(&analysed[kept[position - 1]].surface);
        // What the analyser read this token as in *this* sentence, which is what
        // tells one homograph from another.
        let read = analyser_reading(&mut analysed[index]);
        let word = word_for(
            &mut analysed[index],
            &surface,
            words,
            read.as_deref(),
            counted,
        );
        let has_kanji = surface.chars().any(is_kanji);
        let rt = if has_kanji {
            reading_for(word, &surface, counted, read)
        } else {
            None
        };
        out.push(Segmented {
            surface,
            rt,
            word: word.map(|entry| entry.text.clone()),
        });
    }
    Ok(out)
}

/// Whether a token is a **number**, which is what a counter attaches to.
///
/// The corpus writes its numbers in Arabic digits where the sentence does (`10月`,
/// `2時`), in full-width digits where it does (`１５分`), and in kanji numerals
/// where it does (`三度`, `四月`) — and the interrogative 何 is the same
/// construction with a question in it (`何時` is なんじ). The analyser writes each
/// Arabic digit as its own token, so `10` arrives as two of them and `0` is what
/// the 月 in `10月` follows.
///
/// A token that mixes a numeral with anything else is not one of these; neither is
/// the empty surface, which is what an empty line or a stray space yields.
fn is_numeral_surface(surface: &str) -> bool {
    !surface.is_empty()
        && surface.chars().all(|ch| {
            ch.is_ascii_digit()
                || ('０'..='９').contains(&ch)
                || "〇零一二三四五六七八九十百千万億兆何".contains(ch)
        })
}

/// The vocabulary word a token is, if it is one.
///
/// Two questions, in this order.
///
/// **Which written form it is** — the analyser's *lexeme*, the dictionary form: a
/// passage that says 食べた links to 食べる and 行きます to 行く, because those are
/// the entries the course teaches. Matching the surface first would work for nouns
/// and nouns only.
///
/// **Which of that form's readings it is** — a written form can be taught more than
/// one way, and 199 of them are in the shipped vocabulary. 人 is three words the
/// course teaches (ひと, にん, じん), and the card behind a tapped word has to be the
/// one the sentence used: `WordDataset::find` picks the entry whose reading the
/// analyser gave, where `of_text` answers with whichever entry the artifact lists
/// first — which is how 人は人です came to open the card for 人(じん).
///
/// When the analyser's reading is not one the course teaches for that form, the
/// first entry is what the token links to, and [`reading_for`] then keeps that
/// entry's reading — which is how 私 stays わたし although the analyser reads it
/// わたくし. The one measured exception is [`context_may_choose`]'s.
fn word_for<'a>(
    token: &mut Token<'_>,
    surface: &str,
    words: &'a WordDataset,
    read: Option<&str>,
    counted: bool,
) -> Option<&'a Word> {
    let lexeme = token.get("lexeme").map(str::to_string);
    for candidate in [lexeme.as_deref(), Some(surface)].into_iter().flatten() {
        if let Some(reading) = read {
            if context_may_choose(candidate, reading, counted) {
                if let Some(word) = words.find(candidate, reading) {
                    return Some(word);
                }
            }
        }
        if let Some(word) = words.of_text(candidate) {
            return Some(word);
        }
    }
    None
}

/// Whether the analyser's reading of a token may choose among a text's entries.
///
/// Four exceptions, and every one of them is a shipped sentence rather than a
/// preference. The vocabulary change that made this matter is `prepare-words`'s:
/// emitting every reading EDRDG marks common put 四(よん), 寒気(かんき) and
/// 市場(しじょう) beside the readings the sentences need, and the analyser picks the
/// one the sentence does not mean in three of them. The exception is keyed on the
/// text *and* the reading, so it costs that one entry and leaves every other
/// homograph to the sentence.
///
/// Everything else the analyser gets wrong it gets wrong by giving a reading the
/// course does not teach at all — 仏 as ふつ, 体 as たい, 中 as ちゅう before the
/// vocabulary held it — and the fallback to the first entry covers those without a
/// list of them here.
fn context_may_choose(text: &str, reading: &str, counted: bool) -> bool {
    // 人 is にん when a number counts it — 三人 is さんにん — and じん after a
    // proper noun: the corpus's 日本人ですか。 is にほんじん, and UniDic segments it
    // にっぽん + にん. Without this the card and the ruby would agree with each
    // other and with nothing else.
    if text == "人" && reading == "にん" {
        return counted;
    }

    !matches!(
        (text, reading),
        // 四月一日です。 is しがつついたち: よんがつ is not a word.
        ("四", "よん")
        // 寒気がする。 is さむけがする, a chill. かんき is a mass of cold air.
            | ("寒気", "かんき")
        // 市場に行く。 is いちばに行く, to the market place. しじょう is the
        // market as an abstraction, which is not somewhere one goes.
            | ("市場", "しじょう")
    )
}

/// The reading the analyser gives a token in the sentence it came from, in
/// hiragana, or `None` when it has none.
///
/// The fields are read here and the decision is [`reading_from_fields`]'s, which
/// is where the reasoning is and what the tests reach without a dictionary.
fn analyser_reading(token: &mut Token<'_>) -> Option<String> {
    let surface = token.surface.to_string();
    let orthographic_surface = token.get("orthographic_surface_form").map(str::to_string);
    let orthographic_base = token.get("orthographic_base_form").map(str::to_string);
    let reading = token.get("reading").map(str::to_string);
    let pronunciation = token.get("phonological_surface_form").map(str::to_string);
    reading_from_fields(
        &surface,
        orthographic_surface.as_deref(),
        orthographic_base.as_deref(),
        reading.as_deref(),
        pronunciation.as_deref(),
    )
}

/// The reading of a token, in hiragana, from the UniDic fields that describe it.
///
/// **Which field to read is not obvious, and getting it wrong renders the wrong
/// ruby.** UniDic publishes two candidates and neither is right on its own:
///
/// * `reading` is the reading of the **base form** for an inflected word — 行き
///   gives イク and 食べ gives タベル — which would draw 行き(いく) and
///   食べ(たべる)ます. For a word that is not inflected it is the surface's own
///   reading, and written the conventional way: 今日 gives キョウ and the particle
///   は gives ハ.
/// * `phonological_surface_form` is the surface's, but in *pronunciation*
///   notation: 行き gives イキ and 食べ gives タベ, which is right, while 今日 gives
///   キョー and は gives ワ, which is not — は is written は and read は however it
///   is pronounced.
///
/// So when the surface **is** the base form, `reading` is the answer. UniDic says
/// which is which through `orthographic_surface_form` and `orthographic_base_form`,
/// which this compares rather than inferring from the part of speech.
///
/// **When it is inflected, the reading is spelled out of the base form's.** The
/// pronunciation was the answer here once, and it drew 歌おう as うたおー, 大きく as
/// おーきく and 聞い as きー: pronunciation notation writes every long vowel as ー,
/// and whether that ー is spelled う or お is exactly what it does not say. Fuller
/// UniDic releases carry the surface in kana (仮名形出現形), but
/// `unidic-mecab-2.1.2` — the dictionary `lindera-unidic` builds — has no such
/// field, so the spelling is put together from what it does have. The base form
/// and the surface share a stem, 歌 in 歌う and 歌おう; the base form's reading ends
/// in the base form's own kana tail, ウタ + ウ; and swapping that tail for the
/// surface's gives うた + おう. See [`spelled_inflection`].
///
/// That spelling is then **checked against the pronunciation**, sound for sound,
/// with ー allowed to stand for a vowel (see [`spells`]). The check is what keeps a
/// verb whose kanji changes its reading as it inflects from being spelled wrong:
/// 来た is きた, and the stem of くる spells く, which the pronunciation キ refuses.
/// Where the check fails, or the shapes do not line up at all, the pronunciation is
/// the reading, as it was before.
///
/// UniDic writes `*` for a reading it does not have, and katakana for the ones it
/// does, so the result is folded to hiragana — the artifacts' readings and their
/// furigana are hiragana throughout, and a screen should not have to know which of
/// the two it is looking at.
pub(crate) fn reading_from_fields(
    surface: &str,
    orthographic_surface: Option<&str>,
    orthographic_base: Option<&str>,
    reading: Option<&str>,
    pronunciation: Option<&str>,
) -> Option<String> {
    let inflected = matches!(
        (orthographic_surface, orthographic_base),
        (Some(surface), Some(base)) if surface != base
    );
    if !inflected {
        return known(reading).map(normalise_to_hiragana);
    }

    let pronounced = known(pronunciation).map(normalise_to_hiragana)?;
    let spelled = orthographic_base
        .zip(known(reading))
        .and_then(|(base, reading)| {
            spelled_inflection(surface, base, &normalise_to_hiragana(reading))
        })
        .filter(|spelled| spells(spelled, &pronounced))
        .or_else(|| spelled_from_pronunciation(surface, &pronounced));
    Some(spelled.unwrap_or(pronounced))
}

/// The second way to spell an inflected `surface`, for when its base form's
/// reading cannot: the stem as the pronunciation says it, and the okurigana as the
/// surface writes it.
///
/// A verb whose kanji changes its reading as it inflects has no stem in its base
/// form's reading to borrow — 来よう is こよう, and the stem of くる spells く — so
/// the pronunciation was all that was left, and it says こよー. But the long vowel
/// it marks is in the okurigana, and the okurigana is written out in the surface:
/// よう. So the pronunciation's kana for the stem, こ, and the surface's own tail
/// make こよう. This is only tried where the pronunciation has a ー to put right,
/// only where that ー falls in the tail — a ー in the stem's pronunciation is a
/// spelling this cannot know — and only where the result still [`spells`] the
/// pronunciation.
fn spelled_from_pronunciation(surface: &str, pronounced: &str) -> Option<String> {
    if !pronounced.contains('ー') {
        return None;
    }
    let tail: String = {
        let mut kana: Vec<char> = surface
            .chars()
            .rev()
            .take_while(|&ch| all_kana(ch.encode_utf8(&mut [0; 4])))
            .collect();
        kana.reverse();
        kana.into_iter().collect()
    };
    let (tail_len, said_len) = (tail.chars().count(), pronounced.chars().count());
    if tail_len == 0 || tail_len == surface.chars().count() || said_len <= tail_len {
        return None;
    }
    let stem: String = pronounced.chars().take(said_len - tail_len).collect();
    if stem.contains('ー') {
        return None;
    }
    let spelled = format!("{stem}{}", normalise_to_hiragana(&tail));
    spells(&spelled, pronounced).then_some(spelled)
}

/// The reading of an inflected `surface`, spelled from its base form and the base
/// form's reading in hiragana, or `None` when the two do not line up.
///
/// The stem is the longest start the surface and the base form share, and what
/// follows it on either side has to be kana — the okurigana that inflects. The base
/// form's reading has to end in the base form's tail, and the stem's reading is
/// what is left when it is taken off: 食べる is たべる, its tail after 食べ is る,
/// so 食べ reads たべ and 食べよう reads たべ + よう.
fn spelled_inflection(surface: &str, base: &str, base_reading: &str) -> Option<String> {
    let stem: usize = surface
        .chars()
        .zip(base.chars())
        .take_while(|(a, b)| a == b)
        .map(|(ch, _)| ch.len_utf8())
        .sum();
    let (surface_tail, base_tail) = (&surface[stem..], &base[stem..]);
    if stem == 0 || !all_kana(surface_tail) || !all_kana(base_tail) {
        return None;
    }
    let stem_reading = base_reading.strip_suffix(normalise_to_hiragana(base_tail).as_str())?;
    if stem_reading.is_empty() {
        return None;
    }
    Some(format!(
        "{stem_reading}{}",
        normalise_to_hiragana(surface_tail)
    ))
}

/// Whether a text is kana throughout, as okurigana is. The empty tail of a surface
/// that is all stem — 小さ in 小さすぎる — counts.
fn all_kana(text: &str) -> bool {
    text.chars()
        .all(|ch| matches!(ch as u32, 0x3041..=0x309F | 0x30A1..=0x30FA | 0x30FC))
}

/// Whether `spelled` is a spelling of what `pronounced` says, both in hiragana.
///
/// Kana for kana, with the three differences between spelling and pronunciation
/// that can occur inside a word: ー stands for the vowel it lengthens, so おー is
/// おう or おお; づ is pronounced ず and ぢ is pronounced じ. Anything else — a
/// different consonant, a different length — is a different word, and the spelling
/// is refused.
fn spells(spelled: &str, pronounced: &str) -> bool {
    spelled.chars().count() == pronounced.chars().count()
        && spelled
            .chars()
            .zip(pronounced.chars())
            .all(|(written, said)| {
                written == said
                    || (said == 'ー' && "あいうえお".contains(written))
                    || (written, said) == ('づ', 'ず')
                    || (written, said) == ('ぢ', 'じ')
            })
}

/// A field's value, or `None` for the `*` UniDic writes where it has none.
fn known(field: Option<&str>) -> Option<&str> {
    field
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "*")
}

/// The reading to draw over a token, in hiragana.
///
/// Three readings are in play, and the rule is which one wins.
///
/// **The card's, by default**, so that a text and the word card behind it cannot
/// disagree: UniDic reads 私 as わたくし and the course says わたし, both are correct
/// Japanese, and one of them has to win here. [`word_for`] has already linked the
/// token to the entry the analyser's reading named where the course teaches that
/// reading — 人 in 人は人です is 人(ひと) — so in those cases the card's reading *is*
/// the analyser's and the two cannot disagree.
///
/// **And the analyser's, in a counter's position.** 月 is つき as a word and がつ in
/// 10月, 時 is とき and じ in 2時, 人 is じん and にん in 三人: the card's reading is
/// the word standing on its own, and after a number it is not what the sentence
/// says. The number decides that reading and the analyser read the number, so where
/// `counted` is set the analyser wins regardless of what the card says. Standing
/// alone the word keeps the card's reading — 月は昇った is still つき — which is the
/// whole point of the exception being this narrow.
///
/// When the analyser has no reading at all and the token is a vocabulary word, that
/// word's own reading is exactly right, so the fallback is a fact rather than a
/// guess.
fn reading_for(
    word: Option<&Word>,
    surface: &str,
    counted: bool,
    read: Option<String>,
) -> Option<String> {
    // The course's own reading of this word, when the token is that word written
    // the way the course writes it.
    let course = word.filter(|word| word.text == surface);

    if let Some(course) = course {
        if !counted {
            return Some(course.reading.clone());
        }
    }

    read.or_else(|| course.map(|word| word.reading.clone()))
}

#[cfg(test)]
mod tests {
    use super::{context_may_choose, is_numeral_surface, reading_from_fields};

    /// What a counter may follow, and what it may not.
    ///
    /// The cases are the three ways the corpus writes a number — Arabic digits,
    /// full-width digits, kanji numerals — plus 何, and then the shapes that only
    /// *look* like one: a number with something else attached (which is a word the
    /// analyser wrote as one token), a kanji numeral that is part of a word, and
    /// the empty surface.
    #[test]
    fn a_number_is_what_a_counter_follows() {
        for numeral in [
            "1", "0", "10", "１", "１５", "三", "四", "十", "百", "千", "万", "億", "何", "〇", "零",
        ] {
            assert!(is_numeral_surface(numeral), "{numeral:?} is a number");
        }
        for other in ["", " ", "月", "1月", "第", "一昨日", "二つ", "、", "だ"] {
            assert!(!is_numeral_surface(other), "{other:?} is not a number");
        }
    }

    /// The one place the analyser may not choose the entry — and the places it may.
    ///
    /// 人 is read にん by a number (三人, 何人) and じん after a proper noun
    /// (日本人), and UniDic reads the proper-noun sentence にっぽん + にん. The
    /// other three are readings the course teaches and the sentences do not mean.
    /// Every exception is keyed on both the text and the reading, so every other
    /// homograph is still chosen by the sentence.
    #[test]
    fn a_number_may_choose_the_counter_and_a_proper_noun_may_not() {
        assert!(!context_may_choose("人", "にん", false), "日本人 is にほんじん");
        assert!(context_may_choose("人", "にん", true), "三人 is さんにん");
        assert!(context_may_choose("人", "ひと", false), "人は人です is ひと");
        assert!(context_may_choose("生", "せい", false), "一年生 is せい");
        assert!(context_may_choose("中", "ちゅう", false), "仕事中 is ちゅう");
        assert!(context_may_choose("何", "なん", false), "何人ですか is なんにん");
        assert!(context_may_choose("空", "そら", false), "空が晴れた is そら");

        assert!(!context_may_choose("四", "よん", false), "四月一日です is しがつ");
        assert!(!context_may_choose("寒気", "かんき", false), "寒気がする is さむけ");
        assert!(!context_may_choose("市場", "しじょう", false), "市場に行く is いちば");
    }

    /// The reading of an inflected token in UniDic's own words: its surface, then
    /// the fields `orthographic_surface_form`, `orthographic_base_form`, `reading`
    /// and `phonological_surface_form`, copied from the rows of
    /// `unidic-mecab-2.1.2/lex.csv` the shipped phrases segmented to.
    fn inflected(surface: &str, base: &str, reading: &str, pronunciation: &str) -> Option<String> {
        reading_from_fields(
            surface,
            Some(surface),
            Some(base),
            Some(reading),
            Some(pronunciation),
        )
    }

    /// **A reading is spelled, not pronounced.** These are the shapes the shipped
    /// phrases drew with ー — 歌おう as うたおー, 大きく as おーきく — because the
    /// pronunciation field writes every long vowel as ー, and おう and おお are two
    /// spellings of the same sound. The base form's reading is what knows which.
    #[test]
    fn an_inflected_reading_spells_its_long_vowels() {
        // The volitional: おう and よう, pronounced おー and よー.
        assert_eq!(
            inflected("歌おう", "歌う", "ウタウ", "ウタオー").as_deref(),
            Some("うたおう")
        );
        assert_eq!(
            inflected("行こう", "行く", "イク", "イコー").as_deref(),
            Some("いこう")
        );
        assert_eq!(
            inflected("食べよう", "食べる", "タベル", "タベヨー").as_deref(),
            Some("たべよう")
        );
        // The い-onbin, where き + い is pronounced きー.
        assert_eq!(
            inflected("聞い", "聞く", "キク", "キー").as_deref(),
            Some("きい")
        );
        assert_eq!(
            inflected("言い", "言う", "イウ", "イー").as_deref(),
            Some("いい")
        );
        // And the long vowel inside the stem, which is spelled おお in one word and
        // おう in another: no rule on the sound alone can tell 大き from 放っ.
        assert_eq!(
            inflected("大きく", "大きい", "オオキイ", "オーキク").as_deref(),
            Some("おおきく")
        );
        assert_eq!(
            inflected("放っ", "放る", "ホウル", "ホーッ").as_deref(),
            Some("ほうっ")
        );
        assert_eq!(
            inflected("小さ", "小さい", "チイサイ", "チーサ").as_deref(),
            Some("ちいさ")
        );
        assert_eq!(
            inflected("遠", "遠い", "トオイ", "トー").as_deref(),
            Some("とお")
        );
    }

    /// The other difference between the two notations that occurs inside a word:
    /// づ is pronounced ず, so the pronunciation field drew 続け as つずけ.
    #[test]
    fn an_inflected_reading_spells_its_zu_the_way_the_word_does() {
        assert_eq!(
            inflected("続け", "続ける", "ツヅケル", "ツズケ").as_deref(),
            Some("つづけ")
        );
    }

    /// An inflected reading with no long vowel in it is what it always was, and a
    /// verb whose kanji changes its reading as it inflects is read the way the
    /// sentence says it. 来る is the one: 来た is きた, which no stem of くる spells,
    /// so the pronunciation is the answer — and it has no ー to be wrong about.
    #[test]
    fn an_inflected_reading_the_base_form_cannot_spell_is_the_pronounced_one() {
        assert_eq!(
            inflected("行き", "行く", "イク", "イキ").as_deref(),
            Some("いき")
        );
        assert_eq!(
            inflected("食べ", "食べる", "タベル", "タベ").as_deref(),
            Some("たべ")
        );
        assert_eq!(inflected("来", "来る", "クル", "キ").as_deref(), Some("き"));
        assert_eq!(inflected("来", "来る", "クル", "コ").as_deref(), Some("こ"));
    }

    /// Where the base form's reading cannot spell the stem, the okurigana still can:
    /// it is written out in the surface. 来よう is こよう, not the こよー the
    /// pronunciation says, and 行こう read as ゆこう is ゆこう. A long vowel in the
    /// stem's own pronunciation is a spelling the surface does not show, so that one
    /// stays as it is pronounced.
    #[test]
    fn an_inflected_reading_the_base_form_cannot_spell_takes_its_okurigana_from_the_surface() {
        assert_eq!(
            inflected("来よう", "来る", "クル", "コヨー").as_deref(),
            Some("こよう")
        );
        assert_eq!(
            inflected("行こう", "行く", "イク", "ユコー").as_deref(),
            Some("ゆこう")
        );
        assert_eq!(
            inflected("来", "来る", "クル", "コー").as_deref(),
            Some("こー"),
            "a surface with no okurigana has nothing to spell the ー with"
        );
        assert_eq!(
            inflected("通っ", "通じる", "ツウジル", "トーッ").as_deref(),
            Some("とーっ"),
            "a ー in the stem's pronunciation is not the surface's to spell"
        );
    }

    /// A token that is its own base form takes `reading`, which is the conventional
    /// spelling: 今日 is きょう although it is pronounced きょー, and は is は.
    #[test]
    fn an_uninflected_reading_is_the_dictionary_one() {
        let reading = |surface: &str, reading: &str, pronunciation: &str| {
            reading_from_fields(
                surface,
                Some(surface),
                Some(surface),
                Some(reading),
                Some(pronunciation),
            )
        };
        assert_eq!(
            reading("今日", "キョウ", "キョー").as_deref(),
            Some("きょう")
        );
        assert_eq!(reading("は", "ハ", "ワ").as_deref(), Some("は"));
        assert_eq!(
            reading("珈琲", "コーヒー", "コーヒー").as_deref(),
            Some("こーひー")
        );
        assert_eq!(reading("何", "*", "*"), None);
    }
}
