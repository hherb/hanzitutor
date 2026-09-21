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

    /// The one test that needs real hardware, and the one that needs a person.
    ///
    /// Ignored by default because it needs a microphone and, on macOS, a granted
    /// permission — neither of which a test run can arrange. Run it by hand:
    ///
    /// ```text
    /// cargo test -p hanzi-tutor --lib -- --ignored --nocapture records_from_the_real_microphone
    /// ```
    ///
    /// **It waits for you.** It prints a prompt, and nothing is recorded until you
    /// press Enter; then it says `RECORDING` in as many words, records for a few
    /// seconds, and prints what it measured. Press Enter again for another go, or
    /// `q` to finish. The first version of this started recording the instant the
    /// process did and was over in under two seconds, which is exactly as useful as
    /// it sounds — the report was "no chance to speak before it finishes".
    ///
    /// `--nocapture` is not optional. Without it the harness swallows the prompt
    /// and the test looks like it has hung.
    ///
    /// What it cannot tell you is whether the *permission* was granted: a denied
    /// microphone still opens and still streams, and delivers silence or room
    /// tone. That case shows up as `voiced` staying at zero — which the analyser
    /// reports as "I could not hear enough voice", rather than as a wrong tone.
    /// The printed peak level tells the two apart from a genuinely quiet attempt.
    ///
    /// ## Calibrating the scoring floor with it
    ///
    /// The tones worth trying are the short ones, because they are what the floor
    /// in `hanzi_core::tone` decides about. Set the character and the hold:
    ///
    /// ```text
    /// HANZI_TUTOR_TONE_TEST=是 HANZI_TUTOR_TONE_SECS=3 \
    ///   cargo test -p hanzi-tutor --lib -- --ignored --nocapture records_from_the_real_microphone
    /// ```
    ///
    /// Then read `voicing` against `floor`, which each round prints as a margin.
    /// `voicing` is the number the analyser compares against the floor, and it is
    /// *not* the length of the syllable: a frame counts only when its whole
    /// analysis window holds a period, so the measured figure runs short of the
    /// sound by the window's edges. What is worth having is margin — if a
    /// comfortable 是 lands a long way above the floor, the floor can come down;
    /// if it lands just above, normal-rate speech will fall under it and
    /// `MIN_VOICED_MS` wants revisiting. **Say it normally, not carefully**: the
    /// whole point is what an ordinary attempt measures. Two or three rounds of the
    /// same word is more useful than one.
    #[test]
    #[ignore = "needs a microphone and permission"]
    fn records_from_the_real_microphone() {
        /// One environment value, trimmed, or a fallback when it is unset.
        fn wished_for(key: &str, fallback: &str) -> String {
            std::env::var(key)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| fallback.to_string())
        }

        /// One line from the terminal.
        ///
        /// `None` at end of input, which is what a piped or CI run looks like.
        /// The caller records once and stops rather than waiting on a stdin that
        /// will never produce anything.
        fn read_line() -> Option<String> {
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                // A read error is treated as "no terminal" for the same reason:
                // there is nothing to interact with.
                Ok(0) | Err(_) => None,
                Ok(_) => Some(line),
            }
        }

        let wanted = wished_for("HANZI_TUTOR_TONE_TEST", "是");
        let seconds: f32 = wished_for("HANZI_TUTOR_TONE_SECS", "3")
            .parse()
            .expect("HANZI_TUTOR_TONE_SECS must be a number of seconds");

        let recorder = Recorder::default();
        let status = recorder.status();
        println!("microphone: {status:?}");
        assert!(status.available, "no microphone available: {}", status.detail);

        let state = crate::state::AppState::load(None).expect("the dataset should load");
        let target = state
            .tone_target(&wanted)
            .unwrap_or_else(|| panic!("{wanted} has no scorable reading"));

        let floor_ms = hanzi_core::tone::MIN_VOICED_MS;
        let silence_gate = hanzi_core::tone::PitchConfig::default().silence_rms;

        println!();
        println!("Scoring {wanted}. Nothing is recorded until you press Enter, and each");
        println!("round says RECORDING before it listens. Enter again to repeat, q to stop.");

        let mut interactive = true;
        let mut rounds = 0usize;
        let mut empty = 0usize;

        loop {
            if interactive {
                println!();
                // `println!`, never `print!`, for this prompt. Rust's stdout is
                // line-buffered, so a prompt written without a newline sits in the
                // buffer while `read_line` blocks — the screen shows nothing and
                // the test looks hung. That is the whole failure this test exists
                // to avoid repeating.
                println!(
                    "--- press Enter to record {seconds} s of {wanted}, \
                     or q then Enter to finish ---"
                );
                match read_line() {
                    // No terminal to read from. Record once and stop.
                    None => interactive = false,
                    Some(line) if line.trim().eq_ignore_ascii_case("q") => break,
                    Some(_) => {}
                }
            }

            // The device is opened *before* the prompt to speak rather than after
            // it, so nobody is talking into a microphone that is still opening.
            // That is the order the app's own push-to-talk button uses, and the
            // reason it does not lose the beginning of a syllable.
            if let Err(problem) = recorder.start() {
                println!("could not open the microphone: {problem}");
                break;
            }
            println!(">>> RECORDING — say {wanted} now <<<");
            std::thread::sleep(Duration::from_secs_f32(seconds));
            let recording = recorder.stop().expect("stopping should succeed");
            println!(">>> done <<<");

            rounds += 1;
            println!(
                "captured {} samples at {} Hz from {} (truncated: {})",
                recording.samples.len(),
                recording.sample_rate,
                recording.device,
                recording.truncated
            );
            if recording.samples.is_empty() {
                empty += 1;
                println!("  the stream ran but delivered nothing at all");
            }

            let loudest = recording
                .samples
                .iter()
                .fold(0.0f32, |peak, s| peak.max(s.abs()));
            println!("peak level: {loudest:.4} (the analyser's silence gate is {silence_gate:.4})");
            if loudest < silence_gate {
                println!(
                    "  quieter than that gate, so every frame is unvoiced before the pitch \
                     estimator runs at all. Either nothing was said, or the input is muted — a \
                     terminal that has never been granted the microphone produces exactly this."
                );
            }

            // The instrument reading behind the verdict, which the report itself
            // has no room for: the geometry of the track, the floor it is judged
            // against, and how much voice there was to judge.
            let resampled = hanzi_core::tone::resample(
                &recording.samples,
                recording.sample_rate,
                hanzi_core::tone::TARGET_SAMPLE_RATE,
            );
            let track = hanzi_core::tone::track_pitch(
                &resampled,
                hanzi_core::tone::TARGET_SAMPLE_RATE,
                &hanzi_core::tone::PitchConfig::default(),
            );
            let hop_ms = track.hop as f32 * 1000.0 / track.sample_rate as f32;
            println!(
                "track: {} frames, {} voiced, hop {:.2} ms, window {:.2} ms",
                track.frames.len(),
                track.frames.iter().filter(|f| f.voiced).count(),
                hop_ms,
                hop_ms * 8.0,
            );
            let floor_frames = hanzi_core::tone::min_voiced_frames(&track);
            println!("floor: {floor_frames} frames at this hop ({floor_ms} ms)");

            // The measurement has to come from the track, not from the report.
            //
            // `ToneReport` zeroes `voiced_ms` and `span_ms` when it refuses a
            // syllable, so printing the report's figures here said "voicing 0 ms"
            // about a recording the line above had just counted ten voiced frames
            // in. That is the one number this test exists to print, and it was
            // being read from the wrong place. `contour_in` is exactly what the
            // scorer's floor is applied to, so it is the honest source.
            //
            // For a word this is the *whole utterance* over one span, which is not
            // the figure that decides anything — each syllable is gated on its own
            // segment. It is printed as a sanity check, and the per-syllable
            // numbers below are the ones to read.
            let whole = if target.syllables.len() == 1 {
                "the syllable".to_string()
            } else {
                format!("the whole word ({} syllables)", target.syllables.len())
            };
            let measured = hanzi_core::tone::voiced_span(&track)
                .and_then(|(first, last)| hanzi_core::tone::contour_in(&track, first, last));
            match measured {
                Some(contour) => {
                    let margin = contour.voiced_ms as i64 - floor_ms as i64;
                    println!(
                        "whole span: {whole} — {} ms of voicing in a {} ms span, median {:.0} Hz \
                         (margin {margin:+} ms)",
                        contour.voiced_ms, contour.span_ms, contour.median_hz
                    );
                }
                None => {
                    let gated = track
                        .frames
                        .iter()
                        .filter(|f| f.voiced && f.hz > 0.0)
                        .count();
                    println!(
                        "whole span: {whole} — refused, {gated} voiced frames against \
                         {floor_frames} needed"
                    );
                }
            }

            let result = state.score_tones(&recording, &target);
            println!(
                "verdict {:?}, score {:.1}: {}",
                result.verdict, result.score, result.detail
            );

            // Per syllable, because for a word the whole-utterance figures above
            // are not what decides anything: each syllable is gated on its own
            // segment, and it is the *middle* one that a learner reports going
            // unrecognised. Printing only the verdict would say a word failed
            // without saying which syllable was short of voice, which is the one
            // thing worth knowing.
            let expected = target.syllables.len();
            if result.boundaries_ms.is_empty() && expected == 1 {
                println!("split: none — one syllable");
            } else if result.boundaries_ms.is_empty() {
                // Empty here does not mean "no split was needed": the report is
                // only built with boundaries when the span was long enough to
                // divide, and a refusal leaves this empty however many syllables
                // were asked for. Saying "one syllable" about 是不是 would be a
                // plain lie.
                println!("split: never attempted — the recording was refused before division");
            } else {
                println!(
                    "split after: {} ms (from the start of speech)",
                    result
                        .boundaries_ms
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            for syllable in &result.syllables {
                println!(
                    "  {}. {} ({}, tone {}): {:?} {:.1} — voicing {} ms in a {} ms span, \
                     median {:.0} Hz, range {:.1} st",
                    syllable.position,
                    syllable.ch,
                    syllable.reading,
                    syllable.spoken,
                    syllable.attempt.verdict,
                    syllable.attempt.score,
                    syllable.attempt.voiced_ms,
                    syllable.attempt.span_ms,
                    syllable.attempt.median_hz,
                    syllable.attempt.range_semitones,
                );
                println!("     {}", syllable.attempt.detail);
            }

            // Is a big `range` a real pitch movement, or the track jumping?
            //
            // A hand test on 中国人 reported segments of 188 and 306 ms with ranges
            // of 26.7 and 27.8 semitones — more than two octaves, which no voice
            // does in a fifth of a second, and at durations that are one ordinary
            // syllable. That is a pitch *tracker* question, and the report cannot
            // answer it because it only carries the finished contour.
            //
            // So the raw frames are walked per segment, looking for the step a
            // voice cannot make: adjacent frames are one hop apart, about 3 ms, so
            // the ratio between them is near 1 or the estimator has jumped — most
            // often to half the frequency, which is creak at the end of a syllable
            // and shows up as a whole octave in the range.
            if let Some((first_frame, last_frame)) = hanzi_core::tone::voiced_span(&track) {
                let mut edges = vec![first_frame];
                for boundary in &result.boundaries_ms {
                    edges.push(first_frame + (*boundary as f32 / hop_ms).round() as usize);
                }
                edges.push(last_frame + 1);

                for (index, pair) in edges.windows(2).enumerate() {
                    let from = pair[0].min(track.frames.len());
                    let to = pair[1].min(track.frames.len()).max(from);
                    let mut jumps = 0usize;
                    let mut previous: Option<f32> = None;
                    let mut lowest = f32::MAX;
                    let mut highest = 0.0f32;
                    for frame in &track.frames[from..to] {
                        if !frame.voiced || frame.hz <= 0.0 {
                            continue;
                        }
                        lowest = lowest.min(frame.hz);
                        highest = highest.max(frame.hz);
                        if let Some(p) = previous {
                            if !(0.75..=1.33).contains(&(frame.hz / p)) {
                                jumps += 1;
                            }
                        }
                        previous = Some(frame.hz);
                    }
                    if previous.is_none() {
                        continue;
                    }
                    println!(
                        "     segment {}: {lowest:.0}–{highest:.0} Hz, {jumps} tracking jump(s)",
                        index + 1
                    );
                }
            }

            if !interactive {
                break;
            }
        }

        // The backend check the original assertion existed for: a stream that runs
        // and produces nothing is a broken capture path. A *transient* empty round
        // is not — a learner who pressed Enter and said nothing gets another go —
        // so this only fails when every round was empty.
        if rounds > 0 {
            println!();
            println!("finished after {rounds} round(s).");
            assert!(
                empty < rounds,
                "the stream ran but produced no samples at all, every time"
            );
        }
    }
}
