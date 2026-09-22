//! Read a practice log and say what it implies about the grader.
//!
//! The tolerances were set against synthetic jitter and the four headline
//! weights are an equal quarter each — guesses, both of them. This prints the
//! distributions that can replace them, from the learner's own attempts.
//!
//! Run it against a study directory:
//!
//! ```text
//! pnpm run analyse-attempts                 # the app's own data directory
//! pnpm run analyse-attempts -- /some/dir    # or a specific one
//! ```
//!
//! It reads the database directly and writes nothing, so it is safe to run
//! against real data — and it is deliberately the only one of the app's tools
//! that never touches the network, because the data never leaves the machine.

use std::path::PathBuf;

use hanzi_store::analyse::{analyse, AttemptAnalysis, MeasureStats, PERCENTILES};
use hanzi_store::Db;

fn main() {
    // Handled here rather than by returning from `main`, which would print a
    // `String` error through `Debug`: quoted, with its newlines escaped, which is
    // the wrong shape for a message someone has to read and act on.
    if let Err(error) = run() {
        eprintln!("\n{error}\n");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let dir = data_dir()?;
    let db_path = dir.join("hanzi.db");
    if !db_path.exists() {
        return Err(format!(
            "no study database at {}\n\
             Pass a directory as the first argument, or set HANZI_TUTOR_DATA_DIR.",
            db_path.display()
        )
        .into());
    }

    let db = Db::open(&dir)?;
    let attempts = db.attempts(None)?;
    let report = analyse(&attempts);

    println!("\nPractice log: {}", db_path.display());
    println!(
        "{} attempts on {} characters; {} carry grading measures{}\n",
        report.attempts,
        report.characters,
        report.measured,
        if report.attempts > report.measured {
            format!(
                " ({} unmeasured: recorded before schema 5, or merged from another device)",
                report.attempts - report.measured
            )
        } else {
            String::new()
        }
    );

    if report.measured == 0 {
        println!(
            "Nothing to analyse yet: no attempt in this log carries the measures it was graded\n\
             from. They are written from the moment this build grades an attempt, so practise a\n\
             few characters and run this again."
        );
        return Ok(());
    }

    print_measures(&report);
    print_verdicts(&report);
    print_separation(&report);
    print_integrity(&report);
    print_what_to_look_at();

    Ok(())
}

/// Where the study database lives.
///
/// The first argument wins, then `HANZI_TUTOR_DATA_DIR` — the same override the
/// app itself honours — and otherwise the platform's application data directory,
/// assembled the way the app's own default resolves.
fn data_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(arg) = std::env::args().nth(1) {
        return Ok(PathBuf::from(arg));
    }
    if let Ok(dir) = std::env::var("HANZI_TUTOR_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))?;
    let base = if cfg!(target_os = "macos") {
        PathBuf::from(home).join("Library/Application Support")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").unwrap_or(home))
    } else {
        PathBuf::from(home).join(".local/share")
    };
    Ok(base.join("com.hanzitutor.app"))
}

/// One measure per line, with the percentiles a shape should be read against.
fn print_measures(report: &AttemptAnalysis) {
    println!("Each measure over the measured attempts (0..=1):");
    println!(
        "  {:<13} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}   {:<5} {:>6} {:>6}",
        "", "mean", "p5", "p25", "p50", "p75", "p95", "bar", "above", "near"
    );
    for (name, stats) in measures(report) {
        let bar = stats
            .bar
            .map(|b| format!("{b:.2}"))
            .unwrap_or_else(|| "—".into());
        let above = stats
            .at_or_above_bar
            .map(|v| format!("{:.0}%", v * 100.0))
            .unwrap_or_else(|| "—".into());
        let near = stats
            .near_bar
            .map(|v| format!("{:.0}%", v * 100.0))
            .unwrap_or_else(|| "—".into());
        print!("  {name:<13} {:>6.2}", stats.mean);
        for value in stats.percentiles {
            print!(" {value:>6.2}");
        }
        println!("   {bar:<5} {above:>6} {near:>6}");
    }
    println!(
        "  (bar = the grader's own threshold; near = within 0.10 of it, the attempts it decides)\n"
    );
}

/// The grader's own verdicts, which are not ground truth but do bound the work.
fn print_verdicts(report: &AttemptAnalysis) {
    println!(
        "The grader's own verdicts: legible {}%, correct order {}%",
        (report.legible_rate * 100.0).round(),
        (report.order_correct_rate * 100.0).round()
    );
}

/// Whether each measure actually separates passes from failures.
fn print_separation(report: &AttemptAnalysis) {
    let (Some(pass), Some(fail)) = (report.pass_means, report.failure_means) else {
        println!("\nEvery attempt in this log has the same rating, so there is nothing to compare.\n");
        return;
    };
    println!("\nMean measure for attempts that passed and failed (again / hard):");
    println!(
        "  {:<13} {:>8} {:>8} {:>8}",
        "", "passed", "failed", "gap"
    );
    for (name, pass, fail) in [
        ("shape", pass.shape, fail.shape),
        ("position", pass.position, fail.position),
        ("ink", pass.ink, fail.ink),
        ("ink_coverage", pass.ink_coverage, fail.ink_coverage),
        ("order", pass.order, fail.order),
    ] {
        println!(
            "  {name:<13} {pass:>8.2} {fail:>8.2} {:>8.2}",
            pass - fail
        );
    }
    println!(
        "  (a gap near zero means the score could drop this measure and lose nothing)\n"
    );
}

/// The check that the stored measures are the ones that produced the score.
fn print_integrity(report: &AttemptAnalysis) {
    match report.score_deviation {
        Some((mean, max)) => println!(
            "Recorded score vs the four weights applied to the recorded measures:\n  \
             mean {mean:.4} points off, worst {max:.4}\n  \
             (near zero means the measures are the ones the score came from; if it is not,\n  \
             every figure above is suspect)\n"
        ),
        None => println!("No measured attempt, so the score cannot be rechecked.\n"),
    }
}

/// Print what the numbers above are for, so a reader does not have to hold the
/// tuning question in their head.
fn print_what_to_look_at() {
    println!("What to look at:");
    println!("  · a bar with a large 'near' share is deciding many attempts — moving it moves");
    println!("    a lot of verdicts, so it is worth getting right first");
    println!("  · a measure pinned at 1.00 is not measuring anything; `ink_coverage` was meant");
    println!("    to be the 'you never drew that part' signal, and cannot be if it never moves");
    println!("  · a measure whose passed/failed gap is near zero is not earning its quarter of");
    println!("    the score, whatever the weights say");
    println!(
        "  · percentiles are {}\n",
        PERCENTILES
            .iter()
            .map(|(_, name)| *name)
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// The measures with their names, in the order the report prints them.
fn measures(report: &AttemptAnalysis) -> [(&'static str, &MeasureStats); 5] {
    [
        ("shape", &report.shape),
        ("position", &report.position),
        ("ink", &report.ink),
        ("ink_coverage", &report.ink_coverage),
        ("order", &report.order),
    ]
}
