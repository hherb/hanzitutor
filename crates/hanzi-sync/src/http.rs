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
    /// Dropbox explains itself in the body, and that explanation is the difference
    /// between a bug report nobody can act on and one that names the problem. Three
    /// things are worth having, in order of usefulness to a person: `user_message`,
    /// which Dropbox writes to be shown as-is; `error_summary`, which names the
    /// hierarchy of the failure; and the body itself.
    ///
    /// The body is included **only for a 400**, and that is deliberate. A 400 means
    /// this program built a request Dropbox would not accept — a bug here rather
    /// than anything the learner did — and the first one of these arrived as
    /// `other/...`, which named neither the field nor the reason. A 401, 403, 409 or
    /// 429 is a situation with a sentence attached, and dumping JSON under it would
    /// only make the sentence harder to read.
    fn failed(url: &str, error: ureq::Error) -> SyncError {
        match error {
            ureq::Error::Status(401, _) => SyncError::Unauthorized,
            ureq::Error::Status(400, response) => {
                let body = response.into_string().unwrap_or_default();
                // A 400 means this program built a request Dropbox would not accept.
                // Dropbox usually explains it in `user_message`, and that explanation
                // is worth preferring over the raw body: the missing-scope failure
                // names the scope *and* says which tab of the App Console enables it,
                // which is the whole of what somebody needs to fix it.
                let explanation = user_message(&body).unwrap_or_else(|| body.trim().to_string());
                SyncError::Io(format!("{url} refused the request (400): {explanation}"))
            }
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

/// Dropbox's `user_message`, the field it writes to be shown to a person.
fn user_message(body: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let text = json
        .get("user_message")
        .and_then(|message| message.get("text"))
        .and_then(|text| text.as_str())?;
    (!text.trim().is_empty()).then(|| text.trim().to_string())
}

/// The part of an error body worth putting in front of a person.
///
/// The `user_message` when there is one, then `error_summary`, then the body.
/// Truncated, because a proxy's HTML error page is not a message anybody needs in
/// full. Note that an `error_summary` ends in a random number of dots — Dropbox adds
/// them to discourage exact string matching — so it is never the last word on what
/// went wrong.
fn summarise(body: &str) -> String {
    if let Some(message) = user_message(body) {
        return message;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_scope_is_reported_in_dropboxs_own_words() {
        // The real body of the failure that cost a debugging session: the summary is
        // `other/...`, which says nothing, while the user message names the scope and
        // where to enable it. Preferring the summary would have hidden exactly the
        // sentence that fixed it.
        let body = r#"{
            "error": {".tag": "other"},
            "error_summary": "other/...",
            "user_message": {
                "locale": "en",
                "text": "Error in call to API function \"files/upload\": Your app (ID: 8582195) is not permitted to access this endpoint because it does not have the required scope 'files.content.write'. The owner of the app can enable the scope for the app using the Permissions tab on the App Console."
            }
        }"#;
        let message = summarise(body);
        assert!(message.contains("files.content.write"), "{message}");
        assert!(message.contains("Permissions tab"), "{message}");
        assert!(!message.contains("other/..."), "the useless summary wins otherwise");
    }

    #[test]
    fn an_error_without_a_user_message_falls_back_to_its_summary() {
        let body = r#"{"error":{".tag":"path"},"error_summary":"path/conflict/file/..."}"#;
        assert_eq!(summarise(body), "path/conflict/file/...");
    }

    #[test]
    fn a_body_that_is_not_json_is_shown_rather_than_hidden() {
        // A proxy in the way answers with HTML, and that is worth seeing.
        assert_eq!(summarise("<html>502</html>"), "<html>502</html>");
        assert_eq!(summarise("   "), "with no explanation");
    }
}
