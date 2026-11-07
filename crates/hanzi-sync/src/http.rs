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
    ///
    /// A 429 or a 5xx is [`SyncError::Unavailable`], with the wait Dropbox asked
    /// for: the `Retry-After` header in seconds, or failing that the `retry_after`
    /// its 429 body carries. Those are the failures worth trying again, and the
    /// store does — see [`crate::DropboxStore`].
    fn failed(url: &str, error: ureq::Error) -> SyncError {
        match error {
            ureq::Error::Status(401, _) => SyncError::Unauthorized,
            ureq::Error::Status(400, response) => {
                let body = response.into_string().unwrap_or_default();
                refused(url, &body)
            }
            ureq::Error::Status(code, response) if code == 429 || (500..600).contains(&code) => {
                let header = response
                    .header("Retry-After")
                    .and_then(|value| value.trim().parse::<u64>().ok());
                let body = response.into_string().unwrap_or_default();
                SyncError::Unavailable {
                    status: code,
                    retry_after: header.or_else(|| retry_after_in(&body)),
                    reason: format!("{url} answered {code}: {}", summarise(&body)),
                }
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

/// A 400, as ours.
///
/// Almost always this program built a request Dropbox would not accept. Dropbox
/// usually explains it in `user_message`, and that explanation is worth preferring
/// over the raw body: the missing-scope failure names the scope *and* says which tab
/// of the App Console enables it, which is the whole of what somebody needs to fix it.
///
/// The exception is the token endpoint's `invalid_grant`: the refresh token was
/// revoked (the learner removed the app from their Dropbox account) or has expired.
/// That is not a bug here but a sign-in that is gone, and it is reported as
/// [`SyncError::Unauthorized`] so the learner is told to connect again rather than
/// shown a JSON body on every sync from then on. It is matched on the OAuth `error`
/// field, which is a fixed code, never on the prose beside it.
fn refused(url: &str, body: &str) -> SyncError {
    let error = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|json| {
            json.get("error")
                .and_then(|e| e.as_str())
                .map(str::to_string)
        });
    if error.as_deref() == Some("invalid_grant") {
        return SyncError::Unauthorized;
    }
    let explanation = user_message(body).unwrap_or_else(|| body.trim().to_string());
    SyncError::Io(format!("{url} refused the request (400): {explanation}"))
}

/// The wait a Dropbox 429 body asks for, in seconds: `error.retry_after`.
fn retry_after_in(body: &str) -> Option<u64> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get("error")?
        .get("retry_after")?
        .as_u64()
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
    fn a_revoked_or_expired_refresh_token_is_a_sign_in_to_renew() {
        // What Dropbox's token endpoint answers, with a 400, once the learner has
        // removed the app from their account. It used to reach the screen as raw
        // JSON on every sync, with no hint that connecting again would fix it.
        let body = r#"{"error": "invalid_grant", "error_description": "refresh token is invalid or revoked"}"#;
        assert_eq!(
            refused("https://api.dropboxapi.com/oauth2/token", body),
            SyncError::Unauthorized
        );

        let through_ureq = UreqHttp::failed(
            "https://api.dropboxapi.com/oauth2/token",
            ureq::Error::Status(400, ureq::Response::new(400, "Bad Request", body).unwrap()),
        );
        assert_eq!(through_ureq, SyncError::Unauthorized);
    }

    #[test]
    fn any_other_400_is_still_reported_with_dropboxs_explanation() {
        // Only the fixed OAuth code counts: a 400 that merely *mentions* a grant,
        // or names another error, is still a request this program got wrong.
        for body in [
            r#"{"error": "invalid_request", "error_description": "invalid_grant mentioned"}"#,
            r#"{"error": {".tag": "invalid_grant"}}"#,
            "invalid_grant",
        ] {
            let error = refused("https://example.test/", body);
            assert!(
                matches!(&error, SyncError::Io(why) if why.contains("(400)")),
                "{body}: {error:?}"
            );
        }
    }

    #[test]
    fn a_429_or_a_5xx_is_a_failure_worth_trying_again_with_the_wait_asked_for() {
        let url = "https://api.dropboxapi.com/2/files/list_folder";
        let response = |text: &str| text.parse::<ureq::Response>().unwrap();

        let header = UreqHttp::failed(
            url,
            ureq::Error::Status(
                429,
                response("HTTP/1.1 429 Too Many Requests\r\nRetry-After: 7\r\n\r\n"),
            ),
        );
        assert!(
            matches!(header, SyncError::Unavailable { status: 429, retry_after: Some(7), .. }),
            "{header:?}"
        );

        // Dropbox also says it in the body; the header wins when both are there.
        let body = r#"{"error_summary": "too_many_requests/..", "error": {"reason": {".tag": "too_many_requests"}, "retry_after": 3}}"#;
        let in_body = UreqHttp::failed(
            url,
            ureq::Error::Status(
                429,
                response(&format!(
                    "HTTP/1.1 429 Too Many Requests\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                )),
            ),
        );
        assert!(
            matches!(in_body, SyncError::Unavailable { retry_after: Some(3), .. }),
            "{in_body:?}"
        );

        let unavailable = UreqHttp::failed(
            url,
            ureq::Error::Status(503, response("HTTP/1.1 503 Service Unavailable\r\n\r\n")),
        );
        assert!(
            matches!(unavailable, SyncError::Unavailable { status: 503, retry_after: None, .. }),
            "{unavailable:?}"
        );

        // A conflict is an answer, not a busy server, and is not tried again.
        let conflict = UreqHttp::failed(
            url,
            ureq::Error::Status(409, response("HTTP/1.1 409 Conflict\r\n\r\n")),
        );
        assert!(matches!(conflict, SyncError::Io(_)), "{conflict:?}");
    }

    #[test]
    fn a_body_that_is_not_json_is_shown_rather_than_hidden() {
        // A proxy in the way answers with HTML, and that is worth seeing.
        assert_eq!(summarise("<html>502</html>"), "<html>502</html>");
        assert_eq!(summarise("   "), "with no explanation");
    }
}
