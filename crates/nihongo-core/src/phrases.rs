//! The graded phrases: short sentences from a corpus, segmented into words,
//! levelled on this course's own ladder, and attributed to whoever wrote them.
//!
//! # What a phrase is, and how it differs from a passage
//!
//! A [`crate::passages::Passage`] is a text written for this course and read for
//! its own sake: several lines, a title, and a gloss of the whole thing. A phrase
//! is **one sentence from a corpus**, imported rather than written, with the
//! translation that corpus publishes beside it and the contributor the licence
//! requires to be named. It exists so that the course has something to *read at
//! level* — the graded half — while the passages stay the hand-written long form.
//!
//! # Why the corpus is filtered rather than trusted
//!
//! Tatoeba is per-sentence licensed. Its Japanese sentences are contributed under
//! **CC BY 2.0 FR** by default and **CC0 1.0** by a contributor who chose it, and a
//! sentence whose licence does not permit redistribution or adaptation is of no use
//! here whatever it says. `prepare-phrases` therefore reads the licence column and
//! refuses anything that is not one of those two; the corpus's own difficulty
//! signal is that it has none, which is why the level is computed here rather than
//! read off.
//!
//! # The level is this project's, and it is derived
//!
//! There has been no official JLPT vocabulary or grammar list since 2010, so there
//! is nothing to import a level *from*. A phrase's band is therefore the band of
//! its **hardest word**, on exactly the ladder [`crate::words::band_for`] gives a
//! word from the school grades of its kanji — the same rule that decides which band
//! a word is listed in, applied to a sentence. A sentence of nothing but kana has
//! no word to be hard and sits in band 1.
//!
//! # The invariant a phrase is held to
//!
//! A kanji-bearing token must be a word the course teaches, or the phrase is not
//! written into the artifact at all. That is [`crate::passages::Passage`]'s rule
//! (see `HANDOVER_NIHONGO.md` invariant 22) and it matters more here, not less: an
//! imported sentence cannot be edited to fit the vocabulary — it can only be kept
//! or dropped — so the filter *is* the promise, and `prepare-phrases` reports the
//! funnel that produced what it kept.

use serde::{Deserialize, Serialize};

/// Magic bytes at the head of a phrases artifact, checked before decoding.
pub const PHRASES_ARTIFACT_MAGIC: &[u8; 8] = b"PHRAS001";

/// One word of a phrase, as the analyser segmented it.
///
/// The same three fields as a passage token, and for the same reasons — see
/// [`crate::passages::PassageToken`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhraseToken {
    /// The text as written in the phrase.
    pub surface: String,
    /// The reading to draw over it, in hiragana. `None` for a token whose surface
    /// has no kanji, which needs no ruby.
    pub rt: Option<String>,
    /// The vocabulary word this token is, when the course teaches it: the word's
    /// own `text`, so a screen can look it up in the words artifact.
    ///
    /// A `None` here is only allowed for a token with no kanji — `prepare-phrases`
    /// refuses to write a phrase containing a kanji-bearing word the course does
    /// not teach, because such a phrase is one a learner cannot read.
    pub word: Option<String>,
}

impl PhraseToken {
    pub fn has_kanji(&self) -> bool {
        self.surface.chars().any(crate::words::is_kanji)
    }

    /// The reading this token contributes to the phrase's kana text.
    pub fn reading(&self) -> &str {
        self.rt.as_deref().unwrap_or(&self.surface)
    }
}

/// One graded phrase.
///
/// `id`, `author` and `licence` are not decoration: they are the attribution the
/// corpus's licence requires to travel with the sentence, and the handle a reader
/// needs to find the original. A phrase without them would be an unattributed
/// copy, which is the one thing an import may not produce.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phrase {
    /// The corpus's own identifier for the sentence, so the original can be found
    /// and cited. Tatoeba's sentence id.
    pub id: u32,
    /// The sentence as written, including its final punctuation.
    pub text: String,
    /// The English translation the corpus pairs with it.
    pub english: String,
    /// The contributor the corpus names, for the attribution the licence requires.
    pub author: String,
    /// The licence this sentence is under, as the corpus states it.
    pub licence: String,
    /// This project's ladder: the band of the hardest word in the sentence, or 1
    /// for a sentence of nothing but kana.
    pub band: u8,
    /// The words, in order. Concatenating their surfaces spells [`Phrase::text`].
    pub tokens: Vec<PhraseToken>,
}

impl Phrase {
    /// Every token of the phrase, in order.
    pub fn tokens(&self) -> impl Iterator<Item = &PhraseToken> {
        self.tokens.iter()
    }

    /// The phrase as written, reassembled from its tokens.
    ///
    /// Equal to [`Phrase::text`] by construction — `prepare-phrases` only imports a
    /// sentence with no whitespace in it, which is what makes the reassembly exact
    /// — and the artifact test asserts the two agree rather than assuming it.
    pub fn text_from_tokens(&self) -> String {
        self.tokens
            .iter()
            .map(|token| token.surface.as_str())
            .collect()
    }

    /// The phrase in kana, as the readings spell it.
    pub fn kana(&self) -> String {
        self.tokens.iter().map(PhraseToken::reading).collect()
    }

    /// The vocabulary words the phrase contains, in order, without repeats.
    pub fn words(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        for token in &self.tokens {
            if let Some(word) = token.word.as_deref() {
                if !seen.contains(&word) {
                    seen.push(word);
                }
            }
        }
        seen
    }

    /// The kanji-bearing tokens the course has no word for.
    ///
    /// Empty is the invariant a phrase has to satisfy before it can be written
    /// into the artifact: a kanji the vocabulary does not cover is a character the
    /// learner has no way to read and no card to open.
    pub fn kanji_without_a_word(&self) -> Vec<&PhraseToken> {
        self.tokens
            .iter()
            .filter(|token| token.has_kanji() && token.word.is_none())
            .collect()
    }

    /// The band this phrase's words put it in, recomputed from the vocabulary.
    ///
    /// The artifact stores the band, and this is how it is *derived*, so a test can
    /// recompute every shipped phrase's band and fail if a stored one has drifted
    /// from the vocabulary it claims to be levelled against — the arrangement
    /// `tests/words_artifact.rs` already uses for the words themselves.
    ///
    /// A token the vocabulary does not hold contributes nothing; a phrase with no
    /// word at all — kana only — is band 1, because there is nothing in it to be
    /// hard.
    pub fn band_from(&self, words: &crate::words::WordDataset) -> u8 {
        self.tokens
            .iter()
            .filter_map(|token| token.word.as_deref())
            .filter_map(|text| words.of_text(text))
            .map(|word| word.band)
            .max()
            .unwrap_or(1)
    }
}

/// Which upstream snapshots a phrase corpus came from, and on what rule it was cut
/// down.
///
/// Recorded because an imported phrase set is not a fact about Japanese: it is one
/// corpus, filtered by one rule, on one day. A screen that shows these phrases is
/// showing *that selection*, and a later rebuild has to be able to say what changed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhrasesSource {
    /// e.g. `lindera 6.2`.
    pub analyser: String,
    /// e.g. `unidic-mecab-2.1.2`.
    pub dictionary: String,
    /// The corpus, named: `Tatoeba`.
    pub corpus: String,
    /// Where the corpus lives: `https://tatoeba.org`.
    pub url: String,
    /// The export the sentences were read from, as the file names it itself.
    pub export: String,
    /// Every licence value that occurs among the shipped phrases, comma-separated.
    /// Empty when no phrase is shipped.
    pub licences: String,
    /// The selection rule, in one sentence, so the artifact describes its own cut.
    pub selection: String,
}

/// The decoded payload of the shipped phrases artifact.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PhrasesArtifact {
    pub phrases: Vec<Phrase>,
    pub source: PhrasesSource,
}

impl PhrasesArtifact {
    pub fn new(phrases: Vec<Phrase>, source: PhrasesSource) -> Self {
        Self { phrases, source }
    }
}

/// The phrases, in the order the course offers them.
#[derive(Clone, Debug, Default)]
pub struct PhraseDataset {
    phrases: Vec<Phrase>,
    source: PhrasesSource,
}

impl PhraseDataset {
    /// Order the phrases the way a reader meets them: by band, then shortest
    /// first, then by the text itself, then by the corpus id.
    ///
    /// Shortest first is the reading ladder *inside* a band — the band already says
    /// how hard the hardest word is, and the length is what is left to say how much
    /// of it there is. The last two keys make the order total and stable, so a
    /// screen's list does not reshuffle when the export moves.
    pub fn from_phrases(mut phrases: Vec<Phrase>, source: PhrasesSource) -> Self {
        phrases.sort_by(|a, b| {
            a.band
                .cmp(&b.band)
                .then(a.text.chars().count().cmp(&b.text.chars().count()))
                .then(a.text.cmp(&b.text))
                .then(a.id.cmp(&b.id))
        });
        Self { phrases, source }
    }

    /// Decode an artifact produced by `prepare-phrases`.
    pub fn from_gzip_bytes(bytes: &[u8]) -> std::io::Result<Self> {
        use std::io::Read;

        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut raw = Vec::new();
        decoder.read_to_end(&mut raw)?;

        if raw.len() < PHRASES_ARTIFACT_MAGIC.len()
            || &raw[..PHRASES_ARTIFACT_MAGIC.len()] != PHRASES_ARTIFACT_MAGIC
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "not a phrases artifact (bad or outdated magic); re-run `prepare-phrases`",
            ));
        }
        let artifact: PhrasesArtifact =
            postcard::from_bytes(&raw[PHRASES_ARTIFACT_MAGIC.len()..]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("corrupt phrases artifact: {e}"),
                )
            })?;
        Ok(Self::from_phrases(artifact.phrases, artifact.source))
    }

    pub fn phrases(&self) -> &[Phrase] {
        &self.phrases
    }

    pub fn len(&self) -> usize {
        self.phrases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.phrases.is_empty()
    }

    pub fn source(&self) -> &PhrasesSource {
        &self.source
    }

    /// The phrase with this corpus id.
    pub fn get(&self, id: u32) -> Option<&Phrase> {
        self.phrases.iter().find(|phrase| phrase.id == id)
    }

    /// The phrases of one band, in order.
    pub fn of_band(&self, band: u8) -> impl Iterator<Item = &Phrase> {
        self.phrases.iter().filter(move |phrase| phrase.band == band)
    }

    /// How many phrases each band holds, for the band chips.
    ///
    /// Every band from 1 to [`crate::words::REMAINDER_BAND`] is present even when it
    /// holds nothing, so a screen drawing a chip per band does not have to know
    /// which bands the corpus happened to fill and which it did not.
    pub fn band_counts(&self) -> Vec<(u8, usize)> {
        let last = crate::words::REMAINDER_BAND;
        (1..=last)
            .map(|band| (band, self.of_band(band).count()))
            .collect()
    }

    /// Every licence value among the phrases, each once, in the order met.
    ///
    /// Derived rather than stored so it cannot disagree with the phrases, and the
    /// artifact's [`PhrasesSource::licences`] is what the pipeline writes from this
    /// same rule.
    pub fn licences(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        for phrase in &self.phrases {
            if !seen.contains(&phrase.licence.as_str()) {
                seen.push(&phrase.licence);
            }
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words::{Ruby, Word, WordDataset, WordsSource};

    fn word(text: &str, reading: &str, band: u8) -> Word {
        Word {
            text: text.to_string(),
            reading: reading.to_string(),
            furigana: vec![Ruby::new(text, Some(reading.to_string()))],
            meaning: "x".to_string(),
            band,
            nf: None,
        }
    }

    fn vocab() -> WordDataset {
        WordDataset::from_words(
            vec![
                word("学生", "がくせい", 1),
                word("私", "わたし", 6),
                word("行く", "いく", 2),
            ],
            WordsSource::default(),
        )
    }

    fn phrase(id: u32, band: u8, text: &str, tokens: Vec<PhraseToken>) -> Phrase {
        Phrase {
            id,
            text: text.to_string(),
            english: "t".to_string(),
            author: "someone".to_string(),
            licence: "CC BY 2.0 FR".to_string(),
            band,
            tokens,
        }
    }

    fn token(surface: &str, rt: Option<&str>, word: Option<&str>) -> PhraseToken {
        PhraseToken {
            surface: surface.to_string(),
            rt: rt.map(str::to_string),
            word: word.map(str::to_string),
        }
    }

    #[test]
    fn a_token_knows_whether_it_needs_ruby_and_what_it_reads_as() {
        let kanji = token("学生", Some("がくせい"), Some("学生"));
        assert!(kanji.has_kanji());
        assert_eq!(kanji.reading(), "がくせい");

        let kana = token("です", None, None);
        assert!(!kana.has_kanji());
        assert_eq!(kana.reading(), "です", "kana reads as itself");
    }

    #[test]
    fn the_kana_and_the_words_come_out_of_the_tokens_in_order() {
        let p = phrase(
            1,
            1,
            "私は学生です",
            vec![
                token("私", Some("わたし"), Some("私")),
                token("は", None, None),
                token("学生", Some("がくせい"), Some("学生")),
                token("です", None, None),
            ],
        );
        assert_eq!(p.text_from_tokens(), "私は学生です");
        assert_eq!(p.kana(), "わたしはがくせいです");
        assert_eq!(p.words(), vec!["私", "学生"]);
    }

    #[test]
    fn a_repeated_word_is_listed_once() {
        let p = phrase(
            1,
            1,
            "学生と学生",
            vec![
                token("学生", Some("がくせい"), Some("学生")),
                token("と", None, None),
                token("学生", Some("がくせい"), Some("学生")),
            ],
        );
        assert_eq!(p.words(), vec!["学生"]);
    }

    /// The invariant, as a method: a kanji the vocabulary does not cover is one a
    /// learner cannot read and cannot tap.
    #[test]
    fn a_kanji_the_vocabulary_does_not_cover_is_reported() {
        let p = phrase(
            1,
            1,
            "学生です麒麟",
            vec![
                token("学生", Some("がくせい"), Some("学生")),
                token("です", None, None),
                token("麒麟", Some("きりん"), None),
            ],
        );
        let uncovered = p.kanji_without_a_word();
        assert_eq!(uncovered.len(), 1);
        assert_eq!(uncovered[0].surface, "麒麟");
    }

    #[test]
    fn the_band_is_the_hardest_word_and_a_kana_only_phrase_is_the_first() {
        let words = vocab();
        let hard = phrase(
            1,
            6,
            "私は学生",
            vec![
                token("私", Some("わたし"), Some("私")),
                token("は", None, None),
                token("学生", Some("がくせい"), Some("学生")),
            ],
        );
        assert_eq!(hard.band_from(&words), 6, "the hardest word decides");

        let kana_only = phrase(
            2,
            1,
            "おはよう",
            vec![token("おはよう", None, None)],
        );
        assert_eq!(kana_only.band_from(&words), 1, "nothing in it to be hard");
    }

    #[test]
    fn the_order_is_band_then_text_then_id() {
        let dataset = PhraseDataset::from_phrases(
            vec![
                phrase(9, 2, "b", vec![]),
                phrase(3, 1, "c", vec![]),
                phrase(1, 1, "a", vec![]),
                phrase(2, 1, "a", vec![]),
            ],
            PhrasesSource::default(),
        );
        let ids: Vec<u32> = dataset.phrases().iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![1, 2, 3, 9]);
        assert_eq!(
            dataset.of_band(1).map(|p| p.id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn every_band_has_a_count_even_when_it_holds_nothing() {
        let dataset = PhraseDataset::from_phrases(
            vec![phrase(1, 3, "a", vec![])],
            PhrasesSource::default(),
        );
        let counts = dataset.band_counts();
        assert_eq!(counts.len(), 7);
        assert_eq!(counts[2], (3, 1));
        assert_eq!(counts[0], (1, 0));
        assert_eq!(counts[6], (7, 0));
    }

    #[test]
    fn the_licences_are_reported_once_each_in_the_order_met() {
        let mut a = phrase(1, 1, "a", vec![]);
        let mut b = phrase(2, 1, "b", vec![]);
        a.licence = "CC0 1.0".to_string();
        b.licence = "CC BY 2.0 FR".to_string();
        let dataset =
            PhraseDataset::from_phrases(vec![a, b], PhrasesSource::default());
        assert_eq!(dataset.licences(), vec!["CC0 1.0", "CC BY 2.0 FR"]);
    }

    #[test]
    fn an_artifact_that_is_not_one_is_refused_by_its_magic() {
        use std::io::Write;

        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(b"WORDD002\x00\x00").unwrap();
        let bytes = encoder.finish().unwrap();
        let err = PhraseDataset::from_gzip_bytes(&bytes).unwrap_err();
        assert!(err.to_string().contains("not a phrases artifact"), "{err}");
    }

    #[test]
    fn an_artifact_round_trips_through_postcard_and_gzip() {
        use std::io::Write;

        let dataset = PhraseDataset::from_phrases(
            vec![phrase(
                42,
                2,
                "学生です",
                vec![
                    token("学生", Some("がくせい"), Some("学生")),
                    token("です", None, None),
                ],
            )],
            PhrasesSource {
                analyser: "lindera 6.2".to_string(),
                dictionary: "unidic-mecab-2.1.2".to_string(),
                corpus: "Tatoeba".to_string(),
                url: "https://tatoeba.org".to_string(),
                export: "jpn_sentences_detailed".to_string(),
                licences: "CC BY 2.0 FR".to_string(),
                selection: "one rule".to_string(),
            },
        );
        let payload = postcard::to_allocvec(&PhrasesArtifact::new(
            dataset.phrases().to_vec(),
            dataset.source().clone(),
        ))
        .unwrap();
        let mut raw = PHRASES_ARTIFACT_MAGIC.to_vec();
        raw.extend_from_slice(&payload);
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(&raw).unwrap();
        let bytes = encoder.finish().unwrap();

        let decoded = PhraseDataset::from_gzip_bytes(&bytes).expect("decodes");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded.source().corpus, "Tatoeba");
        assert_eq!(decoded.get(42).map(|p| p.kana()), Some("がくせいです".into()));
        assert!(decoded.get(43).is_none());
    }
}
