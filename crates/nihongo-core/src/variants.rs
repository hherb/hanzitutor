//! A hand that joins strokes, and grading one.
//!
//! The geometry an artifact stores is *taught* stroke by stroke — き is four, さ
//! is three — but a hand does not always lift the pen where the teaching does.
//! さ is very commonly written in two strokes, き in three, and ゆ, る, ろ, ね,
//! れ and わ are all written joined by many hands; `ROADMAP_NIHONGO.md`'s known
//! weak spots carried the concern in the maintainer's own words: *a kana tutor
//! that rejects a legitimate hand is worse than no tutor*.
//!
//! It did, and the measurement is blunt. `GradeReport::legible` requires the
//! attempt's stroke count to **equal** the reference's, so a connected hand is
//! refused before shape is looked at: over all 177 kana, **0 of 339** single
//! adjacent joins were legible against the taught reference, and every one of
//! the six named variants failed.
//!
//! The repair is not to loosen the shared grader — the count gate is what makes
//! "you wrote two of the four strokes" a usable message, and `hanzi-core` grades
//! the Chinese app with it. It is to put the *reference* into the grouping the
//! hand used: when an attempt arrives with fewer strokes than the kana is taught
//! with, every way of drawing the taught strokes as that many by joining
//! adjacent ones is graded, and the best **legible** one is the verdict.
//!
//! ```no_run
//! use nihongo_core::{grade_kana, KanaDataset};
//! use hanzi_core::GradeOptions;
//!
//! # fn main() -> std::io::Result<()> {
//! let bytes = std::fs::read("crates/nihongo-core/data/kana.bin.gz")?;
//! let dataset = KanaDataset::from_gzip_bytes(&bytes)?;
//! let sa = dataset.get('さ').expect("さ is in the dataset");
//!
//! // The taught form is three strokes and grades exactly as it always did.
//! let taught = grade_kana(sa, &sa.medians, &GradeOptions::default());
//! assert!(taught.joined.is_empty());
//!
//! // Drawn as two — the strokes a hand most often joins — it is still さ.
//! let mut joined = sa.medians.clone();
//! let second = joined.remove(1);
//! joined[0].extend(second);
//! let graded = grade_kana(sa, &joined, &GradeOptions::default());
//! assert!(graded.report.legible);
//! assert_eq!(graded.joined, vec![vec![1, 2]]);
//! # Ok(())
//! # }
//! ```
//!
//! Three properties keep it from becoming a way to pass with fewer strokes:
//!
//! * **The taught form is the only form tried when the count matches**, so
//!   grading a taught attempt is the shared engine's answer, unchanged.
//! * **A regrouped form is only returned when every drawn stroke clears the
//!   per-stroke bars** — see [`every_group_is_solid`], which is what tells a
//!   join from a dropped stroke. Mean-based legibility cannot: a missing stroke
//!   costs only its share of the average, and omissions measured as *legible* in
//!   339 candidate groupings before the bars were added.
//! * **A form is never tried for an attempt with *more* strokes than the
//!   reference** — there is no honest way to split a taught stroke, so a hand
//!   that lifts mid-stroke is still told its count is wrong.

use hanzi_core::{
    grade_with_outlines, GradeOptions, GradeReport, Point, StrokeVerdict, INK_OK, POSITION_OK,
    SHAPE_OK,
};

use crate::kana::Kana;

/// The verdict on a handwritten kana, and the form it was read as.
#[derive(Clone, Debug)]
pub struct KanaGrade {
    pub report: GradeReport,
    /// Set when the attempt was read as a **joined** form: the taught strokes the
    /// hand drew as one, 1-based and in taught order — `vec![vec![3, 4]]` for a
    /// き whose third and fourth strokes were written together. Empty for the
    /// taught form.
    ///
    /// The screen wants this for two reasons rather than one: it is the honest
    /// answer to "the prompt says four strokes and you drew three", and the
    /// report's own `refIndex` numbers the *drawn* strokes, so a per-stroke list
    /// would otherwise label the third drawn stroke "3" when it is taught strokes
    /// 3 and 4.
    pub joined: Vec<Vec<u8>>,
}

/// Grade a handwritten kana, accepting a hand that joins adjacent strokes.
///
/// See the module docs for the rule and why it is safe. The taught reference is
/// what an attempt of the taught stroke count is graded against, always; the
/// regrouped references are only built when the attempt has *fewer* strokes.
pub fn grade_kana(kana: &Kana, attempt: &[Vec<Point>], options: &GradeOptions) -> KanaGrade {
    let taught = grade_with_outlines(kana.reference_medians(), &kana.outlines, attempt, options);

    let taught_count = kana.medians.len();
    let given = attempt.len();
    if given == 0 || given >= taught_count {
        return KanaGrade {
            report: taught,
            joined: Vec::new(),
        };
    }

    // Fewer strokes than taught: the hand joined `taught_count - given` places.
    // Every way of grouping the taught strokes into `given` contiguous runs is a
    // reading of the attempt; the best one that survives [`every_group_is_solid`]
    // wins, so an illegible attempt still gets the taught reference's "four
    // strokes expected, three written" rather than a grouping that happens to fit
    // a wrong drawing.
    let mut best: Option<(GradeReport, Vec<Vec<u8>>)> = None;
    for groups in groupings(taught_count, given) {
        let (medians, outlines) = regroup(&kana.medians, &kana.outlines, &groups);
        let report = grade_with_outlines(&medians, &outlines, attempt, options);
        if !report.legible || !every_group_is_solid(&report.strokes) {
            continue;
        }
        let joined: Vec<Vec<u8>> = groups
            .iter()
            .filter(|(start, end)| end - start > 1)
            .map(|(start, end)| ((start + 1) as u8..=(*end) as u8).collect())
            .collect();
        if best
            .as_ref()
            .is_none_or(|(best, _)| report.overall > best.overall)
        {
            best = Some((report, joined));
        }
    }

    match best {
        Some((report, joined)) => KanaGrade { report, joined },
        None => KanaGrade {
            report: taught,
            joined: Vec::new(),
        },
    }
}

/// True when every drawn stroke of a regrouped attempt clears the same per-stroke
/// bars the shared grader uses to call one faulty.
///
/// This is the guard that keeps "the hand joined two strokes" from becoming "the
/// hand dropped one", and it is measured rather than assumed. An omission is
/// *legible* under the mean-based rule — the strokes that are there are perfect,
/// and the missing geometry costs only its share of the average — so the mean
/// cannot tell the two apart. The per-stroke bars can:
///
/// * over every single omission of every kana, **0 of 497** attempts are accepted
///   — where mean-based legibility alone would have passed 339 of the candidate
///   groupings, which is the whole reason the bars are here;
/// * over every adjacent join of every kana drawn by a hand, **331 of 339**
///   cleared them at jitter 15 and 259 at jitter 30.
///
/// The eight joins it refuses are all one kind — a dakuten or handakuten stroke
/// joined to the stroke beside it (ぶ 5+6, ぷ 3+4, ポ 2+3, バ 3+4, ヹ 1+2,
/// ぎ 5+6, ズ 3+4, ぜ 4+5) — which is not a hand form anyone teaches, and the
/// attempt falls back to the taught reference's verdict rather than being
/// accepted as something it is not. `tests` pins both halves.
fn every_group_is_solid(strokes: &[StrokeVerdict]) -> bool {
    strokes.iter().all(|stroke| {
        stroke.shape >= SHAPE_OK && stroke.position >= POSITION_OK && stroke.ink >= INK_OK
    })
}

/// Every way to draw `count` strokes by joining `n` adjacent taught ones, as
/// half-open `[start, end)` runs in taught order.
///
/// A composition of `n` into `count` positive parts: `C(n - 1, count - 1)` of
/// them, which is ten at worst for the widest kana and nothing at all for a kana
/// drawn with the taught number of strokes, where this is not reached.
fn groupings(n: usize, count: usize) -> Vec<Vec<(usize, usize)>> {
    if count == 0 || count > n {
        return Vec::new();
    }
    if count == 1 {
        return vec![vec![(0, n)]];
    }
    let mut out = Vec::new();
    // The first run ends at `end`; the rest still has to make `count - 1` runs.
    for end in 1..=(n - count + 1) {
        for rest in groupings(n - end, count - 1) {
            let mut groups = vec![(0usize, end)];
            groups.extend(rest.into_iter().map(|(start, stop)| (start + end, stop + end)));
            out.push(groups);
        }
    }
    out
}

/// The reference a joined attempt is measured against: the same runs, with the
/// centre-lines concatenated and the outlines joined into one path per run.
///
/// Concatenating SVG path data is valid — a path may hold any number of
/// subpaths — so the ink measure still sees the ink of both taught strokes.
fn regroup(
    medians: &[Vec<Point>],
    outlines: &[String],
    groups: &[(usize, usize)],
) -> (Vec<Vec<Point>>, Vec<String>) {
    let mut regrouped_medians = Vec::with_capacity(groups.len());
    let mut regrouped_outlines = Vec::with_capacity(groups.len());
    for &(start, end) in groups {
        let mut points = Vec::new();
        for median in &medians[start..end] {
            points.extend_from_slice(median);
        }
        regrouped_medians.push(points);
        regrouped_outlines.push(
            outlines[start..end]
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    (regrouped_medians, regrouped_outlines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kana::KanaDataset;

    /// Deterministic noise, so a test that depends on a "hand" is repeatable.
    struct Lcg(u64);
    impl Lcg {
        fn normal(&mut self) -> f32 {
            let mut sum = 0.0;
            for _ in 0..4 {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                sum += ((self.0 >> 11) as f64) / ((1u64 << 53) as f64);
            }
            ((sum - 2.0) * 2.0) as f32
        }
    }

    fn jitter(strokes: &[Vec<Point>], sigma: f32, rng: &mut Lcg) -> Vec<Vec<Point>> {
        strokes
            .iter()
            .map(|s| {
                s.iter()
                    .map(|p| Point::new(p.x + rng.normal() * sigma, p.y + rng.normal() * sigma))
                    .collect()
            })
            .collect()
    }

    fn dataset() -> KanaDataset {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/data/kana.bin.gz"))
            .expect("the committed kana artifact");
        KanaDataset::from_gzip_bytes(&bytes).expect("it decodes")
    }

    /// The strokes at `indices`, in taught order.
    fn take(kana: &Kana, indices: &[usize]) -> Vec<Vec<Point>> {
        indices.iter().map(|&i| kana.medians[i].clone()).collect()
    }

    /// `kana`'s taught strokes with `start..=end` drawn as one.
    fn join(kana: &Kana, start: usize, end: usize) -> Vec<Vec<Point>> {
        let mut strokes = kana.medians.clone();
        let tail = strokes.remove(end);
        strokes[start].extend(tail);
        strokes
    }

    #[test]
    fn the_taught_form_is_the_shared_engines_answer_unchanged() {
        let dataset = dataset();
        for kana in dataset.kana() {
            let graded = grade_kana(kana, &kana.medians, &GradeOptions::default());
            assert!(
                graded.report.is_perfect(),
                "{} is not perfect against itself: {:.2}",
                kana.ch,
                graded.report.overall
            );
            assert!(
                graded.joined.is_empty(),
                "{} was read as a joined form when it is the taught one",
                kana.ch
            );
        }
    }

    #[test]
    fn a_hand_drawn_join_is_read_as_a_join_for_every_kana_but_the_dakuten_ones() {
        // What the app used to refuse: every adjacent join of every kana, drawn
        // as a hand draws it. The eight the per-stroke bars refuse are named
        // rather than tolerated, so a ninth is noticed — and every one of them
        // joins a dakuten or handakuten stroke to the stroke beside it, which is
        // not a hand form anyone teaches. Refusing them costs nothing: the
        // attempt falls back to the taught reference's verdict.
        const REFUSED: &[(char, usize)] = &[
            ('ぎ', 5),
            ('ぜ', 4),
            ('ぶ', 5),
            ('ぷ', 3),
            ('ズ', 3),
            ('バ', 3),
            ('ポ', 2),
            ('ヹ', 1),
        ];
        let dataset = dataset();
        let mut rng = Lcg(0x5EED);
        let mut joins = 0usize;
        let mut refused = 0usize;
        for kana in dataset.kana() {
            if kana.medians.len() < 2 {
                continue;
            }
            for start in 0..kana.medians.len() - 1 {
                let attempt = jitter(&join(kana, start, start + 1), 15.0, &mut rng);
                let graded = grade_kana(kana, &attempt, &GradeOptions::default());
                joins += 1;
                if REFUSED.contains(&(kana.ch, start + 1)) {
                    refused += 1;
                    assert!(
                        graded.joined.is_empty() && !graded.report.legible,
                        "{} {}+{} is on the refused list but was accepted as a join",
                        kana.ch,
                        start + 1,
                        start + 2
                    );
                    assert_eq!(
                        graded.report.expected_strokes,
                        kana.medians.len(),
                        "a refused join falls back to the taught reference"
                    );
                    continue;
                }
                assert!(
                    graded.report.legible,
                    "{} drawn with stroke {}+{} joined is not legible: {:.1}",
                    kana.ch,
                    start + 1,
                    start + 2,
                    graded.report.overall
                );
                assert_eq!(
                    graded.joined,
                    vec![vec![(start + 1) as u8, (start + 2) as u8]],
                    "{} {}+{} was not named",
                    kana.ch,
                    start + 1,
                    start + 2
                );
            }
        }
        assert_eq!(joins, 339, "the kana dataset's adjacent joins");
        assert_eq!(
            refused,
            REFUSED.len(),
            "a join on the refused list was accepted, or a new one was refused"
        );
    }

    #[test]
    fn the_named_variants_are_read_as_the_hand_wrote_them() {
        let dataset = dataset();
        let mut rng = Lcg(0x5EED);
        // き (four taught) joined 3+4, and さ (three taught) joined 1+2: the two
        // the roadmap named, each in the grouping a hand uses.
        let ki = dataset.get('き').expect("き");
        let graded = grade_kana(
            ki,
            &jitter(&join(ki, 2, 3), 15.0, &mut rng),
            &GradeOptions::default(),
        );
        assert!(graded.report.legible, "{:.1}", graded.report.overall);
        assert_eq!(graded.joined, vec![vec![3, 4]]);

        let sa = dataset.get('さ').expect("さ");
        let graded = grade_kana(
            sa,
            &jitter(&join(sa, 0, 1), 15.0, &mut rng),
            &GradeOptions::default(),
        );
        assert!(graded.report.legible, "{:.1}", graded.report.overall);
        assert_eq!(graded.joined, vec![vec![1, 2]]);
    }

    #[test]
    fn an_attempt_missing_a_stroke_is_not_read_as_a_join() {
        // The risk the fix carries: an omission arriving as "one fewer stroke"
        // and being waved through as though the hand had joined something. Every
        // single omission of every kana, with everything else drawn correctly.
        let dataset = dataset();
        let mut legible = Vec::new();
        for kana in dataset.kana() {
            if kana.medians.len() < 2 {
                continue;
            }
            for missing in 0..kana.medians.len() {
                let indices: Vec<usize> =
                    (0..kana.medians.len()).filter(|&i| i != missing).collect();
                let graded =
                    grade_kana(kana, &take(kana, &indices), &GradeOptions::default());
                if graded.report.legible {
                    legible.push((kana.ch, missing + 1, graded.report.overall));
                }
            }
        }
        assert!(
            legible.is_empty(),
            "{} omissions were accepted as joins: {legible:?}",
            legible.len()
        );
    }

    #[test]
    fn an_attempt_with_more_strokes_than_taught_is_still_refused() {
        // A hand that lifts mid-stroke. There is no honest way to split a taught
        // stroke, so this stays the taught reference's answer — pinned over the
        // whole dataset rather than one kana, because "more strokes" is not a
        // shape any regrouping can be built for.
        let dataset = dataset();
        let mut attempts = 0usize;
        for kana in dataset.kana() {
            if kana.medians.len() < 2 {
                continue;
            }
            let Some(index) = kana.medians.iter().position(|m| m.len() >= 2) else {
                continue;
            };
            let median = &kana.medians[index];
            let half = median.len() / 2;
            let (first, second) = (median[..half].to_vec(), median[half..].to_vec());
            let options = GradeOptions::default();
            if hanzi_core::geom::path_length(&first) < options.min_stroke_len
                || hanzi_core::geom::path_length(&second) < options.min_stroke_len
            {
                continue;
            }
            let mut split = kana.medians.clone();
            split.splice(index..=index, [first, second]);
            attempts += 1;
            let graded = grade_kana(kana, &split, &options);
            assert!(
                !graded.report.legible && graded.joined.is_empty(),
                "{} with stroke {} split in two was accepted: {:.1}",
                kana.ch,
                index + 1,
                graded.report.overall
            );
            assert_eq!(
                graded.report.expected_strokes,
                kana.medians.len(),
                "the taught reference's count is what comes back"
            );
        }
        assert_eq!(attempts, 95, "the kana set's splittable single strokes");
    }

    #[test]
    fn an_illegible_attempt_keeps_the_taught_reference() {
        // A wrong drawing with fewer strokes must not be rescued by whatever
        // grouping fits it worst; the taught verdict is the useful message.
        let dataset = dataset();
        let kana = dataset.get('き').expect("き");
        let attempt = vec![vec![Point::new(120.0, 880.0), Point::new(900.0, 890.0)]];
        let graded = grade_kana(kana, &attempt, &GradeOptions::default());
        assert!(!graded.report.legible);
        assert!(graded.joined.is_empty());
        assert_eq!(graded.report.expected_strokes, 4, "four taught strokes expected");
    }

    #[test]
    fn groupings_enumerate_every_composition() {
        assert_eq!(groupings(3, 1), vec![vec![(0, 3)]]);
        assert_eq!(
            groupings(3, 2),
            vec![vec![(0, 1), (1, 3)], vec![(0, 2), (2, 3)]]
        );
        assert_eq!(groupings(3, 3), vec![vec![(0, 1), (1, 2), (2, 3)]]);
        assert_eq!(groupings(4, 2).len(), 3);
        assert_eq!(groupings(6, 3).len(), 10, "C(5, 2)");
        assert!(groupings(3, 0).is_empty());
        assert!(groupings(3, 4).is_empty());
    }
}
