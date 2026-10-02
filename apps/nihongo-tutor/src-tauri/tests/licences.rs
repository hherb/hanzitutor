//! The notice catalogue, checked three ways.
//!
//! Notices are the kind of thing that goes missing quietly: a file is added and
//! never catalogued, or catalogued and never copied into the bundle, and nothing
//! notices until a redistributor goes looking. The main app's catalogue has the
//! same three checks; this is the kana app's, over its own notice set.

use nihongo_tutor_lib::licences::{notices, APP};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir()
        .join("../../..")
        .canonicalize()
        .expect("the repository root resolves")
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn tauri_config() -> Value {
    serde_json::from_str(&read("apps/nihongo-tutor/src-tauri/tauri.conf.json"))
        .expect("tauri.conf.json parses")
}

#[test]
fn every_notice_is_on_disk_and_is_the_text_that_was_compiled_in() {
    for notice in notices() {
        let path = repo_root().join(notice.file);
        assert!(
            path.exists(),
            "{} names {}, which does not exist",
            notice.id,
            notice.file
        );
        // The compiled-in copy is what the app shows and what a redistributor
        // reads out of the binary, so it has to be the same text as the file.
        assert_eq!(
            notice.text,
            read(notice.file),
            "{}: the compiled-in text differs from {}",
            notice.id,
            notice.file
        );
    }
}

#[test]
fn every_notice_text_in_the_apps_own_directory_is_catalogued() {
    let dir = manifest_dir().join("licences");
    let mut catalogued: BTreeSet<String> = notices()
        .iter()
        .filter(|n| n.file.starts_with("apps/nihongo-tutor/src-tauri/licences/"))
        .map(|n| {
            n.file
                .rsplit('/')
                .next()
                .expect("a file name")
                .to_string()
        })
        .collect();

    for entry in std::fs::read_dir(&dir).expect("the app's licences/ exists") {
        let entry = entry.expect("a readable entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().expect("a file type").is_dir() {
            continue;
        }
        assert!(
            catalogued.remove(&name),
            "{name} is in the app's licences/ but is not in the catalogue, so it would \
             ship as an unlisted notice (or not ship at all)"
        );
    }

    assert!(
        catalogued.is_empty(),
        "catalogued but missing from the app's licences/: {catalogued:?}"
    );
}

#[test]
fn the_bundle_copies_exactly_the_catalogued_notices() {
    let config = tauri_config();
    let resources = config["bundle"]["resources"]
        .as_object()
        .expect("bundle.resources is an object");

    let configured: BTreeSet<(String, String)> = resources
        .iter()
        .map(|(from, to)| {
            (
                from.clone(),
                to.as_str().expect("a bundle path").to_string(),
            )
        })
        .collect();

    let expected: BTreeSet<(String, String)> = notices()
        .iter()
        .map(|n| {
            // The catalogue holds paths relative to the repository root; Tauri's
            // `resources` keys are relative to the app's `src-tauri`. A notice
            // inside the app is named directly, anything else climbs three
            // directories to the root.
            const APP_PREFIX: &str = "apps/nihongo-tutor/src-tauri/";
            let from = match n.file.strip_prefix(APP_PREFIX) {
                Some(inside) => inside.to_string(),
                None => format!("../../../{}", n.file),
            };
            (from, n.bundle_path.to_string())
        })
        .collect();

    assert_eq!(
        configured, expected,
        "the bundle config and the catalogue disagree about what ships"
    );
}

/// A file that exists but is a stub would pass every check above, so check that
/// each text is the licence it claims to be.
#[test]
fn the_texts_are_the_real_licences_and_not_placeholders() {
    let expectations: &[(&str, &[&str])] = &[
        ("agpl", &["GNU AFFERO GENERAL PUBLIC LICENSE", "Version 3"]),
        ("lgpl", &["GNU LESSER GENERAL PUBLIC LICENSE", "Version 3"]),
        ("animcjk", &["AnimCJK", "Arphic Public License", "Lesser General Public License"]),
        ("arphic", &["Arphic"]),
        ("provenance", &["Data provenance and licences", "AnimCJK"]),
    ];

    for (id, needles) in expectations {
        let notice = notices()
            .iter()
            .find(|n| n.id == *id)
            .unwrap_or_else(|| panic!("{id} is catalogued"));
        assert!(
            notice.text.len() > 500,
            "{id} is only {} bytes, which is too short to be a licence",
            notice.text.len()
        );
        for needle in *needles {
            assert!(
                notice.text.contains(needle),
                "{id} does not mention {needle:?}"
            );
        }
    }
}

/// The AnimCJK notice is the one obligation specific to this app, and the whole
/// reason the kana pipeline is shaped the way it is.
#[test]
fn the_animcjk_notice_records_the_modification() {
    let notice = notices()
        .iter()
        .find(|n| n.id == "animcjk")
        .expect("an AnimCJK notice");
    let covers = notice.covers.to_lowercase();
    assert!(
        covers.contains("modif") || covers.contains("fold") || covers.contains("segment"),
        "the notice must say what was changed, per LGPL-3.0 §2: {covers}"
    );
    assert!(
        notice.covers.contains("kana.bin.gz") || notice.covers.contains("graphicsJaKana"),
        "the notice must name what it covers"
    );
}

#[test]
fn the_app_info_and_the_bundle_config_agree() {
    let config = tauri_config();
    assert_eq!(config["identifier"], APP.identifier);
    assert_eq!(config["productName"], APP.name);
    assert_eq!(config["version"], APP.version);
    assert_eq!(config["bundle"]["copyright"], APP.copyright);
}

#[test]
fn every_notice_has_something_to_show() {
    let mut seen = BTreeSet::new();
    for notice in notices() {
        assert!(seen.insert(notice.id), "duplicate notice id {}", notice.id);
        assert!(!notice.title.is_empty(), "{} has no title", notice.id);
        assert!(!notice.licence.is_empty(), "{} names no licence", notice.id);
        assert!(!notice.covers.is_empty(), "{} says nothing about what it covers", notice.id);
        assert!(
            notice.source.starts_with("http"),
            "{} has no source a reader can check",
            notice.id
        );
    }
}

#[test]
fn the_notice_files_are_tracked_and_not_ignored() {
    // A notice that is gitignored would vanish from a clone and from the bundle,
    // which is the failure mode this whole module exists to prevent.
    for notice in notices() {
        let path = repo_root().join(notice.file);
        assert!(path.is_file(), "{} is not a file: {}", notice.id, notice.file);
        let gitignored = std::process::Command::new("git")
            .arg("check-ignore")
            .arg("-q")
            .arg(&path)
            .current_dir(repo_root())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(!gitignored, "{} is gitignored: {}", notice.id, notice.file);
    }
}

#[test]
fn the_licences_command_returns_the_catalogue() {
    // The frontend gets the text over IPC rather than carrying a second copy.
    let from_command = nihongo_tutor_lib::licence_notices();
    assert_eq!(from_command.len(), notices().len());
    assert_eq!(from_command[0].id, notices()[0].id);
}

#[test]
fn a_path_helper_resolves_as_the_tests_assume() {
    // Cheap guard on the relative-path arithmetic the other tests depend on: if
    // the app is ever moved, this fails first and says why.
    let root = repo_root();
    assert!(root.join("LICENSE").is_file(), "LICENSE is at the repository root");
    assert!(
        root.join("licences/LGPL-3.0.txt").is_file(),
        "the shared licences/ directory is at the repository root"
    );
    assert!(
        Path::new(&manifest_dir()).ends_with("apps/nihongo-tutor/src-tauri"),
        "the manifest directory is where these paths assume it is"
    );
}
