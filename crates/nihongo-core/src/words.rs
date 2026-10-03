//! The vocabulary dataset: words, with their own readings and the furigana that
//! puts each reading over the right characters.
//!
//! # Why a word and not a character
//!
//! 生 has a dozen readings that only resolve inside a word, and 食べる is 食 plus
//! okurigana. A course that teaches characters and then a reading per character
//! teaches the wrong thing, so the unit here is the word, and a character arrives
//! through the words that use it. That is also why [`Word::reading`] is the
//! **whole word's** reading, measured from JMdict, and never assembled from its
//! characters — assembling it is precisely what gets 大人 (おとな, not だいじん)
//! and 今日 (きょう) wrong.
//!
//! # The two derived fields, and where they come from
//!
//! * [`Word::band`] is **this project's** level ladder, not the JLPT's. There has
//!   been no official JLPT kanji or vocabulary list since 2010, and the best
//!   community list chains to a source that asserts no licence, so the ladder is
//!   derived instead and the UI says so. The rule: a word's band is the **highest
//!   kyōiku grade among its kanji** — 1 to 6 — and band 7 for a word containing a
//!   kanji from the jōyō remainder. A word is readable exactly when its kanji are
//!   known, which is what makes this a teaching order rather than a popularity
//!   contest.
//! * [`Word::nf`] is EDRDG's own frequency ranking, `nf01`–`nf48`: the word is in
//!   the `nf`th block of 500, smaller being more frequent. It orders the words
//!   *within* a band. It lives in EDRDG's `JMdict_e` XML and **not** in the JSON
//!   reformatting this pipeline otherwise reads, which is why the fetch takes both
//!   files.
//!
//! # Which words are in, and which are out
//!
//! In: a JMdict entry **EDRDG marks as common by either of its two signals** — an
//! `nf01`–`nf48` rank, or one of the `ichi1`/`ichi2` markers that name the words
//! of EDRDG's most-common-word corpus — that has a kanji form, and whose **every
//! kanji is in the committed kanji artifact**. The last clause is the important
//! one: it is checked against `data/kanji.bin.gz` rather than against a second
//! opinion, so a word this course offers is always one the board can draw and the
//! character course can teach. It is also why jinmeiyō and hyōgai words are out for
//! now — they become teachable the day those sets are added, not before.
//!
//! **Both signals, and the reason is measured.** Taking only the `nf` rank looks
//! sufficient until it drops 行く (to go) and 本 (book): both carry EDRDG's
//! `ichi1`, neither carries an `nf` rank — `nf05` belongs to a *different* entry
//! of 本, read もと. A beginner course that cannot say "go" or "book" is not one,
//! so membership is EDRDG's judgement and not one of its two fields. What is
//! deliberately *not* taken is `spec1`/`spec2` (specialist vocabulary) and `gai1`
//! (loanwords), which would add about 1,200 words of technical and foreign
//! terminology to a beginner's course.
//!
//! Out: kana-only entries (a kana-only word teaches no character, and the kana
//! course already teaches kana), and words whose kanji the artifact does not hold.
//!
//! # Furigana, and the one thing it is allowed to be missing
//!
//! [`Word::furigana`] comes from [JmdictFurigana](https://github.com/Doublevil/JmdictFurigana)
//! (MIT), joined on the word's own text and reading, and it satisfies two
//! properties that `tests/words_artifact.rs` asserts for the whole set: the
//! `ruby` parts concatenate to the text, and the readings (`rt`, falling back to
//! `ruby`) concatenate to the reading.
//!
//! Alignment is **not** derived here when it is missing. A small number of words
//! have no furigana entry — 66 of 21,902 — and they are carried with an empty
//! `furigana` and their reading intact: the reading is what grading uses, and
//! inventing an alignment would be the very thing this module refuses to do.

use serde::{Deserialize, Serialize};

/// Magic bytes at the head of a words artifact, checked before decoding.
///
/// `postcard` is not self-describing, so a stale or foreign artifact would
/// otherwise decode into nonsense. The trailing digits are the format version.
pub const WORDS_ARTIFACT_MAGIC: &[u8; 8] = b"WORDD002";

/// The bands this project's ladder has: grades 1–6, then the jōyō remainder.
pub const BANDS: [u8; 7] = [1, 2, 3, 4, 5, 6, 7];

/// The band for the jōyō-remainder kanji, which have no school grade.
pub const REMAINDER_BAND: u8 = 7;

/// The band a word's kanji put it in.
///
/// `grades` are the KANJIDIC2 grades of the word's kanji, where 8 means the jōyō
/// remainder. An empty list is a kana-only word, which this course does not carry,
/// and it is reported as band 1 rather than panicking: the caller has already
/// decided what to do about it.
pub fn band_for(grades: impl IntoIterator<Item = u8>) -> u8 {
    grades
        .into_iter()
        .map(|grade| if grade >= 8 { REMAINDER_BAND } else { grade })
        .max()
        .unwrap_or(1)
}

/// What one word's band means, for a screen that has to explain it.
pub fn band_name(band: u8) -> &'static str {
    match band {
        1 => "kyōiku 1",
        2 => "kyōiku 2",
        3 => "kyōiku 3",
        4 => "kyōiku 4",
        5 => "kyōiku 5",
        6 => "kyōiku 6",
        REMAINDER_BAND => "jōyō beyond the school grades",
        _ => "unknown band",
    }
}

/// One piece of a word's furigana: text, and the reading over it if it is kanji.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ruby {
    /// The text this segment covers — kanji, or kana that needs no reading.
    pub ruby: String,
    /// The reading, present only when `ruby` contains a kanji.
    pub rt: Option<String>,
}

impl Ruby {
    pub fn new(ruby: impl Into<String>, rt: Option<String>) -> Self {
        Self {
            ruby: ruby.into(),
            rt,
        }
    }

    /// The text this segment contributes to the reading.
    pub fn reading(&self) -> &str {
        self.rt.as_deref().unwrap_or(&self.ruby)
    }
}

/// One word: a unit of vocabulary, with the reading a learner types and the
/// furigana a screen draws.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    /// The word as it is written, e.g. `食べる`. May mix kanji and kana: the
    /// okurigana is part of the word.
    pub text: String,
    /// The reading of the **whole word**, e.g. `たべる`, taken from JMdict rather
    /// than composed from the characters.
    pub reading: String,
    /// The furigana, in order, covering [`Word::text`] and spelling
    /// [`Word::reading`]. Empty for the small number of words JmdictFurigana does
    /// not align.
    pub furigana: Vec<Ruby>,
    /// English glosses of the entry's first sense, joined with `"; "`.
    pub meaning: String,
    /// This project's level ladder: see the module docs. 1–6 are the kyōiku
    /// grades, 7 is the jōyō remainder.
    pub band: u8,
    /// EDRDG's `nf` ranking, 1–48: the word is in the `nf`th block of 500, and
    /// smaller is more frequent. It orders words within a band.
    ///
    /// `None` for a word EDRDG marks common by its `ichi1`/`ichi2` signal but does
    /// not rank — 行く and 本 are the two that made this an `Option` — and those
    /// words sort after the ranked ones in their band rather than being dropped.
    pub nf: Option<u8>,
}

impl Word {
    /// Every character of the word, in order.
    pub fn characters(&self) -> Vec<char> {
        self.text.chars().collect()
    }

    /// Just the kanji among its characters.
    pub fn kanji(&self) -> Vec<char> {
        self.text.chars().filter(|c| is_kanji(*c)).collect()
    }

    /// The reading its furigana spells out, for a caller that wants to check the
    /// two agree. Empty when there is no furigana.
    pub fn reading_from_furigana(&self) -> String {
        self.furigana.iter().map(Ruby::reading).collect()
    }

    /// The text its furigana covers. Empty when there is no furigana.
    pub fn text_from_furigana(&self) -> String {
        self.furigana.iter().map(|ruby| ruby.ruby.as_str()).collect()
    }

    /// Whether the furigana spells the word's reading.
    ///
    /// Compared after folding katakana to hiragana, and that is not laxness: two of
    /// the shipped words have a written form whose kana disagrees in *type* with the
    /// dictionary's reading — 生ゴミ is read なまごみ but written なま**ゴミ**, so the
    /// ruby for its kana part is ゴミ — and comparing byte-for-byte would call both
    /// the display and the reading wrong. Folding the type is the difference between
    /// "spelled differently" and "spelled the same way with a different kana".
    ///
    /// `false` for a word with no furigana: there is nothing to spell the reading.
    pub fn furigana_spells_reading(&self) -> bool {
        !self.furigana.is_empty()
            && crate::input::normalise_to_hiragana(&self.reading_from_furigana())
                == crate::input::normalise_to_hiragana(&self.reading)
    }

    pub fn has_furigana(&self) -> bool {
        !self.furigana.is_empty()
    }
}

/// Whether a character is a kanji, by block.
///
/// The ranges are the CJK Unified Ideographs, the extension A block and the
/// compatibility ideographs, which is what a Japanese word can contain. Kana,
/// the prolonged sound mark and the iteration marks are not kanji, and neither is
/// the middle dot in a word like リズム.
pub fn is_kanji(ch: char) -> bool {
    matches!(ch as u32,
        0x4E00..=0x9FFF        // CJK Unified Ideographs
        | 0x3400..=0x4DBF      // Extension A
        | 0xF900..=0xFAFF      // Compatibility Ideographs
        | 0x20000..=0x2FA1F    // Extensions B and beyond, listed as a range
    )
}

/// Which upstream snapshots the words in an artifact came from.
///
/// The nf ranking is EDRDG's `JMdict_e`, which is not versioned — the file is
/// rebuilt daily — so its own `JMdict created` comment is recorded instead, and
/// the counts the artifact test pins are what catch it moving.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordsSource {
    /// JMdict's version, from the JSON reformatting's own metadata.
    pub jmdict_version: String,
    /// The dictionary date that JSON was built from.
    pub jmdict_date: String,
    /// The `JMdict created` date of the XML the nf ranking was read from.
    pub nf_source_date: String,
    /// The JmdictFurigana release the alignments came from.
    pub furigana_release: String,
}

/// The decoded payload of the shipped words artifact.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WordsArtifact {
    pub words: Vec<Word>,
    pub source: WordsSource,
}

impl WordsArtifact {
    pub fn new(words: Vec<Word>, source: WordsSource) -> Self {
        Self { words, source }
    }
}

/// An indexed collection of words, ordered by band and then frequency.
#[derive(Clone, Debug, Default)]
pub struct WordDataset {
    words: Vec<Word>,
    source: WordsSource,
}

impl WordDataset {
    /// Order the words the way the course reads them: by band, then EDRDG's
    /// frequency, then the text. The last two keys are what make the order
    /// deterministic, so a rebuild is byte-identical.
    pub fn from_words(mut words: Vec<Word>, source: WordsSource) -> Self {
        words.sort_by(|a, b| {
            a.band
                .cmp(&b.band)
                // An unranked word sorts after every ranked one in its band.
                // `Option`'s own order puts `None` first, which is the opposite
                // of what "unranked" should mean here.
                .then(a.nf.is_none().cmp(&b.nf.is_none()))
                .then(a.nf.cmp(&b.nf))
                .then(a.text.cmp(&b.text))
                .then(a.reading.cmp(&b.reading))
        });
        Self { words, source }
    }

    /// Decode an artifact produced by `prepare-words`: gzip-compressed, magic
    /// prefixed, then a `postcard`-encoded [`WordsArtifact`].
    pub fn from_gzip_bytes(bytes: &[u8]) -> std::io::Result<Self> {
        use std::io::Read;

        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut raw = Vec::new();
        decoder.read_to_end(&mut raw)?;

        if raw.len() < WORDS_ARTIFACT_MAGIC.len()
            || &raw[..WORDS_ARTIFACT_MAGIC.len()] != WORDS_ARTIFACT_MAGIC
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "not a words artifact (bad or outdated magic); re-run `prepare-words`",
            ));
        }
        let artifact: WordsArtifact = postcard::from_bytes(&raw[WORDS_ARTIFACT_MAGIC.len()..])
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("corrupt words artifact: {e}"),
                )
            })?;
        Ok(Self::from_words(artifact.words, artifact.source))
    }

    pub fn words(&self) -> &[Word] {
        &self.words
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn source(&self) -> &WordsSource {
        &self.source
    }

    /// The first word written exactly `text`, if any.
    ///
    /// Homographs mean a text can carry more than one reading — six cases in the
    /// shipped set — so this is a convenience for a screen that already has the
    /// text, and [`WordDataset::find`] is the precise lookup.
    pub fn of_text(&self, text: &str) -> Option<&Word> {
        self.words.iter().find(|word| word.text == text)
    }

    /// The word written `text` and read `reading`.
    pub fn find(&self, text: &str, reading: &str) -> Option<&Word> {
        self.words
            .iter()
            .find(|word| word.text == text && word.reading == reading)
    }

    /// Every word of one band, in course order.
    pub fn of_band(&self, band: u8) -> impl Iterator<Item = &Word> {
        self.words.iter().filter(move |word| word.band == band)
    }

    /// How many words each band holds, in [`BANDS`] order.
    pub fn band_counts(&self) -> Vec<(u8, usize)> {
        BANDS
            .iter()
            .map(|&band| (band, self.of_band(band).count()))
            .collect()
    }

    /// Every word that uses `ch` among its kanji.
    pub fn of_kanji(&self, ch: char) -> impl Iterator<Item = &Word> {
        self.words.iter().filter(move |word| word.kanji().contains(&ch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str, reading: &str, band: u8, nf: u8) -> Word {
        Word {
            text: text.to_string(),
            reading: reading.to_string(),
            furigana: Vec::new(),
            meaning: format!("meaning of {text}"),
            band,
            nf: Some(nf),
        }
    }

    #[test]
    fn a_words_band_is_its_hardest_kanji() {
        assert_eq!(band_for([1, 2, 3]), 3);
        assert_eq!(band_for([6]), 6);
        // The jōyō remainder has no school grade and is the last band.
        assert_eq!(band_for([1, 8]), REMAINDER_BAND);
        assert_eq!(band_for([8]), REMAINDER_BAND);
        // A kana-only word has no grade to go on; the caller has decided about it.
        assert_eq!(band_for([]), 1);
    }

    #[test]
    fn band_names_say_what_they_mean() {
        assert_eq!(band_name(1), "kyōiku 1");
        assert_eq!(band_name(REMAINDER_BAND), "jōyō beyond the school grades");
        assert_eq!(band_name(9), "unknown band");
    }

    #[test]
    fn kanji_are_told_apart_from_kana() {
        assert!(is_kanji('食'));
        assert!(is_kanji('一'));
        assert!(!is_kanji('べ'));
        assert!(!is_kanji('ー'));
        assert!(!is_kanji('・'));
        let w = word("食べる", "たべる", 2, 5);
        assert_eq!(w.characters(), vec!['食', 'べ', 'る']);
        assert_eq!(w.kanji(), vec!['食']);
    }

    #[test]
    fn furigana_says_both_what_it_covers_and_how_it_reads() {
        let mut w = word("大人買い", "おとながい", 7, 5);
        w.furigana = vec![
            Ruby::new("大人", Some("おとな".to_string())),
            Ruby::new("買", Some("が".to_string())),
            Ruby::new("い", None),
        ];
        assert!(w.has_furigana());
        assert_eq!(w.text_from_furigana(), "大人買い");
        assert_eq!(w.reading_from_furigana(), "おとながい");
        assert_eq!(w.furigana[2].reading(), "い", "plain kana reads as itself");
        assert!(w.furigana_spells_reading());
    }

    /// The kana-type case, which is real in the shipped data: 生ゴミ is written with
    /// a katakana ゴミ and read なまごみ, so its ruby cannot be compared to its
    /// reading as bytes.
    #[test]
    fn furigana_over_kana_of_the_other_type_still_spells_the_reading() {
        let mut w = word("生ゴミ", "なまごみ", 1, 30);
        w.furigana = vec![Ruby::new("生", Some("なま".to_string())), Ruby::new("ゴミ", None)];
        assert_eq!(w.reading_from_furigana(), "なまゴミ");
        assert_ne!(w.reading_from_furigana(), w.reading);
        assert!(
            w.furigana_spells_reading(),
            "katakana ゴミ and hiragana ごみ are the same reading"
        );

        // And a word with no furigana has nothing to spell the reading with.
        let bare = word("山", "やま", 1, 40);
        assert!(!bare.furigana_spells_reading());
    }

    #[test]
    fn a_dataset_is_ordered_by_band_then_frequency() {
        let dataset = WordDataset::from_words(
            vec![
                word("後", "あと", 2, 40),
                word("一", "いち", 1, 30),
                word("人", "ひと", 1, 9),
            ],
            WordsSource::default(),
        );
        let order: Vec<&str> = dataset.words().iter().map(|w| w.text.as_str()).collect();
        assert_eq!(order, vec!["人", "一", "後"]);
        assert_eq!(dataset.band_counts(), vec![(1, 2), (2, 1), (3, 0), (4, 0), (5, 0), (6, 0), (7, 0)]);
    }

    #[test]
    fn a_word_is_found_by_its_text_and_its_reading() {
        let dataset = WordDataset::from_words(
            vec![word("生", "なま", 1, 20), word("生", "せい", 1, 21)],
            WordsSource::default(),
        );
        assert_eq!(dataset.of_text("生").map(|w| w.reading.as_str()), Some("なま"));
        assert_eq!(dataset.find("生", "せい").and_then(|w| w.nf), Some(21));
        assert!(dataset.find("生", "しょう").is_none());
    }

    /// EDRDG marks some words common without giving them an `nf` rank, and those
    /// words must not be confused with words it does not consider common at all —
    /// nor may `None` sort to the front, which is what `Option`'s own order would
    /// do. 行く and 本 are the two real examples.
    #[test]
    fn an_unranked_word_sorts_after_the_ranked_ones_in_its_band() {
        let unranked = Word {
            nf: None,
            ..word("行く", "いく", 1, 1)
        };
        let dataset = WordDataset::from_words(
            vec![unranked, word("本", "ほん", 1, 5)],
            WordsSource::default(),
        );
        let order: Vec<&str> = dataset.words().iter().map(|w| w.text.as_str()).collect();
        assert_eq!(order, vec!["本", "行く"]);
        assert!(dataset.words()[1].nf.is_none());
    }

    #[test]
    fn the_words_using_a_kanji_can_be_listed() {
        let dataset = WordDataset::from_words(
            vec![word("食べる", "たべる", 2, 5), word("飲む", "のむ", 3, 6)],
            WordsSource::default(),
        );
        let with_food: Vec<&str> = dataset.of_kanji('食').map(|w| w.text.as_str()).collect();
        assert_eq!(with_food, vec!["食べる"]);
        assert_eq!(dataset.of_kanji('飲').count(), 1);
        assert_eq!(dataset.of_kanji('水').count(), 0);
    }

    #[test]
    fn an_artifact_that_is_not_one_is_refused_by_its_magic() {
        // The kanji artifact is the realistic mistake: they live side by side.
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut encoder, b"KANJD001\x00\x00").unwrap();
        let bytes = encoder.finish().unwrap();
        let err = WordDataset::from_gzip_bytes(&bytes).unwrap_err();
        assert!(err.to_string().contains("not a words artifact"), "{err}");
    }

    #[test]
    fn the_bands_are_the_seven_the_ladder_has() {
        assert_eq!(BANDS, [1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(BANDS.iter().filter(|b| **b == REMAINDER_BAND).count(), 1);
    }
}
