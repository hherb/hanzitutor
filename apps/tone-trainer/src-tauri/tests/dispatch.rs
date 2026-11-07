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

/// `true` when Tauri runs the command `name` in `source` off the main thread: an
/// `async` argument to the attribute, alone or beside others, or an `async fn`.
fn runs_off_the_main_thread(source: &str, name: &str) -> bool {
    let head = command_head(source, name);
    let close = head.find(']').expect("the attribute closes");
    let arguments = head["#[tauri::command".len()..close].trim();
    let async_attribute = arguments
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .is_some_and(|inner| inner.split(',').any(|argument| argument.trim() == "async"));
    let async_fn = head[close + 1..]
        .split_whitespace()
        .any(|word| word == "async");
    async_attribute || async_fn
}

/// Everything from the command attribute above `fn name(` to the name itself.
fn command_head<'a>(source: &'a str, name: &str) -> &'a str {
    let at = source
        .find(&format!("fn {name}("))
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
    head
}

/// The body of the function `name`, up to its closing brace at the margin.
fn body_of<'a>(source: &'a str, name: &str) -> &'a str {
    let at = source
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("{name} should still be a command"));
    let rest = &source[at..];
    &rest[..rest.find("\n}\n").expect("the function ends")]
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

    // The microphone's three go further, onto the blocking pool: `(async)` runs a
    // body on one of the runtime's core workers, and a decode or a device waiting on
    // a permission prompt would park one for seconds.
    for name in ["listen_start", "listen_stop", "listen_cancel"] {
        assert!(
            body_of(&source, name).contains("on_a_blocking_thread("),
            "{name} should run its work on the blocking pool, not a runtime worker"
        );
    }
}

#[test]
fn the_dispatch_reading_understands_every_form_of_the_attribute() {
    let source = "
#[tauri::command]
fn plain() {}
#[tauri::command(async)]
fn attribute() {}
#[tauri::command(rename_all = \"snake_case\", async)]
fn among_others() {}
#[tauri::command(rename_all = \"snake_case\")]
fn renamed_only() {}
#[tauri::command]
async fn keyword() {}
";
    for name in ["attribute", "among_others", "keyword"] {
        assert!(runs_off_the_main_thread(source, name), "{name}");
    }
    for name in ["plain", "renamed_only"] {
        assert!(!runs_off_the_main_thread(source, name), "{name}");
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
