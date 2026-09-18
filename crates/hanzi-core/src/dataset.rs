//! The character dataset: stroke geometry plus lexical metadata.
//!
//! The dataset is built offline by the `prepare-data` binary from two upstream
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
//!
//! See `LICENSES.md` for the full notices that must accompany redistribution.

use std::collections::HashMap;
use std::io::Read;

use serde::{Deserialize, Serialize};

use crate::geom::Point;

/// Magic bytes and format version at the head of an uncompressed artifact.
pub const ARTIFACT_MAGIC: &[u8; 8] = b"HANZID01";

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

/// An indexed collection of characters, ordered by frequency rank.
#[derive(Clone, Debug, Default)]
pub struct Dataset {
    chars: Vec<Character>,
    index: HashMap<char, usize>,
}

impl Dataset {
    /// Build a dataset from characters, sorting them into frequency order.
    ///
    /// The order is normalised here rather than trusted, so [`Dataset::ranked`]
    /// really does yield most-common-first regardless of how the caller
    /// supplied the characters. Unranked characters sort last, then by
    /// codepoint, so the result is deterministic.
    pub fn from_chars(mut chars: Vec<Character>) -> Self {
        chars.sort_by_key(|c| {
            let rank = if c.rank == 0 { u32::MAX } else { c.rank };
            (rank, c.ch as u32)
        });
        let index = chars
            .iter()
            .enumerate()
            .map(|(i, c)| (c.ch, i))
            .collect();
        Self { chars, index }
    }

    /// Decode an artifact produced by `prepare-data`: gzip-compressed, magic
    /// prefixed, then a `postcard`-encoded `Vec<Character>`.
    pub fn from_gzip_bytes(bytes: &[u8]) -> std::io::Result<Self> {
        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut raw = Vec::new();
        decoder.read_to_end(&mut raw)?;

        if raw.len() < ARTIFACT_MAGIC.len() || &raw[..ARTIFACT_MAGIC.len()] != ARTIFACT_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "not a hanzi dataset artifact (bad magic); re-run `prepare-data`",
            ));
        }
        let chars: Vec<Character> = postcard::from_bytes(&raw[ARTIFACT_MAGIC.len()..]).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("corrupt hanzi dataset artifact: {e}"),
            )
        })?;
        Ok(Self::from_chars(chars))
    }

    pub fn get(&self, ch: char) -> Option<&Character> {
        self.index.get(&ch).map(|i| &self.chars[*i])
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
        let mut raw = ARTIFACT_MAGIC.to_vec();
        raw.extend_from_slice(&postcard::to_allocvec(&chars).unwrap());
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut encoder, &raw).unwrap();
        let bytes = encoder.finish().unwrap();

        let dataset = Dataset::from_gzip_bytes(&bytes).unwrap();
        assert_eq!(dataset.len(), 2);
        assert_eq!(dataset.get('的').unwrap().stroke_count, 8);
        assert_eq!(dataset.get('一').unwrap().medians.len(), 1);
    }
}
