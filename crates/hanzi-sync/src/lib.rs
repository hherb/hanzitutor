//! Cross-device sync: the shard format, the merge, and the log fold.
//!
//! The design is ROADMAP M13. Its whole shape follows from two properties that
//! were already in the code before sync existed:
//!
//! - The attempt log is **append-only**. That is a grow-only set, the one shape
//!   that merges across devices with nothing to resolve.
//! - `Sm2::review` is **pure**, so a card is a *fold over the log* rather than a
//!   value to be merged. Two devices that practised the same character while apart
//!   each append their attempts; both then fold the union and agree on one
//!   schedule, with no winner to pick and no clock to trust.
//!
//! So the database itself is never synced. It stays the local materialised view,
//! and what travels is a set of small, immutable, append-only **shards** — one
//! directory per device. Two consequences are worth stating plainly, because they
//! are the reason for that layout rather than a side effect of it:
//!
//! - **A closed shard is never rewritten.** A filename is derived from the
//!   sequence numbers inside it, so a device cannot overwrite a peer's work even
//!   by accident, and a sync that runs twice writes the same bytes twice.
//! - **There is no file-level conflict to have.** Every writer owns its own
//!   directory, so a transport that only understands "put this file" — a synced
//!   folder, a cloud with no compare-and-swap — is still safe.
//!
//! ## What this crate is not
//!
//! It does not talk to a network, and it does not know what Dropbox is. It reads
//! and writes shards through [`RemoteStore`], a three-method trait, so the merge
//! can be tested against a temporary directory and a transport can be added
//! without touching it.

mod document;
mod dropbox;
mod http;
mod local;
mod oauth;
mod reach;
mod shard;
mod store;

pub use document::{
    baseline_shard_name, cursor_shard_name, merge_cursor, merge_vocab, merge_vocab_cursors,
    read_baselines, read_cursor, read_vocab, read_vocab_cursors, vocab_cursor_shard_name,
    vocab_shard_name, write_baseline, write_cursor, write_vocab, write_vocab_cursors,
    Baseline, Baselines,
};
pub use dropbox::DropboxStore;
pub use http::{Http, UreqHttp};
pub use local::{full_log, own_log, publish, pull, recompute, sync, Summary};
pub use oauth::{authorize_url, exchange_code, refresh, revoke, Pkce, Tokens, SCOPES};
pub use reach::{Reach, TcpReach};
pub use shard::{
    attempts_shard_name, device_of_shard, fold_cards, merge_attempts, parse_attempts_shard,
    read_attempts, write_attempts, MergedAttempt, CHUNK,
};
pub use store::{FolderStore, RemoteEntry, RemoteStore, SyncError};
