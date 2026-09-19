//! Hanzi Tutor — learn to read and write simplified Chinese characters.
//!
//! The Rust side owns the character dataset and the grading engine; the webview
//! handles drawing and presentation. See `crate::commands` for the boundary.

mod commands;
pub mod licences;
mod capture;
mod speech;
mod state;

pub use commands::{LevelCount, VocabOutcome, VoiceOption, VoicesView, WordSearchView};
pub use licences::{AppInfo, LicenceNotice};
pub use capture::{MicrophoneStatus, Recorder, Recording};
pub use state::{
    AppState, CursorState, Persisted, ProgressState, SettingsState, VocabState, REVIEW_LIMIT,
};

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

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            let state = AppState::load(data_dir)?;
            app.manage(state);
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
            commands::tone_target,
            commands::microphone_status,
            commands::listen_start,
            commands::listen_stop,
            commands::lookup_text,
            commands::vocabulary,
            commands::vocab_add,
            commands::vocab_update,
            commands::vocab_remove,
            commands::vocab_add_group,
            commands::vocab_rename_group,
            commands::vocab_remove_group,
            commands::vocab_record_attempt,
            commands::vocab_export,
            commands::vocab_import,
            commands::progress,
            commands::record_progress,
            commands::review_queue,
            commands::course_cursor,
            commands::set_course_cursor,
            commands::settings,
            commands::update_settings,
            commands::clear_click_to_draw,
            commands::app_info,
            commands::licence_notices,
            commands::webview_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hanzi Tutor");
}
