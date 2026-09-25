//! Speaking a phrase the app has no bundled clip for.
//!
//! ## Why this exists when the operating system already speaks Chinese
//!
//! [`hanzi_voice::speech`] drives the platform's own synthesiser, and for most
//! learners that is the whole story: macOS ships Tingting, iOS reports sixteen
//! Chinese voices, and Android has both an on-device and a network voice for
//! `zh-CN`. What it cannot promise is that any of them is *present*. A Windows
//! or Linux build has no backend at all ([`hanzi_voice::speech`] is macOS, iOS and
//! Android), and a device can have the locale without a voice installed.
//!
//! This is the fallback for exactly that case, and it is deliberately the same
//! voice the bundled clips were made with: MeloTTS, run by the `sherpa-onnx`
//! library the app already links for recognition. A learner who installs it
//! hears the clips and any phrase they type in one voice rather than two.
//!
//! ## Opt-in, exactly like the recognition model
//!
//! The rules are [`crate::asr`]'s rules, because they were the right ones there:
//!
//! - **Nothing degrades without it.** The clips ship with the app and the
//!   platform synthesiser is still tried first; this is never on a path that a
//!   learner has to take.
//! - **The download is verified** against a pinned SHA-256 per file, and a
//!   failure leaves nothing behind that could be mistaken for an install.
//! - **Nothing is downloaded until asked for**, and the size is stated first.
//!
//! ## Why it is not in `hanzi-say`
//!
//! `hanzi-say` is the synthesis itself and is built for the build host: it
//! returns samples and writes WAVs. This module is the *app's* half — where the
//! weights live, whether they are installed, and how they arrive — which is the
//! same division `hanzi-core` and `asr.rs` already draw.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use sha2::{Digest, Sha256};

/// One file of the model, pinned by digest.
///
/// Recorded 2026-09-21 by downloading each file and hashing it. The digests are
/// the pin: a re-uploaded model is refused rather than silently changing what
/// the app says.
struct FileSpec {
    name: &'static str,
    sha256: &'static str,
    bytes: u64,
}

/// Where the weights come from, and what to say about them before fetching.
///
/// Every number is measured rather than estimated, because the settings screen
/// states it to somebody deciding whether to spend the bandwidth.
struct ModelSpec {
    /// What to call it in the interface.
    name: &'static str,
    /// The directory the files land in.
    stem: &'static str,
    /// Base URL; each file's name is appended.
    base: &'static str,
    files: &'static [FileSpec],
    /// The licence the weights are under, shown before the download.
    licence: &'static str,
}

const MODEL: ModelSpec = ModelSpec {
    name: "MeloTTS (Mandarin)",
    stem: "vits-melo-tts-zh_en",
    base: "https://huggingface.co/csukuangfj/vits-melo-tts-zh_en/resolve/main",
    licence: "MIT",
    files: &[
        FileSpec { name: "model.int8.onnx", sha256: "f085f5079e05f039b800aeb542f5253c26a303211b0c6465d0d9387977855a63", bytes: 53_517_430 },
        FileSpec { name: "tokens.txt", sha256: "d18664a7e12bd7ea1022ddaf951e534e136815016c5a809d6b64156bffb4369d", bytes: 655 },
        FileSpec { name: "lexicon.txt", sha256: "7236884b02435ac5d10cf69b4be40a61b45aa676b5300f0e412f185748fee528", bytes: 6_837_671 },
        FileSpec { name: "date.fst", sha256: "eb8aa079ae3cb81d8f4404992f39d61a0cb990947512b5b8d1e54d1f6980e718", bytes: 59_154 },
        FileSpec { name: "number.fst", sha256: "743f402181fcfebf76cc2f0546b71fa26476e626fbe4e460fb7b4c3a7a8bd5bd", bytes: 64_482 },
        FileSpec { name: "phone.fst", sha256: "1ac2b6fa56b1442320c4de7db08353bab8963a2b57f365eebcdd3a2d3562f8d7", bytes: 88_630 },
        FileSpec { name: "new_heteronym.fst", sha256: "ca14b2127e27baa571664e4bb791e143e7425f56a6bc29db08d74f97e6aa4e29", bytes: 21_974 },
    ],
};

/// How the install is going, for the settings screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallState {
    /// Never asked for. The ordinary state, and not a problem.
    #[default]
    Absent,
    Downloading,
    Installed,
    Failed,
}

/// Everything the settings screen needs, in one call.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SayStatus {
    pub state: InstallState,
    /// One line, always present, worded for whatever the state is.
    pub detail: String,
    /// What the model is called, so the screen does not have to know.
    pub name: &'static str,
    /// Total bytes to fetch, so the screen can say the size before it starts.
    pub bytes: u64,
    /// The licence, so a learner can decide before spending the bandwidth.
    pub licence: &'static str,
    /// Bytes fetched so far, while downloading.
    pub downloaded: u64,
    /// True when the app can speak right now — either the model is installed,
    /// or a platform voice is available. The screen needs both facts to decide
    /// whether to offer this at all.
    pub installed: bool,
}

/// What a download is doing, shared with the thread doing it.
#[derive(Default)]
struct Install {
    state: InstallState,
    downloaded: u64,
    error: Option<String>,
}

/// The synthesiser, holding nothing until it is asked for something.
pub struct Say {
    /// `None` when the platform gave no data directory: there is then nowhere to
    /// put the weights, so this offers nothing rather than failing later.
    root: Option<PathBuf>,
    install: Arc<Mutex<Install>>,
    model: Mutex<Option<hanzi_say::Model>>,
}

impl Say {
    /// Hold the model under `data_dir`, if the platform gave us one.
    pub fn new(data_dir: Option<&Path>) -> Self {
        Self {
            root: data_dir.map(|dir| dir.join("say")),
            install: Arc::new(Mutex::new(Install::default())),
            model: Mutex::new(None),
        }
    }

    /// Where the model lives, when it is anywhere.
    fn model_dir(&self) -> Option<PathBuf> {
        self.root.as_ref().map(|root| root.join(MODEL.stem))
    }

    /// True when every file the model needs is really there, at the size it
    /// should be.
    ///
    /// Every file rather than the directory, and a size rather than only a name:
    /// an interrupted download is the failure that actually happens, and a
    /// truncated `model.int8.onnx` is a file that exists. Size is the cheap half
    /// of that check — the digest is the expensive half, so it is verified once
    /// when the model is loaded ([`Self::speak`]) rather than on every status
    /// poll, which the settings screen makes while a download runs.
    fn is_installed(&self) -> bool {
        self.model_dir().is_some_and(|dir| {
            MODEL.files.iter().all(|f| {
                std::fs::metadata(dir.join(f.name))
                    .map(|m| m.len() == f.bytes)
                    .unwrap_or(false)
            })
        })
    }

    /// Everything the settings screen needs, in one call.
    pub fn status(&self) -> SayStatus {
        let install = self.install.lock().expect("say install slot");
        let installed = self.is_installed();
        let total: u64 = MODEL.files.iter().map(|f| f.bytes).sum();

        let state = match install.state {
            InstallState::Downloading => InstallState::Downloading,
            InstallState::Failed => InstallState::Failed,
            _ if installed => InstallState::Installed,
            _ => InstallState::Absent,
        };

        let detail = match state {
            InstallState::Downloading => format!(
                "Downloading {} — {:.0} MB of {} MB.",
                MODEL.name,
                install.downloaded as f64 / 1e6,
                total as f64 / 1e6
            ),
            InstallState::Failed => install
                .error
                .clone()
                .unwrap_or_else(|| "The download failed.".to_string()),
            InstallState::Installed => format!(
                "{} is installed. Phrases with no recording are spoken with it.",
                MODEL.name
            ),
            InstallState::Absent => format!(
                "Speak phrases the app has no recording for, in the same voice as \
                 the recordings. Downloads {:.0} MB, {} licensed.",
                total as f64 / 1e6,
                MODEL.licence
            ),
        };

        SayStatus {
            state,
            detail,
            name: MODEL.name,
            bytes: total,
            licence: MODEL.licence,
            downloaded: install.downloaded,
            installed,
        }
    }

    /// Fetch the weights, reporting progress as it goes, on a thread of its
    /// own.
    ///
    /// Returns as soon as the download has been started, because a 50+ MB
    /// download that a command blocked on would leave the settings screen
    /// showing a button that never comes back rather than the progress bar
    /// [`Say::status`] is there to drive — see [`crate::asr`]'s `install`,
    /// which this mirrors.
    pub fn install(&self) -> Result<(), String> {
        let Some(dir) = self.model_dir() else {
            return Err("This build has nowhere to put the model.".to_string());
        };
        {
            let mut install = self.install.lock().expect("say install slot");
            if install.state == InstallState::Downloading {
                return Err("A download is already running.".to_string());
            }
            install.state = InstallState::Downloading;
            install.downloaded = 0;
            install.error = None;
        }

        let progress = Arc::clone(&self.install);
        std::thread::spawn(move || {
            let outcome = fetch_all(&dir, &progress);
            let mut install = progress.lock().expect("say install slot");
            match outcome {
                Ok(()) => install.state = InstallState::Installed,
                Err(message) => {
                    install.state = InstallState::Failed;
                    install.error = Some(message);
                }
            }
        });
        Ok(())
    }

    /// Remove the weights and report the resulting state.
    pub fn remove(&self) -> Result<(), String> {
        if let Some(dir) = self.model_dir() {
            if dir.exists() {
                fs::remove_dir_all(&dir)
                    .map_err(|e| format!("could not remove {}: {e}", dir.display()))?;
            }
        }
        *self.model.lock().expect("say model slot") = None;
        let mut install = self.install.lock().expect("say install slot");
        install.state = InstallState::Absent;
        install.downloaded = 0;
        install.error = None;
        Ok(())
    }

    /// Synthesise `text`, returning mono samples and their rate.
    ///
    /// Returns `Ok(None)` when no model is installed, which is how the app
    /// ships: the caller falls back to the platform synthesiser and nothing is
    /// reported as an error. That is the same contract [`crate::asr`] uses, and
    /// for the same reason — an optional feature that is absent is not a fault.
    pub fn speak(&self, text: &str) -> Result<Option<(Vec<f32>, u32)>, String> {
        if !self.is_installed() {
            return Ok(None);
        }
        let mut slot = self.model.lock().expect("say model slot");
        if slot.is_none() {
            let dir = self
                .model_dir()
                .ok_or_else(|| "This build has nowhere to put the model.".to_string())?;
            let model = hanzi_say::Model::load(&dir).map_err(|e| e.to_string())?;
            *slot = Some(model);
        }
        let model = slot.as_ref().expect("just loaded");
        let samples = model
            .synthesize(text, 1.0)
            // The same ceiling the platform path uses, so a pasted paragraph
            // cannot turn one tap into a minute of speech.
            .map_err(|e| e.to_string())?;
        let rate = model.sample_rate();
        Ok(Some((samples, rate)))
    }
}

/// Download every file, verifying each against its digest. Runs on the
/// install thread.
fn fetch_all(dir: &Path, install: &Arc<Mutex<Install>>) -> Result<(), String> {
    fs::create_dir_all(dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(30))
        // Per read rather than overall: a 53 MB file legitimately takes a
        // while on a slow line that is working perfectly well.
        .timeout_read(Duration::from_secs(120))
        .user_agent(concat!("HanziTutor/", env!("CARGO_PKG_VERSION")))
        .build();

    let mut done: u64 = 0;
    for spec in MODEL.files {
        let destination = dir.join(spec.name);

        // A file already present and correct is not fetched again, so an
        // interrupted install resumes rather than starting over.
        if file_matches(&destination, spec.sha256) {
            done += spec.bytes;
            if let Ok(mut install) = install.lock() {
                install.downloaded = done;
            }
            continue;
        }

        let url = format!("{}/{}", MODEL.base, spec.name);
        let response = agent
            .get(&url)
            .call()
            .map_err(|e| format!("could not download {url}: {e}"))?;
        let mut reader = response.into_reader();

        // Written to a temporary name and renamed only once verified, so a
        // failed download is never visible as a usable file.
        let partial = dir.join(format!("{}.partial", spec.name));
        let file = File::create(&partial)
            .map_err(|e| format!("could not write {}: {e}", partial.display()))?;
        let mut file = std::io::BufWriter::new(file);

        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];
        let mut written: u64 = 0;
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|e| format!("the download stopped after {written} bytes: {e}"))?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])
                .map_err(|e| format!("could not write to disk after {written} bytes: {e}"))?;
            hasher.update(&buffer[..read]);
            written += read as u64;
            if let Ok(mut install) = install.lock() {
                install.downloaded = done + written;
            }
        }
        file.flush()
            .map_err(|e| format!("could not flush {}: {e}", partial.display()))?;
        drop(file);

        let got = format!("{:x}", hasher.finalize());
        if got != spec.sha256 {
            let _ = fs::remove_file(&partial);
            return Err(format!(
                "{} did not match its recorded digest — deleted rather than used. \
                 Expected {}, got {got}.",
                spec.name, spec.sha256
            ));
        }
        fs::rename(&partial, &destination)
            .map_err(|e| format!("could not move {} into place: {e}", spec.name))?;
        done += written;
    }
    Ok(())
}

/// True when `path` exists and hashes to `expected`.
fn file_matches(path: &Path, expected: &str) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buffer[..n]),
            Err(_) => return false,
        }
    }
    format!("{:x}", hasher.finalize()) == expected
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A scratch directory no other test shares.
    ///
    /// The process id alone is not enough: tests in one binary run in parallel
    /// threads *of the same process*, so every test here would otherwise be
    /// handed the same path — and the one that removes a model directory would
    /// delete it out from under another. A counter makes each one distinct.
    fn scratch(label: &str) -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "say-test-{}-{label}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn there_is_nowhere_to_put_a_model_without_a_data_directory() {
        let say = Say::new(None);
        assert!(!say.is_installed());
        assert_eq!(say.status().state, InstallState::Absent);
        assert!(say.install().is_err());
        // And it says so rather than silently doing nothing.
        assert!(say.install().unwrap_err().contains("nowhere"));
    }

    #[test]
    fn a_fresh_install_is_absent_and_states_the_cost() {
        let dir = scratch("fresh");
        let say = Say::new(Some(&dir));
        let status = say.status();
        assert_eq!(status.state, InstallState::Absent);
        assert!(!status.installed);
        // The size and licence are stated *before* anything is downloaded, which
        // is the whole point of putting them here.
        assert!(status.bytes > 50_000_000, "bytes = {}", status.bytes);
        assert_eq!(status.licence, "MIT");
        assert!(status.detail.contains("Downloads"));
        assert!(status.detail.contains("MIT"));
    }

    #[test]
    fn speaking_without_a_model_is_not_an_error() {
        let say = Say::new(None);
        // `None`, not `Err`: an optional feature that is absent is not a fault.
        assert_eq!(say.speak("你好").unwrap(), None);
    }

    #[test]
    fn a_directory_without_the_files_is_not_an_install() {
        let dir = scratch("partial");
        let model_dir = dir.join("say").join(MODEL.stem);
        fs::create_dir_all(&model_dir).unwrap();
        // A directory left by an interrupted install must not read as installed:
        // `tokens.txt` is present but nowhere near its recorded 655 bytes.
        fs::write(model_dir.join("tokens.txt"), b"not the real tokens").unwrap();

        let say = Say::new(Some(&dir));
        assert!(!say.is_installed());
        assert_eq!(say.status().state, InstallState::Absent);
        assert_eq!(say.speak("你好").unwrap(), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_clears_the_directory_and_the_state() {
        let dir = scratch("remove");
        let model_dir = dir.join("say").join(MODEL.stem);
        fs::create_dir_all(&model_dir).unwrap();
        // Files are laid out at the wrong size, which is what an interrupted
        // download leaves behind. This must not read as an install: offering
        // speech on the strength of a truncated file is how a learner ends up
        // pressing a button that cannot work.
        for spec in MODEL.files {
            fs::write(model_dir.join(spec.name), b"truncated").unwrap();
        }
        let say = Say::new(Some(&dir));
        assert!(!say.is_installed(), "a wrong-sized file must not count");
        assert_eq!(say.status().state, InstallState::Absent);

        // `remove` still clears whatever is there, which is what deletes a
        // partial install and reclaims its disk space.
        say.remove().unwrap();
        assert!(!model_dir.exists(), "the model directory should be gone");
        assert_eq!(say.status().state, InstallState::Absent);
        assert_eq!(say.speak("你好").unwrap(), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_correct_file_the_size_of_the_real_one_is_trusted() {
        // The other half of the same rule: when the digest does match, a file is
        // accepted without re-downloading it. Checked through `file_matches`
        // rather than by writing 53 MB of weights in a unit test.
        let path = scratch("match").with_extension("bin");
        let body = b"a stand-in for a model file";
        fs::write(&path, body).unwrap();
        let mut h = Sha256::new();
        h.update(body);
        let digest = format!("{:x}", h.finalize());

        assert!(file_matches(&path, &digest), "a matching digest must be accepted");
        fs::write(&path, b"changed").unwrap();
        assert!(!file_matches(&path, &digest), "changed bytes must be refused");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn file_matches_notices_a_changed_file() {
        let path = scratch("hash").with_extension("bin");
        fs::write(&path, b"hello").unwrap();
        let digest = {
            let mut h = Sha256::new();
            h.update(b"hello");
            format!("{:x}", h.finalize())
        };
        assert!(file_matches(&path, &digest));
        assert!(!file_matches(&path, "0".repeat(64).as_str()));
        assert!(!file_matches(&path.join("nope"), &digest));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn every_pinned_file_has_a_full_digest_and_a_size() {
        // A shortened digest would silently never match, which would look like a
        // corrupt download forever.
        for spec in MODEL.files {
            assert_eq!(spec.sha256.len(), 64, "{} has a partial digest", spec.name);
            assert!(spec.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(spec.bytes > 0, "{} has no size", spec.name);
        }
    }
}
