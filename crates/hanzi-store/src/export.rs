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
//!
//! Both carry the attempts written over the faint guide as well as the reviews,
//! each row saying which it was in a `mode` field (`recall` or `trace`). A trace
//! is not in the review log and has no device or sequence number — it never
//! leaves the device that made it — so those two fields are absent on its rows.
//! It is worth exporting all the same: the board starts in trace mode, so for a
//! beginner most measured attempts are traces. They are graded where the guide
//! is rather than with the global fit, so a reader tuning the grader should look
//! at the two modes separately.

use hanzi_core::vocab::csv_field;
use hanzi_core::{AttemptMeasures, PracticeMode};
use serde::{Deserialize, Serialize};

use crate::{LoggedAttempt, TracedAttempt};

/// One attempt as the export writes it.
///
/// A deliberate copy of the storage row rather than `Serialize` on
/// [`LoggedAttempt`] itself: this is a format other programs read, so it should
/// change when someone decides it should rather than whenever the table does.
///
/// It is owned, and reads as well as writes, because it is also the attempt's
/// shape inside the combined backup (see [`crate::backup`]) — which has to be
/// able to read back exactly what the JSON Lines export writes. Two types for
/// one format would be two chances for them to disagree.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedAttempt {
    pub ch: String,
    pub at: String,
    pub score: f32,
    /// `again`, `hard`, `good` or `easy`.
    pub rating: String,
    /// Which device made it, and its number in that device's own log. Carried
    /// because the log read here is the union of every device's, so an attempt
    /// has to keep saying where it came from.
    pub device_id: String,
    pub seq: i64,
    /// Absent when nothing measured this attempt: it predates schema 5, or it
    /// arrived from a peer, whose shard carries the score and the time only.
    /// Absent rather than zero, because a measured zero is a verdict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measures: Option<AttemptMeasures>,
}

impl From<&LoggedAttempt> for ExportedAttempt {
    fn from(attempt: &LoggedAttempt) -> Self {
        Self {
            ch: attempt.ch.clone(),
            at: attempt.at.clone(),
            score: attempt.score,
            rating: attempt.rating.clone(),
            device_id: attempt.device_id.clone(),
            seq: attempt.seq,
            measures: attempt.measures,
        }
    }
}

/// One traced attempt as the backup carries it.
///
/// The same facts as an [`ExportedAttempt`] without the device and sequence
/// number, which a trace does not have: it is never published, so nothing has to
/// name it anywhere else.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedTrace {
    pub ch: String,
    pub at: String,
    pub score: f32,
    /// `again`, `hard`, `good` or `easy`: what the score would have meant as a
    /// review.
    pub rating: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measures: Option<AttemptMeasures>,
}

impl From<&TracedAttempt> for ExportedTrace {
    fn from(trace: &TracedAttempt) -> Self {
        Self {
            ch: trace.ch.clone(),
            at: trace.at.clone(),
            score: trace.score,
            rating: trace.rating.clone(),
            measures: trace.measures,
        }
    }
}

/// One line of the practice-log export: a review or a trace, saying which.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LogLine<'a> {
    ch: &'a str,
    at: &'a str,
    score: f32,
    rating: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seq: Option<i64>,
    mode: PracticeMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    measures: Option<AttemptMeasures>,
}

impl<'a> From<&'a LoggedAttempt> for LogLine<'a> {
    fn from(attempt: &'a LoggedAttempt) -> Self {
        Self {
            ch: &attempt.ch,
            at: &attempt.at,
            score: attempt.score,
            rating: &attempt.rating,
            device_id: Some(&attempt.device_id),
            seq: Some(attempt.seq),
            mode: PracticeMode::Recall,
            measures: attempt.measures,
        }
    }
}

impl<'a> From<&'a TracedAttempt> for LogLine<'a> {
    fn from(trace: &'a TracedAttempt) -> Self {
        Self {
            ch: &trace.ch,
            at: &trace.at,
            score: trace.score,
            rating: &trace.rating,
            device_id: None,
            seq: None,
            mode: PracticeMode::Trace,
            measures: trace.measures,
        }
    }
}

/// The reviews and the traces as one timeline, oldest first.
///
/// Each list is already in its own order, so this is a merge by time rather than
/// a sort; on a tie the review comes first, which keeps the output stable.
fn timeline<'a>(attempts: &'a [LoggedAttempt], traces: &'a [TracedAttempt]) -> Vec<LogLine<'a>> {
    let mut out = Vec::with_capacity(attempts.len() + traces.len());
    let (mut a, mut t) = (attempts.iter().peekable(), traces.iter().peekable());
    loop {
        let take_trace = match (a.peek(), t.peek()) {
            (Some(review), Some(trace)) => trace.at < review.at,
            (Some(_), None) => false,
            (None, Some(_)) => true,
            (None, None) => break,
        };
        if take_trace {
            out.extend(t.next().map(LogLine::from));
        } else {
            out.extend(a.next().map(LogLine::from));
        }
    }
    out
}

/// The attempts and traces as JSON Lines: one object per line, oldest first.
///
/// JSON Lines rather than one array, because the log's virtue is that it is
/// append-only and unbounded — a stream of independent objects is what that
/// looks like on disk, and it can be read a line at a time.
pub fn attempts_to_jsonl(
    attempts: &[LoggedAttempt],
    traces: &[TracedAttempt],
) -> Result<String, serde_json::Error> {
    let mut out = String::new();
    for line in timeline(attempts, traces) {
        out.push_str(&serde_json::to_string(&line)?);
        out.push('\n');
    }
    Ok(out)
}

/// The columns of the CSV export, in order.
///
/// `mode` is last because it was added last: a reader that picks columns by
/// position keeps finding every older column where it was.
const CSV_HEADER: &str = "ch,at,score,rating,device_id,seq,shape,position,ink,\
                          ink_coverage,order_score,legible,order_correct,mode";

/// The attempts and traces as CSV: a header row, then one row each, oldest first.
///
/// Lossy on purpose, and by the same rule as the vocabulary list's CSV export:
/// there is no matching import. The format that round-trips is the combined
/// backup ([`crate::backup`]), not either of these single-purpose exports. A
/// missing measure is an empty field, which is how a spreadsheet spells "not
/// measured" — and so is a trace's device and sequence number, which it does not
/// have.
pub fn attempts_to_csv(attempts: &[LoggedAttempt], traces: &[TracedAttempt]) -> String {
    let mut out = String::from(CSV_HEADER);
    out.push('\n');
    for line in timeline(attempts, traces) {
        let m = line.measures;
        let fields = [
            line.ch.to_string(),
            line.at.to_string(),
            format!("{}", line.score),
            line.rating.to_string(),
            line.device_id.unwrap_or_default().to_string(),
            line.seq.map(|seq| seq.to_string()).unwrap_or_default(),
            opt(m.map(|m| m.shape)),
            opt(m.map(|m| m.position)),
            opt(m.map(|m| m.ink)),
            opt(m.map(|m| m.ink_coverage)),
            opt(m.map(|m| m.order)),
            m.map(|m| m.legible.to_string()).unwrap_or_default(),
            m.map(|m| m.order_correct.to_string()).unwrap_or_default(),
            line.mode.name().to_string(),
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
        let jsonl = attempts_to_jsonl(&rows, &[]).unwrap();
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
        let csv = attempts_to_csv(&rows, &[]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3, "a header and one row per attempt");
        assert_eq!(
            lines[0].split(',').count(),
            lines[1].split(',').count(),
            "every row has as many fields as the header"
        );
        assert!(lines[1].contains("0.95"));
        assert!(lines[1].contains(",false,true"));

        // `学`'s row ends with five empty measures and two empty verdicts, then
        // the mode.
        assert!(
            lines[2].ends_with(",,,,,,,,recall"),
            "an unmeasured attempt is empty, not zeroed: {}",
            lines[2]
        );
    }

    #[test]
    fn a_character_that_would_break_the_csv_is_quoted() {
        // The log's `ch` is one character, but it can be a comma or a quote —
        // the vocabulary list can hold a sentence, and a character can be
        // punctuation. The same quoting rule as the list's export.
        let csv = attempts_to_csv(&[attempt(",", Some(measured()))], &[]);
        let row = csv.lines().nth(1).unwrap();
        assert!(row.starts_with("\",\""), "the comma is quoted: {row}");
    }

    #[test]
    fn a_field_a_spreadsheet_would_run_is_defused() {
        // The same safety boundary as the vocabulary list's export, through this
        // writer: an attempt's device id is a peer's, so it arrives from outside
        // the app, and a CSV is opened in a spreadsheet by design. Quoting alone
        // would not help — CSV quoting is stripped on load.
        let mut row = attempt("=", Some(measured()));
        row.device_id = "@SUM(1+1)".into();
        let csv = attempts_to_csv(&[row], &[]);
        let line = csv.lines().nth(1).unwrap();
        assert!(line.contains("'="), "the character is defused: {line}");
        assert!(line.contains("'@SUM(1+1)"), "the device is defused: {line}");

        // An ordinary attempt is untouched, marker and all.
        let plain = attempts_to_csv(&[attempt("好", Some(measured()))], &[]);
        assert!(plain.lines().nth(1).unwrap().starts_with("好,"));
    }

    fn trace(ch: &str, at: &str, measures: Option<AttemptMeasures>) -> TracedAttempt {
        TracedAttempt {
            id: 1,
            ch: ch.into(),
            at: at.into(),
            score: 97.0,
            rating: "easy".into(),
            measures,
        }
    }

    #[test]
    fn traces_are_exported_in_time_order_and_say_they_are_traces() {
        // The board starts in trace mode, so for a beginner most measured attempts
        // are traces; an export that left them out lost most of the evidence.
        let rows = vec![attempt("好", Some(measured()))]; // 09:00
        let traces = vec![
            trace("学", "2026-09-22T08:59:00Z", Some(measured())),
            trace("习", "2026-09-22T09:01:00Z", None),
        ];

        let jsonl = attempts_to_jsonl(&rows, &traces).unwrap();
        let lines: Vec<serde_json::Value> = jsonl
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let order: Vec<(&str, &str)> = lines
            .iter()
            .map(|l| (l["ch"].as_str().unwrap(), l["mode"].as_str().unwrap()))
            .collect();
        assert_eq!(
            order,
            vec![("学", "trace"), ("好", "recall"), ("习", "trace")]
        );
        assert!(lines[0].get("deviceId").is_none(), "a trace has no device");
        assert!(lines[0].get("seq").is_none());
        assert_eq!(lines[0]["measures"]["inkCoverage"], 0.95);
        assert_eq!(lines[1]["deviceId"], "device-a");

        let csv = attempts_to_csv(&rows, &traces);
        let csv: Vec<&str> = csv.lines().collect();
        assert!(csv[0].starts_with("ch,at,score,rating,device_id,seq,shape,"));
        assert!(
            csv[0].ends_with(",order_correct,mode"),
            "mode is the last column"
        );
        assert!(
            csv[1].starts_with("学,2026-09-22T08:59:00Z,97,easy,,,0.82")
                && csv[1].ends_with(",trace"),
            "{}",
            csv[1]
        );
        assert!(csv[2].contains(",device-a,7,0.82,"), "{}", csv[2]);
        assert!(csv[2].ends_with(",recall"), "{}", csv[2]);
        assert!(csv[3].ends_with(",,,,,,,,,trace"), "{}", csv[3]);
        for line in &csv {
            assert_eq!(line.split(',').count(), csv[0].split(',').count());
        }
    }
}
