//! Grading a handwritten attempt against a reference character.
//!
//! # How grading works
//!
//! The reference data gives one centre-line ("median") per stroke, in the
//! correct stroke order. An attempt is a list of polylines recorded from the
//! canvas. Grading separates two questions that are easy to confuse:
//!
//! 1. **Did you write the right strokes, in the right places?** — solved as an
//!    assignment problem. Every attempt stroke is scored against every
//!    reference stroke on shape (scale- and position-invariant) and on
//!    placement (absolute position and size in the character box), then the
//!    globally cheapest one-to-one pairing is found with the Hungarian
//!    algorithm. Unmatched reference strokes count as missing.
//! 2. **Did you write them in the right order?** — the pairing gives each
//!    attempt stroke a reference index. Read those indices in the order the
//!    user drew them; the longest strictly increasing subsequence is the
//!    largest set of strokes that *are* in order, and everything outside it is
//!    flagged individually. The headline order score is the normalised
//!    inversion count (Kendall tau), which is smooth: one adjacent swap in a
//!    ten-stroke character costs about 2%, while writing a character
//!    completely backwards scores zero.
//!
//! All three headline scores — shape, placement and order — are measured across
//! the whole reference character, so a missing stroke is penalised everywhere it
//! should be and a blank canvas scores zero rather than collecting easy marks
//! for a flawless ordering of nothing.
//!
//! Separating the two means a character written beautifully but in the wrong
//! order still reports as legible, with the order faults called out
//! individually — which is exactly the feedback a learner needs.

use serde::{Deserialize, Serialize};

use crate::geom::{self, Point};

/// Normalised shape distance at which the shape score reaches zero.
///
/// Tuned against the real dataset (`cargo run -p hanzi-core --example
/// selfcheck`). A correct stroke drawn with a realistic hand wobble sits at a
/// normalised distance of roughly 0.14 at 1.5% jitter and 0.27 at 3% jitter,
/// with a 95th percentile around 0.47. This tolerance is set well above that so
/// a shaky trackpad attempt is never failed for shape alone: shape distance is
/// deliberately scale-invariant, so it cannot tell two 横 strokes of different
/// lengths apart, and stroke discrimination is the position term's job. What
/// this score *does* catch is a stroke of the wrong kind entirely — a
/// perpendicular stroke sits near 0.8 and scores about 0.3.
const SHAPE_TOL: f32 = 1.10;
/// Centroid offset, in design units out of 1024, at which placement scores zero.
const POSITION_TOL: f32 = 170.0;
/// Relative bounding-box size mismatch at which placement scores zero.
const SIZE_TOL: f32 = 0.65;
/// Cost of leaving a stroke unmatched. Higher than any real pairing, so the
/// solver always prefers to match two real strokes when one is plausible.
const NO_MATCH_COST: f64 = 1.15;
const W_SHAPE: f64 = 0.60;
const W_POSITION: f64 = 0.40;
/// Below these per-stroke scores a stroke is reported as faulty.
const SHAPE_OK: f32 = 0.60;
const POSITION_OK: f32 = 0.60;

/// Tunables for a grading run.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeOptions {
    /// Points each polyline is resampled to before comparison.
    pub resample_k: usize,
    /// Strokes shorter than this (design units, out of 1024) are treated as
    /// accidental taps rather than strokes.
    pub min_stroke_len: f32,
    /// Fit the attempt's overall position and scale onto the reference before
    /// grading placement. Makes grading forgiving of a uniformly
    /// shifted or differently-sized character, which suits drawing from
    /// memory on a trackpad. Trace mode turns this off so that drifting off the
    /// guide is penalised.
    pub global_fit: bool,
}

impl Default for GradeOptions {
    fn default() -> Self {
        Self {
            resample_k: 16,
            min_stroke_len: 12.0,
            global_fit: true,
        }
    }
}

impl GradeOptions {
    /// Strict, position-sensitive grading — used when tracing a visible guide.
    pub fn tracing() -> Self {
        Self {
            global_fit: false,
            ..Self::default()
        }
    }
}

/// What went wrong with one reference stroke.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Written correctly, in the right order.
    Correct,
    /// Right stroke in roughly the right place, but the shape is wrong.
    ShapeOff,
    /// Recognisable stroke, but in the wrong place or the wrong size.
    PositionOff,
    /// Drawn back to front.
    WrongDirection,
    /// Correct stroke, written at the wrong point in the sequence.
    OutOfOrder,
    /// Never written.
    Missing,
}

impl Verdict {
    fn is_error(self) -> bool {
        self != Verdict::Correct
    }
}

/// Overall quality band for an attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grade {
    Excellent,
    Good,
    Fair,
    Poor,
}

/// How the attempt sat in the character box, before any fitting was applied.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FitInfo {
    /// Uniform scale applied to align the attempt (1.0 = already the right size).
    pub scale: f32,
    /// Distance between attempt and reference centroids, in design units.
    pub offset: f32,
}

/// Per-reference-stroke feedback. `ref_index` indexes into the reference
/// character's stroke list, which is what a UI should colour.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrokeVerdict {
    pub ref_index: usize,
    /// Index into the *original* attempt stroke list, if this stroke was
    /// written at all. Indices are never renumbered, so a UI can colour the
    /// user's own strokes directly.
    pub user_index: Option<usize>,
    pub verdict: Verdict,
    pub shape: f32,
    pub position: f32,
    /// Combined quality of this stroke, 0..=1.
    pub score: f32,
}

/// The result of grading one attempt.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeReport {
    pub expected_strokes: usize,
    /// Strokes counted after discarding accidental taps.
    pub given_strokes: usize,
    /// Marks too short to be strokes.
    pub stray_strokes: usize,
    pub count_ok: bool,
    /// One entry per reference stroke, in reference order.
    pub strokes: Vec<StrokeVerdict>,
    /// `assignment[user_index] = ref_index`, for colouring user strokes.
    pub assignment: Vec<Option<usize>>,
    pub shape_score: f32,
    pub position_score: f32,
    /// How much of the character was written in the correct order, 0..=1.
    /// `1.0` requires every stroke to be present *and* perfectly sequenced.
    pub order_score: f32,
    /// Headline score, 0..=100.
    pub overall: f32,
    /// Are the strokes recognisable and correctly placed? Independent of order:
    /// a legible character may still have been written in the wrong order.
    pub legible: bool,
    pub order_correct: bool,
    pub grade: Grade,
    /// Lowest-indexed stroke that needs attention, for "fix this next" UX.
    pub first_error: Option<usize>,
    pub fit: Option<FitInfo>,
}

impl GradeReport {
    /// True when every stroke is correct, present and in order.
    pub fn is_perfect(&self) -> bool {
        self.count_ok && self.order_correct && self.strokes.iter().all(|s| !s.verdict.is_error())
    }
}

/// A stroke kept for grading, remembering where it came from.
struct Kept {
    original_index: usize,
    points: Vec<Point>,
}

/// Metrics for one (attempt stroke, reference stroke) pair.
#[derive(Clone, Copy)]
struct PairMetrics {
    shape: f32,
    position: f32,
    reversed: bool,
}

pub fn shape_score(distance: f32) -> f32 {
    (1.0 - distance / SHAPE_TOL).clamp(0.0, 1.0)
}

/// How close the stroke sits to the reference, in absolute box coordinates.
///
/// Combines centroid offset with bounding-box size mismatch, so both a stroke
/// in the wrong place and one that is grossly the wrong length are caught.
fn position_score(points: &[Point], reference: &[Point]) -> f32 {
    let Some((min_u, max_u)) = geom::bbox(points) else {
        return 0.0;
    };
    let Some((min_r, max_r)) = geom::bbox(reference) else {
        return 0.0;
    };
    let centroid_offset = geom::centroid(points).distance_to(geom::centroid(reference));
    let diag = |a: Point, b: Point| a.distance_to(b);
    let du = diag(min_u, max_u);
    let dr = diag(min_r, max_r);
    let size_mismatch = if du.max(dr) > 1e-3 {
        (du - dr).abs() / du.max(dr)
    } else {
        0.0
    };
    let penalty = 0.65 * (centroid_offset / POSITION_TOL) + 0.35 * (size_mismatch / SIZE_TOL);
    (1.0 - penalty).clamp(0.0, 1.0)
}

/// Grade `attempt` against a reference character's stroke medians.
///
/// Both the reference medians and the attempt are expected in display space
/// (origin top-left, y growing downwards, box `0..=1024`); see [`crate::geom`].
pub fn grade(
    reference_medians: &[Vec<Point>],
    attempt: &[Vec<Point>],
    options: &GradeOptions,
) -> GradeReport {
    let expected = reference_medians.len();

    // 1. Discard marks too short to be strokes, keeping original indices.
    //
    // The threshold is relative as well as absolute. A handful of characters in
    // the source data have a genuinely tiny stroke (黧's shortest is 4 design
    // units), and a fixed cutoff would throw that real stroke away as a tap,
    // making a correct attempt impossible to score.
    let shortest_reference = reference_medians
        .iter()
        .map(|m| geom::path_length(m))
        .fold(f32::INFINITY, f32::min);
    let stray_below = options.min_stroke_len.min(shortest_reference * 0.5);

    let mut kept = Vec::new();
    let mut stray = 0usize;
    for (i, stroke) in attempt.iter().enumerate() {
        if geom::path_length(stroke) < stray_below {
            stray += 1;
        } else {
            kept.push(Kept {
                original_index: i,
                points: stroke.clone(),
            });
        }
    }

    // 2. Score and pair the strokes.
    //
    // Fitting, when enabled, needs the pairing first, so that the similarity
    // transform is derived from strokes that actually correspond rather than
    // from the attempt's raw extent. Fitting raw extents works when a whole
    // character is written but is unfair otherwise: writing 1 stroke of a
    // 2-stroke character would align that single stroke against the whole
    // character box and then penalise it for being the wrong size.
    let n_ref = expected;
    let mut pairing = score_and_assign(&kept, reference_medians, options, attempt.len());

    // How far the attempt sat from the reference, measured on the pairing we
    // just made. Reported whether or not a fit is applied, so a UI can explain
    // a placement penalty as "drawn too small" or "off to one side".
    let initial_fit = fit_on_matched(&kept, reference_medians, &pairing);
    let fit_info = initial_fit.map(|(fit, offset)| FitInfo {
        scale: fit.scale,
        offset,
    });

    if options.global_fit {
        if let Some((fit, _)) = initial_fit {
            for k in kept.iter_mut() {
                k.points = k.points.iter().map(|p| fit.apply(*p)).collect();
            }
            pairing = score_and_assign(&kept, reference_medians, options, attempt.len());
        }
    }

    let n_user = kept.len();
    let Pairing {
        metrics,
        user_of_ref,
        assignment,
    } = pairing;

    // 5. Order: the reference indices, in the order the user drew them.
    let refs_in_draw_order: Vec<usize> = {
        let mut drawn: Vec<(usize, usize)> = Vec::new(); // (user i, ref j)
        for i in 0..n_user {
            if let Some(j) = assignment[kept[i].original_index] {
                drawn.push((i, j));
            }
        }
        drawn.sort_by_key(|(i, _)| *i);
        drawn.iter().map(|(_, j)| *j).collect()
    };
    let order_flags = out_of_order_flags(&refs_in_draw_order);
    let ordered_score = order_score(&refs_in_draw_order);
    // Re-key the per-position flags by reference stroke, ready for reporting.
    let mut out_of_order = vec![false; n_ref];
    for (pos, &j) in refs_in_draw_order.iter().enumerate() {
        out_of_order[j] = order_flags[pos];
    }

    // 6. Assemble per-stroke feedback.
    let mut strokes = Vec::with_capacity(n_ref);
    for j in 0..n_ref {
        let Some(i) = user_of_ref[j] else {
            strokes.push(StrokeVerdict {
                ref_index: j,
                user_index: None,
                verdict: Verdict::Missing,
                shape: 0.0,
                position: 0.0,
                score: 0.0,
            });
            continue;
        };
        let m = metrics[i][j].expect("matched pairs were scored");
        let verdict = if m.shape < SHAPE_OK {
            Verdict::ShapeOff
        } else if m.position < POSITION_OK {
            Verdict::PositionOff
        } else if m.reversed {
            Verdict::WrongDirection
        } else if out_of_order[j] {
            Verdict::OutOfOrder
        } else {
            Verdict::Correct
        };
        strokes.push(StrokeVerdict {
            ref_index: j,
            user_index: Some(kept[i].original_index),
            verdict,
            shape: m.shape,
            position: m.position,
            score: 0.5 * m.shape + 0.5 * m.position,
        });
    }

    // 7. Aggregate. Every score is measured across the *whole* reference
    // character, with unwritten strokes counting as zero. An incomplete attempt
    // is therefore penalised consistently in shape, placement and order, and a
    // blank canvas scores nothing at all. Measuring order only over the strokes
    // that were written would let "I wrote one stroke correctly" or even "I
    // wrote nothing" collect full marks for stroke order.
    let denom = n_ref.max(1) as f32;
    let matched = strokes.iter().filter(|s| s.user_index.is_some()).count();
    let coverage = if n_ref == 0 {
        1.0
    } else {
        matched as f32 / n_ref as f32
    };
    let shape_score = strokes.iter().map(|s| s.shape).sum::<f32>() / denom;
    let position_score = strokes.iter().map(|s| s.position).sum::<f32>() / denom;
    let order_score = coverage * ordered_score;
    let content = 0.6 * shape_score + 0.4 * position_score;
    let overall = (100.0 * (0.70 * content + 0.30 * order_score)).clamp(0.0, 100.0);
    let grade = match overall {
        s if s >= 92.0 => Grade::Excellent,
        s if s >= 75.0 => Grade::Good,
        s if s >= 55.0 => Grade::Fair,
        _ => Grade::Poor,
    };

    GradeReport {
        expected_strokes: expected,
        given_strokes: n_user,
        stray_strokes: stray,
        count_ok: n_user == n_ref,
        shape_score,
        position_score,
        order_score,
        overall,
        legible: n_ref > 0
            && n_user == n_ref
            && shape_score >= 0.60
            && position_score >= 0.60,
        order_correct: n_user == n_ref && order_score >= 0.999,
        grade,
        first_error: strokes.iter().find(|s| s.verdict.is_error()).map(|s| s.ref_index),
        strokes,
        assignment,
        fit: fit_info,
    }
}

/// The outcome of scoring an attempt against a reference and pairing them up.
struct Pairing {
    /// `metrics[kept_index][ref_index]`.
    metrics: Vec<Vec<Option<PairMetrics>>>,
    /// `user_of_ref[j] = Some(kept_index)` for pairs the solver actually used.
    user_of_ref: Vec<Option<usize>>,
    /// `assignment[original_attempt_index] = Some(ref_index)`.
    assignment: Vec<Option<usize>>,
}

/// Score every attempt stroke against every reference stroke, then choose the
/// cheapest one-to-one pairing, allowing strokes on either side to go unmatched.
fn score_and_assign(
    kept: &[Kept],
    reference: &[Vec<Point>],
    options: &GradeOptions,
    attempt_len: usize,
) -> Pairing {
    let n_user = kept.len();
    let n_ref = reference.len();

    let mut metrics: Vec<Vec<Option<PairMetrics>>> = vec![vec![None; n_ref]; n_user];
    for i in 0..n_user {
        for j in 0..n_ref {
            let sd = geom::shape_distance(&kept[i].points, &reference[j], options.resample_k);
            metrics[i][j] = Some(PairMetrics {
                // Pairing uses the better of the two directions: a stroke drawn
                // backwards is still the right stroke, and is reported
                // separately as a direction fault.
                shape: shape_score(sd.best()),
                position: position_score(&kept[i].points, &reference[j]),
                reversed: sd.is_reversed(),
            });
        }
    }

    let size = n_user.max(n_ref);
    let mut cost = vec![vec![NO_MATCH_COST; size]; size];
    for i in 0..n_user {
        for j in 0..n_ref {
            let m = metrics[i][j].expect("scored just above");
            cost[i][j] =
                W_SHAPE * (1.0 - m.shape as f64) + W_POSITION * (1.0 - m.position as f64);
        }
    }
    let solve = hungarian(&cost);

    let mut user_of_ref = vec![None; n_ref];
    let mut assignment = vec![None; attempt_len];
    for i in 0..n_user {
        let j = solve[i];
        if j < n_ref && cost[i][j] < NO_MATCH_COST {
            user_of_ref[j] = Some(i);
            assignment[kept[i].original_index] = Some(j);
        }
    }

    Pairing {
        metrics,
        user_of_ref,
        assignment,
    }
}

/// The similarity transform implied by the pairs that were actually matched,
/// with the distance between the two centroids before it is applied.
///
/// Returns `None` when nothing matched, or when the matched points carry no
/// extent to scale against.
fn fit_on_matched(
    kept: &[Kept],
    reference: &[Vec<Point>],
    pairing: &Pairing,
) -> Option<(geom::Fit, f32)> {
    let mut user_points: Vec<Point> = Vec::new();
    let mut ref_points: Vec<Point> = Vec::new();
    for k in kept {
        let Some(j) = pairing.assignment.get(k.original_index).copied().flatten() else {
            continue;
        };
        user_points.extend_from_slice(&k.points);
        ref_points.extend_from_slice(&reference[j]);
    }
    if user_points.is_empty() || ref_points.is_empty() {
        return None;
    }
    let fit = geom::fit_similarity(&user_points, &ref_points);
    let offset = geom::centroid(&user_points).distance_to(geom::centroid(&ref_points));
    Some((fit, offset))
}

/// Solve the square assignment problem, minimising total cost.
///
/// This is the O(n^3) Hungarian algorithm (Kuhn–Munkres) in the compact
/// potential-updating form, with row 0 / column 0 used as sentinels.
/// `cost` must be square and non-negative. Returns `assign[row] = col`.
fn hungarian(cost: &[Vec<f64>]) -> Vec<usize> {
    let n = cost.len();
    if n == 0 {
        return Vec::new();
    }
    debug_assert!(cost.iter().all(|r| r.len() == n), "cost matrix must be square");

    let mut u = vec![0.0f64; n + 1];
    let mut v = vec![0.0f64; n + 1];
    let mut p = vec![0usize; n + 1];
    let mut way = vec![0usize; n + 1];

    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0usize;
        let mut minv = vec![f64::INFINITY; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = f64::INFINITY;
            let mut j1 = 0usize;
            for j in 1..=n {
                if used[j] {
                    continue;
                }
                let cur = cost[i0 - 1][j - 1] - u[i0] - v[j];
                if cur < minv[j] {
                    minv[j] = cur;
                    way[j] = j0;
                }
                if minv[j] < delta {
                    delta = minv[j];
                    j1 = j;
                }
            }
            for j in 0..=n {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }

    let mut assign = vec![usize::MAX; n];
    for j in 1..=n {
        if p[j] != 0 {
            assign[p[j] - 1] = j - 1;
        }
    }
    assign
}

/// How well an ordering of reference indices matches the correct sequence.
///
/// The normalised inversion count (Kendall tau distance): `1.0` for a perfectly
/// increasing sequence, `0.0` for a complete reversal. Counting inversions
/// rather than measuring a longest increasing subsequence keeps the measure
/// smooth and fair — one adjacent swap in a ten-stroke character costs about
/// 2%, while a three-stroke character written backwards scores zero instead of
/// a flattering one third.
fn order_score(sequence: &[usize]) -> f32 {
    let n = sequence.len();
    if n < 2 {
        return 1.0;
    }
    let inversions = inversion_count(sequence);
    let possible = n * (n - 1) / 2;
    1.0 - inversions as f32 / possible as f32
}

/// Number of pairs `i < j` with `sequence[i] > sequence[j]`.
fn inversion_count(sequence: &[usize]) -> usize {
    let mut count = 0;
    for i in 0..sequence.len() {
        for j in (i + 1)..sequence.len() {
            if sequence[i] > sequence[j] {
                count += 1;
            }
        }
    }
    count
}

/// Which strokes in a draw-order sequence are out of order.
///
/// A stroke is flagged exactly when it participates in an inversion: some
/// stroke drawn before it belongs after it, or some stroke drawn after it
/// belongs before it. Defining the flag this way makes it agree exactly with
/// [`order_score`] — the flagged strokes are precisely the ones that make up
/// the inversion count — and it flags *both* halves of a swap, rather than the
/// arbitrary single stroke a longest-increasing-subsequence approach picks.
fn out_of_order_flags(sequence: &[usize]) -> Vec<bool> {
    let n = sequence.len();
    let mut flags = vec![false; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if sequence[i] > sequence[j] {
                flags[i] = true;
                flags[j] = true;
            }
        }
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A straight line from `a` to `b`, sampled at `n` points.
    fn line(a: (f32, f32), b: (f32, f32), n: usize) -> Vec<Point> {
        (0..n)
            .map(|i| {
                let t = i as f32 / (n - 1) as f32;
                Point::new(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
            })
            .collect()
    }

    // ---- assignment solver -------------------------------------------------

    fn brute_force_min(cost: &[Vec<f64>]) -> f64 {
        fn recurse(cost: &[Vec<f64>], used: &mut Vec<bool>, row: usize, acc: f64, best: &mut f64) {
            if row == cost.len() {
                if acc < *best {
                    *best = acc;
                }
                return;
            }
            for col in 0..cost.len() {
                if used[col] {
                    continue;
                }
                used[col] = true;
                recurse(cost, used, row + 1, acc + cost[row][col], best);
                used[col] = false;
            }
        }
        let mut best = f64::INFINITY;
        let mut used = vec![false; cost.len()];
        recurse(cost, &mut used, 0, 0.0, &mut best);
        best
    }

    /// Deterministic pseudo-random values, so failures are reproducible.
    struct Lcg(u64);
    impl Lcg {
        fn next_f64(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
        }
    }

    #[test]
    fn hungarian_matches_brute_force() {
        let mut rng = Lcg(0xC0FFEE);
        for n in 1..=6 {
            for _ in 0..40 {
                let cost: Vec<Vec<f64>> = (0..n)
                    .map(|_| (0..n).map(|_| rng.next_f64() * 2.0).collect())
                    .collect();
                let assign = hungarian(&cost);
                // Every row gets a distinct column.
                let mut seen = vec![false; n];
                for &c in &assign {
                    assert!(c < n, "unassigned row in {cost:?}");
                    assert!(!seen[c], "column {c} assigned twice");
                    seen[c] = true;
                }
                let got: f64 = (0..n).map(|r| cost[r][assign[r]]).sum();
                let want = brute_force_min(&cost);
                assert!(
                    (got - want).abs() < 1e-9,
                    "n={n}: hungarian {got} vs brute force {want}\n{cost:?}"
                );
            }
        }
    }

    #[test]
    fn hungarian_handles_empty_and_single() {
        assert!(hungarian(&[]).is_empty());
        assert_eq!(hungarian(&[vec![3.0]]), vec![0]);
    }

    // ---- order analysis ---------------------------------------------------

    #[test]
    fn out_of_order_flags_mark_every_inverted_stroke() {
        assert!(out_of_order_flags(&[]).is_empty());
        assert_eq!(out_of_order_flags(&[0]), vec![false]);
        assert_eq!(out_of_order_flags(&[0, 1, 2, 3]), vec![false; 4]);

        // One adjacent swap flags both swapped strokes and nothing else.
        assert_eq!(
            out_of_order_flags(&[0, 2, 1, 3]),
            vec![false, true, true, false]
        );
        // The first stroke drawn too late: it and the one it jumped are flagged.
        assert_eq!(
            out_of_order_flags(&[1, 0, 2, 3]),
            vec![true, true, false, false]
        );
        // Written completely backwards: every stroke is inverted.
        assert_eq!(out_of_order_flags(&[2, 1, 0]), vec![true; 3]);
    }

    #[test]
    fn order_score_ranges_from_in_order_to_reversed() {
        assert_eq!(order_score(&[]), 1.0);
        assert_eq!(order_score(&[5]), 1.0);
        assert_eq!(order_score(&[0, 1, 2, 3]), 1.0);
        assert_eq!(order_score(&[3, 2, 1, 0]), 0.0);

        // A single adjacent swap out of ten strokes costs one inverted pair.
        let n = 10usize;
        let mut sequence: Vec<usize> = (0..n).collect();
        sequence.swap(4, 5);
        let possible = (n * (n - 1) / 2) as f32;
        assert!((order_score(&sequence) - (1.0 - 1.0 / possible)).abs() < 1e-6);

        // In-order exactly when nothing is flagged, by construction.
        for case in [&[0usize, 1, 2, 3][..], &[0, 2, 1, 3][..], &[2, 1, 0][..]] {
            let flagged = out_of_order_flags(case).iter().any(|f| *f);
            assert_eq!(flagged, order_score(case) < 1.0, "case {case:?}");
        }
    }

    // ---- end-to-end grading ----------------------------------------------

    /// A character shaped like 十: a horizontal stroke then a vertical one.
    fn shi_reference() -> Vec<Vec<Point>> {
        vec![
            line((120.0, 400.0), (900.0, 400.0), 5), // 横
            line((512.0, 120.0), (512.0, 900.0), 5), // 丨
        ]
    }

    #[test]
    fn perfect_attempt_scores_high_and_is_legible() {
        let reference = shi_reference();
        let attempt = reference.clone();
        let report = grade(&reference, &attempt, &GradeOptions::default());
        assert!(report.count_ok);
        assert!(report.order_correct, "{report:#?}");
        assert!(report.legible);
        assert!(report.overall > 95.0, "overall {}", report.overall);
        assert!(report.is_perfect(), "{report:#?}");
        assert_eq!(report.first_error, None);
    }

    #[test]
    fn swapped_stroke_order_is_legible_but_out_of_order() {
        let reference = shi_reference();
        // Same two strokes, drawn in the opposite order.
        let attempt = vec![reference[1].clone(), reference[0].clone()];
        let report = grade(&reference, &attempt, &GradeOptions::default());
        assert!(report.count_ok);
        assert!(!report.order_correct, "order should be wrong: {report:#?}");
        assert!(
            report.legible,
            "shape and placement are fine, so it is still legible: {report:#?}"
        );
        // Both strokes are correctly paired, just sequenced wrongly.
        assert_eq!(report.assignment, vec![Some(1), Some(0)]);
        // Both halves of the swap are flagged, not just one.
        assert_eq!(report.strokes[0].verdict, Verdict::OutOfOrder);
        assert_eq!(report.strokes[1].verdict, Verdict::OutOfOrder);
        // A two-stroke reversal is the worst ordering two strokes can have.
        assert_eq!(report.order_score, 0.0, "{report:#?}");
    }

    #[test]
    fn horizontal_stroke_where_a_vertical_belongs_is_the_wrong_shape() {
        let reference = shi_reference();
        // A horizontal line drawn low, where the vertical stroke should be.
        let attempt = vec![
            reference[0].clone(),
            line((512.0, 850.0), (700.0, 850.0), 5),
        ];
        let report = grade(&reference, &attempt, &GradeOptions::default());
        assert!(
            report.strokes.iter().any(|s| s.verdict == Verdict::ShapeOff),
            "expected a shape fault: {report:#?}"
        );
        assert!(!report.legible);
    }

    #[test]
    fn reversed_stroke_is_reported_as_a_direction_fault() {
        let reference = shi_reference();
        let mut backward = reference[0].clone();
        backward.reverse();
        let attempt = vec![backward, reference[1].clone()];
        let report = grade(&reference, &attempt, &GradeOptions::default());
        assert_eq!(
            report.strokes[0].verdict,
            Verdict::WrongDirection,
            "{report:#?}"
        );
        assert!(!report.is_perfect());
        // Direction is not a legibility problem.
        assert!(report.legible, "{report:#?}");
    }

    #[test]
    fn missing_strokes_are_flagged() {
        let reference = shi_reference();
        let attempt = vec![reference[0].clone()];
        let report = grade(&reference, &attempt, &GradeOptions::default());
        assert!(!report.count_ok);
        assert_eq!(report.strokes[1].verdict, Verdict::Missing);
        assert!(!report.legible);
        // Half the character is missing, so roughly half marks — the one stroke
        // that *was* written must not carry the whole order score.
        assert!(
            (report.overall - 50.0).abs() < 2.0,
            "expected about 50, got {}",
            report.overall
        );
        assert!((report.order_score - 0.5).abs() < 1e-6, "{report:#?}");
    }

    #[test]
    fn extra_strokes_are_flagged_but_real_ones_still_match() {
        let reference = shi_reference();
        let attempt = vec![
            reference[0].clone(),
            reference[1].clone(),
            line((100.0, 950.0), (300.0, 950.0), 5),
        ];
        let report = grade(&reference, &attempt, &GradeOptions::default());
        assert!(!report.count_ok);
        assert_eq!(report.strokes.len(), 2);
        // The two real strokes are still matched correctly.
        assert!(report.strokes.iter().all(|s| s.user_index.is_some()));
        assert_eq!(report.assignment, vec![Some(0), Some(1), None]);
    }

    #[test]
    fn stray_taps_are_ignored() {
        let reference = shi_reference();
        let attempt = vec![
            reference[0].clone(),
            reference[1].clone(),
            vec![Point::new(50.0, 50.0), Point::new(51.0, 51.0)],
        ];
        let report = grade(&reference, &attempt, &GradeOptions::default());
        assert_eq!(report.stray_strokes, 1);
        assert_eq!(report.given_strokes, 2);
        assert!(report.count_ok, "a stray tap should not count as a stroke");
        assert!(report.is_perfect(), "{report:#?}");
    }

    #[test]
    fn nothing_written_scores_zero() {
        let reference = shi_reference();
        let report = grade(&reference, &[], &GradeOptions::default());
        assert_eq!(report.overall, 0.0);
        assert_eq!(report.order_score, 0.0, "no strokes means no order credit");
        assert!(!report.legible);
        assert!(!report.order_correct);
        assert!(report.strokes.iter().all(|s| s.verdict == Verdict::Missing));
    }

    #[test]
    fn global_fit_forgives_a_uniform_shift_and_scale() {
        let reference = shi_reference();
        // Same character, drawn smaller and off to one side.
        let attempt: Vec<Vec<Point>> = reference
            .iter()
            .map(|s| {
                s.iter()
                    .map(|p| Point::new(p.x * 0.6 + 30.0, p.y * 0.6 + 60.0))
                    .collect()
            })
            .collect();

        let forgiving = grade(&reference, &attempt, &GradeOptions::default());
        assert!(
            forgiving.legible,
            "a uniformly smaller character should still be legible: {forgiving:#?}"
        );

        let strict = grade(&reference, &attempt, &GradeOptions::tracing());
        assert!(
            !strict.legible,
            "tracing mode should penalise drawing away from the guide: {strict:#?}"
        );
        assert!(strict.position_score < forgiving.position_score);
    }

    #[test]
    fn order_score_is_partial_for_one_swap_in_three() {
        let reference = vec![
            line((100.0, 100.0), (900.0, 100.0), 4),
            line((100.0, 500.0), (900.0, 500.0), 4),
            line((100.0, 900.0), (900.0, 900.0), 4),
        ];
        // Write 1st, 3rd, 2nd.
        let attempt = vec![
            reference[0].clone(),
            reference[2].clone(),
            reference[1].clone(),
        ];
        let report = grade(&reference, &attempt, &GradeOptions::default());
        assert!(report.count_ok);
        assert_eq!(report.assignment, vec![Some(0), Some(2), Some(1)]);
        assert!(!report.order_correct);
        // One inverted pair out of three possible pairs.
        assert!(
            (report.order_score - 2.0 / 3.0).abs() < 1e-6,
            "order score {}",
            report.order_score
        );
        // The first stroke is untouched; the two that were swapped are flagged.
        assert_eq!(report.strokes[0].verdict, Verdict::Correct);
        assert_eq!(report.strokes[1].verdict, Verdict::OutOfOrder);
        assert_eq!(report.strokes[2].verdict, Verdict::OutOfOrder);
    }
}
