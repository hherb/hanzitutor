//! The reading passages: short texts, segmented into words, with the reading of
//! every kanji-bearing word over it.
//!
//! # Why the passages are written here rather than taken from a corpus
//!
//! Tatoeba gives sentences rather than passages and carries no difficulty signal at
//! all — its own position is that the collection "is not fundamentally sorted" — and
//! it is now what the *phrases* artifact is imported from, where short sentences are
//! what is wanted. Aozora Bunko is genuinely free but is pre-1930s literary
//! Japanese, which uses none of this course's vocabulary. Wikipedia is modern and
//! unlevelled. Writing the passages here costs writing, and it buys the one property
//! none of the corpora have: **the text can be held to the vocabulary the course
//! actually teaches**, which is asserted rather than hoped for — see
//! [`Passage::kanji_without_a_word`] and `prepare-passages`, which refuses to write
//! an artifact whose passages use a kanji the words artifact does not hold.
//!
//! # The source files
//!
//! One UTF-8 file per passage under `crates/nihongo-core/data/passages/`, with
//! `key:`, `title:` and `gloss:` headers and the text after them:
//!
//! ```text
//! # key: asa
//! # title: あさ
//! # gloss: Morning.
//! おはよう。わたしは がくせい です。
//! ```
//!
//! The text is plain Japanese with no markup: the segmentation and every reading
//! come from the analyser at build time, so a passage file stays something a
//! person can read and edit without knowing the artifact format. `key` is the
//! stable identity a screen uses; the file name is the fallback when it is absent.
//!
//! # What a token carries
//!
//! [`PassageToken::surface`] is what is written, [`PassageToken::rt`] is the
//! reading to draw over it (hiragana, and only when the surface contains a kanji —
//! a kana token needs no ruby), and [`PassageToken::word`] is the vocabulary word
//! it is, when it is one, so that tapping it can open that word's own card. The
//! link is to the word's **dictionary form**: a passage that says 食べた links to
//! 食べる, because that is the entry the course teaches.

use serde::{Deserialize, Serialize};

/// Magic bytes at the head of a passages artifact, checked before decoding.
pub const PASSAGES_ARTIFACT_MAGIC: &[u8; 8] = b"PASSD001";

/// One word of a passage, as the analyser segmented it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassageToken {
    /// The text as written in the passage.
    pub surface: String,
    /// The reading to draw over it, in hiragana. `None` for a token whose surface
    /// has no kanji, which needs no ruby.
    pub rt: Option<String>,
    /// The vocabulary word this token is, when the course teaches it: the word's
    /// own `text`, so a screen can look it up in the words artifact.
    ///
    /// A `None` here is only allowed for a token with no kanji — `prepare-passages`
    /// refuses to write a passage containing a kanji-bearing word the course does
    /// not teach, because such a passage is one a learner cannot read.
    pub word: Option<String>,
}

impl PassageToken {
    pub fn has_kanji(&self) -> bool {
        self.surface.chars().any(crate::words::is_kanji)
    }

    /// The reading this token contributes to the passage's kana text.
    pub fn reading(&self) -> &str {
        self.rt.as_deref().unwrap_or(&self.surface)
    }
}

/// One passage: a title, its lines, and an optional English gloss.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Passage {
    /// Stable identity, from the `key:` header or the file name.
    pub key: String,
    pub title: String,
    /// The English gloss, when the source gives one.
    pub gloss: Option<String>,
    /// The tokens, line by line — the line structure is the source's and a screen
    /// renders it rather than re-flowing.
    pub lines: Vec<Vec<PassageToken>>,
}

impl Passage {
    /// The passage as written, one line per line.
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|token| token.surface.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The passage in kana, as the readings spell it.
    pub fn kana(&self) -> String {
        self.lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(PassageToken::reading)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn tokens(&self) -> impl Iterator<Item = &PassageToken> {
        self.lines.iter().flatten()
    }

    /// The kanji-bearing tokens the course has no word for.
    ///
    /// Empty is the invariant a passage has to satisfy before it can be written
    /// into the artifact: a kanji the vocabulary does not cover is a character the
    /// learner has no way to read and no card to open.
    pub fn kanji_without_a_word(&self) -> Vec<&PassageToken> {
        self.tokens()
            .filter(|token| token.has_kanji() && token.word.is_none())
            .collect()
    }
}

/// Which analyser and dictionary produced the segmentation.
///
/// Recorded because a segmentation is not a fact about Japanese the way a stroke
/// count is: it is one tokeniser's answer, and a different dictionary version would
/// move word boundaries. A screen that shows these passages is showing *UniDic's*
/// reading of them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassagesSource {
    /// e.g. `lindera 6.2.0`.
    pub analyser: String,
    /// e.g. `unidic-mecab-2.1.2`.
    pub dictionary: String,
}

/// The decoded payload of the shipped passages artifact.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PassagesArtifact {
    pub passages: Vec<Passage>,
    pub source: PassagesSource,
}

impl PassagesArtifact {
    pub fn new(passages: Vec<Passage>, source: PassagesSource) -> Self {
        Self { passages, source }
    }
}

/// The passages, in the order the course reads them.
#[derive(Clone, Debug, Default)]
pub struct PassageDataset {
    passages: Vec<Passage>,
    source: PassagesSource,
}

impl PassageDataset {
    pub fn from_passages(passages: Vec<Passage>, source: PassagesSource) -> Self {
        Self { passages, source }
    }

    /// Decode an artifact produced by `prepare-passages`.
    pub fn from_gzip_bytes(bytes: &[u8]) -> std::io::Result<Self> {
        use std::io::Read;

        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut raw = Vec::new();
        decoder.read_to_end(&mut raw)?;

        if raw.len() < PASSAGES_ARTIFACT_MAGIC.len()
            || &raw[..PASSAGES_ARTIFACT_MAGIC.len()] != PASSAGES_ARTIFACT_MAGIC
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "not a passages artifact (bad or outdated magic); re-run `prepare-passages`",
            ));
        }
        let artifact: PassagesArtifact =
            postcard::from_bytes(&raw[PASSAGES_ARTIFACT_MAGIC.len()..]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("corrupt passages artifact: {e}"),
                )
            })?;
        Ok(Self::from_passages(artifact.passages, artifact.source))
    }

    pub fn passages(&self) -> &[Passage] {
        &self.passages
    }

    pub fn len(&self) -> usize {
        self.passages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.passages.is_empty()
    }

    pub fn source(&self) -> &PassagesSource {
        &self.source
    }

    pub fn get(&self, key: &str) -> Option<&Passage> {
        self.passages.iter().find(|passage| passage.key == key)
    }
}

/// Parse one passage source file, or say what is wrong with it.
///
/// The headers are `key:`, `title:` and `gloss:`, before the text, each on its own
/// line starting with `#`. An unknown header is an error rather than a shrug: a
/// misspelled `titel:` would otherwise be a passage with no title and nobody the
/// wiser.
pub fn parse_passage(fallback_key: &str, source: &str) -> Result<Passage, String> {
    let mut key = None;
    let mut title = None;
    let mut gloss = None;
    let mut body: Vec<String> = Vec::new();

    for (number, line) in source.lines().enumerate() {
        let where_ = format!("line {}", number + 1);
        if let Some(header) = line.strip_prefix('#') {
            let header = header.trim();
            if header.is_empty() {
                continue;
            }
            let Some((name, value)) = header.split_once(':') else {
                return Err(format!("{where_}: {header:?} is not `name: value`"));
            };
            let value = value.trim().to_string();
            match name.trim() {
                "key" => key = Some(value),
                "title" => title = Some(value),
                "gloss" => gloss = Some(value),
                other => {
                    return Err(format!(
                        "{where_}: unknown header {other:?}; expected key, title or gloss"
                    ))
                }
            }
            continue;
        }
        // A blank line is a paragraph break the source is entitled to; it is not
        // carried into the artifact, which stores lines.
        if !line.trim().is_empty() {
            body.push(line.trim_end().to_string());
        }
    }

    if body.is_empty() {
        return Err("the passage has no text".to_string());
    }
    Ok(Passage {
        key: key.unwrap_or_else(|| fallback_key.to_string()),
        title: title.ok_or_else(|| "the passage has no `title:` header".to_string())?,
        gloss,
        lines: body
            .into_iter()
            .map(|line| {
                vec![PassageToken {
                    surface: line,
                    rt: None,
                    word: None,
                }]
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# key: asa
# title: あさ
# gloss: Morning.
おはよう。

わたしは がくせい です。
";

    #[test]
    fn a_source_file_parses_into_one_passage_per_file() {
        let passage = parse_passage("fallback", SAMPLE).expect("parses");
        assert_eq!(passage.key, "asa");
        assert_eq!(passage.title, "あさ");
        assert_eq!(passage.gloss.as_deref(), Some("Morning."));
        assert_eq!(passage.lines.len(), 2, "a blank line is not a line of text");
        assert_eq!(passage.text(), "おはよう。\nわたしは がくせい です。");
    }

    #[test]
    fn the_file_name_is_the_key_when_the_header_is_absent() {
        let passage = parse_passage("from-the-name", "# title: あ\nあ").expect("parses");
        assert_eq!(passage.key, "from-the-name");
        assert_eq!(passage.gloss, None);
    }

    #[test]
    fn a_misspelled_header_is_refused_rather_than_ignored() {
        let err = parse_passage("k", "# titel: あ\nあ").unwrap_err();
        assert!(err.contains("unknown header"), "{err}");
        let err = parse_passage("k", "# title\nあ").unwrap_err();
        assert!(err.contains("name: value"), "{err}");
    }

    #[test]
    fn a_passage_with_no_title_or_no_text_is_refused() {
        assert!(parse_passage("k", "# key: k\nあ").unwrap_err().contains("title"));
        assert!(parse_passage("k", "# title: あ\n\n").unwrap_err().contains("no text"));
    }

    #[test]
    fn a_token_knows_whether_it_needs_ruby_and_what_it_reads_as() {
        let kanji = PassageToken {
            surface: "学生".to_string(),
            rt: Some("がくせい".to_string()),
            word: Some("学生".to_string()),
        };
        assert!(kanji.has_kanji());
        assert_eq!(kanji.reading(), "がくせい");

        let kana = PassageToken {
            surface: "です".to_string(),
            rt: None,
            word: None,
        };
        assert!(!kana.has_kanji());
        assert_eq!(kana.reading(), "です", "kana reads as itself");
    }

    /// The invariant, as a method: a kanji the vocabulary does not cover is one a
    /// learner cannot read and cannot tap.
    #[test]
    fn a_kanji_the_vocabulary_does_not_cover_is_reported() {
        let passage = Passage {
            key: "k".to_string(),
            title: "t".to_string(),
            gloss: None,
            lines: vec![vec![
                PassageToken {
                    surface: "学生".to_string(),
                    rt: Some("がくせい".to_string()),
                    word: Some("学生".to_string()),
                },
                PassageToken {
                    surface: "です".to_string(),
                    rt: None,
                    word: None,
                },
                PassageToken {
                    surface: "麒麟".to_string(),
                    rt: Some("きりん".to_string()),
                    word: None,
                },
            ]],
        };
        let uncovered = passage.kanji_without_a_word();
        assert_eq!(uncovered.len(), 1);
        assert_eq!(uncovered[0].surface, "麒麟");
        assert_eq!(passage.text(), "学生です麒麟");
        assert_eq!(passage.kana(), "がくせいですきりん");
    }

    #[test]
    fn an_artifact_that_is_not_one_is_refused_by_its_magic() {
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut encoder, b"WORDD001\x00\x00").unwrap();
        let bytes = encoder.finish().unwrap();
        let err = PassageDataset::from_gzip_bytes(&bytes).unwrap_err();
        assert!(err.to_string().contains("not a passages artifact"), "{err}");
    }
}
