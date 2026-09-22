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

use std::collections::{BTreeMap, HashMap, HashSet};
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
    /// The same for characters, in the same order as `chars`.
    character_keys: Vec<CharacterKey>,
}

/// A character's precomputed, comparison-ready text.
///
/// The readings are kept **one per entry** rather than folded into one string:
/// a character with several readings has to be able to match any of them
/// exactly, and `学` reading `xué` must come before `雪` reading `xuě` when
/// `xue` is typed, not after it because the two were concatenated.
#[derive(Clone, Debug, Default)]
struct CharacterKey {
    /// Every reading, folded the same way as [`fold_pinyin`].
    pinyin: Vec<String>,
    /// Definition lowercased.
    meaning: String,
}

/// A character query, prepared once and then compared against every character.
///
/// The same work is needed by a search and by a count of its matches, and the
/// two have to agree about what matches — so the preparation lives here rather
/// than being repeated, which is exactly how the two could drift apart.
#[derive(Clone, Debug, Default)]
struct CharacterQuery {
    /// The query as typed, trimmed.
    text: String,
    /// Readings folded: tone marks and spacing gone, `v` and `ü` unified.
    folded: String,
    /// The query lowercased, for matching definitions.
    lowered: String,
    /// The query's syllables, when it has more than one. Empty for a single
    /// syllable or a query that cannot be read as pinyin at all.
    syllables: Vec<String>,
}

impl CharacterQuery {
    fn new(query: &str) -> Self {
        let text = query.trim().to_string();
        let folded = fold_pinyin(&text);
        // Only a query that is more than one syllable gains anything from being
        // taken apart: one syllable is already asked as a reading, and the exact
        // and prefix rules answer it.
        let syllables = if folded.chars().count() > 1 {
            let parts: Vec<String> = crate::pinyin::syllables(&text)
                .into_iter()
                .flatten()
                .map(|syllable| fold_pinyin(&syllable.text))
                .filter(|syllable| !syllable.is_empty())
                .collect();
            if parts.len() > 1 {
                parts
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        Self {
            lowered: text.to_lowercase(),
            text,
            folded,
            syllables,
        }
    }
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

        let character_keys = chars
            .iter()
            .map(|c| CharacterKey {
                pinyin: c.pinyin.iter().map(|r| fold_pinyin(r)).collect(),
                meaning: c.definition.to_lowercase(),
            })
            .collect();

        Self {
            chars,
            index,
            words,
            word_index,
            words_by_char,
            word_keys,
            character_keys,
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

    // ---- the character dictionary -------------------------------------------

    /// Find characters by character, reading or meaning.
    ///
    /// This is the way out of the course's linear order. The course walks the
    /// frequency list one lesson at a time, which is the right way to *learn*
    /// and the wrong way to *look something up*: a learner who meets 医院 on a
    /// sign, or who wants every character that means "water", has no route to it.
    ///
    /// An empty query browses: every character the course can teach, most common
    /// first. Anything else is matched against the character itself, its
    /// readings (tone marks and spacing ignored, so `xue`, `xué` and `xüe` all
    /// find 学) and its English definition. Two shorthands follow from what a
    /// learner has in hand rather than from the data: several **characters**
    /// typed at once each match (`医院` offers both 医 and 院), and several
    /// **syllables** typed at once each match (`yisheng` does the same through
    /// the readings) — because in both cases what was typed is a word, and a
    /// word is the characters it is made of.
    ///
    /// Results are ranked so that the character itself beats an exact reading,
    /// which beats a reading prefix, which beats a reading that merely contains
    /// the query, which beats a definition, which beats a syllable of a longer
    /// reading; ties keep frequency order, so the more common character comes
    /// first. The definition is deliberately above the syllable rule: an English
    /// word can segment into pinyin syllables by accident (`banana` is
    /// `ba`-`na`-`na`), and a match on what was typed must not be buried under a
    /// decomposition of it. A character the course cannot teach — no frequency
    /// rank, so it is in no lesson — is still found by text, and the caller is
    /// told [`Character::is_teachable`], because finding a character and being
    /// able to drill it are different answers.
    ///
    /// `level` restricts the search to one HSK level. `limit` caps the result;
    /// the caller is told the true total separately by [`Self::count_characters`].
    pub fn search_characters(&self, query: &str, level: Option<u8>, limit: usize) -> Vec<&Character> {
        let query = CharacterQuery::new(query);

        let mut matches: Vec<(u8, usize)> = Vec::new();
        for (i, ch) in self.chars.iter().enumerate() {
            if level.is_some_and(|level| ch.hsk != level) {
                continue;
            }
            match Self::match_character_rank(ch, &self.character_keys[i], &query) {
                Some(rank) => matches.push((rank, i)),
                None => continue,
            }
        }

        matches.sort_by_key(|(rank, i)| (*rank, *i));
        matches
            .into_iter()
            .take(limit)
            .map(|(_, i)| &self.chars[i])
            .collect()
    }

    /// How many characters a query matches, ignoring any limit.
    ///
    /// Shares [`Self::match_character_rank`] with [`Self::search_characters`],
    /// so the two can never disagree about what counts as a match — which is why
    /// the query is prepared in one place too.
    pub fn count_characters(&self, query: &str, level: Option<u8>) -> usize {
        let query = CharacterQuery::new(query);

        self.chars
            .iter()
            .enumerate()
            .filter(|(_, ch)| level.is_none_or(|level| ch.hsk == level))
            .filter(|(i, ch)| Self::match_character_rank(ch, &self.character_keys[*i], &query).is_some())
            .count()
    }

    /// How well a character answers a query: lower is better, `None` is no match.
    fn match_character_rank(
        ch: &Character,
        key: &CharacterKey,
        query: &CharacterQuery,
    ) -> Option<u8> {
        if query.text.is_empty() {
            // Browsing is the course's own list, so it lists what the course can
            // teach: an unranked character is in no lesson and has no place in a
            // frequency-ordered browse, and is still found by text below.
            return ch.is_teachable().then_some(0);
        }
        if query.text.chars().count() == 1 && query.text.starts_with(ch.ch) {
            return Some(0);
        }
        if !query.folded.is_empty() {
            if key.pinyin.iter().any(|reading| reading == &query.folded) {
                return Some(1);
            }
            if key.pinyin.iter().any(|reading| reading.starts_with(&query.folded)) {
                return Some(2);
            }
            if key.pinyin.iter().any(|reading| reading.contains(&query.folded)) {
                return Some(3);
            }
        }
        // A definition comes before the syllable rule below, and that order is the
        // answer to an English query that happens to segment into pinyin: `banana`
        // is `ba`-`na`-`na` by the segmenter's rules, so ranking the syllables
        // first would bury 香蕉 under every character that reads `ba`. A direct
        // match on what was typed outranks a decomposition of it.
        if !query.lowered.is_empty() && key.meaning.contains(&query.lowered) {
            return Some(4);
        }
        // Several syllables typed at once: a word's reading, and each syllable is
        // one of its characters. Ranked below a whole-reading match, so `xuexi`
        // never buries 学 behind a character that merely reads `xi`.
        if query
            .syllables
            .iter()
            .any(|syllable| key.pinyin.iter().any(|reading| reading == syllable))
        {
            return Some(5);
        }
        // Several characters typed at once: each character is a hit, which is how
        // a word the learner met somewhere becomes two characters they can drill.
        if query.text.chars().count() > 1 && query.text.contains(ch.ch) {
            return Some(6);
        }
        None
    }

    /// How many characters the course teaches at each HSK level, lowest level
    /// first. Levels with no characters are omitted rather than reported as zero.
    ///
    /// Only teachable characters are counted, because those are the ones a
    /// `search_characters` browse with that level filter will list — a census
    /// that disagrees with the list it labels is worse than none. Levels are
    /// counted into a map rather than off the end of the previous run: a
    /// character's HSK level has nothing to do with its frequency rank, so the
    /// characters are **not** grouped by level in storage.
    pub fn characters_per_level(&self) -> Vec<(u8, usize)> {
        let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
        for ch in self.ranked() {
            if ch.hsk > 0 {
                *counts.entry(ch.hsk).or_default() += 1;
            }
        }
        counts.into_iter().collect()
    }

    // ---- tone pairs ---------------------------------------------------------

    /// Sets of characters that differ only in tone, most useful first.
    ///
    /// This is the material a tone drill needs and the dataset cannot state
    /// directly: the characters sharing one syllable at different tones are a
    /// **derived** fact, because a character's readings are stored on the character
    /// and nothing groups them by the syllable underneath. Deriving it here rather
    /// than in the interface is what makes it testable, and it is one answer for
    /// every screen that will ever want it.
    ///
    /// Four rules, each a decision:
    ///
    /// * **The course's characters only** ([`Character::is_teachable`]). A pair is
    ///   something to hear *and* to write, and a character in no lesson can be
    ///   neither browsed nor drilled with the course behind it.
    /// * **Only the reading the voice will say.** A member is a character spoken on
    ///   its own, so it is placed by its *first* reading — the one a synthesiser
    ///   says for the glyph by itself. Building a set from a secondary reading would
    ///   play the wrong syllable and the wrong tone: 行 is `xíng` alone, so it may
    ///   appear in a `xing` set and never in a `hang` one, however tempting its
    ///   `háng` reading is. This is also what makes one character one member: a
    ///   character cannot place two tones of a syllable, so `好` (`hǎo`, `hào`)
    ///   never forms a "pair" with itself.
    /// * **A set needs two tones.** One character is not a pair.
    /// * **Ranked by the rarest member.** Four common characters are more use than
    ///   a pair containing an obscure one, and it is the member a learner is least
    ///   likely to know that decides whether the set is worth their time.
    ///
    /// The tone comes from [`crate::pinyin::tone_from_pinyin`] and the syllable from
    /// [`crate::pinyin::base`] — the same two functions the tone scorer uses, so a
    /// set cannot be grouped by one rule and scored by another. **The syllable is
    /// `base` and not [`fold_pinyin`]**: folding is for *searching*, where typing
    /// `nu` should find 女, and it merges `ü` onto `u`. A minimal pair differs only
    /// in tone, and 奴 `nú` against 女 `nǚ` differs in the vowel — a set of those two
    /// would drill the wrong contrast. **The neutral tone is not a member** either:
    /// contrasting a full tone with a neutral one is a real exercise, but a neutral
    /// syllable's pitch is set by the syllable before it and cannot be drilled from
    /// one character on its own.
    pub fn tone_sets(&self, limit: usize) -> Vec<ToneSet> {
        // (syllable, tone) -> the best character for it. A `BTreeMap` so the
        // syllables come out in one fixed order, which is what lets the grouping
        // below be a run-length pass rather than a second map.
        let mut members: BTreeMap<(String, u8), ToneSetMember> = BTreeMap::new();

        for character in self.ranked() {
            let Some(reading) = character.pinyin.first() else {
                continue;
            };
            let base = crate::pinyin::base(reading);
            let Some(tone) = crate::pinyin::tone_from_pinyin(reading) else {
                continue;
            };
            if base.is_empty() || !(1..=4).contains(&tone) {
                continue;
            }
            // `ranked()` is frequency order, so the first character to reach a slot
            // is the most common one for it, and a slot already filled is not
            // improved by anything later.
            members.entry((base, tone)).or_insert(ToneSetMember {
                ch: character.ch,
                reading: reading.clone(),
                tone,
                definition: character.definition.clone(),
                rank: character.rank,
            });
        }

        let mut sets: Vec<ToneSet> = Vec::new();
        for ((base, _), member) in members {
            match sets.last_mut() {
                Some(set) if set.base == base => set.members.push(member),
                _ => sets.push(ToneSet {
                    base,
                    members: vec![member],
                }),
            }
        }

        sets.retain(|set| set.members.len() >= 2);
        for set in &mut sets {
            set.members.sort_by_key(|member| member.tone);
        }
        sets.sort_by(|a, b| rarest(a).cmp(&rarest(b)).then_with(|| a.base.cmp(&b.base)));
        sets.truncate(limit);
        sets
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

/// The rank of a tone set's least common member: the number that decides whether
/// the set is worth a learner's time.
fn rarest(set: &ToneSet) -> u32 {
    set.members
        .iter()
        .map(|member| rank_key(member.rank))
        .max()
        .unwrap_or(u32::MAX)
}

/// One character of a tone set: a syllable read at one tone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneSetMember {
    pub ch: char,
    /// The reading with its tone mark, e.g. `"mā"` — the reading that put this
    /// character in this set, which for a polyphonic character is not necessarily
    /// its first.
    pub reading: String,
    /// The tone of that reading, `1..=4`.
    pub tone: u8,
    pub definition: String,
    /// The character's frequency rank. Used for ordering, and shown beside it.
    pub rank: u32,
}

/// Characters that differ only in tone: one syllable, one character per tone.
///
/// `ToneSet` is derived by [`Dataset::tone_sets`] rather than stored, because it is
/// a grouping of data the dataset holds one character at a time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneSet {
    /// The syllable with tone marks stripped, e.g. `"ma"`. **`ü` stays distinct
    /// from `u`** — `nv` and `nu` are different syllables, not different tones.
    pub base: String,
    /// The members, tone 1 first. Two or more by construction.
    pub members: Vec<ToneSetMember>,
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

    // ---- the character dictionary -----------------------------------------

    /// A character with a chosen reading, meaning, level and rank.
    fn lexeme(ch: char, pinyin: &[&str], definition: &str, hsk: u8, rank: u32) -> Character {
        Character {
            rank,
            hsk,
            ..speaker(ch, pinyin, definition)
        }
    }

    /// Four characters spanning a reading, a meaning and two HSK levels, stored
    /// deliberately out of level order — by rank, as the real artifact is.
    fn character_dataset() -> Dataset {
        Dataset::from_chars(vec![
            lexeme('学', &["xué"], "to study", 1, 10),
            lexeme('雪', &["xuě"], "snow", 2, 20),
            lexeme('血', &["xuè", "xiě"], "blood", 3, 30),
            lexeme('习', &["xí"], "to practise; habit", 1, 40),
        ])
    }

    #[test]
    fn characters_can_be_searched_by_character_reading_or_meaning() {
        let dataset = character_dataset();

        // By the character itself.
        let exact = dataset.search_characters("雪", None, 10);
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].ch, '雪');

        // By reading, with tone marks, without them, and with `v` for `ü`.
        for query in ["xue", "xué", "XUE"] {
            let hits = dataset.search_characters(query, None, 10);
            assert!(
                hits.iter().any(|c| c.ch == '学'),
                "{query} should find 学, got {:?}",
                hits.iter().map(|c| c.ch).collect::<Vec<_>>()
            );
        }

        // By meaning.
        let by_meaning = dataset.search_characters("snow", None, 10);
        assert_eq!(by_meaning.len(), 1);
        assert_eq!(by_meaning[0].ch, '雪');

        // Every reading of a polyphonic character is searchable, not only the
        // first: 血 is xuè before xiě, and either has to find it.
        for reading in ["xue", "xie"] {
            assert!(
                dataset
                    .search_characters(reading, None, 10)
                    .iter()
                    .any(|c| c.ch == '血'),
                "{reading} should find 血"
            );
        }
        assert_eq!(fold_pinyin("nǚ"), "nu", "ü folds like v");
    }

    #[test]
    fn an_exact_reading_comes_before_a_longer_one_containing_it() {
        let dataset = character_dataset();

        // 学 reads xué and nothing else, so `xue` is its exact reading; 想 reads
        // xiǎng and only starts with `xi`. A prefix must never outrank an exact
        // match, whatever the frequency says.
        let exact: Vec<char> = dataset
            .search_characters("xue", None, 10)
            .iter()
            .map(|c| c.ch)
            .collect();
        assert_eq!(exact, vec!['学', '雪', '血'], "exact readings, by frequency");

        // 习 reads xí (exact) while 血 reads xiě (a prefix) and 学 reads xué (no
        // match at all) — so the exact match comes first even though 血 is the
        // more common character.
        let mixed: Vec<char> = dataset
            .search_characters("xi", None, 10)
            .iter()
            .map(|c| c.ch)
            .collect();
        assert_eq!(mixed, vec!['习', '血']);
    }

    #[test]
    fn several_characters_typed_at_once_find_each_of_them() {
        // A word met somewhere is also a way in: 医院 has to offer both halves.
        let dataset = Dataset::from_parts(
            vec![
                lexeme('医', &["yī"], "doctor", 1, 5),
                lexeme('院', &["yuàn"], "courtyard; school", 1, 8),
                lexeme('雪', &["xuě"], "snow", 2, 20),
            ],
            Vec::new(),
        );
        let hits: Vec<char> = dataset
            .search_characters("医院", None, 10)
            .iter()
            .map(|c| c.ch)
            .collect();
        assert_eq!(hits, vec!['医', '院'], "in frequency order");
    }

    #[test]
    fn several_syllables_typed_at_once_find_each_of_them() {
        // The reading half of the same shorthand: a learner who knows how a word
        // sounds, but not how it is written, types `yisheng` and gets 医 and 生.
        let dataset = Dataset::from_chars(vec![
            lexeme('医', &["yī"], "doctor", 1, 5),
            lexeme('生', &["shēng"], "life; to be born", 1, 6),
            lexeme('学', &["xué"], "to study", 1, 10),
            lexeme('血', &["xuè", "xiě"], "blood", 3, 30),
            lexeme('习', &["xí"], "to practise; habit", 1, 40),
        ]);

        let word: Vec<char> = dataset
            .search_characters("yisheng", None, 10)
            .iter()
            .map(|c| c.ch)
            .collect();
        assert_eq!(word, vec!['医', '生'], "in frequency order");

        // `xuexi` is 学 + 习, and 血 rides along on the same syllable.
        let xuexi: Vec<char> = dataset
            .search_characters("xuexi", None, 10)
            .iter()
            .map(|c| c.ch)
            .collect();
        assert_eq!(xuexi, vec!['学', '血', '习']);

        // A whole-reading match still outranks a syllable of one: `xi` is what 习
        // reads, and it must come before 血, which merely starts with it.
        let single = dataset.search_characters("xi", None, 10);
        assert_eq!(single[0].ch, '习', "exact reading first");
        assert_eq!(single[1].ch, '血', "then the prefix");
    }

    #[test]
    fn an_english_word_that_segments_into_pinyin_still_finds_its_meaning_first() {
        // `banana` is `ba`-`na`-`na` to the syllable splitter, so 吧 and 那 match
        // it by reading. The character that *means* banana has to come first
        // anyway: a definition match is a match on what was typed, and the
        // syllable rule is a guess about a different language.
        let dataset = Dataset::from_chars(vec![
            lexeme('吧', &["ba"], "modal particle", 1, 20),
            lexeme('那', &["nà"], "that; those", 1, 30),
            lexeme('蕉', &["jiāo"], "banana", 2, 900),
        ]);
        let hits = dataset.search_characters("banana", None, 10);
        assert_eq!(hits[0].ch, '蕉', "the meaning wins");
        assert!(
            hits.iter().any(|c| c.ch == '吧'),
            "and the reading hits are still there, below it"
        );
    }

    #[test]
    fn an_unranked_character_is_found_by_text_but_not_by_browsing() {
        // No rank means no lesson, so it must not appear in a browse of the
        // course — but searching for it by name has to find it, and say so.
        let dataset = Dataset::from_chars(vec![
            lexeme('学', &["xué"], "to study", 1, 10),
            lexeme('龘', &["dá"], "dragons flying", 0, 0),
        ]);
        let browsed: Vec<char> = dataset
            .search_characters("", None, 100)
            .iter()
            .map(|c| c.ch)
            .collect();
        assert_eq!(browsed, vec!['学'], "the unranked one is not in the course");

        let found = dataset.search_characters("龘", None, 10);
        assert_eq!(found.len(), 1);
        assert!(!found[0].is_teachable(), "found, but not drillable in course");

        // And it is reachable by its reading too.
        assert_eq!(dataset.search_characters("da", None, 10)[0].ch, '龘');
    }

    #[test]
    fn a_character_browse_is_most_common_first_and_a_level_filter_narrows() {
        let dataset = character_dataset();
        let all: Vec<char> = dataset
            .search_characters("", None, 100)
            .iter()
            .map(|c| c.ch)
            .collect();
        assert_eq!(all, vec!['学', '雪', '血', '习'], "rank order");

        let hsk1 = dataset.search_characters("", Some(1), 100);
        assert_eq!(hsk1.len(), 2);
        assert!(hsk1.iter().all(|c| c.hsk == 1));
        assert_eq!(dataset.count_characters("", Some(1)), 2);
        assert_eq!(dataset.count_characters("snow", Some(2)), 1);
        assert_eq!(dataset.count_characters("snow", Some(1)), 0);
    }

    #[test]
    fn the_level_census_counts_characters_that_are_not_stored_level_by_level() {
        // The regression this guards: the four characters are stored in *rank*
        // order, so levels 1, 2, 3, 1 interleave. Counting runs off the end of
        // the previous run — which is what the word census does, safely, because
        // words are stored by level — would report level 1 twice.
        let dataset = character_dataset();
        assert_eq!(dataset.characters_per_level(), vec![(1, 2), (2, 1), (3, 1)]);

        // The census has to agree with what browsing that level lists, or the
        // count beside the filter is a lie.
        for (level, count) in dataset.characters_per_level() {
            assert_eq!(dataset.count_characters("", Some(level)), count);
        }
    }

    #[test]
    fn a_character_search_is_capped_but_the_total_is_counted_honestly() {
        let dataset = character_dataset();
        assert_eq!(dataset.search_characters("", None, 2).len(), 2);
        assert_eq!(dataset.count_characters("", None), 4);
        assert_eq!(dataset.count_characters("xue", None), 3, "学, 雪 and 血");
        assert_eq!(dataset.count_characters("nothing matches this", None), 0);
        assert!(
            dataset.search_characters("blood", None, 10).len() == 1,
            "a definition still matches"
        );
    }

    // ---- tone pairs -------------------------------------------------------

    /// The syllable a set is built on, and its tones in order.
    fn shape(set: &ToneSet) -> (String, Vec<(char, u8)>) {
        (
            set.base.clone(),
            set.members.iter().map(|m| (m.ch, m.tone)).collect(),
        )
    }

    #[test]
    fn a_syllable_read_at_several_tones_becomes_one_set_in_tone_order() {
        // 妈麻马骂: `ma` at all four tones. The set is derived, not stored.
        let dataset = Dataset::from_chars(vec![
            lexeme('妈', &["mā"], "mother", 1, 100),
            lexeme('麻', &["má"], "hemp", 3, 300),
            lexeme('马', &["mǎ"], "horse", 1, 200),
            lexeme('骂', &["mà"], "to scold", 4, 800),
        ]);
        let sets = dataset.tone_sets(10);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].base, "ma");
        assert_eq!(
            shape(&sets[0]).1,
            vec![('妈', 1), ('麻', 2), ('马', 3), ('骂', 4)],
            "members come out in tone order, with the reading that placed them"
        );
        assert_eq!(sets[0].members[1].reading, "má");
        assert_eq!(sets[0].members[0].definition, "mother");
    }

    #[test]
    fn a_tone_set_uses_only_the_reading_the_voice_would_say() {
        // 行 is `xíng` on its own. A `hang` set listing it would *play* `xíng` —
        // the wrong syllable and the wrong tone — so a secondary reading may not
        // place a character, however common that character is.
        let dataset = Dataset::from_chars(vec![
            lexeme('行', &["xíng", "háng"], "to walk; a row", 1, 30),
            lexeme('航', &["háng"], "to sail", 3, 700),
            lexeme('星', &["xīng"], "star", 1, 500),
        ]);
        let sets = dataset.tone_sets(10);
        let bases: Vec<&str> = sets.iter().map(|set| set.base.as_str()).collect();
        assert_eq!(bases, vec!["xing"], "hang has only one usable character");
        assert_eq!(shape(&sets[0]).1, vec![('星', 1), ('行', 2)]);
    }

    #[test]
    fn a_polyphonic_character_fills_only_one_tone_of_its_syllable() {
        // 好 is hǎo and hào: the best candidate for two tones of `hao`, and a
        // "pair" of one character would teach nothing. Its first reading claims it.
        let dataset = Dataset::from_chars(vec![
            lexeme('好', &["hǎo", "hào"], "good; to like", 1, 50),
            lexeme('号', &["hào"], "number", 1, 400),
        ]);
        let sets = dataset.tone_sets(10);
        assert_eq!(sets.len(), 1);
        assert_eq!(
            shape(&sets[0]).1,
            vec![('好', 3), ('号', 4)],
            "好 takes the reading it is listed with first; tone 4 needs another"
        );

        // With no second character for tone 4, the set is only the one member and
        // is not a pair at all.
        let alone = Dataset::from_chars(vec![lexeme('好', &["hǎo", "hào"], "good", 1, 50)]);
        assert!(alone.tone_sets(10).is_empty());
    }

    #[test]
    fn a_character_in_no_lesson_is_not_in_a_tone_set() {
        // A pair is something to hear and to write, and the course behind it is
        // what makes both possible.
        let dataset = Dataset::from_chars(vec![
            lexeme('妈', &["mā"], "mother", 1, 100),
            lexeme('麻', &["má"], "hemp", 0, 0),
            lexeme('马', &["mǎ"], "horse", 1, 0),
        ]);
        assert!(
            dataset.tone_sets(10).is_empty(),
            "an unranked or unlistable character is not a member"
        );
    }

    #[test]
    fn the_neutral_tone_is_not_a_member() {
        // 的 is `de` with no tone mark, and a neutral syllable's pitch is set by
        // the syllable before it — nothing to drill from one character.
        let dataset = Dataset::from_chars(vec![
            lexeme('得', &["dé"], "to obtain", 1, 100),
            lexeme('的', &["de"], "possessive", 1, 1),
        ]);
        assert!(
            dataset.tone_sets(10).is_empty(),
            "a toneless reading must not be given a tone"
        );
    }

    #[test]
    fn two_syllables_that_differ_in_a_vowel_are_not_a_tone_set() {
        // 奴 nú against 女 nǚ. Folding `ü` onto `u` — which searching does on
        // purpose, so that `nu` finds 女 — would make one set of these two, and the
        // contrast they drill would be the vowel rather than the tone.
        let dataset = Dataset::from_chars(vec![
            lexeme('奴', &["nú"], "slave", 3, 500),
            lexeme('女', &["nǚ"], "woman", 1, 300),
        ]);
        assert!(dataset.tone_sets(10).is_empty());

        // The same two readings do differ in tone when the syllable is the same.
        let dataset = Dataset::from_chars(vec![
            lexeme('女', &["nǚ"], "woman", 1, 300),
            lexeme('衄', &["nǜ"], "nosebleed", 0, 900),
        ]);
        let sets = dataset.tone_sets(10);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].base, "nv");
    }

    #[test]
    fn tone_sets_are_ranked_by_their_rarest_member() {
        // `shi` has two common characters; `mao` has one common and one rare. The
        // set a learner is most likely to recognise comes first.
        let dataset = Dataset::from_chars(vec![
            lexeme('是', &["shì"], "to be", 1, 5),
            lexeme('十', &["shí"], "ten", 1, 12),
            lexeme('猫', &["māo"], "cat", 1, 900),
            lexeme('毛', &["máo"], "hair", 1, 1_100),
        ]);
        let bases: Vec<String> = dataset.tone_sets(10).into_iter().map(|s| s.base).collect();
        assert_eq!(bases, vec!["shi", "mao"]);
    }

    #[test]
    fn a_tone_set_is_capped_but_derived_from_every_character() {
        let dataset = Dataset::from_chars(vec![
            lexeme('妈', &["mā"], "mother", 1, 100),
            lexeme('麻', &["má"], "hemp", 3, 300),
            lexeme('猫', &["māo"], "cat", 1, 900),
            lexeme('毛', &["máo"], "hair", 1, 1_100),
        ]);
        assert_eq!(dataset.tone_sets(1).len(), 1);
        assert_eq!(
            dataset.tone_sets(1)[0].base,
            "ma",
            "the cap takes the best sets, not the first ones found"
        );
        assert_eq!(dataset.tone_sets(10).len(), 2);
    }
}
