//! Microphone capture, for tone practice.
//!
//! ## Why capture lives here and not in the webview
//!
//! The samples end up in `hanzi-core`'s pitch tracker, which is Rust. Capturing
//! in Rust keeps them on the same side of the boundary as the code that reads
//! them, with no trip through IPC per audio buffer. It is also the more
//! predictable of the two options on macOS: WKWebView's `getUserMedia` has a
//! history of not raising the permission prompt at all (the research cites wry
//! #1195 and tauri #11951), whereas a Rust capture path is a plain
//! `NSMicrophoneUsageDescription` plus an entitlement.
//!
//! ## The microphone is only open while the learner is holding the button
//!
//! [`Recorder::start`] builds a device stream and [`Recorder::stop`] drops it.
//! That is more expensive than keeping one open — a stream costs a few tens of
//! milliseconds to start — but it means this application never holds the
//! microphone between utterances, so the operating system's recording indicator
//! is lit only when the learner has deliberately pressed something. For an app
//! whose entire pitch is that it is private and offline, that trade is worth the
//! milliseconds, and push-to-talk gives the learner time to press before
//! speaking anyway.
//!
//! ## The stream never leaves its thread
//!
//! `cpal::Stream` is not `Send` on every backend, so it cannot be stored in
//! Tauri's shared state. Each recording owns a thread that builds the stream,
//! plays it, and parks until it is told to stop; only plain data crosses back
//! out. Same shape as the iOS speech backend in `speech.rs`, and for the same
//! reason.
//!
//! ## Two backends, because `cpal`'s Android input does not work
//!
//! Everywhere but Android this is `cpal`, which is what the paragraph above
//! describes. **On Android it is Kotlin's `AudioRecord`, over the platform
//! bridge**, because `cpal`'s AAudio input opens a stream, reports `AAUDIO_OK`,
//! reaches `Started`, and then never calls its data callback at all — no samples,
//! no error, on an emulator and on a phone alike. `HANDOVER.md` §9 has the log
//! and the five-source probe that ruled out the device. `AudioRecord` was probed
//! on the same two devices and delivers exactly the samples asked of it, so this
//! is not a workaround for broken hardware; it is the backend Android apps use.
//!
//! Android's samples do not come back through the IPC boundary the way
//! `speech.rs`'s voice list does — ten seconds of 16 kHz mono is 320 kB, and the
//! bridge carries JSON. The Kotlin side writes them to a scratch file in the
//! app's own cache directory and Rust reads that file, which costs nothing and
//! avoids a megabyte of JSON per utterance.
//!
//! The rate there is 16 kHz, which is `hanzi_core::tone::TARGET_SAMPLE_RATE`
//! anyway, so the Android path skips the resampling the `cpal` path needs rather
//! than adding a step.

// The process backend, and the process types it needs.
#[cfg(not(target_os = "android"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_os = "android"))]
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
#[cfg(not(target_os = "android"))]
use std::thread::JoinHandle;
#[cfg(not(target_os = "android"))]
use std::time::{Duration, Instant};

#[cfg(not(target_os = "android"))]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(target_os = "ios")]
use objc2_avf_audio::{
    AVAudioSession, AVAudioSessionCategoryOptions, AVAudioSessionCategoryPlayAndRecord,
    AVAudioSessionModeMeasurement, AVAudioSessionSetActiveOptions,
};
use serde::Serialize;
// `Arc` belongs to the `cpal` backend, which shares a sample buffer with its
// audio callback. Android's has nothing to share — its samples are Kotlin's
// until they are written to a file.
#[cfg(not(target_os = "android"))]
use std::sync::Arc;
use std::sync::{Mutex, PoisonError};

/// Longest single recording, in seconds.
///
/// A push-to-talk button that is never released, or a stuck key, would otherwise
/// grow the buffer until the process died. Ten seconds is several times the
/// longest syllable this feature scores, so hitting the cap means something has
/// gone wrong rather than that the learner was slow.
///
/// **The cap closes the device**, not only the buffer (issue #27): a stream that
/// ran on after it, recording nothing, kept the system's microphone indicator lit
/// for as long as the button stayed down — or for good, if the release was lost.
/// What was captured up to the cap is kept for [`Recorder::stop`], which still
/// judges it and says it was cut short.
pub const MAX_RECORD_SECS: u32 = 10;

/// How long to wait for the device to open before giving up on it. Opening a
/// microphone can block behind a permission dialog, and a command that hangs
/// forever leaves the interface with a button that never comes back.
#[cfg(not(target_os = "android"))]
const OPEN_TIMEOUT: Duration = Duration::from_secs(8);

/// How often the capture thread looks at whether the cap has been reached.
///
/// The audio callback only sets a flag at the cap — it runs on the device's
/// real-time thread, where waking another thread is not something to do — so the
/// capture thread looks. A twentieth of a second past the cap is nothing to a
/// learner who has held the button for ten.
#[cfg(not(target_os = "android"))]
const CAP_POLL: Duration = Duration::from_millis(50);

/// The longest the device is ever held open, whatever else happens.
///
/// The cap closes it as soon as the buffer is full, but that needs audio to
/// arrive: a device that delivers no callbacks, or a sample format this build
/// cannot read, never fills the buffer. A second past the cap, the device goes
/// regardless — no recording this app wants is that long.
#[cfg(not(target_os = "android"))]
const HOLD_BACKSTOP: Duration = Duration::from_secs(MAX_RECORD_SECS as u64 + 1);

/// What the microphone can do, so the interface can decide whether to offer the
/// control at all rather than offering one that cannot work.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrophoneStatus {
    pub available: bool,
    /// The device's own name, when there is one.
    pub device: Option<String>,
    /// The rate capture will run at. `0` when there is no device.
    pub sample_rate: u32,
    /// One plain sentence: either what will be used, or why nothing can be.
    pub detail: String,
}

/// One recording, ready to be analysed.
#[derive(Debug)]
pub struct Recording {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub device: String,
    /// True when the cap was reached and the tail of the utterance is missing.
    pub truncated: bool,
}

/// A recording in progress, on the platforms whose capture is a `cpal` stream:
/// the buffer being filled, and the way to stop it.
#[cfg(not(target_os = "android"))]
struct Active {
    samples: Arc<Mutex<Vec<f32>>>,
    failure: Arc<Mutex<Option<String>>>,
    truncation: Arc<Mutex<Truncation>>,
    stop: Sender<()>,
    thread: JoinHandle<()>,
    sample_rate: u32,
    device: String,
}

/// The truncation flag, shared with the audio callback.
///
/// A cell of its own rather than a field on the buffer's mutex, so that the
/// callback can set it without taking the lock that the stop path is about to
/// hold while it copies several hundred thousand samples out.
#[cfg(not(target_os = "android"))]
#[derive(Default)]
struct Truncation(bool);

/// A recording in progress on Android: there is nothing to hold but the facts
/// to report, because the buffer is Kotlin's and the samples are on disk.
#[cfg(target_os = "android")]
struct Active {
    sample_rate: u32,
    device: String,
}

/// The microphone, as the app holds it.
#[derive(Default)]
pub struct Recorder {
    active: Mutex<Option<Active>>,
}

/// What `PlatformPlugin.recordStart` answers with.
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordStarted {
    sample_rate: u32,
    source: String,
}

/// What `PlatformPlugin.recordStop` answers with.
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordStopped {
    /// Where the samples were written: PCM16 little-endian, mono.
    path: String,
    sample_rate: u32,
}

/// What `PlatformPlugin.recordStatus` answers with.
#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordStatus {
    available: bool,
    sample_rate: u32,
    source: String,
    /// Whether a recording is running *now*, which is not the same question as
    /// whether the microphone works.
    recording: bool,
    /// Why not, when `available` is false.
    problem: String,
}

impl Recorder {
    /// Whether capture is possible, and at what rate.
    ///
    /// This cannot see a *denied permission*: macOS still hands out a device
    /// when the user has said no, and the stream then delivers silence. That is
    /// why the detail says so rather than claiming the microphone works, and why
    /// "I could not hear enough voice" from the analyser is the symptom to
    /// expect in that case.
    ///
    /// Android answers the same question of `AudioRecord`, which is what actually
    /// captures there. Asking `cpal` instead would report a working microphone on
    /// a phone where every recording came back empty, and that is precisely the
    /// confusion this backend was written to end.
    #[cfg(not(target_os = "android"))]
    pub fn status(&self) -> MicrophoneStatus {
        let Some((device, config)) = open_default_device() else {
            return MicrophoneStatus {
                available: false,
                device: None,
                sample_rate: 0,
                detail: "No microphone was found, so speaking cannot be practised.".into(),
            };
        };
        let name = device.to_string();
        let rate = config.sample_rate();
        MicrophoneStatus {
            available: true,
            device: Some(name.clone()),
            sample_rate: rate,
            detail: format!(
                "Listening on {name} at {rate} Hz while the button is held. If the system has \
                 been told to deny microphone access, this records silence rather than failing."
            ),
        }
    }

    /// Whether capture is possible, and at what rate.
    ///
    /// See the `cpal` version above for what a "denied permission" looks like;
    /// the same caveat applies here, and on Android the permission is a runtime
    /// grant the app asks for on first launch.
    #[cfg(target_os = "android")]
    pub fn status(&self) -> MicrophoneStatus {
        let report: RecordStatus = match crate::platform::call("recordStatus", ()) {
            Ok(report) => report,
            Err(message) => {
                return MicrophoneStatus {
                    available: false,
                    device: None,
                    sample_rate: 0,
                    detail: format!("The microphone could not be asked about: {message}"),
                }
            }
        };
        if !report.available {
            return MicrophoneStatus {
                available: false,
                device: None,
                sample_rate: 0,
                detail: report.problem,
            };
        }
        let name = report.source;
        let rate = report.sample_rate;
        MicrophoneStatus {
            available: true,
            device: Some(name.clone()),
            sample_rate: rate,
            detail: format!(
                "Listening through {name} at {rate} Hz while the button is held. If the system \
                 has been told to deny microphone access, this records silence rather than \
                 failing."
            ),
        }
    }

    /// Begin recording. Returns once the device is actually open and running.
    ///
    /// On iOS the audio session is taken for recording first, and given back
    /// again if no stream came of it — see [`engage_input_session`].
    #[cfg(not(target_os = "android"))]
    pub fn start(&self) -> Result<(), String> {
        #[cfg(target_os = "ios")]
        engage_input_session()?;

        let opened = self.open_stream();

        // A recording that never started must not leave the session taken: it
        // would hold the microphone indicator lit, and the next utterance's
        // speech would be the one that has to fight for it.
        #[cfg(target_os = "ios")]
        if opened.is_err() {
            release_input_session();
        }

        opened
    }

    /// Open the device and start the stream. See [`Recorder::start`].
    #[cfg(not(target_os = "android"))]
    fn open_stream(&self) -> Result<(), String> {
        let mut active = self.active.lock().unwrap_or_else(PoisonError::into_inner);
        // A recording whose device closed itself at the cap, and that nobody came
        // back to stop. The interface only presses with nothing of its own in
        // flight, so this one is orphaned — a webview that reloaded mid-press —
        // and refusing every later press with "Already listening." for it would
        // be the stale flag the Android backend already learned not to trust.
        if let Some(over) = active.take_if(|recording| recording.thread.is_finished()) {
            let _ = over.thread.join();
        }
        if active.is_some() {
            return Err("Already listening.".into());
        }

        let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let failure: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let truncation: Arc<Mutex<Truncation>> = Arc::new(Mutex::new(Truncation::default()));

        let (ready_tx, ready_rx) = mpsc::channel::<Result<(u32, String), String>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let thread = {
            let samples = Arc::clone(&samples);
            let failure = Arc::clone(&failure);
            let truncation = Arc::clone(&truncation);
            std::thread::spawn(move || {
                capture_thread(samples, failure, truncation, ready_tx, stop_rx);
            })
        };

        // Wait for the device to open, so a failure is reported to the caller of
        // `start` rather than swallowed in a thread nobody reads.
        match ready_rx.recv_timeout(OPEN_TIMEOUT) {
            Ok(Ok((sample_rate, device))) => {
                *active = Some(Active {
                    samples,
                    failure,
                    truncation,
                    stop: stop_tx,
                    thread,
                    sample_rate,
                    device,
                });
                Ok(())
            }
            Ok(Err(message)) => {
                let _ = thread.join();
                Err(message)
            }
            Err(RecvTimeoutError::Timeout) => {
                // The thread is still blocked opening the device, most likely on
                // a permission prompt. Let it finish on its own; it owns no
                // state this process needs back. If the device does open later —
                // the prompt answered after the timeout — nothing here will ever
                // stop it, so it is told now: the stop waits in the channel, and
                // the thread also finds nobody listening for its "ready" and lets
                // the stream go (see `report_and_hold`).
                let _ = stop_tx.send(());
                Err(format!(
                    "The microphone did not open within {} seconds.",
                    OPEN_TIMEOUT.as_secs()
                ))
            }
            Err(RecvTimeoutError::Disconnected) => {
                let _ = thread.join();
                Err("The microphone thread stopped before it started.".into())
            }
        }
    }

    /// Stop recording and hand back what was captured.
    ///
    /// A recording whose device already closed at the cap is still here to be
    /// stopped, with `truncated` set: the learner held on too long, and is judged on
    /// the first [`MAX_RECORD_SECS`] rather than told nothing was being recorded.
    #[cfg(not(target_os = "android"))]
    pub fn stop(&self) -> Result<Recording, String> {
        let active = {
            let mut slot = self.active.lock().unwrap_or_else(PoisonError::into_inner);
            slot.take().ok_or("Not listening.")?
        };

        // Ignore a send failure: the thread may already have exited because the
        // cap closed the device, and the samples it did capture are still worth
        // having.
        let _ = active.stop.send(());
        let _ = active.thread.join();

        // The stream is gone, so the session can go back to whatever it was
        // taken from. Same bargain as the speech path: nothing is held while the
        // learner is not actually speaking.
        #[cfg(target_os = "ios")]
        release_input_session();

        let samples = std::mem::take(
            &mut *active
                .samples
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        if let Some(message) = active
            .failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            return Err(message);
        }
        let truncated = active
            .truncation
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .0;

        Ok(Recording {
            samples,
            sample_rate: active.sample_rate,
            device: active.device,
            truncated,
        })
    }

    /// Begin recording, through Kotlin's `AudioRecord`.
    ///
    /// The Kotlin side does the work on a thread of its own and answers as soon
    /// as the device is running, so this returns with the microphone already
    /// open — the same promise the `cpal` version makes.
    ///
    /// The platform is asked which of the two is recording, because this side's
    /// own record of it can go stale: Kotlin's audio thread ends by itself when
    /// it hits the cap or fails, and nothing tells Rust when it does. Trusting
    /// the local flag alone left the app refusing every later press with
    /// "Already listening." for the life of the process — reachable by holding
    /// the button past the cap once.
    #[cfg(target_os = "android")]
    pub fn start(&self) -> Result<(), String> {
        let mut active = self.active.lock().unwrap_or_else(PoisonError::into_inner);
        if active.is_some() {
            let live = crate::platform::call::<RecordStatus>("recordStatus", ())
                .map(|report| report.recording)
                .unwrap_or(false);
            if live {
                return Err("Already listening.".into());
            }
            // A recording that ended without this side being told. Let it go.
            *active = None;
        }
        let started: RecordStarted = crate::platform::call("recordStart", ())?;
        *active = Some(Active {
            sample_rate: started.sample_rate,
            device: started.source,
        });
        Ok(())
    }

    /// Stop recording and read back what Kotlin wrote.
    ///
    /// The file is PCM16 little-endian and mono, at the rate the Kotlin side
    /// reported. It is deleted here rather than left for the next recording to
    /// overwrite: ten seconds of speech is not something to keep lying around in
    /// an app whose whole claim is that it stores nothing it does not have to.
    #[cfg(target_os = "android")]
    pub fn stop(&self) -> Result<Recording, String> {
        let active = {
            let mut slot = self.active.lock().unwrap_or_else(PoisonError::into_inner);
            slot.take().ok_or("Not listening.")?
        };

        let stopped: RecordStopped = crate::platform::call("recordStop", ())?;
        let bytes = std::fs::read(&stopped.path)
            .map_err(|error| format!("could not read the recording: {error}"))?;
        let _ = std::fs::remove_file(&stopped.path);

        let rate = if stopped.sample_rate == 0 {
            active.sample_rate
        } else {
            stopped.sample_rate
        };
        let limit = rate as usize * MAX_RECORD_SECS as usize;
        let mut samples: Vec<f32> = bytes
            .chunks_exact(2)
            // 32768 rather than 32767: it is the exact scale of the negative
            // half, so a full-scale sample lands on -1.0 rather than slightly
            // past it, and the sign is symmetric.
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]) as f32 / 32_768.0)
            .collect();
        let truncated = samples.len() > limit;
        if truncated {
            samples.truncate(limit);
        }

        Ok(Recording {
            samples,
            sample_rate: rate,
            device: active.device,
            truncated,
        })
    }
}

/// The default input device and its configuration, or nothing.
#[cfg(not(target_os = "android"))]
fn open_default_device() -> Option<(cpal::Device, cpal::SupportedStreamConfig)> {
    let host = cpal::default_host();
    let device = host.default_input_device()?;
    let config = device.default_input_config().ok()?;
    Some((device, config))
}

/// Take the audio session for recording, so there is an input to open.
///
/// iOS answers "how many input channels does this session have?" from the
/// session's *category*, and the one this app shares with its speech is
/// `playback` — which has no input at all. `cpal` asks exactly that question,
/// is told zero, and then refuses to build the stream with **"channel count
/// must be at least 1"**, which reads like a broken microphone rather than a
/// category that was never told this app also listens. So a recording takes the
/// session as `playAndRecord` first, and [`release_input_session`] gives it back
/// when the stream is gone. This is the mirror of `speech.rs`'s
/// `engage_session`/`release_session`, and it has to happen *before* the device
/// is asked for its configuration, which is why it is not inside the capture
/// thread.
///
/// `measurement` mode is iOS's counterpart of the `VOICE_RECOGNITION` source
/// Android records from: the pitch tracker wants the signal, not the system's
/// idea of a telephone call, and it is what keeps the input gain from being
/// moved under a learner trying to hold one steady note. `defaultToSpeaker`
/// keeps the pronunciation button on the loudspeaker rather than the earpiece,
/// which is what `playback` did before this took the session, and
/// `allowBluetoothHFP` lets a headset's own microphone be the one that hears.
#[cfg(target_os = "ios")]
fn engage_input_session() -> Result<(), String> {
    crate::speech::with_main(|| {
        // SAFETY: on the main thread, and the session is a process-wide
        // singleton that outlives this call.
        unsafe {
            let session = AVAudioSession::sharedInstance();
            session
                .setCategory_mode_options_error(
                    AVAudioSessionCategoryPlayAndRecord.expect("declared by AVFAudio"),
                    AVAudioSessionModeMeasurement.expect("declared by AVFAudio"),
                    AVAudioSessionCategoryOptions::DefaultToSpeaker
                        | AVAudioSessionCategoryOptions::AllowBluetoothHFP,
                )
                .map_err(|error| error.localizedDescription().to_string())?;
            session
                .setActive_error(true)
                .map_err(|error| error.localizedDescription().to_string())
        }
    })
}

/// Give the audio session back, so whatever was paused for the recording resumes.
#[cfg(target_os = "ios")]
fn release_input_session() {
    crate::speech::with_main(|| {
        // SAFETY: as above.
        unsafe {
            let session = AVAudioSession::sharedInstance();
            let _ = session.setActive_withOptions_error(
                false,
                AVAudioSessionSetActiveOptions::NotifyOthersOnDeactivation,
            );
        }
    });
}

/// Where the audio callback puts what it hears, up to the cap.
///
/// Its own type so that what happens at the cap can be tested without a
/// microphone: the buffer stops growing, and the truncation and `full` flags are
/// set, which the capture thread sees and lets the stream go.
#[cfg(not(target_os = "android"))]
struct Sink {
    samples: Arc<Mutex<Vec<f32>>>,
    truncation: Arc<Mutex<Truncation>>,
    channels: usize,
    limit: usize,
    /// Set at the cap; read by [`hold`]. A flag rather than a message, because
    /// this runs on the device's real-time thread.
    full: Arc<AtomicBool>,
}

#[cfg(not(target_os = "android"))]
impl Sink {
    /// One callback's worth of audio.
    fn accept(&mut self, data: &cpal::Data) {
        // The lock is held only for the copy. The stop path takes it once, after
        // the stream has been dropped, so there is no contention to speak of.
        let Ok(mut out) = self.samples.lock() else {
            return;
        };
        if out.len() < self.limit {
            append_mono(data, self.channels, &mut out, self.limit);
        }
        if out.len() < self.limit {
            return;
        }
        drop(out);
        if let Ok(mut flag) = self.truncation.lock() {
            flag.0 = true;
        }
        self.full.store(true, Ordering::Release);
    }
}

/// Tell `start` the device is open, then keep the stream alive until it should go.
///
/// If nobody is waiting for the "ready" any more, `start` has already given up on
/// this device — it took longer than [`OPEN_TIMEOUT`] to open, behind a permission
/// prompt answered late — and there is no [`Active`] through which it could ever
/// be stopped, so the stream goes at once rather than recording into nothing with
/// the indicator lit.
#[cfg(not(target_os = "android"))]
fn report_and_hold<S>(
    stream: S,
    ready: &Sender<Result<(u32, String), String>>,
    opened: (u32, String),
    stop: &mpsc::Receiver<()>,
    full: &AtomicBool,
    backstop: Duration,
) {
    if ready.send(Ok(opened)).is_err() {
        drop(stream);
        return;
    }
    hold(stream, stop, full, backstop);
}

/// Keep the stream alive until a stop, the cap, or the backstop says to let it go.
///
/// Dropping the stream is what closes the device and puts the system's recording
/// indicator out. A stop, a disconnected channel (the [`Active`] dropped without
/// one), the sink's `full` flag and `backstop` after opening all end it. Generic
/// so a test can stand a stream in.
#[cfg(not(target_os = "android"))]
fn hold<S>(stream: S, stop: &mpsc::Receiver<()>, full: &AtomicBool, backstop: Duration) {
    let opened = Instant::now();
    loop {
        match stop.recv_timeout(CAP_POLL) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                if full.load(Ordering::Acquire) || opened.elapsed() >= backstop {
                    break;
                }
            }
        }
    }
    drop(stream);
}

/// Runs on its own thread for the life of one recording.
#[cfg(not(target_os = "android"))]
fn capture_thread(
    samples: Arc<Mutex<Vec<f32>>>,
    failure: Arc<Mutex<Option<String>>>,
    truncation: Arc<Mutex<Truncation>>,
    ready: Sender<Result<(u32, String), String>>,
    stop: mpsc::Receiver<()>,
) {
    let host = cpal::default_host();
    let Some(device) = host.default_input_device() else {
        let _ = ready.send(Err("No microphone was found.".into()));
        return;
    };
    let name = device.to_string();
    let supported = match device.default_input_config() {
        Ok(config) => config,
        Err(err) => {
            let _ = ready.send(Err(format!("{name} has no usable input configuration: {err}")));
            return;
        }
    };

    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let sample_rate = config.sample_rate;
    let channels = config.channels as usize;
    let limit = sample_rate as usize * MAX_RECORD_SECS as usize;

    let full = Arc::new(AtomicBool::new(false));
    let mut sink = Sink {
        samples,
        truncation,
        channels,
        limit,
        full: Arc::clone(&full),
    };
    let data_error = Arc::clone(&failure);
    let data_callback = move |data: &cpal::Data, _: &cpal::InputCallbackInfo| sink.accept(data);

    let error_callback = move |err: cpal::Error| {
        // A changed route is not the end of a recording, and on iOS *taking* the
        // session for one is itself a route change: the category moves from
        // `playback` to `playAndRecord` and the output from the receiver to the
        // loudspeaker, so the first thing the new stream is told about is this
        // process's own doing. Reported verbatim, that read as "the microphone
        // stopped: Audio route changed" and threw away an utterance that was
        // being captured perfectly well. `cpal` documents `DeviceChanged` as
        // "the stream remains active and no rebuild is required", and its iOS
        // backend only refreshes its latency estimate for either kind, so both
        // are logged and the recording carries on. A session with no route at
        // all is a different matter and still stops it.
        #[cfg(target_os = "ios")]
        if matches!(
            err.kind(),
            cpal::ErrorKind::DeviceChanged | cpal::ErrorKind::StreamInvalidated
        ) {
            eprintln!("[capture] the audio route changed while listening: {err}");
            return;
        }
        if let Ok(mut slot) = data_error.lock() {
            // Keep the first error: later ones are usually consequences of it.
            if slot.is_none() {
                *slot = Some(format!("The microphone stopped: {err}"));
            }
        }
    };

    let stream = match device.build_input_stream_raw(
        config,
        sample_format,
        data_callback,
        error_callback,
        None,
    ) {
        Ok(stream) => stream,
        Err(err) => {
            let _ = ready.send(Err(format!("Could not open {name}: {err}")));
            return;
        }
    };
    if let Err(err) = stream.play() {
        let _ = ready.send(Err(format!("Could not start listening on {name}: {err}")));
        return;
    }

    // Park until told to stop, by the learner or by the cap.
    report_and_hold(
        stream,
        &ready,
        (sample_rate, name),
        &stop,
        &full,
        HOLD_BACKSTOP,
    );
}

/// Downmix one callback's worth of samples to mono and append them.
///
/// Every channel format cpal can hand over is converted rather than only `f32`:
/// a device that offers nothing but `i16` is common on Windows, and refusing it
/// would mean the feature simply does not work on that machine.
///
/// Each format is scaled by its own full scale rather than run through a generic
/// sample-conversion trait, so that the mapping is visible in one place and a
/// signed type's negative range cannot be silently halved. `u8` and the wider
/// unsigned types are centred first: they carry silence at mid-scale, not at
/// zero, and treating them as signed would clip every waveform in half.
#[cfg(not(target_os = "android"))]
fn append_mono(data: &cpal::Data, channels: usize, out: &mut Vec<f32>, limit: usize) {
    use cpal::SampleFormat as F;

    fn extend<T: cpal::SizedSample + Copy>(
        data: &cpal::Data,
        channels: usize,
        out: &mut Vec<f32>,
        limit: usize,
        to_f32: impl Fn(T) -> f32,
    ) {
        let Some(samples) = data.as_slice::<T>() else {
            return;
        };
        let channels = channels.max(1);
        for frame in samples.chunks(channels) {
            if out.len() >= limit {
                return;
            }
            let sum: f32 = frame.iter().map(|s| to_f32(*s)).sum();
            out.push(sum / channels as f32);
        }
    }

    match data.sample_format() {
        F::I8 => extend(data, channels, out, limit, |s: i8| s as f32 / i8::MAX as f32),
        F::I16 => extend(data, channels, out, limit, |s: i16| s as f32 / i16::MAX as f32),
        F::I32 => extend(data, channels, out, limit, |s: i32| s as f32 / i32::MAX as f32),
        F::I64 => extend(data, channels, out, limit, |s: i64| s as f32 / i64::MAX as f32),
        F::U8 => extend(data, channels, out, limit, |s: u8| {
            (s as f32 - 128.0) / 128.0
        }),
        F::U16 => extend(data, channels, out, limit, |s: u16| {
            (s as f32 - 32_768.0) / 32_768.0
        }),
        F::U32 => extend(data, channels, out, limit, |s: u32| {
            (s as f64 as f32 - 2_147_483_648.0) / 2_147_483_648.0
        }),
        F::U64 => extend(data, channels, out, limit, |s: u64| {
            ((s as f64 - 9_223_372_036_854_775_808.0) / 9_223_372_036_854_775_808.0) as f32
        }),
        F::F32 => extend(data, channels, out, limit, |s: f32| s),
        F::F64 => extend(data, channels, out, limit, |s: f64| s as f32),
        // `SampleFormat` is non-exhaustive. An unknown format is a device this
        // build cannot read, and the honest outcome is an empty recording that
        // the analyser then refuses to score — not noise scored as speech.
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_is_honest_when_there_is_no_microphone() {
        // Whatever this machine has, the answer must be one of the two shapes
        // and must never claim a rate it does not have.
        let status = Recorder::default().status();
        if status.available {
            assert!(status.device.is_some());
            assert!(status.sample_rate > 0);
        } else {
            assert!(status.device.is_none());
            assert_eq!(status.sample_rate, 0);
            assert!(status.detail.contains("No microphone"));
        }
    }

    #[test]
    fn stopping_without_starting_is_an_error_not_a_panic() {
        let recorder = Recorder::default();
        assert_eq!(recorder.stop().unwrap_err(), "Not listening.");
    }

    #[test]
    fn a_slot_left_poisoned_by_a_panic_does_not_panic_every_later_press() {
        // The microphone commands run on worker threads (issue #26), where a panic
        // leaves the app running and the slot poisoned — and every later press
        // would then panic on it, for the life of the app.
        let recorder = Recorder::default();
        std::thread::scope(|scope| {
            let _ = scope
                .spawn(|| {
                    let _held = recorder.active.lock().unwrap();
                    panic!("poison the slot");
                })
                .join();
        });
        assert!(recorder.active.is_poisoned());
        assert_eq!(recorder.stop().unwrap_err(), "Not listening.");
    }

    /// What the cap does to a `cpal` recording (issue #27). Android's recorder
    /// stops itself at the cap in Kotlin, and has no stream on this side.
    #[cfg(not(target_os = "android"))]
    mod cap {
        use super::*;

        /// One callback's worth of mono `f32` audio, as cpal would hand it over.
        fn callback(buffer: &mut [f32]) -> cpal::Data {
            // SAFETY: the pointer and length describe `buffer`, which is `f32` and
            // outlives every use of the `Data` in these tests.
            unsafe {
                cpal::Data::from_parts(
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    cpal::SampleFormat::F32,
                )
            }
        }

        /// Stands in for a `cpal::Stream`: says when it has been let go.
        struct Stream(Arc<AtomicBool>);

        impl Drop for Stream {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        /// A recording as `open_stream` leaves it, with a stand-in stream held by the
        /// same `hold` the capture thread uses, and the sink its callback would fill.
        fn recording(limit: usize) -> (Active, Sink, Arc<AtomicBool>) {
            let samples = Arc::new(Mutex::new(Vec::new()));
            let truncation = Arc::new(Mutex::new(Truncation::default()));
            let (stop_tx, stop_rx) = mpsc::channel();
            let closed = Arc::new(AtomicBool::new(false));
            let stream = Stream(Arc::clone(&closed));
            let full = Arc::new(AtomicBool::new(false));
            let thread = {
                let full = Arc::clone(&full);
                std::thread::spawn(move || hold(stream, &stop_rx, &full, HOLD_BACKSTOP))
            };
            let sink = Sink {
                samples: Arc::clone(&samples),
                truncation: Arc::clone(&truncation),
                channels: 1,
                limit,
                full,
            };
            let active = Active {
                samples,
                failure: Arc::new(Mutex::new(None)),
                truncation,
                stop: stop_tx,
                thread,
                sample_rate: 16_000,
                device: "a test input".into(),
            };
            (active, sink, closed)
        }

        /// Wait, briefly, for the capture thread to have let its stream go.
        fn closes(active: &Active) -> bool {
            finishes(&active.thread, Duration::from_secs(5))
        }

        /// Whether `thread` ends within `within`.
        fn finishes(thread: &JoinHandle<()>, within: Duration) -> bool {
            let deadline = Instant::now() + within;
            while !thread.is_finished() {
                if Instant::now() > deadline {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            true
        }

        #[test]
        fn a_device_that_opens_after_start_gave_up_is_let_go_at_once() {
            // The permission prompt answered after the eight-second open timeout:
            // `start` has returned its error, and nothing holds the `Active` that
            // could ever stop this. A sender still alive somewhere (the sink once
            // held one) must not keep the device open with the indicator lit.
            let (ready_tx, ready_rx) = mpsc::channel();
            drop(ready_rx);
            let (stop_tx, stop_rx) = mpsc::channel::<()>();
            let closed = Arc::new(AtomicBool::new(false));
            let stream = Stream(Arc::clone(&closed));
            let full = AtomicBool::new(false);
            let thread = std::thread::spawn(move || {
                report_and_hold(
                    stream,
                    &ready_tx,
                    (16_000, "a test input".into()),
                    &stop_rx,
                    &full,
                    Duration::from_secs(60),
                );
            });

            assert!(
                finishes(&thread, Duration::from_secs(2)),
                "the device must close as soon as nobody is waiting for it"
            );
            assert!(closed.load(Ordering::SeqCst));
            drop(stop_tx);
        }

        #[test]
        fn a_device_that_opens_in_time_is_held_until_it_is_stopped() {
            let (ready_tx, ready_rx) = mpsc::channel();
            let (stop_tx, stop_rx) = mpsc::channel::<()>();
            let closed = Arc::new(AtomicBool::new(false));
            let stream = Stream(Arc::clone(&closed));
            let full = AtomicBool::new(false);
            let thread = std::thread::spawn(move || {
                report_and_hold(
                    stream,
                    &ready_tx,
                    (16_000, "a test input".into()),
                    &stop_rx,
                    &full,
                    Duration::from_secs(60),
                );
            });

            assert_eq!(
                ready_rx.recv().unwrap(),
                Ok((16_000, "a test input".to_string()))
            );
            assert!(!finishes(&thread, Duration::from_millis(150)));
            assert!(!closed.load(Ordering::SeqCst));
            stop_tx.send(()).unwrap();
            assert!(finishes(&thread, Duration::from_secs(2)));
            assert!(closed.load(Ordering::SeqCst));
        }

        #[test]
        fn the_device_closes_by_itself_when_the_cap_is_never_reached() {
            // No callbacks, a sample format this build cannot read, a buffer whose
            // lock was poisoned: the sink never fills, and nothing stops it. The
            // backstop closes the device anyway.
            let (stop_tx, stop_rx) = mpsc::channel::<()>();
            let closed = Arc::new(AtomicBool::new(false));
            let stream = Stream(Arc::clone(&closed));
            let full = AtomicBool::new(false);
            let thread = std::thread::spawn(move || {
                hold(stream, &stop_rx, &full, Duration::from_millis(100));
            });

            assert!(
                finishes(&thread, Duration::from_secs(2)),
                "the backstop must close the device"
            );
            assert!(closed.load(Ordering::SeqCst));
            drop(stop_tx);
        }

        #[test]
        fn a_recorder_dropped_without_a_stop_lets_the_device_go() {
            let (active, _sink, closed) = recording(10);
            let thread = active.thread;
            drop(active.stop);
            assert!(finishes(&thread, Duration::from_secs(2)));
            assert!(closed.load(Ordering::SeqCst));
        }

        #[test]
        fn the_cap_closes_the_device_and_keeps_what_was_captured() {
            // Issue #27: the cap used to stop the buffer and leave the stream running,
            // so the system's microphone indicator stayed lit with nothing recorded.
            let (active, mut sink, closed) = recording(10);

            sink.accept(&callback(&mut [0.25; 6]));
            assert!(
                !active.thread.is_finished(),
                "under the cap, the device stays open"
            );
            assert!(!closed.load(Ordering::SeqCst));

            sink.accept(&callback(&mut [0.5; 6]));
            assert!(closes(&active), "reaching the cap must let the stream go");
            assert!(closed.load(Ordering::SeqCst));
            // A callback already on its way when the device closed changes nothing.
            sink.accept(&callback(&mut [0.75; 6]));

            // And the release that follows is a stop like any other: the first ten
            // samples are judged, and said to be cut short — not "Not listening.".
            let recorder = Recorder::default();
            *recorder.active.lock().unwrap() = Some(active);
            let recording = recorder
                .stop()
                .expect("a capped recording can still be stopped");
            assert_eq!(recording.samples.len(), 10);
            assert_eq!(&recording.samples[..6], &[0.25; 6]);
            assert_eq!(&recording.samples[6..], &[0.5; 4]);
            assert!(recording.truncated);
            assert_eq!(recording.sample_rate, 16_000);
            assert_eq!(recorder.stop().unwrap_err(), "Not listening.");
        }

        #[test]
        fn a_recording_stopped_before_the_cap_is_not_truncated() {
            let (active, mut sink, closed) = recording(10);
            sink.accept(&callback(&mut [0.25; 4]));

            let recorder = Recorder::default();
            *recorder.active.lock().unwrap() = Some(active);
            let recording = recorder.stop().expect("stopping should succeed");
            assert!(closed.load(Ordering::SeqCst), "a stop lets the stream go");
            assert_eq!(recording.samples, vec![0.25; 4]);
            assert!(!recording.truncated);
        }

        #[test]
        fn a_recording_closed_at_the_cap_does_not_refuse_the_next_press() {
            // Nobody came back to stop it — the webview reloaded mid-press, say. Its
            // device is already closed, so it is not a reason for "Already listening."
            let (active, mut sink, _) = recording(4);
            sink.accept(&callback(&mut [0.25; 4]));
            assert!(closes(&active));

            let recorder = Recorder::default();
            *recorder.active.lock().unwrap() = Some(active);
            match recorder.start() {
                Ok(()) => {
                    let _ = recorder.stop();
                }
                Err(message) => assert_ne!(message, "Already listening."),
            }
        }

        #[test]
        fn a_recording_still_running_refuses_the_next_press() {
            let (active, _sink, _) = recording(4);
            let recorder = Recorder::default();
            *recorder.active.lock().unwrap() = Some(active);
            assert_eq!(recorder.start().unwrap_err(), "Already listening.");
            // And is still there to be stopped.
            assert!(recorder.stop().is_ok());
        }
    }

    #[test]
    fn starting_twice_is_refused() {
        // Only meaningful where a device exists; on a machine without one the
        // first start fails for its own reason and there is nothing to assert.
        let recorder = Recorder::default();
        match recorder.start() {
            Ok(()) => {
                let second = recorder.start();
                assert!(second.is_err(), "a second start must be refused");
                let _ = recorder.stop();
            }
            Err(message) => {
                assert!(!message.is_empty(), "a failure must say why");
            }
        }
    }
}
