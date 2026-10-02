//! The app's own study file.
//!
//! Kana Tutor keeps exactly one thing about its learner: which confusion pairs
//! they get wrong. It lives here — in a file of this app's own, under this app's
//! data directory — and deliberately **not** in a shared store. The Japanese and
//! Chinese apps are separate products and their learners' data is separate with
//! them: what is shared between the apps is *code*, never user data. See
//! `HANDOVER_NIHONGO.md` invariant 15.
//!
//! The file is plain JSON, written whole and atomically, so that a person can
//! read it, copy it as a backup, or delete it to start over.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nihongo_core::{ConfusionLog, PairTally};

/// The file's name inside the app's data directory.
pub const FILE_NAME: &str = "confusions.json";

/// What the app knows about this learner's confusion pairs, and where it is kept.
pub struct ConfusionStore {
    /// `None` means nowhere to write — the tallies still work, for this session
    /// only. That is the state the unit tests run in, and what the app falls back
    /// to when the platform will not name a data directory, because an app that
    /// cannot remember is still better than an app that will not start.
    path: Option<PathBuf>,
    log: ConfusionLog,
}

impl ConfusionStore {
    /// A store that keeps its record in memory and writes nothing.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            log: ConfusionLog::new(),
        }
    }

    /// Load `confusions.json` from `dir`, or start empty.
    ///
    /// A missing file is not an error: that is the first run, and the first
    /// answer creates it.
    pub fn at_dir(dir: impl AsRef<Path>) -> Self {
        let path = dir.as_ref().join(FILE_NAME);
        Self {
            log: read(&path),
            path: Some(path),
        }
    }

    /// The record itself.
    pub fn log(&self) -> &ConfusionLog {
        &self.log
    }

    /// Whether anything is written to disk at all.
    pub fn is_persistent(&self) -> bool {
        self.path.is_some()
    }

    /// Record one answer against a pair, write the file, and say what the record
    /// now is.
    ///
    /// The record is kept **even if the write fails** — the answer happened — but
    /// the failure is returned rather than swallowed, because a learner whose
    /// progress is not being saved should be told rather than discover it later.
    pub fn record(&mut self, key: &str, correct: bool) -> io::Result<PairTally> {
        let tally = self.log.record(key, correct);
        self.save()?;
        Ok(tally)
    }

    /// Write the file, if there is one to write.
    pub fn save(&self) -> io::Result<()> {
        match &self.path {
            Some(path) => write(path, &self.log),
            None => Ok(()),
        }
    }
}

/// Read the log, treating anything unreadable as "nothing recorded yet".
///
/// A file that will not parse is **moved aside** as `confusions.json.corrupt`
/// rather than overwritten. It is the learner's history; this app should not be
/// the thing that deletes it, and leaving it where it is would mean failing to
/// write on every subsequent answer. The next answer writes a fresh file.
fn read(path: &Path) -> ConfusionLog {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<ConfusionLog>(&text) {
            Ok(log) => log,
            Err(err) => {
                eprintln!(
                    "{}: {err}; moving it aside and starting with no record",
                    path.display()
                );
                let _ = fs::rename(path, corrupt_path(path));
                ConfusionLog::new()
            }
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => ConfusionLog::new(),
        Err(err) => {
            eprintln!("{}: {err}; starting with no record", path.display());
            ConfusionLog::new()
        }
    }
}

/// Where a file that would not parse is kept.
fn corrupt_path(path: &Path) -> PathBuf {
    path.with_extension("json.corrupt")
}

/// Write the log whole, via a neighbouring temporary and a rename.
///
/// A rename within a directory is atomic, so a reader either sees the previous
/// file or the new one and never a half-written file — which matters because the
/// file is rewritten after *every* answer, and the machine can lose power between
/// two of them.
fn write(path: &Path, log: &ConfusionLog) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(log).map_err(io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text.as_bytes())?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nihongo_core::{find_pair, pair_key};

    /// A directory of our own under the system temporary directory, removed when
    /// the test ends.
    ///
    /// Named with the process id and a counter so that two tests — or two runs —
    /// never share one, which is the failure mode that makes a test that passes
    /// alone fail in a suite.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let mut path = std::env::temp_dir();
            path.push(format!(
                "nihongo-tutor-store-{}-{tag}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("a temporary directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn shi_tsu() -> String {
        pair_key(find_pair("シ|ツ").expect("the pair exists"))
    }

    #[test]
    fn a_first_run_starts_empty_and_the_first_answer_creates_the_file() {
        let dir = TempDir::new("first-run");
        let mut store = ConfusionStore::at_dir(dir.path());
        assert!(store.is_persistent());
        assert!(store.log().is_empty());
        assert!(!dir.path().join(FILE_NAME).exists(), "nothing written yet");

        let tally = store.record(&shi_tsu(), false).expect("saves");
        assert_eq!(tally.asked, 1);
        assert_eq!(tally.wrong, 1);
        assert!(dir.path().join(FILE_NAME).exists(), "the answer was written");
    }

    #[test]
    fn the_record_survives_a_restart() {
        let dir = TempDir::new("restart");
        {
            let mut store = ConfusionStore::at_dir(dir.path());
            store.record(&shi_tsu(), false).expect("saves");
            store.record(&shi_tsu(), true).expect("saves");
        }
        let reopened = ConfusionStore::at_dir(dir.path());
        let tally = reopened.log().tally(&shi_tsu());
        assert_eq!(tally.asked, 2);
        assert_eq!(tally.wrong, 1);
        assert_eq!(tally.correct, 1);
        assert_eq!(
            reopened.log().weight(&shi_tsu()),
            2.0,
            "1 + 2 for the miss − 1 for the hit"
        );
    }

    #[test]
    fn the_file_is_readable_json_of_the_shape_the_docs_promise() {
        let dir = TempDir::new("shape");
        let mut store = ConfusionStore::at_dir(dir.path());
        store.record(&shi_tsu(), false).expect("saves");

        let text = fs::read_to_string(dir.path().join(FILE_NAME)).expect("the file");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        // The whole file is the log, and a tally is three camelCase counts, so a
        // person opening it sees `{"pairs":{"シ|ツ":{"asked":1,...}}}`.
        let tally = &value["pairs"]["シ|ツ"];
        assert_eq!(tally["asked"], 1);
        assert_eq!(tally["correct"], 0);
        assert_eq!(tally["wrong"], 1);
    }

    #[test]
    fn an_empty_object_is_a_valid_file_and_not_a_corrupt_one() {
        // A file written by an older build, or emptied by hand, must read as
        // "nothing recorded" rather than be moved aside as corrupt.
        let dir = TempDir::new("empty-object");
        fs::write(dir.path().join(FILE_NAME), "{}").expect("writes");
        let store = ConfusionStore::at_dir(dir.path());
        assert!(store.log().is_empty());
        assert!(!dir.path().join("confusions.json.corrupt").exists());
    }

    #[test]
    fn a_file_that_will_not_parse_is_kept_aside_and_not_overwritten() {
        let dir = TempDir::new("corrupt");
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, "{ this is not json").expect("writes");

        let mut store = ConfusionStore::at_dir(dir.path());
        assert!(store.log().is_empty(), "a corrupt file is not a crash");
        let aside = dir.path().join("confusions.json.corrupt");
        assert!(aside.exists(), "the unreadable file was kept");
        assert_eq!(
            fs::read_to_string(&aside).expect("still there"),
            "{ this is not json",
            "and kept as it was, not as an empty file"
        );

        // And the app carries on: the next answer writes a valid file.
        store.record(&shi_tsu(), false).expect("saves");
        let reread = ConfusionStore::at_dir(dir.path());
        assert_eq!(reread.log().tally(&shi_tsu()).wrong, 1);
    }

    #[test]
    fn an_in_memory_store_records_without_writing_anything() {
        let mut store = ConfusionStore::in_memory();
        assert!(!store.is_persistent());
        store.record(&shi_tsu(), false).expect("nothing to write");
        assert_eq!(store.log().tally(&shi_tsu()).wrong, 1);
    }

    #[test]
    fn a_write_failure_is_reported_rather_than_hidden() {
        // A directory where the file should be is the cheapest way to make the
        // write fail on every platform, and it must come back as an error rather
        // than a panic — or worse, as nothing at all.
        let dir = TempDir::new("unwritable");
        fs::create_dir_all(dir.path().join(FILE_NAME)).expect("a directory in the file's place");
        let mut store = ConfusionStore::at_dir(dir.path());
        assert!(store.record(&shi_tsu(), false).is_err(), "the failure surfaces");
        assert_eq!(
            store.log().tally(&shi_tsu()).wrong,
            1,
            "and the answer is still counted in memory"
        );
    }
}
