//! Self-consistency and tolerance check against the real dataset.
//!
//! Two questions this answers, which unit tests with hand-made strokes cannot:
//!
//! 1. **Self-consistency** — grading a character's own reference strokes against
//!    itself must score a perfect 100 for *every* character. Anything less means
//!    resampling, normalisation or the tolerance handling misbehaves on some
//!    real stroke, for example a very short 点.
//! 2. **Tolerance** — a correct but hand-wobbly attempt must still be judged
//!    legible. This jitters the reference strokes by a controlled amount and
//!    reports how many characters survive, which is how the shape and placement
//!    tolerances were tuned.
//!
//! ```text
//! cargo run --release -p hanzi-core --example selfcheck [-- <artifact>]
//! ```

use hanzi_core::geom::{path_length, resample, shape_distance};
use hanzi_core::{grade, Dataset, GradeOptions, Point};

/// Deterministic noise, so runs are comparable.
struct Lcg(u64);
impl Lcg {
    fn normal(&mut self) -> f32 {
        // Sum of uniforms is close enough to a bell for this purpose.
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

fn percentile(sorted: &[f32], p: f64) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

/// The same stroke with its samples bunched towards one end.
///
/// Every point still lies on the original polyline, so the endpoints and the
/// ink are untouched — only the density differs, exactly as when a pointer sets
/// off slowly and is flicked at the end (or the reverse). `power` above 1
/// bunches at the start, below 1 at the end.
fn uneven_sampling(stroke: &[Point], power: f32, n: usize) -> Vec<Point> {
    let dense = resample(stroke, 512);
    let m = dense.len();
    if m < 2 {
        return dense;
    }
    (0..n)
        .map(|i| {
            let t = (i as f32 / (n - 1) as f32).powf(power);
            dense[((t * (m - 1) as f32).round() as usize).min(m - 1)]
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/data/hanzi.bin.gz").to_string()
    });
    let dataset = Dataset::from_gzip_bytes(&std::fs::read(&path)?)?;
    let options = GradeOptions::default();
    println!(
        "loaded {} characters from {path}\n(resample_k={}, min_stroke_len={}, global_fit={})\n",
        dataset.len(),
        options.resample_k,
        options.min_stroke_len,
        options.global_fit
    );

    // --- reference median lengths ------------------------------------------
    let mut lengths: Vec<f32> = dataset
        .chars()
        .iter()
        .flat_map(|c| c.medians.iter().map(|m| path_length(m)))
        .collect();
    lengths.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let below = lengths
        .iter()
        .filter(|l| **l < options.min_stroke_len)
        .count();
    println!("reference stroke lengths (design units, box is 1024):");
    println!(
        "  min {:.1}  p1 {:.1}  p50 {:.1}  max {:.1}",
        percentile(&lengths, 0.0),
        percentile(&lengths, 0.01),
        percentile(&lengths, 0.5),
        percentile(&lengths, 1.0),
    );
    println!(
        "  {below} of {} strokes are shorter than min_stroke_len ({}) \
         <- these would be discarded as stray taps\n",
        lengths.len(),
        options.min_stroke_len
    );

    // --- 1. self-consistency ----------------------------------------------
    let mut scores: Vec<f32> = Vec::with_capacity(dataset.len());
    let mut imperfect = Vec::new();
    for character in dataset.chars() {
        let report = grade(character.reference_medians(), &character.medians, &options);
        scores.push(report.overall);
        if !report.is_perfect() && character.is_teachable() {
            imperfect.push((character.ch, report.overall, report.stray_strokes));
        }
    }
    let mut sorted = scores.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("self-consistency (each character graded against its own strokes):");
    println!(
        "  min {:.2}  p1 {:.2}  p50 {:.2}  mean {:.3}",
        percentile(&sorted, 0.0),
        percentile(&sorted, 0.01),
        percentile(&sorted, 0.5),
        sorted.iter().sum::<f32>() / sorted.len() as f32,
    );
    println!(
        "  {} of {} teachable characters are not perfect",
        imperfect.len(),
        dataset.ranked().count()
    );
    for (ch, score, stray) in imperfect.iter().take(15) {
        println!("    {ch}  {score:.2}  stray={stray}");
    }
    println!();

    // --- 2. tolerance under a wobbly hand ---------------------------------
    println!("tolerance: reference strokes jittered, then graded as an attempt");
    println!("  (shape and position are character means; 'fail' counts characters");
    println!("   whose mean for that metric falls below the 0.60 legibility bar)");
    for (label, sigma) in [
        ("tight   sigma=5   ", 5.0f32),
        ("normal  sigma=15  ", 15.0),
        ("sloppy  sigma=30  ", 30.0),
        ("rough   sigma=50  ", 50.0),
    ] {
        let mut rng = Lcg(0x5EED);
        let mut legible = 0usize;
        let (mut total, mut sum) = (0usize, 0.0f32);
        let (mut shape_sum, mut position_sum) = (0.0f32, 0.0f32);
        let (mut shape_fail, mut position_fail) = (0usize, 0usize);
        for character in dataset.ranked() {
            let attempt = jitter(&character.medians, sigma, &mut rng);
            let report = grade(character.reference_medians(), &attempt, &options);
            total += 1;
            sum += report.overall;
            shape_sum += report.shape_score;
            position_sum += report.position_score;
            if report.shape_score < 0.60 {
                shape_fail += 1;
            }
            if report.position_score < 0.60 {
                position_fail += 1;
            }
            if report.legible {
                legible += 1;
            }
        }
        let n = total as f32;
        println!(
            "  {label} legible {:5.1}%  mean {:5.1}  shape {:.2} (fail {:5.1}%)  position {:.2} (fail {:5.1}%)",
            100.0 * legible as f32 / n,
            sum / n,
            shape_sum / n,
            100.0 * shape_fail as f32 / n,
            position_sum / n,
            100.0 * position_fail as f32 / n,
        );
    }

    // --- 3. how well does the shape metric separate right from wrong? ------
    //
    // This is what SHAPE_TOL has to be tuned against. For every stroke we record
    // the distance to its own reference stroke (correct, but jittered as a real
    // hand would be) and to the *nearest other* stroke of the same character —
    // the hardest possible confusion, such as 撇 against 捺 or 一 against 二.
    // A good tolerance sits in the gap between the two distributions.
    println!("\nshape distance, correct vs nearest-wrong pairing (jitter sigma=20):");
    let mut correct_d: Vec<f32> = Vec::new();
    let mut wrong_d: Vec<f32> = Vec::new();
    let mut rng = Lcg(0xBEEF);
    for character in dataset.ranked() {
        let medians = &character.medians;
        if medians.len() < 2 {
            continue;
        }
        let jittered = jitter(medians, 20.0, &mut rng);
        for (i, stroke) in jittered.iter().enumerate() {
            correct_d.push(shape_distance(stroke, &medians[i], 16).best());
            let nearest_wrong = medians
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, other)| shape_distance(stroke, other, 16).best())
                .fold(f32::INFINITY, f32::min);
            wrong_d.push(nearest_wrong);
        }
    }
    correct_d.sort_by(|a, b| a.partial_cmp(b).unwrap());
    wrong_d.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for (label, values) in [("correct ", &correct_d), ("nearest-wrong", &wrong_d)] {
        println!(
            "  {label:14} p05 {:.3}  p25 {:.3}  p50 {:.3}  p75 {:.3}  p95 {:.3}",
            percentile(values, 0.05),
            percentile(values, 0.25),
            percentile(values, 0.50),
            percentile(values, 0.75),
            percentile(values, 0.95),
        );
    }
    let overlap = correct_d
        .iter()
        .filter(|d| **d > percentile(&wrong_d, 0.05))
        .count();
    println!(
        "  correct strokes beyond the 5th percentile of wrong ones: {:.1}%  \
         (lower is better separation)",
        100.0 * overlap as f32 / correct_d.len() as f32
    );

    // --- 4. robustness to how the pointer sampled the stroke ---------------
    //
    // The jitter above CANNOT find a bug in this area: adding noise to the
    // reference preserves its sample density, so an attempt derived that way
    // never exercises the difference between "where the ink is" and "where the
    // samples happen to be". Pointer samples arrive by time, not by distance, so
    // a stroke drawn slowly at one end and flicked at the other arrives bunched
    // — same geometry, different sampling. Grading may not notice.
    //
    // This is here because a placement term built on the sample mean did exactly
    // that, and turned perfect traces of long strokes into "wrong place" while
    // every number above stayed healthy.
    println!("\nsampling robustness: strokes resampled with hand-like density, geometry untouched");
    let mut worst_drop = 0.0f32;
    let mut worst_at = ('?', 0usize);
    let mut verdict_changes = 0usize;
    let mut worst_overall_drop = 0.0f32;
    for character in dataset.chars() {
        let attempt: Vec<Vec<Point>> = character
            .medians
            .iter()
            .enumerate()
            // Alternate which end is dense, so both directions are covered.
            .map(|(i, m)| uneven_sampling(m, if i % 2 == 0 { 2.5 } else { 0.4 }, 96))
            .collect();
        let even = grade(character.reference_medians(), &character.medians, &options);
        let bunched = grade(character.reference_medians(), &attempt, &options);

        // The comparison, not the absolute score: sampling with fewer points
        // along a curve cuts corners, which costs a little on its own. What
        // must not happen is a *verdict* moving, or a stroke's placement
        // collapsing, when the geometry has not changed.
        worst_overall_drop = worst_overall_drop.max(even.overall - bunched.overall);
        for (a, b) in even.strokes.iter().zip(&bunched.strokes) {
            let drop = a.position - b.position;
            if drop > worst_drop {
                worst_drop = drop;
                worst_at = (character.ch, a.ref_index + 1);
            }
            if a.verdict != b.verdict {
                verdict_changes += 1;
            }
        }
    }
    println!(
        "  worst overall drop {worst_overall_drop:.2}  \
         worst stroke placement drop {:.3} ({} stroke {})",
        worst_drop, worst_at.0, worst_at.1
    );
    println!(
        "  strokes whose verdict changed with the sampling: {verdict_changes}  \
         <- must be 0: the geometry is identical"
    );

    Ok(())
}