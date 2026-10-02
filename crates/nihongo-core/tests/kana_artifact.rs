//! The shipped kana artifact, checked.
//!
//! These assertions are the contract the app relies on, and they run against the
//! **committed** artifact, so they hold in CI without any upstream download.
//!
//! The stroke counts frozen here are not this project's opinion: `prepare-kana`
//! derives the grouping from AnimCJK's SVG element ids and then checks every one
//! of the 177 characters against **KanjiVG**, a separate project whose per-stroke
//! paths are an independent statement of how each kana is written. The pipeline
//! fails rather than writes an artifact if the two disagree. These tests then
//! stop that verified result from being lost to a later change.

use nihongo_core::{KanaDataset, Script};

fn dataset() -> KanaDataset {
    KanaDataset::from_gzip_bytes(include_bytes!("../data/kana.bin.gz"))
        .expect("the committed artifact decodes")
}

#[test]
fn the_artifact_holds_every_kana_of_both_scripts() {
    let dataset = dataset();
    assert_eq!(dataset.len(), 177);
    assert_eq!(dataset.of_script(Script::Hiragana).count(), 86);
    assert_eq!(dataset.of_script(Script::Katakana).count(), 91);
}

#[test]
fn every_kana_can_actually_be_practised() {
    for kana in dataset().kana() {
        assert!(
            kana.is_practisable(),
            "{} has no usable geometry",
            kana.ch
        );
        assert_eq!(
            kana.outlines.len(),
            kana.medians.len(),
            "{} has an outline for every median and vice versa",
            kana.ch
        );
        assert_eq!(
            kana.stroke_count as usize,
            kana.medians.len(),
            "{}'s stated stroke count matches its geometry",
            kana.ch
        );
        assert!(
            (1..=6).contains(&kana.stroke_count),
            "{} has {} strokes, which is outside anything a kana uses",
            kana.ch,
            kana.stroke_count
        );
    }
}

/// The heart of the matter. AnimCJK stores these kana with more drawing segments
/// than strokes, because a stroke that crosses itself is split for the animation.
/// If the split is not folded back, the grader looks for a stroke the learner was
/// never taught to draw — so these counts are the regression guard.
///
/// Every count below is confirmed by KanjiVG.
#[test]
fn kana_whose_stroke_is_split_for_animation_are_counted_as_taught() {
    let dataset = dataset();
    let expected: &[(char, u8)] = &[
        ('ぁ', 3),
        ('あ', 3),
        ('ぉ', 3),
        ('お', 3),
        ('す', 2),
        ('ず', 4),
        ('な', 4),
        ('ぬ', 2),
        ('ね', 2),
        ('の', 1),
        ('は', 3),
        ('ば', 5),
        ('ぱ', 4),
        ('ほ', 4),
        ('ぼ', 6),
        ('ぽ', 5),
        ('ま', 3),
        ('み', 2),
        ('む', 3),
        ('め', 2),
        ('ょ', 2),
        ('よ', 2),
        ('る', 1),
        ('ゐ', 1),
        ('ゑ', 1),
    ];

    assert_eq!(expected.len(), 25, "there are 25 such kana");
    for &(ch, strokes) in expected {
        let kana = dataset.get(ch).unwrap_or_else(|| panic!("{ch} is in the dataset"));
        assert_eq!(
            kana.stroke_count, strokes,
            "{ch} is taught with {strokes} strokes and must not be stored with more"
        );
    }
}

/// The split is not a licence to drop geometry: the kana that were folded back
/// must still carry as many strokes as they are taught, no fewer.
#[test]
fn folding_never_removes_a_taught_stroke() {
    let dataset = dataset();
    // あ is the worked example: four drawing segments, three taught strokes, and
    // the third stroke's merged outline must span the loop rather than half of it.
    let a = dataset.get('あ').expect("あ is in the dataset");
    assert_eq!(a.stroke_count, 3);
    assert_eq!(a.outlines.len(), 3);
    assert_eq!(a.medians.len(), 3);

    let third = &a.medians[2];
    assert!(
        third.len() >= 4,
        "the loop's centre-line is a path, not a stub: got {} points",
        third.len()
    );
}

#[test]
fn a_few_common_kana_have_the_stroke_counts_a_learner_is_taught() {
    let dataset = dataset();
    for &(ch, strokes) in &[
        ('ん', 1),
        ('く', 1),
        ('し', 1),
        ('つ', 1),
        ('い', 2),
        ('り', 2),
        ('き', 4),
        ('ふ', 4),
        ('ア', 2),
        ('ー', 1),
    ] {
        let kana = dataset.get(ch).unwrap_or_else(|| panic!("{ch} is in the dataset"));
        assert_eq!(kana.stroke_count, strokes, "{ch}");
    }
}

/// All grading happens in display space, over a 1024×1024 box. A centre-line
/// outside it is a coordinate-conversion bug — and it is exactly the failure the
/// animation's displaced duplicates would cause if they were kept instead of
/// folded away.
#[test]
fn every_centre_line_sits_inside_the_drawing_box() {
    for kana in dataset().kana() {
        for (index, median) in kana.medians.iter().enumerate() {
            for point in median {
                assert!(
                    (0.0..=1024.0).contains(&point.x) && (0.0..=1024.0).contains(&point.y),
                    "{} stroke {} has a point at ({}, {}), outside the 1024 box",
                    kana.ch,
                    index + 1,
                    point.x,
                    point.y
                );
            }
        }
    }
}

/// Grading resamples each stroke and compares shapes, so a one-point stroke is
/// not a stroke.
#[test]
fn every_stroke_is_a_path() {
    for kana in dataset().kana() {
        for (index, median) in kana.medians.iter().enumerate() {
            assert!(
                median.len() >= 2,
                "{} stroke {} has only {} point(s)",
                kana.ch,
                index + 1,
                median.len()
            );
        }
    }
}

#[test]
fn kana_are_stored_in_gojuon_order() {
    let dataset = dataset();
    let codepoints: Vec<u32> = dataset.kana().iter().map(|k| k.ch as u32).collect();
    let mut sorted = codepoints.clone();
    sorted.sort_unstable();
    assert_eq!(
        codepoints, sorted,
        "code-point order is the gojūon order the course follows"
    );
    assert_eq!(
        codepoints.len(),
        codepoints.iter().collect::<std::collections::BTreeSet<_>>().len(),
        "no kana appears twice"
    );
}

#[test]
fn the_script_of_each_kana_matches_its_block() {
    for kana in dataset().kana() {
        assert_eq!(
            Script::of(kana.ch),
            Some(kana.script),
            "{} is filed under the wrong script",
            kana.ch
        );
    }
}

#[test]
fn the_choonpu_is_present_and_kanji_are_not() {
    let dataset = dataset();
    assert!(dataset.get('ー').is_some(), "the prolonged sound mark is taught");
    assert!(dataset.get('一').is_none(), "this artifact is kana only");
    assert!(dataset.get('あ').is_some());
    assert!(dataset.get('ア').is_some());
}

#[test]
fn the_whole_set_is_516_strokes() {
    let total: usize = dataset()
        .kana()
        .iter()
        .map(|k| k.stroke_count as usize)
        .sum();
    assert_eq!(total, 516);
}

#[test]
fn kana_can_be_graded_with_the_chinese_engine() {
    use hanzi_core::{grade, GradeOptions, Point};

    // The engine is shared with the Chinese app, so this is less a test of
    // grading than of the claim that a kana is gradeable without any of it being
    // specialised: a plausible horizontal stroke over 一-like geometry against
    // ー, which is exactly one horizontal stroke.
    let dataset = dataset();
    let kana = dataset.get('ー').expect("ー is in the dataset");
    let reference = kana.reference_medians();
    assert_eq!(reference.len(), 1);

    // Trace the reference centre-line itself; a correct trace must score well.
    let attempt = vec![reference[0].clone()];
    let report = grade(reference, &attempt, &GradeOptions::default());
    assert!(
        report.legible,
        "tracing the stored centre-line must be legible, got {:.0}",
        report.overall
    );

    // And something clearly in the wrong place must not be.
    let elsewhere = vec![vec![Point::new(20.0, 20.0), Point::new(60.0, 20.0)]];
    let wrong = grade(reference, &elsewhere, &GradeOptions::default());
    assert!(
        !wrong.legible,
        "a short stroke in the corner must not pass as ー, got {:.0}",
        wrong.overall
    );
}
