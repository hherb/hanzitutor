//! The Android half of the platform seam.
//!
//! Two things on Android are only reachable from Java, and both are needed by
//! any app that scores tone from a microphone: the system speech synthesiser,
//! and — in the Hanzi Tutor app — the window insets that decide whether the
//! header sits under the status bar. Rather than one binding for each, each app
//! ships a single small Kotlin plugin and registers it here; this module is the
//! typed way in to it.
//!
//! ## Why the package and the class are arguments
//!
//! This module used to name Hanzi Tutor's own Kotlin class directly. It is
//! shared now (both apps depend on this crate), and the applications have
//! different Android application ids, so the class genuinely lives at a
//! different address in each. [`init`] therefore takes the address rather than
//! assuming it, and the plugin *name* — which is what Tauri and this module
//! agree on — is fixed ([`NAME`]). A standalone tone trainer can keep the
//! contract without inheriting the other app's identifier.
//!
//! Everything here is `cfg`-ed to Android except [`init`], which every platform
//! calls: on the others it registers nothing and returns an empty plugin, so the
//! entry point stays free of platform branches.

#[cfg(target_os = "android")]
static HANDLE: std::sync::OnceLock<tauri::plugin::PluginHandle<tauri::Wry>> =
    std::sync::OnceLock::new();

/// The name the plugin is registered under on the Rust side.
///
/// Fixed rather than configurable, because the Kotlin side is written against
/// it: a class called anything else is still registered under this name, and the
/// commands the two sides exchange are named here too.
pub const NAME: &str = "hanzi-platform";

/// The Kotlin plugin to register on Android.
///
/// Only read on Android, where it decides what Tauri instantiates reflectively;
/// the other platforms accept and ignore it, which is what keeps their entry
/// points free of platform branches.
#[derive(Clone, Copy, Debug)]
pub struct Bridge {
    /// The Java package the plugin lives in, e.g. `com.hanzitutor.app`.
    pub package: &'static str,
    /// The plugin class inside that package, e.g. `PlatformPlugin`.
    pub class: &'static str,
}

/// Call one of the Kotlin plugin's commands and decode its answer.
///
/// This **blocks the calling thread** until the Kotlin side answers, because
/// that is what `run_mobile_plugin` does. That is safe here and deliberately
/// not mirrored on the other side: Tauri hands the command to Android's main
/// thread, so the Kotlin handlers are written never to block, and every caller
/// of this function is a Tauri command on a worker thread — or the pronunciation
/// warm-up, which is a thread of its own.
///
/// A plugin that has not been registered yet is an error rather than a silent
/// empty answer, because every caller can say something honest about that.
#[cfg(target_os = "android")]
pub fn call<T: serde::de::DeserializeOwned>(
    command: &str,
    payload: impl serde::Serialize,
) -> Result<T, String> {
    let handle = HANDLE.get().ok_or_else(|| {
        "the Android platform bridge is not registered yet".to_string()
    })?;
    handle
        .run_mobile_plugin(command, payload)
        .map_err(|error| error.to_string())
}

/// Register the Kotlin plugin with the application.
///
/// `bridge` is where that app's Kotlin plugin lives; on every platform but
/// Android it is accepted and unused, so a caller can pass its own address
/// unconditionally.
pub fn init(bridge: Bridge) -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let builder = tauri::plugin::Builder::new(NAME);

    #[cfg(target_os = "android")]
    let builder = builder.setup(move |_app, api| {
        let handle = api
            .register_android_plugin(bridge.package, bridge.class)
            .map_err(|error| error.to_string())?;
        // `setup` runs once, so a second registration is not a case that can
        // happen; ignoring the result keeps that from being a panic if it ever
        // does.
        let _ = HANDLE.set(handle);
        Ok(())
    });

    #[cfg(not(target_os = "android"))]
    let _ = bridge;

    builder.build()
}
