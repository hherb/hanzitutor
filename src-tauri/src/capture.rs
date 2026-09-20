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
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
#[cfg(not(target_os = "android"))]
use std::thread::JoinHandle;
#[cfg(not(target_os = "android"))]
use std::time::Duration;

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
use std::sync::Mutex;

/// Longest single recording, in seconds.
///
/// A push-to-talk button that is never released, or a stuck key, would otherwise
/// grow the buffer until the process died. Ten seconds is several times the
/// longest syllable this feature scores, so hitting the cap means something has
/// gone wrong rather than that the learner was slow.
pub const MAX_RECORD_SECS: u32 = 10;

/// How long to wait for the device to open before giving up on it. Opening a
/// microphone can block behind a permission dialog, and a command that hangs
/// forever leaves the interface with a button that never comes back.
#[cfg(not(target_os = "android"))]
const OPEN_TIMEOUT: Duration = Duration::from_secs(8);

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
        let mut active = self.active.lock().expect("recorder mutex");
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
                // state this process needs back.
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
    #[cfg(not(target_os = "android"))]
    pub fn stop(&self) -> Result<Recording, String> {
        let active = {
            let mut slot = self.active.lock().expect("recorder mutex");
            slot.take().ok_or("Not listening.")?
        };

        // Ignore a send failure: the thread may already have exited because the
        // device disappeared, and the samples it did capture are still worth
        // having.
        let _ = active.stop.send(());
        let _ = active.thread.join();

        // The stream is gone, so the session can go back to whatever it was
        // taken from. Same bargain as the speech path: nothing is held while the
        // learner is not actually speaking.
        #[cfg(target_os = "ios")]
        release_input_session();

        let samples = std::mem::take(&mut *active.samples.lock().expect("sample buffer"));
        if let Some(message) = active.failure.lock().expect("failure slot").take() {
            return Err(message);
        }
        let truncated = active.truncation.lock().expect("truncation slot").0;

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
        let mut active = self.active.lock().expect("recorder mutex");
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
            let mut slot = self.active.lock().expect("recorder mutex");
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

    let data_samples = Arc::clone(&samples);
    let data_truncation = Arc::clone(&truncation);
    let data_error = Arc::clone(&failure);
    let data_callback = move |data: &cpal::Data, _: &cpal::InputCallbackInfo| {
        // The lock is held only for the copy. The stop path takes it once, after
        // the stream has been dropped, so there is no contention to speak of.
        let Ok(mut out) = data_samples.lock() else {
            return;
        };
        if out.len() >= limit {
            if let Ok(mut flag) = data_truncation.lock() {
                flag.0 = true;
            }
            return;
        }
        append_mono(data, channels, &mut out, limit);
    };

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

    let _ = ready.send(Ok((sample_rate, name)));

    // Park until told to stop. A disconnected channel means the `Recorder` was
    // dropped without a stop, which is also a reason to let the stream go.
    let _ = stop.recv();
    drop(stream);
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

    /// The one test that needs real hardware.
    ///
    /// Ignored by default because it needs a microphone and, on macOS, a granted
    /// permission — neither of which a test run can arrange. Run it by hand:
    ///
    /// ```text
    /// cargo test -p hanzi-tutor --lib -- --ignored --nocapture records_from_the_real_microphone
    /// ```
    ///
    /// Say a syllable while it runs. What it cannot tell you is whether the
    /// *permission* was granted: a denied microphone still opens and still
    /// streams, and delivers silence. That case shows up as `voiced` staying at
    /// zero — which the analyser reports as "I could not hear enough voice",
    /// rather than as a wrong tone.
    #[test]
    #[ignore = "needs a microphone and permission"]
    fn records_from_the_real_microphone() {
        let recorder = Recorder::default();
        let status = recorder.status();
        println!("microphone: {status:?}");
        assert!(status.available, "no microphone available: {}", status.detail);

        recorder.start().expect("the device should open");
        std::thread::sleep(Duration::from_millis(1500));
        let recording = recorder.stop().expect("stopping should succeed");

        println!(
            "captured {} samples at {} Hz from {} (truncated: {})",
            recording.samples.len(),
            recording.sample_rate,
            recording.device,
            recording.truncated
        );
        assert!(
            !recording.samples.is_empty(),
            "the stream ran but produced no samples at all"
        );

        let loudest = recording
            .samples
            .iter()
            .fold(0.0f32, |peak, s| peak.max(s.abs()));
        println!("peak level: {loudest:.4}");
        let state = crate::state::AppState::load(None).expect("the dataset should load");
        let target = state
            .tone_target("你")
            .expect("你 has a scorable reading");
        let result = state.score_tones(&recording, &target);
        println!(
            "verdict {:?}, score {:.1}: {}",
            result.verdict, result.score, result.detail
        );
        for syllable in &result.syllables {
            println!(
                "  {} ({}, tone {}): {:?} {:.1} — {}",
                syllable.ch,
                syllable.reading,
                syllable.spoken,
                syllable.attempt.verdict,
                syllable.attempt.score,
                syllable.attempt.detail
            );
        }
    }
}
