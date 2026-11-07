//! The clock that stamps a learner's edits: wall time, never behind what was seen.
//!
//! Every record that syncs by last-writer-wins — an entry, a group, the course
//! position, a group's position — carries an `updated_at` that is compared as text
//! with a peer's. The wall clock alone is a poor judge of "later": a device whose
//! clock runs an hour fast stamps every edit an hour into the future, and until real
//! time catches up each of its writes beats every honest device's, *including edits
//! the honest device made after it had already seen the fast one's*. The learner
//! sees an edit they just made quietly undone by the next sync.
//!
//! So a stamp is `max(now, floor + 1 s)`, where the floor is kept in `meta` and is
//! the latest stamp this device has **seen or issued**:
//!
//! - **Every stamp a peer wrote raises it**, whenever one is seen: applying a merged
//!   view hands every winning stamp to [`observe`]. An edit made after that is then
//!   strictly later than anything this device has seen, which is what "later" has
//!   to mean for a learner: the edit they made last wins, whatever either clock says.
//! - **Every stamp this device issues raises it**, so its stamps strictly increase.
//!   That is what makes this device's next write beat anything it already holds —
//!   including a record the merge derived *at* one of its own stamps, such as the
//!   tombstone of a collapsed duplicate (`hanzi_sync::merge_vocab`) — and it also
//!   means a clock that steps backwards (an NTP correction, a dead RTC battery)
//!   cannot make a newer local edit lose to an older one. The cost is that a burst
//!   of writes inside one second runs a few seconds ahead of real time, until real
//!   time catches up.
//!
//! ## A peer whose clock is badly wrong
//!
//! A stamp more than a day ahead of this device's own clock is **not** taken into
//! the floor. Taking it would pin every stamp this device ever writes to that
//! future — and every peer's after it, since they would see this device's stamps
//! in turn — so one tablet with a dead clock battery stamping 2099 would leave the
//! whole set of devices writing 2099 for good. A day is far more skew than an honest
//! clock has, and less than a far-future one. Such a record still has to be
//! editable here, so an edit of a *particular* record is also stamped past that
//! record's own stamp — see `past_sql!` and [`next_stamp_past`].
//!
//! It is a hybrid logical clock with the logical part folded into the seconds,
//! because the stored format is fixed: whole seconds, ISO-8601 UTC, compared as
//! plain strings (HANDOVER invariant 12). Adding a second keeps that format exactly,
//! so every comparison already in the code stays correct and older builds read the
//! stamps unchanged.
//!
//! The floor is in `meta` rather than in memory so that it survives a restart: a
//! device that saw a fast peer's stamp yesterday must still beat it today.

use hanzi_core::time::{add_seconds, iso8601_from_unix, now_iso8601, parse_iso8601};
use rusqlite::Connection;

use crate::{meta_get, set_meta};

/// Where the floor lives in `meta`.
pub(crate) const CLOCK_KEY: &str = "sync:clock";

/// How far ahead of this device's clock a peer's stamp may be and still move the
/// floor — see the module note.
const MAX_AHEAD_SECONDS: i64 = 24 * 60 * 60;

/// A SQL expression for the stamp to give a record this device is overwriting:
/// `?` (the stamp from [`next_stamp`]) or one second past the record's own
/// `updated_at`, whichever is later.
///
/// The second half only matters for a record stamped by a clock far enough ahead
/// that [`observe`] ignored it: without it, an edit made here would lose to the
/// record it replaced until real time reached that clock. `strftime` writes the
/// same fixed-width format the rest of the code compares as text, and gives NULL
/// for an empty or unreadable stamp, which falls back to the new stamp.
///
/// `new` is the new stamp's SQL (a parameter, or `excluded.updated_at`), `old` the
/// column holding the record's current one. A macro rather than a function so the
/// statements that use it stay string literals.
macro_rules! past_sql {
    ($new:literal, $old:literal) => {
        concat!(
            "MAX(",
            $new,
            ", IFNULL(strftime('%Y-%m-%dT%H:%M:%SZ', ",
            $old,
            ", '+1 second'), ",
            $new,
            "))"
        )
    };
}

/// The stamp for a write this device is making now, and the floor moved to it.
///
/// Strictly later than every stamp this device has issued or seen, and never
/// earlier than its clock — except that a floor more than a day ahead of the clock
/// is dropped. Only this device can have put it there (a peer's stamp that far
/// ahead is never taken in), by issuing stamps while its own clock was wrong; once
/// the clock is put right, going on from that floor would keep every stamp it ever
/// writes in that future. A record carrying such a stamp is still beaten by an edit
/// here, which is stamped past the record itself (`past_sql!`, [`next_stamp_past`]).
pub(crate) fn next_stamp(conn: &Connection) -> rusqlite::Result<String> {
    let now = now_iso8601();
    let limit = parse_iso8601(&now).unwrap_or(0) + MAX_AHEAD_SECONDS;
    let stamp = match floor(conn)? {
        Some(floor) if floor >= now && parse_iso8601(&floor).is_some_and(|at| at <= limit) => {
            add_seconds(&floor, 1).unwrap_or(floor)
        }
        _ => now,
    };
    set_meta(conn, CLOCK_KEY, &stamp)?;
    Ok(stamp)
}

/// A stamp for overwriting one record whose current stamp is `held`: the next
/// stamp, or one second past `held` if that is later — see `past_sql!`.
pub(crate) fn next_stamp_past(conn: &Connection, held: Option<&str>) -> rusqlite::Result<String> {
    let stamp = next_stamp(conn)?;
    let past = held
        .filter(|held| parse_iso8601(held).is_some())
        .and_then(|held| add_seconds(held, 1));
    Ok(match past {
        Some(past) if past > stamp => past,
        _ => stamp,
    })
}

/// Raise the floor to the latest stamp a *peer* wrote.
///
/// `stamps` is `(updated_at, device_id)` pairs. This device's own are skipped: they
/// are already at or below the floor. A stamp that does not parse — an empty one
/// from before stamps existed, a hand-edited one — says nothing about time and is
/// ignored, and so is one more than a day ahead of this device's clock (see the
/// module note).
///
/// `own_device` is whatever `Db::device_id` hands back, borrowed or owned.
pub(crate) fn observe<'a>(
    conn: &Connection,
    own_device: impl AsRef<str>,
    stamps: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> rusqlite::Result<()> {
    let own_device = own_device.as_ref();
    let limit = parse_iso8601(&now_iso8601()).unwrap_or(0) + MAX_AHEAD_SECONDS;
    let seen = stamps
        .into_iter()
        .filter(|(_, device)| *device != own_device)
        .filter(|(stamp, _)| parse_iso8601(stamp).is_some_and(|at| at <= limit))
        .map(|(stamp, _)| stamp)
        .max();
    let Some(seen) = seen else {
        return Ok(());
    };
    if floor(conn)?.is_some_and(|floor| floor.as_str() >= seen) {
        return Ok(());
    }
    set_meta(conn, CLOCK_KEY, seen)
}

/// The stored floor, if there is a readable one.
///
/// Normalised through the parser so that only a stamp in the fixed format is ever
/// compared as text against another.
fn floor(conn: &Connection) -> rusqlite::Result<Option<String>> {
    Ok(meta_get(conn, CLOCK_KEY)?
        .and_then(|text| parse_iso8601(&text))
        .map(iso8601_from_unix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
            .unwrap();
        conn
    }

    #[test]
    fn a_stamp_is_the_wall_clock_when_nothing_later_has_been_seen() {
        let conn = conn();
        let before = now_iso8601();
        let stamp = next_stamp(&conn).unwrap();
        assert!(stamp >= before && stamp <= now_iso8601(), "{stamp}");
        assert_eq!(stamp.len(), "2026-09-20T10:00:00Z".len(), "the fixed width");
    }

    #[test]
    fn a_peer_stamp_from_the_future_is_beaten_by_one_second() {
        let conn = conn();
        let ahead = add_seconds(&now_iso8601(), 3600).unwrap();
        observe(&conn, "me", [(ahead.as_str(), "fast")]).unwrap();
        let first = next_stamp(&conn).unwrap();
        assert_eq!(first, add_seconds(&ahead, 1).unwrap());
        // And every stamp after it is later again, so this device's next write
        // beats anything it already holds — its own last write included.
        assert_eq!(next_stamp(&conn).unwrap(), add_seconds(&ahead, 2).unwrap());
    }

    #[test]
    fn this_devices_own_stamps_and_unreadable_ones_do_not_move_the_floor() {
        let conn = conn();
        let ahead = add_seconds(&now_iso8601(), 3600).unwrap();
        observe(
            &conn,
            "me",
            [(ahead.as_str(), "me"), ("", "peer"), ("tomorrow", "peer")],
        )
        .unwrap();
        assert!(next_stamp(&conn).unwrap() <= now_iso8601());
    }

    #[test]
    fn an_older_peer_stamp_never_lowers_the_floor() {
        let conn = conn();
        let ahead = add_seconds(&now_iso8601(), 3600).unwrap();
        observe(&conn, "me", [(ahead.as_str(), "fast")]).unwrap();
        observe(&conn, "me", [("2000-01-01T10:00:00Z", "slow")]).unwrap();
        assert_eq!(next_stamp(&conn).unwrap(), add_seconds(&ahead, 1).unwrap());
    }

    #[test]
    fn a_peer_stamp_years_ahead_does_not_pin_this_devices_clock() {
        // One device with a dead clock battery stamping 2099 must not drag every
        // stamp this device writes, for ever, to 2099.
        let conn = conn();
        observe(&conn, "me", [("2099-01-01T00:00:00Z", "broken-rtc")]).unwrap();
        assert!(next_stamp(&conn).unwrap().as_str() < "2099");
        // But a record carrying such a stamp can still be overwritten here: an
        // edit of it is stamped past it.
        assert_eq!(
            next_stamp_past(&conn, Some("2099-01-01T00:00:00Z")).unwrap(),
            "2099-01-01T00:00:01Z"
        );
        assert!(next_stamp_past(&conn, Some("")).unwrap().as_str() < "2099");
    }

    #[test]
    fn a_floor_this_device_left_years_ahead_is_dropped_once_its_clock_is_right() {
        // A device whose clock said 2099 for a while issued 2099 stamps, and the
        // floor followed them. With the clock corrected, going on from there would
        // keep it writing 2099 for good.
        let conn = conn();
        set_meta(&conn, CLOCK_KEY, "2099-01-01T00:00:00Z").unwrap();
        let stamp = next_stamp(&conn).unwrap();
        assert!(stamp.as_str() < "2099", "{stamp}");
        // A floor inside the day is still honoured.
        let ahead = add_seconds(&now_iso8601(), 3600).unwrap();
        set_meta(&conn, CLOCK_KEY, &ahead).unwrap();
        assert_eq!(next_stamp(&conn).unwrap(), add_seconds(&ahead, 1).unwrap());
    }

    #[test]
    fn the_sql_form_is_the_later_of_the_two_in_the_same_format() {
        let conn = conn();
        let sql = concat!("SELECT ", past_sql!("?1", "?2"));
        let past = |new: &str, old: &str| -> String {
            conn.query_row(sql, [new, old], |row| row.get(0)).unwrap()
        };
        assert_eq!(
            past("2026-09-20T10:00:00Z", "2099-01-01T00:00:00Z"),
            "2099-01-01T00:00:01Z"
        );
        assert_eq!(
            past("2026-09-20T10:00:00Z", "2026-09-20T09:00:00Z"),
            "2026-09-20T10:00:00Z"
        );
        assert_eq!(past("2026-09-20T10:00:00Z", ""), "2026-09-20T10:00:00Z");
        let null: String = conn
            .query_row(
                concat!("SELECT ", past_sql!("?1", "NULL")),
                ["2026-09-20T10:00:00Z"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(null, "2026-09-20T10:00:00Z");
    }
}
