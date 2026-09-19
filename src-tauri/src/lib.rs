//! Hanzi Tutor — learn to read and write simplified Chinese characters.
//!
//! The Rust side owns the character dataset and the grading engine; the webview
//! handles drawing and presentation. See `crate::commands` for the boundary.

mod commands;
pub mod licences;
mod speech;
mod state;

pub use commands::{LevelCount, VocabOutcome, WordSearchView};
pub use licences::{AppInfo, LicenceNotice};
pub use state::{AppState, CursorState, Persisted, ProgressState, VocabState, REVIEW_LIMIT};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri::Manager;
            // The vocabulary list lives in the platform's application data
            // directory; a failure to locate it is not fatal, the list simply
            // stays in memory and says so.
            let data_dir = crate::state::resolve_data_dir(app.handle()).ok();
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
            commands::app_info,
            commands::licence_notices,
            commands::webview_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hanzi Tutor");
}
