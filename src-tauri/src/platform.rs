//! This app's address for the shared platform bridge.
//!
//! The bridge itself — the Kotlin plugin's registration, the blocking `call`
//! into it, and the recording and synthesis code that uses it — lives in
//! `hanzi-voice`, because the standalone tone trainer needs all of it too. What
//! could not move is *where this app's copy of the Kotlin plugin lives*: that is
//! the Android application id, which belongs to the application and not to a
//! shared crate.
//!
//! So this module is a shim, and deliberately a thin one. It re-exports the
//! shared names under the path the rest of this crate has always used
//! (`crate::platform::call`), and supplies the one thing the shared crate cannot
//! know: the package and class to register.

/// The Kotlin class this app ships, in the package its Gradle project generates.
///
/// `gen/android/app/src/main/java/com/hanzitutor/app/PlatformPlugin.kt`. The
/// independent tone trainer declares the same plugin under its own application
/// id, which is exactly why the shared crate takes this as an argument rather
/// than naming it.
pub const BRIDGE: hanzi_voice::platform::Bridge = hanzi_voice::platform::Bridge {
    package: "com.hanzitutor.app",
    class: "PlatformPlugin",
};

// `call` exists only on Android — on every other platform there is nothing on
// the other side of the bridge to call — so the re-export is gated the same way
// rather than being made unconditional in the shared crate.
#[cfg(target_os = "android")]
pub use hanzi_voice::platform::call;

/// Register this app's Kotlin plugin. Registers nothing off Android.
pub fn init() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    hanzi_voice::platform::init(BRIDGE)
}
