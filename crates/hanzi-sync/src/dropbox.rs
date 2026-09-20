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
//! **Uploads overwrite.** A shard name is derived from the sequence numbers inside
//! it, and this program only ever writes a name once, so an upload is a re-upload of
//! identical bytes and overwriting is harmless — and unlike a create-only upload it
//! survives a retry after a timeout the device could not distinguish from a failure.
//! Immutability is the writer's discipline here rather than the transport's; what
//! checks it is the merge, which refuses two shards that disagree about one attempt.
//!
//! ## What this does not do
//!
//! It does not refresh the access token. A Dropbox access token lasts about four
//! hours, and a sync is a handful of calls, so the caller refreshes once before it
//! starts; if the token has expired anyway, the API answers 401 and this reports
//! [`SyncError::Unauthorized`] rather than pretending it was a network fault.

use serde_json::json;

use crate::http::Http;
use crate::store::{RemoteEntry, RemoteStore, SyncError};

/// The RPC host, for listing.
const API: &str = "https://api.dropboxapi.com/2";
/// The content host, for reading and writing shards.
const CONTENT: &str = "https://content.dropboxapi.com/2";

/// A Dropbox app folder, seen as a store.
pub struct DropboxStore<'a> {
    http: &'a dyn Http,
    access_token: String,
}

impl<'a> DropboxStore<'a> {
    /// A store over the app folder, authorized by `access_token`.
    pub fn new(http: &'a dyn Http, access_token: impl Into<String>) -> Self {
        Self {
            http,
            access_token: access_token.into(),
        }
    }

    /// One RPC call, with the arguments as JSON.
    fn rpc(&self, endpoint: &str, body: serde_json::Value) -> Result<serde_json::Value, SyncError> {
        let url = format!("{API}/{endpoint}");
        let response = self.http.rpc(&url, &self.access_token, &body.to_string())?;
        serde_json::from_str(&response).map_err(|e| {
            SyncError::Malformed(format!("{endpoint} answered something that is not JSON: {e}"))
        })
    }
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
        self.http.download(
            &format!("{CONTENT}/files/download"),
            &self.access_token,
            &json!({ "path": path_of(name) }).to_string(),
        )
    }

    fn put(&self, name: &str, bytes: &[u8]) -> Result<(), SyncError> {
        self.http.upload(
            &format!("{CONTENT}/files/upload"),
            &self.access_token,
            &json!({
                "path": path_of(name),
                // The **object** form, not the bare string. Dropbox's schema types
                // `mode` as a union whose variants are objects with a required
                // `.tag`, and a bare `"overwrite"` is answered with a 400 whose
                // summary is `other/...` — which reads as neither a path problem nor
                // a permissions one, and cost a real debugging session against the
                // live API to find. The test below pins the shape.
                "mode": { ".tag": "overwrite" },
                "autorename": false,
                // Keeps the learner's Dropbox activity feed to themselves: a sync is
                // not something anybody needs a notification about.
                "mute": true,
            })
            .to_string(),
            bytes,
        )?;
        Ok(())
    }
}
