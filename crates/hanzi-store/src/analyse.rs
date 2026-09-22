//! What a log of real attempts can say about the grader.
//!
//! The grading tolerances were set against synthetic Gaussian jitter, and the
//! four headline weights are an equal quarter each — chosen for symmetry, not
//! from data. Both are honest guesses, and the only thing that can replace a
//! guess is what real learners actually wrote. That is what the attempt log
//! holds, and this module is what reads it.
//!
//! It deliberately reports rather than concludes. It cannot know whether an
//! attempt was *right* — a learner may write a character correctly and be marked
//! down for a wobble, or write it badly and be marked up for a generous
//! tolerance — so it does not claim an accuracy. What it can do is show where the
//! mass sits: a bar with a quarter of all attempts within ten points of it is a
//! bar that is deciding a great deal, a measure pinned at 1.0 is a measure that
//! is deciding nothing, and a measure whose mean is the same for passes and
//! failures is not earning its quarter of the score.
//!
//! Everything here is a pure function of the rows, so it is testable without a
//! database and a learner's data never has to move to be analysed.

use hanzi_core::{
    AttemptMeasures, HEADLINE_INK, HEADLINE_ORDER, HEADLINE_POSITION, HEADLINE_SHAPE, INK_OK,
    POSITION_OK, SHAPE_OK,
};

use crate::LoggedAttempt;

/// The percentiles reported for each measure: the tails, the middle, and the
/// quartiles around it.
pub const PERCENTILES: [(f64, &str); 5] =
    [(0.05, "p5"), (0.25, "p25"), (0.50, "p50"), (0.75, "p75"), (0.95, "p95")];

/// How close to a bar counts as "the bar is deciding this attempt".
const NEAR_BAR: f32 = 0.10;

/// The mean of each measure over some set of attempts.
///
/// The useful comparison is passes against failures: the weights exist to tell
/// them apart, so a measure whose two means nearly coincide is one the score
/// could drop without changing much.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Means {
    pub shape: f32,
    pub position: f32,
    pub ink: f32,
    pub ink_coverage: f32,
    pub order: f32,
}

impl Means {
    /// The means of `rows`, or `None` when there are none to average.
    fn of(rows: &[AttemptMeasures]) -> Option<Self> {
        if rows.is_empty() {
            return None;
        }
        let n = rows.len() as f32;
        Some(Self {
            shape: rows.iter().map(|m| m.shape).sum::<f32>() / n,
            position: rows.iter().map(|m| m.position).sum::<f32>() / n,
            ink: rows.iter().map(|m| m.ink).sum::<f32>() / n,
            ink_coverage: rows.iter().map(|m| m.ink_coverage).sum::<f32>() / n,
            order: rows.iter().map(|m| m.order).sum::<f32>() / n,
        })
    }
}

/// One measure's distribution across the measured attempts.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasureStats {
    pub n: usize,
    pub mean: f32,
    /// One value per [`PERCENTILES`] entry, in that order.
    pub percentiles: [f32; PERCENTILES.len()],
    /// The bar the grader applies to this measure, when it applies one.
    pub bar: Option<f32>,
    /// Share of attempts at or above the bar.
    pub at_or_above_bar: Option<f32>,
    /// Share within [`NEAR_BAR`] of the bar: the attempts this bar is deciding.
    pub near_bar: Option<f32>,
    /// Share pinned at 1.0 — nothing left to improve — and at 0.0.
    ///
    /// A measure that is almost always pinned is not measuring anything: ink
    /// coverage was expected to be the "you never drew that part" signal, and if
    /// it never leaves 1.0 it cannot be.
    pub pinned_high: f32,
    pub pinned_low: f32,
}

impl MeasureStats {
    /// The distribution of one measure, with the bar the grader applies to it.
    fn of(mut values: Vec<f32>, bar: Option<f32>) -> Self {
        // A measure that was never recorded has no distribution: report it as
        // empty rather than as a mean of nothing, which would be `NaN` printed
        // as a number.
        if values.is_empty() {
            return Self {
                n: 0,
                mean: f32::NAN,
                percentiles: [f32::NAN; PERCENTILES.len()],
                bar,
                at_or_above_bar: None,
                near_bar: None,
                pinned_high: f32::NAN,
                pinned_low: f32::NAN,
            };
        }
        let n = values.len();
        let mean = values.iter().sum::<f32>() / n as f32;
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let share = |count: usize| count as f32 / n as f32;
        Self {
            n,
            mean,
            percentiles: std::array::from_fn(|i| percentile(&values, PERCENTILES[i].0)),
            bar,
            at_or_above_bar: bar.map(|bar| share(values.iter().filter(|v| **v >= bar).count())),
            near_bar: bar.map(|bar| {
                share(
                    values
                        .iter()
                        .filter(|v| (**v - bar).abs() <= NEAR_BAR)
                        .count(),
                )
            }),
            pinned_high: share(values.iter().filter(|v| **v >= 0.999).count()),
            pinned_low: share(values.iter().filter(|v| **v <= 0.001).count()),
        }
    }
}

/// What the log says, as numbers.
#[derive(Clone, Debug, PartialEq)]
pub struct AttemptAnalysis {
    pub attempts: usize,
    /// How many carry measures at all. Everything below is over these.
    pub measured: usize,
    /// Distinct characters attempted.
    pub characters: usize,
    pub shape: MeasureStats,
    pub position: MeasureStats,
    pub ink: MeasureStats,
    /// Reported though it is not scored: it is the "you never drew that part"
    /// signal, and it is the measure most likely to be found pinned at 1.0.
    pub ink_coverage: MeasureStats,
    pub order: MeasureStats,
    /// Share of measured attempts the grader called legible, and correctly
    /// ordered. These are the grader's own verdicts, not ground truth.
    pub legible_rate: f32,
    pub order_correct_rate: f32,
    /// Mean measures of attempts that failed (`again` or `hard`) and of those
    /// that did not.
    pub failure_means: Option<Means>,
    pub pass_means: Option<Means>,
    /// How far the recorded headline score is from the weights applied to the
    /// recorded measures: `(mean, max)` absolute difference in score points.
    ///
    /// Near zero is the check that the stored measures really are the ones that
    /// produced the stored score — if it is not near zero, either the weights
    /// changed after the rows were written or the wrong number was recorded, and
    /// every other figure here is suspect.
    pub score_deviation: Option<(f32, f32)>,
}

/// Read the log.
///
/// `attempts` is expected in the log's own order; nothing here depends on it.
pub fn analyse(attempts: &[LoggedAttempt]) -> AttemptAnalysis {
    let rows: Vec<AttemptMeasures> = attempts.iter().filter_map(|a| a.measures).collect();
    let characters = attempts
        .iter()
        .map(|a| a.ch.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();

    let mut failed = Vec::new();
    let mut passed = Vec::new();
    let mut deviations = Vec::new();
    let mut scores = Vec::new();
    for attempt in attempts {
        let Some(measures) = attempt.measures else {
            continue;
        };
        // The grader's own rule, read from its constants rather than copied.
        let rebuilt = 100.0
            * (HEADLINE_SHAPE * measures.shape
                + HEADLINE_POSITION * measures.position
                + HEADLINE_ORDER * measures.order
                + HEADLINE_INK * measures.ink);
        deviations.push((attempt.score - rebuilt).abs());
        scores.push(attempt.score);
        if attempt.rating == "again" || attempt.rating == "hard" {
            failed.push(measures);
        } else {
            passed.push(measures);
        }
    }

    let column = |f: fn(&AttemptMeasures) -> f32| rows.iter().map(f).collect::<Vec<f32>>();
    let share = |count: usize| {
        if rows.is_empty() {
            f32::NAN
        } else {
            count as f32 / rows.len() as f32
        }
    };

    AttemptAnalysis {
        attempts: attempts.len(),
        measured: rows.len(),
        characters,
        shape: MeasureStats::of(column(|m| m.shape), Some(SHAPE_OK)),
        position: MeasureStats::of(column(|m| m.position), Some(POSITION_OK)),
        ink: MeasureStats::of(column(|m| m.ink), Some(INK_OK)),
        // Not scored and so without a bar, but reported: it is the signal that
        // was meant to catch a stroke that never reached part of the glyph, and
        // a measure that never moves is worth seeing even when it is not a gate.
        ink_coverage: MeasureStats::of(column(|m| m.ink_coverage), None),
        // Order has no bar of its own: `order_correct` is the verdict, at
        // 0.999, and it is reported as a rate below.
        order: MeasureStats::of(column(|m| m.order), None),
        legible_rate: share(rows.iter().filter(|m| m.legible).count()),
        order_correct_rate: share(rows.iter().filter(|m| m.order_correct).count()),
        failure_means: Means::of(&failed),
        pass_means: Means::of(&passed),
        score_deviation: if deviations.is_empty() {
            None
        } else {
            Some((
                deviations.iter().sum::<f32>() / deviations.len() as f32,
                deviations.iter().cloned().fold(0.0, f32::max),
            ))
        },
    }
}

/// Linear-interpolated percentile, the definition a spreadsheet's `PERCENTILE`
/// uses, so a number printed here can be checked against one there.
fn percentile(sorted: &[f32], p: f64) -> f32 {
    debug_assert!(!sorted.is_empty(), "callers check for an empty slice");
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = p * (sorted.len() - 1) as f64;
    let low = rank.floor() as usize;
    let high = rank.ceil() as usize;
    let fraction = (rank - low as f64) as f32;
    sorted[low] + (sorted[high] - sorted[low]) * fraction
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(
        ch: &str,
        score: f32,
        rating: &str,
        measures: Option<AttemptMeasures>,
    ) -> LoggedAttempt {
        LoggedAttempt {
            id: 1,
            device_id: "device-a".into(),
            seq: 1,
            ch: ch.into(),
            at: "2026-09-22T09:00:00Z".into(),
            score,
            rating: rating.into(),
            measures,
        }
    }

    /// Measures that add up to `score` under the grader's own weights, so the
    /// deviation check is exercised rather than tripped by the test's own data.
    fn consistent(shape: f32, position: f32, ink: f32, order: f32) -> (f32, AttemptMeasures) {
        let score = 100.0
            * (HEADLINE_SHAPE * shape
                + HEADLINE_POSITION * position
                + HEADLINE_ORDER * order
                + HEADLINE_INK * ink);
        let measures = AttemptMeasures {
            shape,
            position,
            ink,
            ink_coverage: 1.0,
            order,
            legible: shape >= SHAPE_OK && position >= POSITION_OK && ink >= INK_OK,
            order_correct: order >= 0.999,
        };
        (score, measures)
    }

    #[test]
    fn a_clean_log_shows_no_deviation_between_the_measures_and_the_score() {
        // The integrity check: if the stored measures are the ones that produced
        // the stored score, recomputing it with the grader's weights gives the
        // same number back.
        let mut rows = Vec::new();
        for (shape, position, ink, order) in
            [(1.0, 1.0, 1.0, 1.0), (0.7, 0.8, 0.9, 1.0), (0.5, 0.4, 0.3, 0.6)]
        {
            let (score, measures) = consistent(shape, position, ink, order);
            rows.push(attempt("好", score, "good", Some(measures)));
        }
        let report = analyse(&rows);
        let (mean, max) = report.score_deviation.expect("there are measured rows");
        assert!(mean < 0.001, "mean deviation {mean}");
        assert!(max < 0.001, "max deviation {max}");
    }

    #[test]
    fn a_score_that_does_not_match_its_measures_is_reported() {
        // The check has to be able to fail, or it says nothing. Here the score
        // is 10 points above what the measures justify.
        let (score, measures) = consistent(0.5, 0.5, 0.5, 0.5);
        let rows = vec![attempt("好", score + 10.0, "good", Some(measures))];
        let (_, max) = analyse(&rows).score_deviation.unwrap();
        assert!((max - 10.0).abs() < 0.001, "max deviation {max}");
    }

    #[test]
    fn unmeasured_attempts_count_but_contribute_nothing() {
        // A log written before schema 5, or merged from a peer, has rows with no
        // measures. They are attempts, so they are counted, and they are not
        // measured, so no distribution may claim them.
        let (score, measures) = consistent(0.8, 0.8, 0.8, 1.0);
        let rows = vec![
            attempt("好", score, "good", Some(measures)),
            attempt("学", 90.0, "good", None),
            attempt("学", 20.0, "again", None),
        ];
        let report = analyse(&rows);
        assert_eq!(report.attempts, 3);
        assert_eq!(report.measured, 1, "only one row carries measures");
        assert_eq!(report.characters, 2);
        assert_eq!(report.shape.n, 1);
        assert!(report.shape.mean.is_finite());
    }

    #[test]
    fn the_bar_reports_the_mass_it_is_deciding() {
        // Shape values around the 0.60 bar: one well below, one inside the
        // ten-point band, one well above. The bar is deciding the middle one.
        let mut rows = Vec::new();
        for shape in [0.30, 0.55, 0.80, 0.90] {
            let (score, measures) = consistent(shape, 0.9, 0.9, 1.0);
            rows.push(attempt("好", score, "good", Some(measures)));
        }
        let report = analyse(&rows);
        assert_eq!(report.shape.bar, Some(SHAPE_OK));
        assert_eq!(report.shape.n, 4);
        // 0.80 and 0.90 clear the bar; 0.30 and 0.55 do not.
        assert_eq!(report.shape.at_or_above_bar, Some(0.5));
        assert_eq!(report.shape.near_bar, Some(0.25), "0.55 is within 0.10 of 0.60");
    }

    #[test]
    fn a_measure_that_never_moves_is_visible_as_saturated() {
        // Ink coverage was expected to be the "you never drew that part" signal.
        // If it is 1.0 on every attempt it cannot tell anything apart, and the
        // report has to say so rather than leaving it to be inferred.
        let mut rows = Vec::new();
        for shape in [0.4, 0.6, 0.8] {
            let (score, mut measures) = consistent(shape, 0.9, 0.9, 1.0);
            measures.ink_coverage = 1.0;
            rows.push(attempt("好", score, "good", Some(measures)));
        }
        // One row where coverage does move, so the mean is not 1.0 — the point
        // is the *pinned* share, not the mean.
        let (score, mut measures) = consistent(0.9, 0.9, 0.9, 1.0);
        measures.ink_coverage = 0.2;
        rows.push(attempt("学", score, "good", Some(measures)));

        let report = analyse(&rows);
        assert_eq!(report.shape.pinned_high, 0.0);
        // Order is 1.0 on every row, so it is pinned — which is exactly the kind
        // of thing the report exists to surface.
        assert_eq!(report.order.pinned_high, 1.0);
    }

    #[test]
    fn failures_and_passes_are_averaged_apart() {
        // The weights exist to tell these apart. Here shape separates them and
        // position does not, which is the shape of finding this report is for.
        let mut rows = Vec::new();
        for _ in 0..3 {
            let (score, measures) = consistent(0.9, 0.7, 0.9, 1.0);
            rows.push(attempt("好", score, "good", Some(measures)));
        }
        for _ in 0..2 {
            let (score, measures) = consistent(0.4, 0.7, 0.9, 1.0);
            rows.push(attempt("好", score, "again", Some(measures)));
        }
        let report = analyse(&rows);
        let pass = report.pass_means.expect("passes were averaged");
        let fail = report.failure_means.expect("failures were averaged");
        assert!((pass.shape - 0.9).abs() < 0.001);
        assert!((fail.shape - 0.4).abs() < 0.001);
        assert!((pass.position - fail.position).abs() < 0.001, "position does not separate them");
        assert_eq!(report.legible_rate, 0.6, "3 of 5 cleared the bars");
        assert_eq!(report.order_correct_rate, 1.0);
    }

    #[test]
    fn an_empty_log_reports_nothing_rather_than_dividing_by_zero() {
        let report = analyse(&[]);
        assert_eq!(report.attempts, 0);
        assert_eq!(report.measured, 0);
        assert_eq!(report.shape.n, 0);
        assert!(report.score_deviation.is_none());
        assert!(report.failure_means.is_none());
    }

    #[test]
    fn percentiles_interpolate_like_a_spreadsheet() {
        let values = [0.0, 1.0, 2.0, 3.0, 4.0];
        assert!((percentile(&values, 0.50) - 2.0).abs() < 1e-6);
        assert!((percentile(&values, 0.25) - 1.0).abs() < 1e-6);
        assert!((percentile(&values, 0.10) - 0.4).abs() < 1e-6);
    }
}
