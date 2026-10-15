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
/// So the rule follows from that: when the surface **is** the base form, take
/// `reading`; when it is inflected, take the surface's pronunciation. UniDic says
/// which is which through `orthographic_surface_form` and
/// `orthographic_base_form`, which this compares rather than inferring from the
/// part of speech.
///
/// UniDic writes `*` for a reading it does not have, and katakana for the ones it
/// does, so the result is folded to hiragana — the artifacts' readings and their
/// furigana are hiragana throughout, and a screen should not have to know which of
/// the two it is looking at.
fn analyser_reading(token: &mut Token<'_>) -> Option<String> {
    let surface_form = token.get("orthographic_surface_form").map(str::to_string);
    let base_form = token.get("orthographic_base_form").map(str::to_string);
    let inflected = matches!((&surface_form, &base_form), (Some(surface), Some(base)) if surface != base);
    let field = if inflected {
        "phonological_surface_form"
    } else {
        "reading"
    };

    token
        .get(field)
        .map(str::trim)
        .filter(|reading| !reading.is_empty() && *reading != "*")
        .map(normalise_to_hiragana)
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
    use super::{context_may_choose, is_numeral_surface};

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
}
