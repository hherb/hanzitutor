//! Typing romaji and getting kana: the input side of the course.
//!
//! A learner who can recognise し still has to be able to *produce* it, and the
//! only keyboard most of them have is a Latin one. This module turns what they
//! type into kana, following the same rules a Japanese IME does, so that typing
//! practice and a real IME agree. The IME followed is **Google Japanese Input**,
//! whose romaji table is published as `src/data/preedit/romanji-hiragana.tsv` in
//! the open-source Mozc; where this module's table and that one disagree, the
//! difference is named below.
//!
//! The table is **built from the readings**, not written out a second time, so it
//! cannot drift from them: every spelling [`Reading::spellings`] offers becomes a
//! way to type that kana, and the yōon digraphs come from [`crate::yoon`]. What
//! the readings cannot supply — the extended katakana a loanword is written with,
//! ファ, ティ, ヴァ, ウィ — is `EXTENDED`, copied from the Mozc table.
//!
//! Four rules are not simple table lookups, and they are the ones that make romaji
//! input feel like anything at all:
//!
//! * **The sokuon.** A doubled consonant is っ: `katta` is かった, `gakkou` is
//!   がっこう. `n` is excluded, because `nn` is a way of writing ん. Hepburn's `tch`
//!   is っ too, so `matcha` is まっちゃ and `kotchi` is こっち, as `cch` already was.
//! * **ん.** `n` before a consonant or at the end is ん; `n` before a vowel or
//!   `y` belongs to the syllable (`na`, `nya`). `n'` forces it, which is how
//!   `shin'ya` stays しんや and does not become しにゃ, and `nn` is ん **taking both
//!   letters**, which is what every IME teaches: `honn` is ほん, `konnnichiha` is
//!   こんにちは, and `kinnyoubi` is きんようび.
//! * **Small kana.** `x` or `l` prefixes the small form, as in a real IME:
//!   `xtu` is っ, `xya` is ゃ. Without this there is no way to type ぁ ぃ ぅ ぇ ぉ
//!   at all.
//! * **Word breaks.** A space or a hyphen between words is skipped, so the
//!   corpus's own `wake de wa nai` types わけでわない; and it ends the word, so an
//!   `n` before one is ん — `hon o` is ほんお, where `hono` is ほの. An IME would
//!   make a hyphen ー; this one does not, for the reason the next paragraph gives.
//!
//! Long vowels are deliberately **not** converted to ー: `ou` is おう, which is
//! what a learner writing hiragana wants. Choosing between おう and おー is a
//! spelling decision the course should teach, not one the input engine should
//! make silently.
//!
//! ## Typing an answer is not the same as typing
//!
//! [`to_kana`] has to give one answer, and it gives the IME's. But a learner
//! answering a question has an answer to be compared with, and three things a
//! real typist does are right by sound while the IME reads them otherwise:
//!
//! * the particles は, を and へ are said `wa`, `o` and `e`, and the corpus's
//!   romaji spells them so;
//! * Hepburn writes んな as `nna` (`onna`, `konnichiwa`), which the IME reads as
//!   んあ — the IME wants `onnna`;
//! * Hepburn writes ティ, ディ, トゥ, ドゥ and ウォ as `ti`, `di`, `tu`, `du` and
//!   `wo`, which the IME reads as ち, ぢ, つ, づ and を.
//!
//! So [`check_kana_text`] does not convert and then compare. It reads the typing
//! **against** the expected answer, trying every way the romaji could be read, and
//! accepts it when one of them is the answer. Each leniency is tied to a position
//! in the answer: `wa` is right where the answer has は and wrong where it has
//! anything else, so `ha` for わ, `wo` for お and `he` for え are all still wrong.

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

/// The spellings Google Japanese Input has and the readings do not: mostly the
/// extended katakana, as two kana from one spelling, written in hiragana with the
/// katakana derived exactly as for the yōon.
///
/// Copied from Mozc's `romanji-hiragana.tsv`, every entry of it whose output is
/// kana and which the readings and the yōon do not already give, with two
/// omissions: `xka` and `xke`, which Mozc maps to the katakana ヵ and ヶ rather
/// than to anything a hiragana answer holds. Six spellings here are ones the
/// readings also claim, and here the IME wins, because a learner's IME does: `va`
/// `vi` `ve` `vo` are ヴァ ヴィ ヴェ ヴォ rather than the archaic ヷ ヸ ヹ ヺ, and `wi`
/// `we` are ウィ and ウェ rather than ゐ and ゑ, which are typed `wyi` and `wye`.
/// The archaic four have no spelling left, as in the IME — [`romaji_for`] says so,
/// and a question is not asked that needs them.
const EXTENDED: &[(&str, &str)] = &[
    // v
    ("va", "ゔぁ"),
    ("vi", "ゔぃ"),
    ("ve", "ゔぇ"),
    ("vo", "ゔぉ"),
    ("vya", "ゔゃ"),
    ("vyi", "ゔぃ"),
    ("vyu", "ゔゅ"),
    ("vye", "ゔぇ"),
    ("vyo", "ゔょ"),
    // The yōon's -i and -e forms.
    ("kyi", "きぃ"),
    ("kye", "きぇ"),
    ("gyi", "ぎぃ"),
    ("gye", "ぎぇ"),
    ("she", "しぇ"),
    ("syi", "しぃ"),
    ("sye", "しぇ"),
    ("je", "じぇ"),
    ("jya", "じゃ"),
    ("jyi", "じぃ"),
    ("jyu", "じゅ"),
    ("jye", "じぇ"),
    ("jyo", "じょ"),
    ("zyi", "じぃ"),
    ("zye", "じぇ"),
    ("che", "ちぇ"),
    ("tyi", "ちぃ"),
    ("tye", "ちぇ"),
    ("cya", "ちゃ"),
    ("cyi", "ちぃ"),
    ("cyu", "ちゅ"),
    ("cye", "ちぇ"),
    ("cyo", "ちょ"),
    ("dya", "ぢゃ"),
    ("dyi", "ぢぃ"),
    ("dyu", "ぢゅ"),
    ("dye", "ぢぇ"),
    ("dyo", "ぢょ"),
    ("nyi", "にぃ"),
    ("nye", "にぇ"),
    ("hyi", "ひぃ"),
    ("hye", "ひぇ"),
    ("byi", "びぃ"),
    ("bye", "びぇ"),
    ("pyi", "ぴぃ"),
    ("pye", "ぴぇ"),
    ("myi", "みぃ"),
    ("mye", "みぇ"),
    ("ryi", "りぃ"),
    ("rye", "りぇ"),
    // ts, th, dh, tw, dw
    ("tsa", "つぁ"),
    ("tsi", "つぃ"),
    ("tse", "つぇ"),
    ("tso", "つぉ"),
    ("thi", "てぃ"),
    ("tha", "てゃ"),
    ("thu", "てゅ"),
    ("the", "てぇ"),
    ("tho", "てょ"),
    ("t'i", "てぃ"),
    ("t'yu", "てゅ"),
    ("dhi", "でぃ"),
    ("dha", "でゃ"),
    ("dhu", "でゅ"),
    ("dhe", "でぇ"),
    ("dho", "でょ"),
    ("d'i", "でぃ"),
    ("d'yu", "でゅ"),
    ("twu", "とぅ"),
    ("twa", "とぁ"),
    ("twi", "とぃ"),
    ("twe", "とぇ"),
    ("two", "とぉ"),
    ("t'u", "とぅ"),
    ("dwu", "どぅ"),
    ("dwa", "どぁ"),
    ("dwi", "どぃ"),
    ("dwe", "どぇ"),
    ("dwo", "どぉ"),
    ("d'u", "どぅ"),
    // f and hw
    ("fa", "ふぁ"),
    ("fi", "ふぃ"),
    ("fe", "ふぇ"),
    ("fo", "ふぉ"),
    ("fya", "ふゃ"),
    ("fyu", "ふゅ"),
    ("fyo", "ふょ"),
    ("hwa", "ふぁ"),
    ("hwi", "ふぃ"),
    ("hwe", "ふぇ"),
    ("hwo", "ふぉ"),
    ("hwyu", "ふゅ"),
    // w
    ("wi", "うぃ"),
    ("we", "うぇ"),
    ("wha", "うぁ"),
    ("whi", "うぃ"),
    ("whu", "う"),
    ("whe", "うぇ"),
    ("who", "うぉ"),
    ("wu", "う"),
    ("wyi", "ゐ"),
    ("wye", "ゑ"),
    // ye, and the small ぃ ぇ by their yōon-like escapes
    ("ye", "いぇ"),
    ("xyi", "ぃ"),
    ("xye", "ぇ"),
    ("lyi", "ぃ"),
    ("lye", "ぇ"),
    // q, kw, gw, sw, zw
    ("qa", "くぁ"),
    ("qi", "くぃ"),
    ("qu", "く"),
    ("qe", "くぇ"),
    ("qo", "くぉ"),
    ("kwa", "くぁ"),
    ("kwi", "くぃ"),
    ("kwu", "くぅ"),
    ("kwe", "くぇ"),
    ("kwo", "くぉ"),
    ("gwa", "ぐぁ"),
    ("gwi", "ぐぃ"),
    ("gwu", "ぐぅ"),
    ("gwe", "ぐぇ"),
    ("gwo", "ぐぉ"),
    ("swa", "すぁ"),
    ("swi", "すぃ"),
    ("swu", "すぅ"),
    ("swe", "すぇ"),
    ("swo", "すぉ"),
    ("zwa", "ずぁ"),
    ("zwi", "ずぃ"),
    ("zwu", "ずぅ"),
    ("zwe", "ずぇ"),
    ("zwo", "ずぉ"),
    // c
    ("ca", "か"),
    ("ci", "し"),
    ("cu", "く"),
    ("ce", "せ"),
    ("co", "こ"),
    // ん, by its own escape
    ("xn", "ん"),
];

/// Spellings an answer is read with and plain typing is not: what Hepburn writes
/// for five extended katakana, where the IME reads the same letters as an older
/// kana (`ti` is ち there, `wo` is を).
///
/// These are tried only by [`check_kana_text`], against an answer, so `ti` is
/// accepted for ティ and still types ち.
const HEPBURN_ONLY: &[(&str, &str)] = &[
    ("ti", "てぃ"),
    ("di", "でぃ"),
    ("tu", "とぅ"),
    ("du", "どぅ"),
    ("wo", "うぉ"),
    ("tyu", "てゅ"),
    ("dyu", "でゅ"),
];

/// The particles that are written with one kana and said with another, as
/// `(written, said)`. An answer holding the written kana accepts a typing that
/// gives the said one, at that position and nowhere else.
const PARTICLES: &[(char, char)] = &[('は', 'わ'), ('を', 'お'), ('へ', 'え')];

/// One way of typing one kana, or one digraph, with both scripts' renderings.
struct Entry {
    romaji: String,
    hiragana: String,
    katakana: String,
    /// The order the spelling was claimed in, so that where several spellings
    /// type the same kana [`romaji_for`] can hand out the first — Hepburn's.
    rank: usize,
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

    // Where the IME and the readings both claim a spelling, the IME's comes first,
    // so it wins: `va` is ヴァ and `wi` is ウィ, as a learner's own keyboard has them.
    let read: Vec<&str> = hiragana_with_readings()
        .chain(katakana_only_with_readings())
        .flat_map(|(_, reading)| reading.spellings())
        .collect();
    for (romaji, kana) in EXTENDED {
        if read.contains(romaji) {
            claim(&mut pairs, romaji, kana);
        }
    }

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

    // The archaic v-series ヷ ヸ ヹ ヺ has no hiragana, so both renderings would be
    // the katakana. Their spellings are the IME's ヴァ ヴィ ヴェ ヴォ by now, so this
    // claims nothing; it stays so that a katakana-only kana given a spelling of its
    // own later is typeable without a second change here.
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

    // And the rest of the IME's extended spellings.
    for (romaji, kana) in EXTENDED {
        claim(&mut pairs, romaji, kana);
    }

    let mut entries: Vec<Entry> = pairs
        .into_iter()
        .enumerate()
        .map(|(rank, (romaji, hiragana))| {
            let katakana = hiragana
                .chars()
                .map(|ch| to_katakana(ch).unwrap_or(ch))
                .collect();
            Entry {
                romaji,
                hiragana,
                katakana,
                rank,
            }
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

/// Whether a character only separates words: a space, or a hyphen.
fn is_separator(c: char) -> bool {
    c.is_whitespace() || c == '-'
}

/// How many characters of `rest` the ん rule claims, or `None` if this `n` starts
/// an ordinary syllable.
fn n_rule(rest: &str) -> Option<usize> {
    let mut chars = rest.chars();
    if chars.next() != Some('n') {
        return None;
    }
    match chars.next() {
        // `n'` is the explicit separator, and `nn` is ん taking both letters, as an
        // IME reads it: `honn` is ほん and `konnnichiha` is こんにちは. Which is also
        // why the IME reads Hepburn's `onna` as おんあ — and why an answer is not
        // graded by this rule alone (see `check_kana_text`).
        Some('\'') | Some('n') => Some(2),
        // A bare final `n`.
        None => Some(1),
        // Before a vowel or `y` the `n` belongs to the syllable: na, ni, nya.
        Some(c) if is_vowel(c) || c == 'y' => None,
        // Before anything else it is ん: shinbun, and `hon o` across the space.
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
    // Hepburn's `tch`, which Mozc has as an entry of its own: `matcha`, `kotchi`.
    if first == 't' && rest[1..].starts_with("ch") {
        return Some(1);
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

    while let Some(first) = rest.chars().next() {
        // A word break carries no kana; it only ends the word, which `n_rule` reads.
        if is_separator(first) {
            rest = &rest[first.len_utf8()..];
            consumed += 1;
            continue;
        }

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
/// nor `ya` alone does. ん is also accepted as `nn`, `n'` and `xn`, the IME's own
/// ways of typing it, which are spellings of the keyboard rather than readings and
/// so are not in the readings table.
pub fn matches_reading(kana: char, typed: &str) -> bool {
    let typed = typed.trim().to_ascii_lowercase();
    if matches!(kana, 'ん' | 'ン') && matches!(typed.as_str(), "nn" | "n'" | "xn") {
        return true;
    }
    crate::readings::reading(kana)
        .map(|r| r.spellings().any(|s| s == typed))
        .unwrap_or(false)
}

/// True when `typed` romaji spells the given kana string, converting through
/// [`to_kana`]. This is the check for a whole word rather than one character.
pub fn matches_word(word: &str, typed: &str) -> bool {
    to_kana(typed).map(|produced| produced == word).unwrap_or(false)
}

/// Fold the two yotsugana pairs onto the kana that spells the same sound.
///
/// ぢ and じ are one sound in modern Japanese and so are づ and ず; `ji` types じ
/// and `zu` types ず, which is what a real IME does. So an exercise that asks a
/// learner to *produce* a string containing ぢ cannot insist on ぢ being typed —
/// the engine cannot type it — and comparing the two as one sound is the honest
/// rule. Every other kana is left alone: を is not お when it is *written*, and
/// kana typed directly is writing. (Typed as romaji, を is read by sound; see
/// [`check_kana_text`].)
pub fn fold_yotsugana(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            'ぢ' => 'じ',
            'づ' => 'ず',
            _ => ch,
        })
        .collect()
}

/// What the learner's typing comes to in kana — the string a screen shows back.
///
/// Kana typed directly is taken as it stands, which is what a learner with a
/// Japanese IME produces; anything else is read as romaji, and a string the table
/// cannot read at all comes back empty rather than as an error, because "you typed
/// nothing this can read" is a wrong answer rather than a failure.
pub fn produce_kana(typed: &str) -> String {
    let typed = typed.trim();
    if typed.chars().any(is_kana) {
        typed.to_string()
    } else {
        to_kana(typed).unwrap_or_default()
    }
}

/// What a typed answer came to, and whether it was right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanaTextCheck {
    /// The kana the typing spelled, read the way that matched the answer when one
    /// did — `onna` against おんな shows おんな, not the IME's おんあ — and as
    /// [`produce_kana`] reads it otherwise.
    pub produced: String,
    /// Whether it says what the answer says.
    pub correct: bool,
}

/// Grade a typed reading against the expected kana, and say what was typed.
///
/// The rule a *production* exercise needs — the learner writes the reading of
/// something, in romaji or in kana, and this is the comparison. Kana typed directly
/// is compared as written, with katakana folded to hiragana, the two yotsugana
/// pairs treated as one sound, and any spaces dropped. Romaji is read **against the
/// answer** rather than converted first, so that each of these is right where the
/// answer has the kana it names and wrong anywhere else:
///
/// * `wa`, `o` and `e` for the particles は, を and へ;
/// * Hepburn's `nn` for ん plus an n-syllable (`onna` for おんな), beside the IME's
///   `nn` for ん alone (`onnna`);
/// * Hepburn's `ti`, `di`, `tu`, `du` and `wo` for ティ, ディ, トゥ, ドゥ and ウォ;
/// * `ji` and `zu` for ぢ and づ, and every other spelling the readings give a kana
///   even where a commoner kana owns it in [`to_kana`].
///
/// Anything [`to_kana`] reads as the answer is accepted here too: this widens what
/// the plain conversion accepts and never narrows it, so a spelling
/// [`romaji_for`] hands out is always right.
pub fn check_kana_text(expected: &str, typed: &str) -> KanaTextCheck {
    let typed = typed.trim();
    let expected: Vec<char> = without_spaces(&normalise_to_hiragana(expected))
        .chars()
        .collect();
    if expected.is_empty() {
        return KanaTextCheck {
            produced: produce_kana(typed),
            correct: false,
        };
    }
    if typed.chars().any(is_kana) {
        let written = fold_yotsugana(&without_spaces(&normalise_to_hiragana(typed)));
        let correct = written == fold_yotsugana(&expected.iter().collect::<String>());
        return KanaTextCheck {
            produced: typed.to_string(),
            correct,
        };
    }
    let lowered: Vec<char> = typed.to_ascii_lowercase().chars().collect();
    match Reader::new(&expected, &lowered).read() {
        Some(produced) => KanaTextCheck {
            produced,
            correct: true,
        },
        None => KanaTextCheck {
            produced: produce_kana(typed),
            correct: false,
        },
    }
}

/// True when `typed` says what `expected` says: [`check_kana_text`]'s verdict
/// without the kana it read.
pub fn matches_kana_text(expected: &str, typed: &str) -> bool {
    check_kana_text(expected, typed).correct
}

fn without_spaces(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

/// Whether two kana say the same thing for grading: equal once the yotsugana are
/// folded, or a particle's written kana in the answer against its said kana typed.
fn agrees(expected: char, typed: char) -> bool {
    let fold = |ch| match ch {
        'ぢ' => 'じ',
        'づ' => 'ず',
        other => other,
    };
    fold(expected) == fold(typed) || PARTICLES.contains(&(expected, typed))
}

/// Romaji read against an expected answer: every way the letters could be read is
/// tried, and the first that spells the answer wins.
///
/// The spellings are [`table`]'s, every spelling the readings give any kana, and
/// [`HEPBURN_ONLY`]; the rules are [`to_kana_in`]'s, with Hepburn's `nn` before a
/// syllable allowed beside the IME's. A position that has been tried and failed is
/// remembered, so the search is linear in practice — an answer is a handful of
/// kana, and each position has at most a few readings.
struct Reader<'a> {
    expected: &'a [char],
    typed: &'a [char],
    spellings: Vec<(Vec<char>, Vec<char>)>,
    failed: std::collections::HashSet<(usize, usize)>,
}

impl<'a> Reader<'a> {
    fn new(expected: &'a [char], typed: &'a [char]) -> Self {
        let mut spellings: Vec<(Vec<char>, Vec<char>)> = table()
            .into_iter()
            .map(|e| (e.romaji.chars().collect(), e.hiragana.chars().collect()))
            .collect();
        // The small kana are left out for the reason `table` leaves them out: ゃ's
        // reading is `ya`, and `kiya` must not be read as きゃ.
        let read = hiragana_with_readings()
            .chain(katakana_only_with_readings())
            .filter(|(kana, _)| !SMALL_KANA.contains(kana))
            .flat_map(|(kana, reading)| reading.spellings().map(move |s| (s, kana.to_string())));
        let hepburn = HEPBURN_ONLY.iter().map(|(r, k)| (*r, k.to_string()));
        for (romaji, kana) in read.chain(hepburn) {
            let pair = (romaji.chars().collect(), kana.chars().collect());
            if !spellings.contains(&pair) {
                spellings.push(pair);
            }
        }
        Reader {
            expected,
            typed,
            spellings,
            failed: Default::default(),
        }
    }

    fn read(&mut self) -> Option<String> {
        self.from(0, 0)
    }

    /// The kana `typed[i..]` spells if it can be read as `expected[j..]`.
    fn from(&mut self, i: usize, j: usize) -> Option<String> {
        if i == self.typed.len() {
            return (j == self.expected.len()).then(String::new);
        }
        if self.failed.contains(&(i, j)) {
            return None;
        }
        let found = self.step(i, j);
        if found.is_none() {
            self.failed.insert((i, j));
        }
        found
    }

    fn step(&mut self, i: usize, j: usize) -> Option<String> {
        let (typed, expected) = (self.typed, self.expected);
        let letter = typed[i];
        let next = typed.get(i + 1).copied();
        if is_separator(letter) {
            return self.from(i + 1, j);
        }
        let wanted = expected.get(j).copied();

        // The sokuon: a doubled consonant, or Hepburn's `tch`.
        if wanted == Some('っ') {
            let rest: String = typed[i..].iter().collect();
            if sokuon_rule(&rest) == Some(1) {
                if let Some(tail) = self.from(i + 1, j + 1) {
                    return Some(format!("っ{tail}"));
                }
            }
        }

        // ん, by `nn`, `n'` or `n` alone. `n` alone is ん before anything but a
        // vowel or `y` — including before another `n`, which is how Hepburn's `onna`
        // reads as おんな while the IME's `nn` reads `onnna` the same way.
        if wanted == Some('ん') && letter == 'n' {
            if matches!(next, Some('n') | Some('\'')) {
                if let Some(tail) = self.from(i + 2, j + 1) {
                    return Some(format!("ん{tail}"));
                }
            }
            if !next.is_some_and(|c| is_vowel(c) || c == 'y' || c == '\'') {
                if let Some(tail) = self.from(i + 1, j + 1) {
                    return Some(format!("ん{tail}"));
                }
            }
        }

        // Every spelling that starts here and spells the answer's next kana.
        let candidates: Vec<(usize, Vec<char>)> = self
            .spellings
            .iter()
            .filter(|(romaji, kana)| {
                typed[i..].starts_with(romaji)
                    && kana.len() <= expected.len() - j
                    && kana.iter().zip(&expected[j..]).all(|(k, e)| agrees(*e, *k))
            })
            .map(|(romaji, kana)| (romaji.len(), kana.clone()))
            .collect();
        for (length, kana) in candidates {
            if let Some(tail) = self.from(i + length, j + kana.len()) {
                return Some(kana.iter().collect::<String>() + &tail);
            }
        }
        None
    }
}

/// Whether a character is kana, for telling a typed reading from typed romaji.
fn is_kana(ch: char) -> bool {
    matches!(ch as u32, 0x3041..=0x309F | 0x30A1..=0x30FA | 0x30FC)
}

/// A romaji spelling that types this kana, or `None` when the table has none.
///
/// The reverse of [`to_kana`], and the half a production exercise needs: it is what
/// says whether an answer **can be typed at all** before the question is asked. A
/// kana the table cannot produce has no spelling here — ー is the one that matters,
/// because a long vowel is a spelling decision this engine deliberately leaves to
/// the learner; the small ゕ ゖ and the archaic ヷ ヸ ヹ ヺ have none either — so a
/// question whose answer contains one is not asked rather than asked and
/// unanswerable.
///
/// The construction is a walk of the same table [`to_kana`] reads, taking the
/// longest run of kana one spelling covers (ふぁ as `fa`, not `fu` and `xa`) and,
/// among spellings of the same kana, the first the table claimed — Hepburn's. Two
/// repairs follow. ん is spelled `n` and takes an apostrophe when the syllable after
/// it starts with a vowel, `y` or `n`, because `na` is な and `nn` is ん alone, so
/// んあ is `n'a` and んな is `n'na`. っ doubles the consonant after it, `tch` before
/// `ch`, and is `xtu` where there is none to double. The result is **verified by
/// typing it back**, so a spelling this function hands out is one [`to_kana`] really
/// produces; a walk that fails to round-trip answers `None` rather than a spelling
/// that does not work.
pub fn romaji_for(kana: &str) -> Option<String> {
    let kana = normalise_to_hiragana(kana);
    if kana.is_empty() {
        return None;
    }
    let entries = table();
    let mut pieces: Vec<(&str, &str)> = Vec::new();
    let mut rest = kana.as_str();
    while !rest.is_empty() {
        let entry = entries
            .iter()
            .filter(|e| rest.starts_with(&e.hiragana))
            .max_by(|a, b| {
                a.hiragana
                    .len()
                    .cmp(&b.hiragana.len())
                    .then_with(|| b.rank.cmp(&a.rank))
            })?;
        pieces.push((entry.hiragana.as_str(), entry.romaji.as_str()));
        rest = &rest[entry.hiragana.len()..];
    }
    let mut out = String::new();
    for (i, (piece, romaji)) in pieces.iter().enumerate() {
        let next = pieces.get(i + 1).map(|(_, romaji)| *romaji).unwrap_or("");
        match *piece {
            "ん" => {
                out.push('n');
                if next.starts_with(|ch: char| is_vowel(ch) || ch == 'y' || ch == 'n') {
                    out.push('\'');
                }
            }
            "っ" => match next.chars().next() {
                _ if next.starts_with("ch") => out.push('t'),
                Some(c) if c != 'n' && !is_vowel(c) && c.is_ascii_alphabetic() => out.push(c),
                _ => out.push_str(romaji),
            },
            _ => out.push_str(romaji),
        }
    }
    (to_kana(&out).ok().as_deref() == Some(kana.as_str())).then_some(out)
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
        assert_eq!(kana("onnna"), "おんな");
        // `nn` is ん taking both letters, so Hepburn's `onna` is the IME's おんあ.
        // Graded against an answer of おんな it is still right; see
        // `a_hepburn_double_n_is_still_accepted_where_the_answer_has_it`.
        assert_eq!(kana("onna"), "おんあ");
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
        // `va` is ヴァ, as in an IME, rather than the archaic ヷ.
        assert_eq!(
            to_kana_in(Script::Katakana, "va").expect("converts"),
            "ヴァ"
        );
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
        //
        // Except where the IME claims the spelling for an extended katakana: `wi`
        // and `we` type ウィ and ウェ, not ゐ and ゑ, and `va` `vi` `ve` `vo` type ヴァ
        // and its row, not ヷ and its. Those are named, so a seventh is noticed.
        let ime = ["va", "vi", "ve", "vo", "wi", "we"];
        for (romaji, expected) in EXTENDED.iter().filter(|(romaji, _)| ime.contains(romaji)) {
            assert_eq!(kana(romaji), *expected, "{romaji} is the IME's");
        }
        for (hiragana, reading) in hiragana_with_readings().chain(katakana_only_with_readings()) {
            for spelling in reading.spellings().filter(|s| !ime.contains(s)) {
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

    #[test]
    fn a_whole_kana_answer_is_matched_by_sound_rather_than_by_writing() {
        // Romaji, in either romanisation.
        assert!(matches_kana_text("てください", "tekudasai"));
        assert!(matches_kana_text("にたいして", "nitaishite"));
        assert!(matches_kana_text("し", "si"));
        // Kana typed directly — a learner with a Japanese IME — is accepted as it
        // stands, which is what `matches_word` cannot do.
        assert!(matches_kana_text("てください", "てください"));
        assert!(matches_kana_text("てください", "テクダサイ"));
        // A reading the corpus writes in katakana is answered in hiragana.
        assert!(matches_kana_text("カナ", "kana"));
        assert!(matches_kana_text("かな", "kana"));
        // And the two yotsugana are one sound in either direction: づ types as `zu`
        // and ぢ as `ji`, so a reading containing them stays answerable.
        assert!(matches_kana_text("つづく", "tsuzuku"));
        assert!(matches_kana_text("つづく", "tsuduku"));
        assert!(matches_kana_text("はなぢ", "hanaji"));
        assert!(matches_kana_text("はなじ", "hanadi"));
        // を typed as romaji is read by sound, so `o` is right where the answer has
        // を, and `wo` still is. The reverse is not: お is never typed `wo`, and を
        // *written* as お is a spelling mistake.
        assert!(matches_kana_text("を", "o"));
        assert!(matches_kana_text("を", "wo"));
        assert!(!matches_kana_text("お", "wo"));
        assert!(!matches_kana_text("を", "お"));
        // Wrong, empty and unreadable answers are all simply not right.
        assert!(!matches_kana_text("てください", "tekudasa"));
        assert!(!matches_kana_text("かな", "kama"));
        assert!(!matches_kana_text("てください", ""));
        assert!(!matches_kana_text("てください", "   "));
        assert!(!matches_kana_text("てください", "!!!"));
        assert!(!matches_kana_text("", "kana"));
        assert_eq!(fold_yotsugana("ぢづじず"), "じずじず");
    }

    #[test]
    fn a_string_can_be_spelled_back_and_is_verified_by_typing_it() {
        for (kana, typed) in [
            ("てください", "tekudasai"),
            ("にたいして", "nitaishite"),
            ("はんして", "hanshite"),
            ("かな", "kana"),
            ("きゃ", "kya"),
            ("きや", "kiya"),
            ("しんぶん", "shinbun"),
        ] {
            let romaji = romaji_for(kana).unwrap_or_else(|| panic!("{kana} has no spelling"));
            assert_eq!(to_kana(&romaji).ok().as_deref(), Some(kana), "{kana} → {romaji}");
            assert_eq!(romaji, typed, "{kana} should be spelled {typed}");
        }
        // The moraic n takes its apostrophe exactly where a following vowel or `y`
        // would otherwise swallow it: んあ is `n'a`, んや is `n'ya`, んか is `nka`.
        assert_eq!(romaji_for("んあ").as_deref(), Some("n'a"));
        assert_eq!(romaji_for("んや").as_deref(), Some("n'ya"));
        assert_eq!(romaji_for("んか").as_deref(), Some("nka"));
        assert_eq!(romaji_for("ん").as_deref(), Some("n"));
        // Nothing that is not kana has a spelling, and neither has the two things
        // the engine deliberately cannot type.
        assert_eq!(romaji_for(""), None);
        assert_eq!(romaji_for("ー"), None, "a long vowel is a spelling, not a sound");
        assert_eq!(romaji_for("kana"), None);
    }

    #[test]
    fn every_kana_the_readings_offer_can_be_typed_back() {
        // The reverse direction is only useful if it covers the table the forward
        // one builds, and two sets are real exceptions. ゕ and ゖ offer `ka` and
        // `ke`, and か and け claim those spellings first, so the small kana cannot
        // be typed at all; and the archaic ヷ ヸ ヹ ヺ offer `va` `vi` `ve` `vo`, which
        // the IME gives ヴァ ヴィ ヴェ ヴォ, as a real one does. Named rather than
        // filtered, so another is noticed.
        for (kana, _) in hiragana_with_readings().chain(katakana_only_with_readings()) {
            if matches!(kana, 'ゕ' | 'ゖ' | 'ヷ' | 'ヸ' | 'ヹ' | 'ヺ') {
                assert_eq!(romaji_for(&kana.to_string()), None, "{kana} is not typeable");
                continue;
            }
            let typed = romaji_for(&kana.to_string())
                .unwrap_or_else(|| panic!("{kana} has no spelling in the table"));
            assert_eq!(to_kana(&typed).ok(), Some(kana.to_string()), "{kana} typed as {typed}");
        }
    }

    // Issue #21, one example at a time. The sweep over the shipped course is in
    // `tests/romaji_artifact.rs`.

    #[test]
    fn the_particles_are_typed_by_how_they_sound() {
        // は, を and へ as particles are said `wa`, `o` and `e`, and a learner types
        // what they hear — the corpus's own romaji spells them that way too.
        assert!(matches_kana_text("わけではない", "wakedewanai"));
        assert!(matches_kana_text("をください", "okudasai"));
        assert!(matches_kana_text("へ", "e"));
        assert!(matches_kana_text("には", "niwa"));
        assert!(matches_kana_text("を", "o"));
        // The written forms are still right.
        assert!(matches_kana_text("わけではない", "wakedehanai"));
        assert!(matches_kana_text("をください", "wokudasai"));
        assert!(matches_kana_text("へ", "he"));
        // But only where the answer has the particle kana: わ is not typed `ha`,
        // お is not typed `wo`, and え is not typed `he`.
        assert!(!matches_kana_text("わ", "ha"));
        assert!(!matches_kana_text("お", "wo"));
        assert!(!matches_kana_text("え", "he"));
        assert!(!matches_kana_text("わけではない", "wakedewanaii"));
        // Kana typed directly is writing, not sound, and is compared as written.
        assert!(!matches_kana_text("わけではない", "わけでわない"));
    }

    #[test]
    fn spaces_and_hyphens_separate_words_rather_than_spoil_them() {
        assert!(matches_kana_text("わけではない", "wake de wa nai"));
        assert!(matches_kana_text("わけではない", "wake-de-wa-nai"));
        assert!(matches_kana_text("てください", "te kudasai"));
        assert_eq!(kana("wake de wa nai"), "わけでわない");
        assert_eq!(produce_kana("wake de wa nai"), "わけでわない");
        // A word ends where the space is, so an `n` before one is ん even when the
        // next word starts with a vowel: `hon o` is ほんお, not ほの.
        assert_eq!(kana("hon o"), "ほんお");
        assert!(matches_kana_text("ほんを", "hon o"));
        assert!(matches_kana_text("んばかり", "n bakari"));
    }

    #[test]
    fn a_doubled_n_is_one_moraic_n() {
        // `nn` is ん and takes both letters, as in every IME.
        assert_eq!(kana("honn"), "ほん");
        assert_eq!(kana("sannpo"), "さんぽ");
        assert_eq!(kana("konnnichiha"), "こんにちは");
        assert_eq!(kana("kinnyoubi"), "きんようび");
        assert_eq!(kana("nn"), "ん");
        assert!(matches_reading('ん', "nn"));
        assert!(matches_reading('ん', "n'"));
        assert!(matches_reading('ん', "n"));
        assert!(matches_reading('ン', "nn"));
        assert!(!matches_reading('な', "nn"));
    }

    #[test]
    fn a_hepburn_double_n_is_still_accepted_where_the_answer_has_it() {
        // Hepburn writes んな as `nna` and an IME reads `nna` as んあ. Graded against
        // an answer, both readings are tried, so a learner typing either way is right.
        assert!(matches_kana_text("おんな", "onna"));
        assert!(matches_kana_text("おんな", "onnna"));
        assert!(matches_kana_text("こんにちは", "konnichiwa"));
        assert!(matches_kana_text("こんにちは", "konnnichiha"));
        assert!(matches_kana_text("きんようび", "kinnyoubi"));
        assert!(matches_kana_text("きんようび", "kin'youbi"));
        assert!(matches_kana_text("おんあ", "onna"));
        assert!(!matches_kana_text("おな", "onna"));
    }

    #[test]
    fn the_extended_katakana_can_be_typed() {
        assert_eq!(
            to_kana_in(Script::Katakana, "vaiorin").expect("converts"),
            "ヴァイオリン"
        );
        assert!(matches_kana_text("ヴァイオリン", "vaiorin"));
        for (typed, expected) in [
            ("va", "ヴァ"),
            ("vi", "ヴィ"),
            ("vu", "ヴ"),
            ("ve", "ヴェ"),
            ("vo", "ヴォ"),
            ("fa", "ファ"),
            ("fi", "フィ"),
            ("fe", "フェ"),
            ("fo", "フォ"),
            ("fyu", "フュ"),
            ("she", "シェ"),
            ("je", "ジェ"),
            ("che", "チェ"),
            ("thi", "ティ"),
            ("dhi", "ディ"),
            ("dhu", "デュ"),
            ("twu", "トゥ"),
            ("dwu", "ドゥ"),
            ("wi", "ウィ"),
            ("we", "ウェ"),
            ("who", "ウォ"),
            ("ye", "イェ"),
            ("kwa", "クァ"),
            ("gwa", "グァ"),
            ("tsa", "ツァ"),
            ("tsi", "ツィ"),
            ("tse", "ツェ"),
            ("tso", "ツォ"),
        ] {
            assert_eq!(
                to_kana_in(Script::Katakana, typed).expect("converts"),
                expected,
                "{typed}"
            );
        }
        // Hepburn writes ティ, ディ, トゥ and ドゥ as `ti`, `di`, `tu` and `du`, which an
        // IME reads as ち, ぢ, つ and づ. Graded against an answer, both are tried.
        assert_eq!(kana("ti"), "ち");
        assert!(matches_kana_text("ティ", "ti"));
        assert!(matches_kana_text("ディズニ", "dizuni"));
        assert!(matches_kana_text("トゥ", "tu"));
        assert!(matches_kana_text("ウォ", "wo"));
        // Reading against the answer never splits a digraph: a small kana is not
        // read from its full-size spelling, so きゃ is not `kiya` and ふぁ is not `fua`.
        assert!(!matches_kana_text("きゃ", "kiya"));
        assert!(!matches_kana_text("ふぁ", "fua"));
        // And every one of them can be spelled back.
        assert_eq!(romaji_for("ヴァイオリン").as_deref(), Some("vaiorin"));
        assert_eq!(romaji_for("ふぁん").as_deref(), Some("fan"));
        assert_eq!(romaji_for("ティ").as_deref(), Some("thi"));
    }

    #[test]
    fn hepburn_tch_is_the_sokuon_before_chi() {
        assert_eq!(kana("matcha"), "まっちゃ");
        assert_eq!(kana("kotchi"), "こっち");
        assert_eq!(kana("kocchi"), "こっち");
        assert!(matches_kana_text("まっちゃ", "matcha"));
    }

    #[test]
    fn whatever_the_gate_spells_the_check_accepts() {
        for kana in [
            "こんにちは",
            "おんな",
            "きんようび",
            "わけではない",
            "をください",
            "まっちゃ",
            "ヴァイオリン",
        ] {
            let typed = romaji_for(kana).unwrap_or_else(|| panic!("{kana} is typeable"));
            assert_eq!(
                to_kana(&typed).ok().map(|k| normalise_to_hiragana(&k)),
                Some(normalise_to_hiragana(kana)),
                "{kana} as {typed}"
            );
            assert!(matches_kana_text(kana, &typed), "{kana} as {typed}");
        }
    }
}
