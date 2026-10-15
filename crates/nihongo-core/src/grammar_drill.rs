//! The grammar drill: the course's own sentences with the pattern taken out.
//!
//! N19 imported the grammar — 526 patterns, their formation, their notes and 1,526
//! example sentences — and gave the learner a course to *read*. What it did not do
//! is ask anything back, which `ROADMAP_NIHONGO.md`'s known weak spots had named as
//! the next piece from the moment the screen shipped: *"grammar is presented and
//! searchable, but nothing asks the learner to produce or choose a pattern"*. This
//! module is the half that asks, and it is the counterpart of [`crate::drill`]: that
//! one weighs the kana pairs this learner confuses, this one weighs the patterns
//! they miss, and [`crate::drill::pick_weighted`] is the one rule both draw by.
//!
//! It asks the same question **two ways**, and the difference between them is the
//! difference between recognising a pattern and producing one:
//!
//! * **recognition** — [`question_for`] offers three patterns to choose between,
//!   with the answer among them and no fixed place for it;
//! * **production** — [`draw_production`] asks the learner to *write* the missing
//!   text, and [`verdict_for`] grades what they typed.
//!
//! Both are one question shape, one weighting rule and one file
//! ([`GrammarLog`]), because a miss is a miss whichever way the learner was asked —
//! invariant 29's rule, one dataset out.
//!
//! # The question, and why it is a cloze
//!
//! The corpus's own sentence is shown with one lump of the pattern taken out, and
//! the learner chooses which pattern fills it. Nothing is invented to build it: the
//! sentence, its English, the furigana over it and the options are all the
//! artifact's, and the answer is the point the sentence belongs to.
//!
//! # The rule the blank is found by, which is the measurement worth reading
//!
//! A pattern is *not* stored in the sentence it appears in: `〜に対して` is the
//! corpus's written form of a pattern whose sentence says に対して, and a pattern
//! like `何も〜ない` is two lumps with the verb between them. So the blank cannot be
//! marked in the data — it is found, by this rule:
//!
//! * the **fixed lumps** of a pattern are its `/` alternatives with the `〜` slots
//!   taken out, with each **bracketed group expanded both ways** — the corpus puts
//!   an optional element or an English gloss in brackets, so `〜上（に）` is 上 or
//!   上に, `〜あまり（に）` is あまり or あまりに and `〜まで（until）` is まで —
//!   and with each `〜`-separated fragment taken as well, because a pattern like
//!   `何も〜ない` is two lumps with the verb between them;
//! * the blank is the **longest** lump that occurs **exactly once** in the
//!   sentence, because a lump that occurs twice is a hole with two places to go;
//! * it is at least [`MIN_BLANK`] characters long;
//! * and it never **partially** covers a furigana span that carries a reading: a
//!   blank may take a kanji and its reading together, or sit inside a span that has
//!   no reading, but it may not leave half a ruby pair behind.
//!
//! Measured over the committed artifact at `MIN_BLANK = 3`: **363 of the 526
//! points** can be asked, over **903 sentences**. Reading the bracketed groups is
//! 20 of those points: without it `〜あまり（に）`'s lump would be `あまり（に）`,
//! which occurs in no Japanese sentence, and the drill would be silent about the
//! whole pattern in exactly the way a filter that matches nothing looks like a
//! filter that found nothing. Dropping the minimum to 2 adds more sentences and
//! would ask `私はマリア[__]。` for `〜です` — a question about an ending rather than
//! about a pattern — which is why 3 is the floor and not 2.
//! `tests/grammar_artifact.rs` pins the numbers, so the corpus moving is a
//! deliberate commit rather than a quietly different course.
//!
//! # What makes a distractor safe, and what makes it plausible
//!
//! **Safe** means no second option could be right, and the test for that is exact:
//! the removed text must not be one of a distractor's own lumps, because filling a
//! hole with the same text back is the only thing that reproduces the sentence the
//! corpus wrote. **34 of the 903 sentences** have such a point elsewhere in the
//! course (16 points are involved) and draw their options from the rest. A
//! distractor that merely *overlaps*
//! the blank is allowed, and deliberately so: `〜まで` beside `〜までに`, and
//! `〜たばかり` beside `〜ばかり`, are two of the mistakes worth asking about.
//!
//! **Plausible** means the two patterns a learner actually mixes up, and the corpus
//! stores two signals for it: a **shared tag** (its own grouping — `request`,
//! `condition`, `concession`) with a **similar length**, and the same **formation
//! head** — `〜なさい` and `〜かねる` are both `Verb-stem`, and a learner reaching for
//! the ending that fits a verb stem has both in mind. Measured over the artifact:
//! **28 of the 903** sentences have fewer than two distractors by tag and length
//! alone, and **none** do once the formation head is counted with it, so the second
//! signal is what keeps every question plausible rather than merely safe — the plain
//! safe remainder below them is a last resort no sentence in the shipped artifact
//! reaches. The options are the corpus's own patterns and nothing else: this drill
//! never writes a wrong answer to ask about.
//!
//! # And what production asks, which is the other half of asking
//!
//! [`production_for`] draws the same sentence with the same lump missing and asks
//! the learner to **write it**, in romaji or in kana. Three things about it are
//! decisions rather than implementation:
//!
//! * **The answer is the corpus's own reading of the hole, never a composed one.**
//!   A blank that takes 反して out of a sentence is answered `hanshite`, and the
//!   reading is the `rt` the corpus put over that kanji — the same rule a word card
//!   grades a typed reading by (invariant 21). [`Askable::blank_kana`] is where that
//!   is read, and it is why the blank rule's refusal to cut a ruby pair matters
//!   twice: a half-swapped pair has no reading to ask for.
//! * **Nothing is asked that cannot be answered.** The answer has to be something
//!   the input engine can produce, so [`romaji_for`](crate::input::romaji_for) —
//!   the reverse of `to_kana` — is the gate. Measured over the committed artifact:
//!   **every one of the 903** askable blanks is typeable, and the longest answer in
//!   the course is `ていただけませんか`. Production therefore reaches exactly the
//!   recognition drill's pool rather than a smaller one, and asks 363 of the 526
//!   patterns over 903 sentences.
//! * **The question carries nothing that answers it.** The recognition drill sends
//!   its answer because the answer is one of its three options; a question with no
//!   options does not have to, so `ProductionQuestion` is the sentence with the hole
//!   in it and the corpus's English and *nothing else* — no pattern, no meaning, no
//!   formation, no note, and neither the text taken out nor its reading. A test
//!   asserts it on the serialised bytes, so a field added later cannot quietly carry
//!   one. The lesson — the pattern, its reading, its meaning, its formation, the
//!   note, the text that was taken out and what it is read as — arrives with the
//!   verdict ([`ProductionVerdict`]) once the learner has answered.
//!
//! What production does *not* do is grade a hand or a whole sentence: the learner
//! types the lump the sentence needs, which is the attempt, and there is nothing
//! else to mark. A free sentence of the learner's own is a different exercise with a
//! different attempt source, and it is not built.
//!
//! # What is deliberately not here
//!
//! * **No schedule.** The review queue is over characters the board can grade
//!   (invariant 25), and a grammar point has no handwritten attempt behind it. What
//!   this keeps is the drill's own small weighting — `asked`, `correct`, `wrong` and
//!   [`crate::drill::MIN_WEIGHT`]'s floor — in **this app's own fourth file**
//!   (`grammar.json`, invariant 15), never in the schedule and never in the kana
//!   drill's file.
//! * **No level and no band.** Invariant 20/38's rule holds here unchanged: the
//!   corpus's levels are not in the artifact, so they cannot be shown, and this
//!   module could not sort a question by one if it wanted to.
//! * **No invented Japanese and no composed reading.** Every string a question
//!   shows is a field of the artifact, or a slice of one; a pattern's kana reading
//!   is shown only where the corpus stores one.
//! * **Nothing is spoken by this module.** The sentence itself may be — invariant
//!   37's exception — but only *after* the answer, because a voice reading the
//!   sentence before it is answered would speak the pattern that was taken out.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::drill::{pick_weighted, PairTally};
use crate::grammar::{GrammarDataset, GrammarExample, GrammarPoint};
use crate::words::Ruby;

/// The shortest blank a question may carry, in characters.
///
/// Three, and the number is measured rather than picked: dropping to two adds 159
/// askable sentences and asks about `〜です` and `〜ます` in sentences
/// whose ending the English gives away, which is a question about a conjugation
/// rather than about a pattern. See the module docs.
pub const MIN_BLANK: usize = 3;

/// How many options a question offers: the answer and two distractors.
///
/// Three rather than two, because a pattern is chosen from a course of 526 and a
/// coin flip teaches nothing; three rather than four, because each option is a whole
/// pattern (`〜なくてもいい`) and the row has to fit a phone. The number is what the
/// measurement supports: every askable sentence has at least two safe distractors.
pub const OPTION_COUNT: usize = 3;

/// How close in length a preferred distractor has to the answer, in characters.
pub const LENGTH_WINDOW: usize = 3;

/// A roll the caller supplies, made safe to index with: NaN becomes 0 and
/// anything outside `0..1` is clamped, so a caller's arithmetic cannot become a
/// panic or an empty question.
fn clamp_roll(roll: f64) -> f64 {
    if roll.is_nan() {
        0.0
    } else {
        roll.clamp(0.0, 1.0)
    }
}

/// One lump of a pattern's fixed text, and where it sits in a sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blank {
    /// The text taken out, exactly as the sentence writes it.
    pub text: String,
    /// Where it starts, in **characters** rather than bytes, so that a caller can
    /// slice the furigana spans without going near UTF-8 offsets.
    pub at: usize,
}

/// The fixed lumps of a pattern, longest first.
///
/// A stored pattern is not all pattern: the corpus writes an **optional** element
/// or an English gloss in brackets — `〜上（に）` is 上 or 上に, `〜あまり（に）` is
/// あまり or あまりに, and `〜まで（until）` is まで with its gloss, while
/// `〜（よ）うではないか` has an optional よ. So each bracketed group is expanded
/// both ways (present and absent, full-width or ASCII brackets), each resulting
/// text is stripped of its `〜` slots, and each `〜`-separated fragment is taken as
/// well — `何も〜ない` gives 何も and ない, because the verb sits between them.
///
/// Measured over the committed artifact: **35 of the 526** patterns carry a
/// bracketed group, and reading them this way is the difference between a course
/// of 343 askable patterns and one of 363 — without it, `〜あまり（に）`'s lump
/// would be `あまり（に）`, which occurs in no Japanese sentence, and the drill
/// would be silent about the whole pattern in exactly the way a filter that
/// matches nothing looks like a filter that found nothing (the trap
/// `HANDOVER_NIHONGO.md` records for the vocabulary).
pub fn lumps(pattern: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for alternative in pattern.split('/') {
        for variant in expand_brackets(alternative) {
            let whole: String = variant
                .chars()
                .filter(|ch| !matches!(ch, '〜' | '～' | ' ' | '　'))
                .collect();
            if !whole.is_empty() {
                out.push(whole);
            }
            for piece in variant.split(['〜', '～']) {
                let piece = piece.trim();
                if !piece.is_empty() {
                    out.push(piece.to_string());
                }
            }
        }
    }
    out.sort_by_key(|lump| std::cmp::Reverse(lump.chars().count()));
    out.dedup();
    out
}

/// One piece of a pattern with each bracketed group either kept or dropped.
///
/// Measured over the corpus, the bracketed groups are of exactly two kinds and the
/// split is clean: **four** of them hold a Japanese optional element — に, は, も and
/// よ, as in `〜上（に）` and `〜（よ）うではないか` — and every one of the other
/// **41** holds the corpus's English gloss, as in `〜まで（until）` and
/// `〜くらい / 〜ぐらい（approximately）`. A group with an ASCII letter in it is
/// therefore a gloss and only the reading without it is kept; any other group is an
/// optional element and both readings are. `〜上（に）` gives `上に` and `上`;
/// `〜まで（until）` gives `まで`; a pattern with no bracket gives itself. Groups do
/// not nest in this corpus, and a nested one would be read literally rather than
/// guessed at.
fn expand_brackets(alternative: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut group: Option<String> = None;
    for ch in alternative.chars() {
        match ch {
            '（' | '(' => match group.as_mut() {
                // A second opener inside a group: read it literally rather than
                // pretending to understand a nesting the corpus does not use.
                Some(inner) => inner.push(ch),
                None => group = Some(String::new()),
            },
            '）' | ')' => match group.take() {
                Some(inner) => {
                    // A gloss is not a reading of the pattern: `〜まで（until）`
                    // is まで, and `までuntil` would be a lump that occurs in no
                    // sentence. An optional element is the opposite, and both of
                    // its readings are pattern text.
                    let gloss = inner.chars().any(|ch| ch.is_ascii_alphabetic());
                    if !gloss {
                        let with: Vec<String> =
                            out.iter().map(|variant| format!("{variant}{inner}")).collect();
                        out.extend(with);
                    }
                }
                None => {
                    for variant in out.iter_mut() {
                        variant.push(ch);
                    }
                }
            },
            _ => match group.as_mut() {
                Some(inner) => inner.push(ch),
                None => {
                    for variant in out.iter_mut() {
                        variant.push(ch);
                    }
                }
            },
        }
    }
    if let Some(inner) = group {
        // An unclosed group is text, not an option.
        for variant in out.iter_mut() {
            variant.push_str(&inner);
        }
    }
    out
}

/// Each furigana span of an example as `(start, end, carries a reading)`, in
/// character offsets.
fn spans(example: &GrammarExample) -> Vec<(usize, usize, bool)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    for part in &example.ruby {
        let len = part.ruby.chars().count();
        out.push((at, at + len, part.rt.is_some()));
        at += len;
    }
    out
}

/// Whether a blank at `at` of `len` characters would cut a furigana span that
/// carries a reading in half.
///
/// A span *entirely inside* the blank is fine — the kanji and its reading come out
/// together — and so is any span that has no reading. What is refused is the
/// partial overlap, which would leave the learner a reading over nothing.
fn cuts_a_reading(example: &GrammarExample, at: usize, len: usize) -> bool {
    let end = at + len;
    spans(example).into_iter().any(|(start, stop, has_rt)| {
        has_rt && start < end && at < stop && !(at <= start && stop <= end)
    })
}

/// Where a lump occurs in a string, in character offsets.
fn occurrences(haystack: &str, needle: &str) -> Vec<usize> {
    let chars: Vec<char> = haystack.chars().collect();
    let needle: Vec<char> = needle.chars().collect();
    if needle.is_empty() || needle.len() > chars.len() {
        return Vec::new();
    }
    (0..=(chars.len() - needle.len()))
        .filter(|start| chars[*start..*start + needle.len()] == needle[..])
        .collect()
}

/// The blank an example can carry for a point, or `None` if the pattern cannot be
/// located in it under the module's rule.
pub fn blank_for(point: &GrammarPoint, example: &GrammarExample) -> Option<Blank> {
    for lump in lumps(&point.pattern) {
        let len = lump.chars().count();
        if len < MIN_BLANK {
            continue;
        }
        let hits = occurrences(&example.ja, &lump);
        if hits.len() != 1 {
            continue;
        }
        let at = hits[0];
        if !cuts_a_reading(example, at, len) {
            return Some(Blank { text: lump, at });
        }
    }
    None
}

/// One sentence of one point that a question can be built from.
#[derive(Clone, Debug)]
pub struct Askable<'a> {
    /// The point the sentence teaches — and so the answer to the question.
    pub point: &'a GrammarPoint,
    /// The sentence, with its own furigana.
    pub example: &'a GrammarExample,
    /// The lump of the pattern that comes out of it.
    pub blank: Blank,
}

impl Askable<'_> {
    /// The sentence's furigana before the blank, with the blank's span removed.
    pub fn before(&self) -> Vec<Ruby> {
        self.split().map(|(before, _)| before).unwrap_or_default()
    }

    /// The sentence's furigana after it.
    pub fn after(&self) -> Vec<Ruby> {
        self.split().map(|(_, after)| after).unwrap_or_default()
    }

    /// The spans on either side of the blank.
    ///
    /// A span that lies wholly inside the blank is dropped — taking a kanji out
    /// takes its reading with it — and a span with no reading that the blank
    /// straddles is split. [`blank_for`] has already refused a blank that would cut
    /// a reading, so the `None` this can still return is a guard against a caller
    /// building an [`Askable`] by hand: the screen is then simply not given a
    /// question rather than shown a sentence with half a ruby pair in it.
    pub fn split(&self) -> Option<(Vec<Ruby>, Vec<Ruby>)> {
        let at = self.blank.at;
        let end = at + self.blank.text.chars().count();
        let mut before = Vec::new();
        let mut after = Vec::new();
        let mut cursor = 0usize;
        for part in &self.example.ruby {
            let chars: Vec<char> = part.ruby.chars().collect();
            let start = cursor;
            let stop = cursor + chars.len();
            cursor = stop;
            if stop <= at {
                before.push(part.clone());
                continue;
            }
            if start >= end {
                after.push(part.clone());
                continue;
            }
            // Overlapping. A span carrying a reading may only be swallowed whole.
            if part.rt.is_some() && !(at <= start && stop <= end) {
                return None;
            }
            if start < at {
                before.push(Ruby {
                    ruby: chars[..at - start].iter().collect(),
                    rt: None,
                });
            }
            if stop > end {
                after.push(Ruby {
                    ruby: chars[end - start..].iter().collect(),
                    rt: None,
                });
            }
        }
        Some((before, after))
    }
}

/// Whether one point's pattern could fill another question's blank.
///
/// True when the removed text is one of `other`'s own lumps — the one case in which
/// two options are both right, because taking a hole out of a sentence and putting
/// the same text back is what reproduces it. Measured over the committed artifact:
/// **34 of the 903** askable sentences have such a point somewhere in the course (16
/// of the 526 points are involved), and the options for those are drawn from the
/// remaining 500-odd.
///
/// A distractor whose lump merely *overlaps* the blank is deliberately allowed: it
/// cannot reproduce the sentence, and those are often the mistakes worth asking
/// about — `〜まで` beside `〜までに`, or `〜たばかり` beside `〜ばかり`.
pub fn fills_the_same_hole(other: &GrammarPoint, blank: &Blank) -> bool {
    lumps(&other.pattern).contains(&blank.text)
}

/// Whether `other` may be offered as a wrong answer to this question.
///
/// The four ways it may not: it is the answer, its pattern is the answer's text, it
/// means the same thing (three meanings in the corpus are shared by more than one
/// point), or its own pattern could fill the same hole.
pub fn can_distract(answer: &Askable<'_>, other: &GrammarPoint) -> bool {
    other.id != answer.point.id
        && other.pattern != answer.point.pattern
        && other.meaning != answer.point.meaning
        && !fills_the_same_hole(other, &answer.blank)
}

/// Whether one pattern is close enough to another to be a *plausible* mistake.
///
/// The corpus's own tag and a similar length: `〜てください` and `〜ないでください`
/// are both `request`, and `〜たばかり` and `〜ばかり` are two characters apart. This
/// is the tightest signal the corpus stores for "these two are mixed up".
pub fn plausible(answer: &GrammarPoint, other: &GrammarPoint) -> bool {
    other.tags.iter().any(|tag| answer.tags.contains(tag))
        && other.pattern.chars().count().abs_diff(answer.pattern.chars().count()) <= LENGTH_WINDOW
}

/// What a pattern attaches to, as the corpus's `formation` states it: the token
/// before the first `+`, so `Verb-dict / Verb-te + いる` gives `Verb-dict / Verb-te`.
///
/// Coarse on purpose — `Noun` covers 166 of the 526 points — which is why it is a
/// *second* preference rather than the first.
pub fn formation_head(point: &GrammarPoint) -> String {
    point.formation.split('+').next().unwrap_or("").trim().to_string()
}

/// Whether two patterns attach to the same kind of thing.
///
/// The learner's other real mistake: `〜なさい` and `〜かねる` are both `Verb-stem`
/// and nothing alike in meaning, but a learner reaching for the one that fits a verb
/// stem has both in mind. Measured: **28 of the 903** askable sentences have fewer
/// than two plausible distractors by tag and length alone, and **none** do once this
/// is counted with it — which is why the two are the drill's first and second
/// preferences rather than one rule.
pub fn attaches_alike(answer: &GrammarPoint, other: &GrammarPoint) -> bool {
    let head = formation_head(answer);
    !head.is_empty() && head == formation_head(other)
}

/// Every sentence of one point a question can be built from, in the corpus's order.
pub fn askable_of(point: &GrammarPoint) -> Vec<Askable<'_>> {
    point
        .examples
        .iter()
        .filter_map(|example| {
            blank_for(point, example).map(|blank| Askable { point, example, blank })
        })
        .collect()
}

/// Every sentence of the whole course a question can be built from.
pub fn askable(dataset: &GrammarDataset) -> Vec<Askable<'_>> {
    dataset.points().iter().flat_map(askable_of).collect()
}

/// The points the drill can ask about: those with at least one sentence the rule
/// finds a blank in.
pub fn askable_points(dataset: &GrammarDataset) -> Vec<&GrammarPoint> {
    dataset.points().iter().filter(|point| !askable_of(point).is_empty()).collect()
}

/// What the drill can ask, for the screen's own line about it.
///
/// Three numbers rather than a promise: the screen says how much of the course can
/// be asked and over how many sentences, because 163 of the 526 patterns cannot be
/// — their pattern's own text is not in any of their sentences, or not uniquely —
/// and a learner who went looking for one of them should find that said rather than
/// find the drill silent about it. The same honesty as the character card's *"No
/// word this course teaches is written with 且"*.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DrillSize {
    /// How many of the course's points can be asked.
    pub points: usize,
    /// How many sentences questions can be built from.
    pub examples: usize,
    /// How many points the course holds at all.
    pub total: usize,
}

/// Measure the pool over a dataset.
pub fn size(dataset: &GrammarDataset) -> DrillSize {
    let askable = askable(dataset);
    DrillSize {
        points: askable.iter().map(|question| question.point.id.as_str()).collect::<BTreeSet<_>>().len(),
        examples: askable.len(),
        total: dataset.len(),
    }
}

impl Askable<'_> {
    /// The reading of the blank, from the corpus's own furigana.
    ///
    /// **Never composed** (invariant 21, one dataset over): [`blank_for`] has already
    /// refused a blank that cuts a span carrying a reading, so the blank is a run of
    /// whole spans — the `rt` of every span it swallows, and the surface of every
    /// plain span it covers, in the sentence's own order. For a blank with no kanji
    /// in it that is the blank itself, and for one that takes a kanji it is the
    /// reading the corpus put over that kanji (反して → はんして).
    pub fn blank_kana(&self) -> String {
        let at = self.blank.at;
        let end = at + self.blank.text.chars().count();
        let mut out = String::new();
        let mut cursor = 0usize;
        for part in &self.example.ruby {
            let chars: Vec<char> = part.ruby.chars().collect();
            let (start, stop) = (cursor, cursor + chars.len());
            cursor = stop;
            if stop <= at || start >= end {
                continue;
            }
            match &part.rt {
                // A span with a reading may only be swallowed whole — the blank rule
                // refuses the partial case — so all of its reading is in the blank.
                Some(reading) => out.push_str(reading),
                // A plain span the blank straddles contributes the part of it the
                // blank covers, which is the text itself: it has no other reading.
                None => {
                    let from = at.saturating_sub(start).min(chars.len());
                    let to = (end - start).min(chars.len());
                    out.extend(chars[from..to].iter());
                }
            }
        }
        out
    }
}

/// Whether this sentence can be asked by *production* rather than by recognition.
///
/// The question is the same one — the corpus's sentence with the pattern's own lump
/// taken out — but the learner writes the missing text instead of choosing it, so
/// the requirement the recognition drill does not have is that **the answer can be
/// typed at all**. The blank's own reading is what has to be typeable, and the
/// answer is the reading rather than the written form: a hole holding 反して is
/// answered `hanshite`, exactly as the word card asks for a word's reading rather
/// than its spelling (invariant 21). [`romaji_for`](crate::input::romaji_for) is the
/// authority, and a blank whose reading contains something the input engine cannot
/// produce — ー is the one that matters — is not asked rather than asked and
/// unanswerable.
pub fn producible(answer: &Askable<'_>) -> bool {
    crate::input::romaji_for(&answer.blank_kana()).is_some()
}

/// One sentence of one point a production question can be built from, and where it
/// sits in the point's own list.
///
/// The position travels with the sentence so that a screen can name which sentence
/// it was asked about **without being sent the sentence** — which matters here,
/// because the sentence contains the answer (see [`ProductionQuestion`]).
pub fn producible_of(point: &GrammarPoint) -> Vec<(usize, Askable<'_>)> {
    askable_of(point)
        .into_iter()
        .enumerate()
        .filter(|(_, answer)| producible(answer))
        .collect()
}

/// The points the production exercise can ask about: those with at least one
/// sentence whose blank can be typed.
pub fn producible_points(dataset: &GrammarDataset) -> Vec<&GrammarPoint> {
    dataset.points().iter().filter(|point| !producible_of(point).is_empty()).collect()
}

/// What the production exercise can ask, for the screen's own line about it.
///
/// The same three numbers [`size`] reports for the recognition drill, over this
/// exercise's own pool: it needs no distractors, so it reaches every sentence the
/// blank rule reaches whose answer is typeable.
pub fn production_size(dataset: &GrammarDataset) -> DrillSize {
    let mut points = 0usize;
    let mut examples = 0usize;
    for point in dataset.points() {
        let count = producible_of(point).len();
        if count > 0 {
            points += 1;
            examples += count;
        }
    }
    DrillSize { points, examples, total: dataset.len() }
}

/// A pattern's sentence with one lump of it taken out, to be **written** rather
/// than chosen.
///
/// The same question the recognition drill asks, drawn the same way — the sentence
/// is handed over in two halves with the corpus's own furigana on each, and the gap
/// between them is the question — with one difference that is the whole point of
/// it: **the payload carries nothing that answers it.** There is no pattern, no
/// meaning, no formation, no note, and neither the text taken out nor its reading,
/// so a screen cannot leak the answer because it has not been given one. What the
/// learner is asked for is the pattern the sentence takes, and the lesson — the
/// pattern, its reading, its meaning, its formation, its note and the sentence
/// whole again — arrives with the verdict ([`ProductionVerdict`]) once an answer has
/// been given. The recognition drill sends its answer because its answer is one of
/// its three options; this one has no options.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionQuestion {
    /// The answer's id, which the answer is recorded against.
    pub point: String,
    /// Where the sentence sits in the point's own list, so the answer can be
    /// recorded without the sentence travelling back over the IPC.
    pub example: usize,
    /// The sentence's furigana before the blank.
    pub before: Vec<Ruby>,
    /// …and after it.
    pub after: Vec<Ruby>,
    /// The English the corpus pairs with the sentence. The prompt: this is a
    /// production exercise about which pattern the sentence takes, and a learner who
    /// cannot yet read the sentence has no other way in.
    pub en: String,
}

/// What the corpus says the hole was, what the learner wrote, and the lesson.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionVerdict {
    /// The point the sentence belongs to — echoed, so a screen cannot record the
    /// verdict against a pattern it was not asked about.
    pub point: String,
    /// The pattern, as the course writes it.
    pub pattern: String,
    /// Its kana reading, where the corpus stores one.
    pub reading: Option<String>,
    /// What the pattern means.
    pub meaning: String,
    /// What it attaches to.
    pub formation: String,
    /// The corpus's note on nuance, where it has one.
    pub notes: Option<String>,
    /// The text the sentence wrote in the hole (反して), for the sentence whole again.
    pub written: String,
    /// That text's reading, from the corpus's own furigana — **the answer**.
    pub answer: String,
    /// What the learner's typing came to, in kana, for the screen to show back.
    pub produced: String,
    /// Whether it says the same thing: [`matches_kana_text`](crate::input::matches_kana_text).
    pub correct: bool,
    /// The sentence whole again **in kana**, the corpus's own reading of every part:
    /// what `Hear it` says once the answer is in.
    ///
    /// Not the sentence as written, and the difference is not cosmetic. The system
    /// voice is handed text and decides the reading itself, and Apple's Japanese
    /// front end prefers the noun compound over the sentence's own parse when both
    /// are available: `お腹痛いの？` is spoken `おふくつういの` — お + 腹痛(ふくつう) +
    /// a dangling い — because 腹痛 is a word and お腹+痛い is an ambiguity the
    /// analyser resolves the other way. Measured against the same voice on macOS:
    /// `お腹が痛いの？` and `おなかがいたいの？` render byte-identical audio, so an
    /// identical reading is the same sound exactly; `お腹痛いの？` is not that audio,
    /// and sits beside `おふくつういの？` rather than `おなかいたいの？`. The corpus's
    /// furigana is right, and this is that furigana spelled out, segment by segment,
    /// so the voice is asked to read a sound rather than to choose one.
    ///
    /// **It belongs to the verdict and not to [`ProductionQuestion`]** on that struct's
    /// own terms: the sentence read aloud contains the hole's reading, so a question
    /// carrying it would answer itself — see
    /// `a_production_question_carries_nothing_that_answers_it`.
    pub kana: String,
}

/// Build the question for one sentence, or `None` when its answer cannot be typed.
///
/// `example` is the sentence's position in its point's own list, and it travels from
/// the caller because a position is a fact about the point rather than about the
/// sentence.
pub fn production_for(example: usize, answer: &Askable<'_>) -> Option<ProductionQuestion> {
    if !producible(answer) {
        return None;
    }
    let (before, after) = answer.split()?;
    Some(ProductionQuestion {
        point: answer.point.id.clone(),
        example,
        before,
        after,
        en: answer.example.en.clone(),
    })
}

/// Grade a typed answer against the sentence's own hole, and say what the corpus
/// says the hole was.
///
/// **Correctness is decided here**, on the data the question was drawn from, and by
/// the same rule whether a screen or a test asks: the learner's typing is read by
/// the input engine, the blank's reading is the corpus's, and
/// [`matches_kana_text`](crate::input::matches_kana_text) compares them by sound.
/// `None` means the sentence is not one this exercise asks — a caller that asked
/// about something else gets no verdict rather than a wrong one.
pub fn verdict_for(
    point: &GrammarPoint,
    example: usize,
    typed: &str,
) -> Option<ProductionVerdict> {
    let answer = producible_of(point).into_iter().find(|(at, _)| *at == example)?.1;
    Some(ProductionVerdict {
        point: point.id.clone(),
        pattern: point.pattern.clone(),
        reading: point.reading.clone(),
        meaning: point.meaning.clone(),
        formation: point.formation.clone(),
        notes: point.notes.clone(),
        written: answer.blank.text.clone(),
        answer: answer.blank_kana(),
        produced: crate::input::produce_kana(typed),
        correct: crate::input::matches_kana_text(&answer.blank_kana(), typed),
        kana: answer.example.kana(),
    })
}

/// Draw the next production question: which pattern (weighted by what this learner
/// has missed, over this exercise's own pool) and which of its sentences.
///
/// Two rolls rather than three, because there are no options to shuffle. Both are
/// the caller's for the reason the recognition drill's are: every decision here is
/// assertable, and the command beside this passes rolls from the clock.
pub fn draw_production(
    dataset: &GrammarDataset,
    log: &GrammarLog,
    point_roll: f64,
    example_roll: f64,
) -> Option<ProductionQuestion> {
    let pool = producible_points(dataset);
    let picked = log.pick(&pool, point_roll)?;
    let start = pool.iter().position(|point| point.id == picked.id)?;
    // The drawn point first, then the rest in course order: a point all of whose
    // sentences are untypeable does not empty the exercise.
    for step in 0..pool.len() {
        let point = pool[(start + step) % pool.len()];
        let sentences = producible_of(point);
        if sentences.is_empty() {
            continue;
        }
        let first = ((clamp_roll(example_roll) * sentences.len() as f64) as usize)
            .min(sentences.len() - 1);
        for inner in 0..sentences.len() {
            let (at, answer) = &sentences[(first + inner) % sentences.len()];
            if let Some(question) = production_for(*at, answer) {
                return Some(question);
            }
        }
    }
    None
}

/// One pattern as an option.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarOption {
    /// The point's own id, which the answer is recorded against.
    pub id: String,
    /// The pattern as the course writes it, `〜` slots and all.
    pub pattern: String,
    /// Its kana reading, where the corpus stores one because the pattern itself is
    /// written with a kanji. Never composed here.
    pub reading: Option<String>,
}

impl GrammarOption {
    fn of(point: &GrammarPoint) -> Self {
        Self {
            id: point.id.clone(),
            pattern: point.pattern.clone(),
            reading: point.reading.clone(),
        }
    }
}

/// A pattern's sentence with one lump of the pattern taken out.
///
/// Everything a screen needs to draw the question and, once it is answered, to
/// explain it — the answer's own pattern, meaning, formation and note are all here,
/// exactly as [`crate::drill::DrillPair`] carries its `tell`. The interface decides
/// what to show when; the apparatus for the wrong answer is not a second round trip.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarQuestion {
    /// The answer's id.
    pub point: String,
    /// The sentence's furigana before the blank.
    pub before: Vec<Ruby>,
    /// …and after it.
    pub after: Vec<Ruby>,
    /// What was taken out — a lump of the answer's pattern, and not necessarily the
    /// whole of it: `〜の方が〜より` leaves `の方が` in the hole and keeps `より`
    /// where it stands.
    ///
    /// Sent so that the reveal can draw the sentence whole again, with the corpus's
    /// furigana still over the kanji around the gap. The answer's own pattern is in
    /// this payload as well, exactly as it is in the kana drill's — the screen
    /// decides what it shows and when, and this is not a second round trip.
    pub blank: String,
    /// The sentence as written, for the reveal once it is answered.
    pub ja: String,
    /// The same sentence in kana, the corpus's own reading of every part including
    /// the blank: what `Hear it` says once the question has been answered.
    ///
    /// **[`Self::ja`] is what the screen draws; this is what the voice is given.**
    /// The system voice chooses its own reading of kanji, and Apple's Japanese front
    /// end picks the noun compound over the sentence's own parse when both exist —
    /// `お腹痛いの？` comes out `おふくつういの`, お + 腹痛(ふくつう) + a dangling い.
    /// The corpus's furigana is not wrong; handing the voice the kanji is what lets
    /// it choose. See [`ProductionVerdict::kana`] for the measurement.
    pub kana: String,
    /// The English the corpus pairs with the sentence. Shown with the question:
    /// this is a drill about which pattern fits, not a reading test, and a learner
    /// who cannot yet read the sentence has no other way in.
    pub en: String,
    /// The answer's pattern.
    pub pattern: String,
    /// The answer's kana reading, where the corpus stores one.
    pub reading: Option<String>,
    /// What the answer means.
    pub meaning: String,
    /// What the answer attaches to.
    pub formation: String,
    /// The corpus's note on nuance, and on how it differs from a similar pattern.
    pub notes: Option<String>,
    /// Every option, the answer among them and in no fixed place.
    pub options: Vec<GrammarOption>,
}

/// The learner's record over the course: how each pattern this drill has asked
/// about went.
///
/// Keyed by the point's own id, which is stable in the corpus (`ni-taishite`), and
/// holding [`PairTally`] — the same type, weight rule and floor as the kana drill,
/// so the two cannot drift apart in what "seen", "missed" and "worth asking again"
/// mean. It is a **separate file** from the kana drill's for invariant 15's reason:
/// a mistake in writing one must not be able to reach the other, and a pattern id
/// is not a pair of kana.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GrammarLog {
    /// Keyed by [`GrammarPoint::id`]. Unknown keys — points a later corpus added,
    /// or a hand-edited file — are kept as they are rather than dropped.
    pub points: BTreeMap<String, PairTally>,
}

impl GrammarLog {
    /// An empty log: every pattern at its starting weight.
    pub fn new() -> Self {
        Self::default()
    }

    /// What this pattern's record is. An unknown id has no record, which is the
    /// same as never having been asked.
    pub fn tally(&self, id: &str) -> PairTally {
        self.points.get(id).copied().unwrap_or_default()
    }

    /// This pattern's weight, per the rule in [`crate::drill`].
    pub fn weight(&self, id: &str) -> f64 {
        self.tally(id).weight()
    }

    /// Record one answer against a pattern, and say what the record now is.
    pub fn record(&mut self, id: &str, correct: bool) -> PairTally {
        let tally = self.points.entry(id.to_string()).or_default();
        tally.record(correct);
        *tally
    }

    /// Whether anything has been recorded at all.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// How many patterns have a record — that is, how many have been asked.
    pub fn patterns_seen(&self) -> usize {
        self.points.len()
    }

    /// Which pattern to ask about next, weighted towards the ones this learner has
    /// missed.
    ///
    /// The draw is [`pick_weighted`], so it is the kana drill's arithmetic over a
    /// different pool, and `roll` is the caller's for the same reason: a test can
    /// name the answer it wants instead of waiting for it.
    pub fn pick<'a>(&self, pool: &'a [&'a GrammarPoint], roll: f64) -> Option<&'a GrammarPoint> {
        pick_weighted(pool, |point| self.weight(&point.id), roll).copied()
    }
}

/// Shuffle the options into a deterministic order from one roll.
///
/// In Rust rather than on the screen, because *where the right answer sits* is part
/// of the question rather than a detail of its drawing: a fixed place is a position
/// a learner can learn instead of a pattern, and a shuffle the spec pins can be
/// asserted on. A Fisher–Yates walked from a seeded linear congruential generator
/// is enough for three items and keeps the draw reproducible from the roll the
/// caller passed — the same property every other decision here has.
fn shuffle(options: &mut [GrammarOption], roll: f64) {
    let mut state = (clamp_roll(roll) * f64::from(u32::MAX)) as u64 ^ 0x9E37_79B9_7F4A_7C15;
    for i in (1..options.len()).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = ((state >> 33) as usize) % (i + 1);
        options.swap(i, j);
    }
}

/// Build the question for one sentence, or `None` when no two safe distractors can
/// be found for it.
///
/// The distractors come from the whole course in three tiers, taken in order: a
/// point that shares a **tag** and is close in length to the answer (the tightest
/// signal the corpus stores), then one that attaches to the same kind of thing
/// (`/Verb-stem/`), then anything else safe. `roll` says where in the tag list the
/// two are taken from — the lists themselves are in course order, so they are stable
/// — and it also seeds the shuffle of the three options.
pub fn question_for(
    answer: &Askable<'_>,
    dataset: &GrammarDataset,
    roll: f64,
) -> Option<GrammarQuestion> {
    let (before, after) = answer.split()?;
    let mut tagged: Vec<&GrammarPoint> = Vec::new();
    let mut attached: Vec<&GrammarPoint> = Vec::new();
    let mut rest: Vec<&GrammarPoint> = Vec::new();
    for other in dataset.points() {
        if !can_distract(answer, other) {
            continue;
        }
        if plausible(answer.point, other) {
            tagged.push(other);
        } else if attaches_alike(answer.point, other) {
            attached.push(other);
        } else {
            rest.push(other);
        }
    }
    let needed = OPTION_COUNT - 1;
    if tagged.len() + attached.len() + rest.len() < needed {
        return None;
    }

    // Walk the three tiers in order from `roll`, taking the first two ids that are
    // not already in hand. The roll only turns the tag list: the tiers below it are
    // used in course order, which is what keeps a fallback from being arbitrary.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    seen.insert(answer.point.id.clone());
    let mut chosen: Vec<&GrammarPoint> = Vec::new();
    let offset = if tagged.is_empty() {
        0
    } else {
        ((clamp_roll(roll) * tagged.len() as f64) as usize).min(tagged.len() - 1)
    };
    for candidate in tagged
        .iter()
        .cycle()
        .skip(offset)
        .take(tagged.len())
        .chain(attached.iter())
        .chain(rest.iter())
    {
        if chosen.len() >= needed {
            break;
        }
        if seen.insert(candidate.id.clone()) {
            chosen.push(candidate);
        }
    }
    if chosen.len() < needed {
        return None;
    }

    let mut options = vec![GrammarOption::of(answer.point)];
    options.extend(chosen.into_iter().map(GrammarOption::of));
    shuffle(&mut options, roll);

    Some(GrammarQuestion {
        point: answer.point.id.clone(),
        before,
        after,
        blank: answer.blank.text.clone(),
        ja: answer.example.ja.clone(),
        kana: answer.example.kana(),
        en: answer.example.en.clone(),
        pattern: answer.point.pattern.clone(),
        reading: answer.point.reading.clone(),
        meaning: answer.point.meaning.clone(),
        formation: answer.point.formation.clone(),
        notes: answer.point.notes.clone(),
        options,
    })
}

/// Draw the next question: which pattern (weighted), which of its sentences, and
/// which wrong answers, in which order.
///
/// All three rolls are the caller's, because all three are decisions this crate
/// wants assertable: the point at `0.0` is the first in course order and at `0.999`
/// the last, the sentence likewise, and the options turn over with the third. The
/// command beside this passes rolls from the clock.
pub fn draw(
    dataset: &GrammarDataset,
    log: &GrammarLog,
    point_roll: f64,
    example_roll: f64,
    option_roll: f64,
) -> Option<GrammarQuestion> {
    let pool = askable_points(dataset);
    let picked = log.pick(&pool, point_roll)?;
    let start = pool.iter().position(|point| point.id == picked.id)?;
    // The drawn point first, then the rest in course order: a sentence whose
    // distractors cannot be found is stepped over rather than drawn with two
    // options, and a point all of whose sentences did so does not empty the drill.
    for step in 0..pool.len() {
        let point = pool[(start + step) % pool.len()];
        let sentences = askable_of(point);
        if sentences.is_empty() {
            continue;
        }
        let first = ((clamp_roll(example_roll) * sentences.len() as f64) as usize)
            .min(sentences.len() - 1);
        for inner in 0..sentences.len() {
            let answer = &sentences[(first + inner) % sentences.len()];
            if let Some(question) = question_for(answer, dataset, option_roll) {
                return Some(question);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::GrammarDataset;

    fn dataset() -> GrammarDataset {
        GrammarDataset::from_gzip_bytes(include_bytes!("../data/grammar.bin.gz"))
            .expect("the committed grammar artifact decodes")
    }

    fn point<'a>(dataset: &'a GrammarDataset, id: &str) -> &'a GrammarPoint {
        dataset.get(id).unwrap_or_else(|| panic!("{id} is in the corpus"))
    }

    #[test]
    fn the_lumps_of_a_pattern_are_its_fixed_text_longest_first() {
        assert_eq!(lumps("〜に対して"), vec!["に対して"]);
        // A pattern whose two lumps have the verb between them keeps both, and the
        // concatenation is tried first — harmless where it does not occur, the
        // whole pattern where it does.
        assert_eq!(lumps("何も〜ない"), vec!["何もない", "何も", "ない"]);
        assert_eq!(lumps("〜の方が〜より"), vec!["の方がより", "の方が", "より"]);
        assert_eq!(lumps("〜くない / 〜かった"), vec!["くない", "かった"]);
        for lump in lumps("〜の方が〜より") {
            assert!(!lump.contains('〜'), "{lump} still carries a slot");
        }

        // A bracketed group is an optional element or a gloss, and both readings
        // are kept: 上 or 上に, あまり or あまりに, まで with its gloss dropped.
        assert_eq!(lumps("〜上（に）"), vec!["上に", "上"]);
        assert_eq!(lumps("〜あまり（に）"), vec!["あまりに", "あまり"]);
        // A gloss is dropped rather than kept as a reading: `までuntil` is not a
        // lump any Japanese sentence contains.
        assert_eq!(lumps("〜まで（until）"), vec!["まで"]);
        // And ASCII brackets are the same thing: the corpus uses both, and this is
        // an optional よ rather than a gloss.
        assert_eq!(lumps("〜(よ)うものなら"), vec!["ようものなら", "うものなら"]);
    }

    #[test]
    fn a_blank_is_a_lump_that_occurs_once_and_the_sentence_reassembles() {
        let dataset = dataset();
        let te = point(&dataset, "te-kudasai");
        let askable = askable_of(te);
        assert_eq!(askable.len(), 2, "two of 〜てください's three sentences can be asked");
        // The third says ぬいでください — the voiced form of the same ending — and
        // the rule cannot locate てください in it, so it is not asked rather than
        // asked with a guessed blank.
        let unaskable = te
            .examples
            .iter()
            .find(|example| blank_for(te, example).is_none())
            .expect("one sentence cannot be located");
        assert!(unaskable.ja.contains("でください"), "{}", unaskable.ja);
        assert!(!unaskable.ja.contains("てください"));

        for question in &askable {
            assert_eq!(question.blank.text, "てください");
            assert_eq!(question.example.ja.matches(&question.blank.text).count(), 1);
            let before: String = question.before().iter().map(|part| part.ruby.clone()).collect();
            let after: String = question.after().iter().map(|part| part.ruby.clone()).collect();
            assert_eq!(
                format!("{before}{}{after}", question.blank.text),
                question.example.ja,
                "the split lost or invented text"
            );
            assert!(!before.is_empty() || !after.is_empty());
        }
    }

    #[test]
    fn a_blank_that_would_cut_a_furigana_pair_is_refused() {
        let example = GrammarExample {
            ja: "私は学生です。".to_string(),
            en: "I am a student.".to_string(),
            ruby: vec![
                Ruby::new("私", Some("わたし".into())),
                Ruby::new("は", None),
                Ruby::new("学生", Some("がくせい".into())),
                Ruby::new("です。", None),
            ],
            words: Vec::new(),
        };
        let mut point = GrammarPoint {
            id: "cut".into(),
            pattern: "〜生です".into(),
            reading: None,
            romaji: String::new(),
            meaning: String::new(),
            formation: String::new(),
            notes: None,
            tags: Vec::new(),
            examples: vec![example.clone()],
        };
        // 生です starts inside 学生 and would leave 学 with a reading and nothing to
        // read: refused, even though the lump is three characters and occurs once.
        assert_eq!(blank_for(&point, &example), None);
        assert_eq!(occurrences(&example.ja, "生です"), vec![3]);

        // The same lump is fine when it takes the whole span with it — kanji and
        // reading out together — which is the case the rule exists to allow.
        point.pattern = "〜学生で〜".into();
        let blank = blank_for(&point, &example).expect("学生で takes the pair whole");
        assert_eq!(blank.text, "学生で");
        let askable = Askable { point: &point, example: &example, blank };
        let (before, after) = askable.split().expect("a split that keeps every reading");
        assert_eq!(before.len(), 2, "私 and は stay");
        assert_eq!(after[0].ruby, "す。", "the rest of です。 is kept");
        for part in before.iter().chain(after.iter()) {
            assert!(!part.ruby.is_empty(), "an empty span was invented");
        }
    }

    #[test]
    fn nothing_shorter_than_the_floor_is_asked() {
        let dataset = dataset();
        let desu = point(&dataset, "desu");
        assert_eq!(blank_for(desu, &desu.examples[0]), None, "〜です is a two-character blank");
        for question in askable(&dataset) {
            assert!(
                question.blank.text.chars().count() >= MIN_BLANK,
                "{} asked a {}-character blank",
                question.point.id,
                question.blank.text.chars().count()
            );
        }
    }

    #[test]
    fn a_distractor_may_not_say_exactly_what_was_taken_out() {
        let dataset = dataset();
        // The corpus stores 〜そうだ twice — appearance and hearsay — and both
        // lumps are そうだ. A question about one may not offer the other, because
        // either would reproduce the sentence.
        let appearance = point(&dataset, "sou-da-appearance");
        let hearsay = point(&dataset, "sou-da-hearsay");
        let askable = askable_of(appearance);
        let question = askable.first().expect("the appearance point can be asked");
        assert!(question.blank.text.contains("そう"));
        assert_eq!(occurrences(&question.example.ja, &question.blank.text).len(), 1);
        assert!(fills_the_same_hole(hearsay, &question.blank));
        assert!(!can_distract(question, hearsay));

        // While a point that merely overlaps the blank is allowed, and is often the
        // mistake worth asking about: 〜たばかり contains ばかり but cannot fill a
        // hole that ばかり came out of.
        let bakari = askable_of(point(&dataset, "bakari"));
        let bakari = bakari.first().expect("〜ばかり can be asked");
        assert!(can_distract(bakari, point(&dataset, "ta-bakari")));
        assert!(can_distract(bakari, point(&dataset, "koto-ga-dekiru")));
        assert!(!can_distract(bakari, bakari.point), "the answer is not its own distractor");
    }

    #[test]
    fn a_distractor_attaches_to_the_same_kind_of_thing() {
        // The second plausibility signal: 〜なさい and 〜かねる have nothing in common
        // but the stem they follow, and that is what a learner is choosing between.
        let dataset = dataset();
        let nasai = point(&dataset, "nasai");
        let kaneru = point(&dataset, "kaneru");
        assert_eq!(formation_head(nasai), "Verb-stem");
        assert_eq!(formation_head(kaneru), "Verb-stem");
        assert!(attaches_alike(nasai, kaneru));
        assert!(!attaches_alike(nasai, point(&dataset, "to-iu-named")), "Noun is other");
        // And a formation with alternatives takes its head from the first.
        assert_eq!(formation_head(point(&dataset, "te-kudasai")), "Verb-te");
        assert!(formation_head(point(&dataset, "koto-ga-dekiru")).starts_with("Verb-dict"));
    }

    #[test]
    fn every_askable_sentence_has_two_plausible_distractors() {
        // The claim the three tiers rest on, measured over the whole artifact: by tag
        // and length alone 28 sentences fall short, and with the formation head
        // counted too, none do.
        let dataset = dataset();
        let mut tagged_thin = 0usize;
        for answer in askable(&dataset) {
            let safe: Vec<&GrammarPoint> = dataset
                .points()
                .iter()
                .filter(|other| can_distract(&answer, other))
                .collect();
            let tagged = safe
                .iter()
                .filter(|other| plausible(answer.point, other))
                .count();
            let attached = safe
                .iter()
                .filter(|other| attaches_alike(answer.point, other))
                .count();
            if tagged < 2 {
                tagged_thin += 1;
            }
            assert!(
                tagged + attached >= 2,
                "{} has no pair of plausible distractors",
                answer.point.id
            );
        }
        assert_eq!(tagged_thin, 28, "28 sentences need the second signal");
    }

    #[test]
    fn every_question_offers_exactly_one_right_answer() {
        let dataset = dataset();
        let mut checked = 0usize;
        for answer in askable(&dataset) {
            let question = question_for(&answer, &dataset, 0.0)
                .unwrap_or_else(|| panic!("{} has no question at all", answer.point.id));
            assert_eq!(question.options.len(), OPTION_COUNT);
            let ids: BTreeSet<&str> = question.options.iter().map(|o| o.id.as_str()).collect();
            assert_eq!(ids.len(), OPTION_COUNT, "an option was offered twice");
            assert!(ids.contains(answer.point.id.as_str()), "the answer is not among the options");
            for option in &question.options {
                if option.id == answer.point.id {
                    continue;
                }
                let other = point(&dataset, &option.id);
                assert!(
                    !fills_the_same_hole(other, &answer.blank),
                    "{} says exactly what {}'s blank took out",
                    other.id,
                    answer.point.id
                );
                assert_ne!(other.meaning, answer.point.meaning, "two options mean the same thing");
                assert_ne!(other.pattern, answer.point.pattern);
            }
            checked += 1;
        }
        assert_eq!(checked, 903, "the askable set is the measured one");
    }

    #[test]
    fn the_pool_is_the_course_in_order_and_nothing_outside_it() {
        // The numbers this pool comes to are pinned in `tests/grammar_artifact.rs`,
        // where the rest of the committed artifact's counts live; what belongs here
        // is the shape: every pool member is a point of the course, the count is the
        // set of ids rather than a second walk that could disagree with it, and the
        // pool keeps the corpus's own order, so a heavier pattern does not move the
        // course around it.
        let dataset = dataset();
        let size = size(&dataset);
        let pool = askable_points(&dataset);
        assert_eq!(pool.len(), size.points);
        assert!(pool.iter().all(|point| dataset.position(&point.id).is_some()));
        let positions: Vec<usize> =
            pool.iter().filter_map(|point| dataset.position(&point.id)).collect();
        let mut sorted = positions.clone();
        sorted.sort_unstable();
        assert_eq!(positions, sorted, "the pool is in course order");
        assert_eq!(size.examples, askable(&dataset).len());
    }

    #[test]
    fn the_draw_is_weighted_by_what_this_learner_misses() {
        let dataset = dataset();
        let pool = askable_points(&dataset);
        let first = pool[0].id.clone();
        let second = pool[1].id.clone();

        // A fresh log draws by position: nothing is heavier than anything else.
        let fresh = GrammarLog::new();
        assert_eq!(fresh.pick(&pool, 0.0).map(|point| point.id.clone()), Some(first));
        assert_eq!(
            fresh.pick(&pool, 0.999).map(|point| point.id.clone()),
            Some(pool[pool.len() - 1].id.clone())
        );

        // A miss makes a pattern heavier; a right answer takes it back down, and
        // the floor holds however well it goes.
        let mut log = GrammarLog::new();
        log.record(&second, false);
        assert_eq!(log.weight(&second), 3.0);
        assert_eq!(log.record(&second, true), PairTally { asked: 2, correct: 1, wrong: 1 });
        assert_eq!(log.weight(&second), 2.0);
        assert_eq!(log.patterns_seen(), 1);
        for _ in 0..20 {
            log.record(&second, true);
        }
        assert_eq!(log.weight(&second), crate::drill::MIN_WEIGHT);
    }

    #[test]
    fn a_learner_who_misses_one_pattern_is_asked_it_more_often() {
        let dataset = dataset();
        let pool = askable_points(&dataset);
        let wanted = pool[5].id.clone();
        let mut log = GrammarLog::new();
        // A roll just past the first five patterns lands on the sixth, and once
        // that one has been missed twice its weight carries the same roll back to
        // it — the rule `nihongo_core::drill` states, over this pool.
        assert_ne!(log.pick(&pool, 0.02).map(|point| point.id.clone()), Some(wanted.clone()));
        log.record(&wanted, false);
        log.record(&wanted, false);
        assert_eq!(log.weight(&wanted), 5.0);
        assert_eq!(log.pick(&pool, 0.02).map(|point| point.id.clone()), Some(wanted));
    }

    #[test]
    fn the_question_is_drawn_from_the_corpus_and_the_rolls_change_it() {
        let dataset = dataset();
        let log = GrammarLog::new();
        let question = draw(&dataset, &log, 0.0, 0.0, 0.0).expect("a question");
        assert_eq!(question.point, askable_points(&dataset)[0].id);
        assert_eq!(question.options.len(), OPTION_COUNT);
        // The sentence really does contain the pattern it is asked about, in one of
        // the pattern's own lumps — the property the whole rule rests on.
        assert_eq!(
            question.before.iter().map(|part| part.ruby.as_str()).collect::<String>()
                + &question.blank
                + &question.after.iter().map(|part| part.ruby.as_str()).collect::<String>(),
            question.ja,
            "the three parts the payload carries are the sentence the corpus wrote"
        );
        assert!(
            lumps(&question.pattern).iter().any(|lump| question.ja.contains(lump)),
            "{} is not in {}",
            question.pattern,
            question.ja
        );
        assert!(!question.en.is_empty());
        assert!(!question.before.is_empty() || !question.after.is_empty());
        // And the sentence is handed over twice: as written, for the screen to draw, and
        // in kana — the blank's own reading included — for the voice. The second is what
        // `Hear it` sends, because the voice reads kanji its own way (お腹痛いの？ comes
        // out おふくつういの; see `GrammarQuestion::kana`).
        assert_ne!(question.kana, question.ja, "the sentence has a kanji to read");
        assert!(!question.kana.chars().any(crate::words::is_kanji), "{}", question.kana);

        // The same rolls are the same question, which is what makes any of this
        // assertable; a later point roll asks about a later point; and the option
        // roll changes the wrong answers.
        assert_eq!(draw(&dataset, &log, 0.0, 0.0, 0.0), Some(question.clone()));
        let later = draw(&dataset, &log, 0.999, 0.0, 0.0).expect("a question");
        assert_ne!(later.point, question.point);
        let other_options = draw(&dataset, &log, 0.0, 0.0, 0.6).expect("a question");
        assert_eq!(other_options.point, question.point);
        assert_ne!(other_options.options, question.options);

        // And an out-of-range or NaN roll is corrected rather than obeyed: 5.0 asks
        // about the last point, NaN about the first.
        let clamped = draw(&dataset, &log, 5.0, -1.0, f64::NAN).expect("a question");
        assert_eq!(clamped.point, askable_points(&dataset).last().unwrap().id);
        assert_eq!(draw(&dataset, &log, f64::NAN, 0.0, 0.0).map(|q| q.point), Some(question.point));
    }

    #[test]
    fn the_right_answer_is_not_always_in_the_same_place() {
        let dataset = dataset();
        let log = GrammarLog::new();
        let mut positions = BTreeSet::new();
        for step in 0..30 {
            let roll = f64::from(step) / 30.0;
            let question = draw(&dataset, &log, 0.0, 0.0, roll).expect("a question");
            let at = question
                .options
                .iter()
                .position(|option| option.id == question.point)
                .expect("the answer is among the options");
            positions.insert(at);
        }
        assert_eq!(positions.len(), OPTION_COUNT, "every place holds the answer over 30 rolls");
    }

    #[test]
    fn the_log_survives_a_round_trip_and_keeps_what_it_does_not_understand() {
        let mut log = GrammarLog::new();
        log.record("te-kudasai", false);
        log.record("te-kudasai", true);
        log.points.insert(
            "a-pattern-a-later-corpus-added".into(),
            PairTally { asked: 2, correct: 2, wrong: 0 },
        );

        let bytes = postcard::to_allocvec(&log).expect("serialises");
        let read_back: GrammarLog = postcard::from_bytes(&bytes).expect("deserialises");
        assert_eq!(read_back, log);
        assert_eq!(read_back.tally("te-kudasai"), PairTally { asked: 2, correct: 1, wrong: 1 });
        assert_eq!(read_back.patterns_seen(), 2, "unknown ids are kept");
        assert!(!read_back.is_empty());
    }

    #[test]
    fn a_grammar_id_is_not_a_kana_pair_key() {
        let dataset = dataset();
        // The two drill files must not be readable as one another: every grammar
        // key is a corpus id, and none of them is the two-spellings-between-one-bar
        // shape the kana drill stores under.
        for point in dataset.points() {
            assert!(!point.id.contains('|'), "{} looks like a kana pair key", point.id);
            assert!(crate::drill::split_key(&point.id).is_none(), "{}", point.id);
        }
    }

    #[test]
    fn an_empty_course_asks_nothing_rather_than_panicking() {
        let empty = GrammarDataset::default();
        assert_eq!(size(&empty).points, 0);
        assert_eq!(draw(&empty, &GrammarLog::new(), 0.5, 0.5, 0.5), None);
        assert_eq!(askable_points(&empty).len(), 0);
        assert!(draw(&dataset(), &GrammarLog::new(), 0.0, 0.0, 0.0).is_some());
        assert_eq!(production_size(&empty), DrillSize::default());
        assert_eq!(draw_production(&empty, &GrammarLog::new(), 0.5, 0.5), None);
        assert_eq!(producible_points(&empty).len(), 0);
    }

    /// Every character of a byte string that is a given needle.
    fn holds(haystack: &[u8], needle: &str) -> bool {
        !needle.is_empty() && haystack.windows(needle.len()).any(|window| window == needle.as_bytes())
    }

    #[test]
    fn the_blank_is_read_from_the_corpus_own_furigana() {
        let dataset = dataset();
        // A blank that is all kana reads as itself: 犬の方が猫より… takes の方が out.
        let yori = point(&dataset, "no-hou-ga-yori");
        let answer = askable_of(yori).into_iter().next().expect("〜の方が〜より can be asked");
        assert_eq!(answer.blank.text, "の方が");
        assert_eq!(answer.blank_kana(), "のほうが");
        // And a blank that takes a kanji takes the corpus's reading with it rather
        // than one composed here.
        let mut took_a_kanji = 0usize;
        for question in askable(&dataset) {
            if !question.blank.text.chars().any(crate::words::is_kanji) {
                continue;
            }
            took_a_kanji += 1;
            assert!(
                !question.blank_kana().chars().any(crate::words::is_kanji),
                "{}'s reading still holds a kanji",
                question.point.id
            );
            assert_ne!(question.blank_kana(), question.blank.text);
        }
        assert!(took_a_kanji > 0, "some blank of the course swallows a kanji");

        // Over the whole course the split is lossless in kana as well as in writing:
        // the sentence's own reading is the two halves' readings plus the blank's.
        let mut checked = 0usize;
        for question in askable(&dataset) {
            let kana_of = |parts: &[Ruby]| -> String {
                parts.iter().map(|part| part.rt.clone().unwrap_or_else(|| part.ruby.clone())).collect()
            };
            assert!(
                !question.blank_kana().is_empty(),
                "{} has a blank with no reading",
                question.point.id
            );
            assert_eq!(
                format!("{}{}{}", kana_of(&question.before()), question.blank_kana(), kana_of(&question.after())),
                question.example.kana(),
                "{}'s split lost or invented a reading",
                question.point.id
            );
            checked += 1;
        }
        assert_eq!(checked, 903, "the askable set is the measured one");
    }

    #[test]
    fn every_answer_the_production_exercise_asks_can_be_typed() {
        // The measurement this exercise rests on: **every** blank the rule finds has
        // a reading the input engine can produce, so production reaches the whole of
        // the recognition drill's pool — 363 of the 526 patterns over 903 sentences —
        // and refuses nothing. Longest answer: ていただけませんか, nine characters.
        let dataset = dataset();
        let mut refused = 0usize;
        let mut longest = String::new();
        for answer in askable(&dataset) {
            let kana = answer.blank_kana();
            if kana.chars().count() > longest.chars().count() {
                longest = kana.clone();
            }
            match crate::input::romaji_for(&kana) {
                Some(romaji) => assert!(
                    crate::input::matches_kana_text(&kana, &romaji),
                    "{kana} does not accept the spelling {romaji} the engine itself offered"
                ),
                None => refused += 1,
            }
        }
        assert_eq!(refused, 0, "no askable blank is untypeable");
        assert_eq!(longest, "ていただけませんか");
        assert_eq!(production_size(&dataset), DrillSize { points: 363, examples: 903, total: 526 });
        assert_eq!(production_size(&dataset), size(&dataset), "the same pool, asked differently");
    }

    #[test]
    fn a_production_question_carries_nothing_that_answers_it() {
        // The recognition drill hands its screen the answer, because the answer is
        // one of its three options; this one has no options, so it is not given the
        // pattern, the meaning, the text taken out or that text's reading — and the
        // property is asserted on the serialised question rather than on the fields,
        // so a field added later cannot quietly carry one of them.
        let dataset = dataset();
        let mut checked = 0usize;
        for (at, answer) in askable(&dataset).into_iter().enumerate() {
            let question = production_for(at, &answer).expect("every askable sentence can be produced");
            let bytes = postcard::to_allocvec(&question).expect("the question serialises");
            let kana = answer.blank_kana();
            // The three things that answer the question: the pattern, the text taken
            // out, and that text's reading. The *meaning* is not on the list — the
            // corpus's English prompt may happen to use the same words as its own
            // gloss (`kana-kashira`'s sentence is "I wonder…" and its meaning is "I
            // wonder") — and it needs no list entry either way, because no field of
            // this payload could carry it.
            for secret in [
                answer.point.pattern.as_str(),
                answer.blank.text.as_str(),
                kana.as_str(),
            ] {
                assert!(
                    !holds(&bytes, secret),
                    "{}'s question carries {secret:?}",
                    answer.point.id
                );
            }
            // The two halves really are the sentence with the hole between them, so
            // the screen can draw it and the reveal can put it back together.
            let before: String = question.before.iter().map(|part| part.ruby.as_str()).collect();
            let after: String = question.after.iter().map(|part| part.ruby.as_str()).collect();
            assert_eq!(
                format!("{before}{}{after}", answer.blank.text),
                answer.example.ja,
                "{}'s question lost or invented sentence text",
                answer.point.id
            );
            assert_eq!(question.point, answer.point.id);
            checked += 1;
        }
        assert_eq!(checked, 903);
    }

    #[test]
    fn a_verdict_is_decided_from_the_corpus_own_reading() {
        let dataset = dataset();
        let question = draw_production(&dataset, &GrammarLog::new(), 0.0, 0.0).expect("a question");
        let drawn = dataset.get(&question.point).expect("the drawn point is in the course");
        let answer = producible_of(drawn)
            .into_iter()
            .find(|(at, _)| *at == question.example)
            .expect("the drawn sentence can be produced")
            .1;
        let kana = answer.blank_kana();

        // Right in romaji, right in kana, and wrong — all three decided here.
        let romaji = crate::input::romaji_for(&kana).expect("the answer is typeable");
        let right = verdict_for(drawn, question.example, &romaji).expect("a verdict");
        assert!(right.correct);
        assert_eq!(right.answer, kana);
        assert_eq!(right.written, answer.blank.text);
        assert_eq!(right.pattern, drawn.pattern);
        assert_eq!(right.meaning, drawn.meaning);
        assert_eq!(right.formation, drawn.formation);
        assert_eq!(right.point, drawn.id);
        assert!(!right.produced.is_empty());
        // And the sentence whole again in kana, which is what the screen speaks once the
        // answer is in: the corpus's own readings with the hole's between them, and no
        // kanji left for the voice to parse (お腹痛いの？ → おふくつういの).
        assert_eq!(right.kana, answer.example.kana());
        assert!(!right.kana.chars().any(crate::words::is_kanji), "{}", right.kana);
        assert!(verdict_for(drawn, question.example, &kana).expect("a verdict").correct);
        let wrong = verdict_for(drawn, question.example, "chigau").expect("a verdict");
        assert!(!wrong.correct);
        assert_eq!(wrong.produced, "ちがう");
        // Input the engine cannot read is a wrong answer rather than a failure, and
        // a sentence this exercise does not ask gets no verdict at all.
        assert_eq!(verdict_for(drawn, question.example, "!!!").expect("a verdict").produced, "");
        let desu = point(&dataset, "desu");
        assert!(producible_of(desu).is_empty(), "〜です is below the blank floor");
        assert_eq!(verdict_for(desu, 0, "desu"), None);
        assert_eq!(verdict_for(drawn, usize::MAX, &romaji), None);
    }

    #[test]
    fn the_production_draw_is_weighted_and_the_rolls_change_it() {
        let dataset = dataset();
        let log = GrammarLog::new();
        let pool = producible_points(&dataset);
        let first = draw_production(&dataset, &log, 0.0, 0.0).expect("a question");
        assert_eq!(first.point, pool[0].id);
        assert_eq!(first.example, 0, "the first sentence of the first point");
        // The same rolls are the same question; a later point roll asks about a later
        // point and a later example roll about another of its sentences.
        assert_eq!(draw_production(&dataset, &log, 0.0, 0.0), Some(first.clone()));
        let later = draw_production(&dataset, &log, 0.999, 0.0).expect("a question");
        assert_ne!(later.point, first.point);
        let second_sentence =
            producible_of(dataset.get(&first.point).expect("in the course")).len() > 1;
        if second_sentence {
            let other = draw_production(&dataset, &log, 0.0, 0.999).expect("a question");
            assert_eq!(other.point, first.point);
            assert_ne!(other.example, first.example);
        }
        // Out-of-range and NaN rolls are corrected rather than obeyed.
        let clamped = draw_production(&dataset, &log, 5.0, -1.0).expect("a question");
        assert_eq!(clamped.point, pool[pool.len() - 1].id);
        assert_eq!(draw_production(&dataset, &log, f64::NAN, f64::NAN).map(|q| q.point), Some(first.point));

        // And a pattern this learner misses is asked more often. The weight is the
        // recognition drill's own, in the same file: a miss is a miss, whichever way
        // the learner was asked about the pattern.
        let mut missed = GrammarLog::new();
        let wanted = pool[5].id.clone();
        assert_ne!(missed.pick(&pool, 0.02).map(|p| p.id.clone()), Some(wanted.clone()));
        missed.record(&wanted, false);
        missed.record(&wanted, false);
        assert_eq!(missed.pick(&pool, 0.02).map(|p| p.id.clone()), Some(wanted));
    }
}
