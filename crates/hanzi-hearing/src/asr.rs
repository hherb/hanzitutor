//! Speech recognition: *what* was said, where `tone.rs` judges *how*.
//!
//! ## Why there is a model at all
//!
//! ROADMAP.md M11 scores tone from the pitch contour and needs no model — tone is
//! an F0 shape, and a contour is not something a recogniser's text output
//! contains. What M11 cannot do is say whether the learner produced the right
//! *syllable*: 四 (`sì`) and 是 (`shì`) can score identically on tone while being
//! different words. There is no non-neural substitute for that, because you
//! cannot pre-render a learner's voice. So M12 adds the one thing in this app
//! that needs model weights, and the research
//! (`docs/research/ASR_TTS_CLAUDE_RESEARCH.md`) is why it is this model:
//! `whisper-tiny` is about 67% CER on Mandarin and `whisper-base` about 51%,
//! against SenseVoice's ~8%.
//!
//! ## The model is fetched by the learner, or not at all
//!
//! 163 MB cannot be bundled, so it is downloaded — which is why this module is
//! also the only thing in the application that touches the network. That was a
//! deliberate product decision, taken in ROADMAP.md M12 and bounded there, and
//! the bounds are implemented here rather than merely intended:
//!
//! - **Nothing degrades when the model is absent.** [`Asr::recognize`] answers
//!   `Ok(None)`, tone practice is untouched, and no code path here runs until
//!   [`Asr::install`] is called. Nothing prompts, ever.
//! - **The download is verified** against a pinned SHA-256, and a failure leaves
//!   nothing behind to be mistaken for a working install.
//! - **Declining is a first-class state.** A learner can use this app for years
//!   and never see any of it.
//!
//! ## What this is evidence of, and what it is not
//!
//! A recogniser is built to be robust to the errors a learner makes: it carries a
//! strong language-model prior and will repair a wrong syllable toward the likely
//! word, most often for the learners who most need telling (research §6.1). Two
//! consequences are designed in, not worked around:
//!
//! - **Contextual biasing is left off on purpose.** sherpa-onnx can be pointed at
//!   a hotwords file, and pointing it at the expected answer would bias decoding
//!   *toward the target* — which is the right tool for transcribing rare
//!   vocabulary and exactly the wrong one for assessment. Nothing here sets
//!   `hotwords_file`.
//! - **The transcript is never presented as a pronunciation score.** It says
//!   which syllables were heard; `hanzi_core::pinyin::heard_against` reads it
//!   against the target and words the result so the tone and the transcription
//!   cannot be confused for one another.
//!
//! Inverse text normalisation is off (`use_itn: false`) for a related reason:
//! with it on, `一` comes back as `1`, which cannot be read as a syllable. The
//! language is pinned to `zh` rather than auto-detected because a single syllable
//! is short enough for auto-detection to guess English.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use sha2::{Digest, Sha256};
use sherpa_onnx::{
    LinearResampler, OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig,
};

/// The model this app recognises with, and everything the settings screen has to
/// be able to say about it *before* the learner agrees to fetch it.
///
/// Every number here is measured rather than estimated, because the settings
/// screen states them to someone who is deciding whether to spend the bandwidth:
/// the archive size is the release asset's own `Content-Length`
/// (163,002,883 bytes) and the unpacked size is the sum of the files inside it
/// (240,506,435 bytes). The digest is of the archive as released, checked by
/// [`Asr::install`] on every download.
pub struct ModelSpec {
    /// What to call it in the interface.
    pub name: &'static str,
    /// The directory the archive unpacks to, which is also its name upstream.
    pub stem: &'static str,
    pub url: &'static str,
    pub archive_bytes: u64,
    pub unpacked_bytes: u64,
    pub sha256: &'static str,
    /// The licence the weights are under — which is **not** the licence of the
    /// toolkit that runs them. See LICENSES.md; this app does not redistribute
    /// the weights, it points at upstream and the learner downloads them.
    pub licence: &'static str,
    pub licence_url: &'static str,
}

/// `SenseVoiceSmall`, int8, as `sherpa-onnx` publishes it.
pub const MODEL: ModelSpec = ModelSpec {
    name: "SenseVoiceSmall (int8)",
    stem: "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17",
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/\
          sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17.tar.bz2",
    archive_bytes: 163_002_883,
    unpacked_bytes: 240_506_435,
    sha256: "7d1efa2138a65b0b488df37f8b89e3d91a60676e416f515b952358d83dfd347e",
    licence: "FunASR Model Open Source License Agreement v1.1 (Alibaba Group)",
    licence_url: "https://github.com/modelscope/FunASR/blob/main/MODEL_LICENSE",
};

/// The two files the recogniser cannot be built without.
const MODEL_FILE: &str = "model.int8.onnx";
const TOKENS_FILE: &str = "tokens.txt";

/// The rate the model was trained at, and the rate it is fed.
///
/// The same number `hanzi_core::tone` analyses at — see [`transcribe`] for why
/// the two agreeing is a convenience rather than a coincidence.
use hanzi_core::tone::TARGET_SAMPLE_RATE as MODEL_RATE;

/// Where an install has got to.
///
/// `Absent` and `Failed` are different states on purpose. "I have not been asked"
/// and "I was asked and it did not work" want different words in the interface,
/// and collapsing them would turn a transient network failure into a permanent
/// silence.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallState {
    /// Nothing downloaded, nothing asked for. The state of every fresh install.
    #[default]
    Absent,
    /// A download is running now.
    Downloading,
    /// The model is unpacked, verified and usable.
    Installed,
    /// The last attempt failed; `error` says how.
    Failed,
}

/// What the interface needs to render the ASR section of the settings screen.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrStatus {
    pub state: InstallState,
    /// True only when the model is present *and* both files are readable, which
    /// is what makes recognition worth offering.
    pub installed: bool,
    /// The model's name, source, size and licence — always present, because the
    /// settings screen has to be able to describe the download to someone who
    /// has not agreed to it yet.
    pub model: &'static str,
    pub url: &'static str,
    pub licence: &'static str,
    pub licence_url: &'static str,
    pub download_bytes: u64,
    pub unpacked_bytes: u64,
    /// Bytes fetched so far, while a download is running.
    pub downloaded: u64,
    /// Where the model is, once it is anywhere.
    pub path: Option<String>,
    /// Why the last attempt failed.
    pub error: Option<String>,
    /// One plain sentence for the interface to show.
    pub detail: String,
}

/// The recogniser, and the model behind it.
///
/// Lives in [`crate::state::AppState`] beside the microphone and the speaker, and
/// holds nothing until it is asked for something: no model is loaded, no file is
/// opened and no socket is touched until [`Asr::install`] or [`Asr::recognize`]
/// is called.
#[derive(Default)]
pub struct Asr {
    /// Where the model lives, once the app knows its data directory. `None` when
    /// the platform could not give one, in which case there is nowhere to put a
    /// 163 MB download and the settings screen says so.
    root: Option<PathBuf>,
    /// The download in flight, or the outcome of the last one. Behind an `Arc`
    /// because the download runs on its own thread and reports progress by
    /// updating this while it goes — there are no events in this app, so the
    /// interface polls [`Asr::status`] instead.
    install: Arc<Mutex<Install>>,
    /// The loaded recogniser, kept between utterances.
    ///
    /// Loading 228 MB of ONNX is a second or two, and a learner practises in
    /// bursts, so it is built once and held. Behind a `Mutex<Option<Arc<_>>>`
    /// rather than a `OnceLock` because an uninstall has to be able to take it
    /// away again, and it is an `Arc` so the lock is not held for the length of
    /// a recognition.
    engine: Mutex<Option<Arc<Engine>>>,
}

/// What a download is doing, shared with the thread doing it.
#[derive(Default)]
struct Install {
    state: InstallState,
    downloaded: u64,
    error: Option<String>,
    /// The learner asked for the download in flight to stop. Read by the
    /// download thread between chunks — see [`Asr::cancel`] — and kept after it
    /// stops, so the attempt it ends reads as "not installed" rather than
    /// "failed".
    cancelled: bool,
}

impl Asr {
    /// Hold the model under `data_dir`, if the platform gave us one.
    ///
    /// Nothing is created here. A directory that does not exist yet is not an
    /// error, it is a model that has not been asked for.
    pub fn new(data_dir: Option<&Path>) -> Self {
        Self {
            root: data_dir.map(|dir| dir.join("asr")),
            install: Arc::new(Mutex::new(Install::default())),
            engine: Mutex::new(None),
        }
    }

    /// Where the unpacked model is, when it is anywhere.
    fn model_dir(&self) -> Option<PathBuf> {
        self.root.as_ref().map(|root| root.join(MODEL.stem))
    }

    /// True when both files the recogniser needs are really there.
    ///
    /// The files rather than the directory: a failed or interrupted install can
    /// leave a directory behind, and offering recognition on the strength of a
    /// directory is how a learner ends up pressing a button that cannot work.
    fn is_installed(&self) -> bool {
        self.model_dir().is_some_and(|dir| {
            dir.join(MODEL_FILE).is_file() && dir.join(TOKENS_FILE).is_file()
        })
    }

    /// Everything the settings screen needs, in one call.
    pub fn status(&self) -> AsrStatus {
        let install = self.install.lock().expect("asr install slot");
        let installed = self.is_installed();

        // A failure is reported as a failure even if an older install is still
        // on disk — the learner asked for something and it did not happen, and
        // saying "installed" would hide that.
        //
        // A *cancelled* attempt is not a failure, though: the learner asked for
        // it to stop, and it did.
        let state = match install.state {
            InstallState::Downloading => InstallState::Downloading,
            InstallState::Failed if !install.cancelled => InstallState::Failed,
            _ if installed => InstallState::Installed,
            _ => InstallState::Absent,
        };

        let detail = match state {
            InstallState::Downloading if install.cancelled => format!(
                "Cancelling the download of {} — it stops at the next chunk, and nothing \
                 is installed.",
                MODEL.name,
            ),
            InstallState::Downloading => format!(
                "Downloading {} — {} of {} so far. Nothing is installed until the \
                 checksum passes.",
                MODEL.name,
                megabytes(install.downloaded),
                megabytes(MODEL.archive_bytes),
            ),
            InstallState::Installed => format!(
                "{} is installed, so a recording is also read as syllables. Tone \
                 practice does not need it and works without it.",
                MODEL.name,
            ),
            InstallState::Failed => install
                .error
                .clone()
                .unwrap_or_else(|| "The last attempt to fetch the model failed.".to_string()),
            InstallState::Absent if self.root.is_none() => "There is nowhere to keep the model: \
                 the platform did not give this app a directory to store it in, so it cannot \
                 be installed. Tone practice is unaffected."
                .to_string(),
            InstallState::Absent => format!(
                "Not installed. Recognising what was said needs {} ({} to download, {} on \
                 disk), which this app does not ship. Until it is installed, a recording is \
                 scored on tone only — which needs no model and is what the app has always \
                 done.",
                MODEL.name,
                megabytes(MODEL.archive_bytes),
                megabytes(MODEL.unpacked_bytes),
            ),
        };

        AsrStatus {
            state,
            installed,
            model: MODEL.name,
            url: MODEL.url,
            licence: MODEL.licence,
            licence_url: MODEL.licence_url,
            download_bytes: MODEL.archive_bytes,
            unpacked_bytes: MODEL.unpacked_bytes,
            downloaded: install.downloaded,
            path: self
                .model_dir()
                .filter(|dir| dir.is_dir())
                .map(|dir| dir.display().to_string()),
            error: if install.cancelled {
                None
            } else {
                install.error.clone()
            },
            detail,
        }
    }

    /// Fetch and unpack the model, on a thread of its own.
    ///
    /// Returns as soon as the download has been started, because a 163 MB
    /// download that a command blocked on would leave the interface with a button
    /// that never comes back. Progress is reported through [`Asr::status`], which
    /// the settings screen polls; the outcome is too. This is the app's only
    /// network access, and it happens only because a learner pressed this.
    pub fn install(&self) -> Result<(), String> {
        let Some(root) = self.root.clone() else {
            return Err("There is nowhere to keep the model: this app was not given a \
                        directory to store it in, so it cannot be installed."
                .to_string());
        };

        {
            let mut install = self.install.lock().expect("asr install slot");
            if install.state == InstallState::Downloading {
                return Err("The model is already being downloaded.".to_string());
            }
            // A retry starts from nothing: the previous attempt's byte count
            // would otherwise show a progress bar that begins part-way and means
            // nothing.
            *install = Install {
                state: InstallState::Downloading,
                downloaded: 0,
                error: None,
                cancelled: false,
            };
        }

        // Anything half-loaded belongs to a model that is about to be replaced.
        *self.engine.lock().expect("asr engine slot") = None;

        let progress = Arc::clone(&self.install);
        std::thread::spawn(move || {
            let outcome = fetch_and_unpack(&root, &progress);
            let mut install = progress.lock().expect("asr install slot");
            match outcome {
                Ok(()) => {
                    install.state = InstallState::Installed;
                    install.downloaded = MODEL.archive_bytes;
                }
                Err(message) => {
                    install.state = InstallState::Failed;
                    install.error = Some(message);
                }
            }
        });

        Ok(())
    }

    /// Ask a running download to stop, and report the resulting state.
    ///
    /// Returns at once. The download thread reads the request between two chunks
    /// of the transfer, and between downloading, checking and unpacking; it then
    /// removes the staging directory exactly as a failed attempt does, and the
    /// status goes back to "not installed". Until it has noticed, the status
    /// still says downloading, with a detail that says it is stopping — so the
    /// screen keeps polling until it settles. A request that arrives after the
    /// model has been moved into place is too late, and the install stands.
    ///
    /// Asking with nothing running changes nothing.
    pub fn cancel(&self) -> AsrStatus {
        {
            let mut install = self
                .install
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if install.state == InstallState::Downloading {
                install.cancelled = true;
            }
        }
        self.status()
    }

    /// True while a download thread is running.
    fn is_downloading(&self) -> bool {
        let install = self
            .install
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        install.state == InstallState::Downloading
    }

    /// Delete the model, and anything a failed attempt left behind.
    ///
    /// The loaded recogniser goes with it: leaving a 228 MB model resident after
    /// the learner has asked for it to be removed would be both a lie in the
    /// status line and the largest thing this process holds.
    pub fn remove(&self) -> Result<(), String> {
        *self.engine.lock().expect("asr engine slot") = None;

        let Some(root) = self.root.as_ref() else {
            return Ok(());
        };
        // Not from under a download: the thread would go on writing into a
        // directory that had just been deleted, and then report an install.
        if self.is_downloading() {
            return Err(
                "The model is still being downloaded. Cancel the download first, or let it \
                 finish, and then remove it."
                    .to_string(),
            );
        }
        if root.is_dir() {
            fs::remove_dir_all(root)
                .map_err(|error| format!("could not remove {}: {error}", root.display()))?;
        }

        let mut install = self.install.lock().expect("asr install slot");
        *install = Install::default();
        Ok(())
    }

    /// The loaded recogniser, building it if this is the first use.
    fn engine(&self) -> Result<Arc<Engine>, String> {
        let mut slot = self.engine.lock().expect("asr engine slot");
        if let Some(engine) = slot.as_ref() {
            return Ok(Arc::clone(engine));
        }
        let dir = self
            .model_dir()
            .ok_or("There is nowhere the model could be: no data directory.")?;
        let engine = Arc::new(Engine::load(&dir)?);
        *slot = Some(Arc::clone(&engine));
        Ok(engine)
    }

    /// Transcribe one recording, or `Ok(None)` when no model is installed.
    ///
    /// `Ok(None)` rather than an error, because "the learner has not installed
    /// this" is not a failure — it is the state the app ships in, and the caller
    /// is expected to carry on with tone scoring regardless.
    pub fn recognize(&self, samples: &[f32], sample_rate: u32) -> Result<Option<String>, String> {
        if !self.is_installed() {
            return Ok(None);
        }
        let engine = self.engine()?;
        engine.transcribe(samples, sample_rate).map(Some)
    }
}

/// Download, verify and unpack the model. Runs on the install thread.
///
/// The order matters: nothing is put where the app will look for it until the
/// digest has matched, so a truncated download, a captive-portal HTML page or a
/// corrupted transfer cannot become a "model" that fails later with a confusing
/// ONNX error. It is also why the work goes to a staging directory and is moved
/// into place at the end — an interrupted install leaves the previous state
/// exactly as it was.
///
/// The free space is checked first (issue #41): staging holds the archive and
/// the unpacked model at once, and a device without room for both used to find
/// out by failing a write part-way through 163 MB. On Apple platforms the check
/// only advises — see [`ROOM_IS_ADVISORY`].
fn fetch_and_unpack(root: &Path, progress: &Arc<Mutex<Install>>) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("could not create {}: {error}", root.display()))?;
    // The wreckage of an attempt that never got to clean up is space this one
    // is about to reclaim, so it goes before the space is counted.
    let _ = fs::remove_dir_all(root.join(".staging"));
    enforce_room(free_bytes(root), ROOM_IS_ADVISORY)?;
    fetch_and_unpack_from(root, progress, &open_release)
}

/// [`fetch_and_unpack`] from whatever `open` returns rather than the release, so
/// a test can drive the staging and the cancellation without a network.
fn fetch_and_unpack_from(
    root: &Path,
    progress: &Arc<Mutex<Install>>,
    open: &dyn Fn() -> Result<Box<dyn Read>, String>,
) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("could not create {}: {error}", root.display()))?;

    // Removed first so a retry cannot "succeed" by finding the wreckage of the
    // attempt before it.
    let staging = root.join(".staging");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)
        .map_err(|error| format!("could not create {}: {error}", staging.display()))?;

    // Every failure from here takes the staging directory with it. A failed
    // attempt must not leave 163 MB of partial archive — or a half-unpacked
    // model — where the next one would have to notice it, and the learner asked
    // for a thing that did not happen rather than for a cache. A cancelled one is
    // cleaned up the same way; `Asr::status` is what tells the two apart.
    //
    // A panic is a failure too (a decoder bug, a corrupt stream): left to unwind
    // it would skip the code that records the outcome, and the state would read
    // "downloading" for the life of the process, with the screen polling it.
    let staged = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        stage(&staging, progress, open)
    }))
    .unwrap_or_else(|panic| {
        Err(format!(
            "the install stopped unexpectedly ({}). Nothing was installed.",
            panic_message(panic.as_ref())
        ))
    });
    let unpacked = match staged {
        Ok(unpacked) => unpacked,
        Err(message) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(message);
        }
    };

    // Into place, replacing any older copy only now that the new one is known
    // good.
    let destination = root.join(MODEL.stem);
    let placed = (|| -> Result<(), String> {
        if destination.exists() {
            fs::remove_dir_all(&destination)
                .map_err(|error| format!("could not replace {}: {error}", destination.display()))?;
        }
        fs::rename(&unpacked, &destination).map_err(|error| {
            format!(
                "the model was downloaded and verified but could not be moved into {}: {error}",
                destination.display()
            )
        })
    })();
    if let Err(message) = placed {
        let _ = fs::remove_dir_all(&staging);
        return Err(message);
    }

    // The archive is 163 MB of something already verified and unpacked, and the
    // model directory is the only copy worth keeping.
    let _ = fs::remove_dir_all(&staging);
    Ok(())
}

/// The text a panic carried, when it carried any.
fn panic_message(panic: &(dyn std::any::Any + Send)) -> &str {
    panic
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("no message")
}

/// What a cancelled attempt returns. Never shown: [`Asr::status`] reads the flag,
/// not the message.
const CANCELLED: &str = "The download was cancelled. Nothing was installed.";

/// True when the learner has asked for the download in flight to stop.
fn cancelled(progress: &Arc<Mutex<Install>>) -> bool {
    progress.lock().is_ok_and(|install| install.cancelled)
}

/// Download, verify and unpack into `staging`, returning the unpacked directory.
///
/// Split out of [`fetch_and_unpack`] so that cleanup happens on *every* failure
/// rather than on the ones somebody remembered to write. A cancel is looked for
/// between the steps as well as inside the download, because the check and the
/// unpack are seconds each on a phone.
fn stage(
    staging: &Path,
    progress: &Arc<Mutex<Install>>,
    open: &dyn Fn() -> Result<Box<dyn Read>, String>,
) -> Result<PathBuf, String> {
    let archive = staging.join("model.tar.bz2");
    let stop_if_cancelled = || {
        if cancelled(progress) {
            Err(CANCELLED.to_string())
        } else {
            Ok(())
        }
    };

    download(&archive, progress, open()?)?;
    stop_if_cancelled()?;
    verify(&archive)?;
    stop_if_cancelled()?;
    unpack(&archive, staging)?;
    stop_if_cancelled()?;

    let unpacked = staging.join(MODEL.stem);
    for required in [MODEL_FILE, TOKENS_FILE] {
        if !unpacked.join(required).is_file() {
            return Err(format!(
                "The download was verified but did not contain {required}, so it is not the \
                 model this app expects. Nothing was installed."
            ));
        }
    }
    Ok(unpacked)
}

/// Open the release asset for reading. This is the network access.
fn open_release() -> Result<Box<dyn Read>, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(30))
        // Per read, not overall: a 163 MB download legitimately takes minutes,
        // and a total timeout would fail a slow connection that is working
        // perfectly well.
        .timeout_read(Duration::from_secs(120))
        .user_agent(concat!("HanziTutor/", env!("CARGO_PKG_VERSION")))
        .build();

    let response = agent
        .get(MODEL.url)
        .call()
        .map_err(|error| format!("could not download {}: {error}", MODEL.url))?;
    Ok(Box::new(response.into_reader()))
}

/// Stream the archive to disk, counting bytes for the progress bar.
///
/// The digest is computed while writing rather than by reading the file back, so
/// verifying a 163 MB download costs no second pass over it. A cancel is read
/// with each chunk's progress, under the same lock.
fn download(
    destination: &Path,
    progress: &Arc<Mutex<Install>>,
    mut reader: Box<dyn Read>,
) -> Result<(), String> {
    let file = File::create(destination)
        .map_err(|error| format!("could not write {}: {error}", destination.display()))?;
    let mut file = std::io::BufWriter::new(file);

    let mut buffer = vec![0u8; 64 * 1024];
    let mut hasher = Sha256::new();
    let mut written: u64 = 0;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("the download stopped after {written} bytes: {error}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| format!("could not write to disk after {written} bytes: {error}"))?;
        hasher.update(&buffer[..read]);
        written += read as u64;
        let stop = match progress.lock() {
            Ok(mut install) => {
                install.downloaded = written;
                install.cancelled
            }
            Err(_) => false,
        };
        if stop {
            return Err(CANCELLED.to_string());
        }
    }
    file.flush()
        .map_err(|error| format!("could not finish writing the download: {error}"))?;

    if written != MODEL.archive_bytes {
        return Err(format!(
            "the download stopped early: {written} bytes where the release has {}. \
             Nothing was installed — try again.",
            MODEL.archive_bytes
        ));
    }

    let actual = hex(&hasher.finalize());
    if actual != MODEL.sha256 {
        let _ = fs::remove_file(destination);
        return Err(format!(
            "the download did not match its published checksum, so it was discarded. \
             Expected {}, got {actual}. Nothing was installed.",
            MODEL.sha256
        ));
    }
    Ok(())
}

/// Re-check the archive on disk.
///
/// [`download`] already hashed the bytes as they arrived, so this is a second,
/// independent read for the case that matters most: the digest is the one thing
/// standing between a hostile or broken network and 228 MB of ONNX being handed
/// to a parser, and reading it back off the disk also catches a write that did
/// not survive.
fn verify(archive: &Path) -> Result<(), String> {
    let mut file = File::open(archive)
        .map_err(|error| format!("could not re-read {}: {error}", archive.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("could not re-read the download: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = hex(&hasher.finalize());
    if actual != MODEL.sha256 {
        return Err(format!(
            "the download did not match its published checksum, so it was discarded. \
             Expected {}, got {actual}. Nothing was installed.",
            MODEL.sha256
        ));
    }
    Ok(())
}

/// Unpack the archive into `into`, which ends up holding `<stem>/`.
fn unpack(archive: &Path, into: &Path) -> Result<(), String> {
    let file = File::open(archive)
        .map_err(|error| format!("could not open {}: {error}", archive.display()))?;
    let decoder = bzip2::read::BzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    // `unpack` refuses entries that would escape the destination, so a verified
    // archive still cannot write outside the staging directory.
    tar.unpack(into)
        .map_err(|error| format!("could not unpack the model: {error}"))?;
    Ok(())
}

/// A lowercase hex digest.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// A byte count as the interface should say it: whole megabytes, rounded up.
///
/// Rounded up rather than to nearest, because under-reporting a download someone
/// is deciding whether to make is the wrong direction to be wrong in.
fn megabytes(bytes: u64) -> String {
    format!("{} MB", bytes.div_ceil(1_000_000))
}

/// The most an install has on disk at once: the archive and the model unpacked
/// beside it in staging, plus a margin so that installing it does not leave the
/// device with no room at all.
///
/// The older copy an install replaces is not counted — it is removed before the
/// new one is moved into place, and the screen offers the download only when
/// there is none.
const INSTALL_NEEDS_BYTES: u64 = MODEL.archive_bytes + MODEL.unpacked_bytes + 50_000_000;

/// Refuse an install there is no room for, before any of it is fetched.
///
/// `available` is `None` when the platform cannot say, which is not a reason to
/// refuse: the download then fails at the write that does not fit, as it always
/// did, and leaves nothing behind.
fn room_for_install(available: Option<u64>) -> Result<(), String> {
    match available {
        Some(free) if free < INSTALL_NEEDS_BYTES => Err(format!(
            "There is not enough free space for {}: installing it needs {} at once (the \
             download and the unpacked model side by side), and this device has {} free. \
             Nothing was downloaded.",
            MODEL.name,
            megabytes(INSTALL_NEEDS_BYTES),
            megabytes(free),
        )),
        _ => Ok(()),
    }
}

/// True where the free-space count leaves out what the system would purge.
///
/// On Apple platforms `f_bavail` does not include purgeable space — caches and
/// offloadable content the system clears on demand — so a phone that could fit
/// the model can look full. The count that does include it is a Foundation
/// resource key (`volumeAvailableCapacityForImportantUsage`), which needs
/// Objective-C glue this crate does not have; so there the check advises.
const ROOM_IS_ADVISORY: bool = cfg!(any(target_os = "ios", target_os = "macos"));

/// [`room_for_install`], refusing only where the count can be trusted: where it
/// is `advisory`, a shortfall is logged and the install goes ahead, and a write
/// that really does not fit fails and is cleaned up as any failure is.
fn enforce_room(available: Option<u64>, advisory: bool) -> Result<(), String> {
    match room_for_install(available) {
        Err(message) if advisory => {
            eprintln!(
                "[asr] {message} Going ahead anyway: this platform's count leaves out \
                 space it would free on demand."
            );
            Ok(())
        }
        other => other,
    }
}

/// The bytes an unprivileged process can still write on the filesystem holding
/// `path`, or `None` when that cannot be asked.
///
/// `statvfs` is the one call that answers this on every Unix the apps ship to —
/// macOS, iOS, Linux and Android — and `f_bavail` rather than `f_bfree` is the
/// count that excludes the blocks reserved for root.
#[cfg(unix)]
fn free_bytes(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `path` is a NUL-terminated string that outlives the call, and
    // `stats` is written in full by a call that returns 0; it is read only then.
    let stats = unsafe {
        if libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) != 0 {
            return None;
        }
        stats.assume_init()
    };
    // The field widths differ between platforms, so both go through `u64`.
    #[allow(clippy::unnecessary_cast)]
    let free = (stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64);
    Some(free)
}

/// No portable way to ask without a new dependency, so the install goes ahead
/// as it always did — see [`room_for_install`].
#[cfg(not(unix))]
fn free_bytes(_path: &Path) -> Option<u64> {
    None
}

/// Where to keep the model: the platform's cache directory when it gives one,
/// and the data directory otherwise. Pass the result to [`Asr::new`].
///
/// The weights are 240 MB of something fetched only on request and re-verified
/// by digest, so they are a cache, not the learner's data. On iOS the data
/// directory is `Library/Application Support`, which iCloud backs up, and Apple's
/// data-storage rules for App Review keep re-downloadable content out of a backup;
/// `Library/Caches` is excluded from one, and the cache directories elsewhere are
/// the platform's own place for exactly this (issue #41). The full app made the
/// same move in its `relocate_model_cache`. The price is that iOS and Android may
/// empty a cache when storage runs low; [`Asr::status`] looks at the files every
/// time, so a purged model reads as not installed and can be fetched again.
///
/// A model an earlier build put under `data_dir` is moved across rather than
/// fetched again — copied, when the two are on different filesystems — unless the
/// cache already holds one; when that one is a complete install, the old copy is
/// deleted. If the move fails the old location is kept for this session, so an
/// installed model keeps working and stays removable from the screen; the next
/// launch tries again.
pub fn model_home(data_dir: Option<&Path>, cache_dir: Option<&Path>) -> Option<PathBuf> {
    let Some(cache_dir) = cache_dir else {
        return data_dir.map(Path::to_path_buf);
    };
    let Some(data_dir) = data_dir else {
        return Some(cache_dir.to_path_buf());
    };
    let (old, new) = (data_dir.join("asr"), cache_dir.join("asr"));
    if !old.is_dir() {
        return Some(cache_dir.to_path_buf());
    }
    if new.exists() {
        // A complete install in the cache makes the old copy up to 240 MB of
        // the same thing in the backup. Anything less and the old copy may be
        // the only one that works, so it stays.
        if Asr::new(Some(cache_dir)).is_installed() {
            if let Err(error) = fs::remove_dir_all(&old) {
                eprintln!(
                    "[asr] could not remove the older copy {}: {error}",
                    old.display()
                );
            }
        }
        return Some(cache_dir.to_path_buf());
    }
    let moved = fs::create_dir_all(cache_dir)
        .and_then(|()| move_model(&old, &new, &|a, b| fs::rename(a, b)));
    match moved {
        Ok(()) => Some(cache_dir.to_path_buf()),
        Err(error) => {
            eprintln!(
                "[asr] could not move {} to {}: {error}; keeping it where it is",
                old.display(),
                new.display()
            );
            Some(data_dir.to_path_buf())
        }
    }
}

/// Move the model directory `old` to `new`, through `rename`.
///
/// A rename cannot cross filesystems, and a cache directory on another volume
/// is not unusual on a desktop. Then the tree is copied to a sibling of `new`
/// and renamed into place, so a copy cut short is never where [`Asr::status`]
/// would read it as an install; the old copy is deleted only once the new one
/// is in place, and failing to delete it is logged rather than undoing the move
/// — the next launch removes it (see [`model_home`]).
fn move_model(
    old: &Path,
    new: &Path,
    rename: &dyn Fn(&Path, &Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    match rename(old, new) {
        Err(error) if error.kind() == std::io::ErrorKind::CrossesDevices => {
            let temporary = new.with_file_name(".asr-moving");
            let _ = fs::remove_dir_all(&temporary);
            if let Err(error) = copy_tree(old, &temporary).and_then(|()| rename(&temporary, new)) {
                let _ = fs::remove_dir_all(&temporary);
                return Err(error);
            }
            if let Err(error) = fs::remove_dir_all(old) {
                eprintln!(
                    "[asr] moved, but could not remove {}: {error}",
                    old.display()
                );
            }
            Ok(())
        }
        other => other,
    }
}

/// Copy a directory tree. The model archive holds only directories and files.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// The recogniser, once the model has been loaded.
///
/// `OfflineRecognizer` is `Send + Sync`, so this is shared behind an `Arc` and a
/// recognition holds no lock on the slot it came from.
struct Engine {
    recognizer: OfflineRecognizer,
}

impl Engine {
    /// Load the model. This is the expensive step — a second or two, and 228 MB
    /// resident — which is why it happens on first use and is then kept.
    fn load(model_dir: &Path) -> Result<Self, String> {
        let model = model_dir.join(MODEL_FILE);
        let tokens = model_dir.join(TOKENS_FILE);

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.sense_voice = OfflineSenseVoiceModelConfig {
            model: Some(model.display().to_string()),
            // Pinned rather than left to the model's own detection: a single
            // syllable — which is most of what this app recognises — is short
            // enough that auto-detection guesses, and a Mandarin syllable
            // transcribed as English is worse than useless here.
            language: Some("zh".to_string()),
            // Off, deliberately. With it on, 一 comes back as "1", which cannot
            // be read as a syllable and so cannot be compared to the target.
            use_itn: false,
        };
        config.model_config.tokens = Some(tokens.display().to_string());
        // Left at the model default: one thread per recognition is already far
        // faster than real time for an utterance this short, and taking every
        // core would make the practice board stutter while it works.
        config.model_config.num_threads = 1;

        OfflineRecognizer::create(&config).ok_or_else(|| {
            format!(
                "the model at {} could not be loaded. Removing it and installing it again \
                 is the thing to try.",
                model_dir.display()
            )
        })
        .map(|recognizer| Self { recognizer })
    }

    /// One utterance in, one transcript out.
    fn transcribe(&self, samples: &[f32], sample_rate: u32) -> Result<String, String> {
        let samples = to_model_rate(samples, sample_rate)?;

        // No hotwords, no contextual biasing — see the module note. This is the
        // API that would make the recogniser agree with the answer sheet.
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(MODEL_RATE as i32, &samples);
        self.recognizer.decode(&stream);

        let result = stream
            .get_result()
            .ok_or("the recogniser returned no result for that recording")?;
        Ok(clean(&result.text))
    }
}

/// Bring a recording to the rate the model was trained at.
///
/// A real resampler, not the linear interpolation in `hanzi_core::tone` — that
/// one is deliberately the simplest thing that works for an F0 estimator and says
/// so in its own documentation ("a later ASR stage needs a real resampler"). It
/// aliases: high-frequency hiss folds down into the pass band, which is cosmetic
/// for pitch and would be audible for speech, and the bands it damages are
/// precisely those that separate `s`/`sh`/`x` — the Mandarin contrasts a learner
/// is here to practise. `LinearResampler` is `sherpa-onnx`'s own, low-pass
/// filtered with six zeros, so the model is fed what it expects and no new
/// dependency is needed for it.
fn to_model_rate(samples: &[f32], sample_rate: u32) -> Result<Vec<f32>, String> {
    if sample_rate == MODEL_RATE {
        return Ok(samples.to_vec());
    }
    // A device that reports no rate at all would otherwise be "resampled" from
    // 0 Hz, which is a division that produces nothing usable — and a filter
    // cutoff of zero, so the resampler would return silence rather than fail.
    if sample_rate == 0 {
        return Err("The recording did not come with a sample rate, so it cannot be \
                    prepared for recognition."
            .to_string());
    }
    let resampler = LinearResampler::create(sample_rate as i32, MODEL_RATE as i32).ok_or_else(
        || format!("could not resample {sample_rate} Hz audio to {MODEL_RATE} Hz"),
    )?;
    // `flush` on the only chunk there is, so nothing is left in the filter.
    Ok(resampler.resample(samples, true))
}

/// Strip the tags SenseVoice writes into its own output.
///
/// The model emits language, emotion and audio-event markers inline —
/// `<|zh|><|NEUTRAL|><|Speech|><|woitn|>开饭时间…` — and only the text after them
/// is a transcription. They are removed here rather than left for a caller to
/// trip over, because they would otherwise become "syllables" that no target
/// could ever match.
///
/// Only the `<|…|>` form is treated as a tag, which is the only form the model
/// emits. Everything else is text, including a `<` with no closing `|>`: an
/// unterminated opener is *kept*, because the alternative — dropping everything
/// after it — would silently lose part of the transcription, and a wrong
/// transcript that looks complete is worse than one that looks odd.
fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<|") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("|>") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => {
                out.push_str(&rest[start..]);
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model directory to test against, from the environment.
    ///
    /// The model is 163 MB and is not in the repository, so the two tests that
    /// need a real one are ignored by default and take the directory from
    /// `HANZI_ASR_MODEL_DIR`. Point it at an unpacked
    /// `sherpa-onnx-sense-voice-…-int8-2024-07-17` and run:
    ///
    /// ```text
    /// HANZI_ASR_MODEL_DIR=… cargo test -p hanzi-tutor --lib -- --ignored --nocapture asr
    /// ```
    fn model_dir_from_env() -> Option<PathBuf> {
        let dir = PathBuf::from(std::env::var("HANZI_ASR_MODEL_DIR").ok()?);
        dir.is_dir().then_some(dir)
    }

    #[test]
    fn a_fresh_install_has_no_model_and_says_so() {
        let directory = std::env::temp_dir().join("hanzi-asr-absent");
        let _ = fs::remove_dir_all(&directory);
        let asr = Asr::new(Some(&directory));

        let status = asr.status();
        assert_eq!(status.state, InstallState::Absent);
        assert!(!status.installed);
        assert!(status.error.is_none());
        // The screen has to be able to describe a download nobody has agreed to
        // yet, so the description is there before the model is.
        assert_eq!(status.model, MODEL.name);
        assert!(status.url.starts_with("https://"));
        assert!(status.download_bytes > 0);
        assert!(status.unpacked_bytes > status.download_bytes);
        assert!(status.detail.contains("Not installed"), "{}", status.detail);
        assert!(
            status.detail.contains("tone only"),
            "the detail must say what still works without it: {}",
            status.detail
        );

        // And no recognition is offered, without that being an error.
        assert_eq!(asr.recognize(&[0.0; 160], 16_000).unwrap(), None);
    }

    #[test]
    fn there_is_nowhere_to_put_a_model_without_a_data_directory() {
        let asr = Asr::new(None);
        assert_eq!(asr.status().state, InstallState::Absent);
        assert!(!asr.status().installed);
        assert!(asr.status().detail.contains("nowhere to keep"), "{}", asr.status().detail);
        // Installing is refused rather than half-done.
        assert!(asr.install().is_err());
    }

    #[test]
    fn a_directory_without_the_model_files_is_not_an_install() {
        // The failure this guards: an interrupted or failed install leaves a
        // directory behind, and offering recognition on the strength of a
        // directory gives the learner a button that cannot work.
        let directory = std::env::temp_dir().join("hanzi-asr-partial");
        let _ = fs::remove_dir_all(&directory);
        let stem = directory.join("asr").join(MODEL.stem);
        fs::create_dir_all(&stem).unwrap();
        fs::write(stem.join(MODEL_FILE), b"not really a model").unwrap();

        let asr = Asr::new(Some(&directory));
        assert!(!asr.status().installed, "one of the two files is not an install");
        assert_eq!(asr.status().state, InstallState::Absent);

        // And the token file alone is not one either.
        fs::remove_file(stem.join(MODEL_FILE)).unwrap();
        fs::write(stem.join(TOKENS_FILE), b"<blk> 0\n").unwrap();
        assert!(!asr.status().installed);

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn removing_takes_the_model_and_the_state_with_it() {
        let directory = std::env::temp_dir().join("hanzi-asr-remove");
        let _ = fs::remove_dir_all(&directory);
        let stem = directory.join("asr").join(MODEL.stem);
        fs::create_dir_all(&stem).unwrap();
        fs::write(stem.join(MODEL_FILE), b"model").unwrap();
        fs::write(stem.join(TOKENS_FILE), b"tokens").unwrap();

        let asr = Asr::new(Some(&directory));
        assert!(asr.status().installed);

        asr.remove().unwrap();
        assert!(!asr.status().installed);
        assert_eq!(asr.status().state, InstallState::Absent);
        assert!(!stem.exists());
        // Removing something that is not there is not an error: the learner
        // asked for it to be gone, and it is.
        asr.remove().unwrap();

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn sense_voice_tags_are_not_mistaken_for_syllables() {
        // What the model actually returns, tags and all.
        assert_eq!(clean("<|zh|><|NEUTRAL|><|Speech|><|woitn|>开饭时间"), "开饭时间");
        assert_eq!(clean("<|en|><|EMO_UNKNOWN|><|Speech|>Hello there"), "Hello there");
        assert_eq!(clean("你好"), "你好");
        assert_eq!(clean("  你好  "), "你好");
        assert_eq!(clean(""), "");
        assert_eq!(clean("<|zh|>"), "");
        // An unterminated opener is kept as text. Dropping everything after it
        // would lose part of the transcription, and a transcript that is silently
        // short is worse than one that looks odd — the comparison would report a
        // missing syllable that was really said.
        assert_eq!(clean("<|zh|>你好<|"), "你好<|");
        // And a stray angle bracket that is not a tag at all is left alone.
        assert_eq!(clean("a < b"), "a < b");
    }

    #[test]
    fn a_byte_count_is_never_under_reported() {
        assert_eq!(megabytes(0), "0 MB");
        assert_eq!(megabytes(1), "1 MB");
        assert_eq!(megabytes(1_000_000), "1 MB");
        // Rounded up: under-reporting a download someone is deciding whether to
        // make is the wrong direction to be wrong in.
        assert_eq!(megabytes(1_000_001), "2 MB");
        assert_eq!(megabytes(MODEL.archive_bytes), "164 MB");
        assert_eq!(megabytes(MODEL.unpacked_bytes), "241 MB");
    }

    #[test]
    fn audio_already_at_the_model_rate_is_left_alone() {
        let samples = vec![0.1f32, -0.2, 0.3];
        assert_eq!(to_model_rate(&samples, MODEL_RATE).unwrap(), samples);
    }

    #[test]
    fn a_recording_with_no_sample_rate_is_refused_rather_than_silenced() {
        // Left unguarded this becomes a resample from 0 Hz: a zero filter cutoff,
        // which returns silence rather than an error, and the learner is told the
        // recogniser heard nothing.
        assert!(to_model_rate(&[0.1, 0.2], 0).is_err());
    }

    #[test]
    fn resampling_to_the_model_rate_lands_on_the_right_length() {
        // A real resampler, so the length is what the ratio says rather than
        // exactly the ratio: the filter's own delay is part of the answer.
        let samples: Vec<f32> = (0..48_000)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();
        let out = to_model_rate(&samples, 48_000).unwrap();
        let expected = 16_000;
        assert!(
            (out.len() as i64 - expected as i64).abs() <= 64,
            "48000 samples at 48 kHz should be about 16000 at 16 kHz, got {}",
            out.len()
        );
        assert!(out.iter().all(|s| s.is_finite()));
        assert!(out.iter().any(|s| s.abs() > 0.1), "the signal must survive");
    }

    /// The one test that needs the real model, and no network.
    ///
    /// It uses the model's own `test_wavs/zh.wav`, which the archive ships, so
    /// what it asserts is that the whole path — load, resample, decode, strip the
    /// tags — produces Chinese text from a known Chinese recording.
    #[test]
    #[ignore = "needs HANZI_ASR_MODEL_DIR pointing at an unpacked model"]
    fn recognises_the_models_own_chinese_recording() {
        let dir = model_dir_from_env()
            .expect("set HANZI_ASR_MODEL_DIR to an unpacked SenseVoice model directory");
        let wav = dir.join("test_wavs/zh.wav");
        let wave = sherpa_onnx::Wave::read(&wav.display().to_string())
            .unwrap_or_else(|| panic!("could not read {}", wav.display()));

        let engine = Engine::load(&dir).expect("the model should load");
        let text = engine
            .transcribe(wave.samples(), wave.sample_rate() as u32)
            .expect("transcription should succeed");

        println!("zh.wav ({} Hz) → {text}", wave.sample_rate());
        assert!(!text.is_empty(), "a 5 second Chinese recording produced nothing");
        assert!(
            text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "the transcript should contain Chinese characters, got {text:?}"
        );
        assert!(!text.contains("<|"), "the tags should have been stripped: {text:?}");
        assert!(
            !text.chars().any(|c| c.is_ascii_digit()),
            "ITN is off, so 一 must not come back as a digit: {text:?}"
        );
    }

    /// The same path through the resampler, for a device that does not record at
    /// 16 kHz — which is most of them.
    ///
    /// A wrong resampler shows up here and almost nowhere else: linear
    /// interpolation aliases the top band down into the pass band, and the sounds
    /// it damages are exactly the sibilants that separate `s`/`sh`/`x`. Taking the
    /// recording up to 48 kHz and letting the recogniser bring it back down must
    /// not change a syllable.
    #[test]
    #[ignore = "needs HANZI_ASR_MODEL_DIR pointing at an unpacked model"]
    fn recognises_a_recording_resampled_from_48k() {
        let dir = model_dir_from_env().expect("set HANZI_ASR_MODEL_DIR");
        let wav = dir.join("test_wavs/zh.wav");
        let wave = sherpa_onnx::Wave::read(&wav.display().to_string()).expect("read zh.wav");

        let engine = Engine::load(&dir).expect("the model should load");
        let at_rate = engine
            .transcribe(wave.samples(), wave.sample_rate() as u32)
            .expect("16 kHz transcription");
        let upsampled = LinearResampler::create(16_000, 48_000)
            .expect("resampler")
            .resample(wave.samples(), true);
        let from_48k = engine
            .transcribe(&upsampled, 48_000)
            .expect("transcription after resampling");

        println!("16 kHz → {at_rate}");
        println!("48 kHz → {from_48k}");
        assert_eq!(
            at_rate, from_48k,
            "resampling to 48 kHz and back must not change what was heard"
        );
    }

    /// The whole feature over the real network: fetch, verify, unpack, recognise.
    ///
    /// Ignored by default because it downloads 163 MB, and it is the only test
    /// that pins the *published* digest — `MODEL.sha256` is checked here against
    /// what GitHub actually serves, so a re-cut or tampered release asset fails
    /// here rather than on a learner's machine. Run it by hand when touching
    /// [`download`], [`verify`] or [`unpack`], or when bumping the model:
    ///
    /// ```text
    /// cargo test -p hanzi-tutor --lib -- --ignored --nocapture downloads_verifies
    /// ```
    #[test]
    #[ignore = "downloads 163 MB from GitHub"]
    fn downloads_verifies_and_installs_the_model() {
        let directory = std::env::temp_dir().join("hanzi-asr-download");
        let _ = fs::remove_dir_all(&directory);
        let asr = Asr::new(Some(&directory));
        assert_eq!(asr.status().state, InstallState::Absent);

        asr.install().expect("the download should start");

        let deadline = std::time::Instant::now() + Duration::from_secs(900);
        let mut last = 0u64;
        loop {
            let status = asr.status();
            if status.state != InstallState::Downloading {
                assert_eq!(
                    status.state,
                    InstallState::Installed,
                    "install failed: {}",
                    status.error.unwrap_or_default()
                );
                assert!(status.downloaded > 0);
                break;
            }
            // Progress has to actually move, or the settings screen shows a bar
            // that never fills and a learner cannot tell working from hung.
            if status.downloaded > last {
                println!("  {} bytes", status.downloaded);
                last = status.downloaded;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the download did not finish within 15 minutes"
            );
            std::thread::sleep(Duration::from_millis(500));
        }

        assert!(asr.status().installed);
        // Nothing of the staging directory is left behind.
        assert!(!directory.join("asr/.staging").exists(), "staging should be cleaned up");

        // And the thing it installed really recognises: the archive ships a known
        // Chinese recording, so the check is end to end rather than "a file
        // exists".
        let wav = asr.model_dir().unwrap().join("test_wavs/zh.wav");
        let wave = sherpa_onnx::Wave::read(&wav.display().to_string()).expect("read zh.wav");
        let text = asr
            .recognize(wave.samples(), wave.sample_rate() as u32)
            .expect("recognition should run")
            .expect("a model is installed");
        println!("installed model heard: {text}");
        assert!(!text.is_empty(), "the freshly installed model transcribed nothing");

        // Removing it takes the files and the state with it.
        asr.remove().expect("removal should succeed");
        assert!(!asr.status().installed);
        assert!(!directory.join("asr").join(MODEL.stem).exists());

        let _ = fs::remove_dir_all(&directory);
    }

    /// A source that never ends, and that asks for the download to be cancelled
    /// once it has been read from — as a learner pressing Cancel part-way does.
    struct CancelledPartWay {
        reads: usize,
        progress: Arc<Mutex<Install>>,
    }

    impl Read for CancelledPartWay {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            if self.reads == 2 {
                self.progress.lock().unwrap().cancelled = true;
            }
            buffer.fill(0);
            Ok(buffer.len())
        }
    }

    #[test]
    fn a_cancelled_download_stops_and_leaves_nothing_behind() {
        // Issue #41: a Cancel button, so a learner on a slow link is not left
        // waiting out a 163 MB download they no longer want. The flag is read
        // between chunks, the partial archive goes with the staging directory,
        // and the screen goes back to "not installed" rather than "failed".
        let directory = std::env::temp_dir().join("hanzi-asr-cancel");
        let _ = fs::remove_dir_all(&directory);
        let asr = Asr::new(Some(&directory));
        let root = asr.root.clone().unwrap();
        hold_install(&asr).state = InstallState::Downloading;

        let progress = Arc::clone(&asr.install);
        let source = || -> Result<Box<dyn Read>, String> {
            Ok(Box::new(CancelledPartWay {
                reads: 0,
                progress: Arc::clone(&progress),
            }))
        };
        let outcome = fetch_and_unpack_from(&root, &asr.install, &source);
        assert!(outcome.is_err(), "a cancelled download is not an install");
        let downloaded = hold_install(&asr).downloaded;
        assert!(
            downloaded > 0 && downloaded < MODEL.archive_bytes,
            "it stopped part-way, after {downloaded} bytes"
        );
        assert!(
            !root.join(".staging").exists(),
            "the partial archive is removed"
        );
        assert!(!root.join(MODEL.stem).exists());

        // What the install thread does with any error, and then what the screen
        // is told.
        {
            let mut install = hold_install(&asr);
            install.state = InstallState::Failed;
            install.error = outcome.err();
        }
        let status = asr.status();
        assert_eq!(status.state, InstallState::Absent);
        assert!(status.error.is_none(), "{:?}", status.error);
        assert!(!status.installed);

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn cancel_is_only_for_a_download_that_is_running() {
        let directory = std::env::temp_dir().join("hanzi-asr-cancel-idle");
        let _ = fs::remove_dir_all(&directory);
        let asr = Asr::new(Some(&directory));

        // Nothing running: nothing to cancel, and nothing changes.
        assert_eq!(asr.cancel().state, InstallState::Absent);
        assert!(!hold_install(&asr).cancelled);

        // Running: the request is recorded, and the screen says it is stopping
        // until the download thread notices, between two chunks.
        hold_install(&asr).state = InstallState::Downloading;
        let status = asr.cancel();
        assert_eq!(status.state, InstallState::Downloading);
        assert!(hold_install(&asr).cancelled);
        assert!(status.detail.contains("Cancelling"), "{}", status.detail);

        // A real failure is still a failure.
        let fresh = Asr::new(Some(&directory));
        {
            let mut install = hold_install(&fresh);
            install.state = InstallState::Failed;
            install.error = Some("the network went away".to_string());
        }
        assert_eq!(fresh.status().state, InstallState::Failed);
    }

    fn hold_install(asr: &Asr) -> std::sync::MutexGuard<'_, Install> {
        asr.install.lock().unwrap()
    }

    #[test]
    fn an_install_needs_room_for_the_archive_and_the_model_at_once() {
        // Issue #41: staging holds the archive and the unpacked model together, so
        // the peak is their sum — and a device without it used to find out by
        // failing a write part-way through 163 MB.
        const { assert!(INSTALL_NEEDS_BYTES >= MODEL.archive_bytes + MODEL.unpacked_bytes) };
        assert!(room_for_install(Some(INSTALL_NEEDS_BYTES)).is_ok());
        assert!(room_for_install(Some(u64::MAX)).is_ok());
        let refused = room_for_install(Some(100_000_000)).unwrap_err();
        assert!(
            refused.contains("100 MB"),
            "it says what is free: {refused}"
        );
        assert!(
            refused.contains(&megabytes(INSTALL_NEEDS_BYTES)),
            "and what is needed: {refused}"
        );
        // A platform that cannot say is not a reason to refuse.
        assert!(room_for_install(None).is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn free_space_is_read_from_the_filesystem() {
        let free = free_bytes(&std::env::temp_dir()).expect("statvfs answers for a real directory");
        assert!(free > 0);
        assert_eq!(free_bytes(Path::new("/no/such/directory/anywhere")), None);
    }

    #[test]
    fn the_model_lives_in_the_cache_directory_when_there_is_one() {
        // Issue #41: on iOS the data directory is Library/Application Support,
        // which iCloud backs up, and 240 MB of re-downloadable weights do not
        // belong in a backup (App Review 2.23). The cache directory is excluded.
        let base = std::env::temp_dir().join("hanzi-asr-home");
        let _ = fs::remove_dir_all(&base);
        let data = base.join("data");
        let cache = base.join("cache");

        assert_eq!(model_home(None, None), None);
        assert_eq!(
            model_home(Some(&data), None),
            Some(data.clone()),
            "no cache: as before"
        );
        assert_eq!(model_home(None, Some(&cache)), Some(cache.clone()));
        assert_eq!(model_home(Some(&data), Some(&cache)), Some(cache.clone()));

        // A model an earlier build put in the data directory is moved, not
        // fetched again.
        let old = data.join("asr").join(MODEL.stem);
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join(MODEL_FILE), b"model").unwrap();
        fs::write(old.join(TOKENS_FILE), b"tokens").unwrap();
        let _ = fs::remove_dir_all(&cache);
        assert_eq!(model_home(Some(&data), Some(&cache)), Some(cache.clone()));
        assert!(
            !data.join("asr").exists(),
            "moved out of the backed-up directory"
        );
        assert!(Asr::new(Some(&cache)).status().installed);

        // One already in the cache is not overwritten by an older copy — and the
        // older copy goes, since it is up to 240 MB of the same thing in the backup.
        fs::create_dir_all(data.join("asr")).unwrap();
        fs::write(data.join("asr").join("stale"), b"old").unwrap();
        assert_eq!(model_home(Some(&data), Some(&cache)), Some(cache.clone()));
        assert!(Asr::new(Some(&cache)).status().installed);
        assert!(
            !data.join("asr").exists(),
            "a complete install in the cache makes the backed-up copy redundant"
        );

        // But not when what the cache holds is not an install: then the old copy
        // may be the only working one, and it stays.
        fs::remove_file(cache.join("asr").join(MODEL.stem).join(MODEL_FILE)).unwrap();
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join(MODEL_FILE), b"model").unwrap();
        assert_eq!(model_home(Some(&data), Some(&cache)), Some(cache.clone()));
        assert!(old.join(MODEL_FILE).is_file());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_model_on_another_filesystem_is_copied_then_moved_into_place() {
        // Issue #41: `rename` cannot cross filesystems, and a cache directory on
        // another volume would otherwise leave the model in the backed-up data
        // directory for good, with the move retried on every launch. The copy goes
        // to a sibling first, so a copy cut short is never read as an install.
        let base = std::env::temp_dir().join("hanzi-asr-exdev");
        let _ = fs::remove_dir_all(&base);
        let old = base.join("data").join("asr");
        let new = base.join("cache").join("asr");
        fs::create_dir_all(old.join(MODEL.stem)).unwrap();
        fs::write(old.join(MODEL.stem).join(MODEL_FILE), b"model").unwrap();
        fs::write(old.join(MODEL.stem).join(TOKENS_FILE), b"tokens").unwrap();
        fs::create_dir_all(new.parent().unwrap()).unwrap();

        let across_devices = |from: &Path, to: &Path| {
            if from == old {
                Err(std::io::Error::from(std::io::ErrorKind::CrossesDevices))
            } else {
                fs::rename(from, to)
            }
        };
        move_model(&old, &new, &across_devices).unwrap();
        assert!(!old.exists(), "the backed-up copy is gone");
        assert_eq!(
            fs::read(new.join(MODEL.stem).join(MODEL_FILE)).unwrap(),
            b"model"
        );
        assert_eq!(
            fs::read(new.join(MODEL.stem).join(TOKENS_FILE)).unwrap(),
            b"tokens"
        );
        let leftovers: Vec<_> = fs::read_dir(new.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(
            leftovers,
            vec![std::ffi::OsString::from("asr")],
            "no temporary left"
        );

        // Any other failure is reported and leaves both sides as they were.
        fs::create_dir_all(&old).unwrap();
        let refused = |_: &Path, _: &Path| Err(std::io::Error::other("no"));
        let _ = fs::remove_dir_all(&new);
        assert!(move_model(&old, &new, &refused).is_err());
        assert!(old.is_dir() && !new.exists());

        let _ = fs::remove_dir_all(&base);
    }

    /// A source that fails the way a library bug or a corrupt stream can: by
    /// panicking on the install thread.
    struct Panics;

    impl Read for Panics {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            panic!("the decoder fell over");
        }
    }

    #[test]
    fn a_panic_on_the_install_thread_ends_the_download_as_a_failure() {
        // Without this the panic unwound past the code that records the outcome,
        // the state stayed "downloading" for the life of the process, and the
        // screen polled — and showed Cancelling… — forever.
        let directory = std::env::temp_dir().join("hanzi-asr-panic");
        let _ = fs::remove_dir_all(&directory);
        let asr = Asr::new(Some(&directory));
        let root = asr.root.clone().unwrap();
        let source = || -> Result<Box<dyn Read>, String> { Ok(Box::new(Panics)) };
        let outcome = fetch_and_unpack_from(&root, &asr.install, &source);
        let message = outcome.expect_err("a panic is a failed install");
        assert!(message.contains("the decoder fell over"), "{message}");
        assert!(
            !root.join(".staging").exists(),
            "and it is cleaned up like one"
        );

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_free_space_check_only_advises_where_the_count_leaves_out_purgeable_space() {
        // On Apple platforms `f_bavail` does not count what the system would purge
        // to make room, so a phone that could fit the model can look full. There the
        // shortfall is logged and the install goes ahead; elsewhere it is refused.
        assert!(enforce_room(Some(1), false).is_err());
        assert!(enforce_room(Some(1), true).is_ok());
        assert!(enforce_room(Some(u64::MAX), false).is_ok());
        assert!(enforce_room(None, false).is_ok());
        assert_eq!(
            ROOM_IS_ADVISORY,
            cfg!(any(target_os = "ios", target_os = "macos"))
        );
    }

    #[test]
    fn the_model_is_not_removed_from_under_a_download() {
        let directory = std::env::temp_dir().join("hanzi-asr-remove-busy");
        let _ = fs::remove_dir_all(&directory);
        let stem = directory.join("asr").join(MODEL.stem);
        fs::create_dir_all(&stem).unwrap();
        fs::write(stem.join(MODEL_FILE), b"model").unwrap();
        fs::write(stem.join(TOKENS_FILE), b"tokens").unwrap();
        let asr = Asr::new(Some(&directory));
        hold_install(&asr).state = InstallState::Downloading;

        let refused = asr.remove().expect_err("a download is running");
        assert!(
            refused.contains("Cancel"),
            "it says what to do instead: {refused}"
        );
        assert!(stem.join(MODEL_FILE).is_file(), "nothing was deleted");
        assert_eq!(hold_install(&asr).state, InstallState::Downloading);

        hold_install(&asr).state = InstallState::Absent;
        asr.remove().unwrap();
        assert!(!stem.exists());

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn having_nowhere_to_keep_the_model_is_worded_for_either_app() {
        // Tone Trainer keeps no study data, and its model lives in the cache
        // directory, so a sentence about study data not being saved was wrong
        // there.
        let asr = Asr::new(None);
        let detail = asr.status().detail;
        let refused = asr.install().unwrap_err();
        for text in [&detail, &refused] {
            assert!(!text.contains("study data"), "{text}");
            assert!(!text.contains("data directory"), "{text}");
            assert!(text.contains("nowhere to keep the model"), "{text}");
        }
    }
}
