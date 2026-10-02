//! The shipped kanji artifact, checked.
//!
//! These assertions are the contract the course relies on, and they run against
//! the **committed** artifact, so they hold in CI without any upstream download.
//!
//! Three things are frozen here, and all three are measurements rather than
//! opinions:
//!
//! * **the jōyō set and its current grades** — KANJIDIC2's, which is the grade
//!   assignment in force; AnimCJK's `dictionaryJa.txt` carries the pre-2017 one
//!   and differs for 59 characters (see [`the_twenty_kanji_added_to_kyoiku_in_2017`]
//!   and [`the_thirty_nine_characters_the_two_sources_place_differently`]);
//! * **the stroke counts**, which `prepare-kanji` checks against KanjiVG — a
//!   separate project's per-stroke paths — and refuses to write an artifact it
//!   could not check. One character, 衷, is a written exception;
//! * **the readings and glosses**, which come from EDRDG's KANJIDIC2 with their
//!   okurigana markers intact.

use nihongo_core::{Kanji, KanjiDataset, JOYO_COUNT, JOYO_GRADES};

fn dataset() -> KanjiDataset {
    KanjiDataset::from_gzip_bytes(include_bytes!("../data/kanji.bin.gz"))
        .expect("the committed artifact decodes")
}

fn kanji(ch: char) -> Kanji {
    dataset()
        .get(ch)
        .cloned()
        .unwrap_or_else(|| panic!("{ch} is in the dataset"))
}

#[test]
fn the_artifact_holds_the_whole_joyo_set() {
    let dataset = dataset();
    assert_eq!(dataset.len(), JOYO_COUNT);
    assert_eq!(dataset.len(), 2_136);
    let kyoiku: usize = dataset
        .kanji()
        .iter()
        .filter(|k| k.is_kyoiku())
        .count();
    assert_eq!(kyoiku, 1_026, "grades 1-6 are the kyōiku set");
    assert_eq!(dataset.len() - kyoiku, 1_110, "grade 8 is the jōyō remainder");
}

/// The per-grade totals are the current curriculum, not 2016's. The difference
/// from AnimCJK's is exactly accounted for below.
#[test]
fn the_grades_are_kanjidic2s_current_assignment() {
    let dataset = dataset();
    assert_eq!(
        dataset.grade_counts(),
        vec![
            (1, 80),
            (2, 160),
            (3, 200),
            (4, 202),
            (5, 193),
            (6, 191),
            (8, 1_110)
        ]
    );
    for k in dataset.kanji() {
        assert!(
            JOYO_GRADES.contains(&k.grade),
            "{} has grade {}, which is not a jōyō grade",
            k.ch,
            k.grade
        );
        assert_ne!(
            k.grade, 7,
            "KANJIDIC2 has no grade 7 — AnimCJK's g7 is grade 8 here, and reading the two \
             numberings as one is the trap this asserts against"
        );
    }
}

/// The 2017 revision of the kyōiku list added 20 prefecture kanji, all of them to
/// grade 4. AnimCJK's data predates it, so it files every one of these under the
/// jōyō remainder (`g7`) — which is why taking its `set` would teach 2016's
/// curriculum.
#[test]
fn the_twenty_kanji_added_to_kyoiku_in_2017() {
    let added = "茨媛岡潟岐熊香佐埼崎滋鹿縄井沖栃奈梨阪阜";
    assert_eq!(added.chars().count(), 20);
    for ch in added.chars() {
        assert_eq!(kanji(ch).grade, 4, "{ch} was added to grade 4 in 2017");
    }
}

/// The other half of the reconciliation, and the half that is easy to miss: the
/// per-grade differences are *not* explained by the 20 additions alone. These 39
/// characters sit in different grades in the two sources, and it is the sum of
/// both effects that turns AnimCJK's 200/185/181 into KANJIDIC2's 202/193/191.
///
/// Every pair below is `(character, the grade KANJIDIC2 gives it)`.
#[test]
fn the_thirty_nine_characters_the_two_sources_place_differently() {
    let expected: &[(char, u8)] = &[
        // AnimCJK g3 -> KANJIDIC2 4
        ('夫', 4),
        // AnimCJK g4 -> 3
        ('央', 3),
        // AnimCJK g4 -> 5 (21)
        ('停', 5), ('史', 5), ('告', 5), ('喜', 5), ('囲', 5), ('型', 5), ('堂', 5),
        ('士', 5), ('得', 5), ('救', 5), ('歴', 5), ('殺', 5), ('毒', 5), ('粉', 5),
        ('紀', 5), ('脈', 5), ('航', 5), ('象', 5), ('貯', 5), ('費', 5), ('賞', 5),
        // AnimCJK g4 -> 6
        ('胃', 6), ('腸', 6),
        // AnimCJK g5 -> 4
        ('賀', 4), ('群', 4), ('徳', 4), ('富', 4),
        // AnimCJK g5 -> 6
        ('俵', 6), ('券', 6), ('恩', 6), ('承', 6), ('敵', 6), ('舌', 6), ('退', 6),
        ('銭', 6), ('預', 6),
        // AnimCJK g6 -> 4
        ('城', 4),
    ];

    assert_eq!(expected.len(), 39, "the reassigned characters, counted");
    for &(ch, grade) in expected {
        assert_eq!(kanji(ch).grade, grade, "{ch}");
    }
}

#[test]
fn every_kanji_can_actually_be_practised() {
    for k in dataset().kanji() {
        assert!(k.is_practisable(), "{} has no usable geometry", k.ch);
        assert_eq!(
            k.outlines.len(),
            k.medians.len(),
            "{} has an outline for every centre-line and vice versa",
            k.ch
        );
        assert_eq!(
            k.stroke_count as usize,
            k.medians.len(),
            "{}'s stated stroke count matches its geometry",
            k.ch
        );
        assert!(
            (1..=29).contains(&k.stroke_count),
            "{} has {} strokes, outside the range jōyō uses (1 for 一, 29 for 鬱)",
            k.ch,
            k.stroke_count
        );
    }
}

/// All grading happens in display space, over a 1024×1024 box. A centre-line
/// outside it is a coordinate-conversion bug — the geometry is stored in font
/// space and converted by `Point::from_font`, and a wrong conversion would flip
/// or displace every kanji.
#[test]
fn every_centre_line_sits_inside_the_drawing_box() {
    for k in dataset().kanji() {
        for (index, median) in k.medians.iter().enumerate() {
            for point in median {
                assert!(
                    (0.0..=1024.0).contains(&point.x) && (0.0..=1024.0).contains(&point.y),
                    "{} stroke {} has a point at ({}, {}), outside the 1024 box",
                    k.ch,
                    index + 1,
                    point.x,
                    point.y
                );
            }
        }
    }
}

#[test]
fn every_stroke_is_a_path() {
    for k in dataset().kanji() {
        for (index, median) in k.medians.iter().enumerate() {
            assert!(
                median.len() >= 2,
                "{} stroke {} has only {} point(s)",
                k.ch,
                index + 1,
                median.len()
            );
        }
    }
}

#[test]
fn kanji_are_stored_in_code_point_order_without_duplicates() {
    let dataset = dataset();
    let codepoints: Vec<u32> = dataset.kanji().iter().map(|k| k.ch as u32).collect();
    let mut sorted = codepoints.clone();
    sorted.sort_unstable();
    assert_eq!(codepoints, sorted, "storage order is deterministic");
    assert_eq!(
        codepoints.len(),
        codepoints.iter().collect::<std::collections::BTreeSet<_>>().len(),
        "no character appears twice"
    );
    assert!(dataset.get('一').is_some());
    assert!(
        dataset.get('あ').is_none(),
        "this artifact is kanji only, and the kana live in their own"
    );
}

#[test]
fn the_whole_set_is_22367_strokes() {
    let total: usize = dataset()
        .kanji()
        .iter()
        .map(|k| k.stroke_count as usize)
        .sum();
    assert_eq!(total, 22_367);
}

/// A handful of characters whose counts are worth knowing by heart, including the
/// one the previous documentation got wrong by transcribing the wrong code point
/// (鳥 is U+9CE5 and has 11 strokes; U+9CE9 is 鳩, which has 13 and is **not
/// jōyō** — the boundary the next test holds).
#[test]
fn a_few_well_known_kanji_have_the_counts_a_learner_is_taught() {
    for &(ch, strokes) in &[
        ('一', 1),
        ('二', 2),
        ('三', 3),
        ('口', 3),
        ('学', 8),
        ('国', 8),
        ('鳥', 11),
        ('愛', 13),
        ('韓', 18),
    ] {
        assert_eq!(kanji(ch).stroke_count, strokes, "{ch}");
    }
}

/// The artifact is the jōyō set and stops there. 鳩 is the example worth having:
/// KANJIDIC2 grades it 9, which is jinmeiyō, so its absence is the boundary
/// rather than an accident — and it is also the character the old documentation
/// confused with 鳥.
#[test]
fn jinmeiyo_and_hyogai_are_not_in_this_artifact() {
    let dataset = dataset();
    assert_eq!(
        kanji('鳥').grade,
        2,
        "鳥 is kyōiku grade 2 — the character the old note confused with 鳩"
    );
    assert!(
        dataset.get('鳩').is_none(),
        "鳩 is jinmeiyō (KANJIDIC2 grade 9) and is deliberately not carried"
    );
    for ch in ['郁', '晟', '滉'] {
        assert!(dataset.get(ch).is_none(), "{ch} is jinmeiyō or hyōgai");
    }
}

/// The one character KanjiVG counts differently.
///
/// `ROADMAP_NIHONGO.md` records nine (謎 賭 葛 餌 遜 僅 遡 餅 牙), and that list was
/// checked against the files rather than repeated: those nine are the characters
/// where KANJIDIC2 lists more than one stroke count and the *first* is not the
/// taught one, so comparing against whichever value happens to come first
/// invents a disagreement. Against the taught count, all nine agree and 衷 is the
/// only real one. `prepare-kanji` carries it with both numbers, so this fails if
/// either side moves.
#[test]
fn the_one_character_kanjivg_counts_differently_is_ten_strokes() {
    assert_eq!(kanji('衷').stroke_count, 10);
    // And the nine the documentation named are ordinary, for completeness.
    for &(ch, strokes) in &[
        ('謎', 17),
        ('賭', 16),
        ('葛', 12),
        ('餌', 15),
        ('遜', 14),
        ('僅', 13),
        ('遡', 14),
        ('餅', 15),
        ('牙', 4),
    ] {
        assert_eq!(kanji(ch).stroke_count, strokes, "{ch}");
    }
}

/// Readings keep KANJIDIC2's okurigana markers: `.` ends the stem and starts the
/// okurigana, and a trailing `-` marks an affix that is not used on its own. They
/// are not stripped, because た.べる is what tells a course that 食べる is written
/// with the kana attached and ひと.つ that 一つ is.
#[test]
fn readings_carry_their_okurigana() {
    assert_eq!(kanji('食').kun, vec!["く.う", "く.らう", "た.べる", "は.む"]);
    assert_eq!(kanji('学').kun, vec!["まな.ぶ"]);
    assert_eq!(kanji('学').on, vec!["ガク"]);
    assert_eq!(kanji('一').kun, vec!["ひと-", "ひと.つ"]);

    let mut on = 0usize;
    let mut kun = 0usize;
    let mut okurigana = 0usize;
    let mut affixes = 0usize;
    for k in dataset().kanji() {
        on += k.on.len();
        kun += k.kun.len();
        okurigana += k.kun.iter().filter(|r| r.contains('.')).count();
        affixes += k.kun.iter().filter(|r| r.contains('-')).count();
        for reading in k.on.iter().chain(k.kun.iter()) {
            assert!(!reading.is_empty(), "{} has an empty reading", k.ch);
        }
    }
    assert_eq!((on, kun), (2_854, 3_904), "every reading KANJIDIC2 gives");
    assert_eq!(okurigana, 2_551, "kun readings with okurigana to attach");
    assert_eq!(affixes, 364, "kun readings that are affixes only");
}

/// The glosses are KANJIDIC2's English senses and nothing else. The `-all`
/// document also carries French, Spanish and Portuguese; an English-only check is
/// the cheapest way to prove none of them leaked in, and the meanings are ASCII
/// in this source, so a non-ASCII one is a leak rather than a legitimate accent.
#[test]
fn meanings_are_english_only() {
    let mut total = 0usize;
    for k in dataset().kanji() {
        assert!(!k.meanings.is_empty(), "{} has no gloss", k.ch);
        for meaning in &k.meanings {
            assert!(
                !meaning.is_empty(),
                "{} has an empty gloss",
                k.ch
            );
            assert!(
                meaning.is_ascii(),
                "{}'s gloss {meaning:?} is not ASCII — the other languages in the KANJIDIC2 \
                 document have leaked in",
                k.ch
            );
        }
        total += k.meanings.len();
    }
    assert_eq!(total, 7_937);
}

#[test]
fn radicals_are_the_kangxi_radicals_with_their_notes() {
    let mut numbers = std::collections::BTreeSet::new();
    for k in dataset().kanji() {
        assert!(
            (1..=214).contains(&k.radical_number),
            "{}'s classical radical number is {}",
            k.ch,
            k.radical_number
        );
        numbers.insert(k.radical_number);
    }
    assert_eq!(numbers.len(), 198, "jōyō uses 198 of the 214 radicals");

    // The radical is the form the character is written with, and the note is what
    // the upstream dictionary says in parentheses.
    let mochi = kanji('持');
    assert_eq!(mochi.radical, '扌');
    assert_eq!(mochi.radical_note.as_deref(), Some("手"));
    assert_eq!(kanji('学').radical, '子');
    assert_eq!(kanji('学').radical_note, None);

    // 769 jōyō characters carry such a note. The one *prose* note in the source —
    // 阝 (阜 or 邑) — belongs to a character that is not jōyō, so it never reaches
    // this artifact; `parse_radical` is unit-tested on it in `kanji.rs` instead of
    // being asserted here, where it would be a claim about nothing.
    let noted = dataset()
        .kanji()
        .iter()
        .filter(|k| k.radical_note.is_some())
        .count();
    assert_eq!(noted, 769);
    for k in dataset().kanji() {
        if let Some(note) = &k.radical_note {
            assert!(!note.is_empty(), "{} has an empty radical note", k.ch);
        }
    }
}

/// The decomposition is AnimCJK's IDS string, which is what a components panel
/// reads. Two jōyō characters have none, and one is not an IDS expression at all —
/// it is the character itself — so both are named rather than smoothed over.
#[test]
fn the_decomposition_is_an_ids_string() {
    assert_eq!(kanji('学').decomposition, "⿳𰃮子");

    let ids: std::collections::BTreeSet<char> = (0x2FF0..=0x2FFB)
        .map(|cp| char::from_u32(cp).expect("a valid IDS operator"))
        .collect();

    let mut with_decomposition = 0usize;
    let mut without_operator = Vec::new();
    for k in dataset().kanji() {
        if k.decomposition.is_empty() {
            continue;
        }
        with_decomposition += 1;
        if !k.decomposition.chars().next().is_some_and(|c| ids.contains(&c)) {
            without_operator.push(k.ch);
        }
    }
    assert_eq!(with_decomposition, 2_134);
    assert_eq!(
        without_operator,
        vec!['衣'],
        "one decomposition is the character itself rather than an IDS expression"
    );

    let empty: Vec<char> = dataset()
        .kanji()
        .iter()
        .filter(|k| k.decomposition.is_empty())
        .map(|k| k.ch)
        .collect();
    assert_eq!(empty, vec!['一', '乙'], "the two with no decomposition");
}

/// Frequency is KANJIDIC2's Mainichi ranking, and it is optional: 99 of the 2,136
/// have none, which is why the field is an `Option` rather than a zero.
#[test]
fn frequency_is_kanjidic2s_rank_and_may_be_absent() {
    assert_eq!(kanji('日').frequency, Some(1));
    assert_eq!(kanji('一').frequency, Some(2));
    assert_eq!(kanji('学').frequency, Some(63));

    let ranked = dataset()
        .kanji()
        .iter()
        .filter(|k| k.frequency.is_some())
        .count();
    assert_eq!(ranked, 2_037);
    for k in dataset().kanji() {
        if let Some(rank) = k.frequency {
            assert!(rank >= 1, "{} has rank 0, which is not a rank", k.ch);
        }
    }
}

#[test]
fn nanori_are_carried_where_kanjidic2_has_them() {
    let with_nanori = dataset()
        .kanji()
        .iter()
        .filter(|k| !k.nanori.is_empty())
        .count();
    assert_eq!(with_nanori, 924);
    assert!(kanji('学').nanori.contains(&"たか".to_string()));
    assert_eq!(
        dataset().kanji().iter().map(|k| k.nanori.len()).sum::<usize>(),
        2_606
    );
}

/// The artifact records which EDRDG snapshot it was built from, because the
/// licence obliges the app to keep the data current and an obligation nobody can
/// check against what shipped is one that lapses unnoticed. Pinning it here makes
/// a snapshot bump a deliberate commit — the refresh procedure in `LICENSES.md`.
#[test]
fn the_artifact_records_the_snapshot_it_was_built_from() {
    let source = dataset().source().clone();
    assert_eq!(source.kanjidic2_version, "3.6.2");
    assert_eq!(source.kanjidic2_date, "2026-09-28");
}

#[test]
fn kanji_can_be_graded_with_the_chinese_engine() {
    use hanzi_core::{grade, GradeOptions, Point};

    // The engine is shared with the Chinese app, and this is the claim that a
    // kanji needs none of it specialised: 一 is exactly one horizontal stroke, so
    // tracing its stored centre-line is a correct attempt and a stroke in the
    // corner is not.
    let ichi = kanji('一');
    let reference = ichi.reference_medians();
    assert_eq!(reference.len(), 1);

    let attempt = vec![reference[0].clone()];
    let report = grade(reference, &attempt, &GradeOptions::default());
    assert!(
        report.legible,
        "tracing the stored centre-line must be legible, got {:.0}",
        report.overall
    );

    let elsewhere = vec![vec![Point::new(20.0, 20.0), Point::new(60.0, 20.0)]];
    let wrong = grade(reference, &elsewhere, &GradeOptions::default());
    assert!(
        !wrong.legible,
        "a short stroke in the corner must not pass as 一, got {:.0}",
        wrong.overall
    );
}

/// The eight-stroke example from the module docs, graded as a whole: a kanji
/// whose strokes are drawn in order and in place is legible, which is what makes
/// the artifact usable for practice rather than only for drawing.
#[test]
fn a_multi_stroke_kanji_grades_as_legible_when_traced() {
    use hanzi_core::{grade, GradeOptions};

    let k = kanji('学');
    let reference = k.reference_medians();
    assert_eq!(reference.len(), 8);
    let attempt: Vec<Vec<hanzi_core::Point>> = reference.to_vec();
    let report = grade(reference, &attempt, &GradeOptions::default());
    assert!(
        report.legible,
        "tracing eight stored centre-lines must be legible, got {:.0}",
        report.overall
    );
}
