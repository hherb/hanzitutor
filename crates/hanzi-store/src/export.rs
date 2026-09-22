//! Getting the attempt log out of the database.
//!
//! The log is worth more than the schedule it feeds. It is the only record of
//! what a learner actually wrote, and since schema 5 it also carries the measures
//! each attempt was graded from — which is the thing the grading tolerances have
//! to be checked against, because the strokes themselves are not kept.
//!
//! Two formats for two readers: JSON Lines for anything that wants the whole
//! record back, CSV for a spreadsheet or a statistics package. Both are pure
//! functions over rows, so the shape of the output is testable without a
//! database, and neither invents a value for a measure that was never taken.

use hanzi_core::vocab::csv_field;
use hanzi_core::AttemptMeasures;
use serde::Serialize;

use crate::LoggedAttempt;

/// One attempt as the export writes it.
///
/// A deliberate copy of the storage row rather than `Serialize` on
/// [`LoggedAttempt`] itself: this is a format other programs read, so it should
/// change when someone decides it should rather than whenever the table does.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportedAttempt<'a> {
    ch: &'a str,
    at: &'a str,
    score: f32,
    /// `again`, `hard`, `good` or `easy`.
    rating: &'a str,
    /// Which device made it, and its number in that device's own log. Carried
    /// because the log read here is the union of every device's, so an attempt
    /// has to keep saying where it came from.
    device_id: &'a str,
    seq: i64,
    /// Absent when nothing measured this attempt: it predates schema 5, or it
    /// arrived from a peer, whose shard carries the score and the time only.
    /// Absent rather than zero, because a measured zero is a verdict.
    #[serde(skip_serializing_if = "Option::is_none")]
    measures: Option<AttemptMeasures>,
}

/// The attempts as JSON Lines: one object per line, in the order given.
///
/// JSON Lines rather than one array, because the log's virtue is that it is
/// append-only and unbounded — a stream of independent objects is what that
/// looks like on disk, and it can be read a line at a time.
pub fn attempts_to_jsonl(attempts: &[LoggedAttempt]) -> Result<String, serde_json::Error> {
    let mut out = String::new();
    for attempt in attempts {
        let row = ExportedAttempt {
            ch: &attempt.ch,
            at: &attempt.at,
            score: attempt.score,
            rating: &attempt.rating,
            device_id: &attempt.device_id,
            seq: attempt.seq,
            measures: attempt.measures,
        };
        out.push_str(&serde_json::to_string(&row)?);
        out.push('\n');
    }
    Ok(out)
}

/// The columns of the CSV export, in order.
const CSV_HEADER: &str = "ch,at,score,rating,device_id,seq,shape,position,ink,\
                          ink_coverage,order_score,legible,order_correct";

/// The attempts as CSV: a header row, then one row per attempt.
///
/// Lossy on purpose, and by the same rule as the vocabulary list's CSV export:
/// there is no matching import, because JSON Lines is the format that round
/// trips. A missing measure is an empty field, which is how a spreadsheet spells
/// "not measured".
pub fn attempts_to_csv(attempts: &[LoggedAttempt]) -> String {
    let mut out = String::from(CSV_HEADER);
    out.push('\n');
    for attempt in attempts {
        let m = attempt.measures;
        let fields = [
            attempt.ch.clone(),
            attempt.at.clone(),
            format!("{}", attempt.score),
            attempt.rating.clone(),
            attempt.device_id.clone(),
            attempt.seq.to_string(),
            opt(m.map(|m| m.shape)),
            opt(m.map(|m| m.position)),
            opt(m.map(|m| m.ink)),
            opt(m.map(|m| m.ink_coverage)),
            opt(m.map(|m| m.order)),
            m.map(|m| m.legible.to_string()).unwrap_or_default(),
            m.map(|m| m.order_correct.to_string()).unwrap_or_default(),
        ];
        let row: Vec<String> = fields.iter().map(|f| csv_field(f)).collect();
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

/// A measure as a CSV field, or nothing at all when it was never taken.
fn opt(value: Option<f32>) -> String {
    value.map(|v| format!("{v}")).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(ch: &str, measures: Option<AttemptMeasures>) -> LoggedAttempt {
        LoggedAttempt {
            id: 1,
            device_id: "device-a".into(),
            seq: 7,
            ch: ch.into(),
            at: "2026-09-22T09:00:00Z".into(),
            score: 63.0,
            rating: "hard".into(),
            measures,
        }
    }

    fn measured() -> AttemptMeasures {
        AttemptMeasures {
            shape: 0.82,
            position: 0.71,
            ink: 0.33,
            ink_coverage: 0.95,
            order: 1.0,
            legible: false,
            order_correct: true,
        }
    }

    #[test]
    fn jsonl_is_one_object_per_attempt_with_the_measures_nested() {
        let rows = vec![attempt("好", Some(measured())), attempt("学", None)];
        let jsonl = attempts_to_jsonl(&rows).unwrap();
        assert_eq!(jsonl.lines().count(), 2, "one line per attempt");

        let first: serde_json::Value = serde_json::from_str(jsonl.lines().next().unwrap()).unwrap();
        assert_eq!(first["ch"], "好");
        assert_eq!(first["rating"], "hard");
        assert_eq!(first["seq"], 7);
        assert_eq!(first["measures"]["inkCoverage"], 0.95);
        assert_eq!(first["measures"]["orderCorrect"], true);

        // The unmeasured attempt says nothing about measures rather than saying
        // they were zero.
        let second: serde_json::Value = serde_json::from_str(jsonl.lines().nth(1).unwrap()).unwrap();
        assert!(second.get("measures").is_none());
    }

    #[test]
    fn csv_has_a_header_and_leaves_a_missing_measure_empty() {
        let rows = vec![attempt("好", Some(measured())), attempt("学", None)];
        let csv = attempts_to_csv(&rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3, "a header and one row per attempt");
        assert_eq!(
            lines[0].split(',').count(),
            lines[1].split(',').count(),
            "every row has as many fields as the header"
        );
        assert!(lines[1].contains("0.95"));
        assert!(lines[1].contains(",false,true"));

        // `学`'s row ends with five empty measures and two empty verdicts.
        assert!(
            lines[2].ends_with(",,,,,,,"),
            "an unmeasured attempt is empty, not zeroed: {}",
            lines[2]
        );
    }

    #[test]
    fn a_character_that_would_break_the_csv_is_quoted() {
        // The log's `ch` is one character, but it can be a comma or a quote —
        // the vocabulary list can hold a sentence, and a character can be
        // punctuation. The same quoting rule as the list's export.
        let csv = attempts_to_csv(&[attempt(",", Some(measured()))]);
        let row = csv.lines().nth(1).unwrap();
        assert!(row.starts_with("\",\""), "the comma is quoted: {row}");
    }
}
