//! Hanzi Tutor — learn to read and write simplified Chinese characters.
//!
//! The Rust side owns the character dataset and the grading engine; the webview
//! handles drawing and presentation. See `crate::commands` for the boundary.

mod commands;
mod speech;
mod state;

pub use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let state = AppState::load()?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::dataset_stats,
            commands::lessons,
            commands::character,
            commands::grade_attempt,
            commands::speak,
            commands::stop_speaking,
            commands::speech_status,
            commands::webview_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hanzi Tutor");
}
