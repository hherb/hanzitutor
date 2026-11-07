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
//! Keychain API, so the gap is Windows and Linux.
//!
//! ## Drawing the screen must not unlock anything
//!
//! This is the rule the rest of the module is arranged around, and it was learned
//! the hard way. A keychain item can be created behind an access control that
//! requires *user presence*, and such an item asks for a fingerprint, a face or the
//! device password on **every single read** — there is no "always allow" for an
//! item built that way. Reading the token to answer "is this device connected?" is
//! therefore a way to prompt somebody for their fingerprint merely for opening
//! Settings; reading it twice inside one sync is a way to prompt them three times
//! for one button press; and a sync that starts by itself at launch would prompt
//! every launch, at a moment nobody chose.
//!
//! So two things are kept apart:
//!
//! - **The credential**, which is the refresh token, in the platform's secret store.
//!   It is read only when something is actually about to use it — a sync, a
//!   disconnect, or a change of protection — and at most **once per run of the
//!   app**, because the answer is cached in [`Open`] for the life of the process.
//! - **A record of what is connected**, which is not a secret: Dropbox's
//!   `account_id` and how well the token is protected. It lives in the database's
//!   `meta` table beside the publish watermark, which is where sync already keeps
//!   its bookkeeping, and it is what [`SyncService::view`] draws from. That call
//!   therefore never touches the keychain — except once, to adopt a sign-in stored
//!   by a build that predates the record. See [`SyncService::record`].
//!
//! ## Why the sign-in asks for nothing by default
//!
//! An item with a user-presence constraint is a real second factor, and it is also
//! the wrong default here: it asks on every read, which for a feature meant to run
//! by itself is friction at the one moment the learner is not paying attention. The
//! default is therefore an item with **no constraint at all** — released to this
//! app, on this device, and unreadable at rest without the device — and asking for a
//! fingerprint is a switch the learner can turn on, in which case it is asked for
//! exactly when the token is about to be used and never merely to draw a screen.
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
//!
//! ## A position that moves does not wait for the next sync
//!
//! The one thing here that is not a sync. A drill writes where it got to in a group
//! after every entry, and a position is the smallest useful thing this app has: it
//! is what makes picking up the other device continue the lesson rather than
//! restart it. Left to the sync at launch and on foreground, a drill finished on the
//! phone was simply not on the laptop until somebody pressed *Sync now* — which
//! reads as the feature being broken rather than as a delay.
//!
//! So [`SyncService::publish_positions_soon`] sends that one document when it
//! changes: on a thread, without the gate, and under [`Self::auto`]'s three
//! refusals rather than a fourth set of rules. It pulls nothing and rebuilds
//! nothing, so it cannot be a second sync by accident.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

use hanzi_store::Db;
use hanzi_sync::{
    authorize_url, exchange_code, refresh, revoke, sync, write_vocab_cursors, DropboxStore, Http,
    Pkce, Reach, RemoteStore, SyncError, TcpReach, Tokens, UreqHttp,
};
use serde::{Deserialize, Serialize};

#[cfg(target_vendor = "apple")]
use security_framework::access_control::{ProtectionMode, SecAccessControl};
#[cfg(target_vendor = "apple")]
use security_framework::passwords_options::{AccessControlOptions, PasswordOptions};

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

/// Where in the database's `meta` table the non-secret half is written.
///
/// Beside `sync:published_seq`, and for the reason given on `Db::meta_value`: sync
/// keeps its bookkeeping in the one file a backup captures. Its value is a
/// [`Stored`] as JSON.
const ACCOUNT_KEY: &str = "sync:account";

/// Where the learner's answer to "ask for my fingerprint?" is written.
///
/// A key of its own rather than a field of [`Stored`], because the two have
/// different lifetimes: disconnecting forgets the account and must not forget that
/// the learner wants a fingerprint the next time they connect.
const LOCK_KEY: &str = "sync:lock";

/// Set when a disconnect removed every copy of the sign-in a read can reach but a
/// copy in the login keychain would not go — removing one another build wrote can
/// ask for the keychain password, and the learner may cancel.
///
/// With no record, a launch looks for a sign-in to adopt everywhere, the login
/// keychain included, and would find that copy: a keychain-password prompt at a
/// moment nobody chose, for a token Dropbox has just been told to forget. While
/// this is set, adoption looks only where a copy a read could reach would be
/// ([`TokenStore::place_past_leftovers`]). A connection that is recorded clears it.
const LEFTOVER_KEY: &str = "sync:leftover";

/// How long an access token fetched for a publish is reused.
///
/// A full sync fetches one every time, because it may be the first thing to happen
/// in a session that outlives the token. A publish happens once per finished entry
/// in a drill — a handful of seconds apart — and one token exchange per character
/// written would be silly, so this one is kept for a while and no longer. Dropbox's
/// access tokens last about four hours, so half an hour is comfortably inside that
/// and still refreshes within any session long enough for it to matter.
const PUBLISH_TOKEN_REUSE: Duration = Duration::from_secs(30 * 60);

/// Everything about a connected account that is **not** a secret.
///
/// The presence of this record is what "connected" means, and it is why drawing the
/// screen costs no keychain read: a refresh token that is worth protecting tells the
/// screen nothing it needs, and the three things it does need — whether there is a
/// sign-in, whose it is, and how well it is protected — are not worth protecting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    /// Dropbox's `account_id`, when Dropbox said.
    #[serde(default)]
    account_id: Option<String>,
    /// How the token in the secret store actually ended up protected, which is not
    /// always what was asked for: see [`TokenStore::save`].
    protection: Protection,
    /// Where the store put the item this record describes, for a store that has
    /// more than one place to keep it — Apple's keychain, see `Slot`. Handed back
    /// to every read and write, so that a write cut short between adding the new
    /// item and removing the old one still reads the item this record describes:
    /// the record is written only once a write has finished. Absent in a record
    /// written before there was a second place, and for a store with one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    place: Option<String>,
}

/// One sync's outcome, in the shape the settings screen reads.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSummaryView {
    pub published: usize,
    pub pulled: usize,
    pub recomputed: usize,
    pub left_alone: usize,
    /// Vocabulary entries and groups whose stored form changed.
    pub vocab_changed: usize,
    /// Whether the place in the course moved on this device.
    pub cursor_moved: bool,
}

/// How well the stored sign-in is protected.
///
/// The screen shows this, and it is not a decoration. Which of these a device gets
/// depends on what the platform can do, on whether the learner asked for a
/// fingerprint, and on whether the build is one the system can identify — so the
/// learner is told which one they have rather than left to guess it from a prompt
/// they were not expecting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Protection {
    /// Nothing is stored yet, the platform has no secret store at all, or — the case
    /// only a *read* can produce — there is an item but this store cannot say how it
    /// is protected. Apple's data-protection keychain is that third case: the password
    /// comes back and the access control on it does not, so an item written with no
    /// constraint and one behind a fingerprint are the same read. The record in `meta`
    /// is what tells them apart, because `save` is what wrote it down.
    #[default]
    Unknown,
    /// In the data-protection keychain with no constraint on it: released to this
    /// app, on this device, and unreadable at rest without the device. Asking for
    /// nothing is what makes a sync that runs by itself possible.
    DeviceOnly,
    /// Behind the data-protection keychain, released only for a fingerprint, a
    /// face, or the device password. Asked for when the token is about to be used,
    /// and never merely to draw the screen.
    UserPresence,
    /// An ordinary login-keychain item, or Android's keystore with a key that
    /// asks for nothing: still encrypted at rest, but released to this app
    /// without asking anybody. This is where an unsigned development build lands,
    /// because the data-protection keychain needs an application identifier that
    /// an ad-hoc signature does not have — and where a phone with no screen lock
    /// lands, because it cannot make a key that asks.
    KeychainOnly,
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
    /// Whether this platform can ask for a fingerprint at all.
    ///
    /// Separate from [`Self::protection`], which says what the item that *is* stored
    /// got: a device with no fingerprint, no face and no screen lock cannot ask, so
    /// the switch is not offered there rather than offered and ignored.
    pub can_lock: bool,
    /// Whether the learner has asked for a fingerprint.
    pub locked: bool,
    /// How the stored sign-in is protected, where one is stored.
    pub protection: Protection,
    /// The last sync this session, if there has been one.
    pub last: Option<SyncSummaryView>,
    /// A sentence for the screen: what just happened, or what went wrong.
    pub message: String,
}

/// What an automatic sync did, or why it did not run.
///
/// Four outcomes rather than a `Result`, because "nothing happened" is the ordinary
/// answer on most launches and the screen has to be able to tell the cases apart:
/// not connected and locked are both **silent** — there is nothing to say and saying
/// it every launch would be noise — offline is worth one calm line, and a failure is
/// worth saying out loud.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum AutoSync {
    /// Nothing was attempted: there is no account, or the sign-in is behind a
    /// fingerprint and a sync that runs by itself cannot satisfy that.
    Skipped { reason: String },
    /// Nothing was attempted because there is no network path. Not a failure: a
    /// phone on a train is the normal case this is written for.
    Offline { reason: String },
    /// A sync ran.
    Synced { view: SyncView },
    /// A sync ran and did not finish.
    Failed { reason: String },
}

/// The sign-in this process has already read, or the fact that it has not read one.
///
/// The third state is the point. `Mutex<Option<Account>>` could not tell "not read
/// yet" from "read, and there is nothing there", and those are different: the first
/// has to go to the secret store and the second must not, or a learner who has never
/// connected would be sent to the keychain every time the screen was drawn.
#[derive(Default)]
enum Open {
    /// Nothing has asked for the token yet.
    #[default]
    Unread,
    /// Read once, and this is the whole of the answer for this run of the app.
    Known(Option<Loaded>),
}

/// The app's sync service.
pub struct SyncService {
    /// The study database. `None` when the app has nowhere to keep study data, in
    /// which case there is nothing to sync and it says so.
    ///
    /// It is also where the non-secret [`Stored`] record and the lock preference
    /// live, which is why they are absent on the same devices.
    db: Option<Db>,
    /// Where the refresh token lives. A trait object so the tests can use memory.
    tokens: Box<dyn TokenStore>,
    /// How the token endpoints are reached. A trait object for the same reason.
    http: Box<dyn Http>,
    /// Between the browser opening and the code coming back.
    pending: Mutex<Option<Pkce>>,
    /// The last thing that happened, for the screen.
    last: Mutex<Option<SyncSummaryView>>,
    /// The token store's answer, read at most once per run. See [`Open`].
    open: Mutex<Open>,
    /// Whether there is a network path, asked before an automatic sync spends
    /// `UreqHttp`'s ten-second connect timeout finding out that there is not.
    reach: Box<dyn Reach>,
    /// One sync at a time.
    ///
    /// The commands in `crate::commands` run off the main thread, which is what lets
    /// the screen paint "Syncing…" while a sync is in flight — and also what makes
    /// two of them possible at once: a sync at launch fires while somebody opens
    /// Settings and presses Sync now. The merge is idempotent, so nothing would be
    /// corrupted; what would be wrong is two passes interleaving their publish and
    /// pull, and a learner told about a sync the other one had already overtaken. A
    /// sync that finds this shut is not a failure: it is a sync with nothing to add.
    gate: Mutex<()>,
    /// One writer of the stored sign-in at a time: connecting, the fingerprint
    /// switch and disconnecting.
    ///
    /// Each of those writes the item and the record of how it is protected, and the
    /// switch writes the preference as well. They all run off the main thread
    /// (issue #26), so two can overlap — a second press of the switch while the
    /// first is still rewriting the item — and two that interleave can leave the
    /// switch saying "off" over an item that asks for a fingerprint at every launch.
    /// Held across the keychain write, which can be a prompt waiting on a person,
    /// and never by a sync, which only reads.
    switching: Mutex<()>,
    /// An access token fetched for a publish, and when it was fetched.
    ///
    /// The one piece of state that exists for the change-publish path. See
    /// [`PUBLISH_TOKEN_REUSE`] and [`SyncService::publish_token`].
    publish_token: Mutex<Option<(String, Instant)>>,
    /// Whether a position has moved that a publish has not written yet.
    ///
    /// Together with [`Self::publishing_positions`] this is the coalescer described
    /// on [`SyncService::publish_positions_soon`].
    positions_owed: AtomicBool,
    /// Whether a thread is already draining [`Self::positions_owed`].
    publishing_positions: AtomicBool,
}

impl SyncService {
    /// A sync service over the study database, using this platform's secret store.
    pub fn new(db: Option<Db>) -> Self {
        Self::with_parts(
            db,
            platform_store(),
            Box::new(UreqHttp::new()),
            Box::new(TcpReach::dropbox()),
        )
    }

    /// The same, with the store, the client and the probe supplied.
    fn with_parts(
        db: Option<Db>,
        tokens: Box<dyn TokenStore>,
        http: Box<dyn Http>,
        reach: Box<dyn Reach>,
    ) -> Self {
        Self {
            db,
            tokens,
            http,
            pending: Mutex::new(None),
            last: Mutex::new(None),
            open: Mutex::new(Open::Unread),
            reach,
            gate: Mutex::new(()),
            switching: Mutex::new(()),
            publish_token: Mutex::new(None),
            positions_owed: AtomicBool::new(false),
            publishing_positions: AtomicBool::new(false),
        }
    }

    /// The gate, taken for the whole of a pass.
    fn take_gate(&self) -> Result<std::sync::MutexGuard<'_, ()>, String> {
        try_hold(&self.gate).ok_or_else(|| {
            "A sync is already running. Its result will appear here when it finishes.".to_string()
        })
    }

    /// The right to write the stored sign-in. See [`Self::switching`].
    fn take_switch(&self) -> std::sync::MutexGuard<'_, ()> {
        self.switching.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The status the screen shows.
    ///
    /// Reads the record, not the secret store — see the module note. The one
    /// exception is the adoption path in [`Self::record`], which runs only on a
    /// device whose sign-in predates the record and only once.
    pub fn view(&self) -> SyncView {
        let record = self.record();
        let can_lock = self.tokens.can_lock();
        let locked = self.locked();
        let message = self.status_message(record.as_ref(), can_lock, locked);
        SyncView {
            connected: record.is_some(),
            account_id: record.as_ref().and_then(|r| r.account_id.clone()),
            can_connect: self.tokens.available(),
            can_lock,
            locked,
            protection: record.map(|r| r.protection).unwrap_or_default(),
            last: self.last.lock().unwrap_or_else(|e| e.into_inner()).clone(),
            message,
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
    ///
    /// The verifier is copied out rather than taken, and given up only once Dropbox
    /// has accepted a code for it. A code it refuses — a typo, or one from an earlier
    /// Connect tab, which `begin` has since replaced — spends nothing, and the code
    /// from the tab the learner is looking at must still work after it; taking the
    /// verifier first answered every later paste with "no sign-in in progress".
    pub fn finish(&self, code: &str) -> Result<SyncView, String> {
        let pkce = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or_else(|| {
                "There is no sign-in in progress. Press Connect to start one.".to_string()
            })?;

        let tokens = exchange_code(self.http.as_ref(), APP_KEY, code.trim(), &pkce.verifier)
            .map_err(|e| describe(&e))?;
        // Accepted, so the code is spent and this authorization with it: whatever
        // happens next, a retry needs a new one. Only if it is still the one this
        // used — a Connect pressed during the exchange started another, and that one
        // is the learner's now.
        {
            let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            if pending
                .as_ref()
                .is_some_and(|p| p.verifier == pkce.verifier)
            {
                *pending = None;
            }
        }
        let account = Account {
            refresh_token: tokens.refresh_token().map_err(|e| describe(&e))?.to_string(),
            account_id: tokens.account_id.clone(),
        };

        // Taken after the exchange rather than before it, so the switch does not
        // wait out a network round trip; what it guards is the preference read
        // here and the write that follows it.
        let _switching = self.take_switch();
        let recorded = self.read_record();
        let place = self.settled_place();
        // The preference, not the default: a learner who asked for a fingerprint
        // gets one from the first read, not from the second connection.
        let saved = match self.tokens.save(&account, self.locked(), place.as_deref()) {
            Ok(saved) => saved,
            Err(why) => {
                // A save that failed has left what the record describes, and with no
                // record that is nothing. A store that could not take back a new item
                // it had already added — Apple's, when a removal and then its rollback
                // are both refused — would otherwise leave this token, about to be
                // revoked, for the next launch to adopt as a connection.
                // Only when there is definitely none: a record the database could not
                // read, or one it holds that this build cannot parse, may describe a
                // sign-in that this must not wipe.
                if matches!(recorded, Ok(None)) {
                    let _ = self.tokens.clear();
                }
                // The code was single-use and is spent, so this sign-in can never be
                // stored now — on Android, locking it asks for the fingerprint, and a
                // learner may cancel. A refresh token kept nowhere is one nobody can
                // revoke later, so Dropbox is told to forget it now, as far as it
                // can be; a failure to tell it changes nothing the learner can do.
                let _ = revoke(self.http.as_ref(), &tokens.access_token);
                return Err(format!("Not connected: {why}. Press Connect to try again."));
            }
        };
        let protection = saved.protection;
        self.remember(Some(Loaded {
            account: account.clone(),
            protection,
            place: saved.place.clone(),
        }));
        // Whichever account was connected before, this is not it: a token fetched
        // for a publish under the old one must not be used under the new one.
        self.forget_publish_token();

        // Written down before the screen is told, and the token given back up if it
        // cannot be: a refresh token the screen does not know about is one the
        // learner cannot disconnect, which is worse than not being connected.
        if let Err(why) = self.write_record(&Stored {
            account_id: account.account_id,
            protection,
            place: saved.place,
        }) {
            let _ = self.tokens.clear();
            self.remember(None);
            return Err(format!(
                "The Dropbox sign-in could not be recorded on this device, so it has been removed \
                 again rather than left where nothing could reach it: {why}"
            ));
        }
        // A connection recorded is one adoption no longer has to be careful around.
        self.forget_leftover();
        // A learner who asked for a fingerprint and did not get one is told so here
        // as well as on the switch. The store only does that when the device cannot
        // ask at all — a screen lock removed since the switch was turned on — and
        // the switch is not offered then, so this sentence is the only place the
        // learner would find out.
        if self.locked() && protection == Protection::KeychainOnly {
            return Ok(self.after(&format!("Connected to Dropbox. {}", lock_unavailable())));
        }
        Ok(self.after("Connected to Dropbox."))
    }

    /// Ask for a fingerprint before the token is read, or stop asking for one.
    ///
    /// Rewrites the stored item when there is one, because the constraint is fixed
    /// when a keychain item is created and cannot be changed afterwards.
    pub fn set_lock(&self, locked: bool) -> Result<SyncView, String> {
        // For the whole of it: the preference below and the item rewritten after it
        // must be one press's answer, not two presses' halves.
        let _switching = self.take_switch();
        if locked && !self.tokens.can_lock() {
            return Err(no_biometric());
        }

        // Reading it is the point rather than a side effect: turning the prompt off
        // has to read the item it is about to rewrite, and that read is the last
        // time the learner is asked for anything.
        if let Some((account, _)) = self.sign_in()? {
            // A rewrite can be refused — on Android, locking asks for the fingerprint,
            // and a learner may cancel — and the store then keeps the item as it was.
            let saved = self
                .tokens
                .save(&account, locked, self.settled_place().as_deref())
                .map_err(|why| {
                    format!("{why}. The sign-in is kept as it was, and the switch is unchanged.")
                })?;
            let protection = saved.protection;
            let place = saved.place.clone();
            // The cache as well as the record, before anything else can fail: they
            // describe the item, which has already changed. The cache is what decides
            // whether a launch may read the sign-in, and a switch flipped to *on* that
            // left it saying "nothing to unlock" would be a prompt at the next launch —
            // which is the one thing this whole arrangement exists to prevent.
            self.remember(Some(Loaded {
                account: account.clone(),
                protection,
                place: place.clone(),
            }));
            let recorded = self.write_record(&Stored {
                account_id: account.account_id,
                protection,
                place,
            });
            // The preference only once the item agrees with it, so a refused rewrite
            // leaves the switch where it was rather than claiming a lock the sign-in
            // does not have.
            self.write_lock(locked)?;
            recorded?;
            let misplaced = locked && protection == Protection::KeychainOnly;
            let message = if misplaced {
                lock_unavailable()
            } else if locked {
                "The sign-in will now ask for your fingerprint before it is used.".to_string()
            } else {
                "The sign-in no longer asks for anything. It is still kept in the system \
                 keychain, on this device only, and is unreadable at rest."
                    .to_string()
            };
            return Ok(self.after(&message));
        }

        self.write_lock(locked)?;
        let message = if locked {
            "The next account you connect will ask for your fingerprint before the sign-in is used."
        } else {
            "The sign-in will not ask for anything."
        };
        Ok(self.after(message))
    }

    /// Forget the account, on this device and on Dropbox's side.
    pub fn disconnect(&self) -> SyncView {
        // Disconnecting underneath a sync in flight would revoke the token the sync
        // is using and clear the record it is about to write. Saying so is better
        // than a race, and the learner only has to press it again.
        let Some(_running) = try_hold(&self.gate) else {
            return self.after("A sync is running. Press Disconnect again in a moment.");
        };
        // A switch pressed while this runs would otherwise write the item back after
        // it had been cleared, and the record with it: connected again, to a token
        // Dropbox has just been told to forget. Tried rather than waited for, like
        // the gate: the switch can be waiting on a fingerprint prompt for as long as
        // the learner leaves it open, and this would hold the gate all that time —
        // so a Sync pressed meanwhile would be told one was already running.
        let Some(_switching) = try_hold(&self.switching) else {
            return self.after(
                "The fingerprint setting is being changed. Press Disconnect again in a moment.",
            );
        };
        // Revoking first is the point: forgetting the token locally would leave the
        // authorization standing on Dropbox's side, which is not what "disconnect"
        // means to somebody who pressed it. A failure to revoke is reported rather
        // than swallowed, but it does not stop the local token being removed.
        let unreported = match self.sign_in() {
            Ok(Some((account, _))) => self
                .fresh_access_token(&account)
                .and_then(|token| revoke(self.http.as_ref(), &token).map_err(|e| describe(&e)))
                .err()
                .map(|why| format!(" (Dropbox was not told: {why})")),
            // Already gone from the store, or nobody ever connected. Either way
            // there is nothing on Dropbox's side this app can name.
            Ok(None) => None,
            Err(why) => Some(format!(" (the sign-in could not be read to revoke it: {why})")),
        };

        // A copy only the login keychain still holds — its removal refused — is one
        // no read reaches once the record is gone and adoption is told to look past
        // it, so the record goes; the learner is told what was left.
        let cleared = self.tokens.clear().and_then(|left| {
            if left.is_some() {
                self.write_leftover()?;
            }
            self.clear_record().map(|()| left)
        });
        self.remember(None);
        // A token fetched for a publish under the account that just went is worth
        // nothing, and keeping it would let a later publish use it.
        self.forget_publish_token();
        *self.last.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let message = match (cleared, unreported) {
            (Ok(None), None) => "Disconnected. Your study data is untouched.".to_string(),
            (Ok(None), Some(why)) => format!("Disconnected{why}."),
            (Ok(Some(left)), why) => format!("Disconnected{}. {left}", why.unwrap_or_default()),
            (Err(why), _) => format!("Could not clear the stored sign-in: {why}"),
        };
        SyncView {
            connected: false,
            account_id: None,
            can_connect: self.tokens.available(),
            can_lock: self.tokens.can_lock(),
            locked: self.locked(),
            protection: Protection::Unknown,
            last: None,
            message,
        }
    }

    /// Sync now, over Dropbox, because somebody pressed a button.
    pub fn now(&self) -> Result<SyncView, String> {
        let _running = self.take_gate()?;
        self.over_dropbox()
    }

    /// One pass over Dropbox, with the gate already held.
    ///
    /// Split from [`Self::now`] so that [`Self::auto`] can take the gate itself and
    /// tell "a sync is already running" apart from "the sync failed" — which the
    /// screen shows very differently, one being nothing at all.
    fn over_dropbox(&self) -> Result<SyncView, String> {
        let account = self.account()?;
        let token = self.fresh_access_token(&account)?;
        let remote = DropboxStore::new(self.http.as_ref(), token);
        self.run(&remote, account.account_id)
    }

    /// Sync because the app started or came back, rather than because somebody
    /// pressed a button.
    ///
    /// The three refusals before the attempt are in ascending order of cost, and
    /// each one is a case where attempting would be worse than not:
    ///
    /// - **No account.** Nothing to sync, and the check is a read of the record —
    ///   no keychain, no network. This is the case for most people, who never
    ///   connect Dropbox at all, and it has to be free.
    /// - **A locked sign-in.** The whole point of the fingerprint is that the token
    ///   is released to a person; a sync that starts by itself has no person to ask,
    ///   and asking at launch is exactly the friction the default exists to avoid.
    ///   So with the lock on, syncing stays something the learner presses — which
    ///   the switch's own wording says, and which is a real cost of turning it on
    ///   rather than a hidden one.
    /// - **No network.** A bounded probe rather than `UreqHttp`'s ten-second connect
    ///   timeout, which would otherwise be paid per request at launch. See
    ///   [`hanzi_sync::Reach`].
    ///
    /// What is left is an ordinary sync, reported the same way a pressed one is.
    pub fn auto(&self) -> AutoSync {
        if self.record().is_none() {
            return AutoSync::Skipped {
                reason: "Not connected to Dropbox.".to_string(),
            };
        }
        // What the *item* needs, not what the switch says. A learner who asked for a
        // fingerprint on a build that could not provide one has the switch on and an
        // item that asks nothing, and reading that is silent — so it is safe to sync.
        // The reverse is the case that matters: an item behind a fingerprint that no
        // preference knows about, which is what an upgrade from an earlier build
        // looks like, and which `record` writes down as soon as it finds it.
        if self.protection() == Protection::UserPresence {
            return AutoSync::Skipped {
                reason: "The sign-in is kept behind your fingerprint, so syncing is something \
                         you press rather than something that happens on its own."
                    .to_string(),
            };
        }
        if !self.reach.reachable() {
            return AutoSync::Offline {
                reason: "No network connection.".to_string(),
            };
        }
        // Two at once is not a failure, and must not be reported as one: the other
        // pass is doing the same work and will report for both of them.
        let Some(_running) = try_hold(&self.gate) else {
            return AutoSync::Skipped {
                reason: "A sync is already running.".to_string(),
            };
        };
        match self.over_dropbox() {
            Ok(view) => AutoSync::Synced { view },
            Err(reason) => AutoSync::Failed { reason },
        }
    }

    /// Publish this device's group positions because one of them just moved.
    ///
    /// ## Why this is not a sync
    ///
    /// A full pass publishes, pulls, and then rebuilds the schedule — that is what
    /// makes two devices agree, and it is far more than this needs. What changed is
    /// one small document this device owns: which entry a drill had reached in each
    /// of the learner's own groups. Sending it is a single `put`, so that is all
    /// this does.
    ///
    /// ## Why it does not take the gate
    ///
    /// [`Self::take_gate`] exists because two *syncs* interleaving publish and pull
    /// would each report work the other had already done. This pulls nothing and
    /// rebuilds nothing: it reads this device's own rows and overwrites this
    /// device's own shard, which is a whole-document write either way. A publish
    /// overlapping a sync is therefore a harmless redundant write of the same file —
    /// while gating it would be worse than harmless, because a sync lasts as long as
    /// the network takes and a position changed during one would either queue behind
    /// it or be dropped, and being dropped is the bug this exists to fix.
    ///
    /// ## The refusals, which are [`Self::auto`]'s first three
    ///
    /// - **No study database**: there is nowhere for a position to have been
    ///   written, so there is nothing to publish.
    /// - **No account**: checked against the record rather than the secret store, so
    ///   this costs no keychain read. Most people never connect Dropbox.
    /// - **A sign-in behind a fingerprint**: a publish that starts by itself has
    ///   nobody to satisfy that, and asking at the moment a character is finished is
    ///   exactly the friction the default exists to avoid. So with the lock on, a
    ///   position travels on the next *pressed* sync — the same cost the settings
    ///   screen already states.
    /// - **No network**: the same bounded probe an automatic sync makes, so a phone
    ///   on a train does not spend a connect timeout per entry.
    ///
    /// A refusal is not a failure and is not reported as one; anything else is
    /// handed back to the caller, which logs it. Nothing here reaches the screen:
    /// the learner's own action — finishing the entry — did succeed, and a position
    /// that did not publish is published by the next sync.
    pub fn publish_positions(&self) -> Result<(), String> {
        if self.db.is_none() || self.record().is_none() {
            return Ok(());
        }
        if self.protection() == Protection::UserPresence {
            return Ok(());
        }
        if !self.reach.reachable() {
            return Ok(());
        }
        let account = self.account()?;
        let token = self.publish_token(&account)?;
        let remote = DropboxStore::new(self.http.as_ref(), token);
        self.publish_positions_to(&remote)
            .map_err(|error| describe(&error))
    }

    /// Write this device's group positions into a store, and nothing else.
    ///
    /// Transport-free, so the document that travels is testable against a
    /// directory. An empty set publishes nothing, which is what absence has to mean
    /// here — see [`hanzi_sync::write_vocab_cursors`].
    fn publish_positions_to(&self, remote: &dyn RemoteStore) -> Result<(), SyncError> {
        let Some(db) = &self.db else {
            return Ok(());
        };
        let cursors = db.vocab_cursors().map_err(SyncError::Io)?;
        if cursors.is_empty() {
            return Ok(());
        }
        write_vocab_cursors(remote, db.device_id(), &cursors)?;
        Ok(())
    }

    /// Ask for the positions to be published, on a thread of their own.
    ///
    /// This is called from the command that moves a position, which is a moment the
    /// learner chose for something else entirely — finishing an entry in a drill.
    /// The network must not hold that up, and `UreqHttp` allows ten seconds to
    /// connect, so the work goes to a thread and the command returns.
    ///
    /// ## The two flags, which are one coalescer
    ///
    /// Both halves are load-bearing:
    ///
    /// - A change that lands **while a publish is in flight** sets
    ///   `positions_owed` again, and the thread goes round once more. Without that,
    ///   the **last** entry of a drill could be the one that never went — which is
    ///   precisely the failure this exists to fix, since a finished drill is when
    ///   the learner puts the phone down and picks up the other device.
    /// - A change that lands while a thread is already draining does **not** start
    ///   a second one. The document is written whole, so a second thread would send
    ///   exactly what the first is about to send.
    pub fn publish_positions_soon(self: &Arc<Self>) {
        self.positions_owed.store(true, Ordering::SeqCst);
        if self.publishing_positions.swap(true, Ordering::SeqCst) {
            return;
        }
        let service = Arc::clone(self);
        std::thread::spawn(move || service.drain_positions());
    }

    /// Publish until no change is outstanding, then stand down.
    fn drain_positions(self: Arc<Self>) {
        self.drain_owed(|| self.publish_positions());
        self.publishing_positions.store(false, Ordering::SeqCst);
        // The flag is cleared *before* this check, so a change that arrives from
        // here on finds `publishing_positions` false and starts a thread of its own.
        // What this catches is the change that arrived while the loop was finishing.
        if self.positions_owed.swap(false, Ordering::SeqCst) {
            self.publish_positions_soon();
        }
    }

    /// The loop above, with the publish handed in so that the coalescing can be
    /// exercised without a thread or a network.
    fn drain_owed(&self, mut publish: impl FnMut() -> Result<(), String>) {
        loop {
            // Cleared *before* the store is read, so a change that lands while this
            // publishes is caught by the check below rather than swallowed.
            self.positions_owed.store(false, Ordering::SeqCst);
            // Logged rather than swallowed, and logged rather than shown: there is
            // no screen this belongs on, and the next sync publishes it anyway. A
            // silent failure here is how a working feature looked broken the first
            // time — the position simply was not on the other device.
            if let Err(why) = publish() {
                eprintln!("[sync] the place in your lists was not published: {why}");
            }
            if !self.positions_owed.swap(false, Ordering::SeqCst) {
                break;
            }
        }
    }

    /// An access token for a publish, reused for [`PUBLISH_TOKEN_REUSE`].
    ///
    /// Deliberately not [`Self::fresh_access_token`], which fetches one every time
    /// because a sync may be the first thing to happen in a session that outlives
    /// the token. A publish happens once per finished entry, so it keeps its own
    /// token for half an hour and then fetches another.
    ///
    /// Cleared whenever the account behind it changes — see [`Self::finish`] and
    /// [`Self::disconnect`] — so a token that belonged to a disconnected account is
    /// never the one a later publish uses.
    fn publish_token(&self, account: &Account) -> Result<String, String> {
        let mut held = self.publish_token.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((token, fetched)) = &*held {
            if fetched.elapsed() < PUBLISH_TOKEN_REUSE {
                return Ok(token.clone());
            }
        }
        let tokens: Tokens =
            refresh(self.http.as_ref(), APP_KEY, &account.refresh_token).map_err(|e| describe(&e))?;
        *held = Some((tokens.access_token.clone(), Instant::now()));
        Ok(tokens.access_token)
    }

    /// Forget the reused token, because the account behind it has gone or changed.
    fn forget_publish_token(&self) {
        *self.publish_token.lock().unwrap_or_else(|e| e.into_inner()) = None;
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
            vocab_changed: summary.vocab_changed,
            cursor_moved: summary.cursor_moved,
        };
        *self.last.lock().unwrap_or_else(|e| e.into_inner()) = Some(view.clone());
        Ok(SyncView {
            connected: true,
            account_id,
            can_connect: self.tokens.available(),
            can_lock: self.tokens.can_lock(),
            locked: self.locked(),
            protection: self.protection(),
            last: Some(view),
            message: describe_summary(&summary),
        })
    }

    /// The connected account, or a sentence saying there is none.
    fn account(&self) -> Result<Account, String> {
        self.sign_in()?
            .map(|(account, _)| account)
            .ok_or_else(|| {
                "No Dropbox account is connected. Connect one from Settings.".to_string()
            })
    }

    /// The sign-in, read from the secret store **at most once per run**.
    ///
    /// This is the only place the token store is read, which is what turns "one
    /// prompt per read" into "one prompt per run of the app" for a sign-in the
    /// learner has asked to have locked. A read that finds nothing also clears the
    /// record, because a record that says connected while the store says otherwise
    /// is a screen that offers Sync and then cannot do it.
    fn sign_in(&self) -> Result<Option<(Account, Protection)>, String> {
        Ok(self.found()?.map(|found| (found.account, found.protection)))
    }

    /// [`Self::sign_in`], with where the store found it.
    ///
    /// A record that names somewhere else is corrected here, before anything can
    /// write to the store with it: a save starts from the place the record names,
    /// and one that started from a place that is empty would put its new item where
    /// a read looks first — the new item under the old record, the moment a save
    /// is cut short. A record that names an empty place is what a write of the
    /// record that failed after a save leaves, or a build that had one place.
    fn found(&self) -> Result<Option<Loaded>, String> {
        let mut open = self.open.lock().unwrap_or_else(|e| e.into_inner());
        if let Open::Known(known) = &*open {
            return Ok(known.clone());
        }
        let found = self.tokens.load(self.read_place().as_deref())?;
        *open = Open::Known(found.clone());
        drop(open);

        match &found {
            None => {
                // Best effort: a record that cannot be cleared is one the next read of
                // this store corrects anyway, and there is no screen to tell from here.
                let _ = self.clear_record();
            }
            Some(loaded) if loaded.place.is_some() => {
                if let Ok(Some(record)) = self.read_record() {
                    if record.place != loaded.place {
                        let _ = self.write_record(&Stored {
                            place: loaded.place.clone(),
                            ..record
                        });
                    }
                }
            }
            Some(_) => {}
        }
        Ok(found)
    }

    /// What the stored sign-in is protected by, without reading it.
    ///
    /// **The record is the authority**, and this asks nothing else. The write that
    /// created the item is what knew how it turned out — `save` reports it, and both
    /// connect and the fingerprint switch write it down — so believing the record is
    /// the only answer that cannot be a guess.
    ///
    /// A *read* is deliberately not consulted, and that is the fix for a bug worth
    /// naming, because the shape looks like a tidiness choice and is not. Apple's
    /// data-protection keychain returns the password and nothing about the access
    /// control on it: an item written with no constraint and one behind a fingerprint
    /// are the same read. The code answered `UserPresence` for both, and this function
    /// preferred that cached answer to the record — so the first time a run touched the
    /// token (which is the first sync, and an automatic sync is exactly that) every
    /// later automatic sync in that run was skipped as though the learner had asked
    /// for a fingerprint, and the settings screen said so too. Per-run on a healthy
    /// device, and permanent on an upgraded one.
    ///
    /// With no record this *is* the adoption path: it reads the store once, writes
    /// down what it found, and answers from that. See [`Self::record`] for what it
    /// does with a store that cannot say.
    fn protection(&self) -> Protection {
        self.record().map(|r| r.protection).unwrap_or_default()
    }

    /// Put the secret store's answer in the cache, without writing to it.
    ///
    /// Used after a save, where the store has just said what it managed to do.
    fn remember(&self, known: Option<Loaded>) {
        *self.open.lock().unwrap_or_else(|e| e.into_inner()) = Open::Known(known);
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
        let mut view = self.view();
        view.message = message.to_string();
        view
    }

    /// What to say when nothing has just happened.
    ///
    /// Reads the record rather than the store, so the answer costs no prompt — which
    /// is the whole reason the record exists. A keychain that cannot be read at all
    /// is not reported here either: nothing was asked of it, and the failure is
    /// reported when something is.
    ///
    /// The one thing said beyond "connected" is a switch that is on over a sign-in
    /// that does not ask. Every Android build before #28 left exactly that — it
    /// could not lock the sign-in, and kept it unlocked instead — so a learner who
    /// turned the switch on then has been told nothing since. Where the device can
    /// ask, turning the switch off and on again rewrites the item behind the prompt;
    /// where it cannot, the switch is not offered, and the sentence says why.
    fn status_message(&self, record: Option<&Stored>, can_lock: bool, locked: bool) -> String {
        if !self.tokens.available() {
            return no_secure_store();
        }
        match record {
            Some(stored) if locked && stored.protection == Protection::KeychainOnly => {
                if can_lock {
                    "Connected to Dropbox. The switch is on, but the stored sign-in does not ask \
                     for your fingerprint yet: turn the switch off and on again to lock it."
                        .to_string()
                } else {
                    format!("Connected to Dropbox. {}", lock_unavailable())
                }
            }
            Some(_) => "Connected to Dropbox.".to_string(),
            None => "Not connected. Your study data stays on this device until you connect."
                .to_string(),
        }
    }

    // ---- the non-secret record, which is what the screen reads --------------

    /// What is connected, without going near the secret store.
    ///
    /// ## The one read that is not a sync
    ///
    /// Before this record existed, "connected?" was answered by reading the token
    /// store. A device that connected under a build like that has a token and no
    /// record, so a record that is absent and a secret store that answers is exactly
    /// how adoption looks: the answer is written down here, and that is the last
    /// time drawing the screen touches the store. On a device that never connected
    /// there is no item to find, so the lookup asks nobody for anything — which is
    /// what makes it safe to attempt unconditionally.
    ///
    /// A failure to write the record down is swallowed deliberately: it costs one
    /// adoption attempt per run, and taking the settings screen down with it would
    /// be a worse answer than showing the connection a moment late.
    fn record(&self) -> Option<Stored> {
        // A record that cannot be read or parsed is treated as absent here, as it
        // always was: the store is still the authority on the token, and adopting
        // what it holds writes the record again.
        if let Ok(Some(record)) = self.read_record() {
            return Some(record);
        }
        let Loaded {
            account,
            protection: reported,
            place,
        } = self.found().ok().flatten()?;
        // A store that cannot say what protection its item carries — Apple's
        // data-protection keychain — is answered from the history here, and the
        // history is not a guess. The record and the *default of asking for nothing*
        // arrived in the same build, so an item with **no record at all** was written
        // by a build that always put the token behind a fingerprint: there was no way
        // to ask for one and no way to have an item that asks for nothing. This is
        // also the only place that inference is allowed to stand, because it is the
        // only place where "no record" is evidence.
        let protection = if reported == Protection::Unknown {
            Protection::UserPresence
        } else {
            reported
        };
        // Where it was found, so that the first save starts from the copy this
        // record describes — not from a place a build with one place would have
        // used, which after a reinstall or a downgrade may be the empty one.
        let stored = Stored {
            account_id: account.account_id,
            protection,
            place,
        };
        let _ = self.write_record(&stored);
        // An item behind a fingerprint that no preference knows about is what an
        // upgrade from an earlier build looks like, and the switch has to be told:
        // left unsaid, the settings screen would show it off while the item asked
        // for a fingerprint, and a launch would read it — which is a prompt at a
        // moment nobody chose, once per launch, for ever. Saying nothing when the
        // item asks for nothing is right, because that is what absent means.
        //
        // This writes a *preference*, which is why it is confined to the case where
        // the item is known to be locked: the write above is what the item is, and
        // the switch is what the learner wants — and on an item written by a build
        // with no switch, those are the same thing.
        if protection == Protection::UserPresence {
            let _ = self.write_lock(true);
        }
        Some(stored)
    }

    /// The record as stored: `Ok(None)` when there is definitely none — no
    /// database, nothing written, or the empty value [`Self::clear_record`] leaves —
    /// and an error when there may be one this cannot read: the database refused,
    /// or holds something this build cannot parse.
    fn read_record(&self) -> Result<Option<Stored>, String> {
        let Some(db) = &self.db else {
            return Ok(None);
        };
        match db.meta_value(ACCOUNT_KEY)? {
            None => Ok(None),
            Some(text) if text.is_empty() => Ok(None),
            Some(text) => serde_json::from_str(&text)
                .map(Some)
                .map_err(|e| format!("the connection record could not be read: {e}")),
        }
    }

    /// Where the record says the stored item is, for the store to read first.
    ///
    /// A record from before there was a second place says nothing, and the store
    /// answers for it from what that record does say: see
    /// [`TokenStore::legacy_place`]. With no record, nowhere — unless a disconnect
    /// left a copy behind that nothing should adopt: see [`LEFTOVER_KEY`].
    fn read_place(&self) -> Option<String> {
        match self.read_record() {
            Ok(Some(record)) => record
                .place
                .or_else(|| self.tokens.legacy_place(record.protection)),
            Ok(None) if self.leftover() => self.tokens.place_past_leftovers(),
            _ => None,
        }
    }

    /// Where the record says the stored item is, made right before a save starts
    /// from it.
    ///
    /// The store is asked where a read would really find the item
    /// ([`TokenStore::locate`]), without reading it, and a record that names
    /// anywhere else is rewritten first: a save that started from an empty place
    /// would put the new item where a read looks first. When the record cannot be
    /// rewritten, the place it names is what the save is given, and a store with
    /// more than one place refuses to start from it rather than guess.
    fn settled_place(&self) -> Option<String> {
        let Ok(Some(record)) = self.read_record() else {
            return self.read_place();
        };
        let named = record
            .place
            .clone()
            .or_else(|| self.tokens.legacy_place(record.protection));
        match self.tokens.locate(named.as_deref()) {
            Ok(Some(found)) if Some(&found) != named.as_ref() => {
                let corrected = Stored {
                    place: Some(found.clone()),
                    ..record
                };
                match self.write_record(&corrected) {
                    Ok(()) => Some(found),
                    Err(_) => named,
                }
            }
            _ => named,
        }
    }

    /// Whether a disconnect left a copy behind that adoption must look past.
    fn leftover(&self) -> bool {
        self.db
            .as_ref()
            .and_then(|db| db.meta_value(LEFTOVER_KEY).ok().flatten())
            .is_some_and(|value| value == "on")
    }

    fn write_leftover(&self) -> Result<(), String> {
        match &self.db {
            Some(db) => db
                .set_meta_value(LEFTOVER_KEY, "on")
                .map_err(|e| format!("the connection could not be forgotten: {e}")),
            None => Ok(()),
        }
    }

    /// Best effort: a flag left set only narrows where a later adoption looks.
    fn forget_leftover(&self) {
        if let Some(db) = &self.db {
            let _ = db.set_meta_value(LEFTOVER_KEY, "");
        }
    }

    /// Write the record down.
    fn write_record(&self, stored: &Stored) -> Result<(), String> {
        let text = serde_json::to_string(stored)
            .map_err(|e| format!("the connection could not be recorded: {e}"))?;
        self.db
            .as_ref()
            .ok_or_else(|| "this build has no study database to record the connection in".to_string())?
            .set_meta_value(ACCOUNT_KEY, &text)
            .map_err(|e| format!("the connection could not be recorded: {e}"))
    }

    /// Forget the record.
    ///
    /// An empty value rather than a deleted row: `meta` is written by upsert with no
    /// delete to reach it from here, and `""` is not JSON, so every reader of this key
    /// has to read it as "nothing written down" — which is the only thing it has to
    /// mean. Best effort, and nothing to fail at when the app has no database.
    fn clear_record(&self) -> Result<(), String> {
        let Some(db) = &self.db else {
            return Ok(());
        };
        db.set_meta_value(ACCOUNT_KEY, "")
            .map_err(|e| format!("the connection could not be forgotten: {e}"))
    }

    /// Whether the learner has asked for a fingerprint.
    ///
    /// Absent means no, which is the default: see the module note on why asking for
    /// nothing is the right default for something meant to run by itself.
    fn locked(&self) -> bool {
        self.db
            .as_ref()
            .and_then(|db| db.meta_value(LOCK_KEY).ok().flatten())
            .is_some_and(|value| value == "on")
    }

    /// Write the learner's answer down.
    fn write_lock(&self, locked: bool) -> Result<(), String> {
        self.db
            .as_ref()
            .ok_or_else(|| {
                "this build has no study database, so a preference cannot be kept".to_string()
            })?
            .set_meta_value(LOCK_KEY, if locked { "on" } else { "off" })
            .map_err(|e| format!("the preference could not be saved: {e}"))
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
    if summary.vocab_changed > 0 {
        let noun = match summary.vocab_changed {
            1 => "list entry",
            _ => "list entries",
        };
        parts.push(format!("updated {} {noun}", summary.vocab_changed));
    }
    if summary.cursor_moved {
        parts.push("moved to where you left off".to_string());
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

/// A fingerprint was asked for, and the store kept the sign-in without one.
///
/// Which of the two it is differs by platform — an unsigned build on Apple, a phone
/// with no screen lock on Android — and both are the same thing to the learner: the
/// ask was not available, and the sign-in is still kept encrypted on this device.
fn lock_unavailable() -> String {
    "This build or this device cannot ask for your fingerprint, so the sign-in is kept in the \
     device's own store instead. Your study data is unaffected."
        .to_string()
}

/// This device has nothing to ask the learner with.
fn no_biometric() -> String {
    "This device cannot ask for a fingerprint or a screen lock, so the sign-in is protected by \
     the device instead — kept where only this app can read it, and unreadable at rest. Your \
     study data is unaffected."
        .to_string()
}

// ---- where the token actually lives ----------------------------------------

/// What a platform's secret store has to do.
///
/// Five methods, and **no notion of a session**, because that belongs to the caller
/// — and every extra method is another thing a platform can get subtly wrong. Two of
/// them are worth reading twice:
///
/// - [`Self::load`] is called when the token is about to be *used*, not to find out
///   whether there is one. That is what keeps a fingerprint prompt off the settings
///   screen; see the module note.
/// - [`Self::save`] takes the learner's answer on asking for a fingerprint and
///   **returns what the item actually got**, because the answer is not always
///   available: a build the system cannot identify lands in the login keychain
///   instead, and an Android phone with no screen lock cannot make a key that asks
///   at all. A store that could not honour the request says so rather than letting
///   the screen claim a prompt that will never appear. A store that *could* but
///   did not — a prompt the learner cancelled — returns an error and leaves the
///   stored item as it was.
trait TokenStore: Send + Sync {
    /// Whether this platform can keep a secret at all.
    fn available(&self) -> bool;
    /// Whether this platform can ask for a fingerprint at all.
    fn can_lock(&self) -> bool;
    /// Read the sign-in, and how well the store can say it is protected.
    ///
    /// The protection is what this store can **prove** about the item it found,
    /// which is not always what the item is. Apple's data-protection keychain
    /// returns the password and nothing about the access control on it, so an item
    /// written with no constraint and one behind a fingerprint are the same read.
    /// [`Protection::Unknown`] is that answer — *there is an item, and this store
    /// cannot say how it is protected* — and it is deliberately **not**
    /// [`Protection::UserPresence`]. The record in `meta` is what knows, because the
    /// write that created the item knew what it managed; see
    /// [`SyncService::protection`].
    ///
    /// `place` is where the record says the item is — what an earlier
    /// [`Self::save`] answered — and a store with more than one place looks there
    /// first. A store with one ignores it. The answer says where the item was
    /// found, so that a record naming somewhere else can be put right.
    fn load(&self, place: Option<&str>) -> Result<Option<Loaded>, String>;
    /// Write the sign-in, asking for a fingerprint on every read if `locked`, in
    /// place of the one the record says is at `place`.
    fn save(&self, account: &Account, locked: bool, place: Option<&str>) -> Result<Saved, String>;
    /// Remove every copy of the sign-in.
    ///
    /// `Ok(Some(why))` when every copy a read reaches is gone but one a read does
    /// not — Apple's login keychain, whose removal can ask for the keychain
    /// password — could not be removed; `why` is said to the learner.
    fn clear(&self) -> Result<Option<String>, String>;
    /// Where a read starting at `place` would really find the item, found without
    /// reading it, so that a record naming an empty place can be put right before
    /// a save starts from it. A store with one place answers `place` itself.
    fn locate(&self, place: Option<&str>) -> Result<Option<String>, String> {
        Ok(place.map(str::to_string))
    }
    /// A place to read from that never reaches a copy a refused [`Self::clear`]
    /// left behind. See [`LEFTOVER_KEY`].
    fn place_past_leftovers(&self) -> Option<String> {
        None
    }
    /// Where a record written before [`Stored::place`] existed says the item is,
    /// from the protection it recorded. Only a store with more than one place
    /// has an answer.
    fn legacy_place(&self, _protection: Protection) -> Option<String> {
        None
    }
}

/// What a read found: the sign-in, what the store can prove about how it is
/// protected (see [`TokenStore::load`]), and where it was.
#[derive(Clone, Debug)]
struct Loaded {
    account: Account,
    protection: Protection,
    /// `None` from a store with one place.
    place: Option<String>,
}

/// What a write did: how the item ended up protected, and where the store put it.
struct Saved {
    protection: Protection,
    /// For the record, so that reads and the next write start there: see
    /// [`Stored::place`]. `None` from a store with one place.
    place: Option<String>,
}

/// The secret store for the platform this build is for.
fn platform_store() -> Box<dyn TokenStore> {
    #[cfg(target_vendor = "apple")]
    {
        Box::new(Keychain)
    }
    #[cfg(target_os = "android")]
    {
        Box::new(AndroidKeystore)
    }
    #[cfg(not(any(target_vendor = "apple", target_os = "android")))]
    {
        Box::new(NoSecureStore)
    }
}

/// Android's keystore, reached through the Kotlin plugin.
///
/// Android has no keychain of the Apple kind: its keystore holds *keys*, not
/// secrets, so the Kotlin side generates an AES-256-GCM key inside it and keeps
/// only the ciphertext. That is a real protection — the key material never leaves
/// the device's secure hardware where there is any, and the file on disk is
/// useless without it.
///
/// **Asking for the learner.** A key made with `setUserAuthenticationRequired`
/// will neither decrypt *nor encrypt* until the learner has identified themselves
/// — an AES key has no public half that keystore would let through unasked — so
/// the Kotlin side shows a `BiometricPrompt` carrying the cipher as its
/// `CryptoObject` both when the token is read and when it is locked. It is a
/// per-*operation* constraint rather than a per-read one, which is what keeps it
/// compatible with the module note: the prompt happens when the token is about to
/// be used, or when the learner connects or turns the switch on, and never to draw
/// a screen. A key made without it decrypts silently, which is what
/// [`Self::load`] reports as [`Protection::KeychainOnly`].
///
/// The mode belongs to the key and cannot be changed afterwards, so the Kotlin
/// side keeps one key per mode and records the mode beside the blob. A locked
/// write that does not happen — the learner cancels the prompt, or keystore will
/// not make the key — is an **error** from [`Self::save`], and the item stored
/// before is left exactly as it was; it is never quietly kept unlocked instead.
/// The one exception is a device that cannot ask at all (its screen lock removed
/// since the switch was turned on, where the switch is no longer offered): there
/// [`Self::save`] reports the mode the item *actually* got — the unprotected one —
/// rather than refusing to connect, and the screen says so.
#[cfg(target_os = "android")]
struct AndroidKeystore;

/// What `loadSecret` answers: the token, how it is protected, or no field at all.
///
/// An absent `secret` rather than a null one, because `JSObject` is a `JSONObject`
/// and `put(key, null)` *removes* the mapping — so a null would arrive as an absent
/// field by a route nobody could read from the code.
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct StoredSecret {
    #[serde(default)]
    secret: Option<String>,
    /// Whether the blob was written under a key that asks for the learner's
    /// fingerprint, face or device password before it will decrypt.
    #[serde(default)]
    protected: bool,
}

/// What the Kotlin side says about whether the learner can be asked at all.
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct Askable {
    available: bool,
}

/// What `saveSecret` answers: how well the sign-in actually ended up protected.
///
/// A separate answer from [`StoredSecret`] because it says what the write did,
/// not what a read found, and the two can disagree: a device that cannot ask for
/// authentication at all keeps the token the way an earlier build did, and says so
/// here rather than failing. Any other failure to lock is a rejection instead.
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct SavedSecret {
    protected: bool,
}

#[cfg(target_os = "android")]
impl TokenStore for AndroidKeystore {
    fn available(&self) -> bool {
        true
    }

    fn can_lock(&self) -> bool {
        // The plugin answers on Android's main thread and this blocks until it
        // does — see `crate::platform::call` — so it is only safe off that thread,
        // and so are `load` and `save`, which can wait on a fingerprint prompt.
        // Every command that reaches this store has to run on Tauri's runtime or
        // its blocking pool rather than on the main thread; a plain
        // `#[tauri::command]` runs there, and would wait on itself (issue #26,
        // which is what moves `sync_set_lock` off it). A bridge that is not up
        // yet, or a device whose answer cannot be read, means no rather than a
        // claim: the switch is then not offered, which is the same answer as a
        // device that cannot ask.
        crate::platform::call::<Askable>("canLock", ())
            .map(|answer| answer.available)
            .unwrap_or(false)
    }

    fn load(&self, _place: Option<&str>) -> Result<Option<Loaded>, String> {
        let answer: StoredSecret = crate::platform::call("loadSecret", ())?;
        let Some(text) = answer.secret else {
            return Ok(None);
        };
        // What the item got, which is the same question the Apple store answers
        // by which keychain it found the password in.
        let protection = if answer.protected {
            Protection::UserPresence
        } else {
            Protection::KeychainOnly
        };
        serde_json::from_str(&text)
            .map(|account| {
                Some(Loaded {
                    account,
                    protection,
                    place: None,
                })
            })
            .map_err(|e| format!("the stored Dropbox sign-in could not be read: {e}"))
    }

    fn save(&self, account: &Account, locked: bool, _place: Option<&str>) -> Result<Saved, String> {
        let text = serde_json::to_string(account)
            .map_err(|e| format!("the Dropbox sign-in could not be encoded: {e}"))?;
        let answer: SavedSecret = crate::platform::call(
            "saveSecret",
            serde_json::json!({ "secret": text, "requireAuth": locked }),
        )?;
        // The Kotlin side may have had to keep less than it was asked for, and its
        // answer says which it managed — the honest label the screen draws from.
        let protection = if answer.protected {
            Protection::UserPresence
        } else {
            Protection::KeychainOnly
        };
        Ok(Saved {
            protection,
            place: None,
        })
    }

    fn clear(&self) -> Result<Option<String>, String> {
        crate::platform::call::<serde_json::Value>("clearSecret", ())?;
        Ok(None)
    }
}

/// Where a copy of the sign-in can be, in Apple's keychain.
///
/// Two places in the data-protection keychain, told apart by their account name,
/// and the login keychain an ad-hoc signed build falls back to.
///
/// Two, because an access control is fixed when an item is created. Changing it —
/// the fingerprint switch — means a new item, and the old one has to go. Deleting
/// it first, as this once did, meant an add the keychain then refused lost the
/// refresh token while the screen and the database still said connected. Adding
/// first cannot be done in the same place: an item with the same service and
/// account is a duplicate, and `set_generic_password_options` answers that by
/// *updating* the old item's password and keeping its access control — the switch
/// saying one thing and the item doing the other. So a save writes into whichever
/// slot is free, and only then removes the other. See [`replace_sign_in`].
///
/// Which slot holds the item the record describes is written in the record
/// ([`Stored::place`], as [`Slot::name`]), and every read and save starts there; a
/// read that finds the item somewhere else puts the record right, and so does a
/// save, asking where the item is without reading it.
/// A save cut short — the app quit, or a removal and then its rollback refused —
/// can leave two copies, and the record is written only once a save has finished,
/// so the copy it names is the one it describes. A fixed order cannot know that:
/// it would find the new copy half the time, under a record describing the old.
#[cfg(any(target_vendor = "apple", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Slot {
    /// The data-protection item every earlier build wrote.
    Primary,
    /// The other data-protection item.
    Secondary,
    /// The login-keychain item: an ad-hoc signed build, or a sign-in made before
    /// the data-protection keychain was used. macOS only.
    Login,
}

#[cfg(any(target_vendor = "apple", test))]
impl Slot {
    /// Where a read looks with no record to say: the slot every earlier build
    /// wrote, then the other, then the login keychain.
    const READ_ORDER: [Slot; 3] = [Slot::Primary, Slot::Secondary, Slot::Login];

    /// Where a read looks when the record names `place`: there first, and then
    /// the other places it may be — the record can be behind a finished save whose
    /// record write failed, and an item it does not name is still better than none.
    ///
    /// Not the login keychain, when the record names a data-protection slot. A copy
    /// there is one a save could not remove and left beside an item that asks for
    /// nothing (see [`replace_sign_in`]), or one a disconnect could not: reading it
    /// is a keychain-password prompt, for a token that may be another account's or
    /// one Dropbox has been told to forget. With no record, or one naming the login
    /// keychain, it is where a build that could not use the data-protection
    /// keychain kept the sign-in, and is read last.
    fn read_order(place: Option<Slot>) -> Vec<Slot> {
        let rest: &[Slot] = match place {
            Some(Slot::Primary) => &[Slot::Secondary],
            Some(Slot::Secondary) => &[Slot::Primary],
            Some(Slot::Login) => &[Slot::Primary, Slot::Secondary],
            None => &Self::READ_ORDER,
        };
        place.into_iter().chain(rest.iter().copied()).collect()
    }

    /// The name the record keeps for this slot. The data-protection ones are the
    /// item's account name, which is what makes them easy to find by hand.
    fn name(self) -> &'static str {
        match self {
            Slot::Primary => "dropbox",
            Slot::Secondary => "dropbox.next",
            Slot::Login => "login",
        }
    }

    /// The slot a record names, or `None` for a name this build does not know.
    fn named(name: &str) -> Option<Slot> {
        Self::READ_ORDER
            .into_iter()
            .find(|slot| slot.name() == name)
    }
}

/// Why a new item was not added.
#[cfg(any(target_vendor = "apple", test))]
#[derive(Debug)]
enum AddRefused {
    /// This build may not use the data-protection keychain at all
    /// (`errSecMissingEntitlement`), so the login keychain is the one to use.
    Unreachable,
    /// Anything else, in words.
    Failed(String),
}

/// The steps a save on Apple's keychain is made of, so that their order can be
/// tested without one: a test that wrote to the developer's keychain would be one
/// that could delete their account.
#[cfg(any(target_vendor = "apple", test))]
trait Slots {
    /// Whether a data-protection slot holds an item, found without reading it —
    /// a read is what asks for a fingerprint. Always `false` for the login slot.
    /// A search that cannot answer is an error, not a guess either way: nothing
    /// has been changed yet, and a wrong guess removes the only copy.
    fn occupied(&self, slot: Slot) -> Result<bool, String>;
    /// Create the new item in a data-protection slot that is free, or create or
    /// update the login-keychain one in place.
    fn add(&self, slot: Slot) -> Result<(), AddRefused>;
    /// Remove whatever a slot holds; nothing there is success.
    fn remove(&self, slot: Slot) -> Result<(), String>;
}

/// Replace the stored sign-in, the new item **before** the old one goes, and say
/// which slot it ended up in. `place` is the slot the record names, and `locked`
/// whether the new item asks for a fingerprint.
///
/// Wherever it stops — an error, or the app quitting between two steps — a read
/// that starts at `place` finds the item the record describes, which is what
/// [`TokenStore::save`] promises and what lets the service go on saying
/// "connected" honestly:
///
/// - The new item goes into a free slot, so a refused add has touched nothing.
/// - The old copies go afterwards, and the one a read finds now — the one the
///   record describes — goes **last**, so until the save has finished it is still
///   there to be found. One that cannot be removed would outlive the new item with
///   the protection the learner asked to replace, so the new item is taken back
///   out and the save reported as failed.
/// - Except a login-keychain copy beside a new item that asks for nothing. Removing
///   one written by another build can ask for the keychain password, and a learner
///   who cancels that should not be unable to connect: the record will name the
///   new item, so the copy left behind is never read, and it asks for nothing
///   either. Beside a locked item it would be the lock's way round, so there it is
///   still a failure.
/// - With both data-protection slots held — a save cut short — the one the record
///   does not name is the stale one, and is removed to make room.
/// - With one free, the new item goes there only if a read starting at `place`
///   reaches it after the copy it finds now. A record naming the empty slot while
///   the copy is in the other leaves no such slot, and the save is refused with
///   nothing changed; the service corrects the record before a save so that this
///   is reached only when it could not (see `SyncService::settled_place`).
/// - A build that cannot use the data-protection keychain falls back to the login
///   keychain, whose one item is updated in place: that is all-or-nothing, and
///   there is no data-protection copy this build could have left to remove.
///
/// ## Left for later
///
/// - **A two-phase save.** The record is written once this returns, so the app
///   quitting after it returned and before that write leaves the record describing
///   the old item while only the new one is left. Writing the record naming the new
///   slot after the add and *before* the old copy is removed would close that
///   window, and with it the cases where the record's place is inferred rather than
///   known: a `set_lock` whose record write failed, and a first connection adopted
///   from a build with one place.
/// - **The login slot counts as held** wherever a read reaches it, because it
///   cannot be looked for without reading it. With the login keychain empty and a
///   record naming it while a data-protection copy is held, a new item can land
///   where a read falling past the empty login slot looks first.
#[cfg(any(target_vendor = "apple", test))]
fn replace_sign_in(slots: &dyn Slots, place: Option<Slot>, locked: bool) -> Result<Slot, String> {
    let primary = slots.occupied(Slot::Primary)?;
    let secondary = slots.occupied(Slot::Secondary)?;
    let held = |slot: Slot| match slot {
        Slot::Primary => primary,
        Slot::Secondary => secondary,
        Slot::Login => true,
    };
    let order = Slot::read_order(place);
    let live = live_slot(place, primary, secondary);
    let position = |slot: Slot| order.iter().position(|s| *s == slot);
    let target = if primary && secondary {
        // Both held: the one a read does not reach first is the stale one.
        let stale = if live == Slot::Secondary {
            Slot::Primary
        } else {
            Slot::Secondary
        };
        slots.remove(stale)?;
        stale
    } else {
        // A free slot a read reaches only *after* the live copy, so that until this
        // save has finished — and the record says where its item is — a read
        // starting where the record says still finds the copy the record describes.
        let free = [Slot::Primary, Slot::Secondary]
            .into_iter()
            .filter(|slot| !held(*slot));
        let mut after_live = free.clone().filter(|slot| position(*slot) > position(live));
        match (after_live.next(), place) {
            (Some(slot), _) => slot,
            // With no record there is nothing for a read to agree with.
            (None, None) => free.clone().next().unwrap_or(Slot::Primary),
            // The record names an empty slot while the copy is in the other one:
            // the only free slot is where a read looks first. Refused rather than
            // written there — the service puts the record right first (see
            // `SyncService::settled_place`), so this is reached only when it could
            // not, and nothing has been changed.
            (None, Some(_)) => {
                return Err(
                    "the record of where the sign-in is kept is out of date, so it was not \
                     replaced; please try again"
                        .to_string(),
                )
            }
        }
    };
    match slots.add(target) {
        Ok(()) => {}
        Err(AddRefused::Unreachable) => {
            return match slots.add(Slot::Login) {
                Ok(()) => Ok(Slot::Login),
                Err(AddRefused::Failed(why)) => Err(why),
                Err(AddRefused::Unreachable) => {
                    Err("the system keychain could not be opened".to_string())
                }
            };
        }
        Err(AddRefused::Failed(why)) => return Err(why),
    }
    let mut stale: Vec<Slot> = Slot::READ_ORDER
        .into_iter()
        .filter(|slot| *slot != target && held(*slot))
        .collect();
    // The live copy last. The sort is stable, so the others keep their order.
    stale.sort_by_key(|slot| *slot == live);
    for old in stale {
        if let Err(why) = slots.remove(old) {
            if old == Slot::Login && !locked {
                eprintln!(
                    "[sync] a copy of the Dropbox sign-in in the login keychain could not be \
                     removed and is left there unused: {why}"
                );
                continue;
            }
            let _ = slots.remove(target);
            return Err(format!(
                "the sign-in stored before could not be removed, so it is kept and the new \
                 one is not: {why}"
            ));
        }
    }
    Ok(target)
}

/// The slot a read starting at `place` finds the item in, given which
/// data-protection slots are held.
///
/// The slot the record names when it is held, and otherwise the first held one a
/// read reaches — never an empty data-protection slot the record happens to name
/// while the copy is in the other. The login slot cannot be looked for without
/// reading it, so it counts as held wherever a read reaches it: that is an
/// approximation, and wrong when the login keychain is empty, which a record
/// naming it with the copy elsewhere would need.
#[cfg(any(target_vendor = "apple", test))]
fn live_slot(place: Option<Slot>, primary: bool, secondary: bool) -> Slot {
    let held = |slot: Slot| match slot {
        Slot::Primary => primary,
        Slot::Secondary => secondary,
        Slot::Login => true,
    };
    Slot::read_order(place)
        .into_iter()
        .find(|slot| held(*slot))
        .or(place)
        .unwrap_or(Slot::Login)
}

/// Where a read starting at `place` would find the item, found without reading.
#[cfg(any(target_vendor = "apple", test))]
fn locate_slot(slots: &dyn Slots, place: Option<Slot>) -> Result<Slot, String> {
    let primary = slots.occupied(Slot::Primary)?;
    let secondary = slots.occupied(Slot::Secondary)?;
    Ok(live_slot(place, primary, secondary))
}

/// Remove every copy of the sign-in, trying each slot even after one refuses.
///
/// A data-protection copy that will not go is an error, and the first is
/// reported. A login-keychain copy that will not go — removing one another build
/// wrote can ask for the keychain password — is `Ok(Some(why))`: no read starting
/// from a data-protection slot reaches it, and the service makes sure no adoption
/// does either (see `LEFTOVER_KEY`).
#[cfg(any(target_vendor = "apple", test))]
fn clear_slots(slots: &dyn Slots) -> Result<Option<String>, String> {
    let mut first = Ok(());
    let mut login = None;
    for slot in Slot::READ_ORDER {
        match slots.remove(slot) {
            Ok(()) => {}
            Err(why) if slot == Slot::Login => login = Some(why),
            Err(why) => {
                if first.is_ok() {
                    first = Err(why);
                }
            }
        }
    }
    first.map(|()| {
        login.map(|why| {
            format!(
                "A copy of the old sign-in in your login keychain could not be removed ({why}); \
                 this app will not use it, and \"HanziTutor cross-device sync\" can be deleted \
                 in Keychain Access."
            )
        })
    })
}

/// Apple's Keychain, which is the same API on macOS and iOS.
///
/// One generic password, named by a service and an account. The service string is
/// what a person sees in Keychain Access, so it says what it is. The account is one
/// of two, because a save writes the new item before it removes the old one — see
/// [`Slot`] — and a read starts at the one the record names.
///
/// ## The two items this can leave behind
///
/// Neither asks for anything by default, and that is the choice the module note
/// argues for:
///
/// - A **data-protection** item with an access control that carries no constraint at
///   all, marked `AccessibleAfterFirstUnlockThisDeviceOnly`. The item is encrypted at
///   rest, is readable only by this app, is not carried to the learner's other
///   devices, and is released without asking anybody — which is what a sync that
///   starts by itself needs. `AfterFirstUnlock` rather than `WhenUnlocked` is what
///   lets that sync work if it is ever woken while the screen is locked; it is still
///   unreadable after a restart until the device has been unlocked once.
/// - A **login-keychain** item, which is where an ad-hoc signed build lands, because
///   access controls need an application identifier that an ad-hoc signature does not
///   have (`errSecMissingEntitlement`). That fallback is also the one case that can
///   ask for the **keychain password** — the system does not recognise a rebuilt
///   binary as the one that wrote the item — and it is why the learner is told which
///   of the two they got rather than left to discover it from a prompt.
///
/// ## Asking for a fingerprint, when the learner wants one
///
/// The same item, created instead with `AccessibleWhenPasscodeSetThisDeviceOnly` and
/// a *user presence* constraint. Apple requires that protection mode on an access
/// control carrying that constraint, and it suits this app: the sign-in is this
/// device's. Such an item asks on **every read** — there is no "always allow" for one
/// built this way — which is exactly why the token is read once per run and never to
/// draw the screen.
#[cfg(target_vendor = "apple")]
struct Keychain;

#[cfg(target_vendor = "apple")]
const KEYCHAIN_SERVICE: &str = "HanziTutor cross-device sync";
#[cfg(target_vendor = "apple")]
const KEYCHAIN_ACCOUNT: &str = "dropbox";
/// The account name of the second data-protection slot. See [`Slot`], whose
/// [`Slot::name`] for each data-protection slot is its account name.
#[cfg(target_vendor = "apple")]
const KEYCHAIN_ACCOUNT_NEXT: &str = "dropbox.next";
/// Whether there is a login keychain apart from the data-protection one.
///
/// Only macOS has one. On iOS every item is a data-protection item, so a query
/// without the data-protection flag names the *same* item as one with it — and
/// removing "the login copy" after writing the new item would remove the new item.
#[cfg(target_vendor = "apple")]
const HAS_LOGIN_KEYCHAIN: bool = cfg!(target_os = "macos");
/// `errSecItemNotFound`, from `Security/SecBase.h`: "no keychain item was found".
#[cfg(target_vendor = "apple")]
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;
/// `errSecUserCanceled`: the learner dismissed the fingerprint prompt.
#[cfg(target_vendor = "apple")]
const ERR_SEC_USER_CANCELED: i32 = -128;
/// `errSecAuthFailed`: the fingerprint or password was not accepted.
#[cfg(target_vendor = "apple")]
const ERR_SEC_AUTH_FAILED: i32 = -25293;
/// `errSecMissingEntitlement`: this binary may not use the data-protection
/// keychain, which is what an ad-hoc signature means.
#[cfg(target_vendor = "apple")]
const ERR_SEC_MISSING_ENTITLEMENT: i32 = -34018;

/// A query for one of this app's items in the data-protection keychain.
#[cfg(target_vendor = "apple")]
fn protected(account: &str) -> PasswordOptions {
    let mut options = PasswordOptions::new_generic_password(KEYCHAIN_SERVICE, account);
    // Without this the item goes to the login keychain, where an access control
    // cannot be attached and the password prompt lives.
    options.use_protected_keychain();
    options
}

/// The access control an item is created with.
///
/// With `locked`, one that asks for a fingerprint, a face or the device password.
/// Without, one that asks for nothing: the protection is still there — the mode
/// below is what makes the item unreadable at rest and this device's alone — but
/// there is no constraint for the system to satisfy, so reading it is silent.
#[cfg(target_vendor = "apple")]
fn access_control(locked: bool) -> Result<SecAccessControl, String> {
    let (mode, options) = if locked {
        (
            ProtectionMode::AccessibleWhenPasscodeSetThisDeviceOnly,
            AccessControlOptions::USER_PRESENCE.bits(),
        )
    } else {
        (
            ProtectionMode::AccessibleAfterFirstUnlockThisDeviceOnly,
            0,
        )
    };
    SecAccessControl::create_with_protection(Some(mode), options)
        .map_err(|e| format!("the access control could not be built: {e}"))
}

/// Describe an OSStatus in words a person can act on.
#[cfg(target_vendor = "apple")]
fn describe_keychain(error: &security_framework::base::Error) -> String {
    match error.code() {
        ERR_SEC_USER_CANCELED => {
            "The request to use your fingerprint or password was cancelled.".to_string()
        }
        ERR_SEC_AUTH_FAILED => {
            "The fingerprint or password was not accepted, so the Dropbox sign-in could not be \
             read."
                .to_string()
        }
        ERR_SEC_MISSING_ENTITLEMENT => {
            "This build is not signed, so the system will not keep a sign-in behind your \
             fingerprint or in the data-protection keychain. It goes in your login keychain \
             instead, which is still encrypted but may ask for your keychain password."
                .to_string()
        }
        _ => error.to_string(),
    }
}

#[cfg(target_vendor = "apple")]
impl TokenStore for Keychain {
    fn available(&self) -> bool {
        true
    }

    fn can_lock(&self) -> bool {
        true
    }

    fn load(&self, place: Option<&str>) -> Result<Option<Loaded>, String> {
        // The slot the record names first, then the rest — the slot every earlier
        // build wrote, the other one, and then whatever an earlier build left in the
        // login keychain, so a connection made before any of this existed is not
        // lost. A save leaves one copy; one cut short can leave two, and the record
        // names the one it describes. See [`Slot`].
        //
        // `errSecMissingEntitlement` moves on for the same reason it falls back in
        // `save`: a build the system cannot identify may not reach the
        // data-protection keychain *at all*, and that is a refusal to open that
        // keychain rather than an answer about this item.
        //
        // **What the item is protected by is not in this answer.** The read returns
        // the password and nothing about the access control on it, and every item
        // this app creates lives in the data-protection keychain either way — so an
        // item written with no constraint and one behind a fingerprint are the same
        // read. `Unknown` says that honestly; `UserPresence` was a guess that the
        // rest of the service believed, which is how every Apple device came to look
        // as though its sign-in were behind a fingerprint. The login-keychain
        // fallback is different and *is* provable: an item there cannot carry an
        // access control at all.
        let mut found = None;
        for slot in Slot::read_order(place.and_then(Slot::named)) {
            let read = match slot.protected_account() {
                Some(account) => {
                    security_framework::passwords::generic_password(protected(account))
                }
                None if !HAS_LOGIN_KEYCHAIN => continue,
                None => security_framework::passwords::get_generic_password(
                    KEYCHAIN_SERVICE,
                    KEYCHAIN_ACCOUNT,
                ),
            };
            match read {
                Ok(bytes) => {
                    let protection = if slot == Slot::Login {
                        Protection::KeychainOnly
                    } else {
                        Protection::Unknown
                    };
                    found = Some((bytes, protection, slot));
                    break;
                }
                Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => {}
                Err(error)
                    if error.code() == ERR_SEC_MISSING_ENTITLEMENT && slot != Slot::Login => {}
                Err(error) => return Err(describe_keychain(&error)),
            }
        }

        let Some((bytes, protection, slot)) = found else {
            return Ok(None);
        };
        let text = String::from_utf8(bytes)
            .map_err(|_| "the stored Dropbox sign-in is not readable text".to_string())?;
        let account: Account = serde_json::from_str(&text)
            .map_err(|e| format!("the stored Dropbox sign-in could not be read: {e}"))?;
        Ok(Some(Loaded {
            account,
            protection,
            place: Some(slot.name().to_string()),
        }))
    }

    fn save(&self, account: &Account, locked: bool, place: Option<&str>) -> Result<Saved, String> {
        let text = serde_json::to_string(account)
            .map_err(|e| format!("the Dropbox sign-in could not be encoded: {e}"))?;
        // The order — the new item before the old one goes — is [`replace_sign_in`]'s,
        // and is tested there; this only says what each step is on a real keychain.
        let slots = AppleSlots {
            text: text.as_bytes(),
            locked,
        };
        let slot = replace_sign_in(&slots, place.and_then(Slot::named), locked)?;
        let protection = match slot {
            Slot::Login => Protection::KeychainOnly,
            Slot::Primary | Slot::Secondary if locked => Protection::UserPresence,
            Slot::Primary | Slot::Secondary => Protection::DeviceOnly,
        };
        Ok(Saved {
            protection,
            place: Some(slot.name().to_string()),
        })
    }

    fn clear(&self) -> Result<Option<String>, String> {
        // Every copy, and a failure to find any is success: "forget the sign-in" has
        // to work on a device that never had one, and on one that has a copy in each.
        // Each is tried even when one before it fails, so a refusal in one keychain
        // does not leave a copy in another. See [`clear_slots`].
        let slots = AppleSlots {
            text: &[],
            locked: false,
        };
        clear_slots(&slots)
    }

    fn locate(&self, place: Option<&str>) -> Result<Option<String>, String> {
        let slots = AppleSlots {
            text: &[],
            locked: false,
        };
        let slot = locate_slot(&slots, place.and_then(Slot::named))?;
        Ok(Some(slot.name().to_string()))
    }

    fn place_past_leftovers(&self) -> Option<String> {
        // A read starting from a data-protection slot never reaches the login one.
        Some(Slot::Primary.name().to_string())
    }

    fn legacy_place(&self, protection: Protection) -> Option<String> {
        // Every build before the record named a place wrote one item: in the login
        // keychain when it could not reach the data-protection one, which is the
        // only way an Apple item comes to be recorded as `KeychainOnly`, and in the
        // first data-protection slot otherwise.
        let slot = if protection == Protection::KeychainOnly {
            Slot::Login
        } else {
            Slot::Primary
        };
        Some(slot.name().to_string())
    }
}

/// [`Slots`] on Apple's keychain: what each step of a save is there.
#[cfg(target_vendor = "apple")]
struct AppleSlots<'a> {
    /// The sign-in being written, as the item's password.
    text: &'a [u8],
    /// Whether the new item asks for a fingerprint.
    locked: bool,
}

#[cfg(target_vendor = "apple")]
impl Slot {
    /// The account name of a data-protection slot; `None` for the login keychain.
    fn protected_account(self) -> Option<&'static str> {
        match self {
            Slot::Primary => Some(KEYCHAIN_ACCOUNT),
            Slot::Secondary => Some(KEYCHAIN_ACCOUNT_NEXT),
            Slot::Login => None,
        }
    }
}

#[cfg(target_vendor = "apple")]
impl Slots for AppleSlots<'_> {
    fn occupied(&self, slot: Slot) -> Result<bool, String> {
        let Some(account) = slot.protected_account() else {
            return Ok(false);
        };
        // The attributes and not the password: an item behind a fingerprint asks
        // for one when its *data* is read, and this must never be a prompt.
        let mut search = security_framework::item::ItemSearchOptions::new();
        search
            .class(security_framework::item::ItemClass::generic_password())
            .service(KEYCHAIN_SERVICE)
            .account(account)
            .load_attributes(true);
        // On macOS the same service and account also name the login-keychain item,
        // which is not this slot.
        #[cfg(target_os = "macos")]
        search.ignore_legacy_keychains();
        match search.search() {
            Ok(found) => Ok(!found.is_empty()),
            Err(error)
                if error.code() == ERR_SEC_ITEM_NOT_FOUND
                    || error.code() == ERR_SEC_MISSING_ENTITLEMENT =>
            {
                Ok(false)
            }
            // Anything else — a keychain that would have to ask before answering
            // included — is not an answer either way. Free would *update* an item
            // that is there and keep its old access control; taken would have a save
            // with both slots "held" remove the other one, which may be the only
            // copy. Nothing has been changed yet, so the save stops here.
            Err(error) => Err(describe_keychain(&error)),
        }
    }

    fn add(&self, slot: Slot) -> Result<(), AddRefused> {
        let Some(account) = slot.protected_account() else {
            // The login keychain holds no access control, so its one item can be
            // updated in place, which is all-or-nothing: no copy has to go first.
            return security_framework::passwords::set_generic_password(
                KEYCHAIN_SERVICE,
                KEYCHAIN_ACCOUNT,
                self.text,
            )
            .map_err(|e| AddRefused::Failed(describe_keychain(&e)));
        };
        let mut with_control = protected(account);
        with_control.set_access_control(access_control(self.locked).map_err(AddRefused::Failed)?);
        // An add, because the slot is free — which is why there are two. Given an
        // item that exists, this call updates its password and keeps the access
        // control it was made with.
        match security_framework::passwords::set_generic_password_options(self.text, with_control) {
            Ok(()) => Ok(()),
            // Only the missing-entitlement case falls back. Any other refusal is a
            // real problem and is reported rather than worked around, because a
            // second attempt would only fail more quietly.
            Err(error) if error.code() == ERR_SEC_MISSING_ENTITLEMENT => {
                Err(AddRefused::Unreachable)
            }
            Err(error) => Err(AddRefused::Failed(describe_keychain(&error))),
        }
    }

    fn remove(&self, slot: Slot) -> Result<(), String> {
        let removed = match slot.protected_account() {
            Some(account) => {
                security_framework::passwords::delete_generic_password_options(protected(account))
            }
            // See [`HAS_LOGIN_KEYCHAIN`]: on iOS this query would name a
            // data-protection item, quite possibly the one just written.
            None if !HAS_LOGIN_KEYCHAIN => return Ok(()),
            None => security_framework::passwords::delete_generic_password(
                KEYCHAIN_SERVICE,
                KEYCHAIN_ACCOUNT,
            ),
        };
        match removed {
            Ok(()) => Ok(()),
            Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
            // The keychain could not be opened at all: there is no item in it that
            // this build could have written, so there is nothing to remove.
            Err(error) if error.code() == ERR_SEC_MISSING_ENTITLEMENT => Ok(()),
            Err(error) => Err(describe_keychain(&error)),
        }
    }
}

/// The platform has no secret store wired up, so connecting is refused.
///
/// Not a stub waiting to be filled in with a file: see the module note. A refusal is
/// a bug report; a plaintext credential is a vulnerability nobody notices.
#[cfg(not(any(target_vendor = "apple", target_os = "android")))]
struct NoSecureStore;

#[cfg(not(any(target_vendor = "apple", target_os = "android")))]
impl TokenStore for NoSecureStore {
    fn available(&self) -> bool {
        false
    }
    fn can_lock(&self) -> bool {
        false
    }
    fn load(&self, _place: Option<&str>) -> Result<Option<Loaded>, String> {
        Ok(None)
    }
    fn save(
        &self,
        _account: &Account,
        _locked: bool,
        _place: Option<&str>,
    ) -> Result<Saved, String> {
        Err(no_secure_store())
    }
    fn clear(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
}

/// A lock taken only if it is free. Busy is `None`; a lock left poisoned by a
/// panic in an earlier holder is taken anyway, as [`SyncService::take_switch`]
/// does — the `()` it guards cannot be half-written, and refusing would leave
/// Sync or Disconnect answering "busy" until the app restarts.
fn try_hold(lock: &Mutex<()>) -> Option<MutexGuard<'_, ()>> {
    match lock.try_lock() {
        Ok(guard) => Some(guard),
        Err(TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    }
}

// ---- tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A panic while the gate or the switch was held must not leave Sync and
    /// Disconnect answering "busy" until the app restarts.
    #[test]
    fn a_lock_left_poisoned_by_a_panic_is_taken_and_a_held_one_is_not() {
        let lock = Arc::new(Mutex::new(()));
        let poisoner = Arc::clone(&lock);
        let _ = std::thread::spawn(move || {
            let _held = poisoner.lock().unwrap();
            panic!("a holder that panicked");
        })
        .join();
        assert!(lock.is_poisoned());
        let taken = try_hold(&lock);
        assert!(taken.is_some(), "a poisoned lock is still free to take");
        assert!(try_hold(&lock).is_none(), "and a held one is busy");
    }
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// A secret store that keeps its one secret in memory.
    ///
    /// The point of the tests below is that they never touch the real Keychain: one
    /// that wrote to the developer's would be one that deleted their account.
    #[derive(Default)]
    struct MemoryStore {
        /// The sign-in, with how the write that stored it left it protected — which
        /// is the fact the service cares about, and which only the store knows.
        account: Mutex<Option<(Account, Protection)>>,
        available: bool,
        can_lock: bool,
        /// How many times the secret was actually read, which is the number this
        /// module's whole design is about: it is what a learner experiences as a
        /// fingerprint prompt.
        reads: Mutex<usize>,
        /// What a write reports back, so the "the platform could not honour the
        /// request" path can be driven.
        reports: Mutex<Option<Protection>>,
        /// A store that fails every read, to prove that some paths do not read.
        refuse: bool,
        /// Why every write fails, when one should: what Android answers when the
        /// learner cancels the fingerprint prompt that locking the sign-in shows.
        /// The item already stored is left as it was, as the real store leaves it.
        refuse_saves: Mutex<Option<String>>,
        /// What a read reports, whatever the write stored.
        ///
        /// `None` — the truth — is what a store that round-trips faithfully does, and
        /// what every other test wants. This exists because the Apple keychain's read
        /// cannot answer that question at all: the password comes back and the access
        /// control on it does not, so an item written with no constraint and one behind
        /// a fingerprint are the same read. `Some(Protection::Unknown)` is what it
        /// answers now that it says so honestly, and `Some(Protection::UserPresence)`
        /// is what it used to answer — the guess the bug was made of. A double that can
        /// only tell the truth is a double that cannot express either.
        read_reports: Option<Protection>,
        /// Where a write says it put the item, for the record; `None` is a store
        /// with one place, which is every store but Apple's.
        place: Option<&'static str>,
        /// The place every read and write was handed, in order.
        places: Mutex<Vec<Option<String>>>,
        /// Where [`TokenStore::locate`] says the item really is; `None` agrees with
        /// whatever it is asked about.
        located: Option<&'static str>,
        /// What a clear says it left behind, as a login copy that would not go.
        leaves_on_clear: Option<&'static str>,
        /// What [`TokenStore::place_past_leftovers`] answers.
        past_leftovers: Option<&'static str>,
    }

    impl MemoryStore {
        fn working() -> Self {
            Self {
                account: Mutex::new(None),
                available: true,
                can_lock: true,
                reads: Mutex::new(0),
                reports: Mutex::new(None),
                refuse: false,
                refuse_saves: Mutex::new(None),
                read_reports: None,
                place: None,
                places: Mutex::new(Vec::new()),
                located: None,
                leaves_on_clear: None,
                past_leftovers: None,
            }
        }

        fn unavailable() -> Self {
            Self {
                available: false,
                ..Self::working()
            }
        }

        /// A store that reports this protection for whatever it holds. Used to drive
        /// the fallback an Apple build without an application identifier lands in,
        /// which is not something a test can bring about on a real keychain.
        fn reporting(protection: Protection) -> Self {
            Self {
                reports: Mutex::new(Some(protection)),
                ..Self::working()
            }
        }

        /// A store that will not be read. What a run of the app that never needs the
        /// token — every run that only draws the screen — must be able to work with.
        fn refusing() -> Self {
            Self {
                refuse: true,
                ..Self::working()
            }
        }

        /// A store that keeps the sign-in but cannot say how it is protected, and says
        /// so. What an Apple data-protection item looks like from the reading side.
        fn undecided() -> Self {
            Self {
                read_reports: Some(Protection::Unknown),
                ..Self::working()
            }
        }

        /// A store whose read cannot tell either, and guesses the worst — which is what
        /// the Apple keychain used to answer before it was taught to say `Unknown`, and
        /// what the reported bug was made of.
        fn guessing() -> Self {
            Self {
                read_reports: Some(Protection::UserPresence),
                ..Self::working()
            }
        }

        fn reads(&self) -> usize {
            *self.reads.lock().unwrap()
        }
    }

    impl TokenStore for MemoryStore {
        fn available(&self) -> bool {
            self.available
        }
        fn can_lock(&self) -> bool {
            self.can_lock
        }
        fn load(&self, place: Option<&str>) -> Result<Option<Loaded>, String> {
            self.places.lock().unwrap().push(place.map(str::to_string));
            *self.reads.lock().unwrap() += 1;
            if self.refuse {
                return Err("this store was not supposed to be read".to_string());
            }
            // The protection a read can report is not always the one the write
            // managed — that is the whole point of `read_reports`.
            Ok(self
                .account
                .lock()
                .unwrap()
                .clone()
                .map(|(account, stored)| Loaded {
                    account,
                    protection: self.read_reports.unwrap_or(stored),
                    place: self.place.map(str::to_string),
                }))
        }
        fn save(
            &self,
            account: &Account,
            locked: bool,
            place: Option<&str>,
        ) -> Result<Saved, String> {
            self.places.lock().unwrap().push(place.map(str::to_string));
            if let Some(why) = self.refuse_saves.lock().unwrap().clone() {
                return Err(why);
            }
            // A store that can honour the request does, which is what the real one
            // does wherever the system lets it. `reports` overrides that, and is how
            // the tests drive the case where it cannot — the login-keychain fallback,
            // which is a build the system does not recognise.
            let protection = self.reports.lock().unwrap().unwrap_or(if locked {
                Protection::UserPresence
            } else {
                Protection::DeviceOnly
            });
            *self.account.lock().unwrap() = Some((account.clone(), protection));
            Ok(Saved {
                protection,
                place: self.place.map(str::to_string),
            })
        }
        fn clear(&self) -> Result<Option<String>, String> {
            *self.account.lock().unwrap() = None;
            Ok(self.leaves_on_clear.map(str::to_string))
        }
        fn locate(&self, place: Option<&str>) -> Result<Option<String>, String> {
            Ok(self.located.or(place).map(str::to_string))
        }
        fn place_past_leftovers(&self) -> Option<String> {
            self.past_leftovers.map(str::to_string)
        }
    }

    /// A reachability probe that answers what the test says, so no test here opens
    /// a socket.
    struct FakeReach(bool);

    impl Reach for FakeReach {
        fn reachable(&self) -> bool {
            self.0
        }
    }

    /// The same store behind a handle the test keeps, so it can count the reads and
    /// look at what was written — which is how the tests about *how often the secret
    /// is read* are able to say anything at all.
    ///
    /// This is what a fingerprint prompt looks like from in here: a read is the one
    /// moment a learner is asked for anything, so `reads()` is the number of prompts.
    impl TokenStore for std::sync::Arc<MemoryStore> {
        fn available(&self) -> bool {
            self.as_ref().available()
        }
        fn can_lock(&self) -> bool {
            self.as_ref().can_lock()
        }
        fn load(&self, place: Option<&str>) -> Result<Option<Loaded>, String> {
            self.as_ref().load(place)
        }
        fn save(
            &self,
            account: &Account,
            locked: bool,
            place: Option<&str>,
        ) -> Result<Saved, String> {
            self.as_ref().save(account, locked, place)
        }
        fn clear(&self) -> Result<Option<String>, String> {
            self.as_ref().clear()
        }
        fn locate(&self, place: Option<&str>) -> Result<Option<String>, String> {
            self.as_ref().locate(place)
        }
        fn place_past_leftovers(&self) -> Option<String> {
            self.as_ref().place_past_leftovers()
        }
    }

    /// An HTTP client that answers the endpoints a sync uses and records what it was
    /// asked, so the OAuth calls and a whole sync can be driven without a socket.
    ///
    /// `Clone`, and the clone shares the recording — which is how a test keeps a
    /// handle on what was sent after the service has taken ownership of its copy.
    /// A newtype would do the same; this is smaller.
    #[derive(Clone, Default)]
    struct FakeHttp {
        inner: std::sync::Arc<FakeInner>,
    }

    #[derive(Default)]
    struct FakeInner {
        forms: Mutex<Vec<String>>,
        revocations: Mutex<Vec<String>>,
        token_response: Mutex<Option<String>>,
        /// What `rpc` answers. The default is an empty app folder, which is what a
        /// Dropbox nobody else has written to looks like — and it is enough for a
        /// whole sync, because an empty log writes no shards and has nothing to pull.
        rpc_response: Mutex<Option<String>>,
        /// Every request, in order, so a test can assert that *nothing* was sent.
        /// That is the claim the offline and locked cases rest on, and it cannot be
        /// checked by looking at the outcome alone.
        calls: Mutex<Vec<String>>,
    }

    impl FakeHttp {
        fn answering(body: serde_json::Value) -> Self {
            Self {
                inner: std::sync::Arc::new(FakeInner {
                    token_response: Mutex::new(Some(body.to_string())),
                    ..FakeInner::default()
                }),
            }
        }

        fn calls(&self) -> usize {
            self.inner.calls.lock().unwrap().len()
        }

        fn note(&self, call: impl Into<String>) {
            self.inner.calls.lock().unwrap().push(call.into());
        }
    }

    impl Http for FakeHttp {
        fn rpc(&self, url: &str, bearer: &str, _body: &str) -> Result<String, SyncError> {
            self.note(url);
            self.inner.revocations.lock().unwrap().push(bearer.to_string());
            Ok(self.inner.rpc_response.lock().unwrap().clone().unwrap_or_else(|| {
                serde_json::json!({ "entries": [], "cursor": "c", "has_more": false }).to_string()
            }))
        }
        fn form(&self, url: &str, body: &str) -> Result<String, SyncError> {
            self.note(url);
            self.inner.forms.lock().unwrap().push(body.to_string());
            self.inner
                .token_response
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SyncError::Io("the fake was not given a token response".into()))
        }
        fn upload(&self, url: &str, _bearer: &str, _arg: &str, _body: &[u8]) -> Result<String, SyncError> {
            self.note(url);
            // The client ignores the body of an upload — a shard is written once and
            // its name already says what is in it — so this only has to be JSON.
            Ok("{}".to_string())
        }
        fn download(&self, url: &str, _bearer: &str, _arg: &str) -> Result<Vec<u8>, SyncError> {
            self.note(url);
            unreachable!("the fake lists an empty app folder, so there is nothing to read")
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("hanzi-sync-service-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// A service over a store the test still has a handle on.
    ///
    /// Always with a probe that says the network is there; [`with_store_offline`] is
    /// the other one, and `a_launch_offline_says_so_without_attempting_anything`
    /// is the test that needs it.
    fn with_store(
        db: Option<Db>,
        store: &std::sync::Arc<MemoryStore>,
        http: Box<dyn Http>,
    ) -> SyncService {
        with_probe(db, store, http, true)
    }

    fn with_probe(
        db: Option<Db>,
        store: &std::sync::Arc<MemoryStore>,
        http: Box<dyn Http>,
        reachable: bool,
    ) -> SyncService {
        SyncService::with_parts(
            db,
            Box::new(std::sync::Arc::clone(store)),
            http,
            Box::new(FakeReach(reachable)),
        )
    }

    /// A service whose HTTP client the test can still see, which is what the tests
    /// about *whether anything was sent* need.
    fn with_http(
        db: Option<Db>,
        store: &std::sync::Arc<MemoryStore>,
        http: &FakeHttp,
        reachable: bool,
    ) -> SyncService {
        SyncService::with_parts(
            db,
            Box::new(std::sync::Arc::clone(store)),
            Box::new(http.clone()),
            Box::new(FakeReach(reachable)),
        )
    }

    /// The token endpoint's answer: a refresh token, and an account to name.
    fn connected_account() -> serde_json::Value {
        serde_json::json!({
            "access_token": "sl.access",
            "refresh_token": "refresh-me",
            "account_id": "dbid:AAAA"
        })
    }

    /// The HTTP client every test that connects wants.
    fn connecting() -> Box<dyn Http> {
        Box::new(FakeHttp::answering(connected_account()))
    }

    /// The same, behind a handle, for the tests that ask what was sent.
    fn connecting_http() -> FakeHttp {
        FakeHttp::answering(connected_account())
    }

    /// A service with no database, over a store the test names and a probe that says
    /// the network is there. Most of the tests want nothing else.
    fn bare(store: MemoryStore) -> SyncService {
        let store = std::sync::Arc::new(store);
        with_probe(None, &store, Box::new(FakeHttp::default()), true)
    }

    /// A `Db` in a directory of its own, named for the test that wants it.
    fn database(name: &str) -> Db {
        Db::open(scratch(name)).unwrap()
    }

    /// Connect this service to Dropbox, the way the settings screen does.
    fn connect(service: &SyncService) {
        service.begin().unwrap();
        let view = service.finish("the-code").unwrap();
        assert!(view.connected, "{}", view.message);
    }

    #[test]
    fn a_view_says_whether_there_is_a_secret_store_and_an_account() {
        let view = bare(MemoryStore::working()).view();
        assert!(view.can_connect, "this platform can keep a secret");
        assert!(!view.connected, "but nobody has connected");
        assert!(view.last.is_none());
        assert!(view.message.contains("stays on this device"), "{}", view.message);

        // A platform with no secret store says so rather than offering a button.
        let unavailable = bare(MemoryStore::unavailable());
        assert!(!unavailable.view().can_connect);
        let refusal = unavailable.begin().unwrap_err();
        assert!(refusal.contains("no secure store"), "{refusal}");
        assert!(refusal.contains("unaffected"), "and it should reassure: {refusal}");
    }

    #[test]
    fn beginning_an_authorization_returns_a_url_to_open() {
        let service = bare(MemoryStore::working());
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
        let service = bare(MemoryStore::working());
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

    /// The one platform call that cannot be covered any other way.
    ///
    /// Building an access control touches no keychain — it is an object in this
    /// process — so this is safe to run here, and it is worth running: the default
    /// path passes **no** constraint flags, and a combination the system rejects with
    /// `errSecParam` would be a connect that fails at the last step and only on a real
    /// device. Both shapes are checked, because both are reachable from the switch.
    #[cfg(target_vendor = "apple")]
    #[test]
    fn both_access_controls_are_ones_the_system_will_build() {
        for locked in [false, true] {
            let control = access_control(locked);
            assert!(control.is_ok(), "locked = {locked}: {:?}", control.err());
        }
    }

    /// Apple's keychain in memory, for the order a save takes its steps in.
    ///
    /// Each item is named by what wrote it, so a test can say which copy a read
    /// would find. `fail_at` makes those steps fail, which is how every way a
    /// save can be cut short is reached.
    struct FakeSlots {
        items: Mutex<BTreeMap<Slot, &'static str>>,
        /// A build that may not use the data-protection keychain at all.
        unreachable: bool,
        fail_at: Vec<usize>,
        /// Every step from this one on fails and does nothing: the process died
        /// there, so nothing after it — a rollback included — happened.
        crash_from: Option<usize>,
        steps: Mutex<usize>,
    }

    impl FakeSlots {
        fn over(items: &[(Slot, &'static str)], unreachable: bool, fail_at: Option<usize>) -> Self {
            Self {
                items: Mutex::new(items.iter().copied().collect()),
                unreachable,
                fail_at: fail_at.into_iter().collect(),
                crash_from: None,
                steps: Mutex::new(0),
            }
        }

        fn failing(&self) -> bool {
            let mut step = self.steps.lock().unwrap();
            let failing =
                self.fail_at.contains(&*step) || self.crash_from.is_some_and(|from| *step >= from);
            *step += 1;
            failing
        }

        /// What a load would find, starting where a record naming `place` says.
        fn read(&self, place: Option<Slot>) -> Option<&'static str> {
            let items = self.items.lock().unwrap();
            Slot::read_order(place)
                .iter()
                .find_map(|slot| items.get(slot).copied())
        }

        fn items(&self) -> BTreeMap<Slot, &'static str> {
            self.items.lock().unwrap().clone()
        }
    }

    impl Slots for FakeSlots {
        fn occupied(&self, slot: Slot) -> Result<bool, String> {
            if self.failing() {
                return Err("the keychain could not be searched".to_string());
            }
            Ok(slot != Slot::Login
                && !self.unreachable
                && self.items.lock().unwrap().contains_key(&slot))
        }
        fn add(&self, slot: Slot) -> Result<(), AddRefused> {
            if self.failing() {
                return Err(AddRefused::Failed(
                    "the keychain refused the new item".to_string(),
                ));
            }
            if slot != Slot::Login && self.unreachable {
                return Err(AddRefused::Unreachable);
            }
            let mut items = self.items.lock().unwrap();
            assert!(
                slot == Slot::Login || !items.contains_key(&slot),
                "an add into a held slot updates the old item and keeps its access control"
            );
            items.insert(slot, "new");
            Ok(())
        }
        fn remove(&self, slot: Slot) -> Result<(), String> {
            if self.failing() {
                return Err("the keychain would not remove an item".to_string());
            }
            if slot == Slot::Login || !self.unreachable {
                self.items.lock().unwrap().remove(&slot);
            }
            Ok(())
        }
    }

    #[test]
    fn the_fingerprint_switch_writes_the_new_item_beside_the_old_one_before_removing_it() {
        // The access control is fixed when an item is made, so each press is a new
        // item — in the slot the old one is not in.
        let keychain = FakeSlots::over(&[(Slot::Primary, "old")], false, None);
        let slot = replace_sign_in(&keychain, Some(Slot::Primary), false).unwrap();
        assert_eq!(slot, Slot::Secondary);
        assert_eq!(keychain.items(), BTreeMap::from([(Slot::Secondary, "new")]));

        // And back again on the next one.
        *keychain.items.lock().unwrap() = BTreeMap::from([(Slot::Secondary, "old")]);
        let slot = replace_sign_in(&keychain, Some(Slot::Secondary), true).unwrap();
        assert_eq!(slot, Slot::Primary);
        assert_eq!(keychain.items(), BTreeMap::from([(Slot::Primary, "new")]));

        // A copy an earlier build left in the login keychain goes too.
        let upgraded = FakeSlots::over(&[(Slot::Login, "old")], false, None);
        assert_eq!(
            replace_sign_in(&upgraded, Some(Slot::Login), false).unwrap(),
            Slot::Primary
        );
        assert_eq!(upgraded.items(), BTreeMap::from([(Slot::Primary, "new")]));

        // An unsigned build updates the login item in place.
        let unsigned = FakeSlots::over(&[(Slot::Login, "old")], true, None);
        assert_eq!(
            replace_sign_in(&unsigned, Some(Slot::Login), false).unwrap(),
            Slot::Login
        );
        assert_eq!(unsigned.items(), BTreeMap::from([(Slot::Login, "new")]));

        // The record's name for each slot is one it reads back.
        for slot in Slot::READ_ORDER {
            assert_eq!(Slot::named(slot.name()), Some(slot));
        }
        assert_eq!(Slot::named("something else"), None);
    }

    #[test]
    fn a_refused_add_keeps_the_sign_in_the_keychain_had() {
        // The bug: both items were deleted before the new one was added, so a
        // refused add lost the refresh token while the screen said connected.
        let keychain = FakeSlots::over(&[(Slot::Primary, "old")], false, Some(2));
        let refusal = replace_sign_in(&keychain, Some(Slot::Primary), false).unwrap_err();
        assert!(refusal.contains("refused the new item"), "{refusal}");
        assert_eq!(keychain.items(), BTreeMap::from([(Slot::Primary, "old")]));
    }

    #[test]
    fn a_search_that_cannot_answer_changes_nothing() {
        // The first slot cannot be searched — on a real keychain, one that would have
        // to ask before it answered. Taking it for held would have the save treat
        // both slots as held and remove the other: here, the only copy there is.
        let keychain = FakeSlots::over(&[(Slot::Secondary, "old")], false, Some(0));
        let refusal = replace_sign_in(&keychain, Some(Slot::Secondary), false).unwrap_err();
        assert!(refusal.contains("could not be searched"), "{refusal}");
        assert_eq!(keychain.items(), BTreeMap::from([(Slot::Secondary, "old")]));
    }

    #[test]
    fn a_login_copy_that_will_not_go_is_left_beside_an_unlocked_item_but_not_a_locked_one() {
        // Removing a login-keychain item another build wrote can ask for the keychain
        // password. Steps: two searches, the add, then that removal.
        let unlocked = FakeSlots::over(&[(Slot::Login, "old")], false, Some(3));
        assert_eq!(
            replace_sign_in(&unlocked, Some(Slot::Login), false).unwrap(),
            Slot::Primary
        );
        assert_eq!(
            unlocked.items(),
            BTreeMap::from([(Slot::Primary, "new"), (Slot::Login, "old")]),
            "connected, with the old copy left where the record no longer points"
        );
        assert_eq!(unlocked.read(Some(Slot::Primary)), Some("new"));

        // Beside a locked item the copy left behind would be the way round the lock.
        let locked = FakeSlots::over(&[(Slot::Login, "old")], false, Some(3));
        let refusal = replace_sign_in(&locked, Some(Slot::Login), true).unwrap_err();
        assert!(refusal.contains("could not be removed"), "{refusal}");
        assert_eq!(locked.items(), BTreeMap::from([(Slot::Login, "old")]));
    }

    #[test]
    fn a_record_naming_an_empty_slot_is_located_where_the_copy_really_is() {
        // What a reinstall that kept the keychain leaves, or a record write that
        // failed after a save: the record says one slot, the copy is in the other.
        let keychain = FakeSlots::over(&[(Slot::Secondary, "old")], false, None);
        assert_eq!(
            locate_slot(&keychain, Some(Slot::Primary)).unwrap(),
            Slot::Secondary
        );
        assert_eq!(
            locate_slot(&keychain, Some(Slot::Secondary)).unwrap(),
            Slot::Secondary
        );
        assert_eq!(locate_slot(&keychain, None).unwrap(), Slot::Secondary);
        // A record naming the login keychain is believed: it cannot be looked in
        // without reading it.
        assert_eq!(
            locate_slot(&keychain, Some(Slot::Login)).unwrap(),
            Slot::Login
        );

        // And a save given such a record changes nothing rather than write the new
        // item where a read starting from it looks first.
        let refusal = replace_sign_in(&keychain, Some(Slot::Primary), false).unwrap_err();
        assert!(refusal.contains("out of date"), "{refusal}");
        assert_eq!(keychain.items(), BTreeMap::from([(Slot::Secondary, "old")]));
        // Put right, it goes through.
        assert_eq!(
            replace_sign_in(&keychain, Some(Slot::Secondary), false).unwrap(),
            Slot::Primary
        );
        assert_eq!(keychain.items(), BTreeMap::from([(Slot::Primary, "new")]));
    }

    #[test]
    fn a_record_naming_a_data_protection_slot_never_reads_the_login_keychain() {
        // The login copy is one a save or a disconnect could not remove.
        let keychain = FakeSlots::over(&[(Slot::Login, "left behind")], false, None);
        assert_eq!(keychain.read(Some(Slot::Primary)), None);
        assert_eq!(keychain.read(Some(Slot::Secondary)), None);
        // With no record, or one naming it, it is a sign-in an ad-hoc build kept.
        assert_eq!(keychain.read(None), Some("left behind"));
        assert_eq!(keychain.read(Some(Slot::Login)), Some("left behind"));
    }

    #[test]
    fn clearing_the_keychain_tries_every_copy_and_reports_the_first_refusal() {
        let keychain = FakeSlots::over(
            &[
                (Slot::Primary, "a"),
                (Slot::Secondary, "b"),
                (Slot::Login, "c"),
            ],
            false,
            Some(0),
        );
        let refusal = clear_slots(&keychain).unwrap_err();
        assert!(refusal.contains("would not remove"), "{refusal}");
        assert_eq!(keychain.items(), BTreeMap::from([(Slot::Primary, "a")]));

        // Only the login copy refusing is not a failure to forget the sign-in — no
        // read starting from a data-protection slot reaches it — but it is said.
        let keychain = FakeSlots::over(&[(Slot::Primary, "a"), (Slot::Login, "c")], false, Some(2));
        let left = clear_slots(&keychain)
            .unwrap()
            .expect("the copy left is reported");
        assert!(left.contains("Keychain Access"), "{left}");
        assert_eq!(keychain.items(), BTreeMap::from([(Slot::Login, "c")]));
        assert_eq!(
            keychain.read(Some(Slot::Primary)),
            None,
            "and nothing reads it"
        );
    }

    /// How a save is cut short in [`a_save_on_the_keychain_cut_short_anywhere_reads_what_the_record_says`].
    #[derive(Clone, Copy, Debug)]
    enum Cut {
        /// One step fails, and the save carries on as it would.
        Refused(usize),
        /// A removal fails and so does the rollback straight after it.
        RollbackRefused(usize),
        /// The app quits at a step: nothing from there on happens.
        Quit(usize),
    }

    #[test]
    fn a_save_on_the_keychain_cut_short_anywhere_reads_what_the_record_says() {
        // Every set of copies a keychain can hold, with the record naming each of
        // them, and the save stopped at every step in every way. The record is
        // written only once a save returns, so after one that did not, a read must
        // find the item the record still describes — not the new one under an old
        // record, which is a switch saying "on" over an item that asks for nothing,
        // or a launch that prompts. After one that did, a read from the place it
        // returned finds the new item, and no old copy is left that could be read
        // instead or that keeps the protection the learner asked to replace.
        let named = [
            (Slot::Primary, "primary"),
            (Slot::Secondary, "secondary"),
            (Slot::Login, "login"),
        ];
        let mut cuts = vec![None];
        for step in 0..8 {
            cuts.extend(
                [
                    Cut::Refused(step),
                    Cut::RollbackRefused(step),
                    Cut::Quit(step),
                ]
                .map(Some),
            );
        }
        for unreachable in [false, true] {
            for held in 0..8u8 {
                let items: Vec<_> = named
                    .iter()
                    .enumerate()
                    .filter(|(bit, _)| held & (1 << bit) != 0)
                    .map(|(_, item)| *item)
                    .collect();
                // A build that cannot open the data-protection keychain sees only
                // the login one.
                if unreachable && items.iter().any(|(slot, _)| *slot != Slot::Login) {
                    continue;
                }
                // Whatever the record names, whether or not anything is there: a
                // record can name an empty slot — one whose write failed after a
                // save, one from a build with one place, a reinstall that kept the
                // keychain and not the database.
                let places = [
                    None,
                    Some(Slot::Primary),
                    Some(Slot::Secondary),
                    Some(Slot::Login),
                ];
                for place in places {
                    let holds = |slot: Slot| items.iter().any(|(held, _)| *held == slot);
                    // The one case this cannot get right, and does not claim to: a
                    // record naming the login keychain while it is empty and a
                    // data-protection copy is held. The login slot cannot be looked
                    // for without reading it, so it is taken for held — see
                    // `replace_sign_in` — and a new item can then land where a read
                    // falling past the empty login slot looks first.
                    if place == Some(Slot::Login)
                        && !holds(Slot::Login)
                        && (holds(Slot::Primary) || holds(Slot::Secondary))
                    {
                        continue;
                    }
                    for locked in [false, true] {
                        for cut in &cuts {
                            let keychain = match cut {
                                None => FakeSlots::over(&items, unreachable, None),
                                Some(Cut::Refused(step)) => {
                                    FakeSlots::over(&items, unreachable, Some(*step))
                                }
                                Some(Cut::RollbackRefused(step)) => FakeSlots {
                                    fail_at: vec![*step, *step + 1],
                                    ..FakeSlots::over(&items, unreachable, None)
                                },
                                Some(Cut::Quit(step)) => FakeSlots {
                                    crash_from: Some(*step),
                                    ..FakeSlots::over(&items, unreachable, None)
                                },
                            };
                            let before = keychain.read(place);
                            let case = format!(
                                "held {items:?}, record at {place:?}, locked {locked}, \
                                 unreachable {unreachable}, cut {cut:?}"
                            );
                            let result = replace_sign_in(&keychain, place, locked);
                            let taken = *keychain.steps.lock().unwrap();
                            // A record under which a read finds nothing at all is out
                            // of step already — the service forgets it at its next
                            // read — and there is nothing for this to keep.
                            //
                            // The app quit inside the save, whatever the save went on
                            // to return: the record was never rewritten. (Quitting
                            // after it returned and before the record was written is
                            // the window a two-phase save would close.)
                            let quit_inside = matches!(cut, Some(Cut::Quit(step)) if *step < taken);
                            if result.is_err() || quit_inside {
                                if cut.is_none() {
                                    // The one refusal with nothing cut: a record naming an
                                    // empty slot while the copy is in the other.
                                    let why = result.as_ref().unwrap_err();
                                    assert!(why.contains("out of date"), "{case}: {why}");
                                    assert_eq!(
                                        keychain.items(),
                                        items.iter().copied().collect(),
                                        "{case}"
                                    );
                                }
                                if place.is_some() && before.is_some() {
                                    assert_eq!(keychain.read(place), before, "{case}: {result:?}");
                                } else if before.is_some() {
                                    // With no record there is nothing for a read to
                                    // agree with, only something to find; and a new
                                    // item a first connection could not take back is
                                    // cleared by the service: see `a_first_connection_
                                    // the_store_could_not_take_back_is_cleared_…`.
                                    assert!(keychain.read(place).is_some(), "{case}: {result:?}");
                                }
                                if matches!(cut, Some(Cut::Refused(_))) {
                                    assert!(
                                        !keychain.items().values().any(|v| *v == "new"),
                                        "{case}: a save reported as failed left its item"
                                    );
                                }
                                continue;
                            }
                            let slot = result.unwrap();
                            assert_eq!(keychain.read(Some(slot)), Some("new"), "{case}");
                            assert_eq!(slot == Slot::Login, unreachable, "{case}");
                            let mut left = keychain.items();
                            left.remove(&slot);
                            // The one copy allowed to stay: see
                            // `a_login_copy_that_will_not_go_is_left_…`.
                            if !locked && cut.is_some() {
                                left.remove(&Slot::Login);
                            }
                            assert!(left.is_empty(), "{case}: left behind {left:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_summary_becomes_a_sentence() {
        let busy = hanzi_sync::Summary {
            published: 3,
            pulled: 2,
            recomputed: 1,
            ..hanzi_sync::Summary::default()
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
    fn the_protection_of_the_stored_sign_in_reaches_the_screen() {
        // The screen has to be able to say "this one asks for your fingerprint" or
        // "this one does not", because the fallback is exactly where the keychain
        // password prompt comes from and a learner should not have to guess.
        for (protection, expected) in [
            (Protection::Unknown, "unknown"),
            (Protection::DeviceOnly, "deviceOnly"),
            (Protection::UserPresence, "userPresence"),
            (Protection::KeychainOnly, "keychainOnly"),
        ] {
            assert_eq!(serde_json::to_string(&protection).unwrap(), format!("\"{expected}\""));
        }

        // And what the store reports is what the screen shows, in both directions —
        // the fallback is the case worth being able to see.
        for protection in [
            Protection::DeviceOnly,
            Protection::UserPresence,
            Protection::KeychainOnly,
        ] {
            let store = std::sync::Arc::new(MemoryStore::reporting(protection));
            let service = with_store(Some(database("protection")), &store, connecting());
            // Nothing is connected, so there is nothing to report a protection for.
            assert_eq!(service.view().protection, Protection::Unknown);

            service.begin().unwrap();
            service.finish("the-code").unwrap();
            assert_eq!(service.view().protection, protection);
        }
    }

    #[test]
    fn drawing_the_screen_no_longer_reads_the_secret_store() {
        // The property the whole module is arranged around. A read is the one moment
        // a learner is asked for a fingerprint, so the settings screen — which this
        // app opens and closes all day, and which a sync at launch would draw without
        // anybody asking — must not cause one.
        let db = database("screen");
        let store = std::sync::Arc::new(MemoryStore::working());
        let service = with_store(Some(db), &store, connecting());

        service.begin().unwrap();
        let connected = service.finish("the-code").unwrap();
        assert!(connected.connected, "{}", connected.message);
        assert_eq!(store.reads(), 0, "connecting does not read back what it just wrote");

        for _ in 0..5 {
            let view = service.view();
            assert!(view.connected);
            assert_eq!(view.account_id.as_deref(), Some("dbid:AAAA"));
            assert_eq!(view.protection, Protection::DeviceOnly);
        }
        assert_eq!(store.reads(), 0, "and neither does drawing the screen, however often");
    }

    #[test]
    fn the_sign_in_is_read_once_per_run_of_the_app() {
        // The other half of the same rule: when the token *is* needed — a sync, a
        // disconnect — it is fetched once and remembered, rather than once per use.
        // A locked sign-in therefore asks for a fingerprint once per run, not once
        // per sync and never to draw the screen.
        let db = database("once");
        let store = std::sync::Arc::new(MemoryStore::working());
        let service = with_store(Some(db.clone()), &store, connecting());
        service.begin().unwrap();
        service.finish("the-code").unwrap();

        // The run that connected already holds the token, so it reads nothing at all.
        assert_eq!(store.reads(), 0);
        assert_eq!(service.account().unwrap().refresh_token, "refresh-me");
        assert!(service.view().connected);
        assert_eq!(store.reads(), 0, "the token this run just wrote is the one it uses");

        // A later run has to fetch it — and fetches it once, however many times it is
        // used. Two of these would be two prompts for a locked sign-in.
        let next = with_store(Some(db), &store, Box::new(FakeHttp::default()));
        assert_eq!(store.reads(), 0, "and nothing is read merely to build the service");
        assert_eq!(next.account().unwrap().refresh_token, "refresh-me");
        assert_eq!(store.reads(), 1, "the first thing that needs it reads it");
        assert_eq!(next.account().unwrap().refresh_token, "refresh-me");
        assert!(next.view().connected);
        assert_eq!(next.view().protection, Protection::DeviceOnly);
        assert_eq!(store.reads(), 1, "and nothing after that asks again");
    }

    #[test]
    fn a_connection_survives_a_restart_without_the_keychain_being_touched() {
        // What the record in `meta` is for. A run of the app that only draws the
        // screen must work with a secret store that will not answer at all, because
        // on a sign-in the learner has locked, "will not answer" is a prompt they
        // have not been given yet.
        let db = database("restart");
        {
            let store = std::sync::Arc::new(MemoryStore::working());
            let first = with_store(Some(db.clone()), &store, connecting());
            first.begin().unwrap();
            first.finish("the-code").unwrap();
        }

        let store = std::sync::Arc::new(MemoryStore::refusing());
        let second = with_store(Some(db), &store, Box::new(FakeHttp::default()));
        let view = second.view();
        assert!(view.connected, "the connection is on disk: {}", view.message);
        assert_eq!(view.account_id.as_deref(), Some("dbid:AAAA"));
        assert_eq!(view.protection, Protection::DeviceOnly);
        assert_eq!(store.reads(), 0, "and the keychain was never asked");
    }

    #[test]
    fn a_sign_in_stored_before_the_record_existed_is_adopted_once() {
        // A device that connected under a build where "connected?" was answered by
        // reading the keychain has a token and no record. Reading it once and writing
        // down what is found is what keeps it connected instead of asking its owner
        // to go through the browser and paste another code.
        let db = database("adopt");
        let store = std::sync::Arc::new(MemoryStore::reporting(Protection::UserPresence));
        store
            .save(
                &Account {
                    refresh_token: "from-an-older-build".to_string(),
                    account_id: Some("dbid:OLD".to_string()),
                },
                true,
                None,
            )
            .unwrap();

        let first = with_store(Some(db.clone()), &store, Box::new(FakeHttp::default()));
        let view = first.view();
        assert!(view.connected, "the older sign-in is found: {}", view.message);
        assert_eq!(view.account_id.as_deref(), Some("dbid:OLD"));
        assert_eq!(
            view.protection,
            Protection::UserPresence,
            "and read from the keychain it was found in, not guessed at"
        );
        let reads = store.reads();

        // The next run draws the screen from the record, which is the whole point.
        let second = with_store(Some(db), &store, Box::new(FakeHttp::default()));
        assert!(second.view().connected);
        assert_eq!(store.reads(), reads, "and the adoption happens once, not every run");
    }

    #[test]
    fn a_sign_in_left_locked_by_an_older_build_is_not_synced_at_launch() {
        // The upgrade path, and the case it would otherwise get wrong. An earlier
        // build put the token behind a fingerprint and the learner never answered a
        // switch, because there was none. Left at that, every launch would read a
        // locked item — a prompt at a moment nobody chose, for ever — so adopting it
        // writes the switch down as well, and the launch does not read it at all.
        let db = database("adopt-locked");
        let store = std::sync::Arc::new(MemoryStore::working());
        store
            .save(
                &Account {
                    refresh_token: "from-an-older-build".to_string(),
                    account_id: Some("dbid:OLD".to_string()),
                },
                true,
                None,
            )
            .unwrap();

        let http = connecting_http();
        let service = with_http(Some(db.clone()), &store, &http, true);

        let view = service.view();
        assert!(view.connected, "the older sign-in is adopted: {}", view.message);
        assert_eq!(view.protection, Protection::UserPresence);
        assert!(view.locked, "and the switch now says what the item needs");
        assert_eq!(db.meta_value(LOCK_KEY).unwrap().as_deref(), Some("on"));
        assert_eq!(store.reads(), 1, "one read to adopt it, and that is the last one");

        match service.auto() {
            AutoSync::Skipped { reason } => assert!(reason.contains("fingerprint"), "{reason}"),
            other => panic!("a locked sign-in is not read at launch: {other:?}"),
        }
        assert_eq!(http.calls(), 0, "and nothing was sent");
        assert_eq!(store.reads(), 1, "nor was the keychain asked again");
    }

    #[test]
    fn a_read_that_guesses_the_worst_does_not_override_what_the_write_recorded() {
        // The bug exactly as it was, and the reason a double that can only tell the
        // truth could never catch it. Apple's read answered `UserPresence` for *every*
        // item — one with no access control and one behind a fingerprint are the same
        // read — and `protection()` preferred that answer to the record. So the first
        // time a run touched the token, which is the first sync and therefore every
        // automatic sync, the rest of that run stopped syncing by itself and the
        // settings screen said the sign-in was behind a fingerprint.
        let db = database("guessing");
        let store = std::sync::Arc::new(MemoryStore::guessing());
        let http = connecting_http();

        let service = with_http(Some(db.clone()), &store, &http, true);
        connect(&service);
        assert_eq!(service.view().protection, Protection::DeviceOnly);
        assert!(!service.view().locked, "asking for nothing is the default");

        // A later run reads the token, which is the moment the guess used to land.
        let next = with_http(Some(db), &store, &http, true);
        assert_eq!(next.account().unwrap().refresh_token, "refresh-me");
        assert_eq!(store.reads(), 1, "the token is read once, as designed");

        assert_eq!(
            next.view().protection,
            Protection::DeviceOnly,
            "the record says what the write did, not what the read guessed"
        );
        assert!(!next.view().locked);
        match next.auto() {
            AutoSync::Synced { .. } => {}
            other => panic!("an item that asks for nothing is still synced by itself: {other:?}"),
        }
    }

    #[test]
    fn a_store_that_cannot_say_how_an_item_is_protected_does_not_displace_the_record() {
        // The same shape with the honest silence the Apple read gives today. Both ends
        // have to hold: a read that guesses must not become the answer, and a read that
        // admits it cannot tell must not either — the record is what the write knew.
        let db = database("undecided");
        let store = std::sync::Arc::new(MemoryStore::undecided());
        let http = connecting_http();

        // Connecting writes definite knowledge down: the write knows what it managed.
        let service = with_http(Some(db.clone()), &store, &http, true);
        connect(&service);
        assert_eq!(service.view().protection, Protection::DeviceOnly);
        assert!(!service.view().locked, "asking for nothing is the default");

        // A later run has to read the token, and that read is the moment the store's
        // answer used to poison everything after it.
        let next = with_http(Some(db), &store, &http, true);
        assert!(next.view().connected);
        assert_eq!(store.reads(), 0, "drawing the screen still reads nothing");
        assert_eq!(next.account().unwrap().refresh_token, "refresh-me");
        assert_eq!(store.reads(), 1, "the first thing that needs it reads it once");

        assert_eq!(
            next.view().protection,
            Protection::DeviceOnly,
            "the record says what the write did, not what the read could not tell"
        );
        assert!(!next.view().locked);

        // And the consequence that matters: automatic syncing still happens.
        match next.auto() {
            AutoSync::Synced { .. } => {}
            other => panic!("an item that asks for nothing is still synced by itself: {other:?}"),
        }
        assert_eq!(store.reads(), 1, "and believing the record cost no extra read");
    }

    #[test]
    fn a_sign_in_the_store_cannot_describe_is_adopted_as_the_locked_item_it_must_be() {
        // The other half of the same mode, and the upgrade path. An item with *no
        // record at all* is the only place the store's silence is answered from the
        // history: the record and the default of asking for nothing arrived in the
        // same build, so an item predating the record was written by a build that
        // always put the token behind a fingerprint. There was no switch to leave off.
        let db = database("adopt-undecided");
        let store = std::sync::Arc::new(MemoryStore::undecided());
        store
            .save(
                &Account {
                    refresh_token: "from-an-older-build".to_string(),
                    account_id: Some("dbid:OLD".to_string()),
                },
                true,
                None,
            )
            .unwrap();

        let http = connecting_http();
        let service = with_http(Some(db.clone()), &store, &http, true);

        let view = service.view();
        assert!(view.connected, "the older sign-in is adopted: {}", view.message);
        assert_eq!(view.protection, Protection::UserPresence);
        assert!(view.locked, "and the switch now says what the item needs");
        assert_eq!(db.meta_value(LOCK_KEY).unwrap().as_deref(), Some("on"));
        assert_eq!(store.reads(), 1, "one read to adopt it, and that is the last one");

        match service.auto() {
            AutoSync::Skipped { reason } => assert!(reason.contains("fingerprint"), "{reason}"),
            other => panic!("a locked sign-in is not read at launch: {other:?}"),
        }
        assert_eq!(http.calls(), 0, "and nothing was sent");
        assert_eq!(store.reads(), 1, "nor was the keychain asked again");
    }

    #[test]
    fn the_fingerprint_is_a_choice_and_survives_disconnecting() {
        let db = database("lock");
        let store = std::sync::Arc::new(MemoryStore::working());
        let service = with_store(Some(db.clone()), &store, connecting());
        service.begin().unwrap();
        let connected = service.finish("the-code").unwrap();
        assert!(!connected.locked, "asking for nothing is the default");
        assert_eq!(connected.protection, Protection::DeviceOnly);

        // Turning it on rewrites the item: an access control is fixed when a keychain
        // item is created and cannot be changed afterwards.
        store.reports.lock().unwrap().replace(Protection::UserPresence);
        let locked = service.set_lock(true).unwrap();
        assert!(locked.locked, "{}", locked.message);
        assert_eq!(locked.protection, Protection::UserPresence);
        assert!(locked.message.contains("will now ask"), "{}", locked.message);
        assert_eq!(db.meta_value(LOCK_KEY).unwrap().as_deref(), Some("on"));

        // And turning it off asks once more, which is the last time anything does.
        store.reports.lock().unwrap().replace(Protection::DeviceOnly);
        let unlocked = service.set_lock(false).unwrap();
        assert!(!unlocked.locked);
        assert_eq!(unlocked.protection, Protection::DeviceOnly);
        assert!(unlocked.message.contains("no longer asks"), "{}", unlocked.message);

        // The preference is not part of the account: disconnecting keeps it.
        service.set_lock(true).unwrap();
        service.disconnect();
        let after = service.view();
        assert!(!after.connected);
        assert!(after.locked, "who wants a fingerprint still wants one");
        assert_eq!(db.meta_value(LOCK_KEY).unwrap().as_deref(), Some("on"));
    }

    /// The memory store, with its next write held until the test lets it go.
    ///
    /// What a slow keychain write looks like from in here — and a write behind a
    /// fingerprint is the slowest thing this module does, because it waits on a
    /// person.
    struct HeldWrite {
        inner: std::sync::Arc<MemoryStore>,
        /// Told when the held write has started, and waited on before it finishes.
        hold: Mutex<Option<(std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>)>>,
    }

    impl TokenStore for HeldWrite {
        fn available(&self) -> bool {
            self.inner.available()
        }
        fn can_lock(&self) -> bool {
            self.inner.can_lock()
        }
        fn load(&self, place: Option<&str>) -> Result<Option<Loaded>, String> {
            self.inner.load(place)
        }
        fn save(
            &self,
            account: &Account,
            locked: bool,
            place: Option<&str>,
        ) -> Result<Saved, String> {
            let held = self.hold.lock().unwrap().take();
            if let Some((started, release)) = held {
                started.send(()).unwrap();
                release.recv().unwrap();
            }
            self.inner.save(account, locked, place)
        }
        fn clear(&self) -> Result<Option<String>, String> {
            self.inner.clear()
        }
    }

    #[test]
    fn two_presses_of_the_fingerprint_switch_cannot_leave_it_disagreeing_with_the_item() {
        // `sync_set_lock` runs off the main thread (issue #26), so two of them can
        // now overlap — a second press while the first is still rewriting the item.
        // Each writes the preference and then the item, and two that interleave
        // could leave the preference saying one thing and the item the other: a
        // switch showing "off" over an item that asks for a fingerprint at every
        // launch, which is the one outcome this module exists to prevent.
        let db = database("lock-overlap");
        let store = std::sync::Arc::new(MemoryStore::working());
        connect(&with_store(Some(db.clone()), &store, connecting()));

        let (started_tx, started) = std::sync::mpsc::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let service = SyncService::with_parts(
            Some(db.clone()),
            Box::new(HeldWrite {
                inner: std::sync::Arc::clone(&store),
                hold: Mutex::new(Some((started_tx, release_rx))),
            }),
            Box::new(FakeHttp::default()),
            Box::new(FakeReach(true)),
        );

        std::thread::scope(|scope| {
            // On: its preference is written, and its write to the item is held.
            let first = scope.spawn(|| service.set_lock(true));
            started.recv().unwrap();
            // Off, pressed while the first is still writing.
            let second = scope.spawn(|| service.set_lock(false));
            // Long enough for the second to have finished, were nothing holding it.
            std::thread::sleep(Duration::from_millis(200));
            release.send(()).unwrap();
            first.join().unwrap().unwrap();
            second.join().unwrap().unwrap();
        });

        let view = service.view();
        assert!(!view.locked, "the last press was off");
        assert_eq!(
            view.protection,
            Protection::DeviceOnly,
            "and the item must say the same as the switch"
        );
        assert_eq!(
            store.account.lock().unwrap().as_ref().map(|(_, p)| *p),
            Some(Protection::DeviceOnly),
            "which is what the store itself holds"
        );
    }

    #[test]
    fn disconnecting_while_the_switch_is_changing_answers_at_once() {
        // The switch can be waiting on a fingerprint prompt for as long as the
        // learner leaves it open. Disconnect must not wait behind it holding the sync
        // gate — that would block a Sync pressed meanwhile as well — but say so and
        // leave the account alone.
        let db = database("disconnect-while-switching");
        let store = std::sync::Arc::new(MemoryStore::working());
        let service = with_store(Some(db), &store, connecting());
        connect(&service);

        let switching = service.switching.lock().unwrap();
        let (answer, answered) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            scope.spawn(|| answer.send(service.disconnect()).unwrap());
            let view = answered
                .recv_timeout(Duration::from_secs(5))
                .expect("disconnect waited behind the fingerprint switch");
            assert!(
                view.message.contains("fingerprint setting"),
                "{}",
                view.message
            );
            assert!(view.connected, "nothing was disconnected");
            assert!(service.gate.try_lock().is_ok(), "and the sync gate is free");
            drop(switching);
        });
        assert!(service.view().connected);
    }

    #[test]
    fn a_lock_the_store_refuses_leaves_the_switch_and_the_sign_in_as_they_were() {
        let db = database("lock-refused");
        let store = std::sync::Arc::new(MemoryStore::working());
        let service = with_store(Some(db.clone()), &store, connecting());
        connect(&service);

        // Locking the sign-in on Android asks for the fingerprint, because keystore
        // will not encrypt under a key that asks without one — and the learner may
        // cancel. The store then keeps the item as it was and says why.
        store.refuse_saves.lock().unwrap().replace(
            "the fingerprint prompt was cancelled, so the Dropbox sign-in was not locked, and \
             nothing was changed"
                .to_string(),
        );
        let refusal = service.set_lock(true).unwrap_err();
        assert!(refusal.contains("cancelled"), "the store's reason is passed on: {refusal}");
        assert!(refusal.contains("kept as it was"), "and says what it means: {refusal}");

        let view = service.view();
        assert!(!view.locked, "the switch must not claim a lock the sign-in does not have");
        assert_ne!(db.meta_value(LOCK_KEY).unwrap().as_deref(), Some("on"));
        assert!(view.connected, "and the sign-in is still there");
        assert_eq!(view.protection, Protection::DeviceOnly);
    }

    #[test]
    fn a_connection_the_store_refuses_is_not_connected_and_is_revoked() {
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(database("finish-refused")), &store, &http, true);
        service.set_lock(true).unwrap();

        // With the switch on, storing the new sign-in asks for the fingerprint on
        // Android, after the single-use code has already been exchanged.
        store.refuse_saves.lock().unwrap().replace(
            "the fingerprint prompt was cancelled, so the Dropbox sign-in was not saved".to_string(),
        );
        service.begin().unwrap();
        let refusal = service.finish("the-code").unwrap_err();
        assert!(refusal.starts_with("Not connected"), "{refusal}");
        assert!(refusal.contains("cancelled"), "{refusal}");
        assert!(refusal.contains("Press Connect"), "and says what to do: {refusal}");

        assert!(!service.view().connected);
        assert!(store.account.lock().unwrap().is_none(), "nothing was stored");
        assert!(
            http.inner.revocations.lock().unwrap().iter().any(|bearer| bearer == "sl.access"),
            "the token nobody kept is revoked rather than left standing"
        );
        assert!(
            http.inner.calls.lock().unwrap().iter().any(|url| url.contains("revoke")),
            "through the revoke endpoint"
        );
    }

    #[test]
    fn a_sign_in_that_cannot_be_read_leaves_the_switch_as_it_was() {
        // Turning the prompt off reads the item it is about to rewrite, and on Apple
        // that read is the fingerprint prompt — which a learner may cancel. Nothing
        // has changed then, so nothing may say it has: not the switch, which would
        // read "off" over an item that still asks at every launch, and not the
        // record of how the item is protected.
        let db = database("lock-read-refused");
        let store = std::sync::Arc::new(MemoryStore::working());
        let service = with_store(Some(db.clone()), &store, connecting());
        connect(&service);
        service.set_lock(true).unwrap();
        let before = db.meta_value(ACCOUNT_KEY).unwrap();

        // The next run of the app, whose first read of the sign-in is refused.
        let cancelling = std::sync::Arc::new(MemoryStore {
            account: Mutex::new(store.account.lock().unwrap().clone()),
            refuse: true,
            ..MemoryStore::working()
        });
        let later = with_store(Some(db.clone()), &cancelling, Box::new(FakeHttp::default()));
        assert!(
            later.set_lock(false).is_err(),
            "a read that did not happen is not a success"
        );

        assert_eq!(db.meta_value(LOCK_KEY).unwrap().as_deref(), Some("on"));
        assert_eq!(
            db.meta_value(ACCOUNT_KEY).unwrap(),
            before,
            "the record is untouched"
        );
        let view = later.view();
        assert!(view.locked, "the switch still says what the item is");
        assert_eq!(view.protection, Protection::UserPresence);
        assert_eq!(
            cancelling.account.lock().unwrap().as_ref().map(|(_, p)| *p),
            Some(Protection::UserPresence),
            "and the item was not rewritten"
        );
    }

    #[test]
    fn a_code_that_fails_to_exchange_leaves_the_sign_in_waiting_for_the_right_one() {
        // A mistyped paste, or a code from an earlier Connect tab, is refused by
        // Dropbox. The verifier must survive that: it is the only one the code from
        // the tab the learner is looking at will match, and losing it turned every
        // later paste into "there is no sign-in in progress".
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = FakeHttp::default();
        let service = with_http(Some(database("finish-retry")), &store, &http, true);
        service.begin().unwrap();

        let refused = service.finish("a-typo").unwrap_err();
        assert!(!refused.contains("no sign-in in progress"), "{refused}");
        assert!(!service.view().connected);

        // The right code, against the same authorization.
        http.inner
            .token_response
            .lock()
            .unwrap()
            .replace(connected_account().to_string());
        let view = service.finish("the-code").unwrap();
        assert!(view.connected, "{}", view.message);

        let forms = http.inner.forms.lock().unwrap().clone();
        assert_eq!(forms.len(), 2, "{forms:?}");
        let verifier = |form: &str| {
            form.split('&')
                .find(|pair| pair.starts_with("code_verifier="))
                .map(str::to_string)
        };
        assert!(verifier(&forms[0]).is_some(), "{}", forms[0]);
        assert_eq!(
            verifier(&forms[0]),
            verifier(&forms[1]),
            "one verifier, presented twice"
        );

        // A spent code is spent: once a sign-in is stored, there is none in progress.
        let again = service.finish("the-code").unwrap_err();
        assert!(again.contains("no sign-in in progress"), "{again}");
    }

    #[test]
    fn a_code_exchanged_but_not_stored_does_not_leave_the_sign_in_in_progress() {
        // The exchange spends the code, so a sign-in the store then refuses cannot be
        // retried with it: Connect has to start a new one, and the message says so.
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(database("finish-spent")), &store, &http, true);
        store
            .refuse_saves
            .lock()
            .unwrap()
            .replace("the keychain said no".to_string());
        service.begin().unwrap();
        assert!(service
            .finish("the-code")
            .unwrap_err()
            .contains("Press Connect"));

        store.refuse_saves.lock().unwrap().take();
        let again = service.finish("the-code").unwrap_err();
        assert!(again.contains("no sign-in in progress"), "{again}");
    }

    #[test]
    fn a_first_connection_the_store_could_not_take_back_is_cleared_rather_than_adopted() {
        // A save that fails with nothing recorded may still have left its new item —
        // Apple's, when a removal and the rollback after it were both refused. The
        // token in it is revoked a moment later, and a launch that found it with no
        // record would adopt it as a connection.
        let db = database("finish-leftover");
        let store = std::sync::Arc::new(MemoryStore::working());
        let service = with_store(Some(db.clone()), &store, connecting());
        service.begin().unwrap();
        // What the failed save left behind, standing in for the keychain's copy.
        *store.account.lock().unwrap() = Some((
            Account {
                refresh_token: "left-behind".to_string(),
                account_id: None,
            },
            Protection::DeviceOnly,
        ));
        store
            .refuse_saves
            .lock()
            .unwrap()
            .replace("the keychain said no".to_string());
        assert!(service.finish("the-code").is_err());
        assert!(
            store.account.lock().unwrap().is_none(),
            "nothing is left to adopt"
        );

        // With a connection recorded, a failed save leaves it alone.
        store.refuse_saves.lock().unwrap().take();
        service.begin().unwrap();
        service.finish("the-code").unwrap();
        store
            .refuse_saves
            .lock()
            .unwrap()
            .replace("the keychain said no".to_string());
        service.begin().unwrap();
        assert!(service.finish("the-code").is_err());
        assert!(
            store.account.lock().unwrap().is_some(),
            "the recorded sign-in is kept"
        );
        assert!(service.view().connected);
    }

    fn record_in(db: &Db) -> Stored {
        serde_json::from_str(&db.meta_value(ACCOUNT_KEY).unwrap().unwrap()).unwrap()
    }

    fn write_record_in(db: &Db, place: &str) {
        let mut record = record_in(db);
        record.place = Some(place.to_string());
        db.set_meta_value(ACCOUNT_KEY, &serde_json::to_string(&record).unwrap())
            .unwrap();
    }

    #[test]
    fn a_record_naming_somewhere_the_sign_in_is_not_is_put_right_by_the_first_read() {
        let db = database("place-read");
        let store = std::sync::Arc::new(MemoryStore {
            place: Some("where-it-is"),
            ..MemoryStore::working()
        });
        connect(&with_store(Some(db.clone()), &store, connecting()));
        write_record_in(&db, "somewhere-else");

        let later = with_store(Some(db.clone()), &store, Box::new(FakeHttp::default()));
        later.set_lock(true).unwrap();
        assert_eq!(record_in(&db).place.as_deref(), Some("where-it-is"));
        // And the save that followed started from where the read found it.
        assert_eq!(
            store
                .places
                .lock()
                .unwrap()
                .last()
                .cloned()
                .flatten()
                .as_deref(),
            Some("where-it-is")
        );
    }

    #[test]
    fn a_sign_in_adopted_without_a_record_is_recorded_where_it_was_found() {
        // A reinstall on iOS keeps the keychain and not the database, and the item
        // may be in either slot.
        let db = database("place-adopt");
        let store = std::sync::Arc::new(MemoryStore {
            place: Some("the-other-slot"),
            ..MemoryStore::working()
        });
        *store.account.lock().unwrap() = Some((
            Account {
                refresh_token: "kept".to_string(),
                account_id: Some("dbid:OLD".to_string()),
            },
            Protection::DeviceOnly,
        ));
        assert!(
            with_store(Some(db.clone()), &store, Box::new(FakeHttp::default()))
                .view()
                .connected
        );
        assert_eq!(record_in(&db).place.as_deref(), Some("the-other-slot"));
    }

    #[test]
    fn a_connection_starts_from_where_the_store_says_the_sign_in_really_is() {
        // Connecting reads nothing — a read is a prompt — so the store is asked
        // where the item is without reading it, and the record put right before the
        // save starts from it.
        let db = database("place-finish");
        let store = std::sync::Arc::new(MemoryStore {
            place: Some("slot-a"),
            ..MemoryStore::working()
        });
        let service = with_store(Some(db.clone()), &store, connecting());
        connect(&service);
        write_record_in(&db, "slot-gone");
        let store_located = std::sync::Arc::new(MemoryStore {
            place: Some("slot-b"),
            located: Some("slot-a"),
            account: Mutex::new(store.account.lock().unwrap().clone()),
            ..MemoryStore::working()
        });
        let service = with_store(Some(db.clone()), &store_located, connecting());
        service.begin().unwrap();
        service.finish("the-code").unwrap();
        assert_eq!(
            *store_located.places.lock().unwrap(),
            vec![Some("slot-a".to_string())],
            "the save was handed the corrected place"
        );
        assert_eq!(record_in(&db).place.as_deref(), Some("slot-b"));
    }

    #[test]
    fn a_disconnect_that_leaves_only_an_unread_login_copy_still_disconnects() {
        let db = database("leftover");
        let store = std::sync::Arc::new(MemoryStore {
            leaves_on_clear: Some("A copy in the login keychain could not be removed."),
            past_leftovers: Some("data-protection-only"),
            ..MemoryStore::working()
        });
        let service = with_store(Some(db.clone()), &store, connecting());
        connect(&service);

        let gone = service.disconnect();
        assert!(!gone.connected, "{}", gone.message);
        assert!(gone.message.starts_with("Disconnected"), "{}", gone.message);
        assert!(
            gone.message.contains("login keychain"),
            "and says what was left: {}",
            gone.message
        );
        assert!(
            !with_store(Some(db.clone()), &store, Box::new(FakeHttp::default()))
                .view()
                .connected
        );

        // A launch with no record looks for a sign-in to adopt only where that copy
        // is not.
        store.places.lock().unwrap().clear();
        let later = with_store(Some(db.clone()), &store, connecting());
        let _ = later.view();
        assert_eq!(
            *store.places.lock().unwrap(),
            vec![Some("data-protection-only".to_string())]
        );

        // Connecting again is a connection like any other, and the caution goes.
        connect(&later);
        assert_ne!(db.meta_value(LEFTOVER_KEY).unwrap().as_deref(), Some("on"));
    }

    #[test]
    fn a_failed_reconnect_over_a_record_that_cannot_be_read_does_not_wipe_the_sign_in() {
        // "No record" has to be certain before a failed save may clear the store:
        // one the database holds and this cannot parse may describe this sign-in.
        let db = database("unreadable-record");
        let store = std::sync::Arc::new(MemoryStore::working());
        let service = with_store(Some(db.clone()), &store, connecting());
        connect(&service);
        db.set_meta_value(ACCOUNT_KEY, "{written by something else")
            .unwrap();

        store
            .refuse_saves
            .lock()
            .unwrap()
            .replace("the keychain said no".to_string());
        service.begin().unwrap();
        assert!(service.finish("the-code").is_err());
        assert!(
            store.account.lock().unwrap().is_some(),
            "the sign-in is still there"
        );
    }

    #[test]
    fn where_the_store_put_the_sign_in_is_recorded_and_handed_back() {
        // Apple's keychain can hold two copies after a save cut short, and only the
        // record knows which one it describes — so the place a save answers is
        // written down with the rest, and every later read and write is told it.
        let db = database("place");
        let store = std::sync::Arc::new(MemoryStore {
            place: Some("slot-a"),
            ..MemoryStore::working()
        });
        connect(&with_store(Some(db.clone()), &store, connecting()));
        let record: Stored =
            serde_json::from_str(&db.meta_value(ACCOUNT_KEY).unwrap().unwrap()).unwrap();
        assert_eq!(record.place.as_deref(), Some("slot-a"));

        // The next run of the app: its read starts there, and so does the rewrite.
        store.places.lock().unwrap().clear();
        let later = with_store(Some(db.clone()), &store, Box::new(FakeHttp::default()));
        later.set_lock(true).unwrap();
        assert_eq!(
            *store.places.lock().unwrap(),
            vec![Some("slot-a".to_string()), Some("slot-a".to_string())]
        );

        // A record from before there was a place says nothing, and is still read.
        let old: Stored =
            serde_json::from_str(r#"{"account_id":"dbid:AAAA","protection":"deviceOnly"}"#)
                .unwrap();
        assert_eq!(old.place, None);
        assert!(!serde_json::to_string(&Stored { place: None, ..old })
            .unwrap()
            .contains("place"));
    }

    #[test]
    fn a_switch_left_on_over_a_sign_in_that_does_not_ask_is_pointed_out() {
        // What every Android build before #28 left behind: the switch on, and the
        // sign-in kept unlocked because locking it always failed.
        let db = database("lock-stale");
        let store = std::sync::Arc::new(MemoryStore::working());
        let service = with_store(Some(db.clone()), &store, connecting());
        service.set_lock(true).unwrap();
        store.reports.lock().unwrap().replace(Protection::KeychainOnly);
        connect(&service);

        let later = with_store(Some(db), &store, Box::new(FakeHttp::default()));
        let view = later.view();
        assert!(view.locked && view.can_lock);
        assert_eq!(view.protection, Protection::KeychainOnly);
        assert!(view.message.contains("turn the switch off and on again"), "{}", view.message);
    }

    #[test]
    fn connecting_without_the_fingerprint_that_was_asked_for_says_so() {
        let store = std::sync::Arc::new(MemoryStore::working());
        let service = with_store(Some(database("lock-unavailable")), &store, connecting());
        // Asked for while the device could ask, with nothing connected yet.
        service.set_lock(true).unwrap();

        // Since then the screen lock was removed, so the store cannot make a key that
        // asks, keeps the sign-in the unlocked way, and says so. The switch is not
        // offered on such a device, which makes this sentence the only place the
        // learner finds out.
        store.reports.lock().unwrap().replace(Protection::KeychainOnly);
        service.begin().unwrap();
        let view = service.finish("the-code").unwrap();
        assert!(view.connected);
        assert_eq!(view.protection, Protection::KeychainOnly);
        assert!(view.message.contains("cannot ask for your fingerprint"), "{}", view.message);
    }

    #[test]
    fn a_platform_that_cannot_ask_for_a_fingerprint_is_told_so_rather_than_offered_it() {
        let store = std::sync::Arc::new(MemoryStore {
            can_lock: false,
            ..MemoryStore::working()
        });
        let service = with_store(None, &store, Box::new(FakeHttp::default()));
        assert!(!service.view().can_lock, "the switch is not offered");

        let refusal = service.set_lock(true).unwrap_err();
        assert!(refusal.contains("cannot ask for a fingerprint"), "{refusal}");
        assert!(refusal.contains("unaffected"), "and it should reassure: {refusal}");
    }

    #[test]
    fn a_whole_connect_then_sync_then_disconnect_pass() {
        // The app's service end to end, with no keychain and no socket: connect
        // against a fake token endpoint, sync over a real directory, disconnect.
        let dir = scratch("pass");
        let db = Db::open(&dir).unwrap();
        let shared = scratch("pass-store");
        let remote = hanzi_sync::FolderStore::open(&shared).unwrap();

        let service = with_store(Some(db.clone()), &std::sync::Arc::new(MemoryStore::working()), connecting());

        // Nothing is connected to begin with.
        assert!(!service.view().connected);
        assert!(service.now().is_err(), "there is no account to sync with yet");

        // Connect: the code is exchanged and the refresh token is stored.
        service.begin().unwrap();
        let connected = service.finish("the-code").unwrap();
        assert!(connected.connected, "{}", connected.message);
        assert_eq!(connected.account_id.as_deref(), Some("dbid:AAAA"));
        assert_eq!(
            service
                .tokens
                .load(None)
                .unwrap()
                .unwrap()
                .account
                .refresh_token,
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

        // Disconnect revokes, then clears — the token and the record together, so a
        // fresh run of the app does not claim a connection that is gone.
        let gone = service.disconnect();
        assert!(!gone.connected);
        assert!(gone.message.contains("Disconnected"), "{}", gone.message);
        assert!(
            service.tokens.load(None).unwrap().is_none(),
            "the token is gone"
        );

        let after_restart = with_store(Some(db), &std::sync::Arc::new(MemoryStore::working()), Box::new(FakeHttp::default()));
        assert!(!after_restart.view().connected, "and nothing remembers it");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&shared).ok();
    }

    // ---- syncing at launch, which nobody asked for --------------------------

    #[test]
    fn a_launch_with_nothing_connected_asks_nothing_of_anybody() {
        // Most people never connect Dropbox at all, so this case has to be free of
        // both the network and the keychain — it is the one that runs for them on
        // every launch.
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(database("auto-none")), &store, &http, true);

        match service.auto() {
            AutoSync::Skipped { reason } => {
                assert!(reason.contains("Not connected"), "{reason}")
            }
            other => panic!("nothing is connected, so nothing should sync: {other:?}"),
        }
        assert_eq!(http.calls(), 0, "and nothing was sent anywhere");
    }

    #[test]
    fn a_launch_syncs_by_itself_and_says_what_it_did() {
        // The whole point: nobody pressed anything. This drives a real sync over the
        // HTTP seam — a token refresh, then a listing of the app folder.
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(database("auto")), &store, &http, true);
        connect(&service);
        assert_eq!(store.reads(), 0, "connecting does not read back what it wrote");

        match service.auto() {
            AutoSync::Synced { view } => {
                assert!(view.connected);
                assert_eq!(view.message, "Already up to date.", "nothing was written yet");
                assert!(view.last.is_some(), "and the screen has a summary to show");
            }
            other => panic!("an account is connected, so a launch should sync: {other:?}"),
        }
        assert!(http.calls() >= 2, "a token was refreshed and the folder listed");
        assert_eq!(store.reads(), 0, "and the token this run wrote is the one it used");
    }

    #[test]
    fn a_launch_with_no_network_says_so_without_attempting_anything() {
        // What the pre-check is for. `UreqHttp` allows ten seconds per connect, and
        // a launch that spends them one request at a time is worse than no launch
        // sync at all — so the probe refuses first and nothing is sent.
        let db = database("auto-offline");
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        connect(&with_http(Some(db.clone()), &store, &http, true));
        let after_connecting = http.calls();

        let offline = with_http(Some(db), &store, &http, false);
        match offline.auto() {
            AutoSync::Offline { reason } => assert!(reason.contains("No network"), "{reason}"),
            other => panic!("no network is not a sync: {other:?}"),
        }
        assert_eq!(
            http.calls(),
            after_connecting,
            "not one request was attempted, which is the whole point of asking first"
        );
    }

    #[test]
    fn a_locked_sign_in_is_not_synced_by_itself() {
        // A sync that runs on its own has nobody to satisfy a fingerprint check, so
        // with the lock on it does not run — and nothing is unlocked to find that
        // out. That is a real cost of turning the switch on, and the switch says so.
        let db = database("auto-locked");
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(db), &store, &http, true);
        connect(&service);
        service.set_lock(true).unwrap();
        let (calls, reads) = (http.calls(), store.reads());

        match service.auto() {
            AutoSync::Skipped { reason } => assert!(reason.contains("fingerprint"), "{reason}"),
            other => panic!("a locked sign-in is not read by itself: {other:?}"),
        }
        assert_eq!(http.calls(), calls, "nothing was sent");
        assert_eq!(store.reads(), reads, "and nothing was unlocked");
    }

    #[test]
    fn a_failed_launch_sync_is_reported_rather_than_hidden() {
        // The one case that has to be said out loud: an automatic sync that fails
        // silently is a device quietly falling out of step with the others.
        let db = database("auto-failed");
        let store = std::sync::Arc::new(MemoryStore::working());
        // No token response, so the refresh fails.
        let http = FakeHttp::default();
        let service = with_http(Some(db), &store, &http, true);
        // Connecting needs a token, so the account is put there by hand — which is
        // also the state a device is in after a restart.
        store
            .save(
                &Account {
                    refresh_token: "refresh-me".to_string(),
                    account_id: Some("dbid:AAAA".to_string()),
                },
                false,
                None,
            )
            .unwrap();

        match service.auto() {
            AutoSync::Failed { reason } => assert!(reason.contains("token"), "{reason}"),
            other => panic!("a sync that could not finish is a failure: {other:?}"),
        }
        assert!(store.reads() > 0, "and it got far enough to need the token");
    }

    #[test]
    fn two_syncs_at_once_is_not_reported_as_a_failure() {
        // Commands run off the main thread now, so a sync at launch and a pressed
        // Sync can genuinely overlap. The second one has nothing to add and must not
        // be dressed up as something going wrong.
        let db = database("gate");
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(db), &store, &http, true);
        connect(&service);
        let (calls, reads) = (http.calls(), store.reads());

        // Holding the gate is what a sync in flight is doing.
        let running = service.gate.lock().unwrap();

        let refused = service.now().unwrap_err();
        assert!(refused.contains("already running"), "{refused}");

        match service.auto() {
            AutoSync::Skipped { reason } => assert!(reason.contains("already running"), "{reason}"),
            other => panic!("an overlap is not a failure: {other:?}"),
        }

        let view = service.disconnect();
        assert!(view.connected, "and it did not disconnect underneath the sync");
        assert!(view.message.contains("sync is running"), "{}", view.message);

        assert_eq!(http.calls(), calls, "nothing was attempted by any of the three");
        assert_eq!(store.reads(), reads, "and the keychain was left alone");

        drop(running);
        assert!(service.now().is_ok(), "and the gate opens again");
    }

    #[test]
    fn a_service_without_a_database_says_so_instead_of_pretending() {        let service = bare(MemoryStore::working());
        let error = service.run(&hanzi_sync::FolderStore::open(scratch("nodir")).unwrap(), None).unwrap_err();
        assert!(error.contains("no study database"), "{error}");
    }

    // ---- a position that moves, published without waiting for a sync --------

    /// A database with one group, one entry in it, and a drill stopped on that
    /// entry — the state finishing something leaves behind.
    ///
    /// Returns the entry's **uuid** as well, because that is what travels: an id
    /// names a different word on every device.
    fn drilled(name: &str) -> (Db, String) {
        let db = database(name);
        let mut vocab = hanzi_core::VocabStore::open_with(Box::new(db.clone())).unwrap();
        let entry = vocab
            .add_entry("学习", "xuéxí", "to study", Some("Lesson 1"))
            .unwrap();
        vocab.save().unwrap();
        db.set_vocab_cursor("Lesson 1", Some(entry.id)).unwrap();
        let uuid = db.vocab_cursors().unwrap()[0]
            .entry_uuid
            .clone()
            .expect("the position names an entry");
        (db, uuid)
    }

    #[test]
    fn a_position_that_moves_is_published_without_a_sync() {
        // The point of the whole path: the position leaves the device when it
        // changes, rather than at the next launch or foreground.
        let (db, _) = drilled("publish-now");
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(db), &store, &http, true);
        connect(&service);
        let before = http.calls();

        service.publish_positions().unwrap();
        assert!(http.calls() > before, "the position was sent");
    }

    #[test]
    fn what_a_publish_writes_is_the_position_a_peer_resumes_from() {
        // The document itself, over a real store. "Something was sent" is a weaker
        // claim than "what was sent names the group, the device and the entry".
        let (db, uuid) = drilled("publish-content");
        let shared = scratch("publish-content-store");
        let remote = hanzi_sync::FolderStore::open(&shared).unwrap();
        let service = with_store(
            Some(db.clone()),
            &std::sync::Arc::new(MemoryStore::working()),
            connecting(),
        );

        service.publish_positions_to(&remote).unwrap();

        let shards = hanzi_sync::read_vocab_cursors(&remote).unwrap();
        assert_eq!(shards.len(), 1, "one device, one document");
        assert_eq!(shards[0].group_name, "Lesson 1");
        assert_eq!(shards[0].device_id, db.device_id());
        assert_eq!(
            shards[0].entry_uuid.as_deref(),
            Some(uuid.as_str()),
            "and it names the entry the drill reached, by uuid rather than by id"
        );
        std::fs::remove_dir_all(&shared).ok();
    }

    #[test]
    fn a_publish_with_nobody_to_publish_to_asks_nothing_of_anybody() {
        // Most people never connect Dropbox, so this is the case that has to cost
        // nothing — no socket, and no keychain either: the record answers it.
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(database("publish-none")), &store, &http, true);

        service.publish_positions().unwrap();
        assert_eq!(http.calls(), 0, "nothing is connected, so nothing is sent");
    }

    #[test]
    fn a_publish_never_asks_for_a_fingerprint() {
        // A publish starts by itself at a moment the learner did not choose — the
        // instant an entry is finished — so a locked sign-in has to stop it dead
        // rather than prompt in the middle of a drill.
        let (db, _) = drilled("publish-locked");
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(db), &store, &http, true);
        connect(&service);
        service.set_lock(true).unwrap();
        let (calls, reads) = (http.calls(), store.reads());

        service.publish_positions().unwrap();

        assert_eq!(http.calls(), calls, "nothing was sent");
        assert_eq!(store.reads(), reads, "and nothing was unlocked to find that out");
    }

    #[test]
    fn a_publish_with_no_network_is_not_attempted() {
        // The same bounded probe an automatic sync makes: a phone on a train must
        // not spend `UreqHttp`'s connect timeout once per finished entry.
        let (db, _) = drilled("publish-offline");
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        connect(&with_http(Some(db.clone()), &store, &http, true));
        let after_connecting = http.calls();
        let reads = store.reads();

        let offline = with_http(Some(db), &store, &http, false);
        offline.publish_positions().unwrap();

        assert_eq!(http.calls(), after_connecting, "not one request was attempted");
        assert_eq!(store.reads(), reads, "and the sign-in was not read either");
    }

    #[test]
    fn a_drill_does_not_exchange_a_token_per_entry() {
        // A publish happens once per finished entry, so a token exchange each time
        // would be one per character written. The token is kept for a while — and
        // *only* for a publish: a sync still fetches its own, which is why this
        // counts the token endpoint rather than the uploads.
        let (db, _) = drilled("publish-token");
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(db), &store, &http, true);
        connect(&service);
        let before = http.inner.forms.lock().unwrap().len();

        for _ in 0..5 {
            service.publish_positions().unwrap();
        }

        let after = http.inner.forms.lock().unwrap().len();
        assert_eq!(after - before, 1, "five entries, one token exchange");
    }

    #[test]
    fn a_publish_that_nothing_overtook_runs_once_and_stands_down() {
        let service = bare(MemoryStore::working());
        let publishes = std::cell::Cell::new(0usize);
        service.drain_owed(|| {
            publishes.set(publishes.get() + 1);
            Ok(())
        });
        assert_eq!(publishes.get(), 1, "one pass, then the loop ends");
    }

    #[test]
    fn a_change_that_lands_during_a_publish_is_published_too() {
        // The trap this loop exists for: a whole-document publish that clears the
        // "something changed" flag and then misses a change that arrived while it
        // was writing. The last entry of a drill is exactly such a change — it is
        // the one that decides where the next device resumes — so it must not be
        // the one that is dropped.
        let service = bare(MemoryStore::working());
        let publishes = std::cell::Cell::new(0usize);
        service.drain_owed(|| {
            let round = publishes.get() + 1;
            publishes.set(round);
            if round == 1 {
                // A second change lands while the first publish is in flight.
                service.positions_owed.store(true, Ordering::SeqCst);
            }
            Ok(())
        });
        assert_eq!(publishes.get(), 2, "the loop went round again for it");
    }

    #[test]
    fn asking_for_a_publish_gets_one_and_leaves_no_thread_behind() {
        // The command does not wait for the publish, so this is the half it relies
        // on: a thread starts, does the work, and stands down.
        let (db, _) = drilled("publish-soon");
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = std::sync::Arc::new(with_http(Some(db), &store, &http, true));
        connect(&service);
        let before = http.calls();

        service.publish_positions_soon();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while service.publishing_positions.load(Ordering::SeqCst) {
            assert!(
                std::time::Instant::now() < deadline,
                "the publish thread never stood down"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(http.calls() > before, "and it did publish");
    }

    #[test]
    fn a_token_fetched_for_one_account_is_not_used_for_another() {
        // A publish keeps its access token, so the two moments the account behind
        // it changes have to throw it away: nothing else would notice, and the
        // stale token would be the one a later publish used.
        let (db, _) = drilled("publish-account");
        let store = std::sync::Arc::new(MemoryStore::working());
        let http = connecting_http();
        let service = with_http(Some(db), &store, &http, true);
        connect(&service);
        service.publish_positions().unwrap();
        assert!(service.publish_token.lock().unwrap().is_some(), "one was kept");

        service.disconnect();
        assert!(
            service.publish_token.lock().unwrap().is_none(),
            "disconnecting forgets it"
        );
    }

    #[test]
    fn the_view_serialises_the_way_the_interface_reads_it() {
        let view = bare(MemoryStore::working()).view();
        let json = serde_json::to_value(&view).unwrap();
        let keys: BTreeMap<&str, &serde_json::Value> = json.as_object().unwrap().iter().map(|(k, v)| (k.as_str(), v)).collect();
        for expected in [
            "connected",
            "accountId",
            "canConnect",
            "canLock",
            "locked",
            "protection",
            "last",
            "message",
        ] {
            assert!(keys.contains_key(expected), "{expected} missing from {json}");
        }
        // The summary inside it too, because the screen reads those names as well.
        let summary = serde_json::to_value(SyncSummaryView {
            published: 1,
            pulled: 2,
            recomputed: 3,
            left_alone: 4,
            vocab_changed: 5,
            cursor_moved: true,
        })
        .unwrap();
        let keys: BTreeMap<&str, &serde_json::Value> = summary.as_object().unwrap().iter().map(|(k, v)| (k.as_str(), v)).collect();
        for expected in [
            "published",
            "pulled",
            "recomputed",
            "leftAlone",
            "vocabChanged",
            "cursorMoved",
        ] {
            assert!(keys.contains_key(expected), "{expected} missing from {summary}");
        }
    }
}
