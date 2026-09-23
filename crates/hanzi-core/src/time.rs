//! ISO-8601 UTC timestamps, and just enough date arithmetic to schedule reviews.
//!
//! Study data is stored as `YYYY-MM-DDTHH:MM:SSZ` *text* rather than as a number
//! for two reasons: the files stay readable and hand-editable, and UTC timestamps
//! in this fixed-width format sort chronologically as plain text. That makes "is
//! this due?" and "which one is most overdue?" into string comparisons, which is
//! what [`crate::progress`] relies on.
//!
//! Only formatting, parsing and whole-second arithmetic are needed — the app
//! never has to ask what day of the week it is — so this is implemented directly
//! rather than pulling in a date library.

use std::time::{SystemTime, UNIX_EPOCH};

/// The last instant the fixed-width format can spell: `9999-12-31T23:59:59Z`.
///
/// The format is four digits wide by design, so a time outside `0..=MAX_UNIX`
/// cannot be written in a way this module could read back. [`add_seconds`]
/// refuses rather than writing a year the parser would reject.
pub const MAX_UNIX: i64 = 253_402_300_799;

/// The current time as an ISO-8601 UTC string, e.g. `2026-09-19T00:12:34Z`.
pub fn now_iso8601() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    iso8601_from_unix(seconds)
}

/// Format a Unix timestamp as ISO-8601 UTC.
pub fn iso8601_from_unix(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let secs_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second) = (
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60,
    );
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Parse an ISO-8601 UTC string back to a Unix timestamp.
///
/// Accepts exactly what [`iso8601_from_unix`] produces, and nothing else: a
/// timestamp the app wrote can always be read back, anything else is rejected
/// rather than guessed at, and any string this accepts is one
/// [`iso8601_from_unix`] formats back to *itself*. A date that does not exist,
/// such as `2026-02-30`, is rejected too, because the civil round-trip would
/// otherwise silently normalise it to March.
///
/// The round-trip property is the reason every field is parsed as a fixed-width
/// run of **ASCII digits** rather than with `str::parse`, which accepts a sign:
/// `2026-09-19T-1:-1:-1Z` is twenty characters with the delimiters in the right
/// places, so an `i64` parse reads the negative fields, every upper-bound check
/// passes, and the string parses to a *different instant* on the previous day.
/// The same parse accepts `+123-09-19T00:00:00Z`, whose negative Unix time
/// [`add_seconds`] refuses — a card carrying it could never advance. Requiring a
/// digit in every numeric position rejects both, and every other `i64`-parseable
/// spelling, in one go.
pub fn parse_iso8601(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    // A fixed-width run of digits, so no field can carry a sign, a space or any
    // other spelling the formatter would never write.
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let field = text.get(range)?;
        if !field.as_bytes().iter().all(u8::is_ascii_digit) {
            return None;
        }
        field.parse::<i64>().ok()
    };
    let year = number(0..4)?;
    let month = number(5..7)?;
    let day = number(8..10)?;
    let hour = number(11..13)?;
    let minute = number(14..16)?;
    let second = number(17..19)?;
    // Both ends of every field, so nothing outside the formatter's range can be
    // read back: four digits hold 0..=9999, and the clock is a real one.
    if !(0..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }
    let days = days_from_civil(year, month as u32, day as u32);
    // Reject a date the calendar does not have: re-deriving it from the day
    // count is the cheapest exact check.
    if civil_from_days(days) != (year, month as u32, day as u32) {
        return None;
    }
    Some(days * 86_400 + hour * 3600 + minute * 60 + second)
}

/// `seconds` after the instant `from`, as an ISO-8601 UTC string.
///
/// `None` when `from` cannot be parsed or the result leaves the representable
/// range; the caller decides what a bad timestamp means. A returned string is
/// always something [`parse_iso8601`] can read back.
pub fn add_seconds(from: &str, seconds: i64) -> Option<String> {
    let unix = parse_iso8601(from)?.saturating_add(seconds);
    if !(0..=MAX_UNIX).contains(&unix) {
        return None;
    }
    Some(iso8601_from_unix(unix))
}

/// Convert a count of days since the Unix epoch to a civil date.
///
/// Howard Hinnant's `civil_from_days`, which is exact for the whole range of
/// `i64` days.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Convert a civil date to a count of days since the Unix epoch.
///
/// The inverse of [`civil_from_days`], also Hinnant's.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = (year - era * 400) as u64; // [0, 399]
    let mp = if month > 2 { month - 3 } else { month + 9 } as u64; // [0, 11]
    let doy = (153 * mp + 2) / 5 + day as u64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe as i64 - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_timestamps() {
        assert_eq!(iso8601_from_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601_from_unix(946_684_800), "2000-01-01T00:00:00Z");
        assert_eq!(iso8601_from_unix(1_234_567_890), "2009-02-13T23:31:30Z");
        // 2000 was a leap year, so this date exists.
        assert_eq!(iso8601_from_unix(951_782_400), "2000-02-29T00:00:00Z");
        // And a time of day, to check the division.
        assert_eq!(iso8601_from_unix(86_399), "1970-01-01T23:59:59Z");
    }

    #[test]
    fn parses_what_it_formats() {
        for unix in [
            0,
            1,
            86_399,
            946_684_800,
            951_782_400,
            1_234_567_890,
            1_700_000_000,
            4_102_444_800, // 2100-01-01, past the next non-leap century
        ] {
            let text = iso8601_from_unix(unix);
            assert_eq!(parse_iso8601(&text), Some(unix), "round trip of {text}");
        }
        assert_eq!(parse_iso8601("2026-09-19T00:12:34Z"), Some(1_789_776_754));
    }

    #[test]
    fn rejects_anything_it_did_not_write() {
        for bad in [
            "",
            "not a timestamp",
            "2026-09-19T00:12:34",       // no zone
            "2026-09-19 00:12:34Z",      // space instead of T
            "2026-09-19T00:12:34+01:00", // offsets are not accepted
            "2026-9-19T00:12:34Z",       // unpadded
            "2026-13-01T00:12:34Z",      // no such month
            "2026-02-30T00:12:34Z",      // no such day
            "2026-09-19T24:00:00Z",      // no such hour
            "2026-09-19T00:60:00Z",      // no such minute
            "2026-09-19T00:12:60Z",      // no such second
            "2026-09-19T-1:-1:-1Z",      // signed fields; parsed to the day before
            "+123-09-19T00:00:00Z",      // a signed year, and before the epoch
            "-001-09-19T00:00:00Z",      // the same, spelled the other way
            "2026-09-19T+1:00:00Z",      // a signed hour
            "2026-09-19T 1:00:00Z",      // a blank where a digit belongs
            "2026-09-19T01:02:0 Z",      // and the same in the seconds
            "２０２６-09-19T00:00:00Z", // full-width digits are not ASCII digits
        ] {
            assert_eq!(parse_iso8601(bad), None, "{bad:?} should be rejected");
        }
    }

    #[test]
    fn anything_it_accepts_is_an_instant_it_would_have_written() {
        // The round-trip property stated forwards, as one predicate rather than
        // a list of cases: a parser that answers `Some` has understood the text
        // completely, and `iso8601_from_unix` gives that understanding back.
        fn means_exactly_one_instant(text: &str) -> bool {
            match parse_iso8601(text) {
                Some(unix) => iso8601_from_unix(unix) == text,
                None => true,
            }
        }

        // The inverse direction of the round trip, over both ends of every
        // field. An accepted string that reformats as something else is the bug
        // this check exists for: `self.due.as_str() <= now` in `CardState::is_due`
        // assumes the text the parser read back means exactly one instant.
        for good in [
            "0000-01-01T00:00:00Z", // the first instant the width can spell
            "0000-12-31T23:59:59Z",
            "0001-01-01T00:00:00Z",
            "1970-01-01T00:00:00Z",
            "2026-09-19T00:12:34Z",
            "9999-12-31T23:59:59Z", // and the last
        ] {
            let unix = parse_iso8601(good).unwrap_or_else(|| panic!("{good} should parse"));
            assert_eq!(iso8601_from_unix(unix), good, "{good} should reformat as itself");
        }
        // The last instant is `MAX_UNIX`, so the accepted range and the
        // representable one are the same range.
        assert_eq!(parse_iso8601("9999-12-31T23:59:59Z"), Some(MAX_UNIX));

        // The two spells an `i64` parse let through, and the reason the rule is
        // "reject or round-trip" rather than "reject these fields": both are
        // twenty characters with the delimiters in the right places, so nothing
        // but the fields themselves can separate them from a real timestamp.
        // The first parses to the *day before* — a different instant that the
        // schedule compares as text; the second parses, but to a negative Unix
        // time [`add_seconds`] will not move, so its card is due for ever.
        assert!(means_exactly_one_instant("2026-09-19T-1:-1:-1Z"));
        assert!(means_exactly_one_instant("+123-09-19T00:00:00Z"));
    }

    #[test]
    fn timestamps_sort_chronologically_as_strings() {
        let earlier = iso8601_from_unix(1_000_000_000);
        let later = iso8601_from_unix(1_700_000_000);
        assert!(earlier < later, "{earlier} should sort before {later}");

        // Across a year boundary, where a naive format would break.
        let new_year = iso8601_from_unix(1_704_067_200); // 2024-01-01
        let old_year = iso8601_from_unix(1_701_000_000); // 2023-11-24
        assert!(old_year < new_year);
    }

    #[test]
    fn now_is_plausible() {
        let now = now_iso8601();
        assert_eq!(now.len(), 20, "unexpected format: {now}");
        assert!(now.starts_with("20"), "unexpected year: {now}");
        assert!(now.ends_with('Z'));
        assert!(parse_iso8601(&now).is_some());
    }

    #[test]
    fn adds_seconds_across_calendar_boundaries() {
        let base = "2026-12-31T23:59:30Z";
        assert_eq!(
            add_seconds(base, 30).unwrap(),
            "2027-01-01T00:00:00Z",
            "a new year"
        );
        assert_eq!(
            add_seconds("2024-02-28T12:00:00Z", 86_400).unwrap(),
            "2024-02-29T12:00:00Z",
            "a leap day"
        );
        assert_eq!(
            add_seconds("2023-02-28T12:00:00Z", 86_400).unwrap(),
            "2023-03-01T12:00:00Z",
            "and a non-leap year"
        );
        // A day is a day: scheduling a review by seconds must not drift.
        assert_eq!(
            add_seconds("2026-09-19T00:12:34Z", 86_400).unwrap(),
            "2026-09-20T00:12:34Z"
        );
        assert_eq!(add_seconds("nonsense", 60), None);
    }

    #[test]
    fn adds_seconds_only_within_the_representable_range() {
        // The last instant the four-digit-year format can spell.
        assert_eq!(iso8601_from_unix(MAX_UNIX), "9999-12-31T23:59:59Z");
        assert_eq!(add_seconds("9999-12-31T23:59:58Z", 1).unwrap(), "9999-12-31T23:59:59Z");
        // Anything past it is refused rather than formatted as a five-digit year
        // the parser could not read back.
        assert_eq!(add_seconds("9999-12-31T23:59:59Z", 1), None);
        assert_eq!(add_seconds("1970-01-01T00:00:00Z", -1), None);
        assert_eq!(add_seconds("1970-01-01T00:00:00Z", i64::MAX), None);
    }
}
