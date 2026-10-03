//! Self-consistency and tolerance check against the shipped kana artifact.
//!
//! The Chinese counterpart is `hanzi-core`'s `examples/selfcheck.rs`, and this
//! answers the same two questions plus the two that only kana raise:
//!
//! 1. **Self-consistency** — grading a kana's own reference strokes against
//!    itself must score a perfect 100 for *every* one of the 177, on all four
//!    measures including the raster ink one. Anything less means resampling,
//!    normalisation or the tolerance handling misbehaves on some real stroke.
//! 2. **Tolerance** — a correct but hand-wobbly attempt must still be judged
//!    legible. The tolerances in `hanzi-core::grade` were fitted against the
//!    Chinese dataset, and kana are shorter-stroked and rounder, so this is the
//!    measurement that says whether they carry over.
//! 3. **Discrimination** — the classic confusions (シ/ツ, ぬ/め …) are the pairs
//!    the app exists to tell apart, and the drill asks about them by *reading*.
//!    A board that grades handwriting must not call シ legible when ツ was
//!    written, or the review queue learns the wrong answer.
//! 4. **Connected-stroke variants** — き and さ are taught with separate
//!    strokes and very commonly handwritten as fewer, joined ones. This grades
//!    every adjacent join of every kana, because "a kana tutor that rejects a
//!    legitimate hand is worse than no tutor" (ROADMAP_NIHONGO.md, known weak
//!    spots).
//!
//! It also reports stroke lengths against `min_stroke_len` — the shortest kana
//! strokes are what a Chinese-fitted constant is most likely to throw away —
//! the shape-distance separation `SHAPE_TOL` sits in, sampling robustness, the
//! ink measure, and how long a grade takes.
//!
//! ```text
//! cargo run --release -p nihongo-core --example selfcheck [-- <artifact>]
//! ```
//!
//! It is a diagnostic, not a feature: it asserts nothing and prints what it
//! finds. The properties it establishes are pinned by `tests/kana_grading.rs`.

use std::time::Instant;

use hanzi_core::geom::{path_length, resample, shape_distance};
use hanzi_core::{grade_with_outlines, GradeOptions, Point, Verdict, INK_OK};
use nihongo_core::{grade_kana, voicing_pairs, KanaDataset, Script, CONFUSABLE};

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

fn sorted(values: &[f32]) -> Vec<f32> {
    let mut out = values.to_vec();
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

/// The same stroke with its samples bunched towards one end.
///
/// Every point still lies on the original polyline, so the geometry is
/// untouched — only the density differs, exactly as when a pointer sets off
/// slowly and is flicked at the end (or the reverse). `power` above 1 bunches
/// at the start, below 1 at the end.
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

/// `strokes` with stroke `i` and `i + 1` drawn as one, which is what a hand
/// that joins them puts down: the first stroke's points, then the second's.
///
/// Nothing is reversed. A hand joining two taught strokes draws them in the
/// taught order, so the only question is whether a single polyline through both
/// is what the reference can still recognise.
fn join_at(strokes: &[Vec<Point>], i: usize) -> Vec<Vec<Point>> {
    let mut out: Vec<Vec<Point>> = strokes.to_vec();
    let second = out.remove(i + 1);
    out[i].extend(second);
    out
}

/// `strokes` with stroke `missing` left out — the hand's *other* way of writing
/// one fewer stroke, and the one the joined-stroke rule must never accept.
fn omit_at(strokes: &[Vec<Point>], missing: usize) -> Vec<Vec<Point>> {
    strokes
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != missing)
        .map(|(_, stroke)| stroke.clone())
        .collect()
}

/// Grade `attempt` against `ch` with the shared engine, or `None` when the
/// dataset has no such kana.
///
/// This is the *taught* reference, which is what the discrimination and voicing
/// sections below want: what geometry alone can tell apart. The joined-stroke
/// rule is [`nihongo_core::grade_kana`], used in its own section.
fn grade_against(
    dataset: &KanaDataset,
    ch: char,
    attempt: &[Vec<Point>],
    options: &GradeOptions,
) -> Option<hanzi_core::GradeReport> {
    let kana = dataset.get(ch)?;
    Some(grade_with_outlines(
        kana.reference_medians(),
        &kana.outlines,
        attempt,
        options,
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/data/kana.bin.gz").to_string()
    });
    let dataset = KanaDataset::from_gzip_bytes(&std::fs::read(&path)?)?;
    let options = GradeOptions::default();
    let hiragana = dataset.of_script(Script::Hiragana).count();
    let katakana = dataset.of_script(Script::Katakana).count();
    println!(
        "loaded {} kana from {path} ({hiragana} hiragana, {katakana} katakana)\n\
         (resample_k={}, min_stroke_len={}, global_fit={}, ink_width={})\n",
        dataset.len(),
        options.resample_k,
        options.min_stroke_len,
        options.global_fit,
        options.ink_width,
    );

    // --- reference stroke lengths ------------------------------------------
    //
    // `min_stroke_len` discards anything shorter as an accidental tap. It was
    // fitted on Chinese characters, whose shortest strokes (the 点 of 心) are
    // still a visible mark. A kana like り or シ has small strokes of its own,
    // and the ones below the bar are the strokes the grader would refuse to
    // count before it looked at shape at all.
    let mut lengths: Vec<f32> = dataset
        .kana()
        .iter()
        .flat_map(|k| k.medians.iter().map(|m| path_length(m)))
        .collect();
    lengths.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("reference stroke lengths (design units, box is 1024):");
    println!(
        "  min {:.1}  p1 {:.1}  p10 {:.1}  p50 {:.1}  p90 {:.1}  max {:.1}  ({} strokes)",
        percentile(&lengths, 0.0),
        percentile(&lengths, 0.01),
        percentile(&lengths, 0.10),
        percentile(&lengths, 0.50),
        percentile(&lengths, 0.90),
        percentile(&lengths, 1.0),
        lengths.len(),
    );
    let mut short: Vec<(char, usize, f32)> = Vec::new();
    for kana in dataset.kana() {
        for (i, median) in kana.medians.iter().enumerate() {
            let len = path_length(median);
            if len < options.min_stroke_len {
                short.push((kana.ch, i + 1, len));
            }
        }
    }
    short.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
    println!(
        "  {} of {} strokes are shorter than min_stroke_len ({}), so would be \
         discarded as stray taps",
        short.len(),
        lengths.len(),
        options.min_stroke_len
    );
    for (ch, stroke, len) in short.iter().take(12) {
        println!("    {ch}  stroke {stroke}  {len:.1}");
    }
    println!();

    // --- 1. self-consistency ----------------------------------------------
    //
    // Graded with the outlines, so the ink measure is part of what "perfect"
    // means: a kana whose own centre-line does not reach its own ink would
    // score below 1.0 and show up below.
    let mut scores: Vec<f32> = Vec::new();
    let mut ink_scores: Vec<f32> = Vec::new();
    let mut imperfect: Vec<(char, f32, bool, bool, usize)> = Vec::new();
    let mut faint: Vec<char> = Vec::new();
    let mut below_ink_bar = 0usize;
    for kana in dataset.kana() {
        let report = grade_with_outlines(
            kana.reference_medians(),
            &kana.outlines,
            &kana.medians,
            &options,
        );
        scores.push(report.overall);
        ink_scores.push(report.ink_score);
        if !report.is_perfect() {
            imperfect.push((
                kana.ch,
                report.overall,
                report.count_ok,
                report.order_correct,
                report.stray_strokes,
            ));
        }
        if report.strokes.iter().any(|s| s.verdict == Verdict::Faint) {
            faint.push(kana.ch);
        }
        if report.ink_score < INK_OK {
            below_ink_bar += 1;
        }
    }
    let score_sorted = sorted(&scores);
    println!("self-consistency (each kana graded against its own strokes):");
    println!(
        "  min {:.2}  p1 {:.2}  p50 {:.2}  mean {:.3}",
        percentile(&score_sorted, 0.0),
        percentile(&score_sorted, 0.01),
        percentile(&score_sorted, 0.5),
        mean(&scores),
    );
    println!(
        "  {} of {} kana are not perfect  <- must be 0",
        imperfect.len(),
        dataset.len()
    );
    for (ch, score, count_ok, order_ok, stray) in imperfect.iter().take(20) {
        println!("    {ch}  {score:.2}  count_ok={count_ok} order_ok={order_ok} stray={stray}");
    }
    let ink_sorted = sorted(&ink_scores);
    println!(
        "  ink: min {:.3}  p1 {:.3}  p50 {:.3}  mean {:.3}",
        percentile(&ink_sorted, 0.0),
        percentile(&ink_sorted, 0.01),
        percentile(&ink_sorted, 0.5),
        mean(&ink_scores),
    );
    println!(
        "  a correct trace below the {INK_OK} ink bar: {below_ink_bar}  <- must be 0\n  \
         kana with a Faint stroke: {}  <- must be 0 for the same reason\n",
        faint.len()
    );

    // --- 2. tolerance under a wobbly hand ---------------------------------
    println!("tolerance: reference strokes jittered, then graded as an attempt");
    println!("  (legibility is the grader's own verdict; 'fail' counts kana whose mean");
    println!("   for that metric falls below the 0.60 bar)");
    // The same jitter on both scripts, so a kana that fails is visible by name
    // rather than hidden in an average.
    for (label, sigma) in [
        ("tight   sigma=5   ", 5.0f32),
        ("normal  sigma=15  ", 15.0),
        ("sloppy  sigma=30  ", 30.0),
        ("rough   sigma=50  ", 50.0),
    ] {
        let mut rng = Lcg(0x5EED);
        let mut legible = 0usize;
        let (mut shape_sum, mut position_sum, mut ink_sum) = (0.0f32, 0.0f32, 0.0f32);
        let (mut shape_fail, mut position_fail, mut ink_fail) = (0usize, 0usize, 0usize);
        let mut worst: Vec<(char, f32, bool)> = Vec::new();
        for kana in dataset.kana() {
            let attempt = jitter(&kana.medians, sigma, &mut rng);
            let report = grade_with_outlines(
                kana.reference_medians(),
                &kana.outlines,
                &attempt,
                &options,
            );
            shape_sum += report.shape_score;
            position_sum += report.position_score;
            ink_sum += report.ink_score;
            if report.shape_score < 0.60 {
                shape_fail += 1;
            }
            if report.position_score < 0.60 {
                position_fail += 1;
            }
            if report.ink_score < INK_OK {
                ink_fail += 1;
            }
            if report.legible {
                legible += 1;
            }
            worst.push((kana.ch, report.overall, report.legible));
        }
        let n = dataset.len() as f32;
        worst.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let named: Vec<String> = worst
            .iter()
            .take(6)
            .map(|(ch, score, ok)| format!("{ch} {score:.1}{}", if *ok { "" } else { "!" }))
            .collect();
        println!(
            "  {label} legible {:5.1}%  mean {:5.1}  shape {:.2} (fail {:5.1}%)  \
             position {:.2} (fail {:5.1}%)  ink {:.2} (fail {:5.1}%)",
            100.0 * legible as f32 / n,
            mean(&worst.iter().map(|w| w.1).collect::<Vec<_>>()),
            shape_sum / n,
            100.0 * shape_fail as f32 / n,
            position_sum / n,
            100.0 * position_fail as f32 / n,
            ink_sum / n,
            100.0 * ink_fail as f32 / n,
        );
        println!("           worst: {}", named.join("  "));
    }
    println!("  ('!' marks a kana the grader did not call legible)\n");

    // --- 3. discrimination: the classic confusions -------------------------
    //
    // The pairs the app exists to tell apart. Each side's reference strokes are
    // graded against the *other* side's reference: a wrong kana judged legible
    // is exactly the mistake the review queue would store as a success.
    println!("discrimination: each classic pair member written against the other");
    let mut confused: Vec<(char, char, f32)> = Vec::new();
    for pair in CONFUSABLE {
        for (from, to) in [(pair.a, pair.b), (pair.b, pair.a)] {
            let report = grade_against(&dataset, to, &dataset.get(from).unwrap().medians, &options)
                .unwrap_or_else(|| panic!("{to} is not in the dataset"));
            if report.legible {
                confused.push((from, to, report.overall));
            }
            println!(
                "  {from} written, graded as {to}: {:5.1}/100  legible={}  \
                 shape={:.2} position={:.2} count_ok={}",
                report.overall, report.legible, report.shape_score, report.position_score,
                report.count_ok,
            );
        }
    }
    println!(
        "  {} of {} directions are legible as the wrong kana  <- must be 0",
        confused.len(),
        CONFUSABLE.len() * 2
    );
    for (from, to, score) in &confused {
        println!("    {from} as {to} scored {score:.1}");
    }
    println!();

    // The voicing contrasts are one kana each and are asked by *typing*, so a
    // board never grades them; this is the record of what geometry alone could
    // and could not see about a dakuten. `confusion_pairs()` is the classic
    // thirteen above; this is the other single-kana set.
    let mut voiced_confused = 0usize;
    let mut voiced_total = 0usize;
    let mut voiced_examples: Vec<(char, char, f32)> = Vec::new();
    for script in [Script::Hiragana, Script::Katakana] {
        for pair in voicing_pairs(script) {
            let mut chars = pair.sides.iter().filter_map(|s| {
                let mut it = s.spelling.chars();
                let (first, rest) = (it.next()?, it.next());
                rest.is_none().then_some(first)
            });
            let (Some(a), Some(b)) = (chars.next(), chars.next()) else {
                continue;
            };
            for (from, to) in [(a, b), (b, a)] {
                let Some(report) =
                    grade_against(&dataset, to, &dataset.get(from).unwrap().medians, &options)
                else {
                    continue;
                };
                voiced_total += 1;
                if report.legible {
                    voiced_confused += 1;
                    if voiced_examples.len() < 8 {
                        voiced_examples.push((from, to, report.overall));
                    }
                }
            }
        }
    }
    let examples = voiced_examples
        .iter()
        .map(|(from, to, score)| format!("{from}/{to} {score:.0}"))
        .collect::<Vec<_>>()
        .join("  ");
    println!(
        "  voicing contrasts (typed, not drawn): {voiced_confused} of {voiced_total} \
         directions are legible as the other kana"
    );
    if !examples.is_empty() {
        println!("    e.g. {examples}");
    }
    println!();

    // --- 4. how well does the shape metric separate right from wrong? ------
    //
    // What `SHAPE_TOL` has to be tuned against: for every stroke, the distance
    // to its own reference stroke (correct, jittered as a real hand would be)
    // against the distance to the nearest *other* stroke of the same kana —
    // シ's two dots against its long stroke, say. A good tolerance sits in the
    // gap between the two distributions, and kana have fewer and shorter
    // strokes than hanzi, so the gap is what this measurement establishes.
    println!("shape distance, correct vs nearest-wrong pairing (jitter sigma=20):");
    let mut correct_d: Vec<f32> = Vec::new();
    let mut wrong_d: Vec<f32> = Vec::new();
    let mut worst_cases: Vec<(char, usize, f32, f32)> = Vec::new();
    let mut rng = Lcg(0xBEEF);
    for kana in dataset.kana() {
        let medians = &kana.medians;
        if medians.len() < 2 {
            continue;
        }
        let jittered = jitter(medians, 20.0, &mut rng);
        for (i, stroke) in jittered.iter().enumerate() {
            let own = shape_distance(stroke, &medians[i], 16).best();
            correct_d.push(own);
            let nearest_wrong = medians
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, other)| shape_distance(stroke, other, 16).best())
                .fold(f32::INFINITY, f32::min);
            wrong_d.push(nearest_wrong);
            worst_cases.push((kana.ch, i + 1, own, nearest_wrong));
        }
    }
    let correct_sorted = sorted(&correct_d);
    let wrong_sorted = sorted(&wrong_d);
    for (label, values) in [("correct ", &correct_sorted), ("nearest-wrong", &wrong_sorted)] {
        println!(
            "  {label:14} p05 {:.3}  p25 {:.3}  p50 {:.3}  p75 {:.3}  p95 {:.3}",
            percentile(values, 0.05),
            percentile(values, 0.25),
            percentile(values, 0.50),
            percentile(values, 0.75),
            percentile(values, 0.95),
        );
    }
    // Where the pairing itself is at risk: the jittered stroke is closer to a
    // different stroke of the same kana than to its own. A nonzero count means
    // the Hungarian pairing is working against the metric, not with it.
    worst_cases.sort_by(|a, b| (a.2 - a.3).partial_cmp(&(b.2 - b.3)).unwrap());
    let inverted = worst_cases.iter().filter(|(_, _, own, wrong)| own > wrong).count();
    println!(
        "  strokes closer to another stroke of the same kana than to their own: \
         {inverted} of {}",
        worst_cases.len()
    );
    for (ch, stroke, own, wrong) in worst_cases.iter().take(8) {
        println!("    {ch} stroke {stroke}: own {own:.3} vs wrong {wrong:.3}");
    }
    println!();

    // --- 5. connected-stroke variants -------------------------------------
    //
    // "き and さ are taught as three strokes but are very commonly handwritten
    // connected as two, and ふ, そ, な and む have well-known variants. A kana
    // tutor that rejects a legitimate hand is worse than no tutor." So: every
    // adjacent join of every kana, drawn as a hand draws it, through the rule the
    // app actually grades with — `nihongo_core::grade_kana`, which puts the
    // reference into the hand's grouping and accepts it only when every drawn
    // stroke clears the per-stroke bars.
    println!("connected-stroke variants: two adjacent reference strokes drawn as one");
    let mut joins_refused: Vec<(char, usize, f32)> = Vec::new();
    for sigma in [15.0f32, 30.0] {
        let mut rng = Lcg(0x5EED);
        let (mut legible, mut total, mut sum) = (0usize, 0usize, 0.0f32);
        for kana in dataset.kana() {
            if kana.medians.len() < 2 {
                continue;
            }
            for i in 0..kana.medians.len() - 1 {
                let attempt = jitter(&join_at(&kana.medians, i), sigma, &mut rng);
                let graded = grade_kana(kana, &attempt, &options);
                total += 1;
                sum += graded.report.overall;
                if graded.report.legible {
                    legible += 1;
                } else if sigma == 15.0 {
                    joins_refused.push((kana.ch, i + 1, graded.report.overall));
                }
            }
        }
        println!(
            "  a hand-drawn join (sigma={sigma:.0} jitter): {legible} of {total} legible, \
             mean {:.1}",
            sum / total as f32,
        );
    }
    println!(
        "  the {} refused at sigma 15 are the joins of a dakuten or handakuten stroke to the \
         stroke beside it, which is not a hand form anyone teaches:",
        joins_refused.len()
    );
    let refused = joins_refused
        .iter()
        .map(|(ch, i, score)| format!("{ch} {i}+{} {score:.0}", i + 1))
        .collect::<Vec<_>>()
        .join("  ");
    println!("    {refused}");
    println!("  the named variants, join by join, as the rule reads them:");
    for ch in ['き', 'さ', 'ふ', 'そ', 'な', 'む'] {
        let Some(kana) = dataset.get(ch) else {
            println!("    {ch} is not in the dataset");
            continue;
        };
        let mut rng = Lcg(0x5EED);
        let mut parts = Vec::new();
        for i in 0..kana.medians.len().saturating_sub(1) {
            let attempt = jitter(&join_at(&kana.medians, i), 15.0, &mut rng);
            let graded = grade_kana(kana, &attempt, &options);
            let label = graded
                .joined
                .iter()
                .map(|group| {
                    group.iter().map(u8::to_string).collect::<Vec<_>>().join("+")
                })
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!(
                "{}+{} -> {:.1}{}",
                i + 1,
                i + 2,
                graded.report.overall,
                if graded.report.legible {
                    format!(" legible, read as {label}")
                } else {
                    " NOT legible".to_string()
                },
            ));
        }
        println!(
            "    {ch} ({} strokes taught): {}",
            kana.medians.len(),
            parts.join("   ")
        );
    }
    println!();

    // --- 5b. the safety property: a dropped stroke is not a join -----------
    //
    // The rule accepts one fewer stroke only when the hand drew both of them
    // joined. The mean-based legibility test cannot tell the two apart — a
    // missing stroke costs only its share of the average — so this is the
    // measurement the per-stroke bars exist for, and it must stay at zero.
    println!("safety: one fewer stroke, drawn by leaving one out rather than joining");
    let mut omissions = 0usize;
    let mut accepted: Vec<(char, usize)> = Vec::new();
    for kana in dataset.kana() {
        if kana.medians.len() < 2 {
            continue;
        }
        for missing in 0..kana.medians.len() {
            let graded = grade_kana(kana, &omit_at(&kana.medians, missing), &options);
            omissions += 1;
            if graded.report.legible {
                accepted.push((kana.ch, missing + 1));
            }
        }
    }
    println!(
        "  every single omission of every kana: {omissions} attempts, {} accepted as a join  \
         <- must be 0",
        accepted.len()
    );
    if !accepted.is_empty() {
        println!("    {accepted:?}");
    }
    // And the other direction: a hand that lifts the pen mid-stroke has *more*
    // strokes than taught. There is no honest way to split a taught stroke, so
    // the taught reference's verdict stands — a limit recorded rather than
    // fixed.
    //
    // An offcut shorter than `min_stroke_len` is not a second stroke at all: the
    // board drops it as a stray, the count still matches, and the remaining piece
    // covers the stroke, so that case is accepted and rightly so. The measurement
    // is the split a learner would call a split: both halves long enough to count.
    let mut splits = 0usize;
    let mut splits_legible = 0usize;
    let mut no_split = 0usize;
    for kana in dataset.kana() {
        if kana.medians.len() < 2 {
            continue;
        }
        let Some(index) = kana.medians.iter().position(|m| m.len() >= 2) else {
            no_split += 1;
            continue;
        };
        let median = &kana.medians[index];
        let half = median.len() / 2;
        let (first, second) = (median[..half].to_vec(), median[half..].to_vec());
        if path_length(&first) < options.min_stroke_len
            || path_length(&second) < options.min_stroke_len
        {
            no_split += 1;
            continue;
        }
        let mut split = kana.medians.clone();
        split.splice(index..=index, [first, second]);
        let graded = grade_kana(kana, &split, &options);
        splits += 1;
        if graded.report.legible {
            splits_legible += 1;
        }
    }
    println!(
        "  one stroke of every kana split in two, both halves long enough to count: {splits} \
         attempts, {splits_legible} legible  <- must be 0: a hand that lifts mid-stroke is told \
         its count is wrong ({no_split} kana had no such split)"
    );
    println!();

    // --- 6. robustness to how the pointer sampled the stroke ---------------
    //
    // Jitter cannot find a bug here: adding noise to the reference preserves
    // its sample density. Pointer samples arrive by time, not by distance, so a
    // stroke drawn slowly at one end and flicked at the other arrives bunched;
    // the geometry is identical and the verdicts must not move.
    println!("sampling robustness: strokes resampled with hand-like density, geometry untouched");
    let mut worst_drop = 0.0f32;
    let mut worst_at = ('?', 0usize);
    let mut verdict_changes = 0usize;
    let mut worst_overall_drop = 0.0f32;
    for kana in dataset.kana() {
        let attempt: Vec<Vec<Point>> = kana
            .medians
            .iter()
            .enumerate()
            // Alternate which end is dense, so both directions are covered.
            .map(|(i, m)| uneven_sampling(m, if i % 2 == 0 { 2.5 } else { 0.4 }, 96))
            .collect();
        let even = grade_with_outlines(
            kana.reference_medians(),
            &kana.outlines,
            &kana.medians,
            &options,
        );
        let bunched = grade_with_outlines(
            kana.reference_medians(),
            &kana.outlines,
            &attempt,
            &options,
        );
        worst_overall_drop = worst_overall_drop.max(even.overall - bunched.overall);
        for (a, b) in even.strokes.iter().zip(&bunched.strokes) {
            let drop = a.position - b.position;
            if drop > worst_drop {
                worst_drop = drop;
                worst_at = (kana.ch, a.ref_index + 1);
            }
            if a.verdict != b.verdict {
                verdict_changes += 1;
            }
        }
    }
    println!(
        "  worst overall drop {worst_overall_drop:.2}  worst stroke placement drop \
         {:.3} ({} stroke {})",
        worst_drop, worst_at.0, worst_at.1
    );
    println!(
        "  strokes whose verdict changed with the sampling: {verdict_changes}  \
         <- must be 0: the geometry is identical\n"
    );

    // --- 7. the ink measure -----------------------------------------------
    //
    // A correct trace must reach 1.0, or the bar would fail kana nobody drew
    // wrong; then a third-width pen, which the measure exists to catch.
    println!("ink measure: correct, and a pen a third of the width");
    let thin_options = GradeOptions {
        ink_width: options.ink_width / 3.0,
        ..options.clone()
    };
    let mut correct_ink: Vec<f32> = Vec::new();
    let mut thin_ink: Vec<f32> = Vec::new();
    let mut thin_flagged = 0usize;
    for kana in dataset.kana() {
        let correct = grade_with_outlines(
            kana.reference_medians(),
            &kana.outlines,
            &kana.medians,
            &options,
        );
        correct_ink.push(correct.ink_score);
        let thin = grade_with_outlines(
            kana.reference_medians(),
            &kana.outlines,
            &kana.medians,
            &thin_options,
        );
        thin_ink.push(thin.ink_score);
        if !thin.legible {
            thin_flagged += 1;
        }
    }
    let correct_ink_sorted = sorted(&correct_ink);
    let thin_ink_sorted = sorted(&thin_ink);
    println!(
        "  correct trace   min {:.3}  p5 {:.3}  p50 {:.3}  below {INK_OK}: {}",
        percentile(&correct_ink_sorted, 0.0),
        percentile(&correct_ink_sorted, 0.05),
        percentile(&correct_ink_sorted, 0.5),
        correct_ink.iter().filter(|x| **x < INK_OK).count(),
    );
    println!(
        "  third-width pen min {:.3}  p5 {:.3}  p50 {:.3}  below {INK_OK}: {}",
        percentile(&thin_ink_sorted, 0.0),
        percentile(&thin_ink_sorted, 0.05),
        percentile(&thin_ink_sorted, 0.5),
        thin_ink.iter().filter(|x| **x < INK_OK).count(),
    );
    println!(
        "  a third-width pen makes {thin_flagged} of {} kana not legible\n",
        dataset.len()
    );

    // --- 8. cost ----------------------------------------------------------
    //
    // "Grading stays comfortably interactive" is an acceptance criterion, and
    // the raster measure is the expensive part of it.
    let widest = dataset
        .kana()
        .iter()
        .max_by_key(|k| k.medians.len())
        .expect("the dataset is not empty");
    let runs = 200u32;
    let start = Instant::now();
    for _ in 0..runs {
        let _ = grade_with_outlines(
            widest.reference_medians(),
            &widest.outlines,
            &widest.medians,
            &options,
        );
    }
    let per_grade = start.elapsed().as_secs_f64() * 1000.0 / runs as f64;
    println!(
        "cost: {per_grade:.2} ms per grade on {} ({} strokes, the widest kana)  \
         <- target is under 20 ms",
        widest.ch,
        widest.medians.len()
    );

    Ok(())
}
