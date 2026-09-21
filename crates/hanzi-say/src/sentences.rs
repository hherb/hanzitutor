//! Reading graded phrases out of the two sentence files.
//!
//! Both corpora are kept deliberately separate — different licences, so a single
//! merged list would make it easy to ship one under the other's notice by
//! accident. What this module does is the narrow part they share: turning
//! whichever shape arrived into a [`Phrase`].
//!
//! It reads the *slice* files committed under `data/slices/`, which are extracts
//! of the upstream datasets rather than the datasets themselves. That keeps the
//! synthesis run reproducible from a clone without pulling 60 MB of upstream
//! text, which is the same trade `data/raw/` makes for the course artifact.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// One phrase to synthesise.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Phrase {
    /// Stable id within the corpus, e.g. `hsk1-0002`. Becomes the file name.
    pub id: String,
    /// HSK level, or a reader shelf name for the graded readers.
    #[serde(default)]
    pub level: String,
    /// The characters to speak.
    pub text: String,
    /// Space-separated readings, for the manifest and the UI.
    #[serde(default)]
    pub pinyin: String,
    /// English gloss.
    #[serde(default)]
    pub translation: String,
}

/// A speed a clip is written at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Speed {
    /// The model's own rate. Written as `<id>.mp3`.
    Normal,
    /// Slower, for a first hearing. Written as `<id>_slow.mp3`.
    Slow,
}

impl Speed {
    /// The rate multiplier handed to the model.
    pub fn multiplier(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Slow => 0.7,
        }
    }

    /// The file-name suffix, empty for the normal take.
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Normal => "",
            Self::Slow => "_slow",
        }
    }

    /// Both speeds, normal first — the order the build tool writes them.
    pub fn all() -> [Speed; 2] {
        [Speed::Normal, Speed::Slow]
    }
}

/// A sentence file could not be read.
#[derive(Debug)]
pub struct LoadError {
    pub path: String,
    pub why: String,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not read {}: {}", self.path, self.why)
    }
}

impl std::error::Error for LoadError {}

/// Raw shapes, before normalisation. One enum arm per corpus.
#[derive(Deserialize)]
#[serde(untagged)]
enum Raw {
    /// `no7z`, in either of its two spellings. As published each line carries
    /// `hsk_level` and a nested `translation`; the flat extract committed under
    /// `data/slices/` carries `hsk` and a bare `en`. Both are accepted by one
    /// arm rather than two, because two arms whose required fields are
    /// `chinese` and `pinyin` cannot be told apart — `serde` would take
    /// whichever came first and silently leave the other's fields empty.
    Rows(Vec<Row>),
    /// `harukicoder` as published: one record per reader with nested sentences.
    Readers(Vec<Reader>),
    /// A flat extract of the above, which is what `data/slices/` holds.
    FlatSentences(Vec<FlatSentence>),
}

/// One graded sentence, in either of the two spellings `no7z` uses.
#[derive(Deserialize)]
struct Row {
    id: String,
    #[serde(default)]
    hsk_level: serde_json::Value,
    #[serde(default)]
    hsk: serde_json::Value,
    chinese: String,
    #[serde(default)]
    pinyin: String,
    #[serde(default)]
    en: String,
    #[serde(default)]
    translation: Translation,
}

impl Row {
    /// The level, from whichever of the two fields was present.
    fn level(&self) -> String {
        let value = if self.hsk_level.is_null() {
            &self.hsk
        } else {
            &self.hsk_level
        };
        match value {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        }
    }

    /// The English gloss, from whichever of the two fields was present.
    fn gloss(&self) -> String {
        if self.translation.en.is_empty() {
            self.en.clone()
        } else {
            self.translation.en.clone()
        }
    }
}

#[derive(Deserialize, Default)]
struct Translation {
    #[serde(default)]
    en: String,
}

#[derive(Deserialize)]
struct Reader {
    id: String,
    #[serde(default)]
    shelf: String,
    #[serde(default)]
    sentences: Vec<ReaderSentence>,
}

#[derive(Deserialize)]
struct ReaderSentence {
    hz: String,
    #[serde(default)]
    en: String,
    #[serde(default)]
    words: Vec<ReaderWord>,
}

#[derive(Deserialize)]
struct ReaderWord {
    #[serde(default)]
    py: String,
}

#[derive(Deserialize)]
struct FlatSentence {
    #[serde(default)]
    text_id: String,
    #[serde(default)]
    shelf: String,
    hz: String,
    #[serde(default)]
    en: String,
    #[serde(default)]
    words: Vec<ReaderWord>,
}

/// Parse either a JSON array or newline-delimited JSON.
///
/// Both are needed and neither is guesswork: the slices committed under
/// `data/slices/` are arrays, because that is what is readable in a diff, while
/// the corpora as published are newline-delimited — one JSON object per line,
/// which is the format that lets a large dataset be streamed. A file that is
/// neither gets the array parser's error, which names the first offending line.
fn parse(text: &str) -> Result<Raw, String> {
    let trimmed = text.trim_start();
    if trimmed.starts_with('[') {
        return serde_json::from_str(text).map_err(|e| e.to_string());
    }
    let mut items = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => items.push(value),
            Err(e) => return Err(format!("line {}: {e}", n + 1)),
        }
    }
    serde_json::from_value(serde_json::Value::Array(items)).map_err(|e| e.to_string())
}

/// Read a sentence file, whichever shape it has.
///
/// The arms are tried in order by `serde`, so a file that is a list of readers
/// is not mistaken for a list of graded rows — the arms' required fields do not
/// overlap enough for that to happen silently.
pub fn load(path: impl AsRef<Path>) -> Result<Vec<Phrase>, LoadError> {    let path = path.as_ref();
    let err = |why: String| LoadError {
        path: path.display().to_string(),
        why,
    };
    let text = std::fs::read_to_string(path).map_err(|e| err(e.to_string()))?;
    let raw: Raw = parse(&text).map_err(err)?;

    let phrases = match raw {
        Raw::Rows(rows) => rows
            .into_iter()
            .map(|r| {
                // Read from both shapes before moving anything out of `r`.
                let level = r.level();
                let translation = r.gloss();
                Phrase {
                    level,
                    id: r.id,
                    pinyin: r.pinyin,
                    text: r.chinese,
                    translation,
                }
            })
            .collect(),
        Raw::Readers(readers) => readers
            .into_iter()
            .flat_map(|r| {
                let shelf = r.shelf.clone();
                let id = r.id.clone();
                r.sentences.into_iter().enumerate().map(move |(i, s)| Phrase {
                    id: format!("{id}-{}", i + 1),
                    level: shelf.clone(),
                    pinyin: s
                        .words
                        .iter()
                        .map(|w| w.py.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    text: s.hz,
                    translation: s.en,
                })
            })
            .collect(),
        Raw::FlatSentences(rows) => rows
            .into_iter()
            .enumerate()
            .map(|(i, r)| Phrase {
                id: format!("{}-{}", if r.text_id.is_empty() { "r" } else { &r.text_id }, i + 1),
                level: r.shelf,
                pinyin: r
                    .words
                    .iter()
                    .map(|w| w.py.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                text: r.hz,
                translation: r.en,
            })
            .collect(),
    };
    Ok(phrases)
}

/// A file-name-safe name for a phrase, namespaced by corpus.
///
/// The corpus is part of the path as well as the name so that the two corpora's
/// clips, and their differing notices, cannot be confused for one another.
pub fn clip_stem(source: &str, id: &str) -> String {
    let safe: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '-' })
        .collect();
    format!("{source}/{safe}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp(name: &str, body: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("hanzi-say-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn reads_newline_delimited_json_too() {
        // The corpora are published one object per line; the slices are arrays.
        // Both go through `parse`, and this is the arm that would otherwise be
        // exercised only by a real download.
        let path = write_temp(
            "ndjson.jsonl",
            "{\"id\":\"hsk1-0001\",\"hsk_level\":1,\"chinese\":\"你好！\",\"pinyin\":\"nǐ hǎo\",\"translation\":{\"en\":\"Hello!\"}}\n\
             {\"id\":\"hsk2-0001\",\"hsk_level\":2,\"chinese\":\"再见！\",\"pinyin\":\"zài jiàn\",\"translation\":{\"en\":\"Goodbye!\"}}\n",
        );
        let phrases = load(&path).unwrap();
        assert_eq!(phrases.len(), 2);
        assert_eq!(phrases[0].level, "1");
        assert_eq!(phrases[1].level, "2");
        assert_eq!(phrases[1].translation, "Goodbye!");
    }

    #[test]
    fn a_malformed_line_names_itself() {
        let path = write_temp("bad.jsonl", "{\"id\":\"a\",\"chinese\":\"好\"}\nnot json\n");
        let err = load(&path).unwrap_err();
        assert!(err.to_string().contains("line 2"), "{err}");
    }

    #[test]
    fn reads_the_upstream_published_shape() {
        // As it appears in the dataset's own `train.jsonl`: `hsk_level` and a
        // nested `translation`. One arm reads both this and the flat extract, so
        // this test is what keeps that arm honest about the nested gloss.
        let path = write_temp(
            "upstream.jsonl",
            r#"[{"id":"hsk1-0001","hsk_level":1,"chinese":"老师，您好！","pinyin":"lǎo shī nín hǎo","translation":{"en":"Hello, teacher!"},"tokens":[{"word":"老师","pinyin":"lǎo shī","gloss_en":"teacher"}]}]"#,
        );
        let phrases = load(&path).unwrap();
        assert_eq!(phrases.len(), 1);
        assert_eq!(phrases[0].level, "1");
        assert_eq!(phrases[0].translation, "Hello, teacher!");
    }

    #[test]
    fn reads_the_graded_row_shape() {
        let path = write_temp(
            "no7z.json",
            r#"[{"id":"hsk1-0001","hsk":1,"chinese":"老师，您好！","pinyin":"lǎo shī nín hǎo","en":"Hello, teacher!"}]"#,
        );
        let phrases = load(&path).unwrap();
        assert_eq!(phrases.len(), 1);
        assert_eq!(phrases[0].id, "hsk1-0001");
        assert_eq!(phrases[0].level, "1");
        assert_eq!(phrases[0].text, "老师，您好！");
        assert_eq!(phrases[0].translation, "Hello, teacher!");
    }

    #[test]
    fn reads_the_nested_reader_shape_and_numbers_its_sentences() {
        let path = write_temp(
            "readers.json",
            r#"[{"id":"c1","shelf":"newbie","sentences":[
                 {"hz":"一只小鸟很渴。","en":"A bird was thirsty.","words":[{"hz":"一只","py":"yì zhī"}]},
                 {"hz":"它想喝水。","en":"It wanted water.","words":[{"hz":"它","py":"tā"}]}]}]"#,
        );
        let phrases = load(&path).unwrap();
        assert_eq!(phrases.len(), 2);
        assert_eq!(phrases[0].id, "c1-1");
        assert_eq!(phrases[1].id, "c1-2");
        assert_eq!(phrases[0].level, "newbie");
        assert_eq!(phrases[0].pinyin, "yì zhī");
    }

    #[test]
    fn reads_the_flat_sentence_shape() {
        let path = write_temp(
            "flat.json",
            r#"[{"text_id":"c1","shelf":"newbie","hz":"你好!","en":"Hello!","words":[{"py":"nǐ hǎo"}]}]"#,
        );
        let phrases = load(&path).unwrap();
        assert_eq!(phrases.len(), 1);
        assert_eq!(phrases[0].id, "c1-1");
        assert_eq!(phrases[0].pinyin, "nǐ hǎo");
    }

    #[test]
    fn a_missing_file_names_itself() {
        let err = load("/nonexistent/phrases.json").unwrap_err();
        assert!(err.to_string().contains("/nonexistent/phrases.json"));
    }

    #[test]
    fn clip_stems_are_namespaced_and_sanitised() {
        assert_eq!(clip_stem("no7z", "hsk1-0001"), "no7z/hsk1-0001");
        assert_eq!(clip_stem("harukicoder", "c1/2"), "harukicoder/c1-2");
    }

    #[test]
    fn speeds_agree_with_their_suffixes() {
        assert_eq!(Speed::Normal.suffix(), "");
        assert_eq!(Speed::Slow.suffix(), "_slow");
        assert_eq!(Speed::Normal.multiplier(), 1.0);
        assert!(Speed::Slow.multiplier() < 1.0);
    }
}
