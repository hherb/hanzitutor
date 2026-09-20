//! Dropbox's half of OAuth 2.0: PKCE, and the three token calls.
//!
//! ## Why PKCE, and why no redirect URI
//!
//! This app is open source and ships no server, so it cannot keep a client secret —
//! which Dropbox names as exactly the case PKCE exists for. There is therefore no
//! secret anywhere in this file, and a test asserts the token request does not
//! carry one.
//!
//! Dropbox will not accept a custom URL scheme as a redirect (`hanzi-tutor://` is
//! refused: every redirect must be HTTPS except `localhost`), so this app registers
//! **no redirect URI at all**. That is permitted for the code flow, and it makes the
//! flow deliberately unglamorous: the authorization page shows the learner a code,
//! and they paste it into the app. One paste per device, once, in exchange for no
//! server, no hosted domain and no per-platform URL registration.
//!
//! ## What is stored, and what is not
//!
//! The **refresh token** is the thing worth protecting: it does not expire on its
//! own and it is the whole of the account access this app has. The access token
//! that comes with it lasts about four hours and is not worth writing down. Neither
//! belongs in the study database, which is an ordinary file in an ordinary
//! directory and gets copied around by backup tools.

use base64::Engine as _;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::http::Http;
use crate::store::SyncError;

/// Where the learner authorizes the app.
pub const AUTHORIZE_URL: &str = "https://www.dropbox.com/oauth2/authorize";
/// Where a code, or a refresh token, becomes an access token.
pub const TOKEN_URL: &str = "https://api.dropboxapi.com/oauth2/token";
/// Where a learner's authorization is given up.
pub const REVOKE_URL: &str = "https://api.dropboxapi.com/2/auth/token/revoke";

/// The permissions this app asks for, and no more.
///
/// `account_info.read` is only so the settings screen can say *which* account is
/// connected rather than just "connected". The three files scopes are the minimum
/// for listing, reading and writing shards in the app's own folder — and the app is
/// registered for **app folder** access, so even those reach nothing outside
/// `Apps/<name>/`.
pub const SCOPES: &str = "account_info.read files.metadata.read files.content.read files.content.write";

/// How long a verifier may be, from RFC 7636: 43 to 128 characters.
const MIN_VERIFIER: usize = 43;
const MAX_VERIFIER: usize = 128;

/// A PKCE verifier, and the challenge derived from it.
///
/// The challenge travels to Dropbox in the browser; the verifier stays here and is
/// presented only when the code is traded in. That is what makes an intercepted
/// code useless to anyone else.
#[derive(Clone, Debug)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    /// A fresh verifier and its S256 challenge.
    ///
    /// The randomness is a pair of UUIDs rather than a byte buffer: `uuid` is
    /// already in the build graph, it draws from the operating system's
    /// cryptographically secure generator, and two version-4 UUIDs are 244 bits of
    /// entropy — far past anything a verifier needs. Hex is entirely within the
    /// character set RFC 7636 allows, and two UUIDs concatenated land inside the
    /// 43-to-128 window.
    pub fn generate() -> Self {
        let verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        Self {
            challenge: challenge_for(&verifier),
            verifier,
        }
    }

    /// Whether this verifier is one Dropbox will accept.
    pub fn is_well_formed(&self) -> bool {
        (MIN_VERIFIER..=MAX_VERIFIER).contains(&self.verifier.len())
            && self
                .verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-._~".contains(c))
    }
}

/// The S256 challenge for a verifier: base64url of its SHA-256, unpadded.
pub fn challenge_for(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

/// The URL to open in the learner's browser.
///
/// The system browser, not a webview: Dropbox asks for that, and it matters for the
/// accounts that sign in to Dropbox through Google, whose policy forbids their flow
/// inside one.
///
/// `state` is optional and this app passes `None`. It exists to stop a forged
/// callback, and there is no callback here — no redirect URI, and the learner types
/// the code in themselves — so there is no request to forge and a state value would
/// be ceremony rather than protection. The parameter is kept because a caller that
/// *does* have a redirect will want it.
pub fn authorize_url(app_key: &str, pkce: &Pkce, state: Option<&str>) -> String {
    let mut query: Vec<(String, String)> = vec![
        ("client_id".to_string(), app_key.to_string()),
        ("response_type".to_string(), "code".to_string()),
        // Without this there is no refresh token, and the learner would have to
        // paste a code every four hours.
        ("token_access_type".to_string(), "offline".to_string()),
        ("code_challenge".to_string(), pkce.challenge.clone()),
        ("code_challenge_method".to_string(), "S256".to_string()),
        ("scope".to_string(), SCOPES.to_string()),
    ];
    if let Some(state) = state {
        query.push(("state".to_string(), state.to_string()));
    }
    let query = query
        .iter()
        .map(|(name, value)| format!("{}={}", encode(name), encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{AUTHORIZE_URL}?{query}")
}

/// What Dropbox returns from the token endpoint.
///
/// Every field is optional to the deserialiser, and validated by [`tokens`] instead.
/// That ordering is deliberate: Dropbox reports a refusal as a 200 with
/// `error_description` and *no* `access_token`, so a struct that demanded one would
/// fail to parse and the learner would be shown a missing-field message rather than
/// the sentence Dropbox wrote.
#[derive(Clone, Debug, Deserialize)]
pub struct Tokens {
    #[serde(default)]
    pub access_token: String,
    /// Present only because the authorization asked for offline access, and the
    /// only part of this worth keeping.
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Seconds until the access token expires — about four hours.
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// The account this authorization belongs to, for naming it on screen.
    #[serde(default)]
    pub account_id: Option<String>,
    /// Dropbox's explanation when it refused.
    #[serde(default)]
    pub error_description: Option<String>,
}

impl Tokens {
    /// The token this authorization must not lose.
    pub fn refresh_token(&self) -> Result<&str, SyncError> {
        self.refresh_token.as_deref().ok_or_else(|| {
            SyncError::Io(
                "Dropbox did not return a refresh token. The authorization has to ask \
                 for offline access, or the learner will have to sign in again every \
                 few hours."
                    .to_string(),
            )
        })
    }
}

/// Trade the pasted code for tokens.
///
/// No `redirect_uri`, because there is none, and no `client_secret`, because PKCE
/// replaces it.
pub fn exchange_code(
    http: &dyn Http,
    app_key: &str,
    code: &str,
    verifier: &str,
) -> Result<Tokens, SyncError> {
    let body = form(&[
        ("code", code),
        ("grant_type", "authorization_code"),
        ("client_id", app_key),
        ("code_verifier", verifier),
    ]);
    tokens(&http.form(TOKEN_URL, &body)?)
}

/// Trade a refresh token for a fresh access token.
///
/// The refresh token does not come back again — it does not expire and is reused
/// for the life of the authorization — so a caller that replaces what it stored
/// with the response would throw the account access away.
pub fn refresh(http: &dyn Http, app_key: &str, refresh_token: &str) -> Result<Tokens, SyncError> {
    let body = form(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", app_key),
    ]);
    tokens(&http.form(TOKEN_URL, &body)?)
}

/// Give up an authorization, so that nothing is left behind on Dropbox's side.
pub fn revoke(http: &dyn Http, access_token: &str) -> Result<(), SyncError> {
    http.rpc(REVOKE_URL, access_token, "null")?;
    Ok(())
}

/// Parse a token response, turning Dropbox's 200-with-an-error into a failure.
///
/// The refusal is checked first, and before the access token is required, because
/// that is the order Dropbox's own answers arrive in: a refusal has no access token,
/// so demanding one first would replace Dropbox's explanation with a parse error.
fn tokens(body: &str) -> Result<Tokens, SyncError> {
    let parsed: Tokens = serde_json::from_str(body).map_err(|e| {
        SyncError::Malformed(format!("Dropbox's token response could not be read: {e}"))
    })?;
    if let Some(description) = &parsed.error_description {
        return Err(SyncError::Io(format!(
            "Dropbox refused the authorization: {description}"
        )));
    }
    if parsed.access_token.is_empty() {
        return Err(SyncError::Malformed(
            "Dropbox's token response carried no access token".to_string(),
        ));
    }
    Ok(parsed)
}

/// A URL-encoded form body.
fn form(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(name, value)| format!("{}={}", encode(name), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Percent-encode one query or form value.
///
/// Hand-written rather than pulled in: it is eight lines, the rule is one line of
/// RFC 3986, and the only characters this app ever passes are a hex app key, a
/// base64url challenge and a scope list with spaces in it.
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}
