//! Whether there is a network path at all, asked *before* a sync spends ten seconds
//! finding out that there is not.
//!
//! ## Why this exists
//!
//! A sync that starts by itself at launch has nobody watching it who chose it, so
//! the one thing it must not do is stall. `UreqHttp` allows ten seconds for a
//! connect — generous on purpose, because the slow case it is written for is a bad
//! mobile connection rather than a dead one — and a device with the radio off, or
//! in a tunnel, or on a wifi network that has gone away, spends all ten of them
//! per request before failing. There are four or five requests in a sync. A launch
//! sync that takes a minute to conclude "no network" is worse than no launch sync
//! at all, and it is the *first* thing a learner would notice about this feature.
//!
//! ## What the check is, and what it is not
//!
//! It resolves the API host and opens a **TCP connection** to it, on port 443, and
//! closes it. No TLS handshake, no request, no data sent — the question is whether
//! there is a path to the machine the sync is about to talk to, and nothing beyond
//! that is worth knowing here.
//!
//! It is therefore not a promise. A captive portal answers a TCP connection and
//! then refuses or mangles the request, and a `true` from here only means "worth
//! trying" — which is all the caller is allowed to conclude. What a `false` means
//! is stronger: nothing was attempted, and nothing is going to work.
//!
//! ## Why the wait is bounded from outside the socket
//!
//! Name resolution has no timeout this code can set: `ToSocketAddrs` goes to the
//! system resolver, which may try several servers with its own ideas about how long
//! to wait. That is precisely the case this module exists for — a device attached
//! to a network with no working DNS is the classic launch-time hang — so the whole
//! probe runs on a **thread** and the caller waits [`GRACE`] past its budget and
//! treats silence as "no". A probe that outlives its welcome is left to finish and
//! its answer is dropped.

use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::dropbox::API_HOST;

/// Whether a connection to the sync host is worth attempting.
///
/// A trait rather than a function so that the app's sync service can be driven in
/// a test with no network at all, which is the same reason [`crate::Http`] is one.
pub trait Reach: Send + Sync {
    fn reachable(&self) -> bool;
}

/// How long the whole probe may take, past the budget it was given for connecting.
///
/// The budget covers the connection attempts *after* the name resolves. The
/// resolver's own share is what this covers, and it is deliberately short: the
/// answer to "is there a network" is worth a couple of seconds at launch and is not
/// worth any more.
const GRACE: Duration = Duration::from_millis(750);

/// The real probe: a TCP connection to the sync host.
pub struct TcpReach {
    host: String,
    port: u16,
    /// How long all the connection attempts together may take.
    budget: Duration,
}

impl TcpReach {
    /// A probe of Dropbox's API host, which is where the first call of a sync goes.
    ///
    /// The host comes from [`crate::dropbox`] rather than being written here, so
    /// the probe cannot end up asking about a machine the client never talks to.
    pub fn dropbox() -> Self {
        Self::new(API_HOST, 443, Duration::from_secs(2))
    }

    /// A probe of any host and port.
    pub fn new(host: impl Into<String>, port: u16, budget: Duration) -> Self {
        Self {
            host: host.into(),
            port,
            budget,
        }
    }
}

impl Reach for TcpReach {
    fn reachable(&self) -> bool {
        let (host, port, budget) = (self.host.clone(), self.port, self.budget);
        let (answer, heard) = mpsc::channel();
        // Detached on purpose: see the module note. Joining it would put the wait
        // for a dead resolver straight back in the caller.
        std::thread::spawn(move || {
            let _ = answer.send(probe(&host, port, budget));
        });
        heard.recv_timeout(budget + GRACE).unwrap_or(false)
    }
}

/// Resolve the name, then try each address, with one budget for the lot.
///
/// One budget rather than one per address, because a host with several A records
/// and no route to any of them would otherwise multiply the wait by the number of
/// records — and a launch-time probe is the one place that arithmetic is visible.
fn probe(host: &str, port: u16, budget: Duration) -> bool {
    let started = Instant::now();
    let Ok(addresses) = (host, port).to_socket_addrs() else {
        return false;
    };
    for address in addresses {
        let left = budget.saturating_sub(started.elapsed());
        if left.is_zero() {
            return false;
        }
        if TcpStream::connect_timeout(&address, left).is_ok() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// A named probe is worth nothing if it names a machine the client never
    /// talks to, and the two strings live in different files.
    #[test]
    fn the_probe_asks_about_the_host_the_api_calls_go_to() {
        assert_eq!(TcpReach::dropbox().host, API_HOST);
        assert_eq!(API_HOST, "api.dropboxapi.com");
        assert!(
            crate::dropbox::api_root().contains(API_HOST),
            "the RPC root is not on the probed host"
        );
    }

    /// The positive case, with no network: something is listening, so there is a
    /// path to it.
    #[test]
    fn a_host_that_is_listening_is_reachable() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let probe = TcpReach::new("127.0.0.1", port, Duration::from_secs(1));
        assert!(probe.reachable(), "a bound socket is a path");
    }

    /// A port with nothing behind it, which is the ordinary "no server there"
    /// answer. Refused immediately rather than waited out.
    #[test]
    fn a_port_with_nothing_behind_it_is_not_reachable() {
        // Port 1 is privileged and unbound: connecting is refused, not dropped.
        let probe = TcpReach::new("127.0.0.1", 1, Duration::from_millis(500));
        assert!(!probe.reachable());
    }

    /// A name that cannot resolve, which is the case the thread and the grace
    /// period exist for. `.invalid` is reserved by RFC 2606 and never resolves, so
    /// this answers no whether or not the machine has a network.
    #[test]
    fn a_name_that_does_not_resolve_is_not_reachable() {
        let probe = TcpReach::new("hanzi-tutor.invalid", 443, Duration::from_millis(300));
        assert!(!probe.reachable());
    }
}
