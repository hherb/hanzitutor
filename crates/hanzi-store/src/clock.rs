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
//! So a stamp is `max(now, floor)`, where the floor is kept in `meta` and rises
//! two ways:
//!
//! - **Past a peer's stamp, by a second, whenever one is seen.** Applying a merged
//!   view hands every winning stamp to [`observe`]. An edit made after that is then
//!   strictly later than anything this device has seen, which is what "later" has
//!   to mean for a learner: the edit they made last wins, whatever either clock says.
//! - **To this device's own last stamp.** So a clock that steps backwards (an NTP
//!   correction, a dead RTC battery) cannot make a newer local edit lose to an older
//!   one. No second is added here: two writes by one device inside one second are
//!   already told apart by the row's revision.
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

/// The stamp for a write this device is making now, and the floor moved to it.
pub(crate) fn next_stamp(conn: &Connection) -> rusqlite::Result<String> {
    let now = now_iso8601();
    let stamp = match floor(conn)? {
        Some(floor) if floor > now => floor,
        _ => now,
    };
    set_meta(conn, CLOCK_KEY, &stamp)?;
    Ok(stamp)
}

/// Raise the floor past every stamp a *peer* wrote.
///
/// `stamps` is `(updated_at, device_id)` pairs. This device's own are skipped: they
/// are already at or below the floor, and adding a second to them would push its
/// next local edit ahead of real time for no reason. A stamp that does not parse —
/// an empty one from before stamps existed, a hand-edited one — says nothing about
/// time and is ignored.
///
/// `own_device` is whatever `Db::device_id` hands back, borrowed or owned.
pub(crate) fn observe<'a>(
    conn: &Connection,
    own_device: impl AsRef<str>,
    stamps: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> rusqlite::Result<()> {
    let own_device = own_device.as_ref();
    let seen = stamps
        .into_iter()
        .filter(|(_, device)| *device != own_device)
        .filter_map(|(stamp, _)| parse_iso8601(stamp).map(|_| stamp))
        .max();
    let Some(seen) = seen else {
        return Ok(());
    };
    // One second past it, unless that would leave the range the format can write;
    // then the stamp itself, which still ties rather than loses.
    let past = add_seconds(seen, 1).unwrap_or_else(|| seen.to_string());
    if floor(conn)?.is_some_and(|floor| floor >= past) {
        return Ok(());
    }
    set_meta(conn, CLOCK_KEY, &past)
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
        observe(&conn, "me", [("2999-01-01T10:00:00Z", "fast")]).unwrap();
        assert_eq!(next_stamp(&conn).unwrap(), "2999-01-01T10:00:01Z");
        // And no further: this device's own stamps raise the floor without a second.
        assert_eq!(next_stamp(&conn).unwrap(), "2999-01-01T10:00:01Z");
    }

    #[test]
    fn this_devices_own_stamps_and_unreadable_ones_do_not_move_the_floor() {
        let conn = conn();
        observe(
            &conn,
            "me",
            [
                ("2999-01-01T10:00:00Z", "me"),
                ("", "peer"),
                ("tomorrow", "peer"),
            ],
        )
        .unwrap();
        assert!(next_stamp(&conn).unwrap().as_str() < "2999");
    }

    #[test]
    fn an_older_peer_stamp_never_lowers_the_floor() {
        let conn = conn();
        observe(&conn, "me", [("2999-01-01T10:00:00Z", "fast")]).unwrap();
        observe(&conn, "me", [("2000-01-01T10:00:00Z", "slow")]).unwrap();
        assert_eq!(next_stamp(&conn).unwrap(), "2999-01-01T10:00:01Z");
    }
}
