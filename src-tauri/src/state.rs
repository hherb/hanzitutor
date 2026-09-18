//! Loading the character dataset that the app grades against, and the pieces of
//! application state that outlive a single command.

use std::sync::Arc;

use hanzi_core::Dataset;

use crate::speech::Speaker;

/// The compact artifact produced by `hanzi-core`'s `prepare-data` binary.
///
/// It is embedded rather than read from disk so that development builds and
/// bundled apps behave identically, with no resource-path resolution to get
/// wrong. It is about 13 MB compressed.
const ARTIFACT: &[u8] = include_bytes!("../../crates/hanzi-core/data/hanzi.bin.gz");

/// State held for the lifetime of the app and shared by all commands.
pub struct AppState {
    pub dataset: Dataset,
    /// Shared with a warm-up thread, so it is behind an `Arc`.
    pub speech: Arc<Speaker>,
}

impl AppState {
    /// Decode the embedded dataset and start the pronunciation warm-up.
    pub fn load() -> Result<Self, String> {
        let dataset = Dataset::from_gzip_bytes(ARTIFACT)
            .map_err(|e| format!("could not read the embedded character dataset: {e}"))?;

        let speech = Arc::new(Speaker::default());
        warm_voice(Arc::clone(&speech));

        Ok(Self { dataset, speech })
    }
}

/// Resolve the system voice on a background thread.
///
/// Enumerating voices means running `say -v '?'`, which takes about a second.
/// Doing it here keeps that cost off both the startup path and the first
/// "hear it" click.
fn warm_voice(speaker: Arc<Speaker>) {
    std::thread::spawn(move || match speaker.status() {
        Some(voice) => eprintln!("[speech] using voice {voice}"),
        None => eprintln!(
            "[speech] no Chinese voice installed; pronunciation will be unavailable"
        ),
    });
}
