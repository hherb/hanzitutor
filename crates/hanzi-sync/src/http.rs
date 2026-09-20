//! The four HTTP shapes Dropbox uses, and the real client for them.
//!
//! The API is not one protocol but three, and the difference is entirely in where
//! the arguments and the payload go:
//!
//! - **RPC** — JSON body in, JSON body out (`api.dropboxapi.com/2/...`). Listing a
//!   folder is this.
//! - **Upload and download** (`content.dropboxapi.com/2/...`) — the *arguments* go
//!   in a `Dropbox-API-Arg` header and the *bytes* are the body, in one direction
//!   or the other.
//! - **Form** — a URL-encoded body with no authorization at all, which is what the
//!   token endpoint takes.
//!
//! [`Http`] is that shape rather than "an HTTP client", so that everything built on
//! it — the OAuth flow, the folder listing, the store itself — can be tested against
//! an in-memory Dropbox with no network. [`UreqHttp`] is the only part that has to
//! touch a socket, and it is deliberately the part with no decisions in it.

use std::io::Read;
use std::time::Duration;

use crate::store::SyncError;

/// The four requests the Dropbox API needs.
///
/// A `bearer` of `None` means no `Authorization` header, which is how the token
/// endpoint is called: it authenticates with the app key and the code instead.
///
/// `Send + Sync` because the app holds one inside Tauri's managed state and a sync
/// may run on any thread that asks for one. `UreqHttp`'s agent already qualifies.
pub trait Http: Send + Sync {
    /// A JSON-RPC call: a JSON body in, a JSON body out.
    fn rpc(&self, url: &str, bearer: &str, body: &str) -> Result<String, SyncError>;
    /// A form-encoded body with no authorization, for the token endpoint.
    fn form(&self, url: &str, body: &str) -> Result<String, SyncError>;
    /// An upload: the arguments in a header, the bytes as the body.
    fn upload(&self, url: &str, bearer: &str, arg: &str, body: &[u8]) -> Result<String, SyncError>;
    /// A download: the arguments in a header, the bytes as the response.
    fn download(&self, url: &str, bearer: &str, arg: &str) -> Result<Vec<u8>, SyncError>;
}

/// The HTTP client, over `ureq`.
///
/// Not `Sync` matters: everything here is called from whatever thread is running
/// the sync, and the app already serialises study-data access behind its own lock.
pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    /// A client with the timeouts a sync wants.
    ///
    /// Ten seconds connect, sixty read. A sync that hangs is a sync the learner
    /// cannot cancel, and a shard is small — so the generous case is a slow
    /// connection, not a big file.
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(10))
                .timeout_read(Duration::from_secs(60))
                .user_agent(concat!("HanziTutor/", env!("CARGO_PKG_VERSION")))
                .build(),
        }
    }

    /// Turn a `ureq` failure into ours, keeping what Dropbox said.
    ///
    /// Dropbox explains itself in the body — `{"error_summary": "..."}` — and that
    /// sentence is the difference between a bug report nobody can act on and one
    /// that names the problem.
    fn failed(url: &str, error: ureq::Error) -> SyncError {
        match error {
            ureq::Error::Status(401, _) => SyncError::Unauthorized,
            ureq::Error::Status(code, response) => {
                let body = response.into_string().unwrap_or_default();
                SyncError::Io(format!("{url} answered {code}: {}", summarise(&body)))
            }
            ureq::Error::Transport(transport) => SyncError::Io(format!("{url}: {transport}")),
        }
    }
}

impl Default for UreqHttp {
    fn default() -> Self {
        Self::new()
    }
}

/// The part of an error body worth putting in front of a person.
///
/// Dropbox's own `error_summary` when there is one, the body otherwise — truncated,
/// because a proxy's HTML error page is not a message anybody needs in full.
fn summarise(body: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(summary) = json.get("error_summary").and_then(|v| v.as_str()) {
            return summary.to_string();
        }
    }
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "with no explanation".to_string();
    }
    trimmed.chars().take(200).collect()
}

impl Http for UreqHttp {
    fn rpc(&self, url: &str, bearer: &str, body: &str) -> Result<String, SyncError> {
        let response = self
            .agent
            .post(url)
            .set("Authorization", &format!("Bearer {bearer}"))
            .set("Content-Type", "application/json")
            .send_string(body)
            .map_err(|e| UreqHttp::failed(url, e))?;
        response
            .into_string()
            .map_err(|e| SyncError::Io(format!("{url}: {e}")))
    }

    fn form(&self, url: &str, body: &str) -> Result<String, SyncError> {
        let response = self
            .agent
            .post(url)
            .set("Content-Type", "application/x-www-form-urlencoded")
            .send_string(body)
            .map_err(|e| UreqHttp::failed(url, e))?;
        response
            .into_string()
            .map_err(|e| SyncError::Io(format!("{url}: {e}")))
    }

    fn upload(&self, url: &str, bearer: &str, arg: &str, body: &[u8]) -> Result<String, SyncError> {
        let response = self
            .agent
            .post(url)
            .set("Authorization", &format!("Bearer {bearer}"))
            .set("Content-Type", "application/octet-stream")
            .set("Dropbox-API-Arg", arg)
            .send_bytes(body)
            .map_err(|e| UreqHttp::failed(url, e))?;
        response
            .into_string()
            .map_err(|e| SyncError::Io(format!("{url}: {e}")))
    }

    fn download(&self, url: &str, bearer: &str, arg: &str) -> Result<Vec<u8>, SyncError> {
        let response = self
            .agent
            .post(url)
            .set("Authorization", &format!("Bearer {bearer}"))
            .set("Dropbox-API-Arg", arg)
            .call()
            .map_err(|e| UreqHttp::failed(url, e))?;
        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| SyncError::Io(format!("{url}: {e}")))?;
        Ok(bytes)
    }
}
