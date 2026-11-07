//! Dropbox as a [`RemoteStore`].
//!
//! Three endpoints, and the mapping between them and the store's three methods is
//! the whole of this file:
//!
//! | [`RemoteStore`] | Dropbox |
//! |---|---|
//! | `list` | `files/list_folder`, followed by `files/list_folder/continue` until `has_more` is false |
//! | `get` | `files/download` |
//! | `put` | `files/upload` |
//!
//! ## Two decisions worth knowing about
//!
//! **The app folder, not the account.** The Dropbox app is registered for app-folder
//! access, so `""` is the root of `Apps/<name>/` and none of these paths can reach
//! anything else the learner keeps in Dropbox. That is enforced by Dropbox, not by
//! this code — which is the point.
//!
//! **Uploads overwrite, except an append.** A new shard, and every document, is
//! written in `overwrite` mode: writing a shard again writes the same bytes, so
//! overwriting is harmless — and unlike a create-only upload it survives a retry
//! after a timeout the device could not distinguish from a failure. The one write
//! that changes a file's contents is a device topping up its own open attempt shard
//! ([`crate::shard`]), and that is [`RemoteStore::replace`]: `update` mode with the
//! `rev` the file was listed with, so if anything else wrote the file since — a
//! second install sharing this device's identity — Dropbox answers with a conflict
//! and nothing is overwritten. A retry of an update that Dropbox had in fact
//! applied sends identical bytes, which Dropbox does not count as a conflict
//! (`strict_conflict` is left off for exactly that).
//!
//! **A busy Dropbox is waited for, briefly.** A 429 or a 5xx
//! ([`SyncError::Unavailable`]) is tried again, up to five times a request, after
//! the wait Dropbox asked for (`Retry-After`) or, when it did not say, a jittered
//! wait that doubles from half a second. All of a store's waiting together is
//! capped at thirty seconds, and a wait Dropbox asks for that would go past that
//! is not started: a sync must not hang. Every request here is safe to repeat —
//! listing and downloading read, an `overwrite` upload writes the same bytes again,
//! and an `update` upload either writes them again or reports a conflict.
//!
//! ## What this does not do
//!
//! It does not refresh the access token. A Dropbox access token lasts about four
//! hours, and a sync is a handful of calls, so the caller refreshes once before it
//! starts; if the token has expired anyway, the API answers 401 and this reports
//! [`SyncError::Unauthorized`] rather than pretending it was a network fault.

use std::cell::Cell;
use std::hash::BuildHasher;
use std::time::Duration;

use serde_json::json;

use crate::http::Http;
use crate::store::{RemoteEntry, RemoteStore, SyncError};

/// The RPC host, for listing.
///
/// Also the host a reachability probe opens its connection to — see
/// [`crate::reach`] — which is why it is written down once and built into the root
/// below rather than spelled out twice. What has to be reachable is where this
/// client is about to talk and nothing else, and two copies of a hostname is how
/// that quietly stops being true.
pub const API_HOST: &str = "api.dropboxapi.com";
/// The content host, for reading and writing shards.
pub const CONTENT_HOST: &str = "content.dropboxapi.com";

/// The RPC root, for listing.
pub fn api_root() -> String {
    format!("https://{API_HOST}/2")
}

/// The content root, for reading and writing shards.
pub fn content_root() -> String {
    format!("https://{CONTENT_HOST}/2")
}

/// What a compare-and-swap that lost says, followed by the file's name.
pub(crate) const WRITTEN_MEANWHILE: &str =
    "Another copy of this app wrote to Dropbox at the same time; sync again. The file was \
     left as the other copy wrote it:";

/// How many times one request is sent before a busy Dropbox is reported.
pub const RETRY_TRIES: u32 = 5;
/// How long one store waits for a busy Dropbox, in all.
pub const RETRY_BUDGET: Duration = Duration::from_secs(30);
/// The first wait, when Dropbox does not say how long; each one after doubles.
const FIRST_WAIT: Duration = Duration::from_millis(500);

/// A Dropbox app folder, seen as a store.
///
/// One is made for each sync, so its retry budget is one sync's.
pub struct DropboxStore<'a> {
    http: &'a dyn Http,
    access_token: String,
    sleep: Box<dyn Fn(Duration) + 'a>,
    waited: Cell<Duration>,
}

impl<'a> DropboxStore<'a> {
    /// A store over the app folder, authorized by `access_token`.
    pub fn new(http: &'a dyn Http, access_token: impl Into<String>) -> Self {
        Self {
            http,
            access_token: access_token.into(),
            sleep: Box::new(std::thread::sleep),
            waited: Cell::new(Duration::ZERO),
        }
    }

    /// The same, waiting through `sleep` rather than the thread's own, so that a
    /// test can see the waits without spending them.
    pub fn with_sleep(mut self, sleep: impl Fn(Duration) + 'a) -> Self {
        self.sleep = Box::new(sleep);
        self
    }

    /// Send a request, and send it again while Dropbox is busy — see the module
    /// note for how long, and why every request here may be repeated.
    fn retrying<T>(&self, mut send: impl FnMut() -> Result<T, SyncError>) -> Result<T, SyncError> {
        let mut tries = 1;
        loop {
            match send() {
                Err(SyncError::Unavailable {
                    status,
                    retry_after,
                    reason,
                }) if tries < RETRY_TRIES => {
                    // Never sooner than the backoff, so that a `Retry-After: 0`
                    // does not spend the remaining tries back to back.
                    let wait = match retry_after {
                        Some(seconds) => Duration::from_secs(seconds).max(backoff(tries)),
                        None => backoff(tries),
                    };
                    let waited = self.waited.get().saturating_add(wait);
                    if waited > RETRY_BUDGET {
                        // Not started: waiting would only end in this same error
                        // with the learner's time spent.
                        return Err(SyncError::Unavailable {
                            status,
                            retry_after,
                            reason,
                        });
                    }
                    (self.sleep)(wait);
                    self.waited.set(waited);
                    tries += 1;
                }
                other => return other,
            }
        }
    }

    /// One RPC call, with the arguments as JSON.
    fn rpc(&self, endpoint: &str, body: serde_json::Value) -> Result<serde_json::Value, SyncError> {
        let url = format!("{}/{endpoint}", api_root());
        let body = body.to_string();
        let response = self.retrying(|| self.http.rpc(&url, &self.access_token, &body))?;
        serde_json::from_str(&response).map_err(|e| {
            SyncError::Malformed(format!("{endpoint} answered something that is not JSON: {e}"))
        })
    }
}

/// The wait before try `tries + 1` when Dropbox gave none: half a second, doubling,
/// each somewhere between half of that and all of it so that devices told to wait
/// at the same moment do not all come back at the same moment.
fn backoff(tries: u32) -> Duration {
    let full = FIRST_WAIT.saturating_mul(1 << (tries - 1).min(16));
    // A random fraction in [0, 1), from the standard library's own randomly seeded
    // hasher: no dependency for what only has to differ between devices.
    let fraction = (std::collections::hash_map::RandomState::new().hash_one(tries) >> 11) as f64
        / (1u64 << 53) as f64;
    full.mul_f64(0.5 + fraction / 2.0)
}

/// A Dropbox path for a shard name.
///
/// Leading slash, because that is what the API documents for a path below the root
/// — and for an app-folder app the root *is* the app folder.
fn path_of(name: &str) -> String {
    format!("/{name}")
}

/// A shard name for a Dropbox path.
///
/// Dropbox answers with absolute-looking paths even inside an app folder, so the
/// leading slash comes off again to get back to the name the shard format uses.
fn name_of(path: &str) -> &str {
    path.trim_start_matches('/')
}

impl RemoteStore for DropboxStore<'_> {
    fn list(&self) -> Result<Vec<RemoteEntry>, SyncError> {
        let mut entries = Vec::new();
        // Recursive, because the shards are two directories down, and one page at a
        // time because Dropbox caps a page at 2000 entries.
        let mut page = self.rpc(
            "files/list_folder",
            json!({ "path": "", "recursive": true, "include_deleted": false }),
        )?;

        loop {
            let listed = page
                .get("entries")
                .and_then(|entries| entries.as_array())
                .ok_or_else(|| {
                    SyncError::Malformed("files/list_folder listed no entries".to_string())
                })?;
            for entry in listed {
                // Deleted entries come back too when `include_deleted` is on, and the
                // root comes back as a folder; only files are shards.
                if entry.get(".tag").and_then(|tag| tag.as_str()) != Some("file") {
                    continue;
                }
                let Some(path) = entry.get("path_display").and_then(|p| p.as_str()) else {
                    continue;
                };
                entries.push(RemoteEntry {
                    name: name_of(path).to_string(),
                    // `rev` changes whenever the file's contents do, which is what a
                    // caller needs to skip re-reading a shard that has not changed.
                    revision: entry
                        .get("rev")
                        .and_then(|rev| rev.as_str())
                        .unwrap_or_default()
                        .to_string(),
                });
            }

            if page.get("has_more").and_then(|more| more.as_bool()) != Some(true) {
                break;
            }
            let cursor = page
                .get("cursor")
                .and_then(|cursor| cursor.as_str())
                .ok_or_else(|| {
                    SyncError::Malformed(
                        "files/list_folder said there was more to come and gave no cursor"
                            .to_string(),
                    )
                })?
                .to_string();
            page = self.rpc("files/list_folder/continue", json!({ "cursor": cursor }))?;
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn get(&self, name: &str) -> Result<Vec<u8>, SyncError> {
        let url = format!("{}/files/download", content_root());
        let arg = json!({ "path": path_of(name) }).to_string();
        self.retrying(|| self.http.download(&url, &self.access_token, &arg))
    }

    fn put(&self, name: &str, bytes: &[u8]) -> Result<(), SyncError> {
        // The **object** form, not the bare string. Dropbox's schema types `mode`
        // as a union whose variants are objects with a required `.tag`, and a bare
        // `"overwrite"` is answered with a 400 whose summary is `other/...` — which
        // reads as neither a path problem nor a permissions one, and cost a real
        // debugging session against the live API to find. The test below pins the
        // shape.
        self.upload(name, bytes, json!({ ".tag": "overwrite" }))
    }

    /// `update` mode with the listed `rev`: Dropbox writes only if the file is
    /// still that version, and otherwise answers with a conflict.
    ///
    /// The conflict is Dropbox's 409 `path/conflict/...`, which is reported in
    /// words rather than as the bare summary.
    fn replace(&self, name: &str, bytes: &[u8], revision: &str) -> Result<(), SyncError> {
        self.upload(name, bytes, json!({ ".tag": "update", "update": revision }))
            .map_err(|error| match error {
                SyncError::Io(why) if why.contains("409") && why.contains("path/conflict") => {
                    SyncError::Io(format!("{WRITTEN_MEANWHILE} {name}"))
                }
                other => other,
            })
    }
}

impl DropboxStore<'_> {
    /// One upload, in the given write mode.
    fn upload(&self, name: &str, bytes: &[u8], mode: serde_json::Value) -> Result<(), SyncError> {
        let url = format!("{}/files/upload", content_root());
        let arg = json!({
            "path": path_of(name),
            "mode": mode,
            "autorename": false,
            // Keeps the learner's Dropbox activity feed to themselves: a sync is
            // not something anybody needs a notification about.
            "mute": true,
        })
        .to_string();
        self.retrying(|| self.http.upload(&url, &self.access_token, &arg, bytes))?;
        Ok(())
    }
}
