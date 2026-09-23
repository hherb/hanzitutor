//! The notices have to travel with the app, so this test makes losing one loud.
//!
//! The roadmap names the trap directly: *"verify the notices survive bundling —
//! this is easy to get wrong and only shows up in the packaged app."* A notice
//! is therefore pinned in three places at once, and this file fails if any two
//! of them disagree:
//!
//! 1. the file in `licences/` (what a redistributor can read),
//! 2. the text compiled into the binary (what the Licences screen shows), and
//! 3. the resource map in `tauri.conf.json` (what the `.app` actually copies).
//!
//! It also pins the version string, which lives in three files that drift apart
//! silently, and checks that the required licences are present *and* are the
//! real texts rather than placeholders.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use hanzi_tutor_lib::licences::{self, NOTICES};

/// The repository root, one level above `src-tauri`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri always has a parent")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()))
}

fn tauri_config() -> serde_json::Value {
    serde_json::from_str(&read("src-tauri/tauri.conf.json")).expect("tauri.conf.json is valid JSON")
}

#[test]
fn every_notice_is_on_disk_and_is_the_text_that_was_compiled_in() {
    for notice in NOTICES {
        let path = repo_root().join(notice.file);
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} is catalogued but unreadable: {e}", notice.file));
        assert!(
            !on_disk.trim().is_empty(),
            "{} is empty — an empty notice is not a notice",
            notice.file
        );
        assert_eq!(
            on_disk.trim_end(),
            notice.text.trim_end(),
            "{} on disk differs from the text compiled into the binary; \
             the Licences screen would show something the bundle does not contain",
            notice.file
        );
    }
}

#[test]
fn every_licence_text_in_the_directory_is_catalogued() {
    let dir = repo_root().join("licences");
    let mut catalogued: BTreeSet<String> = NOTICES
        .iter()
        .filter(|n| n.file.starts_with("licences/"))
        .map(|n| n.file.to_string())
        .collect();

    for entry in std::fs::read_dir(&dir).expect("licences/ exists") {
        let entry = entry.expect("readable directory entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        // README.md indexes the directory; it is not itself a notice.
        if name == "README.md" || entry.file_type().expect("file type").is_dir() {
            continue;
        }
        let relative = format!("licences/{name}");
        assert!(
            catalogued.remove(&relative),
            "{relative} is in licences/ but is not in the catalogue in \
             src-tauri/src/licences.rs, so it would ship as an unlisted notice \
             (or not ship at all)"
        );
    }

    assert!(
        catalogued.is_empty(),
        "catalogued but missing from licences/: {catalogued:?}"
    );
}

/// The Content Security Policy the app actually compiles, as Tauri reads it.
///
/// Parsed through Tauri's own types rather than by hand: this is the same
/// deserialisation `generate_context!` walks, so a test that passes here is a
/// policy the build would accept, and a malformed one fails here instead of at
/// bundle time.
#[test]
fn a_content_security_policy_is_set_and_still_allows_the_ipc_transport() {
    let config: tauri::Context<tauri::Wry> = tauri::generate_context!("tauri.conf.json");
    let policy = config
        .config()
        .app
        .security
        .csp
        .clone()
        .expect(
            "a CSP must be set: with it off, any injected markup can load remote \
             script and the webview can make outbound requests, which is the one \
             claim this app makes that a CSP is what enforces",
        );

    let directives: std::collections::HashMap<String, tauri::utils::config::CspDirectiveSources> =
        policy.into();
    let sources = |name: &str| -> Vec<String> {
        let directive = directives
            .get(name)
            .unwrap_or_else(|| panic!("the CSP has no {name} directive"));
        match serde_json::to_value(directive).unwrap() {
            serde_json::Value::Array(list) => list
                .into_iter()
                .map(|v| v.as_str().expect("a source is a string").to_string())
                .collect(),
            serde_json::Value::String(inline) => {
                inline.split_whitespace().map(str::to_string).collect()
            }
            other => panic!("{name} is neither a list nor a string: {other}"),
        }
    };

    // Nothing loads by default except what the app itself serves. Every asset the
    // frontend asks for — the clip manifests, the font, the audio — is relative,
    // so `'self'` covers them without naming an origin.
    assert!(
        sources("default-src").contains(&"'self'".to_string()),
        "default-src should be 'self'"
    );

    // The IPC transport. `ipc:` is the custom scheme and `http://ipc.localhost`
    // is what Windows and Android route it through; dropping either makes the
    // whole interface unable to call a command, which is the failure this test
    // exists to catch rather than a hardening question.
    let connect = sources("connect-src");
    for needed in ["ipc:", "http://ipc.localhost"] {
        assert!(
            connect.contains(&needed.to_string()),
            "connect-src is missing {needed}, so IPC would be blocked: {connect:?}"
        );
    }

    // `devCsp` is deliberately unset. If one is ever added, the production
    // policy keeps shipping unchanged — `csp` is what is injected into builds,
    // and the separate field exists so a development-only origin cannot reach a
    // release by accident. Setting one is therefore the moment to re-read both.
    let raw: serde_json::Value = serde_json::from_str(&read("src-tauri/tauri.conf.json")).unwrap();
    assert!(
        raw["app"]["security"].get("devCsp").is_none(),
        "devCsp is now set: the development policy has diverged from the one \
         that ships, so check the production `csp` above still stands on its own"
    );
}

#[test]
fn the_bundle_copies_exactly_the_catalogued_notices() {
    let config = tauri_config();
    let resources = config["bundle"]["resources"]
        .as_object()
        .expect("bundle.resources is an object, so the mapping is explicit")
        .iter()
        .map(|(source, target)| {
            (
                source.clone(),
                target
                    .as_str()
                    .expect("a resource target is a string")
                    .to_string(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let expected = NOTICES
        .iter()
        .map(|n| (format!("../{}", n.file), n.bundle_path.to_string()))
        .collect::<std::collections::BTreeMap<_, _>>();

    assert_eq!(
        resources, expected,
        "\ntauri.conf.json's bundle.resources must copy every catalogued notice \
         and nothing else. A notice that is not copied is a notice that is not \
         in the .app."
    );
}

#[test]
fn the_required_licences_are_present_and_are_the_real_texts() {
    // (id, something only the genuine text contains)
    let required = [
        ("app", "GNU AFFERO GENERAL PUBLIC LICENSE"),
        ("arphic", "ARPHIC PUBLIC LICENSE"),
        ("lgpl", "GNU LESSER GENERAL PUBLIC LICENSE"),
        ("hanzidb", "MIT License"),
        ("hsk-vocabulary", "MIT License"),
        ("cc-cedict", "Creative Commons Attribution-ShareAlike 4.0"),
        ("cc-by-sa", "Attribution-ShareAlike 4.0 International"),
        ("font", "SIL OPEN FONT LICENSE"),
        // Not a permissive licence, and the reason it is checked here rather than
        // left to the compiler: it arrives inside a prebuilt native archive that
        // only the mobile builds link whole. See the notice's own text.
        ("espeak-ng", "GNU GENERAL PUBLIC LICENSE"),
    ];

    let ids: BTreeSet<&str> = NOTICES.iter().map(|n| n.id).collect();
    for (id, marker) in required {
        let notice = NOTICES
            .iter()
            .find(|n| n.id == id)
            .unwrap_or_else(|| panic!("the {id} notice is missing; present: {ids:?}"));
        assert!(
            notice.text.contains(marker),
            "the {} notice does not contain {marker:?}, so it is not the licence \
             text it claims to be",
            notice.id
        );
        assert!(
            notice.text.len() > 500,
            "the {} notice is only {} bytes — that is a stub, not a licence",
            notice.id,
            notice.text.len()
        );
    }

    // The notices that were actually fetched from upstream are the ones whose
    // absence would be a licence breach, so name them explicitly.
    for id in ["arphic", "lgpl", "cc-by-sa", "font", "espeak-ng"] {
        assert!(ids.contains(id), "the {id} notice must ship");
    }

    // The study database is a compiled-in third-party library, so it is
    // accounted for too. SQLite's notice is short by nature — it is a statement
    // that there is no licence — so it is checked for its own words rather than
    // for the length a legal code has.
    for (id, marker) in [
        ("sqlite", "public domain"),
        ("sqlite", "disclaims copyright to this source code"),
        ("rusqlite", "The rusqlite developers"),
    ] {
        let notice = NOTICES
            .iter()
            .find(|n| n.id == id)
            .unwrap_or_else(|| panic!("the {id} notice is missing; present: {ids:?}"));
        assert!(
            notice.text.contains(marker),
            "the {id} notice does not contain {marker:?}"
        );
    }

    // Speech recognition added two more compiled-in libraries, and both arrive
    // through a prebuilt native archive rather than through a crate, so cargo
    // will not notice if their notices go missing. They are checked here for the
    // same reason SQLite is: the absence of a notice is a licence breach that no
    // compiler can see. `onnxruntime` is the one that matters most, because it is
    // linked in from inside the sherpa-onnx archive and there is no crate
    // dependency pointing at it at all.
    for (id, marker) in [
        ("sherpa-onnx", "Apache License"),
        ("onnxruntime", "Microsoft Corporation"),
        // The Android set shares the Apache text with cpal and sherpa-onnx, which
        // is why this one is checked for its *presence* rather than for a licence
        // of its own: there is no separate file for it to get wrong.
        ("android-libraries", "Apache License"),
    ] {
        let notice = NOTICES
            .iter()
            .find(|n| n.id == id)
            .unwrap_or_else(|| panic!("the {id} notice is missing; present: {ids:?}"));
        assert!(
            notice.text.contains(marker),
            "the {id} notice does not contain {marker:?}"
        );
        assert!(
            notice.text.len() > 500,
            "the {} notice is only {} bytes — that is a stub, not a licence",
            notice.id,
            notice.text.len()
        );
        // Both are *compiled in*, so their text has to be readable out of the
        // bundle by a redistributor who never launches the app.
        assert!(
            notice.bundle_path.starts_with("licences/"),
            "the {} notice must travel in the bundle, not only in the binary",
            notice.id
        );
    }
}

#[test]
fn the_interface_font_is_present_and_is_not_a_stub() {
    let font = repo_root().join("src/assets/fonts/NotoSansSC-VF.ttf");
    let size = std::fs::metadata(&font)
        .unwrap_or_else(|e| panic!("the bundled font is missing at {}: {e}", font.display()))
        .len();
    // A real CJK face covering the simplified set is megabytes; anything small
    // is a Latin-only font or a failed download.
    assert!(
        size > 5_000_000,
        "the bundled font is only {size} bytes, which cannot cover the CJK set"
    );
}

#[test]
fn the_version_is_the_same_in_every_file_that_carries_one() {
    let cargo = env!("CARGO_PKG_VERSION");
    assert_eq!(
        tauri_config()["version"]
            .as_str()
            .expect("a version string"),
        cargo,
        "tauri.conf.json's version disagrees with Cargo.toml's, so the About \
         screen and the bundle's Info.plist would report different versions"
    );

    let package: serde_json::Value =
        serde_json::from_str(&read("package.json")).expect("package.json is valid JSON");
    assert_eq!(
        package["version"].as_str().expect("a version string"),
        cargo,
        "package.json's version disagrees with Cargo.toml's"
    );

    // The command the About screen uses must report the same thing.
    assert_eq!(licences::APP.version, cargo);
}

#[test]
fn the_app_reports_an_identifier_the_bundle_config_agrees_with() {
    assert_eq!(
        tauri_config()["identifier"]
            .as_str()
            .expect("an identifier"),
        licences::APP.identifier,
        "the About screen would name a different bundle identifier than the one \
         macOS stores study data under"
    );
}
