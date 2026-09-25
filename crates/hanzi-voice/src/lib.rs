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

pub mod capture;
pub mod platform;
pub mod speech;

pub use capture::{MicrophoneStatus, Recorder, Recording, MAX_RECORD_SECS};
pub use platform::Bridge;
pub use speech::{Speaker, Voice};
