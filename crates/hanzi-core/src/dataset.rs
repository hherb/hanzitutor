//! The character dataset: stroke geometry plus lexical metadata.
//!
//! The dataset is built offline by the `prepare-data` binary from three upstream
//! sources and shipped as a single compressed artifact:
//!
//! * **geometry** (stroke outlines and stroke-order centre-lines) comes from
//!   [Make Me a Hanzi](https://github.com/skishore/makemeahanzi), which is
//!   derived from the Arphic PL fonts.
//! * **lexical metadata** (frequency rank, pinyin, meaning, radical, HSK level)
//!   comes from [`hanziDB.csv`](https://github.com/ruddfawcett/hanziDB.csv),
//!   an MIT-licensed list derived from Jun Da's Modern Chinese Character
//!   Frequency List. Etymology hints come from Make Me a Hanzi's
//!   `dictionary.txt`.
//! * **words** come from
//!   [`complete-hsk-vocabulary`](https://github.com/drkameleon/complete-hsk-vocabulary),
//!   an MIT compilation of the official HSK 2.0/3.0 lists whose readings and
//!   definitions are drawn from CC-CEDICT (CC BY-SA 4.0).
//!
//! See `LICENSES.md` for the full notices that must accompany redistribution.

use std::collections::{HashMap, HashSet};
use std::io::Read;

use serde::{Deserialize, Serialize};

use crate::geom::Point;

/// Magic bytes and format version at the head of an uncompressed artifact.
///
/// `02` added the word list. A stale artifact fails loudly on the magic rather
/// than decoding into nonsense, because `postcard` is not self-describing.
pub const ARTIFACT_MAGIC: &[u8; 8] = b"HANZID02";

/// One character, with everything needed both to display it and to grade a
/// handwritten attempt at it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub ch: char,
    /// Frequency rank (1 = most common). `0` means the character is not in the
    /// frequency list, which is the case for most traditional-only forms.
    pub rank: u32,
    /// HSK level 1..=7, or `0` when the character is not in the HSK lists.
    pub hsk: u8,
    /// Number of strokes, as listed upstream.
    pub stroke_count: u8,
    /// Kangxi radical, or `'\0'` when unknown.
    pub radical: char,
    pub pinyin: Vec<String>,
    pub definition: String,
    /// Short mnemonic hint, when one exists.
    pub etymology: String,
    /// SVG path data in **font space**, one entry per stroke, in stroke order.
    /// Render within `scale(1, -1) translate(0, -900)` over a 1024x1024 box.
    pub outlines: Vec<String>,
    /// Stroke centre-lines in **display space**, in stroke order. Compared
    /// against a user's strokes by [`crate::grade`].
    pub medians: Vec<Vec<Point>>,
}

impl Character {
    pub fn reference_medians(&self) -> &[Vec<Point>] {
        &self.medians
    }

    /// True when this character can be taught: it has geometry and a rank.
    pub fn is_teachable(&self) -> bool {
        self.rank > 0 && !self.medians.is_empty()
    }
}

/// One word from the HSK vocabulary: several characters learned and practised
/// together.
///
/// **Single-character HSK entries are deliberately not carried here.** The
/// course already teaches every character with its most common reading, and a
/// second entry for the same glyph would be a competing source of truth — the
/// dictionary lists 安 as the surname `Ān` before `ān` "peaceful". A word is
/// multi-character, which is exactly what the character dataset cannot express.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    /// The word in simplified characters, e.g. `"学习"`.
    pub text: String,
    /// The reading of the **whole word**, e.g. `"xuéxí"`. Measured from a
    /// dictionary rather than composed from the characters, which is what makes
    /// a polyphonic word such as 着急 come out `zháojí` and not `zhejí`.
    pub pinyin: String,
    /// English definition. Several senses are joined with `"; "`.
    pub meaning: String,
    /// Lowest HSK 3.0 level the word appears in, 1..=7.
    pub hsk: u8,
    /// Derived frequency: the rank of the word's **rarest** character, taken
    /// from the same frequency list the course is built from. A word is no more
    /// common than its least common character. `0` when unknown.
    ///
    /// This is used for ordering and must not be presented as a published word
    /// frequency: it is an estimate from character data.
    pub rank: u32,
}

impl Word {
    /// The characters of the word, in order.
    pub fn characters(&self) -> Vec<char> {
        self.text.chars().collect()
    }
}

/// The decoded payload of the shipped artifact.
///
/// It is one document rather than two files so that a build either has a
/// complete dataset or fails the magic check, and so there is a single
/// `include_bytes!` to keep honest.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Artifact {
    pub characters: Vec<Character>,
    pub words: Vec<Word>,
}

impl Artifact {
    pub fn new(characters: Vec<Character>, words: Vec<Word>) -> Self {
        Self { characters, words }
    }
}

/// An indexed collection of characters and words, characters ordered by
/// frequency rank.
#[derive(Clone, Debug, Default)]
pub struct Dataset {
    chars: Vec<Character>,
    index: HashMap<char, usize>,
    /// Words, most useful first: by HSK level, then derived frequency.
    words: Vec<Word>,
    word_index: HashMap<String, usize>,
    /// Which words each character appears in, most useful first.
    words_by_char: HashMap<char, Vec<usize>>,
    /// Search keys, precomputed once because a search box runs on every
    /// keystroke and folding 9,000 readings per keystroke is wasted work.
    word_keys: Vec<WordKey>,
}

/// A word's precomputed, comparison-ready text.
#[derive(Clone, Debug, Default)]
struct WordKey {
    /// Reading with tone marks stripped, spaces removed and `v`/`ü` unified,
    /// so `xuexi`, `xuéxí` and `xüexi` all find 学习.
    pinyin: String,
    /// Definition lowercased.
    meaning: String,
}

impl Dataset {
    /// Build a dataset from characters, sorting them into frequency order.
    ///
    /// The order is normalised here rather than trusted, so [`Dataset::ranked`]
    /// really does yield most-common-first regardless of how the caller
    /// supplied the characters. Unranked characters sort last, then by
    /// codepoint, so the result is deterministic.
    pub fn from_chars(chars: Vec<Character>) -> Self {
        Self::from_parts(chars, Vec::new())
    }

    /// Build a dataset from characters and words.
    ///
    /// Both orders are normalised here rather than trusted, so the artifact's
    /// byte layout can change without changing behaviour.
    pub fn from_parts(mut chars: Vec<Character>, mut words: Vec<Word>) -> Self {
        chars.sort_by_key(|c| {
            let rank = if c.rank == 0 { u32::MAX } else { c.rank };
            (rank, c.ch as u32)
        });
        let index = chars
            .iter()
            .enumerate()
            .map(|(i, c)| (c.ch, i))
            .collect();

        words.sort_by(|a, b| {
            (a.hsk, rank_key(a.rank), &a.text).cmp(&(b.hsk, rank_key(b.rank), &b.text))
        });
        // `dedup_by` would only catch adjacent repeats, and the same word can
        // sort to two places when it was supplied at two levels. Retaining the
        // first occurrence after the sort keeps the most useful one, and means a
        // hand-built dataset cannot hold the same word twice.
        let mut seen: HashSet<String> = HashSet::new();
        words.retain(|w| seen.insert(w.text.clone()));

        let word_index = words
            .iter()
            .enumerate()
            .map(|(i, w)| (w.text.clone(), i))
            .collect();

        let mut words_by_char: HashMap<char, Vec<usize>> = HashMap::new();
        for (i, word) in words.iter().enumerate() {
            for ch in word.text.chars() {
                words_by_char.entry(ch).or_default().push(i);
            }
        }

        let word_keys = words
            .iter()
            .map(|w| WordKey {
                pinyin: fold_pinyin(&w.pinyin),
                meaning: w.meaning.to_lowercase(),
            })
            .collect();

        Self {
            chars,
            index,
            words,
            word_index,
            words_by_char,
            word_keys,
        }
    }

    /// Decode an artifact produced by `prepare-data`: gzip-compressed, magic
    /// prefixed, then a `postcard`-encoded [`Artifact`].
    pub fn from_gzip_bytes(bytes: &[u8]) -> std::io::Result<Self> {
        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut raw = Vec::new();
        decoder.read_to_end(&mut raw)?;

        if raw.len() < ARTIFACT_MAGIC.len() || &raw[..ARTIFACT_MAGIC.len()] != ARTIFACT_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "not a hanzi dataset artifact (bad or outdated magic); re-run `prepare-data`",
            ));
        }
        let artifact: Artifact =
            postcard::from_bytes(&raw[ARTIFACT_MAGIC.len()..]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("corrupt hanzi dataset artifact: {e}"),
                )
            })?;
        Ok(Self::from_parts(artifact.characters, artifact.words))
    }


    pub fn get(&self, ch: char) -> Option<&Character> {
        self.index.get(&ch).map(|i| &self.chars[*i])
    }

    /// True when there is stroke geometry to grade an attempt against.
    pub fn is_practisable(&self, ch: char) -> bool {
        self.get(ch).is_some_and(|c| !c.medians.is_empty())
    }

    /// Every character the board can ask for, so the interface can tell a word
    /// from the punctuation around it instead of trying to draw a comma.
    pub fn practisable_characters(&self) -> Vec<char> {
        self.chars
            .iter()
            .filter(|c| !c.medians.is_empty())
            .map(|c| c.ch)
            .collect()
    }

    pub fn chars(&self) -> &[Character] {
        &self.chars
    }

    /// Characters that appear in the frequency list, most common first.
    pub fn ranked(&self) -> impl Iterator<Item = &Character> {
        self.chars.iter().filter(|c| c.is_teachable())
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    // ---- words -------------------------------------------------------------

    /// Every word, most useful first: by HSK level, then by derived frequency.
    pub fn words(&self) -> &[Word] {
        &self.words
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    /// Exact dictionary lookup for a word.
    pub fn word(&self, text: &str) -> Option<&Word> {
        self.word_index.get(text.trim()).map(|i| &self.words[*i])
    }

    /// How many words sit at each HSK level, lowest level first. Levels with no
    /// words are omitted rather than reported as zero.
    pub fn words_per_level(&self) -> Vec<(u8, usize)> {
        let mut counts: Vec<(u8, usize)> = Vec::new();
        for word in &self.words {
            match counts.last_mut() {
                Some((level, count)) if *level == word.hsk => *count += 1,
                _ => counts.push((word.hsk, 1)),
            }
        }
        counts
    }

    /// Every word containing a character, most useful first.
    ///
    /// This is the inverse index: "what can I read now that I know 学?"
    pub fn words_with(&self, ch: char) -> Vec<&Word> {
        self.words_by_char
            .get(&ch)
            .map(|indexes| indexes.iter().map(|i| &self.words[*i]).collect())
            .unwrap_or_default()
    }

    /// Find words by character, reading or meaning.
    ///
    /// An empty query browses from the most useful word down; anything else is
    /// matched against the word's characters, its reading (tone marks and
    /// spacing ignored, so `xuexi`, `xué xí` and `xüexi` all find 学习) and its
    /// English definition. Results are ranked so that an exact word beats a
    /// prefix, which beats a reading, which beats a definition; ties keep the
    /// browsing order, so the most common word comes first.
    ///
    /// `level` restricts the search to one HSK level. `limit` caps the result;
    /// the caller is told the true total separately by counting matches itself.
    pub fn search_words(&self, query: &str, level: Option<u8>, limit: usize) -> Vec<&Word> {
        let query = query.trim();
        let folded = fold_pinyin(query);
        let lowered = query.to_lowercase();

        let mut matches: Vec<(u8, usize)> = Vec::new();
        for (i, word) in self.words.iter().enumerate() {
            if level.is_some_and(|level| word.hsk != level) {
                continue;
            }
            match Self::match_rank(word, &self.word_keys[i], query, &folded, &lowered) {
                Some(rank) => matches.push((rank, i)),
                None => continue,
            }
        }

        matches.sort_by_key(|(rank, i)| (*rank, *i));
        matches
            .into_iter()
            .take(limit)
            .map(|(_, i)| &self.words[i])
            .collect()
    }

    /// How many words a query matches, ignoring any limit.
    ///
    /// Shares [`Self::match_rank`] with [`Self::search_words`], so the two can
    /// never disagree about what counts as a match.
    pub fn count_words(&self, query: &str, level: Option<u8>) -> usize {
        let query = query.trim();
        let folded = fold_pinyin(query);
        let lowered = query.to_lowercase();

        self.words
            .iter()
            .enumerate()
            .filter(|(_, word)| level.is_none_or(|level| word.hsk == level))
            .filter(|(i, word)| {
                Self::match_rank(word, &self.word_keys[*i], query, &folded, &lowered).is_some()
            })
            .count()
    }

    /// How well a word answers a query: lower is better, `None` is no match.
    ///
    /// A single character is treated as a browse-by-character request, so every
    /// word containing it is a hit — "what can I read now that I know 学?"
    fn match_rank(
        word: &Word,
        key: &WordKey,
        query: &str,
        folded: &str,
        lowered: &str,
    ) -> Option<u8> {
        if query.is_empty() {
            return Some(0);
        }
        if query.chars().count() == 1 && word.text.contains(query) {
            return Some(1);
        }
        if word.text == query {
            Some(0)
        } else if word.text.starts_with(query) {
            Some(1)
        } else if word.text.contains(query) {
            Some(2)
        } else if !folded.is_empty() && key.pinyin.starts_with(folded) {
            Some(3)
        } else if !folded.is_empty() && key.pinyin.contains(folded) {
            Some(4)
        } else if !lowered.is_empty() && key.meaning.contains(lowered) {
            Some(5)
        } else {
            None
        }
    }

    /// What the dataset can tell about a piece of study text before the user
    /// edits it.
    ///
    /// Single characters, dictionary words and everything else are treated
    /// differently, in that order of authority:
    ///
    /// * A **single character** is its own word — its reading and meaning *are*
    ///   the answer, so both are filled in, from the character's own entry.
    /// * A **word that is in the dictionary** gets that dictionary's reading and
    ///   meaning. This is a real reading of the whole word, so a polyphonic word
    ///   is right (着急 → `zháojí`) and the meaning is a definition rather than
    ///   a guess.
    /// * **Anything else** gets a composed reading, because word pinyin is the
    ///   characters' readings run together (学习 → `xuéxí`), and no meaning: a
    ///   word's meaning cannot be composed from its parts, and inventing one
    ///   would be worse than leaving it blank. The per-character hints are
    ///   returned so the learner has the material to write it.
    ///
    /// Tone changes are not applied to the composed case (`你好` composes to
    /// `nǐhǎo`, not `níhǎo`), so that result is a draft to be checked, not an
    /// authority. The per-character hints are returned either way.
    pub fn lookup_text(&self, text: &str) -> TextLookup {
        let characters: Vec<char> = text.trim().chars().collect();

        let hints: Vec<CharacterHint> = characters
            .iter()
            .map(|ch| match self.get(*ch) {
                Some(found) => CharacterHint {
                    ch: *ch,
                    pinyin: found.pinyin.clone(),
                    meaning: found.definition.clone(),
                },
                None => CharacterHint {
                    ch: *ch,
                    pinyin: Vec::new(),
                    meaning: String::new(),
                },
            })
            .collect();

        let complete = !hints.is_empty() && hints.iter().all(|h| !h.pinyin.is_empty());

        let (pinyin, meaning) = if hints.len() == 1 {
            let only = &hints[0];
            (
                only.pinyin.first().cloned().unwrap_or_default(),
                only.meaning.clone(),
            )
        } else if let Some(word) = self.word(text) {
            // A real dictionary entry: the word's own reading, and a meaning
            // that is a definition rather than an invention.
            (word.pinyin.clone(), word.meaning.clone())
        } else {
            // Readings run together, which is how word pinyin is written.
            let composed: String = hints
                .iter()
                .filter_map(|h| h.pinyin.first())
                .cloned()
                .collect();
            (composed, String::new())
        };

        TextLookup {
            pinyin,
            meaning,
            characters: hints,
            complete,
        }
    }
}

/// A rank that sorts most-common-first, with "unknown" last.
fn rank_key(rank: u32) -> u32 {
    if rank == 0 {
        u32::MAX
    } else {
        rank
    }
}

/// Fold a reading for searching: drop tone marks and spacing, lowercase, and
/// treat `v` and `ü` as the same letter so a learner can type `nv` for 女.
///
/// This is deliberately not a pinyin *generator* — it only has to make two
/// spellings of the same reading compare equal.
pub fn fold_pinyin(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        let folded = match ch {
            'ā' | 'á' | 'ǎ' | 'à' | 'a' | 'A' => 'a',
            'ē' | 'é' | 'ě' | 'è' | 'e' | 'E' | 'ê' | 'Ê' => 'e',
            'ī' | 'í' | 'ǐ' | 'ì' | 'i' | 'I' => 'i',
            'ō' | 'ó' | 'ǒ' | 'ò' | 'o' | 'O' => 'o',
            'ū' | 'ú' | 'ǔ' | 'ù' | 'u' | 'U' => 'u',
            'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' | 'ü' | 'Ü' | 'v' | 'V' => 'u',
            'ń' | 'ň' | 'ǹ' | 'n' | 'N' => 'n',
            'ḿ' | 'm' | 'M' => 'm',
            other => other,
        };
        if folded.is_ascii_alphanumeric() {
            out.push(folded.to_ascii_lowercase());
        }
    }
    out
}

/// A draft reading and meaning for some study text, plus what each character in
/// it means on its own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextLookup {
    /// Draft pinyin. Empty when nothing could be resolved.
    pub pinyin: String,
    /// The meaning, filled in only for a single character.
    pub meaning: String,
    /// One entry per character of the text, in order.
    pub characters: Vec<CharacterHint>,
    /// True when every character was found and has a reading.
    pub complete: bool,
}

/// What one character of the text contributes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterHint {
    pub ch: char,
    /// Every reading the dataset knows, most common first.
    pub pinyin: Vec<String>,
    pub meaning: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(ch: char, rank: u32, strokes: usize) -> Character {
        Character {
            ch,
            rank,
            hsk: 1,
            stroke_count: strokes as u8,
            radical: '一',
            pinyin: vec!["yī".into()],
            definition: "test".into(),
            etymology: String::new(),
            outlines: (0..strokes).map(|i| format!("M {i} 0 L {i} 100 Z")).collect(),
            medians: (0..strokes)
                .map(|i| vec![Point::new(i as f32, 0.0), Point::new(i as f32, 100.0)])
                .collect(),
        }
    }

    #[test]
    fn lookup_and_ranking() {
        let dataset = Dataset::from_chars(vec![
            character('一', 2, 1),
            character('的', 1, 8),
            character('⺀', 0, 2), // unranked, traditional-only radical
        ]);
        assert_eq!(dataset.len(), 3);
        assert_eq!(dataset.get('的').map(|c| c.rank), Some(1));

        let ranked: Vec<char> = dataset.ranked().map(|c| c.ch).collect();
        assert_eq!(ranked, vec!['的', '一'], "unranked characters are skipped");
    }

    #[test]
    fn artifact_rejects_bad_magic() {
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut encoder, b"NOTHANZI............").unwrap();
        let bytes = encoder.finish().unwrap();
        let err = Dataset::from_gzip_bytes(&bytes).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn artifact_round_trips() {
        let chars = vec![character('一', 2, 1), character('的', 1, 8)];
        let words = vec![word("学习", "xuéxí", "to study", 1, 40)];
        let mut raw = ARTIFACT_MAGIC.to_vec();
        raw.extend_from_slice(&postcard::to_allocvec(&Artifact::new(chars, words)).unwrap());
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut encoder, &raw).unwrap();
        let bytes = encoder.finish().unwrap();

        let dataset = Dataset::from_gzip_bytes(&bytes).unwrap();
        assert_eq!(dataset.len(), 2);
        assert_eq!(dataset.get('的').unwrap().stroke_count, 8);
        assert_eq!(dataset.get('一').unwrap().medians.len(), 1);
        assert_eq!(dataset.word_count(), 1);
        assert_eq!(dataset.word("学习").unwrap().pinyin, "xuéxí");
    }

    #[test]
    fn an_artifact_from_the_previous_format_is_rejected_loudly() {
        // Version 01 carried a bare `Vec<Character>`. Decoding it as the new
        // payload would silently produce nonsense, which is what the magic is
        // for.
        let mut raw = b"HANZID01".to_vec();
        raw.extend_from_slice(&postcard::to_allocvec(&vec![character('一', 1, 1)]).unwrap());
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut encoder, &raw).unwrap();
        let bytes = encoder.finish().unwrap();

        let err = Dataset::from_gzip_bytes(&bytes).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("re-run"), "{err}");
    }

    // ---- looking up study text --------------------------------------------

    /// A character with a chosen reading and meaning.
    fn speaker(ch: char, pinyin: &[&str], definition: &str) -> Character {
        Character {
            pinyin: pinyin.iter().map(|s| s.to_string()).collect(),
            definition: definition.to_string(),
            ..character(ch, 1, 1)
        }
    }

    #[test]
    fn a_single_character_lookup_fills_reading_and_meaning() {
        let dataset = Dataset::from_chars(vec![speaker('好', &["hǎo", "hào"], "good, well")]);
        let lookup = dataset.lookup_text("好");

        assert_eq!(lookup.pinyin, "hǎo", "the most common reading wins");
        assert_eq!(lookup.meaning, "good, well");
        assert!(lookup.complete);
        assert_eq!(lookup.characters.len(), 1);
        // Every reading is still reported, so the learner can pick another.
        assert_eq!(lookup.characters[0].pinyin, vec!["hǎo", "hào"]);
    }

    #[test]
    fn a_word_lookup_composes_the_reading_but_not_the_meaning() {
        let dataset = Dataset::from_chars(vec![
            speaker('学', &["xué"], "learning, knowledge; to study"),
            speaker('习', &["xí"], "to practise; habit"),
        ]);
        let lookup = dataset.lookup_text("学习");

        // Word pinyin is the readings run together, which is how it is written.
        assert_eq!(lookup.pinyin, "xuéxí");
        // A word's meaning cannot be composed from its characters, so it is left
        // for the learner rather than invented.
        assert_eq!(lookup.meaning, "");
        assert!(lookup.complete);
        assert_eq!(lookup.characters.len(), 2);
        assert_eq!(lookup.characters[0].ch, '学');
        assert_eq!(lookup.characters[1].meaning, "to practise; habit");
    }

    #[test]
    fn a_word_with_an_unknown_character_is_incomplete() {
        let dataset = Dataset::from_chars(vec![speaker('学', &["xué"], "to study")]);
        let lookup = dataset.lookup_text("学X");

        assert!(!lookup.complete, "X is not in the dataset");
        assert_eq!(lookup.pinyin, "xué", "the known part still contributes");
        assert_eq!(lookup.characters.len(), 2);
        assert!(lookup.characters[1].pinyin.is_empty());
        assert!(lookup.characters[1].meaning.is_empty());
    }

    #[test]
    fn an_empty_lookup_is_empty_rather_than_falsely_incomplete() {
        let dataset = Dataset::from_chars(vec![speaker('好', &["hǎo"], "good")]);
        for text in ["", "   "] {
            let lookup = dataset.lookup_text(text);
            assert_eq!(lookup.pinyin, "");
            assert_eq!(lookup.meaning, "");
            assert!(lookup.characters.is_empty());
            assert!(!lookup.complete);
        }
    }

    #[test]
    fn a_longer_word_composes_every_character() {
        let dataset = Dataset::from_chars(vec![
            speaker('中', &["zhōng"], "middle"),
            speaker('国', &["guó"], "country"),
            speaker('人', &["rén"], "person"),
        ]);
        let lookup = dataset.lookup_text("中国人");
        assert_eq!(lookup.pinyin, "zhōngguórén");
        assert_eq!(lookup.meaning, "");
        assert_eq!(lookup.characters.len(), 3);
    }

    #[test]
    fn lookup_ignores_surrounding_whitespace() {
        let dataset = Dataset::from_chars(vec![speaker('好', &["hǎo"], "good")]);
        let lookup = dataset.lookup_text("  好  ");
        assert_eq!(lookup.pinyin, "hǎo");
        assert_eq!(lookup.characters.len(), 1);
    }

    // ---- the word dictionary ----------------------------------------------

    /// A word, for tests that do not care how it was prepared.
    fn word(text: &str, pinyin: &str, meaning: &str, hsk: u8, rank: u32) -> Word {
        Word {
            text: text.to_string(),
            pinyin: pinyin.to_string(),
            meaning: meaning.to_string(),
            hsk,
            rank,
        }
    }

    /// The words of 学 and 习, plus a vocabulary of characters to spell them.
    fn word_dataset() -> Dataset {
        Dataset::from_parts(
            "学习生老老师汉语句子".chars().map(|ch| speaker(ch, &["x"], "x")).collect(),
            vec![
                word("学习", "xuéxí", "to study; to learn", 1, 30),
                word("学生", "xuéshēng", "student; pupil", 1, 40),
                word("老师", "lǎoshī", "teacher", 2, 60),
                word("汉语", "Hànyǔ", "Chinese language", 3, 80),
                word("句子", "jùzi", "sentence", 4, 900),
            ],
        )
    }

    #[test]
    fn a_word_lookup_prefers_the_dictionary_over_a_composed_reading() {
        // The whole point of M3: a word's real reading and meaning, so a
        // polyphonic word is right and the meaning is a definition.
        let dataset = word_dataset();
        let lookup = dataset.lookup_text("学习");

        assert_eq!(lookup.pinyin, "xuéxí");
        assert_eq!(lookup.meaning, "to study; to learn");
        assert!(lookup.complete);
        assert_eq!(lookup.characters.len(), 2);
    }

    #[test]
    fn an_unknown_word_still_composes_and_invents_no_meaning() {
        // 学老 is not a word; it must fall back to the composed draft rather
        // than borrowing a nearby entry's meaning.
        let dataset = word_dataset();
        let lookup = dataset.lookup_text("学老");
        assert_eq!(lookup.pinyin, "xx", "readings run together");
        assert_eq!(lookup.meaning, "", "no meaning is invented");
    }

    #[test]
    fn a_single_character_is_not_served_from_the_word_dictionary() {
        // Single characters come from the character dataset, so a word entry
        // can never give a glyph a second, competing reading.
        let base = word_dataset();
        let dataset = Dataset::from_parts(
            base.chars().to_vec(),
            vec![word("学", "xué", "to study", 1, 30)],
        );
        assert!(dataset.word("学").is_some(), "the dictionary does hold it");
        let lookup = dataset.lookup_text("学");
        assert_eq!(lookup.pinyin, "x", "but the character's own reading wins");
        assert_eq!(lookup.meaning, "x");
    }

    #[test]
    fn a_word_supplied_twice_is_kept_once_at_its_most_useful_level() {
        // Two entries for the same text are not adjacent after the sort, so a
        // naive `dedup_by` would keep both and the index would point at one of
        // them arbitrarily.
        let dataset = Dataset::from_parts(
            "学习".chars().map(|ch| speaker(ch, &["x"], "x")).collect(),
            vec![
                word("学习", "xuéxí", "to study", 4, 900),
                word("学习", "xuéxí", "to study", 1, 30),
            ],
        );
        assert_eq!(dataset.word_count(), 1);
        assert_eq!(dataset.word("学习").unwrap().hsk, 1, "the lower level wins");
        assert_eq!(dataset.words_with('学').len(), 1);
    }

    #[test]
    fn words_are_ordered_by_level_then_frequency() {
        let dataset = word_dataset();
        let order: Vec<&str> = dataset.words().iter().map(|w| w.text.as_str()).collect();
        assert_eq!(order, vec!["学习", "学生", "老师", "汉语", "句子"]);
    }

    #[test]
    fn every_word_containing_a_character_is_found_most_useful_first() {
        let dataset = word_dataset();
        let found: Vec<&str> = dataset.words_with('学').iter().map(|w| w.text.as_str()).collect();
        assert_eq!(found, vec!["学习", "学生"]);
        assert!(dataset.words_with('龙').is_empty());
    }

    #[test]
    fn words_can_be_searched_by_character_reading_or_meaning() {
        let dataset = word_dataset();

        // By character: 学 lists every word using it.
        let by_char: Vec<&str> = dataset
            .search_words("学", None, 10)
            .iter()
            .map(|w| w.text.as_str())
            .collect();
        assert_eq!(by_char, vec!["学习", "学生"]);

        // By reading, with and without tone marks, and with `v` for `ü`.
        for query in ["xuexi", "xuéxí", "xu"] {
            let hits = dataset.search_words(query, None, 10);
            assert!(
                hits.iter().any(|w| w.text == "学习"),
                "{query} should find 学习, got {:?}",
                hits.iter().map(|w| &w.text).collect::<Vec<_>>()
            );
        }
        assert!(dataset.search_words("nv", None, 10).is_empty(), "no ü words here");
        assert_eq!(fold_pinyin("nǚ'ér"), "nuer", "tones, ü and punctuation fold away");
        assert_eq!(fold_pinyin("Lǜsè"), "luse");

        // By meaning.
        let by_meaning = dataset.search_words("teacher", None, 10);
        assert_eq!(by_meaning.len(), 1);
        assert_eq!(by_meaning[0].text, "老师");
    }

    #[test]
    fn an_exact_word_comes_before_a_word_merely_containing_it() {
        let dataset = word_dataset();
        let hits = dataset.search_words("学生", None, 10);
        assert_eq!(hits[0].text, "学生");
    }

    #[test]
    fn an_empty_query_browses_and_a_level_filter_narrows() {
        let dataset = word_dataset();
        let all = dataset.search_words("", None, 100);
        assert_eq!(all.len(), 5);
        assert_eq!(all[0].text, "学习", "most useful first");

        let hsk1 = dataset.search_words("", Some(1), 100);
        assert_eq!(hsk1.len(), 2);
        assert!(hsk1.iter().all(|w| w.hsk == 1));
        assert_eq!(dataset.count_words("", Some(1)), 2);
    }

    #[test]
    fn a_search_is_capped_but_the_total_is_counted_honestly() {
        let dataset = word_dataset();
        assert_eq!(dataset.search_words("", None, 2).len(), 2);
        assert_eq!(dataset.count_words("", None), 5);
        assert_eq!(dataset.count_words("xue", None), 2);
        assert_eq!(dataset.count_words("nothing matches this", None), 0);
    }

    #[test]
    fn levels_with_no_words_are_omitted_from_the_census() {
        let dataset = word_dataset();
        assert_eq!(dataset.words_per_level(), vec![(1, 2), (2, 1), (3, 1), (4, 1)]);
    }

    #[test]
    fn a_word_knows_its_own_characters() {
        let dataset = word_dataset();
        let word = dataset.word("汉语").unwrap();
        assert_eq!(word.characters(), vec!['汉', '语']);
    }
}
