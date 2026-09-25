//! On-device Mandarin speech recognition: *what* was said.
//!
//! The other half of what the two apps measure. Tone is an F0 contour and needs
//! no model — `hanzi-core`'s `tone` module judges it from the pitch alone — but a
//! contour cannot tell 四 (`sì`) from 是 (`shì`), which score identically on tone
//! while being different words. There is no non-neural substitute for that,
//! because a learner's voice cannot be pre-rendered, so this is the one thing in
//! either application that needs model weights.
//!
//! ## The model is fetched by the learner, or not at all
//!
//! 163 MB cannot be bundled, so it is downloaded — which is why this crate is
//! also the only thing that touches the network anywhere in the project. That was
//! a deliberate product decision (ROADMAP.md M12) and the bounds are implemented
//! rather than merely intended:
//!
//! - **Nothing degrades when the model is absent.** [`Asr::recognize`] answers
//!   `Ok(None)`, tone practice is untouched, and no code path here runs until
//!   [`Asr::install`] is called. Nothing prompts, ever.
//! - **The download is verified** against a pinned SHA-256, and a failure leaves
//!   nothing behind to be mistaken for a working install.
//! - **Declining is a first-class state.** A learner can use either app for years
//!   and never see any of it.
//!
//! ## Why this is not in `hanzi-voice`
//!
//! They are split by what an app links, not by what it does. Capture and the
//! system voice are small and always useful; this pulls in `sherpa-onnx`. An app
//! that only ever wants the microphone should not link a speech engine to get it,
//! and one that wants recognition opts into both.

pub mod asr;

pub use asr::{Asr, AsrStatus, InstallState, ModelSpec, MODEL};
