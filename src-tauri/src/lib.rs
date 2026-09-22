//! Hanzi Tutor — learn to read and write simplified Chinese characters.
//!
//! The Rust side owns the character dataset and the grading engine; the webview
//! handles drawing and presentation. See `crate::commands` for the boundary.

mod commands;
pub mod licences;
mod asr;
mod capture;
mod say;
mod platform;
mod speech;
mod state;
mod sync;

pub use commands::{
    LevelCount, MarkedTone, SpokenAudio, VocabOutcome, VoiceOption, VoicesView, WordSearchView,
};
pub use licences::{AppInfo, LicenceNotice};
pub use asr::{Asr, AsrStatus, InstallState};
pub use say::{Say, SayStatus};
pub use capture::{MicrophoneStatus, Recorder, Recording};
pub use state::{
    AppState, CursorState, Persisted, ProgressState, SettingsState, VocabState, REVIEW_LIMIT,
};
pub use sync::{AutoSync, SyncService, SyncSummaryView, SyncView};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Where study data goes has to be settled before anything opens a file, and
    // a malformed `--user-dir` is worth stopping for. Carrying on with the
    // default would scatter a testing session's data into the real application
    // support directory, which is the one thing the flag exists to prevent.
    let user_dir = match crate::state::user_dir_from_args(std::env::args_os().skip(1)) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(2);
        }
    };

    // The expensive half of the state is built *before* the Tauri builder, and
    // that placement is load-bearing on mobile rather than a style choice. The
    // Android webview starts loading while `setup` runs, so a slow `setup` lets
    // the frontend's first commands arrive before `app.manage()` has been
    // called — and a command that finds no state is rejected outright instead of
    // waiting, which left the phone showing "state not managed ... on command
    // review_queue" and an empty board. Decoding the dataset and ordering the
    // course out here leaves `setup` doing nothing but opening the study
    // database, which is fast enough that the webview cannot get there first.
    let prepared = match AppState::prepare() {
        Ok(prepared) => prepared,
        Err(message) => {
            // The dataset is embedded, so this means the build itself is broken;
            // there is nothing to fall back to and no window worth opening.
            eprintln!("error: {message}");
            std::process::exit(2);
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Opens the Dropbox authorization page in the system browser. A plugin
        // rather than `window.open`, because that would put Dropbox's sign-in
        // inside a webview — which Dropbox asks against, and which Google's policy
        // forbids outright for the accounts that sign in through it.
        .plugin(tauri_plugin_opener::init())
        // The Kotlin half of the platform seam: the system synthesiser and the
        // window insets. Registers nothing anywhere but Android.
        .plugin(crate::platform::init())
        .setup(move |app| {
            use tauri::Manager;
            // The vocabulary list lives in the platform's application data
            // directory; a failure to locate it is not fatal, the list simply
            // stays in memory and says so. The chosen location is logged because
            // it is the first thing worth knowing when a save misbehaves — and
            // because `--user-dir` makes it a choice rather than a given.
            let data_dir = match crate::state::resolve_data_dir(app.handle(), user_dir) {
                Ok(dir) => {
                    eprintln!("[data] study files in {}", dir.display());
                    Some(dir)
                }
                Err(message) => {
                    eprintln!("[data] {message}");
                    eprintln!("[data] study data will not be saved this session");
                    None
                }
            };
            let state = AppState::assemble(prepared, data_dir);
            // Sync gets the database itself rather than any of the three views over
            // it, because what it moves is the attempt log underneath them.
            let sync = SyncService::new(state.db.clone());
            app.manage(state);
            app.manage(sync);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::dataset_stats,
            commands::lessons,
            commands::character,
            commands::teachable_characters,
            commands::search_words,
            commands::grade_attempt,
            commands::speak,
            commands::stop_speaking,
            commands::speech_status,
            commands::voices,
            commands::speech_target,
            commands::microphone_status,
            commands::listen_start,
            commands::listen_stop,
            commands::asr_status,
            commands::asr_install,
            commands::asr_remove,
            commands::say_status,
            commands::say_install,
            commands::say_remove,
            commands::say_speak,
            commands::lookup_text,
            commands::mark_tone,
            commands::vocabulary,
            commands::vocab_add,
            commands::vocab_update,
            commands::vocab_remove,
            commands::vocab_add_group,
            commands::vocab_rename_group,
            commands::vocab_remove_group,
            commands::vocab_cursor,
            commands::set_vocab_cursor,
            commands::vocab_record_attempt,
            commands::vocab_export,
            commands::vocab_import,
            commands::progress,
            commands::record_progress,
            commands::export_practice_log,
            commands::review_queue,
            commands::course_cursor,
            commands::set_course_cursor,
            commands::settings,
            commands::update_settings,
            commands::clear_click_to_draw,
            commands::app_info,
            commands::licence_notices,
            commands::android_insets,
            commands::speech_report,
            commands::webview_log,
            commands::sync_status,
            commands::sync_connect,
            commands::sync_connect_finish,
            commands::sync_now,
            commands::sync_disconnect,
            commands::sync_set_lock,
            commands::sync_auto,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hanzi Tutor");
}
