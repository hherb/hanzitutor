//! This app's address for the shared platform bridge.
//!
//! The bridge, the recorder and the synthesiser all live in `hanzi-voice`,
//! because the full app needs them too. What cannot be shared is *where this
//! app's Kotlin plugin lives*: that is the Android application id, which is a
//! fact about this application rather than about a shared crate.
//!
//! So this module is three lines of shim. It supplies the address and nothing
//! else — see `hanzi-voice`'s `platform` module for the contract.
//!
//! ## The Kotlin side is not written yet
//!
//! The full app's `PlatformPlugin.kt` does the recording, speaking and window
//! insets for `com.hanzitutor.app`. This app registers under
//! `com.hanzitutor.tone` and has no Kotlin source of its own yet, so an Android
//! build reaches the bridge, finds nothing registered, and reports that honestly
//! (`capture` and `speech` both turn it into a sentence rather than a panic).
//! Desktop and iOS are complete, and `HANDOVER.md` records what Android still
//! needs. Registering the address now rather than at that point keeps the
//! difference to one Kotlin file.

use hanzi_voice::platform::Bridge;

/// Where this app's Kotlin plugin belongs, under its own application id.
pub const BRIDGE: Bridge = Bridge {
    package: "com.hanzitutor.tone",
    class: "PlatformPlugin",
};

/// Register the plugin. Registers nothing off Android.
pub fn init() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    hanzi_voice::platform::init(BRIDGE)
}
