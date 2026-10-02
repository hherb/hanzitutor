//! How each kana is written in the Latin alphabet.
//!
//! Two romanisations matter to a learner and this module carries both, because
//! they disagree on exactly the sounds that are hardest to guess:
//!
//! * **Hepburn** — what textbooks, signage and place names use. し is `shi`,
//!   つ is `tsu`, ふ is `fu`.
//! * **Kunrei-shiki** — the government standard, which is a straight
//!   transliteration of the gojūon grid. し is `si`, つ is `tu`, ふ is `hu`.
//!
//! A learner meets both, and a kana chart that shows only one of them teaches
//! something they will have to unlearn at the first train station. Where the two
//! agree, [`Reading::kunrei`] is empty rather than repeating the Hepburn form.
//!
//! Readings are **not** in the shipped artifact. They are not upstream data —
//! unlike the geometry, nothing is fetched — and they never change, so a table
//! in the code is the honest place for them. The artifact's kana and this table
//! are held together by a test that every kana has a reading.
//!
//! Two characters have no reading of their own, and neither is an oversight:
//! **っ/ッ** is the sokuon, a gemination mark that doubles the *next* consonant,
//! and **ー** is the chōonpu, which lengthens the vowel *before* it. Both are
//! taught, both are drawn, and neither is pronounced alone.
//!
//! Katakana readings are not written out twice: the katakana block mirrors the
//! hiragana block one code point apart, so they are derived, with the four
//! v-series katakana that have no hiragana counterpart listed on their own.

/// A kana's romanisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// Hepburn forms, the commonest first. Empty when the kana is not pronounced
    /// on its own.
    pub hepburn: &'static [&'static str],
    /// Kunrei-shiki forms, but only where they differ from Hepburn. Empty when
    /// the two agree, or when the kana is not pronounced on its own.
    pub kunrei: &'static [&'static str],
}

impl Reading {
    /// True when the kana carries no sound by itself — っ or ー.
    pub fn is_silent(&self) -> bool {
        self.hepburn.is_empty()
    }

    /// Every spelling a learner might type, Hepburn first, without repeats.
    pub fn spellings(&self) -> impl Iterator<Item = &'static str> {
        let (mut hepburn, kunrei) = (self.hepburn.iter(), self.kunrei.iter());
        let mut out: Vec<&'static str> = Vec::with_capacity(3);
        for form in hepburn.by_ref() {
            if !out.contains(form) {
                out.push(form);
            }
        }
        for form in kunrei {
            if !out.contains(form) {
                out.push(form);
            }
        }
        out.into_iter()
    }
}

/// The hiragana block, in code-point order, as
/// `(kana, Hepburn forms, Kunrei forms where they differ)`.
const HIRAGANA: &[(char, &[&str], &[&str])] = &[
    // a
    ('ぁ', &["a"], &[]),
    ('あ', &["a"], &[]),
    ('ぃ', &["i"], &[]),
    ('い', &["i"], &[]),
    ('ぅ', &["u"], &[]),
    ('う', &["u"], &[]),
    ('ぇ', &["e"], &[]),
    ('え', &["e"], &[]),
    ('ぉ', &["o"], &[]),
    ('お', &["o"], &[]),
    // ka
    ('か', &["ka"], &[]),
    ('が', &["ga"], &[]),
    ('き', &["ki"], &[]),
    ('ぎ', &["gi"], &[]),
    ('く', &["ku"], &[]),
    ('ぐ', &["gu"], &[]),
    ('け', &["ke"], &[]),
    ('げ', &["ge"], &[]),
    ('こ', &["ko"], &[]),
    ('ご', &["go"], &[]),
    // sa
    ('さ', &["sa"], &[]),
    ('ざ', &["za"], &[]),
    ('し', &["shi"], &["si"]),
    ('じ', &["ji"], &["zi"]),
    ('す', &["su"], &[]),
    ('ず', &["zu"], &[]),
    ('せ', &["se"], &[]),
    ('ぜ', &["ze"], &[]),
    ('そ', &["so"], &[]),
    ('ぞ', &["zo"], &[]),
    // ta
    ('た', &["ta"], &[]),
    ('だ', &["da"], &[]),
    ('ち', &["chi"], &["ti"]),
    // ぢ and じ are the same sound; Hepburn writes both `ji`, Kunrei separates
    // them as `di` and `zi`.
    ('ぢ', &["ji"], &["di"]),
    // The sokuon: it has no sound until it doubles the next one.
    ('っ', &[], &[]),
    ('つ', &["tsu"], &["tu"]),
    ('づ', &["zu"], &["du"]),
    ('て', &["te"], &[]),
    ('で', &["de"], &[]),
    ('と', &["to"], &[]),
    ('ど', &["do"], &[]),
    // na
    ('な', &["na"], &[]),
    ('に', &["ni"], &[]),
    ('ぬ', &["nu"], &[]),
    ('ね', &["ne"], &[]),
    ('の', &["no"], &[]),
    // ha
    ('は', &["ha"], &[]),
    ('ば', &["ba"], &[]),
    ('ぱ', &["pa"], &[]),
    ('ひ', &["hi"], &[]),
    ('び', &["bi"], &[]),
    ('ぴ', &["pi"], &[]),
    ('ふ', &["fu"], &["hu"]),
    ('ぶ', &["bu"], &[]),
    ('ぷ', &["pu"], &[]),
    ('へ', &["he"], &[]),
    ('べ', &["be"], &[]),
    ('ぺ', &["pe"], &[]),
    ('ほ', &["ho"], &[]),
    ('ぼ', &["bo"], &[]),
    ('ぽ', &["po"], &[]),
    // ma
    ('ま', &["ma"], &[]),
    ('み', &["mi"], &[]),
    ('む', &["mu"], &[]),
    ('め', &["me"], &[]),
    ('も', &["mo"], &[]),
    // ya
    ('ゃ', &["ya"], &[]),
    ('や', &["ya"], &[]),
    ('ゅ', &["yu"], &[]),
    ('ゆ', &["yu"], &[]),
    ('ょ', &["yo"], &[]),
    ('よ', &["yo"], &[]),
    // ra
    ('ら', &["ra"], &[]),
    ('り', &["ri"], &[]),
    ('る', &["ru"], &[]),
    ('れ', &["re"], &[]),
    ('ろ', &["ro"], &[]),
    // wa
    ('ゎ', &["wa"], &[]),
    ('わ', &["wa"], &[]),
    // ゐ and ゑ are obsolete in modern Japanese but are still taught as kana and
    // still appear in older text.
    ('ゐ', &["wi"], &[]),
    ('ゑ', &["we"], &[]),
    // The particle を is pronounced `o`; `wo` is the spelling, not the sound, so
    // both are offered and Hepburn's own form comes first.
    ('を', &["o", "wo"], &["wo"]),
    // n
    ('ん', &["n"], &[]),
    // The katakana-only sounds that acquired hiragana in modern use.
    ('ゔ', &["vu"], &[]),
    ('ゕ', &["ka"], &[]),
    ('ゖ', &["ke"], &[]),
];

/// The katakana that have no hiragana counterpart: the v-series, which exists
/// only for foreign words. The prolonged sound mark is handled separately.
const KATAKANA_ONLY: &[(char, &[&str])] = &[
    ('ヷ', &["va"]),
    ('ヸ', &["vi"]),
    ('ヹ', &["ve"]),
    ('ヺ', &["vo"]),
];

/// The hiragana a katakana mirrors, if it has one.
///
/// The two blocks are the same 86 characters one code point apart, so this is a
/// subtraction rather than a second table. `None` for the v-series and for ー.
fn mirrored_hiragana(ch: char) -> Option<char> {
    match ch as u32 {
        0x30A1..=0x30F6 => char::from_u32(ch as u32 - 0x60),
        _ => None,
    }
}

/// The romanisation of a kana, or `None` if the character is not a kana this app
/// teaches.
pub fn reading(ch: char) -> Option<Reading> {
    if let Some(hepburn) = KATAKANA_ONLY
        .iter()
        .find(|(kana, _)| *kana == ch)
        .map(|(_, forms)| *forms)
    {
        return Some(Reading { hepburn, kunrei: &[] });
    }

    // The prolonged sound mark lengthens the vowel before it; it is not a sound.
    if ch == 'ー' {
        return Some(Reading { hepburn: &[], kunrei: &[] });
    }

    let hiragana = mirrored_hiragana(ch).unwrap_or(ch);
    HIRAGANA
        .iter()
        .find(|(kana, _, _)| *kana == hiragana)
        .map(|(_, hepburn, kunrei)| Reading { hepburn, kunrei })
}

/// Every hiragana that has a reading, in code-point order.
///
/// For callers that need to enumerate the set rather than look one up — the
/// romaji input engine builds its table this way, so the table cannot drift from
/// the readings.
pub fn hiragana_with_readings() -> impl Iterator<Item = (char, Reading)> {
    HIRAGANA
        .iter()
        .map(|(kana, hepburn, kunrei)| (*kana, Reading { hepburn, kunrei }))
}

/// The katakana that have no hiragana counterpart, with their readings.
pub fn katakana_only_with_readings() -> impl Iterator<Item = (char, Reading)> {
    KATAKANA_ONLY
        .iter()
        .map(|(kana, hepburn)| (*kana, Reading { hepburn, kunrei: &[] }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KanaDataset;

    fn dataset() -> KanaDataset {
        KanaDataset::from_gzip_bytes(include_bytes!("../data/kana.bin.gz"))
            .expect("the committed artifact decodes")
    }

    #[test]
    fn every_kana_in_the_artifact_has_a_reading_entry() {
        for kana in dataset().kana() {
            assert!(
                reading(kana.ch).is_some(),
                "{} (U+{:04X}) has geometry but no reading",
                kana.ch,
                kana.ch as u32
            );
        }
    }

    #[test]
    fn the_table_holds_nothing_the_artifact_does_not() {
        let dataset = dataset();
        for (kana, _, _) in HIRAGANA {
            assert!(
                dataset.get(*kana).is_some(),
                "{kana} has a reading but is not in the dataset"
            );
        }
        for (kana, _) in KATAKANA_ONLY {
            assert!(
                dataset.get(*kana).is_some(),
                "{kana} has a reading but is not in the dataset"
            );
        }
    }

    #[test]
    fn the_table_has_no_duplicate_entries() {
        let mut seen = std::collections::BTreeSet::new();
        for (kana, _, _) in HIRAGANA {
            assert!(seen.insert(*kana), "{kana} is listed twice");
        }
    }

    #[test]
    fn only_the_two_marks_that_are_not_sounds_are_silent() {
        let silent: Vec<char> = dataset()
            .kana()
            .iter()
            .filter(|k| reading(k.ch).expect("has a reading").is_silent())
            .map(|k| k.ch)
            .collect();
        assert_eq!(
            silent,
            vec!['っ', 'ッ', 'ー'],
            "the sokuon and the prolonged sound mark are the only silent kana"
        );
    }

    #[test]
    fn the_sounds_hepburn_and_kunrei_disagree_on_are_all_present() {
        for &(ch, hepburn, kunrei) in &[
            ('し', "shi", "si"),
            ('ち', "chi", "ti"),
            ('つ', "tsu", "tu"),
            ('ふ', "fu", "hu"),
            ('じ', "ji", "zi"),
            ('ぢ', "ji", "di"),
            ('づ', "zu", "du"),
            ('を', "o", "wo"),
        ] {
            let reading = reading(ch).expect("has a reading");
            assert_eq!(reading.hepburn[0], hepburn, "{ch} in Hepburn");
            assert_eq!(reading.kunrei[0], kunrei, "{ch} in Kunrei-shiki");
        }
    }

    #[test]
    fn kunrei_is_only_stated_where_it_differs() {
        for kana in dataset().kana() {
            let reading = reading(kana.ch).expect("has a reading");
            if !reading.kunrei.is_empty() {
                assert_ne!(
                    reading.kunrei[0], reading.hepburn[0],
                    "{} repeats its Hepburn form in the Kunrei field",
                    kana.ch
                );
            }
        }
    }

    #[test]
    fn katakana_mirror_hiragana_for_the_shared_block() {
        for kana in dataset().of_script(crate::Script::Katakana) {
            if let Some(hiragana) = mirrored_hiragana(kana.ch) {
                assert_eq!(
                    reading(kana.ch),
                    reading(hiragana),
                    "{} should read as {hiragana}",
                    kana.ch
                );
            }
        }
        // And the mirror really does cover the whole shared block.
        assert_eq!(reading('ア'), reading('あ'));
        assert_eq!(reading('ン'), reading('ん'));
        assert_eq!(reading('ヺ').expect("has a reading").hepburn[0], "vo");
    }

    #[test]
    fn every_katakana_resolves_including_the_ones_with_no_hiragana() {
        for kana in dataset().of_script(crate::Script::Katakana) {
            assert!(
                reading(kana.ch).is_some(),
                "{} has no reading",
                kana.ch
            );
        }
    }

    #[test]
    fn a_character_that_is_not_a_kana_has_no_reading() {
        assert!(reading('一').is_none());
        assert!(reading('a').is_none());
    }

    #[test]
    fn spellings_offer_both_styles_without_repeating_itself() {
        let forms: Vec<&str> = reading('し').expect("has a reading").spellings().collect();
        assert_eq!(forms, vec!["shi", "si"]);

        let forms: Vec<&str> = reading('ふ').expect("has a reading").spellings().collect();
        assert_eq!(forms, vec!["fu", "hu"]);

        // Where the two agree there is one spelling, not two.
        let forms: Vec<&str> = reading('か').expect("has a reading").spellings().collect();
        assert_eq!(forms, vec!["ka"]);

        // を: Hepburn's `o` and the `wo` spelling, once each.
        let forms: Vec<&str> = reading('を').expect("has a reading").spellings().collect();
        assert_eq!(forms, vec!["o", "wo"]);
    }

    #[test]
    fn a_silent_mark_has_no_spellings() {
        assert_eq!(reading('っ').expect("has a reading").spellings().count(), 0);
        assert_eq!(reading('ー').expect("has a reading").spellings().count(), 0);
    }
}
