//! Microphone capture and system speech synthesis.
//!
//! The two halves of tone practice that are not signal processing: getting the
//! learner's voice into a buffer ([`capture`]), and pronouncing a character back
//! at them with whatever the operating system already has installed
//! ([`speech`]).
//!
//! ## Why this is its own crate
//!
//! Both apps in this repository score tone — the full Hanzi Tutor app and the
//! standalone trainer under `apps/tone-trainer` — and both need exactly these
//! two things. The scoring itself lives in `hanzi-core` and was always shared;
//! capture and speech used to live in the main app's `src-tauri`, which would
//! have meant copying about two thousand lines of platform code into the second
//! app. They are here instead, so a fix to the Android recorder or the iOS audio
//! session reaches both.
//!
//! ## The microphone is a feature, because one caller does not have one
//!
//! `apps/nihongo-tutor` needs only [`speech`]: it pronounces a kana and never
//! records anything. The capture half carries `cpal`, which is a microphone stack
//! on every platform, and linking it into an app that will never open one is the
//! same mistake as bundling the 134 MB analyser — see [`capture`]'s own note on
//! why the two halves are not separated further. So `capture` is a **default**
//! feature: the two tone-scoring apps get it by saying nothing, and an app that
//! only speaks turns it off ([`speech`] itself has no dependency on it).
//!
//! ## What is deliberately still app-side
//!
//! The Android Kotlin plugin itself. Registering it is one call
//! ([`platform::init`]) and the address it lives at differs per application, so
//! each app keeps its own `PlatformPlugin.kt` and passes its own package and
//! class. Everything the Rust side does with that plugin — the recorder, the
//! synthesiser, the voice listing — is here.
//!
//! Bundled neural synthesis (`hanzi-say`) is *not* here, and is not a dependency
//! of this crate: it carries the `sherpa-onnx` native stack, which a system-voice
//! app has no reason to link. An app that wants it layers it on top.

#[cfg(feature = "capture")]
pub mod capture;
pub mod platform;
pub mod speech;

#[cfg(feature = "capture")]
pub use capture::{MicrophoneStatus, Recorder, Recording, MAX_RECORD_SECS};
pub use platform::Bridge;
pub use speech::{Language, Speaker, Voice};
