//! Typing romaji and getting kana: the input side of the course.
//!
//! A learner who can recognise し still has to be able to *produce* it, and the
//! only keyboard most of them have is a Latin one. This module turns what they
//! type into kana, following the same rules a Japanese IME does, so that typing
//! practice and a real IME agree.
//!
//! The table is **built from the readings**, not written out a second time, so it
//! cannot drift from them: every spelling [`Reading::spellings`] offers becomes a
//! way to type that kana, and the yōon digraphs come from [`crate::yoon`].
//!
//! Three rules are not simple table lookups, and they are the three that make
//! romaji input feel like anything at all:
//!
//! * **The sokuon.** A doubled consonant is っ: `katta` is かった, `gakkou` is
//!   がっこう. `n` is excluded, because `nn` is a way of writing ん.
//! * **ん.** `n` before a consonant or at the end is ん; `n` before a vowel or
//!   `y` belongs to the syllable (`na`, `nya`). `n'` forces it, which is how
//!   `shin'ya` stays しんや and does not become しにゃ.
//! * **Small kana.** `x` or `l` prefixes the small form, as in a real IME:
//!   `xtu` is っ, `xya` is ゃ. Without this there is no way to type ぁ ぃ ぅ ぇ ぉ
//!   at all.
//!
//! Long vowels are deliberately **not** converted to ー: `ou` is おう, which is
//! what a learner writing hiragana wants. Choosing between おう and おー is a
//! spelling decision the course should teach, not one the input engine should
//! make silently.

use crate::curriculum::{to_hiragana, to_katakana, yoon, SMALL_KANA};
use crate::readings::{hiragana_with_readings, katakana_only_with_readings};
use crate::Script;

/// Why a romaji string could not be converted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RomajiError {
    /// Nothing in the table matched at this point in the input.
    Unknown { rest: String, at: usize },
}

impl std::fmt::Display for RomajiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RomajiError::Unknown { rest, at } => {
                write!(f, "cannot read {rest:?} as kana (at character {at})")
            }
        }
    }
}

impl std::error::Error for RomajiError {}

/// The `x`/`l` escapes for the small kana, as an IME uses.
const SMALL_ESCAPES: &[(&str, char)] = &[
    ("xa", 'ぁ'),
    ("xi", 'ぃ'),
    ("xu", 'ぅ'),
    ("xe", 'ぇ'),
    ("xo", 'ぉ'),
    ("xya", 'ゃ'),
    ("xyu", 'ゅ'),
    ("xyo", 'ょ'),
    ("xtu", 'っ'),
    ("xtsu", 'っ'),
    ("xwa", 'ゎ'),
    ("la", 'ぁ'),
    ("li", 'ぃ'),
    ("lu", 'ぅ'),
    ("le", 'ぇ'),
    ("lo", 'ぉ'),
    ("lya", 'ゃ'),
    ("lyu", 'ゅ'),
    ("lyo", 'ょ'),
    ("ltu", 'っ'),
    ("ltsu", 'っ'),
    ("lwa", 'ゎ'),
];

/// One way of typing one kana, or one yōon digraph, with both scripts' renderings.
struct Entry {
    romaji: String,
    hiragana: String,
    katakana: String,
}

/// Claim a spelling for a kana, unless an earlier kana already has it.
///
/// Where two kana share a spelling — じ and ぢ are both `ji`, ず and づ are both
/// `zu`, お and を are both `o` — the first to claim it keeps it. Insertion order
/// is code-point order, so the common kana wins: `ji` types じ, `zu` types ず,
/// `o` types お. The uncommon ones stay reachable as `di`, `du` and `wo`.
fn claim(pairs: &mut Vec<(String, String)>, romaji: &str, hiragana: &str) {
    if pairs.iter().any(|(r, _)| r == romaji) {
        return;
    }
    pairs.push((romaji.to_string(), hiragana.to_string()));
}

/// The whole input table, longest romaji first so that the longest match wins
/// (`kya` before `ka`, `sha` before `sa`).
///
/// There is one table, not one per script: each spelling carries both renderings,
/// and the caller picks. A spelling claimed by hiragana must still produce
/// katakana when katakana was asked for, which one shared entry gives for free.
fn table() -> Vec<Entry> {
    let mut pairs: Vec<(String, String)> = Vec::new();

    // The base kana and the dakuten, in code-point order so the common reading of
    // a shared spelling wins.
    //
    // The small kana are skipped deliberately. They sit *before* their full-size
    // counterparts in code-point order — ぁ is U+3041 and あ is U+3042 — so
    // leaving them in would have ぁ claim `a` before あ could, and every vowel
    // would type small. They are reachable through the `x`/`l` escapes below,
    // which is how a real IME does it.
    for (hiragana, reading) in hiragana_with_readings() {
        if SMALL_KANA.contains(&hiragana) {
            continue;
        }
        for spelling in reading.spellings() {
            claim(&mut pairs, spelling, &hiragana.to_string());
        }
    }

    // The v-series has no hiragana, so both renderings are the katakana. Typing
    // `va` in a hiragana context gives ヷ, which is what Japanese actually does.
    for (kana, reading) in katakana_only_with_readings() {
        for spelling in reading.spellings() {
            claim(&mut pairs, spelling, &kana.to_string());
        }
    }

    // The yōon digraphs: two kana from one spelling. Only the hiragana side is
    // listed; the katakana is derived below.
    for digraph in yoon(Script::Hiragana) {
        for spelling in [digraph.hepburn.as_str(), digraph.kunrei.as_str()] {
            claim(&mut pairs, spelling, &digraph.display);
        }
    }

    // The small-kana escapes.
    for (romaji, kana) in SMALL_ESCAPES {
        claim(&mut pairs, romaji, &kana.to_string());
    }

    let mut entries: Vec<Entry> = pairs
        .into_iter()
        .map(|(romaji, hiragana)| {
            let katakana = hiragana
                .chars()
                .map(|ch| to_katakana(ch).unwrap_or(ch))
                .collect();
            Entry { romaji, hiragana, katakana }
        })
        .collect();

    // Longest first, so `kya` is tried before `ka` and `sha` before `sa`.
    entries.sort_by(|a, b| {
        b.romaji
            .len()
            .cmp(&a.romaji.len())
            .then_with(|| a.romaji.cmp(&b.romaji))
    });
    entries
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'i' | 'u' | 'e' | 'o')
}

/// How many characters of `rest` the ん rule claims, or `None` if this `n` starts
/// an ordinary syllable.
fn n_rule(rest: &str) -> Option<usize> {
    let mut chars = rest.chars();
    if chars.next() != Some('n') {
        return None;
    }
    match chars.next() {
        // `n'` is the explicit separator.
        Some('\'') => Some(2),
        // `nn` is ん plus whatever follows.
        Some('n') => Some(1),
        // A bare final `n`.
        None => Some(1),
        // Before a vowel or `y` the `n` belongs to the syllable: na, ni, nya.
        Some(c) if is_vowel(c) || c == 'y' => None,
        // Before anything else it is ん: shinbun, konnichiwa's `n` before `n`.
        Some(_) => Some(1),
    }
}

/// How many characters the sokuon rule claims, or `None` if this is not a
/// doubled consonant.
fn sokuon_rule(rest: &str) -> Option<usize> {
    let mut chars = rest.chars();
    let first = chars.next()?;
    // `n` is excluded: `nn` is ん, handled above.
    if first == 'n' || is_vowel(first) || !first.is_ascii_alphabetic() {
        return None;
    }
    if chars.next() == Some(first) {
        Some(1)
    } else {
        None
    }
}

/// Convert romaji to kana in the given script.
///
/// Returns the kana, or the point at which the input stopped being readable.
pub fn to_kana_in(script: Script, input: &str) -> Result<String, RomajiError> {
    let lowered = input.trim().to_ascii_lowercase();
    let entries = table();
    let mut out = String::new();
    let mut rest = lowered.as_str();
    let mut consumed = 0usize;

    while !rest.is_empty() {
        // っ first: a doubled consonant is always the sokuon, never a syllable.
        if sokuon_rule(rest) == Some(1) {
            out.push(match script {
                Script::Hiragana => 'っ',
                Script::Katakana => 'ッ',
            });
            rest = &rest[1..];
            consumed += 1;
            continue;
        }

        if let Some(take) = n_rule(rest) {
            out.push(match script {
                Script::Hiragana => 'ん',
                Script::Katakana => 'ン',
            });
            rest = &rest[take..];
            consumed += take;
            continue;
        }

        match entries.iter().find(|e| rest.starts_with(&e.romaji)) {
            Some(entry) => {
                let kana = match script {
                    Script::Hiragana => &entry.hiragana,
                    Script::Katakana => &entry.katakana,
                };
                out.push_str(kana);
                rest = &rest[entry.romaji.len()..];
                consumed += entry.romaji.len();
            }
            None => {
                return Err(RomajiError::Unknown {
                    rest: rest.to_string(),
                    at: consumed,
                })
            }
        }
    }

    Ok(out)
}

/// Convert romaji to hiragana.
pub fn to_kana(input: &str) -> Result<String, RomajiError> {
    to_kana_in(Script::Hiragana, input)
}

/// True when `typed` is one of the ways of writing `kana` — the check an input
/// exercise needs, so that `si` is accepted for し just as `shi` is.
///
/// A yōon digraph is accepted as a whole: `kya` matches きゃ, and neither `ki`
/// nor `ya` alone does.
pub fn matches_reading(kana: char, typed: &str) -> bool {
    let typed = typed.trim().to_ascii_lowercase();
    crate::readings::reading(kana)
        .map(|r| r.spellings().any(|s| s == typed))
        .unwrap_or(false)
}

/// True when `typed` romaji spells the given kana string, converting through
/// [`to_kana`]. This is the check for a whole word rather than one character.
pub fn matches_word(word: &str, typed: &str) -> bool {
    to_kana(typed).map(|produced| produced == word).unwrap_or(false)
}

/// The small kana and the yōon a base kana can form, so an input exercise can
/// offer the right next keystrokes.
pub fn continuations(script: Script) -> Vec<(String, String)> {
    yoon(script)
        .into_iter()
        .map(|y| (y.hepburn, y.display))
        .collect()
}

/// The hiragana equivalent of a katakana string, character for character, leaving
/// anything hiragana already alone. Useful for comparing a typed answer with a
/// katakana prompt.
pub fn normalise_to_hiragana(input: &str) -> String {
    input
        .chars()
        .map(|ch| to_hiragana(ch).unwrap_or(ch))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kana(input: &str) -> String {
        to_kana(input).unwrap_or_else(|e| panic!("{input}: {e}"))
    }

    #[test]
    fn plain_gojuon_round_trips() {
        assert_eq!(kana("a"), "あ");
        assert_eq!(kana("ki"), "き");
        assert_eq!(kana("tsu"), "つ");
        assert_eq!(kana("n"), "ん");
        assert_eq!(kana("kana"), "かな");
        assert_eq!(kana("nani"), "なに");
    }

    #[test]
    fn both_romanisations_type_the_same_kana() {
        for (typed, expected) in [
            ("shi", "し"),
            ("si", "し"),
            ("chi", "ち"),
            ("ti", "ち"),
            ("tsu", "つ"),
            ("tu", "つ"),
            ("fu", "ふ"),
            ("hu", "ふ"),
            ("ji", "じ"),
            ("zi", "じ"),
        ] {
            assert_eq!(kana(typed), expected, "{typed}");
        }
    }

    #[test]
    fn a_shared_spelling_gives_the_common_kana() {
        // じ before ぢ, ず before づ, お before を — which is what a typist wants.
        assert_eq!(kana("ji"), "じ");
        assert_eq!(kana("zu"), "ず");
        assert_eq!(kana("o"), "お");
        // And the uncommon ones stay reachable.
        assert_eq!(kana("di"), "ぢ");
        assert_eq!(kana("du"), "づ");
    }

    #[test]
    fn yoon_digraphs_are_one_entry_not_two_syllables() {
        assert_eq!(kana("kya"), "きゃ");
        assert_eq!(kana("kyu"), "きゅ");
        assert_eq!(kana("kyo"), "きょ");
        assert_eq!(kana("sha"), "しゃ");
        assert_eq!(kana("sya"), "しゃ");
        assert_eq!(kana("cha"), "ちゃ");
        assert_eq!(kana("tya"), "ちゃ");
        assert_eq!(kana("ja"), "じゃ");
        assert_eq!(kana("zya"), "じゃ");
        assert_eq!(kana("nyu"), "にゅ");
        // The plain kana must still work.
        assert_eq!(kana("kiya"), "きや", "きや is two syllables, not a digraph");
        assert_eq!(kana("ki"), "き");
    }

    #[test]
    fn the_sokuon_is_a_doubled_consonant() {
        assert_eq!(kana("katta"), "かった");
        assert_eq!(kana("gakkou"), "がっこう");
        assert_eq!(kana("nippon"), "にっぽん");
        assert_eq!(kana("zasshi"), "ざっし");
        // And it can be typed directly.
        assert_eq!(kana("xtu"), "っ");
        assert_eq!(kana("ltu"), "っ");
    }

    #[test]
    fn n_before_a_consonant_or_at_the_end_is_the_moraic_n() {
        assert_eq!(kana("shinbun"), "しんぶん");
        assert_eq!(kana("onna"), "おんな");
        assert_eq!(kana("kangae"), "かんがえ");
        assert_eq!(kana("hon"), "ほん");
    }

    #[test]
    fn n_before_a_vowel_belongs_to_the_syllable() {
        assert_eq!(kana("na"), "な");
        assert_eq!(kana("ni"), "に");
        assert_eq!(kana("nani"), "なに");
        assert_eq!(kana("nyan"), "にゃん");
    }

    #[test]
    fn the_apostrophe_forces_the_moraic_n() {
        // しんや (a different shop) against しにゃ, which is not a word.
        assert_eq!(kana("shin'ya"), "しんや");
        assert_eq!(kana("shinya"), "しにゃ");
        assert_eq!(kana("n'a"), "んあ");
    }

    #[test]
    fn small_kana_can_be_typed_through_the_escapes() {
        assert_eq!(kana("xa"), "ぁ");
        assert_eq!(kana("xya"), "ゃ");
        assert_eq!(kana("xtsu"), "っ");
        assert_eq!(kana("la"), "ぁ");
    }

    #[test]
    fn katakana_are_produced_in_the_katakana_script() {
        assert_eq!(to_kana_in(Script::Katakana, "kana").expect("converts"), "カナ");
        assert_eq!(to_kana_in(Script::Katakana, "kya").expect("converts"), "キャ");
        assert_eq!(to_kana_in(Script::Katakana, "katta").expect("converts"), "カッタ");
        assert_eq!(to_kana_in(Script::Katakana, "shinbun").expect("converts"), "シンブン");
        assert_eq!(to_kana_in(Script::Katakana, "va").expect("converts"), "ヷ");
    }

    #[test]
    fn long_vowels_are_spelled_out_rather_than_made_a_mark() {
        // おう, not おー: which one is right is a spelling lesson, not a rule the
        // input engine should apply behind the learner's back.
        assert_eq!(kana("ou"), "おう");
        assert_eq!(kana("oo"), "おお");
        assert_eq!(kana("toukyou"), "とうきょう");
    }

    #[test]
    fn unreadable_input_says_where_it_stopped() {
        let err = to_kana("kaxz").unwrap_err();
        assert_eq!(err, RomajiError::Unknown { rest: "xz".to_string(), at: 2 });
        assert!(to_kana("!!!").is_err());
    }

    #[test]
    fn input_is_trimmed_and_case_insensitive() {
        assert_eq!(kana("  KANA  "), "かな");
        assert_eq!(kana("Shi"), "し");
    }

    #[test]
    fn an_empty_input_is_an_empty_answer() {
        assert_eq!(kana(""), "");
        assert_eq!(kana("   "), "");
    }

    #[test]
    fn a_single_kana_is_matched_through_any_of_its_spellings() {
        assert!(matches_reading('し', "shi"));
        assert!(matches_reading('し', "SI"));
        assert!(matches_reading('ふ', "fu"));
        assert!(matches_reading('ふ', "hu"));
        assert!(!matches_reading('し', "chi"));
        assert!(!matches_reading('し', ""));
        // A digraph is not a single kana's reading.
        assert!(!matches_reading('き', "kya"));
        assert!(!matches_reading('あ', "kana"));
    }

    #[test]
    fn a_whole_word_can_be_checked() {
        assert!(matches_word("かな", "kana"));
        assert!(matches_word("しんぶん", "shinbun"));
        assert!(matches_word("がっこう", "gakkou"));
        assert!(!matches_word("かな", "kama"));
        assert!(!matches_word("かな", "nonsense!!"));
    }

    #[test]
    fn katakana_prompts_can_be_compared_against_a_hiragana_answer() {
        assert_eq!(normalise_to_hiragana("カナ"), "かな");
        assert_eq!(normalise_to_hiragana("かな"), "かな");
        assert_eq!(normalise_to_hiragana("ー"), "ー");
    }

    #[test]
    fn the_table_covers_every_spelling_the_readings_offer() {
        // The point of building the table from the readings is that it cannot
        // drift. Where two kana share a spelling the table gives the common one —
        // `ji` types じ, not ぢ — so the invariant is not "typing `ji` gives ぢ"
        // but "typing every offered spelling gives a kana that offers it".
        for (hiragana, reading) in hiragana_with_readings() {
            for spelling in reading.spellings() {
                let produced = kana(spelling);
                let produced_char = produced
                    .chars()
                    .next()
                    .unwrap_or_else(|| panic!("{spelling:?} produced nothing"));
                let accepted = crate::readings::reading(produced_char)
                    .map(|r| r.spellings().any(|s| s == spelling))
                    .unwrap_or(false);
                assert!(
                    accepted,
                    "typing {spelling:?} gave {produced}, whose reading does not include it \
                     (it was meant for {hiragana})"
                );
            }
        }
    }

    #[test]
    fn every_yoon_can_be_typed_by_both_of_its_spellings() {
        for script in [Script::Hiragana, Script::Katakana] {
            for y in yoon(script) {
                for spelling in [&y.hepburn, &y.kunrei] {
                    assert_eq!(
                        to_kana_in(script, spelling).expect("converts"),
                        y.display,
                        "{spelling} should give {}",
                        y.display
                    );
                }
            }
        }
    }

    #[test]
    fn continuations_offer_the_yoon() {
        let all = continuations(Script::Hiragana);
        assert_eq!(all.len(), 33);
        assert!(all.iter().any(|(romaji, kana)| romaji == "kyu" && kana == "きゅ"));
    }
}
