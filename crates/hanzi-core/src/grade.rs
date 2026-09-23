//! Grading a handwritten attempt against a reference character.
//!
//! # How grading works
//!
//! The reference data gives one centre-line ("median") per stroke, in the
//! correct stroke order, plus the outline of the stroke's ink. An attempt is a
//! list of polylines recorded from the canvas. Grading separates three questions
//! that are easy to confuse:
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
//! 3. **Did you put down the right amount of ink?** — centre-lines say where a
//!    stroke went and nothing about how much paper it covered, so a trace that
//!    follows the right path but is drawn far too thin, or that overshoots
//!    wildly, is invisible to the two measures above. Each matched pair is also
//!    compared as raster ink, against the stroke outline the interface draws as
//!    the guide; see [`crate::raster`] for why that number is normalised.
//!
//! All four headline scores — shape, placement, order and ink — are measured
//! across the whole reference character, so a missing stroke is penalised
//! everywhere it should be and a blank canvas scores zero rather than collecting
//! easy marks for a flawless ordering of nothing.
//!
//! Separating them means a character written beautifully but in the wrong
//! order still reports as legible, with the order faults called out
//! individually — which is exactly the feedback a learner needs.

use serde::{Deserialize, Serialize};

use crate::geom::{self, Point};
use crate::raster::{Ink, INK_WIDTH};

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
///
/// Public so that anything measuring the grader against real attempts — the
/// attempt log's analysis — reads the bar the grader actually applied rather
/// than a copy of it that can drift.
pub const SHAPE_OK: f32 = 0.60;
pub const POSITION_OK: f32 = 0.60;
/// Per-stroke ink agreement (see [`crate::raster`]) below which a stroke is
/// reported as too faint, and at which the character stops being legible.
pub const INK_OK: f32 = 0.60;

/// Weights of the four headline measures in the 0..=100 score.
///
/// Equal, and exact binary fractions, so a flawless attempt sums to exactly
/// `1.0` and scores exactly `100` rather than `99.999…` — which the interface's
/// contract test pins. Shape, placement, order and ink answer four independent
/// questions ("is the stroke the right shape", "is it in the right place", "was
/// it drawn in the right sequence", "is there ink on the paper"), and M4 exists
/// because the fourth was previously worth nothing at all. A character written
/// with a third of the ink it needs is not an "excellent" attempt, and with a
/// token weight of `1/8` it still scored 92; an equal share puts it at 85 and
/// in the "good" band, alongside the "not yet legible" badge the threshold
/// measures raise. The selfcheck tolerance table was re-measured when this
/// changed; see `ROADMAP.md` M4 and `HANDOVER.md`.
///
/// Public for the same reason as the bars above: the attempt log's analysis
/// recomputes the headline score from the recorded measures, and that check is
/// only worth anything if it uses these weights rather than its own.
pub const HEADLINE_SHAPE: f32 = 0.25;
pub const HEADLINE_POSITION: f32 = 0.25;
pub const HEADLINE_ORDER: f32 = 0.25;
pub const HEADLINE_INK: f32 = 0.25;

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
    /// Width, in design units out of 1024, of the ink the attempt was **actually
    /// drawn with**. The interface passes the width it painted the stroke with,
    /// so the ink measure compares like with like; [`INK_WIDTH`] is the canvas
    /// default and the width a correct trace is measured against. A device that
    /// reports real pen width (a stylus, or a velocity-thickened brush) sets this
    /// per attempt and a stroke put down with too little ink is then caught.
    ///
    /// Defaulted on deserialisation: an older frontend that does not send the
    /// field gets the canvas default rather than a hard failure.
    #[serde(default = "default_ink_width")]
    pub ink_width: f32,
}

fn default_ink_width() -> f32 {
    INK_WIDTH
}

impl Default for GradeOptions {
    fn default() -> Self {
        Self {
            resample_k: 16,
            min_stroke_len: 12.0,
            global_fit: true,
            ink_width: INK_WIDTH,
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
    /// The right stroke in the right place, but not enough ink: the path is
    /// there and the ink is not. Only the raster measure can see this — the
    /// shape score is scale-invariant and the placement score does not look at
    /// width at all.
    Faint,
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

impl Grade {
    /// The band a 0..=100 headline score falls in.
    ///
    /// This is the one place the thresholds live, so the feedback panel and the
    /// review scheduler ([`crate::progress::Rating`]) can never disagree about
    /// what counts as a good attempt.
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s >= 92.0 => Grade::Excellent,
            s if s >= 75.0 => Grade::Good,
            s if s >= 55.0 => Grade::Fair,
            _ => Grade::Poor,
        }
    }
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
    /// How much of this stroke's ink the attempt put down, `0..=1`, with `1.0`
    /// being what a correct trace reaches at the attempt's pen width. `0.0` when
    /// the stroke was never written. This is the only measure that can see a
    /// stroke drawn too thin.
    pub ink: f32,
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
    /// How much of the character's ink was put down, `0..=1`, averaged over the
    /// reference strokes with anything unwritten counting zero. `1.0` means
    /// every stroke reached as much of the outline as a correct trace at the
    /// nominal pen width can. See [`crate::raster`].
    pub ink_score: f32,
    /// How much of the ink a correct trace would touch that the attempt touched,
    /// `0..=1`. This is the "you never drew that part" signal: overshooting ink
    /// costs [`GradeReport::ink_score`] but leaves coverage alone.
    pub ink_coverage: f32,
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

/// Bound a score to `0..=1`, and send anything that is not a number to `0.0`.
///
/// `f32::clamp` does **not** do this: it returns `NaN` for a `NaN` input. So a
/// measure that reached infinity or `NaN` — a coordinate large enough that
/// squaring its delta overflowed did it — used to pass straight through the
/// clamp that looks like a guarantee. Every headline score is documented as a
/// fraction, and a caller cannot tell a `NaN` fraction from a real one, so the
/// bound is enforced here rather than assumed.
///
/// Non-finite goes to `0.0` rather than to the nearest end of the range: a score
/// that could not be measured is the worst score, which is the same answer
/// [`position_score`] already gives a stroke with no extent at all.
fn bounded(score: f32) -> f32 {
    if score.is_finite() {
        score.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub fn shape_score(distance: f32) -> f32 {
    bounded(1.0 - distance / SHAPE_TOL)
}

/// How close the stroke sits to the reference, in absolute box coordinates.
///
/// Combines centroid offset with bounding-box size mismatch, so both a stroke
/// in the wrong place and one that is grossly the wrong length are caught.
///
/// The offset is measured between *length* centroids, not sample means. The
/// bounding box needs no such care — its extremes are extremes however densely
/// the stroke was sampled — but the sample mean moves with drawing speed, and
/// grading how fast someone moved the pointer as if it were position is exactly
/// the kind of verdict a learner cannot act on.
fn position_score(points: &[Point], reference: &[Point]) -> f32 {
    let Some((min_u, max_u)) = geom::bbox(points) else {
        return 0.0;
    };
    let Some((min_r, max_r)) = geom::bbox(reference) else {
        return 0.0;
    };
    let centroid_offset =
        geom::length_centroid(points).distance_to(geom::length_centroid(reference));
    let diag = |a: Point, b: Point| a.distance_to(b);
    let du = diag(min_u, max_u);
    let dr = diag(min_r, max_r);
    let size_mismatch = if du.max(dr) > 1e-3 {
        (du - dr).abs() / du.max(dr)
    } else {
        0.0
    };
    let penalty = 0.65 * (centroid_offset / POSITION_TOL) + 0.35 * (size_mismatch / SIZE_TOL);
    // `size_mismatch` is `(du - dr) / max(du, dr)`, which is `∞ / ∞` = `NaN` once
    // a coordinate is large enough for its delta to square to infinity. `bounded`
    // is what turns that into a score instead of a `NaN` that travels.
    bounded(1.0 - penalty)
}

/// Grade `attempt` against a reference character's stroke medians.
///
/// Both the reference medians and the attempt are expected in display space
/// (origin top-left, y growing downwards, box `0..=1024`); see [`crate::geom`].
///
/// This is the outline-free entry point: the ink measure falls back to comparing
/// the attempt against the reference *medians* stroked at the attempt's pen
/// width, which is the most a centre-line-only reference can say. Pass the
/// character's outline paths to [`grade_with_outlines`] to measure ink against
/// the real glyph.
pub fn grade(
    reference_medians: &[Vec<Point>],
    attempt: &[Vec<Point>],
    options: &GradeOptions,
) -> GradeReport {
    grade_inner(reference_medians, None, attempt, options)
}

/// Grade `attempt` against a reference character's medians *and* its ink.
///
/// `reference_outlines` are the stored SVG outline paths, in font space, one per
/// stroke in stroke order — the same paths the interface fills as the faint
/// guide. They are what lets the ink measure see a stroke that was drawn far too
/// thin or that overshoots the character; with no outline, that part of the
/// report degrades to the medians-only comparison instead of failing.
pub fn grade_with_outlines(
    reference_medians: &[Vec<Point>],
    reference_outlines: &[String],
    attempt: &[Vec<Point>],
    options: &GradeOptions,
) -> GradeReport {
    grade_inner(reference_medians, Some(reference_outlines), attempt, options)
}

fn grade_inner(
    reference_medians: &[Vec<Point>],
    reference_outlines: Option<&[String]>,
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
        // A stroke with no finite point is not a stroke. `NaN` and `±∞` cannot
        // come from a pointer, and one of them in the geometry turns every
        // comparison it touches into nonsense — the cost matrix, the fitted
        // transform and the headline scores alike. Such a mark is counted as a
        // stray, the same as a tap too short to be a stroke, so the report still
        // says it was seen rather than dropping it in silence. The order matters
        // as well as the answer: a non-finite stroke is refused before its length
        // is measured, because that length would be `NaN` too.
        let finite = stroke.iter().all(|p| p.x.is_finite() && p.y.is_finite());
        if !finite || geom::path_length(stroke) < stray_below {
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

    // 6. Ink, on the final pairing and after any fitting, so a character drawn
    // correctly but smaller than the box is judged on where it ended up.
    let ink = ink_scores(
        reference_medians,
        reference_outlines,
        &kept,
        &user_of_ref,
        options.ink_width,
    );

    // 7. Assemble per-stroke feedback.
    let mut strokes = Vec::with_capacity(n_ref);
    for j in 0..n_ref {
        let Some(i) = user_of_ref[j] else {
            strokes.push(StrokeVerdict {
                ref_index: j,
                user_index: None,
                verdict: Verdict::Missing,
                shape: 0.0,
                position: 0.0,
                ink: 0.0,
                score: 0.0,
            });
            continue;
        };
        let m = metrics[i][j].expect("matched pairs were scored");
        // Faint is judged last of all: a stroke that is also the wrong shape, in
        // the wrong place, backwards or out of order has a more useful thing to
        // say about it than "not enough ink", and only the ink measure can see
        // the case where everything else is right.
        let verdict = if m.shape < SHAPE_OK {
            Verdict::ShapeOff
        } else if m.position < POSITION_OK {
            Verdict::PositionOff
        } else if m.reversed {
            Verdict::WrongDirection
        } else if out_of_order[j] {
            Verdict::OutOfOrder
        } else if ink.per_stroke[j] < INK_OK {
            Verdict::Faint
        } else {
            Verdict::Correct
        };
        strokes.push(StrokeVerdict {
            ref_index: j,
            user_index: Some(kept[i].original_index),
            verdict,
            shape: m.shape,
            position: m.position,
            ink: ink.per_stroke[j],
            score: 0.5 * m.shape + 0.5 * m.position,
        });
    }

    // 8. Aggregate. Every score is measured across the *whole* reference
    // character, with unwritten strokes counting as zero. An incomplete attempt
    // is therefore penalised consistently in shape, placement, ink and order,
    // and a blank canvas scores nothing at all. Measuring order only over the
    // strokes that were written would let "I wrote one stroke correctly" or even
    // "I wrote nothing" collect full marks for stroke order.
    let denom = n_ref.max(1) as f32;
    let matched = strokes.iter().filter(|s| s.user_index.is_some()).count();
    let coverage = if n_ref == 0 {
        1.0
    } else {
        matched as f32 / n_ref as f32
    };
    let shape_score = strokes.iter().map(|s| s.shape).sum::<f32>() / denom;
    let position_score = strokes.iter().map(|s| s.position).sum::<f32>() / denom;
    let ink_score = ink.score;
    let order_score = coverage * ordered_score;
    let overall = (100.0
        * (HEADLINE_SHAPE * shape_score
            + HEADLINE_POSITION * position_score
            + HEADLINE_ORDER * order_score
            + HEADLINE_INK * ink_score))
        .clamp(0.0, 100.0);
    let grade = Grade::from_score(overall);

    GradeReport {
        expected_strokes: expected,
        given_strokes: n_user,
        stray_strokes: stray,
        count_ok: n_user == n_ref,
        shape_score,
        position_score,
        ink_score,
        ink_coverage: ink.coverage,
        order_score,
        overall,
        // Ink is part of legibility, not a separate badge: a character written
        // with far too little ink — or scribbled over with far too much — is not
        // legible however well its path was traced. A correct trace scores 1.0,
        // so this cannot fail an attempt the centre-line measures already
        // accepted. Coverage is deliberately *not* a gate: the parts of the
        // glyph a wobbling but correctly-inked stroke misses are a placement
        // fault, already measured above, and gating on them too cost 5% of the
        // "sloppy" tolerance row for nothing.
        legible: n_ref > 0
            && n_user == n_ref
            && shape_score >= SHAPE_OK
            && position_score >= POSITION_OK
            && ink_score >= INK_OK,
        order_correct: n_user == n_ref && order_score >= 0.999,
        grade,
        first_error: strokes.iter().find(|s| s.verdict.is_error()).map(|s| s.ref_index),
        strokes,
        assignment,
        fit: fit_info,
    }
}

/// How the attempt's ink compares with the character's own ink.
struct InkScores {
    /// Mean ink-amount agreement over the reference strokes, `0..=1`, unwritten
    /// strokes counting zero. `1.0` is what a correct trace scores.
    score: f32,
    /// Fraction of the ink a correct trace touches that the attempt touched.
    coverage: f32,
    /// Agreement for each reference stroke, in reference order.
    per_stroke: Vec<f32>,
}

/// Measure the attempt's ink against the reference character's ink.
///
/// Two questions, both asked per reference stroke and averaged over all of them:
///
/// * **How much ink?** The attempt's ink area against the area a correct trace
///   at the nominal pen width ([`INK_WIDTH`]) would put down. This is the
///   measure a centre-line cannot make: a pen a third of the width reads about
///   `0.33`, and a wild overshoot reads the same from the other side. It is
///   deliberately blind to *where* the ink went, because placement is graded
///   separately and counting it twice is what makes a wobbly hand illegible.
/// * **Coverage?** How much of the glyph's own ink the attempt reached, from the
///   stored outlines — the "you never drew that part" signal.
///
/// The baseline width is the wider of the nominal pen and the attempt's reported
/// width, so a device that honestly draws fatter than the canvas is not punished
/// for it; only too *little* ink is a fault.
fn ink_scores(
    reference_medians: &[Vec<Point>],
    reference_outlines: Option<&[String]>,
    kept: &[Kept],
    user_of_ref: &[Option<usize>],
    width: f32,
) -> InkScores {
    let n_ref = reference_medians.len();
    let ideal_width = INK_WIDTH.max(width);
    let mut per_stroke = vec![0.0f32; n_ref];
    let mut reference_all = Ink::empty();
    let mut ideal_all = Ink::empty();
    let mut attempt_all = Ink::empty();

    for j in 0..n_ref {
        // What a correct trace of this stroke would put down.
        let ideal = Ink::from_strokes(std::slice::from_ref(&reference_medians[j]), ideal_width);
        let outline = reference_outlines
            .and_then(|outlines| outlines.get(j))
            .map(|path| Ink::from_outlines(&[path.as_str()]))
            .filter(|ink| !ink.is_empty());
        // With no usable outline the stroke's own band *is* the glyph ink, which
        // is what a centre-line-only reference can honestly say.
        let reference = outline.unwrap_or_else(|| ideal.clone());

        let attempt = match user_of_ref[j] {
            Some(i) => Ink::from_strokes(std::slice::from_ref(&kept[i].points), width),
            None => Ink::empty(),
        };
        per_stroke[j] = if attempt.is_empty() {
            0.0
        } else {
            attempt.amount_agreement(&ideal)
        };

        reference_all.or_with(&reference);
        ideal_all.or_with(&ideal);
        attempt_all.or_with(&attempt);
    }

    let coverable = ideal_all.intersection_count(&reference_all);
    let covered = attempt_all.intersection_count(&reference_all);
    let coverage = if coverable == 0 {
        1.0
    } else {
        (covered as f32 / coverable as f32).min(1.0)
    };

    InkScores {
        score: per_stroke.iter().sum::<f32>() / n_ref.max(1) as f32,
        coverage,
        per_stroke,
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

/// Points each matched stroke is resampled to before the global fit is derived
/// from it.
///
/// The fit is a placement transform, so it must not depend on how fast the
/// learner moved the pointer: resampling each stroke evenly along its length
/// gives every matched stroke the same say, whatever the sample density it
/// arrived with.
const FIT_RESAMPLE_K: usize = 16;

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
        // Even sampling on both sides, so neither the centre nor the scale of
        // the fit is biased by pointer speed.
        user_points.extend(geom::resample(&k.points, FIT_RESAMPLE_K));
        ref_points.extend(geom::resample(&reference[j], FIT_RESAMPLE_K));
    }
    if user_points.is_empty() || ref_points.is_empty() {
        return None;
    }
    let fit = geom::fit_similarity(&user_points, &ref_points);
    let offset = geom::length_centroid(&user_points).distance_to(geom::length_centroid(&ref_points));

    // A fit is a placement transform, so it only means something when the scale
    // and the offset it describes are real numbers. Coordinates large enough to
    // overflow `f32` squares can produce an infinite radius — and `∞ / ∞` for the
    // scale — and applying a `NaN` transform would spread the nonsense to every
    // stroke instead of leaving them as they were drawn. `None` already means
    // "no fit", so an unfittable attempt simply gets none.
    if !fit.scale.is_finite()
        || !offset.is_finite()
        || !fit.from_centroid.x.is_finite()
        || !fit.from_centroid.y.is_finite()
        || !fit.to_centroid.x.is_finite()
        || !fit.to_centroid.y.is_finite()
    {
        return None;
    }
    Some((fit, offset))
}

/// Solve the square assignment problem, minimising total cost.
///
/// This is the O(n^3) Hungarian algorithm (Kuhn–Munkres) in the compact
/// potential-updating form, with row 0 / column 0 used as sentinels.
/// `cost` must be square and non-negative. Returns `assign[row] = col`, with
/// `usize::MAX` for a row no finite cost could place — which the caller reads as
/// "not matched". A non-finite cost is not solvable and is refused rather than
/// followed, because following it never terminates.
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
        // The potentials before this row's search, so an unsolvable row can put
        // them back. See the bail-out below.
        let (u_before, v_before) = (u.clone(), v.clone());
        let mut solvable = true;
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
            // `delta` is `+∞` when no unused column offered a finite cost, which
            // is what a row of `NaN` or `+∞` costs looks like from here: every
            // comparison against either is false, so nothing improves. There is
            // no augmenting path to follow, and following it anyway would update
            // the potentials with `∞` and come back to the sentinel column for
            // ever. The row is abandoned instead — the caller reads `usize::MAX`
            // as "not matched" — which is the only outcome that is not a hang.
            //
            // The callers hand this a finite matrix, because every metric is
            // bounded, so this is the documented precondition enforced rather
            // than a case the grader produces. It is enforced here because the
            // failure it prevents is an infinite loop in a synchronous command.
            if !delta.is_finite() {
                solvable = false;
                break;
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
        if !solvable {
            // Leave the row unmatched, and leave nothing else changed. Two
            // things have to be undone rather than merely not done: the search
            // updated the potentials on its way to the column it could not
            // extend, and a half-updated potential is not a valid state for the
            // rows still to come; and `way` holds a path that never reached the
            // sentinel, so walking it would rewrite `p` into a matching that is
            // not one. Restoring the potentials makes the remaining rows behave
            // exactly as if this row were not in the matrix.
            u = u_before;
            v = v_before;
            continue;
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

    /// The same straight segment with its samples bunched towards one end.
    ///
    /// `power` > 1 clusters them at the start (a slow beginning, a flicked
    /// end); `power` < 1 does the opposite. The geometry is untouched: same
    /// line, same endpoints, same ink.
    fn uneven_line(a: (f32, f32), b: (f32, f32), n: usize, power: f32) -> Vec<Point> {
        (0..n)
            .map(|i| {
                let t = (i as f32 / (n - 1) as f32).powf(power);
                Point::new(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
            })
            .collect()
    }

    #[test]
    fn placement_ignores_how_fast_the_pointer_moved() {
        // Pointer samples arrive by time, not by distance, so the same stroke
        // drawn slowly at one end and flicked at the other arrives with its
        // samples bunched. Nothing a learner can act on has changed, so no
        // verdict may change: grading speed as if it were position is how a
        // perfect trace gets told it is "in the wrong place".
        let reference = shi_reference();
        let uneven = vec![
            uneven_line((120.0, 400.0), (900.0, 400.0), 240, 3.0),
            uneven_line((512.0, 120.0), (512.0, 900.0), 240, 0.33),
        ];

        for options in [
            // Tracing: no guide-fitting, so placement is judged absolutely.
            GradeOptions {
                global_fit: false,
                ..GradeOptions::default()
            },
            GradeOptions::default(),
        ] {
            let even_report = grade(&reference, &reference, &options);
            let uneven_report = grade(&reference, &uneven, &options);

            assert!(uneven_report.count_ok, "sampling must not change the count");
            assert!(
                uneven_report.strokes.iter().all(|s| s.verdict == Verdict::Correct),
                "the same strokes, only sampled differently, were judged: {:#?}",
                uneven_report.strokes
            );
            for (a, b) in even_report.strokes.iter().zip(&uneven_report.strokes) {
                assert!(
                    (a.position - b.position).abs() < 0.01,
                    "stroke {} placed at {:.2} even but {:.2} uneven (global_fit={})",
                    a.ref_index + 1,
                    a.position,
                    b.position,
                    options.global_fit
                );
            }
        }
    }

    #[test]
    fn the_global_fit_is_not_biased_by_pointer_speed() {
        // The attempt is the reference shrunk to 80%, so the fit has to recover
        // a scale of 1.25. Bunching the samples must not move it: the fit is a
        // placement transform, and pointer speed is not placement.
        let reference = shi_reference();
        let shrink = |strokes: &[Vec<Point>]| -> Vec<Vec<Point>> {
            strokes
                .iter()
                .map(|s| {
                    s.iter()
                        .map(|p| Point::new(p.x * 0.8, p.y * 0.8))
                        .collect()
                })
                .collect()
        };

        let evenly = shrink(&reference);
        let unevenly = vec![
            uneven_line((96.0, 320.0), (720.0, 320.0), 240, 3.0),
            uneven_line((409.6, 96.0), (409.6, 720.0), 240, 0.33),
        ];

        for attempt in [&evenly, &unevenly] {
            let report = grade(&reference, attempt, &GradeOptions::default());
            let fit = report.fit.expect("a full attempt should produce a fit");
            assert!(
                (fit.scale - 1.25).abs() < 0.02,
                "fit scale {:.3} for an attempt drawn at 80%",
                fit.scale
            );
            assert!(report.legible, "{report:#?}");
        }
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

    /// Coordinates no pointer can produce must be graded, not spun on.
    ///
    /// `Point::distance_to` squares its deltas in `f32`, so a coordinate near
    /// `1e19` overflows to infinity. The size mismatch in `position_score` then
    /// became `inf / inf` = NaN, `.clamp(0.0, 1.0)` passed the NaN straight
    /// through — `f32::clamp` returns NaN for a NaN input — the cost matrix
    /// carried it, and `hungarian` never returned: every comparison against NaN
    /// is false, so no column ever improved `delta` and `j1` stayed 0 for ever.
    /// `grade_attempt` is a synchronous command, so that was a pegged core and a
    /// board that never came back.
    ///
    /// The contract asserted here is the one that matters: the call finishes, and
    /// everything it reports is a real number. Which score an absurd stroke gets
    /// is not the point — that it is a score at all is.
    #[test]
    fn a_stroke_with_absurd_coordinates_is_graded_rather_than_hanging() {
        let reference = shi_reference();
        let options = GradeOptions::default();

        for v in [
            1e19f32,
            1e20,
            1e30,
            f32::MAX,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
        ] {
            let attempt = vec![vec![Point::new(v, 0.0), Point::new(0.0, 0.0)]];
            let report = grade(&reference, &attempt, &options);

            assert!(
                report.overall.is_finite(),
                "v = {v} scored {}",
                report.overall
            );
            assert!((0.0..=100.0).contains(&report.overall));
            for value in [
                report.shape_score,
                report.position_score,
                report.ink_score,
                report.ink_coverage,
                report.order_score,
            ] {
                assert!(
                    value.is_finite() && (0.0..=1.0).contains(&value),
                    "v = {v} reported {value}"
                );
            }
            for stroke in &report.strokes {
                assert!(
                    stroke.score.is_finite(),
                    "v = {v} reported a stroke score of {}",
                    stroke.score
                );
            }
        }
    }

    /// A stroke with no finite point is not a stroke.
    ///
    /// It is counted as stray, the way a tap too short to be a stroke is: the
    /// report still says the mark was seen, and nothing downstream has to cope
    /// with a position that is not a position.
    #[test]
    fn a_stroke_with_no_finite_point_is_a_stray_mark() {
        let reference = shi_reference();
        let attempt = vec![
            reference[0].clone(),
            vec![Point::new(f32::NAN, 10.0), Point::new(20.0, 20.0)],
            vec![Point::new(f32::INFINITY, 0.0), Point::new(0.0, 0.0)],
        ];
        let report = grade(&reference, &attempt, &GradeOptions::default());

        assert_eq!(report.stray_strokes, 2, "both non-finite marks are strays");
        assert_eq!(report.given_strokes, 1, "only the real stroke was kept");
        assert!(report.overall.is_finite());
    }

    /// The solver's documented precondition, enforced rather than assumed.
    ///
    /// `hungarian` says its matrix must be square and non-negative and relies on
    /// that to terminate. A matrix it cannot solve must come back unmatched, not
    /// spin: this is the guard that makes the difference between a wrong score
    /// and a wedged process. Both spellings of "no finite cost" are covered —
    /// NaN, where every comparison is false, and `+∞`, where `∞ < ∞` is.
    #[test]
    fn hungarian_survives_a_matrix_it_cannot_solve() {
        let unsolvable = |value: f64| vec![vec![value, value], vec![value, value]];
        assert_eq!(
            hungarian(&unsolvable(f64::NAN)),
            vec![usize::MAX, usize::MAX]
        );
        assert_eq!(
            hungarian(&unsolvable(f64::INFINITY)),
            vec![usize::MAX, usize::MAX]
        );

        // A row that cannot be placed must not take a solvable row down with it.
        let mixed = vec![
            vec![f64::INFINITY, f64::INFINITY],
            vec![1.0, 2.0],
        ];
        assert_eq!(
            hungarian(&mixed),
            vec![usize::MAX, 0],
            "the row with no finite cost is left unmatched; the other still pairs"
        );

        // The harder shape: the search reaches a finite column, then finds no
        // finite way out of it. Potentials were updated on the way in, so a
        // guard that only stopped the loop would leave them half-updated and
        // corrupt the rows still to come. The assertion is the property rather
        // than an exact answer — whatever it decides must be a *matching*.
        let dead_end = vec![
            vec![1.0, 1.0, f64::INFINITY],
            vec![1.0, f64::INFINITY, f64::INFINITY],
            vec![f64::INFINITY, 3.0, 1.0],
        ];
        let solved = hungarian(&dead_end);
        assert_eq!(solved.len(), 3);
        let mut taken: Vec<usize> = solved.iter().copied().filter(|c| *c != usize::MAX).collect();
        let assigned = taken.len();
        taken.sort_unstable();
        taken.dedup();
        assert_eq!(taken.len(), assigned, "a column was assigned to two rows");
        assert!(taken.iter().all(|c| *c < 3), "a column out of range");
        for (row, col) in solved.iter().enumerate() {
            if *col != usize::MAX {
                assert!(
                    dead_end[row][*col].is_finite(),
                    "row {row} was matched to a column it has no finite cost for"
                );
            }
        }
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

    // ---- raster ink --------------------------------------------------------

    /// A filled rectangle in display space, as SVG path data in font space.
    ///
    /// The stored outlines are font space (y up); the grader and the canvas are
    /// display space (y down), which is the flip `Point::from_font` applies.
    fn ink_rect(x0: f32, y0: f32, x1: f32, y1: f32) -> String {
        let (top, bottom) = (900.0 - y0, 900.0 - y1);
        format!("M {x0} {top} L {x1} {top} L {x1} {bottom} L {x0} {bottom} Z")
    }

    /// 十 with real ink: two 46-unit bars drawn around the stroke medians.
    fn shi_with_ink() -> (Vec<Vec<Point>>, Vec<String>) {
        let outlines = vec![
            ink_rect(120.0, 377.0, 900.0, 423.0), // 横
            ink_rect(489.0, 120.0, 535.0, 900.0), // 丨
        ];
        (shi_reference(), outlines)
    }

    /// Grade with the pen width a third of the canvas's.
    fn third_width_options() -> GradeOptions {
        GradeOptions {
            ink_width: GradeOptions::default().ink_width / 3.0,
            ..GradeOptions::default()
        }
    }

    #[test]
    fn a_perfect_trace_puts_down_exactly_the_ink_it_should() {
        let (medians, outlines) = shi_with_ink();
        // At the nominal pen width, whatever mode grading is in, a correct trace
        // reaches everything a correct trace can reach: the score is normalised
        // against exactly that. This is what keeps "perfect" reachable for every
        // character and every font weight.
        for options in [GradeOptions::default(), GradeOptions::tracing()] {
            let report = grade_with_outlines(&medians, &outlines, &medians, &options);
            assert_eq!(report.ink_score, 1.0, "pen width {}", options.ink_width);
            assert_eq!(report.ink_coverage, 1.0);
            assert!(report.strokes.iter().all(|s| s.verdict == Verdict::Correct));
            assert!(report.strokes.iter().all(|s| s.ink == 1.0));
            assert!(report.is_perfect(), "{report:#?}");
        }

        // A pen *thinner* than the canvas draws with is a real difference, so
        // the same correct path no longer reaches the ink the guide shows.
        let thin = grade_with_outlines(&medians, &outlines, &medians, &third_width_options());
        assert!(thin.ink_score < INK_OK);
    }

    #[test]
    fn a_perfect_attempt_scores_exactly_one_hundred() {
        // Not "about 100": the four weights are exact binary fractions and a
        // correct trace scores 1.0 on all four, so the sum is exactly 1.0. The
        // interface's contract test asserts this too, and a rounding change here
        // would silently make "100/100" read 99.
        let reference = shi_reference();
        let report = grade(&reference, &reference, &GradeOptions::default());
        assert_eq!(report.overall, 100.0);
        assert_eq!(report.grade, Grade::Excellent);
    }

    #[test]
    fn a_third_of_the_ink_falls_below_the_legibility_bar() {
        let (medians, outlines) = shi_with_ink();
        let full = grade_with_outlines(&medians, &outlines, &medians, &GradeOptions::default());
        let thin = grade_with_outlines(&medians, &outlines, &medians, &third_width_options());

        // The same stroke at the correct width passes.
        assert!(full.legible);
        assert!(full.strokes.iter().all(|s| s.ink >= INK_OK));

        // A third of the width puts every stroke below the bar, and the
        // character is no longer legible.
        for stroke in &thin.strokes {
            assert!(
                stroke.ink < INK_OK,
                "stroke {} scored {:.2} at a third width",
                stroke.ref_index + 1,
                stroke.ink
            );
            assert_eq!(stroke.verdict, Verdict::Faint);
        }
        assert!(thin.ink_score < INK_OK, "ink score {}", thin.ink_score);
        assert!(!thin.legible, "{thin:#?}");
        assert!(!thin.is_perfect());

        // Nothing else noticed: the path, the placement and the order were
        // perfect in both attempts, which is exactly why the ink measure exists.
        assert_eq!(thin.shape_score, full.shape_score);
        assert_eq!(thin.position_score, full.position_score);
        assert_eq!(thin.order_score, full.order_score);
        assert!(thin.overall < full.overall);
        // ...and a character with a third of its ink is not an "excellent"
        // attempt, which is why ink carries a full quarter of the score.
        assert!(thin.overall < 92.0, "overall {}", thin.overall);
    }

    #[test]
    fn a_thin_pen_is_caught_without_an_outline_too() {
        // The medians-only entry point has no glyph ink to compare against, but
        // it can still tell a thin pen from the width the canvas draws with.
        let reference = shi_reference();
        let report = grade(&reference, &reference, &third_width_options());
        assert!(report.strokes.iter().all(|s| s.verdict == Verdict::Faint));
        assert!(!report.legible);
        assert!(grade(&reference, &reference, &GradeOptions::default()).legible);
    }

    #[test]
    fn overshooting_adds_ink_outside_the_character_without_covering_less() {
        let (medians, outlines) = shi_with_ink();
        // Tracing mode, so the global fit does not absorb a stroke that sticks
        // out of the character — that fit is deliberate and tested elsewhere.
        let options = GradeOptions::tracing();
        // The horizontal stroke drawn straight through the character and well
        // out the other side; the vertical one untouched.
        let overshoot = vec![line((0.0, 400.0), (1024.0, 400.0), 5), medians[1].clone()];
        let straight = grade_with_outlines(&medians, &outlines, &medians, &options);
        let report = grade_with_outlines(&medians, &outlines, &overshoot, &options);

        assert!(
            report.strokes[0].ink < straight.strokes[0].ink,
            "overshooting ink scored {:.2}, a correct stroke {:.2}",
            report.strokes[0].ink,
            straight.strokes[0].ink
        );
        // The fault is extra ink, not missing ink, so coverage is untouched and
        // the untouched stroke is unaffected.
        assert_eq!(report.ink_coverage, 1.0);
        assert_eq!(report.strokes[1].ink, 1.0);
    }

    #[test]
    fn ink_survives_a_reasonably_placed_attempt() {
        // A hand-wobbly trace must not be failed by the ink measure: the band is
        // the same width wherever the stroke wandered, so the score dips — the
        // ink really did miss the guide by a few units — but stays well clear of
        // the bar.
        let (medians, outlines) = shi_with_ink();
        let wobble: Vec<Vec<Point>> = medians
            .iter()
            .map(|s| {
                s.iter()
                    .enumerate()
                    .map(|(i, p)| Point::new(p.x + (i % 3) as f32 * 6.0 - 6.0, p.y))
                    .collect()
            })
            .collect();
        let report = grade_with_outlines(&medians, &outlines, &wobble, &GradeOptions::default());
        assert!(
            report.ink_score > INK_OK + 0.15,
            "ink {} on a six-unit wobble",
            report.ink_score
        );
        assert!(report.ink_score < 1.0, "a wobble must not read as perfect");
        assert!(report.legible, "{report:#?}");
    }

    #[test]
    fn a_missing_outline_falls_back_instead_of_failing() {
        // An unparseable outline must not make a correct attempt unscoreable.
        let reference = shi_reference();
        let outlines = vec!["not a path".to_string(), "".to_string()];
        let report =
            grade_with_outlines(&reference, &outlines, &reference, &GradeOptions::default());
        assert_eq!(report.ink_score, 1.0);
        assert_eq!(report.ink_coverage, 1.0);
        assert!(report.legible);
        assert_eq!(report.overall, 100.0);
    }

    #[test]
    fn ink_counts_nothing_for_a_stroke_that_was_never_written() {
        let (medians, outlines) = shi_with_ink();
        let report = grade_with_outlines(
            &medians,
            &outlines,
            &medians[..1],
            &GradeOptions::default(),
        );
        assert_eq!(report.strokes[1].verdict, Verdict::Missing);
        assert_eq!(report.strokes[1].ink, 0.0);
        // Half the character's ink is missing, so half the ink score.
        assert!((report.ink_score - 0.5).abs() < 1e-6, "{}", report.ink_score);
        assert!((report.ink_coverage - 0.5).abs() < 0.15, "{}", report.ink_coverage);
    }
}
