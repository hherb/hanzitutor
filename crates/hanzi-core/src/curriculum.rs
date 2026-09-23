//! Turning the frequency-ranked character list into a sequence of lessons.

use serde::{Deserialize, Serialize};

use crate::dataset::Dataset;

/// How many characters a lesson holds by default.
pub const DEFAULT_LESSON_SIZE: usize = 10;

/// A group of characters studied together.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    /// Zero-based position in the course.
    pub index: usize,
    /// Human-readable name, e.g. `"Characters 1-10"`.
    pub title: String,
    /// The characters to study, in teaching order.
    pub characters: Vec<char>,
    /// Lowest frequency rank in the lesson.
    pub first_rank: u32,
    /// Highest frequency rank in the lesson.
    pub last_rank: u32,
}

/// Split the frequency-ranked characters into consecutive lessons.
///
/// A character only appears if it has both a frequency rank and stroke
/// geometry, so every lesson entry is guaranteed to be practiceable.
pub fn build_lessons(dataset: &Dataset, lesson_size: usize) -> Vec<Lesson> {
    let size = lesson_size.max(1);
    let ranked: Vec<_> = dataset.ranked().collect();

    ranked
        .chunks(size)
        .enumerate()
        .map(|(index, chunk)| {
            let first_rank = chunk.first().map(|c| c.rank).unwrap_or(0);
            let last_rank = chunk.last().map(|c| c.rank).unwrap_or(0);
            Lesson {
                index,
                title: if first_rank == last_rank {
                    format!("Character {first_rank}")
                } else {
                    format!("Characters {first_rank}\u{2013}{last_rank}")
                },
                characters: chunk.iter().map(|c| c.ch).collect(),
                first_rank,
                last_rank,
            }
        })
        .collect()
}

/// The characters of one lesson, falling back to the first lesson when the
/// requested index is past the end.
pub fn lesson_at(dataset: &Dataset, lesson_size: usize, index: usize) -> Option<Lesson> {
    build_lessons(dataset, lesson_size).into_iter().nth(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::Character;
    use crate::geom::Point;

    fn character(ch: char, rank: u32) -> Character {
        Character {
            ch,
            rank,
            hsk: 1,
            stroke_count: 1,
            radical: '一',
            pinyin: vec![],
            definition: String::new(),
            etymology: String::new(),
            decomposition: String::new(),
            outlines: vec!["M 0 0 L 1 1 Z".into()],
            medians: vec![vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]],
        }
    }

    #[test]
    fn lessons_chunk_the_ranked_list_in_order() {
        let chars: Vec<Character> = (1..=25)
            .map(|i| character(char::from_u32(0x4E00 + i).unwrap(), i))
            .collect();
        let dataset = Dataset::from_chars(chars);
        let lessons = build_lessons(&dataset, 10);

        assert_eq!(lessons.len(), 3);
        assert_eq!(lessons[0].characters.len(), 10);
        assert_eq!(lessons[1].characters.len(), 10);
        assert_eq!(lessons[2].characters.len(), 5);
        assert_eq!(lessons[0].first_rank, 1);
        assert_eq!(lessons[0].last_rank, 10);
        assert_eq!(lessons[0].title, "Characters 1\u{2013}10");
        assert_eq!(lessons[2].title, "Characters 21\u{2013}25");
    }

    #[test]
    fn unranked_characters_are_excluded() {
        let dataset = Dataset::from_chars(vec![character('一', 1), character('⺀', 0)]);
        let lessons = build_lessons(&dataset, 10);
        assert_eq!(lessons.len(), 1);
        assert_eq!(lessons[0].characters, vec!['一']);
    }

    #[test]
    fn single_character_lesson_title_is_singular() {
        let dataset = Dataset::from_chars(vec![character('一', 7)]);
        let lessons = build_lessons(&dataset, 10);
        assert_eq!(lessons[0].title, "Character 7");
    }
}
