//! Cross-device sync, as the app drives it (ROADMAP M13).
//!
//! Everything about *whether two devices agree* lives in `hanzi-sync`, which is
//! tested without a network. What is left for this module is the part that is
//! genuinely the app's: where the account's refresh token is kept, what the learner
//! is told, and the four things the settings screen can ask for.
//!
//! ## The refresh token, which is the whole security story here
//!
//! It does not expire on its own and it is the entire account access this app has.
//! It therefore goes in the platform's secret store — Apple's Keychain here, via
//! `security-framework` — and **not** in `hanzi.db`, which is an ordinary file in an
//! ordinary directory that a backup tool copies to a second disk and a cloud
//! service. A database is the wrong place for a credential even when the database is
//! perfectly safe for study data; those are not the same claim.
//!
//! On a platform whose secret store is not wired up yet, connecting is **refused**
//! rather than quietly written somewhere less safe. A refusal is a bug report; a
//! plaintext token is a vulnerability nobody notices. macOS and iOS share the
//! Keychain API, so the gap is Windows, Linux and Android.
//!
//! ## Why the store and the HTTP client are injectable
//!
//! Both are trait objects with a production default. That is not ceremony: it is
//! what lets the tests below drive a whole connect-then-sync-then-disconnect pass
//! against a temporary directory and an in-memory token store, with no keychain
//! touched and no socket opened. A test that wrote to the developer's real Keychain
//! would be a test that deletes their account.
//!
//! ## The app key is not a secret
//!
//! It travels in the authorization URL the browser opens, which is exactly why PKCE
//! exists and why no app secret is used anywhere in this app. It is a constant here
//! rather than a build-time secret so a fresh clone builds something that works,
//! with an environment override for a fork that wants its own Dropbox app.

use std::sync::Mutex;

use hanzi_store::Db;
use hanzi_sync::{
    authorize_url, exchange_code, refresh, revoke, sync, DropboxStore, Http, Pkce, RemoteStore,
    SyncError, Tokens, UreqHttp,
};
use serde::{Deserialize, Serialize};

/// The Dropbox app key.
///
/// Public by design — see the module note. Overridable at compile time so that a
/// fork can point at its own Dropbox app without editing source.
pub const APP_KEY: &str = match option_env!("HANZI_DROPBOX_APP_KEY") {
    Some(key) => key,
    None => "q48frogetl52kp9",
};

/// What is remembered about a connected account.
///
/// Deliberately not the whole [`Tokens`]: the access token lasts about four hours
/// and is not worth writing to a keychain, so only what must survive is stored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The long-lived token, and the reason this type is never logged.
    pub refresh_token: String,
    /// Dropbox's `account_id`, so the screen can say *which* account is connected.
    #[serde(default)]
    pub account_id: Option<String>,
}

/// One sync's outcome, in the shape the settings screen reads.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSummaryView {
    pub published: usize,
    pub pulled: usize,
    pub recomputed: usize,
    pub left_alone: usize,
}

/// What the settings screen shows.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncView {
    /// Whether an account is connected.
    pub connected: bool,
    /// Which account, when Dropbox said.
    pub account_id: Option<String>,
    /// Whether this platform has a secret store to connect with at all.
    ///
    /// The screen needs to know before it offers the button, so that a learner on a
    /// platform without one is told why rather than watching a button fail.
    pub can_connect: bool,
    /// The last sync this session, if there has been one.
    pub last: Option<SyncSummaryView>,
    /// A sentence for the screen: what just happened, or what went wrong.
    pub message: String,
}

/// The app's sync service.
pub struct SyncService {
    /// The study database. `None` when the app has nowhere to keep study data, in
    /// which case there is nothing to sync and it says so.
    db: Option<Db>,
    /// Where the refresh token lives. A trait object so the tests can use memory.
    tokens: Box<dyn TokenStore>,
    /// How the token endpoints are reached. A trait object for the same reason.
    http: Box<dyn Http>,
    /// Between the browser opening and the code coming back.
    pending: Mutex<Option<Pkce>>,
    /// The last thing that happened, for the screen.
    last: Mutex<Option<SyncSummaryView>>,
}

impl SyncService {
    /// A sync service over the study database, using this platform's secret store.
    pub fn new(db: Option<Db>) -> Self {
        Self::with_parts(db, platform_store(), Box::new(UreqHttp::new()))
    }

    /// The same, with the store and the client supplied.
    fn with_parts(db: Option<Db>, tokens: Box<dyn TokenStore>, http: Box<dyn Http>) -> Self {
        Self {
            db,
            tokens,
            http,
            pending: Mutex::new(None),
            last: Mutex::new(None),
        }
    }

    /// The status the screen shows.
    pub fn view(&self) -> SyncView {
        let account = self.tokens.load().ok().flatten();
        SyncView {
            connected: account.is_some(),
            account_id: account.and_then(|account| account.account_id),
            can_connect: self.tokens.available(),
            last: self.last.lock().unwrap_or_else(|e| e.into_inner()).clone(),
            message: self.status_message(),
        }
    }

    /// Start an authorization: the URL to open, and the verifier to keep.
    ///
    /// The verifier stays in this process and is never shown to the learner, which
    /// is what makes the pasted code useless to anybody who sees it.
    pub fn begin(&self) -> Result<String, String> {
        if !self.tokens.available() {
            return Err(no_secure_store());
        }
        let pkce = Pkce::generate();
        // No `state`: it guards a callback, and this flow has none — the code is
        // typed in rather than redirected back, so there is no request to forge.
        let url = authorize_url(APP_KEY, &pkce, None);
        *self.pending.lock().unwrap_or_else(|e| e.into_inner()) = Some(pkce);
        Ok(url)
    }

    /// Finish an authorization with the code the learner pasted.
    pub fn finish(&self, code: &str) -> Result<SyncView, String> {
        let pkce = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .ok_or_else(|| {
                "There is no sign-in in progress. Press Connect to start one.".to_string()
            })?;

        let tokens = exchange_code(self.http.as_ref(), APP_KEY, code.trim(), &pkce.verifier)
            .map_err(|e| describe(&e))?;
        let account = Account {
            refresh_token: tokens.refresh_token().map_err(|e| describe(&e))?.to_string(),
            account_id: tokens.account_id.clone(),
        };
        self.tokens.save(&account)?;
        Ok(self.after("Connected to Dropbox."))
    }

    /// Forget the account, on this device and on Dropbox's side.
    pub fn disconnect(&self) -> SyncView {
        // Revoking first is the point: forgetting the token locally would leave the
        // authorization standing on Dropbox's side, which is not what "disconnect"
        // means to somebody who pressed it. A failure to revoke is reported rather
        // than swallowed, but it does not stop the local token being removed.
        let unreported = match self.tokens.load() {
            Ok(Some(account)) => self
                .fresh_access_token(&account)
                .and_then(|token| revoke(self.http.as_ref(), &token).map_err(|e| describe(&e)))
                .err()
                .map(|why| format!(" (Dropbox was not told: {why})")),
            _ => None,
        };

        let cleared = self.tokens.clear();
        *self.last.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let message = match (cleared, unreported) {
            (Ok(()), None) => "Disconnected. Your study data is untouched.".to_string(),
            (Ok(()), Some(why)) => format!("Disconnected{why}."),
            (Err(why), _) => format!("Could not clear the stored sign-in: {why}"),
        };
        SyncView {
            connected: false,
            account_id: None,
            can_connect: self.tokens.available(),
            last: None,
            message,
        }
    }

    /// Sync now, over Dropbox.
    pub fn now(&self) -> Result<SyncView, String> {
        let account = self.account()?;
        let token = self.fresh_access_token(&account)?;
        let remote = DropboxStore::new(self.http.as_ref(), token);
        self.run(&remote, account.account_id)
    }

    /// Sync now, over whatever store is handed in.
    ///
    /// Split from [`Self::now`] so the pass itself is testable against a directory.
    /// Everything below this line is transport-independent, which is the whole
    /// reason the seam exists.
    fn run(&self, remote: &dyn RemoteStore, account_id: Option<String>) -> Result<SyncView, String> {
        let Some(db) = &self.db else {
            return Err("This build has no study database, so there is nothing to sync.".into());
        };
        let summary = sync(db, remote).map_err(|e| describe(&e))?;
        let view = SyncSummaryView {
            published: summary.published,
            pulled: summary.pulled,
            recomputed: summary.recomputed,
            left_alone: summary.left_alone,
        };
        *self.last.lock().unwrap_or_else(|e| e.into_inner()) = Some(view.clone());
        Ok(SyncView {
            connected: true,
            account_id,
            can_connect: self.tokens.available(),
            last: Some(view),
            message: describe_summary(&summary),
        })
    }

    /// The connected account, or a sentence saying there is none.
    fn account(&self) -> Result<Account, String> {
        self.tokens
            .load()?
            .ok_or_else(|| "No Dropbox account is connected.".to_string())
    }

    /// A fresh access token for an account.
    ///
    /// Fetched per sync rather than held: it lasts about four hours, and a learner
    /// may leave the app open for longer than that.
    fn fresh_access_token(&self, account: &Account) -> Result<String, String> {
        let tokens: Tokens =
            refresh(self.http.as_ref(), APP_KEY, &account.refresh_token).map_err(|e| describe(&e))?;
        Ok(tokens.access_token)
    }

    /// The view after a change, with a fresh message.
    fn after(&self, message: &str) -> SyncView {
        let account = self.tokens.load().ok().flatten();
        SyncView {
            connected: account.is_some(),
            account_id: account.and_then(|account| account.account_id),
            can_connect: self.tokens.available(),
            last: self.last.lock().unwrap_or_else(|e| e.into_inner()).clone(),
            message: message.to_string(),
        }
    }

    /// What to say when nothing has just happened.
    fn status_message(&self) -> String {
        if !self.tokens.available() {
            return no_secure_store();
        }
        match self.tokens.load() {
            Ok(Some(_)) => "Connected to Dropbox.".to_string(),
            Ok(None) => {
                "Not connected. Your study data stays on this device until you connect."
                    .to_string()
            }
            Err(why) => format!("The keychain could not be read: {why}"),
        }
    }
}

/// A sentence for the learner out of a sync failure.
///
/// The one case worth naming differently is an expired token, because its next move
/// differs from every other failure's: reconnect, rather than try again.
fn describe(error: &SyncError) -> String {
    match error {
        SyncError::Unauthorized => {
            "Dropbox refused the sign-in. Disconnect and connect again.".to_string()
        }
        other => other.to_string(),
    }
}

/// What one sync did, in a sentence.
fn describe_summary(summary: &hanzi_sync::Summary) -> String {
    if summary.is_empty() {
        let mut message = "Already up to date.".to_string();
        if summary.left_alone > 0 {
            message.push_str(&format!(
                " {} character{} kept a schedule this device cannot rebuild from its log yet.",
                summary.left_alone,
                if summary.left_alone == 1 { "" } else { "s" }
            ));
        }
        return message;
    }
    let mut parts = Vec::new();
    if summary.published > 0 {
        parts.push(format!("sent {}", plural(summary.published, "attempt")));
    }
    if summary.pulled > 0 {
        parts.push(format!("received {}", plural(summary.pulled, "attempt")));
    }
    if summary.recomputed > 0 {
        parts.push(format!("updated {}", plural(summary.recomputed, "schedule")));
    }
    format!("Synced: {}.", parts.join(", "))
}

/// "1 attempt", "2 attempts".
fn plural(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

/// The keychain could not be used because this platform has none wired up.
fn no_secure_store() -> String {
    format!(
        "This build has no secure store for the Dropbox sign-in on {}, so connecting is \
         switched off rather than keeping the token somewhere it could be read. Your study \
         data is unaffected.",
        std::env::consts::OS
    )
}

// ---- where the token actually lives ----------------------------------------

/// What a platform's secret store has to do.
///
/// Four methods, and no notion of a session, because that is all this app needs and
/// every extra method is another thing a platform can get subtly wrong.
trait TokenStore: Send + Sync {
    /// Whether this platform can keep a secret at all.
    fn available(&self) -> bool;
    fn load(&self) -> Result<Option<Account>, String>;
    fn save(&self, account: &Account) -> Result<(), String>;
    fn clear(&self) -> Result<(), String>;
}

/// The secret store for the platform this build is for.
fn platform_store() -> Box<dyn TokenStore> {
    #[cfg(target_vendor = "apple")]
    {
        Box::new(Keychain)
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        Box::new(NoSecureStore)
    }
}

/// Apple's Keychain, which is the same API on macOS and iOS.
///
/// One generic password, named by a service and an account. The service string is
/// what a person sees in Keychain Access, so it says what it is.
#[cfg(target_vendor = "apple")]
struct Keychain;

#[cfg(target_vendor = "apple")]
const KEYCHAIN_SERVICE: &str = "HanziTutor cross-device sync";
#[cfg(target_vendor = "apple")]
const KEYCHAIN_ACCOUNT: &str = "dropbox";
/// `errSecItemNotFound`, from `Security/SecBase.h`: "no keychain item was found".
#[cfg(target_vendor = "apple")]
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

#[cfg(target_vendor = "apple")]
impl TokenStore for Keychain {
    fn available(&self) -> bool {
        true
    }

    fn load(&self) -> Result<Option<Account>, String> {
        let found =
            security_framework::passwords::get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);
        match found {
            Ok(bytes) => {
                let text = String::from_utf8(bytes)
                    .map_err(|_| "the stored Dropbox sign-in is not readable text".to_string())?;
                serde_json::from_str(&text)
                    .map(Some)
                    .map_err(|e| format!("the stored Dropbox sign-in could not be read: {e}"))
            }
            // No item yet is not a failure; it is a device nobody has connected.
            Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn save(&self, account: &Account) -> Result<(), String> {
        let text = serde_json::to_string(account)
            .map_err(|e| format!("the Dropbox sign-in could not be encoded: {e}"))?;
        security_framework::passwords::set_generic_password(
            KEYCHAIN_SERVICE,
            KEYCHAIN_ACCOUNT,
            text.as_bytes(),
        )
        .map_err(|e| format!("the keychain refused to store the Dropbox sign-in: {e}"))
    }

    fn clear(&self) -> Result<(), String> {
        let removed =
            security_framework::passwords::delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);
        match removed {
            Ok(()) => Ok(()),
            Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// The platform has no secret store wired up, so connecting is refused.
///
/// Not a stub waiting to be filled in with a file: see the module note. A refusal is
/// a bug report; a plaintext credential is a vulnerability nobody notices.
#[cfg(not(target_vendor = "apple"))]
struct NoSecureStore;

#[cfg(not(target_vendor = "apple"))]
impl TokenStore for NoSecureStore {
    fn available(&self) -> bool {
        false
    }
    fn load(&self) -> Result<Option<Account>, String> {
        Ok(None)
    }
    fn save(&self, _account: &Account) -> Result<(), String> {
        Err(no_secure_store())
    }
    fn clear(&self) -> Result<(), String> {
        Ok(())
    }
}

// ---- tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// A secret store that keeps its one secret in memory.
    ///
    /// The point of the tests below is that they never touch the real Keychain: one
    /// that wrote to the developer's would be one that deleted their account.
    #[derive(Default)]
    struct MemoryStore {
        account: Mutex<Option<Account>>,
        available: bool,
    }

    impl MemoryStore {
        fn working() -> Self {
            Self {
                account: Mutex::new(None),
                available: true,
            }
        }

        fn unavailable() -> Self {
            Self {
                account: Mutex::new(None),
                available: false,
            }
        }
    }

    impl TokenStore for MemoryStore {
        fn available(&self) -> bool {
            self.available
        }
        fn load(&self) -> Result<Option<Account>, String> {
            Ok(self.account.lock().unwrap().clone())
        }
        fn save(&self, account: &Account) -> Result<(), String> {
            *self.account.lock().unwrap() = Some(account.clone());
            Ok(())
        }
        fn clear(&self) -> Result<(), String> {
            *self.account.lock().unwrap() = None;
            Ok(())
        }
    }

    /// An HTTP client that answers the token endpoints and records what it was
    /// asked, so the OAuth calls can be driven without a socket.
    #[derive(Default)]
    struct FakeHttp {
        forms: Mutex<Vec<String>>,
        revocations: Mutex<Vec<String>>,
        token_response: Mutex<Option<String>>,
    }

    impl FakeHttp {
        fn answering(body: serde_json::Value) -> Self {
            Self {
                token_response: Mutex::new(Some(body.to_string())),
                ..Self::default()
            }
        }
    }

    impl Http for FakeHttp {
        fn rpc(&self, _url: &str, bearer: &str, _body: &str) -> Result<String, SyncError> {
            self.revocations.lock().unwrap().push(bearer.to_string());
            Ok("null".to_string())
        }
        fn form(&self, _url: &str, body: &str) -> Result<String, SyncError> {
            self.forms.lock().unwrap().push(body.to_string());
            self.token_response
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SyncError::Io("the fake was not given a token response".into()))
        }
        fn upload(&self, _url: &str, _bearer: &str, _arg: &str, _body: &[u8]) -> Result<String, SyncError> {
            unreachable!("the token tests never upload")
        }
        fn download(&self, _url: &str, _bearer: &str, _arg: &str) -> Result<Vec<u8>, SyncError> {
            unreachable!("the token tests never download")
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("hanzi-sync-service-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn a_view_says_whether_there_is_a_secret_store_and_an_account() {
        let view = SyncService::with_parts(
            None,
            Box::new(MemoryStore::working()),
            Box::new(FakeHttp::default()),
        )
        .view();
        assert!(view.can_connect, "this platform can keep a secret");
        assert!(!view.connected, "but nobody has connected");
        assert!(view.last.is_none());
        assert!(view.message.contains("stays on this device"), "{}", view.message);

        // A platform with no secret store says so rather than offering a button.
        let unavailable = SyncService::with_parts(
            None,
            Box::new(MemoryStore::unavailable()),
            Box::new(FakeHttp::default()),
        );
        assert!(!unavailable.view().can_connect);
        let refusal = unavailable.begin().unwrap_err();
        assert!(refusal.contains("no secure store"), "{refusal}");
        assert!(refusal.contains("unaffected"), "and it should reassure: {refusal}");
    }

    #[test]
    fn beginning_an_authorization_returns_a_url_to_open() {
        let service = SyncService::with_parts(
            None,
            Box::new(MemoryStore::working()),
            Box::new(FakeHttp::default()),
        );
        let url = service.begin().unwrap();
        assert!(url.starts_with("https://www.dropbox.com/oauth2/authorize?"), "{url}");
        assert!(url.contains(&format!("client_id={APP_KEY}")), "{url}");
        assert!(url.contains("token_access_type=offline"), "{url}");
        assert!(url.contains("code_challenge_method=S256"), "{url}");
        assert!(!url.contains("client_secret"), "{url}");

        // Two authorizations do not reuse a verifier.
        assert_ne!(url, service.begin().unwrap());
    }

    #[test]
    fn a_code_is_only_accepted_while_an_authorization_is_in_progress() {
        // Typing a code into a stale screen, or pasting twice, must not trade
        // somebody else's code with this app's verifier.
        let service = SyncService::with_parts(
            None,
            Box::new(MemoryStore::working()),
            Box::new(FakeHttp::default()),
        );
        let error = service.finish("some-code").unwrap_err();
        assert!(error.contains("no sign-in in progress"), "{error}");
    }

    #[test]
    fn the_app_key_is_the_one_dropbox_knows_and_is_not_a_secret() {
        // The key is public by design — it travels in the URL above — so this is a
        // guard against it being replaced with something that is not.
        assert_eq!(APP_KEY, "q48frogetl52kp9");
        assert!(!APP_KEY.contains("secret"), "{APP_KEY}");
    }

    #[test]
    fn a_summary_becomes_a_sentence() {
        let busy = hanzi_sync::Summary {
            published: 3,
            pulled: 2,
            recomputed: 1,
            left_alone: 0,
        };
        assert_eq!(describe_summary(&busy), "Synced: sent 3 attempts, received 2 attempts, updated 1 schedule.");
        assert_eq!(plural(1, "attempt"), "1 attempt");
        assert_eq!(plural(2, "attempt"), "2 attempts");

        let quiet = hanzi_sync::Summary::default();
        assert_eq!(describe_summary(&quiet), "Already up to date.");

        let conservative = hanzi_sync::Summary {
            left_alone: 1,
            ..hanzi_sync::Summary::default()
        };
        let message = describe_summary(&conservative);
        assert!(message.contains("Already up to date"), "{message}");
        assert!(message.contains("1 character "), "{message}");
        assert!(!message.contains("characters"), "one character is singular: {message}");
    }

    #[test]
    fn an_expired_sign_in_is_told_to_reconnect_rather_than_retry() {
        let message = describe(&SyncError::Unauthorized);
        assert!(message.contains("Disconnect and connect again"), "{message}");
    }

    #[test]
    fn a_whole_connect_then_sync_then_disconnect_pass() {
        // The app's service end to end, with no keychain and no socket: connect
        // against a fake token endpoint, sync over a real directory, disconnect.
        let dir = scratch("pass");
        let db = Db::open(&dir).unwrap();
        let shared = scratch("pass-store");
        let remote = hanzi_sync::FolderStore::open(&shared).unwrap();

        let http = Box::new(FakeHttp::answering(serde_json::json!({
            "access_token": "sl.access",
            "refresh_token": "refresh-me",
            "account_id": "dbid:AAAA"
        })));
        let service = SyncService::with_parts(
            Some(db.clone()),
            Box::new(MemoryStore::working()),
            http,
        );

        // Nothing is connected to begin with.
        assert!(!service.view().connected);
        assert!(service.now().is_err(), "there is no account to sync with yet");

        // Connect: the code is exchanged and the refresh token is stored.
        service.begin().unwrap();
        let connected = service.finish("the-code").unwrap();
        assert!(connected.connected, "{}", connected.message);
        assert_eq!(connected.account_id.as_deref(), Some("dbid:AAAA"));
        assert_eq!(
            service.tokens.load().unwrap().unwrap().refresh_token,
            "refresh-me",
            "and the long-lived token is what was kept"
        );

        // Practise, then sync over the directory.
        {
            let mut progress =
                hanzi_core::ProgressStore::open_with(Box::new(db.clone())).unwrap();
            progress.record_at('好', 88.0, "2026-09-19T09:00:00Z").unwrap();
            progress.save().unwrap();
        }
        let synced = service.run(&remote, Some("dbid:AAAA".to_string())).unwrap();
        assert!(synced.connected);
        assert!(synced.message.starts_with("Synced: sent 1 attempt"), "{}", synced.message);
        assert_eq!(synced.last.unwrap().published, 1);

        // A second pass over the same store has nothing to do.
        let again = service.run(&remote, None).unwrap();
        assert_eq!(again.message, "Already up to date.");

        // Disconnect revokes, then clears.
        let gone = service.disconnect();
        assert!(!gone.connected);
        assert!(gone.message.contains("Disconnected"), "{}", gone.message);
        assert!(service.tokens.load().unwrap().is_none(), "the token is gone");

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&shared).ok();
    }

    #[test]
    fn a_service_without_a_database_says_so_instead_of_pretending() {
        let service = SyncService::with_parts(
            None,
            Box::new(MemoryStore::working()),
            Box::new(FakeHttp::default()),
        );
        let error = service.run(&hanzi_sync::FolderStore::open(scratch("nodir")).unwrap(), None).unwrap_err();
        assert!(error.contains("no study database"), "{error}");
    }

    #[test]
    fn the_view_serialises_the_way_the_interface_reads_it() {
        let view = SyncService::with_parts(
            None,
            Box::new(MemoryStore::working()),
            Box::new(FakeHttp::default()),
        )
        .view();
        let json = serde_json::to_value(&view).unwrap();
        let keys: BTreeMap<&str, &serde_json::Value> = json.as_object().unwrap().iter().map(|(k, v)| (k.as_str(), v)).collect();
        for expected in ["connected", "accountId", "canConnect", "last", "message"] {
            assert!(keys.contains_key(expected), "{expected} missing from {json}");
        }
        // The summary inside it too, because the screen reads those names as well.
        let summary = serde_json::to_value(SyncSummaryView {
            published: 1,
            pulled: 2,
            recomputed: 3,
            left_alone: 4,
        })
        .unwrap();
        let keys: BTreeMap<&str, &serde_json::Value> = summary.as_object().unwrap().iter().map(|(k, v)| (k.as_str(), v)).collect();
        for expected in ["published", "pulled", "recomputed", "leftAlone"] {
            assert!(keys.contains_key(expected), "{expected} missing from {summary}");
        }
    }
}
