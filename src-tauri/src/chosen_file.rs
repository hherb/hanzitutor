//! A file the learner chose in a native dialog, read and written wherever the
//! platform keeps it.
//!
//! The dialog does not answer in the same shape everywhere. On desktop it gives a
//! path, and `std::fs` can open it. On **Android** it gives a `content://` URI from
//! the Storage Access Framework — the learner may have picked Downloads, a cloud
//! drive or an SD card, and none of those is a path this process can open. That
//! URI used to go through `FilePath::into_path`, which refuses anything but a
//! `file://` URL, so every export, import, backup and restore failed on Android
//! with "that file cannot be written" while the interface went on offering them.
//!
//! `tauri-plugin-fs` is the way through: [`tauri_plugin_fs::Fs::open`] hands a
//! path to `std::fs` on desktop, and on Android asks the plugin's Kotlin half to
//! open the URI with `ContentResolver.openAssetFileDescriptor` and returns the
//! descriptor as an ordinary [`std::fs::File`]. Only Rust uses it. The plugin's
//! commands are registered with the app but no capability grants an `fs:`
//! permission, so the webview cannot call one, and the rule that **the frontend
//! never names a file** still holds: a [`PickedFile`] is made from the dialog's
//! answer and from nothing else.
//!
//! What gets written and read — the vocabulary list, the log, the backup — is
//! decided in [`crate::AppState`] against the [`ChosenFile`] trait, so it can be
//! tested with a plain file and no window.

use std::io::{Read, Write};

use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::FilePath;
use tauri_plugin_fs::{FsExt, OpenOptions};

/// Somewhere the learner chose to export to or import from.
///
/// The methods report failure as the sentence the interface shows, naming the
/// file the way [`ChosenFile::describe`] does.
pub trait ChosenFile {
    /// The file as the learner would recognise it, for "exported … to …".
    fn describe(&self) -> String;

    /// The whole file, as UTF-8 text.
    fn read_to_string(&self) -> Result<String, String>;

    /// Replace the file's contents with `contents`, creating it if need be.
    fn write(&self, contents: &[u8]) -> Result<(), String>;
}

/// A file named by the native dialog, opened through `tauri-plugin-fs` so that an
/// Android `content://` URI works as well as a desktop path.
pub struct PickedFile<'a, R: Runtime> {
    app: &'a AppHandle<R>,
    path: FilePath,
}

impl<'a, R: Runtime> PickedFile<'a, R> {
    /// Wrap what the dialog answered.
    ///
    /// Crate-private, and called only by the commands that showed the dialog:
    /// the argument is the learner's choice, never a string from the webview.
    pub(crate) fn from_dialog(app: &'a AppHandle<R>, chosen: FilePath) -> Self {
        Self {
            app,
            // Strips a Windows `\\?\` prefix from a path; a URL is left alone.
            path: chosen.simplified(),
        }
    }
}

impl<R: Runtime> ChosenFile for PickedFile<'_, R> {
    fn describe(&self) -> String {
        describe(&self.path)
    }

    fn read_to_string(&self) -> Result<String, String> {
        let mut text = String::new();
        self.app
            .fs()
            .open(self.path.clone(), read_options())
            .and_then(|mut file| file.read_to_string(&mut text))
            .map_err(|e| format!("could not read {}: {e}", self.describe()))?;
        Ok(text)
    }

    fn write(&self, contents: &[u8]) -> Result<(), String> {
        // No `sync_all`: a cloud provider on Android hands back one end of a pipe,
        // and `fsync` on a pipe is `EINVAL`. Closing the descriptor is what tells
        // the provider the document is complete, exactly as `std::fs::write` does.
        self.app
            .fs()
            .open(self.path.clone(), write_options())
            .and_then(|mut file| file.write_all(contents))
            .map_err(|e| format!("could not write {}: {e}", self.describe()))
    }
}

/// Read only — `"r"` on Android.
fn read_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true);
    options
}

/// Write, truncating whatever was there — `"wt"` on Android.
///
/// The truncate matters on Android in particular: plain `"w"` does not truncate
/// for every provider, and a backup written over a longer one would be left with
/// the old file's tail and fail to parse. `create` is for desktop, where the save
/// dialog names a file that need not exist yet; on Android the dialog has already
/// created the document, and the mode has no equivalent.
fn write_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true).truncate(true).create(true);
    options
}

/// How to name a chosen file in a message.
///
/// A path is shown as itself. An Android `content://` URI is not shown at all: it
/// names a provider and an opaque document id (`…/document/msf%3A1000000034`),
/// which tells the learner nothing they did not just choose themselves.
pub fn describe(path: &FilePath) -> String {
    match path {
        FilePath::Path(path) => path.display().to_string(),
        FilePath::Url(url) => match url.to_file_path() {
            Ok(path) if url.scheme() == "file" => path.display().to_string(),
            _ => "the file you chose".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::str::FromStr;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hanzi-chosen-file-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A windowless app with the fs plugin registered, as `run` registers it.
    fn app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .plugin(tauri_plugin_fs::init())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("a mock app builds")
    }

    #[test]
    fn a_path_is_named_as_itself_and_a_content_uri_is_not() {
        let path = PathBuf::from("/home/learner/hanzi-tutor-backup.json");
        assert_eq!(
            describe(&FilePath::Path(path.clone())),
            path.display().to_string()
        );
        let url = FilePath::from_str("file:///home/learner/hanzi-tutor-backup.json").unwrap();
        assert!(matches!(url, FilePath::Url(_)), "{url:?}");
        assert_eq!(describe(&url), path.display().to_string());

        // What the Android dialog actually answers with.
        let content = FilePath::from_str(
            "content://com.android.providers.downloads.documents/document/msf%3A1000000034",
        )
        .unwrap();
        assert!(matches!(content, FilePath::Url(_)), "{content:?}");
        assert_eq!(describe(&content), "the file you chose");
    }

    #[test]
    fn a_chosen_path_is_written_replaced_and_read_back() {
        let dir = scratch("path");
        let app = app();
        let file = dir.join("export.json");
        let chosen = PickedFile::from_dialog(app.handle(), FilePath::Path(file.clone()));

        // Created when it does not exist: the save dialog names a new file.
        chosen.write(b"a longer first version").unwrap();
        assert_eq!(chosen.read_to_string().unwrap(), "a longer first version");

        // Replaced, not overwritten in place: a shorter export over a longer one
        // must not keep the old tail, or the JSON would not parse.
        chosen.write(b"short").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "short");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_chosen_file_url_goes_through_the_plugin_too() {
        // The URL half of `FilePath`, which `Fs::open` resolves rather than
        // refusing. On Linux only `file://` can be exercised; `content://` is the
        // same call answered by the plugin's Kotlin half on a device.
        let dir = scratch("url");
        let app = app();
        let file = dir.join("backup.json");
        let url = tauri::Url::from_file_path(&file).expect("an absolute temp path");
        let chosen = PickedFile::from_dialog(app.handle(), FilePath::Url(url));

        chosen.write("{\"学\":1}".as_bytes()).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "{\"学\":1}");
        assert_eq!(chosen.read_to_string().unwrap(), "{\"学\":1}");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_failure_names_the_file_and_says_which_way_it_failed() {
        let dir = scratch("missing");
        let app = app();
        let missing = dir.join("not-there.json");
        let chosen = PickedFile::from_dialog(app.handle(), FilePath::Path(missing.clone()));
        let error = chosen.read_to_string().unwrap_err();
        assert!(error.starts_with("could not read "), "{error}");
        assert!(error.contains(&missing.display().to_string()), "{error}");

        // A URL the desktop plugin cannot open is an error, not a panic.
        let content =
            FilePath::from_str("content://com.android.externalstorage.documents/document/x")
                .unwrap();
        let chosen = PickedFile::from_dialog(app.handle(), content);
        let error = chosen.write(b"x").unwrap_err();
        assert!(
            error.starts_with("could not write the file you chose"),
            "{error}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
