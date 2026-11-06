//! Which thread each command runs on, read from the source (issue #26).
//!
//! Tauri 2 decides that from the attribute and the signature alone. A plain
//! `#[tauri::command]` on a plain `fn` is called inline from the IPC handler, which
//! the webview invokes on the main thread — the thread the window is drawn on —
//! while `#[tauri::command(async)]` or an `async fn` is spawned on the async runtime
//! (`tauri-macros`: `body_blocking` against `body_async`). No behavioural test can
//! see the difference, so the signatures are what is checked, the way the full app's
//! `ipc_contract.rs` checks that no file command takes a path.

use std::path::Path;

/// `true` when Tauri runs the command `name` in `source` off the main thread.
fn runs_off_the_main_thread(source: &str, name: &str) -> bool {
    let needle = format!("fn {name}(");
    let at = source
        .find(&needle)
        .unwrap_or_else(|| panic!("{name} should still be a command"));
    let before = &source[..at];
    let attribute = before
        .rfind("#[tauri::command")
        .unwrap_or_else(|| panic!("{name} has no command attribute"));
    let head = &before[attribute..];
    assert!(
        !head.contains("fn "),
        "the nearest command attribute above {name} belongs to another function"
    );
    head.starts_with("#[tauri::command(async)]") || head.contains("async ")
}

fn source() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("lib.rs is part of the crate")
}

#[test]
fn the_slow_commands_do_not_run_on_the_main_thread() {
    // With the recognition model installed, every release of the button used to
    // freeze the window while the model loaded and decoded, and a microphone
    // permission prompt left the app beachballing for up to eight seconds while
    // `listen_start` waited for the device. These are the commands that can block
    // for that long, or that wait on a lock one of them holds for that long: the
    // recorder's slot (held while the device opens) and the recogniser's (held while
    // the model loads, and while an uninstall deletes it).
    let source = source();
    for name in [
        "listen_start",
        "listen_stop",
        "listen_cancel",
        "asr_install",
        "asr_remove",
    ] {
        assert!(
            runs_off_the_main_thread(&source, name),
            "{name} is a plain command again, so it runs on the main thread and freezes \
             the window while it works: mark it `#[tauri::command(async)]`"
        );
    }
}

#[test]
fn the_voice_keeps_the_order_it_was_asked_in() {
    // Starting and stopping the voice are quick, and they are the two whose order
    // matters: each tap cuts off the utterance before it, and two that overtook each
    // other on the runtime's threads would leave the earlier character the one still
    // speaking. On the main thread they run in the order they were sent.
    let source = source();
    for name in ["speak", "stop_speaking"] {
        assert!(
            !runs_off_the_main_thread(&source, name),
            "{name} should stay in order on the main thread"
        );
    }
}
