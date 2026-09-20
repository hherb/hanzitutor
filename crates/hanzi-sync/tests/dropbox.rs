//! The Dropbox client, against an in-memory Dropbox.
//!
//! Nothing here opens a socket. [`FakeDropbox`] answers the same four request
//! shapes the real API does, so everything built on top of [`Http`] — the
//! authorization URL, the PKCE exchange, the cursor-following listing, and the
//! store itself — is exercised end to end. What is *not* covered is `UreqHttp`,
//! which is the part with no decisions in it: putting bytes on a socket and
//! reading the answer back.
//!
//! The last test is the one that matters most: the same shards written through the
//! Dropbox client and through a directory must read back identically. That is the
//! claim that a transport is interchangeable.

use std::collections::BTreeMap;
use std::sync::Mutex;

use base64::Engine as _;
use hanzi_core::Rating;
use hanzi_sync::{
    authorize_url, exchange_code, read_attempts, refresh, write_attempts, DropboxStore, Http,
    MergedAttempt, Pkce, RemoteStore, SyncError, SCOPES,
};
use serde_json::json;
use sha2::{Digest, Sha256};

// ---- an in-memory Dropbox --------------------------------------------------

/// One request, kept so a test can assert what was actually sent.
#[derive(Clone, Debug, PartialEq)]
enum Request {
    Form { url: String, body: String },
    Rpc { url: String, bearer: String, body: String },
    Upload { url: String, bearer: String, arg: String, bytes: Vec<u8> },
    Download { url: String, bearer: String, arg: String },
}

/// A Dropbox app folder that lives in a `BTreeMap`.
struct FakeDropbox {
    files: Mutex<BTreeMap<String, Vec<u8>>>,
    requests: Mutex<Vec<Request>>,
    /// How many entries one `list_folder` page holds, so pagination is exercised
    /// with something smaller than the real 2000.
    page: usize,
    /// Answer every call with 401, as an expired access token does.
    expired: bool,
    /// What the token endpoint says.
    token_response: String,
}

impl FakeDropbox {
    fn new() -> Self {
        Self {
            files: Mutex::new(BTreeMap::new()),
            requests: Mutex::new(Vec::new()),
            page: 2,
            expired: false,
            token_response: json!({
                "access_token": "sl.access",
                "refresh_token": "refresh-me",
                "expires_in": 14400,
                "token_type": "bearer",
                "account_id": "dbid:AAAA"
            })
            .to_string(),
        }
    }

    fn files(&self) -> Vec<String> {
        self.files.lock().unwrap().keys().cloned().collect()
    }

    fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    fn record(&self, request: Request) {
        self.requests.lock().unwrap().push(request);
    }

    /// A stand-in for Dropbox's `rev`: different whenever the bytes are.
    fn rev_of(bytes: &[u8]) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in bytes {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
        format!("{hash:016x}")
    }

    /// One page of `files/list_folder`, with the cursor naming where to resume.
    fn list_page(&self, offset: usize) -> String {
        let files = self.files.lock().unwrap();
        let names: Vec<&String> = files.keys().collect();
        let end = (offset + self.page).min(names.len());
        let entries: Vec<serde_json::Value> = names[offset..end]
            .iter()
            .map(|name| {
                json!({
                    ".tag": "file",
                    "name": name.rsplit('/').next().unwrap_or(name),
                    "path_display": name,
                    "rev": Self::rev_of(&files[*name]),
                })
            })
            .collect();
        json!({
            "entries": entries,
            "cursor": format!("cursor:{}", end),
            "has_more": end < names.len(),
        })
        .to_string()
    }
}

impl Http for FakeDropbox {
    fn rpc(&self, url: &str, bearer: &str, body: &str) -> Result<String, SyncError> {
        self.record(Request::Rpc {
            url: url.to_string(),
            bearer: bearer.to_string(),
            body: body.to_string(),
        });
        if self.expired {
            return Err(SyncError::Unauthorized);
        }
        if url.ends_with("files/list_folder") {
            // A fresh listing always starts at the beginning.
            return Ok(self.list_page(0));
        }
        if url.ends_with("files/list_folder/continue") {
            let arguments: serde_json::Value = serde_json::from_str(body).unwrap();
            let cursor = arguments["cursor"].as_str().unwrap_or("cursor:0");
            let offset: usize = cursor.trim_start_matches("cursor:").parse().unwrap_or(0);
            return Ok(self.list_page(offset));
        }
        if url.ends_with("auth/token/revoke") {
            return Ok("null".to_string());
        }
        panic!("the fake was asked for an endpoint it does not know: {url}");
    }

    fn form(&self, url: &str, body: &str) -> Result<String, SyncError> {
        self.record(Request::Form {
            url: url.to_string(),
            body: body.to_string(),
        });
        if self.expired {
            return Err(SyncError::Unauthorized);
        }
        Ok(self.token_response.clone())
    }

    fn upload(&self, url: &str, bearer: &str, arg: &str, body: &[u8]) -> Result<String, SyncError> {
        self.record(Request::Upload {
            url: url.to_string(),
            bearer: bearer.to_string(),
            arg: arg.to_string(),
            bytes: body.to_vec(),
        });
        if self.expired {
            return Err(SyncError::Unauthorized);
        }
        let arguments: serde_json::Value = serde_json::from_str(arg).unwrap();
        let path = arguments["path"].as_str().unwrap().to_string();
        self.files
            .lock()
            .unwrap()
            .insert(path.clone(), body.to_vec());
        Ok(json!({ "name": path, "size": body.len() }).to_string())
    }

    fn download(&self, url: &str, bearer: &str, arg: &str) -> Result<Vec<u8>, SyncError> {
        self.record(Request::Download {
            url: url.to_string(),
            bearer: bearer.to_string(),
            arg: arg.to_string(),
        });
        if self.expired {
            return Err(SyncError::Unauthorized);
        }
        let arguments: serde_json::Value = serde_json::from_str(arg).unwrap();
        let path = arguments["path"].as_str().unwrap();
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| SyncError::Io(format!("no such file: {path}")))
    }
}

fn attempt(seq: i64, ch: &str, at: &str, score: f32) -> MergedAttempt {
    MergedAttempt {
        device_id: "phone".to_string(),
        seq,
        ch: ch.to_string(),
        at: at.to_string(),
        score,
        rating: Rating::from_score(score),
    }
}

fn base64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// ---- the authorization URL -------------------------------------------------

#[test]
fn the_authorization_url_asks_for_offline_access_and_carries_no_secret() {
    // No redirect URI, because Dropbox will not accept our scheme and the code flow
    // makes the parameter optional; offline access, because otherwise there is no
    // refresh token and the learner pastes a code every four hours.
    let pkce = Pkce::generate();
    let url = authorize_url("q48frogetl52kp9", &pkce, Some("state-123"));

    assert!(url.starts_with("https://www.dropbox.com/oauth2/authorize?"), "{url}");
    assert!(url.contains("client_id=q48frogetl52kp9"), "{url}");
    assert!(url.contains("response_type=code"), "{url}");
    assert!(url.contains("token_access_type=offline"), "{url}");
    assert!(url.contains(&format!("code_challenge={}", pkce.challenge)), "{url}");
    assert!(url.contains("code_challenge_method=S256"), "{url}");
    assert!(url.contains("state=state-123"), "{url}");

    // The scopes go through a query encoder, so the spaces are escaped — and the
    // scopes are in there at all.
    for scope in SCOPES.split(' ') {
        assert!(url.contains(scope), "{scope} missing from {url}");
    }
    assert!(!url.contains(' '), "a space would truncate the query: {url}");

    // The whole point of PKCE: nothing here is a secret.
    assert!(!url.contains("client_secret"), "{url}");
    assert!(!url.to_lowercase().contains("redirect_uri"), "{url}");

    // This app passes no state, because there is no callback to forge — and then the
    // parameter is not sent at all rather than sent empty.
    let without = authorize_url("q48frogetl52kp9", &pkce, None);
    assert!(!without.contains("state="), "{without}");
    assert!(without.contains("code_challenge="), "{without}");
}

#[test]
fn a_challenge_is_the_base64url_sha256_of_its_verifier() {
    let pkce = Pkce::generate();
    let expected = base64url(&Sha256::digest(pkce.verifier.as_bytes()));
    assert_eq!(pkce.challenge, expected);

    // Unpadded base64url, which is what Dropbox expects of an S256 challenge.
    assert!(!pkce.challenge.contains('='), "{}", pkce.challenge);
    assert!(!pkce.challenge.contains('+'), "{}", pkce.challenge);
    assert!(!pkce.challenge.contains('/'), "{}", pkce.challenge);
    assert_eq!(pkce.challenge.len(), 43, "SHA-256 is 32 bytes, unpadded");
}

#[test]
fn a_verifier_is_the_shape_rfc_7636_allows_and_is_never_reused() {
    let first = Pkce::generate();
    let second = Pkce::generate();
    assert_ne!(first.verifier, second.verifier, "a verifier is used once");
    assert_ne!(first.challenge, second.challenge);
    assert!(first.is_well_formed(), "{} is not", first.verifier);
    assert!(
        (43..=128).contains(&first.verifier.len()),
        "{} characters is outside the range Dropbox accepts",
        first.verifier.len()
    );
}

// ---- the token calls -------------------------------------------------------

#[test]
fn a_pasted_code_becomes_a_refresh_token() {
    let fake = FakeDropbox::new();
    let tokens = exchange_code(&fake, "app-key", "the-code", "the-verifier").unwrap();

    assert_eq!(tokens.access_token, "sl.access");
    assert_eq!(tokens.refresh_token(), Ok("refresh-me"));
    assert_eq!(tokens.expires_in, Some(14400));
    assert_eq!(tokens.account_id.as_deref(), Some("dbid:AAAA"));

    let Request::Form { url, body } = &fake.requests()[0] else {
        panic!("the token call is a form post");
    };
    assert!(url.ends_with("/oauth2/token"), "{url}");
    assert!(body.contains("grant_type=authorization_code"), "{body}");
    assert!(body.contains("code=the-code"), "{body}");
    assert!(body.contains("client_id=app-key"), "{body}");
    assert!(body.contains("code_verifier=the-verifier"), "{body}");
    assert!(!body.contains("client_secret"), "PKCE means there is no secret: {body}");
    assert!(!body.contains("redirect_uri"), "and no redirect to match: {body}");
}

#[test]
fn a_refresh_token_becomes_a_new_access_token() {
    let fake = FakeDropbox::new();
    let tokens = refresh(&fake, "app-key", "refresh-me").unwrap();
    assert_eq!(tokens.access_token, "sl.access");

    let Request::Form { body, .. } = &fake.requests()[0] else {
        panic!("the refresh is a form post");
    };
    assert!(body.contains("grant_type=refresh_token"), "{body}");
    assert!(body.contains("refresh_token=refresh-me"), "{body}");
    // Refreshing must not be able to replace the refresh token with nothing.
    assert!(!body.contains("client_secret"), "{body}");
}

#[test]
fn an_authorization_that_came_back_without_a_refresh_token_is_refused() {
    // Only reachable if the authorization URL stopped asking for offline access, so
    // the failure mode is a code change rather than a network one — which is exactly
    // why it should say what is wrong instead of storing an empty token.
    let fake = FakeDropbox {
        token_response: json!({ "access_token": "sl.access" }).to_string(),
        ..FakeDropbox::new()
    };
    let tokens = exchange_code(&fake, "app-key", "code", "verifier").unwrap();
    let error = tokens.refresh_token().unwrap_err();
    assert!(
        error.to_string().contains("offline access"),
        "the message should name the cause: {error}"
    );
}

#[test]
fn a_dropbox_error_in_a_200_response_is_reported() {
    // Dropbox sometimes answers 200 with the failure in the body, which would
    // otherwise read as success with no access token.
    let fake = FakeDropbox {
        token_response: json!({
            "error": "invalid_grant",
            "error_description": "code has already been used"
        })
        .to_string(),
        ..FakeDropbox::new()
    };
    let error = exchange_code(&fake, "app-key", "code", "verifier").unwrap_err();
    assert!(error.to_string().contains("code has already been used"), "{error}");
}

#[test]
fn an_expired_access_token_is_its_own_kind_of_failure() {
    // The one error with an obvious next move. Reporting it as a network fault
    // would send somebody to check a connection that is working.
    let fake = FakeDropbox {
        expired: true,
        ..FakeDropbox::new()
    };
    let store = DropboxStore::new(&fake, "sl.stale");
    assert_eq!(store.list().unwrap_err(), SyncError::Unauthorized);
    assert_eq!(store.get("x.jsonl").unwrap_err(), SyncError::Unauthorized);
    assert_eq!(store.put("x.jsonl", b"y").unwrap_err(), SyncError::Unauthorized);
}

// ---- listing and the store -------------------------------------------------

#[test]
fn listing_follows_the_cursor_to_the_end() {
    let fake = FakeDropbox::new();
    for seq in 1..=5 {
        let name = format!("/devices/phone/attempts/{seq:012}.jsonl");
        fake.files.lock().unwrap().insert(name, vec![seq as u8]);
    }
    // Something that is not a shard, so the tag and path handling is exercised.
    fake.files
        .lock()
        .unwrap()
        .insert("/notes.txt".to_string(), b"hello".to_vec());

    let store = DropboxStore::new(&fake, "sl.access");
    let entries = store.list().unwrap();

    // Five shards across three pages of two, and the note is listed too: telling
    // shards from other files is the shard format's job, not the transport's.
    assert_eq!(entries.len(), 6, "{entries:?}");
    assert!(entries.iter().any(|e| e.name == "notes.txt"));
    let shards: Vec<&str> = entries
        .iter()
        .map(|entry| entry.name.as_str())
        .filter(|name| name.starts_with("devices/"))
        .collect();
    assert_eq!(shards.len(), 5);
    assert_eq!(shards[0], "devices/phone/attempts/000000000001.jsonl");
    assert!(
        shards.iter().all(|name| !name.starts_with('/')),
        "the leading slash comes off: {shards:?}"
    );
    assert!(
        entries.iter().all(|entry| !entry.revision.is_empty()),
        "every entry carries the revision a caller skips downloads by"
    );

    // Three listing calls: one to start, two to continue.
    let listings = fake
        .requests()
        .iter()
        .filter(|r| matches!(r, Request::Rpc { url, .. } if url.contains("list_folder")))
        .count();
    assert_eq!(listings, 3);
}

#[test]
fn an_upload_lands_in_the_app_folder_and_is_not_announced() {
    let fake = FakeDropbox::new();
    let store = DropboxStore::new(&fake, "sl.access");
    store
        .put("devices/phone/attempts/000000000001.jsonl", b"one\n")
        .unwrap();

    assert_eq!(
        fake.files(),
        vec!["/devices/phone/attempts/000000000001.jsonl".to_string()],
        "a leading slash, relative to the app folder"
    );

    let Request::Upload { arg, bearer, .. } = &fake.requests()[0] else {
        panic!("put is an upload");
    };
    let arguments: serde_json::Value = serde_json::from_str(arg).unwrap();
    assert_eq!(arguments["mode"], "overwrite", "a retry must survive");
    assert_eq!(arguments["autorename"], false, "a rename would break the name");
    assert_eq!(arguments["mute"], true, "a sync is nobody's notification");
    assert_eq!(bearer, "sl.access");
}

#[test]
fn the_same_shards_written_through_dropbox_read_back_the_same() {
    // The claim that matters: a transport is interchangeable. The same attempts go
    // through the Dropbox client and through a directory, and both must read back
    // as the same log — which is only true if the path mapping, the listing, the
    // pagination and the byte handling are all right.
    let fake = FakeDropbox::new();
    let dropbox = DropboxStore::new(&fake, "sl.access");

    let directory = std::env::temp_dir().join(format!("hanzi-sync-dropbox-{}", std::process::id()));
    std::fs::remove_dir_all(&directory).ok();
    let folder = hanzi_sync::FolderStore::open(&directory).unwrap();

    let attempts = vec![
        attempt(1, "好", "2026-09-19T09:00:00Z", 88.0),
        attempt(2, "好", "2026-09-19T21:00:00Z", 91.5),
        attempt(3, "学", "2026-09-21T09:00:00Z", 34.0),
    ];
    write_attempts(&dropbox, "phone", &attempts).unwrap();
    write_attempts(&folder, "phone", &attempts).unwrap();

    let through_dropbox = read_attempts(&dropbox).unwrap();
    let through_a_folder = read_attempts(&folder).unwrap();
    assert_eq!(through_dropbox, through_a_folder);
    assert_eq!(through_dropbox, attempts, "and both are the log that went in");

    std::fs::remove_dir_all(&directory).ok();
}
