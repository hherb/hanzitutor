//! Where shards are kept, and the little sync asks of a transport.
//!
//! Everything above this module is pure: a merge of two lists and a fold of the
//! result. That is deliberate, because the merge is the part that has to be
//! *right* — a mistake there silently corrupts a learner's schedule — while a
//! transport is only the part that has to work. Keeping the two apart is what
//! lets the merge be tested exhaustively against a temporary directory, with no
//! account, no network and no credentials.

use std::fmt;
use std::path::{Component, Path, PathBuf};

/// What can go wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    /// A shard name that this format could not have produced, or one that would
    /// point outside the store.
    Malformed(String),
    /// Two shards disagree about what one attempt was.
    ///
    /// `(device_id, seq)` names an attempt that was written once and never
    /// rewritten, so the same pair carrying different contents means a shard was
    /// replaced — a merge must not guess which copy is real.
    Rewritten { device_id: String, seq: i64 },
    /// The store could not be read or written.
    Io(String),
    /// Dropbox refused the access token, so it has expired or been revoked.
    ///
    /// Distinguished from [`SyncError::Io`] because it is the one failure with an
    /// obvious next move: refresh the token and try again, rather than report a
    /// network fault to somebody whose network is fine.
    Unauthorized,
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::Malformed(why) => write!(f, "{why}"),
            SyncError::Rewritten { device_id, seq } => write!(
                f,
                "two shards disagree about which attempt {seq} of {device_id} was; \
                 a shard must never be rewritten"
            ),
            SyncError::Io(why) => write!(f, "{why}"),
            SyncError::Unauthorized => write!(
                f,
                "Dropbox refused the access token, so it has expired or been revoked"
            ),
        }
    }
}

impl std::error::Error for SyncError {}

/// A shard the store holds, described without fetching it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEntry {
    /// The shard's name, which is also its path within the store.
    pub name: String,
    /// A value that changes whenever the shard's contents do.
    ///
    /// A local file's size and modification time; Dropbox's `rev` or
    /// `content_hash`. Sync compares it with what it recorded last time and skips
    /// the download when it matches, which is the difference between a sync that
    /// re-reads everything and one that reads what changed.
    pub revision: String,
}

/// Somewhere that can hold named blobs.
///
/// Three methods, and no notion of a conflict, a lock or a transaction. That is
/// the point: every writer owns its own directory (see the crate docs), so the
/// weakest useful storage is enough.
pub trait RemoteStore {
    /// Every shard currently held, in no particular order.
    fn list(&self) -> Result<Vec<RemoteEntry>, SyncError>;
    /// One shard's bytes.
    fn get(&self, name: &str) -> Result<Vec<u8>, SyncError>;
    /// Write a shard. Writing a name that already exists is not an error here:
    /// the format makes it a no-op by construction, and a transport that had to
    /// distinguish the two would be a transport doing sync's job.
    fn put(&self, name: &str, bytes: &[u8]) -> Result<(), SyncError>;
}

/// A store that is a directory.
///
/// This is the whole of the "bring your own storage" story for a desktop: point
/// it at a folder that a sync client already keeps in step — Dropbox, Drive, a
/// Syncthing folder, a USB stick — and it works with no account and no API. It is
/// also what the merge is tested against.
pub struct FolderStore {
    root: PathBuf,
}

impl FolderStore {
    /// Open `root` as a store, creating it if it is not there.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, SyncError> {
        let root = root.into();
        std::fs::create_dir_all(&root)
            .map_err(|e| SyncError::Io(format!("{}: {e}", root.display())))?;
        Ok(Self { root })
    }

    /// The directory shards live in.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The file a shard name refers to, refusing anything that would escape the
    /// root.
    ///
    /// A shard name arrives from a remote store, so it is not this program's own
    /// text and must be treated as hostile. `../../.ssh/authorized_keys` is a
    /// perfectly good filename to a filesystem and a catastrophic one to obey.
    fn path_of(&self, name: &str) -> Result<PathBuf, SyncError> {
        if name.is_empty() {
            return Err(SyncError::Malformed("a shard name cannot be empty".into()));
        }
        // Checked by component rather than by string, so that a name is judged by
        // what it means to a filesystem rather than by how it looks. A backslash
        // is a separator on Windows and an ordinary character here, so it is
        // refused outright: a name that means two things on two platforms is not a
        // name this format should accept.
        for component in Path::new(name).components() {
            match component {
                Component::Normal(part) => {
                    if part.to_string_lossy().contains('\\') {
                        return Err(SyncError::Malformed(format!(
                            "the shard name {name:?} contains a backslash, which is a path \
                             separator on some platforms and not others"
                        )));
                    }
                }
                other => {
                    return Err(SyncError::Malformed(format!(
                        "the shard name {name:?} is not a plain relative path ({other:?})"
                    )))
                }
            }
        }
        Ok(self.root.join(name))
    }

    /// Every file under the root, as `(name, revision)`, recursively.
    fn walk(&self, dir: &Path, prefix: &str, out: &mut Vec<RemoteEntry>) -> Result<(), SyncError> {
        let listing = std::fs::read_dir(dir)
            .map_err(|e| SyncError::Io(format!("{}: {e}", dir.display())))?;
        for entry in listing {
            let entry = entry.map_err(|e| SyncError::Io(format!("{}: {e}", dir.display())))?;
            let name = match prefix.is_empty() {
                true => entry.file_name().to_string_lossy().into_owned(),
                false => format!("{prefix}/{}", entry.file_name().to_string_lossy()),
            };
            let metadata = entry
                .metadata()
                .map_err(|e| SyncError::Io(format!("{name}: {e}")))?;
            if metadata.is_dir() {
                self.walk(&entry.path(), &name, out)?;
                continue;
            }
            // Size and modification time: enough to notice a shard changing, and
            // all a plain directory can offer. A store with a real content hash
            // reports that instead.
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            out.push(RemoteEntry {
                name,
                revision: format!("{}:{modified}", metadata.len()),
            });
        }
        Ok(())
    }
}

impl RemoteStore for FolderStore {
    fn list(&self) -> Result<Vec<RemoteEntry>, SyncError> {
        let mut out = Vec::new();
        self.walk(&self.root, "", &mut out)?;
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn get(&self, name: &str) -> Result<Vec<u8>, SyncError> {
        let path = self.path_of(name)?;
        std::fs::read(&path).map_err(|e| SyncError::Io(format!("{}: {e}", path.display())))
    }

    fn put(&self, name: &str, bytes: &[u8]) -> Result<(), SyncError> {
        let path = self.path_of(name)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SyncError::Io(format!("{}: {e}", parent.display())))?;
        }
        // Written to a temporary name and renamed, so a reader — which may be this
        // device on its next run, or a sync client copying the folder — never sees
        // half a shard. A truncated shard would parse as a shorter log rather than
        // as an error, which is the one failure mode worth this much care.
        let temporary = path.with_extension("jsonl.part");
        std::fs::write(&temporary, bytes)
            .map_err(|e| SyncError::Io(format!("{}: {e}", temporary.display())))?;
        std::fs::rename(&temporary, &path)
            .map_err(|e| SyncError::Io(format!("{}: {e}", path.display())))?;
        Ok(())
    }
}
