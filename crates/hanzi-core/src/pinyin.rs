//! Pinyin readings: splitting a word's reading into syllables, and the tone each
//! one carries.
//!
//! ## Why this exists
//!
//! The dataset stores a **character's** reading as a list (`好` → `["hǎo",
//! "hào"]`) but a **word's** reading as one run-together string (`学习` →
//! `"xuéxí"`). Scoring a word's tones needs that string taken apart, and needs to
//! know how many syllables it holds so that the recording can be divided the same
//! way.
//!
//! Tone practice needs one more thing that a dictionary does not give:
//! **sandhi**. The dictionary reads 你好 as `nǐhǎo`, tone 3 + tone 3, but nobody
//! says that — it is spoken `níhǎo`, tone 2 + tone 3. Scoring a learner against
//! the dictionary tones would flag correct speech as wrong, which is the failure
//! the research warns about (§6.2). [`spoken_tones`] is that correction.
//!
//! ## Splitting without a syllable table
//!
//! A full Mandarin syllable inventory is about 400 entries. It is not needed: a
//! syllable is `[initial consonants] vowel-run [coda]`, the only codas are `n`,
//! `ng` and `r`, and pinyin writes an apostrophe before every non-initial syllable
//! that starts with `a`, `o` or `e` (`xī'ān`, not `xiān`; `nǚ'ér`, not `nǚér`) —
//! which is every vowel-initial syllable there is, because one that would start
//! with `i`, `u` or `ü` is written with `y` or `w` instead. So a vowel run
//! followed by anything but a coda starts a new syllable, and a consonant with a
//! vowel straight after it opens the next syllable rather than closing this one —
//! `qùnián` is `qù` + `nián`, and `quánguó` is `quán` + `guó`, because `quáng` +
//! `uó` would have been written `quáng'uó`. An `r` with no vowel after it is erhua and stays
//! on its syllable (`yīdiǎnr` is `yī` + `diǎnr`). That is a rule rather than a
//! table — and the caller checks the result against the number of characters, so
//! a reading this gets wrong is refused rather than scored.
//!
//! [`join_syllables`] is the other half of the rule: it writes syllables as one
//! reading with the apostrophes pinyin uses, which is how the dataset stores a
//! word's reading and how a reading composed from characters is written.

use std::ops::Range;

use serde::{Deserialize, Serialize};

/// One syllable of a reading.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Syllable {
    /// The syllable as written, tone mark included, e.g. `"xué"`.
    pub text: String,
    /// The tone it carries, 1..=4, or `5` for the neutral tone.
    pub tone: u8,
}

/// One syllable of a tone target: which character, how it reads, and which tone
/// to score it against.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSyllable {
    pub ch: char,
    /// The syllable as the dictionary writes it, e.g. `"nǐ"`.
    pub reading: String,
    /// The dictionary's tone, before sandhi.
    pub citation: u8,
    /// The tone actually spoken, after sandhi. This is what a recording is
    /// scored against.
    pub spoken: u8,
}

/// What to score a word or character against.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneTarget {
    pub syllables: Vec<TargetSyllable>,
    /// True when [`spoken_tones`] changed something, so the interface can say
    /// why the tone being asked for is not the one printed in a dictionary.
    pub sandhi_applied: bool,
    /// One plain sentence about the tones, worded here so there is one place the
    /// rule is explained.
    pub detail: String,
}

impl ToneTarget {
    /// The tones to score against, in order.
    pub fn spoken(&self) -> Vec<u8> {
        self.syllables.iter().map(|s| s.spoken).collect()
    }

    /// How many syllables carry a tone that can actually be judged.
    ///
    /// All five do, neutral included — it is judged on being level, and what
    /// that can and cannot see is `tone::NEUTRAL_LIMIT`. The range is written
    /// out here rather than imported from `tone`, which is the one place this
    /// module and that one overlap: they are kept apart on purpose (see the
    /// module notes in `tone.rs`), and a two-number range is a smaller price
    /// than the dependency. **Keep it equal to `tone::is_scorable`.**
    pub fn scorable(&self) -> usize {
        self.syllables
            .iter()
            .filter(|s| (1..=5).contains(&s.spoken))
            .count()
    }
}

/// Is this character part of a syllable's vowel run?
///
/// The syllabic nasals (`ń`, `ň`, `ǹ`, `ḿ`) count: 嗯 is `ń`, a syllable with no
/// vowel letter in it at all. So do capitals: CC-CEDICT capitalises a proper
/// noun, and the `É` of 俄语 `Éyǔ` starts a syllable the way `é` does.
fn is_vowel(ch: char) -> bool {
    matches!(
        lowercase(ch),
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'ü'
            | 'v'
            | 'ā'
            | 'á'
            | 'ǎ'
            | 'à'
            | 'ē'
            | 'é'
            | 'ě'
            | 'è'
            | 'ê'
            | 'ī'
            | 'í'
            | 'ǐ'
            | 'ì'
            | 'ō'
            | 'ó'
            | 'ǒ'
            | 'ò'
            | 'ū'
            | 'ú'
            | 'ǔ'
            | 'ù'
            | 'ǖ'
            | 'ǘ'
            | 'ǚ'
            | 'ǜ'
            | 'ń'
            | 'ň'
            | 'ǹ'
            | 'ḿ'
    )
}

/// The lower-case form of a letter, when it is a single letter.
fn lowercase(ch: char) -> char {
    let mut lower = ch.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(only), None) => only,
        _ => ch,
    }
}

/// The tone a single accented vowel marks, if it marks one.
fn marked_tone(ch: char) -> Option<u8> {
    match lowercase(ch) {
        'ā' | 'ē' | 'ī' | 'ō' | 'ū' | 'ǖ' | 'ń' | 'ḿ' => Some(1),
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'ǘ' => Some(2),
        'ǎ' | 'ě' | 'ǐ' | 'ǒ' | 'ǔ' | 'ǚ' | 'ň' => Some(3),
        'à' | 'è' | 'ì' | 'ò' | 'ù' | 'ǜ' | 'ǹ' => Some(4),
        _ => None,
    }
}

/// The characters that mark a syllable boundary by hand.
fn is_separator(ch: char) -> bool {
    matches!(ch, '\'' | ' ' | '-' | '·' | '/' | ',')
}

/// The tone a digit written after a syllable stands for: `ni3` is `nǐ`.
///
/// `1`–`4` are the four tones and `5` the neutral one, which is how CC-CEDICT
/// numbers them and how a keyboard with no pinyin input method writes them.
fn tone_number(ch: char) -> Option<u8> {
    match ch {
        '1'..='5' => Some(ch as u8 - b'0'),
        _ => None,
    }
}

/// One syllable of a run as the splitter found it, before its tone is read.
struct RunSyllable {
    /// The letters as written, with `u:` already read as `ü`.
    text: String,
    /// Where it sits, in letters from the start of the run.
    from: usize,
    to: usize,
    /// The tone number written after it, if one was.
    number: Option<u8>,
}

/// Split one run of letters into syllables, keeping where each one sits.
///
/// The span is in letters from the start of `chars`, and it exists for the
/// pinyin tone key: "which syllable is the cursor in" cannot be answered from
/// the syllables alone. Nothing else here wants it, which is why the plain
/// [`syllables`] is still the function everything reads.
///
/// A tone number straight after a syllable (`ni3`) ends it and belongs to it,
/// so it is a boundary the way an apostrophe is: `xi1an1` is `xī` + `ān`.
fn split_run_spans(chars: &[char]) -> Vec<RunSyllable> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut start = 0;
    let mut i = 0;

    while i < chars.len() {
        if !is_vowel(chars[i]) {
            if current.is_empty() {
                start = i;
            }
            current.push(chars[i]);
            i += 1;
            continue;
        }

        // A vowel run, then at most one coda: `n`, `ng`, or `r`.
        if current.is_empty() {
            start = i;
        }
        current.push(chars[i]);
        let mut j = i + 1;
        while j < chars.len() && is_vowel(chars[j]) {
            current.push(chars[j]);
            j += 1;
        }

        let vowel_at = |k: usize| chars.get(k).copied().is_some_and(is_vowel);
        // The codas are matched in lower case, as the vowels are, so a reading
        // typed in capitals splits the way it does in lower case.
        let letter_at = |k: usize| chars.get(k).copied().map(lowercase);
        if j < chars.len() {
            let mut nasal = false;
            match lowercase(chars[j]) {
                // `ng` is a coda unless a vowel follows it. Pinyin writes an
                // apostrophe before a syllable that starts with `a`, `o` or `e`
                // (`píng'ān`), and no syllable starts with another vowel, so an unmarked `ng` + vowel can only be `n`
                // closing this syllable and `g` opening the next: `quánguó` is
                // `quán` + `guó`, never `quáng` + `uó`.
                'n' if letter_at(j + 1) == Some('g') && !vowel_at(j + 2) => {
                    current.push(chars[j]);
                    current.push(chars[j + 1]);
                    j += 2;
                    nasal = true;
                }
                // A bare `n` is a coda unless a vowel follows it, in which case
                // it is the next syllable's initial — this is what separates
                // `qùnián` into `qù` + `nián`. Before `g` + vowel it is the coda,
                // by the rule above.
                'n' if !vowel_at(j + 1) => {
                    current.push(chars[j]);
                    j += 1;
                    nasal = true;
                }
                // `r` is a coda only when a vowel does not follow: `ér` is one
                // syllable, `nǚrén` is two. After a vowel this is also erhua:
                // `zhèr` is one syllable.
                'r' if !vowel_at(j + 1) => {
                    current.push(chars[j]);
                    j += 1;
                }
                _ => {}
            }
            // Erhua after a nasal: `diǎnr`, `kòngr`. No syllable is a lone
            // `r`, so an `r` with no vowel after it belongs to this one.
            if nasal && letter_at(j) == Some('r') && !vowel_at(j + 1) {
                current.push(chars[j]);
                j += 1;
            }
        }
        let number = chars.get(j).copied().and_then(tone_number);
        if number.is_some() {
            j += 1;
        }

        out.push(RunSyllable {
            text: std::mem::take(&mut current),
            from: start,
            to: j,
            number,
        });
        i = j;
    }

    if !current.is_empty() {
        out.push(RunSyllable {
            text: current,
            from: start,
            to: chars.len(),
            number: None,
        });
    }
    out
}

/// Does this syllable begin with `a`, `o` or `e`, the letters pinyin writes an
/// apostrophe before when another syllable comes first?
fn opens_with_a_vowel(syllable: &str) -> bool {
    syllable
        .chars()
        .next()
        .is_some_and(|first| matches!(lowercase(plain_vowel(first)), 'a' | 'o' | 'e' | 'ê'))
}

/// Write syllables as one word's reading, the way pinyin writes a word.
///
/// The syllables are run together (`nán` + `guò` → `nánguò`), with an
/// apostrophe before a syllable that begins with `a`, `o` or `e` and is not the
/// first (`nǚ` + `ér` → `nǚ'ér`, `hěn` + `ài` → `hěn'ài`). That is the
/// orthography's own rule, and it is exactly the boundary [`syllables`] could not
/// find without one: every other boundary is unambiguous to it, including an
/// `n` or `ng` before a consonant-initial syllable (`quán` + `guó`). A part that
/// is a bare erhua `r` is written onto the syllable before it, as pinyin writes
/// it (`zhè` + `r` → `zhèr`).
///
/// A part may itself be several syllables already run together; the rule is
/// applied at the joins only. Blank parts are skipped.
pub fn join_syllables<S: AsRef<str>>(parts: &[S]) -> String {
    let mut out = String::new();
    for part in parts {
        let part = part.as_ref().trim();
        if part.is_empty() {
            continue;
        }
        if !out.is_empty() && !part.eq_ignore_ascii_case("r") && opens_with_a_vowel(part) {
            out.push('\'');
        }
        out.push_str(part);
    }
    out
}

/// The tone a written syllable carries.
///
/// A syllable with no mark is the neutral tone (`5`) — but only if it has a vowel
/// at all, so that a stray consonant run is refused rather than called neutral.
fn syllable_tone(text: &str) -> Option<u8> {
    let marks: Vec<u8> = text.chars().filter_map(marked_tone).collect();
    match marks.len() {
        0 => text.chars().any(is_vowel).then_some(5),
        1 => Some(marks[0]),
        // Two tone marks in one syllable is not a syllable. This is what catches
        // an unseparated pair such as `nǐhǎo` being fed in as one.
        _ => None,
    }
}

/// Split a reading into its syllables.
///
/// `None` when the reading is empty or any part of it cannot be read as a
/// syllable. Apostrophes, spaces and hyphens are hard boundaries, which is what
/// makes `xi'an` two syllables and `xian` one.
///
/// A reading typed with tone numbers is read as the marks they stand for:
/// `ni3hao3` is `nǐ` + `hǎo`, and each syllable's text is the marked form, which
/// is the form the dataset writes and the interface shows. `v` and `u:` are the
/// keyboard's ways of writing `ü` (`nv3` is `nǚ`, `lu:4` is `lǜ`). A number that
/// disagrees with a mark already on its syllable (`hǎo4`) is two tones on one
/// syllable and refused, as is a digit anywhere but straight after a syllable.
pub fn syllables(reading: &str) -> Option<Vec<Syllable>> {
    Some(
        syllables_with_spans(reading)?
            .into_iter()
            .map(|(syllable, _)| syllable)
            .collect(),
    )
}

/// Split a reading into its syllables, and say where each one sits.
///
/// [`syllables`] is this without the spans. The tone key needs them: it rewrites
/// the syllable the cursor is in, so it has to know which characters that is,
/// and rebuilding the position by searching for the syllable's text would go
/// wrong the moment a reading contains the same syllable twice — `xuexí` does.
///
/// Offsets are in **characters** from the start of `reading`, matching
/// [`str::chars`] rather than the UTF-16 index a browser reports; the caller
/// converts, because only it knows which of the two it is holding. A span covers
/// what was typed, so `ni3`'s is three characters long though its text, `nǐ`, is
/// two, and `lu:4`'s is four.
pub fn syllables_with_spans(reading: &str) -> Option<Vec<(Syllable, Range<usize>)>> {
    if reading.trim().is_empty() {
        return None;
    }

    let chars: Vec<char> = reading.chars().collect();
    let mut out = Vec::new();
    // The run's letters, and where each one starts in `reading`. The two differ
    // only where `u:` has been read as the one letter `ü`.
    let mut run: Vec<char> = Vec::new();
    let mut origin: Vec<usize> = Vec::new();

    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if is_separator(ch) {
            push_run(&mut out, &run, &origin, index)?;
            run.clear();
            origin.clear();
            index += 1;
            continue;
        }
        origin.push(index);
        if matches!(ch, 'u' | 'U') && chars.get(index + 1) == Some(&':') {
            run.push(if ch == 'u' { 'ü' } else { 'Ü' });
            index += 2;
        } else {
            run.push(ch);
            index += 1;
        }
    }
    push_run(&mut out, &run, &origin, chars.len())?;

    (!out.is_empty()).then_some(out)
}

/// Read one run's syllables into `out`, with their spans in the whole reading.
///
/// `origin` holds where each letter of `run` starts in the reading and `end`
/// where the run stops. `None` when any of the syllables cannot be read as one,
/// which is [`syllables`]'s rule and has to stay one rule.
fn push_run(
    out: &mut Vec<(Syllable, Range<usize>)>,
    run: &[char],
    origin: &[usize],
    end: usize,
) -> Option<()> {
    for syllable in split_run_spans(run) {
        // A digit the splitter did not take as a tone number is not pinyin.
        if syllable.text.chars().any(|ch| ch.is_ascii_digit()) {
            return None;
        }
        let text = match syllable.number {
            None => syllable.text,
            Some(number) => match syllable_tone(&syllable.text)? {
                // No mark yet: the number is written as one.
                5 => mark_syllable(&syllable.text, number)?,
                marked if marked == number => syllable.text,
                _ => return None,
            },
        };
        let tone = syllable_tone(&text)?;
        let start = origin[syllable.from];
        let stop = origin.get(syllable.to).copied().unwrap_or(end);
        out.push((Syllable { text, tone }, start..stop));
    }
    Some(())
}

/// A reading with its tone numbers written as marks: `ni3hao3` → `nǐhǎo`.
///
/// This is what a learner's typed reading is saved as, so a list shows the
/// reading the way the dataset writes one. `u:` becomes `ü` on the way, and an
/// apostrophe is written where the number was the only thing separating a
/// syllable that starts with `a`, `o` or `e` (`xi1an1` → `xī'ān`), because without
/// it the marked reading would split differently from the numbered one.
///
/// Anything [`syllables`] cannot read is returned exactly as given, and so is a
/// reading with no number or `u:` in it: there is nothing to rewrite, and a
/// pinyin box that accepts anything should not change what it was given.
pub fn marked_reading(reading: &str) -> String {
    if !reading.chars().any(|ch| ch.is_ascii_digit() || ch == ':') {
        return reading.to_string();
    }
    let Some(list) = syllables_with_spans(reading) else {
        return reading.to_string();
    };

    let chars: Vec<char> = reading.chars().collect();
    let mut out = String::with_capacity(reading.len());
    let mut at = 0;
    for (syllable, span) in &list {
        out.extend(&chars[at..span.start]);
        if at == span.start && !out.is_empty() && opens_with_a_vowel(&syllable.text) {
            out.push('\'');
        }
        out.push_str(&syllable.text);
        at = span.end;
    }
    out.extend(&chars[at..]);
    out
}

// ---------------------------------------------------------------------------
// Writing a tone mark
// ---------------------------------------------------------------------------

/// Write `tone` onto one syllable, in the place a pinyin tone mark belongs.
///
/// The placement is the standard rule rather than a choice: `a` takes the mark
/// if there is one, then `o`, then `e`, and failing all three the last of
/// `i`/`u`/`ü` — which is what puts the mark on the `u` of `iu` and the `i` of
/// `ui`, the two cases a learner gets wrong by hand.
///
/// The syllable is reduced to plain letters first, so a syllable that already
/// carries a tone is **changed** rather than stacked on, and a neutral tone (5)
/// is written as the syllable with no mark at all. That is the whole reason this
/// lives next to [`syllables`] rather than in the interface: a mark written one
/// way and read back another would be two rules for one thing.
///
/// `None` for a tone outside `1..=5`, and for a syllable with nothing a mark can
/// sit on.
pub fn mark_syllable(syllable: &str, tone: u8) -> Option<String> {
    if !(1..=5).contains(&tone) {
        return None;
    }
    let plain: Vec<char> = syllable.chars().map(plain_vowel).collect();
    if tone == 5 {
        return Some(plain.into_iter().collect());
    }

    let target = plain
        .iter()
        .position(|ch| matches!(ch, 'a' | 'A'))
        .or_else(|| plain.iter().position(|ch| matches!(ch, 'o' | 'O')))
        .or_else(|| plain.iter().position(|ch| matches!(ch, 'e' | 'E')))
        .or_else(|| {
            plain
                .iter()
                .rposition(|ch| matches!(ch, 'i' | 'I' | 'u' | 'U' | 'ü' | 'Ü' | 'v' | 'V'))
        })
        .or_else(|| plain.iter().rposition(|ch| matches!(ch, 'n' | 'N' | 'm' | 'M')))?;

    let mut out = String::with_capacity(syllable.len());
    for (index, ch) in plain.iter().enumerate() {
        if index == target {
            out.push(with_tone(*ch, tone)?);
        } else {
            out.push(*ch);
        }
    }
    Some(out)
}

/// Write a tone onto the syllable the cursor is in.
///
/// `caret` is a **character** offset into `reading`, matching [`str::chars`] and
/// not the UTF-16 index a browser reports — the caller converts, because only it
/// knows which of the two it has. A caret at the end of a syllable belongs to
/// that syllable, which is what makes "type `xue`, tap tone 2" work; a caret at
/// the *start* of one belongs to it too, so a cursor placed before a syllable
/// can still mark it.
///
/// Returns the rewritten reading and where the cursor should land. A mark
/// replaces one letter with one letter, so that is usually where it already was.
/// A syllable typed with a tone number (`ni3`) is rewritten without it, and the
/// cursor moves back with the text; when the number was the only boundary
/// before a syllable starting with `a`, `o` or `e` (`ping2an1`), the apostrophe
/// it stood for is written so the reading still splits where it did. `None`
/// when no syllable holds the cursor, or when [`mark_syllable`] refuses the one
/// that does.
pub fn mark_tone_at(reading: &str, caret: usize, tone: u8) -> Option<(String, usize)> {
    let syllables = syllables_with_spans(reading)?;
    let at = syllables
        .iter()
        .position(|(_, span)| caret > span.start && caret <= span.end)
        .or_else(|| syllables.iter().position(|(_, span)| caret == span.start))?;
    let (syllable, span) = &syllables[at];

    let marked = mark_syllable(&syllable.text, tone)?;
    let characters: Vec<char> = reading.chars().collect();
    // The neighbours that touch this syllable with nothing between them. One
    // before it that ends in its own tone number is already kept apart by it.
    let joined_before = at > 0
        && syllables[at - 1].1.end == span.start
        && !characters[span.start - 1].is_ascii_digit();
    let joined_after = syllables
        .get(at + 1)
        .filter(|(_, next)| next.start == span.end);

    let mut out: String = characters[..span.start].iter().collect();
    let mut start = span.start;
    if joined_before && opens_with_a_vowel(&marked) {
        out.push('\'');
        start += 1;
    }
    out.push_str(&marked);
    if joined_after.is_some_and(|(next, _)| opens_with_a_vowel(&next.text)) {
        out.push('\'');
    }
    out.extend(characters[span.end..].iter());
    let into = (caret - span.start).min(marked.chars().count());
    Some((out, start + into))
}

/// The plain letter under a tone mark, so a mark can be moved or taken off.
///
/// The nasal forms are here too: 嗯 is `ń`, and a tone key that could not touch
/// it would be a key that quietly did nothing on one of the commonest words. A
/// capital keeps its case: `À` is `A`, so a proper noun stays one.
fn plain_vowel(ch: char) -> char {
    let plain = match lowercase(ch) {
        'ā' | 'á' | 'ǎ' | 'à' => 'a',
        'ē' | 'é' | 'ě' | 'è' => 'e',
        'ī' | 'í' | 'ǐ' | 'ì' => 'i',
        'ō' | 'ó' | 'ǒ' | 'ò' => 'o',
        'ū' | 'ú' | 'ǔ' | 'ù' => 'u',
        'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' => 'ü',
        'ń' | 'ň' | 'ǹ' => 'n',
        'ḿ' => 'm',
        _ => return ch,
    };
    if ch.is_uppercase() {
        uppercase(plain)
    } else {
        plain
    }
}

/// The upper-case form of a letter, when it is a single letter.
fn uppercase(ch: char) -> char {
    let mut upper = ch.to_uppercase();
    match (upper.next(), upper.next()) {
        (Some(only), None) => only,
        _ => ch,
    }
}

/// The same letter carrying `tone`, or `None` when it cannot carry one.
///
/// The nasal forms follow [`marked_tone`], which is the table this has to agree
/// with: `ń` is the shape it reads as tone 1, and it has no tone-2 nasal at all,
/// so one is refused here rather than invented. `v` is marked as `ü` — the
/// dataset and CC-CEDICT write the umlaut, and a learner typing `nv` means 女.
/// A capital is marked as a capital (`A` → `Ǎ`), so `Aomen` marks as `Àomen`.
fn with_tone(ch: char, tone: u8) -> Option<char> {
    // Marking a neutral tone is a different job — the syllable with no mark —
    // and is handled before this is reached. The guard is here so the index
    // below cannot underflow if that ever stops being true.
    if !(1..=4).contains(&tone) {
        return None;
    }
    let index = (tone - 1) as usize;
    let marked = match lowercase(ch) {
        'a' => Some(['ā', 'á', 'ǎ', 'à'][index]),
        'e' => Some(['ē', 'é', 'ě', 'è'][index]),
        'i' => Some(['ī', 'í', 'ǐ', 'ì'][index]),
        'o' => Some(['ō', 'ó', 'ǒ', 'ò'][index]),
        'u' => Some(['ū', 'ú', 'ǔ', 'ù'][index]),
        'ü' | 'v' => Some(['ǖ', 'ǘ', 'ǚ', 'ǜ'][index]),
        'n' => match tone {
            1 => Some('ń'),
            3 => Some('ň'),
            4 => Some('ǹ'),
            _ => None,
        },
        'm' => (tone == 1).then_some('ḿ'),
        _ => None,
    }?;
    Some(if ch.is_uppercase() {
        uppercase(marked)
    } else {
        marked
    })
}

/// The tone a single-syllable reading carries, or `None` when it cannot be read.
///
/// A reading of several syllables is refused rather than half-read: it has
/// several tones and none of them is the target of a one-syllable exercise.
pub fn tone_from_pinyin(reading: &str) -> Option<u8> {
    let list = syllables(reading)?;
    (list.len() == 1).then(|| list[0].tone)
}

/// The syllables of `reading`, when they divide into exactly one per character
/// of `text`.
///
/// ## What this is for
///
/// A screen that colours characters by tone needs to know **which syllable
/// belongs to which character**, and that cannot be answered from the reading
/// alone: `xuéxí` is two syllables and two characters only because the caller
/// says there are two. So the pairing rule lives here, next to the rule that
/// finds the syllables in the first place, and a caller that gets `Some` can
/// read one entry per character out of it.
///
/// ## Why `None` rather than a partial answer
///
/// The same refusal [`tone_target`] makes, for the same reason: a reading that
/// does not divide one syllable per character is misaligned, and pairing it up
/// anyway would attach a tone to the wrong glyph. `None` means "this reading
/// cannot be read against this text", and the caller is expected to fall back to
/// each character's own reading rather than to guess.
///
/// An apostrophe, space or hyphen is a hard boundary, so a caller composing
/// readings out of characters (`yī'tiān` for 一天) still gets the alignment it
/// asked for — see [`syllables`].
pub fn aligned_reading(text: &str, reading: &str) -> Option<Vec<Syllable>> {
    let list = syllables(reading)?;
    (!list.is_empty() && list.len() == text.chars().count()).then_some(list)
}

/// The tones as they are actually spoken, given the dictionary's own.
///
/// Three rules, which are the ones that matter for vocabulary:
///
/// 1. **Third tone before third tone** becomes second. A run shortens all but its
///    last syllable: 你好 `3+3` → `2+3`, 我很好 `3+3+3` → `2+2+3`.
/// 2. **不** (`bù`, tone 4) becomes second before a fourth tone: 不是 `4+4` →
///    `2+4`. It stays fourth otherwise.
/// 3. **一** (`yī`, tone 1) becomes second before a fourth tone (一个 → `2+4`) and
///    fourth before anything else (一天 → `4+1`, 一起 → `4+3`) — when it counts
///    something. When it is a number's name rather than a count it stays first:
///    - at the end of a word (统一, 星期一, 万一), with nothing to change before;
///    - as an ordinal after 第 (第一次 `4+1+4`, 第一天 `4+1+1`);
///    - read as a digit, next to another digit (十一, 二十一, 一二三, 一九九八).
///      百, 千, 万 and 亿 are not digits here: 一百 counts a hundred and is
///      `yìbǎi`, so before them 一 changes as it does before any word;
///    - in a date, before 月 or 号 (一月 is January, `yīyuè`; 十一月 is
///      November; 一号 is the first of the month), and before 日 only after a
///      month (五月一日). 一日 on its own is "one day", and is counted: 日复一日
///      and 有朝一日 are both `yírì`.
///
///    Between a verb and its own repetition (看一看, 想一想, 试一试) it is
///    neither: the verb is said twice with a light 一 between, so it is the
///    neutral tone (`4+5+4`).
///
/// These are the rules the standard descriptions give — the usage note under 一
/// in the 现代汉语词典 (first tone alone, at the end of a word and as an ordinal;
/// second before a fourth tone, fourth before the others; light between a
/// repeated verb), and Chao's *A Grammar of Spoken Chinese* (1968) — narrowed to
/// what can be seen from the characters alone. What cannot is left as the counted
/// reading: 一楼 is "the first floor" or "a whole building" depending on the
/// sentence, and the counted reading is the commoner.
///
/// The characters are needed as well as the tones because rules 2 and 3 are about
/// *which* character it is, not which tone.
///
/// Deliberately not modelled: the half-third-tone realisation of a third tone in
/// running speech (which is a matter of how far the dip goes, not which tone it
/// is), and the optional sandhi of 一 in very casual speech.
pub fn spoken_tones(characters: &[char], citation: &[u8]) -> Vec<u8> {
    let mut tones = citation.to_vec();
    let count = tones.len().min(characters.len());

    for i in 0..count {
        let next = tones.get(i + 1).copied();
        match characters[i] {
            '不' if tones[i] == 4 && next == Some(4) => tones[i] = 2,
            '一' if tones[i] == 1 => {
                let before = i.checked_sub(1).map(|k| characters[k]);
                let after = characters[..count].get(i + 1).copied();
                if before.is_some() && before == after && before != Some('一') {
                    // A一A: a verb, a light 一, the verb again.
                    tones[i] = 5;
                } else if before == Some('第')
                    || before.is_some_and(is_digit)
                    || after.is_some_and(|ch| is_digit(ch) || matches!(ch, '月' | '号'))
                    || (before == Some('月') && after == Some('日'))
                {
                    // An ordinal, a digit or a date: the number's own name.
                } else {
                    match next {
                        Some(4) => tones[i] = 2,
                        Some(_) => tones[i] = 4,
                        None => {}
                    }
                }
            }
            _ => {}
        }
    }

    // The third-tone run, last. Applied after the other two so that a 一 which
    // became fourth is not counted as part of a run of thirds.
    let mut i = 0;
    while i < tones.len() {
        if tones[i] == 3 {
            let start = i;
            let mut end = i;
            while end + 1 < tones.len() && tones[end + 1] == 3 {
                end += 1;
            }
            for tone in tones.iter_mut().take(end).skip(start) {
                *tone = 2;
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }

    tones
}

/// A character that names a digit, as 一 is read beside one: 十一, 一二三.
///
/// 十 is here because 十一 is "ten-one", 一 read as the digit it is. 百, 千, 万
/// and 亿 are not: 一百 counts a hundred, and that 一 changes (`yìbǎi`). 两 is
/// not either, because it is the counting form of two and never sits beside a
/// digit-reading 一.
fn is_digit(ch: char) -> bool {
    matches!(
        ch,
        '〇' | '零' | '一' | '二' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '十'
    )
}

/// Build the tone target for `text`, read as `reading`.
///
/// `None` when the reading does not divide into exactly as many syllables as
/// there are characters. That check is the point of the function: it is what
/// makes a splitting rule safe, because a reading the rule gets wrong is refused
/// rather than silently mis-aligned against the recording.
///
/// There is no longer a second refusal for a reading whose syllables are all
/// neutral — 的, 了, 吗 and the rest of the particles. It used to be here, and it
/// was the wrong place for it: whether a tone can be *judged* is a question about
/// the analysis, which this module knows nothing about, not about pinyin. Neutral
/// tones are scored now (see `tone::NEUTRAL_LIMIT` for what that can and cannot
/// see), so the decision no longer exists to make.
pub fn tone_target(text: &str, reading: &str) -> Option<ToneTarget> {
    let characters: Vec<char> = text.chars().collect();
    let list = syllables(reading)?;
    if characters.is_empty() || list.len() != characters.len() {
        return None;
    }

    let citation: Vec<u8> = list.iter().map(|s| s.tone).collect();
    let spoken = spoken_tones(&characters, &citation);

    let syllables: Vec<TargetSyllable> = characters
        .iter()
        .zip(list.iter())
        .zip(spoken.iter())
        .zip(citation.iter())
        .map(|(((ch, written), spoken), citation)| TargetSyllable {
            ch: *ch,
            reading: written.text.clone(),
            citation: *citation,
            spoken: *spoken,
        })
        .collect();

    let sandhi_applied = spoken != citation;
    let detail = if sandhi_applied {
        format!(
            "Said as a word this is {}, not the {} a dictionary lists syllable by \
             syllable — tones are scored as they are spoken.",
            join_tones(&spoken),
            join_tones(&citation)
        )
    } else {
        format!("Scored against {}.", join_tones(&spoken))
    };

    Some(ToneTarget {
        syllables,
        sandhi_applied,
        detail,
    })
}

/// `"2 + 3"`, or `"1 + neutral"`, for a sentence.
///
/// A neutral tone is spelled out rather than numbered. "1 + 5" reads as a fifth
/// full tone beside the four the course teaches, and a learner has no reason to
/// know that 5 means "no tone mark on the reading". The full tones stay digits,
/// because that is what the panel's own labels use.
fn join_tones(tones: &[u8]) -> String {
    tones
        .iter()
        .map(|t| {
            if *t == 5 {
                "neutral".to_string()
            } else {
                t.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

// ---------------------------------------------------------------------------
// What was heard
// ---------------------------------------------------------------------------

/// A reading with its tone taken off: `nǐ` → `ni`, `nǚ` → `nv`.
///
/// This is what a transcription is compared against a target *as*. Two reasons,
/// and they are different reasons:
///
/// - **Comparing characters does not work for single syllables.** 是, 事 and 士
///   are all `shì`, so a recogniser that transcribed the right sound as the
///   wrong character would be reported as a wrong syllable. They are the same
///   sound; that is the whole point of comparing readings.
/// - **The tone is dropped on purpose**, and not because it does not matter. It
///   matters more than anything else here — it is `tone.rs`'s entire job. It is
///   dropped because a recogniser's *reading of a character implies a tone that
///   the learner may never have produced*: the language model repairs a wrong
///   tone toward the likely word (research §6.1), so a dictionary tone attached
///   to a transcribed character is evidence about the language model, not about
///   the learner's voice. A tone score has to come from F0, and it does
///   ([`crate::tone`]).
///
/// `ü` is kept as its own letter rather than folded into `u`: 女 (`nǚ`) and 努
/// (`nǔ`) are different syllables, and treating them as the same sound would
/// call a wrong syllable right — the one outcome worse than saying nothing.
/// Case is folded, marks included, so a capitalised `Àomén` is `aomen`.
pub fn base(reading: &str) -> String {
    reading
        .chars()
        .map(lowercase)
        .filter_map(|ch| match ch {
            'a' | 'ā' | 'á' | 'ǎ' | 'à' => Some('a'),
            'e' | 'ē' | 'é' | 'ě' | 'è' | 'ê' => Some('e'),
            'i' | 'ī' | 'í' | 'ǐ' | 'ì' => Some('i'),
            'o' | 'ō' | 'ó' | 'ǒ' | 'ò' => Some('o'),
            'u' | 'ū' | 'ú' | 'ǔ' | 'ù' => Some('u'),
            'ü' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' | 'v' => Some('v'),
            // The syllabic nasals: 嗯 is `ń`, a syllable with no vowel letter.
            'n' | 'ń' | 'ň' | 'ǹ' => Some('n'),
            'm' | 'ḿ' => Some('m'),
            other => other.is_ascii_alphabetic().then(|| other.to_ascii_lowercase()),
        })
        .collect()
}

/// One syllable of a transcription, beside the one the exercise asked for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeardSyllable {
    /// The syllable as heard, plain: `shi`. No tone mark — see [`base`]. This is
    /// the comparison against [`Self::wanted`], and it is what must keep having
    /// no tone in it.
    pub base: String,
    /// The syllable **as the dictionary reads the character the model wrote**,
    /// tone mark included: `shì`; empty when the dataset has no reading for it.
    ///
    /// This is a fact about the model's *spelling*, not about the learner's voice:
    /// a recogniser's character carries a dictionary tone the learner may never
    /// have produced, because the language model repairs a wrong tone toward the
    /// likely word (research §6.1). It is shown for one reason — a tone mark is
    /// what makes `cóng` identify 从 rather than leave `cong` ambiguous between
    /// 从, 葱 and 匆 — and the interface shows it **only on a syllable the model
    /// heard differently**, never on one that matched. The tone that was said is
    /// scored from the pitch in [`crate::tone`], and this is not it.
    pub reading: String,
    /// The syllable the exercise asked for, the same way: `si`.
    pub wanted: String,
    /// The syllable the exercise asked for **as the dictionary writes it**, tone
    /// mark included: `sì`.
    ///
    /// Paired with [`Self::reading`], this is what lets the interface tell "heard
    /// the right sound at a different tone" (`cóng` written where `zhōng` was
    /// asked for) from "heard the right sound and the same tone". The comparison
    /// itself is on [`Self::base`] and [`Self::wanted`] alone; this pair is
    /// display-only.
    pub wanted_reading: String,
    /// True when they are the same sound once the tone is set aside.
    pub matches: bool,
}

/// What a speech recogniser made of one recording, read against the target.
///
/// ## What this is evidence of, and what it is not
///
/// It answers *which syllables were said*, not *how well*. A recogniser carries a
/// strong language-model prior and is built to be robust to the errors a learner
/// makes, so it under-reports them — most for the learners who need feedback most
/// (research §6.1). Read it as: "the app did not hear the syllable you were asked
/// for", which is a real and useful thing to be told, and never as "your
/// pronunciation was correct", which it cannot know. The tone is scored from the
/// pitch, separately, and [`Heard::detail`] says which half is which so no one
/// has to guess.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Heard {
    /// What was transcribed, in characters. Empty when nothing was recognised.
    pub text: String,
    /// The same, as plain letters with the syllables spaced: `shi shi`.
    pub base: String,
    /// One entry per syllable the transcription divided into.
    pub syllables: Vec<HeardSyllable>,
    /// How many syllables were the sound asked for.
    pub matched: usize,
    /// True when the transcription divided into exactly as many syllables as the
    /// target has, which is what makes a per-syllable comparison meaningful.
    pub same_count: bool,
    /// One plain sentence, worded here so there is one place it is worded.
    pub detail: String,
}

impl Heard {
    /// Everything asked for was heard, syllable for syllable.
    pub fn all_matched(&self) -> bool {
        self.same_count && !self.syllables.is_empty() && self.matched == self.syllables.len()
    }
}

/// Read a transcription against the target it was meant to be.
///
/// `reading` is the transcription's own reading — the word's, from the dataset,
/// so that a polyphone comes out right, or a reading the caller composed from the
/// characters. It is taken as a parameter rather than looked up because this
/// module knows nothing about the dataset, exactly as [`tone_target`] does.
///
/// The same alignment rule as [`tone_target`] applies: a reading that does not
/// divide into one syllable per character is **refused rather than compared out
/// of step**, because a split this module got wrong would be reported to the
/// learner as a syllable they mispronounced.
pub fn heard_against(heard: &str, reading: &str, target: &ToneTarget) -> Heard {
    let wanted: Vec<String> = target.syllables.iter().map(|s| s.reading.clone()).collect();
    heard_against_readings(heard, reading, &wanted, true)
}

/// Read a transcription against the readings that were wanted, one per
/// character.
///
/// This is the general form of [`heard_against`], and it exists because
/// recognition does not need a tone target. A phrase longer than a word has no
/// target — its syllable boundaries cannot be found from the recording, so no
/// tone is scored — but the syllables the learner was asked for are still known,
/// and a transcription can still be read against them. That is what lets the
/// microphone answer for text tone practice refuses.
///
/// `wanted` holds one raw reading per character, tone marks and all; they are
/// reduced with [`base`] here, exactly as a target's are. An **empty** `wanted`
/// means there was nothing to compare against — the dataset could not read every
/// character — and the transcription is reported on its own rather than compared
/// against nothing.
///
/// `tone_follows` says whether a tone judgement is shown under this
/// transcription. It only picks the wording: with no tone panel below, telling
/// the learner "the tone below is unaffected" would point at nothing.
pub fn heard_against_readings(
    heard: &str,
    reading: &str,
    wanted: &[String],
    tone_follows: bool,
) -> Heard {
    let characters: Vec<char> = heard.chars().collect();
    let list = syllables(reading).unwrap_or_default();
    let aligned = !characters.is_empty() && list.len() == characters.len();

    // Two parallel views of the model's reading, both empty when the halves
    // cannot be paired: the syllables as written (tone marks in, for display) and
    // the same syllables reduced (tone marks out, for the comparison).
    let heard_readings: Vec<String> = if aligned {
        list.iter().map(|s| s.text.clone()).collect()
    } else {
        Vec::new()
    };
    let heard_syllables: Vec<String> = heard_readings.iter().map(|reading| base(reading)).collect();
    let wanted_readings: Vec<String> = wanted.to_vec();
    let wanted: Vec<String> = wanted_readings.iter().map(|reading| base(reading)).collect();

    let syllables: Vec<HeardSyllable> = heard_syllables
        .iter()
        .zip(wanted.iter())
        .enumerate()
        .map(|(index, (base, wanted))| HeardSyllable {
            base: base.clone(),
            reading: heard_readings.get(index).cloned().unwrap_or_default(),
            wanted: wanted.clone(),
            wanted_reading: wanted_readings.get(index).cloned().unwrap_or_default(),
            matches: base == wanted,
        })
        .collect();
    let matched = syllables.iter().filter(|s| s.matches).count();
    let same_count = aligned && heard_syllables.len() == wanted.len();
    let base_text = heard_syllables.join(" ");
    let wanted_text = wanted.join(" ");
    // The heard side with a tone mark **only where the sound did not match**: a
    // matched syllable's dictionary tone is the model's spelling, not the
    // learner's pitch, and putting a mark on it would read as a tone judgement.
    let marked_text = syllables
        .iter()
        .map(|s| {
            if s.matches || s.reading.is_empty() {
                s.base.as_str()
            } else {
                s.reading.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let detail = if heard.trim().is_empty() {
        if tone_follows {
            "Nothing was recognised in that recording. The tone below is still judged \
             from the pitch, which does not need a transcription."
                .to_string()
        } else {
            "Nothing was recognised in that recording.".to_string()
        }
    } else if wanted.is_empty() {
        format!(
            "Heard {}. The words on the board could not all be read from the dataset, \
             so what was heard was not compared against them.",
            heard.trim()
        )
    } else if !aligned {
        format!(
            "The transcription ({}) could not be divided into one syllable per character, \
             so it was not read against what was asked for.{}",
            heard.trim(),
            if tone_follows {
                " The tone below is unaffected."
            } else {
                ""
            }
        )
    } else if !same_count {
        format!(
            "Heard {base_text} — {} syllable{} where {wanted_text} ({}) was asked for.{}",
            wanted.len(),
            if wanted.len() == 1 { "" } else { "s" },
            wanted.len(),
            if tone_follows {
                " The tone below is still judged from the pitch."
            } else {
                ""
            }
        )
    } else if matched == syllables.len() {
        format!(
            "Heard {base_text}: the syllable{} asked for. What was heard is a transcription, \
             not a judgement of the tone.{}",
            if syllables.len() == 1 { "" } else { "s" },
            if tone_follows {
                " The tone below is measured from the pitch."
            } else {
                ""
            }
        )
    } else {
        let wrong: Vec<String> = syllables
            .iter()
            .filter(|s| !s.matches)
            .map(|s| {
                format!(
                    "{} where {} was asked for",
                    if s.reading.is_empty() {
                        &s.base
                    } else {
                        &s.reading
                    },
                    s.wanted
                )
            })
            .collect();
        format!(
            "Heard {marked_text} ({}) — {}.{}",
            wrong.join(", "),
            if matched == 0 {
                "none of that is the syllable wanted".to_string()
            } else {
                format!(
                    "{} of {} syllables match",
                    matched,
                    syllables.len()
                )
            },
            if tone_follows {
                " The tone below is measured from the pitch, not from this transcription."
            } else {
                ""
            }
        )
    };

    Heard {
        text: heard.trim().to_string(),
        base: base_text,
        syllables,
        matched,
        same_count,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(reading: &str) -> Vec<String> {
        syllables(reading)
            .unwrap_or_else(|| panic!("{reading} should split"))
            .into_iter()
            .map(|s| s.text)
            .collect()
    }

    fn tones(reading: &str) -> Vec<u8> {
        syllables(reading)
            .unwrap_or_else(|| panic!("{reading} should split"))
            .into_iter()
            .map(|s| s.tone)
            .collect()
    }

    #[test]
    fn a_single_syllable_is_left_whole() {
        assert_eq!(split("hǎo"), ["hǎo"]);
        assert_eq!(split("yī"), ["yī"]);
        assert_eq!(split("ér"), ["ér"]);
        assert_eq!(split("de"), ["de"]);
        assert_eq!(split("xian"), ["xian"], "xian is one syllable");
        assert_eq!(split("zhuāng"), ["zhuāng"]);
        assert_eq!(split("xióng"), ["xióng"]);
    }

    #[test]
    fn syllables_are_split_at_the_vowel_runs() {
        assert_eq!(split("xuéxí"), ["xué", "xí"]);
        assert_eq!(split("zháojí"), ["zháo", "jí"]);
        assert_eq!(split("nǐhǎo"), ["nǐ", "hǎo"]);
        assert_eq!(split("bùcuò"), ["bù", "cuò"]);
        assert_eq!(split("shénme"), ["shén", "me"]);
        assert_eq!(split("péngyou"), ["péng", "you"]);
        assert_eq!(split("yīdiǎn"), ["yī", "diǎn"]);
        assert_eq!(split("wǒmen"), ["wǒ", "men"]);
    }

    #[test]
    fn a_leading_consonant_of_the_next_syllable_is_not_a_coda() {
        // The `n` in `nián` is an initial, not the coda of `qù`.
        assert_eq!(split("qùnián"), ["qù", "nián"]);
        assert_eq!(split("kànkan"), ["kàn", "kan"]);
        // `r` likewise: a coda in `ér`, an initial in `rén`.
        assert_eq!(split("érzi"), ["ér", "zi"]);
        assert_eq!(split("nǚrén"), ["nǚ", "rén"]);
        assert_eq!(tones("nǚrén"), [3, 2]);
    }

    #[test]
    fn an_apostrophe_is_a_hard_boundary() {
        assert_eq!(split("xi'an"), ["xi", "an"]);
        assert_eq!(split("xī'ān"), ["xī", "ān"]);
        assert_eq!(split("píng'ān"), ["píng", "ān"]);
        // And the same letters without it are one syllable.
        assert_eq!(split("xiān"), ["xiān"]);
        assert_eq!(split("nǐ hǎo"), ["nǐ", "hǎo"]);
    }

    #[test]
    fn an_unmarked_ng_before_a_vowel_is_n_then_g() {
        // Pinyin writes an apostrophe before a syllable that starts with a vowel
        // (`píng'ān`), so an `ng` with a vowel straight after it and no apostrophe
        // is `n` closing one syllable and `g` opening the next. Issue #19: every
        // one of these came back as `-ng` + a vowel-initial syllable.
        assert_eq!(split("quánguó"), ["quán", "guó"]);
        assert_eq!(split("jiànguò"), ["jiàn", "guò"]);
        assert_eq!(split("nánguò"), ["nán", "guò"]);
        assert_eq!(split("cānguān"), ["cān", "guān"]);
        assert_eq!(split("bàngōngshì"), ["bàn", "gōng", "shì"]);
        assert_eq!(split("réngōng"), ["rén", "gōng"]);
        assert_eq!(tones("quánguó"), [2, 2]);
        // An `ng` that is not followed by a vowel is still the coda.
        assert_eq!(split("Zhōngguó"), ["Zhōng", "guó"]);
        assert_eq!(split("péngyou"), ["péng", "you"]);
        assert_eq!(split("chénglǐ"), ["chéng", "lǐ"]);
        assert_eq!(split("zhuāng"), ["zhuāng"]);
    }

    #[test]
    fn the_apostrophes_the_orthography_writes_are_boundaries() {
        // The words issue #19 found refused or mis-split once the dataset's
        // separators were lost, written the way the dataset now stores them.
        assert_eq!(split("wǎn'ān"), ["wǎn", "ān"]);
        assert_eq!(split("rán'ér"), ["rán", "ér"]);
        assert_eq!(split("nǚ'ér"), ["nǚ", "ér"]);
        assert_eq!(split("kě'ài"), ["kě", "ài"]);
        assert_eq!(split("dì'èr"), ["dì", "èr"]);
        assert_eq!(tones("nǚ'ér"), [3, 2]);
        assert_eq!(tones("kě'ài"), [3, 4]);
    }

    #[test]
    fn a_trailing_r_is_erhua_and_joins_its_syllable() {
        // 儿化: the `r` is written onto the syllable before it and is not a
        // syllable of its own, whatever coda that syllable already has.
        assert_eq!(split("zhèr"), ["zhèr"]);
        assert_eq!(split("nǎr"), ["nǎr"]);
        assert_eq!(split("wánr"), ["wánr"]);
        assert_eq!(split("yīdiǎnr"), ["yī", "diǎnr"]);
        assert_eq!(split("yǒukòngr"), ["yǒu", "kòngr"]);
        assert_eq!(split("bùyīhuǐr"), ["bù", "yī", "huǐr"]);
        assert_eq!(tones("yīdiǎnr"), [1, 3]);
        // An `r` with a vowel after it is still the next syllable's initial.
        assert_eq!(split("ānrán"), ["ān", "rán"]);
        assert_eq!(split("shēngrì"), ["shēng", "rì"]);
    }

    #[test]
    fn a_capital_vowel_is_still_a_vowel() {
        // 俄语: CC-CEDICT capitalises a proper noun, and `É` has to start a
        // syllable the way `é` does.
        assert_eq!(split("Éyǔ"), ["É", "yǔ"]);
        assert_eq!(tones("Éyǔ"), [2, 3]);
        assert_eq!(split("Ōuzhōu"), ["Ōu", "zhōu"]);
    }

    #[test]
    fn a_reading_in_capitals_splits_as_it_does_in_lower_case() {
        // Capital vowels are vowels, so capital codas have to be codas too, or
        // ZHONGGUO comes apart as ZHO + NGGUO.
        assert_eq!(split("ZHONGGUO"), ["ZHONG", "GUO"]);
        assert_eq!(split("NIHAO"), ["NI", "HAO"]);
        assert_eq!(split("QUANGUO"), ["QUAN", "GUO"]);
        assert_eq!(split("NÜ'ÉR"), ["NÜ", "ÉR"]);
        assert_eq!(split("YIDIANR"), ["YI", "DIANR"]);
        assert_eq!(split("Nánjīng"), ["Nán", "jīng"]);
        assert_eq!(
            aligned_reading("你好", "NIHAO").map(|l| l.len()),
            Some(2),
            "two syllables, not five"
        );
    }

    #[test]
    fn syllables_are_joined_with_an_apostrophe_only_where_pinyin_writes_one() {
        assert_eq!(join_syllables(&["nán", "guò"]), "nánguò");
        assert_eq!(join_syllables(&["quán", "guó"]), "quánguó");
        assert_eq!(join_syllables(&["nǚ", "ér"]), "nǚ'ér");
        assert_eq!(join_syllables(&["kě", "ài"]), "kě'ài");
        assert_eq!(join_syllables(&["dì", "èr"]), "dì'èr");
        assert_eq!(join_syllables(&["wǎn", "ān"]), "wǎn'ān");
        assert_eq!(join_syllables(&["hěn", "ài"]), "hěn'ài");
        assert_eq!(join_syllables(&["Tiān", "ān", "mén"]), "Tiān'ānmén");
        assert_eq!(join_syllables(&["É", "yǔ"]), "Éyǔ");
        // An erhua `r` is written onto the syllable before it.
        assert_eq!(join_syllables(&["zhè", "r"]), "zhèr");
        assert_eq!(join_syllables(&["yī", "diǎn", "r"]), "yīdiǎnr");
        // A leading vowel needs nothing in front of it, and blanks are skipped.
        assert_eq!(join_syllables(&["ài", "hào"]), "àihào");
        assert_eq!(join_syllables(&["", "ān", " "]), "ān");
        assert_eq!(join_syllables::<&str>(&[]), "");
    }

    #[test]
    fn a_joined_reading_splits_back_into_the_syllables_it_was_joined_from() {
        // The join and the split are two halves of one rule, so every pair of
        // initial-less and coda-ending syllables that could run together has to
        // come back apart.
        let cases: &[&[&str]] = &[
            &["nán", "guò"],
            &["quán", "guó"],
            &["hěn", "ài"],
            &["wǒ", "ài", "nǐ"],
            &["nǐ", "è"],
            &["xī", "ān"],
            &["píng", "ān"],
            &["fāng", "àn"],
            &["fǎn", "gǔ"],
            &["rén", "gōng"],
            &["qù", "nián"],
            &["nǚ", "rén"],
            &["ān", "rán"],
            &["ér", "nǚ"],
            &["zhōng", "guó", "rén"],
            &["bàn", "gōng", "shì"],
        ];
        for parts in cases {
            let joined = join_syllables(parts);
            assert_eq!(split(&joined), *parts, "{joined}");
        }
    }

    #[test]
    fn tones_are_read_from_the_marks() {
        assert_eq!(tones("hǎo"), [3]);
        assert_eq!(tones("shì"), [4]);
        assert_eq!(tones("xuéxí"), [2, 2]);
        assert_eq!(tones("lǜ"), [4]);
        assert_eq!(tones("nǐhǎo"), [3, 3]);
        assert_eq!(tones("māma"), [1, 5], "no mark on the second is neutral");
        assert_eq!(tones("péngyou"), [2, 5]);
    }

    #[test]
    fn an_unreadable_reading_is_refused() {
        assert!(syllables("").is_none());
        assert!(syllables("   ").is_none());
        // No vowel at all: a consonant run, not a syllable.
        assert!(syllables("xyz").is_none());
        // Two marks in one run is two syllables run together, which is not one.
        assert!(syllables("hǎohǎo").is_none() || split("hǎohǎo").len() == 2);
    }

    #[test]
    fn tone_from_pinyin_reads_one_syllable_and_refuses_more() {
        assert_eq!(tone_from_pinyin("hǎo"), Some(3));
        assert_eq!(tone_from_pinyin("shì"), Some(4));
        assert_eq!(tone_from_pinyin("lǜ"), Some(4));
        assert_eq!(tone_from_pinyin("de"), Some(5));
        // Several syllables: a different question, refused rather than halved.
        assert_eq!(tone_from_pinyin("nǐhǎo"), None);
        assert_eq!(tone_from_pinyin("xuexi"), None);
        assert_eq!(tone_from_pinyin("nǐ hǎo"), None);
        assert_eq!(tone_from_pinyin(""), None);
    }

    #[test]
    fn a_reading_aligns_one_syllable_per_character() {
        // The pairing a tone colour reads: one syllable per character, in order.
        let aligned = aligned_reading("学习", "xuéxí").expect("two and two");
        let readings: Vec<&str> = aligned.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(readings, ["xué", "xí"]);
        let tones: Vec<u8> = aligned.iter().map(|s| s.tone).collect();
        assert_eq!(tones, [2, 2]);

        // Neutral syllables are carried, not dropped: 的 is `de`, and 的话 is
        // `dehuà` — the case a split driven by tone marks alone gets wrong.
        let aligned = aligned_reading("的话", "dehuà").expect("two and two");
        assert_eq!(aligned.iter().map(|s| s.tone).collect::<Vec<_>>(), [5, 4]);

        // Apostrophes are boundaries, so a caller composing readings still aligns.
        let aligned = aligned_reading("一天", "yī'tiān").expect("two and two");
        assert_eq!(aligned.iter().map(|s| s.tone).collect::<Vec<_>>(), [1, 1]);
    }

    #[test]
    fn a_reading_that_does_not_divide_one_per_character_is_refused() {
        // The refusal that makes colouring safe: a misaligned reading is not
        // paired up anyway, because that would put a tone on the wrong glyph.
        assert!(aligned_reading("学习", "xué").is_none(), "one syllable, two characters");
        assert!(aligned_reading("学习", "xuéxíxí").is_none(), "three for two");
        assert!(aligned_reading("学习", "").is_none());
        assert!(aligned_reading("", "xué").is_none());
        // A character the reading cannot account for — punctuation, or a glyph
        // with no reading — leaves the two out of step, and that is refused too.
        assert!(aligned_reading("你好吗", "nǐhǎo").is_none());
        // A run that is not a syllable at all is refused by `syllables`, not here.
        assert!(aligned_reading("xy", "xyz").is_none());
    }

    #[test]
    fn third_tone_before_third_tone_becomes_second() {
        let chars: Vec<char> = "你好".chars().collect();
        assert_eq!(spoken_tones(&chars, &[3, 3]), [2, 3]);

        // A run shortens all but its last.
        let chars: Vec<char> = "我很好".chars().collect();
        assert_eq!(spoken_tones(&chars, &[3, 3, 3]), [2, 2, 3]);

        // A single third tone, and a pair broken by another tone, are untouched.
        let chars: Vec<char> = "好".chars().collect();
        assert_eq!(spoken_tones(&chars, &[3]), [3]);
        let chars: Vec<char> = "很高".chars().collect();
        assert_eq!(spoken_tones(&chars, &[3, 1]), [3, 1]);
    }

    #[test]
    fn bu_becomes_second_before_a_fourth_tone_only() {
        let chars: Vec<char> = "不是".chars().collect();
        assert_eq!(spoken_tones(&chars, &[4, 4]), [2, 4]);

        // Before anything else it stays fourth.
        let chars: Vec<char> = "不好".chars().collect();
        assert_eq!(spoken_tones(&chars, &[4, 3]), [4, 3]);

        // And a 不 that is not tone 4 in the dictionary is left alone.
        let chars: Vec<char> = "不".chars().collect();
        assert_eq!(spoken_tones(&chars, &[4]), [4]);
    }

    #[test]
    fn yi_changes_with_what_follows_it() {
        // Before a fourth tone it is second.
        let chars: Vec<char> = "一个".chars().collect();
        assert_eq!(spoken_tones(&chars, &[1, 4]), [2, 4]);
        // Before anything else it is fourth.
        let chars: Vec<char> = "一天".chars().collect();
        assert_eq!(spoken_tones(&chars, &[1, 1]), [4, 1]);
        let chars: Vec<char> = "一起".chars().collect();
        assert_eq!(spoken_tones(&chars, &[1, 3]), [4, 3]);
        // Alone or final, it stays first.
        let chars: Vec<char> = "一".chars().collect();
        assert_eq!(spoken_tones(&chars, &[1]), [1]);
        let chars: Vec<char> = "第一".chars().collect();
        assert_eq!(spoken_tones(&chars, &[4, 1]), [4, 1], "final 一 is untouched");
    }

    #[test]
    fn a_target_carries_both_readings_and_the_sandhi_sentence() {
        let target = tone_target("你好", "nǐhǎo").expect("你好 reads as two syllables");
        assert_eq!(target.spoken(), [2, 3]);
        assert!(target.sandhi_applied);
        assert_eq!(target.syllables[0].citation, 3);
        assert_eq!(target.syllables[0].spoken, 2);
        assert_eq!(target.syllables[0].reading, "nǐ");
        assert_eq!(target.syllables[0].ch, '你');
        assert!(
            target.detail.contains("2 + 3") && target.detail.contains("3 + 3"),
            "the sentence must give both readings: {}",
            target.detail
        );

        // A word with no sandhi says so quietly.
        let plain = tone_target("学习", "xuéxí").expect("学习 reads as two syllables");
        assert_eq!(plain.spoken(), [2, 2]);
        assert!(!plain.sandhi_applied);
        assert!(plain.detail.contains("2 + 2"), "{}", plain.detail);
    }

    #[test]
    fn a_target_refuses_a_mismatch_between_syllables_and_characters() {
        // Two characters, one syllable: the reading cannot be aligned, so it is
        // refused rather than scored against the wrong character.
        assert!(tone_target("学习", "xué").is_none());
        assert!(tone_target("学习", "").is_none());
        assert!(tone_target("", "xué").is_none());
        // Three syllables for two characters, likewise.
        assert!(tone_target("学习", "xuéxíxí").is_none());
    }

    #[test]
    fn a_reading_of_particles_is_still_a_target() {
        // 的 is the neutral tone. It used to be refused here — "real, and not
        // something to score" — which left the most common character in the
        // language as the one the tone panel would not look at. Neutral tones
        // are scored now, on being level.
        let target = tone_target("的", "de").expect("a neutral reading is a target");
        assert_eq!(target.spoken(), vec![5]);
        assert_eq!(target.scorable(), 1);
        assert!(!target.sandhi_applied);
        assert!(target.detail.contains("neutral"), "{}", target.detail);

        // A word that mixes them keeps both, and both count.
        assert_eq!(tone_target("妈妈", "māma").map(|t| t.spoken()), Some(vec![1, 5]));
        assert_eq!(tone_target("妈妈", "māma").map(|t| t.scorable()), Some(2));
    }

    #[test]
    fn sandhi_does_not_apply_across_an_apostrophe_free_join() {
        // 一 + 天 as composed readings, joined with an apostrophe by the caller,
        // still splits and still takes sandhi.
        let target = tone_target("一天", "yī'tiān").expect("joined readings still split");
        assert_eq!(target.spoken(), [4, 1]);
        assert_eq!(target.syllables.len(), 2);
    }

    #[test]
    fn a_base_reading_drops_the_tone_but_not_the_umlaut() {
        assert_eq!(base("nǐ"), "ni");
        assert_eq!(base("hǎo"), "hao");
        assert_eq!(base("shì"), "shi");
        assert_eq!(base("de"), "de");
        // 嗯 is `ń`: a syllable with no vowel letter at all.
        assert_eq!(base("ń"), "n");
        // The reason `ü` is not folded into `u`: 女 nǚ and 努 nǔ are different
        // syllables, and calling them the same sound would call a wrong syllable
        // right.
        assert_eq!(base("nǚ"), "nv");
        assert_eq!(base("nǔ"), "nu");
        assert_ne!(base("nǚ"), base("nǔ"));
    }

    #[test]
    fn hearing_what_was_asked_for_says_so() {
        let target = tone_target("你好", "nǐhǎo").expect("你好 is a target");
        let heard = heard_against("你好", "nǐhǎo", &target);
        assert!(heard.all_matched());
        assert_eq!(heard.matched, 2);
        assert_eq!(heard.base, "ni hao");
        assert_eq!(heard.text, "你好");
        assert!(heard.same_count);
        // The wording has to keep the transcription and the tone apart: the
        // recogniser did not judge the tone and must not be presented as if it
        // had.
        assert!(heard.detail.contains("ni hao"), "{}", heard.detail);
        assert!(heard.detail.contains("not a judgement of the tone"), "{}", heard.detail);
    }

    #[test]
    fn a_homophone_is_the_same_syllable_and_a_different_one_is_not() {
        // 是 and 事 are both `shì`. A recogniser picking the other character has
        // still heard the right syllable, which is why the comparison is by
        // reading and not by character — this is the whole reason `heard_against`
        // exists.
        let target = tone_target("是", "shì").expect("是 is a target");
        let homophone = heard_against("事", "shì", &target);
        assert!(homophone.all_matched(), "{}", homophone.detail);
        assert_ne!(homophone.text, "是");

        // sì and shì are different syllables, and that is what a learner needs
        // to be told.
        let target = tone_target("四", "sì").expect("四 is a target");
        let wrong = heard_against("是", "shì", &target);
        assert!(!wrong.all_matched());
        assert_eq!(wrong.matched, 0);
        assert_eq!(wrong.syllables[0].base, "shi");
        assert_eq!(wrong.syllables[0].wanted, "si");
        // The reading keeps its tone mark so the character is identifiable; the
        // *comparison* is still on `base`, which has none.
        assert_eq!(wrong.syllables[0].reading, "shì");
        assert_eq!(wrong.syllables[0].wanted_reading, "sì");
        assert!(wrong.detail.contains("shì where si was asked for"), "{}", wrong.detail);
    }

    #[test]
    fn a_syllable_can_match_the_sound_at_a_different_tone() {
        // `mǎ` asked for, `mā` heard: `matches` is true because the sound is the
        // same, and both readings keep their marks so the interface can show which
        // one the model actually produced — the difference the sound comparison
        // deliberately cannot see.
        let target = tone_target("马", "mǎ").expect("马 is a target");
        let heard = heard_against("妈", "mā", &target);
        assert!(heard.syllables[0].matches, "ma and ma are the same sound");
        assert_eq!(heard.syllables[0].base, "ma");
        assert_eq!(heard.syllables[0].wanted, "ma");
        assert_eq!(heard.syllables[0].reading, "mā");
        assert_eq!(heard.syllables[0].wanted_reading, "mǎ");
    }

    #[test]
    fn a_syllable_heard_differently_keeps_its_dictionary_tone_mark() {
        // `cong` alone is ambiguous between 从, 葱 and 匆. The tone mark is what
        // identifies the character the model wrote, which is the one thing a
        // learner needs in order to act on "it heard a different syllable".
        let target = tone_target("中", "zhōng").expect("中 is a target");
        let heard = heard_against("从", "cóng", &target);
        assert_eq!(heard.matched, 0);
        assert_eq!(heard.syllables[0].base, "cong");
        assert_eq!(heard.syllables[0].reading, "cóng");
        assert!(
            heard.detail.contains("cóng where zhong was asked for"),
            "{}",
            heard.detail
        );
    }

    #[test]
    fn a_syllable_that_matched_is_not_given_a_tone_mark() {
        // 妈 `mā` where 马 `mǎ` was asked for: the sound `ma` matched, so nothing
        // is claimed about the tone of the learner's voice. The model's own
        // reading still carries its tone, and that is exactly what the sentence
        // must not print on a match.
        let target = tone_target("马", "mǎ").expect("马 is a target");
        let heard = heard_against("妈", "mā", &target);
        assert!(heard.all_matched());
        assert_eq!(heard.syllables[0].reading, "mā");
        assert!(
            heard.detail.contains("Heard ma:"),
            "the matched syllable is not tone-marked: {}",
            heard.detail
        );
    }

    #[test]
    fn the_wrong_number_of_syllables_is_reported_rather_than_aligned() {
        let target = tone_target("你好", "nǐhǎo").expect("你好 is a target");
        let heard = heard_against("你", "nǐ", &target);
        assert!(!heard.same_count);
        assert!(!heard.all_matched());
        assert!(heard.detail.contains("2 syllables"), "{}", heard.detail);
        assert!(heard.detail.contains("ni"), "{}", heard.detail);
    }

    #[test]
    fn a_transcription_that_cannot_be_split_is_refused() {
        // Two characters, one syllable: comparing them in order would report a
        // syllable the learner never said. The same refusal `tone_target` makes.
        let target = tone_target("你好", "nǐhǎo").expect("你好 is a target");
        let heard = heard_against("你好", "nǐ", &target);
        assert!(!heard.same_count);
        assert!(heard.syllables.is_empty());
        assert!(heard.detail.contains("could not be divided"), "{}", heard.detail);
    }

    #[test]
    fn hearing_nothing_is_said_plainly_and_leaves_the_tone_alone() {
        let target = tone_target("是", "shì").expect("是 is a target");
        let heard = heard_against("", "", &target);
        assert!(!heard.all_matched());
        assert!(heard.text.is_empty());
        assert!(heard.detail.contains("Nothing was recognised"), "{}", heard.detail);
        assert!(heard.detail.contains("tone"), "{}", heard.detail);
    }

    #[test]
    fn a_transcription_is_read_against_readings_with_no_tone_target() {
        // Six characters, so no target exists — but the readings wanted are
        // still known, which is the whole point of the general form.
        let wanted: Vec<String> = ["xiè", "xie", "nǐ", "de", "bāng", "zhù"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let heard = heard_against_readings("谢谢你的帮助", "xièxienǐdebāngzhù", &wanted, false);
        assert_eq!(heard.syllables.len(), 6);
        assert!(heard.same_count);
        assert!(heard.all_matched(), "{}", heard.detail);
        assert!(
            !heard.detail.contains("tone below"),
            "with no tone panel under it the sentence must not point at one: {}",
            heard.detail
        );
    }

    #[test]
    fn a_transcription_with_nothing_to_read_against_is_still_reported() {
        // The dataset could not read one of the characters, so there is no
        // wanted reading to compare with. The transcription is still the answer.
        let heard = heard_against_readings("你好", "nǐhǎo", &[], false);
        assert_eq!(heard.text, "你好");
        assert_eq!(heard.base, "ni hao");
        assert!(heard.syllables.is_empty());
        assert!(!heard.same_count);
        assert!(!heard.detail.contains("tone below"), "{}", heard.detail);
        assert!(heard.detail.contains("not compared"), "{}", heard.detail);
    }

    #[test]
    fn heard_against_and_the_reading_form_agree() {
        // The tone path is the reading path with the target's own readings, so
        // the two must not be able to drift apart.
        let target = tone_target("四", "sì").expect("四 is a target");
        let wanted: Vec<String> = target.syllables.iter().map(|s| s.reading.clone()).collect();
        assert_eq!(
            heard_against("是", "shì", &target),
            heard_against_readings("是", "shì", &wanted, true)
        );
    }

    #[test]
    fn splitting_keeps_each_syllables_place_in_the_reading() {
        let spans = syllables_with_spans("xuéxí").expect("xuéxí splits");
        assert_eq!(
            spans
                .iter()
                .map(|(syllable, span)| (syllable.text.as_str(), span.clone()))
                .collect::<Vec<_>>(),
            vec![("xué", 0..3), ("xí", 3..5)]
        );
        // The same syllable twice is the case searching for its text gets wrong.
        let spans = syllables_with_spans("xuexue").expect("xuexue splits");
        assert_eq!(spans[0].1, 0..3);
        assert_eq!(spans[1].1, 3..6);
        // Separators are not part of any syllable, and the offsets are counted
        // through them.
        let spans = syllables_with_spans("xī'ān").expect("xī'ān splits");
        assert_eq!(spans[0].1, 0..2);
        assert_eq!(spans[1].1, 3..5);
        assert!(syllables_with_spans("  ").is_none());
    }

    #[test]
    fn a_tone_mark_lands_where_pinyin_puts_it() {
        assert_eq!(mark_syllable("xue", 2).as_deref(), Some("xué"));
        assert_eq!(mark_syllable("hao", 3).as_deref(), Some("hǎo"));
        // No `a`, `o` or `e`, so the last vowel takes it: `iu` marks the `u` and
        // `ui` marks the `i`, which is the pair a learner gets wrong by hand.
        assert_eq!(mark_syllable("liu", 4).as_deref(), Some("liù"));
        assert_eq!(mark_syllable("dui", 4).as_deref(), Some("duì"));
        // `o` beats `e` when there is no `a`.
        assert_eq!(mark_syllable("xiong", 2).as_deref(), Some("xióng"));
        // `v` is the way a plain keyboard writes `ü`.
        assert_eq!(mark_syllable("nv", 3).as_deref(), Some("nǚ"));
        assert_eq!(mark_syllable("nü", 3).as_deref(), Some("nǚ"));
        // Tapping a second tone changes the mark rather than stacking one.
        assert_eq!(mark_syllable("xué", 4).as_deref(), Some("xuè"));
        // A neutral tone is the syllable with no mark at all.
        assert_eq!(mark_syllable("xué", 5).as_deref(), Some("xue"));
        // 嗯, whose syllable has no vowel letter in it.
        assert_eq!(mark_syllable("n", 3).as_deref(), Some("ň"));
        assert_eq!(mark_syllable("n", 2), None, "there is no tone-2 nasal");
        // Nothing a mark can sit on, and a tone that is not a tone.
        assert_eq!(mark_syllable("", 1), None);
        assert_eq!(mark_syllable("x", 1), None);
        assert_eq!(mark_syllable("xue", 0), None);
        assert_eq!(mark_syllable("xue", 6), None);
    }

    #[test]
    fn a_tone_key_marks_the_syllable_the_cursor_is_in() {
        // Typing a syllable and tapping a tone marks what was just typed.
        assert_eq!(
            mark_tone_at("xuexi", 5, 2),
            Some(("xuexí".to_string(), 5))
        );
        // A cursor placed inside an earlier syllable marks that one.
        assert_eq!(
            mark_tone_at("xuexí", 2, 2),
            Some(("xuéxí".to_string(), 2))
        );
        // At a boundary the caret belongs to the syllable it ends, which is what
        // makes marking left to right work.
        assert_eq!(
            mark_tone_at("xuexi", 3, 1),
            Some(("xuēxi".to_string(), 3))
        );
        // Neutral takes a mark off again.
        assert_eq!(
            mark_tone_at("xuéxí", 5, 5),
            Some(("xuéxi".to_string(), 5))
        );
        // A cursor in no syllable at all, and text with no syllable in it.
        assert_eq!(mark_tone_at(" xue", 0, 2), None);
        assert_eq!(mark_tone_at("", 0, 2), None);
        // The mark is written and read back as the same tone, which is the whole
        // reason both halves live in this module.
        for tone in 1..=4u8 {
            let marked = mark_tone_at("ma", 2, tone).expect("ma can take a tone");
            assert_eq!(tone_from_pinyin(&marked.0), Some(tone), "{}", marked.0);
        }
    }

    fn spoken(text: &str, citation: &[u8]) -> Vec<u8> {
        let chars: Vec<char> = text.chars().collect();
        spoken_tones(&chars, citation)
    }

    #[test]
    fn yi_as_an_ordinal_keeps_its_first_tone() {
        // 第 makes 一 an ordinal, and an ordinal is not changed (issue #20).
        assert_eq!(spoken("第一次", &[4, 1, 4]), [4, 1, 4]);
        assert_eq!(spoken("第一天", &[4, 1, 1]), [4, 1, 1]);
        assert_eq!(spoken("第一线", &[4, 1, 4]), [4, 1, 4]);
        assert_eq!(spoken("第一手", &[4, 1, 3]), [4, 1, 3]);
    }

    #[test]
    fn yi_in_a_date_keeps_its_first_tone() {
        // 一月 is January and 一号 the first of the month, not "one month".
        assert_eq!(spoken("一月", &[1, 4]), [1, 4]);
        assert_eq!(spoken("一号", &[1, 4]), [1, 4]);
        assert_eq!(spoken("十一月", &[2, 1, 4]), [2, 1, 4]);
        // 日 is a date only after a month; on its own 一日 is "one day".
        assert_eq!(spoken("五月一日", &[3, 4, 1, 4]), [3, 4, 1, 4]);
        assert_eq!(spoken("日复一日", &[4, 4, 1, 4]), [4, 4, 2, 4]);
        assert_eq!(spoken("有朝一日", &[3, 1, 1, 4]), [3, 1, 2, 4]);
    }

    #[test]
    fn yi_among_digits_keeps_its_first_tone() {
        // Read as a digit: after another digit (十一, 二十一), or counting
        // before one (一二三, a year read digit by digit).
        assert_eq!(spoken("十一", &[2, 1]), [2, 1]);
        assert_eq!(spoken("二十一", &[4, 2, 1]), [4, 2, 1]);
        assert_eq!(spoken("一二三", &[1, 4, 1]), [1, 4, 1]);
        assert_eq!(spoken("一九", &[1, 3]), [1, 3]);
        // 一 counting a hundred, a thousand or ten thousand is a quantity, and
        // changes as it does before any other word.
        assert_eq!(spoken("一百", &[1, 3]), [4, 3]);
        assert_eq!(spoken("一千", &[1, 1]), [4, 1]);
        assert_eq!(spoken("一万", &[1, 4]), [2, 4]);
    }

    #[test]
    fn yi_between_a_reduplicated_verb_is_neutral() {
        // A一A: 看一看 is kàn yi kàn, the middle syllable light.
        assert_eq!(spoken("看一看", &[4, 1, 4]), [4, 5, 4]);
        assert_eq!(spoken("试一试", &[4, 1, 4]), [4, 5, 4]);
        // The neutral 一 breaks the run of thirds, so neither 想 changes.
        assert_eq!(spoken("想一想", &[3, 1, 3]), [3, 5, 3]);
        // 一一 is not a reduplication around 一; both are digits.
        assert_eq!(spoken("一一", &[1, 1]), [1, 1]);
    }

    #[test]
    fn the_sentence_says_no_change_where_yi_keeps_its_tone() {
        let ordinal = tone_target("第一次", "dìyīcì").expect("three and three");
        assert_eq!(ordinal.spoken(), [4, 1, 4]);
        assert!(!ordinal.sandhi_applied, "{}", ordinal.detail);
        assert_eq!(ordinal.detail, "Scored against 4 + 1 + 4.");

        let month = tone_target("一月", "yīyuè").expect("two and two");
        assert!(!month.sandhi_applied, "{}", month.detail);

        let look = tone_target("看一看", "kànyīkàn").expect("three and three");
        assert_eq!(look.spoken(), [4, 5, 4]);
        assert!(look.sandhi_applied);
        assert!(
            look.detail.contains("4 + neutral + 4") && look.detail.contains("4 + 1 + 4"),
            "{}",
            look.detail
        );
    }

    #[test]
    fn a_capitalised_tone_mark_carries_its_tone() {
        // A learner-typed proper noun is read the way a lower-case one is.
        assert_eq!(split("Àomén"), ["Ào", "mén"]);
        assert_eq!(tones("Àomén"), [4, 2]);
        assert_eq!(tones("Ōuzhōu"), [1, 1]);
        assert_eq!(tones("Éyǔ"), [2, 3]);
        assert_eq!(split("Ālābóyǔ"), ["Ā", "lā", "bó", "yǔ"]);
        assert_eq!(tones("Ālābóyǔ"), [1, 1, 2, 3]);
        assert_eq!(tones("ÀOMÉN"), [4, 2]);
        assert_eq!(tone_from_pinyin("Ǎ"), Some(3));
        // And folded the same way when the tone is set aside.
        assert_eq!(base("Àomén"), "aomen");
        assert_eq!(base("NǙ"), "nv");
    }

    #[test]
    fn a_capital_keeps_its_case_when_its_mark_is_written() {
        assert_eq!(mark_syllable("Ao", 4).as_deref(), Some("Ào"));
        assert_eq!(mark_syllable("Ào", 1).as_deref(), Some("Āo"));
        assert_eq!(mark_syllable("Ào", 5).as_deref(), Some("Ao"));
        assert_eq!(mark_syllable("NV", 3).as_deref(), Some("NǙ"));
        assert_eq!(mark_syllable("LIU", 2).as_deref(), Some("LIÚ"));
        assert_eq!(mark_tone_at("Aomen", 2, 4), Some(("Àomen".to_string(), 2)));
    }

    #[test]
    fn a_tone_number_is_read_as_the_mark_it_stands_for() {
        assert_eq!(split("ni3hao3"), ["nǐ", "hǎo"]);
        assert_eq!(tones("ni3hao3"), [3, 3]);
        assert_eq!(split("xue2xi2"), ["xué", "xí"]);
        assert_eq!(split("ma1ma5"), ["mā", "ma"]);
        assert_eq!(tones("ma1ma5"), [1, 5]);
        assert_eq!(split("zhong1 guo2"), ["zhōng", "guó"]);
        assert_eq!(split("ming2"), ["míng"]);
        // A mark-less syllable between numbered ones is still neutral.
        assert_eq!(tones("peng2you"), [2, 5]);
        // The number is a boundary, as an apostrophe is: `xi1an1` is 西安, and
        // `ping2an1` is 平安 rather than pín + gān.
        assert_eq!(split("xi1an1"), ["xī", "ān"]);
        assert_eq!(split("ping2an1"), ["píng", "ān"]);
        // `v` and `u:` are the keyboard's ways of writing `ü`.
        assert_eq!(split("nv3"), ["nǚ"]);
        assert_eq!(split("lu:4"), ["lǜ"]);
        assert_eq!(split("nü3er2"), ["nǚ", "ér"]);
        assert_eq!(split("NI3HAO3"), ["NǏ", "HǍO"]);
        assert_eq!(split("Lu:4"), ["Lǜ"]);
        assert_eq!(tone_from_pinyin("hao3"), Some(3));
        // A number that agrees with a mark is the same tone; one that does not
        // is two tones on one syllable, and refused.
        assert_eq!(split("hǎo3"), ["hǎo"]);
        assert!(syllables("hǎo4").is_none());
        // A number with no syllable before it is not a tone.
        assert!(syllables("3").is_none());
        assert!(syllables("ni33").is_none());
        assert!(syllables("ni6").is_none());
        // Sandhi is worked out from the numbered reading as from the marked one.
        let target = tone_target("你好", "ni3hao3").expect("two and two");
        assert_eq!(target.spoken(), [2, 3]);
        assert_eq!(target.syllables[0].reading, "nǐ");
    }

    #[test]
    fn a_numbered_reading_is_rewritten_with_marks() {
        assert_eq!(marked_reading("ni3hao3"), "nǐhǎo");
        assert_eq!(marked_reading("ni3 hao3"), "nǐ hǎo");
        assert_eq!(marked_reading("lu:4se4"), "lǜsè");
        assert_eq!(marked_reading("NV3"), "NǙ");
        // The apostrophe the number stood in for is written, so the marked
        // reading splits back the way the numbered one did.
        assert_eq!(marked_reading("xi1an1"), "xī'ān");
        assert_eq!(marked_reading("ping2an1"), "píng'ān");
        for reading in ["xi1an1", "ping2an1", "ni3hao3", "nü3er2", "Tian1an1men2"] {
            assert_eq!(
                split(&marked_reading(reading)),
                split(reading),
                "{reading} → {}",
                marked_reading(reading)
            );
        }
        // A marked or toneless reading is left exactly as typed, and so is
        // anything that cannot be read as pinyin at all.
        assert_eq!(marked_reading("xuéxí"), "xuéxí");
        assert_eq!(marked_reading("xue xi"), "xue xi");
        assert_eq!(marked_reading("hsk3"), "hsk3");
        assert_eq!(marked_reading("ni3hao3?"), "ni3hao3?");
        assert_eq!(marked_reading(""), "");
    }

    #[test]
    fn a_tone_key_rewrites_a_numbered_syllable() {
        // The number goes and the mark replaces it; the caret stays at the end
        // of the syllable it was at the end of.
        assert_eq!(mark_tone_at("ni3", 3, 2), Some(("ní".to_string(), 2)));
        assert_eq!(
            mark_tone_at("ni3hao3", 3, 2),
            Some(("níhao3".to_string(), 2))
        );
        // Marking `ping2` of `ping2an1` would leave `píngan1`, which reads as
        // pín + gān; the apostrophe keeps the boundary the number made.
        assert_eq!(
            mark_tone_at("ping2an1", 5, 3),
            Some(("pǐng'an1".to_string(), 4))
        );
        let marked = mark_tone_at("ping2an1", 5, 3).unwrap().0;
        assert_eq!(split(&marked), ["pǐng", "ān"]);
        // A number before the syllable still separates it, so none is added.
        assert_eq!(mark_tone_at("xi1an", 5, 1), Some(("xi1ān".to_string(), 5)));
    }
}
