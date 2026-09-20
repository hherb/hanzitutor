//! The Android half of the platform seam.
//!
//! Two things on Android are only reachable from Java, and both are needed by
//! this app: the system speech synthesiser, and the window insets that decide
//! whether the header sits under the status bar. Rather than one binding for
//! each, there is a single small Kotlin plugin —
//! `gen/android/app/src/main/java/com/hanzitutor/app/PlatformPlugin.kt` — and
//! this is the typed way in to it.
//!
//! Everything here is `cfg`-ed to Android except [`init`], which every platform
//! calls: on the others it registers nothing and returns an empty plugin, so the
//! entry point stays free of platform branches.

#[cfg(target_os = "android")]
static HANDLE: std::sync::OnceLock<tauri::plugin::PluginHandle<tauri::Wry>> =
    std::sync::OnceLock::new();

/// The name the plugin is registered under on the Rust side.
pub const NAME: &str = "hanzi-platform";

/// The Kotlin class, and the package it lives in.
///
/// Both are looked up reflectively by Tauri, which is why they are strings
/// rather than a type: `register_android_plugin` instantiates the class as
/// `(Landroid/app/Activity;)V` and hands it to the plugin manager.
#[cfg(target_os = "android")]
const ANDROID_PACKAGE: &str = "com.hanzitutor.app";
#[cfg(target_os = "android")]
const ANDROID_CLASS: &str = "PlatformPlugin";

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
pub fn init() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let builder = tauri::plugin::Builder::new(NAME);

    #[cfg(target_os = "android")]
    let builder = builder.setup(|_app, api| {
        let handle = api
            .register_android_plugin(ANDROID_PACKAGE, ANDROID_CLASS)
            .map_err(|error| error.to_string())?;
        // `setup` runs once, so a second registration is not a case that can
        // happen; ignoring the result keeps that from being a panic if it ever
        // does.
        let _ = HANDLE.set(handle);
        Ok(())
    });

    builder.build()
}
